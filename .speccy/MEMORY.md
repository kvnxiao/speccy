# Speccy per-repo loop memory

> User-owned, git-tracked, never overwritten by `speccy init --force` or
> `just reeject`. Absence is normal and silent. See
> `resources/modules/references/memory-ledger.md` for the entry shape.

---

- Phase body adds a producer for a file outside the spec dir and promises it ships → confirm the commit step's explicit staging list names that path; promise and staging enumeration must agree. [SPEC-0065/T-004]
- Wiring an apply-mode sub-agent into a vet phase with a snapshot/rollback sequence → snapshot the tree (stash push + apply) BEFORE dispatching the sub-agent, mirroring the Phase 2 simplifier order, so the journal-safe revert undoes the edits instead of restoring them. [SPEC-0066/T-003]
- Renumbering phases in a multi-phase flow → grep the source module for the old phase number before `just reeject`; prose cross-links that targeted edits miss otherwise propagate into the ejected mirrors. [SPEC-0066/T-003]
