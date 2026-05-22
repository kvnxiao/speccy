
# {{ cmd_prefix }}speccy-review

Runs one round of adversarial review on one task per invocation and
exits. With an optional `[SPEC-NNNN/T-NNN]` selector argument, the
session reviews that specific task. Without an argument, the session
resolves the next reviewable task via `speccy next --json` and reviews
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

## When to use

- With a selector (`{{ cmd_prefix }}speccy-review SPEC-0007/T-003`):
  when the task to review is already known.
- Without an argument: when picking up wherever `TASKS.md` left off.
  The session reviews one task and exits.

The target task must already be in `state="in-review"` (typically
flipped there by `{{ cmd_prefix }}speccy-work`).

## Steps

1. Resolve the target task.

   - If a `SPEC-NNNN/T-NNN` selector was passed, that is the target.
   - Otherwise, query the CLI:

     ```bash
     speccy next --json
     ```

     If the result has no entry with `next_action.kind == "review"`,
     exit and report that no reviewable tasks remain. Otherwise,
     construct the disambiguated `<spec>/<task>` form from the JSON's
     `spec_id` and `next_action.task_id` fields (the bare task ID is
     ambiguous across specs — every spec has its own `T-001`).

2. Fan out four reviewer sub-agents in parallel via the host-native
   sub-agent primitive, one per persona in the **default fan-out:
   `business`, `tests`, `security`, `style`**. Two additional
   personas (`architecture`, `docs`) are off the default fan-out
   and are invoked explicitly when an architectural or documentation
   risk is suspected (add them to the spawn call when needed; omit
   them for routine task reviews). Each sub-agent's prompt is the
   bash command form below, not the CLI-rendered prompt text inlined
   into the spawn call. The CLI command remains the source of truth
   for what each persona reads.

   Each spawned reviewer **returns its verdict via its final
   message** as a
   `<review persona="..." verdict="..." model="...">…</review>`
   element block. Reviewers do not write to TASKS.md and do not write
   to `journal/T-NNN.md` directly; they return their verdict to this
   orchestrator. After all spawned reviewers return, this orchestrator
   is the **sole writer to `.speccy/specs/NNNN-slug/journal/T-NNN.md`**
   for the review-induced journal appends, and the **sole writer to
   TASKS.md** for the review-induced `state` transition. No
   `<review>` block is ever appended to the `<task>` body in
   TASKS.md — TSK-006 rejects journal elements there.

   {% if host == "claude-code" %}Invoke the `Task` tool four times in parallel, once per persona,
   with `subagent_type: "reviewer-business"`,
   `subagent_type: "reviewer-tests"`,
   `subagent_type: "reviewer-security"`, and
   `subagent_type: "reviewer-style"`. The prompt for each spawn is:

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

   Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name.
   Each subagent resolves to its markdown file at
   `.claude/agents/reviewer-<persona>.md`, so the persona body is
   already loaded for the sub-agent.{% else %}Invoke Codex's native sub-agent-spawn primitive four times in
   parallel, once per persona, against the registered Codex sub-agents
   `reviewer-business`, `reviewer-tests`, `reviewer-security`, and
   `reviewer-style`. The prompt for each spawn is:

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

   Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name.
   Codex resolves each name to its TOML file at
   `.codex/agents/reviewer-<persona>.toml`, so the persona body is
   already loaded as the sub-agent's developer instructions.{% endif %}

   Canonical journal `<review>` shape: `references/journal-review.md`.

   Canonical journal `<blockers>` shape: `{{ speccy_references_path }}/journal-blockers.md`.

3. After all spawned sub-agents return, **consolidate** the
   `<review>` element blocks from each reviewer's final message and
   write them to `.speccy/specs/NNNN-slug/journal/T-NNN.md`
   **serially in this orchestrator turn** — do not delegate the
   write back to a reviewer subagent, and do not write to TASKS.md.

   When transcribing each returned `<review>` into the journal:

   - Copy the `model` attribute **verbatim** from the reviewer's
     reply per `resources/modules/personas/verdict_return_contract.md`.
     Do not infer a model value from the persona name, the host
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
     `<implementer>` block, so this should be rare — but if the
     file is somehow missing, surface that as an error rather than
     silently creating one without the implementer entry).

   Apply the state transition to **TASKS.md serially in this
   orchestrator turn** (separate write from the journal append):

   - If every spawned reviewer's `<review verdict="...">` is
     `verdict="pass"`, flip the task's `state="..."` attribute
     from `in-review` to `completed`.
   - If any spawned reviewer's `<review verdict="...">` is
     `verdict="blocking"`, flip `state="..."` from `in-review` to
     `pending`, and append a single consolidated
     `<blockers>…</blockers>` element block to
     `journal/T-NNN.md` that aggregates all failing reviewers'
     feedback — not one `<blockers>` per reviewer, not a partial
     write. The block carries required attributes `date` and
     `round` (matching the round of the `<review>` blocks just
     appended) and has the form:

         <blockers date="2026-05-21T22:10:00Z" round="1">
         <one-line summary of what to change before the next
         implementer pass>.
         <optional bullets enumerating each persona's blocker>.
         </blockers>

   This serial write in the orchestrator turn eliminates the
   parallel-write race that would occur if each reviewer subagent
   wrote to the journal or TASKS.md directly (per DEC-008). Per-task
   journal files do not introduce parallel writes from reviewer
   subagents — the orchestrator remains the sole journal writer
   during review.

4. Exit. Do not pick up another `in-review` task. If the caller
   wants another task reviewed, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="pending"` (a retry), suggest
`{{ cmd_prefix }}speccy-work SPEC-NNNN`. If any remain
`state="in-review"`, suggest `{{ cmd_prefix }}speccy-review SPEC-NNNN`
again. If all tasks are `state="completed"`, suggest
`{{ cmd_prefix }}speccy-ship SPEC-NNNN`.
