//! The append-only event log vocabulary (DESIGN § Storage Model, JSONL-first).
//!
//! Spec-scoped and run-scoped `events.jsonl` are canonical; projections are
//! rebuilt by replay (`projection.rs`). Base-fact events are written by their
//! recording operation. Derived task/run transitions are written by `run next`,
//! the single mutation point for derived state; explicit gate operations also
//! write their owning run-state transitions (DESIGN § Deterministic Loop
//! Driving).

use crate::model::ChangeRef;
use crate::model::RequirementStatus;
use crate::model::RunState;
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
    RunStarted {
        run_id: String,
        spec_ref: String,
        spec_id: String,
        revision_id: String,
        risk: String,
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
    /// A run-scoped gate decision (ship, escalation, accepted-risk).
    RunDecision { decision: RunDecisionRecord },
    /// The landed change reference recorded at ship time.
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
    pub kind: String,
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
    pub severity: String,
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
    pub kind: String, // approve | reject | split | scope_change | cancel
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

/// A run-scoped gate decision (SCHEMAS § run record-decision).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDecisionRecord {
    pub decision_id: String,
    #[serde(rename = "type")]
    // Gate answers: waive | provide_setup | confirm_accepted_risk | rework | cancel.
    // Plus the controller-generated `superseded`, written when an amendment's
    // superseding approval closes this run (DESIGN § Amendment at the Escalation Gate).
    pub kind: String,
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
}
