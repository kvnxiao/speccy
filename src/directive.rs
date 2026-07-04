//! `run next` — the single loop-driving operation and the single mutation
//! point for derived state (DESIGN § Deterministic Loop Driving).
//!
//! Each call: apply every derived transition that has no recording operation
//! (task integrate/repair, run implementing→verifying→verified, escalation),
//! creating snapshot commits as it goes, then compute the single next
//! directive. Sequencing, round counting, and gate detection are controller
//! decisions, never prose decisions.

use serde::Serialize;

use crate::config::ProjectConfig;
use crate::error::{Result, SpeccyError};
use crate::event::{Event, FindingRecord};
use crate::gitx;
use crate::ids;
use crate::lease::LeaseState;
use crate::model::{DirectiveAction, Gate, RequirementStatus, RoundScope, RunState, TaskStatus};
use crate::projection::{RunProjection, TaskState};
use crate::provenance;
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
    pub lease: LeaseState,
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

/// Run one `run next` cycle. In order: manage the run lease (contention,
/// expiry-clearing with resume attribution, renewal); detect out-of-band
/// commits; run the provenance scan; apply derived transitions; compute the
/// single next directive from settled state.
pub fn run_next(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    agent: &str,
) -> Result<serde_json::Value> {
    let run0 = store.run_projection(spec_id, run_id)?;

    // --- lease management ---
    let (lease, resume) = manage_lease(store, spec_id, run_id, agent, &run0)?;

    let config = ProjectConfig::load(&store.workspace_root)?;
    let mut applied = Vec::new();

    // Guards that park the run before the loop runs: an out-of-band commit or a
    // resource cap. Both fail closed to an escalated policy gate.
    if let Some(t) = detect_out_of_band(store, spec_id, run_id, &run0)? {
        applied.push(t);
    } else if let Some(t) = detect_resource_caps(store, spec_id, run_id, &run0, &config)? {
        applied.push(t);
    } else {
        // --- provenance scan over the current diff (records blocking findings) ---
        run_provenance_scan(store, spec_id, run_id, &run0)?;
        // --- derived transitions ---
        applied.extend(advance(store, spec_id, run_id)?);
    }

    let run = store.run_projection(spec_id, run_id)?;
    let directive = compute_directive(&run, &config, applied, lease, resume);
    serde_json::to_value(&directive)
        .map_err(|e| SpeccyError::io(format!("failed to serialize directive: {e}")))
}

/// Handle lease contention, expiry-clearing, and renewal. Returns the live
/// lease to embed plus an optional `resume` block when an expired lease was
/// cleared.
fn manage_lease(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    agent: &str,
    run: &RunProjection,
) -> Result<(LeaseState, Option<serde_json::Value>)> {
    let now = jiff::Timestamp::now();
    let current = store.read_lease(spec_id, run_id)?;
    let (lease, resume) = match current {
        Some(l) if !l.is_expired(now) => {
            if l.agent == agent {
                (l.renewed(), None)
            } else {
                return Err(SpeccyError::lease_held(format!(
                    "run lease held by {} until {}",
                    l.agent, l.expires_at
                )));
            }
        }
        Some(l) => {
            // Expired: clear it and report resume attribution.
            let resume = resume_attribution(store, run, &l.agent);
            (LeaseState::issue(agent), Some(resume))
        }
        None => (LeaseState::issue(agent), None),
    };
    store.write_lease(spec_id, run_id, &lease)?;
    Ok((lease, resume))
}

/// Summarize what resume will fold into the in-flight task after clearing a
/// dead session's lease (SCHEMAS § Directive `resume`).
fn resume_attribution(
    store: &Store,
    run: &RunProjection,
    cleared_agent: &str,
) -> serde_json::Value {
    let dirty_diff = run
        .active_task()
        .and_then(|t| t.baseline_commit.as_ref())
        .and_then(|base| {
            gitx::worktree_stat(&store.git_root, base)
                .ok()
                .filter(|d| d.files > 0)
                .map(|d| {
                    serde_json::json!({
                        "files": d.files,
                        "insertions": d.insertions,
                        "deletions": d.deletions,
                        "vs": base,
                        "attributed_to": run.active_task().map(|t| t.id.clone()),
                    })
                })
        });
    serde_json::json!({ "cleared_lease": cleared_agent, "dirty_diff": dirty_diff })
}

/// Detect an out-of-band commit: while a run is active, HEAD must match the
/// last recorded snapshot (or the base before any snapshot). A mismatch means
/// a human or another tool committed; park the run at an escalated policy gate.
fn detect_out_of_band(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    run: &RunProjection,
) -> Result<Option<AppliedTransition>> {
    if !matches!(run.state, RunState::Implementing | RunState::Verifying) {
        return Ok(None);
    }
    let head = gitx::head(&store.git_root)?;
    let expected = run.head_expectation();
    if head == expected {
        return Ok(None);
    }
    store.append_run_event(
        spec_id,
        run_id,
        Event::RunStateTransitioned {
            to: RunState::Escalated,
            snapshot: None,
        },
    )?;
    Ok(Some(AppliedTransition {
        subject: "run".into(),
        from: run.state.as_str().into(),
        to: "escalated".into(),
        snapshot: None,
    }))
}

/// Optional resource caps (task count, wall-clock). Hitting one parks the run
/// at an escalated policy gate; Speccy cannot meter tokens (DESIGN §
/// Capability Escalation and Give-Up Policy).
fn detect_resource_caps(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if !matches!(run.state, RunState::Implementing | RunState::Verifying) {
        return Ok(None);
    }
    let mut reason = None;
    if let Some(max) = config.caps.max_tasks {
        if run.tasks.len() as u32 > max {
            reason = Some(format!("task count {} exceeds cap {max}", run.tasks.len()));
        }
    }
    if reason.is_none() {
        if let (Some(max_min), Some(started)) =
            (config.caps.max_run_wall_clock_minutes, run.started_at)
        {
            let elapsed_min = (jiff::Timestamp::now().as_second() - started.as_second()) / 60;
            if elapsed_min > max_min as i64 {
                reason = Some(format!("wall-clock {elapsed_min}m exceeds cap {max_min}m"));
            }
        }
    }
    if reason.is_none() {
        return Ok(None);
    }
    let from = run.state;
    store.append_run_event(
        spec_id,
        run_id,
        Event::RunStateTransitioned {
            to: RunState::Escalated,
            snapshot: None,
        },
    )?;
    Ok(Some(AppliedTransition {
        subject: "run".into(),
        from: from.as_str().into(),
        to: "escalated".into(),
        snapshot: None,
    }))
}

/// Run the deterministic provenance scan over the current diff and record any
/// hits as blocking findings (once per round, so repeated `run next` calls do
/// not duplicate them). Findings feed the normal repair round.
fn run_provenance_scan(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    run: &RunProjection,
) -> Result<()> {
    let config = ProjectConfig::load(&store.workspace_root)?;
    // Choose the diff scope: an in-review task's diff, or the integrated
    // run diff during verifying.
    let (baseline, task_id, guard_seq) = if let Some(t) = run
        .active_task()
        .filter(|t| t.status == TaskStatus::InReview)
    {
        (
            t.baseline_commit.clone(),
            Some(t.id.clone()),
            t.last_handoff_seq,
        )
    } else if run.state == RunState::Verifying && !run.run_review_reviewed() {
        (
            Some(run.base_commit.clone()),
            None,
            run.verifying_entered_seq(),
        )
    } else {
        return Ok(());
    };
    let Some(baseline) = baseline else {
        return Ok(());
    };

    // Already scanned this round? (a provenance finding recorded after the guard)
    if run.provenance_scanned_after(guard_seq, task_id.as_deref()) {
        return Ok(());
    }

    let diff = gitx::worktree_diff(&store.git_root, &baseline).unwrap_or_default();
    let terms = provenance::deny_terms(
        &run.spec_ref,
        spec_id,
        run_id,
        run.requirements.keys().cloned(),
        &config.provenance.extra_terms,
    );
    for hit in provenance::scan_diff(&diff, &terms) {
        let finding = FindingRecord {
            id: ids::short_id("fd"),
            requirement: None,
            task: task_id.clone(),
            persona: None,
            severity: "blocking".into(),
            note: format!(
                "{}:{} references \"{}\" — provenance deny-list hit",
                hit.file, hit.line, hit.term
            ),
            recorded_by: "controller:provenance-scan".into(),
        };
        store.append_run_event(spec_id, run_id, Event::FindingRecorded { finding })?;
    }
    Ok(())
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
            } else if run.task_has_blocked_requirement(active) {
                // A blocked requirement cannot be repaired; escalate as a
                // human/policy gate without consuming a repair round.
                return escalate(store, run);
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
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if !run.run_review_reviewed() {
        // Run-level review for this round not recorded yet; the directive
        // dispatches the verifier.
        return Ok(None);
    }
    // A critical run with accepted risk parks at the confirmation gate before
    // verifying can complete (the directive surfaces the gate).
    if run.needs_accepted_risk_confirmation() {
        return Ok(None);
    }
    if run.all_requirements_resolved() && run.run_blocking_findings().is_empty() {
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
    // The run-level review left work unresolved. A blocked requirement cannot
    // be repaired, so it escalates directly as a policy gate.
    if run.has_blocked_requirement() {
        return escalate(store, run);
    }
    // Rounds remaining → spawn a run-level repair task (RT<n>) linked to the
    // failing requirements and loop back through `implementing` (DESIGN §
    // Capability Escalation and Give-Up Policy). Otherwise the cap is
    // exhausted and the run gives up.
    if run.run_review_round < config.caps.run_review_rounds {
        return Ok(Some(spawn_run_repair(store, run)?));
    }
    escalate(store, run)
}

/// Append a dynamic run-level repair task (`RT<n>`) linked to the failing
/// requirements and return the run to `implementing` so the normal task loop
/// re-proves them, then re-enters `verifying` for the next review round.
fn spawn_run_repair(store: &Store, run: &RunProjection) -> Result<AppliedTransition> {
    let rt = next_rt_id(run);
    let failing = run.failing_requirements();
    let seed = if failing.is_empty() {
        Some("resolve the run-level review findings".to_string())
    } else {
        Some(format!("re-prove run-level failures: {}", failing.join(", ")))
    };
    store.append_run_event(
        &run.spec_id,
        &run.run_id,
        Event::TaskAppended {
            task: crate::event::TaskInit {
                id: rt.clone(),
                title: Some(format!("Run-level repair (round {})", run.run_review_round + 1)),
                requirements: failing,
                constraints: Vec::new(),
            },
            seed_feedback: seed,
        },
    )?;
    store.append_run_event(
        &run.spec_id,
        &run.run_id,
        Event::RunStateTransitioned {
            to: RunState::Implementing,
            snapshot: None,
        },
    )?;
    Ok(AppliedTransition {
        subject: "run".into(),
        from: "verifying".into(),
        to: "implementing".into(),
        snapshot: None,
    })
}

/// Next `RT<n>` id for the run (shared with the ship-gate rework path).
pub(crate) fn next_rt_id(run: &RunProjection) -> String {
    let n = run.tasks.iter().filter(|t| t.id.starts_with("RT")).count() + 1;
    format!("RT{n}")
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
        from: from.as_str().into(),
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
    lease: LeaseState,
    resume: Option<serde_json::Value>,
) -> Directive {
    let parts = match run.state {
        RunState::Implementing => implementing_parts(run, config),
        RunState::Verifying
            if run.run_review_reviewed() && run.needs_accepted_risk_confirmation() =>
        {
            Parts {
                action: DirectiveAction::AwaitHumanGate,
                subject: Subject {
                    gate: Some(Gate::AcceptedRiskConfirmation),
                    requirements: Some(
                        run.requirements
                            .iter()
                            .filter(|(_, r)| {
                                matches!(
                                    r.status,
                                    RequirementStatus::ReviewPassed | RequirementStatus::Waived
                                )
                            })
                            .map(|(id, _)| id.clone())
                            .collect(),
                    ),
                    ..Default::default()
                },
                round: None,
                packet_with: Some("packet review".into()),
                record_with: Some("run record-decision".into()),
                reason: "critical spec: confirm accepted risk before verifying completes".into(),
                gate_answers: Some(vec![
                    GateAnswer {
                        type_: "confirm_accepted_risk".into(),
                        record_with: "run record-decision".into(),
                    },
                    GateAnswer {
                        type_: "cancel".into(),
                        record_with: "run record-decision".into(),
                    },
                ]),
            }
        }
        RunState::Verifying => Parts {
            action: DirectiveAction::DispatchVerifier,
            subject: Subject {
                requirements: Some(run.requirements.keys().cloned().collect()),
                personas: Some(config.roster_for(run.risk)),
                ..Default::default()
            },
            round: Some(Round {
                current: run.run_review_round,
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
        resume,
        lease,
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

