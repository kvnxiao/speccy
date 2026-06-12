---
spec: SPEC-0055
outcome: implemented
generated_at: 2026-06-10T20:00:00Z
---

# REPORT: SPEC-0055 Mechanical lifecycle write commands — task state transitions, validated journal appends, and direct subagent journal writes

<report spec="SPEC-0055">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 added `speccy task transition <selector> --to <state>` via a new
`TaskCommand::Transition` arm in `speccy-cli/src/main.rs`, backed by a
byte-surgical `splice_task_state` function in
`speccy-core/src/parse/task_xml/mod.rs` that locates the `<task>` open
tag through the parsed `Task.span` and splices the new attribute value
in place without round-tripping through `task_xml::render`. Both
qualified (`SPEC-NNNN/T-NNN`) and unqualified (`T-NNN`) selectors
resolve via the existing `task_lookup::parse_ref` / `task_lookup::find`
seam shared with `speccy check`, surfacing identical ambiguity and
not-found errors. A selector resolving to no task exits non-zero and
leaves TASKS.md unmodified. CHK-001 covers multi-line body, unusual
attribute spacing, and CRLF line endings with a byte comparison that
differs only in the state value. CHK-002 confirms non-zero exit and
byte-identical file on a not-found selector. Retry count: 3.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
The closed six-edge legal graph (pending->in-progress,
in-progress->in-review, in-review->completed, in-review->pending,
in-progress->pending, completed->pending) is enforced as a core function
over a `TaskState` enum in `speccy-core/src/parse/task_xml/transition.rs`.
A target equal to the current state is a no-op that exits 0 and leaves
the file byte-identical (DEC-003). Any other edge exits non-zero with a
diagnostic naming both states and the illegal edge. Unknown `--to` values
are rejected at argument-parse time. CHK-003 tests all 16 ordered state
pairs: exactly six legal edges plus four same-state no-ops succeed, the
remaining six exit non-zero with both state names in the diagnostic.
Retry count: covered by T-001 round count (3).
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004 CHK-005">
T-002 added `speccy journal append <task-selector> --block
{implementer|review|blockers}` reading the block body from stdin. The CLI
stamps `date` (UTC now, ISO8601 with seconds and timezone via `jiff`) and
derives `round` from file state: an `implementer` block opens a new round
(max existing round + 1, or 1 on a fresh file); `review` and `blockers`
blocks attach to the current round and are rejected when no `implementer`
block exists yet. Validation runs before any write: required attributes per
block type, `--persona` against the persona registry, `--verdict` against
{pass, blocking}, body non-empty, no nested journal elements in the body.
A fresh journal is created with CLI-stamped frontmatter (spec, task,
generated_at). Every successful append leaves a file that
`journal_xml::parse` accepts. CHK-004 confirms fresh-file creation with
correct frontmatter and round="1" with CLI-stamped date. CHK-005 confirms
non-zero exit and byte-identical journal on an invalid --persona.
Retry count: 1.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006 CHK-007">
T-003 added a `vet_xml` parser module in `speccy-core/src/parse/vet_xml/`
covering the full VET.md grammar: frontmatter (spec, generated_at), one
`## Invocation N -- ISO8601` section per vet invocation, and all five
block types with their attribute schemas. The parser rejects unknown
blocks, unknown attributes, out-of-domain verdict values, a non-terminal
gate, and a second gate in one section. T-004 extended `journal append` to
accept vet block types against a bare SPEC-NNNN selector routing to
journal/VET.md (DEC-004). Per DEC-008, invocation/round state is derived
from a new `parse_in_flight` mode (relaxing only the last section's
terminal-gate rule), and the would-be-new file is re-parsed before any
write -- the parser is the single authority for both derivation and body
inertness. The CLI computes `tasks_hash` on gate as the SHA-256 of the
sibling TASKS.md. CHK-006 confirms a full drift-review/holistic-fix/gate
sequence produces a parseable VET.md and `speccy next` resolves past the
vet step. CHK-007 confirms a post-gate append opens ## Invocation 2.
T-003 retry: 1; T-004 retry: 13.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008">
T-002 introduced an advisory per-journal-file lock using `fs4` (added to
[workspace.dependencies], cleared `cargo deny check`), acquired before
reading file state for round derivation and released after the write, so
the derive->validate->append sequence is atomic with respect to concurrent
appenders. Acquisition blocks with a 10-second timeout (DEC-002); on
timeout the command exits non-zero naming the journal path with no partial
bytes written (DEC-007). No caller flags expose locking. CHK-008 spawns 8
threads each appending a distinct review block, confirming the parser
accepts the result with no interleaving and all 8 blocks present. Two
concurrent round-opening appends yield distinct ordered round numbers.
Retry count: covered by T-002 round count (1).
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-009">
T-005 added `speccy journal show <selector> --json` emitting a
schema-versioned JSON envelope (schema_version as first field, pinned 1)
carrying frontmatter and blocks with their attributes and bodies. For
VET.md, --round latest|N applies within the last invocation section. Three
conjunctive filters: --round latest|N, --verdict value, --block type. A
missing journal exits non-zero. The --json flag toggles representation
only, not content. CHK-009 confirms --round latest --verdict blocking over
a two-round five-review fixture returns exactly one block with correct
persona/verdict and schema_version: 1. Retry count: 1.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-010">
T-006 added a VET-* lint family in `speccy-core/src/lint/rules/vet.rs`
mirroring the JNL family's posture. VET-001 fires when VET.md fails the
frozen vet_xml grammar (frontmatter, block shapes, attribute domains,
round sequencing). VET-002 fires when a non-last invocation section lacks
a terminal gate, or any block follows a gate within its section. Both
codes are registered as errors. A spec without VET.md emits no VET-*
diagnostics. CHK-010 confirms a verdict="maybe" drift-review fixture
produces a VET-001 error in the JSON envelope with non-zero exit. A
VET.md produced solely by journal append passes with no VET-* codes; a
workspace with no VET.md also produces none. Retry count: 0.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-011">
T-007 updated all skill-pack source modules under resources/ so each block
author writes its own block via `speccy journal append` and returns a thin
`<verdict>` element (persona/role, verdict, one-line rationale). The
shared `verdict_return_contract.md` partial was updated once so every
wrapper inherits the thin-verdict format; persona and phase bodies no
longer instruct agents to compute date, round, tasks_hash, or invocation
numbers. The journal reference templates (journal-implementer.md,
journal-review.md, journal-blockers.md) document the journal append
invocation and mark CLI-stamped attributes as such. `just reeject`
regenerated all host ejections (.claude/, .agents/, .codex/), including
the prose-enforced Codex pack. CHK-011 confirms `just reeject` yields an
empty `git status --porcelain` output. Retry count: 1.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-012">
T-008 updated the orchestrate, review, vet, amend, and reconcile module
sources under resources/modules/ so every TASKS.md state mutation
references `speccy task transition`, persona-completeness checks and
blocker read-back go through `speccy journal show`, and blockers blocks
land via `speccy journal append`. The review-fanout.md partial's
"orchestrator appends each returned review block serially" prose was
replaced with the subagent-self-append contract; the vet-phases.md
partial's hand-bootstrap of VET.md frontmatter and invocation headings was
removed (the CLI owns those). The reconcile-policy auto-fix rows name
`task transition` instead of TASKS.md edits. The retry-shape reference
routes reads through `journal show`. The documented single-writer rule was
restated: the CLI's append lock owns write serialization; the orchestrator
remains the sole author of blockers bodies and git commits. CHK-012
(content-level check by reviewer) confirmed no lifecycle-mutation
instruction bypasses the CLI verbs. Retry count: 1.
</coverage>

<coverage req="REQ-010" result="satisfied" scenarios="CHK-013">
T-009 updated docs/ARCHITECTURE.md across the five contracted sections:
(1) the CLI surface table gains task transition, journal append, and
journal show; (2) the TASKS.md state model "Who sets it" column describes
CLI-mediated writes; (3) the journal sections replace the vet "skill body
is the only writer" sentence with the CLI-append contract; (4) the
concurrency contract replaces the "sole serial writer" rule with the
CLI-serialized append contract; (5) the claim-files/leases exclusion is
narrowed to task claiming with a pointer to this SPEC's
append-serialization decision, and the lint catalogue gains VET-001 and
VET-002. CHK-013 (content check by reviewer) confirmed all five sections
describe post-SPEC-0055 behavior with no stale sole-writer or no-locking
claims. Retry count: 1.
</coverage>

</report>

## Notes

T-004 (route journal append vet block types to VET.md) was the hardest
slice, requiring 13 implementation rounds. The core difficulty was the
VET.md parser's dual-mode design (DEC-008): deriving invocation/round
state from a typed parse_in_flight parse rather than a parallel tolerant
text scan, then re-validating the would-be-new file before any write,
ensured the parser is the single authority -- but the boundary between
strict and in-flight modes, combined with the append-path's lock +
derive + validate + write sequence, required several rounds of
reviewer-driven clarification before the implementation stabilized. The
retry count for T-004 (13) dwarfs the other nine tasks combined (12
total) and reflects that complexity concentration, not a broader
implementation quality issue.

The simplifier pass (VET.md vet invocation 1) extracted a
`report_lookup_error` helper covering the three new commands' identical
LookupError rendering blocks, and promoted `bare_spec_selector_regex`
from journal.rs to pub(crate) for reuse in journal_show.rs, eliminating
two instances of verbatim duplication. Both applied with all four hygiene
gates green.
