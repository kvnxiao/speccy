---
spec: SPEC-0019
outcome: delivered
generated_at: 2026-05-15T22:30:00Z
---

# SPEC-0019: Canonical marker-structured SPEC.md; remove spec.toml

## Outcome

**delivered** — all five requirements satisfied. SPEC.md is now the
single canonical carrier for requirements, scenarios, decisions, and
changelog history, via line-isolated `<!-- speccy:NAME -->` marker
comments. Per-spec `spec.toml` files are gone from every in-tree
spec, the workspace loader rejects stray reintroductions, and
`speccy check`, `speccy verify`, and prompt slicing all read the
typed `SpecDoc` produced by the marker parser. Architecture docs and
shipped skill packs teach the marker form; the ephemeral
`xtask/migrate-spec-markers-0019` tool was deleted before the ship
commit.

<report spec="SPEC-0019">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-005">
</coverage>

</report>

## Task summary

- **Total tasks:** 7 (T-001 through T-007).
- **Retried tasks:** 3.
  - T-003 retried once (in-fence marker example misclassification
    fix; led to the `is_fence_marker` branch and the `--force` flag
    in the migration tool).
  - T-005 retried once (loader/types doc-comment drift after the
    `[[checks]]`-table removal; doc-only rewrite with no behaviour
    change).
  - T-007 retried once (reviewer-tests blocking: the fifth
    "Tests to write" bullet — DEC-003's no-public-`speccy fmt`
    contract — was unpinned; added
    `architecture_md_pins_no_public_speccy_fmt_per_dec_003` in
    `speccy-core/tests/docs_sweep.rs`).
- **SPEC amendments:** 0.

T-001 landed the strict marker scanner and `SpecDoc` model.
T-002 added the deterministic renderer and the parse/render/parse
roundtrip property. T-003 built `xtask/migrate-spec-markers-0019`
and exercised it against one fixture and one real spec. T-004 ran
the migration across every in-tree SPEC.md, added the corpus-level
parse test, and pinned the post-migration `speccy verify` exit-0
contract. T-005 deleted the `spec.toml` types and switched the
workspace loader to `SpecDoc`. T-006 routed `speccy check`,
`speccy verify`, and prompt slicing through the typed `SpecDoc`.
T-007 swept architecture docs and shipped skills, deleted the
migration tool, and added the docs-sweep regression test.

## Out-of-scope items absorbed

- T-003 grew an `is_fence_marker` branch and a `--force` flag in
  the migration tool to handle SPEC-0019/0020-style specs whose
  fenced code blocks contained illustrative marker comments. The
  fix landed inside the tool itself (per the AGENTS.md
  friction-to-skill-update convention applied to internal tooling),
  not as a SPEC amendment.
- T-005's `WorkspaceError::StraySpecToml` was relocated to
  `ParseError::StraySpecToml` so the per-spec stray-toml diagnostic
  flows through the same `ParsedSpec.spec_doc` channel as other
  parse errors. The behavioural contract is unchanged (workspace
  loading still fails and names the stray path); the variant moved
  to keep the error taxonomy single-axis.

## Skill updates

- `resources/modules/skills/speccy-plan.md`, `resources/modules/skills/speccy-amend.md`,
  `.claude/skills/speccy-plan/SKILL.md`, `.claude/skills/speccy-amend/SKILL.md`,
  `.agents/skills/speccy-plan/SKILL.md`, `.agents/skills/speccy-amend/SKILL.md` —
  T-007 sweep: rewrote the planning/amendment skill instructions
  to point at SPEC.md marker blocks instead of per-spec
  `spec.toml`. Friction surfaced by T-007 itself (the task's
  contract was to drive this sweep), not by an in-flight
  friction-fix during another task.
- `resources/modules/personas/{implementer,planner,reviewer-tests}.md`,
  `resources/modules/prompts/{implementer,plan-amend,plan-greenfield,report}.md`,
  `.speccy/skills/personas/{implementer,planner,reviewer-tests}.md`,
  `.speccy/skills/prompts/{implementer,plan-amend,plan-greenfield,report}.md`,
  `.codex/agents/reviewer-tests.toml`, `.claude/agents/reviewer-tests.md` —
  T-007 sweep: shipped persona prompts and prompt modules now
  describe scenarios as nested `<!-- speccy:scenario -->` marker
  blocks under their parent `<!-- speccy:requirement -->`, and
  drop the prior `spec.toml`/`[[checks]]`-table phrasing.
- `xtask/migrate-spec-markers-0019/src/lib.rs` (deleted in T-007) —
  T-003 added the `is_fence_marker` branch and `--force` flag
  in-flight when the tool produced wrong output on a real spec.
  The fix lived in the tool itself per the friction-to-skill-update
  rule applied to internal tooling; the tool was then deleted as
  planned once migration was complete.

## Deferred / known limitations

- **Open questions left open by design.** SPEC.md's two open
  questions are intentionally not resolved by this spec: whether
  the root `<!-- speccy:spec -->` marker should be required (vs
  emitted-only) and whether decision markers should be required for
  every `DEC-NNN` block. Both are observable-after-dogfooding
  decisions and deferred to a follow-up; documenting the
  current-as-shipped contract in REQ-001's "Done when" was the
  reviewer-business-blessed compromise.
- **`docs_sweep` walks fewer mirrors than its docstring suggests.**
  Style review on T-007 flagged that
  `shipped_skills_do_not_instruct_editing_per_spec_spec_toml` (now
  covering `resources/modules/`, `.claude/skills/`, `.agents/skills/`,
  `.codex/agents/`, `.speccy/skills/`) was originally narrower than
  its docstring claimed. The test now walks every render target,
  but the unnecessary `path.clone()` at `docs_sweep.rs:103` and
  the docstring's slightly stale wording were left as a follow-up
  cleanup nit, not in-scope for the SPEC-0019 ship.
- **`in_tree_specs::every_in_tree_spec_md_parses_with_marker_parser`
  is brittle against future-carrier drafts.** The test asserts
  every `.speccy/specs/*/SPEC.md` parses with the SPEC-0019 marker
  parser. SPEC-0020 (raw XML element tags for SPEC.md) and its
  follow-on SPEC-0022 (raw XML for TASKS.md and REPORT.md) are
  already drafted in the new carrier form and will fail this test
  until the SPEC-0020 implementation lands the XML element parser.
  Tightening the test to skip in-progress specs lacking marker
  content is out of scope for SPEC-0019; SPEC-0020 will replace
  this test alongside the carrier swap.
