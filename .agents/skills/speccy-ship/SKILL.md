---
name: speccy-ship
description: Close out a Speccy spec — write REPORT.md, run `speccy verify` as a CI dry-run, commit, and open a pull request. Use when the user says "ship SPEC-NNNN", "wrap up the spec", "open a PR for the spec", or every task is done and the loop is complete.
---

# speccy-ship

Renders the report prompt, writes `REPORT.md`, runs
`speccy verify` once more as a CI dry-run, and opens the pull request.

## When to use

After `speccy-review` has flipped every task to
`state="completed"`. If any task is still `state="pending"`,
`state="in-progress"`, or `state="in-review"`, `speccy report`
refuses with the offending IDs -- pick up `speccy-work`
or `speccy-review` first.

## Steps

1. Confirm all tasks for the spec are `state="completed"` (workspace
   overview; locate the spec row for SPEC-NNNN in the output):

   ```bash
   speccy status
   ```

2. Render the report prompt:

   ```bash
   speccy report SPEC-NNNN
   ```

3. Follow the prompt: write `.speccy/specs/NNNN-slug/REPORT.md` with
   frontmatter (`spec`, `outcome`, `generated_at`), a `<report>`
   root element wrapping one `<coverage req="REQ-NNN"
   result="satisfied|partial|deferred" scenarios="CHK-NNN...">`
   element per surviving SPEC requirement, retry counts, and any
   out-of-scope items implementers absorbed.
4. Flip the SPEC's frontmatter status. Edit
   `.speccy/specs/NNNN-slug/SPEC.md` and change `status: in-progress`
   to `status: implemented`. The diff that ships in this PR is what
   makes the SPEC implemented, so the status flip belongs in the
   same PR, not in a follow-up. The status flip is hash-neutral
   under SPEC-0024's hash function (`status` is excluded from
   `spec_hash_at_generation`), so TASKS.md does not need a hash
   refresh; running `speccy tasks SPEC-NNNN --commit` after the flip
   only refreshes `generated_at`, which is optional. Confirm the
   workspace is still clean:

   ```bash
   speccy status
   ```

   `speccy status` should report no `TSK-003` mismatch for SPEC-NNNN.
5. Run the CI gate locally as a dry-run *after* the status flip so
   verify reads the post-ship tree:

   ```bash
   speccy verify
   ```

6. Commit SPEC.md, TASKS.md, REPORT.md, and the code changes from the
   loop. Then push:

   - If this branch has no open PR yet, open one. Note the
     `REPORT.md` path is spec-local, not repo-root:

     ```bash
     gh pr create --title "<spec id> <slug>" \
       --body "$(cat .speccy/specs/NNNN-slug/REPORT.md)"
     ```

   - If a PR already exists for this branch (e.g., a long-running
     branch carrying multiple specs), push to update it:

     ```bash
     git push
     ```

   The status flip in step 4 lands in the same PR — no follow-up
   commit needed after merge.

This recipe does not loop.
