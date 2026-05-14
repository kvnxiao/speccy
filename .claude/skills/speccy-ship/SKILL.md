---
name: speccy-ship
description: Close out a Speccy spec — write REPORT.md, run `speccy verify` as a CI dry-run, commit, and open a pull request. Use when the user says "ship SPEC-NNNN", "wrap up the spec", "open a PR for the spec", or every task is done and the loop is complete.
---

# /speccy-ship

Renders the report prompt, writes `REPORT.md`, runs
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
5. Flip the SPEC's frontmatter status. Edit
   `.speccy/specs/NNNN-slug/SPEC.md` and change `status: in-progress`
   to `status: implemented`. The diff that ships in this PR is what
   makes the SPEC implemented, so the status flip belongs in the
   same PR, not in a follow-up. The byte-level edit invalidates
   TASKS.md's `spec_hash_at_generation`; refresh it and confirm:

   ```bash
   speccy tasks SPEC-NNNN --commit
   speccy status
   ```

   `speccy status` should report no `TSK-003` mismatch for SPEC-NNNN.
6. Commit SPEC.md, TASKS.md, REPORT.md, and the code changes from the
   loop. Then push:

   - If this branch has no open PR yet, open one:

     ```bash
     gh pr create --title "<spec id> <slug>" --body "$(cat REPORT.md)"
     ```

   - If a PR already exists for this branch (e.g., a long-running
     branch carrying multiple specs), push to update it:

     ```bash
     git push
     ```

   The status flip in step 5 lands in the same PR — no follow-up
   commit needed after merge.

This recipe does not loop.
