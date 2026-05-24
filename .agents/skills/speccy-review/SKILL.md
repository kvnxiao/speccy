---
name: speccy-review
description: 'Review one Speccy task per invocation and exit, running one round of adversarial multi-persona review. With an optional `SPEC-NNNN/T-NNN` selector, the session reviews that task; without it, the skill resolves the next reviewable task via `speccy next --json`. Four personas (business, tests, security, style) fan out in parallel and either pass the task to `completed` or flip it back to `pending` with a `<blockers>` block appended to the per-task journal file. Use when the user says "review T-003" or "review the next task". Requires: a task in `state="in-review"`. If no in-review task and work remains → prefer speccy-work. If all tasks `completed` → prefer speccy-ship. Do NOT trigger on generic "review this PR" or "review my code" asks — this skill runs Speccy task-state review only.'
---

# speccy-review

Runs one round of adversarial review on one task per invocation and
exits. With an optional `[SPEC-NNNN/T-NNN]` selector argument, the
session reviews that specific task. Without an argument, the session
resolves the next reviewable task via `speccy next --json` (workspace
form — used here because no SPEC is known on the no-selector path) and reviews
that one. Task state lives in the `state` attribute on each `<task>`
XML element in TASKS.md; review activity prose lives in the sibling
`.speccy/specs/NNNN-slug/journal/T-NNN.md` file, never inside the
`<task>` body.

This is a single-task primitive. It does not iterate over the
remaining `in-review` tasks; composition across tasks belongs to a
caller (a human at the terminal, the `/loop` skill, or a future
orchestrator).

Within the one task under review, the skill fans out to four
parallel persona sub-agents (default fan-out: `business`, `tests`,
`security`, `style`). That fan-out is intrinsic to the primitive —
adversarial diversity comes from fresh contexts per persona — and is
bounded to one round of four sub-agents on one task.

Because sub-agents cannot spawn sub-agents, this skill must run in a
context that **is** the top-level session — either a human
invocation (`speccy-review …`) where the host CLI
itself runs the skill body, or the
`speccy-orchestrate` outer loop which inlines this
skill body into its own session at the `review` dispatch (it cannot
delegate to a wrapper sub-agent that would then try to spawn the
four persona leaves).

## When to use

- With a selector (`speccy-review SPEC-NNNN/T-003`):
  when the task to review is already known.
- Without an argument: when picking up wherever `TASKS.md` left off.
  The session reviews one task and exits.

The target task must already be in `state="in-review"` (typically
flipped there by `speccy-work`).

## Steps

### Resolve the target task

- If a `SPEC-NNNN/T-NNN` selector was passed, that is the target.
- Otherwise, query the CLI in workspace form (no SPEC selector
  is known on this no-selector invocation path — we must walk
  the active tree to find a reviewable task):

  ```bash
  # workspace form: no SPEC-NNNN known yet; scan the active tree.
  speccy next --json
  ```

  Workspace-form exit-code-stop contract: exit code 2 with a
  top-level `reason="no_active_specs"` field in the JSON envelope
  means the workspace has no active specs at all. Exit gracefully
  and surface the reason; do not treat the non-zero exit as a CLI
  error.

  On exit code 0, if the resulting `specs` array has no entry with
  `next_action.kind == "review"`, exit and report that no
  reviewable tasks remain. Otherwise, construct the disambiguated
  `<spec>/<task>` form from the JSON's `spec_id` and
  `next_action.task_id` fields (the bare task ID is ambiguous
  across specs — every spec has its own `T-001`).

  Exit-code-stop contract: once SPEC-NNNN is resolved, any
  subsequent per-spec query (`speccy next SPEC-NNNN --json`) that
  exits non-zero means the SPEC has reached a terminal state —
  halt and surface the stderr line. Only parse JSON when exit
  code is 0.

### Run the persona fan-out and consolidation

Shared with the `speccy-orchestrate` review
dispatch — both this skill body and that dispatch step include the
same partial below so the fan-out contract has a single source of
truth.


Fan out four reviewer-* sub-agents in parallel against the resolved
task, one per persona. Default fan-out: `reviewer-business`,
`reviewer-tests`, `reviewer-security`, `reviewer-style`. Two
additional personas (`reviewer-architecture`, `reviewer-docs`) are
off the default fan-out and are invoked explicitly when an
architectural or documentation risk is suspected.

The prompt for each spawn is:

> Review task `SPEC-NNNN/T-NNN`. Run `speccy check SPEC-NNNN/T-NNN`
> to load the task scenarios, read the bare `<task>` body in
> TASKS.md and the prior activity in
> `.speccy/specs/NNNN-slug/journal/T-NNN.md`, and apply your
> persona's review criteria. Return your verdict as your final
> message as a
> `<review persona="<persona>" verdict="..." model="...">…</review>`
> element block. The `model` attribute is required and must
> identify the model that produced the verdict (with the optional
> slash-suffix effort convention from the verdict-return contract).
> Do not edit TASKS.md and do not edit the journal file.

Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name per
spawn.

Invoke Codex's native sub-agent-spawn primitive four times in
parallel against the registered Codex sub-agents
`reviewer-business`, `reviewer-tests`, `reviewer-security`, and
`reviewer-style`. Each persona's TOML file at
`.codex/agents/reviewer-<persona>.toml` carries the sub-agent's
developer instructions.

Canonical journal `<review>` shape:
`.agents/speccy-references/journal-review.md`.

Canonical journal `<blockers>` shape:
`.agents/speccy-references/journal-blockers.md`.

After all spawned sub-agents return, **consolidate** the `<review>`
element blocks from each reviewer's final message and append them
to `.speccy/specs/NNNN-slug/journal/T-NNN.md` **serially in the
running session** — do not delegate the write back to a reviewer
sub-agent, and do not write to TASKS.md.

When transcribing each returned `<review>` into the journal:

- Copy the `model` attribute **verbatim** from the reviewer's reply
  per `resources/modules/personas/verdict_return_contract.md`. Do
  not infer a model value from the persona name, the host
  skill-pack identity, or any other source. If a returned
  `<review>` is missing `model`, halt the fan-out and surface the
  non-conforming persona rather than inventing a value.
- Ensure each appended `<review>` carries the full required
  attribute set: `date` (ISO8601 with seconds and timezone),
  `model` (verbatim from the reviewer), `persona`, `verdict`
  (`pass` or `blocking`), and `round` (positive integer matching
  the implementer round under review). All five are required.
- If `journal/T-NNN.md` does not exist yet (a task can reach
  `in-review` only after the implementer wrote its round-1
  `<implementer>` block, so this should be rare — but if the file
  is somehow missing, surface that as an error rather than
  silently creating one without the implementer entry).

Apply the state transition to **TASKS.md serially in the running
session** (separate write from the journal append):

- If every spawned reviewer's `<review verdict="...">` is
  `verdict="pass"`, flip the task's `state="..."` attribute from
  `in-review` to `completed`.
- If any spawned reviewer's `<review verdict="...">` is
  `verdict="blocking"`, flip `state="..."` from `in-review` to
  `pending`, and append a single consolidated
  `<blockers>…</blockers>` element block to `journal/T-NNN.md`
  that aggregates all failing reviewers' feedback — not one
  `<blockers>` per reviewer, not a partial write. The block
  carries required attributes `date` and `round` (matching the
  round of the `<review>` blocks just appended) and has the form:

      <blockers date="2026-05-21T22:10:00Z" round="1">
      <one-line summary of what to change before the next
      implementer pass>.
      <optional bullets enumerating each persona's blocker>.
      </blockers>

This serial write in the running session eliminates the
parallel-write race that would occur if each reviewer sub-agent
wrote to the journal or TASKS.md directly (per DEC-008). Per-task
journal files do not introduce parallel writes from reviewer
sub-agents — the running session remains the sole journal writer
during review.


Reviewers do not write to TASKS.md and do not write to
`journal/T-NNN.md` directly; they return their verdict to this
running session, which is the **sole writer to
`.speccy/specs/NNNN-slug/journal/T-NNN.md`** for the review-induced
journal appends and the **sole writer to TASKS.md** for the
review-induced `state` transition. No `<review>` block is ever
appended to the `<task>` body in TASKS.md — TSK-006 rejects
journal elements there.

### Exit

Do not pick up another `in-review` task. If the caller wants
another task reviewed, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state:
if any task is `state="pending"` (a retry), suggest
`speccy-work SPEC-NNNN`. If any remain
`state="in-review"`, suggest
`speccy-review SPEC-NNNN` again. If all tasks are
`state="completed"`, suggest `speccy-vet SPEC-NNNN`.
