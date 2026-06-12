---
id: SPEC-0055
slug: lifecycle-write-commands
title: Mechanical lifecycle write commands — task state transitions, validated journal appends, and direct subagent journal writes
status: implemented
created: 2026-06-09
supersedes: []
archived_at: 2026-06-12
---

# SPEC-0055: Mechanical lifecycle write commands — task state transitions, validated journal appends, and direct subagent journal writes

## Summary

Every TASKS.md state flip and every journal block written during the
orchestration loop is currently produced by an LLM hand-editing
markdown. The orchestrator relays each subagent's verdict block
through its own context twice (once inbound as a return message, once
outbound as a file edit), pays a full-file read before every edit
(the host harness requires Read-before-Edit), and defers all shape
validation to `speccy verify` long after the bad write landed. This
is bookkeeping expressed in prompts — the inverse of the
"deterministic core, intelligent edges" principle.

This SPEC moves the bookkeeping into the CLI. Three new command
surfaces:

- `speccy task transition SPEC-NNNN/T-NNN --to <state>` — rewrites
  the `state` attribute of one `<task>` element byte-surgically,
  enforcing the legal state graph.
- `speccy journal append` — appends one validated block per
  invocation to a per-task journal (`journal/T-NNN.md`) or the
  per-SPEC vet journal (`journal/VET.md`), with the CLI as the
  authority for every environment-derivable attribute (`date`,
  `round`, `generated_at`, `tasks_hash`, invocation numbering) and
  an internal file lock serializing concurrent appends. The VET.md
  block grammar, today defined only in skill prose plus a tolerant
  `<gate>` scanner in `speccy-core/src/next.rs`, is frozen into a
  real parser in `speccy-core`.
- `speccy journal show` — emits journal content as schema-versioned
  JSON with minimal filters derived from the orchestrator's known
  call sites.

On top of the commands, the shipped skill packs change contract:
loop subagents (the implementer phase, reviewer personas, vet
personas) append their own journal blocks via the CLI and return
thin verdicts (persona + verdict + one-line rationale) instead of
full block bodies. The orchestrator stops hand-editing lifecycle
files entirely: state flips go through `task transition`,
completeness checks and blocker read-back go through
`journal show`, and its own consolidated `<blockers>` block goes
through `journal append`.

Grounding (traced in-session): `speccy-core` already carries the
read-side building blocks — span-based `<task>` extraction
(`Task.span`, `TasksDoc.raw`, `task_lookup::extract_entry_from_raw`),
selector resolution (`task_lookup::parse_ref` / `find`), and a
parse-only `journal_xml` module with round-sequence validation. All
mutations are net-new: `task_xml::render` is a canonical
re-projection (reformats, strips nested blocks) and must not be used
for the surgical state rewrite; `journal_xml` has no serializer;
the codebase has zero locking or atomic-write precedent.

## Goals

<goals>
- No agent hand-edits TASKS.md or any `journal/*.md` file during the
  loop; every lifecycle mutation flows through a CLI verb.
- A subagent's verdict block body traverses the orchestrator context
  zero times: it is written once, at the source, by the subagent
  that produced it.
- A malformed journal block is rejected at write time with a
  diagnostic, leaving the file untouched, instead of landing on disk
  and surfacing later in `speccy verify`.
- Five parallel reviewer personas appending to the same journal file
  produce five well-formed blocks with no interleaving or lost
  writes.
- A hand-edited or grammar-violating `journal/VET.md` fails
  `speccy verify` in CI.
</goals>

## Non-goals

<non-goals>
- No scoped read-side context bundle (`speccy context`). That is the
  agreed follow-up SPEC; this SPEC is the write side only.
- No task claiming, leases, or distributed locks. The
  "Claim files / leases" exclusion stands for claiming; this SPEC
  revisits only same-host append serialization.
- No change to git commit ownership or boundaries. The orchestrator
  still owns commits; direct subagent journal writes land in the
  working tree between commits exactly as orchestrator-written
  blocks do today.
- No general markdown patch verb. Each command does one job against
  one grammar; arbitrary mutation surfaces are out.
- No CLI-authored prose. The CLI stamps attributes and validates
  shape; block bodies are always caller-supplied. In particular the
  consolidated `<blockers>` body remains orchestrator-authored
  semantic judgment.
- No backward-compatibility shim for skill packs running against an
  older `speccy` binary; packs and binary version together and
  `speccy init --force` refreshes both.
</non-goals>

## User Stories

<user-stories>
- As an orchestrator skill driving the work-and-review loop, I want
  state flips and journal writes to be CLI verbs, so that I never
  hand-edit markdown, never re-read a file just to append to it,
  and never relay verdict bodies through my own context.
- As a reviewer persona subagent, I want to append my own
  `<review>` block and return a one-line verdict, so that my full
  findings are written once instead of passing through the
  orchestrator twice.
- As a solo developer running `speccy verify` in CI, I want a
  hand-edited or malformed `journal/VET.md` to fail the gate loudly,
  so that the ship-gate artifact `speccy next` trusts cannot rot
  silently.
</user-stories>

## Assumptions

<assumptions>
- Skill packs and the `speccy` binary version together in a repo;
  `speccy init --force` refreshes the packs, so no compatibility
  shim for older binaries is needed.
- All journal writers are local processes on one host writing to a
  local filesystem; advisory same-host locking is sufficient and
  NFS/distributed writers are out of scope.
- Codex-side write discipline remains prose-enforced through the
  persona bodies, matching the existing read-only-posture parity
  stance; no per-agent mechanical tool gating is assumed on Codex.
- Freezing the VET.md grammar into speccy-core now does not
  conflict with near-term vet-skill iteration; post-SPEC grammar
  changes go through a core PR (DEC-005).
- The consolidated `<blockers>` body remains semantic judgment and
  stays orchestrator-authored; the CLI only transports it.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: `speccy task transition` rewrites exactly one task's `state` attribute

`speccy task transition <selector> --to <state>` resolves the task
via the same selector grammar as `speccy check`
(`task_lookup::parse_ref` accepting `T-NNN` and `SPEC-NNNN/T-NNN`,
then `task_lookup::find`), locates the `<task>` open tag through the
parsed byte span, and splices the new `state` value in place. Every
byte of TASKS.md outside the rewritten attribute value is preserved
verbatim — frontmatter, task bodies, whitespace, and line endings
are untouched. The rewrite must not round-trip through
`task_xml::render`.

<done-when>
- After a transition, re-reading TASKS.md shows the selected task's
  `state` attribute changed and the remainder of the file
  byte-identical to the pre-transition content.
- Both qualified (`SPEC-0042/T-003`) and unqualified (`T-003`)
  selectors resolve, with the same ambiguity and not-found errors
  `speccy check` produces.
- A selector that resolves to no task exits non-zero without
  modifying any file.
</done-when>

<behavior>
- Given a TASKS.md with three tasks, when
  `speccy task transition SPEC-0055/T-002 --to in-progress` runs,
  then T-002's open tag carries `state="in-progress"` and T-001 and
  T-003 are byte-identical to before.
- Given a workspace where `T-009` matches no spec, when a transition
  targets it, then the process exits non-zero, prints a not-found
  diagnostic, and TASKS.md is unmodified.
</behavior>

<scenario id="CHK-001">
Given a unit-test fixture TASKS.md containing a task with a
multi-line body, an unusual-but-legal attribute spacing, and CRLF
line endings,
when the transition function rewrites that task's state,
then a byte comparison of the result against the fixture differs
only in the state attribute value's bytes.
</scenario>

<scenario id="CHK-002">
Given a built `speccy` binary and a scratch workspace,
when `speccy task transition SPEC-0042/T-099 --to completed` runs
against a spec with no T-099,
then the exit code is non-zero and TASKS.md's bytes are unchanged.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Transition command enforces the legal task state graph

The command accepts only edges in the closed legal graph derived
from the documented state model plus the reconcile and amendment
policies:

- `pending → in-progress` (implementer claims)
- `in-progress → in-review` (implementer hands off)
- `in-review → completed` (all persona reviews pass)
- `in-review → pending` (a reviewer blocks)
- `in-progress → pending` (reconcile: orphaned in-progress reset)
- `completed → pending` (amendment invalidates a completed task)

A transition whose target equals the current state is a no-op that
exits 0 (reconcile idempotency, DEC-003). Any other edge exits
non-zero with a diagnostic naming the current state, the requested
state, and the fact that the edge is not in the legal graph.

<done-when>
- All six legal edges succeed against fixture tasks in the matching
  source state.
- Same-state transitions exit 0 and leave the file byte-identical.
- Every edge outside the legal set (e.g. `pending → completed`,
  `completed → in-progress`) exits non-zero, names both states in
  the diagnostic, and leaves the file unmodified.
- An unknown `--to` value (not one of the four states) is rejected
  at argument-parse time.
</done-when>

<behavior>
- Given a task at `state="pending"`, when
  `speccy task transition ... --to completed` runs, then the exit
  code is non-zero and the diagnostic names `pending`, `completed`,
  and the illegal edge.
- Given a task at `state="completed"`, when a transition to
  `completed` runs, then the process exits 0 and TASKS.md is
  byte-identical.
</behavior>

<scenario id="CHK-003">
Given fixture tasks in each of the four states,
when every ordered state pair is attempted (16 combinations),
then exactly the six legal edges plus the four same-state no-ops
succeed and the remaining six exit non-zero with both state names in
the diagnostic.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `speccy journal append` writes one validated block to a per-task journal

`speccy journal append <task-selector> --block
{implementer|review|blockers}` reads the block body from stdin and
appends exactly one well-formed block to
`<spec-dir>/journal/<task-id>.md`. The caller supplies only judgment
and identity inputs (`--model`, `--persona`, `--verdict`, body); the
CLI stamps `date` (UTC now, ISO8601 with seconds and timezone) and
derives `round` from file state: an `implementer` block opens a new
round (`max existing round + 1`, or `1` on a fresh file); `review`
and `blockers` blocks attach to the current round and are rejected
if no `implementer` block exists yet. Validation runs before any
write: required attributes for the block type, `persona` against the
persona registry, `verdict` against `{pass, blocking}`, body
non-empty, and no nested journal elements in the body. A validation
failure exits non-zero and leaves the journal byte-identical. When
the journal file does not exist, the first append creates it with
CLI-stamped frontmatter (`spec`, `task`, `generated_at`). Every
successful append leaves a file that `journal_xml::parse` accepts.

<done-when>
- An `implementer` append on a fresh task creates the journal with
  valid frontmatter and a `round="1"` block whose `date` was not
  supplied by the caller.
- A `review` append lands with the round of the latest
  `implementer` block; a `review` append to a journal with no
  `implementer` block exits non-zero.
- `--persona` values outside the registry, `--verdict` values
  outside `{pass, blocking}`, an empty stdin body, and a body
  containing `<implementer>`/`<review>`/`<blockers>` markup each
  exit non-zero with the journal untouched.
- After any successful append, `journal_xml::parse` accepts the
  file, including its round-sequence validation.
- There is no flag to override `date` or `round`.
</done-when>

<behavior>
- Given a journal whose latest round is 2, when an `implementer`
  block is appended, then it carries `round="3"`.
- Given a journal whose latest round is 3 (opened by an
  implementer block), when a `review` block with
  `--persona tests --verdict blocking` is appended, then it carries
  `round="3"` and the CLI-stamped date.
- Given an empty stdin, when any append runs, then the exit code is
  non-zero and the journal file is byte-identical (or still absent).
</behavior>

<scenario id="CHK-004">
Given a scratch workspace with a pending task and no journal file,
when `speccy journal append SPEC-0042/T-001 --block implementer
--model test-model` runs with a body on stdin,
then the journal file exists, its frontmatter carries `spec`,
`task`, and a CLI-stamped `generated_at`, and the single
`<implementer>` block carries `round="1"` and an ISO8601 `date`.
</scenario>

<scenario id="CHK-005">
Given the journal from CHK-004,
when a `review` append with `--persona not-a-persona` runs,
then the exit code is non-zero and the journal bytes are unchanged.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: VET.md block grammar frozen into speccy-core and served by `journal append`

`speccy-core` gains a VET.md parser covering the full grammar that
today lives only in skill prose: frontmatter (`spec`,
`generated_at`), one `## Invocation N — <ISO8601>` section per vet
invocation, and the five block types with their attribute schemas —
`<drift-review verdict="pass|blocking" round date model>`,
`<holistic-fix verdict="addressed|blocking|stuck" round date model>`,
`<simplifier-scan verdict="clean|candidates">`,
`<simplifier-apply verdict="applied|blocking">`, and the terminal
`<gate verdict="passed|failed" tasks_hash date>` (exactly one per
invocation section, last block in its section). The tolerant
`<gate>` scanner in `next.rs` keeps its read path but the grammar's
source of truth moves to the parser.

`speccy journal append <SPEC-NNNN> --block
{drift-review|holistic-fix|simplifier-scan|simplifier-apply|gate}`
routes to `<spec-dir>/journal/VET.md` — the target journal is
inferred from the block type plus selector shape (DEC-004). The CLI
owns all environment-derivable values: `date` on every block,
`round` (a `drift-review` opens a round, a `holistic-fix` attaches
to the current round), `tasks_hash` on `gate` (lowercase hex SHA-256
of the sibling TASKS.md read at append time), and invocation
sectioning — when the file is absent or the last section is
gate-terminated, the append opens `## Invocation N+1` with a
CLI-stamped datetime before writing the block; a non-gate block
appended after a section's gate never lands in the closed section.

<done-when>
- The parser accepts the VET.md files the current vet skill
  produces (fixture-driven) and rejects unknown blocks, unknown
  attributes, out-of-domain verdict values, a non-terminal `gate`,
  and a second `gate` in one invocation section.
- A first `drift-review` append creates VET.md with frontmatter and
  `## Invocation 1`, and stamps `round="1"`.
- A `gate` append carries a `tasks_hash` equal to the SHA-256 of
  TASKS.md as read at append time, and `speccy next`'s freshness
  check accepts the produced block.
- An append after a gate-terminated section opens the next
  invocation section automatically; the caller has no flag for
  invocation numbers.
- A `holistic-fix` append with no preceding `drift-review` in the
  open section exits non-zero with VET.md untouched.
</done-when>

<behavior>
- Given a VET.md whose only section ends with a `gate`, when a
  `drift-review` append runs, then the file gains `## Invocation 2`
  and the block lands under it with `round="1"`.
- Given an open section with `drift-review` round 2, when a
  `holistic-fix` append runs, then it carries `round="2"`.
- Given a TASKS.md edit between two gate appends, when each gate is
  appended, then the two `tasks_hash` values differ and each equals
  the file hash at its own append time.
</behavior>

<scenario id="CHK-006">
Given a scratch spec with all tasks completed and no VET.md,
when a `drift-review` append, a `holistic-fix` append, and a
`gate --verdict passed` append run in order,
then VET.md parses under the new parser, holds one invocation
section ending in the gate, and `speccy next` resolves the spec past
the vet step (fresh gate-pass).
</scenario>

<scenario id="CHK-007">
Given the VET.md from CHK-006,
when a `simplifier-scan` append runs,
then the block lands under a freshly opened `## Invocation 2`
section rather than the gate-terminated first section.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Concurrent appends to one journal serialize via an internal file lock

`speccy journal append` acquires an advisory per-journal-file lock
before reading file state (round/invocation derivation) and releases
it after the write completes, so the derive-validate-append sequence
is atomic with respect to other appenders. Acquisition blocks until
the lock is free, with a 10-second timeout; on timeout the command
exits non-zero with a diagnostic naming the journal path. Lock usage
is internal — callers need no flags and personas need no retry
prose. The locking crate enters `[workspace.dependencies]` and must
clear `cargo deny check`.

<done-when>
- N processes appending concurrently to one journal produce a file
  with all N blocks, each well-formed, parsing cleanly under the
  matching grammar.
- Two concurrent `implementer`-style round-opening appends yield
  distinct, correctly ordered round numbers (the derivation happens
  under the lock).
- A held lock past the timeout causes the waiting append to exit
  non-zero, naming the journal path, with no partial bytes written.
</done-when>

<behavior>
- Given five concurrent `review` appends for five personas against
  one task journal, when all five complete, then the journal
  contains five `<review>` blocks at the same round and
  `journal_xml::parse` accepts the file.
- Given a process holding the journal lock for longer than the
  timeout, when an append runs, then it exits non-zero after
  roughly the timeout interval and the journal is byte-identical.
</behavior>

<scenario id="CHK-008">
Given a test that spawns 8 threads or processes each appending one
distinct `review` block to the same journal,
when all appenders finish,
then the journal contains exactly 8 review blocks with no
interleaved or truncated markup and the parser accepts the file.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: `speccy journal show` emits filtered journal JSON

`speccy journal show <selector> --json` parses the resolved journal
(task selector → `journal/T-NNN.md`; spec selector → VET.md per
DEC-004) and emits a schema-versioned JSON envelope
(`schema_version` first field, pinned `1`) carrying frontmatter and
blocks with their attributes and bodies. Minimal filters, derived
from the orchestrator's three call sites (persona-completeness
check, blocker read-back, retry-shape detection): `--round
latest|N`, `--verdict <value>`, `--block <type>`. Filters compose
conjunctively. A missing journal file exits non-zero — the known
call sites only run after blocks must exist, so absence is an
anomaly worth surfacing loudly.

<done-when>
- `--round latest` returns only blocks of the highest round in the
  file, plus the round number itself in the envelope.
- `--verdict blocking --round latest` returns exactly the blocking
  blocks of the latest round.
- `--block review --round N` on a task journal lists the personas
  that reviewed round N (the completeness call site).
- Output is valid JSON with `schema_version: 1` as the first field;
  a missing journal exits non-zero with a diagnostic.
</done-when>

<behavior>
- Given a journal with rounds 1–3 where round 3 has one blocking
  review, when `show --round latest --verdict blocking` runs, then
  the JSON contains exactly that one block.
- Given a spec selector, when `show` runs, then VET.md is parsed
  and its invocation sections and blocks appear in the JSON.
</behavior>

<scenario id="CHK-009">
Given a fixture journal with two rounds and five reviews in round 2
(one blocking),
when `speccy journal show SPEC-0042/T-001 --json --round latest
--verdict blocking` runs,
then stdout parses as JSON whose block list has length 1, persona
and verdict matching the fixture, and `schema_version` equal to 1.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: `speccy verify` gains a minimal VET lint family

With the VET grammar frozen in core, `speccy verify` lints
`journal/VET.md` when the file exists, mirroring the JNL family's
posture: `VET-001` — the file fails the frozen grammar (frontmatter,
block shapes, attribute domains, round sequencing); `VET-002` — an
invocation section other than the last lacks a terminal `gate`, or
any block follows a `gate` within its section. Both are errors,
since `speccy next` gates shipping on this artifact. Specs without a
VET.md emit nothing — absence is handled by `speccy next`'s
resolver, not lint.

<done-when>
- A workspace whose VET.md was produced solely by `journal append`
  passes `speccy verify` with no `VET-*` diagnostics.
- A hand-corrupted VET.md (unknown attribute, missing gate in a
  non-final section, block after gate) fails `speccy verify` with
  the matching `VET-*` code, and the JSON envelope counts it as an
  error.
- A spec directory without VET.md produces no `VET-*` diagnostics.
</done-when>

<behavior>
- Given a VET.md whose first invocation section has no `gate` while
  a second section exists, when `speccy verify` runs, then VET-002
  is reported and the exit code is non-zero.
- Given no VET.md anywhere in the workspace, when `speccy verify`
  runs, then no `VET-*` code appears.
</behavior>

<scenario id="CHK-010">
Given a fixture VET.md with a `verdict="maybe"` drift-review block,
when `speccy verify --json` runs,
then the envelope lists a VET-001 error naming the file and the
process exits non-zero.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Loop subagents append their own blocks and return thin verdicts

The skill-pack source modules under `resources/` change contract so
that each block author writes its own block via `speccy journal
append` and returns a thin verdict — persona (or role), verdict, and
a one-line rationale — instead of a full block body: (a) the
`speccy-work` implementer phase appends its `<implementer>` block;
(b) the six reviewer personas append their `<review>` blocks; (c)
the vet personas append `<drift-review>`, `<holistic-fix>`,
`<simplifier-scan>`/`<simplifier-apply>` respectively. The shared
verdict-return partial is updated once and inherited by every
wrapper; persona bodies stop instructing agents to compute dates or
round numbers. Both host ejections (`just reeject`) carry the new
contract.

<done-when>
- No persona or phase module under `resources/modules/` instructs
  the agent to return a full journal block body as its final
  message; each instructs a `journal append` invocation plus a thin
  verdict return.
- No module instructs the agent to compute or supply `date`,
  `round`, `tasks_hash`, or invocation numbers.
- The ejected packs under `.claude/`, `.agents/`, and `.codex/`
  regenerate from source with the new contract present in each
  affected wrapper.
- The Codex pack carries the same contract in prose (its write
  discipline remains prose-enforced; no per-agent tool gating is
  assumed).
</done-when>

<behavior>
- Given the regenerated Claude Code pack, when a reviewer persona
  file is read, then it directs the persona to run `speccy journal
  append ... --block review --persona <self> --verdict <v>` with
  its findings on stdin and to end with a thin verdict line.
- Given the regenerated packs, when the implementer phase body is
  read, then the seven-field handoff lands via `journal append`
  rather than inside the final return message.
</behavior>

<scenario id="CHK-011">
Given a clean checkout after this SPEC lands,
when `just reeject` runs and `git status --porcelain` is checked,
then the working tree is clean — proving the committed ejected packs
match the updated sources.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: Orchestrator and reconcile flows consume the new contract

The orchestrate, review, vet, and amend/reconcile module sources
stop hand-editing lifecycle files: every TASKS.md state mutation
they direct goes through `speccy task transition` (including the
reconcile policy's flips and the amendment `completed → pending`
invalidation); the persona-completeness check before leaving
`in-review` and the blocking-review read-back for `<blockers>`
consolidation go through `speccy journal show`; the consolidated
`<blockers>` block (whose body stays orchestrator-authored) lands
via `speccy journal append`. The standalone `/speccy-work` primitive
likewise flips its own states through the transition command.

<done-when>
- No module under `resources/modules/` instructs an agent to edit a
  TASKS.md `state` attribute or a journal file with file-editing
  tools; lifecycle mutations reference the CLI verbs.
- The orchestrate flow verifies all expected persona `<review>`
  blocks exist for the current round via `journal show` before
  transitioning a task out of `in-review`.
- The reconcile policy's auto-fix rows reference `speccy task
  transition` for state flips.
- The documented single-writer rule is restated: the CLI's append
  lock owns write serialization; the orchestrator remains the sole
  author of `<blockers>` bodies and of git commits.
</done-when>

<behavior>
- Given the regenerated orchestrate skill, when its review
  consolidation section is read, then it directs `journal show
  --block review --round latest` for completeness, `--verdict
  blocking` for read-back, `journal append --block blockers` for
  the directive, and `task transition` for the state flip.
- Given the regenerated reconcile policy reference, when its
  auto-fix table is read, then state-flip remedies name the
  transition command instead of editing TASKS.md.
</behavior>

<scenario id="CHK-012">
Given the regenerated module sources,
when `grep -r` over `resources/modules/` searches for instructions
to hand-edit task `state` attributes or journal files,
then no lifecycle-mutation instruction bypasses the CLI verbs
(content check performed by the reviewer; phrasing may vary, the
invariant is the absence of file-edit-tool mutation paths).
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: ARCHITECTURE.md matches the shipped behavior

`docs/ARCHITECTURE.md` is updated in the same change: the CLI
surface section documents the three new commands and their JSON
envelopes; the TASKS.md state model's "Who sets it" column and the
journal sections describe the CLI-mediated writes; the concurrency
contract replaces the prose-level "sole serial writer" rule with the
CLI-serialized append contract; the "What We Deliberately Don't Do"
row for claim files / leases is narrowed to task claiming with a
pointer to this SPEC's append-serialization decision; the lint-code
catalogue gains the `VET-*` family.

<done-when>
- Each of the five listed sections reflects the new behavior, and
  no section still asserts that sub-agents return blocks for the
  orchestrator to transcribe as the only write path.
- The exclusions table no longer reads as forbidding append
  serialization.
</done-when>

<behavior>
- Given the updated ARCHITECTURE.md, when the vet journal section is
  read, then the "skill body is the only writer" sentence is
  replaced by the CLI-append contract.
</behavior>

<scenario id="CHK-013">
Given the updated ARCHITECTURE.md,
when the reviewer reads the CLI surface table, the state model, the
journal grammar sections, the exclusions row, and the lint
catalogue,
then each describes the post-SPEC-0055 behavior with no stale
sole-writer or no-locking claims (content check by reviewer, not
substring assertions).
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
The CLI is the authority for every environment-derivable attribute:
`date`, `round`, frontmatter `generated_at`, `gate.tasks_hash`, and
invocation numbering. Callers supply only identity and judgment
(`model`, `persona`, `verdict`, block body). No override flags ship;
if a flow needs one it must surface through dogfooding first.
Rationale: agents fabricate and mis-format clocks and counters;
round-sequence violations are today caught only at verify time, and
every such attribute is mechanically derivable from the environment
at append time.
</decision>

<decision id="DEC-002">
Lock acquisition blocks with a 10-second timeout rather than failing
fast. Appends take milliseconds, so contention is invisible to
agents and no persona needs retry prose — pushing retry logic into
prompts would reintroduce the bookkeeping-in-prose pattern this SPEC
removes. The timeout guards against a wedged lock holder and errors
loudly instead of spinning.
</decision>

<decision id="DEC-003">
A transition whose target equals the current state is a no-op
success (exit 0). The reconcile policy is built on idempotent flips;
making same-state transitions errors would force the reconcile flow
to pre-read state before every flip, recreating the read-before-
write tax for no safety gain.
</decision>

<decision id="DEC-004">
The target journal is inferred from block type plus selector shape:
task-journal block types require a task selector and route to
`journal/T-NNN.md`; vet block types require a bare spec selector and
route to `journal/VET.md`. A mismatched pairing is an argument
error. No `--vet` flag; the block type already carries the
information.
</decision>

<decision id="DEC-005">
The VET.md grammar is frozen into `speccy-core` as a real parser.
After this SPEC, changing the vet block grammar requires a core PR,
not just a prose edit to the vet skill. Accepted deliberately: the
grammar is ship-gate-load-bearing (`speccy next` reads it), so
prose-only definition is the riskier posture.
</decision>

<decision id="DEC-006">
The thin verdict a subagent returns is persona (or role), verdict,
and a one-line rationale. Bare verdicts would force the orchestrator
to run `journal show` even on all-pass rounds just to narrate
progress; one sentence is cheap and the full substance is read back
via `journal show --verdict blocking` only when something blocked.
</decision>

<decision id="DEC-007">
Locking is advisory, per-journal-file, same-host only (an
`fd-lock`/`fs2`-class crate; both clear `deny.toml` license policy).
Distributed locking remains a harness concern per the existing
exclusion. Block bodies arrive via stdin to avoid shell-quoting
multi-line markdown through argv.
</decision>

<decision id="DEC-008">
The VET.md parser is the single authority for `journal append`'s vet
path — there is no parallel hand-rolled text scan or body-markup
guard. The parser exposes two entry points over one shared pipeline
(frontmatter split, `xml_scanner` tag scan, block assembly, body-range
and heading exclusion, per-section round-sequence validation): the
strict `parse` requires every invocation section to end in a terminal
`<gate>` — the complete-file grammar `speccy verify`, `journal show`,
and `speccy next`'s freshness check rely on, behaviourally unchanged —
and an in-flight `parse_in_flight` that relaxes only the terminal-gate
rule for the *last* section, so the open trailing section that exists
mid-vet-run (after a `drift-review`, before its `gate`) parses. The
append path derives invocation/round state from the typed in-flight
parse of the existing file (mirroring the per-task journal path, which
already derives `round` from a typed `parse`), and validates the
would-be-new file by re-parsing it through the same parser before any
byte is written: any body that would make the produced file
unparseable is refused at write time, with VET.md left byte-identical
or still absent.

Refines DEC-005: the block grammar stays frozen and byte-for-byte
unchanged — this adds a parse *mode*, not a grammar change, so it does
not reopen the frozen-grammar contract. Rationale: the original
implementation derived state from a separate tolerant text scan plus a
body-inertness guard that each re-implemented the parser's
tag/heading/body-range definitions; the two readers diverged from the
parser needle by needle (a tag-boundary or whitespace class one
honoured and the other missed). Collapsing to one parser removes the
divergence class by construction.
</decision>

## Notes

Rejected framings from the brainstorm session:

- **Orchestrator-only mechanical writes** — CLI verbs invoked solely
  by the orchestrator, subagents still returning full block bodies.
  Half the spec, no locking, no persona contract change; rejected
  because the relay tax (verdict bodies crossing the orchestrator
  context twice) is the dominant token cost and would survive.
- **One generic `speccy edit` patch verb** — a single selector+patch
  grammar mutating any lifecycle file. Rejected: loses write-time
  domain validation, violates one-job-per-command, and opens an
  arbitrary-mutation surface.
- **Sidecar state store (SQLite / state.json)** — cleanest
  concurrency story, rejected because markdown-as-shared-state is
  load-bearing: git-diffable audit trail, human-readable, single
  authoritative artifact.
- **Daemon / MCP server owning all writes** — solves serialization
  by construction but is an orchestration runtime, explicitly
  excluded by the architecture.

This SPEC is the first of an agreed pair; the read-side
`speccy context` task-scoped bundle command is the follow-up and is
explicitly out of scope here.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-09 | claude-fable-5[1m] | Initial SPEC: mechanical lifecycle write commands. REQ-001/002: `speccy task transition` byte-surgical state rewrite over a closed six-edge legal graph with same-state no-ops. REQ-003: validated `journal append` for per-task journals with CLI-stamped date/round and frontmatter-on-create. REQ-004: VET.md grammar frozen into speccy-core (five block types, invocation sections, gate tasks_hash) and served by append with mechanical invocation management. REQ-005: advisory per-file lock, blocking acquire, 10s timeout. REQ-006: `journal show` filtered JSON (round/verdict/block). REQ-007: VET-001/VET-002 lint family in verify. REQ-008: subagents self-append blocks, return thin verdicts. REQ-009: orchestrator/reconcile flows adopt transition + show + append; blockers body stays orchestrator-authored. REQ-010: ARCHITECTURE.md updated (commands, state model, concurrency contract, narrowed claim-files exclusion, VET lints). Seven decisions DEC-001..DEC-007. Follow-up spec: read-side `speccy context` bundle. |
| 2026-06-09 | claude-opus-4-8[1m] | Added DEC-008 (refines DEC-005) mid-implementation of T-004. The VET.md parser becomes the single authority for the vet `journal append` path: a new in-flight parse mode relaxes only the last section's terminal-gate rule so the open mid-vet-run section parses, the append derives invocation/round state from the typed in-flight parse (mirroring the per-task path), and the would-be-new file is re-parsed before any write. Removes the parallel tolerant text scan + body-markup guard, whose independent re-implementations of the parser's tag/heading/body-range definitions diverged from it across seven review rounds. No block-grammar change; no observable contract change to REQ-004. |
</changelog>
