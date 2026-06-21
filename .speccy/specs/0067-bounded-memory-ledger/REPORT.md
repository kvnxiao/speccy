---
spec: SPEC-0067
outcome: implemented
generated_at: 2026-06-21T20:10:00Z
---

# REPORT: SPEC-0067 Bounded memory ledger — a higher capture bar, one-line entries, and autonomous compaction keep `.speccy/MEMORY.md` small and high-signal

<report spec="SPEC-0067">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
T-002 rewrote step 3 of `resources/modules/phases/speccy-ship.md`. The Capture
bullet now states the durable-and-not-already-enforced two-part bar, drops the
"at least one entry per friction loop" mandate and the "no durable lesson this
loop" sentinel, and names recording-nothing as the default outcome including for
gate-already-enforced friction. The dogfood check (CHK-001) is judgment-only: the
live `.speccy/MEMORY.md` compaction produced by T-002 serves as the in-loop
evidence — the SPEC-0066/T-002 entry was dropped because the reviewer-tests
persona already enforces it, confirming the bar suppresses gate-redundant lessons.
Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
T-001 rewrote `resources/modules/references/memory-ledger.md` from the four-part
shape to the one-line shape: trigger, corrective rule, and a compact bracketed
`[SPEC-NNNN/T-NNN]` provenance tag, with no convention/mistake or history field.
The worked example is a single line using only the `SPEC-0042/T-001` carve-out
ids. The provenance bullet documents the bracketed tag and the rule that the task
segment is dropped only for a spec-wide lesson. CHK-002 is judgment-only per
DEC-005 (no format lint; shape adherence is review and dogfooding judgment). The
in-loop MEMORY.md entries written by the T-002 dogfood all conform to the one-line
shape. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
T-002 split the old human-gated "Consolidate and dedupe" bullet into a "Compact
(autonomous)" bullet (refuse-to-append and near-duplicate merge, no human-approval
step, never deletes a non-redundant entry, boundedness rests here) and a separate
"Promote (human-gated)" bullet. CHK-003 is judgment-only. Live dogfood evidence:
the T-002 MEMORY.md compaction collapsed 4 verbose four-part entries to 3 one-line
entries autonomously with no human gate, and did not append a duplicate of the
dropped gate-redundant entry. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
T-002's "Promote (human-gated)" bullet makes promotion the single human-gated
memory mutation, requires human approval before the durable-tier edit lands,
removes the promoted entry from the ledger on approval, and states that
boundedness no longer depends on promotion firing. CHK-004 is judgment-only. The
autonomous compaction (REQ-003) keeps the ledger bounded even in an unattended
`orchestrate → ship` run where no human is present to approve promotion.
Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-005">
T-003 rewrote `resources/modules/references/memory-ledger-summary.md` to name the
one-line shape (trigger, corrective rule, compact provenance tag) instead of the
four-part shape, while leaving the read instruction byte-intact. No residual
four-part wording survives anywhere under `resources/`. CHK-005 is demonstrated by
the two structural tests in `speccy-cli/tests/memory_feedforward.rs`:
`work_phase_body_includes_memory_ledger_summary_once` and
`no_host_wrapper_inlines_memory_ledger_summary_include`, both green. The `{%
include %}` directive appears exactly once in `resources/modules/phases/speccy-work.md`
and in no host wrapper. Retry count: 0.
</coverage>

</report>

## Notes

All five requirements landed on the first implementation round with no review
blockers. The entire diff is prose-only: edits to `resources/modules/phases/speccy-ship.md`
(step 3 retro), `resources/modules/references/memory-ledger.md` (entry-shape
reference), and `resources/modules/references/memory-ledger-summary.md` (read-side
summary), plus `just reeject` propagating those edits into the four ejected host
wrappers (`.claude` and `.codex` for speccy-ship and speccy-work). The Rust CLI was
not touched; `speccy verify` is not coupled to the ledger.

The `.speccy/MEMORY.md` dogfood during T-002 serves as live evidence for REQ-001
through REQ-004: the compaction collapsed 4 verbose entries to 3 one-line entries
autonomously and suppressed one gate-redundant entry without human intervention,
matching the SPEC's described behavior.
