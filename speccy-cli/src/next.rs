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
use crate::next_output::TerminalReason;
use crate::next_output::WORKSPACE_TERMINAL_REASON;
use crate::next_output::render_json_per_spec;
use crate::next_output::render_json_per_spec_with_reason;
use crate::next_output::render_json_workspace;
use crate::next_output::render_text_per_spec;
use crate::next_output::render_text_per_spec_with_reason;
use crate::next_output::render_text_workspace;
use crate::paths::to_repo_relative;
use camino::Utf8Path;
use speccy_core::consistency::ConsistencyBlock;
use speccy_core::consistency::ShellGitProbe;
use speccy_core::consistency::detect as detect_consistency;
use speccy_core::lint::ParsedSpec;
use speccy_core::next::compute_for_spec;
use speccy_core::next::compute_workspace;
use speccy_core::parse::SpecStatus;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan_with_archive;
use std::io::Write;
use thiserror::Error;

/// CLI exit code emitted when the per-spec form resolves to a terminal
/// state (SPEC-0043 REQ-003 / DEC-002). Matches the `speccy archive`
/// convention in the same binary.
pub const TERMINAL_EXIT_CODE: i32 = 2;

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
    /// Also include specs under `.speccy/archive/` in the scan, so the
    /// per-spec form can resolve archived spec IDs (returning their
    /// terminal `reason`). Archived specs are still filtered out of
    /// the workspace form because they carry REPORT.md.
    pub include_archive: bool,
    /// Whether to emit JSON instead of one-line text.
    pub json: bool,
}

/// Run `speccy next` from `cwd`, writing the rendered result to `out`
/// and any terminal-state stderr message to `err`. Returns the
/// dispatcher exit code (`0` for non-terminal, [`TERMINAL_EXIT_CODE`]
/// for terminal per-spec resolutions per SPEC-0043 REQ-003).
///
/// # Errors
///
/// Returns [`NextError::ProjectRootNotFound`] when no `.speccy/` is
/// found, [`NextError::Workspace`] on I/O during discovery,
/// [`NextError::SpecNotFound`] when the `spec_id` argument does not
/// resolve to a known spec, [`NextError::JsonSerialise`] if JSON
/// serialisation fails, or [`NextError::Io`] when writing to `out` or
/// `err` fails.
pub fn run(
    args: &NextArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, NextError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => {
            return Err(NextError::ProjectRootNotFound);
        }
        Err(other) => return Err(NextError::Workspace(other)),
    };
    let workspace = scan_with_archive(&project_root, args.include_archive);

    let (payload, exit_code) = if let Some(ref spec_id) = args.spec_id {
        run_per_spec(spec_id, &workspace, &project_root, args.json, err)?
    } else {
        run_workspace(&workspace, &project_root, args.json, err)?
    };

    out.write_all(payload.as_bytes()).map_err(NextError::Io)?;
    Ok(exit_code)
}

/// Resolve the workspace form: derive an action per active spec, or
/// emit a workspace-level terminal signal when no active specs remain.
///
/// Terminal signal (when `compute_workspace` returns no entries):
/// - JSON envelope carries `reason: "no_active_specs"`.
/// - Text form writes a one-line advisory to stderr.
/// - Both forms exit with [`TERMINAL_EXIT_CODE`] so an AI harness sees the
///   loop-stop signal without parsing stdout.
fn run_workspace(
    workspace: &speccy_core::workspace::Workspace,
    project_root: &Utf8Path,
    json: bool,
    err: &mut dyn Write,
) -> Result<(String, i32), NextError> {
    let raw_entries = compute_workspace(workspace);
    let probe = ShellGitProbe::new(project_root);
    let entries_with_paths: Vec<_> = raw_entries
        .into_iter()
        .map(|entry| {
            let spec_match = workspace
                .specs
                .iter()
                .find(|s| s.spec_id.as_deref() == Some(entry.spec_id.as_str()));
            let paths = spec_match.map_or_else(
                || SpecPaths {
                    spec_md_path: String::new(),
                    tasks_md_path: None,
                    mission_md_path: None,
                },
                |s| spec_paths(s, project_root),
            );
            let consistency = spec_match.map_or_else(ConsistencyBlock::ok, |s| {
                detect_consistency(&entry.spec_id, s, &probe)
            });
            (entry, paths, consistency)
        })
        .collect();
    let is_terminal = entries_with_paths.is_empty();
    let payload = if json {
        let envelope = render_json_workspace(&entries_with_paths);
        let mut text = serde_json::to_string(&envelope)?;
        text.push('\n');
        text
    } else if is_terminal {
        String::new()
    } else {
        render_text_workspace(&entries_with_paths)
    };
    let exit_code = if is_terminal {
        let line = format!(
            "speccy next: no active specs in workspace (reason: {WORKSPACE_TERMINAL_REASON}); run `speccy plan` to draft a new SPEC.\n",
        );
        err.write_all(line.as_bytes()).map_err(NextError::Io)?;
        TERMINAL_EXIT_CODE
    } else {
        0
    };
    Ok((payload, exit_code))
}

/// Resolve the per-spec form: classify terminal state, render the
/// payload, write any terminal stderr advisory, and return the
/// `(payload, exit_code)` pair.
fn run_per_spec(
    spec_id: &str,
    workspace: &speccy_core::workspace::Workspace,
    project_root: &Utf8Path,
    json: bool,
    err: &mut dyn Write,
) -> Result<(String, i32), NextError> {
    let Some(spec) = workspace
        .specs
        .iter()
        .find(|s| s.spec_id.as_deref() == Some(spec_id))
    else {
        return Err(NextError::SpecNotFound {
            spec_id: spec_id.to_owned(),
        });
    };

    // Short-circuit: SPEC frontmatter status overrides task-state
    // derivation. Dropped and superseded specs are terminal even
    // when `compute_for_spec` would otherwise return a non-None
    // action (SPEC-0043 REQ-003).
    let frontmatter_terminal = match spec.status_or_in_progress() {
        SpecStatus::Dropped => Some(TerminalReason::Dropped),
        SpecStatus::Superseded => Some(TerminalReason::Superseded),
        SpecStatus::InProgress | SpecStatus::Implemented => None,
    };

    let action = compute_for_spec(spec);
    // Frontmatter wins; otherwise `compute_for_spec` returning None
    // means REPORT.md is present (per SPEC-0043 REQ-002 after T-001).
    let terminal_reason =
        frontmatter_terminal.or_else(|| action.is_none().then_some(TerminalReason::Completed));

    // When the frontmatter signals dropped/superseded, the JSON and
    // text payloads must show `next_action: null` regardless of what
    // `compute_for_spec` derived from the task list.
    let effective_action = if frontmatter_terminal.is_some() {
        None
    } else {
        action.as_ref()
    };

    let paths = spec_paths(spec, project_root);
    let probe = ShellGitProbe::new(project_root);
    let consistency = detect_consistency(spec_id, spec, &probe);
    let payload = if json {
        let envelope = match terminal_reason {
            Some(reason) => render_json_per_spec_with_reason(
                spec_id,
                effective_action,
                reason,
                paths,
                consistency,
            ),
            None => render_json_per_spec(spec_id, effective_action, paths, consistency),
        };
        let mut text = serde_json::to_string(&envelope)?;
        text.push('\n');
        text
    } else {
        match terminal_reason {
            Some(reason) => render_text_per_spec_with_reason(spec_id, effective_action, reason),
            None => render_text_per_spec(spec_id, effective_action),
        }
    };

    let exit_code = if let Some(reason) = terminal_reason {
        let line = format!(
            "speccy next: {spec_id} is {reason}; run `speccy archive {spec_id}` to move it out of the active tree.\n",
            reason = reason.as_str(),
        );
        err.write_all(line.as_bytes()).map_err(NextError::Io)?;
        TERMINAL_EXIT_CODE
    } else {
        0
    };

    Ok((payload, exit_code))
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
