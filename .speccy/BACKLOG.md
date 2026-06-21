# Speccy backlog — future-spec candidates

> User-owned, git-tracked, never created or overwritten by `speccy init`,
> `speccy init --force`, or reeject. Absence is normal and silent; the CLI
> never reads this file. Distinct from `MEMORY.md` (durable loop conventions)
> and from spec-local deferred surfaces (`## Non-goals`, deferred decisions,
> deferred coverage): each entry below should become its OWN spec. Promotion
> retires an entry by deletion. See
> `resources/modules/references/backlog-ledger.md` for the entry shape.

- Title: First-class provenance-cleanup journal block.
- What & why: give the pre-ship provenance-cleanup pass its own vet journal block (a scan/apply pair or a single fix block) so cleanup history is queryable via `speccy journal show`, at parity with the drift-review and simplifier audit trails, instead of only readable from the diff plus the gate summary.
- Deferred-because: audit value unproven — the cleanup's edits are self-evident in the branch diff today, so a structured block would mostly duplicate `git diff`; promote only if dogfooding shows people want to query the history rather than read it from the diff.
- Provenance: SPEC-0066, plan.
