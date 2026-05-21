---
spec: SPEC-0036
outcome: implemented
generated_at: 2026-05-21T00:00:00Z
---

# REPORT: SPEC-0036 Repin Claude Code speccy-work implementer to opus[1m] / low effort

<report spec="SPEC-0036">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
Commit `3e72382` swaps `model: sonnet[1m]` to `model: opus[1m]` and
`effort: medium` to `effort: low` in both
`.claude/agents/speccy-work.md` and
`resources/agents/.claude/agents/speccy-work.md.tmpl`. The body
content (rendered include text and the `{% include %}` directive
respectively) is byte-unchanged. Two test files
(`speccy-cli/tests/init.rs` and
`speccy-cli/tests/init_phase_agents.rs`) that had uniform-loop
assertions encoding the old pin for all three mechanical phases were
refactored to per-phase `(phase, model, effort)` tuples so
`speccy-work` is validated at `opus[1m]`/`low` while `speccy-tasks`
and `speccy-ship` remain at `sonnet[1m]`/`medium`. The host-pack
drift-check meta-test (`dogfood_outputs_match_committed_tree`) exits 0
because template and rendered file move together. Red-then-green
evidence at `.speccy/specs/0036-speccy-work-opus-low-pin/evidence/T-001.md`.
CHK-001, CHK-002, CHK-003 satisfied. Retry count: 1 (round 1 blocked
on missing evidence file and uncommitted working tree; resolved in
commit `16a9493` then landed with commit `3e72382`).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
Commit `58a5fe9` makes two surgical edits to `README.md`: (1) the
"Pin assignment" table's `speccy-work` row Claude Code column updated
from `model: sonnet[1m]`, `effort: medium` to `model: opus[1m]`,
`effort: low`; (2) the "Overriding a pin" worked example updated so the
before-alias is `model: opus[1m]` and the lock target is
`model: claude-opus-4-7[1m]`. The Codex column, "Agent file ships?"
column, and all other ten table rows are byte-identical to the prior
commit. Grep for `speccy-work.*sonnet[1m]` returns zero matches.
Evidence at `.speccy/specs/0036-speccy-work-opus-low-pin/evidence/T-002.md`.
CHK-004, CHK-005, CHK-006 satisfied. Retry count: 0.
</coverage>

</report>
