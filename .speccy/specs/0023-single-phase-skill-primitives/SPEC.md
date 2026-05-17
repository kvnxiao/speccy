---
id: SPEC-0023
slug: single-phase-skill-primitives
title: Single-phase skill primitives for the development loop
status: implemented
created: 2026-05-17
supersedes: []
---

# SPEC-0023: Single-phase skill primitives for the development loop

## Summary

Today the shipped `/speccy-work` and `/speccy-review` skills each
wrap an internal loop: they query `speccy next`, spawn a per-task
sub-agent, wait for it to return, then iterate until no work
remains. Implementation in practice is slow for three compounding
reasons:

1. Each spawned implementer sub-agent is a cold context; prefix
   caching never carries from one task to the next.
2. The orchestrating main agent stays alive across the whole loop,
   accumulating tool results from every spawn even though it does no
   work itself.
3. Reviewer CLI prompts inline the full branch diff per persona.
   For non-trivial branches the rendered output (~2 MB on
   `SPEC-0022/T-001`) blows past the configured 80 KB render budget
   and the CLI prints a silent guard message; nothing downstream
   notices.

The orchestration shape also entangles two responsibilities in one
skill: doing one phase, and looping over many. That makes the
skills feel like a pseudo-multi-agent runtime rather than the
single-phase primitives the architecture promises.

This spec rebases `/speccy-work` and `/speccy-review` as
single-task primitives: one invocation, one task, fresh context,
exit. The four-persona parallel fan-out inside `/speccy-review`
stays because adversarial review across fresh contexts is the
feature, but each persona sub-agent receives a bash command rather
than an inlined rendered prompt, and the rendered prompt itself
stops inlining the diff. Multi-task orchestration becomes a future
Layer-2 concern that can compose the primitives without changing
them.

This is a skill-layer and prompt-layer change. The CLI surface and
the artifact grammars (SPEC.md / TASKS.md / REPORT.md) are not
touched.

## Goals

<goals>
- `/speccy-work` and `/speccy-review` each execute exactly one task
  per invocation and exit. No internal loop over `speccy next`.
- Skill bodies describe what one fresh-context session does, with no
  "main agent" / "sub-agent" framing baked in. The same body is
  correct whether the caller is a human at the terminal, the
  existing `/loop` skill, or a future orchestrator that spawns
  Speccy primitives as sub-agents.
- The implementation step uses no sub-agent. The session that
  invokes `/speccy-work` is the implementer.
- The review step keeps the four-persona parallel fan-out as
  sub-agents (the within-task internal detail that earns the spawn
  cost), but each persona sub-agent's prompt is the bash command
  form `speccy review <id> --persona <p>` rather than an inlined
  rendered prompt.
- Reviewer CLI prompts stop inlining the branch diff. Each persona
  fetches the diff via `git diff` when it needs to.
- CLI-rendered prompts stop inlining the project's `AGENTS.md`
  context block. The host harness already auto-loads `AGENTS.md`
  (Claude Code, Codex, and other modern coding agents all read this
  file by convention; `CLAUDE.md` is often a symlink to it), so the
  CLI re-inlining is duplicate bytes in every prompt.
- CLI-rendered prompts stop inlining the full text of SPEC.md,
  TASKS.md, and MISSION.md. Each rendered prompt names the file's
  repo-relative path and instructs the agent to read it on demand
  via the host's Read primitive. The per-task `<task>` XML block
  (`{{task_entry}}`) stays inline because it is scoped to the one
  task under work; the broader artifact bodies do not.
- `.speccy/ARCHITECTURE.md` agrees with the shipped skill bodies
  about the primitive contract. The Phase 3 / Phase 4 skill-driven
  loop diagrams are deleted; a short paragraph notes that
  multi-task orchestration is a future Layer-2 concern not built
  today.
</goals>

## Non-goals

<non-goals>
- No new CLI commands. The ten-command surface is unchanged. In
  particular: no `speccy state`, no `speccy note`, no `speccy run`,
  no helper for state-attribute mutation. Skills continue to edit
  the `state="..."` attribute with the host's edit primitive, as
  they do today.
- No bundled scripts inside skill folders (`scripts/`). The
  deterministic substrate is the CLI; adding a parallel script
  layer creates two sources of truth.
- No bundled reference files inside skill folders (`references/`).
  Current skill bodies are ~40-90 lines; progressive disclosure
  earns its keep only when SKILL.md becomes hard to scan. Revisit
  if a body crosses ~200 lines.
- No `/speccy-run` (or equivalent) orchestrator skill in this
  spec. The primitive contract this spec establishes is what
  enables a future orchestrator; building both at once would
  conflate "the right primitive" with "the right composer."
- No removal of the four-persona reviewer fan-out. The fan-out is
  the only sub-agent use that remains; that is intentional.
- No change to the parallel persona set (still `business`,
  `tests`, `security`, `style` by default; `architecture` and
  `docs` remain opt-in via `--persona`).
- No change to the SPEC.md / TASKS.md / REPORT.md grammars or to
  any CLI command's external behavior beyond the prompt template
  content (reviewer prompts drop the diff inline; all prompts drop
  the `AGENTS.md` inline).
- No stable last-line stdout summary across skills. That would be
  load-bearing only for a future orchestrator and can land with
  that orchestrator.
</non-goals>

## User Stories

<user-stories>
- As a solo developer running `/speccy-work`, I want one invocation
  to focus on one task in one fresh session, so cache stays warm
  across the read/edit/test cycle and I can interrupt or redirect
  between tasks without unwinding a loop.
- As a solo developer running `/speccy-review`, I want one
  invocation to drive one round of adversarial review (four parallel
  personas on one task) and exit, so each invocation has a clear
  beginning and end.
- As a reviewer persona sub-agent, I want my CLI-rendered prompt to
  describe my job and tell me how to fetch the diff myself, rather
  than carrying a multi-megabyte inlined diff that may silently
  exceed the render budget.
- As an AI agent maintainer reading the shipped skill bodies, I want
  the text to describe what one session does without main/sub framing,
  so I can call the same skill from a fresh terminal, from a `/loop`
  wrapper, or from a future orchestrator without rewriting it.
- As an invoking agent (Claude Code, Codex, or any host that
  auto-loads `AGENTS.md`), I want CLI-rendered prompts to skip the
  `AGENTS.md` inline so my context window does not carry the same
  project conventions twice.
- As a solo developer running `/speccy-work` or `/speccy-review` on
  a mature spec, I want the rendered prompt size to scale with the
  task slice rather than the spec's accumulated history, so a SPEC.md
  that has grown over many amendments does not balloon every prompt
  the CLI renders.
- As a future maintainer adding a multi-task orchestrator, I want
  the primitive contract documented (in both the SKILL.md files and
  ARCHITECTURE.md) so I can spawn Speccy primitives as sub-agents
  without changing them.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `/speccy-work` is a single-task primitive

The `speccy-work` skill body executes one task per invocation and
exits. The skill does not loop over `speccy next` and does not
spawn an implementer sub-agent; the invoking session is the
implementer.

<done-when>
- `resources/modules/skills/speccy-work.md` describes a single-task
  primitive that accepts an optional `[SPEC-NNNN/T-NNN]` positional
  selector. With the selector, the session implements that task.
  Without the selector, the skill resolves the next implementable
  task via `speccy next --kind implement --json` and implements that
  one. Either way, the skill exits after one task.
- The skill body does not contain language that asks the invoking
  agent to spawn an implementer sub-agent, to loop over `speccy
  next`, or to "drive the implementation loop."
- The skill body is role-agnostic: it describes what one session
  does, with no "main agent" / "sub-agent" framing.
- The Claude Code wrapper at
  `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` and the
  Codex parallel at
  `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl` carry
  frontmatter `description:` text that matches the new primitive
  contract and triggers on single-task intent phrases (e.g.,
  "implement T-003", "work the next task", "run the implementer").
- The skill body still names the exit transition: after
  implementation, the session flips the task's `state="..."`
  attribute from `pending`/`in-progress` to `in-review` and appends
  an implementer note using the handoff template the CLI's
  implement prompt already supplies.
</done-when>

<behavior>
- Given the user invokes `/speccy-work SPEC-0007/T-003`, when the
  skill runs, then the session focuses on T-003 only and exits
  without continuing to the next task.
- Given the user invokes `/speccy-work` with no argument, when the
  skill runs, then it resolves the next implementable task via
  `speccy next --kind implement --json` and implements that single
  task before exiting.
- Given the skill body, when grep'd for orchestration vocabulary
  (`sub-agent`, `subagent`, `spawn`, `loop`, `until no tasks`), then
  no active guidance hits — only historical references, if any, in
  comments or changelog rows.
- Given the rendered implementer prompt (`speccy implement <id>`)
  and the new skill body, when both are read together, then the
  invoking session has everything it needs to implement the one
  task without a hand-off step.
</behavior>

<scenario id="CHK-001">
Given a `SPEC-NNNN/T-NNN` selector argument,
when `/speccy-work` runs,
then the session implements that one task and exits without
processing additional pending tasks.

Given no argument,
when `/speccy-work` runs,
then the skill resolves the next implementable task via
`speccy next --kind implement --json` and implements that one task
only.

Given the rewritten `resources/modules/skills/speccy-work.md`,
when grep'd for `sub-agent`, `subagent`, `spawn`, or `loop`,
then no active instruction hits.

Given the rewritten Claude Code and Codex wrapper frontmatter,
when read,
then the `description:` text matches the single-task contract and
does not mention sub-agent spawning or loops.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `/speccy-review` is a single-task primitive with parallel persona fan-out

The `speccy-review` skill body reviews one task per invocation and
exits. Within that one task, the skill spawns four parallel persona
sub-agents (default fan-out: business, tests, security, style). The
skill aggregates their inline notes and flips the task state to
`completed` (all pass) or `pending` with a `Retry:` note (any
blocking).

<done-when>
- `resources/modules/skills/speccy-review.md` describes a
  single-task primitive that accepts an optional `[SPEC-NNNN/T-NNN]`
  positional selector. With the selector, the session reviews that
  one task. Without the selector, the skill resolves the next
  reviewable task via `speccy next --kind review --json` and
  reviews that one. Either way, the skill exits after one task.
- The skill body does not contain language asking the invoking
  agent to loop over `speccy next` or to drive a multi-task review
  loop.
- The fan-out remains: for the one task under review, the skill
  spawns four parallel persona sub-agents using the host-native
  primitive (Claude Code `Task` tool / Codex equivalent).
- Each persona sub-agent's prompt is the bash command form `Run
  \`speccy review <SPEC-NNNN/T-NNN> --persona <persona>\` and
  follow its output. Your only deliverable is a single inline note
  appended to TASKS.md.` rather than the rendered prompt text
  inlined into the spawn call. The CLI command remains the source of
  truth for what each persona sees; this just stops the orchestrator
  from carrying the rendered prompt in its own context.
- The skill body still names the exit transitions: aggregate the
  four appended notes, flip `in-review` → `completed` if all are
  pass, or `in-review` → `pending` plus a `Retry:` note if any is
  blocking.
- The Claude Code wrapper at
  `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl` and
  the Codex parallel carry frontmatter `description:` text that
  matches the new contract and triggers on single-task review intent
  phrases.
</done-when>

<behavior>
- Given the user invokes `/speccy-review SPEC-0007/T-003`, when the
  skill runs, then four reviewer sub-agents read T-003 and append
  one inline note each before the skill aggregates and exits.
- Given the user invokes `/speccy-review` with no argument, when
  the skill runs, then it resolves the next reviewable task via
  `speccy next --kind review --json` and reviews only that one.
- Given each of the four persona notes is `pass`, when the skill
  aggregates, then T-003 transitions `in-review` → `completed`.
- Given any persona note is `blocking`, when the skill aggregates,
  then T-003 transitions `in-review` → `pending` and a `Retry:`
  bullet summarising the blockers is appended to the task subtree.
- Given each spawned persona sub-agent's prompt, when inspected,
  then it carries the bash command form (`speccy review <id>
  --persona <p>`) rather than the rendered prompt text.
</behavior>

<scenario id="CHK-002">
Given a `SPEC-NNNN/T-NNN` selector,
when `/speccy-review` runs,
then exactly one task is reviewed (one round of four parallel
persona reads) and the skill exits without processing additional
in-review tasks.

Given no argument,
when `/speccy-review` runs,
then the skill resolves the next reviewable task via
`speccy next --kind review --json` and reviews only that one.

Given each of the four spawned persona sub-agents,
when their prompt is inspected,
then it contains the bash command form and does not contain the
rendered prompt text inline.

Given four `pass` persona notes appended to one task,
when the skill aggregates,
then the task transitions `in-review` → `completed`.

Given at least one `blocking` persona note appended to one task,
when the skill aggregates,
then the task transitions `in-review` → `pending` and a `Retry:`
note summarising the blockers is appended.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Reviewer CLI prompts stop inlining the branch diff

The CLI's rendered reviewer prompt for each persona stops inlining
the branch diff. Each prompt instructs the reviewer agent to fetch
the diff via `git diff` when needed, scoped to the task's
`Suggested files` where the prompt can provide that hint.

<done-when>
- `resources/modules/prompts/reviewer-*.md` (the six persona prompt
  templates) does not interpolate `{{diff}}` or otherwise inline
  `git diff` output. Any `diff` template variable used today is
  removed from both the template and the renderer that fills it.
- Each persona prompt instead instructs the agent to run `git diff
  <merge-base>...HEAD -- <suggested-files>` (with `<merge-base>`
  resolved against the repo's main branch as defined by the host's
  remote, and `<suggested-files>` derived from the task entry).
- `resources/modules/personas/reviewer-*.md` (the persona bodies)
  is updated so the "how to read the diff" guidance matches the
  prompt: the persona fetches the diff via git, not via an inlined
  block.
- On any task in this repo's existing `.speccy/specs/`, `speccy
  review <id> --persona <p>` renders well under the configured
  render budget. The current render-budget guard (which prints
  `exceeds budget (80000 chars) after all drops` on
  `SPEC-0022/T-001`) does not trigger.
- The CLI render budget guard's behavior is unchanged; this spec
  only removes the over-budget rendering. (If we later choose to
  raise the guard from a print to an error, that lands in a
  separate spec.)
</done-when>

<behavior>
- Given any task in this repo, when `speccy review <id> --persona
  <p>` is run, then the rendered output is well below 80,000
  characters and does not contain a line beginning `diff --git`.
- Given the rendered prompt, when read, then it tells the reviewer
  agent how to fetch the diff via `git diff` and (where the prompt
  has the data) scopes the diff to the task's suggested files.
- Given any reviewer persona body and the matching reviewer prompt,
  when both are read, then they agree: the persona fetches the diff
  itself rather than expecting it inline.
</behavior>

<scenario id="CHK-003">
Given any existing task in `.speccy/specs/` in this repo,
when `speccy review <SPEC-NNNN/T-NNN> --persona <persona>` is run
for each of the four default personas,
then each rendered prompt is well under the 80,000-character
render budget and does not contain an inlined `diff --git` line.

Given each `resources/modules/prompts/reviewer-*.md` template,
when read,
then it instructs the agent to run `git diff` itself and does not
interpolate a `{{diff}}` variable or equivalent.

Given each `resources/modules/personas/reviewer-*.md` body,
when read,
then it agrees with the prompt: the persona fetches the diff via
git rather than receiving it inline.

Given the renderer code path that previously filled the diff
variable,
when this spec lands,
then that code path is removed (or the variable is no longer
referenced anywhere in the resources tree).
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Architecture docs reflect the primitive contract

`.speccy/ARCHITECTURE.md` is the design source of truth per
AGENTS.md. After this spec lands, its description of Phase 3 and
Phase 4 agrees with the shipped skill bodies: skills are
single-phase primitives; multi-task orchestration is a future
Layer-2 concern explicitly out of scope today.

<done-when>
- ARCHITECTURE.md §"Core Development Loop" describes the five
  phases without implying that phases 3 and 4 are skill-driven
  loops. Instead, those phases describe one primitive invocation
  (one task in, one state transition out).
- ARCHITECTURE.md §"Phase 3" and §"Phase 4" describe the primitive
  contract: one task per invocation, fresh-context per session, exit
  with a state transition visible in TASKS.md. The previous loop
  pseudocode in §"Phase 3" and §"Phase 4" is deleted; it described
  a skill-driven runtime that does not exist after this spec lands.
- A short paragraph (one to two sentences) at the end of §"Phase 3"
  and §"Phase 4" notes that composing multiple primitive
  invocations into a batch is a future Layer-2 concern not built
  today, and points at the existing `/loop` skill as the interim
  composer.
- The text uses no "main agent" / "sub-agent" framing for the
  primitive itself. The Phase 4 description may name the
  within-task four-persona fan-out as the one place sub-agents are
  spawned; that is intrinsic to the primitive and not orchestration.
- The skill-pack shape table (or equivalent) and the shipped
  SKILL.md bodies agree on wording. A grep for orchestration
  vocabulary across active guidance does not surface contradictions.
</done-when>

<behavior>
- Given a reader new to Speccy reads ARCHITECTURE.md, when they
  finish, then their mental model is: skills are single-phase
  primitives; multi-task orchestration is a future addition, not a
  current feature.
- Given the ARCHITECTURE.md, the rewritten `speccy-work.md` skill
  body, and the rewritten `speccy-review.md` skill body, when all
  three are read, then the texts agree on the primitive contract.
</behavior>

<scenario id="CHK-004">
Given the rewritten ARCHITECTURE.md,
when §"Phase 3" and §"Phase 4" are read,
then each describes one primitive invocation (one task in, one
state transition out) rather than a skill-driven loop, and the
previous loop pseudocode is no longer present.

Given ARCHITECTURE.md,
when grep'd for "loop", "spawn", "sub-agent", or "subagent" in the
context of Phase 3 / Phase 4,
then active guidance does not present these as the skill's job.
The Phase 4 description may name the within-task four-persona
fan-out as the one intrinsic sub-agent use.

Given ARCHITECTURE.md, the new `resources/modules/skills/speccy-work.md`,
and the new `resources/modules/skills/speccy-review.md`,
when all three are read,
then they agree on the single-task primitive contract.

Given the end of §"Phase 3" and the end of §"Phase 4",
when read,
then each carries a one-to-two-sentence note that multi-task
composition is a future Layer-2 concern not built today, pointing
at the existing `/loop` skill as the interim composer.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: CLI-rendered prompts stop inlining AGENTS.md

The CLI's prompt templates do not re-inline the project's root
`AGENTS.md` (or its `CLAUDE.md` symlink). Modern AI coding harnesses
auto-load `AGENTS.md` into every session by convention, so the CLI
inlining is duplicate bytes in every rendered prompt.

<done-when>
- Every prompt template under `resources/modules/prompts/`
  (planner, tasks, implementer, reviewer-*, report) is edited so it
  no longer interpolates the `{{agents}}` variable.
- The CLI renderer code that read `AGENTS.md` and filled the
  `{{agents}}` template variable is removed. The variable is no
  longer recognised by the renderer; the templates do not reference
  it.
- The `## Project conventions` heading (or equivalent) is removed
  from every prompt that introduced the `{{agents}}` block, so the
  rendered output is not left with an empty section.
- On any task in this repo's `.speccy/specs/`, the rendered prompts
  for `speccy plan`, `speccy plan <id>`, `speccy tasks <id>`,
  `speccy implement <id>`, `speccy review <id> --persona <p>`, and
  `speccy report <id>` do not contain the literal text of
  `AGENTS.md`'s `## Product north star` heading (the strongest
  fingerprint that the file was inlined).
</done-when>

<behavior>
- Given any of the twelve prompt templates, when read, then it does
  not reference the `{{agents}}` variable and does not carry a
  `## Project conventions` section that wrapped it.
- Given any CLI-rendered prompt after this spec lands, when grep'd
  for the `AGENTS.md` `## Product north star` heading, then no hit
  is returned.
- Given the renderer, when invoked on any prompt, then `AGENTS.md`
  is not read from disk as part of rendering. (Reading it
  separately for other purposes — `speccy init` north-star Q&A,
  for example — is unaffected.)
</behavior>

<scenario id="CHK-005">
Given each of the twelve prompt templates under
`resources/modules/prompts/`,
when grep'd for `{{agents}}` or `{{ agents }}`,
then no hit is returned.

Given the CLI renderer source after this spec lands,
when grep'd for the code path that loaded `AGENTS.md` and filled
the `{{agents}}` variable,
then the code path is removed (or the variable is no longer wired
up anywhere in the renderer).

Given any task in `.speccy/specs/` in this repo,
when `speccy plan`, `speccy plan <id>`, `speccy tasks <id>`,
`speccy implement <id>`, `speccy review <id> --persona <p>`, or
`speccy report <id>` is run,
then the rendered output does not contain the `## Product north
star` heading from `AGENTS.md`.

Given the rendered implementer prompt for any task,
when measured,
then it is smaller than the pre-spec rendered output for the same
task by at least the size of `AGENTS.md`.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: CLI-rendered prompts use file references for SPEC.md, TASKS.md, MISSION.md

The CLI's prompt templates do not inline the full text of SPEC.md,
TASKS.md, or MISSION.md. Each rendered prompt that previously
inlined these files instead names the file's repo-relative path and
instructs the agent to read it via the host's Read primitive. The
scoped per-task `<task>` block (`{{task_entry}}`) remains inline
because it is the slice under work, not an unbounded artifact body.

<done-when>
- Every prompt template under `resources/modules/prompts/` that
  inlines `{{spec_md}}` today (implementer, plan-amend, report,
  reviewer-architecture, reviewer-business, reviewer-docs,
  reviewer-security, reviewer-style, reviewer-tests, tasks-generate,
  tasks-amend; eleven templates) is edited so it no longer
  interpolates `{{spec_md}}`. The full SPEC body is replaced with a
  short instruction naming the spec's repo-relative path (e.g.,
  `Before starting, read SPEC.md at \`.speccy/specs/<slug>/SPEC.md\`.`)
  and the surrounding `## SPEC (full)` heading is removed (or
  retitled to reflect that it is a pointer, not a body).
- `resources/modules/prompts/report.md` and
  `resources/modules/prompts/tasks-amend.md` no longer interpolate
  `{{tasks_md}}`; the full TASKS body is replaced by an equivalent
  short instruction naming the file's repo-relative path.
- `resources/modules/prompts/plan-amend.md` no longer interpolates
  `{{mission}}`; the full MISSION body is replaced by an equivalent
  short instruction naming the file's repo-relative path. The
  renderer should emit no Read instruction at all when the focus has
  no `MISSION.md` (flat single-focus projects).
- The CLI renderer code paths that read SPEC.md, TASKS.md, and
  MISSION.md to fill the `{{spec_md}}`, `{{tasks_md}}`, and
  `{{mission}}` template variables are removed (or the variables
  are no longer wired up anywhere in the renderer).
- The rendered prompt names the repo-relative path explicitly so the
  agent can Read it without searching. The path is derived from the
  spec/focus the CLI is rendering for; it is not a hardcoded string.
- On any task in this repo's `.speccy/specs/`, the rendered
  implementer prompt and each rendered reviewer prompt is smaller
  than the pre-spec output for the same task by at least the size of
  `SPEC.md` (in addition to the `AGENTS.md` shrink from REQ-005).
</done-when>

<behavior>
- Given any of the eleven prompt templates that previously
  interpolated `{{spec_md}}`, when read, then they do not reference
  `{{spec_md}}` and do not carry an embedded SPEC body.
- Given `report.md` or `tasks-amend.md`, when read, then they do not
  reference `{{tasks_md}}` and do not carry an embedded TASKS body.
- Given `plan-amend.md`, when read, then it does not reference
  `{{mission}}` and does not carry an embedded MISSION body.
- Given any CLI-rendered prompt after this spec lands, when read by
  an agent, then the agent has an explicit one-line instruction
  telling it which file(s) to read and where they live on disk
  relative to the repo root.
- Given the renderer, when invoked on any prompt, then SPEC.md,
  TASKS.md, and MISSION.md are not read from disk as part of
  populating the prompt body. (Reading them separately for other
  purposes — e.g., `speccy verify` artifact checks — is unaffected.)
</behavior>

<scenario id="CHK-006">
Given each of the eleven prompt templates that previously
interpolated `{{spec_md}}`,
when grep'd for `{{spec_md}}` or `{{ spec_md }}`,
then no hit is returned.

Given `resources/modules/prompts/report.md` and
`resources/modules/prompts/tasks-amend.md`,
when grep'd for `{{tasks_md}}` or `{{ tasks_md }}`,
then no hit is returned.

Given `resources/modules/prompts/plan-amend.md`,
when grep'd for `{{mission}}` or `{{ mission }}`,
then no hit is returned.

Given the CLI renderer source after this spec lands,
when grep'd for the code paths that loaded SPEC.md, TASKS.md, and
MISSION.md to fill these template variables,
then those code paths are removed (or the variables are no longer
wired up anywhere in the renderer).

Given any task in `.speccy/specs/` in this repo,
when `speccy implement <id>` or `speccy review <id> --persona <p>`
is run,
then the rendered output contains a short instruction naming the
repo-relative path to SPEC.md and does not contain the SPEC body.

Given the rendered implementer prompt for any task,
when measured,
then it is smaller than the pre-spec rendered output for the same
task by at least the size of SPEC.md (in addition to the AGENTS.md
shrink from REQ-005).
</scenario>

</requirement>

## Design

### Approach

Implementation order:

1. Rewrite `resources/modules/skills/speccy-work.md` and
   `resources/modules/skills/speccy-review.md` as single-task
   primitives. Keep wording role-agnostic. Add explicit
   `[SPEC-NNNN/T-NNN]` selector handling.
2. Update the four wrapper templates
   (`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`,
   `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`,
   and the `.agents/` Codex parallels) to carry new frontmatter
   `description:` text matching the primitive contract.
3. Edit `resources/modules/prompts/reviewer-*.md` to remove diff
   interpolation. Edit `resources/modules/personas/reviewer-*.md`
   to describe `git diff` fetching. Remove the renderer code that
   filled the diff variable.
4. Edit every prompt template under `resources/modules/prompts/`
   (planner, tasks, implementer, reviewer-*, report) to drop the
   `{{agents}}` interpolation and the surrounding `## Project
   conventions` heading. Remove the renderer code path that loaded
   `AGENTS.md` and filled the variable.
5. Edit the eleven prompt templates that inline `{{spec_md}}`
   (implementer, plan-amend, report, all six reviewer-*,
   tasks-generate, tasks-amend) to replace the SPEC body with a
   one-line instruction naming the spec's repo-relative path. Edit
   `report.md` and `tasks-amend.md` to do the same for
   `{{tasks_md}}`. Edit `plan-amend.md` to do the same for
   `{{mission}}` (with the renderer suppressing the instruction when
   the focus has no `MISSION.md`). Remove the renderer code paths
   that loaded these files and filled the variables.
6. Update `.speccy/ARCHITECTURE.md` §"Core Development Loop",
   §"Phase 3", §"Phase 4", and any skill-pack table that names
   loop semantics. Delete the existing Phase 3 / Phase 4 loop
   pseudocode and add the short Layer-2 paragraph in its place.
7. Re-eject the host-local skill files via `cargo run -- init
   --force` in this repo so the dogfooded `.claude/skills/` and
   `.agents/skills/` reflect the new shapes.
8. Verify the render-budget guard no longer fires on the
   representative `SPEC-0022/T-001` reviewer prompt, and that the
   rendered implementer prompt for that task is smaller than the
   pre-spec output by at least the combined size of `AGENTS.md` and
   `SPEC.md`.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Skill bodies are role-agnostic

**Status:** Accepted

**Context:** Today's skill bodies use "main agent" / "sub-agent"
framing. That framing locks the skill to one invocation pattern and
makes it awkward to reuse from a future orchestrator that spawns
Speccy primitives as sub-agents (the framing collides with itself).

**Decision:** Skill text describes what one session does without
naming who invoked it. The same body must be correct whether the
caller is a human at the terminal, the existing `/loop` skill, or a
future `/speccy-run` orchestrator.

**Consequences:** Skills become composable. A future orchestrator
adds an outer loop without rewriting the primitives. The primitives
look slightly more abstract, which is the right trade.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Parallel persona fan-out stays in `/speccy-review`

**Status:** Accepted

**Context:** Considered moving to sequential personas in one session
to drop the sub-agent layer entirely. Adversarial review benefits
from fresh contexts per persona — a single agent applying four
lenses tends to converge its reasoning rather than challenge it.

**Decision:** Keep parallel persona fan-out. It is within-task and
bounded (four sub-agents, one round). It is not multi-task
orchestration.

**Consequences:** `/speccy-review` remains the only Layer-1
primitive that spawns sub-agents. The cost is justified by the
adversarial diversity the fan-out is designed to produce. If
dogfooding later shows the personas converge anyway, revisit then.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: No new CLI commands

**Status:** Accepted

**Context:** Considered adding `speccy state SPEC-NNNN/T-NNN --to
<state>` and `speccy note SPEC-NNNN/T-NNN --kind <kind>` so skills
stop fumbling XML attribute edits and note appends.

**Decision:** Defer. Agents handle these byte-level edits correctly
in practice today. The CLI surface stays at ten commands per the
"Stay small" principle.

**Consequences:** Skills continue to use the host's edit primitive
to flip state attributes and append notes. Revisit only if
dogfooding surfaces fumbles in real loops.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: No bundled scripts or references in skill folders

**Status:** Accepted

**Context:** Skill-creator best practices recommend `scripts/` for
deterministic repeated work and `references/` for verbose teaching
content. Speccy's deterministic substrate is already the CLI, and
skill bodies are 40-90 lines today — well below the threshold where
progressive disclosure earns its keep.

**Decision:** Each skill remains a single SKILL.md file. No new
folders.

**Consequences:** No script-vs-CLI source-of-truth drift. Revisit
only if a SKILL.md grows past ~200 lines or if dogfooding surfaces
a specific operation agents fumble in practice.
</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: Reviewer agents fetch the diff themselves

**Status:** Accepted

**Context:** Reviewer CLI prompts inline the full branch diff per
persona. On `SPEC-0022/T-001` this rendered ~2 MB, blowing past the
configured 80 KB render budget. The CLI prints a guard message and
continues, which is silent failure.

**Decision:** Reviewer prompts emit a one-line instruction to run
`git diff <merge-base>...HEAD -- <suggested-files>`. Persona bodies
match. The renderer removes the diff variable.

**Consequences:** Rendered prompts stay well under budget. Each
persona pays the `git diff` cost once, in its own context, where
the cost belongs. The orchestrator skill no longer carries multi-MB
prompts in its tool history.
</decision>

<decision id="DEC-006" status="accepted">
#### DEC-006: CLI prompts trust the host to auto-load AGENTS.md

**Status:** Accepted

**Context:** Every CLI-rendered prompt today inlines the project's
root `AGENTS.md` as a `## Project conventions` block. Claude Code
auto-loads `AGENTS.md` into every session (and treats `CLAUDE.md`
as a symlink to it); Codex and other modern AI coding harnesses
read the same file by the same convention. The CLI inlining is
duplicate bytes in every prompt — measurable in the implementer
prompt (~5 KB on this repo) and multiplied across every persona
fan-out in review.

**Decision:** CLI prompt templates drop the `{{agents}}`
interpolation and the surrounding `## Project conventions` heading.
The CLI renderer no longer reads `AGENTS.md` for prompt rendering.

**Consequences:** Rendered prompts shrink by the size of `AGENTS.md`
on every host. The host is now responsible for surfacing project
conventions into the agent's context — which is the existing
convention anyway. Speccy stops fighting the host's auto-loading
behaviour by re-inlining. If a future host fails to auto-load
`AGENTS.md`, that is a host-integration concern, not a CLI
concern; the response is to fix the host or document the gap, not
to re-inline.
</decision>

<decision id="DEC-007" status="accepted">
#### DEC-007: CLI prompts use file references for SPEC.md, TASKS.md, MISSION.md

**Status:** Accepted

**Context:** Beyond `AGENTS.md` (DEC-006), every CLI-rendered prompt
today inlines the full text of SPEC.md (and `report.md` /
`tasks-amend.md` additionally inline TASKS.md; `plan-amend.md`
additionally inlines MISSION.md). On a non-trivial spec, SPEC.md is
30+ KB; the implementer prompt carries it once, the reviewer fan-out
across four personas carries it four times in flight. None of these
files are auto-loaded by the host, but every modern coding agent has
a Read primitive and can fetch a file by path on demand. Inlining
means the rendered prompt grows with the spec's accumulated history
rather than the task slice.

**Decision:** CLI prompt templates drop the `{{spec_md}}`,
`{{tasks_md}}`, and `{{mission}}` interpolations. The CLI renderer
no longer reads these files for prompt rendering. Each rendered
prompt names the repo-relative path of the file(s) the agent should
read first and instructs the agent to read it. The per-task
`<task>` block (`{{task_entry}}`) stays inline — it is the slice of
work under review, and is bounded to a single task.

**Consequences:** Rendered prompts shrink to the size of the
per-task slice plus a handful of small variables, regardless of how
large the SPEC, TASKS, or MISSION files grow. Each persona in the
reviewer fan-out reads the SPEC once into its own context, where the
cost belongs — and a persona that does not need the full SPEC
(style, for example) may skim or skip it. The trade-off: the agent
must follow the Read instruction. Empirically Claude Code and Codex
agents do this reliably when the prompt names the path explicitly;
if a host fails to, that is a host-integration concern, not a CLI
concern, and the response is to make the instruction more explicit
or to fix the host, not to re-inline.
</decision>

## Open Questions

- Resolved: drop the `AGENTS.md` re-inline. The user confirmed that
  `AGENTS.md` is the standard file modern AI coding harnesses
  auto-load (Claude Code, Codex, others; `CLAUDE.md` is often a
  symlink to it). REQ-005 and DEC-006 capture the change.
- Resolved: delete the Phase 3 / Phase 4 loop pseudocode in
  `.speccy/ARCHITECTURE.md` rather than relabelling it. The
  pseudocode describes a runtime that does not exist after this
  spec lands. REQ-004's done-when reflects the deletion plus a
  short Layer-2 note pointing at the existing `/loop` skill as the
  interim composer.
- Resolved: broaden the inlining cleanup beyond `AGENTS.md` to also
  drop the full-body inlines of SPEC.md, TASKS.md, and MISSION.md.
  Same principle: host agents have a Read primitive and can fetch a
  file by repo-relative path on demand; the rendered prompt should
  carry only the per-task slice plus pointers, not the full artifact
  history. REQ-006 and DEC-007 capture the change. The scoped
  `{{task_entry}}` block stays inline because it is bounded to the
  one task under work.

## Assumptions

<assumptions>
- Claude Code's `Task` tool accepts a short bash-command-style
  prompt as input and the spawned sub-agent will run that command
  via its own Bash tool. The Codex equivalent has the same shape.
  Verified informally; this spec's persona fan-out depends on it.
- The CLI's existing `speccy next --kind {implement|review}
  --json` output is sufficient for the rewritten skills to resolve
  the next task. No new fields needed.
- The host's edit primitive (Claude Code `Edit`, Codex equivalent)
  is reliable for byte-level XML attribute edits at the scale
  Speccy uses today. Empirically true across the existing 22
  shipped specs.
- The `/loop` skill (or a shell loop) is sufficient as an interim
  multi-task composer until a Speccy-aware orchestrator lands.
  Users who want to drain a TASKS.md will type `/loop /speccy-work`.
- The host harness auto-loads the project's root `AGENTS.md` (or
  `CLAUDE.md` as a symlink to it) into every agent session. This is
  the documented behaviour of Claude Code, Codex, and other modern
  AI coding harnesses. REQ-005 depends on this; if a host fails to
  auto-load `AGENTS.md`, fixing the host or documenting the gap is
  the correct response, not re-inlining via the CLI.
- The host agent reliably reads a file by repo-relative path when
  the rendered prompt explicitly names the path and instructs the
  agent to read it. Every modern AI coding agent (Claude Code,
  Codex, others) has a Read primitive that does this. REQ-006
  depends on this; if an agent fails to follow the Read instruction,
  the response is to make the prompt instruction more explicit
  (mark it as a prerequisite, name the file first), not to
  re-inline the body.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-17 | human/kevin | Initial draft. Single-task primitives for `speccy-work` and `speccy-review`; reviewer prompts stop inlining the diff; ARCHITECTURE.md updated. CLI surface unchanged. |
| 2026-05-17 | human/kevin | Resolved both open questions in scope. Added REQ-005 / DEC-006: CLI prompts drop the `{{agents}}` re-inline because modern AI coding harnesses auto-load `AGENTS.md`. Hardened REQ-004 to delete the Phase 3 / Phase 4 loop pseudocode in `.speccy/ARCHITECTURE.md` (rather than relabel it) and require a short Layer-2 note in its place. |
| 2026-05-17 | human/kevin | Added REQ-006 / DEC-007: CLI prompts drop the `{{spec_md}}`, `{{tasks_md}}`, and `{{mission}}` inlines as well, naming the files' repo-relative paths and instructing the agent to read them on demand. Same principle as REQ-005 broadened to all large artifact bodies; the scoped per-task `{{task_entry}}` stays inline. Approach gains a new step 5 and the verification step grows to cover the SPEC.md shrink. |
</changelog>

## Notes

This spec is intentionally narrow. The earlier drafts of this work
included new CLI commands for state mutation, bundled `references/`
and `scripts/` per skill, and a `/speccy-run` orchestrator skill.
Each was cut because it solved a hypothetical rather than an
observed problem. The "Stay small" principle in AGENTS.md applies:
fix the loop slowness and the render-budget bug; defer the rest
until dogfooding earns them in.
