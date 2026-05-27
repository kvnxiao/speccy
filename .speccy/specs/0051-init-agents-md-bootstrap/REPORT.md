---
spec: SPEC-0051
outcome: implemented
generated_at: 2026-05-27T22:50:00Z
---

# REPORT: SPEC-0051 `/speccy-init` AGENTS.md bootstrap — seeds `## Speccy conventions` section and refactors north star Q&A to a brainstorm-style adaptive flow

<report spec="SPEC-0051">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
T-001 authored `resources/modules/references/agents-md-speccy-conventions.md` carrying the canonical `## Speccy conventions` body. The file opens with the upsert-contract preamble naming `/speccy-init` as the section's manager and warning that edits inside the section are overwritten on re-run. The five subsections appear in REQ-001 order: when-to-use-which-skill (one-liner per shipped skill — all ten skills enumerated), the dev loop (Plan → Tasks → Impl → Review → Ship with the journal-file pointer at `.speccy/specs/NNNN-slug/journal/T-NNN.md`), test hygiene (all five vacuous-test anti-patterns in language-neutral phrasing plus the investigate-flakes rule), commit hygiene (`Co-Authored-By` trailer + narrow commits), and CI gate suggestion (`speccy verify` named as the gate, multiple CI platforms named in prose without vendor-specific configuration). The body contains no Rust-specific idioms, no platform CI wiring artifacts, and no Speccy-internal references that would embarrass downstream users. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002 CHK-003">
T-002 extended `resources/modules/phases/speccy-init.md` with an explicit conventions upsert step (step 5) carrying the heading-boundary upsert rule: absent heading → append canonical body with heading; present heading → replace from heading to next top-level `##` or end of file. The replace path runs on every invocation with no "already matches, skip" optimization. An `{% include "modules/references/agents-md-speccy-conventions.md" %}` directive expands the canonical body inline in the rendered prompt. The upsert step explicitly prohibits HTML comment markers as fencing; the heading boundary is the sole delimiter. The closing paragraph was updated to note the conventions section is re-upserted on every invocation. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004 CHK-005">
T-003 replaced the fixed seven-question script in step 4 of `resources/modules/phases/speccy-init.md` with a brainstorm-style adaptive flow. The new flow instructs the agent to first inspect the repo (README, manifest files, source layout, existing `AGENTS.md` prose), then walk the five subsections in template order (opening prose, `### Users`, `### V1.0 outcome`, `### Quality bar`, `### Known unknowns`) with a per-subsection draft-or-Socratic decision. A hard gate forbids writing `## Product north star` until every subsection is user-approved. The skill body explicitly prohibits invoking `/speccy-brainstorm` and inlines brainstorm-style patterns per DEC-002. The prior seven-question script is fully absent from both source and ejected packs. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006">
T-003 preserved the freeze-on-first-write branch for `## Product north star` (State C in step 3). When the heading is already present on re-run, the skill body instructs the agent to confirm the existing content is current and proceed without modification — no re-elicitation, no overwrite, no diff prompt. T-003 added an explicit asymmetry note contrasting north-star freeze-on-first-write (user-authored content) with the always-upsert conventions section (canonical boilerplate sourced from upstream). The State C branch in step 3 was untouched by T-003. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-007 CHK-008">
T-002 documented the AGENTS.md state matrix explicitly: north-star (present/absent) × conventions (present/absent), with four cells named and the action per cell specified. The skill body states the two seeding decisions are made independently — neither outcome biases treatment of the other section. The missing-file path is documented as silent re-bootstrap with no warning, refusal, or regression-detection ceremony. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-009 CHK-010">
T-004 updated the `description:` frontmatter in both `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl` and `resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl` to name both seeded sections while preserving existing trigger phrases, precondition, and Do NOT clause. `just reeject` propagated all changes — adaptive north-star flow (T-003), conventions upsert step (T-002), and inlined canonical body (T-001 via `{% include %}`) — into `.claude/skills/speccy-init/SKILL.md` and `.agents/skills/speccy-init/SKILL.md`. The vet drift-fix round (invocation 1, round 1) additionally corrected the opening preamble and "When to use" prose in both host branches of `resources/modules/phases/speccy-init.md` to drop the stale "three steps" framing and explicitly name the conventions upsert alongside the north-star seed; `just reeject` propagated the fix into the ejected packs. No Rust source files were modified by this SPEC. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-011">
T-005 ran all four standard hygiene gates against the working tree at HEAD after T-001–T-004 plus `just reeject`: `cargo test --workspace` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0; `cargo +nightly fmt --all --check` exit 0; `cargo deny check` exit 0 (advisories ok, bans ok, licenses ok, sources ok). The vet pass additionally confirmed the gates remain green after the round-1 drift-fix. Retry count: 0.
</coverage>

</report>

## Notes

The vet pass (invocation 1, round 1) caught a single drift item: the opening preamble and "When to use" prose in both host branches of the skill body still described the pre-SPEC-0051 three-step behavior after T-001–T-005 landed, undercutting the new conventions upsert step. The holistic-fix in round 1 rewrote both paragraphs and re-ejected. Round 2 returned pass; the simplifier scan returned clean on the prose-only diff.

All task retry counts are 0. The single vet round-1 blocking finding was a preamble-only prose drift with no behavioral regression; it cleared in one fix pass.
