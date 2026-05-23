---
id: SPEC-0037
slug: task-journal-files
title: Per-task journal files — eject implementer / review / blockers from TASKS.md
status: implemented
created: 2026-05-21
supersedes: []
---

# SPEC-0037: Per-task journal files — eject implementer / review / blockers from TASKS.md

## Summary

Today's TASKS.md accumulates three kinds of per-task activity prose:
`<implementer-note>` blocks (one per implementer attempt),
`<review>` blocks (one per persona per review round), and
`<retry>` blocks (one per blocking review round). For a single task
that goes through two implement/review rounds, that's typically one
implementer-note + four reviewer blocks + one retry — repeated for
the second round. For SPEC-0035 today, T-001's TASKS.md entry holds
eight `<review>` blocks plus two implementer-notes plus one retry.
The file is 2-3× the size it had immediately after decomposition,
and most of that growth is forensic prose that no agent reads
again after the loop closes on that task. Every loop iteration
re-reads the entire bloated file to find the parts that matter,
paying a per-iteration token tax that compounds with retry count.
The single-file design also concentrates write-collision risk: the
reviewer fan-out is serialized through the orchestrator today
(DEC-008 in `speccy-review.md`), but the file-level race surface is
still real — one parser quirk or one mid-loop manual edit can
corrupt the activity record for every task in the spec at once.

This SPEC ejects all three element kinds out of TASKS.md and into a
new sibling artifact, `journal/T-NNN.md` — one file per task. The
journal file carries a small YAML frontmatter binding it to the spec
and task (no wrapper element; the filename + frontmatter are the
binding) and a chronological sequence of `<implementer>`,
`<review>`, and `<blockers>` blocks for that task only. Reading
"what happened on T-005" becomes one file read scoped to T-005's
history. Reading TASKS.md becomes scanning task definitions and
state attributes against a near-static file that mutates only on
state transitions and (rarely) net-new task appends.

The element schema picks up traceability attributes the existing
shape lacks. `<implementer-note>` (renamed to `<implementer>` since
the file path already disambiguates) carries `date`, `model`, and
`round`. `<review>` adds `date`, `model`, and `round` to its
existing `persona` and `verdict`. `<retry>` (renamed to `<blockers>`
since the element holds the orchestrator's aggregated blocker
directives, not a generic retry signal) carries `date` and `round`.
TASKS.md additionally drops its redundant `<tasks spec="SPEC-NNNN">`
wrapper element — the filename `TASKS.md`, parent folder
`NNNN-slug`, and frontmatter `spec:` field all already encode the
binding three times over, so the wrapper element earns nothing.
After this SPEC, TASKS.md contains bare `<task>` children directly
under the `# Tasks:` heading.
The `round` attribute unifies what was ad-hoc `session="...attempt-2"`
or `session="...rev2"` string-embedding into a typed integer that
counts implement/review loop turns monotonically from 1. Effort /
reasoning-intensity rides as a slash-suffix on `model` itself
(e.g. `model="claude-opus-4.7[1m]/low"`) — a simple string concat
that avoids an optional attribute for the host harnesses that don't
expose an effort knob.

A new lint family, `JNL-*`, validates journal files in a
lifecycle-aware way: per-task, gated by the task's `state`
attribute. Tasks at `state="pending"` must NOT have a journal file
(clean-slate guarantee after decomposition; also catches orphans
when amend flips a completed task back to pending). Tasks at
`state="in-progress"` or `state="in-review"` skip all journal lints
(the loop is mid-flight; partial state is expected). Tasks at
`state="completed"` MUST have a journal file with a valid schema
and correct bindings. `speccy verify` enforces these as errors —
the pre-ship gate fails on any violation. The lint never runs
mid-loop to second-guess work in progress.

TASKS.md gains an unconditional rule: it no longer accepts
`<implementer>`, `<review>`, or `<blockers>` elements. Lint fires
on any appearance, regardless of task state. TASKS.md continues to
accept net-new `<task>` elements appended mid-implementation when
scope gaps are discovered (R10 below preserves this — it's the
only legitimate mid-loop mutation to TASKS.md besides state
transitions).

The change ripples through three skills that author or aggregate
these elements: `speccy-work.md` (implementer writes to journal),
`speccy-review.md` (orchestrator appends reviews to journal),
`speccy-amend.md` (amendment-driven blockers go to journal, not
into the `<task>` body). The reviewer return contract in
`verdict_return_contract.md` gains a requirement that each
reviewer subagent's returned `<review>` carry its own `model`
attribute — without it, the orchestrator (transcribing the
subagent's reply into the journal) would have to guess model
identity across heterogeneous reviewer subagents.

The original F-11 backlog framing imagined a hash-lock on TASKS.md
after impl and after review. That framing is dropped (DEC-005):
once notes are ejected, TASKS.md becomes near-static, so a
file-level hash lock no longer earns its weight. Drift detection
becomes implicit via the task state machine — every transition
through `state="pending"` (initial claim, blocking review,
amend-flip) forces a fresh re-read of the task subtree on the next
claim. The amend skill already flips affected `completed` tasks
back to `pending` on SPEC change (`speccy-amend.md` lines 106-111);
this SPEC inherits that behavior unchanged.

A full historical rewrite of SPEC-0001 through SPEC-0036 is in
scope. Every legacy `<implementer-note session="...">` and
`<review persona="..." verdict="...">` and `<retry>` element gets
moved from its TASKS.md into a per-task `journal/T-NNN.md` with
the new schema. Migration is performed manually by LLM
implementers (not by a script) — the legacy `session="..."`
strings encode date and attempt-number inconsistently across
specs, and recovering them faithfully requires interpretive
judgment that a regex script can't do. The `JNL-*` lint family
plus the TASKS.md "no notes elements" rule together form the
safety net: an LLM that produces a lint-clean rewrite is trusted
to have preserved legacy content faithfully (A4 below).

## Goals

<goals>
- A new artifact location, `.speccy/specs/NNNN-slug/journal/T-NNN.md`,
  is the canonical home for `<implementer>`, `<review>`, and
  `<blockers>` blocks. One file per task. Filename binds the file
  to its task; YAML frontmatter binds it to the spec.
- TASKS.md becomes near-static after decomposition. The only
  permitted mid-loop mutations are: (a) the `state="..."`
  attribute on existing `<task>` elements as the state machine
  advances, and (b) net-new `<task>` elements appended when
  scope gaps are discovered mid-implementation. The three
  activity-prose element kinds (`<implementer>`, `<review>`,
  `<blockers>`) are unconditionally rejected from TASKS.md by
  lint, at any task state.
- Element renames land: `<implementer-note>` becomes
  `<implementer>` (the "-note" suffix is redundant when the file
  location is itself a journal); `<retry>` becomes `<blockers>`
  (the element holds the orchestrator's aggregated blocker
  directives, so the noun-as-artifact name matches `<review>`'s
  pattern). The redundant `<tasks spec="SPEC-NNNN">` wrapper in
  TASKS.md is dropped — the filename, parent directory, and
  frontmatter `spec:` field already encode the binding (DEC-008).
  The closed XML element set shrinks from six to five:
  `task`, `task-scenarios`, `implementer`, `review`, `blockers`.
- Element attribute schemas pick up traceability fields:
  `<implementer>` carries `date`, `model`, `round`;
  `<review>` carries `date`, `model`, `persona`, `verdict`, `round`;
  `<blockers>` carries `date`, `round`. All attributes are required
  on their respective elements; there are no optional attributes
  in the new schema. The legacy `session="..."` attribute on
  `<implementer-note>` is removed from the allowed set.
- `round` is a monotonic integer counter starting from 1 within a
  journal file. Round 1 = the first implementer attempt + its
  reviewer fan-out + (if blockers fire) the first blockers block.
  Round 2 = post-retry implementer attempt + the next reviewer
  fan-out, and so on. The counter unifies what was ad-hoc
  session-string embedding (`session="...attempt-2"`,
  `session="...rev2"`) into a typed attribute.
- `model` is a free-form string. By convention, an
  effort/reasoning-intensity suffix is concatenated via slash
  (e.g. `model="claude-opus-4.7[1m]/low"`). The slash-suffix is
  documented in skill prose; the lint validates `model` is
  non-empty but does NOT validate the suffix's value membership
  (the model surface is too volatile to make the parser
  authoritative on it).
- `date` is full ISO8601 date-time with seconds and timezone
  designator (e.g. `2026-05-21T18:00:00Z` or
  `2026-05-21T18:00:00+00:00`). Element ordering within a journal
  is recoverable from the timestamp itself, not from file
  position.
- A new lint family, `JNL-*`, validates journal-related shape
  per-task, gated by task state. `JNL-001` fires (error) on
  tasks at `state="pending"` if `journal/T-NNN.md` exists.
  `JNL-002` fires (error) on tasks at `state="completed"` if the
  journal file is missing. `JNL-003` fires (error) on
  shape/binding violations of an existing journal file when its
  task is `state="completed"`. Tasks at `state="in-progress"` or
  `state="in-review"` skip all `JNL-*` lints entirely. The
  three-letter family code matches existing `SPC`, `REQ`, `VAL`,
  `TSK`, `QST`, `RPT` naming.
- Three skill bodies update to read/write the journal instead of
  TASKS.md: `speccy-work.md` (implementer writes `<implementer>`
  to `journal/T-NNN.md`); `speccy-review.md` (orchestrator
  appends each persona's `<review>` block to the journal,
  serially per DEC-008); `speccy-amend.md` (amendment-driven
  `<blockers>` block writes to the journal instead of into the
  `<task>` body in TASKS.md).
- The reviewer return contract (`verdict_return_contract.md`)
  updates: each reviewer subagent's returned `<review>` element
  must carry the subagent's own `model` attribute (with optional
  slash-suffix). The orchestrator transcribes the model verbatim
  into the journal. Without this, the orchestrator cannot
  reliably record per-reviewer model identity across
  heterogeneous reviewer subagents.
- All 37 in-tree specs (SPEC-0001 through SPEC-0037, inclusive
  of SPEC-0037 itself) get a manual one-time content rewrite
  as part of this SPEC's TASKS.md: legacy
  `<implementer-note session="...">`,
  `<review persona="..." verdict="...">`, and `<retry>` elements
  move from each TASKS.md into per-task `journal/T-NNN.md`
  files with the new schema; the `<tasks spec="...">` wrapper
  is also stripped from every TASKS.md (REQ-005, DEC-008).
  SPEC-0037 is included because the implementer + reviewer
  agents working on T-001 through T-006 follow current skill
  bodies and write activity prose in legacy format into
  SPEC-0037's own TASKS.md; T-007 unifies the cleanup.
  Migration is performed by LLM implementer agents, not by a
  script. The `JNL-*` lint family is the migration's correctness
  floor — a lint-clean rewrite is the acceptance criterion.
- `ARCHITECTURE.md` and `AGENTS.md` get updates documenting the
  new layout, element schemas, and `JNL-*` lint family.
</goals>

## Non-goals

<non-goals>
- No hash-lock mechanism for TASKS.md. The original F-11 backlog
  framing imagined "lock TASKS.md when impl is done / when
  review is done" as an analogue to the existing
  `spec_hash_at_generation` SPEC → TASKS hash. Once notes are
  ejected, TASKS.md is near-static (state-attribute flips +
  occasional net-new task appends only); a file-level lock no
  longer earns its weight. State-machine drift detection covers
  the amendment-mid-task case (per DEC-005).
- No `task_hash_at_generation` field in the journal frontmatter
  either. An earlier brainstorm draft proposed it for catching
  amendment-mid-task drift; rejected in favor of skill-level
  state-machine recovery (the amend skill already flips affected
  `completed` tasks back to `pending`, which forces a fresh
  read on the next claim).
- No `effort` attribute on `<implementer>` or `<review>`. Effort
  is encoded as a slash-suffix on `model` (e.g.
  `model="claude-opus-4.7[1m]/low"`). Simple string concat; no
  optional attribute the parser has to special-case; no extra
  schema variant for hosts without an effort knob.
- No `host` attribute. Host identity is recoverable from `model`
  family + the skill-pack identity in practice; an extra
  attribute would be redundant.
- No new closed-set XML element. The journal file uses bare
  child elements (`<implementer>`, `<review>`, `<blockers>`)
  under frontmatter only — no wrapper like `<task-journal>`.
  Symmetrically, TASKS.md loses its `<tasks>` wrapper for the
  same reason: filename + directory + frontmatter already encode
  the binding. The closed set shrinks to five (`task`,
  `task-scenarios`, `implementer`, `review`, `blockers`).
- No `speccy migrate` or `speccy convert` CLI subcommand.
  Migration is purely content rewrite tasked to LLM agents using
  the existing `speccy check` / `speccy verify` lint surface as
  the feedback loop. Adding a CLI verb for a one-time migration
  would walk back stay-small for no durable benefit.
- No migration script (regex-based or otherwise). Legacy
  `session="..."` strings encode date and attempt-number
  inconsistently across the 36 historical specs; faithful
  migration requires interpretive judgment that a script can't
  reliably do. The LLM-driven approach is the contract.
- No changes to `evidence/T-NNN.md` location, format, or
  semantics. Evidence stays a separate sibling directory under
  `.speccy/specs/NNNN-slug/`. The cross-reference convention
  (`Evidence: evidence/T-NNN.md` field inside an
  `<implementer>` block) carries through unchanged.
- No retroactive rewrite of other parts of historical specs.
  The migration touches only the legacy activity-prose
  elements in TASKS.md and the per-task journal files those
  elements move into. SPEC.md, REPORT.md, evidence/, and the
  non-activity parts of TASKS.md (task definitions, state
  attributes, frontmatter) for SPEC-0001..0036 stay
  byte-identical.
- No JNL-* lint at info or warning severity. The three rules
  fire as errors when applicable (state-gated per R4); they
  don't fire at all when the gate doesn't apply. No
  "suspicious but not failing" middle ground.
- No CLI surface for journal navigation (`speccy journal T-005`
  or similar). The file location follows a predictable
  convention; agents and humans read it directly.
- No `<implementer-note>` or `<retry>` element names after F-11
  ships. Post-migration, neither name parses anywhere. The
  parser allow-list contains only the renamed forms.
</non-goals>

## User Stories

<user-stories>
- As a solo developer reading TASKS.md for a long-running spec, I
  want to see only task definitions, state attributes, and the
  occasional net-new task addition — not the 2-3× accumulated
  activity prose. Per-task journal files eat the activity prose;
  TASKS.md stays small and scannable across the spec's lifetime.
- As an implementer agent reading the latest activity on T-005
  before retrying after a blocking review, I want a single file
  that holds only T-005's history — model identities, round
  numbers, blocker directives — without paying the token cost of
  every other task's history. `journal/T-005.md` holds exactly
  what I need.
- As a reviewer-fan-out orchestrator transcribing each persona's
  returned `<review>` block into the journal, I want each
  subagent's reply to carry its own `model` attribute. Without
  that, I'd have to guess model identity across heterogeneous
  reviewer subagents, which is a real possibility when reviewer
  personas pin different model tiers.
- As `speccy verify` running on a SPEC about to ship, I want a
  mechanical check that every `state="completed"` task has a
  well-formed journal file. The `JNL-002` and `JNL-003` lints
  fire as errors at this pre-ship checkpoint; CI fails on any
  violation.
- As an agent reading a historical SPEC (0001-0036) after F-11
  ships, I want one consistent format across the entire repo —
  not legacy `session="..."` cruft in some specs and the new
  schema in others. The historical rewrite ensures format
  uniformity.
- As an agent in the middle of implementing T-005 (state
  `in-progress`), I want `speccy check` to leave my partial
  journal state alone. `JNL-*` lints skip in-progress and
  in-review tasks; the loop doesn't get noisy with "you haven't
  written the journal yet" warnings while I'm still working.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Per-task journal files at `.speccy/specs/NNNN-slug/journal/T-NNN.md`

A new artifact location, `journal/T-NNN.md` (one file per task,
sibling to the existing `evidence/` directory), is the canonical
home for `<implementer>`, `<review>`, and `<blockers>` element
blocks. The file shape is YAML frontmatter followed by bare XML
element blocks — no wrapper element. The filename's `T-NNN`
component and the frontmatter's `task:` field together bind the
file to its task in TASKS.md; the frontmatter's `spec:` field binds
the file to its parent SPEC directory.

<done-when>
- The directory `.speccy/specs/NNNN-slug/journal/` is the canonical
  parent for per-task journal files. The directory is a sibling
  to `evidence/`, not nested inside it.
- Each journal file is named `T-NNN.md` matching the corresponding
  `<task id="T-NNN">` element in TASKS.md. The file extension is
  `.md` (the body is markdown-with-embedded-XML, same as TASKS.md).
- The frontmatter block at the top of each journal file declares
  exactly three fields: `spec:` (the parent SPEC ID, e.g. `SPEC-0037`),
  `task:` (the task ID, e.g. `T-005`), and `generated_at:` (the
  ISO8601 timestamp when the file was created or last
  fully-regenerated).
- The body below the frontmatter contains a chronological sequence
  of bare `<implementer>`, `<review>`, and `<blockers>` element
  blocks — no wrapper element groups them.
</done-when>

<behavior>
- Given a SPEC dir `.speccy/specs/0042-example/` with TASKS.md
  declaring `<task id="T-001">` at `state="completed"`, when an
  agent reads the journal file for that task, then it reads
  exactly `.speccy/specs/0042-example/journal/T-001.md`.
- Given a freshly-decomposed TASKS.md (all tasks `state="pending"`),
  when the spec directory is listed, then the `journal/`
  subdirectory may or may not exist, but it contains no
  per-task files (per REQ-002's `JNL-001`).
- Given a completed SPEC immediately before ship, when the
  spec directory is listed, then `journal/` contains exactly N
  files where N is the number of `<task>` elements in TASKS.md;
  each file is named `T-NNN.md` with `NNN` matching a task ID.
</behavior>

<scenario id="CHK-001">
Given a tempdir workspace with a spec at
`.speccy/specs/0042-example/` whose TASKS.md declares
`<task id="T-001" state="completed">` and contains a
`journal/T-001.md` with valid frontmatter
(`spec: SPEC-0042`, `task: T-001`, `generated_at: 2026-05-21T18:00:00Z`)
and at least one well-formed `<implementer>` block in the body,
when `speccy check SPEC-0042` runs,
then no `JNL-*` lint errors fire on the journal file.
</scenario>

<scenario id="CHK-002">
Given the same tempdir workspace with the journal file's
frontmatter declaring `task: T-001` but the filename being
`journal/T-999.md`,
when `speccy check SPEC-0042` runs,
then `JNL-003` fires (error) with a message naming the
filename ↔ frontmatter `task:` binding mismatch.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Lifecycle-aware `JNL-*` lint family gated by per-task state

A new lint family, `JNL-*`, validates journal-related shape
per-task, gated by the task's `state` attribute in TASKS.md.
Tasks at `state="pending"` must have NO journal file
(`JNL-001`). Tasks at `state="completed"` must have a
well-formed journal file (`JNL-002` on missing,
`JNL-003` on shape/binding violations). Tasks at
`state="in-progress"` or `state="in-review"` skip all `JNL-*`
lints — the lint never runs mid-loop.

<done-when>
- The lint registry contains three new lint codes: `JNL-001`,
  `JNL-002`, `JNL-003`. The family code (`JNL`) follows the
  existing three-letter pattern (`SPC`, `REQ`, `VAL`, `TSK`,
  `QST`, `RPT`).
- `JNL-001` fires at severity `error` on any task at
  `state="pending"` whose corresponding `journal/T-NNN.md`
  file exists.
- `JNL-002` fires at severity `error` on any task at
  `state="completed"` whose corresponding `journal/T-NNN.md`
  file is missing.
- `JNL-003` fires at severity `error` on any task at
  `state="completed"` whose corresponding `journal/T-NNN.md`
  file has shape or binding violations: filename `T-NNN.md` ↔
  frontmatter `task:` mismatch; frontmatter `spec:` ↔ parent
  spec directory mismatch; missing or unparseable frontmatter
  fields; element attribute schema violations per REQ-003.
- Tasks at `state="in-progress"` or `state="in-review"` are
  silently skipped by all three lint codes. No diagnostic
  fires at any severity for these tasks' journal files
  (present, absent, or malformed).
- `speccy verify` exits non-zero on any `JNL-*` error.
- `speccy check` surfaces `JNL-*` errors in its text and JSON
  output but does not by itself exit non-zero (current
  `speccy check` behavior — verify is the gate).
</done-when>

<behavior>
- Given a spec where every task is `state="pending"` and the
  `journal/` directory is empty (or missing), when
  `speccy check SPEC-NNNN` runs, then no `JNL-*` diagnostic
  fires.
- Given a spec where T-001 is `state="pending"` but
  `journal/T-001.md` exists, when `speccy check SPEC-NNNN`
  runs, then `JNL-001` fires as an error naming T-001 and the
  unexpected file path.
- Given a spec where every task is `state="completed"` and
  every task has a well-formed journal file, when
  `speccy verify` runs, then no `JNL-*` diagnostic fires and
  the command exits 0.
- Given a spec where T-002 is `state="completed"` but
  `journal/T-002.md` is missing, when `speccy verify` runs,
  then `JNL-002` fires as an error and the command exits
  non-zero.
- Given a spec where T-003 is `state="in-progress"` and
  `journal/T-003.md` is malformed (frontmatter `task:` doesn't
  match the filename), when `speccy check SPEC-NNNN` runs,
  then no `JNL-*` diagnostic fires for T-003 (in-progress
  state skips the lint).
- Given a spec where T-004 is `state="in-review"` and the
  `journal/T-004.md` file does not yet exist, when
  `speccy check SPEC-NNNN` runs, then no `JNL-002` (missing
  file) diagnostic fires for T-004.
</behavior>

<scenario id="CHK-003">
Given a tempdir spec with `<task id="T-001" state="pending">`
and a `journal/T-001.md` file present,
when `speccy verify` runs,
then exit code is non-zero, stderr contains the literal
substring `JNL-001`, and the JSON envelope (via `speccy verify
--json`) lists exactly one `Diagnostic` with `code == "JNL-001"`
and `file` ending in `/journal/T-001.md`.
</scenario>

<scenario id="CHK-004">
Given a tempdir spec with `<task id="T-002" state="completed">`
and NO `journal/T-002.md` file,
when `speccy verify` runs,
then exit code is non-zero, stderr contains the literal
substring `JNL-002`, and the JSON envelope lists exactly one
`Diagnostic` with `code == "JNL-002"`.
</scenario>

<scenario id="CHK-005">
Given a tempdir spec with `<task id="T-003" state="completed">`
and a `journal/T-003.md` whose frontmatter declares
`task: T-999` (mismatching the filename),
when `speccy verify` runs,
then exit code is non-zero and the JSON envelope lists at
least one `Diagnostic` with `code == "JNL-003"` whose message
names the filename ↔ frontmatter binding mismatch.
</scenario>

<scenario id="CHK-006">
Given a tempdir spec with `<task id="T-004" state="in-progress">`
and a `journal/T-004.md` whose frontmatter is missing entirely
(malformed shape),
when `speccy check SPEC-NNNN` runs,
then no `JNL-*` diagnostic fires for T-004 (in-progress state
skips the family).
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Element attribute schemas for `<implementer>`, `<review>`, `<blockers>`

The three element kinds carry these required attributes (no
element has any optional attribute):

- `<implementer>`: `date`, `model`, `round`.
- `<review>`: `date`, `model`, `persona`, `verdict`, `round`.
- `<blockers>`: `date`, `round`.

The parser fires unknown-attribute errors on any attribute
outside these allow-lists for the named element. Missing
required attributes fire missing-attribute errors. The legacy
`session="..."` attribute is no longer in the allow-list for
`<implementer>`.

<done-when>
- The parser's `allowed_attrs` map (in
  `speccy-core/src/parse/task_xml/mod.rs` or its successor for
  journal files) declares: `implementer` → `["date", "model", "round"]`;
  `review` → `["date", "model", "persona", "verdict", "round"]`;
  `blockers` → `["date", "round"]`.
- Missing-required-attribute errors fire on any of the named
  attributes when the element is parsed without them.
- Unknown-attribute errors fire when an attribute outside the
  allow-list is present.
- The `verdict` attribute on `<review>` keeps its current closed
  value set: `{"pass", "blocking"}`.
- The `persona` attribute on `<review>` keeps its current
  closed value set per the existing persona registry (default:
  `{"business", "tests", "security", "style"}`).
- The `date` attribute on all three elements is validated as
  full ISO8601 date-time with seconds and timezone designator
  (a regex matching `YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)`).
- The `model` attribute on `<implementer>` and `<review>` is
  validated as a non-empty string. The lint does NOT validate
  any internal structure of the model string (slash-suffix for
  effort is a documented convention, not a parser-enforced
  schema).
- The `round` attribute on all three elements is validated as
  a positive integer (regex `[1-9][0-9]*`).
</done-when>

<behavior>
- Given a journal file containing
  `<implementer date="2026-05-21T18:00:00Z" model="claude-opus-4.7[1m]/low" round="1">...</implementer>`,
  when the parser reads it, then the element parses
  successfully with all three required attributes.
- Given a journal file containing
  `<implementer date="2026-05-21" model="..." round="1">...</implementer>`
  (date-only, not full timestamp), when the parser reads it,
  then a date-format validation error fires naming the
  expected ISO8601 timestamp format.
- Given a journal file containing
  `<implementer date="2026-05-21T18:00:00Z" round="1">...</implementer>`
  (missing `model`), when the parser reads it, then a
  missing-required-attribute error fires naming `model`.
- Given a journal file containing
  `<implementer date="..." model="..." round="1" session="legacy">...</implementer>`
  (legacy `session=` attribute), when the parser reads it,
  then an unknown-attribute error fires naming `session` and
  listing the allowed set as `["date", "model", "round"]`.
- Given a journal file containing
  `<blockers date="..." round="1">...</blockers>`,
  when the parser reads it, then the element parses
  successfully.
- Given a journal file containing
  `<review date="..." model="..." persona="tests" verdict="invalid" round="1">...</review>`,
  when the parser reads it, then an invalid-attribute-value
  error fires naming `verdict` and its allowed values.
</behavior>

<scenario id="CHK-007">
Given a tempdir spec with a `journal/T-001.md` containing
`<implementer date="2026-05-21T18:00:00Z" model="claude-opus-4.7[1m]" round="1">body</implementer>`,
when `speccy check SPEC-NNNN --json` runs,
then no parse-error diagnostic fires and the JSON envelope's
`lint_errors[]` contains no entries naming the implementer
element.
</scenario>

<scenario id="CHK-008">
Given a tempdir spec with a `journal/T-001.md` containing
`<implementer date="2026-05-21" model="claude-opus-4.7[1m]" round="1">body</implementer>`,
when `speccy check SPEC-NNNN --json` runs,
then exit is non-zero and the JSON envelope lists a parse
error naming `date` and the ISO8601-with-time format
requirement.
</scenario>

<scenario id="CHK-009">
Given a tempdir spec with a `journal/T-001.md` containing
`<implementer date="2026-05-21T18:00:00Z" model="" round="1">body</implementer>`
(empty `model`),
when `speccy check SPEC-NNNN --json` runs,
then exit is non-zero and the JSON envelope lists a parse
error naming `model` and the non-empty constraint.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `round` is a monotonic integer counter from 1 within a journal file

The `round` attribute on `<implementer>`, `<review>`, and
`<blockers>` elements counts implement/review loop turns
monotonically starting from 1 within a single `journal/T-NNN.md`
file. Round N's blocks (one implementer + N reviewers + at most
one blockers) appear in the file before round N+1's blocks. The
counter unifies what was previously encoded ad-hoc in the legacy
`session="...attempt-2"` / `session="...rev2"` strings.

<done-when>
- A new lint diagnostic fires (`JNL-003` covers this) on a
  journal file whose `round` attributes are non-monotonic
  (e.g., a `round="3"` block followed by a `round="2"` block,
  with no intervening reset).
- A lint diagnostic fires on a journal file whose round
  counter starts at a value other than 1 (e.g., the first
  `<implementer>` block has `round="2"`).
- A lint diagnostic fires on a journal file whose round
  counter skips values (e.g., `round="1"` blocks present,
  then `round="3"` blocks with no `round="2"` blocks
  between).
- The lint allows multiple blocks at the same round (one
  `<implementer round="N">` typically followed by multiple
  `<review round="N" persona="...">` blocks and at most one
  `<blockers round="N">` block).
- Round-skipping caused by a passing review (no blockers
  block for that round) is legal — round N's blocks may be
  followed directly by round N+1's `<implementer>` block
  without a `<blockers round="N">` block in between.
</done-when>

<behavior>
- Given a journal with `<implementer round="1">` followed by
  `<review round="1">` × 4 followed by `<implementer round="2">`,
  when the parser reads it, then no round-validation error
  fires.
- Given a journal whose first `<implementer>` element has
  `round="2"`, when the parser reads it, then a round-start
  error fires naming the first-round-must-be-1 rule.
- Given a journal with `<implementer round="1">`, `<review round="1">`,
  `<implementer round="3">` (skipping round 2), when the parser
  reads it, then a round-skip error fires naming the missing
  round 2.
- Given a journal with `<implementer round="2">` followed by
  `<implementer round="1">` (decreasing), when the parser
  reads it, then a non-monotonic-round error fires.
</behavior>

<scenario id="CHK-010">
Given a tempdir spec with a `journal/T-001.md` containing one
`<implementer round="1">` block then four `<review round="1">`
blocks then one `<implementer round="2">` block then four
`<review round="2">` blocks,
when `speccy check SPEC-NNNN` runs against a
`state="completed"` T-001,
then no `JNL-*` diagnostic fires on T-001.
</scenario>

<scenario id="CHK-011">
Given a tempdir spec with a `journal/T-001.md` whose first
`<implementer>` block has `round="2"`,
when `speccy verify` runs against a `state="completed"` T-001,
then exit is non-zero and the JSON envelope lists a `JNL-003`
diagnostic whose message names the first-round-must-be-1 rule.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Closed XML element set updated; legacy names retired and `<tasks>` wrapper dropped

The closed XML element set shrinks from six elements to five:
`task`, `task-scenarios`, `implementer`, `review`, `blockers`.
Three element-name changes land at the same cutover:

- `<implementer-note>` is removed from the allow-list (renamed
  to `<implementer>` per the journal artifact's context).
- `<retry>` is removed from the allow-list (renamed to
  `<blockers>` per the same context).
- `<tasks spec="SPEC-NNNN">` is removed from the allow-list
  entirely (no rename — the wrapper element earns nothing
  because filename `TASKS.md`, parent directory `NNNN-slug`,
  and frontmatter `spec:` field already encode the binding
  three times). TASKS.md after this SPEC contains bare
  `<task>` children directly under the `# Tasks:` heading.

<done-when>
- The parser's element-name allow-list contains exactly the
  five named elements (`task`, `task-scenarios`, `implementer`,
  `review`, `blockers`). Neither `<implementer-note>`, `<retry>`,
  nor `<tasks>` is in the allow-list.
- An unknown-element parse error fires when `<implementer-note>`,
  `<retry>`, or `<tasks>` is encountered anywhere in TASKS.md,
  in `journal/T-NNN.md`, or in any other speccy artifact.
- The TASKS.md parser accepts bare `<task>` children directly
  under the `# Tasks:` heading, with no wrapper element grouping
  them. The `spec` binding is resolved exclusively from the
  frontmatter `spec:` field plus the parent directory name; the
  wrapper's redundant `spec="SPEC-NNNN"` attribute no longer
  participates in binding resolution.
- ARCHITECTURE.md's closed-element-set documentation reflects
  the renamed names and the shrunken cardinality (five, not six).
</done-when>

<behavior>
- Given a TASKS.md containing `<implementer-note session="...">body</implementer-note>`
  (legacy form), when the parser reads it, then an
  unknown-element error fires naming `implementer-note` and
  the closed allow-list.
- Given a TASKS.md containing `<retry>body</retry>` (legacy
  form), when the parser reads it, then an unknown-element
  error fires naming `retry` and the closed allow-list.
- Given a TASKS.md containing `<tasks spec="SPEC-NNNN">...</tasks>`
  (legacy wrapper), when the parser reads it, then an
  unknown-element error fires naming `tasks` and the closed
  allow-list.
- Given a TASKS.md whose body starts with `# Tasks: SPEC-NNNN ...`
  followed immediately by bare `<task id="T-001" ...>...</task>`
  elements (no wrapper), when the parser reads it, then the
  file parses successfully and each `<task>` child is recognized.
- Given a `journal/T-001.md` containing a properly-named
  `<implementer date="..." model="..." round="1">body</implementer>`
  block, when the parser reads it, then the element parses
  successfully.
</behavior>

<scenario id="CHK-012">
Given a tempdir workspace with a SPEC's TASKS.md containing
`<implementer-note session="legacy">...</implementer-note>`,
when `speccy check SPEC-NNNN --json` runs,
then exit is non-zero and the JSON envelope lists a parse
error naming `implementer-note` as an unknown element.
</scenario>

<scenario id="CHK-013">
Given the same workspace whose TASKS.md instead contains
`<retry>...</retry>`,
when `speccy check SPEC-NNNN --json` runs,
then exit is non-zero and the JSON envelope lists a parse
error naming `retry` as an unknown element.
</scenario>

<scenario id="CHK-025">
Given the same workspace whose TASKS.md instead wraps its
`<task>` elements in `<tasks spec="SPEC-NNNN">...</tasks>`,
when `speccy check SPEC-NNNN --json` runs,
then exit is non-zero and the JSON envelope lists a parse
error naming `tasks` as an unknown element.
</scenario>

<scenario id="CHK-026">
Given a tempdir workspace with a SPEC's TASKS.md whose body
contains bare `<task id="T-001" state="pending" covers="REQ-001">...</task>`
children directly under the `# Tasks: SPEC-NNNN ...` heading
(no `<tasks>` wrapper, no `spec="..."` attribute anywhere
in the body — the `spec` binding comes from frontmatter +
parent directory only),
when `speccy check SPEC-NNNN --json` runs,
then exit is 0 with no parse-error diagnostic, and the JSON
envelope's task inventory correctly lists the parsed `<task>`
elements with their state/covers attributes.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: TASKS.md unconditionally rejects `<implementer>`, `<review>`, `<blockers>`

A new lint rule (a TSK family addition) fires unconditionally
when any `<implementer>`, `<review>`, or `<blockers>` element
appears in a TASKS.md file. The rule is NOT lifecycle-gated by
task state — it fires at any task state, including
`in-progress` and `in-review` (unlike the `JNL-*` family,
which skips those states for the journal file). Net-new
`<task>` elements appended to TASKS.md mid-implementation
remain legal per REQ-007 below; only the three activity-prose
element kinds are rejected.

<done-when>
- A new lint diagnostic in the TSK family fires (severity
  error) on any `<implementer>`, `<review>`, or `<blockers>`
  element parsed inside a TASKS.md file, regardless of the
  containing task's state attribute.
- The diagnostic message names which element appeared, the
  task containing it, and the canonical fix (move to
  `journal/T-NNN.md`).
- `speccy verify` exits non-zero on this diagnostic.
- The diagnostic fires before any `JNL-*` diagnostic on the
  same task — a TASKS.md violation is more fundamental than a
  journal-file shape issue.
</done-when>

<behavior>
- Given a TASKS.md with `<task id="T-001" state="completed">`
  containing an `<implementer date="..." model="..." round="1">...</implementer>`
  block inside the `<task>` body, when `speccy check SPEC-NNNN`
  runs, then a TSK-family diagnostic fires naming the
  misplaced `<implementer>` element and naming the canonical
  fix (move to `journal/T-001.md`).
- Given a TASKS.md with `<task id="T-002" state="in-progress">`
  containing a `<review persona="tests" verdict="pass" ...>...</review>`
  block, when `speccy check SPEC-NNNN` runs, then a
  TSK-family diagnostic fires (the rule is not state-gated).
- Given a TASKS.md with `<task id="T-003" state="pending">`
  containing a `<blockers round="1">...</blockers>` block,
  when `speccy check SPEC-NNNN` runs, then a TSK-family
  diagnostic fires.
</behavior>

<scenario id="CHK-014">
Given a tempdir spec with a TASKS.md containing
`<task id="T-001" state="completed"><implementer date="2026-05-21T18:00:00Z" model="..." round="1">body</implementer></task>`,
when `speccy verify` runs,
then exit is non-zero and the JSON envelope lists a TSK-family
diagnostic whose message names `<implementer>` as a misplaced
element with the canonical fix pointing at `journal/T-001.md`.
</scenario>

<scenario id="CHK-015">
Given the same workspace whose TASKS.md instead contains
`<task id="T-002" state="in-progress"><review persona="tests" verdict="pass" date="..." model="..." round="1">body</review></task>`,
when `speccy verify` runs,
then exit is non-zero and the same TSK-family diagnostic
fires (the rule is not state-gated to in-progress).
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: TASKS.md continues to accept net-new `<task>` elements mid-implementation

TASKS.md's append-allowed surface stays narrow: only net-new
`<task>` elements (added when scope gaps are discovered
mid-implementation) and `state="..."` attribute mutations on
existing `<task>` elements are legitimate mid-loop mutations.
The lint does NOT fire on a TASKS.md that gains a new
`<task>` element after decomposition. The lint does NOT fire
on `state="..."` attribute mutations across the full lifecycle
(`pending` → `in-progress` → `in-review` → `completed`, and
`completed` → `pending` via amendment).

<done-when>
- No lint diagnostic fires when a TASKS.md gains a net-new
  `<task id="T-NNN">` element appended mid-implementation
  (i.e., after the `spec_hash_at_generation` recording).
- No lint diagnostic fires on `state="..."` attribute mutations
  in any direction within the closed-value-set.
- The `spec_hash_at_generation` mechanism continues to work
  unchanged — net-new task appends do NOT trigger TSK-003
  staleness (the hash is over SPEC.md content, not TASKS.md
  content).
- Documentation in ARCHITECTURE.md and AGENTS.md notes the
  TASKS.md-append rule explicitly: state mutations + net-new
  task appends are the only legitimate mid-loop mutations.
</done-when>

<behavior>
- Given a TASKS.md with three tasks decomposed (all
  `state="pending"`), when an implementer working on T-002
  discovers a scope gap and appends a new `<task id="T-004"
  state="pending" covers="REQ-003">...</task>` element, then
  `speccy check SPEC-NNNN` reports no new lint error for the
  append.
- Given a TASKS.md with T-001 at `state="completed"`, when
  `/speccy-amend` flips T-001 back to `state="pending"`, then
  no lint diagnostic fires on the state mutation itself.
</behavior>

<scenario id="CHK-016">
Given a tempdir spec whose TASKS.md initially contains three
tasks, all `state="pending"`, with a recorded
`spec_hash_at_generation`,
when a fourth `<task id="T-004" state="pending" covers="REQ-NNN">body</task>`
element is appended to the file,
and `speccy check SPEC-NNNN` runs,
then the new task is parsed cleanly, no parse error fires on
the append, and no TSK-003 staleness diagnostic fires on the
SPEC hash.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Skill bodies updated to read/write journal files

Three skill bodies update to read and write per-task journal
files instead of appending activity-prose elements into
TASKS.md. The updates land in the canonical authoring surface
at `resources/modules/skills/` and `resources/modules/phases/`
(per the SPEC-0033 architecture); they auto-eject into both
`.claude/skills/` (Claude Code) and `.agents/skills/` (Codex)
host packs via the `speccy init` pipeline.

<done-when>
- `resources/modules/phases/speccy-work.md` instructs the
  implementer to append an `<implementer>` block to
  `.speccy/specs/NNNN-slug/journal/T-NNN.md` (creating the
  file with frontmatter if it does not yet exist). The
  instruction names the required attributes (`date`, `model`,
  `round`) and the slash-suffix convention for encoding
  effort on `model`.
- `resources/modules/skills/speccy-review.md` instructs the
  orchestrator to append each persona's `<review>` block to
  `journal/T-NNN.md` (still serially per DEC-008, not in
  parallel from reviewer subagents). The instruction names
  the required attributes (`date`, `model`, `persona`,
  `verdict`, `round`).
- `resources/modules/skills/speccy-amend.md` instructs the
  amend flow to append a `<blockers round="N+1" date="...">spec amended; ...</blockers>`
  block to `journal/T-NNN.md` when flipping a completed task
  back to `state="pending"` due to a SPEC change, instead of
  appending the legacy `<retry>` element inside the `<task>`
  body in TASKS.md. The current language about appending the
  block "inside the `<task>` body" (in `speccy-amend.md`
  lines 106-111 today) updates to name the journal file as
  the destination.
- Each instruction is mirrored into the ejected SKILL.md
  files at `speccy init` time without manual edits to the
  host-pack files.
</done-when>

<behavior>
- Given a freshly-initialized workspace
  (`speccy init --host claude-code` run after this SPEC
  lands), when an agent reads
  `.claude/skills/speccy-work/SKILL.md`, then the body names
  `journal/T-NNN.md` as the write target for
  `<implementer>` blocks.
- Given the same workspace, when an agent reads
  `.claude/skills/speccy-review/SKILL.md`, then the body
  names `journal/T-NNN.md` as the write target for
  `<review>` blocks (per persona) and instructs serial
  orchestrator-side writes.
- Given the same workspace, when an agent reads
  `.claude/skills/speccy-amend/SKILL.md`, then the body
  names `journal/T-NNN.md` as the write target for
  amendment-driven `<blockers>` blocks (replacing the
  current `<task>`-body destination).
</behavior>

<scenario id="CHK-017">
Given the post-SPEC source tree,
when `resources/modules/phases/speccy-work.md` is read,
then its body contains a reference to `journal/T-NNN.md` as
the write target for `<implementer>` blocks, and contains no
instruction to append `<implementer-note>` to TASKS.md.
</scenario>

<scenario id="CHK-018">
Given the post-SPEC source tree,
when `resources/modules/skills/speccy-review.md` is read,
then its body contains a reference to `journal/T-NNN.md` as
the write target for `<review>` blocks, and the legacy
references to appending into the `<task>` body in TASKS.md
are absent.
</scenario>

<scenario id="CHK-019">
Given the post-SPEC source tree,
when `resources/modules/skills/speccy-amend.md` is read,
then its body's amend-flow step (currently lines 106-111
referencing "appended inside the `<task>` body") names
`journal/T-NNN.md` as the destination for amendment-driven
`<blockers>` blocks.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: Reviewer return contract requires `model` in returned `<review>`

`resources/modules/personas/verdict_return_contract.md`
updates so each reviewer persona's returned `<review>` element
carries its own `model` attribute (with optional slash-suffix
for effort). The orchestrator transcribes the model verbatim
into the journal file. Without this, the orchestrator cannot
reliably record per-reviewer model identity across
heterogeneous reviewer subagents that may pin different model
tiers per persona.

<done-when>
- `resources/modules/personas/verdict_return_contract.md`
  names `model` as a required attribute on the returned
  `<review>` element from each reviewer persona.
- The contract explicitly names the slash-suffix encoding for
  effort (e.g. `model="claude-opus-4.7[1m]/low"`) so
  reviewer subagents that ran with an explicit effort knob
  encode it consistently.
- The orchestrator-side flow in `speccy-review.md`
  transcribes the model attribute verbatim from each
  subagent's returned `<review>` into the journal entry — no
  guessing, no inference from skill-pack identity alone.
- The implementer side of the contract does NOT change: the
  implementer writes its own journal entry directly with the
  model it knows; there is no orchestrator middleman for
  implementer turns, so no analogous return-contract
  requirement applies.
</done-when>

<behavior>
- Given a reviewer subagent (e.g. `reviewer-tests`) returning
  `<review persona="tests" verdict="blocking" model="claude-opus-4.7[1m]/medium">body</review>`,
  when the orchestrator transcribes this into the journal,
  then the resulting journal block carries
  `model="claude-opus-4.7[1m]/medium"` verbatim.
- Given a reviewer subagent returning a `<review>` element
  with no `model` attribute, when the orchestrator validates
  the return shape, then the orchestrator surfaces the
  contract violation (subagent did not declare its model) and
  does not transcribe an invented model value into the
  journal.
</behavior>

<scenario id="CHK-020">
Given the post-SPEC source tree,
when `resources/modules/personas/verdict_return_contract.md`
is read,
then its body contains a requirement that the returned
`<review>` element carries a `model` attribute.
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: Historical rewrite of SPEC-0001 through SPEC-0037

Every existing in-tree spec from SPEC-0001 through SPEC-0037
(inclusive of SPEC-0037 itself) gets its legacy
`<implementer-note session="...">`, `<review persona="..." verdict="...">`,
and `<retry>` elements migrated from TASKS.md into per-task
`journal/T-NNN.md` files under the new schema, AND has its
redundant `<tasks spec="SPEC-NNNN">...</tasks>` wrapper stripped
so `<task>` children sit bare under the `# Tasks:` heading.
Migration is performed manually by LLM implementer agents (no
migration script, no `speccy migrate` subcommand).

SPEC-0037 is included in scope because the implementer + reviewer
agents working on T-001 through T-006 follow the un-updated skill
bodies and write activity prose in the legacy format into
SPEC-0037's own TASKS.md. The cutover at T-001 immediately makes
that format unparseable, leaving SPEC-0037's TASKS.md in the
same broken state as every historical spec. T-007 sweeps all 37
specs uniformly to restore a green workspace. The implementer of
T-007 itself writes its own activity record directly to
`.speccy/specs/0037-task-journal-files/journal/T-007.md` in the
new format (by then the new schema is the only valid format),
not into TASKS.md.

The `JNL-*` lint family plus the TASKS.md "no notes elements"
rule from REQ-006 together form the migration's correctness
floor: a lint-clean rewrite is the acceptance criterion.

<done-when>
- After F-11 ships, no TASKS.md in `.speccy/specs/0001-*/`
  through `.speccy/specs/0037-*/` (inclusive of SPEC-0037)
  contains any `<implementer-note>`, `<implementer>`,
  `<review>`, `<retry>`, `<blockers>`, or `<tasks>` element.
  The `<tasks spec="...">` wrapper is stripped from every
  TASKS.md as part of the migration; the `<task>` children sit
  bare under the `# Tasks:` heading post-migration.
- After F-11 ships, every `state="completed"` task across
  SPEC-0001 through SPEC-0037 has a corresponding
  `journal/T-NNN.md` file with a valid frontmatter
  (`spec:`, `task:`, `generated_at:`) and well-formed
  `<implementer>` / `<review>` / `<blockers>` elements
  conforming to REQ-003's attribute schema.
- After F-11 ships, `speccy verify` exits 0 for every spec
  in `.speccy/specs/0001-*/` through
  `.speccy/specs/0037-*/`.
- The migration touches only the activity-prose ejection plus
  the wrapper strip. Each historical SPEC.md, REPORT.md,
  evidence/, and the non-activity parts of TASKS.md
  (frontmatter, task definitions, `<task-scenarios>` bodies,
  state attributes) stay byte-identical to the pre-F-11 state
  for SPEC-0001 through SPEC-0036. For SPEC-0037 itself, the
  migration touches TASKS.md to eject T-001 through T-006
  activity prose (the `<tasks spec="SPEC-0037">` wrapper was
  already stripped by the 2026-05-21 decomposition-revision
  orchestrator edit); SPEC.md, evidence/, and other surfaces
  stay untouched by T-007.
- The migration is performed by LLM implementer agents as
  part of this SPEC's `TASKS.md` decomposition — task
  granularity is a single migration task per the user
  directive captured in this SPEC's amendment (one
  implementer + reviewer turn covering all 37 specs, with
  multiple retry rounds expected; see open question a below).
- `date` and `model` attributes on migrated elements are
  inferred from the legacy `session="..."` strings and git
  blame where possible. When inference is ambiguous, the
  migrating agent picks a best-effort value and notes the
  inference assumption in its implementer journal entry on
  the migration task itself (recursive use of the new
  artifact).
</done-when>

<behavior>
- Given the post-F-11 source tree, when
  `.speccy/specs/0035-report-md-proof-shape-lint/TASKS.md`
  is read, then it contains no `<implementer-note>`,
  `<implementer>`, `<review>`, `<retry>`, `<blockers>`,
  or `<tasks>` element. Its `<task>` children sit bare under
  the `# Tasks:` heading.
- Given the post-F-11 source tree, when
  `.speccy/specs/0035-report-md-proof-shape-lint/journal/T-001.md`
  is read, then it contains a valid frontmatter and at least
  one well-formed `<implementer>` block plus the
  `<review>` blocks that previously lived inside the T-001
  `<task>` body in TASKS.md.
- Given the post-F-11 source tree, when
  `.speccy/specs/0037-task-journal-files/TASKS.md` is read,
  then it contains no `<implementer-note>`, `<implementer>`,
  `<review>`, `<retry>`, `<blockers>`, or `<tasks>` element,
  and its `<task>` children sit bare under the `# Tasks:`
  heading (SPEC-0037 dogfoods the new shape post-migration).
- Given the post-F-11 source tree, when
  `.speccy/specs/0037-task-journal-files/journal/T-001.md`
  through `T-007.md` are read, then each carries valid
  frontmatter and well-formed activity blocks per REQ-003.
- Given the post-F-11 source tree, when
  `speccy verify` runs across the workspace (or
  spec-by-spec), then it exits 0 for every spec from
  SPEC-0001 through SPEC-0037.
</behavior>

<scenario id="CHK-021">
Given the post-F-11 source tree,
when each TASKS.md file under
`.speccy/specs/000[1-9]-*/`, `.speccy/specs/00[1-3][0-6]-*/`,
and `.speccy/specs/0037-task-journal-files/`
is searched for the literal substrings
`<implementer-note`, `<implementer `, `<review `, `<retry`,
`<blockers`, and `<tasks`,
then no matches appear in any of the 37 files.
</scenario>

<scenario id="CHK-027">
Given the post-F-11 source tree,
when each TASKS.md file under
`.speccy/specs/000[1-9]-*/`, `.speccy/specs/00[1-3][0-6]-*/`,
and `.speccy/specs/0037-task-journal-files/`
is parsed,
then exactly one `# Tasks:` heading appears followed
immediately by one-or-more bare `<task>` children with no
intervening wrapper element (37 files total).
</scenario>

<scenario id="CHK-022">
Given the post-F-11 source tree,
when `speccy verify` runs (or its per-spec equivalent
loops across SPEC-0001 through SPEC-0037),
then exit is 0 for every spec.
</scenario>

</requirement>

<requirement id="REQ-011">
### REQ-011: ARCHITECTURE.md and AGENTS.md document the new layout and schemas

`docs/ARCHITECTURE.md` and `AGENTS.md` (the canonical
spec-workspace documentation surfaces) get updates documenting
the new `journal/` artifact, the renamed element names, the
attribute schemas, the `JNL-*` lint family, and the TASKS.md
"no notes elements" rule. The updates are the authoritative
reference for future contributors and downstream skill packs.

<done-when>
- `docs/ARCHITECTURE.md` contains a section describing the
  `journal/T-NNN.md` artifact: its location, the
  frontmatter shape, the bare-element body shape, and the
  binding rules (filename ↔ task, frontmatter ↔ spec).
- `docs/ARCHITECTURE.md` updates the closed-element-set
  reference to name `implementer` and `blockers` in place of
  the legacy `implementer-note` and `retry`, and drops `tasks`
  from the set entirely. The set cardinality is documented as
  five (down from six).
- `docs/ARCHITECTURE.md` documents the TASKS.md structural
  shape change: bare `<task>` children directly under the
  `# Tasks:` heading; binding is resolved from filename +
  parent directory + frontmatter `spec:`, not from a wrapper
  attribute.
- `docs/ARCHITECTURE.md` documents the `JNL-001`,
  `JNL-002`, `JNL-003` lint codes with their state-gating
  rules.
- `docs/ARCHITECTURE.md` documents the TASKS.md
  "no notes elements" rule (REQ-006) as a TSK-family
  addition.
- `AGENTS.md` references the journal artifact location in
  any prose that discusses the implement/review loop or
  agent activity records.
- No outdated references to the legacy element names
  (`<implementer-note>`, `<retry>`) remain in either file
  outside historical/changelog contexts.
</done-when>

<behavior>
- Given the post-SPEC `docs/ARCHITECTURE.md`, when the
  file is searched for `journal/T-NNN.md`, then the path
  appears at least once as a documented artifact.
- Given the post-SPEC `docs/ARCHITECTURE.md`, when the
  file is searched for `JNL-001`, `JNL-002`, `JNL-003`,
  then each code appears in the lint-family reference
  section.
- Given the post-SPEC `docs/ARCHITECTURE.md`, when the
  file is searched for `<implementer-note>` or `<retry>` in
  the live-workflow prose (excluding any historical
  changelog rows), then no matches appear.
</behavior>

<scenario id="CHK-023">
Given the post-SPEC `docs/ARCHITECTURE.md`,
when the file body is searched for the literal substrings
`journal/T-NNN.md` and `JNL-001`,
then both substrings appear in the file body.
</scenario>

<scenario id="CHK-024">
Given the post-SPEC `docs/ARCHITECTURE.md`,
when the file body is searched for the literal substrings
`<implementer-note>` and `<retry>` outside any
historical-context or changelog section,
then no matches appear in the live-workflow prose.
</scenario>

</requirement>

## Design

### Decisions

<decision id="DEC-001">
**Per-task journal files, not a single per-spec journal or per-attempt sharding.**

Three file-layout shapes were considered during brainstorm:
(a) one `JOURNAL.md` per spec holding all tasks' activity in
one file, grouped by task ID; (b) one `journal/T-NNN.md` per
task; (c) one file per task per retry attempt
(`journal/T-NNN/attempt-1.md`, `attempt-2.md`, ...).

(a) rejected: the dominant read pattern is single-task
("show me T-005's history"), not cross-task. A per-spec
journal forces every implementer and reviewer read to pay
the full-spec context cost, which is exactly the
token-economics problem F-11 aims to solve.

(c) rejected: optimizes for the failure mode (high retry
counts) Speccy would rather not normalize. The right answer
to "T-005 keeps blocking on retry" is `/speccy-amend`, not
sharding the journal into more files. Per-attempt files also
force `speccy-ship` to walk a directory tree to summarize
each task — added complexity for negligible benefit.

(b) wins: dominant read pattern is single-task, so per-task
minimizes context per read; mirrors the existing
`evidence/T-NNN.md` pattern (same parent directory cousin);
no new race surface (the reviewer fan-out is already
serialized through the orchestrator per DEC-008); growth is
bounded by per-task retry count, which is itself a
spec-quality signal that should trip amendment rather than
sharding.
</decision>

<decision id="DEC-002">
**Filename + frontmatter binds the journal to its task; no wrapper element.**

An earlier draft of the journal file shape used a
`<task-journal id="T-NNN">` wrapper element to make the
file→task binding machine-readable. Rejected.

The filename `T-NNN.md` already encodes the task ID. The
frontmatter `task:` field re-states it. A third binding via
a wrapper element would only catch the case of someone
manually editing the wrapper's `id` attribute to disagree
with the filename and frontmatter — a vanishingly rare
failure mode.

TASKS.md uses a `<tasks spec="...">` wrapper because it
holds multiple `<task>` children — the wrapper is the
grouping mechanism. A per-task journal file holds one
task's content; no grouping needed. The relevant precedent
is `evidence/T-NNN.md`, which uses no wrapper element. Per-
task journal files follow the same pattern.

Closed XML element set therefore stays at six (no new
wrapper noun). Stay-small wins.
</decision>

<decision id="DEC-003">
**`effort` is encoded as a slash-suffix on `model`; not a separate attribute.**

Two encodings were considered: (a) `effort` as a separate
optional attribute on `<implementer>` and `<review>`
(closed set `{low, medium, high}`); (b) `effort` as a slash-
suffix on `model` (e.g. `model="claude-opus-4.7[1m]/low"`).

(a) keeps each attribute one-thing-only and lets the lint
validate the effort value's closed-set membership directly.
But not every host harness exposes an effort knob; making
effort optional forces a parser branch for "present/absent"
and a documentation branch for "when to record / when to
omit", and reviewer subagents that don't have an effort knob
have to remember to leave the attribute off.

(b) collapses all of that into a single string field. The
slash-suffix is documented in skill prose as a convention,
not parser-enforced — the lint validates `model` is
non-empty only. Hosts without an effort knob just omit the
suffix; hosts with one append it. No optional attribute
surface to maintain; no extra schema variant for the lint
to handle.

The trade-off is loss of closed-set validation on the
effort value. We accept this: the model string is already
free-form (model names release frequently), so the lint is
opaque to its internal structure anyway. If a future
amendment wants to tighten the suffix to a closed set, the
parser can grow that validation without changing the
attribute shape.
</decision>

<decision id="DEC-004">
**`JNL-*` lint is lifecycle-aware, per-task state-gated.**

Three lint-activation models were considered: (a) always
run — fire on any malformed/missing journal regardless of
task state; (b) verify-only — `JNL-*` runs only inside
`speccy verify`, never during `speccy check` or
`speccy status`; (c) lifecycle-aware per-task — gate each
lint by the corresponding task's `state` attribute.

(a) rejected: fires noise mid-loop on tasks the implementer
is actively working. Pending tasks would correctly trip
JNL-001, but in-progress and in-review tasks would
constantly fire spurious "missing journal" or "malformed
journal" diagnostics on partial state that's expected to be
incomplete. The lint would have to be ignored mid-loop —
which is exactly what users do today with noisy linters,
defeating the purpose.

(b) rejected: hides real problems until pre-ship. If a
pending task has an orphan journal file from a prior
amend-flip, the implementer should know now, not at
`speccy verify` time after they've spent a session reading
the orphan thinking it's still relevant.

(c) wins: pending tasks demand clean slate (JNL-001
catches orphans); in-progress and in-review tasks skip
entirely (no noise mid-loop); completed tasks demand
well-formed journals (JNL-002, JNL-003 are the pre-ship
gate). The state machine handles the activation; no
verify-vs-check split is needed.

The activation rule lives in the lint runner, not in the
individual lint codes. Each `JNL-*` code checks the
condition; the runner decides whether to skip based on
state. Simpler code than three lint codes each carrying
state-aware logic.
</decision>

<decision id="DEC-005">
**No `task_hash_at_generation` field; state machine handles drift.**

An earlier brainstorm draft included a
`task_hash_at_generation` field in the journal frontmatter
(mirroring TASKS.md's `spec_hash_at_generation` pattern).
The hash would catch the amendment-mid-task case:
implementer claims T-005, amend rewrites T-005's
`<task-scenarios>` mid-flight, hash mismatch alerts the
implementer on their next journal write.

Rejected. The use case is real but rare, and the existing
amend flow (`speccy-amend.md` lines 106-111) already flips
affected `state="completed"` tasks back to `state="pending"`
when SPEC changes invalidate their completion. The next
implementer claim on a pending task is necessarily a fresh
read of the task subtree from TASKS.md. The state machine
forces the same re-read the hash would have forced — just
at the natural transition point, not at every journal
write.

For the in-flight case (T-005 is `in-progress` when amend
runs), the amend flow's existing behavior leaves the task
in `in-progress` state, and the implementer is expected to
re-orient when they next read TASKS.md. This is skill-body
discipline, not lint enforcement — consistent with Speccy's
core principle 1 (feedback, not enforcement).

The hash field would add: one frontmatter field on every
journal file (~360 files post-migration); one
canonicalization rule (must exclude `state` attribute or
every state flip fires staleness); one new lint code; one
extra hash recompute per `speccy check` invocation. All
cost; the state-machine path provides equivalent feedback
for zero additional surface.
</decision>

<decision id="DEC-006">
**Migration is manual via LLM; no script, no CLI subcommand.**

The historical rewrite of SPEC-0001 through SPEC-0036
covers ~36 specs × ~10 tasks/spec ≈ 360 journal files to
create. The intuitive automation is a migration script
(regex-based, or a parser-aware Rust tool) that walks each
TASKS.md, extracts the activity elements, infers
attributes, and emits journal files.

Rejected for two reasons. First, legacy `session="..."`
strings encode date and attempt-number inconsistently
across the 36 specs — `session="session-2026-05-20-t001-attempt-1"`,
`session="spec0035-t001-rev2"`, and other variants exist.
Faithful migration requires interpretive judgment a script
can't reliably do. A script that invents `date="unknown"`
or fakes `model="claude-opus-4.7[1m]"` (when we don't
actually know which model generated SPEC-0003's notes)
pollutes the forensic trail worse than leaving the legacy
in place. Second, a `speccy migrate` CLI subcommand would
walk back the stay-small principle for a one-time
operation.

The LLM-driven approach: each migration task in F-11's
TASKS.md handles one spec (or a small batch of specs); the
LLM implementer reads the legacy TASKS.md, infers
attributes from session strings + git blame + context,
writes the journal files, and validates against the
`JNL-*` lint family. A lint-clean rewrite is the
acceptance criterion (assumption A4). The LLM also notes
inference assumptions in its own implementer journal entry
on the migration task — recursive use of the new artifact.
</decision>

<decision id="DEC-008">
**Drop the `<tasks spec="SPEC-NNNN">` wrapper from TASKS.md.**

The wrapper element encodes the `spec` binding via its
`spec="SPEC-NNNN"` attribute. That binding is already encoded by
three other surfaces: the filename `TASKS.md`, the parent
directory `NNNN-slug`, and the frontmatter `spec:` field. A
fourth restating of the same fact earns nothing — it just creates
one more place the SPEC ID can disagree with itself, and one
more line of boilerplate the decomposer has to author for every
new spec.

Three alternatives considered:

(a) Keep the wrapper as-is. Status quo; carries the cost without
the benefit. Rejected as obvious-redundancy by user inspection.

(b) Keep the wrapper but drop the `spec="..."` attribute. Halfway
measure — still wraps every `<task>` child for no semantic gain.
The wrapper-vs-no-wrapper noise (extra `<tasks>` / `</tasks>`
lines, indentation question for `<task>` children) outweighs the
"it's already there, leave it" argument.

(c) Drop the wrapper entirely. Bare `<task>` children sit
directly under the `# Tasks: SPEC-NNNN ...` heading. Binding is
resolved from filename + parent directory + frontmatter alone.
This pairs symmetrically with DEC-002's journal-file decision
(no `<task-journal>` wrapper around journal entries for the
same reason — the filename and frontmatter bind it). Wins.

The closed XML element set therefore shrinks from six to five.
The migration in REQ-010 strips the wrapper from every
historical TASKS.md alongside the activity-prose ejection, in
the same pass (no extra task; just additional lint compliance
the migration agent must satisfy).

This decision came in mid-decomposition for SPEC-0037 itself,
not during the initial brainstorm. It is captured here for
posterity because the rationale is durable — the wrapper is
gone and won't be re-added.
</decision>

<decision id="DEC-007">
**`<implementer-note>` → `<implementer>` and `<retry>` → `<blockers>` renames.**

Two renames apply to the closed XML element set as part of
F-11. The motivation: self-documenting element names in a
context where the file location already disambiguates.

`<implementer-note>` → `<implementer>`: the file
`journal/T-NNN.md` is itself a journal; everything inside
is a journal entry. The "-note" suffix is redundant once
the file location does that work. The implementer is the
actor; `<implementer>` reads naturally as "the
implementer's entry", same way `<review>` reads as "a
review" (noun-as-artifact).

`<retry>` → `<blockers>`: the element holds the
orchestrator's aggregated set of blocker directives from
any `verdict="blocking"` reviewers in the round. The body
is typically a list of per-persona blocker summaries.
`<blockers>` describes the content (a list of blockers,
plural); `<retry>` describes the consequence (the
implementer will retry). Naming the content rather than
the consequence is more self-documenting. The vocabulary
also pairs with the existing `verdict="blocking"` language
in `<review>` — linguistic chain: blocking reviews →
`<blockers>` aggregation.

Renaming is total — no alias period. After F-11 ships, the
legacy names parse nowhere. The 36 historical specs get
the rename as part of REQ-010's migration.
</decision>

## Assumptions

<assumptions>
- The journal file format (markdown YAML frontmatter +
  embedded XML element blocks below) is the right shape.
  No alternative format (TOML, JSON sidecar, plain text
  logs) is considered — the markdown+XML pattern matches
  TASKS.md and the existing speccy artifact shape. Reviewer
  and implementer agents already author this shape today;
  moving the content to a new file preserves the authoring
  shape and minimizes skill-template churn.
- Reviewer fan-out writes continue to serialize through the
  orchestrator (existing DEC-008 pattern from
  `speccy-review.md`). Per-task journal files do NOT
  introduce parallel writes from reviewer subagents
  directly to the file. The orchestrator collects each
  subagent's returned `<review>` block and writes the
  full set in one transaction per round. This SPEC does
  not weaken or change the existing concurrency contract.
- Existing SPEC-0001 through SPEC-0036 contain only
  `<review>` blocks whose `verdict` values are in the
  closed set `{pass, blocking}`, and contain no other
  unrepresentable legacy attributes that the new schema
  can't accommodate after rename and date inference. A spot
  check at decomposition time would confirm before
  decomposing migration tasks.
- The migration's correctness floor is lint-cleanliness
  alone. An LLM implementer that produces a journal file
  passing all `JNL-*` lints (plus TASKS.md passing the new
  TSK-family "no notes elements" rule) is trusted to have
  preserved legacy content faithfully. No human prose-review
  per migrated spec is required beyond that. If this turns
  out wrong (the LLM produces lint-clean but semantically
  divergent migrations), F-11's task count balloons and
  the reviewer fan-out catches it via the reviewer-tests
  persona reading the migrated journal against the legacy
  TASKS.md.
- The CLI gets no new subcommand (`speccy migrate`,
  `speccy convert`, or similar). Migration is pure content
  rewrite tasked to LLM agents using existing
  `speccy check` / `speccy verify` as the feedback loop.
  This keeps the seven-verb CLI surface stable per
  AGENTS.md.
- Post-F-11, agents reading any in-tree spec see only the
  new format. No spec retains legacy notes inside TASKS.md.
  The parser drops the legacy `<implementer-note>` /
  `<retry>` element names from its allow-list, and also drops
  the redundant `<tasks>` wrapper; all three forms fail to
  parse anywhere.
- The `model` attribute is a free-form string. The lint
  validates non-empty only; it does not enforce a closed
  set of model names (model names release frequently and a
  closed-set lint would force a CLI release per new model).
  The slash-suffix for effort (e.g.
  `model="claude-opus-4.7[1m]/low"`) is a documented
  convention, not parser-enforced.
- The `date` attribute is full ISO8601 date-time with
  seconds and a timezone designator (Z or ±HH:MM).
  Within-day ordering is recoverable from the timestamp
  itself, not from element position in the file. The same
  format applies to the `generated_at` field in journal
  frontmatter.
- The amend skill (`speccy-amend.md`) already flips
  affected `state="completed"` tasks back to
  `state="pending"` on SPEC change. F-11 inherits this
  behavior; the only change is REQ-008's third sub-bullet
  (write the amend-driven `<blockers>` block to the journal
  instead of into the `<task>` body in TASKS.md). The
  state-flip semantics themselves are unchanged.
- SPEC-0036 (Repin Claude Code speccy-work implementer to
  opus[1m] / low effort) ships before F-11 begins
  implementation. F-11's migration scope therefore covers
  SPEC-0001 through SPEC-0036 inclusive; SPEC-0036's own
  TASKS.md gets the same activity-prose ejection as its
  predecessors. If SPEC-0036 is still in-progress when
  F-11 lands, a small carve-out at decomposition time
  handles the in-flight specs.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-21 | human/kevin + Claude | Initial draft. Ejects `<implementer-note>` (renamed to `<implementer>`), `<review>`, and `<retry>` (renamed to `<blockers>`) elements out of TASKS.md and into per-task `journal/T-NNN.md` files under `.speccy/specs/NNNN-slug/journal/`. Filename + YAML frontmatter (`spec:`, `task:`, `generated_at:`) bind the file to its task and spec; no wrapper element. Element schemas pick up traceability attributes: `<implementer>` carries `date`, `model`, `round`; `<review>` carries `date`, `model`, `persona`, `verdict`, `round`; `<blockers>` carries `date`, `round`. All required, no optional attributes. `round` is monotonic integer from 1 unifying ad-hoc legacy `session="...attempt-N"` / `session="...rev-N"` encoding. `model` is free-form; effort/reasoning-intensity rides as slash-suffix (`claude-opus-4.7[1m]/low`). `date` is full ISO8601 with seconds + timezone. New `JNL-*` lint family validates journal shape lifecycle-aware per-task: `JNL-001` (pending must not have journal), `JNL-002` (completed must have journal), `JNL-003` (completed journal must be well-formed); in-progress and in-review states skip. TASKS.md gains an unconditional TSK-family rule forbidding the three activity elements at any state. Three skill bodies update (`speccy-work.md`, `speccy-review.md`, `speccy-amend.md`) to read/write the journal instead of TASKS.md. Reviewer return contract (`verdict_return_contract.md`) gains `model` requirement on returned `<review>` so orchestrator transcribes verbatim. Original F-11 hash-lock framing dropped (DEC-005): state-machine drift recovery via amend-flip suffices once TASKS.md is near-static. Full historical rewrite of SPEC-0001..SPEC-0036 is in scope, performed manually by LLM (no script, no `speccy migrate` CLI), with `JNL-*` lint as the correctness floor. `ARCHITECTURE.md` and `AGENTS.md` update to document the new layout, renames, schemas, and lint family. Closed XML element set stays at six (renames net out). No `effort` attribute, no `host` attribute, no `task_hash_at_generation` field, no new wrapper element, no new CLI subcommand. Brainstorm framing approved in conversation 2026-05-21 prior to plan invocation; alternative framings A (hash-lock TASKS.md), B (in-place compaction via `<history>` element), and C (per-attempt sharded files) all rejected. |
| 2026-05-21 | human/kevin + Claude | Mid-decomposition scope expansion: drop the `<tasks spec="SPEC-NNNN">` wrapper from TASKS.md (DEC-008). The wrapper element duplicates a binding already encoded by filename `TASKS.md`, parent directory `NNNN-slug`, and frontmatter `spec:` field. REQ-005 extended to remove `<tasks>` from the closed element allow-list (set shrinks from six to five: `task`, `task-scenarios`, `implementer`, `review`, `blockers`). New scenarios CHK-025 (`<tasks>` wrapper trips unknown-element error) and CHK-026 (bare `<task>` children parse cleanly with no wrapper) capture the parser-side change. REQ-010 migration scope extended: every TASKS.md from SPEC-0001 through SPEC-0036 also has its `<tasks ...>` opening and `</tasks>` closing stripped as part of the activity-prose ejection. CHK-021 extended to grep for `<tasks` alongside the legacy elements; new CHK-027 asserts bare-`<task>`-under-heading shape. REQ-011 doc updates extended to document the smaller closed set and the wrapper-less TASKS.md shape. Decomposition-time decision; the wrapper is gone and won't be re-added. |
| 2026-05-21 | human/kevin + Claude | Migration scope expansion: REQ-010 grows from 36 historical specs (SPEC-0001..0036) to 37 inclusive specs (SPEC-0001..0037). SPEC-0037 itself joins the migration sweep because implementer + reviewer agents working on T-001..T-005 follow the un-updated skill bodies and write activity prose in legacy format into SPEC-0037's own TASKS.md. After T-001's parser cutover, that format is unparseable, so SPEC-0037 needs the same migration as every historical spec. T-006's implementer entry goes directly to `journal/T-006.md` in the new format (the implementer of T-006 is aware of the new schema by then). Decomposition-time decision driven by user preference for "quickest path to completion" — accepts a transient red-CI window in exchange for not requiring per-agent dogfooding discipline during T-001..T-005. Done-when bullets, behavior, and CHK-021 / CHK-022 scenarios updated to reference 37 specs throughout. |
| 2026-05-21 | human/kevin + Claude | Mid-loop decomposition revision: T-001's monolithic scope (Rust code + tests + docs + host-pack regen, covering REQ-001..007 + REQ-011) split into two narrower tasks. T-001 narrowed to code-only (Rust parser, lint, tests, ignore-tags on two integration tests; covers REQ-001..007). New T-002 inserted for docs sweep + host-pack regen (covers REQ-011 — `docs/ARCHITECTURE.md` and `AGENTS.md` describe the new shape; then `speccy init --force` regenerates `.claude` / `.codex` / `.agents`). T-002 lands before the skill-body tasks so their implementers / reviewers read accurate architectural docs. The previously-numbered T-002 through T-006 renumbered to T-003 through T-007. T-007 (was T-006) is the workspace-wide migration sweep. T-001's existing implementer-note from the punted round-1 attempt is preserved as a forensic record of the friction that motivated this split. The `<tasks spec="SPEC-0037">` wrapper was stripped from SPEC-0037's own TASKS.md as part of this orchestrator-side restructure (T-007 still strips the wrapper from the 36 historical specs, but SPEC-0037's own wrapper is gone now, making `speccy check SPEC-0037` parseable immediately). Two integration tests that walk the live in-tree corpus (`every_in_tree_tasks_md_parses_and_has_populated_scenarios` in `speccy-core/tests/in_tree_tasks_reports.rs`, and `speccy_verify_exits_zero_on_migrated_in_tree_workspace` in `speccy-cli/tests/verify_after_migration.rs`) are explicitly marked `#[ignore]` as part of T-001 with a comment pointing at T-007; T-007 removes those attributes as the final step of the sweep. |
</changelog>

## Open Questions

- [x] a. Migration task granularity in F-11's TASKS.md — one task per
  legacy spec (~36 tasks), batched (e.g., 6 tasks each covering 6
  specs), or one meta-task. **Resolved 2026-05-21:** single
  migration task (T-007, was T-006 before the decomposition
  revision) covering all 37 in-tree specs in one implementer +
  reviewer turn, with multiple retry rounds expected. User
  directive: "we'll migrate ALL tasks at once as part of a single
  implementer and reviewer step."
- [x] b. Build sequence for `ARCHITECTURE.md` and `AGENTS.md` updates
  (REQ-011) — land alongside the parser and lint changes (early)
  so docs and code ship together, or as a final documentation
  pass after migration completes (late). **Resolved 2026-05-21
  (revised same-day after T-001 punt):** early-ish — docs split
  out to T-002 (was bundled in T-001's original scope) and run
  immediately after T-001's Rust code lands, before the
  skill-body tasks. Docs describe the new shape
  contemporaneously and migration agents reading the regenerated
  AGENTS.md see the post-cutover format. The reason for the
  split: T-001's combined scope was too large for a single
  implementer round (the punted round-1 attempt surfaced this);
  splitting docs into T-002 keeps each task scoped to a single
  surface (Rust vs prose).
- [x] c. Parser allow-list strategy for the legacy element names
  (`<implementer-note>`, `<retry>`) — drop them outright (hard
  cutover) or keep parseable for a grace period. **Resolved
  2026-05-21:** hard cutover in T-001 foundation task. Element
  renames + `<tasks>` wrapper drop all land simultaneously; no
  alias period. Migration sweep in T-007 restores green CI
  across all 37 specs.
- [x] d. Whether the SPEC-0036 in-flight carve-out (assumption A9
  end) needs a named requirement in this SPEC, or whether
  decomposition-time TASKS.md handles it implicitly. **Resolved
  2026-05-21:** moot — SPEC-0036 shipped (commit `726a9f6 Ship
  SPEC-0036`) before F-11 implementation begins, so no carve-out
  is needed. T-007 migrates SPEC-0036 like every other spec in
  the sweep.

## Notes

The original F-11 backlog item bundled three concerns into one
description: (1) lock TASKS.md after impl/review phases to
prevent file-level drift, (2) standardize the XML attribute
formatting on `<implementer-note>` and `<review>` for
traceability (date / model / retry-num), and (3) compact the
2-3× file bloat from accumulated activity prose. The
brainstorm before this plan untangled these. Concern (3) (file
bloat) is the load-bearing user pain; the eject-to-journal
mechanism solves it directly. Concern (2) (traceability)
rides along on the new element schema. Concern (1) (file lock)
dissolves once the eject lands — TASKS.md becomes near-static
so the file-level lock no longer earns its weight, and
state-machine drift recovery via the existing amend-flip
behavior covers the actually-useful case (amendment-mid-task).
F-11 ships (3) + (2); explicitly drops (1).

Alternative framings considered and rejected during brainstorm:

- **Framing A — Hash-lock TASKS.md in place.** Original F-11
  wording. Add hash mechanism after impl + review phase
  boundaries; keep notes accumulating in TASKS.md; gate writes
  by hash check. Rejected because a hash gate doesn't reduce
  file bloat — it only detects drift. The 2-3× growth problem
  stays; the hash mechanism solves a different (much smaller)
  concern.
- **Framing B — In-place compaction via `<history>` element.**
  Keep current/latest blocks at the top of `<task>`; fold
  older retries into a `<history>` sub-element block within
  TASKS.md. Rejected by user directly during brainstorm: "we
  need a new file for this." Also keeps the single-file race
  surface that the eject solves for free.
- **Framing C — Per-attempt sharded files.** One file per
  retry attempt: `journal/T-NNN/attempt-1.md`, `attempt-2.md`,
  etc. Rejected because it optimizes for the failure mode
  (high retry counts) Speccy would rather not normalize. The
  right answer to "T-005 keeps blocking on retry" is
  `/speccy-amend`, not sharding the journal into more files.
  Also forces `speccy-ship` to walk a directory tree to
  summarize each task — added complexity for negligible
  benefit.
