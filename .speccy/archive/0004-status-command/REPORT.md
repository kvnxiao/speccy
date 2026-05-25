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

<report spec="SPEC-0004">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-007 CHK-008">
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-009">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. Frontmatter status was already
`implemented`; only the REPORT.md gap remained.
