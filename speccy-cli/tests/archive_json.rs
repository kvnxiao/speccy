#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
#![expect(
    clippy::panic,
    reason = "test helpers panic on malformed fixture JSON; tests are infallible setup-side"
)]
//! End-to-end tests for `speccy archive --json` receipt output shape.
//!
//! Covers SPEC-0042 T-003 / REQ-009 / CHK-023, CHK-024.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use serde_json::Value;

fn init_git_repo(root: &Utf8Path) -> TestResult {
    let run = |args: &[&str]| -> TestResult {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root.as_std_path())
            .status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    };
    run(&["init", "-q"])?;
    run(&["config", "user.email", "test@example.com"])?;
    run(&["config", "user.name", "Test"])?;
    run(&["config", "commit.gpgsign", "false"])?;
    run(&["add", "-A"])?;
    run(&["commit", "-q", "-m", "init"])?;
    Ok(())
}

fn ptr<'a>(v: &'a Value, path: &str) -> &'a Value {
    v.pointer(path)
        .unwrap_or_else(|| panic!("missing JSON pointer {path} in {v}"))
}

#[test]
fn archive_json_with_reason_emits_full_receipt() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-artifact-parsers",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0001")
        .arg("--json")
        .arg("--reason")
        .arg("ship cleanup")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?.trim();

    let v: Value = serde_json::from_str(stdout)?;
    assert_eq!(ptr(&v, "/schema_version"), &serde_json::json!(1));
    assert_eq!(ptr(&v, "/archived/id"), &serde_json::json!("SPEC-0001"));
    assert_eq!(
        ptr(&v, "/archived/slug"),
        &serde_json::json!("0001-artifact-parsers")
    );
    assert_eq!(
        ptr(&v, "/archived/archived_reason"),
        &serde_json::json!("ship cleanup")
    );
    assert_eq!(
        ptr(&v, "/archived/to"),
        &serde_json::json!(".speccy/archive/0001-artifact-parsers")
    );
    assert_eq!(
        ptr(&v, "/archived/from"),
        &serde_json::json!(".speccy/specs/0001-artifact-parsers")
    );
    assert!(
        ptr(&v, "/archived/archived_at")
            .as_str()
            .is_some_and(|s| s.len() == 10),
        "archived_at must be YYYY-MM-DD: {stdout}"
    );
    assert_eq!(ptr(&v, "/warnings"), &serde_json::json!([]));
    Ok(())
}

#[test]
fn archive_json_without_reason_emits_null_reason() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-artifact-parsers",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0001")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?.trim();

    // Must contain literal `null`, not omit the key, not be `"null"`.
    assert!(
        stdout.contains("\"archived_reason\":null"),
        "expected literal `archived_reason:null`: {stdout}"
    );

    let v: Value = serde_json::from_str(stdout)?;
    let archived = ptr(&v, "/archived");
    let reason = archived.get("archived_reason");
    assert!(reason.is_some(), "archived_reason key must be present");
    assert!(
        reason.is_some_and(Value::is_null),
        "archived_reason must be JSON null"
    );
    assert_eq!(ptr(&v, "/warnings"), &serde_json::json!([]));
    Ok(())
}

#[test]
fn archive_json_on_in_progress_without_force_fails_with_empty_stdout() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-archive",
        &spec_md_template("SPEC-0042", "in-progress"),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0042")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().failure();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;
    assert!(
        stdout.is_empty(),
        "stdout must be empty on failure under --json: {stdout:?}"
    );
    assert!(!stderr.is_empty(), "stderr must carry human-readable error");
    assert!(stderr.contains("in-progress"), "stderr: {stderr}");
    Ok(())
}

#[test]
fn archive_json_paths_use_forward_slashes() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-artifact-parsers",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0001")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?.trim();
    let v: Value = serde_json::from_str(stdout)?;
    let from = ptr(&v, "/archived/from").as_str().expect("from is string");
    let to = ptr(&v, "/archived/to").as_str().expect("to is string");
    assert!(
        !from.contains('\\'),
        "from must use forward slashes: {from}"
    );
    assert!(!to.contains('\\'), "to must use forward slashes: {to}");
    Ok(())
}
