---
id: SPEC-0060
slug: context-journal-slicing
title: Context bundle journal slicing — latest round inlined in full, prior rounds as an attributes-only index
status: in-progress
created: 2026-06-11
supersedes: []
---

# SPEC-0060: Context bundle journal slicing — latest round inlined in full, prior rounds as an attributes-only index

## Summary

`speccy context` (SPEC-0056) inlines the full per-task journal into
the bundle: `build_journal` at `speccy-cli/src/context.rs:322`
projects every entry across every retry round. On long-running tasks
this dominates the bundle — for `SPEC-0055/T-004` (8 rounds, 49
blocks) the journal contributes 128.7KB of a 141.7KB bundle, while
the `journal show --round latest` slice of the same file is 15.9KB.
Every loop subagent spawned in late rounds pays that cost, and late
rounds are exactly where journals are fattest.

The shipped prompts already operate on the latest round only:
`retry-shape.md` defines retry shape via `--round latest`, and
`phases/speccy-work.md` instructs the implementer to read the most
recent `<implementer>` and `<blockers>` blocks. The full-history
payload is mechanism-level waste under instruction-level slicing.

This SPEC changes the bundle's journal section to carry the latest
round's blocks in full plus an attributes-only index of prior-round
blocks (no bodies). Prior-round bodies stay on disk, reachable via
`speccy journal show <selector> --round N`. The index preserves the
"know when to look" affordance: an agent sees the shape of history
(round, block type, persona, verdict) without paying for its prose.

Grounding: every per-task journal entry carries a mandatory `round`
(`speccy-core/src/parse/journal_xml/mod.rs:71,89,100`;
`JournalEntry::round()` is total), so "latest" is a clean equality
filter against the file-wide maximum with no round-less special case.
This SPEC supersedes the behavior pinned by SPEC-0056 REQ-004
("inlines its full content ... across all rounds") for the journal
section only; the rest of the SPEC-0056 envelope is untouched.

## Goals

<goals>
- The bundle's journal section scales with the size of the latest
  round plus a bounded per-block index entry, not with the task's
  total round count.
- A retry-round implementer still obtains the latest `<implementer>`
  and `<blockers>` bodies in one `speccy context` read; the
  single-read entry property of SPEC-0056 is preserved.
- Prior-round history remains fully reachable on demand via
  `speccy journal show <selector> --round N`, and the bundle's index
  tells the agent that history exists and what shape it has.
- The envelope keeps `schema_version: 1` and all existing field
  names; the change is additive plus a content-semantics change to
  `journal.blocks`.
</goals>

## Non-goals

<non-goals>
- No cumulative "approaches tried" field in the `<implementer>`
  block grammar. That is a separate follow-up SPEC.
- No round-count escalation ladder in the orchestrate skill (e.g.
  "round ≥ 4 → decompose"). Separate follow-up.
- No changes to `speccy journal show`, its filters, or the on-disk
  journal grammar. The filter semantics this SPEC reuses already
  exist there.
- No VET.md bundle handling. `speccy context` accepts task selectors
  only; VET journals are out of scope.
- No `schema_version` bump and no renaming of existing envelope
  fields.
</non-goals>

## User Stories

<user-stories>
- As an implementer subagent on retry round N, I want the bundle
  sized by the latest round rather than all N rounds, so my context
  budget goes to the requirements and the code instead of stale
  handoff prose.
- As a reviewer persona, I want to see that prior rounds exist and
  which personas blocked with what verdicts, so I can detect
  recurrence and pull a specific prior block via `journal show` only
  when it matters.
- As the harness operator, I want worst-case bundle reads to drop
  roughly 5x in tokens without losing any on-disk history or audit
  trail.
</user-stories>

## Assumptions

<assumptions>
- Every per-task `JournalEntry` variant carries a mandatory
  `round: u32` (`Implementer`, `Review`, `Blockers`), so the
  latest-round filter needs no round-less carve-out. Verified at
  `speccy-core/src/parse/journal_xml/mod.rs:71,89,100`.
- The only consumers of the bundle's journal section are the shipped
  skill prompts, which already read the latest round; no external
  consumer is pinned to full-history `blocks` content.
- A journal whose latest round lacks some block type (e.g. round N
  has an `<implementer>` but no `<review>` yet) is normal mid-loop
  state; the filter is a mechanical equality on `round`, not a
  completeness check.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: `journal.blocks` carries only the latest round's blocks in full

The bundle's `journal.blocks` array contains exactly the blocks whose
`round` equals the journal's highest round, in file order, with full
bodies — projected via the existing `to_json_journal_block` mapping.
The highest round resolves identically to
`speccy journal show --round latest` (maximum of
`JournalEntry::round()` across all entries). The absent-journal
contract from SPEC-0056 REQ-004 is unchanged: `exists: false`, empty
arrays, exit 0. A journal that exists but parses to zero entries
yields empty arrays with `exists: true`. The text representation
renders the same latest-round-only content (`--json` toggles
representation, never content).

<done-when>
- Against a two-round fixture journal, `journal.blocks` contains
  only the round-2 blocks, each with its full body.
- Against a single-round fixture journal, `journal.blocks` contains
  every block.
- An absent journal yields `exists: false` with empty `blocks` and
  exit 0.
- The latest round resolved by `context` equals the round
  `journal show --round latest` resolves for the same file.
</done-when>

<behavior>
- Given a journal with rounds 1..=2, when `speccy context
  SPEC-NNNN/T-NNN --json` runs, then every element of
  `journal.blocks` has `round == 2` and a non-empty `body`.
- Given a journal whose only entries are round 1, when the bundle is
  emitted, then `journal.blocks` carries all of them unchanged.
- Given no journal file, when the bundle is emitted, then the
  journal section is the explicit empty marker and the exit code
  is 0.
</behavior>

<scenario id="CHK-001">
Given the two-round journal fixture (five round-1 blocks, three
round-2 blocks) used by the existing
`bundle_inlines_full_journal_with_all_blocks_and_rounds` test,
when `speccy context` emits the bundle for that task,
then `journal.blocks` contains exactly one element per round-2
block of the fixture, each with `round == 2` and its full body, and
the round-1 body markers appear nowhere in the serialized `blocks`
array.
</scenario>

<scenario id="CHK-002">
Given a task with no journal file,
when `speccy context` runs for it,
then the process exits 0 and the journal section carries
`exists: false` with an empty `blocks` array.
</scenario>

<scenario id="CHK-003">
Given a single-round journal fixture,
when the bundle is emitted,
then `journal.blocks` contains every block of that journal with
full bodies.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: `journal.prior_rounds` is an attributes-only index of pre-latest blocks

The journal section gains a `prior_rounds` array: one entry per
block whose `round` is strictly below the highest round, in file
order. Each entry carries the `JsonJournalBlock` attribute fields —
`block`, `date`, `round`, and the optional `model` / `persona` /
`verdict` where the block type defines them — with no `body` field
serialized at all (key absent, not empty string). `prior_rounds` is
empty for single-round journals, zero-entry journals, and absent
journals. The text representation renders one compact index line per
prior-round block carrying the same attributes.

<done-when>
- Against the two-round fixture, `prior_rounds` has one entry per
  round-1 block, in file order.
- Every journal entry appears in exactly one of `blocks` (its round
  equals the highest) or `prior_rounds` (its round is below the
  highest); no entry is dropped or duplicated.
- No `prior_rounds` entry's JSON serialization contains a `body`
  key.
- A `review` index entry carries `persona` and `verdict`; an
  `implementer` entry carries `model`; a `blockers` entry carries
  none of the three optionals.
- Single-round, zero-entry, and absent journals all yield
  `prior_rounds: []`.
- The text output renders a prior-rounds index section listing each
  entry's attributes without any block body content.
</done-when>

<behavior>
- Given a journal with rounds 1..=2, when the bundle is emitted as
  JSON, then `prior_rounds` lists the round-1 blocks' attributes and
  none of their prose.
- Given a single-round journal, when the bundle is emitted, then
  `prior_rounds` is an empty array.
- Given the two-round fixture, when the bundle is emitted as text,
  then the journal section shows the round-2 blocks in full followed
  by an index of round-1 entries (block type, round, and persona /
  verdict where present).
</behavior>

<scenario id="CHK-004">
Given the two-round journal fixture,
when `speccy context --json` emits the bundle,
then `journal.prior_rounds` has exactly one entry per round-1 block
of the fixture, in file order, the round-1 review entry carries its
`persona` and `verdict` values, and the serialized array contains
no `body` key and no round-1 body marker substring.
</scenario>

<scenario id="CHK-005">
Given a single-round journal fixture and separately an absent
journal,
when the bundle is emitted for each,
then `journal.prior_rounds` is `[]` in both bundles.
</scenario>

<scenario id="CHK-006">
Given the two-round journal fixture,
when `speccy context` emits the text representation,
then the journal section renders the round-2 block bodies and a
prior-rounds index naming each round-1 block's type and attributes,
with no round-1 body content.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Shipped prompt modules describe the sliced journal section

The source modules that describe the bundle's journal contents are
reworded for the new shape: `resources/modules/phases/speccy-work.md`
(the entry precondition currently claiming "the full per-task
journal", and the retry-branch reads, which now target the
latest-round inline blocks) and
`resources/modules/skills/partials/review-fanout.md` (the "full
per-task journal" claim). Both gain the drill-down affordance: prior
rounds appear in the bundle as an attributes-only index, and a
specific prior block is fetched via `speccy journal show <selector>
--round N [--block <type>]`. `resources/modules/references/retry-shape.md`
already routes through `journal show --round latest` and needs no
change. Packs are re-ejected so the shipped output matches source.

<done-when>
- No shipped module states that the bundle inlines the full journal
  or all rounds.
- `phases/speccy-work.md`'s retry branch reads the latest
  `<implementer>` / `<blockers>` blocks from the bundle's inline
  latest-round section.
- `review-fanout.md` describes prior rounds as index entries with a
  `journal show --round N` drill-down.
- `just reeject` leaves no diff under `.claude/`, `.agents/`, or
  `.codex/` (ejected packs in sync with source).
</done-when>

<behavior>
- Given the updated source modules, when `just reeject` runs, then
  the working tree shows no changes under the three ejected pack
  roots.
- Given a retry-round implementer following the updated
  `speccy-work.md`, when it opens its bundle, then the blocks it is
  told to read are present inline and no instruction references
  full-history bundle content.
</behavior>

<scenario id="CHK-007">
Given the updated modules committed,
when `just reeject` runs followed by `git status --porcelain` over
`.claude/`, `.agents/`, and `.codex/`,
then the output is empty.
</scenario>

<scenario id="CHK-008">
Given the updated `speccy-work.md` and `review-fanout.md` source
modules,
when reviewed against this SPEC,
then each describes the latest-round inline blocks plus the
prior-rounds index, names the `journal show --round N` drill-down,
and makes no full-journal claim. (Review-verified; prose content is
not substring-gated by tests per AGENTS.md.)
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: ARCHITECTURE.md documents the sliced journal envelope

The `speccy context` envelope documentation in
`docs/ARCHITECTURE.md` is updated: the journal bullet (currently
"the full per-task journal inlined — all blocks across rounds")
describes the latest-round-full + prior-rounds-index shape, the
absent-journal marker, and the `journal show --round N` drill-down;
the implementer/reviewer entry-flow narrative passages that reference
the journal-from-bundle contract are made consistent with the new
shape.

<done-when>
- The envelope section describes `journal.blocks` as latest-round
  blocks with bodies and `journal.prior_rounds` as the
  attributes-only index.
- No ARCHITECTURE.md passage claims the bundle inlines all rounds.
- The size-invariant prose notes that within-task prior rounds add
  only bounded index entries.
</done-when>

<behavior>
- Given the updated ARCHITECTURE.md, when an agent reads the
  `speccy context` section before touching the code, then the
  documented envelope matches the implementation shipped by REQ-001
  and REQ-002.
</behavior>

<scenario id="CHK-009">
Given the updated `docs/ARCHITECTURE.md`,
when reviewed against the implemented envelope,
then the journal-section documentation matches the emitted JSON
field-for-field and no full-journal claim remains.
(Review-verified.)
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
"Latest round" resolves exactly as `journal show --round latest`
does for per-task journals: the maximum of `JournalEntry::round()`
across all entries, then an equality filter. The implementation
shares or mirrors that resolution so the two journal views cannot
drift — extending SPEC-0056's anti-drift decision to reuse
`journal show`'s block projection (`JsonJournalBlock` /
`to_json_journal_block`).
</decision>

<decision id="DEC-002">
`schema_version` stays 1 and existing field names (`blocks`) are
kept. The shapes of existing fields are unchanged; `prior_rounds` is
additive inside the journal section; the first-field contract is
unaffected. The content-semantics change to `blocks` is documented
in ARCHITECTURE.md rather than versioned: envelope versioning is
reserved for shape breaks, and the only known consumers (the shipped
prompts) already operate on the latest round.
</decision>

<decision id="DEC-003">
Prior-round bodies are never inlined anywhere in the bundle. The
drill-down is `speccy journal show <selector> --round N
[--block <type>]`. The index carries `round`, `block`, `persona`,
and `verdict`, which is sufficient for recurrence detection (the
same persona blocking across rounds) without bodies; an agent that
needs prose drills in explicitly.
</decision>

<decision id="DEC-004">
The index entry shape is `JsonJournalBlock` minus `body`: `block`,
`date`, `round`, plus optional `model` / `persona` / `verdict` per
block type, projected by an attributes-only sibling of
`to_json_journal_block`. The `body` key is omitted from
serialization entirely rather than emitted empty, so index entries
are unambiguously distinguishable from full blocks.
</decision>

## Notes

Dogfood evidence motivating the slice (local identifiers are
intentional here): `SPEC-0055/T-004` ran 8 rounds and 49 blocks; its
journal contributed 128.7KB of a 141.7KB bundle (91%), while the
`--round latest` slice of the same journal is 15.9KB. The bundle
drop is roughly 142KB → ~29KB for that worst case, paid on every
late-round subagent spawn.

The repercussion analysis behind the index: latest-round-only with
no index risks dead-end amnesia (a round-8 implementer re-attempting
an approach rejected in round 3) because nothing signals that
history exists. The attributes-only index restores the signal at
~100 bytes per block. The deeper fix — a cumulative "approaches
tried" field in the implementer block grammar — is deliberately
deferred to a follow-up SPEC (see Non-goals) so this slice stays
mechanical.

SPEC-0056 remains `implemented` and is not amended; this SPEC
supersedes its REQ-004 full-inline behavior going forward, recorded
here rather than via frontmatter `supersedes` (which is reserved for
whole-SPEC supersession).

## Open Questions

- [x] a. **Self-review caught:** REQ-002 fixes the two-round
  fixture's `prior_rounds` count at "exactly five entries"
  (CHK-004), which couples the CHK to the current fixture layout in
  `speccy-cli/tests/context.rs`; if decompose reshapes the fixture,
  the scenario's count must be re-derived from the fixture rather
  than treated as a contract value. **Resolved:** CHK-001 and
  CHK-004 rephrased to one-element-per-fixture-block properties, and
  REQ-002's done-when gained the partition invariant (every entry in
  exactly one of `blocks` / `prior_rounds`); counts now derive from
  the fixture instead of being contract values.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-11 | claude-fable-5 | Initial SPEC: bundle journal section carries latest-round blocks in full (REQ-001) plus attributes-only prior-rounds index (REQ-002); shipped prompt modules reworded with `journal show --round N` drill-down (REQ-003); ARCHITECTURE.md envelope docs updated (REQ-004). DEC-001 reuses `--round latest` resolution; DEC-002 keeps `schema_version: 1`; DEC-003 never inlines prior bodies; DEC-004 index = `JsonJournalBlock` minus `body`. Supersedes SPEC-0056 REQ-004 full-inline behavior for the journal section only. |
| 2026-06-11 | claude-fable-5 | Resolved open question a.: CHK-001/CHK-004 fixture counts rephrased as one-element-per-block properties; REQ-002 done-when gained the blocks/prior_rounds partition invariant. No requirement intent changed. |
</changelog>
