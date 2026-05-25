---
spec: SPEC-0041
outcome: implemented
generated_at: 2026-05-23T03:40:00Z
---

# REPORT: SPEC-0041 Vet lifecycle step — `speccy next` returns `kind="vet"` between completed tasks and ship, driven by a renamed `/speccy-vet` skill

<report spec="SPEC-0041">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 added `NextAction::Vet` to `speccy_core::next::NextAction`, sitting between
`Work` and `Ship` in the enum declaration. `speccy-cli/src/next_output.rs` gained a
`Vet` match arm in `to_json_action` emitting `"kind": "vet"` with no additional
fields, and a matching arm in `render_text_per_spec` printing the human-readable
`vet` verb consistent with the existing `work` / `review` / `ship` style. The
module-level doc comment and `compute_for_spec` function doc were updated to list
the new six-step priority rule. Integration tests in
`speccy-cli/tests/next_json.rs` drive the real `speccy next --json` binary against
workspace fixtures and assert `"kind":"vet"` for the all-completed / no-VET.md
case (CHK-001) and `"kind":"ship"` for the fresh-pass VET.md case (CHK-002). All
four hygiene gates passed clean. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-001 updated `compute_for_spec` in `speccy-core/src/next.rs` to add a freshness
check after confirming all tasks are `state="completed"`: it reads
`<spec-dir>/journal/VET.md`, extracts the last `<gate>` block via a tolerant
scanner, and returns `Vet` when the artifact is absent, ends with a failed verdict,
or carries a `tasks_hash` that does not match the current TASKS.md SHA-256.
Only a matching passing block advances to `Ship` (REPORT.md absent) or `None`
(REPORT.md present). Six integration tests in `speccy-core/tests/next_priority.rs`
cover every transition: absent VET.md → Vet; failed verdict → Vet; stale hash →
Vet (CHK-004); fresh-pass + no REPORT.md → Ship; fresh-pass + REPORT.md → None;
one in-review task beats a fresh-pass VET.md (CHK-003). Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
T-004 added a "Phase 3 — write `<gate>` block" section to
`resources/modules/skills/speccy-vet.md` immediately before the "Return contract"
section and propagated it verbatim to `.claude/skills/speccy-vet/SKILL.md` and
`.agents/skills/speccy-vet/SKILL.md`. Every exit path — Phase 0 integrity
failures, Phase 1 round-budget exhaustion, Phase 1 `stuck` reverts, Phase 2
completion, and the success path — appends exactly one `<gate verdict="passed|failed"
tasks_hash="..." date="...">` block to the current `## Invocation N` section of
VET.md. The block is appended after any `<drift-review>` / `<holistic-fix>` /
`<simplifier-scan>` / `<simplifier-apply>` blocks. POSIX (`sha256sum | awk`) and
PowerShell (`Get-FileHash`) hash recipes are both spelled out. The gate block
written by `/speccy-vet` on the SPEC-0041 dogfood run is recorded in
`.speccy/specs/0041-vet-lifecycle-step/journal/VET.md` with a verified
`tasks_hash`. `docs/ARCHITECTURE.md` gained a new "VET.md per-SPEC journal"
section documenting the full sub-agent verdict block family plus the `<gate>` block
grammar and resolver semantics. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
T-002 renamed all five skill / template directory trees via `git mv` (`speccy-holistic-gate`
→ `speccy-vet` in `resources/modules/skills/`, `.claude/skills/`, `.agents/skills/`,
`resources/agents/.claude/skills/`, and `resources/agents/.agents/skills/`) and
renamed `.speccy/specs/0038-skill-pack-references/journal/HOLISTIC.md` →
`VET.md`. A global token sweep across 25 non-historical files replaced every
`speccy-holistic-gate` and `HOLISTIC.md` occurrence. YAML `name:` frontmatter in
the renamed skill bodies updated to `speccy-vet`. `speccy-cli/tests/skill_packs.rs`
now asserts `speccy-vet/` instead of `speccy-holistic-gate/`. Drift detected at vet
time that `speccy-orchestrate` lacked a `VET.md` mention (REQ-004 done-when 3
required at least one match) was fixed by the vet-implementer sub-agent in round 1,
adding the VET.md parenthetical to the orchestrator body and re-ejecting both
copies. Post-fix `rg` scans confirm zero matches for either prohibited token in the
search scope. `ls .claude/skills/speccy-holistic-gate/ 2>&1` exits non-zero.
Retry count: 1 (one vet round for the orchestrate VET.md mention).
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-009 CHK-010 CHK-011">
T-003 renamed all ten persona/agent files via `git mv` — `holistic-reviewer` →
`vet-reviewer` and `holistic-implementer` → `vet-implementer` across
`resources/modules/personas/`, `.claude/agents/`, `.codex/agents/`, and both
`resources/agents/` host-pack template subtrees — and updated the `name:` fields in
each ejected copy. T-003 also created the new speccy-owned `vet-simplifier` persona:
`resources/modules/personas/vet-simplifier.md` carries a five-point body (Preserve
Functionality / Apply Project Standards / Enhance Clarity / Maintain Balance / Phase
2 scope boundary) adapted from the upstream `code-simplifier` template, with
"Apply Project Standards" pointing at the host project's `AGENTS.md` and
`.claude/rules/`-equivalent rule files and "Focus Scope" tightened to "the
cumulative SPEC-NNNN diff against the merge base". Ejected copies at
`.claude/agents/vet-simplifier.md` and `.codex/agents/vet-simplifier.toml` plus
host-pack templates were created. Phase 2's dispatch site in `speccy-vet.md` and
both ejected copies changed from `code-simplifier:code-simplifier` (Claude Code)
/ `code-simplifier` (Codex) to `vet-simplifier` on both hosts, closing the
cross-host parity gap. Post-task `rg` scans confirm zero `holistic-reviewer|holistic-implementer`
matches in scope and zero `code-simplifier` matches in the three speccy-vet skill
bodies. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-012 CHK-013 CHK-014">
T-005 updated the closing suggestion in `resources/modules/skills/speccy-review.md`
and its ejected copies from `/speccy-ship SPEC-NNNN` to `/speccy-vet SPEC-NNNN`,
and likewise in `resources/modules/phases/speccy-work.md` and its ejected copies
(`.claude/agents/speccy-work.md`, `.codex/agents/speccy-work.toml`). The
pointer-only stub files `.claude/skills/speccy-work/SKILL.md` and
`.agents/skills/speccy-work/SKILL.md` were correctly left untouched per the
SPEC-0023 pinned-stub convention. A "Next step after exit" section was added to
`resources/modules/skills/speccy-vet.md` and both ejected copies suggesting
`/speccy-ship SPEC-NNNN` after a passing gate verdict. Post-task `rg` checks
confirm zero `/speccy-ship SPEC-NNNN` mentions in `speccy-review.md` and
`speccy-work.md`, at least one `speccy-vet SPEC-NNNN` mention in each, and at
least one `/speccy-ship SPEC-NNNN` match in `speccy-vet.md` (the post-pass chain).
Retry count: 0.
</coverage>

</report>
