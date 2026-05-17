//! Text + JSON renderers for `speccy verify`.
//!
//! SPEC-0018 REQ-003: verify is shape-only. The text summary reports
//! counts and an error tally; the JSON envelope bumps `schema_version`
//! to `2` and exposes structural counts. The legacy execution-shaped
//! fields (`outcome`, `exit_code`, `duration_ms`) are gone.

use crate::status_output::JsonLintBlock;
use crate::verify::VerifyError;
use crate::verify::VerifyReport;
use serde::Serialize;
use speccy_core::lint::Diagnostic;
use std::io::Write;

/// JSON schema version emitted by `speccy verify --json`. Bumped from
/// `1` to `2` when SPEC-0018 removed the execution-shaped fields.
pub const JSON_SCHEMA_VERSION: u32 = 2;

/// Render the text summary to `out`.
///
/// Output:
///
/// ```text
/// Lint: <E> errors, <W> warnings, <I> info
/// verified <N> specs, <M> requirements, <K> scenarios; <E> errors
/// ```
///
/// `E` in both lines is the gating error count (in-progress demotions
/// already applied).
///
/// # Errors
///
/// Propagates I/O errors from writing to `out`.
pub fn write_text(report: &VerifyReport, out: &mut dyn Write) -> std::io::Result<()> {
    writeln!(
        out,
        "Lint: {e} errors, {w} warnings, {i} info",
        e = report.lint_errors.len(),
        w = report.lint_warnings.len(),
        i = report.lint_info.len(),
    )?;
    writeln!(
        out,
        "verified {n} specs, {m} requirements, {k} scenarios; {e} errors",
        n = report.specs_total,
        m = report.requirements_total,
        k = report.scenarios_total,
        e = report.lint_errors.len(),
    )?;
    Ok(())
}

/// Render the JSON envelope (pretty-printed, trailing newline) to `out`.
///
/// `schema_version = 2`. The envelope intentionally omits per-check
/// execution fields (`outcome`, `exit_code`, `duration_ms`) — speccy
/// no longer runs scenarios.
///
/// # Errors
///
/// Returns [`VerifyError::JsonSerialise`] on serialisation failure or
/// [`VerifyError::Io`] on write failure.
pub fn write_json(report: &VerifyReport, out: &mut dyn Write) -> Result<(), VerifyError> {
    let payload = JsonOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_sha: report.repo_sha.clone(),
        lint: lint_block(report),
        summary: JsonSummary {
            lint: JsonLintCounts {
                errors: report.lint_errors.len(),
                warnings: report.lint_warnings.len(),
                info: report.lint_info.len(),
            },
            shape: JsonShapeCounts {
                specs: report.specs_total,
                requirements: report.requirements_total,
                scenarios: report.scenarios_total,
                errors: report.lint_errors.len(),
            },
        },
        passed: report.passed(),
    };
    let mut text = serde_json::to_string_pretty(&payload)?;
    text.push('\n');
    out.write_all(text.as_bytes())?;
    Ok(())
}

fn lint_block(report: &VerifyReport) -> JsonLintBlock {
    let mut combined: Vec<Diagnostic> = Vec::with_capacity(
        report.lint_errors.len() + report.lint_warnings.len() + report.lint_info.len(),
    );
    combined.extend(report.lint_errors.iter().cloned());
    combined.extend(report.lint_warnings.iter().cloned());
    combined.extend(report.lint_info.iter().cloned());
    JsonLintBlock::from_diagnostics(&combined)
}

/// Top-level JSON envelope.
#[derive(Debug, Clone, Serialize)]
pub struct JsonOutput {
    /// Schema version. Bumped on breaking changes. SPEC-0018 set this
    /// to `2`.
    pub schema_version: u32,
    /// HEAD commit SHA, or `""` if unavailable.
    pub repo_sha: String,
    /// Grouped lint diagnostics (errors / warnings / info).
    pub lint: JsonLintBlock,
    /// Aggregate counts.
    pub summary: JsonSummary,
    /// `true` iff exit code is 0.
    pub passed: bool,
}

/// Aggregate lint and shape counts.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonSummary {
    /// Lint counts by level.
    pub lint: JsonLintCounts,
    /// Structural counts (specs, requirements, scenarios, errors).
    pub shape: JsonShapeCounts,
}

/// Lint count buckets.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonLintCounts {
    /// Error-level diagnostic count (gating, after demotion).
    pub errors: usize,
    /// Warn-level diagnostic count.
    pub warnings: usize,
    /// Info-level diagnostic count (includes demoted in-progress errors).
    pub info: usize,
}

/// Shape counts mirrored from the text summary.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonShapeCounts {
    /// Number of specs walked.
    pub specs: usize,
    /// Total `[[requirements]]` rows in non-defunct specs.
    pub requirements: usize,
    /// Total `[[checks]]` (scenarios) rows in non-defunct specs.
    pub scenarios: usize,
    /// Gating error count (mirrors `summary.lint.errors`).
    pub errors: usize,
}
