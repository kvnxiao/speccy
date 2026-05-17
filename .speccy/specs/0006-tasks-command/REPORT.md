---
spec: SPEC-0006
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0006 tasks-command

## Summary

`speccy tasks SPEC-ID [--commit]` renders either the initial
(TASKS.md absent) or amendment (TASKS.md present) Phase 2 prompt;
the `--commit` sub-action performs a body-byte-preserving frontmatter
rewrite that records the SPEC.md sha256 + `generated_at` timestamp.
Landed in commit `727f48f`.

<report spec="SPEC-0006">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003 CHK-004 CHK-005">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-007 CHK-008">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
