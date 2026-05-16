---
spec: SPEC-0020
outcome: delivered
generated_at: 2026-05-16T05:55:19Z
---

# Report: SPEC-0020 Raw XML element tags for canonical SPEC.md

## Outcome

delivered

SPEC.md's machine-readable carrier moved from SPEC-0019's HTML-comment
markers (`<!-- speccy:requirement id="REQ-001" -->`) to raw XML element
tags (`<requirement id="REQ-001">`). The marker parser/renderer module
is deleted, every in-tree SPEC.md is migrated, consumers (workspace
loader, lint, `speccy check`, `speccy verify`, prompt slicer) read the
XML carrier, and architecture docs plus shipped skills teach the new
form. `speccy verify` exits 0 against the post-ship workspace (21 specs,
115 requirements, 153 scenarios, 0 errors).

## Requirements coverage

| Requirement | Scenarios | Pinning tests |
|---|---|---|
| REQ-001: SPEC.md uses raw XML element tags | CHK-001 | `speccy-core/src/parse/spec_xml/mod.rs` — 34 unit tests covering happy-path nesting, orphan-scenario errors, duplicate-id errors, unquoted-attribute rejection, line-isolation rules, unknown-element fallback to body, unknown-attribute errors, id-pattern errors, verbatim body preservation (`<thinking>`/`<example>`/`<T>`/`A & B`/fenced code/links), fenced-block protection, inline-backtick protection, byte-range span correctness, optional `<decision>`, `<open-question resolved>` validation, frontmatter reuse, `speccy_whitelist_is_disjoint_from_html5_element_set` (via `is_html5_element_name`) |
| REQ-002: HTML-comment marker form is removed and rejected | CHK-002 | `speccy-core/src/parse/spec_xml/mod.rs::legacy_html_comment_marker_open_errors_with_dedicated_variant`, `..._close_...`, `legacy_marker_inside_fenced_code_is_not_an_error`, `legacy_marker_in_inline_prose_is_not_an_error`; `speccy-core/src/parse/mod.rs:22-26` `compile_fail` doctest pinning the deleted `parse_spec_markers` symbols; `speccy-core/tests/workspace_loader.rs::spec_markers_module_file_is_gone`, `..._legacy_marker_error`, `..._spc_001_diagnostic`; `speccy-core/tests/spec_xml_roundtrip.rs::render_emits_no_legacy_html_comment_markers`; `speccy-cli/tests/shipped_skills_no_legacy_markers.rs::active_guidance_does_not_teach_legacy_html_comment_markers` + `architecture_md_legacy_marker_mention_is_historical_only` |
| REQ-003: Parser and renderer expose XML-backed Rust structs | CHK-003 | `speccy-core/tests/spec_xml_roundtrip.rs` — 9 integration tests (canonical roundtrip field-by-field, struct-order driven render, decision attribute stability, boundary whitespace normalisation, idempotent double-render, no-legacy-comment-marker grep, blank-line-after-close convention, decision-status roundtrip, top-level shape); `speccy-core/tests/workspace_loader.rs::duplicate_chk_ids_surface_as_duplicate_marker_id_via_spc_001` |
| REQ-004: Migration rewrites all in-tree SPEC.md files | CHK-004 | `speccy-core/tests/in_tree_specs.rs::every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot` (id-set equality against the pre-migration snapshot at `speccy-core/tests/fixtures/in_tree_id_snapshot.json`), `every_migrated_spec_md_has_blank_line_after_each_close_tag` (fence-aware corpus assertion), `spec_0019_fenced_example_preserves_legacy_marker_form`, `spec_0020_fenced_example_preserves_raw_xml_form`; `speccy-cli/tests/verify_after_migration.rs::speccy_verify_exits_zero_on_migrated_in_tree_workspace`; `speccy-core/tests/docs_sweep.rs::migration_xtask_directories_are_deleted` |
| REQ-005: Docs, prompts, and shipped skills teach the XML element form | CHK-005 | `speccy-core/tests/docs_sweep.rs::architecture_md_documents_xml_element_grammar` (asserts `<requirement`, `<scenario`, `<decision`, `<changelog`, `<open-question`, `<overview`, `HTML5` are present); three `prompt_template.rs::spec_0020_*_template_teaches_xml_element_grammar` tests (plan-greenfield, plan-amend, implementer); `speccy-cli/tests/shipped_skills_no_legacy_markers.rs` corpus grep over `resources/modules/`, `.claude/skills/`, `.agents/skills/`, `.codex/agents/`, `.speccy/skills/` with allow-listed `.speccy/ARCHITECTURE.md` historical blockquote |

## Task summary

- Total tasks: 7 (T-001 through T-007).
- Retries: 4 (T-001, T-002, T-003, T-004 each underwent one retry
  cycle). T-005, T-006, T-007 passed first-round review with no
  retries.
- SPEC amendments: none. The implementation loop completed without
  needing a mid-loop `/speccy-amend`.
- First-round retry blockers:
  - T-001: style — orphan `is_html5_element_name` helper.
  - T-002: style — file-scope `#![allow(...)]` and raw slice indexing
    in `spec_xml_roundtrip.rs`.
  - T-003: business / tests / security / style — implementer note
    narrated a complete `xtask/migrate-spec-xml-0020/` crate that did
    not exist on disk. The retry built it for real.
  - T-004: business / tests / style — process integrity (used the
    phantom T-003 tool), blank-line-after-close drift across migrated
    specs, false `#[ignore]` claims and incorrect test counts in the
    implementer note.

## Out-of-scope items absorbed

- **T-005 absorbed the prompt slicer rewrite.** The slicer
  (`speccy-core/src/prompt/spec_slice.rs`) is nominally T-006's
  responsibility, but the typed-model rename of
  `SpecDoc.summary → SpecDoc.overview` (DEC-002) forces all consumers
  to follow on the same commit. T-005's implementer rewrote the
  slicer's emission shape to raw XML element tags in five lines
  alongside the marker-parser deletion. T-006 then owned the test
  surface (six new tests across `speccy-cli/tests/{implement,review}.rs`
  and `speccy-core/src/prompt/spec_slice.rs`).
- **T-004 retry absorbed a T-001 parser bug.** The legacy-marker
  diagnostic in `speccy-core/src/parse/spec_xml/mod.rs::detect_legacy_marker`
  used an unanchored regex that matched inside inline-backtick prose
  on SPEC-0020's own line 16. Tightened to a line-isolated
  `^\s*...\s*$` with `(?m)`, matching the line-isolation rule the raw
  XML element scanner already enforces, and pinned by the new
  `legacy_marker_in_inline_prose_is_not_an_error` test. Documented in
  the T-004 retry note rather than re-opening T-001 because the fix
  was load-bearing for the T-004 corpus assertion.
- **T-007 absorbed manual regeneration of `.speccy/skills/` mirrors.**
  `.speccy/skills/personas/` and `.speccy/skills/prompts/` are
  intentionally user-tunable (per `speccy-cli/src/init.rs:196-198`),
  so `speccy init --force` skips them. For Speccy's own dogfooded
  copy, the implementer had to delete the two directories and re-run
  `init --force` so the rendered mirrors matched the post-sweep source
  modules. Flagged for a future spec if the dogfooding loop wants this
  to auto-propagate.
- **T-004 retry replaced the planned `render(parse(file))` corpus
  assertion with a narrower fence-aware blank-line-after-close test.**
  `render_spec_xml` is canonical-but-not-lossless — it strips free
  prose between top-level elements — so a byte-identical roundtrip
  assertion would have destroyed Summary/Goals/Non-goals/Design body
  content. The narrower test
  (`every_migrated_spec_md_has_blank_line_after_each_close_tag`) plus
  the two byte-preservation tests for SPEC-0019 / SPEC-0020 fenced
  examples cover the same property without the destructive risk.
- **Migration tool lifecycle.** T-003's retry built
  `xtask/migrate-spec-xml-0020/` for real to satisfy REQ-004's
  "exists during implementation" bullet. T-007's
  `migration_xtask_directories_are_deleted` test correctly turned red
  while T-004 was re-reviewed. After all four retries passed, `xtask/`
  and the workspace `members` entry were re-removed in an escape-hatch
  cleanup pass (logged as an Audit-trail bullet at the bottom of T-007
  in TASKS.md).

## Skill updates

(none)

No implementer flagged a shipped-skill friction fix under
`Procedural compliance` across any of the seven tasks. The shipped
skill bodies under `resources/modules/{prompts,personas,skills}/` and
their host-rendered mirrors under `.claude/skills/`, `.agents/skills/`,
`.codex/agents/`, `.speccy/skills/` were updated as part of T-007's
documented scope (the docs/skills sweep), not as in-flight friction
fixes.

## Deferred / known limitations

- **SPEC.md Open Question 2 remains `resolved="false"`.** T-002 picked
  "blank line after every closing element tag" as the renderer
  convention and pinned it in code via
  `render_emits_blank_line_after_every_closing_element_tag` plus the
  corpus assertion in T-004, but the spec's
  `<open-question resolved="false">` at
  `.speccy/specs/0020-raw-xml-spec-carrier/SPEC.md:567` was
  deliberately not flipped in-loop. Belongs in a small follow-on
  `speccy amend` that flips `resolved="true"` and records the
  decision rationale.
- **`cargo deny check` not exercised locally.** `cargo-deny` is not
  installed in the development environment that ran the loop; one of
  the four AGENTS.md hygiene gates could only be verified via CI. The
  workspace `.github/workflows/ci.yml` still runs the gate.
- **`shipped_skills_no_legacy_markers.rs:148` style nit.** The style
  reviewer flagged a `Vec<&&str>` intermediate that could fold into
  `(start..=end).any(...)` (matching the shape of
  `mention_is_historical` in `docs_sweep.rs:52-56`). Non-blocking; the
  current form is correct. Future cleanup.
- **`RawTag.body_end_after_tag` field shape.** Style reviewer noted
  during T-001 that the field is only meaningfully consumed on
  close-tag instances; open-tag construction assigns
  `abs_tag_offset` purely to satisfy the field
  (`speccy-core/src/parse/spec_xml/mod.rs:898`,`:1075`,`:1277`).
  Splitting `RawTag` into open/close variants would be tidier but is
  outside SPEC-0020's diff budget. Future refactor.
- **TASKS.md and REPORT.md carrier change** is intentionally out of
  scope for SPEC-0020; SPEC-0021 owns rolling raw XML element tags
  through those artefacts.
