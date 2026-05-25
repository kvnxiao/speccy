---
id: SPEC-0043
slug: next-report-terminal-exit
title: Per-spec `speccy next` as a loop-stop signal — REPORT.md-terminal priority, terminal exit code, template sweep
status: implemented
created: 2026-05-23
supersedes: []
archived_at: 2026-05-23
archived_reason: "v1 milestone shipped"
---

# SPEC-0043: Per-spec `speccy next` as a loop-stop signal — REPORT.md-terminal priority, terminal exit code, template sweep

## Summary

`speccy next` is the deterministic loop-signal that drives the
`/speccy-orchestrate` outer loop and the per-task primitives
(`/speccy-work`, `/speccy-review`, `/speccy-vet`, `/speccy-ship`).
Two defects in that signal pollute the loop today.

First, the shipped templates almost always invoke
`speccy next --json` without the optional `SPEC-NNNN` positional,
even when the caller's SPEC scope is known (e.g. the orchestrator
was invoked as `/speccy-orchestrate SPEC-0042`). The CLI then emits
the entire workspace tree as JSON, forcing the LLM to filter the
list by hand and wasting context window on specs that are not part
of the current loop. This is the F-2 entry in `.speccy/BACKLOG.md`.

Second, `compute_for_spec` checks the `journal/VET.md` gate before
it checks `REPORT.md` presence. The vet lifecycle gate was added in
SPEC-0041, but every spec shipped before that — 39 of the 42 specs
in the active tree at HEAD — has `REPORT.md` and no `journal/VET.md`.
Those 39 specs all report `next_action: {kind: "vet"}` despite being
shipped, which pollutes every workspace-form `speccy next` query and
would push the orchestrator into a re-vet loop on already-shipped
work if called against them.

This SPEC closes F-2 by:

1. Reordering `compute_for_spec` so REPORT.md presence is terminal
   regardless of `journal/VET.md` state. Shipped specs stop
   reporting `vet`; the priority becomes the conceptually correct
   "REPORT.md is the durable shipped marker; vet is a pre-ship
   gate".
2. Making `speccy next SPEC-NNNN --json` on a terminal spec
   (REPORT.md present, or `status: dropped`, or `status:
   superseded`) exit with code `2`, write a stderr line naming the
   spec and suggesting `speccy archive SPEC-NNNN`, while preserving
   the existing JSON envelope on stdout (no schema break).
3. Sweeping the canonical templates under `resources/modules/skills/`
   and `resources/modules/phases/` plus their shipped mirrors so the
   `speccy next` invocation always carries a known SPEC-NNNN
   positional and the surrounding prose teaches the LLM to treat
   non-zero exit as the loop-stop signal before parsing JSON.
4. Correcting a small doc drift in `phases/speccy-ship.md` —
   `next_action: null` is terminal, not "all tasks completed, no
   REPORT.md yet"; ship-readiness is signalled by
   `next_action: ship`.

## Goals

<goals>
- `speccy next SPEC-NNNN --json` on a fully-shipped spec exits 2
  with a stderr line that names the spec, its terminal reason, and
  a `speccy archive SPEC-NNNN` suggestion; the JSON envelope on
  stdout keeps `schema_version: 1` and adds no new fields.
- After this SPEC lands, `speccy next` against the 39 historical
  pre-vet specs returns `null` (terminal), not `vet`. The
  workspace-form `speccy next` returns only specs with actual
  pending loop steps.
- Every shipped template that already knows its SPEC-NNNN scope
  passes that scope through to `speccy next`. No template emits an
  unfiltered `speccy next --json` invocation inside a SPEC-scoped
  context.
- Templates teach the LLM to read the shell exit code as the
  loop-stop signal before parsing JSON, so the orchestrator loop
  terminates cleanly when a SPEC enters a terminal state.
</goals>

## Non-goals

<non-goals>
- No per-spec filter added to `speccy verify`. Verify is the
  whole-workspace CI gate by design; per-spec verify is deferred to
  a future SPEC if a concrete consumer surfaces.
- No backfill of synthetic `journal/VET.md` gate files for the
  historical pre-vet specs. The priority reorder removes the need;
  rewriting 39 on-disk files just papers over the logic bug.
- No new JSON envelope fields for `speccy next`. `next_action`,
  `reason`, and the existing path fields cover every terminal-state
  case in the existing `schema_version: 1` shape.
- No change to `speccy check`. Its callsites are already optimally
  scoped via `SPEC-NNNN/T-NNN` selectors in templates.
- No change to the `next_action: null` decompose / vet / ship /
  work / review paths' exit codes. Non-terminal kinds continue to
  exit `0`.
</non-goals>

## User Stories

<user-stories>
- As the `/speccy-orchestrate` outer loop, I want
  `speccy next SPEC-NNNN --json` to tell me unambiguously when the
  SPEC has reached a terminal state via a non-zero exit code, so I
  can stop looping without parsing JSON to detect completion.
- As an LLM agent reading the shipped orchestrator prose, I want
  the `speccy next` invocation in the template to already include
  the SPEC-NNNN I was invoked with, so I do not burn context
  filtering an entire workspace tree to find one spec.
- As a Speccy user who shipped specs before the vet lifecycle, I
  want `speccy next SPEC-0001` to report the SPEC as terminal (not
  `vet`), so the loop-driving tools do not treat my historical work
  as if it still needed a pre-ship gate.
- As a Speccy user with a shipped SPEC I want out of my active
  tree, I want the terminal stderr message to remind me that
  `speccy archive SPEC-NNNN` is the hygiene tool for the situation.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Templates pass their known SPEC-NNNN through to `speccy next`

Every shipped template that already knows its SPEC-NNNN context
invokes `speccy next SPEC-NNNN --json` rather than the unfiltered
`speccy next --json`. Affected canonical templates under
`resources/modules/`:

- `skills/speccy-orchestrate.md` (every `speccy next --json`
  callsite inside the outer dispatch loop)
- `phases/speccy-work.md` (the resolve-next-task step when invoked
  via the selector form)
- `skills/speccy-review.md` (the resolve-next-reviewable-task step
  when invoked via the selector form)
- `skills/partials/vet-phases.md` (the vet-phase resolver)
- `phases/speccy-ship.md` (already filtered; verify no regression)

<done-when>
- After this SPEC lands, `grep -rn 'speccy next --json' resources/modules/`
  returns zero matches inside SPEC-scoped sections of the affected
  templates. Unfiltered callsites that legitimately need the
  workspace form (e.g. brainstorm exploration) carry an inline
  comment naming the reason.
- The non-selector forms of `speccy-work` and `speccy-review`
  (invoked without a `SPEC-NNNN/T-NNN` argument) continue to use
  the unfiltered workspace form; the change is scoped to the
  selector forms where SPEC-NNNN is known.
</done-when>

<behavior>
- Given an orchestrator invoked as `/speccy-orchestrate SPEC-0042`,
  when the orchestrator queries `speccy next`, then the rendered
  shell command in the template is `speccy next SPEC-0042 --json`
  and the LLM receives a per-spec JSON envelope, not a
  workspace-wide tree.
- Given `speccy-work` invoked with selector `SPEC-0042/T-005`, when
  the work skill resolves the task, then it queries
  `speccy next SPEC-0042 --json` rather than the workspace form.
</behavior>

<scenario id="CHK-001">
Given the canonical templates at HEAD after this SPEC lands,
when `grep -n 'speccy next --json' resources/modules/skills/speccy-orchestrate.md resources/modules/skills/speccy-review.md resources/modules/skills/partials/vet-phases.md resources/modules/phases/speccy-work.md resources/modules/phases/speccy-ship.md` runs,
then every match is preceded by a `SPEC-NNNN` (or templated
`{{ spec_id }}`) positional argument, except in clearly-commented
workspace-exploration callsites.
</scenario>

<scenario id="CHK-002">
Given the orchestrator's outer dispatch loop prose,
when a reader follows the instructions for `/speccy-orchestrate SPEC-0042`,
then the very first `speccy next` invocation they execute is
`speccy next SPEC-0042 --json`, not `speccy next --json`.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `compute_for_spec` priority reordered — REPORT.md beats VET.md

When `compute_for_spec` finds all tasks completed, the next-action
derivation checks `REPORT.md` presence before the vet gate. New
priority within the all-tasks-completed branch:

1. `REPORT.md` present → return `None` (terminal).
2. Else, vet gate fresh-pass → return `Some(NextAction::Ship)`.
3. Else → return `Some(NextAction::Vet)`.

REPORT.md presence is the durable shipped marker; the vet gate is
a pre-ship affordance. Once REPORT.md exists, the spec is by
definition shipped and the loop terminates regardless of vet
artifact state.

<done-when>
- A spec with all tasks completed and REPORT.md present returns
  `None` from `compute_for_spec`, even if `journal/VET.md` is
  absent or carries a stale `tasks_hash`.
- A spec with all tasks completed, REPORT.md absent, and a fresh
  passing vet gate returns `Some(NextAction::Ship)`.
- A spec with all tasks completed, REPORT.md absent, and either no
  vet gate or a stale/failing one returns `Some(NextAction::Vet)`.
- After this SPEC lands, `speccy next --json` against the workspace
  at HEAD returns entries only for SPEC-NNNN with genuine pending
  loop steps; the 39 historical pre-vet specs do not appear.
</done-when>

<behavior>
- Given a spec directory with `TASKS.md` listing only completed
  tasks, `REPORT.md` present, and `journal/VET.md` absent, when
  `compute_for_spec` is invoked, then it returns `None`.
- Given the same setup but with `journal/VET.md` carrying a
  failing or stale `<gate>` block, when `compute_for_spec` is
  invoked, then it still returns `None` (REPORT.md wins).
- Given the same setup but with `REPORT.md` absent and
  `journal/VET.md` carrying a fresh passing gate, when
  `compute_for_spec` is invoked, then it returns
  `Some(NextAction::Ship)`.
</behavior>

<scenario id="CHK-003">
Given a built `speccy` binary at HEAD after this SPEC lands,
when `speccy next SPEC-0001 --json` runs against the in-tree
SPEC-0001 directory (REPORT.md present, no journal/VET.md),
then stdout JSON has `"next_action": null` and `"reason": "completed"`.
</scenario>

<scenario id="CHK-004">
Given a fixture spec directory with all tasks completed,
REPORT.md absent, and `journal/VET.md` containing a fresh passing
`<gate>` whose `tasks_hash` matches the current TASKS.md,
when `compute_for_spec` is invoked,
then the returned action is `Some(NextAction::Ship)`.
</scenario>

<scenario id="CHK-005">
Given the workspace at HEAD after this SPEC lands,
when `speccy next --json` is invoked,
then the `specs` array contains no entry whose `spec_id` is one of
the historical pre-vet shipped specs (SPEC-0001 through SPEC-0037,
plus SPEC-0039 and SPEC-0040).
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `speccy next SPEC-NNNN --json` exits 2 on terminal specs

When the per-spec form resolves to a terminal state — REPORT.md
present (`reason: "completed"`), or SPEC frontmatter
`status: dropped` (`reason: "dropped"`), or SPEC frontmatter
`status: superseded` (`reason: "superseded"`) — the CLI:

- Exits with process code `2`.
- Writes a single human-readable line to stderr naming the spec,
  its terminal reason, and the suggestion
  `run \`speccy archive SPEC-NNNN\` to move it out of the active
  tree`.
- Still writes the existing JSON envelope to stdout with
  `schema_version: 1`, `next_action: null`,
  `reason ∈ {"completed", "dropped", "superseded"}`, and the
  existing path fields. The schema does not change.

On every non-terminal kind (`decompose`, `work`, `review`, `vet`,
`ship`), the per-spec form continues to exit `0` with the existing
JSON shape. Only terminal states are loud.

The workspace form (`speccy next --json`, no selector) is
unaffected — it omits terminal specs from the array as it does
today, exits `0`.

<done-when>
- `speccy next SPEC-0001 --json` at HEAD exits 2 (REPORT.md
  present), prints the JSON envelope on stdout, and prints a
  stderr line containing both the substring `completed` and the
  substring `speccy archive SPEC-0001`.
- `speccy next SPEC-XXXX --json` on a fixture spec with
  frontmatter `status: dropped` exits 2, stdout JSON carries
  `"reason": "dropped"`, stderr names the reason as `dropped`.
- `speccy next SPEC-XXXX --json` on a fixture spec with
  `status: superseded` exits 2, stdout JSON carries
  `"reason": "superseded"`.
- `speccy next SPEC-XXXX --json` on a fixture spec with pending
  tasks (kind=work) exits 0, no stderr noise about terminal state.
- The existing `SpecNotFound` and I/O error variants of
  `NextError` continue to exit non-zero with their current
  messages; exit code `2` is not collided.
</done-when>

<behavior>
- Given a built CLI and the in-tree SPEC-0001 directory, when
  `speccy next SPEC-0001 --json` runs, then exit code is 2,
  stdout is the JSON envelope, stderr names the spec and suggests
  archive.
- Given a fixture spec with frontmatter `status: dropped`, when
  the per-spec form runs against it, then exit code is 2 and the
  reason field in JSON is `"dropped"`.
- Given a fixture spec that is mid-loop (one task pending), when
  the per-spec form runs against it, then exit code is 0 and
  stderr is empty.
</behavior>

<scenario id="CHK-006">
Given a built `speccy` binary at HEAD after this SPEC lands and
the in-tree SPEC-0001 directory unchanged,
when `speccy next SPEC-0001 --json` runs,
then the process exits 2, stdout matches the JSON envelope shape
with `"next_action": null` and `"reason": "completed"`, and
stderr contains the substring `speccy archive SPEC-0001`.
</scenario>

<scenario id="CHK-007">
Given a fixture spec with frontmatter `status: dropped`,
when `speccy next SPEC-XXXX --json` runs against it,
then exit code is 2 and stdout JSON carries `"reason": "dropped"`.
</scenario>

<scenario id="CHK-008">
Given a fixture spec with frontmatter `status: superseded`,
when `speccy next SPEC-XXXX --json` runs against it,
then exit code is 2 and stdout JSON carries `"reason": "superseded"`.
</scenario>

<scenario id="CHK-009">
Given a fixture spec with one pending task (kind=work),
when `speccy next SPEC-XXXX --json` runs against it,
then exit code is 0, stderr is empty, and stdout JSON carries a
non-null `next_action`.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Templates treat non-zero exit as the loop-stop signal

Every shipped template whose flow reads `speccy next SPEC-NNNN
--json` carries prose that teaches the LLM to inspect the shell
exit code first and treat any non-zero exit as the loop-stop
signal, before parsing JSON. This applies to the orchestrator's
outer dispatch loop and the selector-form work/review/vet/ship
templates.

The prose names the contract explicitly: exit `0` = continue the
loop using the JSON; exit non-zero = the SPEC has reached a
terminal state, halt the loop and surface the stderr message to
the user.

<done-when>
- Each affected canonical template under `resources/modules/`
  contains a one-line statement of the exit-code contract,
  co-located with its `speccy next` invocation.
- The orchestrator's loop-termination condition reads as "exit
  code from `speccy next SPEC-NNNN --json` is non-zero", not as
  "JSON `next_action` is null". Both signals exist; the prose
  prefers the exit code.
</done-when>

<behavior>
- Given the orchestrator dispatching against a terminal SPEC, when
  the dispatch step runs `speccy next SPEC-NNNN --json`, then the
  orchestrator stops the outer loop based on the non-zero exit
  code and surfaces the stderr line to the user.
- Given the selector form of `speccy-work` or `speccy-review`,
  when the resolver step queries `speccy next SPEC-NNNN --json`
  and observes a non-zero exit, then the skill exits with a brief
  status message rather than attempting to dispatch further work.
</behavior>

<scenario id="CHK-010">
Given the canonical orchestrator template at HEAD after this SPEC
lands,
when a reader scans the outer-loop section,
then they find a statement specifying that non-zero exit from
`speccy next SPEC-NNNN --json` is the loop-stop signal and that
the orchestrator must surface the stderr line to the caller.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: `phases/speccy-ship.md` `next_action: null` doc drift corrected

`phases/speccy-ship.md` today asserts that
`speccy next SPEC-NNNN --json` returning `"next_action": null`
means "all tasks completed, no REPORT.md yet". That claim conflicts
with `compute_for_spec`'s actual semantics: `null` means terminal
(REPORT.md present); ship-readiness is signalled by
`next_action: { kind: "ship" }`. This requirement fixes the prose.

<done-when>
- `phases/speccy-ship.md` no longer contains the string
  `next_action: null` claiming a "no REPORT.md yet" meaning.
- The corrected prose names `next_action: { kind: "ship" }` as the
  ship-readiness signal and `next_action: null` (with exit code 2)
  as the terminal-state signal.
- The shipped mirror in `.claude/agents/speccy-ship.md` and
  `.codex/agents/speccy-ship.toml` carries the same corrected
  prose.
</done-when>

<behavior>
- Given a reader following the ship template, when they reach the
  pre-ship verification step, then the prose tells them to expect
  `next_action.kind == "ship"` for a ship-ready SPEC and to treat
  `next_action: null` paired with non-zero exit as the terminal
  signal (SPEC already shipped).
</behavior>

<scenario id="CHK-011">
Given `resources/modules/phases/speccy-ship.md` at HEAD after this
SPEC lands,
when the file is read,
then it states that `next_action.kind == "ship"` is the
ship-readiness signal and that `next_action: null` paired with a
non-zero exit is the terminal-state signal.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Shipped skill packs mirror the canonical changes

The shipped skill packs reflect the canonical changes from
`resources/modules/` across the three host trees:

- `.claude/skills/...` (Claude Code skill pack)
- `.codex/agents/...` (Codex agent pack)
- `.agents/skills/...` (host-neutral agent pack)

Mirror integrity is enforced by `speccy-cli/tests/skill_packs.rs`;
the canonical edits plus the existing test invariants are the
mechanism. This requirement records the integration contract.

<done-when>
- `cargo test -p speccy-cli skill_packs` passes at HEAD after this
  SPEC lands.
- For each affected canonical template under `resources/modules/`,
  the corresponding files in `.claude/skills/`, `.codex/agents/`,
  and `.agents/skills/` carry the new invocation patterns (per-spec
  `speccy next SPEC-NNNN --json`, exit-code-stop prose).
- The three host trees stay in sync with no manual divergence
  introduced for host-specific reasons during this SPEC.
</done-when>

<behavior>
- Given a contributor editing a canonical template under
  `resources/modules/`, when they run `cargo test -p speccy-cli
  skill_packs`, then the test fails until the corresponding
  shipped-pack files reflect the canonical content.
- Given the three shipped host packs at HEAD after this SPEC lands,
  when each is inspected, then they carry the per-spec
  `speccy next SPEC-NNNN --json` invocation pattern and the
  exit-code-stop prose from the canonical resources.
</behavior>

<scenario id="CHK-012">
Given the workspace at HEAD after this SPEC lands,
when `cargo test -p speccy-cli skill_packs` runs,
then the test suite passes — confirming the canonical resources
and the three shipped host packs are byte-equivalent (modulo
host-specific templating).
</scenario>

<scenario id="CHK-013">
Given the shipped Claude skill pack at HEAD after this SPEC lands,
when a reader opens `.claude/skills/speccy-orchestrate/SKILL.md`
and searches for `speccy next`,
then every match inside the outer-dispatch-loop section uses the
`speccy next SPEC-NNNN --json` form, not unfiltered.
</scenario>

</requirement>

## Assumptions

<assumptions>
- Shipped-pack sync between `resources/modules/` and the three
  host trees (`.claude/`, `.codex/`, `.agents/`) is governed by
  `speccy-cli/tests/skill_packs.rs` and its byte-equivalence
  invariants. Canonical edits propagate to mirrors via the
  existing mechanism; manual per-host divergence is not required
  for this SPEC.
- `REPORT.md` is effectively immutable in the loop sense.
  `/speccy-amend` mid-loop does not pre-write REPORT.md; once
  REPORT.md is written, the spec is shipped. If a future amend
  workflow rewrites REPORT.md, "REPORT.md presence terminal"
  still holds because amend would have to delete REPORT.md to
  re-enter the active loop.
- Exit code `2` is free in `speccy next`'s exit-code space.
  Existing `NextError` variants (`ProjectRootNotFound`,
  `Workspace`, `SpecNotFound`, `JsonSerialise`, `Io`) are mapped
  through generic non-zero exits today; the impl audits and
  reserves `2` for terminal-state without colliding with these.
  `speccy archive` already uses `2` for its error exits in the
  same CLI binary — the convention generalises.
- `ParsedSpec.status_or_in_progress()` already exposes the
  `Dropped` and `Superseded` SPEC frontmatter status values to the
  `next` derivation layer. The terminal-state mapping reads
  through this existing API; no new parsing surface is required.
- No external (non-Speccy) consumer parses `speccy next --json`
  exit codes today. The shipped templates are the only consumers;
  changing the contract for terminal specs does not break any
  out-of-tree integration.
</assumptions>

## Decisions

<decision id="DEC-001">
REPORT.md presence is the durable shipped marker; the vet gate is
a pre-ship affordance. Priority is reordered so REPORT.md beats
`journal/VET.md` rather than the inverse. This SPEC adopts the
conceptual-correctness fix over a migration that backfills
synthetic `journal/VET.md` gates for the 39 historical pre-vet
specs.
</decision>

<decision id="DEC-002">
Exit code `2` is reserved for terminal-state exits from
`speccy next SPEC-NNNN --json`. The choice matches the existing
`speccy archive` convention for error-shaped exits in the same
binary. `1` is reserved for generic CLI errors; a dedicated code
beyond `2` is unnecessary for a single new exit class.
</decision>

<decision id="DEC-003">
The JSON envelope on stdout is preserved unchanged on terminal
exits — `schema_version: 1`, `next_action: null`, `reason ∈
{"completed", "dropped", "superseded"}`, plus the existing path
fields. Schema is not bumped. Exit code is the loop-stop signal;
the envelope continues to carry the data payload. Orthogonal
concerns kept orthogonal.
</decision>

<decision id="DEC-004">
The loud-exit semantic applies only to terminal states. Non-
terminal kinds (`decompose`, `work`, `review`, `vet`, `ship`)
continue to exit `0`. Decompose is "loop continues with the
decompose step", not "loop stops"; the same logic applies to the
other non-terminal kinds.
</decision>

<decision id="DEC-005">
`speccy verify` does not gain a per-spec filter in this SPEC.
Verify is the whole-workspace CI gate by design; no active loop
driver demands per-spec scope. If a concrete consumer surfaces
later, a follow-up SPEC can extend the surface.
</decision>

## Notes

Rejected framings carried for durability:

- **Two-SPEC split (priority reorder hotfix, then F-2).** Would
  ship the priority reorder as a standalone fix and leave F-2's
  template sweep plus loud exit for a separate SPEC. Rejected on
  cohesion grounds: every change touches `speccy next` loop
  semantics, and the user explicitly preferred bundling during
  brainstorm.
- **Three-SPEC split (template sweep / loud exit / priority
  reorder atomized).** Maximises atomicity but fragments the
  dogfood loop across three review cycles for one cohesive
  feature.
- **Adding `speccy verify SPEC-NNNN` per-spec filter to this
  SPEC.** Deferred — verify is whole-workspace CI gate by design;
  no active loop demands per-spec verify scope. Covered by
  DEC-005.
- **Migrating the 39 historical pre-vet specs by backfilling
  synthetic `journal/VET.md` gate files.** Rejected — papers over
  the priority-order bug instead of fixing it. The priority
  reorder is the real fix.

Originating ask: F-2 in `.speccy/BACKLOG.md` ("`speccy next` and
`speccy check` both support passing in a `SPEC-####` as an
immediate argument…"). This SPEC closes F-2; the BACKLOG.md entry
should be removed when REPORT.md lands for SPEC-0043.

## Open Questions

(None — all resolved during brainstorm.)

## Changelog

<changelog>
| Date       | Reason                                                   | Author     |
|------------|----------------------------------------------------------|------------|
| 2026-05-23 | Initial draft. Closes F-2 in `.speccy/BACKLOG.md`. Three coupled changes to `speccy next`: (1) `compute_for_spec` priority reordered so REPORT.md presence is terminal regardless of `journal/VET.md` state — fixes the production symptom where 39 of 42 in-tree specs report `kind: "vet"` despite being shipped pre-vet-lifecycle (SPEC-0041); (2) per-spec form `speccy next SPEC-NNNN --json` on a terminal spec (REPORT.md present, or `status: dropped`, or `status: superseded`) exits with code 2, writes a stderr line naming the spec + terminal reason + a `speccy archive SPEC-NNNN` archive-hygiene suggestion, while preserving the existing `schema_version: 1` JSON envelope on stdout (no schema break); (3) canonical templates under `resources/modules/skills/` and `resources/modules/phases/` plus their three shipped mirrors (`.claude/skills/`, `.codex/agents/`, `.agents/skills/`) sweep so every SPEC-scoped `speccy next` invocation carries a known SPEC-NNNN positional and the surrounding prose teaches the LLM to inspect non-zero exit as the loop-stop signal before parsing JSON. Also fixes the `phases/speccy-ship.md` doc drift that claimed `next_action: null` means "all tasks completed, no REPORT.md yet" — null is terminal; ship-readiness is `next_action.kind == "ship"`. Decisions: exit code 2 (matches `speccy archive` convention in same binary); loud exit applies only to terminal states (decompose/work/review/vet/ship continue to exit 0); no per-spec filter for `speccy verify` (deferred — whole-workspace CI gate by design); no synthetic `journal/VET.md` backfill for historical pre-vet specs (the priority reorder is the real fix). | Kevin Xiao |
</changelog>
