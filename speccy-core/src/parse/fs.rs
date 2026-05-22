//! Filesystem helpers shared by the artifact parsers.
//!
//! Currently houses a single `read_to_string` wrapper that lifts
//! `fs_err::read_to_string` errors into [`crate::error::ParseError::Io`]
//! with the offending path attached. The wrapper is consumed by
//! [`crate::parse::spec_md`] and [`crate::workspace`]; it was relocated
//! here by SPEC-0040 when the surrounding TOML parser module was
//! deleted.

use crate::error::ParseError;
use crate::error::ParseResult;
use camino::Utf8Path;

/// Read a UTF-8 path into a `String`, wrapping I/O failures as
/// [`ParseError::Io`] with the offending path attached.
pub(crate) fn read_to_string(path: &Utf8Path) -> ParseResult<String> {
    fs_err::read_to_string(path.as_std_path()).map_err(|e| {
        Box::new(ParseError::Io {
            path: path.to_path_buf(),
            source: e,
        })
    })
}
