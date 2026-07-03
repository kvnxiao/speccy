# Speccy Controller I/O Shapes

Status: authoritative for payload shapes, provisional until M2 implements them
Date: 2026-07-03

Every `speccy ctl` operation's input payload, the response envelope, and the
`run next` directive. `DESIGN.md` owns behavior; this document owns only the
shapes. Shapes are serde-internal until dogfooding proves them (Open
Question 3 in `DESIGN.md`): they may change without a compatibility promise
before 1.0.

Conventions:

- All fields are required unless marked `optional`.
- Timestamps are RFC 3339 UTC strings.
- IDs follow the ID Scope Summary in `TERMINOLOGY.md`.
- Every `--input` flag accepts a file path or `-` for stdin.

## Envelope

Every operation returns:

```json
{ "ok": true, "data": { } }
```

```json
{ "ok": false, "error": {
    "code": "lease_held",
    "message": "run lease held by claude:sess_8842 until 2026-07-02T14:07:00Z",
    "details": [] } }
```

- `error.code` — closed vocabulary: `not_implemented`, `validation_failed`,
  `invalid_transition`, `lease_held`, `not_a_git_repo`, `dirty_worktree`,
  `not_found`, `ambiguous_selector`, `cap_exhausted`, `io_error`.
- `error.details` — optional array of structured findings (same shape as
  `lint.findings` below).

Write operations that lint return their findings inside `data`:

```json
{ "ok": true, "data": {
    "lint": { "clean": false, "findings": [
      { "code": "missing_evidence_request",
        "path": "requirements[R-AUTH-002]",
        "message": "R-AUTH-002 has no evidence request" } ] } } }
```

## Directive (`run next` output)

```json
{ "run_state": "implementing",
  "action": "dispatch_worker",
  "subject": { "task": "T1", "requirements": ["R-AUTH-003"], "gate": null },
  "round": { "current": 2, "max": 3, "scope": "task" },
  "packet_with": "packet task",
  "record_with": "task record-handoff",
  "reason": "R-AUTH-003 failed in round 1; task repair cap not exhausted",
  "applied_transitions": [
    { "subject": "task:T1", "from": "in_review", "to": "needs_repair" },
    { "subject": "task:T1", "from": "needs_repair", "to": "building" } ],
  "lease": { "token": "lease_01j1c0k8", "agent": "claude:sess_8842",
             "expires_at": "2026-07-02T14:07:00Z" } }
```

- `action` — closed vocabulary: `claim_task`, `dispatch_worker`,
  `dispatch_task_verifier`, `spawn_repair_round`, `run_final_validation`,
  `await_human_gate`, `emit_escalation_packet`, `halt`.
- `subject` — the fields relevant to the action; unused fields are null.
  `gate` values: `ship_decision`, `escalation`, `accepted_risk_confirmation`.
  For `dispatch_task_verifier` and `run_final_validation`, `subject.personas`
  carries the reviewer persona roster to fan out (from `project.yaml`, tier
  scaling applied).
- `round` — null when no round applies. `scope`: `task` | `run`.
- `packet_with` / `record_with` — controller operation names, or null.
- `applied_transitions` — the derived transitions this call applied before
  deriving the directive (see "Deterministic Loop Driving: run next" in
  `DESIGN.md`).
  Each entry is `{subject, from, to}` — `subject` is `task:<id>` or `run` —
  plus `snapshot` (commit SHA) when the transition created a snapshot commit.
  Empty array when the call applied nothing; repeated calls over settled
  state return `[]`.
- `lease` — present on every `run next` response; renewal changes only
  `expires_at`. Idempotency compares all fields except `lease` and
  `applied_transitions`, which report per-call work, not directive state.

## `spec start` — request.json

```yaml
request: "Let users log in without passwords via emailed magic links"  # verbatim user intent
source: "claude:/speccy-plan"        # optional; recording skill or `speccy new`
title: "Passwordless login"          # optional; mutable working title
brainstorm_handoff: "..."            # optional; verbatim handoff text, the only
                                     # persistence point for promoted brainstorms
```

## `spec record-draft` — spec-draft.json

One complete candidate revision. Also the shape seeded into draft N+1 when
the latest revision is approved.

```yaml
goal: "Users can sign in through single-use emailed magic links"
scope:
  in: ["request link by email", "token expiry and replay protection"]
  out: ["OAuth", "email vendor migration"]
risk: standard                        # minimal | standard | high | critical
assumptions: []                       # optional, strings
non_goals: []                         # optional, strings
observations: []                      # optional; intake observations, strings
open_questions: []                    # optional, strings
requirements:
  - id: R-AUTH-001                    # unique within the spec
    statement: "A user can request a magic link by email."
    scenario:                         # optional given/when/then clarification
      given: "..."
      when: "..."
      then: "..."
    evidence:                         # one or more evidence requests
      kind: command                   # command | review | browser | api | manual
      command: "npm test -- auth/magic-link"   # required for kind: command
      note: ""                        # optional; expectations for the collector
tasks:
  - id: T1                            # unique within the spec
    title: "Token model, expiry, request/consume endpoints"
    requirements: ["R-AUTH-001"]      # every requirement covered by ≥1 task
    constraints: []                   # optional, strings
```

Structural lint (returned on `record-draft`/`patch-draft`, approval refused
while dirty): missing required sections, invalid risk tier, duplicate
requirement/task IDs, requirement without an evidence request, task
referencing an unknown requirement, requirement not covered by any task.

## `spec patch-draft` — spec-patch.json

Replaces named top-level sections of the current draft; unnamed sections are
untouched.

```yaml
set:
  risk: high
  requirements: [ ... ]               # full replacement of the section
```

## `spec record-decision` — decision.json (spec-scoped)

```yaml
type: approve                         # approve | reject | split | scope_change | cancel
revision: spec_rev_001-draft          # the draft being decided
actor: human
approved_in_prose: "looks good, go"   # verbatim human reply, required for approve
note: ""                              # optional context
supersedes:                           # optional; links for split/amendment
  spec_ref: SPEC-20260630-A7F4
  run_id: run_01j1bxgvk3tf4qs6mv9zpxwe8d
```

## `run record-decision` — decision.json (run-scoped, gate answers)

```yaml
type: waive                           # waive | defer | provide_setup | confirm_accepted_risk | cancel
requirement: R-AUTH-003               # required for waive; the status flips to
                                      # waived atomically inside this operation
task: T3                              # required for defer
actor: human
reason: "constant-time comparison deferred; tracked in follow-up"
residual_risk: "timing side channel remains measurable"   # required for waive
```

## `task claim`

No `--input`; arguments only: `--run <id> --task <id> --agent <id> --lease <token>`.

## `task record-handoff` — handoff.json

```yaml
task: T1
round: 1
summary: "Added token model, request/consume endpoints, unit tests."
files_touched: ["src/server/auth/magic-link.ts"]
commands_run:
  - command: "npm test -- auth/magic-link"
    exit_code: 0
requirements_claimed: ["R-AUTH-001", "R-AUTH-003"]
known_issues: []                      # optional, strings
deviations: []                        # optional, strings
follow_ups: []                        # optional, strings
```

## `evidence collect`

No `--input`; arguments only: `--run <id> --requirements R1,R2`. The
controller reads each requirement's declared `kind: command` evidence request
and executes it itself (policy in "Requirement Resolution Rules" and the
command-execution bullet of "Acceptance Ledger" in `DESIGN.md`).

## `evidence record` — evidence.json

```yaml
requirement: R-AUTH-002
kind: browser                         # review | browser | api | manual — command is refused
collected_by: "claude:verifier_T1"
note: "second open of same link → 'link already used'; no session cookie set"
artifact: ""                          # optional; path/reference to a stored artifact
```

## `finding record` — finding.json

```yaml
requirement: R-AUTH-003               # optional; findings may be run-scoped
task: T1                              # optional
persona: defects                      # optional; the reviewer persona that produced it
severity: blocking                    # blocking | advisory | positive | uncertain
note: "token accepted at 16m; expiry window compared in ms vs seconds"
recorded_by: "claude:verifier_T1"
```

## `requirement set-status` — status.json

```yaml
updates:
  - requirement: R-AUTH-001
    status: passed                    # per the transition matrix; waived is refused here
    evidence: ["ev_12a4"]             # required for passed/failed/vacuous/review_passed
    findings: ["fd_77e1"]             # optional
    residual_risk: ""                 # required for review_passed at high/critical
    note: ""                          # required for blocked/unproven
```

## `run record-ship` — change-ref.json

```yaml
kind: pull_request                    # pull_request | branch | patch | none
url: "https://github.com/acme/acme-app/pull/123"   # optional for branch/patch
branch: speccy/spec-20260630-a7f4-passwordless-login
head_sha: a7f4c2e
base: main
```

## Packets

Packet operations take no `--input`. Their `data` payloads are
controller-assembled work orders; representative examples live in
`WALKTHROUGH.md` (planning 2.2, task 3.2, verification 3.3). `packet review`
and `packet escalation` carry the rendered human-facing text in a
`data.markdown` field alongside the structured fields.

Verification packets name the persona roster to fan out and, from round 2
on, carry `delta` — the diff since the last reviewed round snapshot —
alongside the full diff reference and `prior_findings` (mechanics in "Repeat
Review Rounds and Token Scoping" in `DESIGN.md`).
