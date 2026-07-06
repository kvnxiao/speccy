//! Human-facing commands (DESIGN § CLI/Admin Flow). These render text, never
//! the controller machinery: no directives, leases, run IDs, or ctl operations
//! on the card. M1 ships `status` and `review`; the rest arrive at M3.

use crate::style;
use serde_json::json;
use speccy_core::config::ProjectConfig;
use speccy_core::error::Result;
use speccy_core::error::SpeccyError;
use speccy_core::event::Event;
use speccy_core::event::SpecDecisionRecord;
use speccy_core::ids;
use speccy_core::model::RequirementStatus;
use speccy_core::model::RunState;
use speccy_core::model::SpecStatus;
use speccy_core::packets;
use speccy_core::projection::RunProjection;
use speccy_core::projection::SpecState;
use speccy_core::store::Store;
use speccy_core::store::write_atomic;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// `speccy status` — one card per active run in the workspace.
pub fn status(store: &Store) -> Result<String> {
    let mut cards = Vec::new();
    for (spec_id, _) in store.list_specs()? {
        let spec = store.spec_state(&spec_id)?;
        for run_id in store.list_runs(&spec_id)? {
            let run = store.run_projection(&spec_id, &run_id)?;
            if is_notable(run.state) {
                cards.push(status_card(store, &spec, &run));
            }
        }
    }
    if cards.is_empty() {
        Ok("No active runs.".to_string())
    } else {
        Ok(cards.join("\n\n"))
    }
}

/// An active run whose lease has expired has no live session driving it — it is
/// interrupted, and the card surfaces resume attribution (DESIGN § Resume and
/// Crash Recovery, § CLI/Admin Flow).
fn is_interrupted(store: &Store, run: &RunProjection) -> bool {
    if !matches!(run.state, RunState::Implementing | RunState::Verifying) {
        return false;
    }
    matches!(
        store.read_lease(&run.spec_id, &run.run_id),
        Ok(Some(lease)) if lease.is_expired(jiff::Timestamp::now())
    )
}

/// `speccy review [selector] [--evidence] [--json]` — the state-aware human
/// packet; `--json` returns the same state-aware view structurally.
pub fn review(
    store: &Store,
    selector: Option<&str>,
    evidence: bool,
    json_out: bool,
) -> Result<String> {
    let spec = resolve_spec(store, selector)?;
    let runs = store.list_runs(&spec.spec_id)?;
    let run = match runs.last() {
        Some(rid) => Some((rid.clone(), store.run_projection(&spec.spec_id, rid)?)),
        None => None,
    };

    if json_out {
        let value = review_value(store, &spec, run.as_ref())?;
        return Ok(serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()));
    }

    let Some((run_id, run)) = run else {
        return Ok(spec_card(&spec));
    };
    let mut out = match run.state {
        RunState::Verified => {
            let packet = packets::review(store, &run_id)?;
            packet
                .get("markdown")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        }
        // Escalated runs show the escalation packet, not a bespoke summary
        // (DESIGN § CLI/Admin Flow).
        RunState::Escalated => {
            let packet = packets::escalation(store, &run_id)?;
            packet
                .get("markdown")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        }
        RunState::Submitted => close_out_card(&spec, &run),
        RunState::Landed => accepted_summary(&spec, &run),
        RunState::Cancelled => format!(
            "{}  {} — run cancelled",
            style::paint(style::SPEC_REF, &run.spec_ref),
            title_of(&spec)
        ),
        _ => status_card(store, &spec, &run),
    };
    if evidence {
        out.push_str("\n\n");
        out.push_str(&evidence_drilldown(&run));
    }
    Ok(out)
}

/// The structural (`--json`) form of `review`, mirroring the text surface by
/// state (DESIGN § CLI/Admin Flow).
fn review_value(
    store: &Store,
    spec: &SpecState,
    run: Option<&(String, RunProjection)>,
) -> Result<serde_json::Value> {
    let Some((run_id, run)) = run else {
        let requirements: Vec<_> = spec
            .latest_revision()
            .map(|r| {
                r.draft
                    .requirements()
                    .iter()
                    .map(|req| json!({ "id": req.id, "statement": req.statement }))
                    .collect()
            })
            .unwrap_or_default();
        return Ok(json!({
            "surface": "spec_card",
            "spec_ref": spec.spec_ref,
            "title": spec.title,
            "spec_status": spec.status,
            "requirements": requirements,
        }));
    };
    Ok(match run.state {
        RunState::Verified => packets::review(store, run_id)?,
        RunState::Escalated => packets::escalation(store, run_id)?,
        RunState::Submitted => json!({
            "surface": "close_out",
            "spec_ref": run.spec_ref,
            "run_state": run.state,
            "change_ref": run.change_ref,
        }),
        RunState::Landed => json!({
            "surface": "accepted",
            "spec_ref": run.spec_ref,
            "run_state": run.state,
        }),
        _ => json!({
            "surface": "status",
            "spec_ref": run.spec_ref,
            "run_state": run.state,
            "risk": run.risk,
            "interrupted": is_interrupted(store, run),
        }),
    })
}

/// `speccy list` — active specs by default; `--query` previews matches;
/// `--json` returns the selector-resolution shape for install-pack skills.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "independent human-facing list display flags"
)]
pub fn list(
    store: &Store,
    query: Option<&str>,
    all: bool,
    accepted: bool,
    archived: bool,
    status_filter: Option<&str>,
    json_out: bool,
) -> Result<String> {
    let needle = query.map(str::to_lowercase);
    let mut rows = Vec::new();
    for (spec_id, spec_ref) in store.list_specs()? {
        let spec = store.spec_state(&spec_id)?;
        if !list_visible(spec.status, all, accepted, archived, status_filter) {
            continue;
        }
        if let Some(n) = &needle {
            let title = spec.title.clone().unwrap_or_default().to_lowercase();
            if !spec_ref.to_lowercase().contains(n) && !title.contains(n) {
                continue;
            }
        }
        rows.push(spec);
    }

    if json_out {
        let arr: Vec<_> = rows
            .iter()
            .map(|s| json!({ "spec_ref": s.spec_ref, "title": s.title, "status": s.status }))
            .collect();
        return Ok(serde_json::to_string(&json!(arr)).unwrap_or_else(|_| "[]".into()));
    }

    if rows.is_empty() {
        return Ok(match query {
            Some(q) => format!("No specs matching \"{q}\"."),
            None => "No active specs.".to_string(),
        });
    }
    let mut out = match query {
        Some(q) => format!(
            "{}\n\n",
            style::paint(style::HEADER, &format!("Specs matching \"{q}\":"))
        ),
        None => format!("{}\n\n", style::paint(style::HEADER, "Active specs:")),
    };
    for (i, s) in rows.iter().enumerate() {
        // writing to a String is infallible
        _ = writeln!(
            out,
            "{}  {}  {}   {}",
            i + 1,
            style::paint(style::SPEC_REF, &s.spec_ref),
            title_of(s),
            style::paint(style::spec_status_style(s.status), s.status.as_str())
        );
    }
    if let Some(first) = rows.first() {
        _ = write!(
            out,
            "\nUse: speccy review {}",
            style::paint(style::SPEC_REF, &first.spec_ref)
        );
    }
    Ok(out)
}

fn list_visible(
    status: SpecStatus,
    all: bool,
    accepted: bool,
    archived: bool,
    status_filter: Option<&str>,
) -> bool {
    if let Some(f) = status_filter {
        return status.as_str() == f;
    }
    if all {
        return true;
    }
    if accepted {
        return status == SpecStatus::Accepted;
    }
    if archived {
        return status == SpecStatus::Archived;
    }
    matches!(status, SpecStatus::Draft | SpecStatus::Approved)
}

/// `speccy accept` — record that a submitted run's change landed. Idempotent;
/// uses the `change_ref` recorded at ship time (DESIGN § Acceptance).
pub fn accept(
    store: &Store,
    selector: Option<&str>,
    pr: Option<&str>,
    note: Option<&str>,
) -> Result<String> {
    let spec = resolve_spec(store, selector)?;
    let runs = store.list_runs(&spec.spec_id)?;
    let mut submitted: Vec<(String, RunProjection)> = Vec::new();
    let mut already_landed = false;
    for rid in &runs {
        let run = store.run_projection(&spec.spec_id, rid)?;
        match run.state {
            RunState::Submitted => submitted.push((rid.clone(), run)),
            RunState::Landed => already_landed = true,
            _ => {}
        }
    }
    if submitted.len() > 1 {
        return Err(SpeccyError::ambiguous_selector(
            "more than one submitted run; name the spec explicitly",
        ));
    }
    let Some((run_id, run)) = submitted.into_iter().next() else {
        if already_landed {
            return Ok(format!(
                "{}  already recorded as landed.",
                style::paint(style::SPEC_REF, &spec.spec_ref)
            ));
        }
        return Err(SpeccyError::not_found("no submitted run to accept"));
    };

    let mut out = String::from("Recording landing for:\n");
    if let Some(cr) = &run.change_ref {
        if let Some(url) = &cr.url {
            _ = writeln!(out, "  {url}");
        }
        if let Some(branch) = &cr.branch {
            _ = writeln!(out, "  branch  {branch}");
        }
        if let Some(head) = &cr.head_sha {
            _ = writeln!(out, "  head    {head}");
        }
        if let Some(base) = &cr.base {
            _ = writeln!(out, "  base    {base}");
        }
    }
    if let Some(pr) = pr {
        _ = writeln!(out, "  pr  {pr}");
    }
    if let Some(note) = note {
        _ = writeln!(out, "  note  {note}");
    }

    store.append_run_event(
        &spec.spec_id,
        &run_id,
        Event::RunStateTransitioned {
            to: RunState::Landed,
            snapshot: None,
        },
    )?;
    store.append_spec_event(
        &spec.spec_id,
        Event::SpecStatusChanged {
            to: SpecStatus::Accepted,
        },
    )?;

    _ = writeln!(
        out,
        "\nRecorded: {}  {}",
        style::paint(style::SPEC_REF, &spec.spec_ref),
        title_of(&spec)
    );
    out.push_str("  run  submitted -> landed\n  spec approved  -> accepted\n");
    out.push_str("Accepted specs leave default status/list output. Show them with:\n");
    out.push_str("  speccy list --accepted");
    Ok(out)
}

/// `speccy archive` — hide a stale accepted spec from active views.
pub fn archive(store: &Store, selector: Option<&str>) -> Result<String> {
    let spec = resolve_spec_any(store, selector)?;
    // Archive is for historical specs, not routine close-out of active work
    // (DESIGN § Acceptance); refuse an active draft/approved spec.
    if is_active_spec(spec.status) {
        return Err(SpeccyError::invalid_transition(format!(
            "{} is {}; archive is for accepted/closed specs — use `speccy cancel` to stop active work",
            spec.spec_ref,
            spec.status.as_str()
        )));
    }
    store.append_spec_event(
        &spec.spec_id,
        Event::SpecStatusChanged {
            to: SpecStatus::Archived,
        },
    )?;
    Ok(format!(
        "Archived {}. It leaves accepted-spec lists; its carry-forward decisions stay recorded.",
        style::paint(style::SPEC_REF, &spec.spec_ref)
    ))
}

/// `speccy cancel` — stop the current or selected spec/run.
pub fn cancel(store: &Store, selector: Option<&str>) -> Result<String> {
    let spec = resolve_spec(store, selector)?;
    let mut cancelled_run = false;
    for rid in store.list_runs(&spec.spec_id)? {
        let run = store.run_projection(&spec.spec_id, &rid)?;
        if run.state.is_active() || run.state == RunState::Escalated {
            store.append_run_event(
                &spec.spec_id,
                &rid,
                Event::RunStateTransitioned {
                    to: RunState::Cancelled,
                    snapshot: None,
                },
            )?;
            cancelled_run = true;
        }
    }
    let latest_rev = spec
        .latest_revision()
        .map(|r| r.id.clone())
        .unwrap_or_default();
    store.append_spec_event(
        &spec.spec_id,
        Event::SpecDecision {
            decision: SpecDecisionRecord {
                decision_id: ids::short_id("dec"),
                kind: "cancel".into(),
                revision_id: latest_rev,
                actor: "human".into(),
                approved_in_prose: None,
                note: Some("cancelled via speccy cancel".into()),
                carry_forward: false,
                supersedes: None,
            },
        },
    )?;
    if cancelled_run {
        Ok(format!(
            "Cancelled {} and its active run. Recorded as a spec decision.",
            style::paint(style::SPEC_REF, &spec.spec_ref)
        ))
    } else {
        Ok(format!(
            "Cancelled {}. Recorded as a spec decision.",
            style::paint(style::SPEC_REF, &spec.spec_ref)
        ))
    }
}

/// `speccy new` — record plain intent as a draft spec, outside a harness.
pub fn new_spec(store: &Store, request: &str, title: Option<&str>) -> Result<String> {
    if request.trim().is_empty() {
        return Err(SpeccyError::validation("request must be non-empty"));
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
            request: request.to_string(),
            source: Some("speccy new".into()),
            title: title.map(str::to_string),
            brainstorm_handoff: None,
        },
    )?;
    let title_str = title.unwrap_or(request);
    let painted_ref = style::paint(style::SPEC_REF, &spec_ref);
    Ok(format!(
        "Created draft spec {painted_ref} \"{title_str}\".\nNext: open your harness and run /speccy-plan {spec_ref}"
    ))
}

/// `speccy export review` — write the review packet to an explicit destination
/// (exempt from provenance scanning).
pub fn export_review(store: &Store, selector: Option<&str>, dest: Option<&str>) -> Result<String> {
    let spec = resolve_spec_any(store, selector)?;
    let run_id = store
        .list_runs(&spec.spec_id)?
        .into_iter()
        .next_back()
        .ok_or_else(|| SpeccyError::not_found("no run to export"))?;
    let packet = packets::review(store, &run_id)?;
    let markdown = packet
        .get("markdown")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let dest_dir = dest.map_or_else(
        || {
            store
                .git_root
                .join("docs")
                .join("specs")
                .join(&spec.spec_ref)
        },
        camino::Utf8PathBuf::from,
    );
    let path = dest_dir.join("review-packet.md");
    write_atomic(&path, markdown.as_bytes())?;
    Ok(format!("Wrote {path}"))
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

fn status_card(store: &Store, spec: &SpecState, run: &RunProjection) -> String {
    let repair_max = ProjectConfig::load(&store.workspace_root).map_or_else(
        |_| ProjectConfig::default().caps.task_repair_rounds,
        |c| c.caps.task_repair_rounds,
    );
    let mut card = format!(
        "{}  {}          Risk: {}\n",
        style::paint(style::SPEC_REF, &run.spec_ref),
        title_of(spec),
        style::paint(style::risk_style(run.risk), run.risk.as_str())
    );
    match run.state {
        RunState::Implementing | RunState::Verifying if is_interrupted(store, run) => {
            card.push_str(&interrupted_lines(store, run));
        }
        RunState::Implementing | RunState::Verifying => {
            let (label, context) = active_context(run, repair_max);
            _ = writeln!(card, "  {label} — {context}");
            _ = writeln!(
                card,
                "{}",
                style::paint(style::DIM, "  · autonomous, nothing needed")
            );
            if let Some(secs) = age_seconds(run) {
                let activity = run.last_event_label.as_deref().unwrap_or("");
                let line = if activity.is_empty() {
                    format!("  Last activity {}", humanize_age(secs))
                } else {
                    format!("  Last activity {} — {activity}", humanize_age(secs))
                };
                _ = writeln!(card, "{}", style::paint(style::DIM, &line));
            }
        }
        RunState::Verified => {
            let accepted = accepted_risk_count(run);
            let suffix = if accepted > 0 {
                format!(" · {}", packets::accepted_risk_phrase(accepted))
            } else {
                String::new()
            };
            _ = writeln!(
                card,
                "  {}{suffix}",
                style::paint(style::OK, "Ready to ship")
            );
            card.push_str("  Next: /speccy-ship\n");
        }
        RunState::Escalated => {
            _ = writeln!(
                card,
                "  {} — a requirement could not be proven",
                style::paint(style::WARN, "Needs you")
            );
            card.push_str("  Next: speccy review\n");
        }
        RunState::Submitted => {
            let pr = run
                .change_ref
                .as_ref()
                .and_then(|c| c.url.clone())
                .unwrap_or_else(|| "change proposed".into());
            _ = writeln!(card, "  Awaiting merge — {pr}");
            card.push_str("  Next: speccy accept   (after the change merges)\n");
        }
        _ => {}
    }
    card.trim_end().to_string()
}

fn active_context(run: &RunProjection, repair_max: u32) -> (&'static str, String) {
    let label = match run.state {
        RunState::Verifying => "Verifying",
        _ => "Implementing",
    };
    let context = match run.active_task() {
        Some(t) => {
            // Tasks appear by title, never a bare controller ID (DESIGN § CLI).
            let title = t.title.clone().unwrap_or_else(|| "the current task".into());
            if t.round > 1 {
                format!("{title} · repair round {} of {repair_max}", t.round)
            } else {
                title
            }
        }
        None => "run-level validation".to_string(),
    };
    (label, context)
}

/// The Interrupted status card lines: what died, the uncommitted-diff
/// attribution, and the resume action (DESIGN § CLI/Admin Flow).
fn interrupted_lines(store: &Store, run: &RunProjection) -> String {
    let repair_max = ProjectConfig::load(&store.workspace_root).map_or_else(
        |_| ProjectConfig::default().caps.task_repair_rounds,
        |c| c.caps.task_repair_rounds,
    );
    let (label, context) = active_context(run, repair_max);
    let mut out = format!(
        "  {} — session died during {label} \"{context}\"\n",
        style::paint(style::WARN, "Interrupted")
    );
    if let Some(task) = run.active_task()
        && let Some(base) = &task.baseline_commit
        && let Ok(diff) = speccy_core::gitx::worktree_stat(&store.git_root, base)
        && diff.files > 0
    {
        _ = writeln!(
            out,
            "  Uncommitted diff ({} files, +{} -{} vs {}) belongs to that task on resume",
            diff.files,
            diff.insertions,
            diff.deletions,
            base.get(..base.len().min(7)).unwrap_or(base)
        );
    }
    out.push_str("  Next: /speccy-implement\n");
    out.push_str("        (stash or commit first if these edits are not the worker's)\n");
    out
}

fn spec_card(spec: &SpecState) -> String {
    let Some(rev) = spec.latest_revision() else {
        return format!(
            "{}  {} — draft (no content yet)",
            style::paint(style::SPEC_REF, &spec.spec_ref),
            title_of(spec)
        );
    };
    let draft = &rev.draft;
    let risk_str = draft.risk.as_deref().unwrap_or("?");
    let painted_risk = match speccy_core::model::RiskTier::parse(risk_str) {
        Some(tier) => style::paint(style::risk_style(tier), risk_str),
        None => risk_str.to_string(),
    };
    let mut out = format!(
        "Spec: {}  {}          Risk: {}\n",
        style::paint(style::SPEC_REF, &spec.spec_ref),
        title_of(spec),
        painted_risk
    );
    let (status, status_style) = if rev.approved {
        ("approved", style::OK)
    } else {
        ("draft — approve to start", style::DIM)
    };
    _ = writeln!(out, "Status: {}", style::paint(status_style, status));
    out.push('\n');
    if let Some(goal) = &draft.goal {
        _ = writeln!(out, "Goal: {goal}");
    }
    if let Some(scope) = &draft.scope {
        if !scope.in_.is_empty() {
            _ = writeln!(out, "In scope: {}", scope.in_.join(" · "));
        }
        if !scope.out.is_empty() {
            _ = writeln!(out, "Out of scope: {}", scope.out.join(" · "));
        }
    }
    if !draft.requirements().is_empty() {
        _ = writeln!(out, "\n{}", style::paint(style::HEADER, "Acceptance"));
        for r in draft.requirements() {
            _ = writeln!(
                out,
                "- {}",
                r.statement.as_deref().unwrap_or("(no statement)")
            );
        }
    }
    let commands = command_evidence_strings(draft);
    if !commands.is_empty() {
        _ = writeln!(out, "\n{}", style::paint(style::HEADER, "Commands"));
        for command in commands {
            _ = writeln!(out, "- {command}");
        }
    }
    out.trim_end().to_string()
}

fn command_evidence_strings(draft: &speccy_core::model::SpecDraft) -> Vec<String> {
    let mut commands = BTreeSet::new();
    for requirement in draft.requirements() {
        for evidence in &requirement.evidence {
            if evidence.kind.as_deref() == Some("command")
                && let Some(command) = evidence.command.as_deref().map(str::trim)
                && !command.is_empty()
            {
                commands.insert(command.to_string());
            }
        }
    }
    commands.into_iter().collect()
}

fn close_out_card(spec: &SpecState, run: &RunProjection) -> String {
    let pr = run
        .change_ref
        .as_ref()
        .and_then(|c| c.url.clone())
        .unwrap_or_default();
    format!(
        "{}  {}\nAwaiting merge — {}\nAfter it merges, record it with: speccy accept",
        style::paint(style::SPEC_REF, &run.spec_ref),
        title_of(spec),
        pr
    )
}

fn accepted_summary(spec: &SpecState, run: &RunProjection) -> String {
    format!(
        "{}  {} — {}",
        style::paint(style::SPEC_REF, &run.spec_ref),
        title_of(spec),
        style::paint(style::OK, "landed")
    )
}

fn evidence_drilldown(run: &RunProjection) -> String {
    let mut out = format!("{}\n", style::paint(style::HEADER, "Ledger"));
    for (id, r) in &run.requirements {
        _ = writeln!(
            out,
            "  {id}  {}",
            style::paint(
                style::req_status_style(r.status),
                packets::req_status_label(r.status)
            )
        );
        if let Some(rr) = &r.residual_risk {
            _ = writeln!(out, "      residual risk: {rr}");
        }
    }
    if !run.findings.is_empty() {
        _ = writeln!(out, "\n{}", style::paint(style::HEADER, "Findings"));
        for (_, f) in &run.findings {
            _ = writeln!(out, "  [{}] {}", f.severity, f.note);
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
                [] if specs.len() == 1 => match specs.first() {
                    Some((id, _)) => store.spec_state(id),
                    None => Err(SpeccyError::not_found("no specs in this workspace")),
                },
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

/// Like `resolve_spec`, but inference across specs of any status (for
/// `archive`/`export`, which act on accepted/landed specs).
fn resolve_spec_any(store: &Store, selector: Option<&str>) -> Result<SpecState> {
    if selector.is_some() {
        return resolve_spec(store, selector);
    }
    let specs = store.list_specs()?;
    match specs.as_slice() {
        [(id, _)] => store.spec_state(id),
        [] => Err(SpeccyError::not_found("no specs in this workspace")),
        _ => Err(SpeccyError::ambiguous_selector(
            "more than one spec; name one explicitly",
        )),
    }
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
