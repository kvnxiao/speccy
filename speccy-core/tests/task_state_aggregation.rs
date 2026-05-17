#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `workspace::TaskCounts::from_tasks` and the
//! workspace-level wiring. Covers SPEC-0004 CHK-004.

use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::parse::parse_task_xml;
use speccy_core::workspace::TaskCounts;
use speccy_core::workspace::scan;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

struct TasksFixture {
    _dir: TempDir,
    path: Utf8PathBuf,
}

fn write_tasks(content: &str) -> TestResult<TasksFixture> {
    let dir = tempfile::tempdir()?;
    let path = Utf8PathBuf::from_path_buf(dir.path().join("TASKS.md"))
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    fs_err::write(path.as_std_path(), content)?;
    Ok(TasksFixture { _dir: dir, path })
}

fn parse(path: &camino::Utf8Path) -> TestResult<speccy_core::parse::TasksDoc> {
    let raw = fs_err::read_to_string(path.as_std_path())?;
    Ok(parse_task_xml(&raw, path)?)
}

#[test]
fn counts_match_state_distribution() -> TestResult {
    let src = indoc! {r#"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks: SPEC-0001

        <tasks spec="SPEC-0001">

        <task id="T-001" state="pending" covers="REQ-001">
        a
        <task-scenarios>
        - placeholder.
        </task-scenarios>
        </task>

        <task id="T-002" state="pending" covers="REQ-001">
        b
        <task-scenarios>
        - placeholder.
        </task-scenarios>
        </task>

        <task id="T-003" state="in-progress" covers="REQ-001">
        c
        <task-scenarios>
        - placeholder.
        </task-scenarios>
        </task>

        <task id="T-004" state="in-review" covers="REQ-001">
        d
        <task-scenarios>
        - placeholder.
        </task-scenarios>
        </task>

        <task id="T-005" state="completed" covers="REQ-001">
        e
        <task-scenarios>
        - placeholder.
        </task-scenarios>
        </task>

        </tasks>
    "#};
    let fx = write_tasks(src)?;
    let parsed = parse(&fx.path)?;
    let counts = TaskCounts::from_tasks(&parsed);
    assert_eq!(
        counts,
        TaskCounts {
            open: 2,
            in_progress: 1,
            awaiting_review: 1,
            done: 1,
        }
    );
    Ok(())
}

#[test]
fn empty_tasks_yield_zero_counts() -> TestResult {
    let src = indoc! {r#"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks: SPEC-0001

        <tasks spec="SPEC-0001">
        </tasks>
    "#};
    let fx = write_tasks(src)?;
    let parsed = parse(&fx.path)?;
    let counts = TaskCounts::from_tasks(&parsed);
    assert_eq!(counts, TaskCounts::default());
    assert_eq!(counts.open, 0);
    assert_eq!(counts.in_progress, 0);
    assert_eq!(counts.awaiting_review, 0);
    assert_eq!(counts.done, 0);
    Ok(())
}

#[test]
fn workspace_specs_without_tasks_md_have_zero_via_default() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let dir = root.join(".speccy").join("specs").join("0001-foo");
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(
        dir.join("SPEC.md").as_std_path(),
        indoc! {r"
            ---
            id: SPEC-0001
            slug: foo
            title: Foo
            status: in-progress
            created: 2026-05-11
            ---

            ### REQ-001: First
        "},
    )?;
    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    // No TASKS.md: counts default to zero.
    assert!(only.tasks_md.is_none());
    let counts = only
        .tasks_md_ok()
        .map_or(TaskCounts::default(), TaskCounts::from_tasks);
    assert_eq!(counts, TaskCounts::default());
    Ok(())
}
