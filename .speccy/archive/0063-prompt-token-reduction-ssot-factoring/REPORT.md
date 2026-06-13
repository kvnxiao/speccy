---
spec: SPEC-0063
outcome: implemented
generated_at: 2026-06-13T04:00:00Z
---

# REPORT: SPEC-0063 Prompt token reduction + SSOT factoring for `resources/` modules — compress SDLC-loop prose, defer two procedures to existing CLI guarantees, then factor duplicated passages into shared modules

<report spec="SPEC-0063">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 (Track 1) compressed Tier A–E passages across `resources/modules/**`, removing
~332 net source lines of redundant restatements and over-explanation while preserving
each operational fact once at its canonical site. The three `> Ported from the
feature-dev …` provenance blockquotes (Track 1 anchors A6/C6 in
`reviewer-correctness.md`, `plan-architect.md`, `plan-explorer.md`) were deferred from
T-001 to T-003 due to the REQ-001/REQ-006 contradiction caught by the pre-ship vet gate
(invocation 1); T-003 completed the deletion after the amendment (DEC-002, REQ-008)
retired the conflicting guardrail. No inline-rule-plus-pointer (retry-shape, evidence
pointers) was reduced to a bare pointer — act-without-read preserved per SPEC-0049
DEC-002. Reviewer business/style audits confirmed no operational fact was lost.
`cargo test --workspace` passed at every commit with all required headings, include
lines, and command strings intact. Retry count: 1 (T-001 round 2 addressed a
"serializes" spelling regression caught by correctness review; T-002 and T-003 each
landed in round 1).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
T-001 removed the post-append journal re-read instruction from
`resources/modules/phases/speccy-work.md`. The work phase now relies on the CLI's
validate-before-write guarantee (a malformed block can never land) and retains at
most a `speccy next --json` consistency check for drift. The ejected
`phases/speccy-work.md` body contains no step instructing the agent to re-read the
journal after `speccy journal append`. Determinism trim recorded. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
T-001 removed both redundant `speccy status SPEC-NNNN --json` calls from
`resources/modules/phases/speccy-ship.md`. Step 1 now derives readiness and
`spec_md_path` / `tasks_md_path` from the single `speccy next --json` already run in
"When to use." Step 3 drops the post-flip `speccy status` re-check (the state-flip is
excluded from `spec_hash_at_generation`, so `TSK-003` cannot fire). Both removals
verified in the ejected ship-phase body. Determinism trims recorded. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005 CHK-006">
T-002 (Track 2) created six new include-only modules and replaced all former inline
copies with `{% include %}` directives: `references/retry-shape-summary.md` (x3
callsites: orchestrate, phases/work, skills/work), `personas/review-role-tail.md`
(x7 reviewers), `personas/vet-input-resolution.md` (x2: vet-reviewer, vet-implementer),
`personas/vet-no-rollback.md` (x2: vet-implementer, vet-simplifier),
`skills/partials/inline-fanout-rationale.md` (x3: orchestrate, vet, review), and
`references/spec-self-review-core.md` (x2: plan, amend). Grep audits confirmed no
non-canonical inline copies remain. `retry-shape-summary.md` expands to the full
inline sentence plus pointer — not a bare pointer — preserving act-without-read per
SPEC-0049 DEC-002. After `just reeject`, the ejected diff was include-expansion only
with no unexplained changes. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-007">
SPEC-0063 DEC-001 supersedes SPEC-0034 DEC-001 (the deferred plan+amend self-review
extraction pre-authorized by that decision). The new `references/spec-self-review-core.md`
module carries the supersession comment referencing the supersession. `speccy-brainstorm.md`
retains its independent inline self-review — its four artifact-oriented check properties
are structurally distinct from the shared core. The stale `<!-- … independent copy …
per DEC-001 / OQ-b … -->` comments were removed from `speccy-plan.md`,
`speccy-amend.md`, and `speccy-brainstorm.md`. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-008">
All four hygiene gates passed at each of the three commits (T-001, T-002, T-003):
`cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features
-- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny check`.
`persona_snippets.rs` and `authoring_commit.rs` passed at every commit with their
asserted headings, include lines, and command strings intact. The pre-ship vet gate
(invocation 1) surfaced the REQ-001/REQ-006 contradiction; invocation 2 cleared the
gate on its first drift-review round after the amendment and T-003 resolved it. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-009">
After each reeject, no ejected file under `.claude/`, `.agents/`, or `.codex/`
contained a `{%` marker (`grep -rn "{%" .claude .agents .codex` returned zero
matches). Track 1's ejected-pack diff mirrored the `resources/modules/**` edits 1:1
(prose compression); Track 2's ejected-pack diff was include-expansion only, with
the expanded text equal to the prior inline text at each callsite and no unexplained
changes. No broken wrapper or template variable was discovered. Retry count: 0.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-010">
T-003 removed the `persona_body.contains("feature-dev")` assertion and its
`read_persona` binding from `speccy-cli/tests/skill_packs.rs`. The test was renamed
to `feature_dev_personas_declare_speccy_model_conventions` (dropping `_and_attribution`).
The structural model-convention assertions (`model: opus[1m]`, `model = "gpt-5.5"`,
both `!contains("sonnet")` checks) were retained. T-003 also completed the deferred
A6/C6 deletion — the three `> Ported from the feature-dev …` provenance blockquotes
were removed from `reviewer-correctness.md`, `plan-architect.md`, and
`plan-explorer.md` in source and eject. `grep -rn 'Ported from' resources/ .claude/
.agents/ .codex/` returns nothing. `cargo test --workspace` passes with the assertion
removed and the provenance lines deleted. Attribution is henceforth editorial-only
per DEC-002. Retry count: 0.
</coverage>

</report>

## Notes

The pre-ship vet gate (invocation 1) caught the REQ-001/REQ-006 contradiction: the
Track 1 A6/C6 instruction to delete the `feature-dev` provenance lines conflicted
with the SPEC-0053 attribution guardrail in `skill_packs.rs`. The vet implementer
returned `stuck` (no code-only resolution exists); a `/speccy-amend` added REQ-008
and DEC-002 authorizing the guardrail retirement, and T-003 completed the work.
Invocation 2 cleared the gate on its first drift-review round and the simplifier
scan found no candidates.
