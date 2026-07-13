//! `run next` — the single loop-driving operation and the single mutation
//! point for derived state (DESIGN § Deterministic Loop Driving).
//!
//! Each call: apply every derived transition that has no recording operation
//! (task integrate/repair, run implementing→verifying→verified, escalation),
//! creating snapshot commits as it goes, then compute the single next
//! directive. Sequencing, round counting, and gate detection are controller
//! decisions, never prose decisions.

use crate::config::ProjectConfig;
use crate::error::Result;
use crate::error::SpeccyError;
use crate::event::Event;
use crate::event::FindingRecord;
use crate::gitx;
use crate::ids;
use crate::lease::LeaseState;
use crate::model::DirectiveAction;
use crate::model::FindingSeverity;
use crate::model::Gate;
use crate::model::RequirementStatus;
use crate::model::RoundScope;
use crate::model::RunState;
use crate::model::TaskStatus;
use crate::projection::RunProjection;
use crate::projection::TaskState;
use crate::provenance;
use crate::store::Store;
use crate::store::StoreLockGuard;
use serde::Serialize;

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

/// The single next directive `run next` returns (SCHEMAS § Directive).
#[derive(Debug, Clone, Serialize)]
pub struct Directive {
    pub run_state: RunState,
    pub action: DirectiveAction,
    pub subject: Subject,
    // Serialized explicitly (as `null` when absent) to match SCHEMAS § Directive.
    pub round: Option<Round>,
    pub packet_with: Option<String>,
    pub record_with: Option<String>,
    pub reason: String,
    pub applied_transitions: Vec<AppliedTransition>,
    pub gate_answers: Option<Vec<GateAnswer>>,
    pub resume: Option<serde_json::Value>,
    pub lease: LeaseState,
}

/// What a directive acts on: a task, its requirements, a gate, and/or the
/// reviewer personas. Every field serializes explicitly (as `null` when
/// absent) to match SCHEMAS § Directive.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Subject {
    pub task: Option<String>,
    pub requirements: Option<Vec<String>>,
    pub gate: Option<Gate>,
    pub personas: Option<Vec<String>>,
}

/// The current repair/review round, its cap, and scope for a directive.
#[derive(Debug, Clone, Serialize)]
pub struct Round {
    pub current: u32,
    pub max: u32,
    pub scope: RoundScope,
}

/// One allowed human answer at a gate and the op that records it.
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
///
/// # Errors
///
/// Returns an error if the run lease is held by another agent, if the
/// workspace store or git operations fail, or if the resulting directive
/// cannot be serialized.
pub fn run_next(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    agent: &str,
) -> Result<serde_json::Value> {
    let config = ProjectConfig::load(&store.workspace_root)?;

    // The whole cycle runs under one store-lock hold: the opening projection
    // read, lease management, the derived-transition appends (and the git
    // snapshots they trigger), and the closing projection read. This stops a
    // concurrent `run next` from reading the same pre-transition state and
    // applying a derived transition twice (DESIGN § Storage Model).
    let (run, applied, lease, resume) = store.with_store_lock(|guard| {
        let run0 = store.run_projection(spec_id, run_id)?;

        // --- lease management ---
        let (lease, resume) = manage_lease(store, guard, spec_id, run_id, agent, &run0)?;

        let mut applied = Vec::new();

        // Guards that park the run before the loop runs: an out-of-band commit
        // or a resource cap. Both fail closed to an escalated policy gate and
        // append an event, so the settled state must be re-projected.
        let settled = if let Some(t) = detect_out_of_band(store, guard, spec_id, run_id, &run0)? {
            applied.push(t);
            store.run_projection(spec_id, run_id)?
        } else if let Some(t) = detect_resource_caps(store, guard, &run0, &config)? {
            applied.push(t);
            store.run_projection(spec_id, run_id)?
        } else {
            // Provenance scan over the current diff records blocking findings;
            // re-project only when it actually recorded some.
            let hits = run_provenance_scan(store, guard, spec_id, run_id, &run0, &config)?;
            let start = if hits == 0 {
                run0
            } else {
                store.run_projection(spec_id, run_id)?
            };
            // advance() starts from `start` and returns the settled projection,
            // so no separate final rebuild is needed.
            let (transitions, settled) = advance(store, guard, spec_id, run_id, start, &config)?;
            applied.extend(transitions);
            settled
        };

        Ok((settled, applied, lease, resume))
    })?;

    let directive = compute_directive(&run, &config, applied, lease, resume);
    serde_json::to_value(&directive)
        .map_err(|e| SpeccyError::io(format!("failed to serialize directive: {e}")))
}

/// Handle lease contention, expiry-clearing, and renewal. Returns the live
/// lease to embed plus an optional `resume` block when an expired lease was
/// cleared.
fn manage_lease(
    store: &Store,
    _guard: &StoreLockGuard,
    spec_id: &str,
    run_id: &str,
    agent: &str,
    run: &RunProjection,
) -> Result<(LeaseState, Option<serde_json::Value>)> {
    let now = jiff::Timestamp::now();
    match store.read_lease(spec_id, run_id)? {
        Some(l) if !l.is_expired(now) => {
            if l.agent != agent {
                return Err(SpeccyError::lease_held(format!(
                    "run lease held by {} until {}",
                    l.agent, l.expires_at
                )));
            }
            // Renewal debounce: skip the rewrite only for rapid back-to-back
            // calls — a lease renewed within the last tenth of its TTL. The
            // on-disk expiry then never trails real time by more than that, so
            // the worst-case dispatch window stays near the full TTL instead of
            // halving to ttl/2. The lease is derived state (DESIGN § Run Lease),
            // so a skipped renewal only risks an earlier expiry, never a torn
            // file.
            let ttl = crate::lease::ttl_seconds();
            let remaining = l.expires_at.as_second() - now.as_second();
            if remaining > ttl - ttl / 10 {
                return Ok((l, None));
            }
            let renewed = l.renewed();
            store.write_lease(spec_id, run_id, &renewed)?;
            Ok((renewed, None))
        }
        Some(l) => {
            // Expired: clear it and report resume attribution.
            let resume = resume_attribution(store, run, &l.agent);
            let lease = LeaseState::issue(agent);
            store.write_lease(spec_id, run_id, &lease)?;
            Ok((lease, Some(resume)))
        }
        None => {
            let lease = LeaseState::issue(agent);
            store.write_lease(spec_id, run_id, &lease)?;
            Ok((lease, None))
        }
    }
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
    guard: &StoreLockGuard,
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
    store.append_run_event_with(
        guard,
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
        to: RunState::Escalated.as_str().into(),
        snapshot: None,
    }))
}

/// Optional resource caps (task count, wall-clock). Hitting one parks the run
/// at an escalated policy gate; Speccy cannot meter tokens (DESIGN §
/// Capability Escalation and Give-Up Policy).
fn detect_resource_caps(
    store: &Store,
    guard: &StoreLockGuard,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if !matches!(run.state, RunState::Implementing | RunState::Verifying) {
        return Ok(None);
    }
    let mut reason = None;
    if let Some(max) = config.caps.max_tasks
        && u32::try_from(run.tasks.len()).unwrap_or(u32::MAX) > max
    {
        reason = Some(format!("task count {} exceeds cap {max}", run.tasks.len()));
    }
    if reason.is_none()
        && let Some(max_min) = config.caps.max_run_wall_clock_minutes
    {
        // Active time only — time parked at a human gate does not count.
        let active_min = run.active_seconds_at(jiff::Timestamp::now()) / 60;
        if active_min > i64::from(max_min) {
            reason = Some(format!("active time {active_min}m exceeds cap {max_min}m"));
        }
    }
    if reason.is_none() {
        return Ok(None);
    }
    // A resource cap commits the in-flight diff as a labeled snapshot, like the
    // other give-up escalations (DESIGN § Deterministic Loop Driving); only the
    // out-of-band escalation takes no snapshot.
    escalate(store, guard, run)
}

/// Run the deterministic provenance scan over the current diff and record any
/// hits as blocking findings (once per round, so repeated `run next` calls do
/// not duplicate them). Findings feed the normal repair round.
fn run_provenance_scan(
    store: &Store,
    guard: &StoreLockGuard,
    spec_id: &str,
    run_id: &str,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<usize> {
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
        return Ok(0);
    };
    let Some(baseline) = baseline else {
        return Ok(0);
    };

    // Already scanned this round? (a provenance finding recorded after the guard)
    if run.provenance_scanned_after(guard_seq, task_id.as_deref()) {
        return Ok(0);
    }

    let diff = gitx::worktree_diff(&store.git_root, &baseline).unwrap_or_default();
    let terms = provenance::deny_terms(
        &run.spec_ref,
        spec_id,
        run_id,
        run.requirements.keys().cloned(),
        &config.provenance.extra_terms,
    );
    let mut recorded = 0;
    for hit in provenance::scan_diff(&diff, &terms) {
        let finding = FindingRecord {
            id: ids::short_id("fd"),
            requirement: None,
            task: task_id.clone(),
            persona: None,
            severity: FindingSeverity::Blocking,
            note: format!(
                "{}:{} references \"{}\" — provenance deny-list hit",
                hit.file, hit.line, hit.term
            ),
            recorded_by: "controller:provenance-scan".into(),
        };
        store.append_run_event_with(guard, spec_id, run_id, Event::FindingRecorded { finding })?;
        recorded += 1;
    }
    Ok(recorded)
}

/// Apply derived transitions to a fixpoint starting from `initial`. Returns the
/// transitions applied this call and the settled projection (so the caller
/// needs no separate final rebuild). `config` is loaded once by the caller
/// rather than re-read every iteration.
fn advance(
    store: &Store,
    guard: &StoreLockGuard,
    spec_id: &str,
    run_id: &str,
    initial: RunProjection,
    config: &ProjectConfig,
) -> Result<(Vec<AppliedTransition>, RunProjection)> {
    let mut applied = Vec::new();
    let mut run = initial;
    while let Some(t) = step(store, guard, &run, config)? {
        applied.push(t);
        run = store.run_projection(spec_id, run_id)?;
    }
    Ok((applied, run))
}

/// Apply at most one derived transition. `None` means state is settled.
fn step(
    store: &Store,
    guard: &StoreLockGuard,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    match run.state {
        RunState::Implementing => step_implementing(store, guard, run, config),
        RunState::Verifying => step_verifying(store, guard, run, config),
        _ => Ok(None),
    }
}

fn step_implementing(
    store: &Store,
    guard: &StoreLockGuard,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if let Some(active) = run.active_task() {
        if active.status == TaskStatus::InReview && run.task_reviewed_this_round(active) {
            let blocking = run.task_blocking_findings_this_round(active);
            let resolved = run.task_requirements_resolved(active) && blocking.is_empty();
            if resolved {
                let sha = task_snapshot(store, run, active)?;
                store.append_run_event_with(
                    guard,
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
                    from: TaskStatus::InReview.as_str().into(),
                    to: TaskStatus::Integrated.as_str().into(),
                    snapshot: Some(sha),
                }));
            } else if run.task_has_blocked_requirement(active) {
                // A blocked requirement cannot be repaired; escalate as a
                // human/policy gate without consuming a repair round.
                return escalate(store, guard, run);
            } else if active.round < config.caps.task_repair_rounds {
                store.append_run_event_with(
                    guard,
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
                    from: TaskStatus::InReview.as_str().into(),
                    to: TaskStatus::Building.as_str().into(),
                    snapshot: None,
                }));
            }
            return escalate(store, guard, run);
        }
        // building, or in_review not yet reviewed — no transition; the
        // directive will dispatch the worker or verifier.
        return Ok(None);
    }
    if run.all_tasks_done() {
        store.append_run_event_with(
            guard,
            &run.spec_id,
            &run.run_id,
            Event::RunStateTransitioned {
                to: RunState::Verifying,
                snapshot: None,
            },
        )?;
        return Ok(Some(AppliedTransition {
            subject: "run".into(),
            from: RunState::Implementing.as_str().into(),
            to: RunState::Verifying.as_str().into(),
            snapshot: None,
        }));
    }
    Ok(None)
}

fn step_verifying(
    store: &Store,
    guard: &StoreLockGuard,
    run: &RunProjection,
    config: &ProjectConfig,
) -> Result<Option<AppliedTransition>> {
    if !run.run_review_reviewed() {
        // Run-level review for this round not recorded yet; the directive
        // dispatches the verifier.
        return Ok(None);
    }
    if run.all_requirements_resolved() && run.run_blocking_findings().is_empty() {
        // Verification is otherwise complete. A critical run with accepted risk
        // now parks at the confirmation gate (the directive surfaces it); this
        // is the only place the gate fires, so an unresolved requirement below
        // reaches repair/escalation instead of pre-empting to the gate.
        if run.needs_accepted_risk_confirmation() {
            return Ok(None);
        }
        store.append_run_event_with(
            guard,
            &run.spec_id,
            &run.run_id,
            Event::RunStateTransitioned {
                to: RunState::Verified,
                snapshot: None,
            },
        )?;
        return Ok(Some(AppliedTransition {
            subject: "run".into(),
            from: RunState::Verifying.as_str().into(),
            to: RunState::Verified.as_str().into(),
            snapshot: None,
        }));
    }
    // The run-level review left work unresolved. A blocked requirement cannot
    // be repaired, so it escalates directly as a policy gate.
    if run.has_blocked_requirement() {
        return escalate(store, guard, run);
    }
    // Rounds remaining → spawn a run-level repair task (RT<n>) linked to the
    // failing requirements and loop back through `implementing` (DESIGN §
    // Capability Escalation and Give-Up Policy). Otherwise the cap is
    // exhausted and the run gives up.
    if run.run_review_round < config.caps.run_review_rounds {
        return Ok(Some(spawn_run_repair(store, guard, run)?));
    }
    escalate(store, guard, run)
}

/// Append a dynamic run-level repair task (`RT<n>`) linked to the failing
/// requirements and return the run to `implementing` so the normal task loop
/// re-proves them, then re-enters `verifying` for the next review round.
fn spawn_run_repair(
    store: &Store,
    guard: &StoreLockGuard,
    run: &RunProjection,
) -> Result<AppliedTransition> {
    let rt = next_rt_id(run);
    let failing = run.failing_requirements();
    let seed = if failing.is_empty() {
        Some("resolve the run-level review findings".to_string())
    } else {
        Some(format!(
            "re-prove run-level failures: {}",
            failing.join(", ")
        ))
    };
    store.append_run_event_with(
        guard,
        &run.spec_id,
        &run.run_id,
        Event::TaskAppended {
            task: crate::event::TaskInit {
                id: rt.clone(),
                title: Some(format!(
                    "Run-level repair (round {})",
                    run.run_review_round + 1
                )),
                requirements: failing,
                constraints: Vec::new(),
            },
            seed_feedback: seed,
        },
    )?;
    store.append_run_event_with(
        guard,
        &run.spec_id,
        &run.run_id,
        Event::RunStateTransitioned {
            to: RunState::Implementing,
            snapshot: None,
        },
    )?;
    Ok(AppliedTransition {
        subject: "run".into(),
        from: RunState::Verifying.as_str().into(),
        to: RunState::Implementing.as_str().into(),
        snapshot: None,
    })
}

/// Next `RT<n>` id for the run (shared with the ship-gate rework path).
#[must_use = "the generated id must be used to append the new repair task"]
pub(crate) fn next_rt_id(run: &RunProjection) -> String {
    let n = run.tasks.iter().filter(|t| t.id.starts_with("RT")).count() + 1;
    format!("RT{n}")
}

/// Commit any in-flight diff as a labeled escalation snapshot and park the run.
pub(crate) fn escalate(
    store: &Store,
    guard: &StoreLockGuard,
    run: &RunProjection,
) -> Result<Option<AppliedTransition>> {
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
    store.append_run_event_with(
        guard,
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
        to: RunState::Escalated.as_str().into(),
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
        RunState::Verifying if run.at_accepted_risk_gate() => accepted_risk_confirmation_parts(run),
        RunState::Verifying => verifying_parts(run, config),
        RunState::Verified => verified_parts(run),
        RunState::Escalated => escalated_parts(run),
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

/// `verifying`, run-level review done, critical spec still needs risk
/// confirmation before verifying can complete.
fn accepted_risk_confirmation_parts(run: &RunProjection) -> Parts {
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

/// `verifying`, run-level review still outstanding this round.
fn verifying_parts(run: &RunProjection, config: &ProjectConfig) -> Parts {
    Parts {
        action: DirectiveAction::DispatchVerifier,
        subject: Subject {
            requirements: Some(status_update_requirements(run)),
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
    }
}

/// `verified`, awaiting the ship-decision gate.
fn verified_parts(run: &RunProjection) -> Parts {
    Parts {
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
    }
}

/// `escalated`, awaiting the escalation policy gate.
fn escalated_parts(run: &RunProjection) -> Parts {
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
        reason: "autonomous progress stopped; a requirement could not be satisfied or proven"
            .into(),
        gate_answers: Some(escalation_gate_answers()),
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

fn status_update_requirements(run: &RunProjection) -> Vec<String> {
    run.requirements
        .iter()
        .filter(|(_, r)| r.status != RequirementStatus::Waived)
        .map(|(id, _)| id.clone())
        .collect()
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
