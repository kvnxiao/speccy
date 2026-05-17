//! `speccy verify` command logic.
//!
//! Shape-only validator. SPEC-0018 REQ-003: walks parsed specs and
//! surfaces lint diagnostics (parse errors, requirement-to-scenario
//! cross-references, stale tasks, open questions); never executes
//! scenarios or spawns child processes. Check execution is deliberately
//! gone — scenarios are English claims for reviewers and humans, not
//! commands speccy runs.
//!
//! See `.speccy/specs/0018-remove-check-execution/SPEC.md` REQ-003.

use crate::git::repo_sha;
use crate::verify_output::write_json;
use crate::verify_output::write_text;
use camino::Utf8Path;
use camino::Utf8PathBuf;
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
    /// I/O failure while writing the summary.
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
    /// Emit JSON instead of the text summary.
    pub json: bool,
}

/// Shape-only counts aggregated by [`run`] and consumed by the
/// renderers in [`crate::verify_output`].
///
/// Every count covers specs that are not `dropped` or `superseded` (those
/// remain non-gating, matching the pre-SPEC-0018 distinction). Workspace-
/// level diagnostics that are not tied to a spec keep their full severity.
#[derive(Debug)]
pub struct VerifyReport {
    /// `Level::Error` lint diagnostics (after in-progress demotion).
    pub lint_errors: Vec<Diagnostic>,
    /// `Level::Warn` lint diagnostics.
    pub lint_warnings: Vec<Diagnostic>,
    /// `Level::Info` lint diagnostics (includes errors demoted on
    /// in-progress specs).
    pub lint_info: Vec<Diagnostic>,
    /// Number of specs walked (all statuses).
    pub specs_total: usize,
    /// Total `SpecDoc.requirements` entries across every spec whose
    /// SPEC.md marker tree parsed and which is not defunct.
    pub requirements_total: usize,
    /// Sum of `Requirement.scenarios.len()` over those same specs — every
    /// `speccy:scenario` marker nested under a `speccy:requirement`
    /// marker in a non-defunct spec's SPEC.md.
    pub scenarios_total: usize,
    /// `HEAD` SHA from `git rev-parse HEAD`, or `""` if unavailable.
    pub repo_sha: String,
}

impl VerifyReport {
    /// Whether the workspace passes the gate.
    ///
    /// `true` iff no `Level::Error` lint diagnostics remain after the
    /// in-progress demotion pass.
    #[must_use = "the pass/fail signal drives the exit code"]
    pub fn passed(&self) -> bool {
        self.lint_errors.is_empty()
    }
}

/// Run `speccy verify` from `cwd`. The text summary or JSON envelope goes
/// to `out`; nothing streams to `err` in normal flow (it remains for
/// future diagnostic surfaces). Returns the process exit code (0 on
/// pass, 1 on fail).
///
/// # Errors
///
/// Returns [`VerifyError`] when discovery or I/O fails. Shape failures
/// are surfaced via the exit code, not the `Result`.
pub fn run(
    args: VerifyArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
    _err: &mut dyn Write,
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
    let (requirements_total, scenarios_total) = shape_totals(&workspace);

    let report = VerifyReport {
        lint_errors,
        lint_warnings,
        lint_info,
        specs_total: workspace.specs.len(),
        requirements_total,
        scenarios_total,
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
/// diagnostics on non-`implemented` specs to `Level::Info`. Workspace-
/// level diagnostics (no `spec_id`) keep their original severity, and
/// diagnostics on `implemented` specs keep theirs too — those gate the
/// exit code. `in-progress` specs are work-in-flight; `dropped` and
/// `superseded` specs replicate the pre-SPEC-0018 non-gating contract
/// (their checks never ran, so their shape errors do not gate either).
fn partition_lint(
    diagnostics: Vec<Diagnostic>,
    status_by_spec: &HashMap<String, SpecStatus>,
) -> (Vec<Diagnostic>, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut info = Vec::new();
    for mut d in diagnostics {
        if matches!(d.level, Level::Error) {
            let spec_status = d
                .spec_id
                .as_deref()
                .and_then(|id| status_by_spec.get(id).copied());
            if matches!(
                spec_status,
                Some(SpecStatus::InProgress | SpecStatus::Dropped | SpecStatus::Superseded)
            ) {
                d.level = Level::Info;
            }
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

/// Walk `SpecDoc.requirements` across every spec whose SPEC.md marker
/// tree parsed and which is not defunct, returning `(requirements,
/// scenarios)` where `scenarios` sums `Requirement.scenarios.len()`.
/// Dropped and superseded specs contribute zero, matching the
/// pre-SPEC-0018 "their checks never ran" distinction.
fn shape_totals(ws: &Workspace) -> (usize, usize) {
    let mut requirements = 0usize;
    let mut scenarios = 0usize;
    for parsed in &ws.specs {
        let spec_status = parsed
            .spec_md
            .as_ref()
            .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);
        if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
            continue;
        }
        if let Ok(doc) = &parsed.spec_doc {
            requirements = requirements.saturating_add(doc.requirements.len());
            let scenario_count: usize = doc.requirements.iter().map(|r| r.scenarios.len()).sum();
            scenarios = scenarios.saturating_add(scenario_count);
        }
    }
    (requirements, scenarios)
}

#[cfg(test)]
mod tests {
    use super::Diagnostic;
    use super::HashMap;
    use super::Level;
    use super::SpecStatus;
    use super::VerifyReport;
    use super::partition_lint;

    fn diag(code: &'static str, level: Level) -> Diagnostic {
        Diagnostic::spec_only(code, level, None, "test")
    }

    fn diag_for_spec(code: &'static str, level: Level, spec_id: &str) -> Diagnostic {
        Diagnostic::spec_only(code, level, Some(spec_id.to_owned()), "test")
    }

    fn empty_report(errors: Vec<Diagnostic>) -> VerifyReport {
        VerifyReport {
            lint_errors: errors,
            lint_warnings: vec![],
            lint_info: vec![],
            specs_total: 0,
            requirements_total: 0,
            scenarios_total: 0,
            repo_sha: String::new(),
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
        let diagnostics = vec![diag("SPC-001", Level::Error)];
        let mut status_map: HashMap<String, SpecStatus> = HashMap::new();
        status_map.insert("SPEC-0001".to_owned(), SpecStatus::InProgress);

        let (errors, _warnings, info) = partition_lint(diagnostics, &status_map);
        assert_eq!(errors.len(), 1, "workspace-level Error must not be demoted");
        assert!(info.is_empty());
    }

    #[test]
    fn passed_requires_no_lint_errors() {
        let report = empty_report(vec![diag("SPC-001", Level::Error)]);
        assert!(!report.passed());
    }

    #[test]
    fn empty_workspace_passes() {
        let report = empty_report(vec![]);
        assert!(report.passed());
    }
}
