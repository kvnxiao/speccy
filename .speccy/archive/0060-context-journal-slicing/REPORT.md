---
spec: SPEC-0060
outcome: implemented
generated_at: 2026-06-12T00:00:00Z
---

# REPORT: SPEC-0060 Context bundle journal slicing — latest round inlined in full, prior rounds as an attributes-only index

<report spec="SPEC-0060">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-001 introduced `latest_round(entries: &[JournalEntry]) -> Option<u32>` in
`speccy-core/src/parse/journal_xml/mod.rs` as the shared resolver for both
`journal show --round latest` and `speccy context`. `build_journal` in
`speccy-cli/src/context.rs` now uses that helper to find the highest round and
projects only entries with `round == highest` through `to_json_journal_block`
into `blocks` (full bodies, file order). The prior
`bundle_inlines_full_journal_with_all_blocks_and_rounds` test was rewritten to
assert round-2-only blocks; absent-journal and single-round fixture tests
confirm the unchanged absent-journal contract and correct single-round behavior.
Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
T-002 added `JsonJournalBlockAttrs` and `to_json_journal_block_attrs` to
`speccy-cli/src/journal_show_output.rs`, carrying `block`, `date`, `round`,
and the optional `model` / `persona` / `verdict` with no `body` field.
`BundleJournal` gained `prior_rounds: Vec<JsonJournalBlockAttrs>` populated
from every entry whose round is strictly below the highest. Tests assert the
two-round fixture produces one entry per round-1 block in file order with
`persona` and `verdict` on the review entry and no `body` key serialized;
single-round and absent journals yield `[]`. The text renderer appends a
compact prior-rounds index after the latest-round block bodies. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-007 CHK-008">
T-003 reworded `resources/modules/phases/speccy-work.md` and
`resources/modules/skills/partials/review-fanout.md` to describe the
latest-round inline blocks plus the attributes-only prior-rounds index and
named the `speccy journal show <selector> --round N [--block <type>]`
drill-down affordance. `just reeject` was run and `git status --porcelain`
over `.claude/`, `.agents/`, and `.codex/` produced no output, confirming
ejected packs are in sync with source. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-009">
T-004 updated `docs/ARCHITECTURE.md`: the journal bullet now describes
`journal.blocks` as latest-round blocks with full bodies and
`journal.prior_rounds` as the attributes-only index of pre-latest blocks,
includes the absent-journal marker, and names the `journal show --round N`
drill-down. The size-invariant prose notes that prior rounds add only bounded
index entries. No passage remains claiming the bundle inlines all rounds.
Retry count: 0.
</coverage>

</report>
