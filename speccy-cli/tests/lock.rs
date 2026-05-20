#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy lock SPEC-NNNN` (SPEC-0033 REQ-002).
//!
//! Exercises CHK-003 (happy-path hash + timestamp rewrite) and CHK-004
//! (SPEC.md parse-failure precondition) plus the SPEC-not-found and
//! `--help` listing scenarios from TASKS T-002.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;

#[test]
fn lock_writes_hash_and_rfc3339_timestamp_into_tasks_md_frontmatter() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let tasks_md = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        !tasks_md.contains("bootstrap-pending"),
        "lock should replace the bootstrap sentinel: {tasks_md}",
    );
    let hash_line = tasks_md
        .lines()
        .find(|l| l.starts_with("spec_hash_at_generation:"))
        .expect("frontmatter must declare spec_hash_at_generation");
    let hash_value = hash_line
        .strip_prefix("spec_hash_at_generation:")
        .map(str::trim)
        .expect("prefix matched by find()");
    assert_eq!(
        hash_value.len(),
        64,
        "sha256 must render as 64 lowercase hex chars: {hash_value}",
    );
    assert!(
        hash_value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
        "hash must be lowercase ASCII hex: {hash_value}",
    );

    let ts_line = tasks_md
        .lines()
        .find(|l| l.starts_with("generated_at:"))
        .expect("frontmatter must declare generated_at");
    let ts_value = ts_line
        .strip_prefix("generated_at:")
        .map(str::trim)
        .expect("prefix matched by find()");
    // RFC-3339 with trailing `Z`, e.g. `2026-05-19T15:30:42Z`.
    assert_eq!(ts_value.len(), 20, "RFC-3339 Z form: {ts_value}");
    assert!(ts_value.ends_with('Z'), "expected Z suffix: {ts_value}");
    assert!(
        ts_value.chars().nth(10) == Some('T') && ts_value.chars().nth(4) == Some('-'),
        "expected ISO date shape: {ts_value}",
    );
    Ok(())
}

#[test]
fn lock_missing_spec_exits_one_with_not_found_message() -> TestResult {
    let ws = Workspace::new()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
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
fn lock_spec_md_parse_failure_exits_one_and_tasks_md_unchanged() -> TestResult {
    let ws = Workspace::new()?;
    let malformed_spec_md = "no frontmatter\n";
    let tasks_before = bootstrap_tasks_md("SPEC-0001");
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        malformed_spec_md,
        &valid_spec_toml(),
        Some(&tasks_before),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("failed to parse"));

    let tasks_after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert_eq!(
        tasks_before, tasks_after,
        "TASKS.md must be byte-identical on SPEC.md parse failure",
    );
    Ok(())
}

#[test]
fn lock_appears_in_help_subcommands() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("--help");
    cmd.assert().success().stdout(contains("lock"));
    Ok(())
}

#[test]
fn lock_invalid_spec_id_format_exits_two() -> TestResult {
    let ws = Workspace::new()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("invalid SPEC-ID"));
    Ok(())
}

#[test]
fn lock_outside_workspace_exits_one_with_clear_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn lock_missing_tasks_md_exits_one_without_creating_file() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("TASKS.md not found"));

    assert!(
        !spec_dir.join("TASKS.md").as_std_path().exists(),
        "lock must not create a missing TASKS.md",
    );
    Ok(())
}

#[test]
fn lock_preserves_body_bytes_byte_identical() -> TestResult {
    let ws = Workspace::new()?;
    let body = "\n# Tasks\n\n<tasks spec=\"SPEC-0001\">\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nfirst\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n</tasks>\n";
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
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        after.ends_with(body),
        "body bytes (after the closing `---` fence) must remain byte-identical: {after}",
    );
    Ok(())
}
