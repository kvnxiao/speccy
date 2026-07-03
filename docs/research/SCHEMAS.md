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
    { "subject": "task:T1", "from": "in_review", "to": "building" } ],
  "gate_answers": null,
  "resume": null,
  "lease": { "token": "lease_01j1c0k8", "agent": "claude:sess_8842",
             "expires_at": "2026-07-02T14:07:00Z" } }
```

- `action` — closed vocabulary: `claim_task`, `dispatch_worker`,
  `dispatch_verifier`, `await_human_gate`, `halt`. A repair round is
  `dispatch_worker` with `round.current > 1`; run-level validation is
  `dispatch_verifier` with `round.scope: run`; an escalation is
  `await_human_gate` with `subject.gate: escalation`.
- `subject` — the fields relevant to the action; unused fields are null.
  `gate` values: `ship_decision`, `escalation`, `accepted_risk_confirmation`.
  For `dispatch_verifier`, `subject.personas` carries the reviewer persona
  roster to fan out (from `project.yaml`, tier scaling applied).
- `round` — null when no round applies. `scope`: `task` | `run`.
- `packet_with` / `record_with` — controller operation names, or null. At a
  human decision point (`await_human_gate`), `record_with` is only the gate's
  default recorder; `gate_answers` is the authoritative per-answer map.
- `gate_answers` — present only on `await_human_gate` directives, null
  otherwise: the closed list of decisions legal at this gate, each
  `{type, record_with}`. Example for the ship gate:

  ```json
  "gate_answers": [
    { "type": "ship",   "record_with": "run record-ship" },
    { "type": "rework", "record_with": "run record-decision" },
    { "type": "amend",  "record_with": "spec record-decision" },
    { "type": "cancel", "record_with": "run record-decision" } ]
  ```

  `amend` is a deferred answer: the run stays parked at its gate while
  `spec patch-draft` (the working step, which records nothing) drafts the
  amendment; the answer is recorded when the superseding approval lands —
  `spec record-decision` with `supersedes.run_id`, which atomically cancels
  this run (see "Amendment at the Escalation Gate" in `DESIGN.md`).
- `applied_transitions` — the derived transitions this call applied before
  deriving the directive (see "Deterministic Loop Driving: run next" in
  `DESIGN.md`).
  Each entry is `{subject, from, to}` — `subject` is `task:<id>` or `run` —
  plus `snapshot` (commit SHA) when the transition created a snapshot commit.
  Empty array when the call applied nothing; repeated calls over settled
  state return `[]`.
- `resume` — null on ordinary calls. When this call cleared an expired lease
  it reports the repair (see "Resume and Crash Recovery" in `DESIGN.md`):

  ```json
  "resume": { "cleared_lease": "claude:sess_8842",
              "dirty_diff": { "files": 3, "insertions": 58, "deletions": 4,
                              "vs": "f3d9e21", "attributed_to": "T1" } }
  ```

  `dirty_diff` is null when the worktree is clean; otherwise it summarizes
  the uncommitted diff against the in-flight task's `baseline_commit` that
  resume attribution will fold into that task.
- `lease` — present on every `run next` response; renewal changes only
  `expires_at`. Idempotency compares all fields except `lease`,
  `applied_transitions`, and `resume`, which report per-call work, not
  directive state.

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
    evidence:                         # one or more evidence requests, as an array
      - id: E1                        # stable, unique within the requirement;
                                      # qualified form R-AUTH-001.E1
        kind: command                 # command | review | browser | api | manual
        command: "npm test -- auth/magic-link"   # required for kind: command
        note: ""                      # optional; expectations for the collector
tasks:
  - id: T1                            # unique within the spec
    title: "Token model, expiry, request/consume endpoints"
    requirements: ["R-AUTH-001"]      # every requirement covered by ≥1 task
    constraints: []                   # optional, strings
```

Structural lint (returned on `record-draft`/`patch-draft`, approval refused
while dirty): missing required sections, invalid risk tier, duplicate
requirement/task IDs, requirement without an evidence request, task
referencing an unknown requirement, requirement not covered by any task,
duplicate evidence-request IDs within a requirement,
`kind: command` string matching no `evidence.command_policy.allow` pattern
(only when that policy is configured).

## `spec patch-draft` — spec-patch.json

Replaces named top-level sections of the current draft; unnamed sections are
untouched.

```yaml
set:
  risk: high
  requirements: [ ... ]               # full replacement of the section
```

Every `record-draft`/`patch-draft` response carries the draft's lint findings.
A draft revision is mutable in place; the controller tracks no per-write
version, and the approval echo is the binding guard (see "Spec Card UX" in
`DESIGN.md`).

## `spec record-decision` — decision.json (spec-scoped)

```yaml
type: approve                         # approve | reject | split | scope_change | cancel
revision: spec_rev_001-draft          # the draft being decided
actor: human
approved_in_prose: "go"               # verbatim human reply, required for approve
note: ""                              # optional context
carry_forward: false                  # optional; true marks a durable constraint
                                      # for a future planning decision index
supersedes:                           # optional; links for split/amendment
  spec_ref: SPEC-20260630-A7F4
  run_id: run_01j1bxgvk3tf4qs6mv9zpxwe8d
```

`supersedes.run_id` on an `approve` makes it a superseding approval: inside
the same operation the controller closes that run as `cancelled` and writes
its linking run-scoped decision record. This is how an `amend` gate answer is
recorded (see "Amendment at the Escalation Gate" in `DESIGN.md`).

## `run record-decision` — decision.json (run-scoped, gate answers)

```yaml
type: waive                           # waive | defer | provide_setup | confirm_accepted_risk | rework | cancel
requirement: R-AUTH-003               # required for waive; the status flips to
                                      # waived atomically inside this operation
task: T3                              # required for defer
actor: human
reason: "constant-time comparison deferred; tracked in follow-up"
residual_risk: "timing side channel remains measurable"   # required for waive
carry_forward: false                  # optional; true marks a durable constraint
                                      # for a future planning decision index
```

`rework` requires `reason` (the human's verbatim feedback): the run moves
`verified -> implementing` and the controller appends a dynamic `RT<n>` task
seeded with that feedback, counted against `run_review_rounds` (mechanics in
"Review UX" in `DESIGN.md`).

`defer` requires `task`. Requirements linked only to that task are waived
atomically inside the decision, so `reason` is always required and
`residual_risk` is required whenever the defer waives any requirement
(mechanics in "Requirement Resolution Rules" in `DESIGN.md`).

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

No `--input`; arguments only: `--run <id> --requirements R1,R2
[--requests R-AUTH-001.E1,R-AUTH-003.E1]`. The controller reads each named
requirement's declared `kind: command` evidence requests and executes each of
them itself, recording one evidence artifact per request tagged with the
request ID (policy in "Requirement Resolution Rules" and the command-execution
bullets of "Acceptance Ledger" in `DESIGN.md`). Optional `--requests` narrows
execution to the listed qualified request IDs (`<requirement>.<request>`); a
request whose requirement is not also in `--requirements` is still collected,
and a `--requests` entry naming a non-command request is refused with
`validation_failed`. Use it to re-prove a single artifact without re-collecting
a requirement's full evidence set.

## `evidence record` — evidence.json

```yaml
requirement: R-AUTH-002
request: E1                           # optional; the declared evidence request this
                                      # satisfies — kind must match when set; omitted
                                      # means supplemental evidence beyond the requests
kind: browser                         # review | browser | api | manual — command is refused
collected_by: "claude:verifier_T1"
note: "second open of same link → 'link already used'; no session cookie set"
artifact: "evidence/ev_12a6/replay.png"  # stored artifact reference; required for
                                         # browser/api at high and critical, else optional
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
    status: passed                    # passed | review_passed | failed | blocked;
                                      # waived is refused here (gate-only)
    evidence: ["ev_12a4"]             # required for passed/review_passed;
                                      # failed needs evidence or a finding
    findings: ["fd_77e1"]             # optional
    residual_risk: ""                 # required for review_passed at high/critical
    note: ""                          # required for blocked
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

Verification packets name the persona roster to fan out and carry the full
task diff reference against `baseline_commit` plus `prior_findings` from
earlier rounds (mechanics in "Repeat Review Rounds" in `DESIGN.md`).
