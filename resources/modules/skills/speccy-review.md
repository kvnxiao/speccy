
# {{ cmd_prefix }}speccy-review

Runs one round of adversarial review on one task per invocation and
exits. With an optional `[SPEC-NNNN/T-NNN]` selector argument, the
session reviews that specific task. Without an argument, the session
resolves the next reviewable task via `speccy next --json` (workspace
form — used here because no SPEC is known on the no-selector path) and reviews
that one. Task state lives in the `state` attribute on each `<task>`
XML element in TASKS.md; review activity prose lives in the sibling
`.speccy/specs/NNNN-slug/journal/T-NNN.md` file, never inside the
`<task>` body.

This is a single-task primitive. It does not iterate over the
remaining `in-review` tasks; composition across tasks belongs to a
caller (a human at the terminal, the `/loop` skill, or a future
orchestrator).

Within the one task under review, the skill fans out to five
parallel persona sub-agents (default fan-out: `business`, `tests`,
`security`, `style`, `correctness`). That fan-out is intrinsic to
the primitive — adversarial diversity comes from fresh contexts per
persona — and is bounded to one round of five sub-agents on one
task.

{% include "modules/skills/partials/inline-fanout-rationale.md" %}
This skill must run in the top-level session — either a human
invocation
(`{{ cmd_prefix }}speccy-review …`) or the
`{{ cmd_prefix }}speccy-orchestrate` outer loop inlining this body at
its `review` dispatch.

## When to use

- With a selector (`{{ cmd_prefix }}speccy-review SPEC-NNNN/T-003`):
  when the task to review is already known.
- Without an argument: when picking up wherever `TASKS.md` left off.
  The session reviews one task and exits.

The target task must already be in `state="in-review"` (typically
flipped there by `{{ cmd_prefix }}speccy-work`).

## Steps

**Entry precondition (REQ-007, REQ-008):** before resolving the target task, query `speccy next --json` (per-spec form when a selector was passed, workspace form otherwise). If the returned envelope's `next_action.kind == "reconcile"`, dispatch the reconcile pass per the **Reconcile policy** below instead of running the normal review flow. Re-query after the pass; resume normal dispatch only when `consistency.status == "ok"`.

{% include "modules/references/reconcile-summary.md" %}

### Resolve the target task

{% set task_kind = "review" %}{% set task_adjective = "reviewable" -%}
{% include "modules/skills/partials/resolve-target-task.md" %}

### Run the persona fan-out and consolidation

This section is the canonical fan-out grammar. The
`{{ cmd_prefix }}speccy-orchestrate` review dispatch runs the same
fan-out inline in its own session and points here rather than
duplicating it.

{% include "modules/skills/partials/review-fanout.md" %}

### Exit

Do not pick up another `in-review` task. If the caller wants
another task reviewed, the caller invokes this skill again.

After exit, the recommended path is
`{{ cmd_prefix }}speccy-orchestrate SPEC-NNNN`, which dispatches the
right next step automatically. To drive it by hand, the step depends on
TASKS.md state: `state="pending"` (a retry) →
`{{ cmd_prefix }}speccy-work SPEC-NNNN`; any still `state="in-review"` →
`{{ cmd_prefix }}speccy-review SPEC-NNNN` again; all `state="completed"`
→ `{{ cmd_prefix }}speccy-vet SPEC-NNNN`.
