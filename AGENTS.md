# Repository instructions

## Charter

Before planning, reviewing, or changing this repository, every agent and subagent must read `docs/CHARTER.md`. Treat its purpose, users, desired outcomes, scope, product principles, constraints, and assumptions as the repository contract.

When verified repository evidence changes a constraint, assumption, risk, or open question, update only the affected charter passage in the same change set. Remove resolved risks and open questions instead of preserving a history. Do not add implementation status, task references, tool provenance, timestamps, or decision-log entries.

Do not change the repository's purpose, users, desired outcomes, scope, or product principles without explicit user approval. If requested work conflicts with the charter, stop and ask whether the work or charter should change.

## Source map

- [CHARTER.md](docs/CHARTER.md) owns durable purpose, outcomes, scope, and product principles.
- [DESIGN.md](docs/research/DESIGN.md) owns product behavior, state, lifecycle, MVP scope, and later feature boundaries.
- [PRINCIPLES.md](docs/research/PRINCIPLES.md) supplies decision tests for proposed mechanisms.
- [SOURCE-SUMMARIES.md](docs/research/SOURCE-SUMMARIES.md) records external evidence and its limits; it does not independently establish policy.
- [REVIEW.md](docs/research/REVIEW.md) assesses documentation readiness and defines validation scenarios; grades do not establish runtime reliability.
- [speccy-charter](.agents/skills/speccy-charter/SKILL.md) creates or maintains a neutral charter and its root loading rule.

Read the governing design sections before changing behavior. Report an implementation conflict with an invariant instead of silently choosing one side.

## Current repository state

The repository contains research documents, a charter, repository instructions, the charter skill, a license, and ignore rules. It has no controller, lifecycle packs, programming language, build manifest, formatter, linter, or test command. Do not invent project commands. When implementation adds a toolchain, update this section with exact setup and verification commands.

## Product boundaries

- Keep the controller harness-neutral and called by the active harness. Controller commands never call models or agents, run product checks, create Git commits, or perform forge actions.
- Keep the local MVP to standalone Specs, flat serial Tasks, review, amendments, recovery, and local handoff. Projects, Milestones, and remote integration retain their later product definitions in the design.
- Keep one active or halted run and one implementation writer per repository. Only the controller writes canonical state and archives. Workers may edit product files; agents write runtime artifacts only in their own invocation directories.
- Keep runtime state under ignored `.speccy/`. Preserve neutral host-repository artifacts without execution identifiers or history. Installed harness packs may be tracked.
- Preserve the closed lifecycle roles `planner`, `worker`, and `reviewer`, and public skills `speccy-plan`, `speccy-next`, and `speccy-run`.
- Preserve fresh Task and cumulative Spec review. Apply the design's packet allowlist and history exclusions; never equate reviewer judgment, raw evidence, and human acceptance.
- Use contract-preserving replans for future work and human-approved amendments for changed behavior. Preserve completed outcomes.
- Keep native planning and direct implementation usable. Deferred mechanisms require the evidence described in the design before they enter scope.

## Change workflow

1. Read the charter and governing design sections.
2. Define observable success before implementation.
3. Preserve unrelated changes and keep the diff within the requested concern.
4. Update affected documentation and durable decisions with implementation changes.
5. Run applicable repository-declared checks and report skipped or unavailable checks.
6. Run `verify-changes` on the accumulated change set before finishing or committing. Leave changes uncommitted unless the user requests a commit.

The design's [verification requirements](docs/research/DESIGN.md#verification-and-release-evidence) govern implementation tests. A documentation grade does not waive them.

## Documentation and generated files

Keep detailed behavior in the design and this file as a repository map. When a template or generator owns a harness pack, edit the source and regenerate. Do not commit runtime state, evidence, logs, archives, caches, or rendered run pages by default.

Keep comments rare. Names and types describe mechanics; comments cover external formats, platform constraints, unsafe invariants, or ordering hazards that code cannot express.
