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
use common::spec_md_template;
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

fn spec_toml_two_scenarios() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002"]

        [[checks]]
        id = "CHK-001"
        scenario = "Given the workspace, when CHK-001 is selected, then alpha is asserted."

        [[checks]]
        id = "CHK-002"
        scenario = "Given the workspace, when CHK-002 is selected, then beta is asserted."
    "#}
    .to_owned()
}

fn spec_toml_three_scenarios(spec_id_suffix: &str) -> String {
    format!(
        indoc! {r#"
            schema_version = 1

            [[requirements]]
            id = "REQ-001"
            checks = ["CHK-001", "CHK-002", "CHK-003"]

            [[checks]]
            id = "CHK-001"
            scenario = "first in {sid}"

            [[checks]]
            id = "CHK-002"
            scenario = "second in {sid}"

            [[checks]]
            id = "CHK-003"
            scenario = "third in {sid}"
        "#},
        sid = spec_id_suffix,
    )
}

fn spec_toml_multiline_scenario() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = """Given a multi-line scenario,
        when CHK-001 is rendered,
        then continuation lines are indented."""
    "#}
    .to_owned()
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0001"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0002"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-gamma",
        &spec_md_template("SPEC-0003", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0003"),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_multiline_scenario(),
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

#[test]
fn legacy_command_field_is_rejected_by_deny_unknown_fields() -> TestResult {
    let ws = Workspace::new()?;
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "Given a check, when it parses, then it accepts only id+scenario."
        command = "cargo test"
    "#};
    write_spec(
        &ws.root,
        "0001-legacy",
        &spec_md_template("SPEC-0001", "in-progress"),
        spec_toml_text,
        None,
    )?;

    let (_code, _out, err) = invoke(&ws.root, None)?;
    assert!(
        err.contains("spec.toml failed to parse"),
        "legacy `command` row should surface as a parse warning: {err}",
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0001"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0002"),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0001"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0002"),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0001"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-gamma",
        &spec_md_template("SPEC-0003", "in-progress"),
        &spec_toml_three_scenarios("SPEC-0003"),
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
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[requirements]]
        id = "REQ-002"
        checks = ["CHK-003"]

        [[checks]]
        id = "CHK-001"
        scenario = "covers REQ-001"

        [[checks]]
        id = "CHK-002"
        scenario = "unrelated"

        [[checks]]
        id = "CHK-003"
        scenario = "covers REQ-002"
    "#};
    let tasks = tasks_md_fixture("SPEC-0010", &[("T-001", "REQ-001"), ("T-002", "REQ-002")]);
    write_spec(
        &ws.root,
        "0010-task-coverage",
        &spec_md_template("SPEC-0010", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

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
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "alpha scenario"
    "#};
    let tasks = tasks_md_fixture("SPEC-0010", &[("T-007", "REQ-001")]);
    write_spec(
        &ws.root,
        "0010-alpha",
        &spec_md_template("SPEC-0010", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

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
    let ws = Workspace::new()?;
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-A"
        checks = ["CHK-001", "CHK-002"]

        [[requirements]]
        id = "REQ-B"
        checks = ["CHK-002", "CHK-003"]

        [[checks]]
        id = "CHK-001"
        scenario = "alpha"

        [[checks]]
        id = "CHK-002"
        scenario = "beta"

        [[checks]]
        id = "CHK-003"
        scenario = "gamma"
    "#};
    let tasks = tasks_md_fixture("SPEC-0020", &[("T-001", "REQ-A, REQ-B")]);
    write_spec(
        &ws.root,
        "0020-dedup",
        &spec_md_template("SPEC-0020", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_scenarios(),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_scenarios(),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_scenarios(),
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
    write_spec(
        &ws.root,
        "0001-broken",
        &spec_md_template("SPEC-0001", "in-progress"),
        "schema_version = 1\n[[checks]]\nthis is not valid toml = = =",
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-alpha",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_two_scenarios(),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(
        code, 1,
        "malformed spec.toml warning must contribute exit 1; out:\n{out}\nerr:\n{err}"
    );
    assert!(
        err.contains("SPEC-0001") && err.contains("spec.toml failed to parse"),
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
        &spec_md_template("SPEC-0001", "dropped"),
        &spec_toml_two_scenarios(),
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
        &spec_md_template("SPEC-0001", "dropped"),
        &spec_toml_two_scenarios(),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_scenarios(),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_scenarios(),
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
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_scenarios(),
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
//   1. Run `speccy check SPEC-0018` against this very repo and assert
//      it exits zero and prints the summary line. SPEC-0018's
//      `spec.toml` declares five CHK entries today; we assert the
//      summary count rather than each header so reordering /
//      additional checks within SPEC-0018 don't break the guard.
//   2. Static grep over `speccy-cli/src/check.rs` to assert no
//      `Command::new`, `process::Command`, or `.spawn(` reference
//      survives in the production check path. This is the
//      load-bearing assertion: it fails the suite if a future
//      contributor reintroduces subprocess execution into `check.rs`
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
