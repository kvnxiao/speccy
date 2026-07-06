# Speccy Spec Quality

Read this before drafting or patching a Speccy spec. The spec is the definition
of done for an autonomous run, so it must be clear enough that implementation,
verification, and review can proceed without hidden product decisions.

## Drafting Standard

- Inspect the current repo before drafting. If the code answers a question, do
  not ask the human.
- Keep requirements outcome-focused: describe externally meaningful behavior or
  durable constraints, not individual implementation steps.
- Give every requirement evidence that would actually prove it. A command needs
  a concrete command string; review/browser/api/manual evidence needs a note
  explaining what the verifier should check.
- Keep tasks implementation-focused and bounded. Every requirement must be
  covered by at least one task, but a task can cover several related
  requirements.
- Open questions are only for product, policy, credential, environment, or
  other external decisions. Include your recommended answer for each one.
- Reconcile prior context against the current code. Carry forward only what
  still appears valid; flag stale or contradicted context instead.

## Semantic Self-Review

Before presenting the spec card, reread the draft as if you were a fresh
reviewer:

- Placeholder scan: remove TBD, TODO, filler, and vague "etc." scope.
- Contradiction scan: goal, scope, requirements, tasks, and evidence must agree.
- Scope scan: split initiatives; do not hide unrelated refactors inside a task.
- Ambiguity scan: if a requirement can be read two ways, choose one and state it.
- Evidence scan: proof must cover success and important failure paths at the
  selected risk tier.
- Coverage scan: every requirement has evidence and at least one task.
- Drift scan: brainstorm handoffs and prior-context candidates are advisory
  until repo inspection confirms them.

## Examples

Weak requirement:

```yaml
statement: "Improve auth."
evidence:
  - kind: command
    command: "npm test"
```

Better requirement:

```yaml
statement: "An expired magic link is rejected and does not create a session."
scenario:
  given: "a magic link older than the configured expiry window"
  when: "the user opens the link"
  then: "the request is rejected and no authenticated session is created"
evidence:
  - kind: command
    command: "npm test -- auth/expired-link"
    note: "Covers expired token rejection and verifies no session is created."
```

Weak task:

```yaml
title: "Do login work"
requirements: ["R-AUTH-001", "R-AUTH-002", "R-AUTH-003"]
```

Better task:

```yaml
title: "Token expiry and consume-path rejection"
requirements: ["R-AUTH-003"]
constraints:
  - "Do not change OAuth behavior."
```
