# Speccy: Report `{{spec_id}}`

Every task in this spec is now `[x]`. Your job is to author
`REPORT.md` — the durable summary of what shipped.

## Project conventions

{{agents}}

## SPEC (full)

{{spec_md}}

## TASKS (full)

{{tasks_md}}

## Retry summary (derived from inline `Retry:` notes)

{{retry_summary}}

## Your task

1. Read SPEC.md, every task entry and its inline notes, and the retry
   summary above.
2. Write `.speccy/specs/.../REPORT.md` with frontmatter
   (`spec`, `outcome`, `generated_at`) and these sections, in this
   order, under the literal headings shown:
   - `## Outcome` — one of `delivered`, `partial`, or `abandoned`.
   - `## Requirements coverage` — every REQ-NNN with the check IDs
     covering it and a short note on which project test(s) satisfy
     each scenario. Speccy does not execute checks; do not write
     `PASS` / `FAIL` here.
   - `## Task summary` — total tasks, count retried, anything that
     triggered a SPEC amendment.
   - `## Out-of-scope items absorbed` — edits implementers made for
     the work to compile that were not part of the planned scope.
   - `## Skill updates` — any `skills/**` files implementers edited
     in-flight to fix friction (wrong command, missing environment
     variable, undocumented step). One bullet per file with a one-line
     summary of what changed and the task that surfaced the friction.
     Derive the file list from `Procedural compliance` lines in the
     inline implementer notes above plus
     `git diff --name-only -- skills/` if you have shell access. If no
     skill files were touched during the run, write `(none)` rather
     than omitting the section.
   - `## Deferred / known limitations` — anything caught by review
     that was intentionally deferred to a future spec.
3. Do NOT open the PR; the orchestrating skill will call `gh` after
   you finish writing REPORT.md.

Surgical only: do not edit SPEC.md or TASKS.md.
