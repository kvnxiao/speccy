# Decision Record

Status: rationale record
Date: 2026-07-04

Durable decisions and the alternatives they rejected, grouped by area. This exists to stop settled questions being re-litigated; `DESIGN.md` and `TERMINOLOGY.md` are authoritative for current behavior, and if anything here conflicts with them, the design docs win. Mechanics that DESIGN already states plainly are not repeated — only the reasoning and the rejected alternative are. Chronology lives in git history; live questions and dogfood watch items live in `OPEN-ITEMS.md`.

## Architecture and scope

- **Two harnesses (Claude Code + Codex) are both first-class in MVP.** Rejected trimming to one: shipping two forces the template renderer's conditional exports to be real from day one instead of speculative single-target code.
- **No outbound agent runner.** The harness calls Speccy's deterministic controller; Speccy never launches an LLM or harness. Non-negotiable — it is what keeps the core deterministic.
- **Runtime state lives in `~/.speccy/` only.** Repo-local `.speccy/` holds only `project.yaml` and `pack-lock.yaml`. Rejected `policies/`/`roles/`/`evidence-presets/` folders — that prose template-renders into the harness packs. The `.gitignore` block stays as a defensive backstop. If mutable state ever becomes git-visible, it must be append-only with a union-by-event-id merge driver; replace-style merges silently lose data.
- **Non-git workspaces are unsupported.** Snapshots and `baseline_commit` require git.
- **Parallel writes are out of MVP.** Serial only, lease-enforced; worktree parallelism is post-MVP.
- **Language/engine/license decided: Rust single static binary, `minijinja`, MIT.** Distribution channels stay open (Q23).

## Harness packs and rendering

- **Codex reads repo-local skills from `.agents/skills/<name>/SKILL.md`** (the Agent Skills standard). `.codex/skills/` does not exist; Codex user-global prompts are deprecated and not rendered. Codex subagents are `.codex/agents/<name>.toml`.
- **Claude entry skills are `.claude/skills/<name>/SKILL.md`** (both `/name` and natural-language auto-invoke). `.claude/commands/` is legacy and never auto-invokes, so it is not rendered. Claude subagents are `.claude/agents/*.md`.
- **The generic `.agents/` role-file target was cut:** no cross-harness convention exists for agent-definition files. Targets are `auto | codex | claude | all`. A core-fields-only generic skills pack plus a root `AGENTS.md` pointer moved to Later Capabilities.
- **Only entry skills may reference the structured-question tool** (`AskUserQuestion` / `request_user_input`); availability inside subagents is unverified.
- **Plain repo-local files are the MVP default.** Plugin manifests deferred — Claude plugin subagents lose `hooks`/`mcpServers`/`permissionMode`.

## Statuses and state machines

- **Requirement statuses are the six kept in `DESIGN.md`.** Rejected a `vacuous` status (vacuity is a finding reason, folds to `failed`) and an `unproven` status (folds to `blocked`; both skip repair and escalate as a human/policy gate).
- **Task statuses are the five kept in `DESIGN.md`.** Rejected `reviewable` (a handoff records straight to `in_review`) and `needs_repair` (a failed review with rounds remaining is derived, re-entering `building` with the round counter incremented).
- **Run opens directly in `implementing`.** Rejected a `created` state.
- **Spec statuses:** rejected `obsolete` (unreachable — nothing sets it, and `archived` already excludes from planning); added `cancelled` for human abandonment.
- **Naming disambiguation:** run terminal state is `landed`, task terminal state is `integrated`, and `accepted` is a spec status only. The risk tier is `minimal` (not `tiny`); the scope ladder keeps `tiny`.
- **`defer`/`deferred` cut entirely.** `waive` (accept a requirement's risk) and `cancel` (abandon the run) already cover the escalation resolutions; a task-scoped defer that cascade-waives orphaned requirements added state-machine surface without an MVP need. Task terminal state is `integrated` only.
- **`waived` is gate-only and terminal for the run**, set atomically inside `run record-decision` — the only requirement-status mutation outside `requirement set-status`.
- **Waive requires a `residual_risk` note at every tier**, not only high/critical. A waiver is an accepted risk by definition, so the note recording what risk is accepted is mandatory regardless of tier; the DESIGN tier table now matches the code, which has always required it. Rejected tier-conditional leniency for waive. `review_passed` stays tier-conditional (its `residual_risk` requirement applies only at high/critical) because it rests on real, if weaker, evidence rather than an accepted gap.
- **`failed` may rest on a reviewer finding, not only recorded evidence.**

## The loop (run next)

- **`run next` is the single loop entry and the single mutation point for derived state.** Rejected a separate resume operation: `run next` clears expired leases and re-derives the directive, so the loop has exactly one entry point.
- **The directive action vocabulary is the five kept in `DESIGN.md`.** Rejected separate `spawn_repair_round` (a repair is `dispatch_worker` with `round.current > 1`), `run_final_validation` (run-gate is `dispatch_verifier` with `round.scope: run`), a distinct task-verifier action, and `emit_escalation_packet` (escalation is `await_human_gate` with `subject.gate: escalation`).
- **Idempotency is semantic**, not byte-identical: all directive fields match, while `lease`, `applied_transitions`, and `resume` are per-call and excluded from the comparison.
- **Directives report derived transitions in `applied_transitions`.** Rejected renaming the op to `run advance`: transparency was the gap, not the name.
- **Gates carry a `gate_answers` map** because a gate has several legal outcomes with different recorders; `record_with` at a gate is only the default recorder. Rejected keying it to "the recommended answer's recorder" (the escalation gate recommends amend while defaulting elsewhere).
- **Repair caps are two policy values** (per-task, run-level review), controller-enforced and surfaced through `run next`; the agent never counts rounds. Run-level repair is a dynamic `RT<n>` task against `run_review_rounds`. Blocked requirements skip repair and escalate directly.

## Lease and concurrency

- **A run-level lease enforces serial writes.** `run next --agent` issues/renews an agent-bound token (10-minute TTL) passed as `--lease` on state-mutating ops; a second session gets `lease_held`; expired leases are cleared by the next `run next`. The lease is run-scoped, so spec-phase ops are not gated.
- **Lease-gated mutations validate and append under one store-lock hold (locked mutation service).** `task claim`, `task record-handoff`, `requirement set-status`, `run record-decision`, `run record-ship`, and `run interrupt` replay the projection, verify the live lease, validate the transition, and append their events inside a single hold of the per-workspace store lock, returning the settled projection. Rejected check-then-append with per-append locking: two processes could validate the same pre-state and both commit an incompatible transition, and a lease cleared or reissued between check and write could let a stale token append.
- **`finding record` and non-command `evidence record` are lease-free** — one file per ID, never a shared journal — so concurrent reviewer personas never contend. Event appends serialize on a per-workspace store lock.
- **`evidence collect` for `kind: command` is not lease-free.** It runs a real command that can mutate the worktree, so it takes a separate workspace command lock: personas collect without the run lease, but commands serialize.

## Evidence and vacuity

- **The controller executes `kind: command` evidence.** `evidence collect` runs the command and records exit code/stdout/stderr/hash; `evidence record` refuses agent-pasted output for that kind, so `passed` on command evidence never rests on a transcript. Trust narrows to review/browser/manual kinds, which the tiers already treat as weaker.
- **Commands run through the platform shell** in the workspace root under timeout and output-byte caps, with pre/post worktree dirty-state recorded and known-secret env values scrubbed (full redaction is Q18).
- **The command allow policy is a drift guardrail, not an authorization boundary.** Patterns match the whole declared command string (glob, never a prefix — `npm test && curl …` matches nothing unless explicitly allowed), linted at draft time and refused at `evidence collect`. The harness sandbox stays the security boundary. Rejected structured-argv / named-script references as heavier than the guardrail's job. Unset means any approved command runs; the spec card always shows the command strings.
- **Browser/api evidence on high/critical requires a stored, hashed artifact** and refuses prose-only records; the controller enforces presence, not authenticity. Optional at minimal/standard.
- **Vacuity is risk-scaled prose review, not mandatory mutation testing.** Requirement-to-test traceability lives in store-side evidence records, never in test names or comments — that would ship process provenance.

## Reviewer personas and provenance

- **Personas are first-class and configurable**, rendered one subagent per persona per target with per-persona `model`; the roster is echoed in verification directives so fan-out is controller-stated, and each finding carries its persona. Default roster `spec-fidelity, defects, security, style`; `minimal` risk collapses to one combined reviewer.
- **"Correctness" is split into `spec-fidelity` and `defects`:** they fail independently, and a combined prompt anchors on the ledger and under-hunts latent bugs. Rejected further default splits (performance, test quality, docs) as roster bloat — teams add custom personas.
- **Every review round runs the full roster.** Rejected skipping personas that raised no prior finding: a repair diff is new code, and a security regression introduced while fixing a style finding must not survive. Findings and rejection reasons carry forward so no persona re-discovers a known failure.
- **Provenance hygiene: shipped file contents carry no Speccy terminology or identifiers.** Three cheap layers: a deterministic deny-list scan over task and integrated diffs (blocking findings into repair rounds), a worker/repair prompt rule, and the `style` persona's semantic backstop. Bare task IDs (`T1`, `RT2`) are excluded from the scan — too short and common to match without false positives, and such leakage falls to the style persona. Rejected a dedicated provenance persona: identifiers are mechanical to catch, and a run-gate-only auditor would catch task-level leaks late. Exempt: rendered packs, `.speccy/`, explicit exports. Git history and PR metadata are team policy — ship offers squash by default.

## Git: branch, snapshot, resume

- **The first `run start` for a spec creates `speccy/<spec-ref>-<slug>` from checked-out HEAD** and records that HEAD as the base; later runs of the same spec reuse the branch. `run start` never picks a base itself — the clean-worktree refusal is the only precondition.
- **Controller commits use the `Speccy <noreply@speccy.local>` identity and `speccy: <spec-ref> …` messages; the controller never squashes.** `baseline_commit` is recorded per task at claim; a snapshot commit is recorded at `integrated`; writes are atomic (temp→fsync→rename, verified read-back for JSONL).
- **An out-of-band commit parks the run at an `escalated` policy gate and takes no snapshot.** Committing a Speccy snapshot on top would bury or misattribute the human's out-of-band commit and worktree edits, so the diff is left as-is. The give-up escalations — cap exhaustion, blocked requirement, resource cap — do commit the in-flight diff as a labeled snapshot. A superseding run reconciles on the same branch; rollback stays the explicit human fallback. Rejected: uniform snapshotting on every escalation.
- **Resume is a `run next` capability, not a command or a human ritual.** Task statuses, the round counter, git snapshots, and expired-lease clearing let a fresh session re-derive the next step. A fresh session re-enters via `/speccy-implement`.
- **Resume attribution is visible, not silent.** `run next` reports lease repair in a `resume` field summarizing the dirty diff versus the task baseline; the skill echoes it before dispatching. Rejected a blocking adopt/stash/cancel gate on dirty resume: human-edits-while-dead is undetectable (no diff is recorded at crash time), so a gate would tax every routine crash resume as a false positive. The escape hatch is git-native — a stash removes edits from attribution; a commit becomes an out-of-band commit that parks the run.

## Human gates and workflow

- **There are exactly five human gates:** spec-card approval, escalation, critical-tier accepted-risk confirmation, ship, and merge acknowledgement. Sandbox permission prompts belong to the harness, not a gate.
- **The accepted-risk ship confirmation fires only when the accepted-risk bucket is non-empty, at any tier.** The trigger is residual risk, not tier. Rejected repeatedly a tier-conditional or high-tier accepted-risk gate: it would double-gate the most common serious tier, the ship gate is already the decision point, and high already requires `residual_risk` notes.
- **Spec-card approval is explicit prose recorded through the controller.** Verbs: `go` (approve and start in-session), `approve only` (approve and print the handoff), plus `revise:`/`split:`/`cancel`; ambiguous prose defaults to `approve only`, and approval alone never auto-starts. Rejected documenting "looks good"/"looks good, go" as primary paths — too subtle for a binding gate.
- **The approval echo is the sole binding guard.** Rejected a controller-tracked draft-version refusal, ref-bearing approval replies, and a `/speccy-approve` command — all re-add the ceremony the prose-approval decision removed. Watch: reinstate a draft-version refusal first if approvals misbind in practice.
- **`/speccy-plan` runs route preflight.** Tiny direct edits, normal harness-plan work, and split-worthy initiatives route away creating no spec/run state; "use Speccy anyway" overrides. This moves "when to use Speccy" from user education into the skill's default behavior.
- **Scope rating is prose guidance the skill reasons through, not a stored artifact;** evidence-ability is the first routing factor, and low evidence-ability routes away even for large work.
- **No per-control skills** (`/speccy-approve`, `/speccy-repair`, `/speccy-waive`): approval is prose in `/speccy-plan`, repair is autonomous, amendment and waivers are conversational, and acceptance is the `speccy accept` CLI command.
- **`rework` keeps minor implementation feedback in the same run:** a `run record-decision` type that moves `verified → implementing`, appends an `RT<n>` seeded with the feedback, and returns to the same ship gate. Scope or requirement feedback stays the amendment path. Rejected forcing full re-approval for trivial feedback.
- **Amendment at the escalation gate is a deferred gate answer:** the run stays parked while the draft loop runs, and `spec record-decision` (approve with `supersedes.run_id`) atomically closes the parked run as `cancelled`. Rejected a new `amend` decision type — it creates zombie-run state and double-records the decision.
- **A superseding approval validates the target run before recording, but its two-append crash window is documented, not reconciled.** `spec record-decision approve --supersedes.run_id` resolves and checks the run (exists, parked at escalation or ship gate) *before* appending the approval, so a bad run_id records nothing. A crash between the approval append and the parked run's cancellation leaves the run open; recovery is the gate's ordinary cancel answer. Rejected a cross-log spec read on every `run next` to auto-reconcile — it taxes the hot loop for a rare crash window. Revisit point: the `run_next` guard section, if dogfooding shows orphaned superseded runs.
- **A gate resume does not count as a review round; `provide_setup` re-opens the current round.** Resuming from `escalated` writes a distinct `RunResumed` event (not a `RunStateTransitioned`), so replay re-enters `verifying`/`implementing` without incrementing the run-level review-round counter. `provide_setup` re-arms the current round (re-review, or the stuck task's worker at its same round); a waiver that resolves the last requirement completes `verifying` with no re-review. Rejected reusing `RunStateTransitioned`: it counts the resume as a fresh round, so a run parked at its cap resumes at cap+1 > max and re-escalates immediately — an infinite gate loop — and re-arming the verifying marker made the waive's own status write read as pre-round, forcing a spurious re-review.

## Human CLI and UX

- **`speccy status` is the everyday hub; the routine surface is four commands** (`status`, `list --query`, `review`, `accept`). Rejected the flat fourteen-command "common" list — it read as an admin surface.
- **Selector inference is the normal path;** full `SPEC-...` references are for ambiguity, scripts, CI, and PR metadata.
- **`speccy review` is state-aware** (spec card → status → review packet → escalation → close-out → accepted summary); `--evidence` is the ledger/artifact/diff drill-down.
- **`speccy accept` is a manual human assertion.** It uses the recorded `change_ref`, is idempotent after landing, and takes `--pr`/`--note` only for recovery. Accepted specs leave default `status`/`list` immediately; `archive` is reserved for stale historical context, not routine close-out.
- **The first `speccy install` previews the would-write listing and asks before writing** (`--yes` skips; noninteractive writes require `--yes`; a no-op never prompts). The preview groups by target when several are detected, and `--target` narrows. Rejected a `--local`/`--team` split (the `.gitignore` backstop already separates runtime state from committed files) and a `--user` trial install (deferred; detection is directory-based, single-harness repos already get one pack, and narrowing a dual-harness repo breaks teammates).
- **Status cards are a rendering rule, not new state:** run status labels, task titles never IDs, and a last-activity line derived from the event log. First screens render `review_passed` as "review-only evidence," never the raw enum.
- **A fresh session for `/speccy-implement` is recommended, never required** — approval and run state are controller-backed.
- **Merge detection is cut from MVP,** including git-native ancestry heuristics. A local ancestry check is silent on squash merges (the common GitHub mode), so an "appears merged?" prompt would mostly not fire and its silence would read as "not merged." Compensation: ship prints the exact `speccy accept` command, the Awaiting-merge card carries it, and PR metadata includes `accept_with`. Watch: a status-card ancestry prompt is the first candidate if accepts get forgotten.

## Storage and validation

- **Workspace identity** is a hash of the canonical workspace root and canonical git root; monorepo subtrees get distinct workspaces, and moves/re-clones yield new IDs (`doctor` reports orphans). `SPECCY_HOME` overrides `~/.speccy` for tests and CI.
- **The store is JSONL-first:** spec-scoped and run-scoped `events.jsonl` are canonical; the YAML/markdown files are derived projections and a generated snapshot.
- **Strict schema validation with bounded repair** (Q6): structured lint errors, a retry cap (default 3), then fail closed to `escalated`.
- **The controller receives the structured-output-retry-exhaustion signal; the count lives in pack prose.** `run interrupt` (lease-gated, closed `reason` vocabulary) records the reason as a run decision, snapshots the in-flight diff, and parks at the existing escalation gate — no new gate or state. Rejected making the controller count retries: it never runs an LLM and never sees raw subagent output, so counting there would be fiction; keeping the count in prose preserves determinism.
- **Resource caps fail closed to an `escalated` policy gate** (Q15): round caps plus optional task-count/wall-clock caps in `project.yaml`; Speccy cannot meter tokens.
- **Approved revisions are immutable in place** (Q20): statements and evidence requests are frozen; agents propose draft patches, humans approve new revisions, and verifiers touch status only. A draft op against an approved spec opens revision N+1.
- **Exports are trimmed to `export review`, `export spec`, and `export run-bundle --redact`;** others wait for proven dogfood need.

## Review hardening fold (2026-07)

- **A four-axis read (correctness, completeness, maintainability, performance) was folded in as Phase A controller-correctness fixes and Phase B mechanical fixes.** The durable per-decision outcomes are recorded inline in the sections above (waive `residual_risk` at all tiers, out-of-band no-snapshot, superseding-approval crash window, gate-resume review-round, `run interrupt`); the rest were bug fixes, dedupe, and doc reconciliations carrying no design change. The three accepted-cost perf items live in `OPEN-ITEMS.md`.
- **Event-record `kind`/`severity` fields are typed enums, and replay fails closed on an out-of-vocabulary value.** `FindingRecord.severity`, `EvidenceRecord.kind`, both decision-record `kind`s, `ChangeRef.kind`, and `RunStarted.risk` deserialize into closed enums whose canonical vocabularies live in `DESIGN.md`. A hand-corrupted log — or one written by a newer binary — fails replay as `corrupt event` rather than silently reading an unknown value as a safe default. Rejected keeping them `String`: the store is already fail-closed on truncation, and a silent severity/kind fallback is exactly the class of bug the ledger must not have. Accepted cost: a log written by a newer binary is unreadable by an older one — acceptable for a local single-binary tool. `run record-ship` now rejects an out-of-vocabulary `kind` at intake instead of storing it.
- **A directive serializes absent optionals as explicit `null`, not omitted keys.** `Directive.round`/`gate_answers` and every `Subject` field match the SCHEMAS § Directive shape; only `AppliedTransition.snapshot` stays presence-conditional. Rejected omission: a stable key set is easier for the packs and any JSON consumer to read, and idempotency already compares by value.

## Deferred to Later Capabilities

Designed in `DESIGN.md`, cut from the MVP build; none changes the state contract.

- **Delta-scoped re-review** — round snapshots and per-round deltas so a repair-round reviewer reads only the changed slice. A token optimization unproven against real reviewer cost; full-diff re-review is correct and simple. Dogfood roster-cost decides whether it earns its machinery.
- **Command-evidence dedup cache** — a per-round cache keyed on the command string. The workspace command lock already serializes runs at MVP scale.
- **Decision index** — the archived-inclusive projection surfacing carry-forward decisions into planning packets, with a cap and overflow drill-down. It matters only once a workspace archives specs, which the single-spec dogfood does not reach; the `carry_forward` flag is recorded from M3 so the projection needs no data migration. Rejected a stored index file (a second source of truth) and a repo-committed export (reverses runtime-state-out-of-repo). Prior-context candidates over active specs ship at M3; the archived-inclusive index must land before multi-spec dogfooding.
- **Automatic merge detection** — see the merge-detection decision above.
