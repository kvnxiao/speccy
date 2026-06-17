
# {{ cmd_prefix }}speccy-brainstorm

Atomizes a fuzzy ask into first-principle requirements **before** any
SPEC.md is written. Walks the user through a Socratic exchange:
explore project context (and split an over-scoped ask before refining
it), ask clarifying questions one at a time, then present a single
**framing brief** — restated ask, atomized requirements, the framing
you recommend with the alternatives you rejected, and one decisions
table the user answers by key. Stop at a hard gate until the user
approves. Only then does the agent invoke
`{{ cmd_prefix }}speccy-plan` to write SPEC.md.

The output is **ephemeral chat**: salient outputs flow into SPEC.md's
existing sections when `{{ cmd_prefix }}speccy-plan` runs next (see
"Routing" below). The one exception is a deliberately deferred
future-spec candidate, appended to `.speccy/BACKLOG.md` at step 7.

## When to use

- Before drafting a new SPEC slice, whenever the ask is fuzzy or the
  framing has not yet been agreed in the chat. If the user already
  has a sharp, named slice with clear scope, skip brainstorming and
  go straight to `{{ cmd_prefix }}speccy-plan`.
- Optionally before an amendment, when the intent shift is itself
  fuzzy. The amendment path rarely needs it — the existing SPEC locks
  the framing — so reach for it on judgment.

## Hard gate

**Do NOT invoke `{{ cmd_prefix }}speccy-plan` and do NOT write
SPEC.md until the user has approved the framing.** Brainstorming
catches framing drift while edits are still cheap (pre-Requirement);
skipping the gate defeats it. If the user explicitly says "skip the
brainstorm, just write the SPEC", treat that as approval to bypass and
invoke `{{ cmd_prefix }}speccy-plan` directly — but never bypass on
your own judgment.

## Steps

1. **Explore project context.** The `AGENTS.md` product north star is
   already in your context — lean on it to frame the ask. If the ask
   might overlap with an existing slice, query the workspace index:

   ```bash
   speccy status --json
   ```

   When the ask touches existing code, invoke the `plan-explorer`
   subagent to trace the relevant feature through its entry points,
   call flows, and architecture layers. Its grounding report is
   **ephemeral**: fold the salient findings into your clarifying
   questions now, and route them into SPEC.md's existing sections when
   `{{ cmd_prefix }}speccy-plan` runs next (see "Routing"). Never write
   it to a standalone `*.md` report file.

   Then read `.speccy/BACKLOG.md` if present, and fold its entries into
   the framings you propose at step 3 — a deferred candidate may be
   exactly the slice the user is now reaching for. Absence is normal
   and silent: a missing file is not an error.

   Don't dump the context back at the user — use it to ground your
   clarifying questions.

   **Scope check (early exit).** Before refining detail, judge whether
   the ask is one slice or several. If it describes multiple
   independent subsystems, say so now and help the user decompose it:
   what the independent pieces are, how they relate, what order to
   build them. Brainstorm only the first slice through the steps
   below; record each deferred sub-slice as a future-spec candidate at
   step 7. One SPEC stays one coherent slice — refining questions
   against a multi-subsystem ask wastes the exchange.

2. **Ask clarifying questions, one at a time.** One question per
   message. Default to multiple-choice: when the answer space is
   enumerable, offer the options through the host's structured-choice
   affordance if it has one, else as a short lettered list in prose —
   a concrete menu is faster to answer and pins the answer space. Name
   the option you recommend and give a one-line reason inline in its
   description: with the host affordance, put it first and tag it
   recommended; in the prose fallback, append `(recommended: why)` to
   that letter. Recommend, don't decide — the user still picks.
   Reserve open-ended questions for genuinely open answer spaces.
   Don't batch three questions into one message — that nudges the user
   toward shallow answers. Keep asking until you can fill the framing
   brief in step 3 without inventing details.

3. **Present the framing brief.** Once you can state the user's
   intent in your own words, present it as ONE cohesive brief — not a
   set of parallel lists. Stay at the "what / why" level throughout —
   brainstorming is about the SPEC shape, not the code; resist diving
   into "how". The brief has four sections, in order:

   1. **What you're asking for.** A short narrative (2-4 sentences)
      restating the ask in your own words — the cohesive "what and
      why". This grounds the `## Summary` prose when
      `{{ cmd_prefix }}speccy-plan` runs next.

   2. **Requirements.** The ask broken into atomic, first-principle
      requirements, numbered. Each is one observable outcome; if you
      write "and"/"also" inside one, split it. Each typically becomes
      a `<requirement>` block in SPEC.md.

      **Collapse-parallels heuristic.** When N requirements differ
      only by one noun, you MAY express them as one requirement with
      an enumerated sub-list to reduce reader load — e.g. six
      "the X self-review verifies Y" lines (differing only in X)
      collapse to one requirement with sub-bullets a-f rather than
      repeating the identical sentence six times. Agent discretion, no
      enforced threshold; keep them atomic if collapsing doesn't
      improve readability.

   3. **How I'd frame it.** One short paragraph naming the framing you
      recommend and why, then a compact "considered & rejected" table
      — one row per alternative, with the reason you rejected it.
      Scale the row count to the ask: a trivial ask may have zero
      alternatives, a load-bearing architecture ask may need four.
      Rows must be structurally distinct framings, not paraphrases of
      the same choice.

   4. **Decisions I need from you.** ONE table merging every fork that
      would change the SPEC shape. Each row carries an alpha key
      (`a.`, `b.`, ...), the question, and your proposed default — or
      `?` when you genuinely have no default and need the user to
      decide. A row with a default is a silent assumption made
      visible; a `?` row is an open question — still name the approach
      you lean toward, and why, in the question text, but leave the
      default `?` so the user actively chooses instead of inheriting a
      silent assumption. Close the table with:
      "Reply 'go' to take the defaults, or override any row by key."
      Reaching key `z.` signals an over-scoped session — a scope
      smell, not a format limit.

4. **Pre-check pass.** Before presenting the brief, run this internal
   review pass exactly once. Do not re-check after it is presented.

   This pre-check fires on every brainstorm invocation — including
   when brainstorm is used as a front-end to amendment flows (since
   amendments routed through brainstorm are still brainstorm sessions).

   **Mechanical/semantic split.** Mechanical issues are
   string-matchable from the draft brief: `TBD`/`TODO` strings,
   "and"/"also" inside a single requirement, untouched `<...>`
   template placeholders, missing alpha keys on decision-table rows.
   Fix mechanical issues inline by revising the draft before
   presenting it — do not mention the fix in chat. If judging requires
   reading semantics, it is semantic.

   Semantic issues surface in chat as a preamble block immediately
   before the brief, using this fixed-format template:

   Opening line (verbatim, unchanged):
   `**Self-review caught the following before presenting the brief:**`

   Then one bullet per semantic issue:
   `- {issue}` (one-line description)

   Closing line (verbatim, unchanged):
   `Proceeding with the framing brief below.`

   If the pre-check finds no issues, omit the preamble block entirely
   and present the brief directly.

   **The four check properties:**

   - **Atomized requirements.** Each requirement describes one
     observable outcome; no single requirement contains "and"/"also"
     multi-outcome wording. Mechanical fix: split before presenting.

   - **Structurally distinct framings.** The rows of the "considered &
     rejected" table are genuinely distinct framings — not paraphrases
     of the same choice. Two that differ only in wording, not in
     structural consequence, should be merged or replaced.

   - **Load-bearing defaults.** Each defaulted decision row would, if
     its default is wrong, change the SPEC shape. A row whose answer
     is trivially true given the project context (e.g. "use Rust" on a
     Rust project) is filler — drop it.

   - **Shape-changing open rows.** Each `?` decision row must be
     answerable in a way that changes the SPEC shape. A question whose
     answer doesn't affect the requirements, summary, or framing is
     not load-bearing and should be dropped.

5. **Stop and wait.** After presenting the brief, **stop and wait**
   for the user to confirm, redirect, or answer the decisions table.
   Do not move on. Do not write SPEC.md. Do not invoke
   `{{ cmd_prefix }}speccy-plan`. The hard gate above applies until
   the user has responded with explicit approval.

6. **Iterate if needed.** If the user redirects or answers a decision
   row, fold the response back into the brief and re-present. A
   resolved `?` row becomes a defaulted row carrying their answer.
   Continue until the user explicitly approves the framing.

7. **Record a future-spec candidate, if one was deliberately deferred.**
   When the exchange cut a piece of scope worth its OWN later spec,
   append a candidate with provenance `SPEC-NNNN, brainstorm`, per the
   reference below. This is brainstorm's only disk write. Brainstorm
   appends but never promotes or strikes an entry (the writing skill
   owns that), and never commits: the step-8 hand-off
   (`{{ cmd_prefix }}speccy-plan` or `{{ cmd_prefix }}speccy-amend`)
   sweeps the dirty `.speccy/BACKLOG.md` into its commit.

{% include "modules/references/backlog-ledger.md" %}

8. **Invoke the writing skill.** Once the user has approved, invoke
   the right skill for the path:

   - For a **new SPEC**, invoke `{{ cmd_prefix }}speccy-plan`, which
     queries `speccy vacancy --json` to allocate the next SPEC ID and
     writes a new SPEC.md following the PRD template.

   - For an **amendment** to an existing SPEC, invoke
     `{{ cmd_prefix }}speccy-amend` (not `{{ cmd_prefix }}speccy-plan`
     directly). `{{ cmd_prefix }}speccy-amend` orchestrates the full
     amendment loop — SPEC.md edit, Changelog row, TASKS.md reconcile,
     and spec-hash re-record — so the brainstormed amendment doesn't
     drop the reconciliation steps and produce hash drift.

## Routing brainstorm outputs into SPEC.md

When `{{ cmd_prefix }}speccy-plan` runs after the brainstorm, fold
the brief's sections into the PRD template's existing sections — do
not invent a new SPEC.md section for brainstorm output:

- **What you're asking for** grounds the `## Summary` prose. Fold it
  in as a designed artifact; don't paste the brainstorm wording
  verbatim.
- Each **requirement** typically becomes a `<requirement>` block
  under `## Requirements`.
- The **decisions table** splits by row. A defaulted row is a
  resolved assumption → the `<assumptions>` block under
  `## Assumptions`. A `?` row is still open → `## Open Questions`, in
  `- [ ] a.` alpha-prefix format, re-lettered contiguously from `a.`
  (the brief's keys are an ephemeral reply handle, not SPEC ordinals).
- The **considered & rejected framings** land under `## Notes`. When a
  trade-off deserves a durable decision record, promote it to a
  `<decision>` block under `### Decisions` (DEC-NNN) instead.

## Exit

The approved framing flows into SPEC.md via the routing list above when
the writing skill runs. The skill ends by invoking
`{{ cmd_prefix }}speccy-plan` (new SPEC) or
`{{ cmd_prefix }}speccy-amend` (existing SPEC); on the new-SPEC path,
`{{ cmd_prefix }}speccy-plan` carries through
`{{ cmd_prefix }}speccy-decompose` to the pre-loop checkpoint on its
own. Single pass, no loop.
