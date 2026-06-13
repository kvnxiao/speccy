#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `workspace::stale_for`.

use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::parse::SpecMd;
use speccy_core::parse::TasksDoc;
use speccy_core::parse::parse_task_xml;
use speccy_core::parse::spec_md;
use speccy_core::workspace::StaleReason;
use speccy_core::workspace::stale_for;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SPEC_MD: &str = indoc! {r"
    ---
    id: SPEC-0001
    slug: x
    title: Test
    status: in-progress
    created: 2026-05-11
    ---

    # Test
"};

struct Fixture {
    _dir: TempDir,
    spec_md_path: Utf8PathBuf,
    tasks_md_path: Utf8PathBuf,
}

fn write_fixture(spec_md_body: &str, tasks_md_body: &str) -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let spec_md_path = root.join("SPEC.md");
    let tasks_md_path = root.join("TASKS.md");
    fs_err::write(spec_md_path.as_std_path(), spec_md_body)?;
    fs_err::write(tasks_md_path.as_std_path(), tasks_md_body)?;
    Ok(Fixture {
        _dir: dir,
        spec_md_path,
        tasks_md_path,
    })
}

fn hex_of(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        if write!(s, "{b:02x}").is_err() {
            break;
        }
    }
    s
}

fn parse_pair(fx: &Fixture) -> TestResult<(SpecMd, TasksDoc)> {
    let spec = spec_md(&fx.spec_md_path)?;
    let raw = fs_err::read_to_string(fx.tasks_md_path.as_std_path())?;
    let tasks = parse_task_xml(&raw, &fx.tasks_md_path)?;
    Ok((spec, tasks))
}

#[test]
fn no_tasks_md_yields_fresh() {
    // Construct a SpecMd inline to avoid needing files when we don't
    // actually have a tasks file.
    let dir = tempfile::tempdir().expect("tempdir");
    let root =
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("tempdir path must be UTF-8");
    let spec_path = root.join("SPEC.md");
    fs_err::write(spec_path.as_std_path(), SPEC_MD).expect("write spec");
    let spec = spec_md(&spec_path).expect("parse spec");

    let result = stale_for(&spec, None);
    assert!(!result.stale);
    assert!(result.reasons.is_empty());
}

#[test]
fn fresh_when_hash_matches() -> TestResult {
    let spec = SPEC_MD;
    // Pre-compute sha256 of spec to put it into tasks frontmatter.
    let tmp = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let spec_md_path = root.join("SPEC.md");
    fs_err::write(spec_md_path.as_std_path(), spec)?;
    let parsed_spec = spec_md(&spec_md_path)?;
    let hash = hex_of(&parsed_spec.sha256);

    let tasks_body = format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: {hash}\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n\n\n\n"
    );
    let tasks_md_path = root.join("TASKS.md");
    fs_err::write(tasks_md_path.as_std_path(), &tasks_body)?;
    let parsed_tasks = parse_task_xml(&tasks_body, &tasks_md_path)?;

    let result = stale_for(&parsed_spec, Some(&parsed_tasks));
    assert!(!result.stale, "should be fresh, got: {:?}", result.reasons);
    assert!(result.reasons.is_empty());
    Ok(())
}

#[test]
fn hash_mismatch_yields_hash_drift() -> TestResult {
    let tasks_body = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks: SPEC-0001

                    "};
    let fx = write_fixture(SPEC_MD, tasks_body)?;
    let (spec, tasks) = parse_pair(&fx)?;

    let result = stale_for(&spec, Some(&tasks));
    assert!(result.stale);
    assert!(result.reasons.contains(&StaleReason::HashDrift));
    Ok(())
}

#[test]
fn hash_drift_fires_alone_when_spec_body_changes() -> TestResult {
    let tasks_body = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: 0000000000000000000000000000000000000000000000000000000000000000
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks: SPEC-0001

                    "};
    let fx = write_fixture(SPEC_MD, tasks_body)?;
    let (spec, tasks) = parse_pair(&fx)?;

    let result = stale_for(&spec, Some(&tasks));
    assert!(result.stale);
    assert_eq!(result.reasons, vec![StaleReason::HashDrift]);
    Ok(())
}

#[test]
fn bootstrap_pending_short_circuits_other_reasons() -> TestResult {
    let tasks_body = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks: SPEC-0001

                    "};
    let fx = write_fixture(SPEC_MD, tasks_body)?;
    let (spec, tasks) = parse_pair(&fx)?;

    let result = stale_for(&spec, Some(&tasks));
    assert!(result.stale);
    assert_eq!(result.reasons, vec![StaleReason::BootstrapPending]);
    Ok(())
}
