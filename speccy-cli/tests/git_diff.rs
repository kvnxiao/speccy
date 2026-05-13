#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `git::diff_for_review`. Covers SPEC-0009 CHK-005.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_cli::git::NO_DIFF_FALLBACK;
use speccy_cli::git::diff_for_review;
use std::fs;
use std::process::Command;
use std::process::Stdio;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn make_tmp_root() -> TestResult<(tempfile::TempDir, Utf8PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir must be UTF-8: {}", p.display()))?;
    Ok((tmp, path))
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn git(args: &[&str], cwd: &Utf8Path) -> TestResult {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd.as_std_path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    assert!(status.success(), "git {args:?} failed in {cwd}");
    Ok(())
}

fn init_repo(root: &Utf8Path) -> TestResult {
    git(&["init", "--quiet", "-b", "main"], root)?;
    git(&["config", "user.email", "test@example.com"], root)?;
    git(&["config", "user.name", "Test"], root)?;
    git(&["config", "commit.gpgsign", "false"], root)?;
    Ok(())
}

fn write(root: &Utf8Path, name: &str, content: &str) -> TestResult {
    fs::write(root.join(name).as_std_path(), content)?;
    Ok(())
}

fn commit_all(root: &Utf8Path, message: &str) -> TestResult {
    git(&["add", "-A"], root)?;
    git(&["commit", "-q", "-m", message], root)?;
    Ok(())
}

#[test]
fn outside_a_repo_returns_fallback_note() -> TestResult {
    let (_tmp, root) = make_tmp_root()?;
    assert_eq!(diff_for_review(&root), NO_DIFF_FALLBACK);
    Ok(())
}

#[test]
fn uncommitted_edits_produce_non_empty_diff() -> TestResult {
    if !git_available() {
        return Ok(());
    }
    let (_tmp, root) = make_tmp_root()?;
    init_repo(&root)?;
    write(&root, "a.txt", "first line\n")?;
    commit_all(&root, "initial")?;
    // Now introduce an uncommitted change.
    write(&root, "a.txt", "first line\nsecond line\n")?;

    let diff = diff_for_review(&root);
    assert_ne!(
        diff, NO_DIFF_FALLBACK,
        "expected diff content for uncommitted edits, got fallback"
    );
    assert!(
        diff.contains("second line"),
        "diff should contain the new line, got: {diff:?}",
    );
    Ok(())
}

#[test]
fn clean_tree_falls_back_to_head_vs_head_tilde_one() -> TestResult {
    if !git_available() {
        return Ok(());
    }
    let (_tmp, root) = make_tmp_root()?;
    init_repo(&root)?;
    write(&root, "a.txt", "first line\n")?;
    commit_all(&root, "initial")?;
    write(&root, "a.txt", "first line\nsecond line\n")?;
    commit_all(&root, "add second line")?;

    // Tree is clean, but HEAD~1 exists.
    let diff = diff_for_review(&root);
    assert_ne!(
        diff, NO_DIFF_FALLBACK,
        "expected HEAD vs HEAD~1 diff content, got fallback"
    );
    assert!(
        diff.contains("second line"),
        "fallback diff should contain content from the most recent commit, got: {diff:?}",
    );
    Ok(())
}

#[test]
fn clean_tree_single_commit_returns_fallback_note() -> TestResult {
    if !git_available() {
        return Ok(());
    }
    let (_tmp, root) = make_tmp_root()?;
    init_repo(&root)?;
    write(&root, "a.txt", "only commit\n")?;
    commit_all(&root, "initial")?;

    // Clean tree, no HEAD~1. Both fallbacks empty.
    assert_eq!(
        diff_for_review(&root),
        NO_DIFF_FALLBACK,
        "single-commit clean repo must hit the literal fallback note"
    );
    Ok(())
}
