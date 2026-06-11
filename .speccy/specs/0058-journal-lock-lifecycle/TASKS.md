---
spec: SPEC-0058
spec_hash_at_generation: 6516a84fad515b2fb29e78641a834d58eaa2585fb246cf4efdf4e8232691eaa9
generated_at: 2026-06-11T04:48:15Z
---
# Tasks: SPEC-0058 Journal lock-file lifecycle — reap advisory lock sidecars at terminal lifecycle boundaries

<task id="T-001" state="completed" covers="REQ-005">
## Adopt `tracing` as the CLI diagnostic channel

Add a `tracing` diagnostic channel to the CLI so later tasks' reap helper can
warn on an unexpected failure instead of swallowing it (DEC-005). This task
adds the dependency and the subscriber only; no reap code uses it yet.

- Add `tracing` and `tracing-subscriber` (the latter with its `env-filter`
  feature) to the root `[workspace.dependencies]` table in `Cargo.toml`,
  alongside the existing entries (e.g. `fs4`, `jiff`). Reference both from
  `speccy-cli/Cargo.toml`'s `[dependencies]` with `{ workspace = true }`,
  matching the existing workspace-dep convention. Both crates are
  `MIT OR Apache-2.0`; confirm `cargo deny check` stays green.
- Initialize the subscriber exactly once at the top of `fn main()` in
  `speccy-cli/src/main.rs:298` (before `Cli::parse()`), writing to **stderr**
  (`tracing_subscriber::fmt().with_writer(std::io::stderr)`), with an
  `EnvFilter` sourced from the environment and a sensible default level. The
  subscriber must never write to stdout — the stable stdout / JSON
  command-output contract is unaffected (REQ-005, DEC-005).
- No Speccy-specific configuration knobs (no new CLI flags, no config file);
  only the standard `tracing-subscriber` env filter governs level. The CLI
  remains deterministic — `tracing` observes, it does not drive control flow
  (Core principle 2).

Add an integration test (drive the built binary via `assert_cmd`) asserting
that a contracted-stdout command — e.g. `speccy status --json` against a
scratch `common::Workspace` — emits stdout that parses cleanly as JSON with
no log line interleaved, proving the subscriber does not pollute stdout
(CHK-008).

<task-scenarios>
Given the built `speccy` binary after this task and a scratch workspace,
when `speccy status --json` runs with the tracing subscriber active,
then stdout parses as a single well-formed JSON value with no log line
interleaved, and any diagnostics appear only on stderr (CHK-008).

Given the workspace after this task,
when `cargo deny check` runs,
then it passes (the two new dependencies clear the license/advisory gates).

Suggested files: `Cargo.toml`, `speccy-cli/Cargo.toml`,
`speccy-cli/src/main.rs`, `speccy-cli/tests/` (new or existing integration
test for stdout cleanliness)
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-003 REQ-004 REQ-005">
## Add the guarded, idempotent `reap_lock_sidecar` helper

Add `pub(crate) fn reap_lock_sidecar(lock_path: &Utf8Path)` to
`speccy-cli/src/journal.rs`, as a sibling of `LockGuard` (place it after the
`Drop for LockGuard` impl ending at `journal.rs:703`). The helper is the
shared reap both call sites in later tasks will invoke; this task adds it and
its direct unit coverage only — no call sites are wired yet.

Behaviour (DEC-004, REQ-003, REQ-005):
- Open `lock_path` with `fs_err::OpenOptions::new().read(true).write(true)` —
  deliberately **without** `create(true)` (unlike `LockGuard::acquire` at
  `journal.rs:658-663`). An open error whose kind is `NotFound` is the
  idempotent no-op: return silently. Reaping must never create a sidecar.
- Unwrap the `fs-err` wrapper to a `std::fs::File` via `.into_parts()`
  (mirror `journal.rs:669`).
- Take a single non-blocking `fs4::FileExt::try_lock`. On
  `Err(TryLockError::WouldBlock)` the lock is held — return silently without
  unlinking (the held-lock skip; no warning, this is an expected no-op).
- On `Ok(())`, **drop the file handle before** calling
  `fs_err::remove_file(lock_path)`. Dropping releases both the advisory lock
  and the OS handle, so the unlink succeeds on Windows (where `remove_file`
  against a path with a live open handle fails with a sharing violation, see
  DEC-001/DEC-004) as well as POSIX.
- The helper returns no `Result` (`-> ()`, per DEC-004): it runs only after
  each command's load-bearing mutation has already landed, so a reap failure
  must never fail the command. But rather than swallow an unexpected error,
  emit exactly one `tracing::warn!` with the sidecar path as a structured
  field on any error short of a clean reap: an open error other than
  `NotFound`, a `try_lock` `Error`, or a `remove_file` error (REQ-005,
  DEC-005). The expected no-ops — `NotFound` open and `WouldBlock` — stay
  silent.

`Drop for LockGuard` and `LockGuard::acquire` are unchanged, and the
append-time lock path is unmoved — the reap is purely additive (REQ-004). No
new CLI subcommand and no public API (DEC-002).

Add a `#[cfg(test)] mod tests` in `journal.rs` exercising the helper directly
against a tempdir-backed `Utf8Path`, capturing emitted `tracing` events with
a test-local subscriber (e.g. `tracing-test`, or
`tracing::subscriber::with_default` over a capturing collector):
- a free sidecar is unlinked, the helper returns, and no `WARN` is emitted;
- a sidecar held by a second locked handle (open the path, `into_parts()`,
  `FileExt::lock(&std_file)`) survives the reap with no `WARN` (expected
  no-op), then the held handle unlocks for teardown;
- an absent sidecar path is a no-op with no `WARN`;
- an induced unexpected failure (e.g. a read-only parent directory so
  `remove_file` fails — gate the induction by platform if needed) emits
  exactly one `WARN` naming the sidecar path and the helper still returns
  (REQ-005, CHK-009).

<task-scenarios>
Given a tempdir holding a `foo.md.lock` with no lock taken on it,
when `reap_lock_sidecar` is called on that path under a capturing subscriber,
then the file no longer exists and no `WARN` event was emitted.

Given a `foo.md.lock` whose exclusive advisory lock is held by a separate
open handle in the test,
when `reap_lock_sidecar` is called on that path,
then the file still exists afterward and no `WARN` was emitted (the held lock
is an expected no-op, never unlinked).

Given a path for which no `.lock` file exists,
when `reap_lock_sidecar` is called on it,
then the call returns, the path still does not exist, and no `WARN` was
emitted.

Given a `foo.md.lock` whose unlink is induced to fail after a successful
`try_lock`,
when `reap_lock_sidecar` runs under a capturing subscriber,
then exactly one `WARN`-level event naming the sidecar path is emitted and
the helper returns without panicking (CHK-009).

Given the workspace after this task,
when `cargo test -p speccy-cli` runs,
then the three SPEC-0055 concurrency tests
(`eight_concurrent_review_appends_serialize_cleanly`,
`two_concurrent_round_opening_appends_get_distinct_rounds`,
`held_lock_makes_waiting_append_time_out_nonzero` in
`speccy-cli/tests/journal_append.rs`) still pass unchanged — the regression
signal that `Drop` stays unlock-only and the append path is unmoved (REQ-004).

Suggested files: `speccy-cli/src/journal.rs`, `speccy-cli/Cargo.toml`
(test-local capturing-subscriber dev-dependency, if used)
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-001 REQ-003">
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

<task id="T-004" state="pending" covers="REQ-002">
## Reap the vet lock sidecar on the terminal `--block gate` append

Wire `reap_lock_sidecar` into `run_vet_append` in
`speccy-cli/src/journal.rs` (lines ~366-427). The vet lock path
`journal_dir.join("VET.md.lock")` is already bound at `journal.rs:413` and the
append's own `_guard` is acquired at `journal.rs:414`. After
`append_vet_under_lock(&inputs)` returns Ok **and the append's `_guard` has
been released**, reap `VET.md.lock` — but only for the terminal `gate` block
(`matches!(kind, VetBlockKind::Gate)`). Restructure the tail of
`run_vet_append` so the guard is provably out of scope before the reap runs
(bind the append result, drop the guard explicitly or close its lexical
scope, then reap, then return the bound result). The reap must observe the
lock as free because the append released it first; the `try_lock` guard
(REQ-003) is the defensive backstop, terminal-boundary quiescence (the gate
is the last vet write on every exit path, DEC-003) is the real safety
contract.

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
