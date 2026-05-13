//! Best-effort git shell-outs.
//!
//! Two helpers:
//!
//! - [`repo_sha`] — returns `HEAD`'s SHA for the JSON status contract.
//! - [`diff_for_review`] — computes the reviewer-prompt diff with a three-step
//!   fallback chain (working tree vs HEAD; HEAD vs HEAD~1; literal "no diff"
//!   note).
//!
//! Both treat git unavailability as a non-fatal lookup: shell-out
//! failures degrade to documented fallbacks rather than propagating an
//! error.

use camino::Utf8Path;
use std::process::Command;
use std::process::Stdio;

/// Literal placeholder inlined into the reviewer prompt when neither
/// fallback produced any diff content.
///
/// Documented in SPEC-0009 REQ-004; reviewer prompts treat this string
/// as a signal to fall back on SPEC.md and prior task notes alone.
pub const NO_DIFF_FALLBACK: &str =
    "<!-- no diff available; review based on SPEC.md and task notes alone -->";

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

/// Compute the diff to inline into the reviewer prompt.
///
/// Fallback chain per SPEC-0009 REQ-004:
/// 1. `git diff HEAD` — captures uncommitted edits.
/// 2. If step 1 succeeds but is empty (clean working tree), try `git diff
///    HEAD~1 HEAD` — captures the most recent commit.
/// 3. If step 2 fails or is empty, return [`NO_DIFF_FALLBACK`].
///
/// Any spawn failure or non-success exit at any step short-circuits to
/// the fallback note; the diff is informational and reviewers operate
/// from SPEC.md and task notes when it is absent.
#[must_use = "the rendered diff is the function's output"]
pub fn diff_for_review(cwd: &Utf8Path) -> String {
    if let Some(text) = run_diff(cwd, &["diff", "HEAD"]).filter(|s| !s.is_empty()) {
        return text;
    }
    if let Some(text) = run_diff(cwd, &["diff", "HEAD~1", "HEAD"]).filter(|s| !s.is_empty()) {
        return text;
    }
    NO_DIFF_FALLBACK.to_owned()
}

fn run_diff(cwd: &Utf8Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd.as_std_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&output.stdout).ok()?;
    Some(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::repo_sha;
    use camino::Utf8PathBuf;

    #[test]
    fn outside_a_repo_returns_empty_string() {
        let tmp = tempfile::tempdir().expect("tempdir creation should succeed");
        let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .expect("tempdir path should be UTF-8");
        assert_eq!(repo_sha(&path), "");
    }
}
