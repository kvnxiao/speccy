---
id: SPEC-0035
slug: report-md-proof-shape-lint
title: RPT lint family — speccy verify gates on REPORT.md proof shape so ship validation matches CI
status: in-progress
created: 2026-05-20
supersedes: []
---

# SPEC-0035: RPT lint family — `speccy verify` gates on REPORT.md proof shape so ship validation matches CI

## Summary

`speccy verify` is documented as "proof-shape lint that exits non-zero
on broken structure" (AGENTS.md core principle 3; `.speccy/ARCHITECTURE.md`
"Lint Codes" section starting at line 1763). Today its lint families
cover SPEC.md structure (`SPC-*`), requirement-to-scenario shape
(`REQ-*`), task-list shape (`TSK-*`), open questions (`QST-*`), and
JSON envelope versioning (`JSON-*`). There is no `RPT-*` family. The
workspace scanner already parses REPORT.md into
`Option<ParseResult<ReportDoc>>` and exposes it on
`speccy_core::lint::types::ParsedSpec::report_md`
(`speccy-core/src/lint/types.rs:165`), but no lint rule consumes that
field, so REPORT.md proof-shape failures are invisible to
`speccy verify`.

SPEC-0033's ship surfaced the gap. The shipped REPORT.md at
`.speccy/specs/0033-eject-prompt-bodies/REPORT.md` carried a
root element `<report>` with no `spec="..."` attribute. The required
attribute is enforced by `speccy_core::parse::report_xml::parse`
(`speccy-core/src/parse/report_xml/mod.rs:55` — `spec_id` is a
required field on `ReportDoc`), so the in-tree integration test
`every_in_tree_report_md_parses_and_resolves_against_parent_spec`
(`speccy-core/tests/in_tree_tasks_reports.rs:104`) caught the
malformed element and failed under `cargo test --workspace`. The
ship skill's CI dry-run step (`speccy-ship.md` step 4 — "Run
`speccy verify`") passed silently because `lint::run`
(`speccy-core/src/lint/mod.rs:32`) never touches `report_md`. Main-
branch CI then went red for three commits (`e48f729`, `4ad0f0b`,
`0967029`) before the malformed root tag was fixed in `cd08622`.

The fix is to add an `RPT-*` lint family that consumes
`ParsedSpec::report_md` and surfaces three failure modes as
`Level::Error` diagnostics: REPORT.md parse failure (mirror of
SPC-002's "SPEC.md marker tree malformed" precedent),
`<coverage req="REQ-NNN">` references a requirement that does not
exist in the sibling SPEC.md, and a scenario id in
`<coverage scenarios="...">` does not resolve to a `<scenario id="...">`
nested under the named requirement. The integration test in
`in_tree_tasks_reports.rs` is the encoded shape; this SPEC moves
that same logic from `cargo test` into the lint engine so
`speccy verify` is equivalent within speccy's scope.

The constraint is explicit: the ship skill is **not** edited to run
`cargo test --workspace` or any project-test invocation. Speccy is
consumed outside its own source tree, where there is no Rust
workspace to test; the equivalence between ship validation and CI
must come from inside `speccy verify` itself, not from layering
project-specific test commands on top.

## Goals

<goals>
- A new lint rules module `speccy-core/src/lint/rules/rpt.rs` ships
  with three rules. `RPT-001` fires when `ParsedSpec.report_md` is
  `Some(Err(_))` (REPORT.md present but failed to parse — covers
  the missing `spec="..."` root attribute, malformed `<coverage>`
  element shape, fenced-code-block boundary issues, and every
  other failure mode `parse_report_xml` already returns). `RPT-002`
  fires when a `<coverage req="REQ-NNN">` element in a parsed
  REPORT.md references a requirement id that has no matching
  `<requirement id="REQ-NNN">` in the sibling SPEC.md (i.e.
  `ParsedSpec.spec_doc.requirements` has no entry with that id).
  `RPT-003` fires when a scenario id in `<coverage scenarios="...">`
  does not match any `<scenario id="...">` nested under the
  resolved requirement (resolution chain: cov.req → spec_doc
  requirement → requirement.scenarios).
- All three RPT codes are appended to
  `speccy-core/src/lint/registry.rs::REGISTRY` with
  `Level::Error`. The lint registry snapshot test
  (`speccy-core/tests/lint_registry.rs`) is updated to include the
  three new entries.
- `speccy-core/src/lint/mod.rs::run` invokes
  `rules::rpt::lint(spec, &mut diagnostics);` in the per-spec loop
  alongside the existing `spc`, `req`, `tsk`, `qst` calls.
- `speccy-core/src/lint/rules/mod.rs` exports the new module
  (`pub mod rpt;`).
- The architecture doc's "Lint Codes" section
  (`.speccy/ARCHITECTURE.md` lines 1763-1800) gains three new
  documented entries `RPT-001`, `RPT-002`, `RPT-003` describing
  each code's trigger and severity. The prefix list in the same
  section's intro paragraph (line 1767) is updated to include
  `RPT-` alongside `SPC-`, `REQ-`, `TSK-`.
- The shipped behavior of `speccy verify` is: when run against a
  workspace that contains an `implemented` SPEC whose REPORT.md
  is malformed or whose coverage rows reference missing
  requirements or scenarios, `speccy verify` exits 1 with the
  RPT diagnostics in its text and JSON output. Combined with the
  ship skill's existing step 4 (`speccy verify` after the status
  flip), this gates ship on the same proof-shape failures the
  in-tree integration test catches today.
- The `in-progress`/`dropped`/`superseded` demotion pass in
  `speccy_cli::verify::partition_lint`
  (`speccy-cli/src/verify.rs:165`) continues to demote RPT
  diagnostics on non-`implemented` specs to `Level::Info`,
  matching SPEC-0018's pre-existing rule for `SPC-*` and `TSK-*`.
  No new demotion logic is added; RPT participates by the existing
  spec-status-keyed mechanism.
</goals>

## Non-goals

<non-goals>
- No change to the ship skill (`resources/modules/phases/speccy-ship.md`)
  or its ejected dogfood copies. The skill's existing step 4
  ("Run `speccy verify`") becomes the right gate by virtue of
  `speccy verify` now surfacing RPT diagnostics; no additional
  command or step is added to the skill body. The equivalence
  between ship validation and CI lives in the CLI, not in the
  skill prompt.
- No introduction of `cargo test --workspace` or any
  project-specific test invocation into the ship skill, into
  `speccy verify`, or into any other speccy command. Speccy is
  consumed outside its own source tree; portability rules out
  embedding project-test commands in any shipped recipe.
- No change to `parse_report_xml`
  (`speccy-core/src/parse/report_xml/mod.rs`) or to the
  `ReportDoc` data shape. The parser already enforces the required
  `spec="..."` attribute, the closed `<coverage result="...">`
  value set, and the line-isolated raw-XML element constraints
  shared with SPEC.md / TASKS.md parsing. RPT-001 surfaces the
  existing parse errors as lint diagnostics; it does not extend
  the parser's validation rules.
- No change to the workspace scanner's REPORT.md ingest path
  (`speccy-core/src/workspace.rs::parse_one_report_xml` —
  surrounding code near line 575). The scanner already produces
  `ParsedSpec.report_md: Option<ParseResult<ReportDoc>>`; this
  SPEC consumes that field, it does not modify the producer.
- No removal of the existing in-tree integration test
  `every_in_tree_report_md_parses_and_resolves_against_parent_spec`
  (`speccy-core/tests/in_tree_tasks_reports.rs:104`). The test
  continues to exist as a belt-and-braces check; the goal is to
  make `speccy verify` catch the same class of failure earlier in
  the loop, not to delete the existing safety net.
- No new lint codes beyond the three named here. Refinements
  (e.g. RPT-004 to flag a missing REPORT.md when `status:
  implemented` — currently the responsibility of the spec
  authoring loop, not the lint engine) are deferred. If a future
  loop surfaces a need, it goes to a follow-up SPEC.
- No restructure of the lint engine's invocation order, the
  diagnostic sort key, or the demotion rules. RPT slots in after
  QST in the per-spec loop and participates in the existing
  sort-by-(spec_id, code, file, line) and
  status-keyed-demotion machinery unchanged.
- No change to `speccy verify`'s JSON envelope schema or its
  `schema_version`. RPT diagnostics serialise through the existing
  `Diagnostic` shape (code, level, message, spec_id, file, line);
  no envelope key gains a new sub-structure.
- No edit to the speccy-ship/speccy-tasks/speccy-work skill bodies
  or their agent-file counterparts. The gap is in the CLI; the
  skills already point to `speccy verify` for validation.
- No `RPT-NNN` code reused from a previous lint family or remapped
  from a retired one. All three codes are net-new entries in the
  registry.
- No micro-optimisation pass on the new lint rule's traversal
  cost. REPORT.md coverage rows are bounded by SPEC.md requirement
  count (typically <20 per spec), and `<coverage>` scenario lists
  are bounded similarly; the naive linear scan per resolution
  step is adequate.
</non-goals>

## User Stories

<user-stories>
- As a Speccy contributor running `/speccy-ship` after every task
  in a spec has been flipped to `state="completed"`, I want the
  skill's step 4 (`speccy verify`) to fail the ship attempt when
  my REPORT.md is malformed or contains coverage rows that don't
  resolve against the sibling SPEC.md. Today the step passes
  silently on a malformed REPORT.md, and the failure only surfaces
  hours later when CI runs `cargo test --workspace` on the
  pushed PR — by then a fix-up commit is the cheapest path
  forward.
- As a maintainer reading `speccy verify` output, I want a
  parseable lint code (`RPT-001`, `RPT-002`, or `RPT-003`) on every
  REPORT.md proof-shape failure, so I can grep CI logs the same
  way I grep for `SPC-*` and `TSK-*` issues today. The integration
  test's panic message is unstructured; a lint code is.
- As a downstream Speccy consumer running speccy on a non-Rust
  project (JavaScript / Python / Go), I want `speccy verify` to
  catch the same class of REPORT.md shape failures the in-tree
  Rust integration tests catch in this repo. There is no
  `cargo test` to run on a TypeScript codebase; verify is the
  only validation surface speccy itself provides, and it must
  carry its weight.
- As a contributor adding a new lint code in the future, I want
  the RPT family to live as a sibling of SPC/REQ/TSK/QST under
  `speccy-core/src/lint/rules/`, follow the same `lint(spec, out)`
  signature, and appear in `lint::run`'s per-spec loop. The shape
  is predictable; the new family is not an exception.
- As a Speccy contributor amending a SPEC mid-loop (per
  `/speccy-amend`), I do not want RPT diagnostics on the in-flight
  spec to gate further work. The existing `partition_lint`
  demotion pass demotes `Level::Error` diagnostics on
  `in-progress` specs to `Level::Info`, so RPT errors on a SPEC
  whose REPORT.md is still being drafted surface as info-level
  and do not gate the exit code.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `RPT-*` lint family lives at `speccy-core/src/lint/rules/rpt.rs`, ships three codes, and wires into `speccy verify`

A new module `speccy-core/src/lint/rules/rpt.rs` exports a single
public function with the signature
`pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>)` (no
`Workspace` parameter — RPT rules need only the per-spec view).
The module emits three lint codes:

- `RPT-001` fires once per spec when `spec.report_md` is
  `Some(Err(parse_error))`. The diagnostic message includes the
  underlying `ParseError` rendered via its `Display` impl so the
  failure mode (missing `spec="..."` attribute, malformed
  `<coverage>`, line-isolation violation, etc.) is visible in the
  text output. The `file` field points to the spec directory's
  `REPORT.md` (computed as `spec.dir.join("REPORT.md")`); the
  `line` field is `None` because `ParseError` already carries
  location detail in its `Display` form.
- `RPT-002` fires once per dangling `<coverage req="REQ-NNN">`
  reference in a successfully-parsed REPORT.md. A reference is
  dangling when `spec.spec_doc` parsed successfully but no
  `Requirement` in `spec_doc.requirements` has matching `id`. If
  `spec.spec_doc` itself failed to parse, `RPT-002` does NOT
  fire (the underlying problem is the SPEC.md, which `SPC-001`
  already surfaces). One diagnostic per missing requirement;
  duplicate references in the same REPORT.md surface once each.
- `RPT-003` fires once per dangling scenario id under a
  successfully-resolved `<coverage>` row. Resolution chain: the
  `<coverage req="REQ-NNN">` first resolves via RPT-002's logic;
  if that succeeded, each id in `scenarios=` is checked against
  the resolved requirement's `scenarios.iter().map(|s| &s.id)`.
  A missing scenario id fires one diagnostic naming the
  requirement and the unresolved scenario id. If the requirement
  itself was missing (RPT-002 fired), RPT-003 does NOT fire for
  any of that row's scenarios (the row is already broken; one
  diagnostic per row, not N).

The registry at `speccy-core/src/lint/registry.rs::REGISTRY` gains
three new tuples `("RPT-001", Level::Error)`,
`("RPT-002", Level::Error)`, `("RPT-003", Level::Error)`,
appended after the existing entries. The lint loop at
`speccy-core/src/lint/mod.rs::run` invokes
`rules::rpt::lint(spec, &mut diagnostics);` after the existing
`rules::qst::lint(spec, &mut diagnostics);` call. The rules
sub-module index at `speccy-core/src/lint/rules/mod.rs` declares
`pub mod rpt;`. No other production-code file changes.

The architecture doc's "Lint Codes" section
(`.speccy/ARCHITECTURE.md` starting line 1763) gains a new
`RPT-*` block listing the three codes with their triggers, and
the intro paragraph (currently line 1767) updates the prefix
list from `SPC-/REQ-/TSK-` to `SPC-/REQ-/TSK-/RPT-` (QST and
JSON are already listed implicitly; this SPEC matches the
existing intro convention).

The lint registry snapshot test
(`speccy-core/tests/lint_registry.rs`) is updated so its
expected snapshot text includes the three new
`<code>\t<severity>\n` lines in sorted position. The snapshot
test continues to enforce that adding or removing a code is a
visible change.

<done-when>
- `speccy-core/src/lint/rules/rpt.rs` exists and exports
  `pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>)`.
- `speccy-core/src/lint/rules/mod.rs` declares `pub mod rpt;`.
- `speccy-core/src/lint/mod.rs::run` invokes
  `rules::rpt::lint(spec, &mut diagnostics);` inside the per-spec
  loop, after `rules::qst::lint`.
- `speccy-core/src/lint/registry.rs::REGISTRY` contains exactly
  three new entries: `("RPT-001", Level::Error)`,
  `("RPT-002", Level::Error)`, `("RPT-003", Level::Error)`.
- The lint registry snapshot test
  (`speccy-core/tests/lint_registry.rs`) passes against an
  updated expected snapshot that includes the three RPT lines in
  sorted order.
- Running `speccy verify` against a workspace whose
  `status: implemented` SPEC has a malformed REPORT.md (e.g.
  `<report>` with no `spec="..."` attribute) exits 1 with text
  output containing the substring `RPT-001` and a JSON envelope
  whose `lint_errors[].code` includes `"RPT-001"`.
- Running `speccy verify` against a workspace whose
  `status: implemented` SPEC has a REPORT.md with a
  `<coverage req="REQ-999">` that does not resolve in the sibling
  SPEC.md exits 1 with text output containing `RPT-002` and
  naming `REQ-999`.
- Running `speccy verify` against a workspace whose
  `status: implemented` SPEC has a REPORT.md with a
  `<coverage req="REQ-001" scenarios="CHK-999">` whose REQ-001
  exists in SPEC.md but whose scenarios list contains no
  `CHK-999` element id exits 1 with text output containing
  `RPT-003` and naming `CHK-999`.
- Running `speccy verify` against the workspace's current
  in-tree specs (after this SPEC's TASKS lifecycle completes
  and the dogfooded SPEC-0035 has its own REPORT.md) exits 0.
- The architecture doc's "Lint Codes" section documents
  `RPT-001`, `RPT-002`, and `RPT-003` with their triggers,
  matching the on-disk severity in the registry.
- The four existing standard-hygiene commands in AGENTS.md pass:
  `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`, `cargo deny check`.
</done-when>

<behavior>
- Given a freshly built `speccy` binary in a workspace where
  one `status: implemented` SPEC has a malformed REPORT.md whose
  root element reads `<report>` (no `spec="..."` attribute),
  when `speccy verify` runs, then the process exits 1, the text
  output names `RPT-001` and the REPORT.md path, and the JSON
  envelope's `lint_errors[]` includes a diagnostic with
  `code == "RPT-001"` and `file` pointing at the REPORT.md.
- Given the same binary, when run against a workspace where one
  `status: implemented` SPEC's REPORT.md contains
  `<coverage req="REQ-999" result="satisfied" scenarios="CHK-001">`
  and that spec's SPEC.md has no `<requirement id="REQ-999">`
  element, then the process exits 1 and the output names
  `RPT-002` plus `REQ-999`. No `RPT-003` diagnostic is emitted
  for `CHK-001` (the resolution chain short-circuited at the
  missing requirement).
- Given the same binary, when run against a workspace where one
  `status: implemented` SPEC's REPORT.md contains
  `<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-999">`
  and SPEC.md's REQ-001 has `<scenario id="CHK-001">` but no
  `<scenario id="CHK-999">`, then the process exits 1 and the
  output names `RPT-003` plus `CHK-999`. No diagnostic fires for
  `CHK-001`.
- Given the same binary, when run against a workspace where the
  spec with the malformed REPORT.md has frontmatter
  `status: in-progress` instead of `implemented`, then the RPT
  diagnostic is demoted to `Level::Info` by the existing
  `partition_lint` demotion pass and the process exits 0 (the
  diagnostic surfaces in the info bucket but does not gate).
- Given a workspace where REPORT.md is absent on every spec
  (the typical mid-loop state before ship runs), when
  `speccy verify` runs, then no `RPT-*` diagnostic fires
  anywhere (`ParsedSpec.report_md` is `None` and the rule
  short-circuits).
- Given a workspace where one spec has SPEC.md that fails to
  parse (SPC-001 fires) and a REPORT.md with a
  `<coverage req="REQ-001">` row, when `speccy verify` runs,
  then `RPT-002` does NOT fire for that REPORT.md (the
  underlying problem is the unparseable SPEC.md; reporting a
  dangling REQ reference on top of that is noise).
</behavior>

<scenario id="CHK-001">
Given a tempdir workspace containing one `status: implemented` SPEC
whose REPORT.md root element is `<report>` with no `spec="..."`
attribute,
when `speccy verify` runs with cwd at the workspace root,
then the process exits 1, the text output contains the substring
`RPT-001`, and `speccy verify --json` emits an envelope whose
`lint_errors[]` array contains a `Diagnostic` with
`code == "RPT-001"` and `file` ending in `/REPORT.md`.
</scenario>

<scenario id="CHK-002">
Given a tempdir workspace containing one `status: implemented` SPEC
whose SPEC.md declares `<requirement id="REQ-001">` (and no
`REQ-999`) and whose REPORT.md contains
`<coverage req="REQ-999" result="satisfied" scenarios="CHK-001">`,
when `speccy verify` runs,
then the process exits 1, text output contains `RPT-002` and
`REQ-999`, and no `RPT-003` diagnostic fires for `CHK-001`.
</scenario>

<scenario id="CHK-003">
Given a tempdir workspace containing one `status: implemented` SPEC
whose SPEC.md declares `<requirement id="REQ-001">` with a single
nested `<scenario id="CHK-001">`, and whose REPORT.md contains
`<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-999">`,
when `speccy verify` runs,
then the process exits 1, text output names `RPT-003` and
`CHK-999`, and no diagnostic fires for `CHK-001`.
</scenario>

<scenario id="CHK-004">
Given the workspace at this repository (`.speccy/specs/` containing
all 34 currently-implemented specs plus this SPEC's own in-progress
entry),
when `speccy verify` runs after this SPEC's TASKS lifecycle
completes and SPEC-0035's own REPORT.md is written and well-formed,
then the process exits 0 with zero `RPT-*` diagnostics at
`Level::Error`.
</scenario>

<scenario id="CHK-005">
Given the lint registry snapshot at
`speccy-core/tests/lint_registry.rs`'s expected text,
when this SPEC's implementation lands,
then the snapshot lines for `RPT-001`, `RPT-002`, and `RPT-003`
each appear exactly once in sorted position with severity
`Error`.
</scenario>

<scenario id="CHK-006">
Given `.speccy/ARCHITECTURE.md` after this SPEC's implementation
lands,
when the "Lint Codes" section is read,
then it documents `RPT-001`, `RPT-002`, and `RPT-003` with the
same description style as the existing `SPC-*` / `TSK-*` entries,
and the section's intro prefix list includes `RPT-` alongside the
other family prefixes.
</scenario>

</requirement>

## Open Questions

- [ ] Should `RPT-001` carry a structured representation of the
      underlying parse failure (e.g., a `parse_error_kind` field on
      the diagnostic) so the JSON envelope is machine-parseable for
      classifying failures, or is the rendered `Display` string in
      the message field enough? Recommendation: rendered string only,
      matching the existing `SPC-001` precedent. Promote if a
      downstream tool surfaces a need.
- [ ] Should `RPT-002` and `RPT-003` fire on a `result="deferred"`
      coverage row even when the row's requirement or scenarios
      don't resolve? Today the parser already allows `deferred`
      with an empty `scenarios=` attribute (per
      `report_xml/mod.rs:36`); the in-tree test treats deferred
      rows the same as satisfied/partial for resolution purposes.
      Recommendation: match the in-tree test (uniform treatment,
      no special case for `deferred`). Promote if review feedback
      surfaces a reason to special-case.

## Changelog

<changelog>
| Date       | Reason                                       | Author |
|------------|----------------------------------------------|--------|
| 2026-05-20 | Initial draft. Add an `RPT-*` lint family to `speccy verify` so REPORT.md proof-shape failures (parse, dangling coverage→requirement, dangling scenario id) gate the ship skill's existing `speccy verify` step. Motivated by SPEC-0033's malformed `<report>` root tag landing on main-branch CI for three commits before the in-tree integration test caught it under `cargo test`. Constraint: no `cargo test` invocation enters the ship skill — speccy is consumed outside its own source tree and the equivalence must live in the CLI. | Kevin Xiao |
</changelog>
