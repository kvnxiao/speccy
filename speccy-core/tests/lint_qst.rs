#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! QST-001 lint diagnostic.

mod lint_common;

use indoc::indoc;
use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::valid_spec_toml;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;
use speccy_core::lint::types::Level;

fn count_code(diags: &[Diagnostic], code: &str) -> usize {
    diags.iter().filter(|d| d.code == code).count()
}

#[test]
fn qst_001_fires_for_each_unchecked_question() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        ### REQ-001: First

        ## Open questions

        - [ ] First question?
        - [x] Already answered.
        - [ ] Second question?
        - [ ] Third question?
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "QST-001"), 3);
    let qst_diags: Vec<_> = diags.iter().filter(|d| d.code == "QST-001").collect();
    for d in &qst_diags {
        assert_eq!(d.level, Level::Info);
    }
    assert!(
        qst_diags
            .iter()
            .any(|d| d.message.contains("Second question?"))
    );
    Ok(())
}

#[test]
fn qst_001_silent_when_all_checked() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        ### REQ-001: First

        ## Open questions

        - [x] all answered
        - [x] yes
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "QST-001"), 0);
    Ok(())
}

#[test]
fn qst_001_heading_match_is_case_insensitive() -> TestResult {
    let spec_md = indoc! {r"
        ---
        id: SPEC-0001
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        ### REQ-001: First

        ## OPEN QUESTIONS

        - [ ] What about case?
    "};
    let fx = write_spec_fixture(spec_md, &valid_spec_toml(), None)?;
    let diags = lint_fixture(&fx);
    assert_eq!(count_code(&diags, "QST-001"), 1);
    Ok(())
}
