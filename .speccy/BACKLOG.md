Speccy improvement backlog

Grouped by priority. Each item: what / why / where it lives / cost.

Tier 1 — do first (prompt/skill markdown only, no CLI surface change)

F-4: Hypothesis-driven debugging branch in speccy implement

- When a Check has previously failed in this task, the rendered prompt loads a "form hypothesis → write failing test that proves it → narrow" sub-template.
- Why: failing Checks should drive systematic debugging, not flailing. Maps [superpowers](https://github.com/obra/superpowers)' debugging discipline onto Check-as-evidence.
- Where: conditional branch in speccy implement prompt template.

Pre-existing tech debt (discovered during other work, blocks the hygiene gate)

Tier 2 — consider, needs design pass

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
