---
spec: SPEC-0006
spec_hash_at_generation: a3a287413a63a9ac81439114c06bb267eba435735e02f6fa300aabe10fd20b59
generated_at: 2026-05-14T03:25:13Z
---

# Tasks: SPEC-0006 tasks-command

> `spec_hash_at_generation` is `bootstrap-pending` until this spec
> lands and `speccy tasks SPEC-0006 --commit` is the first command
> run against it (a fitting self-referential closure).

> Implementer note (retroactive, 2026-05-13): Tasks T-001..T-008
> landed in commit `727f48f`. Checkboxes back-filled during the v1
> dogfood status sweep; no per-task review notes were captured at
> implementation time.

## Phase 1: Spec lookup and argument validation

<tasks spec="SPEC-0006">

<task id="T-001" state="completed" covers="REQ-005">
Implement SPEC-ID parsing and spec-directory lookup

- Suggested files: `speccy-cli/src/tasks.rs`, `speccy-cli/tests/tasks_args.rs`


<task-scenarios>
  - Argument matching `SPEC-\d{4,}` is accepted; non-matching argument returns `TasksError::InvalidSpecIdFormat`.
  - `workspace::scan` (from SPEC-0004) is consulted to find the spec directory; missing ID returns `TasksError::SpecNotFound`.
  - SPEC.md parse error returns `TasksError::Parse` with the parser's underlying error.
</task-scenarios>
</task>

## Phase 2: Initial prompt assembler


<task id="T-002" state="completed" covers="REQ-001">
Detect initial form (TASKS.md absent) and render `tasks-generate.md`

- Suggested files: `speccy-cli/src/tasks.rs` (extend), `skills/shared/prompts/tasks-generate.md` (stub), `speccy-cli/tests/tasks_initial.rs`


<task-scenarios>
  - TASKS.md absent -> initial template is selected.
  - Embedded `tasks-generate.md` template is loaded via `prompt::load_template`.
  - `{{spec_id}}`, `{{spec_md}}`, `{{agents}}` placeholders are substituted.
  - Budget trimming applied via `prompt::trim_to_budget`.
  - Output goes to stdout; exit code 0.
</task-scenarios>
</task>

## Phase 3: Amendment prompt assembler


<task id="T-003" state="completed" covers="REQ-002">
Render `tasks-amend.md` when TASKS.md is present

- Suggested files: `speccy-cli/src/tasks.rs` (extend), `skills/shared/prompts/tasks-amend.md` (stub), `speccy-cli/tests/tasks_amendment.rs`


<task-scenarios>
  - TASKS.md present -> amendment template is selected.
  - Both SPEC.md and TASKS.md are parsed; parse errors on either return `TasksError::Parse`.
  - `{{spec_id}}`, `{{spec_md}}`, `{{tasks_md}}`, `{{agents}}` placeholders are substituted.
  - Budget trimming applied.
  - Output goes to stdout; exit code 0.
</task-scenarios>
</task>

## Phase 4: `--commit` core


<task id="T-004" state="completed" covers="REQ-003 REQ-004">
Implement `tasks::commit_frontmatter` -- body-byte-preserving rewrite

- Suggested files: `speccy-core/src/tasks.rs`, `speccy-core/tests/tasks_commit.rs`

<task-scenarios>
  - SPEC.md sha256 is written as 64-char hex into `spec_hash_at_generation`.
  - `generated_at` is set to the supplied UTC ISO 8601 timestamp.
  - The `spec` frontmatter field, if present, is preserved; if missing, set to the supplied SPEC-ID.
  - Any other frontmatter fields the agent added (e.g. `notes_for_future`) are preserved byte-identically.
  - **Body byte preservation**: body bytes (after the closing `---` fence) are byte-identical before and after the rewrite.
  - CRLF line endings in the body remain CRLF; LF stays LF.
  - Trailing whitespace in the body is preserved verbatim.
  - The function returns `Result<(), CommitError>`.
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-003">
Handle the bootstrap-pending sentinel and missing-frontmatter cases

- Suggested files: `speccy-core/src/tasks.rs` (extend), `speccy-core/tests/tasks_commit.rs` (extend)


<task-scenarios>
  - TASKS.md with `spec_hash_at_generation: bootstrap-pending` -> after commit, sentinel is replaced with the real hex hash.
  - TASKS.md with no frontmatter at all (just markdown body) -> commit prepends a fresh frontmatter block with `spec`, `spec_hash_at_generation`, `generated_at` (in that order) followed by the original body bytes.
  - TASKS.md with frontmatter whose `spec` field differs from the SPEC-ID arg -> commit returns `CommitError::SpecIdMismatch { in_file, in_arg }`; the file is NOT modified.
</task-scenarios>
</task>

## Phase 5: `--commit` wiring


<task id="T-006" state="completed" covers="REQ-003 REQ-005">
Wire `--commit` sub-action through the CLI

- Suggested files: `speccy-cli/src/tasks.rs` (extend), `speccy-cli/tests/tasks_commit.rs`


<task-scenarios>
  - `speccy tasks SPEC-NNNN --commit` with TASKS.md present succeeds; resulting file has updated frontmatter and unchanged body.
  - `speccy tasks SPEC-NNNN --commit` with TASKS.md absent exits 1 with `CommitError::TasksMdNotFound` mapped to a clear stderr message.
  - `speccy tasks SPEC-NNNN --commit` does NOT render any prompt to stdout (mutually exclusive with prompt-rendering forms).
  - UTC `now` is captured at command-start; second precision; `Z` suffix.
</task-scenarios>
</task>

## Phase 6: CLI wiring and integration


<task id="T-007" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005">
Wire `speccy tasks SPEC-ID [--commit]` into the binary

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/tests/integration_tasks.rs`

<task-scenarios>
  - End-to-end via `assert_cmd`:
    - Initial form on a tmpdir fixture writes the rendered prompt to stdout and exits 0.
    - Amendment form (with a pre-existing TASKS.md) writes the amendment prompt to stdout.
    - `--commit` writes frontmatter; the body is byte-identical before/after.
    - From outside a speccy workspace -> exit 1.
    - All argument-validation errors map to exit code 1 with informative messages.
</task-scenarios>
</task>

<task id="T-008" state="completed" covers="REQ-003 REQ-004">
Self-referential dogfood test: commit SPEC-0006's own TASKS.md

- Suggested files: `speccy-cli/tests/dogfood_self_commit.rs`

<task-scenarios>
  - As a one-shot integration test: run `speccy tasks SPEC-0006 --commit` against the actual `.speccy/specs/0006-tasks-command/TASKS.md`; assert the bootstrap-pending sentinel is replaced.
  - Run twice; second run only changes `generated_at`, not the hash.
  - This test is gated behind a feature flag or marked `#[ignore]` by default so CI doesn't mutate the repo; document the runbook for the maintainer to run manually.
</task-scenarios>
</task>

</tasks>
