
# {{ cmd_prefix }}speccy-ship

Writes `REPORT.md`, runs `speccy verify` as a CI dry-run, and opens
the pull request.

## When to use

After `{{ cmd_prefix }}speccy-review` has flipped every task to
`state="completed"` and `{{ cmd_prefix }}speccy-vet` has passed.
Confirm the spec state first:

```bash
speccy next SPEC-NNNN --json
```

Readiness semantics:

- `next_action.kind == "ship"` (exit 0) is the ship-readiness
  signal — all tasks completed, vet gate passing, no REPORT.md
  yet. Proceed.
- `next_action: null` paired with **non-zero exit** is the
  terminal-already-shipped signal — REPORT.md is present, the
  SPEC has already shipped. Stop; do not re-ship. Run
  `speccy archive SPEC-NNNN` if it should be moved out of the
  active tree.
- Any other `next_action.kind` (`work`, `review`, `vet`,
  `decompose`) means tasks remain — pick up
  `{{ cmd_prefix }}speccy-work`, `{{ cmd_prefix }}speccy-review`,
  or `{{ cmd_prefix }}speccy-vet` first.

## Steps

1. The `speccy next SPEC-NNNN --json` already run in "When to use"
   confirms ship-readiness (`next_action.kind == "ship"`, exit 0) and
   carries the `spec_md_path` and `tasks_md_path` fields that locate
   the files — no separate `speccy status` call is needed. If instead
   it returned `next_action: null` with a non-zero exit, REPORT.md
   already exists and the SPEC has already shipped; do not proceed.

2. Write `.speccy/specs/NNNN-slug/REPORT.md` with frontmatter
   (`spec`, `outcome`, `generated_at`), a `<report>` root element
   wrapping one `<coverage req="REQ-NNN"
   result="satisfied|partial|deferred" scenarios="CHK-NNN...">`
   element per surviving SPEC requirement, retry counts, and any
   out-of-scope items implementers absorbed.

   Canonical REPORT.md shape: `{{ skill_install_path }}/speccy-ship/references/report.md`.

3. Ship-time memory retro. With REPORT.md written and before the ship
   commit (step 6), distill the just-completed loop into mutations of
   the repo's loop ledger at `.speccy/MEMORY.md`. Mine the evidence
   already on disk — REPORT.md coverage, the per-task journal
   (`<blockers>` directives, review verdict flips, retry rounds), and
   the spec diff (`git diff origin/main`, two-dot) — rather than
   re-deriving the work from scratch. Use the two-dot `git diff
   origin/main`, **not** `origin/main...HEAD`: the retro runs here at
   step 3, before the step-6 ship commit, while the loop's per-task
   work is still uncommitted in the working tree (the work phase never
   commits per task). The two-dot form diffs the working tree against
   the ref and so captures that uncommitted work; the three-dot
   `...HEAD` form compares the merge-base against committed HEAD and
   silently misses it, handing the retro a stale or empty diff exactly
   when it must mine the just-completed loop. The entry shape you write
   here is defined once at:

   {% include "modules/references/memory-ledger.md" %}

   The retro does three things in one pass:

   - **Capture (both feeds).** Append convention-flavoured and/or
     mistake-flavoured entries to `.speccy/MEMORY.md` in the four-part
     shape, **one entry per write** so the prose-layer append stays
     serial. A loop with recorded friction — a blocking-then-passed
     review round, a retry round, a `<blockers>` directive — yields at
     least one mistake-flavoured entry whose provenance cites that
     evidence and whose corrective rule addresses the cause. A clean,
     frictionless loop with no durable lesson records that explicitly
     ("no durable lesson this loop") rather than inventing one. Create
     the file if it does not yet exist.

   - **Consolidate and dedupe (human-gated).** Propose promoting stable,
     repeatedly-affirmed entries up into the durable tier (`AGENTS.md`
     / rules) and surface each promotion for **human approval** — never
     promote silently or automatically. On approval, make the
     durable-tier edit and **remove the promoted entry from
     `.speccy/MEMORY.md`** so it is not stored in both tiers. Dedupe
     candidates within the ledger and against the repo's existing
     durable docs (`AGENTS.md`, rule files, anything they point at):
     drop a candidate already covered there rather than appending it.

   - **Phantom-reference GC.** Re-validate existing ledger entries
     against the current tree and retire or rewrite any whose
     referenced construct no longer resolves, so the ledger never
     feeds a phantom forward to the next implementer. Abstractly-worded
     convention entries that name no specific construct survive a
     refactor unchanged; entries pinned to a now-gone module or symbol
     are retired or reworded to the surviving convention. This is a
     semantic judgment plus the abstract-authoring discipline the entry
     reference describes — deliberately not a CLI freshness check, and
     no such mechanism is added.

   The resulting `.speccy/MEMORY.md` mutation lands in the same ship
   commit as REPORT.md (step 6), so the lesson and the loop that taught
   it ship together.

4. Flip the SPEC's frontmatter status. Edit
   `.speccy/specs/NNNN-slug/SPEC.md` and change `status: in-progress`
   to `status: implemented`. The diff that ships in this PR is what
   makes the SPEC implemented, so the status flip belongs in the
   same PR, not in a follow-up. The status flip is hash-neutral
   because `status` is excluded from `spec_hash_at_generation`, so
   TASKS.md does not need a hash refresh and the spec-hash-mismatch
   lint cannot fire — no post-flip re-check is needed.
5. Run the CI gate locally as a dry-run *after* the status flip so
   verify reads the post-ship tree:

   ```bash
   speccy verify
   ```

   → expected: exit 0. A non-zero exit means the proof shape is broken
   (uncovered requirement, malformed task state, parser-rejected journal
   element) — stop and fix before opening the PR.

6. Commit SPEC.md, TASKS.md, REPORT.md, the `.speccy/MEMORY.md`
   mutation from the retro (step 3), and the code changes from the
   loop. Then push:

   - If this branch has no open PR yet, open one. Render the PR body
     from the canonical template at
     `{{ skill_install_path }}/speccy-ship/references/pr-body.md`: fill its
     three placeholders (named `spec-dir`, `summary`, `coverage-rows`
     inside angle-bracket markers in the template) from
     `.speccy/specs/NNNN-slug/SPEC.md`'s `## Summary` prose, the
     `<coverage>` elements in `.speccy/specs/NNNN-slug/REPORT.md`, and
     the spec-dir path itself. Write the rendered markdown to a scratch
     file (e.g. `/tmp/pr-body.md`) and pass it via `--body-file`:

     ```bash
     gh pr create --title "<spec id> <slug>" \
       --body-file /tmp/pr-body.md
     ```

     Do **not** pipe `REPORT.md` inline via shell command substitution
     into the `--body` flag. GitHub does not render the `<report>` and
     `<coverage>` XML wrappers as markdown, so the angle brackets leak
     into the PR page as visible prose; always use `--body-file` with
     the rendered template instead.

     Multi-SPEC fallback: branches that bundle multiple SPECs, or
     carry unrelated precursor commits, fall back to a hand-authored
     PR body. The template can serve as a per-SPEC starting skeleton
     when hand-authoring — render once per SPEC and stitch the
     sections — but this recipe does not prescribe multi-SPEC
     composition.

   - If a PR already exists for this branch (e.g., a long-running
     branch carrying multiple specs), push to update it:

     ```bash
     git push
     ```

   The status flip in step 4 lands in the same PR — no follow-up
   commit needed after merge.

## Exit

REPORT.md is written, the SPEC's frontmatter status is flipped to
`implemented`, the loop's uncommitted work is bundled into one atomic ship
commit, and a PR is opened (or the existing branch PR updated by push).
`speccy verify` passed as the CI dry-run. Single pass, no loop — the SPEC has
shipped; run `speccy archive SPEC-NNNN` if it should leave the active tree.
