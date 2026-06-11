#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Tests for SPEC-0059 T-001 and T-002: the shared authoring reference
//! modules at `resources/modules/references/commit-recipe.md` and
//! `resources/modules/references/branch-guard.md`.
//!
//! T-001 — `commit-recipe.md` — checks the three properties the task's
//! scenarios assert over the embedded `RESOURCES` bundle:
//!
//! - [`commit_recipe_states_idempotency_check_exactly_once`]: the module
//!   carries exactly one `git diff --cached --quiet` idempotency check (the
//!   unified stage-then-skip-if-empty form), satisfying CHK-008's "recipe
//!   stated once" property.
//! - [`commit_recipe_delegates_trailer_via_include`]: the module pulls in
//!   `identity-sourcing.md` via `{% include %}` and does not restate the
//!   trailer-resolution rule inline (CHK-008 delegation property).
//! - [`commit_recipe_specifies_no_git_short_circuit`]: the module specifies a
//!   no-git-repository short-circuit that skips the commit step without
//!   erroring (CHK-013 commit-recipe side).
//!
//! T-002 — `branch-guard.md` — checks the three properties the task's
//! scenarios assert over the same bundle:
//!
//! - [`branch_guard_names_three_detection_tiers_in_order`]: the default-branch
//!   detection prose names `origin/HEAD`, then `init.defaultBranch`, then a
//!   `{main, master}` name match, in that order, each gated on the prior tier
//!   not resolving (CHK-003).
//! - [`branch_guard_states_creation_condition_and_notice`]: the branch is
//!   derived as `spec-` + spec-dir basename, created only on the
//!   default-or-detached condition, the current branch is reused otherwise, and
//!   the creation notice is emitted only on the create path (CHK-002, CHK-001
//!   derivation property).
//! - [`branch_guard_specifies_no_git_short_circuit`]: the module specifies a
//!   no-git-repository short-circuit that skips the branch-guard without
//!   erroring (CHK-013 branch-guard side).

use speccy_cli::embedded::RESOURCES;

/// Read the commit-recipe module from the embedded RESOURCES bundle,
/// panicking with a clear message if it is missing.
fn commit_recipe_body() -> &'static str {
    RESOURCES
        .get_file("modules/references/commit-recipe.md")
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| {
            panic_with_message(
                "RESOURCES bundle must contain `modules/references/commit-recipe.md`; \
                 SPEC-0059 T-001 requires this shared module to be created",
            )
        })
}

/// Test-only failure path. Centralised so the `clippy::panic` expectation
/// is scoped to one function instead of every call site.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn panic_with_message(msg: &str) -> ! {
    panic!("{msg}");
}

/// The recipe states the idempotency check exactly once, in the unified
/// `git diff --cached --quiet` form (CHK-008 "recipe stated once", DEC-004).
#[test]
fn commit_recipe_states_idempotency_check_exactly_once() {
    let body = commit_recipe_body();

    let occurrences = body.matches("git diff --cached --quiet").count();
    assert_eq!(
        occurrences, 1,
        "commit-recipe.md must state the `git diff --cached --quiet` idempotency check \
         exactly once (CHK-008 'recipe stated once'); found {occurrences}",
    );
}

/// The recipe delegates the `Co-Authored-By` trailer to the identity-sourcing
/// rule via `{% include %}` and does not restate the trailer-resolution prose
/// inline (CHK-008 delegation property).
#[test]
fn commit_recipe_delegates_trailer_via_include() {
    let body = commit_recipe_body();

    let expected_include = r#"{% include "modules/references/identity-sourcing.md" %}"#;
    assert!(
        body.contains(expected_include),
        "commit-recipe.md must delegate the trailer to identity-sourcing via \
         `{expected_include}` (CHK-008 delegation property)",
    );

    // The canonical identity-sourcing rule opens with this section heading.
    // Its presence inline (rather than via the include) would mean the rule
    // was restated, defeating the delegation.
    assert!(
        !body.contains("## Sourcing your recorded identity"),
        "commit-recipe.md must not restate the identity-sourcing rule inline; \
         it must pull it in via `{expected_include}` (CHK-008 delegation property)",
    );
}

/// The recipe specifies a no-git-repository short-circuit that skips the
/// commit step without erroring (CHK-013, REQ-010).
#[test]
fn commit_recipe_specifies_no_git_short_circuit() {
    let body = commit_recipe_body();

    assert!(
        body.contains("git rev-parse --is-inside-work-tree"),
        "commit-recipe.md must probe for a git repository via \
         `git rev-parse --is-inside-work-tree` to drive the no-git short-circuit (CHK-013)",
    );

    let lower = body.to_lowercase();
    assert!(
        lower.contains("not a git repository") && lower.contains("without erroring"),
        "commit-recipe.md must state that when the project is not a git repository the \
         commit step is skipped without erroring (CHK-013, REQ-010)",
    );
}

/// Read the branch-guard module from the embedded RESOURCES bundle,
/// panicking with a clear message if it is missing.
fn branch_guard_body() -> &'static str {
    RESOURCES
        .get_file("modules/references/branch-guard.md")
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| {
            panic_with_message(
                "RESOURCES bundle must contain `modules/references/branch-guard.md`; \
                 SPEC-0059 T-002 requires this shared module to be created",
            )
        })
}

/// The default-branch detection prose names the three tiers — `origin/HEAD`,
/// then `init.defaultBranch`, then a `{main, master}` name match — in that
/// order, each gated on the prior tier not resolving (CHK-003).
#[test]
fn branch_guard_names_three_detection_tiers_in_order() {
    let body = branch_guard_body();

    let tier1 = body
        .find("origin/HEAD")
        .expect("branch-guard.md must name the `origin/HEAD` detection tier (CHK-003)");
    let tier2 = body
        .find("init.defaultBranch")
        .expect("branch-guard.md must name the `init.defaultBranch` detection tier (CHK-003)");
    // Tier 3 is the `{main, master}` name-match fallback. Anchor on the
    // distinctive `` `main` or `master` `` phrasing rather than bare `main`,
    // which would also match the illustrative `origin/main` in tier 1.
    let tier3 = body
        .find("`main` or `master`")
        .expect("branch-guard.md must name the `{main, master}` name-match tier (CHK-003)");

    assert!(
        tier1 < tier2,
        "branch-guard.md must place the `origin/HEAD` tier before the \
         `init.defaultBranch` tier (CHK-003 ordered chain)",
    );
    assert!(
        tier2 < tier3,
        "branch-guard.md must place the `{{main, master}}` name-match tier last, \
         after `init.defaultBranch` (CHK-003 ordered chain)",
    );

    // The chain is ordered "each tier used only when the prior does not
    // resolve" — assert the distinctive gating phrase is present.
    assert!(
        body.contains("does not resolve"),
        "branch-guard.md must gate each detection tier on the prior one not \
         resolving (CHK-003 'each tier used only when the prior does not resolve')",
    );
}

/// The branch is derived as `spec-` + spec-dir basename, created only on the
/// default-or-detached condition, the current branch reused otherwise, and the
/// creation notice emitted only on the create path (CHK-002, CHK-001).
#[test]
fn branch_guard_states_creation_condition_and_notice() {
    let body = branch_guard_body();
    let lower = body.to_lowercase();

    // Derivation property (CHK-001): `spec-` prefix + spec-dir basename, and
    // the `git switch -c` that creates+switches.
    assert!(
        body.contains("spec-") && lower.contains("basename"),
        "branch-guard.md must derive the branch name as the `spec-` prefix plus the \
         spec directory basename (CHK-001 derivation property)",
    );
    assert!(
        body.contains("git switch -c"),
        "branch-guard.md must use `git switch -c` to create and switch to the branch \
         (CHK-002 create path)",
    );

    // Creation condition (CHK-002): create only on default-branch-or-detached.
    assert!(
        lower.contains("detached"),
        "branch-guard.md must create the branch on the default-branch-or-detached-HEAD \
         condition (CHK-002)",
    );
    assert!(
        lower.contains("reuse") || lower.contains("reuses"),
        "branch-guard.md must reuse the current branch when HEAD is on any other branch \
         (CHK-002 reuse path)",
    );

    // Notice (CHK-002): emitted only on the create path.
    assert!(
        lower.contains("notice"),
        "branch-guard.md must describe the one-line creation notice (CHK-002)",
    );
    assert!(
        lower.contains("only on the create path") || lower.contains("not on the reuse path"),
        "branch-guard.md must emit the creation notice only on the create path, never on \
         the reuse path (CHK-002)",
    );
}

/// The module specifies a no-git-repository short-circuit that skips the
/// branch-guard without erroring (CHK-013, REQ-010, branch-guard side).
#[test]
fn branch_guard_specifies_no_git_short_circuit() {
    let body = branch_guard_body();

    assert!(
        body.contains("git rev-parse --is-inside-work-tree"),
        "branch-guard.md must probe for a git repository via \
         `git rev-parse --is-inside-work-tree` to drive the no-git short-circuit (CHK-013)",
    );

    let lower = body.to_lowercase();
    assert!(
        lower.contains("not a git repository") && lower.contains("without erroring"),
        "branch-guard.md must state that when the project is not a git repository the \
         branch-guard is skipped without erroring (CHK-013, REQ-010)",
    );
}
