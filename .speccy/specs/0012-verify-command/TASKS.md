---
spec: SPEC-0012
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0012 verify-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Captured check execution (speccy-core)

- [x] **T-001**: Implement `exec::run_checks_captured` with tee-to-stderr
  - Covers: REQ-002
  - Tests to write:
    - Child stdio is piped (not inherited); a `Stdio::piped()` setup is wired for both stdout and stderr.
    - Captured output is forwarded to the parent's stderr line-by-line (or chunk-by-chunk) as the child writes.
    - Returned `CheckResult` per check has populated fields (spec_id, check_id, kind, outcome, exit_code, duration_ms).
    - A slow fixture command (e.g. `sleep 0.1; echo a; sleep 0.1; echo b`) produces interleaved output on stderr in roughly real time -- not buffered until completion. (Exact timing assertions are flaky; the test asserts ordering and that the first chunk appears before the child exits.)
    - Manual checks (`kind = "manual"` or any kind with `prompt` and no `command`) return `CheckResult { outcome: Manual, exit_code: None }` without spawning a subprocess; stderr receives the prompt text.
  - Suggested files: `speccy-core/src/exec.rs`, `speccy-core/tests/exec_captured.rs`

## Phase 2: Lint integration

- [x] **T-002**: Run lint via `lint::run` and partition by Level
  - Covers: REQ-001
  - Tests to write:
    - `lint::run` is called against a `lint::Workspace` built from `workspace::scan` output.
    - Returned diagnostics are partitioned into `errors`, `warnings`, `info` based on each diagnostic's `Level`.
    - Lint runs even on workspaces with no checks (verify never skips lint).
    - Empty workspace produces empty buckets without error.
  - Suggested files: `speccy-cli/src/verify.rs`, `speccy-cli/tests/verify_lint_integration.rs`

## Phase 3: Exit-code aggregation

- [x] **T-003**: Compose lint and check outcomes into a binary exit code
  - Covers: REQ-003
  - Tests to write:
    - Clean lint + all checks pass -> exit 0.
    - 1 lint error + all checks pass -> exit 1.
    - Clean lint + 1 failing check -> exit 1.
    - Lint warnings/info only (no errors) + all checks pass -> exit 0.
    - Empty workspace -> exit 0.
    - Deterministic: two runs against the same workspace produce the same exit code.
  - Suggested files: `speccy-cli/src/verify.rs` (extend), `speccy-cli/tests/verify_exit_code.rs`

## Phase 4: Text-mode summary

- [x] **T-004**: Implement text-mode summary output
  - Covers: REQ-004
  - Tests to write:
    - The last three stdout lines are `Lint: <E> errors, <W> warnings, <I> info`, `Checks: <P> passed, <F> failed, <M> manual`, `verify: PASS|FAIL`.
    - PASS appears iff exit code is 0; FAIL otherwise.
    - Stderr received the live streamed output and per-check headers (`==> CHK-NNN ...`) / footers (`<-- CHK-NNN PASS|FAIL`).
    - Empty workspace prints `Lint: 0 errors, 0 warnings, 0 info` / `Checks: 0 passed, 0 failed, 0 manual` / `verify: PASS`.
  - Suggested files: `speccy-cli/src/verify_output.rs`, `speccy-cli/tests/verify_text.rs`

## Phase 5: JSON output

- [x] **T-005**: Implement `--json` envelope and structured per-check output
  - Covers: REQ-005
  - Tests to write:
    - Output begins with `"schema_version": 1` (first non-whitespace key).
    - Includes `repo_sha` (the SHA or `""` if git unavailable, same as SPEC-0004 DEC-003).
    - `lint.errors / warnings / info` arrays contain structured `Diagnostic` objects (not strings).
    - `checks` array contains per-check structured objects with all fields populated.
    - `summary.lint` and `summary.checks` aggregate counts match the arrays.
    - `passed` is `true` iff exit code is 0.
    - Pretty-printed.
    - Two runs against identical state produce byte-identical stdout.
  - Suggested files: `speccy-cli/src/verify_output.rs` (extend), `speccy-cli/tests/verify_json.rs`

## Phase 6: CLI wiring

- [x] **T-006**: Wire `speccy verify [--json]` into the binary
  - Covers: REQ-001..REQ-005
  - Tests to write:
    - End-to-end via `assert_cmd` with a tmpdir fixture workspace.
    - Cover: clean workspace -> exit 0; lint-failing workspace -> exit 1; check-failing workspace -> exit 1; both failing -> exit 1.
    - Text mode and JSON mode each tested separately.
    - From outside a speccy workspace -> exit 1 with `VerifyError::ProjectRootNotFound`.
    - Manual checks are exercised by the fixture (a `kind = "manual"` check that exits the run with PASS but a `manual` count of 1).
  - Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/verify.rs`, `speccy-cli/tests/integration_verify.rs`
