---
spec: SPEC-0018
spec_hash_at_generation: f29dabf1bb99db54799bfdf63e731495d6407122c8e150ceacd9994942e2cfcc
generated_at: 2026-05-15T07:01:45Z
---

# Tasks: SPEC-0018 Remove check execution

## Phase 1: Scenario parser bridge

- [ ] **T-001**: Add temporary scenario-aware check parsing
  - Covers: REQ-001
  - Tests to write:
    - When `spec_toml` parses a `[[checks]]` row with only
      `id = "CHK-001"` and `scenario = "Given ..."`, then it returns a
      `CheckEntry` whose stable id is `CHK-001` and whose scenario text
      is preserved verbatim.
    - When the temporary SPEC-0018 bridge parses a legacy executable
      check with `command` and `proves`, then it derives
      `scenario = old_proves` while keeping the legacy payload available
      only for the still-unmigrated callers that T-002 through T-004
      retire.
    - When the temporary bridge parses a legacy manual check with
      `prompt`, then it derives `scenario = old_prompt`.
    - When a `scenario` field is empty or whitespace-only, then parsing
      fails with `ParseError::InvalidCheckEntry` and the error names the
      containing `CHK-NNN`.
    - When a check declares both a new `scenario` and legacy
      `command`/`prompt` fields during the bridge period, then parsing
      fails rather than guessing which source of truth wins.
    - The bridge code is labelled with a SPEC-0018 removal comment and
      a test or grep guard points T-004 at every remaining legacy branch.
  - Suggested files: `speccy-core/src/parse/toml_files.rs`, `speccy-core/src/parse/mod.rs`, `speccy-core/src/error.rs`, `speccy-core/tests/fixtures/lint/`, `speccy-core/tests/lint_common/mod.rs`

## Phase 2: Render scenarios, do not execute checks

- [ ] **T-002**: Replace `speccy check` execution with scenario rendering
  - Covers: REQ-002
  - Tests to write:
    - When `speccy check` runs with no selector against a workspace with
      multiple specs and multiple scenarios, then stdout prints one
      `==> CHK-NNN (SPEC-NNNN): <scenario first line>` header per
      selected scenario and ends with `N scenarios rendered across M
      specs`.
    - When a scenario spans multiple lines, then the first line appears
      in the header and each continuation line is printed indented
      under that header.
    - When a selected check still carries a legacy `command` during the
      temporary bridge period, then the command is not spawned; use a
      sentinel-file fixture to prove no child process ran.
    - When `speccy check SPEC-9999` runs, then the existing
      no-matching-spec error wording is preserved.
    - When each SPEC-0017 selector shape is used (`SPEC-NNNN`,
      `SPEC-NNNN/CHK-NNN`, `CHK-NNN`, `SPEC-NNNN/T-NNN`, and
      `T-NNN`), then the selected scenario set matches the current
      executable-check selection behavior.
    - When a task selector covers multiple requirements whose check
      lists overlap, then each scenario renders once in first-occurrence
      requirement order.
    - When output is searched, then old `<-- CHK-NNN PASS`, `FAIL`,
      `IN-FLIGHT`, and `MANUAL` footers are absent.
  - Suggested files: `speccy-cli/src/check.rs`, `speccy-cli/tests/check.rs`, `speccy-cli/tests/common/mod.rs`, `speccy-cli/src/main.rs`

- [ ] **T-003**: Make `speccy verify` a shape-only validator
  - Covers: REQ-003
  - Tests to write:
    - When a requirement has an empty `checks` array, then
      `speccy verify` exits 1 and names the requirement.
    - When a requirement references `CHK-099` and no matching scenario
      row exists, then `speccy verify` exits 1 and names both ids.
    - When a scenario row is not referenced by any requirement, then
      `speccy verify` reports the unreferenced `CHK-NNN` as a shape
      error.
    - When a clean workspace is verified, then verify exits 0 without
      calling `speccy check`, `run_checks_captured`, or any shell helper.
    - Text output ends with `verified N specs, M requirements, K
      scenarios; E errors`.
    - `speccy verify --json` emits `schema_version = 2`, contains the
      structural counts needed by the text summary, and contains no
      `outcome`, `exit_code`, or `duration_ms` fields.
    - Dropped and superseded specs remain non-gating in the same cases
      they are non-gating today, while workspace-level parse failures
      still gate verification.
  - Suggested files: `speccy-cli/src/verify.rs`, `speccy-cli/src/verify_output.rs`, `speccy-cli/tests/verify.rs`, `speccy-core/src/lint/rules/req.rs`, `speccy-core/src/lint/rules/val.rs`, `speccy-core/tests/lint_req.rs`, `speccy-core/tests/lint_val.rs`

## Phase 3: Hard-break migration and cleanup

- [ ] **T-004**: Migrate all `spec.toml` files and remove legacy check fields
  - Covers: REQ-001, REQ-004
  - Tests to write:
    - When every in-tree `.speccy/specs/**/spec.toml` file is parsed
      after migration, then every `[[checks]]` row contains exactly
      `id` and `scenario` and no row contains `kind`, `command`,
      `prompt`, or `proves`.
    - When a legacy `command`, `prompt`, `kind`, or `proves` field is
      present after the hard break, then deserialization fails through
      `#[serde(deny_unknown_fields)]`.
    - When old executable checks are migrated, then
      `scenario = old_proves`; when old manual checks are migrated,
      then `scenario = old_prompt`.
    - When `git grep -n "CheckPayload"` runs after cleanup, then there
      are no production-source hits.
    - When `git grep -n "speccy_core::exec\\|run_checks_captured\\|CheckOutcome\\|CheckResult\\|CheckSpec"` runs after cleanup, then there
      are no production-source hits.
    - The `speccy-core/tests/exec_captured.rs` subprocess suite is
      deleted, and any surviving check/verify tests assert rendering or
      shape validation rather than shell exit status.
    - `speccy-core/src/lib.rs` no longer exports `exec`, and
      `speccy-core/src/exec.rs` is deleted.
  - Suggested files: `.speccy/specs/**/spec.toml`, `speccy-core/src/parse/toml_files.rs`, `speccy-core/src/parse/mod.rs`, `speccy-core/src/lib.rs`, `speccy-core/src/exec.rs`, `speccy-core/tests/exec_captured.rs`, `speccy-cli/tests/check.rs`, `speccy-cli/tests/verify.rs`

## Phase 4: Contract language and dogfood outputs

- [ ] **T-005**: Update architecture docs and shipped prompts for scenarios
  - Covers: REQ-005
  - Tests to write:
    - When `.speccy/ARCHITECTURE.md` is inspected, then checks are
      described as English validation scenarios, `speccy check` is
      render-only, and `speccy verify` is shape-only.
    - The "Feedback, Not Enforcement" section explicitly says project
      CI owns test execution and reviewer personas own semantic
      judgment about scenario quality and test coverage.
    - When active docs and shipped prompts are searched for
      check-authoring examples, then `kind =`, `command =`, and
      `prompt =` do not appear except in historical migration notes.
    - Active examples use `scenario = """..."""` and do not tell
      planners, implementers, reviewers, or shippers that Speccy runs
      project tests.
    - The reviewer-tests persona tells reviewers to compare
      implementation and tests against scenario prose, not against
      command exit codes.
  - Suggested files: `.speccy/ARCHITECTURE.md`, `resources/modules/prompts/plan-greenfield.md`, `resources/modules/prompts/plan-amend.md`, `resources/modules/prompts/implementer.md`, `resources/modules/prompts/report.md`, `resources/modules/personas/reviewer-tests.md`, `.speccy/skills/prompts/`, `.speccy/skills/personas/`

- [ ] **T-006**: Refresh materialized host packs and lock CI expectations
  - Covers: REQ-003, REQ-005
  - Tests to write:
    - When `speccy init --force --host claude-code` and
      `speccy init --force --host codex` run in Speccy's checkout,
      then materialized `.claude/`, `.agents/`, `.codex/`, and
      `.speccy/skills/` outputs match the updated `resources/modules/`
      sources.
    - When `.github/workflows/ci.yml` is inspected, then project test
      commands (`cargo test`, clippy, fmt, and `cargo deny check`) run
      directly before `speccy verify`; CI does not rely on `speccy
      verify` to execute them.
    - When active generated host packs are searched, then they contain
      no unsubstituted MiniJinja tokens and no legacy check-authoring
      examples outside historical SPEC/TASKS records.
    - When `speccy check SPEC-0018` runs after the migration, then it
      renders SPEC-0018's scenarios and exits without spawning child
      processes.
    - When `speccy verify` runs after the migration, then it exits zero
      for the post-SPEC workspace shape.
  - Suggested files: `.github/workflows/ci.yml`, `speccy-cli/tests/ci_workflow.rs`, `speccy-cli/tests/init.rs`, `.claude/skills/`, `.claude/agents/`, `.agents/skills/`, `.codex/agents/`, `.speccy/skills/`
