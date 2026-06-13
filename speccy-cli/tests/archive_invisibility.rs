#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end regression tests pinning the invariant that hot-path
//! commands (`status` default mode, `next`, `check`, `verify`, `lock`)
//! never reach into `.speccy/archive/`.
//!
//! Covers the archived-spec invisibility scenarios for `status --json`,
//! `next`, `check`, and `verify`, plus the supplementary `speccy lock`
//! invisibility scenario.

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

/// Build a workspace with SPEC-0001 (archived) and SPEC-0002 (active).
/// The archive step uses the real `speccy archive` command so the
/// on-disk layout mirrors a post-archive workspace.
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

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();
    Ok(ws)
}

#[test]
fn status_json_default_omits_archived_spec() -> TestResult {
    // status --json (no flags) → archived spec absent.
    let ws = workspace_with_one_archived_one_active()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let output = cmd.assert().success();
    let raw = output.get_output();
    let stdout = std::str::from_utf8(&raw.stdout)?.trim();
    let v: Value = serde_json::from_str(stdout)?;

    let specs = v
        .get("specs")
        .and_then(Value::as_array)
        .expect("`.specs` is an array");
    let archived_count = specs
        .iter()
        .filter(|s| s.get("id").and_then(Value::as_str) == Some("SPEC-0001"))
        .count();
    assert_eq!(
        archived_count, 0,
        "SPEC-0001 (archived) must be absent from default `status --json`: {stdout}"
    );
    let active_count = specs
        .iter()
        .filter(|s| s.get("id").and_then(Value::as_str) == Some("SPEC-0002"))
        .count();
    assert_eq!(
        active_count, 1,
        "SPEC-0002 (active) must remain visible: {stdout}"
    );
    Ok(())
}

#[test]
fn next_json_does_not_surface_archived_spec() -> TestResult {
    // `speccy next` discovers via the same active-only scan as
    // `status`; an archived spec must not appear as a next action.
    let ws = workspace_with_one_archived_one_active()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("next")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let output = cmd.assert();
    let raw = output.get_output();
    let stdout = std::str::from_utf8(&raw.stdout)?;
    assert!(
        !stdout.contains("SPEC-0001"),
        "archived SPEC-0001 must not surface in `speccy next --json`: {stdout}"
    );
    Ok(())
}

#[test]
fn check_archived_spec_exits_nonzero_with_not_found_message() -> TestResult {
    // `speccy check SPEC-0001` (archived) exits non-zero;
    // stderr names SPEC-0001 and indicates the spec is not present.
    let ws = workspace_with_one_archived_one_active()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("check")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().failure();
    let raw = assert.get_output();
    let stderr = std::str::from_utf8(&raw.stderr)?;
    assert!(
        stderr.contains("SPEC-0001"),
        "stderr must reference SPEC-0001: {stderr}"
    );
    // `speccy check` renders selector errors as
    // `no spec `SPEC-0001` found in workspace`, which matches the
    // requirement that the message indicate the spec is not found.
    assert!(
        stderr.contains("not found") || stderr.contains("no spec"),
        "stderr must indicate the spec is not found: {stderr}"
    );
    Ok(())
}

#[test]
fn verify_after_archive_exits_zero_and_omits_archived_spec() -> TestResult {
    // Pre-archive `verify` passes; after archiving SPEC-0001
    // (half of the implemented specs), `verify` still exits 0 and the
    // JSON output reflects only the still-active SPEC-0002.
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

    // Pre-archive: verify must exit 0 so the post-archive assertion
    // is meaningful (we are pinning that archive does not regress
    // verify from green).
    let mut pre = Command::cargo_bin("speccy")?;
    pre.arg("verify").current_dir(ws.root.as_std_path());
    pre.assert().success();

    // Archive SPEC-0001 via the real command.
    let mut arc = Command::cargo_bin("speccy")?;
    arc.arg("archive")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    arc.assert().success();

    // Post-archive: verify must still exit 0 (archive lint state is
    // ignored) and the JSON must list only SPEC-0002.
    let mut post = Command::cargo_bin("speccy")?;
    post.arg("verify")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let output = post.assert().success();
    let raw = output.get_output();
    let stdout = std::str::from_utf8(&raw.stdout)?.trim();
    assert!(
        !stdout.contains("SPEC-0001"),
        "archived SPEC-0001 must not appear in `verify --json`: {stdout}"
    );
    let v: Value = serde_json::from_str(stdout)?;
    let specs_total = v
        .pointer("/summary/shape/specs")
        .and_then(Value::as_u64)
        .expect("`/summary/shape/specs` is a number");
    assert_eq!(
        specs_total, 1,
        "verify must count only the still-active SPEC-0002: {stdout}"
    );
    Ok(())
}

#[test]
fn lock_archived_spec_exits_nonzero_and_leaves_archive_untouched() -> TestResult {
    // Supplementary scenario: `speccy lock SPEC-0001` against the
    // archived spec exits non-zero and does not mutate the archived
    // TASKS.md frontmatter. We seed TASKS.md *before* archiving so
    // the archived tree carries it, then capture its bytes and assert
    // they are unchanged after the failed lock.
    let ws = Workspace::new()?;
    let bootstrap_tasks = common::bootstrap_tasks_md("SPEC-0001");
    write_spec(
        &ws.root,
        "0001-artifact-parsers",
        &spec_md_template("SPEC-0001", "implemented"),
        Some(&bootstrap_tasks),
    )?;
    init_git_repo(&ws.root)?;

    // Archive SPEC-0001 via the real command.
    let mut arc = Command::cargo_bin("speccy")?;
    arc.arg("archive")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    arc.assert().success();

    let archived_tasks_path = ws
        .root
        .join(".speccy")
        .join("archive")
        .join("0001-artifact-parsers")
        .join("TASKS.md");
    let before = fs_err::read_to_string(archived_tasks_path.as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().failure();
    let raw = assert.get_output();
    let stderr = std::str::from_utf8(&raw.stderr)?;
    assert!(
        stderr.contains("SPEC-0001"),
        "stderr must reference SPEC-0001: {stderr}"
    );
    assert!(
        stderr.contains("not found"),
        "stderr must indicate the spec is not found: {stderr}"
    );

    let after = fs_err::read_to_string(archived_tasks_path.as_std_path())?;
    assert_eq!(
        before, after,
        "archived TASKS.md must be byte-identical after failed lock"
    );
    Ok(())
}
