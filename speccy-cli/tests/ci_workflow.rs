//! SPEC-0016 T-012 / REQ-005 / CHK-008-companion: CI workflow content guard.
//!
//! The CI workflow at `.github/workflows/ci.yml` runs `speccy init --force`
//! twice (once per host), then runs `git diff --exit-code` against the four
//! committed-output roots so any uncommitted drift between
//! `resources/` and the dogfooded outputs fails the build with a message
//! pointing the contributor at the two refresh commands.
//!
//! Runtime byte-equivalence between the renderer and the committed
//! dogfood tree is exercised by
//! `tests/init.rs::dogfood_outputs_match_committed_tree` (CHK-008). These tests
//! guard the workflow content itself so a contributor cannot regress the step
//! name, the order of the init commands, the diff target list, or the failure
//! message in isolation from the runtime check.

const CI_WORKFLOW: &str = include_str!("../../.github/workflows/ci.yml");

const CLAUDE_INIT: &str = "speccy init --force --host claude-code";
const CODEX_INIT: &str = "speccy init --force --host codex";
const DIFF_COMMAND: &str = "git diff --exit-code .claude .codex .agents .speccy/skills";

#[test]
fn materialization_step_runs_both_hosts_then_diffs_committed_roots() {
    let body = CI_WORKFLOW;

    let claude_idx = body
        .find(CLAUDE_INIT)
        .expect("ci.yml must invoke `speccy init --force --host claude-code`");
    let codex_idx = body
        .find(CODEX_INIT)
        .expect("ci.yml must invoke `speccy init --force --host codex`");
    let diff_msg = format!("ci.yml must run `{DIFF_COMMAND}` after the two init commands");
    let diff_idx = body.find(DIFF_COMMAND).expect(&diff_msg);

    assert!(
        claude_idx < codex_idx,
        "ci.yml must run `{CLAUDE_INIT}` before `{CODEX_INIT}`",
    );
    assert!(
        codex_idx < diff_idx,
        "ci.yml must run `{CODEX_INIT}` before `{DIFF_COMMAND}`",
    );
}

#[test]
fn materialization_step_does_not_reference_stale_claude_commands_path() {
    let body = CI_WORKFLOW;
    assert!(
        !body.contains(".claude/commands"),
        "ci.yml must not reference the stale `.claude/commands` path (SPEC-0015 moved the install destination to `.claude/skills`)",
    );
}

// --------------------------------------------------------------------
// SPEC-0018 T-006 / REQ-003 / REQ-005: project test commands run
// directly in CI; `speccy verify` is a shape gate that runs alongside
// them, not a wrapper around them. The CI workflow must invoke
// `cargo test`, `cargo clippy`, `cargo +nightly fmt`, and
// `cargo deny check` as their own steps before `speccy verify` so a
// failed project test does not get hidden behind a green `speccy
// verify` and vice versa.
// --------------------------------------------------------------------

const CARGO_TEST_STEP: &str = "cargo test --workspace";
const CARGO_CLIPPY_STEP: &str = "cargo clippy --workspace";
const CARGO_FMT_STEP: &str = "cargo +nightly fmt --all --check";
const CARGO_DENY_STEP: &str = "cargo deny check";
const SPECCY_VERIFY_STEP: &str = "./target/debug/speccy verify";

#[test]
fn project_test_commands_run_directly_before_speccy_verify() {
    let body = CI_WORKFLOW;

    let test_msg = format!("ci.yml must invoke `{CARGO_TEST_STEP}` directly");
    let clippy_msg = format!("ci.yml must invoke `{CARGO_CLIPPY_STEP}` directly");
    let fmt_msg = format!("ci.yml must invoke `{CARGO_FMT_STEP}` directly");
    let deny_msg = format!("ci.yml must invoke `{CARGO_DENY_STEP}` directly");
    let verify_msg = format!("ci.yml must invoke `{SPECCY_VERIFY_STEP}`");
    let test_idx = body.find(CARGO_TEST_STEP).expect(&test_msg);
    let clippy_idx = body.find(CARGO_CLIPPY_STEP).expect(&clippy_msg);
    let fmt_idx = body.find(CARGO_FMT_STEP).expect(&fmt_msg);
    let deny_idx = body.find(CARGO_DENY_STEP).expect(&deny_msg);
    let verify_idx = body.find(SPECCY_VERIFY_STEP).expect(&verify_msg);

    // Each project test command must precede `speccy verify` so CI
    // does not depend on `speccy verify` to execute project tests.
    // SPEC-0018 made `speccy verify` a shape-only validator; project
    // test failures must surface as their own failed CI step.
    for (label, idx) in [
        (CARGO_TEST_STEP, test_idx),
        (CARGO_CLIPPY_STEP, clippy_idx),
        (CARGO_FMT_STEP, fmt_idx),
        (CARGO_DENY_STEP, deny_idx),
    ] {
        assert!(
            idx < verify_idx,
            "ci.yml must run `{label}` directly before `{SPECCY_VERIFY_STEP}`; \
             `speccy verify` is a shape-only validator (SPEC-0018 REQ-003), \
             not a wrapper around project tests",
        );
    }
}

#[test]
fn speccy_verify_step_does_not_run_project_tests() {
    // SPEC-0018 REQ-003: `speccy verify` is shape-only. It must not
    // shell out to `cargo test` / `cargo clippy` / etc. and CI must
    // not assume it does. The verify step in ci.yml is a single
    // `./target/debug/speccy verify` invocation with no other commands
    // chained in. Guarding the literal step body (the line that
    // immediately follows the `name: speccy verify` line) keeps a
    // future contributor from quietly conflating the two responsibilities.
    let body = CI_WORKFLOW;
    let mut found = false;
    let lines: Vec<&str> = body.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == "- name: speccy verify" {
            let run_line = lines.get(idx.saturating_add(1)).expect(
                "the `speccy verify` step must have a `run:` line following its `name:` line",
            );
            let trimmed = run_line.trim();
            assert_eq!(
                trimmed, "run: ./target/debug/speccy verify",
                "the `speccy verify` step must invoke only `./target/debug/speccy verify` \
                 (SPEC-0018 REQ-003 makes verify shape-only); got: `{trimmed}`",
            );
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ci.yml must contain a step named `speccy verify` (SPEC-0018 REQ-003 shape gate)",
    );
}

#[test]
fn materialization_step_failure_message_names_both_refresh_commands() {
    let body = CI_WORKFLOW;

    let error_line = body
        .lines()
        .find(|line| line.contains("::error::"))
        .expect("ci.yml must include a `::error::` annotation on the materialization step");

    assert!(
        error_line.contains(CLAUDE_INIT),
        "ci.yml failure message must point contributors at `{CLAUDE_INIT}`; got: {error_line}",
    );
    assert!(
        error_line.contains(CODEX_INIT),
        "ci.yml failure message must point contributors at `{CODEX_INIT}`; got: {error_line}",
    );
}
