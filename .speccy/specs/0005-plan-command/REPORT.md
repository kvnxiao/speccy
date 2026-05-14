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

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Greenfield prompt rendering | delivered |
| REQ-002 | Amendment prompt rendering | delivered |
| REQ-003 | Spec ID allocation | delivered |
| REQ-004 | AGENTS.md loading | delivered |
| REQ-005 | Template loading and placeholder substitution | delivered |
| REQ-006 | Context-budget trimming | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
