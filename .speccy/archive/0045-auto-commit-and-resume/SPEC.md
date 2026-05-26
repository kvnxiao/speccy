---
id: SPEC-0045
slug: auto-commit-and-resume
title: Auto-commit on review pass + autonomous crash-resume
status: implemented
created: 2026-05-25
supersedes: []
---

# SPEC-0045: Auto-commit on review pass + autonomous crash-resume

## Summary

Speccy today drives the Plan → Tasks → Implement → Review loop end-to-end inside a single session, but the orchestration is not crash-safe. When `/speccy-orchestrate` runs unattended and a session terminates mid-loop (host crash, network drop, OS reboot), there is no durable boundary between completed and in-flight work. TASKS.md edits, journal appends, and code changes all live in the working tree until the user manually commits at the end. A resumed session sees a working tree full of changes whose provenance is unclear — was T-003 finished or is it the half-written T-004?

This SPEC introduces two coupled mechanisms that together turn each completed task into a durable save point:

1. **Per-task atomic commit on review pass.** After every review fan-out returns all `verdict="pass"`, the running session appends the consolidated `<review>` blocks to the per-task journal, flips TASKS.md state to `completed`, and produces one atomic git commit containing the code, state, and journal updates. The commit title carries a stable task-identifier prefix (`[SPEC-NNNN/T-NNN]:`) that the consistency check uses to correlate commits back to tasks. Standalone `/speccy-review` invocations exercise the same code path with identical semantics — orchestrate is just a composition over single-task primitives.

2. **Autonomous reconciliation on resume.** `speccy next` gains a `consistency` block in its JSON envelope that reports drift between TASKS.md task state and git log task commits. When drift is detected, `next_action.kind` flips to `reconcile`, and a fixed per-drift-kind policy table runs autonomously to converge state — no user prompts, no forks. The policy is rollback-biased: when recovery is ambiguous, it prefers discarding partial progress to preserving inconsistent state, leaning on the orchestrator's per-task retry budget to redo the work cleanly. Every action is idempotent, so successive crashes during reconciliation converge to the same eventual state.

The two mechanisms compose: per-task commits provide the durable boundary, and autonomous reconciliation closes the only gap (the brief window between TASKS.md edit and git commit) by detecting mismatches deterministically from git log and acting on them without human input.

Two preconditions make the commit step safe to fire mechanically. First, the implementer skill body runs the project's standard hygiene suite before flipping state to `in-review` — preventing the costly path where four reviewer personas fan out only to have the final commit's pre-commit hooks reject the work for lint or fmt issues. Second, `/speccy-work` and the orchestrator's work dispatch refuse to begin a task when the working tree is not clean, which makes "every dirty file at commit time is task-scoped" a structural invariant rather than an assumption — so `git add -A` is sound.

The reconcile policy itself ships as a single markdown partial at `.claude/speccy-references/reconcile-policy.md` and is inlined into `/speccy-orchestrate`, `/speccy-work`, and `/speccy-review` skill bodies via the same shared-partial convention already used by the four-persona fan-out partial. The Rust CLI's role is limited to detecting drift and emitting structured facts; it never invokes `git` itself, and it never applies fixes. Policy lives at the intelligent edges; the deterministic core stays read-only.

## Goals

<goals>
- Every task that reaches `state="completed"` is captured in exactly one
  atomic git commit containing its code changes, the TASKS.md state
  flip, and the appended `<review>` journal blocks.
- Every per-task commit's title matches the pattern
  `^\[SPEC-\d{4}/T-\d{3}\]: .+$` so the consistency check can correlate
  commits to tasks via title-prefix grep.
- A session that crashes mid-loop and is resumed by re-invoking
  `/speccy-orchestrate SPEC-NNNN` autonomously reconciles any drift
  between TASKS.md and git log without prompting the user.
- `speccy next --json` always carries a `consistency` block reporting
  whether drift was detected and, when it was, the per-drift-kind
  details a skill needs to act on the drift.
- The reconcile policy is sourced from a single markdown partial at
  `.claude/speccy-references/reconcile-policy.md`; all three skill
  bodies that need it carry verbatim copies bounded by marker
  comments naming the shared partial.
- The Rust CLI never invokes `git` as a side effect; drift detection
  is read-only and emission-only.
</goals>

## Non-goals

<non-goals>
- No per-task git worktrees. Each task commits to the active working
  tree on the user's current branch; the orchestrator does not create
  or manage worktrees.
- No `/speccy-resume` skill and no `speccy reconcile` CLI subcommand.
  Resume is the existing `/speccy-orchestrate SPEC-NNNN` invocation;
  reconcile dispatch lives inside skill bodies via the shared partial.
- No CLI-side `git commit`, `git add`, or `git status` invocation. The
  Rust binary reads `.git/` (via standard libgit2 / process out of
  scope here; current implementation uses `git log --grep` if needed)
  but issues no mutating commands and runs no hooks.
- No branch safety check. The skill does not refuse to commit on
  `main` or `master`; the user / orchestrator host is trusted to
  have placed the working tree on a feature branch.
- No support for squash-merge of an in-flight SPEC's branch. The
  consistency check correlates commits via title prefix, which
  survives `git rebase` but not a squash whose merge commit uses the
  PR title. Squash happens post-ship; consistency does not run on
  shipped specs.
- No new attribute on the `<implementer>` journal element. The commit
  body comes from mechanical extraction of the existing `Completed`
  field of the six-field handoff template.
- No fallback path that surfaces a drift fork to the user during the
  orchestration loop. Reconcile either converges autonomously or, in
  unrecoverable cases, rolls backward and lets the loop redo the work.
- No hard round budget on the reconcile loop. The mechanism relies on
  idempotency plus re-detectability on subsequent sessions; the
  orchestrator's existing per-task retry budget (5 rounds) bounds the
  total redo cost.
</non-goals>

## User Stories

<user-stories>
- As a solo developer running `/speccy-orchestrate SPEC-NNNN`
  overnight, I want each completed task to land as an atomic commit
  so my git log shows incremental progress and `git bisect` can
  pinpoint regressions to a single task.
- As a developer whose laptop reboots mid-orchestration, I want to
  re-invoke `/speccy-orchestrate SPEC-NNNN` and have the next session
  pick up at the most recent durable commit without me having to
  diagnose what was in flight when the crash hit.
- As a future multi-agent harness driving Speccy autonomously across
  multiple host machines, I want a session crash to be invisible to
  downstream consumers — the next session reconciles silently and
  continues until the SPEC is shipped, with no human in the loop.
- As a reviewer reading the git history of a shipped SPEC after the
  fact, I want every commit to grep-correlate to its task via a
  stable title prefix so I can audit per-task changes without
  cross-referencing TASKS.md.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Implementer enforces standard hygiene before flipping state to `in-review`

The implementer skill body (`/speccy-work` and the agent at
`.claude/agents/speccy-work.md`) runs the project's standard hygiene
suite — the four gates documented in `AGENTS.md`'s "Standard hygiene"
section, or the project-equivalent set — after making code changes
and before flipping the task's TASKS.md `state` attribute from
`in-progress` to `in-review`. If any hygiene check exits non-zero,
the state flip is refused; the implementer either iterates on the
fix or surfaces the failure to the caller. The hygiene exit codes
land in the `<implementer>` block's existing `Hygiene checks` field
per the six-field handoff template in
`.claude/skills/speccy-work/references/journal-implementer.md`.

Promoting hygiene from a convention to a hard gate keeps reviewer
fan-out (the most expensive step in the loop — four sub-agents in
parallel) from being spent on work that the commit step would
later reject for lint or fmt issues.

<done-when>
- `/speccy-work` does not flip the task to `state="in-review"` when
  any of `cargo test --workspace`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo +nightly fmt
  --all --check`, or `cargo deny check` exits non-zero.
- An implementer turn that passes all four hygiene gates writes
  exit-code lines for each gate into the `Hygiene checks` field of
  its `<implementer>` block.
- The `/speccy-work` skill body (and the agent file it dispatches to)
  documents the gate explicitly — "Run the standard hygiene suite;
  refuse the state flip on any non-zero exit" — so future implementer
  authors do not rediscover this discipline from `AGENTS.md` alone.
</done-when>

<behavior>
- Given an implementer turn that adds code with a clippy warning,
  when `/speccy-work` reaches the hygiene step, then `cargo clippy
  ... -- -D warnings` exits non-zero and the state flip is refused.
- Given an implementer turn whose code passes all four gates, when
  `/speccy-work` reaches the hygiene step, then state flips from
  `in-progress` to `in-review` and the `<implementer>` block records
  four `exit 0` lines under `Hygiene checks`.
</behavior>

<scenario id="CHK-001">
Given an implementer sub-agent has just introduced a clippy
warning in a touched file,
when `/speccy-work`'s hygiene step runs `cargo clippy --workspace
--all-targets --all-features -- -D warnings`,
then the command exits non-zero,
and the task's `state` attribute in TASKS.md remains `in-progress`,
and no `<implementer>` block is appended that records a passing
state transition.
</scenario>


<scenario id="CHK-002">
Given an implementer sub-agent has finished a turn whose code passes
all four standard hygiene gates,
when `/speccy-work` runs the gates in sequence,
then each command exits 0,
and the running session flips the task's `state` attribute from
`in-progress` to `in-review`,
and the appended `<implementer>` block's `Hygiene checks` field
contains one line per gate naming its exit code as 0.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `/speccy-work` and orchestrator work dispatch refuse to start a task on a dirty working tree

Before spawning the implementer sub-agent, the running session runs
`git status --porcelain` and inspects the output. If the output is
non-empty (any untracked, unstaged, or staged change exists), the
skill refuses to start the task, surfaces the dirty paths, and
exits. This applies in two invocation paths: standalone
`/speccy-work` entry, and `/speccy-orchestrate`'s work dispatch
(before it spawns the speccy-work sub-agent).

This precondition is the foundation that makes `git add -A` at
commit time sound: under the invariant "working tree clean at every
task boundary", the only dirty paths when the commit step runs are
the ones the just-finished task created, so staging everything
sweeps in exactly the task scope and nothing else.

<done-when>
- `/speccy-work` invoked with a non-empty `git status --porcelain`
  exits before spawning the implementer sub-agent and writes the
  dirty paths to stderr.
- `/speccy-orchestrate`'s work dispatch performs the same check
  before spawning its speccy-work sub-agent; the outer loop halts
  with the same dirty-paths surface.
- The check is documented in both skill bodies (`/speccy-work` entry
  and `/speccy-orchestrate`'s dispatch section).
- A clean working tree (`git status --porcelain` exits with empty
  stdout) allows the implementer sub-agent to spawn normally.
</done-when>

<behavior>
- Given a working tree with one unstaged change to `src/foo.rs`,
  when `/speccy-work` begins, then the skill exits before spawning
  the implementer and the stderr surface contains the line `M
  src/foo.rs`.
- Given a working tree with one untracked file `scratch.txt`, when
  `/speccy-work` begins, then the skill exits and the surface
  contains the line `?? scratch.txt`.
- Given a clean working tree, when `/speccy-work` begins, then the
  implementer sub-agent is spawned.
- Given a dirty working tree, when `/speccy-orchestrate`'s work
  dispatch runs for the next pending task, then the orchestrator's
  outer loop halts with the dirty-paths surface; no implementer
  sub-agent is spawned.
</behavior>

<scenario id="CHK-003">
Given a freshly cloned workspace,
when an unstaged modification is made to any tracked file (e.g.,
`src/foo.rs`),
and `/speccy-work` is invoked,
then the skill exits before any Task tool dispatch happens,
and the stderr output contains a line matching the `git status
--porcelain` format prefix `M `.
</scenario>


<scenario id="CHK-004">
Given the same workspace with a clean working tree (verified by
`git status --porcelain` exiting with empty stdout),
when `/speccy-work` is invoked against a `state="pending"` task,
then the skill spawns the speccy-work sub-agent via the Task tool
without printing any dirty-tree warning.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Conditional atomic commit after review pass

After a review fan-out returns `verdict="pass"` from all spawned
reviewer personas, the running session performs three steps in
order:

1. Append the consolidated `<review>` blocks to the per-task journal
   file (`.speccy/specs/NNNN-slug/journal/T-NNN.md`).
2. Flip the task's `state` attribute in TASKS.md from `in-review` to
   `completed`.
3. Run `git status --porcelain`. If the output is non-empty, run
   `git add -A` followed by `git commit` with the message defined in
   REQ-004. If the output is empty, skip the commit step silently
   (no surface to the user, no error).

The conditional skip at step 3 handles two cases uniformly: tasks
whose work nets out to zero filesystem change (validation-only
tasks that generated and cleaned up temporary files), and
idempotent re-runs of the review-pass code path against an
already-converged state (which the reconcile mechanism in REQ-005
through REQ-007 may trigger).

Both `/speccy-orchestrate`'s review dispatch and standalone
`/speccy-review` execute steps 1-3 in the same running-session
context. The two invocation paths share the existing journal
fan-out partial; the new commit step lives in that same shared
partial so behavior is identical.

<done-when>
- After a passing review pass from `/speccy-orchestrate`, the
  resolved task's journal file contains the appended `<review>`
  blocks, TASKS.md shows the task at `state="completed"`, and the
  working tree is clean as verified by `git status --porcelain`.
- After a passing review pass from standalone `/speccy-review`, the
  same three observable states hold.
- When the review-pass code path runs on an already-committed task
  (the journal already contains the `<review>` blocks for the same
  round, TASKS.md already shows `completed`, working tree clean),
  no new commit is produced — `git log -1 --format=%H` returns the
  same SHA before and after.
- The commit produced by step 3 is a single commit (parent count =
  1) containing the code changes, the TASKS.md edit, and the
  journal edit in one tree.
</done-when>

<behavior>
- Given a task in `state="in-review"` whose four reviewer personas
  all return `verdict="pass"`, when the running session completes
  steps 1-3, then the commit step produces exactly one new commit
  whose tree contains the journal append, the TASKS.md state flip,
  and the implementer's code changes.
- Given the same configuration but with a clean working tree at
  step 3 (e.g., the running session crashed after the commit but
  before exiting; the reconciler detects clean state), when step 3
  runs, then no commit is produced and the running session exits
  cleanly.
- Given a `/speccy-review SPEC-NNNN/T-NNN` invocation outside any
  orchestrator session, when the four reviewer personas all return
  pass and steps 1-3 run, then the same single commit lands as in
  the orchestrate case.
</behavior>

<scenario id="CHK-005">
Given a `state="in-review"` task with the implementer's code
changes uncommitted in the working tree,
when the four reviewer personas all return `verdict="pass"` and the
running session completes the journal append, TASKS.md flip, and
commit step,
then `git log -1 --format=%P` returns exactly one parent SHA (a
single non-merge commit),
and the commit's tree (`git show --stat HEAD`) contains the code
files, TASKS.md, and the per-task journal file as modified paths,
and the working tree is clean (`git status --porcelain` exits with
empty stdout).
</scenario>


<scenario id="CHK-006">
Given the workspace immediately after CHK-005 succeeded (one
commit landed for the task, working tree clean),
when the review-pass code path is re-invoked against the same
task (simulating reconciler-driven re-entry on already-converged
state),
then no new commit is produced,
and `git log -1 --format=%H` returns the same SHA as before the
re-invocation.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Commit message format — title prefix, body source, and trailer

The commit produced by REQ-003 step 3 uses the following message
shape:

- **Title:** `[SPEC-NNNN/T-NNN]: <task title>` where `<task title>`
  is read verbatim from the task's title field in TASKS.md (the
  `title` attribute on the `<task>` element, or whatever field the
  task list grammar designates).
- **Body:** the content of the `Completed` field from the latest
  `<implementer>` block in the per-task journal file, extracted
  mechanically as the bytes between the `- Completed:` bullet
  marker and the next `- <Field>:` marker (where `<Field>` is one
  of the remaining five fields in the six-field handoff template).
  Leading and trailing whitespace is trimmed.
- **Trailer:** a single `Co-Authored-By: <model> <noreply@…>` line
  identifying the model running the skill at commit time. The
  model identifier is sourced from the host harness's runtime
  model-identification mechanism (env var, runtime API, or
  host-specific equivalent). Hosts that do not expose a model
  identifier write `Co-Authored-By: Speccy Skill Pack
  <noreply@anthropic.com>` as the fallback.

The title prefix is the sole task-identity link in the commit; the
consistency check in REQ-005 correlates commits to tasks by
grepping for this prefix. The body provides the human-readable
"what was done" content for `git log` reading.

<done-when>
- The commit's first line matches the regex
  `^\[SPEC-\d{4}/T-\d{3}\]: .+$`.
- The commit's body (lines after the blank line following the title,
  excluding the trailer block) equals the trimmed content of the
  `Completed` field of the latest `<implementer>` block in
  `journal/T-NNN.md` for the just-completed task.
- The commit message ends with a `Co-Authored-By: …` trailer line as
  reported by `git log -1 --format='%(trailers:key=Co-Authored-By,valueonly)'`.
- When the host harness exposes a model identifier (e.g., via an env
  variable the skill body queries), the trailer's identifier matches
  that value. When no identifier is available, the trailer uses the
  documented fallback string.
</done-when>

<behavior>
- Given task SPEC-0045/T-007 with title `Wire reconcile dispatch
  into /speccy-orchestrate startup`, when the commit lands, then
  the commit's first line is exactly `[SPEC-0045/T-007]: Wire
  reconcile dispatch into /speccy-orchestrate startup`.
- Given an `<implementer>` block whose `Completed` field reads
  `Added the dispatch branch in the orchestrate startup check;
  inlined the reconcile partial verbatim with marker comments`,
  when the commit lands, then the body of the commit message
  contains that text with leading/trailing whitespace trimmed.
- Given the skill body runs under a host that exposes the running
  model as `claude-opus-4.7[1m]`, when the commit lands, then the
  trailer line is `Co-Authored-By: claude-opus-4.7[1m] <noreply@anthropic.com>`
  (or the host's documented trailer-format equivalent).
</behavior>

<scenario id="CHK-007">
Given a freshly produced commit from REQ-003 step 3 for task
SPEC-0045/T-003 whose TASKS.md title is `Conditional atomic
commit after review pass`,
when `git log -1 --format='%s'` runs,
then stdout is exactly `[SPEC-0045/T-003]: Conditional atomic
commit after review pass`.
</scenario>


<scenario id="CHK-008">
Given the same commit,
when `git log -1 --format='%b'` runs (body only, no title or
trailers),
then stdout equals the trimmed content of the `Completed` field of
the latest `<implementer>` block in `journal/T-003.md`,
and `git log -1 --format='%(trailers:key=Co-Authored-By,valueonly)'`
returns a non-empty trailer value.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: `speccy next` emits a `consistency` block in its JSON envelope

The `speccy next` command (both per-spec form `speccy next SPEC-NNNN
--json` and workspace form `speccy next --json`) emits a top-level
`consistency` object in its JSON envelope. The object reports drift
between TASKS.md task state and git log task commits, computed by
matching commit titles against the regex
`^\[SPEC-NNNN/T-NNN\]: ` per task in the spec.

The object's shape:

```
"consistency": {
  "status": "ok" | "drift" | "blocked",
  "drifts": [
    {
      "task_id": "T-NNN",
      "kind": "<drift-kind enum>",
      "severity": "auto_fixable" | "blocking",
      "tasks_state": "<the task's current state in TASKS.md>",
      "details": { /* kind-specific fields */ }
    }
  ]
}
```

When `status` is `"ok"`, the `drifts` array is empty (or omitted).
When `status` is anything else, `drifts` is a non-empty array.
When `status != "ok"`, the envelope's `next_action.kind` field is
set to `"reconcile"`, overriding what the normal next-action logic
would otherwise emit. When `status == "ok"`, `next_action.kind`
reflects normal dispatch (`work`, `review`, `ship`, `decompose`, …).

The CLI performs the comparison deterministically — no LLM
judgment, no mutation. It reads TASKS.md, queries `git log --grep`
(or libgit2 equivalent) for matching commit titles, and emits the
result. The CLI never invokes `git commit`, `git add`, `git
restore`, or any mutating git command.

<done-when>
- `speccy next SPEC-NNNN --json` always includes a top-level
  `consistency` field in the JSON envelope.
- `consistency.status` is exactly one of the three values `"ok"`,
  `"drift"`, `"blocked"`.
- When `consistency.status != "ok"`, the JSON envelope's
  `next_action.kind` field is `"reconcile"`.
- When `consistency.status == "ok"`, `next_action.kind` reflects
  the normal dispatch (whatever the per-spec next-action logic
  would have emitted without the consistency layer).
- The CLI binary contains no calls to mutating git commands (`git
  add`, `git commit`, `git restore`, `git clean`, `git stash`)
  verified by source-level grep in `speccy-cli/src/` and the core
  crates.
</done-when>

<behavior>
- Given a SPEC where every task in `state="completed"` has a
  corresponding `[SPEC-NNNN/T-NNN]:`-prefixed commit in git log and
  every task in non-completed state has no such commit, when
  `speccy next SPEC-NNNN --json` runs, then the envelope's
  `consistency.status` is `"ok"`.
- Given a SPEC where T-001 is `state="completed"` in TASKS.md but
  no commit with title prefix `[SPEC-NNNN/T-001]:` exists in git
  log, when the same command runs, then `consistency.status` is
  `"blocked"`, the `drifts` array contains one entry with
  `kind="state_completed_no_commit"` and `severity="blocking"`,
  and `next_action.kind` is `"reconcile"`.
- Given a SPEC where T-002 is at `state="in-review"` in TASKS.md
  but a commit with title prefix `[SPEC-NNNN/T-002]:` exists in
  git log, when the same command runs, then `consistency.status`
  is `"drift"`, the `drifts` array contains one entry with
  `kind="commit_without_state"` and `severity="auto_fixable"`,
  and `next_action.kind` is `"reconcile"`.
</behavior>

<scenario id="CHK-009">
Given a SPEC-NNNN whose TASKS.md marks T-001 as
`state="completed"` and whose git log contains no commit whose
title begins with `[SPEC-NNNN/T-001]: `,
when `speccy next SPEC-NNNN --json` runs and exits 0,
then the JSON envelope contains a top-level field
`consistency.status` equal to `"blocked"`,
and `consistency.drifts` is a JSON array of length ≥ 1,
and at least one entry in `consistency.drifts` has fields
`task_id == "T-001"`, `kind == "state_completed_no_commit"`,
`severity == "blocking"`,
and the envelope's `next_action.kind` equals `"reconcile"`.
</scenario>


<scenario id="CHK-010">
Given a SPEC-NNNN whose TASKS.md marks T-002 as
`state="in-review"` and whose git log contains a commit whose
title begins with `[SPEC-NNNN/T-002]: `,
when `speccy next SPEC-NNNN --json` runs and exits 0,
then `consistency.status` equals `"drift"`,
and `consistency.drifts` contains an entry with `task_id ==
"T-002"`, `kind == "commit_without_state"`, `severity ==
"auto_fixable"`, and a `details.commit_sha` field carrying a
40-character hex string.
</scenario>


<scenario id="CHK-011">
Given the `speccy-cli` and `speccy-core` source trees after this
SPEC lands,
when grepped for the literal substrings `git add`, `git commit`,
`git restore`, `git clean`, or `git stash` (as command invocations,
not in comments or doc strings),
then zero matches are found in any source file under `src/`.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Drift `kind` enum covers four cases with deterministic detection

The `consistency.drifts[].kind` field is a string enum that covers
at minimum the following four values when their respective
conditions hold. Each kind carries a defined `severity` and a
defined `details` object shape.

- **`commit_without_state`** — A commit with title prefix
  `[SPEC-NNNN/T-NNN]:` exists in git log, but TASKS.md marks the
  task at any non-`completed` state. Severity: `auto_fixable`.
  Details: `{ "commit_sha": "<40-hex>", "commit_short_sha": "<8-hex>" }`.

- **`state_completed_no_commit`** — TASKS.md marks the task at
  `state="completed"` but no commit with the matching title prefix
  exists in git log. Severity: `blocking`. Details:
  `{ "expected_trailer": "[SPEC-NNNN/T-NNN]:", "working_tree_dirty": <bool> }`.
  The `working_tree_dirty` field is the result of `git status
  --porcelain` having any output; it disambiguates the recoverable
  case (dirty tree holds the lost commit's changes) from the true
  loss case (clean tree, work truly gone).

- **`state_in_progress_orphaned`** — TASKS.md marks the task at
  `state="in-progress"` with a dirty working tree and no matching
  commit (a crashed implementer pass). Severity: `blocking`.
  Details: `{ "working_tree_dirty": true, "dirty_files_count": <int> }`.

- **`state_in_progress_clean`** — TASKS.md marks the task at
  `state="in-progress"` with a clean working tree and no matching
  commit (a crashed implementer pass whose partial work was already
  discarded, or whose changes never reached disk). Severity:
  `blocking`. Details: `{ "working_tree_dirty": false }`. This kind
  exists so the reconcile pass owns the in-progress case
  autonomously without surfacing a user-facing fork from the
  orchestrator startup check (DEC-004).

- **`journal_xml_malformed`** — The per-task journal file
  (`.speccy/specs/NNNN-slug/journal/T-NNN.md`) contains XML that
  fails to parse against the closed-set journal grammar (unclosed
  element, mismatched tags, etc.). Severity: `blocking`. Details:
  `{ "journal_path": "<path>", "last_well_formed_byte_offset": <int> }`.

The CLI detects each kind via deterministic comparison: file reads,
regex matches, XML parse attempts, and `git log --grep` queries.
No LLM judgment, no heuristics that require natural-language
understanding.

The enum is extensible: future SPECs may add additional `kind`
values. The reconcile partial in REQ-008 documents the procedure
for adding a new kind (CLI detection + partial policy table
entry).

<done-when>
- All four documented kinds are emitted by `speccy next` when their
  conditions hold against test fixtures.
- Each emitted drift carries the documented `details` shape for its
  kind.
- The JSON schema is documented in `docs/ARCHITECTURE.md` alongside
  the existing `speccy next` envelope documentation.
- Adding a fifth kind to the enum requires changes only in the CLI
  detection code and the reconcile partial; no skill-body changes
  are required.
</done-when>

<behavior>
- Given a journal file `journal/T-005.md` whose last `<review>`
  element is unclosed, when `speccy next SPEC-NNNN --json` runs,
  then exactly one drift with `kind="journal_xml_malformed"` is
  emitted and `details.last_well_formed_byte_offset` carries the
  byte offset of the close of the most recent well-formed element.
- Given TASKS.md T-006 at `state="in-progress"` and `git status
  --porcelain` reports four modified files with no commit titled
  `[SPEC-NNNN/T-006]:`, when the command runs, then exactly one
  drift with `kind="state_in_progress_orphaned"` is emitted with
  `details.working_tree_dirty=true` and `details.dirty_files_count=4`.
</behavior>

<scenario id="CHK-012">
Given a test fixture workspace with TASKS.md T-001 at
`state="completed"`, no `[SPEC-NNNN/T-001]:` commit in git log,
and `git status --porcelain` reporting two modified files,
when `speccy next SPEC-NNNN --json` runs,
then the envelope's `consistency.drifts` contains exactly one
entry,
and that entry has `kind == "state_completed_no_commit"`,
`severity == "blocking"`,
and `details.working_tree_dirty == true`.
</scenario>


<scenario id="CHK-013">
Given a test fixture journal file whose body is
`<implementer>...</implementer>\n<review persona="business"`
(unclosed `<review>` tag),
when `speccy next SPEC-NNNN --json` runs,
then `consistency.drifts` contains an entry with
`kind == "journal_xml_malformed"`,
and `details.journal_path` matches the on-disk path of the
malformed file,
and `details.last_well_formed_byte_offset` is the byte offset of
the close of the `<implementer>` element.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Reconcile dispatch is autonomous, rollback-biased, and idempotent

When `speccy next` returns `next_action.kind = "reconcile"`, the
calling skill (one of `/speccy-orchestrate`, `/speccy-work`,
`/speccy-review`) dispatches to a reconcile pass that iterates the
`consistency.drifts[]` array and applies a fixed action per `kind`.
The pass has three properties:

a. **Autonomous.** The pass applies the action for each drift
   without prompting the user, without surfacing a fork ("re-commit
   or roll back?"), and without halting the orchestration loop. The
   policy table is exhaustive over the documented enum; an unknown
   `kind` is the only path that escalates (treated as `blocking`
   and surfaced to caller).

b. **Rollback-biased.** When recovery is ambiguous — most notably,
   `state_completed_no_commit` with a clean working tree, meaning
   the lost commit's content is truly unrecoverable — the policy
   prefers rolling backward (TASKS.md state reset to `in-review`,
   journal preserved as evidence for the next reviewer round) over
   any forward-recovery attempt that might guess at lost content.
   The orchestrator's per-task retry budget absorbs the redo cost.

c. **Idempotent.** Each policy action is a no-op when applied to
   already-converged state. Re-running `git add -A && git commit`
   on a clean tree produces no commit; re-running a TASKS.md state
   flip when the state is already at the target value is a no-op.
   Successive session crashes during reconciliation converge to
   the same eventual state.

The policy actions per kind:

| `kind` | Action |
|---|---|
| `commit_without_state` | Edit TASKS.md: flip task state to `completed` (deterministic write). |
| `state_completed_no_commit` (dirty tree) | `git add -A && git commit` using REQ-004 message format reconstructed from journal + TASKS.md. |
| `state_completed_no_commit` (clean tree) | Edit TASKS.md: roll task state back to `in-review`. Journal stays intact. |
| `state_in_progress_orphaned` | `git restore .`, `git clean -fd`, edit TASKS.md to flip state to `pending`. Discards partial implementer work. |
| `state_in_progress_clean` | Edit TASKS.md: roll task state back to `pending` (no git mutation — the tree is already clean, so there is no partial work to discard). Mirrors the rollback-bias property: when an in-progress task crashed without leaving uncommitted edits, assume the implementer made no non-discarded progress. |
| `journal_xml_malformed` | Truncate journal file to `details.last_well_formed_byte_offset` bytes. Reset corresponding TASKS.md state to whatever the truncated journal implies (e.g., if the last well-formed element is `<implementer>`, state goes to `in-review`; if a `<review>` block survived, the corresponding flip applies). |

After applying actions for all drifts, the skill re-queries `speccy
next`. If `consistency.status` is now `"ok"`, the skill resumes
its normal dispatch on the returned `next_action.kind`. If status
is still `"drift"` or `"blocked"`, the loop applies actions again.
The mechanism has no hard round budget; idempotency plus
re-detectability on subsequent sessions bounds the worst case to
"implementer reimplements the affected task once".

<done-when>
- A `state_completed_no_commit` drift with `details.working_tree_dirty=true`
  is resolved by the reconciler running `git add -A && git commit`
  with the REQ-004 message format; on re-query, the drift is gone.
- A `state_completed_no_commit` drift with `details.working_tree_dirty=false`
  is resolved by the reconciler editing TASKS.md to roll state back
  to `in-review`; on re-query, the drift is gone (the task is now
  awaiting a re-review).
- A `commit_without_state` drift is resolved by the reconciler
  editing TASKS.md to flip the task's state to `completed`; on
  re-query, the drift is gone.
- Running the reconcile pass against a workspace where `speccy next`
  already reports `consistency.status == "ok"` produces no file
  edits, no commits, and no observable state change.
- No code path in any skill body prompts the user or asks a question
  when `next_action.kind == "reconcile"` is dispatched (verified by
  reading the three skill body files and the inlined reconcile
  partial — no `AskUserQuestion` invocation, no "press enter to
  continue" surface).
</done-when>

<behavior>
- Given a session resumed after a mid-commit crash (TASKS.md T-001
  at `completed`, no matching commit, working tree dirty with the
  implementer's code + the TASKS.md edit + the journal append),
  when `/speccy-orchestrate SPEC-NNNN` starts and reaches its
  startup integrity check, then the reconciler dispatches, runs
  `git add -A && git commit` with the reconstructed message, re-
  queries `speccy next`, observes `consistency.status == "ok"`,
  and proceeds to dispatch the next pending task.
- Given the same setup but with a clean working tree (the crash
  destroyed the in-memory edits before they reached disk; TASKS.md
  state survived but code didn't), when the reconciler dispatches,
  then it edits TASKS.md to roll T-001 back to `in-review`, re-
  queries, sees the drift cleared, and proceeds to dispatch the
  reviewer fan-out for T-001 again.
- Given a fully-consistent workspace, when the reconcile dispatch
  runs (perhaps because a stale invocation triggered it), then no
  file edits occur and no commits are produced.
</behavior>

<scenario id="CHK-014">
Given a test fixture workspace with TASKS.md T-001 at
`state="completed"`, no matching commit in git log, and a working
tree dirty with the would-be commit's changes (code file edits,
TASKS.md edit, journal append),
when the reconcile dispatch runs in a controlled harness that
simulates the skill body's behavior,
then `git status --porcelain` exits with empty stdout after the
dispatch completes,
and `git log -1 --format='%s'` matches `^\[SPEC-NNNN/T-001\]: `,
and a re-query of `speccy next SPEC-NNNN --json` returns
`consistency.status == "ok"`.
</scenario>


<scenario id="CHK-015">
Given a test fixture workspace with TASKS.md T-002 at
`state="completed"`, no matching commit in git log, and a clean
working tree,
when the reconcile dispatch runs,
then TASKS.md afterwards shows T-002 at `state="in-review"`,
and the per-task journal `journal/T-002.md` is unchanged from
before the dispatch,
and a re-query of `speccy next SPEC-NNNN --json` returns
`consistency.status == "ok"` with `next_action.kind == "review"`
(the loop will re-dispatch reviewer fan-out on this task).
</scenario>


<scenario id="CHK-016">
Given a test fixture workspace where `speccy next SPEC-NNNN --json`
reports `consistency.status == "ok"`,
when the reconcile dispatch is run anyway (idempotency test),
then no commit is produced (`git log -1 --format='%H'` returns the
same SHA before and after),
and no file modifications occur (`git status --porcelain` exits
with empty stdout),
and a second re-query returns the same envelope as the first.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Reconcile policy ships as a shared markdown partial inlined into three skill bodies

The reconcile policy described in REQ-007 ships as a single source
of truth at `.claude/speccy-references/reconcile-policy.md` (mirroring
the existing convention for `.claude/skills/speccy-review/references/journal-review.md`
and `.claude/speccy-references/journal-blockers.md`). The partial is
manually inlined into three skill body files via the existing
shared-partial convention (the four-persona fan-out partial is the
reference pattern):

- `.claude/skills/speccy-orchestrate/SKILL.md` — in the startup
  integrity check and at the entry of each loop iteration.
- `.claude/skills/speccy-work/SKILL.md` — at the entry of the
  skill, combined with the clean-tree precondition from REQ-002.
- `.claude/skills/speccy-review/SKILL.md` — at the entry of the
  skill.

Each inlined site is bounded by marker comments identifying it as
the shared partial (`<!-- Shared partial: reconcile-policy. … -->`)
so future contributors know to re-sync all three sites when the
partial changes.

The pre-existing `assert_thin_stub_body` test in
`speccy-cli/tests/init.rs` (inherited from archived SPEC-0044/REQ-010)
counts every non-empty line in the three phase-worker SKILL.md bodies
against a `< 12` cap to guard against full-body prose leakage. The
marker-bounded inline section permitted by this REQ is an explicit,
auditable exemption to that cap: contributors can read the marker
comments, see the inline is the reconcile partial, and verify the
content equals `.claude/speccy-references/reconcile-policy.md`. The
test must therefore exclude lines between the open and close marker
comments from its non-empty-line count, so the marker-bounded inline
does not register as "full body has leaked".

The partial documents:

- The dispatch trigger (`next_action.kind == "reconcile"`).
- The four drift kinds from REQ-006 and their policy actions from
  REQ-007 as a table.
- The post-dispatch re-query and loop discipline (re-query until
  `status == "ok"`, then return to normal dispatch).
- The three properties (autonomous, rollback-biased, idempotent)
  and the procedure for adding a new drift kind.

<done-when>
- The file `.claude/speccy-references/reconcile-policy.md` exists
  and documents the policy per the points above.
- The text of the partial appears verbatim (whitespace-normalized)
  in the three skill body files, bounded by marker comments naming
  the partial.
- A diff between the partial file and each of the three inlined
  sections shows no semantic content drift.
- The partial documents the procedure for adding a new drift kind
  in a "Extending the enum" section.
- The `assert_thin_stub_body` test helper in
  `speccy-cli/tests/init.rs` excludes lines between the
  `<!-- Shared partial: reconcile-policy. … -->` open marker and
  the `<!-- End shared partial: reconcile-policy. -->` close marker
  from its non-empty-line count, so a SKILL.md body that contains
  the marker-bounded inline still passes the thin-stub assertion.
</done-when>

<behavior>
- Given the file `.claude/speccy-references/reconcile-policy.md`
  after this SPEC lands, when read, then it contains a Markdown
  table mapping each of the four drift kinds in REQ-006 to its
  policy action from REQ-007.
- Given the three skill body files (orchestrate, work, review)
  after this SPEC lands, when each is searched for the marker
  comment naming the reconcile partial, then exactly one match
  per file is found, and the content between the open and close
  markers matches the partial file's body.
</behavior>

<scenario id="CHK-017">
Given the file `.claude/speccy-references/reconcile-policy.md`
after this SPEC lands,
when grepped for each of the four drift kind names
(`commit_without_state`, `state_completed_no_commit`,
`state_in_progress_orphaned`, `journal_xml_malformed`),
then each name appears at least once in the file.
</scenario>


<scenario id="CHK-018">
Given the three skill body files
`.claude/skills/speccy-orchestrate/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`, and
`.claude/skills/speccy-review/SKILL.md` after this SPEC lands,
when each is searched for the open marker comment
`<!-- Shared partial: reconcile-policy.`,
then exactly one match is found in each file,
and the content between the open and close markers in each file
matches the body of
`.claude/speccy-references/reconcile-policy.md`
when normalized for surrounding whitespace.
</scenario>


<scenario id="CHK-019">
Given the `assert_thin_stub_body` test helper in
`speccy-cli/tests/init.rs` after this SPEC lands,
when its non-empty-line counting logic is read,
then lines that fall between the open marker comment
`<!-- Shared partial: reconcile-policy.` and the close marker
comment `<!-- End shared partial: reconcile-policy. -->` are
excluded from the count,
and the helper's `< 12 non-empty lines` assertion passes for a
phase-worker SKILL.md body that contains exactly the marker-bounded
inline plus the original thin-stub prose (agent path + invocation
pointer).
</scenario>

</requirement>

## Decisions

<decision id="DEC-001" status="accepted">
### DEC-001: CLI stays read-only re: git; never invokes mutating git commands

**Status:** Accepted

**Context:** Drift detection in REQ-005 requires reading git log to
correlate commits to tasks. Two architectural paths exist for the
"detect drift, then fix it" flow: (a) the CLI detects and emits
facts only, and skills apply fixes via Bash; (b) the CLI both
detects and applies fixes, exposing a `speccy reconcile`
subcommand that mutates TASKS.md and invokes `git commit` /
`git restore` itself.

**Decision:** Path (a). The Rust CLI reads git log (and the
filesystem) but issues no mutating git commands and writes no
mutations to TASKS.md as a side effect of `speccy next`. Skills,
running in host harnesses that already have Bash access and shell
tooling, apply fixes via the reconcile partial.

**Alternatives:**
- *Add `speccy reconcile` as a CLI subcommand that mutates state.*
  Rejected: drags substantial git operation logic into the
  deterministic core, including journal parsing for commit message
  reconstruction. Violates the "deterministic core, intelligent
  edges" principle in `AGENTS.md`. Skills have all the
  mechanically-derivable inputs already (the consistency block);
  the action is a small handful of shell commands per drift kind.
- *Have `speccy next` auto-mutate TASKS.md when it detects
  `commit_without_state` drift (deterministic write, no git
  needed).* Rejected: a query command that mutates state as a
  side effect is surprising; downstream consumers (other tools,
  test harnesses, the user reading raw output) cannot rely on
  `speccy next` being read-only.

**Consequences:** Adding a new drift kind requires changes in two
places (CLI detection + reconcile partial action), not one. The
duplication is small and explicit: the CLI knows what it
*detected*, the partial knows what to *do*. Future hosts (Codex,
others) that consume the consistency block reuse the partial,
keeping the action logic DRY across hosts.
</decision>

<decision id="DEC-002" status="accepted">
### DEC-002: Reconcile policy ships as a shared markdown partial, not as a new `/speccy-resume` skill

**Status:** Accepted

**Context:** The reconcile policy needs to be invocable from three
skill bodies (`/speccy-orchestrate`, `/speccy-work`,
`/speccy-review`) so resume detection fires regardless of which
entry point the user takes. Three distribution mechanisms exist:
(a) a new `/speccy-resume` skill that the three skills delegate
to; (b) a shared markdown partial inlined into each skill body
(matching the existing four-persona fan-out partial pattern); (c)
a new sub-agent (`reconciler`) that each skill spawns.

**Decision:** Path (b). The policy lives at
`.claude/speccy-references/reconcile-policy.md` and is inlined
verbatim into the three skill body files with marker comments
naming the shared contract.

**Alternatives:**
- *Add `/speccy-resume` as a new skill.* Rejected: sub-skills
  cannot be invoked from inside other sub-agent contexts in the
  current host harness (the same constraint that forces
  `speccy-review`'s persona fan-out to run inline in
  `/speccy-orchestrate` rather than via a wrapper sub-agent).
  Adding `/speccy-resume` would re-introduce the wrapper problem
  the fan-out partial was created to solve.
- *Spawn a `reconciler` sub-agent from each skill body.*
  Rejected: the reconcile actions are deterministic shell commands
  and file edits; spawning a sub-agent for them spends context
  budget on transcript overhead for work that needs no model
  judgment. Sub-agents are valuable for fresh-context
  adversarial review; they are wasteful for mechanical execution.

**Consequences:** Editing the partial requires re-syncing three
inlined copies (manually, since the host harness does not run a
build step that auto-inlines). The four-persona fan-out partial
already pays this cost; this SPEC's mechanism extends the same
pattern rather than introducing a new one. Future tooling could
add a build step that automates the sync; that work is out of
scope here.
</decision>

<decision id="DEC-003" status="accepted">
### DEC-003: Git log is the source of truth for "task durably completed"; TASKS.md is a reconcilable working view

**Status:** Accepted

**Context:** A per-task commit serves as the durable record that a
task is done. TASKS.md state edits and journal appends happen
inside the same atomic commit, but a session crash can leave
TASKS.md edits in the working tree without a matching commit (or,
less commonly, a commit without a corresponding TASKS.md edit).
The reconciliation policy must pick a tiebreaker for which signal
is authoritative when the two disagree.

**Decision:** Git log is the authoritative source. When TASKS.md
disagrees with git log, the reconciler reconciles TASKS.md toward
git log: if a commit exists but TASKS.md hasn't advanced, the
state flips forward; if TASKS.md has advanced but no commit
exists, the state rolls back (because there's no proof the work
landed durably).

**Alternatives:**
- *TASKS.md is authoritative; treat the absence of a commit as
  evidence the commit step crashed and the running session
  re-fires it.* Rejected: this opens the door to silent
  divergence between the journal/state files and the actual code
  in git. A session that crashes after editing TASKS.md but
  before staging code changes would, under this rule, fire a
  commit containing only the state files — committing a
  "completed task" with no code change. The reverse direction
  (git log authoritative) catches this case as
  `state_completed_no_commit` with `working_tree_dirty=true`,
  which re-fires the commit including the code.
- *Last-write-wins by mtime.* Rejected: filesystem timestamps are
  unreliable across filesystem boundaries (network drives, CI
  caches) and offer no semantic guarantee about which write was
  "intentional".

**Consequences:** Any change to the commit step's ordering
(journal append → TASKS.md flip → commit) needs to preserve the
property that the commit either captures all three changes or
none of them. Re-ordering to commit before state-flip would
invert the tiebreaker premise. Future SPECs touching the commit
sequence must keep "commit captures everything atomically" as an
invariant.
</decision>

<decision id="DEC-004" status="accepted">
### DEC-004: Reconcile is autonomous and rollback-biased; never surfaces a fork to the user during the loop

**Status:** Accepted

**Context:** Drift recovery has cases where the right action is
ambiguous: most notably, `state_completed_no_commit` with a clean
working tree could indicate either (i) work truly lost and the
task needs to be redone, or (ii) a commit that exists on a
different branch the user temporarily checked out. The policy
must pick one default; the question is whether to pick it
autonomously or surface the ambiguity to the user.

**Decision:** Autonomous, rollback-biased. The policy always
picks the safer-for-correctness option: when ambiguous, roll
backward (TASKS.md reverts to `in-review`, journal preserved as
evidence, loop redoes the work). No prompts, no forks during the
orchestration loop.

**Alternatives:**
- *Surface ambiguous cases to the user with a multiple-choice
  prompt (re-commit / roll back / skip).* Rejected: defeats the
  autonomy goal. A multi-agent harness running speccy across many
  specs cannot afford a per-spec human bottleneck on recovery
  decisions; the whole point of resume is that the next session
  picks up without intervention.
- *Roll forward (attempt to reconstruct the commit even with a
  clean tree).* Rejected: there's no source of truth for what
  the lost commit's contents were. Guessing risks committing
  stale content (e.g., the journal's `<implementer>` block) as
  if it were code, producing a commit that doesn't match the
  task's intent.

**Consequences:** A small class of recovery cases (truly-lost
work) costs one implementer re-run rather than zero. The
orchestrator's existing per-task retry budget (5 rounds) bounds
the total redo cost. The principle generalises: future drift
kinds added to the enum should follow the same rule — when in
doubt, roll backward and let the loop redo.
</decision>

## Assumptions

<assumptions>
- The commit body comes from mechanical extraction of the
  `Completed` field from the latest `<implementer>` block in
  `journal/T-NNN.md`. No new journal grammar attribute is added —
  the existing six-field handoff template (documented in
  `.claude/skills/speccy-work/references/journal-implementer.md`)
  carries the field already.
- The "short summary" portion of the commit title comes from the
  task title in TASKS.md, not the per-round implementer summary.
  Task title is stable across implementer rounds; round-specific
  prose lands in the body.
- Per-task commits compose with `speccy-vet`. Vet diffs against
  `<base-ref>` (branch base), so its capture continues to reflect
  the full branch delta whether changes are committed or staged.
  Vet's stash / restore continues to operate only on its own
  round-scoped scratch.
- The host harness preserves git config (signing, identity) across
  sessions; auto-commit does not set up signing itself. If signing
  is required and unavailable, the pre-commit hook fails and
  REQ-001's hygiene gate catches it before review fires the commit.
- The host harness exposes the current model identifier to skill
  bodies at runtime (environment variable, runtime API, or
  equivalent — varies per host). The skill queries this at commit
  time for the `Co-Authored-By` trailer. Hosts that do not expose
  one use the documented fallback string.
- The skill body does not check the current git branch. It trusts
  the user or orchestrator host to have placed the working tree on
  a feature branch off `main`. Commits land on whatever HEAD is;
  the user / host opens a PR via `/speccy-ship` for merge.
- `speccy archive` does not touch git history; commit history is
  immutable across archive. Archived SPECs retain their per-task
  commit trail in git even though their working files move under
  the archive directory.
- The reconcile pass has no hard round budget. Idempotent actions
  plus re-detectable drift on the next session are sufficient —
  worst case is the implementer re-implementing a task from
  scratch, which costs one orchestrate retry slot.
- Squash-merge of the SPEC's PR is out of scope for in-flight
  orchestration. Title-prefix grep survives `git rebase`. A user
  squashing on merge uses the PR title (which discards individual
  commit titles), but that happens post-ship, when the consistency
  check no longer runs on the spec.
- The shared-partial inlining convention (manually duplicated text
  with marker comments) stays as the distribution mechanism. If
  skills later move to a programmatic include, the reconcile
  partial follows uniformly.
- The reviewer fan-out remains the boundary where "task is durably
  complete" is decided. Resume's git-log-as-source-of-truth model
  presumes this.
</assumptions>

## Notes

The brainstorm session preceding this SPEC walked through three
framings before settling on the chosen one:

1. **Commit per task on review pass (the chosen path).** Atomic
   commits give per-task save points; resume is the consistency
   check + autonomous reconcile. Captured by REQ-001 through
   REQ-008.

2. **Commit at spec-end-of-orchestrate.** Considered and rejected:
   loses per-task atomicity, defeats save-point resume entirely,
   and standalone `/speccy-review` has no end-of-spec moment to
   fire on — breaking the symmetry between orchestrate and
   standalone review invocations.

3. **Per-task git worktrees.** Each task in its own isolated
   worktree, merged at spec end. Considered and rejected:
   heavyweight for v1, requires substantial orchestration
   redesign, and doesn't compose with `speccy-vet`'s existing
   snapshot / restore mechanic that assumes a single working
   tree.

The motivation for resume-as-first-class came from imagining the
multi-agent harness future for speccy. The shipped v1 skill packs
are already meant to drive the loop unattended; making them
crash-safe is the smallest delta that turns "useful for my next
greenfield" into "useful for a fleet of autonomous agents driving
spec implementation across many projects". The same mechanism
that lets a solo developer resume after a laptop reboot lets a
harness recover from a transient cloud-instance preemption.

The decision to keep reconciliation as a shared markdown partial
rather than promoting it to a `/speccy-resume` skill or a `speccy
reconcile` CLI subcommand traces back to the "stay small" core
principle in `AGENTS.md`. Every new skill or CLI command is a new
surface to maintain; this work needs neither. The four-persona
fan-out partial already proved the inlining pattern; this SPEC
extends it to one more shared contract.

## Open Questions

(None — brainstorm closed with no open questions remaining.)

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-25 | human/kevin  | Initial draft from approved brainstorm framing. Introduces eight requirements covering: (REQ-001) implementer hygiene gate before in-review flip; (REQ-002) clean-tree precondition at `/speccy-work` and orchestrator work-dispatch entry; (REQ-003) conditional atomic commit after review pass with skip-on-clean-tree idempotency; (REQ-004) commit message format with `[SPEC-NNNN/T-NNN]:` title prefix, `Completed`-field body extraction, and `Co-Authored-By` trailer; (REQ-005) `consistency` block in `speccy next` JSON envelope with `next_action.kind = "reconcile"` when drift detected; (REQ-006) drift kind enum covering `commit_without_state`, `state_completed_no_commit`, `state_in_progress_orphaned`, `journal_xml_malformed`; (REQ-007) autonomous, rollback-biased, idempotent reconcile dispatch with per-kind policy table; (REQ-008) reconcile policy ships as shared markdown partial at `.claude/speccy-references/reconcile-policy.md` inlined into three skill bodies. Four DECs codify: (DEC-001) CLI stays read-only re: git, (DEC-002) shared partial distribution chosen over new `/speccy-resume` skill, (DEC-003) git log is source of truth for task durability, (DEC-004) reconcile is autonomous and rollback-biased. |
| 2026-05-25 | human/kevin  | Amend REQ-006 to extend the drift `kind` enum with a fifth value `state_in_progress_clean` (severity `blocking`; details `{ "working_tree_dirty": false }`) covering the case where TASKS.md marks a task `in-progress` with a clean working tree and no matching commit. Amend REQ-007's policy table with a row mapping the new kind to a deterministic TASKS.md state roll-back to `pending`. Delete the orchestrate startup user-fork prose that previously surfaced this case as a user-facing fork — the reconcile-pass dispatch (added in T-004) now owns the in-progress case autonomously, satisfying REQ-007's autonomy property and DEC-004 ("Reconcile is autonomous and rollback-biased; never surfaces a fork to the user during the loop"). Origin: holistic drift review on the pre-ship boundary surfaced that the legacy orchestrate step 2 ("autonomy-breaking recovery path on purpose") coexisted with the new reconcile dispatch and contradicted the Non-goal "No fallback path that surfaces a drift fork to the user during the orchestration loop". |
| 2026-05-25 | human/kevin  | Amend REQ-008 to require loosening `assert_thin_stub_body` in `speccy-cli/tests/init.rs` (inherited from archived SPEC-0044/REQ-010) so the marker-bounded reconcile-policy inline section is exempted from the `< 12 non-empty lines` cap on phase-worker SKILL.md bodies. T-002's implementer attempt surfaced that the unmodified test makes the SKILL.md inlines mandated by REQ-008 (CHK-018) structurally impossible — any inline of the ~95-line partial trips the leakage guard. The test guards a real concern (full prose leaking back into stubs) so the resolution is to scope the exemption narrowly to the audited marker-bounded section, not to remove the cap. Adds CHK-019 to verify the exemption is mechanical. Extends T-002 task body with a step 0 that performs this test edit before the three SKILL.md additions land. T-002 journal `<blockers>` from the prior attempt is superseded by this amendment and the journal file is removed (the task returns to a clean-slate pending state). |
</changelog>
