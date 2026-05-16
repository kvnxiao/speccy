---
id: SPEC-0099
slug: canonical-fixture
title: Canonical fixture
status: in-progress
created: 2026-05-15
---

# SPEC-0099: Canonical fixture

<overview>
A hand-authored fixture that exercises every element block the SPEC-0020
raw-XML parser recognises. T-001 exercises the parser side; T-002 will
add the roundtrip test that consumes this fixture.
</overview>

<requirement id="REQ-001">
First requirement prose with `<T>` and `A & B` to ensure verbatim
preservation of XML metacharacters in Markdown.

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

<scenario id="CHK-003">
Given REQ-002 has one scenario,
when the parser collects scenarios,
then CHK-003 is the only scenario under REQ-002.
</scenario>
</requirement>

<decision id="DEC-001" status="accepted">
Use raw XML element tags as the canonical SPEC.md carrier (SPEC-0020).
</decision>

<open-question resolved="false">
Should the root `<spec>` element be required after migration?
</open-question>

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial canonical fixture for the SPEC-0020 XML carrier. |
</changelog>
