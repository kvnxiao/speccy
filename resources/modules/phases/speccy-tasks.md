
# {{ cmd_prefix }}speccy-tasks

Decomposes the SPEC into an ordered, single-agent-sized task list in
`TASKS.md`. If `TASKS.md` already exists, amends it surgically instead
(preserving the state of any task already in flight).

## When to use

- Initial: after `{{ cmd_prefix }}speccy-plan` lands a fresh SPEC.
- Amendment: after `{{ cmd_prefix }}speccy-plan SPEC-NNNN` edited an existing SPEC and
  the tasks may now be stale (the CLI surfaces a `TSK-003` lint when
  it detects hash drift).

## Steps

1. Read the spec's current state to locate SPEC.md:

   ```bash
   speccy status SPEC-0007 --json
   ```

   The JSON's `spec_md_path` field names the SPEC.md to decompose.
   If `tasks_md_path` is non-null, an existing TASKS.md is present
   and this is an amendment run (edit surgically; preserve
   `state="completed"` tasks unless invalidated).
2. Write `TASKS.md` as Markdown with a single `<tasks spec="SPEC-NNNN">`
   root element wrapping one
   `<task id="T-NNN" state="pending" covers="REQ-NNN">...</task>`
   block per task. Each `<task>` body contains a `<task-scenarios>`
   element with slice-level Given/When/Then prose and an optional
   `Suggested files:` bullet.
3. After writing, record the current spec hash:

   ```bash
   speccy lock SPEC-0007
   ```

4. Suggest the next step: `{{ cmd_prefix }}speccy-work SPEC-0007` to start the
   implementation loop.

This recipe does not loop.
