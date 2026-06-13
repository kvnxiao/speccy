
# {{ cmd_prefix }}speccy-vet

Final defense mechanism against SPEC drift at the pre-ship boundary.

The per-task implementer/reviewer cycle keeps each task honest
against its own scenarios. It does not catch drift that only appears
at the whole-SPEC level: requirements satisfied by no task, behavior
the diff introduces that the SPEC never authorized, or per-task
code that doesn't add up to the SPEC as a unit. This skill is the
final check before the PR opens.

This skill is **autonomous up to the ship point**. It fans out
review and implementer sub-agents, loops on drift, applies a
simplifier polish pass, and returns a single short verdict to its
caller. The caller (typically `{{ cmd_prefix }}speccy-orchestrate`
at the `ship` boundary, but a human can invoke this directly) is
the one that gates the actual PR opening.

## When to use

- Every task in `SPEC-NNNN` is at `state="completed"` and the next
  step is to confirm the diff matches the SPEC as a unit before
  invoking `{{ cmd_prefix }}speccy-ship`.
- A SPEC was amended mid-implementation and the human wants a final
  defense pass against drift before opening the PR.
- The `{{ cmd_prefix }}speccy-orchestrate` outer loop reaches the
  `ship` boundary and delegates here.

Do not invoke this skill for per-task review (use
`{{ cmd_prefix }}speccy-review`) or while tasks remain at
`pending` / `in-progress` / `in-review` (the skill returns `fail`
immediately if any task is non-completed — this is a pre-ship gate,
not a mid-loop check).

## Argument

```
{{ cmd_prefix }}speccy-vet SPEC-NNNN
```

The `SPEC-NNNN` argument is required. The SPEC's tasks must all be
at `state="completed"` — this is a pre-ship gate, not a mid-loop
check. If any task is not completed, return a `fail` verdict
immediately with that as the reason.

## Why this skill runs in a top-level session

{% include "modules/skills/partials/inline-fanout-rationale.md" %}
This skill's drift-fix loop fans out `vet-reviewer` /
`vet-implementer` / `vet-simplifier` sub-agents across multiple
rounds, so it must run in the top-level session — either a human
invocation
(`{{ cmd_prefix }}speccy-vet SPEC-NNNN`) or the
`{{ cmd_prefix }}speccy-orchestrate` outer loop inlining this body at
its `ship` dispatch. The leaf sub-agents each return one short verdict
block as their final message; only those flow back into the running
session.

## What this skill writes and commits

This skill writes to VET.md and lets implementer / simplifier
sub-agents modify the working tree. It **does not commit
anything**. Per Speccy's atomic-landing convention,
`{{ cmd_prefix }}speccy-ship` is the committer — it bundles all
uncommitted changes from the loop (per-task journal updates,
holistic changes, SPEC.md status flip, REPORT.md) into one commit
at PR time. VET.md, being under
`.speccy/specs/<spec-dir>/journal/`, ships alongside the per-task
journal files.

If a human invokes this skill directly (outside the orchestrator),
they will see uncommitted changes in `git status` after exit and
can decide whether to commit, revert, or continue editing.

## Holistic journal

This skill maintains a single per-SPEC journal file at
`<spec-dir>/journal/VET.md` (`<spec-dir>` is resolved in
Phase 0 below). The journal is the **persistent state** of the
holistic loop:

1. **Round-to-round communication** — round N's reviewer reads it
   to walk round N-1's findings and verify the implementer's claims
   against the current diff. Without it, every round re-derives
   from scratch and wastes the round budget.
2. **Audit trail** — after the skill exits, the human (or
   `{{ cmd_prefix }}speccy-ship`) can read it to see what the loop
   caught and how.

### Single-writer rule

All VET writes go through `speccy journal append`; never edit the
file by hand. The CLI's per-file append lock serializes the parallel
appends: the vet sub-agents append their own `<drift-review>` /
`<holistic-fix>` / `<simplifier-scan>` / `<simplifier-apply>` blocks,
and this skill's session appends the terminal `<gate>` block (it is
the sole author of `<gate>` and, under the orchestrator, of git
commits, but it does not transcribe sub-agent blocks).

### File format

The CLI creates and stamps all of VET.md — frontmatter (`spec`,
`generated_at`), each `## Invocation N — <date>` section, and every
block's `date`/`round` attributes; the skill never writes any of it
by hand. The `round` attribute is **per-invocation**; the CLI resets
it to 1 at the start of each invocation section. `generated_at` is
the file-creation timestamp and is never rewritten.

If a prior invocation crashed mid-loop, its section is left as-is —
the audit trail records what happened. The next append opens a fresh
section.

## Loop

{% include "modules/skills/partials/vet-phases.md" %}

## Return contract

Return exactly one block as the final message of this skill's
session:

```
<orchestrator-verdict verdict="pass|fail" invocation="N" rounds="R" simplifier="clean|applied|reverted|skipped">
<one-line summary of the holistic outcome>
[if fail: one-line suggested next step (amend SPEC vs new task vs manual fix)]
</orchestrator-verdict>
```

Field reference:

- `verdict` — `pass` if drift cleared within budget; `fail`
  otherwise.
- `invocation` — the invocation number for this run, matching the
  VET.md section header.
- `rounds` — how many drift-fix rounds were consumed (0 to 3).
- `simplifier` — `clean` if no candidates were found; `applied` if
  candidates applied + hygiene green; `reverted` if applied but
  rolled back due to hygiene failure; `skipped` if Phase 2 didn't
  run (drift never cleared).

The caller consumes only this final block; the inner blocks live
in VET.md and the sub-agent contexts that produced them.

## Stop conditions

- Drift round budget exhausted (3 rounds in this invocation)
  without a `pass` from the drift reviewer → return `fail`.
- Drift-fix implementer returns `verdict="stuck"` → return `fail`
  immediately.
- Any sub-agent errors or returns a malformed verdict → return
  `fail` with the error in the one-line summary.
- Phase 0 finds the spec is unknown or has incomplete tasks →
  return `fail` immediately.

## When to invoke directly

A human can run `{{ cmd_prefix }}speccy-vet SPEC-NNNN` by hand (each
direct invocation gets its own section in VET.md). The skill behaves
identically whether invoked by the orchestrator or by a human — only
the caller of the verdict differs.

## Next step after exit

When the gate returns `verdict="pass"` and REPORT.md is absent for
the SPEC, suggest `/speccy-ship SPEC-NNNN` (rendered as
`{{ cmd_prefix }}speccy-ship SPEC-NNNN` in the shared body) as the
next reasonable step — the SPEC is freshly vetted and ready to be
committed and PR'd. When the gate returns `verdict="fail"`, the
final block's one-line suggested next step takes precedence over
ship.
