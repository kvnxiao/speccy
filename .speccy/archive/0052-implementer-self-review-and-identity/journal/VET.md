---
spec: SPEC-0052
generated_at: 2026-05-29T00:33:06Z
---

## Invocation 1 — 2026-05-29T00:33:06Z

<drift-review verdict="blocking" round="1" date="2026-05-29T00:41:33Z" model="claude-opus-4-8[1m]/high">
The two clusters land cleanly except for one cross-task contradiction: the new identity-sourcing rule forbids the dot-form model string, but the implementer phase body it is included into still shows the dot form three times as the canonical example, directly undercutting REQ-006/REQ-008.
- REQ-006 / REQ-008 (done-when "version punctuation preserved as `claude-opus-4-8`, never `claude-opus-4.8`"; "dot-form example strings in the shipped resources are corrected") → the `<implementer>` `model`-attribute documentation in the implementer phase still models the forbidden dot form (`model="claude-opus-4.8[1m]/low"`, `.../medium"`, `"claude-opus-4.8"`) as the canonical example, 11 lines above the included identity-sourcing partial that says "keep the hyphen form, never normalise it to a dotted form." The rendered prompt states the rule and contradicts it with its own worked examples. T-005 corrected the dot form only in `review-fanout.md`/`speccy-decompose.md` and left this touched file's examples uncorrected. See resources/modules/phases/speccy-work.md:209-211 (and the contradicting rule it now embeds at :281-282), rendered into .claude/agents/speccy-work.md:256-258 vs :281-282 and .codex/agents/speccy-work.toml.
- REQ-006 / user-story-2 (recorded provenance "trustworthy rather than a parent-session artifact") → an implementer reading the phase body top-to-bottom hits the dot-form `model="claude-opus-4.8[1m]/low"` template before the corrective partial, so the example most likely to be copied verbatim is the exact normalized-string defect the SPEC Summary names as defect #2. The fix is to reconcile these example strings to the hyphen form so the prompt does not model the behavior it forbids. See resources/modules/phases/speccy-work.md:204-214.
</drift-review>

<holistic-fix verdict="addressed" round="1" date="2026-05-29T00:48:00Z" model="claude-opus-4-8[1m]/high">
Summary: Corrected all dot-form `claude-opus-4.8` example strings in the shipped resources to the canonical hyphen form, removing the self-contradiction between the implementer phase examples and the identity-sourcing rule it embeds.

Addressed:
- REQ-006 / REQ-008 (implementer `model`-attribute examples model the forbidden dot form 11 lines above the corrective partial) → fixed the three example strings in resources/modules/phases/speccy-work.md:209-211 from `claude-opus-4.8[1m]/low` / `/medium` / `claude-opus-4.8` to the hyphen form. The phase body no longer models the behaviour the included identity-sourcing partial forbids.
- REQ-008 done-when "dot-form example strings in the shipped resources are corrected" → swept every sibling shipped resource that renders the same defect and corrected each opus dot-form to hyphen: resources/modules/references/journal-implementer.md (model-attr doc + `<implementer>` example), journal-review.md (2), journal-blockers.md (3), retry-shape.md (2), and the persona verdict examples in inline_note_format.md, reviewer-business.md, reviewer-architecture.md. Ran `just reeject` (11 ejected files overwritten across `.claude`/`.agents`/`.codex`). The only remaining `claude-opus-4.8` occurrence is identity-sourcing.md:14, where the dot form is deliberately cited as the forbidden anti-example ("never normalise it to a dotted form (`claude-opus-4.8`)") — leaving it is correct.
- Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny check` all pass.

Side discoveries:
- A `claude-sonnet-4.6[1m]` example string appears in journal-blockers.md:31 and journal-review.md:33. It is a different model used to illustrate per-persona model variance, and `4.6` is not the defect the SPEC names (the SPEC scopes the fix to the `claude-opus-4.8` normalization). Left as-is; flagging in case a future reviewer wants a blanket no-dot-form convention, which would be a SPEC-scope expansion rather than this SPEC's intent.
</holistic-fix>

<drift-review verdict="blocking" round="2" date="2026-05-29T01:05:42Z" model="claude-opus-4-8[1m]/high">
Round-1 opus dot-form contradiction is fully resolved, but the same defect persists for the sonnet example strings in two files the diff itself touched, and the implementer's "out of scope" punt does not survive the model-agnostic REQ-006 rule plus the repo's own existing hyphen-form sonnet convention.
- REQ-006 (model-agnostic done-when "version punctuation is preserved as the host states it... never normalise it to a dotted form") / REQ-008 done-when ("dot-form example strings in the shipped resources are corrected") → the `<review>` example strings still model the forbidden dotted form `claude-sonnet-4.6[1m]/medium`, in the very files the diff edited to fix the opus dot-form. The repo's own canonical sonnet examples already use the hyphen form (`claude-sonnet-4-6[1m]/medium` at resources/modules/personas/reviewer-{docs,security,style,tests}.md), so these are internally inconsistent shipped examples that model the exact normalization the included identity-sourcing partial forbids. journal-review.md now even mixes corrected hyphen-form opus with uncorrected dot-form sonnet in one file. See resources/modules/references/journal-review.md:33,59,68,95 and resources/modules/references/journal-blockers.md:31, rendered into .claude/skills/speccy-review/references/journal-review.md, .claude/speccy-references/journal-blockers.md (and the .agents mirrors). The round-1 "side discovery" scoping argument (SPEC names only `claude-opus-4.8`) is the narrow-literal reading; CHK-010 scopes to "the touched files," these are touched files, and REQ-006's rule is model-agnostic — so this is residual drift, not an out-of-scope expansion. Correct the five sonnet strings to `claude-sonnet-4-6` and re-eject, OR record an explicit SPEC-scope decision that example strings for non-opus models are intentionally left dotted (human adjudication) — do not silently carry it forward a third time.
</drift-review>

<holistic-fix verdict="addressed" round="2" date="2026-05-29T01:18:30Z" model="claude-opus-4-8[1m]/high">
Summary: Corrected the five sonnet dot-form example strings to the canonical hyphen form in the two flagged source modules and re-ejected.

Addressed:
- REQ-006 model-agnostic done-when (version punctuation preserved as host states it, never normalised to dotted form) + REQ-008 done-when (dot-form example strings in shipped resources corrected) → converted `claude-sonnet-4.6` to `claude-sonnet-4-6` in resources/modules/references/journal-review.md:33,59,68,95 and resources/modules/references/journal-blockers.md:31, then ran `just reeject` (2 ejected files overwritten: .claude/skills/speccy-review/references/journal-review.md and .claude/speccy-references/journal-blockers.md plus the .agents mirror). This matches the existing hyphen-form sonnet convention in resources/modules/personas/reviewer-{docs,security,style,tests}.md. journal-review.md no longer mixes corrected opus with uncorrected sonnet.

Verification: `grep -rn 'claude-sonnet-4\.6'` over resources/ and all shipped packs returns no hits. The only surviving `claude-sonnet-4.6` is in VET.md (the round-1 side-discovery note — orchestrator-owned). The only surviving `claude-opus-4.8` dot-forms are the deliberate forbidden anti-example at resources/modules/references/identity-sourcing.md:14 (rendered into .claude/agents/*) and illustrative blocks in docs/ARCHITECTURE.md (not in this SPEC's diff scope; canonical doc prose, not a shipped resource example). Hygiene suite green.

Side discoveries:
- docs/ARCHITECTURE.md:570-630 still carries `claude-opus-4.8[1m]` dot-form in illustrative `<implementer>`/`<review>` example blocks. It is untouched by this diff and outside the SPEC's touched-file scope (an internal design doc, not a shipped resource), so left as-is; a future blanket no-dot-form-anywhere convention would be a separate SPEC.
</holistic-fix>

<drift-review verdict="pass" round="3" date="2026-05-29T00:47:25Z" model="claude-opus-4-8[1m]/high">
All nine requirements land as a unit and the round-1/round-2 dot-form drift is fully resolved: no dot-form model example string remains in any shipped resource or pack (the only surviving `claude-opus-4.8` is the deliberate forbidden anti-example rendered from identity-sourcing.md, and the corrected sonnet strings ship as `claude-sonnet-4-6[1m]/medium`); the docs/ARCHITECTURE.md and Rust-test-fixture dot-forms are out of CHK-010's "shipped resources / touched files" scope and the exclusion is defensible; `just reeject` is clean (0 overwritten) and `cargo test --workspace` is green.
</drift-review>

<simplifier-scan verdict="clean">
No simplification candidates: the diff is mechanical identifier normalization plus the project's mandated dedup-via-include pattern, which is itself the simplification the SPEC delivers; no shadow copies, orphaned prose, or inline-able abstractions remain.
</simplifier-scan>

<gate verdict="passed" tasks_hash="acfeb0d3c2476ef9768b86691236e0c229426c9d847ca351ac9011c2d50fffcf" date="2026-05-29T00:48:55Z">
Drift cleared on round 3 (opus then sonnet dot-form contradictions fixed and re-ejected); simplifier scan clean; all nine requirements land as a unit.
</gate>



