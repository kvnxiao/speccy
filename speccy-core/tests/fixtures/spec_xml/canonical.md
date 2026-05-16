---
id: SPEC-0099
slug: canonical-fixture
title: Canonical fixture
status: in-progress
created: 2026-05-15
---

# SPEC-0099: Canonical fixture

<goals>
A hand-authored fixture that exercises every element block the SPEC-0021
parser recognises.
</goals>

<non-goals>
Not a real spec; the fixture exists only to anchor parser and renderer
tests against the post-SPEC-0021 element shape.
</non-goals>

<user-stories>
- As a parser test, I want every required section to be present so the
  happy path stays exercised.
- As a renderer test, I want representative bodies that include XML
  metacharacters and fenced code so byte-preservation is asserted.
</user-stories>

<requirement id="REQ-001">
First requirement prose with `<T>` and `A & B` to ensure verbatim
preservation of XML metacharacters in Markdown.

<done-when>
- The fixture parses and renders without losing scenario bodies.
- Byte preservation across `<`, `>`, `&` holds for the first
  requirement.
</done-when>

<behavior>
- Given a fixture body containing `<T>` and `A & B`, the parser
  preserves the bytes verbatim.
- Given two scenarios under the first requirement, both are exposed in
  source order.
</behavior>

<scenario id="CHK-001">
Given a fixture body containing `<T>` and `A & B`,
when the parser reads it,
then the bytes are preserved without XML decoding.

```rust
fn example() -> &'static str {
    "<T> & friends"
}
```
</scenario>
<scenario id="CHK-002">
Given the second scenario under REQ-001,
when the parser walks scenarios in source order,
then this scenario follows CHK-001.
</scenario>
</requirement>

<requirement id="REQ-002">
Second requirement, single scenario.

<done-when>
- The fixture covers a single-scenario requirement so renderer logic
  exercises the count-of-one path.
</done-when>

<behavior>
- Given REQ-002 has one scenario, the parser collects exactly one
  scenario under it.
</behavior>

<scenario id="CHK-003">
Given REQ-002 has one scenario,
when the parser collects scenarios,
then CHK-003 is the only scenario under REQ-002.
</scenario>
</requirement>

<decision id="DEC-001" status="accepted">
Use raw XML element tags as the canonical SPEC.md carrier (SPEC-0020),
extended with section-level wrappers (SPEC-0021).
</decision>

<open-question resolved="false">
Should the optional `<assumptions>` element become required in a
future spec if persona prompts start grounding on it?
</open-question>

<assumptions>
- The fixture exists to anchor tests; production specs may omit
  `<assumptions>` per SPEC-0021 DEC-005.
</assumptions>

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial canonical fixture for the SPEC-0020 XML carrier. |
| 2026-05-16 | agent/claude | Re-shape fixture for SPEC-0021 section element tags. |
</changelog>
