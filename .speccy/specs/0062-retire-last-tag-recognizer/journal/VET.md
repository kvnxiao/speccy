---
spec: SPEC-0062
generated_at: 2026-06-12T19:13:40Z
---

## Invocation 1 — 2026-06-12T19:13:40Z

<drift-review verdict="pass" round="1" date="2026-06-12T19:13:40Z" model="claude-opus-4-8[1m]/xhigh">
Diff satisfies SPEC-0062 as a unit: the hand-rolled recovery scanner is retired, the offset now derives from the canonical fence-aware `scan_tags`, and both requirements' done-when are met with all workspace tests and clippy green.

- REQ-001 — covered. `consistency.rs::detect_journal_drift` now calls `journal_xml::last_well_formed_offset` (consistency.rs:426) which reuses the `split_required → collect_code_fence_byte_ranges → ScanConfig{JOURNAL_ELEMENT_NAMES} → scan_tags` preamble and walks the RawTag stream with a depth counter (journal_xml/mod.rs:229-262). The hand-rolled `find('<')` loop and its `#[expect]` are deleted. Audit independently reproduced: `grep` for `find("<`/`find('<'`/`format!("<{` over non-test speccy-core/src + speccy-cli/src yields only vet_xml/serialize.rs:389 (tag emitter, SPEC-0061 CHK-006 exclusion) and journal_common.rs:54 (require_attr building an error string from an already-parsed RawTag.name, not scanning input) — no input recognizer survives. CHK-002/CHK-003 fixtures green at unchanged expected-value formula.
- REQ-002 — covered. detect_journal_xml_malformed_recovery_offset_ignores_fenced_close (consistency_detect.rs:280) drives the full detect_journal_drift path and asserts the recovery offset is the structural close X=153, not the fenced occurrence Y=235; the pre-fix Y is recorded in the test comment and confirmed by reviewers resurrecting the old scan. The fence exclusion comes for free from scan_tags' fence-awareness.
- Implementation correctly recorded `tag.span.end` (byte just past `>`) rather than the `tag.body_end_after_tag` the SPEC summary/TASKS.md hint named — for a close tag body_end_after_tag is the tag *start* (xml_scanner/mod.rs:431) and would truncate mid-tag. The deviation is documented (T-001 Discovered issues) and matches the binding done-when contract. Not drift, but the SPEC body and TASKS.md carry a now-known-wrong hint; worth a human note so a future amendment doesn't re-introduce it.
- Non-blocking handoff note for the human: CHK-001's done-when says the audit is "recorded in REPORT.md", but REPORT.md is a ship-time artifact and does not yet exist. The audit is currently recorded in journal/T-001.md per TASKS.md's two-stage design ("record in the journal so /speccy-ship can lift it into REPORT.md"). Verify /speccy-ship actually performs that lift so CHK-001's done-when is satisfied at ship.
- Goals/non-goals/user-stories all hold: scan_tags is now the sole recognizer with two consumers (single-authority end state); no xml_scanner edit (DEC-003), no DriftDetails/field/reconcile-policy change, no new parser primitive; SPEC-0061 left unedited with the correction-of-record in this SPEC's Notes. The depth==1 pairing refinement diverges from the old "any depth-0 close" only on pathological inputs the SPEC explicitly scopes out, and is more correct.
</drift-review>
<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates clear the bar. The diff retires the hand-rolled find('<') scanner in consistency.rs and replaces it with journal_xml::last_well_formed_offset built on the canonical scan_tags recognizer — the change is already a net simplification (deletes ~80 lines of bespoke tag-walking) and the replacement follows existing codebase idioms.

Considered and rejected:
- Shared scan_tags preamble (collect_code_fence_byte_ranges + ScanConfig + scan_tags) appears in both parse() and last_well_formed_offset(). Only 2 callsites and their error handling diverges immediately (? vs let-else returning 0); extracting a helper would add an abstraction for two divergent users. Not genuine 3-site duplication.
- depth:u32 saturating_add/saturating_sub walk matches the established is_close depth-counter idiom in report_xml/task_xml/spec_xml/journal_common. Consistent.
- The long doc comment records load-bearing DEC-001/DEC-002 rationale and the dual-error-collapses-to-0 contract; trimming would drop invariant documentation, not meta-noise.
</simplifier-scan>
<gate verdict="passed" tasks_hash="d0292d8b2b23dcd6bb853c8c0700a22863fe078db4dc4a92abd406c55381c38e" date="2026-06-12T19:14:48Z">
Drift cleared on round 1; simplifier scan clean. Diff satisfies SPEC-0062 as a unit.
</gate>
