---
spec: SPEC-0023
spec_hash_at_generation: 87a0227fbb8b04b5145913e4bca8163746b82d1da86cfaedc6270ff61fbf5357
generated_at: 2026-05-17T07:39:42Z
---

# Tasks: SPEC-0023 Single-phase skill primitives for the development loop

<tasks spec="SPEC-0023">

<task id="T-001" state="completed" covers="REQ-001">
Rewrite `/speccy-work` skill body as a single-task primitive

- Suggested files: `resources/modules/skills/speccy-work.md`,
  `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`

<task-scenarios>
  - When `resources/modules/skills/speccy-work.md` is grep'd for
    `sub-agent`, `subagent`, `spawn`, `loop`, or `until no tasks`
    after this task, no active guidance hits. Historical references
    in changelog rows or comments are fine.
  - When the rewritten skill body is read, it describes one session
    implementing one task per invocation, accepts an optional
    `[SPEC-NNNN/T-NNN]` selector argument, and exits after one task
    without continuing to the next.
  - When the skill is invoked with no selector argument, the body
    tells the session to resolve the next implementable task via
    `speccy next --kind implement --json` and implement only that
    one.
  - When the rewritten skill body is read, the language is
    role-agnostic. There is no "main agent" / "sub-agent" framing;
    the same body is correct whether the caller is a human at the
    terminal, the existing `/loop` skill, or a future orchestrator.
  - When the skill body's exit transition is read, it tells the
    session to flip the task's `state="..."` attribute from
    `pending` / `in-progress` to `in-review` and append an
    implementer note using the handoff template the CLI's implement
    prompt already supplies.
  - When `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`
    and `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`
    are read after this task, their frontmatter `description:` text
    matches the single-task primitive contract and triggers on
    single-task intent phrases (e.g., "implement T-003", "work the
    next task", "run the implementer"). Neither mentions sub-agent
    spawning or multi-task loops.
</task-scenarios>

- Implementer note (session-t001):
  - Completed: Rewrote `resources/modules/skills/speccy-work.md` as a single-task primitive — one invocation, one task, optional `[SPEC-NNNN/T-NNN]` selector, no-arg path resolves via `speccy next --kind implement --json`, role-agnostic ("one session"), and exit transition names the `state="..."` flip plus the six-field handoff template the implementer prompt supplies. Updated both wrapper frontmatter `description:` strings (`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` and `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`) to match the new contract and trigger on phrases like "implement T-003", "work the next task", and "run the implementer".
  - Undone: (none)
  - Commands run: `grep -n -E 'sub-agent|subagent|spawn|loop|until no tasks' resources/modules/skills/speccy-work.md`; `grep -n -E 'sub-agent|subagent|spawn|loop|until no tasks' resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`; `grep -in 'loop' resources/modules/skills/speccy-work.md`
  - Exit codes: pass (only allowed hit: the proper-noun `/loop` reference in the role-agnostic "caller may be a human, /loop, or a future orchestrator" paragraph — that mirrors DEC-001 and is not active guidance to iterate); pass (no hits in wrappers); pass (single line, the same proper-noun reference)
  - Discovered issues: (none)
  - Procedural compliance: (none) — markdown-only task, no shipped-skill friction encountered; no `cargo test` / `cargo clippy` / `cargo fmt` run because no Rust code was touched.
</task>

<task id="T-002" state="completed" covers="REQ-002">
Rewrite `/speccy-review` skill body as a single-task primitive with bash-command persona fan-out

- Suggested files: `resources/modules/skills/speccy-review.md`,
  `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/speccy-review/SKILL.md.tmpl`

<task-scenarios>
  - When `resources/modules/skills/speccy-review.md` is grep'd for
    multi-task review framing (`until no in-review`, "drive a
    review loop", language asking the agent to iterate over
    `speccy next`) after this task, no active guidance hits. The
    skill body may still describe the within-task four-persona
    fan-out; that is intrinsic to the primitive and not multi-task
    orchestration.
  - When the rewritten skill body is read, it describes one round
    of adversarial review (four parallel personas on one task) per
    invocation, accepts an optional `[SPEC-NNNN/T-NNN]` selector,
    and exits after the one task without processing additional
    in-review tasks.
  - When the skill is invoked with no selector argument, the body
    tells the session to resolve the next reviewable task via
    `speccy next --kind review --json` and review only that one.
  - When the skill body describes the persona fan-out, each spawned
    persona sub-agent's prompt is the bash command form (e.g.,
    `` Run `speccy review <SPEC-NNNN/T-NNN> --persona <persona>` and
    follow its output. Your only deliverable is a single inline
    note appended to TASKS.md. ``); the CLI-rendered prompt text
    is not inlined into the spawn call.
  - When the skill body's exit transitions are read, they describe
    aggregating the four appended persona notes and flipping
    `in-review` → `completed` if all are pass, or `in-review` →
    `pending` plus a `Retry:` bullet summarising the blockers if
    any persona is blocking.
  - When `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`
    and the Codex parallel are read after this task, their
    frontmatter `description:` text matches the single-task review
    contract and triggers on single-task review intent phrases.
    Neither mentions multi-task review loops.
</task-scenarios>

- Implementer note (session-t002):
  - Completed: Rewrote `resources/modules/skills/speccy-review.md` as a single-task primitive — one invocation reviews one task, optional `[SPEC-NNNN/T-NNN]` selector, no-arg path resolves via `speccy next --kind review --json`, role-agnostic ("one session", caller may be a human / `/loop` / future orchestrator), within-task four-persona parallel fan-out (business, tests, security, style) preserved as intrinsic. Each spawned persona sub-agent's prompt is the bash command form (``Run `speccy review SPEC-NNNN/T-NNN --persona <persona>` and follow its output. Your only deliverable is a single inline note appended to TASKS.md.``) rather than the CLI-rendered prompt inlined. Exit transitions name the aggregate step: all-pass flips `in-review` → `completed`; any-blocking flips `in-review` → `pending` plus a `Retry:` bullet. Updated both wrapper frontmatter `description:` strings (`resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl` and `resources/agents/.agents/skills/speccy-review/SKILL.md.tmpl`) to match the single-task review contract and trigger on phrases like "review T-003", "review the next task", "run the reviewers on this task". Neither wrapper mentions multi-task review loops.
  - Undone: (none)
  - Commands run: `grep -nE 'until no in-review|drive a review loop|iterate over .speccy next.?|review loop' resources/modules/skills/speccy-review.md`; `grep -nE 'until no in-review|drive a review loop|iterate over .speccy next.?|review loop|multi-task' resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl resources/agents/.agents/skills/speccy-review/SKILL.md.tmpl`; `grep -n 'speccy review SPEC-NNNN/T-NNN --persona' resources/modules/skills/speccy-review.md`
  - Exit codes: pass (no hits — multi-task framing gone from the rewritten body); pass (no hits in wrappers); pass (bash command form present in both the claude-code and codex branches of the Jinja conditional, lines 63 and 74)
  - Discovered issues: (none)
  - Procedural compliance: (none) — markdown-only task, no shipped-skill friction encountered; no `cargo test` / `cargo clippy` / `cargo fmt` run because no Rust code was touched.

- Implementer note (session-t002-addendum):
  - Completed: Cleanup of three test guards in `speccy-cli/tests/skill_packs.rs` that encoded the pre-T-001/T-002 contracts and broke after the skill bodies and frontmatter were rewritten — `recipe_content_shape` (`LOOP_RECIPES` dropped speccy-work and speccy-review; speccy-amend remains the only loop recipe), `shipped_descriptions_natural_language_triggers` (trimmed the speccy-review frontmatter `description:` to 499 chars to fit the 500-char Codex display cap), and `speccy_review_skill_prefers_native_subagents` (retired the per-persona `--persona <name>` fallback-example requirement that REQ-002 deliberately replaced with the `--persona <persona>` placeholder spawn-prompt form; the persona-by-name presence assertions for `subagent_type:` and the prose `reviewer-<persona>` mentions remain unchanged). Also added the SPEC-0023 entry to `speccy-core/tests/fixtures/in_tree_id_snapshot.json` so `every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot` recognises the new spec. Re-ejected `.claude/skills/speccy-review/SKILL.md` and `.agents/skills/speccy-review/SKILL.md` via `cargo run -- init --force --host claude-code` and `--host codex` so `dogfood_outputs_match_committed_tree` stays green.
  - Undone: (none)
  - Commands run: `cargo test --workspace`; `cargo run -- init --force --host claude-code`; `cargo run -- init --force --host codex`
  - Exit codes: pass; pass; pass
  - Discovered issues: (none) — these were predictable test-guard updates that should have landed with T-001/T-002 since they encoded contracts those tasks retired.
  - Procedural compliance: cleanup landed under T-002 because the description-length and native-subagents guards were specifically about the speccy-review primitive contract T-002 reshaped. The shipped skill files (`resources/modules/skills/speccy-{work,review}.md` and both wrapper templates) needed no further edits.
</task>

<task id="T-003" state="completed" covers="REQ-003">
Reviewer CLI prompts stop inlining the branch diff

- Suggested files: `resources/modules/prompts/reviewer-architecture.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-docs.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/personas/reviewer-architecture.md`,
  `resources/modules/personas/reviewer-business.md`,
  `resources/modules/personas/reviewer-docs.md`,
  `resources/modules/personas/reviewer-security.md`,
  `resources/modules/personas/reviewer-style.md`,
  `resources/modules/personas/reviewer-tests.md`,
  `speccy-cli/src/review.rs`

<task-scenarios>
  - When each of the six `resources/modules/prompts/reviewer-*.md`
    templates is grep'd for `{{diff}}` or `{{ diff }}` after this
    task, no hit is returned.
  - When each `resources/modules/prompts/reviewer-*.md` template is
    read, it instructs the agent to run `git diff
    <merge-base>...HEAD -- <suggested-files>` itself, with
    `<merge-base>` resolved against the host's main branch and
    `<suggested-files>` derived from the task entry where the
    prompt can supply that hint.
  - When each `resources/modules/personas/reviewer-*.md` body is
    read, it agrees with the prompt: the persona fetches the diff
    via `git diff` rather than receiving it inline.
  - When the CLI renderer source under `speccy-cli/src/review.rs`
    is grep'd for the code path that previously computed and
    inserted the diff into the `{{diff}}` template variable, that
    code path is removed; the variable is no longer referenced
    anywhere in the resources tree or the CLI source.
  - When `speccy review SPEC-0022/T-001 --persona business`,
    `--persona tests`, `--persona security`, and `--persona style`
    are each run after this task, every rendered prompt is well
    under the 80,000-character render budget and does not contain
    a line beginning with `diff --git`. The CLI's render-budget
    guard message (`exceeds budget (80000 chars) after all
    drops`) does not appear on stdout or stderr for any of the
    four invocations.
  - When `cargo test --workspace`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, and `cargo
    +nightly fmt --all --check` run after this task, all three
    exit zero.
</task-scenarios>

- Implementer note (session-t003):
  - Completed: Removed the `{{diff}}` interpolation from all six `resources/modules/prompts/reviewer-*.md` templates and replaced the `## Diff under review` section with a one-line instruction that tells the reviewer agent to run `git diff <merge-base>...HEAD -- <suggested-files>` itself (with a defensive merge-base resolver that tries `origin/main`, `origin/master`, `main`, `master` in sequence, and a pathspec list extracted from the task entry's `Suggested files:`). Mirrored the contract to all six `resources/modules/personas/reviewer-*.md` bodies by adding a one-sentence "you fetch the diff yourself via `git diff <merge-base>...HEAD -- <suggested-files>` (the rendered prompt names the exact command); it is not inlined into the prompt." block to each `## Role` section. Removed the data flow end-to-end in `speccy-cli/src/review.rs`: dropped the `use crate::git::diff_for_review;` import, the `let diff = diff_for_review(&project_root);` site, and the `vars.insert("diff", diff);` template binding; updated the module doc comment to point at SPEC-0023 REQ-003. Removed the orphaned `diff_for_review`, `NO_DIFF_FALLBACK`, and `run_diff` helpers from `speccy-cli/src/git.rs` and deleted the now-dead `speccy-cli/tests/git_diff.rs` integration suite. Updated `speccy-cli/tests/review.rs`: dropped the `{{diff}}` placeholder assertion from `prompt_renders_substitutes_every_placeholder` and replaced the SPEC-0009 CHK-005 stub `diff_placeholder_is_substituted_with_fallback_outside_repo` with a new SPEC-0023 REQ-003 test `rendered_prompt_omits_inline_diff_and_instructs_git_fetch` that asserts no `{{diff}}` placeholder, no retired fallback note, no `diff --git` line, and the presence of the `git diff` instruction. Updated `speccy-cli/tests/skill_packs.rs`: removed `"diff"` from the reviewer-required placeholder list and added two SPEC-0023 REQ-003 asserts per persona template (no `{{diff}}`; contains `git diff`). Refreshed the committed dogfood mirrors under `.speccy/skills/personas/reviewer-*.md` and `.speccy/skills/prompts/reviewer-*.md` so they match the new embedded resources and the `dogfood_outputs_match_committed_tree` guard stays green.
  - Undone: (none) — all six scenarios under `<task-scenarios>` are satisfied for the changes within this task's scope. The `cargo test --workspace` requirement has three remaining failures in `speccy-cli/tests/skill_packs.rs` that are not caused by T-003 work; see Discovered issues.
  - Commands run:
    - `cargo run -- review SPEC-0022/T-001 --persona business` (pre-change baseline: 37,457 chars, 6 `diff --git` lines)
    - `cargo run -- review SPEC-0022/T-001 --persona business` (post-change: 21,263 chars, 0 `diff --git`, 0 budget-guard hits)
    - `cargo run -- review SPEC-0022/T-001 --persona tests` (post-change: 22,200 chars, 0 `diff --git`, 0 budget-guard hits)
    - `cargo run -- review SPEC-0022/T-001 --persona security` (post-change: 21,099 chars, 0 `diff --git`, 0 budget-guard hits)
    - `cargo run -- review SPEC-0022/T-001 --persona style` (post-change: 21,112 chars, 0 `diff --git`, 0 budget-guard hits)
    - `cargo run -- init --force --host claude-code` (refreshes `.claude/agents/reviewer-*.md`; `.speccy/skills/` entries skip per user-tunable contract)
    - `cargo run -- init --force --host codex` (refreshes `.codex/agents/reviewer-*.toml` and `.agents/skills/speccy-*/SKILL.md`)
    - `grep -rn "{{diff}}\|{{ diff }}" resources/modules/prompts/ speccy-cli/src/ speccy-core/src/` (no hits)
    - `cargo build --workspace`
    - `cargo test --workspace --test review`
    - `cargo test --workspace --test init`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all --check`
  - Exit codes: pass; pass; pass; pass; pass; pass; pass; pass (no hits); pass; pass (18/18 review tests); pass (21/21 init tests); fail (3 pre-existing failures in `skill_packs.rs` introduced by T-001/T-002 — see Discovered issues); pass; pass.
  - Discovered issues: `cargo test --workspace` exits non-zero because three tests in `speccy-cli/tests/skill_packs.rs` encode the pre-T-001/T-002 loop-recipe contract and are stale relative to the new single-task primitive design from T-001 and T-002. The failing tests are `recipe_content_shape` (requires `speccy-work/SKILL.md` and `speccy-review/SKILL.md` to contain "loop exit" or "exit criteria" because `LOOP_RECIPES` still lists them; T-001 and T-002 deliberately removed that framing), `shipped_descriptions_natural_language_triggers` (caps wrapper `description:` at 500 chars; T-002's new `speccy-review` wrapper description is 639 chars due to the longer trigger list it added), and `speccy_review_skill_prefers_native_subagents` (requires the `speccy-review` SKILL.md to contain explicit `--persona business|tests|security|style` fallback examples that T-002 removed from the body). None of these failures are caused by T-003 work; reproduction is independent of the `{{diff}}` removal. The matching `LOOP_RECIPES` / `MAX_DESCRIPTION_CHARS` / `--persona X` invariants need to be re-pointed at the new single-task primitive contract (this is in-scope for T-001/T-002 cleanup or a follow-up task; out of scope for T-003's surgical surface).
  - Procedural compliance: (none) — no shipped skill files needed updating for friction encountered during this task. The implementer prompt rendered cleanly; the `git diff` workflow the new reviewer prompts now describe was reachable from the implementer's own scope without re-tooling any skill body.
</task>

<task id="T-004" state="completed" covers="REQ-004">
Update ARCHITECTURE.md Phase 3 and Phase 4 to reflect the primitive contract

- Suggested files: `.speccy/ARCHITECTURE.md`

<task-scenarios>
  - When `.speccy/ARCHITECTURE.md` §"Core Development Loop" is
    read after this task, it describes the five phases without
    implying that phases 3 and 4 are skill-driven loops; phases
    3 and 4 describe one primitive invocation each (one task in,
    one state transition out).
  - When `.speccy/ARCHITECTURE.md` §"Phase 3" and §"Phase 4" are
    read after this task, the previous loop pseudocode is no
    longer present (deleted, not relabelled).
  - When `.speccy/ARCHITECTURE.md` is grep'd for `loop`, `spawn`,
    `sub-agent`, or `subagent` in the context of Phase 3 or Phase
    4 after this task, active guidance does not present these as
    the skill's job. The Phase 4 description may name the
    within-task four-persona fan-out as the one intrinsic
    sub-agent use.
  - When the end of §"Phase 3" and the end of §"Phase 4" are read
    after this task, each carries a one-to-two-sentence note that
    multi-task composition is a future Layer-2 concern not built
    today, pointing at the existing `/loop` skill as the interim
    composer.
  - When `.speccy/ARCHITECTURE.md`, the rewritten
    `resources/modules/skills/speccy-work.md`, and the rewritten
    `resources/modules/skills/speccy-review.md` are read together,
    they agree on the single-task primitive contract. Active
    guidance across the three files does not contradict on
    orchestration vocabulary.
  - When `speccy verify` runs after this task, it exits zero and
    no new lints fire as a result of the ARCHITECTURE.md edits.
</task-scenarios>

- Implementer note (session-t004):
  - Completed: Rewrote `.speccy/ARCHITECTURE.md` §"Core Development Loop" (renamed the framing block from "alternating between planning and agent-orchestrated loops" to "Phases 3 and 4 are single-task primitives", retitled rows 3/4 in the ASCII summary as `implement`/`review` with single-task descriptions, and replaced the closing sentence about "loops live in the harness or skills" with explicit naming of the caller layer — human at the terminal, `/loop` skill, or future orchestrator). Rewrote §"Phase 3: Implementation (single-task primitive)" — deleted the prior `loop:` pseudocode block, replaced it with prose describing one selector resolution, the state flips, the test/check workflow, the six-field handoff note, and a closing two-sentence note pointing at the `/loop` skill as the interim composer. Same shape for §"Phase 4: Review (single-task primitive)" — deleted the prior `loop:` pseudocode block, replaced it with prose covering the within-task four-persona fan-out (named intrinsic per DEC-002), aggregate exit transition (all-pass vs any-blocking), preserved the default fan-out paragraph, and added a closing two-sentence note pointing at the `/loop` skill. Reconciled three adjacent table/list rows that named loop semantics: the "What ships in v1" tree (line ~1523 — `speccy-work.md` / `speccy-review.md` rows now read "Implement one task (single-task primitive)" / "Review one task (single-task primitive)"), the workflow recipes bullet list (lines ~1577-1578 — `/speccy:work` and `/speccy:review` now read "Phase 3 (implement one task)" / "Phase 4 (review one task)"), and the "typical full session" code block (lines ~1583-1602 — replaced the multi-task loop prose with single-task invocation pairs plus a note that the caller re-invokes for additional tasks). Also touched two role-framing lines outside Phase 3/4 to align with DEC-001's role-agnostic principle: the "Who sets it" cell for `state="completed"` in the State Model table (line 412, "Main agent after review loop" → "Reviewer skill at exit of review primitive") and the "State transitions" paragraph (line ~1398, replaced "main agent's `/speccy:review` skill" with neutral "skill session" framing).
  - Undone: (none)
  - Commands run: `grep -nE "loop|spawn|sub-agent|subagent" .speccy/ARCHITECTURE.md | grep -iE "phase 3|phase 4"`; `cargo run -- verify`
  - Exit codes: pass (empty output — no active-guidance hits naming loop/spawn/sub-agent as the skill's job in Phase 3 / Phase 4 contexts); pass (`Lint: 0 errors, 21 warnings, 47 info; verified 23 specs, 126 requirements, 164 scenarios; 0 errors` — no new lints attributable to the edits)
  - Discovered issues: (none) — Phase 5 still contains the sentence "When `speccy next` returns empty for both `--kind` values, the loop is complete", and the principles section above the design retains references to "the loops live in skills". Both are about the overall development arc, not Phase 3/4 active guidance, and don't contradict the primitive contract; left untouched per surgical-changes constraint.
  - Procedural compliance: (none) — markdown-only task, no shipped-skill friction encountered; no `cargo test` / `cargo clippy` / `cargo fmt` run because no Rust code was touched.
</task>

<task id="T-005" state="completed" covers="REQ-005">
All CLI prompts stop inlining AGENTS.md

- Suggested files: `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/plan-amend.md`,
  `resources/modules/prompts/plan-greenfield.md`,
  `resources/modules/prompts/report.md`,
  `resources/modules/prompts/reviewer-architecture.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-docs.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/tasks-amend.md`,
  `resources/modules/prompts/tasks-generate.md`,
  `speccy-core/src/prompt/agents_md.rs`,
  `speccy-core/src/prompt/mod.rs`,
  `speccy-cli/src/plan.rs`, `speccy-cli/src/tasks.rs`,
  `speccy-cli/src/implement.rs`, `speccy-cli/src/review.rs`,
  `speccy-cli/src/report.rs`

<task-scenarios>
  - When each of the twelve `resources/modules/prompts/*.md`
    templates is grep'd for `{{agents}}` or `{{ agents }}` after
    this task, no hit is returned.
  - When each prompt template is read, the `## Project
    conventions` heading that wrapped `{{agents}}` is removed
    entirely (no empty section left behind).
  - When the CLI renderer source under `speccy-cli/src/` and
    `speccy-core/src/prompt/` is grep'd for `load_agents_md`,
    `agents_md`, or the code path that filled the `{{agents}}`
    template variable, that code path is removed; the variable
    is no longer recognised by the renderer and the
    `agents_md.rs` module (or its `load_agents_md` export) is
    gone.
  - When `speccy plan` (greenfield form), `speccy plan
    SPEC-0022` (amendment form), `speccy tasks SPEC-0022`,
    `speccy implement SPEC-0022/T-001`, `speccy review
    SPEC-0022/T-001 --persona business`, and `speccy report
    SPEC-0022` are run after this task, none of the rendered
    outputs contains the literal `## Product north star`
    heading from `AGENTS.md`.
  - When the rendered implementer prompt for `SPEC-0022/T-001`
    is measured before and after this task lands, the post-task
    output is smaller than the pre-task output by at least the
    size of `AGENTS.md` on this repo.
  - When `cargo test --workspace`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, and `cargo
    +nightly fmt --all --check` run after this task, all three
    exit zero.
</task-scenarios>

- Implementer note (session-t005):
  - Completed: Removed the `{{agents}}` interpolation and the wrapping `## Project conventions` heading from all twelve `resources/modules/prompts/*.md` templates (`implementer.md`, `plan-amend.md`, `plan-greenfield.md`, `report.md`, `tasks-amend.md`, `tasks-generate.md`, and the six `reviewer-*.md`); the `plan-greenfield.md` retitled-section sentence about the north star being carried in `AGENTS.md` was also dropped because the heading it referenced is gone and modern hosts auto-load `AGENTS.md`. Deleted `speccy-core/src/prompt/agents_md.rs` entirely and the `pub mod agents_md;` declaration plus the `pub use agents_md::load_agents_md` re-export from `speccy-core/src/prompt/mod.rs`; updated the module-level doc comment to reflect the six-helper count and to point at SPEC-0023 REQ-005. Removed the `use speccy_core::prompt::load_agents_md;` import, the `let agents = load_agents_md(...);` call site, and the `vars.insert("agents", agents);` template binding from all five CLI callers (`speccy-cli/src/plan.rs` — both `render_greenfield` and `render_amendment`; `speccy-cli/src/tasks.rs` — both `render_initial` and `render_amendment`, plus dropped the now-unused `project_root: &Utf8Path` parameter from both helpers since the only AGENTS.md-flow argument that needed the root is gone; `speccy-cli/src/implement.rs`; `speccy-cli/src/review.rs`; `speccy-cli/src/report.rs`). Updated each touched CLI file's module doc comment to point at SPEC-0023 REQ-005. Refreshed the committed dogfood mirrors under `.speccy/skills/prompts/*.md` (twelve files) so `dogfood_outputs_match_committed_tree` stays green; `cargo run -- init --force --host claude-code` and `--host codex` were also run to refresh the host-local mirrors. Updated test guards that encoded the old AGENTS.md contract: deleted `speccy-core/tests/prompt_agents_md.rs` (the entire integration suite was about the retired loader); flipped the `loads_plan_greenfield_template_from_embedded_bundle` and `loads_plan_amend_template_from_embedded_bundle` checks in `speccy-core/tests/prompt_template.rs` to negative assertions; flipped the `loads_plan_greenfield_template` unit test in `speccy-core/src/prompt/template.rs` to a negative assertion; in `speccy-cli/tests/plan.rs` replaced `greenfield_renders_agents_and_next_spec_id` (renamed to `greenfield_renders_next_spec_id`) plus added a new `greenfield_does_not_inline_agents_md` negative test that writes a sentinel AGENTS.md and asserts it does not appear in the rendered prompt, dropped `greenfield_missing_agents_warns_but_still_renders`, and added the `#![expect(clippy::panic_in_result_fn, reason = ...)]` attribute the new test required; in `speccy-cli/tests/implement.rs` flipped the `agents placeholder missing` assertion in `prompt_renders_substitutes_every_placeholder` to a negative pair (no AGENTS.md body, no retired `{{agents}}` placeholder), removed `prompt_renders_with_missing_agents_md_succeeds_with_marker`, and rewrote `prompt_single_pass_does_not_substitute_placeholders_inside_scenario_body` to assert top-level `{{task_id}}` substitution (live) while the literal `{{agents}}` text inside the scenario body survives verbatim (single-pass invariant still pinned); in `speccy-cli/tests/review.rs` flipped the `agents missing` assertion in `prompt_renders_substitutes_every_placeholder` to a negative pair; in `speccy-cli/tests/report.rs` flipped the `agents placeholder missing` assertion in `prompt_renders_and_integration_substitutes_every_placeholder` to a negative pair and removed `prompt_renders_and_integration_missing_agents_md_leaves_marker`; in `speccy-cli/tests/tasks.rs` flipped `initial_prompt_rendered_when_tasks_md_absent` to use a sentinel AGENTS.md and assert non-appearance in the rendered output; in `speccy-cli/tests/skill_packs.rs` retitled the `prompt_placeholders_match_commands` test's required-placeholder lists (dropped `"agents"` from every per-template list and added a workspace-wide negative loop asserting `{{agents}}` does not appear in any of the twelve templates). The `agents_md_friction_paragraph` test in `skill_packs.rs` stays as-is because it checks AGENTS.md's *content* (the friction phrase the implementer prompt cross-references), not the retired CLI loader flow.
  - Undone: (none) — all six scenarios under `<task-scenarios>` are satisfied.
  - Commands run:
    - `cargo run --quiet -- implement SPEC-0022/T-001 | wc -c` (pre-change baseline: 20,938 chars)
    - `wc -c AGENTS.md` (8,367 bytes — the savings floor)
    - `grep -rn '{{agents}}\|{{ agents }}' resources/modules/prompts/` (no hits)
    - `grep -rn 'load_agents_md\|agents_md\b' speccy-cli/src/ speccy-core/src/` (no hits)
    - `grep -rn '"agents"' speccy-cli/src/ speccy-core/src/` (two hits in `speccy-cli/src/embedded.rs`, both naming the `resources/agents/` filesystem subtree; not the retired AGENTS.md loader path)
    - `cargo build --workspace`
    - `cp resources/modules/prompts/*.md .speccy/skills/prompts/` (refresh dogfood mirror)
    - `cargo run --quiet -- init --force --host claude-code`
    - `cargo run --quiet -- init --force --host codex`
    - `cargo test --workspace`
    - `cargo run --quiet -- plan | grep -c 'Product north star'` (0)
    - `cargo run --quiet -- plan SPEC-0022 | grep -c 'Product north star'` (0)
    - `cargo run --quiet -- tasks SPEC-0022 | grep -c 'Product north star'` (0)
    - `cargo run --quiet -- implement SPEC-0022/T-001 | grep -c 'Product north star'` (0)
    - `cargo run --quiet -- review SPEC-0022/T-001 --persona business | grep -c 'Product north star'` (0)
    - `cargo run --quiet -- report SPEC-0022 | grep -c 'Product north star'` (0)
    - `cargo run --quiet -- implement SPEC-0022/T-001 | wc -c` (post-change: 12,545 chars; savings 8,393 ≥ 8,367 AGENTS.md size)
    - `cargo run --quiet -- review SPEC-0022/T-001 --persona business | wc -c` (12,722 chars; well under 80,000 render budget)
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all --check`
    - `cargo run --quiet -- verify`
  - Exit codes: pass (measurement); pass (measurement); pass (no hits); pass (no hits); pass (only the resources-path hits, both expected); pass; pass; pass; pass; pass (all suites green, including `dogfood_outputs_match_committed_tree` after the mirror refresh); pass (0); pass (0); pass (0); pass (0); pass (0); pass (0); pass (savings exceed AGENTS.md size); pass (under budget); pass; pass; pass (`Lint: 0 errors, 21 warnings, 47 info; verified 23 specs, 126 requirements, 164 scenarios; 0 errors`).
  - Discovered issues: Several test guards encoded the old AGENTS.md contract — `speccy-core/tests/prompt_agents_md.rs` (entire suite testing the retired loader), one positive assertion in `speccy-core/tests/prompt_template.rs` and one in `speccy-core/src/prompt/template.rs` (positive `body.contains("{{agents}}")` checks), and positive `"Agents conventions go here"` substring assertions in `speccy-cli/tests/{plan,implement,review,report,tasks}.rs` plus a placeholder list in `speccy-cli/tests/skill_packs.rs`. Per the task instructions ("if a test guard refers to AGENTS.md loading or to `{{agents}}` (e.g., asserts a prompt contains AGENTS.md content), it encodes the OLD contract and needs updating"), these were surgically flipped to negative assertions pinning the new contract. No silent workarounds. No new bugs found in adjacent code.
  - Procedural compliance: (none) — no shipped skill file under `skills/` or `resources/agents/` needed updating for friction encountered during this task. The implementer prompt rendered cleanly and the steps were directly executable; no host-skill instructions surfaced as wrong or stale.
</task>

<task id="T-006" state="completed" covers="REQ-006">
All CLI prompts use file references for SPEC.md, TASKS.md, and MISSION.md

- Suggested files: `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/plan-amend.md`,
  `resources/modules/prompts/report.md`,
  `resources/modules/prompts/reviewer-architecture.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-docs.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/tasks-amend.md`,
  `resources/modules/prompts/tasks-generate.md`,
  `speccy-core/src/prompt/mission_md.rs`,
  `speccy-core/src/prompt/mod.rs`,
  `speccy-cli/src/plan.rs`, `speccy-cli/src/tasks.rs`,
  `speccy-cli/src/implement.rs`, `speccy-cli/src/review.rs`,
  `speccy-cli/src/report.rs`

<task-scenarios>
  - When each of the eleven prompt templates that previously
    interpolated `{{spec_md}}` (implementer, plan-amend, report,
    the six reviewer-*, tasks-generate, tasks-amend) is grep'd
    for `{{spec_md}}` or `{{ spec_md }}` after this task, no hit
    is returned.
  - When `resources/modules/prompts/report.md` and
    `resources/modules/prompts/tasks-amend.md` are grep'd for
    `{{tasks_md}}` or `{{ tasks_md }}` after this task, no hit
    is returned.
  - When `resources/modules/prompts/plan-amend.md` is grep'd for
    `{{mission}}` or `{{ mission }}` after this task, no hit is
    returned.
  - When the CLI renderer source is grep'd for the code paths
    that filled the `{{spec_md}}`, `{{tasks_md}}`, and
    `{{mission}}` template variables (e.g. `vars.insert("spec_md",
    ...)`, `vars.insert("tasks_md", ...)`, `vars.insert("mission",
    ...)` and the MISSION.md loader call sites), those code
    paths are removed; the variables are no longer recognised by
    the renderer.
  - When `speccy implement SPEC-0022/T-001` or `speccy review
    SPEC-0022/T-001 --persona business` is run after this task,
    the rendered output contains a short instruction naming the
    repo-relative path to SPEC.md
    (`.speccy/specs/0022-xml-canonical-tasks-report/SPEC.md`) and
    does not contain the SPEC body text. The same path
    resolution works for any other spec under
    `.speccy/specs/` (with or without a focus parent directory).
  - When `speccy plan SPEC-0022` (amendment form) is run on a
    spec whose focus has no `MISSION.md`, the rendered output
    contains no Read instruction for a non-existent MISSION.md.
    When run on a spec whose focus does have a `MISSION.md`, the
    rendered output names the MISSION.md path and instructs the
    agent to read it.
  - When `speccy report SPEC-0022` is run after this task, the
    rendered output instructs the agent to read TASKS.md at its
    repo-relative path rather than inlining the body.
  - When the rendered implementer prompt for `SPEC-0022/T-001`
    is measured before and after this task lands, the post-task
    output is smaller than the pre-task output by at least the
    size of `SPEC.md` for that spec.
  - When `cargo test --workspace`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, and `cargo
    +nightly fmt --all --check` run after this task, all three
    exit zero.
</task-scenarios>

- Implementer note (session-t006):
  - Completed: Dropped the `{{spec_md}}`, `{{tasks_md}}`, and `{{mission}}` interpolations from all eleven `resources/modules/prompts/*.md` templates that carried them (`implementer.md`, `plan-amend.md`, `report.md`, six `reviewer-*.md`, `tasks-amend.md`, `tasks-generate.md`). Each retired `## SPEC (full)` / `## TASKS (full)` / `## Mission context` heading was replaced with a short pointer section that names the file's repo-relative path via two new placeholders (`{{spec_md_path}}`, `{{tasks_md_path}}`) plus a third (`{{mission_read_instruction}}`) that the CLI fills with either a Read sentence or an empty string. Removed `speccy-core/src/prompt/mission_md.rs` entirely and the `pub mod mission_md;` declaration plus the `pub use mission_md::find_nearest_mission_md` re-export from `speccy-core/src/prompt/mod.rs`; updated the module-level doc comment to reflect the five-helper count and to point at SPEC-0023 REQ-006. Extended `speccy_core::task_lookup::TaskLocation` with a `spec_dir: &Utf8Path` field so the CLI commands can derive the repo-relative path without re-walking the workspace. Updated every CLI caller (`speccy-cli/src/plan.rs`, `tasks.rs`, `implement.rs`, `review.rs`, `report.rs`) to remove the SPEC.md / TASKS.md content-loading paths and to bind the new path placeholders; `plan.rs` carries an in-file `find_nearest_mission_md_path` helper that walks upward from the spec dir to the nearest enclosing `MISSION.md` and returns `Option<Utf8PathBuf>`, then formats a one-sentence Read instruction (empty string when no MISSION.md exists). Refreshed the committed dogfood mirrors under `.speccy/skills/prompts/*.md` (twelve files) so `dogfood_outputs_match_committed_tree` stays green; `cargo run -- init --force --host claude-code` and `--host codex` were also run to refresh the host-local mirrors. Surgically updated test guards that encoded the old contract: deleted `speccy-core/tests/prompt_mission_md.rs` (the entire integration suite was about the retired loader); flipped `loads_plan_amend_template` (unit) and `loads_plan_amend_template_from_embedded_bundle` (integration) to negative assertions plus positive assertions on the new `{{spec_md_path}}` / `{{mission_read_instruction}}` placeholders; rewrote three `speccy-cli/tests/plan.rs` tests (`amend_form_*`) to assert path-naming and SPEC body absence rather than SPEC body inlining; rewrote three `speccy-cli/tests/implement.rs` tests (`prompt_renders_substitutes_every_placeholder`, `prompt_does_not_inline_spec_md_body_for_any_requirement`, `prompt_single_pass_substitution_invariant_at_top_level_after_spec_body_retirement`, `prompt_does_not_inline_spec_md_when_parse_fails`) — the SPEC body bytes the old SPEC-0019/SPEC-0020 slicer tests pinned are no longer rendered, so they were collapsed into negative-assertion equivalents that pin SPEC-0023 REQ-006's "path only, body never" contract; replaced three `speccy-cli/tests/review.rs` tests (`reviewer_tests_scenario_text_equals_marker_body_bytes`, `reviewer_tests_multi_paragraph_scenario_body_renders_verbatim`, `reviewer_prompt_falls_back_to_raw_spec_md_when_parse_fails`) with a single `reviewer_prompt_does_not_inline_spec_body_for_grouped_spec` negative-assertion test that pins the same contract on the reviewer side; updated `prompt_renders_substitutes_every_placeholder` in `review.rs` and `report.rs` and `initial_prompt_rendered_when_tasks_md_absent` / `amendment_prompt_rendered_when_tasks_md_present` in `tasks.rs` to assert path-naming and SPEC/TASKS body absence; updated the per-template placeholder lists in `speccy-cli/tests/skill_packs.rs` (swapped `spec_md` for `spec_md_path`, `tasks_md` for `tasks_md_path`, `mission` for `mission_read_instruction`) and added a workspace-wide negative loop asserting `{{spec_md}}`, `{{tasks_md}}`, and `{{mission}}` do not appear in any of the twelve templates. The renderer-side stale-test surface is now empty.
  - Undone: (none) — every scenario under `<task-scenarios>` is satisfied for the changes within this task's scope, except the savings-floor scenario; see Discovered issues for the measurement discrepancy.
  - Commands run:
    - `cargo run --quiet -- implement SPEC-0022/T-001 | wc -c` (pre-task baseline: 12,545 chars, captured before T-006 edits)
    - `wc -c .speccy/specs/0022-xml-canonical-tasks-report/SPEC.md` (20,297 bytes)
    - `grep -rn '{{spec_md}}\|{{ spec_md }}\|{{tasks_md}}\|{{ tasks_md }}\|{{mission}}\|{{ mission }}' resources/modules/prompts/` (no hits)
    - `grep -rn '"spec_md"\|"tasks_md"\|"mission"' speccy-cli/src/ speccy-core/src/` (no hits)
    - `cargo build --workspace`
    - `cp resources/modules/prompts/*.md .speccy/skills/prompts/` (refresh dogfood mirror)
    - `cargo run --quiet -- init --force --host claude-code`
    - `cargo run --quiet -- init --force --host codex`
    - `cargo test --workspace`
    - `cargo run --quiet -- implement SPEC-0022/T-001 | wc -c` (post-task: 9,081 chars; savings 3,464; SPEC.md slice was the only previously-inlined content and the slicer for REQ-003 alone was smaller than 20,297 — see Discovered issues)
    - `cargo run --quiet -- review SPEC-0022/T-001 --persona business | wc -c` (9,304 chars)
    - `cargo run --quiet -- report SPEC-0022 | head` (path-naming confirmed for SPEC.md and TASKS.md)
    - `cargo run --quiet -- plan SPEC-0022 | head` (path-naming confirmed for SPEC.md; empty Mission context section because the spec is flat / no MISSION.md exists on disk)
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all --check`
    - `cargo run --quiet -- verify` (`Lint: 0 errors, 21 warnings, 47 info; verified 23 specs, 126 requirements, 164 scenarios; 0 errors`)
  - Exit codes: pass (measurement); pass (measurement); pass (no hits); pass (no hits); pass; pass; pass; pass; pass (all suites green, including `dogfood_outputs_match_committed_tree` after the mirror refresh); pass (post-task measurement); pass (within-budget); pass; pass; pass; pass; pass.
  - Discovered issues: The `<task-scenarios>` bullet "post-task output is smaller than pre-task output by at least the size of `SPEC.md` for that spec" is unsatisfiable as written for `SPEC-0022/T-001`. Pre-task implementer prompt was 12,545 chars (under T-005's AGENTS.md retirement); SPEC-0022's SPEC.md is 20,297 bytes; full pre-task prompt was already smaller than the bound named in the scenario. The reason is that the implementer prompt before T-006 inlined the **task-scoped slice** of SPEC.md (only the requirements named in `Covers:`, plus decisions), not the full SPEC.md body. For SPEC-0022/T-001 covering only REQ-003, the slice was small relative to the full SPEC.md. The realised post-task savings is 3,464 chars — the entire former slice for REQ-003 plus the surrounding heading and pointer prose, which is the maximum possible savings for this task. The scenario's "size of SPEC.md" bound is the *upper bound* on theoretical savings (true only for specs where every requirement is covered by one task), not a floor the renderer can satisfy across all task-scope sizes. No code-path remediation: the savings is exactly what dropping SPEC.md inlining buys here. Test guards that encoded the SPEC-0019/SPEC-0020 slicer contract surfaced as expected and were flipped per the task-prompt instruction; no other code-level surprises.
  - Procedural compliance: (none) — no shipped skill file under `skills/` or `resources/agents/` needed updating for friction encountered during this task. The implementer prompt rendered cleanly and the steps were directly executable; no host-skill instructions surfaced as wrong or stale.
</task>

<task id="T-007" state="completed" covers="REQ-001 REQ-002">
Re-eject host-local skill files to match the new shipped resources

- Suggested files: `.claude/skills/speccy-work/SKILL.md`,
  `.claude/skills/speccy-review/SKILL.md`,
  `.agents/skills/speccy-work/SKILL.md`,
  `.agents/skills/speccy-review/SKILL.md` (each regenerated by
  running `cargo run -- init --force` in this repo)

<task-scenarios>
  - When `cargo run -- init --force` is run in this repo after
    T-001 and T-002 land, the host-local
    `.claude/skills/speccy-work/SKILL.md`,
    `.claude/skills/speccy-review/SKILL.md`,
    `.agents/skills/speccy-work/SKILL.md`, and
    `.agents/skills/speccy-review/SKILL.md` files match the new
    single-task-primitive shapes shipped under
    `resources/modules/skills/` (byte-equal modulo the rendering
    pipeline's deterministic template substitutions).
  - When the re-ejected skill files are grep'd for orchestration
    vocabulary (`sub-agent`, `subagent`, `spawn`, `loop`, `until
    no tasks`) after this task, no active guidance hits.
  - When the re-ejected wrapper frontmatter `description:` text
    is read, it matches the single-task primitive contract from
    T-001 and T-002 and triggers on single-task intent phrases.
  - When `cargo test --workspace`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, and `cargo
    +nightly fmt --all --check` run after the re-eject, all
    three exit zero. The
    `dogfood_outputs_match_committed_tree`-style guard (if one
    exists) stays green.
</task-scenarios>

- Implementer note (session-t007):
  - Completed: Ran `cargo run -- init --force --host claude-code` and `cargo run -- init --force --host codex` to regenerate the four host-local wrapper files (`.claude/skills/speccy-{work,review}/SKILL.md` and `.agents/skills/speccy-{work,review}/SKILL.md`). The re-ejected wrappers match the new single-task primitive shapes shipped under `resources/modules/skills/` and the `description:` frontmatter from T-001 / T-002 (Claude Code and Codex variants render the appropriate Jinja branch). The within-task four-persona fan-out language in speccy-review is preserved as intrinsic per DEC-002. Note: the earlier T-002-addendum cleanup and the T-005 / T-006 implementer agents had already eagerly run `init --force` after their own edits to keep the dogfood guard green; this task's re-eject was therefore a no-op in terms of additional file changes, but is still recorded as the explicit final verification step the SPEC's approach calls for.
  - Undone: (none)
  - Commands run: `cargo run -- init --force --host claude-code`; `cargo run -- init --force --host codex`; `grep -nE 'sub-agent|subagent|spawn|loop|until no tasks' .claude/skills/speccy-work/SKILL.md .agents/skills/speccy-work/SKILL.md`; `grep -nE 'sub-agent|subagent|spawn|loop|until no tasks' .claude/skills/speccy-review/SKILL.md .agents/skills/speccy-review/SKILL.md`; `grep -E '^description:' .claude/skills/speccy-{work,review}/SKILL.md .agents/skills/speccy-{work,review}/SKILL.md`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all --check`
  - Exit codes: pass (init); pass (init); pass (only hit is the proper-noun `/loop` reference in the role-agnostic caller paragraph from DEC-001 — not active guidance); pass (only hits are the within-task four-persona fan-out language explicitly allowed by DEC-002 and by the T-002 scenarios); pass (all four frontmatter descriptions match the single-task primitive contract from T-001/T-002); pass (584 tests passed, 0 failed across the workspace including `dogfood_outputs_match_committed_tree`); pass; pass.
  - Discovered issues: (none) — the re-eject completed cleanly with `0 created, 14 overwritten, 20 skipped` for both hosts (the 20 skipped are user-tunable files marked `skip` by the init contract).
  - Procedural compliance: (none) — no shipped skill file under `skills/` or `resources/agents/` needed updating; the workflow executed without friction.
</task>

</tasks>
