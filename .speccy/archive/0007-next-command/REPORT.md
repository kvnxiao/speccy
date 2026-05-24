---
spec: SPEC-0007
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0007 next-command

## Summary

`speccy next [--kind <implement|review|report|blocked>] [--json]`
picks the next actionable task across the workspace. Priority is
mechanical: ascending spec ID; within a spec, prefer `[?]` over
`[ ]`. `--kind` filters strictly without fallback. `report` kind
fires when all tasks are `[x]` and any REPORT.md is missing.
`blocked` returns a canonical reason. JSON output follows the
schema_version=1 envelope from SPEC-0004. Landed in commit
`ffad1ec`.

<report spec="SPEC-0007">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-009">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
