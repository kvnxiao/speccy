---
name: speccy-brainstorm
description: Atomize a fuzzy ask into first-principle requirements before any SPEC.md is written. Walks the user through a Socratic exchange (one question at a time, 2-3 alternative framings with trade-offs, silent assumptions, open questions) and stops at a hard gate until the user explicitly approves the framing. Use when the user has a fuzzy idea, says "help me think about", "brainstorm with me", "I want to spec out X but I'm not sure where to start", or before invoking speccy-plan on an unclear ask.
---

# speccy-brainstorm

Atomizes a fuzzy ask into first-principle requirements **before** any
SPEC.md is written. Walks the user through a Socratic exchange:
explore project context, ask clarifying questions one at a time,
propose 2-3 alternative framings with trade-offs, surface silent
assumptions and open questions, then stop and wait for explicit user
approval. Only after the user approves the framing does the agent
invoke `speccy-plan` to write SPEC.md.

The output of this skill is **ephemeral chat**: nothing is written to
disk. Salient outputs flow into SPEC.md's existing sections when
`speccy-plan` runs next (see "Routing" below).
Inspired by the obra/superpowers brainstorming skill, trimmed to
Speccy's stay-small principles.

## When to use

- Before drafting a new SPEC slice, whenever the ask is fuzzy or the
  framing has not yet been agreed in the chat. If the user already
  has a sharp, named slice with clear scope, skip brainstorming and
  go straight to `speccy-plan`.
- Optionally before an amendment, when the intent shift behind the
  amendment is itself fuzzy. In practice the amendment path rarely
  needs brainstorming because the framing is locked in by the
  existing SPEC; the skill is framing-agnostic but reaches for it on
  judgment.

## Hard gate

**Do NOT invoke `speccy-plan` and do NOT write
SPEC.md until the user has approved the framing.** This is a hard
gate. The whole point of brainstorming is to catch framing drift
while edits are still cheap (pre-Requirement); skipping the gate
defeats the skill. If the user explicitly says "skip the brainstorm,
just write the SPEC", treat that as approval to bypass and invoke
`speccy-plan` directly — but do not bypass on your
own judgment.

## Steps

1. **Explore project context.** Read `AGENTS.md` (the host harness
   auto-loads it; re-read on demand via your Read primitive). Scan
   `.speccy/specs/` if the ask might overlap with an existing slice.
   Don't dump the context back at the user — use it to ground your
   clarifying questions.

2. **Ask clarifying questions, one at a time.** One question per
   message. Multiple-choice questions are preferred when the answer
   space is enumerable; open-ended is fine when it isn't. Don't
   batch three questions into one message — that nudges the user
   toward shallow answers. Keep asking until you can produce the
   four artifacts in step 3 without inventing details.

3. **Produce four artifacts.** Once you can answer the user's intent
   in your own words, present these four artifacts in one message:

   1. **Restated ask, atomized.** Restate the user's ask in your own
      words, broken into atomic, first-principle requirements. Each
      requirement should be one sentence describing one observable
      outcome. If you find yourself writing "and" or "also" inside a
      single requirement, split it. This is the slice's
      pre-Requirement skeleton — when `speccy-plan`
      runs next, each bullet here typically becomes a `<requirement>`
      block in SPEC.md.

   2. **2-3 alternative framings.** List 2-3 alternative framings of
      the ask, with trade-offs. The `2-3` is soft guidance — scale
      to the ask's complexity. A trivial ask may surface zero
      alternatives; a load-bearing architecture ask may need four.
      For each framing, include:
      - a one-sentence sketch of what the SPEC would look like under
        that framing; and
      - the reason you rejected it in favor of the chosen framing.

   3. **Silent assumptions.** List the assumptions you would
      otherwise bake into the SPEC without naming. Pick the
      assumptions that, if wrong, would change the SPEC shape rather
      than mechanical filler ("the project will use Rust" on a Rust
      project is true but useless).

   4. **Open questions.** List the open questions whose answers
      would change the SPEC shape. Use the `- [ ]` checkbox format
      that matches the PRD template's `## Open Questions` section,
      so the output is copy-pasteable into the eventual SPEC.md.

4. **Stop and wait.** After presenting the four artifacts, **stop
   and wait** for the user to confirm, redirect, or answer the open
   questions. Do not move on. Do not write SPEC.md. Do not invoke
   `speccy-plan`. The hard gate above applies until
   the user has responded with explicit approval.

5. **Iterate if needed.** If the user redirects or answers an open
   question, fold the response back into the four artifacts and
   re-present. Continue until the user explicitly approves the
   framing.

6. **Invoke the writing skill.** Once the user has approved, invoke
   the right skill for the path:

   - For a **new SPEC**, invoke `speccy-plan`, which
     renders the new-spec prompt:

     ```bash
     speccy plan
     ```

   - For an **amendment** to an existing SPEC, invoke
     `speccy-amend` (not `speccy-plan`
     directly). `speccy-amend` orchestrates the full
     amendment loop — SPEC.md edit, Changelog row, TASKS.md reconcile,
     and spec-hash re-record — so the brainstormed amendment doesn't
     drop the reconciliation steps and produce hash drift.

   The brainstorm chat is ephemeral — nothing was written to disk
   during steps 1-5. The salient outputs flow into SPEC.md via the
   routing list below when the writing prompt runs.

## Routing brainstorm outputs into SPEC.md

When `speccy-plan` runs after the brainstorm, fold
the four artifacts into the PRD template's existing sections — do
not invent a new SPEC.md section for brainstorm output:

- The **restated ask** informs the `## Summary` prose. Fold it in
  as a designed artifact; do not paste the brainstorm wording
  verbatim. Each atomized requirement typically becomes a
  `<requirement>` block under `## Requirements`.
- The **silent assumptions** land in the `<assumptions>` element
  block under `## Assumptions`.
- The **open questions** land under `## Open Questions` in the same
  `- [ ]` checkbox format the brainstorm produced.
- The **rejected alternative framings** land under `## Notes`. When
  a trade-off is load-bearing enough to deserve a durable decision
  record, promote it to a `<decision>` element block under
  `### Decisions` (DEC-NNN) instead.

## Key principles

- **One question at a time.** Don't overwhelm the user with batched
  questions. Multiple-choice is preferred when answers are
  enumerable.
- **Scale to complexity.** The `2-3` alternative-framings count is
  soft guidance. Trivial asks surface fewer artifacts; load-bearing
  asks surface more.
- **Atomize ruthlessly.** If a requirement reads "do X and Y", split
  it. Each requirement should describe one observable outcome.
- **No premature implementation.** Brainstorming is about the SPEC
  shape, not the code. Stay at the "what / why" level; resist
  diving into "how".
- **Hard gate before SPEC.md.** Do not invoke
  `speccy-plan` until the user has explicitly
  approved the framing.

This recipe does not loop. After invoking
`speccy-plan` (new SPEC) or
`speccy-amend` (existing SPEC), the next step is
`speccy-tasks SPEC-NNNN` (decompose into TASKS.md).
