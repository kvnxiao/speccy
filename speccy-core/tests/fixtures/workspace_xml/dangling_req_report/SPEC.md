---
id: SPEC-0099
slug: dangling-req-report
title: Dangling REQ in REPORT fixture
status: in-progress
created: 2026-05-16
---

# SPEC-0099: Dangling REQ in REPORT fixture

<goals>
Fixture SPEC for the workspace_xml cross-ref tests.
</goals>

<non-goals>
Not a real spec.
</non-goals>

<user-stories>
- As a test, I want one requirement so REPORT can dangle on REQ-999.
</user-stories>

<requirement id="REQ-001">
Sole requirement.

<done-when>
- The fixture parses.
</done-when>

<behavior>
- Given the fixture, REQ-001 is the only requirement.
</behavior>

<scenario id="CHK-001">
Given the fixture body,
when the parser walks requirements,
then REQ-001 is the only one.
</scenario>
</requirement>

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-16 | agent/claude | Initial fixture for SPEC-0022 T-004. |
</changelog>
