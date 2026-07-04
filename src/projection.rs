//! Read models rebuilt by replaying the event log (DESIGN § Storage Model).
//!
//! `SpecState` and `RunProjection` are pure functions of their event streams.
//! Directive logic reads them; it never infers state from anything else.

use std::collections::BTreeMap;

use jiff::Timestamp;

use crate::event::{Event, LoggedEvent};
use crate::event::{EvidenceRecord, FindingRecord, Handoff, RunDecisionRecord, SpecDecisionRecord};
use crate::model::{
    ChangeRef, RequirementStatus, RiskTier, RunState, SpecDraft, SpecStatus, TaskStatus,
};

// --------------------------------------------------------------------------
// Spec projection
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Revision {
    pub id: String,
    pub approved: bool,
    pub draft: SpecDraft,
}

#[derive(Debug, Clone)]
pub struct SpecState {
    pub spec_ref: String,
    pub spec_id: String,
    pub workspace_id: String,
    pub request: String,
    pub source: Option<String>,
    pub title: Option<String>,
    pub brainstorm_handoff: Option<String>,
    pub status: SpecStatus,
    pub revisions: Vec<Revision>,
    pub decisions: Vec<SpecDecisionRecord>,
    pub last_event_ts: Option<Timestamp>,
}

impl SpecState {
    /// Rebuild spec state from its event stream.
    pub fn replay(events: &[LoggedEvent]) -> Option<SpecState> {
        let mut state: Option<SpecState> = None;
        for logged in events {
            let ts = logged.ts;
            match &logged.event {
                Event::SpecCreated {
                    spec_ref,
                    spec_id,
                    workspace_id,
                    request,
                    source,
                    title,
                    brainstorm_handoff,
                } => {
                    state = Some(SpecState {
                        spec_ref: spec_ref.clone(),
                        spec_id: spec_id.clone(),
                        workspace_id: workspace_id.clone(),
                        request: request.clone(),
                        source: source.clone(),
                        title: title.clone(),
                        brainstorm_handoff: brainstorm_handoff.clone(),
                        status: SpecStatus::Draft,
                        revisions: Vec::new(),
                        decisions: Vec::new(),
                        last_event_ts: Some(ts),
                    });
                }
                Event::DraftUpdated { revision_id, draft } => {
                    if let Some(s) = state.as_mut() {
                        match s.revisions.iter_mut().find(|r| &r.id == revision_id) {
                            Some(rev) => rev.draft = draft.clone(),
                            None => s.revisions.push(Revision {
                                id: revision_id.clone(),
                                approved: false,
                                draft: draft.clone(),
                            }),
                        }
                        s.last_event_ts = Some(ts);
                    }
                }
                Event::SpecDecision { decision } => {
                    if let Some(s) = state.as_mut() {
                        s.decisions.push(decision.clone());
                        match decision.kind.as_str() {
                            "approve" => {
                                if let Some(rev) = s
                                    .revisions
                                    .iter_mut()
                                    .find(|r| r.id == decision.revision_id)
                                {
                                    rev.approved = true;
                                }
                                s.status = SpecStatus::Approved;
                            }
                            "cancel" => s.status = SpecStatus::Cancelled,
                            _ => {}
                        }
                        s.last_event_ts = Some(ts);
                    }
                }
                Event::SpecStatusChanged { to } => {
                    if let Some(s) = state.as_mut() {
                        s.status = *to;
                        s.last_event_ts = Some(ts);
                    }
                }
                _ => {}
            }
        }
        state
    }

    /// The current (latest) revision, draft or approved.
    pub fn latest_revision(&self) -> Option<&Revision> {
        self.revisions.last()
    }

    /// The approved revision, if any.
    pub fn approved_revision(&self) -> Option<&Revision> {
        self.revisions.iter().find(|r| r.approved)
    }

    pub fn revision(&self, id: &str) -> Option<&Revision> {
        self.revisions.iter().find(|r| r.id == id)
    }
}

// --------------------------------------------------------------------------
// Run projection
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TaskState {
    pub id: String,
    pub title: Option<String>,
    pub requirements: Vec<String>,
    pub constraints: Vec<String>,
    pub status: TaskStatus,
    pub round: u32,
    pub baseline_commit: Option<String>,
    pub snapshot: Option<String>,
    pub seed_feedback: Option<String>,
    /// Sequence index of this task's latest recorded handoff.
    pub last_handoff_seq: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ReqRuntime {
    pub status: RequirementStatus,
    pub evidence: Vec<String>,
    pub findings: Vec<String>,
    pub residual_risk: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HandoffMeta {
    pub handoff_id: String,
    pub task: String,
    pub round: u32,
    pub handoff: Handoff,
}

#[derive(Debug, Clone)]
pub struct RunProjection {
    pub run_id: String,
    pub spec_ref: String,
    pub spec_id: String,
    pub revision_id: String,
    pub risk: RiskTier,
    pub branch: String,
    pub base_commit: String,
    pub state: RunState,
    pub tasks: Vec<TaskState>,
    pub requirements: BTreeMap<String, ReqRuntime>,
    pub evidence: Vec<EvidenceRecord>,
    pub findings: Vec<(usize, FindingRecord)>,
    pub handoffs: Vec<HandoffMeta>,
    pub decisions: Vec<RunDecisionRecord>,
    pub change_ref: Option<ChangeRef>,
    pub last_snapshot: Option<String>,
    pub last_event_ts: Option<Timestamp>,
    max_status_seq: Option<usize>,
    verifying_entered_seq: Option<usize>,
    pub run_review_rounds_completed: u32,
}

impl RunProjection {
    /// Rebuild run state from its event stream.
    pub fn replay(events: &[LoggedEvent]) -> Option<RunProjection> {
        let mut run: Option<RunProjection> = None;
        for (seq, logged) in events.iter().enumerate() {
            let ts = logged.ts;
            match &logged.event {
                Event::RunStarted {
                    run_id,
                    spec_ref,
                    spec_id,
                    revision_id,
                    risk,
                    branch,
                    base_commit,
                    tasks,
                } => {
                    let mut requirements = BTreeMap::new();
                    let task_states: Vec<TaskState> = tasks
                        .iter()
                        .map(|t| {
                            for r in &t.requirements {
                                requirements
                                    .entry(r.clone())
                                    .or_insert_with(ReqRuntime::default);
                            }
                            TaskState {
                                id: t.id.clone(),
                                title: t.title.clone(),
                                requirements: t.requirements.clone(),
                                constraints: t.constraints.clone(),
                                status: TaskStatus::Queued,
                                round: 0,
                                baseline_commit: None,
                                snapshot: None,
                                seed_feedback: None,
                                last_handoff_seq: None,
                            }
                        })
                        .collect();
                    run = Some(RunProjection {
                        run_id: run_id.clone(),
                        spec_ref: spec_ref.clone(),
                        spec_id: spec_id.clone(),
                        revision_id: revision_id.clone(),
                        risk: RiskTier::parse(risk).unwrap_or(RiskTier::Standard),
                        branch: branch.clone(),
                        base_commit: base_commit.clone(),
                        state: RunState::Implementing,
                        tasks: task_states,
                        requirements,
                        evidence: Vec::new(),
                        findings: Vec::new(),
                        handoffs: Vec::new(),
                        decisions: Vec::new(),
                        change_ref: None,
                        last_snapshot: None,
                        last_event_ts: Some(ts),
                        max_status_seq: None,
                        verifying_entered_seq: None,
                        run_review_rounds_completed: 0,
                    });
                }
                _ => {
                    if let Some(r) = run.as_mut() {
                        r.apply(seq, ts, &logged.event);
                    }
                }
            }
        }
        run
    }

    fn apply(&mut self, seq: usize, ts: Timestamp, event: &Event) {
        self.last_event_ts = Some(ts);
        match event {
            Event::TaskClaimed {
                task,
                baseline_commit,
                ..
            } => {
                if let Some(t) = self.task_mut(task) {
                    t.status = TaskStatus::Building;
                    t.round = 1;
                    t.baseline_commit = Some(baseline_commit.clone());
                }
            }
            Event::HandoffRecorded {
                handoff_id,
                task,
                round,
                handoff,
            } => {
                if let Some(t) = self.task_mut(task) {
                    t.status = TaskStatus::InReview;
                    t.round = *round;
                    t.last_handoff_seq = Some(seq);
                }
                self.handoffs.push(HandoffMeta {
                    handoff_id: handoff_id.clone(),
                    task: task.clone(),
                    round: *round,
                    handoff: handoff.clone(),
                });
            }
            Event::EvidenceRecorded { evidence } => self.evidence.push(evidence.clone()),
            Event::FindingRecorded { finding } => self.findings.push((seq, finding.clone())),
            Event::RequirementStatusSet { updates } => {
                for u in updates {
                    let entry = self.requirements.entry(u.requirement.clone()).or_default();
                    entry.status = u.status;
                    if !u.evidence.is_empty() {
                        entry.evidence = u.evidence.clone();
                    }
                    if !u.findings.is_empty() {
                        entry.findings = u.findings.clone();
                    }
                    if u.residual_risk.is_some() {
                        entry.residual_risk = u.residual_risk.clone();
                    }
                    if u.note.is_some() {
                        entry.note = u.note.clone();
                    }
                }
                self.max_status_seq = Some(seq);
                if self.state == RunState::Verifying {
                    // A set-status while verifying completes a run-level review round.
                    if self.verifying_entered_seq.map(|v| seq > v).unwrap_or(false) {
                        self.run_review_rounds_completed += 1;
                    }
                }
            }
            Event::TaskTransitioned {
                task,
                to,
                round,
                snapshot,
            } => {
                if let Some(t) = self.task_mut(task) {
                    t.status = *to;
                    t.round = *round;
                    if let Some(s) = snapshot {
                        t.snapshot = Some(s.clone());
                    }
                }
                if let Some(s) = snapshot {
                    self.last_snapshot = Some(s.clone());
                }
            }
            Event::TaskAppended {
                task,
                seed_feedback,
            } => {
                for r in &task.requirements {
                    self.requirements.entry(r.clone()).or_default();
                }
                self.tasks.push(TaskState {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    requirements: task.requirements.clone(),
                    constraints: task.constraints.clone(),
                    status: TaskStatus::Queued,
                    round: 0,
                    baseline_commit: None,
                    snapshot: None,
                    seed_feedback: seed_feedback.clone(),
                    last_handoff_seq: None,
                });
            }
            Event::RunStateTransitioned { to, snapshot } => {
                self.state = *to;
                if *to == RunState::Verifying {
                    self.verifying_entered_seq = Some(seq);
                }
                if let Some(s) = snapshot {
                    self.last_snapshot = Some(s.clone());
                }
            }
            Event::RunDecision { decision } => self.decisions.push(decision.clone()),
            Event::ShipRecorded { change_ref } => self.change_ref = Some(change_ref.clone()),
            _ => {}
        }
    }

    pub fn task(&self, id: &str) -> Option<&TaskState> {
        self.tasks.iter().find(|t| t.id == id)
    }
    fn task_mut(&mut self, id: &str) -> Option<&mut TaskState> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn req_status(&self, id: &str) -> RequirementStatus {
        self.requirements
            .get(id)
            .map(|r| r.status)
            .unwrap_or(RequirementStatus::Pending)
    }

    /// All linked requirements of a task are resolved.
    pub fn task_requirements_resolved(&self, task: &TaskState) -> bool {
        task.requirements
            .iter()
            .all(|r| self.req_status(r).is_resolved())
    }

    /// Every requirement in the run is resolved.
    pub fn all_requirements_resolved(&self) -> bool {
        self.requirements.values().all(|r| r.status.is_resolved())
    }

    /// The latest snapshot commit, or the recorded base if none yet.
    pub fn head_expectation(&self) -> &str {
        self.last_snapshot.as_deref().unwrap_or(&self.base_commit)
    }

    /// Sequence index at which the run entered `verifying`, if it has.
    pub fn verifying_entered_seq(&self) -> Option<usize> {
        self.verifying_entered_seq
    }

    /// Whether a controller provenance finding was already recorded this round
    /// (after `guard_seq`) for the given task scope (`None` = run scope).
    pub fn provenance_scanned_after(&self, guard_seq: Option<usize>, task: Option<&str>) -> bool {
        let Some(guard) = guard_seq else { return false };
        self.findings.iter().any(|(seq, f)| {
            *seq > guard
                && f.recorded_by == "controller:provenance-scan"
                && f.task.as_deref() == task
        })
    }

    /// Did the verifier record statuses for this task's current review round?
    /// (Serial writes mean any set-status after the task's handoff is its own.)
    pub fn task_reviewed_this_round(&self, task: &TaskState) -> bool {
        match (self.max_status_seq, task.last_handoff_seq) {
            (Some(status_seq), Some(handoff_seq)) => status_seq > handoff_seq,
            _ => false,
        }
    }

    /// Blocking findings recorded for this task's current round (after its
    /// latest handoff), scoped to the task or its linked requirements.
    pub fn task_blocking_findings_this_round(&self, task: &TaskState) -> Vec<&FindingRecord> {
        let after = task.last_handoff_seq.unwrap_or(0);
        self.findings
            .iter()
            .filter(|(seq, f)| {
                *seq > after
                    && f.severity == "blocking"
                    && (f.task.as_deref() == Some(&task.id)
                        || f.requirement
                            .as_ref()
                            .is_some_and(|r| task.requirements.contains(r)))
            })
            .map(|(_, f)| f)
            .collect()
    }

    /// Unresolved run-level blocking findings (task-less, recorded during the
    /// current verifying phase) — e.g. an integrated-diff provenance hit.
    pub fn run_blocking_findings(&self) -> Vec<&FindingRecord> {
        let after = self.verifying_entered_seq.unwrap_or(usize::MAX);
        self.findings
            .iter()
            .filter(|(seq, f)| *seq > after && f.task.is_none() && f.severity == "blocking")
            .map(|(_, f)| f)
            .collect()
    }

    pub fn next_queued_task(&self) -> Option<&TaskState> {
        self.tasks.iter().find(|t| t.status == TaskStatus::Queued)
    }

    pub fn all_tasks_done(&self) -> bool {
        self.tasks
            .iter()
            .all(|t| matches!(t.status, TaskStatus::Integrated | TaskStatus::Deferred))
    }

    pub fn active_task(&self) -> Option<&TaskState> {
        self.tasks
            .iter()
            .find(|t| matches!(t.status, TaskStatus::Building | TaskStatus::InReview))
    }
}
