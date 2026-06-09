---
id: SPEC-0056
slug: task-context-bundle
title: Task-scoped context bundle — `speccy context` emits one JSON read for loop subagents
status: in-progress
created: 2026-06-09
supersedes: []
---

# SPEC-0056: Task-scoped context bundle — `speccy context` emits one JSON read for loop subagents

## Summary

Every per-task loop subagent today opens with the same read recipe:
full SPEC.md, full TASKS.md, the per-task journal, plus a
`speccy check` invocation for scenario prose. With four to six
subagents per task per round, read traffic grows with
**spec size × task count × persona count** — super-linear in spec
size, which is precisely what makes 10–20-task specs expensive while
adding tool-call latency even to 1–3-task specs.

This SPEC adds one read command. `speccy context <task-selector>
--json` emits a single schema-versioned JSON bundle scoped to the
selected task: spec identity and intent blocks, the task's verbatim
`<task>` entry, the covering requirements with their scenarios, the
full per-task journal, a sibling-task index, a suggested diff
command, task-scoped consistency drift, and file paths for follow-up
targeted reads. Loop personas replace their multi-step read recipe
with this one call.

The governing design invariant: **bundle size scales with the task,
not the spec.** Covering requirements are bounded by the task's
`covers` attribute, the journal by this task's rounds, and the only
fields that grow with task count are the one-line-per-task sibling
index and nothing else. The intent blocks are bounded by authorship,
not task count. This is what makes large specs viable; small specs
still gain one tool roundtrip instead of three or four, and shorter
persona prose.

Grounding (traced in-session): the resolution this command needs
already exists for `speccy check` — `task_lookup::parse_ref` /
`task_lookup::find` resolve qualified and unqualified task
selectors, and `check::run_task` walks `task.covers` →
`SpecDoc.requirements` → `req.scenarios`
(`speccy-cli/src/check.rs:265-323`). That walk moves to a shared
`speccy-core` function so `check` and `context` cannot drift apart.
Consistency drift detection already produces per-task drift entries
with `task_id`, and the merge-base git probe used by `speccy next`
covers the suggested-diff-command computation. The JSON envelope
follows the existing convention (`schema_version: 1` first field,
per-command serialize struct).

This is the second SPEC of an agreed pair. SPEC-0055
(lifecycle-write-commands) owns the write side; this SPEC depends on
nothing from SPEC-0055 at the CLI layer (it is a pure read command),
but its skill-migration requirement touches the same persona modules
SPEC-0055's pack-migration tasks touch — see Notes for sequencing.

## Goals

<goals>
- A loop subagent obtains everything its task-scoped role needs from
  one `speccy context` invocation instead of reading full SPEC.md,
  full TASKS.md, the journal, and invoking `speccy check`.
- The bundle payload for a given task is invariant to spec growth
  except for one index line per added sibling task.
- An implementer's reuse survey can see which sibling slices already
  landed (id, state, covers) without reading TASKS.md.
- A subagent reading a bundle on a drifted workspace sees the drift
  status and the drifts affecting its own task at the moment of
  read.
- `speccy check`'s text output is byte-identical before and after
  this SPEC.
</goals>

## Non-goals

<non-goals>
- No bare-spec selector form (`speccy context SPEC-NNNN`). No call
  site exists today; the selector grammar leaves the extension
  additive if dogfooding produces one.
- No role-shaped bundles or content-affecting mode flags. One
  superset payload; roles ignore fields they do not need. `--json`
  continues to toggle representation, never content.
- No vet-persona migration. Vet review is whole-SPEC holistic scope,
  which a task-scoped bundle structurally cannot serve; vet personas
  keep their full reads.
- No inlined diff content. The bundle carries a suggested diff
  command string; personas run it themselves and scope it as they
  see fit.
- No inlined evidence files. Evidence stays a pointer inside the
  implementer's journal block — one extra targeted read for the one
  persona that needs it.
- No change to `speccy check` behavior or output. The two commands
  complement: `check` renders human-facing text, `context` emits the
  agent-facing bundle, both over one shared resolution function.
- No full consistency report in the bundle. Workspace-level status
  plus task-scoped drifts only; the full drift catalogue remains
  `speccy next`'s job.
</non-goals>

## User Stories

<user-stories>
- As a reviewer persona on a 20-task spec, I want one CLI call to
  hand me the task, its requirements, its scenarios, and its journal
  history, so that my context window is not spent on eighteen tasks
  and eight requirements that are not mine to review.
- As an implementer running the pre-implementation reuse survey, I
  want the bundle's sibling index to show what adjacent slices
  already landed, so that I extend existing seams instead of
  re-inventing them.
- As a solo developer on a 2-task spec, I want personas to start
  from one tool call instead of a four-step read recipe, so that
  each loop iteration is faster even when the token savings are
  small.
</user-stories>

## Assumptions

<assumptions>
- Superset over-provisioning is acceptable: fields irrelevant to a
  given role cost each subagent a few KB, still well under today's
  full-file reads.
- Per-task journals stay small (KBs per round); inlining the full
  journal is cheaper than forcing a second tool call to read it.
- Intent prose (`<goals>`, `<non-goals>`, `<decision>` blocks) is
  bounded by authorship discipline, not by task count, so carrying
  it whole does not break the size invariant.
- The merge-base git probe already used for consistency checking is
  sufficient to compute the suggested diff command; no new git
  machinery is needed.
- SPEC-0055's pack migration lands before or together with this
  SPEC's persona migration; the two edit the same persona modules
  and must not be authored against divergent baselines.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: `speccy context <task-selector> --json` emits a schema-versioned bundle

The command resolves the task via the same selector grammar as
`speccy check` (`task_lookup::parse_ref` accepting `T-NNN` and
`SPEC-NNNN/T-NNN`, then `task_lookup::find`, with the same
ambiguity and not-found diagnostics) and prints a single JSON
envelope to stdout whose first field is `schema_version` pinned to
`1`. Selector failures exit non-zero without partial output.

<done-when>
- Qualified and unqualified selectors resolve; ambiguous unqualified
  selectors and unknown tasks produce the same diagnostic classes as
  `speccy check`.
- Stdout parses as a single JSON document with `schema_version: 1`
  as the first serialized field.
- The command performs no writes anywhere in the workspace.
</done-when>

<behavior>
- Given a workspace with one spec containing T-001, when
  `speccy context T-001 --json` runs, then the unqualified selector
  resolves and the envelope is emitted with exit code 0.
- Given two specs both containing T-001, when
  `speccy context T-001 --json` runs, then the process exits
  non-zero with an ambiguity diagnostic naming both specs.
</behavior>

<scenario id="CHK-001">
Given a fixture workspace with two specs that both contain a task
T-001,
when `speccy context T-001 --json` runs,
then the exit code is non-zero and stderr carries the same
ambiguity diagnostic class `speccy check T-001` produces.
</scenario>

<scenario id="CHK-002">
Given a fixture workspace with a single spec and task
SPEC-0042/T-001,
when `speccy context SPEC-0042/T-001 --json` runs,
then stdout parses as JSON whose first field is `schema_version`
with value 1.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: Bundle carries spec identity plus intent blocks

The envelope carries the spec's identity (frontmatter `id`, `title`,
`status`) plus the intent surfaces: the `<goals>` bullet block, the
`<non-goals>` bullet block, and every `<decision>` block with its
DEC id. The Summary narrative, user stories, notes, and requirements
not covered by the task are excluded.

<done-when>
- The envelope exposes id, title, and status matching SPEC.md
  frontmatter.
- Goals and non-goals bullet text appears in the envelope.
- Each `<decision id="DEC-NNN">` appears with its id and body.
- No `## Summary` narrative, `<user-stories>` content, or
  non-covered `<requirement>` body appears in the envelope.
</done-when>

<behavior>
- Given a SPEC.md with three decisions, when the bundle is emitted
  for any of its tasks, then all three DEC ids appear with bodies.
- Given a spec whose Summary is long narrative prose, when the
  bundle is emitted, then that prose is absent from the payload.
</behavior>

<scenario id="CHK-003">
Given a fixture spec with two goals bullets, one non-goals bullet,
two decisions, and a Summary paragraph containing a distinctive
marker string,
when the bundle for one of its tasks is emitted,
then goals, non-goals, and both decisions are present and the
marker string is absent.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Bundle carries the task entry plus covering requirements via the shared resolution walk

The envelope carries the selected task's verbatim `<task>` entry
(id, state, covers, raw body) plus, for each requirement id in
`covers`, the full requirement (heading title, prose body,
`<done-when>`, `<behavior>`) with its `<scenario>` blocks. The
covers → requirements → scenarios walk is extracted from
`check::run_task` into one shared `speccy-core` function consumed by
both `speccy check` and `speccy context`, and `speccy check`'s text
output remains byte-identical across the refactor.

<done-when>
- The task's raw `<task>` entry bytes appear in the envelope along
  with parsed id, state, and covers.
- Exactly the covering requirements appear — full bodies with
  scenarios, deduplicated in declared order, matching `speccy
  check`'s resolution semantics.
- A `covers` token referencing a missing requirement surfaces in
  the envelope the same condition `speccy check` reports.
- One shared core function performs the walk for both commands; the
  CLI crates contain no duplicate of it.
- Existing `speccy check` integration tests pass unchanged.
</done-when>

<behavior>
- Given a task with `covers="REQ-001 REQ-003"` in a five-requirement
  spec, when the bundle is emitted, then REQ-001 and REQ-003 appear
  in full with their scenarios while REQ-002, REQ-004, and REQ-005
  are absent.
- Given the refactor landed, when `speccy check SPEC-0042/T-001`
  runs against a fixture, then its stdout is byte-identical to the
  pre-refactor output.
</behavior>

<scenario id="CHK-004">
Given a fixture spec with five requirements and a task covering two
of them,
when the bundle is emitted,
then the two covered requirements appear with done-when, behavior,
and scenario content, and none of the other three requirement ids
appear anywhere in the payload.
</scenario>

<scenario id="CHK-005">
Given the `speccy check` integration test fixtures at HEAD before
this SPEC's refactor,
when the shared-walk refactor lands,
then every existing `check` test passes without fixture or
expectation changes.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: Bundle inlines the full per-task journal; absence is empty, not an error

When `<spec-dir>/journal/<task-id>.md` exists, the envelope inlines
its full content — frontmatter fields plus every block
(`<implementer>`, `<review>`, `<blockers>`) across all rounds, with
their attributes. When the journal does not exist, the envelope
carries an explicit empty journal section (e.g. an `exists: false`
marker with no blocks) and the command still exits 0 — a round-1
implementer legitimately has no journal yet, so absence here is
normal rather than anomalous.

<done-when>
- A journal with three rounds appears in the envelope with all
  blocks and attributes in file order.
- A task with no journal file yields exit 0 with an explicit
  empty-journal marker in the envelope.
- The journal content in the envelope is sufficient for retry
  context: prior implementer handoffs, review verdicts, and
  blockers directives are all present.
</done-when>

<behavior>
- Given a task journal with two rounds including a blocking review
  and a blockers block, when the bundle is emitted, then all blocks
  appear with persona, verdict, and round attributes.
- Given a pending round-1 task with no journal, when the bundle is
  emitted, then the command exits 0 and the journal section reads
  as explicitly empty.
</behavior>

<scenario id="CHK-006">
Given a fixture journal with rounds 1–2, five review blocks, and one
blockers block,
when the bundle is emitted,
then the JSON journal section contains all eight blocks
(2 implementer + 5 review + 1 blockers) with round attributes
matching the file.
</scenario>

<scenario id="CHK-007">
Given a fixture task with no journal file,
when the bundle is emitted,
then the exit code is 0 and the journal section carries an explicit
absence marker with zero blocks.
</scenario>
</requirement>

<requirement id="REQ-005">
### REQ-005: Bundle carries a sibling index, file paths, and a suggested diff command

The envelope carries three navigation aids: (1) a sibling-task index
listing every other task in the spec as id, state, and covers only —
never bodies; (2) repo-relative paths to SPEC.md, TASKS.md, and the
task's journal file for follow-up targeted reads; (3) a suggested
diff command string in merge-base form computed by the CLI from the
repo's git state, which personas run themselves.

<done-when>
- For a spec with N tasks the index has N−1 entries, each exactly
  id, state, and covers.
- No sibling task body text appears anywhere in the envelope.
- The three paths resolve to the actual files from the repo root.
- The diff command string contains the merge-base form against the
  repository's default branch and is runnable as-is from the repo
  root.
</done-when>

<behavior>
- Given a six-task spec, when the bundle for T-003 is emitted, then
  the index lists T-001, T-002, T-004, T-005, T-006 with state and
  covers and nothing else.
- Given a repo on a feature branch, when the bundle is emitted,
  then running the suggested diff command from the repo root
  produces the branch diff without modification.
</behavior>

<scenario id="CHK-008">
Given a fixture spec with six tasks whose bodies each contain a
distinctive marker string,
when the bundle for T-003 is emitted,
then the sibling index has five entries with only id, state, and
covers fields, and no sibling marker string appears in the payload.
</scenario>
</requirement>

<requirement id="REQ-006">
### REQ-006: Bundle carries workspace consistency status with task-scoped drifts only

The envelope carries a consistency section with the workspace-level
status (the same status classification `speccy next` computes) plus
only the drift entries whose `task_id` matches the selected task.
Drift entries for other tasks never appear regardless of how many
exist. The bundle is still emitted when drift exists — `speccy
context` is a read command and never refuses on drift; surfacing the
status at read time is the feedback mechanism.

<done-when>
- A clean workspace yields a consistency section with an ok status
  and zero drift entries.
- A workspace where the selected task has one drift and four other
  tasks have drifts yields exactly the one matching entry plus the
  non-ok status.
- Drift never changes the exit code of a successful bundle
  emission.
</done-when>

<behavior>
- Given an amended SPEC.md that drifts three tasks including T-002,
  when the bundle for T-002 is emitted, then the consistency status
  is non-ok and exactly T-002's drift entries appear.
- Given the same workspace, when the bundle for an undrifted T-005
  is emitted, then the status is still non-ok and the task-scoped
  drift list is empty.
</behavior>

<scenario id="CHK-009">
Given a fixture workspace with hash drift affecting two tasks,
when bundles for one drifted task and one undrifted task are
emitted,
then both exit 0, both carry the non-ok workspace status, the
drifted task's bundle carries only its own drift entries, and the
undrifted task's bundle carries an empty drift list.
</scenario>
</requirement>

<requirement id="REQ-007">
### REQ-007: Bundle size scales with the task, not the spec

For a fixed task, growing the spec around it changes the bundle
only in bounded, enumerable ways: adding a task to the spec adds
exactly one sibling-index entry; adding a requirement the task does
not cover adds nothing; adding rounds to another task's journal
adds nothing. This invariant is enforced by a property-style test
rather than left as prose intent.

<done-when>
- A test constructs a fixture spec, emits the bundle for one task,
  then grows the spec (one uncovered requirement, one new task, one
  journal round on another task) and re-emits: the second bundle
  differs from the first only by the single added sibling-index
  entry.
- The invariant is documented in the command's ARCHITECTURE.md
  entry as a contract, not an implementation detail.
</done-when>

<behavior>
- Given a bundle emitted for T-001, when an uncovered REQ-009 is
  added to SPEC.md and the bundle is re-emitted, then the two
  payloads are identical except for consistency fields affected by
  the SPEC edit.
- Given a bundle emitted for T-001, when sibling T-008 is appended
  to TASKS.md and the bundle is re-emitted, then the only
  non-consistency difference is one new index entry.
</behavior>

<scenario id="CHK-010">
Given a property-style test that emits a bundle, grows the fixture
spec by one uncovered requirement plus one sibling task plus one
foreign journal round, re-locks the hash, and re-emits,
when the two payloads are diffed,
then the differences are exactly one sibling-index entry and
nothing else.
</scenario>
</requirement>

<requirement id="REQ-008">
### REQ-008: Loop personas open with one `speccy context` call

The skill-pack source modules change the entry-read contract for the
task-scoped roles: the `speccy-work` implementer phase and the six
reviewer personas begin from a single `speccy context
SPEC-NNNN/T-NNN --json` invocation. Persona prose stops instructing
full-file SPEC.md or TASKS.md reads, and stops invoking `speccy
check`, for per-task scoping; targeted follow-up reads via the
bundle's paths remain legitimate when a persona needs something
outside the bundle (e.g. the evidence file). Vet personas are
untouched. Both host ejections regenerate from source.

<done-when>
- No reviewer persona or implementer phase module under
  `resources/modules/` instructs reading SPEC.md or TASKS.md in
  full or invoking `speccy check` as the entry read; each opens
  with the `speccy context` call.
- Follow-up targeted reads (evidence file, bundle-listed paths) are
  still permitted by the prose where the role needs them.
- Vet persona modules are byte-identical before and after this
  SPEC.
- `just reeject` regenerates both host packs and leaves a clean
  tree against the committed ejections.
</done-when>

<behavior>
- Given the regenerated packs, when a reviewer persona body is
  read, then its entry procedure names one `speccy context` call
  and references bundle fields (task, requirements, scenarios,
  journal, diff command) rather than file-read steps.
- Given the regenerated packs, when the vet persona bodies are
  diffed against the prior commit, then they are unchanged.
</behavior>

<scenario id="CHK-011">
Given a clean checkout after this SPEC lands,
when `just reeject` runs and `git status --porcelain` is checked,
then the working tree is clean, proving committed ejections match
the updated sources.
</scenario>

<scenario id="CHK-012">
Given the updated module sources,
when the reviewer reads each task-scoped persona and the implementer
phase module,
then each opens its read procedure with the `speccy context` call
and none instructs a full SPEC.md or TASKS.md read as entry context
(content check by reviewer, not substring assertion).
</scenario>
</requirement>

<requirement id="REQ-009">
### REQ-009: ARCHITECTURE.md documents the command and its envelope

`docs/ARCHITECTURE.md` gains the `speccy context` entry in the CLI
surface section: selector grammar, the envelope's sections
(identity, intent, task, covering requirements, journal, sibling
index, paths, diff command, consistency), the schema_version
contract, and the size invariant from REQ-007 stated as a contract.
The "skills layer reads" prose that currently describes personas
reading SPEC.md and TASKS.md in full is updated to describe the
bundle entry read.

<done-when>
- The CLI surface section lists `speccy context` with its envelope
  contract and the size invariant.
- No ARCHITECTURE.md section still describes full-file persona
  entry reads as the current contract.
</done-when>

<behavior>
- Given the updated ARCHITECTURE.md, when the reviewer reads the
  CLI surface and the persona read-contract prose, then both
  describe the post-SPEC-0056 behavior.
</behavior>

<scenario id="CHK-013">
Given the updated ARCHITECTURE.md,
when the reviewer checks the CLI surface table and the persona
read-contract prose,
then `speccy context` is documented with envelope and invariant,
and no stale full-file entry-read contract remains (content check
by reviewer, not substring assertions).
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
One superset bundle, no role-shaped variants and no
content-affecting flags. Roles ignore fields they do not need.
Role flags (`--for implementer|reviewer`) would introduce the
project's first content-mode toggle (violating "`--json` toggles
representation, not content") and create two payload shapes to keep
in sync for a few KB of savings.
</decision>

<decision id="DEC-002">
`speccy context` complements `speccy check`; both consume one
shared resolution function in `speccy-core`. `check` stays the
human/CI-facing text renderer with byte-stable output; `context` is
the agent-facing JSON bundle. Subsuming `check` or giving it a
bundle mode was rejected — the former breaks the documented human
surface, the latter makes one command two products.
</decision>

<decision id="DEC-003">
The diff is a suggested command string, never inlined content, and
evidence files stay pointer-only. Diffs reach hundreds of KB and
personas already scope their own diff fetches; evidence is needed by
one persona and costs one targeted read.
</decision>

<decision id="DEC-004">
The journal is inlined in full rather than delegated to a second
read (e.g. SPEC-0055's `journal show`). One call beats two for
every subagent on every round, journals are KBs per round, and the
implementer's retry context needs full history anyway.
</decision>

<decision id="DEC-005">
Consistency in the bundle is workspace-level status plus
task-scoped drift entries only. Inlining the full drift catalogue
would make the bundle O(workspace) on heavily drifted large specs,
violating the size invariant; the full catalogue remains
`speccy next`'s job. The bundle never refuses on drift — surfacing
status at read time is the feedback mechanism, and refusing would
deadlock reconcile flows that need context to fix the drift.
</decision>

<decision id="DEC-006">
Task-selector-only surface. The bare-spec form has no call site
(vet personas are holistic and excluded; plan-time grounding uses
the explorer subagent). The polymorphic selector grammar keeps the
extension additive if dogfooding produces a real call site.
</decision>

## Notes

Rejected framings from the brainstorm session:

- **Role-shaped bundles** behind a `--for` flag — tightest payloads,
  rejected as the project's first content-mode toggle (DEC-001).
- **`speccy check --json --bundle` mode** — avoids a new command but
  turns `check` into two products behind flags (DEC-002).
- **Tighter persona prose with targeted reads** — no new command,
  but pushes scoping intelligence into prompts (fragile,
  model-dependent) and still costs multiple tool calls per subagent.
- **Orchestrator embeds context into subagent prompts** — the
  orchestrator would read once and inline the bundle per persona,
  re-creating the relay tax in reverse: the bundle transits the
  orchestrator context once per persona instead of zero times.

**Sequencing against SPEC-0055.** This SPEC's CLI work (REQ-001..
REQ-007) is independent of SPEC-0055 — `speccy context` is a pure
read command. The persona migration (REQ-008) edits the same module
files as SPEC-0055's pack-migration tasks (T-007/T-008 there).
Decompose should order this SPEC's prose tasks after SPEC-0055's
pack migration lands, or the two edits must be authored against a
coordinated baseline; landing them against divergent baselines will
produce conflicting persona bodies.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-09 | claude-fable-5[1m] | Initial SPEC: task-scoped read bundle. REQ-001: `speccy context <task-selector> --json` with check's selector grammar and schema_version 1 envelope. REQ-002: identity + goals/non-goals/decisions intent slice (Summary, user stories, non-covered requirements excluded). REQ-003: task entry + covering requirements + scenarios via a shared speccy-core walk extracted from check::run_task, check output byte-stable. REQ-004: full journal inlined, absence = explicit empty + exit 0. REQ-005: sibling index (id/state/covers only), file paths, suggested merge-base diff command. REQ-006: workspace consistency status + task-scoped drifts only, never refuses on drift. REQ-007: size invariant (bundle scales with task, not spec) enforced by property test. REQ-008: implementer phase + six reviewer personas open with one context call; vet personas untouched; packs re-ejected. REQ-009: ARCHITECTURE.md documents command, envelope, invariant. Six decisions DEC-001..DEC-006. Companion to SPEC-0055 (write side); persona-module sequencing noted. |
</changelog>
