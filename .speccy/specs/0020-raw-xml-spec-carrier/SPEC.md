---
id: SPEC-0020
slug: raw-xml-spec-carrier
title: Raw XML element tags for canonical SPEC.md
status: implemented
created: 2026-05-15
supersedes: []
---

# SPEC-0020: Raw XML element tags for canonical SPEC.md

## Summary

SPEC-0019 made `SPEC.md` the single canonical carrier for
requirements and validation scenarios. Its DEC-001 chose
HTML-comment markers (`<!-- speccy:requirement id="REQ-001" -->`)
over raw XML element tags, on the assumption that comment-wrapped
markers would preserve Markdown rendering on GitHub while still
giving the parser deterministic anchors.

In practice, two things have shifted that assumption:

1. **Vendor guidance disagrees.** Anthropic's published prompt
   engineering guide tells authors to "structure prompts with XML
   tags" using raw elements such as `<instructions>`, `<context>`,
   `<document>`, `<example>`, and `<thinking>`. OpenAI's GPT-4.1
   prompting guide recommends raw XML for "precise section wrapping,
   metadata addition, and nesting — particularly valuable in
   long-context scenarios." Neither vendor recommends HTML
   comment-wrapped tags as a substitute. Speccy's primary readers are
   LLM agents executing the implementation/review loop; we should
   structure the agent-facing contract the way the model providers
   structure their own prompts.

2. **GitHub render fidelity was the wrong thing to optimise for.**
   Speccy's SPEC.md is read by agents from disk during prompt slicing.
   Humans review the diff, not a GitHub-rendered preview. Trading
   vendor-recommended prompt structure for nicer GitHub rendering was
   the wrong tradeoff for the workload Speccy actually has.

This spec switches `SPEC.md`'s carrier from HTML-comment markers to
raw XML element tags such as:

```markdown
<requirement id="REQ-001">
### REQ-001: Render selected scenarios

Plain Markdown prose remains plain Markdown.

<scenario id="CHK-001">
Given a task covers REQ-001,
when `speccy check SPEC-0019/T-001` runs,
then only REQ-001's scenarios are rendered.
</scenario>
</requirement>
```

Tag names are bare semantic words (`requirement`, `scenario`,
`decision`, `changelog`, `open-question`, plus optional `spec` and
`overview` wrappers), matching the form Anthropic and OpenAI publish
in their prompt-engineering examples. The whitelist is also disjoint
from the HTML5 element name set, so parsers, syntax highlighters,
and LLMs never see a Speccy structure tag they might mistake for an
HTML element. (The conceptual "summary" section uses `<overview>`
rather than the HTML5-reserved `<summary>` element name; everything
else in the whitelist is already non-HTML5.) The parser recognises a
closed whitelist of these names; any other element-looking text in a
SPEC.md body is treated as Markdown content. Examples that need to
show Speccy's own grammar live inside fenced code blocks or inline
backticks, where the line-aware scanner already ignores them.

Everything else from SPEC-0019 carries over: SPEC.md remains the
single carrier; `spec.toml` does not return; the parser remains a
line-aware element scanner that treats unrecognised content as
Markdown body; the renderer remains deterministic.

This is the third step in the sequence:

- **SPEC-0018:** checks become English validation scenarios.
- **SPEC-0019:** SPEC.md becomes single-carrier with HTML-comment
  markers; per-spec `spec.toml` is removed.
- **SPEC-0020:** SPEC.md switches from HTML-comment markers to raw
  XML element tags.
- **SPEC-0021:** TASKS.md and REPORT.md adopt the same raw XML
  element style.

## Goals

- SPEC.md carries Speccy structure with raw XML element tags
  (`<requirement>`, `<scenario>`, `<decision>`, `<changelog>`,
  `<open-question>`, optional `<spec>` root and `<overview>` section).
- The set of recognised Speccy element names is disjoint from the
  HTML5 element name set, so no Speccy structure tag collides with a
  standard HTML element.
- Markdown bodies inside elements remain Markdown; they are not XML
  payload and do not require entity escaping.
- The parser remains a line-aware element scanner, not a full XML
  document parser.
- All in-tree SPEC.md files migrate from comment markers to raw XML
  element tags in one mechanical pass.
- Architecture docs and shipped skill prompts teach the raw XML form
  with no leftover guidance pointing at comment markers as the
  active carrier.

## Non-goals

- No revival of `spec.toml` or any other secondary spec carrier.
- No full XML document parser. The file is still Markdown plus a
  small whitelisted set of Speccy element tags; bodies are not XML
  payload.
- No `quick-xml` dependency for SPEC.md parsing. The scanner stays
  line-aware over the Markdown body.
- No new structural concepts. The element set mirrors what SPEC-0019
  shipped: requirement, scenario, decision, changelog, open-question,
  and optional spec/overview wrappers. Adding new structural elements
  is out of scope.
- No migration of TASKS.md or REPORT.md. That belongs to SPEC-0021.
- No back-compat shim that accepts both forms. After migration, the
  comment-marker form is rejected as a parse error.

## User Stories

- As an implementer agent reading a sliced SPEC.md, I want vendor-
  recommended XML structure around requirements and scenarios so I
  ground on the right block without extra prompting effort.
- As a reviewer-tests persona, I want each scenario inside a clearly
  delimited `<scenario>` element so my prompt can ask "are
  these scenarios sufficient?" with the relevant text directly
  attended to.
- As a harness author, I want one carrier form across SPEC.md (this
  spec) and TASKS.md/REPORT.md (the next spec), so prompt rendering
  uses one element scanner instead of two.
- As a human reviewing a SPEC.md diff, I want structure tags that I
  can read at a glance and Markdown bodies that still read as
  Markdown in my editor.

## Requirements

<requirement id="REQ-001">
### REQ-001: SPEC.md uses raw XML element tags

SPEC.md remains a Markdown document with YAML frontmatter and a
level-1 heading. Machine structure is carried by line-isolated raw
XML element tags drawn from a closed whitelist of bare semantic
names.

**Done when:**
- Element syntax is a literal XML open/close tag pair on its own
  line: `<NAME attr="value">` to open, `</NAME>` to close.
- Attribute values are always double-quoted. Unquoted values are
  parse errors.
- The parser recognises a closed whitelist of element names:
  - `spec`, optional root, emitted by the renderer;
  - `overview`, optional single section;
  - `requirement`, required 1+, with `id="REQ-NNN"`;
  - `scenario`, required 1+ inside each requirement, with
    `id="CHK-NNN"`;
  - `decision`, optional 0+, with `id="DEC-NNN"` and optional
    `status="accepted|rejected|deferred|superseded"`;
  - `open-question`, optional 0+, with optional
    `resolved="true|false"`;
  - `changelog`, required single section.
- The whitelist is disjoint from the HTML5 element name set as
  defined by the WHATWG HTML Living Standard element index. A unit
  test in `speccy-core` enforces this invariant: every Speccy
  element name must not appear in the HTML5 reserved set
  (`html`, `head`, `body`, `title`, `header`, `footer`, `main`,
  `nav`, `aside`, `section`, `article`, `summary`, `details`,
  `figure`, `figcaption`, `table`, `thead`, `tbody`, `tr`, `td`,
  `th`, `caption`, `colgroup`, `col`, `form`, `input`, `button`,
  `select`, `option`, `textarea`, `label`, `fieldset`, `legend`,
  `output`, `progress`, `meter`, `dialog`, `script`, `style`,
  `link`, `meta`, `template`, `slot`, `iframe`, `embed`, `object`,
  `param`, `picture`, `source`, `video`, `audio`, `track`, `canvas`,
  `map`, `area`, `img`, `svg`, `math`, plus every other element in
  the standard index).
- Element names outside the whitelist (e.g. `<thinking>` or
  `<example>` in user prose, or any HTML5 element name appearing on
  its own line) are treated as Markdown body content, not structure.
- Element-looking text inside fenced code blocks or inline backticks
  is body content regardless of name.
- Invalid ids are parse errors (same rules as SPEC-0019):
  requirements `REQ-\d{3,}`, scenarios `CHK-\d{3,}`, decisions
  `DEC-\d{3,}`.
- Duplicate requirement ids, duplicate scenario ids within one spec,
  and duplicate decision ids are parse errors.
- A `<scenario>` element must be nested inside exactly one
  `<requirement>` element.
- The body of each required element (`requirement`, `scenario`,
  `changelog`) must contain non-whitespace Markdown.
- Markdown inside elements is preserved verbatim. `<`, `>`, and `&`
  inside scenario/decision bodies remain ordinary Markdown
  characters; the parser does not XML-decode body content.

**Behavior:**
- Given a SPEC.md with a `<requirement>` containing one
  `<scenario>`, parsing returns a typed requirement with one
  scenario.
- Given a `<scenario>` element outside any requirement,
  parsing fails and names the misplaced scenario.
- Given a SPEC.md body containing the literal text `<thinking>` or
  `<example>` outside any Speccy element, parsing preserves it as
  Markdown content.
- Given two `<scenario id="CHK-001">` opens in one spec,
  parsing fails with a duplicate-id error.

<scenario id="CHK-001">
Given a SPEC.md with line-isolated speccy XML element tags wrapping
a requirement and a nested scenario,
when the element parser runs,
then it returns a typed requirement with nested scenarios.

Given a scenario element outside any requirement element,
when parsing runs,
then parsing fails and names the misplaced scenario.

Given Markdown content containing literal angle-bracket text such as
<thinking>, <example>, or <T>,
when parsing runs,
then the content is preserved as Markdown body and is not interpreted
as structure.

Given two scenario elements with the same id within one spec,
when parsing runs,
then parsing fails with a duplicate-id error naming the id.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: HTML-comment marker form is removed and rejected

After this spec lands, the SPEC-0019 marker form is no longer a
valid carrier. The parser, renderer, and migration tool only emit
raw element tags.

**Done when:**
- The marker scanner module (`speccy-core::parse::spec_markers` or
  its current name) is replaced by an XML element scanner module.
  Old marker-comment parsing code is deleted, not feature-flagged.
- A SPEC.md still containing `<!-- speccy:NAME -->` or
  `<!-- /speccy:NAME -->` comments is a parse error after migration,
  with a diagnostic suggesting the equivalent XML element form.
- The renderer emits only raw XML element tags. It never produces
  HTML-comment markers.
- No back-compat acceptance of mixed forms within one SPEC.md.

**Behavior:**
- Given a migrated SPEC.md, every Speccy structure tag is a raw XML
  element, not an HTML comment.
- Given a hand-authored SPEC.md that still contains
  `<!-- speccy:requirement id="REQ-001" -->`, parsing fails and the
  error names the legacy marker form and points to the equivalent
  element syntax.

<scenario id="CHK-002">
Given a SPEC.md authored entirely with raw speccy XML element tags,
when parsing runs,
then it succeeds and returns the typed SpecDoc.

Given a SPEC.md containing legacy HTML-comment Speccy markers after
migration,
when parsing runs,
then it fails and the diagnostic names the legacy marker form and
suggests the equivalent XML element.

Given the renderer is invoked on a typed SpecDoc,
when its output is inspected,
then it contains no HTML-comment Speccy markers.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Parser and renderer expose XML-backed Rust structs

The carrier change does not change Speccy's model. Existing
consumers continue to read `SpecDoc`, `Requirement`, `Scenario`, and
`Decision`; only the on-disk form they parse from changes.

**Done when:**
- `speccy-core::parse::spec_xml` (or the renamed module) exposes:
  - `SpecDoc`;
  - `Requirement`;
  - `Scenario`;
  - `Decision`;
  - `ElementSpan` (replacing `MarkerSpan`);
  - `parse(source, path) -> Result<SpecDoc, ParseError>`;
  - `render(&SpecDoc) -> String`.
- The scanner is line-aware and ignores element-looking text inside
  fenced code blocks (`comrak` events or an equivalent fence-aware
  scanner are acceptable).
- Element spans carry byte ranges so diagnostics can point at the
  offending open/close tag.
- Rendering is deterministic:
  - element tags are line-isolated;
  - attributes are emitted in a stable order;
  - requirement and scenario order follows struct order;
  - Markdown bodies are preserved except for trailing-whitespace
    normalisation at element boundaries.
- Parse then render then parse produces a structurally equivalent
  `SpecDoc`.
- Existing callers of the SPEC-0019 marker parser (`speccy check`,
  prompt slicing, `speccy verify`, workspace loader) compile and
  pass against the new module with no behavioural change beyond the
  carrier form.

**Behavior:**
- Given a hand-authored canonical SPEC.md, parse/render/parse yields
  equal ids, parent links, element names, and Markdown bodies.
- Given a speccy element tag hidden inside a fenced code block, it
  is treated as code content, not structure.
- Given an unknown attribute on a known element, the parse error
  names the attribute, element name, path, and byte offset.

<scenario id="CHK-003">
Given a canonical raw-XML SPEC.md fixture,
when parse, render, and parse run in sequence,
then the resulting SpecDoc is structurally equivalent to the first.

Given a speccy XML element tag inside a fenced code block,
when parsing runs,
then the element is treated as Markdown body content, not structure.

Given a speccy element with an unknown attribute,
when parsing runs,
then the error names the attribute, element name, file, and byte
offset.

Given existing call sites for the SPEC-0019 marker parser,
when the workspace builds against the new XML element parser,
then they consume the same typed SpecDoc model with no behavioural
change beyond carrier form.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Migration rewrites all in-tree SPEC.md files

An ephemeral migration tool converts every in-tree SPEC.md from
comment markers to raw XML element tags.

**Done when:**
- `xtask/migrate-spec-xml-0020` exists during implementation and is
  deleted before the final commit.
- The migration reads each post-SPEC-0019 SPEC.md and rewrites it
  in-place to the raw XML element form. The transform is mechanical:
  - `<!-- speccy:NAME attr="v" -->` becomes `<NAME attr="v">`;
  - `<!-- /speccy:NAME -->` becomes `</NAME>`;
  - the `speccy:` namespace prefix on element names is dropped.
- Frontmatter, the level-1 heading, and all Markdown bodies between
  markers are preserved byte-for-byte.
- Comment markers that appear inside fenced code blocks (e.g.
  illustrative examples in this spec, SPEC-0019, or SPEC-0021) are
  left as-is: they are documentation about the old form, not
  structure.
- The migration emits warnings (not silent guesses) when it cannot
  determine whether a marker is structure or example.
- After migration, `speccy verify` exits 0 across the workspace.

**Behavior:**
- Given a pre-migration SPEC.md with marker-comment requirements and
  scenarios, the migrated file has equivalent raw XML element tags
  in the same nesting and order.
- Given a pre-migration SPEC.md whose summary contains a fenced code
  block showing the old marker syntax as an example, the migrated
  file leaves that example untouched.
- Given the migrated workspace, `speccy verify` exits 0.

<scenario id="CHK-004">
Given a post-SPEC-0019 SPEC.md authored with HTML-comment markers,
when the migration tool runs,
then the file is rewritten to raw speccy XML element tags with
preserved ids, nesting order, attribute values, and Markdown bodies.

Given a fenced code block inside SPEC.md containing example marker
comments as documentation,
when migration runs,
then the example block is left untouched.

Given the migrated workspace,
when speccy verify runs,
then it exits zero.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Docs, prompts, and shipped skills teach the XML element form

Active guidance must stop teaching the HTML-comment marker form as
the canonical carrier.

**Done when:**
- `.speccy/ARCHITECTURE.md` documents the raw XML element grammar
  for SPEC.md, with the HTML-comment marker form listed only as
  migration history.
- Shipped prompts under `resources/modules/prompts/` referencing
  SPEC.md structure use the raw XML element form in examples.
- Shipped skills under `.claude/skills/`, `.agents/skills/`, and
  `.codex/agents/` referencing SPEC.md structure are updated to the
  XML element form.
- Implementer and reviewer persona prompts read scenarios from
  `<scenario>` elements and requirements from
  `<requirement>` elements.
- A grep for `<!-- speccy:` in active (non-historical) guidance
  returns hits only inside migration-context documentation or this
  spec's own summary/decisions.

**Behavior:**
- Given a freshly rendered SPEC.md after this spec lands, every
  Speccy structure tag is a raw XML element.
- Given a task slice rendered for an implementer agent, the
  requirement and scenario blocks are wrapped in raw XML element
  tags inside the rendered prompt.
- Given active shipped guidance, `<!-- speccy:` appears only in
  historical/migration context.

<scenario id="CHK-005">
Given an implementer task slice rendered after this spec lands,
when the prompt is inspected,
then the requirement and scenario blocks are wrapped in raw speccy
XML element tags.

Given the post-spec docs and shipped skills,
when active guidance is searched,
then HTML-comment markers appear only in historical or migration
context.

Given a freshly authored SPEC.md following the post-spec shipped
plan-greenfield prompt,
when the file is inspected,
then it uses raw speccy XML element tags and contains no
HTML-comment Speccy markers.
</scenario>

</requirement>

## Design

### Approach

Implementation order:

1. Add the raw XML element parser/renderer with hand-authored
   fixtures, alongside the existing marker parser. Do not delete the
   marker parser yet.
2. Build the migration tool and test it against one fixture and one
   real in-tree spec.
3. Run migration across all in-tree SPEC.md files.
4. Switch workspace loading, prompt slicing, `speccy check`, and
   `speccy verify` to the XML element parser.
5. Delete the SPEC-0019 marker parser/renderer module and any
   remaining call sites.
6. Sweep architecture docs, shipped skills, and prompts.
7. Delete the migration tool.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Reverse SPEC-0019 DEC-001 — raw XML tags are the carrier

**Status:** Accepted

**Context:** SPEC-0019 DEC-001 chose HTML-comment markers over raw
XML element tags. The stated rationale was GitHub-render fidelity
and avoidance of XML escaping for Markdown bodies. In practice,
Speccy's primary readers are LLM agents, not GitHub previews, and
both Anthropic and OpenAI publicly recommend raw XML tags for
prompt structure. The escaping concern is moot once we keep bodies
as Markdown rather than XML payload (same approach as SPEC-0019, just
with raw element tags as the anchors).

**Decision:** SPEC.md uses raw XML element tags
(`<requirement>` etc.) as the canonical carrier. HTML-comment
markers are not accepted.

**Consequences:** SPEC-0019's DEC-001 is superseded by this
decision. The parser/renderer surface is mostly stable: same typed
model, different on-disk shape. Migration is mechanical. The
GitHub-render trade-off accepted here is explicit: Markdown bodies
inside Speccy element tags will not render as bullets/links/code in
GitHub's preview, because GitHub treats unknown block-level HTML as
opaque. This is acceptable because the agent-facing source is the
contract, not the preview.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Use bare semantic element names disjoint from HTML5

**Status:** Accepted

**Context:** A prior draft of this spec proposed a `speccy-`
kebab-case prefix on every structure tag to avoid collision with
prompt-engineering examples that legitimately contain `<thinking>`,
`<example>`, or `<document>` in body prose. Anthropic and OpenAI both
publish examples using bare semantic names, and Speccy's element
vocabulary (`requirement`, `scenario`, `decision`, `changelog`,
`open-question`, plus the `spec` and `overview` wrappers and the
`task`, `coverage`, `tasks`, `report`, `task-scenarios` set used by
SPEC-0021) is domain-specific enough that no realistic collision with
standard prompt-engineering tags exists.

The second concern is HTML5. A SPEC.md viewed in any HTML-aware
context (editor previews, GitHub partial rendering, prompt-rendering
LLMs that pattern-match HTML elements) should never have its Speccy
structure tags confused with real HTML elements. The original draft
included `<summary>` as a wrapper element, which is also an HTML5
element (`<summary>` inside `<details>`). That collision is resolved
by renaming the wrapper to `<overview>`. The disjointness rule
hardens the invariant going forward.

**Decision:** Speccy structure tags use bare semantic names without
a prefix: `<requirement>`, `<scenario>`, `<decision>`, `<changelog>`,
`<open-question>`, `<overview>`, `<spec>`. The whitelist of
recognised names is and will remain disjoint from the HTML5 element
name set (enforced by a unit test). Element-looking text outside the
whitelist on its own line is treated as Markdown body content.
Element-looking text inside fenced code blocks or inline backticks
is always body content regardless of name.

**Consequences:** Tag names match the vendor-recommended form as
closely as possible and read as short English words. SPEC.md files
that document Speccy's own grammar (this one included) must place
example tags inside fenced code blocks or inline backticks so the
scanner does not interpret them as structure — a constraint the
line-aware scanner already enforces and that SPEC-0019 also relied
on. Future structural additions must avoid HTML5 element names; the
unit-test invariant catches accidental collisions at build time
rather than during dogfooding.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Scanner stays line-aware, not a full XML parser

**Status:** Accepted

**Context:** The body inside every Speccy element is Markdown, not
XML payload. A full XML document parser would either require CDATA
sections around every body or would mis-parse legitimate Markdown
characters such as `<`, `>`, and `&`.

**Decision:** The element scanner remains line-aware. It recognises
whitelisted `<NAME>` opens and `</NAME>` closes when they appear on
their own line outside fenced code blocks. Body content between them
is treated as opaque Markdown text and preserved verbatim.

**Consequences:** We keep the SPEC-0019 invariant that bodies do not
require XML escaping. The parser implementation stays small. No
`quick-xml` dependency for this carrier.
</decision>

## Migration / Rollback

Migration is mechanical, line-by-line, fails closed when an
HTML-comment marker cannot be classified as either structure or
documentation (example inside a fenced code block).

Rollback is `git revert` of the implementation commit set. The
HTML-comment marker form remains in history and the SPEC-0019
parser would still be reachable via git.

## Open Questions

<open-question resolved="false">
Should the `<spec>` root element be required by the parser
after migration, or merely emitted by the renderer? Lean required
once dogfooded; optional in the migration window.
</open-question>

<open-question resolved="false">
Should the renderer emit a blank line after every closing element
tag for visual readability, or pack tags tight to keep diffs
narrower? Lean blank-line-after-close for readability, decide during
implementation against real fixtures.
</open-question>

## Assumptions

- SPEC-0019 has shipped on disk: every SPEC.md is currently in the
  HTML-comment marker form and every per-spec `spec.toml` has been
  removed.
- LLM agents reading SPEC.md from disk benefit at least as much from
  raw XML element tags as from HTML-comment markers, given vendor
  guidance.
- Loss of GitHub Markdown rendering inside structure tags is
  acceptable; humans review diffs and source, not GitHub previews.
- A line-aware element scanner is sufficient for v1 and does not
  require a true XML parser.

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial draft. Switches SPEC.md from HTML-comment markers (SPEC-0019 DEC-001) to raw XML element tags. |
| 2026-05-15 | human/kevin | DEC-002 revised: dropped the `speccy-` prefix; tag names are bare semantic words gated by a closed whitelist. |
| 2026-05-15 | human/kevin | DEC-002 extended: whitelist must be disjoint from HTML5 element names; renamed `<summary>` to `<overview>` to resolve the only existing collision. |
</changelog>

## Notes

This spec is narrow on purpose. It changes one decision in
SPEC-0019: the on-disk carrier form. Everything structural in
SPEC-0019 carries over unchanged — the single-carrier rule, typed
parser/renderer, scenario-under-requirement nesting, deterministic
rendering, and the "Markdown body, not XML payload" property.

The carrier change is timed before SPEC-0021 deliberately: rolling
the form change once across SPEC.md and again across
TASKS.md/REPORT.md would create two breaks in agent-facing structure.
Landing the SPEC.md change first means SPEC-0021 can adopt the
already-chosen XML element shape with no second carrier debate.
