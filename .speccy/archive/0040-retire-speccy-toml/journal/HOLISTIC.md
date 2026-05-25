---
spec: SPEC-0040
generated_at: 2026-05-22T23:24:26Z
---

## Invocation 1 — 2026-05-22T23:24:26Z

<drift-review verdict="blocking" round="1" date="2026-05-22T23:26:55Z" model="claude-opus-4.7[1m]/high">
SPEC-0040 implementation matches contract; hygiene gate is red — `cargo test --workspace` fails two tests due to SPEC.md formatting and unrelated draft pollution.
- REQ-008 done-when (`cargo test --workspace` passes) → `every_migrated_spec_md_has_blank_line_after_each_close_tag` fails because `.speccy/specs/0040-retire-speccy-toml/SPEC.md` has 8 `</scenario>` lines (176, 229, 270, 318, 375, 414, 488, 569) immediately followed by `</requirement>` with no blank line. SPEC-0040's own SPEC.md violates the parser's blank-line-after-close-tag rule. See `.speccy/specs/0040-retire-speccy-toml/SPEC.md:176`.
- REQ-008 done-when (`cargo test --workspace` passes) → `every_in_tree_spec_md_parses_with_xml_parser` fails because `.speccy/specs/0041-vet-lifecycle-step/SPEC.md` is an in-flight draft (missing required `<changelog>`) sitting in the same working tree as SPEC-0040. The pollution makes SPEC-0040's hygiene gate red even though SPEC-0040 itself is correct. See `.speccy/specs/0041-vet-lifecycle-step/SPEC.md`.
- Side discovery: `speccy-core/README.md:5` still lists `speccy.toml` as one of "the five Speccy artifact files". REQ-007 only contracts the root `README.md` and `docs/ARCHITECTURE.md`, so this is out of contracted scope, but it is stale prose that contradicts the post-SPEC-0040 reality. Flag for follow-up rather than fix in-loop.
</drift-review>

<holistic-fix verdict="blocking" round="1" date="2026-05-22T23:28:00Z" model="claude-opus-4.7[1m]/low">
Addressed SPEC-0040 SPEC.md formatting; SPEC-0041 draft pollution and speccy-core/README.md stale prose carried forward as out-of-scope for this SPEC.

Addressed:
- Added a blank line after every `</scenario>` immediately preceding `</requirement>` in `.speccy/specs/0040-retire-speccy-toml/SPEC.md` (8 sites). Re-running `cargo test -p speccy-core --test in_tree_specs every_migrated_spec_md_has_blank_line_after_each_close_tag` would now pass against SPEC-0040 alone; the test still fails because SPEC-0041 also has the same violation (see Not addressed below).

Not addressed (out-of-scope, requires human direction):
- `.speccy/specs/0041-vet-lifecycle-step/SPEC.md` — this is a separate spec drafted in parallel on the same branch. Fixing its `<changelog>` and `</scenario>` formatting is the responsibility of SPEC-0041, not SPEC-0040. Editing it from inside SPEC-0040's holistic loop would scope-creep. Recommend the human either (a) move/stash SPEC-0041 outside the working tree before shipping SPEC-0040, (b) finish drafting SPEC-0041 to a parsable state in a separate commit, or (c) accept the failing test as a known-shape gap and ship SPEC-0040 with the understanding that the next branch will rectify it. Speccy-amend on SPEC-0040 is not the right tool — its contract is correct.
- `speccy-core/README.md:5` — stale prose listing `speccy.toml`. Out of REQ-007's contracted scope (which names only root `README.md` and `docs/ARCHITECTURE.md`). Best handled in the follow-up docs SPEC mentioned under DEC-002 / Out-of-scope follow-ups.

Hygiene after fix: `cargo check --workspace --all-targets --all-features` passes. `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes. `cargo +nightly fmt --all --check` passes. `cargo test --workspace` still fails on the two SPEC-0041-driven tests.
</holistic-fix>

<drift-review verdict="blocking" round="2" date="2026-05-22T23:29:00Z" model="claude-opus-4.7[1m]/high">
Round 1 fix to SPEC-0040 SPEC.md verified against current diff (8 blank lines inserted after `</scenario>` lines preceding `</requirement>`). Carried-forward "Not addressed" items have sound reasons but the hygiene gate remains red, which violates REQ-005 and REQ-008 done-when literally.
- REQ-008 done-when (`cargo test --workspace` passes) → still fails. The implementer correctly identified that SPEC-0041 is out of SPEC-0040's scope, but the SPEC's own done-when text is unambiguous: the full hygiene suite must pass before shipping. The implementer's carry-forward reason is sound (don't scope-creep into SPEC-0041), so the fix belongs outside this loop, not inside another round. Escalating to the human for the disposition of SPEC-0041 draft pollution before `/speccy-ship` is invoked.
- Side discovery (stale `speccy-core/README.md:5`) carried forward; not blocking SPEC-0040 ship, captured under DEC-002 follow-up.
</drift-review>

## Invocation 2 — 2026-05-22T23:31:33Z

<drift-review verdict="pass" round="1" date="2026-05-22T23:34:00Z" model="claude-opus-4.7[1m]/high">
SPEC-0040 implementation matches contract; hygiene gate is now green after the in-place SPEC-0041 fix (blank lines after `</scenario>` closers + `<changelog>` section). All four hygiene gates pass: `cargo test --workspace` exits 0 across every test binary (including `every_migrated_spec_md_has_blank_line_after_each_close_tag` and `every_in_tree_spec_md_parses_with_xml_parser`, which were the two failures blocking round-2 in invocation 1); `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean; `cargo +nightly fmt --all --check` is clean; `cargo deny check` reports `advisories ok, bans ok, licenses ok, sources ok`.
- REQ-001/CHK-001/CHK-002 — `.speccy/speccy.toml` is absent from the working tree; init writes `.speccy/.gitkeep` per the integration test (verified live: file not present in this dev workspace since this workspace was scaffolded long before SPEC-0040).
- REQ-002/CHK-003/CHK-004 — `speccy-core::parse::toml_files` is deleted; no surviving symbols in `*.rs` for `toml_files`, `SpeccyConfig`, `ProjectConfig`, `speccy_toml`, `SUPPORTED_SCHEMA_VERSION`, `guard_schema_version`, `UnsupportedSchemaVersion`.
- REQ-003/CHK-005 — `read_to_string` relocated to `speccy-core/src/parse/fs.rs`; `spec_md` and `workspace` callers compile and pass tests.
- REQ-004/CHK-006/CHK-007 — `speccy-cli/src/templates/speccy.toml.tmpl` deleted; no `speccy.toml` literal in `speccy-cli/src/`.
- REQ-005/CHK-008/CHK-009 — No `speccy.toml` literal in `speccy-cli/tests/` or `speccy-core/tests/`; gitkeep assertion present and green.
- REQ-006/CHK-010/CHK-011 — `toml` crate dropped from `speccy-core/Cargo.toml` explicit dependencies; workspace builds clean.
- REQ-007/CHK-012/CHK-013 — Root `README.md` and `docs/ARCHITECTURE.md` are free of `speccy.toml` and the `## Schema version` / `## speccy.toml` headings; SPC-001 row reflects the SPEC.md parse-failure rule.
- REQ-008/CHK-014/CHK-015/CHK-016 — `"implement"` literal is absent from `speccy-core/src/` and `speccy-cli/src/`; live `speccy next --json` emits `"kind":"decompose"` and `"kind":"ship"` for the current workspace state; no `"implement"` discriminator surfaces.
- Carried-forward `speccy-core/README.md:5` stale prose (`speccy.toml`, `spec.toml` in the parser blurb) is acknowledged out-of-scope for REQ-007 (which contracts only root `README.md` and `docs/ARCHITECTURE.md`) and captured under DEC-002's deferred ARCHITECTURE.md / docs follow-up. Not blocking.
</drift-review>

<simplifier-scan verdict="clean" round="1" date="2026-05-22T23:35:00Z" model="claude-opus-4.7[1m]/medium">
SPEC-0040's diff is dominantly a deletion + a rename (`NextAction::Implement` → `Work`). No simplification candidates worth pursuing:
- `speccy-core/src/parse/fs.rs` (the only new production module) is a single 8-line `read_to_string` wrapper over `fs_err::read_to_string` that lifts errors into `ParseError::Io`. Already minimal — borrows `&Utf8Path`, returns `ParseResult<String>`, uses `Box::new` per the project's boxed-ParseError convention. No further compression without changing behavior.
- The CLI test surface moves (`pin_shape.rs`, `skill_stub_shape.rs` relocated from `speccy-core/tests/` to `speccy-cli/tests/`) are pure relocations to match crate ownership, not new code.
- The `Implement`→`Work` rename touches every callsite mechanically; consolidating further would change call-site shape without a behavior win.
- The deletions (parser module, template, dependency, README/ARCHITECTURE prose) are pure removals; nothing left to simplify.
No file modifications proposed.
</simplifier-scan>
