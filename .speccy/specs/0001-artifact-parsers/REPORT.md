---
spec: SPEC-0001
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0001 artifact-parsers

## Summary

`speccy-core` parsers for every Speccy artifact (`speccy.toml`,
`spec.toml`, `SPEC.md`, `TASKS.md`, `REPORT.md`) plus cross-reference
and supersession-index helpers. Landed in commit `66f5459` on
2026-05-11; CHK-paths corrected in `131c5de` once the workspace
flattened to root-level crates.

<report spec="SPEC-0001">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-007">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004 CHK-005">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-009">
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-010 CHK-011">
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-012">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. No retry counts: this bootstrap spec was
implemented before `/speccy-implement` orchestration existed.
