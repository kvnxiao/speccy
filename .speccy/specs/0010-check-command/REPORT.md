---
spec: SPEC-0010
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0010 check-command

## Summary

`speccy check [SPEC-ID] [--id CHK-ID]` discovers checks across the
workspace, runs `command:` checks via the shell with live streaming,
renders `prompt:` (manual) checks for agent consumption, and summarises
results. Exit code semantics: 0 = all pass (or all manual), 1 = at
least one fail, 2 = invocation error. Landed in commit `68f94de`;
in-progress filter added in `a5b5fea` so failures on `in-progress`
specs are reported as IN-FLIGHT without gating the exit code.

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Check discovery | delivered |
| REQ-002 | CHK-ID filtering | delivered |
| REQ-003 | Shell execution and live streaming | delivered |
| REQ-004 | Exit code semantics | delivered |
| REQ-005 | Manual check rendering | delivered |
| REQ-006 | Output format and summary | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep.
