//! Deterministic work-order packets (DESIGN § Planning Packet, § Task,
//! § Verification, § Review Packet). Packets are controller-assembled JSON;
//! `packet review`/`packet escalation` also carry rendered human text in a
//! `markdown` field. No packet calls an LLM.

use crate::config::ProjectConfig;
use crate::error::Result;
use crate::error::SpeccyError;
use crate::gitx;
use crate::model::FindingSeverity;
use crate::model::RequirementStatus;
use crate::model::RunDecisionKind;
use crate::model::RunState;
use crate::model::SpecStatus;
use crate::projection::HandoffMeta;
use crate::projection::RunProjection;
use crate::projection::SpecState;
use crate::store::Store;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Prior-context candidates: carry-forward hints from other active specs
/// (DESIGN § Carry-Forward Decisions). Archived/cancelled/superseded specs
/// leave planning context.
fn prior_context_candidates(store: &Store, current_ref: &str) -> Result<Value> {
    let mut out = Vec::new();
    for (spec_id, spec_ref) in store.list_specs()? {
        if spec_ref == current_ref {
            continue;
        }
        let spec = store.spec_state(&spec_id)?;
        if matches!(
            spec.status,
            SpecStatus::Cancelled | SpecStatus::Superseded | SpecStatus::Archived
        ) {
            continue;
        }
        let hints: Vec<String> = spec
            .decisions
            .iter()
            .filter(|d| d.carry_forward)
            .map(|d| {
                format!(
                    "{}: {}",
                    d.decision_id,
                    d.note
                        .clone()
                        .unwrap_or_else(|| d.kind.as_str().to_string())
                )
            })
            .collect();
        out.push(json!({
            "spec_ref": spec.spec_ref,
            "title": spec.title,
            "status": spec.status,
            "hints": hints,
        }));
    }
    Ok(Value::Array(out))
}

/// `packet escalation` — focused handback scoped to the requirement(s) the run
/// could not satisfy or prove (DESIGN § Escalation Packet).
///
/// # Errors
///
/// Returns an error if the run or its spec cannot be loaded from the store.
pub fn escalation(store: &Store, run_id: &str) -> Result<Value> {
    let (_, run) = store.run_by_id(run_id)?;
    // Lenient on purpose: the escalation packet must render even if the pinned
    // revision is missing, so a lost draft falls back to an empty one.
    let draft = store.run_draft(&run).unwrap_or_default();

    let failing: Vec<(String, String)> = run
        .requirements
        .iter()
        .filter(|(_, r)| !r.status.is_resolved())
        .map(|(id, _)| {
            let statement = draft
                .requirement(id)
                .and_then(|r| r.statement.clone())
                .unwrap_or_default();
            (id.clone(), statement)
        })
        .collect();

    let tried: Vec<Value> = dedup_handoffs(&run.handoffs)
        .into_iter()
        .map(|h| {
            let rejected: Vec<String> = run
                .findings
                .iter()
                .map(|(_, f)| f)
                .filter(|f| {
                    f.severity == FindingSeverity::Blocking
                        && (f.task.as_deref() == Some(&h.task)
                            || f.requirement
                                .as_ref()
                                .is_some_and(|r| failing.iter().any(|(fid, _)| fid == r)))
                })
                .map(|f| f.note.clone())
                .collect();
            json!({ "round": h.round, "summary": h.handoff.summary, "rejected": rejected })
        })
        .collect();

    // Recorded run decisions ride the escalation packet (DESIGN § Escalation
    // Packet): notably the interrupt reason, the one give-up the controller
    // cannot detect itself.
    let decisions: Vec<Value> = run
        .decisions
        .iter()
        .map(|d| {
            json!({
                "id": d.decision_id,
                "type": d.kind.as_str(),
                "reason": d.reason,
                "requirement": d.requirement,
                "task": d.task,
            })
        })
        .collect();

    let markdown = render_escalation_markdown(&run, &failing, &tried);
    Ok(json!({
        "failing": failing.iter().map(|(id, st)| json!({ "id": id, "statement": st })).collect::<Vec<_>>(),
        "tried": tried,
        "decisions": decisions,
        "partial_work": run.last_snapshot,
        "recommended": "amend the spec",
        "alternatives": ["provide setup", "waive this requirement", "cancel the run"],
        "markdown": markdown,
    }))
}

/// Collapse handoffs to the latest one per `(task, round)`, preserving the
/// order each round was first attempted. A resume can re-open a task at its
/// same round (DESIGN § Escalation Gate), appending a second handoff for that
/// `(task, round)`; the escalation "tried" list is per attempt-round, so those
/// duplicates must not produce duplicate rows.
fn dedup_handoffs(handoffs: &[HandoffMeta]) -> Vec<&HandoffMeta> {
    let mut order: Vec<(&str, u32)> = Vec::new();
    let mut latest: HashMap<(&str, u32), &HandoffMeta> = HashMap::new();
    for h in handoffs {
        let key = (h.task.as_str(), h.round);
        if latest.insert(key, h).is_none() {
            order.push(key);
        }
    }
    // Each key in `order` was inserted into `latest`, so `get` always hits.
    order
        .into_iter()
        .filter_map(|key| latest.get(&key).copied())
        .collect()
}

fn render_escalation_markdown(
    run: &RunProjection,
    failing: &[(String, String)],
    tried: &[Value],
) -> String {
    let ids = failing
        .iter()
        .map(|(id, _)| id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("Speccy stopped because {ids} could not be proven.\n\n");
    if !tried.is_empty() {
        out.push_str("Tried:\n");
        for t in tried {
            let round = t["round"].as_u64().unwrap_or(0);
            let summary = t["summary"].as_str().unwrap_or("");
            let rejected = t["rejected"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_default();
            if rejected.is_empty() {
                // writing to a String is infallible
                _ = writeln!(out, "  round {round} — {summary}");
            } else {
                _ = writeln!(out, "  round {round} — {summary}   (rejected: {rejected})");
            }
        }
        out.push('\n');
    }
    // Surface the interrupt reason (structured-output retry exhaustion): the
    // one give-up the controller cannot detect, recorded as a run decision.
    let interrupts: Vec<&str> = run
        .decisions
        .iter()
        .filter(|d| d.kind == RunDecisionKind::Interrupt)
        .filter_map(|d| d.reason.as_deref())
        .collect();
    if !interrupts.is_empty() {
        out.push_str("Interrupted:\n");
        for reason in interrupts {
            _ = writeln!(out, "  {reason}");
        }
        out.push('\n');
    }
    if let Some(snap) = &run.last_snapshot {
        _ = writeln!(
            out,
            "Partial work applied: snapshot {snap} on {}.",
            run.branch
        );
        out.push('\n');
    }
    out.push_str("Recommended: amend the spec\n");
    out.push_str("Alternatives: provide setup, waive this requirement, cancel the run\n");
    out
}

/// `packet planning` — deterministic planning work order.
///
/// # Errors
///
/// Returns an error if the spec cannot be loaded or the project config fails
/// to load.
pub fn planning(store: &Store, spec_ref: &str) -> Result<Value> {
    let spec = store.spec_state_by_ref(spec_ref)?;
    let config = ProjectConfig::load(&store.workspace_root)?;
    let draft_state = match spec.latest_revision() {
        None => "empty",
        Some(r) if r.approved => "approved",
        Some(_) => "draft",
    };
    // Advisory workspace facts degrade to structured `null`s with a visible
    // warning, never a fabricated empty string or false-clean (DESIGN §
    // Planning Packet and Draft Contract).
    let mut warnings: Vec<String> = Vec::new();
    let head = fact_or_warn(gitx::head(&store.git_root), "git head", &mut warnings);
    let branch = fact_or_warn(
        gitx::current_branch(&store.git_root),
        "git branch",
        &mut warnings,
    );
    let dirty = fact_or_warn(
        gitx::is_dirty(&store.git_root),
        "git dirty state",
        &mut warnings,
    );

    Ok(json!({
        "request": spec.request,
        "brainstorm_handoff": spec.brainstorm_handoff,
        "draft_state": draft_state,
        "workspace": {
            "git": { "head": head, "branch": branch, "dirty": dirty },
            "warnings": warnings,
            "signals": project_signals(store),
        },
        "prior_context_candidates": prior_context_candidates(store, &spec.spec_ref)?,
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
///
/// # Errors
///
/// Returns an error if the run cannot be loaded, its draft is missing, or
/// `task_id` does not name a task in the run.
pub fn task(store: &Store, run_id: &str, task_id: &str) -> Result<Value> {
    let (_, run) = store.run_by_id(run_id)?;
    let draft = store.run_draft(&run)?;
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
///
/// # Errors
///
/// Returns an error if the run cannot be loaded or the project config fails
/// to load.
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
            run.run_review_round.max(1),
            None,
        ),
    };

    // The diff scope is required for verification correctness: an unreadable
    // diff fails the packet rather than reporting a fabricated zero diff
    // (DESIGN § Planning Packet and Draft Contract).
    let diff = gitx::worktree_stat(&store.git_root, &baseline)?;

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
        "command_evidence_anomalies": evidence_identity_anomalies(&run),
        "control_outcomes": control_outcomes(&run),
        "tools": ["evidence collect", "evidence record", "finding record"],
    }))
}

/// `packet review` — human-facing review packet (structured + `markdown`).
///
/// # Errors
///
/// Returns an error if the run or its spec cannot be loaded from the store.
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

fn requirement_json(req: &crate::model::Requirement) -> Value {
    serde_json::to_value(req).unwrap_or(Value::Null)
}

/// An advisory git fact, or `None` plus a visible warning when unavailable.
fn fact_or_warn<T>(fact: Result<T>, name: &str, warnings: &mut Vec<String>) -> Option<T> {
    match fact {
        Ok(v) => Some(v),
        Err(e) => {
            warnings.push(format!("{name} unavailable: {}", e.message));
            None
        }
    }
}

/// Command-evidence records whose captured repository identity shows the
/// command changed HEAD or dirtied new paths — surfaced in verification and
/// review packets (DESIGN § Acceptance Ledger).
fn evidence_identity_anomalies(run: &RunProjection) -> Vec<Value> {
    run.evidence
        .iter()
        .filter_map(|ev| {
            let repo = ev.repo.as_ref()?;
            if !repo.head_changed && repo.newly_dirty.is_empty() {
                return None;
            }
            Some(json!({
                "evidence": ev.id,
                "requirement": ev.requirement,
                "head_changed": repo.head_changed,
                "head_before": repo.head_before,
                "head_after": repo.head_after,
                "newly_dirty": repo.newly_dirty,
            }))
        })
        .collect()
}

/// Every recorded fail-before/pass-after control outcome — surfaced in the
/// verification packet; non-passed outcomes are called out in the review
/// packet (DESIGN § Acceptance Ledger).
fn control_outcomes(run: &RunProjection) -> Vec<Value> {
    run.evidence
        .iter()
        .filter_map(|ev| {
            let control = ev.control.as_ref()?;
            Some(json!({
                "evidence": ev.id,
                "requirement": ev.requirement,
                "status": control.status,
                "isolation_cleanup": control.isolation.cleanup,
                "note": control.note,
            }))
        })
        .collect()
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

    if let Ok(text) = fs_err::read_to_string(root.join("package.json")) {
        language = Some("typescript");
        if let Ok(pkg) = serde_json::from_str::<Value>(&text)
            && let Some(obj) = pkg.get("scripts").and_then(|s| s.as_object())
        {
            scripts = obj.keys().map(|k| format!("npm run {k}")).collect();
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

    let diff = gitx::diff_stat(&store.git_root, &run.base_commit);
    let tasks_done = run
        .tasks
        .iter()
        .filter(|t| t.status == crate::model::TaskStatus::Integrated)
        .count();

    let mut out = String::new();
    _ = writeln!(
        out,
        "Spec   {}  {}      Risk: {}",
        run.spec_ref,
        title,
        run.risk.as_str()
    );
    // The packet is honest about non-verified runs: an escalated or policy-gated
    // run shows the unresolved requirements, not "ready to ship" (DESIGN §
    // Review Packet).
    let needs_you: Vec<(&String, &crate::projection::ReqRuntime)> = run
        .requirements
        .iter()
        .filter(|(_, r)| !r.status.is_resolved())
        .collect();
    if run.state == RunState::Verified {
        let accepted_note = if accepted.is_empty() {
            "ready to ship".to_string()
        } else {
            format!("ready to ship · {}", accepted_risk_phrase(accepted.len()))
        };
        _ = writeln!(out, "Result verified — {accepted_note}");
        out.push_str("Recommended next action: /speccy-ship\n\n");
    } else {
        _ = writeln!(
            out,
            "Result {} — {} unresolved requirement(s)",
            run.state.as_str(),
            needs_you.len()
        );
        out.push_str("Recommended next action: speccy review\n\n");
    }
    _ = writeln!(out, "Requirements ({total})");
    _ = writeln!(out, "  Proven          {proven}");
    if !needs_you.is_empty() {
        _ = writeln!(out, "  Needs you       {}", needs_you.len());
    }
    if !accepted.is_empty() {
        _ = writeln!(out, "  Accepted risk   {}", accepted.len());
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
            _ = writeln!(out, "  {id}  {label}  {note}");
        }
    }
    if !needs_you.is_empty() {
        out.push_str("\nNeeds you\n");
        for (id, r) in &needs_you {
            _ = writeln!(out, "  {id}  {}", req_status_label(r.status));
        }
    }
    render_evidence_anomalies(&mut out, run);
    render_failed_controls(&mut out, run);
    // Honest about an unreadable diff: never a fabricated zero (DESIGN §
    // Planning Packet and Draft Contract).
    match diff {
        Ok(d) => {
            _ = writeln!(
                out,
                "\nChanged  {} files  +{} -{}     {} tasks",
                d.files, d.insertions, d.deletions, tasks_done
            );
        }
        Err(e) => {
            _ = writeln!(
                out,
                "\nChanged  unavailable — git diff failed: {}     {} tasks",
                e.message, tasks_done
            );
        }
    }
    out.push_str("Evidence + full diff:  speccy review --evidence\n");
    out
}

/// Append the command-evidence identity anomalies (changed HEAD, newly dirty
/// paths) to the review markdown so accepted evidence that mutated the
/// workspace is visible at the gate.
fn render_evidence_anomalies(out: &mut String, run: &RunProjection) {
    let anomalies = evidence_identity_anomalies(run);
    if anomalies.is_empty() {
        return;
    }
    out.push_str("\nCommand evidence changed the workspace\n");
    for a in &anomalies {
        let mut parts = Vec::new();
        if a["head_changed"] == Value::Bool(true) {
            parts.push("changed HEAD".to_string());
        }
        if let Some(paths) = a["newly_dirty"].as_array()
            && !paths.is_empty()
        {
            let list: Vec<&str> = paths.iter().filter_map(Value::as_str).collect();
            parts.push(format!("newly dirty: {}", list.join(", ")));
        }
        _ = writeln!(
            out,
            "  {}  {}",
            a["evidence"].as_str().unwrap_or("?"),
            parts.join("; ")
        );
    }
}

/// Append every fail-before/pass-after control that did not pass, so a
/// vacuous or unprovable control is visible at the review gate.
fn render_failed_controls(out: &mut String, run: &RunProjection) {
    let outcomes = control_outcomes(run);
    let not_passed: Vec<&Value> = outcomes
        .iter()
        .filter(|o| o["status"] != json!("passed"))
        .collect();
    if not_passed.is_empty() {
        return;
    }
    out.push_str("\nFail-before/pass-after controls not proven\n");
    for o in &not_passed {
        _ = writeln!(
            out,
            "  {}  {}  {}",
            o["evidence"].as_str().unwrap_or("?"),
            o["status"].as_str().unwrap_or("?"),
            o["note"].as_str().unwrap_or("")
        );
    }
}

/// "1 accepted risk" / "2 accepted risks" (DESIGN § Review UX). Shared with the
/// human status card.
#[must_use = "formats a phrase that must be used"]
pub fn accepted_risk_phrase(n: usize) -> String {
    if n == 1 {
        "1 accepted risk".to_string()
    } else {
        format!("{n} accepted risks")
    }
}

/// Human-facing requirement-status label ("review-only evidence" for
/// `review_passed`), shared with the human CLI's evidence drill-down.
#[must_use = "returns the label without side effects"]
pub fn req_status_label(s: RequirementStatus) -> &'static str {
    match s {
        RequirementStatus::Pending => "pending",
        RequirementStatus::Passed => "passed",
        RequirementStatus::ReviewPassed => "review-only evidence",
        RequirementStatus::Failed => "failed",
        RequirementStatus::Blocked => "blocked",
        RequirementStatus::Waived => "waived",
    }
}
