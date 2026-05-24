---
spec: SPEC-0026
spec_hash_at_generation: 9782497be345665e28c15dbbe5e5373e941f75bb7d3836c08766445aef7deb30
generated_at: 2026-05-23T07:36:27Z
---

# Tasks: SPEC-0026 Anti-triggers in skill descriptions to reduce host-router mis-fires


## Phase 1: Edit source .tmpl frontmatters

<task id="T-001" state="completed" covers="REQ-001 REQ-003 REQ-005">
## T-001: Edit no-precondition skill descriptions (init, brainstorm)

Edit the two `.tmpl` source files per skill (one under
`resources/agents/.claude/skills/` and one under
`resources/agents/.agents/skills/`) for the two skills that have
no workspace preconditions: `speccy-init` and `speccy-brainstorm`.
Each description gains a `Requires: no preconditions` clause and
exactly one `Do NOT trigger…` line per the per-skill matrix in
SPEC.md `### Approach`. The two `.tmpl` files for the same skill
stay byte-identical for the frontmatter portion. No routing cues
are added for these skills (REQ-002 explicitly excludes them since
they have no precondition to route around).

For `speccy-brainstorm`, the description is the brainstorm-locked
text quoted verbatim in SPEC.md `### Approach`; implementer wording
may differ within reason but must preserve the shape (affirmative
triggers → `Requires: no preconditions` → one Do-NOT line).

For `speccy-init`, the Do-NOT line discourages firing when
`.speccy/` already exists; the implementer phrases it concisely
and points the would-be invoker at `speccy-amend` or `speccy-plan`
as the alternative.

The dogfood mirrors at `.claude/skills/speccy-{init,brainstorm}/SKILL.md`
and `.agents/skills/speccy-{init,brainstorm}/SKILL.md` are NOT
edited in this task; they regenerate in T-004.

- Suggested files:
  - `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl`
  - `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`

<task-scenarios>
  - Given the four edited `.tmpl` files, when each file's YAML
    frontmatter `description` value is read, then every value
    contains the literal substring `Requires: no preconditions`.
    The no-precondition marker is explicit (not omitted), so the
    absence is intentional rather than oversight.
  - Given the four edited `.tmpl` files, when each file's YAML
    frontmatter `description` value is read, then every value
    contains the literal substring `Do NOT trigger` exactly once.
  - Given the two `.tmpl` files for `speccy-init` (one per host
    tree), when each frontmatter `description` is extracted, then
    the two values are byte-identical. Same property holds for
    `speccy-brainstorm`'s pair.
  - Given the `speccy-init` description in either host's `.tmpl`,
    when its Do-NOT clause is read, then the clause discourages
    firing when `.speccy/` already exists and names an alternative
    skill (`speccy-amend` or `speccy-plan`) the would-be invoker
    should reach for instead.
  - Given the `speccy-brainstorm` description in either host's
    `.tmpl`, when its Do-NOT clause is read, then the clause
    discourages firing on already-scoped sharp asks and points the
    invoker at `speccy-plan` directly. The description also keeps
    the affirmative trigger phrase `I want to spec out X but I'm
    not sure where to start` (the brainstorm phase decided this
    high-signal phrase is preserved).
  - Given the `speccy-brainstorm` description in either host's
    `.tmpl`, when read, then it carries no `→ prefer speccy-`
    substring (no routing cues, per REQ-002's exemption for
    no-precondition skills). Same property holds for `speccy-init`.
  - Given each of the four edited `.tmpl` files, when the YAML
    frontmatter `description` value is parsed and
    `.chars().count()` is computed, then the count is ≤ 1024.
    Codex's `MAX_DESCRIPTION_LEN` hard-rejects descriptions over
    1024 Unicode chars; staying under is REQ-005's contract.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-005">
## T-002: Edit single-routing skill descriptions (plan, tasks, amend, ship)

Edit the two `.tmpl` source files per skill (one under
`resources/agents/.claude/skills/` and one under
`resources/agents/.agents/skills/`) for the four skills that have
one or two routing cues: `speccy-plan`, `speccy-tasks`,
`speccy-amend`, `speccy-ship`. Each description gains the
matrix-specified `Requires:` clause, at least one
`If <state-missing> → prefer <other-skill>` routing cue, and
exactly one `Do NOT trigger…` line. The two `.tmpl` files for the
same skill stay byte-identical for the frontmatter portion.

Per the per-skill matrix in SPEC.md `### Approach`:

- `speccy-plan`: `Requires: .speccy/ + AGENTS.md` ; route to
  `speccy-init` when `.speccy/` is absent ; Do-NOT on fuzzy asks
  lacking concrete scope (prefer `speccy-brainstorm` first).
- `speccy-tasks`: `Requires: SPEC.md exists` ; route to
  `speccy-plan` when no SPEC.md ; Do-NOT as part of a SPEC
  amendment (`speccy-amend` reconciles `TASKS.md`).
- `speccy-amend`: `Requires: an existing SPEC.md` ; route to
  `speccy-plan` when no SPEC.md exists yet ; Do-NOT for cosmetic
  edits to SPEC.md that do not change Requirements.
- `speccy-ship`: `Requires: all tasks completed` ; route to
  `speccy-work` to finish pending tasks ; Do-NOT while any task
  is still `pending` or `in-progress`.

The dogfood mirrors are NOT edited in this task; they regenerate
in T-004.

- Suggested files:
  - `resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl`
  - `resources/agents/.claude/skills/speccy-tasks/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-tasks/SKILL.md.tmpl`
  - `resources/agents/.claude/skills/speccy-amend/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-amend/SKILL.md.tmpl`
  - `resources/agents/.claude/skills/speccy-ship/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-ship/SKILL.md.tmpl`

<task-scenarios>
  - Given the eight edited `.tmpl` files, when each YAML
    frontmatter `description` value is read, then every value
    contains the literal substring `Requires:` followed by the
    matrix-specified precondition text (e.g., `speccy-tasks`
    contains `Requires: SPEC.md`, `speccy-ship` contains
    `Requires: all tasks completed` or wording matching that
    meaning).
  - Given the eight edited `.tmpl` files, when each YAML
    frontmatter `description` value is read, then every value
    contains at least one substring matching `→ prefer speccy-`
    followed by a known skill name (`init`, `brainstorm`, `plan`,
    `tasks`, `work`, `review`, `amend`, `ship`).
  - Given the eight edited `.tmpl` files, when each YAML
    frontmatter `description` value is read, then every value
    contains the literal substring `Do NOT trigger` exactly once.
  - Given the `speccy-plan` description in either host's `.tmpl`,
    when its Do-NOT clause is read, then the clause discourages
    firing on fuzzy asks and names `speccy-brainstorm` as the
    recommended precursor.
  - Given the `speccy-tasks` description in either host's
    `.tmpl`, when its Do-NOT clause is read, then the clause
    discourages running `speccy-tasks` as part of a SPEC
    amendment and names `speccy-amend` as the responsible skill.
  - Given the `speccy-amend` description in either host's
    `.tmpl`, when its Do-NOT clause is read, then the clause
    discourages firing for cosmetic edits to SPEC.md that do not
    change Requirements.
  - Given the `speccy-ship` description in either host's `.tmpl`,
    when its Do-NOT clause is read, then the clause discourages
    firing while any task is still `pending` or `in-progress`
    (case-insensitive match on `pending` or `in-progress`).
  - Given each pair of `.tmpl` files (one per host, four skills =
    four pairs), when the YAML frontmatter `description` value
    is extracted from each, then the two values in each pair are
    byte-identical.
  - Given each of the eight edited `.tmpl` files, when the YAML
    frontmatter `description` value is parsed and
    `.chars().count()` is computed, then the count is ≤ 1024.
    If any description would exceed the cap, the affirmative
    trigger phrase list (not the Requires/routing/Do-NOT clauses)
    is compressed until it fits.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-005">
## T-003: Edit multi-routing skill descriptions (work, review)

Edit the two `.tmpl` source files per skill for the two skills
that have multiple routing cues: `speccy-work` and
`speccy-review`. These descriptions are the tightest on character
budget (currently 492 and 538 chars, leaving the smallest
headroom under the 1024 cap) and carry the most routing fan-out,
so they are isolated in their own task for reviewability.

Per the per-skill matrix in SPEC.md `### Approach`:

- `speccy-work`: `Requires: TASKS.md with ≥1 pending task` ;
  routes to `speccy-tasks` (no TASKS.md), `speccy-plan` (no SPEC),
  `speccy-init` (no `.speccy/`) ; Do-NOT on generic "fix bug" /
  "refactor X" asks unscoped to a Speccy task.
- `speccy-review`: `Requires: a task in in-review state` ; routes
  to `speccy-work` (when no in-review task, more work to land)
  and `speccy-ship` (when all tasks complete) ; Do-NOT on generic
  "review this PR / code" asks unrelated to Speccy task-state
  review.

The two `.tmpl` files for the same skill stay byte-identical for
the frontmatter portion. The dogfood mirrors are NOT edited in
this task; they regenerate in T-004.

If after adding the matrix-specified tail clauses either
description exceeds 1024 chars, compress the affirmative trigger
phrase list first per REQ-005's done-when criterion. Do not drop
the `Requires:` / routing / Do-NOT clauses to fit the cap.

- Suggested files:
  - `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`
  - `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`
  - `resources/agents/.agents/skills/speccy-review/SKILL.md.tmpl`

<task-scenarios>
  - Given the four edited `.tmpl` files, when each YAML
    frontmatter `description` value is read, then every value
    contains the literal substring `Requires:` followed by the
    matrix-specified precondition text. The `speccy-work`
    description contains `Requires: TASKS.md`; the
    `speccy-review` description contains `Requires:` plus
    `in-review` (the task state required).
  - Given the `speccy-work` description in either host's `.tmpl`,
    when read, then it contains at least three `→ prefer speccy-`
    substrings, naming `speccy-tasks`, `speccy-plan`, and
    `speccy-init` as the routing targets for the three
    precondition shortfalls.
  - Given the `speccy-review` description in either host's
    `.tmpl`, when read, then it contains at least two
    `→ prefer speccy-` substrings, naming `speccy-work` and
    `speccy-ship` as the routing targets for the two
    no-in-review-task conditions (more work to land vs all tasks
    complete).
  - Given the four edited `.tmpl` files, when each YAML
    frontmatter `description` value is read, then every value
    contains the literal substring `Do NOT trigger` exactly once.
  - Given the `speccy-work` description in either host's `.tmpl`,
    when its Do-NOT clause is read, then the clause discourages
    generic "fix bug" or "refactor" asks unscoped to a Speccy
    task (case-insensitive match on `fix bug` or `refactor`).
  - Given the `speccy-review` description in either host's
    `.tmpl`, when its Do-NOT clause is read, then the clause
    discourages generic "review this PR / code" asks (not
    Speccy task-state review).
  - Given each pair of `.tmpl` files (one per host, two skills =
    two pairs), when the YAML frontmatter `description` value
    is extracted from each, then the two values in each pair are
    byte-identical.
  - Given each of the four edited `.tmpl` files, when the YAML
    frontmatter `description` value is parsed and
    `.chars().count()` is computed, then the count is ≤ 1024.
</task-scenarios>
</task>

## Phase 2: Regenerate dogfood mirrors and verify

<task id="T-004" state="completed" covers="REQ-004">
## T-004: Regenerate dogfood mirrors and verify byte-identity

After T-001 through T-003 have edited the 16 source `.tmpl`
files, regenerate the 16 dogfood mirrors at
`.claude/skills/speccy-*/SKILL.md` and
`.agents/skills/speccy-*/SKILL.md` by running
`cargo run -- init --force` for both hosts. The committed dogfood
files must match the renderer output byte-for-byte; the only
diffs against the prior dogfood content are the description-field
changes (the bodies, which delegate to
`resources/modules/skills/*.md` via
`{% include "modules/skills/<name>.md" %}`, are unchanged).

This task also serves as the end-to-end verification: after
regeneration, all 16 dogfood frontmatters carry the new
`Requires:` / routing / Do-NOT clauses introduced in T-001 - T-003,
proving the renderer pipeline propagates the frontmatter changes
correctly without modification.

Commit the regenerated dogfood files in the same PR as the .tmpl
source edits so the workspace stays consistent against the
canonical resource bundle.

- Suggested files:
  - `.claude/skills/speccy-amend/SKILL.md`
  - `.claude/skills/speccy-brainstorm/SKILL.md`
  - `.claude/skills/speccy-init/SKILL.md`
  - `.claude/skills/speccy-plan/SKILL.md`
  - `.claude/skills/speccy-review/SKILL.md`
  - `.claude/skills/speccy-ship/SKILL.md`
  - `.claude/skills/speccy-tasks/SKILL.md`
  - `.claude/skills/speccy-work/SKILL.md`
  - `.agents/skills/speccy-amend/SKILL.md`
  - `.agents/skills/speccy-brainstorm/SKILL.md`
  - `.agents/skills/speccy-init/SKILL.md`
  - `.agents/skills/speccy-plan/SKILL.md`
  - `.agents/skills/speccy-review/SKILL.md`
  - `.agents/skills/speccy-ship/SKILL.md`
  - `.agents/skills/speccy-tasks/SKILL.md`
  - `.agents/skills/speccy-work/SKILL.md`

<task-scenarios>
  - Given the workspace state after T-001 through T-003 have
    landed and `cargo run -- init --force --host claude-code`
    has just run, when `git status -- .claude/skills/speccy-*/SKILL.md`
    is checked, then either the eight files are modified (the
    description changes propagated through the renderer) or they
    are unchanged because the prior commit on this branch already
    captured the regenerated content; in both cases a follow-up
    `git diff .claude/skills/speccy-*/SKILL.md` against the
    committed dogfood after running the init command reports
    zero modifications (the committed state matches renderer
    output).
  - Given the workspace state after T-001 through T-003 and
    `cargo run -- init --force --host codex` has just run, when
    `git diff .agents/skills/speccy-*/SKILL.md` against the
    committed dogfood is checked, then it reports zero
    modifications.
  - Given the 16 regenerated dogfood `SKILL.md` files, when each
    file's YAML frontmatter `description` value is read, then
    every value contains the literal substring `Requires:` (per
    REQ-001).
  - Given the 16 regenerated dogfood `SKILL.md` files, when each
    file's YAML frontmatter `description` value is read, then
    every value contains the literal substring `Do NOT trigger`
    exactly once (per REQ-003).
  - Given the six precondition-bearing dogfood files for
    `speccy-plan`, `speccy-tasks`, `speccy-work`,
    `speccy-review`, `speccy-amend`, `speccy-ship` across both
    host trees (12 files total), when each file's YAML
    frontmatter `description` value is read, then every value
    contains at least one substring matching `→ prefer speccy-`
    (per REQ-002).
  - Given the four no-precondition dogfood files for
    `speccy-init` and `speccy-brainstorm` across both host trees,
    when each file's YAML frontmatter `description` value is
    read, then no value contains the substring `→ prefer speccy-`
    (per REQ-002's exemption).
  - Given each of the 16 regenerated dogfood `SKILL.md` files,
    when the YAML frontmatter `description` value is parsed and
    `.chars().count()` is computed, then the count is ≤ 1024
    (per REQ-005). This validates that the renderer did not
    inject any padding or template artifact that pushes a
    description over the cap.
  - Given the eight skill bodies (lines below the closing `---`
    of the frontmatter) in the 16 regenerated dogfood files,
    when each is compared against the corresponding body in the
    prior commit on this branch, then bodies are unchanged (the
    SPEC's contract is that only the `description` frontmatter
    field changes; body content delegated via
    `{% include "modules/skills/<name>.md" %}` is untouched).
</task-scenarios>
</task>

