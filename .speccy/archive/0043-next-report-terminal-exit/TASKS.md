---
spec: SPEC-0043
spec_hash_at_generation: 41c552654130ddc494ca98ace899705ae0c08d504030f7bf585ff3052ba8fa11
generated_at: 2026-05-23T21:13:05Z
---
# Tasks: SPEC-0043 Per-spec `speccy next` as a loop-stop signal — REPORT.md-terminal priority, terminal exit code, template sweep

<task id="T-001" state="completed" covers="REQ-002">
## Reorder `compute_for_spec` so REPORT.md presence beats `journal/VET.md` state

In `speccy-core/src/next.rs`, change the all-tasks-completed branch
of `compute_for_spec` so the REPORT.md check fires before the
vet-gate check. New ordering when every task is `state="completed"`:

1. `report_md_exists(spec)` → return `None` (terminal).
2. Else, `vet_gate_is_fresh_pass(spec)` → return
   `Some(NextAction::Ship)`.
3. Else → return `Some(NextAction::Vet)`.

The current code at the all-completed branch reads the vet gate
first and returns `Vet` whenever VET.md is absent or stale, then
falls through to REPORT.md. Swap those two checks. The doc comment
above the function (the priority table at lines ~76–86) must be
updated to reflect the new ordering — rule 4 becomes "REPORT.md
present", rule 5 becomes "all tasks completed, no REPORT.md,
fresh vet gate → Ship", rule 6 becomes "all tasks completed, no
REPORT.md, no/stale vet gate → Vet".

The module-level doc block at the top of `next.rs` (lines ~13–26)
carries a numbered "Priority rule" enumeration that also needs to
match the new ordering.

Add or extend unit tests in `speccy-core/tests/` or
`speccy-cli/tests/next_derived.rs` covering the four combinations:

- (REPORT.md present, no VET.md) → `None`
- (REPORT.md present, stale/failing VET.md) → `None`
- (no REPORT.md, fresh passing VET.md) → `Some(Ship)`
- (no REPORT.md, no/stale VET.md) → `Some(Vet)`

After the change, `speccy next SPEC-0001 --json` against the
in-tree workspace must return `"next_action": null` rather than
the current `{"kind": "vet"}`. (Exit code remains 0 in this
task; T-002 wires the non-zero exit.)

<task-scenarios>
Given the modified `compute_for_spec` and a fixture spec with all
tasks completed, REPORT.md present, and `journal/VET.md` absent,
when `compute_for_spec(spec)` is invoked,
then the return is `None`.

Given the same modification and a fixture spec with all tasks
completed, REPORT.md absent, and `journal/VET.md` containing a
fresh passing `<gate>` whose `tasks_hash` matches the current
`TASKS.md`,
when `compute_for_spec(spec)` is invoked,
then the return is `Some(NextAction::Ship)`.

Given the same modification and the in-tree SPEC-0001 directory
unchanged,
when `speccy next SPEC-0001 --json` runs,
then stdout JSON carries `"next_action": null` and
`"reason": "completed"` (exit code semantics covered by T-002).

Suggested files: `speccy-core/src/next.rs`,
`speccy-core/tests/next.rs` (if present) or
`speccy-cli/tests/next_derived.rs`, `speccy-cli/tests/next_json.rs`.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-003">
## Add terminal-state exit code 2 and stderr message to `speccy next SPEC-NNNN`

In `speccy-cli/src/next.rs` and `speccy-cli/src/next_output.rs`,
extend the per-spec form so a terminal-state resolution produces:

- Process exit code `2`.
- A single human-readable stderr line of the shape
  `speccy next: SPEC-NNNN is {reason}; run \`speccy archive
  SPEC-NNNN\` to move it out of the active tree.` where `{reason}`
  is one of `completed` / `dropped` / `superseded`.
- The existing JSON envelope on stdout, unchanged in shape:
  `schema_version: 1`, `next_action: null`, `reason: {reason}`,
  plus the existing path fields.

Terminal states the per-spec form must detect:

- REPORT.md present (T-001 already returns `None` from
  `compute_for_spec`; map this to `reason: "completed"`).
- SPEC frontmatter `status: dropped` (use
  `ParsedSpec.status_or_in_progress()`; map to
  `reason: "dropped"`).
- SPEC frontmatter `status: superseded` (same accessor; map to
  `reason: "superseded"`).

The dropped/superseded paths currently flow through
`compute_for_spec` as if the spec were active. Add a short-circuit
at the top of the per-spec branch in `speccy-cli/src/next.rs`'s
`run` (or in a new helper) that classifies `Dropped` /
`Superseded` before invoking `compute_for_spec` and returns the
terminal envelope directly.

Update `run` in `speccy-cli/src/next.rs` so its return type can
carry a CLI exit code (e.g. return `Result<i32, NextError>` like
`speccy-cli/src/check.rs::run`, then map the returned code in the
dispatcher in `main.rs`). The existing `NextError` variants
(`ProjectRootNotFound`, `Workspace`, `SpecNotFound`,
`JsonSerialise`, `Io`) keep their current non-zero error
behaviour; the new exit `2` is reserved for the terminal-state
success path.

Non-terminal kinds (`work`, `review`, `vet`, `ship`, `decompose`)
continue to exit `0` from both the per-spec and workspace forms.
The workspace form (`speccy next --json` with no selector) is
unchanged — it omits terminal specs from the array and exits `0`.

Add tests in `speccy-cli/tests/next_json.rs` and
`speccy-cli/tests/next_text.rs`:

- `speccy next SPEC-XXXX --json` on a fixture with REPORT.md
  exits 2; stderr contains `completed` and `speccy archive
  SPEC-XXXX`; stdout JSON has `"reason": "completed"`.
- Same call against a fixture with `status: dropped` exits 2;
  stderr names `dropped`; JSON has `"reason": "dropped"`.
- Same call against a fixture with `status: superseded` exits 2;
  JSON has `"reason": "superseded"`.
- Same call against a fixture with one pending task exits 0;
  stderr is empty; JSON has a non-null `next_action`.

<task-scenarios>
Given the modified CLI and the in-tree SPEC-0001 directory at HEAD
(REPORT.md present, no VET.md),
when `speccy next SPEC-0001 --json` runs,
then the process exits 2, stderr contains
`SPEC-0001 is completed` and `speccy archive SPEC-0001`, stdout
matches the JSON envelope with `"next_action": null` and
`"reason": "completed"`.

Given a fixture spec with frontmatter `status: dropped`,
when `speccy next SPEC-XXXX --json` runs against it,
then exit code is 2 and stdout JSON carries `"reason": "dropped"`.

Given a fixture spec with one pending task,
when `speccy next SPEC-XXXX --json` runs against it,
then exit code is 0, stderr is empty, and stdout JSON carries a
non-null `next_action`.

Given a workspace at HEAD after T-001 and this task land,
when `speccy next --json` (no selector) runs,
then exit code is 0 and the `specs` array contains no entries for
the historical pre-vet shipped specs (SPEC-0001..0037, 0039, 0040).

Suggested files: `speccy-cli/src/next.rs`,
`speccy-cli/src/next_output.rs`, `speccy-cli/src/main.rs`,
`speccy-cli/tests/next_json.rs`,
`speccy-cli/tests/next_text.rs`.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001 REQ-004 REQ-005">
## Sweep canonical templates — pass SPEC-NNNN, teach exit-code-stop, fix ship.md drift

Edit the canonical templates under `resources/modules/` so every
`speccy next` invocation inside a SPEC-scoped section carries a
known `SPEC-NNNN` positional, and the surrounding prose teaches
the LLM to inspect the shell exit code as the loop-stop signal
before parsing JSON.

Files to edit:

- `resources/modules/skills/speccy-orchestrate.md` — every
  `speccy next --json` callsite inside the outer dispatch loop
  becomes `speccy next SPEC-NNNN --json` (templated with the
  orchestrator's SPEC ID — match the existing
  `{{ cmd_prefix }}`-style templating in the file). Add a one-line
  statement of the exit-code-stop contract in the outer-loop
  section.
- `resources/modules/skills/speccy-review.md` — the
  resolve-next-reviewable-task step when invoked via the selector
  form. Switch to per-spec form when SPEC-NNNN is known; keep the
  workspace form for the no-selector invocation path with an inline
  comment naming the reason. Add the exit-code-stop one-liner.
- `resources/modules/skills/partials/vet-phases.md` — the
  vet-phase resolver. Switch to per-spec form; add the exit-code
  one-liner.
- `resources/modules/phases/speccy-work.md` — the
  resolve-next-implementable-task step when invoked via the
  selector form. Same per-spec switch + exit-code-stop prose.
- `resources/modules/phases/speccy-ship.md` — REQ-005 doc drift
  fix. Remove the prose claiming `next_action: null` means "all
  tasks completed, no REPORT.md yet". State the corrected
  semantics: `next_action.kind == "ship"` is the ship-readiness
  signal; `next_action: null` paired with non-zero exit is the
  terminal-already-shipped signal.

For the exit-code-stop contract one-liner, use prose along the
shape: "If `speccy next SPEC-NNNN --json` exits non-zero, the SPEC
has reached a terminal state — halt the loop and surface the
stderr line to the user. Only parse the JSON when exit code is
0." Wording is agent discretion; the contract semantics are not.

Do not edit the no-selector invocation paths (workspace form) in
`speccy-work.md` and `speccy-review.md`. Those legitimately need
the workspace form to walk the active tree. If a workspace-form
callsite remains in a SPEC-scoped section after the sweep, attach
an inline comment naming the reason.

Verify after editing:

```bash
grep -nE 'speccy next( --json|.+--json)' resources/modules/skills/speccy-orchestrate.md resources/modules/skills/speccy-review.md resources/modules/skills/partials/vet-phases.md resources/modules/phases/speccy-work.md resources/modules/phases/speccy-ship.md
```

Every match should either carry a `SPEC-NNNN` (or templated SPEC
ID) positional or sit in a no-selector / workspace-exploration
section with an inline reason comment.

<task-scenarios>
Given the canonical templates at HEAD after this task,
when `grep -n 'speccy next --json'` runs against the five edited
files,
then every unfiltered match is in a no-selector workspace-form
context with an inline reason comment; no SPEC-scoped section
emits unfiltered `speccy next --json`.

Given `resources/modules/skills/speccy-orchestrate.md` at HEAD
after this task,
when a reader scans the outer-loop section,
then they find a one-line statement specifying that non-zero
exit from `speccy next SPEC-NNNN --json` is the loop-stop signal
and that the stderr line is surfaced to the caller.

Given `resources/modules/phases/speccy-ship.md` at HEAD after
this task,
when the file is searched for the substring `next_action: null`,
then any remaining occurrence describes it as the terminal /
already-shipped state, not as "all tasks completed, no REPORT.md
yet".

Suggested files: `resources/modules/skills/speccy-orchestrate.md`,
`resources/modules/skills/speccy-review.md`,
`resources/modules/skills/partials/vet-phases.md`,
`resources/modules/phases/speccy-work.md`,
`resources/modules/phases/speccy-ship.md`.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-006">
## Mirror canonical template changes into the three shipped skill packs

Propagate the T-003 canonical edits into the three shipped host
trees so the `cargo test -p speccy-cli skill_packs` invariant
holds:

- `.claude/skills/...` (Claude Code skill pack)
- `.codex/agents/...` (Codex agent pack)
- `.agents/skills/...` (host-neutral agent pack)

The mirror mechanism lives in
`speccy-cli/tests/skill_packs.rs`. Inspect that test to confirm
which files in each host tree correspond to the canonical
templates edited in T-003. If the test computes the mirror at
runtime (i.e. generates the shipped pack content from
`resources/modules/`), the change is mechanical and may amount
to nothing more than running the regeneration step. If the test
asserts byte-equivalence of pre-committed files against the
canonical resources, the shipped-pack files must be edited to
match the canonical content.

Mirror files most likely to need updates:

- `.claude/skills/speccy-orchestrate/SKILL.md`
- `.claude/skills/speccy-review/SKILL.md`
- `.claude/skills/speccy-work/SKILL.md`  (selector-form prose)
- `.claude/skills/speccy-vet/SKILL.md`  (vet-phase resolver)
- `.claude/agents/speccy-ship.md`  (doc drift mirror)
- `.codex/agents/speccy-work.toml`,
  `.codex/agents/speccy-ship.toml`,
  `.codex/agents/reviewer-tests.toml`
- `.agents/skills/speccy-orchestrate/SKILL.md`,
  `.agents/skills/speccy-review/SKILL.md`,
  `.agents/skills/speccy-work/SKILL.md`,
  `.agents/skills/speccy-vet/SKILL.md`

The list is indicative — the authoritative list comes from
`skill_packs.rs`. Run
`cargo test -p speccy-cli skill_packs --no-fail-fast` and let the
test surface every mismatch.

After mirroring, run the full hygiene suite:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly fmt --all --check
cargo deny check
```

<task-scenarios>
Given the workspace at HEAD after T-003 and this task,
when `cargo test -p speccy-cli skill_packs` runs,
then the test suite passes — confirming canonical resources and
the three shipped host packs are in sync.

Given the shipped Claude skill pack at HEAD after this task,
when a reader opens `.claude/skills/speccy-orchestrate/SKILL.md`
and searches for `speccy next`,
then every match inside the outer-dispatch-loop section uses the
`speccy next SPEC-NNNN --json` form, not unfiltered.

Given the workspace at HEAD after this task,
when the full hygiene suite (cargo test, clippy, fmt --check,
cargo deny) runs,
then every step exits 0.

Suggested files: `.claude/skills/...`, `.codex/agents/...`,
`.agents/skills/...`, `speccy-cli/tests/skill_packs.rs` (read-only
audit).
</task-scenarios>
</task>
