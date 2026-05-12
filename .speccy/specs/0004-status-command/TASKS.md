---
spec: SPEC-0004
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0004 status-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Workspace scanner (speccy-core)

- [x] **T-001**: Implement `workspace::find_root`
  - Covers: REQ-001
  - Tests to write:
    - From cwd, walk up parent directories until `.speccy/` is found; return that path.
    - From inside a deeply-nested subdirectory, walk-up still finds the root.
    - From outside any speccy workspace (walking up reaches filesystem root without finding `.speccy/`), return `WorkspaceError::NoSpeccyDir`.
    - I/O errors during traversal return `WorkspaceError::Io(_)`.
  - Suggested files: `crates/speccy-core/src/workspace.rs`, `crates/speccy-core/tests/workspace_find_root.rs`

- [x] **T-002**: Implement `workspace::scan`
  - Covers: REQ-001
  - Tests to write:
    - Discovers every `.speccy/specs/NNNN-slug/` directory matching the regex `^\d{4}-[a-z0-9-]+$`.
    - Non-matching subdirectories (`_scratch`, `notes`) are ignored without warnings.
    - `ScannedSpec.spec_md` / `.spec_toml` / `.tasks_md` carry `Result<_, ParseError>` (with `Option<>` for tasks_md since it's optional); one bad spec doesn't abort the scan.
    - Empty `.speccy/specs/` yields `Workspace { specs: vec![], ... }`.
    - Missing `.speccy/specs/` directory yields the same empty result without error.
    - Specs are returned in ascending spec-ID order regardless of filesystem iteration order.
  - Suggested files: `crates/speccy-core/src/workspace.rs` (extend), `crates/speccy-core/tests/workspace_scan.rs`

## Phase 2: Staleness detector

- [x] **T-003**: Implement `workspace::stale_for`
  - Covers: REQ-002
  - Tests to write:
    - Hash match + TASKS.md mtime >= SPEC.md mtime -> `Staleness { stale: false, reasons: [] }`.
    - Hash mismatch -> `HashDrift` in reasons.
    - SPEC.md mtime > TASKS.md mtime -> `MtimeDrift` in reasons.
    - `spec_hash_at_generation = "bootstrap-pending"` -> `BootstrapPending` is the SOLE reason (short-circuits other checks).
    - No TASKS.md -> `Staleness { stale: false, reasons: [] }`.
    - Both hash mismatch AND mtime drift -> both reasons present in declared order (`HashDrift, MtimeDrift`).
  - Suggested files: `crates/speccy-core/src/workspace.rs` (extend), `crates/speccy-core/tests/stale_detection.rs`

## Phase 3: Task counts and open questions

- [x] **T-004**: Implement task state aggregation
  - Covers: REQ-003
  - Tests to write:
    - Counts match the glyph distribution (`[ ]` / `[~]` / `[?]` / `[x]`).
    - Missing TASKS.md yields all-zero counts.
    - Tasks under different Phase headings still count.
    - Tasks with malformed IDs (per SPEC-0001 REQ-004 recoverable warnings) are skipped from counts.
  - Suggested files: `crates/speccy-core/src/workspace.rs` (extend or split), `crates/speccy-core/tests/task_state_aggregation.rs`

- [x] **T-005**: Implement open-questions counter
  - Covers: REQ-006 (text view), REQ-007 (JSON contract)
  - Tests to write:
    - Count of unchecked `- [ ]` items in `## Open questions` matches.
    - Checked `- [x]` items don't count.
    - Missing `## Open questions` section yields zero.
    - Case-insensitive heading match (`## OPEN QUESTIONS` also works).
  - Suggested files: `crates/speccy-core/src/workspace.rs` (extend) or `crates/speccy-core/src/parse/spec_md.rs` extension

## Phase 4: Supersession + lint integration

- [x] **T-006**: Wire supersession into the workspace result
  - Covers: REQ-004
  - Tests to write:
    - Build `&[&SpecMd]` from successfully-parsed specs; call `supersession_index`; expose `superseded_by` per spec.
    - Specs with parse errors have empty `superseded_by`.
    - `dangling_references()` is exposed on `Workspace` so lint can consume it.
  - Suggested files: `crates/speccy-core/src/workspace.rs` (extend), `crates/speccy-core/tests/workspace_supersession.rs`

- [x] **T-007**: Build `lint::Workspace` and call `lint::run`; partition by spec_id
  - Covers: REQ-005
  - Tests to write:
    - Diagnostics with `spec_id = Some("SPEC-NNNN")` route to that spec's lint block.
    - Diagnostics with `spec_id = None` route to the workspace-level lint block.
    - Each lint block has three arrays keyed by `Level` (errors / warnings / info).
    - Empty workspace produces an empty workspace-level lint block.
    - The ordering within each array is `(code, file, line)` ascending (inherited from lint::run).
  - Suggested files: `crates/speccy/src/status.rs`, `crates/speccy/tests/status_lint_integration.rs`

## Phase 5: Text renderer

- [x] **T-008**: Implement default text view with the in-progress + broken filter
  - Covers: REQ-006
  - Tests to write:
    - `status: in-progress` specs are shown unconditionally.
    - `status: implemented/dropped/superseded` specs WITHOUT errors/staleness/parse-errors are hidden.
    - Same specs WITH any of those signals ARE shown regardless of status.
    - Per-spec output: header line (`SPEC-NNNN <status>: <title>`), tasks counts, lint errors/warnings count, staleness flag with reasons, open-questions count.
    - Empty workspace -> `No specs in workspace.` + exit code 0.
  - Suggested files: `crates/speccy/src/status_output.rs`, `crates/speccy/tests/status_text_render.rs`, `crates/speccy/tests/status_text_filter.rs`

## Phase 6: JSON output

- [x] **T-009**: Implement `--json` envelope and per-spec output struct
  - Covers: REQ-007
  - Tests to write:
    - Output starts with `"schema_version": 1`.
    - Every spec appears in the `specs` array (no filtering by status).
    - Each spec entry has all required fields per SPEC.md contract.
    - Lint diagnostics are structured objects, not strings.
    - Pretty-printed (whitespace-tolerant).
    - Two runs with no filesystem change produce byte-identical output.
    - `stale_reasons` are ordered `HashDrift, MtimeDrift, BootstrapPending` (declared order).
  - Suggested files: `crates/speccy/src/status_output.rs` (extend), `crates/speccy/tests/status_json.rs`

- [x] **T-010**: Wire `repo_sha` via shell-out to `git rev-parse HEAD`
  - Covers: REQ-007
  - Tests to write:
    - Inside a git repo with HEAD: `repo_sha` is the 40-character SHA.
    - Outside a git repo (no `.git/`): `repo_sha` is `""`, no error.
    - Git not on PATH: `repo_sha` is `""`, no error.
    - HEAD unset (fresh repo): `repo_sha` is `""`, no error.
  - Suggested files: `crates/speccy/src/git.rs`, `crates/speccy/tests/git_repo_sha.rs`

## Phase 7: CLI wiring

- [x] **T-011**: Wire `speccy status [--json]` into the binary
  - Covers: REQ-001..REQ-007
  - Tests to write:
    - `speccy status` runs from any cwd inside a speccy workspace.
    - `speccy status` from outside a workspace -> exit code 1 with a clear `WorkspaceError::NoSpeccyDir` message.
    - `speccy status --json` emits valid JSON (`serde_json::from_str` round-trip).
    - End-to-end integration test via `assert_cmd` in a tmpdir with fixture specs.
  - Suggested files: `crates/speccy/src/main.rs`, `crates/speccy/src/status.rs`, `crates/speccy/tests/integration_status.rs`
