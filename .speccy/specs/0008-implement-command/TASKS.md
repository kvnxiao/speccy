---
spec: SPEC-0008
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0008 implement-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0008 --commit` runs.

## Phase 1: Task reference parsing

- [ ] **T-001**: Implement `task_lookup::parse_ref`
  - Covers: REQ-001
  - Tests to write:
    - `"T-001"` parses to `TaskRef::Unqualified { id: "T-001" }`.
    - `"SPEC-0001/T-001"` parses to `TaskRef::Qualified { spec_id: "SPEC-0001", task_id: "T-001" }`.
    - `"T-1234"` (4-digit) and `"T-001"` (3-digit) both parse; minimum 3 digits.
    - `"FOO"`, `"T-"`, `"T-AB"`, `"SPEC-0001/FOO"`, `"/T-001"` all return `LookupError::InvalidFormat`.
  - Suggested files: `crates/speccy-core/src/task_lookup.rs`, `crates/speccy-core/tests/task_lookup.rs`

## Phase 2: Workspace task lookup

- [ ] **T-002**: Implement `task_lookup::find` with unique-match semantics
  - Covers: REQ-002
  - Tests to write:
    - Single spec with one matching `T-NNN` -> `Ok(TaskLocation)` with the spec_id populated.
    - Multiple specs with disjoint task IDs -> unqualified lookup finds the right one.
    - Qualified `SPEC-NNNN/T-NNN` -> scopes to that spec; ignores other specs.
    - No spec has the task -> `LookupError::NotFound { task_ref: "<arg>" }`.
    - Spec with `tasks_md` parse error is skipped (no panic; lookup proceeds to other specs).
  - Suggested files: `crates/speccy-core/src/task_lookup.rs` (extend), `crates/speccy-core/tests/task_lookup.rs` (extend)

- [ ] **T-003**: Implement ambiguity detection
  - Covers: REQ-003
  - Tests to write:
    - T-001 in SPEC-0001 and SPEC-0002 -> `LookupError::Ambiguous` with `candidate_specs = ["SPEC-0001", "SPEC-0002"]` (ascending order).
    - T-001 in three specs -> all three appear in `candidate_specs`.
    - Qualified `SPEC-0001/T-001` against the same workspace bypasses ambiguity (returns `Ok` for SPEC-0001's task).
  - Suggested files: `crates/speccy-core/src/task_lookup.rs` (extend), `crates/speccy-core/tests/task_lookup.rs` (extend)

## Phase 3: Prompt assembly

- [ ] **T-004**: Render implementer prompt with full task entry
  - Covers: REQ-004
  - Tests to write:
    - `implementer.md` template is loaded via `prompt::load_template`.
    - Placeholders substituted: `{{spec_id}}`, `{{spec_md}}` (full content), `{{task_id}}`, `{{task_entry}}` (task line + every sub-list bullet in declared order), `{{suggested_files}}` (CSV or empty string), `{{agents}}`.
    - Budget trimming applied.
    - Output goes to stdout; exit code 0.
    - A task with three review notes from a prior loop has all three included in `{{task_entry}}`.
  - Suggested files: `crates/speccy/src/implement.rs`, `skills/shared/prompts/implementer.md` (stub), `crates/speccy/tests/implement_prompt.rs`

## Phase 4: CLI wiring and error mapping

- [ ] **T-005**: Wire `speccy implement TASK-ID` and map errors
  - Covers: REQ-005
  - Tests to write:
    - End-to-end via `assert_cmd`: valid task ref renders prompt to stdout (exit 0).
    - `InvalidFormat` -> exit 1; stderr names the bad arg and shows accepted formats.
    - `NotFound` -> exit 1; stderr names the task and suggests `speccy status`.
    - `Ambiguous` -> exit 1; stderr lists each candidate as a copy-pasteable command.
    - Outside-workspace -> exit 1; stderr names the issue.
    - Internal `TemplateNotFound` -> exit 2.
  - Suggested files: `crates/speccy/src/main.rs`, `crates/speccy/src/implement.rs`, `crates/speccy/tests/integration_implement.rs`
