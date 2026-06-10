---
spec: SPEC-0056
spec_hash_at_generation: b54cc204a49f72a083ff9874c06c88a64f59a73ababeacb3700d16909e2c5f61
generated_at: 2026-06-10T20:24:43Z
---
# Tasks: SPEC-0056 Task-scoped context bundle — `speccy context` emits one JSON read for loop subagents

<task id="T-001" state="completed" covers="REQ-003">
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

<task id="T-002" state="completed" covers="REQ-001 REQ-002">
## Add the `speccy context <task-selector> --json` command emitting identity + intent

Add `Command::Context { selector: String, json: bool }` to the clap
enum in `speccy-cli/src/main.rs:60-133` and wire its dispatch arm in
`main.rs:140-170`, following the existing command shape. Resolve the
selector through `task_lookup::parse_ref` then `task_lookup::find`
(accepting `T-NNN` and `SPEC-NNNN/T-NNN`), reusing SPEC-0055's shared
`report_lookup_error` helper (`speccy-cli/src/main.rs:664-695`) for the
`LookupError` → exit-code/diagnostic rendering. That helper already
backs `task transition`, `journal append`, and `journal show`, and
covers InvalidFormat, NotFound, and Ambiguous; routing `context`
through it keeps the diagnostic CLASS at parity with `speccy check`
(the contract) via the DRY path rather than re-deriving an inline
mapping. Selector failures exit non-zero with no partial stdout. Without `--json` the
command renders the same bundle content in a human-readable text
form — `--json` toggles representation, never content, per the
workspace-wide convention (the text form needs no stability
guarantee; agents always pass `--json`).

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

<task id="T-003" state="completed" covers="REQ-003">
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

<task id="T-004" state="completed" covers="REQ-004">
## Inline the full per-task journal; absence is an explicit empty marker, not an error

Extend the envelope with a journal section. When
`<spec-dir>/journal/<task-id>.md` exists, parse it via
`journal_xml::parse` (`speccy-core/src/parse/journal_xml/mod.rs:151`)
and inline its full content: the frontmatter fields plus every
`<implementer>` / `<review>` / `<blockers>` entry across all rounds
in file order, each with its attributes (`date`, `model`, `round`,
and for review `persona`/`verdict`). Project each entry through
SPEC-0055's public `journal show` block structs — `JsonJournalBlock`
in `speccy-cli/src/journal_show_output.rs:125` (and its
`to_json_journal_block` mapping) — rather than re-deriving a parallel
journal-to-JSON shape, so `context` and `journal show` cannot drift.
This is the same anti-drift discipline DEC-002 applies to
`check`/`context`. Caveat: do NOT nest the standalone
`JsonTaskJournal` envelope (`journal_show_output.rs:78`) wholesale —
its `schema_version` belongs to the standalone `journal show` command
and would collide with the bundle's own `schema_version`. Reuse the
block structs plus the frontmatter fields only. The journal content
must be sufficient for retry context — prior implementer handoffs,
review verdicts, and blockers directives all present. When the journal
file does not exist, the envelope carries an explicit empty-journal
marker (e.g. an `exists: false` field with zero entries) and the
command still exits 0: a round-1 implementer legitimately has no
journal yet (DEC-004).

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

<task id="T-005" state="completed" covers="REQ-005">
## Add the sibling index, file paths, and a suggested merge-base diff command

Extend the envelope with three navigation aids. (1) A sibling-task
index: every other task in the spec as id, state, and covers only —
never any body text — sourced from the spec's `TasksDoc` tasks
(`task_xml/mod.rs:55-135`), excluding the selected task. (2)
Repo-relative paths to SPEC.md, TASKS.md, and the task's journal file
for follow-up targeted reads, resolved from the spec directory. (3) A
suggested diff command string in merge-base form against the repo's
default branch, computed by the CLI from git state and runnable
as-is from the repo root. This needs NET-NEW git machinery: no
merge-base probe exists today. `speccy-cli/src/git.rs` exposes only
`repo_sha` (`git.rs:21`), and the consistency probe correlates tasks
to commits by commit-title prefix
(`first_commit_sha_with_title_prefix`, in
`speccy-core/src/consistency.rs:177` — NOT git.rs and never by
merge-base); nothing today computes the default branch. Add a
best-effort default-branch probe (e.g. `git symbolic-ref
refs/remotes/origin/HEAD`, falling back to `main`) plus a `git
merge-base` call to `git.rs`, following its existing non-fatal
shell-out convention (`git.rs:1-22`) — git unavailability degrades
the diff-command field, never errors the bundle.

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

<task id="T-006" state="completed" covers="REQ-006">
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

PRECONDITION — satisfied. SPEC-0055 (lifecycle-write-commands) has
merged ahead of this SPEC (commit dff9f33), so the sequencing risk
the original draft guarded against is resolved: this edit lands on
top of the post-0055 fan-out/persona modules, which already route
journal writes through `speccy journal append` and read-backs through
`speccy journal show`. Keep SPEC-0055's append/verdict/commit contract
in those files intact — this task touches only the entry-read step,
leaving the journal-write and commit prose untouched.

Edit the host-neutral source modules under `resources/modules/`
(never the ejected `.claude/`, `.agents/`, `.codex/` copies — see
AGENTS.md "Skill pack source of truth"). The PRIMARY reviewer edit
target is the shared fan-out spawn prompt
`resources/modules/skills/partials/review-fanout.md`: its spawn
prompt today instructs each persona to run `speccy check
SPEC-NNNN/T-NNN`, read the bare `<task>` body in TASKS.md, and read
the journal file — rewrite that entry-read step to dispatch a single
`speccy context SPEC-NNNN/T-NNN --json` call and reference the bundle
fields (task, requirements, scenarios, journal, diff command). The
six reviewer persona bodies carry only abstract "read the SPEC, the
diff, and implementer notes" framing and need no per-file-read
removal — do NOT touch them for entry-scoping. For the implementer,
the `speccy-work` phase opens its per-task context read with `speccy
context`; on the no-selector path `speccy next --json` still runs
first to resolve the task (the selector is unknown until then), and
the retry-shape rule (today a pure journal-file read) reads its
journal from the bundle instead.

Carve-outs to honour: (1) do NOT remove `reviewer-tests`'s caveat
that `speccy check` exit codes are not test evidence
(`resources/modules/personas/reviewer-tests.md:37`) — that is not an
entry-read instruction, and `speccy check` still exists. (2) Leave
vet persona modules byte-identical. (3) Follow-up targeted reads via
the bundle's listed paths (e.g. the evidence file) remain legitimate
where a role needs them. Run `just reeject` to regenerate both host
packs and confirm a clean tree against the committed ejections.

<task-scenarios>
Given the updated module sources,
when the reviewer reads the fan-out spawn prompt
(`review-fanout.md`) and the implementer phase module,
then each opens its per-task read with the `speccy context` call and
neither instructs a full SPEC.md / TASKS.md read or a `speccy check`
entry call as entry context, while `reviewer-tests`'s `speccy check`
exit-code caveat remains intact (content check by reviewer, not
substring assertion) (CHK-012).

Given a clean checkout after this task lands,
when `just reeject` runs and `git status --porcelain` is checked,
then the working tree is clean, proving the committed ejections match
the updated sources, and the vet persona bodies are unchanged from
the prior commit (CHK-011).

Suggested files:
`resources/modules/skills/partials/review-fanout.md` (primary
reviewer entry-read target),
`resources/modules/phases/speccy-work.md` (implementer entry read +
retry-shape rule)
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
