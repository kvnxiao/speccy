---
id: SPEC-0012
slug: verify-command
title: speccy verify -- CI gate composing lint + check execution
status: implemented
created: 2026-05-11
---

# SPEC-0012: speccy verify

## Summary

`speccy verify` is the CI gate. It composes SPEC-0003's lint
engine and SPEC-0010's check execution into a single command with
binary exit-code semantics: 0 if lint has no Error-level
diagnostics AND every executable check passes; 1 otherwise.

Live output streams to **stderr** (so CI logs show progress as
checks run). **stdout** is reserved for either a final summary
(text mode) or structured JSON (`--json`). The stdout/stderr
split lets harnesses pipe stdout into a JSON consumer while still
seeing the streaming output in CI logs.

Verify never flakes on its own state. Speccy's own logic is
deterministic; only the underlying checks (which speccy doesn't
introspect) can vary. Verify reporting the same outcome on two
back-to-back runs (with no filesystem change and identical check
behaviour) is part of the contract.

## Goals

- One binary exit code for CI: 0 = pass, 1 = fail.
- Reuse SPEC-0010's execution logic via a captured-output library
  API; don't reimplement check execution.
- Stream output live to stderr (CI sees progress); stdout reserved
  for summary or JSON.
- Stable JSON contract (`schema_version: 1`) for harness
  consumption.
- Deterministic: speccy verify's own behaviour is reproducible
  given identical workspace state and identical check outputs.

## Non-goals

- No partial-pass states. Verify is binary.
- No parallelism in v1. Checks run serially (matches SPEC-0010).
- No retries on flaky checks. The check author owns flake (DEC-04).
- No persistence of verify results. CI artifact storage handles
  that layer.

## User stories

- As a CI maintainer, I want one command whose exit code I can
  trust for the build gate.
- As an agent at the end of an implementation loop, I want to
  invoke `speccy verify` and know whether to flip a task to `[?]`.
- As a developer running locally, I want live output for slow
  checks AND a final summary I can scan.
- As a harness writer, I want `speccy verify --json` to emit a
  structured report I can parse for per-check pass/fail without
  re-running the checks.

## Requirements

<requirement id="REQ-001">
### REQ-001: Lint integration

Run `speccy_core::lint::run` across the workspace; partition
diagnostics by severity.

**Done when:**
- Discovers the project root via `workspace::find_root` (SPEC-0004).
- Scans the workspace via `workspace::scan`.
- Builds a `lint::Workspace` (SPEC-0003 REQ-006) and calls
  `lint::run`.
- Partitions the returned diagnostics by `Level` into errors,
  warnings, info.
- Lint runs every time `speccy verify` is invoked; it is never
  skipped on the assumption that checks would have failed.

**Behavior:**
- Given a workspace with two `Level::Error` diagnostics and one
  `Level::Warn`, the resulting structured output contains all
  three diagnostics in the appropriate severity buckets.
- Given an empty workspace (no specs), lint runs and returns
  empty buckets.

<scenario id="CHK-001">
- Given a workspace with two `Level::Error` diagnostics and one
  `Level::Warn`, the resulting structured output contains all
  three diagnostics in the appropriate severity buckets.
- Given an empty workspace (no specs), lint runs and returns
  empty buckets.

speccy verify runs lint::run; partitions diagnostics by Level into errors / warnings / info; lint runs every time verify is invoked.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Check execution

Run every executable check and capture structured results, with
output streamed to stderr.

**Done when:**
- Enumerates all executable checks from every parsed `spec.toml`
  in the workspace (same discovery as SPEC-0010 REQ-001).
- Executes each check via the project shell. Child stdio is piped
  (not inherited) and the captured output is forwarded ("tee'd")
  to **stderr** as the child writes it.
- Each check produces a `CheckResult { spec_id, check_id, kind,
  outcome, exit_code, duration_ms }` where `outcome` is one of
  `Pass`, `Fail`, `Manual`.
- Manual checks (`kind = "manual"` or any kind with a `prompt`
  but no `command`) print their prompt to stderr; capture the
  outcome as `Manual`; never affect the exit code.
- All executable checks run regardless of earlier failures
  (run-all, matches SPEC-0010 DEC-002).

**Behavior:**
- Given three executable checks (pass, fail-2, fail-1), all three
  run; CheckResults capture per-check exit codes; the first
  non-zero exit code (2) is the candidate for verify's exit code.
- Given a slow check that prints incrementally, its output
  appears on stderr live as the child writes it (not buffered
  until completion).
- Given two specs each defining `CHK-001` (legitimate scoping per
  SPEC-0010 DEC-003), both run.
- Given a manual check between two executable checks, the manual
  prompt prints to stderr; the executable checks both run and
  contribute to the exit code; the manual does not.

<scenario id="CHK-002">
Executable checks run via the captured-execution API; child output streams live to stderr (not stdout); structured CheckResult per check captures spec_id, check_id, kind, outcome, exit_code, duration_ms.
</scenario>

<scenario id="CHK-003">
All executable checks run regardless of earlier failures; spec-scoped CHK-IDs duplicated across specs all execute; manual checks emit prompts and don't affect exit code.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Binary exit code

Compose lint and check results into a single 0-or-1 exit code.

**Done when:**
- Exit code is `0` if AND only if both hold:
  - `lint.errors` is empty (warnings and info do not count).
  - Every executable check produced `CheckOutcome::Pass`.
- Otherwise, exit code is `1`.
- The exit code does not encode the number of failures (one error
  -> 1; ten errors -> 1).
- The exit code is deterministic for identical workspace state
  and identical check exit codes.

**Behavior:**
- Clean lint + 3 passing checks -> exit 0.
- 1 lint error + 3 passing checks -> exit 1.
- Clean lint + 1 failing check -> exit 1.
- Lint warnings/info but no errors + all checks pass -> exit 0
  (warnings and info never fail).
- Empty workspace (no specs, no checks) -> exit 0.

<scenario id="CHK-004">
- Clean lint + 3 passing checks -> exit 0.
- 1 lint error + 3 passing checks -> exit 1.
- Clean lint + 1 failing check -> exit 1.
- Lint warnings/info but no errors + all checks pass -> exit 0
  (warnings and info never fail).
- Empty workspace (no specs, no checks) -> exit 0.

Exit code is 0 iff lint.errors is empty AND every executable check passed; warnings and info never fail; exit code is deterministic given identical workspace state.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Text mode summary

In text mode, print a final summary to stdout after the live
stderr output completes.

**Done when:**
- After lint and check execution finish, stdout receives exactly
  three lines as the last output:
  - `Lint: <E> errors, <W> warnings, <I> info`
  - `Checks: <P> passed, <F> failed, <M> manual`
  - `verify: PASS` (when exit code 0) or `verify: FAIL` (when
    exit code 1)
- Stderr received the live check output (per REQ-002) including
  the per-check headers and footers from SPEC-0010 conventions.
- The summary lines are the LAST output on stdout.

**Behavior:**
- Given a clean workspace with three passing checks, stdout's
  last three lines are
  `Lint: 0 errors, 0 warnings, 0 info`,
  `Checks: 3 passed, 0 failed, 0 manual`,
  `verify: PASS`.
- Given one lint error and two passing + one failing check,
  stdout shows
  `Lint: 1 errors, 0 warnings, 0 info`,
  `Checks: 2 passed, 1 failed, 0 manual`,
  `verify: FAIL`.

<scenario id="CHK-005">
- Given a clean workspace with three passing checks, stdout's
  last three lines are
  `Lint: 0 errors, 0 warnings, 0 info`,
  `Checks: 3 passed, 0 failed, 0 manual`,
  `verify: PASS`.
- Given one lint error and two passing + one failing check,
  stdout shows
  `Lint: 1 errors, 0 warnings, 0 info`,
  `Checks: 2 passed, 1 failed, 0 manual`,
  `verify: FAIL`.

Text mode prints Lint / Checks / verify summary lines to stdout as the LAST three lines; stderr received the live streamed output and per-check headers/footers.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: JSON output

In `--json` mode, emit a structured report to stdout following the
established envelope.

**Done when:**
- Output begins with `"schema_version": 1` and includes
  `"repo_sha": <string>` (same convention as SPEC-0004 DEC-003;
  empty string if git is unavailable).
- Includes `"lint": { errors, warnings, info }` with structured
  `Diagnostic` objects (matching SPEC-0004 DEC-002 -- structured,
  not strings).
- Includes `"checks": [ ... ]` with per-check objects:
  `{ spec_id, check_id, kind, outcome, exit_code?, duration_ms? }`.
- Includes `"summary": { lint: {errors, warnings, info}, checks:
  {passed, failed, manual} }`.
- Includes `"passed": <bool>` mirroring the exit code (true iff
  exit code 0).
- Output is pretty-printed.
- Output is byte-deterministic across runs given identical
  workspace state and identical check exit codes.

**Behavior:**
- Two back-to-back `speccy verify --json` runs with no filesystem
  change and identical check outcomes produce byte-identical
  stdout.
- A workspace with lint errors and check failures produces JSON
  with `"passed": false`, populated `lint.errors`, and `checks`
  entries with `"outcome": "Fail"`.
- An empty workspace produces JSON with empty arrays throughout
  and `"passed": true`.

<scenario id="CHK-006">
speccy verify --json emits schema_version=1, repo_sha, structured lint and checks blocks, summary, and passed bool; pretty-printed; byte-identical across runs with same workspace state and check outcomes.
</scenario>

<scenario id="CHK-007">
JSON 'passed' field is true iff exit code is 0 across lint-only failures, check-only failures, both, and clean states.
</scenario>

</requirement>

## Design

### Approach

The command lives in `speccy-cli/src/verify.rs`. Flow per
invocation:

1. Discover project root (`workspace::find_root`).
2. Scan workspace (`workspace::scan`).
3. Run lint (`lint::run`).
4. Enumerate executable checks; execute them via the captured-
   execution library API (see DEC-001).
5. Aggregate exit code per REQ-003.
6. Render text summary OR JSON per REQ-004/REQ-005.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Captured check-execution as a library function

**Status:** Accepted
**Context:** SPEC-0010 ships a CLI command that inherits stdio
and returns an exit code. Verify needs structured per-check
results AND to redirect output from stdout (which carries the
summary or JSON) to stderr.
**Decision:** Add
`speccy_core::exec::run_checks_captured(checks, project_root) ->
Vec<CheckResult>` which:
- Pipes child stdio (not inheritance).
- Tees forwarded output to stderr as the child writes.
- Returns structured per-check results.
SPEC-0010's CLI command can either retain inherited stdio (its
current contract) or migrate to this captured API internally;
that's an implementation choice that does not change SPEC-0010's
public surface.
**Alternatives:**
- Run `speccy check` as a subprocess and parse its output --
  rejected. Fragile, slow, awkward error handling.
- Reimplement execution in SPEC-0012 from scratch -- rejected.
  Duplication.
**Consequences:** SPEC-0010's implementation grows a library
function its spec didn't explicitly require. The CLI surface of
SPEC-0010 is unchanged.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Live output to stderr; stdout reserved for summary/JSON

**Status:** Accepted
**Context:** Verify needs both live output (for CI logs) and
structured stdout (for JSON consumers). Mixing them on one stream
breaks downstream parsing.
**Decision:** Stream child output to stderr; emit summary or JSON
to stdout.
**Alternatives:**
- Both on stdout -- rejected. Breaks JSON parsing.
- Both on stderr; nothing on stdout -- rejected. JSON consumers
  expect data on stdout.
- Stdout streams in text mode; switches to silent then emits JSON
  in --json mode -- rejected. Asymmetric behaviour by flag is
  confusing.
**Consequences:** CI logs show streamed output; harnesses can
`speccy verify --json | jq ...` without contamination.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Lint errors and check failures both fail; warnings/info don't

**Status:** Accepted (per SPEC-0003 DEC-003 severity contract)
**Context:** SPEC-0003 hardcodes severity per code. Verify reads
that severity and applies the pass/fail rule.
**Decision:** Exit 0 iff `lint.errors` is empty AND every
executable check passed. Warnings and info never fail the gate.
**Alternatives:**
- Configurable severity threshold -- rejected. Matches the "no
  `--strict` mode" stance from ARCHITECTURE.md and SPEC-0003 DEC-003.
**Consequences:** Projects that want stricter gates must propose
severity changes upstream (SPEC-0003 amendments + the lint
stability registry).
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: No retries; check authors own flake

**Status:** Accepted
**Context:** Flaky checks could be retried, but retries mask real
issues.
**Decision:** Run each check exactly once.
**Alternatives:**
- Retry N times -- rejected. Hides flake; bloats CI runtime.
**Consequences:** Flaky checks fail verify. Encourages root-cause
fixes over symptomatic suppression.
</decision>

### Interfaces

```rust
// speccy-core additions
pub mod exec {
    pub fn run_checks_captured(
        checks: &[CheckSpec],
        project_root: &Path,
    ) -> Vec<CheckResult>;
}

pub struct CheckSpec {
    pub spec_id: String,
    pub check_id: String,
    pub kind: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub proves: String,
}

pub struct CheckResult {
    pub spec_id: String,
    pub check_id: String,
    pub kind: String,
    pub outcome: CheckOutcome,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

pub enum CheckOutcome { Pass, Fail, Manual }

// speccy binary
pub fn run(args: VerifyArgs) -> Result<i32, VerifyError>;

pub struct VerifyArgs { pub json: bool }

pub enum VerifyError {
    ProjectRootNotFound,
    Workspace(WorkspaceError),
    Exec(std::io::Error),
}
```

### Data changes

- New `speccy-core/src/exec.rs` (captured-execution API).
- New `speccy-cli/src/verify.rs` (command logic).
- New `speccy-cli/src/verify_output.rs` (text + JSON
  renderers).

### Migration / rollback

Greenfield command. Rollback via `git revert`. Depends on
SPEC-0003 (lint) and SPEC-0010 (check execution) -- both already
deepened.

## Open questions

- [ ] Should `--json` include `duration_ms` per check by default,
  or only when `--timing` is passed? Including by default is more
  useful (zero cost). Defer to implementer.
- [ ] Should verify support `--lint-only` or `--checks-only`
  flags? Not v1; users call `speccy status` (lint only) or
  `speccy check` (checks only) directly.
- [ ] Should the JSON envelope include start/end timestamps of
  the run? Useful for CI dashboards. Defer.
- [ ] Should the `verify: PASS|FAIL` text line be the canonical
  "did it pass" signal for scripts that grep stdout, or is the
  exit code sufficient? Both work; exit code is preferred but
  the line is a free safety net.

## Assumptions

- `lint::run` from SPEC-0003 is deterministic.
- `workspace::scan` from SPEC-0004 is deterministic.
- `std::process::Command` with piped stdio can tee output to
  stderr without significant buffering on both Unix and Windows.
- The JSON envelope (`schema_version: 1`, structured
  diagnostics) established in SPEC-0004 is the right shape for
  this command's output too.

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
| 2026-05-13 | human/kevin  | Filter by spec status: `dropped`/`superseded` specs are skipped entirely; `Fail` outcomes on `in-progress` specs are categorised as in-flight and do NOT gate the exit code. Only `Level::Error` lint diagnostics and `Fail` on `implemented` specs trigger `verify: FAIL`. JSON envelope gains `summary.checks.in_flight` and per-check `spec_status` fields; text summary becomes `Checks: P passed, F failed, FL in-flight, M manual`. |
| 2026-05-13 | agent/claude | Lint side of the same status filter: `Level::Error` lint diagnostics on `in-progress` specs are demoted to `Level::Info` before partitioning, so drafted-spec lint noise (e.g. TSK-001 on a TASKS.md whose REQs aren't finalised yet) flows into the info bucket and does not gate `verify`. Workspace-level diagnostics (no `spec_id`) and diagnostics on `implemented` specs keep their original severity. |
</changelog>

## Notes

Verify is the gate every speccy-using project will wire into CI.
Determinism and stable exit-code semantics are the contract --
flakes attributable to verify itself (not to the underlying
checks) would erode trust quickly.

DEC-001 introduces a small but real addition to SPEC-0010's
surface (`run_checks_captured`). When SPEC-0010 is implemented,
the implementer should leave hooks for this captured variant so
SPEC-0012's implementation lands cleanly on top.
