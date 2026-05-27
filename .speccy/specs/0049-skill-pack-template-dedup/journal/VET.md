---
spec: SPEC-0049
generated_at: 2026-05-27T19:28:49Z
---

## Invocation 1 — 2026-05-27T19:28:49Z

<drift-review verdict="pass" round="1" date="2026-05-27T19:28:49Z" model="claude-opus-4.7[1m]/high">
The diff satisfies SPEC-0049 as a unit. Source-side dedup removed verbatim retry-shape, reconcile-policy, and vet-phases bodies from non-canonical sites (`resources/modules/skills/speccy-orchestrate.md`, `resources/modules/skills/speccy-review.md`, `resources/modules/phases/speccy-work.md`, plus the two speccy-work wrapper templates); the new `resources/modules/skills/speccy-work.md` carries DEC-002 invariants inline; both speccy-work wrappers are now pure-include per DEC-001(a); `.claude/skills/speccy-vet/SKILL.md` and `.agents/skills/speccy-vet/SKILL.md` retain the full Phase 0/1/2/3 grammar (canonical-owner exception). Grep confirms zero non-canonical matches for the retry-shape sentence, the reconcile-policy table row, and the `### Phase 0 — bootstrap` heading across all ejected non-owner files. Hygiene gates pass; `speccy verify` exits 0 over the workspace. The T-004 test-file edits (`STUB_ONLY_PHASES` narrowing in `skill_stub_shape.rs`, `CLAUDE_STUB_DELEGATE_PHASES` in `init.rs`, removal of `work` from stub-cap iterations in `init_phase_agents.rs`, and removal from `PINNED_STUB_PHASES` in `skill_packs.rs`) are authorized by REQ-003 superseding the prior stub-delegate convention; they narrow rather than weaken. T-005's dogfood is self-dogfood per the SPEC's authorized fallback and honestly discloses that the reconcile branch was not exercised — accepted residual risk per the SPEC's "subjective comprehension" framing.
</drift-review>

<simplifier-scan verdict="clean">
The SPEC-0049 diff is a deduplication pass that replaces verbatim canonical bodies with summary-plus-pointer references; the remaining structure (split test loops with different phase lists, summary paragraphs in module bodies, justifying comments at each carveout site) is load-bearing for the new invariant and not worth collapsing.
</simplifier-scan>

<gate verdict="passed" tasks_hash="8b0af24e01f5327eab6de5aeb0a3c7222a912e90130d576c5b3320d9db5aaa3c" date="2026-05-27T19:34:25Z">
Drift cleared on round 1; simplifier scan clean; no fix rounds needed.
</gate>
