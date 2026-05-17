#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
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
fn greenfield_renders_next_spec_id() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0023 REQ-005: AGENTS.md is no longer inlined into the rendered
    // prompt; modern AI coding harnesses auto-load it themselves. Writing
    // AGENTS.md here would only confirm the renderer ignores it, which the
    // negative `does_not_inline_agents_md` test below pins explicitly.
    write_agents(&ws, "# Agents\nUse Rust.\n")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(ws.root.as_std_path());
    cmd.assert().success().stdout(contains("SPEC-0001"));
    Ok(())
}

#[test]
fn greenfield_does_not_inline_agents_md() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\nUSE_RUST_SENTINEL\n")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("plan").current_dir(ws.root.as_std_path());
    let out = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).into_owned();
    assert!(
        stdout.contains("SPEC-0001"),
        "greenfield should still render the next ID: {stdout}"
    );
    assert!(
        !stdout.contains("USE_RUST_SENTINEL"),
        "AGENTS.md body must not be inlined into the rendered prompt: {stdout}",
    );
    assert!(
        !stdout.contains("{{agents}}"),
        "the retired `{{agents}}` placeholder must not appear in rendered output: {stdout}",
    );
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
fn amend_form_names_existing_flat_spec_md_path() -> TestResult {
    // SPEC-0023 REQ-006: the rendered prompt names the SPEC.md
    // repo-relative path; the body is not inlined.
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
    let assert = cmd
        .assert()
        .success()
        .stdout(contains("SPEC-0001"))
        .stdout(contains(".speccy/specs/0001-foo/SPEC.md"));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        !stdout.contains("Example SPEC-0001"),
        "SPEC.md body must not be inlined into the rendered prompt: {stdout}",
    );
    // Flat single-focus project: no MISSION.md exists anywhere on disk,
    // so the rendered prompt must not name one. The retired
    // "no parent MISSION.md" marker is also gone.
    assert!(
        !stdout.contains("MISSION.md"),
        "flat single-focus project must not surface any MISSION.md Read instruction: {stdout}",
    );
    Ok(())
}

#[test]
fn amend_form_resolves_mission_grouped_spec_and_names_mission_md_path() -> TestResult {
    // SPEC-0023 REQ-006: the rendered prompt names the MISSION.md and
    // SPEC.md repo-relative paths; the bodies are not inlined.
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
    let assert = cmd
        .assert()
        .success()
        .stdout(contains("SPEC-0042"))
        .stdout(contains(".speccy/specs/auth/0042-signup/SPEC.md"))
        .stdout(contains(".speccy/specs/auth/MISSION.md"))
        .stdout(contains("## Mission context"));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        !stdout.contains("Example SPEC-0042"),
        "SPEC.md body must not be inlined into the rendered prompt: {stdout}",
    );
    assert!(
        !stdout.contains("# Mission: auth"),
        "MISSION.md body must not be inlined into the rendered prompt: {stdout}",
    );
    assert!(
        !stdout.contains("signup, login, password reset"),
        "MISSION.md body must not be inlined into the rendered prompt: {stdout}",
    );
    Ok(())
}

#[test]
fn amend_form_for_mission_grouped_spec_without_mission_md_emits_no_mission_read() -> TestResult {
    // SPEC-0023 REQ-006: a spec lives inside a focus folder but the
    // focus folder has no MISSION.md. The rendered prompt must surface
    // no Read instruction for a non-existent file (the retired
    // "ungrouped" marker is gone).
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
    let assert = cmd
        .assert()
        .success()
        .stdout(contains("SPEC-0042"))
        .stdout(contains(".speccy/specs/auth/0042-signup/SPEC.md"));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        !stdout.contains("MISSION.md"),
        "no MISSION.md exists on disk, so the rendered prompt must not name one: {stdout}",
    );
    assert!(
        !stdout.contains("no parent MISSION.md"),
        "the retired `no parent MISSION.md` marker must not appear: {stdout}",
    );
    assert!(
        !stdout.contains("## Mission context"),
        "the `## Mission context` heading must be suppressed when no MISSION.md exists: {stdout}",
    );
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
