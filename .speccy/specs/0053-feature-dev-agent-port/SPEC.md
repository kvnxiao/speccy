---
id: SPEC-0053
slug: feature-dev-agent-port
title: Port feature-dev agents into speccy — correctness reviewer, plan-explorer, plan-architect, read-only tool hardening, brainstorm trigger phrase
status: implemented
created: 2026-05-29
supersedes: []
---

# SPEC-0053: Port feature-dev agents into speccy — correctness reviewer, plan-explorer, plan-architect, read-only tool hardening, brainstorm trigger phrase

## Summary

Anthropic's `feature-dev` plugin ships three Claude-Code-only subagents
that span a mini development loop: `code-explorer` (traces execution
paths and maps dependencies before building), `code-architect` (designs
a feature blueprint from existing codebase patterns), and
`code-reviewer` (bug/logic review with a confidence-≥80 reporting
filter). Speccy already covers most of that surface — plan/decompose own
design, and the `business`/`tests`/`security`/`style` fan-out owns
review — but two gaps remain. First, none of speccy's four default
reviewers is a plain **correctness** reviewer: a logic bug that is not a
SPEC-intent violation, not a vulnerability, not a style problem, and not
a coverage gap falls straight through the fan-out. Second, plan-time
work has no dedicated codebase-grounding or architecture-design subagent
to lean on.

This SPEC ports the three agents into speccy as **host-neutral persona
modules wired into existing phases**, with zero new commands and zero
new artifact files. `code-reviewer` becomes a narrowly-scoped
`reviewer-correctness` persona that joins the default fan-out as a fifth
always-run reviewer. `code-explorer` becomes `plan-explorer`, invoked by
`speccy-brainstorm`/`speccy-plan` for ephemeral codebase grounding.
`code-architect` becomes `plan-architect`, invoked by `speccy-decompose`
to produce a blueprint whose build sequence seeds the candidate task
list. Because the ported agents are Claude-Code-only, integration means
porting their *technique* into `resources/modules/personas/` bodies with
per-host wrappers — never depending on the upstream plugin existing.

The SPEC also hardens an adjacent latent risk surfaced during planning:
speccy's read-only reviewer subagents declare no `tools:` field, so
under Claude Code they inherit the full toolset (including `Edit`/`Write`),
making the "reviewers never mutate the tree" invariant prose-enforced
only. All read-only agents gain explicit read-only tool grants. Finally,
the phrase "can we brainstorm" is added to the `speccy-brainstorm`
trigger list (it failed to auto-fire during this SPEC's own brainstorm).

## Goals

<goals>
- After `just reeject`, three new personas — `reviewer-correctness`,
  `plan-explorer`, `plan-architect` — are registered as both Claude Code
  (`.claude/agents/*.md`) and Codex (`.codex/agents/*.toml`) subagents,
  each rendered from a single host-neutral body under
  `resources/modules/personas/`.
- The default review fan-out dispatches five reviewers
  (`business`, `tests`, `security`, `style`, `correctness`) where it
  dispatched four, driven by `speccy-core`'s `personas::ALL` registry
  (with `review-fanout.md` kept in sync as documentation).
- Every read-only agent's rendered wrapper grants a read-only toolset
  (no `Edit`/`Write`/`NotebookEdit`); every writer agent's wrapper
  retains full grants.
- The rendered `speccy-brainstorm` skill description contains the
  trigger phrase `can we brainstorm` for both hosts.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo +nightly fmt --all --check`,
  `cargo deny check`, and `just reeject` (idempotent: no working-tree
  diff after running) all pass at HEAD.
</goals>

## Non-goals

<non-goals>
- No runtime dependency on the upstream `feature-dev` plugin and no
  verbatim copy of its agent text. Content is ported as technique,
  rewritten host-neutral, with an attribution line.
- No new `/speccy-*` commands. The command surface does not grow; in
  particular no `/speccy-explore` or `/speccy-architect`.
- No new persisted artifact file types. `plan-explorer` and
  `plan-architect` outputs feed existing artifacts (ephemeral chat,
  `TASKS.md`, SPEC.md `### Decisions`); no `EXPLORE.md` / `BLUEPRINT.md`.
- No confidence-≥80 reporting gate applied to the existing
  `business` / `tests` / `security` / `style` / `architecture` / `docs`
  personas. The gate is `reviewer-correctness`-only.
- No gating power for the plan-time agents. `plan-explorer` and
  `plan-architect` are advisory: they emit reports, never a
  `pass`/`blocking` verdict, and never mutate task state.
- No new fan-out *configuration* machinery — no per-project persona
  config file, no `--strict`, no enforcement toggle (core principle #1).
  `correctness` extends the existing `speccy-core` `personas::ALL`
  registry that already drives the fan-out; we widen that single source,
  not invent a parallel one.
</non-goals>

## User Stories

<user-stories>
- As speccy's review loop, I want a dedicated correctness reviewer so
  that plain logic bugs — ones that are not SPEC-intent, security,
  style, or coverage issues — stop falling through the default fan-out.
- As an agent planning a brownfield slice, I want a codebase-grounding
  explorer and an architecture blueprint so that plan and decompose are
  anchored in real code patterns rather than guesses.
- As a speccy maintainer, I want read-only reviewers that cannot mutate
  the working tree, so the "orchestrator owns all writes" invariant is
  enforced by tool grants rather than by prose alone.
- As a user in a speccy repo, I want "can we brainstorm" to route to
  `/speccy-brainstorm` so the framing-first flow triggers on natural
  phrasing.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `reviewer-correctness` persona body — correctness-only scope, deferral, confidence filter

A new host-neutral persona body
`resources/modules/personas/reviewer-correctness.md` defines an
adversarial reviewer scoped to correctness/logic defects only: logic
and control-flow errors, null/`Option`/`Result` mishandling, off-by-one
and boundary conditions, non-security race conditions and deadlocks, and
resource leaks. The body instructs the persona to defer out-of-lane
findings to the owning specialist (vulnerabilities → security,
conventions → style, SPEC intent → business, coverage → tests) rather
than report them, and to report only findings at confidence ≥80,
grouped by severity (Critical / Important). The body returns speccy's
existing `<review>` verdict contract.

<done-when>
- The persona body exists and includes the shared review-contract
  snippets (`verdict_return_contract.md`, `inline_note_format.md`,
  `diff_fetch_command.md`, `no_tasks_md_writes.md`) used by the other
  `reviewer-*` personas.
- The body names the four deferral targets (security, style, business,
  tests) as out of its own scope.
- The body states the confidence-≥80 reporting threshold and
  Critical/Important severity grouping.
</done-when>

<behavior>
- Given a diff containing a null-dereference logic bug that is not a
  security or SPEC-intent issue, when `reviewer-correctness` reviews it,
  then it returns a `blocking` `<review>` naming the defect with
  file:line evidence.
- Given a diff whose only issue is a naming-convention violation, when
  `reviewer-correctness` reviews it, then it does not report the issue
  (deferred to style) and returns `pass` on its own axis.
</behavior>

<scenario id="CHK-001">
Given the repo at HEAD after this SPEC lands, when `just reeject` runs
and `.claude/agents/reviewer-correctness.md` and
`.codex/agents/reviewer-correctness.toml` are read, then both exist with
the persona body's `{% include %}` directives fully expanded (no
unresolved `{% ... %}` or `<...>` placeholder substrings remain).
</scenario>

<scenario id="CHK-002">
Given the rendered `reviewer-correctness` body, when a structural test
inspects it, then it asserts (a) the four deferral targets security /
style / business / tests are each present as out-of-scope, and (b) the
literal confidence threshold value `80` is present as the reporting
gate — gating the regression where the scope or filter is silently
dropped during a future edit.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `plan-explorer` persona body — read-only codebase-grounding report, advisory contract

A new host-neutral persona body
`resources/modules/personas/plan-explorer.md` defines a read-only
codebase-analysis agent that traces feature implementations from entry
points through abstraction layers, and emits a grounding report:
entry points and core files, execution/call flows with data
transformations, architecture layers and patterns, and a dependency
map, each with `file:line` references. The body uses an advisory,
non-verdict output contract: it returns a report, never a
`pass`/`blocking` verdict, and never writes `TASKS.md` or flips task
state.

<done-when>
- The persona body exists and produces a report covering entry points,
  execution flows, layers/patterns, and dependencies with `file:line`
  references.
- The body explicitly states it emits no `pass`/`blocking` verdict and
  performs no state mutation (advisory only).
- The body does not include the `<review>` verdict-contract snippets.
</done-when>

<behavior>
- Given a request to ground a slice that touches an existing module,
  when `plan-explorer` runs, then it returns a report naming the
  relevant entry points and dependencies and returns no verdict element.
</behavior>

<scenario id="CHK-003">
Given the repo at HEAD after this SPEC lands, when `just reeject` runs
and the rendered `plan-explorer` wrappers for both hosts are read, then
both exist with includes expanded, and a structural test asserts the
body contains no `<review` verdict-contract marker (confirming the
advisory, non-verdict contract).
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `plan-architect` persona body — blueprint with agent-sized build-sequence checklist, advisory contract

A new host-neutral persona body
`resources/modules/personas/plan-architect.md` defines a read-only
architecture-design agent that analyzes existing codebase patterns and
emits an implementation blueprint: component design, file map (files to
create/modify), data flow, and a **build sequence rendered as an
ordered checklist whose items are agent-sized** (each item is a
plausible single Speccy task). The body uses the same advisory,
non-verdict contract as `plan-explorer`.

<done-when>
- The persona body exists and produces a blueprint covering component
  design, file map, data flow, and a build-sequence checklist.
- The body specifies that build-sequence items are agent-sized (one
  item ≈ one task), so the checklist is directly consumable as
  candidate tasks.
- The body states it emits no verdict and performs no state mutation.
</done-when>

<behavior>
- Given a SPEC ready for decomposition, when `plan-architect` runs,
  then it returns a blueprint whose build sequence is an ordered
  checklist of agent-sized steps and returns no verdict element.
</behavior>

<scenario id="CHK-004">
Given the repo at HEAD after this SPEC lands, when `just reeject` runs
and the rendered `plan-architect` wrappers for both hosts are read, then
both exist with includes expanded, and a structural test asserts the
body contains no `<review` verdict-contract marker.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Three personas packaged host-neutral with per-host wrappers, speccy model conventions, and attribution

Each of the three personas ships as exactly one body under
`resources/modules/personas/` plus a Claude Code wrapper
(`resources/agents/.claude/agents/<name>.md.tmpl`) and a Codex wrapper
(`resources/agents/.codex/agents/<name>.toml.tmpl`) that pull the body
in via `{% include %}`. Wrappers adopt speccy's existing per-host model
conventions (`model: opus[1m]` + an `effort` value for Claude;
`model = "gpt-5.5"` + `model_reasoning_effort` for Codex), not
`feature-dev`'s `model: sonnet`. Each body carries a one-line
attribution crediting `feature-dev` as inspiration, mirroring the
existing obra/superpowers credit in `speccy-brainstorm`.

<done-when>
- All three personas render to both hosts via `just reeject` with no
  manual edits to `.claude/` / `.codex/`.
- Each wrapper declares `model: opus[1m]` (Claude) and
  `model = "gpt-5.5"` (Codex); none declares `sonnet`.
- Each persona body contains an attribution line naming `feature-dev`
  as the source of inspiration.
</done-when>

<behavior>
- Given the source bodies and wrappers, when `just reeject` runs, then
  the working tree has no diff on a second consecutive run (rendering is
  idempotent and the ejected output matches source).
</behavior>

<scenario id="CHK-005">
Given the repo at HEAD, when `just reeject` runs twice and
`git diff --exit-code` is checked after the second run, then the working
tree is clean — proving each new persona's source and ejected wrappers
are consistent and no ejected file was hand-edited.
</scenario>

<scenario id="CHK-006">
Given the rendered wrappers for `reviewer-correctness`, `plan-explorer`,
and `plan-architect`, when a structural test reads their frontmatter,
then each Claude wrapper's `model` is `opus[1m]`, each Codex wrapper's
`model` is `gpt-5.5`, and no wrapper's `model` is `sonnet`.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: `reviewer-correctness` joins the default fan-out via the `speccy-core` persona registry

The default review fan-out is derived in `speccy-core`, not from prose:
`personas::ALL` declares the persona names and
`next.rs::default_personas()` returns the `ALL[..4]` prefix as the
SPEC-0007 default. `correctness` is inserted into `personas::ALL`
immediately after `style` (index 4), and `default_personas()` is widened
from `ALL[..4]` to `ALL[..5]` so the default fan-out dispatches five
reviewers. The dependent registry and derivation tests, the
`review-fanout.md` documentation partial (whose hardcoded
`subagent_type` dispatch list must stay in sync, across both the Claude
Code and Codex host branches), and the persona-name lists in
`speccy-cli/tests/skill_packs.rs` are updated to match. Because
`parse/journal_xml` validates `<review persona="…">` names against
`personas::ALL`, this same registry entry is what makes a `correctness`
verdict block (REQ-001) parseable rather than rejected as an unknown
persona.

<done-when>
- `personas::ALL` contains `correctness` at index 4 (after `style`,
  before `architecture`); the total persona count is seven.
- `next.rs::default_personas()` returns
  `["business", "tests", "security", "style", "correctness"]`.
- The registry/derivation tests (`personas.rs` inline tests,
  `tests/personas.rs`) and the persona-name lists in `skill_packs.rs`
  are updated to the default-of-five / total-of-seven shape and pass.
- `review-fanout.md` documents `correctness` in the default fan-out and
  both host dispatch branches name `reviewer-correctness`.
- A `<review persona="correctness">` journal block validates against the
  registry rather than being rejected.
</done-when>

<behavior>
- Given the updated registry, when `speccy next` resolves the review
  fan-out, then it returns five default personas including `correctness`.
- Given a journal block
  `<review persona="correctness" verdict="pass" model="…">`, when the
  journal parser reads it, then the persona name validates against
  `personas::ALL`.
</behavior>

<scenario id="CHK-007">
Given the repo at HEAD after this SPEC lands, when
`next.rs::default_personas()` is evaluated by its unit test, then it
equals `["business", "tests", "security", "style", "correctness"]` —
gating the regression where the default fan-out silently drops back to
four. The rendered `.claude/skills/speccy-review/SKILL.md` likewise
dispatches `reviewer-correctness` alongside the original four
`subagent_type` references.
</scenario>

<scenario id="CHK-014">
Given the repo at HEAD after this SPEC lands, when the journal parser
reads a `<review persona="correctness" verdict="pass" model="x">` block,
then it accepts the persona name as registry-valid rather than emitting
an unknown-persona error.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: `speccy-brainstorm` and `speccy-plan` invoke `plan-explorer`; its output feeds SPEC.md

The `speccy-brainstorm` and `speccy-plan` skill bodies under
`resources/modules/skills/` are updated to invoke `plan-explorer`
unconditionally as part of their codebase-context step. The skill prose
routes the explorer's report into the existing SPEC.md sections
(Summary prose and `<requirement>` grounding); the report itself is
ephemeral and is not persisted to a new file.

<done-when>
- `speccy-brainstorm.md` invokes `plan-explorer` in its
  explore-project-context step.
- `speccy-plan.md` invokes `plan-explorer` before/while drafting
  SPEC.md.
- Neither skill writes the explorer report to a new artifact file; the
  routing prose directs it into existing SPEC.md sections.
</done-when>

<behavior>
- Given an invocation of `speccy-plan` for a slice touching existing
  code, when the skill runs, then it spawns `plan-explorer` and folds
  the grounding into the SPEC.md Summary and requirements, writing no
  standalone report file.
</behavior>

<scenario id="CHK-008">
Given the repo at HEAD after this SPEC lands, when the ejected
`speccy-brainstorm` and `speccy-plan` skill bodies are read, then a
structural test confirms each references invoking the `plan-explorer`
subagent, and neither references creating a new `*.md` report artifact
outside the SPEC.md routing targets.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: `speccy-decompose` invokes `plan-architect`; build-sequence checklist seeds candidate tasks

The `speccy-decompose` skill body is updated to invoke `plan-architect`
unconditionally and consume its build-sequence checklist as the
**candidate** task list, while `speccy-decompose` retains final
authorship of the `<task>` blocks (it may merge, split, reorder, and
number them). The skill prose directs that load-bearing design choices
from the blueprint are promoted into SPEC.md `### Decisions` (DEC-NNN)
blocks rather than buried in task prose.

<done-when>
- `speccy-decompose.md` invokes `plan-architect` before authoring
  `TASKS.md`.
- The skill prose states the build-sequence checklist is the candidate
  task list and that decompose retains final `<task>` authorship.
- The skill prose directs promoting load-bearing blueprint decisions
  into SPEC.md `### Decisions` blocks.
</done-when>

<behavior>
- Given an invocation of `speccy-decompose` on a SPEC, when the skill
  runs, then it spawns `plan-architect`, treats the returned checklist
  as candidate tasks, and authors `TASKS.md` itself.
</behavior>

<scenario id="CHK-009">
Given the repo at HEAD after this SPEC lands, when the ejected
`speccy-decompose` skill body is read, then a structural test confirms
it references invoking `plan-architect`, names the build-sequence
checklist as candidate tasks, and references promoting decisions into
`### Decisions`.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Read-only agents get explicit read-only tool grants; writers retain full grants

Every read-only agent wrapper — `plan-explorer`, `plan-architect`,
`reviewer-correctness`, the six existing `reviewer-*` (business, tests,
security, style, architecture, docs), and `vet-reviewer` — declares an
explicit read-only `tools:` grant: `Read`, `Grep`, `Glob`, `LS`,
`Bash` (required for `git diff`), and `WebFetch`, and excludes `Edit`,
`Write`, and `NotebookEdit`. Writer agents — `speccy-work`,
`speccy-decompose`, `speccy-ship`, `vet-implementer`, `vet-simplifier` —
retain their full tool grants (unrestricted).

<done-when>
- All ten read-only agent wrappers declare a `tools:` grant excluding
  `Edit`/`Write`/`NotebookEdit` and including `Read`/`Grep`/`Glob`/`LS`/
  `Bash`/`WebFetch`.
- The five writer agent wrappers retain full (unrestricted) tool access.
- `just reeject` renders the grants idempotently with no working-tree
  diff.
</done-when>

<behavior>
- Given the rendered read-only wrappers, when their frontmatter is
  inspected, then none grants `Edit` or `Write`.
- Given the rendered writer wrappers, when their frontmatter is
  inspected, then their tool access is not narrowed by this change.
</behavior>

<scenario id="CHK-010">
Given the repo at HEAD after this SPEC lands, when a structural test
reads the rendered Claude wrappers for the ten read-only agents, then
each declares a `tools:` field that includes `Read` and excludes `Edit`
and `Write`.
</scenario>

<scenario id="CHK-011">
Given the same rendered wrappers, when the test reads the five writer
agents' wrappers, then none has had its tool access narrowed to the
read-only set (writers retain `Edit`/`Write` capability) — gating an
over-broad application of the read-only grant.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: Codex per-subagent tool-restriction support is verified, not assumed

Before relying on the Codex side of REQ-008, the implementation
determines whether Codex honors a per-subagent tool restriction in its
`.toml` agent definition (the mechanism Claude Code provides via the
`tools:` frontmatter field). The determination — supported, unsupported,
or supported-with-different-syntax — is recorded as a durable note (a
SPEC.md `### Decisions`/`## Notes` entry or an `AGENTS.md` line), so the
Codex read-only posture is not silently assumed to have Claude Code
parity.

<done-when>
- A documented determination of Codex's per-subagent tool-restriction
  support exists in a durable location (SPEC.md or `AGENTS.md`).
- If Codex does not support the restriction, the note states the
  limitation and that the Codex read-only posture remains
  prose-enforced.
</done-when>

<behavior>
- Given the verification step, when it completes, then a durable note
  records the Codex tool-restriction finding rather than leaving parity
  assumed.
</behavior>

<scenario id="CHK-012">
Given the repo at HEAD after this SPEC lands, when the durable note
location is read, then it contains an explicit statement of whether
Codex honors per-subagent tool restriction — gating the regression of
shipping Codex read-only grants on an unverified parity assumption.
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: `can we brainstorm` added to the `speccy-brainstorm` trigger list

The phrase `can we brainstorm` is added to the `speccy-brainstorm` skill
description's trigger-phrase list in the source wrapper(s) under
`resources/agents/`, so that after `just reeject` the rendered skill
description for both hosts contains the phrase alongside the existing
triggers (`help me brainstorm`, `let's brainstorm`, etc.).

<done-when>
- The source skill description for `speccy-brainstorm` lists
  `can we brainstorm` among its trigger phrases.
- After `just reeject`, the rendered description for both hosts contains
  the phrase.
</done-when>

<behavior>
- Given the updated description, when the ejected `speccy-brainstorm`
  skill frontmatter is rendered, then its `description` field contains
  the substring `can we brainstorm`.
</behavior>

<scenario id="CHK-013">
Given the repo at HEAD after this SPEC lands, when `just reeject` runs
and the rendered `speccy-brainstorm` skill description frontmatter is
read for both hosts, then a structural test confirms the `description`
field contains `can we brainstorm`.
</scenario>

</requirement>

## Assumptions

<assumptions>
- Ported personas adopt speccy's per-host model/effort conventions
  (`opus[1m]` for Claude, `gpt-5.5` for Codex), not `feature-dev`'s
  hard-coded `sonnet`.
- `plan-explorer` and `plan-architect` are generative/advisory: they
  emit reports, never a `pass`/`blocking` verdict, and never mutate task
  state. Only `reviewer-correctness` uses the verdict contract.
- The correctness persona's narrow scope plus the confidence-≥80 filter
  is what justifies it as a fifth *default* reviewer (low marginal noise
  alongside the four specialists).
- Skill activation is heuristic model-judgment over the description, not
  a deterministic string match; adding `can we brainstorm` raises
  activation odds but cannot guarantee the skill always fires.
- The read-only `tools:` restriction is defense-in-depth, not a hard
  sandbox: `Bash` is retained for `git diff`, so a determined prompt
  could still shell out. Removing `Edit`/`Write` removes the easy
  mutation path and makes intent explicit.
- The default review fan-out is derived in `speccy-core` from
  `personas::ALL` (`next.rs::default_personas()` = `ALL[..4]`);
  `review-fanout.md` is derived documentation, not the source of truth.
  REQ-005 therefore edits the registry and its derivation, and the same
  registry backs `parse/journal_xml` persona-name validation (so the
  registry entry is a prerequisite for REQ-001's verdict block to
  parse).
</assumptions>

## Decisions

<decision id="DEC-001">
The confidence-≥80 reporting gate is applied to `reviewer-correctness`
only, not to the other personas. It is a false-positive suppressor that
earns its keep in high-FP domains (bug hunting). It is the wrong
direction for security (where suspected-but-unconfirmed vulnerabilities
should still surface; false negatives are costly) and meaningless for
the binary domains (style is deterministic lint; business and tests are
binary against SPEC artifacts).
</decision>

<decision id="DEC-002">
`plan-explorer` and `plan-architect` outputs feed existing artifacts —
ephemeral planning chat, `TASKS.md`, and SPEC.md `### Decisions` — and
introduce zero new persisted artifact file types. This honors the
stay-small principle (#5/#6): no `EXPLORE.md` / `BLUEPRINT.md` files.
</decision>

<decision id="DEC-003">
The `plan-architect` → `speccy-decompose` handoff uses a light
convention (the build-sequence checklist is the candidate task list;
decompose retains final `<task>` authorship) rather than a rigid
blueprint→task schema. This gives a traceable, auditable "build-step →
task" mapping — mirroring requirement→check — without a brittle contract
that would fight decompose's existing ownership of task granularity.
</decision>

<decision id="DEC-004">
Integration is a harness-agnostic port: personas live as host-neutral
bodies under `resources/modules/personas/` with per-host wrappers, with
no runtime dependency on the upstream `feature-dev` plugin. Vendoring or
delegating to the Claude-Code-only plugin was rejected because it
violates core principle #2 and has no Codex equivalent.
</decision>

<decision id="DEC-005">
The port is whole-loop (all three agents), not correctness-only. The
explorer and architect fill genuine plan-time grounding gaps; the
narrower correctness-only framing was rejected because it drops two
thirds of the pack's value.
</decision>

<decision id="DEC-006">
`correctness` is inserted into `speccy-core`'s `personas::ALL` at index
4 and the default derivation widens from `ALL[..4]` to `ALL[..5]`,
extending SPEC-0007 DEC-002's four-persona default. The registry — not
`review-fanout.md` — is the single source for both the `speccy next`
review fan-out and `parse/journal_xml` persona-name validation; the
prose partial is kept in sync as documentation. Recorded after a
decompose-time discovery that the fan-out is registry-driven, correcting
an earlier planning assumption that it was prose-only.
</decision>

## Notes

This SPEC bootstraps the very agents that future `speccy-plan` /
`speccy-decompose` runs will lean on, so `plan-explorer` /
`plan-architect` did not yet exist to ground or design *this* spec; the
brainstorm grounded the codebase manually instead. This is a one-time
bootstrap condition, not a recurring gap.

Rejected framing — new `/speccy-explore` and `/speccy-architect`
commands: more discoverable, but grows the command surface against
stay-small principle #5. The invoked-subagent wiring delivers the same
value with zero new commands.

The read-only tool hardening (REQ-008) is adjacent to the feature-dev
port rather than strictly part of it, but was bundled deliberately: the
new read-only agents establish the read-only-grant precedent, and
shipping them tightened while leaving the six existing reviewers
wide-open would be inconsistent.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-05-29 | kevin | Initial SPEC: port feature-dev agents (reviewer-correctness, plan-explorer, plan-architect) as host-neutral personas, add read-only tool hardening, add `can we brainstorm` trigger phrase. |
| 2026-05-29 | kevin | Correction (pre-decompose): fan-out is `speccy-core` registry-driven, not prose. Rewrote REQ-005 to edit `personas::ALL` + `default_personas()` derivation + dependent tests; fixed assumption #6, the fan-out non-goal, and a goal bullet; added DEC-006 and CHK-014. |
</changelog>
