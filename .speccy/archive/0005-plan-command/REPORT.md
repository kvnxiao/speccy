---
spec: SPEC-0005
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0005 plan-command

## Summary

`speccy plan [SPEC-ID]` renders the Phase 1 prompt (greenfield or
amendment). Lands the cross-cutting `prompt::` helpers consumed by
subsequent commands: `load_template`, `render`, `load_agents_md`,
`allocate_next_spec_id`, and `trim_to_budget` (with ARCHITECTURE.md's
five-step drop ordering). Landed in commit `960a5a1`.

<report spec="SPEC-0005">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006 CHK-007">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-008">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
