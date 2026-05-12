---
spec: SPEC-0005
spec_hash_at_generation: bootstrap-pending
generated_at: 2026-05-11T00:00:00Z
---

# Tasks: SPEC-0005 plan-command

> `spec_hash_at_generation` is `bootstrap-pending` until SPEC-0006
> (`speccy tasks --commit`) lands.

## Phase 1: Template loading

- [ ] **T-001**: Implement `prompt::load_template` consuming embedded resources
  - Covers: REQ-005
  - Tests to write:
    - Looking up `"plan-greenfield.md"` returns the template content (stub or real).
    - Looking up `"plan-amend.md"` returns its template content.
    - Looking up an unknown name (e.g. `"nope.md"`) returns `PromptError::TemplateNotFound { name: "nope.md" }`.
    - The embedded bundle is wired via `include_dir!` (consistent with SPEC-0002 DEC-001).
  - Suggested files: `crates/speccy-core/src/prompt/template.rs`, `crates/speccy-core/Cargo.toml` (add `include_dir`), `skills/shared/prompts/plan-greenfield.md` (stub), `skills/shared/prompts/plan-amend.md` (stub), `crates/speccy-core/tests/prompt_template.rs`

## Phase 2: Placeholder substitution

- [ ] **T-002**: Implement `prompt::render`
  - Covers: REQ-005
  - Tests to write:
    - `"hello {{name}}"` + `{"name": "world"}` renders to `"hello world"`.
    - Multiple placeholders in one template each substitute.
    - Single-pass: `"{{a}} {{b}}"` + `{"a": "{{b}}", "b": "x"}` renders to `"{{b}} x"` (the substituted `{{b}}` from `a` is NOT re-scanned).
    - Unknown placeholders left in place: `"{{unknown}}"` renders to `"{{unknown}}"` AND stderr contains a warning naming `unknown`.
    - Duplicate unknown placeholders produce one warning per unique name, not one per occurrence.
    - Empty `vars` and empty `template` produce empty output without panicking.
  - Suggested files: `crates/speccy-core/src/prompt/render.rs`, `crates/speccy-core/tests/prompt_render.rs`

## Phase 3: AGENTS.md helper

- [ ] **T-003**: Implement `prompt::load_agents_md`
  - Covers: REQ-004
  - Tests to write:
    - Given `<project_root>/AGENTS.md` exists with content `# Agents\n...`, the function returns that content verbatim.
    - Given AGENTS.md is missing, the function returns the literal marker string and stderr contains a one-line warning naming the expected path.
    - Given AGENTS.md exists but is unreadable (simulated permission denied), the function returns the error-marker form and stderr warns.
  - Suggested files: `crates/speccy-core/src/prompt/agents_md.rs`, `crates/speccy-core/tests/prompt_agents_md.rs`

## Phase 4: Spec ID allocator

- [ ] **T-004**: Implement `prompt::allocate_next_spec_id`
  - Covers: REQ-003
  - Tests to write:
    - Empty `specs/` directory -> returns `"0001"`.
    - Absent `specs/` directory -> returns `"0001"`.
    - `0001-foo` + `0003-bar` present -> returns `"0004"` (no gap recycling).
    - `0042-foo` only -> returns `"0043"`.
    - Non-matching directory `_scratch` alongside `0001-foo` -> returns `"0002"` (ignored).
    - Directory with malformed numeric prefix (e.g. `00ab-foo`) -> ignored.
  - Suggested files: `crates/speccy-core/src/prompt/id_alloc.rs`, `crates/speccy-core/tests/id_alloc.rs`

## Phase 5: Context-budget trimmer

- [ ] **T-005**: Implement `prompt::trim_to_budget` with the DESIGN.md drop ordering
  - Covers: REQ-006
  - Tests to write:
    - 60,000-char input + 80,000 budget -> output unchanged, `dropped = []`, `fits = true`.
    - Input with a `## Notes` section that puts it over budget; trimming `## Notes` alone makes it fit -> `dropped = ["## Notes"]`.
    - Each step of the drop order is exercised independently with a fixture designed to make exactly that step the deciding drop.
    - Input that exceeds budget even after all five drop steps -> `fits = false`, output emitted anyway, stderr warning printed naming the overrun.
    - The `dropped` vec preserves the order in which steps fired.
  - Suggested files: `crates/speccy-core/src/prompt/budget.rs`, `crates/speccy-core/tests/prompt_budget.rs`

## Phase 6: Greenfield assembler

- [ ] **T-006**: Implement greenfield prompt assembler
  - Covers: REQ-001
  - Tests to write:
    - End-to-end: VISION.md content appears in the rendered output where `{{vision}}` was; AGENTS.md content appears where `{{agents}}` was; `{{next_spec_id}}` substituted to the allocated ID.
    - Output goes to stdout.
    - VISION.md missing -> exit code 1 with stderr message naming the path.
    - Project root not found (cwd outside any speccy workspace) -> exit code 1 with `PlanError::ProjectRootNotFound`.
    - AGENTS.md missing -> stderr warning per REQ-004; output still produced with marker.
  - Suggested files: `crates/speccy/src/plan.rs`, `crates/speccy/tests/plan_greenfield.rs`

## Phase 7: Amendment assembler

- [ ] **T-007**: Implement amendment prompt assembler
  - Covers: REQ-002
  - Tests to write:
    - `speccy plan SPEC-0001` resolves to `.speccy/specs/0001-<slug>/SPEC.md` and inlines it.
    - Invalid ID format (e.g. `FOO`) -> exit code 1 with `PlanError::InvalidSpecIdFormat`.
    - Missing spec (e.g. `SPEC-9999`) -> exit code 1 with `PlanError::SpecNotFound`.
    - Parse error on the existing SPEC.md -> exit code 1 with the parser error message.
    - `{{spec_md}}` substituted to full content; `{{spec_id}}` to the canonical ID; `{{agents}}` to AGENTS.md content.
  - Suggested files: `crates/speccy/src/plan.rs` (extend), `crates/speccy/tests/plan_amend.rs`

## Phase 8: CLI wiring

- [ ] **T-008**: Wire `speccy plan [SPEC-ID]` into the binary
  - Covers: REQ-001..REQ-006
  - Tests to write:
    - No-arg form runs end-to-end in a tmpdir fixture workspace; stdout receives the rendered prompt.
    - SPEC-ID arg form runs end-to-end with a fixture spec.
    - From outside any speccy workspace -> exit 1.
    - `assert_cmd`-based integration test that exec's the binary and inspects stdout/stderr/exit code.
    - Budget overrun: a deliberately-large fixture VISION.md exercises the trimmer; warning surfaces on stderr.
  - Suggested files: `crates/speccy/src/main.rs`, `crates/speccy/tests/integration_plan.rs`
