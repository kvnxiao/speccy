
# {{ cmd_prefix }}speccy-tasks

Renders the task-decomposition prompt: read the SPEC and decompose it
into an ordered, single-agent-sized task list. If `TASKS.md` already
exists, renders the amendment prompt instead.

## When to use

- Initial: after `{{ cmd_prefix }}speccy-plan` lands a fresh SPEC.
- Amendment: after `{{ cmd_prefix }}speccy-plan SPEC-NNNN` edited an existing SPEC and
  the tasks may now be stale (the CLI surfaces a `TSK-003` lint when
  it detects hash drift).

## Steps

1. Render the prompt:

   ```bash
   speccy tasks SPEC-0007
   ```

   The CLI picks the initial or amendment template based on whether
   `TASKS.md` already exists.
2. Follow the prompt: write `TASKS.md` with one `- [ ] **T-NNN**` line
   per task, plus `Covers:`, `Tests to write:`, and `Suggested files:`
   bullets. For amendment, edit surgically; preserve `[x]` tasks
   unless invalidated.
3. After writing, record the current spec hash:

   ```bash
   speccy tasks SPEC-0007 --commit
   ```

4. Suggest the next step: `{{ cmd_prefix }}speccy-work SPEC-0007` to start the
   implementation loop.

This recipe does not loop.
