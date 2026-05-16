---
id: SPEC-0021
slug: spec-section-xml-tags
title: Section-level XML element tags for SPEC.md
status: in-progress
created: 2026-05-16
supersedes: []
---

# SPEC-0021: Section-level XML element tags for SPEC.md

## Summary

SPEC-0020 made raw XML element tags the canonical carrier for the
machine-load-bearing structure inside `SPEC.md`: `<requirement>`,
`<scenario>`, `<decision>`, `<open-question>`, `<changelog>`, plus
optional `<spec>` and `<overview>` wrappers. The intent-bearing prose
surfaces inside SPEC.md still rely on Markdown conventions:
`## Goals`, `## Non-goals`, `## User Stories`, `**Done when:**`,
`**Behavior:**`, `## Assumptions`.

Anthropic's and OpenAI's published prompt-engineering guides both
recommend raw XML tags for "precise section wrapping, metadata
addition, and nesting — particularly valuable in long-context
scenarios." The full `SPEC.md` body is exactly that long-context
payload: implementer and reviewer prompts inject it whole into
`{{spec_md}}`. A reviewer-tests persona that wants to compare the
behavior contract against the proving scenarios is currently asked to
grep Markdown bold inside prose; tagging the contract surfaces makes
them directly addressable.

This spec extends the SPEC-0020 element whitelist with six new
semantic section tags around the intent-bearing prose, and retires
two unused entries (`<spec>` and `<overview>`) from the SPEC-0020
whitelist. Net change to the SPEC.md grammar: +4 element names.
Two of the new tags live inside each `<requirement>`:

```markdown
<requirement id="REQ-001">
### REQ-001: Render selected scenarios

<done-when>
Implementer-visible acceptance criteria as a bullet list.
</done-when>

<behavior>
- Given/When/Then prose that drives test selection.
</behavior>

<scenario id="CHK-001">
Given/When/Then scenario text proving the behavior.
</scenario>
</requirement>
```

The other four wrap top-level intent sections; each contains a
single Markdown bullet block (or paragraph body):

```markdown
<goals>
- Concrete outcomes this spec must achieve.
</goals>

<non-goals>
- Explicitly out of scope.
</non-goals>

<user-stories>
- As a <role>, I want <capability> so that <benefit>.
- As another <role>, I want <other-capability> so that <other-benefit>.
</user-stories>

<assumptions>
- Preconditions the spec relies on. Optional; omit entirely if there
  are no load-bearing assumptions.
</assumptions>
```

The existing line-aware element scanner from SPEC-0020 handles these
with no parser-shape changes; only the whitelist grows. Markdown
bodies inside the new elements remain opaque Markdown, the same as
inside `<scenario>` today.

This is the fourth step in the sequence:

- **SPEC-0018:** checks become English validation scenarios and
  Speccy stops executing commands.
- **SPEC-0019:** `SPEC.md` becomes single-carrier with HTML-comment
  markers; per-spec `spec.toml` disappears.
- **SPEC-0020:** `SPEC.md` switches from HTML-comment markers to raw
  XML element tags so vendor-recommended prompt structure applies.
- **SPEC-0021:** the SPEC.md element whitelist expands with semantic
  section tags around the intent-bearing prose surfaces.
- **SPEC-0022:** `TASKS.md` and `REPORT.md` adopt the same raw XML
  element style, reusing the SPEC-0021 element scanner.

## Goals

<goals>
- `<behavior>` and `<done-when>` wrap the contract-bearing prose
  inside every `<requirement>`, so prompts can address them directly
  rather than grepping Markdown bold.
- `<goals>`, `<non-goals>`, `<user-stories>` wrap the top-level
  intent sections, so reviewer-business and reviewer-tests personas
  can attend to the right blocks at slice time.
- `<assumptions>` is available as an optional top-level wrapper for
  specs that genuinely record load-bearing assumptions.
- The unused SPEC-0020 entries `<spec>` and `<overview>` are retired
  from the whitelist so dead schema does not tempt future "find a
  use for it" reasoning.
- All new tags are HTML5-disjoint, line-isolated, and parsed by the
  SPEC-0020 element scanner with no new infrastructure.
- Migration of in-tree SPEC.md files is mechanical and fails closed
  on ambiguous prose.
- ARCHITECTURE.md and shipped prompts teach the expanded grammar;
  implementer and reviewer prompts cite the new tags directly.
</goals>

## Non-goals

<non-goals>
- No new tags for narrative sections. `## Design`, `### Approach`,
  `### Decisions`, `## Migration / Rollback`, `## Notes` remain
  Markdown headings. They are narrative context, not contract
  surfaces.
- No plural container wrappers. `<requirements>`, `<decisions>`,
  `<open-questions>` are not introduced. Existing leaf elements
  (`<requirement>`, `<decision>`, `<open-question>`) remain
  top-level inside their Markdown sections; grouping them adds
  tokens without enabling new prompt patterns.
- No changes to existing element semantics. SPEC-0020 elements keep
  their current attributes, cardinality, and parse rules.
- No TASKS.md or REPORT.md changes. That remains SPEC-0022.
- No back-compat acceptance of pre-SPEC-0021 SPEC.md after migration;
  the post-migration parser rejects requirements missing
  `<behavior>` or `<done-when>`.
</non-goals>

## User Stories

<user-stories>
- As an implementer agent, I want the implementer prompt to cite
  `<behavior>` and `<done-when>` directly so I know which prose
  drives test selection and which drives acceptance.
- As a reviewer-tests persona, I want `<behavior>` and the nested
  `<scenario>` blocks side-by-side as separately addressable
  elements so my prompt can ask "does this scenario prove the
  behavior?" against attended-to surfaces.
- As a reviewer-business persona, I want `<goals>` and `<non-goals>`
  as distinct attention anchors so the diff intent is unambiguous
  even in long specs.
- As a human author, I want the new tags to read naturally; the
  template should feel like a slightly more structured version of
  what we already write.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: SPEC.md element whitelist evolves: six additions, two retirements

The SPEC-0020 element whitelist gains six new bare-semantic names:
two sub-requirement tags and four top-level section tags. Two
unused SPEC-0020 entries (`<spec>`, `<overview>`) retire from the
whitelist in the same pass. All new names are line-isolated raw XML
elements following the SPEC-0020 syntax rules.

<done-when>
- The element whitelist adds `behavior`, `done-when`, `goals`,
  `non-goals`, `user-stories`, and `assumptions`.
- The element whitelist removes `spec` and `overview`. The parser
  rejects them as unknown element names with a diagnostic noting
  they were retired in SPEC-0021.
- All six new names are disjoint from the HTML5 element name set;
  the SPEC-0020 unit test extends to enforce this for the new
  entries.
- `<done-when>` is required exactly once inside each `<requirement>`,
  positioned before `<behavior>`.
- `<behavior>` is required exactly once inside each `<requirement>`,
  positioned after `<done-when>` and before the first `<scenario>`.
- `<goals>` is required exactly once at the top level.
- `<non-goals>` is required exactly once at the top level.
- `<user-stories>` is required exactly once at the top level and
  contains opaque Markdown body (typically a bullet list of stories).
- `<assumptions>` is optional, zero or one at the top level. Specs
  without load-bearing assumptions omit the element entirely.
- Bodies inside all six new elements are opaque Markdown; the parser
  does not interpret them further.
- Element-shaped lines inside fenced code blocks or inline backticks
  are body content, not structure, as already enforced by SPEC-0020.
</done-when>

<behavior>
- Given a SPEC.md with a `<requirement>` containing `<done-when>`,
  `<behavior>`, and a `<scenario>` in that order, parsing returns a
  typed requirement with each field populated.
- Given a SPEC.md with a `<requirement>` lacking `<behavior>`,
  parsing fails and the diagnostic names the requirement id and the
  missing element.
- Given a SPEC.md with two `<goals>` elements at the top level,
  parsing fails with a duplicate-section error.
- Given a SPEC.md with a top-level `<spec>` or `<overview>` element,
  parsing fails and the diagnostic notes the element was retired in
  SPEC-0021.
- Given a SPEC.md with no `<assumptions>` element, parsing succeeds
  and the typed `assumptions` field is `None`.
- Given Markdown body content containing the literal text
  `<behavior>`, `<overview>`, or `<assumptions>` inside a fenced
  code block, parsing preserves it as code content, not structure.
</behavior>

<scenario id="CHK-001">
Given a SPEC.md with the six new section tags placed in canonical
order,
when the element parser runs,
then it returns a typed SpecDoc with done_when, behavior, goals,
non_goals, user_stories, and (optional) assumptions fields populated.

Given a SPEC.md with a requirement missing its required behavior
section,
when parsing runs,
then parsing fails and names the requirement id and missing element.

Given a SPEC.md with two top-level goals elements,
when parsing runs,
then parsing fails with a duplicate-section error naming the
element.

Given a SPEC.md containing a top-level spec or overview element,
when parsing runs,
then parsing fails and the diagnostic notes the element was retired
in SPEC-0021.

Given a SPEC.md without an assumptions element,
when parsing runs,
then parsing succeeds and the typed assumptions field is None.

Given Speccy section element tags inside fenced code blocks,
when parsing runs,
then they are preserved as code content, not structure.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Typed SpecDoc surface extends without renaming

The carrier change does not rename existing fields or modules. New
typed fields are added alongside SPEC-0020's; the canonical render
order is fixed so parse/render/parse round-trips remain equivalent.

<done-when>
- `speccy_core::parse::spec_xml::SpecDoc` gains required fields for
  the three top-level intent sections: `goals: MarkdownBody`,
  `non_goals: MarkdownBody`, and `user_stories: MarkdownBody`. It
  also gains one optional field: `assumptions: Option<MarkdownBody>`.
- `speccy_core::parse::spec_xml::Requirement` gains required fields
  `done_when: MarkdownBody` and `behavior: MarkdownBody`.
- The previously unused `SpecDoc.overview` field (if present in the
  SPEC-0020 struct definition) is removed, along with any
  `SpecDoc.root_element` or equivalent indicator of a `<spec>` root.
- Existing `Requirement` and `SpecDoc` fields keep their names and
  shapes. No public API rename beyond the two removals above.
- The renderer emits top-level sections in canonical order:
  `<goals>` → `<non-goals>` → `<user-stories>` → requirements →
  decisions → open-questions → `<assumptions>` (when present) →
  `<changelog>`.
- Inside each requirement the renderer emits `<done-when>` →
  `<behavior>` → nested `<scenario>` blocks in that order.
- The renderer inserts one blank line between adjacent element tags
  inside a requirement (after `</done-when>`, after `</behavior>`,
  between `<scenario>` blocks). Top-level sections are also
  separated by one blank line.
- Parse / render / parse round-trips remain structurally equivalent.
</done-when>

<behavior>
- Given a canonical SPEC.md with all six new tags, parse/render/parse
  yields a structurally equivalent `SpecDoc`.
- Given a typed SpecDoc passed to the renderer, the emitted file
  places `<done-when>` before `<behavior>` before nested
  `<scenario>` elements inside each requirement, separated by blank
  lines.
- Given a typed SpecDoc without an assumptions field set, the
  rendered file omits the `<assumptions>` element entirely.
- Given a typed SpecDoc, the emitted file places top-level sections
  in canonical order with no `<spec>` root and no `<overview>`.
</behavior>

<scenario id="CHK-002">
Given a canonical SPEC.md with all six new section tags,
when parse, render, and parse run in sequence,
then the resulting SpecDoc is structurally equivalent to the first.

Given a Requirement struct after this spec lands,
when rendered,
then done_when emits before behavior before nested scenarios, with
blank lines between adjacent element tags.

Given a SpecDoc struct with assumptions set to None,
when rendered,
then the emitted file contains no assumptions element.

Given a SpecDoc struct,
when rendered,
then top-level sections emit in canonical order: goals, non-goals,
user-stories, requirements, decisions, open-questions, assumptions
(when present), changelog; the rendered file has no spec or overview
elements.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Migration rewrites every in-tree SPEC.md

An ephemeral migration tool wraps existing Markdown intent surfaces
with the new XML element tags.

<done-when>
- `xtask/migrate-spec-sections-0021` exists during implementation
  and is deleted before the final commit.
- The migration reads each post-SPEC-0020 SPEC.md and rewrites
  intent surfaces in place:
  - `## Goals` body content wrapped in `<goals>` ... `</goals>`.
  - `## Non-goals` body content wrapped in `<non-goals>` ...
    `</non-goals>`.
  - `## User Stories` body content (the bullet list as a single
    block) wrapped in `<user-stories>` ... `</user-stories>`.
  - `## Assumptions` body content wrapped in `<assumptions>` ...
    `</assumptions>` when the section exists. Specs lacking a `##
    Assumptions` section migrate cleanly with no element emitted.
- Inside each `<requirement>`, the `**Done when:**` paragraph and
  its bullet body become `<done-when>` ... `</done-when>`, and the
  `**Behavior:**` paragraph and its bullet body become `<behavior>`
  ... `</behavior>`.
- Any pre-existing `<spec>` or `<overview>` tags in fixtures (none
  in the current in-tree corpus, but present in hand-authored
  examples) are stripped from migrated SPEC.md files.
- H2 section headings (`## Goals`, `## Non-goals`, `## User
  Stories`, `## Assumptions`) remain as Markdown above the new
  elements; the tags wrap the *body* of each section.
- Frontmatter, the level-1 heading, narrative sections (`## Design`,
  `## Notes`, etc.), and Markdown prose are preserved byte-for-byte.
- Migration fails closed when a requirement lacks identifiable
  `**Done when:**` or `**Behavior:**` prose; the diagnostic names the
  requirement rather than inventing content.
- After migration, `speccy verify` exits 0 across the workspace.
</done-when>

<behavior>
- Given a pre-migration SPEC.md with standard Markdown intent
  sections, the migrated file has each section's body wrapped in
  the matching new XML element tag while preserving the H2 heading
  above it.
- Given a pre-migration `<requirement>` with `**Done when:**` and
  `**Behavior:**` paragraphs, the migrated requirement contains
  `<done-when>` and `<behavior>` elements wrapping those exact
  paragraphs in canonical order.
- Given a pre-migration SPEC.md whose requirement lacks a
  `**Behavior:**` block, migration fails and names the requirement
  rather than guessing.
</behavior>

<scenario id="CHK-003">
Given a post-SPEC-0020 SPEC.md authored with Markdown H2 sections
and Markdown-bold sub-requirement labels,
when the migration tool runs,
then the file is rewritten with new XML element tags wrapping the
intent-bearing prose while preserving headings, frontmatter, and
unrelated Markdown bodies.

Given a requirement without identifiable Done when or Behavior
prose,
when migration runs,
then migration fails and names the requirement rather than
inventing content.

Given the migrated workspace,
when speccy verify runs,
then it exits zero.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Docs, prompts, and shipped skills cite the new tags

Active guidance must teach prompts to address the new elements
directly rather than grep Markdown bold and headings.

<done-when>
- `.speccy/ARCHITECTURE.md` documents the expanded SPEC.md element
  grammar; the element-names table adds rows for `behavior`,
  `done-when`, `goals`, `non-goals`, `user-stories`, and
  `assumptions` with cardinality and location, and removes the
  retired `spec` and `overview` rows.
- The SPEC.md template in `.speccy/ARCHITECTURE.md` is rewritten to
  use the new tags.
- `resources/modules/prompts/implementer.md` references `<behavior>`
  and `<done-when>` directly when telling the implementer what to
  satisfy.
- `resources/modules/prompts/reviewer-tests.md` references
  `<behavior>` and `<scenario>` directly when telling the persona
  what to compare.
- `resources/modules/prompts/reviewer-business.md` references
  `<goals>` and `<non-goals>` directly.
- Shipped skill packs (`.claude/skills/`, `.agents/skills/`,
  `.codex/agents/`) are updated where they reference SPEC.md
  structure.
- A grep for `**Behavior:**` or `**Done when:**` in active
  (non-historical) guidance returns hits only inside migration-context
  documentation.
</done-when>

<behavior>
- Given the post-spec implementer prompt, it cites `<behavior>` and
  `<done-when>` directly by name.
- Given the post-spec ARCHITECTURE.md, the SPEC.md element-names
  table contains rows for `behavior`, `done-when`, `goals`,
  `non-goals`, `user-stories`, and `assumptions`, and no rows for
  the retired `spec` and `overview` elements.
- Given a freshly authored SPEC.md following the post-spec template,
  the intent-bearing surfaces are wrapped with the new XML element
  tags.
</behavior>

<scenario id="CHK-004">
Given the post-spec implementer prompt,
when inspected,
then it cites the new behavior and done-when elements by name.

Given the post-spec reviewer-tests prompt,
when inspected,
then it cites behavior and scenario elements by name.

Given post-spec ARCHITECTURE.md,
when its SPEC.md element grammar is read,
then it lists the six new entries with cardinality and location and
omits rows for the retired spec and overview elements.

Given a SPEC.md authored from the post-spec template,
when inspected,
then goals, non-goals, user-stories, behavior, and done-when are
wrapped with their matching new XML element tags.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Sequence enables SPEC-0022 to reuse the wider whitelist

SPEC-0021 lands before SPEC-0022. The parser already supports the
wider whitelist when TASKS.md and REPORT.md migration runs.

<done-when>
- The implementation commit set for SPEC-0021 is merged before
  SPEC-0022 implementation begins.
- SPEC-0022's parser and migration reuse the SPEC-0021 element
  scanner with no further parser refactor.
- The HTML5 disjointness unit test enforces both SPEC-0020's and
  SPEC-0021's whitelist entries in one assertion list; SPEC-0022
  adds its entries to the same list when it ships.
</done-when>

<behavior>
- Given SPEC-0022 implementation starts after SPEC-0021 ships, the
  parser already knows the SPEC-0021 element names.
- Given the post-SPEC-0021 disjointness unit test, every Speccy
  element name (pre-SPEC-0022) is verified disjoint from HTML5 in
  one place.
</behavior>

<scenario id="CHK-005">
Given SPEC-0021 has shipped before SPEC-0022 implementation starts,
when SPEC-0022 implementers read the parser surface,
then they reuse the SPEC-0021 element scanner without adding a
parallel implementation.

Given the post-SPEC-0021 disjointness unit test,
when SPEC-0022 adds its element names to the whitelist,
then the same unit test covers the new entries.
</scenario>

</requirement>

## Design

### Approach

Implementation order:

1. Extend the SPEC-0020 element whitelist with the six new names
   (`behavior`, `done-when`, `goals`, `non-goals`, `user-stories`,
   `assumptions`) and add typed fields to `SpecDoc` and
   `Requirement`. Retire `spec` and `overview` from the whitelist
   in the same change; remove their typed-model fields if any
   exist.
2. Update the deterministic renderer to emit the new elements in
   canonical order.
3. Extend the HTML5-disjointness unit test to cover the new names.
4. Build the migration tool and test it against the most complex
   in-tree SPEC.md fixture.
5. Run migration across `.speccy/specs/*/SPEC.md` files.
6. Update ARCHITECTURE.md, the SPEC.md template inside it, and the
   implementer / reviewer-tests / reviewer-business persona prompts.
7. Sweep shipped skill packs.
8. Delete the migration tool.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Ship only Tier 1 and Tier 2 tags

**Status:** Accepted

**Context:** Three tiers of section-tag expansion are conceivable.
Tier 1 (`<behavior>`, `<done-when>` inside `<requirement>`) is the
highest-leverage surface because the implementer prompt drives test
design from `**Behavior:**`. Tier 2 (`<goals>`, `<non-goals>`,
`<user-stories>`, `<assumptions>`) provides top-level attention
anchors for reviewer-business and reviewer-tests personas. Tier 3
(`<requirements>`, `<decisions>`, `<open-questions>` plural container
wrappers around the existing leaf elements) groups already-addressable
nodes; the marginal LLM benefit is unclear. Tier 4 narrative tags
(`<design>`, `<approach>`, `<migration>`, `<notes>`) are pure
narrative context, not contract surfaces.

**Decision:** Ship Tier 1 and Tier 2. Defer Tier 3 to a follow-on
spec if prompt-slicing actually benefits from group containers in
practice. Skip Tier 4 entirely; H2 Markdown headings are sufficient
for narrative.

**Consequences:** The whitelist net change is +4 entries: six new
names added (the two sub-requirement tags plus four top-level
section wrappers) and two unused SPEC-0020 entries (`<spec>`,
`<overview>`) retired. Existing leaf elements stay top-level inside
their Markdown sections, so the diff against SPEC-0020 specs is
localised to the intent-bearing sub-blocks rather than reshuffling
structure.

</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: `<behavior>` and `<done-when>` are required inside every requirement

**Status:** Accepted

**Context:** The SPEC.md template in `.speccy/ARCHITECTURE.md`
already mandates both `**Done when:**` and `**Behavior:**` under
each requirement. Making them required tagged elements aligns the
parser with the existing template contract rather than extending it.

**Decision:** A `<requirement>` element must contain exactly one
`<done-when>` followed by exactly one `<behavior>`, both before the
first `<scenario>`. The order is fixed at the structural level so
parsers and renderers agree on layout.

**Consequences:** Migration fails closed for requirements that lack
either prose block. Going forward, drafting a new requirement
without both is a parse error, which catches "I forgot to write the
behavior" before review.

</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Narrative sections stay Markdown

**Status:** Accepted

**Context:** Anthropic and OpenAI recommend XML tags for "precise
section wrapping" in long-context prompts, but tag explosion has a
cost: every extra tag is a token the LLM reads, an authoring rule
a human remembers, and a whitelist entry the parser carries.
Narrative sections like `## Design`, `### Approach`, `## Migration /
Rollback`, and `## Notes` provide context for review and history;
they do not drive test selection or acceptance.

**Decision:** Narrative sections remain Markdown H2/H3 headings.
No `<design>`, `<approach>`, `<migration>`, `<rollback>`, or
`<notes>` elements are introduced.

**Consequences:** The element whitelist grows by six entries rather
than twelve-plus, with two unused entries retired in the same pass.
Authors keep narrative writing freeform. If a future need surfaces a
load-bearing narrative subsection, the whitelist can extend without
renaming anything that ships in this spec.

</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: Section bodies remain opaque Markdown

**Status:** Accepted

**Context:** SPEC-0019 DEC-001 established that element bodies are
opaque Markdown, not XML payload — letting prose contain `<`, `>`,
`&` and Markdown formatting without XML escaping. SPEC-0020 kept
that property. Inheriting it for the new elements keeps the parser
small and the authoring experience consistent across element kinds.

**Decision:** `<behavior>`, `<done-when>`, `<goals>`, `<non-goals>`,
`<user-stories>`, and `<assumptions>` carry opaque Markdown bodies.
The line-aware scanner from SPEC-0020 handles them with no further
infrastructure.

**Consequences:** No new escaping rules. Lists, links, inline code,
and prose all work inside the new tags the same way they work inside
`<scenario>` today. SPEC-0020's fenced-code handling already catches
documentation examples that show the new tags as content.

</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: `<assumptions>` is optional, zero or one at the top level

**Status:** Accepted

**Context:** Initially designed as required to catch missing-
assumption oversights. Reconsidered after audit (see DEC-008):
no persona prompt today cites `<assumptions>` specifically, the
section is author-facing context rather than a contract surface,
and forcing specs without genuine assumptions to write
`<assumptions>None recorded.</assumptions>` is ceremony that
doesn't catch anything real.

**Decision:** `<assumptions>` is optional, zero or one at the top
level. Specs without load-bearing assumptions omit the element
entirely. The typed model exposes `assumptions: Option<MarkdownBody>`.

**Consequences:** No stub materialization in migration. Authoring is
lighter — write the section only when it carries real preconditions.
Prompt rendering and downstream tools must handle the `None` case.
Future work can promote the element to required if a persona prompt
emerges that genuinely depends on it.

</decision>

<decision id="DEC-006" status="accepted">
#### DEC-006: Renderer emits blank lines between adjacent element tags

**Status:** Accepted

**Context:** Resolved open question from the initial SPEC-0021
draft. Inside a `<requirement>`, the elements `<done-when>`,
`<behavior>`, and one or more `<scenario>` blocks stack. The
renderer can pack them tight (close tag immediately followed by next
open tag) or separate them with a blank line.

**Decision:** The renderer emits exactly one blank line between
adjacent element tags inside a requirement (between
`</done-when>` and `<behavior>`, between `</behavior>` and the
first `<scenario>`, and between sibling `<scenario>` blocks).
Top-level sections are also separated by one blank line. This
matches the visual spacing SPEC-0020's renderer already uses for
decisions and changelog rows.

**Consequences:** Diffs are slightly wider (one extra newline per
boundary) but visually easier to scan. The blank-line rule is part
of the canonical render shape and a parse/render/parse round trip
preserves it.

</decision>

<decision id="DEC-007" status="accepted">
#### DEC-007: `<user-stories>` wraps a single Markdown bullet block

**Status:** Accepted

**Context:** Initially designed with nested `<story>` elements for
per-story addressability. Reconsidered after audit (see DEC-008): no
current prompt addresses individual stories. Reviewer-business reads
them as a set. Adding `<story>` would be a speculative element name
for hypothetical future workflows — the same anti-pattern that
justified retiring `<spec>` and `<overview>`.

**Decision:** `<user-stories>` is a single wrapper element containing
an opaque Markdown body (typically a bullet list). No nested per-story
element. The typed model exposes `user_stories: MarkdownBody`.

**Consequences:** Authors write user stories as a normal Markdown
bullet list inside `<user-stories>` ... `</user-stories>`. Reviewer
prompts cite the section as a whole, the same way they cite
`<goals>` or `<non-goals>`. If individual story identity ever becomes
load-bearing, a future spec can introduce a `<story>` element then
without breaking anything that ships in this spec.

</decision>

<decision id="DEC-008" status="accepted">
#### DEC-008: Retire unused `<spec>` and `<overview>` from SPEC-0020 whitelist

**Status:** Accepted

**Context:** SPEC-0020 introduced `<spec>` as an optional root and
`<overview>` as an optional single section. Neither is queried by
any code or prompt: `SpecDoc` is the typed surface, frontmatter and
the level-1 heading identify the spec, and no implementer or
reviewer prompt grounds on `<overview>`. Both elements are dead
schema in the whitelist.

The principle "tag what's load-bearing" applies in both directions:
adding tags for hypothetical future workflows is sunk-cost reasoning,
and so is keeping unused tags because they were once added. Dead
schema is worse than no schema because it tempts "find a use for it"
reasoning later — exactly the trap I almost fell into when offering
`<overview>` as a Summary wrapper earlier in SPEC-0021's design.

**Decision:** Retire `<spec>` and `<overview>` from the SPEC-0020
whitelist as part of SPEC-0021's element-set change. The parser
rejects them post-migration with a diagnostic noting they were
retired here. The typed `SpecDoc` removes any corresponding fields
(`overview`, `root_element`, etc.) if they exist.

**Consequences:** SPEC-0021 is the single change-set for the SPEC.md
whitelist evolution: six additions plus two retirements, net +4. The
SPEC-0020 grammar documentation in `.speccy/ARCHITECTURE.md`
removes the `spec` and `overview` rows. No in-tree SPEC.md uses
these elements today, so migration impact is limited to fixtures and
hand-authored examples.

</decision>

## Migration / Rollback

Migration is mechanical: walk each in-tree post-SPEC-0020 SPEC.md,
identify intent sections by their canonical Markdown shape (`##
Goals`, `**Behavior:**`, etc.), wrap their bodies with the matching
new XML element tag. The migration fails closed when a requirement
lacks either `**Done when:**` or `**Behavior:**` prose; the diagnostic
names the requirement rather than inventing content.

Rollback is `git revert` of the implementation commit set. The
SPEC-0020 form remains in git history.

## Open Questions

<open-question resolved="false">
Should the migration tool emit a warning (not an error) when it
encounters a SPEC.md that already has a `<spec>` or `<overview>`
element in a fixture or test corpus? Lean yes-warn so test authors
get explicit notice rather than a silent strip. Decide during
implementation against the actual fixture inventory.
</open-question>

## Assumptions

<assumptions>
- SPEC-0020 has shipped on disk: every in-tree SPEC.md uses raw XML
  element tags for requirements, scenarios, decisions,
  open-questions, and changelog.
- The line-aware element scanner from SPEC-0020 is the right
  abstraction; this spec adds whitelist entries and typed fields, no
  parser-shape changes.
- LLM agents addressing `<behavior>` and `<done-when>` benefit at
  least as much from named element tags as the SPEC-0020 jump from
  HTML-comment markers to raw XML benefited downstream prompts.
- The HTML5-disjointness invariant from SPEC-0020 DEC-002 still
  holds for the proposed element names; the unit test extends in one
  place to cover them.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-16 | human/kevin | Initial draft. Extends SPEC-0020's element whitelist with six section-level tags (`behavior`, `done-when`, `goals`, `non-goals`, `user-stories`, `assumptions`) around the intent-bearing prose surfaces inside SPEC.md. Defers plural container wrappers and narrative section tags. |
| 2026-05-16 | human/kevin | Resolved three initial open questions: `<assumptions>` is required exactly once (DEC-005); renderer emits blank lines between adjacent element tags inside requirements (DEC-006); `<user-stories>` wraps nested `<story>` elements rather than carrying one Markdown block (DEC-007). Whitelist now grows by seven entries (added `<story>`), totaling fourteen post-SPEC-0021. |
| 2026-05-16 | human/kevin | Audit pass walked back two recent decisions and retired two SPEC-0020 entries. DEC-005 rewritten: `<assumptions>` is optional (zero or one), not required. DEC-007 rewritten: `<user-stories>` wraps a single Markdown bullet block, no nested `<story>`. Added DEC-008: retire unused `<spec>` and `<overview>` from the SPEC-0020 whitelist. Net change to the SPEC.md whitelist is now +4 (six additions, two retirements). |
</changelog>

## Notes

This spec is narrow on purpose. It extends the element whitelist and
the typed `SpecDoc` / `Requirement` surface; everything else from
SPEC-0020 carries over unchanged — the single-carrier rule, the
line-aware element scanner, fenced-code handling, deterministic
rendering, and the "Markdown body, not XML payload" property.

The carrier expansion is timed before SPEC-0022 deliberately.
Rolling the SPEC.md element scanner once across SPEC-0021's
section-tag additions and again across SPEC-0022's
TASKS.md/REPORT.md additions would create two adjacent breaks in
agent-facing structure; landing SPEC-0021 first means SPEC-0022 can
adopt the wider whitelist with no second parser expansion.
