---
spec: SPEC-0004
spec_hash_at_generation: a3dc6e727ade88a15042e425a38c078e5ba55f2a5d5a059de0f652629942d606
generated_at: 2026-05-14T03:25:13Z
---

# Tasks: SPEC-0004 status-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Workspace scanner (speccy-core)

<tasks spec="SPEC-0004">

<task id="T-001" state="completed" covers="REQ-001">
Implement `workspace::find_root`

- Suggested files: `speccy-core/src/workspace.rs`, `speccy-core/tests/workspace_find_root.rs`

<task-scenarios>
  - From cwd, walk up parent directories until `.speccy/` is found; return that path.
  - From inside a deeply-nested subdirectory, walk-up still finds the root.
  - From outside any speccy workspace (walking up reaches filesystem root without finding `.speccy/`), return `WorkspaceError::NoSpeccyDir`.
  - I/O errors during traversal return `WorkspaceError::Io(_)`.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001">
Implement `workspace::scan`

- Suggested files: `speccy-core/src/workspace.rs` (extend), `speccy-core/tests/workspace_scan.rs`


<task-scenarios>
  - Discovers every `.speccy/specs/NNNN-slug/` directory matching the regex `^\d{4}-[a-z0-9-]+$`.
  - Non-matching subdirectories (`_scratch`, `notes`) are ignored without warnings.
  - `ScannedSpec.spec_md` / `.spec_toml` / `.tasks_md` carry `Result<_, ParseError>` (with `Option<>` for tasks_md since it's optional); one bad spec doesn't abort the scan.
  - Empty `.speccy/specs/` yields `Workspace { specs: vec![], ... }`.
  - Missing `.speccy/specs/` directory yields the same empty result without error.
  - Specs are returned in ascending spec-ID order regardless of filesystem iteration order.
</task-scenarios>
</task>

## Phase 2: Staleness detector


<task id="T-003" state="completed" covers="REQ-002">
Implement `workspace::stale_for`

- Suggested files: `speccy-core/src/workspace.rs` (extend), `speccy-core/tests/stale_detection.rs`


<task-scenarios>
  - Hash match + TASKS.md mtime >= SPEC.md mtime -> `Staleness { stale: false, reasons: [] }`.
  - Hash mismatch -> `HashDrift` in reasons.
  - SPEC.md mtime > TASKS.md mtime -> `MtimeDrift` in reasons.
  - `spec_hash_at_generation = "bootstrap-pending"` -> `BootstrapPending` is the SOLE reason (short-circuits other checks).
  - No TASKS.md -> `Staleness { stale: false, reasons: [] }`.
  - Both hash mismatch AND mtime drift -> both reasons present in declared order (`HashDrift, MtimeDrift`).
</task-scenarios>
</task>

## Phase 3: Task counts and open questions


<task id="T-004" state="completed" covers="REQ-003">
Implement task state aggregation

- Suggested files: `speccy-core/src/workspace.rs` (extend or split), `speccy-core/tests/task_state_aggregation.rs`

<task-scenarios>
  - Counts match the glyph distribution (`[ ]` / `[~]` / `[?]` / `[x]`).
  - Missing TASKS.md yields all-zero counts.
  - Tasks under different Phase headings still count.
  - Tasks with malformed IDs (per SPEC-0001 REQ-004 recoverable warnings) are skipped from counts.
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-006 REQ-007">
Implement open-questions counter

- Suggested files: `speccy-core/src/workspace.rs` (extend) or `speccy-core/src/parse/spec_md.rs` extension


<task-scenarios>
  - Count of unchecked `- [ ]` items in `## Open questions` matches.
  - Checked `- [x]` items don't count.
  - Missing `## Open questions` section yields zero.
  - Case-insensitive heading match (`## OPEN QUESTIONS` also works).
</task-scenarios>
</task>

## Phase 4: Supersession + lint integration


<task id="T-006" state="completed" covers="REQ-004">
Wire supersession into the workspace result

- Suggested files: `speccy-core/src/workspace.rs` (extend), `speccy-core/tests/workspace_supersession.rs`

<task-scenarios>
  - Build `&[&SpecMd]` from successfully-parsed specs; call `supersession_index`; expose `superseded_by` per spec.
  - Specs with parse errors have empty `superseded_by`.
  - `dangling_references()` is exposed on `Workspace` so lint can consume it.
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-005">
Build `lint::Workspace` and call `lint::run`; partition by spec_id

- Suggested files: `speccy-cli/src/status.rs`, `speccy-cli/tests/status_lint_integration.rs`


<task-scenarios>
  - Diagnostics with `spec_id = Some("SPEC-NNNN")` route to that spec's lint block.
  - Diagnostics with `spec_id = None` route to the workspace-level lint block.
  - Each lint block has three arrays keyed by `Level` (errors / warnings / info).
  - Empty workspace produces an empty workspace-level lint block.
  - The ordering within each array is `(code, file, line)` ascending (inherited from lint::run).
</task-scenarios>
</task>

## Phase 5: Text renderer


<task id="T-008" state="completed" covers="REQ-006">
Implement default text view with the in-progress + broken filter

- Suggested files: `speccy-cli/src/status_output.rs`, `speccy-cli/tests/status_text_render.rs`, `speccy-cli/tests/status_text_filter.rs`


<task-scenarios>
  - `status: in-progress` specs are shown unconditionally.
  - `status: implemented/dropped/superseded` specs WITHOUT errors/staleness/parse-errors are hidden.
  - Same specs WITH any of those signals ARE shown regardless of status.
  - Per-spec output: header line (`SPEC-NNNN <status>: <title>`), tasks counts, lint errors/warnings count, staleness flag with reasons, open-questions count.
  - Empty workspace -> `No specs in workspace.` + exit code 0.
</task-scenarios>
</task>

## Phase 6: JSON output


<task id="T-009" state="completed" covers="REQ-007">
Implement `--json` envelope and per-spec output struct

- Suggested files: `speccy-cli/src/status_output.rs` (extend), `speccy-cli/tests/status_json.rs`

<task-scenarios>
  - Output starts with `"schema_version": 1`.
  - Every spec appears in the `specs` array (no filtering by status).
  - Each spec entry has all required fields per SPEC.md contract.
  - Lint diagnostics are structured objects, not strings.
  - Pretty-printed (whitespace-tolerant).
  - Two runs with no filesystem change produce byte-identical output.
  - `stale_reasons` are ordered `HashDrift, MtimeDrift, BootstrapPending` (declared order).
</task-scenarios>
</task>

<task id="T-010" state="completed" covers="REQ-007">
Wire `repo_sha` via shell-out to `git rev-parse HEAD`

- Suggested files: `speccy-cli/src/git.rs`, `speccy-cli/tests/git_repo_sha.rs`


<task-scenarios>
  - Inside a git repo with HEAD: `repo_sha` is the 40-character SHA.
  - Outside a git repo (no `.git/`): `repo_sha` is `""`, no error.
  - Git not on PATH: `repo_sha` is `""`, no error.
  - HEAD unset (fresh repo): `repo_sha` is `""`, no error.
</task-scenarios>
</task>

## Phase 7: CLI wiring


<task id="T-011" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005 REQ-006 REQ-007">
Wire `speccy status [--json]` into the binary

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/status.rs`, `speccy-cli/tests/integration_status.rs`

<task-scenarios>
  - `speccy status` runs from any cwd inside a speccy workspace.
  - `speccy status` from outside a workspace -> exit code 1 with a clear `WorkspaceError::NoSpeccyDir` message.
  - `speccy status --json` emits valid JSON (`serde_json::from_str` round-trip).
  - End-to-end integration test via `assert_cmd` in a tmpdir with fixture specs.
</task-scenarios>
</task>

</tasks>
