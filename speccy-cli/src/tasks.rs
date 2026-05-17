//! `speccy tasks` command logic.
//!
//! Two prompt-rendering forms (form auto-detected by TASKS.md presence)
//! plus the `--commit` sub-action that records SPEC.md hash + UTC
//! timestamp into TASKS.md frontmatter while preserving body bytes.
//!
//! - `speccy tasks SPEC-NNNN` — TASKS.md absent → render `tasks-generate.md`;
//!   present → render `tasks-amend.md`.
//! - `speccy tasks SPEC-NNNN --commit` — rewrite frontmatter via
//!   [`speccy_core::tasks::commit_frontmatter`].
//!
//! See `.speccy/specs/0006-tasks-command/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use jiff::Timestamp;
use regex::Regex;
use speccy_core::ParseError;
use speccy_core::parse::SpecMd;
use speccy_core::parse::parse_task_xml;
use speccy_core::parse::spec_md;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::load_agents_md;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::trim_to_budget;
use speccy_core::tasks::CommitError;
use speccy_core::tasks::commit_frontmatter;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::OnceLock;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TasksError {
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
    /// SPEC.md or TASKS.md parse failure.
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
    /// Template lookup or substitution helper failed.
    #[error("prompt template error")]
    Prompt(#[from] PromptError),
    /// `--commit` sub-action failure.
    #[error("--commit failed")]
    Commit(#[from] CommitError),
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
    /// I/O failure while writing the rendered prompt to stdout.
    #[error("failed to write rendered prompt to stdout")]
    Io(#[from] std::io::Error),
}

/// `speccy tasks` arguments.
#[derive(Debug, Clone)]
pub struct TasksArgs {
    /// The `SPEC-NNNN` argument (required).
    pub spec_id: String,
    /// `true` when `--commit` is passed; toggles file-writing mode.
    pub commit: bool,
}

/// Run `speccy tasks` from `cwd`, writing the rendered prompt to `out`.
///
/// When `args.commit` is true, no prompt is rendered; the file mutation
/// is the sole side effect.
///
/// # Errors
///
/// Returns any [`TasksError`] variant if discovery, parsing, template
/// loading, rendering, or commit-mutation fails.
pub fn run(args: TasksArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), TasksError> {
    let TasksArgs { spec_id, commit } = args;
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(TasksError::ProjectRootNotFound),
        Err(other) => return Err(TasksError::Workspace(other)),
    };

    let canonical_id = validate_spec_id(&spec_id)?;
    let spec_dir = locate_spec_dir(&project_root, &canonical_id)?;
    let spec_md_path = spec_dir.join("SPEC.md");
    let tasks_md_path = spec_dir.join("TASKS.md");
    let tasks_present = fs_err::metadata(tasks_md_path.as_std_path()).is_ok_and(|m| m.is_file());

    let parsed_spec = spec_md(&spec_md_path).map_err(|source| TasksError::Parse {
        artifact: "SPEC.md",
        id: canonical_id.clone(),
        source: Box::new(source),
    })?;

    if commit {
        if !tasks_present {
            return Err(TasksError::Commit(CommitError::TasksMdNotFound {
                path: tasks_md_path,
            }));
        }
        let now = Timestamp::now();
        commit_frontmatter(&tasks_md_path, &canonical_id, &parsed_spec.sha256, now)?;
        return Ok(());
    }

    let rendered = if tasks_present {
        render_amendment(&project_root, &canonical_id, &parsed_spec, &tasks_md_path)?
    } else {
        render_initial(&project_root, &canonical_id, &parsed_spec)?
    };

    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn render_initial(
    project_root: &Utf8Path,
    canonical_id: &str,
    parsed_spec: &SpecMd,
) -> Result<String, TasksError> {
    let agents = load_agents_md(project_root);
    let template = load_template("tasks-generate.md")?;
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", canonical_id.to_owned());
    vars.insert("spec_md", parsed_spec.raw.clone());
    vars.insert("agents", agents);
    Ok(render(template, &vars))
}

fn render_amendment(
    project_root: &Utf8Path,
    canonical_id: &str,
    parsed_spec: &SpecMd,
    tasks_md_path: &Utf8Path,
) -> Result<String, TasksError> {
    // Parse TASKS.md to validate it is well-formed; on failure return a
    // typed error. The rendered prompt inlines the raw bytes (not the
    // parsed structure) so the agent reads exactly what's on disk.
    let tasks_raw = fs_err::read_to_string(tasks_md_path.as_std_path())?;
    parse_task_xml(&tasks_raw, tasks_md_path).map_err(|source| TasksError::Parse {
        artifact: "TASKS.md",
        id: canonical_id.to_owned(),
        source: Box::new(source),
    })?;

    let agents = load_agents_md(project_root);
    let template = load_template("tasks-amend.md")?;
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", canonical_id.to_owned());
    vars.insert("spec_md", parsed_spec.raw.clone());
    vars.insert("tasks_md", tasks_raw);
    vars.insert("agents", agents);
    Ok(render(template, &vars))
}

fn validate_spec_id(raw: &str) -> Result<String, TasksError> {
    if !spec_id_regex().is_match(raw) {
        return Err(TasksError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

fn locate_spec_dir(project_root: &Utf8Path, canonical_id: &str) -> Result<Utf8PathBuf, TasksError> {
    let digits =
        canonical_id
            .strip_prefix("SPEC-")
            .ok_or_else(|| TasksError::InvalidSpecIdFormat {
                arg: canonical_id.to_owned(),
            })?;
    let specs_dir = project_root.join(".speccy").join("specs");
    let read =
        fs_err::read_dir(specs_dir.as_std_path()).map_err(|_io| TasksError::SpecNotFound {
            id: canonical_id.to_owned(),
        })?;
    let prefix = format!("{digits}-");
    for entry in read.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let Some(name) = entry
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        if name.starts_with(&prefix) {
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            return Ok(path);
        }
    }
    Err(TasksError::SpecNotFound {
        id: canonical_id.to_owned(),
    })
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn spec_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^SPEC-\d{4,}$").unwrap())
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`TasksError::Cwd`] if `std::env::current_dir` fails, or
/// [`TasksError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, TasksError> {
    let std_path = std::env::current_dir().map_err(TasksError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| TasksError::CwdNotUtf8)
}

#[cfg(test)]
mod tests {
    use super::spec_id_regex;
    use super::validate_spec_id;

    #[test]
    fn valid_spec_ids_pass_regex() {
        assert!(spec_id_regex().is_match("SPEC-0006"));
        assert!(spec_id_regex().is_match("SPEC-1234"));
        assert!(spec_id_regex().is_match("SPEC-10000"));
    }

    #[test]
    fn invalid_spec_ids_rejected() {
        validate_spec_id("FOO").expect_err("`FOO` must fail format validation");
        validate_spec_id("SPEC-1").expect_err("`SPEC-1` has fewer than 4 digits");
        validate_spec_id("spec-0001").expect_err("lowercase prefix must be rejected");
        validate_spec_id("SPEC-").expect_err("missing digits must be rejected");
    }
}
