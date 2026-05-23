//! `speccy vacancy [--json]` command logic.
//!
//! Walks `.speccy/specs/` and `.speccy/archive/` (flat slug
//! directories plus one level of mission folders), finds the highest
//! existing SPEC-NNNN across both, and returns the next available ID.
//! Text output is the bare `SPEC-NNNN\n` string; `--json` output is
//! `{"schema_version":1,"next_spec_id":"SPEC-NNNN"}\n`. The command
//! performs no filesystem writes.
//!
//! ID-walk logic is delegated to
//! [`speccy_core::prompt::allocate_next_spec_id_across_dirs`] so the
//! scan unions the active and archive directories per SPEC-0042
//! REQ-005 (archived specs retain their SPEC-NNNN slots). A missing
//! `.speccy/archive/` is treated as empty by the helper, so an
//! absent archive directory is the no-op default.
//!
//! See `.speccy/specs/0033-eject-prompt-bodies/SPEC.md` REQ-003 and
//! `.speccy/specs/0042-archive-completed-specs/SPEC.md` REQ-005.

use camino::Utf8Path;
use speccy_core::prompt::allocate_next_spec_id_across_dirs;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VacancyError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// I/O failure writing to stdout.
    #[error("failed to write output: {0}")]
    Io(#[from] std::io::Error),
}

/// `speccy vacancy` arguments.
#[derive(Debug, Clone)]
pub struct VacancyArgs {
    /// Emit JSON envelope instead of bare text.
    pub json: bool,
}

/// Run `speccy vacancy` from `cwd`, writing to `out`.
///
/// Resolves the workspace root, locates `.speccy/specs/` and
/// `.speccy/archive/`, delegates the ID scan to
/// [`allocate_next_spec_id_across_dirs`] (which unions both
/// directories per SPEC-0042 REQ-005), and writes the result to `out`
/// in either text or JSON form.
///
/// # Errors
///
/// Returns [`VacancyError::ProjectRootNotFound`] when no `.speccy/`
/// directory exists in the cwd ancestry. Returns
/// [`VacancyError::Io`] if writing to `out` fails.
pub fn run(args: &VacancyArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), VacancyError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(VacancyError::ProjectRootNotFound),
        Err(other) => return Err(VacancyError::Workspace(other)),
    };

    let specs_dir = project_root.join(".speccy").join("specs");
    let archive_dir = project_root.join(".speccy").join("archive");
    // Per SPEC-0042 REQ-005: archived specs retain their SPEC-NNNN
    // slots, so the vacancy scan unions the active and archive
    // directories. `allocate_next_spec_id_across_dirs` treats a
    // missing directory as empty, so an absent `.speccy/archive/`
    // is the no-op default.
    let next_digits =
        allocate_next_spec_id_across_dirs(&[specs_dir.as_path(), archive_dir.as_path()]);
    let next_id = format!("SPEC-{next_digits}");

    if args.json {
        writeln!(
            out,
            "{{\"schema_version\":1,\"next_spec_id\":\"{next_id}\"}}",
        )?;
    } else {
        writeln!(out, "{next_id}")?;
    }

    Ok(())
}
