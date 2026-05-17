#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! TSK-* lint diagnostics.

mod lint_common;

use indoc::indoc;
use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::valid_spec_md;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;
use speccy_core::lint::types::Level;

fn assert_has_code(diags: &[Diagnostic], code: &str) {
    assert!(
        diags.iter().any(|d| d.code == code),
        "expected {code}, got: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
}

fn tasks_md_xml(state: &str, covers: &str) -> String {
    format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n\n<tasks spec=\"SPEC-0001\">\n\n<task id=\"T-001\" state=\"{state}\" covers=\"{covers}\">\nt\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n</tasks>\n",
    )
}

#[test]
fn tsk_001_fires_for_unknown_covered_req() -> TestResult {
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        Some(&tasks_md_xml("pending", "REQ-099")),
    )?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-001");
    Ok(())
}

#[test]
fn tsk_004_fires_when_frontmatter_missing_generated_at() -> TestResult {
    let tasks_md = indoc! {r#"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        ---

        # Tasks: SPEC-0001

        <tasks spec="SPEC-0001">
        </tasks>
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), Some(tasks_md))?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-004");
    Ok(())
}

#[test]
fn tsk_003_fires_at_info_for_bootstrap_pending() -> TestResult {
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        Some(&tasks_md_xml("pending", "REQ-001")),
    )?;
    let diags = lint_fixture(&fx);
    let tsk_003 = diags
        .iter()
        .find(|d| d.code == "TSK-003")
        .ok_or("TSK-003 expected")?;
    assert_eq!(tsk_003.level, Level::Info);
    assert!(
        tsk_003.message.contains("speccy tasks"),
        "message should advise speccy tasks --commit; got: {}",
        tsk_003.message,
    );
    Ok(())
}

#[test]
fn tsk_003_fires_at_warn_for_hash_mismatch() -> TestResult {
    let body = "---\nspec: SPEC-0001\nspec_hash_at_generation: sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n\n<tasks spec=\"SPEC-0001\">\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nt\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n</tasks>\n".to_owned();
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), Some(&body))?;
    let diags = lint_fixture(&fx);
    let tsk_003 = diags
        .iter()
        .find(|d| d.code == "TSK-003")
        .ok_or("TSK-003 expected")?;
    assert_eq!(tsk_003.level, Level::Warn);
    Ok(())
}
