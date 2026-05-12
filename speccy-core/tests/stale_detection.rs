#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `workspace::stale_for`. Covers SPEC-0004 CHK-003.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::parse::SpecMd;
use speccy_core::parse::TasksMd;
use speccy_core::parse::spec_md;
use speccy_core::parse::tasks_md;
use speccy_core::workspace::StaleReason;
use speccy_core::workspace::stale_for;
use std::time::Duration;
use std::time::SystemTime;
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

fn read_mtime(path: &Utf8Path) -> Option<SystemTime> {
    fs_err::metadata(path.as_std_path())
        .ok()
        .and_then(|m| m.modified().ok())
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

fn parse_pair(fx: &Fixture) -> TestResult<(SpecMd, TasksMd)> {
    let spec = spec_md(&fx.spec_md_path)?;
    let tasks = tasks_md(&fx.tasks_md_path)?;
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
    let mtime = read_mtime(&spec_path);

    let result = stale_for(&spec, None, mtime, None);
    assert!(!result.stale);
    assert!(result.reasons.is_empty());
}

#[test]
fn fresh_when_hash_matches_and_mtime_within() -> TestResult {
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
        "---\nspec: SPEC-0001\nspec_hash_at_generation: {hash}\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks\n"
    );
    let tasks_md_path = root.join("TASKS.md");
    fs_err::write(tasks_md_path.as_std_path(), &tasks_body)?;
    let parsed_tasks = tasks_md(&tasks_md_path)?;

    let spec_mtime = read_mtime(&spec_md_path);
    let tasks_mtime = read_mtime(&tasks_md_path);

    let result = stale_for(&parsed_spec, Some(&parsed_tasks), spec_mtime, tasks_mtime);
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

        # Tasks
    "};
    let fx = write_fixture(SPEC_MD, tasks_body)?;
    let (spec, tasks) = parse_pair(&fx)?;
    let spec_mtime = read_mtime(&fx.spec_md_path);
    let tasks_mtime = read_mtime(&fx.tasks_md_path);

    let result = stale_for(&spec, Some(&tasks), spec_mtime, tasks_mtime);
    assert!(result.stale);
    assert!(result.reasons.contains(&StaleReason::HashDrift));
    Ok(())
}

#[test]
fn mtime_drift_when_spec_newer_than_tasks() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let spec_md_path = root.join("SPEC.md");
    fs_err::write(spec_md_path.as_std_path(), SPEC_MD)?;
    let parsed_spec = spec_md(&spec_md_path)?;
    let hash = hex_of(&parsed_spec.sha256);

    let tasks_body = format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: {hash}\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks\n"
    );
    let tasks_md_path = root.join("TASKS.md");
    fs_err::write(tasks_md_path.as_std_path(), &tasks_body)?;
    let parsed_tasks = tasks_md(&tasks_md_path)?;

    // Synthesize mtimes: tasks older than spec by 5 seconds.
    let now = SystemTime::now();
    let spec_mtime = Some(now);
    let tasks_mtime = Some(now - Duration::from_secs(5));

    let result = stale_for(&parsed_spec, Some(&parsed_tasks), spec_mtime, tasks_mtime);
    assert!(result.stale);
    assert!(result.reasons.contains(&StaleReason::MtimeDrift));
    // Hash matches, so HashDrift should NOT be in reasons.
    assert!(!result.reasons.contains(&StaleReason::HashDrift));
    Ok(())
}

#[test]
fn both_drifts_present_in_declared_order() -> TestResult {
    let tasks_body = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: 0000000000000000000000000000000000000000000000000000000000000000
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks
    "};
    let fx = write_fixture(SPEC_MD, tasks_body)?;
    let (spec, tasks) = parse_pair(&fx)?;

    let now = SystemTime::now();
    let spec_mtime = Some(now);
    let tasks_mtime = Some(now - Duration::from_secs(10));

    let result = stale_for(&spec, Some(&tasks), spec_mtime, tasks_mtime);
    assert!(result.stale);
    assert_eq!(
        result.reasons,
        vec![StaleReason::HashDrift, StaleReason::MtimeDrift],
    );
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

        # Tasks
    "};
    let fx = write_fixture(SPEC_MD, tasks_body)?;
    let (spec, tasks) = parse_pair(&fx)?;

    // Even with mtime drift, the result must be BootstrapPending only.
    let now = SystemTime::now();
    let spec_mtime = Some(now);
    let tasks_mtime = Some(now - Duration::from_mins(1));

    let result = stale_for(&spec, Some(&tasks), spec_mtime, tasks_mtime);
    assert!(result.stale);
    assert_eq!(result.reasons, vec![StaleReason::BootstrapPending]);
    Ok(())
}
