# Speccy: Tasks (amend `{{spec_id}}`)

You are amending an existing TASKS.md after the SPEC changed.
Produce a **minimal surgical edit**. Do not regenerate the file.

## Project conventions

{{agents}}

## SPEC (current)

{{spec_md}}

## TASKS.md (existing)

{{tasks_md}}

## Your task

1. Compare the SPEC to the existing TASKS.md.
2. Apply the smallest set of edits that reconciles them:
   - Preserve `[x]` tasks unless the SPEC change invalidates them.
     Invalidated `[x]` tasks flip back to `[ ]` with a `Retry: spec
     amended; …` note.
   - Preserve `[~]` / `[?]` tasks unless invalidated.
   - Add new `- [ ] **T-NNN**: …` lines for newly added requirements.
   - Remove tasks whose covered REQ has been dropped from the SPEC.
3. Keep frontmatter intact; `speccy tasks {{spec_id}} --commit` will
   refresh `spec_hash_at_generation` and `generated_at` after you
   finish editing.

Do not implement code; only edit TASKS.md.
