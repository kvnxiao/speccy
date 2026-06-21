---
id: SPEC-0067
slug: bounded-memory-ledger
title: Bounded memory ledger — a higher capture bar, one-line entries, and autonomous compaction keep `.speccy/MEMORY.md` small and high-signal
status: implemented
created: 2026-06-21
supersedes: []
---

# SPEC-0067: Bounded memory ledger — a higher capture bar, one-line entries, and autonomous compaction keep `.speccy/MEMORY.md` small and high-signal

## Summary

SPEC-0064 shipped the per-repo loop memory: a `.speccy/MEMORY.md` ledger the
implementer reads before acting, grown by a ship-time retro. Dogfooding on a
sibling project exposed two failure modes the original design baked in. First,
the retro mandates at least one mistake-flavoured entry per loop that recorded
friction (REQ-004 of SPEC-0064), regardless of whether the lesson is durable or
already enforced by an existing reviewer persona or `AGENTS.md` rule — so the
ledger grows roughly one entry per friction event. Second, each entry uses a
four-part shape (REQ-002 of SPEC-0064) with multi-sentence prose and a full-line
provenance citation, reading like an incident report. The sibling project's first
spec produced a 113-line ledger of 6 entries, 5 of which restate a convention an
existing reviewer already blocks on. The ledger grows monotonically and burns
implementer context as it grows — the opposite of the high-signal slice it was
meant to carry forward.

The root cause is intake, not size. The backstop that would prune redundancy —
the consolidate-and-dedupe step (REQ-005 of SPEC-0064) — is human-gated, so it
never fires in an autonomous `orchestrate → ship` run. This SPEC fixes intake
and compaction instead of bolting on a size cap. It raises the capture bar so
the retro records an entry only when the lesson is both durable across specs and
not already enforced by a gate, reviewer persona, or `AGENTS.md`/rule — making
"record nothing" the default, friction notwithstanding. It compresses an entry
to a single line — a trigger, a corrective rule, and a compact provenance tag —
dropping the mistake narrative that carried no forward signal. And it splits the
bounding work so the parts that can only shrink the file (refuse-to-append,
within-ledger dedupe) run autonomously, leaving only promotion into the durable
tier — the one mutation that edits a human-owned doc — behind a human gate.

The change is prose-only: edits to the ship-phase retro body and the
memory-ledger entry-shape reference (plus a wording fix in the read-side
summary). No CLI verb, no new lint, no `speccy verify` coupling — every
soft-guidance invariant SPEC-0064 established is preserved.

## Goals

<goals>
- The ship-time retro records a ledger entry only when the lesson is durable
  across specs and not already enforced by an existing gate, reviewer persona,
  or `AGENTS.md`/rule; the "at least one entry per friction loop" mandate is
  removed and recording nothing is the default outcome.
- A ledger entry is a single line carrying a trigger, a corrective rule, and a
  compact provenance tag that still resolves to a real SPEC/task, with no
  mistake/history narrative.
- The bounding work that can only shrink the ledger — refusing a redundant
  append and within-ledger dedupe — runs autonomously in an `orchestrate → ship`
  run with no human in the loop.
- Promotion of a stable entry into the durable tier (`AGENTS.md`/rules) stays
  the single human-gated memory mutation, and the ledger stays bounded even if
  promotion never runs.
- Every SPEC-0064 soft-guidance invariant is preserved: the ledger remains
  eject-safe, implementer-only on read, and outside every `speccy verify` lint.
</goals>

## Non-goals

<non-goals>
- No hard entry cap or eviction policy. Bounding is achieved by raising intake
  and compacting, not by counting entries and evicting at a threshold — picking
  which entry to evict is a semantic judgment the CLI must not own and an
  autonomous agent gets wrong.
- No `speccy memory` CLI verb. Capture, slicing, and compaction stay prose-layer
  behaviours, exactly as SPEC-0064 left them; the deferred verb is still
  deferred.
- No new lint family, no enforcement, no `--strict` coupling, no CLI change.
  Memory stays soft guidance; `speccy verify` still never reads the ledger.
- No bulk migration of ledgers already on disk. Pre-existing four-part entries
  are reformatted opportunistically when the retro touches them, never by a
  dedicated migration pass.
- Promotion into the durable tier does not become autonomous. It edits a
  human-owned doc, so it keeps its human gate.
- Reviewers and vet still do not read the ledger; no feed-forward attach point
  is added to them.
</non-goals>

## User Stories

<user-stories>
- As a solo developer running many specs through one repo, I want
  `.speccy/MEMORY.md` to stay small and high-signal as the project grows, so the
  ledger I load each task does not balloon into a context tax that dilutes the
  lessons that matter.
- As an implementer subagent, I want each ledger entry to be a one-line rule I
  can apply at a glance, not a multi-paragraph incident report I have to read
  through to extract the actionable instruction.
- As someone driving `orchestrate → ship` unattended, I want the ledger kept
  bounded without a human approving each prune, so the loop does not silently
  accumulate redundant entries whenever no human is present to run the gated
  consolidation.
- As a maintainer, I want lessons promoted into `AGENTS.md` to still pass under
  my eye, so machine-proposed guidance never silently mutates the hand-authored
  north star.
</user-stories>

## Assumptions

<assumptions>
- A loop that records no durable lesson writes nothing to the ledger and leaves
  no "no lesson this loop" sentinel line — a per-loop marker is itself the
  growth this SPEC removes.
- Within-ledger compaction may merge or reword obvious near-duplicate lines but
  never deletes a non-redundant entry, so the operation only ever shrinks the
  file and is safe to run unattended.
- Legacy verbose entries the retro encounters are reformatted into the one-line
  shape in passing; there is no separate migration of untouched entries.
- The provenance tag is `[SPEC-NNNN/T-NNN]`, dropping the `/T-NNN` segment only
  when a lesson is not task-specific; it always resolves to a real SPEC (and
  task, when present), preserving the auditability SPEC-0064 required.
- Verification follows SPEC-0064's precedent (its DEC-009): structural
  invariants plus dogfooding scenarios, with no scenario asserting that specific
  sentences appear in any skill or subagent body.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: The retro records an entry only when the lesson is durable and not already gate-enforced

The ship-time retro replaces SPEC-0064's "a loop with recorded friction yields
at least one mistake-flavoured entry" mandate with a two-part capture bar: it
appends an entry only when the lesson is both durable across specs and not
already enforced by an existing gate, reviewer persona, or `AGENTS.md`/rule. When
a gate already catches the lesson, the gate is the memory and the retro records
nothing. Recording nothing is the default outcome, including for a loop that hit
friction whose only lesson an existing persona already enforces.

<done-when>
- The ship-phase retro body states the durable-and-not-already-enforced bar and
  no longer mandates at least one entry per friction loop.
- A loop whose only friction is a blocking verdict an existing reviewer persona
  already enforces ships with zero new ledger entries.
- A loop that surfaces a genuinely new, durable, not-yet-enforced lesson records
  exactly that lesson.
</done-when>

<behavior>
- Given a shipped spec whose journal shows a blocking-then-passed review round
  that an existing reviewer persona already enforces, when the retro runs, then
  it appends no entry for that round.
- Given a shipped spec that surfaced a durable convention no gate or `AGENTS.md`
  rule covers, when the retro runs, then it appends one entry for it.
</behavior>

<scenario id="CHK-001">
Given a spec shipped through the loop whose only recorded friction is a
style-reviewer block on a convention `AGENTS.md` already states,
when the ship-time retro runs during dogfooding,
then `.speccy/MEMORY.md` gains no entry for that friction — a dogfood check that
the capture bar suppresses gate-redundant lessons, since "already enforced
elsewhere" is a semantic judgment no prose-substring test can gate.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: A ledger entry is one line — trigger, corrective rule, and a compact provenance tag

A ledger entry is a single line carrying a trigger (the situation a future
implementer matches against), a corrective rule (the action to take), and a
compact provenance tag, with no mistake or history narrative. The provenance tag
is bracketed and resolves to a real SPEC and task — dropping the task segment
only for a spec-wide lesson — replacing SPEC-0064's four-part shape and its
full-line provenance citation. The shape is a documented prose/markdown
convention; no parser or CLI grammar is added.

<done-when>
- The memory-ledger entry-shape reference documents the one-line shape and its
  worked example is a single line of that shape.
- A retro-written entry is one line carrying a trigger, a corrective rule, and a
  bracketed provenance tag, and carries no mistake/history narrative.
- The provenance tag resolves to a real SPEC identifier (and task, when the
  lesson is task-specific), not a fabricated one.
</done-when>

<behavior>
- Given the retro captures a durable lesson, when it writes the entry, then the
  entry is one line of the form trigger, then a corrective rule, then a bracketed
  provenance tag, illustrated as:

```text
TRIGGER → CORRECTIVE RULE. [SPEC-NNNN/T-NNN]
```

- Given a lesson that is not specific to one task, when the entry is written,
  then its provenance tag is `[SPEC-NNNN]` with the task segment omitted.
</behavior>

<scenario id="CHK-002">
Given this SPEC (or a later one) ships through the loop and the retro captures a
durable lesson,
when that entry is written to `.speccy/MEMORY.md`,
then manual inspection confirms it is a single line carrying a trigger, a
corrective rule, and a bracketed provenance tag resolving to the producing
SPEC/task, with no narrative field — a dogfood check, since one-line conformance
and tag resolvability are semantic properties no prose-substring test can gate.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Refuse-to-append and within-ledger dedupe run autonomously

The bounding work that can only shrink the ledger runs without human approval in
an `orchestrate → ship` run: before appending, the retro drops a candidate
already covered by an existing ledger line or a durable doc, and merges a new
lesson into a near-duplicate existing line rather than adding a second. This is
the dedupe SPEC-0064 placed behind a human gate; because every outcome either
prevents a write or shrinks the file, it is safe to run unattended and is moved
out of the human-gated path. Compaction never deletes a non-redundant entry.

<done-when>
- The retro performs candidate dedupe against the ledger and the durable docs,
  and near-duplicate merge, with no human-approval step.
- In an `orchestrate → ship` run with no human present, a candidate duplicating
  an existing entry is not appended.
- Compaction only refuses an append, merges, or shrinks; it never removes a
  non-redundant entry.
</done-when>

<behavior>
- Given an autonomous `orchestrate → ship` run and a candidate lesson already
  covered by an existing ledger line, when the retro runs, then the duplicate is
  not appended and no human approval is solicited.
- Given a candidate that is a near-duplicate of an existing line, when the retro
  runs, then the two are merged into one line rather than left as two.
</behavior>

<scenario id="CHK-003">
Given a `.speccy/MEMORY.md` already holding an entry, and an autonomous
`orchestrate → ship` run whose retro derives a candidate duplicating that entry,
when the retro runs with no human in the loop,
then the duplicate is not appended and the ledger does not grow — a dogfood check
that intake dedupe bounds the ledger autonomously, not behind a gate.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Promotion into the durable tier stays the single human-gated mutation

Promotion of a stable, repeatedly-affirmed entry up into the durable tier
(`AGENTS.md`/rules) remains human-gated, and is the only memory mutation that
requires human approval. The ledger's boundedness no longer depends on promotion
firing — REQ-003's autonomous compaction keeps it bounded — so an autonomous run
that never promotes still does not accumulate redundant entries. On approval, a
promoted entry is removed from the ledger so it is not stored in both tiers.

<done-when>
- The retro proposes ledger-to-durable promotion and requires human approval
  before the durable-tier edit lands; no other memory mutation is gated.
- A promoted entry is removed from `.speccy/MEMORY.md` on approval.
- An autonomous run that promotes nothing still keeps the ledger bounded via
  REQ-003 compaction.
</done-when>

<behavior>
- Given a stable entry affirmed across multiple specs, when the retro runs, then
  it surfaces a promotion proposal for human approval and does not promote
  silently or automatically.
- Given an autonomous `orchestrate → ship` run with a promotion-worthy entry,
  when the retro runs, then the entry is not auto-promoted, while autonomous
  compaction still runs.
</behavior>

<scenario id="CHK-004">
Given a `.speccy/MEMORY.md` containing a stable, promotion-worthy entry and an
autonomous `orchestrate → ship` run,
when the retro runs with no human present,
then the entry is surfaced for human-gated promotion rather than auto-promoted,
while autonomous compaction still bounds the ledger — a dogfood check that
promotion keeps its gate without that gate being load-bearing for boundedness.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: The read-side summary reflects the one-line shape

The implementer-facing read-side summary
(`resources/modules/references/memory-ledger-summary.md`) describes the one-line
entry shape rather than the four-part shape, so the read instruction and the
write instruction agree. The summary remains a single `{% include %}` in the
canonical implementer module body with no shadowing inline copy in any host
wrapper — the no-duplicate-snippet invariant SPEC-0064 established.

<done-when>
- The read-side summary describes the one-line entry shape, not the four-part
  shape.
- The summary include appears exactly once in the canonical implementer module
  body and is inlined in no host wrapper.
- After `just reeject`, the ejected implementer body carries the updated summary
  with no Conflict against the rendered module.
</done-when>

<behavior>
- Given the read-side summary and the entry-shape reference after this change,
  when both are read, then they describe the same one-line shape with no residual
  four-part description.
- Given the canonical implementer module and the ejected host bodies after
  `just reeject`, when the source-of-truth placement is checked, then the summary
  include exists once in the module and no shadowing inline copy exists in any
  host wrapper.
</behavior>

<scenario id="CHK-005">
Given the canonical implementer module and every host wrapper after this change
and a `just reeject`,
when the include-placement is checked,
then the memory-ledger-summary include appears exactly once in
`resources/modules/phases/speccy-work.md` and in no host wrapper — a structural
check against the no-duplicate-snippet invariant, the same surface the existing
memory feed-forward test keys on.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
Bounding is achieved by raising the intake bar (REQ-001) and compacting
(REQ-003), not by a hard entry cap with eviction. A cap fights the symptom
rather than the cause — the sibling-repo evidence showed 5 of 6 entries should
never have been written, so a higher bar collapses growth at the source. A cap
also forces an eviction decision ("which entry dies?") that is a semantic
judgment the deterministic CLI must not own and an autonomous agent makes badly.
Rejected: hard cap + eviction policy.
</decision>

<decision id="DEC-002">
Dedupe is folded into capture-time and runs autonomously (REQ-003) rather than
living in a separate, human-gated consolidation pass (as SPEC-0064 had it) or a
standalone periodic compaction phase. Refusing to write is always safe, so the
operation needs no human gate; and the ship retro already runs every loop, so a
separate phase would add a trigger without adding coverage. This is the change
that makes bounding actually fire in an autonomous `orchestrate → ship` run,
where SPEC-0064's human-gated step never did.
</decision>

<decision id="DEC-003">
Promotion into the durable tier stays human-gated (REQ-004); it is not made
autonomous. Promotion edits `AGENTS.md`/rules — the always-loaded, hand-authored
north star — so a human approves each graduation, preserving SPEC-0064's DEC-006.
This is deliberately the one memory mutation that keeps a gate; because REQ-003
bounds the ledger autonomously, keeping promotion gated no longer costs
boundedness. Rejected: autonomous promotion with bounded autonomy.
</decision>

<decision id="DEC-004">
The entry shape is one line — trigger, corrective rule, compact provenance tag —
dropping SPEC-0064's four-part shape and its mistake/history field. The history
of how a lesson was learned is not forward signal; the corrective rule is. The
provenance tag is compressed to a bracketed `[SPEC-NNNN/T-NNN]` form that still
resolves to a real SPEC/task, preserving the auditability SPEC-0064 required
while cutting the per-entry footprint roughly five-fold.
</decision>

<decision id="DEC-005">
No CLI lint enforces the one-line shape or the tag form. The ledger stays soft
guidance the CLI never reads (SPEC-0064 REQ-007 / DEC-009); a format lint would
breach the deterministic-core / feedback-not-enforcement boundary and re-import
the substring-matching-curated-prose anti-pattern the project bans. Shape
adherence is a persona-review and dogfooding judgment, not a scriptable gate.
</decision>

<decision id="DEC-006">
Legacy verbose entries already on disk are reformatted opportunistically when
the retro touches them, with no bulk migration pass. A one-shot migration would
mutate user-owned content the loop has no reason to touch this ship; touch-time
reformatting slims old ledgers gradually as their entries become relevant again,
and leaves untouched entries byte-stable in the meantime.
</decision>

## Notes

This SPEC revises SPEC-0064's REQ-002 (entry shape), REQ-004 (capture mandate),
and REQ-005 (consolidation gate) in light of dogfooding evidence; it does not
replace SPEC-0064, whose ledger-location, eject-safety, implementer-only read,
and no-verify-gate invariants (its REQ-001/003/006/007) carry forward unchanged.
Rejected framings (hard cap, standalone compaction phase, autonomous promotion,
format lint) are recorded in DEC-001 through DEC-005.

**Scope boundary.** Changes land in `resources/modules/phases/speccy-ship.md`
(the retro step), `resources/modules/references/memory-ledger.md` (the entry
shape and authoring discipline), and `resources/modules/references/memory-ledger-summary.md`
(the read-side wording), plus a `just reeject` to regenerate the ejected host
bodies. The speccy Rust CLI is not touched; `docs/SCHEMA.md` describes the
ledger's role, not its entry shape, so it needs no edit. The existing memory
feed-forward test keys on the include structure, not the prose, so it stays
green.

## Open Questions

None — the capture bar, entry format, compaction autonomy, promotion gate,
no-sentinel default, near-duplicate merge behaviour, legacy-reformat policy, and
provenance-tag granularity were all resolved during brainstorm and are recorded
as assumptions and DEC-001 through DEC-006.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-21 | Kevin Xiao | Initial SPEC: bound and slim `.speccy/MEMORY.md` — raise the capture bar to durable-and-not-already-enforced (REQ-001), compress entries to one line with a compact provenance tag (REQ-002), run refuse-to-append and within-ledger dedupe autonomously (REQ-003), keep durable-tier promotion the single human gate (REQ-004), and align the read-side summary (REQ-005). Revises SPEC-0064 REQ-002/004/005; prose-only, no CLI change. |
</changelog>
