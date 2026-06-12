//! Resolve the process current working directory as a `Utf8PathBuf`.
//!
//! Centralises cwd discovery so each command's error surface does not
//! carry its own `Cwd` / `CwdNotUtf8` variants. The non-UTF-8 case
//! folds into a synthesised [`std::io::Error`] so callers observe a
//! single error type. [`resolve_root`] carries the companion
//! project-root discovery step every workspace command shares.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;

/// Resolve the current working directory as a UTF-8 path.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] when
/// [`std::env::current_dir`] fails, or an
/// [`std::io::ErrorKind::InvalidData`] error when the resolved path
/// is not valid UTF-8.
pub fn resolve() -> Result<Utf8PathBuf, std::io::Error> {
    let std_path = std::env::current_dir()?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "current working directory is not valid UTF-8",
        )
    })
}

/// Resolve the project root by walking up from `cwd`, mapping the
/// no-`.speccy/`-found case to the command's `not_found` error and any
/// other discovery failure through the command error's
/// `From<WorkspaceError>` impl.
///
/// # Errors
///
/// Returns `not_found` when no `.speccy/` directory exists on the walk
/// up from `cwd`, or the converted [`WorkspaceError`] on I/O failure.
pub fn resolve_root<E: From<WorkspaceError>>(
    cwd: &Utf8Path,
    not_found: E,
) -> Result<Utf8PathBuf, E> {
    match find_root(cwd) {
        Ok(p) => Ok(p),
        Err(WorkspaceError::NoSpeccyDir { .. }) => Err(not_found),
        Err(other) => Err(E::from(other)),
    }
}
