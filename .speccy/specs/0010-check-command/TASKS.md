---
spec: SPEC-0010
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0010 check-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Check discovery

- [x] **T-001**: Discover checks via `workspace::scan` (SPEC-0004)
  - Covers: REQ-001
  - Tests to write:
    - Enumerate every `[[checks]]` from every parsed `spec.toml`; ordering = `(spec_id ascending, declared check order within spec)`.
    - Each enumerated check carries `{spec_id, check_id, kind, command?, prompt?, proves}`.
    - Malformed spec.toml -> single stderr warning naming the spec; other specs still scan; eventual exit code is at least 1.
    - Empty `.speccy/specs/` -> `No checks defined.` to stdout, exit 0.
    - Workspace with specs but no `[[checks]]` -> same as above.
  - Suggested files: `speccy-cli/src/check.rs`, `speccy-cli/tests/check_discovery.rs`

## Phase 2: Shell invoker

- [x] **T-002**: Implement cross-platform shell selection helper
  - Covers: REQ-003
  - Tests to write:
    - Unix target (`cfg!(unix)`): returns `Command::new("sh").arg("-c").arg(<cmd>)`.
    - Windows target (`cfg!(windows)`): returns `Command::new("cmd").arg("/c").arg(<cmd>)`.
    - Working directory is set to the supplied project root on the returned `Command`.
  - Suggested files: `speccy-cli/src/shell.rs`, `speccy-cli/tests/shell.rs`

- [x] **T-003**: Implement live-streaming child execution
  - Covers: REQ-003
  - Tests to write:
    - Child inherits parent stdio via `Stdio::inherit()` (no piped capture by default).
    - Returns the child's exit code via `ExitStatus::code()` (or 130 for SIGINT-like signals on Unix; documented edge case).
    - Test with a fixture command (`echo hello`) asserts output reaches captured terminal output.
  - Suggested files: `speccy-cli/src/shell.rs` (extend)

## Phase 3: CHK-ID filtering

- [x] **T-004**: Implement CHK-ID filter and validation
  - Covers: REQ-002
  - Tests to write:
    - `speccy check CHK-001` runs only checks with `id == "CHK-001"` across all specs.
    - Two specs each defining `CHK-001` -> both run.
    - Unknown ID (e.g. `CHK-099`) -> `CheckError::NoCheckMatching`; exit 1; stderr names the missing ID and hints at `speccy status`.
    - Malformed ID (e.g. `FOO`, `chk-1`) -> `CheckError::InvalidCheckIdFormat`; exit 1.
  - Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_id_filter.rs`

## Phase 4: Manual checks

- [x] **T-005**: Implement manual-check rendering
  - Covers: REQ-005
  - Tests to write:
    - `kind = "manual"` -> prints `==> CHK-NNN (SPEC-NNNN, manual):` + prompt + `<-- CHK-NNN MANUAL (verify and proceed)`.
    - Any kind with `prompt` and no `command` is also treated as manual.
    - No subprocess is spawned for manual checks.
    - Manual checks never affect exit code.
    - A check with both `command` AND `prompt` prefers `command` and prints a stderr warning naming the offending check ID.
  - Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_manual.rs`

## Phase 5: Output and exit-code aggregation

- [x] **T-006**: Implement header / footer per check and final summary
  - Covers: REQ-006
  - Tests to write:
    - Header `==> CHK-NNN (SPEC-NNNN): <proves>` printed before each executable check.
    - Footer `<-- CHK-NNN PASS` on exit 0; `<-- CHK-NNN FAIL (exit N)` on non-zero.
    - Final summary `<N> passed, <M> failed, <K> manual` as the last stdout line.
    - Empty workspace skips the summary; prints `No checks defined.` instead.
  - Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_output_format.rs`

- [x] **T-007**: Implement run-all + first-non-zero exit code aggregation
  - Covers: REQ-004
  - Tests to write:
    - Three checks (pass, fail-2, fail-1) -> all three run; exit code is 2.
    - Three passing checks -> exit code 0.
    - One pass + one manual + one fail-1 -> exit code 1.
    - Manual checks never change the exit code.
    - Malformed spec.toml warning contributes exit code 1 if no check produces a higher code.
  - Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_exit_code.rs`

## Phase 6: CLI wiring

- [x] **T-008**: Wire `speccy check [CHK-ID]` into the binary
  - Covers: REQ-001..REQ-006
  - Tests to write:
    - `speccy check` runs from any cwd inside a speccy workspace.
    - `speccy check` from outside a workspace -> `CheckError::ProjectRootNotFound`; exit 1.
    - End-to-end via `assert_cmd` in a tmpdir with fixture spec.tomls (mix of test and manual kinds).
  - Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/check.rs`, `speccy-cli/tests/integration_check.rs`

## Phase 7: Cross-platform integration

- [x] **T-009**: Cross-platform integration smoke test
  - Covers: REQ-003
  - Tests to write:
    - On Unix: a fixture `command = "echo hello"` runs and produces `hello` on stdout, exit 0.
    - On Windows: same fixture runs via `cmd /c echo hello` and produces `hello`.
    - CI runs the suite on both Linux and Windows runners.
  - Suggested files: `speccy-cli/tests/integration_check.rs` (extend with cfg-gated assertions), `.github/workflows/ci.yml` (Windows runner if not yet wired; otherwise defer to a CI-setup task in a later spec)
