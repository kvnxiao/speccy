---
spec: SPEC-0021
outcome: delivered
generated_at: 2026-05-16T07:00:00Z
---

## Outcome

delivered

## Requirements coverage

- **REQ-001 — SPEC.md element whitelist evolves: six additions, two
  retirements.** Proved by CHK-001 (canonical-order parse, missing
  `<behavior>`, duplicate `<goals>`, retired-tag rejection,
  optional-`<assumptions>` absence, fenced-code preservation).
  Backed in `speccy-core/src/parse/spec_xml/mod.rs` and the unit /
  fixture tests under `speccy-core/tests/fixtures/spec_xml/` plus
  the HTML5 disjointness assertion in `parse/spec_xml/mod.rs`'s
  whitelist module.
- **REQ-002 — Typed SpecDoc surface extends without renaming.**
  Proved by CHK-002 (parse → render → parse round-trip,
  in-requirement element order, omitted-`<assumptions>` render,
  canonical top-level order without `<spec>` / `<overview>`).
  Backed by `speccy-core/tests/spec_xml_roundtrip.rs` and the
  expanded `tests/fixtures/spec_xml/canonical.md` fixture.
- **REQ-003 — Migration rewrites every in-tree SPEC.md.** Proved by
  CHK-003 (post-SPEC-0020 SPEC.md migrates intent surfaces while
  preserving headings/frontmatter; a requirement without `Done when`
  or `Behavior` prose fails closed; `speccy verify` exits zero across
  the migrated workspace). Backed by the in-flight
  `xtask/migrate-spec-sections-0021` tool (since deleted per T-006),
  the in-tree corpus diff visible in this PR's `.speccy/specs/*/SPEC.md`
  rewrites, and `speccy-core/tests/in_tree_specs.rs`.
- **REQ-004 — Docs, prompts, and shipped skills cite the new tags.**
  Proved by CHK-004 (implementer prompt cites `<behavior>` /
  `<done-when>`; reviewer-tests prompt cites `<behavior>` /
  `<scenario>`; ARCHITECTURE.md SPEC.md grammar lists the six new
  rows and omits the two retired rows; the post-spec SPEC.md template
  uses the new tags). Backed by `resources/modules/personas/*.md`,
  `resources/modules/prompts/implementer.md`, and the updated
  `.speccy/ARCHITECTURE.md` grammar table.
- **REQ-005 — Sequence enables SPEC-0022 to reuse the wider
  whitelist.** Proved by CHK-005 (SPEC-0021 ships first; the HTML5
  disjointness unit test centralises the whitelist so SPEC-0022's
  TASKS.md / REPORT.md element names extend the same assertion).
  Backed by the SPEC-0022 placeholder spec at
  `.speccy/specs/0022-xml-canonical-tasks-report/SPEC.md` and the
  whitelist module in `speccy-core/src/parse/spec_xml/mod.rs`.

## Task summary

- 6 tasks, all `[x]`. No retries.
- No SPEC amendments triggered during implementation.
- The TASKS.md `Covers:` lines initially carried `(parse half)` /
  `(render half)` / `(cleanup)` parentheticals; these were stripped
  during /speccy-ship so `TSK-001` would not flag the IDs as
  undeclared REQs. No behavior change.

## Out-of-scope items absorbed

- (none)

## Skill updates

- (none)

## Deferred / known limitations

- SPEC-0022 (canonical XML tags for TASKS.md and REPORT.md) reuses
  the wider whitelist established here; it is intentionally a
  follow-up spec rather than an in-scope extension.
- `<spec>` / `<overview>` are removed from the whitelist; any
  externally authored SPEC.md still using them must migrate before
  parsing.
