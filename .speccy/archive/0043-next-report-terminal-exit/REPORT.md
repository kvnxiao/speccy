---
spec: SPEC-0043
outcome: implemented
generated_at: 2026-05-24T00:30:00Z
---

# REPORT: SPEC-0043 Per-spec `speccy next` as a loop-stop signal — REPORT.md-terminal priority, terminal exit code, template sweep

<report spec="SPEC-0043">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-003 swept the five canonical templates under `resources/modules/` so every
`speccy next` invocation inside a SPEC-scoped section carries a `SPEC-NNNN`
(or templated SPEC-ID) positional. `speccy-orchestrate.md` now uses
`speccy next SPEC-NNNN --json` throughout the outer dispatch loop;
`speccy-review.md` and `speccy-work.md` use the per-spec form in their
selector-form paths while keeping workspace-form callsites in the no-selector
paths with inline reason comments; `vet-phases.md` uses the per-spec form for
the vet-phase resolver. A post-sweep grep confirms no unfiltered
`speccy next --json` remains inside SPEC-scoped sections. T-004 propagated
these changes into the three shipped host packs via `speccy init --force`.
Retry count: 1.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004 CHK-005">
T-001 reordered the all-tasks-completed branch of `compute_for_spec` in
`speccy-core/src/next.rs` so REPORT.md presence is checked before the vet gate.
New priority: (1) REPORT.md present -> return `None` (terminal); (2) no
REPORT.md, fresh passing vet gate -> return `Some(NextAction::Ship)`; (3) no
REPORT.md, no/stale vet gate -> return `Some(NextAction::Vet)`. The module-level
doc block and the priority-rule enumeration in the doc comment above
`compute_for_spec` were updated to match. Four new unit tests cover the
combinations: (REPORT.md present, no VET.md) -> `None`; (REPORT.md present,
stale VET.md) -> `None`; (no REPORT.md, fresh passing VET.md) -> `Some(Ship)`;
(no REPORT.md, absent VET.md) -> `Some(Vet)`. After this change,
`speccy next SPEC-0001 --json` returns `"next_action": null` rather than
`{"kind": "vet"}` for the 39 historical pre-vet shipped specs.
Retry count: 1.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-006 CHK-007 CHK-008 CHK-009">
T-002 extended the per-spec form of `speccy next` to exit with code `2` on
terminal states, writing a single human-readable stderr line of the form
`speccy next: SPEC-NNNN is {reason}; run \`speccy archive SPEC-NNNN\` to move it
out of the active tree`. The `run` function in `speccy-cli/src/next.rs` now
returns `Result<i32, NextError>` (matching `check.rs`), and `main.rs`
maps the returned code through to the process. A short-circuit at the top of
the per-spec branch classifies `Dropped`/`Superseded` frontmatter status before
invoking `compute_for_spec`. The JSON envelope on stdout is preserved unchanged
with `schema_version: 1`, `next_action: null`, and `reason in {completed,
dropped, superseded}`. Non-terminal kinds continue to exit `0`. Tests in
`speccy-cli/tests/next_json.rs` and `speccy-cli/tests/next_text.rs` cover:
REPORT.md-present exits 2 with `completed`; `status: dropped` exits 2 with
`dropped`; `status: superseded` exits 2 with `superseded`; one-pending-task
exits 0 with empty stderr.
Retry count: 1.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-010">
T-003 added exit-code-stop contract prose to each affected canonical template.
The orchestrator's outer-loop section now carries a one-line statement
specifying that non-zero exit from `speccy next SPEC-NNNN --json` is the
loop-stop signal and that the orchestrator must surface the stderr line to the
caller. The `speccy-work.md`, `speccy-review.md`, and `vet-phases.md` selector
paths carry the same one-liner co-located with their `speccy next` invocations.
The loop-termination condition is expressed as "exit code non-zero" rather than
"JSON `next_action` is null". T-004 mirrored this prose into all three shipped
host packs.
Retry count: 1.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-011">
T-003 corrected the doc drift in `resources/modules/phases/speccy-ship.md`.
The prose that claimed `next_action: null` means "all tasks completed, no
REPORT.md yet" was removed and replaced with the correct semantics:
`next_action.kind == "ship"` is the ship-readiness signal; `next_action: null`
paired with non-zero exit is the terminal-already-shipped signal. T-004
propagated the fix into the shipped mirrors at `.claude/agents/speccy-ship.md`
and `.codex/agents/speccy-ship.toml`.
Retry count: 1.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-012 CHK-013">
T-004 propagated the T-003 canonical resource edits into the three shipped host
packs (`.claude/skills/`, `.claude/agents/`, `.codex/agents/`, `.agents/skills/`)
using the documented `speccy init --force` mirror mechanism as enforced by
the CI `git diff --exit-code` step. `cargo test -p speccy-cli --test skill_packs`
passes 34/34 tests, confirming byte-equivalence between the canonical resources
and the shipped host packs. Every `speccy next` callsite in the outer-dispatch-loop
section of `.claude/skills/speccy-orchestrate/SKILL.md` uses the
`speccy next SPEC-NNNN --json` form.
Retry count: 1.
</coverage>

</report>

## Notes

All four tasks required exactly one retry round. T-001's single retry was
triggered by a tests-reviewer blocker requesting an evidence file; the
underlying reorder was correct from round 1. T-002's retry resolved a style
blocker around the `run` return-type refactor. T-003's retry created the
missing evidence paper trail -- the canonical template edits were structurally
correct from round 1. T-004's retry similarly produced the missing evidence
file at the path named in the journal; the `speccy init --force` mirror was
already applied in round 1.

The JNL-003 lint error in T-004's journal (non-monotonic round counter: a
`<blockers round="1">` element appearing after a `<review round="2">` element)
is a parser sequencing artefact from the review workflow appending the
`<blockers>` block after the round-2 content was already committed. The task
state is `completed` and all four personas passed in round 2; the artefact does
not affect the ground-truth outcome.
