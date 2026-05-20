**Do not edit TASKS.md directly.** You are a subagent; TASKS.md
writes for review-induced state transitions are the orchestrator's
exclusive responsibility. Editing TASKS.md from inside this subagent
causes parallel-write races and splits the state transition across
two turns. Return your verdict via your final message; the
orchestrator applies the state transition.
