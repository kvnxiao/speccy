#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test code may .expect() with descriptive messages and panic! on unexpected enum variants"
)]
//! SPEC-0022 T-004 — workspace-load cross-reference validation between
//! SPEC, TASKS, and REPORT.
//!
//! Each test drives the validation through the seam that T-007 will
//! flip on after T-006 migrates the in-tree corpus:
//! [`speccy_core::workspace::parse_one_spec_xml_artifacts`] reads the
//! typed XML models off disk, and
//! [`speccy_core::workspace::validate_workspace_xml`] consumes them
//! together with a parent [`speccy_core::parse::SpecDoc`] to surface
//! dangling-REQ / dangling-CHK / missing-coverage diagnostics.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::error::ParseError;
use speccy_core::parse::parse_spec_xml;
use speccy_core::workspace::XmlValidationInput;
use speccy_core::workspace::parse_one_spec_xml_artifacts;
use speccy_core::workspace::validate_workspace_xml;

fn fixture_dir(name: &str) -> Utf8PathBuf {
    let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("tests")
        .join("fixtures")
        .join("workspace_xml")
        .join(name)
}

fn load_spec(dir: &Utf8Path) -> speccy_core::parse::SpecDoc {
    let spec_path = dir.join("SPEC.md");
    let src = fs_err::read_to_string(spec_path.as_std_path())
        .expect("fixture SPEC.md should read off disk");
    parse_spec_xml(&src, &spec_path).expect("fixture SPEC.md should parse")
}

fn run(name: &str) -> (Utf8PathBuf, Vec<ParseError>) {
    let dir = fixture_dir(name);
    let spec = load_spec(&dir);
    let artifacts = parse_one_spec_xml_artifacts(&dir);
    let tasks_doc = artifacts
        .tasks
        .map(|r| r.expect("fixture TASKS.md should parse"));
    let report_doc = artifacts
        .report
        .map(|r| r.expect("fixture REPORT.md should parse"));

    let tasks_path = tasks_doc.as_ref().map(|_| dir.join("TASKS.md"));
    let report_path = report_doc.as_ref().map(|_| dir.join("REPORT.md"));

    // Borrow into the input view (paths and docs both owned above so they
    // outlive the call).
    let diagnostics = validate_workspace_xml(&XmlValidationInput {
        spec: &spec,
        tasks: tasks_doc.as_ref(),
        tasks_path: tasks_path.as_deref(),
        report: report_doc.as_ref(),
        report_path: report_path.as_deref(),
    });
    (dir, diagnostics)
}

#[test]
fn dangling_req_in_tasks_names_task_req_and_path() {
    let (dir, diagnostics) = run("dangling_req_tasks");
    let tasks_path = dir.join("TASKS.md");

    assert_eq!(
        diagnostics.len(),
        1,
        "expected exactly one diagnostic; got: {diagnostics:?}",
    );
    let diag = diagnostics
        .first()
        .expect("one diagnostic should be present");
    match diag {
        ParseError::TaskCoversDanglingRequirement {
            path,
            task_id,
            requirement_id,
        } => {
            assert_eq!(task_id, "T-001");
            assert_eq!(requirement_id, "REQ-999");
            assert_eq!(path, &tasks_path);
        }
        other => panic!("expected TaskCoversDanglingRequirement, got {other:?}"),
    }
    // Diagnostic message must name all three so downstream consumers
    // (lint, status, verify) can render actionable text without re-parsing.
    let msg = format!("{diag}");
    assert!(msg.contains("T-001"), "msg `{msg}` missing task id");
    assert!(
        msg.contains("REQ-999"),
        "msg `{msg}` missing requirement id"
    );
    assert!(
        msg.contains(tasks_path.as_str()),
        "msg `{msg}` missing TASKS.md path"
    );
}

#[test]
fn dangling_req_in_report_names_coverage_and_path() {
    let (dir, diagnostics) = run("dangling_req_report");
    let report_path = dir.join("REPORT.md");

    // The dangling-REQ row also skips per-CHK checks, so we expect only
    // one CoverageDanglingRequirement diagnostic for the dangling row.
    assert!(
        diagnostics.iter().any(|d| matches!(
            d,
            ParseError::CoverageDanglingRequirement { path, requirement_id }
                if requirement_id == "REQ-999" && path == &report_path
        )),
        "expected CoverageDanglingRequirement for REQ-999 in {report_path}; got {diagnostics:?}",
    );
    let diag = diagnostics
        .iter()
        .find(|d| matches!(d, ParseError::CoverageDanglingRequirement { .. }))
        .expect("dangling-REQ diagnostic should be present");
    let msg = format!("{diag}");
    assert!(
        msg.contains("REQ-999"),
        "msg `{msg}` missing requirement id"
    );
    assert!(
        msg.contains(report_path.as_str()),
        "msg `{msg}` missing REPORT.md path"
    );
}

#[test]
fn dangling_scenario_in_report_names_req_chk_and_path() {
    let (dir, diagnostics) = run("dangling_scenario");
    let report_path = dir.join("REPORT.md");

    let diag = diagnostics
        .iter()
        .find(|d| matches!(d, ParseError::CoverageDanglingScenario { .. }))
        .expect("dangling-scenario diagnostic should be present");
    match diag {
        ParseError::CoverageDanglingScenario {
            path,
            requirement_id,
            scenario_id,
        } => {
            assert_eq!(requirement_id, "REQ-001");
            assert_eq!(scenario_id, "CHK-099");
            assert_eq!(path, &report_path);
        }
        other => panic!("expected CoverageDanglingScenario, got {other:?}"),
    }
    let msg = format!("{diag}");
    assert!(
        msg.contains("REQ-001"),
        "msg `{msg}` missing requirement id"
    );
    assert!(msg.contains("CHK-099"), "msg `{msg}` missing scenario id");
    assert!(
        msg.contains(report_path.as_str()),
        "msg `{msg}` missing REPORT.md path"
    );
}

#[test]
fn missing_coverage_lists_every_uncovered_requirement() {
    let (dir, diagnostics) = run("missing_coverage");
    let report_path = dir.join("REPORT.md");

    let diag = diagnostics
        .iter()
        .find(|d| matches!(d, ParseError::MissingRequirementCoverage { .. }))
        .expect("missing-coverage diagnostic should be present");
    match diag {
        ParseError::MissingRequirementCoverage {
            path,
            requirement_ids,
        } => {
            assert_eq!(path, &report_path);
            // The fixture has SPEC requirements REQ-001 and REQ-002 and
            // covers only REQ-001. REQ-002 is the uncovered id; assert
            // the diagnostic surfaces it and *only* it (collects all but
            // there is exactly one uncovered here).
            assert_eq!(requirement_ids, &vec!["REQ-002".to_owned()]);
        }
        other => panic!("expected MissingRequirementCoverage, got {other:?}"),
    }
    let msg = format!("{diag}");
    assert!(msg.contains("REQ-002"), "msg `{msg}` missing REQ-002");
}

#[test]
fn report_absent_skips_missing_coverage_but_runs_tasks_dangling() {
    let (dir, diagnostics) = run("no_report_yet");
    let tasks_path = dir.join("TASKS.md");

    // TASKS dangling-req fires.
    assert!(
        diagnostics.iter().any(|d| matches!(
            d,
            ParseError::TaskCoversDanglingRequirement { path, task_id, requirement_id }
                if task_id == "T-001"
                    && requirement_id == "REQ-999"
                    && path == &tasks_path
        )),
        "expected TaskCoversDanglingRequirement; got {diagnostics:?}",
    );
    // Missing-coverage does NOT fire — REPORT.md is absent.
    assert!(
        !diagnostics
            .iter()
            .any(|d| matches!(d, ParseError::MissingRequirementCoverage { .. })),
        "missing-coverage must be skipped when REPORT is absent; got {diagnostics:?}",
    );
    // And no REPORT-side diagnostics in general.
    assert!(
        !diagnostics.iter().any(|d| matches!(
            d,
            ParseError::CoverageDanglingRequirement { .. }
                | ParseError::CoverageDanglingScenario { .. }
        )),
        "no REPORT-side diagnostics should fire when REPORT is absent; got {diagnostics:?}",
    );
}

#[test]
fn valid_post_migration_fixture_produces_no_diagnostics() {
    let (_dir, diagnostics) = run("valid_post_migration");
    assert!(
        diagnostics.is_empty(),
        "expected zero diagnostics on the valid fixture; got {diagnostics:?}",
    );
}
