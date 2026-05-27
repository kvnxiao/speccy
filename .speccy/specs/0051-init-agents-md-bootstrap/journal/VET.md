---
spec: SPEC-0051
generated_at: 2026-05-27T22:03:12Z
---

## Invocation 1 — 2026-05-27T22:03:12Z

<drift-review verdict="blocking" round="1" date="2026-05-27T22:15:00Z" model="claude-opus-4-7[1m]/high">
Skill preamble and "When to use" prose still describe the pre-SPEC-0051 behavior, contradicting the steps below them and the user-stories that motivate the SPEC.
- REQ-002 / user-story #2 → The skill body's opening preamble (`resources/modules/phases/speccy-init.md` lines 4-7 and 14-16, both host branches) still reads "Bootstraps a Speccy workspace in three steps: scaffold `.speccy/`, copy the [Claude Code|Codex] skill pack into [...], and (if needed) seed the product north star into the project's root `AGENTS.md`." The skill now performs seven numbered steps and seeds two sections; the preamble undercuts the new step 5 (conventions upsert) by framing the skill as a three-step north-star-only bootstrap, increasing the risk an agent reading top-down skips step 5. The same drift appears in the ejected packs at `.claude/skills/speccy-init/SKILL.md:8-11` and `.agents/skills/speccy-init/SKILL.md:8-11`. The closing paragraph at `resources/modules/phases/speccy-init.md:186-190` was correctly updated to mention the conventions re-upsert; the opening paragraph was not.
- REQ-002 / user-story #2 ("re-running `/speccy-init` to refresh the `## Speccy conventions` section [...] so my agents pick up newly shipped skills and refined rules automatically") → the "When to use" subsection at `resources/modules/phases/speccy-init.md:11-12` and `20-21` says "Re-run with `--force` after upgrading `speccy` to refresh shipped recipes." Only "shipped recipes" (skill files) is named; the AGENTS.md conventions re-upsert — which is the entire mechanism by which the second user story works — is not mentioned at the point the agent (and a downstream reader) is told what re-running does. The wrapper `description:` frontmatter does name both sections, but the in-body "When to use" still does not.
</drift-review>

<holistic-fix verdict="addressed" round="1" date="2026-05-27T22:30:00Z" model="claude-opus-4-7[1m]/low">
Summary: Updated the skill preamble and "When to use" prose in both host branches to describe the conventions upsert alongside the north-star seed.

Addressed:
- REQ-002 / user-story #2 preamble drift → Rewrote both `{% if host == "claude-code" %}` and `{% else %}` opening paragraphs in `resources/modules/phases/speccy-init.md` (lines 4-23) to drop the misleading "three steps" framing and explicitly name the `## Speccy conventions` upsert as a fourth bootstrap action. After `just reeject`, this propagated to `.claude/skills/speccy-init/SKILL.md` and `.agents/skills/speccy-init/SKILL.md`.
- REQ-002 / user-story #2 "When to use" drift → Rewrote the "When to use" paragraphs in both host branches of `resources/modules/phases/speccy-init.md` to call out that `--force` re-runs refresh both the shipped skill files **and** the `## Speccy conventions` section in `AGENTS.md`, and to contrast that with the freeze-on-first-write `## Product north star`. Ejected packs updated via `just reeject`.

Hygiene: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny check` all green.
</holistic-fix>

<drift-review verdict="pass" round="2" date="2026-05-27T22:45:00Z" model="claude-opus-4-7[1m]/high">
Round 1 drift cleanly resolved; preamble and "When to use" prose in both host branches now describe the conventions upsert alongside the north-star seed, and the fix propagated correctly into both ejected packs. All seven requirements satisfied by the diff as a unit.
</drift-review>

<simplifier-scan verdict="clean">
SPEC-0051 diff is prose-only (skill module, conventions reference, ejected SKILLs, SPEC artifacts); intentional structural redundancy in the agent-facing instructions earns its place and no behavior-preserving cleanup applies.
</simplifier-scan>

<gate verdict="passed" tasks_hash="ba49921294e2d3529d29740d785d7cf7adc53742211d7e61084bef6a62c304ef" date="2026-05-27T22:10:52Z">
Drift cleared on round 2 after round 1 preamble/"When to use" fix; simplifier scan returned clean.
</gate>
