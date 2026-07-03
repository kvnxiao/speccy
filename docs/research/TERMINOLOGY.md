# Speccy Domain Terminology

Status: authoritative for vocabulary
Date: 2026-07-04

This document names Speccy's vocabulary — the proper nouns and set phrases used
across product docs, CLI, controller API, install packs, and review packets. It
defines each term in one line and points to the owning `DESIGN.md` section for
mechanics. Enum *values* (run states, task statuses, requirement statuses, risk
tiers, directive actions) are defined in `DESIGN.md` with the state machine that
owns them, not here; this doc only names the vocabularies and where they live.

Speccy should not feel like process software. Users start from plain engineering
intent; the controller quietly keeps the state needed for resumption, evidence,
and review.

## Core nouns

- **Speccy** — a spec-driven run controller for coding agents; it installs into
  harnesses, supplies deterministic state and evidence tools, and never calls an
  LLM. The harness calls Speccy; Speccy does not call the harness. Canonical:
  "Speccy", "Speccy controller", "Speccy install pack". Avoid "mission
  controller", "Speccy IDE".
- **Workspace** — the git repository (or subtree of one) a spec operates
  against; runtime state always lives outside it, in `~/.speccy/` (see "Storage
  Model" in `DESIGN.md`).
- **Initiative** — an optional grouping of related specs, for intent too broad
  for one coherent spec. Not required in MVP.
- **Spec** — the primary user-facing object: one coherent change, repair, or
  capability with a clear definition of done (see "Core Concepts" in
  `DESIGN.md`).
- **Spec reference** — the stable public handle (`SPEC-20260630-A7F4`); the one
  spec ID a human may see. Routine commands infer the current spec, so it is
  rarely typed. A title and title-slug are mutable and not identifiers.
- **Spec revision** — an approved or draft snapshot of spec intent; one active
  approved revision per run, immutable in place once approved (see "Planning
  Packet and Draft Contract" in `DESIGN.md`).
- **Run** — one attempt to implement and verify an approved revision; the
  execution container for the task graph, evidence, findings, decisions, and the
  review packet. Not the user-facing goal.
- **Task** — a bounded implementation unit mapped to one or more requirements,
  small enough to hand to a worker and review (see "Task" in `DESIGN.md`).
- **Requirement** — an atomic, checkable claim derived from the spec, in plain
  English (`R-AUTH-004`), mapped to tasks and evidence. IDs are spec-local, not
  global.
- **Acceptance ledger** — the binding requirements-to-evidence record: what was
  required, what changed, what evidence holds, what residual risk remains (see
  "Acceptance Ledger" in `DESIGN.md`).
- **Evidence** — the proof for a requirement. An *evidence request* is the
  planned proof (`command | review | browser | api | manual`); an *evidence
  artifact* is the collected result. For `kind: command` the controller executes
  and records it — the agent never pastes command output.
- **Worker** — a fresh-context harness session that implements one task and
  returns a structured *handoff*; never the only validator of its own work.
- **Reviewer persona** — a named review lens dispatched as a fresh-context
  subagent during review (default roster `spec-fidelity`, `defects`, `security`,
  `style`); each records *findings* (see "Reviewer Personas" in `DESIGN.md`). A
  "persona" is always a review lens, never a worker or planner role.
- **Review packet** — the compact human-facing summary of a verified run; humans
  read it instead of transcripts (see "Review UX" in `DESIGN.md`).
- **Escalation packet** — the focused handback when a run gives up on a
  requirement, scoped to that requirement, not the whole run (see "Escalation
  Packet" in `DESIGN.md`).
- **Decision record** — a captured historical decision (approve, waive, scope
  change, architecture, cancel). One carrying `carry_forward: true` reaches
  future planning (see "Carry-Forward Decisions" in `DESIGN.md`). An **ADR** is
  an optional export of a durable architecture decision.
- **Waiver** — a human decision at a gate to accept a requirement's risk without
  sufficient evidence; sets the requirement `waived` atomically inside
  `run record-decision`, the one status path outside `requirement set-status`.
- **Risk tier** — `minimal | standard | high | critical`; controls evidence
  strictness and the number of gates, not the workflow shape (values and the
  evidence table in "Acceptance Ledger" in `DESIGN.md`).
- **Controller** — Speccy's deterministic core: state, gates, scheduling,
  evidence bookkeeping, resume, and packet generation. Not a semantic judge of
  high-level English scenarios.
- **Controller operation** — a machine-facing `speccy ctl ...` command used by
  skills, subagents, custom clients, or tests, not a human workflow command
  (full list in "Controller API Surface" in `DESIGN.md`).
- **Run lease** — the controller's one-writer-at-a-time enforcement for a run;
  state-mutating operations require the token, while `finding record` and
  non-command `evidence record` are lease-free (see "Run Lease and Concurrent
  Writers" in `DESIGN.md`).
- **Install pack** — the harness-facing skills, subagents, and glue that call the
  controller; the only integration surface, rendered per harness from templates
  (see "Harness-Native Install Packs" in `DESIGN.md`).
- **Harness skill** — a Speccy workflow invoked inside the harness, as a slash
  command or by natural-language fallback: `/speccy-brainstorm`, `/speccy-plan`,
  `/speccy-implement`, `/speccy-ship` (see "Harness Skills" in `DESIGN.md`).

## Status vocabularies

Enum values live with the state machine that owns them in `DESIGN.md`:

- **Run states** — "Spec Draft and Run State".
- **Task statuses** — "Task".
- **Requirement statuses** — "Requirement Resolution Rules".
- **Directive actions** — "Deterministic Loop Driving: run next".
- **Risk tiers** — the risk table in "Acceptance Ledger".

Two coarse rollups are rendering rules, never stored values:

- **Human status bucket** — collapses requirement status at human checkpoints to
  **Proven** (`passed`), **Accepted risk** (`review_passed`, `waived`), and
  **Needs you** (`failed`, `blocked`, `pending`). First screens never print raw
  enum values (`review_passed` renders as "review-only evidence"); see
  "Review UX" in `DESIGN.md`.
- **Run status label** — collapses run state on `speccy status` cards:
  Implementing / Verifying / Ready to ship / Needs you (`escalated`) / Awaiting
  merge / Interrupted; see "CLI/Admin Flow" in `DESIGN.md`.

### Spec status

Controls whether an old spec is considered during future planning: `draft`,
`approved`, `cancelled`, `accepted`, `superseded`, `archived`. Only
non-cancelled, non-superseded, non-archived specs are default planning
candidates. Archiving is a list-visibility action; carry-forward decisions from
archived specs reach planning only once the decision index ships (see
"Carry-Forward Decisions" in `DESIGN.md`).

## Naming discipline

Speccy status words are never reused across enums, so no word means two things.

| Say | Avoid | Reason |
| --- | --- | --- |
| spec | mission, prompt, PRD | The primary user-facing object. |
| run | mission, job, run instance | The execution attempt, no strategic connotation. |
| initiative | mission, project | A grouping of related specs. |
| acceptance ledger | validation contract | Lighter; matches evidence/status bookkeeping. |
| requirement | assertion, criterion | Maps cleanly to evidence and status. |
| evidence request / artifact | test / proof | Evidence may be command, review, browser, api, or manual, and still needs adequacy review. |
| reviewer persona | validator (only) | A persona is a review lens, never a worker or planner. |
| handoff | summary | Structured, tied to task and requirements. |
| review packet | report, transcript | Compact and decision-oriented. |
| escalation packet | failure report | Scoped to the unsatisfiable requirement. |
| `minimal` risk tier | `tiny` risk | `tiny` is a scope-rating size; risk is `minimal/standard/high/critical`. Size and strictness must not share words. |
| run `landed` | run "accepted" | `accepted` is a spec status; a merged change's run state is `landed`. |
| task `integrated` | task "accepted" | Task completion vocabulary must not collide with spec acceptance. |
| qualified "accept" forms | bare "accepted" | Qualify every use: "acceptance ledger", "task integrated", "run landed", "spec accepted". |
| controller operation | public command | `speccy ctl` is machine-facing; humans do not type it. |

## ID scope summary

| ID | Scope | User-visible? | Example |
| --- | --- | --- | --- |
| `workspace_id` | Speccy store | Rarely | `ws_a81f23` |
| `spec_ref` | Workspace/export | Yes | `SPEC-20260630-A7F4` |
| `spec_id` | Runtime store | Rarely | `spec_01j1bxgvk3e6q8r2n5tcvh7pyd` |
| `spec_revision_id` | Spec | Sometimes | `spec_rev_002` |
| `run_id` | Spec | Sometimes | `run_01j1bxgvk3tf4qs6mv9zpxwe8d` |
| `requirement_id` | Spec | Yes | `R-AUTH-004` |
| `task_id` | Run | Sometimes | `T3` |
| `evidence_id` / `finding_id` / `handoff_id` | Run | Rarely | `ev_12a4` / `fd_77e1` / `ho_9bc2` |
| `decision_id` | Run | Sometimes | `dec_20260630_001` |

Only the spec reference is routinely user-addressable; run IDs surface only for
precision, debugging, export, or explicit `--run` selection. Routine commands
infer the current target like git.
