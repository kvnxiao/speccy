---
id: SPEC-0039
slug: cross-harness-orchestration-port
title: Cross-harness orchestration port — orchestrator and holistic-gate skills ship from shared modules with thin per-host adapters in both packs
status: implemented
created: 2026-05-22
supersedes: []
---

# SPEC-0039: Cross-harness orchestration port — orchestrator and holistic-gate skills ship from shared modules with thin per-host adapters in both packs

## Summary

Speccy's local pilot of the implementation+review orchestration loop
lives in two hand-written skill bodies at
`.claude/skills/speccy-orchestrate/SKILL.md` and
`.claude/skills/speccy-holistic-review/SKILL.md`, plus two hand-written
sub-agent definitions at `.claude/agents/speccy-holistic-reviewer.md`
and `.claude/agents/speccy-holistic-fixer.md`. These four files exist
only under `.claude/` in this repo; none of them has a counterpart
under `resources/agents/.claude/skills/`, `resources/agents/.agents/skills/`,
`resources/agents/.claude/agents/`, or `resources/agents/.codex/agents/`,
so neither the Claude Code pack nor the Codex pack ships the loop
today. The orchestrator's body explicitly carries a "local-only"
non-goal pending a real-spec exercise of the dispatch contract;
SPEC-0038's landing was the first such exercise and the contract is
stable enough to ship.

The rest of Speccy's shipped skills already use a modules-plus-wrapper
pattern: a host-neutral body lives in `resources/modules/skills/<name>.md`,
included by per-host `SKILL.md.tmpl` wrappers that carry only YAML
frontmatter and a single `{% include %}` directive. Host-variance at
dispatch-primitive boundaries (the syntax for spawning a sub-agent)
uses inline `{% if host == "claude-code" %}...{% else %}...{% endif %}`
blocks within the shared body — see `resources/modules/skills/speccy-review.md`
lines 74-115 for the working precedent. Persona bodies follow the
same pattern at `resources/modules/personas/<name>.md`, included by
per-host agent templates at `resources/agents/.claude/agents/<name>.md.tmpl`
and `resources/agents/.codex/agents/<name>.toml.tmpl`. This SPEC
brings the orchestration loop into that pattern, adds Codex pack
coverage, and pulls along three coupled cleanups.

**Naming.** The lifecycle skill's current name
(`speccy-holistic-review`) collides with the names of its own
sub-agents (`speccy-holistic-reviewer`, `speccy-holistic-fixer`).
The skill is renamed `speccy-holistic-gate` to remove the conflict.
The sub-agents are renamed `holistic-reviewer` and
`holistic-implementer`: dropping the `speccy-` prefix matches the
existing `reviewer-business` / `reviewer-tests` convention, and
replacing `fixer` with `implementer` matches Speccy's existing
implementer-vs-reviewer role vocabulary.

**Codex grant.** Codex now supports native sub-agent spawn from skills
(per `CODEX-SKILLS-AND-SUBAGENTS.md` at the repo root), but the user
must grant the skill explicit permission to spawn. The Codex
orchestrator's rendered SKILL.md ships an inline explanation of how to
grant the permission. The explanation is implemented as a separate
shared-module file (`resources/modules/skills/speccy-orchestrate-codex-grant.md`)
that the Codex `SKILL.md.tmpl` wrapper includes after the host-neutral
body; the Claude wrapper does not include it. This extends the
existing modules pattern with a complementary *additive* mechanism
alongside the existing *substitution* mechanism (see DEC-001).

**Prose-spawn retirement.** Speccy's existing Codex skill templates
use a legacy "prose-spawn" idiom — spawning a sub-agent by naming it
in prose and relying on Codex runtime delegation — that predates
Codex's native sub-agent primitive. The idiom appears in the Codex
branch of `resources/modules/skills/speccy-review.md` (around line 95)
and is pinned by assertions in `speccy-cli/tests/skill_packs.rs`.
This SPEC rewrites every shipped occurrence to use the native
primitive; the new orchestration skills ship using the native
primitive from day one.

The fourth coupled concern is unrelated to orchestration but
similarly belongs in this SPEC's diff: shipped harness content and
production Rust source contain a stray reference to
`ARCHITECTURE.md` at `speccy-core/src/prompt/id_alloc.rs:3`.
`ARCHITECTURE.md` is a project-local byproduct of any repo using
Speccy — speccy-the-project happens to have one, but shipped Speccy
must not bake in the assumption that every user's repo will. The
cleanup is in-scope here because the new modules that REQ-001 and
REQ-003 introduce would otherwise become the next leak point if the
rule did not exist; landing both together is cheaper than two
coordinated PRs.

**Positioning shift.** Until SPEC-0039, Speccy's user-facing
documentation positioned multi-agent orchestration as a *future*
layer that downstream harnesses would build on top of Speccy's
primitive skills (per `AGENTS.md` line 34, `AGENTS.md` line 44,
and `.speccy/ARCHITECTURE.md` "Long-Term Vision"). Once REQ-001
through REQ-003 land, Speccy itself ships an opinionated
implementation+review orchestration loop as an ejected skill in
both packs (`speccy-orchestrate` driving `speccy-holistic-gate`).
The "future layer" framing becomes stale on the day the
orchestration loop becomes a shipped artifact, and the project-local
docs that carry that framing need to be rewritten in the same diff
so the public narrative does not lag the shipped surface. This is a
project-local prose sweep, parallel in shape to the
`ARCHITECTURE.md`-reference sweep of REQ-004 but operating on
narrative positioning rather than implementation hygiene.

## Goals

<goals>
- One source of truth for each of the orchestration loop's two
  skill bodies (`speccy-orchestrate`, `speccy-holistic-gate`) and
  the two holistic-loop persona bodies (`holistic-reviewer`,
  `holistic-implementer`), ejected byte-equivalently into both
  the Claude Code pack and the Codex pack via `speccy init` (modulo
  the inline-substitution and selective-additive-include points
  required by host variance).
- Codex pack ships native sub-agent-spawn syntax for the orchestrator,
  the holistic-gate skill, and both holistic personas; the rendered
  Codex orchestrator SKILL.md includes an inline section explaining
  the sub-agent-spawn permission grant.
- The lifecycle skill name and its sub-agent names do not overlap,
  matching the rest of Speccy's naming conventions (sub-agents have
  no `speccy-` prefix; the implementer role is named `implementer`,
  not `fixer`).
- Production source under `speccy-core/src/` and `speccy-cli/src/`,
  along with all shipped/ejected harness content under `resources/`,
  contain zero literal references to `ARCHITECTURE.md`.
- Existing shipped Codex skill templates use Codex's native
  sub-agent-spawn primitive, not the legacy prose-spawn idiom; the
  matching `speccy-cli/tests/skill_packs.rs` assertions move with
  the rendered output.
- The per-task retry budget (5 rounds) and the holistic drift-fix
  round budget (3 rounds) ship hardcoded inline in the shared module
  bodies; configurability is deliberately not in scope.
- The project-local docs that frame Speccy's product positioning
  (`README.md`, `.speccy/ARCHITECTURE.md`, `AGENTS.md` "Product north
  star") describe the opinionated multi-agent orchestration loop as
  a shipped v1.0 artifact rather than a future layer downstream
  harnesses might build.
</goals>

## Non-goals

<non-goals>
- No end-to-end dogfood validation of the shipped loop on either
  harness inside this SPEC's TASKS.md. The loop is validated
  organically on the next real spec post-ship; gaps surfaced by
  dogfooding become new SPECs.
- No abstract reasoning-level / context-budget / model-class
  identifier across hosts. Each host's agent template pins its own
  model identifier independently; an abstraction layer is deferred
  to a future SPEC if and when a third host is added.
- No new lint code family for shipped-pack content (PCK-NNN,
  SKL-NNN, or otherwise) to enforce the ARCHITECTURE.md decoupling
  or the prose-spawn retirement durably. Both cleanups land as
  one-shot sweeps plus targeted test fixtures.
- No CLI surface change. The seven verbs (`init`, `status`, `next`,
  `check`, `verify`, `lock`, `vacancy`) retain their command shapes,
  JSON envelopes, and lint families. `speccy init` discovers the
  new skills + agents by directory enumeration under
  `resources/agents/<host>/`; no code change in
  `speccy-cli/src/embedded.rs` or `speccy-cli/src/render.rs` is
  required by this SPEC.
- No jinja-variable configurability for the retry budgets (5 / 3).
  Hardcoded inline per the stay-small principle; a future SPEC may
  revisit if a real consumer asks.
- No reconsideration of the four-persona default reviewer fan-out
  (`business`, `tests`, `security`, `style`). The orchestration loop
  delegates per-task review to the existing `/speccy-review` skill,
  which owns persona selection; this SPEC ships the orchestrator
  loop and the holistic gate only.
- No CI-time gate on `speccy verify` for any of the new checks.
  Validation of the rendered shipped-pack outputs lives in
  `speccy-cli/tests/`; `speccy verify` continues to gate only
  SPEC artifact proof shape per its existing lint families.
</non-goals>

## User Stories

<user-stories>
- As Kevin dogfooding Speccy on his own projects, I want the
  orchestration loop body to live in exactly one place so the Claude
  Code and Codex skill packs cannot drift in protocol behavior
  across releases.
- As a Codex-host adopter running `speccy init`, I want the
  orchestrator and holistic-gate skills installed natively in
  `.agents/skills/` with the sub-agent-spawn permission grant
  explained inline in the rendered SKILL.md, so I can run the loop
  without manually porting it from the Claude pilot.
- As a future contributor reading shipped harness content, I want
  zero implicit assumptions about project-local files like
  `ARCHITECTURE.md`, so Speccy works the same way in any repo that
  does not follow speccy-the-project's own layout conventions.
</user-stories>

## Assumptions

<assumptions>
- A1. The existing `resources/modules/` jinja templating handles
  dispatch-primitive differences between Claude and Codex through
  inline `{% if host %}` blocks and selective-include directives at
  the wrapper level — no new template-engine features needed. If
  wrong, REQ-001 expands to include engine work.
- A2. The Codex sub-agent-spawn permission grant is documentable in
  prose inline in the SKILL.md body — no interactive prompt,
  OAuth-style handshake, or sentinel-file mechanic is required. If
  wrong, REQ-003 expands to detection or automation work in
  `speccy init`.
- A3. Codex's model menu can hit the reasoning-quality bar required
  for the `holistic-reviewer` persona (Claude pins `opus[1m]` /
  `high` effort in the pilot). Validated organically post-ship via
  dogfooding rather than in this SPEC's TASKS.md; per the
  corresponding Non-goal.
- A4. `speccy init` discovers skills + agents by directory
  enumeration under `resources/agents/<host>/`, so adding new
  files there is sufficient — no CLI code change in
  `speccy-cli/src/embedded.rs` or `speccy-cli/src/render.rs` is
  required by this SPEC. If wrong, REQ-002 transitively requires
  CLI work.
- A5. Codex's native sub-agent-spawn primitive is invocable from
  skill bodies via a documentable syntax (analogous to Claude
  Code's `Task` tool with `subagent_type`) — not requiring
  CLI-side tooling beyond what `/agent <name>` already does. If
  wrong, REQ-005 expands to coordinate with Codex tooling work.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: Skill bodies and persona bodies for the orchestration loop live in `resources/modules/`

The two orchestration skill bodies (orchestrator, holistic-gate) and
the two holistic-loop persona bodies (drift-reviewer,
drift-implementer) move out of the hand-written pilot locations under
`.claude/` and into the existing `resources/modules/` single-source-of-truth
pattern. Host-variance at sub-agent-spawn dispatch points uses inline
substitution per DEC-001 mechanism A. Additive host-only content
uses the separate-module + selective-wrapper-include pattern per
DEC-001 mechanism B (consumed by REQ-003). Retry-budget integers ship
as literals in the module bodies per DEC-005.

<done-when>
- `resources/modules/skills/speccy-orchestrate.md` exists with the
  host-neutral orchestrator loop body (startup integrity check,
  outer dispatch loop, per-task retry counter with budget 5,
  status-line writes, stop conditions).
- `resources/modules/skills/speccy-holistic-gate.md` exists with the
  host-neutral holistic-loop body (Phase 0 bootstrap, Phase 1 drift
  review + fix loop with budget 3, Phase 2 simplifier polish,
  HOLISTIC.md journal contract, defer-write-before-rollback rule,
  return contract).
- `resources/modules/personas/holistic-reviewer.md` exists with the
  host-neutral drift-reviewer persona body (focus list, round-2+
  scrutiny, verdict-return contract).
- `resources/modules/personas/holistic-implementer.md` exists with
  the host-neutral drift-implementer persona body (scope,
  hygiene-gate, verdict-return contract).
- Each shared body restricts inline
  `{% if host == "claude-code" %}...{% else %}...{% endif %}` blocks
  to sub-agent-spawn dispatch points; non-dispatch prose is
  host-neutral.
- The four hand-written pilot paths no longer exist after factoring:
  `.claude/skills/speccy-holistic-review/SKILL.md`,
  `.claude/agents/speccy-holistic-reviewer.md`,
  `.claude/agents/speccy-holistic-fixer.md` are renamed; the
  pre-existing `.claude/skills/speccy-orchestrate/SKILL.md` is now
  a rendered output of the new template chain rather than a
  hand-written body.
- The retry-budget integers appear as the literals `5` (orchestrator)
  and `3` (holistic-gate) inside the corresponding shared module
  bodies; neither is expressed as a jinja variable, template
  default, or env lookup.
</done-when>

<behavior>
- Given the four module files exist under `resources/modules/`, when
  `speccy init` renders the Claude pack into a fresh workspace, then
  the rendered tree contains
  `.claude/skills/speccy-orchestrate/SKILL.md`,
  `.claude/skills/speccy-holistic-gate/SKILL.md`,
  `.claude/agents/holistic-reviewer.md`, and
  `.claude/agents/holistic-implementer.md`.
- Given the same module files, when `speccy init` renders the Codex
  pack, then the rendered tree contains
  `.agents/skills/speccy-orchestrate/SKILL.md`,
  `.agents/skills/speccy-holistic-gate/SKILL.md`,
  `.codex/agents/holistic-reviewer.toml`, and
  `.codex/agents/holistic-implementer.toml`.
- Given the rendered Claude orchestrator body, when an agent reads
  its dispatch step, then the step instructs invoking the Claude
  `Task` tool with the target sub-agent's `subagent_type`.
- Given the rendered Codex orchestrator body, when an agent reads
  its dispatch step, then the step instructs invoking the Codex
  native sub-agent-spawn primitive against the same target name.
</behavior>

<scenario id="CHK-001">
Given speccy-the-project's `main` branch at HEAD after this SPEC
lands, when the command `ls resources/modules/skills/speccy-orchestrate.md
resources/modules/skills/speccy-holistic-gate.md
resources/modules/personas/holistic-reviewer.md
resources/modules/personas/holistic-implementer.md` runs, then all
four paths exist and `ls` exits 0.
</scenario>

<scenario id="CHK-002">
Given the same checkout, when
`rg -nU 'retry|budget|round' resources/modules/skills/speccy-orchestrate.md`
and `rg -nU 'round|budget' resources/modules/skills/speccy-holistic-gate.md`
each run, then the orchestrator body contains a line that pairs the
literal `5` with the per-task retry budget, and the holistic-gate
body contains a line that pairs the literal `3` with the drift-fix
round budget; neither pairing is expressed as a jinja variable
reference (no `{{ ... }}` surrounds the integer).
</scenario>

<scenario id="CHK-003">
Given the same checkout, when
`test ! -e .claude/skills/speccy-holistic-review/SKILL.md && test ! -e .claude/agents/speccy-holistic-reviewer.md && test ! -e .claude/agents/speccy-holistic-fixer.md`
runs, then it exits 0 (all three legacy paths absent); and the
parallel new-name paths
`.claude/skills/speccy-holistic-gate/SKILL.md`,
`.claude/agents/holistic-reviewer.md`, and
`.claude/agents/holistic-implementer.md` all exist.
</scenario>

<scenario id="CHK-004">
Given the same checkout with no working-tree modifications, when
`speccy init --force` runs and `git status --porcelain .claude/ .agents/ .codex/`
is captured after, then `git status` outputs zero lines — proving
that `.claude/`, `.agents/`, and `.codex/` are regenerated
byte-for-byte from `resources/` and contain no hand-edited content.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Per-host wrappers and agent definitions for the orchestration loop ship in both packs

The new skills' `SKILL.md.tmpl` wrappers ship under both
`resources/agents/.claude/skills/<name>/` and
`resources/agents/.agents/skills/<name>/`. The two holistic-loop
sub-agent definitions ship under both `resources/agents/.claude/agents/`
(as `<name>.md.tmpl`) and `resources/agents/.codex/agents/` (as
`<name>.toml.tmpl`). Sub-agent names drop the `speccy-` prefix and
use `implementer` rather than `fixer` per DEC-002. Each host's
agent template pins its own model identifier independently per
DEC-006; no abstraction across hosts.

<done-when>
- The two Claude `SKILL.md.tmpl` wrappers exist at
  `resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl`
  and `resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl`.
- The two Codex `SKILL.md.tmpl` wrappers exist at
  `resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl`.
- The two Claude agent templates exist at
  `resources/agents/.claude/agents/holistic-reviewer.md.tmpl` and
  `resources/agents/.claude/agents/holistic-implementer.md.tmpl`,
  each with frontmatter that pins `opus[1m]` and the effort level
  carried over from the corresponding pilot file in
  `.claude/agents/`.
- The two Codex agent templates exist at
  `resources/agents/.codex/agents/holistic-reviewer.toml.tmpl` and
  `resources/agents/.codex/agents/holistic-implementer.toml.tmpl`,
  each with TOML that pins a Codex model identifier (not the
  Claude string) and a Codex reasoning-effort setting.
- No template file under `resources/agents/` references the legacy
  sub-agent names (`speccy-holistic-reviewer`, `speccy-holistic-fixer`)
  or the legacy lifecycle skill name (`speccy-holistic-review`).
</done-when>

<behavior>
- Given the four wrapper templates and four agent templates exist,
  when `speccy init` renders into a fresh workspace, then both host
  packs ship the orchestrator + holistic-gate skills plus the two
  holistic-loop sub-agent definitions in their canonical host-native
  locations.
- Given the rendered Claude agent files for the holistic-loop
  personas, when an agent harness reads their frontmatter, then the
  `model` field is `opus[1m]` for both files.
- Given the rendered Codex agent files for the holistic-loop
  personas, when the Codex runtime reads their TOML, then the
  `model` key is a Codex model identifier (not `opus[1m]`).
</behavior>

<scenario id="CHK-005">
Given speccy-the-project's `main` at HEAD after this SPEC lands,
when `ls resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl
resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl
resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl
resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl
resources/agents/.claude/agents/holistic-reviewer.md.tmpl
resources/agents/.claude/agents/holistic-implementer.md.tmpl
resources/agents/.codex/agents/holistic-reviewer.toml.tmpl
resources/agents/.codex/agents/holistic-implementer.toml.tmpl` runs,
then all eight paths exist and `ls` exits 0.
</scenario>

<scenario id="CHK-006">
Given the same checkout, when
`rg -n '^model:' resources/agents/.claude/agents/holistic-reviewer.md.tmpl resources/agents/.claude/agents/holistic-implementer.md.tmpl`
runs, then both files print a line containing the literal substring
`opus[1m]`; and `rg -n '^model =' resources/agents/.codex/agents/holistic-reviewer.toml.tmpl resources/agents/.codex/agents/holistic-implementer.toml.tmpl`
prints a line in each file whose value is non-empty and does not
contain `opus`.
</scenario>

<scenario id="CHK-007">
Given the same checkout, when
`rg -n 'speccy-holistic-(?:review|reviewer|fixer)' resources/`
runs, then it prints zero matches — proving no shipped template
references the legacy lifecycle skill name or the legacy sub-agent
names.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Codex permission-grant section ships as a separate module composed at the wrapper level

Codex requires the user to explicitly grant a skill permission to
spawn sub-agents. The grant explanation ships as a separate
shared-module file. The Codex `speccy-orchestrate` `SKILL.md.tmpl`
wrapper includes the host-neutral body and then the grant module;
the Claude wrapper includes only the host-neutral body. Per DEC-001
mechanism B, additive host-only content uses this
separate-module + selective-include pattern rather than an inline
`{% if host %}` block in the shared body so the composition is
visible at the wrapper.

<done-when>
- `resources/modules/skills/speccy-orchestrate-codex-grant.md`
  exists, containing a self-contained explanation of how a Codex
  user grants the orchestrator skill permission to spawn
  sub-agents.
- `resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl`
  contains a `{% include %}` directive that pulls in
  `modules/skills/speccy-orchestrate-codex-grant.md` after the
  host-neutral body include.
- `resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl`
  contains no reference to `speccy-orchestrate-codex-grant.md` and
  no Codex-permission-grant prose.
- The rendered Codex `.agents/skills/speccy-orchestrate/SKILL.md`
  contains the grant explanation; the rendered Claude
  `.claude/skills/speccy-orchestrate/SKILL.md` does not.
</done-when>

<behavior>
- Given the grant module file exists and the Codex wrapper includes
  it, when an agent reads the rendered Codex
  `.agents/skills/speccy-orchestrate/SKILL.md`, then the body
  carries an inline section describing the sub-agent-spawn
  permission grant before the agent attempts to dispatch any
  sub-agent.
- Given the same source artifacts, when an agent reads the rendered
  Claude `.claude/skills/speccy-orchestrate/SKILL.md`, then no
  permission-grant section appears (Claude has no equivalent gate).
</behavior>

<scenario id="CHK-008">
Given speccy-the-project's `main` at HEAD after this SPEC lands,
when `ls resources/modules/skills/speccy-orchestrate-codex-grant.md`
runs, then the path exists and `ls` exits 0.
</scenario>

<scenario id="CHK-009">
Given the same checkout, when
`rg -n 'speccy-orchestrate-codex-grant' resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl`
runs, then it prints at least one match; and
`rg -n 'speccy-orchestrate-codex-grant|grant.*subagent|permission.*spawn' resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl`
prints zero matches.
</scenario>

<scenario id="CHK-010">
Given the same checkout after `speccy init --force` runs, when
`rg -c 'permission' .agents/skills/speccy-orchestrate/SKILL.md`
and `rg -c 'permission' .claude/skills/speccy-orchestrate/SKILL.md`
each run, then the Codex render reports at least one match and the
Claude render reports zero matches in any line referencing the
sub-agent-spawn grant.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Production source and shipped harness content contain zero literal references to `ARCHITECTURE.md`

`ARCHITECTURE.md` is a project-local byproduct of any repo using
Speccy. Speccy-the-project happens to have one, but shipped Speccy
must not bake in the assumption that every user's repo will.
Production Rust source under `speccy-core/src/` and `speccy-cli/src/`,
plus all shipped/ejected harness content under `resources/`, lose
their literal references. Project-local tests, READMEs, the
speccy-the-project `AGENTS.md`, and existing SPEC history under
`.speccy/specs/` remain exempt — those are project-local artifacts,
not shipped content.

<done-when>
- Zero matches for the literal substring `ARCHITECTURE.md` under
  `speccy-core/src/` after this SPEC lands.
- Zero matches under `speccy-cli/src/`.
- Zero matches under `resources/` (covers both
  `resources/modules/` and `resources/agents/`).
- The exempt project-local references remain — `AGENTS.md` and
  per-crate `README.md` files still mention `ARCHITECTURE.md` where
  they did before, and `.speccy/specs/` history is untouched.
- The known production-source leak at
  `speccy-core/src/prompt/id_alloc.rs:3` is resolved by rewriting
  the doc comment to cite only the SPEC that the constant was
  derived from (SPEC-0005 DEC-005) rather than ARCHITECTURE.md.
</done-when>

<behavior>
- Given the post-SPEC repo at HEAD, when a future contributor
  searches for `ARCHITECTURE.md` references in shipped/production
  paths, then no match returns — proving that the shipped artifact
  surface makes no assumption about the project-local file.
- Given the same repo, when the same contributor searches in
  exempt project-local paths, then matches still return — proving
  the cleanup was scoped, not over-broad.
</behavior>

<scenario id="CHK-011">
Given speccy-the-project's `main` at HEAD after this SPEC lands,
when `rg -n 'ARCHITECTURE\.md' speccy-core/src speccy-cli/src resources`
runs, then it prints zero matches.
</scenario>

<scenario id="CHK-012">
Given the same checkout, when `rg -c 'ARCHITECTURE\.md' AGENTS.md`
runs, then it reports at least one match (the project-local
`AGENTS.md` continues to reference `ARCHITECTURE.md` as it did
before); and `rg -l 'ARCHITECTURE\.md' .speccy/specs/` prints at
least one path (existing SPEC history is preserved).
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Shipped Codex templates use Codex's native sub-agent-spawn primitive, not the legacy prose-spawn idiom

Codex's native sub-agent-spawn primitive is the supported pattern
(per `CODEX-SKILLS-AND-SUBAGENTS.md`). The legacy "prose-spawn"
idiom — spawning a sub-agent by naming it in prose and relying on
Codex runtime delegation — predates the native primitive and persists
in `resources/modules/skills/speccy-review.md`'s Codex branch and in
the test assertions that pin its rendered output. This requirement
sweeps the idiom out of every shipped occurrence and updates the
matching tests. The new orchestration skills ship using the native
primitive from day one (implicit in REQ-001's host-substitution
rule; reaffirmed here so reviewers do not miss it).

<done-when>
- `resources/modules/skills/speccy-review.md`'s Codex branch
  describes sub-agent invocation using Codex's native primitive
  rather than the prose-spawn idiom.
- Zero occurrences of the case-insensitive substring `prose-spawn`
  remain in any file under `resources/modules/`,
  `resources/agents/.agents/`, or `resources/agents/.codex/`.
- `speccy-cli/tests/skill_packs.rs` assertions that previously
  pinned the rendered Codex prose-spawn wording now pin the new
  native-primitive wording; the test suite passes after the
  update.
- The new orchestration skills' Codex branches in
  `resources/modules/skills/speccy-orchestrate.md` and
  `resources/modules/skills/speccy-holistic-gate.md` use the
  native primitive on first ship.
</done-when>

<behavior>
- Given the post-SPEC repo, when a future contributor reads the
  rendered Codex `.agents/skills/speccy-review/SKILL.md`, then the
  dispatch step references Codex's native sub-agent-spawn syntax
  and no longer instructs "prose-spawn the four reviewer
  sub-agents by name".
- Given the same repo, when `cargo test -p speccy-cli skill_packs`
  runs, then it passes — the test assertions match the new
  rendered output.
</behavior>

<scenario id="CHK-013">
Given speccy-the-project's `main` at HEAD after this SPEC lands,
when `rg -in 'prose.?spawn' resources/modules resources/agents/.agents resources/agents/.codex`
runs, then it prints zero matches.
</scenario>

<scenario id="CHK-014">
Given the same checkout, when `cargo test -p speccy-cli --test skill_packs`
runs, then it exits 0 — the previously prose-spawn-pinned
assertions in `speccy-cli/tests/skill_packs.rs` have been updated
to match the new native-primitive rendered output.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Project-local docs reposition the orchestration loop as a shipped v1.0 artifact

Three project-local doc files frame Speccy's positioning to readers
who land on the repo: `README.md` (public-facing introduction),
`.speccy/ARCHITECTURE.md` (canonical design reference), and
`AGENTS.md` "Product north star" (the always-loaded vision block).
Each carries language that defers multi-agent orchestration to a
*future* layer downstream harnesses would build on top of Speccy
(`AGENTS.md` line 34 "Long-term, speccy is the substrate underneath
multi-agent harnesses"; `AGENTS.md` line 44 "(Future) multi-agent
harnesses"; `.speccy/ARCHITECTURE.md` "Long-Term Vision" section).
Once REQ-001 through REQ-003 land, that framing is stale: Speccy
itself ships the orchestrator. This requirement rewrites the three
files in the same diff so the narrative matches the shipped surface
on the day SPEC-0039 ships. The scope is prose only — no code, no
schema, no CLI surface change.

<done-when>
- `README.md` mentions `/speccy-orchestrate` (or the canonical
  rendered skill name) alongside the existing
  `/speccy-plan`/`/speccy-tasks`/`/speccy-work`/`/speccy-review`/`/speccy-ship`
  recipe table, framing it as the opinionated end-to-end driver
  rather than a future layer.
- `.speccy/ARCHITECTURE.md` "Long-Term Vision" no longer lists
  multi-agent orchestration among the deferred future layers; the
  shipped orchestration loop is described as a current artifact of
  the skill layer.
- `AGENTS.md` "Product north star" no longer describes the
  multi-agent orchestrator as `(Future)` or as something
  *downstream* harnesses will build. The shipped orchestration loop
  appears among the v1.0 outcomes.
- The new `AGENTS.md` framing remains compatible with Speccy's
  stay-small principle: the orchestrator is described as an
  *opinionated default* that ships in the skill packs, not as part
  of the CLI surface. The seven-command CLI surface is unchanged.
- No existing positioning principle is reversed by the rewrite:
  the "Feedback, not enforcement" stance, the deterministic-core /
  intelligent-edges split, and the no-CLI-LLM-calls rule all
  survive verbatim.
</done-when>

<behavior>
- Given a reader who arrives at the repo `README.md` post-SPEC,
  when they scan the slash-command recipe table, then they see
  `/speccy-orchestrate` listed as a shipped recipe alongside the
  five existing phases — not as a future layer.
- Given the same reader opening `.speccy/ARCHITECTURE.md` to the
  "Long-Term Vision" section, when they read the list of "future
  layers (not v1)", then multi-agent orchestration is not on it.
- Given an AI agent loading `AGENTS.md` into a planner prompt,
  when the agent reads the "Product north star" block, then the
  block describes the multi-agent orchestrator as a shipped v1.0
  outcome (consistent with the orchestration loop the agent itself
  may be running under).
</behavior>

<scenario id="CHK-015">
Given speccy-the-project's `main` branch at HEAD after this SPEC
lands, when `rg -n 'speccy-orchestrate' README.md` runs, then it
prints at least one match in a line introducing the orchestrator
recipe.
</scenario>

<scenario id="CHK-016">
Given the same checkout, when the
"Long-Term Vision" section of `.speccy/ARCHITECTURE.md` is read,
then no bullet under "Future layers (not v1)" references multi-agent
orchestration; and `rg -n '\(Future\) multi-agent' AGENTS.md` prints
zero matches.
</scenario>

<scenario id="CHK-017">
Given the same checkout, when
`rg -nU 'Long-term, speccy is the substrate underneath multi-agent harnesses' AGENTS.md`
runs, then it prints zero matches — proving the legacy "downstream
harnesses build on top of Speccy" framing has been rewritten.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
Host variance in shipped skill bodies uses two complementary
mechanisms:

- **Substitution** — every host gets a section, contents differ —
  uses inline `{% if host == "claude-code" %}...{% else %}...{% endif %}`
  blocks inside the shared module body. Precedent:
  `resources/modules/skills/speccy-review.md` lines 74-115.
- **Additive** — one host gets an extra section, others do not — uses
  a separate shared-module file that only the relevant per-host
  `SKILL.md.tmpl` wrapper includes. Introduced in this SPEC by REQ-003
  for the Codex permission-grant section.

The two mechanisms target different shapes of variance. Forcing
additive content through inline `{% if %}` blocks would bury the
composition in the shared body and obscure which hosts render
which sections; forcing substitution through separate-module files
would duplicate the surrounding context across N files per
substitution point. Both mechanisms coexist in the pack.
</decision>

<decision id="DEC-002">
Sub-agent names drop the `speccy-` prefix and use `implementer`
rather than `fixer`. The renames are:
`speccy-holistic-reviewer` → `holistic-reviewer`,
`speccy-holistic-fixer` → `holistic-implementer`.

Reasoning: the existing reviewer-persona sub-agents
(`reviewer-business`, `reviewer-tests`, `reviewer-security`,
`reviewer-style`, `reviewer-architecture`, `reviewer-docs`) carry no
`speccy-` prefix. The pilot's `speccy-holistic-*` prefix was an
inconsistency from its hand-written origin. "Implementer" matches
Speccy's existing implementer-vs-reviewer role vocabulary; "fixer"
was a pilot-era one-off term that did not survive the consistency
review.
</decision>

<decision id="DEC-003">
The lifecycle skill is renamed `speccy-holistic-review` →
`speccy-holistic-gate`.

Reasoning: the pilot name collides with the names of its own
sub-agents (`holistic-reviewer`, `holistic-implementer` post-DEC-002).
A user invoking `/speccy-holistic-review` is invoking the lifecycle
loop, not the reviewer sub-agent; the name suggested otherwise. "Gate"
captures the skill's role as the pre-ship gating step that either
passes the SPEC or returns a `fail` verdict for the caller to act on.
The alternative `speccy-holistic` (terse noun) was considered;
`speccy-holistic-gate` was preferred for its semantic specificity.
</decision>

<decision id="DEC-004">
The `ARCHITECTURE.md` decoupling (REQ-004) and the prose-spawn
retirement (REQ-005) land as one-shot sweeps plus targeted test
fixtures, not as new lint code families in `speccy verify`.

Reasoning: Speccy's lint engine targets markdown SPEC artifacts
(`SPC-*`, `REQ-*`, `TSK-*`, `RPT-*`, `QST-*`, `JNL-*`), not arbitrary
Rust source or shipped template body content. Adding lint families
for shipped-content hygiene would expand the lint engine's surface
area for two narrow, one-shot cleanups; the size of the cleanup does
not justify the engine work. If shipped-content hygiene becomes a
recurring drift source in future SPECs, a `PCK-*` family becomes
load-bearing then.
</decision>

<decision id="DEC-005">
The orchestrator's per-task retry budget (5 rounds) and the
holistic-gate's drift-fix round budget (3 rounds) ship as hardcoded
integer literals in the shared module bodies. They are not jinja
variables, template defaults, or environment lookups.

Reasoning: configurable knobs imply a consumer who needs them.
The pilot bounds are derived from one real-spec exercise (SPEC-0038)
and have not surfaced a tuning need. Adding configurability now
would invite premature flexibility per Speccy's stay-small
principle. A future SPEC can extract the integers to jinja variables
if a real consumer asks; the cost of doing so later is low (one
mechanical refactor) and the cost of doing so now is the maintenance
of a config surface no one uses.
</decision>

<decision id="DEC-006">
Per-host agent templates pin their model identifier independently;
no abstraction across hosts (e.g., `reasoning_level: high` mapped to
host-native model identifiers) is introduced in this SPEC.

Reasoning: an abstraction layer would justify its complexity only
when a third host (gemini-cli, cursor, etc.) is added. Today there
are two hosts and two adjacent strings. Side-by-side pins are
locally legible; abstraction is deferred to a future SPEC if and
when the number of hosts grows enough to make per-host pins
repetitive in a load-bearing way.
</decision>

## Notes

End-to-end dogfood validation of the shipped loop on both harnesses
is deliberately out of scope for this SPEC's TASKS.md. The
implementation lands; Kevin exercises it organically on the next
real spec; if dogfooding surfaces gaps in the rendered output
(Codex sub-agent grant flow incorrect, holistic-gate body rendering
wrong, etc.), those gaps become new SPECs with concrete failure
modes attached. This is the same dogfood-as-acceptance-test pattern
Speccy used for SPECs 0023, 0033, and 0037.

The model-pinning abstraction described under DEC-006 is the most
likely follow-up SPEC. If/when a third host pack is added, the
side-by-side pins will start to feel repetitive across N hosts × M
personas; that is the trigger for extracting them.

The `PCK-NNN` lint code family described under DEC-004 is a
potential follow-up if shipped-content hygiene surfaces as a
recurring drift source. Two cleanups in one SPEC is not enough
signal; three or more in a row would be.

The `.claude/skills/speccy-orchestrate/SKILL.md` pilot file is
*not* deleted by this SPEC — it is regenerated as a rendered output
from the new `resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl`
template. Only the three legacy paths whose names change
(`speccy-holistic-review`, `speccy-holistic-reviewer`,
`speccy-holistic-fixer`) disappear post-rename. CHK-004's
`git status --porcelain` check verifies the regenerated outputs
match the new source byte-for-byte.

The rejected alternative framings considered during brainstorm
(two separate SPECs, `PCK-NNN` lint codification, hand-written
side-by-side Codex copies, `.speccy/orchestration/PROTOCOL.md` as
a shared directory) are recorded in the brainstorm chat log; they
do not become decisions because the chosen path supersedes each
on grounds documented above.

## Open Questions

(None.)

## Changelog

<changelog>
| Date       | Reason                                                   | Author     |
|------------|----------------------------------------------------------|------------|
| 2026-05-22 | Initial draft. Factor the orchestration loop's two skill bodies (`speccy-orchestrate`, `speccy-holistic-gate`) and the two holistic-loop persona bodies (`holistic-reviewer`, `holistic-implementer`) from the hand-written Claude pilot into the existing `resources/modules/` single-source-of-truth pattern; ship per-host wrappers and agent templates in both the Claude Code and Codex packs (DEC-002 + DEC-003 renaming conventions, DEC-006 side-by-side model pins); introduce the additive-via-separate-module + selective-wrapper-include host-variance mechanism (DEC-001 mechanism B) for the Codex sub-agent-spawn permission grant; retire the legacy prose-spawn idiom in existing shipped Codex templates; strip stray `ARCHITECTURE.md` references from production Rust source and all shipped harness content (DEC-004 one-shot sweep); hardcode the per-task retry budget (5 rounds) and the holistic drift-fix round budget (3 rounds) inline in the shared module bodies (DEC-005). Motivated by the cross-harness orchestration ask brainstormed in the originating chat, the lifecycle/sub-agent naming conflict observed during framing, and the parallel ARCHITECTURE.md decoupling and prose-spawn retirement requests folded in during brainstorm. | Kevin Xiao |
| 2026-05-22 | Amend: add REQ-006 covering the project-local doc rewrite (`README.md`, `.speccy/ARCHITECTURE.md`, `AGENTS.md` "Product north star") to reposition the multi-agent orchestration loop as a shipped v1.0 artifact rather than a `(Future)` layer downstream harnesses would build. Expand the SPEC Summary with a "Positioning shift" paragraph naming the three doc surfaces whose framing becomes stale once REQ-001-REQ-003 land. Add the corresponding bullet to `<goals>`. The shift is project-local prose; no code, schema, or CLI surface change. Triggered mid-loop by the realization that the v1.0 narrative on those three docs will lag the shipped surface unless rewritten in the same diff. | Kevin Xiao |
</changelog>
