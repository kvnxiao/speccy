# Repository instructions

## Charter

Before planning, reviewing, or changing this repository, every agent and subagent must read `docs/CHARTER.md`. Treat its purpose, users, desired outcomes, scope, product principles, constraints, and assumptions as the repository contract.

When verified repository evidence changes a constraint, assumption, risk, or open question, update only the affected charter passage in the same change set. Remove resolved risks and open questions instead of preserving a history. Do not add implementation status, task references, tool provenance, timestamps, or decision-log entries.

Do not change the repository's purpose, users, desired outcomes, scope, or product principles without explicit user approval. If requested work conflicts with the charter, stop and ask whether the work or charter should change.

## Source map

- [`docs/CHARTER.md`](docs/CHARTER.md) defines repository purpose, outcomes, scope, and product principles.
- [`docs/research/DESIGN.md`](docs/research/DESIGN.md) is authoritative for product behavior, vocabulary, state ownership, lifecycle boundaries, and deferred decisions.
- [`docs/research/PRINCIPLES.md`](docs/research/PRINCIPLES.md) defines the decision tests for new mechanisms.
- [`docs/research/SOURCE-SUMMARIES.md`](docs/research/SOURCE-SUMMARIES.md) is informational. Do not promote a source claim into product policy unless the charter, design, or principles adopts it.

Read the relevant authoritative sections before changing behavior. If implementation conflicts with a documented invariant, report the conflict instead of silently choosing one side.

## Current repository state

The repository contains research documents, repository instructions, a charter, a license, and ignore rules. It does not yet define a programming language, build manifest, formatter, linter, or test command. Do not invent project commands. When a change adds the implementation toolchain, update this section with the exact setup and verification commands in the same change.

## Product boundaries

- Keep the controller harness-neutral. Claude Code and Codex are the initial targets, but controller behavior must not depend on either harness.
- Do not let a controller command call a model, coding agent, or harness. The active harness calls the controller.
- Keep execution identifiers and provenance out of a host repository's product code, tests, ordinary documentation, commits, branches, pull requests, releases, build graphs, runtime dependencies, and production behavior.
- Store runtime state under `.speccy/` and keep it ignored by default. Generated harness packs may be tracked.
- Keep one active run and one implementation worker per repository. Parallel read-only work may write only to separate invocation directories.
- Until dogfooding identifies a concrete failure, do not add controller-owned deterministic checks, extra reviewer seats, concurrent implementation, worktree isolation, task dependencies, timeline features, automatic backups, or a generic forge-adapter framework.
- Keep native harness planning and direct implementation usable for work that does not need persisted orchestration.

## Architecture contract

- Preserve `docs/CHARTER.md` as the repository contract. Do not mirror it into runtime state.
- Keep Project optional. A Project groups independently shippable Specs under one bounded goal.
- Keep Milestones optional and Project-scoped. A Spec may reference one Milestone; Milestones do not contain Tasks or receive reviews.
- Keep Specs as approved change and merge boundaries. Keep Tasks flat beneath a Spec.
- Derive Project and Milestone progress from Specs. Derive `review` and `done` from stored integration facts rather than duplicating those statuses.
- Let only the controller write canonical state and archives. Agents receive scoped packets and write results, evidence, and logs only inside exclusive invocation directories.
- Validate complete affected records and publish canonical updates atomically. If validation or archival fails, preserve the prior state.
- Claim one active run atomically. Explicit takeover must invalidate every outstanding invocation token from the replaced run.
- Keep `next` idempotent until the matching invocation result or human decision is recorded.
- Mint monotonic identifiers and never assign a retired identifier to different work.
- Archive a complete Task record before removing it from the active graph. Restore needed work as a new Task with a new identifier.
- Preserve the closed lifecycle-agent set: `planner`, `worker`, and `reviewer`.

## Planning and amendment policy

- Treat approval as the current contract rather than a fixed implementation forecast.
- Let the planner add, replace, retire, and reorder future Tasks while Spec intent, scope, criteria, and accepted behavior remain unchanged.
- Require a focused human-approved amendment when intent, scope, criteria, or accepted behavior changes.
- Require human approval before changing a Project goal.
- Preserve completed Task outcomes across amendments. Add new Tasks for changed work and judge the cumulative result against the latest contract.
- Record durable technical decisions in repository ADRs when future maintainers need them. Do not use the charter or runtime amendment history as a general decision log.

## Review contract

- Review every Task and the cumulative Spec with one fresh reviewer that checks criterion fidelity and defects.
- Give the reviewer only the latest approved contract, current repository, current diff, and raw current evidence.
- Do not give the reviewer prior amendments, retired Tasks, archives, previous attempts, verdicts, findings, worker narratives, repair instructions, journals, planner rationale, implementation plans, or child verdict summaries.
- Give repair workers the current blocking findings, but keep replacement reviewers history-blind.
- Keep raw evidence, reviewer judgment, and human acceptance distinct in storage and user-facing output.
- Do not claim that model review proves correctness or that the controller executed agent-reported commands.

## Harness workflow

- Keep the public lifecycle skills `speccy-plan`, `speccy-next`, and `speccy-run`.
- Let `speccy-run` spawn `speccy-next` once per directive. The outer run retains short summaries and human decisions rather than implementation context.
- Keep `speccy-charter` responsible for creating or revising the neutral charter and its root instruction. Generated charter content does not mention the skill or controller.
- Resolve user-named model and effort profiles through configuration. Global role defaults apply unless the human approves Spec-level worker or reviewer overrides.
- Do not add Task-level model overrides in the MVP.
- Keep provider publication behavior neutral and configurable, with GitHub as the default. Defer exact adapter mechanics until implementation establishes the required contract.

## Change workflow

1. Read `docs/CHARTER.md` and the governing design section.
2. Define observable success before implementation.
3. Preserve unrelated user changes and keep the diff within the requested concern.
4. When implementation evidence invalidates the current Task graph, replan future Tasks. When it invalidates the approved contract, propose an amendment.
5. Update affected documentation and durable decisions in the implementation change before cumulative review.
6. When the repository declares formatter, linter, type-check, and test commands, run the applicable checks and report skipped checks.
7. Verify the accumulated change set before a commit or pull request.

## Verification requirements

When implementation begins, cover these deterministic boundaries directly:

- atomic active-run claims, resume, takeover, and stale-token rejection;
- exclusive invocation directories and rejection of duplicate or mismatched results;
- whole-record validation, archive-before-retire ordering, and no partial canonical writes;
- monotonic identifier allocation and non-reuse;
- idempotent next-directive computation and repair caps;
- Task and Spec state transitions, accepted risk, dropped scope, and amendments;
- derived Project, Milestone, review, and completion projections;
- changed-head invalidation and idempotent integration reconciliation;
- packet isolation for workers, repair workers, and history-blind reviewers;
- harness-neutral pack rendering and user-defined profile resolution.

## Documentation and generated files

- Keep this file as a repository map. Keep detailed product behavior in `docs/research/DESIGN.md`.
- Do not edit generated harness packs when a template or generator owns them. Update the source and regenerate.
- Do not commit `.speccy/` runtime state, invocation artifacts, raw evidence, logs, archives, caches, or rendered run pages by default.
- Keep comments rare. Use names and types for mechanics; reserve comments for external formats, platform constraints, unsafe invariants, or ordering hazards the code cannot express.
