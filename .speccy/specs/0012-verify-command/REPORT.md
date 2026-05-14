---
spec: SPEC-0012
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0012 verify-command

## Summary

`speccy verify` is the CI gate: it runs lint, executes every
`command:` check, and exits with a binary code (0 = clean,
non-zero = drift detected). Lint errors on `in-progress` specs are
demoted to informational rather than gating, so CI stays green on
in-flight work. Landed in commit `0d2faad`; lint-by-status filter
added in `cb9a4f0`.

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Lint integration | delivered |
| REQ-002 | Check execution | delivered |
| REQ-003 | Binary exit code | delivered |
| REQ-004 | Text mode summary | delivered |
| REQ-005 | JSON output | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep.
