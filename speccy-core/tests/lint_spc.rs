#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! SPC-* lint diagnostics.

mod lint_common;

use indoc::indoc;
use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::parse_fixture;
use lint_common::run_lint;
use lint_common::valid_spec_md;
use lint_common::valid_spec_toml;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;
use speccy_core::lint::types::Level;

fn assert_has_code(diags: &[Diagnostic], code: &str) {
    assert!(
        diags.iter().any(|d| d.code == code),
        "expected diagnostic {code}, got: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
}

fn assert_no_code(diags: &[Diagnostic], code: &str) {
    assert!(
        !diags.iter().any(|d| d.code == code),
        "unexpected diagnostic {code}: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
}

#[test]
fn spc_001_fires_when_spec_toml_has_missing_field() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[checks]]
        id = "CHK-001"
        command = "cargo test"
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-001");
    Ok(())
}

#[test]
fn spc_002_fires_when_req_only_in_spec_md() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        ### REQ-001: First
        ### REQ-002: Second
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-002");
    Ok(())
}

#[test]
fn spc_003_fires_when_req_only_in_spec_toml() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[requirements]]
        id = "REQ-002"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "x"
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-003");
    Ok(())
}

#[test]
fn spc_004_fires_when_spec_md_missing_frontmatter() -> TestResult {
    let spec_md = "# No frontmatter\n";
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-004");
    Ok(())
}

#[test]
fn spc_005_fires_when_status_is_invalid() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: garbage
        created: 2026-05-11
        ---

        ### REQ-001: First
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-005");
    Ok(())
}

#[test]
fn spc_006_fires_when_superseded_without_incoming_edge() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0017
        slug: x
        title: y
        status: superseded
        created: 2026-05-11
        ---

        ### REQ-001: First
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-006");
    Ok(())
}

#[test]
fn spc_006_does_not_fire_when_incoming_edge_exists() -> TestResult {
    let spec_md_old = indoc! {r"
        ---
        id: SPEC-0017
        slug: x
        title: y
        status: superseded
        created: 2026-05-11
        ---

        ### REQ-001: First
    "};
    let spec_md_new = indoc! {r"
        ---
        id: SPEC-0042
        slug: y
        title: y
        status: in-progress
        created: 2026-05-11
        supersedes:
          - SPEC-0017
        ---

        ### REQ-001: First
    "};
    let fx_old = write_spec_fixture(spec_md_old, &valid_spec_toml(), None)?;
    let fx_new = write_spec_fixture(spec_md_new, &valid_spec_toml(), None)?;
    let parsed_old = parse_fixture(&fx_old);
    let parsed_new = parse_fixture(&fx_new);

    let diags = run_lint(&[parsed_old, parsed_new]);
    assert_no_code(&diags, "SPC-006");
    Ok(())
}

#[test]
fn spc_007_fires_on_implemented_with_open_tasks() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: implemented
        created: 2026-05-11
        ---

        ### REQ-001: First
    "};
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **T-001**: still open
          - Covers: REQ-001
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), Some(tasks_md))?;
    let diags = lint_fixture(&fx);
    let info = diags
        .iter()
        .find(|d| d.code == "SPC-007")
        .ok_or("SPC-007 should fire")?;
    assert_eq!(info.level, Level::Info);
    Ok(())
}

#[test]
fn spc_007_does_not_fire_on_implemented_when_all_done() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: implemented
        created: 2026-05-11
        ---

        ### REQ-001: First
    "};
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [x] **T-001**: done
          - Covers: REQ-001
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), Some(tasks_md))?;
    let diags = lint_fixture(&fx);
    assert_no_code(&diags, "SPC-007");
    Ok(())
}
