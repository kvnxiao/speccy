# Principles

New mechanisms must enforce a stated invariant or address an observed failure. [The charter](../CHARTER.md) defines product principles; [the design](DESIGN.md) specifies behavior. This document supplies decision tests without duplicating either contract.

## Authority and Evidence

| Question | Authority |
| --- | --- |
| Who is the product for, and what belongs in its durable scope? | Charter and explicit human approval |
| What does the controller do? | Design and approved Spec contract |
| Does an implementation satisfy a criterion? | Current evidence and reviewer judgment, with uncertainty explicit |
| May an unresolved risk be accepted? | Human disposition within the approved contract |
| Is another mechanism justified? | A violated invariant or observed failure, assessed with these tests |
| Does a cited source establish product policy? | Only an explicit adoption in the charter or design does |

When implementation contradicts an invariant, report the conflict. When evidence contradicts an assumption, revise the affected assumption. Neither route silently changes approved product behavior.

## Decision Tests

### Need

Name the failure and its user-visible consequence. Determine whether native harness features, repository checks, or a simpler workflow already address it. Optional orchestration needs a concrete benefit over native planning, persistence, and fresh review.

A missing dashboard is not itself a failure. An inability to determine which contract produced the current commits is a recoverability failure that persisted identity can address.

### Ownership

Choose one canonical owner for each fact. The controller owns validated state and transitions; the harness owns execution; reviewers own judgments; humans own approval and risk decisions. A packet or token must not be described as stronger isolation than the harness actually enforces.

Prefer a transaction in one established store over a custom protocol for synchronizing canonical files. External evidence still needs explicit publication and integrity rules.

### Minimum State

Store facts that cannot be reconstructed safely. Derive progress and display status from those facts. Keep Task execution phase distinct from review judgment and human disposition; never duplicate a derived completion flag that can disagree with its handoff record.

Before adding a field, identify its writer, reader, invalidation event, and recovery behavior. Before adding a grouping entity, show the independently delivered work that needs it.

### Approval and Adaptation

Approval defines behavior, not an immutable task forecast. Replan unstarted work under the same contract. Use focused amendments for changed intent, scope, criteria, or accepted behavior. Preserve completed outcomes and judge the cumulative result against the latest contract.

Human acceptance of uncertain evidence does not turn it into a passing check. Known failure of required behavior requires repair or amendment.

### Review

Fresh reviewers inspect the current contract, complete review diff, repository, and raw current evidence. Repair workers receive actionable findings. Separate reviewer judgment from the human decision that follows it.

Fresh context reduces supplied prior-attempt information; it does not prove impartiality or correctness. Assess missed defects, blocking false positives, and repeated review cost. Changing review frequency or history exclusion requires an explicit policy decision, not a claim that one study settled the issue.

### Recovery and Bounds

Before automating an action, define its retry identity, commit point, interruption outcome, and stop condition. Identical retries must not duplicate effects. Rejected stale tokens do not establish that the previous worker stopped.

Bound autonomous recovery across the Spec. New Task identifiers and resumed sessions must not create unlimited retries. Use harness cancellation for duration limits and require confirmed cessation before replacement execution.

### Neutral Product Output

Keep execution identifiers and history in ignored runtime state. Product code, tests, ordinary documentation, commits, branches, pull requests, releases, and runtime behavior follow host-repository conventions without controller provenance. Repository-domain facts about agent tooling remain legitimate product content.

### Reversibility

Prefer a local, serial path before hierarchy, remote integration, parallelism, or additional reviewers. Each deferred feature needs an adoption trigger and a testable contract before implementation. Preserve the user's files during failure; recovery must not depend on automatic reset or cleanup.

## Evaluation Rule

Research can motivate a hypothesis, identify a known limitation, or establish a documented API contract. Vendor experience does not establish a general causal effect. Abstracts do not establish reproduced results. A skill taxonomy's detected-defect rate is not a runtime failure probability.

The initial evaluation compares equivalent work under equivalent model and harness settings and records recovery effort, incorrect completion, defects, review noise, and cost. A documented design can be ready to implement while its product value remains unmeasured. If the smaller workflow does not justify its overhead, reduce scope before adding control layers.
