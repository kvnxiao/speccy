//! `speccy next` command logic.
//!
//! Discovers the project root, scans the workspace, hands the parsed
//! [`Workspace`] to [`speccy_core::next::compute`], then renders the
//! result as text or JSON via [`crate::next_output`].
//!
//! See `.speccy/specs/0007-next-command/SPEC.md`.

use crate::next_output::render_json;
use crate::next_output::render_text;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::next::KindFilter;
use speccy_core::next::compute;
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
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
    /// JSON serialisation failed (unreachable for owned types; kept for
    /// future fields that could introduce error cases).
    #[error("failed to serialise next JSON")]
    JsonSerialise(#[from] serde_json::Error),
    /// I/O failure writing the rendered output to stdout.
    #[error("failed to write next output")]
    Io(#[source] std::io::Error),
}

/// `speccy next` arguments.
#[derive(Debug, Clone, Copy)]
pub struct NextArgs {
    /// Strict `--kind` filter, if supplied.
    pub kind: Option<KindFilter>,
    /// Whether to emit JSON instead of one-line text.
    pub json: bool,
}

/// Run `speccy next` from `cwd`, writing the rendered result to `out`.
///
/// # Errors
///
/// Returns [`NextError::ProjectRootNotFound`] when no `.speccy/` is
/// found, [`NextError::Workspace`] on I/O during discovery,
/// [`NextError::JsonSerialise`] if JSON serialisation fails, or
/// [`NextError::Io`] when writing to `out` fails.
pub fn run(args: NextArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), NextError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => {
            return Err(NextError::ProjectRootNotFound);
        }
        Err(other) => return Err(NextError::Workspace(other)),
    };
    let workspace = scan(&project_root);
    let result = compute(&workspace, args.kind);

    let payload = if args.json {
        let json = render_json(&result);
        let mut text = serde_json::to_string_pretty(&json)?;
        text.push('\n');
        text
    } else {
        render_text(&result)
    };

    out.write_all(payload.as_bytes()).map_err(NextError::Io)?;
    Ok(())
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`NextError::Cwd`] if `std::env::current_dir` fails, or
/// [`NextError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, NextError> {
    let std_path = std::env::current_dir().map_err(NextError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| NextError::CwdNotUtf8)
}
