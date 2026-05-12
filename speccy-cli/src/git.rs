//! Best-effort `git rev-parse HEAD` shell-out.
//!
//! Returns the empty string when git is unavailable, the directory is
//! not a git repo, or HEAD is unset. Per SPEC-0004 DEC-003 this is a
//! non-fatal lookup: callers always treat the result as informational.

use camino::Utf8Path;
use std::process::Command;
use std::process::Stdio;

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
