#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy archive SPEC-NNNN`.
//!
//! Covers the directory move, `archived_at`/`archived_reason`
//! frontmatter, force/refusal behaviour, and missing-spec handling. The
//! `git status --porcelain` rename assertion is exercised in
//! [`archive_git_rename`] below.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use predicates::str::contains;

/// Initialise a git repo at `root` and stage + commit all existing
/// files. Required because `speccy archive` shells out to `git mv`,
/// which only works inside a repo.
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

fn archive_dir(root: &Utf8Path, dir_name: &str) -> Utf8PathBuf {
    root.join(".speccy").join("archive").join(dir_name)
}

fn spec_dir(root: &Utf8Path, dir_name: &str) -> Utf8PathBuf {
    root.join(".speccy").join("specs").join(dir_name)
}

#[test]
fn archive_implemented_spec_moves_dir_and_writes_archived_at() -> TestResult {
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
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    assert!(
        !spec_dir(&ws.root, "0001-artifact-parsers")
            .as_std_path()
            .exists(),
        "source dir should be gone"
    );
    let dest = archive_dir(&ws.root, "0001-artifact-parsers");
    assert!(dest.as_std_path().exists(), "archive dir should exist");
    let spec_text = fs_err::read_to_string(dest.join("SPEC.md").as_std_path())?;
    assert!(
        spec_text.contains("archived_at: "),
        "archived_at line missing: {spec_text}"
    );
    Ok(())
}

#[test]
fn archive_in_progress_spec_without_force_refuses() -> TestResult {
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
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .stderr(contains("in-progress"))
        .stderr(contains("implemented"))
        .stderr(contains("dropped"))
        .stderr(contains("superseded"));

    assert!(
        spec_dir(&ws.root, "0042-archive").as_std_path().exists(),
        "source dir must remain on refusal"
    );
    assert!(
        !archive_dir(&ws.root, "0042-archive").as_std_path().exists(),
        "no archive dir should be created on refusal"
    );
    Ok(())
}

#[test]
fn archive_in_progress_spec_with_force_moves_and_keeps_status() -> TestResult {
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
        .arg("--force")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let dest = archive_dir(&ws.root, "0042-archive").join("SPEC.md");
    let body = fs_err::read_to_string(dest.as_std_path())?;
    assert!(
        body.lines().any(|l| l.trim() == "status: in-progress"),
        "status must remain in-progress under --force: {body}"
    );
    Ok(())
}

#[test]
fn archive_superseded_spec_without_force_succeeds() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0050-old",
        &spec_md_template("SPEC-0050", "superseded"),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0050")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();
    assert!(archive_dir(&ws.root, "0050-old").as_std_path().exists());
    Ok(())
}

#[test]
fn archive_missing_spec_id_exits_nonzero() -> TestResult {
    let ws = Workspace::new()?;
    // Need at least one tracked file so `git commit` succeeds.
    fs_err::write(ws.root.join(".speccy/.gitkeep").as_std_path(), b"")?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-9999")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().stderr(contains("SPEC-9999"));
    Ok(())
}

#[test]
fn archive_without_positional_clap_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive").current_dir(ws.root.as_std_path());
    let assert = cmd.assert().failure();
    let output = assert.get_output();
    assert_eq!(
        output.status.code(),
        Some(2),
        "clap arg-parse failure should exit 2"
    );
    Ok(())
}

#[test]
fn archive_reason_with_newline_rejected_at_parse_time() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0001")
        .arg("--reason")
        .arg("line1\nline2")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().failure();
    let output = assert.get_output();
    assert_eq!(output.status.code(), Some(2), "clap exit 2 expected");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--reason") || stderr.contains("reason"),
        "stderr should name --reason: {stderr}"
    );
    Ok(())
}

#[test]
fn archive_with_reason_writes_reason_field() -> TestResult {
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
        .arg("--reason")
        .arg("ship cleanup")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let dest = archive_dir(&ws.root, "0001-artifact-parsers").join("SPEC.md");
    let body = fs_err::read_to_string(dest.as_std_path())?;
    assert!(
        body.contains("archived_reason: \"ship cleanup\""),
        "expected reason in frontmatter: {body}"
    );
    // archived_at must come before archived_reason.
    let at_idx = body.find("archived_at:").expect("archived_at present");
    let reason_idx = body
        .find("archived_reason:")
        .expect("archived_reason present");
    assert!(at_idx < reason_idx);
    Ok(())
}

#[test]
fn archive_without_reason_omits_reason_field() -> TestResult {
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
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let dest = archive_dir(&ws.root, "0001-artifact-parsers").join("SPEC.md");
    let body = fs_err::read_to_string(dest.as_std_path())?;
    assert!(
        !body.contains("archived_reason:"),
        "archived_reason should be absent: {body}"
    );
    let at_matches = body.matches("archived_at:").count();
    assert_eq!(at_matches, 1, "exactly one archived_at expected");
    Ok(())
}

#[test]
fn archive_git_rename_is_recorded() -> TestResult {
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
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(ws.root.as_std_path())
        .output()?;
    let porcelain = String::from_utf8_lossy(&output.stdout).into_owned();
    // After `git mv`, we expect at least one rename entry (`R `) referencing
    // both the old and new path. The `git mv` mutated SPEC.md as well (we
    // wrote archive_at before the move), so git may show the entry as
    // rename+modify; the leading status code starts with `R`.
    assert!(
        porcelain
            .lines()
            .any(|l| l.starts_with("R ") || l.starts_with("RM")),
        "expected at least one rename entry; got: {porcelain}"
    );
    Ok(())
}
