//! `speccy implement` command logic.
//!
//! Renders the Phase 3 implementer prompt for one task. The CLI never
//! invokes a model: it locates the task across the workspace via
//! [`speccy_core::task_lookup::find`] and inlines the task subtree and
//! suggested-files list into the embedded `implementer.md` template,
//! applies budget trimming, and writes the rendered prompt to stdout.
//!
//! SPEC-0023 REQ-005 retired the inlined-`AGENTS.md` flow and REQ-006
//! retired the inlined-`SPEC.md` flow: modern AI coding harnesses
//! auto-load `AGENTS.md` themselves, and every harness ships a Read
//! primitive the agent uses to fetch SPEC.md by path on demand. The
//! rendered prompt names the SPEC.md repo-relative path; the body is
//! no longer inlined.
//!
//! See `.speccy/specs/0008-implement-command/SPEC.md` and
//! `.speccy/specs/0023-single-phase-skill-primitives/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::trim_to_budget;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::collections::BTreeMap;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ImplementError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Argument parsing or workspace lookup failed.
    #[error(transparent)]
    Lookup(#[from] LookupError),
    /// Template lookup or substitution helper failed.
    #[error("prompt template error")]
    Prompt(#[from] PromptError),
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

/// `speccy implement` arguments.
#[derive(Debug, Clone)]
pub struct ImplementArgs {
    /// The `T-NNN` or `SPEC-NNNN/T-NNN` reference (required).
    pub task_ref: String,
}

/// Run `speccy implement` from `cwd`, writing the rendered prompt to
/// `out`.
///
/// # Errors
///
/// Returns any [`ImplementError`] variant if discovery, lookup,
/// template loading, rendering, or I/O fails.
pub fn run(
    args: &ImplementArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
) -> Result<(), ImplementError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => {
            return Err(ImplementError::ProjectRootNotFound);
        }
        Err(other) => return Err(ImplementError::Workspace(other)),
    };

    let task_ref: TaskRef = parse_ref(&args.task_ref)?;
    let workspace = scan(&project_root);
    let location = find(&workspace, &task_ref)?;

    let template = load_template("implementer.md")?;

    let suggested_files = format_suggested_files(&location.task.suggested_files());
    // SPEC-0023 REQ-006: SPEC.md is no longer inlined. The rendered
    // prompt names the repo-relative path; the agent reads the file via
    // the host's Read primitive on demand.
    let spec_md_path = spec_md_path_relative(&project_root, location.spec_dir);
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", location.spec_id.clone());
    vars.insert("spec_md_path", spec_md_path);
    vars.insert("task_id", location.task.id.clone());
    vars.insert("task_entry", location.task_entry_raw.clone());
    vars.insert("suggested_files", suggested_files);

    let rendered = render(template, &vars);
    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

/// Compute the repo-relative path to `<spec_dir>/SPEC.md`.
///
/// `project_root` is the absolute path to the project root (where
/// `.speccy/` lives); `spec_dir` is the absolute path to the spec
/// directory. Returns a forward-slash path string suitable for embedding
/// in the rendered prompt; falls back to the absolute spec path string
/// if the relative computation fails (which would only happen if
/// `spec_dir` were not under `project_root`, a configuration the
/// workspace scanner does not produce).
fn spec_md_path_relative(project_root: &Utf8Path, spec_dir: &Utf8Path) -> String {
    let relative = spec_dir.strip_prefix(project_root).unwrap_or(spec_dir);
    relative.join("SPEC.md").as_str().replace('\\', "/")
}

fn format_suggested_files(files: &[String]) -> String {
    files
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`ImplementError::Cwd`] if `std::env::current_dir` fails, or
/// [`ImplementError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, ImplementError> {
    let std_path = std::env::current_dir().map_err(ImplementError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| ImplementError::CwdNotUtf8)
}

#[cfg(test)]
mod tests {
    use super::format_suggested_files;

    #[test]
    fn format_suggested_files_csv() {
        let files = vec!["a.rs".to_owned(), "b.rs".to_owned()];
        assert_eq!(format_suggested_files(&files), "a.rs, b.rs");
    }

    #[test]
    fn format_suggested_files_empty() {
        assert_eq!(format_suggested_files(&[]), "");
    }

    #[test]
    fn format_suggested_files_single() {
        let files = vec!["only.rs".to_owned()];
        assert_eq!(format_suggested_files(&files), "only.rs");
    }
}
