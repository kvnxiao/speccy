---
name: speccy-amend
description: Orchestrate a mid-loop SPEC change — surgically edit SPEC.md with a Changelog row, reconcile TASKS.md, and re-record the spec hash so the SPEC and task list are back in sync. Use when the user says "amend SPEC-NNNN", "the requirements shifted", or when speccy reports the SPEC and tasks are out of sync.
---

# speccy-amend

Orchestrates a mid-loop SPEC change. Edits SPEC.md surgically via the
plan-amend prompt, then reconciles TASKS.md via the tasks-amend prompt,
then re-records the spec hash.

## When to use

When `speccy status` reports `TSK-003: spec hash mismatch`, or when the
user signals that intent has shifted mid-loop. Use this rather than
manually editing SPEC.md so that the Changelog row and TASKS.md
reconciliation are not forgotten.

## Steps

1. Render the SPEC.md amendment prompt:

   ```bash
   speccy plan SPEC-0007
   ```

2. Follow the prompt: edit SPEC.md surgically; append a `## Changelog`
   row explaining *why* the amendment was needed; update `spec.toml`
   if requirements changed.
3. Render the TASKS.md amendment prompt:

   ```bash
   speccy tasks SPEC-0007
   ```

4. Follow the prompt: preserve `[x]` tasks unless the SPEC change
   invalidated them (those flip back to `[ ]` with a `Retry: spec
   amended; ...` note); add tasks for new requirements; remove tasks
   for dropped requirements.
5. Record the new spec hash:

   ```bash
   speccy tasks SPEC-0007 --commit
   ```

6. Re-run `speccy status` to confirm `TSK-003` cleared.

### Loop exit criteria

This recipe is a single pass, not a loop -- but step 6 is the gate. If
the lint still fires, repeat from step 1 (something was missed).

Suggest the next step: `speccy-work SPEC-0007` to pick up any tasks
that flipped back to `[ ]`.
