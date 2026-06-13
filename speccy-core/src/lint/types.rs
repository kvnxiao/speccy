//! Public types for the lint engine.

use crate::error::ParseResult;
use crate::parse::ReportDoc;
use crate::parse::SpecDoc;
use crate::parse::SpecMd;
use crate::parse::TasksDoc;
use crate::parse::spec_md::SpecStatus;
use crate::parse::supersession::SupersessionIndex;
use camino::Utf8PathBuf;

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
    /// (typically by the `status` command).
    pub supersession: &'a SupersessionIndex,
}

/// One spec's parsed artifacts plus the path metadata the lint engine
/// needs to render diagnostics.
///
/// `spec_md`, `spec_doc`, and `tasks_md` are stored as `Result` so the
/// lint engine can emit diagnostics for parse failures (e.g. SPC-001
/// for a malformed SPEC.md element tree).
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
    /// Path to `TASKS.md`, if present.
    pub tasks_md_path: Option<Utf8PathBuf>,
    /// Path to the mission folder's `MISSION.md`, when this spec lives
    /// under a mission (focus) folder and `MISSION.md` exists there.
    /// `None` for flat specs directly under `.speccy/specs/`.
    pub mission_md_path: Option<Utf8PathBuf>,
    /// Parsed SPEC.md frontmatter / heading view (or the parse error).
    pub spec_md: ParseResult<SpecMd>,
    /// Parsed SPEC.md element tree (or the parse error). Carries the
    /// canonical requirement-to-scenario graph derived from
    /// `<requirement>` / `<scenario>` elements.
    pub spec_doc: ParseResult<SpecDoc>,
    /// Parsed TASKS.md typed XML model (or the parse error), if a
    /// TASKS.md exists.
    pub tasks_md: Option<ParseResult<TasksDoc>>,
    /// Parsed REPORT.md typed XML model (or the parse error), if a
    /// REPORT.md exists.
    pub report_md: Option<ParseResult<ReportDoc>>,
}

impl ParsedSpec {
    /// Convenience: return the parsed SPEC.md if parsing succeeded.
    #[must_use = "callers must handle the None case (parse failure)"]
    pub fn spec_md_ok(&self) -> Option<&SpecMd> {
        self.spec_md.as_ref().ok()
    }

    /// Convenience: return the parsed SPEC.md element tree if parsing
    /// succeeded.
    #[must_use = "callers must handle the None case (parse failure)"]
    pub fn spec_doc_ok(&self) -> Option<&SpecDoc> {
        self.spec_doc.as_ref().ok()
    }

    /// Convenience: return the parsed TASKS.md if present and parsed
    /// successfully.
    #[must_use = "callers must handle the None case (absent or parse failure)"]
    pub fn tasks_md_ok(&self) -> Option<&TasksDoc> {
        self.tasks_md.as_ref().and_then(|r| r.as_ref().ok())
    }

    /// Convenience: return the parsed REPORT.md if present and parsed
    /// successfully.
    #[must_use = "callers must handle the None case (absent or parse failure)"]
    pub fn report_md_ok(&self) -> Option<&ReportDoc> {
        self.report_md.as_ref().and_then(|r| r.as_ref().ok())
    }

    /// Convenience: return the SPEC.md `status` field, falling back to
    /// `SpecStatus::InProgress` when SPEC.md failed to parse. Mirrors
    /// the policy `speccy check` / `speccy verify` apply when deciding
    /// whether to render checks for a spec.
    #[must_use = "the status drives downstream rendering decisions"]
    pub fn status_or_in_progress(&self) -> SpecStatus {
        self.spec_md_ok()
            .map_or(SpecStatus::InProgress, |s| s.frontmatter.status)
    }

    /// Display label for the spec: the canonical `SPEC-NNNN` id when
    /// SPEC.md parsing yielded one, else the spec directory's basename
    /// (or the full path as a last resort). Used by `speccy check`
    /// diagnostics, where the rendered label must be stable regardless
    /// of parse success.
    #[must_use = "the returned label appears in diagnostics"]
    pub fn display_label(&self) -> String {
        self.spec_id.clone().unwrap_or_else(|| {
            self.dir
                .file_name()
                .map_or_else(|| self.dir.to_string(), ToOwned::to_owned)
        })
    }
}
