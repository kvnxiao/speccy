//! `run next` — the single loop-driving operation and the single mutation
//! point for derived state (DESIGN § Deterministic Loop Driving).
//!
//! Each call: apply every derived transition that has no recording operation
//! (task integrate/repair, run implementing→verifying→verified, escalation),
//! creating snapshot commits as it goes, then compute the single next
//! directive. Sequencing, round counting, and gate detection are controller
//! decisions, never prose decisions.

use jiff::ToSpan;
use serde::Serialize;

use crate::config::ProjectConfig;
use crate::error::Result;
use crate::event::Event;
use crate::gitx;
use crate::ids;
use crate::model::{DirectiveAction, Gate, RequirementStatus, RoundScope, RunState, TaskStatus};
use crate::projection::{RunProjection, TaskState};
use crate::store::Store;

/// A derived transition applied by this `run next` call (SCHEMAS § Directive).
#[derive(Debug, Clone, Serialize)]
pub struct AppliedTransition {
    /// `task:<id>` or `run`.
    pub subject: String,
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Directive {
    pub run_state: RunState,
    pub action: DirectiveAction,
    pub subject: Subject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<Round>,
    pub packet_with: Option<String>,
    pub record_with: Option<String>,
    pub reason: String,
    pub applied_transitions: Vec<AppliedTransition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_answers: Option<Vec<GateAnswer>>,
    pub resume: Option<serde_json::Value>,
    pub lease: Lease,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Subject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personas: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Round {
    pub current: u32,
    pub max: u32,
    pub scope: RoundScope,
}

#[derive(Debug, Clone, Serialize)]
pub struct GateAnswer {
    #[serde(rename = "type")]
    pub type_: String,
    pub record_with: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Lease {
    pub token: String,
    pub agent: String,
    pub expires_at: jiff::Timestamp,
}

/// MVP lease TTL (DESIGN § Run Lease and Concurrent Writers).
pub const LEASE_TTL_MINUTES: i64 = 10;

impl Lease {
    /// Issue a fresh lease block. M1 does not persist or enforce it; that is
    /// M2. The lease is excluded from directive idempotency comparison.
    fn issue(agent: &str) -> Lease {
        let expires_at = jiff::Timestamp::now()
            .checked_add(LEASE_TTL_MINUTES.minutes())
            .unwrap_or_else(|_| jiff::Timestamp::now());
        Lease {
            token: ids::short_id("lease"),
            agent: agent.to_string(),
            expires_at,
        }
    }
}

/// Run one `run next` cycle: apply derived transitions, then compute the
/// directive from settled state.
pub fn run_next(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    agent: &str,
) -> Result<serde_json::Value> {
    let applied = advance(store, spec_id, run_id)?;
    let run = store.run_projection(spec_id, run_id)?;
    let config = ProjectConfig::load(&store.workspace_root)?;
    let directive = compute_directive(&run, &config, applied, agent);
    serde_json::to_value(&directive)
        .map_err(|e| crate::error::SpeccyError::io(format!("failed to serialize directive: {e}")))
}

/// Apply derived transitions to a fixpoint, returning those applied this call.
fn advance(store: &Store, spec_id: &str, run_id: &str) -> Result<Vec<AppliedTransition>> {
    let mut applied = Vec::new();
    loop {
        let run = store.run_projection(spec_id, run_id)?;
        let config = ProjectConfig::load(&store.workspace_root)?;
        match step(store, &run, &config)? {
            Some(t) => applied.push(t),
            None => break,
        }
    }
    Ok(applied)
}

/// Apply at most one derived transition. `None` means state is settled.
fn step(
    store: &Store,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    match run.state {
        RunState::Implementing => step_implementing(store, run, config),
        RunState::Verifying => step_verifying(store, run, config),
        _ => Ok(None),
    }
}

fn step_implementing(
    store: &Store,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if let Some(active) = run.active_task() {
        if active.status == TaskStatus::InReview && run.task_reviewed_this_round(active) {
            let blocking = run.task_blocking_findings_this_round(active);
            let resolved = run.task_requirements_resolved(active) && blocking.is_empty();
            if resolved {
                let sha = task_snapshot(store, run, active)?;
                store.append_run_event(
                    &run.spec_id,
                    &run.run_id,
                    Event::TaskTransitioned {
                        task: active.id.clone(),
                        to: TaskStatus::Integrated,
                        round: active.round,
                        snapshot: Some(sha.clone()),
                    },
                )?;
                return Ok(Some(AppliedTransition {
                    subject: format!("task:{}", active.id),
                    from: "in_review".into(),
                    to: "integrated".into(),
                    snapshot: Some(sha),
                }));
            } else if active.round < config.caps.task_repair_rounds {
                store.append_run_event(
                    &run.spec_id,
                    &run.run_id,
                    Event::TaskTransitioned {
                        task: active.id.clone(),
                        to: TaskStatus::Building,
                        round: active.round + 1,
                        snapshot: None,
                    },
                )?;
                return Ok(Some(AppliedTransition {
                    subject: format!("task:{}", active.id),
                    from: "in_review".into(),
                    to: "building".into(),
                    snapshot: None,
                }));
            } else {
                return escalate(store, run);
            }
        }
        // building, or in_review not yet reviewed — no transition; the
        // directive will dispatch the worker or verifier.
        return Ok(None);
    }
    if run.all_tasks_done() {
        store.append_run_event(
            &run.spec_id,
            &run.run_id,
            Event::RunStateTransitioned {
                to: RunState::Verifying,
                snapshot: None,
            },
        )?;
        return Ok(Some(AppliedTransition {
            subject: "run".into(),
            from: "implementing".into(),
            to: "verifying".into(),
            snapshot: None,
        }));
    }
    Ok(None)
}

fn step_verifying(
    store: &Store,
    run: &RunProjection,
    _config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if run.run_review_rounds_completed == 0 {
        // Run-level review not done yet; directive dispatches the verifier.
        return Ok(None);
    }
    if run.all_requirements_resolved() {
        store.append_run_event(
            &run.spec_id,
            &run.run_id,
            Event::RunStateTransitioned {
                to: RunState::Verified,
                snapshot: None,
            },
        )?;
        return Ok(Some(AppliedTransition {
            subject: "run".into(),
            from: "verifying".into(),
            to: "verified".into(),
            snapshot: None,
        }));
    }
    // Unresolved after run-level review. Run-level repair (RT tasks) is M3;
    // for now the run escalates rather than looping.
    escalate(store, run)
}

/// Commit any in-flight diff as a labeled escalation snapshot and park the run.
fn escalate(store: &Store, run: &RunProjection) -> Result<Option<AppliedTransition>> {
    ensure_on_branch(store, run)?;
    let snapshot = if gitx::is_dirty(&store.git_root)? {
        Some(gitx::commit_all(
            &store.git_root,
            &format!("speccy: {} escalation snapshot", run.spec_ref),
        )?)
    } else {
        None
    };
    let from = run.state;
    store.append_run_event(
        &run.spec_id,
        &run.run_id,
        Event::RunStateTransitioned {
            to: RunState::Escalated,
            snapshot: snapshot.clone(),
        },
    )?;
    Ok(Some(AppliedTransition {
        subject: "run".into(),
        from: run_state_str(from).into(),
        to: "escalated".into(),
        snapshot,
    }))
}

fn task_snapshot(store: &Store, run: &RunProjection, task: &TaskState) -> Result<String> {
    ensure_on_branch(store, run)?;
    let msg = format!(
        "speccy: {} {} integrated (round {})",
        run.spec_ref, task.id, task.round
    );
    gitx::commit_all(&store.git_root, &msg)
}

fn ensure_on_branch(store: &Store, run: &RunProjection) -> Result<()> {
    if gitx::current_branch(&store.git_root)? != run.branch {
        gitx::checkout(&store.git_root, &run.branch)?;
    }
    Ok(())
}

/// The variable parts of a directive, computed before the single build.
struct Parts {
    action: DirectiveAction,
    subject: Subject,
    round: Option<Round>,
    packet_with: Option<String>,
    record_with: Option<String>,
    reason: String,
    gate_answers: Option<Vec<GateAnswer>>,
}

fn compute_directive(
    run: &RunProjection,
    config: &ProjectConfig,
    applied: Vec<AppliedTransition>,
    agent: &str,
) -> Directive {
    let parts = match run.state {
        RunState::Implementing => implementing_parts(run, config),
        RunState::Verifying => Parts {
            action: DirectiveAction::DispatchVerifier,
            subject: Subject {
                requirements: Some(run.requirements.keys().cloned().collect()),
                personas: Some(config.roster_for(run.risk)),
                ..Default::default()
            },
            round: Some(Round {
                current: run.run_review_rounds_completed + 1,
                max: config.caps.run_review_rounds,
                scope: RoundScope::Run,
            }),
            packet_with: Some("packet verification".into()),
            record_with: Some("requirement set-status".into()),
            reason: "all tasks integrated; run-level integration and drift review required".into(),
            gate_answers: None,
        },
        RunState::Verified => Parts {
            action: DirectiveAction::AwaitHumanGate,
            subject: Subject {
                gate: Some(Gate::ShipDecision),
                ..Default::default()
            },
            round: None,
            packet_with: Some("packet review".into()),
            record_with: Some("run record-ship".into()),
            reason: ship_reason(run),
            gate_answers: Some(ship_gate_answers()),
        },
        RunState::Escalated => {
            let failing: Vec<String> = run
                .requirements
                .iter()
                .filter(|(_, r)| !r.status.is_resolved())
                .map(|(id, _)| id.clone())
                .collect();
            Parts {
                action: DirectiveAction::AwaitHumanGate,
                subject: Subject {
                    gate: Some(Gate::Escalation),
                    requirements: Some(failing),
                    ..Default::default()
                },
                round: None,
                packet_with: Some("packet escalation".into()),
                record_with: Some("run record-decision".into()),
                reason:
                    "autonomous progress stopped; a requirement could not be satisfied or proven"
                        .into(),
                gate_answers: Some(escalation_gate_answers()),
            }
        }
        RunState::Submitted => {
            halt_parts("submitted awaiting external merge; record it with speccy accept")
        }
        RunState::Landed => halt_parts("run landed"),
        RunState::Cancelled => halt_parts("run cancelled"),
    };
    Directive {
        run_state: run.state,
        action: parts.action,
        subject: parts.subject,
        round: parts.round,
        packet_with: parts.packet_with,
        record_with: parts.record_with,
        reason: parts.reason,
        applied_transitions: applied,
        gate_answers: parts.gate_answers,
        resume: None,
        lease: Lease::issue(agent),
    }
}

fn implementing_parts(run: &RunProjection, config: &ProjectConfig) -> Parts {
    let task_round = |t: &TaskState| Round {
        current: t.round,
        max: config.caps.task_repair_rounds,
        scope: RoundScope::Task,
    };
    if let Some(active) = run.active_task() {
        match active.status {
            TaskStatus::Building => {
                let reason = if active.round > 1 {
                    format!("{} building in round {} (repair)", active.id, active.round)
                } else {
                    format!(
                        "{} is building with no recorded handoff for round 1",
                        active.id
                    )
                };
                Parts {
                    action: DirectiveAction::DispatchWorker,
                    subject: Subject {
                        task: Some(active.id.clone()),
                        requirements: Some(active.requirements.clone()),
                        ..Default::default()
                    },
                    round: Some(task_round(active)),
                    packet_with: Some("packet task".into()),
                    record_with: Some("task record-handoff".into()),
                    reason,
                    gate_answers: None,
                }
            }
            TaskStatus::InReview => Parts {
                action: DirectiveAction::DispatchVerifier,
                subject: Subject {
                    task: Some(active.id.clone()),
                    requirements: Some(active.requirements.clone()),
                    personas: Some(config.roster_for(run.risk)),
                    ..Default::default()
                },
                round: Some(task_round(active)),
                packet_with: Some("packet verification".into()),
                record_with: Some("requirement set-status".into()),
                reason: format!(
                    "handoff recorded; {} in_review, verification required",
                    active.id
                ),
                gate_answers: None,
            },
            _ => halt_parts("no schedulable work"),
        }
    } else if let Some(next) = run.next_queued_task() {
        Parts {
            action: DirectiveAction::ClaimTask,
            subject: Subject {
                task: Some(next.id.clone()),
                ..Default::default()
            },
            round: None,
            packet_with: None,
            record_with: Some("task claim".into()),
            reason: format!("{} is the next queued task", next.id),
            gate_answers: None,
        }
    } else {
        halt_parts("no schedulable work")
    }
}

fn halt_parts(reason: &str) -> Parts {
    Parts {
        action: DirectiveAction::Halt,
        subject: Subject::default(),
        round: None,
        packet_with: None,
        record_with: None,
        reason: reason.to_string(),
        gate_answers: None,
    }
}

fn ship_reason(run: &RunProjection) -> String {
    let passed = run
        .requirements
        .values()
        .filter(|r| r.status == RequirementStatus::Passed)
        .count();
    let accepted = run
        .requirements
        .values()
        .filter(|r| {
            matches!(
                r.status,
                RequirementStatus::ReviewPassed | RequirementStatus::Waived
            )
        })
        .count();
    format!(
        "all requirements resolved ({passed} passed, {accepted} accepted risk); ship or send back"
    )
}

fn ship_gate_answers() -> Vec<GateAnswer> {
    vec![
        GateAnswer {
            type_: "ship".into(),
            record_with: "run record-ship".into(),
        },
        GateAnswer {
            type_: "rework".into(),
            record_with: "run record-decision".into(),
        },
        GateAnswer {
            type_: "amend".into(),
            record_with: "spec record-decision".into(),
        },
        GateAnswer {
            type_: "cancel".into(),
            record_with: "run record-decision".into(),
        },
    ]
}

fn escalation_gate_answers() -> Vec<GateAnswer> {
    vec![
        GateAnswer {
            type_: "amend".into(),
            record_with: "spec record-decision".into(),
        },
        GateAnswer {
            type_: "provide_setup".into(),
            record_with: "run record-decision".into(),
        },
        GateAnswer {
            type_: "waive".into(),
            record_with: "run record-decision".into(),
        },
        GateAnswer {
            type_: "cancel".into(),
            record_with: "run record-decision".into(),
        },
    ]
}

fn run_state_str(s: RunState) -> &'static str {
    match s {
        RunState::Implementing => "implementing",
        RunState::Verifying => "verifying",
        RunState::Verified => "verified",
        RunState::Submitted => "submitted",
        RunState::Landed => "landed",
        RunState::Escalated => "escalated",
        RunState::Cancelled => "cancelled",
    }
}
