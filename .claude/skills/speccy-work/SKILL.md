---
name: speccy-work
description: Drive Speccy's implementation loop — work through each open task in TASKS.md one at a time, spawning an implementer sub-agent per task until none remain. Use when the user says "implement the tasks", "start working on SPEC-NNNN", "build out the spec", "run the work loop", or wants to resume coding against an existing task list.
---

# /speccy-work

Drives the implementation loop. The main agent repeatedly asks
the CLI for the next `[ ]` task and spawns an implementer sub-agent
with the rendered prompt until no open tasks remain.

## When to use

After `/speccy-tasks` has written `TASKS.md` and the spec hash has been
committed. Can be rerun after `/speccy-review` flips tasks back to
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

After exit, suggest `/speccy-review SPEC-NNNN` to start the review
phase.
