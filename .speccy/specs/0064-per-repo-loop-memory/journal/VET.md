---
spec: SPEC-0064
generated_at: 2026-06-13T05:25:26Z
---

## Invocation 1 — 2026-06-13T05:25:26Z

<drift-review verdict="blocking" round="1" date="2026-06-13T05:25:26Z" model="claude-opus-4-8[1m]">
One concrete drift: the ship-time retro mines the wrong git-diff form, so the evidence it is required to distill is empty at the point in the recipe where it runs. Everything else satisfies the SPEC as a unit.

- REQ-004 (retro "mines the evidence already on disk — ... and the spec diff") → the retro step instructs `git diff origin/main...HEAD`, but it runs at step 3, before the step-6 ship commit, where the loop's per-task work is still uncommitted in the working tree (the work phase never commits per-task; implementers leave changes uncommitted between phases). The three-dot `...HEAD` form compares the merge-base against committed HEAD and silently misses all uncommitted loop work — so the retro is handed an empty or stale diff exactly when it must mine the just-completed loop. The repo already documents the correct form (`git diff <base-ref>`, two-dot, "the `...HEAD` form would silently miss [uncommitted changes]"). See resources/modules/phases/speccy-ship.md:55, .claude/agents/speccy-ship.md:59, .codex/agents/speccy-ship.toml (same rendered text), and the documented convention at resources/modules/personas/vet-input-resolution.md:1-5.
</drift-review>
<holistic-fix verdict="addressed" round="1" date="2026-06-13T05:28:36Z" model="claude-opus-4-8[1m]">
Summary: Fixed the ship-time retro's spec-diff form so it mines the uncommitted loop work it is required to distill.

Addressed:
- REQ-004 retro mining the wrong git-diff form (three-dot `git diff origin/main...HEAD` at step 3, before the step-6 ship commit, silently missing uncommitted loop work) → changed the canonical module `resources/modules/phases/speccy-ship.md:55-61` to the two-dot `git diff origin/main` and added a sentence explaining why the two-dot form is required here (retro runs before the ship commit while per-task work is uncommitted; three-dot compares merge-base against committed HEAD and misses it). Re-ejected via `just reeject`; the corrected text now lands in `.claude/agents/speccy-ship.md:57-66` and `.codex/agents/speccy-ship.toml` (1 overwritten, rest unchanged). The remaining `origin/main...HEAD` occurrences in the tree are unrelated and correct in context: `speccy-cli/tests/context.rs` exercises the task-context-bundle `suggested_diff_command` (a different feature), and journal/archive entries are historical records.

Hygiene: all four gates green — `cargo test --workspace` (no failures), `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean), `cargo +nightly fmt --all --check` (exit 0), `cargo deny check` (advisories/bans/licenses/sources ok).

Side discoveries:
- None. The drift reviewer noted everything else satisfies the SPEC as a unit; this was the only concrete drift and it was a single-line prose correction plus reeject.
</holistic-fix>
<drift-review verdict="pass" round="2" date="2026-06-13T05:31:16Z" model="claude-opus-4-8[1m]">
Round-1 drift resolved and the diff satisfies SPEC-0064 as a unit. The retro's git-diff form is now the two-dot `git diff origin/main` in the canonical ship module (resources/modules/phases/speccy-ship.md:55-63) with a clear why-two-dot rationale, and both ejected hosts carry it (.claude/agents/speccy-ship.md:60-62, .codex/agents/speccy-ship.toml:60-62); `dogfood_outputs_match_committed_tree` confirms the eject is in sync. Fresh whole-spec pass found no new drift: REQ-001 (byte-identity + no-create tests in init.rs, ARCHITECTURE.md layout line), REQ-002 (four-part entry shape in the new memory-ledger.md reference), REQ-003 (implementer step 6 in speccy-work.md, single canonical include, CHK-004 structural test green, no wrapper inlines), REQ-004/005/006 (ship retro step 3 at the REPORT.md boundary, human-gated consolidation/dedup, phantom-GC), and REQ-007 (verify baseline-equality test) all hold. Scope is clean: no `speccy memory` CLI verb (DEC-008/non-goal), no reviewer/vet feed-forward read (DEC-005/non-goal), no new lint code, and resources changes confined to exactly the three files the SPEC's scope boundary names. Full `cargo test --workspace` green.
</drift-review>
<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates worth applying in the SPEC-0064 diff.

Reviewed surfaces:
- Rust tests (speccy-cli/tests/init.rs, memory_feedforward.rs, verify.rs): new test code is idiomatic and clear. The recursive collect_files walker and the workspace_root() map_or_else are standard; the two host-arms in force_preserves_speccy_memory_ledger (claude-code, codex) are genuine distinct coverage, not extractable duplication.
- Prose modules (resources/modules/references/memory-ledger.md, phases/speccy-work.md, phases/speccy-ship.md, docs/ARCHITECTURE.md): agent instruction bodies, not executable code. Per AGENTS.md, prose-edit acceptance is authoring judgment, not a simplifier gate. The verbose two-dot vs three-dot git-diff explanation in speccy-ship.md step 3 is load-bearing (guards a named stale/empty-diff failure mode); trimming it would risk the correctness it protects.
- Ejected wrappers under .claude/ and .codex/ are generated output, out of scope.
</simplifier-scan>
<gate verdict="passed" tasks_hash="4a9a8c91f341da0b0811c222b2ea0bf2fcb30fa57c0613cb262b046f6662e6a3" date="2026-06-13T05:32:29Z">
Drift cleared on round 2 (ship retro spec-diff changed from three-dot to two-dot so it captures uncommitted loop work); simplifier scan clean.
</gate>
