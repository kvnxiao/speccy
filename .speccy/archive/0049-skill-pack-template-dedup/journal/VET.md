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

## Invocation 2 — 2026-05-27T20:10:45Z

<drift-review verdict="pass" round="1" date="2026-05-27T20:10:45Z" model="claude-opus-4-7[1m]/high">
The diff satisfies SPEC-0049 as a unit including the REQ-006 amendment. Walked all six requirements against the working tree: REQ-001 (source-side dedup of retry-shape, reconcile-policy, vet-phases at non-canonical sites under `resources/`), REQ-002 (ejected slimming across `.claude/`, `.agents/`, `.codex/`), REQ-003 (wrapper pure-include vs stub-delegate), REQ-004 (hygiene gates), and REQ-005 (work-review-ship dogfood) were already cleared in invocation 1's passing gate, and the diff for those files is unchanged since — re-verified via grep that the retry-shape sentence, reconcile-policy table row, and `### Phase 0 — bootstrap` heading appear only in canonical/owner sites. REQ-006 (the post-gate amendment) is fully delivered: `resources/modules/skills/speccy-orchestrate.md` Loop step 2 enumerates five dispatch kinds with `vet` and `ship` as distinct bullets; the Lifecycle ASCII tree shows the split; the "Vet dispatch" section binds to `vet` and inlines the speccy-vet workflow with the pass/fail reaction; the new "Ship dispatch" section binds to `ship` and performs only user-confirm + speccy-ship spawn under host-conditional Jinja; Stop conditions enum lists `work, review, vet, ship, decompose`; status reporting examples carry the `→ vet` and `→ ready to ship` lines. Both ejected `.claude/skills/speccy-orchestrate/SKILL.md` and `.agents/skills/speccy-orchestrate/SKILL.md` regenerated via `just reeject` carry the post-amendment dispatch tree with correct host-conditional rendering (Task tool vs Codex sub-agent-spawn primitive; `AskUserQuestion` vs Codex user-prompt primitive). Pre-existing "holistic gate → ask the user" prose at Stop conditions is slightly stale relative to the new workflow (the ask now happens under Ship dispatch when CLI emits `ship` rather than on the vet pass verdict directly) but was explicitly flagged out-of-scope by T-006's tests reviewer; REQ-006's done-when does not enumerate this line for editing, and the structural dispatch tree it describes is correct in the enum below it. No scope creep observed — diff is confined to the orchestrator skill body and its ejected siblings as the amendment scoped. Hygiene gates reported green by implementer.
</drift-review>

<simplifier-scan verdict="candidates">
Two stale-wording residues from the T-006 amendment: prose still says rules are "inlined" where they are now pointer summaries, and the Stop conditions list still describes the pre-amendment ship-confirm flow.

- `resources/modules/skills/speccy-orchestrate.md` Work-dispatch step 2 says "apply the retry-shape rule **inlined immediately below** from `{{ speccy_references_path }}/retry-shape.md`." Post-amendment, what follows is a pointer summary, not an inline. Replace with "summarized immediately below (canonical statement at `{{ speccy_references_path }}/retry-shape.md`)." Pure wording fix; matches the parallel fix already applied to the reconcile-policy callsites and to `resources/modules/phases/speccy-work.md` step 2.
- `resources/modules/skills/speccy-orchestrate.md` Stop conditions still reads: `verdict="pass"` from the holistic gate → ask the user before invoking ship.` After the REQ-006 split, vet's `pass` no longer stops the loop and the user-ask now lives in Ship dispatch (the loop iteration re-queries, the next `next_action.kind == "ship"` routes to Ship dispatch, which is the site that asks). Rewrite to `ship` dispatch declined by the user → stop the outer loop.
</simplifier-scan>

<simplifier-apply verdict="applied">
Applied both wording fixes to `resources/modules/skills/speccy-orchestrate.md`, re-ejected both host packs (overwrote `.claude/skills/speccy-orchestrate/SKILL.md` and `.agents/skills/speccy-orchestrate/SKILL.md`), and all four hygiene gates pass.
</simplifier-apply>

<gate verdict="passed" tasks_hash="b0a5bea31c6f2e3762eda391a58e7119884f721a3d1ce8323d3c139ee536d196" date="2026-05-27T20:15:21Z">
Drift cleared on round 1; simplifier applied two post-amendment stale-wording fixes; hygiene gates green.
</gate>
