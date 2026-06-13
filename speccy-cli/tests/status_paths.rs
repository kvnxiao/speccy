#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for the resolved-path fields in `speccy status --json`
//! and `speccy next --json`.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::write_spec;

fn tasks_md_xml_with_pending(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\ndo the thing\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n",
    )
}

/// `speccy status SPEC-0031 --json` includes `spec_md_path`,
/// `tasks_md_path`, and `mission_md_path: null` for a flat spec.
/// The `schema_version` field must equal 1.
#[test]
fn chk009_status_json_carries_resolved_paths_flat_spec() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0031-foo",
        &spec_md_template("SPEC-0031", "in-progress"),
        Some(&bootstrap_tasks_md("SPEC-0031")),
    )?;
    let output = Command::cargo_bin("speccy")?
        .args(["status", "SPEC-0031", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json_text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;

    // schema_version must be 1.
    assert_eq!(
        parsed.get("schema_version"),
        Some(&serde_json::json!(1)),
        "schema_version must be 1"
    );

    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs array");
    assert_eq!(specs.len(), 1, "expected exactly one spec");
    let spec = specs.first().expect("first spec entry");

    // spec_md_path: forward-slash repo-relative path to SPEC.md.
    let spec_md_path = spec
        .get("spec_md_path")
        .and_then(|v| v.as_str())
        .expect("spec_md_path must be present");
    assert_eq!(
        spec_md_path, ".speccy/specs/0031-foo/SPEC.md",
        "spec_md_path mismatch"
    );

    // tasks_md_path: forward-slash repo-relative path to TASKS.md.
    let tasks_md_path = spec
        .get("tasks_md_path")
        .and_then(|v| v.as_str())
        .expect("tasks_md_path must be present (TASKS.md was written)");
    assert_eq!(
        tasks_md_path, ".speccy/specs/0031-foo/TASKS.md",
        "tasks_md_path mismatch"
    );

    // mission_md_path: null for a flat (non-mission-grouped) spec.
    assert_eq!(
        spec.get("mission_md_path"),
        Some(&serde_json::Value::Null),
        "mission_md_path must be null for flat spec"
    );
    Ok(())
}

/// `tasks_md_path` is null when TASKS.md is absent.
#[test]
fn status_json_tasks_md_path_null_when_absent() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0032-bar",
        &spec_md_template("SPEC-0032", "draft"),
        None, // no TASKS.md
    )?;
    let output = Command::cargo_bin("speccy")?
        .args(["status", "SPEC-0032", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json_text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;

    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs array");
    let spec = specs.first().expect("first spec");

    assert_eq!(
        spec.get("tasks_md_path"),
        Some(&serde_json::Value::Null),
        "tasks_md_path must be null when TASKS.md is absent"
    );
    Ok(())
}

/// `speccy next SPEC-0040 --json` carries `mission_md_path`
/// equal to `.speccy/specs/auth/MISSION.md` when the spec lives under a
/// mission folder and MISSION.md is present there.
#[test]
fn chk010_next_json_carries_mission_md_path_for_mission_spec() -> TestResult {
    let ws = Workspace::new()?;
    // Create a mission folder with a MISSION.md file.
    let mission_dir = ws.root.join(".speccy").join("specs").join("auth");
    fs_err::create_dir_all(mission_dir.as_std_path())?;
    fs_err::write(
        mission_dir.join("MISSION.md").as_std_path(),
        "# Auth Mission\n",
    )?;
    // Create a spec inside the mission folder.
    let spec_dir = mission_dir.join("0040-signup");
    fs_err::create_dir_all(spec_dir.as_std_path())?;
    fs_err::write(
        spec_dir.join("SPEC.md").as_std_path(),
        spec_md_template("SPEC-0040", "in-progress"),
    )?;
    fs_err::write(
        spec_dir.join("TASKS.md").as_std_path(),
        tasks_md_xml_with_pending("SPEC-0040"),
    )?;

    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0040", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json_text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;

    assert_eq!(
        parsed.get("schema_version"),
        Some(&serde_json::json!(1)),
        "schema_version must be 1"
    );

    let mission_md_path = parsed
        .get("mission_md_path")
        .and_then(|v| v.as_str())
        .expect("mission_md_path must be present for a mission spec");
    assert_eq!(
        mission_md_path, ".speccy/specs/auth/MISSION.md",
        "mission_md_path mismatch"
    );
    Ok(())
}

/// `mission_md_path` is null in `speccy next` per-spec
/// JSON when the spec does NOT live under a mission folder.
#[test]
fn next_json_mission_md_path_null_for_flat_spec() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-flat",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml_with_pending("SPEC-0001")),
    )?;

    let output = Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json_text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;

    assert_eq!(
        parsed.get("mission_md_path"),
        Some(&serde_json::Value::Null),
        "mission_md_path must be null for a flat spec"
    );
    Ok(())
}

/// Workspace-form `speccy next --json` also carries path fields on each
/// spec entry.
#[test]
fn next_workspace_json_carries_path_fields() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml_with_pending("SPEC-0001")),
    )?;

    let output = Command::cargo_bin("speccy")?
        .args(["next", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json_text = String::from_utf8(output)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;

    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs array");
    let entry = specs.first().expect("first entry");

    assert!(
        entry.get("spec_md_path").is_some(),
        "spec_md_path must be present in workspace form"
    );
    assert!(
        entry.get("tasks_md_path").is_some(),
        "tasks_md_path must be present in workspace form"
    );
    assert!(
        entry.get("mission_md_path").is_some(),
        "mission_md_path must be present in workspace form"
    );
    Ok(())
}
