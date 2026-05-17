---
spec: SPEC-0003
spec_hash_at_generation: 5f8898732459e7147370059c583c88d5c9877f6f1a921ce3b4397f68383f7f64
generated_at: 2026-05-14T03:25:12Z
---

# Tasks: SPEC-0003 lint-engine

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Core types and orchestrator

<tasks spec="SPEC-0003">

<task id="T-001" state="completed" covers="REQ-006">
Define `Diagnostic`, `Level`, `Workspace`, `ParsedSpec` and the `lint::run` skeleton

- Suggested files: `speccy-core/src/lint/mod.rs`, `speccy-core/src/lint/types.rs`, `speccy-core/tests/lint_run.rs`

<task-scenarios>
  - `Level` has exactly three variants: `Error`, `Warn`, `Info`.
  - `Diagnostic.code` is `&'static str`.
  - `lint::run(&Workspace { specs: vec![], supersession: &empty })` returns an empty vec without panics.
  - Calling `lint::run` twice on the same input returns byte-equal vecs (determinism).
  - Output ordering is `(spec_id, code, file, line)` ascending, with `None` sorting before `Some`.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-007">
Set up `REGISTRY` and the stability snapshot test

- Suggested files: `speccy-core/src/lint/registry.rs`, `speccy-core/tests/lint_registry.rs`, `speccy-core/tests/snapshots/lint_registry.snap`


<task-scenarios>
  - `REGISTRY: &[(&'static str, Level)]` enumerates every code the engine emits with its severity.
  - The snapshot test compares `REGISTRY` against `tests/snapshots/lint_registry.snap` and fails if the contents differ.
  - Removing a code from `REGISTRY` produces a snapshot diff (test fails).
  - Adding a new code without snapshot regen produces a snapshot diff.
  - Changing a severity produces a snapshot diff.
</task-scenarios>
</task>

## Phase 2: SPC family


<task id="T-003" state="completed" covers="REQ-001">
Implement SPC-001..SPC-005 (structural / frontmatter codes)

- Suggested files: `speccy-core/src/lint/rules/spc.rs`, `speccy-core/tests/lint_spc.rs`, `speccy-core/tests/fixtures/lint/spc/`

<task-scenarios>
  - SPC-001 fires when the spec.toml parser reports a missing required field; the diagnostic file is the offending spec.toml path.
  - SPC-002 fires when a SPEC.md REQ heading has no matching `[[requirements]]` row in spec.toml.
  - SPC-003 fires when a `[[requirements]]` row has no matching SPEC.md REQ heading.
  - SPC-004 fires when SPEC.md frontmatter is missing any of `id`, `slug`, `title`, `status`, `created`.
  - SPC-005 fires when `status` value is outside `{in-progress, implemented, dropped, superseded}`; the diagnostic message names the offending value.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-001">
Implement SPC-006 (supersession graph consistency)

- Suggested files: `speccy-core/src/lint/rules/spc.rs` (extend), `speccy-core/tests/lint_spc.rs` (extend)

<task-scenarios>
  - `status: superseded` + no incoming `supersedes` edge in the workspace -> SPC-006 fires.
  - `status: superseded` + at least one incoming edge -> SPC-006 does NOT fire.
  - `status: implemented` + incoming edge present -> SPC-006 does NOT fire (an incoming edge alone never flips status).
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-001">
Implement SPC-007 (informational status / task mismatch)

- Suggested files: `speccy-core/src/lint/rules/spc.rs` (extend), `speccy-core/tests/lint_spc.rs` (extend)


<task-scenarios>
  - `status: implemented` + all tasks `[x]` -> no SPC-007.
  - `status: implemented` + at least one `[ ]` / `[~]` / `[?]` task -> SPC-007 fires at `Level::Info`.
  - `status: in-progress` + any task state -> no SPC-007.
</task-scenarios>
</task>

## Phase 3: REQ family


<task id="T-006" state="completed" covers="REQ-002">
Implement REQ-001 and REQ-002 (coverage graph)

- Suggested files: `speccy-core/src/lint/rules/req.rs`, `speccy-core/tests/lint_req.rs`


<task-scenarios>
  - `[[requirements]] id = "REQ-001" checks = []` -> REQ-001 lint code fires naming the requirement.
  - `[[requirements]] id = "REQ-001" checks = ["CHK-999"]` with no `[[checks]] id = "CHK-999"` -> REQ-002 fires naming both the requirement and the missing check.
  - Multiple requirements each missing coverage -> one REQ-001 per requirement, ordered by REQ ID.
</task-scenarios>
</task>

## Phase 4: VAL family


<task id="T-007" state="completed" covers="REQ-003">
Implement VAL-001, VAL-002, VAL-003 (check definition completeness)

- Suggested files: `speccy-core/src/lint/rules/val.rs`, `speccy-core/tests/lint_val.rs`

<task-scenarios>
  - VAL-001 fires for a check missing `proves`.
  - VAL-002 fires for a check with `kind = "test"` or `kind = "command"` missing `command`.
  - VAL-003 fires for a check with `kind = "manual"` missing `prompt`.
  - A free-form `kind` value (e.g. `kind = "property"`) without `command` does NOT trigger VAL-002; the parser only flags missing required fields for the known executable kinds.
</task-scenarios>
</task>

<task id="T-008" state="completed" covers="REQ-003">
Implement VAL-004 (no-op command detection)

- Suggested files: `speccy-core/src/lint/rules/val.rs` (extend), `speccy-core/tests/lint_val.rs` (extend)


<task-scenarios>
  - Each pattern in the closed set fires: `true`, `:`, `exit 0`, `/bin/true`, `cmd /c exit 0`, `exit /b 0`.
  - Leading/trailing whitespace tolerated: `"  true  "` fires.
  - Compound commands do NOT fire: `"true && cargo test"`, `": ; do-real-work"`, `"exit 0 || retry"`.
  - Severity is `Level::Warn`.
  - The diagnostic message names the offending command verbatim.
</task-scenarios>
</task>

## Phase 5: TSK family


<task id="T-009" state="completed" covers="REQ-004">
Implement TSK-001, TSK-002, TSK-004 (task structure)

- Suggested files: `speccy-core/src/lint/rules/tsk.rs`, `speccy-core/tests/lint_tsk.rs`

<task-scenarios>
  - TSK-001 fires when a task `Covers: REQ-099` and REQ-099 is in neither SPEC.md nor spec.toml; the message names both the task ID and the offending REQ.
  - TSK-002 fires when the parser surfaced a malformed task ID warning (e.g. `**TASK-001**` instead of `**T-001**`); the message names the offending bold-span text.
  - TSK-004 fires when TASKS.md frontmatter is missing `spec`, `spec_hash_at_generation`, or `generated_at`; one diagnostic per missing field.
</task-scenarios>
</task>

<task id="T-010" state="completed" covers="REQ-004">
Implement TSK-003 (staleness: hash and mtime drift, plus bootstrap-pending variant)

- Suggested files: `speccy-core/src/lint/rules/tsk.rs` (extend), `speccy-core/tests/lint_tsk.rs` (extend)


<task-scenarios>
  - Hash match + TASKS.md mtime >= SPEC.md mtime -> no TSK-003.
  - Hash mismatch -> TSK-003 at `Level::Warn`; message names both stored and current hashes.
  - SPEC.md mtime > TASKS.md mtime (even with hash match) -> TSK-003 at `Level::Warn`.
  - `spec_hash_at_generation: bootstrap-pending` -> TSK-003 at `Level::Info` with a message naming `speccy tasks SPEC-NNNN --commit` as the remediation.
</task-scenarios>
</task>

## Phase 6: QST family


<task id="T-011" state="completed" covers="REQ-005">
Implement QST-001 (open question soft signal)

- Suggested files: `speccy-core/src/lint/rules/qst.rs`, `speccy-core/tests/lint_qst.rs`


<task-scenarios>
  - Three unchecked `- [ ] question?` lines in `## Open questions` -> three QST-001 diagnostics at `Level::Info`.
  - All checked `- [x] ...` -> no QST-001.
  - Mixed: only unchecked produce QST-001.
  - Question text (after the checkbox glyph) appears in the diagnostic message verbatim.
  - Open questions section is case-insensitive (`## Open Questions` and `## open questions` both work).
</task-scenarios>
</task>

## Phase 7: Fixture corpus and integration


<task id="T-012" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005">
Build the fixture corpus and a loader helper

- Suggested files: `speccy-core/tests/fixtures/lint/spc-001/SPEC.md`, `...`, `speccy-core/tests/lint_fixtures.rs`

<task-scenarios>
  - A `speccy-core/tests/fixtures/lint/<code>/` directory exists per code (or per family), each containing SPEC.md + spec.toml + optional TASKS.md.
  - Each fixture has a header comment naming which codes it should trigger and which it should NOT trigger (defensive against rule overreach).
  - A loader helper reads a fixture via the SPEC-0001 parser and produces a `ParsedSpec` ready for `lint::run`.
  - A meta-test iterates every fixture, runs lint, and asserts the emitted codes match the fixture's header.
</task-scenarios>
</task>

</tasks>
