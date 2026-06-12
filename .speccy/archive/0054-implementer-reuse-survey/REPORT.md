---
spec: SPEC-0054
outcome: implemented
generated_at: 2026-05-31T17:00:00Z
---

# REPORT: SPEC-0054 Implementer pre-implementation reuse survey, forked reuse guidance, and `speccy-work` opus/high repin

<report spec="SPEC-0054">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-002 inserted a numbered bounded-reuse-survey step in
`resources/modules/phases/speccy-work.md` between the read-scenarios
step and the implement step. The step scopes the survey to the task's
area (covered REQs, suggested-files hint, and immediate module /
neighbouring files), explicitly states it is not a whole-repo scan,
defines the three tiers (reuse-as-is / extend / write-fresh), and
states the per-symbol floor as round-agnostic while restricting the
full area-map to round-1 (or a reuse-blocker-triggered retry). The
step includes the implementer reuse variant module.
`just reeject` and `cargo test --workspace` both exited 0. Retry count: 2
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-003 added a `Reuse survey` field to the handoff template in
`resources/modules/references/journal-implementer.md`, renaming the
heading from "Six-field" to "Seven-field" and adding the field to the
worked `<implementer>` example block with round-1-versus-retry
semantics stated (round-1 records the full survey; a retry round that
adds no new symbol records "unchanged — no new symbols, no reuse
blocker"). The inline field roll-call in
`resources/modules/phases/speccy-work.md` was updated to match. No
`speccy-core` parser or lint file was modified. `just reeject` and
`cargo test --workspace` both exited 0. Retry count: 0
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
T-002 created `resources/modules/references/reuse-survey-implementer.md`
(implementer survey-and-build variant) and
`resources/modules/references/reuse-hunt-reviewer.md` (reviewer
adversarially-verify-and-hunt variant). The "Reuse over reinvent" bullet
was removed from `resources/modules/references/convention-checklist.md`;
its other four items (match-local-conventions, docs-match-code,
no-false-complexity, re-apply-hard-rules) remain unchanged. The
implementer variant is included by the `speccy-work` phase; the
reviewer variant is included by `reviewer-style` under its "What to
look for that's easy to miss" section. `just reeject` and
`cargo test --workspace` both exited 0. Retry count: 2
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
T-001 changed `effort: low` to `effort: high` in
`resources/agents/.claude/agents/speccy-work.md.tmpl` (model
`opus[1m]` unchanged) and updated the "Pin assignment" table row in
`README.md`. `just reeject` regenerated `.claude/agents/speccy-work.md`
to `effort: high`; a second `just reeject` left `git status --porcelain`
empty. `cargo test --workspace` (including `speccy-cli/tests/pin_shape.rs`)
exited 0 — `high` was already in the Opus allow-set. The Codex template
`resources/agents/.codex/agents/speccy-work.toml.tmpl` was left
unchanged. CHK-008 (dogfooding: the next `speccy-work` journal records
`/high`) is judgment-only and post-ship. Retry count: 0
</coverage>

</report>

## Notes

Two unrelated tooling commits rode in on this branch outside any SPEC-0054
requirement: commit `f9b20a4` (Fix speccy-decompose: comma-`covers` is a
parse error, not a TSK-004 lint) and commit `88189b8` (Bump vet-implementer
Claude Code pin from `effort: low` to `effort: high`). Both are correct and
intentional. They are noted here for the record; they are not SPEC-0054
deliverables and their coverage belongs to no REQ.
