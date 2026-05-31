---
id: SPEC-0054
slug: implementer-reuse-survey
title: Implementer pre-implementation reuse survey, forked reuse guidance, and `speccy-work` opus/high repin
status: in-progress
created: 2026-05-31
supersedes: []
---

# SPEC-0054: Implementer pre-implementation reuse survey, forked reuse guidance, and `speccy-work` opus/high repin

## Summary

Dogfooding Speccy on the patina repo surfaced a recurring tax in the
implementer/review loop. In `patina/.speccy/specs/0002-patina-complete-cli/`
every task in a run of six (T-004 through T-009) needed at least one
retry round, and two of those rounds were the **style** persona
catching reuse drift: byte-for-byte duplicated helpers
(`current_timestamp`, `resolve_home`) and a constant (`TEMPLATE_SUFFIX`)
that already existed elsewhere and should have been called, not copied.

The diagnosis matters. The "Reuse over reinvent" rule already exists —
SPEC-0052 put it in the shared `convention-checklist.md`, included at
both the implementer self-review and the `reviewer-style` persona.
Same rule, same model tier; the reviewer (`reviewer-style`, sonnet
medium) caught the duplication that the implementer (opus) missed in
its own step-7 self-review. The gap is not the rule and not model
capability — it is that the implementer's self-review re-reads *its own
diff* in the context that already decided to write the duplicate, and
the rule is phrased aspirationally ("check whether one already
exists") rather than as an action taken *before* the code is written.

This SPEC front-loads reuse into the implementer as a **bounded
pre-implementation survey**: before writing code, the implementer maps
the task-relevant area of the codebase and classifies the existing code
it finds into reuse-as-is / extend / write-fresh, so reuse becomes a
design input rather than a post-hoc cleanup. The survey is recorded in
the journal so it is auditable. The reuse guidance is forked from the
single shared checklist into two lifecycle-specific variants — an
implementer "survey-and-build" variant and a `reviewer-style`
"adversarially verify-and-hunt" variant — so the producer's and the
reviewer's reuse framings can diverge as they should. Finally, the
Claude Code `speccy-work` implementer is repinned from `opus[1m]` /
`low` to `opus[1m]` / `high`, because the extend-vs-fresh judgement the
survey introduces benefits from the reasoning budget that the prior
cost-minimised pin withheld.

This is entirely a prompt/skill-layer and agent-frontmatter change
under `resources/`; the deterministic Rust CLI gains no logic.

## Goals

<goals>
- The `speccy-work` implementer phase performs a bounded reuse survey
  of the task-relevant area before the implement step, classifying
  relevant existing code into reuse-as-is / extend / write-fresh and
  naming the existing symbols.
- The implementer journal `<implementer>` block carries a free-form
  `Reuse survey` field recording the survey, with round-1-versus-retry
  semantics, and no parser or lint change is made.
- The reuse guidance is forked into an implementer variant (wired into
  the `speccy-work` phase) and a reviewer variant (wired into
  `reviewer-style`); the shared `convention-checklist.md` retains its
  other four items and loses only the reuse bullet.
- The Claude Code `speccy-work` agent is pinned `opus[1m]` / `high`
  across the template, the in-tree dogfood file, and the README, with
  the skill-pack pin tests green and the Codex pin unchanged.
</goals>

## Non-goals

<non-goals>
- No Rust CLI changes. The deterministic core gains no reuse-detection,
  no journal-field lint, and no identity logic; this is entirely
  skill/prompt prose and agent frontmatter under `resources/`.
- No new reviewer persona. Reuse detection stays in `reviewer-style`
  (its forked reviewer variant); a dedicated `reviewer-reuse` persona
  would duplicate a detector that already works.
- No deterministic duplicate-detection script and no generalisation of
  the hygiene gate to carry one. Exact-match dup detection catches only
  the byte-identical tail; the survey targets the broader extendable-code
  case.
- No measurement or instrumentation step. The efficacy judgement is
  qualitative and revisable, following SPEC-0036's precedent.
- No whole-repo reuse scan. The survey is bounded to the task's area;
  reusable code far outside that area is out of scope by design.
- No change to the Codex `speccy-work` pin
  (`resources/agents/.codex/agents/speccy-work.toml.tmpl`); Codex's
  `model_reasoning_effort` axis is a separate judgement.
- No change to the other four shared convention-checklist items, nor to
  the business, tests, security, or correctness reviewer personas.
</non-goals>

## User Stories

<user-stories>
- As a Speccy contributor driving the loop, I want the implementer to
  survey the task's code area for reusable or extendable code before it
  writes anything, so it builds on what exists instead of reinventing it
  and bouncing at review.
- As someone auditing a journal later, I want the implementer's reuse
  decisions recorded, so I can see what existing code was reused,
  extended, or deliberately not used, and why.
- As a reviewer, I want the reuse lens to be adversarial and
  lifecycle-specific, so it can verify the implementer's survey and hunt
  reinvention without inheriting the implementer's build-framing.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Bounded pre-implementation reuse survey in the `speccy-work` phase

The `speccy-work` implementer phase performs a bounded reuse survey of
the task-relevant area of the codebase *before* the implement step,
classifying the relevant existing code into three tiers — reuse-as-is,
extend, write-fresh — and using the result as a design input rather than
a post-hoc check.

<done-when>
- The ejected `speccy-work` phase body contains a reuse-survey step
  positioned after the read-scenarios step and before the implement
  step.
- The step scopes the survey to the task's area (its covered REQs, the
  suggested-files hint, and the immediate module / neighbouring files)
  and explicitly states it is not a whole-repo scan.
- The step defines the three tiers and requires naming the specific
  existing symbol for reuse-as-is and extend, or naming the search that
  came up empty for write-fresh.
- The per-symbol floor — for each new top-level symbol the
  implementation introduces, name the existing thing reused or extended,
  or the search that found nothing — is stated as round-agnostic; the
  full area-map is round-1 only, re-run on a retry round only when a
  reuse-related blocker was raised.
</done-when>

<behavior>
- Given the implementer has read the task scenarios, when it reaches
  implementation, then it first produces the bounded reuse survey and
  lets the survey inform what it writes.
- Given a retry round addressing a non-reuse blocker that introduces no
  new top-level symbol, when the implementer amends the WIP, then it
  does not re-run the full area-map.
</behavior>

<scenario id="CHK-001">
Given the ejected `speccy-work` phase body after `just reeject`,
when a reviewer reads it,
then a bounded reuse-survey step appears after read-scenarios and before
implement, scoped to the task area (not a whole-repo scan), defining the
three tiers and the round-agnostic per-symbol floor versus the
round-1 / reuse-blocker-triggered area-map.
(judgment-only: prose presence and placement confirmed on the diff.)
</scenario>

<scenario id="CHK-002">
Given the edited `resources/`,
when `just reeject` and `cargo test --workspace` run,
then both exit 0 — the phase-body edit renders at its callsite and no
pack-structure test breaks.
(hygiene)
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Implementer journal records the survey in a free-form `Reuse survey` field

The implementer journal `<implementer>` block carries a `Reuse survey`
field recording the survey. The field is free-form prose; no parser or
lint change is made, because the `<implementer>` body is stored verbatim
and validated only on its `date` / `model` / `round` attributes.

<done-when>
- The ejected `journal-implementer` reference adds a `Reuse survey`
  field to the handoff template, recording the mapped areas and the
  per-tier decisions with named symbols.
- The reference states the round semantics: round-1 records the full
  survey; a retry round records either "unchanged — no new symbols, no
  reuse blocker" or the delta, not a full re-survey.
- No change is made to `speccy-core` journal parsing or to any lint
  rule; the `<implementer>` body remains verbatim and the six existing
  body fields stay unenforced convention alongside the new seventh.
</done-when>

<behavior>
- Given a round-1 implementer turn, when the `<implementer>` block is
  appended, then it carries a `Reuse survey` field naming the mapped
  areas and the per-tier decisions.
- Given a retry round that adds no new top-level symbol and addresses a
  non-reuse blocker, when its `<implementer>` block is appended, then
  the `Reuse survey` field records "unchanged" rather than a fresh
  survey.
</behavior>

<scenario id="CHK-003">
Given the ejected `journal-implementer` reference after `just reeject`,
when a reviewer reads it and the diff,
then the `Reuse survey` field is present with its round-1-versus-retry
semantics, and no `speccy-core` parser or lint file is modified in the
diff.
(judgment-only: confirmed on the diff.)
</scenario>

<scenario id="CHK-004">
Given the edited `resources/`,
when `just reeject` and `cargo test --workspace` run,
then both exit 0 — the journal reference renders at its callsite and the
journal-parsing tests, unchanged, still pass.
(hygiene)
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Reuse guidance forked into implementer and reviewer lifecycle variants

The reuse concern is forked out of the single shared
`convention-checklist.md` into two lifecycle-specific modules: an
implementer variant (survey-and-build) included by the `speccy-work`
phase, and a reviewer variant (adversarially verify-and-hunt) included
by `reviewer-style`. The shared checklist keeps its other four items.

<done-when>
- A new implementer-variant reuse module under `resources/modules/`
  carries the survey-and-build framing and is `{% include %}`d by the
  `speccy-work` phase body.
- A new reviewer-variant reuse module carries the adversarial framing —
  verify the `Reuse survey` is present, confirm named existing symbols
  actually exist, and hunt reinvention or near-duplicate code that
  should have been extended — and is `{% include %}`d by
  `reviewer-style`.
- The "Reuse over reinvent" bullet is removed from
  `resources/modules/references/convention-checklist.md`; its other four
  items (match-local-conventions, docs-match-code, no-false-complexity,
  re-apply-hard-rules) remain shared and unchanged.
</done-when>

<behavior>
- Given a change that reinvents an existing helper, when the implementer
  surveys (implementer variant) and when `reviewer-style` later reviews
  (reviewer variant), then each reads its own lifecycle-scoped reuse
  guidance, and a later edit to one variant does not alter the other.
</behavior>

<scenario id="CHK-005">
Given the ejected `speccy-work` phase body and the ejected
`reviewer-style` persona after `just reeject`,
when a reviewer reads both and the shared checklist,
then the phase includes the implementer reuse variant, `reviewer-style`
includes the reviewer reuse variant, and `convention-checklist.md` no
longer carries a reuse bullet while retaining its other four items.
(judgment-only: confirmed on the diff.)
</scenario>

<scenario id="CHK-006">
Given the edited `resources/`,
when `just reeject` and `cargo test --workspace` run,
then both exit 0 — both new includes resolve at render time at their
respective callsites and no pack-structure test breaks.
(hygiene)
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Claude Code `speccy-work` repinned to `opus[1m]` / `high`

The Claude Code `speccy-work` implementer agent is repinned from
`effort: low` to `effort: high` (model `opus[1m]` unchanged), across the
template, the regenerated in-tree dogfood file, and the README, with the
skill-pack pin tests green and the Codex pin untouched.

<done-when>
- `resources/agents/.claude/agents/speccy-work.md.tmpl` declares
  `effort: high` with `model: opus[1m]` unchanged and all other
  frontmatter and the `{% include %}` body reference unchanged.
- `just reeject` regenerates the in-tree `.claude/agents/speccy-work.md`
  to `effort: high`; re-running `just reeject` leaves
  `git status --porcelain` empty (template and in-tree aligned).
- The README pin-assignment table row for `speccy-work` shows
  `effort: high` in the Claude Code column, with the Codex column and
  the version-lock override example left consistent.
- The Codex template
  `resources/agents/.codex/agents/speccy-work.toml.tmpl` is unchanged.
</done-when>

<behavior>
- Given the bumped template, when `just reeject` runs, then the in-tree
  `speccy-work` agent file declares `effort: high`, and a subsequent
  self-recording implementer turn records the `/high` effort suffix
  sourced from its own definition file per SPEC-0052 REQ-005.
</behavior>

<scenario id="CHK-007">
Given the template edited to `effort: high`,
when `just reeject` runs and then `cargo test --workspace` runs,
then the in-tree `.claude/agents/speccy-work.md` contains `effort: high`,
a second `just reeject` leaves `git status --porcelain` empty, and
`cargo test --workspace` (including `pin_shape`) exits 0.
(hygiene)
</scenario>

<scenario id="CHK-008">
Given the next dogfood `speccy-work` run after this SPEC lands,
when it appends its `<implementer>` block,
then the recorded `model=` suffix is `/high`, confirming the
definition-file effort flows through to the recorded value.
(judgment-only / dogfooding: confirmed by inspecting a post-change
journal entry; not CI-provable.)
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
Fork only the reuse concern out of the shared `convention-checklist.md`;
the other four items stay single-source. SPEC-0052 DEC-002 made the
checklist one shared partial so the implementer's pre-flight and the
reviewer's lens were the same text by construction. That still holds for
match-conventions, docs-match-code, no-false-complexity, and
re-apply-hard-rules. Reuse is the exception: the implementation-lifecycle
framing ("survey the area, build with reuse/extend in mind, before
writing") and the reviewer-lifecycle framing ("adversarially verify the
survey, confirm named symbols exist, hunt reinvention") genuinely
diverge and should evolve independently. This partially supersedes
SPEC-0052 DEC-002 for the reuse item only.
</decision>

<decision id="DEC-002">
Reuse detection stays in `reviewer-style` (its forked reviewer variant),
not a new `reviewer-reuse` persona. `reviewer-style` already detects
reuse — it produced the duplication blockers on patina SPEC-0002 T-005
and T-006. Adding a dedicated persona would duplicate an existing,
working detector — the exact reinvention this SPEC discourages — and
expand every review fan-out by one. The forked reviewer variant gives
reuse its own lifecycle-scoped text inside the persona that already owns
it, preserving the "Review owns semantic judgment" core principle.
</decision>

<decision id="DEC-003">
The `Reuse survey` is a free-form journal field, not a lint-enforced
one. The `<implementer>` body is stored verbatim; the parser validates
only the `date` / `model` / `round` attributes, and the six existing
body fields are unenforced convention. A lint requiring the field would
be inconsistent with those six, would push brittle prose-substring
matching into the deterministic core (the anti-pattern the test-hygiene
rule forbids), and could not judge survey honesty regardless. Whether
the survey is present and honest is a judgement call that belongs to the
forked `reviewer-style` reuse variant, not to a CLI lint.
</decision>

<decision id="DEC-004">
The survey runs in the implementer's own context, not a spawned
sub-agent. Under `speccy-orchestrate` the `speccy-work` agent is a leaf,
and sub-agents cannot spawn sub-agents, so a fresh-context reuse pass
could only live in the orchestrator (deferred — see Notes). Within the
worker's own context, the `opus[1m]` / `high` repin (REQ-004) is what
resources the extend-vs-fresh judgement; `reviewer-style` remains the
independent adversarial backstop for what the same-context survey
misses.
</decision>

<decision id="DEC-005">
Repin Claude Code `speccy-work` to `opus[1m]` / `high`, superseding the
`opus[1m]` / `low` value SPEC-0036 set for the effort field. SPEC-0036
chose `low` to keep cost and latency close to the prior Sonnet-medium
tier and explicitly flagged the choice as revisable by a future
amendment. The bounded survey adds an extend-vs-fresh judgement load on
top of the general single-pass-quality argument SPEC-0036 already
accepted; `high` makes the implementer a peer of the correctness and
security reviewers. The repin is Claude-Code-only — Codex's
`model_reasoning_effort` axis is a separate call per SPEC-0036 — and
ships no measurement step, following SPEC-0036's qualitative precedent.
`high` is already a valid Opus effort in the `pin_shape` allow-set, so no
enum change is needed.
</decision>

<decision id="DEC-006">
Proof shape: no prose-asserting unit tests, which would be vacuous per
the project's test-hygiene rule against substring-matching curated
prose. The only deterministic gate is `just reeject` plus
`cargo test --workspace` staying green, which proves the new includes
resolve, the bumped pin renders, and no pack-structure or pin-shape test
breaks. The survey's efficacy and the reuse-churn reduction are
judgment-only, verified by dogfooding across subsequent runs and never
in CI.
</decision>

## Assumptions

<assumptions>
- The fix belongs entirely in the prompt/skill layer and agent
  frontmatter under `resources/`; the deterministic Rust CLI gains no
  reuse, journal-field, or identity logic.
- The `Reuse survey` field is unenforced by lint or parser. Verified
  against `speccy-core`: `parse/journal_xml` stores the `<implementer>`
  body verbatim and validates only `date` / `model` / `round`; no lint
  rule references the body field set. Enforcement of the field's
  presence and honesty is adversarial, via the forked `reviewer-style`
  reuse variant — a worker could omit it and only the reviewer catches
  that.
- The survey is bounded to the task's area (covered REQs +
  suggested-files hint + immediate neighbours); reusable code far
  outside that area will not be found, and that is accepted.
- A pre-implementation survey run in the worker's own context can
  meaningfully shift the reuse-miss rate. This is plausible but unproven
  causally — the patina sample is small and the effect is observable
  only across future runs, never in CI.
- `effort: high` is Claude-Code-only; the Codex `model_reasoning_effort`
  axis is unchanged, matching SPEC-0036's scoping.
- The recurring reuse-drift pattern generalises beyond Rust/patina; the
  survey and the forked variants are written language- and
  project-agnostic so they ship safely downstream via `speccy init`.
</assumptions>

## Open Questions

None — the brainstorm open questions (a–d) were resolved before
drafting; the retry-round survey cadence (round-1 area-map, re-run only
on a reuse-related blocker; per-symbol floor round-agnostic) is captured
in REQ-001 and REQ-002.

## Notes

This SPEC builds directly on two shipped (archived) specs. SPEC-0052
introduced the implementer self-review and the shared
`convention-checklist.md`; this SPEC adds a pre-implementation survey
upstream of that self-review and forks the reuse item out of the shared
checklist (DEC-001). SPEC-0036 pinned the Claude Code implementer to
`opus[1m]` / `low`; this SPEC raises the effort to `high` (DEC-005).
Both are archived, so this is a new SPEC rather than an amendment;
`supersedes` stays empty because no *active* spec is being replaced.

Framings considered and rejected in brainstorm, for the record:

- **Post-write mechanical search** (grep for the symbols you just added).
  Rejected: it catches exact and near-exact duplicates but structurally
  misses *extendable* similar code, and it treats reuse as cleanup after
  the parallel code already exists rather than as a design input.
- **Dedicated reuse detector / new `reviewer-reuse` persona.** Rejected
  per DEC-002 — redundant with `reviewer-style`.
- **Deterministic script gate at the hygiene step.** Rejected: catches
  only the byte-identical tail, and the hygiene gate is currently
  language-specific; generalising it is a separate project-hook concern.
- **Pre-review reuse slot in `speccy-orchestrate`** running the existing
  `reviewer-style` reuse lens before the persona fan-out. Deferred, not
  rejected: it is the only option that frontloads a *fresh-context* reuse
  pass (DEC-004), but it adds a per-task agent, would be measurement-gated,
  and only works under orchestration — it does not help a standalone
  `/speccy-work` run. Kept out of this slice deliberately, as worker-local
  changes (REQ-001 through REQ-003) work on both invocation paths.

## Changelog

<changelog>
| Date       | Author              | Summary |
|------------|---------------------|---------|
| 2026-05-31 | claude-opus-4-8[1m] | Initial SPEC. Four requirements, all in the `resources/` prompt/frontmatter layer. REQ-001: the `speccy-work` phase runs a bounded pre-implementation reuse survey (three tiers: reuse-as-is / extend / write-fresh; round-agnostic per-symbol floor, round-1 / reuse-blocker-triggered area-map) before the implement step. REQ-002: the `<implementer>` journal records it in a free-form `Reuse survey` field with round-1-versus-retry semantics and no parser/lint change. REQ-003: the reuse concern is forked out of the shared `convention-checklist.md` into an implementer variant (in the phase) and a reviewer variant (in `reviewer-style`), the other four checklist items staying shared. REQ-004: the Claude Code `speccy-work` agent is repinned `opus[1m]` / `low` → `opus[1m]` / `high` across template, regenerated in-tree file, and README, Codex unchanged. Six decisions: DEC-001 (fork only the reuse item, partially superseding SPEC-0052 DEC-002); DEC-002 (no new persona — reuse stays in `reviewer-style`); DEC-003 (free-form journal field, no lint, enforcement via reviewer); DEC-004 (survey in the worker's own context, opus/high resources the judgement, reviewer is the backstop); DEC-005 (opus/high repin superseding SPEC-0036's effort value, Claude-only, no measurement); DEC-006 (proof shape = `just reeject` + `cargo test --workspace` green; efficacy judged by dogfooding). Grounded in patina SPEC-0002 reuse-drift retries (T-005, T-006). |
</changelog>
