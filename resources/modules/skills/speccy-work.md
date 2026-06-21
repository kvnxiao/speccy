# {{ cmd_prefix }}speccy-work

{% if host == "claude-code" -%}
Read `.claude/agents/speccy-work.md` and follow it, or invoke
`/agent speccy-work` for the pinned execution path.
{%- else -%}
Read `.codex/agents/speccy-work.toml` and follow it, or invoke
`/agent speccy-work` for the pinned execution path.
{%- endif %}

Implements one task per invocation and exits. With an optional
`[SPEC-NNNN/T-NNN]` selector argument, the session implements that
specific task. Without an argument, the session resolves the next
implementable task via `speccy next --json` (workspace form) and
implements that one.

## When to use

- With a selector (`{{ cmd_prefix }}speccy-work SPEC-NNNN/T-NNN`):
  when the next task to implement is already known.
- Without an argument: when picking up wherever `TASKS.md` left
  off. The session implements one task and exits.

## Steps

{% include "modules/references/retry-shape-summary.md" %}

Resolve the target task — without a selector, query the CLI in workspace form
with `speccy next --json` — classify its shape from the journal via the rule
above, then apply the clean-tree gate:

{% include "modules/references/entry-precondition.md" %}

{% include "modules/references/reconcile-summary.md" %}

**Hygiene gate.** After the implementer turn, before flipping `state` from
`in-progress` to `in-review`, run the project's hygiene gates as defined in its
`AGENTS.md` (`## Standard hygiene` or the project-equivalent) — the gates and
their count are whatever that project declares; Speccy prescribes no fixed set.
Any non-zero exit refuses the flip and keeps the task at `in-progress`; on all
zeros, the appended `<implementer>` block's `Hygiene checks` field records one
line per gate with its exit code.

## Exit

One task moves to `state="in-review"` with a single `<implementer>` block
appended to its journal via `speccy journal append`. This is a single-task
primitive — it does not pick up the next task. Control returns to the caller;
the next reasonable step is `{{ cmd_prefix }}speccy-review SPEC-NNNN` while any
task is `in-review`, or `{{ cmd_prefix }}speccy-vet SPEC-NNNN` once all tasks
are `completed`.
