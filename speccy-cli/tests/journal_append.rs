#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy journal append` (SPEC-0055 REQ-003 /
//! REQ-005).
//!
//! Drives the built `speccy` binary against scratch workspaces. The
//! load-bearing scenarios are CHK-004 (a fresh `implementer` append creates
//! the journal with CLI-stamped frontmatter and `round="1"`), CHK-005 (a
//! `review` with an unknown persona exits non-zero and leaves the journal
//! byte-identical), CHK-008 (eight concurrent `review` appends produce eight
//! well-formed blocks the parser accepts), and the REQ-005 done-when timeout
//! (a held lock makes a waiting append exit non-zero after roughly the
//! interval with the journal byte-identical).

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
use predicates::str::contains;
use speccy_core::parse::parse_journal_xml;

/// Build a workspace with one in-progress spec carrying a single
/// `state="pending"` task, returning the workspace and the spec dir.
fn workspace_with_pending_task() -> TestResult<(Workspace, Utf8PathBuf)> {
    let ws = Workspace::new()?;
    let spec_id = "SPEC-0042";
    let tasks_md = tasks_md_xml(spec_id, &task_xml("T-001", "pending"));
    let dir = write_spec(
        &ws.root,
        "0042-example-slug",
        &spec_md_template(spec_id, "in-progress"),
        Some(&tasks_md),
    )?;
    Ok((ws, dir))
}

fn journal_path(spec_dir: &Utf8Path) -> Utf8PathBuf {
    spec_dir.join("journal").join("T-001.md")
}

/// CHK-004: a fresh `implementer` append creates the journal with
/// CLI-stamped frontmatter and a single `round="1"` block whose `date` the
/// caller never supplied.
#[test]
fn fresh_implementer_creates_journal_with_frontmatter_and_round_one() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);
    assert!(!jpath.as_std_path().exists(), "journal must start absent");

    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "implementer",
            "--model",
            "test-model",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("Completed: implemented the thing.")
        .assert()
        .success();

    let src = fs_err::read_to_string(jpath.as_std_path())?;
    let doc = parse_journal_xml(&src, &jpath)?;
    assert_eq!(doc.spec, "SPEC-0042");
    assert_eq!(doc.task, "T-001");
    // generated_at is CLI-stamped; the parser already enforces ISO8601.
    assert!(
        !doc.generated_at.is_empty(),
        "generated_at must be stamped by the CLI",
    );
    assert_eq!(doc.entries.len(), 1, "exactly one block");
    let entry = doc.entries.first().expect("one entry");
    assert_eq!(entry.round(), 1, "first block must open round 1");
    let speccy_core::parse::JournalEntry::Implementer { date, model, .. } = entry else {
        return Err(format!("expected implementer block, got {entry:?}").into());
    };
    assert_eq!(model, "test-model");
    // The caller passed no date flag (none exists); the CLI stamped one.
    assert!(date.contains('T') && date.ends_with('Z'), "ISO8601 Z date");
    Ok(())
}

/// CHK-005: a `review` append with an unknown persona exits non-zero and
/// leaves the journal byte-identical.
#[test]
fn unknown_persona_review_exits_nonzero_and_leaves_bytes_unchanged() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);

    // First seed an implementer block so a round exists to attach to.
    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "implementer",
            "--model",
            "test-model",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("Completed: seed.")
        .assert()
        .success();

    let before = fs_err::read(jpath.as_std_path())?;

    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "review",
            "--model",
            "test-model",
            "--persona",
            "not-a-persona",
            "--verdict",
            "pass",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("a review body")
        .assert()
        .failure()
        .stderr(contains("invalid persona"));

    let after = fs_err::read(jpath.as_std_path())?;
    assert_eq!(before, after, "journal bytes must be unchanged");
    Ok(())
}

/// REQ-003 done-when: a `review` append to a journal with no `implementer`
/// block exits non-zero, leaving the file still absent.
#[test]
fn review_with_no_round_exits_nonzero_and_journal_stays_absent() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);

    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "review",
            "--model",
            "m",
            "--persona",
            "tests",
            "--verdict",
            "pass",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("a review body")
        .assert()
        .failure();

    assert!(
        !jpath.as_std_path().exists(),
        "no implementer block means no round; journal must stay absent",
    );
    Ok(())
}

/// REQ-003 done-when: an empty stdin body exits non-zero with the journal
/// still absent.
#[test]
fn empty_body_exits_nonzero() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);

    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "implementer",
            "--model",
            "m",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("")
        .assert()
        .failure();

    assert!(
        !jpath.as_std_path().exists(),
        "empty body must abort before any write",
    );
    Ok(())
}

/// CHK-008: eight concurrent processes each append one distinct `review`
/// block to the same journal; the result holds eight well-formed review
/// blocks (plus the seed implementer) with no interleaving, and the parser
/// accepts the file.
#[test]
fn eight_concurrent_review_appends_serialize_cleanly() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);

    // Seed an implementer block (round 1) for the reviews to attach to.
    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "implementer",
            "--model",
            "test-model",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("Completed: seed.")
        .assert()
        .success();

    // Eight distinct personas (the registry has seven; reuse one to reach
    // eight distinct bodies while keeping every persona valid).
    let personas = [
        "business",
        "tests",
        "security",
        "style",
        "correctness",
        "architecture",
        "docs",
        "business",
    ];
    let root = ws.root.clone();
    let handles: Vec<_> = personas
        .into_iter()
        .enumerate()
        .map(|(i, persona)| {
            let root = root.clone();
            std::thread::spawn(move || {
                let bin = assert_cmd::cargo::cargo_bin("speccy");
                let mut cmd = std::process::Command::new(bin);
                cmd.args([
                    "journal",
                    "append",
                    "SPEC-0042/T-001",
                    "--block",
                    "review",
                    "--model",
                    "test-model",
                    "--persona",
                    persona,
                    "--verdict",
                    "pass",
                ])
                .current_dir(root.as_std_path())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
                let mut child = cmd.spawn().expect("spawn append");
                {
                    use std::io::Write as _;
                    let mut stdin = child.stdin.take().expect("child stdin");
                    write!(stdin, "review body number {i}").expect("write stdin");
                }
                child.wait().expect("wait append").success()
            })
        })
        .collect();

    for h in handles {
        let ok = h.join().map_err(|_e| "append thread panicked")?;
        assert!(ok, "every append must succeed");
    }

    let src = fs_err::read_to_string(jpath.as_std_path())?;
    let doc = parse_journal_xml(&src, &jpath)?;
    let reviews = doc
        .entries
        .iter()
        .filter(|e| matches!(e, speccy_core::parse::JournalEntry::Review { .. }))
        .count();
    assert_eq!(reviews, 8, "all eight review blocks must be present");
    Ok(())
}

/// REQ-005 done-when: two concurrent round-opening (`implementer`) appends
/// derive distinct, correctly ordered round numbers because the
/// derive-validate-append sequence runs under the lock.
#[test]
fn two_concurrent_round_opening_appends_get_distinct_rounds() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);

    let root = ws.root.clone();
    let handles: Vec<_> = (0..2)
        .map(|i| {
            let root = root.clone();
            std::thread::spawn(move || {
                let bin = assert_cmd::cargo::cargo_bin("speccy");
                let mut cmd = std::process::Command::new(bin);
                cmd.args([
                    "journal",
                    "append",
                    "SPEC-0042/T-001",
                    "--block",
                    "implementer",
                    "--model",
                    "test-model",
                ])
                .current_dir(root.as_std_path())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
                let mut child = cmd.spawn().expect("spawn append");
                {
                    use std::io::Write as _;
                    let mut stdin = child.stdin.take().expect("child stdin");
                    write!(stdin, "round-opening body {i}").expect("write stdin");
                }
                child.wait().expect("wait append").success()
            })
        })
        .collect();
    for h in handles {
        let ok = h.join().map_err(|_e| "append thread panicked")?;
        assert!(ok, "both appends must succeed");
    }

    let src = fs_err::read_to_string(jpath.as_std_path())?;
    let doc = parse_journal_xml(&src, &jpath)?;
    let mut rounds: Vec<u32> = doc
        .entries
        .iter()
        .map(speccy_core::parse::JournalEntry::round)
        .collect();
    rounds.sort_unstable();
    assert_eq!(
        rounds,
        vec![1, 2],
        "the two appends must take distinct rounds 1 and 2"
    );
    Ok(())
}

/// REQ-005 done-when: a held lock causes a waiting append to exit non-zero
/// after roughly the timeout interval, naming the journal path, with the
/// journal byte-identical.
///
/// To keep the suite fast the test holds the lock and asserts the waiting
/// append fails within a window comfortably larger than the 10s timeout but
/// shorter than an unbounded hang; it does not assert the exact 10s.
#[test]
fn held_lock_makes_waiting_append_time_out_nonzero() -> TestResult {
    let (ws, spec_dir) = workspace_with_pending_task()?;
    let jpath = journal_path(&spec_dir);

    // Seed an implementer block so the journal exists with stable bytes.
    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "implementer",
            "--model",
            "test-model",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("Completed: seed.")
        .assert()
        .success();

    let before = fs_err::read(jpath.as_std_path())?;

    // Hold the same sidecar lock the command uses, from this test process.
    let lock_path = spec_dir.join("journal").join("T-001.md.lock");
    let held = fs_err::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(lock_path.as_std_path())?;
    let (held_std, _p) = held.into_parts();
    FileExt::lock(&held_std)?;

    let start = std::time::Instant::now();
    Command::cargo_bin("speccy")?
        .args([
            "journal",
            "append",
            "SPEC-0042/T-001",
            "--block",
            "review",
            "--model",
            "test-model",
            "--persona",
            "tests",
            "--verdict",
            "pass",
        ])
        .current_dir(ws.root.as_std_path())
        .write_stdin("a review body")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure()
        .stderr(contains("T-001.md"));
    let elapsed = start.elapsed();

    // Release for tidy teardown.
    FileExt::unlock(&held_std)?;

    assert!(
        elapsed >= std::time::Duration::from_secs(9),
        "the append should have blocked roughly the 10s timeout, waited {elapsed:?}",
    );

    let after = fs_err::read(jpath.as_std_path())?;
    assert_eq!(
        before, after,
        "journal must be byte-identical after a timeout"
    );
    Ok(())
}
