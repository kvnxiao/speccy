#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! REQ-* lint diagnostics.

mod lint_common;

use indoc::indoc;
use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::valid_spec_md;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;

fn count_code(diags: &[Diagnostic], code: &str) -> usize {
    diags.iter().filter(|d| d.code == code).count()
}

#[test]
fn req_001_fires_when_requirement_has_no_checks() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = []
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "REQ-001"), 1);
    Ok(())
}

#[test]
fn req_002_fires_when_check_id_does_not_exist() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-999"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "cargo test"
        proves = "x"
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "REQ-002"), 1);
    Ok(())
}

#[test]
fn multiple_requirements_missing_coverage_emit_one_diag_each() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        ### REQ-001: a
        ### REQ-002: b
    "};
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = []

        [[requirements]]
        id = "REQ-002"
        checks = []
    "#};
    let fx = write_spec_fixture(spec_md, spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "REQ-001"), 2);
    Ok(())
}
