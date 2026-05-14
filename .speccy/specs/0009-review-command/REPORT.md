---
spec: SPEC-0009
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0009 review-command

## Summary

`speccy review TASK-ID --persona <name>` renders the Phase 4
reviewer prompt for one persona per invocation. Persona registry
(`speccy_core::personas::ALL`) is the source of truth that
SPEC-0007 consumes as `&ALL[..4]` for default fan-out. Resolver
chains project-local override (`.speccy/skills/personas/`) before
the embedded bundle. Diff helper shells out to `git diff HEAD`
with fallbacks. Landed in commit `f4720fe`.

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Persona registry | delivered |
| REQ-002 | Persona file resolution | delivered |
| REQ-003 | --persona argument validation | delivered |
| REQ-004 | Diff computation with fallback chain | delivered |
| REQ-005 | Render reviewer prompt | delivered |
| REQ-006 | Reuse SPEC-0008's task lookup; error mapping | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
