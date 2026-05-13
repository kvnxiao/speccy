---
description: Phase 5. Render the report prompt, write REPORT.md, and open a PR.
---

# /speccy-ship

Renders the Phase 5 report prompt, writes `REPORT.md`, runs
`speccy verify` once more as a CI dry-run, and opens the pull request.

## When to use

After `/speccy-review` has flipped every task to `[x]`. If any task is
still `[ ]`, `[~]`, or `[?]`, `speccy report` refuses with the offending
IDs -- pick up `/speccy-work` or `/speccy-review` first.

## Steps

1. Confirm all tasks are `[x]`:

   ```bash
   speccy status SPEC-0007
   ```

2. Run the CI gate locally as a dry-run before report-writing:

   ```bash
   speccy verify
   ```

3. Render the report prompt:

   ```bash
   speccy report SPEC-0007
   ```

4. Follow the prompt: write `.speccy/specs/NNNN-slug/REPORT.md` with
   frontmatter (`spec`, `outcome`, `generated_at`), the requirements
   coverage table, retry counts, and any out-of-scope items
   implementers absorbed.
5. Commit SPEC.md, TASKS.md, REPORT.md, and the code changes from the
   loop. Then open the PR:

   ```bash
   gh pr create --title "<spec id> <slug>" --body "$(cat REPORT.md)"
   ```

6. After the PR is open, set `frontmatter.status` to `implemented` on
   SPEC.md when the PR merges (a future amendment skill may automate
   this).

This recipe does not loop.
