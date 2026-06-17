Open the spec bundle before reviewing or changing anything:

```bash
speccy context SPEC-NNNN --json
```

Use `paths.spec_md`, `paths.tasks_md`, and `paths.vet_journal` from
that bundle for targeted reads. Use its `diff_command` exactly as
given. It is a working-tree diff against the default branch, so it
captures both committed and uncommitted holistic changes between vet
rounds. Do not substitute a `...HEAD` command; that form can miss the
vet-implementer's uncommitted fixes.
