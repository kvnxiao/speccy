//! `speccy report` command logic.
//!
//! Renders the Phase 5 report prompt for one spec. The CLI never invokes
//! a model: it locates the spec via [`speccy_core::workspace::scan`],
//! refuses unless every task is `[x]`, derives a per-task retry count
//! from inline notes beginning with `Retry:`, inlines SPEC.md / TASKS.md
//! / AGENTS.md / the retry summary into the embedded `report.md`
//! template, applies budget trimming, and writes the rendered prompt to
//! stdout.
//!
//! See `.speccy/specs/0011-report-command/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use regex::Regex;
use speccy_core::ParseError;
use speccy_core::parse::Task;
use speccy_core::parse::TaskState;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::load_agents_md;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::trim_to_budget;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;
use std::sync::OnceLock;
use thiserror::Error;

const RETRY_PREFIX: &str = "Retry:";

/// One task that failed the completeness gate, surfaced inside
/// [`ReportError::Incomplete`] so the dispatcher can list every
/// offender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffendingTask {
    /// `T-NNN` identifier of the offending task.
    pub id: String,
    /// State that disqualified the task. Always one of `Open`,
    /// `InProgress`, or `AwaitingReview`.
    pub state: TaskState,
}

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReportError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Argument did not match the `SPEC-\d{4,}` shape.
    #[error("invalid SPEC-ID `{arg}`; expected format SPEC-NNNN (4+ digits)")]
    InvalidSpecIdFormat {
        /// The string supplied by the user.
        arg: String,
    },
    /// No spec directory matched the supplied ID.
    #[error("spec `{id}` not found under .speccy/specs/")]
    SpecNotFound {
        /// The canonical SPEC ID that was looked up.
        id: String,
    },
    /// TASKS.md is required for the report prompt but absent on disk.
    #[error("TASKS.md required for `speccy report {id}`; create it via `speccy tasks {id}` first")]
    TasksMdRequired {
        /// The canonical SPEC ID being reported.
        id: String,
    },
    /// SPEC.md or TASKS.md parse failure. Boxed to keep the variant
    /// size small.
    #[error("failed to parse {artifact} for {id}")]
    Parse {
        /// Which file failed to parse.
        artifact: &'static str,
        /// The SPEC ID we were trying to parse.
        id: String,
        /// Underlying parser error.
        #[source]
        source: Box<ParseError>,
    },
    /// One or more tasks are not in the `Done` state. The dispatcher
    /// lists every offender on stderr before exiting.
    #[error("spec `{id}` has incomplete tasks; all tasks must be [x] before report")]
    Incomplete {
        /// The canonical SPEC ID being reported.
        id: String,
        /// Every task in `Open` / `InProgress` / `AwaitingReview`, in
        /// declared order.
        offending: Vec<OffendingTask>,
    },
    /// Template lookup or substitution helper failed.
    #[error("prompt template error")]
    Prompt(#[from] PromptError),
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
    /// I/O failure while reading TASKS.md raw bytes or writing the
    /// rendered prompt to stdout.
    #[error("I/O error during report rendering")]
    Io(#[from] std::io::Error),
}

/// `speccy report` arguments.
#[derive(Debug, Clone)]
pub struct ReportArgs {
    /// The `SPEC-NNNN` argument (required).
    pub spec_id: String,
}

/// Run `speccy report` from `cwd`, writing the rendered prompt to
/// `out`.
///
/// # Errors
///
/// Returns any [`ReportError`] variant if discovery, lookup, parsing,
/// the completeness gate, template loading, rendering, or I/O fails.
pub fn run(args: &ReportArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), ReportError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(ReportError::ProjectRootNotFound),
        Err(other) => return Err(ReportError::Workspace(other)),
    };

    let canonical_id = validate_spec_id(&args.spec_id)?;

    let mut workspace = scan(&project_root);
    let position = workspace
        .specs
        .iter()
        .position(|s| s.spec_id.as_deref() == Some(canonical_id.as_str()))
        .ok_or_else(|| ReportError::SpecNotFound {
            id: canonical_id.clone(),
        })?;
    let parsed = workspace.specs.swap_remove(position);

    let parsed_spec_md = parsed.spec_md.map_err(|source| ReportError::Parse {
        artifact: "SPEC.md",
        id: canonical_id.clone(),
        source: Box::new(source),
    })?;

    let parsed_tasks_md = match parsed.tasks_md {
        Some(Ok(t)) => t,
        Some(Err(source)) => {
            return Err(ReportError::Parse {
                artifact: "TASKS.md",
                id: canonical_id,
                source: Box::new(source),
            });
        }
        None => return Err(ReportError::TasksMdRequired { id: canonical_id }),
    };

    let offending = collect_offending(&parsed_tasks_md.tasks);
    if !offending.is_empty() {
        return Err(ReportError::Incomplete {
            id: canonical_id,
            offending,
        });
    }

    let tasks_md_path = parsed
        .tasks_md_path
        .ok_or_else(|| ReportError::TasksMdRequired {
            id: canonical_id.clone(),
        })?;
    let tasks_raw = fs_err::read_to_string(tasks_md_path.as_std_path())?;

    let retry_summary = format_retry_summary(&parsed_tasks_md.tasks);
    let agents = load_agents_md(&project_root);
    let template = load_template("report.md")?;

    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", canonical_id);
    vars.insert("spec_md", parsed_spec_md.raw);
    vars.insert("tasks_md", tasks_raw);
    vars.insert("retry_summary", retry_summary);
    vars.insert("agents", agents);

    let rendered = render(template, &vars);
    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn validate_spec_id(raw: &str) -> Result<String, ReportError> {
    if !spec_id_regex().is_match(raw) {
        return Err(ReportError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn spec_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^SPEC-\d{4,}$").unwrap())
}

fn collect_offending(tasks: &[Task]) -> Vec<OffendingTask> {
    tasks
        .iter()
        .filter(|t| t.state != TaskState::Completed)
        .map(|t| OffendingTask {
            id: t.id.clone(),
            state: t.state,
        })
        .collect()
}

fn count_retries(task: &Task) -> usize {
    task.notes()
        .iter()
        .filter(|n| n.starts_with(RETRY_PREFIX))
        .count()
}

fn format_retry_summary(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "_No tasks recorded._".to_owned();
    }
    let mut out = String::new();
    for task in tasks {
        let count = count_retries(task);
        let suffix = if count == 1 { "retry" } else { "retries" };
        if writeln!(out, "- {id}: {count} {suffix}", id = task.id).is_err() {
            // Writing to a String is infallible; this arm is unreachable
            // but absorbing the Result keeps `let_underscore_must_use`
            // happy without `.expect`.
            break;
        }
    }
    out
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`ReportError::Cwd`] if `std::env::current_dir` fails, or
/// [`ReportError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, ReportError> {
    let std_path = std::env::current_dir().map_err(ReportError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| ReportError::CwdNotUtf8)
}

#[cfg(test)]
mod tests {
    use super::OffendingTask;
    use super::RETRY_PREFIX;
    use super::collect_offending;
    use super::count_retries;
    use super::format_retry_summary;
    use super::spec_id_regex;
    use super::validate_spec_id;
    use speccy_core::parse::ElementSpan;
    use speccy_core::parse::Task;
    use speccy_core::parse::TaskState;

    fn task_with_notes(id: &str, state: TaskState, notes: &[String]) -> Task {
        let mut body = String::from("x\n");
        for note in notes {
            body.push_str("- ");
            body.push_str(note);
            body.push('\n');
        }
        let zero_span = ElementSpan { start: 0, end: 0 };
        Task {
            id: id.to_owned(),
            state,
            covers: Vec::new(),
            scenarios_body: "placeholder\n".to_owned(),
            scenarios_span: zero_span,
            body,
            span: zero_span,
        }
    }

    #[test]
    fn valid_spec_ids_pass_regex() {
        assert!(spec_id_regex().is_match("SPEC-0011"));
        assert!(spec_id_regex().is_match("SPEC-9999"));
        assert!(spec_id_regex().is_match("SPEC-10000"));
    }

    #[test]
    fn invalid_spec_ids_rejected() {
        validate_spec_id("FOO").expect_err("`FOO` must fail format validation");
        validate_spec_id("SPEC-1").expect_err("`SPEC-1` has fewer than 4 digits");
        validate_spec_id("spec-0001").expect_err("lowercase prefix must be rejected");
        validate_spec_id("SPEC-").expect_err("missing digits must be rejected");
    }

    #[test]
    fn collect_offending_lists_non_done_tasks_in_order() {
        let tasks = vec![
            task_with_notes("T-001", TaskState::Completed, &[]),
            task_with_notes("T-002", TaskState::Pending, &[]),
            task_with_notes("T-003", TaskState::InProgress, &[]),
            task_with_notes("T-004", TaskState::InReview, &[]),
            task_with_notes("T-005", TaskState::Completed, &[]),
        ];
        let offending = collect_offending(&tasks);
        assert_eq!(
            offending,
            vec![
                OffendingTask {
                    id: "T-002".to_owned(),
                    state: TaskState::Pending,
                },
                OffendingTask {
                    id: "T-003".to_owned(),
                    state: TaskState::InProgress,
                },
                OffendingTask {
                    id: "T-004".to_owned(),
                    state: TaskState::InReview,
                },
            ],
        );
    }

    #[test]
    fn collect_offending_empty_when_all_done() {
        let tasks = vec![
            task_with_notes("T-001", TaskState::Completed, &[]),
            task_with_notes("T-002", TaskState::Completed, &[]),
        ];
        assert!(collect_offending(&tasks).is_empty());
    }

    #[test]
    fn collect_offending_empty_when_no_tasks() {
        assert!(collect_offending(&[]).is_empty());
    }

    #[test]
    fn count_retries_matches_exact_prefix() {
        let task = task_with_notes(
            "T-001",
            TaskState::Completed,
            &[
                "Implementer note (session-abc): added bcrypt".to_owned(),
                "Review (security, blocking): cost 10".to_owned(),
                "Retry: address bcrypt cost.".to_owned(),
                "Implementer note: bumped to 12".to_owned(),
                "Retry: fix style.".to_owned(),
            ],
        );
        assert_eq!(count_retries(&task), 2);
    }

    #[test]
    fn count_retries_zero_when_none_match() {
        let task = task_with_notes(
            "T-001",
            TaskState::Completed,
            &["Review (business, pass): OK".to_owned()],
        );
        assert_eq!(count_retries(&task), 0);
    }

    #[test]
    fn count_retries_rejects_inexact_prefix() {
        let task = task_with_notes(
            "T-001",
            TaskState::Completed,
            &[
                "Retry on bcrypt".to_owned(),
                "retry: lowercase".to_owned(),
                "Retried: past tense".to_owned(),
            ],
        );
        assert_eq!(count_retries(&task), 0);
    }

    #[test]
    fn format_retry_summary_lists_every_task() {
        let tasks = vec![
            task_with_notes(
                "T-001",
                TaskState::Completed,
                &[
                    format!("{RETRY_PREFIX} first"),
                    format!("{RETRY_PREFIX} second"),
                ],
            ),
            task_with_notes("T-002", TaskState::Completed, &[]),
        ];
        let summary = format_retry_summary(&tasks);
        assert!(summary.contains("- T-001: 2 retries"), "got: {summary}");
        assert!(summary.contains("- T-002: 0 retries"), "got: {summary}");
    }

    #[test]
    fn format_retry_summary_uses_singular_for_one_retry() {
        let tasks = vec![task_with_notes(
            "T-001",
            TaskState::Completed,
            &[format!("{RETRY_PREFIX} once")],
        )];
        let summary = format_retry_summary(&tasks);
        assert!(summary.contains("- T-001: 1 retry"), "got: {summary}");
        assert!(!summary.contains("1 retries"), "got: {summary}");
    }

    #[test]
    fn format_retry_summary_empty_tasks_returns_placeholder() {
        let summary = format_retry_summary(&[]);
        assert!(summary.contains("No tasks recorded"), "got: {summary}");
    }
}
