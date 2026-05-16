#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy check`.
//!
//! SPEC-0018 REQ-002: `speccy check` renders English validation
//! scenarios. It must not spawn child processes — even when a legacy
//! `command` field is present during the SPEC-0018 bridge period.
//!
//! Selector behavior (SPEC-0017) is preserved: no selector, `SPEC-NNNN`,
//! `SPEC-NNNN/CHK-NNN`, `CHK-NNN`, `SPEC-NNNN/T-NNN`, `T-NNN`.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use common::Workspace;
use common::write_spec;
use indoc::indoc;
use predicates::str::contains;
use speccy_cli::check::CheckArgs;
use speccy_cli::check::CheckError;
use speccy_cli::check::run;
use speccy_cli::check_selector::SelectorError;

// ---------------------------------------------------------------------------
// Fixture helpers — all use the SPEC-0018 new-shape `scenario` field. A
// dedicated "legacy bridge" fixture lives further down for the
// no-subprocess-on-legacy-row test.
// ---------------------------------------------------------------------------

/// Marker-structured SPEC.md (SPEC-0019) with two scenarios under
/// REQ-001 to match the legacy `spec_toml_two_scenarios` shape.
fn marker_spec_md_two_scenarios(spec_id: &str, status: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: __STATUS__
        created: 2026-05-11
        ---

        # __ID__

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.
        <scenario id="CHK-001">
        Given the workspace, when CHK-001 is selected, then alpha is asserted.
        </scenario>
        <scenario id="CHK-002">
        Given the workspace, when CHK-002 is selected, then beta is asserted.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template
        .replace("__ID__", spec_id)
        .replace("__STATUS__", status)
}

/// Marker SPEC.md with three scenarios labelled "first/second/third in <spec>"
/// matching the legacy `spec_toml_three_scenarios` shape.
fn marker_spec_md_three_scenarios(spec_id: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-05-11
        ---

        # __ID__

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.
        <scenario id="CHK-001">
        first in __ID__
        </scenario>
        <scenario id="CHK-002">
        second in __ID__
        </scenario>
        <scenario id="CHK-003">
        third in __ID__
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template.replace("__ID__", spec_id)
}

/// Marker SPEC.md with one multi-line scenario.
fn marker_spec_md_multiline_scenario(spec_id: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-05-11
        ---

        # __ID__

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.
        <scenario id="CHK-001">
        Given a multi-line scenario,
        when CHK-001 is rendered,
        then continuation lines are indented.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template.replace("__ID__", spec_id)
}

fn invoke(root: &Utf8Path, selector: Option<&str>) -> TestResult<(i32, String, String)> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        CheckArgs {
            selector: selector.map(ToOwned::to_owned),
        },
        root,
        &mut out,
        &mut err,
    )?;
    Ok((code, String::from_utf8(out)?, String::from_utf8(err)?))
}

fn invoke_expect_err(root: &Utf8Path, selector: Option<&str>) -> CheckError {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    run(
        CheckArgs {
            selector: selector.map(ToOwned::to_owned),
        },
        root,
        &mut out,
        &mut err,
    )
    .expect_err("expected CheckError")
}

fn tasks_md_fixture(spec_id: &str, tasks: &[(&str, &str)]) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: \
         bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n\
         # Tasks: {spec_id}\n\n",
    );
    for (task_id, covers) in tasks {
        writeln!(out, "- [ ] **{task_id}**: stub\n  - Covers: {covers}\n")
            .expect("writeln to String must not fail");
    }
    out
}

fn assert_no_legacy_footers(out: &str) {
    for forbidden in &["<-- CHK", "PASS", "FAIL", "IN-FLIGHT", "MANUAL"] {
        assert!(
            !out.contains(forbidden),
            "legacy execution footer `{forbidden}` must be absent in render-only output:\n{out}",
        );
    }
}

// ---------------------------------------------------------------------------
// No-selector run: headers + count summary
// ---------------------------------------------------------------------------

#[test]
fn no_selector_renders_all_scenarios_with_count_summary() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_three_scenarios("SPEC-0001"),
        "",
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &marker_spec_md_three_scenarios("SPEC-0002"),
        "",
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-gamma",
        &marker_spec_md_three_scenarios("SPEC-0003"),
        "",
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(
        code, 0,
        "render-only never gates on scenarios; stderr={err}"
    );

    // Six headers across the first two specs, plus three for SPEC-0003.
    let needles = [
        "==> CHK-001 (SPEC-0001): first in SPEC-0001",
        "==> CHK-002 (SPEC-0001): second in SPEC-0001",
        "==> CHK-003 (SPEC-0001): third in SPEC-0001",
        "==> CHK-001 (SPEC-0002): first in SPEC-0002",
        "==> CHK-002 (SPEC-0002): second in SPEC-0002",
        "==> CHK-003 (SPEC-0002): third in SPEC-0002",
        "==> CHK-001 (SPEC-0003): first in SPEC-0003",
        "==> CHK-002 (SPEC-0003): second in SPEC-0003",
        "==> CHK-003 (SPEC-0003): third in SPEC-0003",
    ];
    for needle in &needles {
        assert!(out.contains(needle), "missing `{needle}` in output:\n{out}");
    }

    // Summary line: nine scenarios across three specs.
    assert!(
        out.contains("9 scenarios rendered across 3 specs"),
        "count summary missing or wrong; out:\n{out}",
    );
    assert_no_legacy_footers(&out);
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiline scenario: first line in header, continuations indented
// ---------------------------------------------------------------------------

#[test]
fn multiline_scenario_header_then_indented_continuations() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-multiline",
        &marker_spec_md_multiline_scenario("SPEC-0001"),
        "",
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);

    assert!(
        out.contains("==> CHK-001 (SPEC-0001): Given a multi-line scenario,"),
        "first line must appear in the header; out:\n{out}",
    );
    assert!(
        out.contains("  when CHK-001 is rendered,"),
        "continuation must be indented; out:\n{out}",
    );
    assert!(
        out.contains("  then continuation lines are indented."),
        "final continuation must be indented; out:\n{out}",
    );
    assert!(out.contains("1 scenarios rendered across 1 specs"));
    assert_no_legacy_footers(&out);
    Ok(())
}

// ---------------------------------------------------------------------------
// Legacy field hard break: post-SPEC-0018 a spec.toml row that still
// carries `kind`, `command`, `prompt`, or `proves` must fail
// deserialization via `#[serde(deny_unknown_fields)]` on `RawCheck`.
// ---------------------------------------------------------------------------

/// Post-SPEC-0019 the analogue to the SPEC-0018 "legacy `command`
/// field" hard break is: a stray per-spec `spec.toml` (regardless of
/// content) surfaces as a parse warning on `speccy check`. The
/// underlying parse error variant changed from `Toml` to
/// `StraySpecToml`.
#[test]
fn legacy_command_field_is_rejected_by_deny_unknown_fields() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-legacy",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        // Stray spec.toml content is irrelevant — its presence alone is
        // the SPEC-0019 violation.
        "schema_version = 1\n",
        None,
    )?;

    let (_code, _out, err) = invoke(&ws.root, None)?;
    assert!(
        err.contains("SPEC.md marker tree failed to parse")
            && err.contains("stray per-spec spec.toml"),
        "stray spec.toml should surface as a parse warning: {err}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Selector shapes (SPEC-0017): preserved set of accepted forms
// ---------------------------------------------------------------------------

#[test]
fn spec_selector_renders_only_named_spec() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_three_scenarios("SPEC-0001"),
        "",
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &marker_spec_md_three_scenarios("SPEC-0002"),
        "",
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(code, 0);
    assert!(out.contains("==> CHK-001 (SPEC-0001)"));
    assert!(out.contains("==> CHK-002 (SPEC-0001)"));
    assert!(out.contains("==> CHK-003 (SPEC-0001)"));
    assert!(
        !out.contains("(SPEC-0002)"),
        "SPEC-0002 must not appear: {out}",
    );
    assert!(out.contains("3 scenarios rendered across 1 specs"));
    Ok(())
}

#[test]
fn qualified_check_selector_renders_one() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_three_scenarios("SPEC-0001"),
        "",
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &marker_spec_md_three_scenarios("SPEC-0002"),
        "",
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0002/CHK-002"))?;
    assert_eq!(code, 0);
    assert!(out.contains("==> CHK-002 (SPEC-0002): second in SPEC-0002"));
    assert!(
        !out.contains("==> CHK-001") && !out.contains("==> CHK-003"),
        "only CHK-002 must render: {out}",
    );
    assert!(out.contains("1 scenarios rendered across 1 specs"));
    Ok(())
}

#[test]
fn bare_chk_selector_renders_across_specs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_three_scenarios("SPEC-0001"),
        "",
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-gamma",
        &marker_spec_md_three_scenarios("SPEC-0003"),
        "",
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("CHK-001"))?;
    assert_eq!(code, 0);
    assert!(out.contains("==> CHK-001 (SPEC-0001)"));
    assert!(out.contains("==> CHK-001 (SPEC-0003)"));
    assert!(!out.contains("==> CHK-002"));
    assert!(out.contains("2 scenarios rendered across 2 specs"));
    Ok(())
}

#[test]
fn qualified_task_selector_renders_covered_scenarios() -> TestResult {
    let ws = Workspace::new()?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0010
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0010

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        covers REQ-001
        </scenario>
        <scenario id="CHK-002">
        unrelated
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        body
        <scenario id="CHK-003">
        covers REQ-002
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    let tasks = tasks_md_fixture("SPEC-0010", &[("T-001", "REQ-001"), ("T-002", "REQ-002")]);
    write_spec(&ws.root, "0010-task-coverage", spec_md, "", Some(&tasks))?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0010/T-002"))?;
    assert_eq!(code, 0);
    assert!(out.contains("==> CHK-003 (SPEC-0010): covers REQ-002"));
    assert!(
        !out.contains("==> CHK-001") && !out.contains("==> CHK-002"),
        "only CHK-003 must render: {out}",
    );
    assert!(out.contains("1 scenarios rendered across 1 specs"));
    Ok(())
}

#[test]
fn unqualified_task_selector_renders_covered_scenarios() -> TestResult {
    let ws = Workspace::new()?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0010
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0010

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        alpha scenario
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    let tasks = tasks_md_fixture("SPEC-0010", &[("T-007", "REQ-001")]);
    write_spec(&ws.root, "0010-alpha", spec_md, "", Some(&tasks))?;

    let (code, out, _err) = invoke(&ws.root, Some("T-007"))?;
    assert_eq!(code, 0);
    assert!(out.contains("==> CHK-001 (SPEC-0010): alpha scenario"));
    assert!(out.contains("1 scenarios rendered across 1 specs"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Task selector with overlapping check lists: first-occurrence order, dedup
// ---------------------------------------------------------------------------

#[test]
fn task_selector_dedups_overlapping_checks_in_first_occurrence_order() -> TestResult {
    // After SPEC-0019 a scenario is owned by exactly one requirement
    // (marker containment), so the legacy "two REQs reference the same
    // CHK" scenario can't be constructed at the marker level. The test
    // is preserved as a regression guard for first-occurrence ordering
    // across requirements covered by one task; the dedup-on-overlap
    // assertion below still holds vacuously.
    let ws = Workspace::new()?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0020
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0020

        <requirement id="REQ-100">
        ### REQ-100: First
        body
        <scenario id="CHK-001">
        alpha
        </scenario>
        <scenario id="CHK-002">
        beta
        </scenario>
        </requirement>
        <requirement id="REQ-200">
        ### REQ-200: Second
        body
        <scenario id="CHK-003">
        gamma
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    let tasks = tasks_md_fixture("SPEC-0020", &[("T-001", "REQ-100, REQ-200")]);
    write_spec(&ws.root, "0020-dedup", spec_md, "", Some(&tasks))?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0020/T-001"))?;
    assert_eq!(code, 0);

    let needles = [
        "==> CHK-001 (SPEC-0020): alpha",
        "==> CHK-002 (SPEC-0020): beta",
        "==> CHK-003 (SPEC-0020): gamma",
    ];
    let positions_opt: Vec<Option<usize>> = needles.iter().map(|n| out.find(n)).collect();
    for (needle, pos) in needles.iter().zip(positions_opt.iter()) {
        assert!(pos.is_some(), "missing `{needle}` in output:\n{out}");
    }
    let positions: Vec<usize> = positions_opt.into_iter().flatten().collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted, positions,
        "headers must appear in CHK-001, CHK-002, CHK-003 order:\n{out}",
    );

    let chk_002_count = out.matches("==> CHK-002 (SPEC-0020)").count();
    assert_eq!(
        chk_002_count, 1,
        "CHK-002 must render exactly once even though two REQs reference it: {out}",
    );

    assert!(out.contains("3 scenarios rendered across 1 specs"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Error surfaces (selector / lookup / parse / workspace)
// ---------------------------------------------------------------------------

#[test]
fn unknown_spec_preserves_no_spec_matching_wording() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let err = invoke_expect_err(&ws.root, Some("SPEC-9999"));
    assert!(
        matches!(
            &err,
            CheckError::Selector(SelectorError::NoSpecMatching { spec_id })
                if spec_id == "SPEC-9999",
        ),
        "expected NoSpecMatching{{SPEC-9999}}, got {err:?}",
    );
    let rendered = format!("{err}");
    assert!(rendered.contains("SPEC-9999"), "Display: {rendered}");
    Ok(())
}

#[test]
fn unknown_check_id_errors_with_no_check_matching() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let err = invoke_expect_err(&ws.root, Some("CHK-099"));
    assert!(matches!(
        err,
        CheckError::NoCheckMatching { ref id } if id == "CHK-099"
    ));
    Ok(())
}

#[test]
fn malformed_selector_errors() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let err = invoke_expect_err(&ws.root, Some("FOO"));
    assert!(
        matches!(
            &err,
            CheckError::Selector(SelectorError::InvalidFormat { arg }) if arg == "FOO",
        ),
        "expected InvalidFormat{{FOO}}, got {err:?}",
    );
    Ok(())
}

#[test]
fn empty_workspace_prints_no_checks_defined() -> TestResult {
    let ws = Workspace::new()?;
    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(out.contains("No checks defined."), "out:\n{out}");
    assert!(err.is_empty(), "stderr should be empty: {err}");
    // Empty workspace path uses `No checks defined.`, not a summary line.
    assert!(
        !out.contains("scenarios rendered"),
        "empty workspace must not print a summary: {out}",
    );
    Ok(())
}

#[test]
fn malformed_spec_toml_warns_and_other_specs_render() -> TestResult {
    let ws = Workspace::new()?;
    // After SPEC-0019 the equivalent "malformed" condition for `speccy
    // check` is a SPEC.md marker tree that fails to parse. Use a SPEC.md
    // missing the required `speccy:changelog` marker to trip the parser.
    let broken_spec_md = indoc! {r#"
        ---
        id: SPEC-0001
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0001

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        scenario
        </scenario>
        </requirement>
    "#};
    write_spec(&ws.root, "0001-broken", broken_spec_md, "", None)?;
    write_spec(
        &ws.root,
        "0002-alpha",
        &marker_spec_md_two_scenarios("SPEC-0002", "in-progress"),
        "",
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(
        code, 1,
        "malformed SPEC.md marker tree must contribute exit 1; out:\n{out}\nerr:\n{err}"
    );
    assert!(
        err.contains("SPEC-0001") && err.contains("SPEC.md marker tree failed to parse"),
        "stderr should name SPEC-0001: {err}",
    );
    assert!(out.contains("2 scenarios rendered across 1 specs"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Dropped / superseded specs: explicit-skip path
// ---------------------------------------------------------------------------

#[test]
fn dropped_spec_skipped_in_run_all() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-dropped",
        &marker_spec_md_two_scenarios("SPEC-0001", "dropped"),
        "",
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(
        !out.contains("CHK-001") && !out.contains("CHK-002"),
        "dropped spec scenarios must not render: {out}",
    );
    assert!(out.contains("No checks defined."));
    Ok(())
}

#[test]
fn dropped_spec_named_directly_surfaces_skip() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-dropped",
        &marker_spec_md_two_scenarios("SPEC-0001", "dropped"),
        "",
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(code, 0);
    assert!(
        out.contains("SPEC-0001") && out.contains("dropped") && out.contains("no checks rendered"),
        "expected explicit skip line; got:\n{out}",
    );
    assert!(!out.contains("==> CHK-"), "no headers for dropped: {out}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Binary-boundary smoke: outside workspace + propagated exit code
// ---------------------------------------------------------------------------

#[test]
fn check_outside_workspace_fails() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check").current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn binary_renders_headers_and_summary() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check").current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("==> CHK-001 (SPEC-0001)"))
        .stdout(contains("==> CHK-002 (SPEC-0001)"))
        .stdout(contains("2 scenarios rendered across 1 specs"));
    Ok(())
}

#[test]
fn binary_chk_099_no_match_preserves_no_check_matching_wording() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check")
        .arg("CHK-099")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(1).stderr(contains(
        "no check with id `CHK-099` found in workspace; run `speccy status` to list specs",
    ));
    Ok(())
}

#[test]
fn binary_spec_9999_preserves_no_matching_spec_wording() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &marker_spec_md_two_scenarios("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check")
        .arg("SPEC-9999")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(1).stderr(contains("SPEC-9999"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0018 T-006 retry / REQ-002 regression guard: `speccy check
// SPEC-0018` must render scenarios without spawning child processes.
//
// Two assertions paired in one test because they cover the same
// contract from complementary angles:
//
//   1. Run `speccy check SPEC-0018` against this very repo and assert it exits
//      zero and prints the summary line. SPEC-0018's `spec.toml` declares five
//      CHK entries today; we assert the summary count rather than each header
//      so reordering / additional checks within SPEC-0018 don't break the
//      guard.
//   2. Static grep over `speccy-cli/src/check.rs` to assert no `Command::new`,
//      `process::Command`, or `.spawn(` reference survives in the production
//      check path. This is the load-bearing assertion: it fails the suite if a
//      future contributor reintroduces subprocess execution into `check.rs`
//      regardless of whether the runtime test still happens to pass.
// ---------------------------------------------------------------------------

/// Source text of `speccy-cli/src/check.rs`, pulled in at compile time
/// so the grep guard below cannot be bypassed by file-system fakery.
const CHECK_SOURCE: &str = include_str!("../src/check.rs");

#[test]
fn check_spec_0018_renders_scenarios_without_spawning_processes() -> TestResult {
    // (1) Static guard: the production `check.rs` must not reference
    // any subprocess-spawning API. Needles cover the common forms a
    // regression would take.
    let forbidden_needles: [&str; 3] = ["Command::new", "process::Command", ".spawn("];
    for needle in forbidden_needles {
        assert!(
            !CHECK_SOURCE.contains(needle),
            "speccy-cli/src/check.rs must not contain `{needle}` \
             (SPEC-0018 REQ-002: `speccy check` renders only, never spawns)",
        );
    }

    // (2) Runtime guard: invoke `speccy check SPEC-0018` against this
    // repo and assert the summary line. The repo root is two levels
    // up from `CARGO_MANIFEST_DIR` (`speccy-cli/`).
    let manifest_dir = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .ok_or("CARGO_MANIFEST_DIR has no parent (expected workspace root)")?
        .to_path_buf();

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args(["check", "SPEC-0018"])
        .current_dir(repo_root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("scenarios rendered across"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0019 T-006: `speccy check` reads scenario text from SPEC.md marker
// bodies (byte-exact, not via a stale TOML mirror).
// ---------------------------------------------------------------------------

#[test]
fn check_task_prints_scenario_body_bytes_from_element_block() -> TestResult {
    // Construct a SPEC.md with a multi-line scenario body whose interior
    // bytes are easy to assert verbatim. The XML element parser
    // preserves body bytes between the open and close tags (with
    // whitespace-only boundary trim), so the stdout continuation lines
    // must equal those bytes line-for-line.
    let ws = Workspace::new()?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0099

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        Given a task covers REQ-001,
        when speccy check runs against that task,
        then the scenario body bytes are printed verbatim.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    let tasks = tasks_md_fixture("SPEC-0099", &[("T-001", "REQ-001")]);
    write_spec(&ws.root, "0099-element-body", spec_md, "", Some(&tasks))?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0099/T-001"))?;
    assert_eq!(code, 0);

    // Locate the scenario body in the source file. The body is the text
    // between `<scenario id="CHK-001">\n` and `\n</scenario>`.
    let start_tag = "<scenario id=\"CHK-001\">\n";
    let end_tag = "</scenario>";
    let after_start = spec_md
        .find(start_tag)
        .map(|i| i + start_tag.len())
        .expect("fixture must contain CHK-001 open tag");
    let tail = spec_md
        .get(after_start..)
        .expect("after_start must be a valid char boundary in fixture");
    let before_end = tail
        .find(end_tag)
        .map(|j| after_start + j)
        .expect("fixture must contain matching close tag");
    let body_bytes = spec_md
        .get(after_start..before_end)
        .expect("body slice must lie on valid char boundaries in fixture")
        .trim_end_matches('\n');

    // Every non-empty line of the scenario body must appear in the
    // rendered output (header takes the first line; the rest are
    // indented continuations, so we substring-match line-by-line).
    for line in body_bytes.lines() {
        assert!(
            out.contains(line),
            "speccy check stdout must contain scenario body line `{line}`; got:\n{out}",
        );
    }
    Ok(())
}

#[test]
fn check_duplicate_scenario_id_across_requirements_is_surfaced_as_parse_warning() -> TestResult {
    // Two `speccy:scenario` markers with the same id across two
    // requirement blocks: the marker parser rejects this with
    // `DuplicateMarkerId`, and `speccy check` should surface it as a
    // per-spec warning (non-zero exit because malformed > 0).
    let ws = Workspace::new()?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0098
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0098

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        first
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        body
        <scenario id="CHK-001">
        duplicate
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    write_spec(&ws.root, "0098-dup-chk", spec_md, "", None)?;

    let (code, _out, err) = invoke(&ws.root, None)?;
    assert_eq!(
        code, 1,
        "duplicate scenario id must surface as a non-zero exit from check",
    );
    assert!(
        err.contains("CHK-001"),
        "warning must name the duplicated scenario id; got: {err}",
    );
    Ok(())
}
