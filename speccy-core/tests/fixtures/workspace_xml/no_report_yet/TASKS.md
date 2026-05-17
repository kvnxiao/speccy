---
spec: SPEC-0099
spec_hash_at_generation: deadbeef
generated_at: 2026-05-17T00:00:00Z
---

# Tasks: SPEC-0099 In-flight, no REPORT yet

<tasks spec="SPEC-0099">

<task id="T-001" state="pending" covers="REQ-999">
Dangling REQ that must still fire even with REPORT absent.

<task-scenarios>
Given a TASKS task with covers REQ-999,
when validation runs and REPORT is absent,
then TASKS dangling-requirement validation still surfaces a diagnostic
and the missing-coverage check is skipped.
</task-scenarios>
</task>

</tasks>
