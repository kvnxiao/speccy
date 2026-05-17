#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy tasks` (initial + amendment + --commit).
//! Exercises SPEC-0006 REQ-001..REQ-005 through the binary entry point.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;

fn write_agents(ws: &Workspace, body: &str) -> TestResult {
    fs_err::write(ws.root.join("AGENTS.md").as_std_path(), body)?;
    Ok(())
}

#[test]
fn initial_prompt_rendered_when_tasks_md_absent() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0023 REQ-005: AGENTS.md is no longer inlined; modern AI
    // coding harnesses auto-load it themselves. SPEC-0023 REQ-006: the
    // SPEC.md body is also no longer inlined; the rendered prompt names
    // the repo-relative path so the agent reads it via the host's Read
    // primitive on demand.
    write_agents(&ws, "# Agents\nUSE_RUST_SENTINEL\n")?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("Speccy: Tasks (initial decomposition"))
        .stdout(contains("SPEC-0001"))
        .stdout(contains(".speccy/specs/0001-foo/SPEC.md"));
    let out = Command::cargo_bin("speccy")?
        .arg("tasks")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path())
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        !stdout.contains("USE_RUST_SENTINEL"),
        "AGENTS.md body must not be inlined into the rendered prompt: {stdout}",
    );
    assert!(
        !stdout.contains("Example SPEC-0001"),
        "SPEC.md body must not be inlined into the rendered prompt: {stdout}",
    );
    Ok(())
}

#[test]
fn amendment_prompt_rendered_when_tasks_md_present() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\nrules\n")?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    // SPEC-0023 REQ-006: the rendered prompt names the SPEC.md and
    // TASKS.md repo-relative paths; the bodies are not inlined.
    let assert = cmd
        .assert()
        .success()
        .stdout(contains("Speccy: Tasks (amend"))
        .stdout(contains("SPEC-0001"))
        .stdout(contains(".speccy/specs/0001-foo/SPEC.md"))
        .stdout(contains(".speccy/specs/0001-foo/TASKS.md"));
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(
        !stdout.contains("<task id=\"T-001\""),
        "TASKS.md body must not be inlined into the rendered prompt: {stdout}",
    );
    Ok(())
}

#[test]
fn argument_validation_invalid_format_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("invalid SPEC-ID"));
    Ok(())
}

#[test]
fn argument_validation_missing_spec_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
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
fn argument_validation_parse_error_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    let malformed = "no frontmatter\n";
    write_spec(&ws.root, "0001-foo", malformed, &valid_spec_toml(), None)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("failed to parse"));
    Ok(())
}

#[test]
fn argument_validation_missing_positional_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks").current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("missing required SPEC-ID"));
    Ok(())
}

#[test]
fn unknown_flag_exits_with_usage_code() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}

#[test]
fn commit_requires_tasks_md_returns_error_when_absent() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .arg("--commit")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("TASKS.md not found"));
    Ok(())
}

#[test]
fn commit_requires_tasks_md_succeeds_when_present() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .arg("--commit")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success().stdout(predicates::str::is_empty());

    let tasks_md = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        !tasks_md.contains("bootstrap-pending"),
        "--commit should replace the sentinel: {tasks_md}",
    );
    assert!(
        tasks_md.contains("<task id=\"T-001\""),
        "body must be preserved verbatim: {tasks_md}",
    );
    Ok(())
}

#[test]
fn tasks_outside_workspace_exits_with_clear_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn commit_preserves_body_bytes_byte_identical() -> TestResult {
    let ws = Workspace::new()?;
    let body = "\n# Tasks\n\n- [ ] **T-001**: trailing space   \n  - Covers: REQ-001\n";
    let bootstrap = format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---{body}",
    );
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .arg("--commit")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        after.ends_with(body),
        "body bytes (including trailing whitespace) must remain byte-identical after --commit",
    );
    Ok(())
}

#[test]
fn commit_refuses_when_spec_md_id_disagrees_with_folder_and_tasks_md_unchanged() -> TestResult {
    // SPEC-0024 REQ-003: 3-way ID command-guard.
    // Folder = "0001-foo" (→ SPEC-0001); SPEC.md.id = SPEC-1234;
    // TASKS.md.spec = SPEC-0001. CLI arg matches folder/TASKS.md, so
    // the legacy 2-way check would not fire; the 3-way guard must.
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-1234", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;
    let before = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("tasks")
        .arg("SPEC-0001")
        .arg("--commit")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("SPEC-0001"))
        .stderr(contains("SPEC-1234"))
        .stderr(contains("ID disagreement"));

    let after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert_eq!(
        before, after,
        "TASKS.md must be byte-unchanged on 3-way disagreement",
    );
    Ok(())
}
