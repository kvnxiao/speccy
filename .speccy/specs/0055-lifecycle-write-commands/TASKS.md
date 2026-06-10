---
spec: SPEC-0055
spec_hash_at_generation: a11469f03bab48c58f7daa764a74f21ef2ddcafd5e744f17c11074cc650e30b9
generated_at: 2026-06-10T01:06:00Z
---
# Tasks: SPEC-0055 Mechanical lifecycle write commands — task state transitions, validated journal appends, and direct subagent journal writes

<task id="T-001" state="completed" covers="REQ-001 REQ-002">
## `speccy task transition` — byte-surgical state rewrite over the closed legal graph

Add a `Task { transition }` subcommand to the `Command` enum in
`speccy-cli/src/main.rs` (a `task` command group with a `transition`
subcommand taking a positional `<selector>` and a `--to <state>`
arg whose value parser accepts only `pending`, `in-progress`,
`in-review`, `completed` — an unknown `--to` is rejected at
argument-parse time). Resolve the selector with the existing
`task_lookup::parse_ref` / `task_lookup::find` seam (the same grammar
`speccy check` uses), surfacing the identical ambiguity and not-found
errors.

In `speccy-core`, add a byte-surgical rewriter that locates the
`<task>` open tag through the parsed `Task.span`
(`speccy-core/src/parse/task_xml/mod.rs:81`) and splices the new
`state` attribute value in place, preserving every other byte of
TASKS.md verbatim (frontmatter, bodies, whitespace, CRLF line
endings). The rewrite must **not** round-trip through
`task_xml::render` (`speccy-core/src/parse/task_xml/mod.rs:471`),
which reformats and strips nested blocks.

Enforce the closed six-edge legal graph plus same-state no-ops as a
core function over a `TaskState` enum: legal edges are
`pending→in-progress`, `in-progress→in-review`, `in-review→completed`,
`in-review→pending`, `in-progress→pending`, `completed→pending`. A
target equal to the current state is a no-op that exits 0 and leaves
the file byte-identical (DEC-003). Any other edge exits non-zero with
a diagnostic naming the current state, the requested state, and that
the edge is not legal. A selector resolving to no task exits non-zero
without modifying any file.

Add a `speccy-cli/src/commands/transition.rs` (or extend an existing
command module) that wires resolution → graph check → splice → write,
and dispatch it from `main.rs`. Add unit tests in core for the splice
(CRLF + multi-line body + unusual attribute spacing fixture; byte
comparison differs only in the state value) and the graph (all 16
ordered state pairs: exactly the six legal edges plus four same-state
no-ops succeed, the other six exit non-zero naming both states). Add
a CLI integration test for the not-found case leaving bytes unchanged.

<task-scenarios>
Given a unit-test fixture TASKS.md containing a task with a
multi-line body, unusual-but-legal attribute spacing, and CRLF line
endings,
when the transition function rewrites that task's state,
then a byte comparison of the result against the fixture differs only
in the state attribute value's bytes (CHK-001).

Given a built `speccy` binary and a scratch workspace,
when `speccy task transition SPEC-0042/T-099 --to completed` runs
against a spec with no T-099,
then the exit code is non-zero and TASKS.md's bytes are unchanged
(CHK-002).

Given fixture tasks in each of the four states,
when every ordered state pair is attempted (16 combinations),
then exactly the six legal edges plus the four same-state no-ops
succeed and the remaining six exit non-zero with both state names in
the diagnostic (CHK-003).

Suggested files: `speccy-cli/src/main.rs`,
`speccy-cli/src/commands/transition.rs`,
`speccy-core/src/parse/task_xml/mod.rs`, `speccy-core/src/lib.rs`,
`speccy-cli/tests/transition.rs`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-003 REQ-005">
## `speccy journal append` for per-task journals with CLI-stamped attributes and an advisory file lock

Add a `journal append <task-selector> --block
{implementer|review|blockers}` subcommand. The body is read from
stdin (DEC-007); the caller supplies only identity/judgment inputs
(`--model`, `--persona`, `--verdict`). The CLI is the sole authority
for `date` (UTC now, ISO8601 with seconds and timezone, via `jiff`)
and `round` (DEC-001): an `implementer` block opens a new round
(`max existing round + 1`, or `1` on a fresh file); `review` and
`blockers` blocks attach to the current round and are rejected when no
`implementer` block exists yet. There is no flag to override `date`
or `round`.

Add a journal serializer in `speccy-core` (the `journal_xml` module at
`speccy-core/src/parse/journal_xml/mod.rs` parses but has no writer)
that renders one `<implementer>` / `<review>` / `<blockers>` block and
stamps frontmatter (`spec`, `task`, `generated_at`) when creating a
fresh file. Validation runs **before** any write: required attributes
per block type, `--persona` against the persona registry
(`speccy-core/src/personas.rs`), `--verdict` against `{pass, blocking}`,
body non-empty, and no nested `<implementer>`/`<review>`/`<blockers>`
markup in the body. A validation failure exits non-zero and leaves the
journal byte-identical (or still absent). Every successful append must
leave a file that `journal_xml::parse` accepts, including its
round-sequence validation.

Introduce an advisory per-journal-file lock (REQ-005, DEC-007): add an
`fd-lock`/`fs2`-class crate to `[workspace.dependencies]` (must clear
`cargo deny check`), acquire the lock before reading file state for
round derivation and release it after the write, so
derive→validate→append is atomic against concurrent appenders.
Acquisition blocks until free with a 10-second timeout (DEC-002); on
timeout the command exits non-zero with a diagnostic naming the
journal path, with no partial bytes written. Lock usage is internal —
no caller flags. Add a concurrency test spawning ≥8 threads/processes
each appending one distinct `review` block, asserting the parser
accepts the result with no interleaving and all blocks present, plus a
test that two concurrent round-opening appends yield distinct ordered
round numbers, plus a timeout test: a deliberately held lock causes a
waiting append to exit non-zero after roughly the timeout interval,
naming the journal path, with the journal byte-identical (REQ-005
done-when).

<task-scenarios>
Given a scratch workspace with a pending task and no journal file,
when `speccy journal append SPEC-0042/T-001 --block implementer
--model test-model` runs with a body on stdin,
then the journal file exists, its frontmatter carries `spec`, `task`,
and a CLI-stamped `generated_at`, and the single `<implementer>` block
carries `round="1"` and an ISO8601 `date` (CHK-004).

Given the journal from CHK-004,
when a `review` append with `--persona not-a-persona` runs,
then the exit code is non-zero and the journal bytes are unchanged
(CHK-005).

Given a test that spawns 8 threads or processes each appending one
distinct `review` block to the same journal,
when all appenders finish,
then the journal contains exactly 8 review blocks with no interleaved
or truncated markup and the parser accepts the file (CHK-008).

Given a process holding the journal lock for longer than the timeout,
when an append runs,
then it exits non-zero after roughly the timeout interval, the
diagnostic names the journal path, and the journal is byte-identical.

Suggested files: `speccy-cli/src/main.rs`,
`speccy-cli/src/commands/journal.rs`,
`speccy-core/src/parse/journal_xml/mod.rs`,
`speccy-core/src/personas.rs`, `Cargo.toml`, `deny.toml`,
`speccy-cli/tests/journal_append.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-004">
## Freeze the VET.md grammar into a `speccy-core` parser

Add a `vet_xml` parser module in `speccy-core/src/parse/` (sibling to
`journal_xml`) covering the full grammar that today lives only in
skill prose and the tolerant `<gate>` scanner in
`speccy-core/src/next.rs`: frontmatter (`spec`, `generated_at`), one
`## Invocation N — <ISO8601>` section per vet invocation, and the five
block types with their attribute schemas —
`<drift-review verdict="pass|blocking" round date model>`,
`<holistic-fix verdict="addressed|blocking|stuck" round date model>`,
`<simplifier-scan verdict="clean|candidates">`,
`<simplifier-apply verdict="applied|blocking">`, and the terminal
`<gate verdict="passed|failed" tasks_hash date>` (exactly one per
invocation section, and the last block in its section). The parser
rejects unknown blocks, unknown attributes, out-of-domain verdict
values, a non-terminal `gate`, and a second `gate` in one section. The
tolerant `<gate>` scanner in `next.rs` keeps its read path, but the
grammar's source of truth moves to this parser (DEC-005).

This task adds only the parser and its types/round-sequence
validation; the `journal append` routing to VET.md lands in T-004.
Drive acceptance with fixtures of the VET.md shape the current vet
skill produces. Expose a public `vet_xml::parse` and wire the module
into `speccy-core/src/parse/mod.rs` and `lib.rs`.

<task-scenarios>
Given a fixture VET.md the current vet skill would produce (frontmatter,
one invocation section, five block types, terminal gate),
when `vet_xml::parse` runs,
then it returns the parsed document with each block's attributes and
the invocation section structure intact.

Given fixtures that each violate one rule — an unknown block, an
unknown attribute, a `verdict="maybe"` drift-review, a non-terminal
gate, and two gates in one section,
when `vet_xml::parse` runs on each,
then each is rejected with a diagnostic identifying the violation.

Suggested files: `speccy-core/src/parse/vet_xml/mod.rs`,
`speccy-core/src/parse/mod.rs`, `speccy-core/src/lib.rs`,
`speccy-core/tests/` (or in-module `#[cfg(test)]` fixtures)
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-004">
## Route `journal append` vet block types to VET.md with mechanical invocation/round/tasks_hash management

Extend `journal append` to accept the vet block types
`{drift-review|holistic-fix|simplifier-scan|simplifier-apply|gate}`
against a bare `<SPEC-NNNN>` selector, routing to
`<spec-dir>/journal/VET.md`. Target inference follows DEC-004: vet
block types require a bare spec selector and task block types require a
task selector; a mismatched pairing is an argument error (no `--vet`
flag). Add a VET serializer alongside the `vet_xml` parser from T-003.

The CLI owns all environment-derivable values: `date` on every block;
`round` (a `drift-review` opens a round, a `holistic-fix` attaches to
the current round); `tasks_hash` on `gate` as the lowercase hex
SHA-256 of the sibling TASKS.md read at append time (matching the
freshness check in `speccy-core/src/next.rs`); and invocation
sectioning — when the file is absent or the last section is
gate-terminated, the append opens `## Invocation N+1` with a
CLI-stamped datetime before writing the block. A non-gate block
appended after a section's gate never lands in the closed section; it
opens the next section. A `holistic-fix` (or any non-opening block)
with no preceding `drift-review` in the open section exits non-zero
with VET.md untouched. The per-file lock and validation-before-write
discipline from T-002 apply to the VET journal too. Confirm a produced
gate block is accepted by `speccy next`'s freshness check.

Per DEC-008, derive invocation/round state from the parser, not a
parallel text scan: add an in-flight parse mode to the `vet_xml`
parser (relaxing only the last section's terminal-gate rule so the
open mid-vet-run section parses, the block grammar otherwise frozen),
derive the append plan from that typed parse of the existing file
(as the per-task path derives `round` from `parse_journal_xml`), and
re-parse the would-be-new file through the parser before any write so
the parser is the single authority for both derivation and body
inertness — no hand-rolled tolerant scan or body-markup guard.

<task-scenarios>
Given a scratch spec with all tasks completed and no VET.md,
when a `drift-review` append, a `holistic-fix` append, and a
`gate --verdict passed` append run in order,
then VET.md parses under the new parser, holds one invocation section
ending in the gate, and `speccy next` resolves the spec past the vet
step (CHK-006).

Given the VET.md from CHK-006,
when a `simplifier-scan` append runs,
then the block lands under a freshly opened `## Invocation 2` section
rather than the gate-terminated first section (CHK-007).

Given a TASKS.md edit between two gate appends,
when each gate is appended,
then the two `tasks_hash` values differ and each equals the file hash
at its own append time.

Suggested files: `speccy-cli/src/commands/journal.rs`,
`speccy-core/src/parse/vet_xml/mod.rs`, `speccy-core/src/next.rs`,
`speccy-cli/tests/journal_append_vet.rs`
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-006">
## `speccy journal show` — schema-versioned filtered JSON

Add a `journal show <selector> --json` subcommand that parses the
resolved journal (task selector → `journal/T-NNN.md` via
`journal_xml`; bare spec selector → VET.md via `vet_xml`, per DEC-004)
and emits a schema-versioned JSON envelope with `schema_version` as the
first field pinned to `1`, carrying frontmatter and blocks with their
attributes and bodies (and, for VET.md, invocation sections). Mirror
the existing JSON-envelope conventions used by `status`/`next`
output modules (`speccy-cli/src/next_output.rs`,
`speccy-cli/src/status_output.rs`).

Implement the three minimal filters derived from the orchestrator's
call sites: `--round latest|N`, `--verdict <value>`, `--block <type>`,
composing conjunctively. `--round latest` returns only the highest
round's blocks plus the round number in the envelope. A missing
journal file exits non-zero with a diagnostic (the known call sites
run only after blocks exist, so absence is a loud anomaly). Add tests
for the filter composition and the missing-file exit.

Two semantics to pin down: (1) without `--json` the command renders
the same filtered content in a human-readable text form — `--json`
toggles representation, never content, matching the workspace-wide
convention; (2) for VET.md, rounds reset per invocation section, so
`--round latest|N` applies within the **last** invocation section
(the open or most recent invocation), which is the slice the vet
flow's call sites need.

<task-scenarios>
Given a fixture journal with two rounds and five reviews in round 2
(one blocking),
when `speccy journal show SPEC-0042/T-001 --json --round latest
--verdict blocking` runs,
then stdout parses as JSON whose block list has length 1, persona and
verdict matching the fixture, and `schema_version` equal to 1
(CHK-009).

Given a `--block review --round N` invocation on a task journal,
when it runs,
then the JSON lists the personas that reviewed round N (the
completeness call site).

Given a spec selector,
when `journal show` runs,
then VET.md is parsed and its invocation sections and blocks appear in
the JSON.

Suggested files: `speccy-cli/src/main.rs`,
`speccy-cli/src/commands/journal.rs`,
`speccy-cli/src/commands/journal_show_output.rs`,
`speccy-cli/tests/journal_show.rs`
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-007">
## `speccy verify` gains the `VET-*` lint family

Add a `VET-*` lint family in `speccy-core/src/lint/rules/vet.rs`
(mirroring the JNL family's posture in
`speccy-core/src/lint/rules/jnl.rs`) and register the codes in
`speccy-core/src/lint/registry.rs` (both errors) and the rules module
list in `speccy-core/src/lint/rules/mod.rs`. The lints run only when
`<spec-dir>/journal/VET.md` exists: `VET-001` fires when the file
fails the frozen `vet_xml` grammar (frontmatter, block shapes,
attribute domains, round sequencing); `VET-002` fires when an
invocation section other than the last lacks a terminal `gate`, or any
block follows a `gate` within its section. A spec without a VET.md
emits no `VET-*` diagnostics — absence is the resolver's concern, not
lint's. Confirm the JSON envelope counts `VET-*` as errors and the
exit code reflects them.

<task-scenarios>
Given a fixture VET.md with a `verdict="maybe"` drift-review block,
when `speccy verify --json` runs,
then the envelope lists a VET-001 error naming the file and the
process exits non-zero (CHK-010).

Given a VET.md whose first invocation section has no `gate` while a
second section exists,
when `speccy verify` runs,
then VET-002 is reported and the exit code is non-zero.

Given a VET.md produced solely by `journal append`,
when `speccy verify` runs,
then no `VET-*` diagnostic appears; and given no VET.md anywhere, no
`VET-*` code appears either.

Suggested files: `speccy-core/src/lint/rules/vet.rs`,
`speccy-core/src/lint/rules/mod.rs`,
`speccy-core/src/lint/registry.rs`,
`speccy-core/src/lint/rules/vet.rs` tests
</task-scenarios>
</task>

<task id="T-007" state="pending" covers="REQ-008">
## Loop subagents append their own blocks and return thin verdicts

Update the skill-pack source modules under `resources/` so each block
author writes its own block via `speccy journal append` and returns a
thin verdict (persona/role, verdict, one-line rationale) instead of a
full block body (DEC-006): (a) the `speccy-work` implementer phase
(`resources/modules/phases/speccy-work.md`) appends its
`<implementer>` block; (b) the six reviewer personas
(`resources/modules/personas/reviewer-*.md`) append their `<review>`
blocks; (c) the vet personas (`resources/modules/personas/vet-*.md`)
append `<drift-review>`, `<holistic-fix>`,
`<simplifier-scan>`/`<simplifier-apply>` respectively. Update the
shared verdict-return partial
(`resources/modules/personas/verdict_return_contract.md`) once so every
wrapper inherits it, and define the thin-verdict **format** there once
— a single parseable shape carrying persona/role, verdict, and the
one-line rationale (e.g. one `<verdict>` element with attributes) — so
the orchestrator parses every persona's return uniformly instead of
six personas inventing six shapes. Update the journal reference
templates whose worked examples currently teach agents to author
`date`/`round` attributes —
`resources/modules/references/journal-implementer.md`,
`journal-review.md`, and `journal-blockers.md` — so they document the
`journal append` invocation and mark `date`/`round` as CLI-stamped;
left unedited they contradict REQ-008's done-when. Persona/phase
bodies stop instructing agents to compute `date`, `round`,
`tasks_hash`, or invocation numbers. Then run
`just reeject` so both host ejections (`.claude/`, `.agents/`,
`.codex/`) regenerate with the new contract — including the
prose-enforced Codex pack. Per AGENTS.md, edit only under `resources/`
and never the ejected files directly.

<task-scenarios>
Given the regenerated Claude Code pack,
when a reviewer persona file is read,
then it directs the persona to run `speccy journal append ... --block
review --persona <self> --verdict <v>` with findings on stdin and to
end with a thin verdict line, and instructs no date/round/tasks_hash
computation.

Given the regenerated packs,
when the implementer phase body is read,
then the handoff lands via `journal append` rather than inside the
final return message.

Given a clean checkout after this task,
when `just reeject` runs and `git status --porcelain` is checked,
then the working tree is clean — proving the committed ejected packs
match the updated sources (CHK-011).

Suggested files: `resources/modules/phases/speccy-work.md`,
`resources/modules/personas/reviewer-*.md`,
`resources/modules/personas/vet-*.md`,
`resources/modules/personas/verdict_return_contract.md`,
`resources/modules/references/journal-implementer.md`,
`resources/modules/references/journal-review.md`,
`resources/modules/references/journal-blockers.md`,
`.claude/` `.agents/` `.codex/` (regenerated via `just reeject`)
</task-scenarios>
</task>

<task id="T-008" state="pending" covers="REQ-009">
## Orchestrator and reconcile flows consume transition + show + append

Update the orchestrate, review, vet, and amend/reconcile module
sources so they stop hand-editing lifecycle files: every TASKS.md
state mutation references `speccy task transition` (including the
reconcile policy's flips in
`resources/modules/references/reconcile-policy.md` and the amendment
`completed → pending` invalidation); the persona-completeness check
before leaving `in-review` and the blocking-review read-back for
`<blockers>` consolidation go through `speccy journal show`
(`--block review --round latest` for completeness, `--verdict blocking`
for read-back); the consolidated `<blockers>` block — whose body stays
orchestrator-authored semantic judgment (DEC-001 non-goal) — lands via
`speccy journal append --block blockers`. The vet flow's terminal
`<gate>` block — written by **every** vet exit path, including the
Phase 0 early exits — lands via `speccy journal append --block gate`
(the CLI computes `tasks_hash` and invocation sectioning, so the vet
prose drops its hand-bootstrap of VET.md frontmatter and invocation
headings entirely). The standalone
`/speccy-work` primitive flips its own states through the transition
command. Restate the single-writer rule: the CLI's append lock owns
write serialization; the orchestrator remains the sole author of
`<blockers>` bodies and of git commits. Run `just reeject`; edit only
under `resources/`. The serial-transcription prose to remove lives
partly in shared partials, not only the skill bodies: the review
fan-out partial carries the "orchestrator appends each returned
`<review>` block serially" contract, the vet-phases partial carries
"append the held `<drift-review>` block to VET.md", and the
retry-shape reference instructs raw journal reads that should now go
through `journal show`.

<task-scenarios>
Given the regenerated orchestrate skill,
when its review consolidation section is read,
then it directs `journal show --block review --round latest` for
completeness, `--verdict blocking` for read-back, `journal append
--block blockers` for the directive, and `task transition` for the
state flip.

Given the regenerated reconcile policy reference,
when its auto-fix table is read,
then state-flip remedies name the transition command instead of
editing TASKS.md.

Given the regenerated module sources,
when `grep -r` over `resources/modules/` searches for instructions to
hand-edit task `state` attributes or journal files,
then no lifecycle-mutation instruction bypasses the CLI verbs
(CHK-012).

Suggested files: `resources/modules/skills/speccy-orchestrate.md`,
`resources/modules/skills/speccy-review.md`,
`resources/modules/skills/speccy-vet.md`,
`resources/modules/skills/speccy-amend.md`,
`resources/modules/skills/speccy-work.md`,
`resources/modules/skills/partials/review-fanout.md`,
`resources/modules/skills/partials/vet-phases.md`,
`resources/modules/references/reconcile-policy.md`,
`resources/modules/references/retry-shape.md`,
`.claude/` `.agents/` `.codex/` (regenerated via `just reeject`)
</task-scenarios>
</task>

<task id="T-009" state="pending" covers="REQ-010">
## ARCHITECTURE.md matches the shipped behavior

Update `docs/ARCHITECTURE.md` in the same change set so it reflects the
post-SPEC-0055 behavior across five sections: (1) the CLI surface
section documents `task transition`, `journal append`, and `journal
show` with their JSON envelopes; (2) the TASKS.md state model's "Who
sets it" column describes the CLI-mediated writes; (3) the journal
sections describe the CLI-mediated writes and replace the vet "skill
body is the only writer" sentence with the CLI-append contract; (4) the
concurrency contract replaces the prose-level "sole serial writer" rule
with the CLI-serialized append contract; (5) the "What We Deliberately
Don't Do" claim-files/leases row is narrowed to task claiming with a
pointer to this SPEC's append-serialization decision, and the
lint-code catalogue gains the `VET-*` family. No section may still
assert that sub-agents return blocks for the orchestrator to transcribe
as the only write path, nor that the exclusions forbid append
serialization.

<task-scenarios>
Given the updated ARCHITECTURE.md,
when the vet journal section is read,
then the "skill body is the only writer" sentence is replaced by the
CLI-append contract.

Given the updated ARCHITECTURE.md,
when the reviewer reads the CLI surface table, the state model, the
journal grammar sections, the exclusions row, and the lint catalogue,
then each describes the post-SPEC-0055 behavior with no stale
sole-writer or no-locking claims (CHK-013).

Suggested files: `docs/ARCHITECTURE.md`
</task-scenarios>
</task>
