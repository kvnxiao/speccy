---
name: speccy-plan
description: Draft a new Speccy SPEC from the `AGENTS.md` product north star, or amend an existing one when intent shifts. Use when the user wants to "write a spec", "draft a SPEC", "spec out X", "plan a new feature with speccy", or asks to amend an existing spec by ID.
---

# speccy-plan

Renders the planning prompt: the host harness auto-loads `AGENTS.md`
(which carries the project-wide product north star), and the prompt
walks the agent through writing or amending `SPEC.md`. Top-level
intent surfaces (`<goals>`, `<non-goals>`, `<user-stories>`, optional
`<assumptions>`) and per-requirement sub-sections (`<done-when>`,
`<behavior>`, `<scenario>`) live as raw XML element blocks inside
SPEC.md itself. With a SPEC-ID argument, renders the amendment
prompt instead and names the nearest parent `MISSION.md` so
focus-area context can be loaded via the host's Read primitive.

## When to use

- New-spec form (no argument): when starting a new spec slice. If
  the ask is still fuzzy, run `speccy-brainstorm`
  first to atomize the intent — this skill writes SPEC.md in a
  single pass and assumes the framing is already agreed.
- Amendment form (`speccy-plan SPEC-NNNN`): when an
  existing spec needs surgical edits after intent shifted mid-loop.

## Steps

1. Identify whether the user wants a new spec or an amendment. If a
   SPEC-ID was passed, it is amendment; otherwise new-spec.
2. Render the prompt:

   ```bash
   speccy plan            # new spec
   speccy plan SPEC-0007  # amendment
   ```

3. Read the rendered prompt and follow it.

   - **New spec**: the prompt names an allocated `SPEC-NNNN` ID.
     Decide placement: flat (`.speccy/specs/NNNN-slug/`) or under an
     existing mission folder (`.speccy/specs/[focus]/NNNN-slug/`).
     Do not invent a new mission folder for a single spec.
   - **Amendment**: the prompt names the nearest parent `MISSION.md`
     path (when one exists), the existing SPEC.md path, and its
     recent changelog. Produce a minimal surgical diff and append a
     `## Changelog` row.

4. Surface any material questions inline in `## Open Questions`.
5. Suggest the next step: `speccy-tasks SPEC-NNNN` to
   decompose into `TASKS.md`.

This recipe does not loop.
