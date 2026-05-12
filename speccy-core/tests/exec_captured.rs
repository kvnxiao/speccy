#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for [`speccy_core::exec::run_checks_captured`].
//!
//! Covers the captured-execution contract introduced by SPEC-0012:
//! per-check `CheckResult`, manual-check rendering without spawning,
//! and live-stream output to a caller-supplied writer.

use camino::Utf8PathBuf;
use speccy_core::exec::CheckOutcome;
use speccy_core::exec::CheckSpec;
use speccy_core::exec::run_checks_captured;
use tempfile::TempDir;

struct Sandbox {
    _dir: TempDir,
    root: Utf8PathBuf,
}

fn sandbox() -> Sandbox {
    let dir = tempfile::tempdir().expect("tempdir creation should succeed");
    let root =
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("tempdir path should be UTF-8");
    Sandbox { _dir: dir, root }
}

fn passing_spec(spec_id: &str, check_id: &str, command: &str) -> CheckSpec {
    CheckSpec {
        spec_id: spec_id.to_owned(),
        check_id: check_id.to_owned(),
        kind: "test".to_owned(),
        command: Some(command.to_owned()),
        prompt: None,
        proves: "proves something".to_owned(),
    }
}

fn manual_spec(spec_id: &str, check_id: &str, prompt: &str) -> CheckSpec {
    CheckSpec {
        spec_id: spec_id.to_owned(),
        check_id: check_id.to_owned(),
        kind: "manual".to_owned(),
        command: None,
        prompt: Some(prompt.to_owned()),
        proves: "manual claim".to_owned(),
    }
}

#[test]
fn executable_check_pass_returns_pass_result_and_emits_framing() {
    let sb = sandbox();
    let checks = vec![passing_spec("SPEC-0001", "CHK-001", "exit 0")];
    let mut err: Vec<u8> = Vec::new();

    let results = run_checks_captured(&checks, &sb.root, &mut err)
        .expect("run_checks_captured should succeed");
    assert_eq!(results.len(), 1);
    let r = results.first().expect("one result");
    assert_eq!(r.outcome, CheckOutcome::Pass);
    assert_eq!(r.exit_code, Some(0));
    assert_eq!(r.spec_id, "SPEC-0001");
    assert_eq!(r.check_id, "CHK-001");
    assert!(r.duration_ms.is_some());

    let captured = String::from_utf8(err).expect("err is utf-8");
    assert!(
        captured.contains("==> CHK-001 (SPEC-0001):"),
        "expected header on stderr; got:\n{captured}",
    );
    assert!(
        captured.contains("<-- CHK-001 PASS"),
        "expected pass footer; got:\n{captured}",
    );
}

#[test]
fn executable_check_failure_records_exit_code() {
    let sb = sandbox();
    let checks = vec![passing_spec("SPEC-0001", "CHK-001", "exit 2")];
    let mut err: Vec<u8> = Vec::new();

    let results = run_checks_captured(&checks, &sb.root, &mut err)
        .expect("run_checks_captured should succeed");
    let r = results.first().expect("one result");
    assert_eq!(r.outcome, CheckOutcome::Fail);
    assert_eq!(r.exit_code, Some(2));

    let captured = String::from_utf8(err).expect("err is utf-8");
    assert!(
        captured.contains("<-- CHK-001 FAIL (exit 2)"),
        "expected fail footer with exit code; got:\n{captured}",
    );
}

#[test]
fn manual_check_does_not_spawn_and_records_manual_outcome() {
    let sb = sandbox();
    let prompt = "Sign up via the UI and confirm error toast appears.";
    let checks = vec![manual_spec("SPEC-0001", "CHK-001", prompt)];
    let mut err: Vec<u8> = Vec::new();

    let results = run_checks_captured(&checks, &sb.root, &mut err)
        .expect("run_checks_captured should succeed");
    let r = results.first().expect("one result");
    assert_eq!(r.outcome, CheckOutcome::Manual);
    assert_eq!(r.exit_code, None);
    assert_eq!(r.duration_ms, None);

    let captured = String::from_utf8(err).expect("err is utf-8");
    assert!(captured.contains("==> CHK-001 (SPEC-0001, manual):"));
    assert!(captured.contains(prompt));
    assert!(captured.contains("<-- CHK-001 MANUAL (verify and proceed)"));
}

#[test]
fn child_stdout_is_captured_to_caller_writer() {
    let sb = sandbox();
    let checks = vec![passing_spec(
        "SPEC-0001",
        "CHK-001",
        "echo hello-from-child",
    )];
    let mut err: Vec<u8> = Vec::new();

    let _results = run_checks_captured(&checks, &sb.root, &mut err)
        .expect("run_checks_captured should succeed");

    let captured = String::from_utf8(err).expect("err is utf-8");
    assert!(
        captured.contains("hello-from-child"),
        "child stdout must reach the caller's writer; got:\n{captured}",
    );
}

#[test]
fn child_stderr_is_captured_to_caller_writer() {
    let sb = sandbox();
    // `1>&2` is supported by both `sh -c` and `cmd /c` for stderr
    // redirection, so a single command string works on both hosts.
    let checks = vec![passing_spec(
        "SPEC-0001",
        "CHK-001",
        "echo child-stderr 1>&2",
    )];
    let mut err: Vec<u8> = Vec::new();

    let _results = run_checks_captured(&checks, &sb.root, &mut err)
        .expect("run_checks_captured should succeed");

    let captured = String::from_utf8(err).expect("err is utf-8");
    assert!(
        captured.contains("child-stderr"),
        "child stderr must reach the caller's writer; got:\n{captured}",
    );
}

#[test]
fn all_checks_run_even_when_earlier_ones_fail() {
    let sb = sandbox();
    let checks = vec![
        passing_spec("SPEC-0001", "CHK-001", "exit 0"),
        passing_spec("SPEC-0001", "CHK-002", "exit 1"),
        passing_spec("SPEC-0001", "CHK-003", "exit 0"),
    ];
    let mut err: Vec<u8> = Vec::new();

    let results = run_checks_captured(&checks, &sb.root, &mut err)
        .expect("run_checks_captured should succeed");
    assert_eq!(results.len(), 3);
    let r0 = results.first().expect("first result");
    let r1 = results.get(1).expect("second result");
    let r2 = results.get(2).expect("third result");
    assert_eq!(r0.outcome, CheckOutcome::Pass);
    assert_eq!(r1.outcome, CheckOutcome::Fail);
    assert_eq!(r2.outcome, CheckOutcome::Pass);
}

#[test]
fn empty_input_returns_empty_results() {
    let sb = sandbox();
    let mut err: Vec<u8> = Vec::new();
    let results = run_checks_captured(&[], &sb.root, &mut err)
        .expect("run_checks_captured should succeed on empty input");
    assert!(results.is_empty());
    assert!(
        err.is_empty(),
        "no checks => no framing output; got: {err:?}"
    );
}
