
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

{% if host == "claude-code" %}Invoke the `Task` tool four times **in one message** (parallel
dispatch) with `subagent_type: "reviewer-business"`,
`subagent_type: "reviewer-tests"`,
`subagent_type: "reviewer-security"`, and
`subagent_type: "reviewer-style"`. Each persona's agent definition
at `.claude/agents/reviewer-<persona>.md` carries the host-native
dispatch metadata.{% else %}Invoke Codex's native sub-agent-spawn primitive four times in
parallel against the registered Codex sub-agents
`reviewer-business`, `reviewer-tests`, `reviewer-security`, and
`reviewer-style`. Each persona's TOML file at
`.codex/agents/reviewer-<persona>.toml` carries the sub-agent's
developer instructions.{% endif %}

Canonical journal `<review>` shape:
`{{ speccy_references_path }}/journal-review.md`.

Canonical journal `<blockers>` shape:
`{{ speccy_references_path }}/journal-blockers.md`.

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

### Atomic commit on review pass (REQ-003, REQ-004)

When every spawned reviewer returned `verdict="pass"` and the
journal append + TASKS.md flip to `completed` are written, the
running session performs the commit step:

1. Run `git status --porcelain`. If stdout is empty, **skip the
   commit step silently** (no surface to the user, no error). This
   handles two cases uniformly: tasks whose net filesystem change is
   zero, and idempotent re-entry from the reconcile pass against an
   already-converged state.
2. If stdout is non-empty, run `git add -A` followed by `git commit`
   with the message format below. The commit captures the
   implementer's code changes, the TASKS.md state flip, and the
   journal append in a single atomic commit (parent count = 1).

Commit message format (REQ-004):

- **Title:** `[SPEC-NNNN/T-NNN]: <task title>` — `<task title>` is
  read verbatim from the `<task>` element's `## ` heading in
  TASKS.md (the one-line H2 immediately after the `<task ...>`
  opening tag). Substitute the resolved spec and task IDs.
- **Body:** the trimmed content of the `Completed` field from the
  latest `<implementer>` block in the per-task journal file. Extract
  mechanically as the bytes between the `- Completed:` bullet marker
  and the next `- <Field>:` bullet marker (one of `Undone`,
  `Hygiene checks`, `Evidence`, `Discovered issues`,
  `Procedural compliance`). Trim leading and trailing whitespace.
- **Trailer:** a single `Co-Authored-By: <model> <noreply@anthropic.com>`
  line where `<model>` is sourced from the host harness's runtime
  model identifier (env var, runtime API, or host-specific
  equivalent). When the host does not expose a model identifier,
  use the documented fallback string
  `Co-Authored-By: Speccy Skill Pack <noreply@anthropic.com>`.

Pass the body via a HEREDOC so newlines and special characters
survive verbatim, e.g.:

```
git commit -m "$(cat <<'EOF'
[SPEC-NNNN/T-NNN]: <task title>

<trimmed Completed field>

Co-Authored-By: <model> <noreply@anthropic.com>
EOF
)"
```

The title prefix is the sole task-identity link in git history; the
consistency check correlates commits to tasks by grepping for this
prefix. Do not stage selectively — `git add -A` is sound under the
clean-tree precondition (REQ-002) that fires at the start of work
dispatch, which guarantees every dirty path at commit time is
task-scoped.

The skill body does not check the current git branch; it trusts the
caller / host to have placed the working tree on a feature branch.
Commits land on whatever HEAD is.
