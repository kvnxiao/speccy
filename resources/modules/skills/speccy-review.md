
# {{ cmd_prefix }}speccy-review

Drives the review loop. For each `[?]` task, the main agent
spawns one reviewer sub-agent per persona in parallel; collects their
inline notes; and flips the task to `[x]` (all pass) or back to `[ ]`
(any blocking, plus a `Retry:` note).

## When to use

After `{{ cmd_prefix }}speccy-work` has flipped tasks to `[?]`. Re-enter after retry
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

   {% if host == "claude-code" %}Invoke the `Task` tool four times in parallel, once per persona,
   with `subagent_type: "reviewer-business"`,
   `subagent_type: "reviewer-tests"`,
   `subagent_type: "reviewer-security"`, and
   `subagent_type: "reviewer-style"`. Each subagent resolves to its
   markdown file at `.claude/agents/reviewer-<persona>.md`, so the
   persona body is already loaded for the sub-agent.{% else %}Prose-spawn the four reviewer subagents by name in parallel:
   `reviewer-business`, `reviewer-tests`, `reviewer-security`, and
   `reviewer-style`. Codex resolves each name to its TOML file at
   `.codex/agents/reviewer-<persona>.toml`, so the persona body is
   already loaded as the sub-agent's developer instructions.{% endif %}

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

After exit, if any tasks are `[ ]` (retries), suggest `{{ cmd_prefix }}speccy-work
SPEC-NNNN` again. Otherwise suggest `{{ cmd_prefix }}speccy-ship SPEC-NNNN`.
