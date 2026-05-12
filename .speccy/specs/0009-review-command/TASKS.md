---
spec: SPEC-0009
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0009 review-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0009 --commit` runs.

## Phase 1: Persona registry

- [ ] **T-001**: Define `personas::ALL` and align with SPEC-0007's DEFAULT_PERSONAS
  - Covers: REQ-001
  - Tests to write:
    - `personas::ALL` is a `&'static [&'static str]` of length 6 with the names `["business", "tests", "security", "style", "architecture", "docs"]` in declared order.
    - SPEC-0007's `DEFAULT_PERSONAS` is `personas::ALL[..4]` -- the two MUST be derived from the same source.
    - Adding a hypothetical 7th persona only requires modifying this constant; a regression test catches divergence with DEFAULT_PERSONAS.
  - Suggested files: `speccy-core/src/personas.rs`, `speccy-core/src/next.rs` (refactor to consume `&personas::ALL[..4]`), `speccy-core/tests/personas_registry.rs`

## Phase 2: Persona file resolver

- [ ] **T-002**: Implement `personas::resolve_file` with project-local-first chain
  - Covers: REQ-002
  - Tests to write:
    - Given `.speccy/skills/personas/reviewer-security.md` exists with content "X", `resolve_file("security", root)` returns `"X"`.
    - Given the override does NOT exist but the embedded bundle has the file, returns the embedded content.
    - Given neither exists, returns `PersonaError::NotFound { name: "security" }`.
    - Given the project-local override is an empty file, warns on stderr and returns the embedded content.
    - Given an unknown persona name (not in `ALL`), returns `PersonaError::UnknownName`.
    - Host-native location (`.claude/commands/`) is NOT checked.
  - Suggested files: `speccy-core/src/personas.rs` (extend), `speccy-core/tests/personas_resolve.rs`

## Phase 3: Diff computer

- [ ] **T-003**: Implement diff with fallback chain
  - Covers: REQ-004
  - Tests to write:
    - Given uncommitted edits exist in a fixture git repo, the returned diff string is non-empty and matches `git diff HEAD` output.
    - Given a clean working tree but `HEAD~1` exists, the returned string matches `git diff HEAD~1 HEAD`.
    - Given a clean tree with no `HEAD~1` (single-commit repo), the returned string is the literal fallback note `<!-- no diff available; ... -->`.
    - Given `git` is not on PATH (simulated), returns the fallback note without error.
    - Given the directory is not a git repo, returns the fallback note.
  - Suggested files: `speccy-cli/src/git.rs`, `speccy-cli/tests/git_diff.rs`

## Phase 4: Argument validation

- [ ] **T-004**: Validate `--persona` against the registry
  - Covers: REQ-003
  - Tests to write:
    - Missing `--persona` exits 1 with a usage message listing the six valid names.
    - `--persona security` succeeds (in registry).
    - `--persona unknown` exits 1; stderr contains `unknown` and the six valid names.
    - `--persona Security` (capitalised) exits 1 (case-sensitive).
  - Suggested files: `speccy-cli/src/review.rs`, `speccy-cli/tests/review_persona_arg.rs`

## Phase 5: Prompt assembly

- [ ] **T-005**: Render reviewer prompt for one persona
  - Covers: REQ-005, REQ-006
  - Tests to write:
    - Looks up the task via `task_lookup::find` (SPEC-0008).
    - Loads `reviewer-<persona>.md` template via `prompt::load_template`.
    - Resolves persona content via `personas::resolve_file`.
    - Computes diff via the diff helper.
    - Placeholders substituted: `{{spec_id}}`, `{{spec_md}}`, `{{task_id}}`, `{{task_entry}}`, `{{diff}}`, `{{persona}}`, `{{persona_content}}`, `{{agents}}`.
    - Budget trimming applied.
    - Output to stdout; exit code 0.
  - Suggested files: `speccy-cli/src/review.rs` (extend), `skills/shared/prompts/reviewer-security.md` (stub; SPEC-0013 fills the other five and the real content), `speccy-cli/tests/review_prompt.rs`

## Phase 6: CLI wiring and integration

- [ ] **T-006**: Wire `speccy review TASK-ID --persona <name>` and integration tests
  - Covers: REQ-001..REQ-006
  - Tests to write:
    - End-to-end via `assert_cmd`: valid task ref + valid persona renders prompt to stdout (exit 0).
    - Ambiguity error suggests `speccy review SPEC-NNNN/T-NNN --persona <name>` form.
    - All persona-error paths (missing arg, unknown name) exit 1.
    - Project-local persona override is picked up without rebuilding the binary.
    - Outside-workspace exits 1.
  - Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/review.rs`, `speccy-cli/tests/integration_review.rs`
