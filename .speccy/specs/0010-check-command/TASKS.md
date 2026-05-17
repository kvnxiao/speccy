---
spec: SPEC-0010
spec_hash_at_generation: 04b5b26cd936e38ad8c59b4f81a4dc852d7a941630c40f2a2302740d96ea183a
generated_at: 2026-05-14T03:25:14Z
---

# Tasks: SPEC-0010 check-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Check discovery

<tasks spec="SPEC-0010">

<task id="T-001" state="completed" covers="REQ-001">
Discover checks via `workspace::scan` (SPEC-0004)

- Suggested files: `speccy-cli/src/check.rs`, `speccy-cli/tests/check_discovery.rs`


<task-scenarios>
  - Enumerate every `[[checks]]` from every parsed `spec.toml`; ordering = `(spec_id ascending, declared check order within spec)`.
  - Each enumerated check carries `{spec_id, check_id, kind, command?, prompt?, proves}`.
  - Malformed spec.toml -> single stderr warning naming the spec; other specs still scan; eventual exit code is at least 1.
  - Empty `.speccy/specs/` -> `No checks defined.` to stdout, exit 0.
  - Workspace with specs but no `[[checks]]` -> same as above.
</task-scenarios>
</task>

## Phase 2: Shell invoker


<task id="T-002" state="completed" covers="REQ-003">
Implement cross-platform shell selection helper

- Suggested files: `speccy-cli/src/shell.rs`, `speccy-cli/tests/shell.rs`

<task-scenarios>
  - Unix target (`cfg!(unix)`): returns `Command::new("sh").arg("-c").arg(<cmd>)`.
  - Windows target (`cfg!(windows)`): returns `Command::new("cmd").arg("/c").arg(<cmd>)`.
  - Working directory is set to the supplied project root on the returned `Command`.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-003">
Implement live-streaming child execution

- Suggested files: `speccy-cli/src/shell.rs` (extend)


<task-scenarios>
  - Child inherits parent stdio via `Stdio::inherit()` (no piped capture by default).
  - Returns the child's exit code via `ExitStatus::code()` (or 130 for SIGINT-like signals on Unix; documented edge case).
  - Test with a fixture command (`echo hello`) asserts output reaches captured terminal output.
</task-scenarios>
</task>

## Phase 3: CHK-ID filtering


<task id="T-004" state="completed" covers="REQ-002">
Implement CHK-ID filter and validation

- Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_id_filter.rs`


<task-scenarios>
  - `speccy check CHK-001` runs only checks with `id == "CHK-001"` across all specs.
  - Two specs each defining `CHK-001` -> both run.
  - Unknown ID (e.g. `CHK-099`) -> `CheckError::NoCheckMatching`; exit 1; stderr names the missing ID and hints at `speccy status`.
  - Malformed ID (e.g. `FOO`, `chk-1`) -> `CheckError::InvalidCheckIdFormat`; exit 1.
</task-scenarios>
</task>

## Phase 4: Manual checks


<task id="T-005" state="completed" covers="REQ-005">
Implement manual-check rendering

- Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_manual.rs`


<task-scenarios>
  - `kind = "manual"` -> prints `==> CHK-NNN (SPEC-NNNN, manual):` + prompt + `<-- CHK-NNN MANUAL (verify and proceed)`.
  - Any kind with `prompt` and no `command` is also treated as manual.
  - No subprocess is spawned for manual checks.
  - Manual checks never affect exit code.
  - A check with both `command` AND `prompt` prefers `command` and prints a stderr warning naming the offending check ID.
</task-scenarios>
</task>

## Phase 5: Output and exit-code aggregation


<task id="T-006" state="completed" covers="REQ-006">
Implement header / footer per check and final summary

- Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_output_format.rs`

<task-scenarios>
  - Header `==> CHK-NNN (SPEC-NNNN): <proves>` printed before each executable check.
  - Footer `<-- CHK-NNN PASS` on exit 0; `<-- CHK-NNN FAIL (exit N)` on non-zero.
  - Final summary `<N> passed, <M> failed, <K> manual` as the last stdout line.
  - Empty workspace skips the summary; prints `No checks defined.` instead.
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-004">
Implement run-all + first-non-zero exit code aggregation

- Suggested files: `speccy-cli/src/check.rs` (extend), `speccy-cli/tests/check_exit_code.rs`


<task-scenarios>
  - Three checks (pass, fail-2, fail-1) -> all three run; exit code is 2.
  - Three passing checks -> exit code 0.
  - One pass + one manual + one fail-1 -> exit code 1.
  - Manual checks never change the exit code.
  - Malformed spec.toml warning contributes exit code 1 if no check produces a higher code.
</task-scenarios>
</task>

## Phase 6: CLI wiring


<task id="T-008" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005 REQ-006">
Wire `speccy check [SELECTOR]` into the binary

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/check.rs`, `speccy-cli/tests/integration_check.rs`


<task-scenarios>
  - `speccy check` runs from any cwd inside a speccy workspace.
  - `speccy check` from outside a workspace -> `CheckError::ProjectRootNotFound`; exit 1.
  - End-to-end via `assert_cmd` in a tmpdir with fixture spec.tomls (mix of test and manual kinds).
</task-scenarios>
</task>

## Phase 7: Cross-platform integration


<task id="T-009" state="completed" covers="REQ-003">
Cross-platform integration smoke test

- Suggested files: `speccy-cli/tests/integration_check.rs` (extend with cfg-gated assertions), `.github/workflows/ci.yml` (Windows runner if not yet wired; otherwise defer to a CI-setup task in a later spec)

<task-scenarios>
  - On Unix: a fixture `command = "echo hello"` runs and produces `hello` on stdout, exit 0.
  - On Windows: same fixture runs via `cmd /c echo hello` and produces `hello`.
  - CI runs the suite on both Linux and Windows runners.
</task-scenarios>
</task>

</tasks>
