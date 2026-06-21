---
spec: SPEC-0066
generated_at: 2026-06-21T10:04:46Z
---

## Invocation 1 — 2026-06-21T10:04:46Z

<drift-review verdict="pass" round="1" date="2026-06-21T10:04:46Z" model="claude-opus-4-8[1m]">
Diff satisfies SPEC-0066 as a unit: all four requirements' done-when met, non-goals honored, full workspace suite green. One non-blocking leanness nit worth a human glance.

- REQ-001 satisfied: the convention-checklist provenance bullet names all four shapes (Speccy-id, descriptive-prose-pointing-at-a-planning-artifact, numbered-project-rule, doc-path) each with concrete negative examples, retains the runtime-artifact carve-out, and the single edit reaches both consuming callsites — re-ejected identically into .claude/agents/speccy-work.md and .claude/agents/reviewer-style.md (parity holds). See resources/modules/references/convention-checklist.md:20.
- REQ-002 satisfied: resources/modules/personas/vet-provenance.md is single-concern (provenance only, other checklist bullets explicitly out of scope), sources the definition via {% include %} of the convention reference rather than inlining it (gated by the new vet_provenance_delegates_convention_via_include test), and instructs intent-preserving rewrite / carve-out respect / prose-only scope.
- REQ-003 satisfied: both wrappers exist, the Claude Code wrapper carries no read-only tools: grant (apply-mode), both declare the medium pin mirroring vet-simplifier exactly (opus[1m]/effort:medium for Claude Code, gpt-5.5/model_reasoning_effort low for Codex), and the description is angle-bracket-free with a "Use when …" clause (wrapper-description hygiene lint green).
- REQ-004 satisfied: vet-phases.md adds a dedicated Phase 3 provenance pass dispatching vet-provenance once over the cumulative diff, recording via the renumbered Phase 4 <gate> summary line and the returned verdict; no new --block type, no new VetBlockKind, no schema change (no speccy-cli/src or docs diff), and the per-task review fan-out is untouched. DEC-002's promote-later candidate is logged in .speccy/BACKLOG.md.
- Non-blocking leanness nit (no SPEC anchor — quality observation for the human): resources/modules/personas/vet-provenance.md:102 includes modules/references/identity-sourcing.md, but this persona appends no journal block and emits no model= attribute anywhere (its thin verdict is `<verdict role="provenance" verdict="…" rationale="…" />`). Every other persona that pulls in identity-sourcing does so to feed `--model` on a `speccy journal append`; the reviewer-* personas that return thin no-block verdicts omit the include. So the include has no consumer here — dead-weight prose in an ejected body, against AGENTS.md "keep ejected content lean". Not a contract violation; surfaced for the human to decide whether to drop the include.
</drift-review>
<simplifier-scan verdict="candidates">
One behaviour-preserving candidate: drop the unconsumed identity-sourcing include from the vet-provenance persona body.

- resources/modules/personas/vet-provenance.md:102 — remove `{% include "modules/references/identity-sourcing.md" %}`. That reference defines only how to build a `model="..."` value, but this persona emits no `model=` attribute (its verdict is `<verdict role="provenance" verdict="..." rationale="..." />`) and appends no journal block needing `--model`, so the include has no consumer. The seven reviewer-* no-block thin-verdict personas already omit it; only personas that append model=-bearing blocks pull it in. Dropping it removes dead-weight prose from an ejected body (AGENTS.md "keep ejected content lean"). Requires `just reeject` to also strip the expanded text from .claude/agents/vet-provenance.md:183 and .codex/agents/vet-provenance.toml:182. No test gates this include (persona_snippets.rs asserts only the convention-checklist include), so it stays green.
</simplifier-scan>
<simplifier-apply verdict="applied">
Removed the unconsumed identity-sourcing include from the vet-provenance persona body; reejected both host packs; full hygiene suite green.

- resources/modules/personas/vet-provenance.md: dropped `{% include "modules/references/identity-sourcing.md" %}` — the persona emits no `model=` attribute and appends no journal block, so the include had no consumer (the no-block reviewer personas already omit it). `just reeject` stripped the expanded text from .claude/agents/vet-provenance.md and .codex/agents/vet-provenance.toml.
- Hygiene: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny check` all green.
</simplifier-apply>
<gate verdict="passed" tasks_hash="bbf4d3281a4d2e11b86d0587038980fd114eb4b4527f666f517a1ba9b69c9823" date="2026-06-21T10:11:27Z">
Drift cleared on round 1; simplifier applied (dropped unconsumed identity-sourcing include from vet-provenance + reejected); provenance: clean.
</gate>
