//! `speccy implement` command logic.
//!
//! Renders the Phase 3 implementer prompt for one task. The CLI never
//! invokes a model: it locates the task across the workspace via
//! [`speccy_core::task_lookup::find`], inlines SPEC.md / AGENTS.md /
//! task subtree / suggested-files into the embedded `implementer.md`
//! template, applies budget trimming, and writes the rendered prompt to
//! stdout.
//!
//! See `.speccy/specs/0008-implement-command/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::load_agents_md;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::slice_for_task;
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

    let agents = load_agents_md(&project_root);
    let template = load_template("implementer.md")?;

    let suggested_files = format_suggested_files(&location.task.suggested_files);
    // After SPEC-0019 REQ-005, prompt slicing reads `SpecDoc` and emits
    // only the requirements this task covers (plus frontmatter, summary,
    // and decision context). Falls back to the raw SPEC.md when the
    // marker tree failed to parse — the lint engine has already flagged
    // that as SPC-001, so the agent at least sees something to act on.
    let spec_md_slice = location.spec_doc.map_or_else(
        || location.spec_md.raw.clone(),
        |doc| slice_for_task(doc, &location.task.covers),
    );
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", location.spec_id.clone());
    vars.insert("spec_md", spec_md_slice);
    vars.insert("task_id", location.task.id.clone());
    vars.insert("task_entry", location.task_entry_raw.clone());
    vars.insert("suggested_files", suggested_files);
    vars.insert("agents", agents);

    let rendered = render(template, &vars);
    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
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
