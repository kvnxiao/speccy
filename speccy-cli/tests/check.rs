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
use speccy_cli::check_selector::SelectorError;

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

    assert!(out.contains("6 passed, 0 failed, 0 in-flight, 0 manual"));
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
    assert!(out.contains("4 passed, 0 failed, 0 in-flight, 0 manual"));
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
    assert!(out.contains("2 passed, 0 failed, 0 in-flight, 0 manual"));
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
    assert!(
        matches!(
            &err,
            CheckError::Selector(SelectorError::InvalidFormat { arg }) if arg == "FOO",
        ),
        "expected CheckError::Selector(InvalidFormat{{FOO}}), got {err:?}",
    );

    let err = invoke_expect_err(&ws.root, Some("chk-001"));
    assert!(
        matches!(
            &err,
            CheckError::Selector(SelectorError::InvalidFormat { arg }) if arg == "chk-001",
        ),
        "expected CheckError::Selector(InvalidFormat{{chk-001}}), got {err:?}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0017 T-002 selector wiring: FOO + CHK-099 surface behaviour
// ---------------------------------------------------------------------------

#[test]
fn binary_check_foo_exits_1_with_five_shape_hint() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("FOO"))
        .stderr(contains("SPEC-NNNN"))
        .stderr(contains("SPEC-NNNN/CHK-NNN"))
        .stderr(contains("SPEC-NNNN/T-NNN"))
        .stderr(contains("CHK-NNN"))
        .stderr(contains("T-NNN"));
    Ok(())
}

#[test]
fn binary_check_chk_099_no_match_preserves_no_check_matching_wording() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check")
        .arg("CHK-099")
        .current_dir(ws.root.as_std_path());
    // Wording is preserved verbatim from the existing CheckError::NoCheckMatching:
    //   "no check with id `CHK-099` found in workspace; run `speccy status` to list
    // specs"
    cmd.assert().failure().code(1).stderr(contains(
        "no check with id `CHK-099` found in workspace; run `speccy status` to list specs",
    ));
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
        .stdout(contains("1 passed, 0 failed, 0 in-flight, 0 manual"));
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
        // Implemented status: failures gate the exit code (regression
        // semantic). In-progress would categorise the failures as
        // in-flight and exit 0; that path is covered separately by
        // `in_progress_spec_failures_are_in_flight_not_gating`.
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_pass_fail_fail(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 2, "first non-zero (exit 2) wins; out:\n{out}");
    // All three checks must have run.
    assert!(out.contains("<-- CHK-001 PASS"));
    assert!(out.contains("<-- CHK-002 FAIL (exit 2)"));
    assert!(out.contains("<-- CHK-003 FAIL (exit 1)"));
    assert!(out.contains("1 passed, 2 failed, 0 in-flight, 0 manual"));
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
    assert!(out.contains("2 passed, 0 failed, 0 in-flight, 0 manual"));
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
        // Implemented so CHK-003's failure gates the exit code; the
        // in-progress / in-flight path is covered separately.
        &spec_md_template("SPEC-0001", "implemented"),
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
    assert!(out.contains("1 passed, 1 failed, 0 in-flight, 1 manual"));
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
    assert!(out.contains("0 passed, 0 failed, 0 in-flight, 1 manual"));
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
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_with_manual(),
        None,
    )?;

    let (_code, out, _err) = invoke(&ws.root, None)?;
    let last_line = out
        .lines()
        .last()
        .expect("output must have at least one line");
    assert_eq!(last_line, "1 passed, 1 failed, 0 in-flight, 1 manual");
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
        // Implemented so the failing checks gate the exit code.
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_pass_fail_fail(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check").current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}

#[test]
fn in_progress_spec_failures_are_in_flight_not_gating() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-mixed",
        // In-progress: failing checks are categorised as in-flight
        // and do NOT gate the exit code.
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_pass_fail_fail(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0, "in-progress failures must not gate; out:\n{out}");
    assert!(
        out.contains("<-- CHK-002 IN-FLIGHT (in-progress spec, exit 2)"),
        "footer must use IN-FLIGHT wording for in-progress failures: {out}",
    );
    assert!(out.contains("1 passed, 0 failed, 2 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn dropped_spec_is_skipped_entirely() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-dropped",
        &spec_md_template("SPEC-0001", "dropped"),
        &spec_toml_pass_fail_fail(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, None)?;
    assert_eq!(code, 0);
    assert!(
        !out.contains("CHK-001") && !out.contains("CHK-002") && !out.contains("CHK-003"),
        "no checks from a dropped spec should run: {out}",
    );
    assert!(out.contains("No checks defined."));
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

// ---------------------------------------------------------------------------
// SPEC-0017 T-003: CheckSelector::Spec (spec-scoped execution)
// ---------------------------------------------------------------------------
//
// Test names share the `spec_selector_` prefix so the CHK-002 invocation
// `cargo test -p speccy-cli --test check -- spec_selector` runs exactly these.

#[test]
fn spec_selector_runs_only_named_spec_checks_in_declared_order() -> TestResult {
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

    let (code, out, err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(code, 0, "all checks pass => exit 0; stderr={err}");

    let needles = [
        "==> CHK-001 (SPEC-0001)",
        "==> CHK-002 (SPEC-0001)",
        "==> CHK-003 (SPEC-0001)",
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
        "SPEC-0001 headers must appear in declared check order:\n{out}",
    );

    // SPEC-0002 must never appear in the output.
    assert!(
        !out.contains("SPEC-0002"),
        "SPEC-0002's checks must not appear under `speccy check SPEC-0001`:\n{out}",
    );

    assert!(out.contains("3 passed, 0 failed, 0 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn spec_selector_unknown_spec_errors_with_no_spec_matching() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
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
    // Display surface (drives stderr at the binary boundary).
    let rendered = format!("{err}");
    assert!(
        rendered.contains("SPEC-9999"),
        "Display must name SPEC-9999: {rendered}",
    );
    Ok(())
}

#[test]
fn spec_selector_dropped_spec_skips_with_status_line_no_subprocesses() -> TestResult {
    let ws = Workspace::new()?;
    // Use a side-effect command (write a sentinel into project root). If
    // skip semantics regress and the spec's checks run, the marker file
    // appears.
    let sentinel = "speccy-dropped-marker.txt";
    let echo_cmd = format!(r"echo dropped-should-not-run > {sentinel}");
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
            proves = "should never execute when spec is dropped"
        "#},
        cmd = echo_cmd,
    );
    write_spec(
        &ws.root,
        "0001-dropped",
        &spec_md_template("SPEC-0001", "dropped"),
        &spec_toml_text,
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(code, 0, "dropped skip path must exit 0; out:\n{out}");

    // The skip line names the spec and its status verbatim.
    assert!(
        out.contains("SPEC-0001") && out.contains("dropped") && out.contains("no checks executed"),
        "expected `SPEC-0001 ... dropped ... no checks executed` line; got:\n{out}",
    );
    // No execution framing, no summary line, no spawned subprocess.
    assert!(
        !out.contains("==> CHK-001") && !out.contains("<-- CHK-001"),
        "no check framing for a dropped spec: {out}",
    );
    assert!(
        !out.contains("passed,"),
        "skip path must not print a summary line: {out}",
    );
    let marker_path: Utf8PathBuf = ws.root.join(sentinel);
    assert!(
        !marker_path.exists(),
        "no subprocess should have run; marker at {marker_path} must not exist",
    );
    Ok(())
}

#[test]
fn spec_selector_superseded_spec_skips_with_status_line_no_subprocesses() -> TestResult {
    let ws = Workspace::new()?;
    let sentinel = "speccy-superseded-marker.txt";
    let echo_cmd = format!(r"echo superseded-should-not-run > {sentinel}");
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
            proves = "should never execute when spec is superseded"
        "#},
        cmd = echo_cmd,
    );
    write_spec(
        &ws.root,
        "0001-superseded",
        &spec_md_template("SPEC-0001", "superseded"),
        &spec_toml_text,
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(code, 0, "superseded skip path must exit 0; out:\n{out}");

    assert!(
        out.contains("SPEC-0001")
            && out.contains("superseded")
            && out.contains("no checks executed"),
        "expected `SPEC-0001 ... superseded ... no checks executed` line; got:\n{out}",
    );
    assert!(
        !out.contains("==> CHK-001") && !out.contains("<-- CHK-001"),
        "no check framing for a superseded spec: {out}",
    );
    assert!(
        !out.contains("passed,"),
        "skip path must not print a summary line: {out}",
    );
    let marker_path: Utf8PathBuf = ws.root.join(sentinel);
    assert!(
        !marker_path.exists(),
        "no subprocess should have run; marker at {marker_path} must not exist",
    );
    Ok(())
}

#[test]
fn spec_selector_in_progress_failure_categorised_in_flight() -> TestResult {
    let ws = Workspace::new()?;
    // spec_toml_pass_fail_fail() is 1 pass + 2 fails; this test wants
    // exactly 1 pass + 1 fail, so use an inline 2-check fixture instead.
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "passing"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 3"
        proves = "failing with code 3"
    "#};
    write_spec(
        &ws.root,
        "0001-mixed",
        &spec_md_template("SPEC-0001", "in-progress"),
        spec_toml_text,
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(
        code, 0,
        "in-progress failures must not gate the exit code; out:\n{out}",
    );
    assert!(out.contains("<-- CHK-001 PASS"), "out:\n{out}");
    assert!(
        out.contains("<-- CHK-002 IN-FLIGHT (in-progress spec, exit 3)"),
        "IN-FLIGHT wording expected for in-progress failure: {out}",
    );
    assert!(out.contains("1 passed, 0 failed, 1 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn spec_selector_implemented_failure_gates_exit_code() -> TestResult {
    let ws = Workspace::new()?;
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "passing"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 3"
        proves = "failing with code 3"
    "#};
    write_spec(
        &ws.root,
        "0001-mixed",
        &spec_md_template("SPEC-0001", "implemented"),
        spec_toml_text,
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0001"))?;
    assert_eq!(code, 3, "failing check's exit code must gate; out:\n{out}");
    assert!(out.contains("<-- CHK-001 PASS"), "out:\n{out}");
    assert!(out.contains("<-- CHK-002 FAIL (exit 3)"), "out:\n{out}");
    assert!(out.contains("1 passed, 1 failed, 0 in-flight, 0 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0017 T-004: CheckSelector::QualifiedCheck (spec-qualified single check)
// + DEC-003 preservation guards for bare CHK-NNN
// ---------------------------------------------------------------------------
//
// Test names share the `bare_chk_preserved_` prefix so the CHK-004 invocation
// `cargo test -p speccy-cli --test check -- bare_chk_preserved` runs exactly
// these. The `spec_toml_three(spec_id_suffix)` fixture deliberately gives
// every spec the same set of CHK-IDs so qualified-vs-bare comparisons are
// well-defined.

#[test]
fn bare_chk_preserved_runs_across_specs_in_ascending_order() -> TestResult {
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
    assert_eq!(code, 0, "both CHK-001s pass => exit 0; out:\n{out}");

    let needles = ["==> CHK-001 (SPEC-0001)", "==> CHK-001 (SPEC-0003)"];
    let positions_opt: Vec<Option<usize>> = needles.iter().map(|n| out.find(n)).collect();
    for (needle, pos) in needles.iter().zip(positions_opt.iter()) {
        assert!(pos.is_some(), "missing `{needle}` in output:\n{out}");
    }
    let positions: Vec<usize> = positions_opt.into_iter().flatten().collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted, positions,
        "bare CHK-001 headers must appear in spec-ascending order:\n{out}",
    );

    assert!(out.contains("2 passed, 0 failed, 0 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn bare_chk_preserved_qualified_runs_only_named_spec() -> TestResult {
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

    let (code, out, _err) = invoke(&ws.root, Some("SPEC-0003/CHK-001"))?;
    assert_eq!(code, 0, "single passing CHK => exit 0; out:\n{out}");

    assert!(
        out.contains("==> CHK-001 (SPEC-0003)"),
        "SPEC-0003 CHK-001 must execute: {out}",
    );
    assert!(
        !out.contains("==> CHK-001 (SPEC-0001)"),
        "SPEC-0001 CHK-001 must not execute under qualified selector: {out}",
    );
    assert!(out.contains("1 passed, 0 failed, 0 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn bare_chk_preserved_qualified_missing_check_errors_with_no_qualified_check_matching() -> TestResult
{
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let err = invoke_expect_err(&ws.root, Some("SPEC-0001/CHK-099"));
    assert!(
        matches!(
            &err,
            CheckError::Selector(SelectorError::NoQualifiedCheckMatching { spec_id, check_id })
                if spec_id == "SPEC-0001" && check_id == "CHK-099",
        ),
        "expected NoQualifiedCheckMatching{{SPEC-0001, CHK-099}}, got {err:?}",
    );
    let rendered = format!("{err}");
    assert!(
        rendered.contains("SPEC-0001") && rendered.contains("CHK-099"),
        "Display must name both SPEC-0001 and CHK-099: {rendered}",
    );
    Ok(())
}

#[test]
fn bare_chk_preserved_qualified_unknown_spec_errors_with_no_spec_matching() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_two_executables(),
        None,
    )?;

    let err = invoke_expect_err(&ws.root, Some("SPEC-9999/CHK-001"));
    assert!(
        matches!(
            &err,
            CheckError::Selector(SelectorError::NoSpecMatching { spec_id })
                if spec_id == "SPEC-9999",
        ),
        "expected NoSpecMatching{{SPEC-9999}}, got {err:?}",
    );
    let rendered = format!("{err}");
    assert!(
        rendered.contains("SPEC-9999"),
        "Display must name SPEC-9999: {rendered}",
    );
    Ok(())
}

#[test]
fn bare_chk_preserved_no_deprecation_or_ambiguity_hint_in_output() -> TestResult {
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

    let (code, out, err) = invoke(&ws.root, Some("CHK-001"))?;
    assert_eq!(code, 0, "out:\n{out}\nerr:\n{err}");

    // DEC-003: bare CHK-NNN stays first-class. The dispatcher must not
    // print any deprecation / ambiguity / migration hint when CHK-001
    // happens to appear in multiple specs.
    for forbidden in &["deprecated", "ambiguous", "use SPEC-NNNN/CHK-NNN"] {
        assert!(
            !out.contains(forbidden),
            "stdout must not contain `{forbidden}` per DEC-003; out:\n{out}",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0017 T-005: CheckSelector::Task (task-scoped execution)
// ---------------------------------------------------------------------------
//
// Test names share the `task_selector_` prefix so the CHK-003 invocation
// `cargo test -p speccy-cli --test check -- task_selector` runs exactly these.

/// Build a TASKS.md fixture for one spec where the listed tasks each declare
/// a `Covers:` bullet. `tasks` is a slice of (`task_id`, `covers_csv`) pairs;
/// the produced markdown matches the parser shape `task_lookup::find`
/// expects (bold task ID, indented `Covers:` sub-bullet).
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

#[test]
fn task_selector_qualified_runs_one_check_for_single_requirement() -> TestResult {
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
        kind = "test"
        command = "exit 0"
        proves = "covers REQ-001"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 0"
        proves = "unrelated"

        [[checks]]
        id = "CHK-003"
        kind = "test"
        command = "exit 0"
        proves = "covers REQ-002"
    "#};
    let tasks = tasks_md_fixture("SPEC-0010", &[("T-001", "REQ-001"), ("T-002", "REQ-002")]);
    write_spec(
        &ws.root,
        "0010-task-coverage",
        &spec_md_template("SPEC-0010", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

    let (code, out, err) = invoke(&ws.root, Some("SPEC-0010/T-002"))?;
    assert_eq!(code, 0, "single passing check => exit 0; stderr={err}");
    assert!(
        out.contains("==> CHK-003 (SPEC-0010)"),
        "CHK-003 header must appear: {out}",
    );
    assert!(
        !out.contains("==> CHK-001") && !out.contains("==> CHK-002"),
        "only CHK-003 must run: {out}",
    );
    assert!(out.contains("1 passed, 0 failed, 0 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn task_selector_dedups_in_first_occurrence_declared_order() -> TestResult {
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
        kind = "test"
        command = "exit 0"
        proves = "alpha"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 0"
        proves = "beta"

        [[checks]]
        id = "CHK-003"
        kind = "test"
        command = "exit 0"
        proves = "gamma"
    "#};
    let tasks = tasks_md_fixture("SPEC-0020", &[("T-001", "REQ-A, REQ-B")]);
    write_spec(
        &ws.root,
        "0020-dedup",
        &spec_md_template("SPEC-0020", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

    let (code, out, err) = invoke(&ws.root, Some("SPEC-0020/T-001"))?;
    assert_eq!(code, 0, "all pass => exit 0; stderr={err}");

    let needles = [
        "==> CHK-001 (SPEC-0020)",
        "==> CHK-002 (SPEC-0020)",
        "==> CHK-003 (SPEC-0020)",
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

    // CHK-002 must appear exactly once, not twice.
    let chk_002_count = out.matches("==> CHK-002 (SPEC-0020)").count();
    assert_eq!(
        chk_002_count, 1,
        "CHK-002 must run exactly once even though two REQs reference it: {out}",
    );

    assert!(out.contains("3 passed, 0 failed, 0 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn task_selector_unqualified_ambiguous_propagates_lookup_error() -> TestResult {
    let ws = Workspace::new()?;
    // Two specs, both define T-002. Unqualified `T-002` must be ambiguous.
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "alpha"
    "#};
    let tasks_10 = tasks_md_fixture("SPEC-0010", &[("T-001", "REQ-001"), ("T-002", "REQ-001")]);
    let tasks_11 = tasks_md_fixture("SPEC-0011", &[("T-001", "REQ-001"), ("T-002", "REQ-001")]);
    write_spec(
        &ws.root,
        "0010-alpha",
        &spec_md_template("SPEC-0010", "in-progress"),
        spec_toml_text,
        Some(&tasks_10),
    )?;
    write_spec(
        &ws.root,
        "0011-beta",
        &spec_md_template("SPEC-0011", "in-progress"),
        spec_toml_text,
        Some(&tasks_11),
    )?;

    let err = invoke_expect_err(&ws.root, Some("T-002"));
    let display = format!("{err}");
    // The wrapped LookupError::Ambiguous wording must reach Display.
    assert!(
        display.contains("T-002"),
        "Display must name T-002: {display}",
    );
    assert!(
        display.contains("ambiguous"),
        "Display must mention `ambiguous`: {display}",
    );
    assert!(
        display.contains("SPEC-0010") && display.contains("SPEC-0011"),
        "Display must list both candidate specs: {display}",
    );
    Ok(())
}

#[test]
fn task_selector_unqualified_not_found_propagates_lookup_error() -> TestResult {
    let ws = Workspace::new()?;
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "alpha"
    "#};
    let tasks = tasks_md_fixture("SPEC-0010", &[("T-001", "REQ-001")]);
    write_spec(
        &ws.root,
        "0010-alpha",
        &spec_md_template("SPEC-0010", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

    let err = invoke_expect_err(&ws.root, Some("T-099"));
    let display = format!("{err}");
    // `speccy status` reference is part of the existing LookupError::NotFound
    // wording and must be preserved verbatim through the wrap.
    assert!(
        display.contains("T-099") && display.contains("speccy status"),
        "Display must preserve LookupError::NotFound wording: {display}",
    );
    Ok(())
}

#[test]
fn task_selector_empty_covers_exits_zero_with_informational_message() -> TestResult {
    let ws = Workspace::new()?;
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "echo should-not-run > task-empty-covers-marker.txt"
        proves = "must never run for empty-covers task"
    "#};
    // Task with an explicitly empty covers list: no `Covers:` bullet at all
    // parses to `covers: vec![]`.
    let tasks_no_covers = "---\nspec: SPEC-0030\nspec_hash_at_generation: \
         bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n\
         # Tasks: SPEC-0030\n\n- [ ] **T-001**: empty covers\n";
    write_spec(
        &ws.root,
        "0030-empty",
        &spec_md_template("SPEC-0030", "in-progress"),
        spec_toml_text,
        Some(tasks_no_covers),
    )?;

    let (code, out, err) = invoke(&ws.root, Some("SPEC-0030/T-001"))?;
    assert_eq!(
        code, 0,
        "Lean 0: empty-covers is informational, exit 0; stderr={err}",
    );
    // No subprocess ran (marker file would exist otherwise).
    let marker_path: Utf8PathBuf = ws.root.join("task-empty-covers-marker.txt");
    assert!(
        !marker_path.exists(),
        "no subprocess should have spawned; marker at {marker_path} must not exist",
    );
    // No execution framing, no summary line.
    assert!(
        !out.contains("==> CHK-001") && !out.contains("<-- CHK-001"),
        "no check framing for empty-covers task: {out}",
    );
    assert!(
        !out.contains("passed,"),
        "empty-covers path must not print a summary line: {out}",
    );
    // Informational line names the task ref.
    assert!(
        out.contains("SPEC-0030/T-001") && out.contains("no requirements"),
        "expected informational line naming the task ref: {out}",
    );
    Ok(())
}

#[test]
fn task_selector_missing_chk_id_in_checks_block_is_silently_skipped() -> TestResult {
    let ws = Workspace::new()?;
    // REQ-001 lists CHK-001 and CHK-999; CHK-999 is missing from [[checks]].
    // The lint engine is the right place to flag the absence (SPEC-0003);
    // `speccy check` silently skips and runs whatever exists.
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-999"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "only CHK-001 is actually defined"
    "#};
    let tasks = tasks_md_fixture("SPEC-0040", &[("T-001", "REQ-001")]);
    write_spec(
        &ws.root,
        "0040-missing-chk",
        &spec_md_template("SPEC-0040", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

    let (code, out, err) = invoke(&ws.root, Some("SPEC-0040/T-001"))?;
    assert_eq!(
        code, 0,
        "missing CHK in [[checks]] is silently skipped; stderr={err}",
    );
    assert!(
        out.contains("==> CHK-001 (SPEC-0040)"),
        "CHK-001 must still run: {out}",
    );
    assert!(
        !out.contains("CHK-999"),
        "missing CHK-999 must not appear anywhere: {out}",
    );
    assert!(out.contains("1 passed, 0 failed, 0 in-flight, 0 manual"));
    Ok(())
}

#[test]
fn task_selector_in_progress_failure_categorised_in_flight() -> TestResult {
    let ws = Workspace::new()?;
    let spec_toml_text = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 2"
        proves = "failing covered check"
    "#};
    let tasks = tasks_md_fixture("SPEC-0050", &[("T-001", "REQ-001")]);
    write_spec(
        &ws.root,
        "0050-in-progress-fail",
        &spec_md_template("SPEC-0050", "in-progress"),
        spec_toml_text,
        Some(&tasks),
    )?;

    let (code, out, err) = invoke(&ws.root, Some("SPEC-0050/T-001"))?;
    assert_eq!(
        code, 0,
        "in-progress failures must not gate the exit code; stderr={err}",
    );
    assert!(
        out.contains("<-- CHK-001 IN-FLIGHT (in-progress spec, exit 2)"),
        "IN-FLIGHT wording must derive from parent spec status: {out}",
    );
    assert!(out.contains("0 passed, 0 failed, 1 in-flight, 0 manual"));
    Ok(())
}
