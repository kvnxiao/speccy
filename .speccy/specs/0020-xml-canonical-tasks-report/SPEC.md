---
id: SPEC-0020
slug: xml-canonical-tasks-report
title: Marker-structured TASKS.md and REPORT.md
status: in-progress
created: 2026-05-15
supersedes: []
---

# SPEC-0020: Marker-structured TASKS.md and REPORT.md

## Summary

SPEC-0019 makes `SPEC.md` the single canonical carrier for
requirements and validation scenarios. Two artifacts still carry
load-bearing orchestration state through Markdown conventions:

- `TASKS.md` carries task ids, task state, requirement coverage, and
  task-local testing expectations.
- `REPORT.md` closes the loop with outcome prose and requirement
  coverage.

The current parser extracts this structure from checkbox prefixes,
bold ids, section headings, and indented bullets. That works well
enough for humans, but it is fragile as a substrate for long-running
multi-agent orchestration. One stray indentation level or slightly
different "Covers:" line can change what the CLI thinks is actionable.

This spec applies SPEC-0019's marker-comment carrier to `TASKS.md` and
`REPORT.md`. The files remain normal Markdown. Speccy-owned structure
is carried by line-isolated comments such as:

```markdown
<!-- speccy:task id="T-001" state="pending" covers="REQ-001 REQ-002" -->
## T-001: Implement scenario rendering

<!-- speccy:task-scenarios -->
Given a task covers REQ-001,
when the implementer finishes,
then tests or reviewer validation prove the selected scenario renders.
<!-- /speccy:task-scenarios -->
<!-- /speccy:task -->
```

This spec deliberately does **not** introduce first-class handoff
schema. Implementer notes, review notes, commands run, and friction
records remain ordinary Markdown for now. A future spec can harden
handoffs once the core contract carrier is settled.

This is the third step in the sequence:

- **SPEC-0018:** checks become English validation scenarios and Speccy
  stops executing commands.
- **SPEC-0019:** `SPEC.md` becomes marker-structured and per-spec
  `spec.toml` disappears.
- **SPEC-0020:** `TASKS.md` and `REPORT.md` use the same marker style
  for task state, task-local scenarios, and report coverage.

## Goals

- `TASKS.md` has deterministic markers for task id, state, covered
  requirements, and task-local validation scenarios.
- `REPORT.md` has deterministic markers for per-requirement outcomes
  and the scenarios claimed as satisfied or deferred.
- Existing Markdown remains readable in GitHub and terminal output.
- The workspace loader no longer depends on checkbox or bullet-shape
  heuristics for core task/report state.
- Migration from current Markdown conventions is mechanical.
- Architecture docs and shipped prompts teach the marker shape.

## Non-goals

- No first-class handoff schema. Do not add structured
  `<implementer-note>` fields, command-run ledgers, exit-code
  subfields, or procedural-compliance tags in this spec.
- No review-note schema. Review notes may remain free Markdown inside
  task bodies until a future spec proves the need for a typed shape.
- No `speccy task append-note` helper.
- No raw XML containers. Like SPEC-0019, this uses XML-style marker
  comments around Markdown.
- No new task states. The four existing states map to marker
  attributes:
  - `[ ]` -> `pending`
  - `[~]` -> `in-progress`
  - `[?]` -> `in-review`
  - `[x]` -> `completed`

## User Stories

- As an orchestrator agent, I want to read task state from a typed
  marker attribute instead of a Markdown checkbox prefix.
- As an implementer agent, I want each task to carry task-local
  Given-When-Then validation prose so I know what tests or local
  checks to add for the slice.
- As a reviewer persona, I want task-local scenarios and SPEC-level
  scenarios to be distinct: the former checks the slice, the latter
  checks the user-facing requirement.
- As a human, I want TASKS.md and REPORT.md to remain readable
  Markdown, with structural markers visible in source but not
  distracting in rendered views.

## Requirements

### REQ-001: TASKS.md marker grammar

`TASKS.md` uses marker comments for task state and task coverage. The
body inside each task remains Markdown.

**Done when:**
- `TASKS.md` has a root marker emitted by the renderer:
  `<!-- speccy:tasks spec="SPEC-NNNN" -->`.
- Each task is wrapped by:
  `<!-- speccy:task id="T-NNN" state="..." covers="REQ-NNN REQ-MMM" -->`
  and `<!-- /speccy:task -->`.
- Valid task states are exactly:
  `pending`, `in-progress`, `in-review`, `completed`.
- `covers` is required and contains one or more `REQ-\d{3,}` ids
  separated by spaces.
- Every covered requirement id is cross-checked against the parent
  SPEC.md marker tree at workspace load time.
- Each task contains exactly one `speccy:task-scenarios` marker block
  with non-empty Markdown prose.
- Task ids are unique within one TASKS.md.
- Unknown task-marker attributes are parse errors.
- Markdown outside Speccy markers is preserved as task body prose and
  is not interpreted as structure.

**Behavior:**
- Given a task marker with `state="pending"` and
  `covers="REQ-001 REQ-002"`, parsing returns a task with those ids
  and state.
- Given a task marker with `state="done"`, parsing fails and lists the
  valid states.
- Given a task marker with no `task-scenarios` block, parsing fails.
- Given a task covering `REQ-999` when the parent SPEC has no
  `REQ-999`, workspace loading fails with a dangling requirement
  reference.

**Covered by:** CHK-001

### REQ-002: REPORT.md marker grammar

`REPORT.md` uses marker comments for requirement coverage. Outcome
and narrative sections remain Markdown.

**Done when:**
- `REPORT.md` has a root marker emitted by the renderer:
  `<!-- speccy:report spec="SPEC-NNNN" -->`.
- Each requirement outcome is wrapped by:
  `<!-- speccy:coverage req="REQ-NNN" result="..." scenarios="CHK-NNN CHK-MMM" -->`
  and `<!-- /speccy:coverage -->`.
- Valid coverage results are:
  `satisfied`, `partial`, `deferred`, `dropped`.
- `scenarios` is required for `satisfied` and `partial`, and may be
  empty for `deferred` or `dropped`.
- Coverage requirement ids must exist in the parent SPEC.md.
- Scenario ids listed in a coverage marker must be nested under the
  matching requirement in the parent SPEC.md.
- Every non-dropped requirement in the parent SPEC.md must have one
  coverage marker in REPORT.md.
- Markdown inside each coverage block is preserved as explanatory
  prose.

**Behavior:**
- Given a coverage marker for `REQ-001` with
  `result="satisfied"` and `scenarios="CHK-001"`, parsing returns a
  coverage row linked to that requirement and scenario.
- Given `result="passed"`, parsing fails and lists the valid result
  values.
- Given `REQ-001` coverage listing `CHK-099` when the SPEC has no
  such scenario under REQ-001, workspace loading fails with a dangling
  scenario reference.

**Covered by:** CHK-002

### REQ-003: Parsers and renderers reuse the marker infrastructure

TASKS.md and REPORT.md use the same marker-comment scanner style
introduced by SPEC-0019.

**Done when:**
- `speccy-core::parse::task_markers` exposes typed `TasksDoc` and
  `Task` models plus `parse` and `render`.
- `speccy-core::parse::report_markers` exposes typed `ReportDoc` and
  `RequirementCoverage` models plus `parse` and `render`.
- Shared marker parsing utilities live in a common module used by
  SPEC, TASKS, and REPORT parsing.
- Parse errors include path and byte offset for malformed markers,
  unknown attributes, duplicate ids, invalid states, invalid coverage
  results, and missing required marker blocks.
- Parse/render/parse round-trips preserve the typed task/report
  structure.
- Existing public APIs for `speccy next`, `speccy status`,
  `speccy implement`, `speccy review`, and `speccy report` keep their
  external behavior while reading from the new typed models.

**Behavior:**
- Given a canonical TASKS.md, parse/render/parse yields equivalent
  task ids, states, covers arrays, and task scenario bodies.
- Given a canonical REPORT.md, parse/render/parse yields equivalent
  requirement coverage rows.
- Given a Speccy marker inside a fenced code block, it is treated as
  Markdown body content, not structure.

**Covered by:** CHK-003

### REQ-004: Migration rewrites in-tree TASKS.md and REPORT.md

An ephemeral migration tool converts existing files to the marker
shape.

**Done when:**
- `xtask/migrate-task-report-markers-0020` exists during
  implementation and is deleted before the final commit.
- TASKS.md migration:
  - frontmatter and level-1 heading are preserved;
  - each checkbox task becomes a `speccy:task` marker block;
  - checkbox state maps to the `state` attribute;
  - `Covers:` lines become the `covers` attribute;
  - existing `Tests to write:` or equivalent task-local behavior
    prose becomes the `speccy:task-scenarios` block;
  - suggested files, implementer notes, retry notes, and review notes
    remain Markdown body content, not typed schema.
- REPORT.md migration:
  - frontmatter and level-1 heading are preserved;
  - each requirements-coverage table row becomes a
    `speccy:coverage` marker block;
  - existing outcome, task summary, skill updates, out-of-scope, and
    deferred-limitations sections remain Markdown.
- Migration fails when it cannot determine task coverage, task state,
  or report coverage without guessing.
- After migration, `speccy verify` exits 0.

**Behavior:**
- Given an existing task `- [?] **T-003**` covering `REQ-002`, the
  migrated task marker has `id="T-003"`, `state="in-review"`, and
  `covers="REQ-002"`.
- Given a task with no test/validation prose, migration fails and
  names the task rather than inventing a scenario.
- Given an existing report row for `REQ-001`, migration creates one
  coverage marker with the mapped result and scenario ids.

**Covered by:** CHK-004

### REQ-005: Docs and shipped skills use marker-structured tasks and reports

The skill layer must stop teaching the old checkbox/bullet grammar as
the machine contract.

**Done when:**
- `.speccy/ARCHITECTURE.md` documents the TASKS.md and REPORT.md
  marker grammars.
- The four task states are documented as marker attribute values, with
  old checkbox markers listed only as migration history.
- `resources/modules/prompts/tasks-generate.md` emits marker-structured
  TASKS.md.
- `resources/modules/prompts/report.md` emits marker-structured
  REPORT.md.
- Implementer and reviewer prompts read task state and task scenarios
  from markers.
- Active shipped guidance does not require structured handoff fields.
  It may still ask implementers and reviewers to leave clear Markdown
  notes.

**Behavior:**
- Given a freshly generated TASKS.md after this spec lands, task ids,
  states, covers, and task scenarios are all represented by Speccy
  marker comments.
- Given a freshly generated REPORT.md, requirement coverage is
  represented by coverage markers.
- Given a grep for old active guidance that treats checkbox bullets as
  the machine contract, there are no hits except historical migration
  notes.

**Covered by:** CHK-005

## Design

### Approach

Implementation order:

1. Extract shared marker parsing utilities from SPEC-0019 if needed.
2. Add TASKS.md and REPORT.md marker parsers/renderers with fixtures.
3. Write and test the migration tool against the most complex in-tree
   TASKS.md and REPORT.md.
4. Switch workspace loading, `next`, `status`, implement/review/report
   prompt rendering, and verify to the typed models.
5. Delete heuristic body-scanner logic that parsed checkboxes and
   coverage tables as machine contract.
6. Sweep docs and shipped skills.
7. Delete the migration tool.

### Decisions

#### DEC-001: Marker state is canonical

**Status:** Accepted

**Context:** Keeping both checkbox state and marker state would create
another drift vector.

**Decision:** The marker attribute is the canonical task state after
this spec lands. The renderer may include human-readable state text in
the Markdown body, but the parser ignores it.

**Consequences:** State changes are single-attribute edits, and
`speccy next` no longer depends on checkbox parsing.

#### DEC-002: Task-local scenarios are required

**Status:** Accepted

**Context:** SPEC-level scenarios describe user-facing behavior.
Tasks also need slice-level validation instructions that implementers
and reviewers can act on.

**Decision:** Every task requires one non-empty `task-scenarios` block.

**Consequences:** Task decomposition becomes more useful to workers
and validators, and task quality is reviewed in prose rather than
through executable commands.

#### DEC-003: Handoffs stay out of scope

**Status:** Accepted

**Context:** Structured handoffs are useful, but adding them here would
mix core artifact hardening with a new workflow artifact.

**Decision:** This spec does not parse or require handoff fields.

**Consequences:** The substrate hardening can land first. A later spec
can add first-class handoffs without changing task state and coverage
markers again.

## Migration / Rollback

Migration is structural and fails closed when the current Markdown does
not contain enough information. The migration tool should not invent
task scenarios or report coverage.

Rollback is `git revert` of the implementation commit set. The old
checkbox and report-table conventions remain in history.

## Open Questions

- [ ] Should `TASKS.md` keep a visible checkbox glyph generated from
      marker state for human scanning? Lean no; use visible state text
      only if dogfooding shows task lists are too hard to scan.
- [ ] Should REPORT.md require coverage for dropped requirements?
      Lean no; dropped requirements should be visible in SPEC.md status
      or changelog rather than report coverage.

## Assumptions

- Existing TASKS.md files have enough `Covers:` and `Tests to write:`
  prose to migrate without inventing validation scenarios.
- Existing REPORT.md files have enough requirements coverage table data
  to migrate coverage markers.
- SPEC-0019's marker parser has already established fenced-code
  handling and deterministic marker rendering.

## Changelog

| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial rewritten draft. Applies marker comments to TASKS.md and REPORT.md while deferring first-class handoffs. |

## Notes

This spec is intentionally narrower than the previous draft. It
hardens shared state and validation prose, which harnesses need
immediately, while leaving handoff schemas to a later design once the
core carrier is proven through dogfooding.
