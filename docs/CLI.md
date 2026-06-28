# Speccy CLI

> The command contract: every `speccy` verb, its flags, its JSON
> envelopes, and the operational details that decide how each command
> behaves.
>
> Part of the Speccy docs set: [ARCHITECTURE](./ARCHITECTURE.md) (design
> rationale) · CLI (commands, this file) · [SCHEMA](./SCHEMA.md) (file
> formats + lints) · [WORKFLOW](./WORKFLOW.md) (loop + harness).

The CLI is intentionally thin. It renders prompts deterministically,
queries workspace state, records hashes, and runs proof-shape lint. It
never calls an LLM and never renders natural-text phase prompts. Phase
prose lives in the shipped skill bodies (see [WORKFLOW.md](./WORKFLOW.md)).

---

## CLI surface

A small set of flat commands. Each has one job. `--json` toggles
representation, never content; there are no other mode flags and no
per-phase rendering verbs. The lifecycle write commands
(`task transition`, `journal append`) are mechanical state writes over a
closed grammar. They record a transition or append a validated block,
not a phase prompt.

```text
speccy init                       Scaffold .speccy/ + host skill pack.
                                    --host claude-code | codex (auto-detected if omitted)
                                    --force            overwrite differing shipped files
speccy status [SELECTOR]          Workspace overview; spec subset by default.
                                    no arg:              attention-list view
                                    SPEC-NNNN:           one spec, unfiltered
                                    --all:               every spec, unfiltered
                                    --include-archive:   also scan `.speccy/archive/`
                                    --json:              schema_version=1 envelope with resolved paths
speccy next [SPEC-ID]             Next actionable per spec, derived from state.
                                    no arg:              every active spec with next_action
                                    SPEC-ID:             one spec or {next_action: null, reason}
                                    --include-archive:   also scan `.speccy/archive/`
                                    --json:              schema_version=1 envelope
                                  Action kind is derived (review > work > vet > ship,
                                  with `decompose` when TASKS.md is absent); spec
                                  state fully determines the kind, so there is no
                                  caller-supplied `--kind` filter. Workspace-form
                                  empty state exits with code 2 and
                                  `reason="no_active_specs"`.
speccy check [SELECTOR]           Render check scenarios (no execution).
                                    no arg:              every scenario across every spec
                                    SPEC-NNNN:           every scenario under one spec
                                    SPEC-NNNN/CHK-NNN:   one scenario, spec-qualified
                                    SPEC-NNNN/T-NNN:     scenarios covering a qualified task
                                    CHK-NNN:             every spec's CHK-NNN
                                    T-NNN:               scenarios covering an unqualified task
                                    --include-archive:   also scan `.speccy/archive/`
speccy context SELECTOR           Emit one JSON bundle for loop subagent entry reads.
                                  Task selectors (`T-NNN`, `SPEC-NNNN/T-NNN`) emit the
                                  existing task-scoped bundle and keep the same
                                  ambiguity / not-found diagnostic classes as
                                  `speccy check`. Bare spec selectors (`SPEC-NNNN`)
                                  emit a spec-scoped vet bundle. Failures exit
                                  non-zero with no partial stdout. A pure read command:
                                  performs no writes anywhere.
                                    --json:              schema_version=1 envelope (first field).
                                                         `--json` toggles representation, never
                                                         content; agents always pass it. There is no
                                                         content-mode flag.
                                  Task envelope sections:
                                    identity     spec frontmatter id, title, status
                                    intent       <goals>, <non-goals>, every <decision>
                                                 with its DEC id + body (Summary,
                                                 user-stories, notes, and non-covered
                                                 requirement bodies excluded)
                                    task         the selected task's verbatim <task> body
                                                 bytes plus parsed id, state, covers
                                    requirements each covering requirement in full
                                                 (title, body, done-when, behavior,
                                                 scenarios), resolved via the same shared
                                                 speccy-core walk `speccy check` uses,
                                                 deduplicated in declared order
                                    journal      the per-task journal sliced to its
                                                 latest round: `blocks` holds that
                                                 round's <implementer>/<review>/
                                                 <blockers> entries with full bodies,
                                                 and `prior_rounds` is an
                                                 attributes-only index (block, date,
                                                 round, + model/persona/verdict per
                                                 type, no body) of every pre-latest
                                                 block. Prior-round prose is never
                                                 inlined; drill into it on demand with
                                                 `speccy journal show <selector>
                                                 --round N`. Absent journal is an
                                                 explicit empty marker (`exists:false`,
                                                 empty `blocks`/`prior_rounds`), not an
                                                 error (exit 0)
                                    siblings     every other task as id, state, covers
                                                 only, never bodies
                                    paths        repo-relative SPEC.md, TASKS.md, journal
                                                 paths for follow-up targeted reads
                                    diff_command suggested merge-base diff string against
                                                 the default branch, runnable as-is from
                                                 the repo root; git unavailability
                                                 degrades this field, never errors
                                    consistency  workspace status plus only the drift
                                                 entries matching the selected task;
                                                 never refuses on drift
                                  Spec envelope sections:
                                    identity     spec frontmatter id, title, status
                                    intent       <goals>, <non-goals>, every <decision>
                                                 with DEC id + body
                                    requirements every requirement contract in full, in
                                                 declared order
                                    tasks        every task as id, state, covers, title
                                                 only, never bodies
                                    non_completed_tasks
                                                 same compact shape, filtered to tasks
                                                 whose state is not `completed`
                                    vet_journal  absent VET.md is `exists:false`;
                                                 otherwise the latest invocation's blocks
                                                 are inlined with bodies and prior
                                                 invocations are attributes-only indexes
                                                 with no bodies
                                    paths        repo-relative SPEC.md, TASKS.md, and VET.md
                                                 paths for follow-up targeted reads
                                    diff_command working-tree diff against the default branch,
                                                 runnable as-is from the repo root; includes
                                                 uncommitted holistic changes and degrades on
                                                 git probe failure
                                  Task size invariant (contract, not implementation detail):
                                  the bundle scales with the task, not the spec. For a
                                  fixed task, growing the spec changes the bundle only in
                                  bounded ways: one added sibling adds exactly one
                                  `siblings` entry; an uncovered requirement adds
                                  nothing; a journal round on another task adds nothing.
                                  Within the task itself, the journal section scales with
                                  the latest round plus a bounded index: each prior round
                                  adds only its attributes-only `prior_rounds` entries,
                                  never its bodies. Enforced by a property-style test, not
                                  left as prose.
speccy verify                     CI gate: proof-shape validation only.
                                    --include-archive:   also scan `.speccy/archive/`
                                    --json:              schema_version=1 envelope
                                    parse errors, requirements with no scenarios,
                                    unresolved scenario refs, stale task hash, etc.
                                    Does NOT run project tests; that's CI's job.
speccy lock SPEC-NNNN             Record SPEC.md sha256 + UTC timestamp into TASKS.md
                                  frontmatter (`spec_hash_at_generation`,
                                  `generated_at`). Used by `/speccy-decompose` after
                                  decomposition.
speccy task transition SELECTOR   Rewrite one task's `state` over the closed legal
                                  graph (byte-surgical splice; every other byte of
                                  TASKS.md is preserved verbatim).
                                    --to <state>: pending | in-progress | in-review
                                                  | completed (unknown rejected at
                                                  argument-parse time)
                                  Legal edges: pending→in-progress,
                                  in-progress→in-review, in-review→completed,
                                  in-review→pending, in-progress→pending,
                                  completed→pending. A same-state target is an
                                  idempotent no-op (exit 0, file byte-identical);
                                  any other edge or an unresolved selector exits
                                  non-zero with TASKS.md untouched.
speccy journal append SELECTOR    Append one validated block to a journal, body from
                                  stdin. The CLI is the sole authority for every
                                  environment-derivable attribute: `date`, `round`,
                                  frontmatter `generated_at`, a `gate` block's
                                  `tasks_hash`, and VET.md invocation numbering; so
                                  there is no flag to override any of them.
                                    --block <type>: implementer | review | blockers
                                                    (task journal) or drift-review |
                                                    holistic-fix | simplifier-scan |
                                                    simplifier-apply | gate (VET.md)
                                    --model <s>:    identity for implementer/review and
                                                    the round-bearing vet blocks
                                    --persona <n>:  reviewer persona (review blocks)
                                    --verdict <v>:  review + every vet block
                                  Block type implies the target: task block types take a
                                  task selector → `journal/<task-id>.md`; vet block types
                                  take a bare `SPEC-NNNN` → VET.md. An acquire-before-read
                                  advisory per-file lock (10s timeout) serializes
                                  concurrent appenders; validation runs before any write,
                                  so a rejected block leaves the journal byte-identical
                                  (or still absent). An `implementer` block is additionally
                                  refused when its `Evidence:` roll call labels a CHK
                                  `demonstrated` while the canonical evidence file
                                  `evidence/T-NNN.md` is absent or carries no `### Scenario`
                                  heading; the error names the offending CHK id(s), the
                                  expected evidence path, and whether the file was missing
                                  or present-without-a-scenario.
speccy journal show SELECTOR      Parse the resolved journal and emit its frontmatter
                                  and blocks, filtered.
                                    --round <latest|N>: highest round, or round N
                                                        (scoped to the last invocation
                                                        section for VET.md)
                                    --verdict <value>:  blocks whose verdict matches
                                    --block <type>:     blocks of one element type
                                    --json:             schema_version=1 envelope
                                  Filters compose conjunctively; `--json` toggles
                                  representation, never content. A missing journal
                                  exits non-zero.
speccy vacancy                    Return the next free `SPEC-NNNN`.
                                    no arg:   bare `SPEC-NNNN\n` to stdout
                                    --json:   {schema_version: 1, next_spec_id: "SPEC-NNNN"}
                                  Used by `/speccy-plan` so the skill never
                                  globs `.speccy/specs/` itself. The scan covers
                                  both `.speccy/specs/` and `.speccy/archive/`
                                  so archived IDs remain reserved.
speccy archive SPEC-NNNN          Relocates a shipped, dropped, or superseded SPEC
                                  from `.speccy/specs/NNNN-slug/` to
                                  `.speccy/archive/NNNN-slug/` via `git mv`;
                                  archived specs retain their SPEC-NNNN IDs and
                                  are invisible to hot-path commands.
                                    --reason "<text>": single-line note recorded
                                                       in `archived_reason:` frontmatter
                                    --force:           bypass the status gate (allows
                                                       archiving an `in-progress` spec)
                                    --json:            schema_version=1 receipt envelope
```

Phase prose lives in skill content under `.claude/skills/...` and
`.agents/skills/...`, not in the CLI. There is no `speccy plan` /
`speccy tasks` (render-form) / `speccy implement` / `speccy review` /
`speccy report` verb; conflating "what loop am I in" with "which
sub-type of reviewer" through a `--persona` flag on a generic `prompt`
command would be the wrong abstraction. Persona selection lives in the
`/speccy-review` skill, which dispatches to the matching
`reviewer-<persona>` sub-agent file (see [WORKFLOW.md](./WORKFLOW.md)).

The CLI is the sole authority on the spec directory rule (`NNNN-slug`
flat or one level inside a mission folder). Skills read paths from
`speccy status --json` / `speccy next --json` / `speccy vacancy --json`
rather than globbing the filesystem; the JSON envelopes carry
`spec_md_path`, `tasks_md_path`, and `mission_md_path` (nullable when
absent).

That is the complete public surface. Anything else is a skill
responsibility.

---

## JSON interfaces

A handful of commands carry stable JSON contracts: `status`, `next`,
`vacancy`, `verify`, `archive` (the archive receipt form), and
`journal show`. `--json` switches representation; the content is the
same as the text output. Schema versions are pinned per-envelope and
bumped only on breaking shape changes.

### `speccy status --json`

```json
{
  "schema_version": 1,
  "repo_sha": "abc123",
  "specs": [
    {
      "id": "SPEC-0001",
      "slug": "user-signup",
      "title": "User signup",
      "status": "in-progress",
      "supersedes": [],
      "superseded_by": [],
      "tasks": {
        "open": 3,
        "in_progress": 1,
        "awaiting_review": 0,
        "done": 2
      },
      "stale": false,
      "stale_reasons": [],
      "open_questions": 1,
      "lint": {
        "errors": [],
        "warnings": [],
        "info": []
      },
      "spec_md_path": ".speccy/specs/0001-user-signup/SPEC.md",
      "tasks_md_path": ".speccy/specs/0001-user-signup/TASKS.md",
      "mission_md_path": null
    }
  ],
  "lint": {
    "errors": [],
    "warnings": [],
    "info": []
  }
}
```

By default `speccy status` shows only specs with `status: in-progress`
plus any with stale evidence or lint errors regardless of status. Pass a
positional `SPEC-NNNN` selector for one spec, or `--all` to render every
spec in workspace order. Specs with `status: implemented`, `dropped`, or
`superseded` are excluded from the default view but always present in
`--json` output so harnesses can filter as needed.

Per-spec entries carry resolved paths (`spec_md_path`, `tasks_md_path`,
`mission_md_path`) as repo-relative forward-slash strings.
`tasks_md_path` is `null` when TASKS.md does not yet exist;
`mission_md_path` is `null` when the spec lives flat (no mission
folder). The `superseded_by` field is **computed** at query time by
walking every parsed SPEC.md's `frontmatter.supersedes` and inverting
the relation; it does not appear on disk.

A few per-spec fields are omitted from the envelope when absent (serde
`skip_serializing_if`) rather than serialised as `null`:

- `parse_error`: first parse error encountered when loading the spec,
  when frontmatter or element-tree parsing failed.
- `archived_at`: UTC archive date (`YYYY-MM-DD`) from the `archived_at`
  frontmatter field. Non-archived specs emit no key.
- `archived_reason`: free-form archive reason from the
  `archived_reason` frontmatter field, when present.

The top-level `lint` block carries workspace-level diagnostics (those
not attributable to any single spec). Per-spec diagnostics live on the
matching `specs[]` entry.

### `speccy next --json`

Workspace form (no positional selector): every active spec with its
derived `next_action`:

```json
{
  "schema_version": 1,
  "specs": [
    {
      "spec_id": "SPEC-0001",
      "next_action": { "kind": "review", "task_id": "T-002" },
      "spec_md_path": ".speccy/specs/0001-user-signup/SPEC.md",
      "tasks_md_path": ".speccy/specs/0001-user-signup/TASKS.md",
      "mission_md_path": null,
      "consistency": { "status": "ok", "drifts": [] }
    },
    {
      "spec_id": "SPEC-0002",
      "next_action": { "kind": "decompose" },
      "spec_md_path": ".speccy/specs/0002-password-reset/SPEC.md",
      "tasks_md_path": null,
      "mission_md_path": null,
      "consistency": { "status": "ok", "drifts": [] }
    }
  ]
}
```

Per-spec form (positional `SPEC-NNNN`): one entry, or
`{ next_action: null, reason }` when the spec is `completed`, `dropped`,
or `superseded`. Action kind is derived from spec state via the priority
rule `review > work > vet > ship`, with `decompose` when TASKS.md is
absent. There is no `--kind` flag: spec state fully determines the kind,
so caller-supplied filtering would be redundant. Skills that want only
one kind read the envelope and filter on `next_action.kind` themselves.
The workspace form exits with code 2 and adds a top-level
`reason="no_active_specs"` field when no active spec remains. Per-spec
envelopes likewise carry a top-level `reason` field, `"completed"`,
`"dropped"`, or `"superseded"`, when `next_action` is `null`; the field
is omitted otherwise.

Every envelope entry (per-spec and each workspace `specs[]` entry)
carries a `consistency` block alongside `next_action`; the shape and
semantics follow.

#### `consistency` block

Every per-spec `speccy next --json` envelope carries a top-level
`consistency` object alongside `next_action`:

```json
{
  "consistency": {
    "status": "ok" | "drift" | "blocked",
    "drifts": [ /* DriftEntry, ... */ ]
  },
  "next_action": { "kind": "reconcile" | "work" | "review" | "vet" | "ship" | "decompose", ... }
}
```

`status` values:

- `"ok"`: no drift detected. `drifts` is `[]`. The `next_action` is
  whatever normal spec-state dispatch resolved.
- `"drift"`: one or more `auto_fixable` entries, no `blocking` entries.
  The reconcile pass can land all fixes without user intervention.
- `"blocked"`: at least one `blocking` entry. Recovery requires the
  dispatched reconcile actions from
  `.claude/speccy-references/reconcile-policy.md`.

**Override rule:** when `consistency.status != "ok"`, `next_action.kind`
is always `"reconcile"`. Other `next_action` fields (e.g. `task_id`,
paths) remain as normal spec-state dispatch would have set them, so the
reconcile pass knows which task the drift relates to.

`drifts[]` entries share this shape:

```json
{
  "task_id": "T-NNN",
  "kind": "<DriftKind>",
  "severity": "auto_fixable" | "blocking",
  "tasks_state": "pending" | "in-progress" | "in-review" | "completed",
  "details": { /* per-kind, see below */ }
}
```

The `kind` values and their `details` shapes:

| `kind` | `severity` | `details` shape |
|---|---|---|
| `commit_without_state` | `auto_fixable` | `{ "commit_sha": "<40-hex>", "commit_short_sha": "<8-hex>" }` |
| `state_completed_no_commit` | `blocking` | `{ "expected_trailer": "[SPEC-NNNN/T-NNN]:", "working_tree_dirty": true \| false }` |
| `state_in_progress_orphaned` | `blocking` | `{ "working_tree_dirty": true \| false, "dirty_files_count": <usize> }` |
| `state_in_progress_clean` | `blocking` | `{ "working_tree_dirty": false }` |
| `journal_xml_malformed` | `blocking` | `{ "journal_path": "<path>", "last_well_formed_byte_offset": <usize> }` |

Each `kind` describes exactly one class of drift between TASKS.md state,
git log, the working tree, and the per-task journal file:

- `commit_without_state`: a commit titled `[SPEC-NNNN/T-NNN]: ...`
  exists in git log but TASKS.md still marks the task at a
  non-`completed` state. The reconcile pass flips the task state forward
  to `completed`.
- `state_completed_no_commit`: TASKS.md marks the task `completed` but
  no matching commit exists. The `working_tree_dirty` boolean
  distinguishes the two recovery branches: dirty → reconstruct the
  commit; clean → roll the task state back to `in-review`.
- `state_in_progress_orphaned`: TASKS.md marks the task `in-progress`,
  the working tree has uncommitted changes, and no matching commit
  exists. Indicates a crashed implementer. Reconcile rolls the state
  back to `pending` and discards the partial work.
- `state_in_progress_clean`: TASKS.md marks the task `in-progress`, the
  working tree is clean, and no matching commit exists. Indicates a
  crashed implementer whose partial work was already discarded (or never
  reached disk). Reconcile rolls the state back to `pending` without any
  git mutation. The reconcile pass owns this case autonomously.
- `journal_xml_malformed`: the per-task journal file
  (`.speccy/specs/NNNN-slug/journal/T-NNN.md`) failed XML parse.
  `last_well_formed_byte_offset` is the byte offset of the last
  successfully parsed element close; reconcile truncates to that offset
  and re-aligns the TASKS.md state to whatever the truncated journal
  implies.

**CLI stays read-only.** The consistency check is detection-only. The
binary never invokes `git add`, `git commit`, `git restore`,
`git clean`, or `git stash`. All mutation lives in the reconcile pass
dispatched by the skill layer.

**Extending the enum.** The `kind` enum is extensible. Adding a new
drift kind is a two-change procedure:

1. Add the variant + detection logic in the Rust source (the `DriftKind`
   enum in `speccy-core` and its detection branch in the consistency
   check). Detection must stay read-only: no mutating git commands, no
   writes to TASKS.md or the journal.
2. Add the matching row to the policy table in
   `resources/modules/references/reconcile-policy.md`, then run
   `just reeject` to re-render the ejected host-shared copies at
   `<host>/speccy-references/reconcile-policy.md`.

No other site needs to change: the consuming skill bodies carry only a
summary plus a pointer to the ejected file, so they pick up new rows
without editing. The CLI knows what it *detected*; the policy file knows
what to *do*.

### `speccy vacancy --json`

```json
{ "schema_version": 1, "next_spec_id": "SPEC-0036" }
```

Used by `/speccy-plan` so the skill never globs `.speccy/specs/` itself.
One field, one query, smallest possible payload.

### `speccy verify --json`

```json
{
  "schema_version": 1,
  "repo_sha": "abc123",
  "lint": {
    "errors": [],
    "warnings": [],
    "info": []
  },
  "summary": {
    "lint": {
      "errors": 0,
      "warnings": 0,
      "info": 0
    },
    "shape": {
      "specs": 35,
      "requirements": 142,
      "scenarios": 287,
      "errors": 0
    }
  },
  "passed": true
}
```

The top-level `lint` block carries the structured diagnostics (errors /
warnings / info) grouped by severity. The `summary` block mirrors the
text output's counts: `summary.lint` holds the post-demotion lint counts
(gating errors after in-progress demotion, plus warning and info
totals), and `summary.shape` holds the structural counts walked from the
workspace (specs, requirements, scenarios) plus a redundant `errors`
count that mirrors `summary.lint.errors`. `passed` is `true` iff the
process exit code is 0.

There are no `outcome`, `exit_code`, or `duration_ms` fields; the binary
exit code is the contract for CI scripts, and the JSON payload is for
downstream tooling that wants structured failure detail. Diagnostics on
`in-progress` / `dropped` / `superseded` specs are demoted to
`Level::Info` so only `implemented` specs gate the build.

### `speccy journal show --json`

For a task journal (`schema_version` first, then the frontmatter fields
and the filtered blocks):

```json
{
  "schema_version": 1,
  "spec": "SPEC-0042",
  "task": "T-001",
  "generated_at": "2026-05-11T18:00:00Z",
  "latest_round": 2,
  "blocks": [
    {
      "block": "review",
      "date": "2026-05-11T19:00:00Z",
      "round": 2,
      "model": "claude-opus-4-8[1m]/high",
      "persona": "security",
      "verdict": "blocking",
      "body": "bcrypt cost 10; policy requires >=12."
    }
  ]
}
```

For VET.md the envelope keeps `schema_version`, `spec`, `generated_at`,
and `latest_round`, and replaces the top-level `task` / `blocks` with
`invocations` (each carrying its `number`, `date`, and `blocks`). Each
block object carries the attributes its type defines plus its `body`.
The `--round` / `--verdict` / `--block` filters compose conjunctively
over the emitted blocks; `latest_round` reports the highest round
present after filtering. The orchestrator's completeness and blocking
read-back call sites parse this envelope rather than re-scanning the
journal markup.

These envelopes are everything a harness needs. The rest of the CLI
surface is text output to humans.

---

## Command behavior

Operational choices that decide how each command resolves. The file
formats those commands read and the lint codes `verify` emits live in
[SCHEMA.md](./SCHEMA.md).

### Spec ID allocation

Global ID space. `speccy vacancy` walks `.speccy/specs/**/SPEC.md` and
`.speccy/archive/**/SPEC.md` across every mission folder and every flat
(ungrouped) spec, finds the maximum `NNNN-` prefix, and increments.
SPEC-NNN IDs are unique repo-wide regardless of which mission folder a
spec sits in. Moving a spec into or out of a mission folder does not
change its ID, and archived specs continue to reserve their IDs. Gaps
left by dropped specs are not recycled.

### `speccy init` behavior

Refuses to run if `.speccy/` already exists, unless `--force` is passed.
Before doing anything destructive, prints the list of files that would
be created or overwritten.

Host detection for the skill-pack copy:

1. `--host <name>` flag if passed (`claude-code` or `codex`)
2. Presence of `.claude/` → Claude Code
3. Presence of `.codex/` → Codex
4. Presence of `.cursor/` → error out with `InitError::CursorDetected`
   (Cursor is not a supported host pack; the project must pass an
   explicit `--host claude-code` or `--host codex` to override)
5. Fall back to `claude-code` and print a warning

The user can re-run `speccy init --host <other> --force` to swap.

Init renders the per-host wrappers into host-native locations (the file
map and which sub-agents ship per host live in
[SCHEMA.md → File layout](./SCHEMA.md#file-layout) and
[WORKFLOW.md → What ships](./WORKFLOW.md#what-ships)). Existing files are
handled by a three-way per-file classification: absent → `created`;
byte-identical to planned content → `unchanged`; differs from planned
content → `Conflict`, and the entire batch refuses atomically unless
`--force` is passed, in which case differing files are overwritten with
the `(!) overwritten` log marker. Recovery from an unwanted overwrite is
via `git checkout`; there is no in-CLI merge or backup mechanism. The
rule is uniform: every rendered host-pack file follows the same Create /
Unchanged / Conflict classification with no per-file exception.

### `speccy verify` exit code

Binary. `0` if proof shape is intact (specs parse, every requirement has
at least one scenario, every referenced scenario resolves, no scenarios
are unreferenced); `1` otherwise. `speccy verify` does not execute
project tests; CI runs the project's own test commands alongside it.
Detailed breakdown is available via `speccy verify --json`. CI scripts
only check the exit code; downstream tooling parses the JSON if it needs
structured failure info.

`speccy verify` is the only command that exits non-zero on findings.
Everything else surfaces problems and exits zero, so drift stays loud
while the CLI never blocks you mid-loop.

### `speccy next` priority

Per-spec, the derived `next_action.kind` follows
`review > work > vet > ship`, with `decompose` when TASKS.md is absent.
`vet` fires when every task is `state="completed"` but the pre-ship
`journal/VET.md` gate artifact is missing or stale (no trailing
`<gate verdict="passed" tasks_hash="...">` block whose hash matches the
current TASKS.md SHA-256); `ship` fires once the vet gate is fresh and
REPORT.md is absent. Drift visibility favours short feedback loops: bugs
caught in the piecewise (implement → review → implement → review)
workflow are cheap, while bugs caught after multiple tasks build on top
of an inherited mistake are expensive, so the default nudges agents
toward piecewise. Callers that want batched implementation override by
invoking `/speccy-work SPEC-NNNN/T-NNN` directly against a
`state="pending"` task; the CLI surfaces a recommendation, not a gate.
Workspace-form ordering is lowest spec ID first. The workspace form
exits with code 2 and `reason="no_active_specs"` when no active spec
remains.

### `speccy check` rendering

Serial. For each selected scenario, the command prints
`==> CHK-NNN (SPEC-NNNN): <scenario first line>` followed by indented
continuation lines, then closes with `N scenarios rendered across M
specs`. The working directory is the project root (the directory
containing `.speccy/`). No subprocesses are spawned; exit code is
non-zero only for selector, lookup, parse, or workspace errors, never
because the project's own tests would fail. Project tests run through the
project's own test runner (e.g. `cargo test`, `pnpm test`); CI
orchestrates both that runner and `speccy verify` side by side.

### Reviewer and vet diff scoping

The diff a reviewer sub-agent reasons over is fetched by the sub-agent
itself, never inlined into its spawn prompt. The command comes from the
`diff_command` field of the task-scoped `speccy context` bundle the
persona opens with: a merge-base diff against the repository's default branch
(`git diff <base>...HEAD`, where `<base>` is e.g. `origin/main`),
optionally scoped with `-- <suggested-files>` from the task body. The
CLI derives the suggestion via read-only git probes and degrades to a
`main`-baseline string when the probe fails (no remote, detached HEAD,
git unavailable); it never fetches the diff itself and never mutates the
repository.

The spec-scoped bundle used by vet also carries `diff_command`, but its
command is `git diff <base>` rather than `...HEAD` so it includes
uncommitted holistic fixes between vet rounds.
