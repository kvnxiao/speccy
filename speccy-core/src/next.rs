//! `speccy next` priority logic.
//!
//! Pure function over [`Workspace`] that decides the next actionable
//! task across the workspace. The CLI binary wraps this with text and
//! JSON renderers; the core module is renderer-free so tests can pin
//! priority without touching stdout.
//!
//! Priority rules (see `.speccy/specs/0007-next-command/SPEC.md`
//! REQ-001..REQ-003):
//!
//! - Walk specs in ascending spec-ID order (`workspace::scan` already sorts).
//! - With no [`KindFilter`], prefer `[?]` review-ready tasks over `[ ]` open
//!   tasks **within a spec**; `[~]` claimed tasks are skipped.
//! - `KindFilter::Implement` returns only `[ ]` tasks across all specs; no
//!   fallback.
//! - `KindFilter::Review` returns only `[?]` tasks across all specs; no
//!   fallback.
//! - When no task matches and every task is `[x]`, fall through to
//!   [`NextResult::Report`] for the lowest-ID spec missing `REPORT.md`.
//! - When still no match, return [`NextResult::Blocked`] with a canonical
//!   reason string.

use crate::lint::ParsedSpec;
use crate::parse::TaskState;
use crate::personas;
use crate::workspace::Workspace;

/// `--kind` argument variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    /// Filter to `[ ]` open tasks only.
    Implement,
    /// Filter to `[?]` awaiting-review tasks only.
    Review,
}

/// The four `kind` variants returned by [`compute`].
///
/// Field shapes mirror the JSON contract documented in
/// `.speccy/specs/0007-next-command/SPEC.md` REQ-004.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextResult {
    /// One open task is ready to implement.
    Implement {
        /// Spec containing the task (`SPEC-NNNN`).
        spec: String,
        /// Task identifier (`T-NNN`).
        task: String,
        /// Task title as parsed from TASKS.md (no leading checkbox or ID).
        task_line: String,
        /// `Covers:` references attached to the task entry.
        covers: Vec<String>,
        /// `Suggested files:` references attached to the task entry.
        suggested_files: Vec<String>,
    },
    /// One task is awaiting review; the four-persona fan-out applies.
    Review {
        /// Spec containing the task (`SPEC-NNNN`).
        spec: String,
        /// Task identifier (`T-NNN`).
        task: String,
        /// Task title as parsed from TASKS.md.
        task_line: String,
        /// Default reviewer fan-out (hardcoded `&personas::ALL[..4]`).
        personas: &'static [&'static str],
    },
    /// Every task is done; this spec has no REPORT.md yet.
    Report {
        /// Spec needing a REPORT.md (`SPEC-NNNN`).
        spec: String,
    },
    /// No actionable work for the requested filter.
    Blocked {
        /// Canonical reason string. See [`BlockedReason`] for the closed
        /// set tests pin against.
        reason: String,
    },
}

/// Canonical phrases used in [`NextResult::Blocked::reason`].
///
/// Defined as `&'static str` constants (not an enum exposed publicly)
/// so [`NextResult`] stays a stable, easy-to-serialise shape. Tests
/// reference these so future renames stay localised.
pub struct BlockedReason;

impl BlockedReason {
    /// `.speccy/specs/` is empty or unreadable.
    pub const NO_SPECS: &'static str = "no specs in workspace";
    /// Every open task is held by another session.
    pub const ALL_CLAIMED: &'static str = "all open tasks are claimed by other sessions";
    /// `--kind implement` ran against a workspace with no `[ ]` tasks.
    pub const NO_OPEN_TASKS: &'static str = "no open tasks; reviews pending";
    /// `--kind review` ran against a workspace with no `[?]` tasks.
    pub const NO_REVIEWS_PENDING: &'static str = "no reviews pending";
    /// Catch-all: no tasks exist at all (e.g. specs exist but TASKS.md
    /// is absent or empty across the board).
    pub const NO_TASKS: &'static str = "no tasks in workspace";
    /// Every task is done AND every REPORT.md is present.
    pub const ALL_DONE: &'static str = "all specs reported";
}

/// Compute the next actionable task.
///
/// The function is read-only: it does not mutate `workspace`, does not
/// touch the filesystem, and does not log. Output ordering is fully
/// determined by `workspace.specs` order, which `workspace::scan`
/// already sorts.
#[must_use = "the result names the next action for the harness or user"]
pub fn compute(workspace: &Workspace, kind_filter: Option<KindFilter>) -> NextResult {
    if workspace.specs.is_empty() {
        return blocked(BlockedReason::NO_SPECS);
    }

    if let Some(result) = pick_actionable(workspace, kind_filter) {
        return result;
    }

    if let Some(result) = detect_report(workspace) {
        return result;
    }

    blocked(blocked_reason_for(workspace, kind_filter))
}

fn pick_actionable(workspace: &Workspace, kind_filter: Option<KindFilter>) -> Option<NextResult> {
    for spec in &workspace.specs {
        let Some(spec_id) = spec.spec_id.as_deref() else {
            continue;
        };
        let Some(tasks) = spec.tasks_md_ok() else {
            continue;
        };
        match kind_filter {
            None => {
                if let Some(task) = first_task_with_state(&tasks.tasks, TaskState::InReview) {
                    return Some(make_review(spec_id, task));
                }
                if let Some(task) = first_task_with_state(&tasks.tasks, TaskState::Pending) {
                    return Some(make_implement(spec_id, task));
                }
            }
            Some(KindFilter::Implement) => {
                if let Some(task) = first_task_with_state(&tasks.tasks, TaskState::Pending) {
                    return Some(make_implement(spec_id, task));
                }
            }
            Some(KindFilter::Review) => {
                if let Some(task) = first_task_with_state(&tasks.tasks, TaskState::InReview) {
                    return Some(make_review(spec_id, task));
                }
            }
        }
    }
    None
}

fn detect_report(workspace: &Workspace) -> Option<NextResult> {
    let mut saw_task = false;
    for spec in &workspace.specs {
        let Some(tasks) = spec.tasks_md_ok() else {
            continue;
        };
        if tasks.tasks.is_empty() {
            continue;
        }
        saw_task = true;
        if tasks.tasks.iter().any(|t| t.state != TaskState::Completed) {
            return None;
        }
    }
    if !saw_task {
        return None;
    }

    for spec in &workspace.specs {
        let Some(spec_id) = spec.spec_id.as_deref() else {
            continue;
        };
        let Some(tasks) = spec.tasks_md_ok() else {
            continue;
        };
        if tasks.tasks.is_empty() {
            continue;
        }
        if !report_md_exists(spec) {
            return Some(NextResult::Report {
                spec: spec_id.to_owned(),
            });
        }
    }
    None
}

fn report_md_exists(spec: &ParsedSpec) -> bool {
    let path = spec.dir.join("REPORT.md");
    fs_err::metadata(path.as_std_path()).is_ok_and(|m| m.is_file())
}

fn blocked_reason_for(workspace: &Workspace, kind_filter: Option<KindFilter>) -> &'static str {
    let mut any_open = false;
    let mut any_review = false;
    let mut any_in_progress = false;
    let mut any_task = false;
    let mut any_unreported_done = false;

    for spec in &workspace.specs {
        let Some(tasks) = spec.tasks_md_ok() else {
            continue;
        };
        for task in &tasks.tasks {
            any_task = true;
            match task.state {
                TaskState::Pending => any_open = true,
                TaskState::InReview => any_review = true,
                TaskState::InProgress => any_in_progress = true,
                TaskState::Completed => {
                    if !report_md_exists(spec) {
                        any_unreported_done = true;
                    }
                }
            }
        }
    }

    match kind_filter {
        Some(KindFilter::Implement) => {
            if any_in_progress && !any_open {
                BlockedReason::ALL_CLAIMED
            } else if any_review && !any_open {
                BlockedReason::NO_OPEN_TASKS
            } else if !any_task {
                BlockedReason::NO_TASKS
            } else {
                BlockedReason::NO_OPEN_TASKS
            }
        }
        Some(KindFilter::Review) => {
            if any_task {
                BlockedReason::NO_REVIEWS_PENDING
            } else {
                BlockedReason::NO_TASKS
            }
        }
        None => {
            if any_in_progress && !any_open && !any_review {
                BlockedReason::ALL_CLAIMED
            } else if any_unreported_done {
                // Should have been caught by detect_report; defensive.
                BlockedReason::NO_TASKS
            } else if !any_task {
                BlockedReason::NO_TASKS
            } else {
                BlockedReason::ALL_DONE
            }
        }
    }
}

fn blocked(reason: &str) -> NextResult {
    NextResult::Blocked {
        reason: reason.to_owned(),
    }
}

fn first_task_with_state(
    tasks: &[crate::parse::Task],
    state: TaskState,
) -> Option<&crate::parse::Task> {
    tasks.iter().find(|t| t.state == state)
}

fn make_implement(spec_id: &str, task: &crate::parse::Task) -> NextResult {
    NextResult::Implement {
        spec: spec_id.to_owned(),
        task: task.id.clone(),
        task_line: task.title(),
        covers: task.covers.clone(),
        suggested_files: task.suggested_files(),
    }
}

fn make_review(spec_id: &str, task: &crate::parse::Task) -> NextResult {
    NextResult::Review {
        spec: spec_id.to_owned(),
        task: task.id.clone(),
        task_line: task.title(),
        personas: default_personas(),
    }
}

/// The hardcoded review fan-out for `--kind review` results.
///
/// Sourced from [`crate::personas::ALL`]; the four-persona prefix is
/// the SPEC-0007 DEC-002 default. Exposed as a function (not a `const`)
/// so the slice keeps a `'static` lifetime borrowed from `ALL` without
/// duplicating the literal.
#[must_use = "the returned slice is the fan-out for review results"]
pub fn default_personas() -> &'static [&'static str] {
    let all = personas::ALL;
    all.get(..4).unwrap_or(all)
}

#[cfg(test)]
mod tests {
    use super::BlockedReason;
    use super::default_personas;

    #[test]
    fn default_personas_is_the_first_four_of_all() {
        assert_eq!(
            default_personas(),
            &["business", "tests", "security", "style"],
        );
    }

    #[test]
    fn blocked_reason_constants_are_unique() {
        let phrases = [
            BlockedReason::NO_SPECS,
            BlockedReason::ALL_CLAIMED,
            BlockedReason::NO_OPEN_TASKS,
            BlockedReason::NO_REVIEWS_PENDING,
            BlockedReason::NO_TASKS,
            BlockedReason::ALL_DONE,
        ];
        let mut copy: Vec<&str> = phrases.to_vec();
        copy.sort_unstable();
        copy.dedup();
        assert_eq!(
            copy.len(),
            phrases.len(),
            "BlockedReason constants must be unique strings",
        );
    }
}
