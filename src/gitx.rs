//! Git operations, shelling out to the `git` CLI (IMPLEMENTATION-PLAN § Build
//! choices: no `gix`/`libgit2`).
//!
//! Snapshots and `baseline_commit` require git; resume and evidence baselines
//! depend on it (DESIGN § Run Branch and Snapshot Policy).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Result, SpeccyError};

/// Controller commit identity (DESIGN § Run Branch and Snapshot Policy).
pub const COMMITTER_NAME: &str = "Speccy";
pub const COMMITTER_EMAIL: &str = "noreply@speccy.local";

/// A `git diff --numstat` rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffStat {
    pub files: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Run a git command in `dir`, returning trimmed stdout or a structured error.
fn git(dir: &Path, args: &[&str]) -> Result<String> {
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
pub fn version() -> Option<String> {
    let out = Command::new("git").arg("--version").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The git repository top level, or `not_a_git_repo`.
pub fn toplevel(dir: &Path) -> Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .map_err(|e| SpeccyError::io(format!("failed to run git: {e}")))?;
    if !out.status.success() {
        return Err(SpeccyError::not_a_git_repo(format!(
            "{} is not inside a git repository",
            dir.display()
        )));
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// The current HEAD commit (full SHA).
pub fn head(dir: &Path) -> Result<String> {
    git(dir, &["rev-parse", "HEAD"])
}

/// The current branch name.
pub fn current_branch(dir: &Path) -> Result<String> {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// True when the worktree has staged or unstaged changes (untracked included).
pub fn is_dirty(dir: &Path) -> Result<bool> {
    Ok(!dirty_files(dir)?.is_empty())
}

/// The list of dirty paths, one per file (`--untracked-files=all` so a new
/// subdirectory lists its files rather than collapsing to the directory).
pub fn dirty_files(dir: &Path) -> Result<Vec<String>> {
    let out = git(dir, &["status", "--porcelain", "--untracked-files=all"])?;
    Ok(out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.get(3..).unwrap_or(l).trim().to_string())
        .collect())
}

/// Create and check out a new branch from the current HEAD.
pub fn create_branch(dir: &Path, name: &str) -> Result<()> {
    git(dir, &["checkout", "-b", name])?;
    Ok(())
}

/// Check out an existing branch.
pub fn checkout(dir: &Path, name: &str) -> Result<()> {
    git(dir, &["checkout", name])?;
    Ok(())
}

/// True if a local branch exists.
pub fn branch_exists(dir: &Path, name: &str) -> Result<bool> {
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

/// Stage everything and commit under the controller identity. Returns the new
/// commit SHA. Never squashes (DESIGN § Run Branch and Snapshot Policy).
pub fn commit_all(dir: &Path, message: &str) -> Result<String> {
    git(dir, &["add", "-A"])?;
    let author = format!("{COMMITTER_NAME} <{COMMITTER_EMAIL}>");
    git(
        dir,
        &[
            "-c",
            &format!("user.name={COMMITTER_NAME}"),
            "-c",
            &format!("user.email={COMMITTER_EMAIL}"),
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
pub fn diff_stat(dir: &Path, base: &str) -> Result<DiffStat> {
    let out = git(dir, &["diff", "--numstat", base])?;
    Ok(parse_numstat(&out))
}

/// Changed file paths against `base` (working tree included).
pub fn diff_files(dir: &Path, base: &str) -> Result<Vec<String>> {
    let out = git(dir, &["diff", "--name-only", base])?;
    Ok(out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect())
}

/// Full unified diff text against `base` (working tree included).
pub fn diff_text(dir: &Path, base: &str) -> Result<String> {
    git(dir, &["diff", base])
}

/// Untracked file paths (respecting `.gitignore`).
pub fn untracked_files(dir: &Path) -> Result<Vec<String>> {
    let out = git(dir, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect())
}

/// A unified diff against `base` that *also* includes untracked new files as
/// additions. `git diff` alone omits untracked files, but a worker's new files
/// are part of its change and must be scanned and attributed.
pub fn worktree_diff(dir: &Path, base: &str) -> Result<String> {
    let mut out = diff_text(dir, base)?;
    for file in untracked_files(dir)? {
        let content = std::fs::read_to_string(dir.join(&file)).unwrap_or_default();
        let count = content.lines().count();
        out.push_str(&format!(
            "\n--- /dev/null\n+++ b/{file}\n@@ -0,0 +1,{count} @@\n"
        ));
        for line in content.lines() {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// Diff stat against `base` including untracked new files (see `worktree_diff`).
pub fn worktree_stat(dir: &Path, base: &str) -> Result<DiffStat> {
    let mut stat = diff_stat(dir, base)?;
    for file in untracked_files(dir)? {
        let content = std::fs::read_to_string(dir.join(&file)).unwrap_or_default();
        stat.files += 1;
        stat.insertions += content.lines().count();
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
