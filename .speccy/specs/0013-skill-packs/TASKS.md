---
spec: SPEC-0013
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0013 skill-packs

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> lands and `speccy tasks SPEC-0013 --commit` runs.

## Phase 1: Persona files

- [ ] **T-001**: Write the planner and implementer personas
  - Covers: REQ-001, REQ-005
  - Tests to write:
    - `skills/shared/personas/planner.md` and `implementer.md` exist with non-zero byte length.
    - Each has the required sections per REQ-005 adapted for non-adversarial roles (Role / Focus / What to consider / Output format / Example).
  - Suggested files: `skills/shared/personas/planner.md`, `skills/shared/personas/implementer.md`

- [ ] **T-002**: Write the six reviewer personas
  - Covers: REQ-001, REQ-005
  - Tests to write:
    - All six files exist: `reviewer-business.md`, `reviewer-tests.md`, `reviewer-security.md`, `reviewer-style.md`, `reviewer-architecture.md`, `reviewer-docs.md`.
    - Each has the required sections per REQ-005 in declared order.
    - File names match `personas::ALL` exactly.
    - Each `## Example` section shows one realistic `Review (<persona>, pass|blocking): ...` note.
  - Suggested files: `skills/shared/personas/reviewer-business.md`, `skills/shared/personas/reviewer-tests.md`, `skills/shared/personas/reviewer-security.md`, `skills/shared/personas/reviewer-style.md`, `skills/shared/personas/reviewer-architecture.md`, `skills/shared/personas/reviewer-docs.md`

## Phase 2: Prompt templates

- [ ] **T-003**: Write the Phase 1 prompt templates (plan-greenfield, plan-amend)
  - Covers: REQ-002
  - Tests to write:
    - `plan-greenfield.md` contains placeholders `{{vision}}`, `{{agents}}`, `{{next_spec_id}}` (per SPEC-0005 REQ-001).
    - `plan-amend.md` contains `{{spec_id}}`, `{{spec_md}}`, `{{agents}}` (per SPEC-0005 REQ-002).
    - Both templates are non-empty and parse as valid markdown.
  - Suggested files: `skills/shared/prompts/plan-greenfield.md`, `skills/shared/prompts/plan-amend.md`

- [ ] **T-004**: Write the Phase 2 prompt templates (tasks-generate, tasks-amend)
  - Covers: REQ-002
  - Tests to write:
    - `tasks-generate.md` contains `{{spec_id}}`, `{{spec_md}}`, `{{agents}}` (per SPEC-0006 REQ-001).
    - `tasks-amend.md` contains `{{spec_id}}`, `{{spec_md}}`, `{{tasks_md}}`, `{{agents}}` (per SPEC-0006 REQ-002).
  - Suggested files: `skills/shared/prompts/tasks-generate.md`, `skills/shared/prompts/tasks-amend.md`

- [ ] **T-005**: Write the Phase 3 prompt template (implementer)
  - Covers: REQ-002
  - Tests to write:
    - `implementer.md` contains `{{spec_id}}`, `{{spec_md}}`, `{{task_id}}`, `{{task_entry}}`, `{{suggested_files}}`, `{{agents}}` (per SPEC-0008 REQ-004).
    - Content explicitly instructs the implementer-agent to flip `[ ]` -> `[~]` on start and `[~]` -> `[?]` on finish.
    - Content instructs running `speccy check` locally before flipping to `[?]`.
  - Suggested files: `skills/shared/prompts/implementer.md`

- [ ] **T-006**: Write the Phase 4 prompt templates (six reviewer prompts)
  - Covers: REQ-002
  - Tests to write:
    - Six files exist: `reviewer-business.md`, `reviewer-tests.md`, `reviewer-security.md`, `reviewer-style.md`, `reviewer-architecture.md`, `reviewer-docs.md` (under `skills/shared/prompts/`, distinct from the persona files in `skills/shared/personas/`).
    - Each contains placeholders `{{spec_id}}`, `{{spec_md}}`, `{{task_id}}`, `{{task_entry}}`, `{{diff}}`, `{{persona}}`, `{{persona_content}}`, `{{agents}}` (per SPEC-0009 REQ-005).
  - Suggested files: `skills/shared/prompts/reviewer-business.md`, `skills/shared/prompts/reviewer-tests.md`, `skills/shared/prompts/reviewer-security.md`, `skills/shared/prompts/reviewer-style.md`, `skills/shared/prompts/reviewer-architecture.md`, `skills/shared/prompts/reviewer-docs.md`

- [ ] **T-007**: Write the Phase 5 prompt template (report)
  - Covers: REQ-002
  - Tests to write:
    - `report.md` contains `{{spec_id}}`, `{{spec_md}}`, `{{tasks_md}}`, `{{retry_summary}}`, `{{agents}}` (per SPEC-0011 REQ-004).
    - Content instructs the agent to write REPORT.md frontmatter matching SPEC-0001 REQ-005 shape (spec / outcome / generated_at).
  - Suggested files: `skills/shared/prompts/report.md`

## Phase 3: Claude Code recipes

- [ ] **T-008**: Write Claude Code top-level recipes
  - Covers: REQ-003, REQ-006
  - Tests to write:
    - Seven files exist under `skills/claude-code/`: `speccy-init.md`, `speccy-plan.md`, `speccy-tasks.md`, `speccy-work.md`, `speccy-review.md`, `speccy-amend.md`, `speccy-ship.md`.
    - Each has Claude Code frontmatter parseable as YAML with a non-empty `description` field.
    - Each body has an intro paragraph, a `## When to use` heading, and at least one fenced code block with a `speccy` command from the v1 surface.
    - Loop recipes (`speccy-work`, `speccy-review`, `speccy-amend`) include explicit loop conditions and exit criteria.
  - Suggested files: `skills/claude-code/speccy-init.md`, `skills/claude-code/speccy-plan.md`, `skills/claude-code/speccy-tasks.md`, `skills/claude-code/speccy-work.md`, `skills/claude-code/speccy-review.md`, `skills/claude-code/speccy-amend.md`, `skills/claude-code/speccy-ship.md`

## Phase 4: Codex recipes

- [ ] **T-009**: Write Codex parallel recipes
  - Covers: REQ-004, REQ-006
  - Tests to write:
    - Same seven file names under `skills/codex/`.
    - Each has Codex-conforming frontmatter (parseable YAML; required fields per Codex's skill convention).
    - Body shape matches the Claude Code counterpart, adapted for Codex's invocation idioms where they differ.
  - Suggested files: `skills/codex/speccy-init.md`, `skills/codex/speccy-plan.md`, `skills/codex/speccy-tasks.md`, `skills/codex/speccy-work.md`, `skills/codex/speccy-review.md`, `skills/codex/speccy-amend.md`, `skills/codex/speccy-ship.md`

## Phase 5: Cross-host manual verification

- [ ] **T-010**: Manual smoke test in Claude Code and Codex
  - Covers: REQ-007
  - Tests to write:
    - This task's checks are manual (CHK-009 in spec.toml).
    - Document the runbook in this task: (1) `speccy init` in a fresh repo with `.claude/`; invoke each recipe; confirm each loads and runs its first CLI step. (2) Repeat with `.codex/` and Codex.
    - Capture findings as inline notes on this task for the report.
  - Suggested files: (none; manual verification only)
