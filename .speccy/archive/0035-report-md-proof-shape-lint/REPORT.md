---
spec: SPEC-0035
outcome: satisfied
generated_at: 2026-05-20T00:00:00Z
---

# REPORT: SPEC-0035 RPT lint family — `speccy verify` gates on REPORT.md proof shape

`speccy verify` now surfaces three new `RPT-*` lint codes — `RPT-001`,
`RPT-002`, and `RPT-003` — that consume `ParsedSpec.report_md` and gate
the ship skill's existing `speccy verify` step on REPORT.md proof-shape
failures. The lint family lives at
`speccy-core/src/lint/rules/rpt.rs`, wires into `lint::run` after
`rules::qst::lint`, appends three `Level::Error` entries to the
registry, and participates in the existing status-keyed demotion pass
(diagnostics on non-`implemented` specs demote to `Level::Info`). No
ship-skill, parser, workspace-scanner, or JSON-envelope change was
needed. The four integration tests added to `speccy-cli/tests/verify.rs`
drive the real `speccy verify` entry point against tempdir workspaces
and cover CHK-001 through CHK-003 plus the in-progress demotion case.
ARCHITECTURE.md's "Lint Codes" section now documents all three RPT
codes with the same description style as the existing SPC-*/TSK-*
blocks; the in-tree `speccy verify` baseline exits 0 with zero
`RPT-*` errors.

<report spec="SPEC-0035">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003 CHK-004 CHK-005 CHK-006">
The `rpt` lint module ships at `speccy-core/src/lint/rules/rpt.rs`
exporting `pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>)`.
`RPT-001` fires when `spec.report_md` is `Some(Err(parse_error))`; the
message renders the underlying `ParseError` via `Display`; `file` is
`spec.dir.join("REPORT.md")`; `line` is `None`. `RPT-002` fires once
per dangling `<coverage req="REQ-NNN">` row whose requirement id has no
matching entry in `spec_doc.requirements`; it short-circuits when
`spec.spec_doc` itself failed to parse. `RPT-003` fires once per
unresolved scenario id under an already-resolved requirement row; it
short-circuits when that row's `req=` itself was missing. All three
codes appear in `REGISTRY` as `("RPT-*", Level::Error)`; `lint::run`
invokes `rules::rpt::lint(spec, &mut diagnostics)` after `rules::qst`;
`rules/mod.rs` declares `pub mod rpt;`.

CHK-001 is satisfied by the integration test
`report_md_missing_spec_attribute_fires_rpt_001` in
`speccy-cli/tests/verify.rs`: a tempdir workspace with one
`status: implemented` spec whose REPORT.md root element is `<report>`
(no `spec="..."` attribute) causes `speccy verify` to exit 1, text
output contains `RPT-001`, and the JSON `lint_errors[]` array contains
a diagnostic with `code == "RPT-001"` and `file` ending in `/REPORT.md`.

CHK-002 is satisfied by `report_md_dangling_req_fires_rpt_002`: a
SPEC.md declaring only `REQ-001` paired with a REPORT.md containing
`<coverage req="REQ-999">` causes exit 1 with `RPT-002` and `REQ-999`
in the output, and zero `RPT-003` diagnostics for the row's scenarios.

CHK-003 is satisfied by `report_md_dangling_scenario_fires_rpt_003`: a
SPEC.md with `REQ-001` containing only `<scenario id="CHK-001">` paired
with `<coverage req="REQ-001" scenarios="CHK-001 CHK-999">` causes exit
1 with `RPT-003` and `CHK-999` in the output, and no diagnostic for
`CHK-001`.

CHK-004 is satisfied by this REPORT.md's own presence: after
SPEC-0035's TASKS lifecycle completed and this well-formed REPORT.md
was written, `speccy verify` exits 0 against the in-tree workspace with
zero `RPT-*` diagnostics at `Level::Error`.

CHK-005 is satisfied by the updated registry snapshot at
`speccy-core/tests/snapshots/lint_registry.snap`: lines for `RPT-001`,
`RPT-002`, and `RPT-003` each appear exactly once in sorted position
with severity `Error`, and `cargo test -p speccy-core --test
lint_registry` passes.

CHK-006 is satisfied by the ARCHITECTURE.md update: the "Lint Codes"
section intro prefix list includes `RPT-` alongside `SPC-`, `REQ-`,
`TSK-`, `QST-`, and `JSON-`; `RPT-001`, `RPT-002`, and `RPT-003` each
have a documented entry in the code block describing their triggers and
`Level::Error` severity, matching the style of the existing `SPC-*` /
`TSK-*` entries.
</coverage>

</report>

## Retry counts

- T-001: 1 retry (tests — missing `Evidence:` field in the
  implementer-note body; style — double-guard pattern in `rpt.rs`
  lines 57-77 was non-idiomatic with a dead `else { continue }` branch;
  `lint/mod.rs` module doc omitted `RPT-*` from the stable-codes list).
  Resolved in attempt-2: `evidence/T-001.md` added with red-then-green
  captures, double-guard collapsed into a single `let Some(req) = ...
  else { push RPT-002; continue; }` expression, and `RPT-*` added to
  the `lint/mod.rs` line-5 module doc.
- T-002: 0 retries (all four personas passed on the first review round).

## Open questions

Two QST-001 unchecked open questions remain in SPEC.md:

1. Whether `RPT-001` should carry a structured representation of the
   underlying parse failure (a `parse_error_kind` field on the
   diagnostic). Resolved at implementation time per the SPEC's
   recommendation: rendered `Display` string only, matching the
   existing `SPC-001` precedent. No `parse_error_kind` field was
   added. Promote if a downstream tool surfaces a need.

2. Whether `RPT-002` and `RPT-003` should special-case
   `result="deferred"` coverage rows. Resolved at implementation time
   per the SPEC's recommendation: uniform treatment — deferred rows
   are validated identically to `satisfied` and `partial` rows,
   matching the in-tree integration test's existing behavior. Promote
   if review feedback surfaces a reason to special-case.

Both questions are info-level lints only and do not block ship per
AGENTS.md "Feedback, not enforcement." Leaving them as `[ ]` in
SPEC.md preserves the historical record.
