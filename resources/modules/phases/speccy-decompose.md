
# {{ cmd_prefix }}speccy-decompose

Decomposes the SPEC into an ordered, single-agent-sized task list in
`TASKS.md`. If `TASKS.md` already exists, amends it surgically instead
(preserving the state of any task already in flight).

## When to use

- Initial: after `{{ cmd_prefix }}speccy-plan` lands a fresh SPEC.
- Amendment: after `{{ cmd_prefix }}speccy-amend SPEC-NNNN` edited an existing SPEC and
  the tasks may now be stale (the CLI surfaces a `TSK-003` lint when
  it detects hash drift).

## Steps

1. Read the spec's current state to locate SPEC.md:

   ```bash
   speccy status SPEC-NNNN --json
   ```

   The JSON's `spec_md_path` field names the SPEC.md to decompose.
   If `tasks_md_path` is non-null, an existing TASKS.md is present
   and this is an amendment run (edit surgically; preserve
   `state="completed"` tasks unless invalidated).

   Before authoring `TASKS.md`, invoke the `plan-architect` subagent
   against the SPEC to produce an implementation blueprint. Treat its
   build-sequence checklist as the **candidate** task list — you
   retain final `<task>` authorship and MAY merge, split, reorder, and
   renumber the candidates to land agent-sized tasks. Promote any
   load-bearing design decision surfaced by the blueprint into a
   SPEC.md `### Decisions` (DEC-NNN) block rather than burying it in
   task prose.

2. Write `TASKS.md` as Markdown with a sequence of
   `<task id="T-NNN" state="pending" covers="REQ-NNN">...</task>`
   block per task directly under the heading — no `<tasks>` wrapper
   element. Each `<task>` body contains a `<task-scenarios>`
   element with slice-level Given/When/Then prose and an optional
   `Suggested files:` bullet.

   The file must open with a YAML frontmatter block followed immediately
   by a level-1 heading.

   Canonical TASKS.md shape: `{{ skill_install_path }}/speccy-decompose/references/tasks.md`.

   Key constraints:
   - The `# Tasks: SPEC-` heading must appear on the line immediately
     after the closing `---` of the frontmatter block (no blank line
     between them).
   - No `<tasks spec="...">` wrapper element. Tasks are a flat sequence
     of `<task>` elements at the top level of the document.
   - Multiple requirements in `covers=` are separated by single ASCII
     spaces — `covers="REQ-001 REQ-002"` — never by commas.
   - Seed `spec_hash_at_generation` and `generated_at` with the
     `bootstrap-pending` sentinel; step 3 fills them in. Do not invoke
     `speccy lock` before TASKS.md exists on disk — it errors when the
     file is missing.
3. After writing, run `speccy lock SPEC-NNNN` to rewrite the two
   `bootstrap-pending` placeholders to the current SPEC.md sha256 and
   the UTC timestamp:

   ```bash
   speccy lock SPEC-NNNN
   ```

   `speccy lock` requires TASKS.md to already exist.

4. Branch-guard, then commit `TASKS.md` alone. This closes the
   bootstrap-commit gap that would otherwise trip the SPEC-0045/REQ-002
   strict clean-tree gate when `{{ cmd_prefix }}speccy-orchestrate
   SPEC-NNNN` is invoked on a freshly decomposed SPEC. The commit runs
   after `speccy lock`. It commits only the spec's `TASKS.md` —
   `SPEC.md` is committed by `{{ cmd_prefix }}speccy-plan`, not here, so
   the new-spec path lands two separate commits (one per skill). The
   step uses narrow file-list staging (never `git add -A` or
   `git add .`), so any unrelated dirty paths outside `<spec-dir>/`
   remain in the working tree untouched. The step is idempotent:
   re-running decompose on an already-committed `TASKS.md` produces no
   new commit.

   First run the branch-guard prelude so the commit lands on a feature
   branch rather than the repository's default branch. Supply the
   prelude's one parameter — the **spec directory** (`<spec-dir>/`,
   i.e. the path that holds `SPEC.md` and `TASKS.md`) — and run it:

{% include "modules/references/branch-guard.md" %}

   Then run the shared commit recipe, supplying its two
   behaviour-varying parameters as follows:

   - **Staging breadth: narrow `git add <spec-dir>/TASKS.md`.** Stage
     exactly the spec's `TASKS.md` and nothing else. Do not use
     `git add -A` or `git add .`. Staging unchanged content is a no-op,
     so passing the path unconditionally is safe regardless of whether
     `TASKS.md` was already committed.
   - **Title and body.**
     - **Title:** `[SPEC-NNNN]: decompose tasks` with `SPEC-NNNN`
       substituted for the resolved spec id.
     - **Body:** the trimmed value of the `title:` field from SPEC.md's
       YAML frontmatter (the one-line title slug, not the full document
       heading).

   With those two parameters fixed, run the shared recipe — it defines
   the no-git short-circuit, the unified stage-then-`git diff --cached
   --quiet` idempotency check (an unchanged `TASKS.md` skips the commit
   silently), the `Co-Authored-By` trailer, and the HEREDOC commit
   mechanics:

{% include "modules/references/commit-recipe.md" %}

5. Suggest the next step: `{{ cmd_prefix }}speccy-work SPEC-NNNN` to start the
   implementation loop.

This recipe does not loop.
