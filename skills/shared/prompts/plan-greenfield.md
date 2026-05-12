# Speccy: Plan (greenfield)

You are drafting a new SPEC for an existing speccy workspace.

## Project conventions

{{agents}}

## Vision

{{vision}}

## Your task

Author the next slice as `SPEC-{{next_spec_id}}`.

1. Propose a slug (lowercase kebab-case).
2. Create `.speccy/specs/{{next_spec_id}}-<slug>/SPEC.md` using the
   PRD-shaped template in `.speccy/DESIGN.md`.
3. Create `.speccy/specs/{{next_spec_id}}-<slug>/spec.toml` mapping
   each requirement to at least one check.
4. Surface any material questions inline in `## Open questions`.

Do not write TASKS.md; the next phase will decompose it.
