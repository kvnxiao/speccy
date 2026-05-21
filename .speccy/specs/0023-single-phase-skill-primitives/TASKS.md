---
spec: SPEC-0023
spec_hash_at_generation: 6b20cbeacdf1a85d8e5ce9d7383c3d623f6dee6a9f4155680a9ab1a848f13f3a
generated_at: 2026-05-17T17:37:24Z
---

# Tasks: SPEC-0023 Single-phase skill primitives for the development loop


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
</task>

