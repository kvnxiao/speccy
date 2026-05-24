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

<report spec="SPEC-0009">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002 CHK-003">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-007">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
