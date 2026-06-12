---
spec: SPEC-0053
spec_hash_at_generation: 5520172edbf5b5bfdf03f68d8cfc2272aead4fb8d359e42fc2e6e178c6034fd8
generated_at: 2026-05-29T18:07:18Z
---
# Tasks: SPEC-0053 Port feature-dev agents into speccy — correctness reviewer, plan-explorer, plan-architect, read-only tool hardening, brainstorm trigger phrase

<task id="T-001" state="completed" covers="REQ-001 REQ-005">
## Add the `reviewer-correctness` persona end-to-end (body, wrappers, core registry, default fan-out)

Land the correctness reviewer as one atomic slice so the build never
goes red mid-task — the body, the host wrappers, the `speccy-core`
registry entry, and the dependent tests are mutually dependent for a
green `cargo test`.

1. Write `resources/modules/personas/reviewer-correctness.md`: an
   adversarial reviewer scoped to correctness/logic defects only (logic
   and control-flow errors, null/`Option`/`Result` mishandling,
   off-by-one and boundary conditions, non-security races/deadlocks,
   resource leaks). It must (a) `{% include %}` the shared review-contract
   snippets the other `reviewer-*` bodies use (`verdict_return_contract.md`,
   `inline_note_format.md`, `diff_fetch_command.md`, `no_tasks_md_writes.md`),
   (b) name the four deferral targets (security, style, business, tests)
   as out of its own lane, (c) state the confidence-≥80 reporting
   threshold and Critical/Important severity grouping, and (d) carry a
   one-line `feature-dev` attribution (REQ-004/DEC-attribution; mirror
   the obra/superpowers credit in `speccy-brainstorm`).
2. Add the host wrappers `resources/agents/.claude/agents/reviewer-correctness.md.tmpl`
   and `resources/agents/.codex/agents/reviewer-correctness.toml.tmpl`,
   mirroring an existing `reviewer-*` wrapper, using speccy's per-host
   model conventions (`model: opus[1m]` + an `effort` value for Claude;
   `model = "gpt-5.5"` + `model_reasoning_effort` for Codex) — never
   `sonnet`.
3. Insert `"correctness"` into `speccy-core/src/personas.rs` `ALL` at
   index 4 (after `style`, before `architecture`) and widen
   `speccy-core/src/next.rs::default_personas()` from `ALL.get(..4)` to
   `ALL.get(..5)`. Update the doc comments that describe the
   "four-persona prefix" / "six personas" to the new five-default /
   seven-total shape.
4. Update the dependent tests to the new shape: the inline tests in
   `personas.rs` (`all_contains_exactly_six_names_in_declared_order`,
   `default_personas_is_prefix_of_all`), `next.rs`
   (`default_personas_is_the_first_four_of_all`),
   `speccy-core/tests/personas.rs`
   (`registry_default_personas_is_first_four_prefix`), and the persona
   lists in `speccy-cli/tests/skill_packs.rs`
   (`DEFAULT_REVIEWER_PERSONAS`, `REVIEWER_PERSONAS`,
   `NON_TESTS_REVIEWER_PERSONAS`). Rename stale "first four"/"six"
   identifiers to reflect reality rather than leaving misleading names.
5. Add `reviewer-correctness` to the default dispatch list in
   `resources/modules/skills/partials/review-fanout.md` (both the Claude
   Code and Codex host branches), then run `just reeject`.

`parse/journal_xml` validates `<review persona="…">` against
`personas::ALL`, so step 3 is also what makes a `correctness` verdict
block parseable (REQ-001 depends on it).

<task-scenarios>
Given the repo at HEAD after this task, when `just reeject` runs and
`.claude/agents/reviewer-correctness.md` and
`.codex/agents/reviewer-correctness.toml` are read, then both exist with
all `{% ... %}` includes expanded and no `<...>` placeholder substrings.

Given the rendered `reviewer-correctness` body, when a structural test
inspects it, then the four deferral targets (security, style, business,
tests) are each present as out-of-scope and the literal confidence
threshold `80` is present (CHK-001, CHK-002).

Given the binary at HEAD, when `next.rs::default_personas()` is
evaluated by its unit test, then it equals
`["business","tests","security","style","correctness"]`, and the
rendered `.claude/skills/speccy-review/SKILL.md` dispatches
`reviewer-correctness` alongside the original four (CHK-007).

Given a `<review persona="correctness" verdict="pass" model="x">` block,
when the journal parser reads it, then the persona name validates as
registry-valid rather than erroring (CHK-014).

Suggested files: `resources/modules/personas/reviewer-correctness.md`,
`resources/agents/.claude/agents/reviewer-correctness.md.tmpl`,
`resources/agents/.codex/agents/reviewer-correctness.toml.tmpl`,
`speccy-core/src/personas.rs`, `speccy-core/src/next.rs`,
`speccy-core/tests/personas.rs`, `speccy-cli/tests/skill_packs.rs`,
`resources/modules/skills/partials/review-fanout.md`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-002">
## Add the `plan-explorer` persona body and host wrappers (read-only, advisory)

Write `resources/modules/personas/plan-explorer.md`: a read-only
codebase-analysis agent that traces feature implementations from entry
points through abstraction layers and emits a grounding report (entry
points and core files, execution/call flows with data transformations,
architecture layers and patterns, dependency map), each with
`file:line` references. The body uses an advisory, non-verdict contract:
it returns a report, never a `pass`/`blocking` verdict, never writes
`TASKS.md` or flips task state, and does NOT include the `<review>`
verdict-contract snippets. Carry a one-line `feature-dev` attribution.

Add the host wrappers
`resources/agents/.claude/agents/plan-explorer.md.tmpl` and
`resources/agents/.codex/agents/plan-explorer.toml.tmpl` using speccy's
per-host model conventions (not `sonnet`). `plan-explorer` is NOT a
reviewer persona, so it is NOT added to `personas::ALL`; if an agent
enumeration test (e.g. `init_phase_agents.rs`) asserts the exhaustive
wrapper set, extend it. Run `just reeject`.

<task-scenarios>
Given the repo at HEAD after this task, when `just reeject` runs and the
rendered `plan-explorer` wrappers for both hosts are read, then both
exist with includes expanded and a structural test asserts the body
contains no `<review` verdict-contract marker (advisory, non-verdict
contract) (CHK-003).

Suggested files: `resources/modules/personas/plan-explorer.md`,
`resources/agents/.claude/agents/plan-explorer.md.tmpl`,
`resources/agents/.codex/agents/plan-explorer.toml.tmpl`,
`speccy-cli/tests/init_phase_agents.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-003">
## Add the `plan-architect` persona body and host wrappers (read-only, advisory)

Write `resources/modules/personas/plan-architect.md`: a read-only
architecture-design agent that analyzes existing codebase patterns and
emits an implementation blueprint (component design, file map of files
to create/modify, data flow, and a build sequence rendered as an
ordered checklist whose items are agent-sized — each item ≈ one Speccy
task). Same advisory, non-verdict contract as `plan-explorer` (no
verdict, no state mutation, no `<review>` snippets). Carry a one-line
`feature-dev` attribution.

Add the host wrappers
`resources/agents/.claude/agents/plan-architect.md.tmpl` and
`resources/agents/.codex/agents/plan-architect.toml.tmpl` with speccy's
per-host model conventions (not `sonnet`). Not a reviewer persona — not
added to `personas::ALL`; extend any exhaustive agent-enumeration test.
Run `just reeject`.

<task-scenarios>
Given the repo at HEAD after this task, when `just reeject` runs and the
rendered `plan-architect` wrappers for both hosts are read, then both
exist with includes expanded and a structural test asserts the body
contains no `<review` verdict-contract marker, and the body specifies
that build-sequence items are agent-sized (CHK-004).

Suggested files: `resources/modules/personas/plan-architect.md`,
`resources/agents/.claude/agents/plan-architect.md.tmpl`,
`resources/agents/.codex/agents/plan-architect.toml.tmpl`,
`speccy-cli/tests/init_phase_agents.rs`
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-004">
## Verify packaging: reeject idempotency, model conventions, and attribution across the three new personas

The three new personas were authored in T-001/T-002/T-003. This task
proves the aggregate packaging invariants. Add a structural test (in
the crate that owns wrapper rendering, e.g. alongside
`speccy-cli/tests/skill_packs.rs`) that, for `reviewer-correctness`,
`plan-explorer`, and `plan-architect`, asserts: each Claude wrapper's
`model` is `opus[1m]`, each Codex wrapper's `model` is `gpt-5.5`, no
wrapper declares `sonnet`, and each persona body contains a
`feature-dev` attribution line. Confirm `just reeject` is idempotent
(no working-tree diff on a second run).

<task-scenarios>
Given the repo at HEAD, when `just reeject` runs twice and
`git diff --exit-code` is checked after the second run, then the working
tree is clean — proving source and ejected wrappers are consistent and
no ejected file was hand-edited (CHK-005).

Given the rendered wrappers for the three new personas, when a
structural test reads their frontmatter, then each Claude `model` is
`opus[1m]`, each Codex `model` is `gpt-5.5`, none is `sonnet`, and each
body carries a `feature-dev` attribution (CHK-006).

Suggested files: `speccy-cli/tests/skill_packs.rs`, `justfile`
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-006 REQ-007">
## Wire the plan-time subagents into their host skills

Update the skill bodies under `resources/modules/skills/` so the
plan-time agents are invoked unconditionally:

- `speccy-brainstorm.md` and `speccy-plan.md` invoke `plan-explorer` in
  their codebase-context step; the routing prose directs the explorer's
  ephemeral report into existing SPEC.md sections (Summary prose and
  `<requirement>` grounding) and explicitly does NOT create a new report
  artifact file.
- `speccy-decompose.md` invokes `plan-architect` before authoring
  `TASKS.md`, consumes its build-sequence checklist as the CANDIDATE
  task list while retaining final `<task>` authorship (merge/split/
  reorder/number), and directs promoting load-bearing blueprint
  decisions into SPEC.md `### Decisions` (DEC-NNN) blocks.

Run `just reeject` so the ejected skills reflect the wiring. Depends on
T-002/T-003 (the agents must exist to be invoked).

<task-scenarios>
Given the ejected `speccy-brainstorm` and `speccy-plan` skill bodies,
when a structural test reads them, then each references invoking the
`plan-explorer` subagent and neither references creating a new `*.md`
report artifact outside the SPEC.md routing targets (CHK-008).

Given the ejected `speccy-decompose` skill body, when a structural test
reads it, then it references invoking `plan-architect`, names the
build-sequence checklist as candidate tasks, and references promoting
decisions into `### Decisions` (CHK-009).

Suggested files: `resources/modules/skills/speccy-brainstorm.md`,
`resources/modules/skills/speccy-plan.md`,
`resources/modules/skills/speccy-decompose.md`
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-008 REQ-009">
## Harden read-only agents' tool grants and verify Codex parity

Add an explicit read-only `tools:` grant to every read-only agent
wrapper — `plan-explorer`, `plan-architect`, `reviewer-correctness`, the
six existing `reviewer-*` (business, tests, security, style,
architecture, docs), and `vet-reviewer` — granting `Read`, `Grep`,
`Glob`, `LS`, `Bash` (required for `git diff`), and `WebFetch`, and
excluding `Edit`, `Write`, and `NotebookEdit`. Leave the five writer
agents (`speccy-work`, `speccy-decompose`, `speccy-ship`,
`vet-implementer`, `vet-simplifier`) with full grants. Run
`just reeject`.

Determine whether Codex honors a per-subagent tool restriction in its
`.toml` agent definition (the mechanism Claude Code provides via the
`tools:` frontmatter field). Record the finding — supported,
unsupported, or supported-with-different-syntax — as a durable note
(SPEC.md `### Decisions`/`## Notes` or an `AGENTS.md` line). If Codex
does not support it, state the limitation and that the Codex read-only
posture remains prose-enforced; do not silently assume parity.

<task-scenarios>
Given the repo at HEAD after this task, when a structural test reads the
rendered Claude wrappers for the ten read-only agents, then each
declares a `tools:` field that includes `Read` and excludes `Edit` and
`Write` (CHK-010).

Given the same rendered wrappers, when the test reads the five writer
agents' wrappers, then none has been narrowed to the read-only set —
writers retain `Edit`/`Write` capability (CHK-011).

Given the durable note location, when it is read, then it states
explicitly whether Codex honors per-subagent tool restriction (CHK-012).

Suggested files: `resources/agents/.claude/agents/reviewer-*.md.tmpl`,
`resources/agents/.claude/agents/vet-reviewer.md.tmpl`,
`resources/agents/.claude/agents/plan-explorer.md.tmpl`,
`resources/agents/.claude/agents/plan-architect.md.tmpl`,
`resources/agents/.codex/agents/*.toml.tmpl`, `AGENTS.md`,
`speccy-cli/tests/skill_packs.rs`
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-010">
## Add the `can we brainstorm` trigger phrase to `speccy-brainstorm`

Add the phrase `can we brainstorm` to the `speccy-brainstorm` skill
description's trigger-phrase list in the source under `resources/`
(the skill description frontmatter for both hosts), so that after
`just reeject` the rendered skill description for both hosts contains
the phrase alongside the existing triggers. Confirm the source location
of the description (skill wrapper frontmatter vs module body) before
editing.

<task-scenarios>
Given the repo at HEAD after this task, when `just reeject` runs and the
rendered `speccy-brainstorm` skill description frontmatter is read for
both hosts, then a structural test confirms the `description` field
contains the substring `can we brainstorm` (CHK-013).

Suggested files:
`resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`,
`resources/modules/skills/speccy-brainstorm.md`,
`speccy-cli/tests/skill_packs.rs`
</task-scenarios>
</task>
