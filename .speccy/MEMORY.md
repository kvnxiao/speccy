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

- Trigger: labelling a CHK `demonstrated` in an implementer evidence roll-call when the cited test only broadly validates a related property (e.g., a token-ban scan) rather than specifically gating the property the CHK names (e.g., include-presence plus no-inline-copy).
- Mistake: T-002 round 1 cited `module_prose_has_no_internal_artifact_id_provenance` as proof that the convention definition was shared via include rather than inlined; that scan only matches banned artifact-ID tokens — a fully-inlined copy of the definition would pass it — so the `demonstrated` label was unsupported.
- Corrective rule: before labelling a CHK `demonstrated`, confirm the cited test actually falsifies the wrong implementation the CHK describes: if removing the feature or inverting the property would not cause the test to fail, the label is false. Either relabel `judgment-only` or add a test that genuinely falsifies the defect.
- Provenance: SPEC-0066 / T-002 (0066-provenance-cleanup), reviewer-tests blocking round 1.

- Trigger: wiring an apply-mode sub-agent into a vet phase that uses a snapshot/rollback sequence to revert on hygiene failure.
- Mistake: T-003 round 1 took the `git stash push` snapshot after dispatching the apply-mode `vet-provenance` sub-agent, so the stash captured the post-rewrite tree; the blocking-path revert restored the edited state rather than undoing it.
- Corrective rule: snapshot the working tree (stash push + stash apply) BEFORE dispatching any apply-mode sub-agent; the stash must hold the pre-edit state so the journal-safe revert can genuinely undo the sub-agent's edits. Mirror the Phase 2 simplifier order exactly: step 1 = snapshot, step 2 = spawn, step 3 = keep-or-revert.
- Provenance: SPEC-0066 / T-003 (0066-provenance-cleanup), reviewer-correctness + reviewer-architecture blocking round 1.

- Trigger: renumbering phases in a multi-phase flow (e.g., inserting a new phase that shifts the gate phase from Phase N to Phase N+1).
- Mistake: T-003 round 1 updated the phase headings, the exit-path list, and the gate body example after renumbering, but missed the Phase 0 bootstrap prose cross-reference that still named the old gate phase number; the defect propagated into ejected mirrors via reejection.
- Corrective rule: after renumbering phases, grep the source module for every occurrence of the old phase number before running `just reeject`; a literal search for "Phase N" (e.g., `grep "Phase 3"`) catches references in prose cross-links that targeted edits miss.
- Provenance: SPEC-0066 / T-003 (0066-provenance-cleanup), reviewer-style + reviewer-business blocking round 1.
