//! Lease-gated run mutations: the locked mutation service (DESIGN § Storage
//! Model, § Run Lease and Concurrent Writers).
//!
//! Every lease-gated controller operation commits through one store-lock hold
//! that covers the projection replay, the live-lease check, the transition
//! validation, and every event append the operation records. Two processes
//! can therefore never validate the same pre-state and both commit an
//! incompatible transition, and a lease cleared or reissued between check and
//! write is caught inside the hold — a stale token appends nothing.
//! Lease-free reviewer findings and non-command evidence stay additive and
//! are recorded outside this module.

use crate::config::ProjectConfig;
use crate::directive;
use crate::error::Finding;
use crate::error::Result;
use crate::error::SpeccyError;
use crate::event::Event;
use crate::event::Handoff;
use crate::event::RequirementUpdate;
use crate::event::RunDecisionRecord;
use crate::gitx;
use crate::ids;
use crate::model::ChangeRef;
use crate::model::RequirementStatus;
use crate::model::RunDecisionKind;
use crate::model::RunState;
use crate::model::TaskStatus;
use crate::projection::RunProjection;
use crate::store::Store;
use crate::store::StoreLockGuard;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

/// A run-scoped mutation context, alive only while the store lock is held.
struct RunTxn<'a> {
    store: &'a Store,
    guard: &'a StoreLockGuard,
    spec_id: &'a str,
    run_id: &'a str,
}

impl RunTxn<'_> {
    fn append(&self, event: Event) -> Result<()> {
        self.store
            .append_run_event_with(self.guard, self.spec_id, self.run_id, event)
    }

    /// Re-replay the projection mid-mutation (still under the lock), for a
    /// step whose validation depends on events appended earlier in the same
    /// mutation.
    fn reload(&self) -> Result<RunProjection> {
        self.store.run_projection(self.spec_id, self.run_id)
    }
}

/// The locked mutation service: replay the projection, verify the live
/// lease, then run `f` (validate + append) — all under one store-lock hold.
fn locked<T>(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    f: impl FnOnce(&RunTxn<'_>, &RunProjection) -> Result<T>,
) -> Result<T> {
    let (spec_id, _) = store.find_run(run_id)?;
    store.with_store_lock(|guard| {
        let run = store.run_projection(&spec_id, run_id)?;
        store.verify_lease(&spec_id, run_id, lease)?;
        let txn = RunTxn {
            store,
            guard,
            spec_id: &spec_id,
            run_id,
        };
        f(&txn, &run)
    })
}

// --------------------------------------------------------------------------
// Task mutations
// --------------------------------------------------------------------------

/// `task claim` — claim a queued task, pinning its baseline commit.
///
/// # Errors
///
/// Returns an error if the run or task does not exist, the lease token is
/// not live, the task is not queued, or the store or git operations fail.
pub fn claim_task(
    store: &Store,
    run_id: &str,
    task_id: &str,
    agent: &str,
    lease: &str,
) -> Result<Value> {
    locked(store, run_id, Some(lease), |txn, run| {
        let task = run
            .task(task_id)
            .ok_or_else(|| SpeccyError::not_found(format!("no task {task_id} in run {run_id}")))?;
        if task.status != TaskStatus::Queued {
            return Err(SpeccyError::invalid_transition(format!(
                "task {task_id} is {:?}, not queued",
                task.status
            )));
        }
        let baseline_commit = gitx::head(&store.git_root)?;
        txn.append(Event::TaskClaimed {
            task: task_id.to_string(),
            agent: agent.to_string(),
            baseline_commit: baseline_commit.clone(),
        })?;
        Ok(json!({
            "task": task_id,
            "status": "building",
            "round": 1,
            "baseline_commit": baseline_commit,
        }))
    })
}

/// `task record-handoff` — record a worker handoff for the task's current
/// round, moving it to `in_review`.
///
/// # Errors
///
/// Returns an error if the payload is incomplete, the run or task does not
/// exist, the lease token is not live, the round does not match, the task is
/// not building, or the store operations fail.
pub fn record_handoff(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    handoff: Handoff,
) -> Result<Value> {
    if handoff.summary.trim().is_empty() {
        return Err(
            SpeccyError::validation("handoff.summary is required").with_details(vec![Finding::at(
                "missing_field",
                "summary",
                "summary is required",
            )]),
        );
    }
    locked(store, run_id, lease, |txn, run| {
        let task = run.task(&handoff.task).ok_or_else(|| {
            SpeccyError::not_found(format!("no task {} in run {run_id}", handoff.task))
        })?;
        let expected_round = task.round.max(1);
        if handoff.round != expected_round {
            return Err(SpeccyError::validation(format!(
                "handoff.round {} does not match current task round {}",
                handoff.round, expected_round
            )));
        }
        if task.status != TaskStatus::Building {
            return Err(SpeccyError::invalid_transition(format!(
                "task {} is {:?}, not building; cannot record a handoff",
                handoff.task, task.status
            )));
        }
        let handoff_id = ids::short_id("ho");
        let task_id = handoff.task.clone();
        txn.append(Event::HandoffRecorded {
            handoff_id: handoff_id.clone(),
            task: task_id.clone(),
            round: expected_round,
            handoff,
        })?;
        Ok(json!({ "task": task_id, "status": "in_review", "handoff_id": handoff_id }))
    })
}

// --------------------------------------------------------------------------
// Requirement status
// --------------------------------------------------------------------------

/// `requirement set-status` — apply one or more requirement status
/// transitions after validating each update's prerequisites.
///
/// # Errors
///
/// Returns an error if the run does not exist, the lease token is not live,
/// any update fails its prerequisites, or the store operations fail.
pub fn set_requirement_status(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    updates: Vec<RequirementUpdate>,
) -> Result<Value> {
    locked(store, run_id, lease, |txn, run| {
        // Validate each update's prerequisites before recording anything.
        for u in &updates {
            validate_status_update(u, run)?;
        }
        let echo: Vec<Value> = updates
            .iter()
            .map(|u| json!({ "requirement": u.requirement, "status": u.status }))
            .collect();
        txn.append(Event::RequirementStatusSet { updates })?;
        Ok(json!({ "updated": echo }))
    })
}

/// Status prerequisites (DESIGN § Requirement Resolution Rules).
fn validate_status_update(u: &RequirementUpdate, run: &RunProjection) -> Result<()> {
    let current = run
        .requirements
        .get(&u.requirement)
        .ok_or_else(|| SpeccyError::not_found(format!("no requirement {}", u.requirement)))?;
    if current.status == RequirementStatus::Waived {
        return Err(SpeccyError::invalid_transition(format!(
            "{} is waived; waived is terminal for this run",
            u.requirement
        )));
    }
    validate_evidence_ids(run, u)?;
    validate_finding_ids(run, u)?;

    match u.status {
        RequirementStatus::Waived => Err(SpeccyError::validation(format!(
            "waived is gate-only; set it through run record-decision, not requirement set-status ({})",
            u.requirement
        ))),
        RequirementStatus::Passed => {
            if u.evidence.is_empty() {
                Err(SpeccyError::validation(format!(
                    "passed requires at least one recorded evidence artifact for {}",
                    u.requirement
                )))
            } else {
                Ok(())
            }
        }
        RequirementStatus::ReviewPassed => {
            if u.evidence.is_empty() {
                return Err(SpeccyError::validation(format!(
                    "review_passed requires at least one recorded evidence artifact for {}",
                    u.requirement
                )));
            }
            if matches!(
                run.risk,
                crate::model::RiskTier::High | crate::model::RiskTier::Critical
            ) && u.residual_risk.as_deref().map_or("", str::trim).is_empty()
            {
                return Err(SpeccyError::validation(format!(
                    "review_passed at {:?} requires a residual_risk note for {}",
                    run.risk, u.requirement
                )));
            }
            Ok(())
        }
        RequirementStatus::Failed => {
            if u.evidence.is_empty() && u.findings.is_empty() {
                Err(SpeccyError::validation(format!(
                    "failed requires at least one evidence artifact or finding for {}",
                    u.requirement
                )))
            } else {
                Ok(())
            }
        }
        RequirementStatus::Blocked => {
            if u.note.as_deref().map_or("", str::trim).is_empty() {
                Err(SpeccyError::validation(format!(
                    "blocked requires a note naming what is missing for {}",
                    u.requirement
                )))
            } else {
                Ok(())
            }
        }
        RequirementStatus::Pending => Err(SpeccyError::validation(
            "pending is the initial status and is never re-entered",
        )),
    }
}

fn validate_evidence_ids(run: &RunProjection, u: &RequirementUpdate) -> Result<()> {
    for id in &u.evidence {
        let record = run
            .evidence
            .iter()
            .find(|e| &e.id == id)
            .ok_or_else(|| SpeccyError::not_found(format!("no evidence artifact {id}")))?;
        if record.requirement != u.requirement {
            return Err(SpeccyError::validation(format!(
                "evidence artifact {id} belongs to {}, not {}",
                record.requirement, u.requirement
            )));
        }
    }
    Ok(())
}

fn validate_finding_ids(run: &RunProjection, u: &RequirementUpdate) -> Result<()> {
    for id in &u.findings {
        let record = run
            .findings
            .iter()
            .map(|(_, f)| f)
            .find(|f| &f.id == id)
            .ok_or_else(|| SpeccyError::not_found(format!("no finding {id}")))?;
        if let Some(requirement) = &record.requirement
            && requirement != &u.requirement
        {
            return Err(SpeccyError::validation(format!(
                "finding {id} belongs to {requirement}, not {}",
                u.requirement
            )));
        }
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Run mutations
// --------------------------------------------------------------------------

/// `run record-ship` — record the landed change reference, one committed
/// event whose replay moves the verified run to `submitted`.
///
/// # Errors
///
/// Returns an error if the run does not exist, the lease token is not live,
/// the run is not verified, or the store operations fail.
pub fn record_ship(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    change_ref: &ChangeRef,
) -> Result<Value> {
    locked(store, run_id, lease, |txn, run| {
        if run.state != RunState::Verified {
            return Err(SpeccyError::invalid_transition(format!(
                "run is {:?}, not verified; cannot ship",
                run.state
            )));
        }
        txn.append(Event::ShipRecorded {
            change_ref: change_ref.clone(),
        })?;
        Ok(json!({ "run_state": "submitted", "change_ref": change_ref }))
    })
}

/// `run interrupt` — record a harness interrupt (e.g. structured-output retry
/// exhaustion) and park the run at the escalation gate. The in-flight diff is
/// committed as a labeled snapshot first, then one decision event carrying
/// the snapshot SHA is appended; its replay applies the escalation. The
/// controller only receives the signal — the retry count lives in pack
/// prose, so the controller stays deterministic (DESIGN § Capability
/// Escalation and Give-Up Policy).
///
/// # Errors
///
/// Returns an error if the reason is outside the closed vocabulary, the run
/// does not exist, the lease token is not live, the run is not
/// `implementing`/`verifying`, or the store or git operations fail.
pub fn interrupt(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    reason: &str,
    detail: Option<&str>,
) -> Result<Value> {
    // Closed vocabulary; a single MVP value.
    if reason != "structured_output_retries_exhausted" {
        return Err(SpeccyError::validation(format!(
            "unknown interrupt reason `{reason}`; expected structured_output_retries_exhausted"
        )));
    }
    locked(store, run_id, lease, |txn, run| {
        if !matches!(run.state, RunState::Implementing | RunState::Verifying) {
            return Err(SpeccyError::invalid_transition(format!(
                "run is {:?}, not implementing or verifying; cannot interrupt",
                run.state
            )));
        }
        directive::ensure_on_branch(store, run)?;
        let snapshot = if gitx::is_dirty(&store.git_root)? {
            Some(gitx::commit_all(
                &store.git_root,
                &format!("speccy: {} escalation snapshot", run.spec_ref),
            )?)
        } else {
            None
        };
        let note = detail.map_or_else(|| reason.to_string(), |d| format!("{reason}: {d}"));
        txn.append(Event::RunDecision {
            decision: RunDecisionRecord {
                decision_id: ids::short_id("dec"),
                kind: RunDecisionKind::Interrupt,
                requirement: None,
                task: None,
                actor: "harness".into(),
                reason: Some(note),
                residual_risk: None,
                carry_forward: false,
                snapshot: snapshot.clone(),
            },
        })?;
        Ok(json!({
            "run_state": "escalated",
            "reason": reason,
            "snapshot": snapshot,
        }))
    })
}

/// The `run record-decision` input payload (SCHEMAS § run record-decision).
#[derive(Debug, Deserialize)]
pub struct RunDecisionInput {
    #[serde(rename = "type")]
    pub kind: String,
    pub requirement: Option<String>,
    pub task: Option<String>,
    #[serde(default = "default_actor")]
    pub actor: String,
    pub reason: Option<String>,
    pub residual_risk: Option<String>,
    #[serde(default)]
    pub carry_forward: bool,
}

fn default_actor() -> String {
    "human".into()
}

/// `run record-decision` — record a gate decision and apply its follow-on
/// transitions (SCHEMAS § run record-decision).
///
/// # Errors
///
/// Returns an error if the decision kind is unknown, the run does not exist,
/// the lease token is not live, the decision's gate preconditions do not
/// hold, or the store operations fail.
#[expect(
    clippy::too_many_lines,
    reason = "single decision-kind dispatch; each match arm is self-contained and \
              splitting would mean threading txn/run/decision through several \
              helpers for no clarity gain"
)]
pub fn record_decision(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    d: &RunDecisionInput,
) -> Result<Value> {
    locked(store, run_id, lease, |txn, run| {
        let record = |kind: RunDecisionKind| RunDecisionRecord {
            decision_id: ids::short_id("dec"),
            kind,
            requirement: d.requirement.clone(),
            task: d.task.clone(),
            actor: d.actor.clone(),
            reason: d.reason.clone(),
            residual_risk: d.residual_risk.clone(),
            carry_forward: d.carry_forward,
            snapshot: None,
        };
        let append_decision = |kind: RunDecisionKind| {
            txn.append(Event::RunDecision {
                decision: record(kind),
            })
        };

        match d.kind.as_str() {
            "rework" => {
                if run.state != RunState::Verified {
                    return Err(SpeccyError::invalid_transition(
                        "rework is only valid at the ship gate",
                    ));
                }
                // Validated here; replay seeds the RT task from the
                // decision's reason.
                require_reason(d, "rework")?;
                let decision = record(RunDecisionKind::Rework);
                let rt = run.next_rt_id();
                txn.append(Event::RunDecision {
                    decision: decision.clone(),
                })?;
                let config = ProjectConfig::load(&store.workspace_root)?;
                Ok(json!({
                    "decision_id": decision.decision_id,
                    "type": "rework",
                    "run_state": "implementing",
                    // The rework becomes the next run-level review round when the RT
                    // task re-enters verifying; report that number, not a constant.
                    "round": {
                        "current": run.run_review_round + 1,
                        "max": config.caps.run_review_rounds,
                        "scope": "run"
                    },
                    "task_appended": rt,
                    "resume": "call run next",
                }))
            }
            "cancel" => {
                append_decision(RunDecisionKind::Cancel)?;
                Ok(json!({ "type": "cancel", "run_state": "cancelled" }))
            }
            "waive" => {
                if run.state != RunState::Escalated {
                    return Err(SpeccyError::invalid_transition(
                        "waive is only valid at an escalation gate",
                    ));
                }
                let requirement = d
                    .requirement
                    .clone()
                    .ok_or_else(|| SpeccyError::validation("waive requires a requirement"))?;
                run.require_requirement(&requirement)?;
                if run.req_status(&requirement).is_resolved() {
                    return Err(SpeccyError::invalid_transition(format!(
                        "waive requires an unresolved requirement at the escalation gate ({requirement})"
                    )));
                }
                require_reason(d, "waive")?;
                require_residual_risk(d, "waive")?;
                // One committed event: replay applies the waived status and
                // the gate resume together.
                append_decision(RunDecisionKind::Waive)?;
                let resumed = txn.reload()?.state;
                Ok(json!({
                    "type": "waive",
                    "requirement": requirement,
                    "requirement_status": "waived",
                    "run_state": resumed.as_str(),
                    "resume": "call run next",
                }))
            }
            "provide_setup" => {
                if run.state != RunState::Escalated {
                    return Err(SpeccyError::invalid_transition(
                        "provide_setup is only valid at an escalation gate",
                    ));
                }
                require_reason(d, &d.kind)?;
                append_decision(RunDecisionKind::ProvideSetup)?;
                let resumed = txn.reload()?.state;
                Ok(
                    json!({ "type": d.kind, "run_state": resumed.as_str(), "resume": "call run next" }),
                )
            }
            "confirm_accepted_risk" => {
                if !run.at_accepted_risk_gate() {
                    return Err(SpeccyError::invalid_transition(
                        "confirm_accepted_risk is only valid at the accepted-risk confirmation gate",
                    ));
                }
                require_reason(d, &d.kind)?;
                append_decision(RunDecisionKind::ConfirmAcceptedRisk)?;
                Ok(json!({ "type": d.kind, "run_state": run.state, "resume": "call run next" }))
            }
            other => Err(SpeccyError::validation(format!(
                "unknown run decision type `{other}`"
            ))),
        }
    })
}

fn require_reason(d: &RunDecisionInput, kind: &str) -> Result<String> {
    match d.reason.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(r) => Ok(r.to_string()),
        None => Err(SpeccyError::validation(format!("{kind} requires a reason"))),
    }
}

fn require_residual_risk(d: &RunDecisionInput, kind: &str) -> Result<()> {
    if d.residual_risk.as_deref().map_or("", str::trim).is_empty() {
        Err(SpeccyError::validation(format!(
            "{kind} requires a residual_risk note"
        )))
    } else {
        Ok(())
    }
}
