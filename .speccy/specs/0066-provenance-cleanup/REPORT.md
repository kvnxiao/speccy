---
spec: SPEC-0066
outcome: implemented
generated_at: 2026-06-21T10:30:00Z
---

# REPORT: SPEC-0066 Pre-ship provenance cleanup — broaden the provenance convention and add a dedicated vet cleanup pass

<report spec="SPEC-0066">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 broadened the "No provenance or doc-pointer meta-annotation" bullet in
`resources/modules/references/convention-checklist.md` to a four-shape nested
sub-list — Speccy-id citation, descriptive prose pointing at a planning artifact
(the no-`// per`-framing class flagged as the most common leak), numbered
project-rule citation, and doc-path citation — each with concrete negative
examples drawn from the eight leaked forms. The runtime-artifact carve-out
(`SPEC.md` / `.speccy/…` path = data the code reads or writes, not provenance)
was retained. `just reeject` propagated the change to all four ejected consumers
(`.claude/agents/{reviewer-style,speccy-work}.md` and their Codex mirrors).
CHK-001 (include-wiring and resource-to-ejected parity) is demonstrated: the
`resource_prose_hygiene` suite (5 tests green) gates ID-ban compliance, and the
existing `init_three_way.rs` parity suite (chk019/chk020/chk021) would catch a
stale mirror. CHK-002 (reviewer-reads judgment) is judgment-only per its scenario
text. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-002 (round 1) created `resources/modules/personas/vet-provenance.md` as a
single-concern provenance-cleanup persona body: sole review dimension is
provenance; the convention definition is sourced via `{% include
"modules/references/convention-checklist.md" %}` with no inline copy; apply-mode
instructions preserve intent while dropping the bare provenance pointer; the
runtime-artifact carve-out and prose-only scope are enforced. The round-1 tests
review blocked on CHK-003 evidence: the implementer labelled it `demonstrated`
citing `module_prose_has_no_internal_artifact_id_provenance`, which only matches
banned artifact-ID tokens and cannot detect a fully-inlined copy. Round 2 added
`vet_provenance_delegates_convention_via_include` in
`speccy-cli/tests/persona_snippets.rs`, asserting (1) the `{% include %}` line is
present and (2) the checklist's `## Convention-drift checklist` heading does not
appear inline — a copy now fails. CHK-003 is genuinely `demonstrated` by this
test. CHK-004 is judgment-only per its scenario text. Retry count: 1.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
T-002 also created the per-host wrappers:
`resources/agents/.claude/agents/vet-provenance.md.tmpl` (no read-only `tools:`
grant, `model: opus[1m]`, `effort: medium`) and
`resources/agents/.codex/agents/vet-provenance.toml.tmpl` (mirroring
vet-simplifier's Codex pin). `just reeject` ejected the subagent to
`.claude/agents/vet-provenance.md` and `.codex/agents/vet-provenance.toml`.
CHK-005 (frontmatter hygiene) is demonstrated by
`wrapper_descriptions_are_upload_safe_and_well_routed` (green) and the model-pin
suite (`claude_pinned_model_matches_alias_with_1m_suffix`,
`opus_pinned_effort_is_valid`, `codex_pinned_model_equals_gpt55`,
`codex_pinned_reasoning_effort_is_valid`, all green). CHK-006 (resource-to-ejected
parity) is demonstrated by `dogfood_outputs_match_committed_tree` (green). The
vet simplifier's unconsumed identity-sourcing include was caught by the vet drift
review and removed by the vet simplifier apply pass. Retry count: 1.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
T-003 added "Phase 3 — provenance-cleanup pass" to
`resources/modules/skills/partials/vet-phases.md`, slotted after the Phase 2
simplifier pass and before the renumbered Phase 4 gate. The phase dispatches
`vet-provenance` once over the cumulative `diff_command` and records via the
Phase 4 `<gate>` summary plus returned verdict — no new `--block` type, no new
`VetBlockKind`. Round 1 was blocked by correctness and architecture reviewers:
the snapshot was taken after the apply-mode sub-agent ran, making the blocking
revert path a no-op. Round 1 was also blocked by style and business reviewers: a
stale "Phase 3 `<gate>`" cross-reference at `vet-phases.md:37` survived the
renumbering. Round 2 reordered Phase 3 to snapshot before dispatch (mirroring
Phase 2's step-1-snapshot -> step-2-spawn -> step-3-keep-or-revert sequence) and
corrected the cross-reference to "Phase 4 `<gate>`". `just reeject` propagated
both fixes to the ejected vet skill mirrors. The per-task review fan-out was
left untouched (speccy-review.md: zero `provenance` occurrences). CHK-007 and
CHK-008 are both judgment-only per their scenario texts. Retry count: 1.
</coverage>

</report>

## Notes

A pre-existing SPEC.md blank-line-after-close-tag authoring defect (four
`</scenario></requirement>` boundaries at plan time) was discovered by T-001 and
repaired by T-003 as a whitespace-only mechanical fix; the spec hash was
re-recorded via `speccy lock SPEC-0066`. No requirement content changed.

The vet drift review caught and the vet simplifier removed an unconsumed
`identity-sourcing` include from `vet-provenance.md`; the persona appends no
journal block and emits no `model=` attribute, so the include had no consumer.
The fix is reflected in the final diff.

DEC-002's deliberate non-ship (no first-class provenance-cleanup journal block)
is recorded as a future-spec candidate in `.speccy/BACKLOG.md`.
