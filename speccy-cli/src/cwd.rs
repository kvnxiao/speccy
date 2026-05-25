//! Resolve the process current working directory as a `Utf8PathBuf`.
//!
//! Centralises cwd discovery so each command's error surface does not
//! carry its own `Cwd` / `CwdNotUtf8` variants. The non-UTF-8 case
//! folds into a synthesised [`std::io::Error`] so callers observe a
//! single error type.

use camino::Utf8PathBuf;

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
