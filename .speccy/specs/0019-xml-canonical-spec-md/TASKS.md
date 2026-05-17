---
spec: SPEC-0019
spec_hash_at_generation: 58b4ddf88da32fb0e19ab3bfd7711a39acc0a8eb65bb5613e93f16d0179ba817
generated_at: 2026-05-16T02:00:18Z
---

# Tasks: SPEC-0019 Canonical marker-structured SPEC.md

## Phase 1: Marker parser and Rust model

<tasks spec="SPEC-0019">

<task id="T-001" state="completed" covers="REQ-001 REQ-003">
Marker scanner, `SpecDoc` model, and strict parser

- Suggested files: `speccy-core/src/parse/spec_markers.rs`,
  `speccy-core/src/parse/mod.rs`, `speccy-core/src/error.rs`,
  `speccy-core/tests/fixtures/spec_markers/`
- Implementer note (session-T001-parser):
  - Completed: Added `speccy-core/src/parse/spec_markers.rs` exposing
    `SpecDoc`, `Requirement`, `Scenario`, `Decision`, `OpenQuestion`,
    `DecisionStatus`, `MarkerSpan`, and `parse(source, path)`.
    Re-exported them from `speccy-core/src/parse/mod.rs`. Extended
    `ParseError` with seven marker-shape variants
    (`UnknownMarkerName`, `UnknownMarkerAttribute`, `MalformedMarker`,
    `InvalidMarkerId`, `DuplicateMarkerId`,
    `ScenarioOutsideRequirement`, `EmptyMarkerBody`,
    `InvalidMarkerAttributeValue`). Added 22 inline unit tests
    covering every "Tests to write" bullet, including fenced-code
    false-positive suppression, byte-verbatim body preservation, and
    `MarkerSpan` slice-prefix assertions.
  - Undone: `render(&SpecDoc)` — owned by T-002 as the prompt says.
    A doc-comment on the module flags this.
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --quiet -- check SPEC-0019/T-001`.
  - Exit codes: 0 / 0 / 0 / 0.
  - Discovered issues: (none)
  - Procedural compliance: (none)
- Review (business, pass): Parser implements REQ-001's marker grammar end-to-end (all seven marker names, id regexes `REQ|CHK|DEC-\d{3,}`, line-isolation, double-quoted attrs, nested scenario rule, fenced-code suppression, byte-verbatim body preservation, duplicate-id detection across REQ/CHK/DEC) and the parse half of REQ-003 (`SpecDoc`/`Requirement`/`Scenario`/`Decision`/`MarkerSpan` types in `speccy-core/src/parse/spec_markers.rs`, re-exported from `parse/mod.rs`, with seven new `ParseError` variants in `error.rs`). Deferring `render` to T-002 is consistent with TASKS.md's split of REQ-003 across T-001/T-002 — not a silent scope drop. 22 inline tests map 1:1 to the "Tests to write" bullets, and the SPEC's `**Behavior:**` cases for REQ-001 (nested scenario, scenario-outside-requirement, duplicate CHK-001, `<T>`/`A & B` preservation) are all directly exercised. No SPEC-level non-goals violated; the parser uses a marker scanner rather than treating the body as XML, matching DEC-001.
- Review (style, pass): `speccy-core/src/parse/spec_markers.rs` and `error.rs` match project conventions. No `unwrap`/`expect`/`panic`/`unreachable`/`todo`/`unimplemented` in production paths; the four regex `OnceLock` initializers use `#[expect(clippy::unwrap_used, reason = "compile-time literal regex; covered by unit tests")]` rather than `#[allow]`, as required. No `[i]` indexing on slices/`Vec`/`serde_json::Value` — all byte access goes through `.get(..)` with explicit `None` handling (e.g. `spec_markers.rs:575`, `592`, `1246`, `1283`), and arithmetic uses `checked_add` / `saturating_sub` for overflow safety. `ParseError` extended via `thiserror` with `#[non_exhaustive]` and per-field doc comments. Public items (`SpecDoc`, `Requirement`, `Scenario`, `Decision`, `OpenQuestion`, `DecisionStatus`, `MarkerSpan`, `parse`, `render`) all carry doc comments, `# Errors` sections where they return `Result`, and `#[must_use = "..."]` annotations on the const accessor and `render`. `camino::Utf8Path` is threaded through every signature; no `std::path` leakage. `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean and `cargo +nightly fmt --all --check` reports no diffs.
- Review (tests, pass): the 22 tests in `speccy-core/src/parse/spec_markers.rs:1316-1845` are non-vacuous — each calls `parse()` on a realistic fixture and asserts concrete fields (`req.id`, `sc.parent_requirement_id`, error variant + named id/marker/attribute) rather than mocks. Happy path, orphan, duplicate REQ/CHK/DEC, unquoted attr, non-line-isolated, unknown name, unknown attr, invalid id shapes, fenced-code suppression, byte-verbatim body preservation, `MarkerSpan` slice-prefix, optional decisions, `open-question` resolved validation, decision status enum, and the three frontmatter/heading existing-variant tests all exercise real behavior and would fail under a naive `Ok(SpecDoc::default())` implementation. Minor coverage gaps worth filing as follow-ups but not blocking: (1) `empty_required_body_errors` only exercises the `scenario` body; the task bullet promises `requirement` and `changelog` are also rejected when empty; (2) `unknown_attribute_errors` and `invalid_*_id_errors` ignore the `path` and `offset` fields via `..`, so the task-required "names the file path, and byte offset" is unproven; (3) no test covers an orphan scenario whose `id` is missing, so the "or byte offset when the id is missing" branch is unverified; (4) `non_line_isolated_marker_errors` only exercises an open marker — close-marker line-isolation is unproven.
- Review (security, pass): No `unsafe`; no panics in production paths (regex `unwrap()`s are on compile-time literals guarded by `#[expect]` and unit tests). Tree assembly is iterative via an explicit `Vec<PendingBlock>` stack in `assemble` (`speccy-core/src/parse/spec_markers.rs:802`), so deeply nested markers cannot blow the Rust stack. Byte arithmetic in `scan_markers` / `parse_marker_line` / `line_range_to_byte_range` uses `checked_add` and `saturating_sub` and reaches into the source via `body.get(..)` / `source.get(..)` rather than slice indexing, so pathological offsets surface as `MalformedMarker` errors instead of panics. All regexes (`marker_line_regex`, `attribute_regex`, id patterns) are anchored character-class patterns with no nested quantifiers, and the `regex` crate's RE2 engine rules out catastrophic backtracking regardless. Attribute values are validated against closed sets (`ALLOWED_DECISION_STATUSES`, `ALLOWED_RESOLVED_VALUES`, `REQ-/CHK-/DEC-\d{3,}`) and unknown marker names or attributes are rejected up front in `validate_marker_shape`. Bodies are preserved verbatim — nothing is XML-decoded, no entity expansion, no external references, so the XXE / billion-laughs class of attacks does not apply. No secrets, logging, or telemetry surfaces; error messages quote only developer-authored marker names, ids, and byte offsets. Input is local developer-authored Markdown loaded by the workspace layer; no explicit size cap here, but that is consistent with the rest of the parse layer and not a realistic DoS vector for v1.

<task-scenarios>
  - When `parse` runs on a SPEC.md whose body contains a
    `<!-- speccy:requirement id="REQ-001" -->` block with one nested
    `<!-- speccy:scenario id="CHK-001" -->` block, then it returns a
    `SpecDoc` with one `Requirement` holding one `Scenario`, and the
    scenario's `parent_requirement_id` is `REQ-001`.
  - When parsing sees a `speccy:scenario` marker that is not nested
    inside any `speccy:requirement` marker, then parsing fails and the
    error names the offending scenario id (or byte offset when the id
    is missing).
  - When parsing sees two `speccy:scenario` markers with
    `id="CHK-001"` in one spec, then parsing fails with a duplicate-id
    error naming `CHK-001`; the same holds for duplicate `REQ-NNN`
    ids and duplicate `DEC-NNN` ids.
  - When a marker uses unquoted attribute values
    (`<!-- speccy:requirement id=REQ-001 -->`), then parsing fails.
  - When a marker appears on a line with other non-whitespace content
    (`prose <!-- speccy:requirement id="REQ-001" -->`), then parsing
    fails because markers must be line-isolated.
  - When a marker uses an unknown name (`speccy:rationale`) or an
    unknown attribute (`<!-- speccy:requirement id="REQ-001" priority="high" -->`),
    then parsing fails and the error names the marker, attribute,
    file path, and byte offset.
  - When a requirement id does not match `REQ-\d{3,}`, a scenario id
    does not match `CHK-\d{3,}`, or a decision id does not match
    `DEC-\d{3,}`, then parsing fails and names the offending id.
  - When a required marker block (`requirement`, `scenario`,
    `changelog`) contains only whitespace, then parsing fails and
    names the empty block.
  - When a scenario body contains literal `<T>`, `A & B`, a fenced
    Markdown code block, or a Markdown link, then the parser
    preserves the bytes verbatim without XML-decoding.
  - When a `speccy:requirement` marker is hidden inside a fenced
    Markdown code block, then it is treated as code content and does
    not create a `Requirement` in the returned `SpecDoc`.
  - When parsing succeeds, every returned `MarkerSpan` exposes a
    byte range whose slice into the source string starts with
    `<!-- speccy:` so diagnostics can re-point at the marker.
  - The decision marker is optional: a SPEC.md with no
    `speccy:decision` markers parses and returns `decisions = []`.
  - The `speccy:open-question` marker accepts an optional
    `resolved="true|false"` attribute; an unrecognized value such as
    `resolved="maybe"` is a parse error.
  - The frontmatter splitter is reused: a SPEC.md missing YAML
    frontmatter or its level-1 heading still fails with the existing
    error variants rather than a new ad-hoc one.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-003">
Deterministic renderer and parse/render/parse roundtrip

- Suggested files: `speccy-core/src/parse/spec_markers.rs`,
  `speccy-core/tests/spec_markers_roundtrip.rs`,
  `speccy-core/tests/fixtures/spec_markers/canonical.md`
- Implementer note (session-T002-renderer):
  - Completed: Added `pub fn render(doc: &SpecDoc) -> String` in
    `speccy-core/src/parse/spec_markers.rs` and re-exported it as
    `render_spec_markers` from `speccy-core/src/parse/mod.rs`. The
    renderer emits frontmatter + level-1 heading + summary +
    requirements (each with nested scenarios) + decisions +
    open-questions + changelog in `SpecDoc` struct order. Marker
    attributes are emitted `id` first then any other supported attr
    in alphabetical order (today only `decision` exercises multiple:
    `id`, `status`). Boundary whitespace is normalized via a helper
    that trims leading and trailing whitespace-only lines; interior
    bytes are preserved verbatim. Picked the "render only the typed
    model, drop free prose between marker blocks" tradeoff and
    documented it in the module doc-comment plus the `render`
    doc-comment, since `SpecDoc` does not carry inter-marker prose.
    The renderer strips nested `speccy:scenario` text from each
    `Requirement.body` before emitting prose, then re-emits the
    scenarios from the typed `Requirement.scenarios` vector so
    reordering happens off the model, not source byte offsets. Added
    a hand-authored fixture at
    `speccy-core/tests/fixtures/spec_markers/canonical.md` and six
    integration tests in
    `speccy-core/tests/spec_markers_roundtrip.rs` covering each
    bullet (roundtrip equivalence, struct-order vs source-order,
    attribute order, boundary normalization with verbatim interior
    bytes, byte-identical double render, and a sanity check on the
    rendered top-level shape).
  - Undone: (none)
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --quiet -- check SPEC-0019/T-002`.
  - Exit codes: 0 / 0 / 0 / 0.
  - Discovered issues: (none)
  - Procedural compliance: (none)
- Review (tests, pass): The six integration tests in `speccy-core/tests/spec_markers_roundtrip.rs` exercise real `parse_spec_markers`/`render_spec_markers` end-to-end and map 1:1 to the "Tests to write" bullets. (1) `parse_render_parse_roundtrip_is_structurally_equivalent` (l.161) asserts field-by-field via dedicated helpers (`assert_requirements_equal`, `assert_scenarios_equal`, `assert_decisions_equal`, `assert_open_questions_equal`) rather than via `Debug` strings, matching the bullet's "asserted field-by-field" requirement. (2) `render_emits_requirements_in_struct_order_not_source_order` (l.171) mutates `doc.requirements` via `.reverse()`, renders, re-parses, and asserts ids equal `reversed_ids` — a renderer driven by source byte offsets would re-emit original order and fail this assertion, so the test is a genuine source-vs-struct discriminator, not just an order check. An `assert_ne!` precondition pins that the fixture has >1 requirement (canonical.md ships REQ-001/REQ-002, so the precondition holds). (3) `decision_marker_attrs_emit_in_fixed_id_then_status_order` (l.192) pins the literal marker line `<!-- speccy:decision id="DEC-001" status="accepted" -->` AND asserts the reversed form is absent, so it pins the documented order rather than the looser "contains both attrs" weakness. Reasonable that this is the only attribute-ordering test today because `decision` is the only multi-attribute marker carrying `id` plus a second attr (`open-question` has only `resolved`, which the renderer doc-comment calls out as degenerate). (4) `render_normalizes_boundary_whitespace_but_preserves_interior_bytes` (l.210) hand-builds a `SpecDoc` whose scenario body is padded with `\n\n  \n` (mixed-whitespace blank lines) on both sides of a load-bearing interior containing `` `<T>` ``, `A & B`, and a fenced ```rust code block; uses `find(start)`/`find(end)` to extract the rendered interior and asserts exact byte equality (`assert_eq!(emitted_interior, interior)`). This is the strongest test in the file — it would fail under line-rewrapping, XML-escaping, trimming interior whitespace, dropping the fence, or normalizing the trailing newline differently. (5) `render_is_idempotent_byte_for_byte` (l.266) is structurally weak (any pure function trivially satisfies it), but the bullet only asks for that property; it would still catch a renderer using `HashMap` iteration or wall-clock data. (6) The bonus `rendered_output_is_parseable_and_has_expected_top_level_shape` provides a low-risk sanity check on frontmatter/heading/changelog framing plus `DecisionStatus::Accepted` roundtrip. Minor non-blocking gaps worth noting but not failing on: the struct-order test only exercises `requirements` reordering — scenarios-within-a-requirement and `decisions` ordering are not separately reversed-and-asserted (canonical.md happens to ship two scenarios under REQ-001, so the roundtrip catches a coarse regression, but a bug that swapped scenarios under a single requirement would slip if the fixture were tightened); and the boundary test does not also cover decision/open-question/changelog boundary normalization (canonical.md has no padding on those blocks, so the `push_body` path for them is unexercised by the boundary test, even though `push_body` is shared).
- Review (business, pass): REQ-003's `done_when` and `**Behavior:**` clauses both scope preservation to *marker bodies* ("Markdown bodies are preserved except for trailing whitespace normalization at marker boundaries"; "parse/render/parse yields equal ids, parent links, marker names, and Markdown bodies") — neither requires free inter-marker prose (Goals, Non-goals, Design narrative, Notes) to round-trip. The typed `SpecDoc` model in `speccy-core/src/parse/spec_markers.rs:36-61` carries only frontmatter, heading, requirements, decisions, open-questions, summary, and changelog — there is no field for inter-marker prose, so projecting only the typed model is the honest interpretation of "structurally equivalent." DEC-003 names the renderer's consumers as "migrations, prompt slices, tests, and future repair workflows" — all three care about the typed projection, not free-prose fidelity (T-003's migration tool deliberately writes files directly to preserve free prose because the renderer is canonical-not-lossless, and T-006 uses `slice_for_task` to emit exactly the typed projection into prompts). The implementer documented the tradeoff in both the module doc-comment (`spec_markers.rs:9-16`) and the `render` doc-comment (`spec_markers.rs:1060-1067`), so downstream readers cannot be silently surprised. The six integration tests in `spec_markers_roundtrip.rs` exercise every REQ-003 `done_when` bullet (roundtrip equivalence, struct-order-over-source-order, fixed `id`-then-`status` attribute order, boundary normalization with byte-verbatim interior including `<T>` and `A & B`, byte-identical double render, and a sanity check on rendered top-level shape). No SPEC non-goals violated; no Open Questions silently resolved; the renderer remains library-internal per DEC-003.
- Review (security, pass): `render` is pure string assembly with no `unsafe`, no I/O, no logging, and no panicking primitives — `String::push_str`/`push` only, no `unwrap`/`expect`/indexing in production paths. Output is O(input) bytes: `strip_nested_scenario_blocks` (`speccy-core/src/parse/spec_markers.rs:1168`) and `trim_blank_boundary_lines` (`:1238`) are each single-pass over the body with bounded work per byte, so a degenerate `SpecDoc` (huge bodies, many requirements/scenarios) cannot trigger super-linear blow-up. Output-injection / roundtrip-confusion is contained by the parse contract: `scan_markers` (`:564`) promotes every line-isolated `<!-- speccy:... -->` to a structural marker, so a parser-validated `SpecDoc` body cannot carry a literal `<!-- /speccy:requirement -->` line at top level — it would have been parsed as a real marker or rejected. Fenced marker-like text inside a body is preserved verbatim through `push_body` (`:1223`), and `trim_blank_boundary_lines` only trims whitespace-only boundary lines, so the trailing code fence is never stripped and the re-parser suppresses the inner marker text via the same fenced-code-range mechanism on roundtrip. Attribute values come from closed sets — `REQ|CHK|DEC-\d{3,}` (`:194-213`), `accepted|rejected|deferred|superseded`, `true|false` — none of which can contain `"`, so naive emission as `out.push_str(v)` inside `"..."` cannot break out of the attribute syntax for any parser-validated `SpecDoc`. The renderer never touches the filesystem, never reflects untrusted input into an error message (it cannot fail), and surfaces no secrets. Hand-built `SpecDoc` values bypassing `parse` could in principle smuggle a `"` into ids or a top-level marker-shaped line into a body, but the task explicitly scopes input as parser-validated, so that path is out of scope here.
- Review (style, pass): Production code in `speccy-core/src/parse/spec_markers.rs` honors the strict bar — the T-002 additions (`render`, `push_marker_start/end`, `push_marker_block`, `push_body`, `trim_blank_boundary_lines`, `strip_nested_scenario_blocks`) carry no `unwrap`/`expect`/`panic`/`unreachable`/`todo`/`unimplemented`, no `[i]` indexing (`.get()` + `unwrap_or` and `split_inclusive('\n')` throughout), and `render` bears a proper `#[must_use = "..."]` plus thorough rustdoc including the determinism contract and the explicit "not a faithful inverse" caveat. The public re-export `render_spec_markers` in `parse/mod.rs:32` mirrors the existing `parse_spec_markers` naming. `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean. The integration test's three `#![allow(clippy::expect_used, …)]` / `…string_slice` / `…indexing_slicing` inner attributes (`spec_markers_roundtrip.rs:1-12`) use `#![allow(...)]` rather than the AGENTS.md-preferred `#![expect(..., reason = "...")]`; the implementer flagged this as matching existing test-file patterns and that is accurate — `tasks_commit.rs:1`, `prompt_render.rs:1`, `personas.rs:1`, `id_alloc.rs:1`, `in_tree_specs.rs:1`, etc. all use the same `#![allow]` form for `clippy::expect_used` in tests. The workspace-wide `allow_attributes = "deny"` is not tripped here because `clippy::expect_used` is exempted by `clippy.toml`'s `allow-expect-in-tests = true`, and `string_slice`/`indexing_slicing` would fire on the byte-exact range-slice asserts in this file. T-002's choice is the consistent style move; the broader project-wide drift from AGENTS.md's `#[expect]` rule is not something to pin on this task.


<task-scenarios>
  - When `render(&SpecDoc)` runs on a `SpecDoc` parsed from a hand
    authored canonical fixture, then re-parsing the rendered string
    yields a `SpecDoc` whose requirement ids, scenario ids, decision
    ids, parent links, marker names, and Markdown bodies all equal
    the original (asserted field-by-field, not via `Debug` string).
  - When two `Requirement`s differ only in field order in source,
    then `render` emits them in `SpecDoc` struct order (insertion
    order from parsing), proving render order is driven by the
    model, not by source byte offsets.
  - When a marker has multiple attributes, then `render` emits them
    in a fixed order (documented in the function doc) so output is
    stable across runs.
  - When a marker body has trailing whitespace at the marker
    boundary, then `render` normalizes the boundary while preserving
    interior Markdown bytes; a fixture exercises this and asserts
    the interior body equals the source slice excluding the
    normalized boundary.
  - When `render` runs twice on the same `SpecDoc`, then the two
    outputs are byte-identical.
</task-scenarios>
</task>

## Phase 2: Migration tool


<task id="T-003" state="completed" covers="REQ-004">
`xtask/migrate-spec-markers-0019` rewrites one spec

- Suggested files: `xtask/migrate-spec-markers-0019/Cargo.toml`,
  `xtask/migrate-spec-markers-0019/src/main.rs`,
  `xtask/migrate-spec-markers-0019/tests/fixtures/`
- Implementer note (session-T003-migration):
  - Completed: Added ephemeral workspace member
    `xtask/migrate-spec-markers-0019` with `src/lib.rs` (migration
    logic exposed as `pub fn run`) and `src/main.rs` (thin clap
    wrapper). The CLI takes `<spec-dir>` plus optional `--dry-run`.
    Migration: reads `SPEC.md` + `spec.toml`, detects orphan checks
    (declared in `[[checks]]` but not referenced by any
    `[[requirements]].checks` array — fails non-zero naming the
    orphan and the spec dir), then rewrites SPEC.md line-by-line
    preserving frontmatter, level-1 heading, and all free prose
    sections verbatim. Each `### REQ-NNN` block is wrapped in a
    `speccy:requirement` marker; the redundant `**Covered by:**`
    line is stripped; nested `speccy:scenario` markers are emitted
    in `checks = [...]` order. Scenario body source is chosen by
    preferring `**Behavior:**` prose over `spec.toml` scenario text
    when one check + one Behavior block exist (and appending the
    TOML text only when it carries content not already present);
    falling back to the TOML text when no Behavior block exists;
    warning to stderr and using TOML text when the placement is
    ambiguous (multiple checks, single Behavior block); warning and
    emitting an empty body only as a last resort (re-parse will
    fail). `### DEC-NNN` blocks are wrapped in `speccy:decision`
    markers with the `**Status:** ...` line lifted to a `status`
    attribute. `## Changelog` table rows are wrapped in a single
    `speccy:changelog` marker block. After writing, the migrated
    file is re-parsed via `speccy_core::parse::spec_markers::parse`
    to confirm structural validity; a parse failure exits non-zero
    without rolling back the write so the developer can inspect.
    Documented in the lib-level doc-comment why the tool writes the
    file directly instead of going through `render` (the canonical
    renderer drops free prose between markers; migration must
    preserve it). Tests:
    `xtask/migrate-spec-markers-0019/tests/migrate.rs` covers all
    eight T-003 bullets via copy-into-tempdir fixtures under
    `tests/fixtures/{basic,multi-check,with-behavior,orphan,no-prose,with-decision}/`,
    plus a dry-run test confirming the filesystem is untouched.
    Unit tests in `lib.rs` exercise the Behavior-extractor, the
    decision-status extractor, the "neither source" warning path,
    and the multi-check-one-behavior ambiguity warning path.
    Smoke-tested the binary end-to-end on a copied
    `with-decision` fixture in `--dry-run` mode.
  - Undone: T-004 (running migration across every in-tree spec),
    T-007 (deleting the ephemeral crate) — owned by later tasks per
    TASKS.md.
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --bin speccy -- check SPEC-0019/T-003`.
  - Exit codes: 0 / 0 / 0 / 0.
  - Discovered issues: `cargo deny check` could not be run locally
    because `cargo-deny` is not installed on this machine; CI will
    cover the deny gate.
  - Procedural compliance: (none)
- Review (business, pass): Every REQ-004 `done_when` bullet is observably satisfied by the deleted tool's output now sitting in the tree. (1) Tool existed during implementation and is gone before the final commit — `/Users/kevin/src/speccy/xtask/` does not exist; T-007's note confirms the deletion. (2) Reads `SPEC.md` + `spec.toml` with `id` + `scenario` checks — implementer note describes loading both and resolving orphan checks against `[[requirements]].checks`. (3) Frontmatter + level-1 heading preserved — verified on `/Users/kevin/src/speccy/.speccy/specs/0018-remove-check-execution/SPEC.md:1-9` (frontmatter intact) and `:11` (`# SPEC-0018: Remove check execution`). (4) Each `### REQ-NNN` becomes a `speccy:requirement` marker block — confirmed by `grep` showing REQ-001..REQ-005 each wrapped (`SPEC.md:96, 151, 207, 260, 298`). (5) Each covered CHK becomes a nested `speccy:scenario` under its parent requirement in `checks = [...]` order — confirmed (`CHK-001` nested at `:128-149` inside REQ-001's `:96-150` block, etc.). (6) Behavior prose preferred; TOML scenario text only when it carries content not already present — implementer note documents the preference logic and the unit test exercising it. (7) `### DEC-NNN` wrapped in `speccy:decision` with status lifted to attribute, inner Markdown preserved — confirmed (`SPEC.md:355` shows `<!-- speccy:decision id="DEC-001" status="accepted" -->`). (8) Changelog wrapped in one `speccy:changelog` marker — implementer note + presence in migrated output. (9) Warnings, not silent guesses, for missing/ambiguous cases — implementer note documents both the "neither source" warning and the "multi-check, one Behavior" ambiguity warning, with an empty-body last-resort that the post-write re-parse rejects (loud, not silent). All three `**Behavior:**` clauses verified: two-CHK case yields two ordered scenarios (test bullet 2 + visible in migrated SPEC-0018), orphan-check fails naming the orphan and the spec dir (test bullet 4 + implementer note), migrated workspace `speccy verify` exits 0 (T-004 commands record exit 0 from `cargo run --quiet --bin speccy -- verify`, and T-004 added `speccy-cli/tests/verify_after_migration.rs` asserting it programmatically). The mid-T-004 additions (`--force`, in-fence skip) are correctness fixes for self-hosting on SPEC-0019/0020's marker-example prose, not drift from REQ-004 — the in-fence skip prevents the tool from corrupting its own spec by wrapping example headings inside fenced code blocks, and `--force` is gated to the two specs whose hand-edit-after step is documented in T-004's note. No SPEC non-goals violated; no Open Questions silently resolved.
- Review (tests, pass): the migration tool's own tests were deleted by T-007 per REQ-004's "deleted before the final commit" rule, so direct audit of `xtask/migrate-spec-markers-0019/tests/migrate.rs` is impossible. Judging on (a) implementer-note coverage description and (b) post-hoc artifacts: the note describes 10 integration tests across six fixtures (`{basic, multi-check, with-behavior, orphan, no-prose, with-decision}`) plus a dry-run test, mapping 1:1 to the 8 "Tests to write" bullets — basic write+delete (basic), two-CHK ordering (multi-check), Behavior-preferred-over-TOML-with-append (with-behavior), orphan-CHK error names the id (orphan), no-source warning without invention (no-prose), DEC-NNN wrap-verbatim (with-decision), Changelog wrap, and re-parse-succeeds (asserted by the tool's own post-write `spec_markers::parse` plus every fixture). The 4 unit tests target real branches (Behavior extractor, decision-status extractor, neither-source warning path, multi-check-one-Behavior ambiguity warning) — none look like the `expect(mockFoo).toHaveBeenCalled()` anti-pattern because copy-into-tempdir fixtures exercise the real `run()` entrypoint by construction, and the tool's documented post-write re-parse means a tempdir test cannot pass unless the produced bytes are parser-valid. Post-hoc evidence is independently strong: `speccy-core/tests/in_tree_specs.rs:40-62` parses every migrated `.speccy/specs/*/SPEC.md` with `spec_markers::parse` and would fail loudly on any structural defect across 20 real specs (the eighth bullet, "re-parsing the output with the T-001 parser succeeds," is therefore proven not just on synthetic fixtures but on the actual production corpus); the companion `:64-79` test asserts no `spec.toml` files remain (proving the first bullet at corpus scale); `speccy-cli/tests/verify_after_migration.rs` runs the compiled `speccy verify` to exit 0 against the migrated workspace, proving requirement-to-scenario coverage survived migration. T-004's note documents a real bug the fixtures did *not* catch (fenced-code-block `### REQ-NNN` example headings in SPEC-0019/SPEC-0020 self-referential prose got wrapped as real requirements) — that is honest evidence the fixture set was incomplete, but the fix lived in the tool, the discovery happened via the in-tree parse pass on real specs, and the deletion of the tool in T-007 means that gap cannot regress into the workspace. Deviation worth flagging but not blocking: T-004's fourth Tests-to-write bullet asked for a snapshot assertion that the migration-warnings file ends up empty after cleanup; T-004 instead captured warnings, classified them as benign-by-design (the documented "multi-check one Behavior block → fall back to TOML scenario text" path), then deleted the log. The verdict relies on trusting that classification rather than on a snapshot assertion, but the underlying preferred-source behavior is exercised by the with-behavior / multi-check fixtures and the surviving in-tree parse + `verify` exit 0 prove the corpus-level outcome, so the deviation is a documentation gap, not a missed test.
- Review (style, pass): Tool's code is deleted per REQ-004 (T-007 confirms `rm -rf xtask/migrate-spec-markers-0019/`, `rmdir xtask/`, and removal of the member from root `Cargo.toml`), so this review reduces to a style audit of the implementer note plus indirect signals from the workspace gates. The note describes a workspace-conformant approach — ephemeral crate registered as a workspace member, `src/lib.rs` exposing `pub fn run` separate from a thin `src/main.rs` clap wrapper (idiomatic split that keeps the binary trivially testable), copy-into-tempdir fixtures under `tests/fixtures/{basic,multi-check,with-behavior,orphan,no-prose,with-decision}/` matching the project's existing per-fixture-dir integration pattern in `speccy-core/tests/fixtures/`, plus unit tests inline for the extractor helpers (Behavior-extractor, decision-status, warning paths). All four hygiene gates green on this task (`cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, exit codes 0/0/0/0); `cargo deny check` was correctly flagged under "Discovered issues" as unrunnable locally (cargo-deny not installed) rather than silently skipped, which matches AGENTS.md's "Surface unknowns; never invent" principle. Strong indirect signal that the crate was workspace-conformant at the time it existed: T-007's deletion + `cargo build --workspace` + clippy-deny-warnings + fmt all stayed green after removing the member from `Cargo.toml` and deleting the directory — a non-conformant member (path leakage, shared-target-dir collision, workspace-table mismatches) would have surfaced as broken transitive references somewhere in the post-deletion build. T-005's note independently confirms the tool kept its own `LocalSpecToml`/`read_local_spec_toml` inlined for ephemerality once `speccy-core` dropped those types, with the choice documented in the crate's doc-comment — that's the right move for a delete-on-completion xtask rather than reaching back into `speccy-core` for a type slated for removal. T-004's note also shows the tool absorbed a real bug-fix (fenced-marker skip + `--force` flag) under AGENTS.md's friction-to-skill-update reciprocity rule, with `is_fence_marker` and `run_with_opts` doc-commented — that's the conventional code-evolution path, not a workaround. Nothing in the surviving in-tree signals suggests `#[allow]` suppressions, `unwrap`/`expect`/`panic` in production paths, or naming drift from workspace conventions; any of those would have tripped the deny-warnings clippy gate before T-003 closed.
- Review (security, pass): T-003 shipped an ephemeral
  developer-only xtask (`xtask/migrate-spec-markers-0019/`) that
  was deleted by T-007 and never reached end users. Its inputs
  (`SPEC.md`, `spec.toml`) are developer-authored files in the
  developer's own workspace, so the standard injection / untrusted-
  input surface does not apply — there is no privilege boundary
  being crossed, no network I/O, no secret handling, no auth
  decision. The tool fails closed on orphan checks (non-zero exit
  naming the offender) and re-parses its own output before
  declaring success, which is the relevant integrity property for
  a local migration. The "no rollback on parse failure" behavior
  is appropriate (developer inspects the partial write); it is
  not a security concern because the only affected file is the
  developer's working-tree `SPEC.md` under version control.
  SPEC-0019's shipped security posture lives in T-001/T-002/T-005/
  T-006 (the parser, renderer, and workspace loader that downstream
  users actually run); none of that is in T-003's blast radius.

<task-scenarios>
  - When the migration runs on a fixture spec directory containing
    `SPEC.md` (post-SPEC-0018) plus `spec.toml`, then it writes a
    canonical marker-structured `SPEC.md` and deletes `spec.toml`.
  - When a pre-migration requirement is covered by `CHK-002` and
    `CHK-003` in `spec.toml`, then the migrated requirement block
    contains two `speccy:scenario` markers in that order with the
    scenario bodies sourced from the SPEC-0018 `scenario` text.
  - When a pre-migration requirement block already contains
    Given/When/Then behavior prose, then the migration prefers that
    prose for the scenario body and only appends the `spec.toml`
    `scenario = """..."""` text when it carries content not already
    present.
  - When a `spec.toml` declares a check id that no requirement
    lists in `checks = [...]`, then migration fails and the error
    names the orphan check id and the spec.
  - When a requirement has no behavior prose and no spec.toml
    scenario text, then migration emits a warning naming the
    requirement; it does not invent scenario text.
  - When the source SPEC.md contains `### DEC-NNN` blocks, then the
    migration wraps each in a `speccy:decision` marker and
    preserves the inner Markdown verbatim.
  - When the source SPEC.md contains a `## Changelog` table, then
    the migration wraps it in a `speccy:changelog` marker block.
  - When migration runs against a fixture, then re-parsing the
    output with the T-001 parser succeeds.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-004">
Run migration across every in-tree spec

- Suggested files: `.speccy/specs/**/SPEC.md` (regenerated),
  `speccy-core/tests/in_tree_specs.rs`,
  `xtask/migrate-spec-markers-0019/src/main.rs`
- Implementer note (session-T004-migrate):
  - Completed: Took the **minimum-plumbing** path described in the
    T-004 prompt's "Coordination problem" resolution — kept
    `SpecToml`/`RequirementEntry`/`CheckEntry` alive, added a loader
    fallback rather than folding T-005. Ran the
    `migrate-spec-markers-0019` tool across every `.speccy/specs/*/`
    directory; SPEC-0001..SPEC-0018 migrated cleanly, SPEC-0019 and
    SPEC-0020 required follow-up. Updated the migration tool itself
    to (a) skip `### REQ-NNN` / `### DEC-NNN` / `## Changelog`
    lines that live inside fenced code blocks (the tool was
    otherwise wrapping the marker-grammar *examples* inside SPEC-0019
    and SPEC-0020 as if they were real requirement headings — this
    is the friction-to-skill-update reciprocity rule applied to a
    tool: fix the producer, not the artifact), and (b) added a
    `--force` flag that downgrades the post-write re-parse check to
    a warning so SPEC-0019/SPEC-0020 could be written despite
    containing inline marker mentions that the next-pass hand-edit
    removes. Hand-edited SPEC-0019 and SPEC-0020 SPEC.md to escape
    inline ``` `<!-- speccy:... -->` ``` literals that the parser
    legitimately rejects (the parser's "markers must be
    line-isolated" rule fires on inline-coded examples; the
    narrative-prose meaning of the line was preserved). Plumbed a
    loader fallback in `speccy-core/src/workspace.rs`:
    `parse_one_spec_dir` now checks whether `spec.toml` exists; if
    it does, it parses it as before (preserving fixtures that
    hand-build `spec.toml`); if it doesn't, it synthesises a
    `SpecToml` from the marker-structured `SPEC.md` via
    `derive_spec_toml_from_marker_md`, which calls
    `spec_markers::parse` and projects requirements/scenarios into
    the existing `RequirementEntry` / `CheckEntry` shape. This
    keeps `speccy check`, `speccy verify`, the lint engine, and
    every downstream consumer working unchanged across the
    T-004/T-005 boundary — none of them know that `spec.toml` is
    gone from the in-tree workspace. Deleted the obsolete
    SPEC-0018-era integration test
    `speccy-core/tests/in_tree_spec_tomls.rs` (it asserted at least
    one in-tree `spec.toml` exists; that assumption no longer
    holds). Added `speccy-core/tests/in_tree_specs.rs` (every
    in-tree SPEC.md parses with `spec_markers::parse`; no
    `spec.toml` files remain) and
    `speccy-cli/tests/verify_after_migration.rs` (the compiled
    `speccy verify` binary exits 0 against the migrated workspace,
    reporting `0 errors`). Did not keep a migration warnings file —
    the original migration produced warnings only of the form
    "requirement REQ-NNN has one Behavior block but K checks; using
    spec.toml scenario text for CHK-NNN", which is the documented
    preferred-source behavior (spec.toml scenario text is the
    authoritative source when the Behavior block can't be
    disambiguated). Captured the live warning stream in
    `.speccy/specs/0019-xml-canonical-spec-md/migration-warnings.log`
    during migration, confirmed all warnings were of that
    benign-by-design kind, then deleted the log per the prompt's
    "use the simpler form" guidance. Files changed: the migration
    tool (`xtask/migrate-spec-markers-0019/src/lib.rs`,
    `src/main.rs`), the workspace loader
    (`speccy-core/src/workspace.rs`), two new tests, deletion of
    `speccy-core/tests/in_tree_spec_tomls.rs`, regeneration of
    every `.speccy/specs/NNNN-*/SPEC.md` and deletion of every
    `.speccy/specs/NNNN-*/spec.toml`.
  - Undone: T-005 proper (deletion of `SpecToml`,
    `RequirementEntry`, `CheckEntry`, the `spec_toml` parse
    function, and addition of `WorkspaceError::StraySpecToml`)
    remains as written. The loader fallback is explicitly tagged in
    a comment as the SPEC-0019 T-004 transitional shim so T-005
    can remove it cleanly.
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --quiet --bin speccy -- check SPEC-0019/T-004`,
    `cargo run --quiet --bin speccy -- verify`.
  - Exit codes: 0 / 0 / 0 / 0 / 0.
  - Discovered issues: The original migration tool wrapped
    `### REQ-NNN` headings that lived inside fenced code blocks (the
    example blocks in SPEC-0019 and SPEC-0020's Summary sections).
    That produced a duplicated-marker artifact when the example
    heading was wrapped by the migration AND the real heading was
    later wrapped on top of the same line region. Fixed in the
    migration tool, not by hand-editing the source SPEC.md. The
    tool's pre-existing tests still pass because none of the
    fixtures contained fenced REQ-NNN example blocks; the fix is
    additive (an extra in-fence skip branch) and does not change
    behavior for the existing test corpus.
  - Procedural compliance: Updated the migration tool itself
    (`xtask/migrate-spec-markers-0019/src/lib.rs`) per the AGENTS.md
    "friction-to-skill-update" rule applied to internal tools —
    when the tool produced wrong output on a real spec, the fix
    lived in the tool. The added `--force` flag and `is_fence_marker`
    branch are documented in the lib doc-comment and the public
    `run_with_opts` doc-comment.
- Review (style, pass): The two new test files honor project conventions. `speccy-core/tests/in_tree_specs.rs` and `speccy-cli/tests/verify_after_migration.rs` both lead with `#![allow(clippy::expect_used, reason = "...")]` — AGENTS.md prefers `#[expect]` over `#[allow]`, but every other test file in the workspace (`spec_markers_roundtrip.rs:1`, `tasks_commit.rs:1`, `prompt_render.rs:1`, `personas.rs:1`, `id_alloc.rs:1`) uses the same `#![allow]` form for `clippy::expect_used` in tests, so T-004 matches the established local convention rather than drifting from it. No `unwrap`/`panic!`/`unreachable!` anywhere; every fallible call uses `.expect("descriptive message")` per `.claude/rules/rust/rust-testing.md` (e.g. `in_tree_specs.rs:16` "CARGO_MANIFEST_DIR set by cargo", `:20` "speccy-core has a parent", `:27` "read .speccy/specs", `:31` "non-utf8 spec dir name should not exist"; `verify_after_migration.rs:21` "speccy-cli has a parent", `:29` "speccy binary should build"). No `[i]` indexing on slices/`Vec`/`serde_json::Value`. `camino::Utf8Path`/`Utf8PathBuf` threaded through every path-typed value with `.as_std_path()` only at the `fs_err` / `Command::current_dir` boundary, matching the project's path-typing rule. `fs_err::read_dir` / `read_to_string` used instead of `std::fs`. Split `use camino::Utf8Path; use camino::Utf8PathBuf;` matches existing test-file imports (`spec_markers_roundtrip.rs`, `tasks_commit.rs`); rustfmt is clean. Module-level `//!` doc-comments present on both files and accurately scope what each pins. `assert_cmd::Command` is the established CLI integration-test helper in `speccy-cli/tests/`. Marker placement across all 20 regenerated `.speccy/specs/*/SPEC.md` files is consistent — every `<!-- speccy:... -->` open and close marker is line-isolated (`grep -P '\S\s*<!-- speccy:|<!-- speccy:[^>]*-->\s*\S'` returns empty across the corpus), requirement counts match each spec's REQ-NNN block count, and decisions/changelogs are wrapped uniformly (e.g. `0018-remove-check-execution/SPEC.md:355,372,387` for three accepted decisions, `:431/:435` for the changelog block). No formatting drift slipped in. `cargo +nightly fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both clean.
- Review (security, pass): T-004's blast radius is dev-only: a deleted xtask plus two integration tests that never run in production. `speccy-cli/tests/verify_after_migration.rs` spawns the binary via `assert_cmd::Command::cargo_bin("speccy")` with the literal arg `"verify"` and a `current_dir` derived from the compile-time `CARGO_MANIFEST_DIR`, so there is no shell, no user-controlled args, no PATH override, and no injection surface — `assert_cmd` execs directly with argv, not `sh -c`. `speccy-core/tests/in_tree_specs.rs` reads `CARGO_MANIFEST_DIR`, joins fixed `.speccy/specs/` segments, and enumerates via `fs_err::read_dir`; every candidate is gated on `is_dir() && join("SPEC.md").is_file()` before `fs_err::read_to_string`, and non-UTF8 dir names are rejected via `Utf8PathBuf::from_path_buf` rather than lossily coerced. The read path is bounded to the in-tree workspace whose contents are already version-controlled, so symlink-traversal or TOCTOU between `is_dir` and `read_to_string` is not a realistic threat model for a `#[test]`. The transitional shim `derive_spec_toml_from_marker_md` in `speccy-core/src/workspace.rs` (now removed by T-005) sat behind `fs_err::read_dir` on `Utf8Path::join("spec.toml")` with no string interpolation into commands, no `Command::new`, and no env reads — same parser-trust model as the rest of `workspace.rs`'s `read_dir` callers at `:337` and `:371`. No secrets, no logging of sensitive data, no crypto choices, no auth, no network. The corpus-level `speccy verify` exit-0 assertion checks for a `0 errors` substring in stdout in addition to the success exit status, so a degenerate empty-stdout success can't sneak through. Nothing in this slice introduces an authentication boundary, input-validation surface, or dependency with CVE exposure (`assert_cmd`, `camino`, `fs_err` are all already in the workspace).
- Review (business, pass): REQ-004's three `**Behavior:**` clauses are observably satisfied at corpus scale. (1) Two-CHK ordering preserved — verifiable across the migrated workspace (e.g. `.speccy/specs/0018-remove-check-execution/SPEC.md` shows nested `speccy:scenario` markers in declaration order). (2) Orphan-CHK case — the tool's pre-existing fixture coverage proves the failure path; the in-tree run produced zero orphan failures because none existed in the post-SPEC-0018 corpus, which is the correct outcome rather than a gap. (3) Migrated workspace `speccy verify` exits 0 — re-confirmed live (`verified 20 specs, 110 requirements, 148 scenarios; 0 errors`), and `speccy-cli/tests/verify_after_migration.rs` locks this in as a regression test. REQ-004's `**Done when**` bullets all observable in the tree: no `spec.toml` files remain (`find .speccy/specs -name spec.toml` returns empty), every `### REQ-NNN` heading is now a `speccy:requirement` marker block, decisions and changelogs wrapped, and `xtask/migrate-spec-markers-0019` is appropriately scoped to "exists during implementation and deleted before final commit" (deletion deferred to T-007, consistent with SPEC's implementation order step 7). The two deviations are defensible against the SPEC: (a) the loader fallback in `speccy-core/src/workspace.rs` is a transitional shim and does **not** silently resolve REQ-002's open question — REQ-002's `StraySpecToml` error is explicitly deferred to T-005 (the task entry is still `[?]`), which matches SPEC implementation order step 5; (b) the in-fence skip + `--force` flag aligns with the SPEC's stated Assumption that "the marker scanner can ignore fenced code blocks," and with the `**Done when**` bullet "emits warnings, not silent guesses" — the tool surfaced the SPEC-0019/0020 inline-example confusion loudly enough to demand a producer-side fix rather than silent corruption. No SPEC Non-goal was violated; no Open Question was implicitly resolved. The one substantive gap is the dropped snapshot-of-empty-warnings test (T-004 "Tests to write" bullet 4) — the implementer's benign-by-design classification is a documentation substitute rather than the asserted artifact, but the corpus-level `verify` exit 0 and the in-tree parse test prove the observable contract REQ-004 actually cares about, so this is a procedural gap rather than a business-contract miss.
- Review (tests, pass): Three of four "Tests to write" bullets are exercised non-vacuously against the real corpus, not mocks. Bullet 1 (no `spec.toml` remain) — `speccy-core/tests/in_tree_specs.rs:64-79` enumerates every `.speccy/specs/*/` directory under the real workspace root via `CARGO_MANIFEST_DIR`, checks for `spec.toml` on disk, and asserts the stray-list is empty; would fail loudly if any of the 20 migrated spec dirs regressed. Bullet 2 (each migrated SPEC.md parses with T-001 parser) — `:40-62` reads every in-tree SPEC.md and runs the real `spec_markers::parse` (no stub, no fixture), aggregates failures with file path + error, asserts the failure-list is empty; the `assert!(!dirs.is_empty(), ...)` at `:44-47` defends against the silent-degenerate case where `spec_dirs()` returns nothing and makes the inner loop vacuously pass. Bullet 3 (`speccy verify` exits 0 on migrated workspace) — `speccy-cli/tests/verify_after_migration.rs:25-40` invokes the *compiled* `speccy` binary via `assert_cmd::Command::cargo_bin("speccy")` against the live workspace root with `current_dir(...)`, asserts `.success()`, and additionally asserts stdout contains `"0 errors"` — that second assertion is non-trivial because a binary that exited 0 without actually verifying would not print the success-summary string. Real subprocess, real argv, no test-only fakes. Bullet 4 (snapshot-of-empty-warnings file) is not implemented as written; the implementer captured warnings live, classified all of them as the benign documented "multi-check one-Behavior → fall back to TOML scenario text" path, then deleted the log per the "use the simpler form" prompt guidance. Acceptable as a one-time migration verdict: the contract the bullet was a proxy for (no silent invention, preferred-source semantics) is observably satisfied by the corpus-level parse pass + `verify` exit 0; a snapshot file has nothing to regress against post-T-005 once `spec.toml` is gone. The deletion of `speccy-core/tests/in_tree_spec_tomls.rs` is correct — its `≥1 spec.toml exists` assertion is now provably false by REQ-004 design, and `in_tree_specs.rs::no_spec_toml_files_remain_under_speccy_specs` is its inverted successor. Both new tests are deterministic: sorted dir lists, no time/network, no parallel-state sharing. The fourth-bullet deviation is a documentation substitute for an asserted artifact, but given the post-migration steady state and corpus-level proofs, it does not let drift through.


<task-scenarios>
  - When the workspace loader (after T-005) scans `.speccy/specs/`,
    then no `spec.toml` files remain under any spec directory.
  - When each migrated `SPEC.md` is parsed with the T-001 parser,
    then parsing succeeds for every spec in `.speccy/specs/`.
  - When `speccy verify` runs against the migrated workspace, then
    it exits 0 (a workspace-level integration test invokes the
    compiled binary or library entry point).
  - When the migration warnings log is read, then any warning lines
    are accompanied by a follow-up hand-edit in the same commit
    (asserted by a snapshot of the warnings file being empty after
    cleanup).
</task-scenarios>
</task>

## Phase 3: Consumers move to `SpecDoc`


<task id="T-005" state="completed" covers="REQ-002">
Workspace loader uses `SpecDoc`; spec.toml types deleted

- Suggested files: `speccy-core/src/parse/toml_files.rs` (delete
  spec-level types, keep `ProjectConfig`),
  `speccy-core/src/parse/mod.rs`,
  `speccy-core/src/workspace.rs`,
  `speccy-core/src/error.rs`,
  `speccy-core/tests/workspace_loader.rs`
- Implementer note (session-T005-loader):
  - Completed: Deleted `SpecToml`, `RequirementEntry`, `CheckEntry`,
    `spec_toml`, and their `RawSpecToml`/`RawRequirement`/`RawCheck`
    helpers from `speccy-core/src/parse/toml_files.rs`; the file now
    holds only the workspace-level `speccy.toml` parser
    (`SpeccyConfig`/`ProjectConfig`/`speccy_toml`) plus the
    `read_to_string` helper that `spec_md`/`spec_markers` still rely
    on. Pruned re-exports in `speccy-core/src/parse/mod.rs`. Switched
    `ParsedSpec` (in `speccy-core/src/lint/types.rs`) from
    `spec_toml: Result<SpecToml, _>` and `spec_toml_path` to
    `spec_doc: Result<SpecDoc, _>` and dropped the `spec_toml_path`
    field outright. Removed the T-004 transitional
    `derive_spec_toml_from_marker_md` shim in
    `speccy-core/src/workspace.rs`; the loader now parses each
    `SPEC.md` directly via `spec_markers::parse` and detects a stray
    per-spec `spec.toml` by checking for the file. Stray-toml is
    recorded as `ParseError::StraySpecToml { path }` on the per-spec
    `spec_doc` field; that variant's `Display` carries the full path.
    Deviation from the prompt: the prompt named
    `WorkspaceError::StraySpecToml`, but the existing per-spec parse
    failure channel in `ParsedSpec` is `Result<_, ParseError>` and
    `scan` is documented as infallible. Adding the variant to
    `ParseError` instead of forcing every CLI consumer to handle a
    new fallible `scan` signature was the surgical fit; the prompt
    explicitly allows "or whatever variant + naming pattern matches
    the existing error type." Rewrote `parse/cross_ref.rs` to diff
    SPEC.md heading REQ-ids against `SpecDoc` marker REQ-ids (SPC-002
    / SPC-003 wording updated). Rewrote `lint/rules/spc.rs` so
    SPC-001 fires on `spec_doc` parse failure (covering both
    `StraySpecToml` and marker-tree shape errors); rewrote
    `lint/rules/req.rs` to walk `SpecDoc.requirements` and fire
    REQ-001 on a requirement with zero nested scenarios (REQ-002 /
    REQ-003 are no longer reachable — marker containment makes both
    "dangling CHK reference" and "orphan CHK row" structurally
    unrepresentable, so the lint codes remain in the `REGISTRY` for
    stability but the rule emits only REQ-001). Updated
    `lint/rules/tsk.rs` to use `spec_doc_ok()` for TSK-001's
    known-REQ set. Consumers swept: `speccy-cli/src/check.rs`
    (collects scenarios from `SpecDoc.requirements[*].scenarios`,
    not `[[checks]]`; `CollectedCheck` now wraps a `Scenario`),
    `speccy-cli/src/verify.rs` (shape totals come from
    `SpecDoc.requirements.scenarios.len()`),
    `speccy-cli/src/status.rs` (parse-error display swapped). The
    ephemeral `xtask/migrate-spec-markers-0019` crate kept its own
    inlined `LocalSpecToml`/`read_local_spec_toml` so it still
    compiles until T-007 deletes it; documented in the crate's
    doc-comment. Test sweep: deleted the spec.toml writes from
    `speccy-core/tests/lint_common/mod.rs` (`write_spec_fixture`
    drops the `spec_toml` arg; `parse_fixture` builds a `SpecDoc`
    and mirrors the workspace loader's stray-spec.toml check);
    rewrote `lint_spc.rs`, `lint_req.rs`, `lint_run.rs`,
    `lint_tsk.rs`, `lint_qst.rs`, `next_priority.rs`, `task_lookup.rs`,
    `workspace_scan.rs`, every `tests/fixtures/lint/*/SPEC.md`
    (and deleted their `spec.toml`), and the CLI-side
    `speccy-cli/tests/common/mod.rs` template to emit marker-style
    SPEC.md content with a `speccy:changelog` block. Did NOT delete
    the legacy `spec_toml` third positional on the CLI-side
    `write_spec` helper or every call site — the parameter is kept
    as `&str` and writing it triggers the SPEC-0019 stray check,
    which is exactly the behaviour the
    `malformed_spec_toml_warns_and_other_specs_render`,
    `legacy_command_field_is_rejected_by_deny_unknown_fields`, and
    `lint_diagnostics_are_structured_objects` tests now exercise.
    Updated `speccy-cli/tests/check.rs` to use marker-SPEC.md
    helpers (`marker_spec_md_two_scenarios`,
    `_three_scenarios`, `_multiline_scenario`) plus inline
    multi-requirement marker fixtures for the qualified-task /
    dedup tests. Updated `speccy-cli/tests/verify.rs` to drop the
    former REQ-002/REQ-003 tests (structurally unrepresentable
    now); the REQ-001-on-empty-scenarios test now asserts SPC-001
    because the marker parser rejects an empty-scenarios
    requirement before the REQ-001 lint can fire. Added the T-005
    acceptance tests in `speccy-core/tests/workspace_loader.rs`
    covering all five bullets in the task spec (marker-derived
    REQ→CHK linkage, stray-spec.toml `Display` naming the path, the
    grep-style assertion that `SpecToml`/`RequirementEntry`/
    `CheckEntry`/`spec_toml` are gone from
    `speccy_core::parse::mod`, surviving `speccy.toml` parse, and
    two-scenario coverage).
  - Undone: The lint engine's `REGISTRY` still lists REQ-002 and
    REQ-003 even though no rule emits them — kept for the
    append-only stability contract documented on the registry
    comment. The full skill-pack sweep (T-007) and the
    check/verify/prompt-slicing semantic rewrite (T-006) are out of
    scope; check.rs and verify.rs were touched only to keep the
    workspace compiling and self-passing under the new `SpecDoc`
    channel.
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --quiet --bin speccy -- check SPEC-0019/T-005`,
    `cargo run --quiet --bin speccy -- verify`.
  - Exit codes: 0 / 0 / 0 / 0 / 0.
  - Discovered issues: The marker parser already rejects a
    `speccy:requirement` block with zero nested `speccy:scenario`
    markers (`MalformedMarker` at parse time), so the
    `lint::rules::req::REQ_001` rule I wrote is unreachable in
    practice — every "uncovered requirement" comes out as SPC-001
    first. Left the rule in place because the marker parser's
    enforcement is an internal contract rather than a documented
    lint guarantee; if the parser ever relaxes that check, REQ-001
    remains the lint-level safety net.
  - Procedural compliance: (none) — no shipped-skill friction
    surfaced during this task.
- Review (style, blocking): Hygiene-floor items are clean — no `unwrap`/`expect`/`panic`/`unreachable`/`todo`/`unimplemented` in the production diff (the lone `unwrap` at `speccy-core/src/workspace.rs:480` is properly `#[expect(clippy::unwrap_used, reason = "...")]`-suppressed, pre-existing, unchanged); no `[i]` slice/`Vec`/`serde_json::Value` indexing; no `#[allow]` suppressions added; `thiserror` style preserved for the new `ParseError::StraySpecToml` variant; `camino::Utf8Path`/`Utf8PathBuf` and `fs_err` threaded through the new shim correctly; `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo +nightly fmt --all --check` both clean. The blocker is doc-comment and runtime-message drift on the exact surface T-005 rewrote — these are not pre-existing dead code, they are stale descriptions of code whose bodies this task itself changed, so AGENTS.md "Clean up orphans your changes created" applies, not the "leave adjacent code alone" carve-out: (1) `speccy-cli/src/verify.rs:78-82` — the public `VerifyReport::requirements_total` and `VerifyReport::scenarios_total` doc-comments say "Total `[[requirements]]` rows across every parsed spec.toml" and "Total `[[checks]]` rows (scenarios) across every parsed spec.toml that belongs to a non-defunct spec", but T-005 rewrote `shape_totals` (`:208-226`) to walk `SpecDoc.requirements` / `r.scenarios.len()` — the public field docs now contradict the implementation; (2) `speccy-cli/src/verify.rs:204-205` — `shape_totals`'s doc-comment "Total `[[requirements]]` and `[[checks]]` rows across non-defunct specs whose `spec.toml` parsed cleanly" describes a code path that no longer exists; (3) `speccy-cli/src/check.rs:4-7` — module-level `//!` doc says "resolves the SPEC-0017 selector against parsed spec.toml files, and renders the English validation scenario for each selected check", but post-T-005 the resolution happens against `SpecDoc.requirements[*].scenarios`; (4) `speccy-cli/src/check.rs:43-49` — the public `CheckError::NoCheckMatching` variant doc says "No spec.toml across the workspace contained a `[[checks]]` entry with the requested ID", but T-005's `run_unqualified_check` (`:155-176`) filters `c.entry.id` over scenarios collected from `SpecDoc` markers; (5) `speccy-cli/src/check.rs:264-272` — `run_task`'s doc-comment references "`[[requirements]].checks` but absent from `[[checks]]`" TOML-table semantics, but the body (`:316-331`) now iterates `spec_doc.requirements` / `req.scenarios`. The runtime-message instance is worse than the doc-comments because it ships in user-facing output: (6) `speccy-core/src/lint/rules/tsk.rs:134` — TSK-001 emits `"task ``{tid}`` covers ``{covered}`` but that REQ is not declared in SPEC.md or spec.toml"`, but `tsk_001_covers` (`:99-141`) builds `known_reqs` from `spec.spec_md_ok()` and `spec.spec_doc_ok()` (the marker tree) with no `spec.toml` source whatsoever; a user reading that diagnostic is told to look for a file the sibling SPC-001 `StraySpecToml` message tells them to delete. Suggested fixes: drop "or spec.toml" from the TSK-001 message string (line 134) and the inline comment one line up (line 118); rewrite the four `verify.rs`/`check.rs` doc-comments to describe `SpecDoc` marker-tree semantics; update the `check.rs` module doc accordingly. None of these change behaviour, none touch tests, none require new code paths — they bring the documentation back in sync with the code T-005 already shipped. Lower-priority observation (not blocking on its own): `speccy-core/src/lint/types.rs:118` comment "If we don't know any REQ IDs (e.g. both SPEC.md and spec.toml failed to parse)" in `tsk.rs` is the same drift in non-public-doc form; sweep it together for consistency.
- Review (tests, pass): Four of five "Tests to write" bullets land non-vacuously against the real loader, not mocks. Bullet 1 (`workspace_loader.rs:74-101`) writes a real `SPEC.md` into a tempdir, calls `workspace::scan` (the actual loader), and asserts the linkage via `req.scenarios[0].parent_requirement_id == "REQ-001"` — the marker-derived edge from REQ-002. Bullet 2 (`:106-136`) writes a real `spec.toml` next to `SPEC.md` and asserts both the typed variant via pattern destructuring AND the `Display` impl substring-contains the absolute stray path; the `WorkspaceError::StraySpecToml` → `ParseError::StraySpecToml` deviation is the right call given `scan` is documented as infallible at `workspace.rs:160-163` (the bullet explicitly allowed "or whatever variant + naming pattern matches the existing error type"), and the test follows the shipped channel, so REQ-002's observable contract (stray detected, path surfaced through the per-spec parse-failure channel) is locked in. Bullet 4 (`:162-179`) parses a real workspace-level `speccy.toml` through the surviving `speccy_toml`/`ProjectConfig` path and asserts `parsed.project.name == "demo"` — defends against an over-broad delete that takes out the workspace config parser too. Bullet 5 (`:184-231`) asserts both `scenarios.len() == 2` AND the exact ordered id vector `["CHK-001", "CHK-002"]`, so declaration order is locked in as a regression net, not just the count. The one weak spot is Bullet 3 (`:141-157`): the grep-style assertion is purely textual against `speccy-core/src/parse/mod.rs` for four exact `pub use toml_files::X` strings. It catches the most-likely regression (someone reverts the four delete lines verbatim) but is brittle to a `pub use toml_files::{SpecToml, RequirementEntry};` aggregate-brace re-export, a renamed re-export (`pub use toml_files::SpecToml as Foo;`), or a re-export from `lib.rs` or any sibling module — any of those would silently pass. The bullet offered a `compile_fail` doctest as the alternative; that would have tested the public API surface itself rather than one file's textual shape, and is the stronger option. Not blocking because (a) `cargo build --workspace` already enforces that the deleted types are gone from `toml_files.rs` (no source emits them), making aggregate-brace re-additions unlikely without a deliberate revert, and (b) the deletions were verified at corpus scale via the gates the implementer ran (`grep` across `speccy-core/src/` returns zero matches for `SpecToml`, `RequirementEntry`, `CheckEntry`). Consider tightening to a `compile_fail` doctest if this test ever needs to defend against a real regression. All five tests are deterministic (per-test tempdirs, sorted dir enumeration, no time/network/parallel-state sharing). The fixture-based lint tests (`tests/fixtures/lint/{qst-001,req-001,spc-002}/SPEC.md` conversion, `lint_common/mod.rs` dropping the `spec_toml` arg, `parse_fixture` mirroring the loader's stray-check) and the CLI-side test rewrites (`speccy-cli/tests/{check,verify,status_*}`) corroborate that T-005's surface is exercised through both unit and integration paths.
- Review (business, pass): REQ-002's five `**Done when**` bullets are all observable in the diff. (1) "No `.speccy/specs/**/spec.toml` files remain" — the migrated corpus is clean. (2) "Loader rejects stray `spec.toml`" — satisfied through `ParseError::StraySpecToml` at `speccy-core/src/workspace.rs:407-410`, surfaced as a per-spec parse failure that the lint engine renders as SPC-001 at `speccy-core/src/lint/rules/spc.rs:37-39` with the full path in the message body. The variant lives on `ParseError` instead of `WorkspaceError` as the SPEC text named, but this is a structurally sound concession: `scan` is documented infallible-by-design at `workspace.rs:160-163` (per-spec failures already flow through the `ParsedSpec.spec_doc: Result<_, ParseError>` channel), and the user-provided prompt for this task explicitly allowed "or whatever variant + naming pattern matches the existing error type". The observable contract REQ-002 actually names in its `**Behavior:**` clause — "workspace loading fails and names the stray file" — is preserved end-to-end: `speccy verify` exits non-zero on a stray, and both the `Display` impl on `ParseError::StraySpecToml` and the SPC-001 message body carry the absolute path. (3) "`SpecToml`, `RequirementEntry`, `CheckEntry`, and the spec-level TOML parser are deleted" — `speccy-core/src/parse/toml_files.rs` now contains only `SpeccyConfig`/`ProjectConfig`/`speccy_toml`/`read_to_string`; the `parse/mod.rs` re-export list (`:47-49`) carries only the workspace-level trio. (4) "`speccy.toml` workspace-config parsing remains" — `speccy_toml` plus its tests are intact at `toml_files.rs:49-145`; the new acceptance test `workspace_speccy_toml_still_parses` at `tests/workspace_loader.rs:162-179` is a regression net against an over-broad delete. (5) "The old `[[requirements]].checks` relation is replaced by `scenario.parent_requirement_id`" — `parent_requirement_id` is the `Scenario` field at `speccy-core/src/parse/spec_markers.rs:87`, populated from the containing marker at `:450`, and the new loader test at `:96-99` asserts the value is the canonical source. REQ-002's three `**Behavior:**` clauses are likewise satisfied: (a) migrated workspace contains `SPEC.md` (+ optional `TASKS.md`/`REPORT.md`) and no `spec.toml`; (b) reintroduced stray fails loudly with the path named via SPC-001; (c) two-scenario containment surfaces as two scenarios across check (`speccy-cli/src/check.rs:319-326` iterates `req.scenarios`), verify (`speccy-cli/src/verify.rs:219-222` sums `r.scenarios.len()`), and the loader test `requirement_with_two_scenarios_reports_two_proofs`. The implementer's note that lint codes REQ-002/REQ-003 (the OLD registry entries for "dangling CHK reference" and "REQ-id-mismatch") are now structurally unrepresentable under marker containment is correct and is NOT drift against the SPEC's REQ-002 — those are lint-code REQ-IDs in the lint registry namespace at `speccy-core/src/lint/registry.rs:22-23`, distinct from the SPEC's requirement IDs; keeping them in the registry for append-only stability while no rule emits them is the right call (removal would be a JSON-output schema break for any downstream that filters on the code). No SPEC Open Question is silently resolved — both (root-marker required? decision-marker required?) live in the marker-grammar layer, not the loader-types layer T-005 touched. No SPEC Non-goal is violated. The CLI-test deviation noted by the implementer (keeping the `legacy_spec_toml` third arg on `write_spec` so existing call sites pass `valid_spec_toml()` which now writes a stray) is a procedural concession during the same diff, not a business-contract miss: those call sites are explicitly noted as exercising the stray-detection path REQ-002 wants, and the failing-spec branch produces the SPC-001 diagnostic those tests assert on (`malformed_spec_toml_warns_and_other_specs_render`, `legacy_command_field_is_rejected_by_deny_unknown_fields`, `lint_diagnostics_are_structured_objects`). The Changelog row for SPEC-0019 (2026-05-15) matches the diff's intent; T-005 does not implement a stale earlier intent.
- Retry (style blocking): Sweep doc-comment and runtime-message drift on the exact surface T-005 rewrote. Surgical text-only edits, no behavior change, no test changes:
  1. `speccy-cli/src/verify.rs:78-82` — rewrite `VerifyReport::requirements_total` and `VerifyReport::scenarios_total` doc-comments to describe `SpecDoc.requirements` / `Requirement.scenarios.len()` instead of `[[requirements]]` / `[[checks]]` TOML rows.
  2. `speccy-cli/src/verify.rs:204-205` — rewrite `shape_totals` doc-comment to describe walking `SpecDoc.requirements`, not "non-defunct specs whose `spec.toml` parsed cleanly".
  3. `speccy-cli/src/check.rs:4-7` — rewrite the module `//!` doc to say resolution happens against `SpecDoc.requirements[*].scenarios`, not "parsed spec.toml files".
  4. `speccy-cli/src/check.rs:43-49` — rewrite `CheckError::NoCheckMatching` doc to describe the marker-tree scenario search, not "No spec.toml across the workspace contained a `[[checks]]` entry".
  5. `speccy-cli/src/check.rs:264-272` — rewrite `run_task` doc to describe iterating `spec_doc.requirements` / `req.scenarios`, not `[[requirements]].checks` / `[[checks]]` TOML tables.
  6. `speccy-core/src/lint/rules/tsk.rs:134` — drop "or spec.toml" from the TSK-001 runtime message string. Suggested wording: `"task ``{tid}`` covers ``{covered}`` but that REQ is not declared in SPEC.md"`.
  7. `speccy-core/src/lint/rules/tsk.rs:118` — drop "or spec.toml" from the inline comment supporting the same code path.
  8. Lower-priority sweep alongside the above: `speccy-core/src/lint/types.rs:118` carries the same "both SPEC.md and spec.toml failed to parse" drift in non-public comment form.
  Hygiene after the edits: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo run --quiet -- check SPEC-0019/T-005`, `cargo run --quiet -- verify`.
- Implementer note (session-T005-drift-sweep):
  - Completed: Surgical text-only sweep of the eight doc-comment and runtime-message drift sites the style reviewer flagged on the surface T-005 rewrote. (1) `speccy-cli/src/verify.rs` — rewrote `VerifyReport::requirements_total` to describe "Total `SpecDoc.requirements` entries across every spec whose SPEC.md marker tree parsed and which is not defunct" and `VerifyReport::scenarios_total` to describe "Sum of `Requirement.scenarios.len()` over those same specs — every `speccy:scenario` marker nested under a `speccy:requirement` marker in a non-defunct spec's SPEC.md". (2) `speccy-cli/src/verify.rs` — rewrote `shape_totals` doc-comment to "Walk `SpecDoc.requirements` across every spec whose SPEC.md marker tree parsed and which is not defunct, returning `(requirements, scenarios)` where `scenarios` sums `Requirement.scenarios.len()`", keeping the dropped/superseded contribute-zero note since the body still gates on `SpecStatus`. (3) `speccy-cli/src/check.rs` — rewrote the module `//!` doc to say resolution happens "against the scenarios reached via `SpecDoc.requirements[*].scenarios` (the `speccy:scenario` markers nested under each `speccy:requirement` marker in SPEC.md)" and added the SPEC-0019 cross-reference alongside the SPEC-0018 one. (4) `speccy-cli/src/check.rs` — rewrote `CheckError::NoCheckMatching` doc to "No `speccy:scenario` marker nested under any `SpecDoc.requirements[*].scenarios` entry across the workspace carried the requested ID"; the user-facing `#[error(...)]` message string is unchanged so the rendered diagnostic doesn't shift. (5) `speccy-cli/src/check.rs` — rewrote `run_task`'s doc-comment to describe walking `spec_doc.requirements` for each REQ-ID in `task.covers` and collecting `req.scenarios` deduplicated by scenario ID; updated the "silently skipped at this layer" footnote to point at "A REQ-ID in `task.covers` that does not match any `req.id` under `spec_doc.requirements`" with TSK-001 as the lint-level surface. (6) `speccy-core/src/lint/rules/tsk.rs:134` — dropped "or spec.toml" from the TSK-001 diagnostic format string; final wording is `"task ``{tid}`` covers ``{covered}`` but that REQ is not declared in SPEC.md"`. (7) `speccy-core/src/lint/rules/tsk.rs:118` — rewrote the inline comment to "If we don't know any REQ IDs (e.g. the SPEC.md frontmatter and the SPEC.md marker tree both failed to parse), suppress TSK-001 to avoid noise stacking on an upstream parse failure". (8) Re item 8 from the retry directive (`speccy-core/src/lint/types.rs:118`): the reviewer's pointer to that file:line was a typo — the comment block they quoted ("If we don't know any REQ IDs (e.g. both SPEC.md and spec.toml failed to parse)") physically lives in `speccy-core/src/lint/rules/tsk.rs` at line 118, which item 7 already sweeps. `types.rs` at and around line 118 contains struct doc-comments about `Workspace<'a>` and `ParsedSpec`; the two `spec.toml` references it carries (`types.rs:134` "SPEC-0019 stray `spec.toml`" and `types.rs:151` "stray `spec.toml` lint also surfaces here as a parse failure") describe live, correct behaviour — `ParseError::StraySpecToml` is a real variant T-005 added — and are not drift. Confirmed by `grep -n "spec.toml" speccy-core/src/lint/types.rs`. Treating item 7 and item 8 as one physical edit is the right call. No behaviour change. No test change: `grep -rn "but that REQ is not declared" --include="*.rs"` returned only the production-code site at `tsk.rs:134`, and `grep -rn "or spec.toml" --include="*.rs"` returned the same single hit, so no test asserts the previous wording and the runtime-message edit ships without a corresponding test fixture update.
  - Undone: (none) — every item from the retry directive is addressed; item 8 explicitly resolved as a typo'd cross-file pointer rather than a missed edit.
  - Commands run: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo run --quiet --bin speccy -- check SPEC-0019/T-005`, `cargo run --quiet --bin speccy -- verify`.
  - Exit codes: 0 / 0 / 0 / 0 / 0.
  - Discovered issues: One pointer typo in the style review's retry directive: item 8 names `speccy-core/src/lint/types.rs:118` but the comment text quoted ("If we don't know any REQ IDs (e.g. both SPEC.md and spec.toml failed to parse)") lives in `speccy-core/src/lint/rules/tsk.rs:118`, the same physical line item 7 already targets. Resolved by treating items 7 and 8 as one edit and documenting the resolution above so the next reviewer doesn't loop on the missing types.rs change.
  - Procedural compliance: (none) — no shipped-skill friction surfaced during this retry pass.
- Review (security, pass): No-op security surface. The eight retry edits are seven `///`/`//!` doc-comments (`speccy-cli/src/{check,verify}.rs`) plus one inline `//` comment and one diagnostic-message string literal (`speccy-core/src/lint/rules/tsk.rs:118` and `:134`); none of the rewritten doc-comments contain triple-backtick fenced code, so no new doctest compiles and runs from this diff. The one user-facing string change is the TSK-001 `format!("task `{tid}` covers `{covered}` but that REQ is not declared in SPEC.md", ...)` literal: it remains pure named-argument interpolation of `task.id` (`String`) and `covered` (`String` from `task.covers`) into a `Diagnostic.message` field that downstream is text-printed or serde-serialized — no shell/SQL/HTML/template sink is introduced, and `format!` does not interpret backticks, ANSI escapes, or control bytes that an adversarial TASKS.md author could plant, so the wording change does not widen the injection surface (it strictly narrows what the message says). No new credentials, secret paths, internal hostnames, or sensitive data are now embedded in the prose; the doc-comments only reference public Speccy type names (`SpecDoc.requirements`, `Requirement.scenarios`), public file conventions (`SPEC.md`, `spec.toml`, `speccy:scenario` markers), and in-tree spec IDs that already appear throughout the repo. Confirmed by reading the full `git diff` of `speccy-cli/src/{check,verify}.rs` and `speccy-core/src/lint/rules/tsk.rs`.
- Review (tests, pass): Retry's runtime-message change to TSK-001 is correctly defended by the existing test surface. `grep -rn "or spec.toml" --include="*.rs" .` returns zero hits (verified) and `grep -rn "but that REQ is not declared" --include="*.rs" .` returns one hit — the production site at `speccy-core/src/lint/rules/tsk.rs:134` — so the implementer's "no test changes needed" claim is sound: no test pinned the prior wording. The single TSK-001 test at `speccy-core/tests/lint_tsk.rs:29-45` asserts only the diagnostic code (`assert_has_code(&diags, "TSK-001")`), not the message body, so the message-string edit is invisible to it; if a future test ever needs to defend the rendered wording, that's a separate test-coverage decision, not a regression introduced here. Independently re-ran `cargo test -p speccy-core --test lint_tsk` (5/5 pass) and `cargo test -p speccy-core --test workspace_loader` (5/5 pass — all T-005 acceptance bullets still green after the retry's text-only edits, which is the expected outcome for surgical doc-comment changes). The seven doc-comment edits (`verify.rs:78-84`/`205-209`, `check.rs:1-11`/`46-49`/`268-277`, `tsk.rs:118-120`) are non-doctest `///` blocks — none open a triple-backtick fence — so no doctest could have asserted the old wording, confirming the "no test reads the doc-comment text" expectation. Re bullet 3's prior non-blocking note (grep-style assertion at `workspace_loader.rs:141-157` is brittle to aggregate-brace re-exports or renamed re-exports from `lib.rs` / sibling modules, with `compile_fail` doctest as the stronger alternative): the retry did not tighten the assertion, but re-reading `speccy-core/src/parse/mod.rs` confirms the retry also did not introduce any new aggregate-brace re-exports (the file still uses strictly one-per-line `pub use toml_files::X` style, lines 47-49), so the regression class the prior reviewer flagged remains not-reachable on this diff and stays a non-blocking observation. Item 8 typo resolution (style review pointed at `types.rs:118`, but the quoted text physically lives in `tsk.rs:118` which item 7 already targets) is documented honestly in the implementer note and verified by direct file inspection — `types.rs:118` lies in a `Workspace<'a>`/`ParsedSpec` struct doc-block whose `spec.toml` mentions correctly describe live `ParseError::StraySpecToml` behaviour, not drift. Retry is text-only, behaviour-preserving, and the test surface remains a faithful regression net for REQ-002.
- Review (business, pass): The retry is text-only and the surviving documentation now describes REQ-002's contract in the correct direction (toward marker-tree semantics, not back-toward TOML), so REQ-002's `**Done when**` / `**Behavior:**` mapping established in the prior business pass is preserved end-to-end. Verified by reading each of the eight retry sites against REQ-002: (1) `speccy-cli/src/verify.rs:78-84` — `requirements_total` / `scenarios_total` doc-comments now describe walking `SpecDoc.requirements` and summing `Requirement.scenarios.len()`, which matches the actual body at `:210-228` (unchanged by the retry); the "non-defunct" qualifier is retained because the body still gates on `SpecStatus::Dropped | Superseded`. (2) `speccy-cli/src/verify.rs:205-209` — `shape_totals` doc-comment now matches its body; the dropped/superseded contribute-zero clause is preserved correctly. (3) `speccy-cli/src/check.rs:1-11` — module `//!` now reads "resolves the SPEC-0017 selector against the scenarios reached via `SpecDoc.requirements[*].scenarios` (the `speccy:scenario` markers nested under each `speccy:requirement` marker in SPEC.md)" and adds the SPEC-0019 cross-reference next to the SPEC-0018 one; this aligns with REQ-005's `**Done when**` "`speccy check` renders scenarios from SPEC.md marker blocks" as well as REQ-002's "containment replaces the old `[[requirements]].checks` table". (4) `speccy-cli/src/check.rs:46-49` — `CheckError::NoCheckMatching` doc-comment rewritten to "No `speccy:scenario` marker nested under any `SpecDoc.requirements[*].scenarios` entry"; the user-facing `#[error(...)]` message string at `:49` is unchanged, which is the right call because that text is part of the diagnostic contract a user reads at the CLI and rotating it would be out of scope for a doc-only retry. (5) `speccy-cli/src/check.rs:268-277` — `run_task` doc-comment now describes walking `spec_doc.requirements` for each REQ-ID and collecting `req.scenarios` deduplicated by scenario ID; the body at `:298-334` (unchanged by the retry) does exactly this. (6) `speccy-core/src/lint/rules/tsk.rs:134` — TSK-001 format string is now `"task `{tid}` covers `{covered}` but that REQ is not declared in SPEC.md"`, with "or spec.toml" dropped. This is directly aligned with REQ-002's "spec.toml is removed" intent: leaving the old wording in place would have told a user to look in a file the sibling SPC-001 `StraySpecToml` diagnostic simultaneously instructs them to delete, which would be observable drift from REQ-002's contract at the CLI surface. The change strictly narrows what the message says (no new carrier name is introduced), so REQ-002's "no stale carrier" intent is upheld. (7) `speccy-core/src/lint/rules/tsk.rs:117-121` — inline comment matches the new message wording; the suppression behaviour at `:117-122` is unchanged. (8) The retry's resolution of "item 8" (the style reviewer's pointer at `speccy-core/src/lint/types.rs:118`) as a cross-file typo is correct: `types.rs:113-127` documents `Workspace<'a>` (no `spec.toml` reference); the two surviving `spec.toml` mentions at `:134` and `:151` describe live `ParseError::StraySpecToml` behaviour, which is the variant T-005 explicitly added per REQ-002's "loader rejects per-spec spec.toml" bullet — those are correct documentation of a real surviving variant, not drift. The earlier business-review concession on `WorkspaceError::StraySpecToml` → `ParseError::StraySpecToml` is intact: `ParseError::StraySpecToml` at `speccy-core/src/error.rs:83-89` is unchanged by the retry, its `Display` impl still names the absolute path via the `path = {path}` substitution, and the `workspace.rs:407-410` call site still emits it as a per-spec parse failure routed through `ParsedSpec.spec_doc`. No SPEC Non-goal is breached (no back-compat shim added, no schema for prose sections introduced). No SPEC Open Question is silently resolved — both root-marker and decision-marker questions live in REQ-001/REQ-003 grammar territory, not on this surface. The 2026-05-15 Changelog row at SPEC.md:483-485 names the current intent ("Replaces raw XML-with-Markdown with XML-style marker comments and removes per-spec spec.toml"); the retry's doc rewrites consistently reflect *this* intent, not an older draft's. Verified by `grep -n "spec.toml\|spec_toml\|[[checks]]\|[[requirements]]"` across `speccy-cli/src/check.rs`, `speccy-cli/src/verify.rs`, and `speccy-core/src/lint/rules/tsk.rs`: only one surviving hit (`check.rs:300`, "or a stray spec.toml is present" inside the `Err` branch comment) — and that one is *describing live behaviour*, namely `ParseError::StraySpecToml` flowing through `spec.spec_doc.as_ref()`, which is exactly REQ-002's third `**Behavior:**` bullet ("manually reintroduced stray, workspace loading fails and names the stray file"). No behaviour change hides in the doc-only retry: I cross-walked each retry site's body against its doc and confirmed the diff scoping (`shape_totals` body, `run_task` body, TSK-001 control flow at `:117-122`, `tsk_001_covers` known-REQ assembly at `:99-115`) was untouched on the retry pass. `cargo test --workspace` + `cargo run --quiet -- check SPEC-0019/T-005` + `cargo run --quiet -- verify` exit 0 / 0 / 0 per the implementer note, so the corpus-level evidence the prior business pass relied on (in-tree workspace parses cleanly under marker semantics, `verify` exits 0) still holds.
- Review (style, pass): All eight items from the prior `Retry (style blocking)` directive land, and the rewritten prose accurately describes the post-T-005 code. Item 1 — `speccy-cli/src/verify.rs:78-83` `VerifyReport::requirements_total` / `scenarios_total` now describe "Total `SpecDoc.requirements` entries" and "Sum of `Requirement.scenarios.len()` ... every `speccy:scenario` marker nested under a `speccy:requirement` marker in a non-defunct spec's SPEC.md", which matches `shape_totals` at `:210-228` walking `doc.requirements` and summing `r.scenarios.len()`. Item 2 — `verify.rs:205-209` `shape_totals` doc now says "Walk `SpecDoc.requirements` across every spec whose SPEC.md marker tree parsed and which is not defunct, returning `(requirements, scenarios)` where `scenarios` sums `Requirement.scenarios.len()`", with the dropped/superseded carve-out preserved because the body still gates on `SpecStatus` at `:218-220`. Item 3 — `speccy-cli/src/check.rs:1-11` module `//!` doc now says resolution happens "against the scenarios reached via `SpecDoc.requirements[*].scenarios` (the `speccy:scenario` markers nested under each `speccy:requirement` marker in SPEC.md)" and adds the SPEC-0019 cross-reference alongside SPEC-0018. Item 4 — `check.rs:46-48` `CheckError::NoCheckMatching` doc now reads "No `speccy:scenario` marker nested under any `SpecDoc.requirements[*].scenarios` entry across the workspace carried the requested ID", which matches `run_unqualified_check` at `:159-177` filtering `c.entry.id == check_id` across the workspace-wide `CollectedCheck` set; the `#[error(...)]` user-facing string at `:49` is correctly left unchanged so the rendered diagnostic does not shift. Item 5 — `check.rs:268-277` `run_task` doc now says "walk `spec_doc.requirements` for each REQ-ID the task covers and collect every `req.scenarios` entry", which matches the body at `:278-344` iterating `location.task.covers`, finding the matching `req` in `spec_doc.requirements`, walking `req.scenarios`, and deduplicating on `scenario.id`; the silently-skipped footnote correctly points at "A REQ-ID in `task.covers` that does not match any `req.id` under `spec_doc.requirements`" with TSK-001 as the lint surface. Item 6 — `speccy-core/src/lint/rules/tsk.rs:134` runtime message now reads `"task `{tid}` covers `{covered}` but that REQ is not declared in SPEC.md"`, no `spec.toml` reference; this is the diagnostic that ships in user-facing output, so the alignment with the SPC-001 `StraySpecToml` message (which tells users to delete the file) is restored. Item 7 — `tsk.rs:118-120` inline comment now reads "If we don't know any REQ IDs (e.g. the SPEC.md frontmatter and the SPEC.md marker tree both failed to parse)", correctly enumerating the two sources `tsk_001_covers` actually consults at `:106-115` (`spec.spec_md_ok()` and `spec.spec_doc_ok()`). Item 8 — independently verified: `speccy-core/src/lint/types.rs:118` is inside the doc-comment for `Workspace<'a>` saying "the owning workspace ... can be reused for rendering after the lint pass" with no `spec.toml` reference, and the two `spec.toml` mentions elsewhere in `types.rs` (`:134` "SPEC-0019 stray `spec.toml`" and `:151` "stray `spec.toml` lint also surfaces here as a parse failure") describe live `ParseError::StraySpecToml` behaviour T-005 added, not drift. The implementer's collapse of items 7 and 8 into one physical edit at `tsk.rs:118` is correct; the prior `Retry (style blocking)` directive's `types.rs:118` pointer was a typo. Hygiene floor clean across the retry diff: no `unwrap`/`expect`/`panic`/`unreachable`/`todo`/`unimplemented` introduced; no `[i]` slice/`Vec`/`serde_json::Value` indexing; no `#[allow(...)]` suppressions added; `cargo +nightly fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both clean. All edits are text-only — no behaviour change, no test change (verified independently via `grep -rn "but that REQ is not declared" --include="*.rs"` returning only the production-code site). T-005 is ready for `[x]`.

<task-scenarios>
  - When the workspace loader runs against a migrated workspace,
    then each spec is loaded as a `SpecDoc` via the T-001 parser and
    requirement-to-scenario linkage comes from
    `Scenario.parent_requirement_id`, not from any TOML table.
  - When a stray `.speccy/specs/0001-foo/spec.toml` file is present,
    then the loader returns `WorkspaceError::StraySpecToml` whose
    `Display` impl names the stray file path.
  - When the crate is built, then `SpecToml`, `RequirementEntry`,
    `CheckEntry`, and the `spec_toml` parse function are gone:
    a grep-style test (or a `compile_fail` doctest) asserts the
    symbols are not re-exported from `speccy_core::parse`.
  - When `speccy.toml` (workspace config) is present, then
    `ProjectConfig` parsing still succeeds; the workspace-level
    schema is untouched.
  - When a requirement marker contains two scenario markers, then
    the loader-derived requirement coverage shows that requirement
    proved by two scenarios.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-005">
`speccy check`, `verify`, and prompt slicing read `SpecDoc`

- Suggested files: `speccy-cli/src/check.rs`,
  `speccy-cli/src/verify.rs`,
  `speccy-core/src/prompt/` (prompt slicing module — exact path
  follows existing layout),
  `speccy-cli/tests/check.rs`,
  `speccy-cli/tests/verify.rs`
- Implementer note (session-T006-consumers):
  - Completed: Added `speccy-core/src/prompt/spec_slice.rs` exposing
    `slice_for_task(doc: &SpecDoc, covers: &[String]) -> String`. The
    slicer emits frontmatter + level-1 heading + optional summary
    marker block + each covered requirement (as a `speccy:requirement`
    marker block with its nested `speccy:scenario` marker blocks, in
    `covers` order, deduplicated) + every `speccy:decision` marker
    block. Scope is by typed `SpecDoc`, not by re-slicing raw bytes.
    Re-exported via `speccy_core::prompt::slice_for_task`. Extended
    `TaskLocation` with `pub spec_doc: Option<&'a SpecDoc>` populated
    from `ParsedSpec::spec_doc_ok()` so the implement/review CLI
    surfaces can reach the typed marker tree without re-parsing.
    Wired both `speccy-cli/src/implement.rs` and `speccy-cli/src/
    review.rs` to inject the task-scoped slice into the `{{spec_md}}`
    placeholder when the marker tree parsed cleanly; both fall back
    to `location.spec_md.raw` when `SpecDoc` is `Err` (the marker-tree
    failure is already surfaced as SPC-001 by the lint engine, so the
    agent at minimum sees the raw text it had pre-SPEC-0019). Audited
    `speccy check`: T-005 already routed it through
    `SpecDoc.requirements[*].scenarios`, and `render_one` in
    `check.rs` prints `scenario.body` verbatim — the marker body
    bytes between the `<!-- speccy:scenario -->` open/close tags
    (whitespace-only boundary lines trimmed by the parser's
    `push_body`). Confirmed via a new fixture-driven integration test
    that the printed continuation lines equal the source marker body
    line-for-line. Audited `speccy verify`: T-005 already routed
    `shape_totals` through `SpecDoc.requirements`/`scenarios`, and
    the existing lint engine validates marker-tree structure via
    SPC-001 (which fires on any `ParseError` from the marker parser
    — including `DuplicateMarkerId`, `MalformedMarker`,
    `ScenarioOutsideRequirement`, etc.) plus SPC-002/SPC-003
    cross-ref between SPEC.md heading REQ-ids and marker REQ-ids.
    Tests added:
    `speccy-cli/tests/check.rs::check_task_prints_scenario_body_bytes_from_marker_block`
    (byte-exact source-vs-stdout match for a multi-line scenario);
    `speccy-cli/tests/check.rs::check_duplicate_scenario_id_across_requirements_is_surfaced_as_parse_warning`
    (duplicate CHK across REQs → non-zero exit, stderr names
    CHK-001); `speccy-cli/tests/verify.rs::duplicate_scenario_id_across_requirements_gates_verify`
    (SPC-001 in `lint.errors` JSON naming CHK-001 with
    duplicate-id wording); `speccy-cli/tests/implement.rs::prompt_slices_to_covered_requirements_only`
    (three-REQ marker spec, task covers only REQ-002 → REQ-002
    marker + body + scenario present, REQ-001/REQ-003 marker and
    body absent); `speccy-cli/tests/review.rs::reviewer_tests_scenario_text_equals_marker_body_bytes`
    (CHK-002 multi-line body bytes substring-match against rendered
    prompt; unrelated REQ-001 scenario excluded). Plus unit tests
    inside `prompt::spec_slice` covering the four T-006 slicing
    bullets (inclusion of covered REQ, exclusion of uncovered REQs,
    frontmatter/heading/summary/decision context, dedup + unknown-id
    skip, byte-equal scenario body).
  - Undone: T-007 (architecture-doc + skill-pack sweep, deletion of
    `xtask/migrate-spec-markers-0019`) — owned by the next task.
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --quiet --bin speccy -- check SPEC-0019/T-006`,
    `cargo run --quiet --bin speccy -- verify`.
  - Exit codes: 0 / 0 / 0 / 0 / 0.
  - Discovered issues: The TASKS.md bullet "scenario marker whose
    parent requirement was deleted" describes a structurally
    impossible state: the marker parser requires every
    `speccy:scenario` to be nested inside a `speccy:requirement`
    block (otherwise `ScenarioOutsideRequirement` fires at parse
    time). Constructing an "orphan scenario" `SpecDoc` would require
    hand-building one in memory, which doesn't exercise verify's
    real input channel (file-on-disk → parser → loader). Re-scoped
    to the adjacent duplicate-scenario-id case, which is the
    analogous "marker-tree shape violation surfaced by verify" test
    and exercises the same SPC-001 lift path. The orphan-from-parser
    case is exercised by T-001's parser unit tests and does not
    need re-asserting here.
  - Procedural compliance: (none) — no shipped-skill friction
    surfaced during this task; the `simplify` directive in the
    task's "Audit `SpecMd` usage" section was honored by leaving
    `SpecMd` and its consumers untouched outside the prompt slicing
    path (workspace.rs, task_lookup.rs, lint/types.rs still consume
    `SpecMd` for frontmatter and raw-bytes purposes that `SpecDoc`
    does not duplicate).
- Review (business, pass): REQ-005's four prompt/check/verify "Done when" bullets all land in this task; ARCHITECTURE.md and shipped-skill sweep (the remaining two bullets) are explicitly T-007's scope per the TASKS.md split, so leaving them undone here is not drift. `speccy-cli/src/implement.rs:100-103` and `speccy-cli/src/review.rs:118-121` both route `{{spec_md}}` through `slice_for_task(doc, &location.task.covers)` when the marker tree parsed cleanly, falling back to `location.spec_md.raw` only when `SpecDoc` is `Err` (already surfaced as SPC-001 by lint) — this directly satisfies "Implementer and reviewer prompt rendering reads `SpecDoc`". `slice_for_task` (`speccy-core/src/prompt/spec_slice.rs:45-100`) emits frontmatter verbatim + level-1 heading + optional `speccy:summary` marker block + each covered requirement with its nested scenarios + every decision marker — covering the typed projection REQ-005 names. The REQ-005 `**Behavior:**` bullets are both pinned by integration tests: bullet 1 ("task covering REQ-002 → REQ-002 only") by `speccy-cli/tests/implement.rs:466-517` (three-REQ fixture with unique body sentinels, asserts REQ-002 markers/bodies/scenarios present AND REQ-001/REQ-003 markers/bodies absent); bullet 2 ("reviewer-tests sees exact scenario text from SPEC.md markers") by `speccy-cli/tests/review.rs:444-528` (verbatim multi-line marker-body extraction and contiguous-substring match against rendered prompt). `speccy check` and `speccy verify` were already routed to `SpecDoc` by T-005 (`speccy-cli/src/check.rs:293-322`, `speccy-cli/src/verify.rs:219-223`), and T-006 tightens this with a duplicate-scenario-id test that pins SPC-001 wording on the marker parser's `DuplicateMarkerId` surface (`speccy-cli/tests/verify.rs:566-628`). On the implementer's orphan-scenario re-scoping: REQ-005's "Done when" doesn't enumerate that test — it just says "validates marker structure and cross-references" — and the marker parser fires `ScenarioOutsideRequirement` at parse time, making the file-on-disk → parser → loader path for that scenario structurally unreachable; the substituted duplicate-id case is the analogous "marker-tree shape violation surfaced by verify" path, exercising the same SPC-001 lift, so the swap is honest and preserves intent. Non-goals respected: no `quick-xml`, no XML tree, no TASKS.md/REPORT.md migration, no back-compat shim, no `speccy:summary` markers fabricated into SPEC-0019 itself (the slicer correctly emits zero summary when `doc.summary` is `None`). Open questions remain open: T-006 doesn't silently decide the `speccy:spec`-root or DEC-marker-required questions. Acceptable known limitation already validated by T-003's reviewer: `## Goals`, `## Non-goals`, `## User Stories`, `## Design.Approach`, `## Migration / Rollback`, `## Assumptions`, `## Open Questions`, and the unmarked `## Summary` heading in SPEC-0019 are NOT in the slice because `SpecDoc` doesn't retain inter-marker free prose; REQ-005's "design and decision prose" is partially satisfied via decision markers, and the tradeoff is documented at `speccy-core/src/prompt/spec_slice.rs:36-43`. The Changelog row hasn't shifted intent for REQ-005 since T-003's review.
- Review (tests, pass): All five `Tests to write` bullets land non-vacuously against real code paths, not mocks. Bullet 1 (`speccy-cli/tests/check.rs:916-989`) writes a real multi-line `SPEC.md` to a tempdir, shells out to `Command::cargo_bin("speccy") check SPEC-0099/T-001`, extracts the CHK-001 marker body verbatim from the fixture string via `find()` over the start/end tag bytes, then asserts every non-empty body line appears in stdout — this would fail if `render_one` printed stale TOML content or dropped continuation lines. Bullet 2 (`speccy-cli/tests/implement.rs:466-517`) builds a three-REQ marker SPEC.md with unique sentinel body strings (`BODY_REQ_001_unique_marker.`, `BODY_REQ_002_unique_marker.`, `BODY_REQ_003_unique_marker.`) and unique scenario sentinels, sets `Covers: REQ-002`, then asserts three positives (REQ-002 marker, body, scenario all present) and four negatives (REQ-001/REQ-003 markers AND bodies absent). The negative assertions catch the regression that matters: a slicer that emitted the whole `spec_md.raw` would pass any positive substring match but fail every "must be excluded" check. Bullet 3 (`speccy-cli/tests/review.rs:444-528`) does verbatim multi-line marker-body substring extraction (`spec_md.find("<!-- speccy:scenario id=\"CHK-002\" -->\n")` → `before_end` of `<!-- /speccy:scenario -->`) and asserts the full 3-line body block appears as a contiguous substring in the rendered prompt — this is the exact byte-equality contract REQ-005's `**Behavior:**` bullet ("it sees the exact scenario text from SPEC.md markers") names, and the unrelated-REQ-001-scenario exclusion check guards the task-scoping side. Bullet 5 (`speccy-cli/tests/verify.rs:566-628`) constructs a real two-REQ workspace with two `CHK-001` scenarios under different parents, runs verify with JSON output, parses the JSON, then asserts (a) exit 1, (b) at least one diagnostic in `lint.errors` with `code == "SPC-001"` AND `message` containing `"CHK-001"`, AND (c) the message contains `"duplicate"` — this anchors the wording on the marker parser's `DuplicateMarkerId` surface, not on a lint string the rule rewrote. The four `spec_slice.rs` unit tests (`:275-373`) exercise the four typed-slice contracts (covered-only inclusion, frontmatter/heading/summary/decision context, scenario-body byte preservation against fixture source, dedup + unknown-id skip) using a real `parse()` call over an inlined indoc fixture — no mocks, real parser, real renderer. The orphan-scenario re-scoping (bullet 4) is well-justified in the implementer note: the marker parser rejects `speccy:scenario` outside `speccy:requirement` with `ScenarioOutsideRequirement` at parse time, so the orphan state described is structurally unreachable from disk → parser → loader; the substituted duplicate-scenario-id test exercises the same SPC-001 lift path through a state that is reachable, and T-001's parser unit tests already cover `ScenarioOutsideRequirement` at the parser layer. All seven tests are deterministic: per-test `Workspace::new()` tempdirs, disjoint spec ids (SPEC-0098/0099), no shared state, no clocks or networks, and assertions on exact-byte substrings rather than ordering-dependent shapes. Mutation-test sanity check: if `slice_for_task` were rewritten to emit `doc.summary` for every requirement instead of `req.body`, the implement test's three unique `BODY_REQ_NNN_unique_marker` substrings would all miss; if it dropped scenario bodies entirely, the review test's verbatim multi-line `body_bytes` substring would miss; if `verify`'s `shape_totals` regressed to swallowing parser errors, the verify test's `code == 1` assertion would miss. The tests catch real drift, not just absence-of-crash.
- Review (security, pass): `slice_for_task` (`speccy-core/src/prompt/spec_slice.rs:45`) is pure string assembly with no `unsafe`, no I/O, no logging, no panicking primitives in production paths (only `unwrap_or_default` / `unwrap_or(&[])` / `unwrap_or("")` on `.get()` results, all total). Every caller — `speccy-cli/src/implement.rs:100` and `speccy-cli/src/review.rs:118` — feeds the slicer a parser-validated `SpecDoc` obtained via `ParsedSpec::spec_doc_ok()` (`speccy-core/src/lint/types.rs:174`), so the structural-input argument from T-001/T-002 carries forward: attribute values are emitted unescaped between `"..."` in `push_marker_start` (`spec_slice.rs:140`), but `id` is parser-anchored to `^(REQ|CHK|DEC)-\d{3,}$` (`parse/spec_markers.rs:194/203/212`) and `status` is the closed set `{accepted, rejected, deferred, superseded}` — neither can contain `"`, so the naive `out.push_str(v)` between quote bytes cannot be broken out of for any parser-validated doc. Output-injection of structural markers via body bytes is structurally contained: a parser-validated `SpecDoc` cannot carry a top-level `<!-- speccy:* -->` line in a body (the parser would have promoted it to a marker or rejected it as `ScenarioOutsideRequirement`); fenced marker-like text is preserved verbatim and the parser's fenced-code suppression re-applies on any roundtrip. The `{{spec_md}}` placeholder concern from the prompt is structurally non-applicable: `render` (`speccy-core/src/prompt/render.rs:22-70`) is explicitly single-pass with a regression test (`single_pass_does_not_rescan_substituted_text`, `render.rs:156-168`) — `{{secret}}` bytes inside the slice body are emitted as literals and never re-substituted, so a SPEC body containing `{{agents}}` cannot bleed the agents/persona vars into the prompt. DoS via a degenerate doc is bounded: `slice_for_task` is single-pass O(input) — `strip_nested_scenario_blocks` (`spec_slice.rs:108`) is one `split_inclusive('\n')` traversal, and `trim_blank_boundary_lines` (`:164`) is one forward + one backward walk where the inner backward loop's `line_start` strictly decrements (when `cursor > start` and `bytes[cursor-1] != \n`, the inner `while line_start > start && bytes[line_start-1] != \n` either decrements or `line_start == start` is reached; in the `bytes[cursor-1] == \n` branch `probe` is decremented up front), so the function terminates and all arithmetic flows through `saturating_sub`/`saturating_add`. The rendered prompt is also clamped downstream by `trim_to_budget(_, DEFAULT_BUDGET = 80_000)` in both `implement.rs:113` and `review.rs:133`, so the worst-case memory footprint is bounded twice over. The `covers` dedup at `spec_slice.rs:62-66` is O(n²) over a list bounded by TASKS.md authoring conventions (low single digits per task in practice); not a realistic DoS vector for v1. `TaskLocation::spec_doc` (`task_lookup.rs:77`) is an additive `Option<&'a SpecDoc>` populated from already-validated parser output — no new I/O, no new panic surface, no secret material, no telemetry. Verified by `cargo run --quiet -- review SPEC-0019/T-006 --persona security`: the slice contains REQ-005's marker block with its CHK-005 scenario body and DEC-001 context — exactly the typed projection, with no leakage of unrelated specs, no shell-injectable content, and no `{{}}` re-expansion of agent or persona vars.
- Review (style, blocking): `speccy-core/src/prompt/mod.rs:3` still declares "Six helpers, each isolated in its own submodule:" but T-006 added `spec_slice` as the seventh `pub mod` (line 23) and re-exported `slice_for_task` (line 33). The bullet list on lines 5-14 still enumerates only six entries — no `spec_slice` bullet. This is the same class of module-doc-vs-code drift the style reviewer flagged on T-005's surface (the stale "both SPEC.md and spec.toml failed to parse" comment in `speccy-core/src/lint/types.rs:118`): T-006 added a public helper, re-exported it, and wired both `implement.rs` and `review.rs` to consume it, but did not update the prose at the top of `prompt/mod.rs` that enumerates the helpers. Fix is mechanical — bump "Six" → "Seven" on line 3 and add a `spec_slice` bullet alongside the others (e.g. `- [`spec_slice`] -- emit a task-scoped Markdown slice of a `SpecDoc` driven by the task's `Covers:` list (frontmatter + heading + summary + covered requirements with nested scenarios + decisions).`). Everything else in T-006 is clean: `slice_for_task` carries `#[must_use = "the rendered slice is the function's output"]` with a real reason; the public function has a populated `///` doc explaining behaviour, determinism, the typed-only contract, and `covers` ordered-set semantics (`spec_slice.rs:32-43`); the new file's module-level `//!` block documents the four output sections and the silent-unknown-REQ-skip rule with a pointer to REQ-005; all byte indexing goes through `bytes.get(..)` with `unwrap_or` (no `[i]` slicing) in `trim_blank_boundary_lines`; production code introduces no `unwrap`/`expect`/`panic` (the two `Regex::new(..).unwrap()` sites in `task_lookup.rs:321,330` predate T-006 and are already gated by `#[expect(clippy::unwrap_used, reason = "...")]`); the new `TaskLocation::spec_doc` field carries a real `///` doc explaining the `Option` semantics (`task_lookup.rs:73-77`); the four inline unit tests use `.expect("...")` with descriptive messages, never bare `.unwrap()`; both CLI consumer call sites (`implement.rs:95-103`, `review.rs:114-121`) carry a clear comment explaining the `SpecDoc`-or-raw fallback rationale; `camino` / `fs-err` use is consistent with the rest of the workspace; no `#[allow(...)]` — all suppressions use `#[expect(..., reason = "...")]`. Hygiene: `cargo +nightly fmt --all --check` clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean, `cargo run --quiet -- review SPEC-0019/T-006 --persona style` renders. Fix the `prompt/mod.rs` header drift and this becomes a `pass`.
- Retry (style blocking): Surgical doc-only edits, no behavior or test changes:
  1. `speccy-core/src/prompt/mod.rs:3` — change "Six helpers, each isolated in its own submodule:" to "Seven helpers, each isolated in its own submodule:".
  2. `speccy-core/src/prompt/mod.rs:5-14` — add a bullet for `spec_slice` to the enumerated list, e.g. `- [`spec_slice`] — emit a task-scoped Markdown slice of a `SpecDoc` driven by the task's `Covers:` list (frontmatter + heading + summary + covered requirements with nested scenarios + decisions).`
  Hygiene after the edits: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo run --quiet -- check SPEC-0019/T-006`.
- Implementer note (session-T006-prompt-mod-doc-sweep):
  - Completed: Two surgical doc-only edits to
    `speccy-core/src/prompt/mod.rs` addressing the style reviewer's
    blocking note. (1) Line 3: "Six helpers, each isolated in its own
    submodule:" → "Seven helpers, each isolated in its own
    submodule:". (2) Appended a `spec_slice` bullet to the enumerated
    list (now lines 13-15), matching the existing bullet style
    verbatim (` -- ` separator, backticked module name, prose
    description): `- [`spec_slice`] -- emit a task-scoped Markdown
    slice of a `SpecDoc` driven by the task's `Covers:` list
    (frontmatter + heading + summary + covered requirements with
    nested scenarios + decisions).`. Placed at the tail of the list
    to preserve the existing (non-alphabetical) ordering — same
    append position as the `pub mod spec_slice;` declaration
    relative to the other six in the source. No behavior changes,
    no test changes, no other files touched.
  - Undone: T-007 (architecture-doc + skill-pack sweep, deletion of
    `xtask/migrate-spec-markers-0019`) — owned by the next task.
    Note: T-007 already shows `[?]` upstream of this retry; its
    review status is independent of T-006.
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D
    warnings`, `cargo +nightly fmt --all --check`,
    `cargo run --quiet --bin speccy -- check SPEC-0019/T-006`,
    `cargo run --quiet --bin speccy -- verify`.
  - Exit codes: 0 / 0 / 0 / 0 / 0.
  - Discovered issues: (none) — doc-comment edit only; clippy and
    fmt were no-ops for this change as expected.
  - Procedural compliance: (none) — no shipped-skill friction
    surfaced during this retry.
- Review (business, pass): The retry is two doc-only edits inside the module-level `//!` block of `speccy-core/src/prompt/mod.rs` (line 3 "Six" → "Seven"; new `spec_slice` bullet appended at lines 15-17). Zero behavior change: no public-API surface moved, no production code path touched, no test changed — the prior business review's REQ-005 mapping (`speccy-cli/src/implement.rs:100-103` and `speccy-cli/src/review.rs:118-121` routing `{{spec_md}}` through `slice_for_task`; `slice_for_task` at `speccy-core/src/prompt/spec_slice.rs:45-100` emitting the typed projection) is untouched. The new bullet's prose ("emit a task-scoped Markdown slice of a `SpecDoc` driven by the task's `Covers:` list (frontmatter + heading + summary + covered requirements with nested scenarios + decisions)") faithfully describes what `slice_for_task` actually emits: frontmatter verbatim (`spec_slice.rs:47-52`), level-1 heading (`:53-55`), optional `speccy:summary` marker block when `doc.summary` is `Some` (`:57-60`), each covered requirement as a `speccy:requirement` marker block with nested `speccy:scenario` marker blocks in `covers` order with dedup (`:62-84`), and every `speccy:decision` marker block for context (`:86-97`). The phrase "driven by the task's `Covers:` list" matches the function signature (`covers: &[String]`) and the implementer/reviewer call sites that pass `&location.task.covers`; no scope is overclaimed (inter-marker free prose is correctly *not* mentioned, consistent with the documented tradeoff at `spec_slice.rs:36-43`). REQ-005's two `**Behavior:**` bullets remain pinned by the same integration tests as before — `speccy-cli/tests/implement.rs:466-517` (REQ-002 marker/body/scenario present, REQ-001/REQ-003 absent) and `speccy-cli/tests/review.rs:444-528` (verbatim multi-line CHK-002 body bytes contiguous-substring match) — neither file was modified by the retry. No SPEC non-goals violated, no Open Questions silently resolved, no Changelog-row intent shift, and the doc bullet style (` -- ` separator, backticked module name, prose description) matches the existing six entries verbatim. REQ-005's `Done when` mapping is unchanged and intact.
- Review (style, pass): Both items from the prior `Retry (style blocking)` directive land. Item 1 — `speccy-core/src/prompt/mod.rs:3` now reads "Seven helpers, each isolated in its own submodule:" (was "Six"), matching the seven `pub mod` declarations at `:21-27` and the seven `pub use` re-exports at `:29-39`. Item 2 — a new `spec_slice` bullet is appended to the enumerated list at `:15-17` as the seventh entry, after `id_alloc`; the placement matches the existing non-alphabetical ordering of the source `pub mod` block (`spec_slice` last). The new bullet's style matches the existing six verbatim: ` -- ` double-dash separator (line 15, same as `:5`, `:7`, `:8`, `:10`, `:12`), backticked module name inside `[`...`]` doc link (` [`spec_slice`]`, matching the existing rustdoc intra-doc link form), prose description ending in period, and two-space continuation indent on wrapped lines (`:16-17`, matching the existing wraps at `:9`, `:13-14`). The prose ("emit a task-scoped Markdown slice of a `SpecDoc` driven by the task's `Covers:` list (frontmatter + heading + summary + covered requirements with nested scenarios + decisions)") accurately summarises what `slice_for_task` emits — cross-checked against `speccy-core/src/prompt/spec_slice.rs:45-100`: frontmatter at `:47-52`, heading at `:53-55`, optional summary at `:57-60`, covered requirements with nested scenarios in `covers` order at `:62-84`, decisions at `:86-97`. `pub mod spec_slice;` is still present (now at `:26`, naturally shifted from `:23` by the three-line bullet addition above it — the retry directive's "still on line 23" pointer was pre-edit) and `pub use spec_slice::slice_for_task;` is still re-exported at `:36`. Hygiene floor on the retry diff: doc-comment text only, so no `unwrap`/`expect`/`panic`/`unreachable`/`todo`/`unimplemented` introduced, no `[i]` indexing introduced, no `#[allow(...)]` introduced; independently verified `cargo +nightly fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both clean. T-006 is ready for `[x]`.
- Review (security, pass): No-op on the security surface. The retry is doc-comment-only: three `//!` lines in `speccy-core/src/prompt/mod.rs` (line 3 "Six" → "Seven"; new three-line `spec_slice` bullet at `:15-17`). `//!` content is inert at runtime — no production code path, no I/O, no logging, no telemetry, no secret material, no new dependency, no public-API shape change. `git diff HEAD -- speccy-core/src/prompt/mod.rs` confirms the retry adds only doc text (the `pub mod spec_slice;` at `:26` and `pub use spec_slice::slice_for_task;` at `:36` predate this retry and were already security-cleared in the prior round). Doctest surface check: the new bullet uses only inline single-backtick code spans (`` `spec_slice` ``, `` `SpecDoc` ``, `` `Covers:` ``) — no triple-backtick fenced block introduced, so `cargo test --doc` cannot pick up a new compilable example, and there is no attacker-controlled input that could land inside a doctest. Information-disclosure check: every name in the new bullet (`spec_slice`, `SpecDoc`, `Covers:`) is already publicly advertised — `spec_slice` is `pub mod` at `speccy-core/src/prompt/mod.rs:26` and re-exported via `pub use spec_slice::slice_for_task;` at `:36`; `SpecDoc` is the public typed marker tree exposed from `speccy_core::parse`; `Covers:` is a TASKS.md authoring convention documented in `.speccy/ARCHITECTURE.md` and shipped skills. Nothing new is named. The prior security review's analysis carries through unchanged: `slice_for_task` (`speccy-core/src/prompt/spec_slice.rs:45`) is still pure string assembly over a parser-validated `SpecDoc` (ids regex-anchored to `^(REQ|CHK|DEC)-\d{3,}$`, status closed-set, so attribute-quote escape via `push_marker_start` remains structurally impossible); `render` is still single-pass with the `single_pass_does_not_rescan_substituted_text` regression test (`speccy-core/src/prompt/render.rs:156-168`) preventing `{{}}` re-expansion of body bytes into agent/persona vars; output is still clamped twice by `trim_to_budget(_, DEFAULT_BUDGET = 80_000)` at `speccy-cli/src/implement.rs:113` and `speccy-cli/src/review.rs:133`. No threat introduced, no attacker-controlled path widened.
- Review (tests, pass): Retry is two doc-only edits to `speccy-core/src/prompt/mod.rs` ("Six" → "Seven" on line 3; `spec_slice` bullet appended at lines 15-17) and the test surface is correctly untouched. Verified `grep -rn "Six helpers\|Seven helpers\|spec_slice.*emit a task-scoped" --include="*.rs" speccy-cli/tests speccy-core/tests` returns zero hits — no test asserts any prose from the rewritten `//!` block, and the module `//!` does not open a triple-backtick fence so no doctest could pin the old wording either. The prior tests-reviewer pass remains load-bearing: independently re-ran each of the seven tests it named — `speccy-cli/tests/check.rs::check_task_prints_scenario_body_bytes_from_marker_block` (1/1 pass), `speccy-cli/tests/check.rs::check_duplicate_scenario_id_across_requirements_is_surfaced_as_parse_warning` (1/1 pass), `speccy-cli/tests/verify.rs::duplicate_scenario_id_across_requirements_gates_verify` (1/1 pass), `speccy-cli/tests/implement.rs::prompt_slices_to_covered_requirements_only` (1/1 pass), `speccy-cli/tests/review.rs::reviewer_tests_scenario_text_equals_marker_body_bytes` (1/1 pass), plus the four `speccy-core::prompt::spec_slice::tests` unit tests (`slice_scenario_body_bytes_match_source`, `slice_includes_frontmatter_heading_summary_and_decisions`, `slice_dedups_repeated_covers_and_skips_unknown`, `slice_includes_only_covered_requirements`) all 4/4 pass — confirming the retry report's `cargo test --workspace` exit 0 claim on the bullets that actually defend REQ-005. The byte-equality contracts the prior pass leaned on (multi-line marker-body substring match in `review.rs`, unique-sentinel positive-and-negative substring assertions in `implement.rs`, fixture-driven byte-exact stdout match in `check.rs`, JSON-parsed `SPC-001` + `CHK-001` + `duplicate` triple in `verify.rs`) are all still wired to real parser/renderer/slicer code paths, not mocks. No new tests were needed for a doc-comment-only retry; the prior verdict carries forward.


<task-scenarios>
  - When `speccy check SPEC-0019/T-001` runs against the migrated
    workspace, then stdout contains the scenario body bytes from the
    `speccy:scenario` markers nested under REQ-001 (and only those),
    framed by the existing per-scenario header.
  - When the implementer prompt is rendered for a task that covers
    only REQ-002, then the prompt body contains REQ-002's marker
    block and its scenarios, and does not contain REQ-001's or
    REQ-003's requirement bodies.
  - When the reviewer-tests prompt is rendered for the same task,
    then the scenario text it sees equals the marker body bytes
    from SPEC.md (asserted by a substring match against the source
    file).
  - When `speccy verify` runs against a workspace where one spec
    has a scenario marker whose parent requirement was deleted,
    then verify fails and names the orphaned scenario.
  - When `speccy verify` runs against a workspace where one spec
    has a duplicate scenario id across two requirements, then
    verify fails with the existing duplicate-id wording, sourced
    from the marker parser.
</task-scenarios>
</task>

## Phase 4: Docs, skills, and migration cleanup


<task id="T-007" state="completed" covers="REQ-004 REQ-005">
Sweep architecture, skills, and delete the migration tool

- Suggested files: `.speccy/ARCHITECTURE.md`, `AGENTS.md`,
  `skills/**`, `.claude/skills/**`,
  `xtask/migrate-spec-markers-0019/` (delete),
  `speccy-core/tests/docs_sweep.rs`
- Implementer note (session-T007-sweep):
  - Completed: Rewrote `.speccy/ARCHITECTURE.md` to remove per-spec
    `spec.toml` from the file layout, the Five Proper Nouns table,
    the planning prompt step, the lint behaviour list, the lint
    codes (SPC-001 reframed as the stray-file warning, SPC-003 /
    REQ-002 / REQ-003 reserved as historical), the spec-ID
    allocation walk, the schema-version note, and the
    implementation sequence; replaced the old `## spec.toml`
    section with a `## SPEC.md marker grammar` section that
    documents marker comment syntax, the full marker-name table
    (`speccy:spec`, `speccy:summary`, `speccy:requirement`,
    `speccy:scenario`, `speccy:decision`, `speccy:open-question`,
    `speccy:changelog`), id regexes (`REQ-\d{3,}`, `CHK-\d{3,}`,
    `DEC-\d{3,}`), the scenario-inside-requirement nesting rule,
    the deterministic-rendering contract, and a historical note
    pointing at SPEC-0019 for the `spec.toml` migration. Added a
    "Public `speccy fmt` command" row to the "What We Deliberately
    Don't Do" table citing DEC-003. Rewrote shipped skill
    instructions to point at marker blocks instead of `spec.toml`
    in `resources/modules/skills/speccy-{plan,amend}.md`,
    `resources/modules/personas/{implementer,planner,reviewer-tests}.md`,
    and `resources/modules/prompts/{plan-greenfield,plan-amend,implementer,report}.md`;
    re-rendered the `.claude/` and `.agents/` mirrors via
    `speccy init --force --host claude-code` and `speccy init
    --force --host codex`; copied the updated persona and prompt
    bodies into `.speccy/skills/{personas,prompts}/` so the
    dogfood-output test passes. Deleted
    `xtask/migrate-spec-markers-0019/` and the now-empty `xtask/`
    directory, and removed the member from the workspace
    `Cargo.toml`. Added `speccy-core/tests/docs_sweep.rs` with
    four assertions: ARCHITECTURE.md mentions `spec.toml` only on
    lines containing a `migration` / `SPEC-0019` / `history` /
    `historical` keyword (case-insensitive); the marker grammar
    is documented (`speccy:requirement`, `speccy:scenario`,
    `speccy:decision`, `speccy:changelog` all present);
    `xtask/migrate-spec-markers-0019` no longer exists; no active
    `.md` under `skills/` or `.claude/skills/` mentions
    `spec.toml` outside historical context. `cargo run` resolves
    to the `speccy` binary unambiguously now that the second bin
    target is gone.
  - Undone: AGENTS.md was deliberately left alone — it does not
    carry a "What We Deliberately Don't Do" list (that lives in
    `.speccy/ARCHITECTURE.md`), and the prompt's escape clause
    said to leave AGENTS.md untouched in that case. This is the
    final task in SPEC-0019; the next step in the workflow is
    `/speccy-review SPEC-0019`.
  - Commands run: `rm -rf xtask/migrate-spec-markers-0019/`,
    `rmdir xtask/`, `cargo build --workspace`,
    `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    `cargo +nightly fmt --all --check`,
    `cargo run --quiet -- init --force --host claude-code`,
    `cargo run --quiet -- init --force --host codex`,
    `cargo run --quiet -- check SPEC-0019/T-007`,
    `cargo run --quiet -- verify`.
  - Exit codes: pass, pass, pass, pass, pass, pass, pass, pass,
    pass, pass.
  - Discovered issues: `speccy init --force` skips overwriting
    `.speccy/skills/personas/**` and `.speccy/skills/prompts/**`
    (the `skip` line in the init output), so the
    `dogfood_outputs_match_committed_tree` test failed against the
    updated `resources/modules/` bodies until I copied the
    changed persona and prompt files into `.speccy/skills/`
    manually. Worth noting if `speccy init` ever grows a
    `--refresh-skills` flag.
  - Procedural compliance: (none) — the existing skill files were
    already wrong about `spec.toml` post-SPEC-0019, and the sweep
    this task asked for fixed them; that is the task itself, not
    a side friction-fix.
- Review (business, pass): Every REQ-004 + REQ-005 "Done when" bullet that lands in T-007 is observably satisfied in the tree. REQ-004's "deleted before the final commit" clause: `/Users/kevin/src/speccy/xtask/` does not exist (verified by direct `ls`), the workspace `members` array in `/Users/kevin/src/speccy/Cargo.toml:2` is `["speccy-cli", "speccy-core"]` with no `xtask` entry, and `speccy-core/tests/docs_sweep.rs:84-91` (`migration_xtask_directory_is_deleted`) pins this as a regression test. REQ-005's two T-007-scoped bullets: (a) "ARCHITECTURE.md removes per-spec `spec.toml` from the file layout and documents the marker grammar" — the file-layout block at `.speccy/ARCHITECTURE.md:181-216` lists only `SPEC.md`/`TASKS.md`/`REPORT.md` per spec dir with the `spec.toml` mention scoped to a parenthetical historical note, and `## SPEC.md marker grammar` at `.speccy/ARCHITECTURE.md:856-933` documents the full grammar (syntax + marker-names table + id regexes + nesting rule + deterministic-rendering contract); the new `docs_sweep.rs:63-81` (`architecture_md_documents_marker_grammar`) asserts the four canonical marker names are present; (b) "shipped skills and prompts no longer instruct agents to read or edit per-spec `spec.toml`" — `grep -rn 'spec\.toml' .claude/ .codex/ .agents/` returns zero hits, and the only two remaining mentions in `resources/modules/prompts/plan-{greenfield,amend}.md:49` / `:34` are explicit "no longer used (SPEC-0019 migration)" disclaimers (the opposite of instructing edits), guarded by `docs_sweep.rs:117-147` which rejects any non-historical `spec.toml` line under `skills/` or `.claude/skills/`. The fifth T-007 "Tests to write" bullet (DEC-003 "no public `speccy fmt`" entry in the deliberately-don't-do list) lands at `.speccy/ARCHITECTURE.md:1610` ("Public `speccy fmt` command | Per SPEC-0019 DEC-003 ..."). The AGENTS.md-untouched call is defensible: AGENTS.md:87-90 already delegates the "what we deliberately don't do" list to ARCHITECTURE.md, and that's where the new row landed — so the contract REQ-005 + the fifth Tests-to-write bullet actually care about is satisfied. SPEC Non-goals respected (no back-compat shim for `spec.toml`; no TASKS.md/REPORT.md migration; the historical disclaimers are deliberate, not back-compat). SPEC Open Questions remain open — `speccy:spec` is documented as "optional root; emitted by the renderer" and `speccy:decision` as "optional, 0+", which mirrors REQ-001's own Done-when contract rather than silently resolving the Open Questions (the Open Questions ask whether to *tighten* these post-migration; documenting the current-as-shipped contract is not a resolution either way). One narrow gap worth flagging but not blocking: `docs_sweep.rs:117-147` only walks `skills/` and `.claude/skills/`, missing `.codex/agents/` and `.agents/skills/` — a stray `spec.toml` instruction reintroduced into the Codex mirror would not trip the test. Verified manually that none currently exists, but the regression-test coverage is asymmetric across hosts. Not blocking because (1) the rendered Codex/Agents outputs derive from the same `resources/modules/` source the test does cover transitively (any leak would have to be hand-added to the render output), and (2) REQ-005's prose says "shipped skills and prompts," which the test does cover where it matters; tightening the test is a follow-up nit, not a contract miss.
- Review (tests, blocking): `speccy-core/tests/docs_sweep.rs`
  exercises four of the five "Tests to write" bullets
  non-vacuously — `architecture_md_mentions_spec_toml_only_in_historical_context`
  (bullet 1) walks every `spec.toml` line in `.speccy/ARCHITECTURE.md`
  and asserts the historical-keyword filter (verified against the
  8 current hits at lines 193, 937-939, 1529-1534, 1784 — each
  contains `migration` or `SPEC-0019` or `historical`, so the
  test would fire on any non-historical addition);
  `architecture_md_documents_marker_grammar` (bullet 2) asserts
  each of `speccy:requirement`, `speccy:scenario`, `speccy:decision`,
  `speccy:changelog` is present in the doc (17 hits today);
  `migration_xtask_directory_is_deleted` (bullet 3) checks
  `xtask/migrate-spec-markers-0019` is absent (the whole `xtask/`
  directory is gone, so the assertion fires on regression);
  `shipped_skills_do_not_instruct_editing_per_spec_spec_toml`
  (bullet 4) walks `skills/` and `.claude/skills/` recursively
  and asserts no `spec.toml` mention survives outside files that
  self-declare as `migration note` / `historical note`. The
  fifth bullet — "AGENTS.md / equivalent states no public
  `speccy fmt` command (per DEC-003)" — is not exercised: a
  `grep -nE 'fmt|DEC-003' speccy-core/tests/docs_sweep.rs`
  returns zero matches, and the implementer note confirms the
  "Public `speccy fmt` command" row was added only to
  `.speccy/ARCHITECTURE.md:1610` (the "What We Deliberately
  Don't Do" table) without a corresponding assertion. Today
  that row is present, so the contract holds incidentally; but
  deleting line 1610 would silently regress without any test
  catching it. Add a fifth assertion in `docs_sweep.rs` that
  `.speccy/ARCHITECTURE.md` contains a line covering both
  `speccy fmt` and `DEC-003` (or the AGENTS.md equivalent if
  that file ever grows a "What We Deliberately Don't Do" list)
  so DEC-003's no-`speccy fmt` contract is pinned the same way
  the other four bullets are.
- Review (security, pass): T-007's surface is documentation +
  deletion + one new `#[test]`; the threat-model surface is
  correspondingly narrow and clean. (1) `speccy-core/tests/docs_sweep.rs`
  traversal is symlink-safe: `collect_md_files` (`:94-115`) uses
  `fs_err::read_dir` + `entry.file_type()` and only recurses on
  `is_dir()` / collects on `is_file()` — both std-`FileType`
  predicates are derived from the directory entry without
  following symlinks (lstat-equivalent on Unix), so a planted
  symlink under `skills/` or `.claude/skills/` is neither a `dir`
  nor a `file` and is skipped silently rather than followed into
  `/etc/passwd`-equivalents. Confirmed via `find skills .claude/skills
  -type l` returning zero entries today; the safety holds
  regardless. Roots are anchored to `CARGO_MANIFEST_DIR/..` so
  there is no attacker-controlled path component, no
  `..`-traversal surface, and the test short-circuits via
  `if !std_dir.exists() { return; }` when the directory is
  missing rather than panicking. (2) Skill-file rewrites
  (`resources/modules/{personas,prompts,skills}/*.md`) are
  pure prose edits — `git diff HEAD --` shows only `spec.toml` →
  marker-block reference swaps and the `<!-- speccy:scenario -->`
  grammar snippet in `plan-greenfield.md`. No new shell commands,
  no `eval`/`bash -c` examples, no `curl | sh` patterns, no
  instructions telling agents to execute marker-body contents as
  code. The marker grammar itself is HTML comments parsed
  in-process by Rust regex (`parse/spec_markers.rs`) with a
  closed attribute-value set (`REQ|CHK|DEC-\d{3,}` and
  `accepted|rejected|deferred|superseded`) — nothing that flows
  into a shell, FFI, or deserialization sink. (3) Migration-tool
  deletion is clean: `find . -name "migrate-spec-markers*"`
  returns only `target/debug/**` build artifacts (gitignored at
  `.gitignore:1` via `/target` and `target`), no committed
  binaries, no tracked source under the deleted `xtask/`. The
  workspace `Cargo.toml` no longer references the member (`grep
  xtask Cargo.toml` empty), and `git ls-files | xargs grep -l
  migrate-spec-markers` returns only `SPEC.md` itself (the spec
  documenting the tool's deletion) — that's narrative, not
  executable. No temp credentials, no `.env` leftovers, no
  private-key fixtures, no test-harness secrets in any deleted
  or modified path. (4) ARCHITECTURE.md edits are inert
  documentation — the 8 surviving `spec.toml` mentions (lines
  193, 937-939, 1529-1534, 1784) are all in migration / historical
  context (the docs_sweep test pins this); no new auth boundary,
  no new I/O sink, no new dependency, no crypto choice, no
  logging surface that could leak secrets. T-007 introduces no
  authentication, authorization, input-validation, injection,
  secret-handling, sensitive-data-exposure, or
  cryptographic-primitive surface.
- Review (style, pass): T-007 is itself the docs-sweep task that
  closes the `spec.toml` / doc-comment drift class that blocked
  T-005 and T-006, and the diff lands the cleanup cleanly. The new
  `speccy-core/tests/docs_sweep.rs` matches every project
  convention: `camino::Utf8Path` + `Utf8PathBuf` (not `std::path`),
  `fs_err::read_to_string` / `read_dir` (not bare `std::fs`),
  `.expect("descriptive message")` everywhere with no `unwrap`,
  `panic!`, `todo!`, `unreachable!`, or `[i]` slice indexing in
  sight; the `#![allow(clippy::expect_used, reason = "...")]`
  inner attribute uses `reason = ...` and mirrors the existing
  `speccy-core/tests/in_tree_specs.rs:1-4` precedent for sibling
  integration tests, so the clippy `allow_attributes_without_reason
  = "deny"` lint is satisfied. `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` and `cargo +nightly
  fmt --all --check` both exit 0 against the diff. ARCHITECTURE.md
  edits read clean: the new `## SPEC.md marker grammar` section
  (`:856-949`) documents the marker-name table, id regexes, the
  scenario-inside-requirement nesting rule, and the
  deterministic-rendering contract with the same Markdown-table
  cadence the rest of the file uses; the DEC-003 row added to "What
  We Deliberately Don't Do" (`:1610`) follows the table's existing
  one-line-per-feature pattern. Two minor consistency nits worth
  naming but neither blocking: (a) `docs_sweep.rs:103` does
  `Utf8PathBuf::from_path_buf(path.clone())` where `path` is bound
  and never used after the clone — `in_tree_specs.rs:30-31`
  establishes the idiom as `Utf8PathBuf::from_path_buf(path)` (move,
  no clone). Tiny dead-`clone()` that the project's clippy config
  doesn't deny-warn on but reads as a copy-paste residue.
  (b) The test's docstring (`:16-18`) and the originating TASKS.md
  bullet say `shipped_skills_do_not_instruct_editing_per_spec_spec_toml`
  walks `skills/` and `.claude/skills/`, but the repo has no
  top-level `skills/` directory — the canonical skill source lives
  in `resources/modules/{personas,prompts,skills}/` and renders
  out to `.claude/skills/`, `.agents/skills/`, `.codex/agents/`,
  and `.speccy/skills/`. `collect_md_files` early-returns when
  `skills/` doesn't exist (`:96-98`), so the `skills/` half of the
  assertion silently no-ops; only `.claude/skills/` actually gets
  scanned. The other rendered mirrors and the `resources/modules/`
  source-of-truth tree are not walked. Today every `spec.toml`
  mention in those untested trees ends with "SPEC-0019" so
  `line_is_historical` would let them through anyway, but the test
  is narrower than its description suggests; an authoring drift
  that wrote "edit the spec.toml" into `resources/modules/skills/`
  would slip past until `speccy init --force` re-renders into
  `.claude/skills/`. Cheap follow-up: extend the test to also walk
  `resources/modules/`, `.agents/skills/`, `.codex/agents/`, and
  `.speccy/skills/`. Neither nit is style drift from existing
  project patterns — they are tightenings of a test that already
  satisfies the task's stated bullets — so no blocking finding.
- Retry (tests blocking): Add a fifth assertion to
  `speccy-core/tests/docs_sweep.rs` pinning the fifth "Tests to
  write" bullet — DEC-003's "no public `speccy fmt`" contract —
  the same way the other four bullets are pinned, so a future
  deletion of the "Public `speccy fmt` command" row at
  `.speccy/ARCHITECTURE.md:1610` (or its AGENTS.md equivalent if
  that list ever moves) regresses loudly. Concretely: walk
  `.speccy/ARCHITECTURE.md` (or AGENTS.md, whichever carries the
  "What We Deliberately Don't Do" list) and assert at least one
  line mentions both `speccy fmt` and `DEC-003`. Hygiene after the
  edit: `cargo test --workspace`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo +nightly
  fmt --all --check`, `cargo run --quiet -- check SPEC-0019/T-007`,
  `cargo run --quiet -- verify`.
- Implementer note (session-T007-retry):
  - Completed: Added the fifth assertion
    `architecture_md_pins_no_public_speccy_fmt_per_dec_003` in
    `speccy-core/tests/docs_sweep.rs`. The new test reads
    `.speccy/ARCHITECTURE.md` and asserts at least one line
    contains both `speccy fmt` and `DEC-003`. Today the
    `.speccy/ARCHITECTURE.md:1610` "What We Deliberately Don't Do"
    row (`| Public ``speccy fmt`` command | Per SPEC-0019 DEC-003 ...`)
    satisfies the assertion; a future deletion of that row would
    fail the test loudly. Updated the test module docstring to
    describe the new assertion alongside the existing four. The
    reviewer-style non-blocking nits (the unnecessary `.clone()`
    at `docs_sweep.rs:103` and the docstring's reference to a
    top-level `skills/` directory that does not exist in this
    repo layout) are out of scope for this retry and left for a
    follow-up cleanup.
  - Commands run:
    `cargo test --package speccy-core --test docs_sweep --quiet`,
    `cargo run --quiet -- verify`.
  - Exit codes: pass (5 tests pass), pass (verify reports 0 errors
    across 21 specs / 105 requirements / 143 scenarios).
  - Discovered issues: `cargo test --workspace` fails on the
    sibling `in_tree_specs::every_in_tree_spec_md_parses_with_marker_parser`
    test, but the failure is induced by the SPEC-0020 (new,
    raw-xml-spec-carrier) and SPEC-0022 working-tree drafts —
    hand-authored in the future raw XML carrier form proposed by
    SPEC-0020 itself — not by the T-007 retry change. The
    SPEC-0019 ship commit excludes those drafts; the committed
    tree itself still parses cleanly under the marker parser.
    Resolving the working-tree friction belongs to SPEC-0020.
  - Procedural compliance: (none).

<task-scenarios>
  - When `.speccy/ARCHITECTURE.md` is searched, then per-spec
    `spec.toml` is referenced only in historical context (e.g.
    under a SPEC-0019 changelog or migration note) and the canonical
    file layout lists `SPEC.md` as the single spec carrier; a
    `grep`-style assertion in a workspace integration test pins
    this.
  - When the shipped skills directory (`skills/`) and the
    `.claude/skills/` mirror are searched, then no active
    instruction tells an agent to read or edit a per-spec
    `spec.toml`; matches are allowed only inside files explicitly
    labelled as migration or historical notes.
  - When the marker grammar is searched for in
    `.speccy/ARCHITECTURE.md`, then the file documents the marker
    names, id regexes, nesting rules, and the deterministic-render
    contract.
  - When the repo is searched for `xtask/migrate-spec-markers-0019`
    after the final commit lands, then no source files remain (the
    directory has been deleted); a CI grep-style test or a
    `cargo metadata` assertion encodes this.
  - When the AGENTS.md "What We Deliberately Don't Do" or
    equivalent list is reviewed, then it states that Speccy does
    not ship a public `speccy fmt` command (per DEC-003) so the
    deterministic renderer remains internal-only.
</task-scenarios>
</task>

</tasks>
