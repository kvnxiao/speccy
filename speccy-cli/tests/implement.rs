#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::unwrap_in_result,
    reason = "test code may .expect() with descriptive messages inside TestResult fns"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy implement`.
//!
//! - `prompt_renders_*` exercises SPEC-0008 REQ-004 (CHK-004): template
//!   loading, placeholder substitution, budget trimming, stdout emission.
//! - `error_paths_and_integration_*` exercises REQ-005 (CHK-005): exit codes
//!   for `InvalidFormat` / `NotFound` / `Ambiguous` / outside-workspace, plus
//!   the happy path through the binary.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::implement::ImplementArgs;
use speccy_cli::implement::run;

// -- Helpers -----------------------------------------------------------------

fn write_agents(ws: &Workspace, body: &str) -> TestResult {
    fs_err::write(ws.root.join("AGENTS.md").as_std_path(), body)?;
    Ok(())
}

fn tasks_md_with(spec_id: &str, body: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

fn capture_stdout(ws: &Workspace, task_ref: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    run(
        &ImplementArgs {
            task_ref: task_ref.to_owned(),
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-004: prompt_renders ------------------------------------------------

#[test]
fn prompt_renders_succeeds_for_unique_match() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\nuse rust\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n  - Suggested files: `src/auth/signup.rs`\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    assert!(
        out.contains("Speccy: Implement `T-001` for `SPEC-0001`"),
        "output missing header: {out}"
    );
    Ok(())
}

#[test]
fn prompt_renders_substitutes_every_placeholder() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents conventions go here\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n  - Suggested files: `src/auth/signup.rs`, `tests/auth/signup_spec.rs`\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    // {{spec_id}}, {{task_id}}
    assert!(out.contains("`T-001`"), "task_id placeholder missing");
    assert!(out.contains("`SPEC-0001`"), "spec_id placeholder missing");
    // {{spec_md}}
    assert!(out.contains("Example SPEC-0001"), "spec_md content missing");
    assert!(
        out.contains("### REQ-001: First"),
        "spec REQ heading missing"
    );
    // {{task_entry}}
    assert!(
        out.contains("**T-001**: implement signup"),
        "task_entry missing"
    );
    assert!(
        out.contains("Covers: REQ-001"),
        "task sub-list bullets missing"
    );
    // {{suggested_files}}
    assert!(
        out.contains("src/auth/signup.rs, tests/auth/signup_spec.rs"),
        "suggested_files placeholder not formatted as CSV: {out}",
    );
    // {{agents}}
    assert!(
        out.contains("Agents conventions go here"),
        "agents placeholder missing: {out}",
    );
    // No raw placeholder left unsubstituted.
    assert!(
        !out.contains("{{spec_id}}"),
        "spec_id placeholder not substituted"
    );
    assert!(
        !out.contains("{{task_entry}}"),
        "task_entry placeholder not substituted"
    );
    Ok(())
}

#[test]
fn prompt_renders_task_entry_preserves_all_sublist_notes_in_order() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [?] **T-001**: password storage\n",
            "  - Covers: REQ-001\n",
            "  - Suggested files: `src/auth/password.rs`\n",
            "  - Implementer note (session-abc): added bcrypt at cost 10.\n",
            "  - Review (business, pass): matches REQ-001 intent.\n",
            "  - Review (tests, pass): hash assertion present.\n",
            "  - Review (security, blocking): bcrypt cost 10 < required 12.\n",
            "  - Retry: address bcrypt cost.\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    let pos_implementer = out
        .find("Implementer note (session-abc)")
        .expect("implementer note line present");
    let pos_review_biz = out
        .find("Review (business, pass)")
        .expect("business review present");
    let pos_review_security = out
        .find("Review (security, blocking)")
        .expect("security review present");
    let pos_retry = out
        .find("Retry: address bcrypt")
        .expect("retry note present");
    assert!(
        pos_implementer < pos_review_biz
            && pos_review_biz < pos_review_security
            && pos_review_security < pos_retry,
        "task subtree bullets must appear in declared order",
    );
    Ok(())
}

#[test]
fn prompt_renders_with_missing_agents_md_succeeds_with_marker() -> TestResult {
    let ws = Workspace::new()?;
    // Deliberately do NOT write AGENTS.md.
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    assert!(
        out.contains("AGENTS.md missing"),
        "missing AGENTS.md should leave the marker in the rendered prompt: {out}",
    );
    Ok(())
}

#[test]
fn prompt_renders_qualified_form_resolves_correctly() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks_a = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
    );
    let tasks_b = tasks_md_with(
        "SPEC-0002",
        "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_a),
    )?;
    write_spec(
        &ws.root,
        "0002-bar",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_b),
    )?;

    let out = capture_stdout(&ws, "SPEC-0002/T-001")?;
    assert!(
        out.contains("in spec-2"),
        "qualified must pick SPEC-0002: {out}"
    );
    assert!(
        !out.contains("in spec-1"),
        "qualified must NOT include SPEC-0001 task: {out}"
    );
    Ok(())
}

// -- CHK-005: error_paths_and_integration -----------------------------------

#[test]
fn error_paths_and_integration_valid_task_exits_zero() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("Speccy: Implement `T-001`"))
        .stdout(contains("**T-001**: implement signup"));
    Ok(())
}

#[test]
fn error_paths_and_integration_invalid_format_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("invalid task reference `FOO`"))
        .stderr(contains("T-NNN"));
    Ok(())
}

#[test]
fn error_paths_and_integration_not_found_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0001",
            "- [ ] **T-001**: real task\n  - Covers: REQ-001\n",
        )),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-999")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("not found"))
        .stderr(contains("speccy status"));
    Ok(())
}

#[test]
fn error_paths_and_integration_ambiguous_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "0001-a",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0001",
            "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
        )),
    )?;
    write_spec(
        &ws.root,
        "0002-b",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0002",
            "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
        )),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("T-001 is ambiguous"))
        .stderr(contains("speccy implement SPEC-0001/T-001"))
        .stderr(contains("speccy implement SPEC-0002/T-001"));
    Ok(())
}

#[test]
fn error_paths_and_integration_outside_workspace_exits_one() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn error_paths_and_integration_missing_positional_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement").current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("missing required TASK-ID"));
    Ok(())
}

#[test]
fn error_paths_and_integration_unknown_flag_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}

#[test]
fn error_paths_and_integration_help_succeeds() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement").arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("usage: speccy implement TASK-ID"));
    Ok(())
}
