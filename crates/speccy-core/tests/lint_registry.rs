#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! Stability snapshot for `speccy_core::lint::REGISTRY`.
//!
//! Removing or renaming a code, or changing a severity, makes this test
//! fail. To intentionally bump the registry: regenerate the snapshot via
//! `cargo test --test lint_registry -- --ignored bless`.

use speccy_core::lint::registry::render_snapshot;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SNAPSHOT_PATH: &str = "tests/snapshots/lint_registry.snap";

#[test]
fn registry_matches_snapshot() -> TestResult {
    let current = render_snapshot();
    let snapshot_path = Path::new(SNAPSHOT_PATH);
    let stored = fs::read_to_string(snapshot_path)?;
    assert_eq!(
        stored, current,
        "lint REGISTRY changed; regenerate the snapshot at {SNAPSHOT_PATH} if intentional",
    );
    Ok(())
}

#[ignore = "blesses the snapshot; run explicitly when REGISTRY intentionally changes"]
#[test]
fn bless_snapshot() -> TestResult {
    let current = render_snapshot();
    fs::write(SNAPSHOT_PATH, current)?;
    Ok(())
}
