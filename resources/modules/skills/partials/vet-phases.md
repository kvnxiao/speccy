
Round budget: **3 rounds per invocation** for drift fixing. Each
round is expensive (full SPEC re-read + diff re-analysis +
implementer pass), so the budget of 3 is intentionally tighter
than the per-task implementer retry budget.

### Phase 0 — bootstrap

Resolve the three values that sub-agent prompts need, then open a
new invocation section in VET.md.

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

   {% if host == "claude-code" %}Invoke the `Task` tool with `subagent_type: "vet-reviewer"`.
   The sub-agent definition at `.claude/agents/vet-reviewer.md`
   carries the host-native dispatch metadata (model pin, effort
   level).{% else %}Invoke Codex's native sub-agent-spawn primitive against the
   registered `vet-reviewer` sub-agent at
   `.codex/agents/vet-reviewer.toml`.{% endif %}

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

      {% if host == "claude-code" %}Invoke the `Task` tool with `subagent_type: "vet-implementer"`.
      The sub-agent definition at
      `.claude/agents/vet-implementer.md` carries the host-native
      dispatch metadata.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
      registered `vet-implementer` sub-agent at
      `.codex/agents/vet-implementer.toml`.{% endif %}

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

   {% if host == "claude-code" %}Invoke the `Task` tool with
   `subagent_type: "vet-simplifier"`.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
   registered `vet-simplifier` sub-agent at
   `.codex/agents/vet-simplifier.toml`.{% endif %}

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

      {% if host == "claude-code" %}Invoke the `Task` tool with
      `subagent_type: "vet-simplifier"`.{% else %}Invoke Codex's native sub-agent-spawn primitive against the
      registered `vet-simplifier` sub-agent at
      `.codex/agents/vet-simplifier.toml`.{% endif %}

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
