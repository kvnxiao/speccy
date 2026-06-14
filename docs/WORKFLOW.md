# Speccy Workflow and Harness

> The development loop and the skill layer that drives it: the five
> phases, the adversarial review fan-out, the amendment path, what the
> skill packs ship, per-host template variables, and the per-phase model
> pins.
>
> Part of the Speccy docs set: [ARCHITECTURE](./ARCHITECTURE.md) (design
> rationale) · [CLI](./CLI.md) (commands) · [SCHEMA](./SCHEMA.md) (file
> formats + lints) · WORKFLOW (loop + harness, this file).

The CLI surface that backs each phase is named where it matters (state
queries, hash recording, scenario rendering), but the prose body the
agent reads lives in a skill file under `.claude/skills/...` (or the
Codex twin), ejected at `speccy init` time. The CLI has no `plan` /
`tasks` / `implement` / `review` / `report` verbs; the phase recipes are
`/speccy-plan`, `/speccy-decompose`, `/speccy-work`, `/speccy-review`,
and `/speccy-ship` respectively.

---

## Workflow phases

The loop has five phases. Phases 3 and 4 are single-task primitives: one
invocation, one task, one state transition recorded in TASKS.md.
Composing those invocations into a batch is a caller concern, not the
skill's.

```text
1. plan       skill writes SPEC.md (PRD-shaped, XML-element-structured)
2. tasks      skill writes TASKS.md (one task sized for one agent session); skill calls `speccy lock`
3. implement  skill implements one task; exits with state transition
4. review     skill fans out the default reviewer personas on one task; exits with state transition
5. report     skill writes REPORT.md and opens PR
```

### Phase 1: Planning

The `/speccy-plan` skill (interactive, full-body SKILL.md) drives the
planning phase. The skill body instructs the agent to:

- read `AGENTS.md` (carries the project-wide product north star);
- read the nearest parent `MISSION.md` if writing into an existing focus
  area (the skill walks upward from the target spec path; absent
  MISSION.md is fine, the spec is ungrouped);
- call `speccy vacancy --json` to learn the next free `SPEC-NNNN` without
  globbing `.speccy/specs/` itself;
- propose the next SPEC slice;
- write `specs/[focus]/NNNN-slug/SPEC.md` when targeting a focus area,
  otherwise `specs/NNNN-slug/SPEC.md` (PRD-shaped; see the
  [SPEC.md template](./SCHEMA.md#specmd)), including `<requirement>` and
  nested `<scenario>` element blocks for IDs and check scenarios;
- surface material questions inline in SPEC.md.

On the new-SPEC path the skill then continues into Phase 2
(`/speccy-decompose`) in the same session rather than returning to the
user, so planning carries straight through to a decomposed task list and
its pre-loop checkpoint.

Mid-loop amendments use the parallel `/speccy-amend` skill (see
[Amendment](#amendment)).

### Phase 2: Task decomposition

The `/speccy-decompose` skill (pinned phase worker; thin SKILL.md stub
plus full body at `.claude/agents/speccy-decompose.md`) drives task
decomposition. The agent body instructs the agent to:

- read the SPEC.md (path supplied via `speccy next --json` or the
  user-supplied SPEC-NNNN argument);
- decompose into ordered tasks small enough for one sub-agent;
- group by phase if useful;
- reference REQ IDs each task covers via the `<task covers="...">`
  attribute;
- write `specs/NNNN-slug/TASKS.md`.

After the agent writes TASKS.md, the skill calls `speccy lock SPEC-001`,
which records the current SPEC.md sha256 + UTC timestamp into TASKS.md
frontmatter for staleness detection. It is the only verb that mutates a
TASKS.md frontmatter field; the skill calls it once after decomposition
lands. If TASKS.md already exists, decomposition runs as an amendment
under `/speccy-amend`, which preserves completed tasks, modifies or
removes invalidated tasks, and adds new ones for new requirements.

Decomposition exits at a pre-loop checkpoint. `SPEC.md` + `TASKS.md` are
the contract the implementation loop is measured against, and nothing
downstream re-checks the contract itself (review and vet only catch the
implementation drifting *from* it), so the skill stops and hands the
contract back for one review before `/speccy-orchestrate` (or
`/speccy-work`) begins.

### Phase 3: Implementation (single-task primitive)

The `/speccy-work` skill is a single-task primitive. One invocation
implements one task and exits with one state transition recorded in
TASKS.md. The skill ships as a thin SKILL.md stub plus a full agent body
at `.claude/agents/speccy-work.md` (pinned `model: opus[1m]`,
`effort: high`) and the Codex twin at `.codex/agents/speccy-work.toml`.

With an optional `[SPEC-NNNN/T-NNN]` selector the session implements that
specific task. Without an argument the session calls `speccy next --json`
and filters for `next_action.kind == "work"` to resolve the next
implementable task (the selector is unknown until then, so `speccy next`
still precedes `context` on this path). In either case the session:

- opens its per-task context with one `speccy context SPEC-NNNN/T-NNN
  --json` call: the bundle carries the task entry, covering requirements
  with scenarios, the latest journal round in full plus an
  attributes-only index of prior rounds, the sibling index for the reuse
  survey, and the suggested diff command; it replaces the former recipe
  of reading full SPEC.md, full TASKS.md, and the journal and invoking
  `speccy check` for scenarios;
- flips `state="pending"` to `state="in-progress"` on the target task via
  `speccy task transition`;
- writes tests first, then code; runs the project's own test command
  locally and fails fast on red;
- appends one `<implementer>` block to
  `.speccy/specs/NNNN-slug/journal/T-NNN.md` via
  `speccy journal append --block implementer`, piping the multi-field
  handoff body on stdin (the CLI stamps `date` and derives `round`,
  creating the journal file on round 1 and appending on subsequent
  rounds);
- flips `state="in-progress"` to `state="in-review"` via
  `speccy task transition` and exits.

The session does not pick up another task on its way out. If two
implementers run in parallel against different `state="pending"` tasks
and touch the same files, they conflict in git; Speccy does not lock.

### Phase 4: Review (single-task primitive)

The `/speccy-review` skill is a single-task primitive. One invocation
runs one round of adversarial review on one task and exits with one state
transition recorded in TASKS.md via `speccy task transition`. The
orchestrator stays in the parent session (no agent file) because it owns
the consolidated verdict and the single `<blockers>` directive, and needs
the parent session's full capacity to fan out, parse the reviewers' thin
verdict returns, and decide the state flip atomically. Write
serialization to the journal is the CLI append lock's job, not the
orchestrator's. Each reviewer self-appends its own `<review>` block.

With an optional `[SPEC-NNNN/T-NNN]` selector the session reviews that
specific task. Without an argument the session calls `speccy next --json`
and filters for `next_action.kind == "review"`. In either case the
session:

- fans out one reviewer sub-agent per persona in the default fan-out
  (`business`, `tests`, `security`, `style`, `correctness`) in parallel
  within this single task; each sub-agent's body is loaded from
  `.claude/agents/reviewer-<persona>.md` or its Codex parallel, with
  per-persona model pins (see [Model pinning](#model-pinning)). Each
  persona opens its per-task read with one `speccy context SPEC-NNNN/T-NNN
  --json` call (dispatched from the shared fan-out spawn prompt), not a
  full SPEC.md / TASKS.md read or a `speccy check` entry call: the
  bundle hands it the task, its requirements and scenarios, the latest
  journal round in full (with the prior rounds indexed by attributes),
  and the suggested diff command in a single roundtrip. (`reviewer-tests`
  keeps its separate caveat that `speccy check` exit codes are not test
  evidence. That is unrelated to the entry read.) Vet personas are
  excluded: their review is whole-SPEC holistic scope, which a
  task-scoped bundle cannot serve, so they keep their full reads;
- has each reviewer sub-agent append its own `<review>` block to
  `.speccy/specs/NNNN-slug/journal/T-NNN.md` via
  `speccy journal append --block review` and return a thin verdict
  (persona, verdict, one-line rationale); the CLI's per-file append lock
  serializes the concurrent appends, so no single session has to be the
  journal's sole writer;
- reads the round's verdicts back via `speccy journal show --block review
  --round latest` and flips `state="in-review"` to `state="completed"` if
  every persona `<review>` carries `verdict="pass"`; otherwise flips
  `state="in-review"` to `state="pending"` via `speccy task transition`
  and appends one orchestrator-authored `<blockers>` block via
  `speccy journal append --block blockers` summarising the blockers, and
  exits.

The within-task fan-out is intrinsic to the primitive, not
orchestration: adversarial diversity requires fresh contexts per persona,
and the fan-out is bounded to one sub-agent per default persona on one
task in one round. Failed tasks return to `state="pending"` for a later
Phase 3 invocation to pick up.

The default fan-out is **business**, **tests**, **security**, **style**,
**correctness**. The other personas (**architecture**, **docs**) are
available by user request but not in the default set.

### Phase 5: Report and PR

When `speccy next` returns no actionable task across the workspace, the
loop is complete. The `/speccy-ship` skill (pinned phase worker at
`.claude/agents/speccy-ship.md`) instructs the agent to write `REPORT.md`
summarising:

- requirements satisfied;
- tasks completed (with retry counts derived from journal rounds);
- out-of-scope items absorbed;
- deferred or known limitations;
- check results summary.

REPORT.md is shaped by raw XML element tags (see
[SCHEMA.md → REPORT.md](./SCHEMA.md#reportmd) for the grammar and the
`RPT-*` lint family). The skill then opens a PR via `gh` (or equivalent);
Speccy itself never touches GitHub.

---

## Review

Review is an adversarial proof challenge. The CLI renders prompts; the
skill layer orchestrates multiple reviewer personas in parallel.

### Personas

Speccy ships with these personas (markdown skill files):

| Persona | Catches |
|---|---|
| `business` | Does the implementation match SPEC.md intent? Are user stories satisfied? Are non-goals respected? |
| `tests` | Are checks meaningful or vacuous? Edge cases covered? Are negative cases asserted? Is the test exercising the actual behavior, or testing the mock? |
| `security` | Auth, input validation, secrets, injection, sensitive data exposure, access control |
| `style` | Project conventions per `AGENTS.md`, lint compliance, naming, dead code |
| `correctness` | Logic and control-flow errors, `Option`/`Result` mishandling, off-by-one and boundary conditions, non-security races and deadlocks, resource leaks |
| `architecture` | Cross-spec invariants, design adherence, layering, premature abstraction, ADR drift |
| `docs` | Comments, READMEs, inline SPEC.md decisions and AGENTS.md updated to match the change |

The default fan-out (run when the skill does a full review) is
`business, tests, security, style, correctness`. Architecture and docs
are off-by-default: the `/speccy-review` skill runs them only when
explicitly asked. Persona selection is a skill-layer concern; there is
no CLI flag for it. A future change may make the fan-out
project-configurable; v1 does not.

### Invocation

The CLI has no `review` verb. Review runs through the `/speccy-review`
skill (Phase 4 primitive). The skill resolves the target task (either via
an explicit `SPEC-NNNN/T-NNN` selector or via `speccy next --json`
filtered for `next_action.kind == "review"`) and fans out one reviewer
sub-agent per persona in the default fan-out.

Each persona sub-agent is loaded from its host-native agent file
(`.claude/agents/reviewer-<persona>.md` or its Codex parallel,
materialised from `resources/modules/personas/reviewer-<persona>.md` by
`speccy init`). The shipped persona body composes a prompt that includes:

- the relevant SPEC.md (full, including its `### Decisions` block)
- the task body from TASKS.md (the bare `<task>` element)
- prior implementer / reviewer / blocker history from
  `.speccy/specs/NNNN-slug/journal/T-NNN.md`
- the diff for the task's claimed work
- `AGENTS.md`
- the persona's review-style guidance from the shipped persona body

The reviewer sub-agent reads the prompt, performs the review, and appends
its own `<review>` block to `.speccy/specs/NNNN-slug/journal/T-NNN.md` via
`speccy journal append --block review --persona <self> --verdict <v>`
(findings on stdin), then returns a thin verdict (persona, verdict,
one-line rationale) to the orchestrator. The CLI's per-file append lock
serializes the parallel appends, so the journal stays well-formed without
any one session being its sole writer. Reviewer sub-agents never write to
TASKS.md and never author `date` / `round` themselves; `TSK-006` rejects
`<review>` elements inside `<task>` bodies, and the CLI stamps the
environment-derivable attributes.

### State transitions

Persona sub-agents **do not** flip the task's `state` attribute. That
would create a race when the personas run in parallel. The
`/speccy-review` skill flips state once via `speccy task transition` after
reading every persona's verdict back through
`speccy journal show --block review --round latest`:

- All `verdict="pass"` → `task transition --to completed`.
- Any `verdict="blocking"` → `task transition --to pending`, and the
  orchestrator appends one `<blockers>` block via `speccy journal append
  --block blockers` summarising the blocking findings (the block body
  stays orchestrator-authored semantic judgment; the CLI derives `round`,
  monotonically increasing across rounds).

This puts state-mutation atomicity in one place (the orchestrator session
decides the single flip) and keeps the journal the single source of truth
for review history.

### Why personas live in skills, not CLI

The CLI cannot know what "security" means in this project. The skill
prompt does. By making personas markdown skill files, three things become
possible:

1. Add a new persona without changing the CLI.
2. Swap persona definitions when models improve.
3. Projects edit the host-native sub-agent file in place
   (`.claude/agents/reviewer-security.md`,
   `.codex/agents/reviewer-security.toml`). The host-native location is
   the only persona surface. Edits are preserved via git, not via a
   CLI-side carve-out. Commit before running `speccy init --force`,
   since the uniform classification overwrites any file that differs from
   the shipped bundle.

---

## Amendment

Amendments are not a separate first-class artifact in v1. The amendment
story is a skill concern built from existing CLI primitives. There is no
`speccy amend` verb; the existing flat CLI surface is sufficient.

The `/speccy-amend SPEC-001` skill orchestrates the mid-loop change in the
parent session:

1. The skill reads `speccy status SPEC-001 --json` to learn
   `spec_md_path`, `tasks_md_path`, and the spec's current `next_action`.
2. The agent edits `SPEC.md` surgically: preserve what works, minimal
   diff, append a `## Changelog` row recording the why.
3. The agent edits `TASKS.md` surgically:
   - keep `state="completed"` tasks unless invalidated by the SPEC
     change;
   - keep `state="in-progress"` / `state="in-review"` tasks unless
     invalidated;
   - flip invalidated `state="completed"` tasks back to `state="pending"`
     with a "spec amended" note;
   - add new `<task>` elements for new requirements;
   - remove tasks for dropped requirements.
4. The skill calls `speccy lock SPEC-001` to record the new
   `spec_hash_at_generation` + `generated_at` into TASKS.md frontmatter so
   subsequent staleness checks pass.

The skill body ships as host skill content under
`.claude/skills/speccy-amend/SKILL.md` and the Codex twin. There is no
CLI-embedded prompt template for amendment; the recipe lives entirely in
the skill file.

Speccy does not maintain an amendment registry. Two mechanisms cover the
lineage need: the `## Changelog` table in SPEC.md (curated, prose-
summarized history of material edits, loaded into review and amendment
prompts) and git history (the authoritative literal lineage via
`git log SPEC.md` / `git log TASKS.md`).

---

## Skills / harness layer

Speccy v1 ships official skill packs alongside the CLI. They are not
optional polish; they are how the system becomes usable end-to-end
without each project inventing its own integration.

### What ships

```text
resources/
  modules/
    skills/                  Interactive skill bodies (speccy-brainstorm,
                             -plan, -amend, -review, -orchestrate, -vet)
                             plus SKILL.md bodies for the pinned workers
                             that defer to the agent file.
                             `partials/` holds sharable skill fragments
                             included from multiple skill bodies.
    phases/                  Agent bodies for the pinned workers and init
                             (speccy-decompose, -work, -ship, -init).
    personas/                Reviewer persona bodies (`reviewer-*.md`),
                             vet persona bodies (`vet-*.md`), plan
                             persona bodies (`plan-*.md`), and
                             topic-named snippets included from those
                             bodies. The snippet/body distinction is
                             carried by filename pattern; no
                             `_partials/` subdirectory exists.
    references/              Canonical reference files. Skill-local
                             refs eject into each skill's `references/`
                             subdirectory; host-shared refs eject under
                             `<host>/speccy-references/`. The full
                             mapping lives in "Skill-pack reference
                             files" below.
  agents/                    Per-host MiniJinja wrappers (rendered at init time)
    .claude/skills/speccy-<verb>/SKILL.md.tmpl
    .claude/agents/speccy-{decompose,work,ship}.md.tmpl
    .claude/agents/reviewer-<persona>.md.tmpl
    .claude/agents/vet-{reviewer,implementer,simplifier}.md.tmpl
    .claude/agents/plan-{explorer,architect}.md.tmpl
    .agents/skills/speccy-<verb>/SKILL.md.tmpl
    .codex/agents/speccy-{decompose,work,ship}.toml.tmpl
    .codex/agents/reviewer-<persona>.toml.tmpl
    .codex/agents/vet-{reviewer,implementer,simplifier}.toml.tmpl
    .codex/agents/plan-{explorer,architect}.toml.tmpl
```

There is no `resources/modules/prompts/` directory and no embedded phase
prompt body inside the CLI binary. The CLI ships `resources/modules/` as
the single source of truth for skill content and `resources/agents/` as
the per-host MiniJinja wrappers; nothing else.

For Claude Code, init lands `.claude/skills/speccy-<verb>/SKILL.md` for
every shipped skill (full body for the interactive skills; defer-to-agent
SKILL.md for the pinned phase workers), plus
`.claude/agents/speccy-{decompose,work,ship}.md` for the pinned
phase-worker bodies, `.claude/agents/reviewer-<persona>.md` for the
reviewer sub-agents,
`.claude/agents/vet-{reviewer,implementer,simplifier}.md` for the vet
sub-agents that `/speccy-vet` drives, and
`.claude/agents/plan-{explorer,architect}.md` for the read-only plan-time
grounding sub-agents. The Codex parallel is
`.agents/skills/speccy-<verb>/SKILL.md` plus
`.codex/agents/speccy-{decompose,work,ship}.toml`,
`.codex/agents/reviewer-<persona>.toml`,
`.codex/agents/vet-{reviewer,implementer,simplifier}.toml`, and
`.codex/agents/plan-{explorer,architect}.toml`.

Skills that stay in the parent session (no agent file) are the ones that
either need interactive user prompts or own serialised writes to TASKS.md
/ the journal: `speccy-bootstrap`, `speccy-review`, `speccy-orchestrate`, and
`speccy-vet` fall into this bucket.

### Per-host template variables

Wrappers under `resources/agents/.<host>/` are MiniJinja templates
rendered at `speccy init` time. A render draws on two variable surfaces:
the per-host `TemplateContext`, and template-local `{% set %}` bindings
declared at the top of individual module bodies. The environment renders
with strict-undefined behaviour, so referencing a variable that is
neither in `TemplateContext` nor bound by a `{% set %}` before its first
use is a hard render error; add the binding before the reference.

| Variable | Source | Claude Code | Codex |
|---|---|---|---|
| `host` | `TemplateContext` | `claude-code` | `codex` |
| `cmd_prefix` | `TemplateContext` | `/` | _(empty)_ |
| `host_display_name` | `TemplateContext` | `Claude Code` | `Codex` |
| `skill_install_path` | `TemplateContext` | `.claude/skills` | `.agents/skills` |
| `speccy_references_path` | `TemplateContext` | `.claude/speccy-references` | `.agents/speccy-references` |
| `persona_name` | `{% set %}` | line 1 of each `resources/modules/personas/reviewer-*.md` | _(same source)_ |
| `task_kind` / `task_adjective` | `{% set %}` | `phases/speccy-work.md`, `skills/speccy-review.md` | _(same source)_ |

`host_display_name` is currently unused: no wrapper or module under
`resources/` references it. It is retained in `TemplateContext` as a ready
binding for a future wrapper; an unused context field is harmless under
strict-undefined (the mode rejects undefined *references*, not unused
*bindings*).

### Skill-pack reference files

Each lintable Speccy artifact has exactly one canonical reference file
under `resources/modules/references/`, ejected by `speccy init` into
either a per-skill `references/` subdirectory (single-consumer) or a
host-shared `speccy-references/` directory at host root (multi-consumer).
The path's first component classifies the file; no separate manifest
declares it. The mapping:

| Artifact | Source | Claude Code path | Codex path | Class |
|---|---|---|---|---|
| SPEC.md | `resources/modules/references/spec.md` | `.claude/skills/speccy-plan/references/spec.md` | `.agents/skills/speccy-plan/references/spec.md` | skill-local |
| TASKS.md | `resources/modules/references/tasks.md` | `.claude/skills/speccy-decompose/references/tasks.md` | `.agents/skills/speccy-decompose/references/tasks.md` | skill-local |
| REPORT.md | `resources/modules/references/report.md` | `.claude/skills/speccy-ship/references/report.md` | `.agents/skills/speccy-ship/references/report.md` | skill-local |
| PR body template | `resources/modules/references/pr-body.md` | `.claude/skills/speccy-ship/references/pr-body.md` | `.agents/skills/speccy-ship/references/pr-body.md` | skill-local |
| journal `<implementer>` block | `resources/modules/references/journal-implementer.md` | `.claude/speccy-references/journal-implementer.md` | `.agents/speccy-references/journal-implementer.md` | host-shared |
| journal `<review>` block | `resources/modules/references/journal-review.md` | `.claude/speccy-references/journal-review.md` | `.agents/speccy-references/journal-review.md` | host-shared |
| evidence file (`evidence/T-NNN.md`) | `resources/modules/references/evidence.md` | `.claude/speccy-references/evidence.md` | `.agents/speccy-references/evidence.md` | host-shared |
| journal `<blockers>` block | `resources/modules/references/journal-blockers.md` | `.claude/speccy-references/journal-blockers.md` | `.agents/speccy-references/journal-blockers.md` | host-shared |
| reconcile policy table | `resources/modules/references/reconcile-policy.md` | `.claude/speccy-references/reconcile-policy.md` | `.agents/speccy-references/reconcile-policy.md` | host-shared |
| retry-shape invariant | `resources/modules/references/retry-shape.md` | `.claude/speccy-references/retry-shape.md` | `.agents/speccy-references/retry-shape.md` | host-shared |

Host-shared rows are referenced from multiple consuming bodies and live
at `<host>/speccy-references/<file>.md` so each consumer imports the same
canonical text. `evidence.md` is referenced by `/speccy-work` (writes
evidence) and the `reviewer-tests` sub-agent (reads evidence);
`journal-blockers.md` by `/speccy-review` and `/speccy-amend`;
`reconcile-policy.md` by `/speccy-work`, `/speccy-review`, and
`/speccy-orchestrate` (all of which dispatch on `consistency.drifts[]`);
`retry-shape.md` by `/speccy-work` and reviewer sub-agents;
`journal-implementer.md` by the `speccy-work` agent body and by
`evidence.md`; `journal-review.md` by the review fan-out partial, which
lands in both `/speccy-review` and `/speccy-orchestrate`. Skill-local rows
have exactly one consuming body each (or two in the case of `pr-body.md`
and `report.md`, both consumed by `/speccy-ship`). Agent bodies eject as
flat files with no sibling `references/` directory, so a skill-local file
consumed from an agent body is pointed at via its host-rooted skill path
(`<skill_install_path>/<skill>/references/<file>.md`), never a bare
`references/…` relative path.

A discovery test asserts every ejected reference file is reached by ≥1
path-substring pointer from a consuming body, and that source-to-host and
cross-host bytes match.

### Persona file resolution

Host-native sub-agent files are the sole canonical persona surface. There
is no project-local override directory. Persona bodies live at
`resources/modules/personas/reviewer-X.md` inside the CLI binary and are
rendered into the host pack at `speccy init` time:

- Claude Code: `.claude/agents/reviewer-<persona>.md`
- Codex: `.codex/agents/reviewer-<persona>.toml`

The host loads that file as the sub-agent's system context when
`speccy-review` (or `speccy-vet`) spawns it. To customise a persona, edit
the rendered file in place; the renderer treats it as a user-owned file
thereafter and will not overwrite it without `--force`.

### Workflow recipes

Each top-level skill is a recipe. Interactive skills eject as a full-body
SKILL.md only. Pinned phase workers (`speccy-decompose`, `speccy-work`,
`speccy-ship`) eject as a SKILL.md body that names the matching agent file
as the canonical procedure source plus a full-body agent file at
`.claude/agents/speccy-<phase>.md` (Codex:
`.codex/agents/speccy-<phase>.toml`). The SKILL.md bodies for pinned
workers are deliberately small, they defer to the agent file, but are
not fixed-line stubs; they may inline entry preconditions or policy
references that callers need to see without reading the agent body.

- `/speccy-bootstrap`: bootstrap the project (interactive)
- `/speccy-brainstorm`: atomize a fuzzy ask before drafting a SPEC
- `/speccy-plan`: Phase 1 (AGENTS.md north star + optional MISSION.md → SPEC)
- `/speccy-decompose`: Phase 2 (SPEC → TASKS, then `speccy lock`)
- `/speccy-work`: Phase 3 (implement one task)
- `/speccy-review`: Phase 4 (review one task; orchestrator)
- `/speccy-amend`: Mid-loop SPEC + TASKS reconciliation
- `/speccy-ship`: Phase 5 (REPORT.md + PR)
- `/speccy-orchestrate`: Chains `/speccy-work` + `/speccy-review` across every task in one SPEC, hands off to `/speccy-vet` before the ship boundary
- `/speccy-vet`: Pre-ship SPEC-vs-implementation drift review with an autonomous fix-retry loop; invoked by the orchestrator and runnable on its own

A typical full session in Claude Code looks like:

```text
/speccy-plan
[agent writes SPEC.md, then carries through /speccy-decompose on its
 own: writes TASKS.md, runs `speccy lock SPEC-001`, and stops at the
 pre-loop checkpoint surfacing SPEC + TASKS for one review]

[review the contract; proceed to the loop, or /speccy-amend to revise]

/speccy-work SPEC-001/T-001
[agent implements one task, flips state="pending" -> state="in-review", exits]

/speccy-review SPEC-001/T-001
[orchestrator fans out the default reviewer personas on one task,
 aggregates notes, flips state="in-review" -> state="completed"
 (or back to "pending" with a Retry note), exits]

[caller re-invokes /speccy-work and /speccy-review on the remaining
 tasks, or runs /speccy-orchestrate to drive the whole loop]

/speccy-ship SPEC-001
[agent writes REPORT.md, opens PR]
```

The CLI is invoked many times during this; the skill knows when.

### Persona definitions

Each persona file is a markdown skill describing the role (one
paragraph), review focus areas (bullet list), what to look for that is
easy to miss, the format of the inline note to append, and a worked
example. Example skeleton for `reviewer-security.md`:

```markdown
# Reviewer Persona: Security

## Role
You are an adversarial security reviewer for one task in one spec.
You read the SPEC.md, the task's diff, and the prior journal
entries in `.speccy/specs/NNNN-slug/journal/T-NNN.md`. You append
your own `<review>` block via `speccy journal append` and return a
thin verdict (persona, verdict, one-line rationale) to the
orchestrator.

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
Append your `<review>` block with `speccy journal append SPEC-NNNN/T-NNN
--block review --persona security --verdict <pass|blocking> --model
<model-id>[/effort]`, piping the one-paragraph summary (file:line refs
encouraged) on stdin. The CLI stamps `date` and `round` and writes the
block in the shape:

    <review persona="security" verdict="pass | blocking"
            date="<ISO8601>" model="<model-id>[/effort]" round="<N>">
    <one-paragraph summary; file:line refs encouraged>
    </review>

You supply only `--persona`, `--verdict`, and `--model`; you never
author `date` or `round`. After the append succeeds, return a thin
verdict line to the orchestrator (persona, verdict, one-line
rationale) so it can decide the state flip.

## Example

    <review persona="security" verdict="blocking"
            date="2026-05-21T19:00:00Z"
            model="claude-opus-4-8[1m]/high" round="1">
    bcrypt cost 10; policy requires >=12.
    See `src/auth/password.ts:14`.
    </review>
```

These files are the durable surface where review intelligence lives. They
are upgradeable as models improve; the CLI is not.

---

## Model pinning

Speccy's shipped skill packs pin specific model and effort tiers for each
phase of the development loop. The pin assignment is asymmetric and
reflects the work-shape of each role: mechanical phases pin a mid-tier
model so they run cheaply; adversarial reviewers pin a higher tier so they
catch real drift. Interactive / orchestrator skills (`/speccy-bootstrap`,
`/speccy-brainstorm`, `/speccy-plan`, `/speccy-amend`, `/speccy-review`,
`/speccy-orchestrate`, `/speccy-vet`) stay unpinned and inherit whatever
model the parent session is using.

### Pin assignment

| Phase / persona | Claude Code (`.claude/agents/...md`) | Codex (`.codex/agents/...toml`) | Agent file ships? |
|---|---|---|---|
| `speccy-decompose` | `model: opus[1m]`, `effort: medium` | `model = "gpt-5.5"`, reasoning effort medium | yes |
| `speccy-work` | `model: opus[1m]`, `effort: high` | `model = "gpt-5.5"`, reasoning effort medium | yes |
| `speccy-ship` | `model: sonnet[1m]`, `effort: medium` | `model = "gpt-5.5"`, reasoning effort medium | yes |
| `speccy-bootstrap` | unpinned, inherits session | unpinned, inherits session | no |
| `speccy-review` | unpinned, inherits session | unpinned, inherits session | no |
| `reviewer-business` | `model: opus[1m]`, `effort: xhigh` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `reviewer-tests` | `model: opus[1m]`, `effort: xhigh` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `reviewer-architecture` | `model: opus[1m]`, `effort: xhigh` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `reviewer-security` | `model: opus[1m]`, `effort: high` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `reviewer-style` | `model: sonnet[1m]`, `effort: medium` | `model = "gpt-5.5"`, reasoning effort low | yes |
| `reviewer-correctness` | `model: opus[1m]`, `effort: high` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `reviewer-docs` | `model: sonnet[1m]`, `effort: medium` | `model = "gpt-5.5"`, reasoning effort low | yes |
| `vet-reviewer` | `model: opus[1m]`, `effort: xhigh` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `vet-implementer` | `model: opus[1m]`, `effort: high` | `model = "gpt-5.5"`, reasoning effort low | yes |
| `vet-simplifier` | `model: opus[1m]`, `effort: medium` | `model = "gpt-5.5"`, reasoning effort low | yes |
| `plan-explorer` | `model: opus[1m]`, `effort: high` | `model = "gpt-5.5"`, reasoning effort high | yes |
| `plan-architect` | `model: opus[1m]`, `effort: high` | `model = "gpt-5.5"`, reasoning effort high | yes |

The `[1m]` suffix selects the 1M-context variant on Claude Code so each
agent can read the full SPEC, the full diff, and the relevant TASKS.md
slice without truncation. On Codex, `gpt-5.5` covers every pinned role and
the asymmetric work-shape is carried entirely by `model_reasoning_effort`.

The pinned phase workers (`speccy-decompose`, `speccy-work`,
`speccy-ship`) ship sub-agent files. `speccy-bootstrap` and `speccy-review`
ship no agent file on either host, only the SKILL.md surface, because
both phases need to drive the parent session directly (interactive Q&A in
init's case; serial TASKS.md writes in review's case).

The `/speccy-review` orchestrator stays unpinned on both hosts
deliberately: it is the sole writer to `TASKS.md` during the review loop
(reviewer sub-agents return their verdicts and the orchestrator serializes
the state transition), and it needs the parent session's full capacity to
fan out, parse return messages, and consolidate verdicts without dropping
state. Pinning it to a sub-agent would either force a serial-write race or
strand the verdict-consolidation logic in a context that does not own
`TASKS.md`.

### Activating a pin (opt-in)

The pin lives in the agent file, not the slash-command surface. Typing
`/speccy-work` runs the workflow in the **parent session at the parent
session's model**; the agent file is ignored. To activate the pin, invoke
the sub-agent explicitly before running the phase:

- **Claude Code:** `/agent speccy-work` (or use the host's sub-agent
  spawning tool), then run `/speccy-work`.
- **Codex:** invoke the equivalent sub-agent spawner against
  `.codex/agents/speccy-work.toml`, then run `/speccy-work`.

For the pinned phases, the SKILL.md body defers to the matching agent file
as the canonical procedure source. The agent file's body is the single
on-disk source of truth for that phase. `/speccy-bootstrap`'s SKILL.md and
`/speccy-review`'s SKILL.md both remain full-body because there is no
sub-agent file for either to defer to.

The opt-in `/agent` surface is deliberate. An earlier draft auto-forked
the mechanical phases into pinned sub-agents, which hid the sub-agent's
tool output from the parent session and produced minutes of dead air in
the parent UI on multi-minute phase work. The opt-in surface preserves the
cost-and-time pin without that silent-by-design UX cost.

### Overriding a pin

The shipped pins are defaults, not policy. To swap models or remove a pin
entirely, edit the agent file's YAML or TOML frontmatter under
`.claude/agents/` or `.codex/agents/` and commit the change. Examples:

- Lock `speccy-work` to a specific Claude version for reproducibility:
  change `model: opus[1m]` to `model: claude-opus-4-8[1m]` in
  `.claude/agents/speccy-work.md`.
- Run a reviewer at a lighter tier in a cost-sensitive repo: change
  `model: opus[1m]` to `model: sonnet[1m]` (and adjust `effort:`
  accordingly) in `.claude/agents/reviewer-business.md`.
- Remove the pin entirely so the sub-agent inherits the parent session's
  model: delete the `model:` and `effort:` lines.

Pins use **aliases** (`sonnet[1m]`, `opus[1m]`, `gpt-5.5`) rather than
long-form versioned snapshot IDs by default so they float forward as
vendors ship newer generations of each tier. Users who want byte-stable
reproducibility across a release boundary can lock to a specific version
by editing the alias to a long-form ID in the ejected file.
