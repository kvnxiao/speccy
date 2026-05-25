#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Priority logic tests for `speccy_core::next::compute_for_spec` and
//! `speccy_core::next::compute_workspace`. Covers SPEC-0007 CHK-001,
//! CHK-002, CHK-005, and CHK-006 (the CHK-003/CHK-004 `--kind` filter
//! tests were removed in SPEC-0033 T-010 when `KindFilter` was deleted).

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::next::NextAction;
use speccy_core::next::compute_for_spec;
use speccy_core::next::compute_workspace;
use speccy_core::workspace::scan;
use std::fmt::Write as _;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

fn valid_spec_md(id: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-05-11
        ---

        # __ID__

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        covers REQ-001
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template.replace("__ID__", id)
}

/// Build a TASKS.md with the given list of `(state, id, title)` rows.
fn tasks_md(spec_id: &str, rows: &[(char, &str, &str)]) -> String {
    let mut body = String::new();
    write!(
        body,
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n",
    )
    .expect("writes to String are infallible");
    for (state, id, title) in rows {
        let state_str = state_for(*state);
        write!(
            body,
            "<task id=\"{id}\" state=\"{state_str}\" covers=\"REQ-001\">\n{title}\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
        )
        .expect("writes to String are infallible");
    }
    body.push('\n');
    body
}

fn state_for(state: char) -> &'static str {
    match state {
        '~' => "in-progress",
        '?' => "in-review",
        'x' => "completed",
        _ => "pending",
    }
}

fn write_spec(
    project_root: &Utf8Path,
    dir_name: &str,
    spec_id: &str,
    tasks_rows: Option<&[(char, &str, &str)]>,
) -> TestResult<Utf8PathBuf> {
    let dir = project_root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), valid_spec_md(spec_id))?;
    if let Some(rows) = tasks_rows {
        fs_err::write(dir.join("TASKS.md").as_std_path(), tasks_md(spec_id, rows))?;
    }
    Ok(dir)
}

// -- CHK-001 ----------------------------------------------------------------
// Within a single spec, in-review beats pending (priority rule 2 > 3).

#[test]
fn chk001_in_review_beats_pending_within_spec() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[
            (' ', "T-001", "open one"),
            ('?', "T-002", "awaiting review"),
            (' ', "T-003", "open two"),
        ]),
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("spec with in-review task must have an action");
    assert!(
        matches!(
            &action,
            NextAction::Review { task_id, .. }
                if task_id == "T-002",
        ),
        "in-review T-002 must beat pending T-001/T-003, got {action:?}",
    );

    // in-progress (claimed) must not win over pending within the same spec.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed"), (' ', "T-002", "open")]),
    )?;
    let ws2 = scan(&root2);
    let spec2 = ws2.specs.first().expect("workspace must contain SPEC-0001");
    let action2 = compute_for_spec(spec2).expect("spec with pending task must have an action");
    assert!(
        matches!(&action2, NextAction::Work { task_id } if task_id == "T-002"),
        "in-progress T-001 must be skipped; pending T-002 wins, got {action2:?}",
    );
    Ok(())
}

// -- CHK-002 ----------------------------------------------------------------
// Workspace ordering: the lowest spec-ID entry is first in compute_workspace.

#[test]
fn chk002_workspace_entries_ordered_by_spec_id() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[(' ', "T-001", "open in SPEC-0001")]),
    )?;
    write_spec(
        &root,
        "0002-bar",
        "SPEC-0002",
        Some(&[('?', "T-002", "review in SPEC-0002")]),
    )?;
    let ws = scan(&root);
    let entries = compute_workspace(&ws);
    assert_eq!(entries.len(), 2, "both specs must appear");
    let e0 = entries.first().expect("first entry must exist");
    let e1 = entries.get(1).expect("second entry must exist");
    assert_eq!(e0.spec_id, "SPEC-0001", "SPEC-0001 must be first");
    assert_eq!(e1.spec_id, "SPEC-0002", "SPEC-0002 must be second");
    // SPEC-0001 has a pending task, so its action is Work.
    assert!(
        matches!(&e0.action, NextAction::Work { task_id } if task_id == "T-001"),
        "SPEC-0001 must return Work T-001, got {:?}",
        e0.action,
    );
    // SPEC-0002 has an in-review task.
    assert!(
        matches!(&e1.action, NextAction::Review { task_id, .. } if task_id == "T-002"),
        "SPEC-0002 must return Review T-002, got {:?}",
        e1.action,
    );

    // When SPEC-0001 only has in-progress tasks, it still appears (Decompose
    // fallback), and SPEC-0002's action is unchanged.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed")]),
    )?;
    write_spec(
        &root2,
        "0002-bar",
        "SPEC-0002",
        Some(&[(' ', "T-002", "open")]),
    )?;
    let ws2 = scan(&root2);
    let entries2 = compute_workspace(&ws2);
    // SPEC-0001 with only in-progress falls through to Decompose defensive default.
    // SPEC-0002 has a pending task → Work.
    let spec2_entry = entries2
        .iter()
        .find(|e| e.spec_id == "SPEC-0002")
        .expect("SPEC-0002 must appear in workspace listing");
    assert!(
        matches!(&spec2_entry.action, NextAction::Work { task_id } if task_id == "T-002"),
        "SPEC-0002 must return Work T-002, got {:?}",
        spec2_entry.action,
    );
    Ok(())
}

// Helper: write a VET.md ending with the given gate block to the spec's
// journal directory.
fn write_vet_md(spec_dir: &Utf8Path, verdict: &str, tasks_hash: &str) -> TestResult<()> {
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let body = format!(
        "## Invocation 1\n\n<gate verdict=\"{verdict}\" tasks_hash=\"{tasks_hash}\" date=\"2026-05-22T00:00:00Z\">\nstub.\n</gate>\n",
    );
    fs_err::write(journal.join("VET.md").as_std_path(), body)?;
    Ok(())
}

fn sha256_hex_of_file(path: &Utf8Path) -> TestResult<String> {
    use sha2::Digest as _;
    let bytes = fs_err::read(path.as_std_path())?;
    Ok(const_hex::encode(sha2::Sha256::digest(&bytes)))
}

// -- SPEC-0041 REQ-001/REQ-002 ----------------------------------------------
// All tasks completed + no VET.md → Vet.

#[test]
fn vet_when_all_done_no_vet_md() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("all-done spec must have an action");
    assert!(
        matches!(action, NextAction::Vet),
        "all-completed + no VET.md must return Vet, got {action:?}",
    );
    Ok(())
}

// All tasks completed + VET.md ending with verdict="failed" → Vet.

#[test]
fn vet_when_vet_md_ends_with_failed_verdict() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    write_vet_md(&spec_dir, "failed", "deadbeef")?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("must have an action");
    assert!(
        matches!(action, NextAction::Vet),
        "VET.md with failed verdict must return Vet, got {action:?}",
    );
    Ok(())
}

// All tasks completed + VET.md ending with passed but stale tasks_hash → Vet.

#[test]
fn vet_when_passed_but_tasks_hash_is_stale() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    write_vet_md(
        &spec_dir,
        "passed",
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("must have an action");
    assert!(
        matches!(action, NextAction::Vet),
        "stale tasks_hash must force re-vet, got {action:?}",
    );
    Ok(())
}

// All tasks completed + passing-fresh VET.md + REPORT.md absent → Ship.

#[test]
fn ship_when_vet_passes_fresh_and_no_report() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    let hash = sha256_hex_of_file(&spec_dir.join("TASKS.md"))?;
    write_vet_md(&spec_dir, "passed", &hash)?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("must have an action");
    assert!(
        matches!(action, NextAction::Ship),
        "fresh passing gate without REPORT.md must return Ship, got {action:?}",
    );
    Ok(())
}

// All tasks completed + passing-fresh VET.md + REPORT.md present → None.

#[test]
fn none_when_vet_passes_fresh_and_report_present() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    let hash = sha256_hex_of_file(&spec_dir.join("TASKS.md"))?;
    write_vet_md(&spec_dir, "passed", &hash)?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0001\n---\n",
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    assert!(
        compute_for_spec(spec).is_none(),
        "fresh-pass + REPORT.md must return None",
    );
    Ok(())
}

// -- SPEC-0043 REQ-002 ------------------------------------------------------
// REPORT.md presence is terminal regardless of journal/VET.md state.

// All tasks completed + REPORT.md present + no VET.md → None.

#[test]
fn report_md_beats_missing_vet_md() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0001\n---\n",
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    assert!(
        compute_for_spec(spec).is_none(),
        "REPORT.md present + no VET.md must return None (terminal)",
    );
    Ok(())
}

// All tasks completed + REPORT.md present + stale-hash VET.md → None.

#[test]
fn report_md_beats_stale_vet_md() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    write_vet_md(
        &spec_dir,
        "passed",
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0001\n---\n",
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    assert!(
        compute_for_spec(spec).is_none(),
        "REPORT.md present + stale VET.md must return None (REPORT.md wins)",
    );
    Ok(())
}

// All tasks completed + REPORT.md present + failed-verdict VET.md → None.

#[test]
fn report_md_beats_failed_vet_md() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    write_vet_md(&spec_dir, "failed", "deadbeef")?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0001\n---\n",
    )?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    assert!(
        compute_for_spec(spec).is_none(),
        "REPORT.md present + failed VET.md must return None (REPORT.md wins)",
    );
    Ok(())
}

// Priority: in-review task always wins over a fresh-pass VET.md.

#[test]
fn review_wins_over_fresh_pass_vet() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('?', "T-001", "awaiting review")]),
    )?;
    let hash = sha256_hex_of_file(&spec_dir.join("TASKS.md"))?;
    write_vet_md(&spec_dir, "passed", &hash)?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("must have an action");
    assert!(
        matches!(&action, NextAction::Review { task_id, .. } if task_id == "T-001"),
        "in-review task must win over any VET.md state, got {action:?}",
    );
    Ok(())
}

// -- CHK-005 ----------------------------------------------------------------
// All tasks completed + fresh-pass VET.md + no REPORT.md → Ship; + REPORT.md
// → omit from listing.

#[test]
fn chk005_ship_when_all_done_no_report() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_dir = write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    let hash = sha256_hex_of_file(&spec_dir.join("TASKS.md"))?;
    write_vet_md(&spec_dir, "passed", &hash)?;
    let ws = scan(&root);
    let spec = ws.specs.first().expect("workspace must contain SPEC-0001");
    let action = compute_for_spec(spec).expect("all-done spec without REPORT.md must have Ship");
    assert!(
        matches!(action, NextAction::Ship),
        "all-[x] spec with fresh-pass VET.md and no REPORT.md must return Ship, got {action:?}",
    );

    // Two all-done specs; SPEC-0001 has REPORT.md, SPEC-0002 does not.
    // compute_workspace must include SPEC-0002 (Ship) and omit SPEC-0001.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    let dir1 = write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    let hash1 = sha256_hex_of_file(&dir1.join("TASKS.md"))?;
    write_vet_md(&dir1, "passed", &hash1)?;
    fs_err::write(
        dir1.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0001\n---\n",
    )?;
    let dir2 = write_spec(
        &root2,
        "0002-bar",
        "SPEC-0002",
        Some(&[('x', "T-002", "done")]),
    )?;
    let hash2 = sha256_hex_of_file(&dir2.join("TASKS.md"))?;
    write_vet_md(&dir2, "passed", &hash2)?;
    let ws2 = scan(&root2);
    let entries = compute_workspace(&ws2);
    assert_eq!(
        entries.len(),
        1,
        "SPEC-0001 (with REPORT.md) must be omitted; only SPEC-0002 active",
    );
    let e = entries.first().expect("one entry must exist");
    assert_eq!(e.spec_id, "SPEC-0002");
    assert!(
        matches!(e.action, NextAction::Ship),
        "SPEC-0002 must return Ship, got {:?}",
        e.action,
    );

    // All specs done + REPORT.md present → empty workspace listing.
    let dir2 = root2.join(".speccy").join("specs").join("0002-bar");
    fs_err::write(
        dir2.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0002\n---\n",
    )?;
    let ws3 = scan(&root2);
    let entries3 = compute_workspace(&ws3);
    assert!(
        entries3.is_empty(),
        "all specs reported: workspace listing must be empty, got {entries3:?}",
    );
    Ok(())
}

// -- CHK-006 ----------------------------------------------------------------
// Edge cases: empty workspace, all claimed.

#[test]
fn chk006_workspace_edge_cases() -> TestResult {
    // Empty workspace → empty listing.
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    fs_err::create_dir_all(root.join(".speccy").as_std_path())?;
    let ws = scan(&root);
    let entries = compute_workspace(&ws);
    assert!(
        entries.is_empty(),
        "empty workspace must yield an empty listing, got {entries:?}",
    );

    // All in-progress (claimed) → Decompose defensive default, not empty.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed")]),
    )?;
    let ws2 = scan(&root2);
    let spec2 = ws2.specs.first().expect("workspace must contain SPEC-0001");
    // compute_for_spec returns Decompose when only in-progress tasks exist.
    let action2 = compute_for_spec(spec2).expect("in-progress spec must still have an action");
    assert!(
        matches!(action2, NextAction::Decompose),
        "all-in-progress spec must fall back to Decompose, got {action2:?}",
    );
    Ok(())
}
