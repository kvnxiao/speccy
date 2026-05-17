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
2. Author `.speccy/specs/.../TASKS.md` as Markdown carrying raw XML
   element tags for structure. Keep the frontmatter intact:
   `spec`, `spec_hash_at_generation: bootstrap-pending`, `generated_at`.
   `speccy tasks {{spec_id}} --commit` will record the real hash
   after you finish writing.
3. Wrap every task in the file inside a single
   `<tasks spec="{{spec_id}}">...</tasks>` root element. Phase
   headings (`## Phase 1: ...`) are decorative for human readers and
   sit between `<task>` elements, not inside them.
4. For each task, emit a `<task id="T-NNN" state="pending"
   covers="REQ-NNN[ REQ-NNN]*">...</task>` block. Inside the block:
   - A level-2 heading naming the task (decorative; the parser reads
     the id from the `id` attribute).
   - Exactly one `<task-scenarios>...</task-scenarios>` element
     containing the slice-level Given/When/Then validation prose for
     this task. Each scenario is one paragraph of Given/When/Then
     prose; the implementer translates each into an executable test
     in the project's framework **before** writing the code path,
     and the reviewer-tests persona checks the listed scenarios
     exist as tests and meaningfully exercise the claimed behavior.
   - An optional `Suggested files:` bullet listing backticked file
     paths that hint where work happens. Advisory only; not enforced.
5. The four valid `state` attribute values are `pending`,
   `in-progress`, `in-review`, `completed`. Initial decomposition
   emits every task as `state="pending"`. **Do not** use the old
   checkbox glyphs (`[ ]` / `[~]` / `[?]` / `[x]`); they are no
   longer the machine contract.
6. The `covers` attribute lists one or more `REQ-NNN` ids separated
   by single ASCII spaces. Every covered requirement must exist in
   the SPEC above.

Do not implement code; only write TASKS.md.
