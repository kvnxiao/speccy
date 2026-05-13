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
//! End-to-end tests for `speccy report`.
//!
//! - `spec_lookup_and_validation_*` exercises SPEC-0011 REQ-001 (CHK-001):
//!   SPEC-ID format validation, spec-not-found, TASKS.md required, and
//!   parse-failure error surfaces.
//! - `completeness_gate_*` exercises REQ-002 (CHK-002): refuses when any task
//!   is [ ] / [~] / [?]; renders only when all are [x]; empty TASKS.md is
//!   vacuously complete.
//! - `retry_count_*` exercises REQ-003 (CHK-003): retry markers counted per
//!   task with exact `Retry:` prefix and rendered as a markdown list.
//! - `prompt_renders_and_integration_*` exercises REQ-004 (CHK-004): template
//!   loading, placeholder substitution, budget trimming, stdout emission, and
//!   the end-to-end CLI exit-code contract.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::report::ReportArgs;
use speccy_cli::report::ReportError;
use speccy_cli::report::run;

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

fn capture_stdout(ws: &Workspace, spec_id: &str) -> Result<String, ReportError> {
    let mut buf = Vec::new();
    run(
        &ReportArgs {
            spec_id: spec_id.to_owned(),
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf).expect("rendered prompt should be valid UTF-8"))
}

fn capture_err(ws: &Workspace, spec_id: &str) -> ReportError {
    let mut buf = Vec::new();
    let result = run(
        &ReportArgs {
            spec_id: spec_id.to_owned(),
        },
        &ws.root,
        &mut buf,
    );
    result.expect_err("report should fail")
}

// -- CHK-001: spec_lookup_and_validation ------------------------------------

#[test]
fn spec_lookup_and_validation_invalid_format_errors() -> TestResult {
    let ws = Workspace::new()?;
    let err = capture_err(&ws, "FOO");
    assert!(
        matches!(err, ReportError::InvalidSpecIdFormat { ref arg } if arg == "FOO"),
        "expected InvalidSpecIdFormat for `FOO`, got: {err:?}",
    );
    Ok(())
}

#[test]
fn spec_lookup_and_validation_spec_not_found_errors() -> TestResult {
    let ws = Workspace::new()?;
    let err = capture_err(&ws, "SPEC-9999");
    assert!(
        matches!(err, ReportError::SpecNotFound { ref id } if id == "SPEC-9999"),
        "expected SpecNotFound for `SPEC-9999`, got: {err:?}",
    );
    Ok(())
}

#[test]
fn spec_lookup_and_validation_tasks_md_required_errors() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;
    let err = capture_err(&ws, "SPEC-0001");
    assert!(
        matches!(err, ReportError::TasksMdRequired { ref id } if id == "SPEC-0001"),
        "expected TasksMdRequired for SPEC-0001, got: {err:?}",
    );
    Ok(())
}

#[test]
fn spec_lookup_and_validation_malformed_tasks_md_surfaces_parse_error() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    // Frontmatter is unterminated; tasks_md parser will return ParseError.
    let broken = "---\nspec: SPEC-0001\nspec_hash_at_generation: x\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(broken),
    )?;
    let err = capture_err(&ws, "SPEC-0001");
    assert!(
        matches!(err, ReportError::Parse { artifact, ref id, .. }
            if artifact == "TASKS.md" && id == "SPEC-0001"),
        "expected Parse(TASKS.md) for SPEC-0001, got: {err:?}",
    );
    Ok(())
}

#[test]
fn spec_lookup_and_validation_outside_workspace_errors() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let mut buf = Vec::new();
    let result = run(
        &ReportArgs {
            spec_id: "SPEC-0001".to_owned(),
        },
        &path,
        &mut buf,
    );
    let err = result.expect_err("must fail outside a speccy workspace");
    assert!(
        matches!(err, ReportError::ProjectRootNotFound),
        "expected ProjectRootNotFound, got: {err:?}",
    );
    Ok(())
}

// -- CHK-002: completeness_gate ---------------------------------------------

#[test]
fn completeness_gate_refuses_when_any_task_open() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [x] **T-001**: done one\n",
            "  - Covers: REQ-001\n",
            "- [x] **T-002**: done two\n",
            "  - Covers: REQ-001\n",
            "- [ ] **T-003**: still open\n",
            "  - Covers: REQ-001\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let err = capture_err(&ws, "SPEC-0001");
    let (id, offending) = match err {
        ReportError::Incomplete { id, offending } => (id, offending),
        other => return Err(format!("expected Incomplete, got: {other:?}").into()),
    };
    assert_eq!(id, "SPEC-0001");
    assert_eq!(offending.len(), 1, "exactly one offender: {offending:?}");
    let first = offending.first().expect("first offender present");
    assert_eq!(first.id, "T-003");
    Ok(())
}

#[test]
fn completeness_gate_lists_in_progress_offender() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [~] **T-001**: running\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let err = capture_err(&ws, "SPEC-0001");
    let offending = match err {
        ReportError::Incomplete { offending, .. } => offending,
        other => return Err(format!("expected Incomplete, got: {other:?}").into()),
    };
    let first = offending.first().expect("first offender present");
    assert_eq!(first.id, "T-001");
    assert_eq!(
        first.state,
        speccy_core::parse::TaskState::InProgress,
        "must surface InProgress state",
    );
    Ok(())
}

#[test]
fn completeness_gate_lists_awaiting_review_offender() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [?] **T-001**: awaiting review\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let err = capture_err(&ws, "SPEC-0001");
    let offending = match err {
        ReportError::Incomplete { offending, .. } => offending,
        other => return Err(format!("expected Incomplete, got: {other:?}").into()),
    };
    let first = offending.first().expect("first offender present");
    assert_eq!(first.id, "T-001");
    assert_eq!(
        first.state,
        speccy_core::parse::TaskState::AwaitingReview,
        "must surface AwaitingReview state",
    );
    Ok(())
}

#[test]
fn completeness_gate_lists_every_offender() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [ ] **T-001**: open\n",
            "  - Covers: REQ-001\n",
            "- [~] **T-002**: running\n",
            "  - Covers: REQ-001\n",
            "- [?] **T-003**: awaiting\n",
            "  - Covers: REQ-001\n",
            "- [x] **T-004**: done\n",
            "  - Covers: REQ-001\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let err = capture_err(&ws, "SPEC-0001");
    let offending = match err {
        ReportError::Incomplete { offending, .. } => offending,
        other => return Err(format!("expected Incomplete, got: {other:?}").into()),
    };
    let ids: Vec<String> = offending.iter().map(|t| t.id.clone()).collect();
    assert_eq!(
        ids,
        vec!["T-001".to_owned(), "T-002".to_owned(), "T-003".to_owned()],
        "every non-done task should appear in declared order",
    );
    Ok(())
}

#[test]
fn completeness_gate_renders_when_all_done() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [x] **T-001**: done one\n",
            "  - Covers: REQ-001\n",
            "- [x] **T-002**: done two\n",
            "  - Covers: REQ-001\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let out = capture_stdout(&ws, "SPEC-0001")?;
    assert!(out.contains("Speccy: Report `SPEC-0001`"), "got: {out}");
    Ok(())
}

#[test]
fn completeness_gate_renders_when_no_task_lines() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    // TASKS.md with frontmatter but no task list at all.
    let tasks = "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(tasks),
    )?;
    let out = capture_stdout(&ws, "SPEC-0001")?;
    assert!(
        out.contains("Speccy: Report `SPEC-0001`"),
        "empty tasks should be vacuously complete: {out}",
    );
    Ok(())
}

// -- CHK-003: retry_count ---------------------------------------------------

#[test]
fn retry_count_appears_in_rendered_retry_summary() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [x] **T-001**: password storage\n",
            "  - Covers: REQ-001\n",
            "  - Implementer note (session-abc): added bcrypt at cost 10.\n",
            "  - Review (security, blocking): cost 10 < required 12.\n",
            "  - Retry: address bcrypt cost.\n",
            "  - Implementer note (session-def): bumped cost to 12.\n",
            "  - Review (style, blocking): nit.\n",
            "  - Retry: fix style.\n",
            "  - Implementer note (session-ghi): style polished.\n",
            "  - Review (security, pass): OK.\n",
            "- [x] **T-002**: no retries here\n",
            "  - Covers: REQ-002\n",
            "  - Review (business, pass): OK.\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let out = capture_stdout(&ws, "SPEC-0001")?;
    assert!(
        out.contains("- T-001: 2 retries"),
        "expected `- T-001: 2 retries` in retry summary, got: {out}",
    );
    assert!(
        out.contains("- T-002: 0 retries"),
        "expected `- T-002: 0 retries` in retry summary, got: {out}",
    );
    Ok(())
}

#[test]
fn retry_count_exact_prefix_only_lowercase_ignored() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    // Note: `Retry on bcrypt` (no colon) and `retry: ...` (lowercase) must
    // NOT contribute. Only `Retry:` (exact prefix) counts.
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [x] **T-001**: x\n",
            "  - Covers: REQ-001\n",
            "  - Retry on bcrypt cost\n",
            "  - retry: lowercase ignored\n",
            "  - Retried: past tense ignored\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let out = capture_stdout(&ws, "SPEC-0001")?;
    assert!(
        out.contains("- T-001: 0 retries"),
        "case-sensitive prefix should exclude non-matching notes, got: {out}",
    );
    Ok(())
}

// -- CHK-004: prompt_renders_and_integration --------------------------------

#[test]
fn prompt_renders_and_integration_substitutes_every_placeholder() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents conventions go here\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [x] **T-001**: implement signup\n",
            "  - Covers: REQ-001\n",
            "  - Suggested files: `src/auth/signup.rs`\n",
            "  - Retry: bcrypt cost.\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let out = capture_stdout(&ws, "SPEC-0001")?;
    assert!(out.contains("`SPEC-0001`"), "spec_id placeholder missing");
    assert!(
        out.contains("Example SPEC-0001"),
        "spec_md content missing: {out}",
    );
    assert!(
        out.contains("**T-001**: implement signup"),
        "tasks_md content missing",
    );
    assert!(
        out.contains("- T-001: 1 retry"),
        "retry_summary placeholder content missing (singular): {out}",
    );
    assert!(
        out.contains("Agents conventions go here"),
        "agents placeholder missing",
    );
    assert!(
        !out.contains("{{spec_id}}"),
        "spec_id placeholder not substituted",
    );
    assert!(
        !out.contains("{{retry_summary}}"),
        "retry_summary placeholder not substituted",
    );
    assert!(
        !out.contains("{{tasks_md}}"),
        "tasks_md placeholder not substituted",
    );
    Ok(())
}

#[test]
fn prompt_renders_and_integration_missing_agents_md_leaves_marker() -> TestResult {
    let ws = Workspace::new()?;
    // Deliberately do NOT write AGENTS.md.
    let tasks = tasks_md_with("SPEC-0001", "- [x] **T-001**: done\n  - Covers: REQ-001\n");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let out = capture_stdout(&ws, "SPEC-0001")?;
    assert!(
        out.contains("AGENTS.md missing"),
        "missing AGENTS.md should leave the marker in the rendered prompt: {out}",
    );
    Ok(())
}

#[test]
fn prompt_renders_and_integration_cli_success() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with("SPEC-0001", "- [x] **T-001**: done\n  - Covers: REQ-001\n");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("report")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("Speccy: Report `SPEC-0001`"))
        .stdout(contains("- T-001: 0 retries"));
    Ok(())
}

#[test]
fn prompt_renders_and_integration_cli_incomplete_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: still open\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("report")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("has incomplete tasks"))
        .stderr(contains("T-001: [ ]"));
    Ok(())
}

#[test]
fn prompt_renders_and_integration_cli_invalid_format_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("report")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("invalid SPEC-ID `FOO`"));
    Ok(())
}

#[test]
fn prompt_renders_and_integration_cli_outside_workspace_exits_one() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("report")
        .arg("SPEC-0001")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn prompt_renders_and_integration_cli_missing_positional_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("report").current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("missing required SPEC-ID"));
    Ok(())
}

#[test]
fn prompt_renders_and_integration_cli_help_succeeds() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("report").arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("usage: speccy report SPEC-ID"));
    Ok(())
}
