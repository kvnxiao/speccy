---
spec: SPEC-0057
outcome: implemented
generated_at: 2026-06-10T00:00:00Z
---

# REPORT: SPEC-0057 Unbalanced foreign-tag lint — `speccy verify` flags leaked orphan XML tags in parsed artifacts

<report spec="SPEC-0057">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-002 added `speccy-core/src/lint/rules/xml.rs` with the `XML-001` balance lint. The lint walks `scan_foreign_tags` output maintaining per-name open-line stacks; a close against an empty stack fires one `XML-001` Error at the close's line; any lines remaining on stacks after the walk fire one `XML-001` Error per dangling open. Integration tests in `speccy-core/tests/lint_xml.rs` confirmed: two `XML-001` diagnostics for a `TASKS.md` ending with bare `</content>` and `</invoke>` closes (CHK-001); one `XML-001` for a fixture non-void open with no matching close (CHK-002); zero `XML-001` for a fixture with a balanced `<details>`…`</details>` pair (CHK-003). Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004">
T-001 added `VOID_ELEMENT_NAMES` (14 HTML5 void names) and `is_void_element_name` in `speccy-core/src/parse/xml_scanner/html5_names.rs`. T-002 wired the void guard into the balance pass: foreign opens whose name is in `VOID_ELEMENT_NAMES` are never pushed onto any stack. Integration test (CHK-004) confirmed a lone `<br>` line fires zero `XML-001` while a lone non-void foreign open fires exactly one. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005">
T-001 placed the fence exemption inside `scan_foreign_tags` itself, reusing `collect_code_fence_byte_ranges` and `range_inside_any_fence` from the existing scanner. Lines inside a fenced range are skipped before any tag-shape match, so they never enter balance accounting. Integration test (CHK-005) confirmed a foreign close that appears only inside a fenced block fires zero `XML-001`; a foreign close outside any fence still fires. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006 CHK-007">
T-002 wired the balance pass over `SPEC.md`, `TASKS.md`, and `REPORT.md` using each document's `raw` field and its artifact-specific whitelist. T-003 extended `xml.rs` to derive `journal/T-NNN.md` paths from `spec.tasks_md_ok().tasks`, read each existing file with `fs_err::read_to_string`, and run the same balance pass with `JOURNAL_ELEMENT_NAMES`. Integration tests in `lint_xml.rs` confirmed three `XML-001` diagnostics (one per artifact) for CHK-006 and one journal-file diagnostic for CHK-007. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008">
T-002 registered `("XML-001", Level::Error)` in `speccy-core/src/lint/registry.rs` and re-blessed the `lint_registry.snap` snapshot. Diagnostics are emitted via `Diagnostic::with_location` carrying the artifact path and 1-indexed orphan-tag line. Integration test (CHK-008) used a `status: implemented` fixture workspace and confirmed `speccy verify` exits non-zero with rendered output naming the artifact path and line. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-009">
T-004 added `speccy-core/tests/raw_retention.rs`, parsing valid fixture sources for `SPEC.md`, `TASKS.md`, and `REPORT.md` via `parse_spec_xml`, `parse_task_xml`, and `parse_report_xml` and asserting each resulting document's `raw` field is byte-identical (`==`) to the input source string. No production code changes were required — the `pub raw: String` field was already present. The vet simplifier pass cleaned up one redundant line in `xml_scanner/mod.rs`. Retry count: 0.
</coverage>

</report>
