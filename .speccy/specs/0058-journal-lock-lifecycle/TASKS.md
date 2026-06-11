---
spec: SPEC-0058
spec_hash_at_generation: 9fda71eac3955cb526b77e55d8c173b7fdb55af76688374bc518ea352104f0c7
generated_at: 2026-06-11T04:25:34Z
---
# Tasks: SPEC-0058 Journal lock-file lifecycle — reap advisory lock sidecars at terminal lifecycle boundaries

<task id="T-001" state="pending" covers="REQ-003 REQ-004">
## Add the guarded, idempotent `reap_lock_sidecar` helper

Add `pub(crate) fn reap_lock_sidecar(lock_path: &Utf8Path)` to
`speccy-cli/src/journal.rs`, as a sibling of `LockGuard` (place it after the
`Drop for LockGuard` impl ending at `journal.rs:703`). The helper is the
shared reap both call sites in later tasks will invoke; this task adds it and
its direct unit coverage only — no call sites are wired yet.

Behaviour (DEC-004, REQ-003):
- Open `lock_path` with `fs_err::OpenOptions::new().read(true).write(true)` —
  deliberately **without** `create(true)` (unlike `LockGuard::acquire` at
  `journal.rs:658-663`). A `NotFound` open error is the idempotent no-op:
  return without error. Reaping must never create a sidecar.
- Unwrap the `fs-err` wrapper to a `std::fs::File` via `.into_parts()`
  (mirror `journal.rs:669`).
- Take a single non-blocking `fs4::FileExt::try_lock`. On
  `Err(TryLockError::WouldBlock)` the lock is held — return without
  unlinking (the held-lock skip). On `Err(TryLockError::Error(_))` also
  return without unlinking (best-effort).
- On `Ok(())`, **drop the file handle before** calling
  `fs_err::remove_file(lock_path)`. Dropping releases both the advisory lock
  and the OS handle, so the unlink succeeds on Windows (where `remove_file`
  against a path with a live open handle fails with a sharing violation, see
  DEC-001/DEC-004) as well as POSIX. Ignore the `remove_file` result (a
  racing reaper or `NotFound` is a safe no-op).

The helper is infallible (returns no `Result`, per DEC-004): it will run only
after each command's load-bearing mutation has already landed, so a reap
failure must never fail the command. This matches the "not actionable on
failure" posture of `Drop for LockGuard` (`journal.rs:695-703`).

`Drop for LockGuard` and `LockGuard::acquire` are unchanged, and the
append-time lock path is unmoved — the reap is purely additive (REQ-004). No
new CLI subcommand and no public API (DEC-002).

Add a `#[cfg(test)] mod tests` in `journal.rs` exercising the helper directly
against a tempdir-backed `Utf8Path`:
- a free sidecar is unlinked and the helper returns;
- a sidecar held by a second locked handle (open the path, `into_parts()`,
  `FileExt::lock(&std_file)`) survives the reap, then the held handle
  unlocks for teardown;
- an absent sidecar path is a no-op (the path still does not exist
  afterward).

<task-scenarios>
Given a tempdir holding a `foo.md.lock` file with no lock currently taken on
it,
when `reap_lock_sidecar` is called on that path,
then the file no longer exists.

Given a `foo.md.lock` whose exclusive advisory lock is currently held by a
separate open file handle in the test,
when `reap_lock_sidecar` is called on that path,
then the file still exists afterward (the held lock was skipped, never
unlinked).

Given a path under a tempdir for which no `.lock` file exists,
when `reap_lock_sidecar` is called on it,
then the call returns with no error and the path still does not exist.

Given the workspace after this task,
when `cargo test -p speccy-cli` runs,
then the three SPEC-0055 concurrency tests
(`eight_concurrent_review_appends_serialize_cleanly`,
`two_concurrent_round_opening_appends_get_distinct_rounds`,
`held_lock_makes_waiting_append_time_out_nonzero` in
`speccy-cli/tests/journal_append.rs`) still pass unchanged — the regression
signal that `Drop` stays unlock-only and the append path is unmoved (REQ-004).

Suggested files: `speccy-cli/src/journal.rs`
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-001 REQ-003">
## Reap the task lock sidecar on the `--to completed` transition

Wire `reap_lock_sidecar` into `speccy-cli/src/transition.rs`. In the
`TransitionKind::Legal` arm (`transition.rs:118-128`), after the
`fs_err::write` of the spliced TASKS.md succeeds and before `Ok(())`, reap the
task journal's lock sidecar — but only when the target state is `completed`
(`to == TaskState::Completed`; `TaskState` is already imported at
`transition.rs:15`). The sidecar path is
`location.spec_dir.join("journal").join(format!("{}.md.lock", location.task.id))`
(`location.spec_dir` and `location.task.id` are in scope at
`transition.rs:119-120`). Call `crate::journal::reap_lock_sidecar(&path)`.

Only the `--to completed` edge reaps; no other transition edge (including
`in-review` → `pending` on a blocking review round) touches the sidecar, and
the journal `.md` itself is never opened or modified (REQ-001). Because the
reap is guarded by `try_lock` (REQ-003), a transition whose sidecar is
unexpectedly held leaves it in place and still exits zero.

Add integration coverage driving the built `speccy` binary via `assert_cmd`
against a scratch `common::Workspace`, following the existing patterns in
`speccy-cli/tests/transition.rs` and the held-lock setup at
`speccy-cli/tests/journal_append.rs:391-426`. Place the new tests in a new
`speccy-cli/tests/transition_reap.rs` binary (keeps the lock-lifecycle
scenarios cohesive), or extend `transition.rs` if that reads cleaner. Cover:
- CHK-001: a task whose journal `.md` and `<task-id>.md.lock` both exist
  transitions to `completed`; afterward the `.lock` is gone and the journal
  `.md` is byte-identical to its pre-transition bytes.
- CHK-002: a task with a journal but no lock sidecar transitions to
  `completed`; the command exits zero and the journal is unchanged.
- CHK-005: with the `<task-id>.md.lock` held by an exclusive lock from the
  test process, a `--to completed` transition exits zero and the sidecar
  still exists afterward (the guard skipped the held lock).
- A non-`completed` edge (e.g. `pending` → `in-progress`) leaves an existing
  lock sidecar in place (REQ-001 done-when / behavior).

<task-scenarios>
Given a `completed`-bound task whose `T-001.md` and `T-001.md.lock` both exist
in its `journal/` directory,
when `speccy task transition SPEC-NNNN/T-001 --to completed` runs,
then `T-001.md.lock` no longer exists and `T-001.md` is byte-identical to its
pre-transition state (CHK-001).

Given a task with a `T-001.md` journal but no `T-001.md.lock`,
when it transitions to `completed`,
then the command exits zero and the journal is unchanged (CHK-002).

Given a task whose `T-001.md.lock` is held by an exclusive advisory lock from
the test process,
when `speccy task transition SPEC-NNNN/T-001 --to completed` runs its reap,
then the command exits zero and `T-001.md.lock` still exists (the guard
skipped the held lock — CHK-005).

Given a task with a `T-001.md.lock` sidecar,
when it transitions to a state other than `completed` (e.g.
`pending` → `in-progress`),
then the sidecar is left in place.

Suggested files: `speccy-cli/src/transition.rs`,
`speccy-cli/tests/transition_reap.rs`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-002">
## Reap the vet lock sidecar on the terminal `--block gate` append

Wire `reap_lock_sidecar` into `run_vet_append` in
`speccy-cli/src/journal.rs` (lines ~366-427). The vet lock path
`journal_dir.join("VET.md.lock")` is already bound at `journal.rs:413` and the
append's own `_guard` is acquired at `journal.rs:414`. After
`append_vet_under_lock(&inputs)` returns Ok **and the append's `_guard` has
been released**, reap `VET.md.lock` — but only for the terminal `gate` block
(`matches!(kind, VetBlockKind::Gate)`). Restructure the tail of
`run_vet_append` so the guard is provably out of scope before the reap runs
(bind the append result, drop the guard explicitly or close its lexical scope,
then reap, then return the bound result). The reap must observe the lock as
free because the append released it first; the `try_lock` guard (REQ-003) is
the defensive backstop, terminal-boundary quiescence (the gate is the last vet
write on every exit path, DEC-003) is the real safety contract.

Only the `gate` block reaps. Every non-gate vet block (`drift-review`,
`holistic-fix`, `simplifier-scan`, `simplifier-apply`) leaves `VET.md.lock` in
place for the next sequential appender, and no task block ever reaps the vet
lock (REQ-002). `VET.md` contents and the append grammar are unchanged.

Add integration coverage in the existing
`speccy-cli/tests/journal_append_vet.rs` (reuse its vet-append fixtures),
driving the built binary:
- CHK-003: a spec whose `VET.md` and `VET.md.lock` exist from prior vet
  blocks; after `speccy journal append SPEC-NNNN --block gate` runs,
  `VET.md.lock` no longer exists and `VET.md` parses cleanly
  (`parse_journal_xml`) with the gate as its last element.
- CHK-004: after a `drift-review` vet append, `VET.md.lock` still exists
  (only the `gate` block reaps).

<task-scenarios>
Given a spec whose `VET.md` and `VET.md.lock` exist from prior vet blocks,
when `speccy journal append SPEC-NNNN --block gate` runs,
then `VET.md.lock` no longer exists and `VET.md` parses cleanly with the gate
as its last element (CHK-003).

Given a vet `drift-review` append to a spec,
when it completes,
then `VET.md.lock` still exists, ready for the next sequential vet appender
(CHK-004).

Given a non-gate vet block other than `drift-review` (e.g. `holistic-fix`),
when it is appended,
then the vet lock sidecar is left in place (only `gate` reaps — REQ-002).

Suggested files: `speccy-cli/src/journal.rs`,
`speccy-cli/tests/journal_append_vet.rs`
</task-scenarios>
</task>
