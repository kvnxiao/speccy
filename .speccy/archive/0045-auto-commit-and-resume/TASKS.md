---
spec: SPEC-0045
spec_hash_at_generation: 9b724b49981abdd71bfd9808ca2dd0d683e2ca1099c9fe9b5cdeaf89e2d9fa0a
generated_at: 2026-05-25T23:51:51Z
---
# Tasks: SPEC-0045 Auto-commit on review pass + autonomous crash-resume

<task id="T-001" state="completed" covers="REQ-008">
## Author the reconcile-policy shared partial

Create the file `.claude/speccy-references/reconcile-policy.md` that
will serve as the single source of truth for the reconcile dispatch
policy. The partial must document:

- The dispatch trigger: when `speccy next --json` returns
  `next_action.kind == "reconcile"`, the calling skill dispatches
  to a reconcile pass.
- A Markdown table mapping each of the four drift kinds
  (`commit_without_state`, `state_completed_no_commit`,
  `state_in_progress_orphaned`, `journal_xml_malformed`) to its
  policy action.
- The three properties: autonomous (no user prompts, no forks),
  rollback-biased (ambiguous recovery prefers backward rollback),
  idempotent (each action is a no-op on already-converged state).
- The post-dispatch re-query discipline: after applying actions for
  all drifts, re-query `speccy next --json`; if
  `consistency.status == "ok"`, resume normal dispatch; if still
  drifting, apply actions again.
- An "Extending the enum" section documenting the two-change
  procedure for adding a new drift kind: (1) add CLI detection in
  the Rust source, (2) add a row to this partial's policy table
  and re-sync all three inlined copies.

The policy table must cover the per-case action details from REQ-007:

| `kind` | `severity` | Action |
|---|---|---|
| `commit_without_state` | `auto_fixable` | Edit TASKS.md: flip task state to `completed`. |
| `state_completed_no_commit` (dirty tree, `working_tree_dirty=true`) | `blocking` | `git add -A && git commit` with REQ-004 message reconstructed from journal + TASKS.md. |
| `state_completed_no_commit` (clean tree, `working_tree_dirty=false`) | `blocking` | Edit TASKS.md: roll task state back to `in-review`. Journal preserved. |
| `state_in_progress_orphaned` | `blocking` | `git restore .`, `git clean -fd`, edit TASKS.md to flip state to `pending`. |
| `journal_xml_malformed` | `blocking` | Truncate journal file to `details.last_well_formed_byte_offset` bytes; reset TASKS.md state to whatever the truncated journal implies. |

The file follows the same convention as existing `.claude/speccy-references/`
partials (no YAML frontmatter; plain Markdown). It will be inlined into
three skill bodies in T-002, T-003, and T-004.

<task-scenarios>
Given the file `.claude/speccy-references/reconcile-policy.md` after
this task lands,
when grepped for each of the four drift kind names
(`commit_without_state`, `state_completed_no_commit`,
`state_in_progress_orphaned`, `journal_xml_malformed`),
then each name appears at least once in the file (covers CHK-017).

Given the same file,
when a reader scans the "Extending the enum" section,
then the prose names two required change sites: the CLI detection code
and the policy table in this partial.

Given the same file,
when a reader scans the dispatch trigger section,
then it explicitly names `next_action.kind == "reconcile"` as the
condition that activates the reconcile pass.

Suggested files: `.claude/speccy-references/reconcile-policy.md` (new)
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-002 REQ-008">
## Loosen thin-stub test, then add hygiene gate, clean-tree precondition, and reconcile-partial inline to speccy-work/SKILL.md

This task has four sub-steps. Step 0 must land before the three skill-body
edits, because the unmodified `assert_thin_stub_body` test in
`speccy-cli/tests/init.rs` (inherited from archived SPEC-0044/REQ-010)
will fail the moment the reconcile partial is inlined into the SKILL.md
body (the inline is ~95 lines and the cap is `< 12 non-empty lines`).

**0. Loosen `assert_thin_stub_body` (REQ-008, CHK-019).** Edit
`speccy-cli/tests/init.rs` so the helper excludes lines that fall
between the `<!-- Shared partial: reconcile-policy.` open marker
comment and the `<!-- End shared partial: reconcile-policy. -->` close
marker comment from its non-empty-line count. The existing assertion
that the body must reference the agent path and the `/agent speccy-<phase>`
invocation pointer remains unchanged. The intent of the original cap
(catch full-body prose leakage back into the stub) is preserved for
lines outside the marker-bounded region; the marker-bounded region is
an explicit, auditable exemption documented in REQ-008. Verify with
`cargo test --workspace -- t007_init_renders_claude_code_pin_assignments_matching_dogfood_pack t007_init_renders_codex_pin_assignments_matching_dogfood_pack` — both must continue to pass before and after the step 1-3 edits land.

**1. Clean-tree precondition (REQ-002).** At the entry of the skill,
before spawning the implementer sub-agent, insert a step that runs
`git status --porcelain`. If the output is non-empty, the skill exits
without spawning any Task tool, and surfaces the dirty paths to the
user. The step must be documented in the skill body explicitly so
future authors see it as a requirement, not a convention. A clean
working tree (`git status --porcelain` exits with empty stdout) allows
normal dispatch.

**2. Hygiene gate (REQ-001).** After the implementer sub-agent
completes its work and before the state flip from `in-progress` to
`in-review`, the skill body must document that the standard hygiene
suite runs: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check`. If any exits non-zero, the state
flip is refused. The `Hygiene checks` field in the `<implementer>`
block's six-field handoff template receives one line per gate naming
its exit code. This must be written as an explicit gate in the skill
prose, not merely referenced via `AGENTS.md`.

**3. Reconcile-partial inline (REQ-008).** Inline the content of
`.claude/speccy-references/reconcile-policy.md` into the skill body
at the entry check, bounded by marker comments:

```
<!-- Shared partial: reconcile-policy. Source: .claude/speccy-references/reconcile-policy.md -->
<partial content here>
<!-- End shared partial: reconcile-policy. -->
```

The inline fires at the entry of the skill (after the clean-tree
check) so that if `speccy next` reports `next_action.kind ==
"reconcile"`, the skill dispatches the reconcile pass rather than
normal task work.

<task-scenarios>
Given the skill body `.claude/skills/speccy-work/SKILL.md` after this
task, when a reader scans it for the phrase `git status --porcelain`,
then exactly one occurrence appears in the entry check prose and it
is framed as a precondition that exits the skill on non-empty output
(covers CHK-003, CHK-004).

Given the same file, when grepped for `Hygiene checks`,
then at least one occurrence appears in the implementer gate prose
and the four hygiene commands (`cargo test`, `cargo clippy`,
`cargo +nightly fmt`, `cargo deny`) are all named (covers CHK-001,
CHK-002).

Given the same file, when searched for the open marker comment
`<!-- Shared partial: reconcile-policy.`,
then exactly one match is found, and the content between the open
and close markers matches the body of
`.claude/speccy-references/reconcile-policy.md` when normalized
for surrounding whitespace (covers CHK-018).

Given `speccy-cli/tests/init.rs` after this task,
when `assert_thin_stub_body` is read,
then lines between the `<!-- Shared partial: reconcile-policy.` open
marker and the `<!-- End shared partial: reconcile-policy. -->` close
marker are excluded from its non-empty-line count, and
`cargo test --workspace -- t007_init_renders_claude_code_pin_assignments_matching_dogfood_pack t007_init_renders_codex_pin_assignments_matching_dogfood_pack`
exits 0 against the post-task SKILL.md bodies (covers CHK-019).

Suggested files: `.claude/skills/speccy-work/SKILL.md`,
`.agents/skills/speccy-work/SKILL.md`,
`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`,
`resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`,
`speccy-cli/tests/init.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-003 REQ-004 REQ-007 REQ-008">
## Add commit step, commit message format, and reconcile-partial inline to speccy-review/SKILL.md

Edit `.claude/skills/speccy-review/SKILL.md` with two additions:

**1. Commit step + message format (REQ-003, REQ-004).** After all
reviewer personas return `verdict="pass"` and before the skill exits,
insert the three-step commit procedure:

1. Append the consolidated `<review>` blocks to the per-task journal
   file (`.speccy/specs/NNNN-slug/journal/T-NNN.md`).
2. Flip the task's `state` attribute in TASKS.md from `in-review` to
   `completed`.
3. Run `git status --porcelain`. If non-empty: run `git add -A`
   followed by `git commit` with the message format below. If empty:
   skip the commit silently (idempotent re-run path).

Commit message format (REQ-004):
- **Title:** `[SPEC-NNNN/T-NNN]: <task title>` — task title read
  verbatim from the `<task>` element in TASKS.md.
- **Body:** trimmed content of the `Completed` field from the latest
  `<implementer>` block in `journal/T-NNN.md` (bytes between
  `- Completed:` and the next `- <Field>:` bullet marker).
- **Trailer:** `Co-Authored-By: <model> <noreply@anthropic.com>` where
  `<model>` is sourced from the host harness's runtime model-
  identification mechanism (env var or equivalent). Fallback when
  unavailable: `Co-Authored-By: Speccy Skill Pack <noreply@anthropic.com>`.

The step must be documented as three numbered sub-steps in the skill
prose. The conditional skip at step 3 must be called out explicitly
("if the working tree is already clean, skip the commit — this handles
idempotent re-runs").

**2. Reconcile-partial inline (REQ-007, REQ-008).** Inline the content
of `.claude/speccy-references/reconcile-policy.md` at the skill entry,
bounded by the standard marker comments naming the shared partial.

<task-scenarios>
Given a `state="in-review"` task whose four reviewer personas all
return `verdict="pass"`, when the skill completes the journal append,
TASKS.md flip, and commit step, then the commit produced is a single
non-merge commit containing the code files, TASKS.md, and the journal
file (`git log -1 --format=%P` returns one parent SHA), and the
working tree is clean after (covers CHK-005).

Given the workspace immediately after the above commit (working tree
clean, task `state="completed"`), when the commit step is re-invoked,
then no new commit is produced (`git log -1 --format=%H` unchanged)
(covers CHK-006).

Given a commit produced from this code path for task T-003 with title
`Conditional atomic commit after review pass`, when `git log -1
--format='%s'` runs, then stdout is exactly
`[SPEC-0045/T-003]: Conditional atomic commit after review pass`
(covers CHK-007).

Given the same commit, when `git log -1 --format='%b'` runs,
then stdout equals the trimmed `Completed` field of the latest
`<implementer>` block in `journal/T-003.md`, and
`git log -1 --format='%(trailers:key=Co-Authored-By,valueonly)'`
returns a non-empty value (covers CHK-008).

Given the skill body after this task, when searched for the open
marker comment `<!-- Shared partial: reconcile-policy.`,
then exactly one match is found and the inlined content matches the
partial source file (covers CHK-018).

Suggested files: `.claude/skills/speccy-review/SKILL.md`
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-002 REQ-007 REQ-008">
## Wire clean-tree check and reconcile-partial inline into speccy-orchestrate/SKILL.md

Edit `.claude/skills/speccy-orchestrate/SKILL.md` with two additions:

**1. Clean-tree check in work dispatch (REQ-002).** In the
orchestrator's work dispatch loop — the section that spawns the
speccy-work sub-agent for the next pending task — prepend a check
that runs `git status --porcelain`. If the output is non-empty, the
orchestrator's outer loop halts and surfaces the dirty paths. No
implementer sub-agent is spawned. This check must be documented in the
dispatch section explicitly.

**2. Reconcile-partial inline (REQ-007, REQ-008).** Inline the content
of `.claude/speccy-references/reconcile-policy.md` in two locations
within the skill body:

- At the startup integrity check (the section that runs `speccy next`
  at the beginning of an orchestrate session to assess what to do).
- At the entry of each loop iteration (before dispatching work or
  review), so that if a per-task operation leaves drift, the next
  loop iteration's `speccy next` call catches it and dispatches the
  reconcile pass before continuing.

Both inline sites use the standard marker comments. If the partial is
logically the same block re-used at two points in the file, it is
sufficient to inline once at the startup check and describe the
loop-iteration re-query in the surrounding prose (rather than
duplicating the full partial twice in one file) — use your judgment on
clarity and file size.

After both additions, verify no `AskUserQuestion` invocation (or
equivalent "press enter to continue" surface) appears anywhere in the
reconcile dispatch path within the skill body.

<task-scenarios>
Given a working tree with one unstaged change to any tracked file,
when `/speccy-orchestrate`'s work dispatch step runs (simulated by
reading the skill body and tracing the dispatch path),
then the prose documents that the outer loop halts with the dirty-paths
surface before spawning any speccy-work sub-agent (covers CHK-003 for
the orchestrate dispatch path).

Given the skill body `.claude/skills/speccy-orchestrate/SKILL.md`
after this task, when searched for the open marker comment
`<!-- Shared partial: reconcile-policy.`,
then exactly one match is found and the inlined content matches
`.claude/speccy-references/reconcile-policy.md` (covers CHK-018).

Given the same file, when scanned for `AskUserQuestion` or any
interactive prompt instruction in the reconcile dispatch path,
then zero matches are found (covers REQ-007 autonomy property).

Suggested files: `.claude/skills/speccy-orchestrate/SKILL.md`
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-005 REQ-006">
## Rust CLI: emit `consistency` block in `speccy next` JSON and detect four drift kinds

In `speccy-core/` and `speccy-cli/`, implement the consistency check
that powers REQ-005 and REQ-006.

**Core data structures (speccy-core):**

Define the consistency types — either in a new `speccy-core/src/consistency.rs`
or alongside the existing `next.rs` data structures, whichever fits the
existing module layout:

```rust
pub enum ConsistencyStatus { Ok, Drift, Blocked }

pub struct DriftEntry {
    pub task_id: String,           // "T-NNN"
    pub kind: DriftKind,
    pub severity: DriftSeverity,
    pub tasks_state: String,
    pub details: DriftDetails,
}

pub enum DriftKind {
    CommitWithoutState,
    StateCompletedNoCommit,
    StateInProgressOrphaned,
    JournalXmlMalformed,
}

pub enum DriftSeverity { AutoFixable, Blocking }

pub enum DriftDetails {
    CommitWithoutState { commit_sha: String, commit_short_sha: String },
    StateCompletedNoCommit { expected_trailer: String, working_tree_dirty: bool },
    StateInProgressOrphaned { working_tree_dirty: bool, dirty_files_count: usize },
    JournalXmlMalformed { journal_path: String, last_well_formed_byte_offset: usize },
}
```

**Detection logic (speccy-core):**

Implement `fn detect_consistency(spec: &SpecContext, workspace_root: &Utf8Path) -> ConsistencyBlock`
(name and exact signature are implementation choices) that:

1. For each task in the spec, constructs the expected commit title
   prefix `[SPEC-NNNN/T-NNN]:` and queries `git log --oneline --grep`
   (via `std::process::Command` or libgit2) for commits matching that
   prefix.
2. Detects `commit_without_state`: a matching commit exists but the
   task is in a non-`completed` state.
3. Detects `state_completed_no_commit`: the task is `completed` but no
   matching commit exists. Runs `git status --porcelain` to populate
   `working_tree_dirty`.
4. Detects `state_in_progress_orphaned`: the task is `in-progress`,
   `git status --porcelain` has output, and no matching commit exists.
   Counts dirty files for `dirty_files_count`.
5. Detects `journal_xml_malformed`: parses the per-task journal file
   with the existing XML parser. On parse failure, uses the parser's
   error output to compute `last_well_formed_byte_offset`.
6. Sets `ConsistencyStatus::Ok` when `drifts` is empty;
   `Drift` when any `auto_fixable` entry exists but no `blocking`;
   `Blocked` when any `blocking` entry exists.

The function is read-only: no `git add`, `git commit`, `git restore`,
`git clean`, or any mutating git command anywhere in this code path.

**JSON serialisation (`speccy-cli`):**

Extend the `speccy next` JSON envelope to include a top-level
`consistency` object:

```json
{
  "consistency": {
    "status": "ok",
    "drifts": []
  },
  "next_action": { ... }
}
```

When `consistency.status != "ok"`, override `next_action.kind` to
`"reconcile"` (all other `next_action` fields remain as they would
have been under normal dispatch).

When `status == "ok"`, `drifts` may be omitted or emitted as `[]` —
pick whichever is simpler with the existing serialization approach.

Verify the constraint from REQ-005: no calls to mutating git commands
exist anywhere under `speccy-cli/src/` or the core crate's `src/`
directory (source-level grep).

<task-scenarios>
Given a SPEC-NNNN whose TASKS.md marks T-001 as `state="completed"`
and whose git log contains no commit whose title begins with
`[SPEC-NNNN/T-001]: `, when `speccy next SPEC-NNNN --json` runs and
exits 0, then `consistency.status == "blocked"`, `consistency.drifts`
has length ≥ 1, the entry has `task_id == "T-001"`,
`kind == "state_completed_no_commit"`, `severity == "blocking"`, and
`next_action.kind == "reconcile"` (covers CHK-009).

Given a SPEC-NNNN whose TASKS.md marks T-002 as `state="in-review"`
and whose git log contains a commit titled `[SPEC-NNNN/T-002]: ...`,
when the same command runs, then `consistency.status == "drift"`,
the entry has `kind == "commit_without_state"`,
`severity == "auto_fixable"`, and `details.commit_sha` is a
40-character hex string (covers CHK-010).

Given a SPEC-NNNN where every `completed` task has a matching commit
and no non-completed task has a commit, when the command runs,
then `consistency.status == "ok"` and `next_action.kind` reflects
normal dispatch (covers CHK-009 happy path, CHK-011 source-grep
constraint).

Suggested files: `speccy-core/src/consistency.rs` (new or merged into
existing next.rs), `speccy-core/src/next.rs`, `speccy-cli/src/next.rs`,
`speccy-cli/src/main.rs`
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-005 REQ-006">
## Add test fixtures and integration tests for all four drift kinds

Write the test fixtures and integration tests that verify the four
drift-kind detections from T-005.

**Fixture workspaces:**

Create test fixture workspaces under `speccy-cli/tests/fixtures/`
(or the existing fixture directory convention). Each fixture is a
minimal git repo with a `.speccy/specs/NNNN-slug/` directory
containing a SPEC.md, TASKS.md, and optionally a `journal/T-NNN.md`.
Use the existing fixture-workspace pattern already established in
`speccy-cli/tests/`.

Fixtures needed:
- `consistency_completed_no_commit_dirty/`: TASKS.md T-001
  `state="completed"`, no matching git commit, two modified tracked
  files in the working tree.
- `consistency_completed_no_commit_clean/`: same but working tree clean.
- `consistency_commit_without_state/`: TASKS.md T-002
  `state="in-review"`, git log contains a commit titled
  `[SPEC-NNNN/T-002]: Some task`.
- `consistency_in_progress_orphaned/`: TASKS.md T-003
  `state="in-progress"`, four modified tracked files, no matching commit.
- `consistency_journal_malformed/`: TASKS.md T-001 `state="completed"`,
  matching commit present, journal file contains an unclosed `<review>`
  tag.
- `consistency_ok/`: all tasks either have matching commits (completed)
  or no commits (non-completed), journal files well-formed.

**Integration tests (`speccy-cli/tests/consistency.rs` or equivalent):**

- `test_state_completed_no_commit_dirty_tree`: runs `speccy next
  SPEC-NNNN --json` against the dirty fixture; asserts
  `consistency.status == "blocked"`, entry `kind ==
  "state_completed_no_commit"`, `details.working_tree_dirty == true`
  (covers CHK-012).
- `test_journal_xml_malformed`: runs against the malformed-journal
  fixture; asserts `kind == "journal_xml_malformed"`,
  `details.journal_path` matches the fixture file path,
  `details.last_well_formed_byte_offset` is a non-negative integer
  (covers CHK-013).
- `test_commit_without_state`: asserts `kind == "commit_without_state"`,
  `details.commit_sha` is 40 hex chars (covers CHK-010).
- `test_state_in_progress_orphaned`: asserts `kind ==
  "state_in_progress_orphaned"`, `details.dirty_files_count == 4`.
- `test_consistency_ok`: asserts `consistency.status == "ok"` and
  `next_action.kind != "reconcile"`.
- `test_no_mutating_git_commands_in_source`: a compile-time or
  source-grep test asserting no call to `git add`, `git commit`,
  `git restore`, `git clean`, or `git stash` appears in any `.rs`
  file under `speccy-cli/src/` or `speccy-core/src/` (covers CHK-011).

Run the full hygiene gate at the end:
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

<task-scenarios>
Given the test fixture `consistency_completed_no_commit_dirty/` with
TASKS.md T-001 `state="completed"`, no matching commit, and two
modified tracked files, when `speccy next SPEC-NNNN --json` runs,
then `consistency.status == "blocked"`, the entry has
`kind == "state_completed_no_commit"`, `severity == "blocking"`, and
`details.working_tree_dirty == true` (covers CHK-012).

Given the test fixture `consistency_journal_malformed/` whose journal
body is `<implementer>...</implementer>\n<review persona="business"`
(unclosed `<review>` tag), when `speccy next SPEC-NNNN --json` runs,
then `consistency.drifts` contains an entry with
`kind == "journal_xml_malformed"`,
`details.journal_path` matches the on-disk fixture file path, and
`details.last_well_formed_byte_offset` equals the byte offset of the
close of the `<implementer>` element (covers CHK-013).

Given the workspace at HEAD after this task,
when `cargo test --workspace` runs,
then it exits 0.

Suggested files: `speccy-cli/tests/consistency.rs` (new),
`speccy-cli/tests/fixtures/consistency_*/` (new fixture dirs),
plus any shared fixture-workspace helper already used by other tests.
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-006">
## Update docs/ARCHITECTURE.md with the consistency envelope shape and drift kind enum

Edit `docs/ARCHITECTURE.md` to document the new `speccy next` JSON
envelope fields introduced by T-005.

Locate the existing `speccy next` JSON envelope documentation
(wherever it currently describes the `next_action` field and the
overall response shape). Add a `consistency` subsection documenting:

- The top-level `consistency` field shape:
  ```
  "consistency": {
    "status": "ok" | "drift" | "blocked",
    "drifts": [...]
  }
  ```
- The three status values and when each is emitted.
- The `drifts[]` entry schema with all four `kind` values, their
  `severity`, and their `details` object shapes (matching the shapes
  defined in REQ-006 and implemented in T-005).
- The override rule: when `consistency.status != "ok"`,
  `next_action.kind` is always `"reconcile"`.
- The "CLI stays read-only" constraint: the consistency check performs
  no mutations; `git add`, `git commit`, `git restore`, `git clean`,
  and `git stash` are not invoked by the binary.

Also add a note that the `kind` enum is extensible: future drift kinds
require changes in two places (CLI detection + reconcile partial), and
a cross-reference to `.claude/speccy-references/reconcile-policy.md`
for the policy table.

This task makes no functional source changes (doc-only). Run the
hygiene gate as a regression guard.

<task-scenarios>
Given `docs/ARCHITECTURE.md` after this task, when grepped for the
literal substring `consistency`, then at least one match appears
inside the `speccy next` JSON documentation section.

Given the same file, when scanned for each of the four drift kind
names (`commit_without_state`, `state_completed_no_commit`,
`state_in_progress_orphaned`, `journal_xml_malformed`), then each
name appears at least once in the architecture doc.

Given the same file, when scanned for the reconcile-policy partial
path (`.claude/speccy-references/reconcile-policy.md`), then at least
one cross-reference is present linking the drift kind enum to the
policy source.

Given the workspace at HEAD after this task, when `cargo test
--workspace` runs, then it exits 0 (no regressions from doc-only
changes).

Suggested files: `docs/ARCHITECTURE.md`
</task-scenarios>
</task>
