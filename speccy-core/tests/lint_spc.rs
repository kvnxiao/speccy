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

/// SPC-001 now fires when a stray per-spec `spec.toml` is present
/// (SPEC-0019 REQ-002) or when the SPEC.md marker tree fails to parse.
#[test]
fn spc_001_fires_when_stray_spec_toml_present() -> TestResult {
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), None)?;
    // Write a stray spec.toml next to the SPEC.md.
    let stray = fx.dir_path.join("spec.toml");
    fs_err::write(stray.as_std_path(), "schema_version = 1\n")?;

    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-001");
    Ok(())
}

#[test]
fn spc_002_fires_when_req_only_in_spec_md_heading() -> TestResult {
    // SPEC.md heading declares REQ-002, but only REQ-001 has a marker.
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        # Spec

        <goals>
        Goals.
        </goals>

        <non-goals>
        Non-goals.
        </non-goals>

        <user-stories>
        - A story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        scenario body
        </scenario>
        </requirement>

        ### REQ-002: Second
        Body.

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    let fx = write_spec_fixture(spec_md, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "SPC-002");
    Ok(())
}

#[test]
fn spc_004_fires_when_spec_md_missing_frontmatter() -> TestResult {
    let spec_md = "# No frontmatter\n";
    let fx = write_spec_fixture(spec_md, None)?;
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
    let fx = write_spec_fixture(spec_md, None)?;
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
    let fx = write_spec_fixture(spec_md, None)?;
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
    let fx_old = write_spec_fixture(spec_md_old, None)?;
    let fx_new = write_spec_fixture(spec_md_new, None)?;
    let parsed_old = parse_fixture(&fx_old);
    let parsed_new = parse_fixture(&fx_new);

    let diags = run_lint(&[parsed_old, parsed_new]);
    assert_no_code(&diags, "SPC-006");
    Ok(())
}

#[test]
fn spc_007_fires_on_implemented_with_open_tasks() -> TestResult {
    let spec_md = valid_spec_md("SPEC-0001").replace("status: in-progress", "status: implemented");
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **T-001**: still open
          - Covers: REQ-001
    "};
    let fx = write_spec_fixture(&spec_md, Some(tasks_md))?;
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
    let spec_md = valid_spec_md("SPEC-0001").replace("status: in-progress", "status: implemented");
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [x] **T-001**: done
          - Covers: REQ-001
    "};
    let fx = write_spec_fixture(&spec_md, Some(tasks_md))?;
    let diags = lint_fixture(&fx);
    assert_no_code(&diags, "SPC-007");
    Ok(())
}
