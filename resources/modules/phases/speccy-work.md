
# {{ cmd_prefix }}speccy-work

Implements one task per invocation and exits. With an optional
`[SPEC-NNNN/T-NNN]` selector argument, the session implements that
specific task. Without an argument, the session resolves the next
implementable task via `speccy next --json` (workspace form — used here
because no SPEC is known on the no-selector path) and implements that one.
Task state lives in the `state` attribute on each `<task>` XML element
in TASKS.md.

This is a single-task primitive. It does not iterate over the task
list; composition across tasks belongs to a caller (a human at the
terminal, the `/loop` skill, or a future orchestrator).

## When to use

- With a selector (`{{ cmd_prefix }}speccy-work SPEC-NNNN/T-003`):
  when the next task to implement is already known — e.g., a retry
  after `{{ cmd_prefix }}speccy-review` flipped a task back to
  `state="pending"`.
- Without an argument: when picking up wherever `TASKS.md` left
  off. The session implements one task and exits.

`{{ cmd_prefix }}speccy-decompose` must have written `TASKS.md` and the
spec hash must have been committed before this skill runs.

The agent automatically detects retry shape from the per-task
journal carried in the `speccy context` bundle's journal section and
switches modes accordingly — the caller does not pass a flag. A first-attempt
task (no journal, or no `<blockers>` matching the highest
`<implementer>` round) runs today's recipe unchanged. A retry-shape
task (the latest `<blockers>` round equals the highest
`<implementer>` round) amends the existing WIP in the working tree
in place and appends a new `<implementer>` block (the CLI stamps the
incremented round).

## What to consider

- Re-read the task's `covers="REQ-NNN"` REQ's `<done-when>`
  (acceptance criteria) and `<behavior>` (test selection) elements
  before writing tests — not the surrounding prose alone.
- The suggested-files hint in the task body is advisory and may be
  stale; verify each path before editing.
- Are you about to add a feature flag, abstraction layer, or
  configurability the SPEC did not ask for? Stop and reconsider —
  scope creep is a blocker for the reviewer round.

## Steps

**Entry precondition (SPEC-0045 REQ-002, extended by SPEC-0047 REQ-002 / REQ-003):** before any Task dispatch, (i) resolve the target task per step 1, then open the per-task context read with a single `speccy context SPEC-NNNN/T-NNN --json` call (the bundle carries the task entry, its covering requirements and scenarios, the latest-round journal blocks inline with prior rounds as an attributes-only index, the sibling index, the file paths, and a suggested merge-base diff command); (ii) apply the retry-shape rule summarized at step 2 (canonical statement at `{{ speccy_references_path }}/retry-shape.md`) against the bundle's journal section rather than a separate file read, (iii) run `git status --porcelain`. **First-attempt shape** with non-empty stdout exits the skill (surface dirty paths on stderr); empty stdout proceeds with the first-attempt branch (today's SPEC-0045/REQ-002 behaviour). **Retry shape** proceeds with the retry branch regardless of stdout — the dirty paths are the prior pass's WIP that the retry implementer amends in place; no dirty-paths surface is written. If `speccy next --json` then returns `next_action.kind == "reconcile"`, dispatch the reconcile pass per the **Reconcile policy** below rather than the normal implementer flow.

{% include "modules/references/reconcile-summary.md" %}

1. Resolve the target task.

{% set task_kind = "work" %}{% set task_adjective = "implementable" -%}
{% include "modules/skills/partials/resolve-target-task.md" %}

2. Apply the REQ-001 retry-shape rule summarized immediately below
   against the journal section of the bundle returned by the
   `speccy context` call in step 1 (no separate journal-file read).
   The rule inspects only that journal content — it makes no further
   git, `speccy next`, or other CLI call. Compute the result
   (first-attempt shape or retry shape); the rest of the recipe
   branches on this result.

   **Retry shape.** A task is in retry shape iff its journal
   contains both an `<implementer>` element and a `<blockers>`
   element whose `round` attribute matches the highest implementer
   round. Otherwise it's first-attempt shape — the strict
   clean-tree gate applies. See
   `{{ speccy_references_path }}/retry-shape.md` for the full rule
   statement, read-only scope, worked examples, and the
   "implementer awaiting review" edge case.

3. Branch on the rule result.

   **First-attempt branch.** Proceed with the recipe below
   (steps 4–10) unchanged: flip state to `in-progress`, read
   scenarios, run the bounded reuse survey, implement from scratch,
   self-review, run the hygiene gate, flip to `in-review`, append the
   round-1 `<implementer>` block via `speccy journal append`.

   **Retry branch.** Enter retry mode:

   - Read the latest-round `<implementer>` block inlined in the
     bundle's journal section to understand the prior pass's stated
     `Completed` work.
   - Read the latest-round `<blockers>` block (the one whose `round`
     matches the highest `<implementer>` round, also inlined) from
     that same journal section for the specific feedback to address.
   - Prior rounds are not inlined: the bundle lists them as an
     attributes-only index (`round`, `block`, `persona`, `verdict`).
     If a prior round's prose is needed — e.g. a persona blocking
     across rounds — drill in explicitly with `speccy journal show
     SPEC-NNNN/T-NNN --round N [--block <type>]`.
   - Amend the existing WIP in the working tree to address the
     blockers. Do not run `git restore`, `git clean`, or
     `git checkout` against the dirty paths; do not rewrite the
     touched files from scratch; do not reset state. The dirty
     tree is the prior pass's WIP — iterate on it in place.
   - Flip state to `in-progress` via `speccy task transition
     SPEC-NNNN/T-NNN --to in-progress` and continue through the same
     hygiene gate and `speccy task transition … --to in-review` flip
     the first-attempt branch uses (the SPEC-0045/REQ-001 hygiene gate
     runs unchanged). Never edit the `state` attribute in TASKS.md
     directly.
   - Append the next `<implementer>` block via `speccy journal
     append` (step 9); the CLI derives and stamps the incremented
     round. The retry-mode `Completed` field describes the amend
     (what changed this round in response to the blockers), not a
     restatement of the cumulative task work.

4. Flip the target task's `state` from `pending` to `in-progress`
   through the transition command — never by editing the `state`
   attribute in TASKS.md directly:

   ```bash
   speccy task transition SPEC-NNNN/T-NNN --to in-progress
   ```

   The CLI enforces the legal state graph and rewrites the `state`
   attribute byte-surgically; an illegal edge or unresolved selector
   exits non-zero with the file untouched.

5. Read the task scenarios to understand what must be implemented
   from the bundle's covering-requirements section returned by the
   `speccy context` call in step 1 — its `requirements` carry each
   covered REQ's `<done-when>`, `<behavior>`, and `<scenario>`
   blocks. No separate entry read of SPEC.md, TASKS.md, or `speccy
   check` is needed here.

6. Bounded reuse survey. Before writing any code, survey the
   task-relevant area and classify the code you are about to add into
   reuse-as-is / extend / write-fresh, so reuse is a design input
   rather than a post-hoc cleanup. Scope the survey to the task's
   area — its covered REQs, the suggested-files hint, and the
   immediate module / neighbouring files — and **not** the whole repo.
   Let the survey inform what you write in the next step.

   {% include "modules/references/reuse-survey-implementer.md" %}

7. Implement the task. Write tests first, then code. Run the
   project's own test command (`cargo test`, `pnpm test`, etc.)
   locally. Use `speccy check SPEC-NNNN/T-NNN` to re-read the
   scenarios being satisfied (it renders them, it does not run
   them).

8. Self-review before handoff. Immediately after implementation and
   **before** the exit transition's `in-review` flip, re-read your
   own diff through the reviewers' lens and fix what you find in
   place. This is the cheap place to catch drift: a fix here is a
   few edits in a diff you already have open, whereas the same drift
   caught at review is a full bounce-and-respawn round. Address the
   findings now; do not defer them to the reviewers.

   **Reviewer north-star map.** Hold your diff to all four review
   outcomes:

   - **Business.** Every changed line traces to a covered REQ — no
     more, no less.
   - **Tests.** Tests drive real behaviour, each covered CHK is
     accounted for, and the evidence is honest.
   - **Security.** Inputs validated, errors handled not swallowed, no
     unsafe shortcut or leaked secret.
   - **Style.** Reads as the surrounding author wrote it; see the
     convention-drift checklist below.

   {% include "modules/references/convention-checklist.md" %}

9. Exit transition. **Hygiene gate (REQ-001):** before flipping `state` from `in-progress` to `in-review`, run the four standard hygiene gates in sequence — `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny check`. Any non-zero exit refuses the flip and keeps the task at `in-progress`; on all zeros, proceed with the flip and record one line per gate naming its exit code in the appended `<implementer>` block's `Hygiene checks` field. When the implementation is done, flip the task's
   `state` from `in-progress` to `in-review` through the transition
   command — never by editing the `state` attribute in TASKS.md
   directly:

   ```bash
   speccy task transition SPEC-NNNN/T-NNN --to in-review
   ```

   Then append one `<implementer>` block to the per-task
   journal via `speccy journal append`. Do NOT inline an
   `<implementer-note>` inside the `<task>` body in TASKS.md — the
   parser rejects that element. The journal file is the canonical
   home for implementer handoff prose.

   Append via the CLI. Pipe the seven-field body on stdin:

   ```bash
   speccy journal append SPEC-NNNN/T-NNN --block implementer \
     --model <your-model> <<'EOF'
   - Reuse survey: ...
   - Completed: ...
   - Undone: ...
   - Hygiene checks: ...
   - Evidence: ...
   - Discovered issues: ...
   - Procedural compliance: ...
   EOF
   ```

   {% include "modules/references/cli-stamps.md" %}

   `--model` is required and validated non-empty.

   {% include "modules/references/identity-sourcing.md" %}

   Canonical journal `<implementer>` shape: `{{ speccy_references_path }}/journal-implementer.md`.

   Canonical evidence file shape: `{{ speccy_references_path }}/evidence.md`.

   Body content. Use the seven-field handoff template the implementer
   prompt supplies (`Reuse survey`, `Completed`, `Undone`,
   `Hygiene checks`, `Evidence`, `Discovered issues`,
   `Procedural compliance`). The
   `Evidence` field must include a CHK-by-CHK roll call labelling
   each CHK under the task's covered REQs as `demonstrated`,
   `hygiene`, or `judgment-only` -- see the canonical reference for
   the format and what each label means (the canonical reference
   carries a full worked roll-call example).

   The CLI validates the block before any write, so a malformed body
   never lands and no re-read is needed; confirm `speccy next --json`
   reports no consistency drift.

10. Exit. Do not continue to the next task. If the caller wants
   another task, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="in-review"`, suggest
`{{ cmd_prefix }}speccy-review SPEC-NNNN`. If all tasks are
`state="completed"`, suggest `{{ cmd_prefix }}speccy-vet SPEC-NNNN`.
