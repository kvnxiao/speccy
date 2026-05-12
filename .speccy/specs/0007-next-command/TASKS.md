---
spec: SPEC-0007
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0007 next-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0007 --commit` runs against this
> spec.

## Phase 1: Task discovery and priority logic (speccy-core)

- [ ] **T-001**: Implement task enumeration from `workspace::scan`
  - Covers: REQ-001
  - Tests to write:
    - Enumeration walks specs in ascending ID order.
    - Per-spec tasks are exposed with `(state, id, line, covers, suggested_files, notes)`.
    - `InProgress` (`[~]`) tasks are visible in the enumeration but should be skipped by priority logic (no caller can claim a claimed task).
    - Empty `.speccy/specs/` yields an empty enumeration; the function returns without error.
  - Suggested files: `speccy-core/src/next.rs`, `speccy-core/tests/next_enumeration.rs`

- [ ] **T-002**: Implement default-priority logic (no `--kind`)
  - Covers: REQ-001
  - Tests to write:
    - Within a spec, an `AwaitingReview` (`[?]`) task is returned before an `Open` (`[ ]`) task in the same spec.
    - Across specs, the lowest spec ID with actionable work wins -- e.g. SPEC-0001 `[ ]` is returned even when SPEC-0002 has `[?]`.
    - `InProgress` (`[~]`) tasks are skipped (treated as "no actionable work for this caller").
    - When no spec has actionable work, the result falls through to report-kind detection (T-005).
  - Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_priority.rs`

## Phase 2: `--kind` filters

- [ ] **T-003**: Implement `KindFilter::Implement` strict filter
  - Covers: REQ-002
  - Tests to write:
    - Only `Open` (`[ ]`) tasks are returned as `NextResult::Implement`.
    - Workspace with `[?]` but no `[ ]` -> `NextResult::Blocked` with the canonical "no open tasks; reviews pending" reason.
    - Workspace with `[~]` only -> `Blocked` with a different canonical reason ("all open tasks claimed by other sessions").
    - Walking order is identical to the default (ascending spec ID).
  - Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_priority.rs` (extend)

- [ ] **T-004**: Implement `KindFilter::Review` strict filter with persona fan-out
  - Covers: REQ-002
  - Tests to write:
    - Only `AwaitingReview` (`[?]`) tasks are returned as `NextResult::Review`.
    - The persona fan-out is the hardcoded `DEFAULT_PERSONAS = ["business", "tests", "security", "style"]`.
    - Workspace with `[ ]` but no `[?]` -> `Blocked` with a "no reviews pending" reason.
  - Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_priority.rs` (extend)

## Phase 3: Report and blocked kinds

- [ ] **T-005**: Implement `kind: report` detection
  - Covers: REQ-003
  - Tests to write:
    - All tasks across all specs are `[x]` AND at least one spec lacks REPORT.md -> `NextResult::Report { spec: <lowest-ID> }`.
    - Some spec has `[x]` and REPORT.md present; another has `[x]` and no REPORT.md -> returns the latter.
    - All specs done AND all REPORT.md present -> falls through to blocked.
  - Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_report.rs`

- [ ] **T-006**: Implement `kind: blocked` with canonical reasons
  - Covers: REQ-003
  - Tests to write:
    - Empty workspace -> `Blocked { reason: "no specs in workspace" }`.
    - All `[~]` (claimed) tasks across all specs -> `Blocked { reason: "all open tasks are claimed by other sessions" }`.
    - `--kind implement` with no `[ ]` tasks -> `Blocked { reason: "no open tasks; reviews pending" }` (or similar canonical phrase).
    - `--kind review` with no `[?]` tasks -> `Blocked { reason: "no reviews pending" }`.
  - Suggested files: `speccy-core/src/next.rs` (extend), `speccy-core/tests/next_blocked.rs`

## Phase 4: Output rendering

- [ ] **T-007**: Implement text-mode renderer
  - Covers: REQ-005
  - Tests to write:
    - `implement` kind: `next: implement T-NNN (SPEC-NNNN) -- <task_line>\n`.
    - `review` kind: `next: review T-NNN (SPEC-NNNN) -- personas: business, tests, security, style\n`.
    - `report` kind: `next: report SPEC-NNNN -- all tasks complete\n`.
    - `blocked` kind: `next: blocked -- <reason>\n`.
    - Exit code is 0 for all four kinds.
  - Suggested files: `speccy-cli/src/next_output.rs`, `speccy-cli/tests/next_text.rs`

- [ ] **T-008**: Implement `--json` renderer
  - Covers: REQ-004
  - Tests to write:
    - Output begins with `"schema_version": 1`.
    - The `"kind"` field discriminates between `"implement"`, `"review"`, `"report"`, `"blocked"`.
    - `implement` variant has `prompt_command: "speccy implement T-NNN"`.
    - `review` variant has `prompt_command_template: "speccy review T-NNN --persona {persona}"`.
    - `report` variant has `prompt_command: "speccy report SPEC-NNNN"`.
    - `blocked` variant has `reason` matching the canonical phrase set.
    - Pretty-printed.
    - Two runs with identical workspace state produce byte-identical stdout.
  - Suggested files: `speccy-cli/src/next_output.rs` (extend), `speccy-cli/tests/next_json.rs`

## Phase 5: CLI wiring

- [ ] **T-009**: Wire `speccy next [--kind] [--json]` into the binary
  - Covers: REQ-001..REQ-005
  - Tests to write:
    - End-to-end via `assert_cmd` against fixture workspaces covering all four kinds.
    - From outside a speccy workspace -> exit 1 with `NextError::ProjectRootNotFound`.
    - Text and JSON modes each tested independently per fixture.
  - Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/next.rs`, `speccy-cli/tests/integration_next.rs`
