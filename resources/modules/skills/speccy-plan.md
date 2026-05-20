
# {{ cmd_prefix }}speccy-plan

Drafts a new `SPEC.md` from the `AGENTS.md` product north star, or
amends an existing one when intent shifts. The host harness auto-loads
`AGENTS.md` (which carries the project-wide product north star); this
recipe walks the agent through writing or amending SPEC.md. Top-level
intent surfaces (`<goals>`, `<non-goals>`, `<user-stories>`, optional
`<assumptions>`) and per-requirement sub-sections (`<done-when>`,
`<behavior>`, `<scenario>`) live as raw XML element blocks inside
SPEC.md itself. With a SPEC-ID argument, the recipe runs the amendment
flow instead and names the nearest parent `MISSION.md` so focus-area
context can be loaded via the host's Read primitive.

## When to use

- New-spec form (no argument): when starting a new spec slice. If
  the ask is still fuzzy, run `{{ cmd_prefix }}speccy-brainstorm`
  first to atomize the intent — this skill writes SPEC.md in a
  single pass and assumes the framing is already agreed.
- Amendment form (`{{ cmd_prefix }}speccy-plan SPEC-NNNN`): when an
  existing spec needs surgical edits after intent shifted mid-loop.

## Steps

1. Identify whether the user wants a new spec or an amendment. If a
   SPEC-ID was passed, it is amendment; otherwise new-spec.
2. Resolve context from the CLI:

   - **New spec**: query the next available ID:

     ```bash
     speccy vacancy --json
     ```

     The JSON's `next_spec_id` field is the allocated `SPEC-NNNN` ID.
     Decide placement: flat (`.speccy/specs/NNNN-slug/`) or under an
     existing mission folder (`.speccy/specs/[focus]/NNNN-slug/`).
     Do not invent a new mission folder for a single spec.

   - **Amendment**: read the existing spec's current state:

     ```bash
     speccy status SPEC-0007 --json
     ```

     The JSON's `spec_md_path` field names the SPEC.md location and
     `mission_md_path` names the nearest parent MISSION.md (when one
     exists). Produce a minimal surgical diff and append a
     `## Changelog` row.

3. Write or amend SPEC.md following the PRD template.

4. Surface any material questions inline in `## Open Questions`.
5. Suggest the next step: `{{ cmd_prefix }}speccy-tasks SPEC-NNNN` to
   decompose into `TASKS.md`.

This recipe does not loop.
