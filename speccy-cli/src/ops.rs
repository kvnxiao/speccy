//! Controller operation dispatch and logic.
//!
//! Routes a parsed `speccy ctl` command to its handler and returns the `data`
//! payload of the response envelope. Behavior is owned by `DESIGN.md`; payload
//! shapes by `SCHEMAS.md`. Every state-mutating write is schema-validated and
//! never coerced into partial state.

use crate::cli::CtlCommand;
use crate::cli::EvidenceOp;
use crate::cli::FindingOp;
use crate::cli::PacketOp;
use crate::cli::RequirementOp;
use crate::cli::RunOp;
use crate::cli::SpecOp;
use crate::cli::TaskOp;
use camino::Utf8Component;
use camino::Utf8Path;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_json::json;
use speccy_core::config::ProjectConfig;
use speccy_core::directive;
use speccy_core::error::Finding;
use speccy_core::error::Result;
use speccy_core::error::SpeccyError;
use speccy_core::event::Event;
use speccy_core::event::EvidenceRecord;
use speccy_core::event::FindingRecord;
use speccy_core::event::Handoff;
use speccy_core::event::RequirementUpdate;
use speccy_core::event::RunDecisionRecord;
use speccy_core::event::SpecDecisionRecord;
use speccy_core::event::TaskInit;
use speccy_core::gitx;
use speccy_core::ids;
use speccy_core::lint;
use speccy_core::model::ChangeRef;
use speccy_core::model::EvidenceKind;
use speccy_core::model::RequirementStatus;
use speccy_core::model::RiskTier;
use speccy_core::model::RunDecisionKind;
use speccy_core::model::RunState;
use speccy_core::model::SpecDecisionKind;
use speccy_core::model::SpecDraft;
use speccy_core::model::TaskStatus;
use speccy_core::packets;
use speccy_core::projection::RunProjection;
use speccy_core::projection::SpecState;
use speccy_core::store::Store;
use std::io::Read;

/// Dispatch a controller operation, returning its `data` payload.
pub fn dispatch(command: CtlCommand) -> Result<Value> {
    let store = Store::open()?;
    match command {
        CtlCommand::Spec(op) => spec(&store, op),
        CtlCommand::Run(op) => run(&store, op),
        CtlCommand::Task(op) => task(&store, op),
        CtlCommand::Packet(op) => packet(&store, op),
        CtlCommand::Evidence(op) => evidence(&store, op),
        CtlCommand::Finding(op) => finding(&store, op),
        CtlCommand::Requirement(op) => requirement(&store, op),
    }
}

// --------------------------------------------------------------------------
// Input reading
// --------------------------------------------------------------------------

/// Read an `--input` payload (path or `-` for stdin) and deserialize it.
/// `serde_saphyr` parses both YAML and JSON (JSON is a subset of YAML).
fn read_input<T: DeserializeOwned>(path: &str) -> Result<T> {
    let text = if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| SpeccyError::io(format!("failed to read stdin: {e}")))?;
        buf
    } else {
        fs_err::read_to_string(path)
            .map_err(|e| SpeccyError::io(format!("failed to read {path}: {e}")))?
    };
    serde_saphyr::from_str(&text)
        .map_err(|e| SpeccyError::validation(format!("failed to parse input payload: {e}")))
}

// --------------------------------------------------------------------------
// Spec operations
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RequestInput {
    request: String,
    source: Option<String>,
    title: Option<String>,
    brainstorm_handoff: Option<String>,
}

fn spec(store: &Store, op: SpecOp) -> Result<Value> {
    match op {
        SpecOp::Start(args) => spec_start(store, &args.input),
        SpecOp::Status(args) => spec_status(store, &args.spec),
        SpecOp::RecordDraft(args) => spec_record_draft(store, &args.spec, &args.input),
        SpecOp::PatchDraft(args) => spec_patch_draft(store, &args.spec, &args.input),
        SpecOp::RecordDecision(args) => spec_record_decision(store, &args.spec, &args.input),
    }
}

fn spec_start(store: &Store, input: &str) -> Result<Value> {
    let req: RequestInput = read_input(input)?;
    if req.request.trim().is_empty() {
        return Err(SpeccyError::validation(
            "request is required and must be non-empty",
        ));
    }
    let (spec_ref, spec_id) =
        store.mint_spec(req.request, req.source, req.title, req.brainstorm_handoff)?;
    Ok(json!({
        "spec_ref": spec_ref,
        "spec_id": spec_id,
        "status": "draft",
        "workspace_id": store.workspace_id,
    }))
}

fn spec_status(store: &Store, spec_ref: &str) -> Result<Value> {
    let spec = store.spec_state_by_ref(spec_ref)?;
    let active = spec.approved_revision().or_else(|| spec.latest_revision());
    let active_revision = active.map(display_rev);
    let risk = active.and_then(|r| r.draft.risk.clone());
    let runs = store.list_runs(&spec.spec_id)?;
    Ok(json!({
        "spec_ref": spec.spec_ref,
        "title": spec.title,
        "status": spec.status,
        "active_revision": active_revision,
        "risk": risk,
        "runs": runs,
    }))
}

fn spec_record_draft(store: &Store, spec_ref: &str, input: &str) -> Result<Value> {
    let payload: SpecDraft = read_input(input)?;
    let mut spec = store.spec_state_by_ref(spec_ref)?;
    let (rev_id, draft) = match spec.latest_revision() {
        None => (next_revision_id(&spec), payload),
        Some(r) if r.approved => (next_revision_id(&spec), payload),
        Some(r) => (r.id.clone(), payload), // replace the current draft wholesale
    };
    apply_draft(store, &mut spec, &rev_id, draft)
}

fn spec_patch_draft(store: &Store, spec_ref: &str, input: &str) -> Result<Value> {
    #[derive(Deserialize)]
    struct PatchInput {
        set: SpecDraft,
    }
    let patch: PatchInput = read_input(input)?;
    let mut spec = store.spec_state_by_ref(spec_ref)?;
    let (rev_id, mut draft) = match spec.latest_revision() {
        None => (next_revision_id(&spec), SpecDraft::default()),
        Some(r) if r.approved => (next_revision_id(&spec), r.draft.clone()),
        Some(r) => (r.id.clone(), r.draft.clone()),
    };
    draft.apply_patch(patch.set);
    apply_draft(store, &mut spec, &rev_id, draft)
}

/// Store a draft revision and return its (display id, lint) result.
fn apply_draft(
    store: &Store,
    spec: &mut SpecState,
    rev_id: &str,
    draft: SpecDraft,
) -> Result<Value> {
    let config = ProjectConfig::load(&store.workspace_root)?;
    let findings = lint::lint_draft(&draft, &config.evidence.command_policy);
    store.append_spec_event(
        &spec.spec_id,
        Event::DraftUpdated {
            revision_id: rev_id.to_string(),
            draft,
        },
    )?;
    Ok(json!({
        "draft": format!("{rev_id}-draft"),
        "lint": lint_value(&findings),
    }))
}

#[derive(Debug, Deserialize)]
struct SpecDecisionInput {
    #[serde(rename = "type")]
    kind: String,
    revision: String,
    #[serde(default = "default_actor")]
    actor: String,
    approved_in_prose: Option<String>,
    note: Option<String>,
    #[serde(default)]
    carry_forward: bool,
    supersedes: Option<speccy_core::event::Supersedes>,
}

fn default_actor() -> String {
    "human".into()
}

#[expect(
    clippy::too_many_lines,
    reason = "single decision-kind dispatch; each match arm is self-contained and \
              splitting would mean threading store/spec/target/decision through \
              several helpers for no clarity gain"
)]
fn spec_record_decision(store: &Store, spec_ref: &str, input: &str) -> Result<Value> {
    let decision: SpecDecisionInput = read_input(input)?;
    let spec = store.spec_state_by_ref(spec_ref)?;
    let target = strip_draft(&decision.revision);

    match decision.kind.as_str() {
        "approve" => {
            if decision
                .approved_in_prose
                .as_deref()
                .map_or("", str::trim)
                .is_empty()
            {
                return Err(SpeccyError::validation(
                    "approve requires approved_in_prose",
                ));
            }
            let rev = spec
                .revision(target)
                .ok_or_else(|| SpeccyError::not_found(format!("no revision {target}")))?;
            if rev.approved {
                return Err(SpeccyError::invalid_transition(format!(
                    "{target} is already approved; an approved revision is immutable"
                )));
            }
            let config = ProjectConfig::load(&store.workspace_root)?;
            let findings = lint::lint_draft(&rev.draft, &config.evidence.command_policy);
            if !findings.is_empty() {
                return Err(SpeccyError::validation(
                    "approval refused while the draft is lint-dirty",
                )
                .with_details(findings));
            }
            // Resolve and validate the superseded run BEFORE recording the
            // approval, so a bad or non-parked run_id can never leave the spec
            // approved with the parked run still open. A residual crash window
            // between the approval append and the run's cancellation remains;
            // its recovery is the gate's ordinary cancel answer (DESIGN §
            // Amendment at the Escalation Gate).
            let superseded = match decision.supersedes.as_ref().and_then(|s| s.run_id.clone()) {
                Some(run_id) => {
                    let (run_spec_id, _) = store.find_run(&run_id)?;
                    let run = store.run_projection(&run_spec_id, &run_id)?;
                    if !matches!(run.state, RunState::Escalated | RunState::Verified) {
                        return Err(SpeccyError::invalid_transition(format!(
                            "run {run_id} is {:?}, not parked at an escalation or ship gate; cannot supersede it",
                            run.state
                        )));
                    }
                    Some((run_spec_id, run_id))
                }
                None => None,
            };

            let record = SpecDecisionRecord {
                decision_id: ids::short_id("dec"),
                kind: SpecDecisionKind::Approve,
                revision_id: target.to_string(),
                actor: decision.actor,
                approved_in_prose: decision.approved_in_prose,
                note: decision.note,
                carry_forward: decision.carry_forward,
                supersedes: decision.supersedes,
            };
            store.append_spec_event(&spec.spec_id, Event::SpecDecision { decision: record })?;

            // Amendment at a gate: a superseding approval atomically closes the
            // parked run as cancelled with a linking decision record (DESIGN §
            // Amendment at the Escalation Gate).
            let mut superseded_run = None;
            if let Some((run_spec_id, run_id)) = superseded {
                store.append_run_event(
                    &run_spec_id,
                    &run_id,
                    Event::RunDecision {
                        decision: RunDecisionRecord {
                            decision_id: ids::short_id("dec"),
                            kind: RunDecisionKind::Superseded,
                            requirement: None,
                            task: None,
                            actor: "human".into(),
                            reason: Some(format!("superseded by revision {target}")),
                            residual_risk: None,
                            carry_forward: false,
                        },
                    },
                )?;
                store.append_run_event(
                    &run_spec_id,
                    &run_id,
                    Event::RunStateTransitioned {
                        to: RunState::Cancelled,
                        snapshot: None,
                    },
                )?;
                superseded_run = Some(run_id);
            }
            Ok(json!({
                "approved_revision": target,
                "spec_status": "approved",
                "requirements_frozen": true,
                "superseded_run": superseded_run,
                "next": "Run /speccy-implement (fresh session recommended).",
            }))
        }
        "cancel" => {
            let record = spec_decision_record(&decision, SpecDecisionKind::Cancel, target);
            store.append_spec_event(&spec.spec_id, Event::SpecDecision { decision: record })?;
            Ok(json!({ "spec_status": "cancelled" }))
        }
        "reject" | "split" | "scope_change" => {
            // The match guarantees a valid kind; parse maps it to the enum.
            let kind = SpecDecisionKind::parse(&decision.kind).ok_or_else(|| {
                SpeccyError::validation(format!("unknown spec decision type `{}`", decision.kind))
            })?;
            let record = spec_decision_record(&decision, kind, target);
            store.append_spec_event(&spec.spec_id, Event::SpecDecision { decision: record })?;
            Ok(json!({ "decision": decision.kind, "spec_status": spec.status }))
        }
        other => Err(SpeccyError::validation(format!(
            "unknown spec decision type `{other}`"
        ))),
    }
}

fn spec_decision_record(
    input: &SpecDecisionInput,
    kind: SpecDecisionKind,
    target: &str,
) -> SpecDecisionRecord {
    SpecDecisionRecord {
        decision_id: ids::short_id("dec"),
        kind,
        revision_id: target.to_string(),
        actor: input.actor.clone(),
        approved_in_prose: input.approved_in_prose.clone(),
        note: input.note.clone(),
        carry_forward: input.carry_forward,
        supersedes: input.supersedes.clone(),
    }
}

// --------------------------------------------------------------------------
// Run operations
// --------------------------------------------------------------------------

fn run(store: &Store, op: RunOp) -> Result<Value> {
    match op {
        RunOp::Start(args) => run_start(store, &args.spec, &args.revision),
        RunOp::Status(args) => run_status(store, &args.run),
        RunOp::Next(args) => run_next(store, &args.run, &args.agent),
        RunOp::RecordDecision(args) => {
            run_record_decision(store, &args.run, args.lease.as_deref(), &args.input)
        }
        RunOp::RecordShip(args) => {
            run_record_ship(store, &args.run, args.lease.as_deref(), &args.input)
        }
        RunOp::Interrupt(args) => {
            run_interrupt(store, &args.run, args.lease.as_deref(), &args.input)
        }
    }
}

#[derive(Debug, Deserialize)]
struct InterruptInput {
    reason: String,
    detail: Option<String>,
}

fn run_interrupt(store: &Store, run_id: &str, lease: Option<&str>, input: &str) -> Result<Value> {
    let payload: InterruptInput = read_input(input)?;
    // Closed vocabulary; a single MVP value.
    if payload.reason != "structured_output_retries_exhausted" {
        return Err(SpeccyError::validation(format!(
            "unknown interrupt reason `{}`; expected structured_output_retries_exhausted",
            payload.reason
        )));
    }
    let (spec_id, _) = store.find_run(run_id)?;
    store.verify_lease(&spec_id, run_id, lease)?;
    let applied = directive::interrupt_run(
        store,
        &spec_id,
        run_id,
        &payload.reason,
        payload.detail.as_deref(),
    )?;
    Ok(json!({
        "run_state": "escalated",
        "reason": payload.reason,
        "snapshot": applied.snapshot,
    }))
}

fn run_start(store: &Store, spec_ref: &str, revision: &str) -> Result<Value> {
    let spec = store.spec_state_by_ref(spec_ref)?;
    let target = strip_draft(revision);
    let rev = spec
        .revision(target)
        .ok_or_else(|| SpeccyError::not_found(format!("no revision {target}")))?;
    if !rev.approved {
        return Err(SpeccyError::invalid_transition(format!(
            "revision {target} is not approved; run start requires an approved revision"
        )));
    }

    // Gate: clean worktree before any run state exists.
    let dirty = gitx::dirty_files(&store.git_root)?;
    if !dirty.is_empty() {
        return Err(SpeccyError::dirty_worktree(format!(
            "run start refused: {} uncommitted files",
            dirty.len()
        ))
        .with_details(
            dirty
                .iter()
                .map(|f| Finding::new("dirty_file", f))
                .collect(),
        ));
    }

    let branch = ids::run_branch(&spec.spec_ref, spec.title.as_deref());
    if gitx::branch_exists(&store.git_root, &branch)? {
        gitx::checkout(&store.git_root, &branch)?;
    } else {
        gitx::create_branch(&store.git_root, &branch)?;
    }
    // Record the base *after* checkout: the run branch's tip. For a reused
    // branch that is the earlier run's last snapshot, not the unrelated commit
    // the user happened to have checked out (`create_branch` is `checkout -b`,
    // so HEAD is unchanged and this works for both paths).
    let base_commit = gitx::head(&store.git_root)?;

    let risk = rev.draft.risk.clone().unwrap_or_else(|| "standard".into());
    let tasks: Vec<TaskInit> = rev
        .draft
        .tasks()
        .iter()
        .map(|t| TaskInit {
            id: t.id.clone(),
            title: t.title.clone(),
            requirements: t.requirements.clone(),
            constraints: t.constraints.clone(),
        })
        .collect();

    let run_id = ids::run_id();
    store.append_run_event(
        &spec.spec_id,
        &run_id,
        Event::RunStarted {
            run_id: run_id.clone(),
            spec_ref: spec.spec_ref.clone(),
            spec_id: spec.spec_id.clone(),
            revision_id: target.to_string(),
            risk,
            branch: branch.clone(),
            base_commit,
            tasks: tasks.clone(),
        },
    )?;

    let task_view: Vec<Value> = tasks
        .iter()
        .map(|t| json!({ "id": t.id, "status": "queued", "requirements": t.requirements }))
        .collect();
    Ok(json!({
        "run_id": run_id,
        "run_state": "implementing",
        "branch": branch,
        "tasks": task_view,
    }))
}

fn run_status(store: &Store, run_id: &str) -> Result<Value> {
    let (_, run) = store.run_by_id(run_id)?;
    let tasks: Vec<Value> = run
        .tasks
        .iter()
        .map(|t| json!({ "id": t.id, "status": t.status, "round": t.round }))
        .collect();
    let requirements: Vec<Value> = run
        .requirements
        .iter()
        .map(|(id, r)| json!({ "id": id, "status": r.status }))
        .collect();
    Ok(json!({
        "run_id": run.run_id,
        "spec_ref": run.spec_ref,
        "run_state": run.state,
        "branch": run.branch,
        "risk": run.risk,
        "tasks": tasks,
        "requirements": requirements,
    }))
}

fn run_next(store: &Store, run_id: &str, agent: &str) -> Result<Value> {
    let (spec_id, _) = store.find_run(run_id)?;
    directive::run_next(store, &spec_id, run_id, agent)
}

fn run_record_ship(store: &Store, run_id: &str, lease: Option<&str>, input: &str) -> Result<Value> {
    let change_ref: ChangeRef = read_input(input)?;
    let (spec_id, run) = store.run_by_id(run_id)?;
    store.verify_lease(&spec_id, run_id, lease)?;
    if run.state != RunState::Verified {
        return Err(SpeccyError::invalid_transition(format!(
            "run is {:?}, not verified; cannot ship",
            run.state
        )));
    }
    store.append_run_event(
        &spec_id,
        run_id,
        Event::ShipRecorded {
            change_ref: change_ref.clone(),
        },
    )?;
    store.append_run_event(
        &spec_id,
        run_id,
        Event::RunStateTransitioned {
            to: RunState::Submitted,
            snapshot: None,
        },
    )?;
    Ok(json!({ "run_state": "submitted", "change_ref": change_ref }))
}

#[derive(Debug, Deserialize)]
struct RunDecisionInput {
    #[serde(rename = "type")]
    kind: String,
    requirement: Option<String>,
    task: Option<String>,
    #[serde(default = "default_actor")]
    actor: String,
    reason: Option<String>,
    residual_risk: Option<String>,
    #[serde(default)]
    carry_forward: bool,
}

#[expect(
    clippy::too_many_lines,
    reason = "single decision-kind dispatch; each match arm is self-contained and \
              splitting would mean threading store/spec_id/run_id/decision through \
              several helpers for no clarity gain"
)]
fn run_record_decision(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    input: &str,
) -> Result<Value> {
    let d: RunDecisionInput = read_input(input)?;
    let (spec_id, run) = store.run_by_id(run_id)?;
    store.verify_lease(&spec_id, run_id, lease)?;

    let record = |kind: RunDecisionKind| RunDecisionRecord {
        decision_id: ids::short_id("dec"),
        kind,
        requirement: d.requirement.clone(),
        task: d.task.clone(),
        actor: d.actor.clone(),
        reason: d.reason.clone(),
        residual_risk: d.residual_risk.clone(),
        carry_forward: d.carry_forward,
    };
    let append_decision = |kind: RunDecisionKind| {
        store.append_run_event(
            &spec_id,
            run_id,
            Event::RunDecision {
                decision: record(kind),
            },
        )
    };

    match d.kind.as_str() {
        "rework" => {
            if run.state != RunState::Verified {
                return Err(SpeccyError::invalid_transition(
                    "rework is only valid at the ship gate",
                ));
            }
            let reason = require_reason(&d, "rework")?;
            let decision = record(RunDecisionKind::Rework);
            store.append_run_event(
                &spec_id,
                run_id,
                Event::RunDecision {
                    decision: decision.clone(),
                },
            )?;
            let rt = directive::next_rt_id(&run);
            store.append_run_event(
                &spec_id,
                run_id,
                Event::TaskAppended {
                    task: TaskInit {
                        id: rt.clone(),
                        title: Some("Rework from ship feedback".into()),
                        requirements: Vec::new(),
                        constraints: Vec::new(),
                    },
                    seed_feedback: Some(reason),
                },
            )?;
            store.append_run_event(
                &spec_id,
                run_id,
                Event::RunStateTransitioned {
                    to: RunState::Implementing,
                    snapshot: None,
                },
            )?;
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
            store.append_run_event(
                &spec_id,
                run_id,
                Event::RunStateTransitioned {
                    to: RunState::Cancelled,
                    snapshot: None,
                },
            )?;
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
            ensure_run_requirement(&run, &requirement)?;
            if run.req_status(&requirement).is_resolved() {
                return Err(SpeccyError::invalid_transition(format!(
                    "waive requires an unresolved requirement at the escalation gate ({requirement})"
                )));
            }
            require_reason(&d, "waive")?;
            require_residual_risk(&d, "waive")?;
            append_decision(RunDecisionKind::Waive)?;
            store.append_run_event(
                &spec_id,
                run_id,
                Event::RequirementStatusSet {
                    updates: vec![RequirementUpdate {
                        requirement: requirement.clone(),
                        status: RequirementStatus::Waived,
                        evidence: Vec::new(),
                        findings: Vec::new(),
                        residual_risk: d.residual_risk.clone(),
                        note: d.reason.clone(),
                    }],
                },
            )?;
            let resumed = resume_from_escalated(store, &spec_id, run_id)?;
            Ok(json!({
                "type": "waive",
                "requirement": requirement,
                "requirement_status": "waived",
                "run_state": resumed,
                "resume": "call run next",
            }))
        }
        "provide_setup" => {
            if run.state != RunState::Escalated {
                return Err(SpeccyError::invalid_transition(
                    "provide_setup is only valid at an escalation gate",
                ));
            }
            require_reason(&d, &d.kind)?;
            append_decision(RunDecisionKind::ProvideSetup)?;
            let resumed = resume_from_escalated(store, &spec_id, run_id)?;
            Ok(json!({ "type": d.kind, "run_state": resumed, "resume": "call run next" }))
        }
        "confirm_accepted_risk" => {
            if !run.at_accepted_risk_gate() {
                return Err(SpeccyError::invalid_transition(
                    "confirm_accepted_risk is only valid at the accepted-risk confirmation gate",
                ));
            }
            require_reason(&d, &d.kind)?;
            append_decision(RunDecisionKind::ConfirmAcceptedRisk)?;
            Ok(json!({ "type": d.kind, "run_state": run.state, "resume": "call run next" }))
        }
        other => Err(SpeccyError::validation(format!(
            "unknown run decision type `{other}`"
        ))),
    }
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

/// After a gate decision that unblocks a parked run, move it back into the
/// loop with a `RunResumed` event — distinct from a state transition, so it
/// re-enters without opening a new review round (DESIGN § Capability
/// Escalation and Give-Up Policy).
fn resume_from_escalated(store: &Store, spec_id: &str, run_id: &str) -> Result<&'static str> {
    let run = store.run_projection(spec_id, run_id)?;
    if run.state != RunState::Escalated {
        return Ok(run.state.as_str());
    }

    if run.all_tasks_done() {
        // Run-level gate. Re-open the current round's review only when work is
        // still outstanding; a waiver that resolved the last requirement lets
        // `verifying` complete directly (subject to the critical gate).
        let reopen_review =
            !(run.all_requirements_resolved() && run.run_blocking_findings().is_empty());
        store.append_run_event(
            spec_id,
            run_id,
            Event::RunResumed {
                to: RunState::Verifying,
                reopen_review,
            },
        )?;
        return Ok(RunState::Verifying.as_str());
    }

    // Task-level gate. Re-enter implementing without opening a review round.
    store.append_run_event(
        spec_id,
        run_id,
        Event::RunResumed {
            to: RunState::Implementing,
            reopen_review: false,
        },
    )?;
    // The stuck task is still `in_review` with its failing statuses. If it would
    // re-escalate immediately (provide_setup, or a partial waiver), re-open it
    // to `building` at the SAME round so the worker retries with the new setup;
    // a waiver that fully resolved the task instead integrates on the next
    // `run next`, so no re-open is needed.
    if let Some(task) = run.active_task()
        && task.status == TaskStatus::InReview
        && run.task_reviewed_this_round(task)
    {
        let would_integrate = run.task_requirements_resolved(task)
            && run.task_blocking_findings_this_round(task).is_empty();
        if !would_integrate {
            store.append_run_event(
                spec_id,
                run_id,
                Event::TaskTransitioned {
                    task: task.id.clone(),
                    to: TaskStatus::Building,
                    round: task.round,
                    snapshot: None,
                },
            )?;
        }
    }
    Ok(RunState::Implementing.as_str())
}

// --------------------------------------------------------------------------
// Task operations
// --------------------------------------------------------------------------

fn task(store: &Store, op: TaskOp) -> Result<Value> {
    match op {
        TaskOp::Claim(args) => task_claim(store, &args.run, &args.task, &args.agent, &args.lease),
        TaskOp::RecordHandoff(args) => {
            task_record_handoff(store, &args.run, args.lease.as_deref(), &args.input)
        }
    }
}

fn task_claim(
    store: &Store,
    run_id: &str,
    task_id: &str,
    agent: &str,
    lease: &str,
) -> Result<Value> {
    let (spec_id, run) = store.run_by_id(run_id)?;
    store.verify_lease(&spec_id, run_id, Some(lease))?;
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
    store.append_run_event(
        &spec_id,
        run_id,
        Event::TaskClaimed {
            task: task_id.to_string(),
            agent: agent.to_string(),
            baseline_commit: baseline_commit.clone(),
        },
    )?;
    Ok(json!({
        "task": task_id,
        "status": "building",
        "round": 1,
        "baseline_commit": baseline_commit,
    }))
}

fn task_record_handoff(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    input: &str,
) -> Result<Value> {
    let handoff: Handoff = read_input(input)?;
    if handoff.summary.trim().is_empty() {
        return Err(
            SpeccyError::validation("handoff.summary is required").with_details(vec![Finding::at(
                "missing_field",
                "summary",
                "summary is required",
            )]),
        );
    }
    let (spec_id, run) = store.run_by_id(run_id)?;
    store.verify_lease(&spec_id, run_id, lease)?;
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
    let round = expected_round;
    store.append_run_event(
        &spec_id,
        run_id,
        Event::HandoffRecorded {
            handoff_id: handoff_id.clone(),
            task: task_id.clone(),
            round,
            handoff,
        },
    )?;
    Ok(json!({ "task": task_id, "status": "in_review", "handoff_id": handoff_id }))
}

// --------------------------------------------------------------------------
// Evidence, findings, requirement status
// --------------------------------------------------------------------------

fn evidence(store: &Store, op: EvidenceOp) -> Result<Value> {
    match op {
        EvidenceOp::Collect(args) => {
            speccy_core::evidence::collect(store, &args.run, &args.requirements, &args.requests)
        }
        EvidenceOp::Record(args) => evidence_record(store, &args.run, &args.input),
    }
}

#[derive(Debug, Deserialize)]
struct EvidenceInput {
    requirement: String,
    request: Option<String>,
    kind: String,
    collected_by: String,
    note: Option<String>,
    artifact: Option<String>,
}

fn evidence_record(store: &Store, run_id: &str, input: &str) -> Result<Value> {
    let ev: EvidenceInput = read_input(input)?;
    let (spec_id, run) = store.run_by_id(run_id)?;
    let kind = match EvidenceKind::parse(&ev.kind) {
        Some(EvidenceKind::Command) => {
            return Err(SpeccyError::validation(
                "evidence record refuses agent-supplied output for kind: command; use evidence collect",
            ));
        }
        None => {
            return Err(SpeccyError::validation(format!(
                "invalid evidence kind `{}`",
                ev.kind
            )));
        }
        Some(k) => k,
    };
    validate_evidence_reference(store, &run, &ev, kind)?;
    // Browser/api evidence on high/critical requires a stored, hashed artifact
    // (DESIGN § Acceptance Ledger); prose-only records are refused.
    if matches!(kind, EvidenceKind::Browser | EvidenceKind::Api)
        && matches!(run.risk, RiskTier::High | RiskTier::Critical)
        && ev.artifact.as_deref().map_or("", str::trim).is_empty()
    {
        return Err(SpeccyError::validation(format!(
            "kind: {} requires an artifact reference at risk {:?}; store a screenshot, trace, or DOM capture",
            ev.kind, run.risk
        )));
    }
    let artifact_hash = hash_artifact_if_present(store, &spec_id, run_id, ev.artifact.as_deref())?;
    let id = ids::short_id("ev");
    let record = EvidenceRecord {
        id: id.clone(),
        requirement: ev.requirement.clone(),
        request: ev.request,
        kind,
        collected_by: ev.collected_by,
        note: ev.note,
        artifact: ev.artifact,
        artifact_hash,
        command: None,
        exit_code: None,
        stdout_hash: None,
    };
    store.append_run_event(
        &spec_id,
        run_id,
        Event::EvidenceRecorded {
            evidence: record.clone(),
        },
    )?;
    serde_json::to_value(&record)
        .map_err(|e| SpeccyError::io(format!("failed to serialize evidence: {e}")))
}

fn finding(store: &Store, op: FindingOp) -> Result<Value> {
    let FindingOp::Record(args) = op;
    finding_record(store, &args.run, &args.input)
}

#[derive(Debug, Deserialize)]
struct FindingInput {
    requirement: Option<String>,
    task: Option<String>,
    persona: Option<String>,
    severity: String,
    note: String,
    #[serde(default = "default_recorder")]
    recorded_by: String,
}

fn default_recorder() -> String {
    "harness".into()
}

fn finding_record(store: &Store, run_id: &str, input: &str) -> Result<Value> {
    let f: FindingInput = read_input(input)?;
    // Validate severity against the closed vocabulary so a typo cannot silently
    // read as non-blocking and let a real blocker slip past aggregation.
    let severity = speccy_core::model::FindingSeverity::parse(&f.severity).ok_or_else(|| {
        SpeccyError::validation(format!(
            "invalid finding severity `{}`; expected blocking|advisory|positive|uncertain",
            f.severity
        ))
    })?;
    if f.note.trim().is_empty() {
        return Err(SpeccyError::validation("finding.note is required"));
    }
    let (spec_id, run) = store.run_by_id(run_id)?;
    if let Some(requirement) = &f.requirement {
        ensure_run_requirement(&run, requirement)?;
    }
    if let Some(task) = &f.task
        && run.task(task).is_none()
    {
        return Err(SpeccyError::not_found(format!(
            "no task {task} in run {run_id}"
        )));
    }
    let id = ids::short_id("fd");
    let record = FindingRecord {
        id: id.clone(),
        requirement: f.requirement,
        task: f.task,
        persona: f.persona,
        severity,
        note: f.note,
        recorded_by: f.recorded_by,
    };
    store.append_run_event(
        &spec_id,
        run_id,
        Event::FindingRecorded {
            finding: record.clone(),
        },
    )?;
    serde_json::to_value(&record)
        .map_err(|e| SpeccyError::io(format!("failed to serialize finding: {e}")))
}

fn requirement(store: &Store, op: RequirementOp) -> Result<Value> {
    let RequirementOp::SetStatus(args) = op;
    requirement_set_status(store, &args.run, args.lease.as_deref(), &args.input)
}

#[derive(Debug, Deserialize)]
struct StatusInput {
    updates: Vec<RequirementUpdate>,
}

fn requirement_set_status(
    store: &Store,
    run_id: &str,
    lease: Option<&str>,
    input: &str,
) -> Result<Value> {
    let payload: StatusInput = read_input(input)?;
    let (spec_id, run) = store.run_by_id(run_id)?;
    store.verify_lease(&spec_id, run_id, lease)?;

    // Validate each update's prerequisites before recording anything.
    for u in &payload.updates {
        validate_status_update(u, &run)?;
    }

    let updated: Vec<Value> = payload
        .updates
        .iter()
        .map(|u| json!({ "requirement": u.requirement, "status": u.status }))
        .collect();
    store.append_run_event(
        &spec_id,
        run_id,
        Event::RequirementStatusSet {
            updates: payload.updates,
        },
    )?;
    Ok(json!({ "updated": updated }))
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
            if matches!(run.risk, RiskTier::High | RiskTier::Critical)
                && u.residual_risk.as_deref().map_or("", str::trim).is_empty()
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

fn ensure_run_requirement(run: &RunProjection, requirement: &str) -> Result<()> {
    if run.requirements.contains_key(requirement) {
        Ok(())
    } else {
        Err(SpeccyError::not_found(format!(
            "no requirement {requirement} in run {}",
            run.run_id
        )))
    }
}

fn validate_evidence_reference(
    store: &Store,
    run: &RunProjection,
    ev: &EvidenceInput,
    kind: EvidenceKind,
) -> Result<()> {
    ensure_run_requirement(run, &ev.requirement)?;
    let Some(request) = ev.request.as_deref() else {
        return Ok(());
    };
    let (req_id, request_id) = match request.split_once('.') {
        Some((req, id)) => {
            if req != ev.requirement {
                return Err(SpeccyError::validation(format!(
                    "evidence request {request} does not belong to requirement {}",
                    ev.requirement
                )));
            }
            (req, id)
        }
        None => (ev.requirement.as_str(), request),
    };
    let draft = store.run_draft(run)?;
    let req = draft
        .requirement(req_id)
        .ok_or_else(|| SpeccyError::not_found(format!("no requirement {req_id}")))?;
    let declared = req
        .evidence
        .iter()
        .find(|e| e.id == request_id)
        .ok_or_else(|| SpeccyError::not_found(format!("no evidence request {request}")))?;
    let declared_kind = declared.kind_enum().ok_or_else(|| {
        SpeccyError::validation(format!("evidence request {request} has no valid kind"))
    })?;
    if declared_kind != kind {
        return Err(SpeccyError::validation(format!(
            "evidence request {request} is kind {declared_kind:?}, not {kind:?}"
        )));
    }
    Ok(())
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

fn hash_artifact_if_present(
    store: &Store,
    spec_id: &str,
    run_id: &str,
    artifact: Option<&str>,
) -> Result<Option<String>> {
    let Some(artifact) = artifact.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let rel = Utf8Path::new(artifact);
    if rel.is_absolute()
        || rel.components().any(|c| {
            matches!(
                c,
                Utf8Component::ParentDir | Utf8Component::RootDir | Utf8Component::Prefix(_)
            )
        })
        || rel
            .components()
            .next()
            .is_none_or(|c| c.as_str() != "evidence")
    {
        return Err(SpeccyError::validation(format!(
            "artifact reference must stay within the run evidence tree under evidence/: {artifact}"
        )));
    }
    let root = store.run_dir(spec_id, run_id);
    let path = root.join(rel);
    let bytes = fs_err::read(&path).map_err(|e| {
        SpeccyError::validation(format!(
            "artifact reference {artifact} is not readable under {root}: {e}"
        ))
    })?;
    Ok(Some(speccy_core::hash::sha256_prefixed(&bytes)))
}

// --------------------------------------------------------------------------
// Packet operations
// --------------------------------------------------------------------------

fn packet(store: &Store, op: PacketOp) -> Result<Value> {
    match op {
        PacketOp::Planning(args) => packets::planning(store, &args.spec),
        PacketOp::Task(args) => packets::task(store, &args.run, &args.task),
        PacketOp::Verification(args) => packets::verification(store, &args.run, &args.requirements),
        PacketOp::Review(args) => packets::review(store, &args.run),
        PacketOp::Escalation(args) => packets::escalation(store, &args.run),
    }
}

// --------------------------------------------------------------------------
// Shared helpers
// --------------------------------------------------------------------------

fn lint_value(findings: &[Finding]) -> Value {
    // `Finding` serializes with `path` omitted when absent (SCHEMAS § Envelope).
    json!({ "clean": findings.is_empty(), "findings": findings })
}

fn strip_draft(revision: &str) -> &str {
    revision.strip_suffix("-draft").unwrap_or(revision)
}

fn display_rev(rev: &speccy_core::projection::Revision) -> String {
    if rev.approved {
        rev.id.clone()
    } else {
        format!("{}-draft", rev.id)
    }
}

fn next_revision_id(spec: &SpecState) -> String {
    format!("spec_rev_{:03}", spec.revisions.len() + 1)
}
