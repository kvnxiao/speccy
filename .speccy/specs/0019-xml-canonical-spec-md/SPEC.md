---
id: SPEC-0019
slug: xml-canonical-spec-md
title: Canonical marker-structured SPEC.md; remove spec.toml
status: implemented
created: 2026-05-15
supersedes: []
---

# SPEC-0019: Canonical marker-structured SPEC.md

## Summary

After SPEC-0018, checks are English validation scenarios, but the
contract still lives in two places: `SPEC.md` carries the prose
requirements, while `spec.toml` carries the REQ-to-CHK graph and the
scenario text. That two-file shape creates exactly the drift Speccy is
supposed to make loud: a requirement can change without its check row
changing, a check can outlive the requirement it used to prove, and
agents must read both files to understand one behavior contract.

This spec collapses the carrier into one canonical `SPEC.md`.
Instead of putting raw XML trees around Markdown prose, the file uses
XML-style HTML comments as structural markers around normal Markdown:

```markdown
<!-- speccy:requirement id="REQ-001" -->
### REQ-001: Render selected scenarios

Plain Markdown prose remains plain Markdown.

<!-- speccy:scenario id="CHK-001" -->
Given a task covers REQ-001,
when `speccy check SPEC-0019/T-001` runs,
then only REQ-001's scenarios are rendered.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
```

The marker comments are the machine-readable structure. The prose
between markers is ordinary Markdown and remains readable on GitHub,
in terminals, and in editor previews. This avoids the brittle raw-XML
design where Markdown inside custom XML/HTML tags may stop rendering
as Markdown and arbitrary prose/code containing `<`, `>`, or `&` must
be escaped to remain valid XML.

Speccy's Rust model becomes the canonical representation. The parser
reads frontmatter, the level-1 heading, and the marker tree into typed
structs. A deterministic renderer writes the canonical file shape back
out. `spec.toml` is deleted; each `<scenario>` marker is linked to the
nearest containing `<requirement>` marker, so the containment relation
replaces the old `[[requirements]].checks` table.

This is the second step in the sequence:

- **SPEC-0018:** checks stop executing and become English scenarios.
- **SPEC-0019:** SPEC.md becomes the single canonical spec carrier
  using HTML-comment markers.
- **SPEC-0020:** SPEC.md switches from HTML-comment markers to raw
  XML element tags so vendor-recommended prompt structure applies.
- **SPEC-0021:** TASKS.md and REPORT.md adopt the same raw XML
  element style, without adding first-class handoff structure.

## Goals

- Every spec directory has `SPEC.md` as the single source for
  requirements and validation scenarios.
- Per-spec `spec.toml` files are removed and rejected as stray state.
- The canonical SPEC.md format remains normal Markdown with
  line-isolated speccy marker comments.
- Parsing is strict for marker shape, ids, nesting, required
  requirement/scenario coverage, and duplicate ids.
- Rendering from Rust structs produces deterministic marker placement.
- Prompt slicing reads the typed marker tree instead of combining
  `SPEC.md` and `spec.toml`.

## Non-goals

- No raw XML tree containing Markdown. Marker comments are deliberate:
  they provide source-level XML-style anchors without making Markdown
  body text XML payload.
- No `quick-xml` dependency for this carrier. The file is not an XML
  document; it is Markdown plus Speccy marker comments.
- No TASKS.md or REPORT.md migration in this spec.
- No schema for every prose section. Design, notes, assumptions, and
  most narrative sections remain Markdown. The parser structures the
  parts orchestration needs: requirements, scenarios, decisions where
  marked, changelog entries, and open questions.
- No back-compat shim for per-spec `spec.toml` after migration.

## User Stories

- As an implementer agent, I want one SPEC.md to contain the product
  intent, requirement text, and validation scenarios for my task.
- As a reviewer-tests persona, I want each validation scenario to sit
  next to the requirement it proves, written before implementation.
- As a human reading on GitHub, I want the document to render as
  normal Markdown; marker comments should not dominate the visual
  reading experience.
- As a harness author, I want a typed parser and deterministic renderer
  so agent edits can be validated and re-rendered without inventing
  project-specific rules.

## Requirements

<requirement id="REQ-001">
### REQ-001: SPEC.md marker grammar is strict and Markdown-friendly

SPEC.md remains a Markdown document with YAML frontmatter and a level-1
heading. Machine structure is carried by line-isolated Speccy marker
comments.

**Done when:**
- Marker syntax is exactly an HTML comment whose body starts with
  `speccy:<name>` for opens (with optional `attr="value"` pairs) and
  `/speccy:<name>` for closes. Marker comments occupy their own line.
- Marker comments must appear on their own lines.
- Attribute values are double-quoted; unquoted values are rejected.
- The parser accepts these marker names:
  - `spec` root, optional but emitted by the renderer;
  - `summary`, optional single section;
  - `requirement`, required 1+, with `id="REQ-NNN"`;
  - `scenario`, required 1+ inside each requirement, with
    `id="CHK-NNN"`;
  - `decision`, optional 0+, with `id="DEC-NNN"` and optional
    `status="accepted|rejected|deferred|superseded"`;
  - `open-question`, optional 0+, with optional
    `resolved="true|false"`;
  - `changelog`, required single section.
- Unknown marker names and unknown attributes are parse errors.
- Invalid ids are parse errors:
  - requirements: `REQ-\d{3,}`;
  - scenarios: `CHK-\d{3,}`;
  - decisions: `DEC-\d{3,}`.
- Duplicate requirement ids, duplicate scenario ids within one spec,
  and duplicate decision ids are parse errors.
- A `scenario` marker must be nested inside exactly one
  `requirement` marker.
- The body of each required marker (`requirement`, `scenario`,
  `changelog`) must contain non-whitespace Markdown.
- Markdown inside markers is not XML-escaped and is not parsed as XML.
  Code fences, Markdown links, `<T>`, `&`, and arbitrary prose remain
  valid Markdown content.

**Behavior:**
- Given a SPEC.md with a requirement marker containing a scenario
  marker, parsing returns a typed requirement with one scenario.
- Given a scenario marker outside any requirement, parsing fails.
- Given two scenario markers with the same `id="CHK-001"` in one
  spec, parsing fails with a duplicate-id error.
- Given prose inside a scenario containing `<html>` or `A & B`,
  parsing preserves it as Markdown text instead of requiring XML
  escaping.

<scenario id="CHK-001">
- Given a SPEC.md with a requirement marker containing a scenario
  marker, parsing returns a typed requirement with one scenario.
- Given a scenario marker outside any requirement, parsing fails.
- Given two scenario markers with the same `id="CHK-001"` in one
  spec, parsing fails with a duplicate-id error.
- Given prose inside a scenario containing `<html>` or `A & B`,
  parsing preserves it as Markdown text instead of requiring XML
  escaping.

Given a SPEC.md with line-isolated speccy requirement and scenario
marker comments,
when the marker parser runs,
then it returns a typed requirement with nested scenarios.

Given a scenario marker outside any requirement marker,
when parsing runs,
then parsing fails and names the misplaced scenario.

Given Markdown content inside a scenario containing XML-looking text
such as <T> or A & B,
when parsing runs,
then the content is preserved as Markdown and is not XML-decoded.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Per-spec spec.toml is removed

After this spec lands, requirement-to-scenario linkage is represented
only by marker containment in SPEC.md.

**Done when:**
- No `.speccy/specs/**/spec.toml` files remain after migration.
- The workspace loader rejects a per-spec `spec.toml` with
  `WorkspaceError::StraySpecToml`.
- `SpecToml`, `RequirementEntry`, `CheckEntry`, and the spec-level
  TOML parser are deleted.
- `speccy.toml` workspace-config parsing remains.
- The old `[[requirements]].checks` relation is replaced by:
  `scenario.parent_requirement_id`.

**Behavior:**
- Given a migrated workspace, each spec directory contains `SPEC.md`
  and optionally `TASKS.md` / `REPORT.md`, but no `spec.toml`.
- Given a manually reintroduced `.speccy/specs/0001-foo/spec.toml`,
  workspace loading fails and names the stray file.
- Given a requirement marker with two nested scenario markers, status,
  check, verify, and prompt rendering all see two scenarios proving
  that requirement.

<scenario id="CHK-002">
- Given a migrated workspace, each spec directory contains `SPEC.md`
  and optionally `TASKS.md` / `REPORT.md`, but no `spec.toml`.
- Given a manually reintroduced `.speccy/specs/0001-foo/spec.toml`,
  workspace loading fails and names the stray file.
- Given a requirement marker with two nested scenario markers, status,
  check, verify, and prompt rendering all see two scenarios proving
  that requirement.

Given the migrated workspace,
when the workspace loader scans .speccy/specs,
then no per-spec spec.toml files are present.

Given a stray per-spec spec.toml file,
when the workspace loader runs,
then it returns a StraySpecToml error naming the file.

Given a requirement marker containing two scenario markers,
when Speccy computes requirement coverage,
then both scenarios are linked to that requirement by containment.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Parser and renderer are backed by Rust structs

The canonical carrier is not "whatever Markdown the agent wrote." It
is a typed model that can be parsed, validated, sliced, and rendered.

**Done when:**
- `speccy-core::parse::spec_markers` exposes:
  - `SpecDoc`;
  - `Requirement`;
  - `Scenario`;
  - `Decision`;
  - `MarkerSpan`;
  - `parse(source, path) -> Result<SpecDoc, ParseError>`;
  - `render(&SpecDoc) -> String`.
- Parsing uses the existing frontmatter splitter and a marker scanner
  over the Markdown body. It may use `comrak` to avoid fenced-code
  false positives, but it does not treat the whole body as XML.
- Marker spans preserve byte ranges so diagnostics can point at the
  offending marker.
- Rendering is deterministic:
  - marker comments are line-isolated;
  - marker attributes are emitted in a stable order;
  - requirement and scenario order follows the struct order;
  - Markdown bodies are preserved except for trailing whitespace
    normalization at marker boundaries.
- Parse then render then parse produces a structurally equivalent
  `SpecDoc`.

**Behavior:**
- Given a hand-authored canonical SPEC.md, parse/render/parse yields
  equal ids, parent links, marker names, and Markdown bodies.
- Given a marker hidden inside a fenced code block, it is treated as
  code content, not as structure.
- Given an unknown marker attribute, the parse error names the marker,
  attribute, path, and byte offset.

<scenario id="CHK-003">
- Given a hand-authored canonical SPEC.md, parse/render/parse yields
  equal ids, parent links, marker names, and Markdown bodies.
- Given a marker hidden inside a fenced code block, it is treated as
  code content, not as structure.
- Given an unknown marker attribute, the parse error names the marker,
  attribute, path, and byte offset.

Given a canonical marker-structured SPEC.md,
when parse, render, and parse run in sequence,
then the resulting SpecDoc is structurally equivalent to the first.

Given a speccy marker inside a fenced code block,
when parsing runs,
then the marker is treated as code content, not structure.

Given a marker with an unknown attribute,
when parsing runs,
then the error names the attribute, marker name, file, and byte offset.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Migration rewrites all in-tree specs

An ephemeral migration tool rewrites the current two-file specs into
canonical marker-structured SPEC.md files.

**Done when:**
- `xtask/migrate-spec-markers-0019` exists during implementation and
  is deleted before the final commit.
- The migration reads the post-SPEC-0018 shape:
  `SPEC.md` plus `spec.toml` with `id` + `scenario` checks.
- Frontmatter and the level-1 heading are preserved.
- Each existing `### REQ-NNN` block becomes a
  `speccy:requirement` marker block.
- Each covered CHK id becomes a nested `speccy:scenario` marker block
  under the requirement that referenced it.
- Existing Behavior / Given-When-Then prose is preferred for scenario
  body text; the old `scenario = """..."""` text is appended only
  when it carries information not already present in the requirement
  block.
- Existing `### DEC-NNN` blocks may be wrapped in `speccy:decision`
  markers, but their inner prose remains Markdown.
- Existing changelog tables are wrapped in one `speccy:changelog`
  marker block.
- The migration emits warnings, not silent guesses, when a requirement
  lacks behavior prose or a scenario cannot be placed unambiguously.

**Behavior:**
- Given a pre-migration requirement covered by `CHK-002` and
  `CHK-003`, the migrated requirement block contains two nested
  scenario markers in that order.
- Given a pre-migration spec with a `spec.toml` check not referenced
  by any requirement, migration fails and names the orphan check.
- Given the migrated workspace, `speccy verify` exits 0.

<scenario id="CHK-004">
- Given a pre-migration requirement covered by `CHK-002` and
  `CHK-003`, the migrated requirement block contains two nested
  scenario markers in that order.
- Given a pre-migration spec with a `spec.toml` check not referenced
  by any requirement, migration fails and names the orphan check.
- Given the migrated workspace, `speccy verify` exits 0.

Given a post-SPEC-0018 workspace with SPEC.md plus spec.toml files,
when the migration tool runs,
then each spec is rewritten to marker-structured SPEC.md and the
spec.toml file is deleted.

Given a check row that is not referenced by any requirement,
when migration runs,
then migration fails and names the orphan check.

Given the migrated workspace,
when speccy verify runs,
then it exits zero.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Prompts, docs, and slicing consume the marker tree

Agent-facing prompts must stop loading two separate spec carriers.

**Done when:**
- Implementer and reviewer prompt rendering reads `SpecDoc`.
- Task-scoped slicing includes:
  - frontmatter summary;
  - the spec summary;
  - every requirement covered by the task;
  - every scenario nested under those requirements;
  - design and decision prose needed for context.
- `speccy check` renders scenarios from SPEC.md marker blocks.
- `speccy verify` validates marker structure and cross-references
  instead of spec-level TOML.
- `.speccy/ARCHITECTURE.md` removes per-spec `spec.toml` from the
  file layout and documents the marker grammar.
- Shipped skills and prompts no longer instruct agents to read or
  edit per-spec `spec.toml`.

**Behavior:**
- Given a task covering `REQ-002`, the implementer prompt includes
  `REQ-002` and its scenarios but not unrelated requirements.
- Given reviewer-tests reads a prompt for the same task, it sees the
  exact scenario text from SPEC.md markers.
- Given active docs and shipped skills, `spec.toml` appears only in
  historical migration context.

<scenario id="CHK-005">
- Given a task covering `REQ-002`, the implementer prompt includes
  `REQ-002` and its scenarios but not unrelated requirements.
- Given reviewer-tests reads a prompt for the same task, it sees the
  exact scenario text from SPEC.md markers.
- Given active docs and shipped skills, `spec.toml` appears only in
  historical migration context.

Given a task covering REQ-002,
when implementer or reviewer prompt slicing runs,
then the prompt includes REQ-002 and its scenario marker bodies but
excludes unrelated requirements.

Given the post-spec docs and shipped skills,
when active guidance is searched,
then per-spec spec.toml appears only in historical or migration notes.
</scenario>

</requirement>

## Design

### Approach

Implementation order:

1. Add the marker parser/renderer with hand-authored fixtures.
2. Build the migration tool and test it on one spec.
3. Run migration across all in-tree specs.
4. Switch workspace loading, check rendering, verify, and prompt
   slicing to `SpecDoc`.
5. Delete the spec-level TOML parser and per-spec `spec.toml` files.
6. Sweep architecture docs and shipped skills.
7. Delete the migration tool.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Use marker comments, not raw XML containers

**Status:** Accepted

**Context:** Raw XML around Markdown looks attractive as a schema, but
it is the wrong carrier for human-authored Markdown. Markdown inside
custom HTML/XML blocks does not reliably render as Markdown, and prose
or code containing XML metacharacters would need escaping.

**Decision:** Use XML-style HTML comments as markers around normal
Markdown. The source remains easy for LLMs to ingest and easy for
humans to read; the parser still gets deterministic anchors.

**Consequences:** We write a small marker scanner instead of using an
XML parser. That is the right tradeoff because the artifact is
Markdown-with-markers, not XML.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Containment replaces the REQ-to-CHK table

**Status:** Accepted

**Context:** The old TOML table duplicated information that belongs
next to the requirement.

**Decision:** A scenario proves the requirement whose marker contains
it.

**Consequences:** Moving a scenario to a different requirement is an
ordinary Markdown edit with a visible diff.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: The renderer is canonical, but no formatter command ships

**Status:** Accepted

**Context:** Speccy needs a deterministic renderer for migrations,
prompt slices, tests, and future repair workflows. A public
`speccy fmt` command is a separate product surface.

**Decision:** Implement rendering as library functionality used by
the CLI internals and migrations. Do not add a user-facing format
command in this spec.

**Consequences:** We get deterministic output where needed without
expanding the v1 command surface.
</decision>

## Migration / Rollback

Migration is structural and fails closed when placement is ambiguous.
The migration tool should never invent scenario text. If a requirement
does not contain enough behavior prose to produce a scenario body, the
tool emits a warning and uses the SPEC-0018 `scenario` text as the
source of truth.

Rollback is `git revert` of the implementation commit set. The old
SPEC.md plus `spec.toml` shape remains in history.

## Open Questions

- [ ] Should the root `speccy:spec` marker be required or merely
      rendered by default? Lean required after migration; optional
      only for parser diagnostics during migration.
- [ ] Should decision markers be required for every `DEC-NNN` block?
      Lean yes if the migration can do it reliably; otherwise allow
      unmarked decision prose and defer strict decision parsing.

## Assumptions

- Existing specs use enough stable headings (`### REQ-NNN`,
  `**Covered by:**`, `### DEC-NNN`) for a structural migration.
- The marker scanner can ignore fenced code blocks, either with
  `comrak` events or a small fence-aware line scanner.
- GitHub hides HTML comments in rendered Markdown, which is
  acceptable because the source file is the agent-facing contract.

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial rewritten draft. Replaces raw XML-with-Markdown with XML-style marker comments and removes per-spec spec.toml. |
</changelog>

## Notes

This spec deliberately does less schema work than the previous raw-XML
draft. The substrate hardening comes from making the load-bearing
contract structurally parseable: requirements, scenarios, ids,
containment, and changelog. Freeform design prose remains freeform
Markdown.
