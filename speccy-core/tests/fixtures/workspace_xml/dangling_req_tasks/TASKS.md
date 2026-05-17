---
spec: SPEC-0099
spec_hash_at_generation: deadbeef
generated_at: 2026-05-17T00:00:00Z
---

# Tasks: SPEC-0099 Dangling REQ in TASKS fixture

<tasks spec="SPEC-0099">

<task id="T-001" state="pending" covers="REQ-999">
Task covering a requirement the parent SPEC.md does not declare.

<task-scenarios>
Given a TASKS.md task whose `covers` names REQ-999,
when workspace cross-ref validation runs,
then a dangling-requirement diagnostic is surfaced.
</task-scenarios>
</task>

</tasks>
