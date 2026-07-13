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
use speccy_core::model::RiskTier;
use speccy_core::model::RunDecisionKind;
use speccy_core::model::RunState;
use speccy_core::model::SpecDecisionKind;
use speccy_core::model::SpecDraft;
use speccy_core::mutation;
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
                // Cross-log convergence: an exact retry of a recorded
                // superseding approval finishes the linked run cancellation
                // without duplicating either decision (DESIGN § Amendment at
                // the Escalation Gate). Anything else is refused — an
                // approved revision is immutable.
                return retry_superseding_approval(store, &spec, &decision, target);
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

            // Amendment at a gate: the recorded approval is the durable
            // intent; the parked run is then closed with a single superseded
            // decision whose replay cancels it (DESIGN § Amendment at the
            // Escalation Gate).
            let mut superseded_run = None;
            if let Some((run_spec_id, run_id)) = superseded {
                supersede_run(store, &run_spec_id, &run_id, target)?;
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

/// Close a superseded run with its single linking decision; replay applies
/// the cancellation (DESIGN § Amendment at the Escalation Gate).
fn supersede_run(store: &Store, run_spec_id: &str, run_id: &str, revision: &str) -> Result<()> {
    store.append_run_event(
        run_spec_id,
        run_id,
        Event::RunDecision {
            decision: RunDecisionRecord {
                decision_id: ids::short_id("dec"),
                kind: RunDecisionKind::Superseded,
                requirement: None,
                task: None,
                actor: "human".into(),
                reason: Some(format!("superseded by revision {revision}")),
                residual_risk: None,
                carry_forward: false,
                snapshot: None,
            },
        },
    )
}

/// An `approve` against an already-approved revision is refused unless it is
/// the exact retry of a recorded superseding approval, in which case it
/// finishes the linked run cancellation and duplicates neither decision
/// (DESIGN § Amendment at the Escalation Gate).
fn retry_superseding_approval(
    store: &Store,
    spec: &SpecState,
    decision: &SpecDecisionInput,
    target: &str,
) -> Result<Value> {
    let refusal = || {
        SpeccyError::invalid_transition(format!(
            "{target} is already approved; an approved revision is immutable"
        ))
    };
    let Some(run_id) = decision
        .supersedes
        .as_ref()
        .and_then(|s| s.run_id.as_deref())
    else {
        return Err(refusal());
    };
    let recorded = spec.decisions.iter().any(|prior| {
        prior.kind == SpecDecisionKind::Approve
            && prior.revision_id == target
            && prior.supersedes.as_ref().and_then(|s| s.run_id.as_deref()) == Some(run_id)
    });
    if !recorded {
        return Err(refusal());
    }
    let (run_spec_id, _) = store.find_run(run_id)?;
    let run = store.run_projection(&run_spec_id, run_id)?;
    if run.state != RunState::Cancelled {
        supersede_run(store, &run_spec_id, run_id, target)?;
    }
    Ok(json!({
        "approved_revision": target,
        "spec_status": "approved",
        "requirements_frozen": true,
        "superseded_run": run_id,
        "next": "Run /speccy-implement (fresh session recommended).",
    }))
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
            let decision: mutation::RunDecisionInput = read_input(&args.input)?;
            mutation::record_decision(store, &args.run, args.lease.as_deref(), &decision)
        }
        RunOp::RecordShip(args) => {
            let change_ref: ChangeRef = read_input(&args.input)?;
            mutation::record_ship(store, &args.run, args.lease.as_deref(), &change_ref)
        }
        RunOp::Interrupt(args) => {
            let payload: InterruptInput = read_input(&args.input)?;
            mutation::interrupt(
                store,
                &args.run,
                args.lease.as_deref(),
                &payload.reason,
                payload.detail.as_deref(),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
struct InterruptInput {
    reason: String,
    detail: Option<String>,
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

    // Parse the tier fail-closed at intake: an invalid declared value must
    // never be stored (or later replayed) as `standard`.
    let risk = match rev.draft.risk.as_deref() {
        None => RiskTier::Standard,
        Some(s) => RiskTier::parse(s).ok_or_else(|| {
            SpeccyError::validation(format!("revision {target} has invalid risk tier `{s}`"))
        })?,
    };
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
        TaskOp::Claim(args) => {
            mutation::claim_task(store, &args.run, &args.task, &args.agent, &args.lease)
        }
        TaskOp::RecordHandoff(args) => {
            let handoff: Handoff = read_input(&args.input)?;
            mutation::record_handoff(store, &args.run, args.lease.as_deref(), handoff)
        }
    }
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
        repo: None,
        control: None,
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
        run.require_requirement(requirement)?;
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
    let payload: StatusInput = read_input(&args.input)?;
    mutation::set_requirement_status(store, &args.run, args.lease.as_deref(), payload.updates)
}

#[derive(Debug, Deserialize)]
struct StatusInput {
    updates: Vec<RequirementUpdate>,
}

fn validate_evidence_reference(
    store: &Store,
    run: &RunProjection,
    ev: &EvidenceInput,
    kind: EvidenceKind,
) -> Result<()> {
    run.require_requirement(&ev.requirement)?;
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
