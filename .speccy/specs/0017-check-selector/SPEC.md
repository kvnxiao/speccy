---
id: SPEC-0017
slug: check-selector
title: speccy check -- polymorphic positional selector (SPEC / task / CHK)
status: in-progress
created: 2026-05-14
supersedes: []
---

# SPEC-0017: speccy check polymorphic selector

## Summary

Today `speccy check` takes an optional `CHK-NNN` argument. That forces
users (humans and agents) to context-switch from spec/task IDs --
which dominate the rest of the CLI -- into check IDs. The natural
reach for "did my new code's checks pass?" is a spec or task ID, not
a check ID.

This spec replaces `speccy check [CHK-ID]` with `speccy check
[SELECTOR]`, where SELECTOR is one of five distinguishable shapes:

| Invocation | Runs |
|---|---|
| `speccy check` | every check across every spec (unchanged) |
| `speccy check SPEC-NNNN` | every check under that spec |
| `speccy check SPEC-NNNN/CHK-NNN` | one check, spec-qualified |
| `speccy check CHK-NNN` | every spec's `CHK-NNN` (unchanged; per SPEC-0010 DEC-003) |
| `speccy check T-NNN` / `SPEC-NNNN/T-NNN` | every check proving the requirements the task covers |

The dispatch is by prefix-regex on a single positional argument. The
five shapes share the existing `speccy_core::task_lookup` resolver
for task forms (so ambiguity and not-found errors are identical to
`speccy implement` and `speccy review`), and add a thin sibling
parser for spec / CHK forms. No new flags. No new noun in
`.speccy/ARCHITECTURE.md`'s five-noun set.

The downstream execution path (live streaming, exit-code aggregation,
in-flight categorisation for in-progress specs) is unchanged. Only
*selection* changes.

## Goals

- One positional argument accepts all five selector shapes; no flag
  clutter.
- Task forms reuse `task_lookup` verbatim -- no second implementation
  of qualified/unqualified/ambiguous handling.
- Bare `CHK-NNN` cross-spec semantics preserved (SPEC-0010 DEC-003
  stands).
- Errors name the offending input verbatim and list the valid shapes.
- Internal consistency with `speccy implement` / `speccy review`
  (also single polymorphic positional).

## Non-goals

- No new flags. The CLI stays at "ten commands, two optional flags."
- No change to execution semantics (live stream, run-all-then-report,
  exit-code aggregation, in-flight categorisation).
- No new noun in the five-noun set.
- No `--list` mode (lists what would run without executing); deferred
  alongside SPEC-0010's deferred `--list` open question.
- No JSON envelope for `speccy check`; separate concern.
- No deprecation of bare `CHK-NNN`. It stays a first-class shape.

## User stories

- As a developer mid-task on SPEC-0017, I want `speccy check
  SPEC-0017` to run just my spec's checks instead of every check in
  the workspace.
- As a developer who just finished T-002, I want `speccy check T-002`
  to run only the checks proving the requirements that task covers,
  not the workspace-wide set.
- As an agent running an implementer sub-agent, I want `speccy check
  SPEC-NNNN/T-NNN` for explicit scoping when the unqualified `T-NNN`
  would be ambiguous across specs.
- As a CI maintainer, I want `speccy verify`'s embedded `speccy check`
  invocation to continue meaning "run everything" -- bare `speccy
  check` behaviour is unchanged.

## Requirements

### REQ-001: Selector parsing

The CLI accepts five shapes and rejects anything else with a single,
informative error.

**Done when:**
- Absent argument is the `All` selector (today's `speccy check`).
- `SPEC-NNNN/T-NNN` parses as the qualified task shape (regex tested
  first because it is the most specific).
- `SPEC-NNNN/CHK-NNN` parses as the qualified check shape.
- `SPEC-NNNN` parses as the spec shape.
- `T-NNN` parses as the unqualified task shape.
- `CHK-NNN` parses as the unqualified check shape.
- Any other input returns a single `SelectorError::InvalidFormat`
  variant that includes the offending input verbatim and a list of
  the five valid shapes.
- The CLI exits 1 (selector-error class) on parse failure.

**Behavior:**
- Given `speccy check FOO`, exit code is 1 and stderr lists the five
  valid shapes plus the verbatim `FOO`.
- Given `speccy check chk-001`, exit code is 1 (case mismatch ->
  parse failure).
- Given `speccy check SPEC-0010/CHK-001`, parsing succeeds.
- Given `speccy check SPEC-0010/T-002`, parsing succeeds and the
  task fragment is delegated to `task_lookup::parse_ref`.

**Covered by:** CHK-001

### REQ-002: Spec selector executes one spec's checks

A `SPEC-NNNN` selector runs every check (executable and manual)
defined in that spec's `spec.toml`.

**Done when:**
- The named spec is looked up in the workspace; if absent, exit 1
  with `SelectorError::NoSpecMatching { spec_id }` naming the
  missing spec.
- Every `[[checks]]` entry in that spec's `spec.toml` is collected,
  in declared order, and executed via the existing `execute_checks`
  path.
- Status filtering is preserved: a `dropped` or `superseded` spec
  prints a one-line "spec is `<status>`; no checks executed" message
  and exits 0 (the existing skip semantics, made explicit when the
  user named the spec directly).
- In-progress failures are categorised as IN-FLIGHT exactly as today
  (no gating). Implemented failures gate the exit code.
- The summary line totals reflect only the selected spec.

**Behavior:**
- Given SPEC-0010 has 8 checks and SPEC-0011 has 5, `speccy check
  SPEC-0010` runs exactly 8.
- Given SPEC-0010 has status `dropped`, `speccy check SPEC-0010`
  prints the skip message and exits 0; no checks run.
- Given SPEC-9999 does not exist, `speccy check SPEC-9999` exits 1
  with stderr naming `SPEC-9999`.
- Given an in-progress SPEC-0017 with one passing and one failing
  check, `speccy check SPEC-0017` exits 0 and the failing check is
  reported as IN-FLIGHT in the footer and summary.

**Covered by:** CHK-002

### REQ-003: Task selector executes the task's covering checks

A task selector runs every check that proves the requirements the
task covers, transitively via `spec.toml`.

**Done when:**
- The task fragment is resolved via `speccy_core::task_lookup::find`,
  which returns the existing `TaskLocation` (or surfaces
  `LookupError::Ambiguous` / `LookupError::NotFound` /
  `LookupError::InvalidFormat` unchanged).
- For each `REQ-ID` in the resolved task's `covers` field, every
  `CHK-ID` in that `[[requirements]].checks` array is collected.
- Duplicate `CHK-ID`s (a check proving multiple covered requirements)
  appear in the run set exactly once, in first-occurrence declared
  order.
- The selected checks execute via the existing `execute_checks`
  path. In-flight categorisation applies based on the parent spec's
  status (same rule as REQ-002).
- A task with `covers: []` prints "task `<task_ref>` covers no
  requirements; no checks to run" and exits 0.
- A task that exists but whose covered requirements name
  `CHK-ID`s absent from `[[checks]]` is the lint engine's concern
  (SPEC-0003 LNT codes); `speccy check` still runs whatever it found
  and exits 0.

**Behavior:**
- Given SPEC-0010/T-002 covers `REQ-002`, and `REQ-002` maps to
  `[CHK-003]`, then `speccy check SPEC-0010/T-002` runs exactly
  `CHK-003`.
- Given SPEC-0010/T-X covers `REQ-A` -> `[CHK-001, CHK-002]` and
  `REQ-B` -> `[CHK-002, CHK-003]`, then `speccy check SPEC-0010/T-X`
  runs `CHK-001, CHK-002, CHK-003` once each (no duplication).
- Given `T-002` exists in both SPEC-0010 and SPEC-0011, `speccy check
  T-002` exits 1 with the existing `LookupError::Ambiguous` message,
  including copy-pasteable `speccy check SPEC-NNNN/T-002` hints.
- Given `T-099` is in no spec, `speccy check T-099` exits 1 with the
  existing `LookupError::NotFound` message (mentioning `speccy
  status`).
- Given SPEC-0010/T-Y covers `[]`, `speccy check SPEC-0010/T-Y`
  prints the informational message and exits 0.

**Covered by:** CHK-003

### REQ-004: Bare CHK-NNN cross-spec semantics preserved

The unqualified `CHK-NNN` form continues to match across all specs.
The new `SPEC-NNNN/CHK-NNN` form is an *addition*, not a replacement.

**Done when:**
- `speccy check CHK-NNN` selects every spec's `CHK-NNN` (matches the
  existing `id_filter_matches_across_specs` test).
- `speccy check SPEC-NNNN/CHK-NNN` runs only that spec's `CHK-NNN`;
  if the spec lacks that ID, exit 1 with a clear "no `CHK-NNN` in
  `SPEC-NNNN`" message.
- No deprecation warning is printed for bare `CHK-NNN`. SPEC-0010
  DEC-003 stands.
- `speccy check CHK-099` (no match anywhere) exits 1 with the
  existing `NoCheckMatching` wording.

**Behavior:**
- Given SPEC-0001 and SPEC-0003 both define `CHK-001`, `speccy check
  CHK-001` runs both (verbatim existing semantics).
- Given `speccy check SPEC-0003/CHK-001`, only SPEC-0003's `CHK-001`
  runs.
- Given `speccy check SPEC-0001/CHK-099` where SPEC-0001 has no
  `CHK-099`, exit 1 with stderr naming both the spec and the
  missing check.

**Covered by:** CHK-004

### REQ-005: ARCHITECTURE.md and shipped skill docs reflect the new shape

The CLI surface is canonically documented in
`.speccy/ARCHITECTURE.md`. That doc, and any shipped skill prompt
that calls `speccy check`, must reflect the new selector shape so
agents don't get a stale invocation from the doc surface.

**Done when:**
- `.speccy/ARCHITECTURE.md` "CLI Surface" table row for
  `speccy check` changes from `[CHK-ID]` to `[SELECTOR]` with
  indented sub-bullets for each shape, mirroring the style of the
  `speccy plan` and `speccy tasks` rows above it.
- `.speccy/ARCHITECTURE.md` "Execution" code fence in the Checks
  section gains lines for spec, qualified-check, and task forms;
  bare `CHK-NNN` line stays.
- Any shipped skill prompt under `.speccy/skills/` and `skills/` that
  invokes `speccy check CHK-...` is reviewed; updated only if the
  surrounding guidance would otherwise mislead a future agent.
- No new lint codes; no new noun.

**Behavior:**
- `git grep -n "speccy check"` across the repo shows updated
  invocations in human-facing docs.
- The reviewer-docs and reviewer-architecture personas, given the
  diff, can trace every doc change to a REQ in this spec.

**Covered by:** CHK-005

## Design

### Approach

Add a `CheckSelector` type and a `parse_selector` function. The
parser is prefix-regex dispatched, testing the most specific shapes
first:

1. `^(SPEC-\d{4,})/(T-\d{3,})$` -> qualified task
2. `^(SPEC-\d{4,})/(CHK-\d{3,})$` -> qualified check
3. `^SPEC-\d{4,}$` -> spec
4. `^T-\d{3,}$` -> unqualified task
5. `^CHK-\d{3,}$` -> unqualified check

Task forms (1 and 4) construct a `task_lookup::TaskRef` from the
fragment and delegate resolution to `task_lookup::find`. Spec and
check forms have their own collection logic but reuse the existing
`CollectedCheck` type and `execute_checks` function from
`speccy-cli/src/check.rs`.

`CheckArgs.id: Option<String>` is renamed to `CheckArgs.selector:
Option<String>`. The arg name in `clap` becomes `SELECTOR` with a
doc comment that lists the five shapes.

### Decisions

#### DEC-001: Polymorphic positional, not flags

**Status:** Accepted
**Context:** The five accepted shapes could be expressed as flags
(`--spec SPEC-NNNN`, `--task T-NNN`, `--id CHK-NNN`) or as a single
polymorphic positional argument. The rest of the CLI already uses
single polymorphic positionals for `implement` and `review`
(`T-NNN | SPEC-NNNN/T-NNN`). Internal consistency wins over the
mild self-documentation gain of flags.
**Decision:** One positional argument, prefix-regex dispatched.
**Alternatives:**
- Flags (`--spec`, `--task`, `--id`) -- rejected. Three new flags
  for what prefix-regex disambiguates unambiguously; inconsistent
  with `implement` and `review`.
- Subcommands (`speccy check spec SPEC-NNNN`, `speccy check task
  T-NNN`) -- rejected. Verbose; inconsistent with the rest of the
  CLI; doubles the surface area users have to learn.
**Consequences:** `clap --help` must describe all five shapes in
one place. Error messages on parse failure must list the valid
shapes, otherwise the failure mode is opaque.

#### DEC-002: Reuse `task_lookup`; do not generalise it

**Status:** Accepted
**Context:** The task forms (`T-NNN`, `SPEC-NNNN/T-NNN`) share
parsing and resolution with `speccy implement` and `speccy review`.
Two options: (a) reuse `task_lookup` verbatim by parsing the task
fragment of the selector and delegating to `task_lookup::parse_ref`
+ `task_lookup::find`, or (b) generalise `task_lookup` into an
`entity_lookup` module that handles tasks, specs, and checks.
**Decision:** Reuse. `parse_selector` dispatches the task shapes to
`task_lookup`; spec and check shapes get a thin sibling resolver.
**Alternatives:**
- Generalise `task_lookup` into a multi-noun resolver -- rejected.
  Premature: only `check` needs spec / CHK resolution. Generalisation
  is cheap to do later if a third caller appears (e.g. `speccy
  status SPEC-NNNN`).
**Consequences:** `task_lookup` stays focused on tasks. The new
`parse_selector` is a thin wrapper. If future commands need
spec-based resolution, the spec-resolution helper added here can be
promoted to `speccy_core` then.

#### DEC-003: Bare `CHK-NNN` keeps cross-spec semantics

**Status:** Accepted
**Context:** SPEC-0010 DEC-003 established that `CHK-NNN` IDs are
spec-scoped, and `speccy check CHK-NNN` deliberately matches every
spec's `CHK-NNN` -- cross-spec collision is a feature, not an error.
Introducing `SPEC-NNNN/CHK-NNN` makes spec-scoped invocation
available, which raises the question: should bare `CHK-NNN` tighten
to "ambiguous when multiple specs match"?
**Decision:** Bare `CHK-NNN` retains cross-spec match. SPEC-0010
DEC-003 stands; this decision layers `SPEC-NNNN/CHK-NNN` on top
without supersession.
**Alternatives:**
- Tighten bare `CHK-NNN` to error on cross-spec match -- rejected.
  Specs deliberately reuse small `CHK-NNN` numbers (every spec's
  REQ-001 is `CHK-001`); "rerun `CHK-001` across the workspace" is
  a legitimate broadcast. Tightening breaks a working pattern with
  no clear benefit.
- Deprecate bare `CHK-NNN` and require qualification -- rejected.
  Adds typing without value. The existing
  `id_filter_matches_across_specs` test documents cross-spec
  behaviour as intentional.
**Consequences:** No deprecation warning. SPEC-0010 DEC-003 requires
no supersession row. Users who want spec-scoped CHK invocation use
the new `SPEC-NNNN/CHK-NNN` form.

#### DEC-004: Selector errors list valid shapes verbatim

**Status:** Accepted
**Context:** A polymorphic positional fails opaquely if the error
just says "invalid format." Users (and agents) need to know what
shapes exist.
**Decision:** `SelectorError::InvalidFormat` formats with the
offending input verbatim plus a list of the five valid shapes,
modelled after `task_lookup::LookupError::InvalidFormat` but with a
richer message.
**Alternatives:**
- Reuse `task_lookup::LookupError::InvalidFormat` verbatim --
  rejected. That error names only the two task shapes; the check
  selector has five. The wrong message in the wrong context teaches
  the wrong lesson.
**Consequences:** `speccy check FOO` stderr is more informative than
today's terse "expected CHK- followed by 3 or more digits."

### Interfaces

```rust
// speccy-cli/src/check_selector.rs (new module)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckSelector {
    /// No argument; run every check across every spec.
    All,
    /// `SPEC-NNNN`: every check under the named spec.
    Spec { spec_id: String },
    /// `SPEC-NNNN/CHK-NNN`: one check, spec-qualified.
    QualifiedCheck { spec_id: String, check_id: String },
    /// `CHK-NNN`: every spec's `CHK-NNN` (cross-spec match).
    UnqualifiedCheck { check_id: String },
    /// `T-NNN` or `SPEC-NNNN/T-NNN`: checks proving the task's
    /// covered requirements.
    Task(speccy_core::task_lookup::TaskRef),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SelectorError {
    #[error(
        "invalid selector `{arg}`; expected one of: SPEC-NNNN, \
         SPEC-NNNN/CHK-NNN, SPEC-NNNN/T-NNN, CHK-NNN, T-NNN"
    )]
    InvalidFormat { arg: String },
    #[error("no spec `{spec_id}` found in workspace")]
    NoSpecMatching { spec_id: String },
    #[error("no `{check_id}` in `{spec_id}`")]
    NoQualifiedCheckMatching { spec_id: String, check_id: String },
    #[error("task `{task_ref}` covers no requirements; no checks to run")]
    TaskCoversNothing { task_ref: String },
}

pub fn parse_selector(arg: Option<&str>) -> Result<CheckSelector, SelectorError>;
```

`CheckArgs.id` becomes `CheckArgs.selector`. `CheckError` gains:

- `Selector(SelectorError)` -- exit 1 (selector-error class).
- The existing `NoCheckMatching` variant continues to handle bare
  `CHK-NNN` not-found (preserved for REQ-004 behaviour).
- The existing `InvalidCheckIdFormat` variant is **removed**;
  `SelectorError::InvalidFormat` subsumes it.

### Data changes

- New: `speccy-cli/src/check_selector.rs` (selector type, parser).
- Modified: `speccy-cli/src/check.rs` -- parse selector, branch
  before `collect_checks`, then call existing `execute_checks`.
- Modified: `speccy-cli/src/main.rs` -- rename arg `id` to
  `selector`; update `clap` doc-comment to list the five shapes.
- Modified: `speccy-cli/tests/check.rs` -- existing tests using
  `Some("CHK-NNN")` keep passing (semantics unchanged); add tests
  for the new shapes.
- Modified: `.speccy/ARCHITECTURE.md` -- CLI Surface row (line ~141)
  and Execution code fence (lines ~1003-1004) per REQ-005.

### Migration / rollback

This change is **user-facing additive but error-variant-breaking**:

- Every old invocation (`speccy check`, `speccy check CHK-NNN`,
  outside-workspace) keeps working with identical observable
  behaviour. No skill, no script, no CI invocation breaks.
- The `CheckError::InvalidCheckIdFormat` variant is removed in
  favour of `SelectorError::InvalidFormat`. This is a public API
  change for any out-of-tree consumer of `speccy_cli::check`. None
  exist today (the CLI is the only consumer of its own library), so
  this is safe.
- The unit-test `validate_chk_id_format` and its module are deleted;
  selector parsing is tested in the new module.

Rollback is a `git revert`. No on-disk state changes.

## Open questions

- [ ] Should `speccy check SPEC-NNNN/T-NNN` where the task has
      `covers: []` exit 0 (informational) or exit 1 (nudge the
      author to map tasks to requirements)? **Lean 0**: matches the
      "no checks defined" empty-workspace semantics; the lint engine
      (SPEC-0003) is the right place to flag missing coverage.
- [ ] Should `CheckSelector` live in `speccy-core` (next to
      `task_lookup`) or in `speccy-cli`? `task_lookup` lives in core
      because two commands consume it; the selector is currently
      consumed only by `check`. **Lean `speccy-cli`**: promote to
      core when a second consumer appears.
- [ ] Should we ship a `--list` mode that prints what *would* run
      without executing? Useful for large workspaces and for agents
      validating their selector before invoking. SPEC-0010 deferred
      this; still deferred. Revisit when reviewer personas need it.

## Assumptions

- `speccy_core::task_lookup::{parse_ref, find, TaskRef, LookupError,
  TaskLocation}` are stable enough that we reuse them without
  signature changes.
- `clap` 4.x can express the five-shape help text via the
  doc-comment on the renamed `selector` field. If the help text
  needs structured rendering, a `long_about` attribute on the
  subcommand is the escape hatch.
- The five regex shapes are mutually disjoint: a string that matches
  one cannot match another. The dispatch order in `parse_selector`
  (qualified-task before qualified-check before bare spec, etc.) is
  a safety net, not a semantic dependency.
- Spec status filtering (drop, supersede, in-progress) is a property
  of the parent spec, not of the invocation. Selecting a single
  in-progress spec by ID does not "upgrade" its failing checks from
  IN-FLIGHT to FAIL.
- All four hygiene gates (`cargo test --workspace`, `cargo clippy
  ... -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny
  check`) pass after implementation.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-14 | human/kevin  | Initial draft. Polymorphic positional selector replaces CHK-only argument; SPEC-0010 DEC-003 preserved. |

## Notes

This spec touches the CLI surface documented in
`.speccy/ARCHITECTURE.md`. The doc edit is in REQ-005 so it lands
inside this spec's review boundary rather than as a separate
follow-up; the architecture reviewer persona should verify the new
shape matches DEC-001's claim of internal consistency with
`implement` and `review`.

The relationship to SPEC-0010:
- SPEC-0010 DEC-001 (compile-time shell selection) -- untouched.
- SPEC-0010 DEC-002 (run-all-then-report) -- untouched.
- SPEC-0010 DEC-003 (CHK IDs are spec-scoped; cross-spec collision
  is legitimate) -- **preserved**; this spec's DEC-003 makes the
  preservation explicit and adds the qualified form as a sibling
  shape rather than a replacement.
- SPEC-0010 DEC-004 (no execution records) -- untouched.

Future agents reading SPEC-0010 will see it remains the canonical
definition of *how* checks run; this SPEC is the canonical
definition of *which* checks run for a given invocation.
