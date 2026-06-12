---
id: SPEC-0061
slug: single-parser-tag-recognition
title: Single parser authority for tag recognition — gate-read and journal-write paths use the canonical parser, deleting both hand-rolled scanners
status: in-progress
created: 2026-06-12
supersedes: []
---

# SPEC-0061: Single parser authority for tag recognition — gate-read and journal-write paths use the canonical parser, deleting both hand-rolled scanners

## Summary

There should be exactly one implementation of "recognize a Speccy XML tag in
text." Today there are three: the canonical line/fence/structure-aware parser
in `xml_scanner` (behind `parse_journal_xml` and `parse_vet_in_flight`), plus
two hand-rolled byte-substring scanners that re-derive — incompletely — what
the canonical parser already does. The first is `next.rs::last_gate_block`, the
read path behind `speccy next`'s ship-gate freshness check; it walks
`cursor.find("<gate")` with no line isolation, no fence awareness, and no
structural model. The second is `journal_xml/serialize.rs::first_nested_journal_element`,
a per-task journal write-side body guard that only matches open tags
(`<implementer`), never close tags (`</implementer>`).

The divergence has produced two defects. A **live** gate-spoof: a failing
terminal gate whose body quotes an inline `<gate verdict="passed">` (with a
matching `tasks_hash`)
makes the tolerant scan pick the quoted gate, so `speccy next` reports `ship`
when it should report `vet`. A **latent** delayed failure: a line-isolated
`</implementer>` in a block body passes the open-tags-only guard, lands on disk,
and detonates as `ExistingJournalUnparseable` on the *next* append, far from its
cause.

DEC-008 (SPEC-0055) already established the cure — "the parser is the single
authority; no parallel hand-rolled tag scan" — and applied it to the vet *write*
path, deleting that path's body-inertness guard in favor of a write-time
round-trip. This SPEC finishes DEC-008 by extending the same principle to the
two readers it did not reach: the gate-read path moves onto `parse_vet_in_flight`'s
typed `VetDoc` (a `<gate>` quoted in a body is not a top-level `VetBlock::Gate`,
so the spoof dies by construction), and the per-task write path gains a
`parse_journal_xml` round-trip mirroring the vet path's. Both hand-rolled
scanners — and their helpers, their dead error variant, and the stale module
doc that described them — are deleted. The same divergence class lives in the
test layer too (four fixtures hand-roll an invalid VET.md the real parser
rejects), so valid VET.md test construction is routed through renderer-backed
helpers (one per test crate) built on the production renderers.

## Goals

<goals>
- `speccy next` cannot be driven to `ship` by a `<gate>` tag quoted inside a
  VET.md block body; only the document's structurally terminal gate gates a
  ship.
- A malformed or in-flight VET.md (no terminal gate) resolves to `vet`, never
  `ship`.
- `speccy journal append` refuses to write a per-task journal that would not
  parse, failing before the file is modified and leaving it byte-identical (or
  absent for a new journal).
- No hand-rolled tag scanner remains in production code: the canonical
  `xml_scanner`-backed parsers are the sole tag recognizers.
- All valid test VET.md is constructed through renderer-backed helpers (one per
  test crate) built on the production renderers, so fixtures cannot diverge from
  the real grammar.
</goals>

## Non-goals

<non-goals>
- No change to the VET.md or per-task journal grammar; only which code path
  recognizes it.
- No new tag-recognition primitive. Recognition stays inside the existing
  canonical parsers; a standalone line-recognizer is explicitly not introduced
  (DEC-003).
- No change to the `review > work > vet > ship` priority ordering or to the
  gate-freshness contract (terminal passing gate whose `tasks_hash` matches the
  on-disk TASKS.md SHA-256); only the implementation of that contract.
- No change to the hash algorithm or `tasks_hash` semantics.
- No shared round-trip helper across the vet and per-task append paths; the two
  remain inline call sites (DEC-002).
- No edit to the archived SPEC-0055; its DEC-008 is refined by reference here,
  not modified in place.
</non-goals>

## User Stories

<user-stories>
- As an agent driving `speccy next`, I want the ship gate to reflect the real
  terminal vet verdict, so a vet narrative that quotes a passing gate inline can
  never spoof a premature ship.
- As an agent appending to a per-task journal, I want a malformed block body
  rejected at write time with the file untouched, so corruption surfaces at its
  cause rather than as a confusing failure on a later append.
- As a maintainer, I want exactly one implementation of tag recognition, so this
  class of divergence bug cannot recur and the test fixtures cannot drift from
  the grammar.
</user-stories>

## Assumptions

<assumptions>
- The gate-read path uses tolerant `parse_vet_in_flight` (not strict `parse`);
  for the ship/vet decision the two are equivalent, and the tolerant mode avoids
  conflating a legitimately mid-run VET.md with a corrupt one.
- The per-task journal round-trip uses strict `parse_journal_xml`; per-task
  journals have no in-flight/open-section concept.
- `parse_vet_in_flight`, `VetBlock`, and the production VET renderers are
  reachable from the call sites that need them (`next.rs` already imports
  `crate::parse`; the renderers are exported from `speccy_core::parse`).
- Routing valid test VET.md through the production renderers makes fixture
  validity a guarantee by construction, eliminating the test-side instance of the
  grammar-divergence class.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: Gate-freshness resolves through the typed VET parser

`vet_gate_is_fresh_pass` derives the terminal gate's `verdict` and `tasks_hash`
from `parse_vet_in_flight`'s typed `VetDoc` — the terminal `VetBlock::Gate` of
the last invocation section — rather than a byte-substring scan. A `<gate>`
appearing inside a block body is not a top-level gate and cannot satisfy the
freshness check. A VET.md that fails to parse, or that has no terminal gate,
counts as not fresh.

<done-when>
- A VET.md whose terminal gate is `verdict="failed"` resolves to `vet`, even
  when its body quotes an inline `<gate verdict="passed">` whose `tasks_hash`
  matches the on-disk TASKS.md.
- A VET.md with a `verdict="passed"` terminal gate whose `tasks_hash` matches the
  on-disk TASKS.md SHA-256, with REPORT.md absent, resolves to `ship`.
- A VET.md that fails to parse, or that has no terminal gate, resolves to `vet`.
- `next.rs` contains no `cursor.find("<gate")`-style scan.
</done-when>

<behavior>
- Given an all-completed spec with a valid VET.md whose terminal gate is
  `failed` but whose body quotes an inline passing gate, when `speccy next`
  resolves, then the action is `vet`.
- Given the same spec with a fresh passing terminal gate and REPORT.md absent,
  when `speccy next` resolves, then the action is `ship`.
- Given a spec whose VET.md lacks required frontmatter, when `speccy next`
  resolves, then the action is `vet`.
</behavior>

<scenario id="CHK-001">
Given a built speccy binary after this SPEC lands and an all-completed spec
whose VET.md has a `failed` terminal gate with an inline
`<gate verdict="passed">` (whose `tasks_hash` matches the on-disk TASKS.md) in
its body,
when `compute_for_spec` resolves the spec,
then the resolved action is `Vet`.
</scenario>

<scenario id="CHK-002">
Given the same binary and an all-completed spec with a fresh passing terminal
gate and REPORT.md absent,
when `compute_for_spec` resolves the spec,
then the resolved action is `Ship`.
</scenario>

<scenario id="CHK-003">
Given the same binary and an all-completed spec whose VET.md is missing the
required frontmatter (and is therefore unparseable),
when `compute_for_spec` resolves the spec,
then the resolved action is `Vet`.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: Per-task journal append round-trips the produced file before writing

`append_under_lock` re-parses the would-be-new content with strict
`parse_journal_xml` before writing any byte, refusing on parse failure with a
new `JournalError::ProducedJournalUnparseable` (mirroring the vet path's
`ProducedVetUnparseable`). On rejection the on-disk journal is unchanged, or
absent if it did not previously exist. A block body whose own line is journal
markup is rejected; a body that mentions an element name inline as prose is
accepted as inert text.

<done-when>
- Appending a block whose body contains a line-isolated `</implementer>` exits
  non-zero and leaves the journal byte-identical (or absent if it did not
  exist).
- Appending a block whose body mentions `<review>` inline within a prose
  sentence succeeds and produces a journal that re-parses under
  `parse_journal_xml`.
- The error raised when the produced file would not parse is
  `ProducedJournalUnparseable`, distinct from `ExistingJournalUnparseable`.
</done-when>

<behavior>
- Given an existing parseable journal, when an append carries a body line that
  is exactly `</implementer>`, then the write is refused and the file is
  unchanged.
- Given a fresh journal, when an append carries a body sentence mentioning
  `<review>` inline, then the file is written and re-parses cleanly.
</behavior>

<scenario id="CHK-004">
Given a CLI workspace with a parseable per-task journal,
when `speccy journal append` is invoked with a block body containing a
line-isolated `</implementer>`,
then the command exits non-zero, stderr names the produced-unparseable
condition, and the journal file is byte-identical to before the invocation.
</scenario>

<scenario id="CHK-005">
Given a CLI workspace with no existing per-task journal,
when `speccy journal append` is invoked with a body mentioning `<review>`
inline in a prose sentence,
then the command exits 0 and the resulting journal parses under
`parse_journal_xml`.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: No hand-rolled tag recognizer remains in production

The two byte-substring scanners and their attendant code are deleted, leaving
the canonical parsers as the only tag recognizers. Removed: `next.rs`'s
`last_gate_block`, `GateBlock`, and `attribute_value`; `serialize.rs`'s
`first_nested_journal_element`, its callsite in `validate_and_render_block`, and
the `SerializeError::NestedJournalMarkup` variant; and the `vet_xml` module-doc
prose that described `next`'s independent tolerant `<gate>` scanner.

<done-when>
- `last_gate_block`, `GateBlock`, and the `next.rs` `attribute_value` helper no
  longer exist in the source tree.
- `first_nested_journal_element` and `SerializeError::NestedJournalMarkup` no
  longer exist; `validate_and_render_block` no longer pre-scans the body for
  nested markup.
- The `vet_xml` module doc no longer references a separate tolerant `<gate>`
  scanner in `next`.
- No hand-rolled tag *recognizer* (a `find("<…")` / `format!("<{…")` scan over
  input text) remains in non-test `speccy-core` or `speccy-cli` source. Block
  *renderers* that emit tags (e.g. the `format!("<{element} …")` calls in the VET
  and journal serializers) are the canonical output path and are explicitly not
  in scope.
</done-when>

<behavior>
- Given the post-SPEC source tree, when the deletions land, then the workspace
  compiles with no reference to the removed symbols and the behavior covered by
  REQ-001 and REQ-002 flows entirely through the canonical parsers.
</behavior>

<scenario id="CHK-006">
Given the post-SPEC `speccy-core` and `speccy-cli` source,
when a reviewer audits production (non-test) code for hand-rolled tag-scan
patterns (`find("<`, `find("</`, `format!("<{`),
then the only matches are block renderers that emit tags (not recognizers
scanning input) — no hand-rolled recognizer remains — and the audit is recorded
in REPORT.md.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: All test VET.md is built through renderer-backed helpers

A renderer-backed test helper constructs VET.md by calling the exported
production renderers (`render_fresh_vet_frontmatter`, `render_vet_section_heading`)
plus a gate block, so every fixture matches the real grammar by construction.
Because Rust integration-test binaries cannot share a module, there is one such
helper per test crate (`speccy-core`'s tests and `speccy-cli`'s tests), both
built on the same renderers (DEC-004). Every test that constructs a *valid*
VET.md routes through its crate's helper, replacing the hand-rolled
`## Invocation N` / `<gate>` strings.

<done-when>
- Each test crate has one renderer-backed VET.md helper, and no test hand-rolls a
  *valid* `## Invocation N` / `<gate>` VET.md string outside such a helper.
- The parser-invalid fixtures (`next_priority.rs`, `next_text.rs`, `next_json.rs`,
  `common/mod.rs`) and the valid hand-rolled VET.md in `journal_show.rs` are
  routed through a helper.
- `lint_vet.rs` and the intentionally-open (gate-less) fixture in
  `journal_show.rs` stay hand-rolled and are out of scope: they build
  deliberately-invalid or structurally-edge VET.md to exercise the parse/lint
  boundary, which a valid-only helper cannot produce (DEC-005).
- `cargo test --workspace` is green with the fixtures routed through the helpers.
</done-when>

<behavior>
- Given a renderer-backed helper, when any test constructs a VET.md for an
  arbitrary `(verdict, tasks_hash)`, then the produced document is accepted by
  `parse_vet_in_flight` and its terminal gate carries that verdict and hash.
</behavior>

<scenario id="CHK-007">
Given a renderer-backed test helper,
when it renders a VET.md for a `(verdict, tasks_hash)` pair,
then `parse_vet_in_flight` accepts the result and its terminal `VetBlock::Gate`
carries that verdict and `tasks_hash`.
</scenario>

<scenario id="CHK-008">
Given the test suite after migration,
when a reviewer audits the test modules for hand-rolled *valid* VET.md
construction,
then none remains outside a renderer-backed helper — excepting the
intentionally-invalid / gate-less fixtures in `lint_vet.rs` and `journal_show.rs`
that exercise the parse/lint boundary — and the audit is recorded in REPORT.md.
</scenario>
</requirement>

<requirement id="REQ-005">
### REQ-005: Gate-spoof regression test fails on HEAD, passes after the fix

A regression test constructs a valid VET.md whose terminal gate is `failed` and
whose body quotes an inline `<gate verdict="passed">` whose `tasks_hash` matches
the fixture's TASKS.md, then asserts `speccy next` resolves to `vet`. The test is
written first and must fail against the pre-fix implementation (the tolerant
scan reads the inline passing gate and yields `ship`), proving the live bug;
after the read path moves to the parser it passes.

<done-when>
- The regression test asserts `vet` for the spoof fixture.
- Run against the pre-fix implementation, the test fails (the resolved action is
  `Ship`), demonstrating the live bug.
- Run against the post-fix implementation, the test passes (the resolved action
  is `Vet`).
</done-when>

<behavior>
- Given the spoof fixture, when `speccy next` resolves at HEAD, then it yields
  `ship`; when it resolves after the fix, then it yields `vet`.
</behavior>

<scenario id="CHK-009">
Given the spoof VET.md fixture (valid document; `failed` terminal gate; inline
`<gate verdict="passed">` in its body whose `tasks_hash` matches the fixture's
TASKS.md; REPORT.md absent; all tasks completed),
when `compute_for_spec` resolves the spec after the fix,
then the action is `Vet`; and the recorded pre-fix run of the same test yields
`Ship`, proving the bug it guards.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
Single-parser-authority (SPEC-0055 DEC-008) is extended to the two readers it
did not reach: the gate-freshness read path in `next.rs` and the per-task
journal write path in `speccy-cli/src/journal.rs`. After this SPEC, no
hand-rolled tag scan remains anywhere in production — recognition lives only
inside the canonical `xml_scanner`-backed parsers. Rationale: the two readers
re-implemented the parser's tag/line/fence/structure rules and diverged from it
needle-by-needle, the exact class DEC-008 named. The read path collapses onto
`parse_vet_in_flight`'s typed `VetDoc`; the per-task write path gains a
`parse_journal_xml` round-trip mirroring the vet path's. This refines DEC-008
rather than restating it; the archived SPEC-0055 is not modified.
</decision>

<decision id="DEC-002">
The vet and per-task write-time round-trips are kept as two inline call sites,
not extracted into a shared helper. They delegate to different canonical parsers
(`parse_vet_in_flight` versus `parse_journal_xml`) and raise different typed
error variants (`ProducedVetUnparseable` versus `ProducedJournalUnparseable`); a
generic helper parameterized over both parser and error constructor would be
less readable than the explicit duplication, which is structural rhyme rather
than divergent logic. The consolidation this SPEC pursues is collapsing the
divergent *recognizers*, not deduplicating two correct delegations.
</decision>

<decision id="DEC-003">
No new tag-recognition primitive is introduced. An alternative framing proposed
adding a shared `recognize_tag_line` line-recognizer to `xml_scanner` and
migrating both byte-scanners onto it. It is rejected: it would add a primitive
only two call sites use — both already covered by the canonical parsers — would
be fence-blind (a line-isolated fake `<gate>` inside fenced prose would still
spoof unless fence exclusion were re-added), and would preserve three
recognizers rather than collapsing to one.
</decision>

<decision id="DEC-004">
REQ-004's "one shared helper" is realized as **one renderer-backed helper per
test crate**, not a single function. The VET.md-constructing fixtures span two
integration-test crates (`next_priority.rs` in `speccy-core`; the rest in
`speccy-cli`), and Rust integration-test binaries cannot share a `mod common`
across crates. The contract REQ-004 enforces is therefore "every test VET.md is
built from the exported production renderers (`render_fresh_vet_frontmatter`,
`render_vet_section_heading`) — no hand-rolled `## Invocation` / `<gate>` string
outside such a helper," with one helper living in each crate's test-support
module, both built on the same renderers. Rationale: the renderers are the
shared authority that makes fixtures grammar-valid by construction; a single
literal function would force a test-only construction surface onto the
production `speccy-core` crate, a smell the per-crate split avoids. The two
helpers are structural rhyme over the same renderers, not divergent logic — the
same shape DEC-002 accepts for the two inline round-trips.
</decision>

<decision id="DEC-005">
The renderer-backed helper produces **valid, gate-terminated** VET.md — CHK-007
requires `parse_vet_in_flight` to accept its output. Fixtures that must be
*invalid* or structurally special to exercise a specific grammar condition are
therefore legitimately outside it and keep hand-rolling by design. Concretely,
`lint_vet.rs` builds deliberately-malformed VET.md to drive the `VET-*` lint
family — a missing
frontmatter to fire VET-001, an out-of-domain `verdict="maybe"`, a gate-ordering
violation to fire VET-002 — none of which a valid-only helper can produce. Its
hand-rolling is the test, not drift. The `journal_show.rs` intentionally-open
(gate-less) fixture is the same kind of exception. CHK-008's "no `## Invocation`
/ `<gate>` string outside the helper" audit therefore covers **valid** VET.md
construction; intentionally grammar-edge fixtures that test the lint/parse
boundary are carved out and recorded as such in REPORT.md. The migration set that
actually routes through the helpers is `next_priority.rs`, `next_text.rs`,
`next_json.rs`, `common/mod.rs`, and the valid VET.md in `journal_show.rs`.
</decision>

## Notes

This SPEC finishes the work DEC-008 (SPEC-0055) began. That decision made the
VET *write* path use the parser as the sole authority and removed its
body-inertness guard in favor of a write-time round-trip; the gate *read* path
and the per-task journal *write* path were left with hand-rolled scanners. This
SPEC removes both.

The gate-freshness contract documented in `docs/ARCHITECTURE.md` (a trailing
passing `<gate>` block whose `tasks_hash` matches the current TASKS.md SHA-256)
is unchanged; only its implementation moves from a tolerant byte-scan to the
typed parser. `vet_gate_is_fresh_pass`'s existing doc comment
already specifies that a parse failure counts as not-fresh because re-vetting is
safer than shipping on a malformed artifact, so the read-path change makes the
implementation match its own documented contract.

The targeted `NestedJournalMarkup` diagnostic is intentionally dropped in favor
of the generic produced-unparseable error, accepting the same trade DEC-008 made
for the vet path: the write-time round-trip is a complete superset of what the
open-tags-only guard checked, so a second hand-rolled guard would only re-introduce
the divergence being removed.

REQ-004 (the helper migration) must land before REQ-001 (the read-path change).
Several existing fixtures currently hand-roll a frontmatter-less VET.md that the
byte-scanner tolerates but `parse_vet_in_flight` rejects; the moment the read
path moves to the parser, those fixtures would parse-fail and flip `ship`→`vet`.
Routing them through the renderer-backed helpers first makes them grammar-valid,
so the read-path change is a no-op for them. (Sequencing detail for the task
list; the requirement set itself is order-independent.)

The CHK-006 audit (REQ-003) targets hand-rolled tag _recognizers_ — code that
scans input text for a tag (`find("<…")`). It deliberately excludes the block
_renderers_ (e.g. the `format!("<{element} verdict=…")` calls in
`vet_xml/serialize.rs` and the journal serializer), which emit tags rather than
recognize them and are the canonical output path. A naive grep for the
`format!("<{` pattern will match those renderers; they are expected hits, not
violations, and the audit recorded in REPORT.md should note them as such.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-12 | Kevin Xiao | Initial SPEC: single parser authority for tag recognition; delete the gate-read and journal-write hand-rolled scanners (refines SPEC-0055 DEC-008). |
| 2026-06-12 | speccy-decompose | Promoted decompose-time decisions: DEC-004 (REQ-004's "one helper" is realized per-crate, not one shared function), DEC-005 (the renderer-backed helper is valid-only, so `lint_vet.rs`'s intentionally-invalid fixtures stay hand-rolled and are carved out of CHK-008), and a Note that CHK-006's audit excludes block renderers. No requirement text changed. |
| 2026-06-12 | speccy-amend | Folded DEC-004/DEC-005 into the requirement text so REQ-004 and REQ-003/CHK-006 stop contradicting themselves: REQ-004 now states per-crate renderer-backed helpers and the corrected migration set (dropped "single helper" and the false "valid-but-hand-rolled" label on `lint_vet.rs`; carved out `lint_vet.rs` and the open `journal_show.rs` fixture); REQ-003 done-when + CHK-006 scope the audit to recognizers, excluding block renderers; added a Notes sequencing line (REQ-004 before REQ-001). Clarification for implementation clarity — no requirement added, dropped, or behaviorally changed. |
</changelog>
