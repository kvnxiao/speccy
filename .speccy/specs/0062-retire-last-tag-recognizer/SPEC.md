---
id: SPEC-0062
slug: retire-last-tag-recognizer
title: Retire the last hand-rolled tag recognizer — reconcile's recovery offset derives from the canonical scanner
status: implemented
created: 2026-06-12
supersedes: []
---

# SPEC-0062: Retire the last hand-rolled tag recognizer — reconcile's recovery offset derives from the canonical scanner

## Summary

SPEC-0061 (merged, #26) collapsed the gate-read and journal-write paths onto the
canonical `xml_scanner`-backed parsers and deleted both hand-rolled tag scanners
on those paths. It deliberately left one out of scope: `consistency.rs::last_well_formed_offset`.

That function is the last surviving hand-rolled tag recognizer in production. It
walks `source.find('<')` plus name-prefix matching for the journal element names
(`implementer`, `review`, `blockers`) over a per-task journal, returning the byte
offset just after the last depth-0 close tag (0 when none closed cleanly). It
runs only *after* the strict journal parser (`journal_xml::parse`) has already
rejected a per-task journal — inside `detect_journal_drift` — to compute the
`DriftDetails::JournalXmlMalformed { last_well_formed_byte_offset }` that the
reconcile policy truncates the corrupt journal to.

It carries the **same fence-blindness divergence class** SPEC-0061 killed in the
other two scanners: a line-isolated journal close tag (e.g. `</implementer>`)
sitting inside a fenced code block in a block body is miscounted as a real
structural close, yielding a wrong truncation offset. The blast radius is low —
it runs only on an already-corrupt journal during reconcile, and a wrong offset
degrades recovery quality rather than spoofing a ship or corrupting a good file —
but it is a real divergence bug, not a hypothetical one.

The fix reuses what already exists. The canonical recognizer `xml_scanner::scan_tags`
is already public and already performs fence-aware, line-isolated recognition,
returning `Vec<RawTag>` (each `RawTag` carries an `ElementSpan { start, end }` and
a `body_end_after_tag` offset). It errors only on byte-arithmetic overflow
(impossible for an in-memory string) and succeeds *independently* of the
structural/attribute validation that makes `journal_xml::parse` fail — so its
token stream is available exactly when reconcile needs it. The recovery offset is
re-derived by walking that token stream (depth-0 open/close pairing over
`JOURNAL_ELEMENT_NAMES`, taking the end of the last depth-0 close) instead of the
hand-rolled `find('<')` loop. After this SPEC, `scan_tags` is the sole tag
recognizer in production, with two consumers — the structural parser and this
recovery helper — which is the single-authority end state DEC-001 (SPEC-0055/0061)
named.

## Goals

<goals>
- The reconcile recovery offset for a malformed per-task journal is computed by
  reusing the canonical `scan_tags` recognizer; the hand-rolled `find('<')` scan
  in `consistency.rs` no longer exists.
- No hand-rolled tag recognizer (a `find("<…")` / `find('<')` / `format!("<{…")`
  scan over input text) remains in non-test `speccy-core` or `speccy-cli`
  source; block renderers that emit tags are the canonical output path and stay
  in scope's exclusion, as in SPEC-0061 CHK-006.
- For a journal that is well-formed up to a trailing corruption (no fenced
  close-tag confusion), the computed `last_well_formed_byte_offset` is unchanged
  from today's behavior.
- A line-isolated journal close tag inside a fenced code block is not counted as
  a structural close when computing the recovery offset.
</goals>

## Non-goals

<non-goals>
- No change to the reconcile policy or truncation semantics: reconcile still
  truncates the corrupt journal to `last_well_formed_byte_offset` bytes.
- No change to the `journal_xml_malformed` drift kind, the `DriftDetails` shape,
  or the `last_well_formed_byte_offset` field name or meaning.
- No new public parser primitive. Recognition stays inside the existing
  canonical `scan_tags`; a standalone last-well-formed-offset recognizer is
  explicitly not introduced (this is the alternative SPEC-0061 DEC-003 rejected).
- No change to the per-task journal or VET.md grammar; only which code path
  computes the recovery offset.
- No edit to the merged SPEC-0061; its DEC-001 rationale is corrected in this
  SPEC's live record (see Notes), not rewritten in place.
</non-goals>

## User Stories

<user-stories>
- As a maintainer, I want exactly one implementation of tag recognition in
  production, so the fence-blindness divergence class cannot recur in the one
  place SPEC-0061 left it.
- As an agent recovering a crashed session via reconcile, I want the truncation
  offset for a corrupt journal to respect fenced code blocks, so a stray close
  tag quoted inside a block body cannot drive recovery to a wrong byte and
  discard or retain the wrong content.
</user-stories>

## Assumptions

<assumptions>
- `xml_scanner::scan_tags`, `RawTag`, `ScanConfig`, `JOURNAL_ELEMENT_NAMES`,
  `split_required`, and `collect_code_fence_byte_ranges` are reachable from a
  helper homed in the `journal_xml` parse module — `journal_xml::parse` already
  composes all of them at the same call site.
- `scan_tags` succeeds on any in-memory journal source regardless of whether the
  higher-level structural parse fails, so the recovery helper can rely on its
  token stream being available precisely in the post-parse-failure path where
  the offset is needed.
- When the journal is corrupt because its frontmatter is missing (so
  `split_required` fails before any scan), no structural tag closed cleanly and
  the recovery offset is legitimately 0 — matching the current scanner's "return
  0 when nothing closed" behavior.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: The reconcile recovery offset derives from the canonical scanner

The recovery offset for a malformed per-task journal is computed by walking the
`RawTag` stream that `xml_scanner::scan_tags` produces — tracking depth-0
open/close pairing over `JOURNAL_ELEMENT_NAMES` and returning the trailing byte
offset of the last depth-0 close (0 when none closed cleanly) — rather than the
hand-rolled `source.find('<')` scan. The hand-rolled scan is deleted. The offset
contract `detect_journal_drift` depends on is preserved: for a journal that is
well-formed up to a trailing corruption, the offset is unchanged from today's
value.

<done-when>
- `consistency.rs` contains no `find('<')`-style tag scan; the recovery offset is
  produced by a helper that reuses `scan_tags` (homed in the `journal_xml`
  module per DEC-001).
- A reviewer auditing non-test `speccy-core` / `speccy-cli` source for
  hand-rolled tag-scan patterns (`find("<`, `find('<'`, `format!("<{`) finds only
  block renderers that emit tags — no recognizer scanning input remains — and the
  audit is recorded in REPORT.md.
- The existing offset tests stay green unchanged in their expected values:
  `last_well_formed_offset_finds_close_of_implementer` and
  `last_well_formed_offset_zero_when_no_close_tag` in `consistency.rs`, the
  `journal_xml_malformed` fixture in `speccy-core/tests/consistency_detect.rs`
  (SPEC-0045 CHK-013), and the `details.last_well_formed_byte_offset` assertion in
  `speccy-cli/tests/consistency.rs`.
</done-when>

<behavior>
- Given a per-task journal that is well-formed through one or more top-level
  elements and then corrupt, when `detect_journal_drift` computes the recovery
  offset, then the offset is the end of the last well-formed depth-0 close tag —
  the same byte value the hand-rolled scan produced for that input.
- Given a corrupt journal whose frontmatter is missing, when the recovery offset
  is computed, then it is 0.
</behavior>

<scenario id="CHK-001">
Given the post-SPEC `speccy-core` source,
when a reviewer audits non-test code for hand-rolled tag-scan patterns
(`find("<`, `find('<'`, `format!("<{`),
then the only matches are block renderers that emit tags (not recognizers
scanning input), `consistency.rs::last_well_formed_offset`'s hand-rolled
`find('<')` loop is gone, and the audit is recorded in REPORT.md.
</scenario>

<scenario id="CHK-002">
Given a per-task journal with a well-formed `<implementer>…</implementer>` block
followed by trailing bytes that make `journal_xml::parse` fail,
when `detect_journal_drift` computes `last_well_formed_byte_offset`,
then the value equals the byte offset just past the `</implementer>` close tag —
identical to the value the pre-SPEC hand-rolled scan returned for the same input
(verified by the preserved `consistency_detect.rs` fixture).
</scenario>

<scenario id="CHK-003">
Given a corrupt per-task journal that is missing its YAML frontmatter (so
`split_required` rejects it before any tag scan),
when `detect_journal_drift` computes `last_well_formed_byte_offset`,
then the value is 0.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Fenced journal close tags are excluded from the recovery offset

A line-isolated journal close tag (e.g. `</implementer>`) appearing inside a
fenced code block in a block body is not recognized as a structural close when
computing the recovery offset, because the offset now derives from the
fence-aware `scan_tags`. A regression test pins this: it fails against the
pre-SPEC hand-rolled scan (which counts the fenced occurrence) and passes after
the rewrite.

<done-when>
- For a journal whose last well-formed structural close ends at byte X, but whose
  body also contains a line-isolated journal close tag inside a fenced code block
  at a later byte position Y > X, and which fails strict parse, the computed
  `last_well_formed_byte_offset` is X (the structural close), not Y (the fenced
  occurrence).
- The regression test fails against the pre-SPEC implementation (the hand-rolled
  scan counts the fenced close and yields the wrong offset) and passes after the
  read path moves to `scan_tags`.
</done-when>

<behavior>
- Given a corrupt journal whose only journal-element close tag after byte X is a
  fenced one at Y > X, when the recovery offset is computed at HEAD, then it
  reflects the fenced occurrence (wrong); when computed after the fix, then it is
  X (the last real structural close).
</behavior>

<scenario id="CHK-004">
Given a per-task journal whose last well-formed structural close (`</implementer>`
or `</review>`) ends at byte X, whose body thereafter contains a line-isolated
journal close tag inside a fenced code block ending at byte Y > X, and whose
trailing content makes `journal_xml::parse` fail,
when `detect_journal_drift` computes `last_well_formed_byte_offset` after the fix,
then the value is X; and the recorded pre-fix run of the same test yields a value
reflecting Y, proving the fence-blindness bug it guards.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
The recovery offset is computed by reusing the existing public
`xml_scanner::scan_tags`, not by adding a new recognition primitive. The
scanner-reuse helper is homed in the `journal_xml` parse module — where the
`JOURNAL_ELEMENT_NAMES` whitelist, the `ScanConfig`, the `split_required`
frontmatter split, and the `collect_code_fence_byte_ranges` setup already live
for `journal_xml::parse` — and is consumed by `consistency.rs::detect_journal_drift`.
Rationale: this makes `scan_tags` the sole tag recognizer with two consumers
(the structural parser and the recovery helper), the single-authority end state
DEC-001 of SPEC-0055/0061 named, while keeping `consistency.rs` free of any
recognition logic. Placing the helper in `journal_xml` (rather than inlining the
scan preamble in `consistency.rs`) avoids duplicating the split/fence/ScanConfig
setup and keeps recognition inside the parse layer.
</decision>

<decision id="DEC-002">
The recovery offset is derived from the scanner's token stream, not from the
`ParseError` returned by the failed strict parse. `ParseError` carries an
`offset` on every variant, but it points at the *failure site* (e.g.
`open.span.start` of an offending element, or a bad attribute), which can fall
*mid-element*. Truncating a journal there would leave a partial tag on disk — an
unsafe recovery point. The correct truncation point is the end of the last
*well-formed* depth-0 close, which the `scan_tags` token stream yields directly.
</decision>

<decision id="DEC-003">
A new `recognize_tag_line` / last-well-formed-offset primitive on `xml_scanner`
is explicitly not introduced. SPEC-0061 DEC-003 rejected adding such a primitive
when the canonical parser already covers recognition; that reasoning holds here.
This SPEC *reuses* the existing public `scan_tags` rather than adding parser
surface — the opposite of growing a new recognizer.
</decision>

## Notes

**Correction of record (SPEC-0061 DEC-001).** SPEC-0061's DEC-001 carve-out
justified leaving `last_well_formed_offset` in place by stating that retiring it
"would require exposing a 'last well-formed offset on parse failure' primitive
from the parser — new surface that DEC-003 disfavors." That rationale was
factually wrong: the canonical recognizer `scan_tags` is already public, and
retirement *consumes* it rather than adding surface. SPEC-0061 left this scanner
out for scope reasons — it was not enumerated in that SPEC's census — not because
of cost or principle. The merged SPEC-0061 is dogfood history and is left
unedited; this Note is the live-record correction.

The fence-blindness path is the same class SPEC-0061 cited for the deleted
`next.rs::last_gate_block` scanner: a recognizer that re-derives the parser's
line/fence/structure rules and diverges from them. The consequence here is milder
(a wrong recovery offset on an already-corrupt journal, not a ship spoof), which
is why SPEC-0061 could defer it without shipping a known-live defect — but
deferring is not the same as the scanner being correct.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-12 | Kevin Xiao | Initial SPEC: retire `consistency.rs::last_well_formed_offset`, the last hand-rolled tag recognizer; re-derive the reconcile recovery offset from the canonical `scan_tags`, fixing its fence-blindness. Follow-up to SPEC-0061; corrects that SPEC's DEC-001 rationale of record. |
</changelog>
