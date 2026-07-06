# Speccy: End to End in Claude Code

Status: illustration (mocked outputs)
Date: 2026-07-03

One complete scenario, from installing the harness pack into an existing
repository through archiving the shipped spec: passwordless login via emailed
magic links, in a TypeScript app, driven from Claude Code.

The scenario is organized by lifecycle area, and each area is bounded by a
human intervention checkpoint. Every section leads with **what the human types
and reads** — routine use never surfaces controller operations, JSON, leases,
or task states — and then a **Behind the checkpoint** part showing the
controller operations that produce it. Read the top of each section for the
human's-eye view; read the machinery for the protocol reference. Every
controller operation and human command appears at least once (coverage table in
Appendix E).

This document is illustrative. `DESIGN.md`, `TERMINOLOGY.md`, and `SCHEMAS.md`
are authoritative; if anything here conflicts with them, they win.

## Scenario

- Repository: `acme-app`, a TypeScript web app (Express API + React), on `main`, clean worktree.
- Existing auth: password + session cookies under `src/server/auth/`. Email sending exists under `src/server/email/`.
- Harness: Claude Code. The developer already uses `.claude/` for team config.
- Goal: passwordless login via emailed magic links.
- A prior spec exists in this workspace: `SPEC-20260401-9C21` "Session hardening" (accepted).

The five human checkpoints, in order: **approve the spec card** (§2), the
autonomous run's terminus — **ship or send back** (§3→§4), and **acknowledge
the merge** (§4→§5). Two more fire only on a branch not taken: the **escalation
gate** (Appendix A) and, on `critical` specs, an **accepted-risk confirmation**.

### Who acts behind the checkpoints

A human never sees these seats in routine use; they are the cast for the
machinery below.

| Seat | Who | Calls |
| --- | --- | --- |
| Human | the developer | `speccy` human CLI, slash commands, prose approvals |
| Orchestrating skill | the Claude Code session running a `/speccy-*` skill; holds the run lease | `spec *`, `run *`, `task *`, `packet *`, `requirement set-status` |
| Worker subagent | fresh-context implementer spawned per task round; its prompt carries the provenance rule (no Speccy identifiers in product files) | none — receives a task packet, returns a handoff |
| Reviewer personas | fresh-context reviewers spawned per verification, one per roster entry (`spec-fidelity`, `defects`, `security`, `style`), each with its own `model` frontmatter | `evidence collect`, `evidence record`, `finding record` — no run lease needed; command `evidence collect` still serializes on the workspace command lock |
| Controller | the `speccy` binary | executes `kind: command` evidence; runs the provenance scan; applies derived transitions, round snapshots, and git snapshots inside `run next` |

All `ctl` outputs use the envelope `{ok, data}` on success and `{ok: false, error: {code, message, details?}}` on refusal.

---

## 1. Setup — install the pack

### What the human sees

```console
$ speccy install
Detected harnesses: claude (.claude/ exists)

This install creates 14 repo files and updates .gitignore:
  create  .speccy/project.yaml
  create  .claude/skills/speccy-plan/SKILL.md
  ... (12 more)

Proceed? [y/N] y

Install OK. These are committed workflow artifacts; commit them to share
the workflow with your team. Runtime state lives in ~/.speccy/ only.

$ git add -A && git commit -m "Add speccy workflow pack"
```

These are committed workflow artifacts — commit them so the team shares the
workflow. Runtime state stays out of the repo, in `~/.speccy/`.

### Behind the checkpoint

The full first-run footprint, grouped by target when more than one harness is
detected:

```console
$ cd ~/code/acme-app
$ speccy install
Detected harnesses: claude (.claude/ exists)
Rendering pack: claude @ pack 0.1.0

This install creates 14 repo files and updates .gitignore:

  create  .speccy/project.yaml
  create  .speccy/pack-lock.yaml
  create  .claude/skills/speccy-brainstorm/SKILL.md
  create  .claude/skills/speccy-plan/SKILL.md
  create  .claude/skills/speccy-implement/SKILL.md
  create  .claude/skills/speccy-ship/SKILL.md
  create  .claude/agents/speccy-planner.md
  create  .claude/agents/speccy-worker.md
  create  .claude/agents/speccy-verifier.md
  create  .claude/agents/speccy-repair.md
  create  .claude/agents/speccy-reviewer-spec-fidelity.md
  create  .claude/agents/speccy-reviewer-defects.md
  create  .claude/agents/speccy-reviewer-security.md
  create  .claude/agents/speccy-reviewer-style.md
  update  .gitignore  (defensive .speccy/ block)

Proceed? [y/N] y

Install OK. These are committed workflow artifacts; commit them to share
the workflow with your team. Runtime state lives in ~/.speccy/ only.
```

Nothing is written before the confirmation; `--yes` skips it, `--dry-run`
prints the same listing and stops, and an install with nothing to do never
prompts. Noninteractive installs require `--yes` to write.

The four `speccy-reviewer-*` files are rendered from the persona roster in `project.yaml` — one subagent per entry, per-persona `model` in the frontmatter. Editing the roster and re-running `speccy install` adds or removes persona files.

`.speccy/project.yaml` (machine-readable policy the controller enforces; full schema in `DESIGN.md`):

```yaml
risk_default: standard
caps:
  task_repair_rounds: 3
  run_review_rounds: 3
  structured_output_retries: 3
  max_tasks: null
  max_run_wall_clock_minutes: null
evidence:
  command_timeout_seconds: 600
  command_output_max_bytes: 1048576
  command_policy:
    allow: []                # empty = any approved command may run
review:
  personas:
    - name: spec-fidelity
    - name: defects
      model: opus
    - name: security
    - name: style
      model: haiku
provenance:
  extra_terms: []
```

`.gitignore` gains the defensive backstop (guards paths that should never exist, not a supported layout):

```gitignore
.speccy/*
!.speccy/project.yaml
!.speccy/pack-lock.yaml
```

Health check, then commit the pack:

```console
$ speccy doctor
git    OK  (2.45.0)
store  OK  (~/.speccy writable; workspace ws_a81f23)
packs  OK  (claude @ 0.1.0; rendered files match pack-lock)

$ git add .speccy .claude .gitignore && git commit -m "Install Speccy claude pack"
```

Re-running `speccy install` is idempotent: it repairs missing managed files and reports available updates, but never rewrites edited prose without `--update`.

---

## 2. Plan — approve the spec card  ·  CHECKPOINT: spec-card approval

### What the human sees

Optionally brainstorm first, then plan. In Claude Code:

```text
/speccy-plan add passwordless login via emailed magic links
```

The skill first checks whether this belongs in Speccy at all. For this auth
feature it recommends a Speccy spec, then inspects the repo, drafts the spec,
and presents one card. A tiny request would route away here with no spec state:
direct edit, regular harness plan, or split guidance.

```text
Spec: SPEC-20260630-A7F4  Passwordless login          Risk: high
Decision needed: approve this spec, or revise scope
Reply: go · approve only · revise: <changes> · split: <guidance> · cancel

Goal: users sign in through single-use emailed magic links.
In scope: request link by email · token expiry + replay protection · expired-link UI
Out of scope: OAuth · admin session revocation · email vendor migration
Plan: token model + endpoints → expired-link UI

Acceptance
- A magic link can be requested by email
- A link is single-use
- Links expire after 15 minutes
- An expired link is rejected and creates no session

Will run:   npm test -- auth/magic-link · npm test -- auth/expiry ·
            npm test -- ui/expired-link
Prior context: tokens stored hashed (SPEC-20260401-9C21)
Main risk: email delivery may need staging validation

— rev spec_rev_001-draft · requirements R-AUTH-001…004 · ledger/evidence: speccy review --evidence
```

The card reads as a decision, not a controller artifact: acceptance is plain
statements, and the process identifiers (revision, draft version, requirement
IDs) sit in the footer for reference. The canonical reply `go` approves and
starts the run right here — the last input the run needs:

```text
Human: go

Recording approval: SPEC-20260630-A7F4 rev spec_rev_001 -> approved
"go" heard — starting the run in this session.
(Prefer a clean context? Reply "approve only", then run
/speccy-implement in a fresh session.)
```

`approve only` approves without starting; you launch it later with
`/speccy-implement` when this is the only approved spec (a fresh session is
recommended, never required). `revise: <changes>`, `split: <guidance>`, and
`cancel` route the other decisions without adding separate commands.

### Behind the checkpoint

#### 2.1 Brainstorm (optional, ephemeral)

The brainstorm skill makes **no controller calls**. It explores the repo read-only and produces a handoff in chat:

```text
/speccy-brainstorm "let users log in without passwords"
```

```text
Brainstorm handoff — passwordless login

Observations:
- Auth lives in src/server/auth/ (bcrypt passwords, session cookies)
- Email delivery exists: src/server/email/ (SES wrapper, used for receipts)
- Scripts: npm test, npm run lint, npm run dev
- Prior spec SPEC-20260401-9C21 (session hardening, accepted):
  tokens stored hashed, cookies httpOnly — carry forward

Candidate approaches: emailed magic links, TOTP, WebAuthn
Suggested scope: magic links only; WebAuthn is a separate spec later

Scope rating:
  size: large
  evidence_ability: high
  risk_domains: [auth]
  estimated_tasks: 2
  recommended_route: speccy_spec
  confidence: high

Recommended next action:
  /speccy-plan Continue from the Speccy brainstorm handoff above.
```

The handoff is chat context only. It is persisted exactly once, in the next step, if the user promotes it.

#### 2.2 Route preflight, create the spec, and fetch the planning packet

`/speccy-plan` uses the same scope-rating rules as brainstorm before it writes
anything. If the request is too small or better handled by normal planning, it
returns a route-away card and stops:

```text
/speccy-plan fix typo in README heading

Recommended route: direct edit
Reason: tiny docs change; Speccy evidence ledger would cost more than the work.
Next action: make the edit directly in this session.
Override: reply "use Speccy anyway" to force a spec.
```

For the passwordless login request the route is `speccy_spec`, so the skill
continues. It writes `request.json` and creates the spec; `brainstorm_handoff`
is the only persistence point for a promoted handoff:


```console
$ speccy ctl spec start --input request.json --json
```
```json
{ "ok": true, "data": {
    "spec_ref": "SPEC-20260630-A7F4",
    "spec_id": "spec_01j1bxgvk3e6q8r2n5tcvh7pyd",
    "status": "draft",
    "workspace_id": "ws_a81f23" } }
```

The planning packet is a deterministic work order — everything below comes from the store, git, and parsed manifests. No LLM call.

```console
$ speccy ctl packet planning --spec SPEC-20260630-A7F4 --json
```
```json
{ "ok": true, "data": {
    "request": "Let users log in without passwords via emailed magic links",
    "brainstorm_handoff": "Brainstorm handoff — passwordless login\n...",
    "draft_state": "empty",
    "workspace": {
      "git": { "head": "f3d9e21", "branch": "main", "dirty": false },
      "signals": { "scripts": ["npm test", "npm run lint", "npm run dev"],
                   "language": "typescript" } },
    "prior_context_candidates": [
      { "spec_ref": "SPEC-20260401-9C21", "title": "Session hardening",
        "status": "accepted",
        "hints": ["dec_20260401_003: tokens/credentials stored hashed"] } ],
    "policy": { "risk_default": "standard",
                "task_repair_cap": 3, "run_review_cap": 3 },
    "output_contract": { "submit_with": "spec record-draft",
                         "required": ["goal", "scope", "risk",
                                      "requirements", "tasks"] } } }
```

`prior_context_candidates` is the active-spec prior context — the accepted `SPEC-20260401-9C21` surfaces because it is not archived. Its carried-forward decisions (all of them, as `hints`) become the card's "Prior context" line. (Surfacing decisions from *archived* specs in planning is deferred to Later Capabilities; the `carry_forward` flag is recorded now. See "Carry-Forward Decisions" in `DESIGN.md`.)

#### 2.3 Draft, lint, patch

The planner inspects the codebase read-only, reconciles the prior spec against current code, and submits one complete candidate (`spec-draft.json`, abbreviated):

```json
{
  "goal": "Users can sign in through single-use emailed magic links",
  "scope": { "in": ["request link by email", "token expiry and replay protection",
                    "expired-link UI state"],
             "out": ["OAuth", "admin session revocation", "email vendor migration"] },
  "risk": "medium",
  "observations": ["auth code under src/server/auth/", "SES wrapper reusable",
                   "prior hashed-token decision carried forward"],
  "requirements": [
    { "id": "R-AUTH-001", "statement": "A user can request a magic link by email.",
      "evidence": [ { "id": "E1", "kind": "command", "command": "npm test -- auth/magic-link" } ] },
    { "id": "R-AUTH-002", "statement": "A magic-link token is single-use." },
    { "id": "R-AUTH-003", "statement": "Tokens expire after 15 minutes.",
      "evidence": [ { "id": "E1", "kind": "command", "command": "npm test -- auth/expiry" } ] },
    { "id": "R-AUTH-004", "statement": "An expired link shows the expired state and creates no session.",
      "evidence": [ { "id": "E1", "kind": "browser" },
                    { "id": "E2", "kind": "command", "command": "npm test -- ui/expired-link" } ] } ],
  "tasks": [
    { "id": "T1", "title": "Token model, expiry, request/consume endpoints",
      "requirements": ["R-AUTH-001", "R-AUTH-002", "R-AUTH-003"] },
    { "id": "T2", "title": "Expired-link UI state",
      "requirements": ["R-AUTH-004"] } ]
}
```

```console
$ speccy ctl spec record-draft --spec SPEC-20260630-A7F4 --input spec-draft.json --json
```
```json
{ "ok": true, "data": {
    "draft": "spec_rev_001-draft",
    "lint": { "clean": false, "findings": [
      { "code": "missing_evidence_request", "path": "requirements[R-AUTH-002]",
        "message": "R-AUTH-002 has no evidence request" },
      { "code": "invalid_risk_tier", "path": "risk",
        "message": "\"medium\" is not one of minimal|standard|high|critical" } ] } } }
```

Lint findings come back in the write response — no separate lint call. The skill repairs with a focused patch (risk becomes `high`, an auth domain; R-AUTH-002 gets a browser evidence request):

```console
$ speccy ctl spec patch-draft --spec SPEC-20260630-A7F4 --input spec-patch.json --json
```
```json
{ "ok": true, "data": { "draft": "spec_rev_001-draft",
    "lint": { "clean": true, "findings": [] } } }
```

#### 2.4 Prose approval and binding

The skill renders the card (shown above) from the clean draft. The card lists the distinct `kind: command` strings the controller will execute, because approval authorizes them to run.

Human: `approve only`

The skill echoes the decision it is about to record, so prose in a long chat cannot silently bind to the wrong spec or a card the human never saw (no further reply needed). The echo is the binding guard; the controller tracks no per-write draft version:

```text
Recording approval: SPEC-20260630-A7F4 rev spec_rev_001 -> approved
```

```console
$ speccy ctl spec record-decision --spec SPEC-20260630-A7F4 --input decision.json --json
```
```json
{ "ok": true, "data": {
    "approved_revision": "spec_rev_001",
    "spec_status": "approved",
    "requirements_frozen": true,
    "next": "Run /speccy-implement (fresh session recommended)." } }
```

Had the approval said `go`, the skill would start the implement loop right after this echo, in the same session. This walkthrough shows `approve only` and a cold start to illustrate the controller-backed handoff.

The revision is now immutable in place. A later `spec patch-draft` opens draft `spec_rev_002` seeded from it; only a new prose approval makes that draft binding, and any operation that would mutate the approved revision itself returns `{"ok": false, "error": {"code": "invalid_transition", ...}}`.

---

## 3. Implement — the autonomous run  ·  CHECKPOINT: ship or send back

### What the human sees

The run is going ("go" started it, or `/speccy-implement` in a fresh session). The loop is autonomous — implementation, fresh-context review by four reviewer personas, repair rounds, re-verification. Glance from any terminal:

```console
$ speccy status
SPEC-20260630-A7F4  Passwordless login          Risk: high
  Implementing — token model + endpoints · repair round 2 of 3
  · autonomous, nothing needed
  Last activity 2m ago — running npm test -- auth/expiry
```

If the run hits something it cannot fix or prove, it stops and asks one question (Appendix A). Otherwise the next thing you see is the review packet — evidence, not a transcript:

```text
Spec   SPEC-20260630-A7F4  Passwordless login      Risk: high
Result verified — ready to ship · 1 accepted risk
Recommended next action: /speccy-ship

Requirements (4)
  Proven          3
  Accepted risk   1   on review-only evidence

Accepted risk
  R-AUTH-002  review-only evidence  Single-use proven in browser only; no unit test

Changed  11 files  +463 -41     2 tasks · 1 repair round
Evidence + full diff:  speccy review --evidence
```

Two decisions here: ship it (§4), or describe what is wrong in prose and the skill sends the work back (minor feedback reruns inside the same run; scope changes amend the spec for re-approval — Appendix C, "send it back").

### Behind the checkpoint

#### 3.1 Gate check and run creation

The human starts a fresh session — recommended for clean implementation context, never required — and runs `/speccy-implement`. With one approved spec, the skill infers the target; if several specs are approved it asks for a selector. Approval is controller state, not chat memory:

```console
$ speccy ctl spec status --spec SPEC-20260630-A7F4 --json
```
```json
{ "ok": true, "data": { "spec_ref": "SPEC-20260630-A7F4",
    "title": "Passwordless login", "status": "approved",
    "active_revision": "spec_rev_001", "risk": "high", "runs": [] } }
```

```console
$ speccy ctl run start --spec SPEC-20260630-A7F4 --revision spec_rev_001 --json
```
```json
{ "ok": true, "data": {
    "run_id": "run_01j1bxgvk3tf4qs6mv9zpxwe8d",
    "run_state": "implementing",
    "branch": "speccy/spec-20260630-a7f4-passwordless-login",
    "tasks": [
      { "id": "T1", "status": "queued",
        "requirements": ["R-AUTH-001", "R-AUTH-002", "R-AUTH-003"] },
      { "id": "T2", "status": "queued", "requirements": ["R-AUTH-004"] } ] } }
```

Had the worktree been dirty, no run state would exist at all:

```json
{ "ok": false, "error": { "code": "dirty_worktree",
    "message": "run start refused: 2 uncommitted files",
    "details": ["src/auth/session.ts", "package-lock.json"] } }
```

#### 3.2 The loop: task T1, round 1

From here the skill is a thin cycle: call `run next`, do what the directive says (`packet_with` → action → `record_with`), ask again.

```console
$ speccy ctl run next --run run_01j1bxgvk3tf4qs6mv9zpxwe8d --agent claude:sess_8842 --json
```
```json
{ "ok": true, "data": {
    "run_state": "implementing",
    "action": "claim_task",
    "subject": { "task": "T1" },
    "round": null,
    "packet_with": null,
    "record_with": "task claim",
    "reason": "T1 is the first queued task; worktree clean at f3d9e21",
    "applied_transitions": [],
    "lease": { "token": "lease_01j1c0k8", "agent": "claude:sess_8842",
               "expires_at": "2026-07-02T14:07:00Z" } } }
```

A second session asking for the same run stops immediately:

```json
{ "ok": false, "error": { "code": "lease_held",
    "message": "run lease held by claude:sess_8842 until 2026-07-02T14:07:00Z" } }
```

Claim the task (baseline pinned for every later diff and resume):

```console
$ speccy ctl task claim --run run_01j1... --task T1 --agent claude:sess_8842 --lease lease_01j1c0k8 --json
```
```json
{ "ok": true, "data": { "task": "T1", "status": "building",
    "round": 1, "baseline_commit": "f3d9e21" } }
```

Next directive — `packet_with` tells the skill which packet to build first:

```json
{ "ok": true, "data": {
    "run_state": "implementing",
    "action": "dispatch_worker",
    "subject": { "task": "T1" },
    "round": { "current": 1, "max": 3, "scope": "task" },
    "packet_with": "packet task",
    "record_with": "task record-handoff",
    "reason": "T1 is building with no recorded handoff for round 1" } }
```

```console
$ speccy ctl packet task --run run_01j1... --task T1 --json
```
```json
{ "ok": true, "data": {
    "task": "T1", "round": 1, "baseline_commit": "f3d9e21",
    "requirements": [
      { "id": "R-AUTH-001", "statement": "A user can request a magic link by email.",
        "evidence": [ { "id": "E1", "kind": "command", "command": "npm test -- auth/magic-link" } ] },
      { "id": "R-AUTH-002", "statement": "A magic-link token is single-use.",
        "evidence": [ { "id": "E1", "kind": "browser" } ] },
      { "id": "R-AUTH-003", "statement": "Tokens expire after 15 minutes.",
        "evidence": [ { "id": "E1", "kind": "command", "command": "npm test -- auth/expiry" } ] } ],
    "constraints": ["store tokens hashed (dec_20260401_003)", "no schema migrations"],
    "prior_findings": [],
    "handoff_contract": { "record_with": "task record-handoff" } } }
```

The skill spawns a fresh **worker subagent** (`speccy-worker` from the pack) with this packet. The worker implements the token model, endpoints, and tests, then returns its report.

The worker's first report omits a required field. Record operations are schema-validated with bounded repair (`structured_output_retries: 3`, then the run fails closed to `escalated`):

```console
$ speccy ctl task record-handoff --run run_01j1... --lease lease_01j1c0k8 --input handoff.json --json
```
```json
{ "ok": false, "error": { "code": "validation_failed",
    "message": "handoff.json failed schema validation",
    "details": [ { "code": "missing_field", "path": "files_touched",
                   "message": "files_touched is required" } ] } }
```

The skill retries with the field filled (attempt 2 of 3):

```json
{ "ok": true, "data": { "task": "T1", "status": "in_review",
    "handoff_id": "ho_9bc2" } }
```

`task record-handoff` records T1 straight to `in_review` — there is no separate reviewable holding state. The verifier dispatches next.

#### 3.3 Verification, round 1

```json
{ "ok": true, "data": {
    "run_state": "implementing",
    "action": "dispatch_verifier",
    "subject": { "task": "T1",
                 "personas": ["spec-fidelity", "defects", "security", "style"] },
    "round": { "current": 1, "max": 3, "scope": "task" },
    "packet_with": "packet verification",
    "record_with": "requirement set-status",
    "reason": "handoff ho_9bc2 recorded; T1 in_review, verification required",
    "applied_transitions": [] } }
```

`dispatch_verifier` with `round.scope: task` is a task review round; `round.scope: run` would be the run-gate review. `subject.personas` is the roster from `project.yaml` with tier scaling applied: this spec is `high`, so the full roster runs (a `minimal`-risk spec would collapse to one combined reviewer). `task record-handoff` already set T1 to `in_review`, so this call applies nothing.

```console
$ speccy ctl packet verification --run run_01j1... --requirements R-AUTH-001,R-AUTH-002,R-AUTH-003 --json
```
```json
{ "ok": true, "data": {
    "scope": { "task": "T1",
               "requirements": ["R-AUTH-001", "R-AUTH-002", "R-AUTH-003"] },
    "round": 1, "handoff": "ho_9bc2",
    "personas": ["spec-fidelity", "defects", "security", "style"],
    "diff": { "baseline": "f3d9e21", "files": 7, "insertions": 348, "deletions": 12 },
    "prior_findings": [],
    "provenance_scan": { "hits": 1, "findings": ["fd_77e0"] },
    "tools": ["evidence collect", "evidence record", "finding record"] } }
```

The controller already ran the deterministic provenance scan over the task diff while assembling the packet; the worker left a process-language comment, and the deny-list caught the requirement ID:

```json
{ "id": "fd_77e0", "task": "T1", "persona": null,
  "severity": "blocking", "recorded_by": "controller:provenance-scan",
  "note": "src/server/auth/magic-link.ts:41 comment references \"R-AUTH-003\" — provenance deny-list hit" }
```

The skill fans out the four fresh **reviewer personas** named in `subject.personas`. They call the lease-free evidence ops concurrently.

Command evidence is executed by the controller itself — the persona only chooses when:

```console
$ speccy ctl evidence collect --run run_01j1... --requirements R-AUTH-001,R-AUTH-003 --json
```
```json
{ "ok": true, "data": { "evidence": [
    { "id": "ev_12a4", "requirement": "R-AUTH-001", "request": "E1",
      "kind": "command",
      "command": "npm test -- auth/magic-link", "exit_code": 0,
      "stdout_hash": "sha256:8c1e…", "artifact": "evidence/ev_12a4.txt",
      "collected_by": "controller" },
    { "id": "ev_12a5", "requirement": "R-AUTH-003", "request": "E1",
      "kind": "command",
      "command": "npm test -- auth/expiry", "exit_code": 1,
      "stdout_hash": "sha256:2b77…", "artifact": "evidence/ev_12a5.txt",
      "collected_by": "controller" } ] } }
```

Command executions serialize on the workspace command lock, so two personas collecting command evidence never interleave test runs. To re-prove a single artifact without re-collecting a whole requirement's evidence, a persona can target one request: `speccy ctl evidence collect --run run_01j1... --requests R-AUTH-003.E1 --json`.

Pasting command output through `evidence record` is refused:

```json
{ "ok": false, "error": { "code": "validation_failed",
    "message": "evidence record refuses agent-supplied output for kind: command; use evidence collect" } }
```

Browser evidence is agent-collected, and this spec is `high`, so a prose-only record is refused — the persona must reference a stored artifact (screenshot, trace, DOM capture):

```console
$ speccy ctl evidence record --run run_01j1... --input evidence.json --json
```
```json
{ "ok": false, "error": { "code": "validation_failed",
    "message": "kind: browser requires an artifact reference at risk high; store a screenshot, trace, or DOM capture" } }
```
```json
{ "ok": true, "data": { "id": "ev_12a6", "requirement": "R-AUTH-002",
    "request": "E1", "kind": "browser",
    "collected_by": "claude:reviewer_spec-fidelity_T1",
    "artifact": "evidence/ev_12a6/replay.png",
    "note": "second open of same link → 'link already used'; no session cookie set" } }
```

At `minimal` and `standard` the artifact stays optional and the first record would have been accepted.

The `defects` persona records the blocking finding on the failed expiry test:

```console
$ speccy ctl finding record --run run_01j1... --input finding.json --json
```
```json
{ "ok": true, "data": { "id": "fd_77e1", "requirement": "R-AUTH-003",
    "persona": "defects", "severity": "blocking",
    "note": "token accepted at 16m; expiry window compared in ms vs seconds" } }
```

The `security` and `style` personas come back clean (the `style` persona's checklist includes semantic provenance leakage; it confirms the scan's hit and finds no other process-language comments). The orchestrator (lease holder) aggregates:

```console
$ speccy ctl requirement set-status --run run_01j1... --lease lease_01j1c0k8 --input status.json --json
```
```json
{ "ok": true, "data": { "updated": [
    { "requirement": "R-AUTH-001", "status": "passed", "evidence": ["ev_12a4"] },
    { "requirement": "R-AUTH-002", "status": "review_passed", "evidence": ["ev_12a6"],
      "residual_risk": "single-use proven in browser only; no unit test" },
    { "requirement": "R-AUTH-003", "status": "failed",
      "evidence": ["ev_12a5"], "findings": ["fd_77e1"] } ] } }
```

`passed` without recorded evidence is refused:

```json
{ "ok": false, "error": { "code": "validation_failed",
    "message": "passed requires at least one recorded evidence artifact for R-AUTH-003" } }
```

#### 3.4 Repair round

```json
{ "ok": true, "data": {
    "run_state": "implementing",
    "action": "dispatch_worker",
    "subject": { "task": "T1", "requirements": ["R-AUTH-003"] },
    "round": { "current": 2, "max": 3, "scope": "task" },
    "packet_with": "packet task",
    "record_with": "task record-handoff",
    "reason": "R-AUTH-003 failed and blocking finding fd_77e0 unresolved after round 1; task repair cap not exhausted",
    "applied_transitions": [
      { "subject": "task:T1", "from": "in_review", "to": "building" } ] } }
```

A repair round is just `dispatch_worker` with `round.current > 1`. The controller counted the round and moved T1 `in_review → building` itself; the skill only reports "starting repair round 2 of 3". Round 2's `packet task` carries `"prior_findings": [{"id": "fd_77e0", ...}, {"id": "fd_77e1", ...}]`, so the repair worker starts from the ms/seconds diagnosis and the flagged comment instead of rediscovering them. The worker fixes the comparison and deletes the `R-AUTH-003` comment.

Round 2's verification packet — the full roster re-reviews the full diff:

```json
{ "ok": true, "data": {
    "scope": { "task": "T1",
               "requirements": ["R-AUTH-001", "R-AUTH-002", "R-AUTH-003"] },
    "round": 2, "handoff": "ho_a103",
    "personas": ["spec-fidelity", "defects", "security", "style"],
    "diff": { "baseline": "f3d9e21", "files": 7, "insertions": 351, "deletions": 15 },
    "prior_findings": [
      { "id": "fd_77e0", "severity": "blocking", "resolution_claim": "comment removed" },
      { "id": "fd_77e1", "severity": "blocking", "resolution_claim": "expiry compared in seconds" } ],
    "provenance_scan": { "hits": 0, "findings": [] },
    "tools": ["evidence collect", "evidence record", "finding record"] } }
```

Every persona re-runs — skipping clean personas is rejected by design, because a repair diff is new code — and each verifies its own prior blockers against the full task diff, guided by the carried-forward `prior_findings` rather than rediscovering them. `evidence collect` re-runs `npm test -- auth/expiry` → `exit_code: 0`, and R-AUTH-003 is set `passed`.

Calling `run next` twice without recording anything returns the same directive — identical apart from lease renewal metadata and an emptied `applied_transitions` — and that idempotency is the whole crash-recovery story (Appendix B).

#### 3.5 T1 integrates, T2 runs

```json
{ "ok": true, "data": {
    "run_state": "implementing",
    "action": "claim_task",
    "subject": { "task": "T2" },
    "packet_with": null,
    "record_with": "task claim",
    "reason": "T1 integrated at snapshot 9c2f1ab; T2 is the next queued task",
    "applied_transitions": [
      { "subject": "task:T1", "from": "in_review", "to": "integrated",
        "snapshot": "9c2f1ab" } ] } }
```

T1's `in_review → integrated` transition and its snapshot commit — `speccy: SPEC-20260630-A7F4 T1 integrated (round 2)`, committed as `Speccy <noreply@speccy.local>` — happened inside `run next` when every linked requirement resolved and no blocking finding remained. T2 (expired-link UI) runs the same claim → dispatch → handoff → verify cycle and passes in one round; R-AUTH-004 is `passed` on both of its declared requests — browser evidence (`E1`) plus the UI test the controller executed (`E2`). Snapshot `c4d81e0`.

#### 3.6 Final validation and the verified gate

```json
{ "ok": true, "data": {
    "run_state": "verifying",
    "action": "dispatch_verifier",
    "subject": { "requirements": ["R-AUTH-001", "R-AUTH-002",
                                   "R-AUTH-003", "R-AUTH-004"],
                 "personas": ["spec-fidelity", "defects", "security", "style"] },
    "round": { "current": 1, "max": 3, "scope": "run" },
    "packet_with": "packet verification",
    "record_with": "requirement set-status",
    "reason": "all tasks integrated; run-level integration and drift review required",
    "applied_transitions": [
      { "subject": "task:T2", "from": "in_review", "to": "integrated",
        "snapshot": "c4d81e0" },
      { "subject": "run", "from": "implementing", "to": "verifying" } ] } }
```

The same roster fans out over the integrated whole-run diff (the `run` scope of `dispatch_verifier`) while a final fresh verifier reads the run-scoped verification packet, re-runs the full suite through `evidence collect`, reviews cross-task drift against the approved revision, and confirms the statuses. The controller's provenance scan of the integrated diff comes back clean. Had anything failed here, the controller would have appended a run-level repair task `RT1` counted against `run_review_rounds` — same claim → dispatch → handoff → verify cycle. Nothing did:

```json
{ "ok": true, "data": {
    "run_state": "verified",
    "action": "await_human_gate",
    "subject": { "gate": "ship_decision" },
    "packet_with": "packet review",
    "record_with": "run record-ship",
    "gate_answers": [
      { "type": "ship",   "record_with": "run record-ship" },
      { "type": "rework", "record_with": "run record-decision" },
      { "type": "amend",  "record_with": "spec record-decision" },
      { "type": "cancel", "record_with": "run record-decision" } ],
    "reason": "all requirements resolved (3 passed, 1 review_passed); ship or send back",
    "applied_transitions": [
      { "subject": "run", "from": "verifying", "to": "verified" } ] } }
```

This spec is `high`, so `review_passed` required a recorded `residual_risk` note. On a `critical` spec the same directive would first park at the accepted-risk confirmation gate. `record_with` is only the gate's default recorder; `gate_answers` closes the set of legal answers and their recorders, so the skill routes the human's prose without guessing — the send-back in Appendix C is the `rework` entry.

The skill builds the review packet and presents it (the human view above). The autonomous session is done.

```console
$ speccy ctl packet review --run run_01j1... --json
```

The rendered packet is the `data.markdown` field of the envelope — the "Result verified" screen shown at the top of this section.

---

## 4. Ship — open the PR  ·  CHECKPOINT: merge acknowledgement

### What the human sees

```text
/speccy-ship

Shipping with 1 accepted risk:
  R-AUTH-002  review-only evidence  Single-use proven in browser only; no unit test

Open the PR anyway?

Human: yes

This branch has 3 speccy-labeled snapshot commits.
Squash them into one commit before opening the PR? (recommended)

Human: yes
https://github.com/acme/acme-app/pull/123
After it merges, record it with: speccy accept
```

The team reviews and merges PR #123 normally — the review packet is its description.

### Behind the checkpoint

Any session, possibly days later. The ship skill re-enters through the controller like every other transition. Its `run next` call clears the old session's expired lease, issues a fresh one, and returns the same ship gate — invoking `/speccy-ship` is the human's answer to it:

```console
$ speccy ctl run next --run run_01j1... --agent claude:sess_9105 --json
```
```json
{ "ok": true, "data": {
    "run_state": "verified",
    "action": "await_human_gate",
    "subject": { "gate": "ship_decision" },
    "packet_with": "packet review",
    "record_with": "run record-ship",
    "gate_answers": [
      { "type": "ship",   "record_with": "run record-ship" },
      { "type": "rework", "record_with": "run record-decision" },
      { "type": "amend",  "record_with": "spec record-decision" },
      { "type": "cancel", "record_with": "run record-decision" } ],
    "reason": "all requirements resolved; ship or send back",
    "lease": { "token": "lease_01j1f2mq", "agent": "claude:sess_9105",
               "expires_at": "2026-07-02T18:31:00Z" } } }
```

The accepted-risk bucket is non-empty, so the skill echoes it and asks one explicit confirmation before creating anything external — the packet was presented days ago in another session, and a PR must never open blind to residual risk. With an empty bucket there is no confirmation; the ship proceeds directly. The ship prose then offers a squash by default, so Speccy-labeled snapshot messages stay off the mainline (see "Provenance Hygiene" in `DESIGN.md`).

Human confirms; the skill squashes harness-side, then opens the PR. PR opening is harness-side prose too; Speccy makes no outbound calls:

```console
$ gh pr create --title "Passwordless login" \
    --body-file review-packet.md --base main \
    --head speccy/spec-20260630-a7f4-passwordless-login
https://github.com/acme/acme-app/pull/123
```

The skill records the result:

```console
$ speccy ctl run record-ship --run run_01j1... --lease lease_01j1f2mq --input change-ref.json --json
```
```json
{ "ok": true, "data": { "run_state": "submitted",
    "change_ref": { "kind": "pull_request",
      "url": "https://github.com/acme/acme-app/pull/123",
      "branch": "speccy/spec-20260630-a7f4-passwordless-login",
      "head_sha": "a7f4c2e", "base": "main" } } }
```

Any later `run next` before the merge is recorded returns `halt` — no autonomous action exists:

```json
{ "ok": true, "data": {
    "run_state": "submitted",
    "action": "halt",
    "subject": null, "round": null,
    "packet_with": null, "record_with": null,
    "reason": "submitted awaiting external merge; record it with speccy accept" } }
```

The skill closes with the boundary statement:

```text
PR merge is the source of truth.
After it merges, record it with: speccy accept
```

GitHub is not required. On a repo with no PR flow the same gate records a different `change_ref`: the skill skips `gh pr create` and records `kind: branch` (no `url`; hand `speccy/spec-20260630-a7f4-passwordless-login` to whatever review flow exists), `kind: patch`, or `kind: none` for a local-only change. `speccy accept` records the landing the same way from the stored `change_ref`.

---

## 5. Accept — record the merge

### What the human sees

The team merges PR #123 on GitHub. Speccy does no merge detection in MVP — the human records it. The PR may have been merged by a teammate, squash-merged, or merged after the local branch was deleted; `speccy accept` uses the `change_ref` saved at ship time and the human assertion that it landed:

```console
$ speccy accept
Recording landing for:
  PR #123  https://github.com/acme/acme-app/pull/123
  branch  speccy/spec-20260630-a7f4-passwordless-login

Recorded: SPEC-20260630-A7F4  Passwordless login
  run  submitted -> landed
  spec approved  -> accepted
Accepted specs leave default status/list output. Show them with:
  speccy list --accepted
```

That is the whole surface: approve a card, come back to a review packet, ship, record the merge. Everything else is the harness and controller's problem. Archive later only when a historical accepted spec no longer describes the codebase.

### Behind the checkpoint

Human commands return human-readable text, not JSON. The accepted spec is hidden from default active views, but remains inspectable:

```console
$ speccy status
No active runs.

$ speccy list --accepted
1 accepted spec:
  SPEC-20260630-A7F4  Passwordless login   landed 2026-07-02 (PR #123)
```

Months later, once the magic-link flow has been reworked and the spec no longer describes the code:

```console
$ speccy archive SPEC-20260630-A7F4
Archived SPEC-20260630-A7F4. It leaves accepted-spec lists; its carry-forward
decisions stay recorded for a future planning decision index.

$ speccy list
No active specs.

$ speccy list --archived
1  SPEC-20260630-A7F4  Passwordless login   archived (landed via PR #123)
```

Archiving is a spec visibility action; the run stays `landed` in run history under `~/.speccy/workspaces/ws_a81f23/`. Archiving removes the spec's decisions from planning context in MVP; the `carry_forward` flag is recorded so a future decision index can surface them across archived specs (see "Carry-Forward Decisions" in `DESIGN.md`).

---

## Appendix A — The branch not taken: escalation

If round 3 had also failed R-AUTH-003:

```json
{ "ok": true, "data": {
    "run_state": "escalated",
    "action": "await_human_gate",
    "subject": { "gate": "escalation", "requirements": ["R-AUTH-003"] },
    "round": { "current": 3, "max": 3, "scope": "task" },
    "packet_with": "packet escalation",
    "record_with": "run record-decision",
    "gate_answers": [
      { "type": "amend",         "record_with": "spec record-decision" },
      { "type": "provide_setup", "record_with": "run record-decision" },
      { "type": "waive",         "record_with": "run record-decision" },
      { "type": "cancel",        "record_with": "run record-decision" } ],
    "reason": "task repair cap exhausted with R-AUTH-003 still failed",
    "applied_transitions": [
      { "subject": "run", "from": "implementing", "to": "escalated",
        "snapshot": "4e8d0aa" } ] } }
```

```console
$ speccy ctl packet escalation --run run_01j1... --json
```

The human sees one question, not a stack trace:

```text
Speccy stopped because R-AUTH-003 could not be proven.

Tried:
  round 1 — expiry compared in ms vs seconds     (rejected: 16m token still accepted)
  round 2 — clamped window at token creation      (rejected: replay path bypasses check)
  round 3 — moved check into token verification   (rejected: flaky under CI clock skew)

Partial work applied: escalation snapshot 4e8d0aa on speccy/spec-20260630-a7f4-passwordless-login.

Recommended: amend the spec
Alternatives: provide setup, waive this requirement, cancel the run

Should expiry be enforced at verification time with a tolerance,
or is the 15-minute window itself the wrong requirement?
```

The human answers in prose. A waiver stays on the same run:

```console
$ speccy ctl run record-decision --run run_01j1... --lease lease_01j1c0k8 --input decision.json --json
```
```json
{ "ok": true, "data": { "decision_id": "dec_20260702_004", "type": "waiver",
    "requirement": "R-AUTH-003", "requirement_status": "waived",
    "run_state": "implementing", "resume": "call run next" } }
```

An amendment instead goes back through `spec patch-draft` → amended spec card → `spec record-decision` (`supersedes.run_id`), which atomically closes this run as `cancelled` with a linking decision record and starts a new run on the same branch, seeded with the escalation snapshot. Until that approval lands nothing is recorded and the run stays parked at this gate.

## Appendix B — The branch not taken: crash resume

Kill the session mid-round-2 of T1 (worker had edited files, handoff never recorded). Before re-entering, `speccy status` already shows what resume will do with the dirty worktree:

```console
$ speccy status
SPEC-20260630-A7F4  Passwordless login          Risk: high
  Interrupted — session died mid "token model + endpoints" (repair round 2)
  Uncommitted diff (3 files, +58 −4 vs f3d9e21) belongs to that task on resume
  Next: /speccy-implement
        (stash or commit first if these edits are not the worker's)
```

The card names the task by title — task IDs like `T1` stay in the ctl JSON below, never on status cards.

The edits are the worker's, so resume directly. Later, in a brand-new session, `/speccy-implement`:

```console
$ speccy ctl run next --run run_01j1... --agent claude:sess_9330 --json
```
```json
{ "ok": true, "data": {
    "run_state": "implementing",
    "action": "dispatch_worker",
    "subject": { "task": "T1" },
    "round": { "current": 2, "max": 3, "scope": "task" },
    "packet_with": "packet task",
    "record_with": "task record-handoff",
    "reason": "T1 building in round 2 with no recorded handoff; dirty worktree diff included as context",
    "resume": { "cleared_lease": "claude:sess_8842",
                "dirty_diff": { "files": 3, "insertions": 58, "deletions": 4,
                                "vs": "f3d9e21", "attributed_to": "T1" } },
    "lease": { "token": "lease_01j1g8xw", "agent": "claude:sess_9330",
               "expires_at": "2026-07-02T15:02:00Z" } } }
```

The skill echoes the `resume` block before dispatching — the same pattern as the approval echo, so attribution is never silent:

```text
Resuming: cleared expired lease (claude:sess_8842).
Uncommitted diff (3 files, +58 −4 vs f3d9e21) attributed to T1.
```

Nothing replays. The dead session's lease was cleared, the round counter and task status say exactly where the loop stopped, and the uncommitted diff belongs to T1 by the resume invariant. The controller cannot tell the worker's partial diff from edits made while the session was dead, so it reports the attribution instead of gating on it; a human who did edit stashes or commits first (a commit parks the run at the out-of-band-commit gate). There is no `speccy resume` command — this is it.

## Appendix C — The branch not taken: send it back

Had the human read the review packet (§3) and replied `the expired-link page should reuse the standard error layout — otherwise good`, the skill would classify the feedback (no requirement, scope, or risk change → same run, not an amendment), echo the decision, and record it:

```text
Recording rework: SPEC-20260630-A7F4 run -> implementing
Feedback: "the expired-link page should reuse the standard error layout"
```

```console
$ speccy ctl run record-decision --run run_01j1... --lease lease_01j1c0k8 --input decision.json --json
```
```json
{ "ok": true, "data": { "decision_id": "dec_20260702_005", "type": "rework",
    "run_state": "implementing", "task_appended": "RT1",
    "round": { "current": 2, "max": 3, "scope": "run" },
    "resume": "call run next" } }
```

`RT1` is seeded with the feedback prose, counted against `run_review_rounds`, and runs the normal claim → dispatch → handoff → verify cycle back to the same ship gate. Feedback that changes scope or requirements goes through the amendment path instead — `spec patch-draft` → amended card → prose approval, closing this run as `cancelled` with a linking decision record (same mechanics as the escalation amendment in Appendix A).

## Appendix D — Human CLI odds and ends

Commands not exercised by the happy path above:

```console
$ speccy new "Rate-limit magic link requests"
Created draft spec SPEC-20260702-D3E8 "Rate-limit magic link requests".
Next: open your harness and run /speccy-plan SPEC-20260702-D3E8

$ speccy list --query magic
Active specs matching "magic":

1  SPEC-20260702-D3E8  Rate-limit magic link requests   draft

Use: speccy review SPEC-20260702-D3E8

$ speccy review
(prints the current human packet for the spec's state: spec card, status,
 review packet, escalation packet, submitted close-out card, or accepted summary;
 add --evidence to drill into the ledger, command logs, artifacts, findings,
 decisions, and full diff when available)

$ speccy export review SPEC-20260630-A7F4 --dest docs/specs/SPEC-20260630-A7F4/
Wrote docs/specs/SPEC-20260630-A7F4/review-packet.md
(explicit export destinations are exempt from provenance scanning)

$ speccy cancel SPEC-20260702-D3E8
Cancelled SPEC-20260702-D3E8 (draft, no runs). Recorded as spec decision.
```

`speccy new` records intent from outside a harness; it never drafts the spec or launches anything. `speccy cancel` on a spec with an active run cancels the run first (any active state → `cancelled`).

## Appendix E — Coverage

All 21 controller operations and the human CLI, by first appearance:

| Operation | Section |
| --- | --- |
| `spec start` / `spec status` | 2.2 / 3.1 |
| `spec record-draft` / `spec patch-draft` / `spec record-decision` | 2.3 / 2.3 / 2.4 |
| `run start` / `run status` / `run next` | 3.1 / (debugging any time) / 3.2 |
| `run record-decision` / `run record-ship` | Appendix C (rework), Appendix A (waiver) / §4 |
| `task claim` / `task record-handoff` | 3.2 / 3.2 |
| `packet planning` / `packet task` / `packet verification` / `packet review` / `packet escalation` | 2.2 / 3.2 / 3.3 / 3.6 / Appendix A |
| `evidence collect` / `evidence record` / `finding record` | 3.3 |
| `requirement set-status` | 3.3 |
| `speccy install` / `doctor` | §1 |
| `speccy accept` / `archive` / `status` / `list` | §5 |
| `speccy new` / `list --query` / `review` / `export review` / `cancel` | Appendix D |

Directive actions covered: `claim_task` (3.2), `dispatch_worker` (round 1 in 3.2, repair round in 3.4), `dispatch_verifier` (task scope in 3.3, run scope in 3.6), `await_human_gate` (ship in 3.6/§4, escalation in Appendix A), `halt` (§4).
