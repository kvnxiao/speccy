---
spec: SPEC-0046
spec_hash_at_generation: e90d342db31e3f4649b0fa9dc93e97b763085c13541eecefe69eefc7bf2e67e1
generated_at: 2026-05-26T18:03:22Z
---
# Tasks: SPEC-0046 Rename the `/speccy-tasks` skill to `/speccy-decompose`

<task id="T-001" state="pending" covers="REQ-001">
## Rename the four installed skill artifacts to `speccy-decompose`

Use `git mv` (or platform equivalent that preserves history) to
rename the four checked-in skill artifacts produced by `speccy init`
so that the on-disk shape matches the new skill name. The renames:

- `.claude/skills/speccy-tasks/` → `.claude/skills/speccy-decompose/`
  (the whole directory, including the `references/` subtree).
- `.claude/agents/speccy-tasks.md` →
  `.claude/agents/speccy-decompose.md`.
- `.agents/skills/speccy-tasks/` → `.agents/skills/speccy-decompose/`
  (the whole directory, including the `references/` subtree).
- `.codex/agents/speccy-tasks.toml` →
  `.codex/agents/speccy-decompose.toml`.

After moving, update each renamed file's frontmatter / TOML so the
`name:` (Markdown frontmatter) or `name = ` (TOML) field reads
`speccy-decompose`. Update the level-1 heading and any
self-referential body lines (e.g. "# /speccy-tasks" → "#
/speccy-decompose") inside the moved files. The body otherwise stays
byte-identical apart from the slug — no behaviour edits in this
slice.

Do not yet touch the resource templates under `resources/agents/`
(that is T-002), the cross-skill references in other shipped skills
(T-003), or the test files (T-004). Leaving the templates and tests
out of sync mid-spec is expected; the spec-final state will reconcile
them.

<task-scenarios>
Given the working tree after this task,
when a recursive search for `speccy-tasks` runs over
`.claude/skills/`, `.claude/agents/`, `.agents/skills/`, and
`.codex/agents/`,
then no matches are returned.

Given the same tree,
when `.claude/skills/speccy-decompose/SKILL.md`,
`.claude/agents/speccy-decompose.md`,
`.agents/skills/speccy-decompose/SKILL.md`, and
`.codex/agents/speccy-decompose.toml` are read,
then each file exists and its `name:` (or `name = `) frontmatter
field equals `speccy-decompose`.

Given the same tree,
when `git log --follow .claude/skills/speccy-decompose/SKILL.md`
runs,
then the log surfaces commits made under the old
`.claude/skills/speccy-tasks/SKILL.md` path (history preserved).

Suggested files:
`.claude/skills/speccy-decompose/SKILL.md`,
`.claude/skills/speccy-decompose/references/*`,
`.claude/agents/speccy-decompose.md`,
`.agents/skills/speccy-decompose/SKILL.md`,
`.agents/skills/speccy-decompose/references/*`,
`.codex/agents/speccy-decompose.toml`
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-002">
## Rename the resource templates that generate the installed skills

Mirror T-001 on the template side so `speccy init` writes the new
name on fresh installs. Use `git mv` for each rename to preserve
history. The renames:

- `resources/agents/.claude/skills/speccy-tasks/` →
  `resources/agents/.claude/skills/speccy-decompose/` (whole
  directory, including any `references/` subtree).
- `resources/agents/.claude/agents/speccy-tasks.md.tmpl` →
  `resources/agents/.claude/agents/speccy-decompose.md.tmpl`.
- `resources/agents/.agents/skills/speccy-tasks/` →
  `resources/agents/.agents/skills/speccy-decompose/` (whole
  directory, including any `references/` subtree).
- `resources/agents/.codex/agents/speccy-tasks.toml.tmpl` →
  `resources/agents/.codex/agents/speccy-decompose.toml.tmpl`.
- `resources/modules/phases/speccy-tasks.md` →
  `resources/modules/phases/speccy-decompose.md` (the phase body that
  the renderer composes into the four template targets).

After moving, sweep each renamed template's body and replace every
remaining `speccy-tasks` token with `speccy-decompose` — frontmatter
`name:` / TOML `name = `, level-1 headings, internal slash-prefixed
invocations, and any references the renderer copies verbatim. The
goal is that `grep -r speccy-tasks resources/` returns zero matches
after this task.

Do not yet update the Rust render code (`speccy-cli/src/render.rs`)
or test files that hard-code the old paths — those land in T-004.

<task-scenarios>
Given the working tree after this task,
when a recursive search for `speccy-tasks` runs over `resources/`,
then no matches are returned.

Given the same tree,
when the four new template paths
`resources/agents/.claude/skills/speccy-decompose/SKILL.md.tmpl`,
`resources/agents/.claude/agents/speccy-decompose.md.tmpl`,
`resources/agents/.agents/skills/speccy-decompose/SKILL.md.tmpl`,
and `resources/agents/.codex/agents/speccy-decompose.toml.tmpl` are
read,
then each exists and contains no `speccy-tasks` token.

Given the same tree,
when `resources/modules/phases/speccy-decompose.md` is read,
then it exists and the matching old path
`resources/modules/phases/speccy-tasks.md` does not.

Suggested files:
`resources/agents/.claude/skills/speccy-decompose/SKILL.md.tmpl`,
`resources/agents/.claude/agents/speccy-decompose.md.tmpl`,
`resources/agents/.agents/skills/speccy-decompose/SKILL.md.tmpl`,
`resources/agents/.codex/agents/speccy-decompose.toml.tmpl`,
`resources/modules/phases/speccy-decompose.md`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-003">
## Update cross-skill and documentation references

Sweep every remaining `speccy-tasks` / `/speccy-tasks` reference in
the working tree (excluding `.speccy/archive/`, `target/`, `.git/`,
and the rename-target files already handled in T-001 / T-002) and
rewrite to `speccy-decompose` / `/speccy-decompose`. Touch only what
the rename requires — preserve surrounding prose verbatim.

In scope (the file list below is the floor, not the ceiling — verify
exhaustively via grep at the start and end of the task):

- Shipped skill bodies that point at the phase-2 skill as the next
  step in the loop:
  `resources/modules/skills/speccy-plan.md`,
  `resources/modules/skills/speccy-brainstorm.md`,
  `resources/modules/skills/speccy-orchestrate.md`,
  `resources/modules/phases/speccy-work.md`, and any others surfaced
  by grep.
- The installed mirrors of those skills under `.claude/skills/`,
  `.agents/skills/`, `.claude/agents/`, and `.codex/agents/` — these
  must stay in sync with their templates after T-002 lands.
  Specifically: `speccy-plan`, `speccy-brainstorm`, `speccy-orchestrate`,
  and `speccy-work` artifacts in each of the four pack locations,
  plus the `.tmpl` template counterparts under `resources/agents/`.
- `README.md` (loop diagram, phase table, prose mentions).
- `docs/ARCHITECTURE.md` (pinned-phase-workers enumeration,
  invocation examples, component table, any diagrams).

After this task, `grep -rn speccy-tasks .` (excluding the standard
exclusions above) returns matches only inside `speccy-cli/tests/`
and `speccy-cli/src/render.rs` — those are T-004's responsibility.

<task-scenarios>
Given the working tree after this task,
when a recursive search for `speccy-tasks` runs over the tree
excluding `.speccy/archive/`, `target/`, `.git/`, `speccy-cli/tests/`,
and `speccy-cli/src/render.rs`,
then no matches are returned.

Given `resources/modules/skills/speccy-plan.md` and
`resources/modules/skills/speccy-brainstorm.md` at HEAD,
when each is grepped for `/speccy-decompose`,
then at least one match is returned in each file (the "suggest the
next step" line).

Given `README.md` and `docs/ARCHITECTURE.md` at HEAD,
when each is grepped for `speccy-decompose`,
then at least one match is returned in each file, and the literal
`speccy-tasks` does not appear in either.

Suggested files:
`resources/modules/skills/speccy-plan.md`,
`resources/modules/skills/speccy-brainstorm.md`,
`resources/modules/skills/speccy-orchestrate.md`,
`resources/modules/phases/speccy-work.md`,
`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`,
`resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`,
`.claude/skills/speccy-plan/SKILL.md`,
`.claude/skills/speccy-brainstorm/SKILL.md`,
`.claude/skills/speccy-orchestrate/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.agents/skills/speccy-plan/SKILL.md`,
`.agents/skills/speccy-brainstorm/SKILL.md`,
`.agents/skills/speccy-orchestrate/SKILL.md`,
`.agents/skills/speccy-work/SKILL.md`,
`.claude/agents/speccy-work.md`,
`.codex/agents/speccy-work.toml`,
`README.md`,
`docs/ARCHITECTURE.md`
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-004">
## Update integration + inline tests and verify the hygiene suite

Update every test file that hard-codes the old `speccy-tasks` slug
or path to assert the new `speccy-decompose` shape, then run the
full standard hygiene suite and confirm zero failures and zero
warnings.

Test file scope (verify exhaustively via grep; the list below is the
floor):

- `speccy-cli/tests/init.rs` — path assertions for installed skill
  shape after `speccy init`.
- `speccy-cli/tests/init_phase_agents.rs` — phase-agent file
  assertions.
- `speccy-cli/tests/pin_shape.rs` — pinned-phase-worker assertions.
- `speccy-cli/tests/skill_packs.rs` — skill pack discovery.
- `speccy-cli/tests/skill_body_discovery.rs` — skill body lookup.
- `speccy-cli/src/render.rs` — inline `#[cfg(test)]` assertions
  (around the rendered `speccy-plan` SKILL body) currently grep for
  `/speccy-tasks` and `speccy-tasks`. Update each assertion and its
  failure-message string to the new slug.

After the edits, run the full hygiene suite. All four commands must
exit 0 with no warnings:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

Final cross-check: after this task, `grep -rn speccy-tasks .`
(excluding `.speccy/archive/`, `target/`, `.git/`, and this SPEC's
own `SPEC.md` / `TASKS.md` / journal files which legitimately
contain the literal in historical context) returns zero matches.

<task-scenarios>
Given the working tree after this task,
when `cargo test --workspace` runs,
then it exits 0 and no test newly skipped relative to `main`.

Given the same tree,
when `cargo clippy --workspace --all-targets --all-features -- -D
warnings`, `cargo +nightly fmt --all --check`, and `cargo deny
check` each run,
then each exits 0.

Given the same tree,
when a recursive search for `speccy-tasks` runs over the working
tree excluding `.speccy/archive/`, `target/`, `.git/`, and the
SPEC-0046 spec / tasks / journal files,
then no matches are returned.

Suggested files:
`speccy-cli/tests/init.rs`,
`speccy-cli/tests/init_phase_agents.rs`,
`speccy-cli/tests/pin_shape.rs`,
`speccy-cli/tests/skill_packs.rs`,
`speccy-cli/tests/skill_body_discovery.rs`,
`speccy-cli/src/render.rs`
</task-scenarios>
</task>
