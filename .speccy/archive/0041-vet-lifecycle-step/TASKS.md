---
spec: SPEC-0041
spec_hash_at_generation: dbd5f27e9d5e4d270924bfb37ecd29993f8da913e486bd02597ff1a947c5c93c
generated_at: 2026-05-23T02:27:52Z
---
# Tasks: SPEC-0041 Vet lifecycle step — `speccy next` returns `kind="vet"` between completed tasks and ship, driven by a renamed `/speccy-vet` skill

<task id="T-001" state="completed" covers="REQ-001 REQ-002">
## Add `NextAction::Vet` variant and update resolver priority

In `speccy-core/src/next.rs`:

- Add a new `Vet` variant to the `NextAction` enum, sitting
  between `Work` and `Ship` in the declaration order. No
  additional fields are required on the variant.
- Update `compute_for_spec` to check — after confirming all
  tasks are `state="completed"` and before emitting `Ship` —
  whether the gate-pass artifact at
  `<spec-dir>/journal/VET.md` is absent, ends with a
  `<gate verdict="failed" ...>` block, or ends with a
  `<gate verdict="passed" tasks_hash="X">` block whose `X`
  does not equal the lowercase hex SHA-256 of the current
  `<spec-dir>/TASKS.md` bytes. Any of those three conditions
  yields `Some(NextAction::Vet)`. Only when VET.md ends with
  a passing block whose `tasks_hash` matches does the
  resolver advance to `Ship` (REPORT.md absent) or `None`
  (REPORT.md present).
- Update the six-step priority-rule doc comment at the top of
  `next.rs` to add the `Vet` step at position 4, shifting the
  existing `Ship` / `None` steps to positions 5 and 6.
- Add unit tests covering: (a) all-completed + no VET.md →
  `Vet`; (b) all-completed + VET.md ends with
  `verdict="failed"` → `Vet`; (c) all-completed + VET.md ends
  with `verdict="passed"` but stale `tasks_hash` → `Vet`;
  (d) all-completed + VET.md ends with `verdict="passed"` and
  matching `tasks_hash`, REPORT.md absent → `Ship`;
  (e) all-completed + passing-fresh VET.md + REPORT.md present
  → `None`; (f) one `in-review` task + passing-fresh VET.md →
  `Review`, not `Vet` (priority ordering).

In `speccy-cli/src/next_output.rs`:

- Add a `NextAction::Vet` match arm to `to_json_action`
  emitting `"kind": "vet"` with no additional fields.
- Add a `NextAction::Vet` match arm to `render_text_per_spec`
  (and any other text renderer that matches exhaustively)
  printing a human-readable "vet" verb consistent with the
  existing `work`, `review`, `ship` style.

In `speccy-cli/tests/`:

- Add or extend `next_json.rs` with a fixture asserting
  `"kind":"vet"` for the all-completed / no-VET.md case.
- Add or extend `next_text.rs` to assert the plain-text "vet"
  rendering.

Hygiene gate: `cargo test --workspace`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check` — all
four must pass before flipping to `in-review`.

<task-scenarios>
Given a parsed spec fixture with all tasks `state="completed"`
and no `journal/VET.md`,
when `compute_for_spec` runs,
then the result is `Some(NextAction::Vet)` (covers CHK-001
unit-test half).

Given a parsed spec fixture with all tasks `state="completed"`
and `journal/VET.md` ending with
`<gate verdict="passed" tasks_hash="deadbeef">` while current
TASKS.md SHA-256 is `cafef00d`,
when `compute_for_spec` runs,
then the result is `Some(NextAction::Vet)` — stale hash forces
re-vetting (covers CHK-004).

Given a parsed spec fixture with one `state="in-review"` task
and a `journal/VET.md` ending with a passing-fresh gate block,
when `compute_for_spec` runs,
then the result is `Some(NextAction::Review { ... })`, not `Vet`
(covers CHK-003).

Given a built `speccy` binary at HEAD,
when `speccy next --json` runs against a workspace where
SPEC-NNNN has every task `state="completed"` and no VET.md,
then the JSON entry contains `"next_action": {"kind": "vet"}`
(covers CHK-001 binary half).

Given a built `speccy` binary at HEAD,
when `speccy next --json` runs against a workspace where
SPEC-NNNN has every task `state="completed"` and VET.md ends
with a matching passing gate block, and REPORT.md is absent,
then the JSON entry contains `"next_action": {"kind": "ship"}`
(covers CHK-002).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files:
`speccy-core/src/next.rs` (add `Vet` variant, resolver logic,
doc comment, unit tests),
`speccy-cli/src/next_output.rs` (add `Vet` match arms in JSON
and text renderers),
`speccy-cli/tests/next_json.rs` (add `kind="vet"` fixture),
`speccy-cli/tests/next_text.rs` (add "vet" text-rendering
assertion).
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-004">
## Rename the lifecycle skill and journal artifact: `speccy-holistic-gate` → `speccy-vet`, `HOLISTIC.md` → `VET.md`

This task owns all file renames and the global prose audit for the
two string tokens. REQ-005's persona renames are handled in T-003.

File moves (use `git mv` for all):

- `resources/modules/skills/speccy-holistic-gate.md` →
  `resources/modules/skills/speccy-vet.md`.
- `.claude/skills/speccy-holistic-gate/SKILL.md` →
  `.claude/skills/speccy-vet/SKILL.md` (directory rename:
  `git mv .claude/skills/speccy-holistic-gate .claude/skills/speccy-vet`).
- `.agents/skills/speccy-holistic-gate/SKILL.md` →
  `.agents/skills/speccy-vet/SKILL.md`.
- `resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl` →
  `resources/agents/.claude/skills/speccy-vet/SKILL.md.tmpl`.
- `resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl` →
  `resources/agents/.agents/skills/speccy-vet/SKILL.md.tmpl`.
- `.speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md` →
  `.speccy/specs/0038-skill-pack-references/journal/VET.md`.

After the renames, update the string content of every moved file
plus all cross-mentioning files:

- YAML `name:` frontmatter inside the renamed skill bodies:
  `speccy-holistic-gate` → `speccy-vet`.
- Every occurrence of `speccy-holistic-gate` and `HOLISTIC.md`
  inside `resources/modules/skills/*.md`,
  `.claude/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md`,
  `resources/agents/.claude/skills/*/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/*/SKILL.md.tmpl`,
  `README.md`, `docs/ARCHITECTURE.md`, `AGENTS.md`, and
  `speccy-cli/tests/skill_packs.rs`.
- In `speccy-cli/tests/skill_packs.rs` replace any assertion
  for the `speccy-holistic-gate/` directory with
  `speccy-vet/` in both `.claude/skills/` and `.agents/skills/`.

Exclusions (do not touch):
- `.speccy/specs/0039-cross-harness-orchestration-port/` and
  `.speccy/specs/0041-vet-lifecycle-step/` — historical records.
- Any `speccy-core/src/` or `speccy-cli/src/` production
  Rust sources (these do not contain the string at HEAD).

Done-when verification commands (run locally, expect zero hits):

```
rg -n 'speccy-holistic-gate' resources/modules/ resources/agents/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md
rg -n 'HOLISTIC\.md' resources/modules/ resources/agents/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md
```

And these must succeed:

```
ls .claude/skills/speccy-vet/SKILL.md
ls .agents/skills/speccy-vet/SKILL.md
ls .speccy/specs/0038-skill-pack-references/journal/VET.md
```

Hygiene gate: `cargo test --workspace` must pass (the renamed
skill_packs assertion must resolve against the new directory name).

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -n 'speccy-holistic-gate' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md` runs,
then it prints zero matches (covers CHK-007 first half).

Given the same checkout,
when `rg -n 'HOLISTIC\.md' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md` runs,
then it prints zero matches (covers CHK-007 second half).

Given the same checkout,
when `ls .claude/skills/speccy-vet/SKILL.md` and
`ls .agents/skills/speccy-vet/SKILL.md` run,
then both files exist and contain non-empty bodies; and
`ls .claude/skills/speccy-holistic-gate/ 2>&1` exits non-zero
(covers CHK-008).

Given the same checkout,
when `ls .speccy/specs/0038-skill-pack-references/journal/VET.md`
runs,
then the file exists; and
`ls .speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md 2>&1`
exits non-zero (on-disk artifact migrated cleanly).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace` runs,
then it exits 0 — `skill_packs.rs` assertions resolve
against the renamed directory.

Suggested files:
`resources/modules/skills/speccy-holistic-gate.md` (git mv →
`speccy-vet.md`; update frontmatter `name:`),
`.claude/skills/speccy-holistic-gate/` (git mv dir →
`.claude/skills/speccy-vet/`),
`.agents/skills/speccy-holistic-gate/` (git mv dir →
`.agents/skills/speccy-vet/`),
`resources/agents/.claude/skills/speccy-holistic-gate/` (git mv),
`resources/agents/.agents/skills/speccy-holistic-gate/` (git mv),
`.speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md`
(git mv → `VET.md`),
`resources/modules/skills/speccy-orchestrate.md` (update
cross-mentions),
`.claude/skills/speccy-orchestrate/SKILL.md` (update
cross-mentions),
`.agents/skills/speccy-orchestrate/SKILL.md` (update
cross-mentions),
`README.md`, `docs/ARCHITECTURE.md`, `AGENTS.md`,
`speccy-cli/tests/skill_packs.rs`.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-005">
## Rename speccy-owned sub-agent personas `holistic-*` → `vet-*` and introduce `vet-simplifier`

**Part A — Rename existing personas.**

File moves (use `git mv`):

- `resources/modules/personas/holistic-reviewer.md` →
  `resources/modules/personas/vet-reviewer.md`.
- `resources/modules/personas/holistic-implementer.md` →
  `resources/modules/personas/vet-implementer.md`.
- `.claude/agents/holistic-reviewer.md` →
  `.claude/agents/vet-reviewer.md`.
- `.claude/agents/holistic-implementer.md` →
  `.claude/agents/vet-implementer.md`.
- `.codex/agents/holistic-reviewer.toml` →
  `.codex/agents/vet-reviewer.toml`.
- `.codex/agents/holistic-implementer.toml` →
  `.codex/agents/vet-implementer.toml`.
- `resources/agents/.claude/agents/holistic-reviewer.md.tmpl` →
  `resources/agents/.claude/agents/vet-reviewer.md.tmpl`.
- `resources/agents/.claude/agents/holistic-implementer.md.tmpl` →
  `resources/agents/.claude/agents/vet-implementer.md.tmpl`.
- `resources/agents/.codex/agents/holistic-reviewer.toml.tmpl` →
  `resources/agents/.codex/agents/vet-reviewer.toml.tmpl`.
- `resources/agents/.codex/agents/holistic-implementer.toml.tmpl` →
  `resources/agents/.codex/agents/vet-implementer.toml.tmpl`.

Inside each renamed file, update the frontmatter `name:` field:
`holistic-reviewer` → `vet-reviewer` and
`holistic-implementer` → `vet-implementer`.

Update every dispatch site that names `holistic-reviewer` or
`holistic-implementer` inside the already-renamed
`resources/modules/skills/speccy-vet.md` and its ejected
copies at `.claude/skills/speccy-vet/SKILL.md` and
`.agents/skills/speccy-vet/SKILL.md`, and inside
`resources/modules/skills/speccy-orchestrate.md` and its ejected
copies.

**Part B — Introduce `vet-simplifier` persona.**

Create `resources/modules/personas/vet-simplifier.md`. The
body should:

- Set `name: vet-simplifier` in YAML frontmatter and match the
  model pinning convention used by the other `vet-*` personas
  in this repo (check the existing `vet-reviewer.md` /
  `vet-implementer.md` for the model field name and value).
- Carry a five-point persona body adapted from the upstream
  `code-simplifier` template (Preserve Functionality / Apply
  Project Standards / Enhance Clarity / Maintain Balance /
  Focus Scope), with these speccy-specific adaptations:
  - "Apply Project Standards" generalizes to the host
    project's `AGENTS.md` conventions and any
    `.claude/rules/`-equivalent rule files, removing the
    TypeScript / React inline naming.
  - A new "Phase 2 scope boundary" item states that the
    sub-agent runs after Phase 1 cleared drift on the
    cumulative SPEC-NNNN working-tree diff, and that the
    candidate scan is bounded to that diff and does not
    refactor unrelated code.
  - "Focus Scope" is tightened to "the cumulative SPEC-NNNN
    diff against the merge base", not "recently modified code
    in the current session".
- Eject to `.claude/agents/vet-simplifier.md` and
  `.codex/agents/vet-simplifier.toml`, following the same
  frontmatter shape as the other `vet-*` ejected copies.
- Create host-pack templates at
  `resources/agents/.claude/agents/vet-simplifier.md.tmpl`
  and `resources/agents/.codex/agents/vet-simplifier.toml.tmpl`,
  following the same `{% include %}` pattern used by the other
  persona templates in those directories.

Update Phase 2's dispatch site inside
`resources/modules/skills/speccy-vet.md` and its ejected copies:
- Claude Code branch: `subagent_type: "code-simplifier:code-simplifier"`
  → `subagent_type: "vet-simplifier"`.
- Codex branch: dispatch target `code-simplifier` →
  `vet-simplifier`.

Done-when verification commands:

```
rg -n 'holistic-reviewer|holistic-implementer' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/
```
(expect zero matches)

```
rg -n 'code-simplifier' resources/modules/skills/speccy-vet.md .claude/skills/speccy-vet/SKILL.md .agents/skills/speccy-vet/SKILL.md
```
(expect zero matches)

```
ls resources/modules/personas/vet-reviewer.md resources/modules/personas/vet-implementer.md resources/modules/personas/vet-simplifier.md .claude/agents/vet-reviewer.md .claude/agents/vet-implementer.md .claude/agents/vet-simplifier.md .codex/agents/vet-reviewer.toml .codex/agents/vet-implementer.toml .codex/agents/vet-simplifier.toml
```
(all must exist)

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -n 'holistic-reviewer|holistic-implementer' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/` runs,
then it prints zero matches (covers CHK-009).

Given the same checkout,
when `ls resources/modules/personas/vet-reviewer.md resources/modules/personas/vet-implementer.md resources/modules/personas/vet-simplifier.md .claude/agents/vet-reviewer.md .claude/agents/vet-implementer.md .claude/agents/vet-simplifier.md .codex/agents/vet-reviewer.toml .codex/agents/vet-implementer.toml .codex/agents/vet-simplifier.toml` runs,
then every listed path exists and contains non-empty body content
(covers CHK-010).

Given the same checkout,
when `rg -n 'code-simplifier' resources/modules/skills/speccy-vet.md .claude/skills/speccy-vet/SKILL.md .agents/skills/speccy-vet/SKILL.md` runs,
then it prints zero matches — Phase 2's dispatch no longer
names the external plugin (covers CHK-011).

Given the same checkout,
when a reader opens `resources/modules/personas/vet-simplifier.md`,
then the body contains a "Phase 2 scope boundary" paragraph
naming the cumulative SPEC diff against the merge base.

Suggested files:
`resources/modules/personas/holistic-reviewer.md` (git mv →
`vet-reviewer.md`; update `name:`),
`resources/modules/personas/holistic-implementer.md` (git mv →
`vet-implementer.md`; update `name:`),
`.claude/agents/holistic-reviewer.md` (git mv → `vet-reviewer.md`),
`.claude/agents/holistic-implementer.md` (git mv →
`vet-implementer.md`),
`.codex/agents/holistic-reviewer.toml` (git mv →
`vet-reviewer.toml`),
`.codex/agents/holistic-implementer.toml` (git mv →
`vet-implementer.toml`),
`resources/agents/.claude/agents/holistic-reviewer.md.tmpl` (git mv),
`resources/agents/.claude/agents/holistic-implementer.md.tmpl` (git mv),
`resources/agents/.codex/agents/holistic-reviewer.toml.tmpl` (git mv),
`resources/agents/.codex/agents/holistic-implementer.toml.tmpl` (git mv),
`resources/modules/personas/vet-simplifier.md` (new),
`.claude/agents/vet-simplifier.md` (new),
`.codex/agents/vet-simplifier.toml` (new),
`resources/agents/.claude/agents/vet-simplifier.md.tmpl` (new),
`resources/agents/.codex/agents/vet-simplifier.toml.tmpl` (new),
`resources/modules/skills/speccy-vet.md` (update dispatch sites),
`.claude/skills/speccy-vet/SKILL.md` (update dispatch sites),
`.agents/skills/speccy-vet/SKILL.md` (update dispatch sites).
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-003">
## Update `/speccy-vet` skill body to write a `<gate>` block to VET.md

Edit `resources/modules/skills/speccy-vet.md` (and propagate to
`.claude/skills/speccy-vet/SKILL.md` and
`.agents/skills/speccy-vet/SKILL.md`) to add an explicit
final-write step at the end of every invocation path:

After the skill body produces an `orchestrator-verdict`
final-message value — whether `verdict="pass"` or
`verdict="fail"`, including early-exit paths from Phase 0
integrity failures — the skill appends a single
`<gate verdict="passed|failed" tasks_hash="..." date="...">` block
to the current `## Invocation N` section of
`<spec-dir>/journal/VET.md`.

The block attributes are:

- `verdict`: `passed` when `orchestrator-verdict verdict="pass"`;
  `failed` when `verdict="fail"` (including Phase 0 early exits).
- `tasks_hash`: lowercase hex SHA-256 of
  `<spec-dir>/TASKS.md` byte contents read immediately before
  appending the block. The skill instructs the agent to compute
  this via `sha256sum <spec-dir>/TASKS.md | awk '{print $1}'`
  (or the PowerShell equivalent on Windows).
- `date`: ISO8601 datetime with seconds and timezone designator,
  e.g. `2026-05-22T14:30:00Z`.

The block body is a one-line human-readable summary of the
invocation outcome (e.g. "Drift cleared on round 2; simplifier
applied; clean." or "Phase 0 integrity check failed: task
T-003 not completed.").

The block is appended **after** any `<drift-review>`,
`<holistic-fix>`, `<simplifier-scan>`, and
`<simplifier-apply>` blocks within the same invocation section.
The grammar of those existing blocks is unchanged (Non-goal).

Update `docs/ARCHITECTURE.md`'s journal-artifact section
(renamed to reference `VET.md` in T-002) to add the
`<gate>` block grammar alongside the existing
`<drift-review>` / `<holistic-fix>` / `<simplifier-*>`
element family.

Note: the `vet-simplifier` Phase 2 dispatch update belongs to T-003
(which also handles the persona creation). This task focuses solely
on the `<gate>` block write mechanics.

<task-scenarios>
Given a SPEC dogfood fixture with all tasks `state="completed"`,
when `/speccy-vet SPEC-NNNN` runs to a passing verdict and exits,
then `rg -nU '<gate verdict="passed" tasks_hash="[0-9a-f]{64}"' <spec-dir>/journal/VET.md`
prints exactly one match in the most recent `## Invocation N`
section (covers CHK-005).

Given the same fixture after a passing gate has written its block,
when `sha256sum <spec-dir>/TASKS.md` is compared against the
`tasks_hash` attribute on the most recent `<gate>` block in VET.md,
then the values are byte-equal (covers CHK-006).

Given a SPEC fixture whose Phase 0 check fails (e.g. a task is
not `state="completed"`),
when `/speccy-vet` exits early with `verdict="fail"`,
then VET.md still receives a `<gate verdict="failed" ...>` block —
the on-disk record exists regardless of verdict outcome.

Given a SPEC fixture whose drift-round budget exhausts at 3
rounds without a pass,
when `/speccy-vet` returns,
then VET.md's final invocation section ends with a
`<gate verdict="failed" ...>` block, not a passing one.

Suggested files:
`resources/modules/skills/speccy-vet.md` (add `<gate>` block
write step to every exit path),
`.claude/skills/speccy-vet/SKILL.md` (propagate),
`.agents/skills/speccy-vet/SKILL.md` (propagate),
`docs/ARCHITECTURE.md` (add `<gate>` block grammar to the
VET.md journal-artifact section).
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-006">
## Update suggestion lines in `/speccy-review` and `/speccy-work` to route through `/speccy-vet`

In `resources/modules/skills/speccy-review.md` and its ejected
copies at `.claude/skills/speccy-review/SKILL.md` and
`.agents/skills/speccy-review/SKILL.md`:

- Find the closing suggestion paragraph for the "all tasks
  `state="completed"`" branch. Change the suggested next
  command from `/speccy-ship SPEC-NNNN` to
  `/speccy-vet SPEC-NNNN`.

In `resources/modules/phases/speccy-work.md` and its ejected
copies at `.claude/agents/speccy-work.md` and
`.codex/agents/speccy-work.toml`:

- Find the closing suggestion for the "all tasks completed"
  case. Change the suggestion from `/speccy-ship SPEC-NNNN`
  to the host-appropriate form — likely
  `{{ cmd_prefix }}speccy-vet SPEC-NNNN` in the shared body
  (check the existing `{{ cmd_prefix }}speccy-ship` pattern to
  match the template variable usage), and the rendered
  `/speccy-vet SPEC-NNNN` in the ejected copies.
- Do not edit the pointer-only stub files at
  `.claude/skills/speccy-work/SKILL.md` and
  `.agents/skills/speccy-work/SKILL.md` — they carry no
  suggestion line per SPEC-0023.

In `resources/modules/skills/speccy-vet.md` and its ejected
copies, confirm (and add if missing) a closing suggestion after a
passing gate verdict:

- When the gate returns `verdict="pass"` and REPORT.md is
  absent, the skill body suggests `/speccy-ship SPEC-NNNN`.

Done-when verification commands:

```
rg -n '/speccy-ship SPEC-NNNN' resources/modules/skills/speccy-review.md resources/modules/phases/speccy-work.md
```
(expect zero matches — neither body suggests ship directly)

```
rg -n 'speccy-vet SPEC-NNNN' resources/modules/skills/speccy-review.md resources/modules/phases/speccy-work.md
```
(expect at least one match per file — covers CHK-013)

```
rg -n '/speccy-ship SPEC-NNNN' resources/modules/skills/speccy-vet.md
```
(expect at least one match — the post-pass chain to ship,
covers CHK-014)

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -n '/speccy-ship SPEC-NNNN' resources/modules/skills/speccy-review.md resources/modules/phases/speccy-work.md` runs,
then it prints zero matches (covers CHK-012).

Given the same checkout,
when `rg -n 'speccy-vet SPEC-NNNN' resources/modules/skills/speccy-review.md resources/modules/phases/speccy-work.md` runs,
then it prints at least one match in each file (covers CHK-013).

Given the same checkout,
when `rg -n '/speccy-ship SPEC-NNNN' resources/modules/skills/speccy-vet.md` runs,
then it prints at least one match — the post-pass suggestion
to ship (covers CHK-014).

Given an agent finishing a `/speccy-review SPEC-NNNN`
invocation that flipped the last in-review task to
`state="completed"`, when the skill body reaches its closing
suggestion paragraph, then it suggests
`/speccy-vet SPEC-NNNN`, not `/speccy-ship SPEC-NNNN`.

Given an agent finishing a `/speccy-work SPEC-NNNN` invocation
that completed the last pending task, when the skill body
reaches its closing suggestion paragraph, then it suggests
`/speccy-vet SPEC-NNNN`.

Given an agent finishing a `/speccy-vet SPEC-NNNN` invocation
that returns `verdict="pass"`, when the skill body reaches its
closing suggestion paragraph, then it suggests
`/speccy-ship SPEC-NNNN`.

Suggested files:
`resources/modules/skills/speccy-review.md`,
`.claude/skills/speccy-review/SKILL.md`,
`.agents/skills/speccy-review/SKILL.md`,
`resources/modules/phases/speccy-work.md`,
`.claude/agents/speccy-work.md`,
`.codex/agents/speccy-work.toml`,
`resources/modules/skills/speccy-vet.md` (confirm/add post-pass
`/speccy-ship` suggestion),
`.claude/skills/speccy-vet/SKILL.md` (propagate),
`.agents/skills/speccy-vet/SKILL.md` (propagate).
</task-scenarios>
</task>
