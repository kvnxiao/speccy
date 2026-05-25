---
spec: SPEC-0035
spec_hash_at_generation: 2668482db9d6182e7693753a13ebf401ffc44b00c856ca2fd6e8d3a7e60f0e70
generated_at: 2026-05-23T07:36:28Z
---

# Tasks: SPEC-0035 RPT lint family — `speccy verify` gates on REPORT.md proof shape


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

Documentation update (`docs/ARCHITECTURE.md` "Lint Codes" section):
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

Given `docs/ARCHITECTURE.md` after this task lands, when the
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
- `docs/ARCHITECTURE.md`
</task-scenarios>
</task>

