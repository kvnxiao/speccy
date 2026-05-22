---
name: speccy-orchestrate
description: 'Drive the Speccy implementation + review loop for one SPEC end-to-end by chaining speccy-work, speccy-review, and speccy-holistic-gate until the spec is ready to ship. Use when the user says "orchestrate SPEC-NNNN", "speccy-orchestrate SPEC-NNNN", "run the full loop on SPEC-NNNN", "autopilot SPEC-NNNN", or wants to drive a spec from current state to ready-to-ship without chaining single-task skills by hand. Requires: an existing SPEC-NNNN with TASKS.md. Stops one step before shipping (calling speccy-ship is irreversible — it opens a PR) and asks the user before continuing. Do NOT trigger for ad-hoc "implement one task" or "review one task" asks — prefer speccy-work or speccy-review for single-task primitives.'
---

# speccy-orchestrate

Thin composition layer over the Speccy single-task primitives.
Queries `speccy next --json`, dispatches each step to a sub-agent,
and re-queries until the SPEC is ready to ship. This skill itself
holds no per-task or per-round state beyond the retry counter — all
heavy work happens in sub-agent contexts that exit when done.

## When to use

- The user wants to drive `SPEC-NNNN` from its current state to
  ready-to-ship without chaining `speccy-work`,
  `speccy-review`, and
  `speccy-holistic-gate` by hand.
- The SPEC already has a `TASKS.md` — this orchestrator dispatches
  against existing tasks; it does not plan or decompose.

Stops one step before invoking `speccy-ship`
(ship opens a PR — irreversible — and is always confirmed by the
user). Do not invoke this skill for ad-hoc "implement one task" or
"review one task" asks; prefer the single-task primitives.

## Argument

```
speccy-orchestrate SPEC-NNNN
```

The `SPEC-NNNN` argument is required. If missing, ask the user
which spec to drive and exit without looping.

## Lifecycle (three nested loops)

```
outer:    speccy-orchestrate dispatch loop  ← this skill
            └── on `next_action.kind`:
                  ├── work   → spawn speccy-work sub-agent
                  ├── review → spawn sub-agent that runs speccy-review
                  │              (inner: 4 reviewer personas fan out)
                  └── ship   → spawn sub-agent that runs
                               speccy-holistic-gate
                                 (inner: drift-fix loop + simplifier
                                  polish, up to 3 rounds)
inner-1:  per-task retry — same task_id flipping pending after review
            (bounded here in the orchestrator: 5 rounds, then stop)
inner-2:  holistic drift fix — owned by speccy-holistic-gate
            (bounded there: 3 rounds, then return fail)
inner-3:  simplifier polish — owned by speccy-holistic-gate
            (no loop: one scan + one apply with hygiene gate)
```

The orchestrator owns only the outer loop and inner-1's retry
counter. It never touches the bodies of inner-2 or inner-3 — those
live entirely inside the delegated skill's sub-agent and surface as
one final verdict block.

## Context discipline

Every dispatch goes through the host's sub-agent-spawn primitive so
the dispatched body runs in a fresh sub-agent context, not inline in
this orchestrator session. Inlining the delegated skill would bloat
the orchestrator's context with every sub-agent prompt it constructs;
sub-agent dispatch keeps long runs (10+ tasks × work + review +
holistic gate) inside one context window because only the
sub-agent's **final message** comes back.

Sub-agent final messages are **status hints, not state**. The
orchestrator always re-queries `speccy next --json` after each
dispatch to get ground truth.

## Startup integrity check

Before entering the dispatch loop, run one one-time sanity check
to catch state left by a crashed prior session.

1. Resolve the spec directory from `speccy next --json`: find the
   entry whose `spec_id` matches `SPEC-NNNN`, take its
   `spec_md_path` field, and strip the trailing `/SPEC.md` to get
   `<spec-dir>`. If no entry matches, stop and report — the spec
   is unknown.

2. Scan `<spec-dir>/TASKS.md` for any task at
   `state="in-progress"`:

   ```bash
   grep -n 'state="in-progress"' <spec-dir>/TASKS.md
   ```

   If any match, **stop and surface** before dispatching anything:

   ```
   SPEC-NNNN/T-NNN is at state="in-progress" — likely a crashed
   prior session. Resolve before re-running speccy-orchestrate:

   - To discard the prior attempt: edit TASKS.md and flip the
     task's state from "in-progress" back to "pending".
   - To inspect what was in progress: check
     <spec-dir>/journal/T-NNN.md for the last <implementer>
     block (if any was written before the crash) and the working
     tree for uncommitted edits.
   ```

   Exit. This is an autonomy-breaking recovery path on purpose —
   silently resetting state could lose real work, and silently
   resuming could re-run an implementer that already made
   progress.

3. If no in-progress tasks, proceed to the dispatch loop.

## Loop

Repeat until a stop condition fires:

1. Query the CLI:

   ```bash
   speccy next --json
   ```

   Find the entry in `specs[]` whose `spec_id` equals the requested
   `SPEC-NNNN`. If no entry matches, stop and report — the spec is
   either unknown or has no actionable next step.

2. Dispatch on `next_action.kind`:

   - **`work`** — spawn a sub-agent that runs the `speccy-work`
     primitive for the resolved task. Prompt:

     > Implement task `SPEC-NNNN/T-NNN` per the `speccy-work` skill.
     > Single-task primitive; do not iterate. Keep your final
     > message short — the caller reads `speccy next --json` for
     > ground truth.

     Substitute the resolved `task_id` from `next_action.task_id`.

     Invoke Codex's native sub-agent-spawn primitive against the
     registered `speccy-work` sub-agent at
     `.codex/agents/speccy-work.toml`.

   - **`review`** — spawn a sub-agent that runs the `speccy-review`
     primitive for the resolved task. Prompt:

     > Follow the `speccy-review` skill for task `SPEC-NNNN/T-NNN`.
     > The skill fans out four persona sub-agents and either flips
     > the task to `completed` or back to `pending` with blockers
     > in the journal. Return only a one-line verdict as your
     > final message:
     > `REVIEW SPEC-NNNN/T-NNN -> completed|pending`

     Substitute the resolved `task_id`. Running the review skill
     inside a sub-agent (rather than inline) keeps the four-persona
     fan-out chatter and journal write logic out of this
     orchestrator's context.

     Invoke Codex's native sub-agent-spawn primitive to spawn a
     sub-agent that loads and follows
     `.agents/skills/speccy-review/SKILL.md` with the prompt above.

   - **`ship`** — spawn a sub-agent that runs the
     `speccy-holistic-gate` primitive for the spec. Prompt:

     > Follow the `speccy-holistic-gate` skill for `SPEC-NNNN`.
     > The skill runs an autonomous drift-review + retry loop and
     > applies any simplifier candidates with a hygiene gate.
     > Return only the final `<orchestrator-verdict>` block as
     > your final message.

     Invoke Codex's native sub-agent-spawn primitive to spawn a
     sub-agent that loads and follows
     `.agents/skills/speccy-holistic-gate/SKILL.md` with the
     prompt above.

     When the sub-agent returns, parse the verdict block:

     - `verdict="pass"` → surface the one-line summary plus the
       round and simplifier counters, then **ask the user** whether
       to invoke `speccy-ship`. Only after explicit
       confirmation, spawn a `speccy-ship` sub-agent. Ship opens a
       PR; never auto-ship.
     - `verdict="fail"` → surface the drift summary and suggested
       next step from the verdict. Stop the loop. The user decides
       how to address it (`speccy-amend`, manual
       edits, etc.).

   - **`decompose`** — STOP. Tell the user to run
     `speccy-tasks` first; the orchestrator cannot
     loop on a spec without a task list.

   - **anything else** (unknown kind, missing field, `done`,
     `plan`, etc.) — STOP and report the observed `next_action`
     verbatim so the user can react.

3. After the dispatched sub-agent returns, re-query
   `speccy next --json` and loop from step 1.

## Stop conditions

- `verdict="pass"` from the holistic gate → ask the user before
  invoking ship.
- Same `task_id` flips back to `pending` after review for
  **5 rounds in a row** → stop. The implementer is stuck on this
  task. Surface the journal path
  (`.speccy/specs/NNNN-slug/journal/T-NNN.md`) so the user can
  read the blockers and decide whether to decompose
  (`speccy-amend` + `speccy-tasks`),
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
`.speccy/specs/NNNN-slug/journal/T-NNN.md`. Don't duplicate them
in the status line.

## Non-goals

- This skill does not run `speccy verify`, write `REPORT.md`, or
  open a PR. Those belong to `speccy-ship`, invoked
  after confirmation.
- This skill does not own the drift-fix loop or the simplifier
  polish — those live in `speccy-holistic-gate`.
  Bugs in those loops get fixed there, not here.
- This skill does not pick a different persona fan-out for review,
  retry blocked tasks with a different model, or split tasks
  automatically. Those are judgment calls; surfacing the stuck
  state to the user is the right move.


## Codex sub-agent-spawn permission grant

Codex requires an **explicit user grant** before any skill is allowed
to spawn sub-agents. Without the grant, the dispatch steps above
return a permission error instead of invoking
`speccy-work`, `speccy-review`, or
`speccy-holistic-gate`, and the outer loop cannot
make progress.

### Granting the permission

On first invocation, Codex prompts the user once per session to
authorize sub-agent spawning for this skill. Approve the prompt to
proceed; the grant is scoped to the current Codex session by default.

To persist the grant across sessions for this skill, add the
following to your Codex project configuration (typically
`.codex/config.toml`):

```toml
[skills.speccy-orchestrate]
allow_subagent_spawn = true
```

With the entry in place, the orchestrator dispatches sub-agents
without an interactive prompt on every session.

### Revoking the permission

Remove the `allow_subagent_spawn` entry from `.codex/config.toml` (or
flip it to `false`), then restart Codex. The next invocation of
`speccy-orchestrate` will prompt for the grant again.

### Why this exists

Sub-agent spawn is a privileged operation: each spawn launches a
delegated execution context with its own model, tool surface, and
working-tree access. Requiring an explicit grant keeps the
sub-agent boundary visible to the user — a skill cannot silently
fan out unbounded delegated work.

This grant is **Codex-specific**. Claude Code's `Task` tool does not
require an equivalent permission step — Claude approves tool calls
through the standard tool-use confirmation flow rather than a
skill-level permission gate.

For background on Codex's sub-agent model, see
`developers.openai.com/codex/concepts/subagents`.
