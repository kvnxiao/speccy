---
id: SPEC-0033
slug: eject-prompt-bodies
title: Eject phase prompt bodies into skill files; CLI does state only, no natural-text rendering
status: implemented
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

The final surface is seven flat commands, each with one job.
Phase prompt bodies live in the host skill pack — pinned phase
workers as agent files at `.claude/agents/speccy-<phase>.md`
(Codex: `.codex/agents/speccy-<phase>.toml`) with thin SKILL.md
stubs as host-router entry points; interactive skills as
full-body SKILL.md under `.claude/skills/speccy-<skill>/`. Users
edit these files freely after init; the trade-off — `init
--force` overwrites differing files with a `(!) overwritten`
warning, and recovery is via git — is acknowledged out of scope
per the `AGENTS.md` quality bar ("useful for my next greenfield").

SPEC-0032 is a hard sequencing predecessor. As shipped, it
introduced three Claude Code phase-worker subagent files
(`.claude/agents/speccy-<phase>.md` for `tasks` / `work` /
`ship`, pinned to `model: sonnet[1m]` and `effort: medium`),
three matching Codex agent TOML files for the same three phases
(`gpt-5.5`, medium effort), asymmetric pins on the six existing
reviewer subagent files (Opus[1m] xhigh on business / tests /
architecture; Sonnet[1m] high on security; Sonnet[1m] medium on
style / docs), and a move of the shared phase-body source from
`resources/modules/skills/<phase>.md` to
`resources/modules/phases/<phase>.md` (SPEC-0032 DEC-009). No
`speccy-init` agent file ships (SPEC-0032 DEC-009 — init is
interactive bootstrap, not a recurring phase) and
`/speccy-review` is unpinned on both hosts (SPEC-0032
REQ-002 / REQ-009 — the orchestrator stays in the parent
session for Task-tool access and verdict-consolidation
capacity). The three pinned-phase SKILL.md files are thin stubs
(≤10 non-blank lines per SPEC-0032 REQ-010) naming the matching
agent file and the `/agent speccy-<phase>` invocation path; the
agent file is the source of phase-body content for each pinned
phase. This SPEC inherits SPEC-0032's frontmatter and file set
wholesale and does not re-litigate DEC-009 or the auto-fork
retreat — `context: fork` was dropped after SPEC-0032 T-002
surfaced its multi-minute UX cost; the opt-in heavy-model path
is `/agent speccy-<phase>`. See DEC-008 below for the rule that
governs which skills eject as SKILL.md stub + agent body and
which eject as full-body SKILL.md only.

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
- Phase prompt bodies eject into the host skill pack at
  `speccy init` time. The eject shape follows the skill's
  invocation pattern (see DEC-008): interactive skills
  (`speccy-init`, `speccy-brainstorm`, `speccy-plan`,
  `speccy-amend`, `speccy-review` orchestrator) eject as
  full-body SKILL.md only; pinned phase workers (`speccy-tasks`,
  `speccy-work`, `speccy-ship`) eject as a thin SKILL.md stub
  plus an agent file at `.claude/agents/speccy-<phase>.md`
  (Claude Code) or `.codex/agents/speccy-<phase>.toml` (Codex).
- `speccy init` handles existing files via a per-file three-way
  classification, with no Skip-on-exists semantic surviving: a
  planned target that is absent is created; a planned target
  whose on-disk bytes match the planned content is logged
  `unchanged` and not written; a planned target whose on-disk
  bytes differ from the planned content refuses the whole batch
  (no partial writes) with an error naming the differing file(s)
  and the `--force` override. `speccy init --force` overwrites
  the differing files (logged `(!) overwritten`) and otherwise
  follows the same classification. Recovery from an unwanted
  overwrite is via git; no in-CLI merge or backup mechanism
  ships.
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
  users who want it run `speccy init --force` (which overwrites
  every shipped file that differs from the planned content,
  logging each as `(!) overwritten`) and reapply any local
  edits afterward via git. The trade-off is explicit; speccy is
  not in the merge business.
- No `--strict` mode that enforces the piecewise workflow
  (implement → review → implement → review). The `next_action.kind`
  priority `review > implement > ship` is a recommendation
  surfaced in the JSON envelope, not a block. Users who want to
  implement multiple tasks before reviewing can call `speccy next
  SPEC-NNNN/T-NNN` directly to override the surfaced priority.
- No master reviewer template with conditional persona
  branches. Per-persona divergence in the six reviewer subagent
  bodies is legitimate (e.g., `reviewer-style` carries
  diff-format guidance that `reviewer-business` does not need;
  `reviewer-tests` carries an Evidence-read step that others do
  not). Six independent persona body files stay; shared blocks
  (verdict return contract, TASKS.md write prohibition, inline
  note format, diff-fetch command) factor out as topic-named
  snippets co-located inside `resources/modules/personas/`, not
  into a master template. No `_partials/` subdirectory exists.
  See REQ-007.
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
- No filesystem globbing inside skill stubs or agent bodies to
  discover **speccy resources** (`.speccy/specs/*`, SPEC.md,
  TASKS.md, MISSION.md, REPORT.md, the slug-pattern directory
  layout). The CLI's JSON envelopes are the sole authority for
  speccy-resource discovery. General-purpose Read / Glob / grep
  against project files unrelated to speccy resources (source
  code, test fixtures, project-level `AGENTS.md`,
  `.editorconfig`, etc.) remains fine; the constraint is a
  resource-discovery boundary, not a blanket filesystem-access
  ban. See REQ-008.
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
  touches the database schema"), I want `speccy init` (no force)
  to refuse with a clear error when I re-run it after a CLI
  bump, so I know my customization would be at risk if I had
  passed `--force`. When I want to pick up upstream improvements
  I run `speccy init --force`, read the `(!) overwritten` log
  lines naming my edited files, and reapply my customizations
  from git history. No silent overwrites; no hand-holding to
  preserve them.
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
### REQ-006: Phase prompt bodies eject at `speccy init`; init handles existing files via three-way classification

`speccy init` writes one SKILL.md per shipped skill into the host
skill pack location, plus the three pinned phase-worker subagent
files SPEC-0032 introduced. The eject shape per skill follows the
rule in DEC-008: interactive skills (`speccy-init`,
`speccy-brainstorm`, `speccy-plan`, `speccy-amend`, the
`speccy-review` orchestrator) eject as full-body SKILL.md only;
pinned phase workers (`speccy-tasks`, `speccy-work`,
`speccy-ship`) eject as a thin SKILL.md stub (≤10 non-blank lines
per SPEC-0032 REQ-010, naming the matching agent file and the
`/agent speccy-<phase>` invocation path) plus a full-body agent
file at `.claude/agents/speccy-<phase>.md` (Claude Code) or
`.codex/agents/speccy-<phase>.toml` (Codex). The agent body is
included from `resources/modules/phases/speccy-<phase>.md` via
MiniJinja `{% include %}` at build time. Pins on the pinned
phase workers carry `model: sonnet[1m]` and `effort: medium` on
Claude Code; `model = "gpt-5.5"` and
`model_reasoning_effort = "medium"` on Codex. No
`speccy-init` agent file ships (SPEC-0032 DEC-009) and no
`speccy-review` agent file ships (SPEC-0032 REQ-002 / REQ-009 —
the orchestrator stays in the parent session). The six reviewer
subagent body files at `.claude/agents/reviewer-<persona>.md`
and `.codex/agents/reviewer-<persona>.toml` are ejected per
SPEC-0027's contract with the per-persona pins SPEC-0032
REQ-003 specifies; this SPEC does not re-eject them but does
subject them to the new init-write semantics below.

The Skip-on-exists semantic SPEC-0027 introduced is removed in
full. `speccy init` (no flag) performs a per-file three-way
classification across every planned write: if the target file
is absent on disk, write it (logged `created`); if the target
file exists and is byte-identical to the planned content,
no-op and log `unchanged`; if the target file exists and
differs from the planned content, refuse the entire batch atomically
with a non-zero exit and a stderr message naming the differing
file(s) and the `--force` override. No partial writes occur on
the refuse path. `speccy init --force` follows the same
classification but overwrites differing files (logged
`(!) overwritten` with the warning marker) instead of refusing.
Files that are already byte-identical are logged `unchanged`
under `--force` too (not `(!) overwritten`) — the comparison
runs regardless of the flag. Recovery from an unwanted
overwrite is via git; no in-CLI merge or backup mechanism ships.

<done-when>
- `resources/modules/phases/` holds one body source per pinned
  phase worker (`speccy-tasks.md`, `speccy-work.md`,
  `speccy-ship.md`); `resources/modules/skills/` holds one body
  source per interactive skill (`speccy-init.md`,
  `speccy-brainstorm.md`, `speccy-plan.md`, `speccy-amend.md`,
  `speccy-review.md`). The SPEC-0032 T-009 rename is preserved.
- The init plan emits, for every planned target, one of three
  classifications: `created` (target absent), `unchanged`
  (target byte-identical to planned content), or
  `(!) overwritten` (target differed; `--force` was supplied).
  No `skipped` classification survives.
- Running `speccy init` (no flag) against an empty directory
  creates the full skill pack on disk: full-body SKILL.md under
  `.claude/skills/speccy-<skill>/` for every interactive skill;
  thin SKILL.md stub under `.claude/skills/speccy-<phase>/` for
  the three pinned phase workers; three full-body agent files
  at `.claude/agents/speccy-<phase>.md`; and the matching Codex
  pack under `.agents/skills/` and `.codex/agents/`.
- Each ejected file is a self-contained markdown or TOML
  document with no MiniJinja markup (no `{% %}`, no `{{ }}`).
  MiniJinja expansion is build-time-only.
- The three pinned phase-worker SKILL.md files at
  `.claude/skills/speccy-<phase>/SKILL.md` for
  `tasks` / `work` / `ship` carry no `context:`, `agent:`,
  `model:`, or `effort:` frontmatter fields (only `name:` and
  `description:`). Their bodies are ≤10 non-blank lines naming
  the matching agent file path and the `/agent speccy-<phase>`
  invocation pattern.
- The three pinned phase-worker agent files at
  `.claude/agents/speccy-<phase>.md` for `tasks` / `work` /
  `ship` carry `model: sonnet[1m]` and `effort: medium`
  frontmatter; the matching Codex TOMLs at
  `.codex/agents/speccy-<phase>.toml` carry
  `model = "gpt-5.5"` and `model_reasoning_effort = "medium"`.
- No `.claude/agents/speccy-init.md`, no
  `.codex/agents/speccy-init.toml`, no
  `.claude/agents/speccy-review.md`, and no
  `.codex/agents/speccy-review.toml` exist — both
  `speccy-init` and `speccy-review` are interactive skills per
  DEC-008 and eject as full-body SKILL.md only.
- Running `speccy init` against a workspace whose shipped files
  exactly match the planned writes (byte-identical) succeeds
  with exit 0, logs each file as `unchanged`, and writes
  nothing.
- Running `speccy init` against a workspace where one or more
  shipped files differ from the planned writes exits non-zero
  with a stderr error naming the differing path(s) and the
  `--force` override. No file writes occur on the refuse path.
- Running `speccy init --force` against the same workspace
  overwrites every differing file, logs each as
  `(!) overwritten`, logs already-identical files as
  `unchanged`, and exits 0.
</done-when>

<behavior>
- Given an empty tempdir, when `speccy init --host claude-code`
  runs, then `.claude/skills/speccy-plan/SKILL.md` exists and
  contains the full Phase 1 prompt body inline (greenfield form
  detection logic, allocation reference, scenarios template,
  etc.).
- Given an empty tempdir, when `speccy init --host claude-code`
  runs, then `.claude/skills/speccy-work/SKILL.md` exists as a
  thin stub (≤10 non-blank lines) naming
  `.claude/agents/speccy-work.md` and the `/agent speccy-work`
  invocation path, and `.claude/agents/speccy-work.md` exists
  with `model: sonnet[1m]` and `effort: medium` plus the full
  phase body included from
  `resources/modules/phases/speccy-work.md`.
- Given an empty tempdir, when `speccy init --host codex` runs,
  then `.agents/skills/speccy-work/SKILL.md` contains a thin
  stub naming `.codex/agents/speccy-work.toml` and the
  `/agent speccy-work` invocation path, and
  `.codex/agents/speccy-work.toml` exists with
  `model = "gpt-5.5"` and `model_reasoning_effort = "medium"`.
- Given a workspace where every shipped file matches the
  planned write byte-for-byte (a re-run after a no-op CLI bump),
  when `speccy init` runs, then every file is logged
  `unchanged`, no writes occur, and the process exits 0.
- Given a workspace where one user has edited
  `.claude/skills/speccy-plan/SKILL.md` (appending a custom
  domain instruction line), when `speccy init` runs without
  `--force`, then the process exits non-zero, stderr names the
  edited file as differing from the planned write and the
  `--force` override, and no writes occur (the file remains
  byte-identical to its user-edited state).
- Given the same workspace, when `speccy init --force` runs,
  then `.claude/skills/speccy-plan/SKILL.md` is overwritten with
  the planned content (the user's custom line is lost), stdout
  logs it as `(!) overwritten`, and the process exits 0.
  Recovery is via `git checkout` of the prior file content.
- Given any ejected SKILL.md, agent.md, or agent.toml file
  produced by `speccy init`, when scanned for the substrings
  `{{`, `{%`, `{#`, then zero matches are found.
</behavior>

<scenario id="CHK-011">
Given a freshly initialized tempdir workspace
(`speccy init --host claude-code` run once),
when the file `.claude/skills/speccy-plan/SKILL.md` is read,
then its content contains substantive prompt body (more than
the trigger-only metadata that existed pre-SPEC) and contains
no MiniJinja template syntax.
</scenario>

<scenario id="CHK-017">
Given a freshly initialized tempdir workspace
(`speccy init --host codex` run once),
when the file `.agents/skills/speccy-work/SKILL.md` is read,
then it is a thin stub of ≤10 non-blank lines naming
`.codex/agents/speccy-work.toml` and the `/agent speccy-work`
invocation path, and `.codex/agents/speccy-work.toml` exists
with `model = "gpt-5.5"` and `model_reasoning_effort = "medium"`
at the top level of the TOML document.
</scenario>

<scenario id="CHK-019">
Given a tempdir workspace where every file
`speccy init --host claude-code` would write already exists on
disk byte-identical to the planned content,
when `speccy init --host claude-code` runs (no `--force`),
then the process exits 0, stdout logs every file as
`unchanged`, and no writes occur (verified by file mtime
unchanged on every planned target).
</scenario>

<scenario id="CHK-020">
Given a tempdir workspace where one shipped file
(`.claude/skills/speccy-plan/SKILL.md`) has a user-appended
line of custom prose, making it differ from the planned write,
when `speccy init --host claude-code` runs without `--force`,
then the process exits non-zero, stderr names the differing
file path and the `--force` override, the offending file
remains byte-identical to its pre-invocation state, and no
other planned target was written (atomic batch refuse).
</scenario>

<scenario id="CHK-021">
Given the same tempdir workspace from CHK-020,
when `speccy init --force --host claude-code` runs,
then the process exits 0, the differing file is overwritten
with the planned content, stdout logs it as `(!) overwritten`
with the warning marker, and every other already-identical
file is logged `unchanged` (not `(!) overwritten`).
</scenario>

<scenario id="CHK-022">
Given a freshly initialized tempdir workspace
(`speccy init --host claude-code` and
`speccy init --host codex` run in turn),
when the workspace is scanned for files at
`.claude/agents/speccy-init.md`,
`.claude/agents/speccy-review.md`,
`.codex/agents/speccy-init.toml`, and
`.codex/agents/speccy-review.toml`,
then zero matches are returned per DEC-008 (interactive skills
have no agent counterpart; both `speccy-init` and the
`speccy-review` orchestrator fall in that category).
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Reviewer persona shared blocks factor into co-located snippets; six persona bodies stay independent

The six reviewer subagent body files at
`resources/modules/personas/reviewer-<persona>.md` stay
independent. Each persona retains its own role narrative, focus
list, "what to look for" guidance, and any persona-specific
content (e.g., `reviewer-style`'s "Diff-format pitfalls"
section, `reviewer-tests`'s Evidence-read step). The shared
blocks that recur verbatim across multiple personas — the
verdict-return contract, the "do not edit TASKS.md"
prohibition, the inline note format template, and the
diff-fetch command boilerplate — factor out into topic-named
snippet files that live INSIDE
`resources/modules/personas/` itself, distinguished from
persona body files by not matching the
`reviewer-<persona>.md` filename pattern. Each persona body
`{% include %}`s the snippets it needs at MiniJinja build time;
the renderer logic that walks "six persona bodies" filters on
the `reviewer-<persona>.md` pattern and naturally excludes the
snippet files as eject-targets.

The same co-location convention applies to any phase-worker
shared snippets: snippet files live IN `resources/modules/phases/`
alongside the phase body sources, with topic names that do not
collide with the `speccy-<phase>.md` pattern. No `_partials/`
subdirectory exists at any level; the snippet-vs-body
distinction is filename-based, not directory-based.
`/speccy-review` is a single orchestrator skill that dispatches
via the Task tool to the six reviewer subagents at
`.claude/agents/reviewer-<persona>.md` (Claude Code) and
`.codex/agents/reviewer-<persona>.toml` (Codex), each ejected
per SPEC-0027's contract with the per-persona pins SPEC-0032
REQ-003 specifies. There is no per-persona ejected skill; the
per-persona content lives in the six subagent body files.

<done-when>
- `resources/modules/personas/` contains exactly the six
  reviewer persona body files (`reviewer-business.md`,
  `reviewer-tests.md`, `reviewer-architecture.md`,
  `reviewer-security.md`, `reviewer-style.md`,
  `reviewer-docs.md`) plus N topic-named snippet files. The
  brainstorm-named snippet candidates are
  `verdict_return_contract.md`, `no_tasks_md_writes.md`,
  `inline_note_format.md`, and `diff_fetch_command.md`; the
  exact filename convention is decompose-time, with the
  category constraint that none collide with the
  `reviewer-<persona>.md` pattern.
- No `_partials/` (or analogous `_includes/` / `shared/`)
  subdirectory exists under `resources/modules/personas/`,
  `resources/modules/phases/`, or anywhere else in the source
  tree.
- Each of the six persona body files `{% include %}`s the
  snippet files it needs at MiniJinja build time; the ejected
  `.claude/agents/reviewer-<persona>.md` and
  `.codex/agents/reviewer-<persona>.toml` files contain the
  fully-expanded content with no MiniJinja markup.
- The `reviewer-style` persona body retains its
  "Diff-format pitfalls" section; the `reviewer-tests` persona
  body retains its Evidence-read step; other persona bodies
  do NOT carry these sections — divergence is legitimate and
  preserved.
- No master template file (e.g., `reviewer.md.j2`) exists; the
  six persona body files are the source of truth for
  per-persona content.
- This SPEC does not re-eject reviewer subagent files —
  SPEC-0027's contract still owns the ejection. SPEC-0032
  REQ-003 owns the per-persona pin assignments
  (`reviewer-business` / `reviewer-tests` /
  `reviewer-architecture` at `opus[1m]` / `xhigh`;
  `reviewer-security` at `sonnet[1m]` / `high`;
  `reviewer-style` / `reviewer-docs` at `sonnet[1m]` /
  `medium`).
</done-when>

<behavior>
- Given the post-SPEC source tree, when a contributor inspects
  `resources/modules/personas/`, then they find six body files
  matching `reviewer-<persona>.md` plus topic-named snippet
  files (e.g., `verdict_return_contract.md`); no `_partials/`
  directory exists.
- Given a freshly initialized workspace, when the six ejected
  reviewer subagent bodies under `.claude/agents/` are
  compared pairwise, then each shares the snippet-included
  text byte-identically AND carries persona-specific narrative
  content that legitimately varies (focus list, look-for
  guidance, persona-specific sections like
  `reviewer-style`'s diff-format pitfalls).
- Given the post-SPEC source tree, when a contributor opens
  `resources/modules/personas/verdict_return_contract.md`
  (decompose-time filename), then it contains the shared
  "Your final message to the orchestrator must be a single
  `<review persona=...>` element block..." paragraph reused
  across all six persona bodies.
- Given the post-SPEC source tree, when a contributor
  searches for `reviewer.md.j2` or any equivalent
  master-template file, then zero matches are found.
</behavior>

<scenario id="CHK-013">
Given the post-SPEC source tree,
when the file
`resources/modules/personas/verdict_return_contract.md` (or
equivalent decompose-time filename) is read,
then it exists and contains the shared
verdict-return-contract paragraph; and when each of the six
reviewer body files at
`resources/modules/personas/reviewer-<persona>.md` is parsed
for an `{% include %}` directive referencing that snippet
file, then each persona body contains the include directive
exactly once.
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
### REQ-008: Skill stubs and agent bodies discover speccy resources via CLI JSON envelopes only; general filesystem access is unconstrained

After this SPEC lands, the shipped skill bodies AND agent
bodies (the interactive skill SKILL.md files; the pinned
phase-worker SKILL.md stubs plus their agent files at
`.claude/agents/speccy-<phase>.md` and
`.codex/agents/speccy-<phase>.toml`; the six reviewer subagent
bodies under `.claude/agents/reviewer-<persona>.md`) discover
all **speccy resources** via the CLI's JSON envelopes
(`speccy status --json`, `speccy next --json`,
`speccy vacancy --json`). Speccy resources are: anything
under `.speccy/specs/` (SPEC.md, TASKS.md, MISSION.md,
REPORT.md, evidence files), the slug-pattern directory layout
(`NNNN-slug` flat or one level inside a mission folder), and
any path computation from a SPEC-NNNN id to a filesystem path.

This is a resource-discovery boundary, not a blanket
filesystem-access ban. Skills and agents may use Read / Glob /
grep freely for general project files (source code, test
fixtures, helper modules, project-level configuration such as
`AGENTS.md`, `.editorconfig`, `Cargo.toml`, `.claude/rules/...`)
— those are the implementer's domain and not the CLI's. The
boundary is specifically: any file under `.speccy/specs/` or
named SPEC.md / TASKS.md / MISSION.md / REPORT.md or any path
computation from a SPEC-NNNN id to a filesystem path must come
from a CLI JSON envelope or from a user-supplied argument to
the skill.

<done-when>
- A grep over `resources/modules/skills/`,
  `resources/modules/phases/`, and `resources/modules/personas/`
  for speccy-resource discovery patterns — glob expressions
  like `.speccy/specs/*`, raw filesystem paths ending in
  `SPEC.md` / `TASKS.md` / `MISSION.md` / `REPORT.md` not
  bound to a `{{ ... }}` template placeholder, or
  directory-enumeration instructions targeting `.speccy/specs/`
  — returns zero hits in skill or agent body content.
  Helper-pointer text that names fully-resolved paths supplied
  via `{{ ... }}` placeholders does not count as a match;
  those paths originate from the CLI's JSON envelopes.
- General-purpose Read / Glob / grep references in skill and
  agent bodies against non-speccy paths (e.g., "read
  `AGENTS.md` for project conventions", "grep for an existing
  helper before introducing a new one", "read
  `.editorconfig`") are NOT flagged by the lint; the boundary
  is speccy-resource-scoped.
- Every shipped skill or agent body that needs a SPEC path,
  TASKS path, MISSION path, or REPORT path obtains it from
  either: (a) the user-supplied argument if the skill takes a
  SPEC-NNNN, (b) a JSON envelope from `speccy status` /
  `speccy next` / `speccy vacancy`, or (c) a `{{ ... }}`
  template placeholder that the build-time MiniJinja
  substitution wires to one of the above sources.
- The `speccy-plan` greenfield-form skill body invokes
  `speccy vacancy --json` (not `speccy status --json`) to
  fetch the next SPEC ID, demonstrating the
  payload-minimization pattern.
</done-when>

<behavior>
- Given the post-SPEC source tree, when a contributor greps
  for a literal `.speccy/specs/*` glob expression in skill or
  agent body content, then zero hits appear.
- Given the post-SPEC source tree, when a contributor greps
  for references to project-level files unrelated to speccy
  resources (e.g., "read AGENTS.md", "grep for an existing
  helper"), then matches DO appear in skill and agent body
  content and are NOT considered violations — the boundary is
  speccy-resource-scoped.
- Given the ejected `.claude/skills/speccy-plan/SKILL.md` in
  greenfield mode, when its contents are inspected, then the
  body invokes `speccy vacancy --json` (and not
  `speccy status --json`) for ID allocation.
- Given the ejected `.claude/agents/speccy-work.md`, when its
  body is followed by an agent, then every speccy-resource
  path the agent needs comes from a `speccy next` or
  `speccy status` JSON envelope, or from the SPEC-NNNN/T-NNN
  argument supplied to the skill — never from a glob against
  `.speccy/specs/`.
</behavior>

<scenario id="CHK-014">
Given the post-SPEC source tree,
when a recursive search runs across
`resources/modules/skills/`, `resources/modules/phases/`, and
`resources/modules/personas/` for speccy-resource discovery
patterns — `.speccy/specs/*` glob expressions; raw filesystem
paths ending in `SPEC.md` / `TASKS.md` / `MISSION.md` /
`REPORT.md` that are not bound to a `{{ ... }}` template
placeholder; directory-enumeration instructions targeting
`.speccy/specs/` — then zero matches appear in skill or agent
body content.
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

<decision id="DEC-008">
**Eject shape follows skill invocation pattern, not name or phase number.**

A Speccy skill's eject shape is determined by its invocation
pattern. Skills whose body runs in a recurring multi-agent
orchestration loop — one invocation per task implemented, one
invocation per spec decomposed, where each invocation is
independent grunt work that benefits from a heavy-model fork —
eject as a thin SKILL.md host-router stub at
`.claude/skills/<skill>/SKILL.md` plus a full-body agent file
at `.claude/agents/<skill>.md` (and the Codex equivalent at
`.codex/agents/<skill>.toml`). The SKILL.md is the
slash-command entry point in the parent session; the agent
file is invoked via `/agent <skill>` for the pinned
heavy-model fork. Skills whose body runs as interactive
conversation requiring per-turn user input — Socratic Q&A,
SPEC drafting, amendment orchestration, multi-persona review
verdict consolidation — eject as full-body SKILL.md only,
with no agent counterpart. The pinned phase workers
(`speccy-tasks`, `speccy-work`, `speccy-ship`) are the first
category. The interactive skills (`speccy-init`,
`speccy-brainstorm`, `speccy-plan`, `speccy-amend`,
`speccy-review` orchestrator) are the second.

This rule earned its place through SPEC-0032's auto-fork
retreat: T-002 of SPEC-0032 surfaced that auto-forking every
slash-command invocation imposed a multi-minute UX cost that
made the pinned-model benefit not worth the latency. The
opt-in `/agent speccy-<phase>` path retreated to the
recurring-loop phases, and the interactive skills stayed in
the parent session. This SPEC names the rule explicitly so
future skill additions apply it as a principle rather than
re-debating each new skill.

Trade-off acknowledged: a future hybrid skill that wants both
an interactive front-end and a forkable heavy phase would
fall between categories. The rule's binary shape is not
load-bearing forever; it's load-bearing for the current set
of shipped skills and any skill that fits cleanly into one of
the two categories. A hybrid would warrant its own DEC at
that time.
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
- Shared snippets live in the same folder as their primary
  consumer (`resources/modules/personas/` for reviewer
  snippets; `resources/modules/phases/` for any phase-worker
  snippets), distinguished from body files by filename
  pattern rather than directory. Topic-named filenames are the
  preferred convention (e.g., `verdict_return_contract.md`).
  Cross-folder includes are explicit and rare. No `_partials/`
  subdirectory exists at any level. Decomposition phase picks
  exact filenames.
- The `speccy_core::prompt::allocate_next_spec_id` function
  (or its successor) remains the single authority on ID
  allocation even after `prompt::template.rs` and
  `prompt::budget.rs` are removed. The function is
  filesystem-only; it has no dependency on the template
  loader.
- Reviewer subagent body ejection produces six distinct
  `.claude/agents/reviewer-<persona>.md` files (one per
  persona) per SPEC-0027's contract; that contract is
  unchanged. SPEC-0032 REQ-003 owns the per-persona pin
  assignments. This SPEC's REQ-007 affects only how those
  bodies are SOURCED in the resources tree (six independent
  files plus shared snippets, not one master template), not
  how they EJECT.
- The shared body source at
  `resources/modules/phases/speccy-<phase>.md` (moved from
  `resources/modules/skills/` per SPEC-0032 DEC-009 / T-009)
  is included by the pinned phase-worker agent body template
  (which renders to `.claude/agents/speccy-<phase>.md` plus
  the Codex agent TOML's body field). The pinned-phase
  SKILL.md stub for each pinned phase worker is hardcoded
  thin prose and does NOT include the phase body — the agent
  file is the single on-disk source of truth for phase-body
  content per SPEC-0032 REQ-010.
- The Skip-on-exists semantic SPEC-0027 introduced is a
  user-edit-protection mechanism. Removing it under the new
  three-way classification (create-if-absent /
  no-op-if-byte-identical / refuse-or-overwrite) trusts git
  as the recovery path. Speccy v1 is unreleased; no
  downstream users depend on the protection. Within the
  dogfooded `.speccy/` workspace, recovery from accidental
  `--force` overwrites uses `git checkout` of the prior
  file content.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-19 | human/kevin | Initial draft. The CLI's two architectural jobs (mechanical state queries vs. authored prompt rendering) are decoupled: five prompt-rendering verbs (`plan`, `tasks` render-form, `implement`, `review`, `report`) and the `trim_to_budget` mechanism are deleted, phase prompt bodies eject as Skip-on-exists SKILL.md files at `init` time, and two new flat verbs (`speccy lock`, `speccy vacancy`) take the real-CLI-work that hid inside the deleted rendering paths. `status` and `next` JSON envelopes bump to schema_version 2 with resolved paths plus derived `next_action`; `next` drops `--kind` because spec state fully determines action kind via the priority rule `review > implement > ship` (drift-visibility favors short feedback loops). CLI resolves all filesystem paths; skills consume via JSON only. Upstream skill authoring uses MiniJinja partials for shared snippets; the six reviewer prompts collapse to one shared template with a tests-only conditional block. Direct extension of SPEC-0023 (REQ-005, REQ-006) and SPEC-0027: when host machinery already delivers content to the agent through some other channel (Read primitive, sub-agent system context, ejected skill body), the CLI prompt-rendering surface stops carrying redundant copies. Final CLI shape: seven flat verbs (`init`, `status`, `next`, `check`, `verify`, `lock`, `vacancy`), each doing one job, no mode flags, `--json` for representation only. |
| 2026-05-19 | human/kevin | Amendment for SPEC-0032 sequencing dependency. SPEC-0032 (per-phase model and effort pinning across the lifecycle) is now a hard sequencing predecessor: it adds skill frontmatter (`context: fork` + `agent:` target on `speccy-tasks` / `speccy-work` / `speccy-ship` / `speccy-init`; direct `model: haiku` on `speccy-review`), four `.claude/agents/speccy-<phase>.md` subagent files, and five `.codex/agents/speccy-<phase>.toml` agent files that this SPEC's ejection must preserve. REQ-006 extends the Skip-on-exists ejection set to include those new files and asserts that ejected SKILL.md frontmatter carries the SPEC-0032 pins through MiniJinja round-trip; the Codex SKILL.md pointer line naming `.codex/agents/<phase>.toml` is required content. REQ-007 reframes the shared reviewer template as the **subagent body** template (six `.claude/agents/reviewer-<persona>.md` bodies from one source under `resources/modules/personas/`), not a phantom set of per-persona skills — SPEC-0032 establishes that `/speccy-review` is a single orchestrator skill that dispatches to six existing subagents. New scenarios CHK-012a (subagent body matches SKILL.md body), CHK-012b (Codex pointer line and agent TOML pinned correctly), CHK-013a (no per-persona review skill exists). New Assumption: `resources/modules/skills/<phase>.md` is dual-consumer (SKILL.md template + subagent body template), and the reviewer side mirrors this with one persona source feeding six subagent bodies. The "per-persona ejected reviewer skill naming" open question is resolved (struck) by the SPEC-0032 dispatch model. |
| 2026-05-19 | claude/opus-4.7 + human/kevin | Reconciliation amendment against SPEC-0032's actual ship shape; the prior 2026-05-19 row anticipated SPEC-0032's brainstorm framing, not its ship shape. Mechanical corrections: THREE Claude Code phase-worker agents (`tasks` / `work` / `ship`), not four; THREE Codex agent TOMLs for the same phases, not five; no `speccy-init` agent file (SPEC-0032 DEC-009); no `speccy-review` agent file (SPEC-0032 REQ-002 / REQ-009 keeps the orchestrator in the parent session for Task-tool access and verdict-consolidation capacity); shared phase-body source path is `resources/modules/phases/<phase>.md` (moved from `resources/modules/skills/` per SPEC-0032 T-009); model aliases require the `[1m]` 1M-context-window suffix on Claude Code (`sonnet[1m]`) and `gpt-5.5` on Codex; no Haiku pin anywhere; `context: fork` was dropped after SPEC-0032 T-002 surfaced its multi-minute UX cost; pinned-phase SKILL.md files are thin stubs ≤10 non-blank lines per SPEC-0032 REQ-010 (not byte-identical to the agent body modulo frontmatter); reviewer pins asymmetric per SPEC-0032 REQ-003 (Opus[1m] xhigh for business / tests / architecture; Sonnet[1m] high for security; Sonnet[1m] medium for style / docs). Three substantive scope shifts confirmed during the 2026-05-19 brainstorm: (1) Skip-on-exists removed entirely; per-file three-way classification (create-if-absent / no-op-if-byte-identical / refuse-or-overwrite) applies uniformly to every file `init` writes including SPEC-0027 reviewer files; git is the recovery path. (2) REQ-007 reframes from "one master MiniJinja template with `{% if persona == 'tests' %}`" to "partial consolidation": six persona body files stay independent under `resources/modules/personas/`; shared blocks factor into topic-named snippets co-located in the same folder; no `_partials/` subdirectory exists at any level. (3) REQ-008 reframes from blanket no-globbing to a speccy-resource-discovery boundary: skill and agent bodies may use Read / Glob / grep for general project files but not to discover `.speccy/specs/*`, SPEC.md, TASKS.md, MISSION.md, REPORT.md, or the slug-pattern layout. New DEC-008 ("eject shape follows skill invocation pattern") promotes the SPEC-0032 auto-fork retreat lesson to a durable rule. Scenarios reconciled: CHK-012 and CHK-016 deleted (Skip-on-exists user-edit survival; byte-identical agent-vs-SKILL body); CHK-013 rewritten around topic-named snippet includes; CHK-017 loosened to ≤10-line stub with `gpt-5.5`; new CHK-019 / CHK-020 / CHK-021 cover the three-way classification; new CHK-022 asserts no-init-agent and no-review-agent existence per DEC-008. |
</changelog>

## Open Questions

- [x] ~~Naming for the per-persona ejected reviewer skills: one
  skill `speccy-review` that accepts `--persona` and ejects
  one SKILL.md, or six `speccy-review-<persona>` skills?~~
  Resolved by the SPEC-0032 dependency: `/speccy-review` is a
  single orchestrator skill that dispatches via the Task tool
  to the six existing reviewer subagents at
  `.claude/agents/reviewer-<persona>.md` (and Codex
  equivalents). There is no per-persona ejected skill; the
  per-persona body content lives in the subagent files
  SPEC-0027 already established, and the model pins live on
  those subagent files per SPEC-0032 REQ-003. (Note: the
  pre-amendment resolution text named Haiku as the
  orchestrator pin; SPEC-0032 actually shipped
  `/speccy-review` unpinned in the parent session per
  REQ-002 / REQ-009. The orchestrator-dispatch shape is
  unchanged.)
- [ ] Whether `speccy vacancy`'s implementation reuses
  `allocate_next_spec_id` directly from speccy-core, or
  whether the function relocates (e.g. out of
  `prompt::id_alloc` into a more general
  `speccy_core::specs::next_id`) as part of the speccy-core
  cleanup that REQ-001 implies. Decompose-time decision.
- [x] ~~Exact path under `resources/modules/skills/` for shared
  partials. Candidates: `_partials/`, `_includes/`, `shared/`.~~
  Resolved by 2026-05-19 brainstorm: no `_partials/`-style
  subdirectory exists at any level. Topic-named snippet files
  live INSIDE the consuming folder
  (`resources/modules/personas/`,
  `resources/modules/phases/`), distinguished from body files
  by filename pattern. See REQ-007.
- [ ] Exact filename convention for shared snippets inside
  `modules/personas/` and `modules/phases/`. Brainstorm
  proposed topic-named (e.g., `verdict_return_contract.md`); a
  prefix-marked alternative (e.g., `_shared_verdict.md`) was
  considered and rejected because the existing folders use no
  prefix convention. Decompose-time picks exact names; SPEC
  names the category constraint only (must not collide with
  the `reviewer-<persona>.md` or `speccy-<phase>.md`
  patterns).
- [ ] Whether `--force` stdout summary lists each overwritten
  file individually or just gives a tally. UX detail;
  SPEC-level position is "log per-file with `(!) overwritten`
  marker plus a final summary tally," but the exact line
  format is decompose-time.
- [ ] Whether the `speccy-review` orchestrator's body source
  belongs in `resources/modules/skills/` (current location,
  consistent with other interactive skills under DEC-008) or
  elsewhere given its dispatch-to-personas role. Brainstorm
  leaned toward staying in `modules/skills/`; decompose-time
  decision.
