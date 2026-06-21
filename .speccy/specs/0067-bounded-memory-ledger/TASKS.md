---
spec: SPEC-0067
spec_hash_at_generation: f4a3ddac50a27face88582fe14a545410f0f214b1cab38e8350c6787bebf5046
generated_at: 2026-06-21T19:34:29Z
---
# Tasks: SPEC-0067 Bounded memory ledger — a higher capture bar, one-line entries, and autonomous compaction keep `.speccy/MEMORY.md` small and high-signal

<task id="T-001" state="pending" covers="REQ-002">
## Rewrite the memory-ledger entry-shape reference to the one-line shape

Replace the four-part entry shape in
`resources/modules/references/memory-ledger.md` with a single-line shape:
a trigger, a corrective rule, and a compact bracketed provenance tag, with
no convention/mistake or history field. Rewrite the worked example to one
line of that shape, and update the provenance bullet in the authoring
discipline to describe the bracketed `[SPEC-NNNN/T-NNN]` tag and the rule
that the task segment is dropped only for a spec-wide lesson. Keep the
existing abstract-wording discipline (prefer convention-level wording over
fragile code coordinates) and the provenance-resolvability requirement. Add
the one-line "what earns an entry" bar (durable across specs and not already
gate-enforced) so the reference and the ship retro agree. This is a
`references/` body, so the worked-instance carve-out ids (`SPEC-0042` /
`T-001`) are permitted in the example; use no other real Speccy ids. Run
`just reeject` so the ejected host trees regenerate against the edit.

<task-scenarios>
Given the rewritten `memory-ledger.md`,
when a reader reads the entry-shape section,
then it describes a single-line entry carrying a trigger, a corrective rule,
and a bracketed provenance tag, with no convention/mistake or history field,
and the worked example is one line of that shape using only the `SPEC-0042`
carve-out ids.

Given the edit followed by `just reeject`,
when the ejected ship-phase body is regenerated,
then it carries the one-line shape with no Conflict against the rendered
module and `cargo test --workspace`, `cargo clippy`, and `cargo +nightly fmt
--check` are all green.

Suggested files: `resources/modules/references/memory-ledger.md`
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-001 REQ-003 REQ-004">
## Rewrite the ship-time retro: capture bar, autonomous compaction, single human gate

Rewrite step 3 of `resources/modules/phases/speccy-ship.md`. Replace the
capture bullet's "a loop with recorded friction yields at least one
mistake-flavoured entry" mandate with the two-part bar: record an entry only
when the lesson is durable across specs and not already enforced by a gate,
reviewer persona, or `AGENTS.md`/rule; recording nothing is the default, and
delete the "no durable lesson this loop" sentinel. Split the
consolidate-and-dedupe bullet so that refuse-to-append and within-ledger
near-duplicate merge run autonomously with no human-approval step (safe
because they only refuse a write or shrink the file), while promotion into
the durable tier remains the sole human-gated mutation and removes the
promoted entry from the ledger on approval; state that boundedness no longer
depends on promotion firing. Reword any "four-part" phrasing in this body to
the one-line shape; leave the phantom-reference GC mechanism intact. Use only
placeholder ids (`SPEC-NNNN` / `T-NNN`) — this is a phase body, not a
reference. Run `just reeject`.

<task-scenarios>
Given the rewritten step 3,
when a reader reads the retro,
then capture states the durable-and-not-already-enforced bar with no
per-friction mandate and no sentinel line; refuse-to-append and
near-duplicate merge are described as autonomous; and promotion into the
durable tier is the only human-gated mutation.

Given the edit followed by `just reeject`,
when the ejected ship-phase body is regenerated,
then it carries the rewritten retro with no Conflict against the rendered
module and the full hygiene suite (`cargo test --workspace`, clippy, fmt) is
green.

Suggested files: `resources/modules/phases/speccy-ship.md`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-005">
## Align the read-side summary and confirm the no-duplicate-snippet invariant

Update `resources/modules/references/memory-ledger-summary.md` so its closing
description names the one-line entry shape rather than the four-part shape,
keeping the read instruction (load the slice whose trigger matches, act on
each corrective rule) intact so the read and write instructions agree. Do not
move, rename, or duplicate the `{% include
"modules/references/memory-ledger-summary.md" %}` directive. Use placeholder
ids only. Run `just reeject` and confirm the structural invariant holds.

<task-scenarios>
Given the rewritten summary,
when it is read alongside the entry-shape reference,
then both describe the same one-line shape with no residual four-part wording.

Given `just reeject` run after the edit,
when the include placement is checked,
then the memory-ledger-summary include appears exactly once in
`resources/modules/phases/speccy-work.md` and in no host wrapper, both tests
in `speccy-cli/tests/memory_feedforward.rs` pass, and the full hygiene suite
(`cargo test --workspace`, clippy, fmt) is green.

Suggested files: `resources/modules/references/memory-ledger-summary.md`
</task-scenarios>
</task>
