//! `speccy next` command logic.
//!
//! Supports two call shapes:
//!
//! - **Workspace form** (`speccy next`): calls [`compute_workspace`] to list
//!   every active spec with its derived [`NextAction`].
//! - **Per-spec form** (`speccy next SPEC-NNNN`): looks up the spec and calls
//!   [`compute_for_spec`] to return one entry or a null-action reason.
//!
//! See `.speccy/specs/0033-eject-prompt-bodies/SPEC.md` REQ-004.

use crate::next_output::SpecPaths;
use crate::next_output::render_json_per_spec;
use crate::next_output::render_json_workspace;
use crate::next_output::render_text_per_spec;
use crate::next_output::render_text_workspace;
use crate::paths::to_repo_relative;
use camino::Utf8Path;
use speccy_core::lint::ParsedSpec;
use speccy_core::next::compute_for_spec;
use speccy_core::next::compute_workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NextError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// The requested SPEC-ID was not found in the workspace.
    #[error("spec `{spec_id}` not found under .speccy/specs/")]
    SpecNotFound {
        /// The spec ID that was requested.
        spec_id: String,
    },
    /// JSON serialisation failed.
    #[error("failed to serialise next JSON")]
    JsonSerialise(#[from] serde_json::Error),
    /// I/O failure writing the rendered output to stdout.
    #[error("failed to write next output")]
    Io(#[source] std::io::Error),
}

/// `speccy next` arguments.
#[derive(Debug)]
pub struct NextArgs {
    /// Optional `SPEC-NNNN` selector for the per-spec form.
    pub spec_id: Option<String>,
    /// Whether to emit JSON instead of one-line text.
    pub json: bool,
}

/// Run `speccy next` from `cwd`, writing the rendered result to `out`.
///
/// # Errors
///
/// Returns [`NextError::ProjectRootNotFound`] when no `.speccy/` is
/// found, [`NextError::Workspace`] on I/O during discovery,
/// [`NextError::SpecNotFound`] when the `spec_id` argument does not
/// resolve to a known spec, [`NextError::JsonSerialise`] if JSON
/// serialisation fails, or [`NextError::Io`] when writing to `out`
/// fails.
pub fn run(args: &NextArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), NextError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => {
            return Err(NextError::ProjectRootNotFound);
        }
        Err(other) => return Err(NextError::Workspace(other)),
    };
    let workspace = scan(&project_root);

    let payload = if let Some(ref spec_id) = args.spec_id {
        // Per-spec form.
        let spec = workspace
            .specs
            .iter()
            .find(|s| s.spec_id.as_deref() == Some(spec_id.as_str()));
        let Some(spec) = spec else {
            return Err(NextError::SpecNotFound {
                spec_id: spec_id.clone(),
            });
        };
        let action = compute_for_spec(spec);
        if args.json {
            let paths = spec_paths(spec, &project_root);
            let json = render_json_per_spec(spec_id, action.as_ref(), paths);
            let mut text = serde_json::to_string(&json)?;
            text.push('\n');
            text
        } else {
            render_text_per_spec(spec_id, action.as_ref())
        }
    } else {
        // Workspace form.
        let raw_entries = compute_workspace(&workspace);
        let entries_with_paths: Vec<_> = raw_entries
            .into_iter()
            .map(|entry| {
                let paths = workspace
                    .specs
                    .iter()
                    .find(|s| s.spec_id.as_deref() == Some(entry.spec_id.as_str()))
                    .map_or_else(
                        || SpecPaths {
                            spec_md_path: String::new(),
                            tasks_md_path: None,
                            mission_md_path: None,
                        },
                        |s| spec_paths(s, &project_root),
                    );
                (entry, paths)
            })
            .collect();
        if args.json {
            let json = render_json_workspace(&entries_with_paths);
            let mut text = serde_json::to_string(&json)?;
            text.push('\n');
            text
        } else {
            render_text_workspace(&entries_with_paths)
        }
    };

    out.write_all(payload.as_bytes()).map_err(NextError::Io)?;
    Ok(())
}

/// Build a [`SpecPaths`] from a parsed spec, making paths repo-relative
/// forward-slash strings by stripping the `project_root` prefix.
fn spec_paths(spec: &ParsedSpec, project_root: &Utf8Path) -> SpecPaths {
    SpecPaths {
        spec_md_path: to_repo_relative(&spec.spec_md_path, project_root),
        tasks_md_path: spec
            .tasks_md_path
            .as_ref()
            .map(|p| to_repo_relative(p, project_root)),
        mission_md_path: spec
            .mission_md_path
            .as_ref()
            .map(|p| to_repo_relative(p, project_root)),
    }
}
