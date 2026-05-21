---
spec: SPEC-0019
spec_hash_at_generation: ce5d56b5b0c1b2a730d3a7e3ac0915616cbd7e25057c4e00894fd4924e27b9c0
generated_at: 2026-05-17T17:37:23Z
---

# Tasks: SPEC-0019 Canonical marker-structured SPEC.md

## Phase 1: Marker parser and Rust model


<task id="T-001" state="completed" covers="REQ-001 REQ-003">
Marker scanner, `SpecDoc` model, and strict parser

- Suggested files: `speccy-core/src/parse/spec_markers.rs`,
  `speccy-core/src/parse/mod.rs`, `speccy-core/src/error.rs`,
  `speccy-core/tests/fixtures/spec_markers/`

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
- Retry (style blocking): Surgical doc-only edits, no behavior or test changes:
  1. `speccy-core/src/prompt/mod.rs:3` — change "Six helpers, each isolated in its own submodule:" to "Seven helpers, each isolated in its own submodule:".
  2. `speccy-core/src/prompt/mod.rs:5-14` — add a bullet for `spec_slice` to the enumerated list, e.g. `- [`spec_slice`] — emit a task-scoped Markdown slice of a `SpecDoc` driven by the task's `Covers:` list (frontmatter + heading + summary + covered requirements with nested scenarios + decisions).`
  Hygiene after the edits: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo run --quiet -- check SPEC-0019/T-006`.

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

