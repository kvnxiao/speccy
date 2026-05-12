#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy check`. Covers SPEC-0010 CHK-001..CHK-008.

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

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn spec_toml_two_executables() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "first check"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 0"
        proves = "second check"
    "#}
    .to_owned()
}

fn spec_toml_three(spec_id_suffix: &str) -> String {
    format!(
        indoc! {r#"
            schema_version = 1

            [[requirements]]
            id = "REQ-001"
            checks = ["CHK-001", "CHK-002", "CHK-003"]

            [[checks]]
            id = "CHK-001"
            kind = "test"
            command = "exit 0"
            proves = "first in {sid}"

            [[checks]]
            id = "CHK-002"
            kind = "test"
            command = "exit 0"
            proves = "second in {sid}"

            [[checks]]
            id = "CHK-003"
            kind = "test"
            command = "exit 0"
            proves = "third in {sid}"
        "#},
        sid = spec_id_suffix,
    )
}

fn spec_toml_pass_fail_fail() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002", "CHK-003"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "passing"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 2"
        proves = "failing with code 2"

        [[checks]]
        id = "CHK-003"
        kind = "test"
        command = "exit 1"
        proves = "failing with code 1"
    "#}
    .to_owned()
}

fn spec_toml_with_manual() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002", "CHK-003"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "executable pass"

        [[checks]]
        id = "CHK-002"
        kind = "manual"
        prompt = "Click the signup button; confirm no errors appear."
        proves = "manual UI check"

        [[checks]]
        id = "CHK-003"
        kind = "test"
        command = "exit 1"
        proves = "executable fail"
    "#}
    .to_owned()
}

fn echo_hello_spec_toml() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "echo hello"
        proves = "echo prints hello live"
    "#}
    .to_owned()
}

fn invoke(root: &Utf8Path, id: Option<&str>) -> TestResult<(i32, String, String)> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        CheckArgs {
            id: id.map(ToOwned::to_owned),
        },
        root,
        &mut out,
        &mut err,
    )?;
    Ok((code, String::from_utf8(out)?, String::from_utf8(err)?))
}

fn invoke_expect_err(root: &Utf8Path, id: Option<&str>) -> CheckError {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    run(
        CheckArgs {
            id: id.map(ToOwned::to_owned),
        },
        root,
        &mut out,
        &mut err,
    )
    .expect_err("expected CheckError")
}

// ---------------------------------------------------------------------------
// CHK-001: workspace discovery
// ---------------------------------------------------------------------------

#[test]
fn discovers_workspace_checks() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_three("SPEC-0001"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_three("SPEC-0002"),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0, "all checks pass => exit 0; stderr={err}");

    // Ordering: SPEC-0001 first (three checks), then SPEC-0002 (three checks).
    let needles = [
        "==> CHK-001 (SPEC-0001)",
        "==> CHK-002 (SPEC-0001)",
        "==> CHK-003 (SPEC-0001)",
        "==> CHK-001 (SPEC-0002)",
        "==> CHK-002 (SPEC-0002)",
        "==> CHK-003 (SPEC-0002)",
    ];
    let positions_opt: Vec<Option<usize>> = needles.iter().map(|n| out.find(n)).collect();
    for (needle, pos) in needles.iter().zip(positions_opt.iter()) {
        assert!(pos.is_some(), "missing `{needle}` in output:\n{out}");
    }
    let positions: Vec<usize> = positions_opt.into_iter().flatten().collect();
    assert_eq!(positions.len(), needles.len());

    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted, positions,
        "headers must appear in (spec ascending, declared check order):\n{out}"
    );

    assert!(out.contains("6 passed, 0 failed, 0 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-002: empty workspace + malformed spec.toml warning
// ---------------------------------------------------------------------------

#[test]
fn empty_workspace_prints_no_checks_defined() -> TestResult {
    let ws = Workspace::new()?;
    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(out.contains("No checks defined."), "out:\n{out}");
    assert!(
        err.is_empty(),
        "stderr should be empty when no specs; got:\n{err}"
    );
    Ok(())
}

#[test]
fn workspace_with_specs_but_no_checks_prints_no_checks_defined() -> TestResult {
    let ws = Workspace::new()?;
    let spec_toml_no_checks = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = []
    "#};
    write_spec(
        &ws.root,
        "0001-empty",
        &spec_md_template("SPEC-0001", "in-progress"),
        spec_toml_no_checks,
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(out.contains("No checks defined."));
    Ok(())
}

#[test]
fn malformed_spec_toml_warns_and_other_specs_run() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0001: malformed (invalid TOML on purpose).
    write_spec(
        &ws.root,
        "0001-broken",
        &spec_md_template("SPEC-0001", "in-progress"),
        "schema_version = 1\n[[checks]]\nthis is not valid toml = = =",
        None,
    )?;
    // SPEC-0002 + SPEC-0003: well-formed.
    write_spec(
        &ws.root,
        "0002-alpha",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-beta",
        &spec_md_template("SPEC-0003", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, None)?;
    assert_eq!(
        code, 1,
        "malformed spec.toml warning must contribute exit 1; out:\n{out}\nerr:\n{err}"
    );
    assert!(
        err.contains("SPEC-0001") && err.contains("spec.toml failed to parse"),
        "stderr should name SPEC-0001 and call out spec.toml; got:\n{err}"
    );
    // 4 checks across SPEC-0002 + SPEC-0003 still ran.
    assert!(out.contains("4 passed, 0 failed, 0 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-003: CHK-ID filtering
// ---------------------------------------------------------------------------

#[test]
fn id_filter_matches_across_specs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_three("SPEC-0001"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-gamma",
        &spec_md_template("SPEC-0003", "in-progress"),
        &spec_toml_three("SPEC-0003"),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("CHK-001"))?;
    assert_eq!(code, 0);
    assert!(out.contains("==> CHK-001 (SPEC-0001)"));
    assert!(out.contains("==> CHK-001 (SPEC-0003)"));
    assert!(!out.contains("CHK-002"));
    assert!(!out.contains("CHK-003"));
    assert!(out.contains("2 passed, 0 failed, 0 manual"));
    Ok(())
}

#[test]
fn id_filter_unknown_id_errors() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
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
fn id_filter_malformed_format_errors() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let err = invoke_expect_err(&ws.root, Some("FOO"));
    assert!(matches!(
        err,
        CheckError::InvalidCheckIdFormat { ref arg } if arg == "FOO"
    ));

    let err = invoke_expect_err(&ws.root, Some("chk-001"));
    assert!(matches!(err, CheckError::InvalidCheckIdFormat { .. }));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-004 / CHK-005: shell selection, cwd, live streaming
// ---------------------------------------------------------------------------

#[test]
fn shell_executes_in_project_root() -> TestResult {
    let ws = Workspace::new()?;
    // Echo a sentinel via redirect into a file in the project root, then assert
    // the file exists at the expected path.
    let sentinel = "speccy-cwd-marker.txt";
    let echo_cmd = format!(r"echo cwd-marker > {sentinel}");
    let spec_toml_text = format!(
        indoc! {r#"
            schema_version = 1

            [[requirements]]
            id = "REQ-001"
            checks = ["CHK-001"]

            [[checks]]
            id = "CHK-001"
            kind = "test"
            command = {cmd:?}
            proves = "write marker into project root"
        "#},
        cmd = echo_cmd,
    );
    write_spec(
        &ws.root,
        "0001-cwd",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_text,
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    let marker_path: Utf8PathBuf = ws.root.join(sentinel);
    assert!(
        marker_path.exists(),
        "child should have written sentinel into project root at {marker_path}"
    );
    Ok(())
}

#[test]
fn live_streaming_smoke_via_binary() -> TestResult {
    // assert_cmd captures the speccy binary's stdout, which inherits the
    // child shell's stdout when speccy spawns its checks.
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-echo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &echo_hello_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check").current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("==> CHK-001 (SPEC-0001)"))
        .stdout(contains("hello"))
        .stdout(contains("<-- CHK-001 PASS"))
        .stdout(contains("1 passed, 0 failed, 0 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-006: exit-code aggregation (run-all; first non-zero wins)
// ---------------------------------------------------------------------------

#[test]
fn exit_code_first_nonzero_wins() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-mixed",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_pass_fail_fail(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 2, "first non-zero (exit 2) wins; out:\n{out}");
    // All three checks must have run.
    assert!(out.contains("<-- CHK-001 PASS"));
    assert!(out.contains("<-- CHK-002 FAIL (exit 2)"));
    assert!(out.contains("<-- CHK-003 FAIL (exit 1)"));
    assert!(out.contains("1 passed, 2 failed, 0 manual"));
    Ok(())
}

#[test]
fn all_passing_returns_zero() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(out.contains("2 passed, 0 failed, 0 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-007: manual rendering (no subprocess; no exit-code impact)
// ---------------------------------------------------------------------------

#[test]
fn manual_check_renders_prompt_and_footer() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-manual",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_with_manual(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    // CHK-003 fails with exit 1; manual CHK-002 doesn't affect that.
    assert_eq!(code, 1);

    assert!(out.contains("==> CHK-002 (SPEC-0001, manual):"));
    assert!(out.contains("Click the signup button"));
    assert!(out.contains("<-- CHK-002 MANUAL (verify and proceed)"));

    // Manual count appears in summary.
    assert!(out.contains("1 passed, 1 failed, 1 manual"));
    Ok(())
}

#[test]
fn manual_only_workspace_returns_zero() -> TestResult {
    let ws = Workspace::new()?;
    let manual_only = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "manual"
        prompt = "Verify."
        proves = "manual"
    "#};
    write_spec(
        &ws.root,
        "0001-manual",
        &spec_md_template("SPEC-0001", "in-progress"),
        manual_only,
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(out.contains("0 passed, 0 failed, 1 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-008: summary line is the final stdout line
// ---------------------------------------------------------------------------

#[test]
fn summary_is_last_stdout_line() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-mix",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_with_manual(),
        None,
    )?;

    let (_code, out, _err) = invoke(&ws.root, None)?;
    let last_line = out
        .lines()
        .last()
        .expect("output must have at least one line");
    assert_eq!(last_line, "1 passed, 1 failed, 1 manual");
    Ok(())
}

#[test]
fn empty_workspace_uses_no_checks_defined_not_summary() -> TestResult {
    let ws = Workspace::new()?;
    let (_code, out, _err) = invoke(&ws.root, None)?;
    assert!(out.contains("No checks defined."));
    assert!(
        !out.contains("passed,"),
        "empty workspace must not print a summary line; out:\n{out}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Binary dispatcher: outside-workspace + exit-code mapping smoke
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
fn binary_propagates_first_nonzero_exit() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-mixed",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_pass_fail_fail(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check").current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}

#[test]
fn binary_rejects_unknown_flag() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}
