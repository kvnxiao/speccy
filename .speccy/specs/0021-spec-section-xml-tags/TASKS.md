---
spec: SPEC-0021
spec_hash_at_generation: b5df62baca46b2804a8af0abf6f71fad725473b49a3222cbe9ff260f1d29e199
generated_at: 2026-05-16T23:45:21Z
---

# Tasks: SPEC-0021 Section-level XML element tags for SPEC.md

## Phase 1: Parser, typed model, and renderer

<tasks spec="SPEC-0021">

<task id="T-001" state="completed" covers="REQ-001 REQ-002 REQ-005">
Whitelist expansion, typed model fields, and parse-side validation

- Suggested files: `speccy-core/src/parse/spec_xml/mod.rs`,
  `speccy-core/src/parse/spec_xml/html5_names.rs`,
  `speccy-core/src/error.rs`,
  `speccy-core/tests/fixtures/spec_xml/`

<task-scenarios>
  - When `parse` runs on a SPEC.md whose `<requirement>` body
    contains a `<done-when>` block immediately followed by a
    `<behavior>` block immediately followed by a `<scenario>` block,
    then it returns a `Requirement` whose `done_when` and `behavior`
    fields hold the verbatim Markdown bodies and whose scenarios
    vector still parses normally.
  - When a `<requirement>` contains a `<behavior>` block but no
    `<done-when>` block, then parsing fails with a diagnostic that
    names the requirement id and the missing `done-when` element.
  - When a `<requirement>` contains a `<done-when>` block but no
    `<behavior>` block, then parsing fails with a diagnostic that
    names the requirement id and the missing `behavior` element.
  - When a `<requirement>` contains `<behavior>` before
    `<done-when>` (reverse order), then parsing fails with an
    ordering diagnostic naming the requirement id, because DEC-002
    fixes the order at the structural level.
  - When a `<requirement>` contains a `<scenario>` open tag before
    `<done-when>` or `<behavior>`, then parsing fails with an
    ordering diagnostic naming the requirement id.
  - When a `<requirement>` contains two `<done-when>` blocks (or
    two `<behavior>` blocks), then parsing fails with a
    duplicate-element diagnostic that names the offending element
    and the requirement id.
  - When `parse` runs on a SPEC.md whose top level contains
    `<goals>`, `<non-goals>`, and `<user-stories>` exactly once,
    then it returns a `SpecDoc` whose `goals`, `non_goals`, and
    `user_stories` fields hold the verbatim Markdown bodies.
  - When a SPEC.md's top level lacks `<goals>` (or `<non-goals>`,
    or `<user-stories>`), then parsing fails with a
    missing-required-section diagnostic that names the missing
    element.
  - When a SPEC.md's top level contains two `<goals>` elements (or
    two `<non-goals>`, or two `<user-stories>`, or two
    `<assumptions>`), then parsing fails with a duplicate-section
    diagnostic that names the offending element.
  - When a SPEC.md's top level contains no `<assumptions>`
    element, then parsing succeeds and `SpecDoc.assumptions` is
    `None`.
  - When a SPEC.md's top level contains exactly one `<assumptions>`
    element, then parsing succeeds and `SpecDoc.assumptions` is
    `Some(body)` with the verbatim Markdown.
  - When a SPEC.md's top level contains a `<spec>` or `<overview>`
    element, then parsing fails with a diagnostic whose `Display`
    message names the element and notes it was retired in
    SPEC-0021.
  - When Markdown body content contains the literal text
    `<behavior>`, `<done-when>`, `<goals>`, `<non-goals>`,
    `<user-stories>`, or `<assumptions>` inside a fenced code block
    or inline backticks on a structure-shaped line, then parsing
    preserves the bytes verbatim as body content and produces no
    structural element.
  - When the HTML5-disjointness unit test runs over the post-T-001
    whitelist, every name (`requirement`, `scenario`, `decision`,
    `open-question`, `changelog`, `behavior`, `done-when`, `goals`,
    `non-goals`, `user-stories`, `assumptions`) is asserted absent
    from the checked-in HTML5 element name set, and the retired
    names (`spec`, `overview`) are no longer present in the
    whitelist constant the test reads from.
  - When source code references `SpecDoc.overview`,
    `SpecDoc.overview_span`, or any `root_element`-style field, no
    such field exists in the typed model after this task
    (`grep`-style test or compile-time deletion suffices).
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-002">
Renderer canonical order, blank-line spacing, and round-trip

- Suggested files: `speccy-core/src/parse/spec_xml/mod.rs`,
  `speccy-core/tests/spec_xml_roundtrip.rs`,
  `speccy-core/tests/fixtures/spec_xml/canonical.md`


<task-scenarios>
  - When `render(&SpecDoc)` runs on a `SpecDoc` parsed from a hand-
    authored canonical fixture exercising all six new section tags,
    then re-parsing the rendered string yields a `SpecDoc` whose
    `goals`, `non_goals`, `user_stories`, `assumptions`,
    `requirements`, `decisions`, `open_questions`, and `changelog`
    fields each equal the original field-by-field (not via `Debug`
    string).
  - When `render` runs on a `SpecDoc`, then the emitted top-level
    element order is `<goals>` → `<non-goals>` → `<user-stories>` →
    requirements → decisions → open-questions → `<assumptions>`
    (only when `Some`) → `<changelog>`, with no `<spec>` root and
    no `<overview>` element anywhere.
  - When `render` runs on a `Requirement`, then the emitted
    element order inside the requirement is `<done-when>` →
    `<behavior>` → nested `<scenario>` blocks, with exactly one
    blank line between `</done-when>` and `<behavior>`, one between
    `</behavior>` and the first `<scenario>`, and one between
    sibling `</scenario>` and `<scenario>` boundaries.
  - When `render` runs on a `SpecDoc` whose `assumptions` field is
    `None`, then the rendered file contains no `<assumptions>` open
    or close tag.
  - When `render` runs on a `SpecDoc` whose `assumptions` field is
    `Some(body)`, then `<assumptions>` appears exactly once
    immediately before `<changelog>`, separated by one blank line.
  - When `render` runs twice on the same `SpecDoc`, then the two
    outputs are byte-identical (idempotence).
  - When `render` runs on a `SpecDoc` whose `Requirement.behavior`
    body contains `<` / `>` / `&` / a fenced Markdown code block /
    a literal `<scenario>` token inside backticks, then those bytes
    pass through verbatim and the re-parser does not promote them to
    structure.
</task-scenarios>
</task>

## Phase 2: Migration tool and in-tree corpus rewrite


<task id="T-003" state="completed" covers="REQ-003">
Build `xtask/migrate-spec-sections-0021`

- Suggested files: `xtask/migrate-spec-sections-0021/Cargo.toml`,
  `xtask/migrate-spec-sections-0021/src/main.rs`,
  `xtask/migrate-spec-sections-0021/src/lib.rs`,
  `xtask/migrate-spec-sections-0021/tests/fixtures/`,
  workspace root `Cargo.toml` (add the workspace member)

<task-scenarios>
  - When the migration runs on a fixture SPEC.md whose top level
    has `## Goals`, `## Non-goals`, and `## User Stories` Markdown
    sections, then the rewritten file wraps each section's body
    with the matching `<goals>` / `<non-goals>` / `<user-stories>`
    open/close tag pair while preserving the H2 heading line above
    each.
  - When the migration runs on a fixture SPEC.md with a `##
    Assumptions` section, then the rewritten file wraps that
    section's body with `<assumptions>` ... `</assumptions>`.
  - When the migration runs on a fixture SPEC.md with no `##
    Assumptions` section, then the rewritten file emits no
    `<assumptions>` element and migration exits zero.
  - When the migration runs on a `<requirement>` block whose body
    contains a `**Done when:**` paragraph followed by a
    `**Behavior:**` paragraph followed by a `<scenario>` block,
    then the rewritten requirement contains `<done-when>` and
    `<behavior>` elements (in that order) wrapping those exact
    paragraph bodies, positioned before the nested `<scenario>`.
  - When the migration runs on a `<requirement>` lacking a
    `**Behavior:**` paragraph, then migration fails with a
    diagnostic that names the requirement id and the missing prose
    block, and emits no rewritten file.
  - When the migration runs on a `<requirement>` lacking a
    `**Done when:**` paragraph, then migration fails with a
    diagnostic that names the requirement id and the missing prose
    block.
  - When the migration runs on a fixture SPEC.md containing a
    literal `<spec>` or `<overview>` element, then the rewritten
    file does not contain those tags (and the tool emits an
    informational note naming the stripped element per the open
    question's lean-yes-warn resolution).
  - When the migration runs on a fixture SPEC.md, then the
    frontmatter block, the level-1 heading line, and every
    narrative section (`## Design`, `## Migration / Rollback`,
    `## Notes`, etc.) plus their body content are byte-identical
    between input and output.
  - When the migration runs on a fixture SPEC.md, then re-parsing
    the rewritten file with the post-T-001/T-002 SPEC-0021 parser
    succeeds and the resulting `SpecDoc` carries populated `goals`,
    `non_goals`, `user_stories`, and per-requirement `done_when` /
    `behavior` fields.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-003">
Apply migration across `.speccy/specs/*/SPEC.md` and dogfood `speccy verify`

- Suggested files: `.speccy/specs/0001-artifact-parsers/SPEC.md`
  through `.speccy/specs/0022-xml-canonical-tasks-report/SPEC.md`,
  `speccy-core/tests/in_tree_specs.rs`


<task-scenarios>
  - When the migration tool has run across every
    `.speccy/specs/*/SPEC.md` in the workspace, then each file
    parses successfully with the post-T-001/T-002 SPEC-0021 parser
    (asserted by `speccy-core/tests/in_tree_specs.rs` or an
    equivalent in-tree-corpus integration test).
  - When `speccy verify` runs against the post-migration workspace,
    then it exits zero with no diagnostics attributable to
    SPEC-0021's parser or proof shape.
  - When the in-tree corpus integration test reads each migrated
    SPEC.md, then `goals`, `non_goals`, and `user_stories` are
    `Some(_)` (or non-empty) and every requirement has populated
    `done_when` and `behavior` fields.
  - When `git diff` is taken between the pre-migration and
    post-migration in-tree SPEC.md files, then the diff only
    adds/removes element tag lines (no prose changes inside intent
    sections, no frontmatter changes, no heading text changes).
</task-scenarios>
</task>

## Phase 3: Docs, prompts, skill packs, and cleanup


<task id="T-005" state="completed" covers="REQ-004">
Sweep ARCHITECTURE.md, prompts, and shipped skill packs

- Suggested files: `.speccy/ARCHITECTURE.md`,
  `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/plan-greenfield.md`,
  `resources/modules/prompts/tasks-generate.md`,
  `.codex/agents/reviewer-tests.toml`,
  `.codex/agents/reviewer-business.toml`,
  `.claude/skills/`, `.agents/`, `.codex/`,
  `resources/agents/.agents/`, `resources/agents/.codex/`

<task-scenarios>
  - When `.speccy/ARCHITECTURE.md` is read after this task, then
    the SPEC.md element-names table contains rows for `behavior`,
    `done-when`, `goals`, `non-goals`, `user-stories`, and
    `assumptions` (each with cardinality and location columns) and
    contains no rows for `spec` or `overview`.
  - When the SPEC.md template inside `.speccy/ARCHITECTURE.md` is
    read after this task, then it uses `<done-when>` and
    `<behavior>` wrappers inside each `<requirement>` example and
    `<goals>`, `<non-goals>`, `<user-stories>` (and optional
    `<assumptions>`) wrappers at the top level — not
    `**Done when:**` / `**Behavior:**` Markdown-bold conventions.
  - When `resources/modules/prompts/implementer.md` is read after
    this task, then it cites `<behavior>` and `<done-when>` by
    name when telling the implementer what drives acceptance and
    test selection.
  - When `resources/modules/prompts/reviewer-tests.md` is read
    after this task, then it cites `<behavior>` and `<scenario>`
    by name when telling the persona what to compare.
  - When `resources/modules/prompts/reviewer-business.md` is read
    after this task, then it cites `<goals>` and `<non-goals>` by
    name.
  - When shipped skill packs under `.claude/skills/`, `.agents/`,
    `.codex/`, `resources/agents/.agents/`, and
    `resources/agents/.codex/` are read after this task, then any
    reference to SPEC.md structure mentions the new XML element
    tags rather than `**Behavior:**` / `**Done when:**` /
    `## Goals` Markdown conventions.
  - When a grep for the literal strings `**Behavior:**` or
    `**Done when:**` runs across active (non-historical) guidance
    after this task, then any hits are confined to
    migration-context documentation (e.g. a migration note that
    explains what the pre-SPEC-0021 form looked like). Active
    prompts and ARCHITECTURE.md contain zero such hits.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-003">
Delete the ephemeral migration tool

- Suggested files: `xtask/migrate-spec-sections-0021/` (delete),
  workspace root `Cargo.toml` (remove the workspace member entry)

<task-scenarios>
  - When the final implementation commit lands, then
    `xtask/migrate-spec-sections-0021/` no longer exists on disk.
  - When the workspace root `Cargo.toml` is read after this task,
    then it lists no `migrate-spec-sections-0021` workspace
    member.
  - When `cargo build --workspace`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    and `cargo +nightly fmt --all --check` all run after the
    migration tool is removed, then all four exit zero.
</task-scenarios>
</task>

</tasks>
