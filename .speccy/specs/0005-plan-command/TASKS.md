---
spec: SPEC-0005
spec_hash_at_generation: 0f9747205df96e3f6e3e96778a44247d025f8c2703b5e83c48ddee9a0e5ce2f3
generated_at: 2026-05-17T17:37:23Z
---

# Tasks: SPEC-0005 plan-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

> Implementer note (retroactive, 2026-05-13): Tasks T-001..T-008
> landed in commit `960a5a1`. Checkboxes back-filled during the v1
> dogfood status sweep; no per-task review notes were captured at
> implementation time.

## Phase 1: Template loading

<tasks spec="SPEC-0005">

<task id="T-001" state="completed" covers="REQ-005">
Implement `prompt::load_template` consuming embedded resources

- Suggested files: `speccy-core/src/prompt/template.rs`, `speccy-core/Cargo.toml` (add `include_dir`), `skills/shared/prompts/plan-greenfield.md` (stub), `skills/shared/prompts/plan-amend.md` (stub), `speccy-core/tests/prompt_template.rs`


<task-scenarios>
  - Looking up `"plan-greenfield.md"` returns the template content (stub or real).
  - Looking up `"plan-amend.md"` returns its template content.
  - Looking up an unknown name (e.g. `"nope.md"`) returns `PromptError::TemplateNotFound { name: "nope.md" }`.
  - The embedded bundle is wired via `include_dir!` (consistent with SPEC-0002 DEC-001).
</task-scenarios>
</task>

## Phase 2: Placeholder substitution


<task id="T-002" state="completed" covers="REQ-005">
Implement `prompt::render`

- Suggested files: `speccy-core/src/prompt/render.rs`, `speccy-core/tests/prompt_render.rs`


<task-scenarios>
  - `"hello {{name}}"` + `{"name": "world"}` renders to `"hello world"`.
  - Multiple placeholders in one template each substitute.
  - Single-pass: `"{{a}} {{b}}"` + `{"a": "{{b}}", "b": "x"}` renders to `"{{b}} x"` (the substituted `{{b}}` from `a` is NOT re-scanned).
  - Unknown placeholders left in place: `"{{unknown}}"` renders to `"{{unknown}}"` AND stderr contains a warning naming `unknown`.
  - Duplicate unknown placeholders produce one warning per unique name, not one per occurrence.
  - Empty `vars` and empty `template` produce empty output without panicking.
</task-scenarios>
</task>

## Phase 3: AGENTS.md helper


<task id="T-003" state="completed" covers="REQ-004">
Implement `prompt::load_agents_md`

- Suggested files: `speccy-core/src/prompt/agents_md.rs`, `speccy-core/tests/prompt_agents_md.rs`


<task-scenarios>
  - Given `<project_root>/AGENTS.md` exists with content `# Agents\n...`, the function returns that content verbatim.
  - Given AGENTS.md is missing, the function returns the literal marker string and stderr contains a one-line warning naming the expected path.
  - Given AGENTS.md exists but is unreadable (simulated permission denied), the function returns the error-marker form and stderr warns.
</task-scenarios>
</task>

## Phase 4: Spec ID allocator


<task id="T-004" state="completed" covers="REQ-003">
Implement `prompt::allocate_next_spec_id`

- Suggested files: `speccy-core/src/prompt/id_alloc.rs`, `speccy-core/tests/id_alloc.rs`


<task-scenarios>
  - Empty `specs/` directory -> returns `"0001"`.
  - Absent `specs/` directory -> returns `"0001"`.
  - `0001-foo` + `0003-bar` present -> returns `"0004"` (no gap recycling).
  - `0042-foo` only -> returns `"0043"`.
  - Non-matching directory `_scratch` alongside `0001-foo` -> returns `"0002"` (ignored).
  - Directory with malformed numeric prefix (e.g. `00ab-foo`) -> ignored.
</task-scenarios>
</task>

## Phase 5: Context-budget trimmer


<task id="T-005" state="completed" covers="REQ-006">
Implement `prompt::trim_to_budget` with the ARCHITECTURE.md drop ordering

- Suggested files: `speccy-core/src/prompt/budget.rs`, `speccy-core/tests/prompt_budget.rs`


<task-scenarios>
  - 60,000-char input + 80,000 budget -> output unchanged, `dropped = []`, `fits = true`.
  - Input with a `## Notes` section that puts it over budget; trimming `## Notes` alone makes it fit -> `dropped = ["## Notes"]`.
  - Each step of the drop order is exercised independently with a fixture designed to make exactly that step the deciding drop.
  - Input that exceeds budget even after all five drop steps -> `fits = false`, output emitted anyway, stderr warning printed naming the overrun.
  - The `dropped` vec preserves the order in which steps fired.
</task-scenarios>
</task>

## Phase 6: Greenfield assembler


<task id="T-006" state="completed" covers="REQ-001">
Implement greenfield prompt assembler

- Suggested files: `speccy-cli/src/plan.rs`, `speccy-cli/tests/plan_greenfield.rs`


<task-scenarios>
  - End-to-end: AGENTS.md content appears in the rendered output where `{{agents}}` was; `{{next_spec_id}}` substituted to the allocated ID. There is no `{{vision}}` placeholder.
  - Output goes to stdout.
  - Project root not found (cwd outside any speccy workspace) -> exit code 1 with `PlanError::ProjectRootNotFound`.
  - AGENTS.md missing -> stderr warning per REQ-004; output still produced with marker.
  - The spec-ID allocator walks `.speccy/specs/**` so flat and mission-grouped specs share one ID space (REQ-003).
</task-scenarios>
</task>

## Phase 7: Amendment assembler


<task id="T-007" state="completed" covers="REQ-002">
Implement amendment prompt assembler

- Suggested files: `speccy-cli/src/plan.rs` (extend), `speccy-cli/tests/plan_amend.rs`


<task-scenarios>
  - `speccy plan SPEC-0001` resolves to `.speccy/specs/0001-<slug>/SPEC.md` and inlines it.
  - Invalid ID format (e.g. `FOO`) -> exit code 1 with `PlanError::InvalidSpecIdFormat`.
  - Missing spec (e.g. `SPEC-9999`) -> exit code 1 with `PlanError::SpecNotFound`.
  - Parse error on the existing SPEC.md -> exit code 1 with the parser error message.
  - `{{spec_md}}` substituted to full content; `{{spec_id}}` to the canonical ID; `{{agents}}` to AGENTS.md content.
</task-scenarios>
</task>

## Phase 8: CLI wiring


<task id="T-008" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005 REQ-006">
Wire `speccy plan [SPEC-ID]` into the binary

- Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/tests/integration_plan.rs`

<task-scenarios>
  - No-arg form runs end-to-end in a tmpdir fixture workspace; stdout receives the rendered prompt.
  - SPEC-ID arg form runs end-to-end with a fixture spec.
  - From outside any speccy workspace -> exit 1.
  - `assert_cmd`-based integration test that exec's the binary and inspects stdout/stderr/exit code.
  - Budget overrun: a deliberately-large fixture AGENTS.md exercises the trimmer; warning surfaces on stderr.
</task-scenarios>
</task>

</tasks>
