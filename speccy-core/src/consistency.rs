//! Drift detection for the `speccy next` JSON envelope (SPEC-0045
//! REQ-005 / REQ-006).
//!
//! Given a parsed spec, this module compares TASKS.md task state against
//! the git log of commits whose title begins with `[SPEC-NNNN/T-NNN]: `
//! plus, for `journal_xml_malformed`, against the on-disk per-task
//! journal file. The CLI calls [`detect`] and serialises the returned
//! [`ConsistencyBlock`] into its JSON output.
//!
//! The detection is read-only by design (DEC-001). Callers supply a
//! [`GitProbe`] trait object that walks `git log` and `git status`; the
//! shipped [`ShellGitProbe`] shells out to the `git` binary in
//! read-only mode (`git log --grep`, `git status --porcelain`). No
//! variant of this code path invokes `git add`, `git commit`,
//! `git restore`, `git clean`, or `git stash`.

use crate::lint::ParsedSpec;
use crate::parse::TaskState;
use crate::parse::journal_xml;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use serde::Serialize;
use std::process::Command;
use std::process::Stdio;

/// Top-level consistency status emitted in the `speccy next` envelope.
///
/// Values map verbatim to the strings `"ok"`, `"drift"`, and `"blocked"`
/// in the JSON output (kebab-case via the `serde(rename_all = ...)`
/// directive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsistencyStatus {
    /// No drift detected; `next_action` reflects normal dispatch.
    Ok,
    /// At least one `auto_fixable` drift; no `blocking` drifts.
    Drift,
    /// At least one `blocking` drift.
    Blocked,
}

/// Closed enum of drift kinds (SPEC-0045 REQ-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftKind {
    /// A `[SPEC-NNNN/T-NNN]:`-prefixed commit exists but the task is in
    /// a non-`completed` state.
    CommitWithoutState,
    /// The task is `completed` in TASKS.md but no matching commit
    /// exists in git log.
    StateCompletedNoCommit,
    /// The task is `in-progress`, no matching commit exists, and the
    /// working tree is dirty (a crashed implementer pass).
    StateInProgressOrphaned,
    /// The task is `in-progress`, no matching commit exists, and the
    /// working tree is clean (a crashed implementer pass whose
    /// partial work was already discarded, or whose changes never
    /// reached disk). The reconcile pass owns this case autonomously
    /// per SPEC-0045 REQ-006/REQ-007 — it must not surface as a
    /// user-facing fork in the orchestrator startup check.
    StateInProgressClean,
    /// The per-task journal file failed to parse against the closed
    /// journal grammar.
    JournalXmlMalformed,
}

/// Severity bucket for one drift entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftSeverity {
    /// Reconcile can resolve this without user input via a deterministic
    /// write (currently: TASKS.md state flip for `commit_without_state`).
    AutoFixable,
    /// Reconcile escalates: the resolution path involves either a real
    /// git mutation or a rollback that loses work.
    Blocking,
}

/// Kind-specific details object emitted under `details` in the JSON.
///
/// The variants are flattened at serialise time so each `kind` carries
/// its documented fields directly under the `details` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum DriftDetails {
    /// `commit_without_state` details.
    CommitWithoutState {
        /// Full 40-character lowercase hex SHA.
        commit_sha: String,
        /// Eight-character abbreviation of `commit_sha`.
        commit_short_sha: String,
    },
    /// `state_completed_no_commit` details.
    StateCompletedNoCommit {
        /// The expected commit-title prefix: `[SPEC-NNNN/T-NNN]:`.
        expected_trailer: String,
        /// `true` iff `git status --porcelain` had non-empty output.
        working_tree_dirty: bool,
    },
    /// `state_in_progress_orphaned` details.
    StateInProgressOrphaned {
        /// Always `true` for this variant — the kind only fires when
        /// the working tree is dirty.
        working_tree_dirty: bool,
        /// Number of entries in `git status --porcelain` output.
        dirty_files_count: usize,
    },
    /// `state_in_progress_clean` details.
    StateInProgressClean {
        /// Always `false` for this variant — the kind only fires
        /// when the working tree is clean.
        working_tree_dirty: bool,
    },
    /// `journal_xml_malformed` details.
    JournalXmlMalformed {
        /// Repo-relative forward-slash path to the malformed journal
        /// file.
        journal_path: String,
        /// Byte offset just after the last well-formed element's close
        /// tag (or `0` when no element parsed cleanly).
        last_well_formed_byte_offset: usize,
    },
}

/// One entry in `consistency.drifts[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftEntry {
    /// Task identifier (`T-NNN`).
    pub task_id: String,
    /// Drift kind enum value.
    pub kind: DriftKind,
    /// Severity bucket.
    pub severity: DriftSeverity,
    /// On-disk TASKS.md state at detection time (lowercase, e.g.
    /// `"completed"`, `"in-review"`).
    pub tasks_state: String,
    /// Kind-specific details object.
    pub details: DriftDetails,
}

/// The top-level `consistency` object the CLI serialises into its JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsistencyBlock {
    /// Aggregate status across `drifts`.
    pub status: ConsistencyStatus,
    /// One entry per detected drift; empty when `status == Ok`.
    pub drifts: Vec<DriftEntry>,
}

impl ConsistencyBlock {
    /// Empty `ok` block.
    #[must_use = "the empty block is the no-drift envelope value"]
    pub fn ok() -> Self {
        Self {
            status: ConsistencyStatus::Ok,
            drifts: Vec::new(),
        }
    }
}

/// Minimal read-only git query surface used by [`detect`].
///
/// Implementors must shell out (or use libgit2) without mutating the
/// repository. The shipped [`ShellGitProbe`] runs `git log --grep` and
/// `git status --porcelain` only.
pub trait GitProbe {
    /// Return `true` when the probe is rooted inside a usable git
    /// repository. The consistency detector skips state-vs-commit
    /// correlation when this returns `false` (there is no source of
    /// truth to compare TASKS.md against). Journal-parse drift is
    /// orthogonal and still detected.
    fn is_git_repo(&self) -> bool;

    /// Return the full 40-character SHA of the first commit (most
    /// recent in `git log` order) whose title begins with `prefix`, or
    /// `None` when no such commit exists or git itself fails.
    fn first_commit_sha_with_title_prefix(&self, prefix: &str) -> Option<String>;

    /// Return the porcelain status output as a vector of lines (each
    /// without trailing newline). An empty vector means a clean tree.
    /// On git failure, returns an empty vector — the caller treats
    /// "git unavailable" as "clean" for the purposes of drift
    /// detection (consistency is best-effort outside a git repo).
    fn porcelain_status(&self) -> Vec<String>;
}

/// Default [`GitProbe`] backed by the `git` binary on `PATH`.
#[derive(Debug, Clone)]
pub struct ShellGitProbe {
    /// Repository root passed to `git -C <root>`.
    root: Utf8PathBuf,
}

impl ShellGitProbe {
    /// Build a probe rooted at `root`.
    #[must_use = "the probe is the input to detect()"]
    pub fn new(root: impl AsRef<Utf8Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl GitProbe for ShellGitProbe {
    fn is_git_repo(&self) -> bool {
        let Ok(output) = Command::new("git")
            .arg("-C")
            .arg(self.root.as_std_path())
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
        else {
            return false;
        };
        output.status.success()
    }

    fn first_commit_sha_with_title_prefix(&self, prefix: &str) -> Option<String> {
        // `git log --grep=<re> --extended-regexp --format=%H` lists
        // every commit whose subject matches. The regex is anchored to
        // the start of the subject; brackets are escaped so the prefix
        // string is matched literally.
        let pattern = format!("^{}", regex_escape(prefix));
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root.as_std_path())
            .arg("log")
            .arg("--all")
            .arg("--extended-regexp")
            .arg(format!("--grep={pattern}"))
            .arg("--format=%H")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = std::str::from_utf8(&output.stdout).ok()?;
        text.lines()
            .next()
            .map(|l| l.trim().to_owned())
            .filter(|s| !s.is_empty())
    }

    fn porcelain_status(&self) -> Vec<String> {
        let Ok(output) = Command::new("git")
            .arg("-C")
            .arg(self.root.as_std_path())
            .arg("status")
            .arg("--porcelain")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        let Ok(text) = std::str::from_utf8(&output.stdout) else {
            return Vec::new();
        };
        text.lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

/// Escape regex metacharacters in `s` for safe embedding in a
/// `git log --grep=<re>` extended-regexp pattern.
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if matches!(
            c,
            '.' | '\\' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Detect consistency drift for one spec.
///
/// `spec_id` is the canonical `SPEC-NNNN` identifier used to build
/// per-task commit-title prefixes. Returns the assembled
/// [`ConsistencyBlock`] (empty + `Ok` when no drift is detected).
///
/// The function is read-only: it parses already-loaded TASKS.md, reads
/// the per-task journal file off disk if present, and queries the
/// supplied [`GitProbe`]. No git mutation is performed at any point.
#[must_use = "the returned block is the consistency envelope value"]
pub fn detect(spec_id: &str, spec: &ParsedSpec, probe: &dyn GitProbe) -> ConsistencyBlock {
    let Some(tasks) = spec.tasks_md_ok() else {
        // No TASKS.md means nothing to correlate; report ok rather than
        // synthesising drift from a parse failure (parse-failure
        // surfacing is the lint engine's job).
        return ConsistencyBlock::ok();
    };

    // Cache porcelain status once; the multiple kinds that consult it
    // share a single shell-out.
    let in_repo = probe.is_git_repo();
    let porcelain = if in_repo {
        probe.porcelain_status()
    } else {
        Vec::new()
    };
    let dirty = !porcelain.is_empty();
    let dirty_count = porcelain.len();

    let mut drifts: Vec<DriftEntry> = Vec::new();
    for task in &tasks.tasks {
        // Outside a git repo, skip state-vs-commit correlation entirely:
        // there is no source of truth to correlate against, so
        // reporting "drift" would be a false positive. Journal-parse
        // drift is orthogonal and still detected.
        if !in_repo {
            if let Some(entry) = detect_journal_drift(spec, &task.id) {
                drifts.push(entry);
            }
            continue;
        }
        let prefix = format!("[{spec_id}/{task_id}]:", task_id = task.id);
        let commit_sha = probe.first_commit_sha_with_title_prefix(&prefix);
        let state = task.state;
        let state_str = state.as_str();

        match (state, commit_sha.as_deref()) {
            (TaskState::Completed, None) => {
                drifts.push(DriftEntry {
                    task_id: task.id.clone(),
                    kind: DriftKind::StateCompletedNoCommit,
                    severity: DriftSeverity::Blocking,
                    tasks_state: state_str.to_owned(),
                    details: DriftDetails::StateCompletedNoCommit {
                        expected_trailer: prefix.clone(),
                        working_tree_dirty: dirty,
                    },
                });
            }
            (TaskState::Completed, Some(_)) => {
                // Healthy: completed task has its commit.
            }
            (_, Some(sha)) => {
                // A commit exists but the task is not yet `completed`.
                let short = sha.get(..8).unwrap_or(sha).to_owned();
                drifts.push(DriftEntry {
                    task_id: task.id.clone(),
                    kind: DriftKind::CommitWithoutState,
                    severity: DriftSeverity::AutoFixable,
                    tasks_state: state_str.to_owned(),
                    details: DriftDetails::CommitWithoutState {
                        commit_sha: sha.to_owned(),
                        commit_short_sha: short,
                    },
                });
            }
            (TaskState::InProgress, None) if dirty => {
                drifts.push(DriftEntry {
                    task_id: task.id.clone(),
                    kind: DriftKind::StateInProgressOrphaned,
                    severity: DriftSeverity::Blocking,
                    tasks_state: state_str.to_owned(),
                    details: DriftDetails::StateInProgressOrphaned {
                        working_tree_dirty: true,
                        dirty_files_count: dirty_count,
                    },
                });
            }
            (TaskState::InProgress, None) => {
                // Clean tree, no matching commit — the reconcile pass
                // owns this autonomously (SPEC-0045 REQ-006). Roll
                // TASKS.md state back to `pending`; no git mutation.
                drifts.push(DriftEntry {
                    task_id: task.id.clone(),
                    kind: DriftKind::StateInProgressClean,
                    severity: DriftSeverity::Blocking,
                    tasks_state: state_str.to_owned(),
                    details: DriftDetails::StateInProgressClean {
                        working_tree_dirty: false,
                    },
                });
            }
            _ => {}
        }

        // Independent of state-vs-commit drift: a malformed per-task
        // journal is always blocking. The path is derived from the
        // spec directory and the task id.
        if let Some(entry) = detect_journal_drift(spec, &task.id) {
            drifts.push(entry);
        }
    }

    let status = if drifts.is_empty() {
        ConsistencyStatus::Ok
    } else if drifts.iter().any(|d| d.severity == DriftSeverity::Blocking) {
        ConsistencyStatus::Blocked
    } else {
        ConsistencyStatus::Drift
    };

    ConsistencyBlock { status, drifts }
}

/// Probe the per-task journal at `<spec-dir>/journal/T-NNN.md` and
/// return a `journal_xml_malformed` entry when parsing fails. Returns
/// `None` when the file is absent or parses cleanly.
fn detect_journal_drift(spec: &ParsedSpec, task_id: &str) -> Option<DriftEntry> {
    let journal_path = spec.dir.join("journal").join(format!("{task_id}.md"));
    let bytes = fs_err::read(journal_path.as_std_path()).ok()?;
    let source = std::str::from_utf8(&bytes).ok()?;
    if journal_xml::parse(source, &journal_path).is_ok() {
        return None;
    }
    let offset = journal_xml::last_well_formed_offset(source, &journal_path);
    let task = task_id.to_owned();
    let state_str = spec
        .tasks_md_ok()
        .and_then(|t| t.tasks.iter().find(|tt| tt.id == task_id))
        .map_or("unknown", |t| t.state.as_str())
        .to_owned();
    Some(DriftEntry {
        task_id: task,
        kind: DriftKind::JournalXmlMalformed,
        severity: DriftSeverity::Blocking,
        tasks_state: state_str,
        details: DriftDetails::JournalXmlMalformed {
            journal_path: journal_path.to_string().replace('\\', "/"),
            last_well_formed_byte_offset: offset,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProbe {
        sha_for_prefix: std::collections::HashMap<String, String>,
        porcelain: Vec<String>,
    }

    impl GitProbe for FakeProbe {
        fn is_git_repo(&self) -> bool {
            true
        }
        fn first_commit_sha_with_title_prefix(&self, prefix: &str) -> Option<String> {
            self.sha_for_prefix.get(prefix).cloned()
        }
        fn porcelain_status(&self) -> Vec<String> {
            self.porcelain.clone()
        }
    }

    #[test]
    fn regex_escape_brackets_and_metacharacters() {
        assert_eq!(regex_escape("[SPEC-0045/T-001]:"), r"\[SPEC-0045/T-001\]:",);
    }

    #[test]
    fn consistency_block_ok_constructor_returns_empty_ok_block() {
        let block = ConsistencyBlock::ok();
        assert_eq!(block.status, ConsistencyStatus::Ok);
        assert!(block.drifts.is_empty());
    }

    #[test]
    fn fake_probe_returns_configured_sha() {
        let mut map = std::collections::HashMap::new();
        map.insert("[SPEC-0001/T-001]:".to_owned(), "a".repeat(40));
        let probe = FakeProbe {
            sha_for_prefix: map,
            porcelain: vec![],
        };
        assert_eq!(
            probe.first_commit_sha_with_title_prefix("[SPEC-0001/T-001]:"),
            Some("a".repeat(40)),
        );
        assert!(
            probe
                .first_commit_sha_with_title_prefix("[SPEC-0001/T-002]:")
                .is_none()
        );
    }
}
