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

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Default priority (no --kind) | delivered |
| REQ-002 | --kind implement and --kind review filters | delivered |
| REQ-003 | Report and blocked kinds | delivered |
| REQ-004 | JSON contract | delivered |
| REQ-005 | Text output | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
