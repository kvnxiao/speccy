---
spec: SPEC-0050
outcome: implemented
generated_at: 2026-05-27T00:00:00Z
---

# REPORT: SPEC-0050 Remove vacuous tests — delete 6 tests that gate editorial prose, file non-emptiness, or constant copies rather than behavior

<report spec="SPEC-0050">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 deleted the four prose-substring-match `#[test]` functions from
`speccy-cli/tests/init.rs` (`reviewer_tests_persona_pins_no_check_exit_code_evidence`,
`architecture_doc_pins_feedback_not_enforcement_contract`,
`architecture_doc_pins_check_command_is_render_only`,
`architecture_doc_pins_verify_command_is_shape_only`) along with the
orphan `/// Positive content pins.` doc comment block that preceded
them. The `include_str!` references to `docs/ARCHITECTURE.md` and
`resources/modules/personas/reviewer-tests.md` were scoped inside
each deleted function body and were removed with them. `cargo test
--workspace` exits 0; none of the four named functions appear in the
runner output. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-001 deleted `bundle_layout_has_skill_md_per_host` from
`speccy-cli/tests/skill_packs.rs` together with the orphan
`// Bundle layout: per-host SKILL.md.tmpl wrappers.` section-header
comment block. The remaining tests in the file (`shipped_skill_md_frontmatter_shape`
and others) continue to gate file existence and wrapper content shape
with meaningful asserts. `cargo test --workspace --test skill_packs`
exits 0. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
T-001 deleted `registry_contains_six_personas_in_declared_order` and
the file-local `const EXPECTED: &[&str]` array from
`speccy-core/tests/personas.rs`. The two remaining tests
(`registry_default_personas_is_first_four_prefix` and
`registry_personas_are_unique`) were preserved unchanged and gate the
load-bearing prefix-relation and uniqueness invariants respectively.
`cargo test --workspace --test personas` exits 0 and reports 2
passed. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007">
All four hygiene commands passed after the deletions: `cargo test
--workspace`, `cargo clippy --workspace --all-targets --all-features
-- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny
check` each exit 0. No orphan imports, dead-code warnings, or
formatting drift were introduced by the deletions. Retry count: 0.
</coverage>

</report>
