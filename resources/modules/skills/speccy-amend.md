
# {{ cmd_prefix }}speccy-amend

Orchestrates a mid-loop SPEC change. Edits SPEC.md surgically via the
plan-amend prompt, then reconciles TASKS.md via the tasks-amend prompt,
then re-records the spec hash.

## When to use

When `speccy status` reports `TSK-003: spec hash mismatch`, or when the
user signals that intent has shifted mid-loop. Use this rather than
manually editing SPEC.md so that the Changelog row and TASKS.md
reconciliation are not forgotten.

## Steps

1. Read the existing SPEC's current location and state:

   ```bash
   speccy status SPEC-0007 --json
   ```

   The JSON's `spec_md_path` field names the SPEC.md file to edit.
2. Edit SPEC.md surgically (including its `<requirement>` /
   `<scenario>` element blocks if requirements changed); append a
   `## Changelog` row explaining *why* the amendment was needed.
3. Reconcile TASKS.md: preserve `state="completed"` tasks unless the
   SPEC change invalidated them (those flip their `state` back to
   `pending` with a `<retry>spec amended; ...</retry>` element
   appended inside the `<task>` body); add new `<task>` elements for
   newly added requirements; remove `<task>` elements for dropped
   requirements.
4. Record the new spec hash:

   ```bash
   speccy lock SPEC-0007
   ```

5. Re-run `speccy status` to confirm `TSK-003` cleared.

### Loop exit criteria

This recipe is a single pass, not a loop -- but step 6 is the gate. If
the lint still fires, repeat from step 1 (something was missed).

Suggest the next step: `{{ cmd_prefix }}speccy-work SPEC-0007` to pick up any tasks
that flipped back to `state="pending"`.
