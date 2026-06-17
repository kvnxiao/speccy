//! Serde `Serialize` structs for the `speccy context` bundle envelope.
//!
//! `speccy context <selector> --json` emits one schema-versioned JSON bundle:
//! task selectors keep the task-scoped envelope, while a bare `SPEC-NNNN`
//! emits the spec-scoped vet envelope. This module owns the envelopes'
//! `Serialize` shapes; assembly from parsed workspace state lives in
//! [`crate::context`].
//!
//! The envelope follows the workspace-wide convention established by
//! `next_output.rs`: `schema_version` is the first serialized field,
//! pinned at `1` pre-v1. The envelope carries spec identity
//! (`schema_version` + id/title/status) and the intent block
//! (goals / non-goals / decisions); the selected task's verbatim `<task>`
//! entry and the covering requirements resolved through the shared core
//! walk; the inlined per-task journal, whose per-block JSON shape reuses
//! `journal show`'s `JsonJournalBlock` so the two journal views cannot
//! drift; the navigation aids — a sibling-task index (id/state/covers
//! only), the repo-relative SPEC.md / TASKS.md / journal paths, and a
//! suggested merge-base diff command; and the consistency section carrying
//! the workspace-level status plus only the drift entries scoped to the
//! selected task.

use crate::journal_show_output::JsonJournalBlock;
use crate::journal_show_output::JsonJournalBlockAttrs;
use crate::journal_show_output::JsonVetBlock;
use crate::journal_show_output::JsonVetBlockAttrs;
use serde::Serialize;
use speccy_core::consistency::ConsistencyBlock;

/// The `speccy context` JSON bundle envelope.
///
/// `schema_version` is declared first so it is the first serialized
/// field, matching the `next` / `status` / `journal show` envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct ContextBundle {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// Spec identity from SPEC.md frontmatter.
    pub spec: SpecIdentity,
    /// Authoring-intent slice: goals, non-goals, decisions.
    pub intent: Intent,
    /// The selected task's verbatim `<task>` entry plus its parsed id,
    /// state, and covers.
    pub task: TaskEntry,
    /// The requirements the task covers, full bodies with scenarios,
    /// deduplicated in covers-list order.
    pub requirements: Vec<CoveringRequirement>,
    /// The selected task's per-task journal, inlined in full when present;
    /// an explicit empty marker when the file does not yet exist.
    pub journal: BundleJournal,
    /// Every other task in the spec as id/state/covers only — never any
    /// body text — in TASKS.md declared order.
    pub siblings: Vec<SiblingEntry>,
    /// Repo-relative paths to SPEC.md, TASKS.md, and the task's journal
    /// file for follow-up targeted reads.
    pub paths: BundlePaths,
    /// A suggested `git diff` command string in merge-base form against the
    /// repo's default branch, runnable as-is from the repo root.
    pub diff_command: String,
    /// Workspace consistency status plus only the drift entries scoped to
    /// the selected task — other tasks' drifts never appear. The
    /// `status` is the same workspace-level classification `speccy next`
    /// computes; the `drifts` list is filtered to the selected task. A
    /// clean workspace yields `status == ok` with zero entries. Reuses
    /// `speccy next`'s [`ConsistencyBlock`] verbatim so the two
    /// consistency views cannot drift.
    pub consistency: ConsistencyBlock,
}

/// Spec identity drawn from SPEC.md frontmatter.
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

/// The authoring-intent slice of the bundle.
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

/// One `<decision>` projected into the bundle.
#[derive(Debug, Clone, Serialize)]
pub struct DecisionEntry {
    /// The `DEC-NNN` id.
    pub id: String,
    /// The decision body, verbatim.
    pub body: String,
}

/// The selected task's `<task>` entry projected into the bundle.
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

/// One covering requirement projected into the bundle.
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

/// One `<scenario>` nested inside a covering requirement.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioEntry {
    /// The `CHK-NNN` id.
    pub id: String,
    /// The scenario body, verbatim.
    pub body: String,
}

/// The selected task's per-task journal, sliced to its latest round and
/// inlined into the bundle.
///
/// When `<spec-dir>/journal/<task-id>.md` exists, `exists` is `true`,
/// the frontmatter fields carry the parsed `spec` / `task` / `generated_at`,
/// and `blocks` holds the `<implementer>` / `<review>` / `<blockers>` entries
/// of the journal's highest round in file order. Prior-round bodies are not
/// inlined — `prior_rounds` carries an attributes-only index of them,
/// and the full prose remains reachable on demand via
/// `speccy journal show <selector> --round N`. When the file does not exist,
/// `exists` is `false`, the frontmatter fields are absent, and `blocks` is
/// empty — a round-1 implementer legitimately has no journal yet,
/// so absence is normal and the command still exits 0.
///
/// The per-block JSON shape reuses the shared [`JsonJournalBlock`] (and its
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
    /// The latest round's journal blocks in file order; empty when the file
    /// is absent or parses to zero entries.
    pub blocks: Vec<JsonJournalBlock>,
    /// An attributes-only index of every block whose round is strictly below
    /// the highest round, in file order. Prior-round
    /// bodies are never inlined; this index tells an agent that history exists
    /// and what shape it has, with the full prose reachable on demand via
    /// `speccy journal show <selector> --round N`. Empty for single-round,
    /// zero-entry, and absent journals. Together with `blocks` it forms a total
    /// and disjoint partition of the parsed entries: round equals highest →
    /// `blocks`; round below highest → `prior_rounds`.
    pub prior_rounds: Vec<JsonJournalBlockAttrs>,
}

/// One sibling task projected into the bundle's navigation index.
///
/// Carries only the parsed `id`, `state`, and `covers` — never any body
/// text. The index lets an implementer's reuse survey see which adjacent
/// slices already landed without reading TASKS.md, while keeping the bundle
/// size bounded to one line per task (the only field that grows with task
/// count).
#[derive(Debug, Clone, Serialize)]
pub struct SiblingEntry {
    /// The sibling's `T-NNN` id.
    pub id: String,
    /// The sibling's state in its on-disk string form (e.g. `completed`).
    pub state: String,
    /// The sibling's `covers` requirement ids in source order.
    pub covers: Vec<String>,
}

/// Repo-relative paths to the spec's files for follow-up targeted reads.
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

/// The `speccy context SPEC-NNNN --json` bundle envelope for whole-SPEC vet
/// workflows.
///
/// `schema_version` is declared first so it is the first serialized field,
/// matching the task-scoped context bundle and the other JSON envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct SpecContextBundle {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// Spec identity from SPEC.md frontmatter.
    pub spec: SpecIdentity,
    /// Authoring-intent slice: goals, non-goals, decisions.
    pub intent: Intent,
    /// Every requirement contract in declared order.
    pub requirements: Vec<CoveringRequirement>,
    /// Every task as a compact index entry: id, state, covers, and title.
    pub tasks: Vec<SpecTaskEntry>,
    /// Every task whose state is not `completed`, using the same compact
    /// entry shape as `tasks`.
    pub non_completed_tasks: Vec<SpecTaskEntry>,
    /// The per-SPEC VET journal sliced to the latest invocation in full plus
    /// prior invocations as attributes only.
    pub vet_journal: SpecVetJournal,
    /// Repo-relative paths to the spec's files for follow-up targeted reads.
    pub paths: SpecBundlePaths,
    /// A suggested working-tree `git diff` command against the repo's default
    /// branch, runnable as-is from the repo root.
    pub diff_command: String,
}

/// One task projected into the spec-scoped task index.
#[derive(Debug, Clone, Serialize)]
pub struct SpecTaskEntry {
    /// The `T-NNN` id.
    pub id: String,
    /// The task state in its on-disk string form.
    pub state: String,
    /// The `covers` requirement ids in source order.
    pub covers: Vec<String>,
    /// The first `##` heading found in the task body, or the empty string when
    /// the task body has no title heading.
    pub title: String,
}

/// The spec-scoped VET journal projection.
///
/// When `<spec-dir>/journal/VET.md` exists, `exists` is `true`, the
/// frontmatter fields carry the parsed `spec` / `generated_at`, and
/// `latest_invocation` holds the last invocation section's full blocks. Prior
/// invocation prose is not inlined: `prior_invocations` carries only
/// attributes so agents can see that older history exists and drill into it
/// with `speccy journal show SPEC-NNNN --json` when needed. When VET.md is
/// absent, `exists` is `false`, the frontmatter fields are absent,
/// `latest_invocation` is `null`, and `prior_invocations` is empty.
#[derive(Debug, Clone, Serialize)]
pub struct SpecVetJournal {
    /// Whether `journal/VET.md` exists.
    pub exists: bool,
    /// `spec:` frontmatter field; present only when the file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    /// `generated_at:` frontmatter field; present only when the file exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    /// The latest invocation section in full, or `null` when the journal is
    /// absent or has no parsed invocation sections.
    pub latest_invocation: Option<SpecVetInvocation>,
    /// Prior invocation sections as attributes-only block indexes.
    pub prior_invocations: Vec<SpecVetInvocationAttrs>,
}

/// One VET invocation section projected with full block bodies.
#[derive(Debug, Clone, Serialize)]
pub struct SpecVetInvocation {
    /// The `N` in `## Invocation N`.
    pub number: u32,
    /// The ISO8601 datetime on the heading line.
    pub date: String,
    /// The highest round among round-bearing blocks in this invocation.
    pub latest_round: Option<u32>,
    /// Blocks in document order, with bodies included.
    pub blocks: Vec<JsonVetBlock>,
}

/// One prior VET invocation projected without block bodies.
#[derive(Debug, Clone, Serialize)]
pub struct SpecVetInvocationAttrs {
    /// The `N` in `## Invocation N`.
    pub number: u32,
    /// The ISO8601 datetime on the heading line.
    pub date: String,
    /// The highest round among round-bearing blocks in this invocation.
    pub latest_round: Option<u32>,
    /// Blocks in document order, attributes only.
    pub blocks: Vec<JsonVetBlockAttrs>,
}

/// Repo-relative paths surfaced by the spec-scoped context bundle.
#[derive(Debug, Clone, Serialize)]
pub struct SpecBundlePaths {
    /// Repo-relative forward-slash path to the spec's `SPEC.md`.
    pub spec_md: String,
    /// Repo-relative forward-slash path to the spec's `TASKS.md`.
    pub tasks_md: String,
    /// Repo-relative forward-slash path to `journal/VET.md`, whether or not
    /// it exists yet.
    pub vet_journal: String,
}
