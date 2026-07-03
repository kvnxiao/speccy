# Speccy Domain Terminology

Status: authoritative
Date: 2026-07-03

This document defines the Speccy vocabulary used by product docs, CLI commands,
controller APIs, install packs, exported artifacts, and review packets.

The short version:

- A **spec** is the primary user-facing object.
- A **run** is one execution attempt against an approved spec revision.
- An **initiative** is an optional grouping of related specs.
- A **decision record** captures important scope, evidence, waiver, and
  architecture decisions.
- A traditional **ADR** is an optional exported subset of decision records.

Speccy should not feel like process software. Users should not have to create a
"mission" before they can brainstorm a spec. The CLI should let users start from
plain engineering intent, while the controller quietly creates the state needed
for resumption, evidence, decisions, and review.

## Concept Hierarchy

```text
workspace
  -> initiative (optional)
       -> spec
            -> public spec reference and title
            -> spec revisions
                 -> planned tasks
            -> draft workflow
            -> acceptance ledger
                 -> requirements
                      -> evidence requests
                      -> evidence artifacts
                      -> requirement status
            -> runs
                 -> task graph (instantiated from the approved revision's planned tasks)
                 -> worker handoffs
                 -> validator findings
                 -> decision records
                 -> review packet
```

For MVP, `initiative` can be omitted. A workspace can simply have specs, and
each spec can have one or more runs.

## Canonical Terms

### Speccy

Speccy is a spec-driven run controller for coding agents.

It does not replace Codex, Claude Code, Cursor, Copilot, OpenHands, or a custom
harness. It installs into those harnesses where possible, supplies deterministic
state and evidence tools, and delegates implementation and semantic verification
to the active harness. The direction of control matters: the active harness calls
Speccy controller operations; Speccy commands do not call LLMs, coding agents, or
AI harnesses.

Use:

- "Speccy"
- "Speccy controller"
- "Speccy install pack"
- "spec-driven run controller"

Avoid as canonical product language:

- "Speccy mission controller"
- "Speccy IDE"
- "Speccy spec folder framework"

### Workspace

A workspace is the target directory or repository a spec/run operates against.

A workspace is a git repository or a subtree of one, such as a monorepo
package. Non-git directories are not supported: resume and evidence baselines
depend on git snapshots and `baseline_commit`. Speccy runtime state always
lives outside the workspace, in `~/.speccy/`. Workspace identity derives from
the workspace root and git root paths; see "Storage Model" in `DESIGN.md`.

### Initiative

An initiative is an optional grouping of conceptually related specs.

Use an initiative when the user's intent is too broad to be one coherent spec but
has a shared product or technical direction.

Example:

```text
Initiative: Modernize authentication
  Spec: Passwordless login
  Spec: OAuth account linking
  Spec: Session expiration hardening
  Spec: Auth audit logging
```

An initiative is not required for normal use. It should not be an MVP ceremony.
If the product can avoid asking users to create initiatives manually, it should.

### Spec

A spec is the primary user-facing object in Speccy.

It describes one coherent change, repair, or capability with a clear definition
of done. A spec starts as a brainstormable draft, becomes binding when approved,
and can then be implemented, verified, reviewed, exported, or rerun.

A spec answers:

- What are we trying to make true?
- Who or what is affected?
- What is in scope?
- What is out of scope?
- What assumptions are being made?
- What constraints must the implementation respect?
- What user-visible or system-visible behavior matters?
- What evidence would make us comfortable calling this done?

A spec is not:

- A permanent project
- A vague product vision
- A whole roadmap
- A raw prompt
- A chat transcript
- A task list by itself

The practical scale rule:

> A spec should describe one meaningful capability or repair that can be completed,
> reviewed, and judged independently.

If the work is not independently valuable, it is probably a task inside a spec.
If the work contains several independently valuable outcomes, it is probably an
initiative containing multiple specs.

### Spec Reference

A spec reference is the stable public handle for a spec.

It is the one spec identifier users may see in review packets, exports, PR
metadata, and scripts. It should be copyable, reasonably short, and portable
across independently created local stores. Avoid repo-global sequential counters
unless there is a shared store that can allocate them safely.

Recommended examples:

```text
SPEC-20260630-A7F4
SPEC-20260702-C91B
```

Rules:

- The controller creates the spec reference.
- The reference is stable for the lifetime of the spec.
- The reference should be globally unique enough for offline creation and later
  merge/export.
- The reference is user-addressable, but routine commands should infer the
  current spec when possible.
- A title is human-facing and mutable.
- A title-derived slug may appear in paths for readability, but the slug is not
  an identifier and does not need to be unique by itself.

Recommended shape:

```yaml
spec_ref: SPEC-20260630-A7F4
title: Passwordless authentication
slug: passwordless-auth
```

### Spec Selector

A spec selector is user input used to find a spec. It is not identity.

Examples:

```bash
speccy list
speccy list --query passwordless
speccy review
speccy review passwordless
speccy review "Passwordless authentication"
speccy review SPEC-20260630-A7F4
```

Behavior:

- No selector means use the only active/current spec when unambiguous.
- A full spec reference resolves exactly.
- Other text is treated as a search query over titles, slugs, spec references,
  and recent review metadata.
- `speccy list --query <text>` previews selector matching without taking an
  action.
- If multiple specs match, the CLI or harness should print a short numbered list
  and ask the user to choose.
- Scripts, CI, exported rerun instructions, and PR metadata should use the full
  spec reference.

### Spec Revision

A spec revision is an approved or draft snapshot of spec intent.

There is one active approved revision for a run. When scope changes materially,
create a new revision and record a decision explaining why the definition of done
changed.

An approved revision is immutable in place, including its acceptance ledger:
requirement statements and evidence requests are frozen at approval. Agents may
only propose draft patches; human prose approval creates the new revision.
Verifiers change requirement status only, through evidence operations.

Suggested IDs:

```text
spec_rev_001
spec_rev_002
```

### Spec Status

Spec status controls whether old specs are considered by default during future
planning.

Canonical statuses:

- `draft`: not yet approved.
- `approved`: accepted as the binding definition of done for a run.
- `cancelled`: abandoned by a human decision before completion.
- `accepted`: completed and accepted by the human, with any residual risk called
  out.
- `superseded`: replaced by a later spec or decision.
- `archived`: retained for history, but excluded from default planning context.
  Archiving also covers accepted specs that no longer describe the current
  codebase; the planner flags staleness during reconciliation rather than
  relying on a dedicated status.

Only relevant specs that are not cancelled, superseded, or archived should be
candidates for carry-forward context. The planner must still reconcile them
against the current codebase before treating them as constraints.

### Active Spec

An active spec is a spec that may still need work, review, repair, or human
attention in the current workspace.

Default active specs include:

- Draft specs.
- Approved specs that have not completed.
- Specs with active runs.
- Escalated specs.
- Specs awaiting final review.
- Specs with validation failures that can be repaired.

Accepted, superseded, cancelled, and archived specs should not appear in the
default active list. They can still be shown through explicit flags such as
`--all`, `--status accepted`, or `--archived`.

### Run

A run is one attempt to implement and verify an approved spec revision.

The run is the execution container. It owns runtime state, scheduling, task
graph, handoffs, evidence artifacts, validator findings, decision records,
resume behavior, and the final review packet.

A run is not the user-facing goal. It is the operational attempt to satisfy the
spec.

Drafting, revising, or approving a spec is not itself a run. A run is created
only after a spec revision is approved, normally by invoking
`/speccy-implement <spec>`.

Runs are useful because a spec may need:

- A first implementation attempt.
- A repair attempt after validation fails.
- A rerun after dependencies or environment setup change.
- A fresh verification attempt against a branch or pull request.
- A later implementation attempt after a spec revision.

### Run ID

A run ID is the stable, opaque handle for one run.

Rules:

- The controller creates the ID.
- The ID is stable for the lifetime of the run.
- The ID is never reused.
- The ID should be filesystem-safe and copyable.
- The ID should be treated as opaque.
- A readable title should live separately from the ID.

Recommended examples:

```text
run_01j1bxgvk3tf4qs6mv9zpxwe8d
```

Use `run_id` in JSON and persisted runtime snapshots. Use `--run <id>` in
harness-facing controller commands when precision is required.

### Run State

Run state is a single flat enum. Progress within a state is read from the task
graph and acceptance ledger, not a second phase field.

Canonical states:

- `created`: the run exists against an approved spec revision but has not
  started implementation.
- `implementing`: the serial task loop is running.
- `verifying`: final validation, drift review, and run-level repair.
- `verified`: verification passed; the work is done and awaiting the human's ship decision.
- `escalated`: autonomous progress stopped and the run needs a human decision,
  such as a spec amendment, environment setup, waiver, or cancellation.
- `submitted`: the change is proposed and awaiting review and merge.
- `landed`: the change merged, recorded by `speccy accept`.
- `cancelled`: a human stopped the run.

Human checkpoints are: spec-card approval before run creation (a prose approval
recorded by `/speccy-plan`), `verified` to ship via `/speccy-ship`, `escalated`
to amend/waive/unblock/cancel, and `submitted` to review and merge the PR, then
record the merge with `speccy accept`. The rest are autonomous.

Do not reuse status words across enums. `submitted` is used instead of
`pending` because `pending` is already a requirement status, and `landed` is
used instead of `accepted` because `accepted` is already a spec status and
"acceptance" already names the ledger.

Archiving is a spec visibility action, not a run state. A landed run remains
`landed`; `speccy archive` marks the accepted spec as archived so it no longer
appears in default active lists.

### Change Reference

A change reference records what a run proposed — kind (PR/branch/patch), URL,
branch, head SHA, and base — as provenance for the eventual landing. MVP does
no merge detection: the human records the merge with `speccy accept`, which
moves the run to `landed`. The field shape and acceptance flow live in
"Acceptance" in `DESIGN.md`.

### Requirement

A requirement is an atomic, checkable claim derived from the spec.

Requirements should be plain English and stable enough to map to tasks and
evidence. A requirement can be behavioral, structural, compatibility-related,
security-related, documentation-related, or process-related.

Examples:

```text
R1: Existing CSV exports keep the same columns.
R2: Timestamps are formatted as ISO-8601 UTC values.
R-AUTH-004: Expired magic links are rejected.
```

Requirement IDs are local to a spec. They are not global product IDs.

### Acceptance Ledger

The acceptance ledger is the binding requirements-to-evidence record for a spec.

It answers:

- What did the user require?
- What changed to satisfy it?
- What evidence was requested?
- What evidence was collected?
- What passed, failed, was waived, is blocked, or remains unproven?
- What residual risk remains?

The acceptance ledger is Speccy's central artifact. A task is not truly complete
unless its linked requirements are resolved in the ledger.

Use "acceptance ledger" as the product term. "Validation contract" is a useful
research analogy, but it sounds heavier and should not be the primary Speccy
artifact name.

### Evidence Request

An evidence request is the planned proof needed for a requirement.

Examples:

- Run a command and capture exit code, stdout, and stderr.
- Inspect a changed file or diff.
- Exercise a browser flow.
- Check an API response.
- Capture a screenshot.
- Ask a fresh-context verifier to review a claim.
- Ask a human to make a manual decision.

An evidence request is not proof by itself. It becomes useful only when evidence
is collected and reviewed for adequacy.

### Evidence Artifact

An evidence artifact is collected proof or attempted proof.

Examples:

- Command record
- Test output
- Diff snapshot
- Screenshot
- Browser trace
- API response
- Verifier finding
- Human waiver

Evidence artifacts should include enough metadata to support review: collector,
timestamp, command or procedure, result, artifact reference, and residual risk
where applicable.

For `kind: command` evidence, the collector is the controller itself:
`speccy ctl evidence collect` executes the command and records exit code,
stdout, stderr, and a content hash, and `evidence record` refuses agent-pasted
output for that kind. Review, browser, and manual evidence remain agent- or
human-collected and are treated as weaker by the risk tiers. On `high` and
`critical` specs, browser and API evidence must reference a stored artifact
(screenshot, trace, DOM capture, HTTP transcript), not prose alone; the rule
lives in "Acceptance Ledger" in `DESIGN.md`.

### Requirement Status

Requirement status records the current evidence-backed judgment for a requirement.

Canonical statuses:

- `pending`: evidence has not been collected yet.
- `passed`: collected evidence satisfies the requirement at the required risk
  depth.
- `review_passed`: review evidence satisfies the requirement, with residual risk
  recorded.
- `failed`: evidence contradicted the requirement.
- `vacuous`: evidence exists but does not actually exercise the requirement.
- `blocked`: evidence could not be collected due to environment, tooling, access,
  or missing dependency.
- `unproven`: no acceptable evidence exists yet.
- `waived`: a human explicitly accepted the risk.

Avoid using task completion as a proxy for requirement status.

Legal transitions, evidence prerequisites, and tier resolution rules are
defined in "Requirement Resolution Rules" in `DESIGN.md`.

### Human Status Bucket

A human status bucket is the coarse rollup of requirement status shown at human
checkpoints. It reduces the eight requirement statuses to three so a reviewer
reads by exception instead of scanning an enum. The precise status stays as an
inline tag on drill-down.

- **Proven**: hard evidence holds. Maps `passed`.
- **Accepted risk**: resolved without hard evidence, worth a glance. Maps
  `review_passed` and `waived`.
- **Needs you**: not resolved. Maps `failed`, `vacuous`, `blocked`, `unproven`,
  and `pending`.

Buckets are a rendering rule for review packets and cards, not a stored value.
The canonical status is always the requirement status. A verified run has an
empty **Needs you** bucket. Blocked, unproven, failed, vacuous, or pending
requirements stop at an escalation or policy gate until they are resolved,
waived, or converted to review-passed with explicit residual risk.

### Run Status Label

A run status label is the coarse rollup of run state shown on `speccy status`
cards, the sibling of the human status bucket: humans read phases, not the
run-state enum.

- **Implementing**: maps `created` and `implementing`; shown with the current
  task and round.
- **Verifying**: maps `verifying`.
- **Ready to ship**: maps `verified`.
- **Needs you**: maps `escalated`.
- **Awaiting merge**: maps `submitted`.
- **Interrupted**: any active state whose run lease has expired with no
  session holding it; shown with resume attribution (see "Resume and Crash
  Recovery" in `DESIGN.md`).

Labels are a rendering rule for status cards, not a stored value. The
canonical state is always the run state; `landed` and `cancelled` runs are
not active and get no card. Card behavior lives in "CLI/Admin Flow" in
`DESIGN.md`.

### Task

A task is a bounded implementation unit mapped to one or more requirements.

A task should be small enough to hand to a worker and small enough to review. It
is a scheduling and implementation unit, not the definition of done.

A worker attempt can finish while requirements remain unproven or failed. That
is useful evidence about what happened, but it is not task acceptance.

A task reaches `integrated` only after its linked requirements are
resolved enough for the selected risk tier: passed, review-passed, waived, or
explicitly deferred by a recorded human/policy decision. Task completion is not a
proxy for requirement satisfaction.

A spec is normally broken into one or more tasks for execution. The planned
task breakdown belongs to the approved spec revision. A run instantiates its
own runtime task graph from that plan when `/speccy-implement` creates the run;
rounds, assignments, and task status live on the run. A new revision produces a
new run with a fresh task graph.

### Task Status

Task status is the runtime task graph's per-task state, kept alongside the
controller-owned round counter so an interrupted session can resume mid-task.

Canonical statuses:

- `queued`: not started.
- `building`: an implementer currently owns the task.
- `reviewable`: a handoff is recorded; review has not started.
- `in_review`: fresh-context reviewers are checking linked requirements.
- `needs_repair`: review failed and the repair cap is not exhausted; the next
  round re-enters `building`.
- `integrated`: linked requirements resolved for the risk tier; the controller
  records a git snapshot commit for the task.
- `deferred`: explicitly set aside by a recorded human/policy decision.

Task statuses are deliberately disjoint from run states, spec statuses, and
requirement statuses so no status word means two things.

`baseline_commit` capture at claim time and snapshot commits at `integrated`
are controller mechanics defined in "Task" and "Resume and Crash Recovery" in
`DESIGN.md`.

### Task Round

A task round is the nested implement-review loop for one task.

Recommended shape:

```text
build task packet
  -> worker implements task
  -> worker records handoff
  -> fresh task reviewer/verifier checks linked requirements
  -> task repair round if needed
  -> task integrated
```

Task rounds run serially by default. The scheduler should not move to the next
write task until the current task is `integrated` or explicitly
deferred. Blocked or unproven linked requirements must be surfaced as a gate
rather than silently treating the task as complete.

Task review is narrower than final run review. It checks a bounded task,
handoff, diff, and linked requirements. It does not replace the final drift and
integration review for the whole spec.

### Worker

A worker is a harness session assigned to implement a task.

The worker receives a scoped task packet and returns a structured handoff. The
worker should not be the only validator of its own work.

### Validator

A validator is a fresh-context harness session, deterministic runner, or human
reviewer that checks work against the acceptance ledger.

Validator types can include:

- Command validator
- Test validator
- Browser or UI validator
- Code review validator
- Security validator
- Documentation validator
- Requirement-coverage validator
- Vacuity validator
- Human validator

Fresh-context code-review validation fans out as reviewer personas; see
"Reviewer Persona".

### Reviewer Persona

A reviewer persona is a named review lens dispatched as a fresh-context
subagent during task review rounds and run-gate review rounds.

The default roster is `spec-fidelity`, `defects`, `security`, and `style`.
The roster, per-persona model selection, pack rendering, and round scoping
are defined in "Reviewer Personas" and "Repeat Review Rounds and Token
Scoping" in `DESIGN.md`.

Personas record findings and non-command evidence lease-free; the lease
holder aggregates. "Persona" always means a reviewer lens, never a worker or
planner role.

### Handoff

A handoff is the worker's structured report after attempting a task.

It should include:

- Task ID
- Summary of changes
- Files touched
- Commands run
- Exit codes
- Requirements claimed satisfied
- Known issues
- Deviations from the plan
- Follow-up recommendations

A handoff is evidence about worker behavior. It is not final proof that the spec
succeeded.

### Finding

A finding is a structured validator observation.

Findings can be positive, negative, uncertain, blocked, or advisory. Negative
findings should name the affected requirement IDs where possible. A finding
records which reviewer persona produced it, when one did.

### Provenance Scan

The provenance scan is the deterministic controller check that product file
contents carry no Speccy terminology or identifiers — no `speccy`, spec
references, or requirement/run/task IDs in shipped source, comments, tests,
docs, or config.

It runs over each task diff at verification and over the integrated diff at
final validation; a hit records a blocking finding. Scope, exemptions, and
the deny-list live in "Provenance Hygiene" in `DESIGN.md`.

### Review Packet

The review packet is the compact human-facing summary of a run.

The default first screen should include:

- Spec summary.
- What changed.
- Acceptance status.
- Evidence summary.
- Findings and residual risk.
- Drift from the approved spec.
- One recommended next action.

Detailed evidence, command logs, validator findings, decision records, and the
full ledger should remain available through drill-down or export rather than
crowding the default checkpoint. The review packet exists so humans do not need
to replay transcripts.

### Escalation Packet

The escalation packet is the focused handback produced when a run gives up on a
requirement. It is distinct from the review packet: it is scoped to the
requirement that could not be satisfied, not the whole run.

It frames the stop as a requirement or evidence-strategy problem, not a generic
agent failure: exhausted autonomous repair usually means the approach or the
requirement is wrong, so the natural resolution is a spec amendment. The
ordinary human choices are: amend the spec, provide missing setup or evidence,
waive the requirement, or cancel the run.

Contents and user-facing copy are specified in "Escalation Packet" in
`DESIGN.md`.

### Capability Escalation

A capability escalation is the event where autonomous repair exhausts its round
cap and the run stops, transitioning to the `escalated` run state. Rounds are
counted on the task; the give-up is judged on the requirement. The counters,
caps, and give-up rule are controller mechanics defined in "Capability
Escalation and Give-Up Policy" in `DESIGN.md`.

A run leaves `escalated` in one of three ways: the human provides setup or a
waiver and the same run resumes, an approved spec amendment supersedes it with
a new run and the escalated run closes as `cancelled` with a linking decision
record, or the human cancels it.

The `escalated` run state is separate from the `blocked` requirement status. A
run enters the `escalated` state after a capability escalation, even though the
triggering requirement is `failed` or `vacuous`. The `blocked` requirement
status means evidence could not be collected due to environment or tooling.
Blocked or unproven requirements can also move a run to `escalated` when they
prevent verification, but that is a human/policy gate rather than a capability
escalation event.

### Decision Record

A decision record is a run-relevant historical decision captured during spec
drafting, implementation, validation, repair, or review.

Examples:

- Approve spec
- Waive requirement
- Change scope
- Mark a requirement obsolete
- Accept residual risk
- Choose an architecture approach
- Retry validation
- Cancel a run

Decision records should be retained in the run store and summarized in the review
packet when they affect scope, risk, or future maintainers.

Suggested shape:

```yaml
id: dec_20260630_001
type: architecture
status: accepted
title: Use magic-link tokens instead of numeric codes
linked_spec_revision: spec_rev_002
linked_requirements:
  - R-AUTH-001
  - R-AUTH-004
actor: human
timestamp: 2026-06-30T20:22:00Z
context: "Users need passwordless login without OAuth."
decision: "Generate single-use email tokens stored hashed in the database."
consequences: "Requires expiry, replay prevention, and email delivery evidence."
```

### ADR

An architectural decision record, or ADR, is a durable project-facing record for
an architectural decision.

In Speccy, ADRs should be optional exports from decision records, not mandatory
runtime artifacts. Most run decisions are too operational to deserve checked-in
ADRs. Promote only decisions that future maintainers should know.

Default:

```text
decision records live in Speccy's run store
```

Opt-in export:

```text
docs/adr/0007-use-magic-link-tokens.md
```

### Waiver

A waiver is a human decision to accept risk for a requirement without sufficient
evidence.

Waivers should include:

- Requirement ID
- Reason
- Approver
- Timestamp
- Residual risk

Waived is a resolved status, but not a passed status. A waiver recorded at a
gate sets the requirement status to `waived` atomically inside
`run record-decision`; this is the one status-mutation path outside
`requirement set-status`.

### Drift

Drift is divergence between the approved spec, plan, task assignment, or evidence
obligations and what the worker actually changed or claimed.

Drift is not always bad. Some drift reflects necessary discovery. Speccy's job is
to make drift visible, record decisions, and prevent silent expansion of scope.

Final drift review is a run-level review after serial task rounds. It checks the
integrated result against the approved spec, task handoffs, decisions,
acceptance ledger, and final diff. This is separate from task review, which is
the fresh review loop for one bounded task.

### Risk Tier

Risk tier controls evidence strictness, not the shape of the workflow.

Canonical tiers:

- `minimal`
- `standard`
- `high`
- `critical`

Higher risk means stronger evidence, more explicit adequacy review, and more
human gates. It should not create a completely different process vocabulary.

The lowest tier is `minimal`, not `tiny`: `tiny` is a scope-rating size (how
big the request is), while the risk tier measures evidence strictness. A spec
can legitimately be scope `large` with risk `minimal`, and the two scales must
not share words.

### Controller

The controller is Speccy's deterministic core.

It manages state, gates, scheduling, evidence bookkeeping, artifact storage,
resume behavior, and review packet generation. It should not pretend to be the
semantic judge of high-level English scenarios.

### Controller Operation

A controller operation is a machine-facing command used by installed skills,
subagents, custom harness clients, or tests.

Examples:

```bash
speccy ctl run next --run <id> --agent <id> --json
speccy ctl task record-handoff --run <id> --lease <token> --input handoff.json --json
speccy ctl evidence collect --run <id> --requirements R1,R2 --json
speccy ctl packet review --run <id> --json
```

The full operation list lives in "Controller API Surface" in `DESIGN.md`.
These are not ordinary human workflow commands.

### Next-Action Directive

A next-action directive is the controller's deterministic answer to "what is
the next required step for this run."

It is produced by:

```bash
speccy ctl run next --run <id> --agent <id> --json
```

The directive names the current run state, the single next action, the subject
task or requirement, the repair-round counter with its policy-configured cap,
the packet operation to build first, the controller operation that must
record the outcome, and the derived transitions the call itself applied
(`applied_transitions`, with snapshot SHAs where created). Installed skills
drive the entire implement loop by repeating: call `run next`, perform the
directive, record the result, ask again.

The directive's exact fields, idempotency rules, and round-cap handling are
defined in "Deterministic Loop Driving: run next" in `DESIGN.md`.

### Run Lease

A run lease is the controller's enforcement of "one writer at a time" for a
run: an agent-bound, expiring token required by state-mutating controller
operations. Additive operations — `finding record` and non-command
`evidence record` — are lease-free so concurrent reviewer personas can
complete simultaneously; `evidence collect` for `kind: command` takes the
workspace command lock instead of the lease. Token
issue/renewal, `lease_held` errors, expired-lease repair, the per-file write
layout, and finding carry-over are defined in "Run Lease and Concurrent
Writers" in `DESIGN.md`.

### Planning Packet

A planning packet is a deterministic controller-to-harness work order for
drafting or revising a spec.

It is produced by:

```bash
speccy ctl packet planning --spec <ref> --json
```

The planning packet should not call an LLM and should not be just a blank
template. It contains known spec draft state, the original user request, workspace
signals that the controller can collect deterministically, relevant prior
context candidates, policy constraints, risk guidance, and the output contract
for a candidate spec draft.

The packet is JSON because it is a machine-facing controller interface. It may
point to Markdown or YAML artifacts, but it is not the canonical spec itself.

### Intake Observations

Intake observations are read-only findings recorded by a harness planner before
or during spec drafting.

Examples:

- Current auth code lives under `src/server/auth/`.
- Prior spec `SPEC-20260630-A7F4` still appears relevant, but its file paths are
  stale.
- The repository has `npm test`, `npm run lint`, and `npm run dev` scripts.
- There is no local email provider configuration.

Observations help with resumability and review. They are evidence about what the
planner saw, not binding requirements. They are submitted as an optional field
of the candidate spec draft, not through a separate controller operation.

### Candidate Spec Draft

A candidate spec draft is the harness-authored proposal for a spec revision.

It should be submitted as one complete candidate artifact, then structurally
linted by Speccy. The controller should avoid section-by-section append commands
such as "append goal" or "append non-goal" because they create many half-valid
intermediate states.

Focused edits can use patch-style updates:

```bash
speccy ctl spec patch-draft --spec <ref> --input spec-patch.json --json
```

Speccy validates draft structure, such as missing required sections, duplicate
requirement IDs, invalid risk tiers, or requirements without evidence requests.
The harness and human remain responsible for semantic quality.

### Brainstorm Handoff

A brainstorm handoff is the structured output of a non-binding brainstorm
session. It is ephemeral chat context by default, not a repo-persisted file.

It can include candidate goals, non-goals, requirements, codebase observations,
prior-context notes, suggested splits, risk notes, open questions, and a
recommended next action. The recommended next action should include the exact
phrase or command the user can use to continue. It is not a spec, not an
approved plan, and not an acceptance ledger.

The user can explicitly route a brainstorm handoff to direct agent work, the
harness's regular planning flow, or a Speccy spec. Speccy persists the handoff
only if `/speccy-plan` promotes it into a spec, the user exports it, or it is
passed into a workflow that normally persists plan artifacts. `/speccy-plan`
treats the handoff as context, not truth, and still drafts against the current
codebase, relevant prior specs, the current user request, and project policy.

### Scope Rating

A scope rating is the brainstorm skill's rough size and risk classification for
the user's request.

Recommended values:

- `tiny`: obvious direct edit.
- `small`: bounded change, usually direct edit or a light plan.
- `medium`: normal harness planning is useful, but a Speccy ledger is probably
  unnecessary.
- `large`: one coherent Speccy spec is likely worthwhile.
- `initiative`: split into multiple specs because the request contains several
  independently valuable outcomes.

The first factor is always evidence-ability: can anyone articulate how the
result will be validated? Low evidence-ability routes away from `speccy_spec`
even for large work. The full factor schema and routing defaults live in
"Scope Rating" in `DESIGN.md`.

The rating is advisory. The user can override the recommendation.

### Route Recommendation

A route recommendation is the brainstorm skill's suggested next workflow:
`direct_edit`, `harness_plan`, `speccy_spec`, or `split_specs`.

It should be shown with a human label, a short reason, confidence, alternatives,
and an exact next action phrase or command rendered for the active harness. For
example, a Claude Code handoff can say `/plan Continue from the Speccy brainstorm
handoff above...` or tell the user to switch to plan mode with `Shift+Tab`; a
Codex handoff uses Codex's own `/plan` Plan Mode command (both harnesses expose
one, verified 2026-07-03).

The `speccy_spec` route's next action is `/speccy-plan`. The `harness_plan`
route points to the harness's own plan mode, not `/speccy-plan`.

Route recommendation does not approve a spec or start a run. It only helps the
human choose the next interaction checkpoint.

### Autonomous Execution

Every Speccy run is fully autonomous by design. After the human approves the
spec card, the harness-driven `/speccy-implement` loop continues through
implementation, verification, and repair to `verified` or `escalated` without
asking the user to steer each implementation step. There is no partially
supervised or step-steered mode.

Autonomy is not permission to ignore gates. A run must pause for policy or
environment checkpoints such as:

- Critical-tier accepted-risk confirmation before `verified`.
- Destructive filesystem actions.
- Dependency installation or network access approvals.
- Production or deployment actions.
- Critical waivers.
- Budget caps.
- Missing credentials or blocked environments.
- Material scope changes or spec contradictions.

The expected user experience is "approve the spec, then come back to a compact
review packet unless the run blocks." Autonomous does not mean policy-free.

### Human-Facing Command

A human-facing command is a command a developer may reasonably type directly.

Common examples:

```bash
speccy install
speccy doctor
speccy new "Add passwordless login"
speccy status
speccy review
speccy accept
```

The full human CLI list and per-command semantics live in "CLI/Admin Flow" in
`DESIGN.md`. There is no `speccy resume`; resume is a controller capability
reached through `/speccy-implement` in a fresh harness session.

Human-facing commands should avoid exposing internal SDLC phases as a manual
checklist. The common CLI should stay spec-first and should not require users to
think in controller operations, evidence phases, task claims, or run IDs during
routine work. No human-facing, admin, or machine-facing Speccy command should
call an LLM or launch a coding-agent harness. Commands that take a spec argument
should accept either a full `SPEC-...` reference or a search selector (see
"Spec Selector"), while scripts should use the full reference.

### Harness Skill

A harness skill is a Speccy workflow the human invokes inside Codex or Claude
Code, as an explicit slash command or by natural-language fallback. The four
entry skills:

- `/speccy-brainstorm <intent>`: optional exploration; produces a brainstorm
  handoff with a recommended route.
- `/speccy-plan <intent | handoff>`: drafts the spec and acceptance ledger,
  presents the spec card, records the human's prose approval.
- `/speccy-implement <spec>`: the autonomous implement-verify-repair loop
  against an approved revision; ends `verified` or `escalated`.
- `/speccy-ship <spec>`: opens the pull request; run -> `submitted`.

The canonical skill definitions, invocation rules, and the
no-per-control-skills rule live in "Harness Skills" in `DESIGN.md`. Harness
skills ship inside the install pack.

### Install Pack

An install pack is the harness-facing prose and glue that lets an LLM interact
with Speccy.

Packs are the integration mechanism, not a convenience. Speccy does not ship
its own harness and cannot hook a third-party harness's internal loop, planner,
or state, so skills, commands, and agent/subagent definition files that call
the controller CLI are the only universal integration surface.

Install packs can contain:

- Skills.
- Commands.
- Subagent definitions.
- Role prompts.
- Risk and evidence policies.
- Spec, acceptance, and review templates.
- Thin wrappers that call `speccy ctl ... --json`.
- Render metadata that records which harness-aware source template produced
  each managed file.

Install packs are rendered from harness-aware templates, not copied from one
neutral prompt tree. Shared partials carry neutral Speccy policy and rubrics;
target overlays carry harness-specific command syntax, file layout, and tool
names. For example, Claude rendered prose may name `AskUserQuestion`, while
Codex rendered prose may name `request_user_input`.

Repo-local install packs are the default for team workflows because they keep
the agent lifecycle prose consistent, reviewable, and tunable per repository.
Machine-global packs are a later capability, exposed through `--user` when
implemented.

`speccy install` is the single command for install, repair, check, and update
planning. Its flags, target auto-detection, idempotency rules, and the
`--update` three-way merge are specified in "Install Flow" in `DESIGN.md`.

### Harness-Aware Template

A harness-aware template is a source template that renders install-pack files
for a specific target harness and scope.

It may use shared partials for neutral Speccy behavior, plus conditional
overlays for harness-specific command syntax, tool names, frontmatter, file
paths, and capability differences. Conditional exports let one source pack
produce a Claude command file, a Codex skill file, or no file at all depending
on the selected harness.

Rendered files are the files humans review and edit in the repo. Source
templates are the upstream pack implementation. The pack lock connects the two
so Speccy can check freshness and perform conservative updates.

## Scale Guidance

Use Speccy when intent, evidence, and review are worth the overhead; skip it
when the overhead is larger than the risk. The canonical use/don't-use lists
live in "When to Use Speccy" in `DESIGN.md`.

Too small:

```text
Goal: Fix typo in README.
Requirement: README typo is fixed.
Evidence: Diff review.
```

That should usually be a direct agent edit.

Too large:

```text
Modernize authentication.
```

Better:

```text
Initiative: Modernize authentication
  Spec: Passwordless login
  Spec: OAuth account linking
  Spec: Session expiration hardening
  Spec: Admin session revocation
  Spec: Auth audit logging
```

## CLI Naming Guidance

### Should "spec" be the top-level user noun?

Yes.

Users should be able to brainstorm, approve, implement, verify, review, and
export a spec without first learning a second heavyweight noun.

Recommended framing:

```text
Speccy helps you create specs and coordinate their execution through the active harness.
Each spec may have one or more runs.
Each run produces evidence, decisions, and a review packet.
```

### Should "mission" be canonical?

No.

"Mission" is useful research vocabulary from Factory-style orchestration, but it
is ambiguous in Speccy's domain. It can mean either a strategic vision grouping
or a runtime execution attempt. Speccy should avoid that overload.

Use `initiative` for grouping and `run` for execution.

### Should spec IDs be numeric, random, or free-form slugs?

Use one generated public spec reference plus a mutable title. Do not make
free-form slugs canonical IDs.

Pure sequential references such as `SPEC-0007` are pleasant locally but become
awkward when two users create specs independently and later merge or export
snapshots. Free-form IDs such as `spec-passwordless-auth` are readable but can
collide, become stale when scope changes, and leak request text.

Recommended public shape:

```text
SPEC-20260630-A7F4  Passwordless authentication
```

The random-looking reference is for precise addressing, scripts, PR metadata,
exports, and reruns. Humans should usually rely on current-spec inference or
search selectors:

```bash
speccy list
speccy list --query passwordless
speccy review
speccy review passwordless
speccy review SPEC-20260630-A7F4
```

The title slug can appear in export paths:

```text
docs/specs/SPEC-20260630-A7F4-passwordless-auth/
```

But `passwordless-auth` is path decoration, not a supported ID.

### Should run IDs be exposed?

Yes, but only at the right layer.

Run IDs are correct for:

- Controller operations
- Harness-facing calls
- Cancel flows and controller resume behavior (`next-action` from a fresh session)
- Exporting review artifacts
- Review packet metadata
- Debugging and run bundles
- CI or noninteractive workflows

Run IDs should be optional or hidden for:

- Starting a spec inside Codex or Claude Code
- Checking the only active run in a workspace
- Reviewing the most recent run
- Exporting the current review packet

The product should behave like git in this respect: durable IDs exist and matter,
but routine commands can infer the current target.

Default review cards should not show the run ID unless disambiguation is needed.
Run IDs belong in full review metadata, exported bundles, debug output, and
explicit `--run` flows.

Recommended behavior:

```bash
speccy status
speccy status --run <run-id>

speccy review
speccy review --run <run-id>

speccy export review
speccy export review --run <run-id>
```

When more than one run is active for a workspace, the CLI should ask the user to
choose or print a short list. Scripts should pass `--run <id>`.

### Should humans type `speccy ctl run start`?

No.

`speccy ctl ...` is a machine-oriented surface for installed skills, subagents,
custom harness clients, and tests. It is acceptable for advanced users to inspect
it, but it should not be the documented ordinary workflow.

Normal interactive usage should happen inside the active harness and look like:

```text
Use Speccy to add passwordless login to this app.
```

There should not be a CLI-only advanced path that launches an agent. `speccy new
"..."` may record intent and create deterministic draft-spec state, but it must
not create a run, draft a complete spec, or implement by calling an LLM or
harness.

## Artifact Shapes

Three artifact layers exist. Their exact directory trees and commit policy are
owned by "Storage Model" and "Git Policy" in `DESIGN.md`; the definitions:

### Repo-Local Harness Pack

The default committed workflow shape: `.speccy/project.yaml` (project config
plus machine-readable policy values) and `.speccy/pack-lock.yaml` (pack version
pins plus render metadata), alongside rendered pack files under
`.agents/skills/`, `.codex/agents/`, and `.claude/`. Policy, role, and
evidence prose has no `.speccy/` folder of its own; it renders into the
harness packs. Rendered files are
reviewable, editable workflow artifacts with no product-source, build, runtime,
or production footprint.

### User-Facing Export

A compact, explicitly exported snapshot folder such as
`docs/specs/SPEC-20260630-A7F4-passwordless-auth/` holding spec, acceptance,
decisions, and review views. Never raw runtime state, transcripts, screenshots,
command logs, or secrets.

### Internal Runtime Store

The canonical execution state under `~/.speccy/workspaces/<workspace-id>/`,
always external to the repo. User-facing folders are explicit exports or
snapshots.

## Naming Pairs

| Say | Avoid | Reason |
| --- | --- | --- |
| spec | mission, prompt, PRD | Spec is the primary user-facing object. |
| spec reference | slug ID, prompt name | A generated public ref is stable and portable. |
| spec selector | alternate ID | Natural user input is search, not identity. |
| title slug | spec ID | Slugs are mutable path decoration. |
| run | mission, run instance, runtime | Run is the execution attempt without strategic connotation. |
| initiative | mission, project | Initiative cleanly names a grouping of related specs. |
| run ID | mission ID, job ID | Aligns the handle with execution state. |
| acceptance ledger | validation contract | Ledger is lighter and matches evidence/status bookkeeping. |
| requirement | assertion, criterion | Requirement maps cleanly to evidence and status. |
| evidence request | test to run | Evidence may be command, review, browser, API, manual, or blocked. |
| evidence artifact | proof | Evidence still needs adequacy review. |
| validator | reviewer only | Validation may include commands, tests, browser checks, review, or humans. |
| task round | phase | Names the nested implement-review-repair loop for one task. |
| final drift review | task review | Final drift review checks the integrated run, not one task. |
| handoff | summary | Handoff is structured and tied to task/requirements. |
| review packet | report, transcript | Packet is compact and decision-oriented. |
| human status bucket | requirement status | Coarse checkpoint rollup; the requirement status stays canonical. |
| run status label | run state, run phase | Coarse status-card rollup; the run state stays canonical. |
| escalation packet | failure report | Scoped to the unsatisfiable requirement, not the whole run. |
| capability escalation | timeout, crash | The run gave up after exhausting its repair cap, not an error. |
| decision record | audit note | Decision records are structured history. |
| ADR | any decision | ADRs are durable architecture decisions only. |
| install pack | plugin, integration | Pack can include skills, subagents, commands, and config. |
| controller operation | public command | `speccy ctl` is machine-facing. |
| `minimal` risk tier | `tiny` risk | `tiny` is a scope-rating size; risk is `minimal/standard/high/critical`. Size and strictness must not share words. |
| run `landed` | run "accepted" | `accepted` is a spec status only; the run state for a merged change is `landed`. |
| task `integrated` | task "accepted", "accepted for integration" | Task completion vocabulary must not collide with spec acceptance. |
| qualified "accept" forms | bare "accepted" | Prose must qualify every use: "acceptance ledger," "task integrated," "run landed," "spec accepted." |
| task status | run phase | `queued/building/reviewable/in_review/needs_repair/integrated/deferred` are task-graph values, disjoint from run states. |

## ID Scope Summary

| ID | Scope | User-visible? | Example |
| --- | --- | --- | --- |
| `workspace_id` | User's Speccy store | Rarely | `ws_a81f23` |
| `initiative_id` | Workspace | Rarely in MVP | `init_auth_modernization` |
| `spec_ref` | Workspace/export ecosystem | Yes | `SPEC-20260630-A7F4` |
| `spec_id` | Local runtime store | Rarely | `spec_01j1bxgvk3e6q8r2n5tcvh7pyd` |
| `spec_revision_id` | Spec | Sometimes | `spec_rev_002` |
| `run_id` | Spec | Sometimes | `run_01j1bxgvk3tf4qs6mv9zpxwe8d` |
| `requirement_id` | Spec | Yes | `R-AUTH-004` |
| `task_id` | Run | Sometimes | `T3` |
| `handoff_id` | Run | Rarely | `ho_9bc2` |
| `evidence_id` | Run | Rarely | `ev_12a4` |
| `finding_id` | Run | Rarely | `fd_77e1` |
| `decision_id` | Run or exported spec snapshot | Sometimes | `dec_20260630_001` |

## Lifecycle Language

Recommended spec lifecycle:

```text
optional brainstorm -> plan (draft) -> approve spec card -> implement (run) -> accept -> archive
```

Recommended run lifecycle:

```text
created -> implementing -> verifying -> verified -> submitted -> landed

escalated when autonomous progress needs a human decision; cancelled when a human stops the run
```

Recommended task lifecycle:

```text
queued -> building -> reviewable -> in_review -> needs_repair (repair rounds) -> integrated
```

Use "cancel" for a human-stopped run.
Use "blocked" when a requirement cannot be verified without external input or an
environment change.
Use "escalated" when autonomous progress cannot continue without a human
decision.
Use "waive" only for requirements, not whole specs or runs.

Avoid saying a spec "passes" unless every requirement is resolved as passed,
review-passed, or waived. Prefer "complete with residual risk" when waived or
review-passed requirements remain important to inspect.

## Current Recommendation

Define Speccy around specs, runs, and optional initiatives.

The CLI should make specs feel lightweight (the human command list lives in
"CLI/Admin Flow" in `DESIGN.md`), while the controller keeps rigorous spec/run
state:

```yaml
spec_ref: SPEC-20260630-A7F4
spec_id: spec_01j1bxgvk3e6q8r2n5tcvh7pyd
title: Passwordless login
active_spec_revision: spec_rev_002
run_id: run_01j1bxgvk3tf4qs6mv9zpxwe8d  # present after /speccy-implement creates a run
```

Only the spec reference is routinely user-addressable. The internal `spec_id` is
for local storage and controller bookkeeping. `run_id` is operational metadata
and should appear only when precision, debugging, export, or explicit `--run`
selection requires it.

Do not promote "mission" to the canonical product noun. It creates exactly the
kind of process-software feeling Speccy is trying to avoid, and it collides with
the better term `initiative` for strategic grouping.
