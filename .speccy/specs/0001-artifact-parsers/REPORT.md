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

## Requirements coverage

| Requirement | Title | Status |
|---|---|---|
| REQ-001 | TOML config parsers | delivered |
| REQ-002 | Frontmatter extraction | delivered |
| REQ-003 | SPEC.md parsing | delivered |
| REQ-004 | TASKS.md parsing | delivered |
| REQ-005 | REPORT.md frontmatter parsing | delivered |
| REQ-006 | Cross-reference SPEC.md against spec.toml | delivered |
| REQ-007 | Public API and hygiene for speccy-core | delivered |
| REQ-008 | Supersession index across the workspace | delivered |

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. No retry counts: this bootstrap spec was
implemented before `/speccy-implement` orchestration existed.
