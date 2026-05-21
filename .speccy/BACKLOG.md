Speccy improvement backlog

Grouped by priority. Each item: what / why / where it lives / cost.

Tier 1 — do first (prompt/skill markdown only, no CLI surface change)

F-4: Hypothesis-driven debugging branch in speccy implement

- When a Check has previously failed in this task, the rendered prompt loads a "form hypothesis → write failing test that proves it → narrow" sub-template.
- Why: failing Checks should drive systematic debugging, not flailing. Maps [superpowers](https://github.com/obra/superpowers)' debugging discipline onto Check-as-evidence.
- Where: conditional branch in speccy implement prompt template.

F-12: For all our given skills and subagent resource prompt bodies and templates, can we do an inventory check to see how many of them have inlined examples that the LLM would use to produce a well-formed (i.e. speccy CLI lint-passing) document (for SPEC.md, TASKS.md, REPORT.md, etc.)? In the past few specs we've worked on, there were a few issues where /speccy-plan created a malformed SPEC.md or /speccy-tasks created a malformed TASKS.md that failed to validate from the speccy CLI linter - which required the agent to expend more tokens to address this. If we are missing instructions in our prompt templates, the LLM may not produce a well-formed document and always waste tokens to correct this output. Obviously, this is what speccy CLI is meant to do as a linter/validator for speccy artifacts, but we should strive to have all our prompt templates and examples produce well-formed documents from the start. Prefer to use progressive disclosure as well (i.e. the template body should reference an example file somewhere else within the resources folder).

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

F-9: Migrate inline examples in personas and prompts to progressive disclosure

- Migrate the remaining inline worked examples across `resources/modules/personas/*.md` and `resources/modules/prompts/*.md` to the progressive-disclosure pattern this SPEC established: eject the example body to a host-agnostic `.speccy/examples/<file>.md` resource and replace the inline body in the persona / prompt with a one-line pointer that names the file and instructs the host to Read it on first encounter.
- Why: per-invocation token cost. Each persona and prompt is rendered into every implementer or reviewer invocation, so duplicated example bodies bloat the rendered prompt on every loop iteration. Pattern established by SPEC-0031 (F-3 red-green paper trail), which ejected the evidence-file worked example into `.speccy/examples/evidence.md` and proved the host-agnostic Read pointer works identically across Claude Code and Codex skill packs.
- Where: the persona files under `resources/modules/personas/*.md` and the prompt files under `resources/modules/prompts/*.md`. Audit candidates by grepping for inline `## Worked example`, `## Example`, or fenced ` ```markdown ` blocks ≥ ~8 lines long.
- Heuristic / risk: eject when an inline example is ≥ ~8 lines OR is referenced by ≥ 2 consuming prompts; keep inline when it is a short shape sketch (≤ ~5 lines, one consumer). Risk is over-ejection — a tiny inline sketch that reads more clearly in place becomes a noisier pointer + a Read round-trip for no token savings.

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
