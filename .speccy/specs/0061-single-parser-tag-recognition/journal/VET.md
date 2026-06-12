---
spec: SPEC-0061
generated_at: 2026-06-12T08:20:41Z
---

## Invocation 1 — 2026-06-12T08:20:41Z

<drift-review verdict="pass" round="1" date="2026-06-12T08:20:41Z" model="claude-opus-4-8[1m]/xhigh">
Diff satisfies SPEC-0061 as a unit: both enumerated hand-rolled scanners are deleted, both read/write paths route through the canonical parsers, all five requirements' done-when hold and the targeted tests pass. One pre-existing-recognizer observation surfaced for the human (non-blocking).

Requirement coverage (all met):
- REQ-001 — vet_gate_is_fresh_pass now derives the terminal gate from parse_vet_in_flight's typed VetDoc (doc.invocations.last().blocks.last() matched as VetBlock::Gate); the cursor.find("<gate") scan is gone. CHK-001/002/003 pass (speccy-core/src/next.rs:164-204, speccy-core/tests/next_priority.rs).
- REQ-002 — append_under_lock round-trips the assembled new_content through strict parse_journal_xml immediately before fs_err::write, raising the new ProducedJournalUnparseable (distinct from ExistingJournalUnparseable). CHK-004/005 pass (speccy-cli/src/journal.rs:537-547, :194-214).
- REQ-003 (enumerated half) — last_gate_block/GateBlock/attribute_value and first_nested_journal_element/SerializeError::NestedJournalMarkup plus their unit tests and the stale module docs are all deleted; validate_and_render_block no longer pre-scans the body. CHK-006 grep over production shows only a renderer (vet_xml/serialize.rs:389) and an error-context format (journal_common.rs:54), no recognizer among the enumerated set.
- REQ-004 — per-crate renderer-backed helpers added (speccy-cli/tests/common/mod.rs:215-328 render_vet_md/render_vet_block; speccy-core/tests/next_priority.rs:237-280); all valid hand-rolled VET.md migrated (next_text.rs, next_json.rs, next_derived.rs, journal_show.rs); lint_vet.rs and the open journal_show fixture left as DEC-005 carve-outs. CHK-007 passes in both crates.
- REQ-005 — gate-spoof regression vet_when_terminal_gate_failed_despite_inline_passing_gate present, inline gate carries a matching tasks_hash, asserts Vet; pre-fix Ship observation recorded in journal/T-002.md (CHK-009). Passes.

Scope: production changes confined to the four named files (next.rs, journal.rs, serialize.rs, vet_xml/mod.rs), net -136 lines, deletion-heavy. Only new public surface is JournalError::ProducedJournalUnparseable, which REQ-002 mandates. No new CLI flags, output shapes, env vars, or config keys. No non-goal violated.

Observation for the human (NOT blocking — pre-existing, outside REQ-003's enumerated scope):
- Goals / Summary ("exactly one implementation of tag recognition"; "No hand-rolled tag scanner remains in production code: the canonical xml_scanner-backed parsers are the sole tag recognizers") read more broadly than REQ-003/CHK-006 deliver. A third hand-rolled tag recognizer survives in production: speccy-core/src/consistency.rs:459 last_well_formed_offset walks <implementer>/<review>/<blockers> open/close pairs via source.find('<') + name matching to compute a reconcile truncation offset. CHK-006's audit pattern (find("<, find("</, format!("<{ — double-quote literals) structurally misses it because the scan uses find('<') (single char), and REQ-003 never enumerated it. The SPEC's stated motivation (it opens "Today there are three" and names exactly the canonical parser + last_gate_block + first_nested_journal_element) suggests the author did not account for this fourth site. Whether the broader "sole recognizer" goal should also retire last_well_formed_offset (it runs only after the strict parser has already rejected the source, so it is a deliberate tolerant recovery scan, not a primary recognizer) is a SPEC-scope decision for the human, not something to re-task the implementer on in this loop.

Report-phase note (not drift): CHK-006/CHK-008 done-when say "the audit is recorded in REPORT.md"; REPORT.md is absent, which is expected pre-ship — the audits are recorded in journal/T-002.md and T-003.md and the REPORT.md recording is a report-phase deliverable, not an implementation-diff one.
</drift-review>
<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates in the SPEC-0061 diff worth applying.

The SPEC's production changes are themselves a net simplification: two hand-rolled scanners (`last_gate_block`/`attribute_value` in next.rs, `first_nested_journal_element` + `SerializeError::NestedJournalMarkup` in serialize.rs) are deleted in favor of routing recognition through the typed parsers (`parse_vet_in_flight`, write-time `parse_journal_xml` round-trip). Orphan cleanup is complete — `JOURNAL_ELEMENT_NAMES` is still used at its remaining call-sites (xml.rs), the import removal from serialize.rs is correct, and no dangling references remain.

Candidates considered and deliberately rejected:
- next_priority.rs:233+ — `render_vet_md` and `render_spoof_vet_md` share a 3-line frontmatter+heading+format tail. Extracting a third helper would add a named indirection for a 3-line DRY win across two functions that differ in gate verdict/body; the SPEC designed these as parallel renderer-backed fixtures (DEC-004). Not worth the cognitive cost.
- The `render_vet_md` helper appearing in both common/mod.rs and next_priority.rs is a Rust integration-test-binary constraint (no shared module across crates), already documented in-code — not removable duplication.
- Threading `spec_id` through the ~12 `write_vet_md` call-sites is load-bearing signal (frontmatter now needs the real spec id), not noise.

The diff is already at the right altitude.
</simplifier-scan>
<gate verdict="passed" tasks_hash="63201864e2efb4d96f0a5009f3a85bcd59e28f53f5502c4021feaa1142ba9270" date="2026-06-12T08:22:13Z">
Drift cleared on round 1; simplifier clean. One non-blocking aside surfaced for human review: pre-existing consistency.rs::last_well_formed_offset recognizer is outside SPEC-0061 scope.
</gate>
## Invocation 2 — 2026-06-12T08:32:17Z

<drift-review verdict="pass" round="1" date="2026-06-12T08:32:17Z" model="claude-opus-4-8[1m]/xhigh">
Re-vet after the narrative-accuracy amend (bb23df0): the SPEC narrative now matches what the diff delivers, and invocation 1's lone non-blocking observation is resolved. No production code changed since invocation 1 (only SPEC.md prose + the re-locked TASKS.md hash); all five requirements' done-when and all nine scenarios are byte-identical and were verified holding in invocation 1.

Narrative-vs-diff verification (the amend's stated purpose):
- Summary now says "Two hand-rolled byte-substring scanners exist on those paths today" (was the "three" census) and names exactly next.rs::last_gate_block and serialize.rs::first_nested_journal_element — the two this SPEC retires — then explicitly carves out consistency.rs::last_well_formed_offset as a tolerant post-parse-failure recovery scan that is out of scope. This matches the code: last_well_formed_offset (speccy-core/src/consistency.rs:459) runs only on the recovery path after the strict parser has rejected the source, per its own doc comment (consistency.rs:451-454).
- Goals' fourth bullet now scopes "no hand-rolled tag scanner remains" to "the two production paths this SPEC addresses" with an explicit consistency.rs carve-out; no longer the global "sole tag recognizers" overstatement invocation 1 flagged.
- DEC-001 now excludes last_well_formed_offset with sound rationale (removing it requires exposing a "last well-formed offset on parse failure" parser primitive that DEC-003 disfavors). Matches the code's structure.
- The new Changelog row (SPEC.md:477) correctly records the fix as narrative-only ("All done-when criteria and scenarios unchanged") and accurately notes that CHK-006's audit pattern (double-quote find("<…") / format!("<{…")) never structurally claimed to cover the single-char find('<') walk.

Production recognizer audit re-run confirms the amended narrative is accurate. The only find/format hits in non-test speccy-core/speccy-cli source are: consistency.rs:466 (the carved-out single-char find('<') recovery walk), journal_common.rs:54 (an error-context format!, not a recognizer), and vet_xml/serialize.rs:389 (a block renderer, explicitly excluded by REQ-003 done-when and the SPEC Notes). consistency.rs:552's find("</implementer>") is inside #[cfg(test)] mod tests (starts :521), so it is test code outside REQ-003's non-test scope. No hand-rolled recognizer remains among the two paths this SPEC addresses.

The vet_xml/mod.rs module doc (updated by T-002, in scope) now reads consistently with the narrowed narrative: it claims sole-recognizer status for the grammar this module owns and points next.rs's freshness check at the typed VetDoc — not a global "no scanner anywhere" claim. No contradiction with the carved-out consistency.rs walk.

No scope creep, no new public surface beyond REQ-002's mandated JournalError::ProducedJournalUnparseable, no non-goal violated. The diff satisfies SPEC-0061 as a unit and the SPEC narrative is now an accurate description of it.
</drift-review>
<simplifier-scan verdict="clean">
No simplification candidates in the SPEC-0061 diff. Production code diff (next.rs, serialize.rs, vet_xml/mod.rs, journal.rs) is byte-identical to the prior clean vet scan — only SPEC.md prose and a re-locked TASKS.md hash changed this invocation. The diff is itself a simplification SPEC (three ad-hoc tag scanners collapsed into one typed parser); the resulting `vet_gate_is_fresh_pass` is a tight let-else guard chain with no duplication or dead code, and the `journal.rs` round-trip is a single `map_err(...)?`. Already at minimal form; any further reduction would change behavior or trip AGENTS.md conventions.
</simplifier-scan>
<gate verdict="passed" tasks_hash="c0856cf6102968521c06247e998b5586eec3702e9c343e329f0426aa9e1356ba" date="2026-06-12T08:33:44Z">
Re-vet after narrative-accuracy amend: drift cleared on round 1 (invocation 1's last_well_formed_offset observation resolved by the amend); simplifier clean.
</gate>
