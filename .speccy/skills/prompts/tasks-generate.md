# Speccy: Tasks (initial decomposition for `{{spec_id}}`)

You are decomposing a SPEC into the smallest sequence of
implementation tasks that an implementer sub-agent can pick up one
at a time.

## Project conventions

{{agents}}

## SPEC to decompose

{{spec_md}}

## Your task

1. Read the SPEC. Identify every REQ heading and the design notes
   that drive implementation order.
2. Author `.speccy/specs/.../TASKS.md` with phase headings (decorative)
   and one `- [ ] **T-NNN**: short title` line per task.
3. For each task, list:
   - `Covers: REQ-NNN[, REQ-NNN]` — every REQ ID this task touches.
   - `Tests to write:` — unit-level test obligations, one bullet each,
     in English. The implementer translates these into executable
     tests before writing code.
   - `Suggested files:` — backticked file paths that hint where work
     happens. Advisory only; not enforced.
4. Keep the frontmatter intact:
   `spec`, `spec_hash_at_generation: bootstrap-pending`, `generated_at`.
   `speccy tasks {{spec_id}} --commit` will record the real hash
   after you finish writing.

Do not implement code; only write TASKS.md.
