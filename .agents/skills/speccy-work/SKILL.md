---
name: speccy-work
description: 'Implement one Speccy task per invocation and exit. With an optional `SPEC-NNNN/T-NNN` selector, the session implements that task; without a selector, it resolves the next implementable task via `speccy next --json` and implements only that one. Use when the user says "implement T-003", "work the next task", "run the implementer", "pick up the next pending task in SPEC-NNNN", or wants to implement one slice against an existing task list. Requires: `TASKS.md` with ≥1 `state="pending"` task. If no `TASKS.md` → prefer speccy-decompose. If no `SPEC.md` → prefer speccy-plan. If no `.speccy/` → prefer speccy-init. Do NOT trigger for generic "fix bug" or "refactor X" asks that are not scoped to an existing Speccy task.'
---

# speccy-work

Read `.codex/agents/speccy-work.toml` and follow it, or invoke
`/agent speccy-work` for the pinned execution path.

**Entry precondition (SPEC-0045 REQ-002, extended by SPEC-0047 REQ-002):** resolve the target task, read `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the retry-shape rule inlined immediately below from `.agents/speccy-references/retry-shape.md`, then run `git status --porcelain`. **First-attempt shape** with non-empty stdout exits the skill with the dirty-paths surface on stderr (today's SPEC-0045/REQ-002 behaviour, unchanged); empty stdout proceeds. **Retry shape** proceeds regardless of stdout — the dirty paths are the prior pass's WIP that the retry implementer amends in place. If `speccy next --json` then returns `next_action.kind == "reconcile"`, dispatch per the inlined reconcile-policy partial below instead of the implementer.

<!-- Shared rule: retry-shape. Source: .agents/speccy-references/retry-shape.md -->
## Rule statement

> `T-NNN` is in **retry shape** at `<spec-dir>` iff
> `<spec-dir>/journal/T-NNN.md` exists, contains at least one
> `<implementer>` element block, and contains at least one
> `<blockers>` element block whose `round` attribute equals the
> highest `round` attribute on any `<implementer>` block in the
> file. Otherwise `T-NNN` is in **first-attempt shape**.

## Read-only scope

The rule reads only `<spec-dir>/journal/T-NNN.md`. It does not read
TASKS.md, does not invoke `git`, does not call `speccy next`, and
does not invoke any other CLI subcommand. Detection is mechanical:
parse the journal's XML elements (using the same closed-set journal
grammar `<implementer>` / `<review>` / `<blockers>` enforced by the
`JNL-*` lint family), read the `round` attributes, compare.

## Worked example 1 — retry shape

```
<implementer round="1" date="2026-05-26T18:00:00Z" model="claude-opus-4.7[1m]/low">
... first-pass implementer body ...
</implementer>

<review persona="style" verdict="blocking" round="1" ...>
... style persona feedback ...
</review>

<blockers round="1" ...>
Style: drop the `println!` short-circuit in `reporter.rs`.
</blockers>
```

Applying the rule: the journal contains one `<implementer>` block
(highest `round="1"`) and a `<blockers round="1">` block whose
`round` attribute equals that highest implementer round. The
result is **retry shape**. The dirty tree from the round-1
implementer is the WIP the round-2 implementer amends in place.

## Worked example 2 — first-attempt shape

```
<implementer round="1" date="2026-05-26T18:00:00Z" model="claude-opus-4.7[1m]/low">
... first-pass implementer body ...
</implementer>
```

Applying the rule: the journal contains one `<implementer>` block
and no `<blockers>` blocks. The result is **first-attempt shape**.
The strict clean-tree gate applies — a non-empty
`git status --porcelain` halts the calling skill with the
dirty-paths surface.

A journal file that does not exist on disk also yields
**first-attempt shape** (the rule's first conjunct fails). The
strict clean-tree gate applies the same way.

## Edge case — implementer awaiting review

```
<implementer round="1" ...>...</implementer>
<review persona="style" verdict="blocking" round="1" ...>...</review>
<blockers round="1" ...>...</blockers>

<implementer round="2" ...>...</implementer>
<review persona="business" verdict="blocking" round="2" ...>...</review>
<blockers round="2" ...>...</blockers>

<implementer round="3" ...>... round-3 pass, awaiting review ...</implementer>
```

Applying the rule: the highest implementer-block round in this
journal is `3`, but no `<blockers round="3">` block
exists (the round-3 reviewer fan-out has not yet fired). The
result is **first-attempt shape** — the task is awaiting review,
not awaiting a retry. The strict clean-tree gate applies; if the
round-3 implementer's WIP is still in the tree, the calling skill
halts. (In practice the round-3 implementer's atomic-commit step
would have already landed its work before the journal entered this
state; this edge case is documented for completeness.)
<!-- End shared rule: retry-shape. -->

<!-- Shared partial: reconcile-policy. Source: .agents/speccy-references/reconcile-policy.md -->
# Reconcile policy: shared partial

This file is the single source of truth for Speccy's reconcile
dispatch policy. It is inlined verbatim into three skill body files
via the existing shared-partial convention:

- `.claude/skills/speccy-orchestrate/SKILL.md`
- `.claude/skills/speccy-work/SKILL.md`
- `.claude/skills/speccy-review/SKILL.md`

Each inlined site is bounded by marker comments naming this partial:

```
<!-- Shared partial: reconcile-policy. Source: .claude/speccy-references/reconcile-policy.md -->
<partial content>
<!-- End shared partial: reconcile-policy. -->
```

When this file changes, all three inlined copies must be re-synced.

---

## Dispatch trigger

The reconcile pass is triggered when `speccy next --json` (in either
per-spec or workspace form) returns `next_action.kind == "reconcile"`.

The CLI sets this value whenever the envelope's `consistency.status`
field is anything other than `"ok"` — i.e. `"drift"` or `"blocked"`.
When `consistency.status == "ok"`, `next_action.kind` reflects the
normal dispatch (`work`, `review`, `ship`, `decompose`, ...) and the
calling skill proceeds normally without invoking this policy.

The calling skill (one of `/speccy-orchestrate`, `/speccy-work`,
`/speccy-review`) iterates the `consistency.drifts[]` array and
applies one action per entry per the table below.

## Policy table

| `kind` | `severity` | Action |
|---|---|---|
| `commit_without_state` | `auto_fixable` | Edit TASKS.md: flip the task's `state` attribute to `completed` (deterministic write). |
| `state_completed_no_commit` (dirty tree, `details.working_tree_dirty == true`) | `blocking` | Run `git add -A` followed by `git commit` using the REQ-004 message format (title `[SPEC-NNNN/T-NNN]: <task title>`; body extracted from the latest `<implementer>` block's `Completed` field in `journal/T-NNN.md`; `Co-Authored-By` trailer per host). |
| `state_completed_no_commit` (clean tree, `details.working_tree_dirty == false`) | `blocking` | Edit TASKS.md: roll the task's `state` back to `in-review`. Journal file is preserved intact as evidence for the next reviewer round. |
| `state_in_progress_orphaned` | `blocking` | Run `git restore .` and `git clean -fd` to discard the partial implementer work, then edit TASKS.md to flip the task's `state` to `pending`. The orchestrator's per-task retry budget will redo the work. |
| `state_in_progress_clean` (`details.working_tree_dirty == false`) | `blocking` | Edit TASKS.md: roll the task's `state` back to `pending`. No git mutation — the tree is already clean, so there is no partial work to discard. The orchestrator's per-task retry budget will redo the work. |
| `journal_xml_malformed` | `blocking` | Truncate the journal file at `details.journal_path` to `details.last_well_formed_byte_offset` bytes. Reset the corresponding TASKS.md `state` to whatever the truncated journal implies (if the last well-formed element is `<implementer>`, state goes to `in-review`; if a closing `<review>` block survived and all four personas passed, the per-task journal already reflects a passing round and state may flip to `completed` via the standard commit step). |

## Post-dispatch re-query discipline

After applying actions for every entry in `consistency.drifts[]`, the
skill re-queries `speccy next --json` (in the same per-spec form it
was invoked with).

- If `consistency.status == "ok"` on the re-query, the skill resumes
  its normal dispatch on the returned `next_action.kind`.
- If `consistency.status` is still `"drift"` or `"blocked"`, the
  skill applies actions again for the new drift list. The mechanism
  has no hard round budget; idempotency plus re-detectability on
  subsequent sessions bounds the worst case.

## Three properties

The reconcile pass holds three properties by construction:

1. **Autonomous.** The pass applies the action for each drift kind
   without prompting the user, without surfacing a fork ("re-commit
   or roll back?"), and without halting the orchestration loop. No
   `AskUserQuestion` invocation, no "press enter to continue"
   surface, appears anywhere in the dispatch path. The policy table
   above is exhaustive over the documented enum; an unknown `kind`
   is the only path that escalates (treated as `blocking` and
   surfaced to the caller).

2. **Rollback-biased.** When recovery is ambiguous — most notably
   `state_completed_no_commit` with a clean working tree, meaning
   the lost commit's content is truly unrecoverable — the policy
   prefers rolling backward (TASKS.md state reset to `in-review`,
   journal preserved as evidence) over any forward-recovery attempt
   that might guess at lost content. The orchestrator's per-task
   retry budget absorbs the redo cost.

3. **Idempotent.** Each policy action is a no-op when applied to
   already-converged state. Re-running `git add -A && git commit`
   on a clean tree produces no commit; re-running a TASKS.md state
   flip when the state is already at the target value is a no-op;
   truncating a journal at its current length is a no-op.
   Successive session crashes during reconciliation converge to the
   same eventual state.

## Extending the enum

Adding a new drift kind to the consistency enum requires changes in
exactly two places:

1. **CLI detection.** Add the new variant to the `DriftKind` enum in
   the Rust source (`speccy-core/src/consistency.rs` or its
   equivalent under the current module layout) and implement the
   deterministic detection logic that emits drift entries of the
   new kind in `speccy next --json`'s `consistency.drifts[]` array.
   Detection must be read-only: no mutating git commands, no
   side-effecting writes to TASKS.md or the journal.

2. **Policy table.** Add a row to the policy table in this partial
   naming the new kind, its `severity`, and the deterministic
   action the calling skill takes when it encounters the kind in
   `consistency.drifts[]`. Then re-sync all three inlined copies in
   the skill body files listed at the top of this partial.

No other site needs to change. The CLI knows what it *detected*;
this partial knows what to *do*. Future hosts (Codex, others) that
consume the consistency block reuse this partial unchanged.
<!-- End shared partial: reconcile-policy. -->

**Hygiene gate (REQ-001):** after the implementer turn, before flipping `state` from `in-progress` to `in-review`, run `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny check`. Any non-zero exit refuses the flip and keeps the task at `in-progress`; on all zeros, the appended `<implementer>` block's `Hygiene checks` field carries one line per gate naming its exit code.
