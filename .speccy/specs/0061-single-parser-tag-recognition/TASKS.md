---
spec: SPEC-0061
spec_hash_at_generation: 3dc76d94d223c255b040b8f607b6750672836d4fb4d100498272e243248f0cab
generated_at: 2026-06-12T08:29:54Z
---
# Tasks: SPEC-0061 Single parser authority for tag recognition — gate-read and journal-write paths use the canonical parser, deleting both hand-rolled scanners

<task id="T-001" state="completed" covers="REQ-004">
## Route test VET.md through per-crate renderer-backed helpers

Add a renderer-backed VET.md builder to each test crate's support module so
every fixture matches the real grammar by construction. Both helpers compose the
already-`pub` production renderers `render_fresh_vet_frontmatter` and
`render_vet_section_heading` (re-exported from `speccy_core::parse`) plus a gate
block for an arbitrary `(verdict, tasks_hash)`, and accept an optional extra body
line (used by T-002's spoof fixture). Per DEC-004 these are two helpers, one per
crate — not one shared function — and **no** test-construction surface is added
to production `speccy-core`.

- `speccy-cli/tests/common/mod.rs`: add the helper and rewrite the existing
  `write_fresh_pass_vet_md` to delegate to it.
- `speccy-core/tests/next_priority.rs`: replace the private `write_vet_md`
  (which emits a frontmatter-less `## Invocation 1` string the parser rejects)
  with a builder over the same renderers.

Migrate the hand-rolled valid VET.md in `next_text.rs`, `next_json.rs`, and the
valid fixture in `journal_show.rs` onto the cli helper. Per DEC-005, `lint_vet.rs`
and the intentionally-open (gate-less) fixture in `journal_show.rs` are out of
scope: their hand-rolling drives deliberately invalid / structurally-special
grammar (missing frontmatter, `verdict="maybe"`, gate-ordering violations) that a
valid-only helper cannot produce — leave them and note the carve-out in the
journal.

This task lands while the gate-read path still uses the byte-scanner, so the
migrated fixtures must stay green: renderer output is a strict superset (it adds
valid frontmatter) the byte-scan still accepts.

<task-scenarios>
Given the cli and core VET.md helpers after this task,
when each renders a VET.md for an arbitrary `(verdict, tasks_hash)` pair,
then `parse_vet_in_flight` accepts the result and its terminal
`VetBlock::Gate` carries that verdict and `tasks_hash`.

Given the migrated test modules,
when they are audited for hand-rolled `## Invocation` / `<gate>` VET.md strings,
then none remain outside a renderer-backed helper except the DEC-005
grammar-edge fixtures (`lint_vet.rs`, the open `journal_show.rs` fixture).

Given the workspace after migration,
when `cargo test --workspace` runs,
then it is green.

Suggested files: `speccy-cli/tests/common/mod.rs`,
`speccy-cli/tests/next_text.rs`, `speccy-cli/tests/next_json.rs`,
`speccy-cli/tests/journal_show.rs`, `speccy-core/tests/next_priority.rs`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-005 REQ-003">
## Reroute gate-freshness onto the typed VET parser, with a gate-spoof regression test

Rewrite `vet_gate_is_fresh_pass` in `speccy-core/src/next.rs` to derive the
terminal gate's `verdict` and `tasks_hash` from `parse_vet_in_flight`'s typed
`VetDoc` — `doc.invocations.last()?.blocks.last()` matched as
`VetBlock::Gate { verdict, tasks_hash, .. }` — instead of the `last_gate_block`
byte-scan. A parse failure, an empty document, or a terminal block that is not a
`Gate` counts as not-fresh (`false`), matching the function's existing doc
contract. The hash comparison against the on-disk TASKS.md SHA-256
(`eq_ignore_ascii_case`) is unchanged. A `<gate>` quoted inside a block body is
captured in that block's body text, never surfaces as the terminal
`VetBlock::Gate`, so the spoof dies by construction.

Author the regression test first (in `speccy-core/tests/next_priority.rs`, using
the T-001 core helper): a valid VET.md whose terminal gate is `failed` and whose
body quotes an inline `<gate verdict="passed">` with a `tasks_hash` matching the
fixture's TASKS.md; all tasks completed; REPORT.md absent. Run it against the
pre-reroute implementation, confirm it resolves `Ship` (the live bug), and record
that observation in the per-task journal (CHK-009). Then apply the reroute so it
resolves `Vet`.

Because the reroute orphans the byte-scanner, delete `last_gate_block`,
`GateBlock`, and the `attribute_value` helper together with their `#[cfg(test)]
mod tests` cases (and the corresponding `use super::…` lines) — leaving them
would only re-introduce the divergence being removed (REQ-003, gate-read half).
Update the `vet_xml/mod.rs` module doc that claims `crate::next` keeps an
independent tolerant `<gate>` scanner; that scanner no longer exists.

<task-scenarios>
Given an all-completed spec whose VET.md has a `failed` terminal gate with an
inline `<gate verdict="passed">` (matching `tasks_hash`) in its body,
when `compute_for_spec` resolves the spec after the reroute,
then the action is `Vet`; and the recorded pre-fix run of the same test yields
`Ship`, proving the bug it guards.

Given the same binary and an all-completed spec with a fresh passing terminal
gate and REPORT.md absent,
when `compute_for_spec` resolves the spec,
then the action is `Ship`.

Given an all-completed spec whose VET.md is missing required frontmatter (and is
therefore unparseable),
when `compute_for_spec` resolves the spec,
then the action is `Vet`.

Given the post-task `speccy-core/src/next.rs`,
when it is searched for `last_gate_block`, `GateBlock`, `attribute_value`, or a
`cursor.find("<gate")` scan,
then none remain.

Suggested files: `speccy-core/src/next.rs`,
`speccy-core/tests/next_priority.rs`, `speccy-core/src/parse/vet_xml/mod.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-002 REQ-003">
## Round-trip the per-task journal before writing, and drop the redundant body guard

Add a `JournalError::ProducedJournalUnparseable { path, source }` variant
(mirroring `ProducedVetUnparseable`) and, in `append_under_lock`
(`speccy-cli/src/journal.rs`), re-parse the assembled would-be-new content with
strict `parse_journal_xml` immediately before the `fs_err::write`, mapping a
parse failure to the new variant and returning before any byte is written. This
mirrors the vet path's write-time round-trip (`append_vet_under_lock`); per
DEC-002 the two stay as separate inline call sites. On rejection the on-disk
journal is unchanged, or absent if it did not previously exist.

The round-trip is a complete superset of the open-tags-only body pre-scan, so
remove `first_nested_journal_element`, its call site in
`validate_and_render_block`, the `SerializeError::NestedJournalMarkup` variant,
the now-dead `nested_journal_markup_rejected` unit test, and the
`JOURNAL_ELEMENT_NAMES` import if it becomes unused
(`speccy-core/src/parse/journal_xml/serialize.rs`) — REQ-003, journal half.
Removing the pre-scan is required, not optional: while it remains it catches a
nested-markup body first and raises the old `NestedJournalMarkup`, so the
produced-unparseable condition would never surface. Update the
`journal_xml/serialize.rs` module doc that promised a nested-markup-free body
guard.

After this task, no hand-rolled tag-recognizer remains in non-test
`speccy-core` / `speccy-cli` source; audit the tree for `find("<`, `find("</`,
and `format!("<{` and confirm the only hits are the legitimate block *renderers*
(which emit tags, not recognize them — see SPEC Notes), recording the audit for
REPORT.md.

<task-scenarios>
Given a CLI workspace with a parseable per-task journal,
when `speccy journal append` is invoked with a block body containing a
line-isolated `</implementer>`,
then the command exits non-zero, stderr names the produced-unparseable
condition (`ProducedJournalUnparseable`, distinct from
`ExistingJournalUnparseable`), and the journal file is byte-identical to before.

Given a CLI workspace with no existing per-task journal,
when `speccy journal append` is invoked with a body mentioning `<review>`
inline in a prose sentence,
then the command exits 0 and the resulting journal parses under
`parse_journal_xml`.

Given the post-task `journal_xml/serialize.rs`,
when it is searched for `first_nested_journal_element` or
`SerializeError::NestedJournalMarkup`,
then neither remains and `validate_and_render_block` no longer pre-scans the
body.

Given the post-task non-test `speccy-core` / `speccy-cli` source,
when it is audited for `find("<`, `find("</`, `format!("<{` tag-scan patterns,
then the only matches are block renderers, and the audit is recorded for
REPORT.md.

Suggested files: `speccy-cli/src/journal.rs`,
`speccy-core/src/parse/journal_xml/serialize.rs`,
`speccy-cli/tests/journal_append.rs`
</task-scenarios>
</task>
