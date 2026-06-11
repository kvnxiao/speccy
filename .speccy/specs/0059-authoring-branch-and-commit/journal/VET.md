---
spec: SPEC-0059
generated_at: 2026-06-11T18:24:13Z
---

## Invocation 1 — 2026-06-11T18:24:13Z

<drift-review verdict="pass" round="1" date="2026-06-11T18:24:13Z" model="claude-opus-4-8[1m]">
The diff satisfies SPEC-0059 as a unit: all ten requirements covered, all three user stories trace end-to-end, no scope creep, non-goals respected. Branch-guard (`resources/modules/references/branch-guard.md`) and the shared commit recipe (`resources/modules/references/commit-recipe.md`) are each defined once and included by exactly their authorized callsites — branch-guard by plan/decompose/amend only (REQ-008), recipe by those three plus review-fanout (REQ-006/REQ-007). The review-pass refactor is behaviour-preserving (`git add -A`, `[SPEC-NNNN/T-NNN]:` prefix, single-parent, unguarded statement retained); the old `create spec and decompose tasks` combined title is gone from every source and ejected file. Both shared modules open with a no-git short-circuit (REQ-010), authoring commits stage narrowly with no clean-tree gate (REQ-009), and the amend commit sources `<why>` from the newest Changelog row and tolerates an absent TASKS.md (REQ-005). `just reeject` is a no-op against the committed packs and all 28 `authoring_commit.rs` tests pass.
</drift-review>
<simplifier-scan verdict="candidates">
One behaviour-preserving dedup candidate in the sole code file, speccy-cli/tests/authoring_commit.rs; all other diff files are prose (skill/SPEC/journal/evidence/reference modules) and out of scope.

- speccy-cli/tests/authoring_commit.rs:226-273 (and the six helpers spanning ~166-815): the six typed module-body readers — commit_recipe_body, branch_guard_body, review_fanout_body, decompose_body, plan_body, amend_body — each restate the identical `RESOURCES.get_file(rel).and_then(|f| f.contents_utf8()).unwrap_or_else(...)` shape. The T-007 generic `module_body(rel)` helper (added at ~line 921) already subsumes all six. Collapse the six typed helpers to call sites of `module_body("modules/.../X.md")`. This matches the existing convention in tests/persona_snippets.rs:45, which uses one generic RESOURCES body-reader rather than one helper per file. Tradeoff: the typed helpers carry task-specific panic messages (e.g. "SPEC-0059 T-001 requires this shared module to be created"); the generic message is uniform. Behaviour is identical (missing file still panics). Net: removes ~50 lines of near-clone fixtures.
- speccy-cli/tests/authoring_commit.rs: the five `rendered_*` tests (rendered_review_skill_fully_expands_commit_recipe ~line 437, rendered_decompose_agent_fully_expands_includes ~595, rendered_plan_skill_fully_expands_includes ~720, rendered_amend_skill_fully_expands_includes ~880, plus the per-file loop body inside rendered_authoring_skills_fully_expand_with_guard_and_recipe ~1050) repeat the same render_host_pack -> find rel_path -> assert no `{{`/`{%`/`{#` markers -> assert recipe text block. A small shared helper (mirroring persona_snippets.rs:59's rendered-file lookup) would remove the repetition. Lower-value than the body-reader collapse; the per-test recipe/guard assertions differ slightly per callsite, so only the lookup+marker-scan portion is genuinely shareable.
</simplifier-scan>
<simplifier-apply verdict="applied">
Applied candidate 1 (body-reader dedup) in speccy-cli/tests/authoring_commit.rs; all four hygiene gates green.

- Collapsed the six typed module-body readers (commit_recipe_body, branch_guard_body, review_fanout_body, decompose_body, plan_body, amend_body) into call sites of the existing generic module_body(rel), matching the persona_snippets.rs convention. Moved module_body + panic_with_message to the top of the file and removed the now-duplicate module_body definition from the T-007 section. Net ~50 lines of near-clone fixtures removed; behaviour identical (a missing module file still panics).
- Did NOT apply candidate 2 (rendered_* lookup helper): the scan flagged it as lower-value with per-callsite recipe/guard assertions differing, so only the lookup+marker-scan slice is shareable — extraction would not reduce cognitive load enough to justify a new shared abstraction.
- Hygiene: cargo test --workspace (28/28 authoring_commit tests pass, full workspace green), clippy -D warnings clean, +nightly fmt --check clean, cargo deny check ok (advisories/bans/licenses/sources ok; the license-not-encountered and duplicate cpufeatures warnings are pre-existing transitive-dep noise).
</simplifier-apply>
<gate verdict="passed" tasks_hash="c12e9de295b64f114cf0f2a600d9c9299c3e548eb4b0a73fe8f8dc4f5b82e693" date="2026-06-11T18:31:36Z">
No drift found on round 1; simplifier collapsed six typed body-readers onto the generic module_body helper, hygiene green.
</gate>
