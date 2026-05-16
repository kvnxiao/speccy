---
id: SPEC-0021
slug: xml-canonical-tasks-report
title: Raw XML element tags for TASKS.md and REPORT.md
status: in-progress
created: 2026-05-15
supersedes: []
---

# SPEC-0021: Raw XML element tags for TASKS.md and REPORT.md

## Summary

SPEC-0020 makes raw XML element tags the canonical carrier for
`SPEC.md`. Two artifacts still carry load-bearing orchestration state
through Markdown conventions:

- `TASKS.md` carries task ids, task state, requirement coverage, and
  task-local testing expectations.
- `REPORT.md` closes the loop with outcome prose and requirement
  coverage.

The current parser extracts this structure from checkbox prefixes,
bold ids, section headings, and indented bullets. That works well
enough for humans, but it is fragile as a substrate for long-running
multi-agent orchestration. One stray indentation level or slightly
different "Covers:" line can change what the CLI thinks is actionable.

This spec applies SPEC-0020's raw XML element carrier to `TASKS.md`
and `REPORT.md`. The files remain normal Markdown. Speccy-owned
structure is carried by line-isolated XML element tags such as:

```markdown
<task id="T-001" state="pending" covers="REQ-001 REQ-002">
## T-001: Implement scenario rendering

<task-scenarios>
Given a task covers REQ-001,
when the implementer finishes,
then tests or reviewer validation prove the selected scenario renders.
</task-scenarios>
</task>
```

This spec deliberately does **not** introduce first-class handoff
schema. Implementer notes, review notes, commands run, and friction
records remain ordinary Markdown for now. A future spec can harden
handoffs once the core contract carrier is settled.

This is the fourth step in the sequence:

- **SPEC-0018:** checks become English validation scenarios and Speccy
  stops executing commands.
- **SPEC-0019:** `SPEC.md` becomes single-carrier with HTML-comment
  markers; per-spec `spec.toml` disappears.
- **SPEC-0020:** `SPEC.md` switches from HTML-comment markers to raw
  XML element tags so vendor-recommended prompt structure applies.
- **SPEC-0021:** `TASKS.md` and `REPORT.md` use the same raw XML
  element style for task state, task-local scenarios, and report
  coverage.

## Goals

- `TASKS.md` has deterministic XML elements for task id, state,
  covered requirements, and task-local validation scenarios.
- `REPORT.md` has deterministic XML elements for per-requirement
  outcomes and the scenarios claimed as satisfied or deferred.
- Existing Markdown remains readable in terminal output; the GitHub
  render trade-off accepted in SPEC-0020 also applies here.
- The workspace loader no longer depends on checkbox or bullet-shape
  heuristics for core task/report state.
- Migration from current Markdown conventions is mechanical.
- Architecture docs and shipped prompts teach the XML element shape.

## Non-goals

- No first-class handoff schema. Do not add structured
  `<implementer-note>` fields, command-run ledgers, exit-code
  subfields, or procedural-compliance tags in this spec.
- No review-note schema. Review notes may remain free Markdown inside
  task bodies until a future spec proves the need for a typed shape.
- No `speccy task append-note` helper.
- No HTML-comment markers. Like SPEC-0020, this uses raw XML element
  tags around Markdown bodies.
- No new task states. The four existing states map to attribute
  values:
  - `[ ]` -> `pending`
  - `[~]` -> `in-progress`
  - `[?]` -> `in-review`
  - `[x]` -> `completed`

## User Stories

- As an orchestrator agent, I want to read task state from a typed
  XML attribute instead of a Markdown checkbox prefix.
- As an implementer agent, I want each task to carry task-local
  Given-When-Then validation prose so I know what tests or local
  checks to add for the slice.
- As a reviewer persona, I want task-local scenarios and SPEC-level
  scenarios to be distinct: the former checks the slice, the latter
  checks the user-facing requirement.
- As a human, I want TASKS.md and REPORT.md to remain readable
  Markdown source, with XML element tags visible as structure anchors.

## Requirements

<requirement id="REQ-001">
### REQ-001: TASKS.md XML element grammar

`TASKS.md` uses raw XML element tags for task state and task
coverage. The body inside each task remains Markdown.

**Done when:**
- `TASKS.md` has a root `<tasks>` element emitted by the
  renderer carrying a `spec="SPEC-NNNN"` attribute.
- Each task is wrapped by a `<task>` element carrying `id`,
  `state`, and `covers` attributes, closed by `</task>`.
- Valid task states are exactly:
  `pending`, `in-progress`, `in-review`, `completed`.
- `covers` is required and contains one or more `REQ-\d{3,}` ids
  separated by spaces.
- Every covered requirement id is cross-checked against the parent
  SPEC.md element tree at workspace load time.
- Each task contains exactly one `<task-scenarios>` block with
  non-empty Markdown prose.
- Task ids are unique within one TASKS.md.
- Unknown task-element attributes are parse errors.
- Markdown outside Speccy elements is preserved as task body prose
  and is not interpreted as structure.

**Behavior:**
- Given a task element with `state="pending"` and
  `covers="REQ-001 REQ-002"`, parsing returns a task with those ids
  and state.
- Given a task element with `state="done"`, parsing fails and lists
  the valid states.
- Given a task element with no `task-scenarios` block, parsing fails.
- Given a task covering `REQ-999` when the parent SPEC has no
  `REQ-999`, workspace loading fails with a dangling requirement
  reference.

<scenario id="CHK-001">
Given a TASKS.md with a task element containing id, state, and covers
attributes,
when the task element parser runs,
then it returns a typed task with the expected id, state, and covered
requirements.

Given a task element with an invalid state,
when parsing runs,
then parsing fails and lists the valid task states.

Given a task element without a task-scenarios block,
when parsing runs,
then parsing fails and names the task.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: REPORT.md XML element grammar

`REPORT.md` uses raw XML element tags for requirement coverage.
Outcome and narrative sections remain Markdown.

**Done when:**
- `REPORT.md` has a root `<report>` element emitted by the
  renderer carrying a `spec="SPEC-NNNN"` attribute.
- Each requirement outcome is wrapped by a `<coverage>`
  element carrying `req`, `result`, and `scenarios` attributes,
  closed by `</coverage>`.
- Valid coverage results are:
  `satisfied`, `partial`, `deferred`, `dropped`.
- `scenarios` is required for `satisfied` and `partial`, and may be
  empty for `deferred` or `dropped`.
- Coverage requirement ids must exist in the parent SPEC.md.
- Scenario ids listed in a coverage element must be nested under the
  matching requirement in the parent SPEC.md.
- Every non-dropped requirement in the parent SPEC.md must have one
  coverage element in REPORT.md.
- Markdown inside each coverage block is preserved as explanatory
  prose.

**Behavior:**
- Given a coverage element for `REQ-001` with
  `result="satisfied"` and `scenarios="CHK-001"`, parsing returns a
  coverage row linked to that requirement and scenario.
- Given `result="passed"`, parsing fails and lists the valid result
  values.
- Given `REQ-001` coverage listing `CHK-099` when the SPEC has no
  such scenario under REQ-001, workspace loading fails with a dangling
  scenario reference.

<scenario id="CHK-002">
Given a REPORT.md coverage element with a valid requirement id,
result, and scenario list,
when the report element parser runs,
then it returns a typed coverage row.

Given a coverage element with an invalid result value,
when parsing runs,
then parsing fails and lists the valid results.

Given a coverage element that references a scenario not nested under
the matching requirement in SPEC.md,
when workspace loading runs,
then it fails with a dangling scenario reference.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Parsers and renderers reuse the XML infrastructure

TASKS.md and REPORT.md use the same XML element parser style
introduced by SPEC-0020.

**Done when:**
- `speccy-core::parse::task_xml` exposes typed `TasksDoc` and
  `Task` models plus `parse` and `render`.
- `speccy-core::parse::report_xml` exposes typed `ReportDoc` and
  `RequirementCoverage` models plus `parse` and `render`.
- Shared XML element parsing utilities live in a common module used
  by SPEC, TASKS, and REPORT parsing.
- Parse errors include path and byte offset for malformed elements,
  unknown attributes, duplicate ids, invalid states, invalid coverage
  results, and missing required element blocks.
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
- Given a Speccy XML element inside a fenced code block, it is
  treated as Markdown body content, not structure.

<scenario id="CHK-003">
Given canonical TASKS.md and REPORT.md fixtures,
when parse, render, and parse run in sequence,
then the typed models are structurally equivalent.

Given a speccy XML element inside a fenced code block,
when parsing runs,
then the element is treated as Markdown body content rather than
structure.

Given post-spec public APIs,
when inspected,
then next, status, implement, review, report, and verify read from
the XML-backed typed models without changing their external CLI
behavior.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: Migration rewrites in-tree TASKS.md and REPORT.md

An ephemeral migration tool converts existing files to the XML
element shape.

**Done when:**
- `xtask/migrate-task-report-xml-0021` exists during implementation
  and is deleted before the final commit.
- TASKS.md migration:
  - frontmatter and level-1 heading are preserved;
  - each checkbox task becomes a `<task>` element block;
  - checkbox state maps to the `state` attribute;
  - `Covers:` lines become the `covers` attribute;
  - existing `Tests to write:` or equivalent task-local behavior
    prose becomes the `<task-scenarios>` block;
  - suggested files, implementer notes, retry notes, and review notes
    remain Markdown body content, not typed schema.
- REPORT.md migration:
  - frontmatter and level-1 heading are preserved;
  - each requirements-coverage table row becomes a
    `<coverage>` element block;
  - existing outcome, task summary, skill updates, out-of-scope, and
    deferred-limitations sections remain Markdown.
- Migration fails when it cannot determine task coverage, task state,
  or report coverage without guessing.
- After migration, `speccy verify` exits 0.

**Behavior:**
- Given an existing task `- [?] **T-003**` covering `REQ-002`, the
  migrated task element has `id="T-003"`, `state="in-review"`, and
  `covers="REQ-002"`.
- Given a task with no test/validation prose, migration fails and
  names the task rather than inventing a scenario.
- Given an existing report row for `REQ-001`, migration creates one
  coverage element with the mapped result and scenario ids.

<scenario id="CHK-004">
Given the pre-migration TASKS.md files,
when the migration tool runs,
then checkbox tasks become task element blocks with mapped state,
covers, and task-scenarios content.

Given a task with no validation prose,
when migration runs,
then migration fails and names the task instead of inventing a
scenario.

Given the pre-migration REPORT.md files,
when migration runs,
then requirements coverage table rows become coverage elements.
</scenario>
</requirement>

<requirement id="REQ-005">
### REQ-005: Docs and shipped skills use XML-structured tasks and reports

The skill layer must stop teaching the old checkbox/bullet grammar as
the machine contract.

**Done when:**
- `.speccy/ARCHITECTURE.md` documents the TASKS.md and REPORT.md XML
  element grammars.
- The four task states are documented as XML attribute values, with
  old checkbox markers listed only as migration history.
- `resources/modules/prompts/tasks-generate.md` emits XML-structured
  TASKS.md.
- `resources/modules/prompts/report.md` emits XML-structured
  REPORT.md.
- Implementer and reviewer prompts read task state and task scenarios
  from XML elements.
- Active shipped guidance does not require structured handoff fields.
  It may still ask implementers and reviewers to leave clear Markdown
  notes.

**Behavior:**
- Given a freshly generated TASKS.md after this spec lands, task ids,
  states, covers, and task scenarios are all represented by Speccy
  XML elements.
- Given a freshly generated REPORT.md, requirement coverage is
  represented by coverage elements.
- Given a grep for old active guidance that treats checkbox bullets
  as the machine contract, there are no hits except historical
  migration notes.

<scenario id="CHK-005">
Given post-spec ARCHITECTURE.md and shipped skill prompts,
when active guidance is read,
then TASKS.md and REPORT.md are documented as XML-structured Markdown.

Given active guidance,
when searched,
then it does not require first-class implementer handoff fields or
review-note schemas.

Given newly generated TASKS.md and REPORT.md files,
when inspected,
then they use speccy XML elements for task state, task scenarios,
and report coverage.
</scenario>
</requirement>

## Design

### Approach

Implementation order:

1. Extract shared XML element parsing utilities from SPEC-0020 if
   needed.
2. Add TASKS.md and REPORT.md XML element parsers/renderers with
   fixtures.
3. Write and test the migration tool against the most complex in-tree
   TASKS.md and REPORT.md.
4. Switch workspace loading, `next`, `status`, implement/review/report
   prompt rendering, and verify to the typed models.
5. Delete heuristic body-scanner logic that parsed checkboxes and
   coverage tables as machine contract.
6. Sweep docs and shipped skills.
7. Delete the migration tool.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: XML attribute state is canonical

**Status:** Accepted

**Context:** Keeping both checkbox state and XML attribute state would
create another drift vector.

**Decision:** The XML attribute is the canonical task state after
this spec lands. The renderer may include human-readable state text
in the Markdown body, but the parser ignores it.

**Consequences:** State changes are single-attribute edits, and
`speccy next` no longer depends on checkbox parsing.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Task-local scenarios are required

**Status:** Accepted

**Context:** SPEC-level scenarios describe user-facing behavior.
Tasks also need slice-level validation instructions that implementers
and reviewers can act on.

**Decision:** Every task requires one non-empty
`<task-scenarios>` block.

**Consequences:** Task decomposition becomes more useful to workers
and validators, and task quality is reviewed in prose rather than
through executable commands.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Handoffs stay out of scope

**Status:** Accepted

**Context:** Structured handoffs are useful, but adding them here
would mix core artifact hardening with a new workflow artifact.

**Decision:** This spec does not parse or require handoff fields.

**Consequences:** The substrate hardening can land first. A later
spec can add first-class handoffs without changing task state and
coverage elements again.
</decision>

## Migration / Rollback

Migration is structural and fails closed when the current Markdown
does not contain enough information. The migration tool should not
invent task scenarios or report coverage.

Rollback is `git revert` of the implementation commit set. The old
checkbox and report-table conventions remain in history.

## Open Questions

- [ ] Should `TASKS.md` keep a visible checkbox glyph generated from
      XML state for human scanning? Lean no; use visible state text
      only if dogfooding shows task lists are too hard to scan.
- [ ] Should REPORT.md require coverage for dropped requirements?
      Lean no; dropped requirements should be visible in SPEC.md
      status or changelog rather than report coverage.

## Assumptions

- Existing TASKS.md files have enough `Covers:` and `Tests to write:`
  prose to migrate without inventing validation scenarios.
- Existing REPORT.md files have enough requirements coverage table
  data to migrate coverage elements.
- SPEC-0020's XML element parser has already established fenced-code
  handling and deterministic element rendering.
- The element names introduced here (`tasks`, `task`, `task-scenarios`,
  `report`, `coverage`) are disjoint from the HTML5 element name set,
  satisfying the SPEC-0020 DEC-002 invariant. The same unit test that
  enforces SPEC-0020's whitelist disjointness also covers these.

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial rewritten draft. Applies marker comments to TASKS.md and REPORT.md while deferring first-class handoffs. |
| 2026-05-15 | human/kevin | Renumbered from SPEC-0020 to SPEC-0021. Rewrote carrier from HTML-comment markers to raw XML element tags to align with the new SPEC-0020. |
| 2026-05-15 | human/kevin | Dropped the `speccy-` prefix on element names to match SPEC-0020 DEC-002; tag names are now bare semantic words (`task`, `coverage`, `tasks`, `report`, `task-scenarios`). |
| 2026-05-15 | human/kevin | Recorded HTML5-disjointness invariant in Assumptions; the SPEC-0021 element set is already disjoint and inherits the SPEC-0020 unit test. |
</changelog>

## Notes

This spec is intentionally narrower than the previous draft. It
hardens shared state and validation prose, which harnesses need
immediately, while leaving handoff schemas to a later design once the
core carrier is proven through dogfooding.
