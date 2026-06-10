
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

**Entry precondition (REQ-007, REQ-008):** before resolving the target task, query `speccy next --json` (per-spec form when a selector was passed, workspace form otherwise). If the returned envelope's `next_action.kind == "reconcile"`, dispatch the reconcile pass per the **Reconcile policy** summary below (canonical policy at `{{ speccy_references_path }}/reconcile-policy.md`) instead of running the normal review flow. Re-query after the pass; resume normal dispatch only when `consistency.status == "ok"`.

**Reconcile policy.** When `speccy next --json` returns `next_action.kind == "reconcile"`, iterate `consistency.drifts[]` and apply the table action per entry, then re-query before proceeding. See `{{ speccy_references_path }}/reconcile-policy.md` for the full policy table, the three properties the dispatch holds by construction (autonomous / rollback-biased / idempotent), and the extension protocol for adding new drift kinds.

### Resolve the target task

- If a `SPEC-NNNN/T-NNN` selector was passed, that is the target.
- Otherwise, query the CLI in workspace form (no SPEC selector
  is known on this no-selector invocation path — we must walk
  the active tree to find a reviewable task):

  ```bash
  # workspace form: no SPEC-NNNN known yet; scan the active tree.
  speccy next --json
  ```

  Workspace-form exit-code-stop contract: exit code 2 with a
  top-level `reason="no_active_specs"` field in the JSON envelope
  means the workspace has no active specs at all. Exit gracefully
  and surface the reason; do not treat the non-zero exit as a CLI
  error.

  On exit code 0, if the resulting `specs` array has no entry with
  `next_action.kind == "review"`, exit and report that no
  reviewable tasks remain. Otherwise, construct the disambiguated
  `<spec>/<task>` form from the JSON's `spec_id` and
  `next_action.task_id` fields (the bare task ID is ambiguous
  across specs — every spec has its own `T-001`).

  Exit-code-stop contract: once SPEC-NNNN is resolved, any
  subsequent per-spec query (`speccy next SPEC-NNNN --json`) that
  exits non-zero means the SPEC has reached a terminal state —
  halt and surface the stderr line. Only parse JSON when exit
  code is 0.

### Run the persona fan-out and consolidation

Shared with the `{{ cmd_prefix }}speccy-orchestrate` review
dispatch — both this skill body and that dispatch step include the
same partial below so the fan-out contract has a single source of
truth.

{% include "modules/skills/partials/review-fanout.md" %}

Reviewers append their own `<review>` block via `speccy journal
append` and return a thin verdict; they never edit TASKS.md or the
journal file with file-editing tools. This running session does not
transcribe `<review>` blocks. It drives the review-induced writes
exclusively through the CLI verbs the partial above details:
`speccy journal show` to verify the round's reviews are complete and
to read back blockers, `speccy task transition` for the `in-review`
→ `completed` / `pending` state flip, and `speccy journal append
--block blockers` for the consolidated orchestrator-authored
`<blockers>` block. No `<review>` or `<blockers>` block is ever
appended to the `<task>` body in TASKS.md — TSK-006 rejects journal
elements there.

### Exit

Do not pick up another `in-review` task. If the caller wants
another task reviewed, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state:
if any task is `state="pending"` (a retry), suggest
`{{ cmd_prefix }}speccy-work SPEC-NNNN`. If any remain
`state="in-review"`, suggest
`{{ cmd_prefix }}speccy-review SPEC-NNNN` again. If all tasks are
`state="completed"`, suggest `{{ cmd_prefix }}speccy-vet SPEC-NNNN`.
