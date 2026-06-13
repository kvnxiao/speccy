---
spec: SPEC-0062
outcome: implemented
generated_at: 2026-06-12T19:20:00Z
---

# REPORT: SPEC-0062 Retire the last hand-rolled tag recognizer — reconcile's recovery offset derives from the canonical scanner

<report spec="SPEC-0062">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-001 added `journal_xml::last_well_formed_offset` — a public helper in
`speccy-core/src/journal_xml/mod.rs` — that reuses the exact
`split_required → collect_code_fence_byte_ranges → ScanConfig → scan_tags`
preamble already present in `journal_xml::parse`, then walks the `RawTag`
stream tracking depth-0 open/close pairing over `JOURNAL_ELEMENT_NAMES` and
recording each depth-0 close's `span.end`. `consistency.rs::detect_journal_drift`
was updated to call this helper; the private `last_well_formed_offset` fn and
its `#[expect(clippy::similar_names)]` annotation were deleted.

CHK-001 (audit): `grep` for hand-rolled tag-scan patterns (`find("<`,
`find('<'`, `format!("<{`) over non-test `speccy-core/src` and `speccy-cli/src`
yields exactly two hits — `vet_xml/serialize.rs:389` (block renderer emitting
`<{element} verdict=...>`, the SPEC-0061 CHK-006 exclusion) and
`journal_common.rs:54` (`format!("<{}>...")` building an error-context string
from an already-parsed `RawTag.name`). Both are tag emitters/diagnostics, not
input recognizers. The `consistency.rs` `find('<')` loop no longer exists.

CHK-002 (well-formed-then-corrupt journal → byte past `</implementer>`): the
preserved integration fixtures in `consistency_detect.rs` and
`speccy-cli/tests/consistency.rs` compute `expected = find("</implementer>") +
len` independently and assert it against the offset `detect_journal_drift`
derives from `scan_tags`. Values unchanged from pre-SPEC behavior.

CHK-003 (missing frontmatter → 0): `last_well_formed_offset_zero_when_frontmatter_missing`
asserts 0 for a frontmatter-less journal that carries a complete
`<implementer>…</implementer>` — the `split_required` short-circuit path.

Note: the implementer correctly recorded `tag.span.end` (byte just past the
close tag's `>`) rather than the task hint's `tag.body_end_after_tag` (which
for a close tag equals the tag *start*). `span.end` is the correct truncation
point per the preserved fixtures' contract formula. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004">
T-002 added the REQ-002 end-to-end regression test
`detect_journal_xml_malformed_recovery_offset_ignores_fenced_close` in
`speccy-core/tests/consistency_detect.rs`. The fixture carries valid frontmatter,
a well-formed round-1 `<implementer>` block whose structural close ends at byte
X=153, and a round-2 `<implementer>` open whose only following
`</implementer>` sits inside a fenced code block (ending at Y=235) with no real
structural close — causing `journal_xml::parse` to fail and the malformed branch
to run. The assertion `last_well_formed_byte_offset == X` (153) passes after the
fence-aware `scan_tags` read path. A one-shot pre-fix measurement resurrecting the
deleted hand-rolled `find('<')` scan against the same fixture yielded Y=235 —
confirming the fence-blindness bug the test guards, with both values recorded
inline in the test doc comment. Retry count: 0.
</coverage>

</report>
