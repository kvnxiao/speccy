//! Workspace task lookup by `T-NNN` reference.
//!
//! Two forms of task reference:
//!
//! - **Unqualified** `T-NNN` — searches every parsed spec. Returns the unique
//!   match, or [`LookupError::Ambiguous`] if multiple specs have the same
//!   `T-NNN`.
//! - **Qualified** `SPEC-NNNN/T-NNN` — scopes the lookup to one spec. Bypasses
//!   ambiguity entirely.
//!
//! Shared by SPEC-0008 (`speccy implement`) and SPEC-0009 (`speccy
//! review`) so the lookup logic lives in one place. See
//! `.speccy/specs/0008-implement-command/SPEC.md` DEC-001.

use crate::parse::SpecDoc;
use crate::parse::SpecMd;
use crate::parse::Task;
use crate::parse::TasksMd;
use crate::workspace::Workspace;
use camino::Utf8PathBuf;
use regex::Regex;
use std::sync::OnceLock;
use thiserror::Error;

/// Parsed task reference handed to [`find`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRef {
    /// `T-NNN` form. The search scans every spec.
    Unqualified {
        /// `T-NNN` identifier (3+ digits).
        id: String,
    },
    /// `SPEC-NNNN/T-NNN` form. The search is scoped to `spec_id`.
    Qualified {
        /// `SPEC-NNNN` identifier (4+ digits).
        spec_id: String,
        /// `T-NNN` identifier (3+ digits).
        task_id: String,
    },
}

impl TaskRef {
    /// Render the reference back to its on-CLI form.
    #[must_use = "the rendered form is the on-CLI representation"]
    pub fn as_arg(&self) -> String {
        match self {
            TaskRef::Unqualified { id } => id.clone(),
            TaskRef::Qualified { spec_id, task_id } => format!("{spec_id}/{task_id}"),
        }
    }

    /// Return the task ID without the optional spec scope.
    #[must_use = "the bare task id is needed to compare against parsed tasks"]
    pub fn task_id(&self) -> &str {
        match self {
            TaskRef::Unqualified { id } | TaskRef::Qualified { task_id: id, .. } => id,
        }
    }
}

/// Successful lookup result.
///
/// `task_entry_raw` is the verbatim slice of TASKS.md from the task line
/// (inclusive) through the end of its indented sub-list. Trailing blank
/// lines are trimmed. Use it as the `{{task_entry}}` placeholder value
/// without further processing.
#[derive(Debug)]
pub struct TaskLocation<'a> {
    /// Stable `SPEC-NNNN` of the spec containing the task.
    pub spec_id: String,
    /// Parsed SPEC.md for the containing spec.
    pub spec_md: &'a SpecMd,
    /// Parsed SPEC.md marker tree (after SPEC-0019) for the containing
    /// spec, when the marker tree parsed successfully. `None` when the
    /// marker tree failed to parse — callers that need it must surface
    /// that as an error themselves.
    pub spec_doc: Option<&'a SpecDoc>,
    /// Parsed TASKS.md for the containing spec.
    pub tasks_md: &'a TasksMd,
    /// The matched task entry.
    pub task: &'a Task,
    /// Verbatim task subtree from TASKS.md (task line + indented
    /// sub-list bullets, trailing blank lines trimmed).
    pub task_entry_raw: String,
}

/// Failure mode of [`parse_ref`] and [`find`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LookupError {
    /// The argument did not match either accepted form.
    #[error(
        "invalid task reference `{arg}`; expected `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN` (qualified)"
    )]
    InvalidFormat {
        /// Verbatim user input.
        arg: String,
    },
    /// Lookup found no spec containing the task.
    #[error("task `{task_ref}` not found in any spec; run `speccy status` to list specs")]
    NotFound {
        /// On-CLI form of the lookup that returned empty.
        task_ref: String,
    },
    /// Two or more specs contain the same unqualified `T-NNN`.
    #[error(
        "task `{task_id}` is ambiguous; matches in {count} specs: {specs}",
        count = candidate_specs.len(),
        specs = candidate_specs.join(", "),
    )]
    Ambiguous {
        /// The `T-NNN` that matched in multiple specs.
        task_id: String,
        /// Spec IDs in ascending order (matches `workspace::scan`).
        candidate_specs: Vec<String>,
    },
    /// I/O failure while reading TASKS.md to extract the verbatim entry.
    #[error("failed to read TASKS.md at {path}")]
    Io {
        /// Path of the TASKS.md that could not be read.
        path: Utf8PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Parse a `T-NNN` or `SPEC-NNNN/T-NNN` argument into a [`TaskRef`].
///
/// # Errors
///
/// Returns [`LookupError::InvalidFormat`] when `arg` matches neither
/// accepted shape. The offending input is preserved verbatim so CLI
/// surfaces can name it back to the user.
pub fn parse_ref(arg: &str) -> Result<TaskRef, LookupError> {
    if let Some(caps) = qualified_regex().captures(arg) {
        let spec_id = caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        let task_id = caps
            .get(2)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        return Ok(TaskRef::Qualified { spec_id, task_id });
    }
    if unqualified_regex().is_match(arg) {
        return Ok(TaskRef::Unqualified { id: arg.to_owned() });
    }
    Err(LookupError::InvalidFormat {
        arg: arg.to_owned(),
    })
}

/// Locate the task referenced by `task_ref` inside `workspace`.
///
/// Specs whose TASKS.md failed to parse are skipped silently — they
/// cannot contain a matched task and should not poison an otherwise
/// successful lookup. Qualified lookups scope to the named spec and
/// bypass ambiguity entirely.
///
/// # Errors
///
/// - [`LookupError::NotFound`] when no spec contains the task.
/// - [`LookupError::Ambiguous`] when an unqualified ID matches in two or more
///   specs.
/// - [`LookupError::Io`] when TASKS.md cannot be re-read to extract the
///   verbatim entry subtree.
pub fn find<'a>(
    workspace: &'a Workspace,
    task_ref: &TaskRef,
) -> Result<TaskLocation<'a>, LookupError> {
    let candidates: Vec<(String, &'a crate::lint::ParsedSpec, &'a TasksMd, &'a Task)> =
        collect_candidates(workspace, task_ref);

    let single = |candidates: &[(String, &'a crate::lint::ParsedSpec, &'a TasksMd, &'a Task)]| {
        let (sid, parsed, tasks_md, task) =
            candidates.first().ok_or_else(|| LookupError::NotFound {
                task_ref: task_ref.as_arg(),
            })?;
        let entry = extract_task_entry(parsed, tasks_md, task)?;
        let parsed_spec_md = parsed.spec_md_ok().ok_or_else(|| LookupError::NotFound {
            task_ref: task_ref.as_arg(),
        })?;
        Ok(TaskLocation {
            spec_id: sid.clone(),
            spec_md: parsed_spec_md,
            spec_doc: parsed.spec_doc_ok(),
            tasks_md,
            task,
            task_entry_raw: entry,
        })
    };

    match (task_ref, candidates.len()) {
        (TaskRef::Qualified { .. } | TaskRef::Unqualified { .. }, 1) => single(&candidates),
        (TaskRef::Unqualified { id }, n) if n > 1 => {
            let candidate_specs: Vec<String> =
                candidates.iter().map(|(sid, ..)| sid.clone()).collect();
            Err(LookupError::Ambiguous {
                task_id: id.clone(),
                candidate_specs,
            })
        }
        _ => Err(LookupError::NotFound {
            task_ref: task_ref.as_arg(),
        }),
    }
}

fn collect_candidates<'a>(
    workspace: &'a Workspace,
    task_ref: &TaskRef,
) -> Vec<(String, &'a crate::lint::ParsedSpec, &'a TasksMd, &'a Task)> {
    let target_task_id = task_ref.task_id();
    let scope_spec_id = match task_ref {
        TaskRef::Qualified { spec_id, .. } => Some(spec_id.as_str()),
        TaskRef::Unqualified { .. } => None,
    };

    let mut out = Vec::new();
    for parsed in &workspace.specs {
        let Some(spec_id) = parsed.spec_id.as_deref() else {
            continue;
        };
        if let Some(scope) = scope_spec_id
            && spec_id != scope
        {
            continue;
        }
        if parsed.spec_md_ok().is_none() {
            // SPEC.md failed to parse; we cannot render the implementer
            // prompt for tasks in this spec. Treat as if the task is not
            // present here.
            continue;
        }
        let Some(tasks_md) = parsed.tasks_md_ok() else {
            continue;
        };
        if let Some(task) = tasks_md.tasks.iter().find(|t| t.id == target_task_id) {
            out.push((spec_id.to_owned(), parsed, tasks_md, task));
        }
    }
    out
}

fn extract_task_entry(
    parsed: &crate::lint::ParsedSpec,
    tasks_md: &TasksMd,
    task: &Task,
) -> Result<String, LookupError> {
    let Some(path) = parsed.tasks_md_path.as_deref() else {
        return Ok(format!("- {} **{}**", task.state.as_glyph(), task.id));
    };
    let raw = fs_err::read_to_string(path.as_std_path()).map_err(|source| LookupError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(extract_entry_from_raw(&raw, tasks_md, task))
}

fn extract_entry_from_raw(raw: &str, tasks_md: &TasksMd, task: &Task) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();
    let task_idx = task.line.saturating_sub(1);
    if task_idx >= total {
        return String::new();
    }

    let next_task_line = tasks_md
        .tasks
        .iter()
        .filter(|t| t.line > task.line)
        .map(|t| t.line)
        .min();

    let stop_idx = next_task_line.map_or(total, |line| line.saturating_sub(1).min(total));

    let block = collect_block_lines(&lines, task_idx, stop_idx);
    let trimmed = trim_trailing_blanks(block);
    trimmed.join("\n")
}

fn collect_block_lines<'a>(lines: &[&'a str], start: usize, stop: usize) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    let Some(first) = lines.get(start) else {
        return out;
    };
    out.push(first);

    let mut i = start.saturating_add(1);
    while i < stop {
        let Some(line) = lines.get(i) else {
            break;
        };
        let trimmed_empty = line.trim().is_empty();
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if trimmed_empty || indented {
            out.push(line);
        } else {
            break;
        }
        i = i.saturating_add(1);
    }
    out
}

fn trim_trailing_blanks(mut block: Vec<&str>) -> Vec<&str> {
    while block.last().is_some_and(|l| l.trim().is_empty()) {
        block.pop();
    }
    block
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn unqualified_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^T-\d{3,}$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn qualified_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^(SPEC-\d{4,})/(T-\d{3,})$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::TaskRef;
    use super::extract_entry_from_raw;
    use super::parse_ref;
    use crate::parse::Task;
    use crate::parse::TaskState;
    use crate::parse::TasksFrontmatter;
    use crate::parse::TasksMd;

    fn make_tasks_md(tasks: Vec<Task>) -> TasksMd {
        TasksMd {
            frontmatter: TasksFrontmatter {
                spec: "SPEC-0001".to_owned(),
                spec_hash_at_generation: "bootstrap-pending".to_owned(),
                generated_at: "2026-05-11T00:00:00Z".to_owned(),
            },
            tasks,
            warnings: Vec::new(),
        }
    }

    fn make_task(id: &str, line: usize) -> Task {
        Task {
            id: id.to_owned(),
            title: "x".to_owned(),
            state: TaskState::Open,
            covers: Vec::new(),
            suggested_files: Vec::new(),
            notes: Vec::new(),
            line,
        }
    }

    #[test]
    fn parse_unqualified_accepts_minimum_3_digits() {
        let parsed = parse_ref("T-001").expect("T-001 must parse");
        assert!(matches!(parsed, TaskRef::Unqualified { id } if id == "T-001"));
        let parsed = parse_ref("T-1234").expect("T-1234 must parse");
        assert!(matches!(parsed, TaskRef::Unqualified { id } if id == "T-1234"));
    }

    #[test]
    fn parse_qualified_extracts_spec_and_task() {
        let parsed = parse_ref("SPEC-0001/T-001").expect("qualified must parse");
        assert!(
            matches!(
                &parsed,
                TaskRef::Qualified { spec_id, task_id }
                    if spec_id == "SPEC-0001" && task_id == "T-001",
            ),
            "expected Qualified{{SPEC-0001/T-001}}, got {parsed:?}",
        );
    }

    #[test]
    fn parse_rejects_short_task_id() {
        let err = parse_ref("T-1").expect_err("T-1 must fail (3+ digits required)");
        assert!(
            matches!(&err, super::LookupError::InvalidFormat { arg } if arg == "T-1"),
            "expected InvalidFormat{{T-1}}, got {err:?}",
        );
    }

    #[test]
    fn parse_rejects_garbage_inputs() {
        for bad in &["FOO", "T-", "T-AB", "SPEC-0001/FOO", "/T-001", "", "T- 001"] {
            let err = parse_ref(bad).expect_err("garbage input must fail");
            assert!(
                matches!(err, super::LookupError::InvalidFormat { ref arg } if arg == bad),
                "expected InvalidFormat carrying `{bad}`, got {err:?}",
            );
        }
    }

    #[test]
    fn extract_entry_pulls_task_line_and_sublist() {
        let raw = "---\nspec: SPEC-0001\nspec_hash_at_generation: x\ngenerated_at: y\n---\n\n# Tasks\n\n- [ ] **T-001**: first\n  - Covers: REQ-001\n  - Suggested files: `a.rs`\n\n- [ ] **T-002**: second\n";
        let tm = make_tasks_md(vec![make_task("T-001", 9), make_task("T-002", 13)]);
        let first_task = tm.tasks.first().expect("fixture has at least one task");
        let entry = extract_entry_from_raw(raw, &tm, first_task);
        assert!(entry.contains("**T-001**"));
        assert!(entry.contains("Covers: REQ-001"));
        assert!(entry.contains("Suggested files: `a.rs`"));
        assert!(!entry.contains("**T-002**"));
        assert!(
            !entry.ends_with('\n'),
            "trailing blank lines should be trimmed: {entry:?}",
        );
    }

    #[test]
    fn extract_entry_handles_last_task_in_file() {
        let raw = "---\nspec: SPEC-0001\nspec_hash_at_generation: x\ngenerated_at: y\n---\n\n# Tasks\n\n- [ ] **T-001**: only\n  - Covers: REQ-001\n";
        let tm = make_tasks_md(vec![make_task("T-001", 9)]);
        let first_task = tm.tasks.first().expect("fixture has at least one task");
        let entry = extract_entry_from_raw(raw, &tm, first_task);
        assert!(entry.contains("**T-001**"));
        assert!(entry.contains("Covers: REQ-001"));
    }

    #[test]
    fn extract_entry_stops_at_next_unindented_line() {
        let raw = "- [ ] **T-001**: a\n  - Covers: REQ-001\n\n## New section\nbody after\n- [ ] **T-002**: b\n";
        let tm = make_tasks_md(vec![make_task("T-001", 1), make_task("T-002", 6)]);
        let first_task = tm.tasks.first().expect("fixture has at least one task");
        let entry = extract_entry_from_raw(raw, &tm, first_task);
        assert!(entry.contains("**T-001**"));
        assert!(!entry.contains("New section"));
        assert!(!entry.contains("body after"));
    }
}
