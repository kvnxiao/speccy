//! `speccy next` priority logic.
//!
//! Provides two compute functions:
//!
//! - [`compute_for_spec`]: derives the [`NextAction`] for a single spec from
//!   its on-disk artifact state, without any user-supplied filter. Used by
//!   `speccy next SPEC-NNNN`.
//! - [`compute_workspace`]: walks every spec in the workspace and returns a
//!   list of [`SpecNextEntry`] values for the active ones (omitting specs that
//!   are fully completed and have REPORT.md). Used by `speccy next` (workspace
//!   form).
//!
//! Priority rule (see SPEC-0033 REQ-004):
//! > if TASKS.md is absent → kind = `"decompose"`
//! > else if any task is `state="in-review"` → kind = `"review"` (first one)
//! > else if any task is `state="pending"` → kind = `"implement"` (first one)
//! > else if all tasks `state="completed"` and REPORT.md absent → kind =
//! > `"ship"`
//! > else → spec is omitted (all done + REPORT.md present)

use crate::lint::ParsedSpec;
use crate::parse::TaskState;
use crate::personas;
use crate::workspace::Workspace;

/// The derived action kind for a single spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextAction {
    /// TASKS.md is absent; the next action is to decompose the spec.
    Decompose,
    /// A task is awaiting review.
    Review {
        /// Task identifier (`T-NNN`) of the first in-review task.
        task_id: String,
        /// Default reviewer fan-out.
        personas: &'static [&'static str],
    },
    /// A task is ready to implement.
    Implement {
        /// Task identifier (`T-NNN`) of the first pending task.
        task_id: String,
    },
    /// All tasks are completed and REPORT.md is absent.
    Ship,
}

/// A per-spec entry returned by [`compute_workspace`].
#[derive(Debug, Clone)]
pub struct SpecNextEntry {
    /// Spec identifier (`SPEC-NNNN`).
    pub spec_id: String,
    /// Derived next action.
    pub action: NextAction,
}

/// Derive the next action for a single spec.
///
/// Returns `None` when the spec is fully done (all tasks completed and
/// REPORT.md present), meaning it should be omitted from workspace listings.
///
/// # Priority
///
/// 1. TASKS.md absent → `Decompose`
/// 2. Any task `state="in-review"` → `Review` (first matching task)
/// 3. Any task `state="pending"` → `Implement` (first matching task)
/// 4. All tasks `state="completed"` and REPORT.md absent → `Ship`
/// 5. All done + REPORT.md present → `None` (omit)
#[must_use = "the derived action names the next step for the spec"]
pub fn compute_for_spec(spec: &ParsedSpec) -> Option<NextAction> {
    let Some(tasks) = spec.tasks_md_ok() else {
        // TASKS.md absent or unparseable → decompose.
        return Some(NextAction::Decompose);
    };

    if let Some(task) = first_task_with_state(&tasks.tasks, TaskState::InReview) {
        return Some(NextAction::Review {
            task_id: task.id.clone(),
            personas: default_personas(),
        });
    }

    if let Some(task) = first_task_with_state(&tasks.tasks, TaskState::Pending) {
        return Some(NextAction::Implement {
            task_id: task.id.clone(),
        });
    }

    // All tasks are completed (or empty). Check REPORT.md.
    if tasks.tasks.iter().all(|t| t.state == TaskState::Completed) {
        if report_md_exists(spec) {
            // Fully done — omit from workspace listing.
            return None;
        }
        return Some(NextAction::Ship);
    }

    // Only in-progress tasks remain (all "claimed"). Treat as decompose to
    // avoid producing a confusing null for a spec with tasks.
    Some(NextAction::Decompose)
}

/// Walk every spec in the workspace and return entries for active specs.
///
/// A spec is active when [`compute_for_spec`] returns `Some(_)`. Specs
/// where all tasks are completed and REPORT.md is present return `None`
/// and are omitted from the result.
///
/// The returned slice is ordered by ascending spec ID (matching
/// [`workspace::scan`] sort order).
#[must_use = "the entries describe the workspace state for all active specs"]
pub fn compute_workspace(workspace: &Workspace) -> Vec<SpecNextEntry> {
    let mut entries = Vec::new();
    for spec in &workspace.specs {
        let Some(spec_id) = spec.spec_id.as_deref() else {
            continue;
        };
        if let Some(action) = compute_for_spec(spec) {
            entries.push(SpecNextEntry {
                spec_id: spec_id.to_owned(),
                action,
            });
        }
    }
    entries
}

fn report_md_exists(spec: &ParsedSpec) -> bool {
    let path = spec.dir.join("REPORT.md");
    fs_err::metadata(path.as_std_path()).is_ok_and(|m| m.is_file())
}

fn first_task_with_state(
    tasks: &[crate::parse::Task],
    state: TaskState,
) -> Option<&crate::parse::Task> {
    tasks.iter().find(|t| t.state == state)
}

/// The hardcoded review fan-out.
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
    use super::default_personas;

    #[test]
    fn default_personas_is_the_first_four_of_all() {
        assert_eq!(
            default_personas(),
            &["business", "tests", "security", "style"],
        );
    }
}
