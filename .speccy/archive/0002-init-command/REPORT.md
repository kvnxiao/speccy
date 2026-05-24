---
spec: SPEC-0002
outcome: delivered
generated_at: 2026-05-13T20:00:00Z
---

# Report: SPEC-0002 init-command

## Summary

`speccy init [--host <name>] [--force]` takes a fresh repo from
`cd new-repo` to a working speccy workspace. Embedded skill bundle
via `include_dir!` means no network access at init time. Host
detection: `--host` wins; otherwise probes `.claude/` > `.codex/`,
refuses on `.cursor/`-only. `.speccy/skills/` mirrors shared
personas so project-local overrides can shadow the embedded bundle.
Landed in commit `5041dc0`.

<report spec="SPEC-0002">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004 CHK-005">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-006">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-009">
</coverage>

</report>

## Notes

REPORT.md back-filled retroactively on 2026-05-13 as part of the v1
dogfood completeness sweep. TASKS.md checkboxes were also
back-filled at this time; the landing commit was authored before
`/speccy-implement` orchestration drove the task state model.
