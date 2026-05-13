# Speccy: Implement `{{task_id}}` for `{{spec_id}}`

You are implementing one task in one spec. Work surgically; do not
touch unrelated files. When done, flip `{{task_id}}` from `[ ]` /
`[~]` to `[?]` in TASKS.md and append your implementer note inline.

## Project conventions

{{agents}}

## SPEC (full)

{{spec_md}}

## Task entry (verbatim from TASKS.md)

{{task_entry}}

## Suggested files

{{suggested_files}}

## Your task

1. Read the SPEC requirements the task covers (`Covers: REQ-NNN`).
2. Read every prior bullet under the task entry: implementer notes
   from past attempts, review feedback, retry annotations.
3. Translate each `Tests to write:` bullet into an executable test
   in the project's framework. **Write the test before the code it
   exercises.**
4. Implement the code path so the tests pass. Run `speccy check`
   locally and fail fast on red.
5. Append one implementer note to the task subtree summarizing what
   you did, including any out-of-scope edits made for the test to
   compile.
6. Flip the task checkbox to `[?]` to signal "awaiting review".

Do not modify SPEC.md or spec.toml. Surgical changes only.
