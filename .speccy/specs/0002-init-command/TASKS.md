---
spec: SPEC-0002
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0002 init-command

> `spec_hash_at_generation` is `bootstrap-pending` because this spec
> was decomposed manually before `speccy tasks --commit` (SPEC-0006)
> exists. Backfill the real sha256 once that command lands.

## Phase 1: Embedded skill bundle

- [ ] **T-001**: Add `include_dir!` and embed `skills/` at compile time
  - Covers: REQ-004
  - Tests to write:
    - The embedded bundle exposes `skills/claude-code/`, `skills/codex/`, `skills/shared/personas/`, `skills/shared/prompts/` as walkable `include_dir::Dir` subtrees.
    - A test walks the bundle and asserts every file has non-empty content (catches accidentally-empty stubs).
    - Bundle compiles in release and debug modes.
  - Suggested files: `crates/speccy/Cargo.toml` (add `include_dir`), `crates/speccy/src/embedded.rs`, `skills/claude-code/.gitkeep`, `skills/codex/.gitkeep`, `skills/shared/personas/.gitkeep`, `skills/shared/prompts/.gitkeep`

## Phase 2: Host detection

- [ ] **T-002**: Implement the host detector
  - Covers: REQ-003
  - Tests to write:
    - `--host claude-code` wins regardless of which host directories exist.
    - No `--host`; `.claude/` present -> `HostChoice::ClaudeCode`.
    - No `--host`; no `.claude/`; `.codex/` present -> `HostChoice::Codex`.
    - No `--host`; only `.cursor/` present -> `InitError::CursorDetected`.
    - No `--host`; no host directories -> `HostChoice::ClaudeCode` with a `WarnedFallback` flag carried alongside.
    - `--host unknown` -> `InitError::UnknownHost { name: "unknown", supported: &["claude-code", "codex"] }`.
    - Probe order is deterministic: `.claude/` checked before `.codex/`.
  - Suggested files: `crates/speccy/src/host.rs`, `crates/speccy/tests/host.rs`

## Phase 3: Scaffold writer

- [ ] **T-003**: Implement the `.speccy/` scaffold writer
  - Covers: REQ-001, REQ-002
  - Tests to write:
    - Writes `.speccy/speccy.toml` with `schema_version = 1`, `[project]` block, `name` from the parent directory of the project root, `root = ".."`.
    - Writes `.speccy/VISION.md` with the template headings: Product, Users, V1.0 outcome, Constraints, Non-goals, Quality bar, Known unknowns (in declared order).
    - Refuses with `InitError::WorkspaceExists { path: ".speccy/" }` when `.speccy/` already exists and `--force` is false.
    - Output: lists `would create <path>` and `would overwrite <path>` lines on stdout before mutating.
    - The scaffolded `.speccy/speccy.toml` round-trips via the SPEC-0001 parser without errors.
  - Suggested files: `crates/speccy/src/scaffold.rs`, `crates/speccy/src/templates/vision_md.txt`, `crates/speccy/src/templates/speccy_toml.txt`, `crates/speccy/tests/scaffold.rs`

## Phase 4: Skill-pack copier

- [ ] **T-004**: Implement the skill-pack copier
  - Covers: REQ-004, REQ-002
  - Tests to write:
    - `claude-code` host -> files copy from embedded `skills/claude-code/` to `.claude/commands/<filename>`.
    - `codex` host -> files copy to `.codex/skills/<filename>`.
    - Destination directory is created (recursively) when missing.
    - Copied bytes match the embedded source via sha256.
    - `--force=true`: shipped files in the destination are overwritten; user-authored files (any filename not in the embedded bundle) are byte-identical before and after.
    - `--force=false` with an existing destination file conflict returns `InitError::WorkspaceExists` (extended variant or distinct error -- decide in T-005).
  - Suggested files: `crates/speccy/src/copy.rs`, `crates/speccy/tests/copy.rs`

## Phase 5: CLI wiring

- [ ] **T-005**: Wire `speccy init` as a subcommand in `main.rs`
  - Covers: REQ-001, REQ-002, REQ-003, REQ-004, REQ-005
  - Tests to write:
    - `speccy init` (no args) on a fresh repo: exits 0; scaffolds `.speccy/`; copies skills to the detected host.
    - `speccy init --host codex` overrides detection regardless of which host directories exist.
    - `speccy init` with existing `.speccy/`: exits 1; stderr names the path.
    - `speccy init --force` on the same: exits 0; overwrites shipped files; preserves user files.
    - `speccy init --host unknown`: exits 1; stderr contains `claude-code, codex`.
    - Cursor-only repo: exits 1; stderr mentions `cursor` and suggests `--host claude-code` or `--host codex`.
    - Simulated I/O failure (e.g. read-only root via `assert_fs`): exits 2; stderr contains the underlying error.
    - `InitError` -> exit-code mapping is consistent with REQ-005.
  - Suggested files: `crates/speccy/src/main.rs`, `crates/speccy/src/init.rs`, `crates/speccy/tests/init.rs`

- [ ] **T-006**: Print "would create / would overwrite" plan before mutating
  - Covers: REQ-002
  - Tests to write:
    - Capture stdout during a successful init; assert a line per file is printed before any actual write.
    - On `--force`, files being overwritten are tagged `overwrite`; new files are tagged `create`.
    - The summary lines appear before the success line.
  - Suggested files: `crates/speccy/src/plan_output.rs` (or inline in init.rs)

## Phase 6: Integration

- [ ] **T-007**: End-to-end integration test via `assert_cmd`
  - Covers: REQ-001..REQ-005
  - Tests to write:
    - Build the binary; exec in a `tempfile::TempDir`; assert on the resulting tree.
    - Cover: fresh init; refuse-without-force; force overwrite; host override; cursor refusal; unknown-host error.
    - Cross-platform: tests pass on Windows (cmd shell) and Linux (sh).
    - Exit-code assertions match REQ-005.
  - Suggested files: `crates/speccy/tests/integration_init.rs`, `crates/speccy/Cargo.toml` (add `assert_cmd`, `tempfile`, `assert_fs` dev-deps)
