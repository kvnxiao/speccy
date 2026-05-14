---
name: speccy-review
description: Run Speccy's adversarial multi-persona review on every task awaiting review — business, tests, security, and style reviewers fan out in parallel and either pass the task or send it back with blocking notes. Use when the user says "review the implementation", "run the reviewers", "do a multi-persona review", or has just finished implementing and wants to validate.
---

# speccy-review

Drives the review loop. For each `[?]` task, the main agent
spawns one reviewer sub-agent per persona in parallel; collects their
inline notes; and flips the task to `[x]` (all pass) or back to `[ ]`
(any blocking, plus a `Retry:` note).

## When to use

After `speccy-work` has flipped tasks to `[?]`. Re-enter after retry
implementations complete.

## Steps

1. Query the CLI for the next reviewable task:

   ```bash
   speccy next --kind review --json
   ```

2. If the result is empty or `blocked`, exit the loop.
3. The JSON includes a `personas` array (default fan-out:
   `business`, `tests`, `security`, `style`).
4. Spawn the four reviewer sub-agents in parallel via the
   host-native subagent primitive. Each sub-agent appends exactly
   one inline note to the task in TASKS.md.

   Prose-spawn the four reviewer subagents by name in parallel:
   `reviewer-business`, `reviewer-tests`, `reviewer-security`, and
   `reviewer-style`. Codex resolves each name to its TOML file at
   `.codex/agents/reviewer-<persona>.toml`, so the persona body is
   already loaded as the sub-agent's developer instructions.

   Fallback for harnesses that do not recognise the subagent type:
   render the persona prompt to stdout with the existing CLI and
   splice the output into the spawned sub-agent's system prompt
   directly:

   ```bash
   speccy review T-003 --persona business
   speccy review T-003 --persona tests
   speccy review T-003 --persona security
   speccy review T-003 --persona style
   ```

5. After all four return, read the appended notes. If every persona
   wrote `pass`, flip `[?]` -> `[x]`. If any wrote `blocking`, flip
   `[?]` -> `[ ]` and append a `Retry: ...` note summarising the
   blockers.
6. Go back to step 1.

### Loop exit criteria

- `speccy next --kind review --json` returns empty.
- The user interrupts.

After exit, if any tasks are `[ ]` (retries), suggest `speccy-work
SPEC-NNNN` again. Otherwise suggest `speccy-ship SPEC-NNNN`.
