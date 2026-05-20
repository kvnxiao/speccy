
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

3. **Inspect `AGENTS.md` at the repo root.** Decide which of the
   three greenfield states applies:

   - **State A — AGENTS.md is missing entirely.** Run the full
     Q&A (see below) and write a new `AGENTS.md` whose first
     section is `## Product north star`.
   - **State B — AGENTS.md exists with process conventions but
     no `## Product north star` section (or equivalent heading
     like `## Mission`, `## Product`, `## Vision`).** Run the
     narrower Q&A (see below) and *append* a `## Product north
     star` section.
   - **State C — AGENTS.md already has a product north star
     section.** Skip the Q&A. Confirm with the user that the
     existing content is current and continue.

   In all three states, never overwrite existing AGENTS.md
   content. The skill appends or stops.

4. **Greenfield Q&A (states A and B).** Ask the user, one
   question at a time, capturing the answers as bullet lists or
   short paragraphs. Suggested questions, in order:

   1. What are we building, and why does it matter?
   2. Who will use it? (1–3 user archetypes.)
   3. What does "done enough to ship v1" look like? (3–5
      concrete deliverables.)
   4. What constraints are we not free to violate? (Tech,
      compliance, deadlines.)
   5. What is explicitly **not** in scope for v1? (Non-goals.)
   6. What does "good enough to ship" look like? (Quality bar.)
   7. What do we expect to learn during construction? (Known
      unknowns.)

   Compose the answers into the `## Product north star` section
   using these subheadings: `### Users`, `### V1.0 outcome`,
   `### Quality bar`, `### Known unknowns`. Non-goals belong as
   prose at the section root or under a `### Non-goals`
   subsection. Constraints should reference the project's
   existing `## Core principles` / `## Standard hygiene` if
   present.

5. **Report.** Tell the user what was scaffolded, what was added
   to `AGENTS.md` (if anything), and the final counts (`N
   created, N overwritten`).

6. **Suggest the next step.** `{{ cmd_prefix }}speccy-plan` to draft the first
   SPEC slice from the now-populated north star.

This recipe does not loop. The greenfield bootstrap runs once;
subsequent re-runs of `speccy init --force` only refresh the
shipped skill files and do not touch `AGENTS.md`.
