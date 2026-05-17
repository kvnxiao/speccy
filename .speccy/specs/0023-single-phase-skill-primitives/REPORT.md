---
spec: SPEC-0023
outcome: delivered
generated_at: 2026-05-17T00:00:00Z
---

# Report: SPEC-0023 Single-phase skill primitives for the development loop

## Outcome

delivered

`/speccy-work` and `/speccy-review` are now single-task primitives:
one invocation, one task, fresh context, exit. The CLI's rendered
prompts no longer inline the branch diff, `AGENTS.md`, `SPEC.md`,
`TASKS.md`, or `MISSION.md`; each prompt names the file's
repo-relative path and instructs the agent to read it via the host's
Read primitive. The four-persona parallel fan-out inside
`/speccy-review` is preserved (DEC-002) and each persona sub-agent
receives a bash-command-form spawn prompt rather than the inlined
rendered prompt. `.speccy/ARCHITECTURE.md` agrees with the shipped
skill bodies on the primitive contract; multi-task orchestration is
explicitly a future Layer-2 concern with the existing `/loop` skill
named as the interim composer. CLI surface unchanged; SPEC.md /
TASKS.md / REPORT.md grammars unchanged. `speccy verify` exits 0
against the post-ship workspace (23 specs, 126 requirements, 164
scenarios, 0 errors).

<report spec="SPEC-0023">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
- **REQ-001 — `/speccy-work` is a single-task primitive.** Proved by
  CHK-001 (selector-form selects one task and exits without
  processing additional pending tasks; no-arg path resolves the next
  implementable task via `speccy next --kind implement --json` and
  implements only that one; orchestration vocabulary
  (`sub-agent`/`subagent`/`spawn`/`loop`/`until no tasks`) grep'd
  against `resources/modules/skills/speccy-work.md` returns no active
  guidance — only the proper-noun `/loop` reference in the
  role-agnostic caller paragraph; both wrapper templates'
  frontmatter `description:` strings match the single-task contract
  and trigger on phrases like "implement T-003" / "work the next
  task"). Backed in
  `resources/modules/skills/speccy-work.md`,
  `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`, and
  `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`. The
  shipped-skill body and frontmatter contracts are guarded in
  `speccy-cli/tests/skill_packs.rs`.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
- **REQ-002 — `/speccy-review` is a single-task primitive with
  parallel persona fan-out.** Proved by CHK-002 (one selector-form
  invocation runs exactly one round of four parallel persona reads
  and exits; no-arg path resolves the next reviewable task via
  `speccy next --kind review --json`; each spawned persona
  sub-agent's prompt is the bash-command form
  ``Run `speccy review SPEC-NNNN/T-NNN --persona <persona>` and
  follow its output. Your only deliverable is a single inline note
  appended to TASKS.md.`` rather than the CLI-rendered prompt
  inlined into the spawn call; aggregation flips `in-review` →
  `completed` on all-pass and `in-review` → `pending` with a `Retry:`
  bullet on any-blocking). The within-task four-persona fan-out is
  intrinsic per DEC-002 and the only Layer-1 primitive that spawns
  sub-agents. Backed in
  `resources/modules/skills/speccy-review.md`,
  `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`, and
  the Codex parallel. Wrapper `description:` strings sit at 499
  chars to fit the 500-char Codex display cap (T-002 addendum). The
  bash-command-form requirement is guarded in
  `speccy-cli/tests/skill_packs.rs::speccy_review_skill_prefers_native_subagents`.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
- **REQ-003 — Reviewer CLI prompts stop inlining the branch diff.**
  Proved by CHK-003 (none of the six
  `resources/modules/prompts/reviewer-*.md` templates references
  `{{diff}}`; each instead instructs the agent to run
  `git diff <merge-base>...HEAD -- <suggested-files>` itself with a
  defensive merge-base resolver across `origin/main`, `origin/master`,
  `main`, `master`; matching guidance is mirrored in every
  `resources/modules/personas/reviewer-*.md` body; the renderer code
  paths `diff_for_review`, `NO_DIFF_FALLBACK`, and `run_diff` are
  removed from `speccy-cli/src/git.rs`; the
  `speccy-cli/tests/git_diff.rs` integration suite is deleted; live
  rendered prompts on `SPEC-0022/T-001` measure 9,303 / 10,243 /
  9,139 / 9,155 chars for the four default personas, well under the
  80,000-char render budget, with zero `diff --git` lines and zero
  budget-guard hits). Backed in the six reviewer prompt and persona
  files, `speccy-cli/src/review.rs`, `speccy-cli/src/git.rs`, and
  `speccy-cli/tests/review.rs::rendered_prompt_omits_inline_diff_and_instructs_git_fetch`.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
- **REQ-004 — Architecture docs reflect the primitive contract.**
  Proved by CHK-004 (§"Core Development Loop", §"Phase 3", and
  §"Phase 4" of `.speccy/ARCHITECTURE.md` describe one primitive
  invocation per phase rather than skill-driven loops; the previous
  `loop:` pseudocode in §"Phase 3" and §"Phase 4" is deleted, not
  relabeled; each phase closes with a two-sentence Layer-2 note that
  multi-task composition is a future concern not built today,
  pointing at the existing `/loop` skill as the interim composer;
  active guidance contains no main-agent / sub-agent framing for the
  primitive itself, though the Phase 4 description names the
  within-task four-persona fan-out as the one intrinsic sub-agent
  use per DEC-002; the "What ships in v1" tree, "Workflow recipes"
  bullet list, and "typical full session" code block were
  reconciled to single-task wording in the same edit; `speccy verify`
  exits 0 and no new lints fire as a result). Backed in
  `.speccy/ARCHITECTURE.md` and the `cargo run -- verify` post-edit
  run.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-005">
- **REQ-005 — CLI-rendered prompts stop inlining AGENTS.md.** Proved
  by CHK-005 (no `{{agents}}` reference in any of the twelve
  `resources/modules/prompts/*.md` templates; the surrounding
  `## Project conventions` heading is removed from every prompt
  that introduced the block, so no rendered output is left with an
  empty section; `speccy-core/src/prompt/agents_md.rs` and its
  `load_agents_md` re-export are deleted; all five CLI callers
  (`plan`, `tasks`, `implement`, `review`, `report`) dropped the
  loader call site and the `vars.insert("agents", ...)` binding;
  live `cargo run -- {plan,plan SPEC-0022,tasks SPEC-0022,implement
  SPEC-0022/T-001,review SPEC-0022/T-001 --persona business,report
  SPEC-0022} | grep -c 'Product north star'` returns 0 across the
  board; rendered implementer prompt on `SPEC-0022/T-001` shrank
  from 20,938 chars pre-task to 12,545 chars post-task (savings
  8,393 ≥ the 8,367-byte `AGENTS.md` floor)). Backed in the twelve
  prompt templates, the five CLI command files, and the
  workspace-wide negative assertion in
  `speccy-cli/tests/skill_packs.rs::prompt_placeholders_match_commands`.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-006">
- **REQ-006 — CLI-rendered prompts use file references for SPEC.md,
  TASKS.md, MISSION.md.** Proved by CHK-006 (no `{{spec_md}}` in any
  of the eleven affected templates; no `{{tasks_md}}` in
  `report.md` / `tasks-amend.md`; no `{{mission}}` in
  `plan-amend.md`; `speccy-core/src/prompt/mission_md.rs` and its
  `find_nearest_mission_md` re-export are deleted;
  `TaskLocation::spec_dir` was added to `speccy-core/src/task_lookup.rs`
  so the CLI commands can derive the repo-relative path without
  re-walking the workspace; new `{{spec_md_path}}`,
  `{{tasks_md_path}}`, and `{{mission_section}}` placeholders are
  wired across all five CLI callers; on `SPEC-0022/T-001` the
  rendered implementer prompt names the SPEC.md path at
  `.speccy/specs/0022-xml-canonical-tasks-report/SPEC.md` and does
  not contain the SPEC body text; on a flat focus with no
  `MISSION.md` the rendered plan-amend prompt emits neither the
  `## Mission context` heading nor a Read instruction; on a focus
  with a `MISSION.md` it emits both; the per-task `{{task_entry}}`
  block stays inline because it is bounded to the one task under
  work). Backed in the eleven prompt templates, the five CLI
  command files, `speccy-core/src/task_lookup.rs`, and the
  workspace-wide negative assertion plus the per-template
  placeholder lists in `speccy-cli/tests/skill_packs.rs`. Specific
  contracts pinned in
  `speccy-cli/tests/plan.rs::amend_form_resolves_mission_grouped_spec_and_names_mission_md_path`
  and
  `speccy-cli/tests/plan.rs::amend_form_for_mission_grouped_spec_without_mission_md_emits_no_mission_read`.
</coverage>

</report>

## Task summary

Seven tasks, all completed, zero retries.

- T-001 — Rewrote `resources/modules/skills/speccy-work.md` as a
  single-task primitive; optional `[SPEC-NNNN/T-NNN]` selector,
  no-arg path resolves via `speccy next --kind implement --json`;
  role-agnostic caller paragraph; six-field handoff template at
  exit. Updated both wrapper frontmatter `description:` strings.
- T-002 — Rewrote `resources/modules/skills/speccy-review.md` as a
  single-task primitive with within-task four-persona parallel
  fan-out (intrinsic per DEC-002). Each spawned persona sub-agent's
  prompt is the bash-command form, not the inlined CLI-rendered
  prompt. Aggregation flips `in-review` → `completed` on all-pass or
  → `pending` with a `Retry:` bullet on any-blocking. T-002 addendum
  trimmed three pre-existing test guards in
  `speccy-cli/tests/skill_packs.rs` that encoded the retired loop-recipe
  contract; speccy-review wrapper `description:` trimmed to 499 chars
  for the Codex 500-char display cap; new SPEC-0023 entry added to
  `speccy-core/tests/fixtures/in_tree_id_snapshot.json`.
- T-003 — Dropped the `{{diff}}` interpolation from all six
  `resources/modules/prompts/reviewer-*.md` templates and matched
  the persona bodies; deleted `diff_for_review`, `NO_DIFF_FALLBACK`,
  and `run_diff` from `speccy-cli/src/git.rs`; deleted
  `speccy-cli/tests/git_diff.rs`. Pre/post measurements on
  `SPEC-0022/T-001 --persona business`: 37,457 → 21,263 chars
  (subsequent shrinks from T-005 / T-006 took this further to
  9,303 chars).
- T-004 — Rewrote `.speccy/ARCHITECTURE.md` §"Core Development Loop",
  §"Phase 3", and §"Phase 4" as single-task primitives; deleted the
  `loop:` pseudocode in each phase; added the Layer-2 note pointing
  at `/loop` as the interim composer at the end of each phase;
  reconciled the "What ships in v1" tree, "Workflow recipes" bullet
  list, and "typical full session" code block; updated the
  State Model `state="completed"` "Who sets it" row and the State
  transitions paragraph to drop main-agent framing.
- T-005 — Dropped the `{{agents}}` interpolation and the wrapping
  `## Project conventions` heading from all twelve prompt templates;
  deleted `speccy-core/src/prompt/agents_md.rs` and the
  `load_agents_md` re-export; removed the loader call site from all
  five CLI callers; refreshed dogfood mirrors. Implementer prompt on
  `SPEC-0022/T-001` shrank by 8,393 chars (AGENTS.md is 8,367 bytes).
- T-006 — Dropped the `{{spec_md}}`, `{{tasks_md}}`, and `{{mission}}`
  interpolations from all eleven affected templates; deleted
  `speccy-core/src/prompt/mission_md.rs` and its
  `find_nearest_mission_md` re-export; added
  `TaskLocation::spec_dir` to
  `speccy-core/src/task_lookup.rs`; wired new placeholders
  (`{{spec_md_path}}`, `{{tasks_md_path}}`, `{{mission_section}}`)
  across all five CLI callers; the `mission_section` helper in
  `speccy-cli/src/plan.rs` builds the entire `## Mission context`
  section (heading plus Read instruction) when a `MISSION.md`
  exists in an enclosing focus folder, else returns an empty
  string so the rendered prompt surfaces neither.
- T-007 — Re-ejected the host-local `.claude/skills/speccy-{work,review}/SKILL.md`
  and `.agents/skills/speccy-{work,review}/SKILL.md` via
  `cargo run -- init --force` for both hosts so the dogfood
  mirrors match the new shipped resources.

## Out-of-scope items absorbed

- The T-002 implementer note records a follow-on cleanup of three
  pre-existing test guards in `speccy-cli/tests/skill_packs.rs`
  (`recipe_content_shape`, `shipped_descriptions_natural_language_triggers`,
  `speccy_review_skill_prefers_native_subagents`). These guards
  encoded the pre-T-001 / T-002 loop-recipe contract and were flipped
  to match the new single-task primitive contract that T-001 / T-002
  established.
- One nit fix surfaced during this session's adversarial review:
  the `## Mission context` heading in `plan-amend.md` rendered
  unconditionally even when no `MISSION.md` existed, leaving an
  empty section. The fix rolled the heading into the
  `{{mission_section}}` placeholder (renamed from
  `{{mission_read_instruction}}`) so the heading and Read instruction
  are either both present or both absent. Strengthened
  `speccy-cli/tests/plan.rs::amend_form_for_mission_grouped_spec_without_mission_md_emits_no_mission_read`
  to assert the heading is absent and
  `amend_form_resolves_mission_grouped_spec_and_names_mission_md_path`
  to assert the heading is present. The SPEC's REQ-006 done-when
  did not explicitly require this; it was caught by review as a
  cosmetic regression that diverged from REQ-005's heading-removal
  contract and fixed in the same PR.

## Skill updates

(none) — no shipped skill file under `skills/` or
`resources/agents/` was edited mid-task to fix friction. The
implementer prompts for all seven tasks rendered cleanly, and the
test-guard updates and code edits were directly executable from the
implementer's own scope.

## Deferred / known limitations

- **The "rendered implementer prompt savings ≥ size of SPEC.md"
  scenario in REQ-006 is unsatisfiable as written on specs whose
  tasks each cover a narrow subset of requirements.** The T-006
  implementer note explains the discrepancy in full: the pre-task
  implementer prompt inlined only the task-scoped slice of SPEC.md
  (the requirements named in `covers="..."` plus decisions), not
  the full SPEC body. For `SPEC-0022/T-001` covering only REQ-003,
  the slice was small relative to the full SPEC.md; realised savings
  was 3,464 chars — the entire former slice plus its surrounding
  heading and pointer prose, which is the maximum possible savings
  for that task. The scenario's named bound is the *upper bound* on
  theoretical savings (saturated only when every requirement is
  covered by one task), not a floor the renderer can hit across all
  task-scope sizes. No code-path remediation possible; the saving is
  exactly what dropping SPEC.md inlining buys on a narrow task.
- **No multi-task orchestrator.** This spec deliberately defers the
  composing layer (the would-be `/speccy-run` skill). Today the
  interim composer is the existing `/loop` skill, which iterates a
  Speccy primitive on its caller's behalf. A future spec will
  introduce a Speccy-aware orchestrator once dogfooding earns it in.
- **`cargo deny check` was not run locally during this ship loop.**
  `cargo-deny` is not installed on this workstation; the workspace
  hygiene check is delegated to CI. The other three AGENTS.md
  hygiene gates (`cargo test --workspace`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo +nightly fmt
  --all --check`) all passed locally before commit (584 passed, 0
  failed; clippy clean; fmt clean).
