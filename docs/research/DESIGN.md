# Design

Speccy persists approved change contracts and coordinates serial implementation with fresh review. The active coding harness runs agents and commands; the controller validates and records their results.

[The charter](../CHARTER.md) owns durable product direction. [The principles document](PRINCIPLES.md) supplies decision tests. [The source summaries](SOURCE-SUMMARIES.md) distinguish external evidence from product choices. This document owns behavior; [the review](REVIEW.md) assesses readiness, not runtime reliability.

## Delivery Scope

The local MVP supports standalone Specs, flat serial Tasks, fresh Task and cumulative Spec review, approved amendments, human risk decisions, halt and resume, and local handoff. Transactional state, exclusive dispatch, bounded recovery, and scoped packets enforce those promises.

A first usable run approves a small Spec, completes and reviews its Tasks, survives an interrupted invocation, and hands off reviewed commits without reconstructing progress from chat. Implementation proves this path on one harness before translating the pack to the other. Claude Code and Codex support is claimed only after each passes the same compatibility checks.

Projects, Milestones, remote publication, merge reconciliation, parallel read-only dispatch, and dashboards are outside the local MVP. Their later boundaries appear below. Native planning and direct implementation remain available. Use Speccy when approved contracts, cross-session recovery, or recorded independent review justify its setup cost; native goals, persistent sessions, and fresh review form the comparison baseline.

## Product Records

### Charter and Decisions

`docs/CHARTER.md` remains a repository document; runtime state does not mirror its body. A missing charter requires the charter workflow before Spec approval. The workflow obtains approval for the proposed product direction instead of inferring intent from implementation. Durable technical decisions follow the repository's ADR convention when one exists.

### Spec

A Spec is an approved, independently shippable change and delivery boundary. Its contract contains intent, scope and exclusions, observable acceptance criteria with stable identifiers, and a delivery policy. The local MVP supports `local` delivery only. Drafts may change freely; approval records the exact revision and human decision.

Execution records include the Spec's Git base, ordered active Tasks, amendments, reviews, human dispositions, and eventual handoff or cancellation. Implementation plans are not acceptance criteria. Each criterion states observable behavior and how a reviewer can assess it. Known evidence limitations are explicit before approval; verification difficulty does not authorize weaker behavior.

### Task

A Task is one coherent implementation step beneath a Spec. It contains a bounded change, current criteria mapped to Spec criteria, relevant repository context, and expected checks. A ready Task fits one worker context and has no unresolved decision that would change the approved contract. A Task may contribute part of a Spec criterion; cumulative review assesses the complete criterion.

Tasks are flat and ordered. Only one Task is implemented at a time. Task identifiers increase monotonically within a Spec; Spec and invocation identifiers increase within the repository. Issued identifiers are never reassigned. Allocation becomes authoritative at transaction commit, including when its response is lost.

Completed Task outcomes remain historical facts. Later work may change the same code; cumulative review judges the resulting Spec without treating earlier Task verdicts as current proof.

## State and Completion

### Task Phase

Task phase describes execution. Review judgment and human acceptance are separate records.

| Current phase | Event and guard | Next phase |
| --- | --- | --- |
| `planned` | A worker claims the current directive | `working` |
| `working` | A complete work result is accepted and its Git identity matches | `reviewing` |
| `working` | Execution is interrupted, the worker punts, or the result is invalid | Unchanged; halt or authorized recovery |
| `reviewing` | Current review is eligible under the disposition rules | `settled` |
| `reviewing` | An authorized repair worker claims the directive | `working` |
| `planned` | A contract-preserving replan retires unstarted work | Removed after archival in the same transaction |
| `settled` | Later work changes the implementation | Unchanged; corrections use new Tasks |

A settled Task does not depend on cumulative review. Cumulative review begins after every remaining active Task settles. An approved Spec requires at least one active Task; a no-change request exits planning without starting a run.

A started Task cannot be retired as unstarted work. Before an amendment supersedes it, the harness reconciles its files and outcomes, and the controller archives its complete record and reconciliation decision. Completed outcomes are preserved across amendments.

### Judgment and Human Disposition

For each criterion, the reviewer records `pass`, `fail`, or `unknown`, with evidence or the reason evidence is missing. Findings state concrete locations and consequences and are `blocking` or `advisory`. A passing judgment requires every criterion to pass and no blocking finding.

Human acceptance never changes a reviewer judgment:

- A known failure of an approved criterion requires repair or amendment. Risk acceptance cannot silently remove required behavior.
- A human may accept an `unknown` criterion with a stated verification limitation, or a defect that does not establish failure of an approved criterion. The decision names the risk, criterion or finding, and exact review identity.
- A review is eligible when each criterion passes or has an explicitly accepted unknown, and every blocking finding is resolved in a current review or explicitly accepted within this boundary.
- Task acceptance permits that Task to settle. It does not waive cumulative review or approve a different review identity. Cumulative risk acceptance must explicitly cover the cumulative result.

Output distinguishes `passed` from `accepted with risk`. A decision that changes scope or accepted behavior uses an amendment. Reviewer judgment, raw evidence, and human disposition remain separately inspectable.

### Spec Progress and Local Handoff

Spec status is derived from facts in this precedence order:

| Status | Required facts |
| --- | --- |
| `done` | Local handoff recorded for an eligible cumulative review |
| `dropped` | Human cancellation before handoff |
| `halted` | Run halted without a terminal disposition |
| `verified` | Current cumulative review passes; handoff pending |
| `accepted` | Current cumulative review is eligible through risk acceptance; handoff pending |
| `reviewing` | Cumulative review dispatched and unresolved |
| `working` | Approved work started without the preceding facts |
| `ready` | Approved contract; work has not started |
| `draft` | Contract lacks approval |

During Task review, the Spec remains `working` and output also names the Task phase. A failed cumulative review authorizes bounded corrective work or halts; it does not create another stored Spec phase.

Local delivery approval authorizes handoff once cumulative review is eligible, the reviewed head and evidence remain current, the worktree is clean, and every invocation has stopped. One transaction records contract revision, base, head, review identity, and accepted risks, then closes the run and releases ownership. `done` means reviewed local delivery was handed off; it does not assert publication or merge.

A handoff remains historical when a later Spec changes the repository. Further changes use a new Spec. Before handoff, unexpected head, contract, or evidence changes invalidate cumulative eligibility. Human cancellation also requires confirmed cessation, preserves files and records, marks the Spec dropped, and closes the run. Cancellation never resets Git or reports delivery.

## Planning and Amendments

The planner may add, replace, retire, and reorder future unstarted Tasks while intent, scope, criteria, and accepted behavior remain unchanged. It submits complete affected records and expected revisions. The controller validates identities, mappings, ordering, and ownership; model and human review judge preservation of meaning.

A material change requires a focused amendment with the proposed contract delta, the discovery requiring it, and affected work. Existing human approval applies to the exact approved delta without another confirmation. A review request alone does not approve changed product direction.

Before applying an amendment or replan affecting current work, the harness stops affected invocations and confirms cessation. A transaction publishes the approved revision, invalidates outstanding tokens and cumulative review, archives retired records, and installs replacement work. Changed requirements receive new Tasks; cumulative review assesses the latest contract.

Retiring an obsolete implementation step does not remove approved behavior. Dropping approved behavior requires amendment; abandoning the entire Spec uses cancellation. Neither erases historical results or rewrites earlier verdicts.

## Canonical State and Artifacts

### Transaction Boundary

The MVP uses one SQLite database at `.speccy/state.db` for contracts, revisions, counters, runs, invocation receipts, judgments, human decisions, and archived Tasks. External evidence files remain outside its transaction. [SQLite documents atomic commit and its filesystem assumptions](https://sqlite.org/atomiccommit.html).

Every mutation validates complete affected records and expected revisions inside a transaction. Complete Task archival, removal from active work, replacement identifiers, and token invalidation commit together. On failure, the prior committed state remains authoritative. Restoring retired work creates a new Task with a new identifier and an archive reference.

The database records a schema version. Unsupported versions, corruption, and failed integrity checks stop mutation; the controller never initializes over existing state. A release that changes the schema must supply an explicit migration path. The first schema does not need a generic migration framework.

Use durable SQLite settings on supported local filesystems. Network shares, synchronization folders, and storage that cannot satisfy database and artifact durability are unsupported. Crash-injection checks on each supported platform are required before claiming recovery guarantees.

### Artifact Publication

```text
.speccy/
  config.json
  state.db
  invocations/<id>/
    input.json
    result.json
    evidence/
    logs/
  render/
```

Configuration is user-controlled and validated on load. Each invocation records its resolved settings; a configuration change does not silently alter a running invocation, approved profile override, or remaining recovery budget. Human-readable renders are disposable projections.

The controller creates exclusive invocation directories and input packets. Agents write results, evidence, and logs only in their own directory; implementation workers also write product files. Only the controller writes canonical records and archives. Packet immutability and directory ownership are protocol rules, not filesystem security guarantees.

Before recording a result, the controller validates its schema, identity, paths, and artifact manifest. Complete artifacts are flushed, closed, and published before the database commits their references. The platform implementation must prevent readers from observing partial published files. A crash may leave unreferenced files, which are ignored. Missing or changed referenced evidence invalidates dependent current review and halts progression.

Content hashes are checked before review packet construction and handoff. Paths are checked after resolution; absolute escapes, parent traversal, symlink or junction escapes, and references to another invocation are rejected. Configured positive packet and result size limits are enforced before ingestion. If required evidence exceeds a size limit, the controller reports a diagnostic and does not silently truncate the evidence.

Hashes identify submitted bytes; they do not prove commands ran. Reviewers may execute repository checks through the harness and submit their own evidence. The controller does not execute product checks.

### Repository Boundary

The MVP supports a non-bare Git repository with committed `HEAD` in its primary worktree on a supported local filesystem. Linked worktrees are rejected until ownership across them is implemented. Path aliases resolve to the same canonical Git directory and working root; independent clones have independent state.

Initialization validates the repository and adds `.speccy/` to ignore rules idempotently while preserving unrelated content. Runtime state stays untracked. Packs may be tracked according to repository policy. Initialization never stashes, resets, cleans, or commits user files.

Ignored state can be lost. Automatic backup, export, and backup-configuration checks are deferred. The MVP does not claim recovery from deleted state.

## Run Ownership, Dispatch, and Recovery

### Run Claim

One active or halted run owns a repository until handoff or cancellation closes it. A draft does not claim the repository. Starting an approved Spec validates a clean worktree and captures its current `HEAD` as the Spec base before product execution. The ownership transaction records that base and a run generation. Concurrent starts cannot create competing owners.

Resume inspects the outstanding directive, dispatch receipt, result, and actual Git state before authorizing work. Takeover requires a human decision. Before takeover permits a replacement invocation, the harness must stop every outstanding invocation and establish that its commands have stopped. The controller increments the generation and invalidates outstanding tokens from the replaced generation.

Tokens prevent stale canonical writes; they do not terminate processes or revoke filesystem access. When liveness is unknown, the run stays halted and does not dispatch a replacement writer. Timeout-based lock stealing is prohibited.

### Directive and Submission Protocol

`next` computes an action from canonical facts. A pending directive remains identical until its result, cancellation, or matching human decision is recorded. Its packet reference identifies invocation, generation, target revision, role, expected action, and exclusive artifact directory.

Reading a directive does not authorize launch. The harness atomically claims it before launching. Only the winning caller receives launch authority. Repeated `next` calls for a claimed invocation report a pending result or reconciliation requirement and never authorize a duplicate launch.

A claim includes a caller-generated request key. An identical retry returns its original receipt; another caller cannot claim the invocation. If the claiming harness loses local launch state, it reconciles liveness before using the receipt. A receipt cannot prove whether a process launched.

Submission must match invocation, generation, target revision, role, and action. Acceptance records the digest and acknowledgement atomically with canonical effects. An identical retry returns the recorded acknowledgement without applying effects again. A conflicting retry is rejected. Old accepted receipts remain readable; unaccepted results with stale generations or revisions are rejected.

Result acceptance and invocation cessation are separate facts. Before the next invocation can claim launch authority, the harness records that the previous invocation and its commands stopped. An accepted result with uncertain cessation requires reconciliation. This guard applies to ordinary worker-to-reviewer and Task-to-Task progression as well as recovery.

Agent result schemas cannot express approval, risk acceptance, cancellation, takeover, or publication authorization. Human decisions use a separate operation bound to the pending gate and current revision. The harness records actual human input, never inferred approval from worker text or repository content.

### Git and Work Results

Before normal worker dispatch, the controller requires a clean index and tracked worktree and no nonignored untracked files. It records the starting head. Workers run repository-prescribed checks and create coherent commits before submission. The controller verifies actual head, cleanliness, and a commit interval descended from the starting head. The Spec base stays fixed for cumulative review.

A work result identifies base and head, evidence, commands and reported outcomes, skipped checks and reasons, and blocking discoveries. Worker narrative may inform planning and repair but is excluded from reviewer packets.

Unexpected commits, a dirty submission workspace, missing results, or mismatched bases halt progression. The controller preserves files and commits and reports observed paths and identities. A missing result never establishes that implementation did not run.

After interruption, the human and harness reconcile existing changes. They may submit an existing result that is complete and valid, continue work after stopping the old writer, or request a replan or amendment. A replacement invocation records the observed starting state and remaining work without claiming to have produced existing changes. Continuing a dirty recovery workspace requires explicit human authorization for those changes; the normal clean-entry rule does not authorize their removal.

### Bounds

Each Spec has one recovery budget, defaulting to three autonomous recovery actions. This is an initial policy, not a measured optimum. Repair dispatch, retry after interruption or invalid output, and discovery-driven replan each consume one action. A replan and its first corrective worker dispatch share one action; later retries consume another. Initial planning and first dispatch of each unchanged planned Task do not consume recovery budget.

New Task identifiers, amendments, and resume do not reset the budget. At zero, further autonomous recovery halts. A human may grant a specific additional budget, amend the contract, accept eligible risk, or cancel. A status request does not authorize retries.

Every invocation has a positive duration limit from validated configuration, enforced by the harness. Expiration requires cancellation and halts the run; replacement still requires confirmed cessation. Elapsed time is neither completion nor proof of termination. A harness without supported cancellation cannot support unattended `speccy-run`.

### Interruption Outcomes

| Interruption | Required recovery |
| --- | --- |
| Before run-claim commit | Retry start; committed ownership decides the winner |
| After directive creation, before claim | Return the same directive; one caller claims it |
| After claim, before or during launch | Reconcile launch state; an uncertain receipt cannot authorize another launch |
| During edits or checks | Locate or stop the invocation, preserve changes, and obtain a recovery decision |
| After worker commit, before submission | Inspect result and Git state; recover evidence or continue explicitly without reapplying completed edits |
| During artifact publication | Ignore unreferenced files; do not infer acceptance |
| During archival or canonical mutation | Read the last committed transaction; partial retirement is never authoritative |
| After result commit, before acknowledgement | Retry identical submission or query the committed receipt |
| During takeover with a live old worker | Stay halted; do not start a replacement writer |
| After review, before handoff | Recheck identity, evidence, cleanliness, and cessation |
| After handoff commit, before response | Read the existing handoff; ownership is already released |

## Review Identity and Context

### Current Review Identity

A review binds repository identity, target and criterion revision, approved Spec revision, diff base, head, and manifest of current raw evidence. Reviewer-produced evidence is recorded against the same repository and contract identity before judgment acceptance. A repository change during review prevents its result from authorizing progression.

Task review covers the Task's complete interval, including repairs. Spec review covers the original Spec base through current head. Repairs never narrow review to the last attempted fix. Missing evidence receives `unknown`, not a pass.

Expected later Task commits preserve settled Task reviews as historical facts. They do not establish current Spec correctness. Changed cumulative identity requires fresh cumulative review; amendments invalidate cumulative eligibility even when code is unchanged.

### Packet Contract

| Role | Permitted inputs | Writable outputs |
| --- | --- | --- |
| Planner | Charter, current Spec, repository, Task graph, discoveries, relevant prior outcomes | Proposed plan, replan, or amendment in its invocation directory |
| Worker | Current Spec and Task contract, bounded implementation context, repository rules, raw current evidence | Product files and its invocation artifacts |
| Repair worker | Worker inputs plus current blocking findings and remaining recovery instruction | Product files and its invocation artifacts |
| Reviewer | Latest approved contract, current Task criteria when applicable, current repository, complete review diff, raw current evidence | Judgments, findings, and independently produced evidence in its invocation directory |

Each Task and the cumulative Spec receives one fresh reviewer. Replacement reviewers receive only these permitted input categories. They do not receive prior amendments, retired Tasks, archives, previous attempts, verdicts, findings, worker narratives, repair instructions, journals, planner rationale, implementation plans, or child verdict summaries. Risk decisions remain controller dispositions and are not instructions to the reviewer.

Packet construction uses an allowlist. The outer harness selects the role before creating a fresh `speccy-next` context and supplies only permitted inputs. That context performs the role directly without spawning another lifecycle agent. Reviewer packets omit repair counters and reasons that reveal earlier attempts.

Reviewers may inspect current repository files and execute prescribed checks. They must not inspect execution archives, unrelated invocation directories, previous review material, or Git history outside the specified interval. The controller does not inject commit messages or full historical logs. Current product ADRs and code diffs remain legitimate inputs.

Under serial execution, repository-prescribed checks may create ignored disposable build and cache outputs. They must not change tracked product files or leave nonignored outputs at submission. Snapshot updates and other intentional product changes belong to a worker. If a check cannot satisfy this boundary or redirect its outputs into the invocation directory, the reviewer records the limitation as `unknown`.

This is a context and cooperation contract. Shared-user filesystem access, current source comments, Git history, and harness memory can disclose additional context. Pack checks inspect inherited instructions and fresh-context behavior; they cannot establish adversarial isolation without access controls. Speccy does not claim that security boundary.

A single-Task Spec still receives Task and cumulative review in the MVP. Dogfooding measures duplicate cost. Coalescing requires evidence that one invocation evaluates both complete contracts at the same identity; a cached Task verdict cannot substitute for cumulative review.

## Harness and Command Surface

`speccy-plan` gathers a contract, writes a draft through the controller, and obtains approval. `speccy-next` performs one claimed directive in the selected role. `speccy-run` repeatedly dispatches fresh `speccy-next` contexts and retains short progress summaries, pending gates, and human decisions. The closed lifecycle-role set is `planner`, `worker`, and `reviewer`.

Structured controller operations cover initialization, draft and approval recording, start, next, claim, submission and receipt lookup, human decisions, resume, status, and diagnosis. Exact command spelling is an implementation choice; each mutation requires its actor, expected revision, and defined failure outcome. Reads never create approval or launch authority. Unknown fields, invalid identifiers, and incompatible schemas fail before mutation.

Configuration resolves user-named model and effort profiles to supported harness values. Global role defaults apply unless a human approves a Spec-level worker or reviewer override. Task overrides and automatic routing are deferred. Unknown profiles fail visibly; packs never silently substitute models. Resolved settings accompany evaluation records without entering product artifacts.

Each pack must demonstrate skill routing, charter loading, fresh role contexts, permission preservation, scoped inputs, cancellation, and human-gate transfer. Harness configuration belongs in pack sources and is checked against the installed client. Claude Code requires a `CLAUDE.md` import of shared `AGENTS.md` instructions. The charter skill owns the neutral root rule; pack installation owns the harness import.

The controller never calls models or agents, runs product checks, creates Git commits, or performs forge actions. The harness performs these operations and submits evidence or provider facts. Controller validation concerns its records, Git identity, artifact integrity, and transition guards.

Repository content, tool output, and agent text cannot authorize approval or configuration changes. Same-user agents can still invoke shell commands or write outside their contract; tokens and separate operations prevent accidental stale actions, not malicious impersonation. Existing harness permissions remain the execution boundary.

## Later Product Extensions

### Projects and Milestones

A Project optionally groups independently shippable Specs under one bounded goal. Goal changes require human approval. A Milestone is an optional Project-scoped grouping; a Spec may reference one Milestone in its Project. Milestones contain Specs, never Tasks, and receive no review.

Progress derives from member Spec delivery facts. Dropped work is separate from delivered work; empty groups never report completion. Membership changes do not alter Spec contracts or historical handoffs. Implement grouping only when independently delivered Specs repeatedly need a shared goal or progress view.

### Publication and Merge Reconciliation

A later delivery stage may add a narrow GitHub path, with provider-neutral publication facts and configurable behavior. Generic forge adapters remain deferred. Authentication and exact API calls belong to that implementation contract.

A Spec selects its delivery mode before approval. In future integration mode, `review` means an eligible reviewed change has a matching open pull request; `done` requires confirmed integration of that change. Local handoff never implies merge. Publication and merge need human authorization for the exact target and reviewed change. Existing approval for that action remains valid without repeated confirmation.

The harness supplies provider, repository, base, head, pull-request reference, and observed state. Ambiguous discovery requires selection. Repeated publication or reconciliation adopts a matching existing operation without duplicates. Offline or stale observations are labeled and cannot establish a new merge fact.

A changed head before merge invalidates review. Unmatched merged contents halt reconciliation as unverified integration. Squash and rebase require content correspondence defined and tested by the provider implementation; commit equality or ancestry alone is insufficient. Base changes require re-evaluation of the integration diff. These are later feature acceptance conditions, not an implemented provider contract.

### Deferred Mechanisms

| Mechanism | Evidence required before addition |
| --- | --- |
| Concurrent implementation or worktree isolation | Serial execution limits useful throughput, with a tested ownership and merge proposal |
| Parallel read-only dispatch | Measured latency warrants additional dispatch and liveness states |
| Extra reviewers or relaxed history exclusion | Measured misses, false positives, or repeated findings justify a focused policy change; charter approval applies |
| Controller-executed product checks | Agent-produced evidence causes a failure that repository or harness checks cannot adequately address |
| Dependencies, scheduling, or timelines | Flat ordered Tasks fail a repeated delivery need |
| Backup, export, or retention automation | Observed state size or recovery needs justify a defined retention contract |
| Routing optimizers, cost accounting, or dashboards | Evaluation identifies a decision that existing profiles and status cannot support |

## Verification and Release Evidence

This repository contains documents and the charter skill, not a controller. Documentation review assesses coherence and testability. Runtime guarantees require implementation tests; effectiveness requires live evaluations.

Before local MVP use, deterministic tests must cover concurrent run and dispatch claims, lost acknowledgements, identical and conflicting retries, stale generations and revisions, complete-record validation, artifact containment and tampering, archive rollback, monotonic identifiers, every Task transition, risk boundaries, cancellation, budget exhaustion, changed review identities, and idempotent handoff. Crash injection covers the interruption table on each supported platform.

Each claimed harness must pass live checks for fresh reviewer context, scoped inputs, actual command cancellation, permission inheritance, human decisions, profiles, and resume after committed work. The charter skill's [acceptance scenarios](REVIEW.md#skill-acceptance-scenarios) distinguish metadata validity from behavior.

Dogfooding compares a regression fix, a contract-preserving replan, and an approved amendment against native planning with persistence and fresh review on comparable changes. Record model and harness settings, human intervention time, incorrect completion claims, escaped defects, blocking false positives, repairs, and duplicate review cost. These evaluation records do not require a product telemetry service.

State corruption, overlapping writers, lost approved contracts, or false completion block release until reproduced and fixed. If recovery and inspectability benefits do not justify interaction cost, revisit the workflow before adding hierarchy or publication. Completion-time and quality thresholds remain unvalidated until representative trials establish a baseline.
