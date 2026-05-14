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
