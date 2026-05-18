---
spec: SPEC-0028
spec_hash_at_generation: 5d5f2c1bba0a61ef66b3565f50acd8af8a2ef3bcd5c111198951590501bc3747
generated_at: 2026-05-18T03:43:50Z
---

# Tasks: SPEC-0028 Retire StaleReason::MtimeDrift; HashDrift is the sole semantic stale signal

<tasks spec="SPEC-0028">

## Phase 1: Delete the mtime signal from code and tests in one atomic edit

<task id="T-001" state="completed" covers="REQ-001 REQ-002 REQ-004">
## T-001: Excise `MtimeDrift` from the enum, `stale_for`, the lint rule, the shared `ParsedSpec`, capture sites, and tests

The data shape changes here are tightly coupled and cannot land
incrementally without an intermediate state that fails to compile
or fails `cargo test --workspace`. Specifically:

- `speccy_core::workspace::StaleReason::MtimeDrift` is consumed by
  both `workspace::stale_for` (REQ-001) and the TSK-003 lint rule
  (REQ-002). Deleting the variant before either consumer is
  updated is a compile error.
- `ParsedSpec::spec_md_mtime` / `ParsedSpec::tasks_md_mtime` are
  declared in `speccy-core/src/lint/types.rs` and re-exported via
  `speccy_core::lint::ParsedSpec` (which is the same struct
  `workspace::ParsedSpec` constructs in `parse_one_spec_dir` —
  verified by `pub use types::ParsedSpec` at
  `speccy-core/src/lint/mod.rs:19`). Removing the fields requires
  updating every constructor in the same edit:
  `speccy-core/src/workspace.rs:436-463` (production),
  `speccy-cli/src/status.rs:615-616` (test-only stub),
  `speccy-core/tests/lint_common/mod.rs:112-131` (lint test
  helper).
- `speccy-core/tests/stale_detection.rs` still calls
  `stale_for(spec, tasks, spec_mtime, tasks_mtime)` and constructs
  `StaleReason::MtimeDrift` directly (REQ-004). The signature change
  and enum-variant removal break compilation of this file unless
  the test file is updated in the same commit.

Land all of the following in a single commit:

### REQ-001: enum and `stale_for` surface

- In `speccy-core/src/workspace.rs`:
  - Delete the `MtimeDrift` variant from
    `StaleReason` (line 98) and its `as_str` arm (line 110).
  - Delete the `use std::time::SystemTime;` import (line 32) once
    no consumer remains.
  - Change `stale_for`'s signature from
    `pub fn stale_for(spec: &SpecMd, tasks: Option<&TasksDoc>,
    spec_mtime: Option<SystemTime>, tasks_mtime: Option<SystemTime>)
    -> Staleness` to
    `pub fn stale_for(spec: &SpecMd, tasks: Option<&TasksDoc>)
    -> Staleness`. Drop the mtime-comparison block at lines
    226-230.
  - Update the function's doc comment (lines 189-197) to drop the
    "spec_mtime / tasks_mtime ..." paragraph.
  - In `parse_one_spec_dir` (lines 436-445), drop the two
    `fs_err::metadata(...).ok().and_then(|m| m.modified().ok())`
    capture sites and the `spec_md_mtime` / `tasks_md_mtime`
    fields of the returned `ParsedSpec` literal (lines 462-463).

- In `speccy-cli/src/status.rs`:
  - Update the `stale_for` call site at lines 226-231 to pass only
    `(spec, parsed.tasks_md_ok())` — drop the
    `parsed.spec_md_mtime` and `parsed.tasks_md_mtime` arguments.
  - Drop the `spec_md_mtime: None, tasks_md_mtime: None` lines
    from the test-only `ParsedSpec` stub at lines 615-616.

### REQ-002: lint rule and shared struct fields

- In `speccy-core/src/lint/types.rs`:
  - Delete the `use std::time::SystemTime;` import (line 10).
  - Delete the `spec_md_mtime: Option<SystemTime>` field (lines
    163-166) and the `tasks_md_mtime: Option<SystemTime>` field
    (lines 167-169) from `ParsedSpec`.

- In `speccy-core/src/lint/rules/tsk.rs`:
  - Delete the `if let (Some(spec_mtime), Some(tasks_mtime)) =
    (spec.spec_md_mtime, spec.tasks_md_mtime) && spec_mtime >
    tasks_mtime { ... }` block at lines 202-213, including its
    "TASKS.md may be stale: SPEC.md mtime is newer than TASKS.md
    mtime. ..." diagnostic. The bootstrap-pending branch and the
    hash-mismatch branch above it stay verbatim.

- In `speccy-core/tests/lint_common/mod.rs`:
  - Drop the `spec_md_mtime` and `tasks_md_mtime` captures at
    lines 112-119 and the corresponding fields in the `ParsedSpec
    { ... }` literal at lines 130-131.

### REQ-004: stale-detection tests collapse to the surviving signals

- In `speccy-core/tests/stale_detection.rs`:
  - Delete the `mtime_drift_when_spec_newer_than_tasks` test (lines
    153-181).
  - Rename `both_drifts_present_in_declared_order` (lines 183-211)
    to a name without the substring `mtime` (suggested:
    `hash_drift_fires_alone_when_spec_body_changes`). Reduce its
    assertion from
    `vec![StaleReason::HashDrift, StaleReason::MtimeDrift]` to
    `vec![StaleReason::HashDrift]`. The mtime synthesis at lines
    200-202 is removed; the fixture stays (it already produces a
    hash mismatch because the tasks frontmatter hash is `0000...`).
  - Drop the `use std::time::Duration;` / `use std::time::SystemTime;`
    imports (lines 20-21) once no surviving test uses them. Verify
    `bootstrap_pending_short_circuits_other_reasons` (lines
    213-239) — it currently synthesises mtimes that are unused by
    the bootstrap short-circuit; the synthesis goes away and the
    `stale_for` call simplifies to `stale_for(&spec, Some(&tasks))`.
  - Drop the `read_mtime` helper (lines 59-63) once no surviving
    test calls it. Update the surviving callers in
    `no_tasks_md_yields_fresh` (line 95) and
    `fresh_when_hash_matches_and_mtime_within` (lines 119-122) and
    `hash_mismatch_yields_hash_drift` (lines 144-147) to drop the
    `spec_mtime` / `tasks_mtime` arguments and the helper calls
    that built them. Rename
    `fresh_when_hash_matches_and_mtime_within` to a name without
    the substring `mtime` (suggested:
    `fresh_when_hash_matches`).

### Snapshot fixture co-edit

The in-tree snapshot test currently fails because SPEC-0028 has
no entry in `speccy-core/tests/fixtures/in_tree_id_snapshot.json`
(verified via `cargo test -p speccy-core --test in_tree_specs`,
which prints "0028-retire-mtime-drift-stale-signal: missing from
pre-migration snapshot fixture"). Add the SPEC-0028 entry under
key `"0028-retire-mtime-drift-stale-signal"` with the four
requirements (`REQ-001`..`REQ-004`), four scenarios
(`CHK-001`..`CHK-004`), and five decisions (`DEC-001`..`DEC-005`)
from `.speccy/specs/0028-retire-mtime-drift-stale-signal/SPEC.md`.
Insert the entry in lexical order between
`"0027-host-native-personas"` and the closing `}` of the JSON
object.

### Hygiene gate

After all edits, the full four-tool gate must pass clean:

- `cargo test --workspace` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0 modulo the carried-forward
  `clippy::result_large_err` against `speccy_core::error::ParseError`
  (pre-existing under SPEC-0026 T-003 procedural compliance; not
  in scope for SPEC-0028).
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0.

DEC-005 verification: between the `speccy-ship`-driven status flip
and the ship commit, run `speccy status` and confirm SPEC-0028's
own row prints clean (no `stale:` line) without any `touch
TASKS.md` workaround. Record the observation in REPORT.md at ship
time. This is dogfood proof that the deletion lands cleanly on
itself.

- Suggested files:
  - `speccy-core/src/workspace.rs`
  - `speccy-core/src/lint/types.rs`
  - `speccy-core/src/lint/rules/tsk.rs`
  - `speccy-cli/src/status.rs`
  - `speccy-core/tests/lint_common/mod.rs`
  - `speccy-core/tests/stale_detection.rs`
  - `speccy-core/tests/fixtures/in_tree_id_snapshot.json`

<task-scenarios>
  - Given the workspace after this task lands, when `grep -rn
    "MtimeDrift\|mtime-drift" speccy-core/src/ speccy-cli/src/`
    runs, then zero matches are found in any production source
    line. (CHK-001 first paragraph: enum variant gone.)
  - Given `speccy_core::workspace::StaleReason`'s public surface
    after this task lands, when its variants are enumerated via
    `cargo doc` / rustdoc / IDE type inspection, then exactly two
    members appear: `HashDrift` and `BootstrapPending`. The
    `as_str` mapping returns `"hash-drift"` and
    `"bootstrap-pending"` and contains no `"mtime-drift"` arm.
    (CHK-001 third paragraph.)
  - Given the public signature of
    `speccy_core::workspace::stale_for` after this task lands, when
    inspected, then the parameter list is exactly
    `(spec: &SpecMd, tasks: Option<&TasksDoc>) -> Staleness` —
    no `Option<SystemTime>` parameters. (CHK-001 fourth paragraph.)
  - Given a workspace where SPEC.md is touched (mtime bumped) but
    body bytes and canonical-frontmatter-minus-status are unchanged
    relative to TASKS.md's stored `spec_hash_at_generation`, when
    `speccy status` runs, then no spec row carries a `stale:` line
    and the JSON `stale_reasons` array for that spec is empty.
    (CHK-001 first scenario paragraph; REQ-001 behavior #1.)
  - Given the same workspace, when `speccy verify` runs and its
    exit code is captured, then the exit code is 0 and no TSK-003
    diagnostic is emitted attributable to the previously-firing
    mtime branch. (CHK-001 second paragraph; CHK-002 second
    paragraph.)
  - Given a workspace where SPEC.md body bytes change (hash
    mismatch) and mtime is also newer, when `speccy status` runs,
    then the spec row carries `stale: hash-drift` as a
    single-reason output (not `stale: hash-drift, mtime-drift`).
    (REQ-001 behavior #3.)
  - Given `speccy-core/src/lint/rules/tsk.rs` after this task
    lands, when grepped for the identifiers `spec_mtime`,
    `tasks_mtime`, and the literal substring `MtimeDrift`, then
    zero matches are found. (REQ-002 behavior #3; CHK-002 third
    paragraph.)
  - Given `speccy-core/src/lint/types.rs`'s `ParsedSpec` struct
    after this task lands, when its public fields are enumerated,
    then no field of type `Option<SystemTime>` named
    `spec_md_mtime` or `tasks_md_mtime` exists, and the
    `use std::time::SystemTime;` import is gone. (CHK-002 first
    paragraph.)
  - Given the same workspace after a real body edit that changes
    the hash, when `speccy check SPEC-NNNN` runs and its JSON
    output is captured, then exactly one TSK-003 diagnostic is
    emitted for the spec carrying the existing
    `"TASKS.md may be stale: stored spec_hash_at_generation = ...
    but current SPEC.md sha256 = ..."` message. (CHK-002 third
    paragraph; REQ-002 behavior #2.)
  - Given `speccy-core/tests/stale_detection.rs` after this task
    lands, when its `#[test]` function names are enumerated, then
    no name contains the substring `mtime` and no function body
    imports `MtimeDrift` or names `spec_mtime` / `tasks_mtime` /
    `SystemTime` / `Duration`. (CHK-004 first paragraph.)
  - Given the surviving test that was previously
    `both_drifts_present_in_declared_order`, when its assertion
    against `result.reasons` is read, then the expected value is
    `vec![StaleReason::HashDrift]` (a single-element vector).
    (CHK-004 second paragraph.)
  - Given `cargo test --workspace -p speccy-core --test
    stale_detection` after this task lands, when run, then the
    exit code is 0 and the test count is strictly smaller than
    before (the `mtime_drift_when_spec_newer_than_tasks` test is
    deleted). (CHK-004 third paragraph.)
  - Given `cargo test -p speccy-core --test in_tree_specs` after
    this task lands, when run, then the exit code is 0 — SPEC-0028
    appears in `tests/fixtures/in_tree_id_snapshot.json` with its
    four REQ ids, four CHK ids, and five DEC ids matching the
    spec body.
  - Given the full hygiene gate (`cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`, `cargo deny
    check`) after this task lands, when each command runs, then
    each exits with status 0 modulo the carried-forward
    `result_large_err` on `ParseError` (documented under
    SPEC-0026 T-003 as out-of-scope).
  - Given the rendered JSON of `speccy status --all --json` against
    any workspace after this task lands, when the union of all
    `stale_reasons` arrays is collected, then no array contains
    the string `"mtime-drift"`. (REQ-001 behavior #4.)
</task-scenarios>

- Implementer note (session-2026-05-17-spec0028-t001):
  - Completed: deleted `StaleReason::MtimeDrift` variant and its `as_str` arm; dropped `stale_for`'s two `Option<SystemTime>` mtime params and the mtime-comparison branch; removed `use std::time::SystemTime` from `workspace.rs` and `lint/types.rs`; removed `spec_md_mtime` / `tasks_md_mtime` fields from both `ParsedSpec` structs (workspace + lint context); excised the mtime branch in `tsk_003_staleness`; updated the three `ParsedSpec` constructors (`workspace::parse_one_spec_dir`, `status::fake_parsed` test stub, `lint_common::parse_lint_fixture` test helper); rewrote `stale_detection.rs` to drop `mtime_drift_when_spec_newer_than_tasks`, rename `both_drifts_present_in_declared_order` to `hash_drift_fires_alone_when_spec_body_changes`, rename `fresh_when_hash_matches_and_mtime_within` to `fresh_when_hash_matches`, drop the `read_mtime` helper, and drop `SystemTime` / `Duration` imports; added SPEC-0028 entry to `in_tree_id_snapshot.json` between `0027-host-native-personas` and the closing brace.
  - Undone: (none)
  - Commands run: `cargo build --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all --check`; `cargo +nightly fmt --all`; `cargo test -p speccy-core --test in_tree_specs`; `.\target\debug\speccy.exe verify`; `.\target\debug\speccy.exe status SPEC-0028`.
  - Exit codes: 0; 0; non-zero (pre-existing `result_large_err` on `ParseError` only, per SPEC-0026 T-003 carry-forward — no new lints); 1 (one line that fmt fixed); 0; 0; 0 (0 errors, 1 warning attributable to SPEC-0004 hash drift after T-002's edits; cleared by re-recording in T-002); 0 (clean row, no `stale:` line — DEC-005 verified).
  - Discovered issues: (none)
  - Procedural compliance: (none)
- Review (business, pass): diff cleanly retires `MtimeDrift` across the four named surfaces (enum + `as_str`, `stale_for` signature + body, TSK-003 mtime branch, both `ParsedSpec` constructors, all six `stale_for` call/test sites, and the snapshot fixture entry); REQ-001/REQ-002/REQ-004 `<done-when>` and `<behavior>` items all observable in the diff; non-goals respected (no new signal, no schema bump, no migration shim, no flag, `BootstrapPending` short-circuit and `HashDrift` semantics untouched); DEC-005 dogfood verification recorded in implementer note ("clean row, no `stale:` line"); the open question on reserving the `"mtime-drift"` wire string (leaned no) was honored by not adding a placeholder comment in `as_str`.
</task>

## Phase 2: Update the documentation surface to match

<task id="T-002" state="completed" covers="REQ-003">
## T-002: Edit SPEC-0004's REQ-002 in place and trim ARCHITECTURE.md's two mtime references

After T-001 lands, the staleness contract is one signal
(`HashDrift`) plus the bootstrap sentinel. The two upstream
documents that still describe the old three-reason contract need to
match the new code.

Edits in this task are docs-only — no code changes. They land in a
separate commit (or as a second slice in a combined PR) so a
reviewer can verify the doc edits are surgical and don't accidentally
touch surrounding prose.

### SPEC-0004 amendment (inline, no formal supersession)

Per DEC-004 of SPEC-0028, SPEC-0004 is amended in place via the
Changelog convention. Do not declare
`supersedes: [SPEC-0004]` on SPEC-0028.

Edit `.speccy/specs/0004-status-command/SPEC.md`:

- In `<requirement id="REQ-002">` (lines 119-164):
  - `<done-when>` bullets: drop the bullet enumerating
    `MtimeDrift` (line 127, "`HashDrift`, `MtimeDrift`,
    `BootstrapPending`" → "`HashDrift`, `BootstrapPending`") and
    delete the dedicated `MtimeDrift` definition bullet (lines
    131-132).
  - `<behavior>` bullets: delete the `Given hash match but SPEC.md
    mtime > TASKS.md mtime, then ...` bullet (lines 145-146).
  - `<scenario id="CHK-003">` body: delete the matching
    `Given hash match but SPEC.md mtime > TASKS.md mtime, then ...`
    bullet (lines 156-157) and update the trailing one-line
    summary at line 161 to drop the `MtimeDrift` mention
    (suggested replacement: `"stale_for returns HashDrift or
    BootstrapPending appropriately; bootstrap-pending sentinel
    short-circuits other reasons; specs without TASKS.md are not
    stale."`).
- In REQ-007's `<done-when>` (lines 334-337), update the
  declared-order parenthetical: `"declared order (\`HashDrift\`,
  \`MtimeDrift\`, \`BootstrapPending\`)"` →
  `"declared order (\`HashDrift\`, \`BootstrapPending\`)"`.
- In the `### Interfaces` Rust snippet at line 499,
  `pub enum StaleReason { HashDrift, MtimeDrift, BootstrapPending }`
  → `pub enum StaleReason { HashDrift, BootstrapPending }`.
- Append a new row to the `<changelog>` block (lines 549-556):
  `| 2026-05-18 | agent/claude | REQ-002 narrowed: removed MtimeDrift per SPEC-0028 (mtime drift is no longer reported as a staleness reason; HashDrift and BootstrapPending are the only surviving signals). |`
  (Or the actual ship date if it slips.)

### ARCHITECTURE.md amendments

Edit `.speccy/ARCHITECTURE.md`:

- Lines 1488-1493 (the "Speccy detects this two ways" block):
  collapse to a one-way detection. Replace the numbered list
  ("1. Content hash. ... 2. Modification time. ...") with a
  single sentence describing the hash-only signal (suggested:
  `"Speccy detects this via the content hash: TASKS.md
  frontmatter's \`spec_hash_at_generation\` stores the sha256 of
  SPEC.md at the time TASKS.md was generated. \`speccy status\`
  recomputes the current hash and compares; a mismatch is the
  sole stale signal beyond the \`bootstrap-pending\` sentinel."`).
  Adjust the "If either drifts" phrasing immediately below
  (line 1495) to "If it drifts" or equivalent.
- Line 1891 (Threat Model bullet list): change
  `"TASKS.md is stale relative to SPEC.md (hash or mtime drift)"`
  to `"TASKS.md is stale relative to SPEC.md (hash drift)"`.

### Hygiene gate

Per REQ-003 done-when:

- `grep -n "MtimeDrift\|mtime drift\|mtime-drift"
  .speccy/specs/0004-status-command/SPEC.md` returns zero matches
  inside `<requirement>`, `<done-when>`, `<behavior>`, and
  `<scenario>` element bodies. A single historical mention in the
  new `<changelog>` row is acceptable per CHK-003.
- `grep -n "mtime\|Modification time" .speccy/ARCHITECTURE.md`
  returns zero matches inside the staleness-detection narrative
  (lines roughly 1480-1505 and the Threat Model bullet at line
  1891). Any unrelated uses of "mtime" elsewhere in the doc (if
  any) are left untouched.
- `cargo test -p speccy-core --test in_tree_specs` exits 0 (the
  SPEC-0004 id-set in `in_tree_id_snapshot.json` is unchanged by
  inline edits to existing requirements; only adding a new REQ /
  CHK / DEC would force a fixture update).
- `speccy verify` exits 0.

- Suggested files:
  - `.speccy/specs/0004-status-command/SPEC.md`
  - `.speccy/ARCHITECTURE.md`

<task-scenarios>
  - Given the file `.speccy/specs/0004-status-command/SPEC.md`
    after this task lands, when grepped for the literal substring
    `MtimeDrift`, then the only match (if any) appears inside the
    `<changelog>` block as historical context; no match appears
    inside any `<requirement>`, `<done-when>`, `<behavior>`, or
    `<scenario>` element body. (CHK-003 first paragraph.)
  - Given the same file's `<changelog>` block after this task
    lands, when its rows are read, then at least one new row
    references SPEC-0028 as the source of the REQ-002 narrowing
    and names the specific change ("removed MtimeDrift" or
    substantially equivalent prose). (CHK-003 second paragraph.)
  - Given `.speccy/ARCHITECTURE.md` after this task lands, when
    scanned for the substrings `mtime` and `Modification time`
    inside the staleness-detection narrative (the prose around
    lines 1480-1505 and the Threat Model bullet at line 1891),
    then both substrings are absent from that narrative. (CHK-003
    third paragraph.)
  - Given the REQ-007 declared-order parenthetical in
    `.speccy/specs/0004-status-command/SPEC.md`, when read after
    this task lands, then the parenthetical reads `(\`HashDrift\`,
    \`BootstrapPending\`)` and does not contain `MtimeDrift`.
  - Given the `### Interfaces` Rust snippet in
    `.speccy/specs/0004-status-command/SPEC.md` around line 499,
    when read after this task lands, then the
    `pub enum StaleReason { ... }` declaration enumerates exactly
    two variants (`HashDrift`, `BootstrapPending`) and does not
    contain `MtimeDrift`.
  - Given `speccy verify` after this task lands, when run against
    the workspace and its exit code is captured, then the exit
    code is 0 and the lint summary reports zero TSK-003 mtime
    warnings (the deletion in T-001 holds and the docs now match).
  - Given `cargo test -p speccy-core --test in_tree_specs` after
    this task lands, when run, then the exit code is 0 — the
    SPEC-0004 id-set in the snapshot fixture is unaffected by
    inline prose edits to existing requirements (only ID-list
    drift would force a fixture update).
</task-scenarios>

- Implementer note (session-2026-05-17-spec0028-t002):
  - Completed: edited `.speccy/specs/0004-status-command/SPEC.md` REQ-002 `<done-when>` (collapsed two bullets to one, dropped the MtimeDrift definition bullet), `<behavior>` (dropped the mtime bullet), `<scenario id="CHK-003">` (dropped the mtime bullet, updated trailing one-line summary); edited REQ-007 declared-order parenthetical to drop MtimeDrift; edited the `### Interfaces` Rust snippet to enumerate two variants; appended a 2026-05-18 changelog row citing SPEC-0028. Edited `.speccy/ARCHITECTURE.md` line ~1486 to collapse the two-way staleness-detection block into a single content-hash sentence and adjust the "If either drifts" phrasing; edited line ~1888 Threat Model bullet from "hash or mtime drift" to "hash drift". Re-recorded SPEC-0004's `TASKS.md` `spec_hash_at_generation` via `speccy tasks SPEC-0004 --commit` to reconcile the legitimate hash drift introduced by the in-place amendment (DEC-004 inline-Changelog convention).
  - Undone: (none)
  - Commands run: `grep -n "MtimeDrift|mtime drift|mtime-drift|Modification time" .speccy/specs/0004-status-command/SPEC.md .speccy/ARCHITECTURE.md`; `.\target\debug\speccy.exe verify`; `.\target\debug\speccy.exe tasks SPEC-0004 --commit`; `cargo test -p speccy-core --test in_tree_specs`; `.\target\debug\speccy.exe verify`.
  - Exit codes: 0 (only the historical Changelog mention remains in SPEC-0004); 0 (1 warning attributable to the pre-recommit SPEC-0004 hash drift); 0; 0 (SPEC-0004 id-set unchanged by inline prose edits, as predicted by the task entry); 0 (0 errors, 0 warnings, 49 info — fully clean).
  - Discovered issues: (none — the legitimate hash drift on SPEC-0004's TASKS.md after the in-place amendment was anticipated by DEC-004 and reconciled via `speccy tasks --commit`, which is the speccy-amend pattern).
  - Procedural compliance: (none)
- Review (business, blocking): SPEC-0004's `<assumptions>` block at `.speccy/specs/0004-status-command/SPEC.md:535-537` still reads "Filesystem mtime is reliable enough for staleness detection. CI environments that mass-touch files at checkout time may produce false-positive staleness; that's acceptable in v1." After T-002's edits this Assumption directly contradicts the surviving REQ-002 prose (which no longer mentions mtime) and the new Changelog row (which says HashDrift and BootstrapPending are the only surviving signals). It also conflicts with SPEC-0028's fourth user story — a maintainer reading SPEC-0004 cold will conclude mtime is still a staleness signal, defeating the "simpler surface" goal. SPEC-0028's REQ-003 and T-002's done-when both scope only to `<requirement>` / `<done-when>` / `<behavior>` / `<scenario>` element bodies, so this assumption-block straggler is technically a SPEC-0028 author miss the implementer faithfully inherited — but it must be cleaned up before SPEC-0028 ships, otherwise SPEC-0004 ships self-contradictory. Retry: either delete the Assumption bullet outright (cleanest, since the assumption is no longer load-bearing for anything in SPEC-0004), or amend it inline to note "mtime was retired in SPEC-0028; this assumption is preserved here as historical context only." If a scope question is preferred, surface back to the user before extending T-002's reach.
- Retry: SPEC-0004's `<assumptions>` block at lines 535-537 contradicts the amended REQ-002 and the new Changelog row by still claiming filesystem mtime is used for staleness detection. Delete the bullet (preferred) or rewrite it as historical context citing SPEC-0028, then re-run `speccy tasks SPEC-0004 --commit` to refresh the hash, then `speccy verify` to confirm clean.
- Implementer note (session-2026-05-17-spec0028-t002-retry):
  - Completed: deleted the obsolete `<assumptions>` bullet at `.speccy/specs/0004-status-command/SPEC.md:535-537` ("Filesystem mtime is reliable enough for staleness detection. CI environments that mass-touch files at checkout time may produce false-positive staleness; that's acceptable in v1."); extended the 2026-05-18 Changelog row to also record the assumption drop alongside the REQ-002 narrowing (single row, since both edits land in the same SPEC-0028 ship slice); re-recorded SPEC-0004's TASKS.md `spec_hash_at_generation` via `speccy tasks SPEC-0004 --commit` to reconcile the new legitimate hash drift.
  - Undone: (none)
  - Commands run: `.\target\debug\speccy.exe tasks SPEC-0004 --commit`; `.\target\debug\speccy.exe verify`; `grep -n "mtime|Modification time|MtimeDrift" .speccy/specs/0004-status-command/SPEC.md`.
  - Exit codes: 0; 0 (0 errors, 0 warnings, 49 info — fully clean); 0 (only the historical Changelog mention remains, which CHK-003 explicitly permits).
  - Discovered issues: (none).
  - Procedural compliance: the business reviewer flagged that SPEC-0028's REQ-003 scoped only to `<requirement>` / `<done-when>` / `<behavior>` / `<scenario>` element bodies and missed the `<assumptions>` block. The retry resolved it via SPEC-0004's own Changelog convention (no new SPEC-0028 amendment needed). If a future SPEC narrows a requirement of another SPEC, the amender should additionally grep `<assumptions>` for now-obsolete bullets — but that procedural rule isn't documented anywhere, and adding it is out of scope for SPEC-0028.
- Review (business, pass): retry resolved the prior blocker — `.speccy/specs/0004-status-command/SPEC.md:535-537` no longer claims filesystem mtime is used for staleness; the obsolete `<assumptions>` bullet was deleted and the 2026-05-18 Changelog row was extended to record both the REQ-002 narrowing and the assumption drop under a single SPEC-0028 attribution. REQ-002 / REQ-007 / Interfaces snippet edits from the first pass remain intact and matching the surviving code. ARCHITECTURE.md narrative is clean. CHK-003's three paragraphs all hold: (a) grep of `MtimeDrift|mtime drift|mtime-drift` against SPEC-0004 returns one match inside the `<changelog>` block (explicitly permitted as historical context); (b) the Changelog row names SPEC-0028 as the source and names the specific change; (c) ARCHITECTURE.md staleness narrative has zero `mtime`/`Modification time` substrings. `speccy verify` exits 0 with 0 warnings (49 info), confirming the hash re-record cleared the legitimate drift from the in-place amendment. Procedural-compliance note correctly surfaces the SPEC-0028 author miss without expanding scope. No new business concerns.
</task>

</tasks>
