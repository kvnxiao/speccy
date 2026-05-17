---
spec: SPEC-0007
spec_hash_at_generation: e5eaedd97730e1f7bfa257f80f85172800d67b310c5f07e66debafa00f25f1ff
generated_at: 2026-05-14T03:25:14Z
---

# Tasks: SPEC-0007 next-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0007 --commit` runs against this
> spec.

> Implementer note (retroactive, 2026-05-13): Tasks T-001..T-009
> landed in commit `ffad1ec`. Checkboxes back-filled during the v1
> dogfood status sweep; no per-task review notes were captured at
> implementation time.

## Phase 1: Task discovery and priority logic (speccy-core)

<tasks spec="SPEC-0007">

<task id="T-001" state="completed" covers="REQ-001">
Implement task enumeration from `workspace::scan`

- Suggested files: `speccy-core/src/next.rs`, `speccy-core/tests/next_enumeration.rs`

<task-scenarios>
  - Enumeration walks specs in ascending ID order.
  - Per-spec tasks are exposed with `(state, id, line, covers, suggested_files, notes)`.
  - `InProgress` (`[~]`) tasks are visible in the enumeration but should be skipped by priority logic (no caller can claim a claimed task).
  - Empty `.speccy/specs/` yields an empty enumeration; the function returns without error.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001">
Implement default-priority logic (no `--kind`)

- Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_priority.rs`


<task-scenarios>
  - Within a spec, an `AwaitingReview` (`[?]`) task is returned before an `Open` (`[ ]`) task in the same spec.
  - Across specs, the lowest spec ID with actionable work wins -- e.g. SPEC-0001 `[ ]` is returned even when SPEC-0002 has `[?]`.
  - `InProgress` (`[~]`) tasks are skipped (treated as "no actionable work for this caller").
  - When no spec has actionable work, the result falls through to report-kind detection (T-005).
</task-scenarios>
</task>

## Phase 2: `--kind` filters


<task id="T-003" state="completed" covers="REQ-002">
Implement `KindFilter::Implement` strict filter

- Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_priority.rs` (extend)

<task-scenarios>
  - Only `Open` (`[ ]`) tasks are returned as `NextResult::Implement`.
  - Workspace with `[?]` but no `[ ]` -> `NextResult::Blocked` with the canonical "no open tasks; reviews pending" reason.
  - Workspace with `[~]` only -> `Blocked` with a different canonical reason ("all open tasks claimed by other sessions").
  - Walking order is identical to the default (ascending spec ID).
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-002">
Implement `KindFilter::Review` strict filter with persona fan-out

- Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_priority.rs` (extend)


<task-scenarios>
  - Only `AwaitingReview` (`[?]`) tasks are returned as `NextResult::Review`.
  - The persona fan-out is the hardcoded `DEFAULT_PERSONAS = ["business", "tests", "security", "style"]`.
  - Workspace with `[ ]` but no `[?]` -> `Blocked` with a "no reviews pending" reason.
</task-scenarios>
</task>

## Phase 3: Report and blocked kinds


<task id="T-005" state="completed" covers="REQ-003">
Implement `kind: report` detection

- Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_report.rs`

<task-scenarios>
  - All tasks across all specs are `[x]` AND at least one spec lacks REPORT.md -> `NextResult::Report { spec: <lowest-ID> }`.
  - Some spec has `[x]` and REPORT.md present; another has `[x]` and no REPORT.md -> returns the latter.
  - All specs done AND all REPORT.md present -> falls through to blocked.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-003">
Implement `kind: blocked` with canonical reasons

- Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_blocked.rs`


<task-scenarios>
  - Empty workspace -> `Blocked { reason: "no specs in workspace" }`.
  - All `[~]` (claimed) tasks across all specs -> `Blocked { reason: "all open tasks are claimed by other sessions" }`.
  - `--kind implement` with no `[ ]` tasks -> `Blocked { reason: "no open tasks; reviews pending" }` (or similar canonical phrase).
  - `--kind review` with no `[?]` tasks -> `Blocked { reason: "no reviews pending" }`.
</task-scenarios>
</task>

## Phase 4: Output rendering


<task id="T-007" state="completed" covers="REQ-005">
Implement text-mode renderer

- Suggested files: `speccy-cli/src/next_output.rs`, `speccy-cli/tests/next_text.rs`

<task-scenarios>
  - `implement` kind: `next: implement T-NNN (SPEC-NNNN) -- <task_line>\n`.
  - `review` kind: `next: review T-NNN (SPEC-NNNN) -- personas: business, tests, security, style\n`.
  - `report` kind: `next: report SPEC-NNNN -- all tasks complete\n`.
  - `blocked` kind: `next: blocked -- <reason>\n`.
  - Exit code is 0 for all four kinds.
</task-scenarios>
</task>

<task id="T-008" state="completed" covers="REQ-004">
Implement `--json` renderer

- Suggested files: `speccy-cli/src/next_output.rs` (extend), `speccy-cli/tests/next_json.rs`


<task-scenarios>
  - Output begins with `"schema_version": 1`.
  - The `"kind"` field discriminates between `"implement"`, `"review"`, `"report"`, `"blocked"`.
  - `implement` variant has `prompt_command: "speccy implement T-NNN"`.
  - `review` variant has `prompt_command_template: "speccy review T-NNN --persona {persona}"`.
  - `report` variant has `prompt_command: "speccy report SPEC-NNNN"`.
  - `blocked` variant has `reason` matching the canonical phrase set.
  - Pretty-printed.
  - Two runs with identical workspace state produce byte-identical stdout.
</task-scenarios>
</task>

## Phase 5: CLI wiring


<task id="T-009" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005">
Wire `speccy next [--kind] [--json]` into the binary

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/next.rs`, `speccy-cli/tests/integration_next.rs`

<task-scenarios>
  - End-to-end via `assert_cmd` against fixture workspaces covering all four kinds.
  - From outside a speccy workspace -> exit 1 with `NextError::ProjectRootNotFound`.
  - Text and JSON modes each tested independently per fixture.
</task-scenarios>
</task>

</tasks>
