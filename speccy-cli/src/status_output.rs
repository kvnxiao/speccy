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
    /// Schema version. Pinned at `1` pre-v1; bump only when an external
    /// consumer of `1` exists and the shape must break.
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
    /// Repo-relative forward-slash path to `SPEC.md`
    /// (e.g. `.speccy/specs/0031-foo/SPEC.md`).
    pub spec_md_path: String,
    /// Repo-relative forward-slash path to `TASKS.md`, or `null` when
    /// TASKS.md is absent.
    pub tasks_md_path: Option<String>,
    /// Repo-relative forward-slash path to the mission folder's
    /// `MISSION.md`, or `null` for flat specs not grouped under a mission
    /// folder.
    pub mission_md_path: Option<String>,
    /// UTC archive date from frontmatter (`YYYY-MM-DD`). Omitted from
    /// the JSON output when the underlying frontmatter has no
    /// `archived_at` field — non-archived specs render byte-identically
    /// to pre-SPEC-0042 output. See SPEC-0042 REQ-007.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
    /// Free-form archive reason from frontmatter. Omitted when absent
    /// in the underlying frontmatter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_reason: Option<String>,
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
