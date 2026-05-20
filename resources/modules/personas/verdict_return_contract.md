Your final message to the orchestrator **must** be a single
`<review persona="{{ persona_name }}" verdict="...">…</review>` element
block — structured enough for the orchestrator to parse without
ambiguity. On a `verdict="pass"` result, a one-line summary
suffices. On a `verdict="blocking"` result, include the `<retry>`
body text you want recorded against the task so the orchestrator
can aggregate it into the consolidated retry note.

{% include "modules/personas/no_tasks_md_writes.md" %}
