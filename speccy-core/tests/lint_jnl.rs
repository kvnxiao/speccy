#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation"
)]
//! SPEC-0037 T-001 integration tests for the JNL-* lint family.
//!
//! Scenario coverage is drawn directly from T-001's `<task-scenarios>`
//! block. Fixtures live under tempdirs — never the real `.speccy/specs/`
//! tree.

use camino::Utf8PathBuf;
use speccy_core::lint::Diagnostic;
use speccy_core::lint::run;
use speccy_core::parse::supersession::supersession_index;
use speccy_core::workspace::scan;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const VALID_SPEC_MD: &str = "---\nid: SPEC-0042\nslug: example\ntitle: Example\nstatus: in-progress\ncreated: 2026-05-21\nsupersedes: []\n---\n\n# SPEC-0042: Example\n\n## Summary\n\nNotes.\n\n## Requirements\n\n<requirement id=\"REQ-001\">\n### REQ-001\n\n<done-when>\n- thing.\n</done-when>\n\n<behavior>\n- thing.\n</behavior>\n\n<scenario id=\"CHK-001\">\nWhen X then Y.\n</scenario>\n</requirement>\n";

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

fn make_workspace(
    spec_id: &str,
    tasks_xml: &str,
    journal_files: &[(&str, &str)],
) -> TestResult<TempDir> {
    let dir = tempfile::tempdir()?;
    let root = utf8(&dir)?;
    let spec_dir_name = format!("{:0>4}-example", spec_id.trim_start_matches("SPEC-"));
    let spec_dir = root.join(".speccy").join("specs").join(&spec_dir_name);
    fs_err::create_dir_all(spec_dir.as_std_path())?;
    let spec_body = VALID_SPEC_MD.replace("SPEC-0042", spec_id);
    fs_err::write(spec_dir.join("SPEC.md").as_std_path(), spec_body)?;
    let tasks_md = format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{tasks_xml}\n"
    );
    fs_err::write(spec_dir.join("TASKS.md").as_std_path(), tasks_md)?;
    if !journal_files.is_empty() {
        let journal_dir = spec_dir.join("journal");
        fs_err::create_dir_all(journal_dir.as_std_path())?;
        for (name, content) in journal_files {
            fs_err::write(journal_dir.join(name).as_std_path(), content)?;
        }
    }
    Ok(dir)
}

fn run_lint(dir: &TempDir) -> TestResult<Vec<Diagnostic>> {
    let root = utf8(dir)?;
    let scanned = scan(root.as_path());
    let parsed_specs: Vec<_> = scanned.specs;
    let spec_md_refs: Vec<&_> = parsed_specs
        .iter()
        .filter_map(|s| s.spec_md.as_ref().ok())
        .collect();
    let supersession = supersession_index(&spec_md_refs);
    let workspace = speccy_core::lint::types::Workspace {
        specs: &parsed_specs,
        supersession: &supersession,
    };
    Ok(run(&workspace))
}

#[test]
fn jnl_001_fires_when_pending_task_has_journal_file() -> TestResult {
    let tasks = r#"<task id="T-001" state="pending" covers="REQ-001">
do it
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let journal_body = "---\nspec: SPEC-0042\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\nbody\n</implementer>\n";
    let dir = make_workspace("SPEC-0042", tasks, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    let hit = diags
        .iter()
        .find(|d| d.code == "JNL-001")
        .ok_or("JNL-001 should fire")?;
    assert!(
        hit.file
            .as_ref()
            .is_some_and(|p| p.as_str().ends_with("T-001.md")),
        "JNL-001 file should point at journal file: {:?}",
        hit.file
    );
    Ok(())
}

#[test]
fn jnl_002_fires_when_completed_task_missing_journal() -> TestResult {
    let tasks = r#"<task id="T-002" state="completed" covers="REQ-001">
done
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    let hit = diags
        .iter()
        .find(|d| d.code == "JNL-002")
        .ok_or("JNL-002 should fire")?;
    assert!(hit.message.contains("T-002"));
    Ok(())
}

#[test]
fn jnl_003_fires_when_filename_task_mismatch() -> TestResult {
    let tasks = r#"<task id="T-003" state="completed" covers="REQ-001">
done
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let journal_body = "---\nspec: SPEC-0042\ntask: T-999\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\nbody\n</implementer>\n";
    let dir = make_workspace("SPEC-0042", tasks, &[("T-003.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    let hit = diags
        .iter()
        .find(|d| d.code == "JNL-003" && d.message.contains("mismatch"))
        .ok_or("JNL-003 should fire on binding mismatch")?;
    assert!(hit.message.contains("T-999"));
    Ok(())
}

#[test]
fn jnl_003_fires_on_first_round_not_1() -> TestResult {
    let tasks = r#"<task id="T-001" state="completed" covers="REQ-001">
done
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let journal_body = "---\nspec: SPEC-0042\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"2\">\nbody\n</implementer>\n";
    let dir = make_workspace("SPEC-0042", tasks, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    let hit = diags
        .iter()
        .find(|d| d.code == "JNL-003" && d.message.contains("first block must have round"))
        .ok_or("JNL-003 should fire on first-round-not-1")?;
    assert!(hit.message.contains("round=\"1\""));
    Ok(())
}

#[test]
fn jnl_skips_in_progress_state() -> TestResult {
    let tasks = r#"<task id="T-004" state="in-progress" covers="REQ-001">
working
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    // No journal file present; if JNL-002 weren't skipped it would fire.
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags.iter().any(|d| d.code.starts_with("JNL-")),
        "no JNL-* should fire for in-progress task: {diags:?}"
    );
    Ok(())
}

#[test]
fn jnl_skips_in_review_state() -> TestResult {
    let tasks = r#"<task id="T-005" state="in-review" covers="REQ-001">
reviewing
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags.iter().any(|d| d.code.starts_with("JNL-")),
        "no JNL-* should fire for in-review task: {diags:?}"
    );
    Ok(())
}

#[test]
fn jnl_silent_on_pending_task_without_journal() -> TestResult {
    let tasks = r#"<task id="T-001" state="pending" covers="REQ-001">
fresh
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags.iter().any(|d| d.code.starts_with("JNL-")),
        "no JNL-* should fire for clean-slate pending task: {diags:?}"
    );
    Ok(())
}

#[test]
fn jnl_silent_on_completed_task_with_valid_journal() -> TestResult {
    let tasks = r#"<task id="T-001" state="completed" covers="REQ-001">
done
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let journal_body = "---\nspec: SPEC-0042\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"claude-opus-4.8[1m]/low\" round=\"1\">\nbody\n</implementer>\n";
    let dir = make_workspace("SPEC-0042", tasks, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags.iter().any(|d| d.code.starts_with("JNL-")),
        "no JNL-* should fire for valid completed-task journal: {diags:?}"
    );
    Ok(())
}

#[test]
fn tsk_006_fires_on_misplaced_implementer_in_tasks_md() -> TestResult {
    let tasks = r#"<task id="T-001" state="completed" covers="REQ-001">
done
<task-scenarios>
- placeholder.
</task-scenarios>
<implementer date="2026-05-21T18:00:00Z" model="m" round="1">
body
</implementer>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    let hit = diags
        .iter()
        .find(|d| d.code == "TSK-006")
        .ok_or("TSK-006 should fire on misplaced <implementer> in TASKS.md")?;
    assert!(hit.message.contains("journal/T-001.md"));
    Ok(())
}

#[test]
fn tsk_006_fires_for_blockers_at_any_state() -> TestResult {
    let tasks = r#"<task id="T-001" state="pending" covers="REQ-001">
work
<task-scenarios>
- placeholder.
</task-scenarios>
<blockers date="2026-05-21T18:00:00Z" round="1">
do
</blockers>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    assert!(diags.iter().any(|d| d.code == "TSK-006"));
    Ok(())
}

#[test]
fn bare_task_at_top_level_parses_cleanly() -> TestResult {
    let tasks = r#"<task id="T-001" state="pending" covers="REQ-001">
work
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    // No parse errors and no JNL on a clean pending task.
    assert!(
        !diags
            .iter()
            .any(|d| d.code.starts_with("JNL-") || d.code == "TSK-006"),
        "clean bare <task> should not trip JNL-* or TSK-006: {diags:?}"
    );
    Ok(())
}

#[test]
fn appending_a_new_task_does_not_trip_tsk006_or_staleness() -> TestResult {
    let tasks = r#"<task id="T-001" state="completed" covers="REQ-001">
done
<task-scenarios>
- placeholder.
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-001">
new
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let journal_body = "---\nspec: SPEC-0042\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\nb\n</implementer>\n";
    let dir = make_workspace("SPEC-0042", tasks, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    assert!(!diags.iter().any(|d| d.code == "TSK-006"));
    Ok(())
}

#[test]
fn tsk_006_fires_for_misplaced_review_in_tasks_md() -> TestResult {
    let tasks = r#"<task id="T-002" state="in-progress" covers="REQ-001">
work
<task-scenarios>
- placeholder.
</task-scenarios>
<review persona="tests" verdict="pass" date="2026-05-21T18:00:00Z" model="m" round="1">
body
</review>
</task>
"#;
    let dir = make_workspace("SPEC-0042", tasks, &[])?;
    let diags = run_lint(&dir)?;
    assert!(
        diags.iter().any(|d| d.code == "TSK-006"),
        "TSK-006 should fire on misplaced <review> in TASKS.md: {diags:?}"
    );
    Ok(())
}

/// Cycle the same task through every state and confirm no JNL-* or TSK-006
/// diagnostic fires at any transition where the on-disk shape is consistent.
#[test]
fn lifecycle_state_transitions_stay_quiet() -> TestResult {
    let journal_body = "---\nspec: SPEC-0042\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\nbody\n</implementer>\n";

    // pending, no journal — clean slate.
    let pending = r#"<task id="T-001" state="pending" covers="REQ-001">
w
<task-scenarios>
- placeholder.
</task-scenarios>
</task>
"#;
    let dir = make_workspace("SPEC-0042", pending, &[])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags
            .iter()
            .any(|d| d.code.starts_with("JNL-") || d.code == "TSK-006"),
        "pending: {diags:?}"
    );

    // in-progress — JNL family is skipped entirely.
    let in_progress = pending.replace("state=\"pending\"", "state=\"in-progress\"");
    let dir = make_workspace("SPEC-0042", &in_progress, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags
            .iter()
            .any(|d| d.code.starts_with("JNL-") || d.code == "TSK-006"),
        "in-progress: {diags:?}"
    );

    // in-review — JNL family still skipped.
    let in_review = pending.replace("state=\"pending\"", "state=\"in-review\"");
    let dir = make_workspace("SPEC-0042", &in_review, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags
            .iter()
            .any(|d| d.code.starts_with("JNL-") || d.code == "TSK-006"),
        "in-review: {diags:?}"
    );

    // completed + valid journal — silent.
    let completed = pending.replace("state=\"pending\"", "state=\"completed\"");
    let dir = make_workspace("SPEC-0042", &completed, &[("T-001.md", journal_body)])?;
    let diags = run_lint(&dir)?;
    assert!(
        !diags
            .iter()
            .any(|d| d.code.starts_with("JNL-") || d.code == "TSK-006"),
        "completed: {diags:?}"
    );

    Ok(())
}
