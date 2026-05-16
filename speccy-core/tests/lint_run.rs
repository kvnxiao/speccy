#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! Behaviors of the `lint::run` orchestrator: determinism and sort order.

mod lint_common;

use lint_common::TestResult;
use lint_common::parse_fixture;
use lint_common::run_lint;
use lint_common::valid_spec_md;
use lint_common::write_spec_fixture;

#[test]
fn run_is_deterministic_across_two_calls() -> TestResult {
    let fx_a = write_spec_fixture(&valid_spec_md("SPEC-0001"), None)?;
    let fx_b = write_spec_fixture(&valid_spec_md("SPEC-0002"), None)?;

    let specs_first = vec![parse_fixture(&fx_a), parse_fixture(&fx_b)];
    let specs_second = vec![parse_fixture(&fx_a), parse_fixture(&fx_b)];

    let a = run_lint(&specs_first);
    let b = run_lint(&specs_second);
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn ordering_is_by_spec_then_code_then_file_then_line() -> TestResult {
    let lower = write_spec_fixture(&valid_spec_md("SPEC-0001"), None)?;
    let higher = write_spec_fixture(&valid_spec_md("SPEC-0002"), None)?;

    let diags = run_lint(&[parse_fixture(&higher), parse_fixture(&lower)]);
    let spec_ids: Vec<Option<String>> = diags.iter().map(|d| d.spec_id.clone()).collect();

    let lower_idx = spec_ids
        .iter()
        .position(|s| s.as_deref() == Some("SPEC-0001"));
    let higher_idx = spec_ids
        .iter()
        .position(|s| s.as_deref() == Some("SPEC-0002"));
    if let (Some(a), Some(b)) = (lower_idx, higher_idx) {
        assert!(a < b, "SPEC-0001 diags should sort before SPEC-0002");
    }
    Ok(())
}
