---
name: speccy:plan
description: Phase 1. Draft a new SPEC from VISION.md or amend an existing one.
---

# speccy:plan

Renders the Phase 1 prompt: read `VISION.md`, propose the next SPEC
slice, and write `SPEC.md` + `spec.toml`. With a SPEC-ID argument,
renders the amendment prompt instead.

## When to use

- Greenfield form (no argument): when starting a new spec slice.
- Amendment form (`speccy:plan SPEC-NNNN`): when an existing spec
  needs surgical edits after intent shifted mid-loop.

## Steps

1. Identify whether the user wants greenfield or amendment. If a
   SPEC-ID was passed, it is amendment; otherwise greenfield.
2. Render the prompt:

   ```bash
   speccy plan          # greenfield
   speccy plan SPEC-0007  # amendment
   ```

3. Read the rendered prompt -- it inlines `VISION.md` (or the existing
   SPEC.md) plus `AGENTS.md` -- and follow it: propose a slug, write
   `.speccy/specs/NNNN-slug/SPEC.md`, write `spec.toml`. For amendment,
   produce a minimal diff plus a `## Changelog` row.
4. Surface any material questions inline in `## Open questions`.
5. Suggest the next step: `speccy:tasks SPEC-NNNN` to decompose into
   `TASKS.md`.

This recipe does not loop.
