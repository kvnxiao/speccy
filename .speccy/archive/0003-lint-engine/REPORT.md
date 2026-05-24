---
spec: SPEC-0003
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0003 lint-engine

## Summary

Structured-diagnostic lint engine emitting the SPC / REQ / VAL / TSK
/ QST code families described in ARCHITECTURE.md, including the
single-no-op-command anti-pattern (VAL-002). Landed in commit
`cbc0eaa`; consumed by both `speccy status` (SPEC-0004) and
`speccy verify` (SPEC-0012).

<report spec="SPEC-0003">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-009">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-010">
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-011">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep.
