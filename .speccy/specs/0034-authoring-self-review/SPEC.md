---
id: SPEC-0034
slug: authoring-self-review
title: Self-review pass in authoring-phase skills (plan, amend, brainstorm)
status: implemented
created: 2026-05-19
supersedes: []
---

# SPEC-0034: Self-review pass in authoring-phase skills (plan, amend, brainstorm)

## Summary

The three authoring-phase skills — `/speccy-plan`, `/speccy-amend`,
`/speccy-brainstorm` — each translate between two representations:
brainstorm-chat artifacts → SPEC.md (plan), the user's amendment
intent → surgical SPEC.md diff (amend), fuzzy ask → atomized
four-artifact draft (brainstorm). Every translation is a drift
opportunity. Requirements get re-worded into ambiguity. Scope creeps
past the approved framing. Brainstorm-artifact outputs land in the
wrong SPEC.md sections. Placeholder template residue leaks through.
None of these are caught by the existing `/speccy-review` pass —
that fires on tasks in `state="in-review"`, after implementation has
already begun. Translation drift gets cheaper to fix the earlier it
is caught; the cheapest moment is immediately after the authoring
agent writes its artifact, before the user reviews it.

This SPEC adds a self-review pass to each of the three skills'
MiniJinja templates (post-SPEC-0033). After writing SPEC.md (plan /
amend) or after drafting the four artifacts internally
(brainstorm), the agent runs a fixed-template self-review pass that
verifies a set of properties. Mechanical issues — string-matchable
problems like `TBD` residue, missing `## Changelog` headers,
"and"/"also" inside `<requirement>` blocks, untouched `<...>`
placeholders — are fixed inline without surfacing. Semantic issues —
LLM-judged problems like internal contradictions, ambiguous
wording, scope-creep beyond the approved framing, false-alternative
framings, mechanical-filler assumptions — surface via fixed template
strings carried in each skill template. For `/speccy-plan` and
`/speccy-amend`, semantic surfacings land as `- [ ] x. **Self-review
caught:** {issue}` rows in `## Open Questions`. For
`/speccy-brainstorm`, they surface as a fixed-format chat preamble
above the four artifacts (brainstorm has no SPEC.md to write into
yet).

Two supporting changes ride alongside the self-review pass:

- The `## Open Questions` format becomes `- [ ] a.` ...
  `- [ ] z.` alpha-prefix across all three skill templates in
  lock-step. Open questions become referenceable by ordinal
  ("answer to b: ..."); the 26-cap doubles as a brainstorm-scope
  smell detector — a session producing 26+ open questions has a
  scoping problem, not a formatting problem.
- A collapse-parallels heuristic is added to `/speccy-brainstorm`:
  when N restated-ask requirements differ only by one noun, the
  agent may express them as one requirement with an enumerated
  sub-list, at agent discretion to reduce reader cognitive load.
  `/speccy-plan` exercises symmetric discretion in reverse at
  SPEC-write time — expanding sub-bullets to atomic `<requirement>`
  blocks when atomicity adds reviewer-fan-out value, or keeping
  them grouped under one `<requirement>` with a `<done-when>`
  bullet list when cohesive grouping serves the SPEC better.

This SPEC is hard-sequenced after SPEC-0033 (eject-prompt-bodies),
which establishes `resources/modules/skills/speccy-<phase>.md` as
the canonical authoring surface. SPEC-0032 (phase-model-pinning)
is a transitive predecessor via SPEC-0033. The self-review pass and
both supporting changes land entirely in the MiniJinja skill
templates and propagate into both Claude Code and Codex host
packs via the existing eject pipeline; no CLI-side change is part
of this slice.

## Goals

<goals>
- Each of the three authoring-phase skills runs a self-review pass
  at handoff. `/speccy-plan` runs it after writing SPEC.md;
  `/speccy-amend` runs it after writing the SPEC.md diff;
  `/speccy-brainstorm` runs it after drafting the four artifacts
  internally and before presenting them to the user in chat.
- The plan self-review verifies six SPEC.md properties: routing
  fidelity (brainstorm artifacts landed in declared SPEC.md
  sections), requirement atomization (no `<requirement>` body
  contains "and"/"also" multi-outcome wording), scope-traces
  (every `<requirement>` traces to a brainstorm artifact or the
  user's stated ask), internal consistency (no contradictions
  across goals, non-goals, requirements, assumptions), no
  placeholder leakage (`TBD`, `TODO`, untouched `<...>`
  template residue), no ambiguity (no `<requirement>` wording
  interpretable two ways).
- The amend self-review additionally verifies a `## Changelog` row
  was written for this amendment, and the diff stays surgical to
  the stated intent shift (no cascading requirement edits beyond
  what the amendment intent named).
- The brainstorm pre-check verifies four properties of the
  four-artifact draft: restated requirements are atomized,
  alternative framings are structurally distinct (not false
  alternatives that collapse to the same SPEC shape), silent
  assumptions are load-bearing (would change SPEC shape if
  wrong), open questions would change SPEC shape if answered.
- Mechanical issues caught by self-review or pre-check are fixed
  inline without user surfacing. Semantic issues surface via
  fixed template strings carried in the respective skill
  template — not freeform agent prose. For plan and amend, the
  template string is a `- [ ] x. **Self-review caught:** {issue}`
  row appended to `## Open Questions`. For brainstorm, the
  template string is a fixed-format chat preamble that prefixes
  the four-artifact message.
- Neither self-review nor pre-check loops. One pass, fix
  mechanical, surface semantic, hand back. The next checkpoint is
  the user reviewing the artifact (or `/speccy-tasks`
  decomposition); re-running self-review is not part of the flow.
- The `## Open Questions` format across all three skill templates
  becomes `- [ ] a.` ... `- [ ] z.` alpha-prefix. The three
  templates change in lock-step so brainstorm chat output remains
  copy-paste-compatible with SPEC.md `## Open Questions` and
  amendment edits to existing `## Open Questions` sections retain
  ordinal references.
- `/speccy-brainstorm` carries a collapse-parallels heuristic:
  when N restated-ask requirements differ only by one noun, the
  agent may group them under one requirement with an enumerated
  sub-list to reduce reader cognitive load. The heuristic is
  agent discretion, not an enforced threshold. `/speccy-plan`
  exercises symmetric discretion at SPEC-write time — sub-bullets
  may expand to atomic `<requirement>` blocks or stay grouped
  under one `<requirement>` with a `<done-when>` bullet list,
  based on whether atomicity adds value at the SPEC level.
</goals>

## Non-goals

<non-goals>
- No new CLI verb (`speccy self-review`, `/speccy-self-review`).
  Self-review is a checkpoint inside existing authoring-phase
  skills, not a separate phase noun. Adding a verb for an
  inline checkpoint violates the stay-small principle and the
  AGENTS.md ten-command ceiling (already at seven after
  SPEC-0033).
- No shared MiniJinja partial extraction for the overlapping
  plan/amend self-review check lists. Both templates carry their
  own ~80% overlapping copy. Premature factoring is rejected;
  a future amendment may extract a `_partials/` shared snippet if
  the duplication becomes a maintenance burden.
- No structural lint in `speccy check` or `speccy verify` for the
  six plan-self-review check properties. Routing fidelity,
  atomization, scope-traces, internal consistency, and ambiguity
  are semantic checks (LLM-judged); the proof-shape lint surface
  stays orthogonal per AGENTS.md core principle 3. The two
  mechanical sub-checks (placeholder strings, Changelog row
  presence) are CLI-suitable in isolation but bundled into the
  skill-level pass for consistency — splitting across CLI and
  skill layers complicates the mental model.
- No loop on self-review or pre-check. One pass, fix mechanical
  or surface semantic, hand back. Matches obra/superpowers'
  "fix any issues inline. No need to re-review — just fix and
  move on" precedent; speccy's `/speccy-review` runs adversarial
  multi-persona review on tasks later in the loop, not on the
  authoring artifacts themselves.
- No extension of `/speccy-review`'s task-state state machine to
  cover SPEC-write checkpoints. `/speccy-review` fires on tasks
  in `state="in-review"`; this SPEC's self-review fires
  immediately at authoring-phase handoff. Conflating triggers
  would muddy the state model.
- No enforcement of the collapse-parallels heuristic. Agent
  discretion only. Whether N parallel requirements get collapsed
  in brainstorm chat (or expanded back in SPEC.md) is not a
  checked precondition; failing to collapse when it would have
  helped is not surfaced as a self-review semantic issue.
- No retroactive re-formatting of historical SPECs'
  `## Open Questions` sections to alpha-prefix. The format change
  applies going forward; existing SPECs retain their `- [ ]`
  formatting unless edited via a normal amendment that touches
  their open-questions section.
- No automated migration tool for in-flight brainstorm sessions
  spanning this SPEC's land date. Sessions started before this
  SPEC ships finish under the old (unordered) format; sessions
  started after ship use the new alpha-prefix format.
</non-goals>

## User Stories

<user-stories>
- As a solo developer ending a brainstorm session, I want to
  reference open questions by ordinal ("the answer to b is ...")
  rather than restating them verbatim each time. The alpha-prefix
  format makes the reference cheap and unambiguous; the 26-cap
  signals when I have over-scoped the session and need to
  decompose before continuing.
- As an AI agent finishing `/speccy-plan`, I want to
  catch translation drift between the user-approved brainstorm
  framing and the SPEC.md I just wrote — silent placeholder
  leakage, requirements I un-atomized while expanding into prose,
  scope I crept beyond the approved framing — before the user
  reviews the SPEC. The self-review pass is the right cheap
  checkpoint; running `/speccy-review` on a SPEC would conflate
  two state machines.
- As an AI agent finishing `/speccy-amend`, I want to verify the
  Changelog row landed, the diff stayed surgical, and the
  amendment's intent shift did not cascade into requirements
  outside the stated shift. Self-review catches the cascade
  before the user reads the diff.
- As an AI agent inside `/speccy-brainstorm`, I want to catch my
  own framings if they are false alternatives (collapse to the
  same SPEC shape), my own assumptions if they are mechanical
  filler, and my own open questions if their answers would not
  change SPEC shape — before I waste the user's attention with
  weak artifacts. The pre-check runs at the cheapest moment:
  before chat presentation.
- As an AI agent presenting many parallel restated-ask
  requirements during brainstorm, I want the option to express
  them as one requirement with an enumerated sub-list when they
  differ only by one noun. The user reads less repetition;
  atomicity restores at SPEC-write time only when it adds
  reviewer-fan-out value.
- As a reviewer fan-out persona working on a SPEC the plan
  self-review passed, I have higher confidence that mechanical
  issues (placeholders, missing structure) are absent and
  semantic issues are at minimum surfaced in `## Open Questions`.
  My adversarial review focuses on what was missed, not what was
  forgotten.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Each authoring-phase skill runs a self-review pass at handoff

The three authoring-phase skill templates each carry a self-review
section that fires after the artifact-write step and before the
handoff suggestion to the next phase. `/speccy-plan` runs the pass
after writing SPEC.md, before suggesting `/speccy-tasks SPEC-NNNN`.
`/speccy-amend` runs the pass after writing the SPEC.md diff
(including the `## Changelog` row), before suggesting next steps.
`/speccy-brainstorm` runs the pass after drafting the four
artifacts internally and before presenting them in chat.

<done-when>
- `resources/modules/skills/speccy-plan.md` contains a self-review
  section positioned after the SPEC.md write step and before the
  `/speccy-tasks` handoff suggestion line.
- `resources/modules/skills/speccy-amend.md` contains a self-review
  section positioned after the diff-write and Changelog-append step
  and before the next-step handoff suggestion.
- `resources/modules/skills/speccy-brainstorm.md` contains a
  pre-check section positioned after the agent's internal artifact
  draft and before the chat presentation of the four artifacts.
- Each section is rendered into the ejected SKILL.md files at
  `speccy init` time without manual intervention from upstream
  authoring.
</done-when>

<behavior>
- Given `/speccy-plan` has just written SPEC.md to
  `.speccy/specs/NNNN-slug/SPEC.md`, when the agent reads the next
  step in the skill body, then it sees the self-review pass before
  the `/speccy-tasks` handoff line.
- Given `/speccy-amend` has just written a surgical diff plus
  Changelog row to an existing SPEC.md, when the agent reads the
  next step in the skill body, then it sees the self-review pass
  before any handoff suggestion.
- Given `/speccy-brainstorm` has internally drafted the four
  artifacts and is about to present them in chat, when the agent
  reads the next step in the skill body, then it sees the
  pre-check pass before the chat-presentation step.
</behavior>

<scenario id="CHK-001">
Given a freshly initialized tempdir workspace
(`speccy init --host claude-code` run once against the post-SPEC-0033
build with this SPEC's templates applied),
when the ejected `.claude/skills/speccy-plan/SKILL.md` is read,
then the file contains a self-review section whose body precedes
the `/speccy-tasks` handoff suggestion line.
</scenario>

<scenario id="CHK-002">
Given the same freshly initialized workspace,
when the ejected `.claude/skills/speccy-amend/SKILL.md` and
`.claude/skills/speccy-brainstorm/SKILL.md` are read,
then each file contains a self-review (or pre-check) section
positioned per REQ-001's done-when criteria.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Plan self-review verifies six named SPEC.md properties

The self-review section in `resources/modules/skills/speccy-plan.md`
enumerates exactly six check properties the agent verifies against
the just-written SPEC.md. The skill template names the properties
inline so the rendered skill body is self-contained — no external
reference reads are required to know what gets checked.

<done-when>
- The plan self-review section names: (a) routing fidelity, (b)
  requirement atomization, (c) scope-traces, (d) internal
  consistency, (e) no placeholder leakage, (f) no ambiguity.
- Each property carries a one- or two-sentence description that
  defines what passing the check looks like.
- Routing fidelity is defined relative to the brainstorm
  artifacts' declared SPEC.md sections per the routing list in
  `/speccy-brainstorm`'s SKILL.md (restated ask → Summary +
  Requirements; assumptions → `<assumptions>`; open questions →
  `## Open Questions`; rejected framings → `## Notes` or
  `<decision>` blocks).
- The skill body indicates routing fidelity applies only when
  brainstorm ran for this SPEC; when brainstorm was skipped,
  scope-traces alone covers the equivalent check (against the
  user's stated ask).
</done-when>

<behavior>
- Given the ejected `.claude/skills/speccy-plan/SKILL.md`, when
  the file's self-review section is read, then the six named
  properties appear in order with their descriptions.
- Given a plan run where the brainstorm phase was skipped (user
  invoked `/speccy-plan` directly with a clear ask), when the
  agent reaches the self-review pass, then the agent reads the
  skill body's guidance that routing fidelity is N/A in this
  path and only the other five properties apply.
</behavior>

<scenario id="CHK-003">
Given the ejected `.claude/skills/speccy-plan/SKILL.md` post-SPEC,
when its self-review section is parsed for property names,
then the six identifiers "routing fidelity", "atomization",
"scope-traces", "internal consistency", "placeholder leakage", and
"ambiguity" all appear in the section body.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Amend self-review additionally verifies Changelog and surgical-diff shape

The self-review section in `resources/modules/skills/speccy-amend.md`
shares the six properties from REQ-002 and additionally names two
amend-specific check properties: `## Changelog` row presence (the
amendment wrote a row) and surgical-diff shape (the diff stays
narrow to the stated intent shift, no cascading edits beyond it).

<done-when>
- The amend self-review section enumerates the six properties
  from REQ-002 plus two additional amend-specific properties:
  Changelog row presence and surgical-diff shape.
- Each amend-specific property carries a one- or two-sentence
  description defining what passing looks like, parallel in
  style to the six shared properties.
- The skill body explains that the diff-shape check fires only
  in the amend self-review surface; `/speccy-plan` writes new
  SPEC.md content rather than a diff, so the check does not
  apply to the plan self-review surface.
</done-when>

<behavior>
- Given the ejected `.claude/skills/speccy-amend/SKILL.md`, when
  the self-review section is read, then the eight check
  properties (six shared + two amend-specific) appear with
  descriptions.
- Given an amendment whose surgical-diff shape check surfaces a
  cascading edit, when the agent runs the self-review, then the
  cascading edit is surfaced as a semantic issue per REQ-006 (not
  silently reverted).
</behavior>

<scenario id="CHK-004">
Given the ejected `.claude/skills/speccy-amend/SKILL.md` post-SPEC,
when its self-review section is parsed for property names,
then the six shared identifiers from CHK-003 appear plus the two
amend-specific identifiers "Changelog row presence" and
"surgical-diff shape".
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Brainstorm pre-check verifies four named artifact properties

The pre-check section in
`resources/modules/skills/speccy-brainstorm.md` enumerates exactly
four check properties the agent verifies against the
internally-drafted four-artifact set before presenting it to the
user. The four properties correspond to the four brainstorm
artifacts.

<done-when>
- The brainstorm pre-check section names: (a) restated requirements
  are atomized, (b) alternative framings are structurally distinct,
  (c) silent assumptions are load-bearing, (d) open questions would
  change SPEC shape.
- Each property carries a one- or two-sentence description of what
  passing the check looks like.
- "Structurally distinct" framings is defined to exclude false
  alternatives — framings that collapse to the same SPEC shape under
  examination, even if they sound different in summary.
- "Load-bearing" assumptions are defined as those that would change
  the SPEC shape if wrong, distinct from mechanical filler that
  states the obvious.
- "Shape-changing" open questions are defined as those whose answers
  would change which requirements appear, not just which prose
  describes existing requirements.
</done-when>

<behavior>
- Given the ejected `.claude/skills/speccy-brainstorm/SKILL.md`,
  when its pre-check section is read, then the four named
  properties appear in order with their descriptions.
- Given a brainstorm draft where two of the three alternative
  framings collapse to the same SPEC shape, when the agent runs
  the pre-check, then "framings structurally distinct" fails and
  the agent surfaces it per REQ-006.
</behavior>

<scenario id="CHK-005">
Given the ejected `.claude/skills/speccy-brainstorm/SKILL.md`
post-SPEC,
when its pre-check section is parsed for property names,
then the four identifiers "atomized restated requirements",
"structurally distinct framings", "load-bearing assumptions", and
"shape-changing open questions" all appear in the section body.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Mechanical issues are fixed inline without user surfacing

The skill templates define a mechanical/semantic split that the
agent applies when self-review or pre-check identifies a problem.
Mechanical issues are string-matchable from the SPEC.md text or
the brainstorm artifacts — `TBD`/`TODO` strings, missing `##
Changelog` headers, "and"/"also" inside `<requirement>` blocks,
untouched `<...>` template placeholders, missing alpha-prefix
ordinals in `## Open Questions`. Mechanical issues are fixed
inline by the agent and not surfaced to the user.

<done-when>
- Each skill template's self-review section defines "mechanical"
  with a concrete list of string-matchable patterns (the five
  example patterns above plus the explicit "if judging requires
  reading semantics, it is semantic" tie-breaker).
- Each skill template instructs the agent to fix mechanical
  issues inline (edit SPEC.md for plan/amend; revise the
  internal draft for brainstorm) without writing anything to
  `## Open Questions` or to chat.
- The mechanical/semantic tie-breaker rule appears in each
  template body verbatim so the agent applies it consistently.
</done-when>

<behavior>
- Given a `/speccy-plan` run that wrote a SPEC.md containing a
  `<requirement>` body with literal text "do X and Y", when the
  agent runs the plan self-review, then the agent edits SPEC.md
  inline to split the requirement and does not write anything
  to `## Open Questions`.
- Given a `/speccy-amend` run that wrote a diff without appending
  a Changelog row, when the agent runs the amend self-review,
  then the agent edits SPEC.md inline to append the missing
  Changelog row and does not write anything to `## Open Questions`.
- Given a `/speccy-brainstorm` draft where one open-questions
  bullet is missing its alpha-prefix ordinal, when the agent
  runs the pre-check, then the agent fixes the prefix in the
  internal draft and proceeds to chat presentation without a
  preamble.
</behavior>

<scenario id="CHK-006">
Given each ejected authoring-phase SKILL.md post-SPEC,
when the body is parsed for the mechanical/semantic split prose,
then each file contains the tie-breaker line "if judging requires
reading semantics, it is semantic" (or equivalent verbatim) and
the concrete mechanical pattern list.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Semantic issues surface via fixed template strings

Each skill template carries a literal template string that the
agent uses verbatim when surfacing a semantic issue caught by
self-review or pre-check. The template string is not freeform
agent prose. For `/speccy-plan` and `/speccy-amend`, the surfacing
target is `## Open Questions` and the template renders as a
`- [ ] x. **Self-review caught:** {issue}` row. For
`/speccy-brainstorm`, the surfacing target is the chat preamble
above the four-artifact message and the template renders as a
fixed-format block introducing the caught issues.

<done-when>
- `resources/modules/skills/speccy-plan.md` contains the literal
  template string `- [ ] {ordinal}. **Self-review caught:**
  {issue}` (or equivalent unambiguous form) for appending
  semantic surfacings to `## Open Questions`.
- `resources/modules/skills/speccy-amend.md` carries the same
  literal template string.
- `resources/modules/skills/speccy-brainstorm.md` contains a
  fixed-format chat-preamble template with a verbatim opening
  line (e.g. "**Self-review caught the following before
  presenting artifacts:**") followed by a bullet list of issues
  and a verbatim closing line (e.g. "Proceeding with the four
  artifacts below.").
- The `{ordinal}` substitution in the plan/amend template
  produces the next free alpha-prefix letter for the
  `## Open Questions` section (continuing the existing
  alpha-prefix sequence).
- The `{issue}` substitution in both templates is a one-line
  description of the semantic problem the agent identified.
</done-when>

<behavior>
- Given a `/speccy-plan` run whose self-review surfaces a
  scope-creep issue, when the agent appends the issue to
  `## Open Questions`, then the appended row matches the
  template string with `{ordinal}` filled and `{issue}` filled.
- Given a `/speccy-brainstorm` pre-check that catches a
  false-alternative framing, when the agent presents the four
  artifacts in chat, then the chat message begins with the fixed
  preamble template, lists the caught framing issue, closes with
  the fixed closing line, and presents the four artifacts below.
- Given a `/speccy-amend` self-review that catches a cascading
  diff edit, when the agent appends the issue to
  `## Open Questions`, then the appended row uses the same
  literal template string as the plan template (consistency
  across the two amend/plan surfaces).
</behavior>

<scenario id="CHK-007">
Given the ejected `.claude/skills/speccy-plan/SKILL.md` post-SPEC,
when the self-review section is searched for the literal
substring `**Self-review caught:**`,
then exactly one match is found, embedded in the documented
fixed-template string for `## Open Questions` surfacing.
</scenario>

<scenario id="CHK-008">
Given the ejected `.claude/skills/speccy-brainstorm/SKILL.md`
post-SPEC,
when the pre-check section is read,
then it contains a verbatim chat-preamble opening line and a
verbatim closing line, both quoted in the template body as the
strings the agent uses unchanged.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Self-review and pre-check do not loop

Each skill template specifies that the self-review (or pre-check)
runs exactly once per artifact-write. After mechanical fixes are
applied inline and semantic issues are surfaced, the agent
proceeds to handoff (or chat presentation, for brainstorm). The
agent does not re-run the pass to verify its own fixes; the next
checkpoint is the user reviewing the artifact (plan / amend) or
the user responding to the four artifacts (brainstorm).

<done-when>
- Each skill template's self-review section contains an explicit
  no-loop instruction (e.g. "Run this pass exactly once. Do not
  re-check after applying fixes.").
- The no-loop instruction appears in each of the three skill
  templates in identical or near-identical wording so the
  contract is uniform.
- The skill template's surrounding flow (the steps before and
  after the self-review section) does not contain any branch
  that would re-enter the self-review section.
</done-when>

<behavior>
- Given any of the three authoring-phase skill bodies, when the
  agent runs the self-review and applies fixes, then the next
  step is handoff/presentation, not re-review.
- Given a `/speccy-plan` run where the self-review identified
  three mechanical issues and one semantic issue, when the agent
  has applied the three fixes and appended the one surfacing row,
  then the agent proceeds directly to suggesting
  `/speccy-tasks SPEC-NNNN`.
</behavior>

<scenario id="CHK-009">
Given each ejected authoring-phase SKILL.md post-SPEC,
when the body is parsed for explicit no-loop instructions,
then each file contains a verbatim "do not re-check" instruction
within or immediately adjacent to the self-review section.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Self-review and pre-check blocks live in `resources/modules/skills/` MiniJinja templates

The self-review prose for all three authoring-phase skills lives
in `resources/modules/skills/speccy-plan.md`,
`resources/modules/skills/speccy-amend.md`, and
`resources/modules/skills/speccy-brainstorm.md` — the MiniJinja
template source files established by SPEC-0033. The ejection
pipeline propagates the self-review sections into both Claude Code
(`.claude/skills/speccy-<phase>/SKILL.md`) and Codex
(`.agents/skills/speccy-<phase>/SKILL.md`, plus the subagent body
files where applicable) host packs identically. No CLI source
change is part of this slice.

<done-when>
- The three named files under `resources/modules/skills/` carry
  the self-review section bodies in their MiniJinja source.
- No file outside `resources/modules/skills/` is edited to add
  self-review prose (excluding the SPEC-0033-established
  `_partials/` directory if a contributor opts to factor any
  literal template-string snippet there — though the brainstorm
  explicitly rejected shared-partial extraction for the
  plan/amend overlap, so the default expectation is two
  independent copies).
- The ejection pipeline (MiniJinja render at `speccy init` time)
  produces ejected SKILL.md files containing the self-review
  sections without manual intervention.
- The ejected files in both Claude Code and Codex host pack
  locations contain the identical self-review section body
  (modulo any host-specific pointer lines, if any).
</done-when>

<behavior>
- Given the post-SPEC source tree, when a contributor opens
  `resources/modules/skills/speccy-plan.md`, then the file
  contains the plan self-review section body inline.
- Given a freshly initialized workspace
  (`speccy init --host claude-code` run after this SPEC lands),
  when `.claude/skills/speccy-plan/SKILL.md` is read, then it
  contains the plan self-review section body rendered from the
  upstream source.
- Given a freshly initialized workspace
  (`speccy init --host codex` run after this SPEC lands), when
  `.agents/skills/speccy-plan/SKILL.md` is read, then it
  contains the same plan self-review section body.
</behavior>

<scenario id="CHK-010">
Given the post-SPEC source tree,
when the three files
`resources/modules/skills/speccy-plan.md`,
`resources/modules/skills/speccy-amend.md`, and
`resources/modules/skills/speccy-brainstorm.md` are read,
then each file contains a self-review (or pre-check) section
body inline.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: `## Open Questions` format becomes `- [ ] a.` alpha-prefix across all three skill templates

The format guidance in all three authoring-phase skill templates
changes from `- [ ]` (today's unordered list) to `- [ ] a.` ...
`- [ ] z.` alpha-prefix. The change lands in lock-step across the
three templates so brainstorm chat output is copy-paste-compatible
with `## Open Questions` rendered by `/speccy-plan` and edits to
`## Open Questions` via `/speccy-amend` retain ordinal references.

<done-when>
- `resources/modules/skills/speccy-plan.md`'s guidance for
  writing `## Open Questions` specifies the `- [ ] a.` ...
  `- [ ] z.` alpha-prefix format.
- `resources/modules/skills/speccy-amend.md`'s guidance for
  editing `## Open Questions` specifies the same alpha-prefix
  format and instructs the agent to preserve existing ordinals
  when editing rows (don't renumber on amend) and to allocate
  the next free letter when appending a new row.
- `resources/modules/skills/speccy-brainstorm.md`'s guidance for
  the open-questions artifact specifies the same alpha-prefix
  format so the artifact is copy-paste-compatible with
  `/speccy-plan`'s rendering.
- All three templates explicitly note the 26-cap doubles as a
  brainstorm-scope smell detector: hitting `z.` signals the
  session is over-scoped, not that the format is insufficient.
- All three templates note the format change is going-forward
  only; existing SPECs retain `- [ ]` unordered formatting
  unless touched by a normal amendment.
</done-when>

<behavior>
- Given `/speccy-plan` writing a SPEC.md with three
  open questions, when the agent renders `## Open Questions`,
  then the section body uses `- [ ] a.`, `- [ ] b.`, `- [ ] c.`
  ordinals.
- Given `/speccy-amend` editing a SPEC.md whose
  `## Open Questions` already uses alpha-prefix and contains
  `- [ ] a.` through `- [ ] d.`, when the amendment appends a new
  question, then the new row uses `- [ ] e.` (the next free
  ordinal); existing letters are not renumbered.
- Given `/speccy-brainstorm` presenting the four artifacts with
  two open questions, when the agent renders the open-questions
  artifact in chat, then the bullets use `- [ ] a.` and
  `- [ ] b.`.
- Given a brainstorm session that reaches a 26th open question
  needing `- [ ] z.`, when the agent renders the open-questions
  artifact, then the artifact body includes a prose flag (per
  the template guidance) that 26 open questions signals
  over-scoped session.
</behavior>

<scenario id="CHK-011">
Given each of the three ejected authoring-phase SKILL.md files
post-SPEC,
when the file body is searched for format guidance referencing
`- [ ] a.` (or equivalent alpha-prefix wording),
then each file contains the guidance.
</scenario>

<scenario id="CHK-012">
Given a tempdir SPEC.md whose `## Open Questions` section
already contains `- [ ] a.`, `- [ ] b.`, `- [ ] c.`,
when an `/speccy-amend` run executes that appends one new
question to the section,
then the resulting SPEC.md `## Open Questions` contains exactly
four rows with ordinals `a.`, `b.`, `c.`, `d.` (the existing
three preserved, the new one allocated as `d.`).
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: `/speccy-brainstorm` carries a collapse-parallels heuristic; `/speccy-plan` exercises symmetric discretion

`/speccy-brainstorm` gains a guidance line: when N restated-ask
requirements differ only by one noun, the agent may express them
as one requirement with an enumerated sub-list to reduce reader
cognitive load. The heuristic is agent discretion, not an
enforced threshold. `/speccy-plan` exercises the same discretion
at SPEC-write time: sub-bullets from a collapsed brainstorm
requirement may expand to atomic `<requirement>` blocks (when
atomicity adds reviewer-fan-out value) or stay grouped under one
`<requirement>` with a `<done-when>` bullet list (when cohesive
grouping serves the SPEC better).

<done-when>
- `resources/modules/skills/speccy-brainstorm.md` contains a
  guidance line (or short paragraph) describing the
  collapse-parallels heuristic.
- The guidance explicitly marks the heuristic as discretionary:
  "MAY" rather than "MUST", with the goal stated as reducing
  reader cognitive load.
- The guidance gives one concrete example (e.g. "if R1-R6 all
  read 'the X self-review verifies Y', collapse to a single R
  with sub-bullets a-f").
- `resources/modules/skills/speccy-plan.md` contains a parallel
  guidance line at SPEC-write time stating that sub-bulleted
  brainstorm requirements may expand to atomic
  `<requirement>` blocks or stay grouped, at agent discretion.
- Neither template enforces the heuristic; neither self-review
  surfaces "failure to collapse" or "failure to expand" as an
  issue.
</done-when>

<behavior>
- Given a `/speccy-brainstorm` session where the agent has
  drafted six restated-ask requirements all reading "the X
  self-review verifies Y", when the agent reads the heuristic
  guidance, then the agent may collapse the six into one with
  six sub-bullets (and does so when judging it improves reader
  scan-ability).
- Given a `/speccy-plan` run building a SPEC from a brainstorm
  output containing one collapsed requirement with six
  sub-bullets, when the agent reads the symmetric guidance, then
  the agent may expand to six atomic `<requirement>` blocks (if
  reviewer fan-out benefits) or keep one `<requirement>` with a
  six-bullet `<done-when>` (if cohesive grouping serves better).
- Given any of the three skill self-review or pre-check passes,
  when the pass runs against an artifact that did not collapse
  parallels (or did not expand them), then the pass does not
  surface the non-collapse / non-expansion as a semantic issue.
</behavior>

<scenario id="CHK-013">
Given the ejected `.claude/skills/speccy-brainstorm/SKILL.md`
post-SPEC,
when the file body is searched for the heuristic guidance,
then the file contains a line or paragraph using "MAY" (not
"MUST") wording and naming the cognitive-load goal as the
heuristic's purpose.
</scenario>

<scenario id="CHK-014">
Given the ejected `.claude/skills/speccy-plan/SKILL.md` post-SPEC,
when the file body is searched for the symmetric expansion
guidance,
then the file contains a line or paragraph that mirrors the
brainstorm side (MAY expand or keep grouped, agent discretion).
</scenario>

</requirement>

<requirement id="REQ-011">
### REQ-011: `/speccy-plan` skill template no longer carries an amendment branch

The `resources/modules/skills/speccy-plan.md` skill template drops
its amendment branch entirely. All amendment traffic routes through
`/speccy-amend`. The change auto-propagates to ejected
`.claude/skills/speccy-plan/SKILL.md` and
`.agents/skills/speccy-plan/SKILL.md` via SPEC-0033's eject
pipeline.

<done-when>
- `resources/modules/skills/speccy-plan.md`'s lede paragraph no
  longer references "or amends an existing one when intent
  shifts" or the SPEC-NNNN argument amendment branch.
- The "When to use" section no longer carries an "Amendment form
  (`/speccy-plan SPEC-NNNN`)" bullet.
- The "Steps" section's step 1 no longer carries the
  identify-amendment-vs-new-spec branch; step 2 no longer carries
  the `**Amendment**:` sub-branch. The remaining steps describe
  new-spec authoring only.
- The skill template's frontmatter `description:` line drops the
  "or amend an existing one when intent shifts" trigger phrase
  and the "asks to amend an existing spec by ID" trigger phrase.
- The ejected `.claude/skills/speccy-plan/SKILL.md` and
  `.agents/skills/speccy-plan/SKILL.md` reflect the upstream
  changes after the next `speccy init` run; no manual edits land
  on the ejected files.
- No `// removed for SPEC-NNNN` comments, no "now sole amendment
  path" callouts in other skill templates. The retirement is
  documented by omission.
</done-when>

<behavior>
- Given the post-amendment source tree, when a contributor opens
  `resources/modules/skills/speccy-plan.md`, then the file
  contains no amendment-branch prose in lede, "When to use", or
  "Steps".
- Given a freshly initialized workspace
  (`speccy init --host claude-code` run after this amendment
  lands), when `.claude/skills/speccy-plan/SKILL.md` is read,
  then the ejected body matches the upstream new-spec-only
  template.
- Given a user invokes `/speccy-plan SPEC-0007` (an
  amendment-form invocation), when the skill template is
  rendered to the agent, then the agent reads no amendment-branch
  guidance — `/speccy-brainstorm`'s routing list already names
  `/speccy-amend` as the amendment surface, so the agent routes
  the user accordingly.
</behavior>

<scenario id="CHK-015">
Given the post-amendment `resources/modules/skills/speccy-plan.md`,
when its body is searched for the literal substrings
"Amendment", "amend an existing", or "SPEC-NNNN argument",
then no matches appear in the lede, "When to use", or "Steps"
sections.
</scenario>

<scenario id="CHK-016">
Given the post-amendment ejected `.claude/skills/speccy-plan/SKILL.md`
and `.agents/skills/speccy-plan/SKILL.md`,
when each file's body is searched for "Amendment" or
"amend an existing",
then no matches appear in the body (matching the upstream
source).
</scenario>

</requirement>

<requirement id="REQ-012">
### REQ-012: "Greenfield" terminology removed from live workflow surfaces

The term "greenfield" is removed from live workflow surfaces
across the repo. Speccy works identically whether the project is
new or has existing code; the term carries no special meaning in
the workflow. Prose that explicitly explains Speccy's agnostic
behavior across new and existing repos may keep the term —
because the term is being denied, not claimed.

<done-when>
- `AGENTS.md`, `README.md`, `docs/ARCHITECTURE.md`,
  `resources/modules/phases/speccy-init.md`,
  `resources/modules/skills/speccy-plan.md`, and
  `speccy-cli/tests/skill_body_discovery.rs` no longer use the
  term "greenfield" as a mode, path, persona qualifier, or
  workflow descriptor.
- Prose that explicitly explains Speccy's no-distinction
  behavior — examples: the `README.md` "Speccy works identically
  whether the project is greenfield (no code yet) or brownfield"
  line, `docs/ARCHITECTURE.md`'s "no greenfield/brownfield
  mode toggle" callout, `AGENTS.md`'s "no greenfield/brownfield
  distinction" callout — retains the term; removing it would
  lose the explicit denial.
- Frozen historical SPECs, TASKS.md, REPORT.md, and evidence
  files under `.speccy/specs/NNNN-*/` are NOT edited
  (going-forward only, matching REQ-009's "no retroactive
  re-formatting" precedent).
- The `chk015_speccy_plan_uses_vacancy_not_status_for_greenfield_id`
  test in `speccy-cli/tests/skill_body_discovery.rs` is renamed
  and restructured. The current implementation partitions
  `speccy-plan.md`'s body on `body.find("**Amendment**")` to
  isolate a "greenfield section"; that anchor disappears under
  REQ-011's retirement of the amendment branch. The simplified
  test asserts `speccy vacancy --json` appears in the file and
  `speccy status --json` does not appear in the file — no
  partitioning needed.
- Ejected `.claude/skills/speccy-init/SKILL.md` and
  `.agents/skills/speccy-init/SKILL.md` reflect the upstream
  changes after the next `speccy init` run; no manual edits
  land on the ejected files.
</done-when>

<behavior>
- Given the post-amendment source tree, when each listed live
  workflow surface is searched for the literal substring
  "greenfield", then matches (if any) appear only in prose that
  explicitly denies the greenfield/brownfield distinction.
- Given `cargo test --workspace` runs against the post-amendment
  tree, when the `skill_body_discovery` test module runs, then
  the renamed test (from `chk015_*greenfield*`) passes against
  the post-REQ-011 `speccy-plan.md` body.
- Given `.speccy/specs/0033-eject-prompt-bodies/SPEC.md` (a
  frozen historical record), when the file is read post-
  amendment, then its body still contains its original uses of
  "greenfield" — historical records are not edited.
</behavior>

<scenario id="CHK-017">
Given the post-amendment `AGENTS.md`, `README.md`,
`docs/ARCHITECTURE.md`, `resources/modules/phases/speccy-init.md`,
and `resources/modules/skills/speccy-plan.md`,
when each file's body is searched for the literal substring
"greenfield",
then matches (if any) appear only in prose that explicitly
denies the greenfield/brownfield distinction.
</scenario>

<scenario id="CHK-018">
Given the post-amendment `speccy-cli/tests/skill_body_discovery.rs`,
when the file body is searched for the literal substring
"greenfield",
then no matches appear in test function names, test bodies, or
comments. The renamed test asserts the same `speccy vacancy --json`
present / `speccy status --json` absent properties without
partitioning on an "Amendment" anchor.
</scenario>

</requirement>

<requirement id="REQ-013">
### REQ-013: Document the TASKS.md output shape inside the speccy-tasks skill template

`resources/modules/phases/speccy-tasks.md` Step 2 currently describes
the `<tasks>` element but omits the three structural elements that the
TASKS.md parser requires: the YAML frontmatter block, the level-1
heading, and the space-separated (not comma-separated) `covers`
attribute form for multi-requirement tasks. Agents generating TASKS.md
from this template infer these elements from sibling files or guess —
producing TSK-004 parse errors (`InvalidCoversFormat`, missing heading)
on the resulting output. Step 2 must show a concrete example fragment
that makes all three required elements unambiguous.

<done-when>
- `resources/modules/phases/speccy-tasks.md` Step 2 contains a concrete
  example fragment that shows, at minimum:
  - Frontmatter with the keys `spec:`, `spec_hash_at_generation:`, and
    `generated_at:` (the three keys the parser requires).
  - The `# Tasks: SPEC-NNNN <title>` level-1 heading on the line
    immediately after the closing `---` of the frontmatter.
  - At least one `<task ... covers="REQ-001 REQ-002">` line demonstrating
    the multi-REQ form with single ASCII spaces (not commas) between REQ IDs.
- The example fragment appears in Step 2 (the write-TASKS.md step), not
  in a separate section or appendix — agents read the step body as the
  authoritative template.
</done-when>

<behavior>
- Given `resources/modules/phases/speccy-tasks.md` after the amendment,
  when an agent reads Step 2 while generating TASKS.md for a SPEC with
  two requirements, then the example fragment shows the agent exactly
  where the frontmatter goes, how to format the heading, and that
  `covers` uses spaces, not commas.
- Given a TASKS.md generated from the amended template,
  when `speccy check SPEC-NNNN` parses the file,
  then no TSK-004 (`InvalidCoversFormat`) lint error fires.
</behavior>

<scenario id="CHK-019">
Given `resources/modules/phases/speccy-tasks.md` after the amendment,
when the file body is searched for the literal substrings
`# Tasks: SPEC-` and `covers="REQ-001 REQ-002"`,
then both substrings appear inside the example fragment in Step 2.
</scenario>

</requirement>

## Design

### Decisions

<decision id="DEC-001">
**Self-review lives inline in three skill templates; no shared partial.**

Plan self-review (REQ-002) and amend self-review (REQ-003) share
six of their check properties identically and differ only in the
two amend-specific additions (Changelog row presence, surgical
diff shape). The intuitive factoring is a shared MiniJinja partial
under `resources/modules/skills/_partials/` that both
`speccy-plan.md` and `speccy-amend.md` include. The brainstorm
session for this SPEC explicitly rejected that factoring: both
templates carry their own ~80% overlapping copy.

Reason: premature factoring locks the two templates' check lists
together at a moment when the lists may still diverge as we
dogfood self-review. The duplication is small (~10 lines per
template), the cost of independent evolution is high if the two
templates need to diverge later, and a future amendment can
extract the partial cleanly once both templates have stabilized.
The brainstorm pre-check is structurally different from the
plan/amend self-review (four artifact properties vs six SPEC
properties), so no partial covers all three uses anyway —
factoring would help only the plan/amend pair.

Trade-off acknowledged: maintenance burden on the two-copy state.
Mitigation: a comment in each template noting the parallel copy
in the other template, so a contributor editing one is reminded
to check the other.
</decision>

<decision id="DEC-002">
**Self-review is a per-skill checkpoint, not a separate `/speccy-self-review` phase or an extension of `/speccy-review`.**

Two alternative shapes were considered: (a) a new
`/speccy-self-review` skill that fires after plan/amend/brainstorm,
and (b) extending `/speccy-review`'s multi-persona pass to also
review SPEC.md post-write. Both rejected.

A new skill violates the stay-small principle. Speccy's noun set
is five (Mission, Spec, Requirement, Task, Check); "self-review"
is not a noun, it is a checkpoint inside an existing authoring
phase. The CLI is already at seven verbs post-SPEC-0033; adding
an eighth for an inline checkpoint walks back stay-small without
gain.

Extending `/speccy-review` conflates two state machines.
`/speccy-review` fires on tasks in `state="in-review"` — a task
the implementer has finished and handed off. Self-review fires on
SPEC.md or brainstorm artifacts that have no task state at all
(the SPEC pre-dates task decomposition). Conflating them would
require `/speccy-review` to handle two distinct kinds of input
(SPEC vs task) with different trigger conditions, different
adversarial persona sets, and different outputs.

Inline placement in each authoring-phase skill is the speccy
shape: the checkpoint fires where the artifact is written, the
context for judging the check is loaded by the host (AGENTS.md
plus the just-written SPEC.md), and the failure mode (fix
mechanical / surface semantic) lives in the same skill body that
just produced the artifact. One state machine per phase.
</decision>

<decision id="DEC-003">
**Mechanical issues fix inline; semantic issues surface via fixed templates.**

obra/superpowers' brainstorming skill (the upstream this SPEC
borrows from) instructs the agent to fix any issues inline
without re-review. Speccy's "Surface unknowns; never invent"
principle pulls the other direction: the agent should not
silently rewrite a SPEC the user just approved at the framing
level.

The split — mechanical (string-matchable) fixes inline, semantic
(LLM-judged) issues surface — bridges the two. Mechanical issues
are objectively-detectable problems with the SPEC.md text or the
brainstorm artifacts that have one correct fix; silently
correcting them respects the user's time without inventing
content. Semantic issues are judgment calls (is this requirement
ambiguous? does this assumption matter?) where silently
"correcting" the agent's own judgment would erase the kind of
drift the self-review is supposed to catch.

The fixed-template-string rule for semantic surfacings (REQ-006)
backs this up: the agent does not write freeform prose about
what it found, it uses a literal template string. The intent is
that surfacings are recognizable, auditable, and consistent across
the three skills — a reviewer reading `## Open Questions` can
spot self-review entries by their prefix and judge them
separately from human-authored open questions.
</decision>

<decision id="DEC-004">
**Alpha-prefix `- [ ] a.` ... `- [ ] z.` for `## Open Questions`; numeric or `OQ-NNN` rejected.**

Three formats were considered: alpha-prefix (`- [ ] a.`),
numeric-prefix (`- [ ] 1.`), and full speccy-ID nomenclature
(`- [ ] **OQ-001:**`).

Numeric-prefix rejected: visually colliding with markdown's own
ordered-list syntax (`1. ...`); some renderers auto-renumber or
collapse on amend; the `1.` looks like the start of a numbered
list, not a manual ordinal label.

Speccy-ID nomenclature (`OQ-NNN`) rejected: heavier surface; open
questions are transient by nature (they get resolved into
assumptions or new requirements during normal SPEC iteration), so
giving them stable IDs over-engineers a category meant to churn.
The other speccy IDs (REQ-NNN, CHK-NNN, DEC-NNN) name things that
persist; open questions explicitly do not.

Alpha-prefix chosen for three reasons. (1) Visually distinct from
markdown's ordered-list syntax — `a.` reads as a manual label, not
auto-numbered. (2) The 26-cap doubles as a brainstorm-scope smell
detector: hitting 26 open questions signals the session is
over-scoped, not that the format is insufficient (engineering past
`z.` would mask the smell). (3) Lightweight enough to copy-paste
between brainstorm chat and SPEC.md without ceremony.
</decision>

<decision id="DEC-005">
**Collapse-parallels heuristic is discretionary at both ends; no enforced threshold.**

Two formulations of the collapse-parallels rule were considered:
(a) a hard threshold ("collapse when ≥3 parallel requirements
share a one-noun delta") with self-review surfacing failures to
collapse, and (b) agent discretion guided by cognitive-load intent
with no surfacing of non-collapse.

Hard threshold rejected: rigid count thresholds fail at edges
(two parallel requirements with a long shared phrasing are worth
collapsing; six unrelated requirements are not). The semantic
judgment — does collapsing reduce reader load here? — is what
matters. A count threshold is a proxy for the judgment, and the
proxy disagrees with the goal often enough to be net-negative.

Discretionary at both ends chosen. `/speccy-brainstorm` collapses
when the agent judges it helpful; `/speccy-plan` expands (or
keeps grouped) when the agent judges expansion helpful at the
SPEC level. Neither phase surfaces "should have collapsed" or
"should have expanded" as a self-review issue, matching speccy
principle 6: "Surface unknowns; never invent" — the heuristic is
not an unknown, it is a presentation preference.

Trade-off acknowledged: the heuristic's effect varies with
agent judgment quality. A weak agent may collapse too eagerly or
too rarely. The mitigation is the symmetry: a too-eager
brainstorm collapse can be expanded back at `/speccy-plan`, and a
too-eager plan expansion can be regrouped via amendment. Neither
direction is one-way.
</decision>

## Assumptions

<assumptions>
- SPEC-0033 (eject-prompt-bodies) is a hard sequencing predecessor;
  this SPEC's templates land in `resources/modules/skills/` only
  after that pipeline exists. SPEC-0032 (phase-model-pinning) is a
  transitive predecessor via SPEC-0033 — the model and effort pins
  on the authoring-phase skills carry through the self-review
  additions unchanged.
- "Mechanical" is defined as string-matchable from the SPEC.md text
  or brainstorm artifacts (TBD strings, missing `## Changelog`
  headers, "and"/"also" inside `<requirement>` blocks, untouched
  `<...>` placeholders, missing alpha-prefix ordinals). "Semantic"
  is defined as LLM-judged (internal contradictions, ambiguity,
  scope-creep, false-alternative framings, mechanical-filler
  assumptions). The tie-breaker rule: if judging the check
  requires reading semantics, the check is semantic.
- Surfaced semantic issues land in `## Open Questions` for plan
  and amend; the surfacing format is a literal template string
  carried in the skill template. The brainstorm pre-check
  surfaces caught issues as a fixed-template chat preamble above
  the four-artifact message (brainstorm has no SPEC.md to write
  into yet).
- Routing fidelity (one of REQ-002's six properties) covers the
  brainstorm `Open Questions` artifact too: the check verifies
  brainstorm-surfaced open questions land in SPEC.md's
  `## Open Questions` section, in alpha-prefix format. The check
  applies only when brainstorm ran for this SPEC; when brainstorm
  was skipped, scope-traces alone covers the equivalent verification
  (against the user's stated ask).
- `/speccy-review` (multi-persona) continues to run on tasks
  later in the loop, on tasks in `state="in-review"`. Self-review
  catches translation drift (brainstorm → SPEC.md, user intent →
  diff, fuzzy ask → atomized artifacts); `/speccy-review` catches
  implementation drift (SPEC.md → code). The two are
  complementary, not overlapping.
- The three skill templates are the authoritative workflow
  surface for the `## Open Questions` format guidance and carry
  their own copies of the alpha-prefix rule lock-step. No surface
  outside the three skill templates carries authoritative format
  guidance for this slice.
- Each surfacing path carries a literal template string in its
  skill template, not freeform agent prose. This makes
  surfacings recognizable to downstream reviewers and consistent
  across the three skills.
- The collapse-parallels heuristic is heuristic guidance only;
  agent discretion to reduce reader cognitive load, not an
  enforced gate. The "differ by one noun" pattern is the typical
  trigger, not a checked precondition. Failing to collapse (or
  failing to expand) is not surfaced as a self-review issue.
- Hitting 26 open questions in a single brainstorm session is a
  scope-failure signal, not a format limitation. The alpha-prefix
  cap is intentional; engineering past `z.` would mask the
  underlying scope problem.
- Greenfield-plan and amend self-review share six of their check
  properties identically; both templates carry their own copy of
  the shared properties per DEC-001. A future amendment may
  extract a shared partial under
  `resources/modules/skills/_partials/` if the duplication
  becomes a maintenance burden.
- The ejection pipeline established by SPEC-0033 renders the
  self-review sections into both Claude Code
  (`.claude/skills/speccy-<phase>/SKILL.md`) and Codex
  (`.agents/skills/speccy-<phase>/SKILL.md` plus subagent body
  files where applicable) host packs without per-host
  customization. The self-review prose is host-agnostic; the
  only host-specific content is the existing subagent-pointer
  lines (Codex) that SPEC-0033 already handles.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-19 | human/kevin | Initial draft. Adds a self-review pass to the three authoring-phase skills (`/speccy-plan` greenfield, `/speccy-amend`, `/speccy-brainstorm`) inside their MiniJinja templates post-SPEC-0033. Mechanical issues (string-matchable: `TBD`, missing `## Changelog`, "and"/"also" inside `<requirement>`, untouched `<...>` placeholders) fix inline without surfacing; semantic issues (LLM-judged) surface via literal template strings — `- [ ] x. **Self-review caught:** {issue}` rows in `## Open Questions` for plan/amend, fixed-format chat preamble for brainstorm. Plan self-review verifies six properties (routing fidelity, atomization, scope-traces, internal consistency, no placeholder leakage, no ambiguity); amend additionally verifies Changelog row presence and surgical-diff shape; brainstorm pre-check verifies four artifact properties (atomized restated requirements, structurally distinct framings, load-bearing assumptions, shape-changing open questions). Two supporting changes ride alongside: `## Open Questions` format becomes `- [ ] a.` ... `- [ ] z.` alpha-prefix lock-step across all three skill templates (26-cap doubles as brainstorm-scope smell detector); a collapse-parallels heuristic is added to `/speccy-brainstorm` (agent discretion to group N parallel requirements differing by one noun under one requirement with sub-bullets), with symmetric expansion discretion at `/speccy-plan`. Hard-sequenced after SPEC-0033 (eject-prompt-bodies); SPEC-0032 transitive predecessor. No CLI surface change; all changes land in MiniJinja templates under `resources/modules/skills/`. obra/superpowers' brainstorming skill is the upstream inspiration for the self-review checkpoint; the speccy variant splits failure handling by mechanical/semantic to honor "Surface unknowns; never invent". |
| 2026-05-20 | human/kevin + Claude | Amendment. Three concerns. (1) Corrected the `<assumptions>` block: the false claim "PRD shape is encoded in `/speccy-plan` skill body itself" is reframed accurately — the three skill templates are the authoritative workflow surface for the `## Open Questions` alpha-prefix format guidance and carry it lock-step; no surface outside the three skill templates is named as authoritative. (2) Added REQ-011: `/speccy-plan` skill template no longer carries an amendment branch — lede, "When to use", "Steps", and frontmatter `description:` drop amendment references; all amendment traffic routes through `/speccy-amend`. The retirement is documented by omission per stay-small (no `// removed` comments, no "sole amendment path" callouts elsewhere). Auto-propagates to ejected `.claude/skills/speccy-plan/SKILL.md` and `.agents/skills/speccy-plan/SKILL.md` via SPEC-0033's eject pipeline. (3) Added REQ-012: "greenfield" terminology removed from live workflow surfaces (`AGENTS.md`, `README.md`, `docs/ARCHITECTURE.md`, `resources/modules/phases/speccy-init.md`, `resources/modules/skills/speccy-plan.md`, `speccy-cli/tests/skill_body_discovery.rs`), with an explicit carve-out for prose that denies the greenfield/brownfield distinction (the term may stay where it is being denied, not claimed). Frozen historical SPECs under `.speccy/specs/NNNN-*/` are explicitly out of scope (going-forward only, per REQ-009's precedent). The `chk015_*greenfield*` test gets renamed and simplified since its `**Amendment**` partition anchor disappears under REQ-011. Editorial cleanup also drops "greenfield" qualifiers from SPEC-0034's own user-stories and REQ prose where they no longer disambiguate against a non-existent non-greenfield mode. REQ-009's lock-step surface count stays at 3 (the three skill templates). |
| 2026-05-20 | human/kevin + Claude | Amendment. Added REQ-013: document the TASKS.md output shape inside the `speccy-tasks` skill template. Root cause of the amendment: during the SPEC-0034 implementation loop, `/speccy-tasks` consistently produced malformed TASKS.md output — the required `# Tasks: SPEC-NNNN <title>` level-1 heading was omitted (TSK-004 parse error), `covers` attributes used commas instead of the required single ASCII spaces (`InvalidCoversFormat`), and the frontmatter keys (`spec:`, `spec_hash_at_generation:`, `generated_at:`) were absent. Root cause traced to Step 2 of `resources/modules/phases/speccy-tasks.md` which only named the `<tasks>` element without a concrete example fragment showing all three required elements. REQ-013 requires Step 2 to contain a concrete example fragment covering frontmatter, the level-1 heading, and the space-separated multi-REQ `covers` form. CHK-019 asserts both literal substrings `# Tasks: SPEC-` and `covers="REQ-001 REQ-002"` appear in Step 2 post-amendment. CHK-020 (ejected SKILL.md same property post-reinit) deferred to implementation. |
</changelog>

## Open Questions

- [ ] a. Exact prose of the fixed chat-preamble template string
  for `/speccy-brainstorm` semantic surfacings. The SPEC names
  the shape (verbatim opening line, bullet list of caught issues,
  verbatim closing line) but defers the literal opening/closing
  lines to decomposition. Candidate: opening
  "**Self-review caught the following before presenting
  artifacts:**", closing "Proceeding with the four artifacts
  below." — decompose-time decision.
- [ ] b. Whether to factor a shared partial under
  `resources/modules/skills/_partials/` for the six shared
  plan/amend check properties at decomposition time, despite the
  brainstorm-level rejection of premature factoring. Watch the
  duplication weight during implementation; if both copies stay
  ~10 lines and diverge in zero places, the rejection holds.
  If they grow or diverge, revisit.
- [ ] c. Whether the brainstorm pre-check fires on amendments
  routed through `/speccy-brainstorm` (some amendments are
  themselves fuzzy and brainstorm before
  `/speccy-amend` runs). REQ-001 implies the pre-check fires on
  every brainstorm invocation regardless of downstream path; the
  amendment-via-brainstorm case is just one more invocation.
  Decompose-time confirmation.
- [ ] d. Whether the `speccy-amend` recipe should become
  single-pass (retire the `### Loop exit criteria` shape and hand
  off hash recording to `/speccy-tasks` exclusively). Identified
  during T-002 review as out of scope for SPEC-0034; defer to a
  follow-up SPEC or amendment if dogfooding surfaces the need.
