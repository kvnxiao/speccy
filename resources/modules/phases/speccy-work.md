
# {{ cmd_prefix }}speccy-work

Implements one task per invocation and exits. With an optional
`[SPEC-NNNN/T-NNN]` selector argument, the session implements that
specific task. Without an argument, the session resolves the next
implementable task via `speccy next --json` (workspace form — used here
because no SPEC is known on the no-selector path) and implements that one.
Task state lives in the `state` attribute on each `<task>` XML element
in TASKS.md.

This is a single-task primitive. It does not iterate over the task
list; composition across tasks belongs to a caller (a human at the
terminal, the `/loop` skill, or a future orchestrator).

## When to use

- With a selector (`{{ cmd_prefix }}speccy-work SPEC-NNNN/T-003`):
  when the next task to implement is already known — e.g., a retry
  after `{{ cmd_prefix }}speccy-review` flipped a task back to
  `state="pending"`.
- Without an argument: when picking up wherever `TASKS.md` left
  off. The session implements one task and exits.

`{{ cmd_prefix }}speccy-decompose` must have written `TASKS.md` and the
spec hash must have been committed before this skill runs.

The agent automatically detects retry shape from the per-task
journal at `<spec-dir>/journal/T-NNN.md` and switches modes
accordingly — the caller does not pass a flag. A first-attempt
task (no journal, or no `<blockers>` matching the highest
`<implementer>` round) runs today's recipe unchanged. A retry-shape
task (the latest `<blockers>` round equals the highest
`<implementer>` round) amends the existing WIP in the working tree
in place and appends `<implementer round="N+1">`.

## What to consider

- Re-read the task's `covers="REQ-NNN"` REQ's `<done-when>`
  (acceptance criteria) and `<behavior>` (test selection) elements
  before writing tests — not the surrounding prose alone.
- The suggested-files hint in the task body is advisory and may be
  stale; verify each path before editing.
- Are you about to add a feature flag, abstraction layer, or
  configurability the SPEC did not ask for? Stop and reconsider —
  scope creep is a blocker for the reviewer round.

## Steps

**Entry precondition (SPEC-0045 REQ-002, extended by SPEC-0047 REQ-002 / REQ-003):** before any Task dispatch, (i) resolve the target task per step 1, (ii) read `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the retry-shape rule inlined at step 2 from `{{ speccy_references_path }}/retry-shape.md`, (iii) run `git status --porcelain`. **First-attempt shape** with non-empty stdout exits the skill (surface dirty paths on stderr); empty stdout proceeds with the first-attempt branch (today's SPEC-0045/REQ-002 behaviour). **Retry shape** proceeds with the retry branch regardless of stdout — the dirty paths are the prior pass's WIP that the retry implementer amends in place; no dirty-paths surface is written. If `speccy next --json` then returns `next_action.kind == "reconcile"`, dispatch the reconcile pass per the shared reconcile-policy partial inlined into the calling SKILL.md body rather than the normal implementer flow.

1. Resolve the target task.

   - If a `SPEC-NNNN/T-NNN` selector was passed, that is the target.
   - Otherwise, query the CLI in workspace form (no SPEC selector
     is known on this no-selector invocation path — we must walk
     the active tree to find an implementable task):

     ```bash
     # workspace form: no SPEC-NNNN known yet; scan the active tree.
     speccy next --json
     ```

     Workspace-form exit-code-stop contract: exit code 2 with a
     top-level `reason="no_active_specs"` field in the JSON envelope
     means the workspace has no active specs at all (fresh repo, or
     every spec has shipped or been archived). Exit gracefully and
     surface the reason; do not treat the non-zero exit as a CLI
     error.

     On exit code 0, if the resulting `specs` array has no entry
     with `next_action.kind == "work"`, exit and report that no
     implementable tasks remain. Otherwise, construct the
     disambiguated `<spec>/<task>` form from the JSON's `spec_id`
     and `next_action.task_id` fields (the bare task ID is
     ambiguous across specs — every spec has its own `T-001`).

     Exit-code-stop contract: once SPEC-NNNN is resolved, any
     subsequent per-spec query (`speccy next SPEC-NNNN --json`) that
     exits non-zero means the SPEC has reached a terminal state —
     halt and surface the stderr line. Only parse JSON when exit
     code is 0.

2. Read `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the
   REQ-001 retry-shape rule inlined immediately below. The rule
   reads only the journal file — no git, no `speccy next`, no
   other CLI subcommand. Compute the result (first-attempt shape
   or retry shape); the rest of the recipe branches on this
   result.

<!-- Shared rule: retry-shape. Source: {{ speccy_references_path }}/retry-shape.md -->
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

3. Branch on the rule result.

   **First-attempt branch.** Proceed with the recipe below
   (steps 4–8) unchanged: flip state to `in-progress`, read
   scenarios, implement from scratch, run the hygiene gate, flip
   to `in-review`, append `<implementer round="1">`.

   **Retry branch.** Enter retry mode:

   - Read the most recent `<implementer>` block in the journal to
     understand the prior pass's stated `Completed` work.
   - Read the latest `<blockers>` block (the one whose `round`
     matches the highest `<implementer>` round) for the specific
     feedback to address.
   - Amend the existing WIP in the working tree to address the
     blockers. Do not run `git restore`, `git clean`, or
     `git checkout` against the dirty paths; do not rewrite the
     touched files from scratch; do not reset state. The dirty
     tree is the prior pass's WIP — iterate on it in place.
   - Flip state to `in-progress` and continue through the same
     hygiene gate and `in-review` flip the first-attempt branch
     uses (the SPEC-0045/REQ-001 hygiene gate runs unchanged).
   - Append the next `<implementer round="N+1">` block where `N`
     is the highest prior `<implementer>` round in the journal,
     monotonically incremented by exactly 1. The retry-mode
     `Completed` field describes the amend (what changed this
     round in response to the blockers), not a restatement of
     the cumulative task work.

4. Flip the target task's `state` from `pending` to `in-progress`
   by editing TASKS.md.

5. Read the task scenarios to understand what must be implemented:

   ```bash
   speccy check SPEC-NNNN/T-003
   ```

6. Implement the task. Write tests first, then code. Run the
   project's own test command (`cargo test`, `pnpm test`, etc.)
   locally. Use `speccy check SPEC-NNNN/T-NNN` to re-read the
   scenarios being satisfied (it renders them, it does not run
   them).

7. Exit transition. **Hygiene gate (REQ-001):** before flipping `state` from `in-progress` to `in-review`, run the four standard hygiene gates in sequence — `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny check`. Any non-zero exit refuses the flip and keeps the task at `in-progress`; on all zeros, proceed with the flip and record one line per gate naming its exit code in the appended `<implementer>` block's `Hygiene checks` field. When the implementation is done, flip the task's
   `state="..."` attribute from `in-progress` to `in-review` in
   TASKS.md, then append one `<implementer>` block to the per-task
   journal file at `.speccy/specs/NNNN-slug/journal/T-NNN.md` (a
   sibling of `SPEC.md` and `TASKS.md`). Do NOT inline an
   `<implementer-note>` inside the `<task>` body in TASKS.md — the
   parser rejects that element. The journal file is the canonical
   home for implementer handoff prose.

   File creation. If `journal/T-NNN.md` does not yet exist (round 1,
   first implementer attempt on the task), create it with YAML
   frontmatter declaring exactly three fields, then the
   `<implementer>` block beneath.

   Canonical journal `<implementer>` shape: `references/journal-implementer.md`.

   Canonical evidence file shape: `{{ speccy_references_path }}/evidence.md`.

   `generated_at` is the ISO8601 timestamp at file creation; do not
   rewrite it on later appends. On subsequent rounds, append the new
   `<implementer>` block after the existing journal contents — do
   not modify earlier blocks.

   Required attributes on `<implementer>`. All three are required;
   there are no optional attributes:

   - `date` — full ISO8601 date-time with seconds and timezone
     designator (e.g. `2026-05-21T18:00:00Z` or
     `2026-05-21T18:00:00+00:00`).
   - `model` — the model identity that ran the implementer turn. A
     slash-suffix encodes effort / reasoning-intensity when the host
     harness exposes that knob (e.g.
     `model="claude-opus-4.7[1m]/low"`,
     `model="claude-opus-4.7[1m]/medium"`). Hosts without an effort
     knob omit the suffix entirely (e.g. `model="claude-opus-4.7"`).
     The slash-suffix is a documented convention; the parser
     validates `model` is non-empty but does not enforce suffix
     membership.
   - `round` — a monotonic positive integer starting at 1.
     Increment by exactly 1 on each post-blocker retry attempt. The
     first implementer turn on a task is `round="1"`; if a review
     round blocks and the task flips back to `pending`, the next
     implementer attempt writes `round="2"`, and so on. Do not skip
     values; do not reset.

   Body content. Use the six-field handoff template the implementer
   prompt supplies (`Completed`, `Undone`, `Hygiene checks`,
   `Evidence`, `Discovered issues`, `Procedural compliance`). The
   `Evidence` field must include a CHK-by-CHK roll call labelling
   each CHK under the task's covered REQs as `demonstrated`,
   `hygiene`, or `judgment-only` -- see the canonical reference for
   the format and what each label means.

   Minimal Evidence roll-call shape -- substitute real CHK ids,
   paths, and test names; the canonical reference carries the full
   worked example:

   ```
   - Evidence: paper trail at `.speccy/specs/NNNN-slug/evidence/T-NNN.md`.
     Roll call for CHKs under REQ-NNN:
     - CHK-NNN (one-line CHK description): demonstrated →
       evidence Scenario N covers <what the red/green pair shows>.
     - CHK-NNN (one-line CHK description): hygiene →
       `<test_name>` in `<file:path>` covers it under the project
       test command.
     - CHK-NNN (one-line CHK description): judgment-only →
       no scriptable proof; reviewer-business / reviewer-style
       judges on the diff.
   ```

8. Exit. Do not continue to the next task. If the caller wants
   another task, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="in-review"`, suggest
`{{ cmd_prefix }}speccy-review SPEC-NNNN`. If all tasks are
`state="completed"`, suggest `{{ cmd_prefix }}speccy-vet SPEC-NNNN`.
