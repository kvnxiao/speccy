#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for the SPEC-0058 task-lock reap on the `--to completed`
//! transition (REQ-001 / REQ-003).
//!
//! Drives the built `speccy` binary against scratch workspaces. The
//! load-bearing scenarios are CHK-001 (an `in-review` -> `completed`
//! transition unlinks a free `<task-id>.md.lock` while leaving the journal
//! `.md` byte-identical), CHK-002 (the same transition with no sidecar exits
//! zero and leaves the journal unchanged), CHK-005 (a sidecar held by the test
//! process survives the reap and the transition still exits zero), and the
//! REQ-001 done-when negative case (a non-`completed` edge leaves an existing
//! sidecar in place).

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use fs4::FileExt;

/// Build a workspace with one in-progress spec carrying a single task in the
/// given starting `state`, returning the workspace and the spec dir.
fn workspace_with_task(state: &str) -> TestResult<(Workspace, Utf8PathBuf)> {
    let ws = Workspace::new()?;
    let spec_id = "SPEC-0042";
    let tasks_md = tasks_md_xml(spec_id, &task_xml("T-001", state));
    let dir = write_spec(
        &ws.root,
        "0042-example-slug",
        &spec_md_template(spec_id, "in-progress"),
        Some(&tasks_md),
    )?;
    Ok((ws, dir))
}

/// Path of the task journal `.md` for `T-001` under `spec_dir`.
fn journal_md(spec_dir: &Utf8Path) -> Utf8PathBuf {
    spec_dir.join("journal").join("T-001.md")
}

/// Path of the task journal lock sidecar for `T-001` under `spec_dir`.
fn lock_sidecar(spec_dir: &Utf8Path) -> Utf8PathBuf {
    spec_dir.join("journal").join("T-001.md.lock")
}

/// Seed a `journal/` directory with a `T-001.md` journal and return its path.
fn seed_journal(spec_dir: &Utf8Path) -> TestResult<Utf8PathBuf> {
    let journal = journal_md(spec_dir);
    fs_err::create_dir_all(spec_dir.join("journal").as_std_path())?;
    fs_err::write(journal.as_std_path(), b"journal body\n")?;
    Ok(journal)
}

/// Create an empty lock sidecar beside the journal (no lock held) and return
/// its path.
fn seed_free_sidecar(spec_dir: &Utf8Path) -> TestResult<Utf8PathBuf> {
    let sidecar = lock_sidecar(spec_dir);
    fs_err::write(sidecar.as_std_path(), b"")?;
    Ok(sidecar)
}

fn transition_to(ws: &Workspace, to: &str) -> Command {
    let mut cmd = Command::cargo_bin("speccy").expect("speccy binary should build");
    cmd.args(["task", "transition", "SPEC-0042/T-001", "--to", to])
        .current_dir(ws.root.as_std_path());
    cmd
}

/// CHK-001: a task whose journal `.md` and `<task-id>.md.lock` both exist
/// transitions `in-review` -> `completed`; afterward the sidecar is gone and
/// the journal `.md` is byte-identical to its pre-transition bytes.
#[test]
fn completed_transition_reaps_free_sidecar_and_leaves_journal_intact() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("in-review")?;
    let journal = seed_journal(&spec_dir)?;
    let sidecar = seed_free_sidecar(&spec_dir)?;
    let journal_before = fs_err::read(journal.as_std_path())?;

    transition_to(&ws, "completed").assert().success();

    assert!(
        !sidecar.as_std_path().exists(),
        "the free lock sidecar must be reaped by the --to completed transition",
    );
    let journal_after = fs_err::read(journal.as_std_path())?;
    assert_eq!(
        journal_before, journal_after,
        "the journal .md must be byte-identical: the reap never touches it",
    );
    Ok(())
}

/// CHK-002: a task with a journal but no lock sidecar transitions to
/// `completed`; the command exits zero and the journal is unchanged (the reap
/// is an idempotent no-op against an absent sidecar).
#[test]
fn completed_transition_with_no_sidecar_exits_zero_and_leaves_journal_intact() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("in-review")?;
    let journal = seed_journal(&spec_dir)?;
    let sidecar = lock_sidecar(&spec_dir);
    assert!(
        !sidecar.as_std_path().exists(),
        "precondition: no sidecar present",
    );
    let journal_before = fs_err::read(journal.as_std_path())?;

    transition_to(&ws, "completed").assert().success();

    assert!(
        !sidecar.as_std_path().exists(),
        "the absent sidecar must never be created by the reap",
    );
    let journal_after = fs_err::read(journal.as_std_path())?;
    assert_eq!(
        journal_before, journal_after,
        "the journal .md must be unchanged",
    );
    Ok(())
}

/// CHK-005: with the `<task-id>.md.lock` held by an exclusive advisory lock
/// from the test process, a `--to completed` transition exits zero and the
/// sidecar still exists afterward (the `try_lock` guard skipped the held
/// lock).
#[test]
fn completed_transition_skips_held_sidecar_and_still_exits_zero() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("in-review")?;
    seed_journal(&spec_dir)?;
    let sidecar = seed_free_sidecar(&spec_dir)?;

    // Hold the same sidecar lock the reap probes, from this test process.
    let held = fs_err::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(sidecar.as_std_path())?;
    let (held_std, _p) = held.into_parts();
    FileExt::lock(&held_std)?;

    transition_to(&ws, "completed").assert().success();

    assert!(
        sidecar.as_std_path().exists(),
        "a held sidecar must be left intact: the try_lock guard skips it (REQ-003)",
    );

    // Release for tidy teardown.
    FileExt::unlock(&held_std)?;
    Ok(())
}

/// REQ-001 done-when / behavior: a non-`completed` edge (here
/// `pending` -> `in-progress`) leaves an existing lock sidecar in place — only
/// the terminal `--to completed` boundary reaps.
#[test]
fn non_completed_transition_leaves_sidecar_in_place() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("pending")?;
    seed_journal(&spec_dir)?;
    let sidecar = seed_free_sidecar(&spec_dir)?;

    transition_to(&ws, "in-progress").assert().success();

    assert!(
        sidecar.as_std_path().exists(),
        "a non-completed transition must not touch the lock sidecar (REQ-001)",
    );
    Ok(())
}
