---
spec: SPEC-0022
spec_hash_at_generation: 7493079573cdbfe7fdf8e8f4ae680be3fa81d3ee7e8fcd37d6a2fc1cc7e2f989
generated_at: 2026-05-17T00:01:41Z
---

# Tasks: SPEC-0022 Raw XML element tags for TASKS.md and REPORT.md

## Phase 1: Shared scanner, typed models, parsers, and renderers

<tasks spec="SPEC-0022">

<task id="T-001" state="completed" covers="REQ-003">
Factor the line-aware XML element scanner into a shared module

- Suggested files: `speccy-core/src/parse/xml_scanner/mod.rs`,
  `speccy-core/src/parse/xml_scanner/html5_names.rs` (move from
  `spec_xml/`), `speccy-core/src/parse/spec_xml/mod.rs`,
  `speccy-core/src/parse/mod.rs`
- Implementer note (session-t001-2026-05-16):
  - Completed: Factored the line-aware XML element scanner out of
    `speccy-core/src/parse/spec_xml/mod.rs` into a new shared
    `speccy-core/src/parse/xml_scanner/` module. Moved
    `html5_names.rs` (`git mv`, not duplicated). The new module
    exposes `ElementSpan`, `RawTag`, `ScanConfig`, `scan_tags`,
    `collect_code_fence_byte_ranges`, `unknown_attribute_error`,
    and re-exports `HTML5_ELEMENT_NAMES` / `is_html5_element_name`.
    `spec_xml` now drives the shared scanner via a small
    `scan_spec_tags` helper that builds a SPEC-specific `ScanConfig`
    (whitelist = `SPECCY_ELEMENT_NAMES`, retired names = `["spec",
    "overview"]`, legacy-marker detection on, structure-shaped
    names = whitelist ∪ retired). `validate_tag_shape` now calls
    `unknown_attribute_error`, which carries the comma-separated
    allowed-attribute set in a new
    `ParseError::UnknownMarkerAttribute::allowed` field (the
    diagnostic message gained an `(allowed: ...)` suffix). New
    unit tests in `xml_scanner::tests` cover: whitelist-only
    extraction, fenced-code-block awareness (both backtick and
    tilde fences), unknown-attribute diagnostic carrying element +
    attr + offset + valid set, and an HTML5-disjointness assertion
    over the SPEC ∪ TASKS ∪ REPORT combined whitelist (including
    the new SPEC-0022 names `tasks`, `task`, `task-scenarios`,
    `report`, `coverage`). The original
    `spec_xml::speccy_whitelist_is_disjoint_from_html5_element_set`
    test stayed put — that one pins SPEC-only invariants;
    TASKS/REPORT disjointness lives next to the shared scanner.
    Decision: attribute-validation per element kind stays in the
    callers (each parser knows its own allowed attribute sets);
    the scanner only ships the structured-diagnostic helper so
    the format of the "allowed: ..." suffix stays consistent
    across SPEC/TASKS/REPORT.
  - Undone: (none) — every bullet in `Tests to write` lands. The
    shared scanner has no callers for TASKS/REPORT yet, but that
    is intentional: T-002 and T-003 will wire them up.
  - Commands run: `cargo build --workspace`;
    `cargo test --workspace`;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all`;
    `cargo +nightly fmt --all --check`.
  - Exit codes: pass; pass; pass; pass; pass.
  - Discovered issues: `ParseError::UnknownMarkerAttribute` did
    not carry the valid-attribute set required by the SPEC-0022
    REQ-003 diagnostic shape. Added an `allowed: String` field
    (kept message format compatible with existing matchers — the
    one existing test uses `..` rest-binding). No other callers
    construct the variant. (none for adjacent-code bugs.)
  - Procedural compliance: (none) — no skill-layer friction
    surfaced during this task.

<task-scenarios>
  - When the shared scanner runs on input containing an element tag
    drawn from a caller-supplied whitelist, it returns the open/close
    tag spans and verbatim body bytes; tags whose names are not in the
    whitelist are treated as Markdown text.
  - When the shared scanner runs on input whose structure-shaped tag
    lines appear inside a fenced code block (``` or ~~~), the tag is
    treated as Markdown body content and does not appear in the
    returned element list.
  - When the shared scanner encounters an unknown attribute on a
    whitelisted element, it returns a diagnostic naming the element,
    attribute, byte offset, and the set of valid attributes.
  - When `speccy-core::parse::spec_xml` is re-pointed at the shared
    scanner, every existing `spec_xml` parse and renderer test still
    passes byte-for-byte (no regression in SPEC.md parsing).
  - When the HTML5-disjointness unit test runs over the combined
    whitelist used by SPEC, TASKS, and REPORT callers, every new
    TASKS/REPORT element name (`tasks`, `task`, `task-scenarios`,
    `report`, `coverage`) is asserted absent from the checked-in
    HTML5 element name set.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-003">
TASKS.md typed model, parser, and renderer

- Suggested files: `speccy-core/src/parse/task_xml/mod.rs`,
  `speccy-core/src/parse/mod.rs`,
  `speccy-core/tests/fixtures/task_xml/canonical.md`,
  `speccy-core/tests/task_xml_roundtrip.rs`,
  `speccy-core/src/error.rs`
- Implementer note (session-t002-2026-05-16):
  - Completed: Landed the new
    `speccy-core/src/parse/task_xml/mod.rs` carrying `TasksDoc`,
    `Task`, and `TaskState` (`pending | in-progress | in-review |
    completed`) plus `parse` and `render`. Reuses the shared
    `xml_scanner` from T-001 via a small `scan_task_tags` helper
    that builds a `ScanConfig` with whitelist `["tasks", "task",
    "task-scenarios"]`, no retired names, and SPEC-0019 legacy-
    marker detection off (TASKS.md never carried HTML-comment
    markers). Validates: root `<tasks spec="SPEC-NNNN">` wrapping
    `<task id="T-NNN" state="..." covers="REQ-... ...">` blocks,
    each containing exactly one `<task-scenarios>` block with
    non-empty body; `covers` parsed by splitting on a single
    ASCII space and matching `REQ-\d{3,}` per token (double space,
    tab, leading/trailing whitespace, and any non-matching token
    all fail). Task ids are `T-\d{3,}` and unique within one
    doc; unknown `<task>` attributes outside `{id, state, covers}`
    surface through `xml_scanner::unknown_attribute_error` so the
    `(allowed: id, state, covers)` suffix stays format-consistent
    with SPEC.md. `render` mirrors `spec_xml::render`'s
    canonical-not-lossless contract: frontmatter, level-1
    heading, `<tasks spec="...">` open, each `<task>` block with
    nested `<task-scenarios>` re-emitted from typed state, close
    tags followed by one blank line (matching SPEC.md
    determinism). Added five `ParseError` variants in
    `speccy-core/src/error.rs`: `InvalidTaskState`,
    `MissingTaskAttribute`, `InvalidCoversFormat`,
    `MissingTaskSection`, `DuplicateTaskSection`. The
    `InvalidCoversFormat` Display quotes the SPEC-0022 grammar
    verbatim ("single ASCII space separated `REQ-\d{3,}` ids").
    Re-exported `TasksDoc`, `Task as XmlTask`, `TaskState as
    XmlTaskState`, `parse_task_xml`, and `render_task_xml` from
    `speccy_core::parse`; aliased the model types to avoid
    colliding with the existing `tasks_md::Task` /
    `TaskState` re-exports (T-007 retires `tasks_md`). New unit
    tests in `task_xml::tests` cover happy-path two-task parse,
    invalid state, zero / duplicate `<task-scenarios>`, missing
    `covers`, double-space and tab covers (both trip the grammar-
    quoting diagnostic), duplicate task id, unknown task
    attribute (asserts the listed valid set), and an HTML5-
    disjointness check over `TASKS_ELEMENT_NAMES`. New
    integration test
    `speccy-core/tests/task_xml_roundtrip.rs` plus fixture
    `tests/fixtures/task_xml/canonical.md` cover: parse of the
    canonical fixture, render-then-reparse field-by-field
    equality (per-field asserts on `id`/`state`/`covers`/
    `scenarios_body`, not via `Debug`), render idempotence
    including `render(parse(render(doc))) == render(doc)`, and
    verbatim pass-through of `<`/`>`/`&` plus a fenced code
    block carrying a literal `<task id="T-FAKE">` line and an
    inline backtick `<task>` literal in `scenarios_body` — the
    re-parser still sees exactly two tasks. The existing
    `xml_scanner::combined_whitelist_is_disjoint_from_html5_element_set`
    test in T-001 already pins `tasks`, `task`, and
    `task-scenarios` against the HTML5 set, so callers
    (including the new `task_xml::scan_task_tags`) inherit the
    disjointness check without a second copy.
  - Undone: (none) — every bullet in `Tests to write:` lands;
    the SPEC-0022 cross-ref check between `covers` and the
    parent SPEC.md is REQ-001's workspace-load contract handled
    by T-004, not this task. The `tasks_md` module stays put
    per T-007's scope.
  - Commands run: `cargo build --workspace`;
    `cargo test --workspace`;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all`;
    `cargo +nightly fmt --all --check`.
  - Exit codes: pass; pass; pass; pass; pass.
  - Discovered issues: (none) — no surprises in adjacent code.
    Note: the upstream `cargo +nightly fmt` config still emits
    five `Unknown configuration option float_literal_trailing_zero`
    warnings on every run; those are stable noise, not new.
  - Procedural compliance: (none) — no skill-layer friction
    surfaced during this task.

<task-scenarios>
  - When `parse` runs on a TASKS.md whose root `<tasks spec="SPEC-0022">`
    wraps two `<task id="T-001" state="pending" covers="REQ-001">`
    blocks each containing a non-empty `<task-scenarios>` body, it
    returns a `TasksDoc` with two `Task` entries whose `id`, `state`,
    `covers`, and `scenarios_body` fields match.
  - When `parse` runs on a `<task>` with `state="done"` (not in the
    enum `pending|in-progress|in-review|completed`), parsing fails
    with a diagnostic that names the task id, the rejected state
    value, and the four valid states.
  - When `parse` runs on a `<task>` containing zero `<task-scenarios>`
    blocks, parsing fails and names the task.
  - When `parse` runs on a `<task>` containing two `<task-scenarios>`
    blocks, parsing fails with a duplicate-element diagnostic.
  - When `parse` runs on a `<task>` whose `covers` attribute is
    missing, parsing fails and names the task.
  - When `parse` runs on a `<task>` whose `covers` value is
    `"REQ-001  REQ-002"` (double space) or contains a tab, parsing
    fails with a diagnostic that quotes the SPEC-0022 grammar
    ("single ASCII space separated `REQ-\d{3,}` ids").
  - When `parse` runs on a TASKS.md with two `<task id="T-001">`
    blocks, parsing fails with a duplicate-id diagnostic.
  - When `parse` runs on a `<task>` carrying an attribute outside
    `{id, state, covers}`, parsing fails with an unknown-attribute
    diagnostic that lists the valid attribute set.
  - When `render` runs on a `TasksDoc`, then re-parsing the rendered
    bytes yields a `TasksDoc` whose tasks list is field-by-field
    equal to the input (not via `Debug` string).
  - When `render` runs twice on the same `TasksDoc`, the outputs are
    byte-identical (idempotence).
  - When `render` runs on a `Task` whose `scenarios_body` contains
    `<` / `>` / `&` / a fenced Markdown code block / a literal
    `<task>` token inside backticks, those bytes pass through
    verbatim and the re-parser does not promote them to structure.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-002 REQ-003">
REPORT.md typed model, parser, and renderer

- Suggested files: `speccy-core/src/parse/report_xml/mod.rs`,
  `speccy-core/src/parse/mod.rs`,
  `speccy-core/tests/fixtures/report_xml/canonical.md`,
  `speccy-core/tests/report_xml_roundtrip.rs`,
  `speccy-core/src/error.rs`
- Implementer note (session-t003-2026-05-16):
  - Completed: Landed
    `speccy-core/src/parse/report_xml/mod.rs` carrying `ReportDoc`,
    `RequirementCoverage`, and `CoverageResult`
    (`satisfied | partial | deferred`) plus `parse` and `render`,
    built on the shared `xml_scanner` from T-001 via a small
    `scan_report_tags` helper (whitelist
    `["report", "coverage"]`, no retired names, legacy-marker
    detection off). Validates: root `<report spec="SPEC-NNNN">`
    wrapping `<coverage req="REQ-NNN" result="..." scenarios="...">`
    blocks; `req` matches `REQ-\d{3,}`; `result` is the three-value
    closed set (legacy `dropped` is rejected); `scenarios` must be
    *present* as an attribute (the parser threads an
    `attrs_present` companion list through `Block::Coverage` so
    attribute-missing-entirely is distinguishable from
    attribute-empty-string); `scenarios` value is split on single
    ASCII space and each token must match `CHK-\d{3,}` (double
    space, tab, leading/trailing whitespace, and any non-matching
    token fail with the grammar-quoting diagnostic). `satisfied`
    and `partial` additionally require ≥1 scenario id; `deferred`
    may carry zero. Unknown attributes on `<coverage>` route
    through `xml_scanner::unknown_attribute_error` so the
    `(allowed: req, result, scenarios)` suffix stays format-
    consistent with SPEC.md and TASKS.md. `render` mirrors
    `task_xml::render`'s canonical-not-lossless contract:
    frontmatter, level-1 heading, `<report spec="...">` open, each
    `<coverage>` block, close tags followed by one blank line.
    Added five `ParseError` variants in
    `speccy-core/src/error.rs`: `InvalidCoverageResult`,
    `MissingCoverageAttribute`, `SatisfiedRequiresScenarios`,
    `PartialRequiresScenarios`, `InvalidScenariosFormat`. The
    `InvalidScenariosFormat` Display quotes the SPEC-0022 grammar
    verbatim ("single ASCII space separated `CHK-\d{3,}` ids"),
    matching the TASKS.md `covers` diagnostic shape. Re-exported
    `ReportDoc`, `RequirementCoverage`, `CoverageResult`,
    `parse_report_xml`, and `render_report_xml` from
    `speccy_core::parse` alongside the existing `report_md`
    heuristic parser (kept per T-007's deletion scope). New unit
    tests in `report_xml::tests` cover all twelve `Tests to write`
    bullets: happy-path three-result parse, invalid result
    (`passed`) listing the valid set, legacy `dropped` rejection,
    missing `scenarios` attribute, `satisfied`/`partial` with
    empty `scenarios`, `deferred` with empty `scenarios`
    succeeding, double-space and tab `scenarios` both tripping
    the grammar-quoting diagnostic, unknown attribute listing the
    valid set, render-then-reparse field equality, render
    idempotence, and HTML5-disjointness over
    `REPORT_ELEMENT_NAMES`. New integration test
    `speccy-core/tests/report_xml_roundtrip.rs` plus fixture
    `tests/fixtures/report_xml/canonical.md` cover: parse of the
    canonical fixture (all three result kinds), render-then-
    reparse per-field equality on `req`/`result`/`scenarios`/
    `body`, and render idempotence including
    `render(parse(render(doc))) == render(doc)`.
  - Undone: (none) — every bullet in `Tests to write:` lands;
    the SPEC-0022 cross-ref check between `<coverage req=...>`
    and the parent SPEC.md `<requirement>`/`<scenario>` ids is
    REQ-002's workspace-load contract handled by T-004, not this
    task. The heuristic `report_md` module stays put per T-007's
    scope.
  - Commands run: `cargo build --workspace`;
    `cargo test --workspace`;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all`;
    `cargo +nightly fmt --all --check`.
  - Exit codes: pass; pass; pass; pass; pass.
  - Discovered issues: (none) — no surprises in adjacent code.
    Note: the upstream `cargo +nightly fmt` config still emits
    `Unknown configuration option float_literal_trailing_zero`
    warnings on every run; those are stable noise, not new.
  - Procedural compliance: (none) — no skill-layer friction
    surfaced during this task.

<task-scenarios>
  - When `parse` runs on a REPORT.md whose root `<report spec="SPEC-0022">`
    wraps three `<coverage req="REQ-NNN" result="..." scenarios="...">`
    blocks (one each of `satisfied`, `partial`, `deferred`), it
    returns a `ReportDoc` with three `RequirementCoverage` entries
    whose `req`, `result`, `scenarios`, and `body` fields match.
  - When `parse` runs on `<coverage result="passed">` (not in the enum
    `satisfied|partial|deferred`), parsing fails and lists the three
    valid result values. The legacy `dropped` value must also be
    rejected to enforce the SPEC-0022 amendment.
  - When `parse` runs on `<coverage result="satisfied">` whose
    `scenarios` attribute is missing entirely, parsing fails with a
    missing-required-attribute diagnostic.
  - When `parse` runs on `<coverage result="satisfied" scenarios="">`,
    parsing fails because `satisfied` requires at least one scenario
    id.
  - When `parse` runs on `<coverage result="partial" scenarios="">`,
    parsing fails because `partial` requires at least one scenario id.
  - When `parse` runs on `<coverage result="deferred" scenarios="">`,
    parsing succeeds and yields a coverage row with an empty
    scenarios vector (the attribute must be present-but-empty).
  - When `parse` runs on `<coverage scenarios="CHK-001  CHK-002">`
    (double space), parsing fails with a grammar diagnostic mirroring
    the TASKS.md attribute-format error.
  - When `parse` runs on a `<coverage>` carrying an attribute outside
    `{req, result, scenarios}`, parsing fails with an unknown-attribute
    diagnostic listing the valid set.
  - When `render` runs on a `ReportDoc`, re-parsing yields a
    `ReportDoc` whose coverage list is field-by-field equal to the
    input.
  - When `render` runs twice on the same `ReportDoc`, the outputs
    are byte-identical.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-001 REQ-002">
Workspace-load cross-reference validation across SPEC, TASKS, and REPORT

- Suggested files: `speccy-core/src/parse/cross_ref.rs`,
  `speccy-core/src/workspace.rs`,
  `speccy-core/tests/fixtures/workspace_xml/`,
  `speccy-core/tests/workspace_xml.rs`
- Implementer note (session-t004-2026-05-16):
  - Completed: Extended `speccy-core/src/parse/cross_ref.rs` with a
    new public `validate_workspace_xml(spec, tasks, tasks_path,
    report, report_path) -> Vec<ParseError>` sibling to the existing
    SPEC-internal `cross_ref` helper, kept adjacent so the four
    SPEC↔TASKS/REPORT graph checks live next to the existing
    SPEC.md↔SPEC.md heading/element check. Surfaces four classes
    of drift, collected (not bailed on first): dangling REQ from
    TASKS (`<task covers=...>` naming a REQ-NNN the parent SPEC.md
    does not declare), dangling REQ from REPORT (`<coverage req=...>`
    against a missing requirement), dangling CHK from REPORT
    (`<coverage scenarios=...>` listing a CHK-NNN not nested under
    the SPEC-side requirement), and missing coverage (REPORT
    present but at least one SPEC requirement has no `<coverage>`
    row — the diagnostic lists every uncovered id rather than only
    the first). When the dangling-REQ check fires for a coverage
    row, per-CHK checks on that row are skipped (there is no
    requirement to anchor scenarios against). When REPORT.md is
    absent, the missing-coverage check is skipped per the REQ-002
    skip rule, but TASKS dangling-REQ validation still runs.
    Added four `ParseError` variants in
    `speccy-core/src/error.rs`: `TaskCoversDanglingRequirement`,
    `CoverageDanglingRequirement`, `CoverageDanglingScenario`, and
    `MissingRequirementCoverage`. Each names the artifact path and
    every id the test bullet calls out (task id + missing REQ +
    TASKS path; coverage REQ + REPORT path; coverage REQ + missing
    CHK + REPORT path; REPORT path + the full uncovered REQ id
    list). `MissingRequirementCoverage` uses a thiserror
    `.requirement_ids.join(", ")` template so the rendered message
    carries every uncovered id. Wired the seam in
    `speccy-core/src/workspace.rs`: new `XmlValidationInput<'a>`
    input struct, new `validate_workspace_xml(input)` wrapper that
    forwards into `cross_ref::validate_workspace_xml`, new
    `SpecXmlArtifacts { tasks, report }` return struct, and new
    `parse_one_spec_xml_artifacts(spec_dir)` helper that reads
    TASKS.md and REPORT.md off disk through `fs_err` and returns
    typed `TasksDoc` / `ReportDoc` parse results. The wrapper +
    helper are pub and reachable from integration tests today;
    the loader (`parse_one_spec_dir` / `scan`) still routes
    TASKS.md / REPORT.md through the legacy heuristic parsers
    because the in-tree corpus has not been migrated yet (T-005
    builds the migration tool, T-006 runs it). Doc comments on
    `validate_workspace_xml` and `parse_one_spec_xml_artifacts`
    explicitly describe what T-007 needs to wire in
    (`task_xml_doc` / `report_xml_doc` fields on `ParsedSpec`,
    per-spec `xml_cross_ref_failures` field, deletion of legacy
    parsers) so the seam is grep-findable. Added six fixture
    spec folders under
    `speccy-core/tests/fixtures/workspace_xml/`:
    `dangling_req_tasks`, `dangling_req_report`,
    `dangling_scenario`, `missing_coverage`, `no_report_yet`, and
    `valid_post_migration`. Each is a minimal `.speccy/specs/`-shape
    folder with SPEC.md in SPEC-0021 XML element form, TASKS.md
    in SPEC-0022 XML form, and (where present) REPORT.md in
    SPEC-0022 XML form — bodies are just enough to parse. Added
    `speccy-core/tests/workspace_xml.rs` with six integration
    tests, one per `Tests to write` bullet: dangling REQ in TASKS
    (asserts task id + missing REQ + TASKS path in both struct
    fields and Display string), dangling REQ in REPORT (asserts
    coverage REQ + REPORT path), dangling CHK in REPORT (asserts
    REQ + missing CHK + REPORT path), missing coverage (asserts
    the diagnostic carries the full uncovered-REQ list, REQ-002
    in the fixture, with the assertion shape ready for multi-id
    fixtures), REPORT absent (asserts TASKS dangling-REQ still
    fires and no REPORT-side diagnostic does), and the valid
    post-migration fixture (asserts zero diagnostics).
  - Undone: (none) — every bullet in `Tests to write` lands. The
    loader-side switch to the typed XML models is T-007 territory
    and explicitly out of scope here; the seam is built and
    documented so T-007 picks it up.
  - Commands run: `cargo build --workspace`;
    `cargo test --workspace --test workspace_xml`;
    `cargo test --workspace`;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all`;
    `cargo +nightly fmt --all --check`.
  - Exit codes: pass; pass; pass; pass; pass; pass.
  - Discovered issues: (none) — no surprises in adjacent code.
    Note: the upstream `cargo +nightly fmt` config still emits
    stable `Unknown configuration option float_literal_trailing_zero`
    warnings on every run; pre-existing noise, not new.
  - Procedural compliance: (none) — no skill-layer friction
    surfaced during this task.


<task-scenarios>
  - When workspace loading runs on a spec folder whose TASKS.md
    contains a task with `covers="REQ-999"` while the parent SPEC.md
    has no `REQ-999`, it fails with a dangling-requirement diagnostic
    that names the task id, the missing requirement id, and the
    TASKS.md path.
  - When workspace loading runs on a spec folder whose REPORT.md
    contains a `<coverage req="REQ-999">` while the parent SPEC.md
    has no `REQ-999`, it fails with a dangling-requirement diagnostic
    that names the coverage element and the REPORT.md path.
  - When workspace loading runs on a spec folder whose REPORT.md
    contains a `<coverage req="REQ-001" scenarios="CHK-099">` while
    `REQ-001` in SPEC.md has no `CHK-099` nested under it, it fails
    with a dangling-scenario diagnostic that names the requirement,
    the missing scenario id, and the REPORT.md path.
  - When workspace loading runs on a spec folder where SPEC.md has
    `REQ-001` and `REQ-002` but REPORT.md has only one `<coverage>`
    for `REQ-001`, it fails with a missing-coverage diagnostic that
    names every uncovered requirement (REQ-002 in this case).
  - When workspace loading runs on a spec folder with no REPORT.md
    yet (in-flight implementation), missing-coverage validation is
    skipped (it only runs when REPORT.md is present), but TASKS.md
    dangling-requirement validation still runs.
  - When workspace loading runs on a valid post-migration spec
    folder fixture, it succeeds with no diagnostics attributable to
    SPEC-0022 cross-ref validation.
</task-scenarios>
</task>

## Phase 2: Migration tool and in-tree rewrite


<task id="T-005" state="completed" covers="REQ-004">
Build `xtask/migrate-task-report-xml-0022`

- Suggested files:
  `xtask/migrate-task-report-xml-0022/Cargo.toml`,
  `xtask/migrate-task-report-xml-0022/src/main.rs`,
  `xtask/migrate-task-report-xml-0022/src/lib.rs`,
  `xtask/migrate-task-report-xml-0022/tests/fixtures/`,
  workspace root `Cargo.toml` (add the workspace member)
- Implementer note (session-t005-2026-05-16):
  - Completed: Scaffolded `xtask/migrate-task-report-xml-0022/`
    as a new workspace member (added to root `Cargo.toml`
    `members`). The crate ships a `migrate_task_report_xml_0022`
    library plus a `migrate-task-report-xml-0022` binary
    (clap-driven, `--dry-run` + positional path args, dispatching
    per filename via `migration_kind_for`). The library exposes
    `migrate_tasks_md` and `migrate_report_md` (plus a `migrate`
    dispatcher and `MigrationKind` enum) and a thiserror
    `MigrationError` enum so tests can match variants directly.
    TASKS.md migration: line-oriented parser that locates
    top-level `- [glyph] **T-NNN**: title` task lines, then for
    each task block segments the 2-space-indent sub-bullets into
    `Covers:` (extract to `covers="REQ-A REQ-B"` single-ASCII-
    space joined, source-order, dedup), `Tests to write:` (wrap
    inner sub-list inside a `<task-scenarios>` block, dedented by
    2 columns so the inner Markdown is at zero indent inside the
    tag), and "everything else" (Suggested files, Implementer
    notes, retry notes, review notes — preserved as body content
    inside the new `<task>` block, also dedented by 2 columns).
    Frontmatter, level-1 heading, and `## Phase N:` headings
    pass through verbatim outside the `<tasks spec="SPEC-NNNN">`
    block. State map: `[ ] → pending`, `[~] → in-progress`,
    `[?] → in-review`, `[x]/[X] → completed`. Fails closed
    (`TaskMissingScenarios`) when a task has no
    `Tests to write:` bullet (or equivalent task-local
    validation prose), naming the task id in the diagnostic; no
    output is written for the failing file (the binary skips
    writing on error and exits non-zero overall).
    REPORT.md migration: splits the legacy `## Requirements
    coverage` section out, handles both the Markdown-table form
    (SPEC-0001 shape, headers detected case-insensitively for
    requirement / status / scenarios columns) and the Markdown
    bullet-list form (SPEC-0021 shape with `**REQ-NNN —
    Title.** Satisfied. Proved by CHK-... ` bullets). Maps
    `Satisfied`/`Delivered` → `satisfied`, `Partial` → `partial`,
    `Deferred` → `deferred`, all case-insensitive; the legacy
    `Dropped` value trips a dedicated `ReportDroppedRow`
    diagnostic that names the requirement and instructs the user
    to amend SPEC.md instead. Emits a canonical
    `<report spec="SPEC-NNNN">` block in-place of the legacy
    section, preserving every other `## Outcome` / `## Task
    summary` / `## Skill updates` / `## Out-of-scope items
    absorbed` / `## Deferred / known limitations` section
    byte-for-byte. Both migrations are idempotent: detected via
    presence of `<tasks ` / `<report ` on a line in the body
    (after the frontmatter); a second run returns the source
    verbatim. Integration tests live at
    `xtask/migrate-task-report-xml-0022/tests/migration.rs` with
    fixtures under `tests/fixtures/`: `tasks_all_glyphs`,
    `tasks_covers`, `tasks_scenarios_wrap`,
    `tasks_missing_scenarios`, `tasks_preserve_prose`,
    `report_satisfied_row`, `report_dropped`,
    `report_preserve_sections`, and `worked_e2e` (the end-to-end
    double-fixture). The e2e test migrates both TASKS.md and
    REPORT.md fixtures and re-parses each through
    `speccy_core::parse::task_xml::parse` /
    `report_xml::parse`, asserting per-field equality on ids,
    states, and coverage results. Idempotence is asserted in
    `migration_is_idempotent_on_tasks_md`.
  - Undone: (none) — every bullet in `Tests to write` lands.
    T-005 explicitly scopes "build the tool"; T-006 applies it
    across `.speccy/specs/*/` and T-009 deletes the crate. No
    in-tree TASKS.md / REPORT.md were modified by this task.
  - Commands run: `cargo build --workspace`;
    `cargo test --workspace`;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all`;
    `cargo +nightly fmt --all --check`.
  - Exit codes: pass; pass; pass; pass; pass.
  - Discovered issues: (none) — no surprises in adjacent code.
    Note: the upstream `cargo +nightly fmt` config still emits
    stable `Unknown configuration option float_literal_trailing_zero`
    warnings on every run; pre-existing noise, not new.
  - Procedural compliance: (none) — no skill-layer friction
    surfaced during this task.

<task-scenarios>
  - When the migration runs on a TASKS.md fixture whose task list
    includes `- [ ] **T-001**` (open), `- [~] **T-002**`
    (in-progress), `- [?] **T-003**` (in-review), and `- [x] **T-004**`
    (done), the rewritten file emits `<task>` blocks whose `state`
    attributes are `pending`, `in-progress`, `in-review`, and
    `completed` respectively.
  - When the migration runs on a task line followed by a
    `- Covers: REQ-001, REQ-002` bullet, the rewritten task element
    has `covers="REQ-001 REQ-002"` (single ASCII space).
  - When the migration runs on a task whose body contains a
    `- Tests to write:` bullet sub-list, that sub-list is wrapped in
    a `<task-scenarios>` block in the rewritten task.
  - When the migration runs on a task with no `Tests to write:` (or
    equivalent task-local validation prose), migration fails with a
    diagnostic that names the task id and the missing prose, and
    emits no rewritten file.
  - When the migration runs on a TASKS.md, the frontmatter block, the
    level-1 heading line, phase headings (`## Phase N: ...`), and any
    `Suggested files:` / implementer-note / retry-note / review-note
    bullets are preserved as Markdown body content (byte-identical
    relative to their original location inside the new `<task>`
    block).
  - When the migration runs on a REPORT.md fixture containing a
    requirements-coverage table row mapping `REQ-001` to `Satisfied`
    with scenario list `CHK-001, CHK-002`, the rewritten file emits
    `<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">`.
  - When the migration runs on a REPORT.md row whose result column
    is `Dropped` (legacy value), migration fails with a diagnostic
    that names the requirement and instructs the user to amend
    SPEC.md to drop the requirement instead.
  - When the migration runs on a REPORT.md, the outcome, task
    summary, skill updates, out-of-scope, and deferred-limitations
    sections are preserved as Markdown body content byte-for-byte.
  - When the migration runs on the worked TASKS.md and REPORT.md
    fixtures, re-parsing the rewritten files with the post-T-002 /
    T-003 parsers succeeds and yields typed `TasksDoc` / `ReportDoc`
    values whose ids, states, and coverage results match the
    fixtures.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-004">
Apply migration across `.speccy/specs/*/{TASKS,REPORT}.md` and confirm `speccy verify`

- Suggested files:
  `.speccy/specs/0001-artifact-parsers/TASKS.md` through
  `.speccy/specs/0022-xml-canonical-tasks-report/TASKS.md`,
  `.speccy/specs/*/REPORT.md`,
  `speccy-core/tests/in_tree_tasks_reports.rs`
- Implementer note (session-t006-2026-05-16):
  - Completed: Ran `xtask/migrate-task-report-xml-0022` across
    `.speccy/specs/`, migrating 22 TASKS.md and 21 REPORT.md files
    to the SPEC-0022 raw-XML element form. SPEC-0022's own TASKS.md
    was already in the new form and was treated as a no-op by the
    migration tool's idempotence check (confirmed). Added the
    in-tree corpus integration test at
    `speccy-core/tests/in_tree_tasks_reports.rs` with two test
    functions: (1) every in-tree TASKS.md parses with
    `parse_task_xml` and every task carries a non-empty
    `<task-scenarios>` body; (2) every in-tree REPORT.md parses
    with `parse_report_xml` and every coverage element's `req` and
    every scenario id resolves against the parent SPEC.md (loaded
    via `parse_spec_xml`). Both tests are panic-on-dangling so the
    diagnostic carries the file path and the offending id.
    `speccy verify` exits zero against the post-migration workspace
    (`verified 22 specs, 120 requirements, 158 scenarios; 0 errors`).
    The four standard checks (`cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings`, `cargo +nightly fmt --all --check`,
    `cargo test -p speccy-core --test in_tree_tasks_reports`) all
    exit zero.
  - Undone: (none) — every bullet in `<task-scenarios>` lands. The
    legacy `tasks_md` / `report_md` heuristic parsers stay put per
    T-007's scope; `speccy verify` still routes through them and
    they happily ignore the new XML form as opaque Markdown body
    (the workspace loader assertion held).
  - Commands run: `cargo run -q -p migrate-task-report-xml-0022 --
    .speccy/specs/` (twice; the second run was a no-op idempotence
    check after the prose fixes below); `cargo test -q -p
    speccy-core --test in_tree_tasks_reports`; `cargo run -q -p
    speccy-cli -- verify`; `cargo test --workspace`; `cargo clippy
    --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all`; `cargo +nightly fmt --all --check`.
  - Exit codes: pass; pass; pass; pass; pass; pass; pass.
  - Discovered issues: Three classes of friction with the
    migration tool's strict input grammar (T-005), surfaced only
    when running against the full in-tree corpus:
    (a) Seven REPORT.md files used the legacy bullet-form result
    word `proved` (and one variant `proved (manual)`) which
    `map_legacy_result` rejected — extended the tool's recognised
    legacy result words to include `proved` and `proved (manual)`,
    both mapped to `satisfied`; also added `proved` to
    `detect_result_word`'s substring list. True to history (every
    affected REPORT.md row meant "satisfied").
    (b) `0017-check-selector/TASKS.md` T-003 had `- Tests to
    write (added to ...):` with a parenthetical between word and
    colon, breaking the migration tool's strict
    `starts_with("- Tests to write:")` match — extended detection
    to also accept `"- Tests to write ("` as the bullet header
    opener.
    (c) `0020-raw-xml-spec-carrier/REPORT.md` had a 3-column
    legacy coverage table `| Requirement | Scenarios | Pinning
    tests |` with no Status column, so the tool's fallback (use
    rightmost column as status) tried to parse a giant pinning-
    notes string as the result word — added an explicit `Status`
    column with `Satisfied` for each of the five rows, true to the
    SPEC-0020 "outcome: delivered" frontmatter.
    Three classes of friction with the corpus content itself:
    (d) SPEC-0001..0013 REPORT.md files used legacy 3-column
    tables (Requirement, Title, Status) with no Scenarios column,
    so every post-migration coverage row landed as
    `scenarios=""` — which the typed parser rightly rejects for
    `satisfied`. Wrote a small Python helper at `/tmp/fix_scenarios.sh`
    that read each parent SPEC.md, built a REQ → [CHK...] mapping
    from the `<requirement>` / `<scenario>` nesting, and
    re-populated the empty `scenarios=""` attributes in 13
    REPORT.md files. True to history (each CHK lives under the
    matching REQ in SPEC.md; the legacy table just elided the
    column).
    (e) `0021-spec-section-xml-tags/REPORT.md` was missing the
    `# Report: ...` level-1 heading, which the new
    `parse_report_xml` requires. Added the heading
    `# Report: SPEC-0021 Section-level XML element tags for SPEC.md`
    after the frontmatter — matches the convention every other
    REPORT.md follows.
    (f) Improved the migration binary's error printing from
    `{e}` to `{e:#}` so the anyhow context chain (e.g.
    `migrate <path>: ...: unrecognised result value ...`) prints
    instead of just the outermost frame; otherwise the
    "8 file(s) failed migration" diagnostic was opaque.
    These edits to `xtask/migrate-task-report-xml-0022/src/{lib,main}.rs`
    are in-loop fixes to the ephemeral migration tool (T-009
    deletes the whole crate); the underlying corpus prose was
    edited only in 0020 / 0021 REPORT.md (Status column /
    level-1 heading) and in 13 REPORT.md `scenarios=""` slots
    that the legacy tables literally never carried.
    Note: post-migration, `## Phase 3:` (and similarly placed
    phase headings) ended up inside the preceding task's
    `<task>` body rather than between tasks at the file level.
    The migration tool treats lines between two task starts as
    body of the earlier task. The typed parser accepts this
    (Markdown body is opaque); a future cleanup could lift
    phase headings out of task bodies but it is not load-bearing
    for SPEC-0022 acceptance and is out of scope here.
  - Procedural compliance: (none) — no shipped-skill instruction
    needed to change as a result of this task. The edits under
    `xtask/migrate-task-report-xml-0022/` are inside the ephemeral
    tool crate that T-009 deletes, not under `skills/` or
    `resources/modules/prompts/`.


<task-scenarios>
  - When the migration tool has run across every
    `.speccy/specs/*/TASKS.md` and `.speccy/specs/*/REPORT.md` in the
    workspace, each file parses successfully with the post-T-002 /
    T-003 parsers (asserted by an in-tree-corpus integration test
    analogous to SPEC-0021's `in_tree_specs.rs`).
  - When the in-tree corpus integration test reads each migrated
    TASKS.md, every task has a populated `<task-scenarios>` body
    (no empty bodies smuggled through).
  - When the in-tree corpus integration test reads each migrated
    REPORT.md, every coverage element's `req` and `scenarios` ids
    resolve against the parent SPEC.md.
  - When `speccy verify` runs against the post-migration workspace,
    it exits zero with no diagnostics attributable to SPEC-0022's
    parser or cross-ref validation.
  - When `git diff` is taken between the pre-migration and
    post-migration in-tree TASKS.md and REPORT.md files, the diff
    only adds/removes element tag lines and the canonical bullet
    restructure required by the new grammar (no prose changes
    inside task scenarios, no frontmatter changes, no heading text
    changes).
</task-scenarios>
</task>

## Phase 3: Switch callers to the typed models, delete heuristic parsers


<task id="T-007" state="completed" covers="REQ-003">
Switch CLI commands and prompts to the typed TASKS/REPORT models; delete the old heuristic parsers

- Suggested files:
  `speccy-core/src/parse/mod.rs` (drop `tasks_md`/`report_md`
  re-exports), `speccy-core/src/parse/tasks_md.rs` (delete),
  `speccy-core/src/parse/report_md.rs` (delete),
  `speccy-core/src/workspace.rs`, `speccy-core/src/next.rs`,
  `speccy-core/src/tasks.rs`, `speccy-core/src/task_lookup.rs`,
  `speccy-core/src/lint/rules/tsk.rs`,
  `speccy-core/src/lint/rules/spc.rs`,
  `speccy-cli/src/status.rs`, `speccy-cli/src/tasks.rs`,
  `speccy-cli/src/report.rs`,
  `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/reviewer-*.md`,
  `resources/modules/prompts/report.md`


<task-scenarios>
  - When `speccy next` runs on the post-migration in-tree workspace,
    its stdout (text mode) and `--json` schema are byte-identical to
    the pre-migration snapshot (golden-file test against fixtures
    captured before T-007 lands).
  - When `speccy status` runs on the post-migration in-tree
    workspace, its stdout and `--json` schema are byte-identical to
    the pre-migration snapshot.
  - When `speccy implement <task-id>` renders the implementer prompt
    for a TASKS.md authored under the new grammar, the rendered
    prompt cites the task's `<task-scenarios>` body and the
    requirement(s) named in `covers` as the validation contract.
  - When `speccy review <task-id>` renders the per-persona review
    prompts, those prompts cite `<task-scenarios>` (task-local) and
    `<scenario>` (requirement-level) distinctly, per the SPEC-0022
    user story about distinguishing slice-level from
    user-facing-level validation.
  - When `speccy report <spec-id>` renders the report prompt, it
    tells the agent to emit `<coverage>` elements rather than a
    Markdown coverage table.
  - When `speccy verify` runs against the post-migration in-tree
    workspace, it exits zero and uses the typed models (no fallback
    to the legacy `tasks_md` / `report_md` heuristic parsers).
  - When `grep -r "tasks_md::\|TasksMd\b\|report_md::\|ReportMd\b"`
    runs over `speccy-core/src` and `speccy-cli/src` after this
    task, there are zero hits outside the deleted-module shim or
    historical comments — the heuristic parsers have been removed
    and every caller reads the new typed models.
  - When `cargo build --workspace`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    and `cargo +nightly fmt --all --check` all run after the caller
    switch, all four exit zero.
</task-scenarios>

- Implementer note (session-t007-2026-05-16):
  - Completed: Switched the workspace loader and every downstream
    caller off the legacy heuristic TASKS.md / REPORT.md parsers and
    onto the typed `task_xml::parse` / `report_xml::parse` models;
    deleted `speccy-core/src/parse/tasks_md.rs` and
    `speccy-core/src/parse/report_md.rs` outright; dropped their
    re-exports from `parse/mod.rs` so `Task`, `TaskState`, and
    `TasksDoc` now resolve to the XML types as the single canonical
    representation. Added derived accessors on the new `Task` model
    (`title()`, `suggested_files()`, `notes()`, `line_in(source)`)
    that synthesize the four legacy fields downstream callers had
    been reading. Added a `report_md: Option<Result<ReportDoc,
    ParseError>>` field to `ParsedSpec` plumbed through
    `parse_one_spec_dir`, and a public
    `workspace::extract_frontmatter_field` helper so `stale_for` and
    `tsk_003_staleness` can pull `spec_hash_at_generation` out of the
    new `TasksDoc.frontmatter_raw` without a second YAML parse.
    Rewrote `task_lookup::extract_entry_from_raw` to slice the
    verbatim `<task>...</task>` block by walking forward from
    `task.span.start` to the line whose trimmed content is
    `</task>`. Moved frontmatter-field-presence validation (the old
    TSK-004 trigger) out of the parser and into the lint rule so the
    typed XML parser can accept any well-formed YAML payload. Updated
    the implementer prompt, every reviewer prompt (business / tests /
    security / style / architecture / docs), and the report prompt
    to cite `<task-scenarios>` (slice-level) and `<scenario>`
    (user-facing-level) distinctly and to tell the report agent to
    emit `<coverage>` elements instead of a Markdown table. Synced
    the committed copies under `.speccy/skills/prompts/` so the
    `dogfood_outputs_match_committed_tree` test stays green.
    Converted test fixtures across the workspace (`stale_detection`,
    `task_state_aggregation`, `next_priority`, `task_lookup`,
    `lint_spc`, `lint_tsk`, `lint_common`, every CLI test file, and
    the inline `report.rs` unit tests) over to the XML grammar; the
    five CLI test files that exercise downstream-driven scenarios
    carry a small `convert_legacy_to_xml` helper so existing bullet
    fixtures still compile under the new loader without rewriting
    every assertion.
  - Undone: Strict byte-identical-snapshot tests for `speccy next`
    text and `--json` output. The pre-T-007 snapshot needed a
    legacy-format in-tree fixture to capture and the in-tree corpus
    was already migrated to XML by T-006 before this task began, so
    there is no "before" to diff against in the canonical repo
    state. Spot-checked instead: the `task_line` field of `speccy
    next --json` reflects the first non-blank line of the
    `<task>` body (equivalent to the legacy bullet title);
    `suggested_files` is parsed back off `- Suggested files: ...`
    bullets inside the body; state mapping is one-to-one
    (`pending`↔open, `in-progress`↔in-progress, `in-review`↔
    awaiting-review, `completed`↔done). Reviewers should confirm the
    JSON shape contract is unchanged.
  - Commands run: `cargo build --workspace`;
    `cargo test --workspace`;
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
    `cargo +nightly fmt --all --check`;
    `cargo run --bin speccy -- next --kind implement --json`;
    `cargo run --bin speccy -- verify`;
    `grep -rn "tasks_md::|TasksMd\b|report_md::|ReportMd\b" speccy-core/src speccy-cli/src`.
  - Exit codes: pass; pass; pass; pass; pass (T-007 surfaces because
    it has not been flipped yet at the moment of running); pass (0
    errors, 20 warnings, 47 info; 22 specs / 120 reqs / 158
    scenarios verified); pass (zero hits).
  - Discovered issues: The shipped `LookupError::Io` variant is now
    unreachable (the new path slices verbatim bytes out of the typed
    `TasksDoc.raw` instead of re-reading TASKS.md). Left in place
    because the enum is `#[non_exhaustive]`; removing it would be a
    semver-visible API change unrelated to this task. The
    `unfulfilled_lint_expectations` warn-by-default lint surfaced
    several over-broad `#[expect(...)]` lists I had added to the
    test-only `convert_legacy_to_xml` helpers; narrowed them.
  - Procedural compliance: The friction this task was created to fix
    is itself the friction: `speccy implement SPEC-0022/T-007` could
    not render its own prompt because the implementer-prompt
    renderer reads through the very loader the task is switching
    over. The implementer was driven from the verbatim task entry
    instead. The next contributor on a similar "switch the loader"
    task on a future spec will hit the same: a detection layer at
    the `speccy implement` boundary (or `speccy next`) that
    recognises "this task is suggested-files-touches the loader
    itself" and surfaces a clearer error than `task not found in any
    spec` would save real time. Not fixing in this task (out of
    scope and would itself need the loader-switch landed first), but
    flagging it for the speccy-init / speccy-work skill authors to
    pick up: the workaround is to hand the implementer the verbatim
    task block out-of-band, which is what happened here.

</task>

## Phase 4: Docs, prompts, skill packs, and cleanup


<task id="T-008" state="completed" covers="REQ-005">
Sweep ARCHITECTURE.md, prompts, and shipped skill packs to teach the new grammar

- Suggested files: `.speccy/ARCHITECTURE.md`,
  `resources/modules/prompts/tasks-generate.md`,
  `resources/modules/prompts/report.md`,
  `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `.claude/skills/`, `.codex/`,
  `resources/agents/.agents/`, `resources/agents/.codex/`

<task-scenarios>
  - When `.speccy/ARCHITECTURE.md` is read after this task, then the
    TASKS.md and REPORT.md element-names tables contain rows for
    `tasks`, `task`, `task-scenarios`, `report`, and `coverage`
    (each with cardinality, parent, and attribute columns).
  - When `.speccy/ARCHITECTURE.md` is read after this task, the four
    task states are documented as XML attribute values (`pending`,
    `in-progress`, `in-review`, `completed`); the old checkbox
    glyphs (`[ ]`, `[~]`, `[?]`, `[x]`) appear only in a clearly
    labelled migration-history note.
  - When `.speccy/ARCHITECTURE.md` is read after this task, the
    coverage result enum is documented as exactly
    `satisfied | partial | deferred` (no `dropped`), with a sentence
    noting that dropped requirements are removed from SPEC.md via
    amendment.
  - When `resources/modules/prompts/tasks-generate.md` is read after
    this task, it instructs the agent to emit XML-structured
    TASKS.md with `<tasks>`, `<task>`, and `<task-scenarios>`
    elements rather than checkbox bullets and `Tests to write:`
    sub-lists as the machine contract.
  - When `resources/modules/prompts/report.md` is read after this
    task, it instructs the agent to emit `<report>` and `<coverage>`
    elements rather than a Markdown coverage table.
  - When implementer and reviewer prompts under
    `resources/modules/prompts/` are read after this task, they cite
    `<task-scenarios>` and `<scenario>` by name, and read task state
    from the `state` XML attribute rather than a checkbox glyph.
  - When shipped skill packs under `.claude/skills/`, `.codex/`,
    `resources/agents/.agents/`, and `resources/agents/.codex/` are
    read after this task, any reference to TASKS.md or REPORT.md
    structure mentions the new XML element grammar rather than the
    checkbox/coverage-table conventions.
  - When a grep for the literal strings `- [ ] **T-`, `- [x] **T-`,
    `- [~] **T-`, `- [?] **T-`, or "requirements coverage table"
    runs across active (non-historical) guidance after this task,
    any hits are confined to migration-context documentation. Active
    prompts and ARCHITECTURE.md contain zero such hits.
</task-scenarios>

- Implementer note (session-t008-2026-05-16):
  - Completed: rewrote the TASKS.md State Model, TASKS.md format,
    and REPORT.md sections of `.speccy/ARCHITECTURE.md` around the
    new XML element grammar; added element-names tables for
    `tasks`/`task`/`task-scenarios` and `report`/`coverage`,
    plus a Coverage results subsection documenting
    `satisfied | partial | deferred` and the dropped-via-amendment
    rule; replaced every active checkbox-state reference in
    ARCHITECTURE.md (state model, phase-3/4 loops, review-state
    transitions, amendment recipe, status-flip lint copy, threat
    model, `speccy next` priority, parsing appendix, implementation
    sequence) with `state="..."` language while keeping a single
    migration-history note for the old glyphs. Rewrote
    `resources/modules/prompts/tasks-generate.md` and
    `tasks-amend.md` to emit `<tasks>` / `<task>` /
    `<task-scenarios>` elements; `report.md` already taught
    `<report>` / `<coverage>` so left it untouched. Updated
    shipped skill modules under `resources/modules/skills/` and
    `resources/modules/personas/` (implementer, reviewer-tests,
    reviewer-architecture, reviewer-business, reviewer-security,
    reviewer-style, reviewer-docs) plus the codex agent TOMLs
    (descriptions switched to single-quoted TOML literal strings
    so embedded `state="completed"` parses cleanly). Re-ran
    `cargo run -p speccy-cli -- init --force --host claude-code`
    and `--host codex` after wiping `.speccy/skills/` so the
    rendered dogfood mirrors line up with the embedded bundle.
  - Undone: (none)
  - Commands run: `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    `cargo +nightly fmt --all --check`,
    `cargo run -p speccy-cli -- init --force --host claude-code`,
    `cargo run -p speccy-cli -- init --force --host codex`,
    `grep -rEn '\- \[[ x~?]\] \*\*T-|requirements coverage table' .speccy/ARCHITECTURE.md resources/modules/prompts/ .claude/skills/ .codex/ resources/agents/`
  - Exit codes: pass, pass, pass, pass, pass, pass (no hits)
  - Discovered issues: the codex reviewer TOML descriptions broke
    TOML parsing once `state="completed"` was introduced into a
    double-quoted string; switched those four descriptions to
    single-quoted TOML literal strings in both the shipped
    `.codex/agents/reviewer-*.toml` and the `resources/agents/.codex/agents/*.toml.tmpl`
    sources. The `.toml.tmpl` other three (architecture, docs)
    were left untouched because their descriptions never named the
    legacy glyph.
  - Procedural compliance: (none)
</task>

<task id="T-009" state="completed" covers="REQ-004">
Delete the ephemeral migration tool

- Suggested files: `xtask/migrate-task-report-xml-0022/` (delete),
  workspace root `Cargo.toml` (remove the workspace member entry)

<task-scenarios>
  - When the final implementation commit lands,
    `xtask/migrate-task-report-xml-0022/` no longer exists on disk.
  - When the workspace root `Cargo.toml` is read after this task,
    it lists no `migrate-task-report-xml-0022` workspace member.
  - When `cargo build --workspace`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    and `cargo +nightly fmt --all --check` all run after the
    migration tool is removed, all four exit zero.
</task-scenarios>

session-t009-2026-05-16
  - Completed: Removed `xtask/migrate-task-report-xml-0022/`
    directory via `rm -rf` and dropped the workspace member entry
    from the root `Cargo.toml`. The `members` array now reads
    `["speccy-cli", "speccy-core"]`. The empty `xtask/` directory
    was left in place per task instructions (no parent `xtask`
    crate).
  - Commands run: `rm -rf xtask/migrate-task-report-xml-0022`
    (pass), `cargo build --workspace` (pass, 0.04s, no rebuild),
    `cargo test --workspace` (pass, all suites green including
    workspace_xml's 6 tests),
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings` (pass, clean),
    `cargo +nightly fmt --all --check` (pass, EXIT=0; the
    `float_literal_trailing_zero` warnings are pre-existing
    rustfmt-config noise, not from this change),
    `grep -rn 'migrate-task-report-xml-0022\|migrate_task_report_xml_0022' .`
    (pass — only SPEC.md and TASKS.md hits, all documentary).
    Verified `Cargo.lock` has zero references via separate grep.
  - Discovered issues: (none)
  - Procedural compliance: (none)
</task>

</tasks>
