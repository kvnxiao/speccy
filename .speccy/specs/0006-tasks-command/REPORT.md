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

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | Initial prompt rendering | delivered |
| REQ-002 | Amendment prompt rendering | delivered |
| REQ-003 | --commit records spec hash and timestamp | delivered |
| REQ-004 | --commit preserves TASKS.md body bytes | delivered |
| REQ-005 | SPEC-ID argument validation and state checks | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes also back-filled.
