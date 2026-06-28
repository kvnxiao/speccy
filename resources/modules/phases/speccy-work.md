
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

- With a selector (`{{ cmd_prefix }}speccy-work SPEC-NNNN/T-NNN`):
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

**Entry precondition.** Before any Task dispatch: resolve the target task
(step 1), then open its context bundle with a single `speccy context
SPEC-NNNN/T-NNN --json` call. The bundle carries the task entry, its covering
requirements and scenarios, the latest-round journal blocks (prior rounds as an
attributes-only index), the sibling index, the file paths, and a suggested
merge-base diff command — so the retry-shape classification (step 2) reads the
bundle's journal section rather than a separate file read. Then apply the
clean-tree gate:

{% include "modules/references/entry-precondition.md" %}

If `speccy next --json` then reports `next_action.kind == "reconcile"`, dispatch
the reconcile pass (below) rather than the normal implementer flow.

{% include "modules/references/reconcile-summary.md" %}

1. Resolve the target task.

{% set task_kind = "work" %}{% set task_adjective = "implementable" -%}
{% include "modules/skills/partials/resolve-target-task.md" %}

2. Apply the retry-shape rule summarized immediately below
   against the journal section of the bundle returned by the
   `speccy context` call in step 1 (no separate journal-file read).
   The rule inspects only that journal content — it makes no further
   git, `speccy next`, or other CLI call. Compute the result
   (first-attempt shape or retry shape); the rest of the recipe
   branches on this result.

   {% include "modules/references/retry-shape-summary.md" %}

3. Branch on the rule result.

   **First-attempt branch.** Proceed with the recipe below
   (steps 4–12) unchanged: flip state to `in-progress`, read
   scenarios, load the memory ledger slice, run the bounded reuse
   survey, implement from scratch, write the evidence file, self-review,
   run the hygiene gate, flip to `in-review`, append the round-1
   `<implementer>` block via `speccy journal append`.

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
     the first-attempt branch uses (the hygiene gate runs unchanged).
     Never edit the `state` attribute in TASKS.md directly.
   - Append the next `<implementer>` block via `speccy journal
     append` (step 11); the CLI derives and stamps the incremented
     round. The retry-mode `Completed` field describes the amend
     (what changed this round in response to the blockers), not a
     restatement of the cumulative task work. If the retry adds or
     changes a `demonstrated` CHK, update the evidence file (step 9)
     first — the append refuses an unbacked `demonstrated` claim on
     every round, not just round 1.

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

6. Load the memory ledger slice.

   {% include "modules/references/memory-ledger-summary.md" %}

7. Bounded reuse survey. Before writing any code, survey the
   task-relevant area and classify the code you are about to add into
   reuse-as-is / extend / write-fresh, so reuse is a design input
   rather than a post-hoc cleanup. Scope the survey to the task's
   area — its covered REQs, the suggested-files hint, and the
   immediate module / neighbouring files — and **not** the whole repo.
   Let the survey inform what you write in the next step.

   {% include "modules/references/reuse-survey-implementer.md" %}

8. Implement the task. Write tests first, then code. Run the
   project's own test command (whatever its `AGENTS.md` defines)
   locally. Use `speccy check SPEC-NNNN/T-NNN` to re-read the
   scenarios being satisfied (it renders them, it does not run
   them).

9. Write the evidence file before appending. For every CHK you intend
   to label `demonstrated` in the step-11 roll call, write a
   red-then-green `### Scenario` into the canonical evidence file at
   `.speccy/specs/NNNN-slug/evidence/T-NNN.md` (sibling of `SPEC.md`
   and `TASKS.md`) — one scenario per `demonstrated` CHK, each showing
   the pre-edit red and post-edit green. This is a hard precondition of
   the append, not optional paperwork: `speccy journal append --block
   implementer` now refuses the block when its roll call labels a CHK
   `demonstrated` while that evidence file is absent or carries no
   `### Scenario` heading, naming the offending CHK id(s) and leaving
   the journal byte-identical. A CHK proved by a passing suite test is
   `hygiene` (cite the test), not `demonstrated`, and needs no
   scenario. Canonical evidence shape: `{{ speccy_references_path }}/evidence.md`.

10. Self-review before handoff. Immediately after implementation and
   **before** the exit transition's `in-review` flip, re-read your
   own diff through the reviewers' lens and fix what you find in
   place. This is the cheap place to catch drift: a fix here is a
   few edits in a diff you already have open, whereas the same drift
   caught at review is a full bounce-and-respawn round. Address the
   findings now; do not defer them to the reviewers.

   **Scan for leaked provenance first — the most common avoidable
   bounce.** Strip any planning-artifact citation used as the reason a
   line exists — a bare REQ/CHK/DEC/SPEC id, a doc path (`docs/…`), or a
   rule-file pointer in source, docstrings, or test comments — keeping the
   self-standing prose. The checklist below carries the full taxonomy and
   is what the style reviewer runs, so a miss here is a guaranteed bounce.

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

11. Exit transition. **Hygiene gate.** Before flipping `state` from `in-progress` to `in-review`, run the project's hygiene gates as defined in its `AGENTS.md` (`## Standard hygiene` or the project-equivalent — the gates and their count are whatever that project declares). Any non-zero exit refuses the flip and keeps the task at `in-progress`; on all zeros, proceed with the flip and record one line per gate naming its exit code in the appended `<implementer>` block's `Hygiene checks` field. When the implementation is done, flip the task's
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

12. Exit. Do not continue to the next task. If the caller wants
   another task, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="in-review"`, suggest
`{{ cmd_prefix }}speccy-review SPEC-NNNN`. If all tasks are
`state="completed"`, suggest `{{ cmd_prefix }}speccy-vet SPEC-NNNN`.
