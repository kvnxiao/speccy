Speccy improvement backlog

Grouped by priority. Each item: what / why / where it lives / cost.

Tier 1 — do first (prompt/skill markdown only, no CLI surface change)

F-3: Red-green paper trail in task closure

- Implementer prompt requires captured red-output then green-output before flipping [~] → [?]. reviewer-tests treats absence or fabricated-looking output as blocking.
- Why: structural Check-mapping proves count, not order. Red-state visibility is the strongest non-mechanical TDD evidence — closes the gap between "Requirement has a Check" and "the Check was actually adversarial."
- Where: speccy implement + speccy review prompt templates.

F-4: Hypothesis-driven debugging branch in speccy implement

- When a Check has previously failed in this task, the rendered prompt loads a "form hypothesis → write failing test that proves it → narrow" sub-template.
- Why: failing Checks should drive systematic debugging, not flailing. Maps [superpowers](https://github.com/obra/superpowers)' debugging discipline onto Check-as-evidence.
- Where: conditional branch in speccy implement prompt template.

F-5: Per-skill model + effort pinning across the lifecycle

- Today every shipped skill and reviewer subagent inherits the session's model and effort. That wastes tokens (Sonnet-grade work pulling Opus rates) and time (Opus-grade work flailing at low effort).
- Pin each phase to the model + effort the work actually needs:
  - opus / max — `speccy-plan`, `speccy-amend` (contract-writing; lowest frequency, highest leverage)
  - opus / xhigh — `speccy-brainstorm`, `reviewer-business`, `reviewer-tests`, `reviewer-architecture` (semantic adversarial reasoning; matches Opus 4.7 default but explicit insulates against future default drift)
  - sonnet / high — `reviewer-security` (pattern-heavy with edge-case judgment)
  - sonnet / medium — `speccy-tasks`, `speccy-work`, `speccy-ship`, `reviewer-style`, `reviewer-docs` (bulk-volume phases; mechanical given a tight SPEC)
  - haiku / low — `speccy-init`, `speccy-review` orchestrator (pure scaffolding / JSON-parsing fan-out)
- Why: drift catching lives where Opus runs; volume lives where Sonnet runs; mechanics live where Haiku runs. Asymmetric reviewer assignment is deliberate — business + tests carry semantic load, security + style carry pattern load. Pinning all four to Opus burns ~2× tokens for marginal style/security gain.
- Where (Claude Code): `model:` + `effort:` frontmatter fields in every `.claude/skills/speccy-*/SKILL.md` and every `.claude/agents/reviewer-*.md`. Mirror into `resources/agents/.claude/{skills,agents}/` so `speccy init` ships the same assignments to new users.
- Where (Codex): open question. Codex skill frontmatter today is `name` + `description` only; per-skill model and `model_reasoning_effort` pinning is not currently exposed by the Codex CLI. Options to evaluate:
  - Document recommended `model` / `model_reasoning_effort` per skill in the SKILL.md body so a human can `/model` switch before invoking the skill — informational, not enforced.
  - Upstream a `model:` / `effort:` frontmatter knob to Codex skills (analogous to Claude Code's surface) and ship the parallel pinning once it lands.
  - Accept that Codex runs the whole loop at session-level model/effort until upstream catches up; document the asymmetry in the skill pack README so users know what they're trading.
- Risk: per-model effort ceilings (`xhigh` vs `max`) are not equally available across Opus / Sonnet / Haiku — verify each pin against the current model card before locking in. Also: pinning rather than inheriting means a user who wants to dogfood the loop on Sonnet-only (or a future model family) has to override every skill; consider whether the Codex documentation-only route is actually the right shape for Claude Code too.
- Cost: low for Claude Code (frontmatter edits across 8 skills × 2 trees + 6 agents × 2 trees = ~28 files, no code change). Unknown for Codex pending upstream.

Pre-existing tech debt (discovered during other work, blocks the hygiene gate)

F-7: Box the large `ParseError` variants in `speccy-core`

- `speccy-core::parse::error::ParseError` has variants ≥128 bytes (e.g. `MissingAttribute { allowed: String, ... }` and similar `String`-carrying variants). Workspace clippy denies `clippy::result_large_err`; the lint fires at 42+ sites across `speccy-core` (lib + lib tests) every time the parser returns a `Result<_, ParseError>`. Net effect: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (the third leg of AGENTS.md "Standard hygiene") is red on `main`; `cargo test`, `cargo +nightly fmt --check`, and `cargo deny check` all stay green.
- Why: blocks the hygiene gate. Confirmed pre-existing on the `6ed6e39 Ship SPEC-0025` baseline via `git stash` during SPEC-0026 T-003 implementation, so no recent SPEC has shipped with a clean clippy gate. The longer this sits, the more new sites get added on every parser tweak, and the more painful the fix becomes.
- Where: `speccy-core/src/parse/error.rs` (definition) and every call site that propagates `?` through `Result<_, ParseError>` — those mostly Just Work after the box because `Box<E>` propagates the same as `E` through `?`. A workspace-level `#[expect(clippy::result_large_err, reason = "...")]` is the escape hatch if boxing turns out to hurt readability for diagnostic-rich variants, but the cleaner fix is to box.
- Cost: small. Box the offending variants (or the whole enum) in `error.rs`, run `cargo clippy --workspace --all-targets --all-features -- -D warnings` until green, run `cargo test --workspace` to confirm no regressions. Likely under an hour. Worth doing before F-2 lands to PR (so SPEC-0026's commit is the one that finally unblocks the gate, not a separate cleanup-after).

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
