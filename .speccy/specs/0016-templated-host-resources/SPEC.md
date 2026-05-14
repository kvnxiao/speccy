---
id: SPEC-0016
slug: templated-host-resources
title: Templated host resources and reviewer subagents
status: implemented
created: 2026-05-14
supersedes: []
---

# SPEC-0016: Templated host resources and reviewer subagents

## Summary

Two related drift sources surfaced after SPEC-0015 settled the
host-skill directory layout. First, every shipped skill lives twice
on disk: `skills/claude-code/speccy-<verb>/SKILL.md` and
`skills/codex/speccy-<verb>/SKILL.md` are near-identical, differing
only in the slash prefix on inline command references (`/speccy-plan`
vs `speccy-plan`), the host display name, and the install path (the
last two only in `speccy-init`). Seven such pairs ship today (14
files); every cross-cutting wording fix is an N-way hand-edit and
already drifts in practice. Second, reviewer personas render as
inline prompt bodies via `speccy review --persona X`: the host LLM
captures stdout and splices the persona into a spawned sub-agent's
system prompt. The harness-native primitives that exist for this
exact pattern — Claude Code's `Task` tool with
`subagent_type: "reviewer-<persona>"` and Codex's name-based prose
spawn — are bypassed because Speccy never renders persona bodies
into host-native subagent files.

This spec introduces MiniJinja as the rendering engine and splits
`resources/` into two roles: `resources/agents/.<host>/...` carries
thin host-specific wrappers whose folder structure mirrors the
install destination 1:1, and `resources/modules/...` carries
host-neutral content (persona bodies, prompt templates, and skill
bodies) consumed by both `speccy review` and by every host's
wrappers via `{% include %}`. The same machinery materialises a new
artifact type: reviewer subagent files. Six personas render per
host to `.claude/agents/reviewer-*.md` (markdown + YAML
frontmatter) and `.codex/agents/reviewer-*.toml` (flat TOML), so
the `/speccy-review` skill can prefer the host-native subagent
invocation while keeping the existing `speccy review --persona X`
CLI as a fallback path for harnesses that don't recognise the
subagent type.

The change is intentionally narrow at the persona-resolver and
prompt-template layers: SPEC-0009 DEC-002's project-local override
chain stays; only the embedded source path moves from
`skills/shared/personas/` to `resources/modules/personas/`. The
prompt rendering at `speccy review`, `speccy plan`, etc. continues
to read persona bodies inline as before. The migration is
pre-v1, so no production installs need a migration story; the
`skills/` source tree is deleted as part of the cut-over and
Speccy's own dogfooded `.claude/`, `.agents/`, and `.codex/`
outputs are refreshed in the same commit.

## Goals

- One canonical source file for any shared resource — persona
  body, prompt template, or skill body — so cross-cutting edits
  land once and propagate to every host on next render.
- Bundle source layout under `resources/agents/.<host>/...`
  mirrors the install destination structure 1:1, so a contributor
  reading `resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl`
  immediately knows where it lands and what host it serves.
- Reviewer subagent files exist as a first-class shipped artifact
  per host, populated from the same persona bodies that
  `speccy review` already renders inline.
- The Claude Code `/speccy-review` skill prefers the `Task` tool
  with `subagent_type: "reviewer-<persona>"`; the Codex skill
  prefers prose-spawning the named reviewer subagents; both keep
  the existing `speccy review --persona X` CLI as a fallback step
  for harnesses that don't recognise the subagent type.
- CI fails fast when `resources/` source drifts from the committed
  dogfood outputs under `.claude/`, `.agents/`, `.codex/`, and
  `.speccy/skills/`.

## Non-goals

- Changing the persona resolver chain. SPEC-0009 DEC-002's
  lookup order (project-local override at
  `.speccy/skills/personas/reviewer-X.md`, then shipped) is
  preserved; only the embedded source path for the shipped
  fallback moves.
- Wiring `.speccy/skills/prompts/` into a runtime loader. The
  prompts are still embedded into the CLI binary and loaded
  directly from there; the disk copy at init time remains
  documentation/override-future-proofing only, same as today.
- Migrating `speccy.toml.tmpl` rendering off the existing
  `str::replace` at `speccy-cli/src/init.rs:269-270`. That
  template is not part of the host pack; consolidating it onto
  MiniJinja is a follow-up worth doing opportunistically, not
  in this spec.
- Adding a third host. The architecture supports adding one by
  creating a new `resources/agents/.<host>/` sibling; v1 still
  ships Claude Code and Codex only.
- Changing the `speccy review` CLI surface or its inline prompt
  rendering. Subagent files are a new artifact alongside, not a
  replacement for, the existing `--persona` rendering.

## User stories

- As a speccy contributor, when I update the prose in
  `resources/modules/skills/speccy-review.md` once, both hosts'
  shipped `SKILL.md` pick up the change after `speccy init --force`
  without any further hand-editing.
- As a speccy contributor, when I edit
  `resources/modules/personas/reviewer-security.md`, both
  `speccy review --persona security` (inline) and the rendered
  `.claude/agents/reviewer-security.md` /
  `.codex/agents/reviewer-security.toml` subagent files reflect
  the change after `speccy init --force`.
- As a Claude Code user, after `speccy init`, running
  `/speccy-review` causes the main agent to invoke its `Task`
  tool with `subagent_type: "reviewer-business"`,
  `"reviewer-tests"`, `"reviewer-security"`, and
  `"reviewer-style"`, with the persona bodies coming from the
  rendered `.claude/agents/reviewer-*.md` files rather than from
  stdout of `speccy review`.
- As a Codex user, after `speccy init --host codex`, the same
  loop is triggered by the `speccy-review` skill referencing
  the four reviewer subagents by name, which Codex's
  orchestrator resolves to `.codex/agents/reviewer-*.toml`.
- As a CI maintainer, when a contributor edits a module body
  without re-running `speccy init --force` for both hosts, the
  PR build fails with a clear diff pointing at the stale
  materialised outputs.

## Requirements

### REQ-001: Single-source content modules

Persona bodies, prompt templates, and skill bodies each have
exactly one source-of-truth file under
`resources/modules/{personas,prompts,skills}/`. All host-specific
rendered outputs derive from these modules.

**Done when:**

- `resources/modules/personas/reviewer-*.md` is the only source of
  truth for the eight personas listed in SPEC-0009 (six default
  plus `reviewer-architecture` and `reviewer-docs`).
- `resources/modules/prompts/*.md` is the only source of truth
  for the twelve prompt templates currently under
  `skills/shared/prompts/`.
- `resources/modules/skills/speccy-<verb>.md` is the only source
  of truth for the seven shipped skill bodies; no per-host
  duplicate body files exist anywhere in the repository (the
  per-host `.tmpl` wrappers are thin and `{% include %}` the
  module body).
- `speccy-core::personas::SHIPPED_PERSONAS` and
  `speccy-core::prompt::template`'s embedded directory both
  point at the new `resources/modules/...` paths and are
  non-empty.
- The legacy `skills/` tree is deleted from the workspace.

**Behavior:**

- Given the embedded `RESOURCES` bundle, when iterated, then
  `modules/personas/reviewer-security.md`,
  `modules/prompts/plan-greenfield.md`, and
  `modules/skills/speccy-review.md` all exist as walkable files.
- Given the embedded `RESOURCES` bundle, when walked, then no
  files exist directly under
  `agents/.claude/skills/speccy-*/SKILL.md` whose body content
  duplicates the matching `modules/skills/speccy-<verb>.md` body
  verbatim — the wrapper bodies consist of a frontmatter block
  plus an `{% include %}` directive.
- Given the workspace tree, when walked, then no path matches
  `skills/`.

**Covered by:** CHK-001, CHK-002

### REQ-002: Host-templated skill packs install to correct destinations

`speccy init --host <host>` walks `resources/agents/.<host>/...`
in the embedded bundle, renders each `.tmpl` file via MiniJinja
with that host's template context, strips the `.tmpl` suffix,
and writes the result to `<project_root>/<rel>` where `<rel>`
matches the source path under `resources/agents/.<host>/`.

**Done when:**

- For `--host claude-code`, the seven rendered `SKILL.md` files
  land at `.claude/skills/speccy-<verb>/SKILL.md` and contain
  slash-prefixed command references (`/speccy-plan`,
  `/speccy-tasks`, etc.) wherever the source uses
  `{{ cmd_prefix }}{{ verb }}`.
- For `--host codex`, the seven rendered `SKILL.md` files land
  at `.agents/skills/speccy-<verb>/SKILL.md` and contain
  unprefixed command references.
- `speccy-init`'s rendered SKILL.md contains the matching host
  display name (`Claude Code skill pack` vs `Codex skill pack`)
  and install path (`.claude/skills` vs `.agents/skills`).
- Cross-host isolation holds: `--host claude-code` does not
  create any path under `.agents/` or `.codex/`; `--host codex`
  does not create any path under `.claude/`.
- `--force` still preserves user-authored files outside the
  shipped tree (SPEC-0015 REQ-002's classification behaviour is
  unchanged in spirit; it operates on the rendered output set
  rather than a flat copy).

**Behavior:**

- Given a fresh repo with `.claude/`, when
  `speccy init --host claude-code` runs, then
  `.claude/skills/speccy-plan/SKILL.md` contains the literal
  string `/speccy-tasks` (the suggested next step inside the
  skill body) and does not contain a bare `speccy-tasks` token
  without the slash prefix.
- Given a fresh repo with `.codex/`, when
  `speccy init --host codex` runs, then
  `.agents/skills/speccy-plan/SKILL.md` contains the literal
  string `speccy-tasks` (without slash prefix) and does not
  contain `/speccy-tasks`.
- Given a fresh repo, when `speccy init --host claude-code`
  runs, then no path is created under `.agents/` or `.codex/`.

**Covered by:** CHK-003, CHK-004

### REQ-003: Per-host reviewer subagent files

For each shipped reviewer persona, the per-host renderer
materialises a subagent file in the host's native format at the
host's native location.

**Done when:**

- For `--host claude-code`, six files
  `.claude/agents/reviewer-{business,tests,security,style,architecture,docs}.md`
  exist after init. Each opens with a YAML frontmatter block
  declaring at least `name: reviewer-<persona>` and a
  `description:` string, followed by the persona body content.
- For `--host codex`, six files
  `.codex/agents/reviewer-{business,tests,security,style,architecture,docs}.toml`
  exist after init. Each parses as TOML and contains string-typed
  keys `name`, `description`, and `developer_instructions`. The
  `developer_instructions` value is the persona body.
- The persona body text in both rendered forms equals the
  byte content of `resources/modules/personas/reviewer-<persona>.md`
  (see DEC-004 for the bare-`{% include %}` rendering strategy).
- No persona body in `resources/modules/personas/` contains the
  literal substring `"""`; this invariant keeps the Codex
  triple-quoted `developer_instructions = """..."""` block
  unambiguous.

**Behavior:**

- Given a fresh repo with `.claude/`, when
  `speccy init --host claude-code` runs, then a file at
  `.claude/agents/reviewer-security.md` exists, opens with a
  `---` line, and contains the focus bullet
  "Authentication and authorization boundaries" (drawn from the
  persona body).
- Given a fresh repo with `.codex/`, when
  `speccy init --host codex` runs, then
  `.codex/agents/reviewer-security.toml` parses as TOML via
  `toml::from_str::<toml::Value>(&contents)` and has a top-level
  table with keys `name`, `description`, and
  `developer_instructions`.
- Given the workspace, when persona module files are read, then
  none contain the substring `"""`.

**Covered by:** CHK-005, CHK-006

### REQ-004: `/speccy-review` skill prefers host-native subagents

The rendered `/speccy-review` SKILL.md for each host instructs
the main agent to spawn the four default reviewer personas
through the host's native subagent primitive, with the existing
`speccy review --persona X` CLI as an explicit fallback step for
harnesses that don't recognise the subagent type.

**Done when:**

- Claude Code's rendered `.claude/skills/speccy-review/SKILL.md`
  step 4 instructs the main agent to use the `Task` tool with
  `subagent_type: "reviewer-business"`,
  `"reviewer-tests"`, `"reviewer-security"`, and
  `"reviewer-style"` (the default fan-out per the architecture's
  Phase 4 description).
- Codex's rendered `.agents/skills/speccy-review/SKILL.md`
  step 4 instructs the main agent to spawn the four named
  reviewer subagents in prose (per OpenAI's Codex subagents
  docs).
- Both rendered files include a follow-on "fallback" paragraph
  pointing at `speccy review TASK-ID --persona X` for harnesses
  that don't recognise the subagent type.
- The `speccy review` CLI command itself is unchanged: it still
  renders the per-persona prompt to stdout and exits zero.

**Behavior:**

- Given Claude Code's rendered SKILL.md, when searched, then
  step 4 contains the string `subagent_type: "reviewer-` and
  the four default persona names.
- Given Codex's rendered SKILL.md, when searched, then step 4
  does not contain `subagent_type:` and instead references the
  four reviewer subagents by name (e.g.
  `reviewer-business, reviewer-tests, reviewer-security,
  reviewer-style`).
- Given either host's rendered SKILL.md, when searched, then it
  contains the literal `speccy review` CLI command as a
  fallback reference.

**Covered by:** CHK-007

### REQ-005: Dogfooded outputs stay in sync via CI

Speccy's own repo commits the materialised outputs under
`.claude/`, `.agents/`, `.codex/`, and `.speccy/skills/`. CI
runs `speccy init --force` for both hosts in sequence and fails
the build if any committed output drifts from what would be
freshly rendered.

**Done when:**

- `.github/workflows/ci.yml` includes a job step that runs
  `speccy init --force --host claude-code` and
  `speccy init --force --host codex` in order, then
  `git diff --exit-code .claude .codex .agents .speccy/skills`.
- The job step's failure message points contributors at the
  exact commands needed to refresh outputs locally.
- The stale `.claude/commands` path is removed from the CI diff
  check (it has been a no-op since SPEC-0015 moved the install
  destination to `.claude/skills`).

**Behavior:**

- Given a clean repo at HEAD, when both init commands run in
  CI, then `git diff --exit-code` against the listed paths
  succeeds.
- Given a repo where a contributor has edited
  `resources/modules/skills/speccy-review.md` without
  re-running init, when CI runs, then the diff check fails and
  the error message names `.claude/skills/speccy-review/SKILL.md`
  and `.agents/skills/speccy-review/SKILL.md` as drifted.

**Covered by:** CHK-008

### REQ-006: Rendering is deterministic and idempotent

The MiniJinja-backed renderer produces byte-identical output on
repeated runs against the same source, and the renderer's
output contains no unsubstituted template tokens.

**Done when:**

- Rendering the full host pack for either host into a tempdir
  twice in a row produces byte-identical files in both passes.
- No rendered output file (across both hosts and all artifact
  kinds) contains the literal substrings `{{` or `{%` outside
  fenced code blocks where skill bodies intentionally reference
  example template syntax. (DEC-004 establishes that no
  `{% raw %}`-wrapped regions exist in the actual implementation.)
- The embedded `RESOURCES` bundle is non-empty and contains the
  three expected top-level subtrees: `agents/`, `modules/`.

**Behavior:**

- Given a tempdir with `.claude/`, when
  `speccy init --force --host claude-code` runs twice in
  succession, then every file under `.claude/` is byte-identical
  between the two runs (modulo file mtime).
- Given any rendered output file, when its contents are
  searched, then they do not contain `{{` or `{%` outside the
  literal example blocks the skill bodies intentionally
  reference (and those examples live inside fenced code blocks
  the test ignores).
- Given the embedded `RESOURCES` bundle, when iterated, then it
  contains at least the two expected top-level entries
  `agents/` and `modules/`, each non-empty.

**Covered by:** CHK-009, CHK-010

## Design

### Approach

The work splits into three layers.

**Resource layout.** `resources/` replaces `skills/` at the
workspace root. Under it:

```text
resources/
  agents/
    .claude/
      skills/
        speccy-<verb>/SKILL.md.tmpl    (x7)
      agents/
        reviewer-<persona>.md.tmpl     (x6)
    .codex/
      agents/
        reviewer-<persona>.toml.tmpl   (x6)
    .agents/
      skills/
        speccy-<verb>/SKILL.md.tmpl    (x7)
  modules/
    skills/
      speccy-<verb>.md                 (x7)
    personas/
      reviewer-<persona>.md            (x8)
    prompts/
      <name>.md                        (x12)
```

`resources/agents/.<host>/` mirrors the install destination 1:1,
so a contributor reading the path immediately knows where it
lands. The `.tmpl` suffix marks every templated file; the
renderer strips the suffix when writing. `resources/modules/` is
the host-neutral content layer consumed both directly (the
persona resolver and prompt template loader still read from
here) and transitively (the host wrappers `{% include %}` skill
and persona bodies).

Note that the Codex pack writes to two roots:
`.agents/skills/...` (per OpenAI's documented project-local scan
path, established in SPEC-0015 REQ-002) and `.codex/agents/...`
(per the Codex subagents docs, which list the project-local
subagent scan path under `.codex/agents/`). Both destinations
are populated when `--host codex` is selected.

**Templating engine.** MiniJinja 2.x. Pure Rust, ~150KB, stable
2.x API. Three features carry the work:

- `{{ var }}` for host-context substitution
  (`cmd_prefix`, `host_display_name`, `skill_install_path`).
- `{% include "modules/personas/reviewer-X.md" %}` to pull a
  module body into a wrapper. The same mechanism pulls
  `modules/skills/speccy-<verb>.md` into the per-host SKILL.md
  wrapper.
- `{% if host == "claude-code" %}...{% else %}...{% endif %}`
  for the few places module bodies need to diverge — primarily
  step 4 of `speccy-review.md` (Task tool vs. prose-spawn) and
  the `speccy-init` body's install-path mention.

**Subagent generation.** Reviewer subagents are a new shipped
artifact rendered alongside the SKILL.md files. Per-host
wrapper templates `{% include %}` the matching persona body and
add the host-native frontmatter (YAML for Claude Code, flat
TOML for Codex). The wrappers live at
`resources/agents/.<host>/agents/reviewer-<persona>.<ext>.tmpl`;
the renderer materialises them to `.<host>/agents/...`. The
default fan-out the `/speccy-review` skill references is the
SPEC-0009 default (business, tests, security, style); all six
shipped personas have subagent files so explicit-persona
invocation has somewhere to resolve.

### Decisions

#### DEC-001: MiniJinja over alternatives

**Status:** Accepted

**Context:** The renderer needs three features at minimum:
variable substitution for host-specific tokens, file inclusion
so module bodies are single-sourced, and conditional blocks for
the small set of host-specific divergences inside module
bodies. The current `speccy.toml.tmpl` rendering uses
`str::replace` (`speccy-cli/src/init.rs:269-270`); extending it
to handle inclusion and conditionals would re-invent a template
engine badly.

**Decision:** Adopt MiniJinja 2.x as a single workspace
dependency (`minijinja = "2"` in `[workspace.dependencies]`).
Use the `Environment::add_template_owned` API to register
in-memory templates and `Environment::get_template` /
`render` to materialise them. Configure the environment with
`undefined = Undefined::Strict` so missing context variables
fail loudly during the render round-trip test rather than
silently producing empty strings.

**Alternatives:**

- `tinytemplate`. Rejected — lighter than MiniJinja, but its
  inclusion model is bolted on and the Jinja-compatible syntax
  in MiniJinja is more familiar to contributors.
- `handlebars-rust`. Rejected — Mustache-family syntax is less
  expressive for the `{% if %}` divergences and the crate is
  larger than MiniJinja.
- Hand-rolled string substitution extended to handle includes
  and conditionals. Rejected — that path leads to either a
  fragile half-engine or a slow rewrite, neither of which beats
  a 150KB dependency.
- Defer templating; collapse only via cross-symlinking. Rejected
  — symlinks don't survive Windows checkouts cleanly and
  cross-host divergence (slash prefix, install path) still
  needs a substitution step.

**Consequences:** One new workspace dependency. MiniJinja's
strict-undefined mode tightens the failure surface (a missing
context var fails the render round-trip test cleanly).
`speccy.toml.tmpl`'s `str::replace` keeps working until a
follow-up opportunistically migrates it onto the same engine.

#### DEC-002: `resources/agents/.<host>/...` mirrors install destinations

**Status:** Accepted

**Context:** SPEC-0015 already established that the bundle
source should mirror install destinations 1:1, so a contributor
reading the source path can predict where it lands. With a
templating engine introduced, the question becomes how to
arrange host-specific wrappers vs. host-neutral content.

**Decision:** Two top-level directories. `resources/agents/`
holds host-specific wrappers under per-host subtrees named for
the dotfile destination (`.claude/`, `.codex/`, `.agents/`).
The folder structure under each host subtree mirrors the
install destination exactly. `resources/modules/` holds
host-neutral content (`personas/`, `prompts/`, `skills/`). The
renderer walks each host's `resources/agents/.<host>/` subtree
and writes outputs to the matching destination path.

**Alternatives:**

- Flat `resources/agents/<host>/...` (no dot in folder names).
  Rejected — the dot directly cues "this lands at the
  same-named hidden directory in the project", which is the
  property contributors most want to verify at a glance.
- One mixed subtree per host with both wrappers and module
  bodies copied per-host. Rejected — defeats the deduplication
  goal.
- Keep module bodies under `resources/agents/_modules/` so it
  sits next to the host subtrees. Rejected — the rendering
  pipeline already needs two different walk roots
  (per-host walk for outputs, module root for `{% include %}`),
  so the visual co-location adds nothing.

**Consequences:** Codex's two install roots (`.agents/` and
`.codex/`) are visible as two siblings under
`resources/agents/`; no special-case glue lives in code beyond
the renderer iterating over `HostChoice::install_roots()`.
Adding a third host means adding a sibling subtree, not
touching the renderer.

#### DEC-003: Reviewer subagents are a new shipped artifact

**Status:** Accepted

**Context:** Today, `speccy review --persona X` renders the
persona body inline, the harness LLM captures the output, and
splices it into a freshly spawned sub-agent's system prompt.
Both Claude Code and Codex have a host-native subagent primitive
(Claude Code's `Task` tool with `subagent_type`, Codex's
name-based prose spawn) that does the same wiring at a lower
level; the artifact those primitives consume — a markdown or
TOML file at a host-specific path — does not exist today.

**Decision:** Ship six reviewer subagent files per host. Claude
Code subagents are markdown with YAML frontmatter at
`.claude/agents/reviewer-<persona>.md`. Codex subagents are flat
TOML at `.codex/agents/reviewer-<persona>.toml`. Both formats
embed the same persona body via `{% include %}` of
`resources/modules/personas/reviewer-<persona>.md`. The
`/speccy-review` skill prefers the host-native invocation; the
existing `speccy review --persona X` CLI stays as a fallback
step.

**Alternatives:**

- Keep persona invocation CLI-only. Rejected — bypasses the
  harness's native primitive for no reason once the artifact
  layer exists.
- Generate subagents only for the default fan-out (business,
  tests, security, style). Rejected — explicit-persona
  invocation (`speccy review --persona architecture`) would
  have no subagent file to resolve, surfacing as confusing
  asymmetry. Six files is two extra; the cost is negligible.
- Replace the inline rendering entirely. Rejected — keeping
  the CLI fallback preserves correctness for harnesses that
  don't recognise the subagent type.

**Consequences:** The reviewer-persona contract grows a new
dimension: "what does a subagent file look like for this
persona on this host?". DEC-004 below addresses the resulting
TOML-safety invariant on persona body text.

#### DEC-004: Wrapper includes use bare `{% include %}`; persona bodies must avoid `"""`

**Status:** Accepted (amended 2026-05-14; see Changelog row of
the same date — original draft prescribed `{% raw %}`-wrapped
includes, which conflicted with REQ-002 / REQ-004).

**Context:** MiniJinja's `{% include %}` treats the included
file as a template — `{{` and `{%` inside the body are
expanded by the renderer. Two distinct module-body shapes
flow through the same `{% include %}` mechanism:

1. **Skill bodies** (`resources/modules/skills/speccy-<verb>.md`)
   deliberately contain Jinja tokens — `{{ cmd_prefix }}` and
   `{% if host == "claude-code" %}...{% else %}...{% endif %}`
   blocks. REQ-002's "rendered SKILL.md contains `/speccy-tasks`
   for Claude Code and bare `speccy-tasks` for Codex" contract
   and REQ-004's step-4 host-divergence contract both *require*
   the renderer to expand those tokens. Wrapping the include
   in `{% raw %}` would block the very expansion the behaviour
   contracts depend on.
2. **Persona bodies** (`resources/modules/personas/reviewer-<persona>.md`)
   are prose-only Markdown. They currently contain no `{{` or
   `{%` literals. The strict-undefined MiniJinja environment
   (`UndefinedBehavior::Strict` in `speccy-cli/src/render.rs`)
   would surface any future stray token as a render-time error
   that names the offending file, rather than silently emitting
   an empty string.

Separately, the Codex subagent template uses TOML's
triple-quoted string literal (`developer_instructions =
"""..."""`) to wrap the persona-body include; a `"""`
substring anywhere in the persona body would terminate the
string prematurely.

**Decision:** Two invariants.

1. Every `{% include %}` of a module body inside a wrapper
   template uses bare `{% include %}` form — no `{% raw %}`
   wrapping. This is load-bearing for skill-body wrappers
   (where token expansion is the point of the substitution)
   and safe for persona-body wrappers (where bodies contain no
   Jinja tokens today; strict-undefined mode is the safety
   net for any future regression).
2. Persona body files (`resources/modules/personas/*.md`) must
   not contain the literal substring `"""`. A guard test
   (`t010_persona_bodies_have_no_toml_triple_quote` in
   `speccy-cli/tests/skill_packs.rs`) asserts the invariant on
   every persona file, with a failure message naming the
   offending file. The reason is documented inline in the test.

**Alternatives:**

- Wrap every module-body include in
  `{% raw %}{% include %}{% endraw %}` uniformly (the original
  DEC-004 form). Rejected — blocks `{{ cmd_prefix }}` and
  `{% if host %}` expansion that REQ-002 / REQ-004 require,
  breaking the host-divergence behaviour contract.
- Wrap only persona-body includes in `{% raw %}` while leaving
  skill-body includes bare. Rejected — splits the wrapper
  shape into two divergent forms, complicates the renderer's
  trailing-newline contract (a `{% raw %}` block adjacent to
  the include introduces extra whitespace that the helpers
  would have to neutralise), and adds no safety the
  strict-undefined environment plus the TOML-safety invariant
  don't already cover.
- Set `Environment::set_syntax(...)` to use distinctive
  delimiters that don't collide with example content. Rejected
  — fragments the template syntax across the codebase
  (`speccy.toml.tmpl` and other future templates would have
  to choose between styles).
- Pre-process persona bodies to escape `{{` / `{%` literals.
  Rejected — adds a hidden transform layer; reviewers reading
  the persona body and the rendered subagent would see
  different content for no obvious reason.
- Use single-quoted TOML strings for `developer_instructions`.
  Rejected — TOML single-quoted strings forbid newlines, which
  the multi-line persona body needs.

**Consequences:** Contributors editing persona files must avoid
triple-quotes. The invariant test catches the violation at
`cargo test` time with a clear message naming the offending
file. Strict-undefined mode plus the TOML-safety invariant
together cover the residual risk that the original
`{% raw %}` wrapping was meant to address; a future
contributor adding a Jinja token to a persona body will see a
clean render-time error rather than silently-broken output.
Adding a parallel `persona_bodies_have_no_jinja_tokens` guard
test would be cheap belt-and-braces hygiene; tracked as a
follow-up, not blocking v1.

### Interfaces

```rust
// speccy-cli/src/host.rs
impl HostChoice {
    /// Install roots this host writes to under the project root.
    /// Claude Code writes only to `.claude/`. Codex writes to both
    /// `.agents/` (for skills, per SPEC-0015) and `.codex/` (for
    /// subagents, per OpenAI's Codex subagents docs).
    #[must_use = "the install roots drive which resources/agents/ subtrees are rendered"]
    pub const fn install_roots(self) -> &'static [&'static str] {
        match self {
            HostChoice::ClaudeCode => &[".claude"],
            HostChoice::Codex => &[".agents", ".codex"],
        }
    }

    /// MiniJinja template context for this host.
    ///
    /// Keys: `host` (`"claude-code"` | `"codex"`),
    /// `cmd_prefix` (`"/"` | `""`),
    /// `host_display_name` (`"Claude Code"` | `"Codex"`),
    /// `skill_install_path` (`".claude/skills"` | `".agents/skills"`).
    #[must_use = "the template context drives every substitution in resources/agents/<host>/*.tmpl"]
    pub fn template_context(self) -> minijinja::Value { /* ... */ }
}
```

```rust
// speccy-cli/src/embedded.rs
/// Embedded copy of the workspace `resources/` directory.
pub static RESOURCES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../resources");
```

```rust
// speccy-cli/src/init.rs — replaces append_bundle_items's per-host SKILL.md path.
fn render_host_pack(host: HostChoice, project_root: &Utf8Path, force: ForceMode)
    -> Result<Vec<PlanItem>, InitError>;
```

Bundle source layout (new):

```text
resources/
  agents/
    .claude/
      skills/speccy-<verb>/SKILL.md.tmpl        (x7)
      agents/reviewer-<persona>.md.tmpl         (x6)
    .agents/
      skills/speccy-<verb>/SKILL.md.tmpl        (x7)
    .codex/
      agents/reviewer-<persona>.toml.tmpl       (x6)
  modules/
    skills/speccy-<verb>.md                     (x7)
    personas/reviewer-<persona>.md              (x8)
    prompts/<name>.md                           (x12)
```

Install destinations (rendered):

```text
.claude/skills/speccy-<verb>/SKILL.md           (x7, Claude Code)
.claude/agents/reviewer-<persona>.md            (x6, Claude Code)
.agents/skills/speccy-<verb>/SKILL.md           (x7, Codex)
.codex/agents/reviewer-<persona>.toml           (x6, Codex)
.speccy/skills/personas/reviewer-<persona>.md   (x8, both — SPEC-0009 DEC-002 override path)
.speccy/skills/prompts/<name>.md                (x12, both — documented override-future-proofing)
```

### Data changes

- Workspace `Cargo.toml`: add `minijinja = "2"` to
  `[workspace.dependencies]`.
- `speccy-cli/Cargo.toml`: add `minijinja.workspace = true`.
- `speccy-cli/src/embedded.rs`: rename `SKILLS` to `RESOURCES`;
  retarget `include_dir!` from `../skills` to `../resources`;
  refresh doc comments to reflect the two-subtree layout. Add
  non-empty assertions for the `agents/` and `modules/`
  subtrees.
- `speccy-cli/src/host.rs`: add `install_roots()` and
  `template_context()` methods on `HostChoice`. Keep
  `destination_segments()` (still used by SPEC-0015 invariants)
  and `bundle_subpath()` (no longer drives the walk; mark
  with `#[expect(dead_code, reason = "...")]` if unused after
  the migration, or remove if cleanly orphaned).
- `speccy-cli/src/init.rs`: replace `append_bundle_items`'s
  per-host SKILL.md branch with `render_host_pack`, which for
  each install root in `host.install_roots()` walks the
  embedded `resources/agents/<root>/` subtree, renders each
  `.tmpl` file via MiniJinja with the host's template context
  (and a loader rooted at `resources/modules/`), strips the
  `.tmpl` suffix, and writes to
  `<project_root>/<root>/<rel>/<basename without .tmpl>`. The
  plan-print and Create/Overwrite classification remain
  unchanged. Update the shared-persona copy step to read from
  `resources/modules/personas/` instead of
  `skills/shared/personas/`; same for prompts.
- `speccy-core/src/personas.rs`: retarget `include_dir!` from
  `skills/shared/personas` to `resources/modules/personas`. No
  behavioural change to the resolver chain (SPEC-0009 DEC-002
  preserved).
- `speccy-core/src/prompt/template.rs`: retarget `include_dir!`
  from `skills/shared/prompts` to `resources/modules/prompts`.
- Move `skills/shared/personas/*.md` →
  `resources/modules/personas/*.md` (content unchanged).
- Move `skills/shared/prompts/*.md` →
  `resources/modules/prompts/*.md` (content unchanged).
- Consolidate `skills/claude-code/speccy-<verb>/SKILL.md` and
  `skills/codex/speccy-<verb>/SKILL.md` into
  `resources/modules/skills/speccy-<verb>.md` (one body each,
  with `{{ cmd_prefix }}` tokens replacing slash-prefix
  differences and minimal `{% if host == ... %}` blocks where
  prose materially diverges — `speccy-init` for install path
  and host display name, `speccy-review` for step 4
  subagent-vs-prose-spawn).
- Create 14 host wrapper templates (7 for `.claude/skills/`, 7
  for `.agents/skills/`) at
  `resources/agents/.<host>/skills/speccy-<verb>/SKILL.md.tmpl`.
  Each is frontmatter (`name`, `description`) plus a
  `{% raw %}{% include "modules/skills/speccy-<verb>.md" %}{% endraw %}`
  directive.
- Create 12 subagent wrapper templates (6 for `.claude/agents/`
  markdown, 6 for `.codex/agents/` TOML) at
  `resources/agents/.<host>/agents/reviewer-<persona>.<ext>.tmpl`.
- Delete the `skills/` tree once content is migrated.
- Update `resources/modules/skills/speccy-review.md` step 4
  with `{% if host == "claude-code" %}` Task-tool guidance vs.
  `{% else %}` Codex prose-spawn guidance. Retain the
  `speccy review --persona X` CLI as a fallback paragraph.
- Update `.github/workflows/ci.yml` to run
  `speccy init --force --host claude-code` followed by
  `speccy init --force --host codex`, then
  `git diff --exit-code .claude .codex .agents .speccy/skills`.
  Drop the stale `.claude/commands` path from the diff target.
- Refresh Speccy's own dogfooded outputs: `.claude/skills/`,
  `.claude/agents/`, `.agents/skills/`, `.codex/agents/` all
  regenerated via the two init runs and committed.

### Migration / rollback

- Forward: ship as one SPEC's worth of phased tasks. Pre-v1,
  no production installs exist, so there's no user-facing
  migration. The repo's own dogfooded outputs under
  `.claude/skills/`, `.claude/agents/`, `.agents/skills/`, and
  `.codex/agents/` are refreshed by running both init commands
  with `--force` after the templating layer lands; the diff is
  large (creates + deletes) but reviewable as a layout
  migration.
- Rollback: `git revert` of the introducing PR. The legacy
  `skills/` tree is restored; the renderer is removed; the
  dogfooded outputs revert to their pre-SPEC-0016 shape. The
  one workspace dependency (`minijinja`) is dropped by the
  revert.

## Open questions

- [ ] Will Codex consistently spawn the reviewer subagents
      based on the prose instruction "Spawn reviewer-business,
      reviewer-tests, reviewer-security, reviewer-style in
      parallel"? OpenAI's docs describe prose-spawn as the
      canonical pattern, but real-world reliability is a known
      unknown. Mitigation: the CLI fallback is wired
      unconditionally, so a missed prose-spawn degrades to the
      existing inline-render path. Flag in the SPEC-0016
      REPORT.md after dogfooding.
- [ ] Should `bundle_subpath()` on `HostChoice` be removed in
      this spec or kept as legacy state for callers that
      haven't migrated yet? Defer the decision to the
      implementer; if removable cleanly, remove; otherwise
      mark with `#[expect(dead_code, reason = "...")]`.

## Assumptions

- MiniJinja 2.x's `UndefinedBehavior::Strict` causes any
  reference to an undefined variable inside a rendered template
  to return a render-time error (rather than silently emitting
  an empty string). The renderer in `speccy-cli/src/render.rs`
  sets this mode on the `Environment`; it is the safety net
  for any future persona body that accidentally contains a
  `{{` / `{%` token after DEC-004 chose bare `{% include %}`
  over `{% raw %}` wrapping. Verified against the MiniJinja
  2.x reference.
- `include_dir!` continues to walk arbitrary directory depth
  without configuration. Same assumption as SPEC-0015,
  re-verified here for the deeper `resources/agents/.<host>/`
  layout.
- Claude Code's subagent registry walks
  `.claude/agents/reviewer-<persona>.md` and registers the
  filename stem (`reviewer-<persona>`) as a valid
  `subagent_type` value for the `Task` tool. Verified against
  Anthropic's Claude Code subagent docs current as of
  2026-05.
- Codex's subagent registry walks
  `.codex/agents/reviewer-<persona>.toml` and parses each as a
  flat TOML table with at least `name`, `description`, and
  `developer_instructions`. Verified against OpenAI's Codex
  subagents docs current as of 2026-05.
- Persona body files do not currently contain `"""`. The guard
  test asserts the invariant going forward; if a current
  persona already violates it, the migration includes a
  rewrite. (Initial inspection during planning found no
  violations; the test catches future regressions.)

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-14 | agent/claude | Initial draft. Introduces MiniJinja-backed `resources/` layout (host wrappers under `agents/.<host>/`, host-neutral content under `modules/`), reviewer subagent files as a new shipped artifact per host, and a dual-host CI materialization check. Pre-v1, so no installed-user migration story; Speccy's own dogfooded outputs are refreshed in the same PR. |
| 2026-05-14 | agent/claude | DEC-004 amendment. The original "every `{% include %}` is wrapped in `{% raw %}...{% endraw %}`" invariant conflicted with REQ-002 (rendered SKILL.md must contain `/speccy-tasks` for Claude Code and bare `speccy-tasks` for Codex, via `{{ cmd_prefix }}` expansion) and REQ-004 (rendered `speccy-review` step 4 must diverge per host via `{% if host == "claude-code" %}` blocks). The actual T-005 / T-006 / T-009 / T-010 implementations chose bare `{% include %}` over `{% raw %}` to preserve those behaviour contracts; this row aligns DEC-004's text with the as-built design, retitles the decision, and updates REQ-003 / REQ-006 / Assumptions to drop the now-superseded `{% raw %}` references. Strict-undefined mode plus the existing TOML-safety invariant test (`t010_persona_bodies_have_no_toml_triple_quote`) cover the residual risk the original wrapping was meant to address. |

## Notes

This spec is the natural continuation of SPEC-0013 (skill packs
shipped end-to-end) and SPEC-0015 (host-skill directory layout
correctness). Together, those three specs lock down the
"shipped artifacts" surface of Speccy v1: which directories
host packs land in (SPEC-0015), what format each artifact takes
(SPEC-0013), and how the per-host divergence is generated from
a single source of truth (this spec).

The two halves of this spec — the templating layer and the
subagent generation — are bundled rather than split into two
SPECs because the templating layer's existence motivates the
subagent generation (without templates, subagent files would
duplicate persona content yet again), and the subagent
generation exercises a `{% include %}` edge case (TOML
triple-quoted string body wrapping a markdown persona) that
shakes out the templating layer in a way the SKILL.md
collapse alone wouldn't. Landing them together also keeps the
`/speccy-review` skill update — which depends on both — in one
PR.
