---
id: SPEC-0099
slug: canonical-fixture
title: Canonical fixture
status: in-progress
created: 2026-05-15
---

# SPEC-0099: Canonical fixture

<!-- speccy:summary -->
A hand-authored fixture that exercises every marker block the renderer
emits. Used by `tests/spec_markers_roundtrip.rs` to assert parse/render
roundtrip equivalence.
<!-- /speccy:summary -->

<!-- speccy:requirement id="REQ-001" -->
First requirement prose with `<T>` and `A & B` to ensure verbatim
preservation of XML metacharacters in Markdown.

<!-- speccy:scenario id="CHK-001" -->
Given a fixture body containing `<T>` and `A & B`,
when the renderer emits it,
then the bytes are preserved without XML escaping.

```rust
fn example() -> &'static str {
    "<T> & friends"
}
```
<!-- /speccy:scenario -->
<!-- speccy:scenario id="CHK-002" -->
Given the second scenario under REQ-001,
when render walks `Requirement.scenarios` in order,
then this scenario follows CHK-001.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->

<!-- speccy:requirement id="REQ-002" -->
Second requirement, single scenario.

<!-- speccy:scenario id="CHK-003" -->
Given REQ-002 has one scenario,
when render emits it,
then CHK-003 is the only scenario under REQ-002.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->

<!-- speccy:decision id="DEC-001" status="accepted" -->
Use marker comments instead of raw XML containers around Markdown.
<!-- /speccy:decision -->

<!-- speccy:open-question resolved="false" -->
Should the root `speccy:spec` marker be required after migration?
<!-- /speccy:open-question -->

<!-- speccy:changelog -->
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-15 | human/kevin | Initial canonical fixture. |
<!-- /speccy:changelog -->
