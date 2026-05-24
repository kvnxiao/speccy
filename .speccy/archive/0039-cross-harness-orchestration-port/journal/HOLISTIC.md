---
spec: SPEC-0039
generated_at: 2026-05-22T22:24:56Z
---

## Invocation 1 — 2026-05-22T22:24:56Z

<drift-review verdict="blocking" round="1" date="2026-05-22T22:30:00-04:00" model="claude-opus-4.7[1m]/high">
SPEC-0039 module factoring lands cleanly but stale working-tree artifacts break the `cargo test --workspace` hygiene gate that REQ-004/T-006 explicitly require, and the SPEC↔TASKS hash is stale after the REQ-006 amendment.
- REQ-004 / T-006 task scenario "`cargo test --workspace` exits 0" → fails: `speccy-cli::verify_after_migration::speccy_verify_exits_zero_on_migrated_in_tree_workspace` panics because `speccy verify` returns SPC-001/SPC-004 errors against an empty `.speccy/specs/0041-vet-lifecycle-step/` directory in the working tree. See `speccy-cli/tests/verify_after_migration.rs:38` and the empty dir at `.speccy/specs/0041-vet-lifecycle-step/`.
- SPEC-0039 changelog row 2 (REQ-006 amendment) → SPEC.md was edited adding REQ-006 + Goal bullet + Summary paragraph but TASKS.md `spec_hash_at_generation` was never re-recorded; `speccy verify` reports TSK-003 stale-tasks warning (current SPEC sha256 `1513c6e…` vs stored `9d09b01…`). The standard convention is to run `/speccy-amend` so the recorded hash matches the post-amendment SPEC. See `.speccy/specs/0039-cross-harness-orchestration-port/TASKS.md:3`.
- Scope creep beyond SPEC-0039 → working tree carries an untracked unrelated SPEC scaffold at `.speccy/specs/0040-retire-speccy-toml/` and an empty `.speccy/specs/0041-vet-lifecycle-step/`. Neither is authorized by SPEC-0039 (Summary, Goals, or any Requirement) and the latter is the direct cause of the failing hygiene test above. See `.speccy/specs/0040-retire-speccy-toml/` and `.speccy/specs/0041-vet-lifecycle-step/`.
- REQ-001 "Phase 2 simplifier polish" mechanism is a hidden plugin dependency → `resources/modules/skills/speccy-holistic-gate.md:362` and `:417` dispatch to `subagent_type: "code-simplifier:code-simplifier"`, a plugin-namespaced sub-agent that Speccy itself does not ship in either `resources/agents/.claude/agents/` or `resources/agents/.codex/agents/`. The SPEC does not authorize a non-shipped runtime dependency for a shipped skill; on a user's box without that plugin the simplifier phase will fail rather than degrade. Either ship a Speccy-owned simplifier sub-agent, or rewrite Phase 2 to a degradable form (skip when the sub-agent is absent) and note the optionality explicitly. See `resources/modules/skills/speccy-holistic-gate.md:361-363` and `:416-418`.
</drift-review>

<orchestrator-triage date="2026-05-22T22:35:00Z" actor="user">
User triaged the four findings before any implementer dispatch. Only finding #2 (SPEC-0039 TASKS.md hash stale post-REQ-006 amendment) is legitimate SPEC-0039 drift; the remaining three are out of scope:
- Finding #1 (`cargo test --workspace` failing) is *caused by* finding #3, not by SPEC-0039 code.
- Finding #3 (`.speccy/specs/0040-retire-speccy-toml/` and `.speccy/specs/0041-vet-lifecycle-step/`) is user's parallel WIP on future specs sitting untracked in the working tree, not SPEC-0039 scope. Destructive cleanup would lose in-progress work.
- Finding #4 (`code-simplifier:code-simplifier` plugin sub-agent) is a pre-existing pattern carried over from the `.claude/skills/speccy-holistic-review/` pilot, not introduced by SPEC-0039's factoring diff. Rewriting Phase 2's mechanism is a separate SPEC, not a holistic-gate drift fix.

Also recorded: the round-1 drift-reviewer self-admitted running `git stash` for diagnosis, violating its read-only contract. Working tree was confirmed intact via `git status` before proceeding.
</orchestrator-triage>

<holistic-fix verdict="addressed" round="1" date="2026-05-22T22:35:00Z" model="claude-opus-4.7[1m]" actor="orchestrator-direct">
Applied finding #2 surgically (no implementer sub-agent dispatch). Updated `.speccy/specs/0039-cross-harness-orchestration-port/TASKS.md:3` `spec_hash_at_generation` from `9d09b01a13c5a8de26fce4a4b65de01646b3bfc367ec733b7251c1ddbb6bee98` to `168f69a88535da4cdda603f24e6c488daaa5bead0b69f82f15238c9f33cc01dc` (current SPEC.md sha256). Confirmed: `speccy verify` reports `0 errors, 2 warnings, 60 info` and TSK-003 no longer fires against SPEC-0039 (the two remaining warnings target SPEC-0040/0041, user WIP).

Deferred per user direction:
- Findings #1, #3, #4 left as-is; will be addressed in follow-up specs or after user finalizes SPEC-0040/0041.
- `cargo test --workspace` remains red until user moves or finalizes the SPEC-0040/0041 SPEC.md files (currently malformed: missing blank-line-after-close-tag, SPEC-0041 missing `<changelog>`).
</holistic-fix>

<simplifier-scan verdict="skipped">
Phase 2 simplifier polish not run — drift loop exited with deferred findings rather than a clean `pass`, so the polish phase is not appropriate.
</simplifier-scan>
