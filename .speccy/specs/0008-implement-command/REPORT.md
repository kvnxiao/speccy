---
spec: SPEC-0008
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0008 implement-command

## Summary

`speccy implement TASK-ID` renders the Phase 3 implementer prompt
with full SPEC.md, the task subtree (verbatim from TASKS.md),
AGENTS.md, and suggested-files inlined. Lands
`speccy_core::task_lookup` (parses `T-NNN` and `SPEC-NNNN/T-NNN`,
surfaces `Ambiguous` with candidate spec IDs) which SPEC-0009 reuses
for review. Landed in commit `2b1ee4c`.

<report spec="SPEC-0008">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-005">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
