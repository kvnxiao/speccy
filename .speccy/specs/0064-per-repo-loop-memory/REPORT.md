---
spec: SPEC-0064
outcome: implemented
generated_at: 2026-06-13T05:35:00Z
---

# REPORT: SPEC-0064 per-repo-loop-memory — eject-safe `.speccy/MEMORY.md` ledger grown by a ship-time retro

<report spec="SPEC-0064">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
T-004 added two regression tests in `speccy-cli/tests/init.rs`:
`force_preserves_speccy_memory_ledger` seeds `.speccy/MEMORY.md` with
non-empty content and runs `speccy init --force` for both shipped hosts
(`claude-code` then `codex`), asserting byte-identity after each pass;
`fresh_init_does_not_create_speccy_memory_ledger` asserts the file is
absent after a fresh init. The invisibility property holds for free by
DEC-002 (the file sits in the same never-enumerated bucket as `BACKLOG.md`),
so no production code was changed. T-004 also added a `MEMORY.md` line to
`docs/ARCHITECTURE.md` File Layout documenting the user-owned / git-tracked /
never-enumerated / never-read-by-verify status. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
T-001 authored `resources/modules/references/memory-ledger.md` as a
self-contained, H2-start, `{% include %}`-able reference documenting the
canonical `.speccy/MEMORY.md` path and status, the four-part entry shape
(trigger / convention-or-mistake / corrective rule / provenance), the
authoring discipline (prefer abstract convention wording over fragile code
coordinates; provenance must resolve to a real identifier), and a worked
example using fictional placeholders labelled illustrative. T-003 wired this
reference into the ship retro step via `{% include %}`. CHK-002 is a dogfood
check (DEC-009): confirmed by manual inspection during this retro that the
MEMORY.md entry written below carries all four parts with provenance
resolving to SPEC-0064. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003 CHK-004">
T-002 inserted step 6 ("Load the memory ledger slice") into
`resources/modules/phases/speccy-work.md` before the bounded reuse survey,
instructing the implementer to read `.speccy/MEMORY.md` when present, load
the trigger-matched slice, and treat absence as a silent no-op. The step
includes `memory-ledger.md` rather than restating the entry shape.
CHK-004 is demonstrated by `speccy-cli/tests/memory_feedforward.rs`:
`work_phase_body_includes_memory_ledger_reference_once` (include exists
exactly once in the canonical module) and
`no_host_wrapper_inlines_memory_ledger_include` (no host wrapper under
`resources/agents/` shadows the directive). The reeject-sync half is covered
by the pre-existing `dogfood_outputs_match_committed_tree` test. CHK-003 is a
dogfood check deferred to future loop runs when the ledger is populated.
Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
T-003 added step 3 ("Ship-time memory retro") to
`resources/modules/phases/speccy-ship.md`, positioned after the REPORT.md
write (step 2) and before the ship commit (step 6), covering capture from both
feeds (conventions followed, blocking verdicts / retry rounds / blockers),
one-entry-per-write discipline, and an explicit "no durable lesson" record for
clean loops. The retro draws on REPORT.md / per-task journal / `git diff
origin/main` (two-dot, capturing uncommitted loop work; the vet drift-fix
corrected an initial three-dot form that would have silently missed
uncommitted changes). CHK-005 is a dogfood check: the retro for this spec runs
at ship time — see the ship-time retro section below. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006">
T-003's step 3 retro includes a consolidation sub-step: propose ledger-to-durable
promotion of stable, repeatedly-affirmed entries, require human approval before
any durable-tier edit, remove promoted entries from the ledger, and drop
candidates already covered by an existing durable doc. CHK-006 is a dogfood
check: the retro below checks entries against `AGENTS.md` and existing rules
before appending. No stable or pre-covered candidate was found on this first run
(ledger is new; no pattern has repeated across specs yet), so no promotion is
proposed. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-007">
T-003's step 3 retro includes a phantom-reference GC sub-step: re-validate
ledger entries against the current tree and retire or rewrite any whose
referenced construct is gone; prefer abstract, convention-level wording over
fragile code coordinates. The authoring-discipline guidance lives in the
`memory-ledger.md` reference (T-001). No CLI freshness-hashing mechanism was
added. CHK-007 is a dogfood check: the ledger is new for this spec and the one
entry written below uses abstract wording with no pinned code coordinates, so
no phantom-reference GC is triggered on this run. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-008">
T-004 added `malformed_speccy_memory_does_not_affect_verify_lint_output` to
`speccy-cli/tests/verify.rs`: captures the lint baseline with no ledger
present, writes a deliberately malformed `.speccy/MEMORY.md` (dangling
`REQ-999 CHK-999` and a fake `<report>` tag that would trip RPT-001 if
parsed), then asserts both exit code and the full `lint` object are byte-equal
to the baseline. No new lint code referencing the ledger was added. Retry
count: 0.
</coverage>

</report>

## Ship-time retro

**Evidence surveyed:** per-task journals (T-001 through T-004), VET.md
(invocation 1), REPORT.md coverage above, `git diff origin/main`.

**Friction signals:**
- VET.md invocation 1 recorded one blocking drift: the retro's git-diff form
  was `git diff origin/main...HEAD` (three-dot), which silently misses
  uncommitted loop work because the retro runs before the ship commit. The
  holistic-fix changed it to `git diff origin/main` (two-dot) plus a
  load-bearing explanation.

**Dedup check against `AGENTS.md` and rule files:** the lesson below is not
already covered by any durable-tier doc.

**Ledger entry appended to `.speccy/MEMORY.md`:**

See the entry appended to `.speccy/MEMORY.md` in this commit.

**Promotion check:** this is the first entry in the ledger; no cross-spec
affirmation has accumulated. No promotion proposed.

**No other durable lessons** emerged from the T-001 through T-004 journals
(all four tasks passed on round 1 with no blocking verdicts or retry rounds in
the task-level review loop).
