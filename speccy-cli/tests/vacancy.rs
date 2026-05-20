#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! End-to-end tests for `speccy vacancy [--json]` (SPEC-0033 REQ-003).
//!
//! Exercises CHK-005 (json output with mission folder), CHK-006
//! (no workspace exits 1) and additional scenarios from T-003
//! task-scenarios: empty specs dir (text form) and `--help` listing.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use predicates::str::contains;

/// Helper that creates `.speccy/specs/<dir_name>/` inside `root`.
fn mkdir_spec(root: &camino::Utf8Path, dir_name: &str) -> TestResult {
    let dir = root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-005: json output with flat + mission-folder specs
// ---------------------------------------------------------------------------

#[test]
fn vacancy_json_with_flat_and_mission_specs() -> TestResult {
    let ws = Workspace::new()?;
    mkdir_spec(&ws.root, "0001-foo")?;
    mkdir_spec(&ws.root, "0027-bar")?;
    mkdir_spec(&ws.root, "0032-baz")?;
    // mission folder: auth/0033-signup/
    mkdir_spec(&ws.root, "auth/0033-signup")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout("{\"schema_version\":1,\"next_spec_id\":\"SPEC-0034\"}\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// T-003 scenario 2: empty specs dir → text output is SPEC-0001
// ---------------------------------------------------------------------------

#[test]
fn vacancy_empty_specs_dir_returns_spec_0001() -> TestResult {
    let ws = Workspace::new()?;
    // Create an empty specs dir
    let specs_dir = ws.root.join(".speccy").join("specs");
    fs_err::create_dir_all(specs_dir.as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy").current_dir(ws.root.as_std_path());
    cmd.assert().success().stdout("SPEC-0001\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-006: no .speccy/ directory → exits 1, stderr contains expected message
// ---------------------------------------------------------------------------

#[test]
fn vacancy_outside_workspace_exits_one_with_not_found_message() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy").current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

// ---------------------------------------------------------------------------
// T-003 scenario 4: `speccy --help` lists `vacancy` alongside `lock`
// ---------------------------------------------------------------------------

#[test]
fn vacancy_appears_in_help_subcommands() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("vacancy"))
        .stdout(contains("lock"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional: text output for workspace with a few flat specs
// ---------------------------------------------------------------------------

#[test]
fn vacancy_text_output_is_bare_spec_id() -> TestResult {
    let ws = Workspace::new()?;
    mkdir_spec(&ws.root, "0001-foo")?;
    mkdir_spec(&ws.root, "0032-baz")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy").current_dir(ws.root.as_std_path());
    cmd.assert().success().stdout("SPEC-0033\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional: json form also works for simple workspace
// ---------------------------------------------------------------------------

#[test]
fn vacancy_json_simple_workspace() -> TestResult {
    let ws = Workspace::new()?;
    mkdir_spec(&ws.root, "0001-foo")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout("{\"schema_version\":1,\"next_spec_id\":\"SPEC-0002\"}\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional: stdout is empty on workspace-not-found failure
// ---------------------------------------------------------------------------

#[test]
fn vacancy_outside_workspace_stdout_is_empty() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy").current_dir(path.as_std_path());
    cmd.assert().failure().stdout("");
    Ok(())
}
