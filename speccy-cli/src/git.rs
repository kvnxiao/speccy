//! Best-effort git shell-outs.
//!
//! Helpers:
//!
//! - [`repo_sha`] — returns `HEAD`'s SHA for the JSON status contract.
//! - [`suggested_diff_command`] — returns a `git diff` command string in
//!   merge-base form against the repository's default branch, for the `speccy
//!   context` bundle.
//!
//! Treats git unavailability as a non-fatal lookup: shell-out failures
//! degrade rather than propagating an error — `repo_sha` to the empty
//! string, `suggested_diff_command` to a `main`-baseline fallback that is
//! still runnable as-is.

use camino::Utf8Path;
use std::process::Command;
use std::process::Stdio;

/// Default branch name used when the default-branch probe cannot resolve a
/// remote `origin/HEAD` (no remote, detached HEAD, git unavailable). The
/// suggested diff command stays runnable as-is against this baseline.
const DEFAULT_BRANCH_FALLBACK: &str = "main";

/// Look up `HEAD`'s SHA in the git repository containing `cwd`.
///
/// Returns `""` (the empty string) when:
/// - `git` is not on PATH (the spawn itself fails);
/// - the command exits non-zero (not a repo, HEAD missing);
/// - the captured stdout is not valid UTF-8 or is unexpectedly empty.
#[must_use = "the SHA is part of the status JSON contract"]
pub fn repo_sha(cwd: &Utf8Path) -> String {
    let Ok(output) = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(cwd.as_std_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    let Ok(text) = std::str::from_utf8(&output.stdout) else {
        return String::new();
    };
    text.trim().to_owned()
}

/// Build the suggested `git diff` command string for the `speccy context`
/// bundle, in merge-base form against the repository's default branch.
///
/// The returned string is runnable as-is from the repo root:
/// `git diff <base>...HEAD` where `<base>` is the resolved default branch
/// (e.g. `origin/main`). The triple-dot form diffs against the merge-base
/// of `<base>` and `HEAD`, which is exactly the feature-branch change set a
/// reviewer wants — commits unique to `HEAD`, ignoring later commits on the
/// base branch.
///
/// Best-effort, never fatal: when the default-branch probe cannot resolve a
/// remote `origin/HEAD` (no remote, detached HEAD, or `git` not on PATH),
/// the command falls back to the [`DEFAULT_BRANCH_FALLBACK`] baseline so the
/// field is still populated with a runnable command rather than errored.
#[must_use = "the suggested diff command is part of the context bundle contract"]
pub fn suggested_diff_command(cwd: &Utf8Path) -> String {
    let base = default_branch(cwd);
    format!("git diff {base}...HEAD")
}

/// Probe the repository's default branch via `git symbolic-ref
/// refs/remotes/origin/HEAD`, returning a ref usable as a diff baseline
/// (e.g. `origin/main`).
///
/// Returns [`DEFAULT_BRANCH_FALLBACK`] (`main`) when:
/// - `git` is not on PATH (the spawn itself fails);
/// - the command exits non-zero (no `origin` remote, no `origin/HEAD` ref);
/// - the captured stdout is not valid UTF-8 or is unexpectedly empty.
///
/// The probe returns the short ref form (`origin/main` rather than the full
/// `refs/remotes/origin/HEAD`), which `git diff` accepts directly.
fn default_branch(cwd: &Utf8Path) -> String {
    let Ok(output) = Command::new("git")
        .arg("symbolic-ref")
        .arg("--short")
        .arg("refs/remotes/origin/HEAD")
        .current_dir(cwd.as_std_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return DEFAULT_BRANCH_FALLBACK.to_owned();
    };
    if !output.status.success() {
        return DEFAULT_BRANCH_FALLBACK.to_owned();
    }
    let Ok(text) = std::str::from_utf8(&output.stdout) else {
        return DEFAULT_BRANCH_FALLBACK.to_owned();
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return DEFAULT_BRANCH_FALLBACK.to_owned();
    }
    trimmed.to_owned()
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_BRANCH_FALLBACK;
    use super::repo_sha;
    use super::suggested_diff_command;
    use camino::Utf8PathBuf;

    #[test]
    fn outside_a_repo_returns_empty_string() {
        let tmp = tempfile::tempdir().expect("tempdir creation should succeed");
        let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .expect("tempdir path should be UTF-8");
        assert_eq!(repo_sha(&path), "");
    }

    #[test]
    fn suggested_diff_command_outside_a_repo_falls_back_to_main_baseline() {
        // No git repo here, so the default-branch probe fails and the
        // command degrades to the `main` baseline — still a runnable
        // `git diff main...HEAD`, never an error or empty field.
        let tmp = tempfile::tempdir().expect("tempdir creation should succeed");
        let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .expect("tempdir path should be UTF-8");
        assert_eq!(
            suggested_diff_command(&path),
            format!("git diff {DEFAULT_BRANCH_FALLBACK}...HEAD"),
        );
    }
}
