# Charter

## Purpose

This repository builds a small run controller for coding-agent harnesses. The controller persists approved work contracts, dispatches bounded implementation and review contexts, and records enough state for a human to resume or inspect a run without continuously steering it.

Long-running agent work cannot rely on conversation context alone to preserve plans, progress, and review state.

## Users

The primary users are software developers who already work in coding-agent harnesses and want persisted planning and independent review for changes that exceed a simple one-session workflow.

Repository maintainers are material stakeholders because they review the resulting code, define local engineering rules, and decide which workflow files may be tracked.

## Desired Outcomes

- A developer can approve a bounded change contract before implementation begins.
- Fresh worker contexts can complete Tasks from scoped repository context.
- Fresh reviewers can judge the current implementation without prior-attempt bias.
- A stopped run can resume from filesystem state without reconstructing progress from chat history.
- Parallel agent activity does not create shared-file races or overwrite canonical state.
- Implementation discoveries can revise future work without silently changing approved product behavior.
- Changes produced in a host repository read like ordinary repository work and do not expose the planning controller.

No adoption, completion-time, or review-quality target has been validated yet. Dogfooding must establish useful measures before the project treats one as a success threshold.

## Scope

### In Scope

- A deterministic local controller for identities, state transitions, ownership, validation, and projections.
- Harness-native skills and agent roles for planning, implementation, and review.
- A repository charter, optional bounded Projects, Project-scoped Milestones, independently shippable Specs, and flat Tasks.
- Exclusive invocation directories for agent results, evidence, and logs.
- Controlled Spec amendments and recoverable retirement of obsolete Tasks.
- Independent Task review and cumulative Spec review.
- Optional pull-request publication and eventual merge reconciliation.
- Initial support for Claude Code and Codex without coupling controller behavior to either harness.

### Out of Scope

- Replacing native harness planning or requiring the controller for every change.
- Calling models or coding-agent harnesses from the controller.
- An IDE, web application, general issue tracker, roadmap suite, or resource-planning system.
- Proof of correctness or a general command-execution and evidence-verification platform.
- Concurrent implementation writers in one repository.
- Required dates, estimates, cycles, owners, health scores, or Task dependency graphs.
- Automatic publication of planning state or execution provenance in product artifacts.

## Product Principles

### Optional Workflow

Persisted orchestration must earn its setup cost. Small changes remain compatible with native plan mode or direct implementation.

### Invisible Execution

Changes produced in a host repository do not expose controller identifiers or execution history through product source, tests, documentation, commits, branches, pull requests, releases, build graphs, runtime dependencies, or production behavior.

### Progressive Commitment

Approval establishes the current contract. Planning remains revisable when implementation produces information that was unavailable earlier.

### Deterministic Ownership

Code controls canonical state, file ownership, transition validity, and stale-result rejection. Models control planning, implementation, and judgment within scoped contexts.

### Independent Current-State Review

Reviewers judge the latest contract and implementation without previous attempts, verdicts, or worker narratives.

### Human Authority

Humans approve product contracts, material amendments, goal changes, accepted risk, and external publication.

### Minimum Mechanism

New control layers require a concrete failure or invariant. Implementation details remain open until evidence requires a choice.

## Constraints and Assumptions

- The controller operates inside a Git repository and may inspect repository state.
- One active run owns canonical state for a repository.
- Runtime state is local and ignored by default.
- Each agent invocation has exclusive writable artifact paths.
- The active harness can launch fresh agent or skill contexts, read repository files, cancel invocations, and establish that their commands have stopped before replacement execution.
- Workers can run repository-prescribed checks and create coherent commits.
- Model review remains fallible, and recorded command evidence remains agent-produced unless a later deterministic sensor verifies it.
- Scoped packets and exclusive artifact paths depend on cooperative agents and harness permissions. They do not isolate a malicious process running with the same filesystem access.
- Teams may track installed harness packs, but runtime state and raw evidence remain untracked unless they choose otherwise.

## Risks and Open Questions

- A single reviewer may miss defects or accept criteria satisfied through a weakened implementation.
- Ignored local state can be lost without an external backup.
- The first forge integration may expose portability or authentication requirements that the current design has not settled.
- Large repositories may require stronger packet construction or artifact retention limits.
- The useful boundary between a direct harness workflow and a persisted run needs dogfooding evidence.
- Model and effort routing needs representative evaluations before defaults can be treated as reliable.
