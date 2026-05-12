//! JSON output types for `speccy status --json`.
//!
//! Hand-defined serde structs with declared field order so two runs
//! with no filesystem change produce byte-identical pretty-printed
//! output. See SPEC-0004 REQ-007.

use serde::Serialize;

/// Top-level JSON envelope. `schema_version` is the first field so
/// downstream consumers can sniff it cheaply.
#[derive(Debug, Clone, Serialize)]
pub struct JsonOutput {
    /// Schema version. Bumped on breaking changes.
    pub schema_version: u32,
    /// HEAD commit SHA, or `""` if unavailable.
    pub repo_sha: String,
    /// Every spec in workspace order (ascending spec ID).
    pub specs: Vec<JsonSpec>,
    /// Workspace-level lint diagnostics (those without a `spec_id`).
    pub lint: JsonLintBlock,
}

/// One spec entry inside [`JsonOutput::specs`].
#[derive(Debug, Clone, Serialize)]
pub struct JsonSpec {
    /// `SPEC-NNNN` identifier. Falls back to dir-derived form when
    /// frontmatter parsing fails.
    pub id: String,
    /// Folder-name slug. Empty string when frontmatter parse failed.
    pub slug: String,
    /// Title from frontmatter. `"<unparseable>"` on parse failure.
    pub title: String,
    /// Lifecycle status. `"unknown"` on parse failure.
    pub status: String,
    /// Frontmatter `supersedes` list.
    pub supersedes: Vec<String>,
    /// Inverse supersession: IDs of specs that replace this one.
    pub superseded_by: Vec<String>,
    /// Aggregated task state counts.
    pub tasks: JsonTaskCounts,
    /// Whether TASKS.md is stale relative to SPEC.md.
    pub stale: bool,
    /// Staleness reasons in declared order.
    pub stale_reasons: Vec<String>,
    /// Count of unchecked `## Open questions` bullets.
    pub open_questions: usize,
    /// Per-spec lint diagnostics.
    pub lint: JsonLintBlock,
    /// First parse error encountered, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

/// JSON-shaped task counts. Mirrors [`speccy_core::workspace::TaskCounts`].
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonTaskCounts {
    /// `[ ]` count.
    pub open: usize,
    /// `[~]` count.
    pub in_progress: usize,
    /// `[?]` count.
    pub awaiting_review: usize,
    /// `[x]` count.
    pub done: usize,
}

/// Grouped lint diagnostics for one spec (or workspace level).
#[derive(Debug, Clone, Default, Serialize)]
pub struct JsonLintBlock {
    /// Error-level diagnostics.
    pub errors: Vec<JsonDiagnostic>,
    /// Warn-level diagnostics.
    pub warnings: Vec<JsonDiagnostic>,
    /// Info-level diagnostics.
    pub info: Vec<JsonDiagnostic>,
}

/// One structured diagnostic.
#[derive(Debug, Clone, Serialize)]
pub struct JsonDiagnostic {
    /// Stable diagnostic code (e.g. `"SPC-001"`).
    pub code: String,
    /// Severity as a short string (`"error"`, `"warn"`, `"info"`).
    pub level: String,
    /// Human-readable message.
    pub message: String,
    /// File path the diagnostic points at, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// 1-indexed source line, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}
