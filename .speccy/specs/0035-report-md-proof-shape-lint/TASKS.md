---
spec: SPEC-0035
spec_hash_at_generation: dd7398768128f99c4ab33cf1a6ae066defc8056dceece5cf1f42e79c8349fc11
generated_at: 2026-05-21T03:11:16Z
---

# Tasks: SPEC-0035 RPT lint family — `speccy verify` gates on REPORT.md proof shape

<tasks spec="SPEC-0035">

<task id="T-001" state="completed" covers="REQ-001">

## Add `rpt` lint module, wire into the lint engine, and bless the registry snapshot

Create `speccy-core/src/lint/rules/rpt.rs` exporting
`pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>)`. The function
emits three codes against `spec.report_md`:

- `RPT-001` fires once when `spec.report_md` is `Some(Err(parse_error))`.
  Message renders the underlying `ParseError` via `Display`; `file` is
  `spec.dir.join("REPORT.md")`; `line` is `None`.
- `RPT-002` fires once per `<coverage req="REQ-NNN">` whose `req=` does
  not appear as a `<requirement id="...">` in
  `spec.spec_doc.requirements`. Short-circuit and emit nothing when
  `spec.spec_doc` itself failed to parse (SPC-001 owns that surface).
- `RPT-003` fires once per id in `<coverage scenarios="...">` that does
  not resolve under the row's already-resolved requirement. Skip a
  row entirely when its `req=` did not resolve (RPT-002 already fired
  for that row).

Wire-in:
- `speccy-core/src/lint/rules/mod.rs` declares `pub mod rpt;`.
- `speccy-core/src/lint/mod.rs::run` invokes
  `rules::rpt::lint(spec, &mut diagnostics);` inside the per-spec loop,
  after `rules::qst::lint(...)`.
- `speccy-core/src/lint/registry.rs::REGISTRY` appends
  `("RPT-001", Level::Error)`, `("RPT-002", Level::Error)`,
  `("RPT-003", Level::Error)`.
- `speccy-core/tests/snapshots/lint_registry.snap` gains
  `RPT-001\terror\n`, `RPT-002\terror\n`, `RPT-003\terror\n` in sorted
  position (above the SPC block per ASCII ordering of the codes).

Module-level `#[cfg(test)]` unit tests in `rpt.rs` cover:
- RPT-001 fires when `report_md = Some(Err(_))` (build a
  hand-constructed `ParsedSpec` with an injected parse failure).
- RPT-002 fires once per dangling `req=`, names the missing id, and
  does NOT fire when `spec.spec_doc` is `Err(_)`.
- RPT-003 fires once per dangling scenario id, names the requirement
  + scenario id, and does NOT fire when the row's `req=` itself was
  missing.
- Negative case: a well-formed REPORT.md whose coverage rows all
  resolve emits zero RPT diagnostics.

Open question resolution (matches REQ-001 done-when):
- SPEC's first `## Open Questions` item ("structured representation of
  the underlying parse failure"): keep RPT-001 message as the rendered
  `Display` string only, matching SPC-001 precedent. No new
  `parse_error_kind` field on the diagnostic envelope.
- SPEC's second `## Open Questions` item (`deferred` coverage rows):
  treat `result="deferred"` rows uniformly with `satisfied` and
  `partial` for resolution. No special-case branch.

<task-scenarios>
Given a hand-built `ParsedSpec` whose `report_md` field is
`Some(Err(parse_error))`,
when `rules::rpt::lint(spec, &mut diags)` runs,
then `diags` contains exactly one diagnostic with `code == "RPT-001"`,
`level == Level::Error`, `file == Some(spec.dir.join("REPORT.md"))`,
and `line == None`.

Given a hand-built `ParsedSpec` whose `spec_doc` parses with a single
`REQ-001` requirement and whose `report_md` parses with one
`<coverage req="REQ-999">` row,
when the rule runs,
then `diags` contains exactly one `RPT-002` naming `REQ-999`, and
zero `RPT-003` diagnostics (the row short-circuited).

Given a `ParsedSpec` whose `spec_doc` is `Err(_)` and whose `report_md`
parses with a `<coverage req="REQ-001">` row,
when the rule runs,
then zero `RPT-002` and zero `RPT-003` diagnostics are emitted (SPC-001
owns the SPEC.md surface).

Given a `ParsedSpec` whose `spec_doc` has `REQ-001` with one nested
`<scenario id="CHK-001">` and whose `report_md` has
`<coverage req="REQ-001" scenarios="CHK-001 CHK-999">`,
when the rule runs,
then `diags` contains exactly one `RPT-003` naming `REQ-001` and
`CHK-999`, and zero diagnostics for `CHK-001`.

Given the updated registry, when
`cargo test -p speccy-core --test lint_registry` runs, then the
snapshot test passes with the three new RPT lines.

Given the updated `lint/mod.rs`, when `speccy_core::lint::run` is
invoked against a workspace with one implemented spec whose
REPORT.md has a malformed root tag, then the returned diagnostics
include a `Diagnostic { code: "RPT-001", level: Level::Error, .. }`.

Suggested files:
- `speccy-core/src/lint/rules/rpt.rs` (new)
- `speccy-core/src/lint/rules/mod.rs`
- `speccy-core/src/lint/mod.rs`
- `speccy-core/src/lint/registry.rs`
- `speccy-core/tests/snapshots/lint_registry.snap`
</task-scenarios>

<implementer-note session="spec0035-t001">
- Completed: Created `speccy-core/src/lint/rules/rpt.rs` exporting
  `pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>)` with
  three RPT codes. Wired into `rules/mod.rs`, `lint/mod.rs`, and
  `registry.rs`. Updated `tests/snapshots/lint_registry.snap` with the
  three new sorted entries. All five unit tests pass covering RPT-001,
  RPT-002, RPT-003, the negative case, and the spec_doc-Err guard.
- Undone: T-002 (integration tests and ARCHITECTURE.md update) is left
  pending per scope constraint.
- Commands run: `cargo build -p speccy-core`, `cargo test -p speccy-core`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all`, `cargo +nightly fmt --all --check`,
  `cargo deny check`, `cargo test --workspace`
- Exit codes: all 0.
- Discovered issues: (1) The test module could not construct `SpecMd`
  directly — `SpecFrontmatter.created` is `jiff::civil::Date` and
  `SpecMd.sha256` is `[u8; 32]`, inaccessible without the real parser.
  Resolved by setting `spec_md: Err(...)` in fixtures; `rpt::lint`
  never reads `spec_md`. (2) `expect_used` is denied in production code,
  so the resolved_req guard uses `let Some(req) = resolved_req else {
  continue; }` after the `is_none()` check, which is logically
  unreachable but avoids the lint. (3) `implementer-note` requires a
  `session=` XML attribute — the first write used plain prose markup
  which the in-tree integration test caught.
- Procedural compliance: No skill file edits required. The
  `implementer-note` XML schema (requiring `session=` attribute) was
  not documented in the speccy-work skill body; discovered via test
  failure on `every_in_tree_tasks_md_parses_and_has_populated_scenarios`.
</implementer-note>

<review persona="business" verdict="pass">
T-001 delivers exactly what its scope promises: the `rpt` lint module,
wire-in to `rules/mod.rs` and `lint/mod.rs::run` (after `qst::lint` per
SPEC), three appended `REGISTRY` entries with `Level::Error`, and the
corresponding snapshot lines in sorted position. All six unit tests
covering the task-scenarios pass. RPT-001 fires on `Some(Err(_))` with
correct `code`/`level`/`file`/`line=None`; RPT-002 short-circuits when
`spec_doc` is `Err`; RPT-002 names the missing requirement id and
short-circuits RPT-003 for that row; RPT-003 names both requirement and
scenario, fires only for the dangling id; well-formed REPORT.md emits
zero diagnostics; REPORT.md absent short-circuits silently. Open
questions are explicitly resolved in-task as the SPEC recommended
(rendered `Display` string only for RPT-001; uniform treatment of
`deferred` rows). The ARCHITECTURE.md update and CLI-level integration
tests covering CHK-001/002/003 and in-progress demotion are correctly
deferred to T-002.
</review>

<review persona="security" verdict="pass">
No security issues. The `rpt` lint module operates entirely on typed
in-memory structs produced by the parser boundary; it performs no I/O,
no subprocess invocation, no authentication, and introduces no new
dependencies. The only string-to-output paths are `format!` calls that
embed structured identifiers (REQ-NNN, CHK-NNN) into diagnostic
messages — no shell, SQL, or template injection surface exists. The
single path construction (`spec.dir.join("REPORT.md")`) produces a
`Utf8PathBuf` for a diagnostic file field only; it is never used for a
filesystem read or write in this module. No credential, secret, or
sensitive data handling is present.
</review>

<review persona="tests" verdict="blocking">
Evidence loading failed: the `<implementer-note session="spec0035-t001">`
body has no `Evidence:` field. There is no per-task evidence file path
naming the red-then-green paper trail for the five RPT unit tests in
`speccy-core/src/lint/rules/rpt.rs`, so the tests cannot be verified to
have gone red before they went green — fabrication risk on a
test-suite-only slice is exactly where this gate exists. The
implementer-note enumerates "Completed", "Undone", "Commands run",
"Exit codes", "Discovered issues", and "Procedural compliance"
subsections, but the required `Evidence:` field naming a captured
runner-output file is absent. Test code itself reads cleanly (real
`lint()` invocation, no mocks, distinct positive/negative assertions),
so this block is exclusively about the missing evidence trail.
</review>

<review persona="style" verdict="blocking">
Two style issues. (1) `speccy-core/src/lint/rules/rpt.rs` lines 57-77:
the double-guard pattern is non-idiomatic and contains dead code. After
the `if resolved_req.is_none() { ... continue; }` block, the code then
does `let Some(req) = resolved_req else { continue; }`. The
`else { continue }` branch can never execute because the prior
`continue` already handles the `None` case. The accompanying comment
("`resolved_req` is `Some` here because the `continue` above
short-circuits on `None`") is documenting why the branch is dead —
signal that the pattern should be restructured. Idiomatic form:
collapse the `is_none()` guard and the `let-else` into a single
`let Some(req) = resolved_req else { push RPT-002; continue; };`.
(2) `speccy-core/src/lint/mod.rs` line 5 module doc lists the stable
code families emitted by the lint engine (`SPC-*`, `REQ-*`, `VAL-*`,
`TSK-*`, `QST-*`) but omits `RPT-*`. Since this task adds `RPT-*` to
the engine, the doc comment must include it.
</review>

<retry>
Two blockers from this review round. Address both before re-submitting.

- tests: add an `Evidence:` field to the `<implementer-note
  session="spec0035-t001">` body naming a per-task evidence file at
  `.speccy/specs/0035-report-md-proof-shape-lint/evidence/T-001.md`.
  The file must contain captured `cargo test -p speccy-core --lib
  lint::rules::rpt::tests --` output for two phases: a red phase
  (taken before `rpt.rs` was wired in or with one assertion deliberately
  broken, showing the named tests failing with stack frames) and a
  green phase (taken after the fix, showing the same test names
  passing). Workspace-wide `cargo test --workspace` does not satisfy
  this — the command must be scoped per-test. Follow the shape in
  `.speccy/examples/evidence.md` and the SPEC-0033 / SPEC-0032
  evidence files for precedent.
- style (a): collapse the double-guard in `speccy-core/src/lint/rules/rpt.rs`
  lines 57-77 into a single `let Some(req) = resolved_req else { ... };`
  expression that emits RPT-002 in the `else` block and then `continue`s.
  This eliminates the logically dead `else { continue }` branch and
  removes the explanatory comment.
- style (b): add `RPT-*` to the stable-codes list in the
  `speccy-core/src/lint/mod.rs` line 5 module doc comment so the
  documented set matches the implemented set.
</retry>

<implementer-note session="spec0035-t001-rev2">
- Completed: Addressed all three blockers from the rev1 review round.
  (1) Collapsed the double-guard in `speccy-core/src/lint/rules/rpt.rs`
  into a single `let Some(req) = ... else { push RPT-002; continue; };`
  expression, eliminating the logically dead `else { continue }` branch
  and its explanatory comment. (2) Added `RPT-*` to the stable-codes
  list in the `speccy-core/src/lint/mod.rs` module doc comment.
  (3) Created `evidence/T-001.md` with red-then-green captures of
  `cargo test -p speccy-core --lib lint::rules::rpt::tests --`.
- Undone: T-002 (integration tests and ARCHITECTURE.md update) remains
  pending per scope constraint.
- Commands run: `cargo test -p speccy-core --lib lint::rules::rpt::tests --`
  (red + green phases), `cargo clippy -p speccy-core --all-targets
  --all-features -- -D warnings`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`, `cargo deny check`
- Exit codes: all 0.
- Discovered issues: None new. The clippy `allow_attributes` lint was
  satisfied because no `#[allow]` was needed — the refactor eliminated
  the dead code without suppression.
- Evidence: `.speccy/specs/0035-report-md-proof-shape-lint/evidence/T-001.md`
- Procedural compliance: No skill file edits required. Red phase was
  produced by temporarily flipping the expected code in
  `rpt_001_fires_on_parse_failure` from `"RPT-001"` to `"RPT-999"`;
  reverted before the green capture.
</implementer-note>

<review persona="business" verdict="pass">
Round-2 confirms the round-1 business pass. T-001 still delivers exactly
what its scope promises and the rev2 changes do not expand or shift
scope. REQ-001 done-when items in T-001's slice are satisfied; non-goals
intact (no parser, scanner, ship-skill, or JSON-envelope changes); open
questions resolved per SPEC recommendations; ARCHITECTURE.md update and
CLI-level integration tests correctly deferred to T-002. The three rev2
fixes (evidence/T-001.md, collapsed let-else at rpt.rs:56, RPT-* in
lint/mod.rs line-5 doc) are surgical and behavior-preserving.
</review>

<review persona="tests" verdict="pass">
Evidence trail is intact. rev2 implementer-note carries an `Evidence:`
field pointing to `.speccy/specs/0035-report-md-proof-shape-lint/evidence/T-001.md`,
which exists and contains red-then-green captures of
`cargo test -p speccy-core --lib lint::rules::rpt::tests --`. Red phase
(exit 101) shows `rpt_001_fires_on_parse_failure ... FAILED` with a
real panic at `rpt.rs:218:9` and `left: "RPT-001"` / `right: "RPT-999"`,
consistent with the implementer's documented flip of the expected code
in that single `assert_eq!` line. Green phase (exit 0) shows all six
tests passing under the same scoped command. Runner output carries the
structural artifacts a real cargo test invocation would emit; red and
green halves are materially different. Command is scoped per-module
rather than workspace-wide. Round-1 blocker resolved.
</review>

<review persona="security" verdict="pass">
Rev2 changes introduce no new security exposure. The collapsed
`let-else` guard in `rpt.rs:56-68` has identical security surface as the
previous two-step form (format strings still embed typed identifiers
from parsed-XML fields into terminal diagnostic messages only); the
`RPT-*` addition to the `lint/mod.rs:5` module-doc comment is purely
textual; the evidence file contains only test names and assertion code
strings, no secrets. No I/O paths, auth boundaries, injection surfaces,
or dependency additions were introduced.
</review>

<review persona="style" verdict="pass">
Both round-1 blockers cleanly resolved. The double-guard in
`speccy-core/src/lint/rules/rpt.rs` is now a single
`let Some(req) = ...find(...) else { push RPT-002; continue; };`
expression (lines 56-68); the `is_none()` pre-check, dead `else
{ continue }` branch, and the explanatory comment are all gone. The
`speccy-core/src/lint/mod.rs` module doc (line 5) now lists `RPT-*`
alongside the other stable code families. No new style drift introduced
by the retry pass — no `#[allow]` suppressions, no duplicated helpers,
no production `unwrap()`/`expect()`, and import ordering is consistent.
</review>

</task>

<task id="T-002" state="completed" covers="REQ-001">

## Add `speccy verify` integration tests covering CHK-001 / CHK-002 / CHK-003 plus the in-progress demotion case

Add integration tests to `speccy-cli/tests/verify.rs` that drive the
real `speccy verify` entry point against tempdir workspaces. Reuse the
existing `common::Workspace` helper and `write_spec` builder.

Tests to add:
- `report_md_missing_spec_attribute_fires_rpt_001` — writes one
  `status: implemented` spec whose SPEC.md is well-formed and whose
  REPORT.md root element is `<report>` (no `spec="..."`). Asserts
  exit code 1, text output contains `RPT-001`, JSON envelope
  `lint_errors[]` contains a diagnostic with `code == "RPT-001"` and
  `file` ending in `/REPORT.md`. (CHK-001.)
- `report_md_dangling_req_fires_rpt_002` — one `status: implemented`
  spec whose SPEC.md declares `<requirement id="REQ-001">` and whose
  REPORT.md contains
  `<coverage req="REQ-999" result="satisfied" scenarios="CHK-001">`.
  Asserts exit 1, text contains `RPT-002` and `REQ-999`, and zero
  `RPT-003` diagnostic for `CHK-001`. (CHK-002.)
- `report_md_dangling_scenario_fires_rpt_003` — SPEC.md with
  `REQ-001` and nested `<scenario id="CHK-001">`; REPORT.md with
  `<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-999">`.
  Asserts exit 1, text contains `RPT-003` and `CHK-999`, no diagnostic
  for `CHK-001`. (CHK-003.)
- `report_md_rpt_demotes_on_in_progress_spec` — same malformed
  REPORT.md as CHK-001, but the SPEC.md frontmatter is
  `status: in-progress`. Asserts exit 0 (RPT-001 demoted to
  `Level::Info` by the existing `partition_lint` machinery; the
  diagnostic surfaces in the info bucket of `speccy verify --json`
  but does not gate). Covers the SPEC user-story for an in-flight
  amendment loop.

REPORT.md fixtures are inline `indoc!` strings written via the same
mechanism `write_spec` already supports (extend the helper or write
the REPORT.md alongside via `fs_err::write` once the spec dir
exists — match whatever pattern the test file already uses for
multi-file specs).

The exit-0 baseline for the workspace's in-tree specs (SPEC's
CHK-004) is left to ship-time verification — the in-tree REPORT.md
files are well-formed today, and SPEC-0035's own REPORT.md will be
written by `/speccy-ship`. No new test asserts this; the existing
`every_in_tree_report_md_parses_and_resolves_against_parent_spec`
integration test stays as the belt-and-braces check (per SPEC's
non-goal "No removal of the existing in-tree integration test").

Documentation update (`.speccy/ARCHITECTURE.md` "Lint Codes" section):
- Update the intro paragraph's prefix list from `SPC-/REQ-/TSK-` to
  include `RPT-` alongside the existing entries (CHK-006 — prefix
  list update).
- Add three new entries documenting `RPT-001`, `RPT-002`, `RPT-003`
  with their triggers and severities, matching the description style
  of the existing `SPC-*` / `TSK-*` blocks (CHK-006 — entry style).

Standard hygiene must pass at the end of the task:
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

<task-scenarios>
Given a tempdir workspace built by `common::Workspace::new()` with
one `status: implemented` spec whose REPORT.md root reads `<report>`
(no `spec=`),
when the new `report_md_missing_spec_attribute_fires_rpt_001` test
invokes `speccy_cli::verify::run` (or shells out via `assert_cmd`),
then it asserts exit 1, text output contains the substring `RPT-001`,
and `--json` envelope contains a `lint_errors[]` entry with
`code == "RPT-001"` and `file` ending in `/REPORT.md`.

Given a tempdir workspace whose REPORT.md has
`<coverage req="REQ-999" ...>` against a SPEC.md whose only
requirement is `REQ-001`,
when the new `report_md_dangling_req_fires_rpt_002` test runs,
then exit code is 1, text output contains `RPT-002` and `REQ-999`,
and the JSON envelope shows zero `RPT-003` diagnostics for the row's
scenarios.

Given a tempdir workspace whose REPORT.md has
`<coverage req="REQ-001" scenarios="CHK-001 CHK-999">` against a
SPEC.md whose `REQ-001` has only `<scenario id="CHK-001">`,
when the new `report_md_dangling_scenario_fires_rpt_003` test runs,
then exit code is 1, text output contains `RPT-003` and `CHK-999`,
and no diagnostic mentions `CHK-001`.

Given a tempdir workspace whose REPORT.md root reads `<report>` (no
`spec=`) but whose SPEC.md frontmatter is `status: in-progress`,
when the new `report_md_rpt_demotes_on_in_progress_spec` test runs,
then exit code is 0, the JSON envelope's `lint_info[]` contains a
diagnostic with `code == "RPT-001"` and `level == "info"`, and
`lint_errors[]` contains no RPT diagnostic.

Given `.speccy/ARCHITECTURE.md` after this task lands, when the
"Lint Codes" section is read, then it documents `RPT-001`, `RPT-002`,
`RPT-003` with the same shape as the existing `SPC-*` / `TSK-*`
entries, and the intro prefix list at line 1767 includes `RPT-`
alongside the other family prefixes (CHK-006).

Given all four standard-hygiene commands run from the project root
after the task lands, when each is invoked, then each exits 0
(workspace tests, workspace clippy with `-D warnings`, nightly
fmt --check, `cargo deny check`).

Suggested files:
- `speccy-cli/tests/verify.rs`
- `speccy-cli/tests/common.rs` (only if `write_spec` needs an extra
  parameter for the REPORT.md body; otherwise leave untouched)
- `.speccy/ARCHITECTURE.md`
</task-scenarios>

<implementer-note session="spec0035-t002">
- Completed: Added four integration tests to `speccy-cli/tests/verify.rs`
  covering CHK-001 / CHK-002 / CHK-003 and the in-progress demotion case.
  Tests are `report_md_missing_spec_attribute_fires_rpt_001`,
  `report_md_dangling_req_fires_rpt_002`,
  `report_md_dangling_scenario_fires_rpt_003`, and
  `report_md_rpt_demotes_on_in_progress_spec`. Each test writes a tempdir
  workspace via `common::Workspace::new()` + `write_spec`, then writes the
  REPORT.md fixture directly via `fs_err::write` into the returned spec dir.
  Updated `.speccy/ARCHITECTURE.md` "Lint Codes" section: removed the
  "in-flight" prose paragraph and added three RPT-001 / RPT-002 / RPT-003
  entries to the code block; the intro paragraph already listed `RPT-` so no
  change was needed there.
- Undone: None. `common.rs` was not modified (REPORT.md is written directly
  via `fs_err::write` after `write_spec` returns the spec dir).
- Commands run:
  `cargo test -p speccy-cli --test verify report_md_` (red + green phases),
  `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`,
  `cargo deny check`
- Exit codes: all 0 (after fixing two clippy issues: `r#"..."#` → `r"..."` for
  a raw string without hashes, and `rpt_003_diags[0]` → `.first().expect(...)`
  to satisfy `indexing_slicing`).
- Discovered issues: (1) `report_md_missing_spec_attr` initially used
  `indoc! {r#"..."#}` which triggered `needless_raw_string_hashes` clippy
  lint — simplified to `indoc! {r"..."}`. (2) Direct index `rpt_003_diags[0]`
  triggered `indexing_slicing` — replaced with `.first().expect(...)`.
  (3) The ARCHITECTURE.md intro paragraph already included `RPT-` in its
  prefix list (it said "in flight"), so only the "in-flight" paragraph and
  the three RPT entries in the code block required editing.
- Evidence: `.speccy/specs/0035-report-md-proof-shape-lint/evidence/T-002.md`
- Procedural compliance: No skill file edits required. The `fs_err::write`
  pattern for writing REPORT.md alongside a spec dir (without modifying
  `write_spec`) is a clean extension of the existing test pattern.
</implementer-note>

<review persona="business" verdict="pass">
T-002 delivers exactly what the SPEC requires of its slice. The four new
integration tests in `speccy-cli/tests/verify.rs`
(`report_md_missing_spec_attribute_fires_rpt_001`,
`report_md_dangling_req_fires_rpt_002`,
`report_md_dangling_scenario_fires_rpt_003`,
`report_md_rpt_demotes_on_in_progress_spec`) each map one-to-one to
REQ-001's `<done-when>` bullets for CHK-001/CHK-002/CHK-003 and the
in-progress demotion case. Assertions check the load-bearing facts: exit
code, lint code substring in text output, diagnostic code in the JSON
`lint.errors[]` (or `lint.info[]` for demotion), named requirement and
scenario ids, and the negative invariant that RPT-003 does not fire
when RPT-002 fired for the same row. ARCHITECTURE.md "Lint Codes" now
lists RPT-001/002/003 with triggers and Level::Error matching the
SPC-*/TSK-* description style; the section intro prefix list includes
`RPT-`. Non-goals intact (no ship-skill change, no cargo-test inside
any speccy command, no parser or scanner edit, no in-tree integration
test removed, exactly three new lint codes, no envelope schema bump).
CHK-004 in-tree exit-0 baseline correctly deferred to ship-time per
task body rationale; live `speccy verify` against the in-tree workspace
returns 0 errors today. Minor non-blocking observation: the Threat
Model section at .speccy/ARCHITECTURE.md still carries an "(in flight)"
prefix on the REPORT.md proof-shape bullet, which is now stale but
out-of-scope for T-002.
</review>

<review persona="tests" verdict="pass">
Evidence trail intact and the four new integration tests genuinely
exercise the RPT-* gate. `.speccy/specs/0035-report-md-proof-shape-lint/evidence/T-002.md`
exists and is named by the `Evidence:` field on
`<implementer-note session="spec0035-t002">`. Red phase (exit 101)
shows `report_md_missing_spec_attribute_fires_rpt_001 ... FAILED` with
a real panic at `speccy-cli\tests\verify.rs:777:5` and
`left: 1 / right: 0`, consistent with the implementer's documented
flip of the gating-exit assertion in that one test; the other three
tests stay green during the red capture (single-assertion flip shape).
Green phase (exit 0) shows all four tests passing. Output carries the
cargo runner's structural artifacts (Compiling/Finished/Running lines,
test-name lines, summary with timing and filtered-out count) and the
two halves are materially different. Command is scoped per-test
(`cargo test -p speccy-cli --test verify report_md_`), not
workspace-wide. All four test names in the evidence appear in the diff
at `speccy-cli/tests/verify.rs`. Tests drive the real
`speccy_cli::verify::run` via the `invoke()` helper and write
hand-built REPORT.md strings via `fs_err::write` — no mocks.
Assertions are specific (named codes, ids, exact exit codes,
bucket-specific JSON paths) rather than catch-any-error or
assertion-by-snapshot.
</review>

<review persona="security" verdict="pass">
T-002 introduces no new security exposure. The four RPT integration
tests write hardcoded `indoc!` string literals to `tempfile::TempDir`-
allocated paths via `fs_err::write`; the temp-dir API uses the OS's
secure randomised allocation, `Utf8PathBuf::from_path_buf` enforces
UTF-8 validity, and `spec_dir.join("REPORT.md")` appends a fixed
literal with no separator characters. No user-supplied data flows into
any fixture string. Tests invoke `speccy_cli::verify::run` in-process
or via `assert_cmd` with an explicit `current_dir`; no shell
interpolation of fixture content occurs. Diagnostic messages embed
only identifiers (`REQ-NNN`, `CHK-NNN`) that have already passed the
parser's `r"^REQ-\d{3,}$"` / `r"^CHK-\d{3,}$"` regex gates before any
`format!` call. No new crate dependencies added. The ARCHITECTURE.md
update is plain prose with no executable surface. The `repo_sha`
field is a git commit hash, not a credential.
</review>

<review persona="style" verdict="pass">
All four RPT-* integration tests and their fixture helpers match the
existing patterns in `speccy-cli/tests/verify.rs`. The
`#![allow(clippy::expect_used, reason = "...")]` crate attribute is
identical to the convention used in every other test file in the
project (`check.rs`, `lock.rs`, `next_json.rs`, etc.) and passes the
workspace `allow_attributes = "deny"` lint because `clippy.toml`'s
`allow-expect-in-tests = true` makes the suppressed lint a no-op in
test binaries. The three inline fixture helpers follow the same
`indoc!` + `fn returning String` naming pattern as the existing
`spec_md_empty_scenarios` and `spec_md_missing_id` helpers. REPORT.md
is written via `fs_err::write` after `write_spec` returns the spec dir
— matching the multi-file spec pattern in `init.rs` and `lock.rs`
rather than inventing a new helper. The ARCHITECTURE.md RPT-001/002/003
entries match the indentation and description style of the SPC-*/TSK-*
blocks. No `unwrap()`, no bare `[i]` index access, no `#[allow]`
without `reason`, no dead code, no duplicated helpers. All four
hygiene commands exit 0.
</review>

</task>

</tasks>
