# Documentation Readiness Review

The documented proposal targets a small local controller with explicit ownership, recovery, review, and completion rules. Projects, Milestones, and forge integration remain later product extensions. Grades assess whether each section is coherent, supported, bounded, and testable enough to implement; they do not assess an existing runtime.

[The charter](../CHARTER.md), [design](DESIGN.md), and [principles](PRINCIPLES.md) own their respective contracts. This review records the grading criteria and their evidence. It cannot override those documents.

## Grading Rule

An A section must have a clear purpose and authority, consistent mechanics, explicit failure or uncertainty boundaries, justified scope, and observable validation criteria. A B section has a focused unresolved contract gap. A C section contains a contradiction or an unresolved choice that forces materially different implementations. More detail alone does not improve a grade.

Documentation readiness, implemented reliability, live skill behavior, and measured product value are separate assessments. An A documentation grade does not waive implementation or harness release checks.

## Section Grades

| Area | Grade | Basis for readiness | Evidence still required after documentation |
| --- | --- | --- | --- |
| Purpose and charter | A | Users, optional use, neutral output, human authority, and long-term scope are explicit; cancellation and cooperation assumptions are stated | Useful workflow boundary and user benefit need dogfooding |
| Research basis | A | Decision-relevant claims link primary sources, distinguish experience from experiment, and state limitations; unchecked material is a reading queue | Reproduce relevant findings or run local evaluations before claiming effectiveness |
| Principles | A | Decision tests identify need, owner, minimum state, approval, retry behavior, and reconsideration evidence | Apply the tests during implementation and measure proposed policies |
| MVP scope | A | Standalone Specs and serial Tasks reach local handoff; grouping, remote integration, and parallelism have explicit later triggers | Demonstrate a complete interrupted-and-resumed local run |
| Design consistency | A | Task phases, review judgment, risk disposition, local completion, dispatch, and persistence have separate meanings and defined guards | State-transition and fault-injection tests |
| Existing charter skill | A | Routing, absent-charter creation, prior approval, neutral output, idempotent root instructions, and harness-import ownership are explicit | Live creation and maintenance scenarios on each claimed harness |
| Repository instructions | A | Root instructions load the charter and route detailed behavior to the design; implementation commands are not invented | Keep setup and check commands accurate when implementation begins |
| Overall documented proposal | A | Required mechanisms enforce named invariants; deferred features require evidence; release conditions distinguish specification from proof | Runtime reliability and product value remain ungraded |

## Design Coverage

| Design section | A-level contract and inspection case |
| --- | --- |
| Delivery scope | A local Spec can finish without a Project, Milestone, provider, dashboard, or second implementation writer |
| Product records | Approval binds an exact Spec revision; Tasks map to behavior without becoming independent scope authority |
| Task phase | `planned` through `settled` is reachable; Task settlement does not depend on cumulative review |
| Judgment and disposition | A failed requirement cannot be waived as risk; unknown evidence can be accepted without rewriting the verdict |
| Spec completion | Eligible cumulative review produces local handoff and releases ownership without asserting merge |
| Planning and amendments | Future work can change under the contract; changed behavior requires approval and preserves historical outcomes |
| Canonical storage | One transaction commits affected records and archives; external evidence publication precedes references |
| Repository boundary | Clean normal entry, explicit dirty recovery, primary worktree support, and local-state loss limits are stated |
| Dispatch and recovery | Run ownership and launch claims are separate; uncertain liveness blocks replacement; retries preserve committed effects |
| Bounds | Recovery belongs to the Spec and cannot reset through Task replacement or resume; duration requires harness cancellation |
| Review identity and context | Complete review intervals and current evidence bind judgments; packets exclude history while acknowledging cooperation limits |
| Harness surface | The outer harness selects the role before fresh context creation; one context performs each directive |
| Later extensions | Grouping and publication have adoption and acceptance conditions without a speculative adapter framework |
| Release evidence | Deterministic failures, live harness compatibility, and comparative dogfooding have separate checks |

## Contract Walkthroughs

These are document-level traces of the specified rules, not executions of a controller.

| Scenario | Required observable result |
| --- | --- |
| Task review passes | Task settles immediately; after remaining Tasks settle, cumulative review starts |
| A Task criterion is unknown and the human accepts its verification risk | Judgment remains unknown; Task settles with risk; cumulative review still assesses the full contract |
| A required behavior fails | Repair or amendment is required; risk acceptance cannot mark it delivered |
| A local Spec passes cumulative review | Current identity and evidence are checked, the handoff transaction commits, and the run closes |
| A caller loses an accepted-result response | Identical resubmission returns the original acknowledgement without repeated mutations |
| Callers read the same pending directive | Only one atomically claims launch authority; other callers cannot launch it |
| A worker submits before its process exits | Acceptance does not authorize another invocation; the harness must confirm cessation |
| A worker commits and disappears before submitting | Resume inspects Git and existing artifacts, preserves commits, and does not blindly repeat implementation |
| A takeover leaves an old worker alive | Replacement stays blocked until the harness confirms the old invocation and its commands stopped |
| Archival fails during a replan | The transaction preserves the prior active graph and complete records |
| A contract changes after review | Cumulative eligibility is invalidated; historical Task outcomes remain recorded |
| Recovery creates new Task identifiers | The Spec budget is preserved; exhausted budget requires a human decision |
| The human cancels with partial edits | Invocations stop, state is preserved, ownership is released, and delivery is not reported |

## Skill Acceptance Scenarios

The repository contains only `speccy-charter` and its UI metadata. Lifecycle skills and packs are specified, not implemented. Metadata validation establishes format compliance; scenario inspection establishes instruction coverage. Neither establishes reliable live behavior.

| Request or fixture | Expected behavior |
| --- | --- |
| Create a charter in a repository without one | Inspect evidence, leave unsupported strategic facts unknown, present the proposed contract, and obtain approval before writing |
| Apply an already approved exact charter delta | Apply that delta without requesting approval again |
| Review a charter without approving a strategic change | Report the proposed direction change; update only authorized or verified maintenance passages |
| Update a verified constraint | Change the affected passage and remove resolved questions without a decision history |
| Describe an agent-controller product | Retain necessary product vocabulary while excluding execution identifiers and generation attribution |
| Invoke the skill again with an existing root rule | Preserve unrelated instructions and avoid a duplicate loading rule |
| Request an ADR, delivery plan, or work Spec | Do not route it through charter creation |
| Use the charter from Claude Code | Pack setup installs and checks the harness import; the charter skill does not claim universal loading |

## Verification State

Contract walkthroughs and skill-scenario expectations were inspected against the documented rules. The independent contract review and documentation synchronization check found no remaining blocker. Skill metadata validation, local Markdown-link checks, and diff whitespace checks pass.

Runtime tests and live cross-harness evaluations cannot run because the controller and packs do not exist. No programming toolchain or repository formatter, linter, or test command is defined. The skill's live behavior remains unevaluated; the recorded scenario assessment is instruction inspection, not a harness execution.

## Remaining Product Questions

Dogfooding must establish whether persisted contracts reduce recovery effort enough to justify setup and review overhead. It must also assess single-reviewer misses, false positives, repeated findings, and duplicate Task/Spec review cost. These are explicit empirical questions, not unresolved lifecycle rules.

If the local workflow does not justify its cost, reduce the workflow before adding hierarchy, provider automation, extra reviewers, or routing infrastructure. A feature's absence does not lower documentation readiness when its deferral has a clear reason and adoption test.
