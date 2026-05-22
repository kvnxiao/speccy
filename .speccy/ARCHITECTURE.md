# Speccy Architecture

> Canonical architecture for Speccy.
>
> Speccy is an AI-first, lightweight feedback engine for spec-driven
> development. It does not try to enforce determinism on LLMs. It makes
> intent, proof, and drift mechanically visible so agents and humans
> can catch divergence before it ships.

---

# Mission

Speccy is a deterministic CLI that lets humans and AI agents collaborate
on software with bounded drift. It exists because LLM nondeterminism
compounds: small misreadings of intent accumulate across features until
what shipped no longer matches what was asked for.

Speccy does not try to make LLMs deterministic. It makes the **contract**
between user intent and shipped behavior visible, so drift is loud the
moment it happens.

Speccy is built for a post AI-assisted engineering world where agents
draft specs, decompose tasks, implement, review adversarially, and
report. The human role is to:

- state intent
- answer material product questions
- approve or reject major tradeoffs
- perform final acceptance

The CLI is intentionally thin. The intelligence lives at the edges:
in skills, prompts, personas, and reviewers. The Rust CLI does not
call LLMs in v1.

---

# Stance: Feedback, Not Enforcement

LLMs do not reliably follow instructions. Treating Speccy as an
enforcement system would be a category error: every gate we add is
just another instruction an LLM can fail to obey, and every failure
mode of enforcement (false positives, blocked-but-actually-fine,
agent works around the gate) is worse than visibility.

So Speccy is a **feedback engine**:

- The CLI tells you what looks off; you decide.
- `speccy verify` is the only command that exits non-zero on
  problems, and it only exits non-zero on broken **proof shape**
  (parse errors, requirements with no scenarios, scenario refs that
  don't resolve, internal inconsistency). CI calls it. Local runs
  print findings and exit zero.
- **Speccy does not run project tests.** Project CI owns test
  execution: `cargo test`, `pnpm test`, lint, type-check, and
  `cargo deny check` run alongside `speccy verify`, not through it.
- **Reviewer personas own semantic judgment.** Whether a scenario
  is meaningful, whether the diff actually satisfies it, and
  whether the project tests cover the scenario meaningfully are
  questions for the business / tests / security / style reviewer
  loop, not for the CLI.
- There is no `--strict` mode, no policy file, no configurable
  enforcement. Speccy is opinionated about what to surface and
  silent about what to do about it.
- Skills wrap this feedback into agent workflows. The skill layer
  is where the loops live, where personas are defined, and where
  intelligence about "what to do next" gets exercised.

If Speccy ever feels like it's getting in the way, that's a bug in
Speccy, not in the user's workflow.

---

# Five Proper Nouns

| Noun | What it is | Where it lives |
|---|---|---|
| **Mission** | Scope of one long-running initiative composed of 1+ specs | `specs/[focus]/MISSION.md` (optional grouping; omit for flat single-focus projects) |
| **Spec** | One bounded behavior contract | `specs/[focus]/NNNN-slug/SPEC.md`, or `specs/NNNN-slug/SPEC.md` when ungrouped |
| **Requirement** | One observable behavior with a done condition | `<requirement>` element block in SPEC.md |
| **Task** | One implementation slice sized for one agent | Line in `TASKS.md` |
| **Check** | One English validation scenario a requirement must satisfy | `<scenario>` element block nested under a `<requirement>` in SPEC.md |

The project-wide product north star ("what we're building, why, who
for, what 'good enough to ship v1' looks like") is **not** a Speccy
noun. It lives as a section inside `AGENTS.md` at the repo root.
AGENTS.md is loaded into every rendered prompt, so the north star
is always in context for any planner, implementer, or reviewer agent.

A **Mission** is a narrower thing: the scope of one long-running
initiative composed of multiple related specs. Mission folders are
optional. A project with one focus area may have zero MISSION.md
files; specs live flat under `.speccy/specs/`. When
a focus accumulates 2+ specs that share enough context that loading
them together at plan time is cheaper than rediscovering it, the
planner skill creates `specs/[focus]/MISSION.md` and writes new
specs into the focus folder.

That is the complete conceptual vocabulary. Capability, milestone,
release, decision, amendment, assumption, constraint, invariant,
question, scenario, claim, lease, validation, evidence, finding,
review, and amendment are all either cut, derived from artifact
state, or rendered as freeform markdown sections inside SPEC.md /
TASKS.md / MISSION.md / AGENTS.md.

---

# Core Development Loop

The full loop is five phases. Phases 3 and 4 are single-task
primitives: one invocation, one task, one state transition recorded
in TASKS.md. Composing those invocations into a batch is a caller
concern, not the skill's.

```
1. plan       skill writes SPEC.md (PRD-shaped, XML-element-structured)
2. tasks      skill writes TASKS.md (one task sized for one agent session); skill calls `speccy lock`
3. implement  skill implements one task; exits with state transition
4. review     skill fans out four personas on one task; exits with state transition
5. report     skill writes REPORT.md and opens PR
```

Phase verbs are skill responsibilities, not CLI verbs. The CLI
never renders natural-text prompts. Its job is deterministic state
work: scaffolding (`init`), state queries (`status`, `next`,
`vacancy`), hash recording (`lock`), scenario rendering (`check`),
and proof-shape gating (`verify`). Skills discover paths and the
derived `next_action` through the CLI's `schema_version: 1` JSON
envelopes; the CLI is the sole authority on the `NNNN-slug`
directory rule. Multi-task composition lives in the caller (a human
at the terminal, the existing `/loop` skill, or a future
orchestrator).

---

# CLI Surface

Seven flat commands. Each has one job. `--json` toggles
representation, never content; there are no other mode flags, no
state-transition verbs, no per-phase rendering verbs.

```text
speccy init                       Scaffold .speccy/ + host skill pack.
                                    --host claude-code | codex (auto-detected if omitted)
                                    --force            overwrite differing shipped files
speccy status [SELECTOR]          Workspace overview; spec subset by default.
                                    no arg:        attention-list view
                                    SPEC-NNNN:     one spec, unfiltered
                                    --all:         every spec, unfiltered
                                    --json:        schema_version=1 envelope with resolved paths
speccy next [SPEC-ID]             Next actionable per spec, derived from state.
                                    no arg:        every active spec with next_action
                                    SPEC-ID:       one spec or {next_action: null, reason}
                                    --json:        schema_version=1 envelope
                                  Action kind is derived (review > implement > ship,
                                  with `decompose` when TASKS.md is absent); spec
                                  state fully determines the kind, so there is no
                                  caller-supplied `--kind` filter.
speccy check [SELECTOR]           Render check scenarios (no execution).
                                    no arg:            every scenario across every spec
                                    SPEC-NNNN:         every scenario under one spec
                                    SPEC-NNNN/CHK-NNN: one scenario, spec-qualified
                                    SPEC-NNNN/T-NNN:   scenarios covering a qualified task
                                    CHK-NNN:           every spec's CHK-NNN
                                    T-NNN:             scenarios covering an unqualified task
speccy verify                     CI gate: proof-shape validation only.
                                    --json:        schema_version=1 envelope
                                    parse errors, requirements with no scenarios,
                                    unresolved scenario refs, stale task hash, etc.
                                    Does NOT run project tests; that's CI's job.
speccy lock SPEC-NNNN             Record SPEC.md sha256 + UTC timestamp into TASKS.md
                                  frontmatter (`spec_hash_at_generation`,
                                  `generated_at`). Used by `/speccy-tasks` after
                                  decomposition; replaces the old
                                  `speccy tasks --commit` sub-action.
speccy vacancy                    Return the next free `SPEC-NNNN`.
                                    no arg:   bare `SPEC-NNNN\n` to stdout
                                    --json:   {schema_version: 1, next_spec_id: "SPEC-NNNN"}
                                  Used by `/speccy-plan` so the skill never
                                  globs `.speccy/specs/` itself.
```

Phase prose lives in skill content under `.claude/skills/...` and
`.agents/skills/...`, not in the CLI. There is no `speccy plan` /
`speccy tasks` (render-form) / `speccy implement` / `speccy review`
/ `speccy report` verb; conflating "what loop am I in" with "which
sub-type of reviewer" through a `--persona` flag on a generic
`prompt` command would be the wrong abstraction. Persona selection
lives in the `/speccy-review` skill, which dispatches to the
matching `reviewer-<persona>` sub-agent file.

The CLI is the sole authority on the spec directory rule
(`NNNN-slug` flat or one level inside a mission folder). Skills
read paths from `speccy status --json` / `speccy next --json` /
`speccy vacancy --json` rather than globbing the filesystem; the
JSON envelopes carry `spec_md_path`, `tasks_md_path`, and
`mission_md_path` (nullable when absent).

That is the complete public surface. Anything else is a skill
responsibility.

---

# File Layout

```text
AGENTS.md                Project-wide product north star + agent conventions
                         (root, not inside .speccy/)

.speccy/
  speccy.toml
  specs/
    0001-user-signup/                Ungrouped spec (no mission folder)
      SPEC.md            Frontmatter + PRD prose + nested XML element tree
                         (<requirement>/<scenario>/<decision>/<open-question>
                         /<changelog>); the requirement-to-scenario graph is
                         carried in-band by these elements
      TASKS.md           Frontmatter (spec_hash_at_generation, generated_at)
                         + bare <task>/<task-scenarios> XML tree (no
                         <tasks> wrapper; no implementer / review prose)
      REPORT.md          Frontmatter (outcome) + <report>/<coverage> XML tree
                         (end of loop)
      journal/           Per-task activity journal (see "TASKS.md
        T-001.md         per-task journal" below). One T-NNN.md per task
        T-002.md         that has been claimed by an implementer; each
        T-003.md         carries <implementer>/<review>/<blockers> blocks.
    auth/                            Mission folder (optional grouping)
      MISSION.md                     Scope/context for this focus area
      0002-signup/
        SPEC.md
        TASKS.md
        REPORT.md
      0003-password-reset/
        SPEC.md
        ...

resources/               Shipped with Speccy; rendered/copied by `speccy init`
  modules/               Single-source bodies (no host duplication)
    personas/            Six reviewer persona body files + co-located
                         topic-named snippets (verdict_return_contract.md,
                         inline_note_format.md, no_tasks_md_writes.md,
                         diff_fetch_command.md)
    phases/              Three pinned phase-worker bodies (speccy-tasks.md,
                         speccy-work.md, speccy-ship.md); plus speccy-init.md
                         which is dual-consumed by `speccy init` itself
    skills/              Five interactive skill bodies (speccy-brainstorm.md,
                         speccy-plan.md, speccy-amend.md, speccy-review.md
                         orchestrator) — full-body SKILL.md eject targets
    examples/            Progressive-disclosure example bodies (evidence.md)
  agents/                Per-host wrappers (MiniJinja-templated)
    .claude/             Renders to <project>/.claude/{skills,agents}/
    .agents/             Renders to <project>/.agents/skills/ (Codex)
    .codex/              Renders to <project>/.codex/agents/ (Codex)
```

There is no `resources/modules/prompts/` directory and no
CLI-embedded phase prompt body. Phase prose ships as host skill
content; the CLI does not render natural text. Reviewer persona
content lives at the host-native sub-agent files
(`.claude/agents/reviewer-<persona>.md` and the Codex twin) and
there is no project-local `.speccy/skills/personas/` override.

`AGENTS.md` lives at project root, not inside `.speccy/`. Every
project already keeps `AGENTS.md` (and often `CLAUDE.md` as a symlink)
at the root for the broader agent ecosystem; speccy reads the file
in place rather than asking projects to duplicate it under `.speccy/`.
AGENTS.md carries both the product north star (what we're building,
who for, v1 outcome, quality bar) and the cross-cutting agent
conventions (hygiene, rule files, behavioral expectations). Section
the file explicitly so reviewer-business and reviewer-architecture
personas can find the product context, while reviewer-style finds
the conventions.

Mission folders are optional. A flat project with one focus area
may have zero MISSION.md files — specs live directly under
`.speccy/specs/NNNN-slug/`. When grouping emerges, the planner
skill creates `.speccy/specs/[focus]/MISSION.md` and writes new
specs into the focus folder. Existing flat specs may be moved into
a focus folder retroactively; spec IDs do not change.

`resources/` is the top-level directory in the Speccy workspace that
holds shipped bodies. `resources/modules/{personas,phases,skills,examples}/`
are the single source of truth, and `resources/agents/` carries the
per-host wrappers as MiniJinja templates. `speccy init` renders those
wrappers into the user's project at the host-native location.

For Claude Code that lands as `.claude/skills/speccy-<verb>/SKILL.md`
for every shipped skill (full body for the five interactive skills;
thin ≤10-line stub for the three pinned phase workers), plus
`.claude/agents/speccy-{tasks,work,ship}.md` for the pinned phase
worker bodies and `.claude/agents/reviewer-<persona>.md` for the six
reviewer sub-agents. Neither `speccy-init` nor `speccy-review` ships
an agent file: both stay in the parent session because both are
interactive (init Q&A; review verdict consolidation) and need the
parent session's full capacity for user prompts and serialised
TASKS.md writes.

For Codex the parallel is `.agents/skills/speccy-<verb>/SKILL.md`
plus `.codex/agents/speccy-{tasks,work,ship}.toml` and
`.codex/agents/reviewer-<persona>.toml`.

There is no project-local persona override directory. The
host-native sub-agent files under `.claude/agents/` and
`.codex/agents/` are the sole canonical persona surface, and
`speccy init` classifies them Skip-on-exists so local edits to a
persona's body survive `speccy init --force`.

Decisions (ADRs) are not a separate folder. Each spec's `## Design
> Decisions` subsection holds the architectural choices made for
that spec. Project-wide conventions that span specs belong in
`AGENTS.md`. Cross-spec context bounded to one focus area belongs
in that focus area's `MISSION.md`.

---

# Workflow Phases

Each phase below is a skill responsibility, not a CLI invocation.
The CLI surface that backs each phase is named where it matters
(state queries, hash recording, scenario rendering), but the prose
body the agent reads lives in a skill file under
`.claude/skills/...` (or the Codex twin), ejected at `speccy init`
time. The CLI has no `plan` / `tasks` / `implement` / `review` /
`report` verbs; the phase recipes are `/speccy-plan`,
`/speccy-tasks`, `/speccy-work`, `/speccy-review`, and
`/speccy-ship` respectively.

## Phase 1: Planning

The `/speccy-plan` skill (interactive, full-body SKILL.md) drives
the planning phase. The skill body instructs the agent to:

- read `AGENTS.md` (carries the project-wide product north star);
- read the nearest parent `MISSION.md` if writing into an existing
  focus area (the skill walks upward from the target spec path;
  absent MISSION.md is fine, the spec is ungrouped);
- call `speccy vacancy --json` to learn the next free `SPEC-NNNN`
  without globbing `.speccy/specs/` itself;
- propose the next SPEC slice;
- write `specs/[focus]/NNNN-slug/SPEC.md` when targeting a focus
  area, otherwise `specs/NNNN-slug/SPEC.md` (PRD-shaped; see the
  template under "SPEC.md" below), including `<requirement>` and
  nested `<scenario>` element blocks for IDs and check scenarios;
- surface material questions inline in SPEC.md.

Mid-loop amendments use the parallel `/speccy-amend` skill, which
walks the SPEC.md edits and reconciles TASKS.md against the new
shape, then calls `speccy lock SPEC-NNNN` to re-record the hash.

## Phase 2: Task decomposition

The `/speccy-tasks` skill (pinned phase worker; thin SKILL.md stub
plus full body at `.claude/agents/speccy-tasks.md`) drives task
decomposition. The agent body instructs the agent to:

- read the SPEC.md (path supplied via `speccy next --json` or the
  user-supplied SPEC-NNNN argument);
- decompose into ordered tasks small enough for one sub-agent;
- group by phase if useful;
- reference REQ IDs each task covers via the `<task covers="...">`
  attribute;
- write `specs/NNNN-slug/TASKS.md`.

After the agent writes TASKS.md, the skill calls:

```sh
speccy lock SPEC-001
```

`speccy lock` records the current SPEC.md sha256 + UTC timestamp
into TASKS.md frontmatter (`spec_hash_at_generation`,
`generated_at`). Used for staleness detection in later phases. It
is the only verb that mutates a TASKS.md frontmatter field; the
skill calls it once after decomposition lands.

If TASKS.md already exists, decomposition runs as an amendment
under `/speccy-amend`, which preserves completed tasks, modifies
or removes invalidated tasks, and adds new ones for new
requirements.

## Phase 3: Implementation (single-task primitive)

The `/speccy-work` skill is a single-task primitive. One invocation
implements one task and exits with one state transition recorded in
TASKS.md. The skill ships as a thin SKILL.md stub plus a full agent
body at `.claude/agents/speccy-work.md` (pinned `model: sonnet[1m]`,
`effort: medium`) and the Codex twin at
`.codex/agents/speccy-work.toml`.

With an optional `[SPEC-NNNN/T-NNN]` selector the session implements
that specific task. Without an argument the session calls
`speccy next --json` and filters for `next_action.kind == "implement"`
to resolve the next implementable task. In either case the session:

- flips `state="pending"` to `state="in-progress"` on the target
  task;
- writes tests first, then code; runs the project's own test
  command locally and fails fast on red;
- uses `speccy check SPEC-NNNN/T-NNN` only to render the scenarios
  it is satisfying;
- appends one `<implementer>` block to
  `.speccy/specs/NNNN-slug/journal/T-NNN.md` using the six-field
  handoff template the agent body supplies (the journal file is
  created on round 1 if it does not exist; subsequent rounds
  append);
- flips `state="in-progress"` to `state="in-review"` and exits.

The session does not pick up another task on its way out. If two
implementers run in parallel against different `state="pending"`
tasks and touch the same files, they conflict in git; Speccy does
not lock.

Composing multiple Phase 3 invocations into a batch is a future
Layer-2 concern not built today. The interim composer is the
existing `/loop` skill, which iterates the primitive on its caller's
behalf.

## Phase 4: Review (single-task primitive)

The `/speccy-review` skill is a single-task primitive. One
invocation runs one round of adversarial review on one task and
exits with one state transition recorded in TASKS.md. The
orchestrator stays in the parent session (no agent file) because
it is the sole writer to TASKS.md during the review loop and needs
the parent session's full capacity to fan out, parse return
messages, and consolidate verdicts atomically.

With an optional `[SPEC-NNNN/T-NNN]` selector the session reviews
that specific task. Without an argument the session calls
`speccy next --json` and filters for `next_action.kind == "review"`.
In either case the session:

- fans out four reviewer sub-agents in parallel within this single
  task, one per persona in the default fan-out (`business`, `tests`,
  `security`, `style`); each sub-agent's body is loaded from
  `.claude/agents/reviewer-<persona>.md` or its Codex parallel, with
  per-persona model pins (Opus[1m] xhigh for business / tests /
  architecture; Sonnet[1m] high for security; Sonnet[1m] medium for
  style / docs);
- aggregates the four returned `<review>` blocks and appends each
  to `.speccy/specs/NNNN-slug/journal/T-NNN.md` (the orchestrator
  is the sole writer to the journal during review — reviewer
  sub-agents return their `<review>` element to the orchestrator
  which writes serially, preserving the existing concurrency
  contract);
- flips `state="in-review"` to `state="completed"` if every persona
  `<review>` carries `verdict="pass"`; otherwise flips
  `state="in-review"` to `state="pending"` and appends a
  `<blockers>` block to the journal summarising the blockers, and
  exits.

The within-task four-persona fan-out is intrinsic to the primitive,
not orchestration: adversarial diversity requires fresh contexts per
persona, and the fan-out is bounded to four sub-agents on one task
in one round. Failed tasks return to `state="pending"` for a later
Phase 3 invocation to pick up.

The default fan-out is **business**, **tests**, **security**,
**style**. The other personas (**architecture**, **docs**) are
available by user request but not in the default set.

Composing multiple Phase 4 invocations into a batch is a future
Layer-2 concern not built today. The interim composer is the
existing `/loop` skill, which iterates the primitive on its caller's
behalf.

## Phase 5: Report and PR

When `speccy next` returns no actionable task across the workspace,
the loop is complete. The `/speccy-ship` skill (pinned phase worker
at `.claude/agents/speccy-ship.md`) instructs the agent to write
`REPORT.md` summarising:

- requirements satisfied;
- tasks completed (with retry counts derived from journal rounds);
- out-of-scope items absorbed;
- deferred or known limitations;
- check results summary.

REPORT.md is shaped by raw XML element tags (`<report spec="...">`
wrapping one `<coverage req="...">` per surviving requirement); the
"REPORT.md format" section below covers the grammar and the
`RPT-*` lint family that gates the proof shape. The skill then opens
a PR via `gh` (or equivalent); Speccy itself never touches GitHub.

---

# TASKS.md State Model

Four task states, carried by the `state` attribute on each `<task>`
XML element (see "TASKS.md format" below for the full grammar).

| `state` value | Meaning | Who sets it |
|---|---|---|
| `pending` | Needs work (new or retry) | Initial generation; reviewer on blocking |
| `in-progress` | Claimed by an implementer | Implementer when starting |
| `in-review` | Implementation done, awaiting review | Implementer when finishing |
| `completed` | All persona reviews passed | Reviewer skill at exit of review primitive |

A retry is just `state="pending"` with prior activity entries
attached in the per-task journal. We do not introduce a fifth state
because the journal entries already say "this is a retry; see
review findings." Adding a state would add cases for skills to
handle without adding information.

## TASKS.md per-task journal

Implementer handoff prose, reviewer verdicts, and amendment-driven
blocker directives **do not live inside the `<task>` element body
in TASKS.md**. They live in a sibling `journal/T-NNN.md` file
under the same spec directory:

```text
.speccy/specs/0001-user-signup/
  SPEC.md
  TASKS.md
  REPORT.md
  journal/
    T-001.md
    T-002.md
    T-003.md
```

The journal directory sits alongside `SPEC.md`, `TASKS.md`, and
`REPORT.md`. A journal file is created on the first `<implementer>`
write (round 1 of an implementer attempt) and accumulates one
`<implementer>` block per round plus N `<review>` blocks per round
of fan-out plus at most one `<blockers>` block per round (when a
reviewer blocks or an amendment flips the task back to `pending`).

Each `journal/T-NNN.md` file has YAML frontmatter binding it to its
task plus a chronological body of bare `<implementer>`, `<review>`,
and `<blockers>` element blocks (no wrapper element):

```markdown
---
spec: SPEC-0001
task: T-002
generated_at: 2026-05-11T18:00:00Z
---

<implementer date="2026-05-11T18:00:00Z" model="claude-opus-4.7[1m]/low" round="1">
Renamed existing `password` column. Added migration to hash
plaintext rows. **Out of scope**: touched
`tests/migration_helpers.ts` to fix a test helper assuming
plaintext.
</implementer>

<review persona="business" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-opus-4.7[1m]/high" round="1">
Matches REQ-002 intent.
</review>

<review persona="tests" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-opus-4.7[1m]/medium" round="1">
Hash assertion present.
</review>

<review persona="security" verdict="blocking" date="2026-05-11T19:00:00Z" model="claude-sonnet-4-6[1m]/medium" round="1">
bcrypt cost 10; policy requires >=12. See `src/auth/password.ts:14`.
</review>

<review persona="style" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-sonnet-4-6[1m]/medium" round="1">
Conventions OK.
</review>

<blockers date="2026-05-11T19:00:00Z" round="2">
Address bcrypt cost.
</blockers>
```

### Journal binding rules

Two bindings tie a journal file to its task and spec:

- **Filename ↔ task.** `journal/T-NNN.md` carries activity for the
  `<task id="T-NNN">` in the sibling TASKS.md. The frontmatter's
  `task:` field must agree with the filename digits; mismatches
  fire `JNL-003`.
- **Frontmatter ↔ spec.** The frontmatter's `spec:` field must
  agree with the parent directory's spec id and the sibling
  TASKS.md frontmatter's `spec:` field; mismatches fire `JNL-003`.

The frontmatter requires exactly three fields: `spec` (matching
`SPEC-\d{3,}`), `task` (matching `T-\d{3,}`), and `generated_at`
(ISO8601 timestamp with seconds and timezone designator).

### Journal element grammar

| Element | Cardinality | Parent | Required attributes | Notes |
|---|---|---|---|---|
| `implementer` | 1+ per round, ≥1 round total | bare under frontmatter | `date`, `model`, `round` | Implementer handoff for one round. Body is Markdown using the six-field handoff template (Completed / Undone / Commands run / Exit codes / Discovered issues / Procedural compliance). |
| `review` | 1+ per reviewed round | bare under frontmatter | `date`, `model`, `persona`, `verdict`, `round` | One reviewer's verdict for one round. `verdict` is `pass` or `blocking`; `persona` is one of the persona registry values. |
| `blockers` | 0 or 1 per round | bare under frontmatter | `date`, `round` | Directive carried across a retry boundary — either reviewer-aggregated blockers or an amendment-driven blocker. Body names what the next round must address. |

All attributes listed are required; there are no optional
attributes in the journal schema. Attribute value rules:

- `date` — full ISO8601 with seconds and timezone designator
  (regex `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[+-]\d{2}:\d{2})$`).
  `generated_at` in frontmatter uses the same format.
- `model` — non-empty string. The agreed skill-layer convention
  encodes effort via a slash suffix (e.g.
  `claude-opus-4.7[1m]/low`, `claude-sonnet-4-6[1m]/medium`); the
  parser does NOT validate slash-suffix internal structure — it
  only enforces non-empty.
- `round` — positive integer (regex `^[1-9][0-9]*$`).
- `verdict` — closed value set `{pass, blocking}`.
- `persona` — closed persona registry (`business`, `tests`,
  `security`, `style`, `architecture`, `docs`).

### Round monotonicity

The journal parser validates round sequence within a single file:

- The first `<implementer>` block must have `round="1"`.
- The `round` counter is monotonic non-decreasing across blocks.
- Counter must not skip values (no jumping from N to N+2 without an
  intervening N+1 block).
- Multiple blocks at the same round are allowed (one
  `<implementer>` plus N `<review>` plus at most one `<blockers>`
  per round).

Shape violations under either binding or monotonicity surface as
`JNL-003`.

### Lint family for journal artifacts (JNL-*)

Three new lint codes (registered in the canonical registry alongside
`SPC-*`, `TSK-*`, `RPT-*`) enforce the journal contract. All three
default to `Level::Error` and gate `speccy verify`:

- **JNL-001** — Task is `state="pending"` but `journal/T-NNN.md`
  exists. A pending task has not yet been claimed by an
  implementer; a journal file at that ID is unexpected (likely a
  leftover from a prior loop, or a state-flip that did not also
  clean up the journal).
- **JNL-002** — Task is `state="completed"` but `journal/T-NNN.md`
  is missing. Every completed task must carry its journal as the
  durable record of how it was implemented and reviewed.
- **JNL-003** — Task is `state="completed"` but the journal file
  has shape or binding violations (filename ↔ frontmatter mismatch,
  spec ↔ parent-dir mismatch, missing frontmatter field,
  attribute-schema violation, or round-monotonicity violation).

Tasks at `state="in-progress"` or `state="in-review"` are silently
skipped by all three JNL codes — the family never runs mid-loop, so
a half-written journal in flight is not a lint error. The
activation gate lives in the lint runner; each rule does its own
work assuming activation is granted.

### TSK-006: no journal elements inside TASKS.md

`<implementer>`, `<review>`, and `<blockers>` elements are not in
the allow-list for TASKS.md bodies. If any of them appears inside a
`<task>` element in TASKS.md, the parser still records the
location, and `TSK-006` fires at `Level::Error` regardless of task
state. The diagnostic names which element appeared, the containing
task id, and the canonical fix (move the block to
`journal/T-NNN.md`).

`TSK-006` is not state-gated — the misplaced element fires
identically against `pending`, `in-progress`, `in-review`, and
`completed` tasks. It fires before any `JNL-*` diagnostic on the
same task, because a misplaced element in TASKS.md is more
fundamental than a journal-shape issue.

### Lifecycle reading

An implementer picking up a task reads TASKS.md to find the next
`state="pending"` task, then reads `journal/T-NNN.md` (if it
exists) to learn what prior rounds did, what reviewers blocked, and
what an amendment-driven `<blockers>` directive (if any) asks the
next round to address. The implementer then flips `state` back to
`in-progress`, appends a new `<implementer>` block with the next
`round` value, does the work, flips `state` to `in-review`, and
exits.

## Concurrent pickup

`state="in-progress"` with a session marker is enough for
`speccy next --kind implement` to skip in-progress tasks. If two
agents race to claim the same `state="pending"` task, git will
conflict on the TASKS.md edit and one will lose. That is acceptable
for v1.

A future harness may add file-locking, ticket queues, or worktree
isolation. Speccy v1 does not.

---

# Artifacts

## MISSION.md

Optional parent-context artifact for a focus area. Not required: a
flat single-focus project may have zero MISSION.md files. When
present, it lives at `.speccy/specs/[focus]/MISSION.md` and the
planner / implementer / reviewer skills walk upward from any spec
path looking for the nearest MISSION.md and include it in rendered
prompts.

The project-wide product north star (what we're building, who for,
v1 outcome, quality bar) does **not** live here — it lives in
`AGENTS.md` at the repo root. MISSION.md is narrower: the scope of
one focus area within the broader product.

Recommended sections:

```markdown
# Mission: <focus name>

## Scope
What this focus area covers. What it doesn't.

## Why now
The motivation driving this initiative, and any deadline / sequencing
constraints.

## Specs in scope
- SPEC-NNN — short title
- SPEC-NNN — short title

## Cross-spec invariants
Things every spec in this mission must respect (auth model, data
ownership, error semantics, etc.).

## Open questions
Things we expect to learn as specs land.
```

MISSION.md is markdown; Speccy does not parse its structure beyond
detecting its presence to scope prompts. No `MIS-NNN` lint codes
exist in v1. No `speccy mission` command exists. Mission is a
filesystem-and-skill convention, not a CLI-aware noun. (This is a
deliberate v1 simplification; promote to a parsed noun later only
if dogfooding shows pain.)

### AGENTS.md bootstrap

When `AGENTS.md` is missing or lacks a product north star section,
the **`speccy-init` skill** (not the CLI) runs an interactive Q&A to
populate it. The skill detects three states:

1. AGENTS.md missing entirely → bootstrap from scratch via full Q&A
   (product, users, v1 outcome, constraints, non-goals, quality bar,
   known unknowns).
2. AGENTS.md exists with process conventions but no `## Product
   north star` section (or equivalent) → narrower Q&A; append the
   section.
3. AGENTS.md already has a north star → leave alone; confirm with
   the user.

The skill never overwrites: always append, or stop. The CLI's
`speccy init` only scaffolds `.speccy/` and copies the host skill
pack; it never edits `AGENTS.md`.

## speccy.toml

```toml
schema_version = 1

[project]
name = "taskify"
```

That is the complete configuration. There is no `[policy]` block,
no `[env]` block, no review identity setting, no `[[global_checks]]`
array, no `root` field — the project root is the directory containing
`.speccy/` (found by `find_root` walking up). Project-level conventions
and toolchain context belong in `AGENTS.md`, which every skill prompt
already loads. If the CLI ever needs structured access to environment
metadata, the block will come back with a real purpose; until then,
it isn't here.

## SPEC.md (PRD-shaped template)

```markdown
---
id: SPEC-001
slug: user-signup
title: User signup
status: in-progress
created: 2026-05-11
supersedes: []
---

# SPEC-001: User signup

## Summary
2-4 paragraphs. What this spec covers, why it matters, how it fits
into the broader product.

## Goals
- Concrete outcomes this spec must achieve.

## Non-goals
- Explicitly out of scope. Things readers might assume but shouldn't.

## User stories
- As a new visitor, I want to create an account with email/password
  so that I can save my work between sessions.
- As a returning user, I want a clear error when I try to sign up
  with an email that already exists.

## Requirements

### REQ-001: Account creation
Users can create an account with email and password.

**Done when:** A valid signup request persists a user record and
returns a session token; duplicate email returns 409 with an
actionable message.

**Behavior:**
- Given no account exists for `alice@example.com`, when a signup
  request submits valid credentials, then a user record is
  persisted and the response includes a session token.
- Given an account already exists for `alice@example.com`, when a
  signup request submits the same email, then the response is 409
  with an error message containing "already exists".
- Given a signup request submits an invalid email format, when
  processed, then the response is 400 with a validation error.

**Covered by:** CHK-001, CHK-002, CHK-003

### REQ-002: Password storage
Passwords are hashed before persistence; plaintext never touches
storage.

**Done when:** Inspection of the users table shows hashed values;
a direct DB query for the password column never returns plaintext.

**Behavior:**
- Given a signup request with password `correct horse battery
  staple`, when the user record is persisted, then the password
  column contains a hash and never the original string.
- Given the users table is dumped to logs, when inspected, then
  no plaintext passwords appear.

**Covered by:** CHK-004

## Design

### Approach
[1-2 paragraphs of technical approach.]

### Decisions

#### DEC-001: Password hashing algorithm
**Status:** Accepted
**Context:** Signup requires password auth without hosted services.
**Decision:** bcrypt with cost factor 12.
**Alternatives:** Hosted auth (deferred, requires email
infrastructure); argon2 (deferred, no clear need yet); plaintext
with separate KMS (rejected: KMS not yet provisioned).
**Consequences:** App owns credential storage risk. Security
review must inspect password handling on every auth-touching
change. Cost factor revisits required when hardware baselines
shift.

#### DEC-002: Session storage
**Status:** Accepted
**Context:** Signup must return something a returning user can
present to authenticate later requests.
**Decision:** JWT signed with project secret, 24h expiry, stored
in httpOnly Secure cookie.
**Alternatives:** Server-side sessions in Redis (rejected: adds
infrastructure dependency); long-lived API tokens (rejected:
revocation story is poor).
**Consequences:** Stateless auth; horizontal scaling is trivial.
Token revocation requires key rotation or a blocklist (deferred).

### Interfaces
- `POST /api/signup` -- accepts `{email, password}`, returns
  `{token}` or `{error}`.
- `users` table -- new columns: `email` (unique index),
  `password_hash`.

### Data changes
- Migration: `users` table with unique email index.

### Migration / rollback
- Forward: standard migration.
- Rollback: drop columns; no data loss because feature is new.

## Open questions
- [ ] Should signup auto-login the user, or just create the account?
- [x] Email case-sensitivity? -- Normalize to lowercase on write.

## Assumptions
- Email uniqueness enforced at the DB layer via index.

## Changelog

| Date       | Author          | Summary |
|------------|-----------------|---------|
| 2026-05-11 | agent/claude-1  | Initial draft from AGENTS.md north star |

## Notes
Free-form context for future agents and reviewers.
```

### Frontmatter

The YAML frontmatter is the single source of truth for spec lifecycle:

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Stable spec ID (`SPEC-NNN`). |
| `slug` | yes | Folder-name slug. |
| `title` | yes | Human-readable title. |
| `status` | yes | One of: `in-progress`, `implemented`, `dropped`, `superseded`. |
| `created` | yes | ISO date when the spec was first drafted. |
| `supersedes` | no | List of prior spec IDs this one replaces. Omit or `[]` if none. |

Supersession is stored on the **new** spec (the one doing the
replacing) via `supersedes`. The inverse direction is **computed**
by walking the supersedes graph across all specs in the workspace;
no `superseded_by` field is stored. This keeps lineage
single-sourced; the older spec does not need to be updated when a
new spec replaces it.

`status` transitions:

```text
in-progress -> implemented      All tasks state="completed", REPORT.md written, PR merged.
in-progress -> dropped          Intent abandoned. Add a Changelog row stating why.
implemented -> superseded       A later spec declared `supersedes` pointing here.
in-progress -> superseded       Rare; replaced before completion.
```

Skills (specifically `/speccy:ship` and `/speccy:amend`) update `status`.
The CLI doesn't auto-transition state — it surfaces inconsistencies via lint
(e.g. `status: implemented` but some tasks have `state != "completed"`).

### Changelog table

The `## Changelog` table is the in-doc lineage. Every material change to
SPEC.md after initial draft adds a row:

| Date | Author | Summary |
|------|--------|---------|
| 2026-05-11 | agent/claude-1 | Initial draft |
| 2026-05-13 | agent/claude-1 | REQ-002 bcrypt cost bumped to 12 per security review F-001 |
| 2026-05-14 | human/kevin | Dropped REQ-003 (magic-link auth) — out of v1 scope |

The Changelog replaces both the cut delta markers and the cut amendment
artifact. It is git-history-redundant by design — git tells you *what*
changed; the Changelog summarizes *why* and is loaded into every prompt
that reads SPEC.md.

Reviewer personas read the Changelog to understand recent intent
shifts. The skill prompt for `/speccy:amend` instructs the agent to
append a Changelog row whenever it edits SPEC.md.

### Lint behavior

Speccy lints three things in SPEC.md:

1. Required frontmatter fields are present.
2. The element tree is well-formed: every `<requirement>` has at
   least one nested `<scenario>`, every id matches its regex,
   and no ids duplicate within a spec.
3. Any unchecked `- [ ]` in the **Open questions** section is reported
   in `speccy status` as a soft signal.

Nothing else in SPEC.md is parsed or enforced. The template is a
convention; the agent's skill prompts nudge the shape.

### Tests in English first (TDD convention)

The `**Behavior:**` block under each requirement is the **higher-level
test specification** in prose. Each bullet is one Given/When/Then
scenario that maps to one or more Checks. These describe integration
or end-to-end behavior at the requirement level.

Unit-level tests live in TASKS.md (see below) as `<task-scenarios>`
element blocks nested inside each `<task>`. This split is
intentional:

- **SPEC.md behavior**: what the system does, observable from outside.
  Maps to `<scenario>` element blocks nested under each
  requirement; the project's integration tests must satisfy them.
- **TASKS.md `<task-scenarios>`**: what each implementation slice
  must verify. Maps to unit tests the implementer writes before code.

Agents writing implementation code translate these prose tests into
executable tests in the project's framework, then implement to make
them pass. Speccy does not run those tests and does not enforce TDD
ordering (red-before-green); it makes the test obligations visible
and the reviewer-tests persona checks that they're meaningful.

### Brownfield posture

There is no greenfield/brownfield mode toggle, no `origin` field,
and no per-requirement delta markers. Brownfield-aware spec
authoring is the planner skill's job:

- The planner persona detects existing code, lockfiles, and
  conventions in the repo.
- It reads enough context to write SPEC.md prose that accurately
  reflects "this behavior already exists" vs "this is new."
- When a new spec changes a previously-shipped spec, the new spec's
  frontmatter sets `supersedes: [SPEC-NNN]` and the prose explicitly
  references which prior behavior is being changed.

The combination of `frontmatter.status`, `frontmatter.supersedes`,
and the `## Changelog` table is sufficient to track spec evolution
without per-requirement annotations. Reviewers reading a SPEC.md
see immediately what state it's in, what (if anything) it replaces,
and how it has evolved.

## TASKS.md format

`TASKS.md` is Markdown with structure carried by raw XML element
tags. Frontmatter records the generating spec hash; the body holds
each task as a bare `<task>` element directly under the
`# Tasks: SPEC-NNNN ...` heading (no wrapper element). The spec
binding resolves from the frontmatter `spec:` field plus the parent
directory name; there is no redundant `spec="..."` attribute on the
body root.

```markdown
---
spec: SPEC-001
spec_hash_at_generation: sha256:abc...123
generated_at: 2026-05-11T18:00:00Z
---

# Tasks: SPEC-001 User signup

## Phase 1: Schema

<task id="T-001" state="pending" covers="REQ-001">
## T-001: Add `users` table migration with unique email index

<task-scenarios>
Given a fresh database,
when the migration runs forward,
then the `users` table exists with a unique index on `email`.

Given an existing row with email `alice@example.com`,
when a second insert uses the same email,
then the insert fails with a uniqueness violation.
</task-scenarios>

- Suggested files: `migrations/`, `db/schema/users.ts`
</task>

<task id="T-002" state="pending" covers="REQ-002">
## T-002: Add `password_hash` column to `users`

<task-scenarios>
Given a row inserted with a non-empty `password_hash` value,
when the row is read back,
then the column stores the hashed value verbatim.

Given an insert without `password_hash`,
when the database constraint fires,
then the row is rejected.
</task-scenarios>

- Suggested files: `migrations/`, `db/schema/users.ts`
</task>

## Phase 2: API

<task id="T-003" state="pending" covers="REQ-001">
## T-003: Implement `POST /api/signup` handler

<task-scenarios>
Given a request with valid credentials,
when the handler runs,
then it returns 200 with a session token and persists a user row.

Given a request with a duplicate email,
when the handler runs,
then it returns 409 with a message containing "already exists".

Given a request with an uppercase email,
when the handler runs,
then the email is normalized to lowercase before insertion.

Given a request with a malformed email,
when the handler runs,
then it returns 400 with a validation error.
</task-scenarios>

- Suggested files: `src/auth/signup.ts`, `tests/auth/signup.spec.ts`
</task>

<task id="T-004" state="pending" covers="REQ-002">
## T-004: Wire password hashing into signup flow

<task-scenarios>
Given a successful signup,
when the user row is inspected,
then `password_hash` is a valid hash and is not the plaintext password.

Given the hashing routine invoked twice with identical input,
when the resulting hashes are compared,
then they differ (salt is applied).
</task-scenarios>

- Suggested files: `src/auth/signup.ts`, `src/auth/password.ts`
</task>
```

### TASKS.md element grammar

The element shapes mirror the SPEC.md grammar described above
(line-isolated open and close tags, double-quoted attributes,
deterministic rendering).

| Element | Cardinality | Parent | Required attributes | Notes |
|---|---|---|---|---|
| `task` | required, 1+ | top-level (bare under `# Tasks:` heading) | `id="T-NNN"`, `state="..."`, `covers="REQ-NNN[ REQ-NNN]*"` | Body is Markdown plus exactly one `<task-scenarios>` element. No `<implementer>` / `<review>` / `<blockers>` element may appear inside a `<task>` body — that activity prose lives in the sibling `journal/T-NNN.md` file (see "TASKS.md per-task journal" below). |
| `task-scenarios` | required, single per `<task>` | inside `<task>` | none | Slice-level Given/When/Then prose. Must be non-empty. |

Only `task` and `task-scenarios` are the live Speccy element names
inside a TASKS.md body. The closed XML element set across all
Speccy artifacts is five element names total — `task`,
`task-scenarios`, `implementer`, `review`, `blockers` — with the
last three only ever appearing inside `journal/T-NNN.md` (never in
TASKS.md). The legacy `<tasks spec="...">` wrapper, the legacy
`<implementer-note session="...">` element, and the legacy
`<retry>` element are no longer in the allow-list; the parser
rejects any of them as unknown elements.

Valid `state` attribute values are exactly `pending`, `in-progress`,
`in-review`, `completed`. The `covers` attribute is one or more
`REQ-\d{3,}` ids separated by single ASCII spaces. Every covered
requirement id is cross-checked against the parent SPEC.md element
tree at workspace load time. Unknown attributes on a known Speccy
element are parse errors.

Conventions:

- `T-NNN` ids in `<task id="...">` are unique within the file. The
  level-2 heading inside the body is decorative for human readers;
  the parser reads the id from the attribute.
- `covers="..."` is parsed by `speccy next` to know which
  requirements a task touches.
- `<task-scenarios>` carries the slice-level validation contract.
  The implementer translates each Given/When/Then in the block into
  an executable test in the project's framework, **writes the test
  before implementing the code path**, and ensures it passes before
  flipping the task's `state` to `in-review`.
- `Suggested files:` bullets are advisory; Speccy does not enforce
  write scope.
- Phase headings outside `<task>` elements are decorative.

The `<task-scenarios>` convention is what makes TDD legible without
making it a CLI gate. Skills prompt the implementer to write tests
first; the reviewer-tests persona checks that the listed scenarios
exist as tests and meaningfully exercise the claimed behavior.
Speccy itself doesn't verify the order of edits — that's a review
concern.

Speccy parses TASKS.md to:

- read each task's `id`, `state`, and `covers` from the `<task>`
  element attributes
- read the slice-level scenarios from the nested `<task-scenarios>`
  block
- find the next actionable task (`state="pending"`)
- detect "suggested files" hints in the task body

It does not validate journal prose. The sibling `journal/T-NNN.md`
file carries `<implementer>`, `<review>`, and `<blockers>` activity
prose; TASKS.md itself stays free of those elements (see TSK-006).

## SPEC.md element grammar

The machine-readable structure inside `SPEC.md` is carried by
line-isolated **raw XML element tags** wrapping ordinary Markdown.
The Markdown body remains valid Markdown: `<T>` / `A & B` style
content inside a scenario does not need XML escaping, fenced code
blocks pass through verbatim, and the parser is line-aware rather
than being a full XML document parser.

### Syntax

Every Speccy element open tag and close tag occupies its own line.
Opening tags may carry double-quoted attributes; closing tags carry
only the element name with a leading slash.

```markdown
<requirement id="REQ-001">
### REQ-001: Render selected scenarios

Plain Markdown prose remains plain Markdown.

<done-when>
- Implementer-visible acceptance criteria as a bullet list.
</done-when>

<behavior>
- Given/When/Then prose that drives test selection.
</behavior>

<scenario id="CHK-001">
Given a task covers REQ-001,
when `speccy check SPEC-NNNN/T-NNN` runs,
then only REQ-001's scenarios are rendered.
</scenario>
</requirement>
```

Top-level intent sections are wrapped the same way:

```markdown
<goals>
- Concrete outcomes this spec must achieve.
</goals>

<non-goals>
- Explicitly out of scope.
</non-goals>

<user-stories>
- As a <role>, I want <capability> so that <benefit>.
</user-stories>

<assumptions>
- Optional. Preconditions the spec relies on; omit entirely if none.
</assumptions>
```

A Speccy element tag sharing a line with non-whitespace prose is a
parse error. Attribute values without surrounding double quotes are
a parse error. Unknown attributes on a known Speccy element are a
parse error. Element-shaped text outside the whitelist on its own
line is treated as Markdown body content (no parse error, no
structural element).

### Element names

| Element | Cardinality | Location | Attributes |
|---|---|---|---|
| `goals` | required, single | top-level | none |
| `non-goals` | required, single | top-level | none |
| `user-stories` | required, single | top-level | none |
| `assumptions` | optional, 0 or 1 | top-level | none |
| `requirement` | required, 1+ | top-level | `id="REQ-NNN"` |
| `done-when` | required, single | inside `<requirement>`, before `<behavior>` | none |
| `behavior` | required, single | inside `<requirement>`, after `<done-when>` and before `<scenario>` | none |
| `scenario` | required, 1+ inside each requirement | inside `<requirement>`, after `<behavior>` | `id="CHK-NNN"` |
| `decision` | optional, 0+ | top-level | `id="DEC-NNN"`, optional `status="accepted\|rejected\|deferred\|superseded"` |
| `open-question` | optional, 0+ | top-level | optional `resolved="true\|false"` |
| `changelog` | required, single | top-level | none |

Open-tag forms in canonical order:

```markdown
<goals>
<non-goals>
<user-stories>
<requirement id="REQ-001">
<done-when>
<behavior>
<scenario id="CHK-001">
<decision id="DEC-001" status="accepted">
<open-question resolved="false">
<assumptions>
<changelog>
```

The Speccy element whitelist is **disjoint from the HTML5 element
name set** by construction: a `<section>` or `<details>` line in a
SPEC.md body is unambiguously prose, never Speccy structure. The
disjointness invariant is enforced by a unit test against a
checked-in copy of the WHATWG element index. New structural
additions must avoid HTML5 element names; that test catches
accidental collisions at build time.

### IDs and nesting

- Requirement ids match `REQ-\d{3,}`.
- Scenario ids match `CHK-\d{3,}`.
- Decision ids match `DEC-\d{3,}`.
- A `<scenario>` element must be nested inside exactly one
  `<requirement>` element. Containment replaces the old
  `[[requirements]].checks` TOML relation; the parent requirement
  is recorded as `scenario.parent_requirement_id`.
- Duplicate `REQ-`, `CHK-` (within one spec), or `DEC-` ids are
  parse errors.
- The body of each required element (`requirement`, `scenario`,
  `changelog`) must contain non-whitespace Markdown.
- Element-shaped lines hidden inside fenced code blocks or inline
  backticks are treated as code content, not structure. SPEC.md
  files that document Speccy's own grammar (this file included) put
  example tags inside fenced code blocks so the scanner does not
  promote them.

### Deterministic rendering

`speccy-core::parse::spec_xml` exposes `SpecDoc`, `Requirement`,
`Scenario`, `Decision`, `ElementSpan`,
`parse(source, path) -> Result<SpecDoc, ParseError>`, and
`render(&SpecDoc) -> String`. Rendering is deterministic:

- element tags are line-isolated;
- element attributes emit in a stable order;
- requirement and scenario order follows the struct order;
- Markdown bodies are preserved except for trailing whitespace
  normalization at element boundaries;
- parse / render / parse yields a structurally equivalent
  `SpecDoc`.

The deterministic renderer is library-internal. There is no public
`speccy fmt` command; rendering is used by CLI internals, prompt
slicing, and tests.

## REPORT.md

Written by the agent at the end of Phase 5 via the `/speccy-ship`
skill body. Speccy itself does not author REPORT.md and never
renders natural-text prompts.

REPORT.md is Markdown with requirement coverage carried by raw XML
element tags, mirroring SPEC.md and TASKS.md. Outcome and narrative
sections remain plain Markdown.

Suggested shape:

```markdown
---
spec: SPEC-001
outcome: delivered
generated_at: 2026-05-11T19:00:00Z
---

# Report: SPEC-001 User signup

<report spec="SPEC-001">

## Outcome
delivered | partial | abandoned

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
Account creation — project tests in `tests/auth/signup.spec.ts`
exercise CHK-001 and CHK-002 end to end.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
Password storage — project tests in `tests/auth/password.spec.ts`
exercise CHK-003.
</coverage>

## Task summary
- 6 tasks completed
- 1 task retried twice (T-002: bcrypt cost policy)
- 1 task triggered SPEC amendment (T-005 surfaced unknown about
  session TTL)

## Out-of-scope items absorbed
- `tests/migration_helpers.ts` updated alongside T-002
  (implementer note)

## Deferred / known limitations
- Rate limiting on signup endpoint (flagged by security review;
  deferred to SPEC-002)

## PR
[link filled in by agent after `gh pr create`]

</report>
```

### REPORT.md element grammar

| Element | Cardinality | Parent | Required attributes | Notes |
|---|---|---|---|---|
| `report` | required, single | top-level | `spec="SPEC-NNNN"` | Wraps every `<coverage>` element in the file. |
| `coverage` | required, exactly one per surviving SPEC requirement | inside `<report>` | `req="REQ-NNN"`, `result="..."`, `scenarios="CHK-NNN[ CHK-NNN]*"` | Body is plain Markdown explanatory prose. |

### Coverage results

Valid `result` attribute values are exactly `satisfied`, `partial`,
and `deferred`.

- `satisfied` — every scenario nested under the requirement in
  SPEC.md is exercised by a project test that the implementer or
  reviewer can point at.
- `partial` — some scenarios are exercised; others remain. The body
  prose names which ones and why.
- `deferred` — coverage is intentionally pushed to a later spec.
  `scenarios=""` is permitted on `deferred` rows.

There is **no** `dropped` value. If a requirement is genuinely no
longer in scope it is removed from SPEC.md via amendment (with a
Changelog row stating why) rather than carried as a `<coverage>`
row. The renderer enforces "exactly one `<coverage>` per surviving
SPEC requirement"; a requirement that was dropped from the SPEC
disappears from REPORT.md alongside it.

`scenarios` is one or more `CHK-\d{3,}` ids separated by single
ASCII spaces. Each scenario id must be nested under the matching
`<requirement>` in SPEC.md; dangling ids are RPT-* lint errors
(see below).

### Proof-shape gates (RPT-* lint family)

The grammar above is enforced at workspace-load time by the
`RPT-*` lint family. The full entries live in the "Lint Codes"
section below; the short form:

- **RPT-001** — REPORT.md is present but failed to parse (e.g. a
  `<report>` root element missing its `spec="..."` attribute, or
  any malformed XML the `report_xml` parser rejects).
- **RPT-002** — a `<coverage req="REQ-NNN">` row points at a
  requirement id that has no matching `<requirement id="REQ-NNN">`
  in the sibling SPEC.md.
- **RPT-003** — a scenario id listed in
  `<coverage scenarios="...">` does not resolve to a
  `<scenario id="...">` nested under the named requirement in the
  sibling SPEC.md.

All three default to `Level::Error` and gate `speccy verify`. The
existing `partition_lint` demotion pass downgrades them to
`Level::Info` when the owning SPEC.md is `status: in-progress`, so
an in-flight amendment loop is never blocked by a REPORT.md that
has not yet been written.

REPORT.md is the durable record of what happened during the loop.
Future agents reading the repo can reconstruct intent from SPEC.md
and execution history from REPORT.md.

## Decisions (inline ADRs)

Decisions live inside each SPEC.md under `## Design > ### Decisions`
as `#### DEC-NNN: <title>` sub-headings. They follow the classic
ADR shape:

- **Status:** Accepted | Proposed | Rejected | Superseded
- **Context:** Why this decision needs to be made.
- **Decision:** What was chosen.
- **Alternatives:** Other options considered, with brief reason
  each was rejected or deferred.
- **Consequences:** What this commits the project to.

> **Decisions are a documented convention, not a CLI noun.** Speccy
> does not parse decision blocks beyond surfacing them in prompts.
> There is no `speccy decision` command, no separate lifecycle, no
> linting of decision shape. The structure is a convention skill
> prompts nudge agents toward.

`DEC-NNN` IDs are scoped to the spec (like `REQ-NNN` and `CHK-NNN`).
Two specs can both have `DEC-001`; they're local.

When a later spec changes a decision made in an earlier spec, the
later spec records the supersession in its own `### Decisions` block
and references the prior spec in prose:

```markdown
#### DEC-001: Password hashing algorithm
**Status:** Accepted
**Context:** SPEC-001 chose bcrypt cost 12. Subsequent benchmarking
showed argon2id is faster at equivalent security on current
hardware.
**Decision:** Migrate to argon2id with project-standard parameters.
**Supersedes:** SPEC-001 / DEC-001.
**Consequences:** ...
```

Project-wide conventions that aren't tied to any one spec belong in
`AGENTS.md` as prose. AGENTS.md is loaded into every prompt; it's
the natural home for "this is how we do things across all features."

The reviewer-architecture persona reads `### Decisions` blocks in
the SPEC.md it's reviewing. The reviewer-docs persona may notice
when an implementation has drifted from a decision the spec records.

---

# Checks

A Check is an English validation scenario: a durable description of
behavior a requirement must satisfy. The CLI renders scenarios; it
does not execute them. Whether the project tests actually satisfy a
scenario is a question for project CI and for the reviewer-tests
persona.

## Definition

Scenarios live inside SPEC.md as `<scenario id="CHK-NNN">` elements
nested under their parent `<requirement id="REQ-NNN">`:

```markdown
<requirement id="REQ-001">
### REQ-001: Account creation
...

<scenario id="CHK-001">
Given no account exists for alice@example.com,
when the signup endpoint receives a valid request,
then a user row is persisted and the response includes a session token.
</scenario>
</requirement>
```

Required attribute: `id` matching `CHK-\d{3,}`. Unknown attributes
on a `<scenario>` element are parse errors. Empty or whitespace-only
scenario bodies are parse errors naming the containing `CHK-NNN`.

Scenarios are typically Given/When/Then prose, but the CLI does not
parse the inner structure. The body is preserved verbatim except for
trailing whitespace normalisation at element boundaries.

## Rendering

```sh
speccy check                       # render all scenarios across all specs
speccy check SPEC-NNNN             # every scenario under one spec
speccy check SPEC-NNNN/CHK-NNN     # one spec-scoped scenario
speccy check SPEC-NNNN/T-NNN       # scenarios covering one task
speccy check CHK-NNN               # CHK-NNN across every spec (unqualified)
```

Behavior:

- Prints one `==> CHK-NNN (SPEC-NNNN): <scenario first line>` header
  per selected scenario, with continuation lines indented under it.
- Spawns no child processes; writes to no files outside stdout.
- Closes with `N scenarios rendered across M specs`.
- Exits non-zero only for selector, lookup, parse, or workspace
  errors — never because the project's own tests would fail.

That is the whole command. Project tests run through the project's
own test runner (e.g. `cargo test`, `pnpm test`); CI orchestrates
both that runner and `speccy verify` side by side.

---

# Review

Review is an adversarial proof challenge. The CLI renders prompts;
the skill layer orchestrates multiple reviewer personas in parallel.

## Personas

Speccy ships with these personas (markdown skill files):

| Persona | Catches |
|---|---|
| `business` | Does the implementation match SPEC.md intent? Are user stories satisfied? Are non-goals respected? |
| `tests` | Are checks meaningful or vacuous? Edge cases covered? Are negative cases asserted? Is the test exercising the actual behavior, or testing the mock? |
| `security` | Auth, input validation, secrets, injection, sensitive data exposure, access control |
| `style` | Project conventions per `AGENTS.md`, lint compliance, naming, dead code |
| `architecture` | Cross-spec invariants, design adherence, layering, premature abstraction, ADR drift |
| `docs` | Comments, READMEs, inline SPEC.md decisions and AGENTS.md updated to match the change |

The default fan-out (run when the skill does a full review) is:

```
business, tests, security, style
```

Architecture and docs are available via explicit `--persona` but
not in the default set. A future change may make the fan-out
project-configurable; v1 does not.

## Invocation

The seven-command CLI has no `review` verb. Review runs through the
`/speccy-review` skill (Phase 4 primitive). The skill resolves the
target task (either via an explicit `SPEC-NNNN/T-NNN` selector or
via `speccy next --json` filtered for `next_action.kind == "review"`)
and fans out one reviewer sub-agent per persona in the default
fan-out.

Each persona sub-agent is loaded from its host-native agent file
(`.claude/agents/reviewer-<persona>.md` or its Codex parallel,
materialised from `resources/modules/personas/reviewer-<persona>.md`
by `speccy init`). The shipped persona body composes a prompt that
includes:

- the relevant SPEC.md (full, including its `### Decisions` block)
- the task body from TASKS.md (the bare `<task>` element — no inline
  notes live there post-T-001)
- prior implementer / reviewer / blocker history from
  `.speccy/specs/NNNN-slug/journal/T-NNN.md`
- the diff for the task's claimed work
- `AGENTS.md`
- the persona's review-style guidance from the shipped persona body

The reviewer sub-agent reads the prompt, performs the review, and
returns a single `<review>` element to the orchestrator. The
orchestrator (the `/speccy-review` skill session, sole writer to the
journal during review) appends each returned `<review>` to
`.speccy/specs/NNNN-slug/journal/T-NNN.md` serially. Reviewer
sub-agents never write to TASKS.md or to the journal themselves —
TSK-006 rejects `<review>` elements inside `<task>` bodies, and the
journal-writer concurrency contract belongs to the orchestrator.

## State transitions

Persona sub-agents **do not** flip the task's `state` attribute.
That would create a race when the four personas run in parallel. The
`/speccy-review` skill flips state once after all four persona
`<review>` elements have been aggregated:

- All `verdict="pass"` -> `state="in-review"` becomes
  `state="completed"`.
- Any `verdict="blocking"` -> `state="in-review"` becomes
  `state="pending"`, and the orchestrator appends a `<blockers>`
  block to `journal/T-NNN.md` summarising the blocking findings
  (`round` monotonically increasing across rounds; see the journal
  element grammar).

This puts state-mutation atomicity in one place (the orchestrator
session) and keeps the journal the single source of truth for
review history.

## Why personas live in skills, not CLI

The CLI cannot know what "security" means in this project. The
skill prompt does. By making personas markdown skill files, three
things become possible:

1. Add a new persona without changing the CLI.
2. Swap persona definitions when models improve.
3. Projects edit the host-native sub-agent file in place
   (`.claude/agents/reviewer-security.md`,
   `.codex/agents/reviewer-security.toml`). The host-native location
   is the only persona surface, and `speccy init` classifies those
   files Skip-on-exists so a local edit survives
   `speccy init --force`.

---

# Amendment

Amendments are not a separate first-class artifact in v1. The
amendment story is a **skill concern** built from existing CLI
primitives. There is no `speccy amend` verb, and no longer any
`speccy plan` / `speccy tasks` rendering verbs either; the existing
seven-verb CLI is sufficient.

## What happens when SPEC.md needs to change

The `/speccy-amend SPEC-001` skill orchestrates the mid-loop change
in the parent session:

1. The skill reads `speccy status SPEC-001 --json` to learn
   `spec_md_path`, `tasks_md_path`, and the spec's current
   `next_action`.
2. The agent edits `SPEC.md` surgically — preserve what works,
   minimal diff, append a `## Changelog` row recording the why.
3. The agent edits `TASKS.md` surgically:
   - keep `state="completed"` tasks unless invalidated by the
     SPEC change;
   - keep `state="in-progress"` / `state="in-review"` tasks unless
     invalidated;
   - flip invalidated `state="completed"` tasks back to
     `state="pending"` with a "spec amended" note;
   - add new `<task>` elements for new requirements;
   - remove tasks for dropped requirements.
4. The skill calls `speccy lock SPEC-001` to record the new
   `spec_hash_at_generation` + `generated_at` into TASKS.md
   frontmatter so subsequent staleness checks pass.

The skill body ships as host skill content under
`.claude/skills/speccy-amend/SKILL.md` and the Codex twin. There is
no CLI-embedded prompt template for amendment; the recipe lives
entirely in the skill file.

## Lineage

Speccy does not maintain an amendment registry. Two mechanisms
cover the lineage need:

1. **`## Changelog` table in SPEC.md.** Curated, prose-summarized
   history of material edits. Each row records date, author, and
   summary. This is what gets loaded into review and amendment
   prompts so future agents understand recent intent shifts.
2. **Git history.** Authoritative literal lineage. `git log SPEC.md`
   and `git log TASKS.md` show every change ever made.

The previous design's `amendments/` folder and `AMD-NNN` IDs were
ceremony that duplicated git's job. The Changelog table replaces
both at far lower cost.

---

# Spec Staleness Detection

When SPEC.md is edited mid-loop (between Phase 2 and Phase 5),
TASKS.md may no longer reflect the current spec. Speccy detects
this via the content hash: TASKS.md frontmatter's
`spec_hash_at_generation` stores the sha256 of SPEC.md at the time
TASKS.md was generated. `speccy status` recomputes the current
hash and compares; a mismatch is the sole stale signal beyond the
`bootstrap-pending` sentinel.

If it drifts, `speccy status` reports:

```text
SPEC-001: TASKS.md may be stale.
  Hash drift: SPEC.md sha256 changed since tasks were generated.
  Run /speccy:amend to reconcile.
```

This is a soft warning. The user / skill decides what to do. No
gate fires.

---

# Skills / Harness Layer

Speccy v1 ships official skill packs alongside the CLI. They are
not optional polish; they are how the system becomes usable end-
to-end without each project inventing its own integration.

## What ships in v1

```
resources/
  modules/
    skills/                  Full-body SKILL.md sources for interactive skills
      speccy-brainstorm.md
      speccy-plan.md
      speccy-amend.md
      speccy-review.md       Orchestrator (dispatches the six reviewer sub-agents)
      speccy-orchestrate.md  Drives the per-task work + review loop across one SPEC
      speccy-holistic-gate.md Pre-ship SPEC-vs-impl drift review + autonomous fix loop
    phases/                  Full-body sources for the pinned phase workers
      speccy-init.md         Used by speccy-init's interactive SKILL.md too
      speccy-tasks.md        Decompose one SPEC into TASKS.md
      speccy-work.md         Implement one task (single-task primitive)
      speccy-ship.md         Write REPORT.md + open PR
    personas/                Six reviewer persona bodies + co-located snippets
      reviewer-business.md
      reviewer-tests.md
      reviewer-security.md
      reviewer-style.md
      reviewer-architecture.md
      reviewer-docs.md
      verdict_return_contract.md   Topic-named snippets {% included %} by
      inline_note_format.md         the persona bodies above. Filename pattern
      no_tasks_md_writes.md         distinguishes snippets from body files;
      diff_fetch_command.md         no `_partials/` subdirectory exists.
    references/              Canonical worked-instance reference files,
      spec.md                ejected into per-skill `references/` and
      tasks.md               host-shared `speccy-references/` directories
      report.md              under both host packs. See "Skill-pack
      journal-implementer.md reference files" below for the seven-row
      journal-review.md      artifact→path mapping (SPEC-0038 REQ-002 is
      evidence.md            the source of truth).
      journal-blockers.md
  agents/                    Per-host MiniJinja wrappers (rendered at init time)
    .claude/skills/speccy-<verb>/SKILL.md.tmpl   Eight skills total
    .claude/agents/speccy-{tasks,work,ship}.md.tmpl   Pinned phase workers
    .claude/agents/reviewer-<persona>.md.tmpl    Six reviewer sub-agents
    .agents/skills/speccy-<verb>/SKILL.md.tmpl   Codex skill files
    .codex/agents/speccy-{tasks,work,ship}.toml.tmpl
    .codex/agents/reviewer-<persona>.toml.tmpl
```

There is no `resources/modules/prompts/` directory and no embedded
phase prompt body inside the CLI binary. The CLI ships
`resources/modules/` as the single source of truth for skill
content and `resources/agents/` as the per-host MiniJinja wrappers;
nothing else.

## Skill-pack reference files

Each lintable Speccy artifact has exactly one canonical reference
file under `resources/modules/references/`, ejected by `speccy init`
into either a per-skill `references/` subdirectory (single-consumer)
or a host-shared `speccy-references/` directory at host root
(multi-consumer). The path's first component classifies the file;
no separate manifest declares it. The mapping:

| Artifact                        | Source                                              | Claude Code path                                                  | Codex path                                                       | Class       |
|---------------------------------|-----------------------------------------------------|-------------------------------------------------------------------|------------------------------------------------------------------|-------------|
| SPEC.md                         | `resources/modules/references/spec.md`              | `.claude/skills/speccy-plan/references/spec.md`                   | `.agents/skills/speccy-plan/references/spec.md`                  | skill-local |
| TASKS.md                        | `resources/modules/references/tasks.md`             | `.claude/skills/speccy-tasks/references/tasks.md`                 | `.agents/skills/speccy-tasks/references/tasks.md`                | skill-local |
| REPORT.md                       | `resources/modules/references/report.md`            | `.claude/skills/speccy-ship/references/report.md`                 | `.agents/skills/speccy-ship/references/report.md`                | skill-local |
| journal `<implementer>` block   | `resources/modules/references/journal-implementer.md` | `.claude/skills/speccy-work/references/journal-implementer.md`  | `.agents/skills/speccy-work/references/journal-implementer.md`   | skill-local |
| journal `<review>` block        | `resources/modules/references/journal-review.md`    | `.claude/skills/speccy-review/references/journal-review.md`       | `.agents/skills/speccy-review/references/journal-review.md`      | skill-local |
| evidence file (`evidence/T-NNN.md`) | `resources/modules/references/evidence.md`      | `.claude/speccy-references/evidence.md`                           | `.agents/speccy-references/evidence.md`                          | host-shared |
| journal `<blockers>` block      | `resources/modules/references/journal-blockers.md`  | `.claude/speccy-references/journal-blockers.md`                   | `.agents/speccy-references/journal-blockers.md`                  | host-shared |

Host-shared rows have two consuming bodies each: `evidence.md` is
referenced by `/speccy-work` (writes evidence) and the
`reviewer-tests` sub-agent (reads evidence); `journal-blockers.md`
is referenced by `/speccy-review` (writes review-induced blockers)
and `/speccy-amend` (writes amendment-induced blockers). All other
rows have exactly one consuming body and live skill-local.

SPEC-0038 REQ-002 is the source of truth. The
`chkNNN_no_orphan_references` test in
`speccy-cli/tests/skill_body_discovery.rs` asserts every ejected
reference file is reached by ≥1 path-substring pointer from a
consuming body, and that source-to-host and cross-host bytes match.

## `speccy init` host detection

```sh
speccy init                  # detects host from environment
speccy init --host claude-code
speccy init --host codex
```

Init renders the per-host wrappers into host-native locations:

- Claude Code: `.claude/skills/speccy-<verb>/SKILL.md` plus
  `.claude/agents/speccy-{tasks,work,ship}.md` for the pinned phase
  workers and `.claude/agents/reviewer-<persona>.md` for the six
  reviewer sub-agents.
- Codex: `.agents/skills/speccy-<verb>/SKILL.md` plus
  `.codex/agents/speccy-{tasks,work,ship}.toml` and
  `.codex/agents/reviewer-<persona>.toml`.

Existing files are handled by a three-way per-file classification:
absent → `created`; byte-identical to planned content → `unchanged`;
differs from planned content → `Conflict`, and the entire batch
refuses atomically unless `--force` is passed, in which case
differing files are overwritten with the `(!) overwritten` log
marker. Recovery from an unwanted overwrite is via `git checkout`;
there is no in-CLI merge or backup mechanism. Host-native reviewer
files (`.claude/agents/reviewer-<persona>.md` and the Codex twin)
remain Skip-on-exists so local persona-body edits survive
`--force`.

## Workflow recipes

Each top-level skill is a recipe. The five interactive skills
(`speccy-init`, `speccy-brainstorm`, `speccy-plan`, `speccy-amend`,
`speccy-review` orchestrator) eject as full-body SKILL.md only. The
three pinned phase workers (`speccy-tasks`, `speccy-work`,
`speccy-ship`) eject as a thin SKILL.md stub (≤10 non-blank lines
naming the matching agent file) plus a full-body agent file at
`.claude/agents/speccy-<phase>.md` (Codex:
`.codex/agents/speccy-<phase>.toml`). The eject shape follows the
invocation pattern: recurring loop-friendly phases pin a
heavy-model fork via the agent file; interactive skills stay in
the parent session.

- `/speccy-init` -- bootstrap the project (interactive)
- `/speccy-brainstorm` -- atomize a fuzzy ask before drafting a SPEC
- `/speccy-plan` -- Phase 1 (AGENTS.md north star + optional MISSION.md -> SPEC)
- `/speccy-tasks` -- Phase 2 (SPEC -> TASKS, then `speccy lock`)
- `/speccy-work` -- Phase 3 (implement one task)
- `/speccy-review` -- Phase 4 (review one task; orchestrator)
- `/speccy-amend` -- Mid-loop SPEC + TASKS reconciliation
- `/speccy-ship` -- Phase 5 (REPORT.md + PR)
- `/speccy-orchestrate` -- Chains `/speccy-work` + `/speccy-review` across every task in one SPEC, hands off to `/speccy-holistic-gate` before the ship boundary
- `/speccy-holistic-gate` -- Pre-ship SPEC-vs-implementation drift review with an autonomous fix-retry loop; invoked by the orchestrator and runnable on its own

A typical full session in Claude Code looks like:

```
/speccy-plan
[agent reads `speccy vacancy --json`, writes SPEC.md]

/speccy-tasks SPEC-001
[agent writes TASKS.md, then `speccy lock SPEC-001`]

/speccy-work SPEC-001/T-001
[agent implements one task, flips state="pending" -> state="in-review", exits]

/speccy-review SPEC-001/T-001
[orchestrator fans out four personas on one task, aggregates notes,
 flips state="in-review" -> state="completed" (or back to "pending"
 with a Retry note), exits]

[caller re-invokes /speccy-work and /speccy-review on the remaining
 tasks; the existing /loop skill is the interim composer for batched
 iteration]

/speccy-ship SPEC-001
[agent writes REPORT.md, opens PR]
```

The CLI is invoked many times during this; the skill knows when.

## Persona definitions

Each persona file is a markdown skill describing:

- the role (one paragraph)
- review focus areas (bullet list)
- what to look for that is easy to miss
- format of the inline note to append
- a worked example

Example skeleton for `reviewer-security.md`:

```markdown
# Reviewer Persona: Security

## Role
You are an adversarial security reviewer for one task in one spec.
You read the SPEC.md, the task's diff, and the prior journal
entries in `.speccy/specs/NNNN-slug/journal/T-NNN.md`. You return
a single `<review>` element to the orchestrator, which appends it
verbatim to the journal file.

## Focus
- Authentication and authorization boundaries
- Input validation and injection vectors
- Secret handling, credential storage, token lifecycle
- Sensitive data exposure in logs, errors, responses
- Race conditions affecting authorization
- Cryptographic primitives and parameter choices

## What to look for that's easy to miss
- Plaintext leaks in logs even when storage is hashed
- Authorization checks that pass before resource lookup (TOCTOU)
- Error messages that disclose user existence
- Missing rate limiting on auth endpoints

## Return format
Return one `<review>` element to the orchestrator, of the shape:

    <review persona="security" verdict="pass | blocking"
            date="<ISO8601>" model="<model-id>[/effort]" round="<N>">
    <one-paragraph summary; file:line refs encouraged>
    </review>

The orchestrator transcribes the element verbatim into
`journal/T-NNN.md`. The `model` attribute is required and is
copied verbatim from this returned element; the orchestrator does
not infer it from skill-pack identity.

## Example

    <review persona="security" verdict="blocking"
            date="2026-05-21T19:00:00Z"
            model="claude-sonnet-4-6[1m]/medium" round="1">
    bcrypt cost 10; policy requires >=12.
    See `src/auth/password.ts:14`.
    </review>
```

These files are the durable surface where review intelligence
lives. They are upgradeable as models improve; the CLI is not.

---

# JSON Interfaces

Four commands carry stable JSON contracts: `status`, `next`,
`vacancy`, and `verify`. `--json` switches representation; the
content is the same as the text output. Schema versions are pinned
per-envelope and bumped only on breaking shape changes.

## `speccy status --json`

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
  ]
}
```

By default `speccy status` shows only specs with
`status: in-progress` plus any with stale evidence or lint errors
regardless of status. Pass a positional `SPEC-NNNN` selector for one
spec, or `--all` to render every spec in workspace order. Specs with
`status: implemented`, `dropped`, or `superseded` are excluded from
the default view but always present in `--json` output so harnesses
can filter as needed.

Per-spec entries carry resolved paths (`spec_md_path`,
`tasks_md_path`, `mission_md_path`) as repo-relative forward-slash
strings. `tasks_md_path` is `null` when TASKS.md does not yet
exist; `mission_md_path` is `null` when the spec lives flat (no
mission folder). The `superseded_by` field is **computed** at query
time by walking every parsed SPEC.md's `frontmatter.supersedes` and
inverting the relation; it does not appear on disk.

## `speccy next --json`

Workspace form (no positional selector) — every active spec with
its derived `next_action`:

```json
{
  "schema_version": 1,
  "specs": [
    {
      "spec_id": "SPEC-0001",
      "next_action": { "kind": "review", "task_id": "T-002" },
      "spec_md_path": ".speccy/specs/0001-user-signup/SPEC.md",
      "tasks_md_path": ".speccy/specs/0001-user-signup/TASKS.md",
      "mission_md_path": null
    },
    {
      "spec_id": "SPEC-0002",
      "next_action": { "kind": "decompose" },
      "spec_md_path": ".speccy/specs/0002-password-reset/SPEC.md",
      "tasks_md_path": null,
      "mission_md_path": null
    }
  ]
}
```

Per-spec form (positional `SPEC-NNNN`) — one entry, or
`{ next_action: null, reason }` when the spec is `completed` or
`superseded`. Action kind is derived from spec state via the
priority rule `review > implement > ship`, with `decompose` when
TASKS.md is absent. There is no `--kind` flag: spec state fully
determines the kind, so caller-supplied filtering would be
redundant. Skills that want only one kind read the envelope and
filter on `next_action.kind` themselves.

## `speccy vacancy --json`

```json
{ "schema_version": 1, "next_spec_id": "SPEC-0036" }
```

Used by `/speccy-plan` so the skill never globs `.speccy/specs/`
itself. One field, one query, smallest possible
payload.

## `speccy verify --json`

```json
{
  "schema_version": 1,
  "repo_sha": "abc123",
  "lint": {
    "errors": [],
    "warnings": [],
    "info": []
  },
  "specs_total": 35,
  "requirements_total": 142,
  "scenarios_total": 287
}
```

No `outcome`, `exit_code`, or `duration_ms` fields; the binary exit
code is the contract for CI scripts, and the JSON payload is for
downstream tooling that wants structured failure detail. Diagnostics
on `in-progress` / `dropped` / `superseded` specs are demoted to
`Level::Info` so only `implemented` specs gate the build.

These four envelopes are everything a harness needs. The rest of
the CLI surface is text output to humans.

---

# Lint Codes

Speccy emits a small set of deterministic lint codes. None depend
on LLM judgment. All have stable prefixes: `SPC-` for spec
structure, `REQ-` for requirements, `TSK-` for task structure,
`QST-` for open questions, `RPT-` for REPORT.md proof shape, and
`JNL-` for `journal/T-NNN.md` per-task journal proof shape.
The canonical, append-only list lives in
`speccy-core::lint::registry::REGISTRY`; the snapshot test at
`speccy-core/tests/lint_registry.rs` pins it. The summary below
mirrors the registry.

```text
SPC-001  Stray per-spec spec.toml file present in spec directory
         (the only TOML carried at spec level today is the workspace
         `.speccy/speccy.toml`; a per-spec spec.toml is a stale file)
SPC-002  SPEC.md element tree malformed: heading declares an ID but
         no matching `<requirement>` element exists
SPC-003  SPEC.md element tree malformed: `<requirement>` element
         exists but SPEC.md has no matching `### REQ-NNN` heading
SPC-004  SPEC.md frontmatter missing required field
         (id / slug / title / status / created)
SPC-005  SPEC.md frontmatter status value is not one of:
         in-progress, implemented, dropped, superseded
SPC-006  status = superseded but no other spec in the workspace
         declares `supersedes` pointing to this spec
SPC-007  status = implemented but some tasks have state != "completed"
         (informational)

REQ-001  Requirement has no nested <scenario> element

TSK-001  TASKS.md task references non-existent REQ ID
TSK-002  TASKS.md task ID format invalid (expected T-NNN)
TSK-003  Spec hash mismatch: TASKS.md may be stale relative to
         SPEC.md (warning, not error)
TSK-004  TASKS.md frontmatter missing required field
         (spec / spec_hash_at_generation / generated_at)
TSK-005  Spec ID disagreement: folder digits, SPEC.md frontmatter
         `id:`, and TASKS.md frontmatter `spec:` must all agree
         (error; skipped when any of the three is unobtainable so
         upstream parse-error diagnostics cover those cases)
TSK-006  Misplaced journal element in TASKS.md: an `<implementer>`,
         `<review>`, or `<blockers>` element appears inside a
         `<task>` body. These elements only ever live in
         `journal/T-NNN.md` (Level::Error). Not gated by task
         state — fires identically against pending, in-progress,
         in-review, and completed tasks. Fires before any JNL-*
         diagnostic on the same task.

JNL-001  Task `state="pending"` but `journal/T-NNN.md` exists
         (Level::Error). A pending task has no implementer history;
         a journal file is unexpected.
JNL-002  Task `state="completed"` but `journal/T-NNN.md` is missing
         (Level::Error). Every completed task must carry its
         journal as the durable record of how it was implemented
         and reviewed.
JNL-003  Task `state="completed"` and `journal/T-NNN.md` has a
         shape or binding violation (Level::Error). Covers
         filename ↔ frontmatter `task:` mismatch, frontmatter
         `spec:` ↔ parent-dir mismatch, missing or unparseable
         frontmatter, attribute-schema violations on
         `<implementer>` / `<review>` / `<blockers>`, and
         round-monotonicity violations (first round must be 1,
         monotonic non-decreasing, no skipped rounds).
         The JNL-* family silently skips tasks at
         `state="in-progress"` or `state="in-review"` — a
         half-written journal in flight is not a lint error.

QST-001  SPEC.md has unchecked open question (informational)

RPT-001  REPORT.md present but failed to parse (Level::Error).
         Fires when `ParsedSpec.report_md` is `Some(Err(_))`.
         Covers every failure the parser returns: missing `spec="..."`
         attribute on the root `<report>` element, malformed
         `<coverage>` shape, fenced-code-block boundary violations,
         and any other parse error. The diagnostic message includes
         the underlying parse error rendered via its Display impl.
RPT-002  `<coverage req="REQ-NNN">` row references a requirement id
         that has no matching `<requirement id="REQ-NNN">` in the
         sibling SPEC.md (Level::Error). Fires once per dangling
         reference. Does not fire when SPEC.md itself failed to parse
         (SPC-001 owns that surface). When RPT-002 fires for a row,
         RPT-003 does not fire for any of that row's scenarios.
RPT-003  Scenario id in `<coverage scenarios="...">` does not resolve
         to a `<scenario id="...">` nested under the named requirement
         in the sibling SPEC.md (Level::Error). Fires once per
         dangling scenario id. Suppressed for rows where RPT-002
         already fired (the row is already broken at the requirement
         level; one diagnostic per row rather than N).
```

`REQ-002` and `REQ-003` are registry-only entries kept for stability:
both fired pre-XML-canonical-SPEC.md but are no longer reachable at
parse time (the parser rejects orphan scenarios before lint runs).
Their slots stay in the snapshot so removing them would be a
breaking change.

Nothing in this list grades scenario quality mechanically. The CLI
flags presence and reference shape only; whether a scenario is
meaningful and whether the project tests actually cover it goes to
review.

Lint codes are stable: changing a code's meaning between minor
versions is a breaking change. Adding new codes is fine.

---

# What We Deliberately Don't Do

These are not v1 features. Each was considered and rejected.

| Cut | Reason |
|---|---|
| Capability map (`CAP-NNN`) | Mission folders (`specs/[focus]/MISSION.md`) cover grouping. No second taxonomy. |
| Milestone state machine | Replaced by tag-based releases + a checklist file if the project wants one. Missions are *scope*, not lifecycle. |
| Release readiness as separate gate | Same: git tag + checklist. Not first-class. |
| Decision (ADR) as a separate noun | Decisions live inline in SPEC.md as `### Decisions` blocks. No separate folder, no CLI command, no lifecycle machinery. |
| Amendment as TOML | Replaced by SPEC.md frontmatter `status` + `## Changelog` table. |
| Assumption / Constraint / Invariant / Question as TOML | All collapse into SPEC.md narrative sections. |
| Scenario as separate noun | Folded into `Requirement.done_when` prose. |
| Per-requirement delta markers (`[ADDED]`/`[MODIFIED]`/`[REMOVED]`) | SPEC.md frontmatter `status` + `supersedes` + `## Changelog` table cover lifecycle. |
| Archive folder for completed specs | Frontmatter `status` is the indicator. Filesystem reorganization adds friction with no information gain. |
| Task `writes` globs and scope enforcement | LLMs declare them wrong; enforcement was net-negative. |
| Claim files / leases | No locking. `state="in-progress"` + session marker on the `<task>` element is enough. |
| TDD exception registry | Don't gate on TDD. Review's job. |
| `critical` flag on requirements | All requirements equal. |
| `origin` field | Brownfield context is the planner skill's responsibility, not a TOML field. |
| Check `inputs` and freshness hashing | Wrong inputs poison the model worse than no inputs. Project CI runs tests. |
| Check evidence records | Project CI captures execution; no need to commit. |
| Speccy executing project tests | Project CI runs `cargo test` / `pnpm test` directly; `speccy verify` only validates proof shape. |
| Phase prompt rendering in the CLI | Skill bodies under `.claude/skills/` and `.agents/skills/` carry the phase prose; the binary never renders natural-text prompts. |
| `--strict` flag | Opinionated, not configurable. |
| Validation kind enum | Free-form string with conventions. |
| Solo review policy toggle | Different sessions / personas suffice. |
| In-process LLM calls | CLI ships state queries and lint only; never invokes models. |
| Worktree orchestration | Harness concern. |
| Distributed locks | Harness concern. |
| External tracker sync | Harness concern. |
| Plugin ecosystem | Premature. |
| Identity provider integration | Premature. |
| Runtime telemetry | Out of scope. |
| Mutation testing | Out of scope. |
| Semantic dependency analysis | Out of scope. |
| Bad-test detection beyond no-op commands | Review owns this. |
| Public `speccy fmt` command | The deterministic SPEC.md renderer ships as library functionality (used by CLI internals and tests); a user-facing formatter is out of scope for v1. |

The point is not that these features are wrong. The point is that
v1 is small enough to trust.

---

# Comparison to Peers

Brief positioning. None of these are wrong; Speccy borrows from
each.

| Tool | Strength Speccy borrows | Speccy diverges by |
|---|---|---|
| **OpenSpec** | Lightweight change proposals, low-ceremony | Smaller surface; less focused on iterative review loop |
| **Spec Kit** | `/specify` `/plan` `/tasks` opinionated flow, PRD-shaped templates | Speccy adds adversarial review loop, multi-persona |
| **Kiro** | Steering files for durable agent context | We use `AGENTS.md` + `skills/`; no IDE coupling |
| **GSD** | Milestone-driven verification, autonomy levels | Speccy drops formal milestones; verification stays |
| **BMAD** | Phased context engineering, agent personas | Personas in skills, not built-ins; phases match |
| **Cursor rules** | Rule-folder layering for persistent context | `AGENTS.md` + `.claude/rules/` adopted directly |

Speccy's distinctive bet: **multi-persona adversarial review run by
the same agent host that did the implementation**, with state and
notes living in markdown the same agent will read in the next
iteration. That is where drift gets caught in this system.

---

# Threat Model

V1 makes these failures loud:

- Spec has no requirements
- Requirement has no nested `<scenario>` element
- Spec ID disagreement: folder digits, SPEC.md frontmatter `id:`,
  and TASKS.md frontmatter `spec:` are not all the same
- TASKS.md references requirements that don't exist
- TASKS.md is stale relative to SPEC.md (hash drift)
- Open question in SPEC.md is unchecked
- Reviewer persona returns `blocking`
- Task is `state="in-review"` but at least one persona review is missing
- REPORT.md `<coverage>` element references a requirement or
  scenario that does not resolve under the sibling SPEC.md
- Per-task `journal/T-NNN.md` is missing for a completed task,
  exists for a pending task, or has shape / binding / round-sequence
  violations
- `<implementer>`, `<review>`, or `<blockers>` element appears
  inside a `<task>` body in TASKS.md (misplaced — they belong in
  the sibling journal file)

V1 intentionally does not catch:

- Semantic correctness of any scenario
- Whether the project tests actually satisfy a scenario (project CI
  and the reviewer-tests persona own this)
- Whether the implementation actually meets `done_when`
- Whether the reviewer was thorough
- Whether the agent invented assumptions in `<implementer>` journal entries
- Whether the PR description matches REPORT.md
- Whether the project will work end-to-end in production
- Architecture drift across specs

Those failures are review's job, the human's job, or out of scope
for a feedback engine.

---

# Operational Details

Implementation choices and conventions. Each was considered and
locked in so implementers don't rediscover them.

## Parsing stack

| Concern | Crate | Version pin |
|---|---|---|
| Markdown body | `comrak` (CommonMark + GFM tables) | latest stable |
| YAML deserialization | `serde-saphyr` (serde adapter over `saphyr-parser`) | exact `0.0.x` |
| TOML deserialization | `toml` | latest stable |
| Targeted regex | `regex` (only for ID extraction from heading text) | latest stable |

**YAML choice rationale.** The Rust YAML ecosystem is in flux as of
May 2026: dtolnay's `serde_yaml` is deprecated, and the most common
"fork" `serde_yml` was archived in September 2025 with RUSTSEC-2025-0068
(unsound, panics on malformed input). `serde-saphyr` is the live
choice — built on `saphyr-parser` (the actively-developed pure-Rust
YAML 1.2 parser, successor to `yaml-rust`), with direct-to-struct
deserialization (no `Value` AST roundtrip), panic-free on malformed
input, and configurable resource budgets that defend against
Billion-Laughs attacks. The first-party `saphyr-serde` is announced
but not yet released; `serde-saphyr` is the practical choice today.

Caveat: `serde-saphyr` is `0.0.x` (pre-`0.1.0`). Pin exact version and
expect a minor refactor when it stabilizes. Acceptable tradeoff vs.
shipping a CI gatekeeper on top of an actively-unsafe crate.

**Frontmatter extraction is DIY.** The `---` fence parser is ~4 lines
of string slicing returning `&str` slices for the YAML chunk and the
markdown body. `gray_matter` was considered and rejected: it would
pull in `yaml-rust2` transitively for zero gain over a tiny custom
splitter, and tying frontmatter parsing to one specific YAML crate
makes future migration harder.

**Pure-regex parsing was considered and rejected.** SPEC.md contains
fenced code blocks with example markdown (this document does too),
and regex cannot reliably skip those contexts. The 4-crate cost is
worth the robustness.

TASKS.md, REPORT.md, and `journal/T-NNN.md` share the same
line-aware XML element scanner as SPEC.md.
`speccy-core::parse::task_xml` extracts the bare `<task>` /
`<task-scenarios>` tree (no `<tasks>` wrapper); `report_xml`
extracts the `<report>` / `<coverage>` tree; `journal_xml` extracts
the chronological bare-element sequence of `<implementer>` /
`<review>` / `<blockers>` blocks (no wrapper) under the
frontmatter. Body Markdown inside each element is preserved
verbatim except for trailing whitespace normalization at element
boundaries. No regex is used for structure; element opens, closes,
and attributes are parsed line-by-line with fenced-code awareness
inherited from SPEC.md.

## Spec ID allocation

Global ID space. `speccy plan` walks `.speccy/specs/**/SPEC.md`
across every mission folder and every flat (ungrouped) spec, finds
the maximum `NNNN-` prefix, and increments. SPEC-NNN IDs are unique
repo-wide regardless of which mission folder a spec sits in. Moving
a spec into or out of a mission folder does not change its ID. Gaps
left by dropped specs are not recycled.

## `speccy init` behavior

Refuses to run if `.speccy/` already exists, unless `--force` is
passed. Before doing anything destructive, prints the list of
files that would be created or overwritten.

Host detection for skill-pack copy:

1. `--host <name>` flag if passed
2. Presence of `.claude/` -> Claude Code
3. Presence of `.codex/` -> Codex
4. Presence of `.cursor/` -> Cursor
5. Fall back to `claude-code` and print a warning

The user can re-run `speccy init --host <other> --force` to swap.

## Section heading discovery in SPEC.md

Case-insensitive exact match. `## Open Questions`, `## open
questions`, `## OPEN QUESTIONS` all match. Hyphens and spaces in
heading text are treated equivalently for matching. Unknown
headings are ignored, not flagged.

## Frontmatter dates

- `created`: ISO 8601 date (`YYYY-MM-DD`)
- `generated_at`, `recorded_at`: ISO 8601 datetime in UTC
  (`YYYY-MM-DDTHH:MM:SSZ`)

Missing optional frontmatter fields are treated identically to
empty lists or null. The parser does not distinguish.

## Persona file resolution

Lookup order:

1. Project-local: `.speccy/skills/personas/reviewer-X.md` (copied
   to `.speccy/skills/` at `speccy init` time so users can tune
   them)
2. Shipped fallback compiled into the CLI binary:
   `resources/modules/personas/reviewer-X.md`

If the project-local override exists but is malformed, lint warns
and the CLI falls through to the shipped version.

## Subdirectory naming

Spec folders: `NNNN-slug-with-hyphens`. Slug is lowercase ASCII
only. Lint warns on uppercase or non-ASCII. Mismatch between
`frontmatter.slug` and the actual folder name is a lint error.

## Schema version

`.speccy/speccy.toml` requires `schema_version = 1`. SPEC.md /
TASKS.md / REPORT.md frontmatter implicitly target schema version
1; no declaration needed. The CLI rejects unknown `schema_version`
values with a clear error.

## `speccy verify` exit code

Binary. `0` if proof shape is intact (specs parse, every requirement
has at least one scenario, every referenced scenario resolves, no
scenarios are unreferenced); `1` otherwise. `speccy verify` does
not execute project tests; CI runs the project's own test commands
alongside it. Detailed breakdown is available via
`speccy verify --json` (`schema_version = 1`; no `outcome`,
`exit_code`, or `duration_ms` fields). CI scripts only check the
exit code; downstream tooling parses the JSON if it needs
structured failure info.

## `speccy next` priority

Per-spec, the derived `next_action.kind` follows
`review > implement > ship`, with `decompose` when TASKS.md is
absent. Drift visibility favours short feedback
loops; bugs caught in the piecewise (implement → review → implement
→ review) workflow are cheap, while bugs caught after multiple
tasks build on top of an inherited mistake are expensive, so the
default nudges agents toward piecewise. Callers that want
batched-implementation Pattern B override by invoking
`/speccy-work SPEC-NNNN/T-NNN` directly against a `state="pending"`
task; the CLI surfaces a recommendation, not a gate. Workspace-form
ordering is lowest spec ID first.

## `speccy check` rendering

Serial. For each selected scenario, the command prints
`==> CHK-NNN (SPEC-NNNN): <scenario first line>` followed by
indented continuation lines, then closes with `N scenarios
rendered across M specs`. The working directory is the project
root (the directory containing `.speccy/`). No subprocesses are
spawned; exit code is non-zero only for selector, lookup, parse,
or workspace errors.

## Reviewer diff scoping

The diff the reviewer sub-agent reasons over is fetched by the
`/speccy-review` orchestrator (or the persona body itself, depending
on the persona) via `git diff HEAD` — the implementer's
uncommitted edits, which is the natural moment of review. When the
working tree is clean (the implementer already committed) the
fallback is `git diff HEAD~1 HEAD`; if that yields nothing relevant,
the persona body falls back to SPEC.md plus task notes alone. The
CLI is not in this loop; it does not invoke git.

---

# Implementation Sequence

Speccy was built in this order, and the current binary reflects the
end state:

1. Artifact parser: `speccy.toml`, SPEC.md (YAML frontmatter + XML
   element tree via `speccy-core::parse::spec_xml` + Changelog
   table), TASKS.md (YAML frontmatter + `task_xml` element tree),
   REPORT.md (YAML frontmatter + `report_xml` element tree).
2. `speccy init` — scaffold + host skill copy with three-way
   per-file classification.
3. Lint engine with the codes listed in "Lint Codes".
4. `speccy status` (text + `--json schema_version: 1`).
5. `speccy next` (text + `--json schema_version: 1`); action kind
   derived from spec state.
6. `speccy check` (scenario rendering).
7. `speccy verify` (proof-shape validation; `--json schema_version: 1`).
8. `speccy lock` (record SPEC.md hash into TASKS.md frontmatter).
9. `speccy vacancy` (next free SPEC-NNNN; `--json schema_version: 1`).
10. Skill packs: shipped under `resources/modules/{personas,phases,skills,references}/`
    plus per-host MiniJinja wrappers under `resources/agents/`.
11. Dogfood: Speccy's own development tracked under `.speccy/specs/`,
    with every shipped CLI verb proven by its own SPEC.

Speccy dogfoods its own development. Every SPEC in this repo's
`.speccy/specs/` is the proof for the corresponding slice of the
binary; if a SPEC's `status` is `implemented`, the behaviour it
describes is what the binary does today.

---

# Success Criteria

Speccy v1 is complete enough when:

- A solo developer can run `speccy init` in a fresh repo and reach
  their first green check via the shipped skills without inventing
  process.
- The same developer can run `speccy init` in an existing repo at
  any point in its life and use Speccy productively on a small
  slice without reverse-engineering the whole codebase.
- An AI coding agent driven by the shipped skills can complete a
  full Plan -> Tasks -> Impl -> Review -> Report loop on a non-
  trivial spec without needing the human to chain commands manually.
- Reviewer personas catch at least one class of drift per review
  run on representative work (the proof here is the dogfooded
  Speccy itself).
- `speccy verify` is a reliable CI gate: passes when the proof
  shape is intact, fails when it isn't, never flakes on its own
  state.
- Speccy drives its own development. The repo contains
  `.speccy/specs/` for the implementation sequence above, with
  passing checks and review records.

Speccy v1 does not need to autonomously ship software. It needs
to make autonomous software construction less blind, and to make
the next project I (or anyone using it) build feel
qualitatively different from "ask the agent to do everything and
hope."

---

# Long-Term Vision

Speccy aims to become the **deterministic feedback substrate** that
multi-agent harnesses can build on. The in-pack implementation +
review orchestration loop now ships as part of the skill layer
(`/speccy-orchestrate` chained with `/speccy-holistic-gate`), so
single-host end-to-end execution is no longer a future layer. The
following remain future layers (not v1):

- Concurrent task pickup with file-locking or task queues
- Worktree orchestration per task
- Cross-spec dependency reasoning
- Project-level dashboard / kanban UI consuming `status --json`
- Production telemetry feedback into spec state
- Cross-repository orchestration

The foundation should remain unchanged across these layers:

> Explicit, inspectable, feedback-only contracts between intent
> and shipped behavior, with adversarial multi-persona review as
> the primary drift-detection mechanism.
