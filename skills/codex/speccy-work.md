---
name: speccy-work
description: Phase 3. Implementation loop. Spawn one implementer sub-agent per open task until none remain.
---

# speccy-work

Drives the Phase 3 implementation loop. The main agent repeatedly asks
the CLI for the next `[ ]` task and spawns an implementer sub-agent
with the rendered prompt until no open tasks remain.

## When to use

After `speccy-tasks` has written `TASKS.md` and the spec hash has been
committed. Can be rerun after `speccy-review` flips tasks back to
`[ ]` (retry path).

## Steps

1. Query the CLI for the next implementable task:

   ```bash
   speccy next --kind implement --json
   ```

2. If the result is `kind: blocked` or empty, exit the loop and tell
   the user no implementable tasks remain.
3. If the result is a task, render the implementer prompt:

   ```bash
   speccy implement T-003
   ```

4. Spawn an implementer sub-agent with that prompt. The sub-agent
   flips the task `[ ]` -> `[~]` on start, writes tests + code, runs
   `speccy check` locally, and flips `[~]` -> `[?]` on finish.
5. After the sub-agent returns, go back to step 1.

### Loop exit criteria

- `speccy next --kind implement --json` returns empty or `blocked`.
- The user interrupts the loop.

After exit, suggest `speccy-review SPEC-NNNN` to start Phase 4.
