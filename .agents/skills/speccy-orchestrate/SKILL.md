---
name: speccy-orchestrate
description: 'Drive the Speccy implementation + review loop for one SPEC end-to-end by chaining speccy-work, speccy-review, and speccy-vet until the spec is ready to ship. Use when the user says "orchestrate SPEC-NNNN", "speccy-orchestrate SPEC-NNNN", "run the full loop on SPEC-NNNN", "autopilot SPEC-NNNN", or wants to drive a spec from current state to ready-to-ship without chaining single-task skills by hand. Requires: an existing SPEC-NNNN with TASKS.md. Stops one step before shipping (calling speccy-ship is irreversible — it opens a PR) and asks the user before continuing. Do NOT trigger for ad-hoc "implement one task" or "review one task" asks — prefer speccy-work or speccy-review for single-task primitives.'
---

# speccy-orchestrate

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
  ready-to-ship without chaining `speccy-work`,
  `speccy-review`, and
  `speccy-vet` by hand.
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

## Lifecycle (outer loop + inline fan-outs)

```
outer:    speccy-orchestrate dispatch loop  ← this skill's session
            └── on `next_action.kind`:
                  ├── work   → spawn ONE speccy-work sub-agent
                  ├── review → fan out reviewer-* sub-agents
                  │             in parallel from this session
                  │             (each self-appends its <review> via
                  │              `speccy journal append`; orchestrator
                  │              flips state via `speccy task transition`)
                  ├── vet    → run the speccy-vet skill body inline
                  │             in this session, spawning
                  │             vet-reviewer / vet-implementer /
                  │             vet-simplifier leaf sub-agents directly
                  │             (drift-fix loop bounded to 3 rounds,
                  │              then one simplifier polish pass)
                  └── ship   → ask the user, then spawn a
                                speccy-ship sub-agent on confirm
inner-1:  per-task retry — same task_id flipping pending after review
            (bounded here in the orchestrator: 5 rounds, then stop)
inner-2:  holistic drift fix — described in speccy-vet, run inline
            here (bounded: 3 rounds, then fail)
inner-3:  simplifier polish — described in speccy-vet, run inline
            here (no loop: one scan + one apply with hygiene gate)
```

The orchestrator owns the outer loop, the per-task retry counter,
the review consolidation step (verify completeness via `speccy
journal show`, flip state via `speccy task transition`, append the
consolidated `<blockers>` via `speccy journal append`), and the vet
round counter / snapshot management. Only the leaf work (one
implementer pass, one persona review, one drift review, one drift
fix, one simplifier scan / apply) lives in a spawned sub-agent. All
lifecycle writes the orchestrator performs go through the CLI verbs
(`task transition`, `journal append`, `journal show`); it never edits
TASKS.md `state` attributes or journal files with file-editing tools.

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
reconcile pass per the **Reconcile policy** summary below (canonical
policy at `.agents/speccy-references/reconcile-policy.md`)
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

**Reconcile policy.** When `speccy next SPEC-NNNN --json` returns
`next_action.kind == "reconcile"`, iterate `consistency.drifts[]` and
apply the table action per entry, then re-query before proceeding.
See `.agents/speccy-references/reconcile-policy.md` for the full
policy table, the three properties the dispatch holds by construction
(autonomous / rollback-biased / idempotent), and the extension
protocol for adding new drift kinds.

1. Resolve the spec directory from `speccy next SPEC-NNNN --json`:
   take the `spec_md_path` field and strip the trailing `/SPEC.md`
   to get `<spec-dir>`. If the command exits non-zero, surface the
   stderr line and stop — the SPEC is in a terminal state. If the
   command reports the spec is unknown, stop and report.

2. If `next_action.kind == "reconcile"`, run the reconcile pass per
   the **Reconcile policy** summary above (canonical policy at
   `.agents/speccy-references/reconcile-policy.md`), then re-query
   and continue. Otherwise proceed directly to the dispatch loop.

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
   - **`vet`** — execute the [Vet dispatch](#vet-dispatch) section
     below.
   - **`ship`** — execute the [Ship dispatch](#ship-dispatch)
     section below.
   - **`decompose`** — STOP. Tell the user to run
     `speccy-decompose` first; the orchestrator cannot
     loop on a spec without a task list.
   - **anything else** (unknown kind, missing field, `done`,
     `plan`, etc.) — STOP and report the observed `next_action`
     verbatim so the user can react.

3. After the dispatch settles (the one `speccy-work` sub-agent
   returns; or the reviewer-* personas have all returned, the
   orchestrator has verified completeness via `speccy journal show`
   and flipped state via `speccy task transition`; or the inline vet
   workflow has appended its `<gate>` block via `speccy journal
   append`), re-query `speccy next SPEC-NNNN --json` and loop from
   step 1.

## Work dispatch

**Clean-tree precondition (SPEC-0045 REQ-002, extended by SPEC-0047
REQ-002).** Before spawning the `speccy-work` sub-agent, run three
steps in the orchestrator's running session:

1. Resolve the target task from `next_action.task_id`.
2. Read `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the
   retry-shape rule summarized immediately below (canonical
   statement at `.agents/speccy-references/retry-shape.md`).
3. Run `git status --porcelain`. **First-attempt shape** with
   non-empty stdout halts the outer loop and surfaces the dirty
   paths to the user — no `speccy-work` sub-agent is spawned, and
   the loop does not advance (today's SPEC-0045/REQ-002 behaviour,
   unchanged). Empty stdout proceeds to the dispatch below.
   **Retry shape** proceeds to spawn the `speccy-work` sub-agent
   regardless of stdout — the dirty paths are the prior pass's WIP
   that the retry implementer amends in place; no dirty-paths
   surface is written. The retry case is the one named by the
   `SPEC-NNNN → work T-NNN (retry 2/5 after blocking review)`
   status line below.

This check fires in the orchestrator's session (not delegated to
the sub-agent) because the outer loop's invariant — every
dispatched implementer turn begins on a known tree shape — must be
enforced by the loop owner. The `speccy-work` skill body re-runs
the same retry-aware precondition defensively at its own entry as
a second-level guard.

**Retry shape.** A task is in retry shape iff its journal contains
both an `<implementer>` element and a `<blockers>` element whose
`round` attribute matches the highest implementer round. Otherwise
it's first-attempt shape — the strict clean-tree gate applies. See
`.agents/speccy-references/retry-shape.md` for the full rule
statement, read-only scope, worked examples, and the
"implementer awaiting review" edge case.

Spawn a sub-agent that runs the `speccy-work` primitive for the
resolved task. Prompt:

> Implement task `SPEC-NNNN/T-NNN` per the `speccy-work` skill.
> Single-task primitive; do not iterate. Keep your final message
> short — the caller reads `speccy next SPEC-NNNN --json` for ground truth.

Substitute the resolved `task_id` from `next_action.task_id`.

Invoke Codex's native sub-agent-spawn primitive against the
registered `speccy-work` sub-agent at
`.codex/agents/speccy-work.toml`.

## Review dispatch

Run the `speccy-review` skill body **inline in this orchestrator
session** (do NOT wrap it in a single general-purpose sub-agent —
that wrapper would need to spawn the four persona leaves, and
sub-agents cannot spawn sub-agents). The shared partial below is
the single source of truth, included by both this orchestrator's
review dispatch and the `speccy-review` skill
body.


Fan out five reviewer-* sub-agents in parallel against the resolved
task, one per persona. Default fan-out: `reviewer-business`,
`reviewer-tests`, `reviewer-security`, `reviewer-style`,
`reviewer-correctness`. Two additional personas
(`reviewer-architecture`, `reviewer-docs`) are off the default
fan-out and are invoked explicitly when an architectural or
documentation risk is suspected.

The prompt for each spawn is:

> Review task `SPEC-NNNN/T-NNN`. Run `speccy check SPEC-NNNN/T-NNN`
> to load the task scenarios, read the bare `<task>` body in
> TASKS.md and the prior activity in
> `.speccy/specs/NNNN-slug/journal/T-NNN.md`, and apply your
> persona's review criteria. Append your own `<review>` block to the
> per-task journal by running
> `speccy journal append SPEC-NNNN/T-NNN --block review --persona <persona> --verdict <pass|blocking> --model <your-model>`
> with the review body on stdin, then return a thin self-closing
> `<verdict persona="<persona>" verdict="..." model="..." rationale="..." />`
> element as your final message (per the verdict-return contract). The
> `--model` value is required and must identify the model that produced
> the verdict (with the optional slash-suffix effort convention from the
> verdict-return contract). Do not edit TASKS.md. The journal write goes
> through `speccy journal append` — do not use file-editing tools on the
> journal.
>
> The working tree may be dirty: the implementer leaves changes
> uncommitted on purpose, and the orchestrator (not the implementer)
> owns the single atomic commit on review pass per REQ-003/REQ-004.
> On retry rounds the dirty tree is the prior pass's WIP that the
> retry implementer amended in place per the retry-shape contract.
> Do not flag uncommitted state, commit timing, or "changes not
> committed before the in-review flip" -- those are out of scope
> for per-task review.

Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name per
spawn.

Invoke Codex's native sub-agent-spawn primitive five times in
parallel against the registered Codex sub-agents
`reviewer-business`, `reviewer-tests`, `reviewer-security`,
`reviewer-style`, and `reviewer-correctness`. Each persona's TOML
file at `.codex/agents/reviewer-<persona>.toml` carries the
sub-agent's developer instructions.

Canonical journal `<review>` shape:
`.agents/speccy-references/journal-review.md`.

Canonical journal `<blockers>` shape:
`.agents/speccy-references/journal-blockers.md`.

Each reviewer sub-agent appends its own `<review>` block to
`.speccy/specs/NNNN-slug/journal/T-NNN.md` via `speccy journal
append --block review` before returning a thin `<verdict>` element
(see `resources/modules/personas/verdict_return_contract.md`). The
CLI's per-file lock serializes those parallel appends, so the
running session never transcribes `<review>` blocks itself and never
edits the journal with file-editing tools. The orchestrator's job
after the fan-out settles is to **verify completeness, read back any
blockers, then drive the state flip through the CLI verbs**.

### Step 1 — verify the round's reviews are complete

Read back the appended `<review>` blocks for the round under review
through the CLI rather than trusting the returned thin verdicts:

```bash
speccy journal show SPEC-NNNN/T-NNN --json --block review --round latest
```

Confirm every persona you spawned appears in the result for the
latest round. If a persona is missing (its append failed, or its
sub-agent errored before appending), halt the fan-out and surface
the missing persona rather than flipping state on an incomplete
round. Do not parse the journal file by hand — `journal show` is the
read-back authority.

### Step 2 — drive the state flip through `speccy task transition`

Decide pass vs blocking from the verdicts the reviewers appended,
then flip the task's `state` with the transition command — never by
editing the `state` attribute in TASKS.md directly:

- If every spawned reviewer's appended `<review verdict="...">` is
  `verdict="pass"`, flip `in-review` → `completed`:

      speccy task transition SPEC-NNNN/T-NNN --to completed

- If any spawned reviewer's `<review verdict="...">` is
  `verdict="blocking"`, flip `in-review` → `pending`:

      speccy task transition SPEC-NNNN/T-NNN --to pending

  then append a single consolidated `<blockers>` block (step 3).

### Step 3 — consolidate blockers via `speccy journal append`

On a blocking round, read back the failing reviews and write **one**
consolidated `<blockers>` block — not one per reviewer, not a partial
write. Read the blocking review bodies through the CLI:

```bash
speccy journal show SPEC-NNNN/T-NNN --json --verdict blocking --round latest
```

The `<blockers>` **body is orchestrator-authored semantic judgment**
(DEC-001 non-goal: the CLI never synthesizes blocker prose). Compose
the body from the blocking reviews you just read back, then append it
with the body on stdin:

```bash
speccy journal append SPEC-NNNN/T-NNN --block blockers <<'EOF'
<one-line summary of what to change before the next implementer pass>.
<optional bullets enumerating each persona's blocker>.
EOF
```

The CLI is the sole authority for the block's `date` and `round` and
emits the paired `<blockers>…</blockers>` element — **do not compute,
supply, or hand-author `date`, `round`, or the open/close tags**.
There is no flag to override them; the body you pipe is the inner
text only. Validation runs before any write; a malformed body leaves
the journal byte-identical.

The single-writer rule holds: the CLI's append lock owns write
serialization across the parallel reviewer appends and this
consolidated `<blockers>` append, and the orchestrator remains the
sole author of `<blockers>` bodies (and, per the commit step below,
of git commits). The running session issues only CLI verbs — `journal
show`, `journal append`, `task transition` — for the review-induced
journal and state writes; it never edits TASKS.md or the journal file
with file-editing tools.

### Atomic commit on review pass (REQ-003, REQ-004)

When every spawned reviewer returned `verdict="pass"` and the
`speccy task transition … --to completed` flip has run (the reviewer
`<review>` appends already landed via the CLI during the fan-out),
the running session performs the commit step:

1. Run `git status --porcelain`. If stdout is empty, **skip the
   commit step silently** (no surface to the user, no error). This
   handles two cases uniformly: tasks whose net filesystem change is
   zero, and idempotent re-entry from the reconcile pass against an
   already-converged state.
2. If stdout is non-empty, run `git add -A` followed by `git commit`
   with the message format below. The commit captures the
   implementer's code changes, the TASKS.md state flip, and the
   journal append in a single atomic commit (parent count = 1).

Commit message format (REQ-004):

- **Title:** `[SPEC-NNNN/T-NNN]: <task title>` — `<task title>` is
  read verbatim from the `<task>` element's `## ` heading in
  TASKS.md (the one-line H2 immediately after the `<task ...>`
  opening tag). Substitute the resolved spec and task IDs.
- **Body:** the trimmed content of the `Completed` field from the
  latest `<implementer>` block in the per-task journal file. Extract
  mechanically as the bytes between the `- Completed:` bullet marker
  and the next `- <Field>:` bullet marker (one of `Undone`,
  `Hygiene checks`, `Evidence`, `Discovered issues`,
  `Procedural compliance`). Trim leading and trailing whitespace.
- **Trailer:** a single `Co-Authored-By: <model> <noreply@anthropic.com>`
  line where `<model>` is the model segment sourced per the
  "Sourcing your recorded identity" rule — the host's in-context
  identifier transcribed verbatim in hyphen form (e.g.
  `claude-opus-4-8[1m]`), never a dotted form or a configured alias.
  When the host states no resolved identifier in-context, use the
  documented fallback string
  `Co-Authored-By: Speccy Skill Pack <noreply@anthropic.com>`.

Pass the body via a HEREDOC so newlines and special characters
survive verbatim, e.g.:

```
git commit -m "$(cat <<'EOF'
[SPEC-NNNN/T-NNN]: <task title>

<trimmed Completed field>

Co-Authored-By: <model> <noreply@anthropic.com>
EOF
)"
```

The title prefix is the sole task-identity link in git history; the
consistency check correlates commits to tasks by grepping for this
prefix. Do not stage selectively — `git add -A` is sound under the
clean-tree precondition (REQ-002) that fires at the start of work
dispatch, which guarantees every dirty path at commit time is
task-scoped.

The skill body does not check the current git branch; it trusts the
caller / host to have placed the working tree on a feature branch.
Commits land on whatever HEAD is.


After the write settles, increment the per-task retry counter if
the task flipped back to `pending` (this is what feeds the
5-round stop condition below). Then re-query
`speccy next SPEC-NNNN --json`.

The `speccy-review` skill remains independently invocable as
`speccy-review`; this orchestrator's review
dispatch shares the same fan-out contract via the partial above so
behaviour stays in sync across invocation paths.

## Vet dispatch

Run the `speccy-vet` skill body **inline in this orchestrator
session** (do NOT wrap it in a single general-purpose sub-agent —
that wrapper would need to spawn the vet-reviewer /
vet-implementer / vet-simplifier leaves across up to three rounds,
and sub-agents cannot spawn sub-agents). The vet-phases grammar
lives canonically in the `speccy-vet` skill body
(which includes the `modules/skills/partials/vet-phases.md`
partial); this site carries only a pointer summary so the two
invocation paths stay in sync without duplicating the grammar.

**Vet phases.** Phase 0 bootstraps the journal; Phase 1 runs drift
review with an autonomous fix-and-retry loop; Phase 2 runs the
simplifier polish pass; Phase 3 writes the final `<gate>` block. Run
in order; see `.agents/skills/speccy-vet/SKILL.md` § Phase N for the full grammar.

After the vet workflow appends its `<gate>` block and surfaces a
verdict to this orchestrator session, react as follows:

- `verdict="pass"` → write a one-line summary plus the round and
  simplifier counters, then re-query
  `speccy next SPEC-NNNN --json`. The next iteration will observe
  `next_action.kind == "ship"` and route to the [Ship
  dispatch](#ship-dispatch) section below.
- `verdict="fail"` → surface the drift summary and one-line
  suggested next step. Stop the outer loop. The user decides how
  to address it (`speccy-amend`, manual edits,
  etc.).

## Ship dispatch

The `ship` kind is emitted by the CLI after a fresh passing
vet-gate artifact lands and `REPORT.md` is absent, so the vet
workflow has already completed and the only remaining step is user
confirmation before opening a PR.

Ask the user via the Codex equivalent user-prompt primitive whether to invoke
`speccy-ship` now. Ship opens a PR — irreversible —
so this confirmation is always explicit; never auto-ship.

- On confirm: spawn a `speccy-ship` sub-agent.
  Invoke Codex's native sub-agent-spawn primitive against the
  registered `speccy-ship` sub-agent at
  `.codex/agents/speccy-ship.toml`.
- On decline: stop the outer loop.

## Stop conditions

- `ship` dispatch declined by the user → stop the outer loop.
- Same `task_id` flips back to `pending` after review for
  **5 rounds in a row** → stop. The implementer is stuck on this
  task. Surface the journal path
  (`.speccy/specs/NNNN-slug/journal/T-NNN.md`) so the user can
  read the blockers and decide whether to decompose
  (`speccy-amend` + `speccy-decompose`),
  pick a different model, or intervene by hand. Track per-task
  retry counts in memory across loop iterations; the budget of 5
  is the orchestrator's only per-task retry bound.
- A dispatched sub-agent errors out → stop and surface the error.
- `next_action.kind` is not one of `work`, `review`, `vet`, `ship`,
  `decompose` → stop and report.
- User interrupts → stop on the next loop boundary.

## Status reporting

Before each dispatch, write one short status line so the user can
follow the loop without reading sub-agent transcripts:

```
SPEC-NNNN → work T-003
SPEC-NNNN → review T-003
SPEC-NNNN → work T-003 (retry 2/5 after blocking review)
SPEC-NNNN → vet
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
  open a PR. Those belong to `speccy-ship`, invoked
  after confirmation.
- The drift-fix loop and simplifier polish are **defined** in
  `speccy-vet` and re-described above only at the
  dispatch-shape level. The skill body of
  `speccy-vet` remains the source of truth for
  Phase 0–3 semantics — phase grammar bugs and `<gate>` block shape
  bugs get fixed there, not here. The orchestrator inlines the
  fan-out only because sub-agents cannot spawn sub-agents.
- This skill does not pick a different persona fan-out for review,
  retry blocked tasks with a different model, or split tasks
  automatically. Those are judgment calls; surfacing the stuck
  state to the user is the right move.


## Codex sub-agent-spawn permission grant

Codex requires an **explicit user grant** before any skill is allowed
to spawn sub-agents. Without the grant, the dispatch steps above
return a permission error instead of spawning `speccy-work`,
the four `reviewer-*` personas, or the `vet-*` leaf sub-agents,
and the outer loop cannot make progress.

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
