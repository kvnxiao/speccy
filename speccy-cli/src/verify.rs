//! `speccy verify` command logic.
//!
//! Composes SPEC-0003's lint engine with SPEC-0012's captured check
//! execution into a single CI gate. Live output (per-check headers,
//! child stdout/stderr, footers, malformed-spec.toml warnings) streams
//! to stderr; stdout is reserved for the final summary (text mode) or
//! the structured JSON envelope (`--json`).
//!
//! See `.speccy/specs/0012-verify-command/SPEC.md`.

use crate::git::repo_sha;
use crate::verify_output::write_json;
use crate::verify_output::write_text;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::exec::CheckOutcome;
use speccy_core::exec::CheckResult;
use speccy_core::exec::CheckSpec;
use speccy_core::exec::run_checks_captured;
use speccy_core::lint;
use speccy_core::lint::Diagnostic;
use speccy_core::lint::Level;
use speccy_core::parse::SpecStatus;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::collections::HashMap;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VerifyError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery (e.g. unreadable parent
    /// directory metadata).
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// I/O failure while streaming live output or writing the summary.
    #[error("I/O error during verify")]
    Io(#[from] std::io::Error),
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
    /// JSON serialisation failed.
    #[error("failed to serialise verify JSON")]
    JsonSerialise(#[from] serde_json::Error),
}

/// `speccy verify` arguments.
#[derive(Debug, Clone, Copy, Default)]
pub struct VerifyArgs {
    /// Emit JSON instead of the three-line text summary.
    pub json: bool,
}

/// Aggregated lint + check outcomes assembled by [`run`] and consumed
/// by the renderers in [`crate::verify_output`].
#[derive(Debug)]
pub struct VerifyReport {
    /// `Level::Error` lint diagnostics.
    pub lint_errors: Vec<Diagnostic>,
    /// `Level::Warn` lint diagnostics.
    pub lint_warnings: Vec<Diagnostic>,
    /// `Level::Info` lint diagnostics.
    pub lint_info: Vec<Diagnostic>,
    /// Per-check results in execution order (which is workspace order
    /// then declared check order).
    pub checks: Vec<CheckResult>,
    /// `HEAD` SHA from `git rev-parse HEAD`, or `""` if unavailable.
    pub repo_sha: String,
}

impl VerifyReport {
    /// Whether the workspace passes the gate.
    ///
    /// `true` iff no `Level::Error` lint diagnostics fired and every
    /// `Fail` outcome came from an `in-progress` spec (work-in-flight,
    /// not a regression). `Fail` outcomes on `implemented` specs gate
    /// the exit code.
    #[must_use = "the pass/fail signal drives the exit code"]
    pub fn passed(&self) -> bool {
        self.lint_errors.is_empty() && !self.checks.iter().any(CheckResult::is_gating_failure)
    }

    /// Number of checks with outcome [`CheckOutcome::Pass`].
    #[must_use = "the count is part of the summary output"]
    pub fn passed_checks(&self) -> usize {
        self.checks
            .iter()
            .filter(|r| matches!(r.outcome, CheckOutcome::Pass))
            .count()
    }

    /// Number of checks with outcome [`CheckOutcome::Fail`] on
    /// `implemented` specs — the "real" failure count that gates the
    /// exit code.
    #[must_use = "the count is part of the summary output"]
    pub fn failed_checks(&self) -> usize {
        self.checks.iter().filter(|r| r.is_gating_failure()).count()
    }

    /// Number of checks with outcome [`CheckOutcome::Fail`] on
    /// `in-progress` specs — informational only, does not gate.
    #[must_use = "the count is part of the summary output"]
    pub fn in_flight_checks(&self) -> usize {
        self.checks.iter().filter(|r| r.is_in_flight()).count()
    }

    /// Number of checks with outcome [`CheckOutcome::Manual`].
    #[must_use = "the count is part of the summary output"]
    pub fn manual_checks(&self) -> usize {
        self.checks
            .iter()
            .filter(|r| matches!(r.outcome, CheckOutcome::Manual))
            .count()
    }
}

/// Run `speccy verify` from `cwd`. Live output (per-check headers,
/// child stdio, footers, malformed-spec warnings) goes to `err`; the
/// final summary or JSON goes to `out`. Returns the process exit code
/// (0 on pass, 1 on fail).
///
/// # Errors
///
/// Returns [`VerifyError`] when discovery or I/O fails. Check failures
/// are surfaced via the exit code, not the `Result`.
pub fn run(
    args: VerifyArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, VerifyError> {
    let VerifyArgs { json } = args;
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(VerifyError::ProjectRootNotFound),
        Err(other) => return Err(VerifyError::Workspace(other)),
    };

    let workspace = scan(&project_root);
    let diagnostics = lint::run(&workspace.as_lint_workspace());
    let status_by_spec = build_status_map(&workspace);
    let (lint_errors, lint_warnings, lint_info) = partition_lint(diagnostics, &status_by_spec);

    let check_specs = collect_check_specs(&workspace, err)?;
    let checks = run_checks_captured(&check_specs, &project_root, err)?;

    let report = VerifyReport {
        lint_errors,
        lint_warnings,
        lint_info,
        checks,
        repo_sha: repo_sha(&project_root),
    };

    if json {
        write_json(&report, out)?;
    } else {
        write_text(&report, out)?;
    }

    Ok(i32::from(!report.passed()))
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`VerifyError::Cwd`] if `std::env::current_dir` fails, or
/// [`VerifyError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, VerifyError> {
    let std_path = std::env::current_dir().map_err(VerifyError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| VerifyError::CwdNotUtf8)
}

/// Partition diagnostics into severity buckets, demoting `Level::Error`
/// diagnostics on `in-progress` specs to `Level::Info`.
///
/// Mirrors the check-side filter introduced in commit a5b5fea: drafted
/// specs (those still being authored) should not gate `speccy verify`
/// when their TASKS.md or SPEC.md hasn't been polished yet. Workspace-
/// level diagnostics (no `spec_id`) and diagnostics on `implemented` /
/// `dropped` / `superseded` specs keep their original severity.
fn partition_lint(
    diagnostics: Vec<Diagnostic>,
    status_by_spec: &HashMap<String, SpecStatus>,
) -> (Vec<Diagnostic>, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();
    for mut d in diagnostics {
        if matches!(d.level, Level::Error)
            && d.spec_id
                .as_deref()
                .and_then(|id| status_by_spec.get(id).copied())
                == Some(SpecStatus::InProgress)
        {
            d.level = Level::Info;
        }
        match d.level {
            Level::Error => errors.push(d),
            Level::Warn => warnings.push(d),
            Level::Info => info.push(d),
        }
    }
    (errors, warnings, info)
}

fn build_status_map(ws: &Workspace) -> HashMap<String, SpecStatus> {
    ws.specs
        .iter()
        .filter_map(|s| {
            let id = s.spec_id.clone()?;
            let status = s.spec_md.as_ref().ok()?.frontmatter.status;
            Some((id, status))
        })
        .collect()
}

fn collect_check_specs(ws: &Workspace, err: &mut dyn Write) -> Result<Vec<CheckSpec>, VerifyError> {
    let mut out = Vec::new();
    for parsed in &ws.specs {
        let label = parsed
            .spec_id
            .clone()
            .unwrap_or_else(|| display_spec_label(&parsed.dir));
        let spec_status = parsed
            .spec_md
            .as_ref()
            .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);
        // Skip defunct specs entirely: their checks should never run.
        if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
            continue;
        }
        match &parsed.spec_toml {
            Ok(toml) => {
                for check in &toml.checks {
                    out.push(CheckSpec::from_entry(
                        label.clone(),
                        spec_status.as_str(),
                        check,
                    ));
                }
            }
            Err(e) => {
                writeln!(
                    err,
                    "speccy verify: warning: {label} spec.toml failed to parse: {e}; skipping",
                )?;
            }
        }
    }
    Ok(out)
}

fn display_spec_label(dir: &Utf8Path) -> String {
    dir.file_name()
        .map_or_else(|| dir.to_string(), ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::Diagnostic;
    use super::HashMap;
    use super::Level;
    use super::SpecStatus;
    use super::VerifyReport;
    use super::partition_lint;
    use speccy_core::exec::CheckOutcome;
    use speccy_core::exec::CheckResult;

    fn diag(code: &'static str, level: Level) -> Diagnostic {
        Diagnostic::spec_only(code, level, None, "test")
    }

    fn diag_for_spec(code: &'static str, level: Level, spec_id: &str) -> Diagnostic {
        Diagnostic::spec_only(code, level, Some(spec_id.to_owned()), "test")
    }

    fn check(outcome: CheckOutcome) -> CheckResult {
        check_with_status(outcome, "implemented")
    }

    fn check_with_status(outcome: CheckOutcome, status: &str) -> CheckResult {
        CheckResult {
            spec_id: "SPEC-0001".into(),
            spec_status: status.into(),
            check_id: "CHK-001".into(),
            kind: "test".into(),
            outcome,
            exit_code: match outcome {
                CheckOutcome::Pass => Some(0),
                CheckOutcome::Fail => Some(1),
                CheckOutcome::Manual => None,
            },
            duration_ms: None,
        }
    }

    #[test]
    fn partition_lint_groups_by_level() {
        let diagnostics = vec![
            diag("SPC-001", Level::Error),
            diag("QST-001", Level::Info),
            diag("SPC-006", Level::Warn),
            diag("SPC-002", Level::Error),
        ];
        let (errors, warnings, info) = partition_lint(diagnostics, &HashMap::new());
        assert_eq!(errors.len(), 2);
        assert_eq!(warnings.len(), 1);
        assert_eq!(info.len(), 1);
    }

    #[test]
    fn partition_lint_demotes_errors_on_in_progress_specs() {
        let diagnostics = vec![
            diag_for_spec("TSK-001", Level::Error, "SPEC-0001"),
            diag_for_spec("TSK-001", Level::Error, "SPEC-0002"),
            diag_for_spec("SPC-006", Level::Warn, "SPEC-0001"),
        ];
        let mut status_map: HashMap<String, SpecStatus> = HashMap::new();
        status_map.insert("SPEC-0001".to_owned(), SpecStatus::InProgress);
        status_map.insert("SPEC-0002".to_owned(), SpecStatus::Implemented);

        let (errors, warnings, info) = partition_lint(diagnostics, &status_map);
        assert_eq!(errors.len(), 1, "implemented spec error must remain gating");
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            info.len(),
            1,
            "in-progress spec error must be demoted to info"
        );
        assert_eq!(
            info.first().map(|d| d.spec_id.as_deref()),
            Some(Some("SPEC-0001"))
        );
    }

    #[test]
    fn partition_lint_keeps_workspace_level_errors_gating() {
        // Diagnostics with no spec_id (workspace-level) keep Error.
        let diagnostics = vec![diag("SPC-001", Level::Error)];
        let mut status_map: HashMap<String, SpecStatus> = HashMap::new();
        status_map.insert("SPEC-0001".to_owned(), SpecStatus::InProgress);

        let (errors, _warnings, info) = partition_lint(diagnostics, &status_map);
        assert_eq!(errors.len(), 1, "workspace-level Error must not be demoted");
        assert!(info.is_empty());
    }

    #[test]
    fn passed_requires_no_lint_errors() {
        let report = VerifyReport {
            lint_errors: vec![diag("SPC-001", Level::Error)],
            lint_warnings: vec![],
            lint_info: vec![],
            checks: vec![check(CheckOutcome::Pass)],
            repo_sha: String::new(),
        };
        assert!(!report.passed());
    }

    #[test]
    fn passed_requires_no_failing_checks() {
        let report = VerifyReport {
            lint_errors: vec![],
            lint_warnings: vec![],
            lint_info: vec![],
            checks: vec![check(CheckOutcome::Pass), check(CheckOutcome::Fail)],
            repo_sha: String::new(),
        };
        assert!(!report.passed());
    }

    #[test]
    fn passed_allows_warnings_and_info() {
        let report = VerifyReport {
            lint_errors: vec![],
            lint_warnings: vec![diag("SPC-006", Level::Warn)],
            lint_info: vec![diag("QST-001", Level::Info)],
            checks: vec![check(CheckOutcome::Pass)],
            repo_sha: String::new(),
        };
        assert!(report.passed());
    }

    #[test]
    fn manual_checks_do_not_block_pass() {
        let report = VerifyReport {
            lint_errors: vec![],
            lint_warnings: vec![],
            lint_info: vec![],
            checks: vec![check(CheckOutcome::Pass), check(CheckOutcome::Manual)],
            repo_sha: String::new(),
        };
        assert!(report.passed());
        assert_eq!(report.passed_checks(), 1);
        assert_eq!(report.failed_checks(), 0);
        assert_eq!(report.manual_checks(), 1);
    }

    #[test]
    fn empty_workspace_passes() {
        let report = VerifyReport {
            lint_errors: vec![],
            lint_warnings: vec![],
            lint_info: vec![],
            checks: vec![],
            repo_sha: String::new(),
        };
        assert!(report.passed());
    }

    #[test]
    fn in_progress_fail_is_in_flight_not_gating() {
        let report = VerifyReport {
            lint_errors: vec![],
            lint_warnings: vec![],
            lint_info: vec![],
            checks: vec![
                check(CheckOutcome::Pass),
                check_with_status(CheckOutcome::Fail, "in-progress"),
            ],
            repo_sha: String::new(),
        };
        assert!(
            report.passed(),
            "Fail on in-progress spec must not gate verify",
        );
        assert_eq!(report.passed_checks(), 1);
        assert_eq!(report.failed_checks(), 0, "implemented-failures count");
        assert_eq!(report.in_flight_checks(), 1, "in-progress failures count");
    }

    #[test]
    fn implemented_fail_still_gates() {
        let report = VerifyReport {
            lint_errors: vec![],
            lint_warnings: vec![],
            lint_info: vec![],
            checks: vec![check_with_status(CheckOutcome::Fail, "implemented")],
            repo_sha: String::new(),
        };
        assert!(
            !report.passed(),
            "Fail on implemented spec must gate verify"
        );
        assert_eq!(report.failed_checks(), 1);
        assert_eq!(report.in_flight_checks(), 0);
    }
}
