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

<report spec="SPEC-0010">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004 CHK-005">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-007">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-008">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep.
