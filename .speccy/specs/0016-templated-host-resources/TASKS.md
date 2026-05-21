---
spec: SPEC-0016
spec_hash_at_generation: 49927c66ce0225587137517400c85a80eba530d101eb28de596de2cb332fd101
generated_at: 2026-05-17T17:37:23Z
---

# Tasks: SPEC-0016 Templated host resources and reviewer subagents

## Phase 1: Dependency and shared-content relocation


<task id="T-001" state="completed" covers="REQ-006">
Add `minijinja = "2"` to workspace dependencies (session-T001, 2026-05-14)

- Suggested files: `Cargo.toml`, `speccy-cli/Cargo.toml`

<task-scenarios>
  - When `cargo build --workspace --locked` runs, then the build
    succeeds and `minijinja` resolves to the latest 2.x.
  - When `cargo deny check` runs, then no advisory or license
    warning fires against the new dependency.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001">
Relocate shared personas and prompts into `resources/modules/` (session-T002, 2026-05-14)

- Suggested files: `resources/modules/personas/` (new),
  `resources/modules/prompts/` (new),
  `speccy-core/src/personas.rs`,
  `speccy-core/src/prompt/template.rs`,
  `speccy-core/tests/personas.rs`

<task-scenarios>
  - When `speccy-core::personas::find` looks up `reviewer-security`,
    then it returns the shipped body and the body's first line is
    unchanged from the pre-move content.
  - When the prompt template loader renders `plan-greenfield`,
    then the rendered prompt is byte-identical to the pre-move
    output (no behavioural change to consumers).
  - When the workspace tree is walked, then no path matches
    `skills/shared/personas/*` or `skills/shared/prompts/*`.
  - When the embedded persona/prompt directories are inspected,
    then both are non-empty (matching the existing SPEC-0002
    bundle-non-empty invariant for the skill pack).
</task-scenarios>
</task>

## Phase 2: Skill body modules and host wrappers


<task id="T-003" state="completed" covers="REQ-001 REQ-002">
Author `resources/modules/skills/speccy-<verb>.md` bodies (session-T003, 2026-05-14)

- Suggested files: `resources/modules/skills/speccy-init.md`
  through `resources/modules/skills/speccy-amend.md` (7 new
  files), `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - When each of the seven module bodies (`speccy-init`,
    `speccy-plan`, `speccy-tasks`, `speccy-work`, `speccy-review`,
    `speccy-ship`, `speccy-amend`) is rendered with the
    Claude Code template context (`cmd_prefix = "/"`,
    `host_display_name = "Claude Code"`,
    `skill_install_path = ".claude/skills"`), then the rendered
    output is byte-identical to the body section
    (post-frontmatter) of `skills/claude-code/speccy-<verb>/SKILL.md`.
  - When each module body is rendered with the Codex template
    context (`cmd_prefix = ""`,
    `host_display_name = "Codex"`,
    `skill_install_path = ".agents/skills"`), then the rendered
    output is byte-identical to the body section of
    `skills/codex/speccy-<verb>/SKILL.md`.
  - When `speccy-review.md` is searched in module form, then it
    contains a `{% if host == "claude-code" %}` block and an
    `{% else %}` block bracketing step 4 (the divergence point
    where Claude Code uses the Task tool and Codex prose-spawns).
  - The byte-equivalence test is annotated as transient and
    scheduled for deletion alongside the legacy tree in T-008.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-002">
Extend `HostChoice` with `install_roots` and `template_context` (session-T004, 2026-05-14)

- Suggested files: `speccy-cli/src/host.rs`

<task-scenarios>
  - When `HostChoice::ClaudeCode.install_roots()` is called, then
    it returns `&[".claude"]`.
  - When `HostChoice::Codex.install_roots()` is called, then it
    returns `&[".agents", ".codex"]`.
  - When `HostChoice::ClaudeCode.template_context()` is converted
    to a MiniJinja `Value` and inspected, then it carries the
    keys `host = "claude-code"`, `cmd_prefix = "/"`,
    `host_display_name = "Claude Code"`,
    `skill_install_path = ".claude/skills"`.
  - When `HostChoice::Codex.template_context()` is inspected,
    then `host = "codex"`, `cmd_prefix = ""`,
    `host_display_name = "Codex"`,
    `skill_install_path = ".agents/skills"`.
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-001 REQ-002">
Create Claude Code SKILL.md wrappers under `resources/agents/.claude/skills/` (session-T005, 2026-05-14)

- Suggested files: `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl`
  through `resources/agents/.claude/skills/speccy-amend/SKILL.md.tmpl`
  (7 new files)

<task-scenarios>
  - When the embedded bundle is walked, then exactly seven files
    match `agents/.claude/skills/speccy-*/SKILL.md.tmpl`, named
    after the seven shipped verbs.
  - When each wrapper is read, then it consists of a YAML
    frontmatter block (`name`, `description`) followed by a
    bare `{% include "modules/skills/speccy-<verb>.md" %}`
    directive and nothing else. (Amended 2026-05-14 per DEC-004:
    no `{% raw %}` wrapping; the module body's `{{ cmd_prefix }}`
    / `{% if host %}` tokens must expand for REQ-002 to hold.)
  - When the frontmatter is parsed, then `name` equals
    `speccy-<verb>` and `description` is a non-empty single-line
    string. (Amended 2026-05-14: dropped the "matches the
    pre-migration SKILL.md description" sub-clause — the legacy
    oracle was deleted by T-008.)
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-001 REQ-002">
Create Codex SKILL.md wrappers under `resources/agents/.agents/skills/` (session-T006, 2026-05-14)

- Suggested files: `resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl`
  through `resources/agents/.agents/skills/speccy-amend/SKILL.md.tmpl`
  (7 new files)
- Retry (resolved 2026-05-14): Both blockers were resolved by
  `/speccy-amend SPEC-0016`. The amended DEC-004 makes bare
  `{% include %}` the canonical form (matching the as-built
  impl); REQ-002 / REQ-004 expansion is now load-bearing
  rather than divergent. The Tests-to-write bullet 2 above was
  updated to expect bare `{% include %}` rather than the
  `{% raw %}`-wrapped form, and bullet 3's
  description-matches-legacy sub-clause was dropped (the
  oracle is gone post-T-008). Next implementer pass should
  verify both updated tests pass on the existing diff and
  flip `[ ]` -> `[?]` with a note pointing at the resolved
  blockers.

<task-scenarios>
  - When the embedded bundle is walked, then exactly seven files
    match `agents/.agents/skills/speccy-*/SKILL.md.tmpl`.
  - When each wrapper is read, then it consists of a YAML
    frontmatter block plus a bare `{% include ... %}` directive
    identical in structure to the Claude Code wrapper. (Amended
    2026-05-14 per DEC-004: no `{% raw %}` wrapping.)
  - When the frontmatter is parsed, then `name` equals
    `speccy-<verb>` and `description` is a non-empty single-line
    string. (Amended 2026-05-14: dropped the "matches the
    pre-migration Codex SKILL.md description" sub-clause — the
    legacy oracle was deleted by T-008.)
</task-scenarios>
</task>

## Phase 3: Renderer wiring and legacy-tree removal


<task id="T-007" state="completed" covers="REQ-002 REQ-006">
Wire MiniJinja rendering into `speccy init` (session-T007, 2026-05-14)

- Suggested files: `speccy-cli/src/embedded.rs`,
  `speccy-cli/src/init.rs`,
  `speccy-cli/tests/init.rs`,
  `speccy-cli/tests/skill_packs.rs`,
  `speccy-cli/tests/embedded.rs` (new)

<task-scenarios>
  - When `embedded::RESOURCES` is queried for `agents/.claude/skills/speccy-plan/SKILL.md.tmpl`,
    then the file is present.
  - When `RESOURCES` is queried for `modules/skills/speccy-plan.md`,
    then the file is present.
  - When `RESOURCES.dirs().count()` is inspected, then at least
    two top-level entries (`agents/`, `modules/`) exist and each
    is non-empty.
  - When `speccy init --host claude-code` runs in a tempdir
    containing `.claude/`, then it walks
    `agents/.claude/` under the embedded bundle, renders each
    `.tmpl` file via MiniJinja with the Claude Code template
    context, strips the `.tmpl` suffix, and writes to the
    matching path under the tempdir; no path is created under
    `.agents/` or `.codex/`.
  - When the rendered `.claude/skills/speccy-plan/SKILL.md` is
    read, then it contains `/speccy-tasks` and does not contain
    a bare `speccy-tasks` token without the slash prefix.
  - When `speccy init --host codex` runs in a tempdir containing
    `.codex/`, then it walks both `agents/.agents/` and
    `agents/.codex/` and writes to the matching paths; the
    rendered `.agents/skills/speccy-plan/SKILL.md` contains
    `speccy-tasks` without slash prefix.
  - When a strict-undefined MiniJinja `Environment` renders any
    `.tmpl` file with the appropriate host context, then the
    render does not error (every variable referenced has a
    value).
  - When `speccy init --force` runs against a project root with a
    user-authored file at `.claude/skills/my-skill/SKILL.md`,
    then that file is byte-identical before and after.
</task-scenarios>
</task>

<task id="T-008" state="completed" covers="REQ-001">
Delete the legacy `skills/` tree (session-T008, 2026-05-14)

- Suggested files: `skills/` (deleted),
  `speccy-cli/tests/skill_packs.rs`,
  `speccy-cli/src/embedded.rs` (doc-comment refresh)

<task-scenarios>
  - When the workspace tree is walked, then no path matches
    `skills/` at the workspace root.
  - When `cargo build --workspace --locked` runs, then it
    succeeds with no references to the removed tree (no
    `include_dir!` invocation, no test path, no doc comment
    pointing at `skills/`).
  - The transient byte-equivalence tests introduced in T-003 are
    removed in the same commit; assert via grep-style search
    that no test under `speccy-cli/tests/` references
    `skills/claude-code/` or `skills/codex/` paths.
</task-scenarios>
</task>

## Phase 4: Reviewer subagent files


<task id="T-009" state="completed" covers="REQ-003">
Create Claude Code reviewer subagent wrappers (session-T009, 2026-05-14)

- Suggested files: `resources/agents/.claude/agents/reviewer-business.md.tmpl`
  through `resources/agents/.claude/agents/reviewer-docs.md.tmpl`
  (6 new files), `speccy-cli/tests/init.rs`

<task-scenarios>
  - When the embedded bundle is walked, then exactly six files
    match `agents/.claude/agents/reviewer-*.md.tmpl`, named for
    the six personas (`business`, `tests`, `security`, `style`,
    `architecture`, `docs`).
  - When each wrapper is read, then it consists of YAML
    frontmatter (`name: reviewer-<persona>`,
    `description: <one-line string>`) followed by a bare
    `{% include "modules/personas/reviewer-<persona>.md" %}`
    directive. (Amended 2026-05-14 per DEC-004: no `{% raw %}`
    wrapping; persona bodies currently contain no Jinja tokens,
    and strict-undefined mode is the safety net for any future
    regression.)
  - When `speccy init --host claude-code` renders the pack into
    a tempdir, then `.claude/agents/reviewer-security.md` exists,
    opens with `---`, parses as YAML frontmatter, and the body
    contains the focus bullet
    "Authentication and authorization boundaries" (drawn from
    the persona body verbatim).
  - When all six rendered files are parsed, then each carries a
    `name` value equal to its filename stem.
</task-scenarios>
</task>

<task id="T-010" state="completed" covers="REQ-003">
Create Codex reviewer subagent wrappers (session-T010, 2026-05-14)

- Suggested files: `resources/agents/.codex/agents/reviewer-business.toml.tmpl`
  through `resources/agents/.codex/agents/reviewer-docs.toml.tmpl`
  (6 new files), `speccy-cli/tests/init.rs`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - When the embedded bundle is walked, then exactly six files
    match `agents/.codex/agents/reviewer-*.toml.tmpl`.
  - When each wrapper is read, then it sets `name`,
    `description`, and `developer_instructions` keys, with
    `developer_instructions` declared as a TOML triple-quoted
    string wrapping a bare
    `{% include "modules/personas/reviewer-<persona>.md" %}`
    directive. (Amended 2026-05-14 per DEC-004: no `{% raw %}`
    wrapping. The TOML-safety invariant test
    `t010_persona_bodies_have_no_toml_triple_quote` enforces the
    `"""`-free contract on persona bodies.)
  - When `speccy init --host codex` renders the pack into a
    tempdir, then `.codex/agents/reviewer-security.toml` parses
    via `toml::from_str::<toml::Value>` and exposes top-level
    keys `name = "reviewer-security"`, `description` (non-empty
    string), and `developer_instructions` (non-empty string).
  - When the six rendered TOML files are all parsed, then each
    carries `name` equal to its filename stem.
  - When every file under `resources/modules/personas/` is read,
    then none contains the literal substring `"""` (TOML-safety
    invariant; failure message names the offending file).
</task-scenarios>
</task>

## Phase 5: Skill divergence, CI, and dogfood


<task id="T-011" state="completed" covers="REQ-004">
Diverge `speccy-review` step 4 per host

- Suggested files: `resources/modules/skills/speccy-review.md`,
  `speccy-cli/tests/skill_packs.rs`

<task-scenarios>
  - When `resources/modules/skills/speccy-review.md` is rendered
    with the Claude Code template context, then step 4 contains
    the literal substring `subagent_type: "reviewer-` and names
    the four default personas (`reviewer-business`,
    `reviewer-tests`, `reviewer-security`, `reviewer-style`).
  - When the same module is rendered with the Codex template
    context, then step 4 does not contain `subagent_type:` and
    instead references the four reviewer subagents by name in
    prose.
  - When either rendered SKILL.md is searched, then it contains
    `speccy review` as a fallback reference (with explicit
    `--persona X` example) for harnesses that don't recognise
    the subagent type.
</task-scenarios>
</task>

<task id="T-012" state="completed" covers="REQ-005">
Update CI workflow with dual-host materialization check

- Suggested files: `.github/workflows/ci.yml`

<task-scenarios>
  - When `.github/workflows/ci.yml` is inspected, then the
    "materialized host packs in sync" job step runs
    `speccy init --force --host claude-code` followed by
    `speccy init --force --host codex` and then
    `git diff --exit-code .claude .codex .agents .speccy/skills`.
  - When the workflow file is inspected, then no diff target
    path mentions `.claude/commands` (the stale path from
    pre-SPEC-0015 layout is removed).
  - When the failure message string is inspected, then it points
    contributors at the two `speccy init --force --host ...`
    commands needed to refresh outputs locally.
</task-scenarios>
</task>

<task id="T-013" state="completed" covers="REQ-005 REQ-006">
Refresh dogfooded host outputs and verify byte-identity

- Suggested files: `.claude/skills/`, `.claude/agents/`,
  `.agents/skills/`, `.codex/agents/`, `.speccy/skills/`
  (regenerated outputs)

<task-scenarios>
  - When `speccy init --force --host claude-code` runs in
    Speccy's own checkout, then `git diff --exit-code .claude
    .speccy/skills` succeeds.
  - When `speccy init --force --host codex` runs in the same
    checkout, then `git diff --exit-code .agents .codex
    .speccy/skills` succeeds.
  - When either init command is run twice in succession against
    the same checkout, then the second run produces no file
    modifications (idempotency).
  - When the committed dogfood outputs are searched, then no
    file contains the literal substrings `{{` or `{%` outside
    fenced code blocks (no unsubstituted tokens).
  - When `speccy verify` runs at the post-refresh HEAD, then it
    exits zero.
</task-scenarios>
</task>

