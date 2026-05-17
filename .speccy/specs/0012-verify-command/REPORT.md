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

<report spec="SPEC-0012">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002 CHK-003">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006 CHK-007">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep.
