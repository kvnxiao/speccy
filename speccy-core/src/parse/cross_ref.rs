//! Symmetric REQ-ID diff between SPEC.md headings and SPEC.md element tree.
//!
//! Before SPEC-0019 this compared SPEC.md against per-spec `spec.toml`.
//! After SPEC-0020 the requirement graph lives in the SPEC.md raw XML
//! element tree (see [`crate::parse::spec_xml`]); the heading view from
//! [`SpecMd`] and the element view from [`SpecDoc`] should agree on the
//! same REQ-ID set.
//!
//! Pure, deterministic, idempotent. See
//! `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-006.
//!
//! # SPEC-0022 workspace-load cross-reference validation
//!
//! [`validate_workspace_xml`] is the second public entry point in this
//! module: it ties the typed [`crate::parse::TasksDoc`] and
//! [`crate::parse::ReportDoc`] models against their parent
//! [`SpecDoc`], surfacing dangling REQ ids, dangling CHK ids, and missing
//! coverage as diagnostics. It is reachable from the workspace loader
//! through [`crate::workspace::validate_workspace_xml`], the seam T-007
//! flips on after the corpus migration in T-005 / T-006 — until then,
//! the in-tree TASKS.md / REPORT.md files still use the legacy
//! checkbox/Markdown form, and the loader does **not** route them
//! through this validator.

use crate::error::ParseError;
use crate::parse::ReportDoc;
use crate::parse::SpecDoc;
use crate::parse::SpecMd;
use crate::parse::TasksDoc;
use camino::Utf8Path;
use std::collections::HashSet;

/// Symmetric diff between SPEC.md REQ headings and `<requirement>`
/// elements in the same SPEC.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRef {
    /// IDs that appear in SPEC.md headings but not in the element tree,
    /// in heading-declared order.
    pub only_in_spec_md: Vec<String>,
    /// IDs that appear in the element tree but not in SPEC.md headings,
    /// in element-declared order.
    pub only_in_markers: Vec<String>,
    /// IDs present on both sides, in SPEC.md heading-declared order.
    pub in_both: Vec<String>,
}

/// Compute the symmetric REQ-ID diff between SPEC.md headings and the
/// SPEC.md element tree.
#[must_use = "the diff is the entire purpose of this call"]
pub fn cross_ref(spec: &SpecMd, doc: &SpecDoc) -> CrossRef {
    let md_ids: Vec<&str> = spec.requirements.iter().map(|r| r.id.as_str()).collect();
    let marker_ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();

    let md_set: HashSet<&str> = md_ids.iter().copied().collect();
    let marker_set: HashSet<&str> = marker_ids.iter().copied().collect();

    let only_in_spec_md: Vec<String> = md_ids
        .iter()
        .filter(|id| !marker_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    let only_in_markers: Vec<String> = marker_ids
        .iter()
        .filter(|id| !md_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    let in_both: Vec<String> = md_ids
        .iter()
        .filter(|id| marker_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    CrossRef {
        only_in_spec_md,
        only_in_markers,
        in_both,
    }
}

/// SPEC-0022 cross-reference validation between SPEC, TASKS, and REPORT.
///
/// Surfaces four classes of drift as [`ParseError`] diagnostics:
///
/// - **Dangling REQ from TASKS**
///   ([`ParseError::TaskCoversDanglingRequirement`]): a `<task>` whose
///   `covers="REQ-NNN"` includes a `REQ-NNN` the parent SPEC.md does not
///   declare.
/// - **Dangling REQ from REPORT**
///   ([`ParseError::CoverageDanglingRequirement`]): a `<coverage
///   req="REQ-NNN">` row whose `REQ-NNN` is not in SPEC.md.
/// - **Dangling CHK from REPORT** ([`ParseError::CoverageDanglingScenario`]): a
///   `<coverage>` element listing a `CHK-NNN` that is not nested under the
///   matching `<requirement>` in SPEC.md.
/// - **Missing coverage** ([`ParseError::MissingRequirementCoverage`]):
///   REPORT.md is present but does not cover every requirement in SPEC. The
///   diagnostic names every uncovered requirement (not just the first).
///
/// Skip rule: when `report` is `None` (in-flight implementation with no
/// REPORT.md yet) the missing-coverage check is skipped. TASKS.md
/// dangling-requirement validation still runs whenever `tasks` is `Some`.
///
/// `tasks_path` and `report_path` are used only to populate diagnostics;
/// this function does no filesystem IO.
///
/// All four classes are collected before returning, so the caller sees
/// every diagnostic in one pass rather than only the first.
///
/// # Errors
///
/// Returns a [`Vec<ParseError>`]. The caller surfaces them through the
/// existing per-spec parse-failure channel.
#[must_use = "the returned diagnostics are the entire purpose of this call"]
pub fn validate_workspace_xml(
    spec: &SpecDoc,
    tasks: Option<&TasksDoc>,
    tasks_path: Option<&Utf8Path>,
    report: Option<&ReportDoc>,
    report_path: Option<&Utf8Path>,
) -> Vec<ParseError> {
    let mut diagnostics: Vec<ParseError> = Vec::new();

    let spec_req_ids: HashSet<&str> = spec.requirements.iter().map(|r| r.id.as_str()).collect();

    if let (Some(tasks_doc), Some(path)) = (tasks, tasks_path) {
        for task in &tasks_doc.tasks {
            for req in &task.covers {
                if !spec_req_ids.contains(req.as_str()) {
                    diagnostics.push(ParseError::TaskCoversDanglingRequirement {
                        path: path.to_path_buf(),
                        task_id: task.id.clone(),
                        requirement_id: req.clone(),
                    });
                }
            }
        }
    }

    if let (Some(report_doc), Some(path)) = (report, report_path) {
        // Dangling REQ from REPORT, and dangling CHK from REPORT.
        for row in &report_doc.coverage {
            if !spec_req_ids.contains(row.req.as_str()) {
                diagnostics.push(ParseError::CoverageDanglingRequirement {
                    path: path.to_path_buf(),
                    requirement_id: row.req.clone(),
                });
                // No point checking scenarios against a requirement that
                // does not exist; the dangling-REQ diagnostic is the
                // headline. Skip per-CHK checks for this row.
                continue;
            }
            // Requirement exists; check each CHK against the SPEC-side
            // scenario set under that requirement.
            let scenario_ids: HashSet<&str> = spec
                .requirements
                .iter()
                .find(|r| r.id == row.req)
                .map(|r| r.scenarios.iter().map(|s| s.id.as_str()).collect())
                .unwrap_or_default();
            for chk in &row.scenarios {
                if !scenario_ids.contains(chk.as_str()) {
                    diagnostics.push(ParseError::CoverageDanglingScenario {
                        path: path.to_path_buf(),
                        requirement_id: row.req.clone(),
                        scenario_id: chk.clone(),
                    });
                }
            }
        }

        // Missing-coverage check: every SPEC requirement must have a
        // coverage row in REPORT.md. Collect *all* uncovered ids so the
        // diagnostic lists them in one shot.
        let covered: HashSet<&str> = report_doc.coverage.iter().map(|c| c.req.as_str()).collect();
        let uncovered: Vec<String> = spec
            .requirements
            .iter()
            .filter(|r| !covered.contains(r.id.as_str()))
            .map(|r| r.id.clone())
            .collect();
        if !uncovered.is_empty() {
            diagnostics.push(ParseError::MissingRequirementCoverage {
                path: path.to_path_buf(),
                requirement_ids: uncovered,
            });
        }
    }

    diagnostics
}
