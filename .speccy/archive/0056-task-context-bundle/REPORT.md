---
spec: SPEC-0056
outcome: implemented
generated_at: 2026-06-10T23:45:00Z
---

# REPORT: SPEC-0056 Task-scoped context bundle — `speccy context` emits one JSON read for loop subagents

<report spec="SPEC-0056">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-002 added `Command::Context { selector: String, json: bool }` to the clap enum in `speccy-cli/src/main.rs`, wired via `report_lookup_error` for `LookupError` → exit-code/diagnostic rendering at parity with `speccy check` (ambiguous, not-found, invalid-format). Selector failures exit non-zero with no partial stdout. Stdout parses as a single JSON document with `schema_version: 1` as the first serialized field, enforced by the struct ordering in `context_output.rs`. The command performs no writes. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
T-002 populated the identity slice (frontmatter `id`, `title`, `status`) and intent slice (`<goals>` body, `<non-goals>` body, all `<decision>` blocks with DEC ids) from `SpecDoc.goals` / `SpecDoc.non_goals` / `SpecDoc.decisions`. The Summary narrative, `<user-stories>`, Notes, and non-covered requirement bodies are excluded by construction. The integration test confirms goals, non-goals, and DEC ids are present while a Summary marker string is absent (CHK-003). Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004 CHK-005">
T-001 extracted the covers → requirements → scenarios walk from `check::run_task` into `speccy_core::context::resolve_covering_requirements`, consumed by both `speccy check` and `speccy context`. T-003 extended the envelope to carry the selected task's raw `<task>` body bytes plus parsed id, state, covers, and the covering requirements in full (heading, prose, `<done-when>`, `<behavior>`, scenarios). `speccy check`'s text output is byte-identical: all existing check integration tests pass unchanged (CHK-005). Retry count: 1 (T-003 required a second round to fix duplicate iteration over `covers` tokens in the initial envelope assembly).
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006 CHK-007">
T-004 inlined the full per-task journal via `journal_xml::parse` and projected each block through SPEC-0055's public `to_json_journal_block` (from `journal_show_output.rs`) — the same anti-drift discipline DEC-002 applies to check/context. The standalone `JsonTaskJournal` envelope's `schema_version` is not nested; only block structs plus frontmatter fields are reused. When the journal file is absent the envelope carries `exists: false` with zero blocks and exits 0. CHK-006 (8 blocks across 2 rounds) and CHK-007 (explicit absence marker) pass. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008">
T-005 added: (1) sibling-task index — every other task as id, state, covers only, validated by CHK-008 (5 entries, no sibling marker strings); (2) repo-relative paths to SPEC.md, TASKS.md, and the task's journal file; (3) a suggested merge-base diff command string using git's triple-dot form (`git diff <base>...HEAD`), computed via a new best-effort default-branch probe added to `speccy-cli/src/git.rs`. Git unavailability degrades the diff-command field without erroring the bundle. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-009">
T-006 extended the envelope with a consistency section carrying the workspace-level `ConsistencyStatus` (from `consistency::detect` via `ShellGitProbe`) plus only the `DriftEntry` items whose `task_id` matches the selected task. Drift never changes the exit code. CHK-009 confirms both drifted and undrifted bundles exit 0, carry the non-ok workspace status, and only the drifted bundle carries drift entries. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-010">
T-007 added a property-style integration test (`speccy-cli/tests/context.rs`) that emits a bundle, grows the fixture spec by one uncovered requirement plus one sibling task plus one foreign journal round, re-locks the spec hash, and re-emits — after normalizing consistency fields, the two payloads differ by exactly one added sibling-index entry and nothing else. The invariant is documented in `docs/ARCHITECTURE.md` as a contract. Retry count: 1 (T-007 required a second round to fix hash re-locking logic in the fixture growth helper).
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-011 CHK-012">
T-008 rewrote the entry-read step in `resources/modules/skills/partials/review-fanout.md` to dispatch a single `speccy context SPEC-NNNN/T-NNN --json` call referencing bundle fields (task, requirements, scenarios, journal, diff command). The `speccy-work` implementer phase was updated similarly. The six reviewer persona bodies and vet persona modules are byte-identical. The `reviewer-tests` `speccy check` exit-code caveat is intact. `just reeject` regenerated both host packs and produced a clean tree (CHK-011). Retry count: 0.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-013">
T-009 added the `speccy context` entry to the CLI surface section of `docs/ARCHITECTURE.md` (selector grammar, all envelope sections, schema_version contract, size invariant as a contract). The persona read-contract prose was updated to describe the bundle entry read; no section still presents full-file entry reads as the current contract. Retry count: 1 (T-009 required a second round to update a stale persona entry-reads subsection missed in the first pass).
</coverage>

</report>
