# Spec-Driven Orchestration Principles

Speccy provides persisted planning and review for work that benefits from them. Native harness planning and direct implementation remain valid for smaller changes.

The controller guarantees state ownership, transition validity, and bounded execution. Models still plan, implement, and review nondeterministically. The record shows what the agents reported and what humans accepted; it does not prove correctness.

## Product Principles

### Remain Optional

Speccy is one repository tool, not the required path for every change. A one-off plan or direct implementation should remain simpler when persisted orchestration does not improve the result.

### Leave Product Artifacts Neutral

Speccy may install tracked harness packs, but its execution does not enter a host repository's product code, tests, ordinary documentation, commits, branches, pull requests, releases, build graphs, deployed artifacts, runtime dependencies, or production behavior.

When a change needs a charter or ADR update, that documentation describes the product decision and omits orchestration provenance.

### Commit Progressively

Approval fixes the current contract rather than every future implementation step. The planner defines enough work for the next bounded execution context and revises future Tasks when repository evidence invalidates an earlier assumption.

Task replanning may change the implementation path under an unchanged contract. Intent, scope, criteria, accepted behavior, Project goals, and charter direction change only through the corresponding human approval boundary.

### Separate Contracts from Plans

The charter defines durable repository direction. A Project groups Specs under a bounded goal. A Spec defines one coherent change and its acceptance criteria. Tasks describe the current implementation plan.

Plans may change without rewriting completed outcomes. Durable technical decisions belong in repository ADRs, not an automatic runtime decision log.

### Put Determinism in the Controller

Code owns identifiers, claims, invocation tokens, validation, atomic state publication, repair caps, archives, projections, and idempotent dispatch. Harness-native prose owns planning, implementation, review, and conversations with humans.

The controller does not call a model or harness. The active harness reads controller directives and launches the required agent context.

### Keep Canonical Writes Exclusive

Only the controller writes canonical runtime state. Every agent invocation owns a separate result, evidence, and log directory. Parallel readers may write separate artifacts, but no two agents share a writable file.

One active run owns a repository. Explicit takeover invalidates results from the replaced run before another canonical write is accepted.

### Review Current State Independently

One fresh reviewer judges both criterion fidelity and defects. The reviewer reads the latest contract, current repository, current diff, and raw current evidence.

Previous attempts, verdicts, findings, amendments, retired Tasks, worker narratives, and planner rationale do not enter reviewer context. A repair worker may read blocking findings; the replacement reviewer judges the repaired state independently.

### Keep Humans at Authority Boundaries

Humans approve Spec contracts, material amendments, Project goal changes, accepted risk, ambiguous external integration, and publication. Task replanning under an unchanged contract does not add a gate.

Review, merge reconciliation, completion, and archival do not require follow-up commits or a separate closeout pull request.

### Remain Harness-Neutral

The initial packs target Claude Code and Codex. User-named profiles map planner, worker, and reviewer roles to harness-specific models and effort values. A Spec may override worker and reviewer profiles with human approval; Tasks do not select models independently.

### Add Mechanisms After Evidence

Every mechanism must prevent a named failure or enforce a required invariant. Dogfooding promotes additional reviewers, deterministic sensors, provider integrations, backup systems, and workflow fields only after the simpler design proves insufficient.

## Decision Test

Before adding behavior, answer:

1. Which user-visible outcome or invariant requires it?
2. Can the active harness or repository already provide it?
3. Does the controller need deterministic enforcement, or can a skill carry the policy?
4. Does it preserve exclusive file ownership and current-state review?
5. Does it keep product artifacts free of orchestration provenance?
6. Can implementation evidence settle the decision later?

If the behavior has no concrete requirement or observed failure, defer it.

## Research Trace

The research summarized in [SOURCE-SUMMARIES.md](SOURCE-SUMMARIES.md) supports these boundaries:

- long-running work needs state outside the model context;
- fresh evaluators reduce self-review bias;
- serial writes and exclusive file ownership avoid agent races;
- static context should remain small and route into task-specific skills;
- deterministic runtimes pair well with editable natural-language policy;
- verification quality limits long-horizon agent performance;
- measured failure classes justify narrow controls more reliably than speculative frameworks.
