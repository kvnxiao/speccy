//! Human-facing commands (DESIGN § CLI/Admin Flow). These render text, never
//! the controller machinery: no directives, leases, run IDs, or ctl operations
//! on the card. M1 ships `status` and `review`; the rest arrive at M3.

use crate::error::{Result, SpeccyError};
use crate::model::{RequirementStatus, RiskTier, RunState, SpecStatus};
use crate::packets;
use crate::projection::{RunProjection, SpecState};
use crate::store::Store;

/// `speccy status` — one card per active run in the workspace.
pub fn status(store: &Store) -> Result<String> {
    let mut cards = Vec::new();
    for (spec_id, _) in store.list_specs()? {
        let spec = store.spec_state(&spec_id)?;
        for run_id in store.list_runs(&spec_id)? {
            let run = store.run_projection(&spec_id, &run_id)?;
            if is_notable(run.state) {
                cards.push(status_card(&spec, &run));
            }
        }
    }
    if cards.is_empty() {
        Ok("No active runs.".to_string())
    } else {
        Ok(cards.join("\n\n"))
    }
}

/// `speccy review [selector] [--evidence]` — the state-aware human packet.
pub fn review(store: &Store, selector: Option<&str>, evidence: bool) -> Result<String> {
    let spec = resolve_spec(store, selector)?;
    let runs = store.list_runs(&spec.spec_id)?;
    let Some(run_id) = runs.last() else {
        return Ok(spec_card(&spec));
    };
    let run = store.run_projection(&spec.spec_id, run_id)?;
    let mut out = match run.state {
        RunState::Verified => {
            let packet = packets::review(store, run_id)?;
            packet["markdown"].as_str().unwrap_or("").to_string()
        }
        RunState::Escalated => escalation_summary(&run),
        RunState::Submitted => close_out_card(&spec, &run),
        RunState::Landed => accepted_summary(&spec, &run),
        RunState::Cancelled => format!("{}  {} — run cancelled", run.spec_ref, title_of(&spec)),
        _ => status_card(&spec, &run),
    };
    if evidence {
        out.push_str("\n\n");
        out.push_str(&evidence_drilldown(&run));
    }
    Ok(out)
}

// --- rendering ---

fn is_notable(state: RunState) -> bool {
    matches!(
        state,
        RunState::Implementing
            | RunState::Verifying
            | RunState::Verified
            | RunState::Escalated
            | RunState::Submitted
    )
}

fn status_card(spec: &SpecState, run: &RunProjection) -> String {
    let mut card = format!(
        "{}  {}          Risk: {}\n",
        run.spec_ref,
        title_of(spec),
        risk_str(run.risk)
    );
    match run.state {
        RunState::Implementing | RunState::Verifying => {
            let (label, context) = active_context(run);
            card.push_str(&format!("  {label} — {context}\n"));
            card.push_str("  · autonomous, nothing needed\n");
            if let Some(secs) = age_seconds(run) {
                card.push_str(&format!("  Last activity {}\n", humanize_age(secs)));
            }
        }
        RunState::Verified => {
            let accepted = accepted_risk_count(run);
            let suffix = if accepted > 0 {
                format!(" · {accepted} accepted risk")
            } else {
                String::new()
            };
            card.push_str(&format!("  Ready to ship{suffix}\n"));
            card.push_str("  Next: /speccy-ship\n");
        }
        RunState::Escalated => {
            card.push_str("  Needs you — a requirement could not be proven\n");
            card.push_str("  Next: speccy review\n");
        }
        RunState::Submitted => {
            let pr = run
                .change_ref
                .as_ref()
                .and_then(|c| c.url.clone())
                .unwrap_or_else(|| "change proposed".into());
            card.push_str(&format!("  Awaiting merge — {pr}\n"));
            card.push_str("  Next: speccy accept   (after the change merges)\n");
        }
        _ => {}
    }
    card.trim_end().to_string()
}

fn active_context(run: &RunProjection) -> (&'static str, String) {
    let label = match run.state {
        RunState::Verifying => "Verifying",
        _ => "Implementing",
    };
    let context = match run.active_task() {
        Some(t) => {
            let title = t.title.clone().unwrap_or_else(|| t.id.clone());
            if t.round > 1 {
                format!("{title} · repair round {} of 3", t.round)
            } else {
                title
            }
        }
        None => "run-level validation".to_string(),
    };
    (label, context)
}

fn spec_card(spec: &SpecState) -> String {
    let Some(rev) = spec.latest_revision() else {
        return format!(
            "{}  {} — draft (no content yet)",
            spec.spec_ref,
            title_of(spec)
        );
    };
    let draft = &rev.draft;
    let mut out = format!(
        "Spec: {}  {}          Risk: {}\n",
        spec.spec_ref,
        title_of(spec),
        draft.risk.as_deref().unwrap_or("?")
    );
    let status = if rev.approved {
        "approved"
    } else {
        "draft — approve to start"
    };
    out.push_str(&format!("Status: {status}\n\n"));
    if let Some(goal) = &draft.goal {
        out.push_str(&format!("Goal: {goal}\n"));
    }
    if let Some(scope) = &draft.scope {
        if !scope.in_.is_empty() {
            out.push_str(&format!("In scope: {}\n", scope.in_.join(" · ")));
        }
        if !scope.out.is_empty() {
            out.push_str(&format!("Out of scope: {}\n", scope.out.join(" · ")));
        }
    }
    if !draft.requirements().is_empty() {
        out.push_str("\nAcceptance\n");
        for r in draft.requirements() {
            out.push_str(&format!(
                "- {}\n",
                r.statement.as_deref().unwrap_or("(no statement)")
            ));
        }
    }
    out.trim_end().to_string()
}

fn escalation_summary(run: &RunProjection) -> String {
    let failing: Vec<&String> = run
        .requirements
        .iter()
        .filter(|(_, r)| !r.status.is_resolved())
        .map(|(id, _)| id)
        .collect();
    let list = failing
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}  Needs you\nSpeccy stopped because {} could not be proven.\nReply in prose: amend the spec, provide setup, waive the requirement, or cancel.",
        run.spec_ref, list
    )
}

fn close_out_card(spec: &SpecState, run: &RunProjection) -> String {
    let pr = run
        .change_ref
        .as_ref()
        .and_then(|c| c.url.clone())
        .unwrap_or_default();
    format!(
        "{}  {}\nAwaiting merge — {}\nAfter it merges, record it with: speccy accept",
        run.spec_ref,
        title_of(spec),
        pr
    )
}

fn accepted_summary(spec: &SpecState, run: &RunProjection) -> String {
    format!("{}  {} — landed", run.spec_ref, title_of(spec))
}

fn evidence_drilldown(run: &RunProjection) -> String {
    let mut out = String::from("Ledger\n");
    for (id, r) in &run.requirements {
        out.push_str(&format!("  {id}  {}\n", req_status_str(r.status)));
        if let Some(rr) = &r.residual_risk {
            out.push_str(&format!("      residual risk: {rr}\n"));
        }
    }
    if !run.findings.is_empty() {
        out.push_str("\nFindings\n");
        for (_, f) in &run.findings {
            out.push_str(&format!("  [{}] {}\n", f.severity, f.note));
        }
    }
    out.trim_end().to_string()
}

// --- selectors ---

/// Resolve a selector to a spec: exact ref, then case-insensitive substring on
/// ref/title. Inference picks the single spec when unambiguous.
pub fn resolve_spec(store: &Store, selector: Option<&str>) -> Result<SpecState> {
    let specs = store.list_specs()?;
    if specs.is_empty() {
        return Err(SpeccyError::not_found("no specs in this workspace"));
    }
    match selector {
        None => {
            let active: Vec<_> = specs
                .iter()
                .filter_map(|(id, _)| store.spec_state(id).ok())
                .filter(|s| is_active_spec(s.status))
                .collect();
            match active.as_slice() {
                [one] => Ok(one.clone()),
                [] if specs.len() == 1 => store.spec_state(&specs[0].0),
                [] => Err(SpeccyError::ambiguous_selector(
                    "no single active spec; name one with `speccy review <ref>`",
                )),
                _ => Err(SpeccyError::ambiguous_selector(
                    "more than one active spec; name one with `speccy review <ref>`",
                )),
            }
        }
        Some(sel) => {
            if let Some((id, _)) = specs.iter().find(|(_, r)| r == sel) {
                return store.spec_state(id);
            }
            let needle = sel.to_lowercase();
            let mut matches = Vec::new();
            for (id, r) in &specs {
                let spec = store.spec_state(id)?;
                let title = spec.title.clone().unwrap_or_default().to_lowercase();
                if r.to_lowercase().contains(&needle) || title.contains(&needle) {
                    matches.push(spec);
                }
            }
            match matches.as_slice() {
                [one] => Ok(one.clone()),
                [] => Err(SpeccyError::not_found(format!("no spec matching `{sel}`"))),
                _ => Err(SpeccyError::ambiguous_selector(format!(
                    "`{sel}` matches {} specs; use the full SPEC- reference",
                    matches.len()
                ))),
            }
        }
    }
}

fn is_active_spec(status: SpecStatus) -> bool {
    matches!(status, SpecStatus::Draft | SpecStatus::Approved)
}

fn accepted_risk_count(run: &RunProjection) -> usize {
    run.requirements
        .values()
        .filter(|r| {
            matches!(
                r.status,
                RequirementStatus::ReviewPassed | RequirementStatus::Waived
            )
        })
        .count()
}

fn title_of(spec: &SpecState) -> String {
    spec.title.clone().unwrap_or_else(|| "(untitled)".into())
}

fn age_seconds(run: &RunProjection) -> Option<i64> {
    let ts = run.last_event_ts?;
    Some((jiff::Timestamp::now().as_second() - ts.as_second()).max(0))
}

fn humanize_age(secs: i64) -> String {
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

fn risk_str(r: RiskTier) -> &'static str {
    match r {
        RiskTier::Minimal => "minimal",
        RiskTier::Standard => "standard",
        RiskTier::High => "high",
        RiskTier::Critical => "critical",
    }
}

fn req_status_str(s: RequirementStatus) -> &'static str {
    match s {
        RequirementStatus::Pending => "pending",
        RequirementStatus::Passed => "passed",
        RequirementStatus::ReviewPassed => "review-only evidence",
        RequirementStatus::Failed => "failed",
        RequirementStatus::Blocked => "blocked",
        RequirementStatus::Waived => "waived",
    }
}
