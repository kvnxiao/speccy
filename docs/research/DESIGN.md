# Design: Spec-Driven Multi-Agent Orchestration Tool

Status: authoritative
Date: 2026-07-03

This document proposes a design for a modular spec-driven orchestration tool that coordinates existing coding-agent harnesses. The design is intentionally open-ended and includes unresolved questions.

Working name in this document: `speccy`.

## Product Thesis

`speccy` is a small higher-layer spec-driven run controller for coding agents.

It does not write code itself and does not replace Claude Code, Codex, Cursor, Copilot, Jules, OpenHands, or a custom Pi-based harness. It installs into Codex and Claude Code as harness-native skills/agents, turns an engineering request into a lightweight spec, acceptance ledger, task sequence, run state, and review packet, then delegates implementation and validation through the user's active harness.

The core product promise:

> Spend more attention up front on intent and evidence, spend less attention babysitting the agent while it implements, and receive a compact review packet showing what changed, what was tested, what drifted, and what still needs human judgment.

## Design Principles

1. **Harness-neutral by construction.** The run controller exposes stable controller operations and install packs, not vendor-specific execution paths.
2. **Zero product-code footprint.** Speccy must not affect product source, the build graph, deployed artifacts, runtime dependencies, or production behavior. Repo-local harness packs are workflow artifacts and may be committed; runtime run state remains external or ignored by default. Shipped file contents must also carry no Speccy provenance (see "Provenance Hygiene").
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
      - id: E1
        kind: review
        note: "Diff only changes timestamp formatting path."
    status: pending
  - id: R2
    statement: "Timestamps are formatted as ISO-8601 UTC values."
    evidence:
      - id: E1
        kind: command
        command: "npm test -- csv-export"
    status: pending
```

The `goal`, `scope`, and `risk` here are denormalized from the spec revision for readability. The spec revision stays their source of truth; the ledger's own concern is the `requirements` rows and their evidence.

This shape is intentionally provisional for the MVP. Speccy should not promise a stable public artifact format until real usage proves the right boundaries.

Baseline rules:

- Every approved spec has an acceptance ledger before implementation starts.
- Every requirement has a stable local ID, a plain-English statement, and one or more evidence requests: command/test output, browser/API observation, file/diff review, harness review, manual evidence, explicit waiver, or blocked status. Evidence requests are an array, each with a stable ID unique within its requirement (`E1`, `E2`; qualified as `R-AUTH-001.E1`); collected evidence records reference the request they satisfy, so a requirement with several declared proofs stays auditable per proof rather than per requirement (shapes in `SCHEMAS.md`).
- For `kind: command` evidence, the controller executes the command: `speccy ctl evidence collect` runs it and records exit code, stdout, stderr, and a content hash. `evidence record` refuses agent-pasted output for that kind, so `passed` on command evidence never rests on a transcript claim. Trust narrows to review, browser, and manual kinds, which the risk tiers already treat as weaker.
- Command execution policy: the declared command string runs through the platform shell (`sh -c`; `cmd /c` on Windows) in the workspace root, under the `evidence.command_timeout_seconds` and `evidence.command_output_max_bytes` caps from `.speccy/project.yaml`, with known-secret environment values scrubbed from stored output (full redaction model: Q18 in `OPEN-ITEMS.md`). Command executions serialize on the workspace command lock (see "Run Lease and Concurrent Writers"), and the controller records worktree dirty-state before and after the run so command-induced changes stay attributable.
- Command allow policy: when `evidence.command_policy.allow` is set in `.speccy/project.yaml`, a declared `kind: command` string that matches no pattern is flagged by structural lint at `record-draft`/`patch-draft` — so the mismatch surfaces at spec time, where the human can fix or approve around it — and refused at `evidence collect` (`validation_failed`). Patterns match the whole declared command string (glob), never a prefix: commands execute through a shell, so `npm test && curl …` is a different whole string from `npm test` and matches nothing unless a pattern explicitly allows it. The policy is a drift guardrail — lint plus refusal against commands nobody meant to declare — not an authorization boundary; the harness sandbox remains the security boundary for whatever an approved command spawns (see "Human Gates"). Unset means any approved command may run; the spec card always shows the command strings either way (see "Spec Card UX").
- On `high` and `critical` specs, `evidence record` for `kind: browser` and `kind: api` requires a non-empty `artifact` reference — a screenshot, trace, DOM capture, or HTTP transcript stored under the run's evidence tree — and refuses prose-only records. The controller enforces presence and hashes the artifact; it cannot vouch for authenticity, but a stored artifact is inspectable at review where a transcript claim is not. At `minimal` and `standard` the artifact stays optional.
- An approved revision's ledger is immutable in place: requirement statements and evidence requests are frozen at approval. Agents may only propose draft patches; a human prose approval creates a new revision and a new run. Verifiers change requirement status only, through evidence operations.
- The final review packet includes the ledger, status, commands run, evidence links, and residual risk.
- A task cannot reach `integrated` while any linked requirement is unresolved
  (see "Requirement Resolution Rules").
- A run can become `verified` only when every requirement is resolved as
  `passed`, `review_passed`, or `waived`, subject to the tier constraints in
  "Requirement Resolution Rules". A raw `blocked` status
  remains **Needs you**; it forces an escalation or policy gate unless a human
  decision records an explicit `review_passed` judgment or waiver with residual
  risk.

Risk still matters, but it changes the burden of evidence inside the same ledger rather than introducing a separate workflow:

| Risk | Use For | Ledger Requirement | Verification Depth |
| --- | --- | --- | --- |
| Minimal | Formatting, docs, typo fixes, obvious one-line repairs, dependency metadata with no behavioral impact. | One to three requirements, evidence can be command output or focused review. | Existing relevant checks plus final packet. |
| Standard | Normal bug fixes and small features with localized blast radius. | Requirements mapped to declared evidence requests. | Verifier gathers command/test/diff/review evidence and does lightweight evidence-adequacy review for new tests. |
| High | Auth, billing, data loss, migrations, security, broad refactors, concurrency, public APIs, compliance-sensitive behavior. | Same ledger, but important requirements need stronger evidence such as negative cases, positive cases, pre-fix failure, or explicit human waiver. | Fresh-context verifier, evidence-adequacy review, and residual-risk notes on accepted-risk requirements. |
| Critical | Production safety, regulated domains, irreversible migrations, incident repair, or explicit audit needs. | Same ledger plus retained evidence, decision log, and optional redacted run bundle. | Accepted-risk confirmation gate before `verified`, stronger evidence retention, and optional external review. |

Scenario prose is allowed when useful, but it should remain in the ledger as clarification, not become a new mandatory artifact. A `given/when/then` scenario should map to one or more evidence requests:

- Command/test evidence: shell command, unit test, integration test, static analyzer, database query, or similar machine-run output.
- Browser/API evidence: a harness-driven browser or API observation with captured result.
- File/diff evidence: review of changed files, selectors, routes, migrations, or configuration.
- Harness review evidence: fresh-context harness review with structured findings.
- Manual evidence: explicit human decision.
- Blocked: the requirement cannot currently be verified.

The verifier agent should collect evidence for all of these. Speccy provides evidence tools that make some collection reproducible, such as running a command and storing exit code/stdout/stderr, but it should not force users to think in separate deterministic versus LLM-verification phases. The ledger records evidence type, collector, raw artifact reference, reviewer judgment, and residual risk.

### Requirement Resolution Rules

"Resolved" is a deterministic controller judgment. A requirement is resolved
when its status is `passed`, `review_passed`, or `waived`. The task
`integrated` gate requires every linked requirement resolved; the run
`verified` gate requires every requirement resolved.

The six requirement statuses (canonical; TERMINOLOGY names them, this section
owns them):

- `pending`: evidence not collected yet (initial status).
- `passed`: collected evidence satisfies it at the required risk depth.
- `review_passed`: review-only evidence satisfies it, residual risk recorded.
- `failed`: evidence or a finding contradicts it — including a vacuity finding,
  which is a finding reason, not its own status.
- `blocked`: no acceptable evidence can currently be collected — a missing
  environment, tool, credential, access, or dependency.
- `waived`: a human explicitly accepted the risk at a gate.

The risk tier adds constraints inside the resolution rule rather than changing
its shape:

| Tier | `passed` | `review_passed` | `waived` |
| --- | --- | --- | --- |
| minimal | resolves | resolves | resolves |
| standard | resolves | resolves | resolves |
| high | resolves | resolves, requires a recorded `residual_risk` note | resolves, requires a recorded `residual_risk` note |
| critical | resolves | requires human confirmation at a gate | requires human confirmation at a gate |

On a `critical` spec, any requirement resolved by `review_passed` or `waived`
parks the run at an `await_human_gate` directive ("confirm accepted risk")
before `verifying` can complete; the confirmation is recorded through
`run record-decision`. This is the only tier-added gate (see "Human Gates").
The gate is the last stop before `verified`: it fires only once the run-level
review is recorded, every requirement is resolved, and no blocking finding
remains. A run still holding a `failed` requirement or an unresolved blocking
finding takes the run-repair or escalation path first — the accepted-risk gate
never pre-empts unfinished verification.

Status prerequisites, enforced by `requirement set-status`:

- `passed` requires at least one recorded evidence artifact.
- `failed` requires at least one recorded evidence artifact or finding. A
  vacuity finding — evidence that does not exercise the requirement — is a
  legitimate basis for `failed`.
- `review_passed` requires at least one recorded evidence artifact and, at
  `high` and `critical`, a `residual_risk` note.
- `blocked` requires a note naming what is missing.
- `waived` is set only through a gate decision (`run record-decision`), never
  through `requirement set-status`.

Status transitions:

- `pending` is the initial status and is never re-entered.
- Any transition out of `pending` is legal, subject to the prerequisites
  above.
- Transitions between `passed`, `review_passed`, `failed`, and `blocked` are
  legal when justified by new evidence or findings; final validation may
  demote a task-level `passed`.
- `waived` is terminal for the run. A later revision starts a fresh ledger.

### Verification Ownership

The dependency should be inverted: a harness verification agent collects evidence for the acceptance ledger, and Speccy provides state, evidence capture tools, and evidence recording. The Speccy CLI is not an LLM and should not pretend to semantically judge scenario prose.

Practical meaning:

- Acceptance linting, evidence capture, evidence recording, and requirement status updates should be internal controller operations, not public SDLC-shaped CLI commands.
- Verification is a phase of `/speccy-implement`, not a separate entry skill. It runs as the internal verifier role inside the implement loop.
- The `/speccy-implement` loop fetches `packet verification`, dispatches the verifier role, and records the verifier's returned status payload through `requirement set-status` with the live run lease.
- For `kind: command`, the controller is the collector: `evidence collect` executes the declared command and records exit code/stdout/stderr/hash itself, and `evidence record` rejects agent-supplied output for that kind. The verifier still decides when to collect and judges adequacy; it never transcribes command results.
- The verifier role is responsible for collecting evidence, interpreting semantic scenario prose, reviewing evidence adequacy, and performing adversarial vacuity review.
- The verifier role should call Speccy tools to collect command/test/diff evidence when useful, record evidence, and write structured reviewer findings. It should not take the run lease; aggregation and status writes stay with the lease-holding loop driver.
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
- Requirement-to-test traceability: the run store maps every generated test to the requirement it proves through the evidence artifacts recorded against that requirement. Traceability never goes into test names, comments, or any product file; that would ship process provenance (see "Provenance Hygiene").
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
      - id: E1
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

Each runtime task carries a task status and a controller-owned round counter,
so an interrupted session can resume mid-task. The four statuses:

- `queued`: not started.
- `building`: an implementer owns the task; a repair round re-enters here with
  the round counter incremented.
- `in_review`: a handoff is recorded and fresh-context reviewers check the
  linked requirements.
- `integrated`: linked requirements resolved for the tier; the controller
  records a git snapshot commit for the task.

When a task enters `building`, the controller records `baseline_commit` — the workspace git HEAD at claim time — on the task and preserves it across resume, so every diff, review, and evidence check has a stable baseline even after a crash mid-round.

When a task reaches `integrated`, the controller snapshots the workspace as a git commit on the run's working branch and records the commit SHA on the task. Snapshots make resume deterministic: any uncommitted diff belongs to the current in-flight task, so `run next` can tell "resume a partially built task" from "dispatch a fresh task" by comparing task status, round counter, and workspace dirtiness against the last snapshot.

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

Per-role model selection lives in the skill/subagent frontmatter of the install pack, so the implementation seat and the verification seat can use different models or providers where the harness supports it. Reviewer personas add per-persona model selection through the roster config (see "Reviewer Personas").

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
- `action`: one of the closed directive vocabulary: `claim_task`,
  `dispatch_worker`, `dispatch_verifier`, `await_human_gate`, or `halt`. A
  repair round is `dispatch_worker` with `round.current > 1`; run-level
  validation is `dispatch_verifier` with `round.scope: run`; an escalation is
  `await_human_gate` with `subject.gate: escalation`. `halt` means no
  autonomous action exists for this run — it is `cancelled`, `landed`, or
  `submitted` awaiting an external merge — and `reason` says which. The
  vocabulary is versioned with the controller protocol; a skill that sees an
  unknown action stops and surfaces it.
- `subject`: the task, requirement, or gate the directive applies to.
- `packet_with`: the packet operation to run before performing the action —
  `packet task` for `dispatch_worker`, `packet verification` for
  `dispatch_verifier`, `packet review` or `packet escalation` for
  `await_human_gate` (by gate) — or null when no packet is needed.
- `round`: for repair directives, the controller-owned counter and its
  policy-configured cap, such as `{ "current": 2, "max": 3, "scope": "task" }`.
  Per-task repair rounds and run-level review rounds have separate policy
  values. The orchestrating agent reports what the controller said — "starting
  repair round 2 of 3" — and never counts rounds itself.
- `record_with`: the controller operation that must record the outcome, so the
  loop closes deterministically. Human decision points have more than one
  legal outcome by design, so on `await_human_gate` directives `record_with`
  is only the gate's default recorder, and a `gate_answers` field is the
  authoritative map: every legal answer with its recording operation
  (`{type, record_with}`, shape in `SCHEMAS.md`). The skill routes the human's
  prose to a listed answer and never guesses which operation records which
  decision.
- `reason`: a compact explanation the skill can surface verbatim in status
  updates.

Rules:

- `run next` is idempotent. Calling it again without recording a result
  returns the same directive. Idempotency is semantic: every directive field
  is identical, while the `lease` block may differ because each call renews
  the lease expiry.
- The harness must not infer the next step from transcript memory. After every
  recorded result, it asks the controller again.
- `await_human_gate` stops scheduling and surfaces the gate's packet. An
  unrecognized directive or a controller error also stops the loop and
  surfaces the error; the prose never guesses.

`run next` is also the single mutation point for derived state. Before
returning a directive it clears expired leases and applies every transition
that has no recording operation. Task transitions: `in_review -> building`
when the recorded review leaves a linked requirement failed or a blocking
finding unresolved with repair rounds remaining (the round counter
increments), and `in_review -> integrated` — including the task's snapshot
commit — once every linked requirement is resolved (see "Requirement
Resolution Rules"). The failure and success halves of review aggregation are
the same judgment, so both derive here: `requirement set-status` records
requirement statuses and never moves the task. `task record-handoff` records
the task straight to `in_review`; there is no separate reviewable or
needs_repair holding state. Run transitions:
`implementing -> verifying` when every task is `integrated`,
`verifying -> verified` when every requirement is resolved and any
critical-tier confirmation gate is answered, and `-> escalated` on cap
exhaustion, a blocked requirement, or a resource cap — each committing the
in-flight diff as a labeled escalation snapshot — and on an out-of-band
commit, which escalates but takes **no** snapshot: a snapshot commit there
would bury or misattribute the human's out-of-band commit and worktree edits
(see "Run Branch and Snapshot Policy").
`run start` opens the run directly in `implementing`. The run-level
review-round counter counts review rounds *opened* — each derived
`-> verifying` transition — so a gate resume, which re-enters `verifying` or
`implementing` through a distinct resume event rather than a transition, does
not open a round and a run parked at its cap resumes within the cap.
Idempotency is over
settled state: once derived transitions apply, repeated calls return the same
directive without re-applying them.

Derived transitions are reported, not silent: every `run next` response
carries an `applied_transitions` array listing exactly the transitions that
call applied, with the snapshot commit SHA on entries that created one (shape
in `SCHEMAS.md`). The skill can narrate them ("T1 integrated at 9c2f1ab")
without inferring state changes from the `reason` string, and the event log
gains nothing new — the field echoes what was already applied. Like `lease`,
`applied_transitions` is excluded from the idempotency comparison: repeated
calls over settled state return the same directive with an empty array.

Lease repair gets the same transparency: a call that clears an expired lease
reports it in the directive's `resume` field, with a summary of any dirty
worktree diff that resume attribution will fold into the in-flight task
(shape in `SCHEMAS.md`; behavior in "Resume and Crash Recovery"). `resume`
is per-call work like `applied_transitions` and is likewise excluded from
the idempotency comparison.

### Run Lease and Concurrent Writers

"Serial writes" is enforced, not asserted. Two `/speccy-implement` sessions on
the same run must not interleave `ctl` calls and corrupt round counting, so the
controller contract includes a run-level lease:

- `run next --agent <id>` issues or renews a lease token bound to that agent
  ID, with a 10-minute expiry (MVP default), renewed on every controller
  call. The token returns with the directive and is passed back as
  `--lease <token>` on state-mutating operations. Agent IDs are opaque
  caller-chosen strings; the packs use a `<harness>:<session>` convention.
- State-mutating operations — `task claim`, `task record-handoff`,
  `requirement set-status`, `run record-decision`, `run record-ship`, and any
  operation a `run next` directive names in `record_with` — require the live
  token, passed as `--lease <token>`. The lease is run-scoped: spec-scoped
  operations predate the run and are not lease-gated.
- A second session asking for the run gets a `lease_held` error naming the
  holder and its expiry, and stops.
- Expired leases are cleared deterministically by the following `run next`
  call; a crashed session never wedges the run.

Concurrent reviewers are the deliberate exception. A task's review phase fans
out the configured fresh-context reviewer personas (see "Reviewer Personas"),
which can complete at the same moment. Their
operations are additive, not state-mutating, so they do not take the lease:

- `finding record` and `evidence record` for non-command kinds are lease-free
  additive operations. Each finding and
  evidence artifact is written as its own file keyed by its ID (plus an
  append-only event), never appended to a shared per-task journal, so
  simultaneous completions cannot contend. Concurrent event appends serialize
  on the per-workspace store lock (see "Storage Model").
- `evidence collect` for `kind: command` executes a real command that can
  mutate caches, lockfiles, or generated files, so it is not free to
  interleave. It takes the workspace command lock — separate from the run
  lease, so verifier personas can collect without holding the lease, but only
  one command runs at a time. `--requirements` collects every `kind: command`
  request under the named requirements; optional `--requests R-AUTH-001.E1,…`
  narrows collection to specific evidence requests, so a persona can re-prove
  one artifact without re-collecting a whole requirement's evidence.
- Aggregation stays with the lease holder: after all reviewer personas report,
  the orchestrating session (holding the lease) records the resulting task
  status transition.

Findings must carry forward. When a round fails, the next round's task packet
and verification packet include the prior rounds' findings and the verifier's
rejection reasons, so a repair round starts from what was learned instead of
re-discovering the same failure.

### Reviewer Personas

A reviewer persona is a named review lens — a fresh-context subagent with its
own charter prompt and its own model selection — dispatched during review.
Personas are first-class pack citizens: each renders as a harness-native
subagent file (`.claude/agents/speccy-reviewer-<persona>.md`,
`.codex/agents/speccy-reviewer-<persona>.toml`), and the roster is
configuration, not prose convention.

Personas review at both loop scopes:

- **Task review rounds.** Every `dispatch_verifier` directive with
  `round.scope: task` names the roster; the orchestrating skill fans the
  personas out over the task diff and linked requirements. Rounds are capped
  by `caps.task_repair_rounds`.
- **Run-gate review rounds.** `dispatch_verifier` with `round.scope: run` fans
  the same roster out over the integrated whole-run diff, alongside the drift
  and requirement-coverage review. Rounds are capped by
  `caps.run_review_rounds`.

Default roster:

| Persona | Charter |
| --- | --- |
| `spec-fidelity` | Do the changes satisfy the linked requirements? Any scope drift? Is the evidence non-vacuous and adequate for the risk tier? |
| `defects` | Implementation correctness independent of the spec text: logic errors, edge cases, error handling, concurrency/races, leaks, silent failures, and meaningful performance regressions. |
| `security` | Injection, authn/authz, secret handling, unsafe defaults, dependency risk. |
| `style` | Documented conventions (`CLAUDE.md`/`AGENTS.md`, lint configs), idioms and known gotchas for the languages and frameworks in use, behavior-preserving simplification of touched code, and comment quality — including process-provenance leakage (see "Provenance Hygiene"). |

Reviewer findings use a high-signal admission bar. A persona records a finding
only when the issue is material, high-confidence, caused or exposed by the
reviewed diff, and actionable. Low-confidence guesses, unrelated pre-existing
issues, and stylistic preferences not grounded in project conventions are
omitted rather than turned into review noise; `uncertain` is reserved for
genuine human judgment gaps. Each `blocking` or `advisory` note should name the
concrete failure mode or guideline violation and the smallest plausible fix.

"Correctness" is deliberately split into `spec-fidelity` and `defects`:
the two fail independently — a change can
satisfy every requirement and still break on an untested path — and a single
combined prompt anchors on the ledger and under-hunts latent bugs. The split
also lets the two lenses use different models. Further default splits
(performance, test quality, docs) were rejected as roster bloat; teams add
them as custom personas where the repo warrants it.

A dedicated simplifier subagent is likewise rejected for the default pack. The
worker and repair roles do a final, touched-diff cleanup pass before handoff,
and the `style` persona can record simplification findings. Any resulting edit
then flows through the normal repair and review loop, preserving the single
writer path and fresh-context verification.

The roster lives in `.speccy/project.yaml` (schema in "Harness-Native Install
Packs") and is a render input: `speccy install` renders one subagent file per
persona per target, with the persona's `model` — a plain string, or a map
keyed by target when a repo renders multiple harnesses — in the rendered
frontmatter. A persona with no `model` inherits the harness default. This is
how reviewer models differ from each other and from the implementation seat:
the worker, planner, verifier, and repair roles keep their own frontmatter
`model` fields, and each persona carries its own. The rendered frontmatter
stays the harness-authoritative surface; hand edits survive `--update`
through the normal three-way merge.

Controller involvement is deterministic and minimal: the roster is echoed in
verification directives and packets so fan-out is controller-stated rather
than prose-remembered, and each recorded finding carries the persona that
produced it. The controller never judges persona prose. Persona findings and
non-command evidence record lease-free, per "Run Lease and Concurrent
Writers"; aggregation stays with the lease holder.

Risk tiers scale the roster instead of adding process: `minimal`-risk specs
collapse review to a single combined reviewer, because the full fan-out would
be heavier than the change; `standard` and above run the full configured
roster. An optional persona can carry a `min_risk` tier so it joins only
`high` or `critical` reviews.

### Repeat Review Rounds

Every review round runs the full persona roster. Skipping personas that
raised no blocking findings in the prior round was considered and rejected:
a repair diff is new code, and any scheme where
some persona never sees some shipped line trades correctness for tokens in
the wrong direction — a security regression introduced while fixing a style
finding must not survive to a later round. Output correctness outranks token
savings.

Findings and claims carry forward. Each round's verification packet includes
prior rounds' findings, the verifier's rejection reasons, and the repair
handoff's resolution claims, so no persona rediscovers a known failure. Each
round reviews the full task diff against `baseline_commit`, and persona prose
instructs each reviewer to confirm its own prior blocking findings are
actually resolved.

Scoping a re-review to only the changed slice of a repair diff — a token
optimization — is deferred to Later Capabilities. It is unproven against real
reviewer cost, full-diff re-review is correct and simple, and the roster cost
measured during dogfooding decides whether the optimization earns its
machinery (round snapshots, per-round deltas). See "Later Capabilities."

### Resume and Crash Recovery

There is no `speccy resume` command and no human resume ritual. Resume is a
controller capability: `run next` must be able to answer "what is the next
required step" for a fresh agent session at any point, including after a crash,
a killed session, context compaction, or a rate-limit abort.

Three mechanisms make that answer deterministic:

- **Task statuses and the round counter.** The runtime task graph records
  `queued | building | in_review | integrated` per task plus the
  controller-owned round counter, so the controller knows exactly which phase
  of which round was interrupted (see "Task").
- **Git snapshots at task boundaries.** Every task records `baseline_commit`
  at claim time, and every `integrated` task ends in a snapshot commit recorded
  on the task. Uncommitted workspace changes therefore belong to the current
  in-flight task: a dirty worktree with a task in `building` means "resume or
  restart this task with the partial diff as context, diffed against
  `baseline_commit`"; a clean worktree means "dispatch fresh."
- **Lease repair.** `run next` clears expired leases before answering, so a
  dead session's lease never blocks the successor.

The flow after any interruption is always the same: start a fresh harness
session, invoke `/speccy-implement` (or add a selector when needed), and the skill calls
`run next`, which replays nothing and re-derives the directive
from stored state. Mid-directive interruptions are safe because `run next`
is idempotent: a directive whose result was never recorded is simply returned
again.

Resume attribution is visible, not silent. The controller cannot tell an
agent's partial diff from edits a human made while the session was dead —
there is no recorded diff at crash time — so instead of guessing it reports:
a `run next` call that clears an expired lease carries a `resume` field
naming the cleared lease and summarizing the dirty diff against the task's
`baseline_commit` (shape in `SCHEMAS.md`), the skill echoes that summary
before dispatching — the same pattern as the approval echo — and
`speccy status` shows the same attribution for an interrupted run (see
"CLI/Admin Flow"). A human who edited during the gap stashes or commits
first: a stash removes the edits from attribution, and a commit becomes an
out-of-band commit that parks the run at the existing escalated policy gate.
A blocking adopt/stash/cancel gate on every dirty resume was rejected:
the condition it guards against is
undetectable, so it would tax the overwhelmingly common case — the worker's
own partial diff — as a permanent false positive.

### Run Branch and Snapshot Policy

The first `run start` for a spec creates the spec's run branch,
`speccy/<spec-ref-lowercased>-<slug>` (for example
`speccy/spec-20260630-a7f4-passwordless-login`), from the currently
checked-out HEAD. The base recorded is the run branch's tip *after* checkout:
for a first run that is the HEAD it was branched from; for a later run that
reuses an existing branch — after an amendment or a cancelled run — it is that
branch's tip, which already carries the earlier run's snapshots. Recording the
pre-checkout HEAD would set the base to whatever unrelated commit the user had
out, making the reused run's diff and out-of-band check span commits that are
not its own. Reusing the branch is how a superseding run reconciles the
escalation snapshot instead of redoing work. `run start` never chooses a base
branch itself: whatever the user has checked out is the branch point, and the
clean-worktree refusal is the only precondition.

Controller-created commits — task snapshots and escalation snapshots — use
the committer identity `Speccy <noreply@speccy.local>` and the message
formats `speccy: <spec-ref> <task-id> integrated (round <n>)` and
`speccy: <spec-ref> escalation snapshot`. The controller never squashes;
`/speccy-ship` prose offers a squash by default before opening the PR, so
Speccy-labeled messages stay off the mainline unless the team wants them
(see "Provenance Hygiene").

The resume invariant assumes HEAD only moves through controller snapshots
while a run is active. `run next` verifies that HEAD matches the last
recorded snapshot (or the recorded base before any snapshot exists); if a
human or another tool committed out-of-band, the run parks at an `escalated`
policy gate naming the unexpected commits, and the human decides whether to
fold them in, reset, or cancel. This escalation takes no snapshot — the
in-flight diff is left as-is, since a Speccy snapshot commit on top would bury
or misattribute the human's out-of-band work. The cap-driven escalations
(cap exhaustion, blocked requirement, resource cap) do commit the labeled
snapshot.

### Provenance Hygiene

Speccy is an ephemeral layer, and the shipped artifact must not betray it.
Product file contents — source, comments, tests, docs, config, migrations —
must never reference Speccy terminology or identifiers: no `speccy`, no
`SPEC-...` references, no requirement/run/task IDs, no ledger or round
vocabulary. The end state of the code must read as if it were implemented by
hand or through the harness's ordinary plan-and-implement flow.

The rule scopes to file contents. Deliberate workflow artifacts are exempt:
the rendered harness packs, `.speccy/`, and anything the user explicitly
exports (`speccy export` destinations such as `docs/specs/`).
Change-management metadata is a policy boundary, not a violation: run-branch
snapshot commits are Speccy-labeled by design, and the PR metadata block in
"Lightweight Team Sharing" is intentionally Speccy-labeled. Teams that do not
want `speccy:` messages in mainline history squash on merge; `/speccy-ship`
offers the squash by default before opening the PR.

Enforcement is three cheap layers, none of them a dedicated agent:

1. **Deterministic provenance scan.** The controller scans the diff — each
   task diff at verification, the integrated diff at final validation —
   against a deny-list: `speccy` (case-insensitive), spec references
   (`SPEC-YYYYMMDD-XXXX`), the run's ULID-based run and spec IDs, its
   requirement IDs (strongly delimited forms like `R-AUTH-003`), and any
   extra terms configured under `provenance.extra_terms` in `project.yaml`.
   Bare task IDs (`T1`, `RT2`) are deliberately excluded: they are too short
   and too common in ordinary code — type parameters, test fixtures, CSS —
   to scan without false positives, and task-ID leakage reads as process
   language, which the `style` persona's semantic backstop covers. Exempt
   paths are excluded.
   A hit records a blocking finding and feeds the normal repair round. String
   matching is exactly deterministic-core work: zero tokens, no judgment
   calls, runs every round.
2. **Prevention in role prose.** Worker and repair prompts carry a standing
   rule: never write Speccy identifiers or process language into product
   files, including test names — requirement-to-test traceability lives in
   the run store's evidence records, never in the test itself.
3. **Semantic backstop in review.** The `style` persona's checklist covers
   leakage a regex cannot catch: comments or docs written in process language
   ("this satisfies the requirement that...", "round 2 fix"), or narrative
   that explains the change to a reviewer instead of the next reader.

A dedicated provenance reviewer persona was considered and rejected:
identifiers are mechanical to catch, and a run-gate-only auditor would let a
task-level leak survive every task round before costing a run-level repair
round to fix.

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
safe escaping for markdown/YAML frontmatter and TOML where needed (the Codex
target renders agent definitions as TOML). The implementation
language is Rust, and the intended engine is `minijinja`
(Jinja2 syntax): includes/macros cover partials, it supports strict
undefined-variable errors, and markdown/YAML/TOML escaping can be handled
through custom filters. The choice stands unless implementation proves it cannot meet
a requirement above; the design requirement remains a structured template
engine with testable render inputs and outputs, not loyalty to any particular
engine.

The template context should include at least:

- `target.harness`: `codex`, `claude`, or a future harness key.
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
question prompt is needed. A Codex handoff can likewise reference Codex's
`/plan` Plan Mode command (both harnesses expose one, verified 2026-07-03)
and should name `request_user_input` for the corresponding structured
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

The `/speccy-plan` skill also installs a small `references/` directory beside
the skill. These managed reference files are progressive-disclosure prompt
material, not controller state: the planning skill and planner role read them
only when drafting, semantically self-reviewing, or presenting the approval
card. They carry examples and quality standards that would otherwise bloat the
always-loaded skill prompt.

Codex install pack (paths verified against Codex docs, 2026-07-03):

- Entry skills at `.agents/skills/speccy-*/SKILL.md` — the Agent Skills
  standard location Codex scans from the working directory up to the repo
  root. Implicit natural-language invocation comes from the skill
  `description`; explicit invocation is the slash form. (`.codex/skills/`
  does not exist; Codex custom prompts are user-global and deprecated in
  favor of skills, so Speccy renders neither.)
- Role/subagent definitions at `.codex/agents/speccy-*.toml` — Codex custom
  agents are TOML files with `name`, `description`,
  `developer_instructions`, and optional `model`, `model_reasoning_effort`,
  and `sandbox_mode`. This includes one reviewer subagent per configured
  persona (`speccy-reviewer-<persona>.toml`; see "Reviewer Personas").
- Optional Codex plugin manifest (`.codex-plugin/plugin.json`) bundling
  skills and MCP configuration — a later packaging convenience, not the MVP
  default.

Claude Code install pack (paths verified against Claude Code docs,
2026-07-03):

- Entry skills at `.claude/skills/speccy-*/SKILL.md` — invocable as
  `/speccy-*` and auto-invocable by `description` matching.
  (`.claude/commands/*.md` still works but is legacy and never
  auto-invokes; Speccy renders skills.)
- Claude subagent definitions at `.claude/agents/speccy-*.md` for the
  planner, worker, verifier, and repair roles, plus one reviewer subagent per
  configured persona (see "Reviewer Personas"), with per-role and per-persona
  `model` frontmatter.
- Optional MCP configuration only for workflows that explicitly need MCP.

One pack-prose rule from the same verification: only entry skills, which run
in the main session, may reference the harness's structured-question tool
(`AskUserQuestion`, `request_user_input`); its availability inside subagents
is unverified, so subagent prompts must not depend on it.

The installed skills/agents should be thin. They should guide the harness, call the deterministic Speccy controller for spec/run state, and return compact checkpoints. They should not attempt to keep the full spec ledger inside the model context.

Recommended repo-local shape:

```text
.speccy/
  project.yaml
  pack-lock.yaml

.agents/skills/speccy-*/SKILL.md   # entry skills, Agent Skills standard (Codex)
.codex/agents/speccy-*.toml        # Codex role/subagent definitions
.claude/skills/speccy-*/SKILL.md   # entry skills (Claude Code)
.claude/agents/speccy-*.md         # Claude subagent definitions
```

Repo-local `.speccy/` holds exactly two files. `project.yaml` carries project
configuration and machine-readable policy values (risk defaults, repair and
retry caps, evidence execution limits, the reviewer persona roster with
per-persona models, provenance deny terms); `pack-lock.yaml` pins pack
versions and render metadata. There are no `policies/`, `roles/`, or
`evidence-presets/` folders: that prose is harness-facing, so it is
template-rendered into the selected harness pack (`.claude/`, `.codex/`,
`.agents/`) where the agent actually reads it, and edited there. Runtime run
state never lives in the repo.

The full `project.yaml` schema:

```yaml
risk_default: standard
caps:
  task_repair_rounds: 3
  run_review_rounds: 3
  structured_output_retries: 3
  max_tasks: null                   # optional; null = uncapped
  max_run_wall_clock_minutes: null  # optional; null = uncapped
evidence:
  command_timeout_seconds: 600
  command_output_max_bytes: 1048576
  command_policy:
    allow: []                       # optional whole-command glob patterns;
                                    # empty = any approved command may run.
                                    # A drift guardrail, not a sandbox.
review:
  personas:                         # roster; render input for reviewer subagents
    - name: spec-fidelity
    - name: defects
      model: opus                   # optional; string, or map keyed by target
    - name: security
    - name: style
      model: haiku
      min_risk: null                # optional; persona joins only at this tier or above
provenance:
  extra_terms: []                   # extra deny-list terms for the provenance scan
```

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

speccy ctl evidence collect --run <id> --requirements R1,R2 [--requests R1.E1,R2.E1] --json
speccy ctl evidence record --run <id> --input evidence.json --json
speccy ctl finding record --run <id> --input finding.json --json
speccy ctl requirement set-status --run <id> --lease <token> --input status.json --json
```

Every operation returns the JSON envelope `{ok, data}` or
`{ok: false, error: {code, message, details?}}`. Packet operations return
structured JSON; the human-formatted packets (`packet review`,
`packet escalation`) carry their rendered form in a `markdown` field inside
`data`. Every `--input` flag accepts a file path or `-` to read the payload
from stdin. Payload shapes are specified in `SCHEMAS.md`.

Naming convention: operations are noun-first — `spec`, `run`, `task`, `packet`, `evidence`, `finding`, `requirement`, mirroring the nouns in `TERMINOLOGY.md` — with a small verb vocabulary: `start`/`status` for lifecycle, `next` for the loop directive, `claim` and `collect` for actions the controller performs, `record-*` for append-style writes, `patch-*` for partial edits, and `set-status` for status transitions. `speccy ctl <noun> --help` lists that noun's operations.

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
- Relevant prior spec, decision, and review summaries that are not cancelled, superseded, or archived, including their carry-forward decisions (see "Carry-Forward Decisions").
- Policy constraints, risk guidance, and the applicable human gates.
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

An approved revision is never patched in place. When the spec's latest
revision is approved, `spec record-draft` or `spec patch-draft` opens draft
revision N+1 seeded from the approved revision; the draft then follows the
same lint-and-approve cycle, and a new prose approval produces the new
revision (and, later, a new run). `invalid_transition` is reserved for
operations that would mutate the approved revision itself.

The planner must draft from the current codebase first, then reconcile relevant prior specs and decisions. Prior specs are context, not truth. If current code contradicts a prior accepted spec, the planner should flag drift or staleness rather than silently carrying the old requirement forward.

Relevant prior context should be candidate-scoped. The controller can retrieve candidates by status, tags, touched paths, requirement topics, and decision summaries; the harness classifies each as relevant, stale, obsolete, superseded, or ignored. The human checkpoint should summarize only the carried-forward constraints and notable drift, with links or commands to open the full prior spec when needed.

### Carry-Forward Decisions

A decision record may carry `carry_forward: true`, set at recording time when
the decision constrains future work — an architecture choice, a durable
security posture — rather than run mechanics. The skill sets the flag as part
of the recording echo; the controller treats it as data and never judges it
(shapes in `SCHEMAS.md`). The planning phase's prior-context reconcile pass
verifies each carried-forward decision against the current codebase and flags
contradictions on the spec card; the controller performs no staleness
detection.

In MVP, planning context comes from active-spec prior-context candidates: the
planning packet surfaces carry-forward decisions from every non-cancelled,
non-superseded, non-archived spec, and the planner reconciles them against the
current codebase. Archiving a spec therefore removes its decisions from
planning context.

Surfacing carry-forward decisions from *archived* specs — a derived decision
index (a projection over the whole store, with a rendered cap and overflow
drill-down) so a constraint recorded long ago still reaches the planner after
its spec leaves the active list — is deferred to Later Capabilities. It
matters only once a workspace archives specs, which the single-spec MVP does
not exercise. The `carry_forward` flag is recorded from day one, so the
projection can be built without a data migration when multi-spec use proves it
necessary. See "Later Capabilities."

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
can exist before any run exists. `/speccy-plan` first route-checks the request;
only the Speccy-spec route decomposes intent into a draft spec and acceptance
ledger and presents the spec card. The human approves the card in prose, which
the plan skill records through the controller, moving the spec revision to
`approved`. `/speccy-implement` then runs against that approved revision,
inferring it when unambiguous: it exits early if the revision is not approved,
otherwise it creates a run and starts implementation. Approval is persisted
controller state, not chat state, so `/speccy-implement` can run in a fresh,
cleared session.

Spec draft lifecycle:

```text
brainstorm handoff (optional)
  -> /speccy-plan route preflight
       -> route away: direct edit / regular harness plan / split
       -> Speccy spec
            -> draft spec + acceptance ledger + spec card
            -> revise
            -> approved revision      prose approval recorded by /speccy-plan
                 -> /speccy-implement creates a run
            -> cancelled
            -> split/superseded
```

The run state is a single flat enum (canonical; TERMINOLOGY names it, this
section owns it). `run start` opens the run in `implementing`; there is no
separate `created` holding state.

- `implementing`: the serial task loop is running.
- `verifying`: final validation, drift review, and run-level repair.
- `verified`: verification passed; awaiting the human's ship decision.
- `submitted`: change proposed, awaiting review and merge.
- `landed`: change merged, recorded by `speccy accept`.
- `escalated`: autonomous progress stopped; needs a human decision.
- `cancelled`: a human stopped the run.

Run state machine:

```text
implementing              run start opens the run here
  -> implementing       next task (rounds tracked in task graph)
  -> verifying          all tasks integrated
  -> escalated          task repair cap exhausted or human/policy gate
verifying
  -> verifying          run-repair rounds (tracked in ledger)
  -> verified           all requirements resolved
  -> escalated          run repair cap exhausted or human/policy gate
verified
  -> submitted          /speccy-ship opens the PR
  -> implementing       rework: human sends the work back with prose
                        feedback (run record-decision, type rework)
  -> cancelled
submitted
  -> landed             speccy accept records that the change merged
  -> cancelled          PR closed unmerged, you stop it
escalated
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
- No task reaches `integrated` until linked acceptance requirements are resolved for the selected risk tier.
- Tasks execute serially by default, and each task can repeat implement-review-repair rounds before the scheduler moves to the next task.
- Higher-risk work increases the evidence requirements inside the same ledger.
- A failed task reviewer creates a task-scoped repair round. A failed final validator creates a run-level repair task, a waiver request, or an escalated state.
- A run-level repair task is created dynamically: the controller appends task `RT<n>` to the runtime task graph, linked to the failing requirement IDs, and it runs the same claim → dispatch → handoff → verify cycle. Run-level rounds are counted per run against `run_review_rounds`, independent of any task's counter.
- Blocked task-linked requirements do not consume repair rounds: repair cannot manufacture missing environment or evidence, so the run moves straight to `escalated` as a human/policy gate.
- Each repair loop is capped by policy, defaulting to 3 rounds. The task repair loop and the run-level repair loop each keep an independent count and an independent cap.
- When a loop exhausts its cap and a linked requirement is still `failed`, the run gives up, transitions to `escalated`, and emits an escalation packet. A blocked requirement that prevents verification also transitions to `escalated`, but as a human/policy gate rather than a capability-escalation event. See "Capability Escalation and Give-Up Policy."
- After verifying passes, the run enters `verified`: the work is done and awaiting the human's ship decision. `/speccy-ship` opens the PR and moves the run to `submitted`. Minor implementation feedback at this gate is recorded as a `rework` decision (`run record-decision`, feedback prose required): the run returns to `implementing` and the controller appends a dynamic `RT<n>` task seeded with that feedback, counted against `run_review_rounds` like any run-level round, so send-backs are bounded by the same cap and re-verify through the normal cycle back to the same gate. Feedback that changes scope, requirements, or risk is a spec amendment instead — the definition of done changed (see "Review UX").
- `submitted` advances to `landed` when the human runs `speccy accept` after the change merges. The spec then becomes `accepted`; archive later only when the accepted spec no longer describes the codebase. See "Acceptance."
- Human waivers are recorded in the review packet.
- The run state is a single flat enum. Progress within `implementing` and
  `verifying` is read from the task graph and acceptance ledger, not a second
  state field.
- Run state is append-only where possible.

### Capability Escalation and Give-Up Policy

Autonomous repair must terminate. Without a cap, a run can loop on an unsatisfiable requirement and burn the token budget the checkpoint model is meant to protect. Speccy caps repair effort, then hands the problem back to the human.

Not every escalation is a repair-cap failure. A missing credential, unavailable
local environment, production-only behavior, or subjective requirement can also
stop verification. Those cases keep the relevant requirement `blocked` and move
the run to `escalated` as a human/policy gate rather than a
capability-escalation event.

The counting model uses two nouns for two jobs:

- **Task is the unit that is retried.** A repair round re-runs a task, because the implementer edits a task, not one requirement in isolation. The round counter lives on the task.
- **Requirement is the unit that is judged.** A round fails when a linked requirement is `failed` after the attempt, including a `failed` on a vacuity finding. The give-up decision and the escalation packet are scoped to the requirement, not the task.

The rule:

> A task runs at most the policy-configured number of repair rounds. When the cap is exhausted and any linked requirement is still `failed`, the run gives up, transitions to `escalated`, and emits an escalation packet naming those requirements.

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
Round 3: R6 failed (vacuity finding)       -> cap hit

Run -> escalated
Escalation packet scoped to R6.
Tasks after T3 are not scheduled.
```

#### Escalation Packet

The escalation packet is a distinct artifact from the run's review packet. It is scoped to the requirement that could not be satisfied or proven, not the whole run. It is assembled deterministically by `packet escalation` from recorded rounds, findings, and decisions. Exhausting the repair cap is Speccy's signal that the approach or the requirement itself is wrong, not that one more implementation attempt is needed. A blocked requirement is a signal that the environment, policy, or evidence strategy needs a human decision. The natural resolution is usually a spec amendment, environment fix, waiver, or review-passed judgment with residual risk, not another blind repair.

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

At the escalation gate the human responds in prose, and the harness records the right decision through the operation `gate_answers` names for that answer rather than offering a menu of process verbs. A gate decision that resolves a requirement — a waiver — sets the linked requirement status atomically inside the same operation; this is the only status-mutation path outside `requirement set-status`, and it is reserved for human gate decisions:

- **Amend the spec.** The usual outcome. Creates a new approved spec revision and a new run, with a decision record explaining why the definition of done changed. The escalated run is closed as `cancelled` atomically inside the superseding approval, with a decision record naming the superseding revision and run (see "Amendment at the Escalation Gate"). Any guidance the human gives is folded into the amendment.
- **Provide missing setup or evidence.** Keeps the spec revision, records the gate decision, and resumes the same run in `implementing` or `verifying` when the environment is ready. Resume re-opens the *current* review round — it re-dispatches the run-level verifier, or re-opens the stuck task's worker at its same round — and never counts a fresh round, so a run parked at its cap resumes within the cap rather than at cap+1.
- **Waive the requirement.** Accept the residual risk; the decision sets the requirement to `waived` atomically, and the same run resumes from where it stopped. If the waiver leaves every requirement resolved with no blocking finding, `verifying` completes straight to `verified` (subject to the critical accepted-risk gate) without opening a new review round; if work remains, resume behaves like provide-setup and re-opens the current round.
- **Cancel the run.**

#### Amendment at the Escalation Gate

The escalation gate is a conversation, not a form. The escalation packet ends with one question, and the human answers in prose in the same harness session. If that session is gone, `speccy status` re-surfaces the pending gate, and any later harness session picks it up from controller state.

The amendment path reuses the planning machinery instead of adding a new surface:

1. The human describes the change in prose, such as "expiry should be 30 minutes, drop R6" or "verify this via the API instead of the browser."
2. The harness runs the same draft-revision loop `/speccy-plan` uses: patch the spec draft, lint it, and present an amended spec card that shows the diff against the prior approved revision and names the escalation that motivated it. Nothing is recorded yet — amend is a deferred gate answer, and the run stays parked at its gate while the draft loop runs.
3. The human approves the amended card in prose; the harness records the approval through `spec record-decision` (type `approve`, with `supersedes.run_id` naming the parked run), producing a new approved spec revision.
4. Inside that same operation the controller atomically closes the parked run as `cancelled`, writing its run-scoped decision record linking the superseding revision and run — the same atomicity rule as gate waivers. The checkpoint copy tells the user to run `/speccy-implement` (fresh session recommended, selector only when ambiguous).

An abandoned amendment records nothing: if the amended card is never approved, the run is still parked at its gate and every other gate answer remains available. This is why `gate_answers` names `spec record-decision` as the amend recorder — `spec patch-draft` is only the working step and records no decision.

At escalation the controller commits any uncommitted in-flight diff as a labeled escalation snapshot, so the parked worktree is clean and the superseding run's clean-worktree rule holds. The new run starts on the same branch, seeded with the prior run's summary and the escalation snapshot reference, so it reconciles rather than redoes. Rolling back to the run baseline remains the human's explicit fallback at the gate.

Setup and waiver answers stay on the same run: the harness records the decision, and the following `run next` call resumes the loop from where it stopped. The resume is recorded as a distinct event from an ordinary state transition, so replay re-enters `verifying` or `implementing` without incrementing the run-level review-round counter — a gate resume is not a fresh round. `provide_setup` re-arms the current round (re-review at the run gate, or the stuck task's worker at its same round); a waiver that resolves the last outstanding requirement lets `verifying` complete without any re-review. Only amendment replaces the run, because only amendment changes the definition of done.

### Acceptance

When `verifying` passes, the run enters `verified`, and `run next` returns an `await_human_gate` directive for the ship decision with `record_with: run record-ship`. Invoking `/speccy-ship` answers that gate: the skill calls `run next --agent <id>` to take the lease, proposes the change as a pull request or hands it off as a branch or patch, and records the transition and the change reference through `speccy ctl run record-ship`, which moves the run to `submitted`. Speccy is ephemeral, holds no provider credentials, and makes no outbound calls, so in MVP it does not detect the merge at all: the human records it.

The run store persists a small change reference for what was proposed, as provenance:

```yaml
change_ref:
  kind: pull_request        # pull_request | branch | patch | none
  url: https://github.com/org/repo/pull/123
  branch: speccy/spec-20260630-a7f4-passwordless-login
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
speccy accept --pr <url> --note "<text>"         # recovery/manual association
speccy archive [<selector>]                      # hide stale historical context from active views
```

- `speccy accept` is a human assertion. Speccy does not verify the merge in MVP; the human is telling Speccy what already happened. The command uses the `change_ref` recorded at ship time, so the routine path never repeats `--pr`.
- Before recording a submitted run as landed, `speccy accept` displays the recorded `change_ref` (PR URL, branch/patch, head SHA, and base when present) so the human can catch a wrong selector. If the run is already `landed`, it prints "already recorded" and exits successfully. If more than one submitted run matches, selector resolution asks the human to disambiguate instead of guessing.
- The assertion is order-independent with respect to teammates and remote review. A teammate may merge the PR, close the local branch, squash the commits, or advance the base branch before the original author returns; `speccy accept` still closes out the submitted run because the recorded `change_ref` identifies what was proposed and the human assertion identifies that it landed. `--pr` and `--note` exist for recovery, local-only changes, or manual association when no useful `change_ref` was recorded.
- Because the step is manual, it must be impossible to lose: `/speccy-ship` ends by printing the shortest unambiguous command (`speccy accept` in the common single-submitted-run case, or `speccy accept <selector>` when needed), the Awaiting-merge status card carries it as the next action (see "CLI/Admin Flow"), and the default PR metadata block includes a full-reference `accept_with` command for durable PR context (see "Lightweight Team Sharing"), so the reminder survives in whichever surface the human returns to.
- A PR closed without merging is a flag on `submitted`, not a separate state. The human starts a new run or cancels.

Automatic merge detection — git-native ancestry checks, squash-merge heuristics, or a configurable host probe — is deliberately cut from MVP. It is an external-integration convenience outside core Speccy's goals, and manual acceptance is enough to dogfood the loop. See "Later Capabilities."

When a run reaches `landed`, the spec becomes `accepted` and leaves default
`speccy status`/`speccy list` output; show it with `speccy list --accepted`,
`--status accepted`, or `--all`. Archiving is a later list-visibility action
for accepted specs that no longer describe the codebase: the landed run remains
`landed` in run history, and the spec's `carry_forward` decisions stay recorded
for a future decision index (see "Carry-Forward Decisions").

## Storage Model

Runtime state lives in
`~/.speccy/` only. Repo-local `.speccy/` holds exactly `project.yaml` and
`pack-lock.yaml`; all policy, role, and evidence prose is rendered into the
harness packs, and exports are opt-in snapshots written to explicit
destinations such as `docs/specs/`. There is no repo-local runtime mode. A
survey of how prior tools store runtime state, which validates this split,
was captured in `runtime-state-storage-survey.md` (external research note,
not in this repo).

Workspace identity: `workspace_id` is a hash of the canonicalized workspace
root plus the canonicalized git repository root, both stored in
`workspace.json`. A monorepo subtree therefore gets its own workspace,
distinct from the repo root's. Moving or re-cloning a repository yields a new
workspace ID; `speccy doctor` reports store entries whose recorded paths no
longer exist. The store root defaults to `~/.speccy` and can be overridden
with the `SPECCY_HOME` environment variable, which tests and CI use for
isolation.

Runtime storage is external:

```text
~/.speccy/                        # override with SPECCY_HOME
  config.toml
  workspaces/
    <workspace-id>/
      workspace.json
      specs/
        <internal-spec-id>/
          spec-ref.txt
          events.jsonl            # canonical: spec-scoped log (request, drafts,
                                  # revisions, approvals, spec decisions)
          spec.yaml               # derived projection of the current revision
          runs/
            <run-id>/
              events.jsonl        # canonical: run-scoped log (state, tasks,
                                  # rounds, statuses, run decisions)
              handoffs/           # canonical artifacts, one file per ID
              evidence/
              findings/
              artifacts/
              run.yaml            # derived projections, rebuilt by replay
              acceptance-ledger.yaml
              task-graph.yaml
              review-packet.md    # generated snapshot
```

The state model (JSONL-first):

- Portable canonical log: append-only JSONL events, the source of truth from day one; state is rebuilt by replay.
- Runtime query store: an optional SQLite projection rebuilt from the JSONL log, deferred until queries or scale demand it. The walking skeleton uses an in-memory projection.
- Large artifacts: files referenced by ID and content hash, such as transcripts, diffs, screenshots, command logs, and evidence.
- Generated snapshots: markdown/YAML views for review, not the primary source of truth.

All controller state writes are atomic: write to a temp file, fsync, then
rename over the target. JSONL event appends use verified read-back — the
appended record is re-read and checked before the operation reports success —
so a crash never leaves a half-written transition. Resume from the store is
only trustworthy if every write follows this discipline.

The event vocabulary grows additively: a new binary may write event variants
an older binary does not know (for example `run_resumed`). Replay is
fail-closed, so an older binary reading a newer log errors on the unknown
variant rather than silently dropping it. This is accepted for a local
single-binary tool — the store is not a shared wire format — and it is why a
downgrade after a run has advanced is unsupported.

Concurrent `speccy` processes — the orchestrating skill plus lease-free
reviewer personas — may append events at the same time. Event-log appends
therefore serialize on a per-workspace store lock file; artifact files are
written per-ID and never contend. Most appends hold the lock only for their own
duration. The exception is `run next`: it holds the store lock across its whole
cycle — the opening projection read, the derived-transition appends (including
the git snapshot commits and diffs they trigger), and the closing projection
read — so a second concurrent `run next` cannot read the same pre-transition
state and apply a derived transition twice. Lease-free reviewer appends may
therefore wait briefly while a `run next` cycle holds the lock. Lock order is
one-directional: an operation that needs both takes the command lock first, then
the store lock (`run next` takes only the store lock; `evidence collect` takes
the command lock around execution and the store lock only for its own append),
so the two locks cannot deadlock.

SQLite should not be committed to git. It is binary, noisy, and poor for review. JSONL event logs are text and portable, but they are still operational run history and should not be committed by default either.

### Git Policy

Repo-local Speccy harness packs are commit-safe workflow artifacts. The repo commits exactly two `.speccy/` files plus the rendered packs; everything operational lives in `~/.speccy/`.

Commit by default:

```text
.speccy/
  project.yaml
  pack-lock.yaml

.agents/skills/speccy-*/SKILL.md
.codex/agents/speccy-*.toml
.claude/skills/speccy-*/SKILL.md
.claude/agents/speccy-*.md
```

Commit or attach selectively, via explicit `speccy export` commands: compact review packets and spec exports, written to explicit destinations such as `docs/specs/<spec-ref>/` or attached to the PR. These are useful when the team wants PR-visible evidence. They should be generated intentionally and kept compact. `.speccy/` itself holds no export folders.

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
  accept_with: speccy accept SPEC-20260630-A7F4
  acceptance_hash: sha256:...
  review_packet_hash: sha256:...
  result_summary_hash: sha256:...
  rerun:
    harness_instruction: "Re-run Speccy verification against this branch, acceptance snapshot, and result summary."
```

No-server sharing options:

- Paste or attach the review packet to the PR.
- Commit or attach only compact snapshots when useful: the review packet or a spec export.
- Let other engineers rerun verification against the shared acceptance snapshot; this reuses the verifier role and the local controller tools rather than adding a separate skill, and is a later/team capability.
- Export a redacted run bundle only when debugging or audit needs it: `speccy export run-bundle --redact`.
- Attach that bundle to an issue, PR, CI artifact, or file share outside git rather than committing it.

Optional hosted mode:

- A `run_url` can be included for organizations that choose to run a shared Speccy run store.
- The hosted store should be an optional convenience, not a requirement for collaboration.

Repo writes remain opt-in:

- `speccy export spec`
- `speccy export review`
- `speccy export run-bundle --redact` for debugging, compliance, or reproducibility only

This reconciles shared lifecycle prose with zero product-code footprint.

Harness installation writes repo-local workflow artifacts by default:

- `speccy install` detects supported harnesses and installs or repairs repo-local packs.
- `speccy install --target codex` installs or repairs the repo-local Codex pack.
- `speccy install --target claude` installs or repairs the repo-local Claude pack.
- `speccy install --update` applies reviewable pack updates.

Operational run state, transcripts, raw evidence, screenshots, command logs, and databases still live outside git or under ignored paths unless explicitly exported.

## Execution Strategy

### Planning Phase

Planning is the `/speccy-plan` skill. It runs after an optional `/speccy-brainstorm` or directly from intent. It owns route selection as well as Speccy drafting: invoking `/speccy-plan` does not mean the user has already chosen the full Speccy workflow.

1. Intake asks clarifying questions only when necessary.
2. The skill performs a read-only route preflight using the scope-rating rules below: direct edit, regular harness plan, Speccy spec, or split into multiple specs.
3. If the recommended route is `direct_edit`, `harness_plan`, or `split_specs`, the skill returns a compact route card with the exact next action and creates no spec, task graph, acceptance ledger, or run. The user can still explicitly reply "use Speccy anyway" to override.
4. Only when the preflight recommends `speccy_spec` — or the user explicitly overrides — does `/speccy-plan` create Speccy spec state.
5. The controller builds a deterministic planning packet with current workspace state, policy, output contract, and relevant prior context candidates.
6. The harness planner inspects the current codebase read-only and includes intake observations in the draft submission when they are useful for resumability or later review.
7. The planner reconciles relevant prior specs and decisions against the current codebase, carrying forward only constraints that still appear valid and flagging stale, obsolete, contradicted, or superseded context.
8. The planner classifies task risk and creates a complete candidate spec draft with goal, non-goals, scope, assumptions, acceptance requirements, expected evidence, and open questions.
9. Speccy structurally lints the draft. The harness repairs missing or invalid sections through focused draft patches rather than section-by-section append commands.
10. After structural lint is clean, the harness runs a semantic self-review against the installed planning reference: placeholders and vague language, contradictions, hidden scope, stale prior context, weak evidence, task/requirement coverage, risk-tier evidence depth, and open questions that the repo could have answered. Fixes are recorded as draft patches before the approval card.
11. Human approval is requested through a compact spec card: goal, scope, non-goals, plan summary, key requirements, prior context carried forward, open questions with recommended answers, and main risks. The full spec and ledger are available on request.
12. The planner creates only as much design/task detail as the request needs.
13. Each acceptance requirement gets at least one evidence request; structural
   lint flags any requirement without one and approval is refused while the
   draft is lint-dirty. `blocked` and `waived` are runtime outcomes, never
   planned statuses. Manual human judgment is recorded as evidence or as a
   waiver/review-passed decision, not as its own requirement status.
14. Higher-risk work stays in the same ledger but requires stronger evidence, such as negative cases, positive cases, pre-fix failure, fresh-context review, or human approval.
15. Fresh-context adversarial review is required when new tests or review-only evidence carry an important acceptance decision.
16. The human approves the spec card in prose for every spec; `/speccy-plan` records the approval through the controller, moving the revision to `approved`. `/speccy-implement` exits early until that approval exists.

### Implementation Phase

Implementation is a serial task execution loop. Each task gets its own implementer and fresh task reviewer/verifier, and can run repeatable repair rounds before the scheduler advances:

1. Scheduler selects the next task.
2. Harness worker receives a task packet scoped to linked requirements, expected files/areas, evidence requests, and known constraints.
3. Worker implements only that task.
4. Worker returns a handoff.
5. The configured reviewer personas fan out fresh-context over the handoff, diff, commands, and linked requirement evidence (see "Reviewer Personas"); the verifier aggregates their findings and returns status updates.
6. The task verifier collects evidence for linked requirements, using Speccy evidence tools when useful.
7. The verifier handles semantic review and evidence adequacy review at the depth required by the risk tier.
8. The lease-holding loop driver records acceptance statuses from collected evidence plus structured verifier findings.
9. Failed task-linked items create task-scoped repair rounds up to the tier's cap. Blocked items never enter repair — rounds cannot manufacture missing environment or evidence — and escalate directly as a human/policy gate.
10. The scheduler advances only after the task is `integrated`.

### Validation Phase

Final validation is a run-level evidence and drift review after task execution. It should not duplicate every task review by default; it checks whether the whole spec still holds after integration:

1. Final validation fans out the reviewer persona roster over the integrated whole-run diff (see "Reviewer Personas"); the final verifier reads the acceptance ledger, task handoffs, validator findings, and integrated diff.
2. Verifier gathers baseline integration evidence: format, lint, typecheck, targeted project commands, relevant existing tests, or browser/API checks.
3. Verifier checks requirement coverage across all tasks and identifies requirements that remain failed, blocked, waived, or only review-passed.
4. Verifier performs drift review: compare approved spec/plan/task scopes against the final diff, handoffs, and decisions.
5. Verifier reviews whether the evidence set actually supports the spec at the selected risk depth.
6. The controller runs the deterministic provenance scan over the integrated diff; hits record blocking findings (see "Provenance Hygiene").
7. Verifier records run-level findings, residual risk, and repair recommendations.
8. Failed integration checks, cross-task regressions, drift, or provenance hits create run-level repair tasks, waiver requests, scope-change decisions, or escalated states.
9. Human final review happens when policy requires it.

Acceptance status uses the six canonical requirement statuses defined in "Requirement Resolution Rules". `pending` marks an item whose evidence has not been collected yet; validation resolves each remaining item to `passed`, `review_passed`, `failed`, `blocked`, or `waived`.

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

1. If `.codex` or `.agents` exists, include `codex` (Codex reads skills from `.agents/skills/` and agents from `.codex/agents/`).
2. If `.claude` exists, include `claude`.
3. If multiple supported harnesses exist, install all detected targets unless `--target` narrows them.
4. If no harness directory exists, no target can be auto-detected. Explain that no supported harness was detected and ask the user to choose `--target codex`, `--target claude`, or `--target all`. In noninteractive mode, fail unless `--target` is provided.

Target values:

- `auto`: default detection, and the behavior when no `--target` is given. Renders every detected harness, so a repo with both `.codex` and `.claude` gets both.
- `codex`: repo-local Codex pack (`.agents/skills/` + `.codex/agents/`).
- `claude`: repo-local Claude pack (`.claude/skills/` + `.claude/agents/`).
- `all`: all supported harness packs.

There is no generic `agents` target: no cross-harness convention exists for
role/agent definition files (verified against harness docs, 2026-07-03).
`.agents/` is standardized territory for Agent Skills only (agentskills.io;
read by Codex, Amp, and OpenHands). A core-fields-only generic skills pack
plus a root `AGENTS.md` pointer is a later capability.

Install should be idempotent. A plain `speccy install` may create missing packs, repair missing managed files, update lock metadata, and report outdated packs. It must not apply upstream changes to existing managed prose unless `--update` is passed.

Install touches a dozen-plus repo files on first run, so an install that would write anything previews first: it prints the exact creations, repairs, and `.gitignore` edits — grouped by target when more than one harness is detected, naming `--target codex|claude` as the way to narrow — then asks to proceed before writing. `--yes` skips the prompt; in noninteractive mode, writing requires `--yes`, mirroring the `--update` policy below. An install with nothing to do prints its status and never prompts. (`--dry-run` alone only helps users who already know to ask.)

`--dry-run` composes with any install invocation, not just `--update`: it prints the same would-write listing and stops, for scripts and users who want the preview without the prompt.

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

`/speccy-brainstorm` is the optional exploration skill. It is not required — a user can invoke `/speccy-plan` directly — but it is encouraged when scope or route is uncertain, and it activates by slash command or natural language. It stays exploratory and read-only while it inspects the codebase, sketches options, lists open questions, identifies possible splits, and produces a brainstorm handoff. It does not draft a spec. The handoff is not a spec, not an approved plan, and not an acceptance ledger. By default it is ephemeral chat context; Speccy persists it only if `/speccy-plan` promotes it into a spec or the user explicitly exports it.

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
- **Plan a Speccy spec** - for larger, riskier, multi-task, user-visible, or evidence-sensitive work. `/speccy-plan` creates the draft spec and acceptance ledger only after its route preflight recommends a Speccy spec, or after the user explicitly overrides a route-away recommendation.
- **Split into multiple specs** - when the request is too broad, propose an initiative with multiple specs.

Both `/speccy-brainstorm` and `/speccy-plan` use the same route vocabulary. The difference is commitment: brainstorm always returns an ephemeral handoff; `/speccy-plan` first returns a route-away card when the request is too small, too broad, or better handled by normal harness planning, and only proceeds to controller-backed spec drafting on the `speccy_spec` route or an explicit override. A route-away card should show one recommended route, with alternatives secondary, plus a scope rating so the user understands why Speccy is recommending direct work, a regular plan, one spec, or multiple specs. Example:

```text
Scope: medium
Recommended route: regular harness planning
Confidence: medium
Reason: medium complexity, low risk, and no need for an acceptance ledger or autonomous repair loop.
Next action: Continue from the Speccy brainstorm handoff above in the active harness's normal planning flow. Produce a normal harness plan only; do not create a Speccy spec or acceptance ledger yet.

Alternatives: direct edit, plan a Speccy spec (/speccy-plan), split into multiple specs
```

Each route recommendation should include an exact next action phrase or command. Route selection can stay conversational inside the harness; Speccy should not add per-route commands such as `/speccy-promote` unless a harness cannot support the interaction cleanly. When `/speccy-plan` routes away, the response creates no controller state and ends there unless the user explicitly says to use Speccy anyway.

#### Scope Rating

The brainstorm skill and `/speccy-plan` preflight rate the request before
recommending a route. The rating is prose guidance the skill reasons through,
not a structured artifact the controller stores or validates: a size
(`tiny | small | medium | large | initiative`), a recommended route
(`direct_edit | harness_plan | speccy_spec | split_specs`), and a short reason.
Whether the work can be evidenced is always the first question the skill asks,
and low evidence-ability is what most often routes large work away from
`speccy_spec`.

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

When the recommended route is regular harness planning, Speccy should not create a spec, task graph, acceptance ledger, or run. This route uses the harness's own plan mode, not the Speccy drafting path. It should return an ephemeral handoff that the user can feed into the active harness's normal planning mode.

Codex handoff:

```text
Recommended route: Codex Plan Mode
Next action: /plan Continue from the Speccy brainstorm handoff above. Produce a normal Codex plan only; do not create a Speccy spec or acceptance ledger yet.
Structured-question tool name for rendered Codex prose: request_user_input
Alternative: cycle to Plan Mode with Shift+Tab, then paste the handoff.
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

When the route is a Speccy spec, `/speccy-plan` treats the brainstorm handoff, prior specs, decisions, and the current user request as context, and reconciles them against the current codebase rather than carrying them forward blindly. It creates a draft, never an approved spec. Approval happens only when the human approves the spec card in prose, which `/speccy-plan` records through the controller; `/speccy-implement` later refuses to run until that approval exists.

Every run is fully autonomous by design: after spec-card approval there is no step-by-step implementation steering, and no step-steered mode exists. Autonomy does not bypass policy, permission, environment, budget, production/deployment, critical-waiver, missing-credential, or spec-gap checkpoints.

### Spec Card UX

Human planning checkpoints should default to a compact spec card instead of the full technical spec or ledger. The card should answer four user questions: what will change, what will not change, how Speccy will know it worked, and what could go wrong. It should contain enough information to approve intent, scope, risk, and proof strategy:

```text
Spec: SPEC-20260630-A7F4  Passwordless login
Risk: high
Decision needed: approve this spec, or revise scope
Reply:
  go                 approve and start now
  approve only       approve, but do not start
  revise: <changes>  update the spec card
  split: <guidance>  split into multiple specs
  cancel             stop this draft

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
- A magic link can be requested by email.
- A link is single-use.
- Links expire after 15 minutes.
- An expired link is rejected and creates no session.

Will run (evidence commands):
- npm test -- auth/magic-link
- npm test -- auth/expiry
- npm test -- ui/expired-link

Prior context:
- Carry forward prior decision to store magic-link tokens hashed.
- Prior file paths appear stale; current auth code moved under src/server/auth.

Main risks:
- Email delivery may need staging or production validation.

— rev spec_rev_001-draft · requirements R-AUTH-001…004 · ledger/evidence: speccy review --evidence
```

The card lists the distinct `kind: command` strings the controller will execute as evidence, because approval is what authorizes them to run: the human must see what will run, not just which requirements it proves (see the command allow policy under "Acceptance Ledger").

The card reads as a human decision surface, not a controller artifact: acceptance appears as plain statements the human evaluates, and the process identifiers a human rarely needs at approval — the draft revision and the requirement IDs — collapse into a single footer line. The footer keeps them visible for reference; the full ledger with per-requirement IDs is one drill-down away (`speccy review --evidence`). The spec reference and the evidence commands stay in the body: the ref is how a human names the spec across sessions, and the commands are what approval authorizes.

The approval echo is the binding surface. Before recording, the skill echoes the spec ref, the revision, and the decision (`Recording approval: SPEC-… rev spec_rev_001 -> approved`), so a prose reply in a long chat cannot silently bind to the wrong spec or a card the human never saw — with nothing extra to type (see "Harness Skills").

The spec card should make the approval boundary unmistakable: the human approves the card in prose, and `/speccy-plan` records that approval through the controller, moving the spec revision to `approved`. Approval is required and always explicit; there is no auto-approve. `/speccy-implement` then runs against the approved revision and exits early if the revision is not approved. Because approval and run state are controller-backed, it is safe to run from any session; a fresh, cleared session is recommended for clean implementation context, never required. The canonical replies are explicit: `go` records approval and starts the implement loop in the same session after the binding echo and fresh-session note; `approve only` records approval and prints the `/speccy-implement` handoff. Natural variants may map to those commands when unambiguous, but the card should teach the exact words above, and ambiguous approval prose defaults to `approve only`. The card should show one recommended next action first, with alternatives secondary: `go`, `approve only`, `revise: ...`, `split: ...`, `cancel`, or open the ledger/evidence drill-down. The full ledger remains available for power users and high-risk review, but it should not be the default checkpoint surface.

The spec-card approval is mandatory for every spec, regardless of risk. It is the single pre-implementation gate. Higher risk raises the evidence bar inside the same card and ledger rather than adding another approval step; the card simply carries more detail, such as the full task list and flagged destructive steps, so the human approves with the right information.

### Human Gates

Speccy has exactly five human gates, and no additional approval steps hide
inside phases:

1. **Spec-card approval** — prose approval recorded by `/speccy-plan`; the
   single pre-implementation gate for every spec, regardless of risk.
2. **Escalation gate** — the run parked at `escalated`: repair-cap
   exhaustion, a blocked requirement, resource caps,
   structured-output retry exhaustion, or out-of-band commits.
3. **Critical-tier accepted-risk confirmation** — on `critical` specs only,
   the last stop before `verified`, covering every `review_passed`/`waived`
   requirement. Fires only once the run-level review is recorded, every
   requirement is resolved, and no blocking finding remains; a still-failing
   run repairs or escalates first (see "Requirement Resolution Rules").
4. **Ship decision** — `verified`, answered by `/speccy-ship`, a `rework`
   decision (send it back), an amendment, or cancel.
5. **Merge acknowledgement** — `submitted`, answered by `speccy accept`.

Higher risk raises the evidence bar inside the same ledger and, at
`critical`, adds gate 3. Sandbox permission prompts — destructive commands,
network access, dependency installs — belong to the harness, not Speccy; the
packs must not suppress them.

### Harness Skills

Speccy installs the harness entry skills below. Each is invocable as an explicit slash command and by natural-language fallback. Brainstorm is optional; planning, implementation, and shipping are the load-bearing handoffs. Spec-card approval is an explicit prose act recorded through the controller, not a side effect of invoking the next skill. Every other checkpoint copy must still state its effect explicitly.

- **`/speccy-brainstorm <intent>`** - optional exploration: inspect the repo read-only, clarify open questions, rate scope, identify scale, and produce a brainstorm handoff with a recommended route. It does not draft a spec, and it is skippable when scope is already clear. Natural language: "brainstorm passwordless login."
- **`/speccy-plan <intent | handoff>`** - route-check the request first. If it is too small for Speccy, better handled by normal harness planning, or too broad for one spec, it returns a route-away card and creates no controller state. If the route is `speccy_spec` or the user explicitly overrides, it decomposes intent into a draft spec, task graph, and acceptance ledger, runs a one-time pass to resolve contradictions and reconcile prior context, then presents the spec card. On the human's prose approval it records the approval through the controller, moving the spec revision to `approved`; the canonical `go` reply also hands straight into the implement loop in-session (see Rules). It creates a draft, never an approved spec, until that prose approval. Distinct from the harness's own plan mode. Natural language: "plan passwordless login as a Speccy spec."
- **`/speccy-implement [<selector>]`** - run against an approved spec revision, inferring the current approved spec when unambiguous: serial task implement-and-review rounds, then the holistic run-gate validation and drift-correction loop. Ends in `verified` on success or `escalated` on a spec/evidence/policy gap. It exits early if the spec revision is not `approved`, and should usually be run in a fresh, cleared session for clean implementation context. Natural language: "implement the passwordless login spec."
- **`/speccy-ship [<selector>]`** — open the pull request and move the run to `submitted`, inferring the current verified run when unambiguous. When the run's accepted-risk bucket is non-empty, the skill echoes the accepted-risk lines and asks one explicit confirmation ("Open the PR anyway?") before creating anything external — the ship may happen days later in a fresh session where the review packet is not on screen, and the PR is the last moment before the work leaves the repo. The confirmation is part of answering the ship gate, not a sixth gate, and it fires only when accepted risks exist; with an empty bucket the ship proceeds without it. A tier-conditional gate is rejected: the trigger is residual risk, not tier. Natural language: "ship the passwordless login spec."

Rules:

- The slash command is the documented, deterministic entry; natural language is a convenience, not the contract.
- The spec argument accepts a full `SPEC-...` reference or a search selector, and is inferred when the current spec is unambiguous.
- `/speccy-plan` performs route preflight before `spec start`; route-away responses are ephemeral and create no spec state. The override phrase is conversational and explicit, such as "use Speccy anyway."
- Spec-card approval is a prose act recorded by `/speccy-plan` through the controller; it is required and always explicit.
- Before recording an approval or gate decision, the skill echoes exactly what it is about to record — spec ref, revision, and decision, e.g. `Recording approval: SPEC-20260630-A7F4 rev spec_rev_001 -> approved` — so a prose reply in a long chat cannot silently bind to the wrong spec or a stale card. The echo is confirmation copy, not a sixth gate; it requires no further reply. It is the only binding guard: the controller does not track a per-write draft version (see "Spec Card UX").
- `go` records the approval and starts the implement loop in the same session, after the echo and a printed fresh-session note. `approve only` records the approval and prints the `/speccy-implement` handoff instead. Natural variants may map to those commands when unambiguous; ambiguous prose defaults to `approve only`. Run-start rides on explicit prose intent — approval itself never auto-starts anything.
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
  -> route preflight
     -> direct edit / regular harness plan / split: stop with an exact next action
     -> Speccy spec: continue
  -> draft spec + task graph + acceptance ledger
  -> one-time contradiction + prior-context reconcile pass
  -> compact spec card
  -> human reviews and approves in prose
       -> /speccy-plan records approval -> spec revision = approved
       -> approval said "go" -> the run starts in this session; otherwise:

/speccy-implement                             (fresh session recommended)
  -> exits early unless the revision is approved
  -> creates run against approved revision
  -> serial task implement-and-review rounds
  -> holistic run-gate validation + drift correction
  -> ends verified, or escalated on a spec/evidence/policy gap
  -> human reviews the verified summary, or amends the gap and re-runs `/speccy-implement`

/speccy-ship                                  (invocation = ship approval)
  -> opens the PR, run -> submitted
  -> PR merged normally -> human runs speccy accept -> landed
```

Throughout, each skill calls the local Speccy controller for state, packets, and evidence, and `/speccy-implement` sequences its loop by repeatedly asking `speccy ctl run next` for the next deterministic step. The controller stays deterministic and never launches an LLM. Fresh-context validators run through the active harness. `/speccy-implement` always runs its full loop without step-by-step implementation steering; it stops only at `escalated` or a policy/environment gate.

### CLI/Admin Flow

The CLI remains useful for installation, status, export, and deterministic controller integration. It should not expose the internal SDLC as a sequence of public commands. Humans should not have to run acceptance/evidence/repair phases by hand, and no CLI command should call an LLM or launch an AI harness.

`speccy status` is the everyday hub; routine use needs four commands, and
everything else is setup, occasional lifecycle, or admin.

```bash
speccy status
speccy list --query passwordless
speccy review
speccy accept
```

Those examples intentionally omit spec references. Human commands and harness
skills infer the current or only unambiguous spec/run; a selector is required
only when there is more than one plausible target, and scripts should still use
the full `SPEC-...` reference.

Setup and diagnostics:

```bash
speccy install
speccy install --dry-run
speccy install --target codex
speccy install --update --dry-run
speccy doctor
```

Occasional lifecycle commands:

```bash
speccy new "Add passwordless login"
speccy cancel
speccy archive
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
- `status` and `cancel` manage the current spec/run when the user is outside the harness. Resuming is not a CLI action; a fresh harness session re-enters via `/speccy-implement`, with a selector only when ambiguous.
- `review` shows the current human packet for a selected spec, choosing the packet by state: draft or approved specs show the spec card/approved summary; implementing or verifying runs show the current status card plus last activity; verified runs show the review packet; escalated runs show the escalation packet; submitted runs show the recorded change reference and close-out instruction; landed/accepted specs show the final accepted summary. `--evidence` drills into the ledger, command logs, evidence artifacts, findings, decisions, and full diff where available; `--json` returns the same state-aware view structurally.
- `accept` closes out a `submitted` run as a human assertion that the recorded change landed. It uses the `change_ref` saved by `run record-ship` by default, displays that reference before recording, is idempotent for already-landed runs, and accepts optional `--pr <url>`/`--note "<text>"` only for recovery or manual association. MVP does no merge detection.
- `archive` marks an accepted spec archived when it no longer describes the codebase. Accepted specs are already hidden from default `status`/`list`; archive is not part of routine close-out. The landed run remains `landed` in run history, and archiving removes the spec's decisions from planning context in MVP; its `carry_forward` decisions stay recorded for a future decision index (see "Carry-Forward Decisions").
- `export review` produces the normal human review artifact.
- `export spec` and `export run-bundle` are advanced paths for audits, diagnostics, and custom harness integrations.
- Full planning, repair, and verification happen through the installed Speccy skills/agents inside Codex or Claude Code.

`speccy status` is the human's one glance at a workspace. It prints one card
per active run, rolling run state up into run status labels — Implementing,
Verifying, Ready to ship, Needs you (`escalated`), Awaiting merge, Interrupted
— the same way review packets roll requirement statuses into human status
buckets. Both are a rendering rule, not new stored state. The card names the next human action with the exact command
when one exists, and shows no controller machinery: no directives, leases,
run IDs, or ctl operations. Tasks appear by title, never bare task IDs —
`T1` is controller vocabulary, kept to drill-down and debug output.
An autonomous run says so:

```text
SPEC-20260630-A7F4  Passwordless login          Risk: high
  Implementing — token model + endpoints · repair round 2 of 3
  · autonomous, nothing needed
  Last activity 2m ago — running npm test -- auth/expiry
```

The last-activity line is derived from the run's event log — the timestamp
of the most recent recorded event plus a human rendering of it — so a long
autonomous run is visibly alive rather than indistinguishable from wedged.
No new state is stored, and a stale timestamp is itself the signal to look
closer. (A `speccy status --watch` polling mode is a later capability.)

A run waiting on a human leads with the action:

```text
SPEC-20260630-A7F4  Passwordless login          Risk: high
  Ready to ship · 1 accepted risk
  Next: /speccy-ship
```

A submitted run keeps the manual acceptance step visible until the human
records the merge:

```text
SPEC-20260630-A7F4  Passwordless login          Risk: high
  Awaiting merge — PR #123 open
  Next: speccy accept   (after the PR merges)
```

An interrupted run — expired lease, no active session — surfaces resume
attribution before the human re-enters (see "Resume and Crash Recovery"):

```text
SPEC-20260630-A7F4  Passwordless login          Risk: high
  Interrupted — session died mid "token model + endpoints" (repair round 2)
  Uncommitted diff (3 files, +58 −4 vs f3d9e21) belongs to that task on resume
  Next: /speccy-implement
        (stash or commit first if these edits are not the worker's)
```

Internal controller operations still exist, but they are tool calls used by the harness pack, not ordinary human-facing workflow commands.

`speccy list` should default to active specs in the current workspace: drafts, approved specs, specs with active runs, escalated specs, specs awaiting review, or repairable validation failures. Accepted, superseded, cancelled, and archived specs should be hidden unless the user passes an explicit flag such as `--all`, `--accepted`, `--status accepted`, or `--archived`.

`--query` should apply the same selector matching used by commands such as `speccy review passwordless`, but without taking an action. This lets users preview which specs would match a natural selector:

```bash
speccy list --query passwordless
speccy list --query "auth expiry"
speccy list --all --query login
speccy list --accepted
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
Result verified — ready to ship · 2 accepted risks
Recommended next action: /speccy-ship

Requirements (11)
  Proven          9
  Accepted risk   2   1 waived · 1 on review-only evidence

Accepted risk
  R-SEC-002    waived                Email enumeration mitigated   — "constant-time deferred, tracked in follow-up"
  R-EMAIL-001  review-only evidence  Email delivery integration reviewed — staging send not run locally

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

Requirement statuses collapse into three human status buckets — **Proven** (`passed`), **Accepted risk** (`review_passed`, `waived`), and **Needs you** (`failed`, `blocked`, `pending`) — a rendering rule, with the precise status kept as an inline tag on drill-down. The first screen never prints requirement-status enum values: `review_passed` renders as "review-only evidence" and `waived` as "waived"; the raw enum lives in JSON and `--evidence` drill-down. Proven is collapsed to a count and never enumerated. When the accepted-risk bucket is non-empty, its count appears on the result line itself (`verified — ready to ship · N accepted risks`): a bare "ready to ship" must never hide residual risk below the fold. `/speccy-ship` re-echoes those lines and asks one explicit confirmation before opening the PR (see "Harness Skills"), so a ship recorded days later in a fresh session still puts the risks in front of the human and gets a deliberate yes.

A verified run has an empty **Needs you** bucket by construction. A fixable failure never waits behind a button; it loops autonomously until it is proven or the run escalates. So the summary is a result, not a menu, and it carries no approve/reject/repair/waive controls.

An **escalated** run produces the escalation packet instead: scoped to the one requirement the run could not satisfy or prove, ending in a single question the human answers in prose. See "Escalation Packet."

The two decisions a human actually makes:

- **Ship it.** Invoke `/speccy-ship` to open the PR and move the run to `submitted`. The PR is merged normally, and the human records the merge with `speccy accept`, moving the run to `landed`.
- **Send it back.** Describe what is wrong in prose. There is no `speccy reject`; feedback is conversational, and the skill routes it down one of the two paths below — echoing what it is about to record first, like any other gate decision.

Post-verification feedback has two routes, both controller-recorded:

- **Minor implementation feedback** — no change to scope, requirements, or risk — stays in the same run as a `rework` decision: `run record-decision` (type `rework`, feedback prose required) moves the run `verified -> implementing` and appends a dynamic `RT<n>` task seeded with the feedback, counted against `run_review_rounds`. The rework round runs the normal claim → dispatch → handoff → verify cycle and returns to the same ship gate.
- **Scope or requirement feedback** creates a spec amendment and a new run, because the definition of done changed: `spec patch-draft` → amended spec card → prose approval recorded by `spec record-decision` (`supersedes.run_id`), which atomically closes this run as `cancelled` with a linking decision record. Until that approval lands, nothing is recorded and the run stays at the ship gate (see "Amendment at the Escalation Gate" — the same transaction).

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

- The user starts a fresh harness session and invokes `/speccy-implement`, adding a selector only when there is more than one plausible run. Speccy makes no outbound calls, so it never reattaches to or relaunches a harness session itself.
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
- Requirements passed, review-passed, failed, blocked, and waived.
- Task-scoped repair round count.
- Run-level repair loop count.
- Capability-escalation give-up events, with the requirement IDs that triggered them.
- Commands run and exit codes.
- Files touched per task.
- Drift events.
- Human gate decisions.
- Validator disagreement.
- Findings by reviewer persona.
- Provenance scan hits.

These metrics should feed both local improvement and product evaluation.

## MVP Proposal

MVP should be intentionally narrow. One scope decision is deliberately not narrow: Claude Code and Codex are both first-class MVP harnesses. Shipping two targets forces the template renderer's conditional exports to be real from day one rather than speculative single-target code. Brainstorm remains optional by design.

The MVP list:

1. Local Speccy controller with run store external to the target repo.
2. Built-in harness-aware template renderer with shared partials, Codex/Claude
   conditional exports, strict variables, and golden render tests.
3. Repo-local Codex install pack: entry skills (brainstorm, plan, implement, ship) at `.agents/skills/`, role/subagent definitions at `.codex/agents/*.toml`.
4. Repo-local Claude Code install pack: the same entry skills at `.claude/skills/`, subagent definitions at `.claude/agents/*.md`.
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
15. Configurable reviewer persona roster (default: `spec-fidelity`, `defects`, `security`, `style`) rendered as per-persona subagent files with per-persona model selection, fanned out in task review and run-gate review.
16. Deterministic provenance scan over task diffs and the integrated run diff.

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

- Delta-scoped re-review — round snapshots (dangling commits at each handoff) and per-round `delta` diffs so a repair-round reviewer reads only the changed slice. Deferred from MVP: it is a token optimization unproven against real reviewer cost, and full-diff re-review is correct and simple. Dogfood roster-cost measurement decides whether it earns its machinery (see "Repeat Review Rounds").
- Command-evidence dedup — a per-round cache keyed on the command string so repeated `evidence collect` of an unchanged command returns the recorded artifact instead of re-executing. Deferred from MVP: at serial-write scale the workspace command lock already serializes runs, and the cache is unproven optimization.
- Decision index — a derived projection surfacing carry-forward decisions from archived specs into planning packets, with a rendered cap and overflow drill-down. Deferred from MVP: it matters only once a workspace archives specs, which the single-spec MVP does not exercise. The `carry_forward` flag is recorded from day one so the projection needs no data migration (see "Carry-Forward Decisions").
- Automatic merge detection — git-native ancestry checks, squash-merge heuristics, and an optional configurable host probe — so `submitted -> landed` can be recorded without a manual `speccy accept`. Cut from MVP because it is an external-integration convenience, not core to the loop.
- Optional MCP server exposing `speccy` to clients where MCP is worth the token overhead.
- User-level skills/commands for Codex and Claude Code through `speccy install --user`.
- Importers for OpenSpec, Spec Kit, Kiro-style specs, GSD Core, and other repo-local spec formats.
- Exporters that write `speccy` specs and acceptance ledgers into those formats when a team explicitly wants repo-local artifacts.
- Web dashboard for long-running runs.
- `speccy status --watch`: live-refreshing status cards for long autonomous runs (the static card's last-activity line covers MVP).
- Worktree-based parallel experiments.
- Browser validator integration.
- GitHub issue/PR integration.
- Deterministic CI checks for specs, ledgers, review packets, and pack freshness.
- Policy packs for regulated environments.
- Optional team-shared run store for enterprise/audit use, only after no-server review packets and run bundles prove insufficient.
- If any mutable state ever becomes git-visible (for example, a team-shared mode committing state snapshots), it must be append-only with a union-by-event-id git merge driver; replace-style merges of state files silently lose data (per the external runtime-state storage survey, OpenSpec vs Spec Kitty).
- Additional exports — lessons learned, acceptance snapshots, result summaries, raw run logs — if dogfooding proves the review packet and spec export are not enough.
- Generic Agent Skills fallback pack: core-fields-only `SKILL.md` files for other `.agents/skills/` readers (Amp, OpenHands), plus a root `AGENTS.md` pointer — the maximally compatible cross-harness shape.
- Model routing and budget optimizer.
- Inbound Agent2Agent-compatible bridge owned by an external harness, if a team proves it adds value without moving orchestration state out of Speccy.
- Reusable evidence templates, if real usage proves they reduce friction.
- Worker self-editing skills for in-run continuous learning, added only after the single-spec MVP shape is proven. If run horizons ever extend beyond hours, per-repo skill self-evolution becomes a prerequisite per the Factory analysis, not an enhancement.

## Open Questions

Live open questions (Q2–Q24 numbering) and dogfood watch items are tracked
in `OPEN-ITEMS.md`; resolved decision history is archived in
`DECISION-LOG.md`. Build order and the walking-skeleton milestones live in
`IMPLEMENTATION-PLAN.md`.
