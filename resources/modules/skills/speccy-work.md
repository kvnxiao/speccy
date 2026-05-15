
# {{ cmd_prefix }}speccy-work

Drives the implementation loop. The main agent repeatedly asks
the CLI for the next `[ ]` task and spawns an implementer sub-agent
with the rendered prompt until no open tasks remain.

## When to use

After `{{ cmd_prefix }}speccy-tasks` has written `TASKS.md` and the spec hash has been
committed. Can be rerun after `{{ cmd_prefix }}speccy-review` flips tasks back to
`[ ]` (retry path).

## Steps

1. Query the CLI for the next implementable task:

   ```bash
   speccy next --kind implement --json
   ```

2. If the result is `kind: blocked` or empty, exit the loop and tell
   the user no implementable tasks remain.
3. If the result is a task, render the implementer prompt using the
   disambiguated `<spec>/<task>` form constructed from the JSON's
   `spec` and `task` fields (the bare `prompt_command` field is
   ambiguous across specs — every spec has its own `T-001`):

   ```bash
   speccy implement SPEC-0007/T-003
   ```

4. Spawn an implementer sub-agent with that prompt. The sub-agent
   flips the task `[ ]` -> `[~]` on start, writes tests + code, runs
   the project's own test command (`cargo test`, `pnpm test`, etc.)
   locally (using `speccy check SPEC-NNNN/T-NNN` to re-read the
   scenarios it is satisfying), and flips `[~]` -> `[?]` on finish.
5. After the sub-agent returns, go back to step 1.

### Loop exit criteria

- `speccy next --kind implement --json` returns empty or `blocked`.
- The user interrupts the loop.

After exit, suggest `{{ cmd_prefix }}speccy-review SPEC-NNNN` to start the review
phase.
