---
spec: SPEC-0022
spec_hash_at_generation: deadbeef
generated_at: 2026-05-17T00:00:00Z
---

# Tasks: SPEC-0022 Canonical TASKS.md fixture

<tasks spec="SPEC-0022">

<task id="T-001" state="pending" covers="REQ-001">
Implement the first slice.

<task-scenarios>
Given a task element with id, state, and covers,
when the parser runs,
then it returns a typed task carrying those values.
</task-scenarios>
</task>

<task id="T-002" state="in-review" covers="REQ-001 REQ-003">
Wire the typed model into the workspace loader.

This body contains escape-prone bytes: `<`, `>`, `&`. Inside a fenced
code block we keep literal element tokens so the scanner cannot
promote them to structure:

```markdown
<task id="T-FAKE" state="pending" covers="REQ-001">
literal text, not parsed
</task>
```

<task-scenarios>
Given a TASKS.md containing the canonical fixture,
when parse runs,
then it returns a `TasksDoc` whose tasks list field-equals the input.

```text
literal `<task>` inside backticks
```
</task-scenarios>
</task>

</tasks>
