# Speccy: Plan (amend `{{spec_id}}`)

You are amending an existing SPEC. Produce a **minimal surgical
diff** to the SPEC.md below. Do not rewrite sections that are still
correct.

## Project conventions

{{agents}}

## Existing SPEC

{{spec_md}}

## Recent changelog

{{changelog}}

## Your task

1. Identify the smallest change set that resolves the amendment
   need without invalidating completed tasks.
2. Edit `.speccy/specs/.../SPEC.md` in place.
3. Append a new row to the `## Changelog` table describing **why**
   the amendment was needed.
4. If the amendment invalidates the requirement-to-check mapping,
   update `spec.toml` accordingly.

Do not regenerate TASKS.md; the next phase will reconcile it.
