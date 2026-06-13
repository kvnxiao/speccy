#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy task transition`.
//!
//! Drives the built `speccy` binary against scratch workspaces. The
//! load-bearing scenario: a selector that resolves to no task
//! exits non-zero and leaves TASKS.md byte-identical. The happy-path,
//! illegal-edge, same-state no-op, and unknown-`--to` cases round out the
//! command surface.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use predicates::str::contains;

/// Build a workspace with one in-progress spec carrying a single
/// `state="pending"` task, returning the workspace and the TASKS.md path.
fn workspace_with_pending_task() -> TestResult<(Workspace, camino::Utf8PathBuf)> {
    let ws = Workspace::new()?;
    let spec_id = "SPEC-0042";
    let tasks_md = tasks_md_xml(spec_id, &task_xml("T-001", "pending"));
    let dir = write_spec(
        &ws.root,
        "0042-example-slug",
        &spec_md_template(spec_id, "in-progress"),
        Some(&tasks_md),
    )?;
    Ok((ws, dir.join("TASKS.md")))
}

#[test]
fn legal_edge_rewrites_only_the_state_value() -> TestResult {
    let (ws, tasks_path) = workspace_with_pending_task()?;
    let before = fs_err::read_to_string(tasks_path.as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args([
        "task",
        "transition",
        "SPEC-0042/T-001",
        "--to",
        "in-progress",
    ])
    .current_dir(ws.root.as_std_path())
    .assert()
    .success();

    let after = fs_err::read_to_string(tasks_path.as_std_path())?;
    // The only difference is the single state value.
    assert_eq!(
        after,
        before.replacen("state=\"pending\"", "state=\"in-progress\"", 1),
        "only the state value should change",
    );
    Ok(())
}

/// A selector resolving to no task exits non-zero and leaves
/// TASKS.md byte-identical.
#[test]
fn not_found_selector_exits_nonzero_and_leaves_bytes_unchanged() -> TestResult {
    let (ws, tasks_path) = workspace_with_pending_task()?;
    let before = fs_err::read(tasks_path.as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args(["task", "transition", "SPEC-0042/T-099", "--to", "completed"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .failure()
        .stderr(contains("not found"));

    let after = fs_err::read(tasks_path.as_std_path())?;
    assert_eq!(before, after, "TASKS.md bytes must be unchanged");
    Ok(())
}

#[test]
fn illegal_edge_exits_nonzero_naming_both_states_and_leaves_bytes_unchanged() -> TestResult {
    let (ws, tasks_path) = workspace_with_pending_task()?;
    let before = fs_err::read(tasks_path.as_std_path())?;

    // pending -> completed is not a legal edge.
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args(["task", "transition", "SPEC-0042/T-001", "--to", "completed"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .failure()
        .stderr(contains("pending"))
        .stderr(contains("completed"))
        .stderr(contains("not in the legal state graph"));

    let after = fs_err::read(tasks_path.as_std_path())?;
    assert_eq!(before, after, "illegal edge must leave bytes unchanged");
    Ok(())
}

#[test]
fn same_state_is_a_noop_success_leaving_bytes_unchanged() -> TestResult {
    let (ws, tasks_path) = workspace_with_pending_task()?;
    let before = fs_err::read(tasks_path.as_std_path())?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args(["task", "transition", "SPEC-0042/T-001", "--to", "pending"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();

    let after = fs_err::read(tasks_path.as_std_path())?;
    assert_eq!(before, after, "same-state no-op must leave bytes unchanged");
    Ok(())
}

#[test]
fn unknown_to_value_rejected_at_argument_parse_time() -> TestResult {
    let (ws, _tasks_path) = workspace_with_pending_task()?;

    // `shipped` is not one of the four legal states; clap's value parser
    // rejects it before any workspace resolution. Clap argument errors
    // exit with code 2.
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args(["task", "transition", "SPEC-0042/T-001", "--to", "shipped"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .code(2)
        .stderr(contains("invalid state"));
    Ok(())
}

#[test]
fn unqualified_selector_resolves() -> TestResult {
    let (ws, tasks_path) = workspace_with_pending_task()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.args(["task", "transition", "T-001", "--to", "in-progress"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();

    let after = fs_err::read_to_string(tasks_path.as_std_path())?;
    assert!(
        after.contains("state=\"in-progress\""),
        "unqualified selector should resolve and rewrite the task",
    );
    Ok(())
}
