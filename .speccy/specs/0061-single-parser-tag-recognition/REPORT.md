---
spec: SPEC-0061
outcome: implemented
generated_at: 2026-06-12T09:00:00Z
---

# REPORT: SPEC-0061 Single parser authority for tag recognition — gate-read and journal-write paths use the canonical parser, deleting both hand-rolled scanners

<report spec="SPEC-0061">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-002 rewrote `vet_gate_is_fresh_pass` in `speccy-core/src/next.rs` to derive the
terminal gate from `parse_vet_in_flight`'s typed `VetDoc`
(`doc.invocations.last().and_then(|inv| inv.blocks.last())` matched as
`VetBlock::Gate { verdict, tasks_hash, .. }`). Parse failure, empty document, and a
non-Gate terminal block all return false. CHK-001 (spoof: failed terminal gate with an
inline passing gate in its body — the old byte-scanner yielded Ship; the typed path
yields Vet) is covered by
`vet_when_terminal_gate_failed_despite_inline_passing_gate` in
`speccy-core/tests/next_priority.rs`. CHK-002 (fresh passing terminal gate, REPORT.md
absent → Ship) is covered by `ship_when_vet_passes_fresh_and_no_report`. CHK-003
(present-but-unparseable VET.md, missing frontmatter → Vet) is covered by the
round-2-added `vet_when_vet_md_present_but_unparseable`, which uses a gate whose
`tasks_hash` matches the on-disk TASKS.md so only the parse-failure branch at
`next.rs:185` prevents a false Ship — a genuine gate on that branch. Retry count: 1
(round 2 added the CHK-003 dedicated fixture after the tests persona flagged the
parse-failure branch as ungated).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005">
T-003 added `JournalError::ProducedJournalUnparseable { path, source }` (mirroring
`ProducedVetUnparseable`) and inserted a strict `parse_journal_xml` round-trip in
`append_under_lock` (`speccy-cli/src/journal.rs:537`) immediately before
`fs_err::write`, returning the new variant on failure before any byte is written. CHK-004
(line-isolated `</implementer>` in body refused, journal byte-identical, stderr contains
"unparseable") is covered by `line_isolated_close_tag_in_body_is_rejected_at_write_time`
in `speccy-cli/tests/journal_append.rs`. CHK-005 (inline element-name prose accepted,
journal re-parses) is covered by `inline_element_mention_in_prose_is_accepted_and_reparses`.
The `ProducedJournalUnparseable` Display ("...would make the journal...unparseable...")
is distinct from `ExistingJournalUnparseable` ("failed to parse"), so the
`stderr(contains("unparseable"))` assertion in CHK-004 discriminates the correct variant.
Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-006">
T-002 deleted `last_gate_block`, `GateBlock`, and the `next.rs` `attribute_value`
helper (plus their `#[cfg(test)]` unit tests and the stale `vet_xml/mod.rs` module-doc
claim). T-003 deleted `first_nested_journal_element`, its call site in
`validate_and_render_block`, `SerializeError::NestedJournalMarkup`, and two orphaned
unit tests that exercised only the removed pre-scan; the `JOURNAL_ELEMENT_NAMES` import
in `serialize.rs` was also dropped (still used at its remaining call sites in `xml.rs`).

CHK-006 audit — grep over non-test `speccy-core/src` and `speccy-cli/src` for
hand-rolled tag-recognizer patterns (`find("<`, `find("</`, `format!("<{`):

- `vet_xml/serialize.rs:389`: a block *renderer* emitting a tag via `format!("<{element}
  …")` — the canonical output path, explicitly carved out by REQ-003's done-when.
- `journal_common.rs:54`: a `format!` building a diagnostic context string — emits, does
  not scan input; not a recognizer.
- `consistency.rs:552`: inside `#[cfg(test)] mod tests` — test code, outside the
  non-test scope.

`consistency.rs::last_well_formed_offset` (line 459) uses single-char `find('<')` plus
name matching as a tolerant post-parse-failure recovery scan for reconcile truncation. It
is deliberately out of scope: it runs only after the strict parser has already rejected
the source (makes no recognition decision on valid input), and retiring it would require
exposing a "last well-formed offset on parse failure" parser primitive that DEC-003
disfavors. This carve-out is explicit in SPEC-0061 DEC-001.

No hand-rolled tag recognizer remains on the gate-read or journal-write production paths.
Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
T-001 added a renderer-backed VET.md helper to each test crate:
`speccy-core/tests/next_priority.rs` (covers `next_priority.rs` itself) and
`speccy-cli/tests/common/mod.rs` (covers the CLI integration tests). Both are built on the
exported production renderers (`render_fresh_vet_frontmatter`,
`render_vet_section_heading`, `validate_and_render_vet_block`) and take a `(verdict,
tasks_hash)` pair, guaranteeing every produced document is accepted by
`parse_vet_in_flight` by construction. CHK-007 (helper output round-trips through the
parser) is demonstrated by `chk007_core_helper_output_round_trips_through_parser` in
`next_priority.rs` and the equivalent in `common/mod.rs`.

CHK-008 audit — valid hand-rolled VET.md migration set: `next_text.rs`, `next_json.rs`,
`next_derived.rs`, `common/mod.rs` (the previously hand-rolled fixtures), and the
valid-but-hand-rolled fixture in `journal_show.rs` — all routed through the
renderer-backed helper. Legitimately-excluded fixtures not routed through the helper:
`lint_vet.rs` (builds deliberately-malformed VET.md to drive the `VET-*` lint family —
missing frontmatter, out-of-domain verdict, gate-ordering violations — none of which a
valid-only helper can produce; per DEC-005) and the intentionally gate-less fixture in
`journal_show.rs` (exercises the parse/lint boundary by design). No valid
`## Invocation N` / `<gate>` string remains outside a renderer-backed helper except
these DEC-005 carve-outs. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-009">
The gate-spoof regression test
`vet_when_terminal_gate_failed_despite_inline_passing_gate`
(`speccy-core/tests/next_priority.rs`) uses a valid VET.md built from production
renderers whose terminal gate is `failed` and whose body embeds an inline
`<gate verdict="passed">` carrying a `tasks_hash` matching the fixture's TASKS.md. Only
consulting the terminal (failed) gate — not the inline quoted one — yields Vet. The
pre-fix Ship observation is recorded in `journal/T-002.md` round-1 implementer handoff:
running the test against the old byte-scanner produced "expected Vet, got Ship", proving
the live bug. Post-fix the test passes asserting Vet. Retry count: 0.
</coverage>

</report>
