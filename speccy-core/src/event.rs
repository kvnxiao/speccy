//! The append-only event log vocabulary (DESIGN § Storage Model, JSONL-first).
//!
//! Spec-scoped and run-scoped `events.jsonl` are canonical; projections are
//! rebuilt by replay (`projection.rs`). Base-fact events are written by their
//! recording operation. Derived task/run transitions are written by `run next`,
//! the single mutation point for derived state; explicit gate operations also
//! write their owning run-state transitions (DESIGN § Deterministic Loop
//! Driving).

use crate::model::ChangeRef;
use crate::model::EvidenceControl;
use crate::model::EvidenceKind;
use crate::model::FindingSeverity;
use crate::model::RequirementStatus;
use crate::model::RiskTier;
use crate::model::RunDecisionKind;
use crate::model::RunState;
use crate::model::SpecDecisionKind;
use crate::model::SpecDraft;
use crate::model::TaskStatus;
use serde::Deserialize;
use serde::Serialize;

/// One line of a `events.jsonl` file: a timestamp plus a tagged event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedEvent {
    pub ts: jiff::Timestamp,
    #[serde(flatten)]
    pub event: Event,
}

impl LoggedEvent {
    /// Stamp an event with the current wall-clock time.
    #[must_use = "constructs a value that must be used"]
    pub fn now(event: Event) -> Self {
        Self {
            ts: jiff::Timestamp::now(),
            event,
        }
    }
}

/// A tagged, append-only fact in a spec- or run-scoped `events.jsonl`.
///
/// Serialized with a `type` discriminant; projections replay these to rebuild
/// read models (`projection.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // --- spec-scoped ---
    /// A new spec (and its workspace) was created from a request.
    SpecCreated {
        spec_ref: String,
        spec_id: String,
        workspace_id: String,
        request: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        brainstorm_handoff: Option<String>,
    },
    /// The full post-state of a draft revision after record-draft/patch-draft.
    DraftUpdated {
        revision_id: String,
        draft: SpecDraft,
    },
    /// A spec-scoped decision (approve, reject, split, scope change, cancel).
    SpecDecision { decision: SpecDecisionRecord },
    /// Spec status transition not driven by a decision (accepted, archived,
    /// superseded), written by the owning human/controller op.
    SpecStatusChanged { to: crate::model::SpecStatus },

    // --- run-scoped ---
    /// A run began against an approved revision, seeding its task list.
    /// `risk` is a closed [`RiskTier`]; an out-of-vocabulary stored value
    /// fails replay rather than falling back to `standard`.
    RunStarted {
        run_id: String,
        spec_ref: String,
        spec_id: String,
        revision_id: String,
        risk: RiskTier,
        branch: String,
        base_commit: String,
        tasks: Vec<TaskInit>,
    },
    /// An agent claimed a task, pinning its baseline commit.
    TaskClaimed {
        task: String,
        agent: String,
        baseline_commit: String,
    },
    /// A worker recorded a handoff for a task round.
    HandoffRecorded {
        handoff_id: String,
        task: String,
        round: u32,
        handoff: Handoff,
    },
    /// A stored evidence artifact for a requirement.
    EvidenceRecorded { evidence: EvidenceRecord },
    /// A structured reviewer finding.
    FindingRecorded { finding: FindingRecord },
    /// One or more requirement status transitions.
    RequirementStatusSet { updates: Vec<RequirementUpdate> },
    /// Derived task transition applied by `run next`.
    TaskTransitioned {
        task: String,
        to: TaskStatus,
        round: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        snapshot: Option<String>,
    },
    /// A dynamically appended run-level repair/rework task.
    TaskAppended {
        task: TaskInit,
        #[serde(skip_serializing_if = "Option::is_none")]
        seed_feedback: Option<String>,
    },
    /// Derived or op-driven run-state transition.
    RunStateTransitioned {
        to: RunState,
        #[serde(skip_serializing_if = "Option::is_none")]
        snapshot: Option<String>,
    },
    /// A run-scoped gate decision (ship, escalation, accepted-risk). One
    /// committed logical event: replay applies the decision's complete
    /// outcome — cancellation, escalation with its snapshot, the rework
    /// `RT<n>` task and re-entry, or the waived status plus gate resume
    /// (DESIGN § Storage Model, "Write guarantees and crash recovery").
    RunDecision { decision: RunDecisionRecord },
    /// The landed change reference recorded at ship time. One committed
    /// logical event: replay applies the `submitted` transition.
    ShipRecorded { change_ref: ChangeRef },
}

/// A task's seed definition at run start or when appended mid-run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInit {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

/// A worker handoff (SCHEMAS § task record-handoff).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handoff {
    pub task: String,
    pub round: u32,
    pub summary: String,
    #[serde(default)]
    pub files_touched: Vec<String>,
    #[serde(default)]
    pub commands_run: Vec<CommandRun>,
    #[serde(default)]
    pub requirements_claimed: Vec<String>,
    #[serde(default)]
    pub known_issues: Vec<String>,
    #[serde(default)]
    pub deviations: Vec<String>,
    #[serde(default)]
    pub follow_ups: Vec<String>,
}

/// A command a worker ran, with its exit code, reported in a handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRun {
    pub command: String,
    pub exit_code: i32,
}

/// A stored evidence artifact (SCHEMAS § evidence record / collect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub requirement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    pub kind: EvidenceKind,
    pub collected_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_hash: Option<String>,
    // command-evidence fields (M2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<EvidenceRepoIdentity>,
    // Boxed: the control record (a second execution plus repo identity) would
    // otherwise dominate the `Event` enum's size for every stored event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<Box<EvidenceControlRecord>>,
}

/// Before/after repository identity captured around a `kind: command`
/// evidence execution, so the command's effect is attributable to exact
/// repository states (DESIGN § Acceptance Ledger).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRepoIdentity {
    pub head_before: String,
    pub head_after: String,
    pub head_changed: bool,
    pub diff_hash_before: String,
    pub diff_hash_after: String,
    #[serde(default)]
    pub newly_dirty: Vec<String>,
}

/// The recorded outcome of a declared fail-before/pass-after control
/// (SCHEMAS § evidence collect; semantics in DESIGN § Acceptance Ledger).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceControlRecord {
    pub kind: EvidenceControl,
    pub status: ControlStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<ControlBaselineRecord>,
    pub isolation: ControlIsolation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The baseline-side execution of a controlled command, run against the
/// pinned run base commit in an isolated worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlBaselineRecord {
    pub commit: String,
    pub exit_code: i32,
    pub stdout_hash: String,
    pub artifact: String,
    pub artifact_hash: String,
    pub contained: bool,
    pub repo: EvidenceRepoIdentity,
}

/// Where the baseline ran and whether that path was verifiably cleaned up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlIsolation {
    pub path: String,
    pub cleanup: ControlCleanup,
}

/// Cleanup verification result for the control's isolation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCleanup {
    /// The isolation worktree was removed and its absence verified.
    Removed,
    /// The isolation path still exists; it is surfaced in the control note
    /// and the control never reports `passed`.
    Leaked,
}

/// Control verdict vocabulary (closed; unknown stored values fail replay).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlStatus {
    /// Baseline failed and candidate succeeded.
    Passed,
    /// Baseline passed (vacuous evidence) or the candidate failed.
    Failed,
    /// The control could not be established; the note names why.
    Blocked,
}

/// A structured reviewer finding (SCHEMAS § finding record).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingRecord {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    pub severity: FindingSeverity,
    pub note: String,
    pub recorded_by: String,
}

/// One requirement status transition (SCHEMAS § requirement set-status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementUpdate {
    pub requirement: String,
    pub status: RequirementStatus,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub findings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_risk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A spec-scoped decision (SCHEMAS § spec record-decision).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecDecisionRecord {
    pub decision_id: String,
    #[serde(rename = "type")]
    pub kind: SpecDecisionKind,
    pub revision_id: String,
    pub actor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_in_prose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default)]
    pub carry_forward: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<Supersedes>,
}

/// What a superseding decision replaces: a prior spec and/or run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supersedes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

/// A run-scoped gate decision (SCHEMAS § run record-decision). `snapshot` is
/// controller-set on an `interrupt` decision: the labeled escalation snapshot
/// committed just before the decision was recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDecisionRecord {
    pub decision_id: String,
    #[serde(rename = "type")]
    pub kind: RunDecisionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    pub actor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_risk: Option<String>,
    #[serde(default)]
    pub carry_forward: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}
