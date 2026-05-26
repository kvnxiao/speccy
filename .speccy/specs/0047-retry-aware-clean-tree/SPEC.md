---
id: SPEC-0047
slug: retry-aware-clean-tree
title: Retry-aware clean-tree precondition — work dispatch tolerates dirty trees on review-blocked retries
status: in-progress
created: 2026-05-26
supersedes: []
---

# SPEC-0047: Retry-aware clean-tree precondition — work dispatch tolerates dirty trees on review-blocked retries

## Summary

SPEC-0045 shipped a strict clean-tree precondition (REQ-002) at both
`/speccy-work` entry and the orchestrator's work dispatch: any
non-empty `git status --porcelain` halts before the implementer
sub-agent is spawned. The gate's stated rationale is to keep
`git add -A` sound at REQ-003's commit step ("every dirty path at
commit time is task-scoped, so staging everything sweeps in exactly
the task scope and nothing else").

The shipped gate models two scenarios cleanly: first-attempt dispatch
of a task (clean tree expected) and crash-recovery (delegated to
REQ-007's reconcile policy via the consistency enum). It does **not**
model a third scenario that arises on every blocking review: when the
reviewer fan-out returns `verdict="blocking"`, the orchestrator flips
state from `in-review` back to `pending` and appends a `<blockers>`
block to the journal, but the implementer's hygiene-clean code
changes from the failed pass stay in the working tree. The very next
loop iteration's `next_action.kind == "work"` then trips the strict
gate and halts — even though the dirty paths are scoped to the
exact task being retried.

The reconcile enum (REQ-006) does not catch this either. The
`state_in_progress_*` kinds require `state="in-progress"`, but the
orchestrator already flipped to `pending`; `state_completed_no_commit`
requires `state="completed"`; `commit_without_state` requires a
commit. The tuple `(state=pending, dirty tree, no commit, journal has
prior <implementer> + <blockers>)` is invisible to the consistency
check and is treated by the work dispatch as "unexpected dirty tree".

This SPEC closes that gap by making the work-dispatch precondition
**retry-aware**: it reads the per-task journal `journal/T-NNN.md` for
the resolved task and, when the journal exhibits a review-blocked
retry shape (at least one `<implementer>` block AND a `<blockers>`
block whose `round` attribute matches the latest `<implementer>`
round), the dirty tree is permitted. The dirty paths in that case
are the prior pass's WIP, which the retry implementer amends in
place per the latest `<blockers>` feedback rather than reimplementing
from scratch. First-attempt dispatch retains the original strict
gate.

The `git add -A` soundness invariant from REQ-003 is preserved: the
dirty paths in a retry are still task-scoped to the same task, so
the eventual atomic commit on a passing review still captures only
task-scope.

## Goals

<goals>
- `/speccy-work` and `/speccy-orchestrate`'s work dispatch
  no longer halt on a dirty tree when the resolved task's journal
  shows a review-blocked retry shape.
- A first-attempt dispatch (the resolved task's journal has no
  prior `<implementer>` blocks) continues to require a clean tree
  with the same surface behaviour as today.
- The retry implementer reads the latest `<blockers>` block and
  amends the WIP already in the working tree in place rather than
  starting from a clean reimplementation.
- The retry implementer re-runs the standard hygiene suite before
  flipping `state` from `in-progress` to `in-review`, exactly as
  the first-attempt path does.
- The atomic commit on the eventual passing review continues to
  capture all task-scoped changes (the amended WIP) in one commit
  whose title carries the standard `[SPEC-NNNN/T-NNN]:` prefix.
- The retry-shape detection rule is documented at every site that
  reads it, so the orchestrator and the standalone `/speccy-work`
  invocation paths agree on what "retry shape" means.
</goals>

## Non-goals

<non-goals>
- No change to the commit message format (the SPEC-0045 REQ-004
  title prefix, body source, and `Co-Authored-By` trailer are
  unchanged).
- No change to the atomic-commit semantics (the SPEC-0045 REQ-003
  three-step journal-append → state-flip → commit sequence runs
  unchanged on a passing review, whether or not the round was a
  retry).
- No change to the four-persona fan-out shape, the per-task retry
  budget (5 rounds), or the vet / ship path.
- The reconcile drift enum (SPEC-0045 REQ-006) does not gain a new
  kind for this case. A review-blocked retry is normal in-loop
  flow, not crash drift; detection is local to the skill body and
  does not pass through `speccy next`'s consistency block.
- No deletion or amendment of any SPEC-0045 REQ in the archive. This
  SPEC layers on top of REQ-002's shipped behaviour by extending
  the precondition rule; the archived REQ-002 prose stays as-is.
- No special handling for intermediate retry rounds where the
  blocker mix changes between rounds (a round-2 implementer reads
  the round-2 `<blockers>` and amends; the round-1 `<blockers>` is
  preserved in the journal as audit trail but not re-applied).
- No new CLI subcommand, no extension of `speccy next` output. The
  detection logic stays at the skill-body intelligent edges per
  Speccy's deterministic-core / intelligent-edges principle.
- No change to the speccy-work agent's six-field handoff template
  or the journal `<implementer>` element grammar. The retry-mode
  implementer writes the same block shape; only its prompt
  changes.
- No analogous bootstrap commit step in `/speccy-plan` or
  `/speccy-amend`. Plan and amend leave their SPEC.md edits
  uncommitted; the user (or, in the freshly-decomposed bootstrap
  path, `/speccy-decompose` itself via REQ-004) lands them.
  Extending the same pattern to plan or amend may follow in a
  later SPEC if friction surfaces.
</non-goals>

## User Stories

<user-stories>
- As a developer running `/speccy-orchestrate SPEC-NNNN`
  unattended, I want the loop to autonomously retry a task after a
  blocking review without halting on the failed pass's WIP, so a
  single design-level blocker does not force me back to a manual
  intervention every time.
- As a developer reading the journal of a task that went through
  multiple review rounds, I want each round's `<implementer>` block
  to reflect an incremental amend against the prior round rather
  than a full reimplementation, so the journal tells a coherent
  story of what changed and why.
- As a future multi-agent harness driving Speccy across many specs
  unattended, I want a blocking review to cost one retry slot from
  the per-task budget without requiring any external intervention
  to clear the working tree between rounds.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Retry-shape detection rule has a canonical definition

The retry-shape detection rule is a read-only, file-local
predicate: given a spec directory `<spec-dir>` and a task identifier
`T-NNN`, the rule returns whether the per-task journal at
`<spec-dir>/journal/T-NNN.md` exhibits a review-blocked retry
shape.

The rule:

> `T-NNN` is in **retry shape** at `<spec-dir>` iff
> `<spec-dir>/journal/T-NNN.md` exists, contains at least one
> `<implementer>` element block, and contains at least one
> `<blockers>` element block whose `round` attribute equals the
> highest `round` attribute on any `<implementer>` block in the
> file. Otherwise `T-NNN` is in **first-attempt shape**.

The rule reads only the journal file. It does not read TASKS.md,
does not invoke `git`, does not call `speccy next`, and does not
invoke any other CLI subcommand. Detection is mechanical: parse
the journal's XML elements (using the same closed-set journal
grammar `<implementer>` / `<review>` / `<blockers>` enforced by
the `JNL-*` lint family), read the `round` attributes, compare.

The canonical statement of the rule lives in a reference file
sibling to the existing journal references. REQ-002 and REQ-003
specify how each reader site picks up the rule.

<done-when>
- A `references/retry-shape.md` reference file exists under the
  Speccy resource tree alongside the existing journal references
  (mirroring the `references/journal-implementer.md` pattern from
  `.claude/skills/speccy-work/`). It carries the rule statement
  above plus one worked example showing a journal in retry shape
  plus one worked example showing a journal in first-attempt
  shape.
</done-when>

<behavior>
- Given a fresh journal file that contains exactly one
  `<implementer round="1">` block and no `<blockers>` blocks, when
  the rule is applied, then the task is in first-attempt shape
  (so the strict clean-tree gate applies).
- Given a journal file that contains
  `<implementer round="1">…</implementer>` followed by
  `<blockers round="1">…</blockers>`, when the rule is applied,
  then the task is in retry shape (so the dirty tree is permitted
  on the next work dispatch).
- Given a journal file that contains two rounds of
  implementer + blockers and a third `<implementer round="3">`
  block (the round-3 implementer has run but no round-3 review has
  fired yet), when the rule is applied, then the task is in
  first-attempt shape (no `<blockers round="3">` block exists, so
  the highest implementer round does not have a matching blockers
  block — the task is awaiting review, not a retry).
</behavior>

<scenario id="CHK-001">
Given a test fixture journal file containing exactly:
```
<implementer round="1" date="2026-05-26T00:00:00Z" model="claude-opus-4.7[1m]/low">
... body ...
</implementer>
```
when the retry-shape rule is applied,
then the result is first-attempt shape,
and the dirty-tree gate in the calling skill remains strict.
</scenario>

<scenario id="CHK-002">
Given a test fixture journal file containing
`<implementer round="1">…</implementer>` followed by
`<blockers round="1">…</blockers>`,
when the retry-shape rule is applied,
then the result is retry shape,
and the calling skill permits a dirty tree on the next dispatch.
</scenario>

<scenario id="CHK-003">
Given a test fixture journal file containing two rounds of
`<implementer>` + `<blockers>` and a trailing
`<implementer round="3">` block with no subsequent `<blockers
round="3">`,
when the retry-shape rule is applied,
then the result is first-attempt shape (the highest implementer
round has no matching blockers).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Work dispatch precondition becomes retry-aware

The clean-tree precondition shipped at SPEC-0045's REQ-002 is
extended to be retry-aware at both invocation paths it runs at:
standalone `/speccy-work` entry and `/speccy-orchestrate`'s work
dispatch (before spawning the speccy-work sub-agent).

The extended precondition reads the resolved task's journal file
and applies the REQ-001 retry-shape rule before inspecting
`git status --porcelain`:

1. If the task is in **first-attempt shape**, the strict clean-tree
   gate applies as today: a non-empty `git status --porcelain`
   halts the dispatch and surfaces the dirty paths.
2. If the task is in **retry shape**, the precondition permits a
   dirty tree and proceeds to dispatch the implementer. An empty
   `git status --porcelain` is also acceptable in this branch
   (idempotent: a retry whose prior WIP was somehow already cleaned
   simply restarts from clean state).

Both invocation paths document the extended precondition at the
same entry-point step they document today's strict gate. The
shared retry-shape rule from REQ-001 is inlined at each site (or
read from the central reference) and bounded by the marker comment
naming the rule.

<done-when>
- `/speccy-work`'s skill body documents the retry-aware
  precondition: read the resolved task's journal, apply the
  REQ-001 rule, then inspect `git status --porcelain`.
- `/speccy-orchestrate`'s work dispatch section documents the same
  retry-aware precondition before spawning the speccy-work
  sub-agent.
- Both skill body files (`.claude/skills/speccy-work/SKILL.md` and
  `.claude/skills/speccy-orchestrate/SKILL.md`) carry the REQ-001
  rule statement verbatim, bounded by the marker comment pair
  `<!-- Shared rule: retry-shape. Source: references/retry-shape.md -->`
  / `<!-- End shared rule: retry-shape. -->` so a future
  contributor editing one site is signposted to the other. The
  rule text between the markers is byte-for-byte identical to the
  REQ-001 reference file after whitespace normalisation. The
  mirrored `.agents/skills/...` host-portable copies stay in sync
  via the existing resource templating pipeline.
- The pre-existing dirty-paths surface still fires in the
  first-attempt branch: a dirty tree on a first-attempt task halts
  with the same stderr shape as SPEC-0045's REQ-002 behaviour.
- A dirty tree on a retry-shape task does not halt; the dispatch
  proceeds to the implementer.
</done-when>

<behavior>
- Given a clean working tree and a first-attempt task, when
  `/speccy-work` begins, then the implementer sub-agent is
  spawned (unchanged from today).
- Given a dirty working tree and a first-attempt task (journal
  file does not exist, or has no `<implementer>` blocks), when
  `/speccy-work` begins, then the skill halts before spawning the
  implementer and stderr carries the dirty-paths surface
  (unchanged from today).
- Given a dirty working tree and a retry-shape task (journal has
  `<implementer round="1">` + `<blockers round="1">`), when
  `/speccy-work` begins, then the skill spawns the implementer
  sub-agent without halting; no dirty-paths surface is written.
- Given a dirty working tree and a retry-shape task, when
  `/speccy-orchestrate` reaches its work dispatch step, then the
  outer loop proceeds to spawn the speccy-work sub-agent rather
  than halting the loop with the dirty-paths surface.
- Given a clean working tree and a retry-shape task (the prior
  pass's WIP was somehow cleared before the retry), when either
  invocation path begins, then the implementer sub-agent spawns
  normally and the retry-aware implementer prompt (REQ-003) reads
  the latest `<blockers>` from the journal and starts fresh.
</behavior>

<scenario id="CHK-004">
Given a working tree with one unstaged modification to a tracked
file and a per-task journal file containing exactly one
`<implementer round="1">…</implementer>` block (no `<blockers>`
block),
when `/speccy-work SPEC-NNNN/T-NNN` is invoked,
then the skill exits before any implementer dispatch happens,
and stderr contains a `git status --porcelain` format prefix line
for the dirty file (matching the SPEC-0045/REQ-002 baseline
behaviour).
</scenario>

<scenario id="CHK-005">
Given a working tree with one unstaged modification to a tracked
file and a per-task journal file containing
`<implementer round="1">…</implementer>` followed by
`<blockers round="1">…</blockers>`,
when `/speccy-work SPEC-NNNN/T-NNN` is invoked,
then the skill proceeds to spawn the implementer sub-agent,
and no dirty-paths surface is written to stderr.
</scenario>

<scenario id="CHK-006">
Given a workspace with TASKS.md T-001 at `state="pending"`, a
journal file with `<implementer round="1">` + `<blockers round="1">`,
a dirty working tree carrying the round-1 WIP, and
`speccy next SPEC-NNNN --json` returning
`next_action.kind == "work"` with `next_action.task_id == "T-001"`,
when `/speccy-orchestrate SPEC-NNNN` reaches its work dispatch
step,
then the outer loop spawns one speccy-work sub-agent for T-001
without halting,
and no dirty-paths surface is written to the orchestrator's
output.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Implementer prompt grows a retry-aware mode

The speccy-work implementer (the prompt body in
`.claude/agents/speccy-work.md` and its host-portable mirrors)
applies the REQ-001 retry-shape rule as its first action after
resolving the target task:

1. Resolve the target task per the existing step 1.
2. Read the per-task journal file
   `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the
   REQ-001 retry-shape rule.
3. If the task is in **first-attempt shape**, proceed with the
   existing flow: flip state to `in-progress`, read scenarios,
   implement, hygiene-gate, flip to `in-review`, append the
   `<implementer round="1">` block.
4. If the task is in **retry shape**, enter retry mode: read the
   latest `<blockers>` block in the journal and the most recent
   `<implementer>` block to understand the prior pass's stated
   `Completed` work, then **amend the existing WIP in the working
   tree** to address the blockers rather than starting from a
   clean reimplementation. After the amend, run the standard
   hygiene suite (per SPEC-0045/REQ-001) and only then flip
   `state` from `in-progress` to `in-review` and append the next
   `<implementer round="N+1">` block (with `N` being the highest
   prior implementer round). Do not delete or rewrite earlier
   journal blocks.

The retry-mode implementer's `<implementer>` block follows the
same six-field handoff template (`Completed`, `Undone`, `Hygiene
checks`, `Evidence`, `Discovered issues`, `Procedural compliance`)
as the first-attempt path. The `Completed` field describes what
changed in this round (the amend), not a full restatement of the
prior round's completed work.

<done-when>
- The speccy-work agent prompt at `.claude/agents/speccy-work.md`
  documents the retry-shape branch at the same step that
  documents the first-attempt branch. The agent prompt carries
  the REQ-001 rule statement verbatim, bounded by the marker
  comment pair
  `<!-- Shared rule: retry-shape. Source: references/retry-shape.md -->`
  / `<!-- End shared rule: retry-shape. -->`. The rule text
  between the markers is byte-for-byte identical to the REQ-001
  reference file after whitespace normalisation.
- The retry-mode branch instructs the implementer to read the
  latest `<blockers>` block and the most recent `<implementer>`
  block before editing files.
- The retry-mode branch instructs the implementer to amend the
  WIP in place rather than reset the tree or rewrite files from
  scratch.
- The retry-mode branch routes through the same SPEC-0045/REQ-001
  hygiene gate before the `state` flip to `in-review`.
- The retry-mode `<implementer>` block carries
  `round="N+1"` where `N` is the highest prior round in the
  journal, monotonically incremented by exactly 1.
- The retry-mode `Completed` field describes the amend (what
  changed in this round), not the cumulative task work.
</done-when>

<behavior>
- Given a journal with `<implementer round="1">` and
  `<blockers round="1">` whose body names a style-persona
  blocker about a specific function, when the retry implementer
  runs, then it reads the blocker, edits the specific function
  in place (rather than rewriting the surrounding file), runs the
  hygiene suite, and appends an `<implementer round="2">` block
  whose `Completed` field describes the targeted edit.
- Given the same journal but with a clean working tree (prior WIP
  was cleared before the retry dispatched), when the retry
  implementer runs, then it reads the blocker, reconstructs the
  needed change from scratch using the prior `<implementer>`
  block's `Completed` description as context, and proceeds with
  the standard flow — round still increments to 2.
- Given a multi-round retry where round 1, round 2, and round 3
  have each blocked, when the round-4 implementer runs, then it
  reads the round-3 `<blockers>` block (not round-1 or round-2)
  and amends the existing WIP per round-3's feedback.
</behavior>

<scenario id="CHK-007">
Given a per-task journal at
`<spec-dir>/journal/T-001.md` containing
`<implementer round="1">…</implementer>` followed by
`<blockers round="1">Style: drop the println! short-circuit and
group anyhow imports.</blockers>`,
and a working tree dirty with the round-1 implementer's code,
when `/speccy-work SPEC-NNNN/T-001` runs to completion and the
implementer appends its block to the journal,
then the appended block is `<implementer round="2">…</implementer>`,
and its `Completed` field references the blocker-driven amend
(e.g. removing the `println!` short-circuit and grouping the
imports) rather than a full restatement of the round-1 task work.
</scenario>

<scenario id="CHK-008">
Given a per-task journal with two completed retry rounds
(round-1 and round-2 each have implementer + blockers) and a
working tree dirty with the round-2 implementer's code,
when `/speccy-work SPEC-NNNN/T-001` runs in retry mode,
then the appended block is `<implementer round="3">…</implementer>`,
and the `Completed` field references the round-2 blockers
specifically (not round-1).
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `/speccy-decompose` commits SPEC + TASKS as final step

The `/speccy-decompose` skill commits the SPEC artefacts as the
final step of its execution, before returning. This closes the
bootstrap-commit gap that trips the SPEC-0045/REQ-002 strict
clean-tree gate when `/speccy-orchestrate SPEC-NNNN` starts on a
freshly decomposed SPEC.

The commit step runs after `speccy lock SPEC-NNNN` writes the
final spec hash and timestamp into TASKS.md's frontmatter:

1. Stage the SPEC's own files via narrow `git add` calls (not
   `git add -A`):

   ```
   git add <spec-dir>/SPEC.md <spec-dir>/TASKS.md
   ```

   Both files are staged regardless of whether SPEC.md was already
   committed; staging unchanged content is a no-op.

2. Run `git diff --cached --quiet`. If the call exits 0 (nothing
   staged), skip the commit silently — both files are already
   committed at their current content. If the exit code is
   non-zero, proceed to commit.

3. Commit with the message format:

   - **Title:** `[SPEC-NNNN]: create spec and decompose tasks`.
   - **Body:** the SPEC's `title:` frontmatter value read from
     SPEC.md (the one-line slug after the `title:` key).
   - **Trailer:** `Co-Authored-By: <model>
     <noreply@anthropic.com>` resolved per SPEC-0045/REQ-004's
     host-harness convention, with the documented `Speccy Skill
     Pack` fallback when the host exposes no model identifier.

   Pass the body via a HEREDOC so newlines survive verbatim,
   matching SPEC-0045/REQ-004's commit invocation pattern.

The commit captures exactly two files: `SPEC.md` and `TASKS.md`
under the spec's own directory. The narrow `git add` invocation
guarantees no unrelated dirty paths are swept in, so the soundness
argument from SPEC-0045/REQ-003 (which assumes a clean-tree
precondition) does not need to apply here — `git add -A` is
deliberately not used.

This requirement applies only to `/speccy-decompose`. The
analogous case for `/speccy-plan` (committing SPEC.md when plan
finishes) and `/speccy-amend` (committing SPEC.md + reconciled
TASKS.md after an amendment) is out of scope; see Non-goals.

<done-when>
- The `/speccy-decompose` skill body
  (`.claude/skills/speccy-decompose/SKILL.md` and host-portable
  mirrors) documents the commit step as the final step of the
  recipe, after `speccy lock SPEC-NNNN`.
- The same is documented in the speccy-decompose agent prompt
  (`.claude/agents/speccy-decompose.md` and any Codex variant),
  inserted as a new step between today's step 3 (`speccy lock`)
  and step 4 ("Suggest the next step").
- The commit step uses narrow `git add <spec-dir>/SPEC.md
  <spec-dir>/TASKS.md` rather than `git add -A`.
- The commit step skips silently when `git diff --cached --quiet`
  reports nothing staged (e.g. re-running decompose on an
  already-committed SPEC).
- The commit message title is `[SPEC-NNNN]: create spec and
  decompose tasks` with the SPEC's `title:` field as the body.
- The commit message carries the `Co-Authored-By` trailer
  resolved per SPEC-0045/REQ-004's host-harness convention.
</done-when>

<behavior>
- Given a working tree with an untracked `<spec-dir>/SPEC.md` and
  no `<spec-dir>/TASKS.md`, when `/speccy-decompose SPEC-NNNN`
  runs to completion, then exactly one new commit lands whose
  tree contains both `<spec-dir>/SPEC.md` and
  `<spec-dir>/TASKS.md`, whose title is `[SPEC-NNNN]: create
  spec and decompose tasks`, and whose body is the SPEC's `title:`
  field.
- Given a working tree where `<spec-dir>/SPEC.md` is already
  committed at HEAD and `<spec-dir>/TASKS.md` is untracked, when
  `/speccy-decompose SPEC-NNNN` runs to completion, then exactly
  one new commit lands whose tree adds `<spec-dir>/TASKS.md` only;
  the title is still `[SPEC-NNNN]: create spec and decompose
  tasks`.
- Given a working tree where both files are already committed at
  HEAD at the same content the decompose run would produce, when
  `/speccy-decompose` is re-run for the same SPEC, then no new
  commit lands (the staged diff is empty and the commit step
  skips silently).
- Given a working tree with unrelated dirty paths outside
  `<spec-dir>/` (e.g. a stray edit to `src/lib.rs`) plus an
  untracked `<spec-dir>/TASKS.md` from the decompose run, when
  `/speccy-decompose SPEC-NNNN` runs to completion, then the
  resulting commit contains only the two SPEC files; the
  unrelated dirty paths remain dirty and untouched in the tree.
- Given that the decompose commit landed, when the next caller
  runs `/speccy-orchestrate SPEC-NNNN`, then the orchestrator's
  startup integrity check sees an empty `git status --porcelain`
  (or only paths unrelated to the SPEC), and the outer loop
  proceeds to its first work dispatch without halting.
</behavior>

<scenario id="CHK-009">
Given a fresh working tree where `<spec-dir>/SPEC.md` is
untracked and `<spec-dir>/TASKS.md` does not yet exist on disk,
when `/speccy-decompose SPEC-NNNN` runs to completion,
then `git log -1 --format=%s` returns `[SPEC-NNNN]: create spec
and decompose tasks`,
and `git log -1 --name-only --format=` lists exactly two paths
`<spec-dir>/SPEC.md` and `<spec-dir>/TASKS.md`,
and `git status --porcelain` is empty.
</scenario>

<scenario id="CHK-010">
Given a working tree with an unrelated stray edit to `src/lib.rs`
and an untracked `<spec-dir>/SPEC.md`,
when `/speccy-decompose SPEC-NNNN` runs to completion,
then the resulting commit's name-only listing contains exactly
`<spec-dir>/SPEC.md` and `<spec-dir>/TASKS.md` (the `src/lib.rs`
modification is not present in the commit),
and `git status --porcelain` after the commit still shows the
`src/lib.rs` modification untouched.
</scenario>

<scenario id="CHK-011">
Given a working tree where `<spec-dir>/SPEC.md` and
`<spec-dir>/TASKS.md` are both already committed at HEAD with the
exact content `/speccy-decompose` would re-emit,
when `/speccy-decompose SPEC-NNNN` is re-invoked,
then `git rev-parse HEAD` returns the same SHA as before
(no new commit was created),
and no `git commit` stderr line surfaces a "nothing to commit"
error.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
### DEC-001: Review-blocked retry is normal flow, not consistency drift

**Status:** Accepted

**Context:** The shipped consistency enum (SPEC-0045/REQ-006)
classifies several `(state, tree, commit)` tuples as drift kinds
that the reconcile policy auto-handles. The
`(state=pending, dirty tree, no commit, journal has prior
<implementer> + <blockers>)` tuple is invisible to that enum
today. One natural fix would be to extend the enum with a new
kind for this tuple and route the retry through the reconcile
dispatch.

**Decision:** The review-blocked retry case is **not** modelled as
a consistency drift kind. The detection and handling stay local
to the skill bodies (REQ-002, REQ-003).

**Alternatives:**

- *Add a new `state_pending_retry_dirty` drift kind to the
  consistency enum.* Rejected: the reconcile policy is designed
  for crashed sessions where the running session did not finish
  what it started — the in-flight task state is genuinely
  ambiguous, and the policy must pick a tiebreaker. A
  review-blocked retry is the opposite: the prior session
  finished what it intended (a complete implementer pass + a
  complete reviewer fan-out), the state machine is in a
  well-defined position (`state=pending` with blockers ready to
  read), and the next action is fully determined by the loop. No
  policy decision is needed — only the precondition needs to
  recognise the shape. Routing through reconcile would add a
  level of indirection (consistency block → reconcile partial →
  retry-aware action) for no semantic gain.

- *Have the orchestrator discard the dirty tree on the
  `in-review → pending` flip, restoring the strict invariant.*
  Rejected (this was Option B in the brainstorm): throws away the
  prior pass's hygiene-clean structural work in cases where the
  blocker calls for a small targeted change. Each retry pays the
  full implementer cost, burning the per-task retry budget faster
  than necessary.

**Consequences:** The consistency block in `speccy next --json`
stays unchanged; downstream consumers (the reconcile partial, any
future host that reads consistency) need no adjustment. The
retry-shape rule lives entirely in skill bodies, mirroring the
"intelligent edges" half of Speccy's deterministic-core /
intelligent-edges principle.

</decision>

<decision id="DEC-002">
### DEC-002: Detection rule lives in skill bodies, not the Rust CLI

**Status:** Accepted

**Context:** The retry-shape detection rule is a deterministic
file-read predicate that could plausibly live in the CLI (as a
new field on `speccy next --json`'s envelope, for example,
analogous to the consistency block) or in the skill bodies
(read directly by the dispatch and the agent prompt).

**Decision:** The rule lives in the skill bodies. The CLI does
not learn it.

**Alternatives:**

- *Add a `retry_shape: bool` field per task to
  `speccy next --json`'s envelope.* Rejected: the field would be
  used only by two skill bodies and the agent prompt; no other
  downstream consumer needs it. Adding a CLI surface for a
  two-reader value violates "stay small" and accumulates schema
  surface that future versions must preserve. The deterministic
  core stays read-only re: this rule; skills do their own
  file-read.

- *Introduce a `speccy retry-shape SPEC-NNNN/T-NNN` subcommand
  that prints `retry|first-attempt`.* Rejected for the same
  reason plus an additional one: it introduces a new CLI
  subcommand whose only callers are skill bodies running on the
  same machine that can read the journal file directly. A shell
  exec for a five-byte answer is needless overhead.

**Consequences:** Three sites (the two skill bodies and the
agent prompt) carry the same rule text. Adding a fourth site
requires copying the rule there too. DEC-003 addresses the
duplication strategy.

</decision>

<decision id="DEC-003">
### DEC-003: Rule replicated across sites without a shared partial

**Status:** Accepted

**Context:** Speccy ships two shared-markdown-partial precedents:
the four-persona fan-out partial (~80 lines) and the
reconcile-policy partial (~95 lines). Both are large bodies of
co-edited prose where the cost of maintaining three inlined
copies is high enough to justify the build-time-or-manual sync
ceremony.

The retry-shape rule is small: ~10 lines of policy plus a marker
comment. Three sites carry it.

**Decision:** No shared partial. The rule is replicated verbatim
across the three sites, bounded by a marker comment
(`<!-- Shared rule: retry-shape. Source:
references/retry-shape.md -->`) that names the reference file
so a future contributor editing one site is signposted to the
others. The reference file at `references/retry-shape.md` carries
the canonical statement.

**Alternatives:**

- *Promote the rule to a shared markdown partial matching the
  reconcile-policy precedent.* Rejected: the rule is too small
  to amortise the partial-syncing overhead. The four-persona and
  reconcile partials each pull their weight; a 10-line partial
  is a maintenance liability without offsetting clarity gain.

- *Co-edit the three sites with no marker comment.* Rejected:
  future contributors editing one site would have no signal that
  two other sites need to stay in sync. The marker comment is
  free.

**Consequences:** Adding a fourth reader of the retry-shape rule
requires the contributor to: (a) copy the rule body and marker
comment to the new site, (b) verify it matches the reference
file. If the rule ever grows substantially (e.g., grows to model
multi-blocker rounds with cross-cutting feedback), this decision
is revisited.

</decision>

## Assumptions

<assumptions>
- The journal closed-set grammar (`<implementer>`, `<review>`,
  `<blockers>`) and the required `round` attribute on each are
  stable per the shipped `JNL-*` lint family. The retry-shape
  rule's correctness depends on `round` being a monotonic
  positive integer per the existing implementer prompt; this is
  already documented in
  `.claude/skills/speccy-work/references/journal-implementer.md`.
- A blocking review writes exactly one consolidated `<blockers>`
  block per round (per SPEC-0045's existing review-dispatch
  language: "append a single consolidated `<blockers>…</blockers>`
  element block to `journal/T-NNN.md` that aggregates all failing
  reviewers' feedback — not one `<blockers>` per reviewer"). The
  detection rule relies on this consolidation invariant.
- The host harness's resource templating pipeline keeps
  `.claude/skills/...` and `.agents/skills/...` in sync via the
  existing `resources/modules/` source files. This SPEC's edits
  flow through the same pipeline; no new sync mechanism is
  introduced.
- The retry-mode implementer trusts the dirty tree's contents to
  be the prior pass's WIP, not unrelated edits. SPEC-0045's
  REQ-002 strict-gate invariant on first-attempt dispatch
  guarantees this: the working tree was clean before the round-1
  implementer ran, so every dirty path after a blocking review
  traces to the round-1 implementer. The clean-tree-clean-task
  invariant carries through every subsequent round by induction.
- `speccy next --json` continues to return
  `next_action.kind == "work"` when a task is at
  `state="pending"`. The retry-shape distinction is invisible to
  the CLI; only the dispatching skill body knows whether the
  pending state is "first attempt" or "retry".
- The orchestrator's per-task retry budget (5 rounds, tracked in
  memory) continues to bound the maximum retry depth. A
  retry-shape task is not exempt from the budget; round 5's
  blocker still triggers the stop condition.
</assumptions>

## Notes

The friction surfaced concretely while dogfooding Speccy on the
`patina` greenfield project: `/speccy-orchestrate` dispatched
T-001, the implementer wrote code that passed hygiene, the
reviewer fan-out returned `verdict="blocking"` from the style
persona for a `println!` outside the reporter layer, the
orchestrator flipped state to `pending` and appended the
`<blockers>` block, and the next loop iteration's work dispatch
halted on the SPEC-0045/REQ-002 strict gate because the round-1
WIP was still in the tree. Round 1/5 of the per-task retry budget
was used without making any progress past the blocker.

The brainstorm that preceded this SPEC walked three framings:

1. **Retry-aware precondition (the chosen path).** Captured by
   REQ-001 through REQ-003 above.

2. **Discard the dirty tree on the `in-review → pending` flip.**
   Considered and rejected per DEC-001's alternatives: throws
   away the prior pass's hygiene-clean WIP, forces each retry to
   redo structural work for what may be a one-line blocker fix,
   and burns the per-task retry budget faster than necessary.

3. **Commit failed passes with a distinct title prefix, then
   revert.** Considered and rejected: muddies git history with
   intentionally-failed commits, breaks the SPEC-0045/REQ-004
   title-prefix-means-task-durably-completed contract, and
   requires the consistency check to learn a second prefix
   variant.

The chosen path is the smallest delta that closes the gap: it
preserves every shipped invariant from SPEC-0045 (commit message
format, atomic commit semantics, hygiene gate, persona fan-out
shape, per-task retry budget) and adds only one new file (the
retry-shape reference) plus three co-edited inline rules.

## Open Questions

(None — brainstorm closed with no open questions remaining.)

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-26 | human/kevin  | Initial draft. Introduces three requirements covering: (REQ-001) the retry-shape detection rule as a deterministic, file-read predicate canonically documented in a new `references/retry-shape.md` reference; (REQ-002) extension of SPEC-0045/REQ-002's clean-tree precondition at both `/speccy-work` entry and `/speccy-orchestrate` work dispatch to permit a dirty tree when the task is in retry shape, with the rule inlined verbatim at both skill body sites bounded by marker comments; (REQ-003) growth of the speccy-work implementer prompt with a retry-aware mode that reads the latest `<blockers>` and amends the WIP in place rather than reimplementing from scratch, with the rule inlined verbatim at the agent prompt site bounded by marker comments. Three DECs codify: (DEC-001) review-blocked retry is normal flow, not consistency drift, so the reconcile enum is not extended; (DEC-002) the detection rule lives in skill bodies, not the CLI; (DEC-003) the rule is replicated across three sites without a shared partial because it is small enough that the partial overhead is not justified. |
| 2026-05-26 | human/kevin  | Add REQ-004: `/speccy-decompose` commits SPEC.md + TASKS.md atomically as the final step of decompose completion. Title `[SPEC-NNNN]: create spec and decompose tasks`; body is the SPEC's `title:` frontmatter value; trailer per SPEC-0045/REQ-004. Narrow `git add <spec-dir>/SPEC.md <spec-dir>/TASKS.md` keeps unrelated dirty paths out of the commit; `git diff --cached --quiet` makes the step idempotent on re-runs. Closes the bootstrap-commit gap that trips SPEC-0045/REQ-002's strict clean-tree gate when `/speccy-orchestrate SPEC-NNNN` is invoked on a freshly decomposed SPEC. T-005 added to implement in the `/speccy-decompose` skill body, agent prompt, and host-portable mirrors. Non-goals updated to record that the analogous pattern for `/speccy-plan` and `/speccy-amend` is explicitly out of scope for this SPEC; CHK-009/CHK-010/CHK-011 cover REQ-004. Amendment driven by the user surfacing the bootstrap-commit friction during the initial `/speccy-orchestrate SPEC-0047` invocation. |
</changelog>
