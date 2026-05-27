
# {{ cmd_prefix }}speccy-init

{% if host == "claude-code" %}Bootstraps a Speccy workspace in three steps: scaffold `.speccy/`,
copy the Claude Code skill pack into `.claude/skills/`, and (if
needed) seed the product north star into the project's root
`AGENTS.md`.

## When to use

Run once per project, before any other Speccy slash-command. Re-run
with `--force` after upgrading `speccy` to refresh shipped recipes.
`speccy init` only ever touches files it ships; user-authored skill
files in `.claude/skills/` are left alone.{% else %}Bootstraps a Speccy workspace in three steps: scaffold `.speccy/`,
copy the Codex skill pack into `.agents/skills/`, and (if needed)
seed the product north star into the project's root `AGENTS.md`.

## When to use

Run once per project, before any other Speccy skill. Re-run with
`--force` after upgrading `speccy` to refresh shipped recipes.
`speccy init` only ever touches files it ships; user-authored skill
files in `.agents/skills/` are left alone.{% endif %}

## Steps

1. **Scaffold the workspace.** Run the CLI:

   ```bash
   speccy init
   ```

   If `.speccy/` already exists, ask the user whether to pass
   `--force` to refresh shipped files in place. Re-run as needed.

2. **Read the plan summary.** The CLI prints every file it will
   `create` or `overwrite`, then writes them. There is no
   "preserve" category: Speccy never plans writes against
   user-authored files in the host skill directory — they are
   simply not enumerated.

3. **Inspect `AGENTS.md` at the repo root and decide per-section.**
   `/speccy-init` seeds two independent sections — `## Product north
   star` and `## Speccy conventions`. Make the two seeding decisions
   independently per the AGENTS.md state matrix: north-star
   (present / absent) × conventions (present / absent), four cells.

   - **North star — absent.** Run the Q&A flow (step 4) and write
     the `## Product north star` section. Equivalent headings like
     `## Mission`, `## Product`, or `## Vision` count as present —
     do not duplicate.
   - **North star — present.** Skip the Q&A entirely. Confirm with
     the user that the existing content is current and continue
     without modification (freeze-on-first-write — the section
     carries user-authored prose that must not be stomped on
     re-run).
   - **Conventions — absent.** Append the canonical body (step 5).
   - **Conventions — present.** Replace the body verbatim (step 5).

   The four cells of the matrix are the four combinations: (north
   star absent + conventions absent) runs both Q&A and the append
   path; (north star absent + conventions present) runs Q&A then the
   replace path; (north star present + conventions absent) skips Q&A
   and runs the append path; (north star present + conventions
   present) skips Q&A and runs the replace path.

   **Missing-file path.** When `AGENTS.md` is missing entirely
   (first init, or the user deleted it between invocations),
   re-bootstrap from scratch — run the Q&A and write a fresh file
   with both sections. Do not warn, refuse, or special-case "user
   deleted AGENTS.md after a prior init" as a regression; the skill
   silently re-bootstraps.

   The two seeding decisions are independent: the state of one
   section does not bias the treatment of the other. In particular,
   the conventions upsert (step 5) runs on every invocation
   regardless of whether the north-star Q&A ran or was skipped.
   Never overwrite `## Product north star` content; the conventions
   section is the only section the skill replaces in place.

4. **North-star adaptive flow (states A and B — north-star absent).**
   Drive the `## Product north star` section as a brainstorm-style
   adaptive iteration over its five subsections in template order:
   opening prose (the project description and motivation paragraph),
   `### Users`, `### V1.0 outcome`, `### Quality bar`, and
   `### Known unknowns`. Do not run a fixed question script.

   **Inspect the repo first.** Before asking the user anything, read
   the obvious context the working tree already carries: any
   top-level `README.md`, manifest files (`Cargo.toml`,
   `package.json`, `pyproject.toml`, `go.mod`, etc.), the top-level
   source layout, and any prose already in `AGENTS.md`. Use the
   inspection to gauge per-subsection legibility — which of the five
   subsections you can plausibly draft from context, and which
   require eliciting answers from the user.

   **Per-subsection draft-or-Socratic decision.** Walk the five
   subsections in the documented order. For each one, decide
   independently:

   - **Legible from repo context → draft and confirm.** Draft the
     subsection from the inspected context and present it to the
     user for confirmation. Iterate on user redirects until the
     subsection is approved.
   - **Not legible → one-at-a-time Socratic Q&A.** Ask one question
     at a time. Prefer multiple-choice framings when the answer
     space is enumerable (e.g. "Is this primarily for: (a) solo
     developers, (b) a small team, (c) a public audience, (d)
     other — please describe?"); fall back to open prose only when
     enumeration genuinely doesn't fit. Iterate until the
     subsection is user-approved.

   These brainstorm-style patterns — one question at a time,
   multiple-choice when enumerable, draft-and-confirm, hard gate
   before write — are inlined here deliberately. Do not invoke
   `/speccy-brainstorm` or any other sub-skill from this path;
   `/speccy-init` stays self-contained.

   **Hard gate before write.** Do not write the `## Product north
   star` section to `AGENTS.md` until every one of the five
   subsections is explicitly user-approved. Partial drafts stay in
   the conversation; only the fully approved composition lands on
   disk. User redirects mid-iteration return to the relevant
   subsection rather than restarting the whole flow.

   When all five subsections are approved, compose them into the
   `## Product north star` section under the subheadings
   `### Users`, `### V1.0 outcome`, `### Quality bar`, and
   `### Known unknowns`, with the opening prose at the section root.
   Non-goals belong as prose at the section root or under an
   optional `### Non-goals` subsection if they surfaced during
   iteration. Constraints should reference the project's existing
   `## Core principles` / `## Standard hygiene` if present.

   **Asymmetry vs. the conventions upsert (step 5).** The north
   star is freeze-on-first-write because its content is
   user-authored — once the user has approved a draft and it lands,
   re-runs of `/speccy-init` must not stomp it (step 3, State C
   path). The `## Speccy conventions` section is the opposite:
   canonical boilerplate sourced from upstream, safe to refresh
   verbatim on every invocation. The asymmetry is principled —
   each section's update policy follows from who owns its content.

5. **Upsert the `## Speccy conventions` section.** After the
   north-star step completes (whether the Q&A ran or was skipped),
   perform a deterministic upsert on the `## Speccy conventions`
   section using the heading boundary as the delimiter:

   - **Heading absent.** Append the canonical body (with its
     `## Speccy conventions` heading) to the end of `AGENTS.md`.
   - **Heading present.** Replace everything from the
     `## Speccy conventions` heading down to (but not including)
     the next top-level `##` heading — or to end of file if no
     subsequent top-level heading exists — with the canonical
     body. Content under sibling `##` headings (before or after
     the conventions section) is left byte-identical.

   The replace path runs on every invocation. There is no "section
   already matches canonical body, skip" optimization — the
   canonical body is rewritten verbatim every time so upstream
   prose refreshes propagate after a Speccy upgrade.

   The heading boundary is the sole delimiter. Do not introduce
   HTML comment markers (`<!-- speccy:conventions:start -->` or
   similar) to fence the region — the heading plus the preamble
   line inside the canonical body make the upsert contract
   visible without machine-readable markers.

   The canonical body is the literal content below, expanded from
   the shared reference module at render time:

   {% include "modules/references/agents-md-speccy-conventions.md" %}

   Use that body verbatim (heading and all) when writing or
   replacing the section. Do not paraphrase, reorder subsections,
   or add project-specific bullets — project-specific additions
   belong in a sibling section per the preamble line.

6. **Report.** Tell the user what was scaffolded, what was added
   to `AGENTS.md` (if anything), and the final counts (`N
   created, N overwritten`).

7. **Suggest the next step.** `{{ cmd_prefix }}speccy-plan` to draft the first
   SPEC slice from the now-populated north star.

This recipe does not loop. The bootstrap runs once; subsequent
re-runs of `speccy init --force` only refresh the
shipped skill files and re-upsert the `## Speccy conventions`
section. The `## Product north star` section is never overwritten
once written.
