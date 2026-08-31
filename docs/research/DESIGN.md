# Design: Spec-Driven Run Controller

Status: authoritative
Date: 2026-08-31

This document defines MVP behavior and vocabulary. Exact serialization, CLI verbs, and forge-adapter mechanics remain implementation-time decisions where the product contract does not require one form.

Working name: `speccy`.

## Product Thesis

Speccy is a small state controller for coding-agent harnesses. It turns an engineering request into an approved Spec, dispatches bounded Tasks in fresh contexts, records independent review, and stops when the run needs a human decision.

The harness calls Speccy. Speccy does not call a model or coding agent. The repository's charter defines durable direction; Speccy records temporary planning and execution state.

The promise:

> Approve the current contract, let the harness execute bounded work, and receive a reviewable result whose decisions and uncertainties remain explicit.

## Product Principles

1. **Optional workflow.** Native planning and one-off harness work remain valid. Speccy is reserved for work that benefits from persisted planning, fresh execution contexts, or cumulative review.
2. **Invisible orchestration.** Changes produced in a host repository do not expose Speccy identifiers, lifecycle vocabulary, or tool provenance through product code, tests, ordinary documentation, commits, branches, pull requests, or releases.
3. **Progressive commitment.** Approval establishes the current contract, not an immutable forecast. Planning proceeds to the next useful boundary and revises future work when implementation produces new evidence.
4. **Deterministic state, model judgment.** Code owns identity, ownership, transitions, and validation. Harness-native prose owns planning, implementation, and review.
5. **Single canonical writer.** Only the controller changes canonical state. Each agent writes results, evidence, and logs inside an exclusive invocation directory.
6. **Current-state review.** Fresh reviewers judge the latest contract and implementation without prior attempts, verdicts, amendment history, or worker narratives.
7. **Human authority at contract boundaries.** Humans approve Spec contracts, material amendments, Project goal changes, accepted risk, and external publication.
8. **Harness-neutral configuration.** User-named profiles map roles to harness-specific models and effort values. Product policy does not hardcode vendor model names.
9. **Minimum mechanism.** A feature enters the controller after a concrete failure or required invariant justifies it.

## Research Basis

The research corpus supports the design choices without requiring Speccy to reproduce any surveyed tool.

| Design choice | Research thread |
| --- | --- |
| Persist state outside model context | Long-running agent systems use files, task stores, or runtime state because context windows are not reliable memory. |
| Separate implementation from review | Fresh evaluators reduce self-review bias and make verification easier than generation. |
| Keep canonical writes serial | The sources favor serial writes, exclusive file ownership, and parallel read-only work. |
| Keep static instructions small | Harness guidance works best as a map with skills and task packets loaded only when needed. |
| Use code for state and prose for judgment | Deterministic runtimes enforce transitions while editable policies adapt to changing models and repositories. |
| Defer unproven machinery | The surveyed harnesses improve through measured failures and narrow sensors rather than speculative control layers. |

The supporting studies, product documentation, and case reports are summarized in [SOURCE-SUMMARIES.md](SOURCE-SUMMARIES.md), especially its cross-cutting threads.

## Non-Goals

- Replacing coding agents or native harness planning.
- Shipping an IDE, web application, general issue tracker, roadmap system, or team resource planner.
- Calling an LLM, agent, or harness from the controller.
- Proving correctness or executing a controller-owned deterministic evidence suite.
- Supporting concurrent canonical writers or parallel implementation Tasks in one repository.
- Requiring Projects, Milestones, pull requests, or committed runtime state.
- Prescribing dates, estimates, cycles, owners, health scores, Task dependencies, or within-Spec phases.
- Mirroring the repository charter or maintaining an automatic decision log in runtime state.
- Finalizing a generic forge-adapter protocol before the first implementation establishes its requirements.

## Repository Contract

### Charter

`docs/CHARTER.md` is the repository-level product contract. It applies to Speccy runs, native plan mode, and one-off work. The charter describes:

- purpose and users;
- desired outcomes;
- in-scope and out-of-scope boundaries;
- product principles;
- constraints and assumptions;
- risks and open questions.

The root `AGENTS.md` requires every agent and subagent to read the charter before planning, reviewing, or changing the repository.

The charter is a current-state document. When verified evidence changes a constraint, assumption, risk, or open question, the agent updates only the affected passage in the same change set. Purpose, users, outcomes, scope, and product principles require explicit human approval. The charter does not contain implementation status, task references, orchestration provenance, timestamps, or a decision history.

The `speccy-charter` skill creates or updates the charter and its root `AGENTS.md` instruction together. Generated prose does not mention Speccy. If initialization finds no charter, it directs the user to the skill and does not invent product direction.

### Durable Technical Decisions

When implementation establishes a technical decision that future maintainers need, an ordinary repository ADR records it. A Spec amendment records a change to the active contract and Task graph; it does not replace an ADR. Charter and ADR updates are planned work completed before final Spec review, never closeout side effects.

## Work Model

### Hierarchy

```text
Repository
├── docs/CHARTER.md
├── standalone Spec
│   └── Tasks
└── optional Project
    ├── Milestones
    └── Specs
        ├── Spec ── optional milestone reference
        │   └── Tasks
        └── Spec ── no milestone
            └── Tasks
```

### Project

A Project is an optional, bounded outcome comparable to a Jira Epic or Linear Project. It groups independently shippable Specs under one goal.

A Project contains a title, goal, optional Milestones, and Spec membership. The known Milestones and Specs may evolve as work produces evidence. Changing the goal requires human approval; adding, retiring, or reordering work beneath the unchanged goal does not create another Project gate.

Project progress and completion derive from its Specs. A Project has no implementation loop, reviewer, charter, decision log, required date, lead, health field, or dependency graph in the MVP.

### Milestone

A Milestone is an optional, Project-scoped delivery stage. A Spec inside a Project may reference one Milestone or remain unassigned. Standalone Specs do not use Milestones.

Milestone progress derives from assigned Specs. Milestones have no stored execution status, criteria, reviewer, or Task list. Work that needs separately delivered stages splits into multiple Specs rather than adding Milestones inside one Spec.

### Spec

A Spec is the approved contract and independent merge boundary for one coherent change. It contains:

- an intent traced to `docs/CHARTER.md` and, when present, its Project goal;
- in-scope and out-of-scope behavior;
- checkable acceptance criteria;
- material risk and unknowns;
- optional Project and Milestone references;
- flat Tasks;
- approved amendments;
- current integration facts.

The Spec contract is independent of its implementation plan. The contract may change through an approved amendment, while future Tasks may change without another gate when they continue to satisfy the approved intent, scope, and criteria.

### Task

A Task is the unit one worker implements and one fresh reviewer judges. Tasks are leaves. A ready Task defines one coherent diff, checkable criteria, expected paths or components, required reading, and a finishable plan.

When a Task still contains an unresolved product or architecture decision, planning continues before dispatch. When implementation reveals an unknown that cannot be resolved within the contract, the worker reports the discovery and the run enters replanning or amendment instead of forcing the obsolete plan.

### Identifiers

The controller assigns monotonic identifiers and never assigns a retired identifier to different work. Projects, Specs, Tasks, Milestones, criteria, amendments, invocations, and attempts have distinct identities. Task identifiers are scoped to a Spec; Milestone identifiers are scoped to a Project. Invocation tokens also identify the active run generation and attempt.

Exact serialized patterns remain an implementation decision, but every user-visible identifier must remain stable for the lifetime of its record.

## Lifecycle

### Stored Task State

```text
planned    defined but not dispatched
working    owned by a worker invocation
reviewing  worker result accepted; fresh review pending
verified   criteria pass and no blocking finding remains
failed     current review blocks completion; repair available
halted     repair cap reached or the worker cannot proceed
accepted   human accepts the current result with recorded risk
dropped    human removes the Task from current scope
```

Task transitions:

```text
planned   -> working
working   -> reviewing | halted
reviewing -> verified | failed
failed    -> working | halted
halted    -> working | dropped
```

A contract-preserving replan may retire an unstarted Task and add another. A contract amendment may also retire working or completed Tasks under the archive rules below. Retired Tasks leave the active graph and do not gain a reusable status.

### Stored Spec State

```text
draft      contract under discussion
approved   human approved the current contract
working    at least one current Task is active or pending
reviewing  current Tasks settled; cumulative review pending
verified   cumulative review passes the latest approved contract
failed     cumulative review blocks completion; corrective work pending
halted     repair cap or unresolved decision requires a human
dropped    human abandons the Spec
```

Spec transitions:

```text
draft     -> approved | dropped
approved  -> working
working   -> reviewing | halted
reviewing -> verified | failed
failed    -> working | halted
halted    -> working | accepted | dropped
```

An accepted Task records human-owned risk. A dropped Task records an explicit scope decision. `verified`, `accepted`, and `dropped` Tasks count as settled only when cumulative Spec review passes the resulting implementation against the current contract.

### Derived Project and Milestone State

Project and Milestone progress derive from the current Specs. Neither record duplicates child statuses. A Project is complete when all current Specs are done; a Milestone is complete when all Specs assigned to it are done. Empty containers remain planned rather than complete.

### Derived Integration State

```text
verified + no open pull request                    -> verified
verified + open pull request                       -> review
verified + merged pull request + settled Task tree -> done
```

`review` and `done` are projections, not independent stored statuses. Stable pull-request and merge facts are stored once discovered. A pull request closed without merging returns the projection to `verified` and may be replaced. A merge completes the Spec only when it targets the recorded base branch.

When the pull-request head changes after cumulative review, the old review no longer establishes completion. If reconciliation finds the change before merge, the Spec returns to cumulative review. If reconciliation first discovers the changed head after merge, the Spec halts as merged but unverified.

## Deferred Planning and Amendments

### Task-Graph Revision

The planner may add, replace, retire, or reorder future Tasks while the approved Spec intent, scope, and criteria remain unchanged. The controller records the active graph; agents do not preserve obsolete Tasks in current packets.

### Contract Amendment

When evidence changes intent, scope, criteria, or accepted behavior, the planner proposes a focused amendment. The human sees:

- the changed contract fields;
- the discovery that requires the change;
- Tasks to add or retire;
- the effect on completed work.

Approval applies the contract and Task changes together. A failed implementation cannot weaken its criteria silently; any criterion change remains visible in the amendment gate.

The current Spec stores the latest contract and a compact amendment record. Full prior Spec snapshots do not enter normal packets.

### Retired Task Archive

Before the controller removes a Task from the active graph, it writes the complete Task record to the local archive. If archival fails, the amendment or replan changes nothing.

After archival, the controller:

- removes the Task from the active graph;
- records its identifier in the amendment or revision;
- invalidates outstanding invocation tokens;
- preserves the identifier against reassignment.

When later planning needs the retired work, the planner reads the archive and creates a new Task with a new identifier and a reference to the retired Task. It does not revive the old invocation identity.

Archives protect against logical deletion during replanning. When `.speccy/` is ignored, they do not protect against directory deletion or disk loss. `status` or `doctor` warns when runtime state has no configured external backup.

### Completed Work Across Amendments

Completed Task outcomes remain historical facts. An amendment adds Tasks for any required change and does not rewrite prior verdicts. Final Spec review judges the current repository against the latest approved contract.

## Storage and Concurrency

### Logical Layout

```text
.speccy/                         ignored by default
  config.yaml                    role profiles, caps, integration selection
  state.yaml                     active run claim and repository counters
  projects/                      optional Project records
  specs/                         current Spec contracts and Task graphs
  invocations/<invocation-id>/
    input.json                   immutable scoped packet
    result.json                  invocation result
    evidence/                    raw current evidence
    logs/                        invocation logs
  archive/<spec-id>/tasks/       complete retired Task records
  render/                        disposable human-readable projections
```

The paths are the MVP ownership model, not a commitment to every serialized field.

### Canonical State

Only the controller writes `state.yaml`, Project records, Spec records, amendment records, integration facts, and archives. Every canonical update validates the complete affected records and publishes atomically. On validation or archival failure, the prior state remains readable.

Agents receive scoped packets rather than canonical YAML. No agent may rewrite a Spec file, Project file, shared journal, or shared evidence file.

### Invocation Artifacts

Every agent invocation receives an exclusive directory and token. The controller writes `input.json` before dispatch. The agent writes only `result.json`, `evidence/`, and `logs/` in that directory. Parallel read-only agents may write to separate invocation directories without sharing a file.

`/speccy-next` submits the completed invocation to the controller. The controller accepts the result once, checks its run generation, target, attempt, and expected action, and records its artifact location in the canonical manifest. A stale, duplicate, retired-Task, or replaced-run result is rejected.

### Active Run Claim

One run may own a repository at a time. Run creation claims ownership atomically. A later session may resume the claim or perform an explicit takeover. Takeover increments the run generation and invalidates every outstanding token from the previous owner.

The single active run limits implementation and canonical writes. An invocation may fan out read-only research inside its own context when file ownership remains exclusive.

## Git and Product Footprint

`speccy init` ignores `.speccy/` by default. Teams may choose to track runtime state, but Speccy does not require commits for operation, review, reconciliation, completion, or archival.

Rendered harness packs may be committed. Changes produced in a host repository remain neutral:

- product source and tests do not contain Speccy identifiers;
- ordinary product documentation does not describe Speccy execution;
- branch names use product terms rather than run or Task identifiers;
- commit messages describe product changes;
- pull-request titles and bodies describe the change and its evidence without orchestration provenance;
- release artifacts and runtime dependencies do not include Speccy.

Workers commit coherent Task changes before review. The controller does not create commits, squash history, or append closeout documentation. When a charter or ADR update belongs to the change, a worker completes it before cumulative Spec review. No post-merge accept, archive, or documentation commit is required.

## Harness Workflow

### Public Skills

- **`speccy-charter`** creates or revises the neutral repository charter and its `AGENTS.md` instruction.
- **`/speccy-plan <intent>`** decides whether persisted orchestration is useful, creates or selects an optional Project, drafts the Spec contract, and records human approval.
- **`/speccy-next [spec-id]`** executes exactly one controller directive in a fresh context and returns a short result.
- **`/speccy-run [spec-id]`** repeatedly spawns `/speccy-next`, retains one summary per directive, and handles human gates in the outer session.

`/speccy-run` does not implement lifecycle policy from memory. Each iteration asks the controller for the next directive.

### Lifecycle Agents

| Agent | Responsibility | Writes |
| --- | --- | --- |
| `planner` | Drafts Tasks, replans the active graph, and proposes amendments | Its exclusive invocation directory |
| `worker` | Implements one Task, runs repository-prescribed checks, and commits the coherent result | Product files plus its exclusive invocation directory |
| `reviewer` | Judges criteria and defects from current state in a fresh, history-blind context | Its exclusive invocation directory |

Agent names form a closed MVP set. The controller never launches these agents; the active harness does.

### Directive Contract

`next` returns one directive with the target, invocation identity, scoped input path, expected result path, role, repair round, and reason. Until the controller accepts the matching result or a human changes state, repeated calls return the same directive.

The directive vocabulary covers:

- planning or replanning Tasks;
- implementing a Task;
- reviewing a Task;
- reviewing the cumulative Spec;
- proposing a contract amendment;
- resolving a halt;
- offering external publication;
- waiting for or reconciling review;
- reporting completion.

Exact action strings and controller subcommands remain implementation decisions. The behavior must preserve idempotence, exclusive ownership, and explicit human gates.

## Task Execution

### Definition of Ready

A Task is ready when a fresh worker can complete it from the current packet and repository:

1. The Task owns one coherent change.
2. Its criteria are checkable without reading the plan as requirements.
3. Required files, constraints, and known decisions are present.
4. No unresolved contract or architecture choice remains.
5. The expected work fits one worker context and one reviewable commit.

The planner applies this definition. A worker that discovers a false premise reports it and stops; the repair and amendment paths are the backstop. The MVP does not add a separate readiness agent or token-estimation subsystem.

### Work Result

The worker receives the current Task packet, implements the Task, runs checks required by repository instructions, and commits a coherent result. Its result distinguishes:

- what changed;
- raw commands and outcomes;
- skipped checks;
- evidence paths;
- discoveries that require replanning or amendment;
- a punt when coherent completion is not possible.

The controller verifies repository cleanliness and the expected commit range before moving the Task to review. Agent-reported evidence remains model-produced evidence; Speccy does not claim that the controller executed or proved it.

### Repair and Halt

When review blocks a Task, the repair worker receives the current contract and blocking findings. A configurable repair cap bounds retries. The replacement reviewer receives current state without previous findings.

At the cap, or when a worker punts, the run halts. The human may retry with guidance, accept the current result with risk, drop the Task, or approve a proposed split or amendment. The controller records the decision before work resumes.

## Review Contract

One fresh reviewer combines criterion fidelity and defect detection. The reviewer returns a result for every criterion and lists blocking or advisory defects.

The reviewer receives:

- the latest approved Task or Spec contract;
- the current repository;
- the current Task or cumulative Spec diff;
- raw current command evidence.

The reviewer does not receive:

- prior Spec revisions or amendment records;
- retired Tasks or archives;
- previous attempts, verdicts, or findings;
- worker narratives, repair instructions, or journals;
- planner rationale or implementation plans;
- child verdict summaries during cumulative Spec review.

Task review judges one Task result. Cumulative Spec review judges the complete current implementation against the latest Spec contract and charter. Projects and Milestones receive no review.

The reviewer passes the target when every criterion passes and no blocking defect remains. Advisory defects remain visible but do not block. Because the reviewer is history-blind, a repair cannot rely on a prior reviewer remembering the original finding.

## Model and Effort Routing

Configuration defines user-named model and effort profiles and global defaults for `planner`, `worker`, and `reviewer`. Each profile maps to harness-specific values.

During Spec planning, the planner may propose worker and reviewer profile overrides for the work at hand. The human approves those overrides with the Spec contract. The planner starts from its global profile. Task-level overrides remain outside the MVP.

Packs contain resolved harness fields but do not hardcode product-level vendor model names. Configuration validation rejects unresolved profiles and unknown lifecycle roles.

## Pull-Request Handoff and Completion

After cumulative review verifies a Spec, the human may leave the work local or opt into neutral pull-request publication. GitHub is the default forge; users may configure another forge. Provider configuration must not let tracked repository data silently gain arbitrary command-execution authority.

Publication and reconciliation are idempotent. When exactly one existing pull request matches the recorded head and base, Speccy may adopt it rather than require a manual link. Ambiguity requires human direction.

`status`, `next`, and `/speccy-run` reconcile unresolved integration state. Read-only commands that do not affect lifecycle remain usable offline. If the provider is unavailable, reconciliation preserves the last verified facts and reports that review state may be stale.

The first forge implementation determines the narrow command and data contract needed for discovery, publication, and inspection. The design does not prescribe an adapter executable, template language, or provider API before that work begins.

## Human Gates

Speccy stops for a human when authority or external side effects require one:

- approving a new Spec contract;
- changing a Project goal;
- approving a material Spec amendment;
- resolving a halt or accepting risk;
- publishing work externally;
- resolving ambiguous pull-request adoption or merged-but-unverified state.

Creating and maintaining `docs/CHARTER.md` is a repository workflow governed by `AGENTS.md`, not a duplicated runtime gate. Task replanning under an unchanged contract does not require human approval.

## Controller Boundary

The human-facing CLI needs capabilities to initialize packs, inspect status and scoped records, validate local state, diagnose configuration, request the next directive, and refresh integration facts.

The machine-facing controller needs capabilities to create and revise Projects, Milestones, Specs, Tasks, amendments, invocations, reviews, halts, archives, and integration facts. Every write accepts structured file input, validates the complete affected records, returns a stable machine-readable success or error envelope, and performs no partial canonical update.

Exact command names and payload schemas should be fixed alongside their implementation and tests. The MVP design does not benefit from specifying unused commands in advance.

## Verification Stance

Speccy records model judgment and raw agent-produced evidence. It does not prove that criteria are satisfied, defects are absent, or reported commands ran as described.

The MVP bounds this risk through:

- written criteria before implementation;
- one worker per Task;
- a fresh reviewer with no trajectory context;
- criterion results and defect findings in one review artifact;
- cumulative Spec review;
- bounded repair and explicit human acceptance;
- clear separation between raw evidence, reviewer judgment, and human decisions.

Dogfooding must identify a repeated failure before Speccy adds a deterministic sensor or another reviewer. Any later sensor names the failure it detects and remains narrower than a general command-execution framework.

## Deferred Capabilities

- Additional reviewer personas or multiple reviewer seats.
- Controller-owned deterministic checks, provenance scanning, or evidence execution.
- Task dependencies, within-Spec phases, estimates, cycles, dates, leads, health, and Project dependencies.
- Concurrent implementation workers and worktree isolation.
- Automatic external backup or cross-machine synchronization of ignored state.
- Committed runtime state as a default.
- Timeline, dashboard, Linear, or Jira synchronization.
- Live rendering or a web server.
- Cost and token accounting.
- Self-modifying skills.
- Forge-specific integrations beyond demonstrated provider needs.

## Implementation-Time Decisions

The following choices remain deliberately open until implementation provides evidence:

- exact YAML schemas and normalized field names;
- exact CLI verbs and action strings;
- the first forge integration's command and response contract;
- evidence file conventions beyond exclusive directory ownership;
- archive export after local-only warnings prove insufficient.

Changing one of these choices does not require a product-design amendment unless it changes a user-visible contract or an invariant above.
