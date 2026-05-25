#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! REQ-* lint diagnostics.
//!
//! After SPEC-0019 the requirement-to-scenario graph lives in SPEC.md
//! marker nesting. The marker parser already rejects orphan scenarios,
//! dangling references, AND a `speccy:requirement` block with zero
//! nested `speccy:scenario` markers at parse time, so the REQ-001 lint
//! rule is unreachable in practice (the parser fires SPC-001 first).
//! The single test below documents that contract.

mod lint_common;

use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::valid_spec_md;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;

fn count_code(diags: &[Diagnostic], code: &str) -> usize {
    diags.iter().filter(|d| d.code == code).count()
}

#[test]
fn req_001_silent_when_every_requirement_has_a_scenario() -> TestResult {
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "REQ-001"), 0);
    Ok(())
}
