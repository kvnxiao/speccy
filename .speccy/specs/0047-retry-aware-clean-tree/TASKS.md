---
spec: SPEC-0047
spec_hash_at_generation: 0b139bf595e1ee043bf139a8ab2839809e16d769567228d386c14854ebe5454f
generated_at: 2026-05-26T20:27:02Z
---
# Tasks: SPEC-0047 Retry-aware clean-tree precondition — work dispatch tolerates dirty trees on review-blocked retries

<task id="T-001" state="pending" covers="REQ-001">
## Author the retry-shape canonical reference file

Create a new reference file that carries the canonical statement of
the retry-shape detection rule, mirroring the cross-cutting
references pattern already established by
`.claude/speccy-references/reconcile-policy.md` and
`.claude/speccy-references/journal-blockers.md`. The rule is read by
two skill bodies and one agent prompt (T-002, T-003, T-004 inline it
verbatim with marker comments), so it belongs at the cross-skill
location rather than nested under one skill's own `references/`
directory.

Place the canonical source under the templating pipeline so the
host-portable mirrors (`.claude/speccy-references/retry-shape.md`
for the Claude pack, `.agents/speccy-references/retry-shape.md` for
the Agents pack, plus any Codex equivalent) are produced by the
existing `resources/modules/...` → host fan-out. Use whatever
mechanism the reconcile-policy partial already uses; do not invent a
new sync mechanism.

The file body documents:

- **Rule statement.** Verbatim copy of the REQ-001 rule text from
  SPEC.md (a task `T-NNN` is in retry shape iff the journal file
  exists, contains ≥1 `<implementer>` block, and contains a
  `<blockers>` block whose `round` attribute equals the highest
  `round` on any `<implementer>` block; otherwise it is in
  first-attempt shape).
- **Read-only scope.** The rule reads only `journal/T-NNN.md`. It
  does not read TASKS.md, does not invoke `git`, does not call
  `speccy next`, and does not invoke any other CLI subcommand.
- **Worked example 1 — retry shape.** A short journal snippet
  showing `<implementer round="1">` followed by
  `<blockers round="1">`. State explicitly that applying the rule
  to this journal yields retry shape (the dirty tree from the
  round-1 implementer is the WIP the round-2 implementer amends in
  place).
- **Worked example 2 — first-attempt shape.** A short journal
  snippet showing only `<implementer round="1">` and no
  `<blockers>` block. State explicitly that the rule yields
  first-attempt shape (the strict clean-tree gate applies).
- **Edge case — implementer awaiting review.** A short journal
  snippet showing two completed rounds (each with implementer +
  blockers) and a trailing `<implementer round="3">` block with no
  `<blockers round="3">`. State that the rule yields first-attempt
  shape because the highest implementer round has no matching
  blockers — the task is awaiting review, not awaiting a retry.

The file uses the same convention as `.claude/speccy-references/`
partials: no YAML frontmatter; plain Markdown. Do not introduce a
`speccy verify` lint rule for this file; the rule is enforced by
reviewer judgment on the three inlined copies in T-002/T-003/T-004,
not by the CLI.

<task-scenarios>
Given the canonical reference file after this task lands,
when grepped for the literal phrase
`highest \`round\` attribute on any \`<implementer>\` block`,
then exactly one match is found in the rule statement section
(covers CHK-001/CHK-002/CHK-003 as the source-of-truth statement
that the three inlined copies in T-002/T-003/T-004 will be checked
against).

Given the same file,
when scanned for the worked examples,
then it contains exactly three example journal snippets labelled
as retry shape, first-attempt shape, and the awaiting-review edge
case respectively.

Given the post-task workspace,
when the host-portable mirror paths are inspected (the Claude pack
location, the Agents pack location, and any Codex equivalent
documented in `resources/modules/`),
then each mirror exists and its content is byte-identical to the
canonical source under `resources/modules/` after the templating
pipeline runs.

Suggested files: `resources/modules/references/retry-shape.md`
(new canonical source), `.claude/speccy-references/retry-shape.md`
(new Claude mirror), `.agents/speccy-references/retry-shape.md`
(new Agents mirror), plus the resource fan-out manifest if one
exists.
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-002">
## Make `/speccy-work` skill body's entry precondition retry-aware

Edit `.claude/skills/speccy-work/SKILL.md` (and the mirrored
`.agents/skills/speccy-work/SKILL.md` plus any `resources/agents/...`
template source) to extend the existing SPEC-0045/REQ-002 entry
precondition with retry-shape awareness.

Today's entry precondition runs `git status --porcelain` and exits
on non-empty stdout. The new precondition runs three steps in order:

1. Resolve the target task per the existing step 1 of the agent
   recipe (selector argument or `speccy next --json`).
2. Read `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the
   REQ-001 retry-shape rule.
3. Run `git status --porcelain`. If first-attempt shape and stdout
   is non-empty, exit with the dirty-paths surface (today's
   behaviour). If retry shape, proceed regardless of stdout. If
   first-attempt shape and stdout is empty, proceed.

Inline the REQ-001 rule text verbatim at the precondition step,
bounded by the marker comment pair:

```
<!-- Shared rule: retry-shape. Source: .claude/speccy-references/retry-shape.md -->
<rule text from T-001's reference file>
<!-- End shared rule: retry-shape. -->
```

The rule text between the markers must be byte-for-byte identical
(after whitespace normalisation) to T-001's canonical source.

Keep the existing reconcile-policy partial inline at its current
location. The retry-shape rule and the reconcile-policy partial are
two separate inlines, each bounded by its own marker comments.

The mirrored `.agents/skills/speccy-work/SKILL.md` and the
`resources/agents/...` template source must stay in sync via the
existing pipeline; do not edit only one of the three locations and
leave the others stale.

<task-scenarios>
Given the skill body `.claude/skills/speccy-work/SKILL.md` after
this task,
when grepped for the open marker comment
`<!-- Shared rule: retry-shape.`,
then exactly one match is found,
and the content between the open and close markers is byte-for-byte
identical (after whitespace normalisation) to the T-001 canonical
reference file (covers CHK-004 path / CHK-005 path through
documented rule).

Given the same file,
when a reader traces the entry precondition prose,
then it documents the two branches explicitly: first-attempt shape
keeps the existing strict gate (non-empty `git status --porcelain`
halts the skill with the dirty-paths surface), retry shape permits
a dirty tree and proceeds to dispatch the implementer (covers
CHK-004 first-attempt branch, CHK-005 retry branch).

Given the mirrored `.agents/skills/speccy-work/SKILL.md`,
when its entry precondition prose is compared to the Claude
mirror,
then the retry-shape rule and the two-branch documentation appear
verbatim (modulo any host-specific wording the templating pipeline
substitutes).

Suggested files: `.claude/skills/speccy-work/SKILL.md`,
`.agents/skills/speccy-work/SKILL.md`,
`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`,
`resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`.
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-002">
## Make `/speccy-orchestrate` work dispatch retry-aware

Edit `.claude/skills/speccy-orchestrate/SKILL.md` (and the mirrored
`.agents/skills/speccy-orchestrate/SKILL.md` plus the
`resources/agents/...` template source) to extend the work dispatch
section's clean-tree precondition with retry-shape awareness.

Today the work dispatch section runs `git status --porcelain` in the
orchestrator's running session before spawning the speccy-work
sub-agent and halts the outer loop on non-empty stdout. The new
precondition runs three steps in the running session:

1. Resolve the target task from `next_action.task_id`.
2. Read `<spec-dir>/journal/T-NNN.md` (if it exists) and apply the
   REQ-001 retry-shape rule.
3. Run `git status --porcelain`. If first-attempt shape and stdout
   is non-empty, halt the outer loop with the dirty-paths surface
   (today's behaviour). If retry shape, proceed to spawn the
   speccy-work sub-agent regardless of stdout. If first-attempt
   shape and stdout is empty, proceed.

Inline the REQ-001 rule text verbatim at the work dispatch
precondition, bounded by the same marker comment pair used in
T-002:

```
<!-- Shared rule: retry-shape. Source: .claude/speccy-references/retry-shape.md -->
<rule text from T-001's reference file>
<!-- End shared rule: retry-shape. -->
```

The rule text between the markers must be byte-for-byte identical
(after whitespace normalisation) to T-001's canonical source and
to the inline in T-002.

Update the orchestrator's status-reporting prose if the retry case
should be visible to the user. For example, the current
`SPEC-NNNN → work T-003 (retry 2/5 after blocking review)` status
line already names the retry case; extend the surrounding prose
explaining that the retry dispatch proceeds despite a dirty tree
when the journal shows the retry shape. This is documentation
inside the skill body; no new status line shape is required.

Keep the existing reconcile-policy partial inline at the startup
integrity check section unchanged. The retry-shape inline at the
work dispatch is a separate site.

<task-scenarios>
Given the skill body `.claude/skills/speccy-orchestrate/SKILL.md`
after this task,
when grepped for the open marker comment
`<!-- Shared rule: retry-shape.`,
then exactly one match is found,
and the content between the open and close markers is byte-for-byte
identical to T-001's canonical reference file and to the T-002
inline.

Given a workspace with TASKS.md T-001 at `state="pending"`, a
journal file containing `<implementer round="1">` +
`<blockers round="1">`, a dirty working tree, and
`speccy next SPEC-NNNN --json` returning
`next_action.kind == "work"` with `next_action.task_id == "T-001"`,
when a reader traces the work dispatch prose,
then the documented behaviour is to proceed and spawn the
speccy-work sub-agent without halting the outer loop, and no
dirty-paths surface is written (covers CHK-006).

Given the mirrored `.agents/skills/speccy-orchestrate/SKILL.md`,
when its work dispatch prose is compared to the Claude mirror,
then the retry-shape rule and the retry-aware precondition prose
appear verbatim.

Suggested files: `.claude/skills/speccy-orchestrate/SKILL.md`,
`.agents/skills/speccy-orchestrate/SKILL.md`,
`resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl`,
`resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl`.
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-003">
## Add retry-aware mode to the speccy-work agent prompt

Edit `.claude/agents/speccy-work.md` (and the Codex variant at
`.codex/agents/speccy-work.toml` plus any `resources/agents/...`
template source) to make the implementer prompt retry-aware.

Today the agent prompt's Steps section flows: resolve target task
→ flip state to `in-progress` → read scenarios → implement → run
hygiene → flip to `in-review` → append `<implementer>` block. The
extended flow inserts a retry-shape check between resolving the
target and flipping state, branching the rest of the recipe:

1. Resolve the target task (existing step 1).
2. **New step:** Read `<spec-dir>/journal/T-NNN.md` (if it exists)
   and apply the REQ-001 retry-shape rule.
3. If first-attempt shape, proceed with today's flow: flip state to
   `in-progress`, read scenarios, implement, hygiene-gate, flip to
   `in-review`, append `<implementer round="1">`.
4. If retry shape, enter retry mode:
   - Read the most recent `<implementer>` block to understand the
     prior pass's stated `Completed` work.
   - Read the latest `<blockers>` block (the one whose `round`
     matches the highest implementer round) for the specific
     feedback to address.
   - Amend the existing WIP in the working tree to address the
     blockers — do not run `git restore` or `git clean`, do not
     rewrite files from scratch, do not reset state. The dirty
     tree is the prior pass's WIP; iterate on it in place.
   - Flip state to `in-progress` and proceed through hygiene-gate
     and the `in-review` flip exactly as the first-attempt branch
     does (the SPEC-0045/REQ-001 hygiene gate runs unchanged).
   - Append the next `<implementer round="N+1">` block where `N`
     is the highest prior implementer round, monotonically
     incremented by exactly 1.
   - The retry-mode `Completed` field describes the amend (what
     changed in this round), not the cumulative task work.

Inline the REQ-001 rule text verbatim at the new step 2, bounded by
the same marker comment pair used in T-002 and T-003:

```
<!-- Shared rule: retry-shape. Source: .claude/speccy-references/retry-shape.md -->
<rule text from T-001's reference file>
<!-- End shared rule: retry-shape. -->
```

The rule text between the markers must be byte-for-byte identical
(after whitespace normalisation) to T-001's canonical source and
to the inlines in T-002 and T-003.

Update the "When to use" prose to mention that the agent
automatically detects retry shape from the journal and switches
modes; the caller does not pass a flag.

The six-field handoff template (`Completed`, `Undone`,
`Hygiene checks`, `Evidence`, `Discovered issues`,
`Procedural compliance`) stays unchanged. The CHK-by-CHK Evidence
roll-call convention stays unchanged. Only the prose around step 2
grows the retry-aware branch.

<task-scenarios>
Given the agent prompt `.claude/agents/speccy-work.md` after this
task,
when grepped for the open marker comment
`<!-- Shared rule: retry-shape.`,
then exactly one match is found,
and the content between the open and close markers is byte-for-byte
identical to T-001's canonical reference file and to the T-002 and
T-003 inlines.

Given the same file,
when a reader traces the Steps section,
then the prose documents the two branches: first-attempt mode runs
today's recipe unchanged, retry mode reads the latest `<blockers>`
and the most recent `<implementer>`, amends the WIP in place
without resetting the tree, and appends `<implementer round="N+1">`
with `N+1` derived from the highest prior round (covers CHK-007,
CHK-008).

Given the same file,
when scanned for any instruction to run `git restore`, `git clean`,
or `git checkout` inside the retry-mode branch,
then zero matches are found (the retry-mode implementer never
discards the prior pass's WIP).

Given the Codex variant at `.codex/agents/speccy-work.toml`,
when its prompt body is compared to the Claude agent prompt,
then the retry-shape rule and the retry-mode branch appear
verbatim (modulo any host-specific wording the templating pipeline
substitutes).

Suggested files: `.claude/agents/speccy-work.md`,
`.codex/agents/speccy-work.toml`,
`resources/agents/.claude/agents/speccy-work.md.tmpl`,
`resources/agents/.codex/agents/speccy-work.toml.tmpl`.
</task-scenarios>
</task>

<task id="T-005" state="pending" covers="REQ-004">
## Add bootstrap commit step to `/speccy-decompose`

Edit the `/speccy-decompose` skill body, the speccy-decompose agent
prompt, and their host-portable mirrors / template sources to add
the REQ-004 bootstrap commit step as the final step before
returning.

Today the agent prompt's Steps section flows: resolve SPEC.md →
write TASKS.md → `speccy lock SPEC-NNNN` → suggest next step. The
new flow inserts the commit step between `speccy lock` and the
"Suggest the next step" line:

1. After `speccy lock SPEC-NNNN` runs successfully, stage exactly
   the two SPEC artefacts via narrow `git add`:

   ```
   git add <spec-dir>/SPEC.md <spec-dir>/TASKS.md
   ```

   Do not use `git add -A` or `git add .`. Staging unchanged
   content is a no-op, so passing both paths unconditionally is
   safe regardless of whether SPEC.md was already committed.

2. Run `git diff --cached --quiet`. If exit code is 0 (nothing
   staged), skip the commit silently — both files are already
   committed at their current content. If non-zero, proceed.

3. Build the commit message:

   - Title: `[SPEC-NNNN]: create spec and decompose tasks`.
     Substitute `SPEC-NNNN` with the resolved spec id.
   - Body: the value of the `title:` field from SPEC.md's YAML
     frontmatter, trimmed (the one-line title slug, not the
     full document heading).
   - Trailer: a single `Co-Authored-By: <model>
     <noreply@anthropic.com>` line where `<model>` is sourced from
     the host harness's runtime model identifier (env var, runtime
     API, or host-specific equivalent). When the host exposes no
     model identifier, fall back to the documented
     `Co-Authored-By: Speccy Skill Pack <noreply@anthropic.com>`
     string. Match SPEC-0045/REQ-004's trailer resolution
     verbatim.

4. Pass the body via a HEREDOC so newlines and any special
   characters in the SPEC title survive verbatim, e.g.:

   ```
   git commit -m "$(cat <<'EOF'
   [SPEC-NNNN]: create spec and decompose tasks

   <SPEC title from frontmatter>

   Co-Authored-By: <model> <noreply@anthropic.com>
   EOF
   )"
   ```

Update both reader sites identically:

- `.claude/skills/speccy-decompose/SKILL.md` — the skill body
  (today this is a thin wrapper that defers to the agent prompt;
  if the wrapper does not document steps, document the new step
  there only if the existing skill body documents the recipe).
- `.claude/agents/speccy-decompose.md` — insert the new step
  between today's step 3 (`speccy lock`) and step 4 ("Suggest the
  next step"). Renumber the suggest-next-step line accordingly.

Mirror the change to host-portable copies under
`.agents/skills/speccy-decompose/SKILL.md`, the Codex agent
variant if one exists, and any `resources/agents/...` /
`resources/modules/...` template sources, via the existing
templating pipeline. Do not edit only one of these locations and
leave the others stale.

Document the step's idempotency (re-running decompose on an
already-committed SPEC produces no new commit) and the narrow
staging scope (unrelated dirty paths outside `<spec-dir>/` are
not swept in) inline in the agent prompt prose, so a future
reader can trace the design intent without re-reading SPEC-0047.

<task-scenarios>
Given the agent prompt `.claude/agents/speccy-decompose.md` after
this task,
when a reader traces the Steps section,
then a new step appears between today's step 3 (`speccy lock
SPEC-NNNN`) and the "Suggest the next step" line, documenting
the three-step bootstrap commit (narrow stage, diff check,
HEREDOC commit) per REQ-004 (covers CHK-009 commit-shape and
CHK-011 idempotency-skip).

Given the same file,
when grepped for `git add -A` or `git add .` inside the new
bootstrap commit step,
then zero matches are found (the step uses narrow file-list
staging only — covers CHK-010 narrow-staging scope).

Given the same file,
when scanned for the commit message title format,
then the literal string `[SPEC-NNNN]: create spec and decompose
tasks` appears (with `SPEC-NNNN` either as the literal
placeholder for runtime substitution or substituted at runtime).

Given the same file,
when scanned for the commit message body source,
then the prose names the SPEC's `title:` frontmatter field as the
body source (trimmed of leading/trailing whitespace).

Given the same file,
when scanned for the `Co-Authored-By` trailer resolution,
then the prose names the host-harness model identifier as the
primary source and the `Speccy Skill Pack` literal as the
fallback when no host model is exposed (matching SPEC-0045/REQ-004
verbatim).

Given the skill body `.claude/skills/speccy-decompose/SKILL.md`,
when its body is compared to the agent prompt,
then either (a) the skill body is a thin wrapper that defers to
the agent prompt unchanged, in which case no edits are required,
or (b) the skill body documents the same step inline; pick the
path that matches the existing wrapper convention and apply it
consistently.

Given the mirrored `.agents/skills/speccy-decompose/SKILL.md`
(and any Codex variant of the agent prompt),
when its bootstrap commit step prose is compared to the Claude
mirror,
then the step appears verbatim (modulo any host-specific wording
the templating pipeline substitutes).

Suggested files: `.claude/skills/speccy-decompose/SKILL.md`,
`.claude/agents/speccy-decompose.md`,
`.agents/skills/speccy-decompose/SKILL.md`,
`.codex/agents/speccy-decompose.toml` (if it exists),
`resources/agents/.claude/skills/speccy-decompose/SKILL.md.tmpl`,
`resources/agents/.claude/agents/speccy-decompose.md.tmpl`,
`resources/agents/.agents/skills/speccy-decompose/SKILL.md.tmpl`.
</task-scenarios>
</task>
