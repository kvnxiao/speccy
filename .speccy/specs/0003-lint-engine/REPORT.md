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

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | SPC-* lint codes (spec structure) | delivered |
| REQ-002 | REQ-* lint codes (requirement coverage) | delivered |
| REQ-003 | VAL-* lint codes (check definitions) | delivered |
| REQ-004 | TSK-* lint codes (task structure) | delivered |
| REQ-005 | QST-001 lint code (open questions) | delivered |
| REQ-006 | Public API and lint::run entry point | delivered |
| REQ-007 | Lint code stability contract | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep.
