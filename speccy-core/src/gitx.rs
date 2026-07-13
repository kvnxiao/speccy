//! Git operations, shelling out to the `git` CLI (IMPLEMENTATION-PLAN § Build
//! choices: no `gix`/`libgit2`).
//!
//! Snapshots and `baseline_commit` require git; resume and evidence baselines
//! depend on it (DESIGN § Run Branch and Snapshot Policy).

use crate::error::Result;
use crate::error::SpeccyError;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use std::fmt::Write as _;
use std::process::Command;

/// Controller commit identity (DESIGN § Run Branch and Snapshot Policy).
pub const COMMITTER_NAME: &str = "Speccy";
/// Controller commit email, paired with `COMMITTER_NAME`.
pub const COMMITTER_EMAIL: &str = "noreply@speccy.local";

/// A `git diff --numstat` rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffStat {
    pub files: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Run a git command in `dir`, returning trimmed stdout or a structured error.
fn git(dir: &Utf8Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| SpeccyError::io(format!("failed to run git: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpeccyError::io(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The installed git version string (`git --version`), if git is available.
#[must_use = "the detected git version should be inspected, not discarded"]
pub fn version() -> Option<String> {
    let out = Command::new("git").arg("--version").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The git repository top level, or `not_a_git_repo`.
///
/// # Errors
///
/// Returns an error if `git` cannot be run, or if `dir` is not inside a git
/// repository.
pub fn toplevel(dir: &Utf8Path) -> Result<Utf8PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .map_err(|e| SpeccyError::io(format!("failed to run git: {e}")))?;
    if !out.status.success() {
        return Err(SpeccyError::not_a_git_repo(format!(
            "{dir} is not inside a git repository"
        )));
    }
    Ok(Utf8PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim(),
    ))
}

/// The current HEAD commit (full SHA).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the command fails.
pub fn head(dir: &Utf8Path) -> Result<String> {
    git(dir, &["rev-parse", "HEAD"])
}

/// The current branch name.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the command fails.
pub fn current_branch(dir: &Utf8Path) -> Result<String> {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// True when the worktree has staged or unstaged changes (untracked included).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the command fails.
pub fn is_dirty(dir: &Utf8Path) -> Result<bool> {
    Ok(!dirty_files(dir)?.is_empty())
}

/// The list of dirty paths, one per file (`--untracked-files=all` so a new
/// subdirectory lists its files rather than collapsing to the directory).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the command fails.
pub fn dirty_files(dir: &Utf8Path) -> Result<Vec<String>> {
    let out = git(dir, &["status", "--porcelain", "--untracked-files=all"])?;
    Ok(out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.get(3..).unwrap_or(l).trim().to_string())
        .collect())
}

/// Create and check out a new branch from the current HEAD.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the checkout fails.
pub fn create_branch(dir: &Utf8Path, name: &str) -> Result<()> {
    git(dir, &["checkout", "-b", name])?;
    Ok(())
}

/// Check out an existing branch.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the checkout fails.
pub fn checkout(dir: &Utf8Path, name: &str) -> Result<()> {
    git(dir, &["checkout", name])?;
    Ok(())
}

/// True if a local branch exists.
///
/// # Errors
///
/// Returns an error if `git` cannot be run.
pub fn branch_exists(dir: &Utf8Path, name: &str) -> Result<bool> {
    let out = Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ])
        .current_dir(dir)
        .output()
        .map_err(|e| SpeccyError::io(format!("failed to run git: {e}")))?;
    Ok(out.status.success())
}

/// Create a detached worktree at `path` checked out at `commit` (evidence
/// control baselines; DESIGN § Acceptance Ledger).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the worktree cannot be created.
pub fn worktree_add(dir: &Utf8Path, path: &Utf8Path, commit: &str) -> Result<()> {
    git(dir, &["worktree", "add", "--detach", path.as_str(), commit])?;
    Ok(())
}

/// Remove the worktree registered at `path`, discarding its state.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the removal fails.
pub fn worktree_remove(dir: &Utf8Path, path: &Utf8Path) -> Result<()> {
    git(dir, &["worktree", "remove", "--force", path.as_str()])?;
    Ok(())
}

/// Drop worktree registrations whose directories no longer exist.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the prune fails.
pub fn worktree_prune(dir: &Utf8Path) -> Result<()> {
    git(dir, &["worktree", "prune"])?;
    Ok(())
}

/// Stage everything and commit under the controller identity. Returns the new
/// commit SHA. Never squashes (DESIGN § Run Branch and Snapshot Policy).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or any staging/commit step fails.
pub fn commit_all(dir: &Utf8Path, message: &str) -> Result<String> {
    git(dir, &["add", "-A"])?;
    let author = format!("{COMMITTER_NAME} <{COMMITTER_EMAIL}>");
    git(
        dir,
        &[
            "-c",
            &format!("user.name={COMMITTER_NAME}"),
            "-c",
            &format!("user.email={COMMITTER_EMAIL}"),
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--no-verify",
            "--allow-empty",
            "--author",
            &author,
            "-m",
            message,
        ],
    )?;
    head(dir)
}

/// `git diff --numstat <base>` rollup against the working tree.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the diff fails.
pub fn diff_stat(dir: &Utf8Path, base: &str) -> Result<DiffStat> {
    let out = git(dir, &["diff", "--numstat", base])?;
    Ok(parse_numstat(&out))
}

/// Changed file paths against `base` (working tree included).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the diff fails.
pub fn diff_files(dir: &Utf8Path, base: &str) -> Result<Vec<String>> {
    let out = git(dir, &["diff", "--name-only", base])?;
    Ok(out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect())
}

/// Full unified diff text against `base` (working tree included).
///
/// The default `a/`/`b/` path prefixes are forced so the header shape holds
/// even under a user's `diff.noprefix`/`diff.mnemonicPrefix` git config: the
/// provenance scanner (`provenance::scan_diff`) relies on that shape to tell a
/// real file header from a content line beginning with `-- `/`++ `.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the diff fails.
pub fn diff_text(dir: &Utf8Path, base: &str) -> Result<String> {
    git(dir, &["diff", "--src-prefix=a/", "--dst-prefix=b/", base])
}

/// Full unified diff text between two commits (worktree excluded), with the
/// same forced `a/`/`b/` prefixes as `diff_text`.
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the diff fails.
pub fn range_diff_text(dir: &Utf8Path, base: &str, head: &str) -> Result<String> {
    git(
        dir,
        &[
            "diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            &format!("{base}..{head}"),
        ],
    )
}

/// Untracked file paths (respecting `.gitignore`).
///
/// # Errors
///
/// Returns an error if `git` is unavailable or the command fails.
pub fn untracked_files(dir: &Utf8Path) -> Result<Vec<String>> {
    let out = git(dir, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect())
}

/// A unified diff against `base` that *also* includes untracked new files as
/// additions. `git diff` alone omits untracked files, but a worker's new files
/// are part of its change and must be scanned and attributed. Fail-closed: an
/// unreadable untracked file is an error, never silently-empty content —
/// provenance scanning and evidence identity both hash this diff (binary
/// content is included lossily rather than dropped).
///
/// # Errors
///
/// Returns an error if `git` is unavailable, any underlying diff fails, or an
/// untracked file cannot be read.
pub fn worktree_diff(dir: &Utf8Path, base: &str) -> Result<String> {
    let mut out = diff_text(dir, base)?;
    for file in untracked_files(dir)? {
        let bytes = fs_err::read(dir.join(&file))?;
        let content = String::from_utf8_lossy(&bytes);
        let count = content.lines().count();
        // Writing to a `String` is infallible.
        _ = write!(
            out,
            "\n--- /dev/null\n+++ b/{file}\n@@ -0,0 +1,{count} @@\n"
        );
        for line in content.lines() {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// Diff stat against `base` including untracked new files (see
/// `worktree_diff`).
///
/// # Errors
///
/// Returns an error if `git` is unavailable, the diff fails, or an untracked
/// file cannot be read.
pub fn worktree_stat(dir: &Utf8Path, base: &str) -> Result<DiffStat> {
    let mut stat = diff_stat(dir, base)?;
    for file in untracked_files(dir)? {
        let bytes = fs_err::read(dir.join(&file))?;
        stat.files += 1;
        stat.insertions += String::from_utf8_lossy(&bytes).lines().count();
    }
    Ok(stat)
}

fn parse_numstat(out: &str) -> DiffStat {
    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;
    for line in out.lines() {
        let mut cols = line.split('\t');
        let add = cols.next().unwrap_or("0");
        let del = cols.next().unwrap_or("0");
        if cols.next().is_none() {
            continue;
        }
        files += 1;
        // Binary files report "-"; count them as touched, zero lines.
        insertions += add.parse::<usize>().unwrap_or(0);
        deletions += del.parse::<usize>().unwrap_or(0);
    }
    DiffStat {
        files,
        insertions,
        deletions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_parses() {
        let s = "3\t1\tsrc/a.rs\n10\t0\tsrc/b.rs\n-\t-\tbin.png\n";
        let d = parse_numstat(s);
        assert_eq!(
            d,
            DiffStat {
                files: 3,
                insertions: 13,
                deletions: 1
            }
        );
    }
}
