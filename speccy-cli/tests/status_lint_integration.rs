#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests that `assemble`'s lint partitioning routes diagnostics to the
//! correct per-spec or workspace-level bucket. Covers SPEC-0004
//! CHK-006.

mod common;

use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use speccy_cli::status::assemble;
use speccy_core::lint;
use speccy_core::workspace::scan;

#[test]
fn diagnostics_route_by_spec_id() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-first",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;
    write_spec(
        &ws.root,
        "0002-second",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0002")),
    )?;

    let workspace = scan(&ws.root);
    let diags = lint::run(&workspace.as_lint_workspace());
    let report = assemble(&workspace, diags, String::new());

    // Each spec view should bucket its own diagnostics; we expect at
    // least TSK-003 (bootstrap pending) for each spec.
    let first = report
        .specs
        .iter()
        .find(|v| v.display_id == "SPEC-0001")
        .expect("first spec must be in the report");
    let second = report
        .specs
        .iter()
        .find(|v| v.display_id == "SPEC-0002")
        .expect("second spec must be in the report");

    assert!(
        first.diagnostics.iter().any(|d| d.code == "TSK-003"),
        "SPEC-0001 should carry its own TSK-003"
    );
    assert!(
        second.diagnostics.iter().any(|d| d.code == "TSK-003"),
        "SPEC-0002 should carry its own TSK-003"
    );

    // No diagnostic should belong to one spec's bucket but reference
    // the other.
    for diag in &first.diagnostics {
        assert_eq!(diag.spec_id.as_deref(), Some("SPEC-0001"));
    }
    for diag in &second.diagnostics {
        assert_eq!(diag.spec_id.as_deref(), Some("SPEC-0002"));
    }

    Ok(())
}

#[test]
fn workspace_level_diagnostics_route_to_top_block() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0001 declares supersedes: [SPEC-9999] which is dangling.
    let spec_md = "---\nid: SPEC-0001\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-11\nsupersedes: [\"SPEC-9999\"]\n---\n\n# SPEC-0001\n\n### REQ-001: First\n";
    write_spec(&ws.root, "0001-x", spec_md, &valid_spec_toml(), None)?;

    let workspace = scan(&ws.root);
    let diags = lint::run(&workspace.as_lint_workspace());
    let report = assemble(&workspace, diags, String::new());

    // The dangling-supersedes diagnostic has spec_id = None, so it
    // should sit in workspace_diagnostics, not on SPEC-0001.
    let dangling_in_workspace = report
        .workspace_diagnostics
        .iter()
        .any(|d| d.message.contains("SPEC-9999"));
    assert!(
        dangling_in_workspace,
        "expected dangling SPEC-9999 in workspace_diagnostics, got: {:?}",
        report.workspace_diagnostics
    );
    Ok(())
}

#[test]
fn empty_workspace_has_empty_diagnostic_buckets() -> TestResult {
    let ws = Workspace::new()?;
    let workspace = scan(&ws.root);
    let diags = lint::run(&workspace.as_lint_workspace());
    let report = assemble(&workspace, diags, String::new());

    assert!(report.specs.is_empty());
    assert!(report.workspace_diagnostics.is_empty());
    Ok(())
}
