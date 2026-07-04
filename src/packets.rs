//! Deterministic work-order packets (DESIGN § Planning Packet, § Task,
//! § Verification, § Review Packet). Packets are controller-assembled JSON;
//! `packet review`/`packet escalation` also carry rendered human text in a
//! `markdown` field. No packet calls an LLM.

use serde_json::{json, Value};

use crate::config::ProjectConfig;
use crate::error::{Result, SpeccyError};
use crate::gitx;
use crate::model::{RequirementStatus, SpecDraft};
use crate::projection::{RunProjection, SpecState};
use crate::store::Store;

/// `packet planning` — deterministic planning work order.
pub fn planning(store: &Store, spec_ref: &str) -> Result<Value> {
    let spec = store.spec_state_by_ref(spec_ref)?;
    let config = ProjectConfig::load(&store.workspace_root)?;
    let draft_state = match spec.latest_revision() {
        None => "empty",
        Some(r) if r.approved => "approved",
        Some(_) => "draft",
    };
    let head = gitx::head(&store.git_root).unwrap_or_default();
    let branch = gitx::current_branch(&store.git_root).unwrap_or_default();
    let dirty = gitx::is_dirty(&store.git_root).unwrap_or(false);

    Ok(json!({
        "request": spec.request,
        "brainstorm_handoff": spec.brainstorm_handoff,
        "draft_state": draft_state,
        "workspace": {
            "git": { "head": head, "branch": branch, "dirty": dirty },
            "signals": project_signals(store),
        },
        // Prior-context candidates over active specs arrive at M3.
        "prior_context_candidates": [],
        "policy": {
            "risk_default": config.risk_default,
            "task_repair_cap": config.caps.task_repair_rounds,
            "run_review_cap": config.caps.run_review_rounds,
        },
        "output_contract": {
            "submit_with": "spec record-draft",
            "required": ["goal", "scope", "risk", "requirements", "tasks"],
        },
    }))
}

/// `packet task` — task packet scoped to linked requirements.
pub fn task(store: &Store, run_id: &str, task_id: &str) -> Result<Value> {
    let (_, run) = store.run_by_id(run_id)?;
    let draft = run_draft(store, &run)?;
    let task = run
        .task(task_id)
        .ok_or_else(|| SpeccyError::not_found(format!("no task {task_id} in run {run_id}")))?;

    let requirements: Vec<Value> = task
        .requirements
        .iter()
        .filter_map(|id| draft.requirement(id))
        .map(requirement_json)
        .collect();

    Ok(json!({
        "task": task.id,
        "round": task.round.max(1),
        "baseline_commit": task.baseline_commit,
        "requirements": requirements,
        "constraints": task.constraints,
        "seed_feedback": task.seed_feedback,
        "prior_findings": prior_findings(&run, task_id),
        "handoff_contract": { "record_with": "task record-handoff" },
    }))
}

/// `packet verification` — verification packet naming the persona roster.
pub fn verification(store: &Store, run_id: &str, requirements: &[String]) -> Result<Value> {
    let (_, run) = store.run_by_id(run_id)?;
    let config = ProjectConfig::load(&store.workspace_root)?;

    // Task scope if the requirements belong to a single in-review/building task.
    let task = run.tasks.iter().find(|t| {
        matches!(
            t.status,
            crate::model::TaskStatus::InReview | crate::model::TaskStatus::Building
        )
    });
    let (scope, baseline, round, handoff) = match task {
        Some(t) => {
            let baseline = t
                .baseline_commit
                .clone()
                .unwrap_or_else(|| run.base_commit.clone());
            let handoff = run
                .handoffs
                .iter()
                .rfind(|h| h.task == t.id)
                .map(|h| h.handoff_id.clone());
            (
                json!({ "task": t.id, "requirements": t.requirements }),
                baseline,
                t.round,
                handoff,
            )
        }
        None => (
            json!({ "requirements": run.requirements.keys().cloned().collect::<Vec<_>>() }),
            run.base_commit.clone(),
            run.run_review_rounds_completed + 1,
            None,
        ),
    };

    let diff = gitx::worktree_stat(&store.git_root, &baseline).unwrap_or(gitx::DiffStat {
        files: 0,
        insertions: 0,
        deletions: 0,
    });

    // The controller runs the provenance scan inside `run next` (the mutation
    // point); the packet reports the findings that scan recorded this round.
    let prov: Vec<&crate::event::FindingRecord> = run
        .findings
        .iter()
        .map(|(_, f)| f)
        .filter(|f| {
            f.recorded_by == "controller:provenance-scan"
                && match task {
                    Some(t) => f.task.as_deref() == Some(&t.id),
                    None => f.task.is_none(),
                }
        })
        .collect();

    let _ = requirements; // requested scope is advisory; packet reports the active task/run scope
    Ok(json!({
        "scope": scope,
        "round": round,
        "handoff": handoff,
        "personas": config.roster_for(run.risk),
        "diff": {
            "baseline": baseline,
            "files": diff.files,
            "insertions": diff.insertions,
            "deletions": diff.deletions,
        },
        "prior_findings": task.map(|t| prior_findings(&run, &t.id)).unwrap_or_default(),
        "provenance_scan": {
            "hits": prov.len(),
            "findings": prov.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        },
        "tools": ["evidence collect", "evidence record", "finding record"],
    }))
}

/// `packet review` — human-facing review packet (structured + `markdown`).
pub fn review(store: &Store, run_id: &str) -> Result<Value> {
    let (_, run) = store.run_by_id(run_id)?;
    let spec = store.spec_state(&run.spec_id)?;
    let markdown = render_review_markdown(&spec, &run, store);

    let mut proven = 0;
    let mut accepted = 0;
    let mut needs_you = 0;
    for r in run.requirements.values() {
        match r.status {
            RequirementStatus::Passed => proven += 1,
            RequirementStatus::ReviewPassed | RequirementStatus::Waived => accepted += 1,
            _ => needs_you += 1,
        }
    }

    Ok(json!({
        "spec_ref": run.spec_ref,
        "title": spec.title,
        "run_state": run.state,
        "risk": run.risk,
        "buckets": { "proven": proven, "accepted_risk": accepted, "needs_you": needs_you },
        "markdown": markdown,
    }))
}

// --- helpers ---

fn run_draft(store: &Store, run: &RunProjection) -> Result<SpecDraft> {
    let spec = store.spec_state(&run.spec_id)?;
    spec.revision(&run.revision_id)
        .map(|r| r.draft.clone())
        .ok_or_else(|| SpeccyError::not_found(format!("revision {} not found", run.revision_id)))
}

fn requirement_json(req: &crate::model::Requirement) -> Value {
    serde_json::to_value(req).unwrap_or(Value::Null)
}

fn prior_findings(run: &RunProjection, task_id: &str) -> Vec<Value> {
    run.findings
        .iter()
        .filter(|(_, f)| {
            f.task.as_deref() == Some(task_id)
                || f.requirement
                    .as_ref()
                    .and_then(|r| run.task(task_id).map(|t| t.requirements.contains(r)))
                    .unwrap_or(false)
        })
        .map(|(_, f)| json!({ "id": f.id, "severity": f.severity, "note": f.note }))
        .collect()
}

/// Deterministically parsed project signals (scripts, language).
fn project_signals(store: &Store) -> Value {
    let root = &store.workspace_root;
    let mut scripts: Vec<String> = Vec::new();
    let mut language: Option<&str> = None;

    if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
        language = Some("typescript");
        if let Ok(pkg) = serde_json::from_str::<Value>(&text) {
            if let Some(obj) = pkg.get("scripts").and_then(|s| s.as_object()) {
                scripts = obj.keys().map(|k| format!("npm run {k}")).collect();
            }
        }
    } else if root.join("Cargo.toml").exists() {
        language = Some("rust");
        scripts = vec!["cargo build".into(), "cargo test".into()];
    }

    json!({ "scripts": scripts, "language": language })
}

fn render_review_markdown(spec: &SpecState, run: &RunProjection, store: &Store) -> String {
    let title = spec.title.as_deref().unwrap_or("(untitled)");
    let total = run.requirements.len();
    let proven = run
        .requirements
        .values()
        .filter(|r| r.status == RequirementStatus::Passed)
        .count();
    let accepted: Vec<(&String, &crate::projection::ReqRuntime)> = run
        .requirements
        .iter()
        .filter(|(_, r)| {
            matches!(
                r.status,
                RequirementStatus::ReviewPassed | RequirementStatus::Waived
            )
        })
        .collect();

    let diff = gitx::diff_stat(&store.git_root, &run.base_commit).unwrap_or(gitx::DiffStat {
        files: 0,
        insertions: 0,
        deletions: 0,
    });
    let tasks_done = run
        .tasks
        .iter()
        .filter(|t| t.status == crate::model::TaskStatus::Integrated)
        .count();

    let mut out = String::new();
    out.push_str(&format!(
        "Spec   {}  {}      Risk: {}\n",
        run.spec_ref,
        title,
        risk_str(run.risk)
    ));
    let accepted_note = if accepted.is_empty() {
        "ready to ship".to_string()
    } else {
        format!("ready to ship · {} accepted risk", accepted.len())
    };
    out.push_str(&format!("Result verified — {accepted_note}\n"));
    out.push_str("Recommended next action: /speccy-ship\n\n");
    out.push_str(&format!("Requirements ({total})\n"));
    out.push_str(&format!("  Proven          {proven}\n"));
    if !accepted.is_empty() {
        out.push_str(&format!("  Accepted risk   {}\n", accepted.len()));
        out.push_str("\nAccepted risk\n");
        for (id, r) in &accepted {
            let label = match r.status {
                RequirementStatus::Waived => "waived",
                _ => "review-only evidence",
            };
            let note = r
                .residual_risk
                .as_deref()
                .or(r.note.as_deref())
                .unwrap_or("");
            out.push_str(&format!("  {id}  {label}  {note}\n"));
        }
    }
    out.push_str(&format!(
        "\nChanged  {} files  +{} -{}     {} tasks\n",
        diff.files, diff.insertions, diff.deletions, tasks_done
    ));
    out.push_str("Evidence + full diff:  speccy review --evidence\n");
    out
}

fn risk_str(r: crate::model::RiskTier) -> &'static str {
    match r {
        crate::model::RiskTier::Minimal => "minimal",
        crate::model::RiskTier::Standard => "standard",
        crate::model::RiskTier::High => "high",
        crate::model::RiskTier::Critical => "critical",
    }
}
