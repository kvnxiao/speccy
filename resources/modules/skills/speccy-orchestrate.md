
# {{ cmd_prefix }}speccy-orchestrate

Thin composition layer over the Speccy single-task primitives.
Queries `speccy next SPEC-NNNN --json`, dispatches each step to one or more
sub-agents, and re-queries until the SPEC is ready to ship. This
skill itself holds the outer dispatch loop, the per-task retry
counter, and (for review and ship steps) the multi-persona /
multi-round fan-out — all leaf work happens in sub-agent contexts
that exit when done.

**Why fan-outs run inline in this skill's session.** Sub-agents
cannot spawn sub-agents. The persona fan-out in `speccy-review`
(four reviewer personas in parallel) and the drift-fix loop in
`speccy-vet` (reviewer + implementer + simplifier sub-agents across
up to three rounds) therefore cannot be delegated to a single
"wrapper" sub-agent that follows the skill body — the wrapper would
fail to spawn its leaf sub-agents. Instead, this orchestrator
follows the `speccy-review` and `speccy-vet` skill bodies **inline
in its own session** and spawns the leaf sub-agents directly. Only
`speccy-work` (which never fans out) is delegated to a single
sub-agent.

## When to use

- The user wants to drive `SPEC-NNNN` from its current state to
  ready-to-ship without chaining `{{ cmd_prefix }}speccy-work`,
  `{{ cmd_prefix }}speccy-review`, and
  `{{ cmd_prefix }}speccy-vet` by hand.
- The SPEC already has a `TASKS.md` — this orchestrator dispatches
  against existing tasks; it does not plan or decompose.

Stops one step before invoking `{{ cmd_prefix }}speccy-ship`
(ship opens a PR — irreversible — and is always confirmed by the
user). Do not invoke this skill for ad-hoc "implement one task" or
"review one task" asks; prefer the single-task primitives.

## Argument

```
{{ cmd_prefix }}speccy-orchestrate SPEC-NNNN
```

The `SPEC-NNNN` argument is required. If missing, ask the user
which spec to drive and exit without looping.

## Lifecycle (outer loop + inline fan-outs)

```
outer:    speccy-orchestrate dispatch loop  ← this skill's session
            └── on `next_action.kind`:
                  ├── work   → spawn ONE speccy-work sub-agent
                  ├── review → fan out 4 reviewer-* sub-agents
                  │             in parallel from this session
                  │             (follows the speccy-review skill body)
                  └── ship   → run the speccy-vet skill body inline
                                in this session, spawning
                                vet-reviewer / vet-implementer /
                                vet-simplifier leaf sub-agents directly
                                (drift-fix loop bounded to 3 rounds,
                                 then one simplifier polish pass)
inner-1:  per-task retry — same task_id flipping pending after review
            (bounded here in the orchestrator: 5 rounds, then stop)
inner-2:  holistic drift fix — described in speccy-vet, run inline
            here (bounded: 3 rounds, then fail)
inner-3:  simplifier polish — described in speccy-vet, run inline
            here (no loop: one scan + one apply with hygiene gate)
```

The orchestrator owns the outer loop, the per-task retry counter,
the review consolidation step (journal append + TASKS.md state
flip), and the vet round counter / snapshot management. Only the
leaf work (one implementer pass, one persona review, one drift
review, one drift fix, one simplifier scan / apply) lives in a
spawned sub-agent.

## Context discipline

`speccy-work` dispatches to **one** sub-agent because the
implementer pass is single-shot and does not need to spawn its own
sub-agents. Its final message comes back as a short status hint.

`speccy-review` and `speccy-vet` cannot be delegated to a single
wrapper sub-agent because they themselves fan out — and sub-agents
cannot spawn sub-agents. This orchestrator follows their skill
bodies inline in its own session and spawns the leaf sub-agents
(`reviewer-business`, `reviewer-tests`, `reviewer-security`,
`reviewer-style`, `vet-reviewer`, `vet-implementer`,
`vet-simplifier`) directly. The leaf sub-agents each return one
short verdict block as their final message; only those final
messages — not the per-persona reasoning — flow back into the
orchestrator's context.

Sub-agent final messages are **status hints, not state**. The
orchestrator always re-queries `speccy next SPEC-NNNN --json` after a
dispatch settles to get ground truth.

**Exit-code-stop contract.** If `speccy next SPEC-NNNN --json` exits
non-zero, the SPEC has reached a terminal state — halt the loop and
surface the stderr line to the caller. Only parse the JSON envelope
when exit code is 0.

## Startup integrity check

Before entering the dispatch loop, run one one-time sanity check
to catch state left by a crashed prior session.

**Reconcile-pass dispatch (REQ-007, REQ-008).** The very first
`speccy next SPEC-NNNN --json` of the session — the same one that
resolves the spec directory in step 1 below — may return
`next_action.kind == "reconcile"` when the CLI's consistency check
flags drift from a prior crashed session. When it does, dispatch the
reconcile pass per the shared partial inlined immediately below
**before** entering the dispatch loop. Apply the per-drift action
for every entry in `consistency.drifts[]`, then re-query
`speccy next SPEC-NNNN --json`. Only when the re-query returns
`consistency.status == "ok"` does this orchestrator proceed to the
dispatch loop. The reconcile pass is autonomous — no
`AskUserQuestion`, no "press enter to continue" surface, anywhere
in the dispatch path. Each subsequent loop iteration's
`speccy next SPEC-NNNN --json` (step 1 of the [Loop](#loop) section
below) re-runs the same check: if a per-task operation leaves
drift, the next iteration catches it and dispatches the reconcile
pass before continuing to the normal `work` / `review` / `ship`
dispatch. The reconcile pass owns every drift kind in REQ-006's
enum — including `state_in_progress_orphaned` (dirty tree) and
`state_in_progress_clean` (clean tree) — so the orchestrator no
longer scans TASKS.md for `state="in-progress"` itself.

<!-- Shared partial: reconcile-policy. Source: {{ speccy_references_path }}/reconcile-policy.md -->
{% include "modules/references/reconcile-policy.md" %}
<!-- End shared partial: reconcile-policy. -->

1. Resolve the spec directory from `speccy next SPEC-NNNN --json`:
   take the `spec_md_path` field and strip the trailing `/SPEC.md`
   to get `<spec-dir>`. If the command exits non-zero, surface the
   stderr line and stop — the SPEC is in a terminal state. If the
   command reports the spec is unknown, stop and report.

2. If `next_action.kind == "reconcile"`, run the reconcile pass per
   the shared partial above, then re-query and continue. Otherwise
   proceed directly to the dispatch loop.

## Loop

Repeat until a stop condition fires:

1. Query the CLI:

   ```bash
   speccy next SPEC-NNNN --json
   ```

   If the command exits non-zero, the SPEC has reached a terminal
   state (completed / dropped / superseded). Surface the stderr
   line to the caller and stop the loop. Only parse the JSON
   envelope when exit code is 0.

2. Dispatch on `next_action.kind`:

   - **`work`** — execute the [Work dispatch](#work-dispatch)
     section below.
   - **`review`** — execute the [Review dispatch](#review-dispatch)
     section below.
   - **`ship`** — execute the [Ship dispatch](#ship-dispatch)
     section below.
   - **`decompose`** — STOP. Tell the user to run
     `{{ cmd_prefix }}speccy-decompose` first; the orchestrator cannot
     loop on a spec without a task list.
   - **anything else** (unknown kind, missing field, `done`,
     `plan`, etc.) — STOP and report the observed `next_action`
     verbatim so the user can react.

3. After the dispatch settles (the one `speccy-work` sub-agent
   returns; or the four reviewer-* personas have all returned and
   this orchestrator has written the journal + TASKS.md; or the
   inline vet workflow has written its `<gate>` block), re-query
   `speccy next SPEC-NNNN --json` and loop from step 1.

## Work dispatch

**Clean-tree precondition (REQ-002).** Before spawning the
`speccy-work` sub-agent, run `git status --porcelain` in the
orchestrator's running session. If stdout is non-empty, **halt the
outer loop** and surface the dirty paths to the user — no
`speccy-work` sub-agent is spawned, and the loop does not advance.
Empty stdout permits the dispatch below. This check fires in the
orchestrator's session (not delegated to the sub-agent) because the
outer loop's invariant — every dispatched implementer turn begins on
a clean tree — must be enforced by the loop owner. The `speccy-work`
skill body re-runs the same check defensively at its own entry as a
second-level guard.

Spawn a sub-agent that runs the `speccy-work` primitive for the
resolved task. Prompt:

> Implement task `SPEC-NNNN/T-NNN` per the `speccy-work` skill.
> Single-task primitive; do not iterate. Keep your final message
> short — the caller reads `speccy next SPEC-NNNN --json` for ground truth.

Substitute the resolved `task_id` from `next_action.task_id`.

{% if host == "claude-code" %}Invoke the `Task` tool with `subagent_type: "speccy-work"`. The
sub-agent definition at `.claude/agents/speccy-work.md` carries
the host-native dispatch metadata.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
registered `speccy-work` sub-agent at
`.codex/agents/speccy-work.toml`.{% endif %}

## Review dispatch

Run the `speccy-review` skill body **inline in this orchestrator
session** (do NOT wrap it in a single general-purpose sub-agent —
that wrapper would need to spawn the four persona leaves, and
sub-agents cannot spawn sub-agents). The shared partial below is
the single source of truth, included by both this orchestrator's
review dispatch and the `{{ cmd_prefix }}speccy-review` skill
body.

{% include "modules/skills/partials/review-fanout.md" %}

After the write settles, increment the per-task retry counter if
the task flipped back to `pending` (this is what feeds the
5-round stop condition below). Then re-query
`speccy next SPEC-NNNN --json`.

The `speccy-review` skill remains independently invocable as
`{{ cmd_prefix }}speccy-review`; this orchestrator's review
dispatch shares the same fan-out contract via the partial above so
behaviour stays in sync across invocation paths.

## Ship dispatch

Run the `speccy-vet` skill body **inline in this orchestrator
session** (do NOT wrap it in a single general-purpose sub-agent —
that wrapper would need to spawn the vet-reviewer /
vet-implementer / vet-simplifier leaves across up to three rounds,
and sub-agents cannot spawn sub-agents). The shared partial below
is the single source of truth, included by both this
orchestrator's ship dispatch and the `{{ cmd_prefix }}speccy-vet`
skill body.

{% include "modules/skills/partials/vet-phases.md" %}

After the vet workflow appends its `<gate>` block and surfaces a
verdict to this orchestrator session, react as follows:

- `verdict="pass"` → write a one-line summary plus the round and
  simplifier counters, then **ask the user** whether to invoke
  `{{ cmd_prefix }}speccy-ship`. Only after explicit confirmation,
  spawn a `speccy-ship` sub-agent. Ship opens a PR; never
  auto-ship.
- `verdict="fail"` → surface the drift summary and one-line
  suggested next step. Stop the outer loop. The user decides how
  to address it (`{{ cmd_prefix }}speccy-amend`, manual edits,
  etc.).

## Stop conditions

- `verdict="pass"` from the holistic gate → ask the user before
  invoking ship.
- Same `task_id` flips back to `pending` after review for
  **5 rounds in a row** → stop. The implementer is stuck on this
  task. Surface the journal path
  (`.speccy/specs/NNNN-slug/journal/T-NNN.md`) so the user can
  read the blockers and decide whether to decompose
  (`{{ cmd_prefix }}speccy-amend` + `{{ cmd_prefix }}speccy-decompose`),
  pick a different model, or intervene by hand. Track per-task
  retry counts in memory across loop iterations; the budget of 5
  is the orchestrator's only per-task retry bound.
- A dispatched sub-agent errors out → stop and surface the error.
- `next_action.kind` is not one of `work`, `review`, `ship`,
  `decompose` → stop and report.
- User interrupts → stop on the next loop boundary.

## Status reporting

Before each dispatch, write one short status line so the user can
follow the loop without reading sub-agent transcripts:

```
SPEC-NNNN → work T-003
SPEC-NNNN → review T-003
SPEC-NNNN → work T-003 (retry 2/5 after blocking review)
SPEC-NNNN → holistic gate
SPEC-NNNN → ready to ship — confirm before proceeding?
```

Round numbers, per-persona verdicts, holistic drift findings, and
simplifier candidate details live inside sub-agent contexts and in
the per-task journals at
`.speccy/specs/NNNN-slug/journal/T-NNN.md` (and, for the holistic
gate, in `.speccy/specs/NNNN-slug/journal/VET.md`). Don't duplicate
them in the status line.

## Non-goals

- This skill does not run `speccy verify`, write `REPORT.md`, or
  open a PR. Those belong to `{{ cmd_prefix }}speccy-ship`, invoked
  after confirmation.
- The drift-fix loop and simplifier polish are **defined** in
  `{{ cmd_prefix }}speccy-vet` and re-described above only at the
  dispatch-shape level. The skill body of
  `{{ cmd_prefix }}speccy-vet` remains the source of truth for
  Phase 0–3 semantics — phase grammar bugs and `<gate>` block shape
  bugs get fixed there, not here. The orchestrator inlines the
  fan-out only because sub-agents cannot spawn sub-agents.
- This skill does not pick a different persona fan-out for review,
  retry blocked tasks with a different model, or split tasks
  automatically. Those are judgment calls; surfacing the stuck
  state to the user is the right move.
