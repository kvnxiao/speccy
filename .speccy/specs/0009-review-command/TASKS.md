---
spec: SPEC-0009
spec_hash_at_generation: c1c6b1678065da3541173b9a5c9602cfc122869cf02f7d1f2dea2f348652efe8
generated_at: 2026-05-14T03:25:14Z
---

# Tasks: SPEC-0009 review-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0009 --commit` runs.

> Implementer note (retroactive, 2026-05-13): Tasks T-001..T-006
> landed in commit `f4720fe`. Checkboxes back-filled during the v1
> dogfood status sweep; no per-task review notes were captured at
> implementation time.

## Phase 1: Persona registry

<tasks spec="SPEC-0009">

<task id="T-001" state="completed" covers="REQ-001">
Define `personas::ALL` and align with SPEC-0007's DEFAULT_PERSONAS

- Suggested files: `speccy-core/src/personas.rs`, `speccy-core/src/next.rs` (refactor to consume `&personas::ALL[..4]`), `speccy-core/tests/personas_registry.rs`


<task-scenarios>
  - `personas::ALL` is a `&'static [&'static str]` of length 6 with the names `["business", "tests", "security", "style", "architecture", "docs"]` in declared order.
  - SPEC-0007's `DEFAULT_PERSONAS` is `personas::ALL[..4]` -- the two MUST be derived from the same source.
  - Adding a hypothetical 7th persona only requires modifying this constant; a regression test catches divergence with DEFAULT_PERSONAS.
</task-scenarios>
</task>

## Phase 2: Persona file resolver


<task id="T-002" state="completed" covers="REQ-002">
Implement `personas::resolve_file` with project-local-first chain

- Suggested files: `speccy-core/src/personas.rs` (extend), `speccy-core/tests/personas_resolve.rs`


<task-scenarios>
  - Given `.speccy/skills/personas/reviewer-security.md` exists with content "X", `resolve_file("security", root)` returns `"X"`.
  - Given the override does NOT exist but the embedded bundle has the file, returns the embedded content.
  - Given neither exists, returns `PersonaError::NotFound { name: "security" }`.
  - Given the project-local override is an empty file, warns on stderr and returns the embedded content.
  - Given an unknown persona name (not in `ALL`), returns `PersonaError::UnknownName`.
  - Host-native location (`.claude/commands/`) is NOT checked.
</task-scenarios>
</task>

## Phase 3: Diff computer


<task id="T-003" state="completed" covers="REQ-004">
Implement diff with fallback chain

- Suggested files: `speccy-cli/src/git.rs`, `speccy-cli/tests/git_diff.rs`


<task-scenarios>
  - Given uncommitted edits exist in a fixture git repo, the returned diff string is non-empty and matches `git diff HEAD` output.
  - Given a clean working tree but `HEAD~1` exists, the returned string matches `git diff HEAD~1 HEAD`.
  - Given a clean tree with no `HEAD~1` (single-commit repo), the returned string is the literal fallback note `<!-- no diff available; ... -->`.
  - Given `git` is not on PATH (simulated), returns the fallback note without error.
  - Given the directory is not a git repo, returns the fallback note.
</task-scenarios>
</task>

## Phase 4: Argument validation


<task id="T-004" state="completed" covers="REQ-003">
Validate `--persona` against the registry

- Suggested files: `speccy-cli/src/review.rs`, `speccy-cli/tests/review_persona_arg.rs`


<task-scenarios>
  - Missing `--persona` exits 1 with a usage message listing the six valid names.
  - `--persona security` succeeds (in registry).
  - `--persona unknown` exits 1; stderr contains `unknown` and the six valid names.
  - `--persona Security` (capitalised) exits 1 (case-sensitive).
</task-scenarios>
</task>

## Phase 5: Prompt assembly


<task id="T-005" state="completed" covers="REQ-005 REQ-006">
Render reviewer prompt for one persona

- Suggested files: `speccy-cli/src/review.rs` (extend), `skills/shared/prompts/reviewer-security.md` (stub; SPEC-0013 fills the other five and the real content), `speccy-cli/tests/review_prompt.rs`


<task-scenarios>
  - Looks up the task via `task_lookup::find` (SPEC-0008).
  - Loads `reviewer-<persona>.md` template via `prompt::load_template`.
  - Resolves persona content via `personas::resolve_file`.
  - Computes diff via the diff helper.
  - Placeholders substituted: `{{spec_id}}`, `{{spec_md}}`, `{{task_id}}`, `{{task_entry}}`, `{{diff}}`, `{{persona}}`, `{{persona_content}}`, `{{agents}}`.
  - Budget trimming applied.
  - Output to stdout; exit code 0.
</task-scenarios>
</task>

## Phase 6: CLI wiring and integration


<task id="T-006" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005 REQ-006">
Wire `speccy review TASK-ID --persona <name>` and integration tests

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/review.rs`, `speccy-cli/tests/integration_review.rs`

<task-scenarios>
  - End-to-end via `assert_cmd`: valid task ref + valid persona renders prompt to stdout (exit 0).
  - Ambiguity error suggests `speccy review SPEC-NNNN/T-NNN --persona <name>` form.
  - All persona-error paths (missing arg, unknown name) exit 1.
  - Project-local persona override is picked up without rebuilding the binary.
  - Outside-workspace exits 1.
</task-scenarios>
</task>

</tasks>
