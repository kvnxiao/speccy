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
use lint_common::valid_spec_toml;
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

#[test]
fn tsk_001_fires_for_unknown_covered_req() -> TestResult {
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **T-001**: thing
          - Covers: REQ-099
    "};
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        &valid_spec_toml(),
        Some(tasks_md),
    )?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-001");
    Ok(())
}

#[test]
fn tsk_002_fires_when_parser_warning_present() -> TestResult {
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **TASK-001**: malformed prefix
    "};
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        &valid_spec_toml(),
        Some(tasks_md),
    )?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-002");
    Ok(())
}

#[test]
fn tsk_004_fires_when_frontmatter_missing_generated_at() -> TestResult {
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        ---

        - [ ] **T-001**: t
    "};
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        &valid_spec_toml(),
        Some(tasks_md),
    )?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-004");
    Ok(())
}

#[test]
fn tsk_003_fires_at_info_for_bootstrap_pending() -> TestResult {
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **T-001**: t
          - Covers: REQ-001
    "};
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        &valid_spec_toml(),
        Some(tasks_md),
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
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **T-001**: t
          - Covers: REQ-001
    "};
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        &valid_spec_toml(),
        Some(tasks_md),
    )?;
    let diags = lint_fixture(&fx);
    let tsk_003 = diags
        .iter()
        .find(|d| d.code == "TSK-003")
        .ok_or("TSK-003 expected")?;
    assert_eq!(tsk_003.level, Level::Warn);
    Ok(())
}
