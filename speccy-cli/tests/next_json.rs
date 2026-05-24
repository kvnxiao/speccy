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
use common::sha256_hex;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;

fn render_workspace(ws: &Workspace) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: None,
            json: true,
        },
        &ws.root,
        &mut buf,
        &mut err,
    )?;
    Ok(String::from_utf8(buf)?)
}

fn render_per_spec(ws: &Workspace, spec_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: Some(spec_id.to_owned()),
            json: true,
        },
        &ws.root,
        &mut buf,
        &mut err,
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

// -- CHK-007: per-spec JSON envelope shape (work) ----------------------------

#[test]
fn per_spec_json_envelope_shape_work() -> TestResult {
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
        Some(&serde_json::json!("work")),
        "kind must be work: {parsed}",
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
        Some(&serde_json::json!("work")),
        "kind in entry: {entry}",
    );
    Ok(())
}

// -- SPEC-0041 REQ-001/REQ-002: vet kind in JSON -----------------------------

#[test]
fn workspace_json_emits_vet_when_all_completed_and_no_vet_md() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "completed");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let text = render_workspace(&ws)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let specs = parsed
        .get("specs")
        .expect("specs array")
        .as_array()
        .expect("array");
    let entry = specs.first().expect("one entry");
    let next_action = entry.get("next_action").expect("next_action present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("vet")),
        "kind must be vet: {entry}",
    );
    // No task_id field when kind is vet.
    assert!(
        next_action.get("task_id").is_none(),
        "task_id must be absent when kind=vet: {entry}",
    );
    Ok(())
}

#[test]
fn workspace_json_emits_ship_when_vet_passes_fresh() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "completed");
    let tasks_md = tasks_md_xml("SPEC-0001", &tasks_xml);
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md),
    )?;
    let hash = sha256_hex(tasks_md.as_bytes());
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let vet_body = format!(
        "## Invocation 1\n\n<gate verdict=\"passed\" tasks_hash=\"{hash}\" date=\"2026-05-22T00:00:00Z\">\nstub.\n</gate>\n",
    );
    fs_err::write(journal.join("VET.md").as_std_path(), vet_body)?;
    let text = render_workspace(&ws)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let specs = parsed
        .get("specs")
        .expect("specs")
        .as_array()
        .expect("array");
    let entry = specs.first().expect("one entry");
    let next_action = entry.get("next_action").expect("next_action");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("ship")),
        "kind must be ship when VET.md passes fresh: {entry}",
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

// ---------------------------------------------------------------------------
// SPEC-0043 REQ-003: terminal-state exit code 2 and stderr line.
// ---------------------------------------------------------------------------

fn run_per_spec_capture(
    ws: &Workspace,
    spec_id: &str,
) -> Result<(i32, String, String), Box<dyn std::error::Error>> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        &NextArgs {
            spec_id: Some(spec_id.to_owned()),
            json: true,
        },
        &ws.root,
        &mut out,
        &mut err,
    )?;
    Ok((code, String::from_utf8(out)?, String::from_utf8(err)?))
}

fn run_workspace_capture(
    ws: &Workspace,
) -> Result<(i32, String, String), Box<dyn std::error::Error>> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        &NextArgs {
            spec_id: None,
            json: true,
        },
        &ws.root,
        &mut out,
        &mut err,
    )?;
    Ok((code, String::from_utf8(out)?, String::from_utf8(err)?))
}

#[test]
fn per_spec_terminal_completed_exits_2_with_stderr_and_envelope() -> TestResult {
    // All tasks completed + REPORT.md present → terminal "completed".
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "completed");
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        "# Report\n\nstub.\n",
    )?;

    let (code, stdout, stderr) = run_per_spec_capture(&ws, "SPEC-0001")?;
    assert_eq!(code, 2, "terminal completed must exit 2: stderr={stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(parsed.get("next_action"), Some(&serde_json::Value::Null));
    assert_eq!(
        parsed.get("reason"),
        Some(&serde_json::json!("completed")),
        "reason must be `completed`: {parsed}",
    );
    assert!(
        stderr.contains("SPEC-0001 is completed"),
        "stderr must name the spec + reason: {stderr:?}",
    );
    assert!(
        stderr.contains("speccy archive SPEC-0001"),
        "stderr must include archive suggestion: {stderr:?}",
    );
    Ok(())
}

#[test]
fn per_spec_terminal_dropped_exits_2_with_dropped_reason() -> TestResult {
    let ws = Workspace::new()?;
    // status: dropped, with a pending task — frontmatter status
    // wins per SPEC-0043 REQ-003.
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "dropped"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;

    let (code, stdout, stderr) = run_per_spec_capture(&ws, "SPEC-0001")?;
    assert_eq!(code, 2, "dropped must exit 2: stderr={stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(parsed.get("next_action"), Some(&serde_json::Value::Null));
    assert_eq!(parsed.get("reason"), Some(&serde_json::json!("dropped")));
    assert!(
        stderr.contains("SPEC-0001 is dropped"),
        "stderr must name dropped reason: {stderr:?}",
    );
    assert!(
        stderr.contains("speccy archive SPEC-0001"),
        "stderr must include archive suggestion: {stderr:?}",
    );
    Ok(())
}

#[test]
fn per_spec_terminal_superseded_exits_2_with_superseded_reason() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "superseded"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;

    let (code, stdout, _stderr) = run_per_spec_capture(&ws, "SPEC-0001")?;
    assert_eq!(code, 2);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(parsed.get("reason"), Some(&serde_json::json!("superseded")));
    Ok(())
}

#[test]
fn empty_workspace_json_carries_no_active_specs_reason_and_exits_2() -> TestResult {
    // Empty `.speccy/specs/` → workspace-level terminal: exit 2, JSON
    // envelope carries `reason: "no_active_specs"` alongside the empty
    // `specs` array, and a stderr advisory is written so an AI harness
    // sees the loop-stop signal in both stdout and exit code.
    let ws_empty = Workspace::new()?;
    let (code, stdout, stderr) = run_workspace_capture(&ws_empty)?;
    assert_eq!(code, 2, "empty workspace must exit 2: stderr={stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));
    assert_eq!(parsed.get("specs"), Some(&serde_json::json!([])));
    assert_eq!(
        parsed.get("reason"),
        Some(&serde_json::json!("no_active_specs")),
    );
    assert!(
        stderr.contains("no active specs"),
        "stderr must name the workspace-terminal reason: {stderr:?}",
    );
    Ok(())
}

#[test]
fn non_empty_workspace_json_omits_reason_field() -> TestResult {
    // Non-terminal workspace → `reason` field absent so consumers can
    // distinguish "still has work" from "loop-stop" purely by presence.
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let stdout = render_workspace(&ws)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    assert!(
        parsed.get("reason").is_none(),
        "non-terminal workspace must omit `reason`: {parsed}",
    );
    Ok(())
}

#[test]
fn per_spec_non_terminal_exits_0_with_empty_stderr() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let (code, stdout, stderr) = run_per_spec_capture(&ws, "SPEC-0001")?;
    assert_eq!(code, 0, "non-terminal must exit 0");
    assert!(stderr.is_empty(), "stderr must be empty: {stderr:?}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    assert!(
        parsed.get("next_action").is_some_and(|v| !v.is_null()),
        "non-terminal next_action must be non-null: {parsed}",
    );
    Ok(())
}
