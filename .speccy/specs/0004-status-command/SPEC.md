---
id: SPEC-0004
slug: status-command
title: speccy status -- workspace overview, stale detection, lint surfacing
status: implemented
created: 2026-05-11
---

# SPEC-0004: speccy status

## Summary

`speccy status` is the workspace overview command. It scans
`.speccy/specs/` for every spec directory, parses each one
(SPEC.md, spec.toml, optional TASKS.md), runs `lint::run` from
SPEC-0003, computes staleness and the supersession inverse, then
renders either filtered text (default, optimised for human reading)
or stable JSON (`--json`, for harness consumption).

It is the first JSON-emitting command in the speccy CLI, so it
establishes the JSON envelope conventions (`schema_version: 1` at
the top) that SPEC-0007 (`next`) and SPEC-0012 (`verify`) will
follow.

The command is forgiving about per-spec parse failures: one
malformed spec doesn't kill the overview. The offending spec is
listed with its parse error inline, and the other specs still
display normally.

`speccy status` is also the de-facto landing place for
`speccy_core::workspace::scan` -- the shared utility for project-
root discovery, spec-directory enumeration, and staleness detection.
SPEC-0010 (`check`) and SPEC-0012 (`verify`) reuse it.

## Goals

<goals>
- One-shot overview of every spec's state, suitable for both human
  scanning and harness parsing.
- Stable JSON contract (`schema_version: 1`) that downstream tooling
  can rely on across speccy minor versions.
- Default text view stays scannable -- shows what's in flight and
  what's broken, hides what's done unless broken.
- Per-spec parse failures are surfaced, not fatal.
- The shared workspace scanner lives in `speccy-core` so later
  specs don't duplicate it.
</goals>

## Non-goals

<non-goals>
- No mutation of any artifact. `status` is strictly read-only.
- No execution of checks. Use `speccy check` (SPEC-0010).
- No interactive UI. Plain text and JSON only.
- No filtering flags in v1 beyond `--json`. Filtering is the
  harness's job once it has JSON.
- No watch mode or polling. One-shot only.
</non-goals>

## User stories

<user-stories>
- As a developer mid-loop, I want one command that tells me which
  specs are open, which tasks are next, and whether anything is
  stale or lint-broken.
- As a harness writer (skill), I want `--json` output with a stable
  `schema_version` so I can parse the result without
  re-implementing artifact parsing.
- As a reviewer, I want unchecked open questions on a spec to be
  visible in the overview so they're not missed at review time.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Workspace scan

Discover and parse every spec under `.speccy/specs/`. Surface
per-spec parse failures inline rather than failing the whole scan.

<done-when>
- A scan starting from the project root finds every subdirectory of
  `.speccy/specs/` whose name matches `^\d{4}-[a-z0-9-]+$`.
- For each match, the scan attempts to parse `SPEC.md`,
  `spec.toml`, and `TASKS.md` (the last is optional) via the
  SPEC-0001 parser.
- Parse failure on any individual artifact produces a `ScannedSpec`
  entry carrying the error and does NOT abort the whole scan.
- Subdirectories not matching the pattern (e.g. `_scratch`, `notes`)
  are silently ignored.
- A scan from a cwd inside a speccy workspace discovers the project
  root by walking up parent directories until `.speccy/` is found.
</done-when>

<behavior>
- Given a workspace with `0001-foo/`, `0002-bar/`, and `_scratch/`,
  when scanned, then the result contains two specs and `_scratch`
  is absent.
- Given `0001-foo/SPEC.md` is malformed and `0002-bar/` is well-
  formed, when scanned, then the result has two entries; the first
  has `spec_md: Err(...)` and the second is fully parsed.
- Given `.speccy/specs/` doesn't exist, when scanned, then the
  result is an empty `specs` vec without error.
- Given cwd is `/foo/bar/some/nested/dir` and `/foo/bar/.speccy/`
  exists, when project-root discovery runs, then it returns
  `/foo/bar`.
</behavior>

<scenario id="CHK-001">
workspace::scan finds every NNNN-slug directory under .speccy/specs/, parses each artifact, and ignores non-matching subdirectories.
</scenario>

<scenario id="CHK-002">
Malformed individual specs produce ScannedSpec entries with parse errors; other specs still scan successfully; missing .speccy/specs/ yields an empty result without error.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Staleness detection

Compute whether each spec's TASKS.md is stale relative to SPEC.md.

<done-when>
- For each spec with a TASKS.md, compute
  `Staleness { stale: bool, reasons: Vec<StaleReason> }` where
  `StaleReason` is one of `HashDrift`, `MtimeDrift`,
  `BootstrapPending`.
- `HashDrift`: TASKS.md frontmatter `spec_hash_at_generation`
  doesn't equal the parsed SPEC.md's computed sha256.
- `MtimeDrift`: SPEC.md filesystem mtime is strictly greater than
  TASKS.md's mtime.
- `BootstrapPending`: `spec_hash_at_generation` equals the literal
  string `bootstrap-pending`. When this fires, no other reasons are
  added (the sentinel short-circuits the rest of the check).
- `stale` is `true` if `reasons` is non-empty.
- Specs without TASKS.md have `Staleness { stale: false, reasons: [] }`.
</done-when>

<behavior>
- Given hash match AND TASKS.md mtime >= SPEC.md mtime, then
  `stale = false`.
- Given hash mismatch, then `stale = true` with `HashDrift` in
  reasons.
- Given hash match but SPEC.md mtime > TASKS.md mtime, then
  `stale = true` with `MtimeDrift`.
- Given `spec_hash_at_generation: bootstrap-pending`, then
  `stale = true` with `BootstrapPending` as the sole reason.
</behavior>

<scenario id="CHK-003">
- Given hash match AND TASKS.md mtime >= SPEC.md mtime, then
  `stale = false`.
- Given hash mismatch, then `stale = true` with `HashDrift` in
  reasons.
- Given hash match but SPEC.md mtime > TASKS.md mtime, then
  `stale = true` with `MtimeDrift`.
- Given `spec_hash_at_generation: bootstrap-pending`, then
  `stale = true` with `BootstrapPending` as the sole reason.

stale_for returns HashDrift, MtimeDrift, or BootstrapPending appropriately; bootstrap-pending sentinel short-circuits other reasons; specs without TASKS.md are not stale.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Task state aggregation

Count tasks per state for each spec.

<done-when>
- For each spec with a TASKS.md, compute `TaskCounts { open,
  in_progress, awaiting_review, done }` from the parsed task list.
- Specs without TASKS.md have all counts at zero.
</done-when>

<behavior>
- Given a TASKS.md with two `[ ]`, one `[~]`, one `[?]`, one `[x]`
  task, then `TaskCounts { open: 2, in_progress: 1,
  awaiting_review: 1, done: 1 }`.
- Given a TASKS.md with only phase headings and no task lines, then
  all counts are zero.
</behavior>

<scenario id="CHK-004">
- Given a TASKS.md with two `[ ]`, one `[~]`, one `[?]`, one `[x]`
  task, then `TaskCounts { open: 2, in_progress: 1,
  awaiting_review: 1, done: 1 }`.
- Given a TASKS.md with only phase headings and no task lines, then
  all counts are zero.

Task state counts match the [ ] / [~] / [?] / [x] glyph distribution; missing TASKS.md yields zero counts.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Supersession inverse

Compute `superseded_by` per spec by inverting the supersedes graph
via `supersession_index` (SPEC-0001 REQ-008).

<done-when>
- The scanner builds a `&[&SpecMd]` slice over successfully-parsed
  specs and calls `supersession_index`.
- Each spec entry exposes `superseded_by: Vec<String>` from the
  index.
- Specs that failed to parse have an empty `superseded_by` (we
  can't index something we couldn't parse).
- The supersession index's `dangling_references()` is also surfaced
  on the workspace result so SPEC-0003 lint can emit diagnostics
  about them.
</done-when>

<behavior>
- Given SPEC-0017 (no supersedes) and SPEC-0042
  (`supersedes: [SPEC-0017]`), then SPEC-0017's entry has
  `superseded_by: ["SPEC-0042"]` and SPEC-0042's entry has
  `superseded_by: []`.
- Given a spec with `parse_error`, then its `superseded_by` is `[]`.
</behavior>

<scenario id="CHK-005">
- Given SPEC-0017 (no supersedes) and SPEC-0042
  (`supersedes: [SPEC-0017]`), then SPEC-0017's entry has
  `superseded_by: ["SPEC-0042"]` and SPEC-0042's entry has
  `superseded_by: []`.
- Given a spec with `parse_error`, then its `superseded_by` is `[]`.

superseded_by per spec is computed via supersession_index inversion over successfully-parsed specs; specs with parse errors have empty superseded_by; dangling references are surfaced for lint.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Lint integration

Run `lint::run` from SPEC-0003 and surface diagnostics per spec and
at workspace level.

<done-when>
- The command constructs a `lint::Workspace` (per SPEC-0003 REQ-006)
  from the parsed specs and the supersession index.
- It calls `speccy_core::lint::run(&workspace)` and partitions the
  returned diagnostics by `spec_id`.
- Each spec entry exposes `lint: { errors, warnings, info }` --
  three arrays grouped by `Level`.
- Workspace-level diagnostics (those without a `spec_id`) appear in
  a top-level `lint: { errors, warnings, info }` block.
</done-when>

<behavior>
- Given SPEC-0001 emits one Error and one Warn diagnostic, then
  SPEC-0001's entry's `lint.errors` has one item and `lint.warnings`
  has one item, each as a structured object.
- Given every spec is clean, then every spec's lint block has three
  empty arrays.
- Given a dangling supersedes reference (e.g. `supersedes:
  [SPEC-9999]`), then the workspace-level lint block contains a
  diagnostic naming SPEC-9999.
</behavior>

<scenario id="CHK-006">
- Given SPEC-0001 emits one Error and one Warn diagnostic, then
  SPEC-0001's entry's `lint.errors` has one item and `lint.warnings`
  has one item, each as a structured object.
- Given every spec is clean, then every spec's lint block has three
  empty arrays.
- Given a dangling supersedes reference (e.g. `supersedes:
  [SPEC-9999]`), then the workspace-level lint block contains a
  diagnostic naming SPEC-9999.

status partitions lint::run diagnostics by spec_id; workspace-level diagnostics (no spec_id) appear in the top-level lint block; each lint block has errors/warnings/info arrays.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Text view with default filter

Render a human-scannable view filtered to specs that need attention.

<done-when>
- Default text view shows specs with `status: in-progress` plus any
  spec where any of the following hold, regardless of status:
  - `lint.errors` is non-empty.
  - `stale = true`.
  - `parse_error` is `Some(_)` on any parsed artifact.
- Specs with `status` in `{implemented, dropped, superseded}` AND
  no errors AND no staleness are excluded from the default view.
- Each shown spec has a one-line header (`SPEC-NNNN <status>: <title>`)
  plus per-line summaries for tasks, lint counts, staleness, open
  questions.
- An empty workspace prints `No specs in workspace.` to stdout and
  exits 0.
</done-when>

<behavior>
- Given five specs (two `in-progress`, two clean `implemented`, one
  `implemented` with stale TASKS.md), then the text output shows
  three specs.
- Given no specs, then stdout shows `No specs in workspace.` and
  exit code is 0.
</behavior>

<scenario id="CHK-007">
Default text view shows in-progress specs plus any with errors / staleness / parse failures; clean implemented/dropped/superseded specs are excluded.
</scenario>

<scenario id="CHK-008">
Empty workspace prints 'No specs in workspace.' and exits 0; single-spec workspace prints a one-line header plus per-line summaries.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: JSON output with stable schema

Render structured JSON for harness consumption.

<done-when>
- `--json` emits valid UTF-8 JSON beginning with `"schema_version":
  1` and including `"repo_sha": <string>` (the HEAD git SHA, or `""`
  if git is unavailable or the repo has no HEAD).
- Every spec appears in the `specs` array regardless of status (no
  filtering).
- Each spec entry includes: `id`, `slug`, `title`, `status`,
  `supersedes`, `superseded_by`, `tasks`, `stale`, `stale_reasons`,
  `open_questions`, `lint`, and optional `parse_error`.
- Lint diagnostics inside `lint.errors/warnings/info` are structured
  objects: `{ code, level, message, file?, line? }`. (ARCHITECTURE.md
  showed them as strings; this spec upgrades to structured -- the
  string form would force every harness to re-parse them.)
- Output is deterministic: spec order is ascending by spec ID;
  diagnostics within a spec are ordered by `(code, file, line)`;
  `stale_reasons` are in declared order (`HashDrift`, `MtimeDrift`,
  `BootstrapPending`).
- Output is pretty-printed.
</done-when>

<behavior>
- Given any workspace, when `speccy status --json` runs twice with
  no intervening filesystem change, the outputs are byte-identical.
- Given no git is installed (or no HEAD exists), then `repo_sha`
  is `""` and the command still succeeds.
- Given a workspace with one spec, when `--json` runs, the output
  validates against the contract above.
</behavior>

<scenario id="CHK-009">
- Given any workspace, when `speccy status --json` runs twice with
  no intervening filesystem change, the outputs are byte-identical.
- Given no git is installed (or no HEAD exists), then `repo_sha`
  is `""` and the command still succeeds.
- Given a workspace with one spec, when `--json` runs, the output
  validates against the contract above.

speccy status --json emits schema_version=1, repo_sha (empty if no git), every spec regardless of status, structured lint diagnostics, deterministic byte-identical output across runs with no filesystem change.
</scenario>

</requirement>

## Design

### Approach

The command lives in `speccy-cli/src/status.rs`. The heavy
lifting goes into `speccy-core/src/workspace.rs`:

- `workspace::find_root(start: &Path)` -- walk up to find `.speccy/`.
- `workspace::scan(project_root: &Path)` -- enumerate spec
  directories and parse each artifact.
- `workspace::stale_for(...)` -- compute staleness for one spec.

The flow per invocation:

1. Discover project root.
2. Scan `.speccy/specs/` for spec directories.
3. Parse each spec; record parse errors but continue.
4. Compute `supersession_index` over successfully-parsed specs.
5. Build the `lint::Workspace` and call `lint::run`.
6. For each spec, compute task counts, staleness, open-questions
   count.
7. Render text (filtered) or JSON (everything).

Text rendering is `println!`-formatted output. JSON uses
`serde_json::to_string_pretty` with a hand-defined output struct
that mirrors the contract.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Per-spec parse failure is non-fatal

**Status:** Accepted
**Context:** A single malformed spec shouldn't blind the user to
the rest of the workspace. The most common case is mid-edit -- a
spec is being drafted and isn't valid yet.
**Decision:** Parse failures produce a `ScannedSpec` entry with
`spec_md: Err(...)` (or similar) and the scan proceeds.
**Alternatives:**
- Abort on first parse failure -- rejected. Blinds the user.
- Skip malformed specs silently -- rejected. No signal something
  is wrong.
**Consequences:** Downstream processing (lint integration,
supersession index) operates over `Result<_, _>` per artifact and
skips errored ones.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Structured lint diagnostics in JSON, not strings

**Status:** Accepted
**Context:** ARCHITECTURE.md's JSON example shows lint diagnostics as
strings (`"VAL-001: CHK-003 missing 'proves' field"`). Strings
force every consumer to re-parse them, which is a small but real
foot-gun.
**Decision:** Emit each diagnostic as a structured object: `{ code,
level, message, file, line }`. Text view continues to render them
as one-line strings.
**Alternatives:**
- Strings (per ARCHITECTURE.md example) -- rejected.
**Consequences:** ARCHITECTURE.md's JSON example is illustrative, not
literal. A one-line clarification in ARCHITECTURE.md is queued as a
non-blocking follow-up.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: `repo_sha` via shell-out to git, optional

**Status:** Accepted
**Context:** ARCHITECTURE.md's JSON shape includes `repo_sha`. It's
useful for "which commit was this snapshot taken on?" but adding a
git library is heavy for one field.
**Decision:** Shell out to `git rev-parse HEAD` once per
invocation. If the command fails (no git on PATH, no HEAD, not a
git repo), set `repo_sha = ""` and continue without error.
**Alternatives:**
- `gix` library -- rejected for v1.
- Omit the field -- rejected; ARCHITECTURE.md contract has it.
**Consequences:** One subprocess per `status` invocation.
Negligible cost.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: Workspace utilities live in `speccy-core`, not the binary

**Status:** Accepted
**Context:** Workspace scanning, staleness detection, and project-
root discovery are general utilities. Putting them in the binary
crate would force SPEC-0010 (check) and SPEC-0012 (verify) to
duplicate them.
**Decision:** Land
`speccy_core::workspace::{find_root, scan, stale_for}` plus the
`Workspace`, `ScannedSpec`, `Staleness`, `StaleReason` types as
part of this spec.
**Alternatives:**
- Put scan logic in `speccy-cli/src/` -- rejected. Forces
  duplication in later commands.
**Consequences:** SPEC-0010 and SPEC-0012 consume these utilities.
PLANNING.md updates SPEC-0010's dependencies to reflect the
relationship.
</decision>

### Interfaces

```rust
// speccy-core additions
pub mod workspace {
    pub fn find_root(start: &Path) -> Result<PathBuf, WorkspaceError>;
    pub fn scan(project_root: &Path) -> Workspace;
    pub fn stale_for(
        spec: &SpecMd,
        tasks: Option<&TasksMd>,
        spec_path: &Path,
        tasks_path: Option<&Path>,
    ) -> Staleness;
}

pub struct Workspace {
    pub project_root: PathBuf,
    pub specs: Vec<ScannedSpec>,
    pub supersession: SupersessionIndex<'static>, // owns its data
}

pub struct ScannedSpec {
    pub id_from_dir: String,                 // "SPEC-0001" from dir
    pub dir: PathBuf,
    pub spec_md: Result<SpecMd, ParseError>,
    pub spec_toml: Result<SpecToml, ParseError>,
    pub tasks_md: Option<Result<TasksMd, ParseError>>,
}

pub struct Staleness {
    pub stale: bool,
    pub reasons: Vec<StaleReason>,
}

pub enum StaleReason { HashDrift, MtimeDrift, BootstrapPending }

pub enum WorkspaceError {
    NoSpeccyDir,                             // walked up to filesystem root
    Io(std::io::Error),
}

// speccy binary
pub fn run(args: StatusArgs) -> Result<(), StatusError>;
pub struct StatusArgs { pub json: bool }
```

### Data changes

- New `speccy-core/src/workspace.rs` (scan, staleness types).
- New `speccy-cli/src/status.rs` (command logic).
- New `speccy-cli/src/status_output.rs` (text + JSON renderers).
- `speccy-cli/Cargo.toml` adds `serde_json` (JSON output).

### Migration / rollback

Greenfield code. Rollback via `git revert`; nothing else consumes
the new types until SPEC-0010 and SPEC-0012 land.

## Open questions

- [ ] Should `--json` accept a `--filter <pattern>` flag for harness
  ergonomics, or is "filter on the harness side" sufficient? Likely
  sufficient for v1 -- JSON output is small enough.
- [ ] `JSON-001` from ARCHITECTURE.md's lint codes is the only `JSON-*`
  family member. Its purpose is unclear (status emits JSON, doesn't
  read it). Defer; SPEC-0003 didn't implement it either.
- [ ] Should `repo_sha` use the short SHA (7 chars) or full SHA (40
  chars)? Full is unambiguous; defer to first dogfood pass.

## Assumptions

<assumptions>
- `lint::run` from SPEC-0003 returns diagnostics sorted by
  `(spec_id, code, file, line)`. Status preserves that ordering
  when partitioning by spec.
- `serde_json::to_string_pretty` produces deterministic output for
  a given input struct with fixed field order.
- `git rev-parse HEAD` exits non-zero when not in a git repo or
  HEAD is unset; we treat both as `repo_sha = ""`.
- Filesystem mtime is reliable enough for staleness detection.
  CI environments that mass-touch files at checkout time may
  produce false-positive staleness; that's acceptable in v1.
</assumptions>

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
| 2026-05-12 | agent/claude | Implemented: `speccy_core::workspace` (scan, find_root, stale_for, TaskCounts, count_open_questions), `lint::Workspace` refactored to borrow specs, `speccy status [--json]` with text + JSON renderers + dangling-supersedes synthesis (WS-001). |
</changelog>

## Notes

This spec is the de-facto home for `speccy_core::workspace` --
the shared workspace-scanning utility consumed by SPEC-0010
(check) and SPEC-0012 (verify), and likely SPEC-0007 (next) too.
PLANNING.md's dependency column for SPEC-0010 is updated to reflect
this.

The JSON output is a forward-looking contract: once SPEC-0007 and
SPEC-0012 land their own JSON outputs, the `schema_version: 1`
field plus the structured-diagnostics convention should match
across all three commands.
