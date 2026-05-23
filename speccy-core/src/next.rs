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
//! Priority rule (see SPEC-0033 REQ-004, SPEC-0041 REQ-002, and
//! SPEC-0043 REQ-002):
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
    // durable shipped marker and beats vet-gate state (SPEC-0043
    // REQ-002): once REPORT.md exists, the spec is terminal regardless
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

/// Returns true when `<spec-dir>/journal/VET.md` exists, its final
/// non-whitespace `<gate ...>` element block has `verdict="passed"`,
/// and its `tasks_hash="X"` attribute equals the lowercase hex SHA-256
/// of the current `<spec-dir>/TASKS.md` byte contents.
///
/// Returns false on any other shape (file absent, no `<gate>` block,
/// failed verdict, stale hash, parse failure, or read error). Treating
/// parse failures as "not fresh" is deliberate: re-vetting is safer
/// than shipping on a malformed artifact.
fn vet_gate_is_fresh_pass(spec: &ParsedSpec) -> bool {
    let vet_path = spec.dir.join("journal").join("VET.md");
    let Ok(vet_bytes) = fs_err::read(vet_path.as_std_path()) else {
        return false;
    };
    let Ok(vet_text) = std::str::from_utf8(&vet_bytes) else {
        return false;
    };
    let Some(gate) = last_gate_block(vet_text) else {
        return false;
    };
    if gate.verdict != "passed" {
        return false;
    }
    let tasks_path = spec.dir.join("TASKS.md");
    let Ok(tasks_bytes) = fs_err::read(tasks_path.as_std_path()) else {
        return false;
    };
    let actual_hash = const_hex::encode(Sha256::digest(&tasks_bytes));
    gate.tasks_hash.eq_ignore_ascii_case(&actual_hash)
}

#[derive(Debug, Clone)]
struct GateBlock {
    verdict: String,
    tasks_hash: String,
}

/// Scan the VET.md text for the final `<gate ...>` opening tag and
/// extract its `verdict` and `tasks_hash` attributes. Returns `None`
/// when no `<gate ...>` tag is present or the required attributes are
/// missing.
///
/// This is a deliberately small, tolerant parser: the `<gate>` block
/// grammar is owned by the skill layer (SPEC-0041 REQ-003), and a
/// strict XML parse would couple this resolver to that grammar's
/// whitespace and ordering. The resolver only needs the two
/// attributes off the most recent opening tag.
fn last_gate_block(text: &str) -> Option<GateBlock> {
    let mut last: Option<GateBlock> = None;
    let mut cursor = text;
    while let Some(open_idx) = cursor.find("<gate") {
        let after_open = cursor.get(open_idx..)?;
        // Require the next char after "<gate" to be whitespace or '>',
        // so we do not match a hypothetical `<gateway>` tag.
        let following = after_open.get("<gate".len()..)?;
        let first_char = following.chars().next();
        if !matches!(first_char, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
            // Advance past this `<gate` literal and keep scanning.
            cursor = following;
            continue;
        }
        let close_idx = following.find('>')?;
        let attrs = following.get(..close_idx)?;
        let verdict = attribute_value(attrs, "verdict")?;
        let tasks_hash = attribute_value(attrs, "tasks_hash")?;
        last = Some(GateBlock {
            verdict,
            tasks_hash,
        });
        cursor = following.get(close_idx..)?;
    }
    last
}

/// Extract a double-quoted attribute value from an opening-tag attribute
/// string, e.g. `verdict` from ` verdict="passed" tasks_hash="..."`.
fn attribute_value(attrs: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = attrs.find(&needle)?;
    let value_start = start.checked_add(needle.len())?;
    let rest = attrs.get(value_start..)?;
    let end = rest.find('"')?;
    Some(rest.get(..end)?.to_owned())
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
    use super::attribute_value;
    use super::default_personas;
    use super::last_gate_block;

    #[test]
    fn default_personas_is_the_first_four_of_all() {
        assert_eq!(
            default_personas(),
            &["business", "tests", "security", "style"],
        );
    }

    #[test]
    fn attribute_value_extracts_simple() {
        assert_eq!(
            attribute_value(" verdict=\"passed\" tasks_hash=\"abc123\"", "verdict")
                .expect("present"),
            "passed",
        );
        assert_eq!(
            attribute_value(" verdict=\"passed\" tasks_hash=\"abc123\"", "tasks_hash")
                .expect("present"),
            "abc123",
        );
        assert!(attribute_value(" verdict=\"passed\"", "missing").is_none());
    }

    #[test]
    fn last_gate_block_picks_final_block() {
        let text = r#"
## Invocation 1

<gate verdict="failed" tasks_hash="deadbeef" date="2026-01-01T00:00:00Z">
First attempt failed.
</gate>

## Invocation 2

<gate verdict="passed" tasks_hash="cafef00d" date="2026-01-02T00:00:00Z">
Second attempt passed.
</gate>
"#;
        let block = last_gate_block(text).expect("two gate blocks present");
        assert_eq!(block.verdict, "passed");
        assert_eq!(block.tasks_hash, "cafef00d");
    }

    #[test]
    fn last_gate_block_returns_none_when_no_gate_tag() {
        assert!(last_gate_block("# VET\n\nno gate here.\n").is_none());
    }

    #[test]
    fn last_gate_block_ignores_unrelated_prefixes() {
        // A made-up `<gateway>` tag must not be treated as a `<gate>`.
        let text = "<gateway verdict=\"passed\" tasks_hash=\"x\">body</gateway>\n";
        assert!(last_gate_block(text).is_none());
    }
}
