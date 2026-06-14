
# {{ cmd_prefix }}speccy-plan

Drafts a new `SPEC.md` from the `AGENTS.md` product north star. The
host harness auto-loads `AGENTS.md` (which carries the project-wide
product north star); this recipe walks the agent through writing
SPEC.md. Top-level intent surfaces (`<goals>`, `<non-goals>`,
`<user-stories>`, optional `<assumptions>`) and per-requirement
sub-sections (`<done-when>`, `<behavior>`, `<scenario>`) live as raw
XML element blocks inside SPEC.md itself.

## When to use

When starting a new spec slice. If the ask is still fuzzy, run
`{{ cmd_prefix }}speccy-brainstorm` first to atomize the intent —
this skill writes SPEC.md in a single pass and assumes the framing
is already agreed.

## What to consider

- Bounded scope. One SPEC must answer one product question; refuse
  to bundle unrelated work. If the scope is too large to be tested
  end-to-end within one PR, split it.
- Decisions hidden inside requirement prose belong in `### Decisions`
  instead. Keep `<requirement>` bodies focused on observable behaviour
  and lift any architectural commitment into a `### Decisions` block.

## Steps

1. Query the next available ID:

   ```bash
   speccy vacancy --json
   ```

   The JSON's `next_spec_id` field is the allocated `SPEC-NNNN` ID.
   Decide placement: flat (`.speccy/specs/NNNN-slug/`) or under an
   existing mission folder (`.speccy/specs/[focus]/NNNN-slug/`).
   Do not invent a new mission folder for a single spec.

   Then read the backlog as candidate input. When `.speccy/BACKLOG.md`
   is present, read it and surface its entries as candidate slices for
   the spec being framed — a deferred candidate may be exactly the slice
   to draft now. Absence is normal and silent: a missing file is not an
   error, proceed without comment. If this spec promotes a backlog
   candidate, note which entry so step 5 can strike it.

2. Write SPEC.md following the PRD template.

   When the slice touches existing code, invoke the `plan-explorer`
   subagent before/while drafting to trace the relevant feature
   through its entry points, call flows, and architecture layers.
   Fold its grounding into the `## Summary` prose and the
   `<requirement>` blocks. The explorer's report is **ephemeral**: do
   NOT persist it to a new `*.md` artifact file — its only durable home
   is the existing SPEC.md sections above.

   Canonical SPEC.md shape: `references/spec.md`.

   If the brainstorm output
   contains collapsed requirements (one requirement with an enumerated
   sub-list), you MAY expand each sub-bullet to its own atomic
   `<requirement>` block (when atomicity adds reviewer-fan-out value)
   or keep them grouped under one `<requirement>` with a `<done-when>`
   bullet list (when cohesive grouping serves the SPEC better). Agent
   discretion; neither choice is surfaced as a self-review issue.

   **Record a future-spec candidate, if one was cut.** When framing
   this spec deliberately cuts a piece of scope worth its OWN later
   spec, append a candidate with provenance `SPEC-NNNN, plan`, per the
   reference below.

{% include "modules/references/backlog-ledger.md" %}

3. **Self-review pass.** Run this pass exactly once after writing
   SPEC.md. Do not re-check after applying fixes.

   {% include "modules/references/spec-self-review-core.md" %}

4. Surface any material questions inline in `## Open Questions` using
   the alpha-prefix format: `- [ ] a.`, `- [ ] b.`, ..., `- [ ] z.`.
   Each question gets the next free letter in sequence. If the section
   already exists, preserve existing ordinals and allocate the next free
   letter for any new question added (no renumbering). Reaching `z.`
   signals an over-scoped session — 26 open questions is a scope smell,
   not a format limitation.

5. Strike a promoted backlog candidate, if this spec promoted one.
   When the spec just drafted IS a backlog candidate promoted into its
   own SPEC (noted at step 1), delete that entry from
   `.speccy/BACKLOG.md` outright. No struck-through line, no "promoted
   to SPEC-NNNN" residue — git history and the new SPEC's own provenance
   are the trail, and the backlog stays a live list of current
   candidates only. If no candidate was promoted, skip this step.

6. Branch-guard, then commit `SPEC.md` alone. After the self-review
   pass completes, commit the just-written `SPEC.md` so a
   `{{ cmd_prefix }}speccy-plan` run-then-stop leaves `SPEC.md` already
   committed. The commit covers only the spec's `SPEC.md` —
   `TASKS.md` is committed by `{{ cmd_prefix }}speccy-decompose`, not
   here, so the new-spec path lands two separate commits (one per
   skill).

   First run the branch-guard prelude so the commit lands on a feature
   branch rather than the repository's default branch. Supply the
   prelude's one parameter — the **spec directory** (`<spec-dir>/`,
   i.e. the path that holds `SPEC.md`) — and run it:

{% include "modules/references/branch-guard.md" %}

   Then run the shared commit recipe, supplying its two
   behaviour-varying parameters as follows:

   - **Staging breadth: narrow `git add <spec-dir>/SPEC.md`, plus
     `.speccy/BACKLOG.md` when it is dirty.** Stage the spec's
     `SPEC.md`, and the backlog file too when this loop touched it —
     a step-2 append, a step-5 strike, or an entry the preceding
     `{{ cmd_prefix }}speccy-brainstorm` appended and left for this
     commit to sweep up — under the existence guard below. Stage
     nothing else. Do not use `git add -A` or `git add .`.
   - **Title and body.**
     - **Title:** `[SPEC-NNNN]: create spec` with `SPEC-NNNN`
       substituted for the resolved spec id.
     - **Body:** the trimmed value of the `title:` field from SPEC.md's
       YAML frontmatter (the one-line title slug, not the full document
       heading).

{% include "modules/references/backlog-staging.md" %}

   With those two parameters fixed, run the shared recipe — it defines
   the no-git short-circuit, the unified stage-then-`git diff --cached
   --quiet` idempotency check (an unchanged `SPEC.md` skips the commit
   silently), the `Co-Authored-By` trailer, and the HEREDOC commit
   mechanics:

{% include "modules/references/commit-recipe.md" %}

## Exit

`SPEC.md` is written, self-reviewed once, and committed alone on a feature
branch — `TASKS.md` is `{{ cmd_prefix }}speccy-decompose`'s commit, not this
skill's, so the new-spec path lands two commits (one per skill). Single pass,
no loop. Next step: `{{ cmd_prefix }}speccy-decompose SPEC-NNNN` to decompose
into `TASKS.md`.
