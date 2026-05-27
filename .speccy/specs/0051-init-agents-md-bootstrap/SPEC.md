---
id: SPEC-0051
slug: init-agents-md-bootstrap
title: "`/speccy-init` AGENTS.md bootstrap — seeds `## Speccy conventions` section and refactors north star Q&A to a brainstorm-style adaptive flow"
status: in-progress
created: 2026-05-27
supersedes: []
---

# SPEC-0051: `/speccy-init` AGENTS.md bootstrap — seeds `## Speccy conventions` section and refactors north star Q&A to a brainstorm-style adaptive flow

## Summary

Today's `/speccy-init` skill (`resources/modules/phases/speccy-init.md`)
seeds exactly one section into the target repo's `AGENTS.md`: the
`## Product north star`. It does so via a fixed seven-question Q&A
regardless of what the repo already tells the agent. Two consequences
follow.

First, agents working in a repo bootstrapped by `/speccy-init` lack
the operational context that Speccy's own `AGENTS.md` provides — when
to invoke each shipped skill, the Plan → Tasks → Impl → Review → Ship
loop, the journal/blockers mechanism, the no-vacuous-tests rule, the
`Co-Authored-By` commit trailer. Downstream Speccy users are forced to
either re-discover these conventions or hand-copy from Speccy's own
repo. The "useful for my next project" quality bar in this repo's
own product north star is not yet met for AGENTS.md content.

Second, the fixed seven-question Q&A grills users on questions a
legible repo (README, manifest files, existing prose) often already
answers. The friction is highest for the brownfield case the skill
explicitly supports (per AGENTS.md Core principle #5 — Speccy works
identically in any project state), where the repo already has rich
context the agent should draft from rather than re-elicit.

This SPEC extends the skill body to seed a second canonical section,
`## Speccy conventions`, carrying the load-bearing meta content
(when-to-use-which-skill, dev-loop pointer, test hygiene, commit
hygiene, CI-gate suggestion). The section is upserted on every
`/speccy-init` run: appended if absent, body replaced verbatim if
present — heading boundary serves as the upsert delimiter. The
canonical body lives in a new reference module under
`resources/modules/references/` so `just reeject` propagates upstream
edits into every host pack.

The same skill body also refactors the north star Q&A flow. Today's
fixed seven-question script is replaced with a brainstorm-style
adaptive flow: inspect the repo first, draft per-subsection from
context where the repo is legible, fall back to one-at-a-time Socratic
Q&A where it is not, walk the subsections in template order, gate
the write on explicit per-subsection approval. The north star
section keeps its current freeze-on-first-write semantics — the
asymmetry vs. the always-upsert conventions section is principled
because the north star carries user content while the conventions
section is canonical boilerplate.

This is a skill-body-plus-new-reference-module change. The Rust CLI
is unchanged. Verification is the standard hygiene suite plus a
dogfood pass exercising `/speccy-init` against a fresh test repo
state.

## Goals

<goals>
- After `/speccy-init` runs on a repo whose `AGENTS.md` lacks the
  section, `AGENTS.md` contains a `## Speccy conventions` section
  with the canonical body (preamble, when-to-use-which-skill,
  dev-loop pointer, test hygiene, commit hygiene, CI-gate
  suggestion).
- On `/speccy-init` re-run after a Speccy upgrade, the existing
  `## Speccy conventions` body is replaced verbatim with the
  upstream-canonical body; user-authored text outside the section
  (and the heading itself) is preserved.
- The canonical `## Speccy conventions` body is sourced from a
  single reference module under `resources/modules/references/`;
  `just reeject` propagates the latest body into every host pack
  (`.claude/`, `.agents/`, `.codex/`).
- After `/speccy-init` runs on a repo whose `AGENTS.md` lacks a
  `## Product north star` section, the agent walks the user through
  the section's subsections in template order (opening prose →
  Users → V1.0 outcome → Quality bar → Known unknowns), drafting
  from repo context where legible and falling back to Socratic Q&A
  where it is not, and writes the section only after the user has
  approved every subsection.
- On `/speccy-init` re-run when `## Product north star` is already
  present, the agent leaves the section alone (no re-elicitation,
  no overwrite) — preserving today's freeze-on-first-write
  behavior.
- The standard four-gate hygiene suite passes after the skill-body
  edits and `just reeject`.
</goals>

## Non-goals

<non-goals>
- No Rust CLI changes. The `speccy init` binary still scaffolds
  `.speccy/` and ejects skill packs; it does not gain any
  knowledge of `AGENTS.md` content per Core principle #2
  ("Deterministic core, intelligent edges").
- No new `speccy verify` lint codes for AGENTS.md content. The
  upsert contract is enforced socially via the skill body and
  reviewer judgment; no schema or lint surface gates the section
  body shape.
- No platform-specific CI wiring shipped with the conventions
  body. The CI-gate suggestion subsection mentions `speccy verify`
  is designed to run as a CI gate but does not include GitHub
  Actions YAML, GitLab CI YAML, Jenkinsfile snippets, or any
  other vendor-specific scaffolding.
- No marker-comment fencing of the upserted section. The `##`
  heading is the boundary; the skill replaces everything from the
  heading to the next top-level `##` (or end of file).
- No detection of "user deleted AGENTS.md after first init" as a
  special-cased regression. Re-runs are silently idempotent — if
  the file is missing, the skill re-bootstraps from scratch.
- No split into two skills (`/speccy-init-north-star` plus
  `/speccy-init-conventions`). The bootstrap stays a single
  command.
- No cross-skill dispatch from `/speccy-init` into
  `/speccy-brainstorm`. The brainstorm-style adaptive Q&A
  patterns are reimplemented inline in the init skill body per
  DEC-002 ("Stay small").
- No backward-compatibility preservation of the fixed
  seven-question Q&A. The new adaptive flow replaces it entirely;
  the seven conceptual buckets survive as the section ordering,
  but the rigid script does not.
</non-goals>

## User Stories

<user-stories>
- As a solo developer bootstrapping Speccy in a new repo, I want
  `/speccy-init` to seed both my project's product north star and
  a Speccy conventions section into `AGENTS.md` in one pass, so
  future agent sessions load the operational context (when to
  invoke each skill, test hygiene, journal mechanism) without me
  hand-copying it from Speccy's own repo.
- As a Speccy user who just upgraded the binary and skill packs,
  I want re-running `/speccy-init` to refresh the
  `## Speccy conventions` section in `AGENTS.md` so my agents
  pick up newly shipped skills and refined rules automatically,
  without me having to hand-merge upstream prose changes.
- As a developer running `/speccy-init` in a repo with a clear
  README and manifest files, I want the agent to draft my product
  north star from repo context and present it for confirmation
  rather than grilling me with seven generic questions whose
  answers are already legible from the working tree.
- As a developer running `/speccy-init` in an empty or unclear
  repo, I want the agent to walk me through the north star
  subsections one at a time with Socratic questions (multi-choice
  where the answer space is enumerable) so I can capture intent I
  have not yet written down.
- As an agent reading `AGENTS.md` in a Speccy-bootstrapped repo
  before doing work, I want a stable canonical heading
  (`## Speccy conventions`) at which to find the load-bearing
  meta content, so I can quote rules to the user (e.g. the
  no-vacuous-tests rule) without having to grep Speccy's own
  repo.
</user-stories>

## Assumptions

<assumptions>
- The shipped skill set is stable enough that refreshing the
  "when to use which skill" subsection on every `/speccy-init`
  re-run is desirable rather than disruptive. Downstream users
  benefit from new skills landing in their `AGENTS.md`
  automatically after a Speccy upgrade; users who do not want
  the refresh can pin to a Speccy release and skip re-running
  `/speccy-init`.
- `AGENTS.md` (not `CLAUDE.md`) is the canonical target file
  for both sections. The `CLAUDE.md`-as-symlink convention is
  the user's responsibility; the skill does not write to
  `CLAUDE.md` directly.
- The Claude Code and Codex host packs seed identical
  `## Speccy conventions` content — no host-specific divergence
  in the Speccy-meta body itself. (The skill body's existing
  host-specific divergence about path naming under `.claude/`
  vs. `.agents/` for the skill pack scaffolding remains the
  only conditional in the rendered text.)
- "Repo legible enough to draft" is an agent judgment, not an
  algorithmic test. The skill body instructs the agent what to
  inspect (README, manifest files, top-level source structure,
  existing prose in `AGENTS.md` if any); the agent decides
  per-subsection whether to draft or fall back to Socratic Q&A.
- Users will not rename the canonical `## Speccy conventions`
  heading. If they do, the skill cannot find the section and
  appends a second copy on the next re-run. Accepted as a v1
  edge case; not designed for.
- The north star subsection ordering (opening prose → Users →
  V1.0 outcome → Quality bar → Known unknowns) matches today's
  template structure. This ordering is load-bearing for the
  section-by-section approval flow; reshuffling the ordering
  would require a parallel SPEC.
- Brainstorm-style patterns (one question at a time,
  multi-choice when enumerable, draft-and-confirm, hard gate)
  are general enough to apply to whole-project north star
  scoping in addition to SPEC-slice framing. Reusing the
  patterns inline in `/speccy-init` does not require
  borrowing brainstorm's four-artifact contract (restated ask,
  alternative framings, silent assumptions, open questions),
  which is scoped to pre-Requirement SPEC framing.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: New reference module carries the canonical `## Speccy conventions` body

A new file exists at
`resources/modules/references/agents-md-speccy-conventions.md`
carrying the canonical body of the `## Speccy conventions` section.
The body opens with a one-line preamble making the upsert contract
visible (e.g. "Managed by `/speccy-init`; edits inside this section
are overwritten on re-run. Put project-specific additions in a
sibling section."). The body contains five subsections in order:
when-to-use-which-skill (one-liner per shipped skill), the dev loop
pointer (Plan → Tasks → Impl → Review → Ship with the
journal/blockers file location), test hygiene (the no-vacuous-tests
rule with all five generic anti-patterns plus the no-flake-retry
rule), commit hygiene (`Co-Authored-By` trailer + narrow commits),
and a CI-gate suggestion (one paragraph noting `speccy verify` is
designed as a CI gate and the user should wire it up in whichever CI
they use). The body contains no language-specific examples (no
`is_ok()`, no `unwrap()` references) and no platform-specific CI
wiring.

<done-when>
- `resources/modules/references/agents-md-speccy-conventions.md`
  exists and is non-empty.
- The file opens with a one-line preamble that names
  `/speccy-init` as the section's manager and tells users edits
  will be overwritten on re-run.
- The when-to-use-which-skill subsection contains a one-liner
  for each of: `/speccy-init`, `/speccy-brainstorm`,
  `/speccy-plan`, `/speccy-amend`, `/speccy-decompose`,
  `/speccy-work`, `/speccy-review`, `/speccy-vet`,
  `/speccy-ship`, `/speccy-orchestrate`.
- The dev-loop subsection names the five phases in order (Plan,
  Tasks, Impl, Review, Ship) and points at the journal file
  path (`.speccy/specs/NNNN-slug/journal/T-NNN.md`) where
  `<implementer>`, `<review>`, and `<blockers>` blocks live.
- The test-hygiene subsection enumerates all five vacuous-test
  anti-patterns (substring-matching curated prose, copying
  production constants, file-existence-only asserts,
  mock-then-assert-mock-called, loose-outcome assertions) in
  language-agnostic phrasing and includes the
  investigate-flakes-don't-retry rule.
- The commit-hygiene subsection states the `Co-Authored-By`
  trailer expectation for AI commits and the preference for
  narrow, well-scoped commits.
- The CI-gate-suggestion subsection mentions `speccy verify` as
  a CI gate the user may wire up and uses platform-agnostic
  phrasing (e.g. "GitHub Actions, GitLab CI, Jenkins, etc.")
  without shipping any vendor-specific configuration.
- The body contains no Rust-specific examples; the test-hygiene
  anti-patterns are phrased in language-neutral terms.
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands, when a
  reviewer reads
  `resources/modules/references/agents-md-speccy-conventions.md`,
  then the reviewer finds the preamble line followed by five
  named subsections in the order specified above.
- Given the same tree, when a reviewer searches the body for
  language-specific anti-pattern names (`is_ok()`, `unwrap()`,
  `expect()`), then no matches appear in the reference module
  body (these terms may still appear in this repo's own
  `AGENTS.md` and are not removed there).
- Given the same tree, when a reviewer reads the CI-gate
  subsection, then the prose names `speccy verify` as the gate
  and mentions multiple CI platforms by name without shipping
  configuration for any of them.
</behavior>

<scenario id="CHK-001">
Given the working tree at HEAD after this SPEC lands,
when a reviewer audits
`resources/modules/references/agents-md-speccy-conventions.md`,
then the file exists, opens with the upsert-contract preamble,
carries the five named subsections in the specified order, and
each subsection meets its done-when criteria above. Reviewer
judgment confirms language-agnostic phrasing and absence of
vendor-specific CI configuration.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: Skill body upserts the `## Speccy conventions` section into `AGENTS.md`

The `/speccy-init` skill body
(`resources/modules/phases/speccy-init.md`) instructs the agent to
perform a deterministic upsert on the `## Speccy conventions`
section after the existing scaffolding and north-star steps. The
skill body includes the canonical reference module via MiniJinja
`{% include "modules/references/agents-md-speccy-conventions.md" %}`
so that `just reeject` propagates upstream edits into every host
pack. The upsert logic is: if the heading `## Speccy conventions`
is absent from `AGENTS.md`, append the canonical body (with
heading); if the heading is present, replace everything from the
heading to the next top-level `##` heading (or end of file) with
the canonical body.

<done-when>
- `resources/modules/phases/speccy-init.md` carries explicit
  instructions to the agent for detecting and upserting the
  `## Speccy conventions` section.
- The skill body uses
  `{% include "modules/references/agents-md-speccy-conventions.md" %}`
  to pull the canonical body into the rendered prompt.
- The skill body specifies the heading boundary as the upsert
  delimiter (replace from `## Speccy conventions` to the next
  top-level `##` heading or end of file).
- The skill body explicitly states that the upsert runs on
  every invocation — there is no detection of "section already
  matches canonical body, skip" optimization.
- The skill body instructs the agent to make the two seeding
  decisions (north star and conventions) independently — either
  section may exist or not when the skill runs.
</done-when>

<behavior>
- Given a target repo whose `AGENTS.md` lacks a
  `## Speccy conventions` heading, when an operator runs
  `/speccy-init`, then `AGENTS.md` gains the section with the
  canonical body appended.
- Given a target repo whose `AGENTS.md` already contains a
  `## Speccy conventions` heading with arbitrary body text (e.g.
  from a prior Speccy version with different prose), when an
  operator runs `/speccy-init`, then the section body is
  replaced verbatim with the current upstream-canonical body
  while content outside the section (and the heading itself) is
  preserved.
- Given a target repo whose `AGENTS.md` contains both a
  `## Speccy conventions` section and unrelated content after it,
  when `/speccy-init` runs, then the section body is replaced
  but the unrelated content (under a sibling `##` heading) is
  preserved.
</behavior>

<scenario id="CHK-002">
Given a fresh test repo with an `AGENTS.md` that contains a
`## Product north star` section but no `## Speccy conventions`
section,
when an operator runs `/speccy-init` and the skill executes its
upsert step,
then `AGENTS.md` gains a `## Speccy conventions` section appended
after the existing content, the section body matches the
canonical body rendered from
`modules/references/agents-md-speccy-conventions.md`, and the
pre-existing `## Product north star` section is untouched.
</scenario>

<scenario id="CHK-003">
Given a fresh test repo with an `AGENTS.md` that already contains
a `## Speccy conventions` section (with deliberately stale body
text — e.g. "old conventions placeholder") followed by a sibling
`## My project notes` section with user-authored prose,
when an operator runs `/speccy-init` and the skill executes its
upsert step,
then the `## Speccy conventions` body is replaced verbatim with
the upstream-canonical body, the `## My project notes` section
remains byte-identical to its pre-run state, and no second copy
of `## Speccy conventions` is appended.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Skill body's north-star Q&A becomes a brainstorm-style adaptive flow

The `/speccy-init` skill body
(`resources/modules/phases/speccy-init.md`) replaces today's fixed
seven-question script with a brainstorm-style adaptive flow for
the `## Product north star` section. The new flow instructs the
agent to: (a) first inspect the repo (README, manifest files like
`Cargo.toml` / `package.json` / `pyproject.toml`, top-level source
structure, any existing `AGENTS.md` prose) to gauge legibility;
(b) walk the user through the section's subsections in template
order — opening prose, `### Users`, `### V1.0 outcome`,
`### Quality bar`, `### Known unknowns`; (c) for each subsection,
draft from repo context when legible and present for confirmation,
or fall back to one-at-a-time Socratic Q&A (multi-choice when
answers are enumerable) when it is not; (d) gate the write on
explicit per-subsection approval — do not write the section until
every subsection is approved. The flow borrows brainstorm-style
patterns inline; it does not dispatch to `/speccy-brainstorm`.

<done-when>
- `resources/modules/phases/speccy-init.md` no longer contains
  the fixed seven-question script ("What are we building, and
  why does it matter?", "Who will use it?", "What does 'done
  enough to ship v1' look like?", etc.).
- The skill body instructs the agent to inspect the repo
  (README, manifest files, top-level source structure, existing
  `AGENTS.md` prose) before deciding draft-vs-Socratic per
  subsection.
- The skill body names the five subsections in template order
  (opening prose, Users, V1.0 outcome, Quality bar, Known
  unknowns) as the iteration order for approval.
- The skill body instructs the agent to draft from repo context
  when the subsection's content is legible and to fall back to
  one-at-a-time Socratic questions (multi-choice when
  enumerable) when it is not.
- The skill body specifies a hard gate: the agent does not
  write `## Product north star` until every subsection is
  user-approved.
- The skill body does not call `/speccy-brainstorm` or any
  other sub-skill for the north-star path; brainstorm-style
  patterns are inlined.
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands, when a
  reviewer reads `resources/modules/phases/speccy-init.md`,
  then the reviewer finds the adaptive flow described above and
  does not find the seven-question script.
- Given a target repo with a clear README and manifest file,
  when an operator runs `/speccy-init` and the skill reaches
  the north-star step, then the agent inspects the repo,
  drafts each subsection from context, and presents the draft
  for confirmation rather than asking the seven generic
  questions.
- Given a target repo with no README and no manifest file (or
  with deliberately ambiguous content), when an operator runs
  `/speccy-init` and the skill reaches the north-star step,
  then the agent falls back to one-at-a-time Socratic Q&A for
  each subsection.
</behavior>

<scenario id="CHK-004">
Given the working tree at HEAD after this SPEC lands,
when a reviewer audits `resources/modules/phases/speccy-init.md`
for the north-star flow,
then the body describes a five-subsection adaptive iteration in
the documented template order with a per-subsection
draft-or-Socratic decision and a hard gate against writing the
section before all subsections are approved. The fixed
seven-question script from the prior body is absent. Reviewer
judgment confirms the absence of cross-skill dispatch into
`/speccy-brainstorm`.
</scenario>

<scenario id="CHK-005">
Given a fresh test repo with a clear `README.md` (e.g. a
two-paragraph description of a hypothetical product) and a
populated manifest file (`Cargo.toml`, `package.json`, or
`pyproject.toml`),
when an operator runs `/speccy-init` and the skill reaches the
north-star step,
then the agent inspects the repo before asking any question, drafts
at least one subsection from the inspected context, and presents
the draft for user confirmation. The fixed seven-question
script does not run.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: `## Product north star` freeze-on-first-write semantics preserved

The `/speccy-init` skill body preserves today's freeze-on-first-write
behavior for the `## Product north star` section. When `AGENTS.md`
already contains a `## Product north star` heading on re-run, the
skill leaves the section alone — no re-elicitation, no overwrite,
no diff prompt. The asymmetry vs. the always-upsert
`## Speccy conventions` section (REQ-002) is principled: the north
star carries user-authored content from the Q&A pass, while the
conventions section is canonical boilerplate.

<done-when>
- `resources/modules/phases/speccy-init.md` retains a
  freeze-on-first-write branch for the `## Product north star`
  section — the agent confirms the existing section is current
  and continues without modification when the heading is
  already present.
- The skill body does not introduce any path that overwrites,
  diffs against, or re-elicits an existing
  `## Product north star` section on re-run.
- The skill body documents the asymmetry explicitly so a
  reader understands why north star is freeze-on-first-write
  while conventions is always-upsert.
</done-when>

<behavior>
- Given a target repo whose `AGENTS.md` already contains a
  `## Product north star` section (from any prior init pass or
  hand-authored), when an operator re-runs `/speccy-init`, then
  the agent skips the north-star Q&A entirely and proceeds to
  the conventions upsert step.
- Given the same target repo, after re-run, the
  `## Product north star` section body is byte-identical to its
  pre-run state.
</behavior>

<scenario id="CHK-006">
Given a fresh test repo whose `AGENTS.md` already contains a
`## Product north star` section with deliberately distinctive
content (e.g. an unusual project description not derivable from
any defaults),
when an operator re-runs `/speccy-init`,
then after the run the `## Product north star` section body
remains byte-identical to its pre-run state, no north-star
Q&A questions are asked, and the skill proceeds to upsert the
`## Speccy conventions` section.
</scenario>
</requirement>

<requirement id="REQ-005">
### REQ-005: Idempotent re-run across the AGENTS.md state matrix

The `/speccy-init` skill is idempotent on every invocation across
the full state matrix of `AGENTS.md`: file missing, file present
without north star, file present with north star, file present
without conventions, file present with conventions, and any
combination thereof. The two seeding decisions (north star and
conventions) are made independently per the state of their
respective sections. There is no detection of "user deleted
`AGENTS.md`" as a special-cased regression — the skill silently
re-bootstraps from whatever state it finds.

<done-when>
- `resources/modules/phases/speccy-init.md` describes the state
  matrix explicitly: north star (present / absent) × conventions
  (present / absent), with the action per cell specified.
- The two seeding decisions are made independently; the
  skill body does not couple them (e.g. "if north star is
  present, skip conventions too").
- No branch in the skill body warns about, refuses, or
  otherwise special-cases the "`AGENTS.md` missing after prior
  init" state. The missing-file path simply re-bootstraps from
  scratch.
- Running `/speccy-init` twice in succession on the same target
  repo leaves the second run's `AGENTS.md` byte-identical to
  the first run's `AGENTS.md` (modulo canonical-body refreshes
  that are upstream-driven, not target-driven).
</done-when>

<behavior>
- Given a target repo whose `AGENTS.md` is missing entirely,
  when an operator runs `/speccy-init`, then the agent runs
  the full north-star adaptive flow and appends the
  conventions section in a single pass.
- Given a target repo whose `AGENTS.md` is present with
  `## Product north star` but no `## Speccy conventions`, when
  an operator runs `/speccy-init`, then the agent skips the
  north-star step (freeze-on-first-write) and appends only the
  conventions section.
- Given a target repo whose `AGENTS.md` is present with
  `## Speccy conventions` but no `## Product north star`, when
  an operator runs `/speccy-init`, then the agent runs the
  north-star adaptive flow and replaces the conventions
  section body verbatim from the canonical reference.
- Given a target repo whose `AGENTS.md` is present with both
  sections, when an operator runs `/speccy-init`, then the
  agent skips the north-star step and refreshes only the
  conventions section body.
- Given that a user deleted `AGENTS.md` between two
  `/speccy-init` invocations, when the second invocation runs,
  then the skill re-bootstraps as if it were a first invocation
  — no warning, no detection, no refusal.
</behavior>

<scenario id="CHK-007">
Given a fresh test repo,
when an operator runs `/speccy-init` twice in succession with
no intervening user edits and no Speccy binary upgrade,
then the second run's `AGENTS.md` is byte-identical to the
first run's `AGENTS.md`. Reviewer judgment confirms the skill
body's state matrix description is consistent with the observed
idempotent behavior.
</scenario>

<scenario id="CHK-008">
Given a fresh test repo where `AGENTS.md` carries a
`## Speccy conventions` section but no `## Product north star`
section (a deliberately constructed state matrix corner),
when an operator runs `/speccy-init`,
then the agent runs the north-star adaptive flow (producing the
section) and refreshes the conventions section body — both
seeding decisions are made independently. The post-run
`AGENTS.md` contains both sections.
</scenario>
</requirement>

<requirement id="REQ-006">
### REQ-006: Ejected packs carry the updated skill body and canonical reference

After `just reeject`, the host packs under `.claude/`, `.agents/`,
and `.codex/` carry the post-amendment skill body (the adaptive
north-star flow and the conventions upsert step) and the canonical
`## Speccy conventions` body. The Rust CLI is untouched — render
plumbing is purely additive through new module file plus
`{% include %}` directive.

<done-when>
- After `just reeject`, `.claude/skills/speccy-init/SKILL.md` and
  the corresponding `.agents/` file contain the new adaptive
  north-star flow and the conventions upsert step.
- After `just reeject`, the canonical conventions body (from
  `modules/references/agents-md-speccy-conventions.md`) is
  expanded into every ejected `speccy-init` skill body via the
  `{% include %}` directive.
- The `speccy init` Rust CLI binary surface is unchanged: no new
  flags, no new subcommands, no new behavior in
  `speccy-cli/src/`.
- The skill description frontmatter (the `description:` field
  in each `SKILL.md.tmpl` wrapper) is updated to mention that
  the skill seeds both the north star and the `## Speccy
  conventions` section, so host-side skill-routing prompts
  reflect the new scope.
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands and
  `just reeject` has run, when a reviewer reads
  `.claude/skills/speccy-init/SKILL.md`, then the body carries
  the adaptive north-star flow, the conventions upsert step,
  and the inlined canonical conventions body.
- Given the same state, when a reviewer reads the equivalent
  `.agents/skills/speccy-init/SKILL.md`, then the body carries
  the same content (modulo host-specific path-naming
  conditionals already present in the source module).
- Given the same state, when a reviewer inspects
  `speccy-cli/src/` for changes attributable to this SPEC,
  then no Rust source files have been modified.
</behavior>

<scenario id="CHK-009">
Given the working tree at HEAD after this SPEC lands and
`just reeject` has run,
when a reviewer audits the ejected skill bodies
(`.claude/skills/speccy-init/SKILL.md` and
`.agents/skills/speccy-init/SKILL.md`),
then each file contains the adaptive north-star flow described
in REQ-003, the conventions upsert step described in REQ-002,
and the canonical conventions body (expanded from the
`{% include %}` directive). Reviewer judgment confirms the
ejected content matches the source under
`resources/modules/`.
</scenario>

<scenario id="CHK-010">
Given the same working tree,
when a reviewer compares `speccy-cli/src/` before and after the
SPEC's changes via `git diff --stat`,
then no `.rs` files appear in the diff scoped to this SPEC.
Reviewer judgment confirms the Rust CLI surface is unchanged.
</scenario>
</requirement>

<requirement id="REQ-007">
### REQ-007: Standard hygiene gates pass after the refactor

After the source-side template edits land and `just reeject` has
run, the standard four-gate hygiene suite continues to pass.

<done-when>
- `cargo test --workspace` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  exits 0.
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0.
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands, when an
  operator runs each gate command in sequence, then each exits 0.
- Given the same tree, when CI runs the equivalent workflow,
  then the workflow passes.
</behavior>

<scenario id="CHK-011">
Given the working tree at HEAD after this SPEC lands and
`just reeject` has run,
when `cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, and `cargo deny check` each
run in sequence,
then every command exits 0 with no warnings or test failures
attributable to this SPEC.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
### DEC-001: AGENTS.md prose seeding stays in the skill layer, not the CLI

The Rust CLI is the wrong home for `AGENTS.md` prose composition
and upsert logic. Speccy's `## Core principles` §2 — "Deterministic
core, intelligent edges. The CLI is mechanical: renders prompts,
queries state, runs checks. […] The Rust CLI does not call LLMs."
— forbids the CLI from owning prose-writing responsibilities.
Prose composition is intelligent-edge work; the agent doing the
work has more context (the user's chat, the repo's existing
content, judgment about legibility) than the deterministic CLI can
reasonably encode.

The CLI's contribution to AGENTS.md bootstrap remains zero: it
scaffolds `.speccy/`, ejects skill packs, and exits. The
`/speccy-init` skill is what reads `AGENTS.md`, asks the user
questions, drafts prose, and writes the result.

The rejected alternative was a CLI-owned write path — e.g. a
`speccy init --seed-agents-md` flag that wrote both sections
deterministically from a fixed template. Rejected because (a) the
north-star section requires user input the CLI cannot collect
non-interactively without re-implementing chat-style UX in Rust,
and (b) the conventions section's "always-upsert" behavior could
in principle be CLI-owned but would split bootstrap into two
parallel write paths (Rust for conventions, skill for north star)
when one unified skill-layer path is simpler.
</decision>

<decision id="DEC-002">
### DEC-002: `/speccy-init` inlines brainstorm-style patterns rather than dispatching to `/speccy-brainstorm`

The `/speccy-init` skill body adopts brainstorm-style Q&A
patterns (one question at a time, multi-choice when enumerable,
draft-and-confirm, hard gate before writing) inline. It does not
dispatch to `/speccy-brainstorm` as a sub-skill.

The reasoning is twofold. First, `/speccy-brainstorm` is scoped
to SPEC slices — its four-artifact contract (restated ask,
alternative framings, silent assumptions, open questions) is
designed for pre-Requirement framing of a single feature slice,
not for whole-project north star scoping. Reusing the contract
verbatim would force-fit subsections like "alternative framings"
onto north-star elicitation where the framing question (what is
the whole project?) is not naturally framed as "two-or-three
alternative SPEC shapes". Second, AGENTS.md `## Core principles`
§5 ("Stay small. […] no orchestration runtime") prefers a flat
skill surface to cross-skill dispatch. Inlining the patterns
keeps `/speccy-init` self-contained.

The rejected alternative was a "brainstorm-as-sub-skill" path
that would have `/speccy-init` invoke `/speccy-brainstorm` for
the north-star Q&A. Rejected per the reasoning above. The
brainstorm-style patterns the init skill borrows are documented
as patterns (in this SPEC's REQ-003) rather than as a shared
helper module, on the assumption that the two skills' Q&A
flavors diverge enough over time that a forced abstraction
would be more friction than dedup.
</decision>

<decision id="DEC-003">
### DEC-003: Heading boundary is sufficient for the conventions upsert; no marker comments

The `## Speccy conventions` heading is the upsert boundary. The
skill replaces everything from that heading to the next top-level
`##` heading (or end of file). No HTML comment markers
(`<!-- speccy:conventions:start -->` / `:end -->`) fence the
region.

Markers add cosmetic noise to the rendered `AGENTS.md`, can
interact unpredictably with markdown linters and tooling, and
imply a more sophisticated lifecycle than the upsert contract
needs. The heading already serves as a clear, human-readable
delimiter; the preamble line inside the section makes the upsert
contract visible without machine-readable markers.

The rejected alternative was the fenced-region pattern. Rejected
because heading boundary covers all the use cases without the
visual and tooling cost of markers.
</decision>

<decision id="DEC-004">
### DEC-004: Single `/speccy-init` skill, not split into two

The bootstrap is a single user-facing event — "set up Speccy in
this repo" — and stays a single skill. The skill internally
performs two independent decisions (north star seeding and
conventions seeding) but does not split into
`/speccy-init-north-star` plus `/speccy-init-conventions`.

The rejected alternative was the split-skill path. Rejected
because users should not have to remember two commands for one
bootstrap event, and host-side skill-routing prompts work better
with one well-described skill than two narrowly-named siblings
that always co-occur.
</decision>

<decision id="DEC-005">
### DEC-005: CI-gate mention is platform-agnostic; no vendor-specific wiring shipped

The conventions section's CI-gate subsection mentions
`speccy verify` as a CI gate the user may wire up, names example
platforms in prose ("GitHub Actions, GitLab CI, Jenkins, etc."),
and ships no platform-specific configuration files or YAML
templates.

The reasoning is that Speccy cannot reliably know which CI
service a downstream user is on; shipping a GitHub Actions
workflow into a GitLab project (or vice versa) creates dead
configuration that confuses agents and users alike. The
suggestion is framed as guidance ("designed to run as a CI
gate"), not enforcement.

The rejected alternative was shipping a `.github/workflows/`
template alongside the canonical body, possibly with multiple
opt-in files for each major CI platform. Rejected per the
reasoning above and per the "Stay small" principle.
</decision>

## Open Questions

All open questions from the `/speccy-brainstorm` session
(`a` through `e`) were resolved before this SPEC was drafted.
No outstanding questions remain at draft time.

## Notes

The `## Product north star` flow refactor (REQ-003) intentionally
preserves the freeze-on-first-write semantic (REQ-004) rather than
introducing an upsert path symmetric with the conventions section.
The asymmetry traces to content ownership: the north star is
user-authored prose captured via Q&A; replacing it would stomp
project-specific intent. The conventions body is canonical
boilerplate sourced from upstream; replacing it is the desired
refresh path. If users want north-star refresh, they should edit
the section in place; the skill stays out of the way.

The `<implementer>` should treat the canonical `## Speccy
conventions` body's prose as content the user (and downstream
agents) will read frequently. Generic boilerplate that an agent
would skim past is less useful than tight, evocative phrasing
that lands. The five-anti-pattern enumeration in test hygiene is
worth particular care — phrasing must be language-agnostic
without losing the diagnostic value the Rust-specific phrasings
in Speccy's own `AGENTS.md` carry.

The implementer drafting the canonical reference module may
shorten or rephrase the four-section bullet structure if a
meaningful clarity improvement is possible — the done-when items
in REQ-001 specify content shape, not exact wording. The
implementer should not, however, drop any of the five
anti-patterns from test hygiene or drop the no-flake-retry rule.

The verification scenarios under each requirement use
reviewer-audit framing rather than automated prose-substring
matching, per AGENTS.md § "Conventions for AI agents specifically"
no-vacuous-tests rule. Automated grep over canonical conventions
prose would gate editorial decisions rather than the upsert
behavior; the SPEC enforces shape socially via review judgment and
via dogfood passes on test repos.

Three alternative framings were considered and rejected during
`/speccy-brainstorm`:

- **CLI-owned writes** — `speccy init` writes both sections
  deterministically. Rejected per DEC-001.
- **Marker-fenced section** — wrap the conventions body in HTML
  comment markers. Rejected per DEC-003.
- **Split into two skills** (`/speccy-init-north-star` plus
  `/speccy-init-conventions`). Rejected per DEC-004.
- **Per-platform CI wiring shipped** (GitHub Actions YAML, GitLab
  template, Jenkinsfile). Rejected per DEC-005.
- **Cross-skill dispatch into `/speccy-brainstorm` for the
  north-star path**. Rejected per DEC-002.

## Changelog

<changelog>
| Date       | Author              | Summary |
|------------|---------------------|---------|
| 2026-05-27 | claude-opus-4-7[1m] | Initial draft. Seven requirements: (REQ-001) new canonical reference module under `resources/modules/references/` carries the `## Speccy conventions` body with preamble, when-to-use-which-skill, dev-loop, test hygiene, commit hygiene, CI-gate suggestion subsections in language-agnostic phrasing; (REQ-002) skill body upserts the conventions section via heading boundary — insert if absent, replace body if present; (REQ-003) skill body's north-star Q&A becomes a brainstorm-style adaptive flow with repo-inspection-first, draft-or-Socratic per subsection, section-by-section approval, hard gate before write; (REQ-004) `## Product north star` freeze-on-first-write semantics preserved (no regression); (REQ-005) idempotent re-run across the AGENTS.md state matrix with independent seeding decisions per section; (REQ-006) ejected packs carry the updated skill body and inlined canonical body after `just reeject`, with Rust CLI surface unchanged; (REQ-007) standard hygiene gates pass. Five decisions: DEC-001 (AGENTS.md prose seeding stays in skill, not CLI, per Core principle #2); DEC-002 (`/speccy-init` inlines brainstorm-style patterns rather than dispatching to `/speccy-brainstorm`); DEC-003 (heading boundary suffices for conventions upsert; no marker comments per DEC-003); DEC-004 (single skill, not split); DEC-005 (CI mention is platform-agnostic; no vendor-specific wiring). All five `/speccy-brainstorm` open questions resolved before draft. |
</changelog>
