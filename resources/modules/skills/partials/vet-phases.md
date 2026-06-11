
Round budget: **3 rounds per invocation** for drift fixing. Each
round is expensive (full SPEC re-read + diff re-analysis +
implementer pass), so the budget of 3 is intentionally tighter
than the per-task implementer retry budget.

### Phase 0 — bootstrap

Resolve the two values that sub-agent prompts need. The CLI owns
VET.md's frontmatter and invocation sectioning, so this phase no
longer hand-bootstraps the file or hand-writes an `## Invocation N`
heading — the first `speccy journal append` for this invocation does
both for you (it creates the file with frontmatter on first ever
append, and opens `## Invocation N+1` automatically when the file is
absent or its last section is gate-terminated).

1. **Spec directory.** Run:

   ```bash
   speccy next SPEC-NNNN --json
   ```

   The `spec_md_path` field (e.g.,
   `.speccy/specs/NNNN-slug/SPEC.md`) gives the absolute path to
   `SPEC.md`; strip the trailing `/SPEC.md` to get `<spec-dir>`
   (e.g., `.speccy/specs/NNNN-slug/`). If the command exits
   non-zero, the SPEC has reached a terminal state — surface the
   stderr line and return `fail`. Only parse the JSON envelope
   when exit code is 0. If the spec is unknown, return `fail`
   immediately.

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

The invocation number `N` for this run is whatever the CLI assigns
on the first append — you do not pick it or write the heading. After
the first block of this invocation lands (the round-1
`<drift-review>` the vet-reviewer appends, or the Phase 3 `<gate>` on
a Phase 0 early-exit), read it back with `speccy journal show
SPEC-NNNN --json` to learn the invocation number for the return
contract's `invocation` field.

### Phase 1 — drift review and fix

Repeat for up to 3 rounds per invocation. The running session owns
the round counter and the working-tree snapshots; sub-agents own the
substantive review and fix work **and append their own blocks** to
VET.md via `speccy journal append` (the vet-reviewer appends
`<drift-review>`, the vet-implementer appends `<holistic-fix>`). The
CLI's per-file lock serializes those appends and stamps `date`,
`round`, and the invocation section — the running session never
transcribes a sub-agent's block and never edits VET.md with
file-editing tools.

**Protect the journal from rollback.** The sub-agents append to
VET.md *during their own runs*, so a block is already on disk before
the running session reaches its keep-vs-revert decision. Because
VET.md lives in the working tree, a naive `git restore .` /
`git stash pop` would revert those appends and erase the audit trail.
The journal is the durable record of *what the loop did* and must
survive any rollback. Two git facts drive the mechanism below:

- `git restore -- ':!…/journal/'` and `git clean -fd -e '…/journal/'`
  **do** honour the journal exclusion — a path-excluded restore and an
  `-e`-excluded clean both leave VET.md on disk. Use them directly.
- `git stash push --include-untracked` does **not** honour a pathspec
  exclusion for the *untracked* journal file: it sweeps VET.md into the
  stash regardless of any `':!…/journal/'` argument. A later
  `git stash pop` then tries to restore that stale copy over the live
  journal — at best it aborts with `already exists, no checkout`
  (leaving stash litter and a non-zero exit), at worst it clobbers the
  blocks appended since the snapshot. **Never `git stash pop` in this
  loop.**

So the loop snapshots with a plain `--include-untracked` stash (no
pathspec — the journal is swept in, then immediately restored by
`git stash apply`; because the journal is always dirty at vet time the
push always creates a stash, so `stash@{0}` reliably names *our*
snapshot and never a pre-existing unrelated one). On rollback it
restores prior-round **code** from the stash with a tracked-only
`git checkout 'stash@{0}' -- ':!…/journal/'`, which never touches the
stash's untracked journal copy, then drops the stash. The live on-disk
journal — with every block appended since the snapshot — is left
untouched throughout. The reusable revert sequence is:

```bash
# Revert code to the pre-sub-agent snapshot; never touch the journal.
git restore -- ':!.speccy/specs/*/journal/'
git clean -fd -e '.speccy/specs/*/journal/'
git checkout 'stash@{0}' -- ':!.speccy/specs/*/journal/'
git stash drop
```

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
   > contract in your agent file. Append your `<drift-review>` block
   > to VET.md via `speccy journal append` and return a single thin
   > `<verdict>` element as your final message.

   Substitute `SPEC-NNNN`, `N`, `R`, `<spec-dir>`, and `<base-ref>`
   with the resolved values. The sub-agent appends its own
   `<drift-review>` block; do not transcribe it. Read the thin
   `<verdict>` it returns to decide the next step.

   {% if host == "claude-code" %}Invoke the `Task` tool with `subagent_type: "vet-reviewer"`.
   The sub-agent definition at `.claude/agents/vet-reviewer.md`
   carries the host-native dispatch metadata (model pin, effort
   level).{% else %}Invoke Codex's native sub-agent-spawn primitive against the
   registered `vet-reviewer` sub-agent at
   `.codex/agents/vet-reviewer.toml`.{% endif %}

2. **If `verdict="pass"`** → the vet-reviewer already appended its
   `<drift-review>` block. Exit the loop and go to Phase 2.

3. **If `verdict="blocking"` and no rounds remain** → the
   `<drift-review>` block is already in VET.md (the reviewer
   appended it), so the trail is complete. Return a `fail` verdict.

4. **Otherwise** (`verdict="blocking"` with budget remaining):

   a. **Snapshot the working tree** before the implementer call,
      so the running session can revert on `stuck`. The snapshot
      captures code state; the reviewer's just-appended
      `<drift-review>` block in VET.md survives because the revert
      scopes out the journal directory (see "Protect the journal
      from rollback" above):

      ```bash
      git stash push --include-untracked -m "speccy-holistic-pre-implementer-<spec>-inv<N>-r<R>"
      git stash apply
      ```

      The `push` snapshots all uncommitted state and clears it to
      HEAD; the `apply` restores the working tree so the implementer
      has the current implementation to work on. The stash stays
      available as the rollback target. The journal is swept into the
      stash by `--include-untracked` and immediately restored by
      `apply` (see "Protect the journal from rollback" above); the
      rollback path below restores code from the stash without ever
      touching that copy.

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
      > The running session will revert your code changes if you
      > return `verdict="stuck"`. Do not manage rollback yourself.
      >
      > Follow the scope, hygiene-gate, and verdict-return
      > contract in your agent file. Append your `<holistic-fix>`
      > block to VET.md via `speccy journal append` and return a
      > single thin `<verdict>` element as your final message.
      >
      > The drift findings to address are in the most recent
      > `<drift-review>` block in VET.md; read them with
      > `speccy journal show SPEC-NNNN --block drift-review
      > --round latest`.

      The sub-agent appends its own `<holistic-fix>` block; do not
      transcribe it. Read the thin `<verdict>` it returns.

      {% if host == "claude-code" %}Invoke the `Task` tool with `subagent_type: "vet-implementer"`.
      The sub-agent definition at
      `.claude/agents/vet-implementer.md` carries the host-native
      dispatch metadata.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
      registered `vet-implementer` sub-agent at
      `.codex/agents/vet-implementer.toml`.{% endif %}

   c. **Resolve the snapshot based on the implementer's verdict.**
      The `<drift-review>` and `<holistic-fix>` blocks are already
      in VET.md (each sub-agent appended its own), so this step only
      keeps or reverts the **code** changes — it never writes
      blocks:

      - **`addressed` or `blocking`**: keep the implementer's
        edits.

        ```bash
        git stash drop
        ```

        Decrement the round counter and go back to step 1. The next
        reviewer reads the journal the sub-agents appended and
        verifies the implementer's claims against the now-updated
        diff.

      - **`stuck`**: revert the implementer's **code** edits while
        preserving the journal audit trail:

        ```bash
        git restore -- ':!.speccy/specs/*/journal/'
        git clean -fd -e '.speccy/specs/*/journal/'
        git checkout 'stash@{0}' -- ':!.speccy/specs/*/journal/'
        git stash drop
        ```

        The journal-scoped restore/clean undo implementer edits to
        tracked files and remove new files the implementer added; the
        tracked-only `git checkout 'stash@{0}'` then restores the
        pre-implementer code from the snapshot and `git stash drop`
        discards it. All four steps exclude the journal directory, so
        the `<drift-review>` and `<holistic-fix>` blocks in VET.md stay
        intact. Do **not** `git stash pop` — it would collide on the
        swept-in untracked journal (see "Protect the journal from
        rollback" above). Return a `fail` verdict.

      - **Sub-agent error or missing/malformed `<verdict>`**: treat
        as `stuck`. Revert the code as above. The vet-implementer
        appends its `<holistic-fix>` block (including the `stuck`
        case) as part of its own run; if it errored before
        appending, append a synthesized `<holistic-fix>` recording
        the sub-agent failure via `speccy journal append SPEC-NNNN
        --block holistic-fix --verdict stuck --model <orchestrator-model>`
        so the trail is complete. Return `fail`.

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
   > Append your `<simplifier-scan>` block to VET.md via `speccy
   > journal append` and return a single thin `<verdict>` element as
   > your final message.

   {% if host == "claude-code" %}Invoke the `Task` tool with
   `subagent_type: "vet-simplifier"`.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
   registered `vet-simplifier` sub-agent at
   `.codex/agents/vet-simplifier.toml`.{% endif %}

   The scan sub-agent appends its own `<simplifier-scan>` block; do
   not transcribe it. The scan makes no code modifications, so it is
   part of the audit trail whether or not an apply step follows. Read
   the thin `<verdict>` it returns.

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

      The first command snapshots uncommitted state and clears it to
      HEAD; the second restores the working tree so the simplifier
      sees the drift-fix changes. The stash remains as the rollback
      target. As in Phase 1, `--include-untracked` sweeps the journal
      into the stash and `apply` restores it; the rollback path below
      restores code from the stash without touching that copy (see
      "Protect the journal from rollback" above).

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
      > The candidates to apply are in the most recent
      > `<simplifier-scan>` block in VET.md; read them with
      > `speccy journal show SPEC-NNNN --block simplifier-scan`.
      >
      > Append your `<simplifier-apply>` block to VET.md via `speccy
      > journal append` and return a single thin `<verdict>` element
      > as your final message.

      {% if host == "claude-code" %}Invoke the `Task` tool with
      `subagent_type: "vet-simplifier"`.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
      registered `vet-simplifier` sub-agent at
      `.codex/agents/vet-simplifier.toml`.{% endif %}

      The apply sub-agent appends its own `<simplifier-apply>` block;
      do not transcribe it. Read the thin `<verdict>` it returns.

   c. Resolve the snapshot based on the verdict. The
      `<simplifier-apply>` block is already in VET.md (the sub-agent
      appended it), so this step only keeps or reverts **code**:

      - **`applied`** (hygiene green): `git stash drop` — discard
        the snapshot, keep the simplifications. Record
        `simplifier="applied"` for the return block.

      - **`blocking`** (hygiene failed), sub-agent error, or
        missing/malformed verdict: roll back the **code** while
        preserving the journal audit trail.

        ```bash
        git restore -- ':!.speccy/specs/*/journal/'
        git clean -fd -e '.speccy/specs/*/journal/'
        git checkout 'stash@{0}' -- ':!.speccy/specs/*/journal/'
        git stash drop
        ```

        The journal-scoped restore/clean wipe simplifier changes from
        tracked files and remove untracked files the simplifier
        created; the tracked-only `git checkout 'stash@{0}'` then
        restores the pre-simplifier code and `git stash drop` discards
        the snapshot. All four steps exclude the journal directory, so
        the `<simplifier-apply>` block in VET.md stays intact. Do
        **not** `git stash pop` — see "Protect the journal from
        rollback" above. If the sub-agent errored before appending,
        append a synthesized `<simplifier-apply>` recording the failure
        via `speccy journal append SPEC-NNNN --block simplifier-apply
        --verdict blocking` so the trail is complete. Record
        `simplifier="reverted"`.

### Phase 3 — append the `<gate>` block via the CLI

**Every** exit path — Phase 0 integrity failures, Phase 1
round-budget exhaustion, Phase 1 `stuck` reverts, Phase 2
completion (pass or revert), and the success path — appends
exactly one `<gate>` block to `<spec-dir>/journal/VET.md` via
`speccy journal append --block gate`, **before** surfacing the
verdict to the caller. This is the running session's own write (the
gate is not authored by a sub-agent), and it goes through the CLI
verb — never by editing VET.md with file-editing tools.

```bash
speccy journal append SPEC-NNNN --block gate --verdict <passed|failed> <<'EOF'
<one-line human-readable summary of the invocation outcome>
EOF
```

The CLI owns everything environment-derivable and the gate's
placement:

- It computes `tasks_hash` as the lowercase hex SHA-256 of the
  sibling TASKS.md read at append time — **do not compute or supply
  a hash**, and there is no `sha256sum` / `Get-FileHash` step.
- It stamps `date` (UTC now).
- It manages invocation sectioning: the gate lands as the **last**
  element of the current open invocation section, after any
  `<drift-review>`, `<holistic-fix>`, `<simplifier-scan>`, and
  `<simplifier-apply>` blocks the sub-agents appended. If no section
  is open yet (a Phase 0 early exit fired before any block landed,
  so VET.md is absent or its last section is gate-terminated), the
  same append creates the file with frontmatter and opens a fresh
  `## Invocation N` section before writing the gate. **Do not
  hand-bootstrap frontmatter or write an `## Invocation N`
  heading** — the append does both. The on-disk gate record exists
  regardless of where the early exit fired.

You supply only `--verdict` and the one-line body:

- `--verdict` — `passed` when the surfaced verdict will be
  `verdict="pass"`; `failed` when it will be `verdict="fail"`
  (including every Phase 0 early-exit path).
- body — a single line summarising what happened (examples: `"Drift
  cleared on round 2; simplifier applied; clean."`, `"Phase 0
  integrity check failed: task T-003 not completed."`, `"Drift round
  budget exhausted at round 3 without a pass."`).

Validation runs before any write; a malformed body or an attempt to
add a second gate to a gate-terminated section leaves VET.md
byte-identical.

`speccy next` reads the most recent `<gate>` block's `verdict`
and `tasks_hash` to decide whether the SPEC is freshly vetted; a
`passed` gate whose `tasks_hash` no longer matches the on-disk
TASKS.md forces a re-vet. That is the contract this block exists
to satisfy.
