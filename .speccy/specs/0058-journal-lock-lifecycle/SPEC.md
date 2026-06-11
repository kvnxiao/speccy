---
id: SPEC-0058
slug: journal-lock-lifecycle
title: Journal lock-file lifecycle — reap advisory lock sidecars at terminal lifecycle boundaries
status: in-progress
created: 2026-06-10
supersedes: []
---

# SPEC-0058: Journal lock-file lifecycle — reap advisory lock sidecars at terminal lifecycle boundaries

## Summary

`speccy journal append` serializes concurrent appenders with an advisory
file lock (SPEC-0055 REQ-005, DEC-007). The lock is taken on a *sidecar*
file beside the journal — `<spec-dir>/journal/<task-id>.md.lock` for task
blocks (`speccy-cli/src/journal.rs:349`) and `<spec-dir>/journal/VET.md.lock`
for vet blocks (`journal.rs:413`). `LockGuard::acquire` opens that path with
`create(true)` (`journal.rs:657-667`), but `Drop for LockGuard`
(`journal.rs:695-703`) only calls `FileExt::unlock`; it never unlinks the
sidecar. Every append therefore leaves an empty `.lock` file behind, and
because appends land across every spec's journal, the working tree
accumulates `.lock` traces scattered through `.speccy/specs/*/journal/`
directories.

The sidecars are kept out of git by the repo `.gitignore` (`*.md.lock`,
`.gitignore:20`), so this is a working-tree-cleanliness problem, not a
commit-hygiene one. Deleting a held advisory lock *at release* is unsafe:
it reintroduces the classic unlink-while-locked race that would break the
SPEC-0055 mutual-exclusion contract (a waiter locks the orphaned inode
after the holder unlinks, while a third appender's `create(true)` makes a
fresh inode at the same path and locks that — two appenders then hold "the"
lock). That is why `Drop` deliberately never unlinks (see DEC-001).

Deletion *is* safe at a **quiescent boundary**: a point the orchestrator
guarantees has no in-flight append and no waiter. The orchestrator runs
tasks serially, and the only real concurrency anywhere is the per-task
review fan-out against an already-existing task journal (the implementer
creates the journal solo; vet appends are sequential). Two terminal
quiescent boundaries therefore exist — a task reaching `completed`, and the
vet `<gate>` block (the terminal vet write, appended on every exit path).
This SPEC **reaps** each lock sidecar at its terminal lifecycle boundary,
folding the deletion into the commands that already own those boundaries —
`speccy task transition --to completed` for task locks and `speccy journal
append --block gate` for the vet lock — via a shared, guarded, idempotent
internal helper, with no new CLI subcommand. The append/release mechanism,
the sidecar path, and `Drop` are all unchanged; only an explicit terminal
reap is added.

## Goals

<goals>
- After a task transitions to `completed`, its `journal/` directory
  contains the task's journal `.md` and no `<task-id>.md.lock`.
- After the vet `<gate>` block is appended, the spec's `journal/`
  directory contains `VET.md` and no `VET.md.lock`.
- The reap deletes only a currently-unheld lock and is idempotent, so a
  mistimed or repeated reap is a safe no-op rather than a corruption.
- Every SPEC-0055 REQ-005 concurrent-appender guarantee (mutual exclusion
  across the derive→validate→write critical section, the 10s timeout
  leaving the journal byte-identical, locking working before the journal
  `.md` exists) holds unchanged: `Drop` still never unlinks and the
  append-time lock path is unmoved.
</goals>

## Non-goals

<non-goals>
- No unlink at lock *release*. `Drop for LockGuard` stays unlock-only;
  reaping is a separate explicit step at a terminal lifecycle boundary,
  never inside the append critical section or its `Drop` (see DEC-001).
- No new CLI subcommand. Reaping folds into `task transition` and `journal
  append --block gate`; it is not a user-facing verb (see DEC-002).
- No change to the lock path, timeout, poll interval, or advisory
  same-host semantics. SPEC-0055 DEC-002 (10s timeout) and DEC-007
  (advisory, per-target, same-host) stay exactly as shipped; the sidecar
  stays beside the journal.
- No distributed or cross-host locking.
- No change to journal `.md` contents, frontmatter, the append grammar, or
  any `speccy journal` subcommand surface other than the terminal reap on
  the `gate` block.
</non-goals>

## User Stories

<user-stories>
- As a speccy contributor, I want a completed task's `journal/` directory
  to hold only its journal `.md` — no `.lock` sidecar — so my working tree
  is not littered with lock files as the loop advances task by task.
- As a maintainer, I want every lock sidecar gone once its journal's
  lifecycle is over (task completed, or spec vetted), so a finished spec
  carries zero runtime lock artifacts.
</user-stories>

## Assumptions

<assumptions>
- The orchestrator runs tasks strictly serially, so a task reaching
  `completed` is a quiescent boundary for that task's journal: the review
  fan-out has settled, no appender is in flight, and the next task targets
  a different journal. If tasks ever ran in parallel, a per-task reap would
  reintroduce the release-time race.
- The only concurrent appends anywhere are the per-task review personas
  writing to one already-existing task journal; vet appends are sequential
  (the vet loop spawns one sub-agent at a time) and the `<gate>` is the
  terminal vet write, appended on every vet exit path. So the gate append
  is a guaranteed quiescent boundary for `VET.md`.
- The last task reaches `completed` (and its lock is reaped) before vet
  begins, so `VET.md.lock` — created during vet — has no task-completion
  edge to attach to and can only be reaped at a vet-or-later boundary,
  hence the gate.
- A non-blocking `try_lock` immediately before unlink makes the reap a safe
  no-op when the lock is unexpectedly held; the real safety contract is the
  terminal-boundary quiescence, not the guard alone (a guard cannot beat an
  appender arriving during the unlink window, which the serial orchestrator
  never produces).
- The sidecar path is unchanged, so the existing `*.md.lock` `.gitignore`
  pattern (`.gitignore:20`) keeps transient mid-run sidecars untracked —
  including across the per-task `git add -A` commit on review pass.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: A completed task's lock sidecar is reaped by the `--to completed` transition

`speccy task transition SPEC-NNNN/T-NNN --to completed` deletes the task
journal's lock sidecar (`<spec-dir>/journal/<task-id>.md.lock`) after it
performs the TASKS.md state rewrite. Only the `--to completed` edge reaps;
no other transition edge touches the sidecar. The journal `.md` itself is
never touched.

<done-when>
- After a `--to completed` transition for a task whose lock sidecar
  exists, the sidecar no longer exists and the journal `.md` is unchanged.
- After a transition to any state other than `completed`, an existing task
  lock sidecar is left in place.
- A `--to completed` transition for a task with no lock sidecar exits zero
  (idempotent no-op reap).
</done-when>

<behavior>
- Given a task with a `<task-id>.md.lock` sidecar present, when it
  transitions to `completed`, then the sidecar is gone and the journal
  `.md` is byte-identical.
- Given a task transitioning `in-review` → `pending` on a blocking round,
  when the transition runs, then any existing lock sidecar persists (the
  journal is not yet at a terminal boundary).
</behavior>

<scenario id="CHK-001">
Given a task whose journal `.md` and `<task-id>.md.lock` both exist,
when `speccy task transition SPEC-NNNN/T-NNN --to completed` runs,
then `<task-id>.md.lock` no longer exists and `<task-id>.md` is
byte-identical to its pre-transition state.
</scenario>

<scenario id="CHK-002">
Given a task with a journal but no lock sidecar,
when it transitions to `completed`,
then the command exits zero and the journal is unchanged.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: The vet lock sidecar is reaped by the terminal `<gate>` append

`speccy journal append SPEC-NNNN --block gate` deletes `VET.md.lock` after
the gate block is written and the append's own lock has been released. The
reap fires only for the `gate` block — the terminal vet write — and for no
other vet or task block.

<done-when>
- After a `--block gate` append, `VET.md.lock` does not exist and `VET.md`
  is intact.
- After a non-gate vet append (`drift-review`, `holistic-fix`,
  `simplifier-scan`, `simplifier-apply`), the `VET.md.lock` it used still
  exists for the next sequential appender.
- A `--block gate` append whose `VET.md.lock` is already absent exits zero
  (idempotent).
</done-when>

<behavior>
- Given a vet invocation whose prior blocks have landed (so `VET.md.lock`
  exists), when the terminal `<gate>` block is appended, then `VET.md.lock`
  is gone and `VET.md` parses cleanly with the gate as its last element.
- Given a non-gate vet append, when it returns, then `VET.md.lock`
  persists.
</behavior>

<scenario id="CHK-003">
Given a spec whose `VET.md` and `VET.md.lock` exist from prior vet blocks,
when `speccy journal append SPEC-NNNN --block gate` runs,
then `VET.md.lock` no longer exists and `VET.md` parses cleanly with the
gate as its last element.
</scenario>

<scenario id="CHK-004">
Given a vet `drift-review` append to a spec,
when it completes,
then `VET.md.lock` still exists (only the `gate` block reaps).
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: The reap skips a currently-held lock via a `try_lock` guard

The reap deletes a lock sidecar only when the lock is currently free,
verified by a non-blocking `try_lock` immediately before the unlink; a held
lock is skipped, never unlinked. This makes a mistimed reap a safe no-op
rather than a mutual-exclusion break.

<done-when>
- A reap invoked while the lock is held by another handle leaves the
  sidecar in place.
- A reap whose `try_lock` succeeds proceeds to unlink the sidecar.
</done-when>

<behavior>
- Given a lock sidecar held by another open handle, when a reap runs
  against it, then the `try_lock` fails and the sidecar is left intact.
- Given a free lock sidecar at a terminal boundary, when the reap runs,
  then the `try_lock` succeeds and the sidecar is removed.
</behavior>

<scenario id="CHK-005">
Given a test holding an exclusive lock on a task journal's
`<task-id>.md.lock`,
when a `speccy task transition SPEC-NNNN/T-NNN --to completed` runs its
reap,
then the sidecar still exists afterward (the guard skipped the held lock)
and the transition itself exits zero.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: The append-time mutual-exclusion contract is preserved

`Drop for LockGuard` is unchanged — it never unlinks — and the append-time
lock path is unmoved, so no append ever deletes a lock at release and every
SPEC-0055 REQ-005 guarantee holds verbatim. The reap is additive; it does
not alter how the lock is acquired, held, or released during an append.

<done-when>
- The eight-concurrent-append and two-concurrent-round-opening behaviors
  from SPEC-0055 REQ-005 still hold unchanged.
- A held lock past the 10s timeout still makes a waiting append exit
  non-zero with the journal byte-identical and the diagnostic naming the
  journal path.
</done-when>

<behavior>
- Given eight processes appending distinct review blocks to one journal
  concurrently, when they run, then exactly eight well-formed blocks are
  present and the file parses cleanly.
- Given an externally held lock past the timeout, when another append
  waits, then it exits non-zero with the journal byte-identical and stderr
  naming the journal path.
</behavior>

<scenario id="CHK-006">
Given a fresh journal and eight processes each appending one distinct
review block to it concurrently,
when all processes complete,
then the journal parses cleanly and contains exactly eight review blocks.
</scenario>

<scenario id="CHK-007">
Given a test holding an exclusive lock on a task journal's lock path,
when a `speccy journal append` to that journal waits past the 10s timeout,
then the append exits non-zero, stderr names the journal path, and the
journal file is byte-identical to its pre-append state.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
Reap at a quiescent boundary; never unlink at release. Unlinking a held
advisory lock reintroduces the classic unlink-while-locked race that
SPEC-0055's contract cannot tolerate. On POSIX, a waiter that already
opened the lock path locks the now-orphaned inode after the holder unlinks,
while a third appender's `create(true)` makes a fresh inode at the same
path and locks that — two appenders then hold "the" lock simultaneously,
breaking REQ-005 mutual exclusion. On Windows (a supported host),
`remove_file` on a lock file with an open handle fails with a sharing
violation, so deletion would error mid-critical-section. Therefore `Drop
for LockGuard` stays unlock-only, exactly as shipped, and deletion happens
only where the orchestrator guarantees no in-flight append and no waiter —
a quiescent boundary. The serial task loop and the sequential vet flow
provide two such boundaries (a task reaching `completed`; the terminal
`<gate>` write), and the reap runs only there. This is what makes "delete
the lock files" safe, where the original never-delete posture — and the
superseded relocate-and-persist framing — avoided the race instead of
resolving it.
</decision>

<decision id="DEC-002">
Fold the reap into the commands that own each terminal boundary, adding no
CLI subcommand. The task lock is reaped by `speccy task transition --to
completed` after its TASKS.md state rewrite; the vet lock by `speccy
journal append --block gate` after the gate is written and the append's own
lock releases. A shared internal helper performs the guarded, idempotent
unlink for both call sites. This was preferred over (a) a dedicated `speccy
journal sweep` verb — rejected to keep the command surface small (Core
principle 5, "stay small"); (b) folding deletion onto `speccy verify` —
rejected because `verify` is the read-only CI gate and must never mutate
the working tree; and (c) relying on `speccy archive` — rejected because
archiving is optional and cannot guarantee the vet lock is reaped. The lock
is born in `journal append` and dies in `journal append --block gate` /
`task transition`, keeping its whole lifecycle inside the commands that own
it.
</decision>

<decision id="DEC-003">
The vet lock is reaped at the gate, not at a task transition, because the
last task reaches `completed` (and its lock is reaped) before vet begins —
so `VET.md.lock`, created during vet, has no task-completion edge to attach
to. The `<gate>` is the terminal vet write and Phase 3 of the vet loop
appends exactly one on every exit path, so the gate hook fires whenever a
vet run completes. The "lock works before the journal exists" property (the
original reason the lock was a sidecar rather than the journal `.md`
itself) is untouched: the sidecar path and `LockGuard::acquire`'s
`create(true)` are unchanged; reaping only removes the sidecar after the
journal's lifecycle is over, and a later re-vet recreates it via the same
`create(true)`.
</decision>

<decision id="DEC-004">
The reap helper verifies the lock is free, then **closes its handle before
unlinking**. The shared `reap_lock_sidecar` helper opens the sidecar
*without* `create(true)` — an absent sidecar is the idempotent no-op, never a
freshly created inode — takes one non-blocking `try_lock`, and only on
success drops the file handle *before* calling `remove_file`. REQ-003's
"`try_lock` immediately before the unlink" verifies the lock is free; it does
not mean holding the lock across the unlink. On Windows (a supported host)
`remove_file` against a path with a live open handle fails with a sharing
violation (see DEC-001), so unlinking while still holding the handle would
silently no-op on Windows while succeeding on POSIX. Closing first releases
both the advisory lock and the OS handle, so the unlink behaves identically
on every host. The brief window between release and unlink is safe because
the real safety contract is terminal-boundary quiescence (DEC-001, REQ-003),
not the `try_lock` alone — the serial orchestrator never produces an appender
inside that window, and the `try_lock` only guards a mistimed reap. The
helper is infallible (returns no `Result`): it runs only after the command's
load-bearing mutation (the TASKS.md state rewrite or the gate append) has
already succeeded, so any I/O error short of a clean reap leaves the sidecar
in place and the command still exits zero, matching the unlock-only,
"not actionable on failure" posture of `Drop for LockGuard`.
</decision>

## Notes

The three SPEC-0055 concurrency tests are mechanism-agnostic and the
append-time lock path is unmoved, so they continue to pass unchanged and
are the regression signal that reaping did not touch the append contract:
`eight_concurrent_review_appends_serialize_cleanly`,
`two_concurrent_round_opening_appends_get_distinct_rounds`, and
`held_lock_makes_waiting_append_time_out_nonzero`
(`speccy-cli/tests/journal_append.rs`). Unlike the superseded relocate
framing, no existing test's hardcoded lock path needs updating, because the
sidecar stays beside the journal.

The existing `*.md.lock` line in `.gitignore` still covers the sidecars
while they transiently exist mid-run; after each terminal boundary the reap
removes them, so a shipped spec carries none. The `.speccy/locks/` ignore
entry contemplated by the prior framing is no longer relevant — that
relocation is dropped.

Rejected framings (from brainstorm): relocate-and-persist into
`.speccy/locks/` (never reaches zero lock files — only relocates the pile);
lock the journal `.md` directly (eliminates the sidecar but restructures
the append I/O, adds an empty-file-on-failed-first-append edge, and drops
the defensive concurrent-first-create guarantees); per-writer
create-take-remove (the unlink-while-locked race itself).

## Open Questions

<!-- alpha-prefix ordinals; preserve existing, allocate next free letter -->

- [x] a. (Resolved by this amendment.) The original ask was to "cleanup
  lock files after each task is done." This is now implemented as true
  deletion at terminal lifecycle boundaries — a task reaching `completed`
  and the vet `<gate>` — rather than the relocate-and-persist reading the
  SPEC first adopted. True deletion is safe here because the serial
  orchestrator makes those boundaries quiescent (see DEC-001); no
  documented race-mitigation cost is incurred because nothing is deleted at
  lock release.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-10 | kevin | Initial SPEC: relocate journal advisory-lock sidecars out of `journal/` into `.speccy/locks/`; deterministic collision-free mapping; never-delete decision; SPEC-0055 contract preserved. |
| 2026-06-11 | claude | Decomposition — added DEC-004 capturing the reap helper's cross-platform delete ordering (verify-free via `try_lock`, close the handle, then `remove_file`, because Windows rejects unlinking a path with a live open handle) and its infallible post-mutation posture. No requirement change. |
| 2026-06-11 | kevin | Amendment — reframed from relocate-and-persist to **delete at terminal lifecycle boundaries**. Brainstorm established that no journal is ever created concurrently (the implementer creates each task journal solo; reviews require a pre-existing round; vet appends are sequential), so task-`completed` and the terminal `<gate>` are quiescent boundaries where unlinking is safe. Reaping folds into `task transition --to completed` (task lock) and `journal append --block gate` (vet lock) via a guarded, idempotent internal helper — no new subcommand (rejected `journal sweep`; `verify` is read-only; `archive` is optional). Inverts the prior "No deletion" non-goal and DEC-001; drops the `.speccy/locks/` relocation (old DEC-002/003). `Drop` stays unlock-only and the append-time lock path is unmoved, so SPEC-0055 REQ-005 holds and its three concurrency tests pass unchanged. Resolved OQ-a. |
</changelog>
