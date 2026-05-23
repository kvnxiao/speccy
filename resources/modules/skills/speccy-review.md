
# {{ cmd_prefix }}speccy-review

Runs one round of adversarial review on one task per invocation and
exits. With an optional `[SPEC-NNNN/T-NNN]` selector argument, the
session reviews that specific task. Without an argument, the session
resolves the next reviewable task via `speccy next --json` and reviews
that one. Task state lives in the `state` attribute on each `<task>`
XML element in TASKS.md; review activity prose lives in the sibling
`.speccy/specs/NNNN-slug/journal/T-NNN.md` file, never inside the
`<task>` body.

This is a single-task primitive. It does not iterate over the
remaining `in-review` tasks; composition across tasks belongs to a
caller (a human at the terminal, the `/loop` skill, or a future
orchestrator).

Within the one task under review, the skill fans out to four
parallel persona sub-agents (default fan-out: `business`, `tests`,
`security`, `style`). That fan-out is intrinsic to the primitive —
adversarial diversity comes from fresh contexts per persona — and is
bounded to one round of four sub-agents on one task.

Because sub-agents cannot spawn sub-agents, this skill must run in a
context that **is** the top-level session — either a human
invocation (`{{ cmd_prefix }}speccy-review …`) where the host CLI
itself runs the skill body, or the
`{{ cmd_prefix }}speccy-orchestrate` outer loop which inlines this
skill body into its own session at the `review` dispatch (it cannot
delegate to a wrapper sub-agent that would then try to spawn the
four persona leaves).

## When to use

- With a selector (`{{ cmd_prefix }}speccy-review SPEC-NNNN/T-003`):
  when the task to review is already known.
- Without an argument: when picking up wherever `TASKS.md` left off.
  The session reviews one task and exits.

The target task must already be in `state="in-review"` (typically
flipped there by `{{ cmd_prefix }}speccy-work`).

## Steps

### Resolve the target task

- If a `SPEC-NNNN/T-NNN` selector was passed, that is the target.
- Otherwise, query the CLI:

  ```bash
  speccy next --json
  ```

  If the result has no entry with `next_action.kind == "review"`,
  exit and report that no reviewable tasks remain. Otherwise,
  construct the disambiguated `<spec>/<task>` form from the
  JSON's `spec_id` and `next_action.task_id` fields (the bare
  task ID is ambiguous across specs — every spec has its own
  `T-001`).

### Run the persona fan-out and consolidation

Shared with the `{{ cmd_prefix }}speccy-orchestrate` review
dispatch — both this skill body and that dispatch step include the
same partial below so the fan-out contract has a single source of
truth.

{% include "modules/skills/partials/review-fanout.md" %}

Reviewers do not write to TASKS.md and do not write to
`journal/T-NNN.md` directly; they return their verdict to this
running session, which is the **sole writer to
`.speccy/specs/NNNN-slug/journal/T-NNN.md`** for the review-induced
journal appends and the **sole writer to TASKS.md** for the
review-induced `state` transition. No `<review>` block is ever
appended to the `<task>` body in TASKS.md — TSK-006 rejects
journal elements there.

### Exit

Do not pick up another `in-review` task. If the caller wants
another task reviewed, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state:
if any task is `state="pending"` (a retry), suggest
`{{ cmd_prefix }}speccy-work SPEC-NNNN`. If any remain
`state="in-review"`, suggest
`{{ cmd_prefix }}speccy-review SPEC-NNNN` again. If all tasks are
`state="completed"`, suggest `{{ cmd_prefix }}speccy-vet SPEC-NNNN`.
