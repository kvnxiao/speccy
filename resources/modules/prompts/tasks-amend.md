# Speccy: Tasks (amend `{{spec_id}}`)

You are amending an existing TASKS.md after the SPEC changed.
Produce a **minimal surgical edit**. Do not regenerate the file.

## SPEC (pointer)

Before reconciling, read SPEC.md at `{{spec_md_path}}`. The CLI no
longer inlines the SPEC body into this prompt; load it via your Read
primitive.

## TASKS.md (pointer)

Read the existing TASKS.md at `{{tasks_md_path}}`. The CLI no longer
inlines the TASKS body into this prompt; load it via your Read
primitive before producing the surgical diff.

## Your task

1. Compare the SPEC to the existing TASKS.md.
2. Apply the smallest set of edits that reconciles them. Task state
   lives in the `state` attribute on each `<task>` element. Valid
   values are exactly `pending`, `in-progress`, `in-review`,
   `completed`:
   - Preserve `state="completed"` tasks unless the SPEC change
     invalidates them. Invalidated `state="completed"` tasks flip
     back to `state="pending"` with a `Retry: spec amended; …`
     note appended inside the `<task>` body.
   - Preserve `state="in-progress"` and `state="in-review"` tasks
     unless invalidated.
   - For newly added requirements, append a new `<task
     id="T-NNN" state="pending" covers="REQ-NNN">...</task>`
     element. Each new task must include exactly one
     `<task-scenarios>` block with non-empty Given/When/Then prose
     for the slice-level validation contract.
   - Remove `<task>` elements whose covered REQ has been dropped
     from the SPEC.
3. Keep frontmatter intact; `speccy tasks {{spec_id}} --commit` will
   refresh `spec_hash_at_generation` and `generated_at` after you
   finish editing.

Do not implement code; only edit TASKS.md.
