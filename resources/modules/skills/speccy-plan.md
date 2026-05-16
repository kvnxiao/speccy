
# {{ cmd_prefix }}speccy-plan

Renders the planning prompt: read `AGENTS.md` (which carries the
project-wide product north star), propose the next SPEC slice, and
write `SPEC.md` (requirements and validation scenarios live as
raw XML element blocks — `<requirement>` / `<scenario>` — inside
SPEC.md itself). With a SPEC-ID argument, renders the amendment
prompt instead and inlines the nearest parent `MISSION.md` so
focus-area context is in scope.

## When to use

- Greenfield form (no argument): when starting a new spec slice.
- Amendment form (`{{ cmd_prefix }}speccy-plan SPEC-NNNN`): when an existing spec
  needs surgical edits after intent shifted mid-loop.

## Steps

1. Identify whether the user wants greenfield or amendment. If a
   SPEC-ID was passed, it is amendment; otherwise greenfield.
2. Render the prompt:

   ```bash
   speccy plan          # greenfield
   speccy plan SPEC-0007  # amendment
   ```

3. Read the rendered prompt and follow it.

   - **Greenfield**: the prompt inlines `AGENTS.md` (product north
     star plus conventions) and an allocated `SPEC-NNNN` ID. Decide
     placement: flat (`.speccy/specs/NNNN-slug/`) or under an
     existing mission folder (`.speccy/specs/[focus]/NNNN-slug/`).
     Do not invent a new mission folder for a single spec.
   - **Amendment**: the prompt inlines `AGENTS.md`, the nearest
     parent `MISSION.md` (or a marker saying the spec is
     ungrouped), the existing SPEC.md, and its recent changelog.
     Produce a minimal diff and append a `## Changelog` row.

4. Surface any material questions inline in `## Open questions`.
5. Suggest the next step: `{{ cmd_prefix }}speccy-tasks SPEC-NNNN` to decompose
   into `TASKS.md`.

This recipe does not loop.
