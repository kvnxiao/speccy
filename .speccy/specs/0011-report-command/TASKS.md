---
spec: SPEC-0011
spec_hash_at_generation: 4275e8d80495e62d4df10a0ba5c58e0ee1750a0482c202d23141d22544dfc480
generated_at: 2026-05-14T03:25:15Z
---

# Tasks: SPEC-0011 report-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0011 --commit` runs.

## Phase 1: Argument validation and spec lookup

- [x] **T-001**: Parse SPEC-ID and locate the spec
  - Covers: REQ-001
  - Tests to write:
    - `"SPEC-0001"` -> proceed; `"FOO"` -> exit 1 with format error.
    - Spec directory exists -> proceeds; missing -> exit 1 with spec-not-found error.
    - SPEC.md and TASKS.md both required; either missing returns a clear "X required for report" error.
    - SPEC.md or TASKS.md parse failure surfaces the parser error to stderr; exit 1.
  - Suggested files: `speccy-cli/src/report.rs`, `speccy-cli/tests/report_args.rs`

## Phase 2: Completeness gate

- [x] **T-002**: Refuse when tasks aren't all `[x]`
  - Covers: REQ-002
  - Tests to write:
    - 5 `[x]` + 1 `[ ]` task -> exit 1; stderr lists the `[ ]` task ID and state.
    - 1 `[~]` task -> exit 1; stderr lists it as InProgress.
    - 1 `[?]` task -> exit 1; stderr lists it as AwaitingReview.
    - All `[x]` -> proceed to render.
    - TASKS.md with no task lines -> proceed (vacuously complete).
    - Multiple offending tasks -> all listed in stderr.
  - Suggested files: `speccy-cli/src/report.rs` (extend), `speccy-cli/tests/report_completeness.rs`

## Phase 3: Retry count computation

- [x] **T-003**: Count `Retry:` markers per task
  - Covers: REQ-003
  - Tests to write:
    - Task with two notes starting `Retry:` -> count 2.
    - Task with zero `Retry:` notes -> count 0.
    - Task with note `Retry on bcrypt` (no colon after `Retry`) -> count 0 (exact prefix match).
    - Case-sensitive: `retry:` (lowercase) does NOT count.
    - Rendered `{{retry_summary}}` is a markdown list with one bullet per task: `- T-NNN: N retries`.
  - Suggested files: `speccy-cli/src/report.rs` (extend), `speccy-cli/tests/report_retry.rs`

## Phase 4: Prompt assembly and CLI wiring

- [x] **T-004**: Render report prompt and wire CLI
  - Covers: REQ-004
  - Tests to write:
    - `report.md` template loaded via `prompt::load_template`.
    - Placeholders substituted: `{{spec_id}}`, `{{spec_md}}` (full SPEC.md content), `{{tasks_md}}` (full TASKS.md content), `{{retry_summary}}` (the markdown list), `{{agents}}`.
    - Budget trimming applied.
    - Output to stdout; exit code 0.
    - End-to-end via `assert_cmd` in a tmpdir fixture: completeness passes -> prompt rendered; completeness fails -> exit 1; outside-workspace -> exit 1.
  - Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/report.rs` (extend), `skills/shared/prompts/report.md` (stub; SPEC-0013 fills real content), `speccy-cli/tests/integration_report.rs`
