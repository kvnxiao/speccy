//! Serde `Serialize` structs for the `speccy context` bundle envelope.
//!
//! `speccy context <task-selector> --json` emits one schema-versioned
//! JSON bundle scoped to a single task. This module owns the envelope's
//! `Serialize` shape; assembly from parsed workspace state lives in
//! [`crate::context`].
//!
//! The envelope follows the workspace-wide convention established by
//! `next_output.rs`: `schema_version` is the first serialized field,
//! pinned at `1` pre-v1. SPEC-0056 grows this envelope across tasks
//! T-002..T-006; this file carries the T-002 slice — spec identity
//! (REQ-001's `schema_version` + REQ-002's id/title/status) and the
//! intent block (REQ-002's goals / non-goals / decisions) — plus the
//! T-003 slice: the selected task's verbatim `<task>` entry and the
//! covering requirements resolved through the shared core walk
//! (REQ-003); and the T-004 slice: the inlined per-task journal, whose
//! per-block JSON shape reuses `journal show`'s `JsonJournalBlock` so the
//! two journal views cannot drift (REQ-004); and the T-005 slice: the
//! navigation aids — a sibling-task index (id/state/covers only), the
//! repo-relative SPEC.md / TASKS.md / journal paths, and a suggested
//! merge-base diff command (REQ-005). Later tasks add the consistency
//! section.
//!
//! See `.speccy/specs/0056-task-context-bundle/SPEC.md`.

use crate::journal_show_output::JsonJournalBlock;
use serde::Serialize;

/// The `speccy context` JSON bundle envelope.
///
/// `schema_version` is declared first so it is the first serialized
/// field, matching the `next` / `status` / `journal show` envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct ContextBundle {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// Spec identity from SPEC.md frontmatter (REQ-002).
    pub spec: SpecIdentity,
    /// Authoring-intent slice: goals, non-goals, decisions (REQ-002).
    pub intent: Intent,
    /// The selected task's verbatim `<task>` entry plus its parsed id,
    /// state, and covers (REQ-003).
    pub task: TaskEntry,
    /// The requirements the task covers, full bodies with scenarios,
    /// deduplicated in covers-list order (REQ-003).
    pub requirements: Vec<CoveringRequirement>,
    /// The selected task's per-task journal, inlined in full when present;
    /// an explicit empty marker when the file does not yet exist (REQ-004).
    pub journal: BundleJournal,
    /// Every other task in the spec as id/state/covers only — never any
    /// body text — in TASKS.md declared order (REQ-005).
    pub siblings: Vec<SiblingEntry>,
    /// Repo-relative paths to SPEC.md, TASKS.md, and the task's journal
    /// file for follow-up targeted reads (REQ-005).
    pub paths: BundlePaths,
    /// A suggested `git diff` command string in merge-base form against the
    /// repo's default branch, runnable as-is from the repo root (REQ-005).
    pub diff_command: String,
}

/// Spec identity drawn from SPEC.md frontmatter (REQ-002).
#[derive(Debug, Clone, Serialize)]
pub struct SpecIdentity {
    /// Frontmatter `id` (`SPEC-NNNN`).
    pub id: String,
    /// Frontmatter `title`.
    pub title: String,
    /// Frontmatter `status`, in its on-disk string form (e.g.
    /// `in-progress`).
    pub status: String,
}

/// The authoring-intent slice of the bundle (REQ-002).
///
/// Carries the `<goals>` and `<non-goals>` bodies verbatim plus every
/// `<decision>` with its DEC id and body. The Summary narrative,
/// `<user-stories>`, Notes, and non-covered requirement bodies are
/// deliberately excluded — they are not part of the task-scoped intent
/// slice.
#[derive(Debug, Clone, Serialize)]
pub struct Intent {
    /// Body of the `<goals>` element, verbatim.
    pub goals: String,
    /// Body of the `<non-goals>` element, verbatim.
    pub non_goals: String,
    /// Every `<decision>` in declared order.
    pub decisions: Vec<DecisionEntry>,
}

/// One `<decision>` projected into the bundle (REQ-002).
#[derive(Debug, Clone, Serialize)]
pub struct DecisionEntry {
    /// The `DEC-NNN` id.
    pub id: String,
    /// The decision body, verbatim.
    pub body: String,
}

/// The selected task's `<task>` entry projected into the bundle (REQ-003).
///
/// Carries the parsed `id`, `state`, and `covers` alongside the verbatim
/// `<task>` body bytes, so a consumer reads the task entry from the bundle
/// without a TASKS.md read.
#[derive(Debug, Clone, Serialize)]
pub struct TaskEntry {
    /// The `T-NNN` id.
    pub id: String,
    /// The task state in its on-disk string form (e.g. `in-progress`).
    pub state: String,
    /// The `covers` requirement ids in source order.
    pub covers: Vec<String>,
    /// The verbatim body between the `<task>` open and close tags.
    pub body: String,
}

/// One covering requirement projected into the bundle (REQ-003).
///
/// Resolved through the shared `resolve_covering_requirements` walk so
/// `context` and `check` cannot diverge. The `body` is the requirement's
/// verbatim markdown (heading title plus prose); `done_when` and
/// `behavior` are the nested sub-element bodies; `scenarios` carries every
/// `<scenario>` in source order.
#[derive(Debug, Clone, Serialize)]
pub struct CoveringRequirement {
    /// The `REQ-NNN` id.
    pub id: String,
    /// The requirement body, verbatim (heading title and prose).
    pub body: String,
    /// Body of the nested `<done-when>` sub-element, verbatim.
    pub done_when: String,
    /// Body of the nested `<behavior>` sub-element, verbatim.
    pub behavior: String,
    /// Every nested `<scenario>` in source order.
    pub scenarios: Vec<ScenarioEntry>,
}

/// One `<scenario>` nested inside a covering requirement (REQ-003).
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioEntry {
    /// The `CHK-NNN` id.
    pub id: String,
    /// The scenario body, verbatim.
    pub body: String,
}

/// The selected task's per-task journal, inlined into the bundle (REQ-004).
///
/// When `<spec-dir>/journal/<task-id>.md` exists, `exists` is `true`,
/// the frontmatter fields carry the parsed `spec` / `task` / `generated_at`,
/// and `blocks` holds every `<implementer>` / `<review>` / `<blockers>`
/// entry across all rounds in file order. When the file does not exist,
/// `exists` is `false`, the frontmatter fields are absent, and `blocks` is
/// empty — a round-1 implementer legitimately has no journal yet (DEC-004),
/// so absence is normal and the command still exits 0.
///
/// The per-block JSON shape reuses SPEC-0055's [`JsonJournalBlock`] (and its
/// `to_json_journal_block` mapping) so `context` and `journal show` cannot
/// drift. The standalone `JsonTaskJournal` envelope is deliberately **not**
/// nested here: its own `schema_version` would collide with the bundle's.
#[derive(Debug, Clone, Serialize)]
pub struct BundleJournal {
    /// Whether the journal file exists. `false` is the explicit
    /// empty-journal marker for a task with no journal yet.
    pub exists: bool,
    /// `spec:` frontmatter field; present only when the file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    /// `task:` frontmatter field; present only when the file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// `generated_at:` frontmatter field; present only when the file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    /// Every journal block in file order; empty when the file is absent.
    pub blocks: Vec<JsonJournalBlock>,
}

/// One sibling task projected into the bundle's navigation index (REQ-005).
///
/// Carries only the parsed `id`, `state`, and `covers` — never any body
/// text. The index lets an implementer's reuse survey see which adjacent
/// slices already landed without reading TASKS.md, while keeping the bundle
/// size bounded to one line per task (the only field that grows with task
/// count, per REQ-007).
#[derive(Debug, Clone, Serialize)]
pub struct SiblingEntry {
    /// The sibling's `T-NNN` id.
    pub id: String,
    /// The sibling's state in its on-disk string form (e.g. `completed`).
    pub state: String,
    /// The sibling's `covers` requirement ids in source order.
    pub covers: Vec<String>,
}

/// Repo-relative paths to the spec's files for follow-up targeted reads
/// (REQ-005).
///
/// All paths are forward-slash strings relative to the project root, so a
/// consumer can read SPEC.md, TASKS.md, or the journal directly when it
/// needs something outside the bundle. The journal path is surfaced even
/// when the file does not yet exist — it is the path a round-1 implementer
/// will write to.
#[derive(Debug, Clone, Serialize)]
pub struct BundlePaths {
    /// Repo-relative forward-slash path to the spec's `SPEC.md`.
    pub spec_md: String,
    /// Repo-relative forward-slash path to the spec's `TASKS.md`.
    pub tasks_md: String,
    /// Repo-relative forward-slash path to the task's journal file under
    /// `<spec-dir>/journal/<task-id>.md`, whether or not it exists yet.
    pub journal: String,
}
