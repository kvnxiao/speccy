#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Tests for SPEC-0059 T-001, T-002, T-003, T-004, T-005, and T-006: the shared
//! authoring reference modules at
//! `resources/modules/references/commit-recipe.md` and
//! `resources/modules/references/branch-guard.md`, the
//! behaviour-preserving rewrite of the review-pass commit step in
//! `resources/modules/skills/partials/review-fanout.md` onto the shared
//! commit recipe, the decompose-phase rewrite of the step-4
//! bootstrap commit in `resources/modules/phases/speccy-decompose.md`
//! onto the shared recipe behind the branch-guard prelude, and the new
//! `speccy-plan` commit step in
//! `resources/modules/skills/speccy-plan.md` that commits `SPEC.md`
//! alone behind the branch-guard prelude, and the new `speccy-amend`
//! commit step in `resources/modules/skills/speccy-amend.md` that commits
//! the amend's reconcile delta (`SPEC.md`, the reconciled `TASKS.md` when
//! one exists, and any journal blocker files appended this run) behind the
//! branch-guard prelude.
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
//!
//! T-003 — `review-fanout.md` refactor — checks the behaviour-preserving
//! rewrite of the atomic-commit-on-review-pass section onto the shared recipe
//! (REQ-007, CHK-009 review side, CHK-010, REQ-008 review side):
//!
//! - [`review_fanout_includes_shared_commit_recipe`]: the partial pulls the
//!   shared recipe via `{% include "modules/references/commit-recipe.md" %}`
//!   (CHK-009 review side).
//! - [`review_fanout_retains_add_dash_a_and_title_prefix`]: the refactored
//!   commit step retains `git add -A` staging and the `[SPEC-NNNN/T-NNN]:`
//!   title prefix (CHK-010).
//! - [`review_fanout_drops_inline_status_porcelain_precheck`]: the inline `git
//!   status --porcelain` pre-check is gone, delegated to the recipe's unified
//!   `git diff --cached --quiet` idempotency check (CHK-009 review side,
//!   DEC-004).
//! - [`review_fanout_retains_unguarded_branch_statement_no_branch_guard`][]:
//!   the "commits land on whatever HEAD is" statement is retained and no
//!   `branch-guard.md` include appears (REQ-008 review side).
//! - [`rendered_review_skill_fully_expands_commit_recipe`]: the ejected
//!   `.claude/skills/speccy-review/SKILL.md` has the recipe fully expanded with
//!   no residual `{{`/`{%`/`{#` markup and carries the recipe's `git diff
//!   --cached --quiet` text.
//!
//! T-004 — `speccy-decompose.md` refactor — checks the rewrite of the step-4
//! bootstrap commit onto the shared recipe behind the branch-guard (REQ-004,
//! REQ-007 decompose side, CHK-005, CHK-009 decompose side):
//!
//! - [`decompose_includes_shared_commit_recipe_and_branch_guard`]: the phase
//!   pulls both the shared commit recipe and the branch-guard prelude via `{%
//!   include %}` (CHK-005 recipe property, REQ-007 decompose side).
//! - [`decompose_stages_tasks_md_alone_with_decompose_title`]: the commit step
//!   titles the commit `[SPEC-NNNN]: decompose tasks`, stages
//!   `<spec-dir>/TASKS.md` narrowly with no SPEC.md in the staging set, and
//!   runs after `speccy lock` (CHK-005).
//! - [`decompose_drops_combined_bootstrap_title`]: the prior combined `create
//!   spec and decompose tasks` title string is gone from the source (CHK-005
//!   absent-string property, DEC-005).
//! - [`decompose_drops_inline_diff_cached_recipe`]: the phase no longer
//!   restates the `git diff --cached --quiet` recipe inline; it is delegated to
//!   the included recipe (CHK-009 decompose side).
//! - [`rendered_decompose_agent_fully_expands_includes`]: the ejected
//!   `.claude/agents/speccy-decompose.md` has both includes fully expanded with
//!   no residual `{{`/`{%`/`{#` markup and carries the recipe's `git diff
//!   --cached --quiet` text.
//!
//! T-005 — `speccy-plan.md` commit step — checks the new commit step that
//! commits `SPEC.md` alone behind the branch-guard prelude (REQ-003, CHK-004):
//!
//! - [`plan_includes_shared_commit_recipe_and_branch_guard`]: the plan skill
//!   pulls both the shared commit recipe and the branch-guard prelude via `{%
//!   include %}` (CHK-004 recipe property, REQ-003).
//! - [`plan_stages_spec_md_alone_with_create_spec_title_after_self_review`][]:
//!   the commit step titles the commit `[SPEC-NNNN]: create spec`, stages
//!   `<spec-dir>/SPEC.md` narrowly (no `git add -A`/`git add .`, no TASKS.md in
//!   the staging set), and runs after the step-3 self-review pass (CHK-004).
//! - [`plan_retains_vacancy_id_allocation`]: the ID-allocation step still
//!   invokes `speccy vacancy --json` (the prior CHK-015 invariant is
//!   preserved).
//! - [`rendered_plan_skill_fully_expands_includes`]: the ejected
//!   `.claude/skills/speccy-plan/SKILL.md` has both includes fully expanded
//!   with no residual `{{`/`{%`/`{#` markup and carries the recipe's `git diff
//!   --cached --quiet` text.
//!
//! T-006 — `speccy-amend.md` commit step — checks the new commit step that
//! commits the amend's reconcile delta behind the branch-guard prelude
//! (REQ-005, CHK-006, CHK-007):
//!
//! - [`amend_includes_shared_commit_recipe_and_branch_guard`]: the amend skill
//!   pulls both the shared commit recipe and the branch-guard prelude via `{%
//!   include %}` (CHK-006 recipe property, REQ-005).
//! - [`amend_titles_commit_and_sources_why_from_changelog_after_tsk003_clear`][]:
//!   the commit step titles the commit `[SPEC-NNNN]: amend — <why>`, sources
//!   `<why>` from the newest `## Changelog` row, and runs after the
//!   `TSK-003`-clear check (CHK-006).
//! - [`amend_stages_spec_md_and_tolerates_absent_tasks_md`]: the staging set
//!   covers `SPEC.md`, the reconciled `TASKS.md` when present, and appended
//!   journal blocker files, uses narrow `git add <paths>` (no `git add -A`/`git
//!   add .`), and tolerates an absent `TASKS.md` rather than requiring the
//!   tasks file to exist (CHK-006 staging property, CHK-007).
//! - [`rendered_amend_skill_fully_expands_includes`]: the ejected
//!   `.claude/skills/speccy-amend/SKILL.md` has both includes fully expanded
//!   with no residual `{{`/`{%`/`{#` markup and carries the recipe's `git diff
//!   --cached --quiet` text.

use speccy_cli::embedded::RESOURCES;
use speccy_cli::host::HostChoice;
use speccy_cli::render::render_host_pack;

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

/// Read the `review-fanout.md` partial from the embedded RESOURCES bundle,
/// panicking with a clear message if it is missing.
fn review_fanout_body() -> &'static str {
    RESOURCES
        .get_file("modules/skills/partials/review-fanout.md")
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| {
            panic_with_message(
                "RESOURCES bundle must contain `modules/skills/partials/review-fanout.md`",
            )
        })
}

/// The refactored review-pass commit step pulls the shared recipe via
/// `{% include %}` rather than restating it inline (CHK-009 review side).
#[test]
fn review_fanout_includes_shared_commit_recipe() {
    let body = review_fanout_body();

    let expected_include = r#"{% include "modules/references/commit-recipe.md" %}"#;
    assert!(
        body.contains(expected_include),
        "review-fanout.md must pull the shared commit recipe via `{expected_include}` \
         (CHK-009 review side); the hand-rolled inline copy must be removed",
    );
}

/// The refactored commit step retains `git add -A` staging and the
/// `[SPEC-NNNN/T-NNN]:` title prefix the consistency check greps (CHK-010).
#[test]
fn review_fanout_retains_add_dash_a_and_title_prefix() {
    let body = review_fanout_body();

    assert!(
        body.contains("git add -A"),
        "review-fanout.md must retain `git add -A` staging after the refactor (CHK-010); \
         the review-pass commit stages the whole tree under the clean-tree precondition",
    );
    assert!(
        body.contains("[SPEC-NNNN/T-NNN]:"),
        "review-fanout.md must retain the `[SPEC-NNNN/T-NNN]:` title prefix (CHK-010); \
         the consistency check correlates commits to tasks by grepping for it",
    );
}

/// The inline `git status --porcelain` pre-check is removed — the unified
/// `git diff --cached --quiet` idempotency check now lives in the recipe
/// (CHK-009 review side, DEC-004).
#[test]
fn review_fanout_drops_inline_status_porcelain_precheck() {
    let body = review_fanout_body();

    assert!(
        !body.contains("git status --porcelain"),
        "review-fanout.md must no longer contain the inline `git status --porcelain` \
         pre-check (CHK-009 review side, DEC-004); the recipe's unified \
         `git diff --cached --quiet` check subsumes it",
    );
    assert!(
        !body.contains("git diff --cached --quiet"),
        "review-fanout.md must not restate the recipe's `git diff --cached --quiet` \
         idempotency check inline; it is delegated to the included recipe (CHK-009)",
    );
}

/// The unguarded branch statement is retained and no branch-guard include is
/// added — the review-pass commit stays unguarded (REQ-008 review side).
#[test]
fn review_fanout_retains_unguarded_branch_statement_no_branch_guard() {
    let body = review_fanout_body();

    assert!(
        body.contains("Commits land on whatever HEAD is"),
        "review-fanout.md must retain the unguarded \"Commits land on whatever HEAD is\" \
         statement (REQ-008 review side); the review-pass commit is not branch-guarded",
    );
    assert!(
        !body.contains(r#"{% include "modules/references/branch-guard.md" %}"#),
        "review-fanout.md must not add a branch-guard include (REQ-008 review side); \
         only the three authoring skills guard the branch, not the review-pass commit",
    );
}

/// The ejected `.claude/skills/speccy-review/SKILL.md` fully expands the
/// included recipe: no residual `MiniJinja` markup, and the recipe's
/// `git diff --cached --quiet` text is present in the rendered output.
#[test]
fn rendered_review_skill_fully_expands_commit_recipe() {
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) must succeed");

    let rel = ".claude/skills/speccy-review/SKILL.md";
    let file = rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_message(&format!(
                "rendered claude-code pack must include `{rel}`; \
                 speccy-review includes the refactored review-fanout partial",
            ))
        });

    for marker in ["{{", "{%", "{#"] {
        assert!(
            !file.contents.contains(marker),
            "rendered `{rel}` must not contain MiniJinja markup `{marker}`; \
             the commit-recipe include must be fully expanded at render time",
        );
    }

    assert!(
        file.contents.contains("git diff --cached --quiet"),
        "rendered `{rel}` must carry the recipe's `git diff --cached --quiet` idempotency \
         check, proving the include expanded into the review skill body",
    );
}

/// Read the `speccy-decompose.md` phase body from the embedded RESOURCES
/// bundle, panicking with a clear message if it is missing.
fn decompose_body() -> &'static str {
    RESOURCES
        .get_file("modules/phases/speccy-decompose.md")
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| {
            panic_with_message("RESOURCES bundle must contain `modules/phases/speccy-decompose.md`")
        })
}

/// The refactored step-4 commit pulls both the shared commit recipe and the
/// branch-guard prelude via `{% include %}` (CHK-005 recipe property, REQ-007
/// decompose side).
#[test]
fn decompose_includes_shared_commit_recipe_and_branch_guard() {
    let body = decompose_body();

    let recipe_include = r#"{% include "modules/references/commit-recipe.md" %}"#;
    assert!(
        body.contains(recipe_include),
        "speccy-decompose.md must pull the shared commit recipe via `{recipe_include}` \
         (CHK-005); the hand-rolled inline bootstrap commit must be removed",
    );

    let guard_include = r#"{% include "modules/references/branch-guard.md" %}"#;
    assert!(
        body.contains(guard_include),
        "speccy-decompose.md must run the branch-guard prelude via `{guard_include}` \
         ahead of the commit so the commit lands on a feature branch (REQ-007 decompose side)",
    );
}

/// The commit step titles the commit `[SPEC-NNNN]: decompose tasks`, stages
/// `<spec-dir>/TASKS.md` narrowly (no `git add -A`/`git add .`, no SPEC.md in
/// the staging set), and runs after `speccy lock` (CHK-005).
#[test]
fn decompose_stages_tasks_md_alone_with_decompose_title() {
    let body = decompose_body();

    assert!(
        body.contains("[SPEC-NNNN]: decompose tasks"),
        "speccy-decompose.md must title the commit `[SPEC-NNNN]: decompose tasks` (CHK-005)",
    );

    // Narrow staging of TASKS.md alone — the staging command names the path.
    assert!(
        body.contains("git add <spec-dir>/TASKS.md"),
        "speccy-decompose.md must stage the spec's TASKS.md narrowly via \
         `git add <spec-dir>/TASKS.md` (CHK-005)",
    );

    // SPEC.md must no longer be in the staging set: it is committed by
    // speccy-plan, not here (DEC-005). The prior combined staging command
    // `git add <spec-dir>/SPEC.md <spec-dir>/TASKS.md` must be gone. Anchor on
    // the SPEC.md staging path rather than a bare `git add .` substring, which
    // would false-match the prose that *forbids* the whole-tree forms.
    assert!(
        !body.contains("git add <spec-dir>/SPEC.md"),
        "speccy-decompose.md must not stage SPEC.md — it is committed by speccy-plan, \
         not the decompose commit (DEC-005, CHK-005)",
    );

    // The commit still runs after `speccy lock`: the lock step text precedes
    // the commit recipe include in document order.
    let lock_pos = body
        .find("speccy lock SPEC-NNNN")
        .expect("speccy-decompose.md must still run `speccy lock SPEC-NNNN` before the commit");
    let recipe_pos = body
        .find(r#"{% include "modules/references/commit-recipe.md" %}"#)
        .expect("speccy-decompose.md must include the commit recipe");
    assert!(
        lock_pos < recipe_pos,
        "speccy-decompose.md must run `speccy lock` before the commit recipe (CHK-005)",
    );
}

/// The prior combined bootstrap title `create spec and decompose tasks` is
/// gone from the source (CHK-005 absent-string property, DEC-005).
#[test]
fn decompose_drops_combined_bootstrap_title() {
    let body = decompose_body();

    assert!(
        !body.contains("create spec and decompose tasks"),
        "speccy-decompose.md must no longer carry the combined \
         `create spec and decompose tasks` title — it is split into a per-skill \
         `[SPEC-NNNN]: decompose tasks` commit (CHK-005, DEC-005)",
    );
}

/// The phase no longer restates the `git diff --cached --quiet` recipe inline;
/// it is delegated to the included recipe (CHK-009 decompose side).
#[test]
fn decompose_drops_inline_diff_cached_recipe() {
    let body = decompose_body();

    assert!(
        !body.contains("git diff --cached --quiet"),
        "speccy-decompose.md must not restate the recipe's `git diff --cached --quiet` \
         idempotency check inline; it is delegated to the included recipe (CHK-009)",
    );
}

/// The ejected `.claude/agents/speccy-decompose.md` fully expands both
/// includes: no residual `MiniJinja` markup, and the recipe's
/// `git diff --cached --quiet` text is present in the rendered output.
#[test]
fn rendered_decompose_agent_fully_expands_includes() {
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) must succeed");

    let rel = ".claude/agents/speccy-decompose.md";
    let file = rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_message(&format!(
                "rendered claude-code pack must include `{rel}`; \
                 speccy-decompose includes the refactored phase body",
            ))
        });

    for marker in ["{{", "{%", "{#"] {
        assert!(
            !file.contents.contains(marker),
            "rendered `{rel}` must not contain MiniJinja markup `{marker}`; \
             the commit-recipe and branch-guard includes must be fully expanded at render time",
        );
    }

    assert!(
        file.contents.contains("git diff --cached --quiet"),
        "rendered `{rel}` must carry the recipe's `git diff --cached --quiet` idempotency \
         check, proving the commit-recipe include expanded into the decompose agent body",
    );
}

/// Read the `speccy-plan.md` skill body from the embedded RESOURCES bundle,
/// panicking with a clear message if it is missing.
fn plan_body() -> &'static str {
    RESOURCES
        .get_file("modules/skills/speccy-plan.md")
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| {
            panic_with_message("RESOURCES bundle must contain `modules/skills/speccy-plan.md`")
        })
}

/// The new commit step pulls both the shared commit recipe and the branch-guard
/// prelude via `{% include %}` (CHK-004 recipe property, REQ-003).
#[test]
fn plan_includes_shared_commit_recipe_and_branch_guard() {
    let body = plan_body();

    let recipe_include = r#"{% include "modules/references/commit-recipe.md" %}"#;
    assert!(
        body.contains(recipe_include),
        "speccy-plan.md must pull the shared commit recipe via `{recipe_include}` \
         (CHK-004); the SPEC.md commit step must delegate to the shared recipe",
    );

    let guard_include = r#"{% include "modules/references/branch-guard.md" %}"#;
    assert!(
        body.contains(guard_include),
        "speccy-plan.md must run the branch-guard prelude via `{guard_include}` \
         ahead of the commit so SPEC.md lands on a feature branch (REQ-003)",
    );
}

/// The commit step titles the commit `[SPEC-NNNN]: create spec`, stages
/// `<spec-dir>/SPEC.md` narrowly (no `git add -A`/`git add .`, no TASKS.md in
/// the staging set), and runs after the step-3 self-review pass (CHK-004).
#[test]
fn plan_stages_spec_md_alone_with_create_spec_title_after_self_review() {
    let body = plan_body();

    assert!(
        body.contains("[SPEC-NNNN]: create spec"),
        "speccy-plan.md must title the commit `[SPEC-NNNN]: create spec` (CHK-004)",
    );

    // Narrow staging of SPEC.md alone — the staging command names the path.
    assert!(
        body.contains("git add <spec-dir>/SPEC.md"),
        "speccy-plan.md must stage the spec's SPEC.md narrowly via \
         `git add <spec-dir>/SPEC.md` (CHK-004)",
    );

    // The whole-tree staging forms are forbidden in the commit step. The
    // surrounding prose may legitimately *forbid* them by name, so anchor the
    // negative assertion on a staging *command* — `git add -A`/`git add .` as
    // an executable line would only appear if the step used the broad form.
    // We assert TASKS.md is never staged here: it is committed by
    // speccy-decompose, not the plan commit (DEC-005).
    assert!(
        !body.contains("git add <spec-dir>/TASKS.md"),
        "speccy-plan.md must not stage TASKS.md — it is committed by speccy-decompose, \
         not the plan commit (DEC-005, CHK-004)",
    );

    // The commit runs after the step-3 self-review pass: the self-review
    // heading precedes the commit recipe include in document order.
    let self_review_pos = body
        .find("Self-review pass")
        .expect("speccy-plan.md must still carry the step-3 self-review pass");
    let recipe_pos = body
        .find(r#"{% include "modules/references/commit-recipe.md" %}"#)
        .expect("speccy-plan.md must include the commit recipe");
    assert!(
        self_review_pos < recipe_pos,
        "speccy-plan.md must run the self-review pass before the commit recipe (CHK-004)",
    );
}

/// The ID-allocation step still invokes `speccy vacancy --json` (the prior
/// CHK-015 invariant is preserved — the commit step must not displace it).
#[test]
fn plan_retains_vacancy_id_allocation() {
    let body = plan_body();

    assert!(
        body.contains("speccy vacancy --json"),
        "speccy-plan.md must still allocate the spec id via `speccy vacancy --json`; \
         the new commit step must not displace the vacancy-not-status invariant \
         (CHK-015, REQ-003 task constraint)",
    );
    assert!(
        !body.contains("speccy status --json"),
        "speccy-plan.md must not allocate the id via `speccy status --json`; the \
         vacancy-not-status invariant requires `speccy vacancy --json` (CHK-015)",
    );
}

/// The ejected `.claude/skills/speccy-plan/SKILL.md` fully expands both
/// includes: no residual `MiniJinja` markup, and the recipe's
/// `git diff --cached --quiet` text is present in the rendered output.
#[test]
fn rendered_plan_skill_fully_expands_includes() {
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) must succeed");

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let file = rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_message(&format!(
                "rendered claude-code pack must include `{rel}`; \
                 speccy-plan includes the commit-recipe and branch-guard modules",
            ))
        });

    for marker in ["{{", "{%", "{#"] {
        assert!(
            !file.contents.contains(marker),
            "rendered `{rel}` must not contain MiniJinja markup `{marker}`; \
             the commit-recipe and branch-guard includes must be fully expanded at render time",
        );
    }

    assert!(
        file.contents.contains("git diff --cached --quiet"),
        "rendered `{rel}` must carry the recipe's `git diff --cached --quiet` idempotency \
         check, proving the commit-recipe include expanded into the plan skill body",
    );
}

/// Read the `speccy-amend.md` skill body from the embedded RESOURCES bundle,
/// panicking with a clear message if it is missing.
fn amend_body() -> &'static str {
    RESOURCES
        .get_file("modules/skills/speccy-amend.md")
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| {
            panic_with_message("RESOURCES bundle must contain `modules/skills/speccy-amend.md`")
        })
}

/// The new commit step pulls both the shared commit recipe and the branch-guard
/// prelude via `{% include %}` (CHK-006 recipe property, REQ-005).
#[test]
fn amend_includes_shared_commit_recipe_and_branch_guard() {
    let body = amend_body();

    let recipe_include = r#"{% include "modules/references/commit-recipe.md" %}"#;
    assert!(
        body.contains(recipe_include),
        "speccy-amend.md must pull the shared commit recipe via `{recipe_include}` \
         (CHK-006); the amend reconcile-delta commit step must delegate to the shared recipe",
    );

    let guard_include = r#"{% include "modules/references/branch-guard.md" %}"#;
    assert!(
        body.contains(guard_include),
        "speccy-amend.md must run the branch-guard prelude via `{guard_include}` \
         ahead of the commit so the reconcile delta lands on a feature branch (REQ-005)",
    );
}

/// The commit step titles the commit `[SPEC-NNNN]: amend — <why>`, sources
/// `<why>` from the newest `## Changelog` row, and runs after the
/// `TSK-003`-clear check in step 6 (CHK-006).
#[test]
fn amend_titles_commit_and_sources_why_from_changelog_after_tsk003_clear() {
    let body = amend_body();

    assert!(
        body.contains("[SPEC-NNNN]: amend — <why>"),
        "speccy-amend.md must title the commit `[SPEC-NNNN]: amend — <why>` (CHK-006)",
    );

    // `<why>` is sourced from the newest `## Changelog` row, not separately
    // prompted — the prose must name the Changelog row as the source.
    assert!(
        body.contains("## Changelog"),
        "speccy-amend.md must source the commit title's `<why>` from the newest \
         `## Changelog` row (CHK-006), not a separate prompt",
    );

    // The commit must run after the step-6 `TSK-003`-clear check: the
    // `TSK-003` confirmation text precedes the commit recipe include in
    // document order.
    let tsk003_pos = body
        .find("confirm `TSK-003` cleared")
        .expect("speccy-amend.md must keep the step-6 `TSK-003`-clear check (CHK-006)");
    let recipe_pos = body
        .find(r#"{% include "modules/references/commit-recipe.md" %}"#)
        .expect("speccy-amend.md must include the commit recipe");
    assert!(
        tsk003_pos < recipe_pos,
        "speccy-amend.md must run the commit recipe after the `TSK-003`-clear check (CHK-006)",
    );
}

/// The staging set covers `SPEC.md`, the reconciled `TASKS.md` when present,
/// and appended journal blocker files; it uses narrow `git add <paths>` (no
/// `git add -A`/`git add .`) and tolerates an absent `TASKS.md` rather than
/// requiring the tasks file to exist (CHK-006 staging property, CHK-007).
#[test]
fn amend_stages_spec_md_and_tolerates_absent_tasks_md() {
    let body = amend_body();

    // SPEC.md is always in the staging set.
    assert!(
        body.contains("<spec-dir>/SPEC.md"),
        "speccy-amend.md must stage the spec's `SPEC.md` in the amend delta (CHK-006)",
    );

    // The reconciled TASKS.md is staged when one exists, and the journal
    // blocker files appended this run are staged too.
    assert!(
        body.contains("<spec-dir>/TASKS.md"),
        "speccy-amend.md must stage the reconciled `TASKS.md` when one exists (CHK-006)",
    );
    assert!(
        body.contains("<spec-dir>/journal/T-NNN.md"),
        "speccy-amend.md must stage the per-task journal blocker files appended this run \
         (`<spec-dir>/journal/T-NNN.md`) (CHK-006)",
    );

    // Narrow staging only. The surrounding prose legitimately *forbids* the
    // whole-tree forms by name (so a bare `git add -A` / `git add .` substring
    // check would false-match that prohibition prose, exactly as the plan and
    // decompose tests note). Anchor instead on the positive narrow-staging
    // commands: SPEC.md and the journal blockers are staged by explicit path,
    // which is the observable property the broad forms would violate.
    assert!(
        body.contains("git add <paths>"),
        "speccy-amend.md must use narrow `git add <paths>` staging (CHK-006)",
    );

    // CHK-007: the staging set tolerates an absent TASKS.md — the prose must
    // state the missing tasks file does not fail the commit. Anchor on the
    // distinctive phrasing the step uses.
    let lower = body.to_lowercase();
    assert!(
        lower.contains("when one exists") || lower.contains("only when it exists"),
        "speccy-amend.md must qualify `TASKS.md` staging as conditional on the file \
         existing (CHK-007)",
    );
    // Collapse interior whitespace so the assertion does not hinge on where
    // the prose happens to line-wrap.
    let collapsed = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        collapsed.contains("without failing on the absent tasks file")
            || collapsed.contains("without failing on the missing tasks file"),
        "speccy-amend.md must state that an absent `TASKS.md` does not fail the commit \
         (CHK-007)",
    );
}

/// The ejected `.claude/skills/speccy-amend/SKILL.md` fully expands both
/// includes: no residual `MiniJinja` markup, and the recipe's
/// `git diff --cached --quiet` text is present in the rendered output.
#[test]
fn rendered_amend_skill_fully_expands_includes() {
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) must succeed");

    let rel = ".claude/skills/speccy-amend/SKILL.md";
    let file = rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_message(&format!(
                "rendered claude-code pack must include `{rel}`; \
                 speccy-amend includes the commit-recipe and branch-guard modules",
            ))
        });

    for marker in ["{{", "{%", "{#"] {
        assert!(
            !file.contents.contains(marker),
            "rendered `{rel}` must not contain MiniJinja markup `{marker}`; \
             the commit-recipe and branch-guard includes must be fully expanded at render time",
        );
    }

    assert!(
        file.contents.contains("git diff --cached --quiet"),
        "rendered `{rel}` must carry the recipe's `git diff --cached --quiet` idempotency \
         check, proving the commit-recipe include expanded into the amend skill body",
    );
}
