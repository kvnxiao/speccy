---
spec: SPEC-0046
outcome: implemented
generated_at: 2026-05-26T20:00:00Z
---

# REPORT: SPEC-0046 Rename the `/speccy-tasks` skill to `/speccy-decompose`

<report spec="SPEC-0046">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 renamed the four installed skill artifacts using `git mv` to preserve history: `.claude/skills/speccy-tasks/` became `.claude/skills/speccy-decompose/`, `.claude/agents/speccy-tasks.md` became `.claude/agents/speccy-decompose.md`, `.agents/skills/speccy-tasks/` became `.agents/skills/speccy-decompose/`, and `.codex/agents/speccy-tasks.toml` became `.codex/agents/speccy-decompose.toml`. The frontmatter `name:` / TOML `name = ` field was updated to `speccy-decompose` in each renamed file, and level-1 headings and self-referential body lines were updated to match. A recursive search for `speccy-tasks` over the four install-location trees returns no matches. `git log --follow` on each new path resolves to commits on the old paths. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
T-002 mirrored the install-side renames on the template side so `speccy init` writes the new name on fresh installs. `resources/agents/.claude/skills/speccy-tasks/`, `resources/agents/.claude/agents/speccy-tasks.md.tmpl`, `resources/agents/.agents/skills/speccy-tasks/`, `resources/agents/.codex/agents/speccy-tasks.toml.tmpl`, and `resources/modules/phases/speccy-tasks.md` were each renamed to their `speccy-decompose` counterparts via `git mv`. All `speccy-tasks` tokens inside the renamed template bodies were replaced. A recursive search for `speccy-tasks` over `resources/` returns no matches. The integration test in T-004 verifies that `speccy init` installs `speccy-decompose` entries and no `speccy-tasks` entries. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004 CHK-005">
T-003 swept every remaining cross-skill and documentation reference. Updated files include the shipped skill bodies for `speccy-plan`, `speccy-brainstorm`, `speccy-orchestrate`, and `speccy-work` in all four pack locations (`.claude/skills/`, `.agents/skills/`, `.claude/agents/`, `.codex/agents/`) and their `resources/agents/` template counterparts, plus `README.md` and `docs/ARCHITECTURE.md`. A post-vet commit (`902aeb0`) renamed the test function `chk019_speccy_tasks_template_documents_output_shape` to `chk019_speccy_decompose_template_documents_output_shape` to remove the last underscore-form residue. A recursive search for `speccy-tasks` over the tree (excluding `.speccy/archive/`, `target/`, `.git/`, and the SPEC-0046 historical-context files) returns no matches. Both `speccy-plan` and `speccy-brainstorm` contain at least one `/speccy-decompose` suggestion. `README.md` and `docs/ARCHITECTURE.md` contain `speccy-decompose` and no `speccy-tasks`. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006 CHK-007">
T-004 updated all integration and inline test files that hard-coded the old slug: `speccy-cli/tests/init.rs`, `speccy-cli/tests/init_phase_agents.rs`, `speccy-cli/tests/pin_shape.rs`, `speccy-cli/tests/skill_packs.rs`, `speccy-cli/tests/skill_body_discovery.rs`, and the inline `#[cfg(test)]` block in `speccy-cli/src/render.rs`. All four hygiene commands pass: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny check`. Retry count: 0.
</coverage>

</report>
