# Design: Spec-Driven Run Controller

Status: authoritative
Date: 2026-08-30

Payload shapes and vocabulary stay with this design until implementation requires separate homes.

Working name: `speccy`.

## Product Thesis

Speccy is a small run controller for coding-agent harnesses. It turns an engineering request into a tree — a spec, its milestones, and small fully specified tasks — then lets the harness implement each task in a fresh context and judge the result against written criteria, halting for a human only at the charter, the contract, the ship, and any point where the loop cannot make progress.

Speccy writes no code and calls no model. The harness calls Speccy; Speccy owns the tree, the sequencing, and the record of what was judged.

The promise:

> Spend attention on the charter and the contract, not on watching implementation. Receive a compact, judged record of what changed and what was accepted on judgment alone.

## Design Principles

1. **Harness-neutral.** Claude Code and Codex are the targets. The controller exposes one CLI; the packs are plain harness-native files.
2. **No outbound agent runner.** No `speccy` command calls an LLM, a coding agent, or a harness.
3. **Zero product-code footprint.** Speccy does not modify product artifacts. Runtime state lives under `.speccy/`, gitignored by default. Rendered packs are the only committed artifacts.
4. **Decompose to ready tasks.** Work is planned down to tasks a fresh session on the weakest configured model can finish from the packet alone. A task that exceeds this boundary splits into sibling tasks; it does not gain children.
5. **Verification is judgment, not proof.** Fresh-context judges decide whether criteria are met. Speccy runs no deterministic checks of its own and says so at ship. Dogfooding must identify a failure class before deterministic tooling is added.
6. **Single writer, scoped readers.** Only the `speccy` binary writes the tree. Agents read scoped packets, never storage.
7. **Deterministic core, prose policy.** Sequencing, state, ids, lint, and the packet format are code. Planning, implementing, and judging are prose the harness executes.
8. **Human attention at the edges.** Three gates and bounded halts; no steering during implementation.
9. **Less is more.** One node schema, one loop, one directive vocabulary. Each mechanism in this document names the failure it prevents.

## Non-Goals

- Replacing coding agents or harnesses, or shipping an IDE or web UI.
- Deterministic evidence collection, command execution, or provenance scanning inside Speccy (deferred, see "Later Capabilities").
- Multi-day autonomous missions; a spec is expected to complete in one sitting to a few hours of loop time.
- Parallel writers. One task is worked at a time; read-only work inside an agent may fan out.
- Maintaining a current-truth description of the system. Code, README, and AGENTS.md are current truth; prior specs are context.
- A constitution or policy engine. Cross-cutting rules live in AGENTS.md and the repository's own rule files.
- Time boxes, cycles, or estimates. Work is bounded by criteria and caps, not calendar.
- Persisting run state in the repository by default, or requiring that it be committed.

## SDLC Invariants

The tree maps the shared layers of development methods to one record structure and drops process that does not support those layers.

| Invariant | Waterfall / RUP | Agile / Scrum | Shape Up | BDD / spec-driven tools | Speccy |
| --- | --- | --- | --- | --- | --- |
| **Why** — the problem and value that outlive any solution | Vision, charter | Product vision, theme | Problem + appetite | Constitution, project context | Project charter |
| **What** — observable behavior, independent of how | Requirements baseline | Story + acceptance criteria | Pitch | Requirements, WHEN/THEN scenarios | Criteria on every node |
| **How** — decisions that constrain implementation | Design doc, ADRs | Spike, tech notes | Breadboard | `design.md` | Rationale on spec and milestone; task plan |
| **Work units** — finishable, ordered pieces | WBS | Backlog task | Scope | `tasks.md` | Task |
| **Change** — the reviewable diff that leaves the team | Change request | PR | Ship the bet | Apply, archive | Spec (the ship boundary) |
| **Independent check** against What | V&V, QA | Definition of done, review | QA in cycle | Verify phase | Fresh-context judges |
| **Memory** — durable record of decisions and events | Decision log | Retro, changelog | Hill-chart history | Archive, deltas | Decision log, journal |
| **Traceability** — each level points to what justifies it | Traceability matrix | Story → epic → theme | Scope → pitch | Task → requirement | Computed by judging each node against its parent |

The charter has its own gate because later review checks drift from the spec, not whether the spec expresses the right problem. The durable project record and the bounded spec are separate record types because a requirements baseline and a change request serve different purposes; OpenSpec uses the same split with `specs/` and `changes/`.

### Issue-tracker mapping

Speccy maps its records to shapes familiar from issue trackers.

| Invariant | Jira | Linear | GitHub | Symphony | Speccy |
| --- | --- | --- | --- | --- | --- |
| Why | Initiative | Initiative | — | — | Project |
| Outcome container | Epic | Project | Milestone / parent issue | — | Spec |
| Ordered sub-goal | — | Milestone | Parent issue | — | Milestone |
| Unit of work | Story, Task | Issue | Issue | Issue (one workspace each) | Task |
| Checklist | Sub-task | Sub-issue | Sub-issue | — | Plan steps |
| Time / release box | Sprint, Version | Cycle | Milestone due date | — | None |
| Workflow state | Status | Status | Open / closed | Tracker state drives dispatch | `status` field |
| Policy versioned with code | — | — | — | `WORKFLOW.md` | Packs, AGENTS.md |

"Milestone" uses Linear's meaning — an ordered stage of completion inside a project — rather than Jira's or GitHub's release box. Speccy treats the task as its dispatch unit and the plan beneath it as a checklist.

## Core Concepts

### Records

Two record types define the tree. A `Project` is durable and never completes. A bounded `Node` has one of three roles: the root of a spec file is the **spec**, its children are **milestones** or **tasks**, and a milestone's children are **tasks**. A task has a `plan`; a milestone has `tasks`. Node depth is fixed at three levels below the project. A task that exceeds its boundary splits into sibling tasks; it does not gain children.

```text
Project  (.speccy/project.yaml)          durable: charter, decision log
└── Spec (.speccy/specs/spec-N.yaml)     ships once; the change boundary
    ├── Milestone                        ordered stage; expanded lazily into tasks
    │   └── Task                         the unit a worker implements
    └── Task                             a spec may hold tasks directly
```

### Project

```yaml
# .speccy/project.yaml
status: approved                # draft | approved
purpose: |                      # markdown
users: |
non_goals: |
constraints: AGENTS.md          # where cross-cutting rules live; Speccy never restates them
decisions:
  - id: decision-3
    at: 2026-09-02
    spec: spec-2
    text: "Magic-link tokens are stored hashed."
```

The charter is the "why" every spec is judged against. The decision log is the only cross-spec memory: entries are proposed by the spec judge at ship, shown on the ship card, and appended only with human approval. The spec index is derived from `specs/*.yaml`, never stored.

### Spec, milestone, task

```yaml
# .speccy/specs/spec-2.yaml
id: spec-2
title: Passwordless login
status: approved
intent: |                       # traced to the charter
scope:
  in: |
  out: |
risk: |                         # free text; no tier vocabulary
criteria:
  - { id: criterion-1, text: "A link can be requested by email." }
  - { id: criterion-2, text: "A link is single-use." }
  - { id: criterion-3, text: "An expired link is rejected and creates no session." }
rationale: |                    # how the milestones and tasks partition the intent
milestones:
  - id: milestone-1
    title: Token model and endpoints
    status: working
    intent: |
    criteria: [ { id: criterion-1, text: "…" } ]
    rationale: |
    tasks:
      - id: task-1
        title: Token model with expiry
        status: verified
        intent: |
        criteria: [ { id: criterion-1, text: "…" } ]
        plan: |                 # numbered steps, markdown
        touches: [ src/server/auth/ ]
        read_first: [ src/server/db/schema.rs, docs/adr/0004-sessions.md ]
        base: 3f9c1e2           # HEAD when work was dispatched
        commits: [ a7f4c2e ]    # recorded at handoff
        round: 1
        journal:
          - { at: 2026-09-01T10:02:00Z, agent: worker, kind: handoff, body: | … }
        verdicts:
          - { at: 2026-09-01T10:40:00Z, agent: fidelity-judge, criterion: criterion-1, result: pass, reason: "…" }
        findings:
          - { at: 2026-09-01T10:41:00Z, agent: defects-judge, severity: advisory, text: "…" }
tasks: []                       # tasks directly under the spec
ship: null                      # { at, range, disclosure, mode } once shipped
```

Field rules:

- **Criteria** are the definition of done for the node that carries them. Each is checkable from the result alone. A task carries at most a handful; the lint warns past `lint.max_criteria`.
- **Plan** is the implementation plan in numbered markdown steps. **`touches`** names the paths the diff should stay within. **`read_first`** names files the worker reads before editing and supplies the context the task requires.
- **Journal** entries are typed: `handoff` (worker report at commit), `note`, `friction` (something the worker had to discover or work around; recorded so a later skill-improvement loop has data), `punt` (the worker could not complete the task coherently), `halt` (the human's resolution).
- **Verdicts** are per criterion, from the fidelity judge, with a one-line reason. **Findings** are from the defects judge, `blocking` or `advisory`.
- **Journals and verdicts have no ids**; they are positional within their node.

### Ids

Ids name record types rather than field shapes: `spec-N`, `milestone-N`, `task-N`, `criterion-N`, `decision-N`. `spec` and `decision` are unique per repository; `milestone` and `task` per spec; `criterion` per node and referenced as `task-7/criterion-2`. The tool assigns ids monotonically within each scope and never reuses one. Lint pattern: `^(spec|milestone|task|criterion|decision)-[1-9][0-9]*$`.

### Statuses

Milestones and tasks share one vocabulary. The spec also uses `draft`, `approved`, and `shipped`.

```text
planned    written, not yet checked for readiness (task) or not yet started (milestone)
ready      task passed lint and, when enabled, the readiness check
working    a worker owns it (task) or a child is in progress (milestone, spec)
judging    handoff recorded; judges dispatched
verified   every criterion passed and no blocking finding remains
failed     a criterion failed or a blocking finding stands; repair pending
halted     repair cap reached, or a punt; waiting on a human
accepted   a human took it as-is at a halt; reported as accepted risk
dropped    removed from scope by a human

spec only: draft -> approved   (the contract gate); verified -> shipped
```

Task state machine:

```text
planned  -> ready      lint clean; readiness check passed or disabled
planned  -> planned    readiness check found unknowns; planner re-plans or splits (round unchanged)
ready    -> working    next dispatches work; base = HEAD
working  -> judging    handoff recorded on a clean worktree
working  -> halted     worker punts
judging  -> verified   all criteria pass, no blocking finding
judging  -> failed     otherwise
failed   -> working    round < cap: repair with prior verdicts and findings in the packet
failed   -> halted     round = cap
halted   -> working    human: retry with guidance (round resets to 1)
halted   -> dropped    human: split (siblings are added by the planner) or drop
halted   -> accepted   human: accept as-is
```

Milestone state machine:

```text
planned  -> working    first child task dispatched
working  -> judging    every child is verified, accepted, or dropped, and at least one is verified
judging  -> verified   milestone judges pass
judging  -> failed     otherwise; planner adds fix tasks (round increments)
failed   -> working    round < cap
failed   -> halted     round = cap
halted   -> working | accepted | dropped    human decision, as for tasks
```

Spec state machine:

```text
draft    -> approved   human approves the contract card
approved -> working    first next call
working  -> judging    every milestone and direct task is verified, accepted, or dropped
judging  -> verified   spec judges pass
judging  -> failed     planner adds a fix milestone or task (round increments); failed -> working | halted as above
verified -> shipped    human confirms the ship card
any      -> dropped    human
```

Readiness is derived, never stored: a milestone is judgeable when its children are settled; a spec is judgeable when its milestones and tasks are. `next` computes these on every call.

### Definition of Ready

A task is ready when a fresh session on the weakest configured model could implement it from `speccy show task-N` alone:

1. One concern, one coherent diff, `touches` named.
2. Criteria checkable from the result without reading the plan; no more than `lint.max_criteria`.
3. No research left: every decision the worker would face is settled in the plan or the parent's rationale. An unknown means not ready.
4. The packet fits the budget (`lint.packet_tokens`) and the plan is finishable in one session.

Enforcement is layered: the planner prompt carries the definition; `speccy check` warns on structural signals (criteria count, `touches` count, plan length, estimated packet tokens); the optional readiness check (`readiness_check: on`) has the `readiness` agent restate the plan from the packet and list unknowns — any unknown sends the task back to the planner; and a worker punt is the backstop signal that the task was too big.

## Storage and I/O

### Layout

```text
.speccy/                    gitignored by default (see "Git Policy")
  config.yaml               tiers, agents, caps, lint thresholds, ship mode
  project.yaml              charter and decision log
  specs/spec-N.yaml         one file per spec; milestones, tasks, journals, verdicts inside
  render/                   speccy render output
```

YAML is the storage format: it provides explicit nesting, addressable fields, per-write schema validation, and diff-readable prose in string fields. Markdown does not delimit a node independently of headings; a `##` in a journal entry changes that extent. Re-serializing a Markdown AST also rewrites unrelated lines. HTML provides explicit nesting and a rendered view but requires an HTML parser.

### Single writer

Only the `speccy` binary writes under `.speccy/`. Agents pass markdown fragments to `speccy ctl … --input <file|->`; the tool validates the record, assigns ids, applies the status rule, and writes the file. Agents do not read storage. For Claude, `speccy init --target claude` adds `Read(./.speccy/**)` to the project's `.claude/settings.json` deny list; Codex packs instruct agents to use the scoped `speccy show` view.

### The projection: `speccy show`

`speccy show <id> [--format packet|card|json]` is the agent-to-agent contract. `packet` (default) is a scoped markdown view:

```markdown
# spec-2 Passwordless login  (working)
## Charter
<purpose, non-goals — summarized>
## Intent chain
- spec-2: <intent>
- milestone-1 Token model and endpoints: <intent>
## task-7 Consume endpoint  (working, round 2 of 2)
Intent: …
Criteria:
- criterion-1 …
- criterion-2 …
Plan: …
Touches: src/server/auth/
Read first: src/server/db/schema.rs
## Journal
- 10:02 worker handoff: …
## Verdicts (round 1)
- criterion-2 fail (fidelity-judge): "…"
## Findings (round 1)
- blocking (defects-judge): "…"
```

Ids and statuses always appear. The charter is summarized, ancestors give only their intent, and journals and verdicts appear in full for the target node only. The `card` format is the human approval surface (see "Human Gates"). The format is golden-tested; a change to it is a change to the protocol.

### Rendering

`speccy render [spec-N]` writes a standalone HTML page with an inline stylesheet to `.speccy/render/` and prints the path. It is a view, not storage; a live `serve` is a later capability.

### Git Policy

`speccy init` appends `.speccy/` to `.gitignore`. A team may remove the line to keep the tree in history, accepting journal noise in diffs. Because the tree is not committed, workers run in the primary worktree; per-task worktree isolation is a later capability that depends on a committed or shared tree.

Committed by `init`: the rendered packs (`.claude/skills/speccy-*/`, `.claude/agents/speccy-*.md`, `.agents/skills/speccy-*/`, `.codex/agents/speccy-*.toml`) and the Codex `[agents]` config keys. Rendered agent files carry model and effort values resolved from the gitignored `config.yaml`; a team that shares packs across machines un-ignores `config.yaml` too.

Commits are made by workers, one per task attempt, at handoff. Speccy records the range on the task and never commits, squashes, or branches itself; `/speccy-run` creates a branch at start as prose convention, and squashing belongs to the PR tool.

## Architecture

```text
   /speccy-plan   /speccy-run ──loop──▶ /speccy-next ──▶ planner | worker | judges | readiness
        │              │                     │                        (fresh subagents)
        └──────────────┴──── speccy ctl … ───┴──────── speccy next / show ─────────┐
                                     │                                             │
                              ┌──────▼──────┐                                      │
                              │   speccy    │  validate · mint ids · next · show   │
                              │   binary    │  render · lint · init · doctor       │
                              └──────┬──────┘                                      │
                              .speccy/*.yaml  +  git status / diff  ◀──────────────┘
```

### Deterministic core

The `speccy` binary owns:

- Schema validation and lint of every write; id minting.
- `next`: the single loop directive, computed from the YAML files and `git status`; it also records derived transitions.
- `show` and `render`: the projections.
- Status derivation when verdicts and findings are recorded.
- `init` and `init --update`: pack generation from templates and config; the `.gitignore` line; the Claude deny rule; the Codex depth setting.
- `check`, `status`, `doctor`, `config resolve`.
- One git rule: `next` refuses `work` while the worktree is dirty, and `journal append --kind handoff` refuses a dirty worktree, so every judged diff is one task's commits.

The core has no lease, no event log, no snapshots, no command execution, and no knowledge of models beyond resolving config names. A single active session per repository is an assumption the design states rather than enforces.

### Prose layer

The prose layer contains three skills and five lifecycle agents rendered per harness. Skills drive the loop; agents do one directive each in a fresh context. Their prose carries the rubrics, the Definition of Ready, and the defensive rules: fail closed on a controller error, never infer the next step from memory, never write Speccy identifiers into product files.

## The Loop

### `speccy next`

```bash
speccy next [--spec spec-N] --json
```

Returns one directive:

```json
{
  "action": "work",
  "spec": "spec-2",
  "target": "task-7",
  "agents": ["worker"],
  "round": 2,
  "cap": 2,
  "show": "speccy show task-7",
  "record_with": "speccy ctl journal append --target task-7 --kind handoff --input -",
  "reason": "task-7 failed criterion-2 in round 1"
}
```

`action` is one of:

| Action | Target | Agents | Recorded with |
| --- | --- | --- | --- |
| `expand` | milestone (or the spec, for fix tasks) | `planner` | `speccy ctl task add` / `task set` / `task split` |
| `check_ready` | task | `readiness` | `speccy ctl ready record` |
| `work` | task | `worker` | `speccy ctl journal append --kind handoff` (or `--kind punt`) |
| `judge` | task, milestone, or spec | `fidelity-judge`, `defects-judge` | `speccy ctl judge record` |
| `halt` | the halted node | none | `speccy ctl halt resolve` |
| `ship` | spec | none | `speccy ctl ship record` |
| `done` | — | — | — |

`next` is idempotent: until the directive's outcome is recorded, the same call returns the same directive. It never mutates the tree except to apply derived readiness (`working -> judging` on milestones and specs whose children settled).

### Sequencing

Order within a spec is document order. `next` picks, in priority: a `halt` anywhere; a `judge` for any node whose children settled; the first task that is `failed` with rounds remaining; the first `planned` task (readiness) or `ready` task (work) under the first unfinished milestone or direct task list; an `expand` for the first milestone with no tasks; `ship` when the spec is `verified`; `done`. One node is worked at a time.

### Expansion

Milestones are written at plan time: they are the shape the human approves. Tasks are written when `next` first reaches their milestone, by the `planner` reading the code as it now is. Deferring tasks under later milestones keeps their plans aligned with landed work.

The planner also handles `expand` with a `reason`: re-planning a task the readiness check rejected, adding fix tasks after a failed milestone or spec judgment, or splitting a punted task into siblings.

### Readiness

With `readiness_check: off` (default) a lint-clean task moves `planned -> ready` when the tool writes it. With `on`, `next` emits `check_ready`: the `readiness` agent — the weakest configured model — reads only the packet, restates the plan in its own words, and lists unknowns. `ready record` with no unknowns sets `ready`; with unknowns it appends a `note` and `next` emits `expand` on the parent with the unknowns as `reason`.

### Work

The worker receives the packet, implements only the task, runs whatever checks the repository's AGENTS.md prescribes, commits, and records a `handoff` journal entry: what changed, what was skipped, commands run and their exit codes, and any `friction`. The tool records `commits` as the range from `base` to HEAD and moves the task to `judging`. A worker that cannot complete the task coherently records a `punt` instead; the task halts.

### Judging

Two fresh agents per node, dispatched together (both read-only):

- **`fidelity-judge`** is plan-blind. It receives the node's intent, criteria, and the diff (task: its commit range; milestone or spec: the integrated range plus the children's verdict summaries) and returns `pass | fail` per criterion with a one-line reason. It judges whether the change does what the node says — nothing missing, nothing extra, nothing reinterpreted.
- **`defects-judge`** receives the diff, `touches`, and repository access and returns findings the criteria never mentioned: logic errors, edge cases, error handling, regressions in adjacent paths, and **gamed criteria** — a criterion satisfied by weakening or removing a test, a type, or a check. Each finding is `blocking` or `advisory`.

`judge record` takes both outputs in one payload and derives status: `verified` when every criterion passed and no blocking finding stands, else `failed`. Advisory findings are recorded and shown; they never block. Separate prompts keep criterion judging independent from defect hunting. The two seats may use different models.

At milestone and spec level the fidelity question becomes "do the settled children together satisfy this node's intent and criteria", and the defects question becomes integration: seams between children, and drift from the charter for the spec.

### Repair and halts

A `failed` task re-enters `working` with the prior verdicts and findings in its packet, up to `caps.repair_rounds` (default 2). A failed milestone or spec re-enters through `expand`: the planner adds fix tasks, counted against the same cap on that node. At the cap the node is `halted` and `next` returns `halt` with the human's options:

| Answer | Effect |
| --- | --- |
| retry with guidance | `working`, round reset to 1, guidance appended as a `halt` journal entry and included in the packet |
| split | task `dropped`; `expand` on the parent with the guidance; siblings added |
| accept as-is | `accepted`; reported on the ship card as accepted risk |
| drop | `dropped` |

A punt halts immediately without consuming rounds. Halts are conversations in the `/speccy-run` session; if that session is gone, `speccy status` shows the pending halt and any later session resumes it.

### Ship

When the spec is `verified`, `next` returns `ship`. `/speccy-run` shows the ship card, the human confirms, and `ship record` writes `shipped` with the commit range, the disclosure line, and the approved decision-log entries. Then, by `config.ship`: `ask` offers a PR (opened by the harness through its own `gh`) or leaving the branch; `pr` always opens one; `branch` leaves the branch. Speccy does not squash and does not detect merges.

## Human Gates

The charter and contract gates protect decisions that later review does not re-check. The ship gate protects the outward-facing change. Nothing else blocks; halts are questions the loop asks when it cannot proceed.

1. **Charter** — once per repository, on the first `/speccy-plan` when `project.yaml` is `draft`. A short conversation produces `purpose`, `users`, `non_goals`; the human approves in prose; `project approve` records it.
2. **Contract** — per spec. The card is `speccy show spec-N --format card`:

```text
spec-2  Passwordless login                            risk: auth surface
Why:   Users sign in with a single-use emailed link   (charter: "no passwords")
In:    request link by email · token expiry and replay protection · expired-link UI
Out:   OAuth · admin session revocation · email vendor change
Done when:
  criterion-1  A link can be requested by email
  criterion-2  A link is single-use
  criterion-3  An expired link is rejected and creates no session
Shape:
  milestone-1  Token model and endpoints     criterion-1, criterion-2
  milestone-2  Expired-link UI state         criterion-3
Judged by: fidelity-judge big/deep · defects-judge mid/deep · pack v2.0
Reply: approve · revise: <text> · drop
```

   Approval moves the spec `draft -> approved`. `/speccy-run` refuses any other status.

3. **Ship** — per spec:

```text
spec-2  Passwordless login                            verified
Changed   9 files  +412 −38   a7f4c2e..d13b0f9   2 milestones · 5 tasks · 2 repair rounds
Criteria  3 of 3 passed at spec level · 11 of 11 across tasks
Accepted  task-4 accepted as-is: "rate limiting deferred; tracked in decision-4"
Advisory  2 findings recorded (speccy show spec-2 --format json)
Decisions to record
  decision-3  Magic-link tokens are stored hashed
Disclosure  Judged by fidelity-judge and defects-judge at their configured model and effort levels, pack v2.0.
            No Speccy-owned deterministic checks ran; repository checks ran under the worker.
Reply: ship · rework: <text> · drop
```

   `rework` appends a fix task seeded with the text and returns to the loop.

The skill echoes each gate decision before recording it (`Recording: spec-2 draft -> approved`).

## Skills and Agents

### Skills

- **`/speccy-plan <intent>`** — conversational. Runs the charter gate when needed; reads the charter and repository read-only; route-checks the request (too small: a "just do it" card, nothing recorded); drafts the spec with milestones; presents the contract card; records approval.
- **`/speccy-next [spec-N]`** — performs exactly one directive: calls `speccy next`, spawns the named agents in fresh context with the `show` packet and the `record_with` command, and exits with a one-line summary. Run directly by a human in the main session it may ask questions; run as a subagent it cannot, so on `halt` it returns the halt for the caller to handle.
- **`/speccy-run [spec-N]`** — the long-running layer: `while next != done: spawn /speccy-next`; on `halt` asks the human in the main session and records the answer; on `ship` shows the ship card. Its own context accumulates one summary line per directive.

### Lifecycle agents

| Agent | Directive | Sees | Returns |
| --- | --- | --- | --- |
| `planner` | `expand` | packet for the target, repository read-only, `reason` | new or revised tasks (or milestones for spec-level fixes) |
| `readiness` | `check_ready` | the task packet only | restated plan, unknowns |
| `worker` | `work` | task packet, repository read-write | commits and a `handoff` or `punt` |
| `fidelity-judge` | `judge` | intent, criteria, diff; children's verdicts for composites | verdict per criterion |
| `defects-judge` | `judge` | diff, `touches`, repository read-only | findings |

Agent names are a closed set; `config.yaml` may only configure these five.

### Harness requirements

Depth: main session (0) → `/speccy-next` (1) → lifecycle agent (2). Claude Code's default subagent depth (3) suffices. Codex defaults to `agents.max_depth = 1`; `speccy init --target codex` merges `[features] multi_agent = true` and `[agents] max_depth = 2` into the project `.codex/config.toml`, warning if a lower value is present, and `speccy doctor` checks it.

Only skills running in the main session may use the harness's structured-question tool; agents never depend on it.

### Packs

```text
.claude/skills/speccy-{plan,next,run}/SKILL.md
.claude/agents/speccy-{planner,readiness,worker,fidelity-judge,defects-judge}.md
.agents/skills/speccy-{plan,next,run}/SKILL.md
.codex/agents/speccy-{planner,readiness,worker,fidelity-judge,defects-judge}.toml
.codex/config.toml            [features] multi_agent, [agents] max_depth (merged keys only)
.claude/settings.json         permissions.deny Read(./.speccy/**) (merged key only)
```

Packs are generated output: `speccy init` writes them from templates plus `config.yaml`, resolving each agent's model and effort into the harness's fields (`model`/`effort` frontmatter for Claude; `model`/`model_reasoning_effort` for Codex). `init --update` applies pack updates; a file whose hash differs from its last render is skipped with a warning unless `--force`. Customization is config, AGENTS.md, and the repository's own rules, not edits to pack files. Skill bodies follow the Agent Skills spec (metadata under ~100 tokens, body under 500 lines) and stay language- and toolchain-agnostic; repository checks are named by the repository's AGENTS.md.

## Configuration

```yaml
# .speccy/config.yaml
models:                                   # your names → per-harness model id
  big:   { claude: "<model-id>", codex: "<model-id>" }
  mid:   { claude: "<model-id>", codex: "<model-id>" }
  small: { claude: "<model-id>", codex: "<model-id>" }
efforts:                                  # your names → per-harness effort value
  deep:   { claude: high,   codex: xhigh }
  normal: { claude: medium, codex: medium }
  quick:  { claude: low,    codex: low }
agents:                                   # fixed keys
  planner:        { model: big,   effort: deep }
  worker:         { model: mid,   effort: normal }
  fidelity-judge: { model: big,   effort: deep }
  defects-judge:  { model: mid,   effort: deep }
  readiness:      { model: small, effort: quick }
caps:
  repair_rounds: 2
readiness_check: off                      # off | on
lint:                                     # warnings, never errors
  max_criteria: 4
  max_touches: 8
  packet_tokens: 8000
ship: ask                                 # ask | pr | branch
```

The model and effort names are the user's; `init` writes placeholders taken from each harness's documentation at implementation time. `speccy check` rejects an agent referencing a name absent from `models` or `efforts`, an agent key outside the fixed set, and a table missing a column for an installed harness. `speccy config resolve <agent> --harness <h> --json` returns the concrete `{model, effort}`; `init` uses it when rendering.

## CLI Surface

Human-facing:

```bash
speccy init [--target claude|codex|all] [--update] [--force]
speccy status                      # specs in flight, pending halts, next human action
speccy show <id> [--format packet|card|json]
speccy render [spec-N]
speccy check                       # schema, ids, lint warnings, config
speccy doctor                      # packs present, depth setting, deny rule, git
speccy next [--spec spec-N] --json # the directive; also used by skills
speccy config resolve <agent> --harness <h> --json
```

Machine-facing writes, all returning `{ok, data}` or `{ok: false, error: {code, message, details?}}` and all taking `--input <file|->` for prose:

```bash
speccy ctl project set        --input charter.md
speccy ctl project approve
speccy ctl spec new           --input spec.md          # title, intent, scope, risk, criteria, rationale, milestones
speccy ctl spec approve       --spec spec-N
speccy ctl spec set-status    --spec spec-N --status dropped
speccy ctl task add           --parent milestone-N|spec-N --input task.md
speccy ctl task set           --target task-N --input task.md
speccy ctl task split         --target task-N --input tasks.md      # drops the target, adds siblings
speccy ctl ready record       --target task-N --input readiness.json
speccy ctl journal append     --target <id> --kind handoff|note|friction|punt --input -
speccy ctl judge record       --target <id> --input judgment.json   # both judges' outputs; derives status
speccy ctl halt resolve       --target <id> --input decision.json   # retry | split | accept | drop, guidance
speccy ctl ship record        --spec spec-N --input ship.json       # decisions to append, mode chosen
```

Payload shapes are the YAML records above minus tool-owned fields (ids, timestamps, `base`, `commits`, statuses). Every write validates the whole file after the change and refuses on any error; nothing is written partially.

## Verification Stance

The harness verifies by LLM judgment, and Speccy records the result. This is a deliberate trade:

- **What a judge can miss.** A criterion that reads as met but is not; a defect outside the diff's neighborhood; a gamed check the defects judge did not notice; anything the model's training biases it to overlook. Judges can miss criteria and defects and cannot prove absence.
- **What the trade buys.** No evidence-execution subsystem, no command sandboxing or output capture, no controls, no provenance scanner: each of those grows the codebase in proportion to what it verifies, and each was present in the previous design.
- **How the risk is bounded.** Two judges with different briefs and, by config, different models; plan-blind fidelity judging; an explicit anti-gaming brief; per-criterion reasons the human can read; a bounded loop; and the repository's own checks running under the worker as its AGENTS.md prescribes — Speccy does not own them, but they run.
- **How the trade is disclosed.** The contract card names the judges and pack version; the ship card carries the disclosure line naming the judge models, the pack version, and the statement that no Speccy-owned deterministic checks ran.

Speccy adds deterministic checks only when dogfooding names a failure class judges keep missing. Candidates include a provenance scan over the diff (Speccy identifiers leaking into product files) and a fail-before/pass-after control for bug-fix tasks. Each is added as its own decision with the failure class that motivates it.

## Later Capabilities

Each item names the argument that would promote it.

- **Deterministic checks** — provenance scan, fail-before/pass-after control, command evidence capture. Promoted by a judged failure class dogfooding shows judges miss.
- **Reviewer personas beyond two judges** (security, style). Promoted by findings the defects judge repeatedly misses in a class.
- **Committed tree** — `.speccy/` in git for history and cross-machine sync. Promoted by a second machine or a second collaborator.
- **Per-task worktree isolation with setup hooks** (Symphony's `after_create`/`before_run`). Depends on the committed tree.
- **Self-improving skills** — workers editing their own skill files from `friction` entries. Promoted once `friction` data shows the same lesson recurring across specs.
- **Dropping the contract gate** in favor of "corrections are cheap, waiting is expensive." Promoted if dogfooding shows contract approvals are rubber stamps.
- **Linear export** — the tree maps one-to-one onto Project → Milestone → Issue.
- **`speccy serve`** — live render while a run is in progress.
- **Markdown-fragment lint** in `ctl` writes beyond schema validity.
- **Cost recording** — the harness reports tokens per directive into the journal; Speccy cannot measure them itself.

## Open Questions

- Whether the readiness check should default `on` for a repository whose `worker` tier is `small`.
- Whether milestone-level judging earns its cost on specs with a single milestone, or should collapse into the spec judgment.
- What the packet token estimate uses when no tokenizer is available; a character heuristic is the placeholder.
