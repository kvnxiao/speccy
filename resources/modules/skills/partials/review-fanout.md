
Fan out five reviewer-* sub-agents in parallel against the resolved
task, one per persona. Default fan-out: `reviewer-business`,
`reviewer-tests`, `reviewer-security`, `reviewer-style`,
`reviewer-correctness`. Two additional personas
(`reviewer-architecture`, `reviewer-docs`) are off the default
fan-out and are invoked explicitly when an architectural or
documentation risk is suspected.

The prompt for each spawn is:

> Review task `SPEC-NNNN/T-NNN`. Open your per-task context read with a
> single `speccy context SPEC-NNNN/T-NNN --json` call — the bundle
> carries the task entry, its covering requirements and scenarios, the
> full per-task journal (prior implementer handoffs, review verdicts,
> and blockers), the sibling index, the file paths, and a suggested
> merge-base diff command. Read the diff with that command, then apply
> your persona's review criteria. Targeted follow-up reads via the
> bundle's listed paths (e.g. the evidence file) remain legitimate
> where your persona needs something outside the bundle. Append your
> own `<review>` block to the
> per-task journal by running
> `speccy journal append SPEC-NNNN/T-NNN --block review --persona <persona> --verdict <pass|blocking> --model <your-model>`
> with the review body on stdin, then return a thin self-closing
> `<verdict persona="<persona>" verdict="..." model="..." rationale="..." />`
> element as your final message (per the verdict-return contract). The
> `--model` value is required and must identify the model that produced
> the verdict (with the optional slash-suffix effort convention from the
> verdict-return contract). Do not edit TASKS.md. The journal write goes
> through `speccy journal append` — do not use file-editing tools on the
> journal.
>
> The working tree may be dirty: the implementer leaves changes
> uncommitted on purpose, and the orchestrator (not the implementer)
> owns the single atomic commit on review pass per REQ-003/REQ-004.
> On retry rounds the dirty tree is the prior pass's WIP that the
> retry implementer amended in place per the retry-shape contract.
> Do not flag uncommitted state, commit timing, or "changes not
> committed before the in-review flip" -- those are out of scope
> for per-task review.

Substitute the resolved `SPEC-NNNN/T-NNN` and the persona name per
spawn.

{% if host == "claude-code" %}Invoke the `Task` tool five times **in one message** (parallel
dispatch) with `subagent_type: "reviewer-business"`,
`subagent_type: "reviewer-tests"`,
`subagent_type: "reviewer-security"`,
`subagent_type: "reviewer-style"`, and
`subagent_type: "reviewer-correctness"`. Each persona's agent
definition at `.claude/agents/reviewer-<persona>.md` carries the
host-native dispatch metadata.{% else %}Invoke Codex's native sub-agent-spawn primitive five times in
parallel against the registered Codex sub-agents
`reviewer-business`, `reviewer-tests`, `reviewer-security`,
`reviewer-style`, and `reviewer-correctness`. Each persona's TOML
file at `.codex/agents/reviewer-<persona>.toml` carries the
sub-agent's developer instructions.{% endif %}

Canonical journal `<review>` shape:
`{{ speccy_references_path }}/journal-review.md`.

Canonical journal `<blockers>` shape:
`{{ speccy_references_path }}/journal-blockers.md`.

Each reviewer sub-agent appends its own `<review>` block to
`.speccy/specs/NNNN-slug/journal/T-NNN.md` via `speccy journal
append --block review` before returning a thin `<verdict>` element
(see `resources/modules/personas/verdict_return_contract.md`). The
CLI's per-file lock serializes those parallel appends, so the
running session never transcribes `<review>` blocks itself and never
edits the journal with file-editing tools. The orchestrator's job
after the fan-out settles is to **verify completeness, read back any
blockers, then drive the state flip through the CLI verbs**.

### Step 1 — verify the round's reviews are complete

Read back the appended `<review>` blocks for the round under review
through the CLI rather than trusting the returned thin verdicts:

```bash
speccy journal show SPEC-NNNN/T-NNN --json --block review --round latest
```

Confirm every persona you spawned appears in the result for the
latest round. If a persona is missing (its append failed, or its
sub-agent errored before appending), halt the fan-out and surface
the missing persona rather than flipping state on an incomplete
round. Do not parse the journal file by hand — `journal show` is the
read-back authority.

### Step 2 — drive the state flip through `speccy task transition`

Decide pass vs blocking from the verdicts the reviewers appended,
then flip the task's `state` with the transition command — never by
editing the `state` attribute in TASKS.md directly:

- If every spawned reviewer's appended `<review verdict="...">` is
  `verdict="pass"`, flip `in-review` → `completed`:

      speccy task transition SPEC-NNNN/T-NNN --to completed

- If any spawned reviewer's `<review verdict="...">` is
  `verdict="blocking"`, flip `in-review` → `pending`:

      speccy task transition SPEC-NNNN/T-NNN --to pending

  then append a single consolidated `<blockers>` block (step 3).

### Step 3 — consolidate blockers via `speccy journal append`

On a blocking round, read back the failing reviews and write **one**
consolidated `<blockers>` block — not one per reviewer, not a partial
write. Read the blocking review bodies through the CLI:

```bash
speccy journal show SPEC-NNNN/T-NNN --json --verdict blocking --round latest
```

The `<blockers>` **body is orchestrator-authored semantic judgment**
(DEC-001 non-goal: the CLI never synthesizes blocker prose). Compose
the body from the blocking reviews you just read back, then append it
with the body on stdin:

```bash
speccy journal append SPEC-NNNN/T-NNN --block blockers <<'EOF'
<one-line summary of what to change before the next implementer pass>.
<optional bullets enumerating each persona's blocker>.
EOF
```

The CLI is the sole authority for the block's `date` and `round` and
emits the paired `<blockers>…</blockers>` element — **do not compute,
supply, or hand-author `date`, `round`, or the open/close tags**.
There is no flag to override them; the body you pipe is the inner
text only. Validation runs before any write; a malformed body leaves
the journal byte-identical.

The single-writer rule holds: the CLI's append lock owns write
serialization across the parallel reviewer appends and this
consolidated `<blockers>` append, and the orchestrator remains the
sole author of `<blockers>` bodies (and, per the commit step below,
of git commits). The running session issues only CLI verbs — `journal
show`, `journal append`, `task transition` — for the review-induced
journal and state writes; it never edits TASKS.md or the journal file
with file-editing tools.

### Atomic commit on review pass (REQ-003, REQ-004)

When every spawned reviewer returned `verdict="pass"` and the
`speccy task transition … --to completed` flip has run (the reviewer
`<review>` appends already landed via the CLI during the fan-out),
the running session performs the commit step:

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
  line where `<model>` is the model segment sourced per the
  "Sourcing your recorded identity" rule — the host's in-context
  identifier transcribed verbatim in hyphen form (e.g.
  `claude-opus-4-8[1m]`), never a dotted form or a configured alias.
  When the host states no resolved identifier in-context, use the
  documented fallback string
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
