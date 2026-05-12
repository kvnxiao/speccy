---
spec: SPEC-0006
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0006 tasks-command

> `spec_hash_at_generation` is `bootstrap-pending` until this spec
> lands and `speccy tasks SPEC-0006 --commit` is the first command
> run against it (a fitting self-referential closure).

## Phase 1: Spec lookup and argument validation

- [ ] **T-001**: Implement SPEC-ID parsing and spec-directory lookup
  - Covers: REQ-005
  - Tests to write:
    - Argument matching `SPEC-\d{4,}` is accepted; non-matching argument returns `TasksError::InvalidSpecIdFormat`.
    - `workspace::scan` (from SPEC-0004) is consulted to find the spec directory; missing ID returns `TasksError::SpecNotFound`.
    - SPEC.md parse error returns `TasksError::Parse` with the parser's underlying error.
  - Suggested files: `speccy-cli/src/tasks.rs`, `speccy-cli/tests/tasks_args.rs`

## Phase 2: Initial prompt assembler

- [ ] **T-002**: Detect initial form (TASKS.md absent) and render `tasks-generate.md`
  - Covers: REQ-001
  - Tests to write:
    - TASKS.md absent -> initial template is selected.
    - Embedded `tasks-generate.md` template is loaded via `prompt::load_template`.
    - `{{spec_id}}`, `{{spec_md}}`, `{{agents}}` placeholders are substituted.
    - Budget trimming applied via `prompt::trim_to_budget`.
    - Output goes to stdout; exit code 0.
  - Suggested files: `speccy-cli/src/tasks.rs` (extend), `skills/shared/prompts/tasks-generate.md` (stub), `speccy-cli/tests/tasks_initial.rs`

## Phase 3: Amendment prompt assembler

- [ ] **T-003**: Render `tasks-amend.md` when TASKS.md is present
  - Covers: REQ-002
  - Tests to write:
    - TASKS.md present -> amendment template is selected.
    - Both SPEC.md and TASKS.md are parsed; parse errors on either return `TasksError::Parse`.
    - `{{spec_id}}`, `{{spec_md}}`, `{{tasks_md}}`, `{{agents}}` placeholders are substituted.
    - Budget trimming applied.
    - Output goes to stdout; exit code 0.
  - Suggested files: `speccy-cli/src/tasks.rs` (extend), `skills/shared/prompts/tasks-amend.md` (stub), `speccy-cli/tests/tasks_amendment.rs`

## Phase 4: `--commit` core

- [ ] **T-004**: Implement `tasks::commit_frontmatter` -- body-byte-preserving rewrite
  - Covers: REQ-003, REQ-004
  - Tests to write:
    - SPEC.md sha256 is written as 64-char hex into `spec_hash_at_generation`.
    - `generated_at` is set to the supplied UTC ISO 8601 timestamp.
    - The `spec` frontmatter field, if present, is preserved; if missing, set to the supplied SPEC-ID.
    - Any other frontmatter fields the agent added (e.g. `notes_for_future`) are preserved byte-identically.
    - **Body byte preservation**: body bytes (after the closing `---` fence) are byte-identical before and after the rewrite.
    - CRLF line endings in the body remain CRLF; LF stays LF.
    - Trailing whitespace in the body is preserved verbatim.
    - The function returns `Result<(), CommitError>`.
  - Suggested files: `speccy-core/src/tasks.rs`, `speccy-core/tests/tasks_commit.rs`

- [ ] **T-005**: Handle the bootstrap-pending sentinel and missing-frontmatter cases
  - Covers: REQ-003
  - Tests to write:
    - TASKS.md with `spec_hash_at_generation: bootstrap-pending` -> after commit, sentinel is replaced with the real hex hash.
    - TASKS.md with no frontmatter at all (just markdown body) -> commit prepends a fresh frontmatter block with `spec`, `spec_hash_at_generation`, `generated_at` (in that order) followed by the original body bytes.
    - TASKS.md with frontmatter whose `spec` field differs from the SPEC-ID arg -> commit returns `CommitError::SpecIdMismatch { in_file, in_arg }`; the file is NOT modified.
  - Suggested files: `speccy-core/src/tasks.rs` (extend), `speccy-core/tests/tasks_commit.rs` (extend)

## Phase 5: `--commit` wiring

- [ ] **T-006**: Wire `--commit` sub-action through the CLI
  - Covers: REQ-003, REQ-005
  - Tests to write:
    - `speccy tasks SPEC-NNNN --commit` with TASKS.md present succeeds; resulting file has updated frontmatter and unchanged body.
    - `speccy tasks SPEC-NNNN --commit` with TASKS.md absent exits 1 with `CommitError::TasksMdNotFound` mapped to a clear stderr message.
    - `speccy tasks SPEC-NNNN --commit` does NOT render any prompt to stdout (mutually exclusive with prompt-rendering forms).
    - UTC `now` is captured at command-start; second precision; `Z` suffix.
  - Suggested files: `speccy-cli/src/tasks.rs` (extend), `speccy-cli/tests/tasks_commit.rs`

## Phase 6: CLI wiring and integration

- [ ] **T-007**: Wire `speccy tasks SPEC-ID [--commit]` into the binary
  - Covers: REQ-001..REQ-005
  - Tests to write:
    - End-to-end via `assert_cmd`:
      - Initial form on a tmpdir fixture writes the rendered prompt to stdout and exits 0.
      - Amendment form (with a pre-existing TASKS.md) writes the amendment prompt to stdout.
      - `--commit` writes frontmatter; the body is byte-identical before/after.
      - From outside a speccy workspace -> exit 1.
      - All argument-validation errors map to exit code 1 with informative messages.
  - Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/tests/integration_tasks.rs`

- [ ] **T-008**: Self-referential dogfood test: commit SPEC-0006's own TASKS.md
  - Covers: REQ-003, REQ-004
  - Tests to write:
    - As a one-shot integration test: run `speccy tasks SPEC-0006 --commit` against the actual `.speccy/specs/0006-tasks-command/TASKS.md`; assert the bootstrap-pending sentinel is replaced.
    - Run twice; second run only changes `generated_at`, not the hash.
    - This test is gated behind a feature flag or marked `#[ignore]` by default so CI doesn't mutate the repo; document the runbook for the maintainer to run manually.
  - Suggested files: `speccy-cli/tests/dogfood_self_commit.rs`
