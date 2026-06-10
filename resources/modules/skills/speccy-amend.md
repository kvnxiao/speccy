
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

   <!-- Note: the amend self-review below is an independent copy.
        The parallel copy for plan lives in speccy-plan.md (per DEC-001 /
        OQ-b: two independent copies, no shared partial). -->

   **Mechanical/semantic split.** Mechanical issues are
   string-matchable from the SPEC.md text: `TBD`/`TODO` strings,
   "and"/"also" inside `<requirement>` blocks, untouched `<...>`
   template placeholders, missing alpha-prefix ordinals in
   `## Open Questions`. Fix mechanical issues inline by editing
   SPEC.md — do not write anything to `## Open Questions` or to
   chat. If judging requires reading semantics, it is semantic.

   Semantic issues surface as a row appended to `## Open Questions`
   using this fixed template string verbatim:

   `- [ ] {ordinal}. **Self-review caught:** {issue}`

   where `{ordinal}` is the next free alpha-prefix letter continuing
   any existing sequence, and `{issue}` is a one-line description of
   the problem. Do not substitute freeform prose.

   **The eight check properties:**

   - **Routing fidelity.** Brainstorm artifacts landed in the
     correct SPEC.md sections: restated ask → Summary +
     Requirements; assumptions → `<assumptions>`; open questions →
     `## Open Questions`; rejected framings → `## Notes` or
     `<decision>` blocks. This check applies only when brainstorm
     ran for this SPEC. When brainstorm was skipped, scope-traces
     alone covers the equivalent verification against the user's
     stated ask.

   - **Atomization.** No `<requirement>` body contains "and"/"also"
     multi-outcome wording that implies two distinct verifiable
     outcomes in one requirement. A requirement that bundles two
     outcomes should be split.

   - **Scope-traces.** Every `<requirement>` traces to a brainstorm
     artifact or to the user's explicitly stated ask. Requirements
     that appeared without a visible source in the approved framing
     are scope creep.

   - **Internal consistency.** No contradictions exist across the
     goals, non-goals, requirements, and assumptions sections. A
     goal that a non-goal denies, or a requirement that violates an
     assumption, is an internal contradiction.

   - **Placeholder leakage.** No `TBD`, `TODO`, or untouched
     `<...>` template-placeholder strings remain in SPEC.md.
     These are mechanical and should be fixed inline, not surfaced.

   - **Ambiguity.** No `<requirement>` wording is interpretable in
     two materially different ways that would lead to different
     implementations. If the requirement is ambiguous, surface it
     as a semantic issue.

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
   must address. The CLI is the sole authority for the block's `date`
   and `round` — it stamps `date` (UTC now), derives `round` (matching
   the current implementer round so the next attempt continues at
   `N+1`), creates the journal with frontmatter if it is somehow
   absent, and emits the paired `<blockers>…</blockers>` element.
   **Do not compute, supply, or hand-author `date`, `round`, or the
   open/close tags**; the body you pipe is the inner text only.

   Canonical journal `<blockers>` shape: `{{ speccy_references_path }}/journal-blockers.md`.

   The `completed` → `pending` transition and the `<blockers>` append
   on the affected task are part of the same amendment turn.
5. Record the new spec hash:

   ```bash
   speccy lock SPEC-NNNN
   ```

6. Re-run `speccy status` to confirm `TSK-003` cleared.

### Loop exit criteria

This recipe is a single pass, not a loop -- but step 6 is the gate. If
the lint still fires, repeat from step 1 (something was missed).

Suggest the next step: `{{ cmd_prefix }}speccy-work SPEC-NNNN` to pick up any tasks
that flipped back to `state="pending"`.
