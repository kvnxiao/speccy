---
spec: SPEC-0057
generated_at: 2026-06-11T02:11:50Z
---

## Invocation 1 — 2026-06-11T02:11:50Z

<drift-review verdict="pass" round="1" date="2026-06-11T02:11:50Z" model="claude-opus-4-8[1m]">
Diff satisfies SPEC-0057 as a unit: all six requirements (REQ-001..006) and nine scenarios (CHK-001..009) are delivered, no scope creep, no non-goal violated, and `speccy verify` runs clean over the real workspace with zero XML-001 false positives.

- REQ-001..006 all covered: scanner seam (`scan_foreign_tags`, `ForeignTag`, void set) per DEC-004 in `speccy-core/src/parse/xml_scanner/mod.rs` and `html5_names.rs`; name-scoped fence-aware balance pass with single shared `orphan_diagnostic` template in `speccy-core/src/lint/rules/xml.rs`; journal coverage via the JNL-* path-derivation pattern (no `ParsedSpec` field); `XML-001` appended at Error in `registry.rs` with the snapshot re-blessed. All 8 lint_xml + 3 raw_retention + 4 scanner-unit + 1 CLI-verify + registry-snapshot tests pass locally.
- Non-goals respected: scanner passthrough untouched (detection is lint-only, DEC-001); no cross-name nesting enforcement; no unparsed-prose coverage; no autofix. No new CLI flag/env/config — XML-001 rides the existing verify exit-code policy. The only new public items are exactly those the tasks/DEC-004 authorized.
- HashMap-iteration order on the dangling-open emission is not a determinism hazard: `lint::run` sorts all diagnostics by (spec_id, code, file, line) before returning.
- Merge-ordering risk from the SPEC's own Notes is resolved on this branch: no leaked `</content>`/`</invoke>` orphans remain in `.speccy/specs/`, and `cargo run -p speccy-cli -- verify` exits 0 with 0 errors over 5 specs.
</drift-review>
<simplifier-scan verdict="candidates">
One in-scope candidate: collapse the duplicated open/close arms of `classify_foreign_line`.

- speccy-core/src/parse/xml_scanner/mod.rs:292-320 (`classify_foreign_line`) — the open-tag and close-tag arms are byte-identical except `is_close: false` vs `true`: both run `caps.get(1).map(|m| m.as_str().to_owned()).unwrap_or_default()`, check `!whitelist.contains(&name.as_str())`, and push a `ForeignTag`. Collapse to a single match resolving `(captures, is_close)` first, then one push. Behavior-preserving: open is tried before close exactly as today (the regexes are mutually exclusive on a given line, and order is preserved by the if/else-if structure), the whitelist guard and field values are unchanged. Reduces ~18 lines of two-arm duplication to one path. Note: the same `caps.get(1)...unwrap_or_default()` idiom also appears in the pre-existing `classify_line`/`detect_malformed_tag` (out of scope — not in this diff).
</simplifier-scan>
<simplifier-apply verdict="applied">
Collapsed the byte-identical open-tag/close-tag arms in `classify_foreign_line` (speccy-core/src/parse/xml_scanner/mod.rs) into a single resolve-then-push path: open tried before close via `.or_else`, then one shared whitelist-guard + `ForeignTag` push carrying the resolved `is_close`. Behavior-preserving (open-before-close ordering, whitelist guard, and emitted field values unchanged); removed ~13 lines of duplication. All four hygiene gates green: cargo test --workspace, clippy -D warnings, +nightly fmt --check, cargo deny check.
</simplifier-apply>
<gate verdict="passed" tasks_hash="b4fe76735a385f9f6af214a7e9601ac58375b8ef24d1120d441d625d07c96f5c" date="2026-06-11T02:26:10Z">
Drift cleared on round 1 (0 fix rounds); simplifier applied one behavior-preserving candidate (classify_foreign_line arm collapse), hygiene green.
</gate>
