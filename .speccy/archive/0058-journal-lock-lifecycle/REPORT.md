---
spec: SPEC-0058
outcome: implemented
generated_at: 2026-06-11T06:15:00Z
---

# REPORT: SPEC-0058 Journal lock-file lifecycle — reap advisory lock sidecars at terminal lifecycle boundaries

<report spec="SPEC-0058">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-003 wired `reap_lock_sidecar` into `speccy-cli/src/transition.rs` at the
`TransitionKind::Legal` arm. The reap fires only when `to == TaskState::Completed`,
after the TASKS.md state rewrite succeeds. No other transition edge touches the
sidecar and the journal `.md` is never opened or modified. Integration tests in
`speccy-cli/tests/transition_reap.rs` cover CHK-001 (lock present: sidecar deleted,
journal byte-identical), CHK-002 (no sidecar: exits zero, journal unchanged), the
held-sidecar guard case, and the non-completed-edge negative case (existing sidecar
persists through a `pending -> in-progress` transition). Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-004 wired `reap_lock_sidecar` into `run_vet_append` in
`speccy-cli/src/journal.rs`, restructuring the tail of the function so the
append `_guard` is provably out of scope before the reap runs. The reap fires
only when `matches!(kind, VetBlockKind::Gate)` and only on a successful append.
Non-gate vet blocks (`drift-review`, `holistic-fix`, `simplifier-scan`,
`simplifier-apply`) leave `VET.md.lock` in place. Integration tests in
`speccy-cli/tests/journal_append_vet.rs` cover CHK-003 (gate append: sidecar
deleted, VET.md parses cleanly with gate as last element), CHK-004
(drift-review append: sidecar persists), the absent-sidecar idempotent case,
and the non-gate `holistic-fix` negative case. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005">
T-002 added `pub(crate) fn reap_lock_sidecar(lock_path: &Utf8Path)` to
`speccy-cli/src/journal.rs` (after the `Drop for LockGuard` impl). The helper
opens the sidecar without `create(true)`, takes one non-blocking
`FileExt::try_lock`, and only proceeds to unlink on `Ok(())`. A `WouldBlock`
result returns silently without unlinking — the held-lock skip. CHK-005 is
exercised via the transition integration test holding an exclusive advisory lock
from the test process and confirming the transition exits zero with the sidecar
intact. The held-lock unit case in `journal::tests` also covers this path
directly. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006 CHK-007">
T-002 added the reap helper as a purely additive sibling of `LockGuard`; neither
`LockGuard::acquire` nor `Drop for LockGuard` was modified. The append-time lock
path is unmoved. The three SPEC-0055 concurrency tests
(`eight_concurrent_review_appends_serialize_cleanly`,
`two_concurrent_round_opening_appends_get_distinct_rounds`,
`held_lock_makes_waiting_append_time_out_nonzero` in
`speccy-cli/tests/journal_append.rs`) all pass unchanged — the regression signal
that `Drop` stays unlock-only and the mutual-exclusion contract is preserved. The
only structural change to `run_vet_append` was lexical re-scoping of the guard so
it drops before the reap, with the append-time lock path and timeout untouched.
Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008 CHK-009">
T-001 added `tracing` and `tracing-subscriber` (with `env-filter`) as workspace
dependencies and initialized the subscriber once at the top of `fn main()` in
`speccy-cli/src/main.rs` before `Cli::parse()`, writing exclusively to stderr
with an `EnvFilter` sourced from the environment and a default `WARN` level. No
stdout / JSON output contract is affected. T-002 implemented the `tracing::warn!`
emission in `reap_lock_sidecar`: unexpected errors (open error other than
`NotFound`, `try_lock` `Error`, `remove_file` error) emit exactly one `WARN`
event carrying the sidecar path as a structured field; expected no-ops
(`NotFound` open and `WouldBlock` try_lock) return silently. CHK-008 is covered
by an integration test in `speccy-cli/tests/tracing_stdout.rs` asserting that
`speccy status --json` stdout parses as well-formed JSON with no log lines
interleaved. CHK-009 is covered by a `#[cfg(unix)]`-gated unit test in
`journal::tests` using a read-only parent directory to induce a `remove_file`
failure and a capturing subscriber to assert exactly one `WARN` event naming the
sidecar path. `cargo deny check` clears for both new MIT/Apache-2.0 dependencies.
T-001 required 3 review rounds (2 retries) due to tracing subscriber initialization
placement and env-filter configuration; T-002 required 1 round. Retry count: 2 (T-001: 2, T-002: 0).
</coverage>

</report>
