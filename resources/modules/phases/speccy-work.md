
# {{ cmd_prefix }}speccy-work

Implements one task per invocation and exits. With an optional
`[SPEC-NNNN/T-NNN]` selector argument, the session implements that
specific task. Without an argument, the session resolves the next
implementable task via `speccy next --kind implement --json` and
implements that one. Task state lives in the `state` attribute on
each `<task>` XML element in TASKS.md.

This is a single-task primitive. It does not iterate over the task
list; composition across tasks belongs to a caller (a human at the
terminal, the `/loop` skill, or a future orchestrator).

## When to use

- With a selector (`{{ cmd_prefix }}speccy-work SPEC-0007/T-003`):
  when the next task to implement is already known — e.g., a retry
  after `{{ cmd_prefix }}speccy-review` flipped a task back to
  `state="pending"`.
- Without an argument: when picking up wherever `TASKS.md` left
  off. The session implements one task and exits.

`{{ cmd_prefix }}speccy-tasks` must have written `TASKS.md` and the
spec hash must have been committed before this skill runs.

## Steps

1. Resolve the target task.

   - If a `SPEC-NNNN/T-NNN` selector was passed, that is the target.
   - Otherwise, query the CLI:

     ```bash
     speccy next --kind implement --json
     ```

     If the result is `kind: blocked` or empty, exit and report that
     no implementable tasks remain. Otherwise, construct the
     disambiguated `<spec>/<task>` form from the JSON's `spec` and
     `task` fields (the bare `prompt_command` field is ambiguous
     across specs — every spec has its own `T-001`).

2. Flip the target task's `state` from `pending` to `in-progress`
   by editing TASKS.md.

3. Render the implementer prompt:

   ```bash
   speccy implement SPEC-0007/T-003
   ```

4. Follow the rendered prompt. Write tests first, then code. Run the
   project's own test command (`cargo test`, `pnpm test`, etc.)
   locally. Use `speccy check SPEC-NNNN/T-NNN` to re-read the
   scenarios being satisfied (it renders them, it does not run
   them).

5. Exit transition. When the implementation is done, flip the task's
   `state="..."` attribute from `in-progress` to `in-review` and
   append one implementer note using the six-field handoff template
   the implementer prompt supplies (`Completed`, `Undone`,
   `Commands run`, `Exit codes`, `Discovered issues`,
   `Procedural compliance`).

6. Exit. Do not continue to the next task. If the caller wants
   another task, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="in-review"`, suggest
`{{ cmd_prefix }}speccy-review SPEC-NNNN`. If all tasks are
`state="completed"`, suggest `{{ cmd_prefix }}speccy-ship SPEC-NNNN`.
