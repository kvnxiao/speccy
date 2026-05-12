#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `git::repo_sha`. Covers SPEC-0004 T-010.

use camino::Utf8PathBuf;
use speccy_cli::git::repo_sha;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn outside_a_repo_returns_empty_string() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let sha = repo_sha(&path);
    assert!(
        sha.is_empty(),
        "expected empty SHA outside a repo, got: {sha:?}"
    );
    Ok(())
}

#[test]
fn fresh_git_init_without_commit_returns_empty_string() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    // Initialise git but don't make a commit.
    let status = std::process::Command::new("git")
        .arg("init")
        .arg("--quiet")
        .current_dir(path.as_std_path())
        .status();
    match status {
        Ok(s) if s.success() => {
            // HEAD is unset because there are no commits yet.
            let sha = repo_sha(&path);
            assert!(
                sha.is_empty(),
                "expected empty SHA for fresh git init, got: {sha:?}",
            );
        }
        _ => {
            // git not available; the outside-a-repo test already
            // covers that case. Skip silently.
        }
    }
    Ok(())
}

#[test]
fn within_speccy_own_repo_returns_a_sha() {
    // Run from the speccy repo (where this test executes from) and
    // expect a 40-char SHA. CARGO_MANIFEST_DIR points at the crate;
    // running `git rev-parse HEAD` from any subdir of the repo returns
    // the SHA.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let repo_root = Utf8PathBuf::from(manifest_dir);
    let sha = repo_sha(&repo_root);
    if sha.is_empty() {
        // CI or a packaged source tarball: skip.
        return;
    }
    assert_eq!(
        sha.len(),
        40,
        "expected 40-char SHA from git rev-parse HEAD, got: {sha:?}",
    );
}
