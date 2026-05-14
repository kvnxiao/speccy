#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! End-to-end tests for `speccy plan` (greenfield + amendment).
//! Exercises SPEC-0005 REQ-001..REQ-007 through the binary entry point.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;

fn write_agents(ws: &Workspace, body: &str) -> TestResult {
    fs_err::write(ws.root.join("AGENTS.md").as_std_path(), body)?;
    Ok(())
}

#[test]
fn greenfield_renders_agents_and_next_spec_id() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\nUse Rust.\n")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("Use Rust"))
        .stdout(contains("SPEC-0001"));
    Ok(())
}

#[test]
fn greenfield_renders_next_spec_id_skipping_gaps() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    write_spec(
        &ws.root,
        "0003-bar",
        &spec_md_template("SPEC-0003", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(ws.root.as_std_path());
    cmd.assert().success().stdout(contains("SPEC-0004"));
    Ok(())
}

#[test]
fn greenfield_walks_mission_folders_for_id_allocation() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    // Flat 0001 + mission-folder 0002 + mission-folder 0010.
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    write_spec(
        &ws.root,
        "auth/0002-signup",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    write_spec(
        &ws.root,
        "billing/0010-invoice",
        &spec_md_template("SPEC-0010", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(ws.root.as_std_path());
    cmd.assert().success().stdout(contains("SPEC-0011"));
    Ok(())
}

#[test]
fn greenfield_missing_agents_warns_but_still_renders() -> TestResult {
    let ws = Workspace::new()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("AGENTS.md missing"))
        .stderr(contains("AGENTS.md not found"));
    Ok(())
}

#[test]
fn plan_outside_workspace_exits_with_clear_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn amend_form_inlines_existing_flat_spec_md() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\nrules\n")?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("SPEC-0001"))
        .stdout(contains("Example SPEC-0001"))
        .stdout(contains("no parent MISSION.md"));
    Ok(())
}

#[test]
fn amend_form_resolves_mission_grouped_spec_and_inlines_mission_md() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "auth/0042-signup",
        &spec_md_template("SPEC-0042", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    let mission_path = ws
        .root
        .join(".speccy")
        .join("specs")
        .join("auth")
        .join("MISSION.md");
    fs_err::write(
        mission_path.as_std_path(),
        "# Mission: auth\n\nScope: signup, login, password reset.\n",
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan")
        .arg("SPEC-0042")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("SPEC-0042"))
        .stdout(contains("Example SPEC-0042"))
        .stdout(contains("# Mission: auth"))
        .stdout(contains("signup, login, password reset"));
    Ok(())
}

#[test]
fn amend_form_for_mission_grouped_spec_without_mission_md_yields_ungrouped_marker() -> TestResult {
    // Edge case: a spec lives inside a focus folder but the focus
    // folder has no MISSION.md (e.g. created prematurely, or planned
    // but never written). The walker must still return the ungrouped
    // marker rather than failing.
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "auth/0042-signup",
        &spec_md_template("SPEC-0042", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    // Note: no MISSION.md is written anywhere.

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan")
        .arg("SPEC-0042")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("SPEC-0042"))
        .stdout(contains("no parent MISSION.md"));
    Ok(())
}

#[test]
fn amend_form_rejects_invalid_id_format_with_code_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("invalid SPEC-ID"));
    Ok(())
}

#[test]
fn amend_form_reports_missing_spec() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan")
        .arg("SPEC-9999")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("SPEC-9999"))
        .stderr(contains("not found"));
    Ok(())
}

#[test]
fn unknown_flag_exits_with_usage_code() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}
