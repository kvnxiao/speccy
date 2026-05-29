---
spec: SPEC-0052
outcome: implemented
generated_at: 2026-05-29T01:30:00Z
---

# REPORT: SPEC-0052 Implementer pre-handoff self-review and accurate model/effort recording

<report spec="SPEC-0052">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
T-002 added a self-review step (step 7) to `resources/modules/phases/speccy-work.md`, positioned after the implement step (step 6) and before the `in-review` state flip / hygiene gate (step 8). The step instructs the agent to re-read its own diff against the reviewers' criteria and fix findings in place, framed as the cheap place to catch drift versus a later review round. Propagated via `just reeject` to `.claude/agents/speccy-work.md` and `.codex/agents/speccy-work.toml`. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
T-002 added a four-persona north-star map inside the self-review step, naming business, tests, security, and style each with a one-line outcome statement. The framing explicitly states "not how the reviewers hunt for problems," withholding detection tactics. The style entry defers to the shared convention checklist via `{% include %}` rather than restating its contents. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003 CHK-004">
T-001 created `resources/modules/references/convention-checklist.md` with five language- and project-agnostic categories: reuse-over-reinvent; match-local-conventions; docs-match-code; no-false-complexity (including splitting a function past the file's complexity ceiling); and re-apply-the-project's-own-hard-rules (including vacuous/constant-copy tests and suppressions that must carry a justification). T-002 wired the `{% include %}` into both `resources/modules/phases/speccy-work.md` (implementer self-review) and `resources/modules/personas/reviewer-style.md` (style reviewer), removing the style reviewer's prior bespoke five-bullet enumeration. `just reeject` and `cargo test --workspace` both exited 0. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
T-003 added a `## Grounding a lint-driven verdict` section to `resources/modules/personas/reviewer-style.md`. The section requires lint-fire confirmation before raising a blocking verdict demanding a lint-driven change (especially a suppression annotation), names the sibling-consistency argument as insufficient grounds on its own, and routes an unconfirmable demand to a one-line aside outside the `<review>` block. Propagated to both ejected hosts via `just reeject`. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006">
T-004 created `resources/modules/references/identity-sourcing.md` with the effort-sourcing rule: the effort suffix is read from the agent's own definition file (`effort:` on Claude Code, `model_reasoning_effort` on Codex) and explicitly forbids deriving it from `CLAUDE_EFFORT` or other inherited environment variables. The documented `CLAUDE_CODE_EFFORT_LEVEL` runtime-override limitation (deliberately not read; a run that sets it records the definition-file effort) is included. Wired into the implementer phase, the reviewer verdict-return contract, and the vet personas. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-007">
T-004's identity-sourcing partial states the model id is the resolved long-form identifier the host states in-context, transcribed verbatim with version punctuation preserved (hyphen form, `claude-opus-4-8`, never the dot form `claude-opus-4.8`), with a definition-file fallback where the host states no in-context identifier. The vet loop (rounds 1 and 2) caught and corrected residual dot-form example strings in the implementer phase body and sibling resource modules; `just reeject` propagated all corrections. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-008 CHK-009">
T-004 created the single shared `resources/modules/references/identity-sourcing.md` partial and wired `{% include %}` into five callsites: the implementer phase (`speccy-work.md`), the reviewer verdict-return contract (`verdict_return_contract.md`), and all three vet personas (`vet-implementer.md`, `vet-reviewer.md`, `vet-simplifier.md`). `just reeject` exited 0, confirming the partial resolves at every include site. Retry count: 0.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-010">
T-005 replaced the "sourced from the host harness's runtime model identifier (env var, runtime API, ...)" phrasing in `resources/modules/skills/partials/review-fanout.md` and `resources/modules/phases/speccy-decompose.md` with the single-source rule deferring to the identity-sourcing partial. The vet loop (rounds 1 and 2) additionally caught dot-form opus and sonnet example strings across `speccy-work.md`, `journal-implementer.md`, `journal-review.md`, `journal-blockers.md`, `retry-shape.md`, `inline_note_format.md`, `reviewer-business.md`, and `reviewer-architecture.md`; all were corrected and re-ejected. No dot-form `claude-opus-4.8` or `claude-sonnet-4.6` example strings remain in any shipped resource (the only surviving dot-form is the deliberate forbidden anti-example in `identity-sourcing.md`). `just reeject` exited 0. Retry count: 0.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-011">
T-006 added a `## Changelog` section with a `<changelog>` table worked example (using the `Date | Author | Summary` shape archived specs use) to `resources/modules/references/spec.md`, and added `<changelog>` to the reference's "Shape invariants the lint suite enforces" list. `just reeject` propagated the corrected reference to the ejected `speccy-plan` skill across both hosts. Retry count: 0.
</coverage>

</report>

## Notes

The vet loop ran one invocation with three drift-review rounds. Round 1 caught opus dot-form example strings in the implementer phase body that directly contradicted the embedded identity-sourcing rule; round 2 caught the same pattern for sonnet example strings in journal-review.md and journal-blockers.md that the round-1 fix had left inconsistent. Both were corrected and re-ejected before the round-3 pass verdict. The simplifier scan was clean — the diff is mechanical identifier normalization plus the mandated dedup-via-include pattern.

The one open question (does the all-four-persona north-star map measurably reduce non-style first-round blocks, or is it noise that should be trimmed to the style-only checklist?) is deferred to dogfooding per SPEC decision DEC-005 and the SPEC's own annotation.
