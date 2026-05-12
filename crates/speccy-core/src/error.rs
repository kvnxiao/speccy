//! Structured error type for every parser in `speccy-core`.
//!
//! All public parsers return [`Result<T, ParseError>`]. The variants
//! intentionally name the file path or label whenever it is available so
//! downstream consumers (`speccy status`, `speccy verify`, the lint engine)
//! can render actionable diagnostics without re-parsing the source file.

use camino::Utf8PathBuf;
use std::io;
use thiserror::Error;

/// Every failure mode reachable through [`crate::parse`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The file could not be read off disk.
    #[error("failed to read {path}")]
    Io {
        /// Path that failed to read.
        path: Utf8PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Path is not valid UTF-8. Speccy targets UTF-8 paths only outside OS
    /// boundaries; non-UTF-8 inputs are refused up front.
    #[error("path is not valid UTF-8: {0}")]
    NonUtf8Path(String),

    /// File content is not valid UTF-8. Speccy does not attempt encoding
    /// detection.
    #[error("{path} is not valid UTF-8")]
    NonUtf8File {
        /// Path that contained the invalid bytes.
        path: Utf8PathBuf,
    },

    /// The TOML payload at `path` could not be deserialised.
    #[error("failed to parse TOML in {path}: {message}")]
    Toml {
        /// Path that failed to parse.
        path: Utf8PathBuf,
        /// Underlying TOML error message.
        message: String,
    },

    /// The YAML frontmatter could not be deserialised.
    #[error("failed to parse YAML frontmatter{location}: {message}", location = location_suffix(.label.as_deref()))]
    Yaml {
        /// Optional file or section label.
        label: Option<String>,
        /// Underlying YAML error message.
        message: String,
    },

    /// `schema_version` declared in a TOML config is not a supported value.
    #[error(
        "{path} declares unsupported schema_version = {value}; speccy supports schema_version = 1"
    )]
    UnsupportedSchemaVersion {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Value found in the file.
        value: i64,
    },

    /// A required field was missing from a parsed payload.
    #[error("missing required field `{field}` in {context}")]
    MissingField {
        /// Field that was expected.
        field: String,
        /// Description of the parent table or struct.
        context: String,
    },

    /// A `[[checks]]` entry violated the command/prompt invariant.
    #[error("check `{check_id}` in {path} is invalid: {reason}")]
    InvalidCheckEntry {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// `id` of the offending check.
        check_id: String,
        /// Human-readable reason.
        reason: String,
    },

    /// A markdown file declared an opening `---` fence but no closing one.
    #[error("unterminated YAML frontmatter in {path}")]
    UnterminatedFrontmatter {
        /// Path of the offending file.
        path: Utf8PathBuf,
    },

    /// A markdown frontmatter field carried a value outside the allowed
    /// closed set (e.g. SPEC.md `status` or REPORT.md `outcome`).
    #[error("invalid value for `{field}` in {path}: `{value}` is not one of {allowed}")]
    InvalidEnumValue {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Field that carried the invalid value.
        field: String,
        /// Offending value.
        value: String,
        /// Comma-separated list of allowed values.
        allowed: String,
    },
}

fn location_suffix(label: Option<&str>) -> String {
    match label {
        Some(label) => format!(" in {label}"),
        None => String::new(),
    }
}
