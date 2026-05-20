
# {{ cmd_prefix }}speccy-review

Runs one round of adversarial review on one task per invocation and
exits. With an optional `[SPEC-NNNN/T-NNN]` selector argument, the
session reviews that specific task. Without an argument, the session
resolves the next reviewable task via
`speccy next --kind review --json` and reviews that one. Task state
lives in the `state` attribute on each `<task>` XML element in
TASKS.md.

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
     speccy next --kind review --json
     ```

     If the result is `kind: blocked` or empty, exit and report that
     no reviewable tasks remain. Otherwise, construct the
     disambiguated `<spec>/<task>` form from the JSON's `spec` and
     `task` fields (the bare `prompt_command` field is ambiguous
     across specs — every spec has its own `T-001`).

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
   message** as a `<review persona="..." verdict="...">…</review>`
   element block. Reviewers do not write to TASKS.md directly;
   they return their verdict to this orchestrator. After all spawned
   reviewers return, this orchestrator is the **sole writer to
   TASKS.md** for the review-induced state transition.

   {% if host == "claude-code" %}Invoke the `Task` tool four times in parallel, once per persona,
   with `subagent_type: "reviewer-business"`,
   `subagent_type: "reviewer-tests"`,
   `subagent_type: "reviewer-security"`, and
   `subagent_type: "reviewer-style"`. The prompt for each spawn is:

   > Run `speccy review SPEC-NNNN/T-NNN --persona <persona>` and
   > follow its output. Return your verdict as your final message
   > as a `<review persona="<persona>" verdict="...">…</review>`
   > element block. Do not edit TASKS.md.

   Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name
   into the command. Each subagent resolves to its markdown file at
   `.claude/agents/reviewer-<persona>.md`, so the persona body is
   already loaded for the sub-agent.{% else %}Prose-spawn the four reviewer subagents by name in parallel:
   `reviewer-business`, `reviewer-tests`, `reviewer-security`, and
   `reviewer-style`. The prompt for each spawn is:

   > Run `speccy review SPEC-NNNN/T-NNN --persona <persona>` and
   > follow its output. Return your verdict as your final message
   > as a `<review persona="<persona>" verdict="...">…</review>`
   > element block. Do not edit TASKS.md.

   Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name
   into the command. Codex resolves each name to its TOML file at
   `.codex/agents/reviewer-<persona>.toml`, so the persona body is
   already loaded as the sub-agent's developer instructions.{% endif %}

3. After all spawned sub-agents return, **consolidate** the
   `<review>` element blocks from each reviewer's final message into
   a single per-task verdict. Apply the state transition to
   **TASKS.md serially in this orchestrator turn** — do not
   delegate the write back to a reviewer subagent. Exit transition:

   - If every spawned reviewer's `<review verdict="...">` is
     `verdict="pass"`, flip the task's `state="..."` attribute
     from `in-review` to `completed` and append each
     `<review>` block to the task subtree.
   - If any spawned reviewer's `<review verdict="...">` is
     `verdict="blocking"`, flip `state="..."` from `in-review` to
     `pending`, append each `<review>` block to the task subtree,
     and append a single consolidated `<retry>…</retry>` element
     block that aggregates all failing reviewers' feedback — not
     one `<retry>` per reviewer, not a partial write. The block
     has the form:

         <retry>
         <one-line summary of what to change before the next
         implementer pass>.
         <optional bullets enumerating each persona's blocker>.
         </retry>

   This serial write in the orchestrator turn eliminates the
   parallel-write race that would occur if each reviewer subagent
   wrote to TASKS.md directly (per DEC-008).

4. Exit. Do not pick up another `in-review` task. If the caller
   wants another task reviewed, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="pending"` (a retry), suggest
`{{ cmd_prefix }}speccy-work SPEC-NNNN`. If any remain
`state="in-review"`, suggest `{{ cmd_prefix }}speccy-review SPEC-NNNN`
again. If all tasks are `state="completed"`, suggest
`{{ cmd_prefix }}speccy-ship SPEC-NNNN`.
