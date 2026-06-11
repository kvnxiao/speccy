---
spec: SPEC-0058
generated_at: 2026-06-11T05:59:56Z
---

## Invocation 1 — 2026-06-11T05:59:56Z

<drift-review verdict="pass" round="1" date="2026-06-11T05:59:56Z" model="claude-opus-4-8[1m]/high">
Diff satisfies SPEC-0058 as a unit: all five requirements covered, all five decisions honored, every non-goal intact, and the SPEC-0055 append contract preserved verbatim.

Requirement-by-requirement walk (all green):
- REQ-001 — `transition.rs:131-138` reaps the task lock sidecar only on `to == TaskState::Completed`, after the TASKS.md state rewrite, never opening the journal `.md`. CHK-001/002/005 plus the non-completed negative case pass (`tests/transition_reap.rs`).
- REQ-002 — `run_vet_append` (`journal.rs:418-446`) reaps `VET.md.lock` only on `matches!(kind, VetBlockKind::Gate)` and only on `appended.is_ok()`, with the append `_guard` provably dropped first via the bound-then-close-scope restructure. CHK-003/004 plus idempotent-absent and non-gate holistic-fix cases pass (`tests/journal_append_vet.rs`).
- REQ-003 — `reap_lock_sidecar` (`journal.rs:741-800`) takes one non-blocking `FileExt::try_lock`; `WouldBlock` returns silently without unlinking. Held-lock skip verified at both call sites.
- REQ-004 — `LockGuard::acquire` and `Drop for LockGuard` are byte-unchanged; the only edit to the guard is lexical re-scoping so it drops before the reap (append-time lock path unmoved). The three SPEC-0055 concurrency tests (`eight_concurrent_review_appends_serialize_cleanly`, `two_concurrent_round_opening_appends_get_distinct_rounds`, `held_lock_makes_waiting_append_time_out_nonzero`) pass unchanged.
- REQ-005 — `init_tracing` (`main.rs`) installs the subscriber once at the top of `fn main()` (before `Cli::parse()`), stderr-only, default `WARN` env filter; the reap emits exactly one `warn!` carrying the `sidecar` path field on each unexpected error (open!=NotFound, try_lock Error, remove_file error) and stays silent on the expected NotFound/WouldBlock no-ops. CHK-008 passes (`tests/tracing_stdout.rs`); CHK-009 is correctly `#[cfg(unix)]`-gated per the T-002 allowance (read-only-dir induction is POSIX-only) and runs on Unix CI.

Verification run on this host: targeted binaries (`transition_reap`, `journal_append_vet`, `tracing_stdout`, `journal_append`) all pass; `journal::tests` unit reap coverage passes (3 of 4; CHK-009 Unix-gated); `cargo clippy -p speccy-cli --all-targets --all-features -- -D warnings` clean; `cargo deny check` ends "advisories ok, bans ok, licenses ok, sources ok" (the two new MIT/Apache deps clear). No `.md.lock` sidecars are tracked and no stray untracked files remain.

Decisions honored: DEC-001 (no unlink at release; reap only at quiescent boundary), DEC-002 (no new subcommand — folded into transition + gate append), DEC-003 (vet lock reaped at gate, sidecar path/create(true) untouched), DEC-004 (open without create(true), try_lock, drop handle before remove_file), DEC-005 (tracing adopted, stderr-only, no Speccy knobs, single init). No non-goal violated: no release-time unlink, no new CLI verb, no lock-path/timeout/grammar change, no public API beyond the `pub(crate)` internal helper.

Observation for the human (not a SPEC-0058 blocker): the branch also carries commit 98fe23d "reviewer personas: forbid mutating the shared working tree" — a self-contained, properly-ejected (resources/ + .claude/ + .codex/ in sync) repo-infrastructure fix that touches no SPEC-0058 code path and violates no SPEC-0058 non-goal. It is out-of-band relative to this SPEC's contract; flag only so the eventual PR scope is intentional.
</drift-review>
<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates worth applying in the SPEC-0058 diff.

Reviewed the production code (speccy-cli/src/journal.rs reap_lock_sidecar + run_vet_append gate reap, src/main.rs init_tracing, src/transition.rs completed-edge reap) and the new tests. Considered and rejected:
- journal.rs:~760-790 reap_lock_sidecar's three tracing::warn! sites (open / probe / unlink) — each names a distinct failed operation in its message; collapsing them would lose the diagnostic specificity that REQ-005/DEC-005 calls for. Not a simplification.
- transition.rs / journal.rs both build a *.lock sidecar path inline before reaping — the two paths differ (VET.md.lock vs {task-id}.md.lock) and live in separate files; extracting a shared builder would expand the diff surface across modules for two callsites. Out of scope, not genuine duplication.
- journal_append_vet.rs repeats the drift-review append arg array across ~4 tests — deliberate inline scenario spelling; per AGENTS.md test hygiene and simplifier discipline, factoring 2-4 similar-looking fixtures hurts readability. Skip.
Comments are load-bearing (encode SPEC decisions); leaving as-is.
</simplifier-scan>
<gate verdict="passed" tasks_hash="e4835c58699faaf8b6d42e0f8adb5b075668abea81e030d5719c444ea3fe1122" date="2026-06-11T06:01:03Z">
Drift review passed on round 1 (0 fix rounds); simplifier scan clean. SPEC-0058 adheres to the SPEC as a unit and is ready to ship.
</gate>
