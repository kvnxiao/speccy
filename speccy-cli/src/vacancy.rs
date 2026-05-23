//! `speccy vacancy [--json]` command logic.
//!
//! Walks `.speccy/specs/` (flat slug directories plus one level of
//! mission folders), finds the highest existing SPEC-NNNN, and returns
//! the next available ID. Text output is the bare `SPEC-NNNN\n` string;
//! `--json` output is `{"schema_version":1,"next_spec_id":"SPEC-NNNN"}\n`.
//! The command performs no filesystem writes.
//!
//! ID-walk logic is delegated to
//! [`speccy_core::prompt::allocate_next_spec_id`] (the same function
//! that drove ID allocation inside the now-deleted `speccy plan`
//! command). The open question deferred from T-001 — whether to
//! relocate `allocate_next_spec_id` out of `prompt::` — is resolved
//! here: the function is still the right tool (pure directory scan,
//! no prompt-rendering concern), and the `prompt::` namespace
//! retains it unchanged. No relocation is warranted for v1.
//!
//! See `.speccy/specs/0033-eject-prompt-bodies/SPEC.md` REQ-003.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::prompt::allocate_next_spec_id;
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
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
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
/// Resolves the workspace root, locates `.speccy/specs/`, delegates
/// the ID scan to [`allocate_next_spec_id`], and writes the result to
/// `out` in either text or JSON form.
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
    let next_digits = allocate_next_spec_id(&specs_dir);
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

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`VacancyError::Cwd`] if `std::env::current_dir` fails, or
/// [`VacancyError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, VacancyError> {
    let std_path = std::env::current_dir().map_err(VacancyError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| VacancyError::CwdNotUtf8)
}
