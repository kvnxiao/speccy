#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for the `speccy status` binary entry point.
//! Exercises argument parsing and exit codes. Covers SPEC-0004 T-011.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;

#[test]
fn status_runs_from_workspace_root() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status").current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    assert.stdout(contains("SPEC-0001"));
    Ok(())
}

#[test]
fn status_runs_from_nested_subdir() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    let nested = ws.root.join("a").join("b");
    fs_err::create_dir_all(nested.as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status").current_dir(nested.as_std_path());
    cmd.assert().success().stdout(contains("SPEC-0001"));
    Ok(())
}

#[test]
fn status_outside_workspace_exits_with_clear_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status").current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn status_json_emits_valid_json() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let output = cmd.assert().success().get_output().clone();
    let text = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));
    Ok(())
}

#[test]
fn unknown_arg_exits_with_usage_code() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}
