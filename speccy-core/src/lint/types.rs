//! Public types for the lint engine.

use crate::error::ParseError;
use crate::parse::SpecMd;
use crate::parse::SpecToml;
use crate::parse::TasksMd;
use crate::parse::supersession::SupersessionIndex;
use camino::Utf8PathBuf;
use std::time::SystemTime;

/// Severity of a lint diagnostic. Used by `speccy verify` to map
/// diagnostics onto exit-code policy (Error -> 1; Warn and Info -> 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Hard failure: `speccy verify` exits non-zero.
    Error,
    /// Soft signal: surfaced in `speccy status` and `speccy verify` but
    /// non-fatal.
    Warn,
    /// Informational: surfaced for awareness.
    Info,
}

impl Level {
    /// Render the severity as a short string (e.g. `error`).
    #[must_use = "the rendered severity is the on-disk / on-wire form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
        }
    }
}

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    /// Stable code, e.g. `"SPC-001"`. `&'static str` so the catalogue is
    /// compile-time and inexpensive.
    pub code: &'static str,
    /// Severity.
    pub level: Level,
    /// Human-readable message.
    pub message: String,
    /// Spec the diagnostic belongs to, if any.
    pub spec_id: Option<String>,
    /// File the diagnostic points at, if any.
    pub file: Option<Utf8PathBuf>,
    /// 1-indexed source line, if any.
    pub line: Option<u32>,
}

impl Diagnostic {
    /// Construct a diagnostic with no file/line attached.
    #[must_use = "the constructed diagnostic must be emitted"]
    pub fn spec_only(
        code: &'static str,
        level: Level,
        spec_id: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            level,
            message: message.into(),
            spec_id: spec_id.into(),
            file: None,
            line: None,
        }
    }

    /// Construct a diagnostic with a file path attached.
    #[must_use = "the constructed diagnostic must be emitted"]
    pub fn with_file(
        code: &'static str,
        level: Level,
        spec_id: impl Into<Option<String>>,
        file: Utf8PathBuf,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            level,
            message: message.into(),
            spec_id: spec_id.into(),
            file: Some(file),
            line: None,
        }
    }

    /// Construct a diagnostic with a file path and source line attached.
    #[must_use = "the constructed diagnostic must be emitted"]
    pub fn with_location(
        code: &'static str,
        level: Level,
        spec_id: impl Into<Option<String>>,
        file: Utf8PathBuf,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            level,
            message: message.into(),
            spec_id: spec_id.into(),
            file: Some(file),
            line: Some(line),
        }
    }
}

/// Workspace handed to [`crate::lint::run`].
///
/// The workspace bundles every parsed spec under `.speccy/specs/` plus
/// a borrowed [`SupersessionIndex`] computed across all SPEC.md files.
/// Both fields are borrowed so the owning workspace
/// ([`crate::workspace::Workspace`]) can be reused for rendering after the lint
/// pass.
#[derive(Debug)]
pub struct Workspace<'a> {
    /// Every spec directory discovered under `.speccy/specs/`, parsed.
    pub specs: &'a [ParsedSpec],
    /// Inverse `supersedes` relation. Computed once per workspace scan
    /// (typically by SPEC-0004 `status`).
    pub supersession: &'a SupersessionIndex,
}

/// One spec's parsed artifacts plus the path metadata the lint engine
/// needs to render diagnostics.
///
/// `spec_md`, `spec_toml`, and `tasks_md` are stored as `Result` so the
/// lint engine can emit diagnostics for parse failures (e.g. SPC-001 for
/// a malformed `spec.toml`).
#[derive(Debug)]
pub struct ParsedSpec {
    /// Stable `SPEC-NNNN` id pulled from the SPEC.md frontmatter when
    /// parsing succeeded.
    pub spec_id: Option<String>,
    /// Path to the spec directory (e.g.
    /// `.speccy/specs/0001-artifact-parsers`).
    pub dir: Utf8PathBuf,
    /// Path to `SPEC.md`.
    pub spec_md_path: Utf8PathBuf,
    /// Path to `spec.toml`.
    pub spec_toml_path: Utf8PathBuf,
    /// Path to `TASKS.md`, if present.
    pub tasks_md_path: Option<Utf8PathBuf>,
    /// Parsed SPEC.md (or the parse error).
    pub spec_md: Result<SpecMd, ParseError>,
    /// Parsed spec.toml (or the parse error).
    pub spec_toml: Result<SpecToml, ParseError>,
    /// Parsed TASKS.md (or the parse error), if a TASKS.md exists.
    pub tasks_md: Option<Result<TasksMd, ParseError>>,
    /// Modification time of `SPEC.md`, captured by the workspace
    /// scanner. Used by TSK-003 mtime drift detection. `None` if mtime
    /// could not be read.
    pub spec_md_mtime: Option<SystemTime>,
    /// Modification time of `TASKS.md`, captured by the workspace
    /// scanner. `None` if absent or unreadable.
    pub tasks_md_mtime: Option<SystemTime>,
}

impl ParsedSpec {
    /// Convenience: return the parsed SPEC.md if parsing succeeded.
    #[must_use = "callers must handle the None case (parse failure)"]
    pub fn spec_md_ok(&self) -> Option<&SpecMd> {
        self.spec_md.as_ref().ok()
    }

    /// Convenience: return the parsed spec.toml if parsing succeeded.
    #[must_use = "callers must handle the None case (parse failure)"]
    pub fn spec_toml_ok(&self) -> Option<&SpecToml> {
        self.spec_toml.as_ref().ok()
    }

    /// Convenience: return the parsed TASKS.md if present and parsed
    /// successfully.
    #[must_use = "callers must handle the None case (absent or parse failure)"]
    pub fn tasks_md_ok(&self) -> Option<&TasksMd> {
        self.tasks_md.as_ref().and_then(|r| r.as_ref().ok())
    }
}
