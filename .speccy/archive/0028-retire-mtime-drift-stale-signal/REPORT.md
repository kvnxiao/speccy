---
spec: SPEC-0028
outcome: delivered
generated_at: 2026-05-18T03:42:20Z
---

# Report: SPEC-0028 Retire StaleReason::MtimeDrift; HashDrift is the sole semantic stale signal

<report spec="SPEC-0028">

## Outcome

delivered

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
`StaleReason::MtimeDrift` and its `as_str` arm are removed from
`speccy-core/src/workspace.rs`; `stale_for` lost its two
`Option<SystemTime>` parameters along with the mtime-comparison
block; `parse_one_spec_dir` no longer captures filesystem metadata
for mtime purposes; call sites in `speccy-cli/src/status.rs` were
updated to the new two-argument signature.

CHK-001's four paragraphs are satisfied by:

- `cargo test --workspace -p speccy-core --test stale_detection`
  (the renamed `fresh_when_hash_matches`,
  `hash_drift_fires_alone_when_spec_body_changes`, and
  `bootstrap_pending_short_circuits_other_reasons` tests assert the
  surviving two-reason contract).
- DEC-005 dogfood verification recorded in T-001's implementer note
  (`speccy status SPEC-0028` printed a clean row with no `stale:`
  line on the working tree mid-ship, with no `touch TASKS.md`
  workaround).
- `speccy verify` exits 0 with 0 warnings against the post-ship tree
  (verified locally; see the Task summary for the exact run).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
The mtime-comparison branch in `tsk_003_staleness`
(`speccy-core/src/lint/rules/tsk.rs`) was excised, along with the
preceding `return` statement that existed solely to prevent the
hash-mismatch and mtime-drift branches from firing together. The
`spec_md_mtime` and `tasks_md_mtime` fields on the shared
`ParsedSpec` (`speccy-core/src/lint/types.rs`) were removed, as was
the `use std::time::SystemTime;` import. The TSK-003 bootstrap and
hash-mismatch branches stay verbatim.

CHK-002 is satisfied by:

- `cargo test --workspace` (the lint-rule tests still cover the
  surviving TSK-003 branches via `speccy-core/tests/lint_common`
  fixtures).
- A targeted `grep -n 'spec_mtime|tasks_mtime|MtimeDrift|mtime drift|mtime-drift' speccy-core/src/lint/`
  returns zero matches.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
SPEC-0004's REQ-002 prose (`<done-when>`, `<behavior>`,
`<scenario id="CHK-003">`) was edited in place to drop every
`MtimeDrift` mention. The REQ-007 declared-order parenthetical and
the `### Interfaces` Rust snippet both lost `MtimeDrift` too. The
obsolete `<assumptions>` bullet that claimed filesystem mtime was
used for staleness detection was deleted on the T-002 retry after
the business reviewer flagged the resulting self-contradiction. A
single 2026-05-18 Changelog row records both the REQ-002 narrowing
and the assumption drop, citing SPEC-0028.

`docs/ARCHITECTURE.md` was edited in two places: the two-way
staleness-detection block at line ~1486 was collapsed to a
content-hash-only paragraph, and the Threat Model bullet at line
~1888 was shortened from "hash or mtime drift" to "hash drift".

CHK-003 is satisfied by:

- `grep -n 'MtimeDrift|mtime drift|mtime-drift' .speccy/specs/0004-status-command/SPEC.md`
  returns a single match inside the `<changelog>` block (explicitly
  permitted as historical context by CHK-003 paragraph 1).
- `grep -n 'mtime|Modification time' docs/ARCHITECTURE.md`
  returns zero matches inside the staleness-detection narrative.
- `cargo test -p speccy-core --test in_tree_specs` exits 0 (the
  SPEC-0004 id-set in the snapshot fixture was unaffected by inline
  prose edits to existing requirements).
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
`speccy-core/tests/stale_detection.rs` was rewritten in place:
`mtime_drift_when_spec_newer_than_tasks` was deleted;
`both_drifts_present_in_declared_order` was renamed to
`hash_drift_fires_alone_when_spec_body_changes` and its assertion
collapsed to `vec![StaleReason::HashDrift]`;
`fresh_when_hash_matches_and_mtime_within` was renamed to
`fresh_when_hash_matches`; the `read_mtime` helper and the
`std::time::{Duration, SystemTime}` imports were dropped. The
`bootstrap_pending_short_circuits_other_reasons` test no longer
synthesises mtimes.

CHK-004 is satisfied by `cargo test --workspace -p speccy-core --test stale_detection`
(exit 0; test count strictly smaller than before because
`mtime_drift_when_spec_newer_than_tasks` was deleted).
</coverage>

## Task summary

- Total tasks: 2.
- Retried: 1 (T-002 retried once after the business reviewer
  flagged that SPEC-0004's `<assumptions>` block still claimed
  filesystem mtime was used for staleness detection, leaving the
  amended SPEC-0004 self-contradictory).
- SPEC amendments triggered: 0 (the retry resolved itself via
  SPEC-0004's own Changelog convention; SPEC-0028 itself was not
  amended in flight).
- DEC-005 dogfood verification: confirmed mid-ship by running
  `speccy status SPEC-0028` against the post-ship tree — the row
  prints clean with no `stale:` line and no `touch TASKS.md`
  workaround. This is the dogfood proof that the deletion lands
  cleanly on itself.
- Final hygiene gate (post-status-flip):
  `speccy verify` → 0 errors, 0 warnings, 49 info across 28 specs,
  147 requirements, 185 scenarios.

## Out-of-scope items absorbed

- Removed the `return;` statement at the top of `tsk_003_staleness`
  that previously existed only to prevent the hash-mismatch and
  mtime-drift branches from both firing. With the mtime branch
  gone, the early return is unnecessary; leaving it would be dead
  control flow with no behavioural effect. This is a one-line tidy
  inside the same function the SPEC already targets, so it lands
  with REQ-002 rather than as a follow-up.
- Re-recorded SPEC-0004's TASKS.md `spec_hash_at_generation` via
  `speccy tasks SPEC-0004 --commit` twice (once at the end of T-002
  initial pass, once after the T-002 retry's `<assumptions>` edit)
  to reconcile the legitimate hash drift introduced by the in-place
  REQ-003 amendments. This is the speccy-amend pattern as documented
  under DEC-004; no scope expansion.

## Skill updates

(none)

## Deferred / known limitations

- The procedural rule "when narrowing a requirement of another SPEC
  inline via the Changelog convention, additionally grep the target
  SPEC's `<assumptions>` block for now-obsolete bullets" surfaced
  on the T-002 retry but is not documented anywhere in
  `AGENTS.md` or the `speccy-amend` skill body. T-002's
  implementer note flagged this under Procedural compliance.
  Adding the rule to either surface would be a non-trivial edit
  with cross-cutting reach (every future SPEC amendment) and was
  intentionally left out of SPEC-0028's scope.

</report>
