---
id: SPEC-0028
slug: retire-mtime-drift-stale-signal
title: Retire StaleReason::MtimeDrift; HashDrift is the sole semantic stale signal
status: implemented
created: 2026-05-18
supersedes: []
---

# SPEC-0028: Retire StaleReason::MtimeDrift; HashDrift is the sole semantic stale signal

## Summary

`speccy status`, `speccy check`, and `speccy verify` currently
report `stale: mtime-drift` (and a paired TSK-003 warning) for
every shipped spec on every developer's machine, every time. The
trigger is structural: `speccy-ship` writes the SPEC.md
`status: in-progress` → `status: implemented` flip *after* TASKS.md
is final, so SPEC.md mtime ends up strictly greater than TASKS.md
mtime. The `MtimeDrift` reason in `speccy_core::workspace::stale_for`
fires independently of `HashDrift` (`speccy-core/src/workspace.rs:226-230`),
and the mtime branch of `TSK-003` mirrors the same logic
(`speccy-core/src/lint/rules/tsk.rs:202-213`). A subsequent
`git pull` that updates only SPEC.md refreshes its mtime against
an unchanged TASKS.md, reproducing the drift for every puller.

The signal was never semantic. It was kept as a fallback before
SPEC-0024 made the hash semantic (canonical-frontmatter-minus-status
+ body bytes); now every meaningful drift is caught by `HashDrift`.
The only edits MtimeDrift can catch that HashDrift doesn't are
`status:` field flips and bare `touch SPEC.md` calls — and
SPEC-0024 explicitly enumerated those as intentionally invisible:
"No new staleness signal for 'frontmatter-only edit since hash
committed.' Once the hash scope excludes `status`, that signal is
intentionally invisible; if a future need arises to detect
frontmatter-only edits, that lands in its own spec."
(`.speccy/specs/0024-meaningful-hash-semantics/SPEC.md:111-114`).

This spec deletes the signal. The `StaleReason::MtimeDrift` enum
variant goes, the mtime branch in `stale_for` goes, the mtime
branch in `tsk_003_staleness` goes, and the `spec_md_mtime` /
`tasks_md_mtime` capture sites and `ParsedSpec` fields go (no
other rule consumes them). The dedicated test
`mtime_drift_when_spec_newer_than_tasks` is deleted;
`both_drifts_present_in_declared_order` collapses to a
`HashDrift`-only assertion. SPEC-0004's enumeration of staleness
reasons is amended via the changelog convention (Changelog row +
inline edit to REQ-002), and ARCHITECTURE.md's two mtime
references (`docs/ARCHITECTURE.md:1493` and `:1891`) are
trimmed to match.

The change is a deletion, not a redesign. `HashDrift` and
`BootstrapPending` keep their existing semantics, declared order,
JSON shapes, lint codes, and exit-code contracts. No CLI flag is
added, no migration path is required (the signal was a
non-blocking warning), and no on-disk artifact format changes.

## Goals

<goals>
- `speccy status`, `speccy check`, and `speccy verify` no longer
  report `mtime-drift` for any spec under any filesystem state.
  The only stale reasons that survive are `HashDrift` and
  `BootstrapPending`.
- The `StaleReason` enum has two variants (`HashDrift`,
  `BootstrapPending`), down from three. The `as_str` mapping
  drops the `"mtime-drift"` arm.
- `speccy_core::workspace::stale_for`'s signature loses the two
  `Option<SystemTime>` mtime parameters; call sites in
  `speccy-cli/src/status.rs` stop passing them.
- The `spec_md_mtime` / `tasks_md_mtime` fields on
  `speccy_core::workspace::ParsedSpec` and on the lint scanner
  context (`speccy_core::lint::types`) are removed along with
  the `fs_err::metadata(…).modified()` capture sites that
  populate them.
- `speccy-core/tests/stale_detection.rs` drops the dedicated
  `mtime_drift_when_spec_newer_than_tasks` test. The
  `both_drifts_present_in_declared_order` test is renamed to
  `hash_drift_only_when_spec_body_changes` (or similar) and
  asserts a single-reason `HashDrift` outcome.
- SPEC-0004 carries a `## Changelog` row noting the REQ-002
  reduction from three reasons to two, and REQ-002's prose +
  `<done-when>` / `<behavior>` / `<scenario>` blocks are edited
  in place to drop every `MtimeDrift` mention.
- `docs/ARCHITECTURE.md` references to mtime drift (the
  "Modification time" fallback bullet at line ~1493 and the
  staleness summary at line ~1891) are trimmed to match.
</goals>

## Non-goals

<non-goals>
- No new staleness signal that detects "frontmatter-only edit
  since hash committed" or "status-only flip post-TASKS-generation".
  SPEC-0024 stated this is intentionally invisible; SPEC-0028
  honours that intent rather than reintroducing the signal under
  a new name.
- No change to `HashDrift`'s computation, its position in
  declared order, or the `"hash-drift"` JSON string. SPEC-0024
  owns the hash semantics; SPEC-0028 leaves them untouched.
- No change to `BootstrapPending`'s short-circuit behaviour, its
  sentinel value (`bootstrap-pending`), or the
  `"bootstrap-pending"` JSON string.
- No change to TSK-003's severity (`Warn` for non-bootstrap,
  `Info` for bootstrap) or its diagnostic message format for the
  surviving branches. Only the mtime-drift code path is excised.
- No CLI flag (`--strict`, `--no-mtime-check`, `--allow-stale`)
  added or removed. There was no flag to begin with; removing
  the signal removes the need for one.
- No alteration to the `Staleness` struct's public shape beyond
  what falls out of dropping one enum variant. `stale: bool` and
  `reasons: Vec<StaleReason>` stay; `reasons` is now bounded by
  the smaller enum.
- No JSON schema version bump. The on-wire fields are
  `stale_reasons: Vec<String>` and `stale: bool`; the string set
  shrinks from `{"hash-drift", "mtime-drift", "bootstrap-pending"}`
  to `{"hash-drift", "bootstrap-pending"}` but the shape is
  identical. Consumers that switch on `mtime-drift` get fewer
  matches, never more; `schema_version` stays at 1.
- No migration path or grace period. `MtimeDrift` was a soft
  warning; nothing depends on it firing. Deleting it on one
  commit is safe.
- No change to `speccy-ship`'s write order. The skill still
  flips SPEC.md status after TASKS.md is final; the SPEC just
  stops complaining about the resulting mtime ordering.
- No edits to TASKS.md for already-shipped specs. The local
  workaround (`touch TASKS.md`) becomes unnecessary on the
  commit that lands this SPEC; prior `touch`-induced mtime
  values are irrelevant once the check is gone.
- No `speccy migrate stale-signal` command or similar one-shot
  helper. Deletion alone is the whole change.
</non-goals>

## User Stories

<user-stories>
- As a developer who just ran `speccy-ship` and committed,
  I want `speccy status` afterwards to report a clean
  workspace if the hash matches. Today I see a phantom
  `stale: mtime-drift` and have to `touch TASKS.md` to silence
  it; after this SPEC I no longer see the phantom.
- As a developer who pulled a colleague's ship commit (one that
  modified SPEC.md but not TASKS.md), I want `speccy status` on
  my machine to match the colleague's: clean. Today the pull
  refreshes SPEC.md mtime against an unchanged TASKS.md mtime
  and the signal fires for me even though nothing semantically
  drifted.
- As a CI maintainer, I want `speccy verify` warnings to stay
  meaningful. Today `verify` emits one TSK-003 mtime warning
  per shipped spec post-ship — noise that trains operators to
  ignore the entire TSK-003 family. After this SPEC, TSK-003
  warnings indicate genuine drift (hash mismatch) and are worth
  reading.
- As the speccy maintainer reading `git log --grep="stale"`, I
  want the staleness contract to be one signal (`HashDrift`) plus
  the bootstrap sentinel, not two parallel signals with one
  documented as redundant. The simpler surface is easier to
  reason about when amending downstream lint rules.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `StaleReason::MtimeDrift` and its branches are deleted

The `MtimeDrift` variant of `speccy_core::workspace::StaleReason`
is removed from the enum. The `stale_for` function loses its two
`Option<SystemTime>` mtime parameters and the mtime-comparison
branch that pushed `MtimeDrift` into the reasons vector. Call
sites in `speccy-cli/src/status.rs` update to the new signature.
The `as_str` mapping drops the `"mtime-drift"` arm. The
`StaleReason` enum has exactly two variants after this requirement
lands: `HashDrift` and `BootstrapPending`.

<done-when>
- `grep -rn "MtimeDrift" speccy-core/ speccy-cli/` returns zero
  matches in non-test source files (matches in deleted-test
  bodies are acceptable transiently but should be cleaned up by
  REQ-004's test removal in the same task).
- `speccy_core::workspace::StaleReason` has two variants
  (`HashDrift`, `BootstrapPending`); a fresh `cargo doc` and
  `cargo expand` confirms the third variant is gone.
- `stale_for`'s signature is `fn stale_for(spec: &SpecMd, tasks:
  Option<&TasksDoc>) -> Staleness` (no `Option<SystemTime>`
  parameters).
- `speccy_core::workspace::ParsedSpec` no longer has
  `spec_md_mtime` or `tasks_md_mtime` fields, and
  `parse_one_spec_dir` no longer captures filesystem metadata
  for mtime purposes (the `fs_err::metadata(…).modified()` calls
  that previously populated those fields are gone).
- `cargo test --workspace` and `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` exit with status
  0 after the removal (modulo the pre-existing
  `result_large_err` carried forward across SPECs 0026, 0027).
- `cargo +nightly fmt --all --check` exits 0.
</done-when>

<behavior>
- Given a workspace where SPEC.md mtime > TASKS.md mtime and the
  hash matches, when `speccy status` runs, then no spec row
  carries a `stale: mtime-drift` line and the workspace summary
  shows zero stale specs.
- Given the same workspace, when `speccy verify` runs, then no
  TSK-003 warning is emitted for the mtime-only case.
- Given a workspace where SPEC.md content changed (hash
  mismatch) and mtime is also newer, when `speccy status` runs,
  then the spec row carries `stale: hash-drift` (single reason)
  rather than `stale: hash-drift, mtime-drift`.
- Given the rendered JSON of `speccy status --all --json`, when
  the union of all `stale_reasons` arrays across all specs is
  collected, then no array contains the string `"mtime-drift"`.
</behavior>

<scenario id="CHK-001">
Given a workspace where SPEC.md mtime is strictly greater than
TASKS.md mtime (e.g., simulated via `touch SPEC.md` after a
fresh checkout) but the hash matches the value stored in
TASKS.md's `spec_hash_at_generation`,
when `speccy status` runs against that workspace,
then the spec's row in the output carries no `stale:` line and
the JSON `stale_reasons` array for that spec is empty.

Given the same workspace,
when `speccy verify` runs and its exit code is captured,
then the captured exit code is 0 and the verify summary line
`Lint: N errors, M warnings, …` reports zero warnings
attributable to the previously-firing TSK-003 mtime branch
(the only TSK-003 warning that should ever appear after this
SPEC is the hash-mismatch one).

Given the `StaleReason` enum in `speccy-core/src/workspace.rs`
after this requirement lands,
when its variants are enumerated by reflection (rustdoc, IDE
type inspection, or `cargo expand`),
then the enumeration yields exactly two members: `HashDrift`
and `BootstrapPending`.

Given the public signature of `speccy_core::workspace::stale_for`
after this requirement lands,
when introspected via rustdoc or grep against the function
definition line,
then the parameter list is exactly
`(spec: &SpecMd, tasks: Option<&TasksDoc>)` — no
`Option<SystemTime>` parameters.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: TSK-003 lint's mtime-drift branch is deleted

The mtime-comparison branch in
`speccy_core::lint::rules::tsk::tsk_003_staleness`
(lines 202-213, the `if let (Some(spec_mtime), Some(tasks_mtime))
= (spec.spec_md_mtime, spec.tasks_md_mtime) && spec_mtime >
tasks_mtime` block) is removed along with its diagnostic message
("TASKS.md may be stale: SPEC.md mtime is newer than TASKS.md
mtime. Run `/speccy:amend` to reconcile.").

The `spec_md_mtime` and `tasks_md_mtime` fields on
`speccy_core::lint::types::ParsedSpec` (or whichever lint
context struct carries them) are removed. The populating code
in `speccy_core::lint::scanner` (or wherever the lint context
is constructed) stops capturing filesystem mtime for SPEC.md /
TASKS.md.

`TSK-003`'s remaining branches are unchanged: the bootstrap-pending
info diagnostic stays, and the hash-mismatch warning stays.

<done-when>
- `grep -n "spec_mtime\|tasks_mtime\|MtimeDrift\|mtime drift\|mtime-drift" speccy-core/src/lint/`
  returns zero matches.
- The lint context struct (`ParsedSpec` in
  `speccy-core/src/lint/types.rs` or the equivalent post-rename)
  has no `spec_md_mtime` / `tasks_md_mtime` fields.
- Running `speccy check SPEC-NNNN` against a workspace where the
  spec has SPEC.md mtime > TASKS.md mtime and a matching hash
  produces no TSK-003 diagnostic for that spec.
- Running `speccy check SPEC-NNNN` against a workspace where the
  hash mismatches still produces a TSK-003 warning with the
  existing message format.
</done-when>

<behavior>
- Given a workspace where SPEC.md mtime is strictly greater
  than TASKS.md mtime and the hash matches, when `speccy check`
  runs across the workspace, then the produced diagnostics list
  contains no TSK-003 entry attributable to mtime drift.
- Given the same workspace, when the user introduces a body
  edit to SPEC.md that changes the hash, then a TSK-003 warning
  appears citing the hash mismatch (the surviving branch
  continues to fire correctly).
- Given `speccy_core::lint::rules::tsk::tsk_003_staleness` after
  this requirement lands, when grepped for the identifier
  `spec_mtime` or `tasks_mtime`, then zero matches are found.
</behavior>

<scenario id="CHK-002">
Given the lint context (`ParsedSpec` in
`speccy-core/src/lint/types.rs` or the equivalent struct that
feeds the rule modules) after this requirement lands,
when its public fields are enumerated,
then no field of type `Option<SystemTime>` named `spec_md_mtime`
or `tasks_md_mtime` exists.

Given a workspace where SPEC.md was touched (mtime bumped) but
the body bytes and the canonical-frontmatter-minus-status are
unchanged from the value hashed at TASKS.md generation,
when `speccy check` runs against the workspace and its JSON
output is captured,
then no diagnostic with `code = "TSK-003"` is emitted for the
affected spec.

Given the same workspace after a real body edit that changes
the hash,
when `speccy check` runs and its JSON output is captured,
then exactly one TSK-003 diagnostic is emitted for the spec
with the existing "TASKS.md may be stale: stored
`spec_hash_at_generation` = … but current SPEC.md sha256 = …"
message format.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: SPEC-0004 and ARCHITECTURE.md are updated to match

SPEC-0004's enumeration of staleness reasons is amended in place
to drop every `MtimeDrift` mention: the REQ-002 prose, the
`<done-when>` bullets, the `<behavior>` Given/When/Then prose,
and the CHK-003 scenario all lose their mtime references. A
`## Changelog` row is appended to SPEC-0004 noting the reduction
from three reasons to two and citing SPEC-0028.

`docs/ARCHITECTURE.md` is edited in two places to match: the
"Modification time" bullet under the staleness detection section
(currently around line 1493) is removed, and the "(hash or mtime
drift)" parenthetical (currently around line 1891) is shortened
to "(hash drift)".

<done-when>
- `grep -n "MtimeDrift\|mtime drift\|mtime-drift" .speccy/specs/0004-status-command/SPEC.md`
  returns zero matches in the requirement / behavior / scenario
  prose. (The new `## Changelog` row referencing the removal is
  allowed to mention `MtimeDrift` once historically.)
- SPEC-0004's `## Changelog` carries a new row dated 2026-05-18
  (or the actual ship date) with reason "REQ-002 reduced from
  three staleness reasons to two; MtimeDrift retired per
  SPEC-0028" and a link/reference to SPEC-0028.
- `grep -n "mtime" docs/ARCHITECTURE.md` returns zero matches
  in the staleness-detection prose. Other unrelated uses of
  "mtime" elsewhere in ARCHITECTURE.md (if any) are left
  untouched.
- `speccy verify` exits 0 after both files are edited.
</done-when>

<behavior>
- Given SPEC-0004's SPEC.md after this requirement lands, when
  REQ-002's full subtree (prose, `<done-when>`, `<behavior>`,
  CHK-003) is read, then the words "mtime", "MtimeDrift", and
  the string "modification time" do not appear (except possibly
  in the prose history of the `## Changelog`).
- Given `docs/ARCHITECTURE.md` after this requirement lands,
  when its staleness section is read, then the only stale
  signal it lists is `HashDrift` (plus the `BootstrapPending`
  sentinel that lives elsewhere in the architecture doc).
</behavior>

<scenario id="CHK-003">
Given the file `.speccy/specs/0004-status-command/SPEC.md`
after this requirement lands,
when grepped for the literal substring `MtimeDrift`,
then the only match (if any) appears inside the
`## Changelog` block as historical context, and no match
appears inside any `<requirement>`, `<done-when>`,
`<behavior>`, or `<scenario>` element body.

Given the same file's `## Changelog` after this requirement
lands,
when its rows are read,
then at least one row references SPEC-0028 as the source of
the REQ-002 reduction and the row's `reason` column names the
specific change ("removed MtimeDrift" or substantially
equivalent prose).

Given `docs/ARCHITECTURE.md` after this requirement lands,
when scanned for the substrings `mtime` and `Modification
time` inside the staleness-detection narrative,
then both substrings are absent from that narrative.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `stale_detection.rs` tests collapse to the surviving signals

The test file `speccy-core/tests/stale_detection.rs` is updated
in place:

- `mtime_drift_when_spec_newer_than_tasks` (the test that
  synthesises spec_mtime / tasks_mtime via `SystemTime::now()`
  and asserts the lone `MtimeDrift` outcome) is deleted.
- `both_drifts_present_in_declared_order` is renamed (e.g.,
  `hash_drift_fires_alone_when_spec_body_changes`) and its
  assertion is reduced to `vec![StaleReason::HashDrift]` — the
  `MtimeDrift` half of the prior tuple goes away.
- Any `use` of `MtimeDrift` from the test file is removed.
- Any helper that synthesised `Option<SystemTime>` values
  purely to feed the deleted mtime branch is removed; helpers
  that serve the surviving tests stay.

<done-when>
- `grep -n "MtimeDrift\|spec_mtime\|tasks_mtime"
  speccy-core/tests/stale_detection.rs` returns zero matches.
- The test file contains no test function whose body inspects
  filesystem mtime via `SystemTime::now()` for the purpose of
  driving `stale_for`'s removed branch.
- `cargo test --workspace -p speccy-core --test stale_detection`
  exits 0 with the reduced test set.
- The surviving tests still cover: (a) hash match + no
  bootstrap sentinel + `stale_for` returns fresh, (b) hash
  mismatch + `stale_for` returns `vec![HashDrift]`, (c) bootstrap
  sentinel + `stale_for` returns `vec![BootstrapPending]` and
  short-circuits.
</done-when>

<behavior>
- Given `speccy-core/tests/stale_detection.rs` after this
  requirement lands, when the test function names are
  enumerated, then no name contains the substring `mtime`.
- Given the surviving `both_drifts_present_in_declared_order`
  (or its renamed successor) after this requirement lands,
  when its assertion is read, then it expects a
  single-element vector `vec![StaleReason::HashDrift]`.
- Given `cargo test --workspace`, when run after this
  requirement lands, then the run exits 0 and the
  stale_detection test crate reports a strictly smaller test
  count than before (one test deleted, others preserved or
  renamed).
</behavior>

<scenario id="CHK-004">
Given the file `speccy-core/tests/stale_detection.rs` after
this requirement lands,
when its `#[test]` function names are enumerated,
then no function name contains the substring `mtime` and no
function body imports or names `MtimeDrift`.

Given the surviving test that was previously
`both_drifts_present_in_declared_order`,
when its assertion against `result.reasons` is read,
then the expected value is `vec![StaleReason::HashDrift]`
(a single-element vector), not the prior two-element
`vec![StaleReason::HashDrift, StaleReason::MtimeDrift]`.

Given `cargo test --workspace -p speccy-core --test
stale_detection` after this requirement lands,
when run, then the exit code is 0.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001" status="accepted">
### DEC-001: Delete rather than gate or rename

The mtime check is dropped entirely, not gated behind a
configuration flag, renamed to a softer-sounding signal, or
demoted from `Warn` to `Info`. Configuration flags violate
Principle 5 ("stay small — no mode toggles"). Renaming
preserves the same false-positive on every shipped spec with
new wording. Demotion to Info still floods status output with
a structurally meaningless line per shipped spec. Deletion is
the only option that matches both SPEC-0024's stated intent
and AGENTS.md Principle 5.
</decision>

<decision id="DEC-002" status="accepted">
### DEC-002: Drop the mtime capture sites too, not just the comparison

`fs_err::metadata(spec_md_path).ok().and_then(|m| m.modified().ok())`
in `parse_one_spec_dir` (and its TASKS.md sibling) becomes
dead code once both consumers (`stale_for` and
`tsk_003_staleness`) drop their branches. The metadata
syscalls also vanish — minor I/O reduction (one per spec per
workspace scan), more importantly aligning the data shape
with the surviving contract. `ParsedSpec` and lint
`ParsedSpec` both shed two `Option<SystemTime>` fields each;
the smaller struct is easier to test-construct and easier to
reason about.
</decision>

<decision id="DEC-003" status="accepted">
### DEC-003: No migration shim, no schema bump

The on-wire JSON shape stays at `schema_version: 1`. The
`stale_reasons` array's value set shrinks (no
`"mtime-drift"` ever appears) but never grows. Consumers
that switched on `"mtime-drift"` get fewer matches, never
unrecognised strings; the change is forward-compatible by
construction. No `schema_version: 2`, no deprecation window,
no `--legacy-stale-reasons` flag. The signal was a
non-blocking warning; nothing in the documented Speccy
contract depends on it firing.
</decision>

<decision id="DEC-004" status="accepted">
### DEC-004: Edit SPEC-0004 in place, do not formally supersede

SPEC-0004 stays the canonical source for the staleness
contract; SPEC-0028 narrows it. The amendment goes inline (a
`## Changelog` row plus surgical edits to REQ-002's prose,
done-when, behavior, and CHK-003 blocks) rather than via a
`supersedes: [SPEC-0004]` declaration. Full supersession
would require re-stating REQ-001 (`speccy status` text
output), REQ-003 (lint surfacing), and REQ-004 (JSON output)
unchanged — verbose and invites drift between the restated
prose and the surviving REQ-002 text. The Changelog
convention exists precisely for this kind of narrowing
amendment.
</decision>

<decision id="DEC-005" status="accepted">
### DEC-005: Touch the staleness lint test for SPEC-0028 itself

After this SPEC ships, every prior shipped spec's
`mtime-drift` ceases to fire — that is the win, but it also
means the SPEC-0028 ship commit itself will produce
`stale: ` lines for SPEC-0028 only if HashDrift fires (and
it will not, since `speccy tasks --commit` runs at the
correct moment). This means SPEC-0028's own ship can land
without the `touch TASKS.md` workaround that SPECs 0026 and
0027 required. The implementer should verify this by running
`speccy status` between the status flip and the commit,
confirming a clean line, and noting the outcome in
REPORT.md.
</decision>

## Open questions

- [x] Should the deleted `mtime-drift` JSON value be reserved
      against future re-use (e.g., a comment in the
      `StaleReason::as_str` arm warning future contributors not
      to revive the string under different semantics)? Lean
      no: the comment would drift; the SPEC-0024 +
      SPEC-0028 history in `.speccy/specs/` is the durable
      record. Revisit if anyone proposes a "frontmatter-only
      edit" signal that happens to want the same wire string.
      **Resolved (T-001): no placeholder added; SPEC history is
      the durable record.**

## Assumptions

<assumptions>
- `git pull` against a commit that modifies SPEC.md but not
  TASKS.md updates SPEC.md's local mtime against an unchanged
  TASKS.md mtime. Verified empirically on git ≥ 2.20 (default
  behavior); the SPEC-0028 bug report is reproducible by
  `touch SPEC.md` against a fresh checkout.
- No third-party tool, dashboard, or harness consumes the
  string `"mtime-drift"` from `speccy status --json` output.
  This is a v1 internal artifact; no public API consumer has
  been advertised. If one surfaces post-ship, they would
  receive fewer matches, never crash.
- The pre-existing `clippy::result_large_err` against
  `speccy_core::error::ParseError` (carried forward from
  SPEC-0026 T-003) remains out of scope. SPEC-0028 does not
  add new lint suppressions; the existing pin survives.
</assumptions>

## Changelog

<changelog>
| Date       | Reason                                       | Author |
|------------|----------------------------------------------|--------|
| 2026-05-18 | Initial draft. Retire `StaleReason::MtimeDrift` and the TSK-003 mtime branch. Honours SPEC-0024's stated non-goal that frontmatter-only edits should be invisible. | Kevin Xiao |
</changelog>
