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

/// Helper that creates `.speccy/archive/<dir_name>/` inside `root`.
fn mkdir_archive(root: &camino::Utf8Path, dir_name: &str) -> TestResult {
    let dir = root.join(".speccy").join("archive").join(dir_name);
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
// SPEC-0042 CHK-012: vacancy unions `.speccy/specs/` and `.speccy/archive/`
// ---------------------------------------------------------------------------

#[test]
fn vacancy_json_archive_blocks_id_reuse() -> TestResult {
    let ws = Workspace::new()?;
    mkdir_spec(&ws.root, "0001-foo")?;
    mkdir_archive(&ws.root, "0002-bar")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("\"next_spec_id\":\"SPEC-0003\""));
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0042 CHK-013: archiving an active spec must not change next_spec_id
// (the archived spec still occupies its slot)
// ---------------------------------------------------------------------------

#[test]
fn vacancy_next_id_unchanged_when_spec_moves_from_specs_to_archive() -> TestResult {
    let ws = Workspace::new()?;
    mkdir_spec(&ws.root, "0001-foo")?;
    mkdir_spec(&ws.root, "0002-bar")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout("{\"schema_version\":1,\"next_spec_id\":\"SPEC-0003\"}\n");

    // Simulate `speccy archive SPEC-0001` by moving the directory
    // from `.speccy/specs/` to `.speccy/archive/`.
    let from = ws.root.join(".speccy").join("specs").join("0001-foo");
    let to_parent = ws.root.join(".speccy").join("archive");
    fs_err::create_dir_all(to_parent.as_std_path())?;
    fs_err::rename(from.as_std_path(), to_parent.join("0001-foo").as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout("{\"schema_version\":1,\"next_spec_id\":\"SPEC-0003\"}\n");
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0042 REQ-005: absent `.speccy/archive/` is treated as empty
// ---------------------------------------------------------------------------

#[test]
fn vacancy_with_no_archive_dir_uses_specs_only() -> TestResult {
    let ws = Workspace::new()?;
    mkdir_spec(&ws.root, "0001-foo")?;
    mkdir_spec(&ws.root, "0002-bar")?;
    // Note: no .speccy/archive/ created.

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("vacancy")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout("{\"schema_version\":1,\"next_spec_id\":\"SPEC-0003\"}\n");
    Ok(())
}

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
