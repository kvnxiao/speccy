---
id: SPEC-0066
slug: provenance-cleanup
title: Pre-ship provenance cleanup — broaden the provenance convention and add a dedicated vet cleanup pass
status: in-progress
created: 2026-06-21
supersedes: []
---

# SPEC-0066: Pre-ship provenance cleanup — broaden the provenance convention and add a dedicated vet cleanup pass

## Summary

Provenance references to planning artifacts — the SPEC, "later specs",
numbered project rules, doc-path citations — leaked into shipped product
code and test comments on a downstream project and survived the full
implement → per-persona review → vet → ship loop. A post-ship sweep found
eight such sites. None used the `// per REQ-NNN` form; they were
descriptive natural-language prose ("every failure mode the spec defines",
"later specs populate", "a later spec can ask"), a numbered project-rule
citation ("cardinal rule #4"), and a doc-path citation
("(docs/implementation)").

Two gaps let them through. First, the convention forbidding provenance —
the "No provenance or doc-pointer meta-annotation" bullet in the shared
`convention-checklist.md` — illustrates only the `// per X` /
`(Core principle 2)` / `see docs/ARCHITECTURE.md` shape, so a reader
applying it literally lets descriptive prose and numbered-rule citations
through. Second, the convention is a single diffuse checklist item spread
across the per-persona reviewers, the implementer self-review, and vet;
in every one of those gates provenance is one concern among many, and that
divided attention is why it slipped past all of them.

This SPEC closes both gaps. It broadens the shared definition so the cheap
early gates (implementer self-review and the style reviewer, which both
include the same checklist) catch the wider class. And it adds a
**dedicated, single-responsibility pre-ship cleanup pass** to the
`speccy-vet` flow whose entire remit is provenance — model-driven, not a
grep, because the defect is unbounded natural language. The pass rewrites
offending comment and doc prose to drop the bare pointer while keeping the
intent, honours the runtime-artifact carve-out (naming a path the code
operates on is data, not provenance), and records its outcome through the
existing CLI-owned `<gate>` block rather than a new journal block type — so
no Rust or schema change ships.

## Goals

<goals>
- The shared provenance convention names the descriptive-prose,
  numbered-project-rule, and doc-path-citation classes explicitly, with
  concrete negative examples, so a reviewer cannot rationalise the leaked
  forms as acceptable.
- The runtime-artifact carve-out survives the broadening: naming a path the
  code operates on (`SPEC.md`, a `.speccy/…` path) stays data, not
  provenance.
- A dedicated provenance-cleanup pass runs once at the pre-ship boundary
  over the cumulative working-tree diff, with provenance as its sole
  concern.
- The pass rewrites offending comment/doc prose to drop the bare provenance
  pointer while preserving the intent the comment conveys; it never edits
  logic.
- The whole change ships as `resources/**` edits plus reejection — no new
  CLI command, no new journal block type, no schema change.
</goals>

## Non-goals

<non-goals>
- No `speccy verify` lint over product code. A grep token list cannot bound
  natural-language provenance, and a content lint over arbitrary product
  source is unscopable in a downstream repo and against the
  feedback-not-enforcement and stay-small principles.
- No new CLI command, no new `VetBlockKind` / journal block grammar, and no
  `schema_version` change. The pass records through the existing `<gate>`
  block summary and the verdict it returns to its caller.
- Not blocking or enforcement. The cleanup pass does work inside the loop,
  the same as the simplifier; the CLI gates nothing new.
- Not run in the per-task review fan-out. The pass is pre-ship only; the
  per-task early catch is the broadened checklist in the implementer
  self-review and the style reviewer.
</non-goals>

## User Stories

<user-stories>
- As a developer running Speccy on my own project, I want provenance
  references to planning artifacts caught and cleaned before a PR opens, so
  comments that mean nothing once the code stands alone never ship.
- As a reviewer applying the convention, I want it to name the descriptive,
  numbered-rule, and doc-path forms explicitly, so I am not left to
  generalise from a single `// per X` example and wave borderline prose
  through.
- As a maintainer of the skill pack, I want the cleanup pass to share one
  definition of provenance with the early gates, so the convention and the
  pass cannot drift apart.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Broaden the shared provenance definition in the convention checklist

The "No provenance or doc-pointer meta-annotation" bullet in
`resources/modules/references/convention-checklist.md` is broadened to name
three reference classes beyond the `// per X` form it currently
illustrates, each with a concrete negative example: (a) descriptive prose
that points at a planning artifact as the reason a line exists, with no
`// per` framing; (b) numbered project-rule citations; (c) doc-path
citations. The runtime-artifact carve-out — naming a path the code operates
on is data — is preserved. Because this file is `{% include %}`d by the
implementer self-review (work phase) and the style reviewer, one edit reaches
both early gates.

<done-when>
- The provenance bullet explicitly names the descriptive-prose,
  numbered-project-rule, and doc-path-citation classes, each with a concrete
  negative example.
- The bullet retains the runtime-artifact carve-out wording.
- The broadened bullet reaches both consuming callsites (the work-phase
  implementer self-review and the style-reviewer persona) and the ejected
  host packs after reejection (parity holds).
</done-when>

<behavior>
- Given the convention checklist at HEAD, when its provenance bullet is
  read, then it names all three reference classes with negative examples and
  retains the carve-out.
- Given the work-phase and style-reviewer module bodies, when their
  `{% include %}` directives resolve, then the broadened bullet is present in
  both.
</behavior>

<scenario id="CHK-001">
Given the resources tree and the ejected host packs at HEAD,
when the work-phase and style-reviewer modules are re-ejected and their
includes resolved,
then the broadened provenance bullet appears in both consumers' ejected
output — gating include-wiring and resource-to-ejected parity, not file
existence.
</scenario>

<scenario id="CHK-002">
Given the broadened bullet,
when a reviewer reads it against the eight leaked forms (descriptive prose
naming the SPEC / "later specs", a numbered project-rule citation, a doc-path
citation),
then none of those forms can be rationalised as acceptable, and the carve-out
still clearly permits naming a path the code operates on. This is a
persona-review judgment, not a scriptable assertion.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: Dedicated single-concern provenance-cleanup persona body

Speccy ships a persona body under `resources/modules/personas/` whose sole
review dimension is provenance — no competing remit. It sources the
provenance definition by `{% include %}` of the same convention reference
REQ-001 broadens, so the definition is shared, not copied. Its fix
instruction is to rewrite offending comment, doc, and test-doc prose to drop
the bare provenance pointer while preserving the intent the comment conveys,
to honour the runtime-artifact carve-out, and to confine edits to prose —
never logic.

<done-when>
- A persona body exists whose only review dimension is provenance cleanup.
- It pulls the provenance definition in via `{% include %}` of the
  convention reference REQ-001 broadens; the definition text is not also
  duplicated inline in the body.
- Its instructions say to rewrite offending prose to drop the bare pointer
  while keeping the intent, to honour the carve-out, and to touch only
  comment/doc/test-doc prose (behaviour-preserving).
</done-when>

<behavior>
- Given the persona body at HEAD, when it is read, then its only review
  dimension is provenance and its fix instruction is intent-preserving
  rewrite, not blind deletion.
- Given the body, when its includes resolve, then the broadened definition is
  present without a second inline copy.
</behavior>

<scenario id="CHK-003">
Given the resources tree at HEAD,
when the persona body's `{% include %}` directives resolve,
then the convention reference is included and the definition text does not
also appear inline in the body — gating the share-via-include rather than a
copy.
</scenario>

<scenario id="CHK-004">
Given the persona body,
when a reviewer reads it,
then it reads as a single-concern provenance pass and instructs
intent-preserving rewrite, carve-out respect, and prose-only scope. This is a
persona-review judgment, not a scriptable assertion.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Per-host wrappers carry apply-mode dispatch metadata and the model pin

The subagent ejects through per-host wrappers — a Claude Code `.md.tmpl` and
a Codex `.toml.tmpl` under `resources/agents/` — each pulling the persona
body in via `{% include %}`. Because the pass is apply-mode (it edits files
and runs hygiene), the Claude Code wrapper carries no read-only `tools:`
restriction, unlike the read-only reviewer personas. The frontmatter declares
the medium model/effort pin, mirroring the existing `vet-simplifier` pin. The
description is angle-bracket-free and carries a "Use when …" clause, as a
programmatically-spawned subagent wrapper requires.

<done-when>
- A Claude Code wrapper and a Codex wrapper for the subagent exist under
  `resources/agents/`, each including the persona body.
- The Claude Code wrapper declares no read-only `tools:` grant — the pass is
  apply-mode and needs write-capable tools.
- The wrapper frontmatter declares the medium model/effort pin.
- The wrapper description is angle-bracket-free and carries a "Use when …"
  clause, satisfying the wrapper-description hygiene lint.
</done-when>

<behavior>
- Given the wrappers at HEAD, when their frontmatter is read, then each
  includes the persona body, declares the medium pin, and (Claude Code) omits
  a read-only tool restriction.
- Given the wrappers, when re-ejected, then the subagent appears in both
  `.claude/agents/` and `.codex/agents/` with the persona body inlined.
</behavior>

<scenario id="CHK-005">
Given the wrapper templates at HEAD,
when their frontmatter is parsed,
then the Claude Code wrapper omits a read-only `tools:` restriction
(apply-mode), both declare the model pin, and the description passes the
angle-bracket and "Use when …" checks — asserting the stable frontmatter
surface, not body prose.
</scenario>

<scenario id="CHK-006">
Given the resources tree and the ejected host packs at HEAD,
when the wrappers are re-ejected,
then the subagent is present in both `.claude/agents/` and `.codex/agents/`
with the persona body inlined — gating resource-to-ejected parity.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: Vet flow runs the cleanup once at pre-ship and records via the gate summary

The `speccy-vet` phase flow gains a dedicated provenance-cleanup phase that
dispatches the subagent once over the cumulative working-tree diff at the
pre-ship polish stage. The phase records its outcome through the existing
CLI-owned `<gate>` block's one-line summary and the verdict returned to the
caller; it appends no new journal block and the CLI's closed block set is
unchanged. Like the simplifier phase, the pass is behaviour-preserving and
does not change the drift pass/fail verdict. The per-task review fan-out is
left unchanged.

<done-when>
- The vet phase flow adds a dedicated provenance-cleanup phase that dispatches
  the subagent over the cumulative diff at the pre-ship polish stage, before
  the `<gate>` block is appended.
- The phase records its result through the `<gate>` summary line and the
  returned verdict — no `speccy journal append --block <new-type>`, no new
  `VetBlockKind`, and the CLI block set is unchanged.
- The phase runs once per invocation over the whole diff, not per task; the
  per-task review skill gains no provenance persona.
</done-when>

<behavior>
- Given the vet phase flow at HEAD, when it is read, then a provenance-cleanup
  phase dispatches the subagent once over the cumulative diff before the gate
  and surfaces its outcome via the gate summary.
- Given the per-task review skill at HEAD, when it is read, then no provenance
  persona has been added to its fan-out.
</behavior>

<scenario id="CHK-007">
Given the ejected vet skill pack and the CLI block set at HEAD,
when the vet phase flow is read,
then it contains a provenance-cleanup phase that spawns the subagent and
references no new `--block` type, and the CLI's journal block set is
unchanged — gating the wiring and the no-new-block contract together.
</scenario>

<scenario id="CHK-008">
Given the vet phase prose and the per-task review skill body,
when a reviewer reads them,
then the provenance phase runs once over the cumulative diff at pre-ship and
records via the gate summary plus verdict, and the per-task review fan-out is
unchanged. This is a persona-review judgment, not a scriptable assertion.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
#### DEC-001: A dedicated subagent, not a concern folded into vet-simplifier

**Context:** The cleanup could be its own subagent or folded into the
existing `vet-simplifier`, which already scans the whole cumulative diff and
applies behaviour-preserving fixes.

**Decision:** Ship a dedicated, single-responsibility provenance subagent.

**Alternatives:** Folding provenance into `vet-simplifier` — rejected:
divided attention is the documented root cause of the original leak (the
convention cleared six general-purpose gates because provenance was one
concern among many in each). Folding re-introduces exactly that dilution into
the one pass meant to fix it.

**Consequences:** One more agent in the pack (body plus two wrappers), in
exchange for an undiluted prompt. The simplifier's remit is untouched.
</decision>

<decision id="DEC-002">
#### DEC-002: Record via the gate summary, not a new journal block

**Context:** The vet journal block set is closed and CLI-validated. Every
other vet role (drift review, holistic fix, simplifier scan/apply) earned its
own block via Rust changes. A dedicated provenance block would mean the same.

**Decision:** The pass records its outcome through the existing CLI-owned
`<gate>` block's one-line summary and the verdict it returns. No new block
type ships.

**Alternatives:** A first-class `provenance-fix` block (or a scan/apply pair)
— rejected for now: the audit value is lower than for the roles that earned a
block. Drift fixes trace to SPEC requirements and gate a re-review loop; a
provenance cleanup's entire output is self-evident in the final diff, so a
structured block would mostly duplicate `git diff`. The Rust and schema cost
is not yet justified; recorded as a backlog candidate to promote if dogfooding
shows the audit gap bites. Reusing the `simplifier-scan`/`simplifier-apply`
blocks — rejected: a provenance pass appending `<simplifier-*>` blocks is a
dishonest audit trail, strictly worse than the gate summary at the same
zero-Rust cost.

**Consequences:** Zero Rust and zero schema change. The per-invocation audit
record is the gate summary line plus the verdict and the visible diff, not a
queryable provenance block.
</decision>

<decision id="DEC-003">
#### DEC-003: Apply-mode, not flag-only

**Context:** The pass could rewrite the offending prose itself (apply-mode,
like the simplifier) or only flag findings for a separate fixer to address.

**Decision:** Apply-mode — the subagent rewrites the prose and runs the
project's hygiene suite.

**Alternatives:** Flag-only, handing fixes to the existing `vet-implementer` —
rejected: the fix is trivial prose; a reviewer→implementer round-trip is
heavier machinery than the defect warrants.

**Consequences:** The subagent needs write-capable tools (REQ-003), so its
Claude Code wrapper is not read-only. The orchestrator owns the rollback
snapshot, as it does for the simplifier apply step.
</decision>

<decision id="DEC-004">
#### DEC-004: Medium model/effort pin

**Context:** Provenance triage needs more than pattern-matching — judging the
carve-out and preserving comment intent are genuine judgment — but it is not a
deep-reasoning task.

**Decision:** Pin the pass to the medium tier, mirroring `vet-simplifier`'s
existing pin.

**Alternatives:** A cheaper tier — deferred: ratchet down later if dogfooding
shows medium is overkill. Pinning the model identifier itself is not the
lever; effort is, matching the project's existing per-phase pin convention.

**Consequences:** Cost parity with the simplifier pass it runs beside.
</decision>

## Assumptions

<assumptions>
- The pass is pre-ship only; it is not added to the per-task review fan-out.
  The per-task early catch is the broadened checklist (REQ-001) in the
  implementer self-review and the style reviewer.
- "Medium tier" maps to the project's existing per-phase pin convention —
  the same model identifier the other vet subagents pin, with `effort: medium`
  — mirroring `vet-simplifier`; the effort knob is the lever, not the model id.
- The cleanup's edits are visible in the final branch diff, so the `<gate>`
  summary plus the returned verdict are a sufficient audit record without a
  dedicated journal block.
- Provenance cleanup is behaviour-preserving (prose only), so — like the
  simplifier phase — a revert of its changes leaves the drift pass/fail verdict
  intact.
</assumptions>

## Open questions

None — the brainstorm resolved framing questions a through e, and the
audit-record fork (gate summary vs. a new journal block) was resolved before
this SPEC was drafted.

## Notes

Three alternatives to the dedicated pass were considered and rejected. A grep
recipe over the diff (a token regex): provenance is unbounded natural
language, so a regex misses paraphrase, and tightening it to chase paraphrase
reintroduces the false-positive surface ("specify", "specification",
"required"). A `speccy verify` lint over product code: it is unscopable in an
arbitrary repo (the CLI cannot tell product source from Speccy-manipulating
tooling without the config Speccy will not grow) and runs against the
feedback-not-enforcement and stay-small principles. A sixth per-task review
persona: it pays the cost on every task, where provenance is a whole-diff
concern best swept once, and the broadened checklist already arms the per-task
early catch.

Promoting DEC-002's gate-summary record to a first-class journal block is a
deliberate future-spec candidate, recorded in `.speccy/BACKLOG.md`: build it
if dogfooding shows people want to query provenance-cleanup history rather
than read it from the diff.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-21 | Kevin Xiao | Initial SPEC: broaden the shared provenance convention to name the descriptive-prose, numbered-rule, and doc-path classes (REQ-001); ship a dedicated single-concern provenance-cleanup persona that shares the broadened definition via include (REQ-002); eject it through apply-mode per-host wrappers with a medium pin (REQ-003); and wire a once-per-invocation pre-ship cleanup phase into the vet flow that records via the gate summary, no new journal block (REQ-004). |
</changelog>
