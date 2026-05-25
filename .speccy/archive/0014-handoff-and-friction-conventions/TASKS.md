---
spec: SPEC-0014
spec_hash_at_generation: 987ee57bbb1f1e02ce7426f13965a958015a43cfca9458d4602ae82ab8c6c4f9
generated_at: 2026-05-17T17:37:23Z
---

# Tasks: SPEC-0014 Handoff template + friction-to-skill-update conventions

> Hash recorded inline pending the next `speccy tasks SPEC-0014
> --commit` run; matches the sha256 of SPEC.md at draft time.

## Phase 1: Implementer-note handoff template


<task id="T-001" state="completed" covers="REQ-001">
Embed the six-field handoff template in the implementer prompt

- Suggested files: `skills/shared/prompts/implementer.md`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - `cargo test -p speccy-cli --test skill_packs -- implementer_prompt_handoff_template`
    asserts the prompt body contains a fenced ```` ```markdown ```` block
    that includes the six exact labels: `Completed`, `Undone`,
    `Commands run`, `Exit codes`, `Discovered issues`,
    `Procedural compliance`.
  - The test should locate the fenced block (not just any
    substring) so unrelated mentions in prose don't false-positive.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001">
Rewrite the prompt's `## Your task` step that produces the note

- Suggested files: `skills/shared/prompts/implementer.md`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - `cargo test -p speccy-cli --test skill_packs -- implementer_prompt_handoff_referenced_in_task_steps`
    asserts the `## Your task` body references the handoff template
    (by phrase or by repeating the field list) and instructs writing
    `(none)` for empty fields.
  - Negative case: confirm the old freeform "Append one implementer
    note ..." sentence is removed (string-absence check on the
    pre-edit text).
</task-scenarios>
</task>

## Phase 2: Friction-to-skill-update pattern


<task id="T-003" state="completed" covers="REQ-002">
Add the `## When you hit friction` section to the implementer prompt

- Suggested files: `skills/shared/prompts/implementer.md`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - `cargo test -p speccy-cli --test skill_packs -- implementer_prompt_friction_section`
    asserts the literal heading `## When you hit friction` exists
    in the prompt body and that the section contains at least one
    fenced code block whose contents reference a `skills/` path
    (e.g. `skills/claude-code/...` or `skills/shared/...`).
  - The section should sit between `## Suggested files` and
    `## Your task` so an implementer hits it before producing work.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-002">
Mirror the friction pointer in the implementer persona

- Suggested files: `skills/shared/personas/implementer.md`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - `cargo test -p speccy-cli --test skill_packs -- implementer_persona_friction_reference`
    asserts the `## What to consider` section of
    `skills/shared/personas/implementer.md` contains a bullet that
    mentions friction and points back to the prompt (substring
    match on a stable phrase like
    `update the relevant skill file under skills/`).
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-002">
Dogfood the pattern in speccy's own AGENTS.md

- Suggested files: `AGENTS.md`, `speccy-cli/tests/skill_packs.rs`
- Note: `AGENTS.md` is not part of the embedded bundle, so the test
  reads it from the source tree at compile-time. Document this in
  the test comment so future readers don't expect bundle access.

<task-scenarios>
  - `cargo test -p speccy-cli --test skill_packs -- agents_md_friction_paragraph`
    reads `AGENTS.md` from the workspace root (via
    `include_str!("../../AGENTS.md")` or `env!("CARGO_MANIFEST_DIR")`
    + `fs_err::read_to_string`) and asserts a paragraph under
    `## Conventions for AI agents specifically` documents the
    friction-to-skill-update pattern. The test should match on a
    stable invariant phrase the implementer commits to in the same
    change (e.g. `"update the relevant skill file under skills/"`).
</task-scenarios>
</task>

## Phase 3: REPORT.md skill-updates surfacing


<task id="T-006" state="completed" covers="REQ-003">
Add `## Skill updates` to the report prompt

- Suggested files: `skills/shared/prompts/report.md`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - `cargo test -p speccy-cli --test skill_packs -- report_prompt_skill_updates_section`
    asserts the literal substring `## Skill updates` is present in
    `skills/shared/prompts/report.md` and that it falls between
    `## Out-of-scope items absorbed` and
    `## Deferred / known limitations` in document order. Verify
    ordering by `body.find()` index comparison.
  - The prompt instructions for that section should mention
    `git diff --name-only -- skills/` or equivalent so the
    report-writing agent has a concrete derivation path.
</task-scenarios>
</task>

## Phase 4: Wire-up and verification


<task id="T-007" state="completed" covers="REQ-001 REQ-002 REQ-003">
Run the full hygiene sweep

- Suggested files: (none directly; iterates as needed)

<task-scenarios>
  - This task adds no new tests. It runs the four-command pre-commit
    gate and adjusts any drift the new content causes:
    `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    `cargo +nightly fmt --all --check`,
    `cargo deny check`.
  - Also runs `cargo run -p speccy-cli -- verify` to confirm the new
    checks pass and lint is clean.
</task-scenarios>
</task>

