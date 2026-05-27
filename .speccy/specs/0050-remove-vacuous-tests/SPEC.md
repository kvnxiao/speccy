---
id: SPEC-0050
slug: remove-vacuous-tests
title: Remove vacuous tests — delete 6 tests that gate editorial prose, file non-emptiness, or constant copies rather than behavior
status: implemented
created: 2026-05-27
supersedes: []
---

# SPEC-0050: Remove vacuous tests — delete 6 tests that gate editorial prose, file non-emptiness, or constant copies rather than behavior

## Summary

`AGENTS.md` "Conventions for AI agents specifically" enumerates five
patterns that make a test vacuous: prose substring-match, hard-coded
constant re-assert, file existence / non-emptiness only,
self-referential mocks, and loose outcome asserts. A sweep of the
workspace surfaces six existing tests that match these patterns. They
either break on legitimate doc rewrites without catching real
regressions, or assert tautologies the surrounding machinery already
guarantees. This SPEC deletes those six tests (and their orphan
scaffolding) without adding replacements. Doc-drift concerns the
prose pins were defending revert to social review per AGENTS.md.

## Goals

<goals>
- The four prose-substring-match tests in `speccy-cli/tests/init.rs`
  (`reviewer_tests_persona_pins_no_check_exit_code_evidence`,
  `architecture_doc_pins_feedback_not_enforcement_contract`,
  `architecture_doc_pins_check_command_is_render_only`,
  `architecture_doc_pins_verify_command_is_shape_only`) no longer
  exist in the codebase.
- The file-non-emptiness test
  `bundle_layout_has_skill_md_per_host` in
  `speccy-cli/tests/skill_packs.rs` no longer exists in the codebase.
- The constant re-assert test
  `registry_contains_six_personas_in_declared_order` in
  `speccy-core/tests/personas.rs` no longer exists in the codebase,
  and the `EXPECTED` constant that only fed it is also gone.
- `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  and `cargo +nightly fmt --all --check` all pass after the deletions.
- No orphan section comments or doc comments survive the deletions;
  the surrounding file structure stays internally coherent.
</goals>

## Non-goals

<non-goals>
- No changes to the source prose files
  (`docs/ARCHITECTURE.md`, `resources/modules/personas/reviewer-tests.md`,
  any `resources/agents/.../SKILL.md.tmpl` wrapper). The tests go;
  the prose they pinned stays as-is.
- No replacement tests. AGENTS.md explicitly endorses social review
  for doc-drift concerns; downstream render/parse tests cover wrapper
  content shape; persona uniqueness and prefix tests (#2 and #3 in
  `personas.rs`) continue to gate the load-bearing properties.
- No structural lints introduced to defend the contracts the prose
  pins were guarding. If such lints are valuable, they belong to a
  follow-up SPEC, not this cleanup.
- No removal of the lint-registry snapshot test in
  `speccy-core/tests/lint_registry.rs`. It pins structured registry
  data (codes, severities, ordering), not human-curated prose, and is
  a legitimate fingerprint pin.
- No removal of XML round-trip tests in `speccy-core/tests/`
  (`report_xml_roundtrip.rs`, `spec_xml_roundtrip.rs`,
  `task_xml_roundtrip.rs`). They gate real custom parser/renderer
  behavior, not derive-only serde round-trips.
- No refactoring of adjacent non-vacuous tests in the same files. The
  scope is exactly the six functions named in `## Goals`.
</non-goals>

## User Stories

<user-stories>
- As a contributor rewriting `docs/ARCHITECTURE.md` or the
  reviewer-tests persona body, I want to rephrase or reformat prose
  without CI failing on substring asserts that only gate editorial
  wording.
- As a reviewer reading the test suite, I want every `#[test]` in the
  workspace to gate a real invariant of the system under test, so that
  "all tests pass" carries meaningful signal rather than noise from
  tautological asserts.
- As a future contributor wondering whether a similar test is worth
  writing, I want the existing suite to reflect the AGENTS.md vacuous
  taxonomy as removed code, not as living counterexamples.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Four prose-substring-match tests removed from `speccy-cli/tests/init.rs`

The four `#[test]` functions in `speccy-cli/tests/init.rs` that load
`docs/ARCHITECTURE.md` or
`resources/modules/personas/reviewer-tests.md` via `include_str!`
and assert on `.contains(<verbatim-sentence>)` are deleted from the
file. The orphan introductory doc comment that precedes them
(currently at L928-936, beginning `/// Positive content pins.`) is
also deleted so the file does not carry a doc comment that
introduces nothing.

<done-when>
- `rg "fn reviewer_tests_persona_pins_no_check_exit_code_evidence" speccy-cli/tests/init.rs` returns no matches.
- `rg "fn architecture_doc_pins_feedback_not_enforcement_contract" speccy-cli/tests/init.rs` returns no matches.
- `rg "fn architecture_doc_pins_check_command_is_render_only" speccy-cli/tests/init.rs` returns no matches.
- `rg "fn architecture_doc_pins_verify_command_is_shape_only" speccy-cli/tests/init.rs` returns no matches.
- `rg "Positive content pins" speccy-cli/tests/init.rs` returns no matches.
- `rg 'include_str!\("\.\./\.\./docs/ARCHITECTURE\.md"\)' speccy-cli/tests/init.rs` returns no matches.
- `rg 'include_str!\("\.\./\.\./resources/modules/personas/reviewer-tests\.md"\)' speccy-cli/tests/init.rs` returns no matches.
</done-when>

<behavior>
- Given the test file after this requirement lands, when `cargo test --workspace` runs, then none of the four named test functions execute (they no longer exist) and the rest of the suite passes.
- Given a contributor rewords any of the previously-pinned sentences in `docs/ARCHITECTURE.md` or `resources/modules/personas/reviewer-tests.md`, when `cargo test --workspace` runs, then the suite passes (the pins no longer gate editorial wording).
- Given `cargo clippy --workspace --all-targets --all-features -- -D warnings` runs after the deletions, then it passes with no `unused_imports`, `dead_code`, or similar warnings introduced by orphan scaffolding.
</behavior>

<scenario id="CHK-001">
Given the speccy workspace at HEAD after this SPEC lands,
when `rg -c "fn (reviewer_tests_persona_pins_no_check_exit_code_evidence|architecture_doc_pins_feedback_not_enforcement_contract|architecture_doc_pins_check_command_is_render_only|architecture_doc_pins_verify_command_is_shape_only)" speccy-cli/tests/init.rs` runs,
then it reports `0` matches (or exits non-zero with no output, depending on ripgrep version).
</scenario>

<scenario id="CHK-002">
Given the same workspace state,
when `cargo test --workspace --test init` runs,
then it exits 0 and the test runner output does not contain any of the four removed function names.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: File-non-emptiness test removed from `speccy-cli/tests/skill_packs.rs`

The `bundle_layout_has_skill_md_per_host` function at L386-402, plus
its orphan section-header comment block at L382-384
(`// Bundle layout: per-host SKILL.md.tmpl wrappers.`), are deleted
from `speccy-cli/tests/skill_packs.rs`. The function's only
non-tautological work was a `read_to_string` whose failure was
caught by `panic_with_test_message`; the `assert!(!body.trim().is_empty())`
follow-up was the vacuous part per AGENTS.md pattern 3. Downstream
tests in the same file that read and parse the wrapper bodies (e.g.
`shipped_skill_md_frontmatter_shape`) already gate file existence
and content shape with meaningful asserts.

<done-when>
- `rg "fn bundle_layout_has_skill_md_per_host" speccy-cli/tests/skill_packs.rs` returns no matches.
- `rg "Bundle layout: per-host SKILL\.md\.tmpl wrappers" speccy-cli/tests/skill_packs.rs` returns no matches.
- The function `shipped_skill_md_frontmatter_shape` and other adjacent tests remain unchanged.
</done-when>

<behavior>
- Given the test file after this requirement lands, when `cargo test --workspace` runs, then `bundle_layout_has_skill_md_per_host` no longer executes and the rest of `skill_packs.rs` continues to pass.
- Given any wrapper template under `resources/agents/<host>/skills/<verb>/SKILL.md.tmpl` is deleted or emptied, when `cargo test --workspace` runs, then `shipped_skill_md_frontmatter_shape` (or another adjacent test that reads the wrapper) fails with a meaningful message — confirming the deleted test added no unique coverage.
</behavior>

<scenario id="CHK-003">
Given the speccy workspace at HEAD after this SPEC lands,
when `rg "fn bundle_layout_has_skill_md_per_host" speccy-cli/tests/skill_packs.rs` runs,
then it reports zero matches.
</scenario>

<scenario id="CHK-004">
Given the same workspace state,
when `cargo test --workspace --test skill_packs` runs,
then it exits 0 and the runner output does not contain `bundle_layout_has_skill_md_per_host`.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Constant re-assert test removed from `speccy-core/tests/personas.rs`

The `#[test]` function
`registry_contains_six_personas_in_declared_order` at L21-28, and
the file-local `EXPECTED` constant at L12-19 that only fed it, are
deleted from `speccy-core/tests/personas.rs`. The other two tests in
the file (`registry_default_personas_is_first_four_prefix` and
`registry_personas_are_unique`) remain unchanged; together they
gate the prefix-relation invariant (`ALL[..4]` is the
default-persona set, per SPEC-0007) and the no-duplicates invariant.
Order-of-the-last-two and exact cardinality are no longer gated;
the project accepts that loss in exchange for not maintaining a
hard-coded duplicate of `ALL`.

<done-when>
- `rg "fn registry_contains_six_personas_in_declared_order" speccy-core/tests/personas.rs` returns no matches.
- `rg "const EXPECTED" speccy-core/tests/personas.rs` returns no matches.
- `rg "fn registry_default_personas_is_first_four_prefix" speccy-core/tests/personas.rs` returns exactly one match (the test is preserved).
- `rg "fn registry_personas_are_unique" speccy-core/tests/personas.rs` returns exactly one match (the test is preserved).
</done-when>

<behavior>
- Given the test file after this requirement lands, when `cargo test --workspace` runs, then `registry_contains_six_personas_in_declared_order` no longer executes; `registry_default_personas_is_first_four_prefix` and `registry_personas_are_unique` continue to pass.
- Given a contributor reorders the last two entries of `speccy_core::personas::ALL` (e.g. swaps `"architecture"` and `"docs"`), when `cargo test --workspace` runs, then the suite still passes — confirming the deleted test was the only thing gating that order, and the project accepts that gap.
- Given a contributor adds a seventh persona to `ALL` without updating the prefix-test default list or violating uniqueness, when `cargo test --workspace` runs, then the suite still passes — confirming the cardinality gate is also gone.
</behavior>

<scenario id="CHK-005">
Given the speccy workspace at HEAD after this SPEC lands,
when `rg "fn registry_contains_six_personas_in_declared_order|const EXPECTED" speccy-core/tests/personas.rs` runs,
then it reports zero matches.
</scenario>

<scenario id="CHK-006">
Given the same workspace state,
when `cargo test --workspace --test personas` runs,
then it exits 0, reports `2 passed`, and the runner output does not contain `registry_contains_six_personas_in_declared_order`.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Workspace-wide hygiene suite passes after deletions

After the test-removal requirements above are satisfied, the
standard four-command hygiene suite that gates every speccy commit
(per AGENTS.md "Standard hygiene") passes end-to-end. This
requirement exists separately from the per-file requirements
because clippy and fmt can flag orphan imports, dead-code, or
formatting drift introduced by deletions that the per-file
`<done-when>` greps would miss.

<done-when>
- `cargo test --workspace` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0.
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0.
</done-when>

<behavior>
- Given REQ-001, REQ-002, and REQ-003 have landed, when the four hygiene commands run sequentially, then each exits 0.
- Given a clippy warning surfaces in any of the three touched files due to an orphan import or dead-code, then REQ-004 is not satisfied and the orphan must be cleaned up.
</behavior>

<scenario id="CHK-007">
Given the speccy workspace at HEAD after this SPEC lands,
when the four hygiene commands (`cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check`) run in sequence,
then each exits 0.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
The four prose-pin tests in `init.rs` are deleted outright rather
than replaced with structural lints (e.g. asserting that
`docs/ARCHITECTURE.md` contains a `# Stance: Feedback, Not
Enforcement` heading without pinning the surrounding sentence).
AGENTS.md is explicit that doc-drift concerns belong to social
review; a structural lint that re-asserts an editorial heading is
itself borderline-vacuous (pattern 2). The asymmetry — losing a
doc-drift safety net in exchange for accepting that vacuous tests
don't belong in the suite — is consistent with the project's
"feedback, not enforcement" stance.
</decision>

<decision id="DEC-002">
The file-non-emptiness test in `skill_packs.rs` is deleted without
a replacement structural check (e.g. "wrapper contains at least one
`{% include %}` directive"). Downstream tests in the same file
already render and parse the wrapper bodies; an additional shape
check would duplicate that coverage. If a future bug shows that a
truly-empty-but-readable wrapper template can slip past the existing
render tests, the right fix is to harden the render test, not to
reintroduce a wrapper-non-emptiness pin.
</decision>

<decision id="DEC-003">
The `personas.rs` test #1 is deleted as a whole function, not
reduced to a `assert_eq!(ALL.len(), 6)` cardinality check. AGENTS.md
allows length as a property carveout, but the residual cardinality
check would gate only the count and would itself be a one-line
re-assert of a fact embedded in the source. The two remaining tests
(prefix-relation and uniqueness) already enforce the load-bearing
invariants; absolute cardinality is not load-bearing. If it later
becomes load-bearing, a follow-up SPEC can introduce a real test
for it.
</decision>

## Notes

The verification of the candidate set deliberately rejected several
flagged-but-non-vacuous tests so the SPEC stays surgical:

- `speccy-core/tests/lint_registry.rs::registry_matches_snapshot`
  pins structured registry data (lint codes, severities, ordering)
  via a deterministic `render_snapshot()` serializer. It is not a
  prose pin and is preserved.
- The three `*_xml_roundtrip.rs` tests in `speccy-core/tests/`
  exercise custom parser/renderer implementations with field-level
  equality after parse → render → parse. They are not trivial
  derive-only round-trips and are preserved.
- `personas.rs` tests #2 (`registry_default_personas_is_first_four_prefix`)
  and #3 (`registry_personas_are_unique`) gate real properties
  (prefix-relation and uniqueness) per the AGENTS.md carveouts. They
  are preserved.
- The `bless_snapshot` `#[ignore]`'d test in `lint_registry.rs` is
  tooling for snapshot regeneration, not a test of system behavior.
  It is preserved.

## Changelog

<changelog>
| Date       | Author              | Summary |
|------------|---------------------|---------|
| 2026-05-27 | claude-opus-4-7[1m] | Initial draft. Four requirements: (REQ-001) delete four prose-substring-match tests in `speccy-cli/tests/init.rs` plus the orphan introductory doc comment; (REQ-002) delete file-non-emptiness test `bundle_layout_has_skill_md_per_host` in `speccy-cli/tests/skill_packs.rs` plus its orphan section header; (REQ-003) delete `registry_contains_six_personas_in_declared_order` and the file-local `EXPECTED` constant in `speccy-core/tests/personas.rs`, keeping the prefix and uniqueness tests; (REQ-004) standard four-command hygiene suite passes. Three decisions: DEC-001 (no replacement structural lints for the prose pins — social review owns doc-drift per AGENTS.md), DEC-002 (no replacement wrapper-shape check — downstream render/parse tests already cover the wrapper bodies), DEC-003 (delete the personas constant test whole rather than reducing to a cardinality assert — the remaining tests already gate the load-bearing properties). Scope deliberately rejects flagged-but-non-vacuous candidates documented in `## Notes`: the structured lint-registry snapshot, the custom-parser XML round-trips, the prefix and uniqueness persona tests, and the `bless_snapshot` regeneration tool. No source-prose files are edited. |
</changelog>
