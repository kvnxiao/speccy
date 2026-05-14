---
id: SPEC-0003
slug: lint-engine
title: Lint engine -- SPC / REQ / VAL / TSK / QST diagnostics
status: implemented
created: 2026-05-11
---

# SPEC-0003: Lint engine

## Summary

`speccy-core::lint` is a pure library that takes parsed artifacts
(SPEC.md, spec.toml, TASKS.md, the supersession index -- all from
SPEC-0001) and emits structured `Diagnostic` values with stable lint
codes. It is consumed by SPEC-0004 (`status`) and SPEC-0012
(`verify`); future commands and dashboards can call it the same way.

The catalogue of codes is the one in `.speccy/DESIGN.md` "Lint
Codes": `SPC-001` through `SPC-007`, `REQ-001` and `REQ-002`,
`VAL-001` through `VAL-004`, `TSK-001` through `TSK-004`, and
`QST-001`. This spec implements every one and locks the catalogue
into a stability registry that breaks the build if a code is
silently removed or renamed.

All semantic judgement of *quality* stays in review. Lint catches
only mechanical inconsistencies that don't require an LLM to spot:
missing fields, mismatched IDs, dangling references, known no-op
commands, stale TASKS.md frontmatter. The one stylistic call this
spec makes is VAL-004 (no-op detection), and that uses a closed
regex set rather than a heuristic.

## Goals

- Every code in `.speccy/DESIGN.md` "Lint Codes" is implemented and
  triggered by at least one fixture.
- Codes are stable: removing or renaming one is a build-breaking
  change; adding a new one is non-breaking.
- VAL-004 fires on a closed set of known no-op patterns and never
  on real commands.
- Severity per code is hardcoded and predictable (drives
  `speccy verify` behaviour in SPEC-0012).
- Diagnostics are emitted in deterministic order so snapshot tests
  are stable.

## Non-goals

- No semantic quality checks. Lint does not evaluate whether
  `proves:` strings are meaningful or whether tests are exhaustive.
- No autofix. Lint reports; users or skill prompts decide what to do.
- No project-configurable severity overrides. There is no
  `lint.toml`; matching the "no `--strict` mode" stance.
- No execution of commands. VAL-004 is pattern-based, not behavioural.
- No I/O. Lint receives parsed structures; callers do the parsing.

## User stories

- As a future `speccy status` implementer (SPEC-0004), I want a
  single function that takes a parsed workspace and returns a
  deterministic vec of diagnostics so I can render them inline per
  spec.
- As a future `speccy verify` implementer (SPEC-0012), I want
  diagnostics tagged with severity so I know which ones fail CI
  (errors) and which are informational (warns and infos).
- As a reviewer-architecture persona reading a SPEC.md, I want
  unchecked open questions surfaced (QST-001) so I notice them
  before approving the spec.
- As a spec author, I want a clear "your TASKS.md is stale" signal
  (TSK-003) so I know when to run `/speccy-amend`.

## Requirements

### REQ-001: SPC-* lint codes (spec structure)

Emit SPC-001 through SPC-007 against parsed `SpecToml`, parsed
`SpecMd`, and the workspace supersession index.

**Done when:**
- `SPC-001`: A required field in `spec.toml` is missing. (Error.) The
  parser already surfaces this; lint emits the diagnostic with
  `file = .../spec.toml`.
- `SPC-002`: A REQ heading in SPEC.md has no matching `[[requirements]]`
  row in spec.toml. (Error.)
- `SPC-003`: A `[[requirements]]` row in spec.toml has no matching
  REQ heading in SPEC.md. (Error.)
- `SPC-004`: A required field in SPEC.md frontmatter (id, slug, title,
  status, created) is missing. (Error.)
- `SPC-005`: SPEC.md frontmatter `status` value is not in the closed
  set `{in-progress, implemented, dropped, superseded}`. (Error.)
- `SPC-006`: SPEC.md `status: superseded` but no other spec in the
  workspace declares `supersedes` pointing here. Computed via the
  supersession index from SPEC-0001 REQ-008. (Error.)
- `SPC-007`: SPEC.md `status: implemented` but TASKS.md has any
  non-`[x]` task. (Info; informational.)

**Behavior:**
- Given a fixture with SPEC.md REQ-001+REQ-002 and spec.toml only
  REQ-001, when linted, exactly one SPC-002 is emitted (for REQ-002).
- Given a fixture with `status: implemented` and one `[ ]` task,
  when linted, exactly one SPC-007 is emitted at severity = Info.
- Given a workspace where SPEC-0017 has `status: superseded` and no
  other spec declares `supersedes: [SPEC-0017]`, when linted on
  SPEC-0017, then SPC-006 is emitted naming SPEC-0017.
- Given a fixture SPEC.md with `status: paused` (invalid), when
  linted, then SPC-005 is emitted naming the value.

**Covered by:** CHK-001, CHK-002, CHK-003

### REQ-002: REQ-* lint codes (requirement coverage)

Emit REQ-001 and REQ-002 against the requirement-to-check graph in
spec.toml.

**Done when:**
- `REQ-001`: A `[[requirements]]` row has an empty `checks` array.
  (Error.)
- `REQ-002`: A `[[requirements]]` row's `checks` array references a
  CHK ID that has no matching `[[checks]]` entry. (Error.)

**Behavior:**
- Given `[[requirements]] id = "REQ-001" checks = []`, when linted,
  then REQ-001 lint code is emitted naming `REQ-001`.
- Given `[[requirements]] id = "REQ-001" checks = ["CHK-999"]` and
  no `[[checks]] id = "CHK-999"`, when linted, then REQ-002 is
  emitted naming both the requirement and the missing check.

**Covered by:** CHK-004

### REQ-003: VAL-* lint codes (check definitions)

Emit VAL-001 through VAL-004 against parsed `[[checks]]` entries.

**Done when:**
- `VAL-001`: A check is missing the `proves` field. (Error.)
- `VAL-002`: A check with `kind = "test"` or `kind = "command"` is
  missing `command`. (Error.)
- `VAL-003`: A check with `kind = "manual"` is missing `prompt`.
  (Error.)
- `VAL-004`: A check's `command` matches a known no-op pattern.
  (Warn.)

VAL-004 closed pattern set (regex match against the *trimmed* command
string; whitespace-tolerant):

```text
^true$
^:$
^exit\s+0$
^/bin/true$
^cmd\s+/c\s+exit\s+0$
^exit\s+/b\s+0$
```

Compound commands containing a no-op prefix do **not** match. For
example, `true && cargo test` is NOT flagged; the lint targets pure
no-ops only.

**Behavior:**
- Given a check with `command = "true"`, when linted, then VAL-004
  is emitted at severity = Warn.
- Given a check with `command = "  true  "`, when linted, then
  VAL-004 is emitted (whitespace tolerated).
- Given a check with `command = "true && cargo test"`, when linted,
  then VAL-004 is NOT emitted.
- Given a check with `kind = "manual"` and no `prompt`, when linted,
  then VAL-003 is emitted naming the check ID.

**Covered by:** CHK-005, CHK-006

### REQ-004: TSK-* lint codes (task structure)

Emit TSK-001 through TSK-004 against parsed `TasksMd`.

**Done when:**
- `TSK-001`: A task's `Covers:` line references a REQ ID that doesn't
  exist in the spec's SPEC.md or spec.toml. (Error.)
- `TSK-002`: The TASKS.md parser surfaced a recoverable warning about
  a malformed task ID (e.g. `**TASK-001**` instead of `**T-001**`).
  The lint code consumes the parser's warning. (Error.)
- `TSK-003`: Staleness signal: either (a) TASKS.md's
  `spec_hash_at_generation` does not match the current SPEC.md sha256,
  or (b) SPEC.md mtime is newer than TASKS.md mtime. (Warn.)
- `TSK-004`: TASKS.md frontmatter is missing a required field
  (`spec`, `spec_hash_at_generation`, `generated_at`). (Error.)

The `bootstrap-pending` sentinel for `spec_hash_at_generation` is a
specific TSK-003 variant: same code, severity = Info, and the
message advises `speccy tasks SPEC-NNNN --commit` rather than
`/speccy-amend`. Lint distinguishes the cases via message text only.

**Behavior:**
- Given a TASKS.md with a task `Covers: REQ-099` where REQ-099 isn't
  in the spec, when linted, then TSK-001 is emitted naming the task
  and the missing REQ.
- Given a TASKS.md whose `spec_hash_at_generation` is `abc123` but
  the current SPEC.md sha256 is `def456`, when linted, then TSK-003
  is emitted at severity = Warn naming both hashes.
- Given a TASKS.md with `spec_hash_at_generation: bootstrap-pending`,
  when linted, then TSK-003 is emitted at severity = Info with a
  message naming `speccy tasks --commit` as the remediation.
- Given a TASKS.md without `generated_at` in its frontmatter, when
  linted, then TSK-004 is emitted naming the missing field.

**Covered by:** CHK-007, CHK-008

### REQ-005: QST-001 lint code (open questions)

Emit QST-001 per unchecked open question in SPEC.md.

**Done when:**
- A SPEC.md `## Open questions` section with `- [ ] some question?`
  produces a QST-001 diagnostic at severity = Info.
- The same question with `- [x] resolved...` produces nothing.
- The question text (minus the checkbox glyph) is included in the
  diagnostic message.

**Behavior:**
- Given a SPEC.md with three unchecked and two checked open
  questions, when linted, then exactly three QST-001 diagnostics are
  emitted, each carrying the corresponding question text.

**Covered by:** CHK-009

### REQ-006: Public API and `lint::run` entry point

Expose a single function consumed by SPEC-0004 and SPEC-0012.

**Done when:**
- `lint::run(workspace: &Workspace) -> Vec<Diagnostic>` is the
  entry point.
- `Workspace` bundles parsed specs (each with SPEC.md, spec.toml,
  optional TASKS.md) plus a borrowed `SupersessionIndex` from
  SPEC-0001.
- `Diagnostic { code: &'static str, level: Level, message: String,
  spec_id: Option<String>, file: Option<PathBuf>, line: Option<u32> }`.
- `Level` is the closed enum `{ Error, Warn, Info }`.
- The function is pure; no filesystem access; no panics from public
  paths.
- Diagnostics are returned in deterministic order: by `spec_id`,
  then `code`, then `file`, then `line` (each ascending; `None`
  sorts before `Some`).

**Behavior:**
- Given two identical `Workspace` inputs, when `lint::run` is called
  twice, then the resulting vecs are byte-equal.
- Given an empty workspace, when `lint::run` is called, then the
  result is an empty vec (no panics).
- Given a workspace where one spec emits SPC-002 and another emits
  REQ-001, when linted, then the SPC-002 from the lower spec ID
  appears before any diagnostic from the higher spec ID, regardless
  of internal iteration order.

**Covered by:** CHK-010

### REQ-007: Lint code stability contract

Codes are stable across minor versions; severity is part of the
contract.

**Done when:**
- A `const REGISTRY: &[(&'static str, Level)]` lists every code the
  engine emits, with its severity.
- A snapshot test compares `REGISTRY` against an on-disk snapshot
  (`speccy-core/tests/snapshots/lint_registry.snap`).
- Removing or renaming a code fails the snapshot test.
- Adding a new code requires snapshot regeneration; the test fails
  until the snapshot includes it.
- Severity changes (Error -> Warn, etc.) also fail the snapshot.

**Behavior:**
- Given a developer removes `SPC-007` from the engine, when the
  registry test runs, then it fails with a clear message naming the
  removed code.
- Given a developer adds a new `SPC-008` code without updating the
  snapshot, when the registry test runs, then it fails with a clear
  message asking for snapshot regeneration.
- Given a developer changes `VAL-004` from Warn to Error, when the
  registry test runs, then it fails with the severity diff.

**Covered by:** CHK-011

## Design

### Approach

A pure module tree at `speccy-core/src/lint/`:

- `lint::types` -- `Diagnostic`, `Level`, `Workspace`, `ParsedSpec`.
- `lint::registry` -- the stability `REGISTRY` const + a snapshot
  test helper.
- `lint::rules::{spc, req, val, tsk, qst}` -- one module per code
  family.
- `lint::run` -- the orchestrator that calls every rule against the
  `Workspace` and concatenates results, then sorts.

Each rule module exposes a `pub fn lint(spec: &ParsedSpec, ws: &Workspace) -> Vec<Diagnostic>`
(or a similar shape). The orchestrator drives them, collects, sorts,
and returns.

### Decisions

#### DEC-001: Pure library, no I/O

**Status:** Accepted
**Context:** Lint is consumed by multiple commands (status, verify,
future dashboards). Each calls it after parsing via SPEC-0001.
**Decision:** `lint::run` takes parsed artifacts as input. No
filesystem access inside the lint engine.
**Alternatives:**
- Lint reads files itself -- rejected. Forces every consumer to
  share path conventions; harder to test with in-memory fixtures;
  duplicates SPEC-0001's job.
**Consequences:** Callers parse first. In-memory fixtures make
testing trivial.

#### DEC-002: VAL-004 closed pattern set, not heuristics

**Status:** Accepted
**Context:** VAL-004 flags no-op commands. A general "does this
command do anything?" heuristic is impossible without execution.
**Decision:** Hardcode the six regex patterns listed in REQ-003.
Add patterns when new no-op idioms are observed in the wild.
**Alternatives:**
- Execute the command in a sandbox and check the exit code --
  rejected. Wrong layer; lint must not execute checks.
- Whitelist known-good commands and flag everything else --
  rejected. False-positive disaster.
**Consequences:** VAL-004 catches the obvious cases without false
positives. Misses cleverer no-ops (e.g. `cargo --version`, which
always exits 0 but proves nothing) -- those go to the
reviewer-tests persona, where they belong.

#### DEC-003: Severity baked in, not configurable

**Status:** Accepted
**Context:** Severity drives `speccy verify` exit policy in SPEC-0012.
A `lint.toml` would let a project silence errors and break the
guarantee that `verify` means something.
**Decision:** Severity per code is hardcoded in `REGISTRY`. No
configuration knob.
**Alternatives:**
- `lint.toml` with overrides -- rejected. Violates the "no `--strict`
  mode" stance; lets projects vote themselves into broken CI.
**Consequences:** Predictable lint behaviour across projects.
Severity changes require a speccy release and snapshot-test update.

#### DEC-004: Deterministic diagnostic ordering

**Status:** Accepted
**Context:** Callers (status text, verify JSON) display diagnostics;
tests snapshot them. Non-deterministic ordering breaks snapshot
tests.
**Decision:** Sort diagnostics by `(spec_id, code, file, line)`
ascending before returning. `None` sorts before `Some`.
**Alternatives:**
- Emission order -- rejected. Depends on internal iteration order,
  which varies with refactoring.
**Consequences:** Diff-friendly output; stable snapshot tests.

#### DEC-005: TSK-003 `bootstrap-pending` as a message variant, not a separate code

**Status:** Accepted
**Context:** The `bootstrap-pending` sentinel for
`spec_hash_at_generation` is meaningfully different from real
staleness (the remediation is `speccy tasks --commit`, not
`/speccy-amend`). A separate code would be cleaner.
**Decision:** Use TSK-003 with a different message text and
severity = Info (not Warn). One code, two messages.
**Alternatives:**
- Allocate `TSK-005` for the bootstrap-pending case -- rejected for
  v1. Adds a code without adding information (it's structurally the
  same problem: stored hash != computed hash). Revisit if the
  message-only distinction proves confusing.
**Consequences:** Slightly subtle test setup (the test must inspect
the message text, not just the code). Acceptable for v1.

### Interfaces

```rust
pub mod lint {
    pub fn run(workspace: &Workspace) -> Vec<Diagnostic>;
}

pub struct Workspace<'a> {
    pub specs: Vec<ParsedSpec<'a>>,
    pub supersession: &'a SupersessionIndex<'a>,
}

pub struct ParsedSpec<'a> {
    pub spec_md: &'a SpecMd,
    pub spec_toml: &'a SpecToml,
    pub tasks_md: Option<&'a TasksMd>,
    pub spec_md_path: PathBuf,
    pub tasks_md_path: Option<PathBuf>,
}

pub struct Diagnostic {
    pub code: &'static str,          // "SPC-001", "REQ-002", ...
    pub level: Level,
    pub message: String,
    pub spec_id: Option<String>,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

pub enum Level { Error, Warn, Info }
```

### Data changes

- New module tree `speccy-core/src/lint/` and submodules.
- New `speccy-core/tests/lint_*.rs` per code family.
- New `speccy-core/tests/snapshots/lint_registry.snap` (the
  stability snapshot).
- New `speccy-core/tests/fixtures/lint/` corpus.

### Migration / rollback

Greenfield library code. Rollback is `git revert`; no migration since
nothing else depends on the lint engine until SPEC-0004 and
SPEC-0012 land.

## Open questions

- [ ] Should VAL-004 flag trivially-passing commands like
  `cargo --version` (always exits 0 but proves nothing)? Likely no
  -- that's a quality judgment for the reviewer-tests persona, not
  a mechanical pattern. Defer.
- [ ] Should TSK-003 severity escalate to Error when staleness has
  persisted for some threshold? Out of scope for v1; revisit if
  staleness ignored over time becomes a real failure mode.

## Assumptions

- The SPEC-0001 parser surfaces recoverable warnings (e.g. malformed
  task IDs) on the returned struct so the lint engine can consume
  them without re-parsing. (REQ-004 of SPEC-0001 specifies this.)
- The supersession index from SPEC-0001 REQ-008 is computed once per
  workspace scan and borrowed into the `Workspace`.
- `&'static str` codes (compile-time constants) are sufficient for
  the public API; no allocated `String` codes.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from DESIGN.md decomposition (bootstrap of speccy). |

## Notes

This spec is "narrow but exhaustive": every code in DESIGN.md's
"Lint Codes" section is implemented, and the stability registry
ensures none are silently lost. The fixture corpus (T-012) is the
durable test surface -- each code has at least one fixture that
triggers it.

SPEC-0004 (status) and SPEC-0012 (verify) are direct consumers. When
those specs land, they should call `lint::run` and never re-implement
diagnostic logic. SPEC-0012 in particular maps `Level::Error` to
exit-code-1 behaviour; Warn and Info do not affect the exit code.
