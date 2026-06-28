---
spec: SPEC-0068
generated_at: 2026-06-28T02:43:26Z
---

## Invocation 1 — 2026-06-28T02:43:26Z

<drift-review verdict="pass" round="1" date="2026-06-28T02:43:26Z" model="claude-opus-4-8[1m]/high">
PASS — the diff satisfies SPEC-0068 as a unit; all three Requirements' done-when items are met, no scope creep, no non-goal violations, full workspace green.

REQ-001: `append_under_lock` (speccy-cli/src/journal.rs:543-571) gates the implementer block kind only, runs `demonstrated_chk_ids` after `validate_and_render_block` and strictly before the round-trip parse and `fs_err::write`, preserving the byte-identical-on-failure contract; `MissingDemonstratedEvidence` (journal.rs:226-231) names the CHK id(s), the expected `evidence/T-NNN.md` path, and distinct missing-vs-present-without-scenario reasons. Bullet and prose forms both refused; integration tests bullet_/prose_demonstrated_with_no_evidence_is_refused and evidence_present_without_scenario_is_refused (speccy-cli/tests/journal_append.rs:536-635) cover all three CHKs.

REQ-002: backed-by-scenario, hygiene/judgment-only-only, and incidental-CHK-less-token paths all accept (journal_append.rs:640-740) backed by line-scoped unit tests (speccy-core/src/parse/journal_xml/evidence.rs:65-136). The whole-file at-least-one `### Scenario` semantics (not per-CHK) is exactly what REQ-002 done-when, DEC-002, and the "presence ... nothing deeper" non-goal authorize — not drift.

REQ-003: resources/modules/phases/speccy-work.md inserts ordered step 9 (write evidence file) between implement and append with correct trailing renumbering and consistent intra-body cross-references (steps 4-12, retry refs step 9/11); both references carry the one-line demonstrated-vs-hygiene disambiguation naming the refusal; docs/CLI.md and docs/SCHEMA.md document the refusal condition matching the error shape; resource_prose_hygiene passes over the edited phases body; eject == source (dogfood test green).

Scope: the only new public surface is `demonstrated_chk_ids`, `scenario_heading_count`, `pub mod evidence`, and the `MissingDemonstratedEvidence` variant — all SPEC/T-001-authorized. No new CLI flag, env var, or config key; no `speccy verify` lint (honoring the non-goal/DEC-003). The gate is dogfooded: T-001's own implementer append exercised it end-to-end, and T-002's first append was correctly refused as a DEC-001 recoverable false positive (trigger token co-occurring with a CHK id in roll-call prose) then reworded — the gate working as designed, not drift.
</drift-review>
<simplifier-scan verdict="clean">
No simplification candidates worth applying. The SPEC-0068 production diff is already minimal and idiomatic: two pure helpers in `speccy-core/src/parse/journal_xml/evidence.rs` (`demonstrated_chk_ids`, `scenario_heading_count`) and a focused gate block in `speccy-cli/src/journal.rs`.

Considered and rejected:
- evidence.rs:33-46 `demonstrated_chk_ids` — the lines/filter/flat_map chain is already the boring, obvious form; collapsing further would hurt clarity.
- journal.rs:551-563 — the three-arm match maps each on-disk state (present-with-scenario / present-without-scenario / missing) to the distinct reason string REQ-001 mandates; folding the arms would drop the missing-vs-present-without-scenario distinction, i.e. a behavior change, not a simplification.
- The `&'static str` reason field is the right altitude for a private two-value distinction used at one callsite; an enum would add lines for no clarity gain.

Doc/prose edits (recipe, references, CLI.md, SCHEMA.md) are outside simplifier scope.
</simplifier-scan>
<gate verdict="passed" tasks_hash="2954611176bf2d71bab71a99e96f35bda044363c077183ebe842bb561cf78d03" date="2026-06-28T02:46:15Z">
Drift cleared on round 1; simplifier: clean; provenance: clean.
</gate>
