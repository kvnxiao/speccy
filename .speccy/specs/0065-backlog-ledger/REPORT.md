---
spec: SPEC-0065
outcome: implemented
generated_at: 2026-06-14T04:00:00Z
---

# REPORT: SPEC-0065 Backlog ledger — a convention-only `.speccy/BACKLOG.md` register of future-spec candidates

<report spec="SPEC-0065">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 created `resources/modules/references/backlog-ledger.md`, structurally
parallel to `memory-ledger.md`, documenting the `.speccy/BACKLOG.md` file
header and the four-field entry shape (Title / What & why / Deferred-because /
Provenance) with authoring discipline and a `SPEC-0042`-family worked example.
T-002, T-003, and T-004 added `{% include "modules/references/backlog-ledger.md" %}`
to the speccy-plan, speccy-brainstorm, and speccy-ship modules respectively;
reeject confirmed the shape appears in all ejected host packs. CHK-001 (parity
verified by reejection) and CHK-002 (parallelism and clarity confirmed by
reviewer personas) both passed. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-006 added two init-immunity regression tests to `speccy-cli/tests/init.rs`,
mirroring the existing MEMORY.md test pair:
`fresh_init_does_not_create_speccy_backlog_ledger` confirms no BACKLOG.md is
created in a fresh repo; `force_preserves_speccy_backlog_ledger` seeds a
sentinel file of known SHA-256, runs `speccy init --force` for both hosts, and
asserts byte-identity and absence from the ejected paths list. A third assertion
confirmed `speccy verify` exits 0 with no backlog-attributable diagnostic when
the file is present. No production init code was changed — the manifest-driven
design already excluded user-owned files, exactly as MEMORY.md enjoys. Retry
count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005">
T-002 wired speccy-plan and T-003 wired speccy-brainstorm to read
`.speccy/BACKLOG.md` at framing time and surface existing entries as candidate
slices, treating absence as silent and non-fatal. CHK-005 (correct placement and
wording of the read step in both ejected skill bodies) was confirmed by
reviewer personas. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006">
T-002 wired speccy-plan and T-003 wired speccy-brainstorm to append a four-field
entry on deliberate future-spec scope-cut, self-creating the file with its header
if absent, and to route spec-local Non-goals only to the SPEC's `## Non-goals`
section. The vet round-1 drift review identified that the plan staging step
omitted `.speccy/BACKLOG.md` (outside `<spec-dir>/`), leaving appended entries
uncommitted. The holistic fix widened the plan staging step to also stage
`.speccy/BACKLOG.md` whenever it exists; the brainstorm path was resolved by
documenting the hand-off to the subsequent speccy-plan commit. Retry count: 1 (vet round).
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-007">
T-004 added a per-item "its own future SPEC, or just a limitation of this one?"
judgment step in `resources/modules/phases/speccy-ship.md`, mirroring only
future-spec-worthy items into `.speccy/BACKLOG.md` with ship-phase provenance.
The vet round-1 drift review found an internal contradiction: step 3 promised
the backlog append lands in the ship commit but step 6's staging enumeration
omitted it. The holistic fix added `.speccy/BACKLOG.md` to step 6's explicit
staging list, resolving the contradiction. Retry count: 1 (vet round).
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-008">
T-002 wired speccy-plan to delete a promoted backlog entry outright — no
struck-through or "promoted to" residue — relying on git history and the new
SPEC's provenance for traceability. The vet round-1 staging gap finding covered
the strike path as well (same plan staging fix). Retry count: 1 (vet round, shared with REQ-004).
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-009">
T-005 added one terse line to
`resources/modules/references/agents-md-speccy-conventions.md` naming
`.speccy/BACKLOG.md` and its read/append roles (future-spec candidates; planning
reads, plan/ship append). The line reaches the conventions block produced by
`/speccy-bootstrap` through the existing `{% include %}` chain, confirmed by
reejection parity. Retry count: 0.
</coverage>

</report>

## Notes

The vet gate (invocation 1, round 2) flagged one open human decision not
blocking ship: a brainstorm session that frames an amendment (rather than a new
SPEC) may append to BACKLOG.md, but speccy-amend's narrow staging does not
include BACKLOG.md, leaving that entry uncommitted until the next plan run.
The vet reviewer judged this outside the SPEC's stated producer set (DEC-002)
and did not block. This is a candidate for a future backlog entry.
