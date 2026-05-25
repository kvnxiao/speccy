---
id: SPEC-0099
slug: valid-post-migration
title: Valid post-migration fixture
status: in-progress
created: 2026-05-16
---

# SPEC-0099: Valid post-migration fixture

<goals>
Fixture SPEC for the workspace_xml cross-ref tests.
</goals>

<non-goals>
Not a real spec.
</non-goals>

<user-stories>
- As a test, I want REQ-001 and REQ-002 each with one scenario so the
  REPORT can cover both cleanly.
</user-stories>

<requirement id="REQ-001">
First requirement.

<done-when>
- The fixture parses.
</done-when>

<behavior>
- Given the fixture, REQ-001 exists.
</behavior>

<scenario id="CHK-001">
Given the fixture body,
when the parser walks scenarios,
then CHK-001 is under REQ-001.
</scenario>
</requirement>

<requirement id="REQ-002">
Second requirement.

<done-when>
- The fixture parses.
</done-when>

<behavior>
- Given the fixture, REQ-002 exists.
</behavior>

<scenario id="CHK-002">
Given the fixture body,
when the parser walks scenarios,
then CHK-002 is under REQ-002.
</scenario>
</requirement>

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-16 | agent/claude | Initial fixture for SPEC-0022 T-004. |
</changelog>
