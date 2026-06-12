---
spec: SPEC-0060
spec_hash_at_generation: bf215c47b8ba14e1d81bbc725b240f904d2c76bf3a77d1e8b2d03c39ab1259dd
generated_at: 2026-06-12T04:40:04Z
---
# Tasks: SPEC-0060 Context bundle journal slicing — latest round inlined in full, prior rounds as an attributes-only index

<task id="T-001" state="completed" covers="REQ-001">
## Slice `journal.blocks` to the latest round only

Today `build_journal` (`speccy-cli/src/context.rs:322`) projects every
entry across every round into `blocks`. Change it to inline only the
latest round.

Add a shared `latest_round(entries: &[JournalEntry]) -> Option<u32>`
free function to `speccy-core/src/parse/journal_xml/mod.rs`, beside
`JournalEntry::round()` — this is DEC-001's anti-drift realization: a
single definition that both journal views call. Replace the inline
`doc.entries.iter().map(JournalEntry::round).max()` at
`speccy-cli/src/journal_show.rs:209` with a call to it; `journal show`
behavior must stay byte-identical (existing journal-show tests green).

In `build_journal`, resolve the highest round via that helper and
project only entries whose `round` equals it through the existing
`to_json_journal_block` mapping into `blocks` (full bodies, file
order). Prior-round entries are not inlined here — their index is
REQ-002's job (T-002). The absent-journal contract is unchanged
(`exists: false`, empty `blocks`, exit 0); a journal that parses to
zero entries yields empty `blocks` with `exists: true`. Update the
`BundleJournal` doc-comment (`speccy-cli/src/context_output.rs:156`),
which currently claims `blocks` holds "every entry across all rounds".
The text renderer already iterates `bundle.journal.blocks`
(`context.rs:457`), so it renders latest-round-only content with no
further change — `--json` toggles representation, never content.

Rewrite the superseded `bundle_inlines_full_journal_with_all_blocks_and_rounds`
test (`speccy-cli/tests/context.rs:791`) to the latest-round contract:
its `blocks.len() == 8` and `rounds == [1,1,1,1,1,2,2,2]` assertions
encode the SPEC-0056 full-inline behavior this SPEC reverses and must
be replaced with round-2-only assertions. Keep the absent-journal
test. Add a single-round fixture test. Reuse the existing
`journal_two_rounds` fixture (five round-1 blocks, three round-2
blocks) as-is.

<task-scenarios>
Given the two-round journal fixture (five round-1 blocks, three
round-2 blocks),
when `speccy context SPEC-NNNN/T-NNN --json` runs,
then `journal.blocks` contains exactly one element per round-2 block,
each with `round == 2` and a non-empty `body`, and no round-1 body
marker appears anywhere in the serialized `blocks` array.

Given a task with no journal file,
when `speccy context` runs for it,
then the process exits 0 and the journal section carries
`exists: false` with an empty `blocks` array.

Given a single-round journal fixture,
when the bundle is emitted,
then `journal.blocks` contains every block of that journal with full
bodies.

Given the same journal file,
when `speccy journal show --round latest` and `speccy context` each
resolve the highest round,
then both resolve the identical round through the one shared
`latest_round` helper.

Suggested files: `speccy-core/src/parse/journal_xml/mod.rs`,
`speccy-cli/src/journal_show.rs`, `speccy-cli/src/context.rs`,
`speccy-cli/src/context_output.rs`, `speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-002">
## Add the attributes-only `journal.prior_rounds` index

Add the prior-rounds index so within-task history stays visible
without paying for its prose.

Add a `JsonJournalBlockAttrs` struct and a
`to_json_journal_block_attrs(entry: &JournalEntry) -> JsonJournalBlockAttrs`
projection to `speccy-cli/src/journal_show_output.rs`, beside their
full-block siblings (`JsonJournalBlock` / `to_json_journal_block`).
Per DEC-004 the new struct carries `block`, `date`, `round`, plus the
optional `model` / `persona` / `verdict` (keep `skip_serializing_if`
so a `blockers` index entry carries none of the three and an
`implementer` carries only `model`) and **no `body` field at all** —
the `body` key must be absent from the serialization, not emitted
empty, which is what makes index entries unambiguously distinguishable
from full blocks. Using a separate struct (rather than a serde-skip
flag on `JsonJournalBlock`) keeps the full-block type's `body`
invariant intact for `journal show` consumers.

Add `prior_rounds: Vec<JsonJournalBlockAttrs>` to `BundleJournal`
(`speccy-cli/src/context_output.rs`). In `build_journal`, project
every entry whose `round` is strictly below the highest round through
`to_json_journal_block_attrs` into `prior_rounds`, in file order. The
`blocks` / `prior_rounds` partition is total and disjoint — every
parsed entry lands in exactly one (round equals highest → `blocks`;
round below highest → `prior_rounds`), none dropped or duplicated.
`prior_rounds` is `[]` for single-round journals, zero-entry journals,
and absent journals.

Extend the `## Journal` text section (`context.rs:457`) to render a
prior-rounds index after the latest-round block bodies: one compact
line per attrs entry carrying its block type, round, and
persona / verdict where present, with no prior-round body content.

Tests (`speccy-cli/tests/context.rs`): assert `prior_rounds` against
the two-round fixture (one entry per round-1 block in file order, the
round-1 security review entry carries its `persona` and `verdict`, no
serialized `body` key, no round-1 body marker substring); assert `[]`
for single-round and absent journals; assert the text index renders
round-1 attributes with no round-1 bodies.

<task-scenarios>
Given the two-round journal fixture,
when `speccy context --json` emits the bundle,
then `journal.prior_rounds` has exactly one entry per round-1 block of
the fixture, in file order, the round-1 review entry carries its
`persona` and `verdict` values, and the serialized array contains no
`body` key and no round-1 body marker substring.

Given a single-round journal fixture and separately an absent journal,
when the bundle is emitted for each,
then `journal.prior_rounds` is `[]` in both bundles.

Given the two-round journal fixture,
when `speccy context` emits the text representation,
then the journal section renders the round-2 block bodies followed by
a prior-rounds index naming each round-1 block's type, round, and
persona / verdict where present, with no round-1 body content.

Suggested files: `speccy-cli/src/journal_show_output.rs`,
`speccy-cli/src/context.rs`, `speccy-cli/src/context_output.rs`,
`speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-003">
## Reword shipped prompt modules for the sliced journal and re-eject

Reword the source modules that describe the bundle's journal contents,
then re-eject so the shipped packs match source.

In `resources/modules/phases/speccy-work.md`, reword the entry
precondition that claims the bundle inlines "the full per-task
journal", and the retry-branch reads so they target the latest-round
inline `<implementer>` / `<blockers>` blocks. In
`resources/modules/skills/partials/review-fanout.md`, reword the "full
per-task journal" claim. Both gain the drill-down affordance: prior
rounds appear in the bundle as an attributes-only index, and a
specific prior block is fetched via
`speccy journal show <selector> --round N [--block <type>]`.
`resources/modules/references/retry-shape.md` already routes through
`journal show --round latest` and needs no change.

Run `just reeject` so the ejected packs under `.claude/`, `.agents/`,
and `.codex/` match the updated source. Never hand-edit the ejected
files.

<task-scenarios>
Given the updated modules committed,
when `just reeject` runs followed by `git status --porcelain` over
`.claude/`, `.agents/`, and `.codex/`,
then the output is empty.

Given the updated `speccy-work.md` and `review-fanout.md` source
modules,
when reviewed against this SPEC,
then each describes the latest-round inline blocks plus the
prior-rounds index, names the `journal show --round N` drill-down, and
makes no full-journal claim. (Review-verified; prose content is not
substring-gated by tests per AGENTS.md.)

Suggested files: `resources/modules/phases/speccy-work.md`,
`resources/modules/skills/partials/review-fanout.md` (then
`just reeject` regenerates `.claude/`, `.agents/`, `.codex/`)
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-004">
## Document the sliced journal envelope in ARCHITECTURE.md

Update the `speccy context` envelope documentation in
`docs/ARCHITECTURE.md` to match the shipped shape.

Reword the journal bullet (currently "the full per-task journal
inlined — all blocks across rounds") to describe `journal.blocks` as
the latest round's blocks with full bodies and `journal.prior_rounds`
as the attributes-only index of pre-latest blocks, plus the
absent-journal marker and the `journal show --round N` drill-down.
Make the implementer/reviewer entry-flow narrative passages that
reference the journal-from-bundle contract consistent with the new
shape, so no passage claims the bundle inlines all rounds. Note in the
size-invariant prose that within-task prior rounds add only bounded
index entries.

<task-scenarios>
Given the updated `docs/ARCHITECTURE.md`,
when reviewed against the implemented envelope,
then the journal-section documentation matches the emitted JSON
field-for-field (`blocks` = latest-round blocks with bodies,
`prior_rounds` = attributes-only index) and no full-journal claim
remains. (Review-verified.)

Given the updated ARCHITECTURE.md,
when an agent reads the `speccy context` section before touching the
code,
then the documented envelope matches the shape shipped by REQ-001 and
REQ-002.

Suggested files: `docs/ARCHITECTURE.md`
</task-scenarios>
</task>
