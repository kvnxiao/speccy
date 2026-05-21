#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! JSON output contract tests for `speccy next --json`.
//!
//! Tests the new derived-kind JSON envelopes (SPEC-0033 REQ-004).
//! Per-spec `--json` form is covered by `next_derived.rs`. This file
//! covers additional workspace-form and per-spec-form JSON contract
//! checks not duplicated there.

mod common;

use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;

fn tasks_md_xml(spec_id: &str, tasks_xml: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n<tasks spec=\"{spec_id}\">\n\n{tasks_xml}\n</tasks>\n",
    )
}

fn task_xml(id: &str, state: &str) -> String {
    format!(
        "<task id=\"{id}\" state=\"{state}\" covers=\"REQ-001\">\ndo the thing\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
    )
}

fn render_workspace(ws: &Workspace) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: None,
            json: true,
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

fn render_per_spec(ws: &Workspace, spec_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: Some(spec_id.to_owned()),
            json: true,
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-007: per-spec JSON envelope shape -----------------------------------

#[test]
fn per_spec_json_envelope_shape_review() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = format!(
        "{}{}",
        task_xml("T-001", "completed"),
        task_xml("T-002", "in-review"),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let text = render_per_spec(&ws, "SPEC-0001")?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(
        parsed.get("schema_version"),
        Some(&serde_json::json!(1)),
        "schema_version must be 1: {parsed}",
    );
    assert_eq!(
        parsed.get("spec_id"),
        Some(&serde_json::json!("SPEC-0001")),
        "spec_id must be SPEC-0001: {parsed}",
    );
    let next_action = parsed
        .get("next_action")
        .expect("next_action must be present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("review")),
        "kind must be review: {parsed}",
    );
    assert_eq!(
        next_action.get("task_id"),
        Some(&serde_json::json!("T-002")),
        "task_id must be T-002: {parsed}",
    );
    // No reason field on a non-null next_action.
    assert!(
        parsed.get("reason").is_none(),
        "reason must be absent when next_action is present: {parsed}",
    );
    Ok(())
}

// -- CHK-007: per-spec JSON envelope shape (implement) -----------------------

#[test]
fn per_spec_json_envelope_shape_implement() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-003", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let text = render_per_spec(&ws, "SPEC-0001")?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let next_action = parsed
        .get("next_action")
        .expect("next_action must be present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("implement")),
        "kind must be implement: {parsed}",
    );
    assert_eq!(
        next_action.get("task_id"),
        Some(&serde_json::json!("T-003")),
        "task_id must be T-003: {parsed}",
    );
    Ok(())
}

// -- workspace form JSON envelope shape --------------------------------------

#[test]
fn workspace_json_envelope_shape() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let text = render_workspace(&ws)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(
        parsed.get("schema_version"),
        Some(&serde_json::json!(1)),
        "schema_version must be 1: {parsed}",
    );
    let specs = parsed.get("specs").expect("specs must be present");
    assert!(specs.is_array(), "specs must be an array: {parsed}");
    let arr = specs.as_array().expect("already checked");
    assert_eq!(arr.len(), 1, "expected 1 spec: {parsed}");
    let entry = arr.first().expect("first entry");
    assert_eq!(
        entry.get("spec_id"),
        Some(&serde_json::json!("SPEC-0001")),
        "spec_id in entry: {entry}",
    );
    let next_action = entry.get("next_action").expect("next_action in entry");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("implement")),
        "kind in entry: {entry}",
    );
    Ok(())
}

// -- determinism -------------------------------------------------------------

#[test]
fn determinism() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = format!(
        "{}{}",
        task_xml("T-001", "pending"),
        task_xml("T-002", "in-review"),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;

    // Workspace form.
    let a = render_workspace(&ws)?;
    let b = render_workspace(&ws)?;
    assert_eq!(
        a, b,
        "two consecutive workspace JSON renders must be byte-identical"
    );

    // Per-spec form.
    let a_ps = render_per_spec(&ws, "SPEC-0001")?;
    let b_ps = render_per_spec(&ws, "SPEC-0001")?;
    assert_eq!(
        a_ps, b_ps,
        "two consecutive per-spec JSON renders must be byte-identical"
    );

    Ok(())
}
