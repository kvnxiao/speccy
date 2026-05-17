---
spec: SPEC-0008
spec_hash_at_generation: 705e5d3a48a279968e7507afe74f8cef0d298360b124cb9727d6ecce1505eb06
generated_at: 2026-05-14T03:25:14Z
---

# Tasks: SPEC-0008 implement-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0008 --commit` runs.

> Implementer note (retroactive, 2026-05-13): Tasks T-001..T-005
> landed in commit `2b1ee4c`. Checkboxes back-filled during the v1
> dogfood status sweep; no per-task review notes were captured at
> implementation time.

## Phase 1: Task reference parsing

<tasks spec="SPEC-0008">

<task id="T-001" state="completed" covers="REQ-001">
Implement `task_lookup::parse_ref`

- Suggested files: `speccy-core/src/task_lookup.rs`, `speccy-core/tests/task_lookup.rs`


<task-scenarios>
  - `"T-001"` parses to `TaskRef::Unqualified { id: "T-001" }`.
  - `"SPEC-0001/T-001"` parses to `TaskRef::Qualified { spec_id: "SPEC-0001", task_id: "T-001" }`.
  - `"T-1234"` (4-digit) and `"T-001"` (3-digit) both parse; minimum 3 digits.
  - `"FOO"`, `"T-"`, `"T-AB"`, `"SPEC-0001/FOO"`, `"/T-001"` all return `LookupError::InvalidFormat`.
</task-scenarios>
</task>

## Phase 2: Workspace task lookup


<task id="T-002" state="completed" covers="REQ-002">
Implement `task_lookup::find` with unique-match semantics

- Suggested files: `speccy-core/src/task_lookup.rs` (extend), `speccy-core/tests/task_lookup.rs` (extend)

<task-scenarios>
  - Single spec with one matching `T-NNN` -> `Ok(TaskLocation)` with the spec_id populated.
  - Multiple specs with disjoint task IDs -> unqualified lookup finds the right one.
  - Qualified `SPEC-NNNN/T-NNN` -> scopes to that spec; ignores other specs.
  - No spec has the task -> `LookupError::NotFound { task_ref: "<arg>" }`.
  - Spec with `tasks_md` parse error is skipped (no panic; lookup proceeds to other specs).
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-003">
Implement ambiguity detection

- Suggested files: `speccy-core/src/task_lookup.rs` (extend), `speccy-core/tests/task_lookup.rs` (extend)


<task-scenarios>
  - T-001 in SPEC-0001 and SPEC-0002 -> `LookupError::Ambiguous` with `candidate_specs = ["SPEC-0001", "SPEC-0002"]` (ascending order).
  - T-001 in three specs -> all three appear in `candidate_specs`.
  - Qualified `SPEC-0001/T-001` against the same workspace bypasses ambiguity (returns `Ok` for SPEC-0001's task).
</task-scenarios>
</task>

## Phase 3: Prompt assembly


<task id="T-004" state="completed" covers="REQ-004">
Render implementer prompt with full task entry

- Suggested files: `speccy-cli/src/implement.rs`, `skills/shared/prompts/implementer.md` (stub), `speccy-cli/tests/implement_prompt.rs`


<task-scenarios>
  - `implementer.md` template is loaded via `prompt::load_template`.
  - Placeholders substituted: `{{spec_id}}`, `{{spec_md}}` (full content), `{{task_id}}`, `{{task_entry}}` (task line + every sub-list bullet in declared order), `{{suggested_files}}` (CSV or empty string), `{{agents}}`.
  - Budget trimming applied.
  - Output goes to stdout; exit code 0.
  - A task with three review notes from a prior loop has all three included in `{{task_entry}}`.
</task-scenarios>
</task>

## Phase 4: CLI wiring and error mapping


<task id="T-005" state="completed" covers="REQ-005">
Wire `speccy implement TASK-ID` and map errors

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/implement.rs`, `speccy-cli/tests/integration_implement.rs`

<task-scenarios>
  - End-to-end via `assert_cmd`: valid task ref renders prompt to stdout (exit 0).
  - `InvalidFormat` -> exit 1; stderr names the bad arg and shows accepted formats.
  - `NotFound` -> exit 1; stderr names the task and suggests `speccy status`.
  - `Ambiguous` -> exit 1; stderr lists each candidate as a copy-pasteable command.
  - Outside-workspace -> exit 1; stderr names the issue.
  - Internal `TemplateNotFound` -> exit 2.
</task-scenarios>
</task>

</tasks>
