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
//! End-to-end tests for `speccy status --include-archive` (SPEC-0042
//! T-006 / REQ-007). Also pins the negative invariant that
//! `--include-archive` is a `status`-only flag: every other hot-path
//! command (`next`, `check`, `verify`, `lock`) must reject it with a
//! clap exit-2 error.

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

/// Build a workspace with one `implemented` spec (SPEC-0001) and one
/// other active `implemented` spec (SPEC-0002), then archive SPEC-0001
/// via `speccy archive`. Returns the workspace so the test can run
/// further status commands against it.
fn workspace_with_one_archived_one_active() -> TestResult<Workspace> {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-artifact-parsers",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-active",
        &spec_md_template("SPEC-0002", "implemented"),
        None,
    )?;
    init_git_repo(&ws.root)?;

    // Archive SPEC-0001 via the real command so the on-disk layout
    // mirrors a post-archive workspace.
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();
    Ok(ws)
}

fn run_status(ws: &Workspace, args: &[&str]) -> TestResult<(Value, String, String)> {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(ws.root.as_std_path());
    let output = cmd.assert().success();
    let raw = output.get_output();
    let stdout = std::str::from_utf8(&raw.stdout)?.to_owned();
    let stderr = std::str::from_utf8(&raw.stderr)?.to_owned();
    if args.contains(&"--json") {
        let v: Value = serde_json::from_str(stdout.trim())?;
        Ok((v, stdout, stderr))
    } else {
        Ok((Value::Null, stdout, stderr))
    }
}

fn find_spec<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
    v.get("specs")?
        .as_array()?
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(id))
}

#[test]
fn status_json_default_omits_archived_spec() -> TestResult {
    let ws = workspace_with_one_archived_one_active()?;
    let (v, _stdout, _stderr) = run_status(&ws, &["--json"])?;
    let count = v.get("specs").and_then(Value::as_array).map_or(0, |a| {
        a.iter()
            .filter(|s| s.get("id").and_then(Value::as_str) == Some("SPEC-0001"))
            .count()
    });
    assert_eq!(
        count, 0,
        "archived SPEC-0001 must not appear without --include-archive: {v}"
    );
    // Active spec is present.
    assert!(
        find_spec(&v, "SPEC-0002").is_some(),
        "active SPEC-0002 must still appear"
    );
    Ok(())
}

#[test]
fn status_json_include_archive_surfaces_archived_spec_with_archived_at() -> TestResult {
    let ws = workspace_with_one_archived_one_active()?;
    let (v, _stdout, _stderr) = run_status(&ws, &["--include-archive", "--json"])?;
    let spec = find_spec(&v, "SPEC-0001")
        .unwrap_or_else(|| panic!("SPEC-0001 must appear with --include-archive: {v}"));
    let archived_at = spec
        .get("archived_at")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("archived_at must be present on archived spec: {spec}"));
    assert_eq!(
        archived_at.len(),
        10,
        "archived_at must be YYYY-MM-DD: {archived_at}"
    );
    // Active spec remains.
    assert!(find_spec(&v, "SPEC-0002").is_some());
    Ok(())
}

#[test]
fn status_json_all_include_archive_surfaces_both() -> TestResult {
    let ws = workspace_with_one_archived_one_active()?;
    let (v, _stdout, _stderr) = run_status(&ws, &["--all", "--include-archive", "--json"])?;
    let archived =
        find_spec(&v, "SPEC-0001").unwrap_or_else(|| panic!("archived SPEC-0001 must appear: {v}"));
    assert!(
        archived
            .get("archived_at")
            .and_then(Value::as_str)
            .is_some(),
        "archived_at must be populated on archived spec"
    );
    assert!(
        find_spec(&v, "SPEC-0002").is_some(),
        "active SPEC-0002 must appear under --all"
    );
    Ok(())
}

#[test]
fn status_text_include_archive_marks_archived_entry() -> TestResult {
    let ws = workspace_with_one_archived_one_active()?;
    let (_v, stdout, _stderr) = run_status(&ws, &["--all", "--include-archive"])?;
    assert!(
        stdout.contains("SPEC-0001"),
        "archived spec must appear in text output: {stdout}"
    );
    assert!(
        stdout.contains("[archived "),
        "archived marker must render in text output: {stdout}"
    );
    Ok(())
}

/// Helper: assert that the given command rejects `--include-archive`
/// with a clap exit-2 error mentioning the flag.
fn assert_rejects_include_archive(args: &[&str]) -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let mut cmd = Command::cargo_bin("speccy")?;
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--include-archive");
    cmd.current_dir(ws.root.as_std_path());
    let output = cmd.assert().code(2);
    let stderr = std::str::from_utf8(&output.get_output().stderr)?;
    assert!(
        stderr.contains("--include-archive") || stderr.contains("include-archive"),
        "stderr must name the offending flag: {stderr}"
    );
    Ok(())
}

#[test]
fn next_rejects_include_archive() -> TestResult {
    assert_rejects_include_archive(&["next"])
}

#[test]
fn check_rejects_include_archive() -> TestResult {
    assert_rejects_include_archive(&["check"])
}

#[test]
fn verify_rejects_include_archive() -> TestResult {
    assert_rejects_include_archive(&["verify"])
}

#[test]
fn lock_rejects_include_archive() -> TestResult {
    assert_rejects_include_archive(&["lock", "SPEC-0001"])
}
