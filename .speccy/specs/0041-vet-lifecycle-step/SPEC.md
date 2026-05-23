---
id: SPEC-0041
slug: vet-lifecycle-step
title: Vet lifecycle step — `speccy next` returns `kind="vet"` between completed tasks and ship, driven by a renamed `/speccy-vet` skill
status: implemented
created: 2026-05-22
supersedes: []
---

# SPEC-0041: Vet lifecycle step — `speccy next` returns `kind="vet"` between completed tasks and ship, driven by a renamed `/speccy-vet` skill

## Summary

The shipped lifecycle today is plan → tasks → work → review → ship.
SPEC-0039 added a `/speccy-holistic-gate` skill that runs a holistic
SPEC-vs-implementation drift review with a budgeted fix loop and a
simplifier polish pass at the pre-ship boundary, but the CLI's
`speccy next` resolver knows nothing about it — once every task is
`state="completed"`, the resolver jumps straight to
`next_action.kind == "ship"` and the gate is bypassed unless a human
remembers to invoke it manually.

This SPEC closes that loop. `speccy next` gains a new
`next_action.kind == "vet"` variant returned when every task is
`state="completed"` and no fresh gate-pass artifact exists at the spec
root; only after the gate writes a passing artifact does `kind` flip
to `"ship"`. The driving skill is renamed from
`/speccy-holistic-gate` to `/speccy-vet`, its journal artifact is
renamed from `HOLISTIC.md` to `VET.md`, and the speccy-owned
sub-agents the skill dispatches are renamed `holistic-reviewer` →
`vet-reviewer` and `holistic-implementer` → `vet-implementer`. A new
speccy-owned persona `vet-simplifier` is added and Phase 2's
dispatch site is changed from the external
`code-simplifier:code-simplifier` plugin to `vet-simplifier`, so all
three sub-agents the gate fans out to are speccy-owned and prefixed
`vet-*`. Net effect: the lifecycle verb, the slash-command,
the on-disk artifact, and every speccy-owned sub-agent involved in
the step share a single naming root — matching the existing
TASKS.md / REPORT.md naming convention where each artifact follows
the lifecycle step that produces it.

## Goals

<goals>
- `speccy next` returns `next_action.kind == "vet"` for a spec whose
  tasks are all `state="completed"` and whose
  `<spec-dir>/journal/VET.md` either does not exist or does not
  end with a passing gate block whose `tasks_hash` matches the
  current TASKS.md SHA-256.
- `speccy next` returns `next_action.kind == "ship"` for a spec whose
  tasks are all `state="completed"` and whose
  `<spec-dir>/journal/VET.md` ends with a passing gate block
  whose `tasks_hash` matches the current TASKS.md SHA-256.
- The skill at `resources/modules/skills/speccy-holistic-gate.md` and
  its host-pack wrappers under `.claude/skills/speccy-holistic-gate/`,
  `.agents/skills/speccy-holistic-gate/`,
  `resources/agents/.claude/skills/speccy-holistic-gate/`, and
  `resources/agents/.agents/skills/speccy-holistic-gate/` are renamed
  to `speccy-vet` end-to-end (file paths, frontmatter `name:`, and
  every prose mention inside shipped skill / agent / README /
  ARCHITECTURE / AGENTS bodies).
- The speccy-owned sub-agents are renamed: `holistic-reviewer` →
  `vet-reviewer` and `holistic-implementer` → `vet-implementer`,
  across `resources/modules/personas/`, `.claude/agents/`,
  `.codex/agents/`, the host-pack template directories under
  `resources/agents/`, and every dispatch site inside the renamed
  `speccy-vet` body and inside `speccy-orchestrate`.
- A new speccy-owned persona `vet-simplifier` is added at
  `resources/modules/personas/vet-simplifier.md` with ejected
  `.claude/agents/vet-simplifier.md` and `.codex/agents/vet-simplifier.toml`
  copies. Phase 2's dispatch site inside `speccy-vet.md` changes from
  `code-simplifier:code-simplifier` (Claude Code) /
  `code-simplifier` (Codex) to `vet-simplifier` on both hosts.
- The `/speccy-vet` skill body writes a closing
  `<gate verdict="passed" tasks_hash="...">` block to the current
  invocation section of `VET.md` when it returns
  `orchestrator-verdict verdict="pass"`, and writes
  `<gate verdict="failed" tasks_hash="...">` when it returns
  `orchestrator-verdict verdict="fail"`. `tasks_hash` is the
  lowercase hex SHA-256 of the working tree's TASKS.md bytes at the
  moment the gate block is appended.
- The closing suggestion line at the end of `/speccy-review` (shared
  body at `resources/modules/skills/speccy-review.md`, ejected to
  `.claude/skills/speccy-review/SKILL.md` and
  `.agents/skills/speccy-review/SKILL.md`) and the closing suggestion
  line at the end of `/speccy-work` (shared body at
  `resources/modules/phases/speccy-work.md`, ejected to
  `.claude/agents/speccy-work.md` and `.codex/agents/speccy-work.toml`)
  change from `/speccy-ship SPEC-NNNN` to `/speccy-vet SPEC-NNNN` for
  the "all tasks completed" case. The `/speccy-vet` body's own closing
  suggestion line names `/speccy-ship SPEC-NNNN` after a passing
  gate verdict. (`/speccy-work` is a pinned-stub phase per SPEC-0023:
  its ejected `.claude/skills/speccy-work/SKILL.md` and
  `.agents/skills/speccy-work/SKILL.md` are pointer-only and carry no
  suggestion line; the real suggestion lives in the phase body and
  agent files.)
</goals>

## Non-goals

<non-goals>
- No change to the structural shape inside each `## Invocation N`
  section of the journal. The `<drift-review>`, `<holistic-fix>`,
  and `<simplifier-scan>` / `<simplifier-apply>` element grammar
  stays exactly as SPEC-0039 defined it. The new
  `<gate verdict="...">` block is appended *after* those existing
  blocks within the same invocation section, not woven in.
- No change to the external `code-simplifier` plugin itself.
  Phase 2's dispatch target moves from the external plugin to the
  new speccy-owned `vet-simplifier` persona, but the plugin remains
  installable for other uses; this SPEC does not uninstall, vendor,
  or wrap it.
- No new CLI verb. `speccy vet` is not introduced; the lifecycle step
  is driven entirely by the skill layer, and `speccy next`'s
  `kind="vet"` payload is what tells the calling agent (or human) to
  invoke `/speccy-vet`.
- No deprecation alias. `/speccy-holistic-gate` is renamed cleanly
  to `/speccy-vet`; no transitional symlink, no
  `holistic_gate_legacy_alias` skill body, no parser fallback. The
  rename lands atomically in one PR.
- No staleness check beyond the TASKS.md SHA-256 match. The
  `tasks_hash` field is the only freshness signal; a working-tree
  change to source code that does not touch TASKS.md does not
  invalidate a passing gate. (Re-running tasks that lift code
  changes always flips a task back through `pending → in-progress
  → in-review → completed`, which mutates TASKS.md and therefore
  the hash.)
- No change to the existing `orchestrator-verdict` final-message
  return contract from the skill. That return value is what callers
  consume; the new `<gate>` block in VET.md is purely the
  on-disk artifact `speccy next` reads.
- No data migration tool. The one pre-existing `HOLISTIC.md` (in
  SPEC-0038, already shipped) is renamed in place via `git mv` as
  part of this SPEC's task list. No legacy-name fallback in any
  reader; the resolver only knows about `VET.md` after this SPEC
  lands.
</non-goals>

## User Stories

<user-stories>
- As a solo developer running `speccy next --json` between Speccy
  recipes, I want the JSON to tell me to vet the spec before
  shipping, so I do not skip the holistic drift check by accident.
- As an AI agent driving `/speccy-orchestrate` through a full SPEC,
  I want the resolver-returned `kind` to be the single source of
  truth for "what comes next", so the orchestrator does not need a
  hardcoded "after the last review, also run holistic-gate" branch
  that drifts from the CLI's view of state.
- As a reviewer reading a finished SPEC's `journal/VET.md`, I
  want a closing `<gate verdict="passed" tasks_hash="...">` block I
  can grep for, so I can tell at a glance whether the gate cleared
  on the working tree that actually shipped.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `NextAction` enum gains a `Vet` variant; JSON emits `kind="vet"`

The `speccy_core::next::NextAction` enum gains a new variant `Vet`
sitting between `Work` and `Ship` in the priority order. The
JSON serialization layer emits `"kind": "vet"` for that variant in
the `speccy next --json` output. No fields beyond the discriminant
are required on `Vet` — the caller already has the spec ID, and the
gate skill itself reads `VET.md` from `<spec-dir>` to figure
out the round/invocation number.

<done-when>
- `cargo doc -p speccy-core` shows `NextAction::Vet` in the public
  enum.
- `speccy next --json` for a spec with all tasks `state="completed"`
  and no VET.md prints `"next_action": {"kind": "vet"}` with
  no other fields under `next_action`.
- The plain-text `speccy next` output for the same spec prints a
  human-readable "vet" verb (matching the formatting style of the
  existing `ship`, `review`, `work` verbs).
- A unit test in `speccy-core/src/next.rs` constructs a parsed spec
  fixture with all tasks completed and no VET.md, calls
  `compute_for_spec`, and asserts the result is `Some(NextAction::Vet)`.
</done-when>

<behavior>
- Given a parsed spec where every task is `state="completed"` and
  `<spec-dir>/journal/VET.md` is absent, when `compute_for_spec`
  runs, then the result is `Some(NextAction::Vet)`.
- Given a parsed spec where every task is `state="completed"` and
  `<spec-dir>/journal/VET.md` exists but ends with a `<gate
  verdict="failed" tasks_hash="...">` block, when `compute_for_spec`
  runs, then the result is `Some(NextAction::Vet)`.
- Given the same spec but `VET.md` ends with a passing
  `<gate verdict="passed" tasks_hash="...">` block whose
  `tasks_hash` matches the current TASKS.md SHA-256, when
  `compute_for_spec` runs, then the result is `Some(NextAction::Ship)`
  (REPORT.md absent) or `None` (REPORT.md present).
</behavior>

<scenario id="CHK-001">
Given a `speccy` binary built at HEAD,
when `speccy next --json` runs against a workspace where SPEC-NNNN
has every task `state="completed"` and no
`.speccy/specs/NNNN-slug/journal/VET.md`,
then the JSON entry for that spec contains
`"next_action": {"kind": "vet"}`.
</scenario>

<scenario id="CHK-002">
Given a `speccy` binary built at HEAD,
when `speccy next --json` runs against a workspace where SPEC-NNNN
has every task `state="completed"` and VET.md ends with
`<gate verdict="passed" tasks_hash="...">` where the hex SHA-256
matches the current TASKS.md byte contents, and REPORT.md is absent,
then the JSON entry for that spec contains
`"next_action": {"kind": "ship"}`.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Resolver priority slots `Vet` between `Work` and `Ship`

The priority rule in `speccy_core::next::compute_for_spec` is updated
to:

1. TASKS.md absent → `Decompose`
2. Any task `state="in-review"` → `Review`
3. Any task `state="pending"` → `Work`
4. All tasks `state="completed"`, gate-pass artifact missing or
   stale → `Vet`
5. All tasks `state="completed"`, gate-pass artifact present and
   fresh, REPORT.md absent → `Ship`
6. All tasks `state="completed"`, gate-pass artifact present and
   fresh, REPORT.md present → `None` (spec omitted from workspace
   listing)

"Gate-pass artifact fresh" means: `VET.md` exists, its final
non-whitespace block is `<gate verdict="passed" tasks_hash="X">`,
and `X` equals the lowercase hex SHA-256 of the current
`<spec-dir>/TASKS.md` byte contents.

<done-when>
- The priority rule documented in `speccy-core/src/next.rs`'s
  module-level doc comment is updated to list six rules in the
  order above (or equivalent prose).
- The `compute_for_spec` function's match logic implements that
  order — verified by unit tests covering each transition.
- A spec with `state="in-review"` tasks still returns `Review`
  regardless of any VET.md presence (the gate-pass artifact
  is checked only when all tasks are completed).
- A spec with `state="pending"` tasks still returns `Work`
  regardless of any VET.md presence.
</done-when>

<behavior>
- Given a parsed spec with one `state="in-review"` task and a
  passing-fresh VET.md, when `compute_for_spec` runs, then
  the result is `Some(NextAction::Review { ... })`.
- Given a parsed spec with all tasks completed, REPORT.md absent,
  VET.md present with `<gate verdict="passed">` but whose
  `tasks_hash` does NOT match current TASKS.md, when
  `compute_for_spec` runs, then the result is
  `Some(NextAction::Vet)` (stale gate-pass forces re-vetting).
- Given a parsed spec with all tasks completed, REPORT.md present,
  VET.md ending with a passing-fresh gate block, when
  `compute_for_spec` runs, then the result is `None` (omitted from
  workspace listing).
</behavior>

<scenario id="CHK-003">
Given a parsed spec fixture with one `state="in-review"` task and
a `journal/VET.md` ending with
`<gate verdict="passed" tasks_hash="...">`,
when `compute_for_spec` runs,
then the returned `NextAction` is `Review { task_id, personas }`,
not `Vet` and not `Ship`.
</scenario>

<scenario id="CHK-004">
Given a parsed spec fixture with all tasks `state="completed"`,
`journal/VET.md` ending with
`<gate verdict="passed" tasks_hash="deadbeef">`, and a current
TASKS.md whose SHA-256 is `cafef00d`,
when `compute_for_spec` runs,
then the returned `NextAction` is `Vet` (stale-hash branch).
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `/speccy-vet` skill body writes a `<gate>` block to VET.md

The shared skill body at `resources/modules/skills/speccy-vet.md`
(renamed from `speccy-holistic-gate.md`) is updated so that the
final on-disk write of every invocation appends a single
`<gate verdict="passed|failed" tasks_hash="..." date="...">` element
block to the current `## Invocation N` section of
`<spec-dir>/journal/VET.md`, **after** any
`<drift-review>` / `<holistic-fix>` / `<simplifier-scan>` /
`<simplifier-apply>` blocks the invocation already wrote.

The block carries these required attributes:

- `verdict` — `passed` or `failed`, mirroring the
  `orchestrator-verdict` final-message verdict (passed when
  `verdict="pass"`, failed when `verdict="fail"`).
- `tasks_hash` — lowercase hex SHA-256 of the
  `<spec-dir>/TASKS.md` byte contents read immediately before the
  block is appended.
- `date` — ISO8601 with seconds and timezone designator.

The block body is a one-line human-readable summary of the
invocation outcome (e.g. "Drift cleared on round 2; simplifier
applied; clean.").

<done-when>
- A run of `/speccy-vet` against a SPEC with all tasks completed
  appends exactly one `<gate>` block to the current invocation
  section; no `<gate>` block is appended to any other invocation
  section.
- The `<gate>` block always comes after every other element block
  in the same invocation section.
- The `tasks_hash` value reproduces under
  `sha256sum <spec-dir>/TASKS.md | awk '{print $1}'` against the
  working tree at the moment the gate appended the block.
- A failing gate writes `verdict="failed"`; a passing gate writes
  `verdict="passed"`. The block is written in both outcomes so the
  audit trail records the verdict regardless.
</done-when>

<behavior>
- Given a spec whose drift-review subagent returns `verdict="pass"`
  on round 1, when `/speccy-vet` returns, then VET.md's final
  invocation section ends with a `<gate verdict="passed" ...>` block
  whose `tasks_hash` matches the current TASKS.md.
- Given a spec whose drift round budget exhausts at 3 without a pass,
  when `/speccy-vet` returns, then VET.md's final invocation
  section ends with a `<gate verdict="failed" ...>` block whose
  `tasks_hash` matches the current TASKS.md.
- Given a spec whose Phase 0 integrity check fails (e.g. a task is
  not `state="completed"`), when `/speccy-vet` returns early with
  `verdict="fail"`, then VET.md still gets a `<gate
  verdict="failed">` block recording the early exit. (Phase 0
  failures are recorded as failed gate-passes so `speccy next` keeps
  surfacing `kind="vet"` rather than oscillating.)
</behavior>

<scenario id="CHK-005">
Given a built `speccy` binary at HEAD and a SPEC dogfood fixture
whose tasks are all `state="completed"`,
when `/speccy-vet SPEC-NNNN` runs to a passing verdict and exits,
then `rg -nU '<gate verdict="passed" tasks_hash="[0-9a-f]{64}"' <spec-dir>/journal/VET.md`
prints exactly one match in the most recent `## Invocation N`
section.
</scenario>

<scenario id="CHK-006">
Given the same fixture after a passing gate has written its block,
when `sha256sum <spec-dir>/TASKS.md` is compared against the
`tasks_hash` attribute on the most recent `<gate>` block in
VET.md,
then the values are byte-equal.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: The lifecycle skill and journal artifact are renamed `speccy-holistic-gate` → `speccy-vet` and `HOLISTIC.md` → `VET.md`

Every shipped path, frontmatter `name:` field, and prose mention of
the `speccy-holistic-gate` skill is renamed to `speccy-vet`, and the
journal artifact `<spec-dir>/journal/HOLISTIC.md` is renamed to
`<spec-dir>/journal/VET.md` everywhere — both the existing on-disk
file under SPEC-0038 and every prose mention in shipped skill / agent
/ docs bodies:

- `resources/modules/skills/speccy-holistic-gate.md` →
  `resources/modules/skills/speccy-vet.md`.
- `.claude/skills/speccy-holistic-gate/SKILL.md` →
  `.claude/skills/speccy-vet/SKILL.md` (directory rename).
- `.agents/skills/speccy-holistic-gate/SKILL.md` →
  `.agents/skills/speccy-vet/SKILL.md`.
- `resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl` →
  `resources/agents/.claude/skills/speccy-vet/SKILL.md.tmpl`.
- `resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl` →
  `resources/agents/.agents/skills/speccy-vet/SKILL.md.tmpl`.
- YAML `name:` frontmatter inside the renamed skill bodies updates
  from `speccy-holistic-gate` to `speccy-vet`.
- Every cross-mention inside `resources/modules/skills/*.md`,
  `.claude/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md`,
  `README.md`, `docs/ARCHITECTURE.md`, `AGENTS.md`, and the
  shipped skill-pack assertions in
  `speccy-cli/tests/skill_packs.rs` updates from
  `speccy-holistic-gate` to `speccy-vet`.
- No `speccy-holistic-gate` literal survives in any production
  source file under `speccy-core/src/`, `speccy-cli/src/`,
  `resources/`, `.claude/`, or `.agents/`. The string remains
  permissible inside `.speccy/specs/0039-cross-harness-orchestration-port/`
  (SPEC-0039's own history names the old skill) and inside this
  SPEC-0041 directory (which documents the rename).
- The pre-existing journal file at
  `.speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md` is
  renamed to `VET.md` via `git mv` as part of this SPEC's task list.
  No legacy-name fallback in any reader.
- Every prose mention of `HOLISTIC.md` inside
  `resources/modules/skills/*.md`, `.claude/skills/*/SKILL.md`,
  `.agents/skills/*/SKILL.md`, `resources/agents/.claude/skills/*/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/*/SKILL.md.tmpl`,
  `docs/ARCHITECTURE.md`, `README.md`, and `AGENTS.md` is
  updated to `VET.md`. The string `HOLISTIC.md` remains permissible
  inside `.speccy/specs/0039-cross-harness-orchestration-port/`
  (SPEC-0039's own history names the old artifact) and inside this
  SPEC-0041 directory (which documents the rename).
- The speccy-owned sub-agent personas are renamed and reintroduced in
  REQ-005; this REQ owns the skill + journal rename and the docs
  audit, REQ-005 owns the persona work.

<done-when>
- `rg -n 'speccy-holistic-gate' resources/modules/ resources/agents/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md`
  prints zero matches.
- `rg -n 'HOLISTIC\.md' resources/modules/ resources/agents/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md`
  prints zero matches.
- `rg -n 'VET\.md' resources/modules/skills/speccy-vet.md resources/modules/skills/speccy-orchestrate.md`
  prints at least one match in each file, proving the renamed journal
  artifact name lives in the renamed skill body and in the
  orchestrator's references to it.
- `ls .speccy/specs/0038-skill-pack-references/journal/VET.md` succeeds
  and `ls .speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md 2>&1`
  exits non-zero (the SPEC-0038 file moved cleanly).
- `speccy-cli/tests/skill_packs.rs` asserts the renamed
  `speccy-vet/` directory in both `.claude/skills/` and
  `.agents/skills/`, with no `speccy-holistic-gate/` directory
  assertion remaining.
</done-when>

<behavior>
- Given the rendered Claude Code skill pack at HEAD after this SPEC
  lands, when a user invokes `/speccy-vet SPEC-NNNN`, then the
  skill at `.claude/skills/speccy-vet/SKILL.md` is loaded and
  executes the holistic-gate logic.
- Given the rendered Codex skill pack at HEAD after this SPEC lands,
  when a user invokes `/speccy-vet SPEC-NNNN`, then the skill at
  `.agents/skills/speccy-vet/SKILL.md` is loaded and executes the
  same logic.
- Given an agent reading `resources/modules/skills/speccy-vet.md`,
  when it reaches a dispatch site for the drift reviewer, then the
  sub-agent name in the dispatch call is `vet-reviewer` (per REQ-005).
</behavior>

<scenario id="CHK-007">
Given the source tree at HEAD after this SPEC lands,
when `rg -n 'speccy-holistic-gate' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md` runs,
then it prints zero matches; and when `rg -n 'HOLISTIC\.md' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/ README.md AGENTS.md docs/ARCHITECTURE.md` runs, then it prints zero matches.
</scenario>

<scenario id="CHK-008">
Given the same checkout,
when `ls .claude/skills/speccy-vet/SKILL.md` and `ls .agents/skills/speccy-vet/SKILL.md` run,
then both files exist and contain non-empty bodies; and `ls .claude/skills/speccy-holistic-gate/ 2>&1` exits non-zero (directory absent).
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Speccy-owned sub-agents renamed `holistic-*` → `vet-*`; new `vet-simplifier` introduced

The two existing speccy-owned persona files are renamed and their
frontmatter `name:` fields updated:

- `resources/modules/personas/holistic-reviewer.md` →
  `resources/modules/personas/vet-reviewer.md`; `name: holistic-reviewer`
  → `name: vet-reviewer`.
- `resources/modules/personas/holistic-implementer.md` →
  `resources/modules/personas/vet-implementer.md`;
  `name: holistic-implementer` → `name: vet-implementer`.

The ejected host-pack copies are renamed in lockstep:

- `.claude/agents/holistic-reviewer.md` → `.claude/agents/vet-reviewer.md`.
- `.claude/agents/holistic-implementer.md` → `.claude/agents/vet-implementer.md`.
- `.codex/agents/holistic-reviewer.toml` → `.codex/agents/vet-reviewer.toml`.
- `.codex/agents/holistic-implementer.toml` → `.codex/agents/vet-implementer.toml`.
- `resources/agents/.claude/agents/holistic-reviewer.md.tmpl` →
  `resources/agents/.claude/agents/vet-reviewer.md.tmpl`.
- `resources/agents/.claude/agents/holistic-implementer.md.tmpl` →
  `resources/agents/.claude/agents/vet-implementer.md.tmpl`.
- `resources/agents/.codex/agents/holistic-reviewer.toml.tmpl` →
  `resources/agents/.codex/agents/vet-reviewer.toml.tmpl`.
- `resources/agents/.codex/agents/holistic-implementer.toml.tmpl` →
  `resources/agents/.codex/agents/vet-implementer.toml.tmpl`.

A new third speccy-owned persona `vet-simplifier` is added:

- New `resources/modules/personas/vet-simplifier.md`. The body is
  adapted from the upstream
  `claude-plugins-official/plugins/code-simplifier/agents/code-simplifier.md`
  template (frontmatter `name: code-simplifier`, model `opus`, the
  five-point "Preserve Functionality / Apply Project Standards /
  Enhance Clarity / Maintain Balance / Focus Scope" persona body).
  Adaptations for the speccy Phase 2 context:
  - `name:` becomes `vet-simplifier`.
  - "Project Standards" item is generalized to point at the host
    project's `AGENTS.md` conventions and any
    `.claude/rules/`-equivalent rule files, rather than naming
    specific language conventions inline (the upstream body names
    TypeScript / React patterns).
  - A new "Phase 2 scope boundary" item is added stating that the
    sub-agent runs after Phase 1 cleared drift on the cumulative
    SPEC-NNNN working-tree diff; the candidate scan is bounded to
    that diff and does not refactor unrelated code.
  - The "Focus Scope" item is tightened to "the cumulative SPEC-NNNN
    diff against the merge base", not "recently modified code in
    the current session".
- Ejected copies at `.claude/agents/vet-simplifier.md` and
  `.codex/agents/vet-simplifier.toml`.
- Host-pack templates at
  `resources/agents/.claude/agents/vet-simplifier.md.tmpl` and
  `resources/agents/.codex/agents/vet-simplifier.toml.tmpl`.

Phase 2's dispatch site inside the renamed `speccy-vet.md` skill
body changes:

- Claude Code branch: `subagent_type: "code-simplifier:code-simplifier"`
  → `subagent_type: "vet-simplifier"`.
- Codex branch: dispatch target `code-simplifier` → `vet-simplifier`.

Every dispatch site that names the old `holistic-*` persona inside
`resources/modules/skills/speccy-vet.md` (and its ejected copies)
and inside `resources/modules/skills/speccy-orchestrate.md` (and its
ejected copies) is renamed to the corresponding `vet-*` name.

<done-when>
- `ls resources/modules/personas/vet-reviewer.md
  resources/modules/personas/vet-implementer.md
  resources/modules/personas/vet-simplifier.md` succeeds (all three
  exist).
- `ls resources/modules/personas/holistic-reviewer.md 2>&1` and
  `ls resources/modules/personas/holistic-implementer.md 2>&1` both
  exit non-zero (the two source persona files moved).
- `rg -n 'holistic-reviewer|holistic-implementer' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/`
  prints zero matches.
- `rg -n 'code-simplifier' resources/modules/skills/speccy-vet.md
  .claude/skills/speccy-vet/SKILL.md
  .agents/skills/speccy-vet/SKILL.md`
  prints zero matches — Phase 2's dispatch no longer names the
  external plugin.
- `rg -n 'vet-reviewer|vet-implementer|vet-simplifier' resources/modules/personas/`
  prints at least one match per file (each persona body references
  its own `name:`).
- The new `vet-simplifier.md` body contains a "Phase 2 scope boundary"
  paragraph naming the cumulative SPEC diff against the merge base.
</done-when>

<behavior>
- Given the rendered Claude Code skill pack at HEAD after this SPEC
  lands, when `/speccy-vet` runs Phase 1, then the dispatch target
  for drift review is `vet-reviewer` and the dispatch target for
  drift fixing is `vet-implementer`.
- Given the same skill pack runs Phase 2, when the simplifier scan
  spawns, then the dispatch target is `vet-simplifier` (speccy-owned)
  on both Claude Code and Codex — proving the cross-host parity gap
  is closed (the external `code-simplifier:code-simplifier` plugin
  is Claude-Code-only).
- Given the rendered Codex skill pack at HEAD after this SPEC lands,
  when `/speccy-vet` runs any phase, then every dispatch target is a
  speccy-owned `vet-*` sub-agent and zero dispatch sites name
  `code-simplifier`.
</behavior>

<scenario id="CHK-009">
Given the source tree at HEAD after this SPEC lands,
when `rg -n 'holistic-reviewer|holistic-implementer' resources/ .claude/ .agents/ speccy-core/src/ speccy-cli/src/ speccy-cli/tests/` runs,
then it prints zero matches.
</scenario>

<scenario id="CHK-010">
Given the same checkout,
when `ls resources/modules/personas/vet-reviewer.md resources/modules/personas/vet-implementer.md resources/modules/personas/vet-simplifier.md .claude/agents/vet-reviewer.md .claude/agents/vet-implementer.md .claude/agents/vet-simplifier.md .codex/agents/vet-reviewer.toml .codex/agents/vet-implementer.toml .codex/agents/vet-simplifier.toml` runs,
then every listed path exists and contains non-empty body content.
</scenario>

<scenario id="CHK-011">
Given the same checkout,
when `rg -n 'code-simplifier' resources/modules/skills/speccy-vet.md .claude/skills/speccy-vet/SKILL.md .agents/skills/speccy-vet/SKILL.md` runs,
then it prints zero matches.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Skill-suggestion lines route through `/speccy-vet` instead of `/speccy-ship`

The closing suggestion line in `/speccy-review` — shared body at
`resources/modules/skills/speccy-review.md`, ejected to
`.claude/skills/speccy-review/SKILL.md` and
`.agents/skills/speccy-review/SKILL.md` — and the closing suggestion
line in `/speccy-work` — shared body at
`resources/modules/phases/speccy-work.md`, ejected to
`.claude/agents/speccy-work.md` and `.codex/agents/speccy-work.toml` —
update so that the "all tasks `state="completed"`" branch suggests
`/speccy-vet SPEC-NNNN` instead of `/speccy-ship SPEC-NNNN`.

`/speccy-work` is a pinned-stub phase per SPEC-0023; the ejected
`.claude/skills/speccy-work/SKILL.md` and
`.agents/skills/speccy-work/SKILL.md` are pointer-only bodies that
delegate to the agent file and therefore carry no suggestion line to
edit. The host-pack templates at
`resources/agents/.claude/agents/speccy-work.md.tmpl` and
`resources/agents/.codex/agents/speccy-work.toml.tmpl` already
`{% include %}` the renamed phase body and need no edit beyond the
shared-body change.

The renamed `/speccy-vet` skill body's own closing suggestion line
names `/speccy-ship SPEC-NNNN` when the gate verdict is `passed` and
the spec still has no REPORT.md, mirroring how
`/speccy-review` and `/speccy-work` already chain to whatever the
next reasonable step is.

The orchestrator at `resources/modules/skills/speccy-orchestrate.md`
(and its ejected copies) already dispatches to `/speccy-vet` for the
pre-ship gate; this SPEC also updates that dispatch site if it
currently names `/speccy-holistic-gate` (per REQ-004 it already
renames as part of the global rename).

<done-when>
- `rg -n '/speccy-ship' resources/modules/skills/speccy-review.md
  resources/modules/phases/speccy-work.md`
  no longer prints the "all tasks completed → /speccy-ship"
  suggestion. The string `/speccy-ship` survives in
  `resources/modules/skills/speccy-vet.md` (its own
  closing-after-pass suggestion).
- `rg -n 'speccy-vet' resources/modules/skills/speccy-review.md
  resources/modules/phases/speccy-work.md` prints at least one
  match in each file. (The phase body addresses the verb as
  `{{ cmd_prefix }}speccy-vet`, so the pattern omits the leading
  slash to match both call sites.)
- The ejected `.claude/skills/speccy-review/SKILL.md` and
  `.agents/skills/speccy-review/SKILL.md` carry the `/speccy-vet`
  suggestion line, and the ejected `.claude/agents/speccy-work.md`
  and `.codex/agents/speccy-work.toml` carry the `/speccy-vet`
  suggestion line.
</done-when>

<behavior>
- Given an agent finishing a `/speccy-review SPEC-NNNN` invocation
  that flipped the last in-review task to `state="completed"`,
  when the skill body reaches its closing suggestion paragraph,
  then it suggests `/speccy-vet SPEC-NNNN` (not
  `/speccy-ship SPEC-NNNN`).
- Given an agent finishing a `/speccy-work SPEC-NNNN` invocation
  that completed the last pending task, when the skill body
  reaches its closing suggestion paragraph, then it suggests
  `/speccy-vet SPEC-NNNN`.
- Given an agent finishing a `/speccy-vet SPEC-NNNN` invocation
  that returns `verdict="pass"`, when the skill body reaches its
  closing suggestion paragraph, then it suggests
  `/speccy-ship SPEC-NNNN`.
</behavior>

<scenario id="CHK-012">
Given the source tree at HEAD,
when `rg -n '/speccy-ship SPEC-NNNN' resources/modules/skills/speccy-review.md resources/modules/phases/speccy-work.md` runs,
then it prints zero matches.
</scenario>

<scenario id="CHK-013">
Given the same checkout,
when `rg -n 'speccy-vet SPEC-NNNN' resources/modules/skills/speccy-review.md resources/modules/phases/speccy-work.md` runs,
then it prints at least one match in each file.
</scenario>

<scenario id="CHK-014">
Given the same checkout,
when `rg -n '/speccy-ship SPEC-NNNN' resources/modules/skills/speccy-vet.md` runs,
then it prints at least one match (the post-pass chain to ship).
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
The new `next_action.kind` value is `"vet"`, not `"audit"`,
`"reconcile"`, `"harden"`, `"certify"`, or `"finalize"`. `audit`
implies inspect-only; the gate also fixes via its budgeted loop.
`reconcile` and `harden` overweight the fix-loop side when the
common outcome is pass-on-first-try. `certify` reads as formal-tone
for a JSON enum. `vet` captures "checkpoint that mostly passes,
occasionally needs a round of corrections before clearing" with the
right imperative cadence alongside `implement` / `review` / `ship`.
</decision>

<decision id="DEC-002">
The lifecycle skill, journal artifact, AND the speccy-owned
sub-agent personas all carry the `vet-*` prefix: skill `/speccy-vet`,
artifact `VET.md`, sub-agents `vet-reviewer` / `vet-implementer` /
`vet-simplifier`. Discoverability beats the activity-vs-step
distinction: a user looking at a `vet-*` named persona under
`.claude/agents/` understands at a glance which lifecycle step it
serves, the same way users of `/speccy-tasks` recognise `TASKS.md`
without reading the schema. The reviewer-fan-out personas
(`reviewer-business`, `reviewer-tests`, etc.) follow the same
prefix-by-step convention. The one pre-existing `HOLISTIC.md` (in
SPEC-0038, already shipped) is migrated in place via `git mv` —
SPEC-0038's REPORT.md is present so `compute_for_spec` returns
`None` for it regardless of the file's name, but the rename keeps
the on-disk artifact discoverable under the new name.
</decision>

<decision id="DEC-005">
Phase 2's simplifier sub-agent moves from the external
`code-simplifier:code-simplifier` (Anthropic claude-plugins-official)
to a speccy-owned `vet-simplifier` persona. Two motivations:

1. **Cross-host parity.** The `code-simplifier` plugin is delivered
   as a Claude Code plugin; the Codex host does not have access to
   it. Dispatching to an external Claude-only plugin breaks the
   "skill packs ship the full development loop end-to-end on either
   host" promise in `AGENTS.md`. A speccy-owned persona that ejects
   into both `.claude/agents/` and `.codex/agents/` closes the gap.
2. **Scope alignment.** The upstream `code-simplifier` focuses on
   "recently modified code in the current session". Phase 2 of the
   gate needs the cumulative SPEC-NNNN diff against the merge base,
   not session-scoped recent edits — the gate runs as one
   sub-agent invocation in a fresh context with no prior session
   history. Owning the persona lets us scope it accurately.

The body is adapted from the upstream template at
`https://raw.githubusercontent.com/anthropics/claude-plugins-official/refs/heads/main/plugins/code-simplifier/agents/code-simplifier.md`,
preserving the five-point structure and intent while replacing the
language-specific TypeScript / React conventions with a pointer to
the host project's `AGENTS.md` and `.claude/rules/`-equivalent rule
files, and tightening "Focus Scope" to the cumulative SPEC diff.
</decision>

<decision id="DEC-003">
Gate-pass freshness is signalled by a `tasks_hash` SHA-256 of
TASKS.md recorded on the `<gate>` block. No content hash of source
code, no commit SHA, no working-tree diff hash. Rationale: any
re-work that lifts code changes always passes through a per-task
state transition (`completed → pending → in-progress → in-review →
completed`), which mutates TASKS.md and therefore the hash. A
working-tree edit that does not touch any task's state is not the
kind of change the gate is supposed to re-verify; that is what
PR review is for.
</decision>

<decision id="DEC-004">
The rename is atomic — no `/speccy-holistic-gate` alias survives,
no parser fallback for the legacy slash-command name. Speccy is
pre-v1; the cost of renaming cleanly is one PR, the cost of
carrying a transitional alias is permanent muscle-memory ambiguity
in `speccy next`'s contract and in every shipped skill body.
</decision>

## Notes

- The `<gate>` block grammar is added to `docs/ARCHITECTURE.md`'s
  journal-artifact section (renamed to reference `VET.md`) as part of
  this SPEC's task list, alongside the existing `<drift-review>` /
  `<holistic-fix>` / `<simplifier-*>` element family.
- The pre-existing journal file at
  `.speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md` is
  renamed to `VET.md` via `git mv`. After the rename, the file has no
  closing `<gate>` block — that pre-dates this SPEC. SPEC-0038
  already shipped (`state="completed"` plus REPORT.md present means
  `compute_for_spec` returns `None` for it regardless of journal
  contents), so the historical record stays readable under the new
  name without needing a `<gate>` retrofit. The new staleness check
  applies only to specs that reach the gate boundary after this SPEC
  lands.
- The lint suite gains no new code family for the `<gate>` block.
  Shape errors in the block surface as parse failures in
  `compute_for_spec` (which falls back to `Vet` on parse failure —
  safer to re-vet than to ship). Lint inspection of VET.md as
  a structural artifact is a follow-up SPEC if the on-disk shape
  ever starts drifting.

## Open Questions

(None.)

## Changelog

<changelog>
| Date       | Reason                                                   | Author     |
|------------|----------------------------------------------------------|------------|
| 2026-05-22 | Initial draft. Add a `vet` lifecycle step between completed tasks and ship, driven by a renamed `/speccy-vet` skill (formerly `/speccy-holistic-gate`). Introduces `NextAction::Vet` and `kind = "vet"`; renames the journal artifact to `VET.md`; renames the speccy-owned sub-agents to `vet-reviewer` / `vet-implementer` and adds a new speccy-owned `vet-simplifier` persona so Phase 2 no longer depends on the external Claude-only `code-simplifier` plugin. Gate writes a `<gate verdict="passed\|failed" tasks_hash="...">` block to `VET.md`; resolver uses `tasks_hash` against current TASKS.md as the freshness signal. Skill-suggestion lines in `/speccy-review` and `/speccy-work` route to `/speccy-vet` instead of `/speccy-ship`. Pre-v1; atomic rename, no transitional alias. | Kevin Xiao |
| 2026-05-22 | Pre-decomposition correction. Reconcile lifecycle-verb terminology with SPEC-0040, which renamed `NextAction::Implement` → `NextAction::Work` and JSON `kind="implement"` → `kind="work"`: update REQ-001 / REQ-002 prose, priority-rule list, and human-readable verb examples accordingly. Correct REQ-006 and the matching goal line to point at the real shared-body locations for `/speccy-work`: the pinned-stub phase body lives at `resources/modules/phases/speccy-work.md` (not under `resources/modules/skills/`) and ejects to `.claude/agents/speccy-work.md` + `.codex/agents/speccy-work.toml`, not the pointer-only `.claude/skills/speccy-work/SKILL.md` / `.agents/skills/speccy-work/SKILL.md` shims. Updates CHK-012 / CHK-013 paths to match. | Kevin Xiao |
</changelog>
