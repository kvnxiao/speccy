#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy next` with derived action-kind logic
//! (SPEC-0033 REQ-004, CHK-007, CHK-008).
//!
//! Covers: per-spec form with `--json` and `--kind`-removal.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;

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

// -- CHK-007 ------------------------------------------------------------------

/// CHK-007: per-spec JSON with in-review task, `next_action.kind` == "review".
#[test]
fn chk007_per_spec_json_in_review_priority() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = format!(
        "{}{}{}",
        task_xml("T-001", "completed"),
        task_xml("T-002", "in-review"),
        task_xml("T-003", "pending"),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let next_action = parsed
        .get("next_action")
        .expect("next_action must be present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("review")),
        "expected kind=review but got: {parsed}",
    );
    assert_eq!(
        next_action.get("task_id"),
        Some(&serde_json::json!("T-002")),
        "expected task_id=T-002 but got: {parsed}",
    );
    Ok(())
}

/// After the in-review task transitions to completed, kind becomes "implement".
#[test]
fn chk007_per_spec_json_implement_after_review_done() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = format!(
        "{}{}",
        task_xml("T-002", "completed"),
        task_xml("T-003", "pending"),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let next_action = parsed
        .get("next_action")
        .expect("next_action must be present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("implement")),
        "expected kind=implement but got: {parsed}",
    );
    assert_eq!(
        next_action.get("task_id"),
        Some(&serde_json::json!("T-003")),
        "expected task_id=T-003 but got: {parsed}",
    );
    Ok(())
}

// -- CHK-008 ------------------------------------------------------------------

/// CHK-008: workspace form (no args, text) with SPEC-0002 (no TASKS.md) gives
/// one line containing "decompose" and SPEC-0002; no task-id in the line.
#[test]
fn chk008_workspace_text_decompose_when_no_tasks_md() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0002-bar",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        None, // no TASKS.md
    )?;
    let output = Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    assert_eq!(
        text.lines().count(),
        1,
        "expected exactly one output line, got {}: {text:?}",
        text.lines().count(),
    );
    let output_line = text.lines().next().expect("line count verified above");
    assert!(
        output_line.contains("SPEC-0002"),
        "line must reference SPEC-0002: {output_line:?}",
    );
    assert!(
        output_line.contains("decompose"),
        "line must contain 'decompose': {output_line:?}",
    );
    // No task-id should appear (decompose has no associated task).
    assert!(
        !output_line.contains("T-"),
        "line must not contain a task id for decompose: {output_line:?}",
    );
    Ok(())
}

// -- per-spec form: decompose via --json -------------------------------------

#[test]
fn per_spec_json_decompose_when_no_tasks_md() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0002-bar",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0002", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let next_action = parsed
        .get("next_action")
        .expect("next_action must be present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("decompose")),
        "expected kind=decompose but got: {parsed}",
    );
    assert!(
        next_action.get("task_id").is_none(),
        "decompose must not carry a task_id: {parsed}",
    );
    Ok(())
}

// -- per-spec form: null next_action when completed + REPORT.md exists --------

#[test]
fn per_spec_json_null_when_all_done_and_report_present() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "completed");
    let spec_dir = write_spec(
        &ws.root,
        "0003-baz",
        &spec_md_template("SPEC-0003", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0003", &tasks_xml)),
    )?;
    // Write REPORT.md so kind resolution lands on "completed".
    fs_err::write(spec_dir.join("REPORT.md").as_std_path(), "# Report\n")?;

    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0003", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert!(
        parsed
            .get("next_action")
            .is_none_or(serde_json::Value::is_null),
        "next_action must be null: {parsed}",
    );
    assert_eq!(
        parsed.get("reason"),
        Some(&serde_json::json!("completed")),
        "reason must be 'completed' but got: {parsed}",
    );
    Ok(())
}

// -- workspace form: completed spec omitted -----------------------------------

#[test]
fn workspace_text_completed_spec_omitted() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0001: has pending task (should appear).
    let tasks_xml_active = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml_active)),
    )?;
    // SPEC-0002: all done + REPORT.md (should be omitted).
    let tasks_xml_done = task_xml("T-001", "completed");
    let spec_dir_done = write_spec(
        &ws.root,
        "0002-bar",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0002", &tasks_xml_done)),
    )?;
    fs_err::write(spec_dir_done.join("REPORT.md").as_std_path(), "# Report\n")?;

    let output = Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    assert!(
        text.contains("SPEC-0001"),
        "SPEC-0001 (pending) must appear in output: {text:?}",
    );
    assert!(
        !text.contains("SPEC-0002"),
        "SPEC-0002 (completed) must be omitted: {text:?}",
    );
    Ok(())
}

// -- --kind flag is removed ---------------------------------------------------

/// CHK-007 scenario 5: `speccy next --kind implement` → clap error.
#[test]
fn kind_flag_is_rejected() -> TestResult {
    let ws = Workspace::new()?;
    Command::cargo_bin("speccy")?
        .args(["next", "--kind", "implement"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .failure()
        .stderr(contains("unexpected argument"));
    Ok(())
}

// -- per-spec form: ship when all done and no REPORT.md ----------------------

#[test]
fn per_spec_json_ship_when_all_done_no_report() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "completed");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    // No REPORT.md → kind should be "ship".
    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let next_action = parsed
        .get("next_action")
        .expect("next_action must be present");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("ship")),
        "expected kind=ship but got: {parsed}",
    );
    Ok(())
}

// -- per-spec form: unknown SPEC-ID exits 1 ----------------------------------

#[test]
fn per_spec_unknown_spec_id_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    Command::cargo_bin("speccy")?
        .args(["next", "SPEC-9999"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .failure()
        .stderr(contains("SPEC-9999"));
    Ok(())
}

// -- `speccy next` appears in --help -----------------------------------------

#[test]
fn next_appears_in_help_subcommands() -> TestResult {
    let ws = Workspace::new()?;
    Command::cargo_bin("speccy")?
        .arg("--help")
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("next"));
    Ok(())
}

// -- workspace JSON form: active specs listed --------------------------------

#[test]
fn workspace_json_active_specs_listed() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let output = Command::cargo_bin("speccy")?
        .args(["next", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let specs = parsed.get("specs").expect("specs array must be present");
    assert!(specs.is_array(), "specs must be a JSON array: {parsed}");
    let arr = specs.as_array().expect("already checked");
    assert_eq!(
        arr.len(),
        1,
        "expected 1 spec entry but got {}: {parsed}",
        arr.len()
    );
    let entry = arr.first().expect("first entry");
    assert_eq!(
        entry.get("spec_id"),
        Some(&serde_json::json!("SPEC-0001")),
        "spec_id mismatch: {entry}",
    );
    let next_action = entry.get("next_action").expect("next_action in entry");
    assert_eq!(
        next_action.get("kind"),
        Some(&serde_json::json!("implement")),
        "expected kind=implement in entry: {entry}",
    );
    Ok(())
}
