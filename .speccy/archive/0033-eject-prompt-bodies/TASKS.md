---
spec: SPEC-0033
spec_hash_at_generation: e9ff3d3f140629d871646462dd086bdcfb8ccc5ffb57e1cabd47b45279739205
generated_at: 2026-05-20T19:00:01Z
---

# Tasks: SPEC-0033 Eject phase prompt bodies into skill files; CLI does state only, no natural-text rendering


## Phase 1: CLI surface cleanup — delete prompt-rendering commands and trim mechanism

<task id="T-001" state="completed" covers="REQ-001">
## Delete the five prompt-rendering CLI commands and the trim_to_budget mechanism

Remove `plan`, `tasks` (render form), `implement`, `review`, and `report` from the
`clap` Command enum and from `main.rs` dispatch. Delete the corresponding module
files (`speccy-cli/src/plan.rs`, `speccy-cli/src/implement.rs`,
`speccy-cli/src/review.rs`, `speccy-cli/src/report.rs`, `speccy-cli/src/tasks.rs`)
and the `speccy-core/src/prompt/budget.rs` module along with its tests and every
caller site that wires it. Remove the `resources/modules/prompts/` directory (all
embedded phase-prompt and reviewer-prompt templates). Remove the template loader and
substitution helpers in `speccy-core` if no remaining caller consumes them after
removing the prompt commands. The hash-record logic from `tasks --commit` stays alive
— it will migrate to `speccy-cli/src/lock.rs` in T-002.

<task-scenarios>

Given a freshly compiled `speccy-cli` binary after this task lands,
when `speccy --help` runs,
then stdout does not list `plan`, `tasks`, `implement`, `review`, or `report` as
subcommands, and each of those five names returns a clap "unrecognized subcommand"
error when invoked directly.

Given the post-task workspace source tree,
when a recursive symbol search runs for `trim_to_budget`, `TrimResult`,
`DEFAULT_BUDGET`, and `budget.rs` across all non-deleted files,
then zero hits are returned — the symbols and the module are fully removed,
not merely unused.

Given the workspace after this task,
when the path `resources/modules/prompts/` is stat'd or listed,
then the directory does not exist.

Given `cargo test --workspace` run against the post-deletion build,
then all tests pass and `cargo clippy --workspace --all-targets
--all-features -- -D warnings` exits 0 (no orphaned imports or dead code
warnings from the deletions).

</task-scenarios>

Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/plan.rs`, `speccy-cli/src/tasks.rs`, `speccy-cli/src/implement.rs`, `speccy-cli/src/review.rs`, `speccy-cli/src/report.rs`, `speccy-core/src/prompt/budget.rs`, `speccy-core/src/prompt/template.rs`, `resources/modules/prompts/`
</task>

## Phase 2: New CLI verbs — `speccy lock` and `speccy vacancy`

<task id="T-002" state="completed" covers="REQ-002">
## Add `speccy lock SPEC-NNNN` command

Create `speccy-cli/src/lock.rs` exporting a `run(args: LockArgs, cwd: &Utf8Path) -> Result<(), LockError>` function.
Wire a `Lock { spec_id: String }` variant into the `Command` enum and `main.rs` dispatch.
The implementation resolves the spec directory, validates that SPEC.md and TASKS.md
both exist and parse, computes the SPEC.md sha256 hash plus current UTC timestamp,
and rewrites TASKS.md frontmatter (`spec_hash_at_generation`, `generated_at`) preserving
body bytes. Reuse `speccy_core::tasks::commit_frontmatter` (or its equivalent) for the
hash-and-rewrite logic — no re-implementation. On any precondition failure the command
exits non-zero with a stderr message and leaves TASKS.md unmodified.

<task-scenarios>

Given a tempdir workspace containing a valid SPEC.md and a TASKS.md with
`spec_hash_at_generation: bootstrap-pending`,
when `speccy lock SPEC-0001` runs,
then the process exits 0 and the rewritten TASKS.md frontmatter carries the SPEC.md
sha256 in `spec_hash_at_generation` plus a UTC `generated_at` field of RFC-3339 shape.

Given a tempdir workspace where SPEC-9999 does not exist under `.speccy/specs/`,
when `speccy lock SPEC-9999` runs,
then the process exits 1 and stderr contains the substring `SPEC-9999` and `not found`.

Given a tempdir workspace where SPEC-0001's SPEC.md is missing the required `id`
frontmatter field,
when `speccy lock SPEC-0001` runs,
then the process exits 1, stderr names the parse failure, and TASKS.md is
byte-identical to its pre-invocation state (no partial write).

Given `speccy --help` after adding the new command,
then `lock` appears in the listed subcommands.

</task-scenarios>

Suggested files: `speccy-cli/src/lock.rs`, `speccy-cli/src/main.rs`, `speccy-core/src/tasks.rs` (commit_frontmatter reuse)
</task>

<task id="T-003" state="completed" covers="REQ-003">
## Add `speccy vacancy [--json]` command

Create `speccy-cli/src/vacancy.rs` exporting a `run` function that walks
`.speccy/specs/` (flat slug directories plus one level of mission folders), finds
the highest existing SPEC-NNNN, and returns the next ID. Wire a
`Vacancy { json: bool }` variant into the `Command` enum and `main.rs` dispatch.
Text output is the bare ID string (`SPEC-NNNN\n`); `--json` output is
`{"schema_version":1,"next_spec_id":"SPEC-NNNN"}\n`. The command performs no
filesystem writes. Reuse `speccy_core::prompt::allocate_next_spec_id` (or its
successor after REQ-001 cleanup) — the ID-walk logic is not re-implemented.
Resolve the open question from the SPEC about whether to relocate the function to
a more general module (e.g. `speccy_core::specs::next_id`).

<task-scenarios>

Given a tempdir workspace with `.speccy/specs/` containing directories
`0001-foo/`, `0027-bar/`, `0032-baz/`, and a mission folder `auth/0033-signup/`,
when `speccy vacancy --json` runs with cwd at the workspace root,
then stdout exactly equals `{"schema_version":1,"next_spec_id":"SPEC-0034"}\n`
and the process exits 0.

Given a tempdir workspace with an empty `.speccy/specs/` directory,
when `speccy vacancy` runs (text form),
then stdout is `SPEC-0001\n` and the process exits 0.

Given a tempdir with no `.speccy/` directory anywhere in the cwd ancestry,
when `speccy vacancy` runs,
then stdout is empty, the process exits 1, and stderr contains the substring
`.speccy/ directory not found`.

Given `speccy --help` after adding the new command,
then `vacancy` appears in the listed subcommands alongside `lock`.

</task-scenarios>

Suggested files: `speccy-cli/src/vacancy.rs`, `speccy-cli/src/main.rs`, `speccy-core/src/prompt/` or `speccy-core/src/specs/` (allocate_next_spec_id reuse/relocation)
</task>

## Phase 3: `speccy next` simplification and schema_version 2 envelopes

<task id="T-004" state="completed" covers="REQ-004">
## Drop `--kind` from `speccy next`; implement derived action-kind logic

Remove the `kind: Option<String>` field from the `Next` variant in `Command` and
the `KindFilter` type (and any filtering logic consuming it) from `speccy_core`.
Add a `spec_id: Option<String>` positional to enable the per-spec form
(`speccy next SPEC-NNNN`). Implement the priority rule
`decompose > review > implement > ship` based on on-disk artifact state: if
TASKS.md is absent, kind = `"decompose"`; else if any task is `state="in-review"`,
kind = `"review"` (with the in-review task_id); else if any task is
`state="pending"`, kind = `"implement"` (with the first pending task_id); else if
all tasks are `state="completed"` and REPORT.md is absent, kind = `"ship"`; else
kind = `null` (completed/superseded, omit from workspace listing). The per-spec
form (`speccy next SPEC-NNNN`) returns one entry or
`{ "next_action": null, "reason": "completed" | "superseded" }`.

<task-scenarios>

Given a tempdir workspace where SPEC-0001's TASKS.md contains one
`<task id="T-002" state="in-review">`, one `<task id="T-001" state="completed">`,
and one `<task id="T-003" state="pending">`,
when `speccy next SPEC-0001 --json` runs,
then the JSON output's `next_action` field equals
`{"kind":"review","task_id":"T-002"}`.

Given the same SPEC-0001 after the in-review task transitions to `state="completed"`,
when `speccy next SPEC-0001 --json` runs,
then `next_action.kind` equals `"implement"` and `next_action.task_id` is `"T-003"`.

Given a tempdir workspace containing only SPEC-0002 with SPEC.md present, no TASKS.md,
no REPORT.md, when `speccy next` runs (workspace form, text output),
then stdout contains exactly one line referencing SPEC-0002 with action kind
`decompose` and no task_id.

Given a tempdir workspace where SPEC-0003 has every task `state="completed"` and
a REPORT.md present, when `speccy next SPEC-0003 --json` runs,
then `next_action` is `null` and `reason` is `"completed"`, and when
`speccy next` (workspace form) runs then SPEC-0003 is omitted from the listing.

Given `speccy next --kind implement` is attempted on the new binary,
then clap returns an "unexpected argument `--kind`" error.

</task-scenarios>

Suggested files: `speccy-cli/src/main.rs`, `speccy-cli/src/next.rs`, `speccy-cli/src/next_output.rs`, `speccy-core/src/next/` (KindFilter removal, kind derivation)
</task>

<task id="T-005" state="completed" covers="REQ-005">
## Bump `speccy status` and `speccy next` JSON envelopes to schema_version 2 with resolved paths

Change `schema_version` from `1` to `2` in both `speccy status --json` and
`speccy next --json` envelopes. Add `spec_md_path`, `tasks_md_path` (nullable),
and `mission_md_path` (nullable) to every per-spec object in both envelopes.
Paths are repo-relative forward-slash strings (e.g., `.speccy/specs/0031-foo/SPEC.md`).
`speccy next --json` entries additionally carry `next_action: { kind, task_id? }`
per the derived logic from T-004. Reuse the existing `speccy_core::workspace` scanner
for path resolution; no new path-discovery code in the JSON-serialization layer.

<task-scenarios>

Given a tempdir workspace with one flat spec at `.speccy/specs/0031-foo/` containing
valid SPEC.md and TASKS.md, when `speccy status SPEC-0031 --json` runs,
then the JSON output's `schema_version` field equals `2` and the per-spec entry
carries `"spec_md_path": ".speccy/specs/0031-foo/SPEC.md"`,
`"tasks_md_path": ".speccy/specs/0031-foo/TASKS.md"`, and `"mission_md_path": null`.

Given a tempdir workspace where SPEC-0040 lives under `.speccy/specs/auth/0040-signup/`
and `.speccy/specs/auth/MISSION.md` exists, when `speccy next SPEC-0040 --json` runs,
then the resulting envelope's `mission_md_path` equals `.speccy/specs/auth/MISSION.md`.

Given a tempdir workspace where SPEC-0032 has SPEC.md but no TASKS.md,
when `speccy next SPEC-0032 --json` runs,
then the per-spec object's `tasks_md_path` equals `null` and
`next_action.kind` equals `"decompose"`.

Given `speccy status --json` run on any workspace with specs,
then every per-spec entry in the JSON has `schema_version: 2` at the envelope level.

</task-scenarios>

Suggested files: `speccy-cli/src/status.rs`, `speccy-cli/src/status_output.rs`, `speccy-cli/src/next.rs`, `speccy-cli/src/next_output.rs`, `speccy-core/src/workspace.rs`
</task>

## Phase 4: Resource authoring — shared persona snippets and REQ-008 compliance

<task id="T-006" state="completed" covers="REQ-007">
## Factor reviewer persona shared blocks into co-located topic-named snippet files

Inspect the six reviewer persona body files under `resources/modules/personas/`
and identify the blocks that recur verbatim: the verdict-return contract,
the "do not edit TASKS.md" prohibition, the inline note format template,
and the diff-fetch command boilerplate. Extract each block into a topic-named
snippet file co-located inside `resources/modules/personas/` (e.g.,
`verdict_return_contract.md`, `no_tasks_md_writes.md`, `inline_note_format.md`,
`diff_fetch_command.md`). Update each persona body file to `{% include %}` the
snippets it needs; confirm the `reviewer-style` persona retains its
"Diff-format pitfalls" section and `reviewer-tests` retains its Evidence-read step
and other non-shared per-persona content. The renderer logic that walks the six
persona bodies must filter on the `reviewer-<persona>.md` filename pattern so the
snippet files are not treated as eject targets. No `_partials/` subdirectory.

<task-scenarios>

Given the post-task source tree, when `resources/modules/personas/` is listed,
then exactly six files matching `reviewer-<persona>.md` are present plus N
topic-named snippet files (none matching `reviewer-<persona>.md`), and no
`_partials/` directory exists.

Given the post-task source tree, when each of the six persona body files is parsed
for `{% include %}` directives referencing the `verdict_return_contract.md` snippet,
then each persona body contains the include directive exactly once.

Given the post-task source tree, when `reviewer-style.md` is read,
then it still contains its "Diff-format pitfalls" section (not moved to a snippet);
when `reviewer-tests.md` is read, then it still contains its Evidence-read step.

Given the post-task source tree, when a search runs for any file named
`reviewer.md.j2` or similar master-template file,
then zero matches are found.

</task-scenarios>

Suggested files: `resources/modules/personas/reviewer-business.md`, `resources/modules/personas/reviewer-tests.md`, `resources/modules/personas/reviewer-architecture.md`, `resources/modules/personas/reviewer-security.md`, `resources/modules/personas/reviewer-style.md`, `resources/modules/personas/reviewer-docs.md`, `resources/modules/personas/verdict_return_contract.md` (new), `resources/modules/personas/no_tasks_md_writes.md` (new), `resources/modules/personas/inline_note_format.md` (new), `resources/modules/personas/diff_fetch_command.md` (new)
</task>

<task id="T-007" state="completed" covers="REQ-008">
## Audit and update skill/phase bodies to discover speccy resources via CLI JSON envelopes only

Inspect all skill body files under `resources/modules/skills/` and all phase body
files under `resources/modules/phases/` for speccy-resource discovery patterns:
glob expressions like `.speccy/specs/*`; raw filesystem paths ending in `SPEC.md`,
`TASKS.md`, `MISSION.md`, or `REPORT.md` that are not bound to a `{{ ... }}`
template placeholder; directory-enumeration instructions targeting `.speccy/specs/`.
Replace any direct-discovery references with calls to the appropriate CLI JSON
envelopes (`speccy status --json`, `speccy next --json`, `speccy vacancy --json`),
or with `{{ ... }}` template placeholders wired to those envelopes. Verify that the
`speccy-plan` skill body invokes `speccy vacancy --json` (not `speccy status --json`)
to fetch the next SPEC ID in the greenfield form. General-purpose Read/Glob/grep
references for non-speccy project files (AGENTS.md, Cargo.toml, source code, etc.)
are NOT violations and must not be removed.

<task-scenarios>

Given the post-task source tree, when a recursive search runs across
`resources/modules/skills/`, `resources/modules/phases/`, and
`resources/modules/personas/` for speccy-resource discovery patterns
(`.speccy/specs/*` glob expressions; raw paths ending in `SPEC.md` / `TASKS.md` /
`MISSION.md` / `REPORT.md` not bound to a `{{ ... }}` placeholder;
directory-enumeration instructions targeting `.speccy/specs/`),
then zero matches appear in skill or agent body content.

Given the post-task `resources/modules/skills/speccy-plan.md` file,
when its body is parsed for command invocations in the greenfield form,
then it invokes `speccy vacancy --json` to learn the next SPEC ID, not
`speccy status --json`.

Given the post-task source tree, when a search runs for Read/Glob/grep references
against non-speccy project files (e.g., "read AGENTS.md", "grep for an existing
helper"),
then matches DO appear in skill and agent body content and are not considered
violations (the boundary is speccy-resource-scoped, not blanket filesystem access).

</task-scenarios>

Suggested files: `resources/modules/skills/speccy-plan.md`, `resources/modules/skills/speccy-amend.md`, `resources/modules/skills/speccy-brainstorm.md`, `resources/modules/skills/speccy-review.md`, `resources/modules/skills/speccy-init.md`, `resources/modules/phases/speccy-tasks.md`, `resources/modules/phases/speccy-work.md`, `resources/modules/phases/speccy-ship.md`
</task>

## Phase 5: `speccy init` eject redesign — three-way classification and phase body ejection

<task id="T-008" state="completed" covers="REQ-006">
## Implement three-way init classification (replace Skip-on-exists) and eject interactive skill bodies

Replace the Skip-on-exists semantic in `speccy init` with the three-way
per-file classification: (1) target absent → write and log `created`;
(2) target byte-identical to planned content → no-op and log `unchanged`;
(3) target exists and differs → refuse the entire batch atomically (no partial
writes) with stderr naming the differing file(s) and the `--force` override.
Under `--force`, classification (3) writes the file and logs `(!) overwritten`
instead of refusing. Files byte-identical under `--force` still log `unchanged`
(not `(!) overwritten`). Add the interactive skill bodies — full-body SKILL.md for
`speccy-init`, `speccy-brainstorm`, `speccy-plan`, `speccy-amend`, and
`speccy-review` — to the eject plan, sourced from `resources/modules/skills/`
and rendered via MiniJinja `{% include %}` for any shared snippets. Confirm no
MiniJinja markup survives in ejected files (all `{{ }}` and `{% %}` are expanded
at build/render time).

<task-scenarios>

Given an empty tempdir, when `speccy init --host claude-code` runs,
then `.claude/skills/speccy-plan/SKILL.md` is created and its content contains
substantive prompt body with no MiniJinja template syntax (no `{{`, `{%`, `{#`
substrings).

Given a tempdir workspace where every file `speccy init --host claude-code` would
write already exists on disk byte-identical to the planned content,
when `speccy init --host claude-code` runs (no `--force`),
then the process exits 0, stdout logs every file as `unchanged`, and no writes
occur (verified by no mtime change on planned targets).

Given a tempdir workspace where one shipped file (`.claude/skills/speccy-plan/SKILL.md`)
has a user-appended line of custom prose, making it differ from the planned write,
when `speccy init --host claude-code` runs without `--force`,
then the process exits non-zero, stderr names the differing file path and the
`--force` override, the offending file is byte-identical to its pre-invocation state,
and no other planned target was written (atomic batch refuse).

Given the same tempdir workspace, when `speccy init --force --host claude-code` runs,
then the process exits 0, the differing file is overwritten with the planned content,
stdout logs it as `(!) overwritten` with the warning marker, and every other
already-identical file is logged `unchanged` (not `(!) overwritten`).

</task-scenarios>

Suggested files: `speccy-cli/src/init.rs`, `resources/modules/skills/speccy-plan.md`, `resources/modules/skills/speccy-brainstorm.md`, `resources/modules/skills/speccy-amend.md`, `resources/modules/skills/speccy-review.md`, `resources/modules/skills/speccy-init.md`
</task>

<task id="T-009" state="completed" covers="REQ-006">
## Eject pinned phase-worker agent files and thin SKILL.md stubs at `speccy init`

Add to the `speccy init` eject plan: for each of the three pinned phase workers
(`speccy-tasks`, `speccy-work`, `speccy-ship`), eject a thin SKILL.md stub
(≤10 non-blank lines, no `context:`, `agent:`, `model:`, or `effort:` frontmatter,
naming the matching agent file path and the `/agent speccy-<phase>` invocation
pattern) plus a full-body agent file at `.claude/agents/speccy-<phase>.md` with
`model: sonnet[1m]` and `effort: medium` frontmatter and the phase body sourced
from `resources/modules/phases/speccy-<phase>.md` via MiniJinja `{% include %}`.
Eject matching Codex TOMLs at `.codex/agents/speccy-<phase>.toml` with
`model = "gpt-5.5"` and `model_reasoning_effort = "medium"`. Confirm no
`.claude/agents/speccy-init.md`, no `.claude/agents/speccy-review.md`, and no
Codex equivalents are created (both `speccy-init` and `speccy-review` are
interactive skills — no agent counterpart per DEC-008). Also eject the six
reviewer subagent body files per SPEC-0027's contract with the SPEC-0032
per-persona pins (they are subject to the new three-way classification from T-008).

<task-scenarios>

Given a freshly initialized tempdir workspace (`speccy init --host claude-code` run once),
when `.claude/skills/speccy-work/SKILL.md` is read,
then it is a thin stub of ≤10 non-blank lines with no `context:`, `agent:`, `model:`,
or `effort:` frontmatter, naming `.claude/agents/speccy-work.md` and the
`/agent speccy-work` invocation path.

Given the same tempdir, when `.claude/agents/speccy-work.md` is read,
then it contains `model: sonnet[1m]` and `effort: medium` frontmatter plus the
full phase body (not just a stub), and contains no MiniJinja markup.

Given a freshly initialized tempdir workspace (`speccy init --host codex` run once),
when `.agents/skills/speccy-work/SKILL.md` is read and `.codex/agents/speccy-work.toml`
is read, then the stub names `.codex/agents/speccy-work.toml` and the
`/agent speccy-work` invocation path; the TOML contains
`model = "gpt-5.5"` and `model_reasoning_effort = "medium"` at the document top level.

Given a freshly initialized tempdir workspace where both
`speccy init --host claude-code` and `speccy init --host codex` have run,
when the workspace is scanned for `.claude/agents/speccy-init.md`,
`.claude/agents/speccy-review.md`, `.codex/agents/speccy-init.toml`, and
`.codex/agents/speccy-review.toml`, then zero matches are returned.

</task-scenarios>

Suggested files: `speccy-cli/src/init.rs`, `resources/modules/phases/speccy-tasks.md`, `resources/modules/phases/speccy-work.md`, `resources/modules/phases/speccy-ship.md`, `resources/agents/.claude/agents/speccy-tasks.md.tmpl` (new), `resources/agents/.claude/agents/speccy-work.md.tmpl` (new), `resources/agents/.claude/agents/speccy-ship.md.tmpl` (new), `resources/agents/.codex/agents/speccy-tasks.toml.tmpl` (new)
</task>

## Phase 6: Workspace migration and final verification

<task id="T-010" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005 REQ-006 REQ-007 REQ-008">
## Migrate the dogfooded `.speccy/` workspace and verify the final seven-verb CLI surface

Perform the hand migration of the dogfooded `.speccy/` workspace to work with the
new CLI surface: replace any existing `speccy tasks SPEC-NNNN --commit` invocations
in skill bodies or documentation with `speccy lock SPEC-NNNN`; remove references to
the deleted verbs; run `speccy lock` on each active SPEC to re-record hashes via the
new command. Run the full hygiene suite (`cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check`) and confirm `speccy --help`
lists exactly seven subcommands: `init`, `status`, `next`, `check`, `verify`,
`lock`, `vacancy` with no others present. Run `speccy verify` as a final CI
dry-run to confirm proof shape is intact.

<task-scenarios>

Given the fully-built `speccy` binary after all prior tasks complete,
when `speccy --help` runs,
then stdout lists exactly the seven subcommands `init`, `status`, `next`, `check`,
`verify`, `lock`, `vacancy` and contains no reference to `plan`, `tasks`,
`implement`, `review`, or `report`.

Given the post-migration workspace, when `cargo test --workspace` runs,
then all tests pass; when `cargo clippy --workspace --all-targets --all-features
-- -D warnings` runs, then it exits 0.

Given the post-migration workspace, when `speccy verify` runs,
then it exits 0 (proof shape intact, no broken checks).

Given the post-migration workspace skill files, when a search runs for
the old command pattern `speccy tasks.*--commit`,
then zero matches are found in any skill body or documentation file.

</task-scenarios>

Suggested files: `.speccy/specs/*/TASKS.md` (hash re-record via `speccy lock`), skill bodies referencing old CLI verbs, `AGENTS.md` if it references deleted verbs
</task>

