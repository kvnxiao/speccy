---
spec: SPEC-0055
generated_at: 2026-06-10T19:32:20Z
---

## Invocation 1 — 2026-06-10T19:32:20Z

<drift-review verdict="pass" round="1" date="2026-06-10T19:32:20Z" model="claude-opus-4-8[1m]/high">
Diff satisfies SPEC-0055 as a unit: all ten requirements' done-when traced to working code; four hygiene gates green; reeject and verify clean on the repo's own state.

Traced every requirement against the full diff:
- REQ-001/002 (task transition): byte-surgical splice via `splice_task_state` (never round-trips the renderer), closed six-edge legal graph + same-state no-op (DEC-003) in transition.rs:108-129; selector grammar shared with `speccy check`.
- REQ-003/005 (journal append + lock): CLI-stamped date/round, validation-before-write, advisory fs4 per-file lock with 10s blocking-acquire timeout (DEC-002/007) in journal.rs:441-509,656-691; cargo deny clean.
- REQ-004 (VET grammar frozen + routing): vet_xml parser with strict + in-flight modes (DEC-008), DEC-004 block-type/selector routing, gate tasks_hash as SHA-256 of sibling TASKS.md, re-parse-before-write in journal.rs:534-634.
- REQ-006 (journal show): schema_version=1 as first field in both envelopes (untagged serde), three conjunctive filters, VET round-within-last-section semantics; text/JSON parity.
- REQ-007 (VET lint family): VET-001/VET-002 split off typed ParseError::VetGateStructure in lint/rules/vet.rs; `speccy verify` passes 0-error on the repo's own VET-less specs and a journal-append-produced VET.md is grammar-valid.
- REQ-008 (thin verdicts): verdict_return_contract.md + persona/phase bodies append own blocks and return a single `<verdict>` element; `just reeject` yields a clean tree (CHK-011: 0 overwritten, empty porcelain).
- REQ-009 (orchestrator/reconcile consume verbs): reconcile-policy auto-fix rows name `task transition`; retry-shape routes reads through `journal show`; vet-phases documents the gate on every exit path incl. Phase 0 early-exits and drops hand-bootstrap of frontmatter/invocation headings.
- REQ-010 (ARCHITECTURE.md): CLI surface, state-model "Who sets it", VET-001/002 catalogue, and the narrowed claim-files/leases exclusion all reflect post-SPEC behavior; the "sole serial writer" prose is replaced by the CLI append-lock contract.

Scope check: CLI adds exactly the three authorized commands (`task transition`, `journal append`, `journal show`) and no unauthorized flag, env var, or config key. The `.speccy/specs/0056-task-context-bundle/` directory in the diff is the agreed follow-up SPEC's planning artifacts only (SPEC.md + TASKS.md, created in their own commits) — no 0056 implementation code leaked in, consistent with the non-goal that defers it. The reconcile-policy `journal_xml_malformed` truncation is a reasoned, documented corruption-recovery exception (no CLI equivalent exists), not a hand-edit bypass of the no-hand-edit rule.

Full suite (cargo test --workspace), clippy -D warnings, and cargo deny all green; changelog's DEC-008 final intent (parser as single authority, in-flight mode) is reflected in the shipped code, not the superseded tolerant-text-scan framing.
</drift-review>
<simplifier-scan verdict="candidates">
Two candidates in the SPEC-0055 Rust diff; the rest of the new code is clean, single-responsibility, and well-tested.

- speccy-cli/src/main.rs:341-507 — the three-arm `LookupError` rendering block (InvalidFormat / NotFound / Ambiguous) is repeated verbatim across `run_transition`, `run_journal_append`, and `run_journal_show`, differing only in the command-name prefix and the one disambiguation example line. Genuine 3-callsite duplication (~25 lines each). Extract a helper like `report_lookup_error(cmd: &str, disambig_suffix: &str, err: &LookupError) -> u8` and call it from the three new commands. Note: a fourth identical copy already exists in the pre-existing `run_check` (main.rs:~700, outside the SPEC-0055 diff) — leave that one untouched to stay in Phase 2 scope; the helper covering the three new sites is self-contained and does not require editing run_check.
- speccy-cli/src/journal.rs:64-72 and speccy-cli/src/journal_show.rs:51-59 — `bare_spec_selector_regex()` is duplicated verbatim (OnceLock + `^SPEC-\d{4,}$`). Weaker candidate: only two sites, ~8 lines, and there is no existing cli-side shared module to host it, so deduping would mean introducing a new shared surface. Borderline; flag for judgment rather than strong recommendation.
</simplifier-scan>
<simplifier-apply verdict="applied">
Both candidates applied; all four hygiene gates green (test, clippy -D warnings, nightly fmt --check, deny check).

- High-value LookupError rendering dedup: extracted `report_lookup_error(cmd, disambig_suffix, &LookupError) -> u8` in main.rs and replaced the three verbatim ~25-line three-arm blocks in `run_transition` / `run_journal_append` / `run_journal_show` with single calls. The pre-existing `run_check` copy was left untouched (outside SPEC-0055 scope). Behavior preserved exactly: the helper carries a `#[non_exhaustive]` wildcard arm that renders `LookupError`'s `Display`, matching the prior `Err(e)` fall-through path for the `Io`/future variants (`TransitionError`/`JournalError`/`ShowError` all forward `TaskLookup` transparently). The now-unused per-fn `use ... LookupError;` imports were removed.
- Borderline `bare_spec_selector_regex` dedup: applied without a new module. Promoted the journal.rs copy to `pub(crate)` and replaced journal_show.rs's verbatim OnceLock copy with `use crate::journal::bare_spec_selector_regex;`. No new shared surface or indirection — journal_show already imports freely from sibling crate modules — so the borderline cost noted in the scan did not materialize.
</simplifier-apply>
<gate verdict="passed" tasks_hash="a1b427ac0b7b7e1922db0104e3e7a54a350ca20e992186dd1c1165d72db2a2cb" date="2026-06-10T19:40:01Z">
Holistic drift cleared on round 1 (all 10 REQ done-when satisfied as a unit); simplifier applied LookupError-rendering + bare_spec_selector_regex dedup with hygiene green.
</gate>
