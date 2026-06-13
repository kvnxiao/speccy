---
spec: SPEC-0063
generated_at: 2026-06-12T22:38:11Z
---

## Invocation 1 — 2026-06-12T22:38:11Z

<drift-review verdict="blocking" round="1" date="2026-06-12T22:38:11Z" model="claude-opus-4-8[1m]">
Diff satisfies REQ-002 through REQ-007 as a unit; one unresolvable internal SPEC contradiction blocks REQ-001 and needs a human SPEC reconciliation, not another retry round.
- REQ-001 (`<done-when>` bullet 1: every Track-1-enumerated passage "no longer appears") vs REQ-006 (suite stays green) → the A6/C6 instruction in T-001 to delete the `> Ported from the feature-dev …` provenance lines is unapplied and unappliable: those lines are the only `feature-dev` string in the three bodies, and deleting them fails the SPEC-0053 CHK-006 guardrail that requires it. The two requirements are in direct, unresolvable conflict for these three lines. The implementer correctly kept them and surfaced the tension (Core Principle #6); this needs a human to amend REQ-001's A6/C6 scope or the SPEC-0053 attribution test, not a retry. See resources/modules/personas/reviewer-correctness.md, resources/modules/personas/plan-architect.md, resources/modules/personas/plan-explorer.md (all still carry "Ported from") and speccy-cli/tests/skill_packs.rs:1793 (the asserting guardrail).
</drift-review>
<holistic-fix verdict="stuck" round="1" date="2026-06-12T22:41:06Z" model="claude-opus-4-8[1m]">
Summary: The sole blocking finding is a genuine SPEC-internal contradiction; no code-only fix exists that does not either break a hard hygiene gate or silently retire another spec's guardrail without authorization. Returning stuck for a human SPEC reconciliation.

Not addressed:
- REQ-001 (T-001 A6/C6: delete the `> Ported from the feature-dev …` provenance lines from personas/reviewer-correctness.md:4, personas/plan-architect.md:3, personas/plan-explorer.md:3) vs REQ-006/CHK-008 (`cargo test --workspace` stays green) → unresolvable by code. Those provenance lines are the only `feature-dev` string in each of the three bodies, and `speccy-cli/tests/skill_packs.rs:1792` (`feature_dev_personas_declare_speccy_model_conventions_and_attribution`, a SPEC-0053 CHK-006 guardrail) asserts each body must contain `feature-dev`. The two code-fix paths both violate the SPEC: (a) applying A6/C6 fails that guardrail → breaks REQ-006/CHK-008, a hard done-when and a hard hygiene gate I am contractually forbidden to break; (b) deleting/relaxing the skill_packs.rs assertion so the suite stays green is unauthorized — SPEC-0063 `supersedes: []` lists nothing for SPEC-0053, it supersedes only SPEC-0034 DEC-001, and REQ-001's done-when enumerates only persona_snippets.rs and authoring_commit.rs as the tests whose assertions must stay intact, never naming skill_packs.rs as relaxable. Picking either side is a SPEC decision, not an implementation detail.

Side discoveries:
- The T-001 implementer already surfaced this exact tension (journal/T-001.md round 1, Discovered issues #1) and correctly kept the provenance lines under Core Principle #6; the working tree is in the right state — the conflict is purely at the SPEC layer.
- Resolution is a /speccy-amend, not a retry: either narrow REQ-001's A6/C6 anchor scope to exclude these three provenance lines (keep attribution), or explicitly authorize retiring the SPEC-0053 CHK-006 attribution assertion in skill_packs.rs. A human must choose which contract gives.
</holistic-fix>
<gate verdict="failed" tasks_hash="dbeaca3fb2c0306779999a2d7ab7e2df3a536c0ccf081512bc0d848533a69072" date="2026-06-12T22:41:42Z">
Round 1 drift review blocking on a SPEC-internal contradiction; implementer returned stuck. REQ-001 A6/C6 (delete the feature-dev provenance lines) conflicts with REQ-006's SPEC-0053 attribution guardrail (skill_packs.rs:1793 asserts those lines exist). Needs /speccy-amend, not a code fix.
</gate>
## Invocation 2 — 2026-06-13T03:19:34Z

<drift-review verdict="pass" round="1" date="2026-06-13T03:19:34Z" model="claude-opus-4-8[1m]">
Invocation-1 contradiction fully resolved: the DEC-002/REQ-008 amendment retired the brittle feature-dev substring guardrail and T-003 completed the deferred A6/C6 provenance-line deletion; the full branch diff now coheres against the amended SPEC, with all four hygiene gates green and no scope creep.

Verified each requirement against the working tree (git diff main, clean tree, T-001/T-002/T-003 committed):
- REQ-008/REQ-001 (the round-1 blocker): the `persona_body.contains("feature-dev")` assertion and its `read_persona` binding are gone, the test is renamed to `feature_dev_personas_declare_speccy_model_conventions`, and the `model: opus[1m]` / `model = "gpt-5.5"` / no-`sonnet` structural assertions are retained; `read_persona` is still used by other tests so no dangling helper. `grep -rn 'Ported from' resources/ .claude/ .agents/ .codex/` returns nothing — all three A6/C6 provenance blockquotes are gone from source and eject.
- REQ-002/REQ-003: work phase post-append re-read replaced with validate-before-write reliance + `speccy next --json` (speccy-work.md:362); ship phase carries only the negated "no separate `speccy status` call is needed".
- REQ-004: six factored modules each `{% include %}`d at exactly the enumerated callsites (retry-shape-summary ×3, review-role-tail ×7, vet-input-resolution ×2, vet-no-rollback ×2, inline-fanout-rationale ×3, spec-self-review-core ×2 plan+amend); retry-shape-summary expands to the full inline sentence plus pointer (act-without-read preserved, SPEC-0049 DEC-002).
- REQ-005: supersession comment present in spec-self-review-core.md:1; speccy-brainstorm retains independent inline self-review (0 includes of the core); no DEC-001/OQ-b stale comments remain in plan/amend/brainstorm.
- REQ-006: cargo test --workspace, clippy -D warnings, +nightly fmt --check, and cargo deny check all pass (the lone deny output is a pre-existing MPL-2.0 unmatched-allowance warning, exit 0).
- REQ-007: no `{%` marker survives in any ejected file; the .codex eject for reviewer-correctness mirrors source (blockquote deleted, role-tail include expanded, cli-stamps compressed) — include-expansion + prose-compression, no unexplained change.
- Scope: only `skill_packs.rs` changed under Rust (the single REQ-008-authorized edit); no production code, no new CLI command/env var/git-mutating helper — non-goals respected.
- REPORT.md is not yet present, but every audit-recording `<done-when>` bullet is explicitly "at ship time" per all three task close-outs; the pre-ship vet gate runs before that, so its absence is not drift.
</drift-review>
<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates worth applying.

The diff is a prose-factoring + determinism-trim + test-hygiene change set; the conventional code touched is a single vacuous-test retirement (speccy-cli/tests/skill_packs.rs). Reviewed:

- Four new shared modules (retry-shape-summary, spec-self-review-core, review-role-tail, vet-input-resolution, vet-no-rollback, inline-fanout-rationale) each have 2+ genuine callsites — correct dedup, verified no shadow inline copies remain via grep sweep.
- The two work/ship determinism trims remove redundant CLI re-check calls the SPEC deliberately retires — behavior-preserving by design, not simplifier scope.
- evidence.md's 37-line delta is a canonical-example trim (Scenario 3 removed); the module file and both .tmpl wrappers stay intact — no orphan.
- Persona prose compressions preserve every load-bearing distinction (diff-side caution, fabrication patterns, scope exclusions).

Further prose compression would be editorial churn; AGENTS.md explicitly gates prose on content not size, so it is out of simplifier scope.
</simplifier-scan>
<gate verdict="passed" tasks_hash="a700d100b5b4bb9b21aa85d3de6bc330c7979eb0a53987e3f4b1d9a95d7c5d11" date="2026-06-13T03:21:17Z">
Drift cleared on invocation 2 round 1 after the DEC-002/REQ-008 amendment and T-003 retired the brittle feature-dev guardrail; simplifier clean. No drift.
</gate>
