---
name: speccy-orchestrate
description: 'Drive the Speccy implementation + review loop for one SPEC end-to-end by chaining speccy-work, speccy-review, and speccy-vet until the spec is ready to ship. Use when the user says "orchestrate SPEC-NNNN", "speccy-orchestrate SPEC-NNNN", "run the full loop on SPEC-NNNN", "autopilot SPEC-NNNN", or wants to drive a spec from current state to ready-to-ship without chaining single-task skills by hand. Requires: an existing SPEC-NNNN with TASKS.md. Stops one step before shipping (calling speccy-ship is irreversible — it opens a PR) and asks the user before continuing. Do NOT trigger for ad-hoc "implement one task" or "review one task" asks — prefer speccy-work or speccy-review for single-task primitives.'
---

# speccy-orchestrate

Thin composition layer over the Speccy single-task primitives.
Queries `speccy next --json`, dispatches each step to one or more
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
orchestrator always re-queries `speccy next --json` after a
dispatch settles to get ground truth.

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

   - **`work`** — execute the [Work dispatch](#work-dispatch)
     section below.
   - **`review`** — execute the [Review dispatch](#review-dispatch)
     section below.
   - **`ship`** — execute the [Ship dispatch](#ship-dispatch)
     section below.
   - **`decompose`** — STOP. Tell the user to run
     `speccy-tasks` first; the orchestrator cannot
     loop on a spec without a task list.
   - **anything else** (unknown kind, missing field, `done`,
     `plan`, etc.) — STOP and report the observed `next_action`
     verbatim so the user can react.

3. After the dispatch settles (the one `speccy-work` sub-agent
   returns; or the four reviewer-* personas have all returned and
   this orchestrator has written the journal + TASKS.md; or the
   inline vet workflow has written its `<gate>` block), re-query
   `speccy next --json` and loop from step 1.

## Work dispatch

Spawn a sub-agent that runs the `speccy-work` primitive for the
resolved task. Prompt:

> Implement task `SPEC-NNNN/T-NNN` per the `speccy-work` skill.
> Single-task primitive; do not iterate. Keep your final message
> short — the caller reads `speccy next --json` for ground truth.

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


Fan out four reviewer-* sub-agents in parallel against the resolved
task, one per persona. Default fan-out: `reviewer-business`,
`reviewer-tests`, `reviewer-security`, `reviewer-style`. Two
additional personas (`reviewer-architecture`, `reviewer-docs`) are
off the default fan-out and are invoked explicitly when an
architectural or documentation risk is suspected.

The prompt for each spawn is:

> Review task `SPEC-NNNN/T-NNN`. Run `speccy check SPEC-NNNN/T-NNN`
> to load the task scenarios, read the bare `<task>` body in
> TASKS.md and the prior activity in
> `.speccy/specs/NNNN-slug/journal/T-NNN.md`, and apply your
> persona's review criteria. Return your verdict as your final
> message as a
> `<review persona="<persona>" verdict="..." model="...">…</review>`
> element block. The `model` attribute is required and must
> identify the model that produced the verdict (with the optional
> slash-suffix effort convention from the verdict-return contract).
> Do not edit TASKS.md and do not edit the journal file.

Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name per
spawn.

Invoke Codex's native sub-agent-spawn primitive four times in
parallel against the registered Codex sub-agents
`reviewer-business`, `reviewer-tests`, `reviewer-security`, and
`reviewer-style`. Each persona's TOML file at
`.codex/agents/reviewer-<persona>.toml` carries the sub-agent's
developer instructions.

Canonical journal `<review>` shape:
`.agents/speccy-references/journal-review.md`.

Canonical journal `<blockers>` shape:
`.agents/speccy-references/journal-blockers.md`.

After all spawned sub-agents return, **consolidate** the `<review>`
element blocks from each reviewer's final message and append them
to `.speccy/specs/NNNN-slug/journal/T-NNN.md` **serially in the
running session** — do not delegate the write back to a reviewer
sub-agent, and do not write to TASKS.md.

When transcribing each returned `<review>` into the journal:

- Copy the `model` attribute **verbatim** from the reviewer's reply
  per `resources/modules/personas/verdict_return_contract.md`. Do
  not infer a model value from the persona name, the host
  skill-pack identity, or any other source. If a returned
  `<review>` is missing `model`, halt the fan-out and surface the
  non-conforming persona rather than inventing a value.
- Ensure each appended `<review>` carries the full required
  attribute set: `date` (ISO8601 with seconds and timezone),
  `model` (verbatim from the reviewer), `persona`, `verdict`
  (`pass` or `blocking`), and `round` (positive integer matching
  the implementer round under review). All five are required.
- If `journal/T-NNN.md` does not exist yet (a task can reach
  `in-review` only after the implementer wrote its round-1
  `<implementer>` block, so this should be rare — but if the file
  is somehow missing, surface that as an error rather than
  silently creating one without the implementer entry).

Apply the state transition to **TASKS.md serially in the running
session** (separate write from the journal append):

- If every spawned reviewer's `<review verdict="...">` is
  `verdict="pass"`, flip the task's `state="..."` attribute from
  `in-review` to `completed`.
- If any spawned reviewer's `<review verdict="...">` is
  `verdict="blocking"`, flip `state="..."` from `in-review` to
  `pending`, and append a single consolidated
  `<blockers>…</blockers>` element block to `journal/T-NNN.md`
  that aggregates all failing reviewers' feedback — not one
  `<blockers>` per reviewer, not a partial write. The block
  carries required attributes `date` and `round` (matching the
  round of the `<review>` blocks just appended) and has the form:

      <blockers date="2026-05-21T22:10:00Z" round="1">
      <one-line summary of what to change before the next
      implementer pass>.
      <optional bullets enumerating each persona's blocker>.
      </blockers>

This serial write in the running session eliminates the
parallel-write race that would occur if each reviewer sub-agent
wrote to the journal or TASKS.md directly (per DEC-008). Per-task
journal files do not introduce parallel writes from reviewer
sub-agents — the running session remains the sole journal writer
during review.


After the write settles, increment the per-task retry counter if
the task flipped back to `pending` (this is what feeds the
5-round stop condition below). Then re-query `speccy next --json`.

The `speccy-review` skill remains independently invocable as
`speccy-review`; this orchestrator's review
dispatch shares the same fan-out contract via the partial above so
behaviour stays in sync across invocation paths.

## Ship dispatch

Run the `speccy-vet` skill body **inline in this orchestrator
session** (do NOT wrap it in a single general-purpose sub-agent —
that wrapper would need to spawn the vet-reviewer /
vet-implementer / vet-simplifier leaves across up to three rounds,
and sub-agents cannot spawn sub-agents). The shared partial below
is the single source of truth, included by both this
orchestrator's ship dispatch and the `speccy-vet`
skill body.


Round budget: **3 rounds per invocation** for drift fixing. Each
round is expensive (full SPEC re-read + diff re-analysis +
implementer pass), so the budget of 3 is intentionally tighter
than the per-task implementer retry budget.

### Phase 0 — bootstrap

Resolve the three values that sub-agent prompts need, then open a
new invocation section in VET.md.

1. **Spec directory.** Run:

   ```bash
   speccy next --json
   ```

   Find the entry in `specs[]` whose `spec_id` equals the requested
   `SPEC-NNNN`. Its `spec_md_path` field (e.g.,
   `.speccy/specs/NNNN-slug/SPEC.md`) gives the absolute path to
   `SPEC.md`; strip the trailing `/SPEC.md` to get `<spec-dir>`
   (e.g., `.speccy/specs/NNNN-slug/`). If no entry matches, return
   `fail` immediately — the spec is unknown.

   Also verify every task in this spec is at `state="completed"`
   (read `<spec-dir>/TASKS.md`). If any task is `pending`,
   `in-progress`, or `in-review`, return `fail` — this is a
   pre-ship gate, not a mid-loop check.

2. **Diff baseline ref.** Run:

   ```bash
   git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@'
   ```

   Use the output as `<base-ref>`. If empty (no remote, detached
   HEAD), fall back to `main`. Sub-agent prompts will pass this in
   for `git diff <base-ref>` — that command compares the **working
   tree** against the ref, including uncommitted changes, which is
   essential because the drift-implementer leaves changes
   uncommitted between rounds.

3. **Journal bootstrap and new invocation section.** The journal
   is at `<spec-dir>/journal/VET.md`.

   - If the file does not exist, create it with the YAML
     frontmatter (`spec`, `generated_at`).
   - Scan the file for `^## Invocation (\d+)` headers, take the
     max, and add 1 to get the new invocation number `N`. If no
     prior headers exist, `N = 1`.
   - Append a new section header:

     ```markdown

     ## Invocation N — <ISO8601 timestamp>
     ```

     (Blank line above the heading for markdown readability.) Do
     not modify prior sections, even if a prior invocation
     crashed mid-loop.

### Phase 1 — drift review and fix

Repeat for up to 3 rounds per invocation. The running session owns
the round counter, the working-tree snapshots, and the VET.md
writes; sub-agents own the substantive review and fix work.

**Defer-write pattern.** Hold returned verdict blocks in memory
across each round and write to VET.md only **after** the
snapshot-keep-vs-revert decision. Writing earlier would put VET.md
changes inside the snapshot, and a stuck-revert would erase the
audit trail. The journal is the durable record of *what the loop
did*; it must survive any rollback the loop performs.

1. **Spawn the drift reviewer sub-agent.** Prompt:

   > Holistic drift review for `SPEC-NNNN`, invocation `N`, round
   > `R`.
   >
   > Resolved paths:
   > - Spec directory: `<spec-dir>` (use this for `SPEC.md`,
   >   `TASKS.md`, mission file if any, and the journal at
   >   `<spec-dir>/journal/VET.md`).
   > - Diff baseline: `<base-ref>` (run `git diff <base-ref>` —
   >   that captures the working tree including uncommitted
   >   changes, which the implementer leaves between rounds).
   >
   > Follow the focus, round-2+ scrutiny, and verdict-return
   > contract in your agent file. Return a single `<drift-review>`
   > block as your final message.

   Substitute `SPEC-NNNN`, `N`, `R`, `<spec-dir>`, and `<base-ref>`
   with the resolved values. Hold the returned `<drift-review>`
   block in memory; do not write to VET.md yet.

   Invoke Codex's native sub-agent-spawn primitive against the
   registered `vet-reviewer` sub-agent at
   `.codex/agents/vet-reviewer.toml`.

2. **If `verdict="pass"`** → append the held `<drift-review>`
   block to `<spec-dir>/journal/VET.md` under the current
   invocation section. Exit the loop and go to Phase 2.

3. **If `verdict="blocking"` and no rounds remain** → append the
   held `<drift-review>` block to VET.md (so the trail is
   complete) and return a `fail` verdict.

4. **Otherwise** (`verdict="blocking"` with budget remaining):

   a. **Snapshot the working tree** before the implementer call,
      so the running session can revert on `stuck` without losing
      the VET.md writes:

      ```bash
      git stash push --include-untracked -m "speccy-holistic-pre-implementer-<spec>-inv<N>-r<R>"
      git stash apply
      ```

      The `push` saves all uncommitted state and clears the working
      tree to HEAD; the `apply` restores the working tree so the
      implementer has the current implementation to work on. The
      stash stays available as the rollback target.

   b. **Spawn the drift-implementer sub-agent.** Prompt:

      > Holistic drift fix for `SPEC-NNNN`, invocation `N`, round
      > `R`.
      >
      > Resolved paths:
      > - Spec directory: `<spec-dir>`.
      > - Diff baseline: `<base-ref>` (use `git diff <base-ref>`
      >   to see the existing implementation; leave your changes
      >   uncommitted — the next reviewer reads the same command
      >   and will pick them up).
      >
      > The running session will revert your changes if you
      > return `verdict="stuck"`. Do not manage rollback yourself.
      >
      > Follow the scope, hygiene-gate, and verdict-return
      > contract in your agent file. Return a single
      > `<holistic-fix>` block as your final message.
      >
      > Drift findings (the held `<drift-review>` block):
      >
      > [paste the held `<drift-review>` block verbatim]

      Hold the returned `<holistic-fix>` block in memory.

      Invoke Codex's native sub-agent-spawn primitive against the
      registered `vet-implementer` sub-agent at
      `.codex/agents/vet-implementer.toml`.

   c. **Resolve the snapshot based on the implementer's verdict**:

      - **`addressed` or `blocking`**: keep the implementer's
        edits.

        ```bash
        git stash drop
        ```

        Then append **both** the held `<drift-review>` block and
        the held `<holistic-fix>` block to VET.md under the
        current invocation section (drift-review first, then
        fix). Decrement the round counter and go back to step 1.
        The next reviewer reads the journal you just appended and
        verifies the implementer's claims against the now-updated
        diff.

      - **`stuck`**: revert the implementer's edits, then preserve
        the audit trail:

        ```bash
        git restore .
        git clean -fd
        git stash pop
        ```

        `git restore .` undoes implementer edits to tracked
        files; `git clean -fd` removes any new files the
        implementer added; `git stash pop` restores the
        pre-implementer snapshot. Now append both held blocks to
        VET.md under the current invocation section — the write
        happens **after** the revert, so it survives. Return a
        `fail` verdict.

      - **Sub-agent error or missing/malformed `<holistic-fix>`**:
        treat as `stuck`. Revert as above. Append the held
        `<drift-review>` block and a synthesized
        `<holistic-fix verdict="stuck">` block describing the
        sub-agent failure. Return `fail`.

### Phase 2 — simplifier polish pass

Drift is now `pass`. Run one polish pass for code quality. This
phase does not affect the verdict (a revert still yields
`verdict="pass"`); it only sets the `simplifier="..."` field on
the return block.

1. **Spawn the simplifier scan sub-agent.** Prompt:

   > Identify simplification candidates in the diff for
   > `SPEC-NNNN`. Run `git diff <base-ref>` to see all changes
   > (working tree included). **Report only — do NOT modify
   > files.** Skip anything that would change behavior, weaken
   > invariants, or trip project conventions in `AGENTS.md` and
   > project-local rule files.
   >
   > Return your verdict as your final message:
   >
   > ```
   > <simplifier-scan verdict="clean|candidates">
   > <one-line summary>
   > [optional bullets, each with file:line + proposed change]
   > </simplifier-scan>
   > ```

   Invoke Codex's native sub-agent-spawn primitive against the
   registered `vet-simplifier` sub-agent at
   `.codex/agents/vet-simplifier.toml`.

   The scan makes no modifications, so no defer-write is needed
   — **append the returned `<simplifier-scan>` block to VET.md
   immediately** (under the current invocation section). The
   block is part of the audit trail whether or not an apply step
   follows.

2. If `verdict="clean"` → record `simplifier="clean"` for the
   return block and go to Phase 3.

3. If `verdict="candidates"`:

   a. **Snapshot the working tree** before the apply, so the
      running session owns the rollback. The simplifier
      sub-agent cannot reliably roll back itself — `git
      checkout` doesn't undo new files and `git clean -fd` is
      dangerous if scoped wrong. Owning the rollback here bounds
      the blast radius.

      ```bash
      git stash push --include-untracked -m "speccy-holistic-pre-simplifier-<spec>-<invocation>"
      git stash apply
      ```

      The first command saves a snapshot of all uncommitted state
      (tracked + untracked) and clears the working tree to HEAD;
      the second restores the working tree so the simplifier sees
      the drift-fix changes. The stash remains as the rollback
      target.

   b. **Spawn the simplifier apply sub-agent.** Prompt:

      > Apply the simplification candidates listed below.
      > Preserve all functionality. After applying, run the
      > standard hygiene suite per `AGENTS.md` (the project's
      > four standard hygiene gates).
      >
      > **If any hygiene step fails, do NOT attempt to revert
      > yourself.** Return `verdict="blocking"` with a one-line
      > description of what failed; the caller owns the rollback.
      >
      > Candidates to apply:
      >
      > [paste the `<simplifier-scan>` block from step 1 verbatim]
      >
      > Return your final message:
      >
      > ```
      > <simplifier-apply verdict="applied|blocking">
      > <one-line summary>
      > </simplifier-apply>
      > ```

      Invoke Codex's native sub-agent-spawn primitive against the
      registered `vet-simplifier` sub-agent at
      `.codex/agents/vet-simplifier.toml`.

      Hold the returned `<simplifier-apply>` block in memory; do
      not write to VET.md yet (same defer-write pattern as Phase 1
      — write after the revert decision so the audit trail
      survives any rollback).

   c. Resolve the snapshot based on the verdict, then transcribe:

      - **`applied`** (hygiene green): `git stash drop` — discard
        the snapshot, keep the simplifications. Append the held
        `<simplifier-apply>` block to VET.md under the current
        invocation section. Record `simplifier="applied"` for the
        return block.

      - **`blocking`** (hygiene failed), sub-agent error, or
        missing/malformed verdict: roll back.

        ```bash
        git restore .
        git clean -fd
        git stash pop
        ```

        `git restore .` wipes simplifier changes from tracked
        files. `git clean -fd` removes untracked files (including
        any new files the simplifier created). `git stash pop`
        restores the pre-simplifier snapshot. Then append the
        held `<simplifier-apply>` block to VET.md under the
        current invocation section (synthesize a placeholder if
        the sub-agent returned nothing parseable). Record
        `simplifier="reverted"`.

### Phase 3 — write `<gate>` block

**Every** exit path — Phase 0 integrity failures, Phase 1
round-budget exhaustion, Phase 1 `stuck` reverts, Phase 2
completion (pass or revert), and the success path — appends
exactly one `<gate>` block to `<spec-dir>/journal/VET.md` under
the current `## Invocation N` section, **before** surfacing the
verdict to the caller.

If Phase 0 failed before opening the invocation section (for
example, the spec is unknown so `<spec-dir>` was never resolved
or the journal file does not exist yet), bootstrap the file and
section per Phase 0 step 3, then append the `<gate>` block. The
on-disk gate record exists regardless of where the early exit
fired.

The `<gate>` block is appended **after** any `<drift-review>`,
`<holistic-fix>`, `<simplifier-scan>`, and `<simplifier-apply>`
blocks already written for the current invocation. It is the
**last** element in the section.

Block shape:

```
<gate verdict="passed|failed" tasks_hash="<lowercase-hex-sha256>" date="<ISO8601>">
<one-line human-readable summary of the invocation outcome>
</gate>
```

Attribute rules:

- `verdict` — `passed` when the surfaced verdict will be
  `verdict="pass"`; `failed` when it will be `verdict="fail"`
  (including every Phase 0 early-exit path).
- `tasks_hash` — lowercase hex SHA-256 of the byte contents of
  `<spec-dir>/TASKS.md` read **immediately before** appending this
  block. Compute via:

  ```bash
  sha256sum <spec-dir>/TASKS.md | awk '{print $1}'
  ```

  PowerShell equivalent on Windows:

  ```powershell
  (Get-FileHash -Algorithm SHA256 <spec-dir>/TASKS.md).Hash.ToLower()
  ```

- `date` — ISO8601 datetime with seconds and timezone designator,
  e.g. `2026-05-22T14:30:00Z`.

The block body is a single line summarising what happened
(examples: `"Drift cleared on round 2; simplifier applied;
clean."`, `"Phase 0 integrity check failed: task T-003 not
completed."`, `"Drift round budget exhausted at round 3 without a
pass."`).

`speccy next` reads the most recent `<gate>` block's `verdict`
and `tasks_hash` to decide whether the SPEC is freshly vetted; a
`passed` gate whose `tasks_hash` no longer matches the on-disk
TASKS.md forces a re-vet. That is the contract this block exists
to satisfy.


After the vet workflow appends its `<gate>` block and surfaces a
verdict to this orchestrator session, react as follows:

- `verdict="pass"` → write a one-line summary plus the round and
  simplifier counters, then **ask the user** whether to invoke
  `speccy-ship`. Only after explicit confirmation,
  spawn a `speccy-ship` sub-agent. Ship opens a PR; never
  auto-ship.
- `verdict="fail"` → surface the drift summary and one-line
  suggested next step. Stop the outer loop. The user decides how
  to address it (`speccy-amend`, manual edits,
  etc.).

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
