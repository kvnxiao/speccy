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
//! Priority rule:
//! > 1. TASKS.md is absent → kind = `"decompose"`
//! > 2. Any task is `state="in-review"` → kind = `"review"` (first one)
//! > 3. Any task is `state="pending"` → kind = `"work"` (first one)
//! > 4. All tasks `state="completed"`, REPORT.md present → spec is
//! > omitted (terminal — REPORT.md is the durable shipped marker)
//! > 5. All tasks `state="completed"`, REPORT.md absent, gate-pass
//! > artifact (VET.md) present and fresh → kind = `"ship"`
//! > 6. Else (all tasks completed, REPORT.md absent, gate-pass
//! > artifact missing or stale) → kind = `"vet"`
//!
//! "Gate-pass artifact fresh" means: `<spec-dir>/journal/VET.md` exists,
//! its final non-whitespace `<gate ...>` block has `verdict="passed"`,
//! and its `tasks_hash="X"` attribute equals the lowercase hex SHA-256
//! of the current `<spec-dir>/TASKS.md` byte contents.

use crate::lint::ParsedSpec;
use crate::parse::TaskState;
use crate::parse::VetBlock;
use crate::parse::parse_vet_in_flight;
use crate::personas;
use crate::workspace::Workspace;
use sha2::Digest as _;
use sha2::Sha256;

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
    Work {
        /// Task identifier (`T-NNN`) of the first pending task.
        task_id: String,
    },
    /// All tasks are completed but the pre-ship vet gate has not yet
    /// produced a fresh passing artifact (`<spec-dir>/journal/VET.md`
    /// is absent, ends with a failing `<gate>` block, or ends with a
    /// passing `<gate>` block whose `tasks_hash` does not match the
    /// current TASKS.md SHA-256).
    Vet,
    /// All tasks are completed, a fresh passing vet-gate artifact
    /// exists, and REPORT.md is absent.
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
/// 3. Any task `state="pending"` → `Work` (first matching task)
/// 4. All tasks `state="completed"`, REPORT.md present → `None` (omit;
///    REPORT.md is the durable shipped marker, terminal regardless of vet-gate
///    artifact state)
/// 5. All tasks `state="completed"`, REPORT.md absent, gate-pass artifact
///    present and fresh → `Ship`
/// 6. All tasks `state="completed"`, REPORT.md absent, gate-pass artifact
///    missing or stale → `Vet`
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
        return Some(NextAction::Work {
            task_id: task.id.clone(),
        });
    }

    // All tasks are completed (or empty). REPORT.md presence is the
    // durable shipped marker and beats vet-gate state: once REPORT.md
    // exists, the spec is terminal regardless
    // of whether a fresh `journal/VET.md` gate is present.
    if tasks.tasks.iter().all(|t| t.state == TaskState::Completed) {
        if report_md_exists(spec) {
            // Fully done — omit from workspace listing.
            return None;
        }
        if vet_gate_is_fresh_pass(spec) {
            return Some(NextAction::Ship);
        }
        return Some(NextAction::Vet);
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

/// Returns true when `<spec-dir>/journal/VET.md` exists, its terminal
/// `<gate>` block (the last block of the last invocation section, per
/// the typed [`VetDoc`](crate::parse::VetDoc)) has `verdict="passed"`,
/// and its `tasks_hash` attribute equals the lowercase hex SHA-256 of
/// the current `<spec-dir>/TASKS.md` byte contents.
///
/// Returns false on any other shape (file absent, parse failure, empty
/// document, a terminal block that is not a `Gate`, a failed verdict, a
/// stale hash, or a read error). Recognition flows entirely through
/// [`parse_vet_in_flight`]: a `<gate>` quoted inside
/// a block body is captured in that block's body text and never surfaces
/// as the terminal gate, so it cannot satisfy freshness. Treating parse
/// failures as "not fresh" is deliberate: re-vetting is safer than
/// shipping on a malformed artifact.
fn vet_gate_is_fresh_pass(spec: &ParsedSpec) -> bool {
    let vet_path = spec.dir.join("journal").join("VET.md");
    let Ok(vet_bytes) = fs_err::read(vet_path.as_std_path()) else {
        return false;
    };
    let Ok(vet_text) = std::str::from_utf8(&vet_bytes) else {
        return false;
    };
    let Ok(doc) = parse_vet_in_flight(vet_text, &vet_path) else {
        return false;
    };
    let Some(VetBlock::Gate {
        verdict,
        tasks_hash,
        ..
    }) = doc.invocations.last().and_then(|inv| inv.blocks.last())
    else {
        return false;
    };
    if verdict != "passed" {
        return false;
    }
    let tasks_path = spec.dir.join("TASKS.md");
    let Ok(tasks_bytes) = fs_err::read(tasks_path.as_std_path()) else {
        return false;
    };
    let actual_hash = const_hex::encode(Sha256::digest(&tasks_bytes));
    tasks_hash.eq_ignore_ascii_case(&actual_hash)
}

fn first_task_with_state(
    tasks: &[crate::parse::Task],
    state: TaskState,
) -> Option<&crate::parse::Task> {
    tasks.iter().find(|t| t.state == state)
}

/// The hardcoded review fan-out.
///
/// Sourced from [`crate::personas::ALL`]; the five-persona prefix is
/// the default. Exposed as a function (not a `const`)
/// so the slice keeps a `'static` lifetime borrowed from `ALL` without
/// duplicating the literal.
#[must_use = "the returned slice is the fan-out for review results"]
pub fn default_personas() -> &'static [&'static str] {
    let all = personas::ALL;
    all.get(..5).unwrap_or(all)
}

#[cfg(test)]
mod tests {
    use super::default_personas;

    #[test]
    fn default_personas_is_the_first_five_of_all() {
        assert_eq!(
            default_personas(),
            &["business", "tests", "security", "style", "correctness"],
        );
    }
}
