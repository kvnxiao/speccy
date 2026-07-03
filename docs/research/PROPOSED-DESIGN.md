# Design: Spec-Driven Multi-Agent Orchestration Tool

Status: authoritative
Date: 2026-07-02

This document proposes a design for a modular spec-driven orchestration tool that coordinates existing coding-agent harnesses. The design is intentionally open-ended and includes unresolved questions.

Working name in this document: `speccy`.

## Product Thesis

`speccy` is a small higher-layer spec-driven run controller for coding agents.

It does not write code itself and does not replace Claude Code, Codex, Cursor, Copilot, Jules, OpenHands, or a custom Pi-based harness. It installs into Codex and Claude Code as harness-native skills/agents, turns an engineering request into a lightweight spec, acceptance ledger, task sequence, run state, and review packet, then delegates implementation and validation through the user's active harness.

The core product promise:

> Spend more attention up front on intent and evidence, spend less attention babysitting the agent while it implements, and receive a compact review packet showing what changed, what was tested, what drifted, and what still needs human judgment.

## Design Principles

1. **Harness-neutral by construction.** The run controller exposes stable controller operations and install packs, not vendor-specific execution paths.
2. **Zero product-code footprint.** Speccy must not affect product source, the build graph, deployed artifacts, runtime dependencies, or production behavior. Repo-local harness packs are workflow artifacts and may be committed; runtime run state remains external or ignored by default.
3. **Acceptance ledger, not process ceremony.** Every approved spec captures requirements, evidence, and status in one small ledger. Higher risk changes evidence strictness, not the workflow shape.
4. **Shared state, not agent chat.** The run store is authoritative. Agent transcripts are evidence, not state.
5. **Serial writes, parallel reads.** One writer at a time by default, enforced by a run-level lease. Parallelize research, review, and validation, including concurrent reviewer personas whose findings record without contention.
6. **Fresh-context verification by pack design.** Speccy runs no agents, so it cannot force verifier independence at runtime. Its install pack is structured so the harness runs implementation and verification in separate fresh-context subagents, and the controller refuses to mark a requirement `passed` without recorded evidence. Independence is a convention the pack arranges and the evidence trail exposes, not a property Speccy enforces.
7. **Deterministic core, prose policy.** State transitions, scheduling, artifact storage, and gating are code. Role behavior, review rubrics, task decomposition, and repair strategies are editable prose/templates.
8. **Explicit human gates.** Human attention is requested at high-value checkpoints, not continuously.
9. **Portable failure recovery.** A run must resume after process crashes, context compaction, harness failure, rate limits, and partial implementation.
10. **Harness-native first, no outbound agent calls.** Codex and Claude Code skills/agents are the preferred user interface. Speccy commands expose deterministic local controller behavior only; harnesses call Speccy, Speccy does not call LLMs or coding-agent harnesses.

## Non-Goals

- Replacing coding agents.
- Building a general multi-agent chat platform.
- Creating a new IDE.
- Making all implementation parallel.
- Multi-day autonomous missions and in-run skill self-evolution. These are deferred until the single-spec MVP shape is proven. If run horizons ever extend beyond hours, per-repo skill self-evolution becomes a prerequisite per the Factory analysis, not an enhancement.
- Persisting run state, transcripts, or per-spec process artifacts in the repo by default. Committed harness packs and the `.speccy/` project config are deliberate workflow artifacts, not this non-goal.
- Supporting non-git workspaces. Resume and evidence baselines depend on git.
- Requiring `claude -p`, `codex exec`, or API-key-backed headless execution for normal interactive use.
- Calling out to LLMs, coding agents, or AI harnesses from any `speccy` command or subcommand.
- Hiding uncertainty behind green checkmarks.
- Auto-deploying production changes without explicit policy.
- Treating LLM narrative as equivalent to captured evidence.

## Core Concepts

### Initiative

An initiative is an optional grouping of conceptually related specs.

It is useful when the user's request is too broad to be one coherent spec but has a shared direction, such as "modernize authentication." Initiatives should not be required in MVP. They exist to keep broad work from becoming a single oversized spec.

Example:

```text
Initiative: Modernize authentication
  Spec: Passwordless login
  Spec: OAuth account linking
  Spec: Session expiration hardening
  Spec: Auth audit logging
```

### Spec

A spec is the primary user-facing object.

It describes one coherent change, repair, or capability with a clear definition of done. A spec starts as a brainstormable draft, becomes binding when approved, and can then be implemented, verified, reviewed, exported, or rerun.

A spec contains:

- User request.
- Clarifications and assumptions.
- Scope boundaries.
- Goals and non-goals.
- Requirements.
- Acceptance ledger.
- Design plan, if needed.
- Planned task breakdown for the approved revision.
- Decision records.
- One or more runs.

The scale rule:

> A spec should describe one meaningful capability or repair that can be completed, reviewed, and judged independently.

If the work is not independently valuable, it is probably a task inside a spec. If it contains several independently valuable outcomes, it is probably an initiative containing multiple specs.

Each spec should have one stable public reference, such as `SPEC-20260630-A7F4`, plus a mutable human title. The public reference is used in review packets, exports, CI, and scripts. Humans should rarely need to type it during normal interactive work: `speccy status`, `speccy review`, and in-harness skills should infer the current spec when unambiguous, and free-form arguments such as `passwordless` should be treated as search selectors rather than alternate IDs. Title-derived slugs may appear in export paths for readability, but they are not identifiers and do not need to be unique by themselves.

### Run

A run is one autonomous attempt to implement and verify an approved spec revision against a workspace.

It is the execution container. A run owns runtime state, scheduling, task graph, harness assignments, worker handoffs, validator findings, evidence artifacts, resume behavior, and the final review packet.

A run is not the user-facing goal. It is the operational attempt to satisfy the spec.

A run targets one coherent spec and is expected to complete in minutes to a few hours. Speccy does not target multi-day autonomous missions. The long-horizon Factory mission pattern is an architectural reference, not the MVP scale.

### Decision Records and ADRs

A decision record captures an important scope, evidence, waiver, validation, or architecture decision made during spec drafting, implementation, validation, repair, or review. Spec-scoped decisions, such as revision approvals and amendments, are recorded through `spec record-decision` and stored on the spec; run-scoped decisions, such as gate decisions and waivers, are recorded through `run record-decision` and stored on the run.

Architectural decision records (ADRs) are a narrower subset: durable architecture decisions that future maintainers should know. Speccy should keep decision records in the run store by default and optionally export selected architecture decisions as ADRs, such as `docs/adr/0007-use-magic-link-tokens.md`.

### Acceptance Ledger

The MVP baseline should be one streamlined acceptance ledger. It is the capture area that prevents agents from claiming completion without tying requirements to checks, review evidence, or explicit waivers.

The ledger answers four questions:

- What did the user require?
- What changed to satisfy it?
- What evidence shows it was tested or reviewed?
- What is still unproven, waived, blocked, or risky?

The default artifact should stay small:

```yaml
goal: "Fix the CSV export timestamp formatting bug."
scope:
  in:
    - "CSV export timestamps"
  out:
    - "database schema changes"
risk: standard
requirements:
  - id: R1
    statement: "Existing exports keep the same columns."
    evidence:
      kind: review
      note: "Diff only changes timestamp formatting path."
    status: pending
  - id: R2
    statement: "Timestamps are formatted as ISO-8601 UTC values."
    evidence:
      kind: command
      command: "npm test -- csv-export"
    status: pending
```

The `goal`, `scope`, and `risk` here are denormalized from the spec revision for readability. The spec revision stays their source of truth; the ledger's own concern is the `requirements` rows and their evidence.

This shape is intentionally provisional for the MVP. Speccy should not promise a stable public artifact format until real usage proves the right boundaries.

Baseline rules:

- Every approved spec has an acceptance ledger before implementation starts.
- Every requirement has a stable local ID, a plain-English statement, and one or more evidence requests: command/test output, browser/API observation, file/diff review, harness review, manual evidence, explicit waiver, or blocked/unproven status.
- For `kind: command` evidence, the controller executes the command: `speccy ctl evidence collect` runs it and records exit code, stdout, stderr, and a content hash. `evidence record` refuses agent-pasted output for that kind, so `passed` on command evidence never rests on a transcript claim. Trust narrows to review, browser, and manual kinds, which the risk tiers already treat as weaker.
- An approved revision's ledger is immutable in place: requirement statements and evidence requests are frozen at approval. Agents may only propose draft patches; a human prose approval creates a new revision and a new run. Verifiers change requirement status only, through evidence operations.
- The final review packet includes the ledger, status, commands run, evidence links, and residual risk.
- A task cannot reach `integrated` while any linked requirement is still
  `pending`.
- A run can become `verified` only when every requirement is resolved as
  `passed`, `review_passed`, or `waived`. Raw `blocked` and `unproven` statuses
  remain **Needs you**; they force an escalation or policy gate unless a human
  decision records an explicit `review_passed` judgment or waiver with residual
  risk.

Risk still matters, but it changes the burden of evidence inside the same ledger rather than introducing a separate workflow:

| Risk | Use For | Ledger Requirement | Verification Depth |
| --- | --- | --- | --- |
| Minimal | Formatting, docs, typo fixes, obvious one-line repairs, dependency metadata with no behavioral impact. | One to three requirements, evidence can be command output or focused review. | Existing relevant checks plus final packet. |
| Standard | Normal bug fixes and small features with localized blast radius. | Requirements mapped to declared evidence requests. | Verifier gathers command/test/diff/review evidence and does lightweight evidence-adequacy review for new tests. |
| High | Auth, billing, data loss, migrations, security, broad refactors, concurrency, public APIs, compliance-sensitive behavior. | Same ledger, but important requirements need stronger evidence such as negative cases, positive cases, pre-fix failure, or explicit human waiver. | Fresh-context verifier, evidence-adequacy review, and human approval gates where policy requires them. |
| Critical | Production safety, regulated domains, irreversible migrations, incident repair, or explicit audit needs. | Same ledger plus retained evidence, decision log, and optional redacted run bundle. | Human gate before risky writes, stronger evidence retention, and optional external review. |

Scenario prose is allowed when useful, but it should remain in the ledger as clarification, not become a new mandatory artifact. A `given/when/then` scenario should map to one or more evidence requests:

- Command/test evidence: shell command, unit test, integration test, static analyzer, database query, or similar machine-run output.
- Browser/API evidence: a harness-driven browser or API observation with captured result.
- File/diff evidence: review of changed files, selectors, routes, migrations, or configuration.
- Harness review evidence: fresh-context harness review with structured findings.
- Manual evidence: explicit human decision.
- Blocked/unproven: the requirement cannot currently be verified.

The verifier agent should collect evidence for all of these. Speccy provides evidence tools that make some collection reproducible, such as running a command and storing exit code/stdout/stderr, but it should not force users to think in separate deterministic versus LLM-verification phases. The ledger records evidence type, collector, raw artifact reference, reviewer judgment, and residual risk.

### Verification Ownership

The dependency should be inverted: a harness verification agent collects evidence for the acceptance ledger, and Speccy provides state, evidence capture tools, and evidence recording. The Speccy CLI is not an LLM and should not pretend to semantically judge scenario prose.

Practical meaning:

- Acceptance linting, evidence capture, evidence recording, and requirement status updates should be internal controller operations, not public SDLC-shaped CLI commands.
- Verification is a phase of `/speccy-implement`, not a separate entry skill. It runs as the internal verifier role inside the implement loop.
- The verifier role calls Speccy controller tools such as `packet verification`, `evidence collect`, `evidence record`, `finding record`, and `requirement set-status`.
- For `kind: command`, the controller is the collector: `evidence collect` executes the declared command and records exit code/stdout/stderr/hash itself, and `evidence record` rejects agent-supplied output for that kind. The verifier still decides when to collect and judges adequacy; it never transcribes command results.
- The verifier role is responsible for collecting evidence, interpreting semantic scenario prose, reviewing evidence adequacy, and performing adversarial vacuity review.
- The verifier role should call Speccy tools to fetch scoped acceptance packets, collect command/test/diff evidence when useful, record evidence, and write structured reviewer findings.
- For minimal- and standard-risk specs, verification should usually mean "check the ledger, run declared commands, review the diff, and record residual risk." It should not manufacture a heavy process because a small task entered Speccy.

So the normal flow is:

```text
Speccy controller produces scoped verification packet
  -> harness verification agent reads acceptance ledger requirements
  -> agent asks Speccy controller tools to collect reproducible evidence where useful
  -> agent reviews whether evidence actually supports the requirement
  -> agent records structured findings through Speccy
  -> Speccy updates the acceptance ledger and review packet
```

The CLI should stay small. Its job is install/admin/export/custom-run plumbing. The verification agent gets reliable controller tools for evidence capture, evidence hashing, status recording, rerun support, and result packaging. Neither the CLI nor the controller is the semantic judge for high-level English scenarios.

### Vacuity Checks

Avoiding vacuous tests is a hard requirement for any evidence Speccy relies on. The MVP should not turn that into mandatory mutation testing or heavyweight controls for every task. The right default is risk-scaled inside the same ledger:

- Minimal: no formal vacuity procedure. The final packet records the commands run and the reviewer checks that the diff matches the small request.
- Standard: lightweight vacuity review. If a new test or evidence item is used, the verifier asks, "What failure would this catch, and does it exercise the changed path?"
- High: explicit evidence adequacy. Important requirements need evidence that distinguishes success from failure, preferably with positive/negative controls or a pre-fix failure.
- Critical: audit-grade evidence. Require stronger evidence retention, explicit waivers, and optional external/human review before marking critical requirements as satisfied.

A requirement should not pass merely because a test exists, a command exits zero, or a reviewer says "looks good." Speccy should require the evidence to explain how it would fail if the requirement were broken.

Vacuity checks can include:

- Mutation checks: intentionally perturb relevant code, config, test data, or route wiring and confirm the evidence path fails. This is later/high-risk functionality, not an MVP default.
- Negative controls: run the scenario with invalid inputs and confirm the expected rejection/error path.
- Positive controls: run the scenario with valid inputs and confirm the success path.
- Coverage anchors: require evidence that the test exercised specific files, endpoints, selectors, functions, or branches.
- Diff relevance checks: confirm evidence references files changed by the task or known dependencies.
- Requirement-to-test traceability: every generated test should name the requirement ID it proves.
- Pre-fix failure, when possible: for bug fixes, the evidence path should fail against the baseline branch and pass after the fix.
- Fresh-context adversarial review: a validator agent reviews the evidence path for vacuity, shallow checks, mocked-away behavior, overbroad snapshots, or tests that only assert implementation details.

Example:

```yaml
requirements:
  - id: R-AUTH-004
    statement: "Expired magic links are rejected."
    scenario:
      given: "a magic-link token older than the allowed expiration window"
      when: "the user opens the magic-link URL"
      then: "the login attempt is rejected and the user remains unauthenticated"
    evidence:
      kind: browser
      setup:
        command: "npm run dev"
        wait_for: "http://localhost:3000"
      steps:
        - seed_fixture: "expired_magic_link"
        - goto: "http://localhost:3000/auth/magic?token=expired-test-token"
        - expect_text: "This login link has expired"
        - expect_session_absent: true
        - expect_no_exported_cookie: "session"
    vacuity:
      controls:
        positive:
          fixture: "valid_magic_link"
          expected: "login succeeds"
        negative:
          fixture: "expired_magic_link"
          expected: "login fails"
      mutation_checks:
        - name: "disable expiration comparison"
          patch_hint: "force token expiry check to always return false"
          expected_to_fail: true
      evidence_must_reference:
        - "magic link token expiry check"
        - "session creation path"
```

The mutation check does not need to be sophisticated in MVP. It can start as an adversarial reviewer task: "Identify the smallest code change that would break this requirement. Would the proposed evidence path fail?" Later, Speccy can automate common mutations.

### Task

A task is a bounded implementation unit mapped to one or more ledger requirements. Tasks should be serializable into a worker prompt and small enough to review.

The planned task breakdown belongs to the spec revision. When `/speccy-implement` creates a run, the run instantiates its own runtime task graph from the approved revision's plan; rounds, assignments, and task status live on the run, and a new revision yields a new run with a fresh task graph. Each task runs as its own serial implement-review loop:

```text
task packet
  -> implementer edits only the task scope
  -> implementer handoff
  -> fresh task reviewer/verifier checks linked requirements
  -> task repair round, if needed
  -> task integrated
```

Each runtime task carries a task status — `queued | building | reviewable | in_review | needs_repair | integrated | deferred`, defined in `TERMINOLOGY.md` ("Task Status") — and a controller-owned round counter, so an interrupted session can resume mid-task.

When a task enters `building`, the controller records `baseline_commit` — the workspace git HEAD at claim time — on the task and preserves it across resume, so every diff, review, and evidence check has a stable baseline even after a crash mid-round.

When a task reaches `integrated`, the controller snapshots the workspace as a git commit on the run's working branch and records the commit SHA on the task. Snapshots make resume deterministic: any uncommitted diff belongs to the current in-flight task, so `next-action` can tell "resume a partially built task" from "dispatch a fresh task" by comparing task status, round counter, and workspace dirtiness against the last snapshot.

This nested task loop is different from final run review. Task review is about whether a bounded unit is correct enough to move on. Final run review is about cross-task drift, requirement coverage, integration behavior, residual risk, and whether the whole spec should be accepted, repaired, waived, or rejected.

### Worker

A worker is a harness session that performs implementation. A worker receives only the relevant spec/design/task/context packet and must return a structured handoff.

### Validator

A validator is a fresh-context harness session or deterministic runner that checks work against the acceptance ledger. Validators can be:

- Command validators.
- Test validators.
- Browser/UI validators.
- Code review validators.
- Security validators.
- Documentation validators.
- Requirement-coverage validators.
- Vacuity validators.
- Human validators.

### Handoff

A handoff is the worker's signed report. It should include:

- Task ID.
- Summary of changes.
- Files touched.
- Commands run.
- Exit codes.
- Requirements claimed satisfied.
- Known issues.
- Deviations from plan.
- Follow-up recommendations.

### Review Packet

The review packet is the human-facing output. It should be compact enough to review without reading raw transcripts.

It includes:

- Spec summary.
- Run summary.
- Diff summary.
- Requirement coverage table.
- Accepted-risk requirements, plus unresolved requirements for escalated or
  policy-gated runs.
- Validator findings.
- Risk summary.
- Commands and evidence.
- Open questions.
- Suggested next action.

## Architecture

```text
      +------------------+     +------------------+     +------------------+
      | Codex/Claude     |     | custom harness   |     | CLI/admin        |
      | install packs    |     | integration      |     | deterministic    |
      +--------+---------+     +--------+---------+     +--------+---------+
               |                        |                        |
               +-----------+------------+------------------------+
                           |
                +----------v-----------+
                |  Spec/Run Controller |
                | state machine/gates  |
                +-----+-----------+----+
                      |           |
      +---------------v--+     +--v----------------+
      | Run Store        |     | Policy/Role Packs |
      | external state   |     | markdown/templates|
      +------------------+     +-------------------+
```

### Deterministic Core

Responsibilities:

- Parse spec/run inputs.
- Manage run state.
- Enforce gate transitions.
- Schedule work.
- Serve the next deterministic loop action through `run next`.
- Serialize write tasks.
- Parallelize read-only tasks safely.
- Track acceptance and evidence status.
- Persist logs and evidence.
- Compare claimed versus proven completion.
- Build review packets.
- Resume interrupted runs.

The deterministic core should avoid encoding high-level planning intelligence. That belongs in role packs and model prompts.

### Prose Layer

Responsibilities:

- Planner prompt.
- Worker prompt.
- Validator prompt.
- Repair prompt.
- Review rubric.
- Spec-card and evidence-adequacy rubric.
- Project-intake interview.
- Risk classification guidance.

These should be versioned with the tool and overrideable by users.

### Loop Ownership and Defensive Prose

Speccy makes no outbound agent calls, so it does not drive the autonomous loop. The install-pack skills and subagent prompts drive it: the harness agent reads run state, spawns workers and fresh-context verifiers, records handoffs and evidence, and asks the controller whether to repair or advance. Speccy owns the gates, not the loop.

This places part of the loop's reliability in prose the harness executes. To keep that surface small, the controller owns sequencing: the loop-driving prose is a thin cycle around `speccy ctl run next` (see "Deterministic Loop Driving"). The prose asks the controller for the next step, performs it, records the result, and asks again. The remaining prose rules are defensive:

- Fail closed: when state is ambiguous or a controller call errors, stop and surface it rather than assuming success.
- Never mark a requirement satisfied from the same context that produced the change; dispatch a fresh-context verifier.
- Read the controller's returned state after every call instead of assuming the transition happened.
- Never infer the next step from transcript memory; only `run next` decides sequencing, rounds, and gates.

The controller backstops a misbehaving loop-driver with deterministic gates: it rejects invalid state transitions, refuses `verified` while any requirement is unresolved, refuses `passed` without recorded evidence, and enforces the repair cap. It cannot detect whether a verifier was genuinely fresh-context or whether recorded evidence is honest. That independence is arranged by the pack and made auditable by the evidence trail, not enforced at runtime.

Per-role model selection lives in the skill/subagent frontmatter of the install pack, so the implementation seat and the verification seat can use different models or providers where the harness supports it.

### Deterministic Loop Driving: run next

The controller exposes one loop-driving operation:

```bash
speccy ctl run next --run <id> --agent <id> --json
```

`run next` reads current run state and returns the single next required
step, the next-action directive. The install-pack loop prose reduces to a
thin cycle: call `run next`, perform the directive, record the result through
the named controller operation, and call `run next` again. Sequencing, round counting, cap
enforcement, and gate detection are controller decisions, never prose
decisions.

A directive includes at least:

- `run_state`: the current run state.
- `action`: one directive, such as `dispatch_worker`,
  `dispatch_task_verifier`, `spawn_repair_round`, `run_final_validation`,
  `await_human_gate`, `emit_escalation_packet`, or `halt`.
- `subject`: the task, requirement, or gate the directive applies to.
- `packet_with`: the packet operation to run before performing the action —
  `packet task` for `dispatch_worker`, `packet verification` for
  `dispatch_task_verifier` and `run_final_validation`, `packet escalation`
  for `emit_escalation_packet` — or null when no packet is needed.
- `round`: for repair directives, the controller-owned counter and its
  policy-configured cap, such as `{ "current": 2, "max": 3, "scope": "task" }`.
  Per-task repair rounds and run-level review rounds have separate policy
  values. The orchestrating agent reports what the controller said — "starting
  repair round 2 of 3" — and never counts rounds itself.
- `record_with`: the controller operation that must record the outcome, so the
  loop closes deterministically.
- `reason`: a compact explanation the skill can surface verbatim in status
  updates.

Rules:

- `run next` is idempotent. Calling it again without recording a result
  returns the same directive.
- The harness must not infer the next step from transcript memory. After every
  recorded result, it asks the controller again.
- `await_human_gate` and `emit_escalation_packet` stop scheduling and surface
  the gate or packet. An unrecognized directive or a controller error also
  stops the loop and surfaces the error; the prose never guesses.

`run next` is also the single mutation point for derived state. Before
returning a directive it clears expired leases and applies the task
transitions that have no recording operation: `reviewable -> in_review` when
verification dispatches, `needs_repair -> building` when a repair round
starts, and `in_review -> integrated` — including the task's snapshot commit —
once every linked requirement is resolved for the risk tier. Idempotency is
over settled state: once derived transitions apply, repeated calls return the
same directive without re-applying them.

### Run Lease and Concurrent Writers

"Serial writes" is enforced, not asserted. Two `/speccy-implement` sessions on
the same run must not interleave `ctl` calls and corrupt round counting, so the
controller contract includes a run-level lease:

- `run next --agent <id>` issues or renews a lease token bound to that agent
  ID, with an expiry on the order of minutes, renewed on every controller
  call. The token returns with the directive and is passed back as
  `--lease <token>` on state-mutating operations.
- State-mutating operations — `task claim`, `task record-handoff`,
  `requirement set-status`, `run record-decision`, `run record-ship`, and any
  operation a `run next` directive names in `record_with` — require the live
  token, passed as `--lease <token>`. The lease is run-scoped: spec-scoped
  operations predate the run and are not lease-gated.
- A second session asking for the run gets a `lease_held` error naming the
  holder and its expiry, and stops.
- Expired leases are cleared deterministically by the following `run next`
  call; a crashed session never wedges the run.

Concurrent reviewers are the deliberate exception. A task's review phase may
fan out several fresh-context reviewer personas — security, business-logic
correctness, code style — that can complete at the same moment. Their
operations are additive, not state-mutating, so they do not take the lease:

- `finding record`, `evidence record`, and `evidence collect` are lease-free
  additive operations. Each finding and
  evidence artifact is written as its own file keyed by its ID (plus an
  append-only event), never appended to a shared per-task journal, so
  simultaneous completions cannot contend. The SQLite projection serializes
  index updates transactionally.
- Aggregation stays with the lease holder: after all reviewer personas report,
  the orchestrating session (holding the lease) records the resulting task
  status transition.

Findings must carry forward. When a round fails, the next round's task packet
and verification packet include the prior rounds' findings and the verifier's
rejection reasons, so a repair round starts from what was learned instead of
re-discovering the same failure.

### Resume and Crash Recovery

There is no `speccy resume` command and no human resume ritual. Resume is a
controller capability: `run next` must be able to answer "what is the next
required step" for a fresh agent session at any point, including after a crash,
a killed session, context compaction, or a rate-limit abort.

Three mechanisms make that answer deterministic:

- **Task statuses and the round counter.** The runtime task graph records
  `queued | building | reviewable | in_review | needs_repair | integrated |
  deferred` per task plus the controller-owned round counter, so the controller
  knows exactly which phase of which round was interrupted (see "Task").
- **Git snapshots at task boundaries.** Every task records `baseline_commit`
  at claim time, and every `integrated` task ends in a snapshot commit recorded
  on the task. Uncommitted workspace changes therefore belong to the current
  in-flight task: a dirty worktree with a task in `building` means "resume or
  restart this task with the partial diff as context, diffed against
  `baseline_commit`"; a clean worktree means "dispatch fresh."
- **Lease repair.** `run next` clears expired leases before answering, so a
  dead session's lease never blocks the successor.

The flow after any interruption is always the same: start a fresh harness
session, invoke `/speccy-implement <spec>`, and the skill calls
`run next`, which replays nothing and re-derives the directive
from stored state. Mid-directive interruptions are safe because `run next`
is idempotent: a directive whose result was never recorded is simply returned
again.

### Harness-Aware Template Rendering

Install packs should not be copied from one neutral markdown tree into every
harness. Claude Code, Codex, and future harnesses expose different command
syntax, tool names, planning modes, subagent formats, permission prompts, and
skill-loading rules. The source pack should therefore be a template bundle that
renders target-specific exports for a selected harness.

The template bundle ships inside the Speccy tool's own source, not the target repository. Everything under repo-local `.speccy/` and the harness pack directories is rendered output that already targets the selected harness; the repo never holds neutral source templates.

Speccy should use a real implementation-native templating library rather than
ad hoc string replacement. The renderer must support partials/includes,
conditionals, loops, strict missing-variable errors, deterministic output, and
safe escaping for markdown/YAML frontmatter where needed. The implementation
language is Rust (decided 2026-07-02), and the intended engine is `minijinja`
(Jinja2 syntax): includes/macros cover partials, it supports strict
undefined-variable errors, and markdown/YAML escaping can be handled through
custom filters. The choice stands unless implementation proves it cannot meet
a requirement above; the design requirement remains a structured template
engine with testable render inputs and outputs, not loyalty to any particular
engine.

The template context should include at least:

- `target.harness`: `codex`, `claude`, `agents`, or a future harness key.
- `target.scope`: `repo` or `user`.
- `capabilities`: supported primitives such as slash commands, skills,
  subagents, MCP, hooks, structured user questions, and plan mode.
- `names`: harness-native names for important actions and tools, such as the
  planning command/mode and structured-question tool.
- `paths`: rendered output paths for skills, commands, agents, and pack files.
- `controller`: the `speccy ctl ... --json` command prefix and controller
  protocol version.
- `pack`: pack version, source template IDs, managed file IDs, and template
  hashes.

Shared partials should hold harness-neutral policy, acceptance, review, and
repair guidance. Target overlays should provide harness-specific prose,
invocation syntax, direct tool names, frontmatter, file layout, and conditional
exports. For example, a Claude Code brainstorm handoff can reference the
`/plan` command directly and can name `AskUserQuestion` when a structured
question prompt is needed. A Codex handoff should use Codex's planning-mode
wording and should name `request_user_input` for the corresponding structured
question tool. These differences belong in the rendered pack, not in the
deterministic controller.

The renderer should support conditional template exports: a source template may
produce a file only when a harness or capability matches. A Claude command file,
a Claude subagent definition, a Codex skill `SKILL.md`, and a generic `.agents`
role file can share core partials while still rendering different files and
different prose.

Pack update and drift checks should compare rendered outputs, not raw source
templates. `.speccy/pack-lock.yaml` should record enough metadata to reproduce a
managed file's render inputs: target, scope, pack version, source template ID,
source hash, rendered hash, destination path, and relevant capability flags.

### Harness-Native Install Packs

The preferred user experience is "install once, use inside the harness." A user should not need to leave Codex or Claude Code and run a separate one-off prompt command for normal use.

Harness packs are also the only integration level available, not just the preferred one. Speccy integrates into existing harnesses rather than shipping its own, and no mainstream harness exposes a supported way for an external tool to hook its internal loop, planner, scheduler, or state. The one surface every supported harness shares is workflow prose plus local command execution: skills, commands, and agent/subagent definition files whose prose calls `speccy ctl`. Speccy therefore delivers its entire workflow as rendered harness packs by necessity, and accepts the consequence: pack prose is convention the harness follows, while the controller's deterministic gates are the only hard enforcement.

For team workflows, Speccy should install repo-local harness packs by default. This keeps lifecycle prose, role prompts, commands, policies, and templates deterministic across machines and reviewable in ordinary code review. Machine-global packs can be added later through `--user`, but they are not part of the MVP surface.

Installed packs are rendered from the harness-aware template bundle for the
selected target. The committed output should be plain harness-native files, but
the lockfile must retain the source template metadata needed to check freshness,
diagnose drift, and perform future three-way updates.

The install command should be idempotent:

- If a pack is missing, create it.
- If managed files are missing, repair them.
- If the installed pack is old, report that an update is available.
- Do not rewrite existing managed prose with upstream changes unless `--update` is passed.

Both packs ship the same entry skills, listed in "Harness Skills", plus internal role prompts. The entry skills are the primary human-invoked surface; the role prompts and subagent definitions are dispatched by the `/speccy-plan` and `/speccy-implement` loops, not invoked directly.

Codex install pack:

- Entry skills: the `speccy-*` entry skills defined in "Harness Skills".
- Internal role prompts / subagent definitions for planning, implementation, review/verification, and repair.
- Optional Codex plugin manifest that bundles skills and MCP configuration.
- Repo-local `.codex` writes are acceptable when installing the Codex target. In this design, "agents" means role/subagent definitions in the harness install pack, not mandatory repo instruction files.

Claude Code install pack:

- Entry skills / commands: the `speccy-*` entry skills defined in "Harness Skills".
- Claude subagent definitions for the planner, researcher, worker, reviewer, validator, and repair roles.
- Optional MCP configuration only for workflows that explicitly need MCP.
- Repo-local `.claude` writes are acceptable when installing the Claude target.

The installed skills/agents should be thin. They should guide the harness, call the deterministic Speccy controller for spec/run state, and return compact checkpoints. They should not attempt to keep the full spec ledger inside the model context.

Recommended repo-local shape:

```text
.speccy/
  project.yaml
  pack-lock.yaml

.codex/skills/speccy-*/
.claude/commands/speccy-*.md
.claude/agents/speccy-*.md
.agents/speccy-*.md
```

Repo-local `.speccy/` holds exactly two files. `project.yaml` carries project
configuration and machine-readable policy values (risk defaults, repair-round
caps, human-gate rules); `pack-lock.yaml` pins pack versions and render
metadata. There are no `policies/`, `roles/`, or `evidence-presets/` folders:
that prose is harness-facing, so it is template-rendered into the selected
harness pack (`.claude/`, `.codex/`, `.agents/`) where the agent actually reads
it, and edited there. Runtime run state never lives in the repo.

### Controller API Surface

"Not human-facing CLI commands" does not mean "not exposed." Speccy should expose a small harness-facing controller API while keeping the human CLI minimal.

Recommended packaging:

- One `speccy` binary.
- Harness-specific install packs for Codex and Claude Code.
- A machine-oriented CLI controller surface for installed skills and subagents.
- Optional MCP only for harnesses/workflows where tool discovery or external integrations justify the token overhead.
- No separate `speccy-codex` or `speccy-claude` binaries unless packaging constraints force thin entrypoints.
- No `speccy` command that launches Codex, Claude Code, another LLM, or any AI harness.

Human-facing CLI: the command list and per-command semantics live in
"CLI/Admin Flow" under User Experience. There is no `speccy resume` command;
resume is a controller capability (see "Resume and Crash Recovery").

Harness-facing CLI operations:

```bash
speccy ctl spec start --input request.json --json
speccy ctl spec status --spec <ref> --json
speccy ctl spec record-draft --spec <ref> --input spec-draft.json --json
speccy ctl spec patch-draft --spec <ref> --input spec-patch.json --json
speccy ctl spec record-decision --spec <ref> --input decision.json --json

speccy ctl run start --spec <ref> --revision <id> --json
speccy ctl run status --run <id> --json
speccy ctl run next --run <id> --agent <id> --json
speccy ctl run record-decision --run <id> --lease <token> --input decision.json --json
speccy ctl run record-ship --run <id> --lease <token> --input change-ref.json --json

speccy ctl task claim --run <id> --task <id> --agent <id> --lease <token> --json
speccy ctl task record-handoff --run <id> --lease <token> --input handoff.json --json

speccy ctl packet planning --spec <ref> --json
speccy ctl packet task --run <id> --task <id> --json
speccy ctl packet verification --run <id> --requirements R1,R2 --json
speccy ctl packet review --run <id> --json
speccy ctl packet escalation --run <id> --json

speccy ctl evidence collect --run <id> --requirements R1,R2 --json
speccy ctl evidence record --run <id> --input evidence.json --json
speccy ctl finding record --run <id> --input finding.json --json
speccy ctl requirement set-status --run <id> --lease <token> --input status.json --json
```

Naming convention (decided 2026-07-02): operations are noun-first — `spec`, `run`, `task`, `packet`, `evidence`, `finding`, `requirement`, mirroring the nouns in `TERMINOLOGY.md` — with a small verb vocabulary: `start`/`status` for lifecycle, `next` for the loop directive, `claim` and `collect` for actions the controller performs, `record-*` for append-style writes, `patch-*` for partial edits, and `set-status` for status transitions. `speccy ctl <noun> --help` lists that noun's operations.

These commands are implementation details for the installed skills/agents; routine use never requires typing them. They are still designed to be steppable by hand: a human debugging a run can walk `spec status` → `run status` → `run next` and read each directive as a sentence.

Transport options:

1. **CLI, preferred:** installed skills call `speccy ctl ... --json`. LLM coding agents are already good at CLI use, CLI calls are transparent in transcripts, and the interface avoids MCP tool-list/context overhead.
2. **Stdio JSON-RPC, optional:** `speccy rpc` can batch controller operations for custom harnesses or high-throughput integrations.
3. **MCP, optional later:** `speccy mcp` can expose the same controller operations when a harness strongly prefers MCP or when tool discovery matters more than token footprint.
4. **Library API, later:** language bindings can wrap the same controller methods, but should not become a separate product surface.

The public UX remains harness-native. The API exists so skills and subagents have deterministic tools to call.

### Planning Packet and Draft Contract

`spec start --input request.json` creates the spec from a small intent
record: `request` (required — the user's engineering intent, verbatim), plus
optional `source` (which harness skill or `speccy new` recorded it), `title`
(a mutable working title), and `brainstorm_handoff` (the verbatim handoff
when `/speccy-plan` promotes a brainstorm; this is the only point where a
brainstorm handoff is persisted). The controller stores the request on the
spec and echoes it in every planning packet, so a fresh session never depends
on transcript memory for the original ask.

`speccy ctl packet planning --spec <ref> --json` should not call an LLM and should not return a bare template. It returns a deterministic planning work order built from controller state:

- Original user request and source.
- Current spec draft state.
- Workspace path, git state, dirty files, and selected file-tree/manifests.
- Deterministically parsed project signals, such as package scripts, dependencies, language manifests, and configured harness packs.
- Relevant prior spec, decision, and review summaries that are not archived, obsolete, or superseded.
- Policy constraints, risk guidance, and human-gate rules.
- A harness work order telling the planner what to inspect read-only.
- The output contract for the candidate spec draft.

The packet should be JSON because it is a controller-to-harness interface. It may include references to YAML or Markdown artifacts, but it is not itself the canonical spec. The canonical draft and runtime state remains in the controller store; exported specs and ledgers may be rendered as Markdown/YAML for humans.

The harness planner uses the packet to inspect the current codebase and draft a complete candidate spec revision. Speccy should expect imperfect drafts and provide structural validation rather than requiring the LLM to fill every field perfectly on the first attempt:

```text
packet planning
  -> harness inspects repo read-only
  -> spec record-draft as one complete candidate revision, with optional
     intake observations riding along; the response carries structural
     lint findings
  -> spec patch-draft for focused repairs or user edits; the response
     carries lint findings
  -> human sees compact spec card
  -> spec record-decision for approval, rejection, split, or scope change;
     approval is refused while the draft is lint-dirty
```

The initial candidate spec should be submitted all at once instead of appending one section per controller call. This prevents the controller store from accumulating many half-valid intermediate states. Piecewise refinement should happen through patch-style draft updates, such as replacing `scope`, `requirements`, `evidence_requests`, `open_questions`, or `tasks`.

The planner must draft from the current codebase first, then reconcile relevant prior specs and decisions. Prior specs are context, not truth. If current code contradicts a prior accepted spec, the planner should flag drift or staleness rather than silently carrying the old requirement forward.

Relevant prior context should be candidate-scoped. The controller can retrieve candidates by status, tags, touched paths, requirement topics, and decision summaries; the harness classifies each as relevant, stale, obsolete, superseded, or ignored. The human checkpoint should summarize only the carried-forward constraints and notable drift, with links or commands to open the full prior spec when needed.

### Inbound Harness Integration

Speccy integrates with harnesses through an inbound controller interface. The harness owns LLM invocation, agent scheduling, tool use, sandbox prompts, model selection, and harness-specific behavior. Speccy owns state, gates, packets, evidence bookkeeping, and review packet generation.

Minimum integration contract:

```text
harness reads packet from Speccy
  -> harness performs planning, implementation, verification, or review
  -> harness records structured outputs, evidence, findings, handoffs, and decisions
  -> Speccy validates structure and updates deterministic spec/run state
```

Malformed output is handled strictly, with bounded repair. Every record
operation is schema-validated; on failure the controller rejects the payload
and returns structured lint errors; the calling skill retries with a focused
fix up to a policy cap, defaulting to 3 attempts; when the cap is exhausted the
loop fails closed and the run escalates. The controller never coerces partial
payloads into state.

Supported inbound transports:

1. **CLI controller operations, preferred:** installed skills call `speccy ctl ... --json`.
2. **Stdio JSON-RPC, optional:** `speccy rpc` can batch controller operations for custom harnesses or high-throughput integrations.
3. **MCP server, optional later:** `speccy mcp` can expose controller operations to harnesses that strongly prefer MCP.
4. **Library API, later:** language bindings can wrap the same deterministic controller methods.

Out of bounds:

- `speccy run --adapter ...`
- `speccy adapters ...`
- `speccy` invoking `codex exec`, `claude -p`, SDKs, app servers, MCP clients, or generic prompt runners.
- Any `speccy` command or subcommand that calls an LLM or coding-agent harness.

For a Pi-based or internal harness, write a Speccy client inside that harness. The client calls `speccy ctl ... --json`, `speccy rpc`, or eventually `speccy mcp`; Speccy never launches the harness process.

## Spec Draft and Run State

Spec drafting is separate from run execution. A brainstorm handoff or draft spec
can exist before any run exists. `/speccy-plan` decomposes intent into a draft
spec and acceptance ledger and presents the spec card. The human approves the
card in prose, which the plan skill records through the controller, moving the
spec revision to `approved`. `/speccy-implement <spec>` then runs against that
approved revision: it exits early if the revision is not approved, otherwise it
creates a run and starts implementation. Approval is persisted controller state,
not chat state, so `/speccy-implement` can run in a fresh, cleared session.

Spec draft lifecycle:

```text
brainstorm handoff (optional)
  -> /speccy-plan: draft spec + acceptance ledger + spec card
       -> revise
       -> approved revision      prose approval recorded by /speccy-plan
            -> /speccy-implement creates a run
       -> cancelled
       -> split/superseded
```

Run state machine:

```text
created
  -> implementing
       -> implementing       next task (rounds tracked in task graph)
       -> escalated          task repair cap exhausted or human/policy gate
       -> verifying          all tasks integrated
  -> verifying
       -> verifying          run-repair rounds (tracked in ledger)
       -> escalated          run repair cap exhausted or human/policy gate
       -> verified           all requirements resolved
  -> verified
       -> submitted          /speccy-ship opens the PR
       -> cancelled
  -> submitted
       -> landed             speccy accept records that the change merged
       -> cancelled          PR closed unmerged, you stop it
  -> escalated
       -> implementing       setup provided or requirement waived, tasks remain
       -> verifying          setup provided or requirement waived during final validation
       -> cancelled          human stops it, or an approved spec amendment
                             supersedes this run with a new run (decision record links them)
  (any active state) -> cancelled
```

Important rules:

- No run before the approved spec has goal, scope, risk tier, and acceptance criteria.
- No run starts on a dirty worktree: `run start` refuses uncommitted changes before any run state exists, so for the run's lifetime every dirty diff is attributable to the in-flight task.
- The workspace must be a git repository or a subtree of one. Non-git directories are refused; resume and evidence baselines depend on git snapshots and `baseline_commit`.
- No task reaches `integrated` until linked acceptance requirements are resolved for the selected risk tier or explicitly deferred by a recorded human/policy decision.
- Tasks execute serially by default, and each task can repeat implement-review-repair rounds before the scheduler moves to the next task.
- Higher-risk work increases the evidence requirements inside the same ledger.
- A failed task reviewer creates a task-scoped repair round. A failed final validator creates a run-level repair task, a waiver request, or an escalated state.
- Each repair loop is capped by policy, defaulting to 3 rounds. The task repair loop and the run-level repair loop each keep an independent count and an independent cap.
- When a loop exhausts its cap and a linked requirement is still `failed` or `vacuous`, the run gives up, transitions to `escalated`, and emits an escalation packet. Blocked or unproven requirements that prevent verification also transition to `escalated`, but as a human/policy gate rather than a capability-escalation event. See "Capability Escalation and Give-Up Policy."
- After verifying passes, the run enters `verified`: the work is done and awaiting the human's ship decision. `/speccy-ship` opens the PR and moves the run to `submitted`.
- `submitted` advances to `landed` when the human runs `speccy accept` after the change merges. The spec then becomes `accepted` and can be archived. See "Acceptance."
- Human waivers are recorded in the review packet.
- The run state is a single flat enum. Progress within `implementing` and
  `verifying` is read from the task graph and acceptance ledger, not a second
  state field.
- Run state is append-only where possible.

### Capability Escalation and Give-Up Policy

Autonomous repair must terminate. Without a cap, a run can loop on an unsatisfiable requirement and burn the token budget the checkpoint model is meant to protect. Speccy caps repair effort, then hands the problem back to the human.

Not every escalation is a repair-cap failure. A missing credential, unavailable
local environment, production-only behavior, or subjective requirement can also
stop verification. Those cases keep the relevant requirement `blocked` or
`unproven` and move the run to `escalated` as a human/policy gate rather than a
capability-escalation event.

The counting model uses two nouns for two jobs:

- **Task is the unit that is retried.** A repair round re-runs a task, because the implementer edits a task, not one requirement in isolation. The round counter lives on the task.
- **Requirement is the unit that is judged.** A round fails when a linked requirement is `failed` or `vacuous` after the attempt. The give-up decision and the escalation packet are scoped to the requirement, not the task.

The rule:

> A task runs at most the policy-configured number of repair rounds. When the cap is exhausted and any linked requirement is still `failed` or `vacuous`, the run gives up, transitions to `escalated`, and emits an escalation packet naming those requirements.

The same rule applies to the run-level repair loop inside `verifying`. Final validation can fail a requirement that every task passed in isolation. The run-level loop keeps its own independent count, its own policy-configured cap, and the same requirement-scoped give-up.

Why the counter is per task, not per requirement:

- A task maps to one or more requirements. A per-requirement counter would let a task with five requirements loop up to fifteen times, which defeats early exit.
- The implementer cannot retry one requirement in isolation; it re-runs the whole task.

Why the judgment is per requirement, not per task:

- A task has no pass/fail of its own. Worker attempts can finish while linked
  requirements remain unproven, but task acceptance is separate. Task acceptance
  is not requirement satisfaction, so the give-up gate must read requirement
  status.

The same fail-closed rule covers resource caps beyond rounds: optional policy caps on task count and run wall-clock. Speccy makes no LLM calls and cannot meter tokens, so token budgets belong to the harness. Hitting any cap parks the run at an `escalated` policy gate; the human raises the cap or cancels, and the same run resumes.

The caps are two policy values in `.speccy/project.yaml`, not hard-coded constants: one for per-task repair rounds and one for run-level review rounds, each defaulting to 3. The controller reads and enforces them. Every `run next` directive that starts a repair round carries the counter — "spawn repair round 2 of 3" — and the controller returns an escalation directive instead of another round when the cap is exhausted, so the orchestrating agent reports rounds without ever counting them. High-risk specs can lower the caps and prototypes can raise them without touching pack prose.

#### Early Exit, Not Quarantine

When a loop hits the cap, the run stops scheduling forward instead of parking the requirement and finishing the rest of the spec. A stuck requirement often has trickle-down effects: later tasks may depend on the behavior it was supposed to establish. Completing them would burn tokens on work the spec amendment may invalidate.

Early exit preserves already-applied work. Task diffs completed before the escalation stay in the workspace. Nothing rolls back. Before parking, the controller commits any uncommitted in-flight diff as a labeled escalation snapshot, so the workspace stays clean while the run sits in `escalated` with partial work intact.

Example:

```text
Task T3 is linked to R5, R6, R7.

Round 1: R5 passed, R6 failed, R7 passed  -> repair
Round 2: R6 failed                         -> repair
Round 3: R6 vacuous                         -> cap hit

Run -> escalated
Escalation packet scoped to R6.
Tasks after T3 are not scheduled.
```

#### Escalation Packet

The escalation packet is a distinct artifact from the run's review packet. It is scoped to the requirement that could not be satisfied or proven, not the whole run. It is assembled deterministically by `packet escalation` from recorded rounds, findings, and decisions. Exhausting the repair cap is Speccy's signal that the approach or the requirement itself is wrong, not that one more implementation attempt is needed. Blocked or unproven requirements are a signal that the environment, policy, or evidence strategy needs a human decision. The natural resolution is usually a spec amendment, environment fix, waiver, or review-passed judgment with residual risk, not another blind repair.

The user-facing copy should frame escalation around the stuck requirement, not around an agent failure:

```text
Speccy stopped because R-AUTH-004 could not be proven.
This may require changing the requirement, fixing the environment, accepting residual risk, or cancelling.

Recommended: amend the spec
Alternatives: provide setup, waive this requirement, cancel the run
```

It includes:

- The failing requirement ID and statement.
- The approaches the loop tried, each with the verifier's reason for rejecting it.
- What partial work is already applied to the workspace.
- Suggested amendments, when the planner has any.

At the escalation gate the human responds in prose, and the harness records the right decision through `run record-decision` rather than offering a menu of process verbs:

- **Amend the spec.** The usual outcome. Creates a new approved spec revision and a new run, with a decision record explaining why the definition of done changed. The escalated run is closed as `cancelled` with a decision record naming the superseding revision and run. Any guidance the human gives is folded into the amendment.
- **Provide missing setup or evidence.** Keeps the spec revision, records the gate decision, and resumes the same run in `implementing` or `verifying` when the environment is ready.
- **Waive the requirement.** Accept the residual risk; the same run resumes from where it stopped.
- **Cancel the run.**

#### Amendment at the Escalation Gate

The escalation gate is a conversation, not a form. The escalation packet ends with one question, and the human answers in prose in the same harness session. If that session is gone, `speccy status` re-surfaces the pending gate, and any later harness session picks it up from controller state.

The amendment path reuses the planning machinery instead of adding a new surface:

1. The human describes the change in prose, such as "expiry should be 30 minutes, drop R6" or "verify this via the API instead of the browser."
2. The harness records the gate decision, then runs the same draft-revision loop `/speccy-plan` uses: patch the spec draft, lint it, and present an amended spec card that shows the diff against the prior approved revision and names the escalation that motivated it.
3. The human approves the amended card in prose; the harness records the approval through the controller, producing a new approved spec revision.
4. The controller closes the escalated run as `cancelled` with a decision record linking the superseding revision, and the checkpoint copy tells the user to run `/speccy-implement <spec>` in a fresh session.

At escalation the controller commits any uncommitted in-flight diff as a labeled escalation snapshot, so the parked worktree is clean and the superseding run's clean-worktree rule holds. The new run starts on the same branch, seeded with the prior run's summary and the escalation snapshot reference, so it reconciles rather than redoes. Rolling back to the run baseline remains the human's explicit fallback at the gate.

Setup and waiver answers stay on the same run: the harness records the decision, and the following `run next` call resumes the loop from where it stopped. Only amendment replaces the run, because only amendment changes the definition of done.

### Acceptance

When `verifying` passes, the run enters `verified`, and `run next` returns an `await_human_gate` directive for the ship decision with `record_with: run record-ship`. Invoking `/speccy-ship` answers that gate: the skill calls `run next --agent <id>` to take the lease, proposes the change as a pull request or hands it off as a branch or patch, and records the transition and the change reference through `speccy ctl run record-ship`, which moves the run to `submitted`. Speccy is ephemeral, holds no provider credentials, and makes no outbound calls, so in MVP it does not detect the merge at all: the human records it.

The run store persists a small change reference for what was proposed, as provenance:

```yaml
change_ref:
  kind: pull_request        # pull_request | branch | patch | none
  url: https://github.com/org/repo/pull/123
  branch: speccy/passwordless-login
  head_sha: a7f4c2e
  base: main
```

Submitted runs should explain the boundary plainly:

```text
PR merge is the source of truth.
After it merges, record it with:
speccy accept
```

`speccy accept` transitions `submitted -> landed` and records provenance:

```yaml
landing:
  actor: human
  at: 2026-07-01T14:20:00Z
  pr_url: https://github.com/org/repo/pull/123   # optional
  note: "shipped via hotfix branch, PR skipped"  # optional
```

Commands:

```bash
speccy accept [<selector>]                       # record that the change landed
speccy accept --pr <url> --note "<text>"         # optional provenance
speccy archive [<selector>]                      # mark an accepted spec archived
```

- `speccy accept` is a human assertion. Speccy does not verify the merge in MVP; the human is telling Speccy what already happened.
- A PR closed without merging is a flag on `submitted`, not a separate state. The human starts a new run or cancels.

Automatic merge detection — git-native ancestry checks, squash-merge heuristics, or a configurable host probe — is deliberately cut from MVP. It is an external-integration convenience outside core Speccy's goals, and manual acceptance is enough to dogfood the loop. See "Later Capabilities."

When a run reaches `landed`, `speccy status` and `speccy list` surface the
accepted spec once and offer to archive it. Archiving is a spec visibility
action; the landed run remains `landed` in run history.

## Storage Model

Decision (2026-07-01, closes former Open Question 1): runtime state lives in
`~/.speccy/` only. Repo-local `.speccy/` holds exactly `project.yaml` and
`pack-lock.yaml`; all policy, role, and evidence prose is rendered into the
harness packs, and exports are opt-in snapshots written to explicit
destinations such as `docs/specs/`. There is no repo-local runtime mode. A
survey of how prior tools store runtime state, which validates this split, is
in `runtime-state-storage-survey.md`.

Runtime storage is external:

```text
~/.speccy/
  config.toml
  workspaces/
    <workspace-id>/
      workspace.json
      lessons/
      specs/
        <internal-spec-id>/
          spec-ref.txt
          request.md
          assumptions.md
          decisions.jsonl
          spec/
            current.md
            revisions/
              spec-rev-001.md
              spec-rev-002.md
          runs/
            <run-id>/
              run.yaml
              design.md
              acceptance-ledger.yaml
              task-graph.yaml
              events.jsonl
              decisions.jsonl
              handoffs/
              evidence/
              findings/
              review-packet.md
              artifacts/
```

The state model (JSONL-first, decided 2026-07-02):

- Portable canonical log: append-only JSONL events, the source of truth from day one; state is rebuilt by replay.
- Runtime query store: an optional SQLite projection rebuilt from the JSONL log, deferred until queries or scale demand it. The walking skeleton uses an in-memory projection.
- Large artifacts: files referenced by ID and content hash, such as transcripts, diffs, screenshots, command logs, and evidence.
- Generated snapshots: markdown/YAML views for review, not the primary source of truth.

All controller state writes are atomic: write to a temp file, fsync, then
rename over the target. JSONL event appends use verified read-back — the
appended record is re-read and checked before the operation reports success —
so a crash never leaves a half-written transition. Resume from the store is
only trustworthy if every write follows this discipline.

SQLite should not be committed to git. It is binary, noisy, and poor for review. JSONL event logs are text and portable, but they are still operational run history and should not be committed by default either.

### Git Policy

Repo-local Speccy harness packs are commit-safe workflow artifacts. The repo commits exactly two `.speccy/` files plus the rendered packs; everything operational lives in `~/.speccy/`.

Commit by default:

```text
.speccy/
  project.yaml
  pack-lock.yaml

.codex/skills/speccy-*/
.claude/commands/speccy-*.md
.claude/agents/speccy-*.md
.agents/speccy-*.md
```

Commit or attach selectively, via explicit `speccy export` commands: compact review packets, acceptance snapshots, and result summaries, written to explicit destinations such as `docs/specs/<spec-ref>/` or attached to the PR. These are useful when the team wants PR-visible evidence. They should be generated intentionally and kept compact. `.speccy/` itself holds no export folders.

Never committed, because it never exists in the repo: run state, event logs, transcripts, caches, evidence artifacts, and databases. These live in `~/.speccy/`.

Defensive backstop: `speccy install` still writes a `.gitignore` block so that a bug, an older Speccy version, or a stray tool writing runtime paths under `.speccy/` cannot leak state into the repo. It guards against paths that should never exist, rather than documenting a supported layout:

```gitignore
.speccy/*
!.speccy/project.yaml
!.speccy/pack-lock.yaml
```

Rationale:

- Every engineer should not inherit every local agent-run history through git.
- Event logs are append-only and can create noisy diffs and merge conflicts.
- Run logs can contain prompts, paths, errors, screenshots, environment details, or accidental secrets.
- PRs should review product code and compact evidence packets, not operational transcripts.
- Git should store stable project policy and reviewable snapshots; local Speccy storage should store run state by default.

### Lightweight Team Sharing

Do not require a hosted Speccy server for normal team use. The default sharing unit should be the review packet plus compact snapshots, not a run URL.

Default PR/review metadata:

```yaml
speccy:
  spec_ref: SPEC-20260630-A7F4
  spec_title: Passwordless login
  run_id: run_01j1bxgvk3tf4qs6mv9zpxwe8d
  acceptance_hash: sha256:...
  review_packet_hash: sha256:...
  result_summary_hash: sha256:...
  rerun:
    harness_instruction: "Re-run Speccy verification against this branch, acceptance snapshot, and result summary."
```

No-server sharing options:

- Paste or attach the review packet to the PR.
- Commit or attach only compact snapshots when useful: acceptance snapshot, result summary, decision log, and command summary.
- Let other engineers rerun verification against the shared acceptance snapshot; this reuses the verifier role and the local controller tools rather than adding a separate skill, and is a later/team capability.
- Export a redacted run bundle only when debugging or audit needs it: `speccy export run-bundle --redact`.
- Attach that bundle to an issue, PR, CI artifact, or file share outside git rather than committing it.

Optional hosted mode:

- A `run_url` can be included for organizations that choose to run a shared Speccy run store.
- The hosted store should be an optional convenience, not a requirement for collaboration.

Repo writes remain opt-in:

- `speccy export spec`
- `speccy export review`
- `speccy export lessons`
- `speccy export acceptance-snapshot`
- `speccy export result-summary`
- `speccy export run-log` for debugging, compliance, or reproducibility only

This reconciles shared lifecycle prose with zero product-code footprint.

Harness installation writes repo-local workflow artifacts by default:

- `speccy install` detects supported harnesses and installs or repairs repo-local packs.
- `speccy install --target codex` installs or repairs the repo-local Codex pack.
- `speccy install --target claude` installs or repairs the repo-local Claude pack.
- `speccy install --update` applies reviewable pack updates.

Operational run state, transcripts, raw evidence, screenshots, command logs, and databases still live outside git or under ignored paths unless explicitly exported.

## Execution Strategy

### Planning Phase

Planning is the `/speccy-plan` skill. It runs after an optional `/speccy-brainstorm` or directly from intent.

1. Intake prompt asks clarifying questions only when necessary.
2. The controller builds a deterministic planning packet with current workspace state, policy, output contract, and relevant prior context candidates.
3. The harness planner inspects the current codebase read-only and includes intake observations in the draft submission when they are useful for resumability or later review.
4. The planner reconciles relevant prior specs and decisions against the current codebase, carrying forward only constraints that still appear valid and flagging stale, obsolete, contradicted, or superseded context.
5. The planner classifies task risk and creates a complete candidate spec draft with goal, non-goals, scope, assumptions, acceptance requirements, expected evidence, and open questions.
6. Speccy structurally lints the draft. The harness repairs missing or invalid sections through focused draft patches rather than section-by-section append commands.
7. Human approval is requested through a compact spec card: goal, scope, non-goals, plan summary, key requirements, prior context carried forward, open questions, and main risks. The full spec and ledger are available on request.
8. The planner creates only as much design/task detail as the request needs.
9. Each acceptance requirement gets at least one evidence request or an explicit
   `unproven` or `waived` status. Manual human judgment is recorded as evidence
   or as a waiver/review-passed decision, not as its own requirement status.
10. Higher-risk work stays in the same ledger but requires stronger evidence, such as negative cases, positive cases, pre-fix failure, fresh-context review, or human approval.
11. Fresh-context adversarial review is required when new tests or review-only evidence carry an important acceptance decision.
12. The human approves the spec card in prose for every spec; `/speccy-plan` records the approval through the controller, moving the revision to `approved`. `/speccy-implement` exits early until that approval exists.

### Implementation Phase

Implementation is a serial task execution loop. Each task gets its own implementer and fresh task reviewer/verifier, and can run repeatable repair rounds before the scheduler advances:

1. Scheduler selects the next task.
2. Harness worker receives a task packet scoped to linked requirements, expected files/areas, evidence requests, and known constraints.
3. Worker implements only that task.
4. Worker returns a handoff.
5. A fresh task reviewer/verifier reviews the handoff, diff, commands, and linked requirement evidence.
6. The task verifier collects evidence for linked requirements, using Speccy evidence tools when useful.
7. The verifier handles semantic review and evidence adequacy review at the depth required by the risk tier.
8. Acceptance statuses update from collected evidence plus structured verifier findings.
9. Failed, vacuous, or unproven task-linked items create task-scoped repair rounds when the tier requires repair instead of waiver or escalation.
10. The scheduler advances only after the task is `integrated` or explicitly deferred by a recorded human/policy decision.

### Validation Phase

Final validation is a run-level evidence and drift review after task execution. It should not duplicate every task review by default; it checks whether the whole spec still holds after integration:

1. Final verifier reads the acceptance ledger, task handoffs, validator findings, and integrated diff.
2. Verifier gathers baseline integration evidence: format, lint, typecheck, targeted project commands, relevant existing tests, or browser/API checks.
3. Verifier checks requirement coverage across all tasks and identifies requirements that remain failed, vacuous, blocked, unproven, waived, or only review-passed.
4. Verifier performs drift review: compare approved spec/plan/task scopes against the final diff, handoffs, and decisions.
5. Verifier reviews whether the evidence set actually supports the spec at the selected risk depth.
6. Verifier records run-level findings, residual risk, and repair recommendations.
7. Failed integration checks, cross-task regressions, or drift create run-level repair tasks, waiver requests, scope-change decisions, or escalated states.
8. Human final review happens when policy requires it.

Acceptance status uses the eight canonical requirement statuses defined in `TERMINOLOGY.md` ("Requirement Status"). `pending` marks an item whose evidence has not been collected yet; validation resolves each remaining item to `passed`, `review_passed`, `failed`, `vacuous`, `blocked`, `unproven`, or `waived`.

### Parallelism Policy

Allowed by default:

- Codebase exploration by multiple read-only research agents.
- Documentation lookup.
- Test gap analysis.
- Code review validators after a milestone.
- Security/doc validators on read-only snapshots.

Disallowed by default:

- Multiple agents editing the same workspace.
- Parallel dependency installation.
- Parallel migrations touching shared schemas.
- Parallel branch merges without explicit integration task.

Expert opt-in (all post-MVP; MVP is strictly serial-write, enforced by the run lease):

- Parallel write tasks in separate worktrees.
- Multi-repo specs/runs.
- Production infrastructure validation.

## User Experience

### Install Flow

```bash
speccy install
speccy install --target codex
speccy install --target claude
speccy install --target all
```

Installation should add repo-local harness skills/agents by default and wire them to the local Speccy controller. The pack is meant to be committed as shared workflow prose. Machine-global install is a later capability exposed through `--user` when implemented.

Install renders managed files from the harness-aware template bundle for the
selected target and records the render metadata in `.speccy/pack-lock.yaml`.
Rendered files should be ordinary harness-native markdown/YAML/config files so
humans can review and edit them without understanding the source template
language.

Default target detection:

1. If `.codex` exists, include `codex`.
2. If `.claude` exists, include `claude`.
3. If `.agents` exists, include `agents`.
4. If multiple supported harnesses exist, install all detected targets unless `--target` narrows them.
5. If no harness directory exists, no target can be auto-detected. Explain that no supported harness was detected and ask the user to choose `--target codex`, `--target claude`, `--target agents`, or `--target all`. In noninteractive mode, fail unless `--target` is provided.

Target values:

- `auto`: default detection, and the behavior when no `--target` is given. Renders every detected harness, so a repo with both `.codex` and `.claude` gets both.
- `codex`: repo-local Codex skills.
- `claude`: repo-local Claude commands/agents.
- `agents`: generic `.agents` pack.
- `all`: all supported harness packs.

Install should be idempotent. A plain `speccy install` may create missing packs, repair missing managed files, update lock metadata, and report outdated packs. It must not apply upstream changes to existing managed prose unless `--update` is passed.

Update behavior:

```bash
speccy install --update
speccy install --update --dry-run
speccy install --update --yes
speccy install --update --force
```

`--update` uses a three-way merge for each managed file. Each input is the
template rendered for the same harness target, scope, capability set, and pack
version, not the raw template source:

```text
base = rendered output from installed pack version
local = current repo file
new = rendered output from current Speccy pack version
```

Default update policy:

- Apply clean changes only after interactive confirmation.
- Preserve local-only prose edits.
- Do not overwrite conflicted files.
- In noninteractive mode, require `--yes` to write updates.
- Write conflict summaries and proposed patches under `.speccy/pack-updates/<timestamp>/` (transient output, covered by the defensive `.gitignore` backstop).
- `--dry-run` always writes nothing.
- `--check` exits nonzero when packs are missing, outdated, or conflicted.
- `--force` may overwrite managed pack files with the current template, but must never touch product source or run state.

### When to Use Speccy

Speccy should remain just another tool in the box. It is useful when the cost of being wrong is higher than the cost of writing down intent and evidence.

Use Speccy when:

- The request is ambiguous enough that implementation could drift.
- The work spans multiple files, systems, UI flows, or services.
- The change has user-visible behavior.
- The change touches auth, billing, data loss, migrations, security, APIs, or infrastructure.
- The user wants evidence, a fresh-context verifier, resumability, or a compact review packet.
- Repair loops or validation failures are expected.
- The task is too large to comfortably hold in one normal agent turn.

Do not use Speccy when the overhead is larger than the risk:

- Fixing a typo.
- Renaming a local variable.
- Updating copy in one component.
- Running a formatter.
- Making a tiny obvious CSS tweak.
- Asking the agent to inspect or explain something interactively.

If a request is too small, the harness skill should recommend direct agent work. If it needs thought but not a full acceptance ledger and autonomous repair loop, it should recommend the harness's normal planning flow. If a request is too broad, it should suggest splitting it into an initiative with multiple specs. Speccy should make the scale recommendation explicit instead of treating every brainstorm as a spec.

### Brainstorm and Route Flow

`/speccy-brainstorm` is the optional exploration skill. It is not required — a user who already knows the scope can invoke `/speccy-plan` directly — but it is encouraged when scope or route is uncertain, and it activates by slash command or natural language. It stays exploratory and read-only while it inspects the codebase, sketches options, lists open questions, identifies possible splits, and produces a brainstorm handoff. It does not draft a spec. The handoff is not a spec, not an approved plan, and not an acceptance ledger. By default it is ephemeral chat context; Speccy persists it only if `/speccy-plan` promotes it into a spec or the user explicitly exports it.

```text
/speccy-brainstorm "add passwordless login"
  -> read-only exploration + open questions
  -> brainstorm handoff
  -> recommended route
  -> direct edit   or   regular harness plan   or   /speccy-plan (Speccy spec)   or   split into multiple specs
```

Where it can land:

- **Direct agent edit** - for small, obvious work where Speccy overhead is larger than the risk.
- **Regular harness plan** - for medium work that benefits from clarification and a plan artifact, but does not need a Speccy acceptance ledger, fresh validators, or autonomous repair loops. This uses the harness's own plan mode, not `/speccy-plan`.
- **Plan a Speccy spec** - for larger, riskier, multi-task, user-visible, or evidence-sensitive work. Invoking `/speccy-plan` creates the draft spec and acceptance ledger, then stops at the spec card.
- **Split into multiple specs** - when the request is too broad, propose an initiative with multiple specs.

The brainstorm handoff should show one recommended route, with alternatives secondary. It should also include a scope rating so the user understands why Speccy is recommending direct work, a regular plan, one spec, or multiple specs. Example:

```text
Scope: medium
Recommended route: regular harness planning
Confidence: medium
Reason: medium complexity, low risk, and no need for an acceptance ledger or autonomous repair loop.
Next action: Continue from the Speccy brainstorm handoff above in the active harness's normal planning flow. Produce a normal harness plan only; do not create a Speccy spec or acceptance ledger yet.

Alternatives: direct edit, plan a Speccy spec (/speccy-plan), split into multiple specs
```

Each route recommendation should include an exact next action phrase or command. Route selection can stay conversational inside the harness; Speccy should not add per-route commands such as `/speccy-promote` unless a harness cannot support the interaction cleanly.

#### Scope Rating

The brainstorm skill should rate the request before recommending a route:

```yaml
scope_rating:
  size: tiny | small | medium | large | initiative
  recommended_route: direct_edit | harness_plan | speccy_spec | split_specs
  confidence: low | medium | high
  factors:
    evidence_ability: low | medium | high   # first question: can we articulate how this work will be validated?
    touched_areas: []
    estimated_tasks: 1
    risk_domains: []
    unknowns: []
    evidence_need: low | medium | high
    autonomy_value: low | medium | high
    split_candidates: []
```

"Can this work be evidenced?" is the first routing question, per the Factory
diagnostic: when nobody can articulate how the result will be validated, an
autonomous workflow produces roughly-correct output whose repair costs more
than doing the work by hand. Low `evidence_ability` pushes the recommendation
away from `speccy_spec` toward the harness's own planning flow even when the
work is large, and the handoff must say that this is why.

Recommended defaults:

- **Tiny -> direct edit.** One obvious local change, low risk, usually one file, obvious validation, and no durable evidence need.
- **Small -> direct edit or harness plan.** One bounded component or behavior, one to two touched areas, tests are obvious, and ambiguity is low.
- **Medium -> regular harness planning.** A normal coding-agent plan is useful, but the work can still be handled as one conversational implementation without a ledger, fresh verifier loop, or resumable SDLC record.
- **Large -> Speccy spec.** The work spans multiple tasks, files, systems, or user-visible behaviors, and benefits from explicit requirements, evidence mapping, autonomous repair, and a final review packet.
- **Initiative -> split into multiple specs.** The request contains several independently valuable outcomes, a broad migration, or product direction that would make one spec too large to review or prove coherently.
- **Low evidence-ability -> harness plan, regardless of size.** If the handoff cannot name how the result will be validated, do not recommend `speccy_spec`; the ledger would be built on unprovable requirements.

Escalate the recommendation toward Speccy when the request touches auth, billing, data loss, migrations, security, public APIs, infrastructure, compliance, production behavior, or durable product semantics. Also escalate when the user asks for evidence, resumability, review packets, autonomous repair, or multiple dependent implementation tasks.

De-escalate toward direct work when the request is a typo, copy edit, local rename, formatting pass, tiny CSS tweak, one obvious test, or interactive inspection/explanation request.

The scope rating is advisory, not identity. The user can override it, and the handoff should make the override path clear.

#### Regular Planning Handoff

When the recommended route is regular harness planning, Speccy should not create a spec, task graph, acceptance ledger, or run. This route uses the harness's own plan mode, not `/speccy-plan`. It should return an ephemeral handoff that the user can feed into the active harness's normal planning mode.

Codex handoff:

```text
Recommended route: Codex planning mode
Next action: Switch Codex into its normal planning mode, then continue from the Speccy brainstorm handoff above. Produce a normal Codex plan only; do not create a Speccy spec or acceptance ledger yet.
Structured-question tool name for rendered Codex prose: request_user_input
```

Claude Code handoff:

```text
Recommended route: Claude /plan
Next action: /plan Continue from the Speccy brainstorm handoff above. Produce a normal Claude plan only; do not create a Speccy spec or acceptance ledger yet.
Structured-question tool name for rendered Claude prose: AskUserQuestion
Alternative: switch Claude to plan mode with Shift+Tab or start Claude with `claude --permission-mode plan`, then paste the handoff.
```

Generic handoff prompt shape before target rendering:

```text
Continue from the Speccy brainstorm handoff above. Produce a normal harness plan only; do not create a Speccy spec or acceptance ledger yet. Keep the plan concise. Include open questions, implementation steps, validation steps, risks, and the point where implementation should begin.
```

The template renderer is responsible for adding harness-native wrappers such as
Claude's `/plan` command or Codex's plan-mode instruction. This keeps the
brainstorm skill useful even when the user does not want a Speccy-driven SDLC.
The handoff should be copyable, but it remains chat context unless the user asks
to export it or pass it to `/speccy-plan`.

When the user chooses to plan a Speccy spec, `/speccy-plan` treats the brainstorm handoff, prior specs, decisions, and the current user request as context, and reconciles them against the current codebase rather than carrying them forward blindly. It creates a draft, never an approved spec. Approval happens only when the human approves the spec card in prose, which `/speccy-plan` records through the controller; `/speccy-implement` later refuses to run until that approval exists.

Every run is fully autonomous by design: after spec-card approval there is no step-by-step implementation steering, and no step-steered mode exists. Autonomy does not bypass policy, permission, environment, budget, production/deployment, critical-waiver, missing-credential, or spec-gap checkpoints.

### Spec Card UX

Human planning checkpoints should default to a compact spec card instead of the full technical spec or ledger. The card should answer four user questions: what will change, what will not change, how Speccy will know it worked, and what could go wrong. It should contain enough information to approve intent, scope, risk, and proof strategy:

```text
Spec: SPEC-20260630-A7F4 Passwordless login
Risk: high
Decision needed: approve this spec, or revise scope
On approval: spec revision -> approved  (recorded by /speccy-plan)
Approve by replying in prose, e.g. "approve" or "looks good, go"
Then, in a fresh session: /speccy-implement SPEC-20260630-A7F4

Goal:
Let users sign in through single-use magic links.

In scope:
- Request login link by email
- Token expiry and replay protection
- Expired-link UI state

Out of scope:
- OAuth
- Admin session revocation
- Email vendor migration

Plan:
1. Add token model and expiry checks.
2. Add request/consume endpoints.
3. Add UI states.
4. Add tests and fresh-context verification.

Acceptance:
R-AUTH-001 Magic link can be requested.
R-AUTH-002 Token is single-use.
R-AUTH-003 Token expires after 15 minutes.
R-AUTH-004 Expired token does not create session.

Prior context:
- Carry forward prior decision to store magic-link tokens hashed.
- Prior file paths appear stale; current auth code moved under src/server/auth.

Main risks:
- Email delivery may need staging or production validation.
```

The spec card should make the approval boundary unmistakable: the human approves the card in prose, and `/speccy-plan` records that approval through the controller, moving the spec revision to `approved`. Approval is required and always explicit; there is no auto-approve. `/speccy-implement` then runs against the approved revision and exits early if the revision is not approved, so it is safe to run in a fresh, cleared session. The card should show one recommended next action first, with alternatives secondary: approve, revise spec, split into multiple specs, use the harness's regular plan mode, cancel, or open the full spec/ledger. The full ledger remains available for power users and high-risk review, but it should not be the default checkpoint surface.

The spec-card approval is mandatory for every spec, regardless of risk. It is the single pre-implementation gate. Higher risk raises the evidence bar inside the same card and ledger rather than adding another approval step; the card simply carries more detail, such as the full task list and flagged destructive steps, so the human approves with the right information.

### Harness Skills

Speccy installs the harness entry skills below. Each is invocable as an explicit slash command and by natural-language fallback. Brainstorm is optional; planning, implementation, and shipping are the load-bearing handoffs. Spec-card approval is an explicit prose act recorded through the controller, not a side effect of invoking the next skill. Every other checkpoint copy must still state its effect explicitly.

- **`/speccy-brainstorm <intent>`** - optional exploration: inspect the repo read-only, clarify open questions, rate scope, identify scale, and produce a brainstorm handoff with a recommended route. It does not draft a spec, and it is skippable when scope is already clear. Natural language: "brainstorm passwordless login."
- **`/speccy-plan <intent | handoff>`** - decompose intent into a draft spec, task graph, and acceptance ledger, run a one-time pass to resolve contradictions and reconcile prior context, then present the spec card. On the human's prose approval it records the approval through the controller, moving the spec revision to `approved`. It creates a draft, never an approved spec, until that prose approval. Distinct from the harness's own plan mode. Natural language: "plan passwordless login as a Speccy spec."
- **`/speccy-implement <spec>`** - run against an approved spec revision: serial task implement-and-review rounds, then the holistic run-gate validation and drift-correction loop. Ends in `verified` on success or `escalated` on a spec/evidence/policy gap. It exits early if the spec revision is not `approved`, and should usually be run in a fresh, cleared session for clean implementation context. Natural language: "implement SPEC-20260630-A7F4."
- **`/speccy-ship <spec>`** — open the pull request and move the run to `submitted`. Natural language: "ship the passwordless login spec."

Rules:

- The slash command is the documented, deterministic entry; natural language is a convenience, not the contract.
- The spec argument accepts a full `SPEC-...` reference or a search selector, and is inferred when the current spec is unambiguous.
- Spec-card approval is a prose act recorded by `/speccy-plan` through the controller; it is required and always explicit.
- `/speccy-implement` is gated on the approved revision and exits early otherwise, so approval survives across sessions and implementation can start cold.
- Do not add per-control skills such as `/speccy-approve`, `/speccy-repair`, or `/speccy-waive`. Approval is recorded prose inside `/speccy-plan`, repair is autonomous, and amendment and waivers are conversational.
- Acceptance has no skill: `submitted -> landed` happens through the `speccy accept` CLI command.

### In-Harness Flow

Work moves through the skills, pausing at each human handoff:

```text
/speccy-brainstorm "add passwordless login"   (optional)
  -> brainstorm handoff + recommended route
  -> human chooses direct edit, regular harness plan, Speccy spec, or split

/speccy-plan "add passwordless login"          (or continues from the handoff)
  -> draft spec + task graph + acceptance ledger
  -> one-time contradiction + prior-context reconcile pass
  -> compact spec card
  -> human reviews and approves in prose
       -> /speccy-plan records approval -> spec revision = approved

/speccy-implement SPEC-20260630-A7F4          (fresh session recommended)
  -> exits early unless the revision is approved
  -> creates run against approved revision
  -> serial task implement-and-review rounds
  -> holistic run-gate validation + drift correction
  -> ends verified, or escalated on a spec/evidence/policy gap
  -> human reviews the verified summary, or amends the gap and re-runs `/speccy-implement`

/speccy-ship SPEC-20260630-A7F4               (invocation = ship approval)
  -> opens the PR, run -> submitted
  -> PR merged normally -> human runs speccy accept -> landed
```

Throughout, each skill calls the local Speccy controller for state, packets, and evidence, and `/speccy-implement` sequences its loop by repeatedly asking `speccy ctl run next` for the next deterministic step. The controller stays deterministic and never launches an LLM. Fresh-context validators run through the active harness. `/speccy-implement` always runs its full loop without step-by-step implementation steering; it stops only at `escalated` or a policy/environment gate.

### CLI/Admin Flow

The CLI remains useful for installation, status, export, and deterministic controller integration. It should not expose the internal SDLC as a sequence of public commands. Humans should not have to run acceptance/evidence/repair phases by hand, and no CLI command should call an LLM or launch an AI harness.

Common human commands:

```bash
speccy install
speccy install --target codex
speccy install --update --dry-run
speccy doctor
speccy new "Add passwordless login"
speccy list
speccy list --query passwordless
speccy status
speccy review
speccy accept
speccy archive
speccy cancel
speccy export review
```

Advanced/admin commands:

```bash
speccy export spec
speccy export run-bundle --redact
```

Command semantics:

- `install` installs, repairs, checks, or updates repo-local harness packs by default. A future `--user` flag can switch to machine-global scope.
- `doctor` checks the local controller, harness install, and optional MCP wiring if enabled.
- `new` records plain engineering intent and creates deterministic draft-spec state when the user is outside an installed harness. It may print the next in-harness instruction or a controller packet reference, but it must not create a run, draft the complete spec by calling an LLM, or launch a harness.
- `list` shows active specs in the current workspace and can filter them with `--query`; it is the human discovery path for choosing a spec without typing an opaque reference. With `--json` it is also the selector-resolution path for installed skills: they resolve a user's free text to a full `SPEC-...` reference before calling `ctl` operations, which take exact references only.
- `status`, `review`, and `cancel` manage the current spec/run when the user is outside the harness. Resuming is not a CLI action; a fresh harness session re-enters via `/speccy-implement <spec>`.
- `accept` closes out a `submitted` run as a human assertion that the change landed, with optional `--pr <url>` and `--note "<text>"` provenance. MVP does no merge detection.
- `archive` marks an `accepted` spec archived so it leaves the active list. The landed run remains `landed` in run history.
- `export review` produces the normal human review artifact.
- `export spec` and `export run-bundle` are advanced paths for audits, diagnostics, and custom harness integrations.
- Full planning, repair, and verification happen through the installed Speccy skills/agents inside Codex or Claude Code.

Internal controller operations still exist, but they are tool calls used by the harness pack, not ordinary human-facing workflow commands.

`speccy list` should default to active specs in the current workspace: drafts, approved specs, specs with active runs, escalated specs, specs awaiting review, or repairable validation failures. Accepted, superseded, obsolete, and archived specs should be hidden unless the user passes an explicit flag such as `--all`, `--status accepted`, or `--archived`.

`--query` should apply the same selector matching used by commands such as `speccy review passwordless`, but without taking an action. This lets users preview which specs would match a natural selector:

```bash
speccy list --query passwordless
speccy list --query "auth expiry"
speccy list --all --query login
speccy list --status escalated
```

Example output:

```text
Active specs matching "passwordless":

1  SPEC-20260630-A7F4  Passwordless login          escalated
2  SPEC-20260702-C91B  Passwordless login repair   draft

Use: speccy review SPEC-20260630-A7F4
```

### Review UX

The human sees a run only at its two endpoints, and the surface differs by outcome.

A **verified** run produces a compact summary the human reviews before shipping, not a control panel:

```text
Spec   SPEC-20260630-A7F4  Passwordless login      Risk: high
Result verified — ready to ship
Recommended next action: /speccy-ship SPEC-20260630-A7F4

Requirements (11)
  Proven          9
  Accepted risk   2   1 waived · 1 review-passed with residual risk

Accepted risk
  R-SEC-002    waived    Email enumeration mitigated   — "constant-time deferred, tracked in follow-up"
  R-EMAIL-001  review_passed   Email delivery integration reviewed — staging send not run locally

Changed  9 files  +412 -38     3 tasks · 2 repair rounds
Evidence + full diff:  speccy review --evidence
```

The first screen should stay compact and decision-oriented:

- What changed
- Acceptance status
- Evidence summary
- Findings and residual risk
- Drift from the approved spec
- Recommended next action

Requirement statuses collapse into the three human status buckets defined in `TERMINOLOGY.md` ("Human Status Bucket") — **Proven**, **Accepted risk**, and **Needs you** — with the precise status kept as an inline tag on drill-down. Proven is collapsed to a count and never enumerated.

A verified run has an empty **Needs you** bucket by construction. A fixable failure never waits behind a button; it loops autonomously until it is proven or the run escalates. So the summary is a result, not a menu, and it carries no approve/reject/repair/waive controls.

An **escalated** run produces the escalation packet instead: scoped to the one requirement the run could not satisfy or prove, ending in a single question the human answers in prose. See "Escalation Packet."

The two decisions a human actually makes:

- **Ship it.** Invoke `/speccy-ship` to open the PR and move the run to `submitted`. The PR is merged normally, and the human records the merge with `speccy accept`, moving the run to `landed`.
- **Send it back.** Describe what is wrong in prose. The harness folds it into a spec amendment and re-runs `/speccy-implement`. There is no `speccy reject`; feedback is conversational.

Post-verification feedback has two routes:

- **Minor implementation feedback** stays in the same run or PR when it does not change scope, requirements, or risk.
- **Scope or requirement feedback** creates a spec amendment and a new run, because the definition of done changed.

The full evidence, ledger, command logs, validator findings, and decision records remain available through `speccy review --evidence`, one drill-down deeper. The first screen summarizes; the human opens evidence only to audit a requirement or a high-risk waiver.

## Use Cases and Scenarios

### 1. Brownfield Feature in a Real Codebase

User asks for a feature in an existing app. After the user chooses the Speccy route, the tool inspects patterns, drafts a spec, maps tasks to requirement evidence, and serially implements. Browser and test validators confirm behavior.

Edge cases:

- The app cannot run locally.
- Tests are flaky.
- Existing patterns conflict.
- Authentication requires external credentials.

Expected behavior:

- Mark environment-dependent requirements blocked.
- Ask for missing setup only at the gate.
- Avoid pretending unproven UI behavior passed.

### 2. Legacy Migration

User asks to migrate a package, framework version, or API surface.

Edge cases:

- Global dependency changes affect unrelated tests.
- Generated lockfiles are huge.
- Multiple packages in a monorepo depend on old behavior.

Expected behavior:

- Break into serial package/task groups.
- Require regression tests.
- Use fresh validators to review compatibility risk.

### 3. Greenfield Prototype with Production Discipline

User wants a new app. The tool can generate a complete spec, choose simple validation methods, and build quickly.

Edge cases:

- User asks for vague design quality.
- The acceptance ledger overfits implementation details.

Expected behavior:

- Keep MVP scope tight.
- Validate observable behavior.
- Avoid excessive architecture.

### 4. CI Failure Repair

User points the tool at failing CI logs.

Expected behavior:

- Ingest logs as evidence.
- Draft a narrow repair spec.
- Make the smallest change.
- Prove by rerunning targeted tests.

### 5. Custom Pi-Based Harness

User has a local small model or custom coding harness on a Raspberry Pi.

Expected behavior:

- Implement a small Speccy client inside the harness that calls `speccy ctl ... --json`, `speccy rpc`, or eventually `speccy mcp`.
- Advertise limited capabilities in harness-side role instructions or project policy.
- Let the harness claim only roles that fit the model/hardware.
- Use stronger hosted validators only when the team configures them inside the harness; Speccy does not launch them.

### 6. Regulated or Security-Sensitive Repo

User cannot allow repo-local agent config or broad network access.

Expected behavior:

- Store run state externally.
- Run with read/write boundaries.
- Deny secret reads.
- Keep network disabled unless a validator explicitly requires it.
- Produce an audit packet.

### 7. Long-Running Run Resume

The process dies midway through validation.

Expected behavior:

- The user starts a fresh harness session and invokes `/speccy-implement <spec>`. Speccy makes no outbound calls, so it never reattaches to or relaunches a harness session itself.
- The skill calls `run next`; the controller clears the dead session's expired lease, reads task statuses, round counters, and the last git snapshot, and returns the exact next directive.
- Uncommitted workspace changes are attributed to the interrupted in-flight task and surfaced as context, not silently discarded.
- Verification work restarts as fresh-context validators from stored state; no transcript memory is assumed.

### 8. Human Rejects the Plan

User rejects the plan after spec/design draft.

Expected behavior:

- Record rejection reason.
- Let planner revise.
- Do not implement until the gate is approved.

### 9. Worker Drifts from Spec

Worker implements adjacent cleanup or broad refactor.

Expected behavior:

- Detect drift through diff scope and handoff comparison.
- Ask validator to classify risk.
- Either revert via repair task, ask human, or split into a separate suggested spec.

### 10. Validation Is Not Mechanically Possible

User asks for subjective UX polish or production-only behavior.

Expected behavior:

- Convert what can be evidenced into concrete requirements.
- Mark subjective or production-only requirements as human/prod-gated.
- Do not mark them satisfied without evidence.

## Edge Cases to Design For

- Repo is not a git repository.
- Dirty worktree before a run starts.
- User edits files during a run.
- Harness changes files outside assigned task.
- Merge conflict after worker task.
- Dependency install requires network approval.
- Tool hits rate limits.
- Harness crashes or returns malformed output.
- Worker omits handoff fields.
- Validator disagrees with another validator.
- Validation command is flaky.
- UI screenshot differs by platform.
- Browser automation cannot pass bot-protected auth.
- External service credentials are missing.
- Long command hangs.
- Agent attempts destructive command.
- Generated code introduces hallucinated packages.
- Model vendor outage.
- Token budget is exceeded.
- Spec changes mid-run.
- A requirement becomes obsolete.
- Human waives a critical requirement.
- Multi-repo changes require synchronized commits.
- Binary assets are required.
- Secrets appear in logs.

## Security and Permissions

The default permission stance should be conservative:

- Start read-only during intake and planning.
- Move to workspace-write only during implementation.
- Use no network unless required.
- Require explicit approval for dependency installation, external API calls, production credentials, destructive filesystem actions, or deployment.
- Redact secrets from run logs.
- Store transcripts and evidence locally by default.
- Do not send full repo context to validators unless necessary.
- Let each harness integration declare its sandbox capabilities and limits through policy or controller metadata.

## Data and Observability

Track:

- Run duration.
- Tokens/cost by role and harness.
- Task success/failure.
- Task review round count.
- Requirements passed, review-passed, failed, vacuous, blocked, unproven, and
  waived.
- Task-scoped repair round count.
- Run-level repair loop count.
- Capability-escalation give-up events, with the requirement IDs that triggered them.
- Commands run and exit codes.
- Files touched per task.
- Drift events.
- Human gate decisions.
- Validator disagreement.

These metrics should feed both local improvement and product evaluation.

## MVP Proposal

MVP should be intentionally narrow. One scope decision is deliberately not narrow (2026-07-01): Claude Code and Codex are both first-class MVP harnesses. Shipping two targets forces the template renderer's conditional exports to be real from day one rather than speculative single-target code. Brainstorm remains optional by design.

The MVP list:

1. Local Speccy controller with run store external to the target repo.
2. Built-in harness-aware template renderer with shared partials, Codex/Claude
   conditional exports, strict variables, and golden render tests.
3. Repo-local Codex install pack with the entry skills (brainstorm, plan, implement, ship) plus internal role prompts for planning, implementation, review/verification, and repair.
4. Repo-local Claude Code install pack with the same entry skills plus subagent definitions for the planner, worker, reviewer, validator, and repair roles.
5. Machine-oriented `speccy ctl ... --json` CLI surface that installed skills call for state transitions, evidence tools, and `run next` loop driving.
6. Deterministic inbound custom-harness contract through `speccy ctl ... --json`; no Speccy-launched harness adapters.
7. Deterministic planning packet generation plus harness-authored intake observations and candidate spec drafts.
8. Spec/design/task/acceptance-ledger generation.
9. Compact human gates surfaced inside the active harness.
10. Serial implementation tasks.
11. Evidence collection for command output, diffs, review findings, and manual waivers.
12. Acceptance ledger completeness enforced structurally: the draft lint requires an evidence request per requirement before approval, and the `verified` gate requires every requirement resolved.
13. Speccy verifier role/agent for semantic scenario review and adversarial evidence review, invoked inside `/speccy-implement`.
14. Markdown review packet.

Avoid in MVP:

- Web UI.
- Parallel write worktrees.
- Production deployment.
- Hosted/shared pack registry. Built-in repo-local pack generation is in scope.
- Multi-repo specs/runs.
- Complex marketplace/plugin distribution.
- Any `speccy` command that launches an LLM, coding agent, or harness.
- `speccy run --adapter`, `speccy adapters`, `codex exec`, `claude -p`, SDK, or generic prompt-runner integration.

## Later Capabilities

- Automatic merge detection — git-native ancestry checks, squash-merge heuristics, and an optional configurable host probe — so `submitted -> landed` can be recorded without a manual `speccy accept`. Cut from MVP because it is an external-integration convenience, not core to the loop.
- Optional MCP server exposing `speccy` to clients where MCP is worth the token overhead.
- User-level skills/commands for Codex and Claude Code through `speccy install --user`.
- Importers for OpenSpec, Spec Kit, Kiro-style specs, GSD Core, and other repo-local spec formats.
- Exporters that write `speccy` specs and acceptance ledgers into those formats when a team explicitly wants repo-local artifacts.
- Web dashboard for long-running runs.
- Worktree-based parallel experiments.
- Browser validator integration.
- GitHub issue/PR integration.
- Deterministic CI checks for specs, ledgers, review packets, and pack freshness.
- Policy packs for regulated environments.
- Optional team-shared run store for enterprise/audit use, only after no-server review packets and run bundles prove insufficient.
- If any mutable state ever becomes git-visible (for example, a team-shared mode committing state snapshots), it must be append-only with a union-by-event-id git merge driver; replace-style merges of state files silently lose data (see `runtime-state-storage-survey.md` on OpenSpec vs Spec Kitty).
- Model routing and budget optimizer.
- Inbound Agent2Agent-compatible bridge owned by an external harness, if a team proves it adds value without moving orchestration state out of Speccy.
- Reusable evidence templates, if real usage proves they reduce friction.
- Worker self-editing skills for in-run continuous learning, added only after the single-spec MVP shape is proven. If run horizons ever extend beyond hours, per-repo skill self-evolution becomes a prerequisite per the Factory analysis, not an enhancement.

## Open Questions

1. **State location — resolved 2026-07-01.** Runtime state lives in `~/.speccy/` only. Repo `.speccy/` holds exactly `project.yaml` and `pack-lock.yaml`; policy/role/evidence prose renders into harness packs. See "Storage Model."
2. **Repo artifact export:** Which artifacts should be easiest to export: spec, acceptance ledger, review packet, lessons learned, or all of them?
3. **Artifact shape:** What is the smallest useful spec draft and acceptance ledger shape? Do not lock public format compatibility until MVP usage proves it.
4. **No-server sharing:** Are review packets, compact snapshots, rerun commands, and optional redacted run bundles enough for team use before considering any shared run store?
5. **Spec interop:** Which external spec formats should be first-class import targets: OpenSpec, Spec Kit, Kiro, GSD Core, Spec Kitty, or a generic markdown mapper?
6. **Harness output reliability — resolved 2026-07-02.** Strict schema validation on every record operation, with bounded repair: the controller rejects the payload and returns structured lint errors, the skill retries with a focused fix up to a policy cap (default 3), then the run fails closed to `escalated`. No lenient coercion.
7. **Vacuity threshold:** What minimum anti-vacuity evidence is required before the verifier can mark a high-priority requirement as `passed`?
8. **Scenario evidence:** How much should Speccy help convert `given/when/then` prose into evidence requests versus delegating that to harness agents?
9. **Custom harness integration:** Are `speccy ctl ... --json` calls enough for custom harnesses, or should `speccy rpc`/`speccy mcp` be supported earlier?
10. **Human gates:** How much editing should happen inside `speccy` versus opening `$EDITOR`?
11. **Review packet format:** Markdown only, JSON plus Markdown, or an HTML report?
12. **Lessons learned:** How can the system accumulate project learning without leaking operational state or affecting product-code/build/runtime footprint?
13. **Parallel writes — resolved 2026-07-02.** Explicitly out of scope for MVP: writes are serial, enforced by the run lease. Worktree-based parallel writes remain a later capability; the enabling threshold will be revisited with real usage data.
14. **Validator diversity:** Should implementation and validation default to different harnesses/models when available?
15. **Cost controls — resolved 2026-07-02.** Fail closed at the cap. Resource caps — repair rounds, plus optional task-count and wall-clock caps in `.speccy/project.yaml` — park the run at an `escalated` policy gate; the human raises the cap or cancels, and the same run resumes. Speccy makes no LLM calls and cannot meter tokens; token budgets belong to the harness.
16. **Dirty worktrees — resolved 2026-07-02.** `run start` refuses a dirty worktree; no run is created until the workspace is clean. This keeps the resume invariant sound: once a run exists, any uncommitted diff is attributable to the run's in-flight task.
17. **No-git projects — resolved 2026-07-02.** Not supported, in MVP or later. Resume and evidence baselines depend on git snapshots and `baseline_commit`; a workspace must be a git repository or a subtree of one. Speccy refuses non-git directories with a clear error.
18. **Security model:** How should secret redaction and deny-read rules work across harnesses with different sandbox systems?
19. **Production validation:** How should the tool prove behavior that only exists in deployed environments?
20. **Spec mutation — resolved 2026-07-02.** Nobody mutates an approved revision's ledger in place. Requirement statements and evidence requests are frozen at approval; agents may only propose draft patches; human prose approval creates a new revision and a new run; verifiers change requirement status only, through evidence operations.
21. **Long-term storage:** How long should transcripts/evidence be retained?
22. **Team mode:** When multiple humans review gates, what is the approval policy?
23. **License/package strategy — resolved 2026-07-02 (language and engine).** Rust, shipped as a single static `speccy` binary. Templating: `minijinja` (Jinja2 syntax) is the intended engine, pending verification against the renderer requirements. Distribution channels and license remain open.
24. **Name:** Is `speccy` the right name, or should the tool use a more explicit name around specs/evidence?
25. **Escalated-run reconciliation — resolved 2026-07-02.** Snapshot and reconcile: at escalation the controller commits any uncommitted in-flight diff as a labeled escalation snapshot, and the superseding run starts on the same branch seeded with the prior run's summary, reconciling rather than redoing. Rolling back to the run baseline remains the human's explicit fallback at the gate.

## Recommended Next Step

Build a walking skeleton:

1. `speccy install` creates or repairs repo-local harness packs by auto-detecting `.codex`, `.claude`, and `.agents`; `--target` overrides detection.
2. Local run store external to the target repo.
3. `speccy ctl ... --json` exposes controller operations to those packs, including `run next` loop driving; `speccy rpc` remains optional for custom harness tests.
4. A minimal built-in template renderer with shared role partials and target
   conditionals for Codex and Claude Code.
5. Acceptance ledger generation.
6. One serial task.
7. One verifier evidence-collection pass.
8. One anti-vacuity reviewer question for generated tests or review-only evidence.
9. One in-harness Speccy verifier role call for semantic scenario/evidence review.
10. Review packet output.

Once that loop works end to end inside Codex and Claude Code, add only deterministic integration surfaces such as `speccy rpc` or `speccy mcp` if custom harnesses need them. Do not add Speccy-launched agent runners.
