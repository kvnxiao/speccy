# Speccy backlog — future-spec candidates

> User-owned, git-tracked, never created or overwritten by `speccy init`,
> `speccy init --force`, or reeject. Absence is normal and silent; the CLI
> never reads this file. Distinct from `MEMORY.md` (durable loop conventions)
> and from spec-local deferred surfaces (`## Non-goals`, deferred decisions,
> deferred coverage): each entry below should become its OWN spec. Promotion
> retires an entry by deletion. See
> `resources/modules/references/backlog-ledger.md` for the entry shape.

- Title: `speccy verify` lint for demonstrated-to-scenario consistency.
- What & why: add a lint that flags an implementer block labelling a CHK `demonstrated` when the canonical `evidence/T-NNN.md` is absent or carries no `### Scenario` heading, surfacing the gap at `speccy verify` time in addition to the existing append-time gate; useful for auditing older specs retroactively once they carry evidence files.
- Deferred-because: existing completed specs predate evidence files, so an error-level lint would retroactively fail CI; a warn-level lint adds a lint family and snapshot churn for little gain until the append gate has driven evidence-file adoption.
- Provenance: SPEC-0068, ship.

- Title: First-class provenance-cleanup journal block.
- What & why: give the pre-ship provenance-cleanup pass its own vet journal block (a scan/apply pair or a single fix block) so cleanup history is queryable via `speccy journal show`, at parity with the drift-review and simplifier audit trails, instead of only readable from the diff plus the gate summary.
- Deferred-because: audit value unproven — the cleanup's edits are self-evident in the branch diff today, so a structured block would mostly duplicate `git diff`; promote only if dogfooding shows people want to query the history rather than read it from the diff.
- Provenance: SPEC-0066, plan.
