---
spec: SPEC-0059
spec_hash_at_generation: 5cb028ea831daddebdb72f629e5b99c834399244fe112e1ec9c20ea573e55ab0
generated_at: 2026-06-11T06:41:43Z
---
# Tasks: SPEC-0059 Branch-guard and consolidated git commits for the authoring skills (plan / decompose / amend)

<task id="T-001" state="completed" covers="REQ-006 REQ-010">
## Create the shared commit-recipe reference module

Create `resources/modules/references/commit-recipe.md` defining the
stage → `git diff --cached --quiet` → skip-if-empty → commit recipe
exactly once (REQ-006, DEC-002, DEC-004). Structure the prose so:

- It opens with a **no-git short-circuit** (REQ-010, CHK-013): when the
  working directory is not a git repository, the commit step is skipped
  without erroring and the just-written file is left in place.
- The caller supplies the **staging breadth** (`git add -A` versus a
  narrow `git add <paths>` list) and the **title/body** — those are the
  only behaviour-varying parameters. The idempotency check is the single
  unified `git diff --cached --quiet` form after staging (not a
  per-caller `git status --porcelain` variant), per DEC-004.
- The `Co-Authored-By` trailer is **delegated** to the identity-sourcing
  rule via `{% include "modules/references/identity-sourcing.md" %}` —
  do not restate the trailer-resolution prose inline.
- The commit message is passed via a HEREDOC so newlines and special
  characters survive.

No callsite is wired to this module in this task — the four consumers
land in T-003..T-006. This module ships in other people's repos via
`speccy init`, so every illustrative value must be a fictional
placeholder (`SPEC-NNNN`, `<spec-dir>`), never a real Speccy spec id.

Edit only under `resources/`; this module is included (not ejected as a
standalone reference file), so no `just reeject` artifact changes are
expected from creating it alone, but run `just reeject` to confirm the
render is still clean.

<task-scenarios>
Given the embedded `RESOURCES` bundle after this task,
when `modules/references/commit-recipe.md` is read,
then the file exists and contains exactly one
`git diff --cached --quiet` idempotency check (the unified
stage-then-skip-if-empty form), satisfying CHK-008's "recipe stated
once" property.

Given the same module body,
when its trailer handling is inspected,
then it contains `{% include "modules/references/identity-sourcing.md" %}`
and does not restate the trailer-resolution rule inline (CHK-008
delegation property).

Given the same module body,
when its preconditions are read,
then it specifies a no-git-repository short-circuit that skips the
commit step without erroring (CHK-013 commit-recipe side).

Suggested files: `resources/modules/references/commit-recipe.md`,
`resources/modules/references/identity-sourcing.md` (delegated-to,
unchanged), `speccy-cli/tests/authoring_commit.rs`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-002 REQ-010">
## Create the shared branch-guard prelude module

Create `resources/modules/references/branch-guard.md` defining the
branch-guard prelude (REQ-001) and its default-branch detection chain
(REQ-002). Structure the prose so:

- It opens with a **no-git short-circuit** (REQ-010, CHK-013): when the
  working directory is not a git repository, the branch-guard is skipped
  without erroring.
- **Default-branch detection** is an ordered three-tier chain, each tier
  used only when the prior does not resolve (REQ-002, CHK-003): (1) the
  remote symbolic ref `origin/HEAD` when a remote exists; (2) otherwise
  `git config init.defaultBranch`; (3) otherwise a `{main, master}`
  HEAD-name match.
- **Branch creation condition** (REQ-001, CHK-002): when HEAD is the
  default branch or a detached HEAD, derive the branch name as the
  literal `spec-` prefix followed by the spec directory basename
  (`NNNN-slug`) — dropping the focus segment for mission-foldered
  (`[focus]/NNNN-slug`) specs so the name stays flat `spec-NNNN-slug` —
  then `git switch -c` to it. When HEAD is on any other branch, reuse it
  and create nothing.
- A **one-line creation notice** naming the created branch is emitted
  only on the create path, never on the reuse path.

No callsite is wired in this task — the three authoring consumers land
in T-004, T-005, T-006. This module ships downstream via `speccy init`,
so use only fictional placeholders (`spec-NNNN-slug`, `0042-example-slug`,
`acme/widget`) in any example, never a real Speccy branch or spec id.

Edit only under `resources/`; run `just reeject` to confirm the render
stays clean.

<task-scenarios>
Given the embedded `RESOURCES` bundle after this task,
when `modules/references/branch-guard.md` is read,
then its default-branch detection prose names the three tiers in order —
`origin/HEAD`, then `init.defaultBranch`, then a `{main, master}` name
match — each gated on the prior tier not resolving (CHK-003).

Given the same module body,
when its branch-creation condition is read,
then it creates and switches to `spec-` + spec-dir basename only when
HEAD is the default branch or detached, reuses the current branch
otherwise, and emits the creation notice only on the create path
(CHK-001 derivation property, CHK-002).

Given the same module body,
when its preconditions are read,
then it specifies a no-git-repository short-circuit that skips the
branch-guard without erroring (CHK-013 branch-guard side).

Suggested files: `resources/modules/references/branch-guard.md`,
`speccy-cli/tests/authoring_commit.rs`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-007">
## Refactor the review-pass commit onto the shared recipe (behaviour-preserving)

Rewrite the "Atomic commit on review pass" section in
`resources/modules/skills/partials/review-fanout.md` to pull the shared
recipe via `{% include "modules/references/commit-recipe.md" %}`
(REQ-007), parameterised to `git add -A` staging and the
`[SPEC-NNNN/T-NNN]: <task title>` title with the `Completed`-field body.
Remove the hand-rolled inline copy: the `git status --porcelain`
pre-check, the inline commit-message-format block, and the inline
`Co-Authored-By` trailer restatement (which the recipe now delegates to
identity-sourcing).

The refactor is **behaviour-preserving**: the title format
`[SPEC-NNNN/T-NNN]: <task title>` (the prefix the CLI consistency check
greps), the `git add -A` staging, the single-parent commit shape, and
the trailer are all retained. Per DEC-004 the unified
stage-then-`git diff --cached --quiet` check yields the same commit/skip
outcome the prior `git status --porcelain` pre-check produced for the
`-A` callsite (the only difference is one harmless no-op `git add -A` on
an already-clean tree). **Retain** the existing "the skill body does not
check the current git branch … commits land on whatever HEAD is"
statement, and add **no** branch-guard include here (the review-pass
commit stays unguarded — REQ-008).

Edit only under `resources/`, then run `just reeject` and commit the
regenerated `.claude/`, `.agents/`, `.codex/` output.

<task-scenarios>
Given the `review-fanout.md` body after this task,
when its commit section is inspected,
then it contains
`{% include "modules/references/commit-recipe.md" %}`, retains the
`git add -A` staging and the `[SPEC-NNNN/T-NNN]:` title prefix, and no
longer contains the inline `git status --porcelain` pre-check (CHK-009
review side, CHK-010).

Given the same body,
when its branch statement is inspected,
then the unguarded "commits land on whatever HEAD is" statement is
retained and no `{% include "modules/references/branch-guard.md" %}`
appears (REQ-008 review side).

Given the rendered Claude Code pack after `just reeject`,
when the skills that include `review-fanout.md` are inspected,
then the recipe text is fully expanded with no residual `{{`/`{%`/`{#`
markup.

Suggested files:
`resources/modules/skills/partials/review-fanout.md`,
`speccy-cli/tests/authoring_commit.rs`,
`.claude/` `.agents/` `.codex/` (regenerated via `just reeject`)
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-004">
## Decompose commits TASKS.md alone via the shared recipe, behind the branch-guard

Rework step 4 of `resources/modules/phases/speccy-decompose.md`
(REQ-004, REQ-007 decompose side). Two edits:

- Insert a `{% include "modules/references/branch-guard.md" %}` prelude
  ahead of the commit step so the commit lands on a feature branch.
- Replace the hand-rolled bootstrap commit body with
  `{% include "modules/references/commit-recipe.md" %}` parameterised to
  **narrow staging of `<spec-dir>/TASKS.md` alone** and the title
  `[SPEC-NNNN]: decompose tasks`. The commit still runs after
  `speccy lock`.

Remove `<spec-dir>/SPEC.md` from the staging set and delete the prior
combined title string `create spec and decompose tasks` entirely —
`SPEC.md` is now committed by `speccy-plan` (REQ-003 / T-005), not here
(DEC-005). Keep the narrow `git add <path>` form (no `git add -A` /
`git add .`).

Edit only under `resources/`, then run `just reeject` and commit the
regenerated harness output.

<task-scenarios>
Given the `speccy-decompose.md` body after this task,
when its step-4 commit step is inspected,
then it includes the shared commit recipe via `{% include %}`,
stages `<spec-dir>/TASKS.md` alone (no `git add -A`/`git add .`), titles
the commit `[SPEC-NNNN]: decompose tasks`, runs after `speccy lock`, and
the string `create spec and decompose tasks` is absent (CHK-005).

Given both refactored callsites,
when `review-fanout.md` and `speccy-decompose.md` are inspected together,
then both pull the recipe via
`{% include "modules/references/commit-recipe.md" %}` and neither
restates the `git diff --cached --quiet` recipe inline (CHK-009).

Given the rendered decompose agent after `just reeject`,
when its body is inspected,
then the recipe and branch-guard text are fully expanded with no
residual MiniJinja markup.

Suggested files: `resources/modules/phases/speccy-decompose.md`,
`speccy-cli/tests/authoring_commit.rs`,
`.claude/` `.agents/` `.codex/` (regenerated via `just reeject`)
</task-scenarios>
</task>

<task id="T-005" state="pending" covers="REQ-003">
## speccy-plan commits SPEC.md after the self-review pass, behind the branch-guard

Add a commit step to `resources/modules/skills/speccy-plan.md`
(REQ-003). After the step-3 self-review pass and before the "next step"
suggestion, insert a new step that:

- runs the branch-guard prelude via
  `{% include "modules/references/branch-guard.md" %}`, then
- commits via `{% include "modules/references/commit-recipe.md" %}`
  parameterised to **narrow staging of `<spec-dir>/SPEC.md` alone** and
  the title `[SPEC-NNNN]: create spec`.

Renumber the subsequent step(s) accordingly. Use the narrow
`git add <path>` form (no `git add -A`/`git add .`). Do not alter the
existing `speccy vacancy --json` ID-allocation call (the
`speccy-plan`-must-use-`vacancy`-not-`status` invariant must stay
green).

Edit only under `resources/`, then run `just reeject` and commit the
regenerated harness output.

<task-scenarios>
Given the `speccy-plan.md` body after this task,
when its new commit step is inspected,
then it includes the branch-guard prelude and the shared commit recipe,
stages `<spec-dir>/SPEC.md` alone (no `git add -A`/`git add .`), titles
the commit `[SPEC-NNNN]: create spec`, and is positioned after the
self-review pass (CHK-004).

Given the same body,
when its ID-allocation step is inspected,
then it still invokes `speccy vacancy --json` (the prior CHK-015
invariant is preserved).

Given the rendered plan skill after `just reeject`,
when its body is inspected,
then the branch-guard and commit-recipe text are fully expanded with no
residual MiniJinja markup.

Suggested files: `resources/modules/skills/speccy-plan.md`,
`speccy-cli/tests/authoring_commit.rs`,
`.claude/` `.agents/` `.codex/` (regenerated via `just reeject`)
</task-scenarios>
</task>

<task id="T-006" state="pending" covers="REQ-005">
## speccy-amend commits its reconcile delta behind the branch-guard

Add a commit step to `resources/modules/skills/speccy-amend.md`
(REQ-005). After step 6 (re-running `speccy status` to confirm `TSK-003`
cleared) and before the "next step" suggestion, insert a new step that:

- runs the branch-guard prelude via
  `{% include "modules/references/branch-guard.md" %}`, then
- commits via `{% include "modules/references/commit-recipe.md" %}`
  parameterised to stage the amend delta — `<spec-dir>/SPEC.md`, the
  reconciled `<spec-dir>/TASKS.md` **when one exists**, and any journal
  blocker files appended this run (`<spec-dir>/journal/T-NNN.md`) — with
  the title `[SPEC-NNNN]: amend — <why>`, where `<why>` is a
  title-length phrase derived from the **newest `## Changelog` row**
  added during this amend (not separately prompted).

Specify that an absent `TASKS.md` is tolerated: when the spec has no
`TASKS.md` yet, the commit contains `SPEC.md` (plus any journal files)
without failing on the missing tasks file (CHK-007). Use narrow
`git add <path>` staging (no `git add -A`/`git add .`).

Edit only under `resources/`, then run `just reeject` and commit the
regenerated harness output.

<task-scenarios>
Given the `speccy-amend.md` body after this task,
when its new commit step is inspected,
then it includes the branch-guard prelude and the shared commit recipe,
titles the commit `[SPEC-NNNN]: amend — <why>` with `<why>` sourced from
the newest `## Changelog` row, stages SPEC.md plus reconciled-TASKS.md-
when-present plus appended journal files (no `git add -A`/`git add .`),
and runs after the `TSK-003`-clear check (CHK-006).

Given a spec with no `TASKS.md`,
when the amend commit staging set is read,
then it tolerates the absent `TASKS.md` (commits SPEC.md and any journal
files) rather than requiring the tasks file to exist (CHK-007).

Given the rendered amend skill after `just reeject`,
when its body is inspected,
then the branch-guard and commit-recipe text are fully expanded with no
residual MiniJinja markup.

Suggested files: `resources/modules/skills/speccy-amend.md`,
`speccy-cli/tests/authoring_commit.rs`,
`.claude/` `.agents/` `.codex/` (regenerated via `just reeject`)
</task-scenarios>
</task>

<task id="T-007" state="pending" covers="REQ-008 REQ-009">
## Lock the whole-set scoping and narrow-staging invariants

Add the cross-set invariant assertions to
`speccy-cli/tests/authoring_commit.rs` that can only be checked once all
four callsites (T-003..T-006) have landed. These gate genuine
regressions over a stable structural surface (the include graph and the
staging tokens), in the spirit of the existing `persona_snippets.rs` and
`skill_body_discovery.rs` suites — they must not substring-match curated
prose sentences.

Assert, over the raw `RESOURCES` module bodies:

- **Branch-guard scoping (REQ-008, CHK-011, CHK-001):** the
  `{% include "modules/references/branch-guard.md" %}` directive appears
  in exactly `skills/speccy-plan.md`, `phases/speccy-decompose.md`, and
  `skills/speccy-amend.md`, and is **absent** from
  `phases/speccy-work.md`, `phases/speccy-ship.md`, and
  `skills/partials/review-fanout.md`. The review-pass commit retains its
  unguarded "lands on whatever HEAD is" statement.
- **Narrow staging, no clean-tree gate (REQ-009, CHK-012):** none of the
  three authoring bodies contains a `git add -A` / `git add .` token in
  its commit step, and none contains a clean-tree refusal gate (assert
  against a stable token such as `git stash` / a refuse-on-dirty marker
  rather than a free-text sentence).

Then assert against `render_host_pack(HostChoice::ClaudeCode)` that the
three rendered authoring skills carry no residual `{{`/`{%`/`{#` markup
and that the branch-guard and commit-recipe text survive expansion.
Finally confirm `just reeject` leaves the working tree clean (the
committed ejected packs match their sources).

This task edits no source module; it is the whole-set verification slice.
If the reeject-clean check surfaces drift, regenerate the harness output
under `resources/` policy (never hand-edit ejected files) before
finishing.

<task-scenarios>
Given the raw `RESOURCES` module bodies after T-003..T-006,
when the branch-guard include directives are enumerated across the
skill/phase sources,
then the directive is present in exactly `speccy-plan.md`,
`speccy-decompose.md`, and `speccy-amend.md` and absent from
`speccy-work.md`, `speccy-ship.md`, and `review-fanout.md`, and the
review-pass commit retains its unguarded statement (CHK-001, CHK-011).

Given the three authoring-skill bodies,
when their commit and branch-guard steps are inspected,
then each stages a narrow path list (no `git add -A`/`git add .`) and
none contains a clean-tree refusal gate (CHK-012).

Given a clean checkout after this task,
when `just reeject` runs and `git status --porcelain` is checked,
then the working tree is clean — proving the committed ejected packs
match the updated sources.

Suggested files: `speccy-cli/tests/authoring_commit.rs`
</task-scenarios>
</task>
