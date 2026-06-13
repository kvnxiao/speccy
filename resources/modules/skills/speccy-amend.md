
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
   speccy status SPEC-NNNN --json
   ```

   The JSON's `spec_md_path` field names the SPEC.md file to edit.
2. Edit SPEC.md surgically (including its `<requirement>` /
   `<scenario>` element blocks if requirements changed); append a
   `## Changelog` row explaining *why* the amendment was needed.
   If editing `## Open Questions`, use the alpha-prefix format:
   `- [ ] a.`, `- [ ] b.`, ..., `- [ ] z.`. Preserve existing ordinals
   (do not renumber on amend); allocate the next free letter when
   appending a new question. Reaching `z.` signals an over-scoped
   session — 26 open questions is a scope smell, not a format
   limitation.
3. **Self-review pass.** Run this pass exactly once after writing the
   SPEC.md diff and appending the Changelog row. Do not re-check after
   applying fixes.

   {% include "modules/references/spec-self-review-core.md" %}

   Amend adds two deltas beyond the shared core:

   - **Changelog row presence.** The `## Changelog` section contains
     a new row explaining *why* this amendment was needed. A missing
     or empty Changelog entry is a mechanical issue; fix it inline
     before handing off to TASKS.md reconciliation.

   - **Surgical-diff shape.** Only the requirements and sections
     directly affected by the triggering intent shift were edited.
     A diff that rewrites unrelated requirements, re-words stable
     prose, or restructures sections that the amendment did not
     touch is out-of-scope and should be reverted inline.

4. Reconcile TASKS.md. Three kinds of reconciliation:

   - **Structural edits** — add new `<task>` elements for newly
     added requirements and remove `<task>` elements for dropped
     requirements. These are structural TASKS.md edits, not `state`
     mutations, so edit TASKS.md directly.
   - **State invalidation** — preserve `state="completed"` tasks
     unless the SPEC change invalidated them. For each invalidated
     task, flip `completed` → `pending` through the transition
     command, never by editing the `state` attribute directly:

     ```bash
     speccy task transition SPEC-NNNN/T-NNN --to pending
     ```

   - **Blocker directive** — for each invalidated task, append an
     amendment-driven `<blockers>` block to the per-task journal at
     `.speccy/specs/NNNN-slug/journal/T-NNN.md` via `speccy journal
     append`, never by editing the journal file directly and never
     into the `<task>` body in TASKS.md (the parser rejects journal
     elements there):

     ```bash
     speccy journal append SPEC-NNNN/T-NNN --block blockers <<'EOF'
     spec amended; <what changed in SPEC and what the next
     implementer attempt must address>.
     EOF
     ```

   The `<blockers>` body stays amendment-authored semantic judgment:
   name what changed in SPEC and what the next implementer attempt
   must address.

   {% include "modules/references/cli-stamps.md" %}

   Here `round` matches the current implementer round, so the next
   attempt continues at `N+1`.

   Canonical journal `<blockers>` shape: `{{ speccy_references_path }}/journal-blockers.md`.

   The `completed` → `pending` transition and the `<blockers>` append
   on the affected task are part of the same amendment turn.
5. Record the new spec hash:

   ```bash
   speccy lock SPEC-NNNN
   ```

6. Re-run `speccy status` to confirm `TSK-003` cleared.

7. Branch-guard, then commit the amend's reconcile delta. After the
   `TSK-003`-clear check in step 6 confirms the SPEC and tasks are back
   in sync, commit this amend's delta so the reconciled artifacts are
   recorded together. The commit covers the spec's `SPEC.md`, the
   reconciled `TASKS.md` **when one exists**, and any per-task journal
   blocker files this amend appended this run
   (`<spec-dir>/journal/T-NNN.md`). When the spec has no `TASKS.md` yet,
   the commit contains `SPEC.md` (plus any journal files) without failing
   on the absent tasks file — drop the missing `TASKS.md` from the
   staging list rather than requiring it to exist.

   First run the branch-guard prelude so the commit lands on a feature
   branch rather than the repository's default branch. Supply the
   prelude's one parameter — the **spec directory** (`<spec-dir>/`, i.e.
   the path that holds `SPEC.md`) — and run it:

{% include "modules/references/branch-guard.md" %}

   Then run the shared commit recipe, supplying its two
   behaviour-varying parameters as follows:

   - **Staging breadth: narrow `git add <paths>`.** Stage exactly the
     amend delta and nothing else: `<spec-dir>/SPEC.md`, the reconciled
     `<spec-dir>/TASKS.md` **only when it exists** (omit it from the list
     when the spec has no tasks file yet — do not let a missing path
     fail the stage), and each `<spec-dir>/journal/T-NNN.md` blocker file
     appended this run. Do not use `git add -A` or `git add .`.
   - **Title and body.**
     - **Title:** `[SPEC-NNNN]: amend — <why>` with `SPEC-NNNN`
       substituted for the resolved spec id, and `<why>` a title-length
       phrase derived from the **newest `## Changelog` row** added during
       this amend (step 2). Do not separately prompt for `<why>`; read it
       off the row you just wrote.
     - **Body:** the full text of that newest `## Changelog` row,
       explaining why the amendment was needed.

   With those two parameters fixed, run the shared recipe — it defines
   the no-git short-circuit, the unified stage-then-`git diff --cached
   --quiet` idempotency check (nothing new to record skips the commit
   silently), the `Co-Authored-By` trailer, and the HEREDOC commit
   mechanics:

{% include "modules/references/commit-recipe.md" %}

### Loop exit criteria

This recipe is a single pass, not a loop -- but step 6 is the gate. If
the lint still fires, repeat from step 1 (something was missed).

Suggest the next step: `{{ cmd_prefix }}speccy-orchestrate SPEC-NNNN` to
drive the loop over any tasks that flipped back to `state="pending"`, or
`{{ cmd_prefix }}speccy-work SPEC-NNNN` to pick them up one at a time.
