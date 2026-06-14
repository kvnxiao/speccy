# Speccy per-repo loop memory

> User-owned, git-tracked, never overwritten by `speccy init --force` or
> `just reeject`. Absence is normal and silent. See
> `resources/modules/references/memory-ledger.md` for the entry shape.

---

- Trigger: implementing a new phase or skill that appends a file outside
  `<spec-dir>/` (e.g., a shared ledger at `.speccy/BACKLOG.md`) and commits
  the result in a step that lists staged paths explicitly.
- Mistake: a phase body's narrative promised the out-of-spec-dir file would
  land in the ship commit (step 3), but the explicit staging enumeration in
  the commit step (step 6) omitted it — an internal contradiction that would
  cause agents following the step literally to leave the appended file
  uncommitted.
- Corrective rule: whenever a phase body adds a producer for a new file
  outside the spec directory and promises it will be committed, verify the
  commit step's explicit staging list names that path too; promise and staging
  enumeration must agree.
- Provenance: SPEC-0065 / T-004 (0065-backlog-ledger), vet-reviewer blocking
  round 1 / holistic-fix round 1.
