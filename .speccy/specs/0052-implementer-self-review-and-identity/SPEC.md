---
id: SPEC-0052
slug: implementer-self-review-and-identity
title: Implementer pre-handoff self-review and accurate model/effort recording
status: in-progress
created: 2026-05-28
supersedes: []
---

# SPEC-0052: Implementer pre-handoff self-review and accurate model/effort recording

## Summary

Dogfooding Speccy on the patina repo surfaced two defects in the
implementer/review loop, both rooted in the `speccy-work` implementer
prompt.

First, review-round churn: 13 of 15 tasks in
`patina/.speccy/specs/0001-patina-core-engine/` needed at least one
retry round, and the tax was almost entirely the **style** persona —
11 legitimate style blockers plus one false-positive that still cost a
round. Every legitimate style blocker shared two properties: it passed
the hygiene gate (clippy / fmt / tests green before the `in-review`
flip), so more lints cannot catch it; and catching it required reading
the new diff against the existing codebase or the project's own
`AGENTS.md`. The implementer never re-reads its own diff through the
reviewers' lens before handing off, so convention drift that is cheap
to fix in place becomes a full review round plus respawn.

Second, wrong provenance: the `<implementer>` journal block records
the model identity from inherited environment (`CLAUDE_EFFORT`,
`ANTHROPIC_MODEL`), which for a sub-agent reflects the **parent
session**, not the sub-agent's own configuration. T-016's round-1
block recorded `model="claude-opus-4.8[1m]/high"` while the
`speccy-work` sub-agent was pinned `effort: low` — wrong effort
(inherited from a high-effort orchestrator) and a normalized model
string (`4.8` dot instead of the canonical `4-8` hyphen).

This SPEC front-loads the reviewers' standards into a pre-handoff
self-review (north stars, not adversarial tactics) backed by a single
shared convention checklist, adds a grounding rule that stops the
style reviewer's false-positive blockers, and fixes identity recording
to derive from the single source that already configures each
sub-agent — its own definition file — rather than inherited
environment.

## Goals

<goals>
- The `speccy-work` implementer runs a self-review immediately after
  implementation, before the `in-review` flip, that surfaces all four
  reviewer personas' north stars and a convention checklist.
- The convention checklist lives in one shared source included
  verbatim at both review-relevant callsites: the implementer
  self-review and the style reviewer.
- The style reviewer stops raising blocking verdicts that demand a
  lint-driven change without first confirming the lint fires.
- A self-recording agent records the effort it was configured with
  (read from its own definition file) rather than the parent session's
  inherited effort.
- A self-recording agent records the resolved long-form model
  identifier its host states in-context, transcribed verbatim without
  punctuation normalization.
- The `speccy-plan` SPEC template reference documents the parser-required
  `<changelog>` element, so an author following it verbatim does not hit
  an `SPC-001` missing-changelog parse error.
</goals>

## Non-goals

<non-goals>
- No Rust CLI changes. The deterministic core gains no lint logic and
  no identity-resolution logic; this is entirely skill/prompt prose
  under `resources/`.
- No front-loading of reviewers' adversarial **tactics** — only their
  north stars. Teaching the implementer how reviewers detect fakes
  would erode the "Review owns semantic judgment" independence.
- No edits to the business, tests, or security persona review bodies
  beyond the one shared identity-sourcing line they already carry.
- No prose-asserting unit tests. The review-churn reduction and the
  recorded-identity correctness are verified by dogfooding, not by CI
  string-matching curated prose.
- No reading of `CLAUDE_CODE_EFFORT_LEVEL` and no attempt to reflect a
  deliberate runtime effort override in the recorded value.
</non-goals>

## User Stories

<user-stories>
- As a Speccy contributor driving the loop, I want the implementer to
  self-catch convention drift before handoff so first-round work
  bounces less and review cycles shrink.
- As someone auditing a journal months later, I want the recorded
  model and effort to reflect what actually ran, so the provenance is
  trustworthy rather than a parent-session artifact.
- As someone customizing a sub-agent, I want its model and effort
  configured in one file (the agent definition) so an override is a
  single edit that flows to both execution and recording.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Implementer self-review before handoff

The `speccy-work` implementer phase performs a self-review
immediately after implementation, before flipping the task to
`in-review`, prompting the agent to re-read its own diff against the
reviewers' criteria and fix what it finds in place.

<done-when>
- The ejected `speccy-work` phase body contains a self-review step
  positioned after the implement step and before the `in-review`
  state flip / hygiene gate.
- The step instructs the agent to address findings before the flip,
  framed as the cheap place to catch drift versus a later review
  round.
</done-when>

<behavior>
- Given the implementer has finished writing code, when it reaches the
  exit transition, then it first performs the self-review and resolves
  findings before flipping `state` to `in-review`.
</behavior>

<scenario id="CHK-001">
Given the ejected `speccy-work` phase body after `just reeject`,
when a reviewer reads it,
then a self-review-before-handoff step appears after the implement
step and before the `in-review` flip, instructing the agent to fix
drift in place first.
(judgment-only: prose presence and placement confirmed on the diff.)
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Reviewer north-star map, not tactics

The implementer self-review presents each of the four reviewer
personas' north stars — what good looks like — without disclosing the
reviewers' adversarial detection tactics.

<done-when>
- The north-star map names business, tests, security, and style, each
  with a one-line statement of the outcome to achieve.
- The map withholds reviewer tactics; it does not describe how any
  reviewer hunts for fakes (e.g. no mention of the tests reviewer's
  fabrication-detection method or mutation experiments).
- The style entry points at the shared convention checklist rather
  than restating its contents.
</done-when>

<behavior>
- Given the north-star map, when the implementer reads the tests
  entry, then it sees the outcome to achieve (drive real behaviour,
  assert the specific contract, keep evidence honest and complete) and
  not the method the reviewer uses to catch fabrication.
</behavior>

<scenario id="CHK-002">
Given the self-review map in the ejected phase body,
when a reviewer reads the four persona entries,
then each states a north star, none discloses a reviewer's detection
tactic, and the style entry defers to the shared checklist.
(judgment-only: confirmed on the diff.)
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Shared convention checklist — one source, two callsites

The convention-drift categories live in one shared partial under
`resources/modules/`, included verbatim at both review-relevant
callsites: the implementer self-review and the style reviewer persona.

<done-when>
- A new partial enumerates five categories: reuse-over-reinvent;
  match-local-conventions; docs-match-code; no-false-complexity
  (including splitting a function past the file's complexity ceiling);
  re-apply-the-project's-own-hard-rules (including vacuous /
  constant-copy tests and suppressions that must carry a
  justification).
- The implementer phase body and the style reviewer persona each
  `{% include %}` that partial.
- The style reviewer's prior bespoke "what to look for that's easy to
  miss" enumeration is removed; its unique items are subsumed by the
  partial.
</done-when>

<behavior>
- Given a change that duplicates an existing helper, when the
  implementer self-reviews and when the style reviewer later reviews,
  then both read the same reuse-over-reinvent criterion from the one
  shared source.
</behavior>

<scenario id="CHK-003">
Given the ejected implementer phase body and the ejected style
reviewer persona after `just reeject`,
when a reviewer reads both,
then the same convention-checklist text appears in each, sourced from
one partial, and the style reviewer's old bespoke enumeration is gone.
(judgment-only: confirmed on the diff.)
</scenario>

<scenario id="CHK-004">
Given the edited `resources/`,
when `just reeject` and `cargo test --workspace` run,
then both exit 0 — proving the new include resolves at render time at
both callsites and no pack-structure test breaks.
(hygiene)
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Style reviewer grounding rule against false-positive lint blocks

The style reviewer does not raise a blocking verdict demanding a
lint-driven change — above all, demanding that a suppression
annotation be added — without first confirming the underlying lint
fires.

<done-when>
- The style persona carries a rule: before blocking, confirm the lint
  fires without the demanded change, and treat "every sibling file
  carries it" as insufficient grounds on its own.
- The rule directs an unconfirmable demand to a one-line aside outside
  the `<review>` block rather than a blocking verdict.
</done-when>

<behavior>
- Given a test file that lacks a suppression annotation its siblings
  carry, when the style reviewer considers blocking, then it first
  checks whether the lint actually fires on this file and downgrades
  to an aside when it cannot show that it does.
</behavior>

<scenario id="CHK-005">
Given the ejected style reviewer persona after `just reeject`,
when a reviewer reads it,
then it carries a rule requiring lint-fire confirmation before a
lint-driven blocking verdict, with the sibling-consistency argument
named as insufficient on its own.
(judgment-only: confirmed on the diff; the originating false positive
is patina T-015.)
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Recorded effort sourced from the agent's own definition file

An agent that records its own identity sources the effort suffix from
its own sub-agent definition file, never from inherited environment
variables.

<done-when>
- The recording instruction reads effort from the agent's definition
  file (`effort:` on Claude Code, `model_reasoning_effort` on Codex)
  and explicitly forbids deriving it from `CLAUDE_EFFORT` or other
  inherited environment variables.
- A `speccy-work` sub-agent pinned `effort: low`, dispatched from a
  higher-effort orchestrator session, records the `/low` suffix.
</done-when>

<behavior>
- Given a sub-agent whose definition file pins `effort: low`, spawned
  under a parent session running at high effort, when it appends its
  `<implementer>` block, then the recorded effort suffix is `/low`.
</behavior>

<scenario id="CHK-006">
Given the next dogfood run of a `speccy-work` sub-agent under a
high-effort orchestrator,
when it appends its `<implementer>` block,
then the `model=` suffix is the definition-file effort (`/low`), not
the parent session's effort.
(judgment-only / dogfooding: not CI-provable; confirmed by inspecting
a post-change journal entry.)
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Recorded model is the verbatim resolved long-form identifier

An agent records the resolved long-form model identifier exactly as
its host states it in-context, transcribed verbatim without
punctuation normalization.

<done-when>
- The recorded model id is the host-stated resolved identifier (e.g.
  `claude-opus-4-8[1m]`), not the `ANTHROPIC_MODEL` alias and not a
  reformatted string.
- Version punctuation is preserved as the host states it
  (`claude-opus-4-8`, never `claude-opus-4.8`).
- Where a host states no resolved identifier in-context, the recorded
  value falls back to the `model` value in the agent's definition
  file.
</done-when>

<behavior>
- Given a host that states `The exact model ID is claude-opus-4-8[1m]`
  in the agent's context, when the agent records identity, then the
  model segment is `claude-opus-4-8[1m]` verbatim.
</behavior>

<scenario id="CHK-007">
Given the next dogfood run of any self-recording agent on Claude Code,
when it records its identity,
then the model segment matches the host-stated in-context model id
verbatim, hyphens preserved.
(judgment-only / dogfooding: confirmed by inspecting a post-change
journal or review block.)
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Identity-sourcing rule consolidated into one shared partial

The model-plus-effort identity-sourcing rule lives in a single shared
partial included at every site where an agent records its own
identity.

<done-when>
- A single shared partial states the sourcing rule (model from
  host-stated in-context identity; effort from the agent's own
  definition file) and documents the `CLAUDE_CODE_EFFORT_LEVEL`
  runtime-override limitation.
- That partial is included by the implementer phase, the reviewer
  verdict-return contract, and the vet personas.
</done-when>

<behavior>
- Given the reviewer verdict contract and the implementer phase, when
  each renders, then both pull the identity-sourcing rule from the
  same partial rather than restating it.
</behavior>

<scenario id="CHK-008">
Given the ejected implementer phase, reviewer verdict contract, and
vet personas after `just reeject`,
when a reviewer reads them,
then each carries the identity-sourcing rule from the one shared
partial, including the documented override limitation.
(judgment-only: confirmed on the diff.)
</scenario>

<scenario id="CHK-009">
Given the edited `resources/`,
when `just reeject` runs,
then it exits 0 — proving the identity-sourcing partial resolves at
every include site.
(hygiene)
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Misleading identity guidance and example strings corrected

The previously misleading identity guidance and example strings are
corrected to match the single-source rule.

<done-when>
- The "sourced from the host harness's runtime model identifier (env
  var, runtime API, ...)" phrasing in the review fan-out partial and
  in the decompose phase is replaced with the single-source rule.
- Dot-form example strings (`claude-opus-4.8`) in the shipped
  resources are corrected to the canonical hyphen form
  (`claude-opus-4-8`).
</done-when>

<behavior>
- Given the shipped resources after this change, when an agent looks
  for guidance on how to source its model identity, then it finds the
  single-source rule rather than the env-var phrasing, and the example
  strings model the canonical hyphen form.
</behavior>

<scenario id="CHK-010">
Given the ejected resources after `just reeject`,
when a reviewer greps the changed files,
then the env-var/runtime-API sourcing phrasing is gone from the review
fan-out and decompose paths, and no dot-form `claude-opus-4.8` example
remains in the touched files.
(judgment-only: confirmed on the diff.)
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: speccy-plan SPEC template documents the required `<changelog>` element

The `speccy-plan` SPEC template reference documents the `<changelog>`
element the parser requires, so an author following the template
verbatim does not produce an `SPC-001` missing-changelog parse error.

<done-when>
- `resources/modules/references/spec.md` carries a `## Changelog`
  section with a `<changelog>` table in its worked example, matching
  the `Date | Author | Summary` shape archived specs already use.
- The reference's "Shape invariants the lint suite enforces" list names
  `<changelog>` among the required elements.
- `just reeject` propagates the corrected reference to the ejected
  `speccy-plan` skill across hosts.
</done-when>

<behavior>
- Given the corrected template reference, when an author writes a new
  SPEC.md following it verbatim, then the result includes a
  `<changelog>` element and `speccy verify` does not report `SPC-001`
  for a missing changelog.
</behavior>

<scenario id="CHK-011">
Given the ejected `speccy-plan` SPEC template reference after
`just reeject`,
when a reviewer reads it,
then it shows a `## Changelog` / `<changelog>` worked example and lists
`<changelog>` among the enforced shape invariants.
(judgment-only: confirmed on the diff; the originating friction is this
SPEC's own drafting tripping SPC-001.)
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
Front-load each reviewer persona's **north star** (what good looks
like) but not the reviewers' adversarial **tactics**. Mechanical and
convention drift has zero adversarial value in deferring, so catching
it early is pure win; semantic judgment stays with the independent
reviewers per the "Review owns semantic judgment" core principle.
Disclosing tactics (especially the tests reviewer's fabrication
detection) would invite the implementer to write to pass the review
rather than to be correct.
</decision>

<decision id="DEC-002">
The convention checklist is a single shared partial `{% include %}`d
by both the implementer self-review and the style reviewer, per the
repo's own dedup convention, so the implementer's pre-flight and the
reviewer's lens are the same text by construction.
</decision>

<decision id="DEC-003">
Identity recording is dual-sourced from the single configuration
source that already governs each sub-agent — its own definition file —
with no copy inlined at any spawn site or baked into a rendered body.
The model id is read from the resolved identifier the host states
in-context (which reflects the definition's `model:`); the effort is
read from the definition file's effort field at runtime. The
orchestrator and skill entry already dispatch by name and defer to the
definition file, so the override knob stays a single file edit.
Render-time injection of the configured values was rejected because it
would bake a second copy into the body and a downstream edit of the
ejected definition file would not flow to the recorded value.
</decision>

<decision id="DEC-004">
The `CLAUDE_CODE_EFFORT_LEVEL` runtime override is deliberately not
read; a run that sets it will record the definition-file effort, not
the override. This is an accepted, documented limitation — the
override is a rare, deliberate action, and reading it would reintroduce
host-specific environment logic the single-source rule is meant to
remove.
</decision>

<decision id="DEC-005">
Proof shape: no prose-asserting unit tests (they would be vacuous per
the project's test-hygiene rule against substring-matching curated
prose). The only deterministic gate is `just reeject` plus
`cargo test --workspace` staying green, which proves the new includes
resolve and no pack-structure test breaks. The review-churn reduction
and the recorded-identity correctness are judgment-only, verified by
dogfooding across subsequent runs.
</decision>

## Assumptions

<assumptions>
- The fix belongs entirely in the prompt/skill layer under
  `resources/`; the deterministic Rust CLI gains no lint or
  identity-resolution logic.
- A prompt-level self-review can meaningfully shift the first-round
  pass rate. This is plausible but unproven causally — the patina
  sample is n=1 and the effect is observable only across future runs,
  never in CI.
- The recurring convention-drift categories generalize beyond
  Rust/patina; the checklist is written language- and
  project-agnostic so it ships safely to downstream repos via
  `speccy init`.
- The host states a resolved long-form model identifier in the agent's
  context (Claude Code does); where a host does not, the definition
  file's `model` value is an acceptable fallback.
- Folding the style reviewer's bespoke "what to look for" enumeration
  into the shared partial does not weaken the reviewer, because the
  partial subsumes those items.
</assumptions>

## Open Questions

- [ ] a. **Self-review caught:** Does the all-four-persona north-star
  map measurably reduce non-style first-round blocks, or is the
  business/tests/security orientation noise that should be trimmed to
  the style-only checklist (the one category with evidence)? Resolvable
  only by dogfooding; revisit after several post-change runs.

## Notes

Two framings were rejected in brainstorm. **Style-only self-review**
(checklist, no persona map) is the fallback if the all-persona map
proves to be noise per open question a. **Full reviewer playbook**
(tactics included) was rejected outright: it maximizes first-pass rate
but erodes the adversarial independence that the "Review owns semantic
judgment" core principle depends on.

The two requirement clusters — review-churn reduction (REQ-001 through
REQ-004) and identity recording (REQ-005 through REQ-008) — are
distinct outcomes bundled into one SPEC because they share the
implementer edit surface and the implementer/review-loop-fidelity
theme. `speccy-decompose` will split them into separate tasks. REQ-009
is a small adjacent fix to the `speccy-plan` template reference, folded
in at the author's request after this SPEC's own drafting hit the
missing-`<changelog>` bug; decompose will give it its own task.

The recorded-effort accuracy depends on the orchestrator and skill
entry continuing to dispatch sub-agents by name and deferring
model/effort to the definition file (confirmed at
`resources/modules/skills/speccy-orchestrate.md` work dispatch and
`resources/modules/skills/speccy-work.md` skill entry). If a future
change inlines a model/effort at a spawn site, the single-source
property breaks.

## Changelog

<changelog>
| Date       | Author              | Summary |
|------------|---------------------|---------|
| 2026-05-28 | claude-opus-4-8[1m] | Initial draft. Eight requirements across two clusters sharing the implementer edit surface. Review-churn cluster: (REQ-001) `speccy-work` runs a pre-handoff self-review before the `in-review` flip; (REQ-002) the self-review carries all four reviewer personas' north stars, not their adversarial tactics; (REQ-003) a single shared convention checklist partial is `{% include %}`d by both the implementer self-review and the style reviewer; (REQ-004) the style reviewer gains a grounding rule against false-positive lint-suppression blocks. Identity-recording cluster: (REQ-005) recorded effort is sourced from the agent's own definition file, never inherited env; (REQ-006) recorded model is the host-stated resolved long-form id transcribed verbatim; (REQ-007) the identity-sourcing rule is consolidated into one shared partial included at all self-record sites; (REQ-008) the misleading env-var sourcing phrasing and dot-form example strings are corrected. Five decisions: DEC-001 (north stars not tactics, preserving Review-owns-semantic-judgment); DEC-002 (shared-include checklist); DEC-003 (dual-source identity from the agent definition file, no spawn-site or render-baked copy); DEC-004 (CLAUDE_CODE_EFFORT_LEVEL override accepted as a documented limitation); DEC-005 (no prose-asserting tests — dogfooding plus the reeject/build gate). All brainstorm open questions a–f resolved before draft; one residual question (a) on whether the all-persona map helps or is noise, deferred to dogfooding. |
| 2026-05-28 | claude-opus-4-8[1m] | Amendment: added REQ-009 and a matching goal to fix friction discovered while drafting this SPEC — the `speccy-plan` template reference at `resources/modules/references/spec.md` omits the parser-required `<changelog>` element (and its shape-invariants list does not mention it), so following the template verbatim trips `SPC-001`. Adjacent tooling fix to the planning skill, folded in at the author's request. |
</changelog>
