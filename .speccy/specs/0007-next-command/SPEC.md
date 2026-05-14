---
id: SPEC-0007
slug: next-command
title: speccy next -- pick the next actionable task with priority + JSON
status: implemented
created: 2026-05-11
---

# SPEC-0007: speccy next

## Summary

`speccy next` answers "what should I (or my agent) do next?" by
scanning task state across the workspace, applying a deterministic
priority rule, and emitting a single result -- either the next
actionable task, a "write the report" signal, or a "blocked" reason.

Two flags shape the answer:

- `--kind implement | review` filters across all specs to a single
  state class: `[ ]` (implement) or `[?]` (review).
- `--json` switches the output from one-line human text to a
  structured JSON envelope (tagged union by `kind`).

The command is read-only. It does not mutate any artifact. It is
the linchpin between SPEC-0004 (`status`, the overview) and the
prompt-rendering / execution commands (`implement`, `review`,
`report`) -- harnesses ask `next` what to do, then dispatch to the
right specific command.

## Goals

- Single deterministic query for "next thing to do" across all
  specs.
- `--kind` filters cleanly so implementer loops and reviewer loops
  can each pick up their own work without contention.
- JSON contract matches SPEC-0004's `schema_version: 1` envelope
  conventions; the four `kind` variants are stable.
- The four output kinds (`implement`, `review`, `report`,
  `blocked`) cover every workspace state without falling off the
  edge.

## Non-goals

- No mutation of task state. `next` is strictly read-only; the
  implementer or reviewer flips checkboxes.
- No claiming or locking. If two agents both ask for the next
  task, they get the same answer; if they both try to start work,
  git resolves the contention (per DESIGN.md "Concurrent pickup").
- No predictive scheduling. Priority is mechanical: lowest spec
  ID, then within-spec ordering. No estimated-time-to-complete,
  no critical-path computation.
- No persona-fan-out configuration in v1. The default set is
  hardcoded.

## User stories

- As `/speccy:work` (the implementation-loop skill), I want
  `speccy next --kind implement --json` to give me one
  `T-NNN` to dispatch a sub-agent against, or `kind: blocked` if
  nothing is ready.
- As `/speccy:review` (the review-loop skill), I want
  `speccy next --kind review --json` to give me one `T-NNN` plus
  the persona fan-out list so I can spawn parallel reviewer
  sub-agents.
- As a developer scanning manually, I want plain
  `speccy next` to tell me the single highest-priority thing
  across the whole workspace.
- As `/speccy:ship` (the report skill), I want to detect that
  "all tasks are `[x]` and REPORT.md is missing" so I know to
  generate the report.

## Requirements

### REQ-001: Default priority (no `--kind`)

Walk specs in ascending ID order; within each spec, prefer
review-ready (`[?]`) tasks over open (`[ ]`) tasks; return the
first match.

**Done when:**
- The scanner enumerates specs via `workspace::scan` in ascending
  spec-ID order.
- For each spec, it inspects task state in this order:
  1. Any task with state `AwaitingReview` (`[?]`) -> return as
     `kind: review` with persona fan-out.
  2. Any task with state `Open` (`[ ]`) -> return as
     `kind: implement`.
  3. Otherwise, advance to the next spec.
- If walking all specs returns no `[?]` or `[ ]` task, fall
  through to report-kind detection (REQ-002) and then blocked-kind
  detection (REQ-003).
- `InProgress` (`[~]`) tasks are skipped (they're claimed by some
  other session).

**Behavior:**
- Given SPEC-0001 with one `[?]` and two `[ ]` tasks, when
  `speccy next` runs (no `--kind`), then the result is a review
  for the `[?]` task -- the within-spec `[?]` preference wins
  over the `[ ]` tasks in the same spec.
- Given SPEC-0001 with only `[ ]` tasks and SPEC-0002 with `[?]`
  tasks, when `speccy next` runs, then the result is an implement
  for the `[ ]` task from SPEC-0001 -- the lowest-spec-ID rule
  wins over the within-spec preference.
- Given SPEC-0001 with only `[~]` (claimed) tasks and SPEC-0002
  with a `[ ]` task, then the result is an implement for the `[ ]`
  task from SPEC-0002 (SPEC-0001 has no actionable work for any
  caller).

**Covered by:** CHK-001, CHK-002

### REQ-002: `--kind implement` and `--kind review` filters

Filter strictly to the requested state class across all specs.

**Done when:**
- `--kind implement` walks specs in ascending order; within each
  spec, returns the first task with state `Open` (`[ ]`).
- `--kind review` walks specs in ascending order; within each
  spec, returns the first task with state `AwaitingReview`
  (`[?]`); the result includes the persona fan-out list.
- Neither filter falls back to the other kind. `--kind implement`
  with no `[ ]` tasks anywhere does NOT return a `[?]` task.
- If no task matches the filter, fall through to blocked detection
  (REQ-003).

**Behavior:**
- Given SPEC-0001 with one `[?]` task and no `[ ]` tasks, when
  `speccy next --kind implement` runs, then the result is
  `kind: blocked` (not a `[?]` review fallback).
- Given SPEC-0001 with one `[ ]` task and SPEC-0002 with one
  `[?]` task, when `speccy next --kind review` runs, then the
  result is the `[?]` from SPEC-0002 with persona fan-out
  `["business", "tests", "security", "style"]`.

**Covered by:** CHK-003, CHK-004

### REQ-003: Report and blocked kinds

When no actionable task exists, classify the workspace state.

**Done when:**
- If every task across every spec is `Done` (`[x]`) AND at least
  one spec lacks REPORT.md, return `kind: report` for the lowest-
  ID spec missing REPORT.md.
- If no task is actionable for the requested filter (or for any
  filter when none is supplied) AND no report is pending, return
  `kind: blocked` with a `reason` string describing the workspace
  state.

**Behavior:**
- Given SPEC-0001 with all `[x]` tasks and no `REPORT.md`, when
  `speccy next` runs, then the result is
  `kind: report, spec: "SPEC-0001"`.
- Given SPEC-0001 with all `[x]` tasks AND `REPORT.md` present,
  AND SPEC-0002 with all `[x]` tasks and no `REPORT.md`, then the
  result is `kind: report, spec: "SPEC-0002"`.
- Given SPEC-0001 with only `[~]` (claimed) tasks, when
  `speccy next --kind implement` runs, then the result is
  `kind: blocked, reason: "all open tasks are claimed by other
  sessions"`.
- Given an empty workspace (no specs), then the result is
  `kind: blocked, reason: "no specs in workspace"`.

**Covered by:** CHK-005, CHK-006

### REQ-004: JSON contract

`--json` emits a tagged union by `kind` following SPEC-0004's
envelope conventions.

**Done when:**
- Output begins with `"schema_version": 1`.
- The `"kind"` field discriminates four variants:
  - `"implement"`: `{ kind, spec, task, task_line, covers,
    suggested_files, prompt_command }`.
  - `"review"`: `{ kind, spec, task, task_line, personas,
    prompt_command_template }`.
  - `"report"`: `{ kind, spec, prompt_command }`.
  - `"blocked"`: `{ kind, reason }`.
- `prompt_command` and `prompt_command_template` follow the
  shapes in DESIGN.md "speccy next --json":
  - implement: `"speccy implement T-NNN"`.
  - review: `"speccy review T-NNN --persona {persona}"`.
  - report: `"speccy report SPEC-NNNN"`.
- Output is pretty-printed and deterministic given identical
  workspace state.
- A workspace with absent or malformed `.speccy/specs/` cleanly
  produces `kind: blocked, reason: "no specs in workspace"`
  rather than panicking.

**Behavior:**
- A `--kind review` result has `personas: ["business", "tests",
  "security", "style"]` (the v1 default fan-out, hardcoded per
  DEC-002).
- An `implement` result includes `covers: [<REQ-IDs>]` parsed
  from the `Covers:` bullet under the task line.
- A `blocked` result's `reason` string is one of a small set of
  canonical phrases (testable enum-like).

**Covered by:** CHK-007, CHK-008

### REQ-005: Text output

Text mode prints one line per kind variant.

**Done when:**
- `implement` kind prints one line:
  `next: implement T-NNN (SPEC-NNNN) -- <task_line>`.
- `review` kind prints one line:
  `next: review T-NNN (SPEC-NNNN) -- personas: business, tests, security, style`.
- `report` kind prints one line:
  `next: report SPEC-NNNN -- all tasks complete`.
- `blocked` kind prints one line:
  `next: blocked -- <reason>`.
- All four lines end with a newline; exit code is 0 in all four
  cases (blocked is not an error -- it's a valid state).

**Behavior:**
- Given each of the four kinds, text output matches the shape
  above. Snapshot tests assert per-kind.

**Covered by:** CHK-009

## Design

### Approach

The command lives in `speccy-cli/src/next.rs`. Priority logic
lives in `speccy-core/src/next.rs` as a pure function
taking the parsed `Workspace` and returning a `NextResult`. The
binary wraps it with output rendering.

Flow per invocation:

1. Discover project root via `workspace::find_root` (SPEC-0004).
2. Scan workspace via `workspace::scan`.
3. Apply priority logic (REQ-001 / REQ-002 / REQ-003) -> produce
   `NextResult` enum.
4. Render text (one line) or JSON (tagged union) per the flags.

### Decisions

#### DEC-001: Walk specs ascending; within-spec, `[?]` before `[ ]`

**Status:** Accepted (per DESIGN.md "speccy next priority")
**Context:** Two competing orderings exist: by spec ID
(stable, locality of work) and by task age / priority (which
speccy doesn't track). DESIGN.md picks spec ID.
**Decision:** Lowest spec ID first; within a spec, `[?]` review-
ready tasks before `[ ]` open tasks. `--kind` filters override
the within-spec preference.
**Alternatives:**
- Task age (oldest first) -- rejected. Speccy doesn't track
  creation time per task.
- Random selection -- rejected. Non-deterministic; breaks
  harnesses.
**Consequences:** Stable, predictable selection. Reviews don't
accumulate (the within-spec `[?]` preference). Lowest-ID-first
locality means a developer who started on SPEC-0001 stays there
until done before SPEC-0002 demands attention.

#### DEC-002: Persona fan-out is hardcoded in v1

**Status:** Accepted (per DESIGN.md "Phase 4: Review loop")
**Context:** DESIGN.md says: "The default reviewer persona fan-
out is: business, tests, security, style. The other personas
(architecture, docs) are available via `--persona` but not in
the default fan-out. Projects can override the default set in
`speccy.toml` later if necessary; v1 ships with this default."
**Decision:** Hardcode `["business", "tests", "security",
"style"]` as the fan-out list returned for `kind: review`
results. No project-level config in v1.
**Alternatives:**
- Configurable via `speccy.toml` -- rejected for v1. Matches the
  DESIGN.md stance and "no `--strict` mode" philosophy.
**Consequences:** All v1 deployments get the same review
discipline. Future config knob can override.

#### DEC-003: `blocked` carries a `reason` string, not an enum

**Status:** Accepted
**Context:** Several distinct workspace states all map to
"blocked": all tasks claimed by other sessions, no specs at all,
no `[ ]` tasks but no `[?]` either, etc. An enum would force
exhaustive matching for marginal value; a string is more
flexible.
**Decision:** `kind: blocked` carries a free-form `reason:
String`. Tests pin a small set of canonical phrases for
predictability.
**Alternatives:**
- Enumerated reason codes -- rejected. Adds API surface without
  adding actionable information for harnesses (which mostly want
  to know "no work right now" and decide what to do).
**Consequences:** Reasons are testable as exact strings; future
reasons can be added without breaking JSON schema.

#### DEC-004: `--kind` filters strictly; no fallback to the other kind

**Status:** Accepted
**Context:** When `--kind implement` finds no `[ ]` tasks, should
the result fall back to `[?]` review tasks? That would surprise
harnesses expecting strict semantics.
**Decision:** `--kind implement` returns only `kind: implement`
or `kind: blocked` (never `kind: review`). `--kind review`
symmetrically returns only `kind: review` or `kind: blocked`.
The `report` kind can be returned by either filter when all
tasks are `[x]` (since report is the natural successor to "no
work left").
**Alternatives:**
- Soft fallback to the other kind -- rejected. Surprising
  semantics for skill writers.
**Consequences:** Skills can rely on the kind matching their
filter. `/speccy:work` knows it won't accidentally get a review
task.

### Interfaces

```rust
// speccy-core additions
pub mod next {
    pub fn compute(
        workspace: &Workspace,
        kind_filter: Option<KindFilter>,
    ) -> NextResult;
}

pub enum KindFilter { Implement, Review }

pub enum NextResult {
    Implement {
        spec: String,
        task: String,
        task_line: String,
        covers: Vec<String>,
        suggested_files: Vec<String>,
    },
    Review {
        spec: String,
        task: String,
        task_line: String,
        personas: &'static [&'static str],
    },
    Report { spec: String },
    Blocked { reason: String },
}

pub const DEFAULT_PERSONAS: &[&str] =
    &["business", "tests", "security", "style"];

// speccy binary
pub fn run(args: NextArgs) -> Result<(), NextError>;

pub struct NextArgs {
    pub kind: Option<KindFilter>,
    pub json: bool,
}

pub enum NextError {
    ProjectRootNotFound,
    Workspace(WorkspaceError),
}
```

### Data changes

- New `speccy-core/src/next.rs` (priority logic + types).
- New `speccy-cli/src/next.rs` (command logic).
- New `speccy-cli/src/next_output.rs` (text + JSON renderers).

### Migration / rollback

Greenfield. Rollback via `git revert`. Depends on SPEC-0001
(parsers), SPEC-0004 (workspace::scan + JSON envelope conventions)
-- both deepened.

## Open questions

- [ ] Should `kind: report` be returnable under `--kind implement`
  or `--kind review`, or only under the no-filter form? Returning
  under either filter is more useful (the harness gets a "wrap
  up" signal even when scoped); deferring the exact contract to
  implementer.
- [ ] Should the persona fan-out be exposed as a CLI flag (e.g.
  `--personas business,security`) for one-off overrides? Not v1.
- [ ] Should `--kind review` also accept `--persona <name>` to
  return a single-persona pending review (rather than a full
  fan-out)? Not v1; reviewers iterate over the full fan-out.

## Assumptions

- `speccy_core::workspace::scan` from SPEC-0004 surfaces task
  state per spec; the next-command consumer doesn't re-parse.
- Task `Covers:` bullets in TASKS.md are parsed by the SPEC-0001
  parser into `Task.covers: Vec<String>`.
- Suggested-files bullets are parsed similarly into
  `Task.suggested_files: Vec<String>`.
- An empty workspace (no specs at all) returns `kind: blocked`
  rather than erroring -- consistent with SPEC-0004's empty-
  workspace behaviour.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from DESIGN.md decomposition. |

## Notes

`speccy next` is the smallest spec in the workflow-query side of
the CLI, but the JSON contract is the most-consumed by skills:
`/speccy:work` and `/speccy:review` both loop on its output.
The four `kind` variants and their field shapes are part of the
durable contract; adding fields is non-breaking, removing or
renaming bumps `schema_version`.

The `report` kind is a one-time signal -- once REPORT.md is
written for a spec, that spec is no longer reported. When
SPEC-0011 (`report`) lands, it should write a stub REPORT.md so
the next-command stops nagging.
