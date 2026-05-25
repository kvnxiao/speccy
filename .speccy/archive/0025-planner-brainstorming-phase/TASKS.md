---
spec: SPEC-0025
spec_hash_at_generation: b17c689c916007f142de43ceb5a733429727f4c7355fa087daff0c79d79b4428
generated_at: 2026-05-23T07:36:27Z
---

# Tasks: SPEC-0025 Brainstorming skill for atomizing intent before SPEC creation


## Phase 1: New skill body

<task id="T-001" state="completed" covers="REQ-002">
Write the speccy-brainstorm module skill body

- Suggested files: `resources/modules/skills/speccy-brainstorm.md`

<task-scenarios>
  - Given `resources/modules/skills/speccy-brainstorm.md` after this
    task, when read, then it begins with the `{{ cmd_prefix }}speccy-brainstorm`
    slug-style heading (matching the other shipped skill bodies' `#`
    heading shape) and a one-paragraph summary of the skill's purpose.
  - Given the skill body, when read, then it names four artifacts the
    agent must produce during the brainstorm: (1) a restated ask
    broken into atomic, first-principle requirements; (2) 2-3
    alternative framings with one-sentence sketches and explicit
    rejection reasons; (3) silent assumptions the agent would
    otherwise bake in; (4) open questions in the `- [ ]` checkbox
    format used by the PRD template's `## Open Questions` section.
  - Given the skill body, when read, then it names "2-3" as soft
    guidance for alternative-framing counts and explicitly instructs
    the agent to scale to slice complexity (so a trivial ask may
    surface zero alternatives, a load-bearing one may need four).
  - Given the skill body, when grep'd case-insensitively for
    "one question at a time", then a hit is returned. The skill
    teaches the obra/superpowers interaction discipline of asking
    clarifying questions one at a time rather than batching them.
  - Given the skill body, when read, then it carries an explicit
    hard-gate instruction telling the agent not to invoke
    `{{ cmd_prefix }}speccy-plan` and not to write SPEC.md until the
    user has approved the framing. The gate is named in prose using
    strong language (e.g., "do NOT", "STOP", or "hard gate"). No
    machine marker (XML sentinel, regex hook) is introduced.
  - Given the skill body, when read, then it names the four
    destinations Phase-1-style brainstorm outputs flow into when the
    agent next invokes `/speccy-plan`: `## Summary` (restated ask,
    folded as designed artifact not pasted verbatim);
    `<assumptions>` inside `## Assumptions` (silent assumptions);
    `## Open Questions` (unresolved questions); `## Notes` (rejected
    alternative framings, with `### Decisions` / `<decision>`
    escalation for load-bearing trade-offs).
  - Given the skill body, when read, then it names
    `{{ cmd_prefix }}speccy-plan` as the terminal next step after
    the user has approved the framing.
</task-scenarios>
</task>

## Phase 2: Host wrappers and skill-enumeration tests

<task id="T-002" state="completed" covers="REQ-001">
Add host wrappers for speccy-brainstorm and extend the skill-enumeration tests

- Suggested files:
  `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`,
  `speccy-cli/tests/skill_packs.rs`,
  `speccy-cli/tests/init.rs`

<task-scenarios>
  - Given `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
    after this task, when read, then it carries a YAML frontmatter
    block with `name: speccy-brainstorm` and a `description:` line
    naming the brainstorming purpose, followed by exactly one
    `{% include "modules/skills/speccy-brainstorm.md" %}` directive.
    The pattern matches the other speccy-* Claude Code wrappers.
  - Given `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
    after this task, when read, then it carries the same YAML
    frontmatter / include pattern and follows the existing Codex
    wrapper convention.
  - Given `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` after this
    task, when inspected, then the slice contains
    `"speccy-brainstorm"` as an element. The total length grows from
    7 to 8.
  - Given `speccy-cli/tests/init.rs::SKILL_NAMES` after this task,
    when inspected, then the array contains `"speccy-brainstorm"` as
    an element and its declared length type updates from
    `[&str; 7]` to `[&str; 8]`.
  - Given `cargo test --workspace`, when run after this task, then
    every skill-pack assertion that iterates `SKILL_NAMES` (presence,
    YAML frontmatter shape, rendered-byte-identity, host-pack
    coverage) passes for `speccy-brainstorm` without any code change
    in the renderer or `init` command.
</task-scenarios>
</task>

## Phase 3: Cross-reference speccy-plan + dogfood re-eject

<task id="T-003" state="completed" covers="REQ-003">
Update speccy-plan skill body, re-eject dogfood mirrors, and run hygiene

- Suggested files: `resources/modules/skills/speccy-plan.md`,
  `.claude/skills/speccy-plan/SKILL.md`,
  `.agents/skills/speccy-plan/SKILL.md`,
  `.claude/skills/speccy-brainstorm/SKILL.md`,
  `.agents/skills/speccy-brainstorm/SKILL.md`

<task-scenarios>
  - Given `resources/modules/skills/speccy-plan.md` after this task,
    when its "When to use" section is read, then it names
    `{{ cmd_prefix }}speccy-brainstorm` as a recommended precursor
    for fuzzy asks (one-line note: when the framing is not yet
    agreed, run brainstorm first).
  - Given the same file, when grep'd case-sensitively for the
    literal strings `inlines \`AGENTS.md\``, `inlines AGENTS.md`,
    `inlines the nearest parent \`MISSION.md\``, `inlines \`MISSION.md\``,
    `inlines MISSION.md`, then no hit is returned. Any equivalent
    claim that the rendered prompt embeds those bodies is removed.
  - Given the amendment-form description in the same file, when
    read, then it remains a single-pass surgical edit description
    without a mandatory brainstorm step (consistent with the SPEC's
    non-goal that brainstorm is recommendation, not requirement).
  - Given the host-local dogfood files at
    `.claude/skills/speccy-brainstorm/SKILL.md` and
    `.agents/skills/speccy-brainstorm/SKILL.md` after this task,
    when read, then they exist and match the rendered output of
    `render_host_pack` byte-for-byte (the repo has been re-ejected
    via `cargo run -- init --force --host claude-code` and
    `cargo run -- init --force --host codex`).
  - Given the host-local dogfood files at
    `.claude/skills/speccy-plan/SKILL.md` and
    `.agents/skills/speccy-plan/SKILL.md` after this task, when
    read, then they reflect the updated `speccy-plan` module body
    (i.e., they reference `speccy-brainstorm` and no longer carry
    the retired "inlines AGENTS.md" / "inlines MISSION.md" claims).
  - Given `cargo test --workspace`, when run after this task, then
    every test passes — including the `dogfood_outputs_match_committed_tree`
    test which compares `render_host_pack` output to the committed
    `.claude/skills/` and `.agents/skills/` mirrors.
  - Given `cargo clippy --workspace --all-targets --all-features --
    -D warnings`, when run after this task, then no warnings are
    reported.
  - Given `cargo +nightly fmt --all --check`, when run after this
    task, then no formatting drift is reported.
</task-scenarios>
</task>

