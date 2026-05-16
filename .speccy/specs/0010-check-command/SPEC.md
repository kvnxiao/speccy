---
id: SPEC-0010
slug: check-command
title: speccy check -- execute proofs, surface results, never persist
status: implemented
created: 2026-05-11
---

# SPEC-0010: speccy check

## Summary

`speccy check` executes the proof obligations declared in `spec.toml`
files across the workspace. Executable checks (`kind = "test"`,
`kind = "command"`, or any free-form kind with a `command` field)
run through the project shell. Manual checks (`kind = "manual"` or
any kind with a `prompt` field but no `command`) print their prompt
and exit advisory.

The command never persists execution results. CI (which calls
`speccy verify` in SPEC-0012, which wraps this same execution path)
is the authoritative place for "did this pass on HEAD?" Local runs
are for the dev loop.

Output is live-streamed: stdout and stderr from each check go
directly to the terminal as the check runs. Speccy adds a one-line
header before each check and a one-line footer after. A final
summary prints `<N> passed, <M> failed, <K> manual`.

The exit code is 0 if every executable check passed, otherwise the
first non-zero exit code encountered. All checks run regardless of
earlier failures (run-all, not fail-fast) so the developer sees the
full picture in one invocation.

## Goals

<goals>
- Mechanical proof execution that matches each project's existing
  test/command tooling without inventing a new runner.
- Live output (no buffering) so long-running checks are observable.
- Deterministic exit-code semantics for CI consumption (via
  `speccy verify` in SPEC-0012).
- Cross-platform: identical behaviour on Unix (`sh -c`) and Windows
  (`cmd /c`).
- Zero persistence -- no result files, no caches.
</goals>

## Non-goals

<non-goals>
- No timeouts. If a check hangs, the developer Ctrl+C's it.
- No parallelism in v1. Checks run serially.
- No internal test runner. We shell out to whatever the project
  uses (`cargo test`, `pytest`, `go test`, etc.).
- No detection of which checks "changed" since the last run. Always
  run all (or the selected CHK-ID).
- No record of execution results. CI artifact storage handles that
  layer.
</non-goals>

## User stories

<user-stories>
- As a developer mid-task, I want one command that runs every check
  for the work I just did, with live output so I can watch slow
  tests progress.
- As a CI maintainer, I want `speccy verify` (which wraps check
  execution) to exit non-zero if any check fails, with the first
  non-zero exit code surfaced.
- As an agent writing implementations, I want a way to print manual
  check prompts so I know which checks need human attention.
- As a developer with a flaky check, I want `speccy check CHK-001`
  to run just that one without waiting on the rest.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Check discovery

Collect every check from every spec's `spec.toml`.

<done-when>
- The command consumes `speccy_core::workspace::scan` (SPEC-0004) to
  discover specs.
- For each parsed `spec.toml`, every `[[checks]]` entry is collected
  into a single workspace-wide check list.
- Each collected check carries its `(spec_id, check_id, kind,
  command, prompt, proves)` shape so manual / executable routing
  and ID filtering work.
- Specs whose `spec.toml` failed to parse are skipped; a single
  stderr warning per spec is printed. Other specs still scan.
- Empty workspaces (no specs, or no checks across specs) print
  `No checks defined.` and exit 0.
</done-when>

<behavior>
- Given two specs (SPEC-0001 with three checks, SPEC-0002 with
  three checks), when `speccy check` runs, then all six are queued
  in `(spec_id ascending, declared check order)` order.
- Given `.speccy/specs/` is empty or only contains specs with no
  `[[checks]]` entries, then stdout prints `No checks defined.` and
  exit code is 0.
- Given one of three spec.toml files is malformed, then stderr
  contains a one-line warning naming the offending spec; checks
  from the other two specs still run; the eventual exit code is
  non-zero (a malformed spec.toml is a real error).
</behavior>

<scenario id="CHK-001">
speccy check enumerates every [[checks]] across all spec.toml files in (spec_id ascending, declared check order) order.
</scenario>

<scenario id="CHK-002">
Empty workspace prints 'No checks defined.' and exits 0; malformed spec.toml is skipped with a stderr warning and contributes exit code 1.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: CHK-ID filtering

Restrict execution to a single check ID when one is supplied.

<done-when>
- `speccy check CHK-NNN` runs only checks whose `id == "CHK-NNN"`
  across every spec.
- If multiple specs each define `CHK-NNN`, ALL of them run
  (ARCHITECTURE.md scopes CHK IDs to specs; cross-spec collision is
  legitimate, not an error).
- If no check in any spec.toml matches the supplied ID, exit code
  is 1 and stderr names the missing ID plus suggests `speccy
  status` to list specs.
- If the argument doesn't match the format `CHK-\d{3,}`, exit code
  is 1 with a clear "not a valid check ID" message.
</done-when>

<behavior>
- Given SPEC-0001 has CHK-001 and SPEC-0003 has CHK-001 (both
  legitimate; scoped per spec), when `speccy check CHK-001` runs,
  then both matching checks execute in spec-ID ascending order.
- Given no spec has CHK-099, when `speccy check CHK-099` runs, then
  exit code is 1 and stderr contains the string `CHK-099` and a
  hint.
- Given `speccy check FOO`, exit code is 1 with a format error.
</behavior>

<scenario id="CHK-003">
- Given SPEC-0001 has CHK-001 and SPEC-0003 has CHK-001 (both
  legitimate; scoped per spec), when `speccy check CHK-001` runs,
  then both matching checks execute in spec-ID ascending order.
- Given no spec has CHK-099, when `speccy check CHK-099` runs, then
  exit code is 1 and stderr contains the string `CHK-099` and a
  hint.
- Given `speccy check FOO`, exit code is 1 with a format error.

speccy check CHK-NNN runs only matching IDs (across all specs where that ID exists); unknown ID exits 1; malformed ID format exits 1.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Shell execution and live streaming

Execute commands via the project shell with stdout/stderr inheriting
the parent.

<done-when>
- Unix: each check's `command` is invoked as `sh -c "<command>"`.
- Windows: invoked as `cmd /c "<command>"`.
- Working directory is the project root (the directory containing
  `.speccy/`).
- Process stdout and stderr are inherited (not piped), so output
  reaches the terminal as the child writes it.
- Before each check: print `==> CHK-NNN (SPEC-NNNN): <proves>`.
- After each check: print
  `<-- CHK-NNN PASS` (exit 0) or `<-- CHK-NNN FAIL (exit N)`
  (non-zero).
</done-when>

<behavior>
- Given a check with `command = "cargo test -p foo"` on Linux,
  then `sh -c "cargo test -p foo"` is spawned with cwd = project
  root.
- Given the same check on Windows, then `cmd /c "cargo test -p
  foo"` is spawned.
- Given a check that prints to stdout in slow chunks, the output
  appears in the terminal as it's produced (no buffering until
  completion).
- Given a check exiting with code 2, the footer reads `<-- CHK-NNN
  FAIL (exit 2)`.
</behavior>

<scenario id="CHK-004">
Unix uses sh -c; Windows uses cmd /c; working directory is the project root containing .speccy/.
</scenario>

<scenario id="CHK-005">
Child stdout/stderr stream live to the terminal via inherited stdio; speccy prints header (==>) and footer (<--) lines around each executable check.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Exit code semantics

Run every check; exit with the first non-zero exit code encountered.

<done-when>
- All executable checks run to completion regardless of earlier
  failures.
- The recorded exit code is the first non-zero exit code from any
  executable check.
- If every executable check passes (or there are none), exit code
  is 0.
- Manual checks never affect the exit code.
- `SpecParseError` warnings during discovery contribute exit code
  1 if no check sets a higher non-zero code.
</done-when>

<behavior>
- Given three checks (pass, fail-2, fail-1), then all three run and
  the final exit code is 2.
- Given three passing checks, then exit code is 0.
- Given two executable checks and one manual check between them,
  both executable checks run, the manual check prints its prompt,
  and the exit code reflects only the executable checks.
</behavior>

<scenario id="CHK-006">
- Given three checks (pass, fail-2, fail-1), then all three run and
  the final exit code is 2.
- Given three passing checks, then exit code is 0.
- Given two executable checks and one manual check between them,
  both executable checks run, the manual check prints its prompt,
  and the exit code reflects only the executable checks.

All executable checks run regardless of earlier failures; exit code is the first non-zero exit code from any check; manual checks don't affect exit code.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Manual check rendering

Print the prompt for kind = `manual` (or any check with `prompt`
but no `command`); never execute them.

<done-when>
- A manual check prints `==> CHK-NNN (SPEC-NNNN, manual):` on its
  header line, followed by the `prompt` text verbatim on the next
  lines.
- A trailing line `<-- CHK-NNN MANUAL (verify and proceed)` follows.
- The check does not spawn a subprocess.
- The check does not affect the exit code.
- A check that has both `command` and `prompt` is a lint failure
  upstream (VAL-002 / VAL-003 in SPEC-0003). At runtime, `speccy
  check` prefers `command` and prints a stderr warning naming the
  check.
</done-when>

<behavior>
- Given a check with `kind = "manual"` and `prompt = "Sign up via
  the UI; confirm duplicate email shows the error toast."`, when
  it runs, then stdout contains both the prompt text verbatim and
  the header + footer lines.
- Given a check with `kind = "property"` (free-form) and a `prompt`
  field but no `command`, it is treated as manual.
</behavior>

<scenario id="CHK-007">
- Given a check with `kind = "manual"` and `prompt = "Sign up via
  the UI; confirm duplicate email shows the error toast."`, when
  it runs, then stdout contains both the prompt text verbatim and
  the header + footer lines.
- Given a check with `kind = "property"` (free-form) and a `prompt`
  field but no `command`, it is treated as manual.

Manual checks (kind=manual or any kind with prompt but no command) print the prompt and a MANUAL footer; never spawn subprocesses; never affect exit code.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Output format and summary

Print a deterministic final summary after every run.

<done-when>
- After all checks complete, print
  `<N> passed, <M> failed, <K> manual` on its own line as the last
  line of output before the process exits.
- The summary appears regardless of exit code.
- Empty-workspace case (no checks at all) prints `No checks
  defined.` instead of a summary line.
</done-when>

<behavior>
- Given five checks (3 pass, 1 fail, 1 manual), the last stdout
  line is `3 passed, 1 failed, 1 manual`.
- Given zero checks, the only stdout line is `No checks defined.`
  and exit code is 0.
</behavior>

<scenario id="CHK-008">
- Given five checks (3 pass, 1 fail, 1 manual), the last stdout
  line is `3 passed, 1 failed, 1 manual`.
- Given zero checks, the only stdout line is `No checks defined.`
  and exit code is 0.

Final summary '<N> passed, <M> failed, <K> manual' is the last stdout line regardless of pass/fail outcome; empty workspace prints 'No checks defined.' instead.
</scenario>

</requirement>

## Design

### Approach

The command lives in `speccy-cli/src/check.rs`. It consumes
`speccy_core::workspace::scan` (from SPEC-0004) to discover specs.
Shell selection is compile-time via `cfg!(unix)` / `cfg!(windows)`,
encapsulated in a small `speccy-cli/src/shell.rs` helper.

Execution uses `std::process::Command` with inherited stdio so
output streams live without explicit pipe forwarding.

The command produces no record. Output is the only artifact.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Compile-time shell selection

**Status:** Accepted
**Context:** Unix and Windows have different default shells for
"run this command string." We must pick one at the call site;
runtime detection just adds cost without changing behaviour.
**Decision:** `cfg!(unix)` -> `sh -c "..."`; `cfg!(windows)` ->
`cmd /c "..."`. No environment-variable override in v1.
**Alternatives:**
- Runtime detection via `std::env::consts::OS` -- rejected.
  Equivalent behaviour with extra runtime cost.
- `SPECCY_SHELL` env var override -- rejected. Configuration
  surface bloat for v1.
**Consequences:** Users on Windows who prefer PowerShell must write
`cmd`-compatible commands or wait for a future override knob.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Run-all-then-report, not fail-fast

**Status:** Accepted
**Context:** Two schedules are reasonable: stop on first failure
(fail-fast) or run everything and report (run-all).
**Decision:** Run all checks regardless of earlier failures; the
recorded exit code is the first non-zero exit code observed.
**Alternatives:**
- Fail-fast -- rejected. Local dev often wants to see *all*
  failures in one invocation (matches `cargo test` default).
- Configurable -- rejected. Adds a knob without a clear use case.
**Consequences:** Long invocations don't short-circuit. Users with
slow check suites run `speccy check CHK-NNN` to scope to one.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: CHK IDs are spec-scoped; cross-spec collision is legitimate

**Status:** Accepted
**Context:** ARCHITECTURE.md scopes `CHK-NNN` IDs to specs. SPEC-0001 and
SPEC-0003 might both define `CHK-001`. Treating IDs as global
would force disambiguation.
**Decision:** `speccy check CHK-NNN` matches every spec where
`CHK-NNN` exists; all matches execute in spec-ID order. Not an
error.
**Alternatives:**
- Require `speccy check SPEC-NNNN/CHK-NNN` for disambiguation --
  rejected for v1. Verbose; same default behaviour either way.
- Error on cross-spec ID collision -- rejected. IDs are scoped by
  design.
**Consequences:** A future flag (`--spec SPEC-NNNN`) could narrow
selection; not needed for v1.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: No execution records (per ARCHITECTURE.md)

**Status:** Accepted
**Context:** ARCHITECTURE.md "Checks / No freshness, no records"
explicitly rejects committing check-execution artifacts.
**Decision:** `speccy check` writes nothing. Output goes to the
terminal; CI artifact storage handles persistence.
**Alternatives:**
- Write a `.speccy/check_results.json` -- rejected per ARCHITECTURE.md.
**Consequences:** The proof-chain in v1 is: SPEC.md says what
matters; spec.toml says what checks prove it; running them is
on-demand.
</decision>

### Interfaces

```rust
pub fn run(args: CheckArgs) -> Result<i32, CheckError>;

pub struct CheckArgs {
    pub id: Option<String>,           // e.g. Some("CHK-001")
}

pub enum CheckError {
    InvalidCheckIdFormat { arg: String },
    NoCheckMatching { id: String },
    ProjectRootNotFound,
    ChildSpawn(std::io::Error),
}
```

CLI mapping:
- `Ok(exit_code)` -> propagate as the process exit code.
- `InvalidCheckIdFormat`, `NoCheckMatching`, `ProjectRootNotFound`
  -> exit 1.
- `ChildSpawn(_)` -> exit 2.

### Data changes

- New `speccy-cli/src/check.rs` (command logic).
- New `speccy-cli/src/shell.rs` (compile-time shell selection).
- Reuses `speccy_core::workspace::scan` from SPEC-0004.

### Migration / rollback

Greenfield command. Rollback via `git revert`. Depends on SPEC-0004
landing first.

## Open questions

- [ ] Should `speccy check --list` print the discovered check
  inventory without running anything? Useful for debugging large
  workspaces. Defer; small follow-up if asked.
- [ ] Should `--spec SPEC-NNNN` scope check execution to one spec?
  Useful when a project grows past ~50 checks. Defer; not v1.
- [ ] Should manual-check prompts include some structured output
  (a markdown checkbox, say) so agents reading stdout can record
  their conclusion? Probably yes when SPEC-0013 personas learn to
  consume it. Defer.

## Assumptions

<assumptions>
- `sh` is on PATH on Unix; `cmd` is on PATH on Windows. True on
  any reasonable target.
- The project shell can interpret arbitrary command strings from
  `spec.toml`. Users authoring checks own their shell quoting.
- `std::process::Command::status()` with inherited stdio streams
  output live on both platforms.
- Single non-zero exit codes can be propagated as-is; speccy does
  not transform exit codes outside its own error categories.
</assumptions>

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
| 2026-05-13 | human/kevin  | Filter by spec status: `dropped`/`superseded` checks are skipped; `Fail` outcomes on `in-progress` specs are reported as IN-FLIGHT and do not gate the exit code (only `implemented` failures gate). Summary line gains an `in-flight` count. |
</changelog>

## Notes

`speccy verify` (SPEC-0012) is the CI gate that wraps this command
with lint output and a binary exit code. SPEC-0012 should reuse this
command's execution logic rather than reimplementing it.

The output header/footer convention (`==> CHK-NNN`, `<-- CHK-NNN
PASS|FAIL`) is a small but durable interface. Skill prompts and
harnesses may key off these markers; changing them is a breaking
change.
