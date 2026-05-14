---
spec: SPEC-0011
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0011 report-command

## Summary

`speccy report SPEC-ID` renders the Phase 5 report prompt. Refuses
when any task is not `[x]`, derives a per-task retry count from
inline `Retry:` notes, and inlines SPEC.md, TASKS.md, AGENTS.md, and
the retry summary into the embedded `report.md` template before
budget trimming. Landed in commit `6a4ee36`.

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | SPEC-ID validation and spec lookup | delivered |
| REQ-002 | Completeness gate | delivered |
| REQ-003 | Retry count computation per task | delivered |
| REQ-004 | Render report prompt | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. Meta-observation: this is the spec that
defines the REPORT.md format; the back-filled REPORT.md is the
minimal shape its own template prescribes.
