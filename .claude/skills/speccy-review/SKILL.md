---
name: speccy-review
description: Review one Speccy task per invocation and exit, running one round of adversarial multi-persona review. With an optional `SPEC-NNNN/T-NNN` selector, the session reviews that task; without it, the skill resolves the next reviewable task via `speccy next --kind review --json`. Four personas (business, tests, security, style) fan out in parallel and either pass the task to `completed` or flip it back to `pending` with a `Retry:` note. Use when the user says "review T-003" or "review the next task".
---

# /speccy-review

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

- With a selector (`/speccy-review SPEC-0007/T-003`):
  when the task to review is already known.
- Without an argument: when picking up wherever `TASKS.md` left off.
  The session reviews one task and exits.

The target task must already be in `state="in-review"` (typically
flipped there by `/speccy-work`).

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
   sub-agent primitive, one per persona in the default fan-out:
   `business`, `tests`, `security`, `style`. Each sub-agent's prompt
   is the bash command form below, not the CLI-rendered prompt text
   inlined into the spawn call. The CLI command remains the source
   of truth for what each persona reads.

   Invoke the `Task` tool four times in parallel, once per persona,
   with `subagent_type: "reviewer-business"`,
   `subagent_type: "reviewer-tests"`,
   `subagent_type: "reviewer-security"`, and
   `subagent_type: "reviewer-style"`. The prompt for each spawn is:

   > Run `speccy review SPEC-NNNN/T-NNN --persona <persona>` and
   > follow its output. Your only deliverable is a single inline
   > note appended to TASKS.md.

   Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name
   into the command. Each subagent resolves to its markdown file at
   `.claude/agents/reviewer-<persona>.md`, so the persona body is
   already loaded for the sub-agent.

3. After all four sub-agents return, aggregate the four inline notes
   they appended to the task subtree. Exit transition:

   - If every persona note is `pass`, flip the task's `state="..."`
     attribute from `in-review` to `completed`.
   - If any persona note is `blocking`, flip `state="..."` from
     `in-review` to `pending` and append a `Retry: ...` bullet to
     the task subtree summarising the blockers.

4. Exit. Do not pick up another `in-review` task. If the caller
   wants another task reviewed, the caller invokes this skill again.

After exit, the next reasonable step depends on TASKS.md state: if
any task is `state="pending"` (a retry), suggest
`/speccy-work SPEC-NNNN`. If any remain
`state="in-review"`, suggest `/speccy-review SPEC-NNNN`
again. If all tasks are `state="completed"`, suggest
`/speccy-ship SPEC-NNNN`.
