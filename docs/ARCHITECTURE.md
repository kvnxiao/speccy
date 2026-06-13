# Speccy Architecture

> The design hub: what Speccy is, what it believes, the vocabulary it
> uses, and the boundaries it deliberately holds. The mechanical
> contract (commands, file formats, lints, the loop) lives in the
> sibling docs indexed below.
>
> Speccy is an AI-first, lightweight feedback engine for spec-driven
> development. It does not try to enforce determinism on LLMs. It makes
> intent, proof, and drift mechanically visible so agents and humans can
> catch divergence before it ships.

---

## Where the details live

This file is the *why*. The mechanical contract is split across three
sibling docs so each has a single topic:

| Doc | Owns |
|---|---|
| [CLI.md](./CLI.md) | Every `speccy` command, its flags, the `--json` envelopes, and command-behavior details (spec-ID allocation, init detection, `verify` exit code, `next` priority, `check` rendering, diff scoping). |
| [SCHEMA.md](./SCHEMA.md) | The file layout, every artifact template (SPEC / TASKS / REPORT / MISSION) and element grammar, the TASKS.md state model, the per-task and per-SPEC journals, staleness detection, and the full lint code registry. |
| [WORKFLOW.md](./WORKFLOW.md) | The five phases, the adversarial review fan-out, amendment, the skill / harness layer (what ships, per-host template variables, recipes, persona definitions), and the per-phase model pins. |

---

## Mission

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

The CLI is intentionally thin. The intelligence lives at the edges: in
skills, prompts, personas, and reviewers. The Rust CLI does not call LLMs
in v1.

---

## Stance: feedback, not enforcement

LLMs do not reliably follow instructions. Treating Speccy as an
enforcement system would be a category error: every gate we add is just
another instruction an LLM can fail to obey, and every failure mode of
enforcement (false positives, blocked-but-actually-fine, agent works
around the gate) is worse than visibility.

So Speccy is a **feedback engine**:

- The CLI tells you what looks off; you decide.
- `speccy verify` is the only command that exits non-zero on problems,
  and it only exits non-zero on broken **proof shape** (parse errors,
  requirements with no scenarios, scenario refs that don't resolve,
  internal inconsistency). CI calls it. Local runs print findings and
  exit zero.
- **Speccy does not run project tests.** Project CI owns test execution:
  `cargo test`, `pnpm test`, lint, type-check, and `cargo deny check` run
  alongside `speccy verify`, not through it.
- **Reviewer personas own semantic judgment.** Whether a scenario is
  meaningful, whether the diff actually satisfies it, and whether the
  project tests cover the scenario meaningfully are questions for the
  business / tests / security / style / correctness reviewer loop, not
  for the CLI.
- There is no `--strict` mode, no policy file, no configurable
  enforcement. Speccy is opinionated about what to surface and silent
  about what to do about it.
- Skills wrap this feedback into agent workflows. The skill layer is
  where the loops live, where personas are defined, and where
  intelligence about "what to do next" gets exercised.

If Speccy ever feels like it's getting in the way, that's a bug in
Speccy, not in the user's workflow.

---

## Proper nouns

| Noun | What it is | Where it lives |
|---|---|---|
| **Mission** | Scope of one long-running initiative composed of 1+ specs | `specs/[focus]/MISSION.md` (optional grouping; omit for flat single-focus projects) |
| **Spec** | One bounded behavior contract | `specs/[focus]/NNNN-slug/SPEC.md`, or `specs/NNNN-slug/SPEC.md` when ungrouped |
| **Requirement** | One observable behavior with a done condition | `<requirement>` element block in SPEC.md |
| **Task** | One implementation slice sized for one agent | `<task>` element in `TASKS.md` |
| **Check** | One English validation scenario a requirement must satisfy | `<scenario>` element block nested under a `<requirement>` in SPEC.md |

The project-wide product north star ("what we're building, why, who for,
what 'good enough to ship v1' looks like") is **not** a Speccy noun. It
lives as a section inside `AGENTS.md` at the repo root, loaded into every
rendered prompt so the north star is always in context.

A **Mission** is the scope of one long-running initiative composed of
multiple related specs. Mission folders are optional: a project with one
focus area may have zero MISSION.md files, with specs living flat under
`.speccy/specs/`. When a focus accumulates 2+ specs that share enough
context that loading them together at plan time is cheaper than
rediscovering it, the planner skill creates `specs/[focus]/MISSION.md` and
writes new specs into the focus folder.

That is the complete conceptual vocabulary. Capability, milestone,
release, decision, amendment, assumption, constraint, invariant,
question, scenario, claim, lease, validation, evidence, finding, and
review are all either cut, derived from artifact state, or rendered as
freeform markdown sections inside SPEC.md / TASKS.md / MISSION.md /
AGENTS.md.

---

## Core development loop

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

Phase verbs are skill responsibilities, not CLI verbs. The CLI never
renders natural-text prompts. Its job is deterministic state work:
scaffolding (`init`), state queries (`status`, `next`, `vacancy`), hash
recording (`lock`), scenario rendering (`check`), and proof-shape gating
(`verify`). Skills discover paths and the derived `next_action` through
the CLI's `schema_version: 1` JSON envelopes; the CLI is the sole
authority on the `NNNN-slug` directory rule. The full phase prose lives in
[WORKFLOW.md](./WORKFLOW.md).

---

## What we deliberately don't do

These are not v1 features. Each was considered and rejected.

| Cut | Reason |
|---|---|
| Capability map (`CAP-NNN`) | Mission folders (`specs/[focus]/MISSION.md`) cover grouping. No second taxonomy. |
| Milestone state machine | Replaced by tag-based releases + a checklist file if the project wants one. Missions are *scope*, not lifecycle. |
| Release readiness as separate gate | Same: git tag + checklist. Not first-class. |
| Decision (ADR) as a separate artifact | Decisions live inline in SPEC.md as `<decision id="DEC-NNN">` elements (ids parsed and validated). No separate folder, no CLI command, no lifecycle machinery. |
| Amendment as TOML | Replaced by SPEC.md frontmatter `status` + `## Changelog` table. |
| Assumption / Constraint / Invariant / Question as TOML | All collapse into SPEC.md narrative sections. |
| Scenario as a standalone artifact | Scenarios are `<scenario>` elements nested inside their parent `<requirement>` in SPEC.md; there is no separate scenario file, registry, or lifecycle. |
| Per-requirement delta markers (`[ADDED]`/`[MODIFIED]`/`[REMOVED]`) | SPEC.md frontmatter `status` + `supersedes` + `## Changelog` table cover lifecycle. |
| Automatic archiving of completed specs | `speccy archive` relocates a spec to `.speccy/archive/` only when explicitly invoked; the CLI never archives on its own, and frontmatter `status` remains the lifecycle indicator. |
| Task `writes` globs and scope enforcement | LLMs declare them wrong; enforcement was net-negative. |
| Claim files / leases for task pickup | No locking on the task-claim race: `state="in-progress"` on the `<task>` element is enough, and a git conflict resolves a double-claim. This exclusion is scoped to task claiming; it does *not* forbid append serialization. `speccy journal append` does take a per-file advisory lock around journal writes; that is internal to the append command, not a task-claim lease. |
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
| Public `speccy fmt` command | The canonical SPEC.md form is a grammar contract enforced by the parser; a user-facing formatter is out of scope for v1. |

The point is not that these features are wrong. The point is that v1 is
small enough to trust.

---

## Comparison to peers

Brief positioning. None of these are wrong; Speccy borrows from each.

| Tool | Strength Speccy borrows | Speccy diverges by |
|---|---|---|
| **OpenSpec** | Lightweight change proposals, low-ceremony | Smaller surface; less focused on iterative review loop |
| **Spec Kit** | `/specify` `/plan` `/tasks` opinionated flow, PRD-shaped templates | Speccy adds adversarial review loop, multi-persona |
| **Kiro** | Steering files for durable agent context | We use `AGENTS.md` + `skills/`; no IDE coupling |
| **GSD** | Milestone-driven verification, autonomy levels | Speccy drops formal milestones; verification stays |
| **BMAD** | Phased context engineering, agent personas | Personas in skills, not built-ins; phases match |
| **Cursor rules** | Rule-folder layering for persistent context | `AGENTS.md` + `.claude/rules/` adopted directly |

Speccy's distinctive bet: **multi-persona adversarial review run by the
same agent host that did the implementation**, with state and notes
living in markdown the same agent will read in the next iteration. That is
where drift gets caught in this system.

---

## Threat model

V1 makes these failures loud:

- Spec has no requirements
- Requirement has no nested `<scenario>` element
- Spec ID disagreement: folder digits, SPEC.md frontmatter `id:`, and
  TASKS.md frontmatter `spec:` are not all the same
- TASKS.md references requirements that don't exist
- TASKS.md is stale relative to SPEC.md (hash drift)
- Open question in SPEC.md is unchecked
- Reviewer persona returns `blocking`
- Task is `state="in-review"` but at least one persona review is missing
- REPORT.md `<coverage>` element references a requirement or scenario
  that does not resolve under the sibling SPEC.md
- Per-task `journal/T-NNN.md` is missing for a completed task, exists for
  a pending task, or has shape / binding / round-sequence violations
- `<implementer>`, `<review>`, or `<blockers>` element appears inside a
  `<task>` body in TASKS.md (misplaced; they belong in the sibling
  journal file)

V1 intentionally does not catch:

- Semantic correctness of any scenario
- Whether the project tests actually satisfy a scenario (project CI and
  the reviewer-tests persona own this)
- Whether the implementation actually meets `done-when`
- Whether the reviewer was thorough
- Whether the agent invented assumptions in `<implementer>` journal
  entries
- Whether the PR description matches REPORT.md
- Whether the project will work end-to-end in production
- Architecture drift across specs

Those failures are review's job, the human's job, or out of scope for a
feedback engine.

---

## Success criteria

Speccy v1 is complete enough when:

- A solo developer can run `speccy init` in a fresh repo and reach their
  first green check via the shipped skills without inventing process.
- The same developer can run `speccy init` in an existing repo at any
  point in its life and use Speccy productively on a small slice without
  reverse-engineering the whole codebase.
- An AI coding agent driven by the shipped skills can complete a full
  Plan → Tasks → Impl → Review → Report loop on a non-trivial spec
  without needing the human to chain commands manually.
- Reviewer personas catch at least one class of drift per review run on
  representative work (the proof here is the dogfooded Speccy itself).
- `speccy verify` is a reliable CI gate: passes when the proof shape is
  intact, fails when it isn't, never flakes on its own state.
- Speccy drives its own development. The repo tracks its implementation
  under `.speccy/specs/` and preserves it under `.speccy/archive/` after
  each spec ships, with passing checks and review records.

Speccy v1 does not need to autonomously ship software. It needs to make
autonomous software construction less blind, and to make the next project
anyone builds with it feel qualitatively different from "ask the agent to
do everything and hope."

Speccy dogfoods its own development. Every SPEC in this repo's
`.speccy/` tree is the proof for the corresponding slice of the binary;
if a SPEC's `status` is `implemented`, the behaviour it describes is what
the binary does today.

---

## Long-term vision

Speccy aims to become the **deterministic feedback substrate** that
multi-agent harnesses can build on. The in-pack implementation + review
orchestration loop now ships as part of the skill layer
(`/speccy-orchestrate` chained with `/speccy-vet`), so single-host
end-to-end execution is no longer a future layer. The following remain
future layers (not v1):

- Concurrent task pickup with file-locking or task queues
- Worktree orchestration per task
- Cross-spec dependency reasoning
- Project-level dashboard / kanban UI consuming `status --json`
- Production telemetry feedback into spec state
- Cross-repository orchestration

The foundation should remain unchanged across these layers:

> Explicit, inspectable, feedback-only contracts between intent and
> shipped behavior, with adversarial multi-persona review as the primary
> drift-detection mechanism.
