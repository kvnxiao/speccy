---
id: SPEC-0058
slug: journal-lock-lifecycle
title: Journal lock-file lifecycle — relocate advisory lock sidecars out of journal directories
status: in-progress
created: 2026-06-10
supersedes: []
---

# SPEC-0058: Journal lock-file lifecycle — relocate advisory lock sidecars out of journal directories

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

The sidecars are already kept out of git by the repo `.gitignore`
(`*.md.lock`, `.gitignore:19-20`), so this is a working-tree-cleanliness
problem, not a commit-hygiene one. The naive fix — unlink the sidecar on
release — is unsafe: deleting a held advisory lock reintroduces the
classic unlink-while-locked race that would break the SPEC-0055 mutual
exclusion contract (see DEC-001), and the current `Drop` deliberately
sidesteps it by never unlinking.

This SPEC relocates the advisory lock sidecars out of the journal
directories into a single dedicated runtime locks directory under
`.speccy/locks/`, derived by a stable collision-free mapping from each
journal target. Journal directories then contain only journal `.md`
files. The lock files persist by design (they are never deleted), so
every SPEC-0055 concurrent-appender guarantee is preserved unchanged;
only the lock's *location* moves.

## Goals

<goals>
- After any `speccy journal append`, the target's `journal/` directory
  contains its journal `.md` and no `*.lock` file.
- All advisory lock artifacts live under a single dedicated runtime
  directory at `.speccy/locks/`, derived deterministically from the
  journal target.
- Every SPEC-0055 REQ-005 concurrent-appender guarantee (mutual
  exclusion across the derive→validate→write critical section, the 10s
  timeout leaving the journal byte-identical, locking working before the
  journal `.md` exists) holds under the relocated lock.
</goals>

## Non-goals

<non-goals>
- No deletion of lock files. Lock files are not unlinked on release;
  "cleanup" means relocating them out of the journal directories, not
  removing them (see DEC-001). Safe per-append deletion is not
  achievable without weakening mutual exclusion.
- No change to the lock timeout, poll interval, or advisory same-host
  semantics. SPEC-0055 DEC-002 (10s timeout) and DEC-007 (advisory,
  per-target, same-host) stay exactly as shipped.
- No distributed or cross-host locking.
- No change to journal `.md` contents, frontmatter, the append grammar,
  or any `speccy journal` subcommand surface other than where the lock
  file is placed.
</non-goals>

## User Stories

<user-stories>
- As a speccy contributor, I want `journal/` directories to hold only
  journal `.md` files after appends, so my working tree is not littered
  with `.lock` sidecars scattered across every spec.
- As a maintainer debugging lock contention, I want every lock artifact
  in one predictable, ignored location, so I can inspect or clear them
  without hunting through each spec's journal directory.
</user-stories>

## Assumptions

<assumptions>
- A lock file must persist at a stable path shared across concurrent
  appender *processes* to provide mutual exclusion; a per-process or
  deleted-after-use lock cannot serialize separate `speccy journal
  append` invocations. So "cleanup" can only mean relocation, never
  deletion.
- The project root (the directory containing `.speccy/`) is reachable
  via the same `find_root` walk-up the append path already performs
  (`journal.rs:277-281`), so `.speccy/locks/` is derivable wherever an
  append runs.
- `fs4` advisory locks are keyed by path: two distinct lock paths never
  serialize against each other, and the same path always does. A
  collision-free mapping from journal target to lock path is therefore
  necessary and sufficient to preserve serialization.
- Retaining the `.md.lock` filename suffix keeps the existing
  `*.md.lock` `.gitignore` pattern (`.gitignore:19-20`) matching the
  relocated files, so they stay untracked without a new ignore rule.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: Lock sidecars are placed under `.speccy/locks/`, never in `journal/`

The advisory lock for an append is acquired on a file under a single
workspace-level locks directory rooted at `.speccy/locks/`, instead of a
sidecar inside the journal directory. After any append — whether it
succeeds, fails validation, or times out — the target's `journal/`
directory contains no `*.lock` file.

<done-when>
- After a task-block append, `<spec-dir>/journal/` contains
  `<task-id>.md` and no `<task-id>.md.lock`.
- After a vet-block append, `<spec-dir>/journal/` contains `VET.md` and
  no `VET.md.lock`.
- After either append returns, the lock file it used exists under
  `.speccy/locks/`.
</done-when>

<behavior>
- Given a task-block append that succeeds, when the command returns,
  then the lock artifact resides under `.speccy/locks/` and the journal
  directory holds no `.lock` file.
- Given a vet-block append, when the command returns, then `VET.md.lock`
  does not exist anywhere under `<spec-dir>/journal/`.
</behavior>

<scenario id="CHK-001">
Given a workspace with one spec and one task,
when `speccy journal append` writes a task block to that task's journal,
then no `*.lock` file exists under `<spec-dir>/journal/` and a lock file
for that task exists under `.speccy/locks/`.
</scenario>

<scenario id="CHK-002">
Given a workspace with one spec,
when `speccy journal append` writes a vet block to that spec's VET.md,
then no `VET.md.lock` exists under `<spec-dir>/journal/`.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: Lock path is a deterministic, collision-free function of the journal target

Every journal target maps to exactly one lock path: the same
`<spec-dir>/journal/<task-id>.md` (or `VET.md`) always resolves to the
same lock file across invocations, and two distinct journal targets
always resolve to distinct lock files. This mapping is what preserves
per-journal serialization after the relocation.

<done-when>
- Two lock-path resolutions for the same task journal yield byte-equal
  paths.
- Two lock-path resolutions for journals differing in task id resolve to
  different paths.
- Two lock-path resolutions for same-id journals under different specs
  resolve to different paths.
</done-when>

<behavior>
- Given two appends targeting the same task journal, when each resolves
  its lock path, then both resolve to the identical path so they contend
  on one lock.
- Given two appends targeting journals in different specs, when each
  resolves its lock path, then the paths differ so neither blocks the
  other.
</behavior>

<scenario id="CHK-003">
Given the lock-path resolver,
when it is called twice with the same spec id and task id,
then it returns byte-identical paths both times.
</scenario>

<scenario id="CHK-004">
Given the lock-path resolver,
when it is called for two journal targets that differ in spec id or task
id,
then it returns two distinct paths.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: The relocated lock preserves the SPEC-0055 concurrent-appender contract

Relocating the lock must not regress any SPEC-0055 REQ-005 guarantee.
The guarantees, enumerated in `<done-when>`, hold verbatim under the
new lock location; the only change is where the lock file lives.

<done-when>
- Eight concurrent task appends to one journal produce exactly eight
  parser-clean blocks.
- Two concurrent round-opening appends to one journal derive distinct,
  correctly ordered rounds.
- When a lock is held past the 10s timeout, a waiting append exits
  non-zero, the journal is left byte-identical, and the diagnostic names
  the journal path.
- A first append to a not-yet-existing journal acquires its lock and
  succeeds (lock placement does not depend on the journal `.md`
  pre-existing).
</done-when>

<behavior>
- Given eight processes appending distinct review blocks to one journal,
  when they run concurrently, then exactly eight well-formed blocks are
  present and the file parses cleanly.
- Given an externally held lock on a journal's relocated lock file, when
  another append waits past the timeout, then it exits non-zero with the
  journal byte-identical and stderr naming the journal path.
</behavior>

<scenario id="CHK-005">
Given a fresh journal and eight processes each appending one distinct
review block to it concurrently,
when all processes complete,
then the journal parses cleanly and contains exactly eight review
blocks.
</scenario>

<scenario id="CHK-006">
Given a test that holds an exclusive lock on a task journal's relocated
lock path under `.speccy/locks/`,
when a `speccy journal append` to that journal runs and waits past the
10s timeout,
then the append exits non-zero, stderr names the journal path, and the
journal file is byte-identical to its pre-append state.
</scenario>

<scenario id="CHK-007">
Given a fresh journal with no prior blocks and two processes each
appending a round-opening `implementer` block concurrently,
when both complete,
then the two blocks carry distinct, correctly ordered rounds.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
Relocate the lock sidecars; do not delete them. Unlinking a held
advisory lock reintroduces the classic unlink-while-locked race that
SPEC-0055's contract cannot tolerate. On POSIX, a waiter that already
opened the lock path locks the now-orphaned inode after the holder
unlinks, while a third appender's `create(true)` makes a fresh inode at
the same path and locks that — two appenders then hold "the" lock
simultaneously, breaking REQ-005 mutual exclusion. On Windows (a
supported host), `remove_file` on a lock file with an open handle fails
with a sharing violation, so deletion would error mid-critical-section.
The never-delete-the-lockfile pattern matches ecosystem tools (e.g.
cargo). Therefore "cleanup" is achieved by moving the sidecars out of
the journal directories into one ignored runtime location, and the lock
files persist by design.
</decision>

<decision id="DEC-002">
Locks live under `.speccy/locks/`, mirroring the spec/task structure so
the path is human-readable and collision-free:
`.speccy/locks/<spec-id>/<task-id>.md.lock` for task journals and
`.speccy/locks/<spec-id>/VET.md.lock` for vet journals. Spec id plus
task id (or the literal `VET`) is already unique, so structured paths
need no hashing to satisfy REQ-002. This was preferred over (a) the OS
temporary directory — opaque, harder to inspect or clear when debugging
contention, and subject to platform-variable cleanup that could remove a
live lock — and (b) a flat hashed filename — collision-free but
unreadable when diagnosing which journal a lock belongs to. Retaining
the `.md.lock` suffix keeps the existing `*.md.lock` gitignore pattern
matching, so no new ignore rule is required.
</decision>

<decision id="DEC-003">
The "locking works before the journal exists" property (the original
reason the lock was a sidecar rather than the journal `.md` itself,
`journal.rs:346-348`) is preserved by creating the locks directory with
`create_dir_all` before acquiring, exactly as the journal directory is
created today (`journal.rs:341-344`, `:408-411`). The lock target's
existence stays independent of the journal `.md`'s existence; only the
parent directory of the lock file changes. Locking the journal `.md`
directly was rejected for this reason — it would force creating an empty
`.md` to lock it, regressing REQ-005's "journal still absent on failed
first append" guarantee.
</decision>

## Notes

The integration test `held_lock_makes_waiting_append_time_out_nonzero`
(`speccy-cli/tests/journal_append.rs:368-439`) hardcodes the sidecar
path `<journal-dir>/T-001.md.lock` to simulate contention; it must be
updated to the relocated path under `.speccy/locks/`. The other two
concurrency tests — `eight_concurrent_review_appends_serialize_cleanly`
and `two_concurrent_round_opening_appends_get_distinct_rounds`
(`journal_append.rs:216-359`) — are mechanism-agnostic (they observe
only block/round outcomes), so their continuing to pass unchanged is the
regression signal that relocation preserved the REQ-003 contract.

The existing `*.md.lock` line in `.gitignore` already covers the
relocated files because they keep the `.md.lock` suffix; a redundant
`.speccy/locks/` entry is optional and not required by any requirement
here.

## Open Questions

<!-- alpha-prefix ordinals; preserve existing, allocate next free letter -->

- [ ] a. The original ask was to "cleanup lock files after each task is
  done." This SPEC reinterprets that as relocate-and-persist (one
  ignored directory) rather than delete-after-each-append, because safe
  per-append deletion is not achievable without weakening SPEC-0055
  REQ-005 mutual exclusion (see DEC-001). Confirm relocation satisfies
  the intent; if true deletion is still required, the SPEC must instead
  adopt a documented race-mitigation strategy and accept its cost.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-10 | kevin | Initial SPEC: relocate journal advisory-lock sidecars out of `journal/` into `.speccy/locks/`; deterministic collision-free mapping; never-delete decision; SPEC-0055 contract preserved. |
</changelog>
