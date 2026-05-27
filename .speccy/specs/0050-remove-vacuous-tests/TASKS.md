---
spec: SPEC-0050
spec_hash_at_generation: b2f86baf25f08d1b537899dbd00a6182595becb401c6cff55871137ba72969ca
generated_at: 2026-05-27T20:54:45Z
---
# Tasks: SPEC-0050 Remove vacuous tests — delete 6 tests that gate editorial prose, file non-emptiness, or constant copies rather than behavior

<task id="T-001" state="pending" covers="REQ-001 REQ-002 REQ-003 REQ-004">
## Delete the six vacuous tests and verify the workspace hygiene suite stays green

Delete six `#[test]` functions across three files plus the orphan
scaffolding (introductory doc comments, section-header comment, and
the file-local `EXPECTED` constant in `personas.rs`) that only
served them. No source-prose files are edited; no replacement tests
are added. Then run the standard four-command hygiene suite to
catch any orphan-import, dead-code, or formatting drift introduced
by the deletions.

The six functions are:

1. `reviewer_tests_persona_pins_no_check_exit_code_evidence` in
   `speccy-cli/tests/init.rs` (currently L938-951).
2. `architecture_doc_pins_feedback_not_enforcement_contract` in
   `speccy-cli/tests/init.rs` (currently L954-971).
3. `architecture_doc_pins_check_command_is_render_only` in
   `speccy-cli/tests/init.rs` (currently L974-983).
4. `architecture_doc_pins_verify_command_is_shape_only` in
   `speccy-cli/tests/init.rs` (currently L986-999).
5. `bundle_layout_has_skill_md_per_host` in
   `speccy-cli/tests/skill_packs.rs` (currently L386-402).
6. `registry_contains_six_personas_in_declared_order` in
   `speccy-core/tests/personas.rs` (currently L21-28).

Orphan scaffolding to delete alongside the functions:

- The `/// Positive content pins.` doc comment block in `init.rs`
  (currently L928-936) that introduces the four prose pins. It
  attaches to the first prose-pin function by Rust doc-comment
  semantics; without those functions it is orphaned.
- The `// --------------------------------------------------------------------`
  bracketed section header `// Bundle layout: per-host SKILL.md.tmpl wrappers.`
  in `skill_packs.rs` (currently L382-384) that introduces only the
  single deleted test.
- The file-local `const EXPECTED: &[&str]` array in `personas.rs`
  (currently L12-19) that only fed the deleted test.

Line numbers above are at the SPEC-0050 starting commit and may
drift if the file changes for other reasons before this task runs;
the function and constant names are the canonical anchors.

Note on `include_str!` constants in `init.rs`: each of the four
deleted prose-pin functions defines its `REVIEWER_TESTS` or
`ARCHITECTURE` constant inside its own function body, so deleting
the functions takes the `include_str!` references with them — no
module-level orphan cleanup is required there.

<task-scenarios>
Given the speccy workspace at HEAD after this task lands,
when `rg "fn (reviewer_tests_persona_pins_no_check_exit_code_evidence|architecture_doc_pins_feedback_not_enforcement_contract|architecture_doc_pins_check_command_is_render_only|architecture_doc_pins_verify_command_is_shape_only|bundle_layout_has_skill_md_per_host|registry_contains_six_personas_in_declared_order)" speccy-cli/tests speccy-core/tests` runs,
then it reports zero matches across all six function names.

Given the same workspace state,
when `rg "Positive content pins|Bundle layout: per-host SKILL\.md\.tmpl wrappers|const EXPECTED" speccy-cli/tests speccy-core/tests` runs,
then it reports zero matches across all three orphan-scaffolding
anchors.

Given the same workspace state,
when `cargo test --workspace` runs,
then it exits 0 and the runner output does not contain any of the
six deleted function names.

Given the same workspace state,
when `cargo clippy --workspace --all-targets --all-features -- -D warnings` runs,
then it exits 0 with no `unused_imports`, `dead_code`, or similar
warnings introduced by the deletions.

Given the same workspace state,
when `cargo +nightly fmt --all --check` runs,
then it exits 0.

Given the same workspace state,
when `cargo deny check` runs,
then it exits 0.

Given a contributor rewords any of the previously-pinned sentences
in `docs/ARCHITECTURE.md` or
`resources/modules/personas/reviewer-tests.md` after this task
lands,
when `cargo test --workspace` runs,
then the suite still passes — confirming the prose pins are gone.

Given a contributor reorders the last two entries of
`speccy_core::personas::ALL` after this task lands,
when `cargo test --workspace` runs,
then the suite still passes — confirming the deleted personas test
was the only thing gating that order, consistent with DEC-003.

Suggested files: `speccy-cli/tests/init.rs`,
`speccy-cli/tests/skill_packs.rs`,
`speccy-core/tests/personas.rs`
</task-scenarios>
</task>
