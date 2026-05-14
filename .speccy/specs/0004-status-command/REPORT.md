---
spec: SPEC-0004
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0004 status-command

## Summary

`speccy status` workspace overview with text and JSON output,
staleness detection via spec hash mismatch, task-state aggregation,
supersession inverse, and lint integration. Default text view
filters to in-progress specs; `--all` includes every status. Landed
in commit `703ba4b`; lint-by-status filter refined in `cb9a4f0`.

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Workspace scan | delivered |
| REQ-002 | Staleness detection | delivered |
| REQ-003 | Task state aggregation | delivered |
| REQ-004 | Supersession inverse | delivered |
| REQ-005 | Lint integration | delivered |
| REQ-006 | Text view with default filter | delivered |
| REQ-007 | JSON output with stable schema | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. Frontmatter status was already
`implemented`; only the REPORT.md gap remained.
