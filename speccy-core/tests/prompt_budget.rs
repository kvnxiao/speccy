#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::prompt::trim_to_budget`.
//! Covers SPEC-0005 REQ-006 via the public API.

use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::trim_to_budget;

#[test]
fn small_content_passes_through_unchanged() {
    let body = "tiny prompt".to_owned();
    let TrimResult {
        output,
        dropped,
        fits,
    } = trim_to_budget(body.clone(), DEFAULT_BUDGET);
    assert_eq!(output, body);
    assert!(dropped.is_empty());
    assert!(fits);
}

#[test]
fn dropping_notes_brings_content_within_budget() {
    let notes_body = "x".repeat(5_000);
    let body = String::new()
        + "# Title\n\n## Goals\nbody\n\n"
        + "## Notes\n"
        + &notes_body
        + "\n\n## After\nmore\n";
    let budget = body.len().saturating_sub(100);
    let TrimResult {
        output,
        dropped,
        fits,
    } = trim_to_budget(body, budget);
    assert!(fits, "dropping Notes should fit budget");
    assert!(
        !output.contains("## Notes"),
        "## Notes heading should be removed",
    );
    assert_eq!(dropped, vec!["## Notes".to_owned()]);
}

#[test]
fn budget_overrun_returns_fits_false() {
    let body = "x".repeat(2_000);
    let TrimResult {
        output,
        dropped,
        fits,
    } = trim_to_budget(body.clone(), 100);
    assert!(!fits);
    assert_eq!(output, body);
    assert!(dropped.is_empty());
}

#[test]
fn default_budget_constant_is_80k() {
    assert_eq!(DEFAULT_BUDGET, 80_000);
}
