# Speccy per-repo loop memory

> User-owned, git-tracked, never overwritten by `speccy init --force` or
> `just reeject`. Absence is normal and silent. See
> `resources/modules/references/memory-ledger.md` for the entry shape.

---

## Entry: git diff form when retro or ship step runs before the ship commit

**Trigger:** any vet or ship phase step that needs to inspect the
just-completed loop's uncommitted working-tree changes (e.g., a ship-time
retro mining the spec diff for memory capture).

**Convention/mistake:** using `git diff origin/main...HEAD` (three-dot form)
at a point before the ship commit silently produces an empty or stale diff,
because three-dot compares the merge-base against committed HEAD and misses
all uncommitted loop work.

**Corrective rule:** use `git diff origin/main` (two-dot) when the step runs
before the ship commit and must see uncommitted changes. Add a brief
why-two-dot rationale in the prose so future readers understand the
constraint.

**Provenance:** SPEC-0064 / vet invocation 1, drift-review round 1
(`journal/VET.md`).
