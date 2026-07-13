//! `export run-bundle` — the safe-by-construction immutable run receipt
//! (DESIGN § Lightweight Team Sharing; shape in SCHEMAS § Run receipt).
//!
//! The receipt is built from an allowlist of stored facts only — IDs,
//! statuses, hashes, round counts, and residual-risk notes — never raw
//! command output, transcripts, environment values, or identity claims the
//! controller did not observe. Identical stored facts produce identical
//! bytes: object keys serialize alphabetically, arrays are sorted by ID, and
//! durations end at the last recorded event, never at export time.

use crate::config::ProjectConfig;
use crate::error::Result;
use crate::evidence::scrub_secrets;
use crate::evidence::secret_env_values;
use crate::gitx;
use crate::projection::RunProjection;
use crate::store::Store;
use serde_json::Value;
use serde_json::json;
use std::fmt::Write as _;

/// Version of the receipt JSON shape; bump on any structural change.
const RECEIPT_SCHEMA: u32 = 1;

/// The ID-sorted allowlisted arrays shared by the JSON and Markdown views.
struct Sections {
    tasks: Vec<Value>,
    requirements: Vec<Value>,
    evidence: Vec<Value>,
    findings: Vec<Value>,
    decisions: Vec<Value>,
}

/// Build the run receipt: the versioned JSON (with `manifest_hash`) plus its
/// compact Markdown view.
///
/// # Errors
///
/// Returns an error if the run cannot be loaded or the base-to-head diff
/// backing `run.diff_hash` cannot be produced (fail closed, never a
/// fabricated empty diff).
pub fn run_bundle(store: &Store, run_id: &str) -> Result<(Value, String)> {
    let (_, run) = store.run_by_id(run_id)?;
    let config = ProjectConfig::load(&store.workspace_root)?;

    let final_head = run
        .change_ref
        .as_ref()
        .and_then(|c| c.head_sha.clone())
        .or_else(|| run.last_snapshot.clone())
        .unwrap_or_else(|| run.base_commit.clone());
    let diff = gitx::range_diff_text(&store.git_root, &run.base_commit, &final_head)?;
    let diff_hash = crate::hash::sha256_prefixed(diff.as_bytes());

    let sections = build_sections(&run);
    let mut receipt = json!({
        "receipt_schema": RECEIPT_SCHEMA,
        "controller_version": env!("CARGO_PKG_VERSION"),
        "spec": { "ref": run.spec_ref, "revision": run.revision_id, "risk": run.risk },
        "run": {
            "id": run.run_id, "state": run.state, "branch": run.branch,
            "base_commit": run.base_commit, "final_head": final_head,
            "diff_hash": diff_hash, "review_rounds": run.run_review_round,
            "active_seconds": run.last_event_ts.map_or(0, |ts| run.active_seconds_at(ts)),
        },
        "caps": {
            "task_repair_rounds": config.caps.task_repair_rounds,
            "run_review_rounds": config.caps.run_review_rounds,
        },
        "tasks": sections.tasks,
        "requirements": sections.requirements,
        "evidence": sections.evidence,
        "findings": sections.findings,
        "decisions": sections.decisions,
        "change_ref": run.change_ref,
    });

    // The manifest hash covers the receipt serialized without the hash field;
    // serde_json orders object keys alphabetically, so the bytes are stable.
    let manifest = crate::hash::sha256_prefixed(receipt.to_string().as_bytes());
    if let Some(fields) = receipt.as_object_mut() {
        fields.insert("manifest_hash".into(), json!(manifest));
    }
    let markdown = render_markdown(&run, &receipt, &manifest);
    Ok((receipt, markdown))
}

/// Assemble the allowlisted arrays from the run projection. Residual-risk
/// notes are the only included prose and pass known-secret scrubbing.
fn build_sections(run: &RunProjection) -> Sections {
    let secrets = secret_env_values();
    let scrub = |note: &Option<String>| -> Value {
        note.as_ref().map_or(Value::Null, |n| {
            Value::String(
                String::from_utf8_lossy(&scrub_secrets(n.as_bytes(), &secrets)).into_owned(),
            )
        })
    };

    let mut tasks: Vec<Value> = run
        .tasks
        .iter()
        .map(|t| {
            json!({
                "id": t.id, "title": t.title, "status": t.status,
                "rounds": t.round, "requirements": t.requirements,
            })
        })
        .collect();
    sort_by_id(&mut tasks);

    // BTreeMap iteration is already ID-sorted.
    let requirements: Vec<Value> = run
        .requirements
        .iter()
        .map(|(id, r)| {
            json!({ "id": id, "status": r.status, "residual_risk": scrub(&r.residual_risk) })
        })
        .collect();

    let mut evidence: Vec<Value> = run
        .evidence
        .iter()
        .map(|ev| {
            json!({
                "id": ev.id, "requirement": ev.requirement, "request": ev.request,
                "kind": ev.kind, "exit_code": ev.exit_code,
                "stdout_hash": ev.stdout_hash, "artifact_hash": ev.artifact_hash,
                "control": ev.control.as_ref().map(|c| c.status),
            })
        })
        .collect();
    sort_by_id(&mut evidence);

    let mut findings: Vec<Value> = run
        .findings
        .iter()
        .map(|(_, f)| {
            json!({
                "id": f.id, "persona": f.persona, "severity": f.severity,
                "requirement": f.requirement, "task": f.task,
            })
        })
        .collect();
    sort_by_id(&mut findings);

    let mut decisions: Vec<Value> = run
        .decisions
        .iter()
        .map(|d| {
            json!({
                "id": d.decision_id, "type": d.kind, "actor": d.actor,
                "requirement": d.requirement, "task": d.task,
                "residual_risk": scrub(&d.residual_risk),
            })
        })
        .collect();
    sort_by_id(&mut decisions);

    Sections {
        tasks,
        requirements,
        evidence,
        findings,
        decisions,
    }
}

/// The string at `key` of a JSON object, or a placeholder.
fn field<'a>(item: &'a Value, key: &str) -> &'a str {
    item.get(key).and_then(Value::as_str).unwrap_or("?")
}

/// Sort a JSON array by each object's `id` field for deterministic output.
fn sort_by_id(items: &mut [Value]) {
    items.sort_by(|a, b| field(a, "id").cmp(field(b, "id")));
}

/// The compact human view of the same allowlisted facts.
fn render_markdown(run: &RunProjection, receipt: &Value, manifest: &str) -> String {
    let run_obj = receipt.get("run").cloned().unwrap_or_default();
    let mut out = String::new();
    _ = writeln!(
        out,
        "# Speccy run receipt — {} ({}, {:?})\n",
        run.spec_ref, run.revision_id, run.risk
    );
    _ = writeln!(
        out,
        "Run {} — {:?} · branch {} · review rounds {}",
        run.run_id, run.state, run.branch, run.run_review_round
    );
    _ = writeln!(
        out,
        "Base {} → head {} · diff {}\n",
        run.base_commit,
        field(&run_obj, "final_head"),
        field(&run_obj, "diff_hash"),
    );
    render_section(&mut out, "Tasks", receipt.get("tasks"), |t| {
        format!(
            "{} {} — {} ({} rounds)",
            field(t, "id"),
            t.get("title").and_then(Value::as_str).unwrap_or(""),
            field(t, "status"),
            t.get("rounds").cloned().unwrap_or_default()
        )
    });
    render_section(&mut out, "Requirements", receipt.get("requirements"), |r| {
        let risk = r
            .get("residual_risk")
            .and_then(Value::as_str)
            .map(|n| format!(" — residual risk: {n}"))
            .unwrap_or_default();
        format!("{} {}{risk}", field(r, "id"), field(r, "status"))
    });
    render_section(&mut out, "Evidence", receipt.get("evidence"), |e| {
        let control = e
            .get("control")
            .and_then(Value::as_str)
            .map(|c| format!(" · control {c}"))
            .unwrap_or_default();
        format!(
            "{} {} {}{control}",
            field(e, "id"),
            field(e, "kind"),
            field(e, "requirement")
        )
    });
    render_section(&mut out, "Findings", receipt.get("findings"), |f| {
        format!(
            "{} {} ({})",
            field(f, "id"),
            field(f, "severity"),
            f.get("persona")
                .and_then(Value::as_str)
                .unwrap_or("controller")
        )
    });
    render_section(&mut out, "Decisions", receipt.get("decisions"), |d| {
        format!(
            "{} {} by {}",
            field(d, "id"),
            field(d, "type"),
            field(d, "actor")
        )
    });
    _ = writeln!(out, "Manifest {manifest}");
    out
}

/// Append one titled bullet section; empty sections render as "none".
fn render_section(
    out: &mut String,
    title: &str,
    items: Option<&Value>,
    line: impl Fn(&Value) -> String,
) {
    let items = items
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if items.is_empty() {
        _ = writeln!(out, "{title}: none\n");
        return;
    }
    _ = writeln!(out, "{title}:");
    for item in items {
        _ = writeln!(out, "- {}", line(item));
    }
    out.push('\n');
}
