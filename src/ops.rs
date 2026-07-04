//! Controller operation dispatch and logic.
//!
//! Routes a parsed `speccy ctl` command to its handler and returns the `data`
//! payload of the response envelope. Behavior is owned by `DESIGN.md`; payload
//! shapes by `SCHEMAS.md`. Every state-mutating write is schema-validated and
//! never coerced into partial state.

use std::io::Read;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::cli::{
    CtlCommand, EvidenceOp, FindingOp, PacketOp, RequirementOp, RunOp, SpecOp, TaskOp,
};
use crate::config::ProjectConfig;
use crate::directive;
use crate::error::{Finding, Result, SpeccyError};
use crate::event::{
    Event, EvidenceRecord, FindingRecord, Handoff, RequirementUpdate, SpecDecisionRecord, TaskInit,
};
use crate::gitx;
use crate::ids;
use crate::lint;
use crate::model::{EvidenceKind, RequirementStatus, RiskTier, SpecDraft, TaskStatus};
use crate::packets;
use crate::projection::SpecState;
use crate::store::Store;

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
        std::fs::read_to_string(path)
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
    let existing: Vec<String> = store.list_specs()?.into_iter().map(|(_, r)| r).collect();
    let mut spec_ref = ids::spec_ref();
    for _ in 0..8 {
        if !existing.contains(&spec_ref) {
            break;
        }
        spec_ref = ids::spec_ref();
    }
    let spec_id = ids::spec_id();
    store.create_spec(&spec_id, &spec_ref)?;
    store.append_spec_event(
        &spec_id,
        Event::SpecCreated {
            spec_ref: spec_ref.clone(),
            spec_id: spec_id.clone(),
            workspace_id: store.workspace_id.clone(),
            request: req.request,
            source: req.source,
            title: req.title,
            brainstorm_handoff: req.brainstorm_handoff,
        },
    )?;
    Ok(json!({
        "spec_ref": spec_ref,
        "spec_id": spec_id,
        "status": "draft",
        "workspace_id": store.workspace_id,
    }))
}

fn spec_status(store: &Store, spec_ref: &str) -> Result<Value> {
    let spec = store.spec_state_by_ref(spec_ref)?;
    let active_revision = spec
        .approved_revision()
        .map(|r| r.id.clone())
        .or_else(|| spec.latest_revision().map(display_rev));
    let risk = spec.latest_revision().and_then(|r| r.draft.risk.clone());
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
    supersedes: Option<crate::event::Supersedes>,
}

fn default_actor() -> String {
    "human".into()
}

fn spec_record_decision(store: &Store, spec_ref: &str, input: &str) -> Result<Value> {
    let decision: SpecDecisionInput = read_input(input)?;
    let spec = store.spec_state_by_ref(spec_ref)?;
    let target = strip_draft(&decision.revision);

    match decision.kind.as_str() {
        "approve" => {
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
            let record = SpecDecisionRecord {
                decision_id: ids::short_id("dec"),
                kind: "approve".into(),
                revision_id: target.to_string(),
                actor: decision.actor,
                approved_in_prose: decision.approved_in_prose,
                note: decision.note,
                carry_forward: decision.carry_forward,
                supersedes: decision.supersedes,
            };
            store.append_spec_event(&spec.spec_id, Event::SpecDecision { decision: record })?;
            Ok(json!({
                "approved_revision": target,
                "spec_status": "approved",
                "requirements_frozen": true,
                "next": "Run /speccy-implement (fresh session recommended).",
            }))
        }
        "cancel" => {
            let record = spec_decision_record(&decision, "cancel", target);
            store.append_spec_event(&spec.spec_id, Event::SpecDecision { decision: record })?;
            Ok(json!({ "spec_status": "cancelled" }))
        }
        "reject" | "split" | "scope_change" => {
            let record = spec_decision_record(&decision, &decision.kind, target);
            store.append_spec_event(&spec.spec_id, Event::SpecDecision { decision: record })?;
            Ok(json!({ "decision": decision.kind, "spec_status": spec.status }))
        }
        other => Err(SpeccyError::validation(format!(
            "unknown spec decision type `{other}`"
        ))),
    }
}

fn spec_decision_record(input: &SpecDecisionInput, kind: &str, target: &str) -> SpecDecisionRecord {
    SpecDecisionRecord {
        decision_id: ids::short_id("dec"),
        kind: kind.to_string(),
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
        RunOp::RecordDecision(_) => Err(SpeccyError::not_implemented("run record-decision (M3)")),
        RunOp::RecordShip(_) => Err(SpeccyError::not_implemented("run record-ship (M3)")),
    }
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
    let base_commit = gitx::head(&store.git_root)?;
    if gitx::branch_exists(&store.git_root, &branch)? {
        gitx::checkout(&store.git_root, &branch)?;
    } else {
        gitx::create_branch(&store.git_root, &branch)?;
    }

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

// --------------------------------------------------------------------------
// Task operations
// --------------------------------------------------------------------------

fn task(store: &Store, op: TaskOp) -> Result<Value> {
    match op {
        TaskOp::Claim(args) => task_claim(store, &args.run, &args.task),
        TaskOp::RecordHandoff(args) => task_record_handoff(store, &args.run, &args.input),
    }
}

fn task_claim(store: &Store, run_id: &str, task_id: &str) -> Result<Value> {
    let (spec_id, run) = store.run_by_id(run_id)?;
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
            agent: "claude".into(),
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

fn task_record_handoff(store: &Store, run_id: &str, input: &str) -> Result<Value> {
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
    let task = run.task(&handoff.task).ok_or_else(|| {
        SpeccyError::not_found(format!("no task {} in run {run_id}", handoff.task))
    })?;
    if task.status != TaskStatus::Building {
        return Err(SpeccyError::invalid_transition(format!(
            "task {} is {:?}, not building; cannot record a handoff",
            handoff.task, task.status
        )));
    }
    let handoff_id = ids::short_id("ho");
    let task_id = handoff.task.clone();
    let round = task.round.max(1);
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
        EvidenceOp::Collect(_) => Err(SpeccyError::not_implemented("evidence collect (M2)")),
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
    match EvidenceKind::parse(&ev.kind) {
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
        _ => {}
    }
    let id = ids::short_id("ev");
    let record = EvidenceRecord {
        id: id.clone(),
        requirement: ev.requirement.clone(),
        request: ev.request,
        kind: ev.kind.clone(),
        collected_by: ev.collected_by,
        note: ev.note,
        artifact: ev.artifact,
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
    let _ = run;
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
    let (spec_id, _) = store.run_by_id(run_id)?;
    let id = ids::short_id("fd");
    let record = FindingRecord {
        id: id.clone(),
        requirement: f.requirement,
        task: f.task,
        persona: f.persona,
        severity: f.severity,
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
    requirement_set_status(store, &args.run, &args.input)
}

#[derive(Debug, Deserialize)]
struct StatusInput {
    updates: Vec<RequirementUpdate>,
}

fn requirement_set_status(store: &Store, run_id: &str, input: &str) -> Result<Value> {
    let payload: StatusInput = read_input(input)?;
    let (spec_id, run) = store.run_by_id(run_id)?;

    // Validate each update's prerequisites before recording anything.
    for u in &payload.updates {
        validate_status_update(u, run.risk)?;
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
fn validate_status_update(u: &RequirementUpdate, tier: RiskTier) -> Result<()> {
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
            if matches!(tier, RiskTier::High | RiskTier::Critical)
                && u.residual_risk.as_deref().map(str::trim).unwrap_or("").is_empty()
            {
                return Err(SpeccyError::validation(format!(
                    "review_passed at {tier:?} requires a residual_risk note for {}",
                    u.requirement
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
            if u.note.as_deref().map(str::trim).unwrap_or("").is_empty() {
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

// --------------------------------------------------------------------------
// Packet operations
// --------------------------------------------------------------------------

fn packet(store: &Store, op: PacketOp) -> Result<Value> {
    match op {
        PacketOp::Planning(args) => packets::planning(store, &args.spec),
        PacketOp::Task(args) => packets::task(store, &args.run, &args.task),
        PacketOp::Verification(args) => packets::verification(store, &args.run, &args.requirements),
        PacketOp::Review(args) => packets::review(store, &args.run),
        PacketOp::Escalation(_) => Err(SpeccyError::not_implemented("packet escalation (M3)")),
    }
}

// --------------------------------------------------------------------------
// Shared helpers
// --------------------------------------------------------------------------

fn lint_value(findings: &[Finding]) -> Value {
    json!({
        "clean": findings.is_empty(),
        "findings": findings.iter().map(|f| json!({
            "code": f.code,
            "path": f.path,
            "message": f.message,
        })).collect::<Vec<_>>(),
    })
}

fn strip_draft(revision: &str) -> &str {
    revision.strip_suffix("-draft").unwrap_or(revision)
}

fn display_rev(rev: &crate::projection::Revision) -> String {
    if rev.approved {
        rev.id.clone()
    } else {
        format!("{}-draft", rev.id)
    }
}

fn next_revision_id(spec: &SpecState) -> String {
    format!("spec_rev_{:03}", spec.revisions.len() + 1)
}
