---
id: SPEC-0099
slug: dangling-scenario
title: Dangling CHK in REPORT fixture
status: in-progress
created: 2026-05-16
---

# SPEC-0099: Dangling CHK in REPORT fixture

<goals>
Fixture SPEC for the workspace_xml cross-ref tests.
</goals>

<non-goals>
Not a real spec.
</non-goals>

<user-stories>
- As a test, I want REQ-001 with one scenario CHK-001 so REPORT can
  dangle on CHK-099.
</user-stories>

<requirement id="REQ-001">
Sole requirement, one scenario.

<done-when>
- The fixture parses.
</done-when>

<behavior>
- Given the fixture, REQ-001 has CHK-001 only.
</behavior>

<scenario id="CHK-001">
Given the fixture body,
when the parser walks scenarios,
then CHK-001 is the only one under REQ-001.
</scenario>
</requirement>

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-16 | agent/claude | Initial fixture for SPEC-0022 T-004. |
</changelog>
