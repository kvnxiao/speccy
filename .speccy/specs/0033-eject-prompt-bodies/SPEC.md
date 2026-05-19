---
id: SPEC-0033
slug: eject-prompt-bodies
title: Eject phase prompt bodies into skill files; CLI does state only, no natural-text rendering
status: in-progress
created: 2026-05-19
supersedes: []
---

# SPEC-0033: Eject phase prompt bodies into skill files; CLI does state only, no natural-text rendering

## Summary

The CLI today does two architecturally different jobs. The first is
mechanical: discovering workspaces, parsing artifacts, resolving
paths, recording hashes, running proof-shape lints. The second is
authoring-adjacent: rendering five phase prompts (`speccy plan`,
`speccy tasks` render-form, `speccy implement`, `speccy review`,
`speccy report`) by substituting variables into embedded markdown
templates and then mutating the rendered text via `trim_to_budget`
when it exceeds an 80,000-character budget. Both halves ship as one
binary; both are described as "deterministic core" in
`AGENTS.md`'s core principles.

The second half doesn't honor the principle. Rendering authored
natural text from CLI-embedded templates means the prompt the agent
reads is shaped at runtime by code in the binary rather than by
content in files the user can see and edit. `trim_to_budget` then
silently mutates that text further by string-matching on
heading-name conventions (`## Notes`, `## Other specs`, answered
`- [x]` items, old `## Changelog` rows): a hidden contract between
template authors and the trim engine that template readers cannot
see. Both behaviors run counter to "intelligent edges, deterministic
core" — they put intelligence (prompt body, trim heuristics) inside
the core.

SPEC-0023 (REQ-005, REQ-006) and SPEC-0027 have already partially
walked this back: AGENTS.md, SPEC.md, MISSION.md, and persona body
content stopped being inlined into rendered prompts when host
machinery delivers them via another channel (`Read` primitive,
sub-agent system context). This SPEC extends that direction to its
natural endpoint: the CLI carries no natural-text prompt bodies at
all. Phase prompts become ejected files the user owns after
`speccy init`. The CLI's prompt-related surface area shrinks to
structured state — IDs, paths, derived action kinds — emitted as
JSON envelopes that skills consume.

The CLI surface drops five verbs (`plan`, `tasks` render-form,
`implement`, `review`, `report`), drops the `trim_to_budget` module,
and gains two narrowly-scoped verbs that have always been "real CLI
work" hiding inside prompt-rendering paths: `speccy lock` (records
SPEC.md hash into TASKS.md frontmatter, formerly `tasks --commit`)
and `speccy vacancy` (returns the next free SPEC-NNNN, formerly the
ID-allocation side effect of `speccy plan` greenfield rendering).
`speccy next` is simplified — its `--kind` flag goes away because
the kind of next action is fully derived from spec state, not
user-supplied. The `speccy status` and `speccy next` JSON envelopes
bump to `schema_version: 2` and carry resolved paths plus the
derived `next_action`, so skills no longer need filesystem-globbing
logic of their own.

The final surface is seven flat commands, each with one job. All
phase prompts live in `.claude/skills/speccy-<verb>/SKILL.md` (and
Codex equivalents) as Skip-on-exists files; users edit them freely
after init and the trade-off — re-eject + manual merge to pick up
upstream changes — is acknowledged out of scope per the
`AGENTS.md` quality bar ("useful for my next greenfield").

SPEC-0032 is a hard sequencing predecessor. It introduces per-phase
model and effort pinning across the lifecycle, which lands as new
frontmatter on five Claude Code skills (`context: fork` plus an
`agent:` target on `speccy-tasks` / `speccy-work` / `speccy-ship` /
`speccy-init`; direct `model: haiku` on `speccy-review`), four new
Claude Code subagent files under `.claude/agents/speccy-<phase>.md`,
five new Codex agent TOML files under `.codex/agents/speccy-<phase>.toml`,
and one-line Codex SKILL.md pointer additions naming the `/agent
<name>` invocation path. This SPEC inherits that frontmatter and
file set wholesale: the MiniJinja templates that produce the ejected
SKILL.md files carry SPEC-0032's frontmatter unchanged, the new
subagent files join REQ-006's Skip-on-exists ejection set, and the
shared body source at `resources/modules/skills/<phase>.md` is
included by both the SKILL.md template and the subagent body
template so a single source of phase-body content feeds both
consumers.

## Goals

<goals>
- The CLI no longer renders natural-text prompt bodies. Five
  prompt-rendering commands are deleted: `speccy plan`, `speccy
  tasks` (render form), `speccy implement`, `speccy review`,
  `speccy report`. Their command modules
  (`speccy-cli/src/{plan,tasks,implement,review,report}.rs`),
  their associated prompt templates under
  `resources/modules/prompts/`, and any speccy-core support code
  that exists solely to feed them (e.g. template loader,
  substitution helpers, persona resolver) are removed.
- The `trim_to_budget` mechanism is deleted entirely. The
  `speccy-core/src/prompt/budget.rs` module, the `DEFAULT_BUDGET`
  constant, the `TrimResult` type, the `dropped`-vec plumbing
  through caller sites, and all associated tests are removed. The
  hidden contract that prompts must name sections `## Notes` /
  `## Other specs` to opt into being droppable goes away with it.
- Phase prompt bodies eject into the host skill pack as
  Skip-on-exists files. After `speccy init`, every shipped skill
  (`speccy-plan`, `speccy-tasks`, `speccy-work`, `speccy-review`,
  `speccy-ship`, `speccy-amend`, `speccy-brainstorm`, `speccy-init`)
  carries its full prompt body inline in its SKILL.md. `speccy init
  --force` does not overwrite these files; users who customize
  them keep their edits across re-init.
- The CLI surface is exactly seven verbs: `init`, `status`,
  `next`, `check`, `verify`, plus two new flat verbs `lock` and
  `vacancy`. No mode flags select between content types; each verb
  has one job.
- `speccy lock SPEC-NNNN` is the canonical hash-record verb. It
  takes the work that lived behind `speccy tasks SPEC-NNNN --commit`
  (write current SPEC.md sha256 + UTC timestamp into TASKS.md
  frontmatter) and exposes it as its own top-level command.
- `speccy vacancy [--json]` is the canonical "next free SPEC ID"
  query. Text output is the bare SPEC-NNNN string; `--json` output
  is `{ schema_version: 1, next_spec_id: "SPEC-NNNN" }`. The
  command does not write any files.
- `speccy next` drops `--kind`. The action kind is derived from
  spec state via the priority rule `review > implement > ship`
  (with `decompose` when TASKS.md is absent), so user-supplied
  kind filtering is redundant. The workspace form lists every
  active spec with its derived `next_action`; the per-spec form
  returns one entry or `null + reason` for completed/superseded
  specs.
- `speccy status` and `speccy next` JSON envelopes bump to
  `schema_version: 2`. Per-spec entries carry `spec_md_path`,
  `tasks_md_path`, `mission_md_path` (nullable). `speccy next`
  entries additionally carry `next_action: { kind, task_id? }`.
- All filesystem path resolution lives in the CLI. Skill agents
  read paths from JSON envelopes; they do not glob
  `.speccy/specs/` themselves. The slug-pattern rule for spec
  directories (`NNNN-slug`, optionally one level of mission
  folder) is enforced by the CLI as the sole authority.
- Upstream skill authoring uses MiniJinja templating with
  `{% include %}` for shared snippets (SPEC pointer boilerplate,
  task-entry intro, state-attribute warning, the reviewer
  template's conditional tests block). The ejected SKILL.md files
  are plain markdown; MiniJinja is a build-time-only concern in
  `resources/modules/skills/`.
</goals>

## Non-goals

<non-goals>
- No in-CLI merge tool for upstream skill-pack updates. When
  speccy ships an improvement to a phase prompt body upstream,
  users who want it run `speccy init --force` (overwriting only
  unedited files) and manually re-apply any local edits. The
  one-time-eject trade-off is explicit per the brainstorm
  framing; speccy is not in the merge business.
- No `--strict` mode that enforces the piecewise workflow
  (implement → review → implement → review). The `next_action.kind`
  priority `review > implement > ship` is a recommendation
  surfaced in the JSON envelope, not a block. Users who want to
  implement multiple tasks before reviewing can call `speccy next
  SPEC-NNNN/T-NNN` directly to override the surfaced priority.
- No per-persona reviewer prompt ejection. Today, five of the six
  reviewer prompt templates are byte-identical; only
  `reviewer-tests` adds a single extra step (the evidence-read
  block). One shared reviewer template with a `{% if persona ==
  "tests" %}` block is the ejected shape. Per-persona prompt
  ejection is deferred until structural divergence appears.
- No new context-endpoint commands per phase (e.g.
  `speccy plan-context`, `speccy task-context`). The existing
  `speccy status --json` and `speccy next --json` envelopes carry
  enough state for every skill after the schema_version 2
  augmentation. Adding phase-named query commands would proliferate
  surface for no payload reduction.
- No backwards-compatibility shims for the deleted CLI verbs
  (`plan`, `tasks` render-form, `implement`, `review`, `report`).
  Speccy v1 is not released; there are no downstream users to
  preserve. The dogfooded `.speccy/` workspace migration is done
  by hand as part of this SPEC's implementation.
- No filesystem globbing inside skill bodies. The slug-pattern
  rule (`NNNN-slug` under `.speccy/specs/`, optionally inside one
  level of mission folder) is documented in `ARCHITECTURE.md` as
  the filesystem contract, but skills do not implement it
  themselves; they read resolved paths from JSON envelopes.
- No expansion of the `speccy lock` precondition surface beyond
  what `tasks --commit` validates today (SPEC.md exists and
  parses, TASKS.md exists and parses, then rewrite frontmatter).
  No additional consistency checks; no lock-removal verb; no
  lock-status query.
- No JSON output for `speccy lock`. The command is a write-side
  effect; success is signaled by exit code 0, failures by exit 1
  with stderr messages. Future-proofing JSON output for a
  side-effect verb has no consumer.
- No removal of `speccy check`'s scenario-text rendering. Despite
  the word "render" in its description, `check` reads
  English Given/When/Then prose from SPEC.md `<scenario>` element
  blocks and surfaces them; it does not author natural text from
  templates. This SPEC's "no CLI natural-text rendering" principle
  applies to template-substituted prompts only.
- No change to `speccy verify`. Its responsibility (proof-shape
  lint that exits non-zero on broken structure) is orthogonal to
  prompt rendering and stays.
- No structural change to the `.speccy/` layout (e.g. no rename
  of `specs/`, no new top-level directories). Phase prompt bodies
  land in the host skill pack location (`.claude/skills/speccy-<verb>/`,
  `.agents/skills/speccy-<verb>/`), not under `.speccy/`.
</non-goals>

## User Stories

<user-stories>
- As a solo developer who customized my project's
  `/speccy-plan` skill body to add a domain-specific instruction
  ("always include a `## Data migration` section when the SPEC
  touches the database schema"), I want my edit to persist across
  `speccy init --force`. The shipped SKILL.md is Skip-on-exists;
  my edit survives. To pick up upstream improvements I re-eject
  manually and re-apply, which is the documented update path.
- As an AI agent invoking `/speccy-plan` in greenfield mode, I
  want to fetch the next free SPEC ID with the smallest possible
  payload. `speccy vacancy --json` returns
  `{ schema_version: 1, next_spec_id: "SPEC-0034" }` — one query,
  one field — instead of loading every active spec's state via
  `speccy status --json`.
- As an AI agent working on a known SPEC, I want to query that
  spec's state without paying the context cost of the full
  workspace overview. `speccy status SPEC-0031 --json` returns
  just that spec's envelope; `speccy next SPEC-0031 --json`
  returns just that spec's next action. The workspace-wide forms
  are reserved for skills that don't have a target yet.
- As an AI agent driving `/speccy-work` for the next implementable
  task, I read `speccy next --json` and filter for
  `next_action.kind == "implement"`. The CLI surfaces only what's
  in-flight; the skill decides which kind matches its job. No
  `--kind` flag, no inverted "skill tells the CLI what to look
  for" contract.
- As a contributor authoring upstream skill bodies, I want to
  reuse the same boilerplate ("Before X, read SPEC.md at
  `{{spec_md_path}}` via your Read primitive") across multiple
  skills without copy-paste. MiniJinja `{% include %}` at
  `speccy init`-time renders one canonical snippet into every
  consuming SKILL.md. The user sees one plain-markdown SKILL.md
  per skill; MiniJinja is invisible after init.
- As a developer auditing the CLI surface, I want each command to
  have exactly one job. `speccy status` queries state; `speccy
  next` queries actionable; `speccy vacancy` allocates IDs;
  `speccy lock` records hashes; `speccy check` surfaces scenario
  text; `speccy verify` lints; `speccy init` scaffolds. No mode
  flag swaps a command between meanings; `--json` is the only
  flag, and it changes representation, not content.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Five prompt-rendering CLI commands and the trim mechanism are deleted

The CLI no longer renders natural-text prompt bodies. The five
commands whose sole responsibility today is rendering an embedded
markdown template — `speccy plan`, `speccy tasks` (render form),
`speccy implement`, `speccy review`, `speccy report` — are removed
from the `clap` subcommand enum, from `main.rs` dispatch, and from
the `speccy-cli` crate entirely. The `trim_to_budget` mechanism
(`speccy-core/src/prompt/budget.rs`, the `DEFAULT_BUDGET` constant,
the `TrimResult` type) is deleted along with its tests and every
caller-site that wires it.

<done-when>
- The `Command` enum in `speccy-cli/src/main.rs` no longer
  contains `Plan`, `Tasks { ..., commit: false }` (the rendering
  arm), `Implement`, `Review`, or `Report` variants.
- The files `speccy-cli/src/plan.rs`, `speccy-cli/src/implement.rs`,
  `speccy-cli/src/review.rs`, `speccy-cli/src/report.rs` are
  removed from the repository.
- `speccy-cli/src/tasks.rs` is removed entirely (the
  hash-record half migrates to `speccy-cli/src/lock.rs` under
  REQ-003).
- The file `speccy-core/src/prompt/budget.rs` is removed.
- The file `speccy-core/src/prompt/template.rs`, the `PROMPTS`
  static, and the embedded `resources/modules/prompts/`
  directory are removed if no remaining caller consumes them.
- The files under `resources/modules/prompts/` (the embedded
  phase-prompt and reviewer-prompt templates) are removed from
  the repository.
- Running `speccy --help` against the rebuilt binary lists
  exactly seven subcommands: `init`, `status`, `next`, `check`,
  `verify`, `lock`, `vacancy`.
- `cargo test --workspace` passes; `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` exits 0.
</done-when>

<behavior>
- Given a freshly built `speccy` binary, when the user runs
  `speccy plan`, then the process exits non-zero with the standard
  `clap` error "unrecognized subcommand `plan`" — there is no
  graceful-deprecation message, no shim.
- Given the same binary, when the user runs `speccy implement`,
  `speccy review`, `speccy report`, then the same `clap` error
  appears for each.
- Given the workspace source tree after this SPEC lands, when a
  contributor greps for `trim_to_budget`, then zero matches are
  found outside of removed-file deletions in git history.
- Given the workspace source tree, when a contributor lists
  `resources/modules/prompts/`, then the directory does not
  exist.
</behavior>

<scenario id="CHK-001">
Given a freshly compiled `speccy-cli` binary,
when `speccy --help` runs,
then stdout lists exactly the seven subcommands
`init`, `status`, `next`, `check`, `verify`, `lock`, `vacancy`
and contains no reference to `plan`, `tasks`, `implement`,
`review`, or `report`.
</scenario>

<scenario id="CHK-002">
Given the post-SPEC workspace source tree,
when a recursive search runs for the symbols `trim_to_budget`,
`TrimResult`, `DEFAULT_BUDGET`, and `budget.rs` across all
non-deleted files,
then zero hits are returned (the symbols and the module are
fully removed, not merely unused).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `speccy lock SPEC-NNNN` records the SPEC.md hash into TASKS.md frontmatter

A new top-level command `speccy lock SPEC-NNNN` takes the
hash-record side effect that lives today behind
`speccy tasks SPEC-NNNN --commit` and exposes it as its own verb.
The command resolves the spec directory, validates that SPEC.md and
TASKS.md both exist and parse cleanly, computes the SPEC.md sha256
hash plus current UTC timestamp, and rewrites TASKS.md frontmatter
(`spec_hash_at_generation`, `generated_at`) preserving body bytes.
On precondition failure, the command exits non-zero with a stderr
message; no partial write occurs.

<done-when>
- A new file `speccy-cli/src/lock.rs` exists, exporting a `run`
  function with the signature
  `pub fn run(args: LockArgs, cwd: &Utf8Path) -> Result<(), LockError>`.
- The `Command` enum in `speccy-cli/src/main.rs` contains a
  `Lock { spec_id: String }` variant requiring the SPEC-ID
  positional.
- The hash-and-rewrite logic reuses the existing
  `speccy_core::tasks::commit_frontmatter` function unchanged
  (the SPEC has no opinion on the core-side helper's name or
  shape; only the CLI surface moves).
- Running `speccy lock SPEC-0001` against a workspace where
  SPEC-0001's TASKS.md exists rewrites the frontmatter's
  `spec_hash_at_generation` to the current SPEC.md sha256 and
  `generated_at` to the current UTC timestamp.
- Running `speccy lock SPEC-9999` against a workspace where no
  matching directory exists exits 1 with stderr message
  `speccy lock: spec `SPEC-9999` not found under .speccy/specs/`.
- Running `speccy lock SPEC-0001` against a workspace where
  SPEC-0001's SPEC.md has a parse error exits 1 with stderr
  message naming the parse failure; TASKS.md frontmatter is
  not modified.
</done-when>

<behavior>
- Given a workspace where SPEC-0001's TASKS.md has
  `spec_hash_at_generation: bootstrap-pending` and the current
  SPEC.md hashes to `abc123...`, when `speccy lock SPEC-0001`
  runs, then the resulting TASKS.md frontmatter contains
  `spec_hash_at_generation: abc123...` and a `generated_at`
  field matching the current UTC timestamp.
- Given the same workspace, when `speccy lock SPEC-0001` runs a
  second time without any SPEC.md edits, then `spec_hash_at_generation`
  is unchanged (same hash) and `generated_at` advances to the
  current UTC timestamp.
- Given a workspace with malformed SPEC.md (e.g. frontmatter
  missing the `id` field), when `speccy lock SPEC-0001` runs,
  then the process exits 1, stderr names the parse error, and
  TASKS.md is unchanged on disk.
</behavior>

<scenario id="CHK-003">
Given a tempdir workspace containing a valid SPEC.md and a
valid TASKS.md with `spec_hash_at_generation: bootstrap-pending`,
when `speccy lock SPEC-0001` runs,
then the process exits 0 and the rewritten TASKS.md frontmatter
carries the SPEC.md sha256 in `spec_hash_at_generation` plus a
UTC `generated_at` field of the expected RFC-3339 shape.
</scenario>

<scenario id="CHK-004">
Given a tempdir workspace where SPEC-0001's SPEC.md is missing
the required `id` frontmatter field (parse error),
when `speccy lock SPEC-0001` runs,
then the process exits 1, stderr contains a message naming the
parse failure, and the workspace's TASKS.md frontmatter is
byte-identical to its pre-invocation state.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `speccy vacancy [--json]` returns the next free SPEC ID

A new top-level command `speccy vacancy` walks the
`.speccy/specs/` tree (flat slug directories plus one level of
mission folders), finds the highest existing SPEC-NNNN, and
returns the next ID. Text output is the bare ID string;
`--json` output is a one-field envelope. The command performs no
filesystem writes.

<done-when>
- A new file `speccy-cli/src/vacancy.rs` exists, exporting a
  `run` function that writes the resolved next SPEC-NNNN to
  stdout.
- The `Command` enum in `speccy-cli/src/main.rs` contains a
  `Vacancy { json: bool }` variant.
- Running `speccy vacancy` against a workspace whose highest
  existing SPEC is `SPEC-0032` writes `SPEC-0033\n` to stdout
  and exits 0.
- Running `speccy vacancy --json` against the same workspace
  writes `{"schema_version":1,"next_spec_id":"SPEC-0033"}\n`
  to stdout and exits 0.
- Running `speccy vacancy` against a workspace with no
  `.speccy/specs/` directory exits 1 with the standard
  "`.speccy/` directory not found" message used by other
  workspace-discovery-requiring commands.
- The implementation reuses
  `speccy_core::prompt::allocate_next_spec_id` (or its
  successor after the `speccy-core` cleanup in REQ-001) — the
  ID-walk logic is not re-implemented.
</done-when>

<behavior>
- Given a workspace where `.speccy/specs/` contains directories
  `0001-foo/`, `0027-bar/`, and `0032-baz/`, when
  `speccy vacancy` runs, then stdout is `SPEC-0033\n`.
- Given a workspace where `.speccy/specs/` contains
  `0032-baz/` and `auth/0033-signup/` (mission folder), when
  `speccy vacancy` runs, then stdout is `SPEC-0034\n` (the
  mission folder is walked).
- Given a workspace with an empty `.speccy/specs/` directory,
  when `speccy vacancy` runs, then stdout is `SPEC-0001\n`.
- Given a workspace with no `.speccy/` directory, when
  `speccy vacancy` runs, then the process exits 1 with stderr
  message `speccy vacancy: .speccy/ directory not found walking
  up from current directory`.
</behavior>

<scenario id="CHK-005">
Given a tempdir workspace with `.speccy/specs/` containing
directories `0001-foo/`, `0027-bar/`, `0032-baz/`, and a
mission folder `auth/` containing `0033-signup/`,
when `speccy vacancy --json` runs with cwd at the workspace
root,
then stdout exactly equals
`{"schema_version":1,"next_spec_id":"SPEC-0034"}\n` and the
process exits 0.
</scenario>

<scenario id="CHK-006">
Given a tempdir with no `.speccy/` directory anywhere in the
ancestry of cwd,
when `speccy vacancy` runs,
then stdout is empty, the process exits 1, and stderr contains
the substring `.speccy/ directory not found`.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `speccy next` drops `--kind`; action kind is derived from spec state

The `--kind` flag is removed from `speccy next`. The action kind
returned per active spec is derived from on-disk artifact state
via the priority rule `review > implement > ship`, with
`decompose` as the kind when TASKS.md does not exist for an
otherwise-active spec. The workspace form (`speccy next`) lists
every active spec with its derived `next_action`; the per-spec
form (`speccy next SPEC-NNNN`) returns one entry or
`{ next_action: null, reason: "completed" | "superseded" }`.

<done-when>
- The `Next { kind, json }` variant in `speccy-cli/src/main.rs`
  loses its `kind` field; the variant becomes
  `Next { spec_id: Option<String>, json: bool }`.
- The `KindFilter` type in `speccy-core/src/next/...` and any
  filtering logic that consumed it are removed.
- `speccy next --kind implement` is rejected by `clap` with
  "unexpected argument `--kind`".
- The kind derivation logic in `speccy_core::next` (or its
  successor module) implements exactly: if TASKS.md is absent,
  kind = "decompose"; else if any task has
  `state="in-review"`, kind = "review"; else if any task has
  `state="pending"`, kind = "implement"; else if all tasks are
  `state="completed"` and REPORT.md is absent, kind = "ship";
  else the spec is omitted from workspace listing (or surfaces
  `next_action: null` with reason for per-spec query).
- `speccy next` workspace-form text output one-line-per-spec
  format includes the derived kind plus task_id where
  applicable.
- `speccy next SPEC-NNNN` per-spec form against a spec where
  every task is `state="completed"` and a REPORT.md exists
  emits text `SPEC-NNNN: completed` and exits 0; with `--json`,
  emits `{ "schema_version": 2, "spec_id": "...",
  "next_action": null, "reason": "completed" }`.
</done-when>

<behavior>
- Given a workspace with SPEC-0001 having one
  `state="in-review"` task and two `state="pending"` tasks,
  when `speccy next SPEC-0001 --json` runs, then the resulting
  envelope's `next_action.kind` equals `"review"` and its
  `next_action.task_id` is the ID of the in-review task.
- Given the same SPEC-0001 after the in-review task transitions
  to `state="completed"`, when `speccy next SPEC-0001 --json`
  runs, then `next_action.kind` equals `"implement"` and
  `next_action.task_id` is the first remaining pending task.
- Given a workspace where SPEC-0002 has SPEC.md but no TASKS.md,
  when `speccy next SPEC-0002 --json` runs, then
  `next_action.kind` equals `"decompose"` and `next_action`
  has no `task_id` field.
- Given a workspace where SPEC-0003 has every task
  `state="completed"` and a REPORT.md present, when
  `speccy next SPEC-0003 --json` runs, then `next_action` is
  `null` and `reason` is `"completed"`.
- Given the same workspace, when `speccy next` (no SPEC-ID)
  runs, then SPEC-0003 is omitted from the `specs[]` list and
  SPEC-0001 plus SPEC-0002 are listed with their derived
  next_action.
</behavior>

<scenario id="CHK-007">
Given a tempdir workspace where SPEC-0001's TASKS.md contains
one `<task id="T-002" state="in-review">` and one
`<task id="T-001" state="completed">` and one
`<task id="T-003" state="pending">`,
when `speccy next SPEC-0001 --json` runs,
then the JSON output's `next_action` field equals
`{"kind":"review","task_id":"T-002"}`.
</scenario>

<scenario id="CHK-008">
Given a tempdir workspace containing only SPEC-0002 with
SPEC.md present, no TASKS.md, no REPORT.md,
when `speccy next` runs (workspace form, no args, text output),
then stdout contains exactly one line referencing SPEC-0002
with action kind `decompose` and no task_id.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: `speccy status` and `speccy next` JSON envelopes bump to schema_version 2 with resolved paths

The JSON envelopes emitted by `speccy status --json` and
`speccy next --json` change to `schema_version: 2`. Per-spec
entries gain `spec_md_path` (always present, relative to
workspace root, forward-slash separators), `tasks_md_path`
(present and non-null when TASKS.md exists; `null` otherwise),
and `mission_md_path` (present and non-null when the spec lives
under a mission folder; `null` otherwise). `speccy next` entries
additionally gain `next_action: { kind, task_id? }` per REQ-004.

<done-when>
- The `schema_version` field in both envelopes is `2`.
- Every per-spec object in `speccy status --json` carries
  `spec_md_path`, `tasks_md_path`, `mission_md_path` keys (with
  appropriate null handling).
- Every per-spec object in `speccy next --json` carries the
  same path fields plus the `next_action` block.
- Paths are emitted as repo-relative forward-slash strings
  (`.speccy/specs/0031-foo/SPEC.md`), not absolute paths.
- Path resolution reuses the existing
  `speccy_core::workspace` scanner; no new path-discovery code
  is added to the JSON-serialization layer.
</done-when>

<behavior>
- Given a workspace with SPEC-0031 living flat at
  `.speccy/specs/0031-foo/`, when `speccy status SPEC-0031
  --json` runs, then the per-spec object's `spec_md_path`
  equals `.speccy/specs/0031-foo/SPEC.md` and
  `mission_md_path` equals `null`.
- Given a workspace with SPEC-0040 living under mission folder
  `auth/`, when `speccy status SPEC-0040 --json` runs, then the
  per-spec object's `spec_md_path` equals
  `.speccy/specs/auth/0040-signup/SPEC.md` and
  `mission_md_path` equals `.speccy/specs/auth/MISSION.md`.
- Given a workspace with SPEC-0032 having SPEC.md but no
  TASKS.md, when `speccy next SPEC-0032 --json` runs, then the
  per-spec object's `tasks_md_path` equals `null` and
  `next_action.kind` equals `"decompose"`.
</behavior>

<scenario id="CHK-009">
Given a tempdir workspace with one flat spec at
`.speccy/specs/0031-foo/` containing valid SPEC.md and
TASKS.md,
when `speccy status SPEC-0031 --json` runs,
then the JSON output's `schema_version` field equals `2` and
the per-spec entry carries
`"spec_md_path": ".speccy/specs/0031-foo/SPEC.md"`,
`"tasks_md_path": ".speccy/specs/0031-foo/TASKS.md"`, and
`"mission_md_path": null`.
</scenario>

<scenario id="CHK-010">
Given a tempdir workspace where SPEC-0040 lives under
`.speccy/specs/auth/0040-signup/` and `.speccy/specs/auth/MISSION.md`
exists,
when `speccy next SPEC-0040 --json` runs,
then the resulting envelope's `mission_md_path` equals
`.speccy/specs/auth/MISSION.md` (the resolved parent mission
path).
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Phase prompt bodies eject as Skip-on-exists SKILL.md and subagent files at `speccy init`

`speccy init` writes one SKILL.md per shipped skill into the host
skill pack location, plus the per-phase subagent files SPEC-0032
introduced. Each SKILL.md carries the full prompt body inline (the
substance that today lives in `resources/modules/prompts/<phase>.md`
ends up as ejected text the skill agent reads directly, with no CLI
call to render it). The four Claude Code phase-worker subagent files
(`.claude/agents/speccy-<phase>.md` for `tasks` / `work` / `ship` /
`init`) and the five Codex phase-worker agent TOML files
(`.codex/agents/speccy-<phase>.toml` for `tasks` / `work` / `ship` /
`init` / `review`) eject from the same shared body source as the
matching SKILL.md, ensuring one source of phase-body content. `init`
classifies every one of these files as Skip-on-exists; `speccy init
--force` does not overwrite them when they already exist on disk —
the same protection applies to user edits of model pins or agent
bodies as to user edits of skill bodies. The shipped skills are:
`speccy-init`, `speccy-brainstorm`, `speccy-plan`, `speccy-tasks`,
`speccy-work`, `speccy-review`, `speccy-ship`, `speccy-amend`. The
ejected SKILL.md files carry the frontmatter SPEC-0032 specifies
(`context: fork` + `agent:` target on the four phase-worker skills;
direct `model: haiku` on `speccy-review`); the conversational
skills (`speccy-brainstorm`, `speccy-plan`, `speccy-amend`) eject
without model frontmatter per SPEC-0032's non-goal. The Codex
phase-worker SKILL.md bodies carry a one-line pointer naming the
`.codex/agents/<phase>.toml` invocation path; the MiniJinja
decomposition preserves this pointer through eject.

<done-when>
- The directory `resources/modules/skills/` (or its successor
  location chosen during implementation) holds one MiniJinja
  template per shipped skill plus a `_partials/` subdirectory
  containing shared snippets.
- The init plan classifies every ejected SKILL.md under
  `.claude/skills/speccy-<verb>/` and `.agents/skills/speccy-<verb>/`,
  every Claude Code phase-worker subagent file under
  `.claude/agents/speccy-<phase>.md`, and every Codex
  phase-worker agent TOML under `.codex/agents/speccy-<phase>.toml`
  as Skip-on-exists (per the existing `Action::Skip` semantics
  used for reviewer files in SPEC-0027).
- Running `speccy init` against an empty directory creates the
  full skill pack on disk with prompt bodies inlined in
  SKILL.md, the four `.claude/agents/speccy-<phase>.md` subagent
  files, and the five `.codex/agents/speccy-<phase>.toml` agent
  files.
- Running `speccy init --force` against a workspace where a
  user has edited `.claude/skills/speccy-plan/SKILL.md`,
  `.claude/agents/speccy-work.md`, or
  `.codex/agents/speccy-tasks.toml` leaves each of those files
  byte-identical to its pre-invocation state.
- Each ejected SKILL.md and subagent body file is a self-contained
  markdown file containing no MiniJinja markup (no `{% %}`, no
  `{{ }}`).
- The ejected `.claude/skills/speccy-work/SKILL.md` frontmatter
  carries `context: fork` and `agent: speccy-work`; the ejected
  `.claude/skills/speccy-review/SKILL.md` frontmatter carries
  `model: haiku` directly (no fork, per SPEC-0032's
  Task-tool-access constraint).
- Each ejected Codex phase-worker SKILL.md at
  `.agents/skills/speccy-<phase>/SKILL.md` contains one prose line
  naming the corresponding `.codex/agents/speccy-<phase>.toml`
  path as the `/agent <name>` invocation route.
</done-when>

<behavior>
- Given an empty tempdir, when `speccy init --host claude-code`
  runs, then `.claude/skills/speccy-plan/SKILL.md` exists and
  contains the full Phase 1 prompt body inline (greenfield form
  detection logic, allocation reference, scenarios template,
  etc.).
- Given the same tempdir after init, when a user appends
  `\nCustom domain note: always include a Data migration
  section.\n` to `.claude/skills/speccy-plan/SKILL.md` and then
  runs `speccy init --force --host claude-code`, then the file
  retains the user's appended line byte-for-byte.
- Given any ejected SKILL.md or subagent body file produced by
  `speccy init`, when scanned for the substrings `{{`, `{%`,
  `{#`, then zero matches are found.
- Given an empty tempdir, when `speccy init --host claude-code`
  runs, then `.claude/agents/speccy-work.md` exists, its
  frontmatter carries `model: sonnet` plus `effort: medium`, and
  its body is byte-identical to the body of
  `.claude/skills/speccy-work/SKILL.md` modulo frontmatter (one
  shared body source feeds both).
- Given an empty tempdir, when `speccy init --host codex` runs,
  then `.agents/skills/speccy-work/SKILL.md` contains a line
  naming `.codex/agents/speccy-work.toml` as the pinned
  invocation path, and `.codex/agents/speccy-work.toml` exists
  with `model = "sonnet"` and `model_reasoning_effort = "medium"`
  set in its top-level table.
</behavior>

<scenario id="CHK-011">
Given a freshly initialized tempdir workspace
(`speccy init --host claude-code` run once),
when the file `.claude/skills/speccy-plan/SKILL.md` is read,
then its content contains substantive prompt body (more than
the trigger-only metadata that existed pre-SPEC) and contains
no MiniJinja template syntax.
</scenario>

<scenario id="CHK-012">
Given a tempdir workspace where `.claude/skills/speccy-work/SKILL.md`
has been user-edited (one line appended at end-of-file),
when `speccy init --force --host claude-code` runs,
then the file remains byte-identical to its user-edited state
(the appended line survives).
</scenario>

<scenario id="CHK-016">
Given a freshly initialized tempdir workspace
(`speccy init --host claude-code` run once),
when the file `.claude/agents/speccy-work.md` is read,
then it exists, its YAML frontmatter parses with `model: sonnet`
and `effort: medium`, and its post-frontmatter body is
byte-identical to the post-frontmatter body of
`.claude/skills/speccy-work/SKILL.md` (one shared body source
feeds both consumers).
</scenario>

<scenario id="CHK-017">
Given a freshly initialized tempdir workspace
(`speccy init --host codex` run once),
when the file `.agents/skills/speccy-work/SKILL.md` is read,
then it contains exactly one prose line that names
`.codex/agents/speccy-work.toml` as the `/agent`-invocation path,
and `.codex/agents/speccy-work.toml` exists with
`model = "sonnet"` and `model_reasoning_effort = "medium"` at the
top level of the TOML document.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Shared snippets live in MiniJinja partials; reviewer subagent body is a single template with conditional tests block

Upstream authoring deduplicates shared boilerplate via MiniJinja
`{% include %}` over partial files in
`resources/modules/skills/_partials/` (or the equivalent path).
The reviewer concern after this SPEC + SPEC-0032 is **subagent
bodies**, not skill bodies: `/speccy-review` is a single
orchestrator skill that dispatches via the Task tool to six
reviewer subagents (`reviewer-business`, `reviewer-tests`,
`reviewer-architecture`, `reviewer-security`, `reviewer-style`,
`reviewer-docs`), each already ejected per SPEC-0027 to
`.claude/agents/reviewer-<persona>.md` (Claude Code) and
`.codex/agents/reviewer-<persona>.toml` (Codex). There is no
per-persona ejected skill — the orchestrator is the only skill
on the review side. The five reviewer subagent bodies that are
byte-identical today (business, architecture, docs, security,
style) plus the reviewer-tests variant collapse to one shared
template; the tests-specific extra step (the Evidence-read
block) appears inside a `{% if persona == "tests" %}`
conditional.

<done-when>
- The pre-SPEC files
  `resources/modules/prompts/reviewer-{business,architecture,docs,security,style,tests}.md`
  are deleted (per REQ-001) and the reviewer subagent body
  source consolidates to a single template at
  `resources/modules/personas/reviewer.md.j2` (or equivalent
  path under the existing `resources/modules/personas/` tree
  that SPEC-0027 established).
- Shared boilerplate snippets are extracted into the
  `_partials/` subdirectory and `{% include %}`d by each
  consuming template (skills and subagent bodies alike).
- At `speccy init` time, the MiniJinja renderer expands
  `{% include %}` and conditional `{% if %}` blocks into the
  final ejected subagent body files, producing six reviewer
  subagent bodies (one per persona) from one source template.
- The ejected reviewer subagent bodies under
  `.claude/agents/reviewer-<persona>.md` have identical post-
  frontmatter prose except for: the persona-name substitution
  (already a build-time substitution today via
  `{{ persona }}`) and the Evidence-read step that appears
  only in `.claude/agents/reviewer-tests.md`. Each carries the
  per-persona frontmatter pin SPEC-0032 specifies
  (`reviewer-business` / `reviewer-tests` /
  `reviewer-architecture` at Opus xhigh; `reviewer-security`
  at Sonnet high; `reviewer-style` / `reviewer-docs` at
  Sonnet medium).
- A test in `speccy-cli/tests/init.rs` (or its successor)
  asserts the per-persona reviewer subagent bodies are
  byte-identical modulo the persona name and the
  tests-conditional block.
</done-when>

<behavior>
- Given the post-SPEC source tree, when a contributor inspects
  the reviewer body source, then there is exactly one reviewer
  template file (`reviewer.md.j2` or equivalent), not six.
- Given a freshly initialized workspace, when the six ejected
  reviewer subagent bodies under `.claude/agents/` are
  compared pairwise, then the diff between any non-tests pair
  is empty modulo persona-name string substitution; the diff
  between any non-tests file and the tests file contains
  exactly the Evidence-read block as an additional step.
- Given the post-SPEC source tree, when a contributor opens
  `resources/modules/skills/_partials/spec-pointer.md` (or
  equivalent name), then it contains the shared
  "Before X, read SPEC.md at `{{ spec_md_path }}`..."
  boilerplate text used across multiple consumers (skills and
  reviewer subagent bodies).
- Given the post-SPEC ejection, when a contributor looks for a
  `.claude/skills/speccy-review-<persona>/SKILL.md` file under
  any persona name, then no such file exists — `/speccy-review`
  is one orchestrator skill, and per-persona content lives in
  the six `.claude/agents/reviewer-<persona>.md` subagent
  files.
</behavior>

<scenario id="CHK-013">
Given the post-SPEC source tree,
when the file `resources/modules/personas/reviewer.md.j2` (or
equivalent path) is rendered by the MiniJinja engine with
`persona = "tests"` and again with `persona = "business"`,
then the two rendered outputs differ exactly in: the
`{{ persona }}` substitution (one occurrence per rendering)
and the presence of the Evidence-read paragraph in the tests
rendering only.
</scenario>

<scenario id="CHK-018">
Given a freshly initialized tempdir workspace
(`speccy init --host claude-code` run once),
when the workspace is scanned for files matching
`.claude/skills/speccy-review-*/SKILL.md`,
then zero matches are returned — there is one
`.claude/skills/speccy-review/SKILL.md` orchestrator skill and
six `.claude/agents/reviewer-<persona>.md` subagent body files
covering the per-persona content.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Skills consume CLI state via JSON envelopes; no skill globs the filesystem

After this SPEC lands, the shipped skill bodies (`speccy-plan`,
`speccy-tasks`, `speccy-work`, `speccy-review`, `speccy-ship`,
`speccy-amend`) read all spec/task/path state from
`speccy status --json` and `speccy next --json` (plus
`speccy vacancy --json` for greenfield-plan). No skill body
contains filesystem-globbing logic, glob patterns matching
`.speccy/specs/*`, or path computation from SPEC IDs to file
paths.

<done-when>
- A grep over `resources/modules/skills/` for the patterns
  `.speccy/specs/*`, `glob`, `Glob`, `walk`, `read_dir`,
  `os.listdir`, or `fs.readdir` returns zero hits in
  skill-body content (matches inside `_partials/` that are
  part of host pointer instructions like
  "read SPEC.md at `{{ spec_md_path }}`" do not count — the
  prohibition is on filesystem *discovery*, not on consuming
  paths the CLI emitted).
- Every shipped skill body that needs a SPEC path or TASKS
  path obtains it from either: (a) the user-supplied argument
  if the skill takes a SPEC-ID, or (b) a JSON envelope from
  `speccy status` / `speccy next` / `speccy vacancy`.
- The `speccy-plan` greenfield-form skill body invokes
  `speccy vacancy --json` (not `speccy status --json`) to
  fetch the next SPEC ID, demonstrating the payload-minimization
  pattern.
</done-when>

<behavior>
- Given the post-SPEC `resources/modules/skills/` source tree,
  when a contributor greps for `.speccy/specs/*` (glob
  pattern), then zero hits appear in skill-body content.
- Given the ejected `.claude/skills/speccy-plan/SKILL.md` in
  greenfield mode, when its contents are inspected, then the
  body invokes `speccy vacancy --json` (and not
  `speccy status --json`) for ID allocation.
- Given the ejected `.claude/skills/speccy-work/SKILL.md`,
  when its body is followed by an agent, then every path the
  agent needs comes from a `speccy next` or `speccy status`
  JSON envelope; the skill body itself names no fixed
  filesystem paths beyond `.speccy/` as the root.
</behavior>

<scenario id="CHK-014">
Given the post-SPEC `resources/modules/skills/` source tree,
when a recursive search runs for filesystem-discovery
patterns (`glob`, `Glob`, `walk`, `read_dir`, raw
`.speccy/specs/*` glob expressions),
then zero matches appear inside skill-body content (helper
partials that name fully-resolved paths supplied via
`{{ ... }}` placeholders do not count, as those paths
originate from the CLI's JSON envelopes).
</scenario>

<scenario id="CHK-015">
Given the ejected `.claude/skills/speccy-plan/SKILL.md` file
post-SPEC,
when its body is parsed for command invocations,
then the greenfield-form path invokes `speccy vacancy --json`
to learn the next SPEC ID, not `speccy status --json`.
</scenario>

</requirement>

## Design

### Decisions

<decision id="DEC-001">
**CLI resolves filesystem paths; skills consume paths via JSON envelopes.**

The slug-pattern rule for spec directories (`NNNN-slug` under
`.speccy/specs/`, optionally one level inside a mission folder)
lives exclusively in the CLI. Skills do not glob filesystem
state. Reason: single source of truth for the spec-discovery
contract. If the rule ever changes (e.g. deeper mission nesting),
only the CLI updates and every skill picks up the new behavior
for free via the unchanged JSON shape. Trade-off acknowledged:
skills become dependent on the CLI being present, but that's
already true (skills invoke the CLI for every state query).
</decision>

<decision id="DEC-002">
**`speccy next` derives action kind from spec state; no user-supplied filter.**

Each active spec is in exactly one state at any moment, and that
state fully determines the kind of next action (decompose,
implement, review, ship). The `--kind` flag that existed pre-SPEC
was an inverted contract: the caller told the CLI what kind to
filter for, when in fact the CLI knows the unambiguous answer.
Removing the flag simplifies callers, eliminates an ambiguous
case (what if the caller asks for `--kind implement` against a
spec whose only pending action is `review`?), and makes the
priority rule discoverable.
</decision>

<decision id="DEC-003">
**Priority rule: `review > implement > ship`, with `decompose` when TASKS.md is absent.**

When a spec has both `state="pending"` and `state="in-review"`
tasks, the surfaced next action is `review`. Reason: drift
visibility favors short feedback loops. Bugs caught in the
piecewise (implement → review → implement → review) workflow
are cheap; bugs caught after multiple tasks build on top of an
inherited mistake are expensive. The priority rule nudges
agents toward Pattern A (piecewise) by default. Users who want
Pattern B (batched: implement many tasks, then review them all)
override by invoking `speccy next SPEC-NNNN/T-NNN` directly.
Not enforced — per AGENTS.md "Feedback, not enforcement."
</decision>

<decision id="DEC-004">
**Single shared reviewer prompt template; per-persona ejection deferred.**

Today, five of the six reviewer prompts are byte-identical;
only `reviewer-tests` adds one step (the Evidence-read block).
One shared MiniJinja template with a `{% if persona == "tests" %}`
conditional is the right factoring. Per-persona prompt
ejection (six separate files) is deferred until structural
divergence between personas appears at the prompt level
(beyond the existing tests-only block). The persona body itself
remains separately ejected per persona per SPEC-0027 — that
ejection is unchanged.
</decision>

<decision id="DEC-005">
**`speccy vacancy` is a dedicated command, not a `--next-id` flag on `status`.**

Folding the next-ID query into `speccy status --next-id` would
violate AGENTS.md's "no mode toggles" core principle: the flag
would change *what content* the command returns, not how
(`--json` changes representation but not content). Mode flags
as content selectors scale badly. Symmetry forces consistency:
if `vacancy` becomes a status flag, so would `next`, and the
CLI degenerates into a kitchen-sink. Seven flat verbs where
each does one job is more learnable than a five-verb surface
with three modes per verb.
</decision>

<decision id="DEC-006">
**`speccy lock` preserves `tasks --commit` validation; no new precondition surface.**

`speccy lock SPEC-NNNN` validates that SPEC.md exists and
parses, and that TASKS.md exists and parses, before rewriting
TASKS.md frontmatter. This is exactly the validation
`tasks --commit` did pre-SPEC. No additional consistency checks
(e.g. "all task IDs map to REQs", "no orphaned tasks") are
added — those checks live in `speccy verify` already, and
duplicating them in `lock` would create two enforcement paths
for the same invariant.
</decision>

<decision id="DEC-007">
**Existing JSON envelopes are augmented; no new per-phase context endpoints.**

Skills that need state call `speccy status --json` /
`speccy next --json` / `speccy vacancy --json`. No
`speccy plan-context`, `speccy implement-context`,
`speccy review-context` per-phase query verbs are introduced.
Reason: the structured fields skills actually need (spec ID,
resolved paths, action kind) fit cleanly in the existing
envelopes; per-phase commands would be a wrapper layer that
trims a few fields off `status`/`next` payloads, which is more
surface for marginal payload savings. The one exception is
`vacancy` — it earns its own command because its payload
(one ID) is fundamentally smaller than `status` and its hot
path (greenfield plan) is performance-sensitive.
</decision>

## Assumptions

<assumptions>
- Modern host context windows (Claude 4.x, GPT-4.x) absorb fresh
  reads of SPEC.md plus TASKS.md without budget pressure for
  typical specs (1-5 KB). Specs that balloon past this size are
  themselves a drift signal — the structural answer is
  decomposition into multiple specs under a MISSION folder, not
  CLI-side text trimming. Speccy's existing nouns already carry
  the growth answer; the CLI does not need a second growth
  answer at the rendering layer.
- The host's `Read` primitive is reliable enough that agents do
  not need pre-inlined content for latency or cost reasons.
  Empirically true today across Claude Code and Codex.
- The existing MiniJinja-include mechanism used for host-pack
  wrappers (`resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`
  per SPEC-0027) extends naturally to including phase prompt
  bodies into ejected SKILL.md files. No new build mechanism
  needs to be designed.
- Users will not depend on the deleted CLI verbs (`plan`,
  `tasks` render-form, `implement`, `review`, `report`). Speccy
  v1 is not released; there are no external integrations. The
  dogfooded `.speccy/` workspace migration is done by hand as
  part of this SPEC's implementation tasks.
- The `_partials/` directory naming and exact MiniJinja
  include-resolution paths are implementation details, not
  contract surface. The SPEC names the layering goal (shared
  snippets factor out, reviewer template collapses to one
  source); the decomposition phase chooses concrete paths.
- The `speccy_core::prompt::allocate_next_spec_id` function (or
  its successor) remains the single authority on ID allocation
  even after `prompt::template.rs` and `prompt::budget.rs` are
  removed. The function is filesystem-only; it has no dependency
  on the template loader.
- Reviewer skill ejection produces six distinct SKILL.md files
  (one per persona) so that the host's skill discovery
  mechanism can address each independently. The shared upstream
  template is a build-time deduplication, not a runtime
  deduplication.
- The shared body source at
  `resources/modules/skills/speccy-<phase>.md` is included by
  both the SKILL.md template (which renders to
  `.claude/skills/speccy-<phase>/SKILL.md` plus the Codex
  equivalent) and the subagent body template (which renders to
  `.claude/agents/speccy-<phase>.md` plus the Codex agent
  TOML's body field) per the SPEC-0032 dependency. Single
  source of truth for phase-body content; the MiniJinja
  decomposition must keep both consumers green at every
  refactor step. The reviewer side mirrors this pattern: one
  `resources/modules/personas/reviewer.md.j2` source feeds the
  six `.claude/agents/reviewer-<persona>.md` subagent bodies
  via `{% if persona == "tests" %}` conditional rendering.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-19 | human/kevin | Initial draft. The CLI's two architectural jobs (mechanical state queries vs. authored prompt rendering) are decoupled: five prompt-rendering verbs (`plan`, `tasks` render-form, `implement`, `review`, `report`) and the `trim_to_budget` mechanism are deleted, phase prompt bodies eject as Skip-on-exists SKILL.md files at `init` time, and two new flat verbs (`speccy lock`, `speccy vacancy`) take the real-CLI-work that hid inside the deleted rendering paths. `status` and `next` JSON envelopes bump to schema_version 2 with resolved paths plus derived `next_action`; `next` drops `--kind` because spec state fully determines action kind via the priority rule `review > implement > ship` (drift-visibility favors short feedback loops). CLI resolves all filesystem paths; skills consume via JSON only. Upstream skill authoring uses MiniJinja partials for shared snippets; the six reviewer prompts collapse to one shared template with a tests-only conditional block. Direct extension of SPEC-0023 (REQ-005, REQ-006) and SPEC-0027: when host machinery already delivers content to the agent through some other channel (Read primitive, sub-agent system context, ejected skill body), the CLI prompt-rendering surface stops carrying redundant copies. Final CLI shape: seven flat verbs (`init`, `status`, `next`, `check`, `verify`, `lock`, `vacancy`), each doing one job, no mode flags, `--json` for representation only. |
| 2026-05-19 | human/kevin | Amendment for SPEC-0032 sequencing dependency. SPEC-0032 (per-phase model and effort pinning across the lifecycle) is now a hard sequencing predecessor: it adds skill frontmatter (`context: fork` + `agent:` target on `speccy-tasks` / `speccy-work` / `speccy-ship` / `speccy-init`; direct `model: haiku` on `speccy-review`), four `.claude/agents/speccy-<phase>.md` subagent files, and five `.codex/agents/speccy-<phase>.toml` agent files that this SPEC's ejection must preserve. REQ-006 extends the Skip-on-exists ejection set to include those new files and asserts that ejected SKILL.md frontmatter carries the SPEC-0032 pins through MiniJinja round-trip; the Codex SKILL.md pointer line naming `.codex/agents/<phase>.toml` is required content. REQ-007 reframes the shared reviewer template as the **subagent body** template (six `.claude/agents/reviewer-<persona>.md` bodies from one source under `resources/modules/personas/`), not a phantom set of per-persona skills — SPEC-0032 establishes that `/speccy-review` is a single orchestrator skill that dispatches to six existing subagents. New scenarios CHK-012a (subagent body matches SKILL.md body), CHK-012b (Codex pointer line and agent TOML pinned correctly), CHK-013a (no per-persona review skill exists). New Assumption: `resources/modules/skills/<phase>.md` is dual-consumer (SKILL.md template + subagent body template), and the reviewer side mirrors this with one persona source feeding six subagent bodies. The "per-persona ejected reviewer skill naming" open question is resolved (struck) by the SPEC-0032 dispatch model. |
</changelog>

## Open Questions

- [x] ~~Naming for the per-persona ejected reviewer skills: one
  skill `speccy-review` that accepts `--persona` and ejects
  one SKILL.md, or six `speccy-review-<persona>` skills?~~
  Resolved by the SPEC-0032 dependency: `/speccy-review` is a
  single orchestrator skill at Haiku that dispatches via the
  Task tool to the six existing reviewer subagents at
  `.claude/agents/reviewer-<persona>.md` (and Codex
  equivalents). There is no per-persona ejected skill; the
  per-persona body content lives in the subagent files SPEC-0027
  already established, and the model pins live on those
  subagent files per SPEC-0032. REQ-007 now targets the shared
  reviewer subagent body template, not a skill template.
- [ ] Whether `speccy vacancy`'s implementation reuses
  `allocate_next_spec_id` directly from speccy-core, or whether
  the function relocates (e.g. out of `prompt::id_alloc` into
  a more general `speccy_core::specs::next_id`) as part of the
  speccy-core cleanup that REQ-001 implies. Decompose-time
  decision.
- [ ] Exact path under `resources/modules/skills/` for shared
  partials. Candidates: `_partials/`, `_includes/`, `shared/`.
  Mechanical naming choice; no SPEC-level position.
