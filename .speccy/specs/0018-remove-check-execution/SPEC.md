---
id: SPEC-0018
slug: remove-check-execution
title: Remove check execution; checks become validation scenarios
status: implemented
created: 2026-05-15
supersedes: []
---

# SPEC-0018: Remove check execution

## Summary

Speccy currently treats `speccy check` as a small test runner:
`spec.toml` check rows carry `kind`, `command` or `prompt`, and
`proves`; `speccy check` shells out through the host shell; and
`speccy verify` rolls those execution results into its CI gate.

That shape gives the CLI responsibility it cannot actually satisfy.
A command that exits 0 may be vacuous. A command that names a
specific test file drifts whenever the project test layout changes.
Whether a test meaningfully proves behavior is semantic judgment,
and semantic judgment belongs to implementer/reviewer/validator
agents and the project's own CI, not to Speccy's deterministic core.

This spec removes execution from Speccy. Checks become plain English
validation scenarios: durable assertions of behavior that
implementers must satisfy and reviewers must validate. `speccy check`
stays only as a selector-aware renderer for those scenarios, because
that is still useful in agent prompts and CI logs. `speccy verify`
becomes a fast shape validator: it verifies that specs parse, every
requirement has at least one scenario, every referenced scenario
exists, and the workspace contract is internally consistent.

The rest of the sequence depends on this semantic shift:

- **SPEC-0018:** remove execution and collapse check rows to
  `id` + `scenario` while keeping the current `SPEC.md` +
  `spec.toml` two-file carrier.
- **SPEC-0019:** remove the two-file carrier by moving
  requirement/scenario structure into a canonical marker-structured
  `SPEC.md`.
- **SPEC-0020:** apply the same marker-structured carrier to
  `TASKS.md` and `REPORT.md`, without introducing first-class
  handoff structure.

The current `spec.toml` for this spec intentionally remains in the
pre-SPEC-0018 schema so today's Speccy can still parse the workspace.
The implementation of this spec migrates every in-tree `spec.toml`,
including this one, to the new shape.

## Goals

- `speccy check` and `speccy verify` never spawn child processes.
- `spec.toml` `[[checks]]` rows shrink to exactly `id` and
  `scenario`.
- Existing in-tree `spec.toml` files migrate mechanically from
  `kind` / `command` / `prompt` / `proves` to `scenario`.
- `speccy verify` exits non-zero only for malformed or inconsistent
  Speccy artifact shape, not for project test failures.
- CI continues to run project test commands directly, alongside
  `speccy verify`.
- `.speccy/ARCHITECTURE.md` and shipped skills teach the new
  feedback-only contract.

## Non-goals

- No XML or marker-structured Markdown carrier change. SPEC-0019 owns
  that.
- No `speccy verify --exec`, no project test runner wrapper, and no
  configurable command registry.
- No attempt to grade scenario quality mechanically. Reviewers own
  vacuity and coverage judgment.
- No new top-level noun. The existing Check noun remains for now, but
  its payload is an English validation scenario rather than an
  executable command.
- No compatibility shim for the old check row shape after this spec
  lands. Speccy has not shipped v1; the migration can be a hard break.

## User Stories

- As an implementer agent, I want each check to describe behavior in
  English so I can choose the right project test surface without
  deciphering a stale command string.
- As a reviewer-tests persona, I want to compare the diff and the
  test changes against a scenario written before implementation, not
  against a command chosen by the implementer.
- As a CI maintainer, I want `speccy verify` to be a fast structural
  gate that runs beside the real test suite, not instead of it.
- As a developer refactoring tests, I want Speccy artifacts to avoid
  churn when file names or command flags change but the behavior
  contract remains stable.

## Requirements

### REQ-001: Check schema collapses to `id` and `scenario`

`spec.toml` keeps its existing role for one more spec, but each
`[[checks]]` row carries only a stable id and an English validation
scenario.

**Done when:**
- `CheckEntry` in `speccy-core::parse::toml_files` is reduced to
  `{ id: CheckId, scenario: NonEmptyString }`.
- `CheckPayload` and all production references to `kind`, `command`,
  `prompt`, and `proves` are deleted.
- Check deserialization rejects unknown fields via
  `#[serde(deny_unknown_fields)]`.
- Empty or whitespace-only `scenario` values are parse errors naming
  the containing `CHK-NNN`.
- The migration rewrites every in-tree
  `.speccy/specs/**/spec.toml`, including SPEC-0018 and later draft
  specs that still use the pre-SPEC-0018 schema.
- Migration rules:
  - if an old check has `command`, new `scenario = old_proves`;
  - if an old check has `prompt`, new `scenario = old_prompt`;
  - `kind`, `command`, `prompt`, and `proves` are removed.

**Behavior:**
- Given a check row with `id = "CHK-001"` and
  `scenario = "Given ..."`, parsing succeeds.
- Given a check row with a legacy `command` or `prompt` field after
  migration, parsing fails with an unknown-field error.
- Given a check row with `scenario = ""`, parsing fails with an
  empty-scenario error.

**Covered by:** CHK-001

### REQ-002: `speccy check` renders scenarios only

`speccy check [SELECTOR]` keeps SPEC-0017's selector surface but
renders selected scenarios instead of executing commands.

**Done when:**
- `speccy-cli::check::run` has no dependency on `speccy_core::exec`.
- Selector behavior remains unchanged for:
  - no selector;
  - `SPEC-NNNN`;
  - `SPEC-NNNN/CHK-NNN`;
  - `CHK-NNN`;
  - `SPEC-NNNN/T-NNN`;
  - `T-NNN`.
- For each selected scenario, stdout prints:
  `==> CHK-NNN (SPEC-NNNN): <scenario first line>`, followed by
  indented continuation lines for multiline scenarios.
- The old `<-- CHK-NNN PASS|FAIL|IN-FLIGHT` footer is removed.
- The summary line reports only counts:
  `N scenarios rendered across M specs`.
- Exit code is non-zero only for selector, lookup, parse, or
  workspace errors.

**Behavior:**
- Given three specs with two checks each, `speccy check` prints six
  `==>` headers, no child process output, and the count summary.
- Given `speccy check SPEC-9999`, the existing no-matching-spec error
  is preserved.
- Given `speccy check SPEC-0001/T-002`, selected scenarios are derived
  from the task's covered requirements exactly as SPEC-0017 did for
  executable checks.

**Covered by:** CHK-002

### REQ-003: `speccy verify` is shape-only

`speccy verify` validates Speccy artifacts and cross-references. It
does not call `speccy check`, execute scenarios, or infer test
quality.

**Done when:**
- `speccy-cli::verify::run` no longer calls any execution helper.
- Verification walks all specs and reports:
  - parse errors in `SPEC.md`, `TASKS.md`, `REPORT.md`, or
    `spec.toml`;
  - requirements with no scenarios;
  - requirements referencing unknown scenarios;
  - scenario rows not referenced by any requirement;
  - existing stale-task and open-question diagnostics.
- Dropped and superseded specs remain non-gating where previous specs
  already made that distinction.
- Text output ends with:
  `verified N specs, M requirements, K scenarios; E errors`.
- `speccy verify --json` bumps to `schema_version = 2` and removes
  execution-shaped fields (`outcome`, `exit_code`, `duration_ms`).

**Behavior:**
- Given a requirement with an empty `checks` array, verify exits 1 and
  names the requirement.
- Given a requirement that references `CHK-099` with no corresponding
  check row, verify exits 1 and names both ids.
- Given a clean workspace, verify exits 0 without spawning any child
  process.

**Covered by:** CHK-003

### REQ-004: Execution code and tests are deleted

The old execution layer is removed rather than deprecated.

**Done when:**
- `speccy-core/src/exec.rs` is deleted.
- `speccy-core/src/lib.rs` no longer exports `exec`.
- Production source has zero references to `run_checks_captured`,
  `CheckOutcome`, `CheckResult`, `CheckSpec`, or the execution
  `shell_command` helper.
- Tests that only exercised subprocess execution are deleted or
  replaced with renderer/shape tests.
- `cargo build --workspace`, `cargo test --workspace`, and clippy run
  without dead-code warnings from the removed execution surface.

**Behavior:**
- Given the post-spec workspace, `git grep -n "speccy_core::exec"`
  returns no production-source hits.
- Given the post-spec test suite, no test depends on a shell command
  succeeding or failing through Speccy.

**Covered by:** CHK-004

### REQ-005: Docs and shipped skills teach the new contract

Architecture docs and skill prompts stop implying that Speccy runs or
grades tests.

**Done when:**
- `.speccy/ARCHITECTURE.md` describes checks as validation scenarios.
- The `check` command row is render-only.
- The `verify` command row is shape-only.
- The "Feedback, Not Enforcement" section explicitly says project CI
  owns test execution and reviewer personas own semantic judgment.
- Shipped prompts under `resources/modules/` and matching
  `.speccy/skills/` copies no longer tell agents to author
  `kind =`, `command =`, or `prompt =` check rows.
- Active guidance uses `scenario = """..."""` examples.

**Behavior:**
- Given a grep for legacy check-authoring snippets in active docs and
  shipped skills, there are no hits except historical migration notes.
- Given reviewer-tests reads the post-spec prompt, it is instructed to
  compare tests against scenario prose, not command exit codes.

**Covered by:** CHK-005

## Design

### Approach

Implementation order:

1. Update the check schema and migrate all in-tree `spec.toml` files.
2. Replace `speccy check` execution with scenario rendering.
3. Replace `speccy verify` execution aggregation with shape
   validation.
4. Delete `speccy-core::exec` and stale tests.
5. Sweep architecture docs and shipped skills.

That order leaves the repository coherent after each major step and
keeps SPEC-0019 free to remove `spec.toml` entirely later.

### Decisions

#### DEC-001: Keep the `check` command name for now

**Status:** Accepted

**Context:** The name "check" now carries some stale executable
connotation, but it is already part of the ten-command surface and
SPEC-0017 just made its selector semantics useful.

**Decision:** Keep `speccy check` as a render-only command in this
spec. The payload becomes a scenario; the command remains the stable
selector surface.

**Consequences:** There is less CLI churn. A future rename to
`scenario` or `assert` can be evaluated after the canonical carrier
work lands.

#### DEC-002: Scenario quality remains reviewer-owned

**Status:** Accepted

**Context:** Mechanical heuristics like minimum scenario length are
easy to satisfy without improving validation quality.

**Decision:** The CLI validates presence, ids, and references only.
Reviewer personas judge whether a scenario is meaningful and whether
the implementation/tests satisfy it.

**Consequences:** Speccy stays a feedback substrate instead of a weak
test-quality classifier.

#### DEC-003: Hard break before v1

**Status:** Accepted

**Context:** Speccy has no released v1 compatibility promise yet.

**Decision:** Reject old check fields after migration rather than
accepting both schemas.

**Consequences:** The parser surface shrinks immediately, and the
docs do not need to teach two shapes.

## Migration / Rollback

Migration is a one-shot `xtask` or temporary binary that reads the
old parser structs, writes the new schema, and is deleted before the
final commit lands. The script must be structural, not line-oriented,
because scenario text may be multiline TOML.

Rollback is `git revert` of the implementation commit set. Because
the old parser and old TOML shape live in git history, a revert
restores both.

## Open Questions

- [ ] Should a future command rename `check` to `scenario` or
      `assert` after SPEC-0019 removes `spec.toml`? Lean defer.
- [ ] Should `speccy verify --json` include warnings as a separate
      array from errors? Lean yes only if an existing consumer needs
      machine-readable warnings.

## Assumptions

- All in-tree `spec.toml` files parse under the current schema before
  the migration starts.
- Current project CI continues to run the four standard hygiene gates
  directly.
- SPEC-0019 will remove the `spec.toml` carrier after this spec lands,
  so SPEC-0018 should avoid adding new fields that would immediately
  be migrated away.

## Changelog

| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial rewritten draft. Narrows SPEC-0018 to execution removal and scenario-shaped checks while keeping the current carrier. |

## Notes

This is the correct first step because it changes semantics without
also changing the artifact carrier. Once checks are English scenarios,
SPEC-0019 can move those scenarios into `SPEC.md` without preserving
the old executable-command vocabulary.
