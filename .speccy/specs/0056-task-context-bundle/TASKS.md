---
spec: SPEC-0056
spec_hash_at_generation: bed490bc1dde18ac3be2ed3dcb3472cffcc8330465909a5fba137c3aba305682
generated_at: 2026-06-09T19:52:47Z
---
# Tasks: SPEC-0056 Task-scoped context bundle — `speccy context` emits one JSON read for loop subagents

<task id="T-001" state="pending" covers="REQ-003">
## Extract the covers → requirements → scenarios walk into a shared `speccy-core` function

Lift the resolution walk currently inlined in `check::run_task`
(`speccy-cli/src/check.rs:300-315` — iterate `location.task.covers`,
find each id in `spec_doc.requirements`, accumulate `req.scenarios`
deduplicated by scenario id in first-occurrence requirement-declared
order) into one public function in a new `speccy-core` module (e.g.
`speccy-core/src/context.rs` or a `resolve` submodule). The function
takes the parsed `Task` and the `SpecDoc` and returns the covering
requirements (each with its `scenarios`) in declared order,
deduplicated, preserving the existing semantics exactly: empty-covers
yields the empty set, and a `covers` token with no matching
`req.id` is silently skipped at this layer (the lint engine's TSK-001
owns that absence, per the `check.rs:261-264` doc comment).

Rewrite `check::run_task` to consume the shared function so the CLI
crate holds no duplicate of the walk. `speccy check`'s text output
must remain byte-identical: the rendered scenario set, ordering, and
the `render_checks` count summary are unchanged. Run the existing
`speccy check` integration tests unmodified to prove byte-stability;
do not edit their fixtures or expectations.

This task lands first because both the refactor (REQ-003) and the
new `speccy context` command consume this one function — extracting
it before the command exists keeps the two commands from drifting
apart (DEC-002).

<task-scenarios>
Given the `speccy check` integration test fixtures at HEAD before
this refactor,
when the shared-walk refactor lands and `cargo test --workspace`
runs,
then every existing `check` test passes without any fixture or
expectation change (CHK-005).

Given a fixture spec with five requirements and a task covering two
of them,
when the shared function resolves the task,
then exactly the two covered requirements are returned, each with
its scenarios, in declared order, and the other three are absent.

Given a task whose `covers` names a requirement id absent from the
spec,
when the shared function resolves the task,
then that token is skipped without error, matching today's
`run_task` behavior.

Suggested files: `speccy-core/src/context.rs`,
`speccy-core/src/lib.rs`, `speccy-cli/src/check.rs`,
`speccy-cli/tests/` (existing check integration tests, run unchanged)
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-001 REQ-002">
## Add the `speccy context <task-selector> --json` command emitting identity + intent

Add `Command::Context { selector: String, json: bool }` to the clap
enum in `speccy-cli/src/main.rs:60-133` and wire its dispatch arm in
`main.rs:140-170`, following the existing command shape. Resolve the
selector through `task_lookup::parse_ref` then `task_lookup::find`
(accepting `T-NNN` and `SPEC-NNNN/T-NNN`), reusing the same
`LookupError` → exit-code/diagnostic mapping `speccy check` uses at
`main.rs:354-368` (InvalidFormat, NotFound, Ambiguous). Selector
failures exit non-zero with no partial stdout.

Introduce `speccy-cli/src/context.rs` (run entry + bundle assembly)
and `speccy-cli/src/context_output.rs` (serde `Serialize` structs)
modelled on `next_output.rs:22-105`: the envelope's first serialized
field is `schema_version` pinned to `1`. In this task the envelope
carries (1) spec identity — frontmatter `id`, `title`, `status`; and
(2) the intent slice — the `<goals>` body, the `<non-goals>` body,
and every `<decision>` with its DEC id and body, read from
`SpecDoc.goals` / `SpecDoc.non_goals` / `SpecDoc.decisions`
(`speccy-core/src/parse/spec_xml/mod.rs:53-142`). The Summary
narrative, `<user-stories>`, Notes, and non-covered requirement
bodies are excluded. The command performs no writes anywhere.

Later tasks (T-003..T-006) extend this same envelope; this task
establishes the command, the selector contract, and the JSON
skeleton with identity + intent populated.

<task-scenarios>
Given a fixture workspace with two specs that both contain a task
T-001,
when `speccy context T-001 --json` runs,
then the exit code is non-zero and stderr carries the same
ambiguity diagnostic class `speccy check T-001` produces (CHK-001).

Given a fixture workspace with a single spec and task
SPEC-0042/T-001,
when `speccy context SPEC-0042/T-001 --json` runs,
then stdout parses as one JSON document whose first field is
`schema_version` with value 1 (CHK-002).

Given a fixture spec with two goals bullets, one non-goals bullet,
two decisions, and a Summary paragraph containing a distinctive
marker string,
when the bundle for one of its tasks is emitted,
then goals, non-goals, and both DEC ids with bodies are present and
the Summary marker string is absent from the payload (CHK-003).

Suggested files: `speccy-cli/src/main.rs`,
`speccy-cli/src/context.rs`, `speccy-cli/src/context_output.rs`,
`speccy-cli/src/lib.rs`, `speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-003">
## Populate the bundle's task entry and covering requirements via the shared walk

Extend the `speccy context` envelope to carry the selected task's
verbatim `<task>` entry — the raw body bytes (`Task.body`) plus the
parsed `id`, `state`, and `covers`
(`speccy-core/src/parse/task_xml/mod.rs:122-135`) — and, for each id
in `covers`, the full covering requirement: heading title, prose
body, `<done-when>`, `<behavior>`, and its `<scenario>` blocks
(`Requirement` at `spec_xml/mod.rs:94-118`). Resolve the covering
requirements through the shared function landed in T-001 so `context`
and `check` cannot diverge. Requirements appear deduplicated in
declared order; a `covers` token referencing a missing requirement
surfaces the same condition the shared walk reports.

<task-scenarios>
Given a fixture spec with five requirements and a task covering two
of them,
when the bundle is emitted,
then the two covered requirements appear with done-when, behavior,
and scenario content, and none of the other three requirement ids
appear anywhere in the payload (CHK-004).

Given a task with `covers="REQ-001 REQ-003"` in a five-requirement
spec,
when the bundle is emitted,
then REQ-001 and REQ-003 appear in full with their scenarios while
REQ-002, REQ-004, and REQ-005 are absent, and the task's raw
`<task>` body bytes appear alongside the parsed id, state, and
covers.

Suggested files: `speccy-cli/src/context.rs`,
`speccy-cli/src/context_output.rs`, `speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-004">
## Inline the full per-task journal; absence is an explicit empty marker, not an error

Extend the envelope with a journal section. When
`<spec-dir>/journal/<task-id>.md` exists, parse it via
`journal_xml::parse` (`speccy-core/src/parse/journal_xml/mod.rs:151`)
and inline its full content: the frontmatter fields plus every
`<implementer>` / `<review>` / `<blockers>` entry across all rounds
in file order, each with its attributes (`date`, `model`, `round`,
and for review `persona`/`verdict`). The journal content must be
sufficient for retry context — prior implementer handoffs, review
verdicts, and blockers directives all present. When the journal file
does not exist, the envelope carries an explicit empty-journal marker
(e.g. an `exists: false` field with zero entries) and the command
still exits 0: a round-1 implementer legitimately has no journal yet
(DEC-004).

<task-scenarios>
Given a fixture journal with rounds 1–2, five review blocks, and one
blockers block,
when the bundle is emitted,
then the JSON journal section contains all eight blocks
(2 implementer + 5 review + 1 blockers) with round attributes
matching the file (CHK-006).

Given a fixture task with no journal file,
when the bundle is emitted,
then the exit code is 0 and the journal section carries an explicit
absence marker with zero blocks (CHK-007).

Suggested files: `speccy-cli/src/context.rs`,
`speccy-cli/src/context_output.rs`, `speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-005" state="pending" covers="REQ-005">
## Add the sibling index, file paths, and a suggested merge-base diff command

Extend the envelope with three navigation aids. (1) A sibling-task
index: every other task in the spec as id, state, and covers only —
never any body text — sourced from the spec's `TasksDoc` tasks
(`task_xml/mod.rs:55-135`), excluding the selected task. (2)
Repo-relative paths to SPEC.md, TASKS.md, and the task's journal file
for follow-up targeted reads, resolved from the spec directory. (3) A
suggested diff command string in merge-base form against the repo's
default branch, computed by the CLI from git state and runnable
as-is from the repo root. The merge-base computation is new ground —
`speccy-cli/src/git.rs` today exposes only `repo_sha`
(`git.rs:21`); add a best-effort default-branch + merge-base probe in
the same module following its non-fatal shell-out convention
(`git.rs:1-22`), reusing the assumption from DEC-003 that no new git
machinery beyond the existing probe pattern is needed.

<task-scenarios>
Given a fixture spec with six tasks whose bodies each contain a
distinctive marker string,
when the bundle for T-003 is emitted,
then the sibling index has five entries (T-001, T-002, T-004, T-005,
T-006) with only id, state, and covers fields, and no sibling marker
string appears in the payload (CHK-008).

Given a repo on a feature branch,
when the bundle is emitted,
then the three paths resolve to the actual files from the repo root
and the suggested diff command string is in merge-base form against
the default branch and runs as-is from the repo root.

Suggested files: `speccy-cli/src/git.rs`,
`speccy-cli/src/context.rs`, `speccy-cli/src/context_output.rs`,
`speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-006" state="pending" covers="REQ-006">
## Add workspace consistency status with task-scoped drift entries only

Extend the envelope with a consistency section carrying the
workspace-level status (the same `ConsistencyStatus` classification
`speccy next` computes via `consistency::detect`, invoked through
`ShellGitProbe` — see `speccy-cli/src/next.rs:24-25,132`) plus only
the `DriftEntry` items whose `task_id`
(`speccy-core/src/consistency.rs:127-129`) matches the selected task.
Drift entries for other tasks never appear regardless of count. The
bundle is still emitted when drift exists and drift never changes the
exit code of a successful emission — `speccy context` is a read
command and never refuses on drift; surfacing status at read time is
the feedback mechanism (DEC-005). A clean workspace yields an ok
status with zero drift entries.

<task-scenarios>
Given a fixture workspace with hash drift affecting two tasks,
when bundles for one drifted task and one undrifted task are
emitted,
then both exit 0, both carry the non-ok workspace status, the
drifted task's bundle carries only its own drift entries, and the
undrifted task's bundle carries an empty drift list (CHK-009).

Given an amended SPEC.md that drifts three tasks including T-002,
when the bundle for T-002 is emitted,
then the consistency status is non-ok and exactly T-002's drift
entries appear, with no other task's drift entries present.

Suggested files: `speccy-cli/src/context.rs`,
`speccy-cli/src/context_output.rs`, `speccy-cli/tests/context.rs`
</task-scenarios>
</task>

<task id="T-007" state="pending" covers="REQ-007">
## Enforce the size invariant with a property-style test

Add a property-style integration test that constructs a fixture
spec, emits the `speccy context` bundle for one task, then grows the
spec in three ways — one requirement the task does not cover, one new
sibling task, and one journal round on a different task — re-locks
the SPEC hash, and re-emits the bundle for the same task. After
zeroing out consistency fields affected by the SPEC edit, the two
payloads must differ by exactly one added sibling-index entry and
nothing else. This pins the governing invariant (bundle size scales
with the task, not the spec) as an executable contract rather than
prose intent.

<task-scenarios>
Given a property-style test that emits a bundle, grows the fixture
spec by one uncovered requirement plus one sibling task plus one
foreign journal round, re-locks the hash, and re-emits,
when the two payloads are diffed (consistency fields normalized),
then the differences are exactly one sibling-index entry and nothing
else (CHK-010).

Given a bundle emitted for T-001,
when an uncovered requirement is added to SPEC.md and the bundle is
re-emitted,
then the two payloads are identical except for consistency fields
affected by the SPEC edit.

Suggested files: `speccy-cli/tests/context.rs`,
`speccy-cli/tests/` (shared fixture helpers)
</task-scenarios>
</task>

<task id="T-008" state="pending" covers="REQ-008">
## Migrate implementer phase and six reviewer personas to open with one `speccy context` call

PRECONDITION — coordinate with SPEC-0055 before authoring this task.
Per the SPEC Notes "Sequencing against SPEC-0055" and Assumption on
pack migration: SPEC-0055 (lifecycle-write-commands, in-progress, not
yet implemented) edits the same persona module files in its own
T-007/T-008. Author this task only after SPEC-0055's pack migration
has landed, or against a coordinated baseline shared with it —
landing the two against divergent baselines produces conflicting
persona bodies. If SPEC-0055's pack migration has not landed when
this task is picked up, surface the conflict and stop rather than
editing against a divergent baseline.

Edit the host-neutral source modules under `resources/modules/`
(never the ejected `.claude/`, `.agents/`, `.codex/` copies — see
AGENTS.md "Skill pack source of truth"): the `speccy-work`
implementer phase and the six reviewer personas begin their entry
read with a single `speccy context SPEC-NNNN/T-NNN --json`
invocation. Persona prose stops instructing full-file SPEC.md or
TASKS.md reads and stops invoking `speccy check` for per-task
scoping; it references the bundle fields (task, requirements,
scenarios, journal, diff command). Follow-up targeted reads via the
bundle's listed paths (e.g. the evidence file) remain legitimate
where a role needs them. Vet persona modules are left byte-identical.
Run `just reeject` to regenerate both host packs and confirm a clean
tree against the committed ejections.

<task-scenarios>
Given the updated module sources,
when the reviewer reads each task-scoped persona and the implementer
phase module,
then each opens its read procedure with the `speccy context` call
and none instructs a full SPEC.md or TASKS.md read as entry context
(content check by reviewer, not substring assertion) (CHK-012).

Given a clean checkout after this task lands,
when `just reeject` runs and `git status --porcelain` is checked,
then the working tree is clean, proving the committed ejections match
the updated sources, and the vet persona bodies are unchanged from
the prior commit (CHK-011).

Suggested files:
`resources/modules/phases/speccy-work.md`,
`resources/modules/personas/*.md` (six reviewer personas),
`resources/modules/skills/partials/review-fanout.md` (if it carries
the entry-read prose)
</task-scenarios>
</task>

<task id="T-009" state="pending" covers="REQ-009">
## Document `speccy context`, its envelope, and the size invariant in ARCHITECTURE.md

Add the `speccy context` entry to the CLI surface section of
`docs/ARCHITECTURE.md` (the command table near `docs/ARCHITECTURE.md`
lines 152-163 alongside `speccy next` / `speccy check`): the selector
grammar, the envelope's sections (identity, intent, task, covering
requirements, journal, sibling index, paths, diff command,
consistency), the `schema_version` contract, and the REQ-007 size
invariant stated as a contract rather than an implementation detail.
Update the "skills layer reads" prose that currently describes
personas reading SPEC.md and TASKS.md in full (the persona phase
prose around `docs/ARCHITECTURE.md:348-497`) to describe the bundle
entry read instead, so no section still presents full-file persona
entry reads as the current contract.

<task-scenarios>
Given the updated ARCHITECTURE.md,
when the reviewer checks the CLI surface table and the persona
read-contract prose,
then `speccy context` is documented with its envelope and the size
invariant, and no stale full-file entry-read contract remains
(content check by reviewer, not substring assertion) (CHK-013).

Suggested files: `docs/ARCHITECTURE.md`
</task-scenarios>
</task>
</content>
</invoke>
