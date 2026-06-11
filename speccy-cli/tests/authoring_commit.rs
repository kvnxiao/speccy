#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Tests for SPEC-0059 T-001: the shared commit-recipe reference module
//! at `resources/modules/references/commit-recipe.md`.
//!
//! Checks the three properties the task's scenarios assert over the
//! embedded `RESOURCES` bundle:
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
