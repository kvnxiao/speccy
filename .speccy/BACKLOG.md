Speccy improvement backlog

Grouped by priority. Each item: what / why / where it lives / cost.

Tier 1 — do first (prompt/skill markdown only, no CLI surface change)

F-1: Hypothesis-driven debugging branch in speccy implement

- When a Check has previously failed in this task, the rendered prompt loads a "form hypothesis → write failing test that proves it → narrow" sub-template.
- Why: failing Checks should drive systematic debugging, not flailing. Maps [superpowers](https://github.com/obra/superpowers)' debugging discipline onto Check-as-evidence.
- Where: conditional branch in speccy implement prompt template.

Pre-existing tech debt (discovered during other work, blocks the hygiene gate)

Tier 2 — consider, needs design pass

F-2: `speccy next` and `speccy check` both support passing in a `SPEC-####` as an immediate argument to filter down to a specific spec. However, our skills and subagent resource templates are currently not using this with `--json`, which causes the entire spec tree to be printed out. This makes the LLM waste context by having to potentially write a script to filter down the list manually when a user calls for `/speccy-orchestrate SPEC-####` for example. We need to change our instructions to use that optional `SPEC-####` filter argument with `--json`, when passed in, so the LLM can filter down to the specific spec without wasting context. Note that `SPEC-####` is optional — if not passed in, the LLM will still print the entire spec tree. E.g. `speccy next --json` - skill will print the whole spec tree in JSON format and pick the next available spec to implement. Whereas `speccy next SPEC-#### --json` - skill will filter down to the specific spec and print it in JSON format to figure out what's next in its development lifecycle - or alternatively print some error message if the spec is already implemented (this error clause may not exist today and may need to be added).

F-8: Strip implementer context from reviewer prompts

- Reviewer fan-out sees diff + SPEC + Checks only — not the implementer's task notes/rationale.
- Why: anchoring reviewers on the implementer's framing weakens the adversarial property.
- Risk: business reviewer may genuinely need rationale to judge intent. Exceptions worth keeping — this isn't a uniform strip.
- Where: per-persona review prompt templates.

F-6: Optional PreToolUse hook templates shipped by speccy init

- Commented-out hooks in generated .claude/settings.json that emit stderr warnings on:
  - Edit to SPEC.md while any task is [~] (suggest /speccy-amend)
  - Edit to spec.toml outside /speccy-plan or /speccy-amend
  - Edit to impl files before test files within a task's [~] window
  - git commit while any task is [~]
- Why: warnings (not blocks) fit Principle 1; host-side fits Principle 2.
- Risk: each hook is one more thing speccy-init can wreck — ship opt-in.

Tier 3 — reject

┌─────┬──────────────────────────────────────────┬────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ID │ Item │ Why reject │
├─────┼──────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ R-1 │ Meta skill-evolver skill │ Friction-to-skill-update is already in AGENTS.md; ceremony without payoff at v1 │
├─────┼──────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ R-2 │ Anti-sycophancy hooks │ Host concern; speccy shouldn't touch response style │
├─────┼──────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ R-3 │ Worktree-by-default │ Host concern; speccy must work identically in or out of a worktree │
├─────┼──────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ R-4 │ Commit-order TDD check │ Couples to git history, gameable by squash, adds CLI surface; F-3 gives 80% of the value at 5% of the cost │
├─────┼──────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ R-5 │ CLI surface for TDD ordering enforcement │ Behavioral discipline belongs in hooks (F-6) or prompts (F-3), not the deterministic core │
└─────┴──────────────────────────────────────────┴────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

Cross-cutting observation

F-1, F-3, F-4 all live in the rendered prompt templates inside the Rust CLI, not in new commands or new skills. The CLI surface is already the right size — the leverage is in making the rendered prompts stronger. Stay-small
principle holds.
