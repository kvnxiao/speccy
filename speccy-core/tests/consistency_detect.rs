#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation"
)]
//! SPEC-0045 T-005 behavioural tests for `speccy_core::consistency::detect`.
//!
//! Each test builds a tempdir spec fixture (SPEC.md + TASKS.md +
//! optionally a `journal/T-NNN.md` file), parses it through the shipped
//! `workspace::parse_spec_dir`, and drives `detect` with a `FakeProbe`
//! that returns canned `git log` / `git status` answers. The fake's
//! `is_git_repo()` returns `true` so the state-vs-commit correlation
//! branches run.

use camino::Utf8PathBuf;
use speccy_core::consistency::ConsistencyStatus;
use speccy_core::consistency::DriftDetails;
use speccy_core::consistency::DriftKind;
use speccy_core::consistency::DriftSeverity;
use speccy_core::consistency::GitProbe;
use speccy_core::consistency::detect;
use speccy_core::workspace::parse_spec_dir;
use std::collections::HashMap;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const VALID_SPEC_MD: &str = "---\nid: SPEC-0099\nslug: example\ntitle: Example\nstatus: in-progress\ncreated: 2026-05-21\nsupersedes: []\n---\n\n# SPEC-0099: Example\n\n## Summary\n\nNotes.\n\n## Requirements\n\n<requirement id=\"REQ-001\">\n### REQ-001\n\n<done-when>\n- thing.\n</done-when>\n\n<behavior>\n- thing.\n</behavior>\n\n<scenario id=\"CHK-001\">\nWhen X then Y.\n</scenario>\n</requirement>\n";

struct FakeProbe {
    sha_for_prefix: HashMap<String, String>,
    porcelain: Vec<String>,
}

impl FakeProbe {
    fn new() -> Self {
        Self {
            sha_for_prefix: HashMap::new(),
            porcelain: Vec::new(),
        }
    }
    fn with_commit(mut self, prefix: &str, sha: &str) -> Self {
        self.sha_for_prefix
            .insert(prefix.to_owned(), sha.to_owned());
        self
    }
    fn with_porcelain(mut self, lines: &[&str]) -> Self {
        self.porcelain = lines.iter().map(|s| (*s).to_owned()).collect();
        self
    }
}

impl GitProbe for FakeProbe {
    fn is_git_repo(&self) -> bool {
        true
    }
    fn first_commit_sha_with_title_prefix(&self, prefix: &str) -> Option<String> {
        self.sha_for_prefix.get(prefix).cloned()
    }
    fn porcelain_status(&self) -> Vec<String> {
        self.porcelain.clone()
    }
}

fn make_spec_dir(
    tasks_xml: &str,
    journal_files: &[(&str, &str)],
) -> TestResult<(TempDir, Utf8PathBuf)> {
    let dir = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let spec_dir = root.join("0099-example");
    fs_err::create_dir_all(spec_dir.as_std_path())?;
    fs_err::write(spec_dir.join("SPEC.md").as_std_path(), VALID_SPEC_MD)?;
    let tasks_md = format!(
        "---\nspec: SPEC-0099\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n# Tasks: SPEC-0099\n\n{tasks_xml}\n"
    );
    fs_err::write(spec_dir.join("TASKS.md").as_std_path(), tasks_md)?;
    if !journal_files.is_empty() {
        let journal_dir = spec_dir.join("journal");
        fs_err::create_dir_all(journal_dir.as_std_path())?;
        for (name, content) in journal_files {
            fs_err::write(journal_dir.join(name).as_std_path(), content)?;
        }
    }
    Ok((dir, spec_dir))
}

fn one_task(state: &str) -> String {
    format!(
        "<task id=\"T-001\" state=\"{state}\" covers=\"REQ-001\">\ndo it\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n"
    )
}

#[test]
fn detect_state_completed_no_commit_dirty_tree_is_blocking() -> TestResult {
    let (_tmp, spec_dir) = make_spec_dir(&one_task("completed"), &[])?;
    let spec = parse_spec_dir(&spec_dir);
    let probe = FakeProbe::new().with_porcelain(&[" M src/foo.rs", "?? newfile"]);

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Blocked);
    assert_eq!(block.drifts.len(), 1);
    let d = block.drifts.first().ok_or("one drift entry")?;
    assert_eq!(d.task_id, "T-001");
    assert_eq!(d.kind, DriftKind::StateCompletedNoCommit);
    assert_eq!(d.severity, DriftSeverity::Blocking);
    assert_eq!(d.tasks_state, "completed");
    match &d.details {
        DriftDetails::StateCompletedNoCommit {
            expected_trailer,
            working_tree_dirty,
        } => {
            assert_eq!(expected_trailer, "[SPEC-0099/T-001]:");
            assert!(*working_tree_dirty);
        }
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_state_completed_no_commit_clean_tree_is_blocking() -> TestResult {
    let (_tmp, spec_dir) = make_spec_dir(&one_task("completed"), &[])?;
    let spec = parse_spec_dir(&spec_dir);
    let probe = FakeProbe::new(); // clean tree, no commit

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Blocked);
    let d = block.drifts.first().ok_or("one drift entry")?;
    assert_eq!(d.kind, DriftKind::StateCompletedNoCommit);
    match &d.details {
        DriftDetails::StateCompletedNoCommit {
            working_tree_dirty, ..
        } => assert!(!*working_tree_dirty),
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_commit_without_state_is_auto_fixable_drift() -> TestResult {
    // Task is in-review with a matching commit. The (_, Some(sha))
    // arm fires for any non-completed state.
    let (_tmp, spec_dir) = make_spec_dir(&one_task("in-review"), &[])?;
    let spec = parse_spec_dir(&spec_dir);
    let sha = "abcdef0123456789abcdef0123456789abcdef01";
    let probe = FakeProbe::new().with_commit("[SPEC-0099/T-001]:", sha);

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Drift);
    let d = block.drifts.first().ok_or("one drift entry")?;
    assert_eq!(d.kind, DriftKind::CommitWithoutState);
    assert_eq!(d.severity, DriftSeverity::AutoFixable);
    assert_eq!(d.tasks_state, "in-review");
    match &d.details {
        DriftDetails::CommitWithoutState {
            commit_sha,
            commit_short_sha,
        } => {
            assert_eq!(commit_sha, sha);
            assert_eq!(commit_sha.len(), 40);
            assert_eq!(commit_short_sha, sha.get(..8).ok_or("8-char prefix")?);
            assert_eq!(commit_short_sha.len(), 8);
        }
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_state_in_progress_orphaned_is_blocking_when_dirty() -> TestResult {
    let (_tmp, spec_dir) = make_spec_dir(&one_task("in-progress"), &[])?;
    let spec = parse_spec_dir(&spec_dir);
    let probe = FakeProbe::new().with_porcelain(&[" M a", " M b", "?? c"]);

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Blocked);
    let d = block.drifts.first().ok_or("one drift entry")?;
    assert_eq!(d.kind, DriftKind::StateInProgressOrphaned);
    assert_eq!(d.severity, DriftSeverity::Blocking);
    assert_eq!(d.tasks_state, "in-progress");
    match &d.details {
        DriftDetails::StateInProgressOrphaned {
            working_tree_dirty,
            dirty_files_count,
        } => {
            assert!(*working_tree_dirty);
            assert_eq!(*dirty_files_count, 3);
        }
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_state_in_progress_clean_tree_is_blocking_with_new_kind() -> TestResult {
    // SPEC-0045 REQ-006 fifth drift kind: in-progress + clean tree
    // + no matching commit. The reconcile pass owns this case
    // autonomously per DEC-004; the orchestrator startup check
    // no longer surfaces a user-facing fork for it.
    let (_tmp, spec_dir) = make_spec_dir(&one_task("in-progress"), &[])?;
    let spec = parse_spec_dir(&spec_dir);
    let probe = FakeProbe::new();

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Blocked);
    assert_eq!(block.drifts.len(), 1);
    let d = block.drifts.first().ok_or("one drift entry")?;
    assert_eq!(d.task_id, "T-001");
    assert_eq!(d.kind, DriftKind::StateInProgressClean);
    assert_eq!(d.severity, DriftSeverity::Blocking);
    assert_eq!(d.tasks_state, "in-progress");
    match &d.details {
        DriftDetails::StateInProgressClean { working_tree_dirty } => {
            assert!(!*working_tree_dirty);
        }
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_journal_xml_malformed_is_blocking_with_forward_slash_path() -> TestResult {
    // Journal with valid frontmatter, a well-formed implementer block,
    // then a trailing whitelisted open tag with no matching close ->
    // strict parse rejects it (unbalanced element) while the tolerant
    // `scan_tags` recovery still pairs the first block. There is no
    // matching commit and the task is completed, so we *also* expect a
    // StateCompletedNoCommit drift; assert on the malformed entry
    // specifically. Use a clean tree and a fake commit to isolate the
    // journal-malformed case. Frontmatter is required: the recovery
    // helper reuses `scan_tags` behind `split_required`, so a journal
    // missing frontmatter recovers to offset 0 (SPEC-0062 CHK-003); the
    // non-zero recovery offset asserted here depends on the frontmatter
    // being present.
    let malformed = "---\nspec: SPEC-0099\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\nbody\n</implementer>\n<implementer date=\"2026-05-21T19:00:00Z\" model=\"m\" round=\"2\">";
    let (_tmp, spec_dir) = make_spec_dir(&one_task("completed"), &[("T-001.md", malformed)])?;
    let spec = parse_spec_dir(&spec_dir);
    let sha = "0".repeat(40);
    let probe = FakeProbe::new().with_commit("[SPEC-0099/T-001]:", &sha);

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Blocked);
    let d = block
        .drifts
        .iter()
        .find(|d| d.kind == DriftKind::JournalXmlMalformed)
        .ok_or("journal_xml_malformed drift expected")?;
    assert_eq!(d.severity, DriftSeverity::Blocking);
    assert_eq!(d.task_id, "T-001");
    assert_eq!(d.tasks_state, "completed");
    match &d.details {
        DriftDetails::JournalXmlMalformed {
            journal_path,
            last_well_formed_byte_offset,
        } => {
            assert!(
                !journal_path.contains('\\'),
                "journal_path must be forward-slash normalised: {journal_path}"
            );
            assert!(
                journal_path.ends_with("journal/T-001.md"),
                "journal_path should end with journal/T-001.md: {journal_path}",
            );
            let expected =
                malformed.find("</implementer>").ok_or("close present")? + "</implementer>".len();
            assert_eq!(*last_well_formed_byte_offset, expected);
        }
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_journal_xml_malformed_recovery_offset_ignores_fenced_close() -> TestResult {
    // SPEC-0062 REQ-002 / CHK-004 regression. A journal with valid
    // frontmatter and one well-formed `<implementer>` block whose
    // structural close ends at byte X, then a second `<implementer>`
    // open whose only following `</implementer>` is line-isolated
    // *inside a fenced code block* (its close ending at byte Y > X),
    // then no real close. Strict `journal_xml::parse` fails (the
    // round-2 open never closes, because the fence-aware `scan_tags`
    // excludes the fenced occurrence), so the malformed branch runs.
    //
    // The recovery offset must be X (the real structural close), not Y
    // (the fenced occurrence). Because T-001 moved the read path onto
    // the fence-aware `journal_xml::last_well_formed_offset`, the fenced
    // close is excluded for free.
    //
    // Recorded pre-fix measurement (CHK-004 "recorded pre-fix run"):
    // the pre-SPEC hand-rolled `find('<')` scan (recovered from the
    // merged SPEC-0061 revision and run once against this exact fixture)
    // counted the fenced close and yielded Y = 235 — the fence-blindness
    // bug this test guards. The post-fix value asserted below is X = 153.
    // The blank line before the fence is load-bearing: it makes the
    // markdown block parser recognize a fenced code block rather than
    // folding the ``` into the preceding HTML block.
    let malformed = concat!(
        "---\nspec: SPEC-0099\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n",
        "<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\n",
        "body\n",
        "</implementer>\n",
        "<implementer date=\"2026-05-21T19:00:00Z\" model=\"m\" round=\"2\">\n",
        "\n",
        "```\n",
        "</implementer>\n",
        "```\n",
    );
    // X: end of the first (real, structural) close. The marker is the
    // unique boundary between the first close and the second open.
    let first_close_marker = "</implementer>\n<implementer date=\"2026-05-21T19";
    let x = malformed
        .find(first_close_marker)
        .ok_or("first close present")?
        + "</implementer>".len();
    // Y: end of the fenced (non-structural) close — the last occurrence.
    let y = malformed
        .rfind("</implementer>")
        .ok_or("fenced close present")?
        + "</implementer>".len();
    assert!(
        y > x,
        "fenced close (Y={y}) must be after structural close (X={x})"
    );

    let (_tmp, spec_dir) = make_spec_dir(&one_task("completed"), &[("T-001.md", malformed)])?;
    let spec = parse_spec_dir(&spec_dir);
    let sha = "0".repeat(40);
    let probe = FakeProbe::new().with_commit("[SPEC-0099/T-001]:", &sha);

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Blocked);
    let d = block
        .drifts
        .iter()
        .find(|d| d.kind == DriftKind::JournalXmlMalformed)
        .ok_or("journal_xml_malformed drift expected")?;
    match &d.details {
        DriftDetails::JournalXmlMalformed {
            last_well_formed_byte_offset,
            ..
        } => {
            assert_eq!(
                *last_well_formed_byte_offset, x,
                "recovery offset must be the structural close X={x}, \
                 not the fenced occurrence Y={y}",
            );
        }
        other => return Err(format!("unexpected details: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn detect_all_healthy_is_ok() -> TestResult {
    // Completed task with a matching commit and clean tree.
    let (_tmp, spec_dir) = make_spec_dir(&one_task("completed"), &[])?;
    let spec = parse_spec_dir(&spec_dir);
    let probe = FakeProbe::new().with_commit("[SPEC-0099/T-001]:", &"f".repeat(40));

    let block = detect("SPEC-0099", &spec, &probe);

    assert_eq!(block.status, ConsistencyStatus::Ok);
    assert!(block.drifts.is_empty());
    Ok(())
}
