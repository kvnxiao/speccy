---
spec: SPEC-0002
spec_hash_at_generation: 868f3f178cd9446f7af238b58e05aa16dc526b592ead2b23a0de2d280dea7dd7
generated_at: 2026-05-14T05:28:44Z
---

# Tasks: SPEC-0002 init-command

> `spec_hash_at_generation` is `bootstrap-pending` because this spec
> was decomposed manually before `speccy tasks --commit` (SPEC-0006)
> exists. Backfill the real sha256 once that command lands.

> Implementer note (retroactive, 2026-05-13): Tasks T-001..T-007
> landed in commit `5041dc0`. Checkboxes back-filled during the v1
> dogfood status sweep; no per-task review notes were captured at
> implementation time.

## Phase 1: Embedded skill bundle

<tasks spec="SPEC-0002">

<task id="T-001" state="completed" covers="REQ-004">
Add `include_dir!` and embed `skills/` at compile time

- Suggested files: `speccy-cli/Cargo.toml` (add `include_dir`), `speccy-cli/src/embedded.rs`, `skills/claude-code/.gitkeep`, `skills/codex/.gitkeep`, `skills/shared/personas/.gitkeep`, `skills/shared/prompts/.gitkeep`


<task-scenarios>
  - The embedded bundle exposes `skills/claude-code/`, `skills/codex/`, `skills/shared/personas/`, `skills/shared/prompts/` as walkable `include_dir::Dir` subtrees.
  - A test walks the bundle and asserts every file has non-empty content (catches accidentally-empty stubs).
  - Bundle compiles in release and debug modes.
</task-scenarios>
</task>

## Phase 2: Host detection


<task id="T-002" state="completed" covers="REQ-003">
Implement the host detector

- Suggested files: `speccy-cli/src/host.rs`, `speccy-cli/tests/host.rs`


<task-scenarios>
  - `--host claude-code` wins regardless of which host directories exist.
  - No `--host`; `.claude/` present -> `HostChoice::ClaudeCode`.
  - No `--host`; no `.claude/`; `.codex/` present -> `HostChoice::Codex`.
  - No `--host`; only `.cursor/` present -> `InitError::CursorDetected`.
  - No `--host`; no host directories -> `HostChoice::ClaudeCode` with a `WarnedFallback` flag carried alongside.
  - `--host unknown` -> `InitError::UnknownHost { name: "unknown", supported: &["claude-code", "codex"] }`.
  - Probe order is deterministic: `.claude/` checked before `.codex/`.
</task-scenarios>
</task>

## Phase 3: Scaffold writer


<task id="T-003" state="completed" covers="REQ-001 REQ-002">
Implement the `.speccy/` scaffold writer

- Suggested files: `speccy-cli/src/scaffold.rs`, `speccy-cli/src/templates/speccy_toml.txt`, `speccy-cli/tests/scaffold.rs`


<task-scenarios>
  - Writes `.speccy/speccy.toml` with `schema_version = 1`, `[project]` block, and `name` from the parent directory of the project root.
  - Does **not** scaffold `.speccy/VISION.md` (the noun has been retired; the product north star lives in `AGENTS.md` instead, populated by the `speccy-init` skill).
  - Refuses with `InitError::WorkspaceExists { path: ".speccy/" }` when `.speccy/` already exists and `--force` is false.
  - Output: lists `would create <path>` and `would overwrite <path>` lines on stdout before mutating.
  - The scaffolded `.speccy/speccy.toml` round-trips via the SPEC-0001 parser without errors.
</task-scenarios>
</task>

## Phase 4: Skill-pack copier


<task id="T-004" state="completed" covers="REQ-004 REQ-002">
Implement the skill-pack copier

- Suggested files: `speccy-cli/src/copy.rs`, `speccy-cli/tests/copy.rs`


<task-scenarios>
  - `claude-code` host -> files copy from embedded `skills/claude-code/` to `.claude/commands/<filename>`.
  - `codex` host -> files copy to `.codex/skills/<filename>`.
  - Destination directory is created (recursively) when missing.
  - Copied bytes match the embedded source via sha256.
  - `--force=true`: shipped files in the destination are overwritten; user-authored files (any filename not in the embedded bundle) are byte-identical before and after.
  - `--force=false` with an existing destination file conflict returns `InitError::WorkspaceExists` (extended variant or distinct error -- decide in T-005).
</task-scenarios>
</task>

## Phase 5: CLI wiring


<task id="T-005" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005">
Wire `speccy init` as a subcommand in `main.rs`

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/init.rs`, `speccy-cli/tests/init.rs`

<task-scenarios>
  - `speccy init` (no args) on a fresh repo: exits 0; scaffolds `.speccy/`; copies skills to the detected host.
  - `speccy init --host codex` overrides detection regardless of which host directories exist.
  - `speccy init` with existing `.speccy/`: exits 1; stderr names the path.
  - `speccy init --force` on the same: exits 0; overwrites shipped files; preserves user files.
  - `speccy init --host unknown`: exits 1; stderr contains `claude-code, codex`.
  - Cursor-only repo: exits 1; stderr mentions `cursor` and suggests `--host claude-code` or `--host codex`.
  - Simulated I/O failure (e.g. read-only root via `assert_fs`): exits 2; stderr contains the underlying error.
  - `InitError` -> exit-code mapping is consistent with REQ-005.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-002">
Print "would create / would overwrite" plan before mutating

- Suggested files: `speccy-cli/src/plan_output.rs` (or inline in init.rs)


<task-scenarios>
  - Capture stdout during a successful init; assert a line per file is printed before any actual write.
  - On `--force`, files being overwritten are tagged `overwrite`; new files are tagged `create`.
  - The summary lines appear before the success line.
</task-scenarios>
</task>

## Phase 6: Integration


<task id="T-007" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005">
End-to-end integration test via `assert_cmd`

- Suggested files: `speccy-cli/tests/integration_init.rs`, `speccy-cli/Cargo.toml` (add `assert_cmd`, `tempfile`, `assert_fs` dev-deps)

<task-scenarios>
  - Build the binary; exec in a `tempfile::TempDir`; assert on the resulting tree.
  - Cover: fresh init; refuse-without-force; force overwrite; host override; cursor refusal; unknown-host error.
  - Cross-platform: tests pass on Windows (cmd shell) and Linux (sh).
  - Exit-code assertions match REQ-005.
</task-scenarios>
</task>

</tasks>
