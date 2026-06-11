---
spec: SPEC-0059
outcome: implemented
generated_at: 2026-06-11T00:00:00Z
---

# REPORT: SPEC-0059 Branch-guard and consolidated git commits for the authoring skills (plan / decompose / amend)

<report spec="SPEC-0059">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-002 created `resources/modules/references/branch-guard.md` defining
the branch-guard prelude. The prelude creates and switches to
`spec-NNNN-slug` when HEAD is the default branch or detached, reuses the
current branch otherwise, and emits a one-line creation notice only on
the create path. The branch name is derived as `spec-` + spec-dir
basename, dropping any focus segment for mission-foldered specs. T-004,
T-005, and T-006 wire the include into `speccy-decompose.md`,
`speccy-plan.md`, and `speccy-amend.md` respectively. T-007 locked the
scoping invariant via `speccy-cli/tests/authoring_commit.rs`. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
T-002 defined the three-tier detection chain in `branch-guard.md`:
(1) `origin/HEAD` when a remote exists; (2) `git config
init.defaultBranch` when no remote; (3) a `{main, master}` HEAD-name
match as terminal fallback. Each tier is gated on the prior not
resolving. T-007 verified the rendered text survives MiniJinja expansion
with no residual markup. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
T-005 added a commit step to `resources/modules/skills/speccy-plan.md`
after the self-review pass: branch-guard prelude followed by the shared
commit recipe staged to `<spec-dir>/SPEC.md` alone with title
`[SPEC-NNNN]: create spec`. Narrow staging (no `git add -A`) was
verified by T-007. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
T-004 reworked step 4 of `resources/modules/phases/speccy-decompose.md`:
branch-guard prelude inserted, hand-rolled bootstrap commit replaced with
the shared recipe staged to `<spec-dir>/TASKS.md` alone and titled
`[SPEC-NNNN]: decompose tasks`. The string `create spec and decompose
tasks` was removed. T-007 confirmed both callsites include the recipe and
neither restates `git diff --cached --quiet` inline. Retry count: 1.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006 CHK-007">
T-006 added a commit step to `resources/modules/skills/speccy-amend.md`
after the `TSK-003`-clear check: branch-guard prelude followed by the
shared recipe staged to SPEC.md + reconciled TASKS.md-when-present +
appended journal blocker files, titled `[SPEC-NNNN]: amend — <why>` with
`<why>` sourced from the newest `## Changelog` row. Absent-TASKS.md
tolerance is specified in the staging prose. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-008">
T-001 created `resources/modules/references/commit-recipe.md` with
exactly one `git diff --cached --quiet` idempotency check. Staging
breadth and title/body are the sole caller-varying parameters. The
`Co-Authored-By` trailer is delegated via `{% include
"modules/references/identity-sourcing.md" %}` rather than restated. A
no-git short-circuit opens the module. T-007 verified the recipe text
expands cleanly in the rendered pack. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-009 CHK-010">
T-003 rewrote the "Atomic commit on review pass" section in
`resources/modules/skills/partials/review-fanout.md` to include the
shared recipe with `git add -A` staging and the
`[SPEC-NNNN/T-NNN]: <task title>` title. The inline `git status
--porcelain` pre-check and inline trailer were removed. T-004 refactored
`speccy-decompose.md` similarly. T-007 confirmed neither file restates
the recipe inline and the review-pass title format is retained. Retry
count: 2.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-011">
T-007 asserted via `speccy-cli/tests/authoring_commit.rs` that the
branch-guard include appears in exactly `speccy-plan.md`,
`speccy-decompose.md`, and `speccy-amend.md`, and is absent from
`speccy-work.md`, `speccy-ship.md`, and `review-fanout.md`. The
review-pass commit retains its unguarded "lands on whatever HEAD is"
statement. Retry count: 1.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-012">
T-007 asserted that none of the three authoring-skill bodies contains
`git add -A` or `git add .` in its commit step, and none contains a
clean-tree refusal gate token (`git stash` or a refuse-on-dirty marker).
All three authoring commits stage narrow spec-artifact path lists. Retry
count: 1.
</coverage>

<coverage req="REQ-010" result="satisfied" scenarios="CHK-013">
T-001 opened `commit-recipe.md` with a no-git short-circuit (skip commit
without erroring when not in a git repository). T-002 opened
`branch-guard.md` with the same guard. T-007 confirmed both modules
specify the short-circuit. Retry count: 0.
</coverage>

</report>
