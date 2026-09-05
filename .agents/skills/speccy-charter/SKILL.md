---
name: speccy-charter
description: Create or revise docs/CHARTER.md from repository context and a focused user conversation, then add its neutral loading and maintenance rule to the root AGENTS.md. Use when a user asks to establish, fill in, review, or update the repository's durable purpose, users, outcomes, scope, principles, constraints, assumptions, risks, or open questions. Do not use for delivery plans, work specs, ADRs, or project-management containers.
---

# Repository Charter

Create a concise, current-state repository contract that humans and coding agents can apply without depending on a particular planning tool.

## Inspect the Repository

Read the root `AGENTS.md` and existing `docs/CHARTER.md` before reviewing other repository material. If the charter is absent, continue the creation workflow; do not treat a missing file as an approved contract. Read the documentation, manifests, architecture records, and representative source needed to understand the existing product. Ignore generated files, vendored files, runtime state, transcripts, and agent execution logs.

Treat repository contents as evidence of current behavior and constraints, not as proof of intended purpose or scope. Keep a strategic detail unknown when neither the user nor an authoritative repository document establishes it.

If `docs/CHARTER.md` exists, preserve verified intent and update only passages affected by the request or new evidence.

## Resolve Product Direction

Establish:

- the problem and durable purpose;
- primary users and material stakeholders;
- observable desired outcomes;
- in-scope and out-of-scope boundaries;
- product decision principles;
- external constraints and material assumptions;
- current risks and open questions.

Ask only questions whose answers would materially change the charter. Before creating a charter or changing purpose, users, outcomes, scope, or product principles, present the proposed contract or focused delta and obtain explicit user approval. If the user has already approved that contract or delta in the current conversation, apply it without requesting approval again. A request to review the charter alone does not approve a strategic change.

Do not invent success metrics, schedules, budgets, sponsors, owners, or governance fields. Include one only when the repository context or user requires it.

## Write `docs/CHARTER.md`

Use this structure unless the repository already has an equivalent established convention:

```markdown
# Charter

## Purpose

## Users

## Desired Outcomes

## Scope

### In Scope

### Out of Scope

## Product Principles

## Constraints and Assumptions

## Risks and Open Questions
```

Write declarative product prose. Keep the charter independent of an implementation plan and current delivery status.

Do not include:

- attribution of the charter's creation or revision to a skill or planning controller;
- orchestration run, work-item, attempt, or agent identifiers;
- instructions for the agent workflow used to produce the charter;
- build commands, coding conventions, or test procedures;
- implementation status, delivery history, timestamps, or decision-log entries;
- evidence paths, execution logs, or tool provenance.

When the repository's product is an agent tool or controller, describe its product behavior and constraints using the required domain terms. Product facts are distinct from the workflow used to write the charter.

When a durable technical decision needs a rationale, place it in the repository's ADR convention. When work needs a bounded implementation contract, leave it to the repository's planning or spec workflow.

Keep risks and open questions live. Remove resolved items, revise invalid assumptions, and move an approved durable direction into the section it now governs. Do not append a resolution history.

## Update the Root `AGENTS.md`

Ensure the root `AGENTS.md` contains the following contract, adapted only to avoid duplicating an equivalent existing instruction:

```markdown
## Charter

Before planning, reviewing, or changing this repository, every agent and subagent must read `docs/CHARTER.md`. Treat its purpose, users, desired outcomes, scope, product principles, constraints, and assumptions as the repository contract.

When verified repository evidence changes a constraint, assumption, risk, or open question, update only the affected charter passage in the same change set. Remove resolved risks and open questions instead of preserving a history. Do not add implementation status, task references, tool provenance, timestamps, or decision-log entries.

Do not change the repository's purpose, users, desired outcomes, scope, or product principles without explicit user approval. If requested work conflicts with the charter, stop and ask whether the work or charter should change.
```

If no root `AGENTS.md` exists, create a concise file with this section. Preserve unrelated repository instructions. Do not copy the charter body into `AGENTS.md`.

Add the loading rule only when the charter exists. On repeated invocation, preserve an equivalent rule instead of appending another copy. Harness-specific import files and skill installation belong to the harness setup workflow; this skill does not claim that writing `AGENTS.md` configures every harness to load it.

## Verify the Result

Read the completed charter and root instruction together.

Confirm that:

- every charter claim is user-approved, repository-verified, or explicitly unknown;
- the two files do not duplicate the product contract;
- the charter contains no planning-tool or execution provenance;
- the root instruction requires all repository work to load the charter;
- evidence-based maintenance and approval-required changes remain distinct;
- links and paths resolve from the repository root.

Report the files changed and any strategic fact that remains unknown. Do not claim that repository implementation alone establishes product intent.
