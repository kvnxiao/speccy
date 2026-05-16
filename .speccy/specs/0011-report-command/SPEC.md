---
id: SPEC-0011
slug: report-command
title: speccy report -- render Phase 5 prompt for a completed spec
status: implemented
created: 2026-05-11
---

# SPEC-0011: speccy report

## Summary

`speccy report SPEC-NNNN` is the Phase 5 command. It renders the
prompt an agent reads to write `REPORT.md` -- the durable summary
of what shipped for one spec. The CLI inlines SPEC.md, TASKS.md
(with all task notes), AGENTS.md, and a per-task retry count
derived from `Retry:` markers in the inline notes.

The command **refuses** to render the prompt if any task is not
`[x]`. Phase 5 is the moment of "wrap-up"; partial reports for
in-flight specs are out of scope for v1. Specs that get
abandoned should be marked `status: dropped` in their frontmatter
(per SPEC-0001 SpecFrontmatter status), not reported via this
command.

## Goals

- One CLI surface for the Phase 5 prompt.
- Completeness gate forces all tasks to be `[x]` before the
  prompt renders.
- Retry counts per task are computed deterministically from
  `Retry:` markers in already-parsed notes.
- Reuse SPEC-0005's prompt infrastructure.

## Non-goals

- No PR opening. The orchestrating skill (`/speccy:ship`) calls
  `gh` after the report is written.
- No partial-report mode in v1. Abandoned specs use
  `status: dropped`.
- No automatic REPORT.md writing. The agent writes the file
  after reading the prompt.
- No retry-cause analysis. The retry count is a number; agents
  reading review notes can deduce the cause.

## User stories

- As `/speccy:ship`, I want `speccy report SPEC-0001` to render
  a prompt that asks the agent to summarize what happened in
  one loop -- requirements satisfied, tasks completed,
  out-of-scope items absorbed, deferred limitations.
- As a developer reviewing a PR, I want REPORT.md (which the
  agent writes from this prompt) to honestly reflect retry
  counts so I can see which tasks were thrashed.
- As a future SPEC-0007 (next) consumer, I want `speccy report`
  to be the ONLY way to transition a spec to "ready for PR" so
  the report kind is meaningful.

## Requirements

<!-- speccy:requirement id="REQ-001" -->
### REQ-001: SPEC-ID validation and spec lookup

Validate the argument and locate the spec.

**Done when:**
- `^SPEC-\d{4,}$` -> proceed; anything else returns exit code 1
  with a format-error message.
- The matching spec directory is located via
  `workspace::scan`; missing returns exit code 1 with a
  "spec not found" message naming the ID.
- SPEC.md and TASKS.md are both required; either missing or
  failing to parse returns exit code 1 with a clear error.

**Behavior:**
- `speccy report FOO` -> exit 1, format error.
- `speccy report SPEC-9999` -> exit 1, spec-not-found.
- `speccy report SPEC-0001` (TASKS.md missing) -> exit 1,
  TASKS.md-required error.

<!-- speccy:scenario id="CHK-001" -->
- `speccy report FOO` -> exit 1, format error.
- `speccy report SPEC-9999` -> exit 1, spec-not-found.
- `speccy report SPEC-0001` (TASKS.md missing) -> exit 1,
  TASKS.md-required error.

speccy report validates SPEC-ID format; spec-not-found exits 1; missing or malformed SPEC.md / TASKS.md exits 1 with clear error messages.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-002" -->
### REQ-002: Completeness gate

Refuse to render if any task is not `[x]`.

**Done when:**
- The command counts tasks by state.
- If any task has state `Open`, `InProgress`, or
  `AwaitingReview`, exit code is 1 with a stderr message
  listing the offending task IDs and their states.
- Only when every task is `Done` (`[x]`) does the prompt render.
- Empty TASKS.md (no tasks) is treated as "complete" (vacuously
  true; the agent can still write a meaningful report).

**Behavior:**
- Given SPEC-0001 with 5 `[x]` tasks and 1 `[ ]` task, exit
  code is 1 and stderr names the `[ ]` task.
- Given SPEC-0001 with one `[~]` task, exit code 1; stderr
  names it as InProgress.
- Given all 6 tasks are `[x]`, the prompt renders.
- Given TASKS.md has no task lines at all, the prompt renders
  (vacuous completeness).

<!-- speccy:scenario id="CHK-002" -->
- Given SPEC-0001 with 5 `[x]` tasks and 1 `[ ]` task, exit
  code is 1 and stderr names the `[ ]` task.
- Given SPEC-0001 with one `[~]` task, exit code 1; stderr
  names it as InProgress.
- Given all 6 tasks are `[x]`, the prompt renders.
- Given TASKS.md has no task lines at all, the prompt renders
  (vacuous completeness).

Refuses with exit 1 when any task is [ ], [~], or [?]; renders the prompt only when all tasks are [x]; empty TASKS.md (no tasks) is treated as vacuously complete.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-003" -->
### REQ-003: Retry count computation per task

Count `Retry:` markers per task from inline notes.

**Done when:**
- For each task in TASKS.md, count notes (`Task.notes` from
  SPEC-0001 REQ-004) that begin with `Retry:` (case-sensitive,
  exact prefix after optional leading whitespace and bullet
  marker stripping handled by the parser).
- Surface as a `Vec<(task_id, retry_count)>` available to the
  prompt template via the `{{retry_summary}}` placeholder
  (rendered as a markdown list).

**Behavior:**
- Given T-001 has notes
  `["Implementer note: ...", "Review (business, pass): ...",
  "Retry: address bcrypt cost.", "Implementer note: ...",
  "Retry: fix style."]`, the count is 2.
- Given T-002 has zero `Retry:` notes, the count is 0.
- The `{{retry_summary}}` rendering:
  ```
  - T-001: 2 retries
  - T-002: 0 retries
  ```

<!-- speccy:scenario id="CHK-003" -->
- Given T-001 has notes
  `["Implementer note: ...", "Review (business, pass): ...",
  "Retry: address bcrypt cost.", "Implementer note: ...",
  "Retry: fix style."]`, the count is 2.
- Given T-002 has zero `Retry:` notes, the count is 0.
- The `{{retry_summary}}` rendering:
  ```
  - T-001: 2 retries
  - T-002: 0 retries
  ```

Retry count per task equals the number of notes beginning with 'Retry:' (case-sensitive prefix); rendered as a markdown list under the retry_summary placeholder.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-004" -->
### REQ-004: Render report prompt

Render the Phase 5 prompt to stdout.

**Done when:**
- Loads `report.md` template via `prompt::load_template`.
- Substitutes placeholders: `{{spec_id}}`, `{{spec_md}}`,
  `{{tasks_md}}` (full content), `{{retry_summary}}`,
  `{{agents}}`.
- Trims to budget via `prompt::trim_to_budget`.
- Writes the trimmed output to stdout; exit code 0.

**Behavior:**
- Given the completeness gate passes, the prompt renders with
  all placeholders substituted.
- Retry summary appears where `{{retry_summary}}` was.

<!-- speccy:scenario id="CHK-004" -->
- Given the completeness gate passes, the prompt renders with
  all placeholders substituted.
- Retry summary appears where `{{retry_summary}}` was.

When completeness passes, report.md template loaded with spec_id, spec_md, tasks_md, retry_summary, agents placeholders substituted; budget trimming applied; output to stdout.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
## Design

### Approach

The command lives in `speccy-cli/src/report.rs`. The retry
counter is a small helper in the same file -- no shared
abstraction needed for v1 since no other command computes
retries.

Flow per invocation:

1. Discover project root.
2. Parse SPEC-ID; locate spec.
3. Parse SPEC.md and TASKS.md.
4. Run the completeness gate.
5. Compute retry counts per task.
6. Render the prompt with all inlines.
7. Write to stdout.

### Decisions

<!-- speccy:decision id="DEC-001" status="accepted" -->
#### DEC-001: All tasks must be `[x]`; no partial-report flag in v1

**Status:** Accepted
**Context:** Two valid postures: strict (only complete specs
get a report) and lenient (partial reports for abandoned work).
**Decision:** Strict. Specs with incomplete tasks must either
finish the tasks or transition to `status: dropped` (handled
elsewhere). The CLI refuses to render otherwise.
**Alternatives:**
- `--allow-incomplete` flag -- rejected for v1. Adds a knob
  without a strong motivating use case; the dropped status
  already covers abandoned work.
**Consequences:** The orchestrating skill (`/speccy:ship`) can
trust that "if `speccy report` succeeds, the spec is done."
<!-- /speccy:decision -->
<!-- speccy:decision id="DEC-002" status="accepted" -->
#### DEC-002: Retry count = `Retry:` marker occurrences in notes

**Status:** Accepted (per ARCHITECTURE.md "TASKS.md State Model")
**Context:** Review notes follow conventions like
`Retry: <reason>`. Counting them is a straightforward
proxy for "how many times did this task come back."
**Decision:** Count notes that begin with the exact prefix
`Retry:` (after parser-handled bullet stripping).
**Alternatives:**
- Count state-transition events from git log -- rejected.
  Speccy doesn't index git history.
- Parse review (blocking, ...) markers -- rejected. Less
  direct; a task could be blocked without being retried (e.g.
  blocked then abandoned).
**Consequences:** A task that's been retried but had its
`Retry:` notes manually edited out wouldn't count -- which is
fine; the agent can always edit the report.
<!-- /speccy:decision -->
<!-- speccy:decision id="DEC-003" status="accepted" -->
#### DEC-003: `status: dropped` does NOT bypass the completeness gate

**Status:** Accepted for v1; revisitable
**Context:** A spec marked `status: dropped` is abandoned. The
question: should `speccy report` work on dropped specs?
**Decision:** For v1, no. Dropped specs don't need a report;
their abandonment is documented in the SPEC.md `## Changelog`
table (per ARCHITECTURE.md). The CLI's completeness gate applies
uniformly regardless of status.
**Alternatives:**
- Allow report for `status: dropped` -- rejected for v1.
  Conflates two outcomes (delivered vs abandoned).
**Consequences:** Abandoned specs end at the SPEC.md changelog
row noting why; no REPORT.md is written. Revisit if a real
workflow needs the contrary.
<!-- /speccy:decision -->
### Interfaces

```rust
pub fn run(args: ReportArgs) -> Result<(), ReportError>;

pub struct ReportArgs { pub spec_id: String }

pub enum ReportError {
    InvalidSpecIdFormat { arg: String },
    SpecNotFound { id: String },
    TasksMdRequired { spec_id: String },
    Incomplete { offending: Vec<(String, TaskState)> },
    ProjectRootNotFound,
    Prompt(PromptError),
    Parse(ParseError),
}
```

### Data changes

- New `speccy-cli/src/report.rs`.
- New embedded template `skills/shared/prompts/report.md`
  (stub; SPEC-0013 fills real content).

### Migration / rollback

Greenfield. Depends on SPEC-0001, SPEC-0004, SPEC-0005.

## Open questions

- [ ] Should `--allow-incomplete` exist as an escape hatch for
  edge cases? Not v1.
- [ ] Should retry-count rendering distinguish "task retried N
  times" from "task with N blocking reviews that didn't lead
  to retry"? No for v1; agents can read review notes for
  nuance.

## Assumptions

- SPEC-0001's TASKS.md parser exposes task state and notes per
  REQ-004.
- SPEC-0005's prompt helpers are available.
- SPEC-0004's `workspace::scan` is available for spec lookup.

## Changelog

<!-- speccy:changelog -->
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
<!-- /speccy:changelog -->

## Notes

SPEC-0011 is the smallest command spec in the workflow. Its job
is mostly to gate (completeness) and render. The intelligence
(what should REPORT.md actually say) lives in the
`skills/shared/prompts/report.md` template -- SPEC-0013's
concern.

When SPEC-0007 (`next`) returns `kind: report` for a spec, the
orchestrating skill should call this command. The two specs
agree on the "spec is done" semantics.
