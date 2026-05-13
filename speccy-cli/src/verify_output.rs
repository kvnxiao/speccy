//! Text + JSON renderers for `speccy verify`.
//!
//! See `.speccy/specs/0012-verify-command/SPEC.md` REQ-004 (text) and
//! REQ-005 (JSON).

use crate::status_output::JsonLintBlock;
use crate::verify::VerifyError;
use crate::verify::VerifyReport;
use serde::Serialize;
use speccy_core::exec::CheckResult;
use speccy_core::lint::Diagnostic;
use std::io::Write;

/// Render the three-line text summary to `out`.
///
/// Output:
///
/// ```text
/// Lint: <E> errors, <W> warnings, <I> info
/// Checks: <P> passed, <F> failed, <FL> in-flight, <M> manual
/// verify: PASS|FAIL
/// ```
///
/// `failed` counts only `Fail` outcomes on `implemented` specs (which
/// gate the exit code); `in-flight` counts `Fail` outcomes on
/// `in-progress` specs (informational, do not gate). See SPEC-0012.
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
        "Checks: {p} passed, {f} failed, {fl} in-flight, {m} manual",
        p = report.passed_checks(),
        f = report.failed_checks(),
        fl = report.in_flight_checks(),
        m = report.manual_checks(),
    )?;
    let verdict = if report.passed() { "PASS" } else { "FAIL" };
    writeln!(out, "verify: {verdict}")?;
    Ok(())
}

/// Render the JSON envelope (pretty-printed, trailing newline) to `out`.
///
/// `duration_ms` is intentionally omitted from per-check entries in v1
/// to keep two back-to-back runs byte-identical given identical
/// workspace state and identical check exit codes (SPEC REQ-005).
///
/// # Errors
///
/// Returns [`VerifyError::JsonSerialise`] on serialisation failure or
/// [`VerifyError::Io`] on write failure.
pub fn write_json(report: &VerifyReport, out: &mut dyn Write) -> Result<(), VerifyError> {
    let payload = JsonOutput {
        schema_version: 1,
        repo_sha: report.repo_sha.clone(),
        lint: lint_block(report),
        checks: report.checks.iter().map(json_check).collect(),
        summary: JsonSummary {
            lint: JsonLintCounts {
                errors: report.lint_errors.len(),
                warnings: report.lint_warnings.len(),
                info: report.lint_info.len(),
            },
            checks: JsonCheckCounts {
                passed: report.passed_checks(),
                failed: report.failed_checks(),
                in_flight: report.in_flight_checks(),
                manual: report.manual_checks(),
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

fn json_check(r: &CheckResult) -> JsonCheck {
    JsonCheck {
        spec_id: r.spec_id.clone(),
        spec_status: r.spec_status.clone(),
        check_id: r.check_id.clone(),
        kind: r.kind.clone(),
        outcome: r.outcome.as_str().to_owned(),
        exit_code: r.exit_code,
    }
}

/// Top-level JSON envelope.
#[derive(Debug, Clone, Serialize)]
pub struct JsonOutput {
    /// Schema version. Bumped on breaking changes.
    pub schema_version: u32,
    /// HEAD commit SHA, or `""` if unavailable.
    pub repo_sha: String,
    /// Grouped lint diagnostics (errors / warnings / info).
    pub lint: JsonLintBlock,
    /// Per-check results in execution order.
    pub checks: Vec<JsonCheck>,
    /// Aggregate counts.
    pub summary: JsonSummary,
    /// `true` iff exit code is 0.
    pub passed: bool,
}

/// One check result in JSON shape. `duration_ms` is intentionally
/// omitted for byte-determinism (see [`write_json`]).
#[derive(Debug, Clone, Serialize)]
pub struct JsonCheck {
    /// Spec the check belongs to.
    pub spec_id: String,
    /// Lifecycle status of the parent spec (`"in-progress"` or
    /// `"implemented"`). Lets consumers distinguish gating failures
    /// from in-flight signal without re-parsing the workspace.
    pub spec_status: String,
    /// Stable `CHK-NNN` identifier.
    pub check_id: String,
    /// Free-form kind label.
    pub kind: String,
    /// `"Pass"`, `"Fail"`, or `"Manual"`.
    pub outcome: String,
    /// Child exit code, if the check spawned a process and exited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// Aggregate lint and check counts.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonSummary {
    /// Lint counts by level.
    pub lint: JsonLintCounts,
    /// Check counts by outcome.
    pub checks: JsonCheckCounts,
}

/// Lint count buckets.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonLintCounts {
    /// Error-level diagnostic count.
    pub errors: usize,
    /// Warn-level diagnostic count.
    pub warnings: usize,
    /// Info-level diagnostic count.
    pub info: usize,
}

/// Check outcome buckets.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct JsonCheckCounts {
    /// `Pass` count.
    pub passed: usize,
    /// `Fail` count on `implemented` specs (gates the exit code).
    pub failed: usize,
    /// `Fail` count on `in-progress` specs (work-in-flight; informational).
    pub in_flight: usize,
    /// `Manual` count.
    pub manual: usize,
}
