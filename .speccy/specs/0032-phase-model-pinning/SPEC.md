---
id: SPEC-0032
slug: phase-model-pinning
title: Per-phase model and effort pinning across the lifecycle
status: in-progress
created: 2026-05-19
supersedes: []
---

# SPEC-0032: Per-phase model and effort pinning across the lifecycle

## Summary

Every shipped Speccy skill and reviewer subagent currently inherits
the host session's model and effort. The mechanical phases
(`speccy-tasks`, `speccy-work`, `speccy-ship`, `speccy-init`, the
`speccy-review` orchestrator) and the six adversarial reviewer
personas (`reviewer-business`, `reviewer-tests`,
`reviewer-architecture`, `reviewer-security`, `reviewer-style`,
`reviewer-docs`) all run at whatever the user's session is set to at
invocation time. When that session is at Opus 4.7 / xhigh — the
recommended default for the framing-sensitive opening phases — every
downstream mechanical phase pulls Opus rates and Opus latency on
work that Sonnet 4.6 at medium effort would dispatch in a fraction
of the time and tokens.

SPEC-0032 pins each phase to the model and effort the work actually
needs, exploiting the asymmetry the rest of Speccy already trades
on: drift-catching lives where Opus runs, bulk volume lives where
Sonnet runs, and pure scaffolding lives where Haiku runs. The
asymmetric assignment is deliberate. Business, tests, and
architecture reviewers carry semantic adversarial load and stay on
Opus at xhigh. Security carries pattern-plus-judgment load and
moves to Sonnet at high. Style and docs reviewers carry pure
pattern load and move to Sonnet at medium. The five mechanical
phases split into Sonnet/medium for the implementer-grade phases
(`tasks`, `work`, `ship`) and Haiku for the scaffolding and
dispatch phases (`init`, the `review` orchestrator itself).

The cost-and-time win materializes only if the pin actually applies
to the work, which means the pin must outlive a single assistant
turn. Claude Code's `model:` skill frontmatter applies for the
current turn only, which is fine for one-shot invocations but bloats
the main session's context with every worker's intermediate tool
output across a multi-phase loop. To get a sustained pin plus
context isolation, the four implementing phases (`tasks`, `work`,
`ship`, `init`) gain `context: fork` plus a dedicated `agent:`
target in their skill frontmatter, and Speccy ships matching
`.claude/agents/speccy-<phase>.md` subagent definitions that carry
the actual model and effort. The forked subagent runs in its own
isolated context across however many internal turns the work
requires; only its final return message reaches the parent session.
The `/speccy-review` skill is the one exception: it must keep
Task-tool access to fan out to the six reviewer subagents, and
Claude Code's documented rule is that subagents cannot spawn other
subagents. So `/speccy-review` pins via direct skill frontmatter
(no fork) and accepts the single-turn pin scope; the fan-out
dispatcher is mechanical enough that one Haiku turn is sufficient.

Codex's CLI exposes per-subagent `model` and `model_reasoning_effort`
in `.codex/agents/*.toml` files, but Codex skills do not accept a
model field, and Codex does not expose a Claude-Code-equivalent
`context: fork` primitive. The Codex path is therefore asymmetric:
the cost-and-time win is opt-in via `/agent <name>`, not automatic
on `/speccy-<phase>` invocation. The five new `.codex/agents/`
TOML files plus the existing six reviewer TOML files carry the
same pin assignments as the Claude Code half; the Codex skill
bodies for the mechanical phases gain a one-line pointer to the
`.codex/agents/<phase>.toml` invocation path so users discover the
pinned execution path without consulting the README. The asymmetry
itself is documented under a new "Model pinning and host
asymmetry" section in the project README.

The three conversational skills (`speccy-brainstorm`, `speccy-plan`,
`speccy-amend`) stay unpinned. They are multi-turn dialogues with
the user; Claude Code's skill `model:` override only applies for
the first assistant turn after invocation, so a frontmatter pin
would only steer the opening response and revert immediately. The
assumption is that the user starts a session at the latest Opus
generation at xhigh or max effort, which is the contract-writing
tier these phases need; if the session is on something cheaper,
the user is in control of switching via `/model`.

Pin values use Anthropic model aliases (`opus`, `sonnet`, `haiku`)
rather than versioned model IDs (`claude-opus-4-7`, etc.). Aliases
float forward as Anthropic ships new generations; the user is free
to lock to a specific version by editing the ejected files
post-`speccy init`. The Haiku 4.5 generation does not yet support
the new `effort` parameter that Opus and Sonnet expose; Haiku pins
declare `model:` alone with no `effort:` field. A future Haiku
release that adopts the effort enum can absorb the field via a
future amendment without breaking the current shape.

All ejection paths (the `.claude/skills/`, `.claude/agents/`,
`.codex/agents/`, and `.agents/skills/` trees rendered by
`speccy init` into a fresh project) carry the same pin assignments
that Speccy uses in its own in-tree dogfood pack. The change is
bounded: skill frontmatter edits on five Claude Code skills (four
add `context: fork` and an `agent:` target; one adds direct
`model:` and `effort:`); four new Claude Code subagent files;
five new Codex agent TOML files; frontmatter additions to the six
existing reviewer files on each host; one-line pointer additions
to four Codex phase-worker skill bodies; one new README section;
one README audit pass for current-repo-state accuracy.

## Goals

<goals>
- Every invocation of `/speccy-tasks`, `/speccy-work`, or
  `/speccy-ship` on Claude Code executes the phase's work in a
  forked subagent at Sonnet / medium effort, regardless of the
  parent session's model and effort.
- Every invocation of `/speccy-init` on Claude Code executes its
  scaffolding work in a forked subagent at Haiku (no `effort:`
  field), regardless of the parent session's model.
- Every invocation of `/speccy-review` on Claude Code runs its
  orchestrator turn at Haiku via direct skill frontmatter pin (no
  fork), preserving Task-tool access to spawn the six reviewer
  subagents.
- Each forked phase worker's intermediate tool output stays inside
  the subagent's isolated context; only the subagent's final return
  message reaches the parent session, keeping the parent context
  clean across a multi-phase loop.
- Each reviewer subagent on Claude Code runs at the model and
  effort matching its work shape: `reviewer-business`,
  `reviewer-tests`, and `reviewer-architecture` at Opus / xhigh;
  `reviewer-security` at Sonnet / high; `reviewer-style` and
  `reviewer-docs` at Sonnet / medium.
- Each reviewer subagent on Codex carries the same pin assignment
  as its Claude Code counterpart, expressed in
  `.codex/agents/reviewer-<persona>.toml` via the `model` and
  `model_reasoning_effort` fields (or `model` alone for Haiku-tier
  agents, if any reviewer is Haiku-pinned in future).
- Each mechanical phase on Codex (`speccy-tasks`, `speccy-work`,
  `speccy-ship`, `speccy-init`, `speccy-review`) gains a
  `.codex/agents/speccy-<phase>.toml` subagent file with the same
  pin assignment as its Claude Code counterpart, invocable via
  `/agent <name>`.
- Each Codex phase-worker skill body (`.agents/skills/speccy-<phase>/SKILL.md`)
  carries a one-line pointer to the corresponding
  `.codex/agents/speccy-<phase>.toml` invocation path so users
  discover the pinned-execution path without consulting the
  README.
- The pin shape respects each model family's effort-parameter
  support: Opus pins include an `effort:` field with values up to
  `xhigh` or `max`; Sonnet pins include an `effort:` field with
  values up to `max` (never `xhigh`, which is Opus-only); Haiku
  pins set `model:` alone with no `effort:` field, since Haiku 4.5
  uses budget-based extended thinking rather than the new effort
  enum.
- `model:` (Claude Code) and `model` (Codex) values use Anthropic
  aliases (`opus`, `sonnet`, `haiku`) and the Codex / OpenAI
  equivalents, not versioned model IDs; the ejected files are
  user-editable so a project that wants to lock to a specific
  version can do so post-eject.
- `speccy init` in a fresh project renders all new and updated
  agent files with the same pin assignments as Speccy's own in-tree
  dogfood pack.
- The phase-worker subagent body on Claude Code derives from
  `resources/modules/skills/speccy-<phase>.md` via MiniJinja
  `{% include %}`, the same shared-body pattern the reviewer agents
  already use through `resources/modules/personas/`. The Codex
  agent TOML files include their bodies the same way.
- The project README gains a new "Model pinning and host
  asymmetry" section that names the assignment table, explains the
  Claude Code automatic fork vs Codex `/agent <name>` opt-in
  asymmetry, and tells users how to override individual pins.
- The rest of the README is audited and refreshed to match the
  current state of the repository (`speccy-cli` commands, shipped
  skill pack contents, current ejection paths, and any other prose
  that has drifted since the last refresh).
- All four standard-hygiene gates (`cargo test`, `cargo clippy`,
  `cargo +nightly fmt --all --check`, `cargo deny check`) exit 0
  against the post-SPEC workspace.
</goals>

## Non-goals

<non-goals>
- No CLI subcommand surface change. No `speccy model`,
  `speccy pin`, `--model` flag, or per-invocation override knob.
  Pins live in skill and agent frontmatter only; the deterministic
  CLI does not read or write them.
- No pin enforcement. A user who runs `/speccy-work` on Claude
  Code can still override the fork by editing the SKILL.md frontmatter
  in their project. A user on Codex can ignore the `/agent` pointer
  and run the skill at session model. Speccy makes the recommended
  shape easy and obvious; it does not block alternative paths.
- No conversational-skill pins. `/speccy-brainstorm`,
  `/speccy-plan`, and `/speccy-amend` do not gain `model:`,
  `effort:`, `context:`, or `agent:` frontmatter additions. They
  continue to run at the parent session's model and effort
  throughout their multi-turn dialogues.
- No version-locked model IDs. `model:` values are aliases
  (`opus`, `sonnet`, `haiku`) so the pin floats forward as
  Anthropic ships new generations. A user who wants their loop to
  behave deterministically across a model upgrade can edit the
  ejected files to lock a specific version; Speccy does not lock
  on their behalf.
- No measurement step. The SPEC does not include a benchmarking
  task that records token-and-latency deltas before and after the
  pin lands. Dogfooding governs whether an individual pin needs
  re-tuning, and a re-pin is a future amendment, not a
  measurement-loop product.
- No Haiku `effort:` field. Even though future Haiku generations
  may grow the effort enum, the current Haiku 4.5 uses budget-based
  thinking; the SPEC ships `model:` alone for Haiku pins and
  accepts that a future amendment will add `effort:` once the
  enum becomes available.
- No Codex automatic-dispatch implementation. The cost-and-time win
  on Codex remains opt-in via `/agent <name>`. The SPEC does not
  ship workarounds for Codex's missing skill-spawns-subagent
  primitive; the asymmetry is documented and accepted indefinitely
  until Codex CLI ships a programmatic equivalent.
- No new `resources/modules/agents/` directory. The phase-worker
  subagent bodies share the existing `resources/modules/skills/`
  source via templating; we do not duplicate the body or split it
  across a new directory.
- No per-phase configuration file. Pins live in their respective
  skill and agent frontmatter; there is no central registry, no
  `.speccy/pins.toml`, no `[pins]` table in any existing file. A
  user who wants to audit the assignment reads the frontmatter
  directly.
- No automatic README sync. The README refresh in this SPEC is a
  one-time audit; we do not introduce a meta-test that asserts
  README content matches repo state. Future drift between README
  and repo is caught by reviewer-docs at review time, not by CI.
- No retroactive pin assignment to historical specs or to
  Speccy's own previously-rendered host pack. The pins apply from
  this SPEC's tasks landing forward; pre-existing ejected packs in
  user projects are unaffected until those users re-run
  `speccy init` or copy the new frontmatter manually.
- No bump of `schema_version` in any frontmatter. The skill and
  agent frontmatter shapes absorb new optional fields (`model`,
  `effort`, `context`, `agent` on Claude Code; `model`,
  `model_reasoning_effort` on Codex), all of which are documented
  optional fields in the respective host's existing schema. No
  wire-format change.
- No CI host-pack drift-check extension. The existing drift check
  already covers `resources/agents/` paths; the new subagent files
  land under the same tree and are picked up automatically.
</non-goals>

## User Stories

<user-stories>
- As a Speccy user running `/speccy-work` in a Claude Code session
  open at Opus 4.7 / xhigh (the right level for the contract-writing
  phases I just finished), I want the implementation work to drop
  to Sonnet 4.6 / medium automatically so I am not paying Opus
  rates and Opus latency on mechanical task execution that Sonnet
  dispatches in a fraction of the time.
- As a Speccy user iterating across `/speccy-work`, `/speccy-review`,
  `/speccy-ship` in a single chat session, I want each phase's
  intermediate tool output to stay out of my main session context
  so the session does not bloat with every worker's file reads,
  code edits, and command output as the loop progresses.
- As a Speccy user invoking `/speccy-review`, I want the
  orchestrator's JSON-parsing-and-dispatch turn to run at Haiku so
  the fan-out itself does not cost an Opus turn, while still
  preserving the orchestrator's ability to spawn the six reviewer
  subagents at their own pinned tiers.
- As the `reviewer-business` persona reviewing a task that the
  implementer claims is complete, I want my own subagent to run at
  Opus / xhigh so my semantic adversarial reasoning has the
  capacity to catch drift between SPEC intent and shipped diff
  rather than missing nuance at a lower tier.
- As the `reviewer-style` persona reviewing the same task, I want
  my subagent to run at Sonnet / medium so my pattern-matching
  load completes quickly and at lower cost than the
  semantic reviewers, without bottlenecking the fan-out.
- As a Codex user who has invoked `/agent speccy-work` to get the
  pinned-execution path, I want `.codex/agents/speccy-work.toml` to
  declare its model and reasoning_effort so my Codex session
  switches to the pinned tier when I activate the agent.
- As a Codex user reading `.agents/skills/speccy-work/SKILL.md`
  for the first time, I want a one-line pointer at the top of the
  body telling me to invoke `/agent speccy-work` first if I want
  the cost-and-time-win execution path, so I do not have to consult
  the README to discover that pattern.
- As a maintainer auditing the in-tree workspace, I want every
  Haiku pin to omit the `effort:` field so the file shape matches
  the current Haiku 4.5 API contract; a Haiku release that ships
  the effort enum becomes a future amendment, not a retroactive
  invalidation of the v1 shape.
- As a Speccy user who wants to dogfood the loop on Sonnet-only
  (no Opus access in my plan), I want every ejected agent file to
  be a plain text file I can edit to change the pin, so I can
  swap all six reviewers and all five phase workers to Sonnet
  variants by hand without fighting the CLI.
- As a Speccy user pinning a project to a specific model version
  for reproducibility, I want the ejected files to use aliases by
  default but accept full version IDs when I edit them, so I can
  swap `model: opus` for `model: claude-opus-4-7` in the files I
  care about while leaving the rest of the pack on aliases.
- As a Speccy user invoking `/speccy-brainstorm`, `/speccy-plan`,
  or `/speccy-amend`, I want my session model to be respected
  throughout the multi-turn dialogue, so the conversational
  phases run at whatever I picked when I opened the session
  rather than reverting partway through a brainstorm.
- As a Speccy user running `speccy init` in a fresh project, I
  want the same pin assignments Speccy uses on itself to be
  installed in my project automatically, so my first loop on a
  greenfield repo inherits the cost-and-time tuning rather than
  requiring me to re-derive it.
- As a Speccy user reading the project README to understand the
  current state of the repo, I want the README to match what
  actually ships (commands, skill pack contents, ejection paths,
  pin assignments, host asymmetry), so I do not need to grep the
  code to discover what the documentation has drifted past.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Four Claude Code mechanical phases fork to pinned subagents

`.claude/skills/speccy-tasks/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.claude/skills/speccy-ship/SKILL.md`, and
`.claude/skills/speccy-init/SKILL.md` each declare `context: fork`
and `agent: speccy-<phase>` in their YAML frontmatter, where
`<phase>` is the matching phase name. Four new subagent files
ship alongside them at `.claude/agents/speccy-tasks.md`,
`.claude/agents/speccy-work.md`, `.claude/agents/speccy-ship.md`,
and `.claude/agents/speccy-init.md`. The three Sonnet-tier files
(`speccy-tasks`, `speccy-work`, `speccy-ship`) declare
`model: sonnet` and `effort: medium` in their frontmatter. The
Haiku-tier file (`speccy-init`) declares `model: haiku` with no
`effort:` field.

<done-when>
- The four named SKILL.md files each contain `context: fork` and
  `agent: speccy-<matching-phase>` in their YAML frontmatter,
  appended to the existing `name:` / `description:` lines.
- The four new agent files exist at the four paths named above.
- Each new agent file's frontmatter contains `name:`,
  `description:`, and `model:` fields.
- The three Sonnet-tier agent files also contain `effort: medium`.
- The Haiku-tier agent file (`speccy-init.md`) does not contain
  an `effort:` line.
- Each new agent file's body includes the corresponding skill body
  via `{% include "modules/skills/speccy-<phase>.md" %}` in the
  templated source under `resources/agents/.claude/agents/speccy-<phase>.md.tmpl`,
  matching the existing reviewer-agent template pattern.
- The four SKILL.md bodies are unchanged below the frontmatter.
- Invoking `/speccy-work` on Claude Code in a session set to any
  model triggers a forked subagent that runs at Sonnet (the test
  is observational: the host emits the fork transition).
</done-when>

<behavior>
- Given the post-SPEC `.claude/skills/speccy-work/SKILL.md`, when
  its YAML frontmatter is parsed, then it contains
  `context: fork` and `agent: speccy-work` keys with those literal
  values.
- Given the post-SPEC `.claude/agents/speccy-work.md`, when its
  YAML frontmatter is parsed, then it contains `model: sonnet`
  and `effort: medium` keys with those literal values.
- Given the post-SPEC `.claude/agents/speccy-init.md`, when its
  YAML frontmatter is parsed, then it contains `model: haiku` and
  does not contain an `effort:` key.
- Given `resources/agents/.claude/agents/speccy-work.md.tmpl`,
  when read, then it includes
  `{% include "modules/skills/speccy-work.md" %}` so the rendered
  agent file inlines the shared skill body.
- Given the four SKILL.md files for the mechanical phases, when
  their bodies (below frontmatter) are diffed against the previous
  shipped version, then only frontmatter is changed.
</behavior>

<scenario id="CHK-001">
Given `.claude/skills/speccy-work/SKILL.md` after this SPEC's
tasks land, when its YAML frontmatter is parsed, then it contains
the keys `context` with value `fork` and `agent` with value
`speccy-work`.

Given the same file at `speccy-tasks/SKILL.md`,
`speccy-ship/SKILL.md`, and `speccy-init/SKILL.md` after this
SPEC's tasks land, when each one's YAML frontmatter is parsed,
then each contains `context: fork` and an `agent:` key pointing
to its own phase name.

Given the four new agent files at `.claude/agents/speccy-tasks.md`,
`.claude/agents/speccy-work.md`, `.claude/agents/speccy-ship.md`,
`.claude/agents/speccy-init.md`, when each one is read, then each
file exists.

Given each of the three Sonnet-tier agent files
(`speccy-tasks.md`, `speccy-work.md`, `speccy-ship.md`), when its
YAML frontmatter is parsed, then it contains `model: sonnet` and
`effort: medium`.

Given the Haiku-tier agent file (`speccy-init.md`), when its YAML
frontmatter is parsed, then it contains `model: haiku` and does
not contain a key named `effort`.

Given the templated source at
`resources/agents/.claude/agents/speccy-work.md.tmpl`, when read,
then it contains the literal string
`{% include "modules/skills/speccy-work.md" %}`. The same
relationship holds for the three other phase-worker templates,
each including its matching skill body module.

Given the SKILL.md body content (below frontmatter) for each of
the four mechanical-phase skills, when diffed against the previous
shipped version, then the body content is unchanged.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: speccy-review pins via direct skill frontmatter, no fork

`.claude/skills/speccy-review/SKILL.md` declares `model: haiku` in
its YAML frontmatter without `context: fork`, without an `agent:`
target, and without an `effort:` field. The skill body is
unchanged below the frontmatter. The reviewer fan-out mechanism
(Task-tool spawning of the six reviewer subagents from the
orchestrator's body) is preserved, so the orchestrator's Haiku
turn dispatches the reviewers, each of which runs at its own
pinned tier per REQ-003.

<done-when>
- `.claude/skills/speccy-review/SKILL.md` YAML frontmatter contains
  `model: haiku`.
- The same frontmatter does not contain `context:` or `agent:` keys.
- The same frontmatter does not contain an `effort:` key.
- The skill body (below frontmatter) is unchanged from the
  pre-SPEC version.
- No new file at `.claude/agents/speccy-review.md` is created on
  Claude Code.
</done-when>

<behavior>
- Given the post-SPEC `.claude/skills/speccy-review/SKILL.md`,
  when its YAML frontmatter is parsed, then it contains
  `model: haiku` and does not contain `context`, `agent`, or
  `effort` keys.
- Given the same file, when its body content (below frontmatter)
  is diffed against the previous shipped version, then it is
  unchanged.
- Given an invocation of `/speccy-review` on Claude Code in a
  session set to Opus, when the orchestrator turn executes, then
  the Task-tool calls that spawn the six reviewer subagents
  succeed (preserving fan-out behavior).
</behavior>

<scenario id="CHK-002">
Given `.claude/skills/speccy-review/SKILL.md` after this SPEC's
tasks land, when its YAML frontmatter is parsed, then it contains
the key `model` with value `haiku`.

Given the same file's YAML frontmatter, when scanned for the keys
`context`, `agent`, and `effort`, then none of those keys are
present.

Given the same file's body content below the frontmatter, when
diffed against the pre-SPEC version, then the body is unchanged.

Given the templated source at
`resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`,
when read, then its frontmatter mirrors the rendered file:
`model: haiku` present, `context` / `agent` / `effort` absent.

Given the post-SPEC `.claude/agents/` directory contents, when
listed, then there is no file named `speccy-review.md` (the
orchestrator does not become a subagent).
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Six Claude Code reviewer subagents pinned asymmetrically

The six existing reviewer agent files at
`.claude/agents/reviewer-business.md`,
`.claude/agents/reviewer-tests.md`,
`.claude/agents/reviewer-architecture.md`,
`.claude/agents/reviewer-security.md`,
`.claude/agents/reviewer-style.md`, and
`.claude/agents/reviewer-docs.md` gain `model:` and `effort:`
frontmatter fields. The assignment is asymmetric across personas:

- `reviewer-business`, `reviewer-tests`, `reviewer-architecture`:
  `model: opus`, `effort: xhigh`.
- `reviewer-security`: `model: sonnet`, `effort: high`.
- `reviewer-style`, `reviewer-docs`: `model: sonnet`,
  `effort: medium`.

The reviewer body content (below frontmatter) is unchanged in
every file.

<done-when>
- Each of the six reviewer agent files contains `model:` and
  `effort:` keys in its YAML frontmatter with the values named
  above.
- The body content (below frontmatter) of each reviewer agent
  file is unchanged from the pre-SPEC version.
- The matching templates under
  `resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`
  carry the same `model:` and `effort:` values so `speccy init`
  in a fresh project renders the same pin assignments.
</done-when>

<behavior>
- Given each of the six post-SPEC reviewer agent files, when its
  YAML frontmatter is parsed, then `model:` and `effort:` keys
  are present with values matching the assignment table.
- Given `reviewer-business.md`, when its frontmatter is parsed,
  then `model: opus` and `effort: xhigh` are present.
- Given `reviewer-security.md`, when its frontmatter is parsed,
  then `model: sonnet` and `effort: high` are present.
- Given `reviewer-style.md` and `reviewer-docs.md`, when each
  one's frontmatter is parsed, then `model: sonnet` and
  `effort: medium` are present.
- Given the body content (below frontmatter) of each reviewer
  file, when diffed against the pre-SPEC version, then the body
  is unchanged.
</behavior>

<scenario id="CHK-003">
Given `.claude/agents/reviewer-business.md`,
`.claude/agents/reviewer-tests.md`, and
`.claude/agents/reviewer-architecture.md` after this SPEC's tasks
land, when each one's YAML frontmatter is parsed, then each
contains `model: opus` and `effort: xhigh`.

Given `.claude/agents/reviewer-security.md` after this SPEC's
tasks land, when its YAML frontmatter is parsed, then it contains
`model: sonnet` and `effort: high`.

Given `.claude/agents/reviewer-style.md` and
`.claude/agents/reviewer-docs.md` after this SPEC's tasks land,
when each one's YAML frontmatter is parsed, then each contains
`model: sonnet` and `effort: medium`.

Given the body content (below frontmatter) of each of the six
reviewer agent files, when diffed against the pre-SPEC version,
then the body is byte-identical except for the frontmatter
additions.

Given the matching template files under
`resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`,
when each one is read, then its frontmatter carries the same
`model:` and `effort:` values as the rendered output above.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Codex parallel ships matching pins at the subagent layer

Five new Codex subagent files exist at
`.codex/agents/speccy-tasks.toml`,
`.codex/agents/speccy-work.toml`,
`.codex/agents/speccy-ship.toml`,
`.codex/agents/speccy-init.toml`, and
`.codex/agents/speccy-review.toml`. Each new file declares
`name`, `description`, `model`, and (where applicable)
`model_reasoning_effort` keys. The pin assignment matches the
Claude Code half: the three Sonnet-tier files declare
`model_reasoning_effort = "medium"`; the two Haiku-tier files
declare no `model_reasoning_effort` key. Each new file's
`developer_instructions` field includes the shared phase body
via `{% include "modules/skills/speccy-<phase>.md" %}` in the
template source.

The six existing Codex reviewer files at
`.codex/agents/reviewer-<persona>.toml` gain `model` and
`model_reasoning_effort` keys with the same assignment as their
Claude Code counterparts per REQ-003.

The five Codex phase-worker skill bodies at
`.agents/skills/speccy-<phase>/SKILL.md` each gain a one-line
pointer at the top of the body (after the frontmatter, before the
existing skill content) naming the corresponding
`.codex/agents/speccy-<phase>.toml` invocation path with the
literal text "for the cost-and-time-win execution path, invoke
this skill via `/agent speccy-<phase>` first" (or
substantially equivalent prose).

<done-when>
- Five new files exist at the five `.codex/agents/speccy-*.toml`
  paths.
- Each new file contains `name`, `description`, and `model` keys.
- The three Sonnet-tier files (`speccy-tasks`, `speccy-work`,
  `speccy-ship`) also contain `model_reasoning_effort = "medium"`.
- The two Haiku-tier files (`speccy-init`, `speccy-review`) do
  not contain a `model_reasoning_effort` key.
- Each new file's `developer_instructions` field renders the
  shared phase body via templated include.
- The six existing reviewer TOML files contain `model` and
  `model_reasoning_effort` keys matching the REQ-003 assignment.
- The five Codex phase-worker SKILL.md bodies each contain a
  one-line pointer at the top of the body naming the
  corresponding TOML file and the `/agent <name>` invocation.
</done-when>

<behavior>
- Given each new file at `.codex/agents/speccy-<phase>.toml`,
  when parsed, then `name`, `description`, and `model` keys are
  present.
- Given `.codex/agents/speccy-work.toml`, when parsed, then
  `model = "sonnet"` (or the Codex provider equivalent alias)
  and `model_reasoning_effort = "medium"` are present.
- Given `.codex/agents/speccy-init.toml` and
  `.codex/agents/speccy-review.toml`, when parsed, then `model`
  is set to the Haiku alias and `model_reasoning_effort` is
  absent.
- Given each Codex reviewer TOML file, when parsed, then its
  `model` and `model_reasoning_effort` values match its Claude
  Code counterpart per REQ-003.
- Given each Codex phase-worker SKILL.md file, when its body is
  read, then the first non-frontmatter line names the
  corresponding TOML file path and the `/agent` invocation
  pattern.
</behavior>

<scenario id="CHK-004">
Given `.codex/agents/speccy-tasks.toml`,
`.codex/agents/speccy-work.toml`,
`.codex/agents/speccy-ship.toml`,
`.codex/agents/speccy-init.toml`, and
`.codex/agents/speccy-review.toml` after this SPEC's tasks land,
when each is read, then each file exists and parses as TOML.

Given each of the three Sonnet-tier Codex phase-worker files
(`speccy-tasks.toml`, `speccy-work.toml`, `speccy-ship.toml`),
when its TOML is parsed, then it contains `model_reasoning_effort`
with value `"medium"`.

Given the two Haiku-tier Codex phase-worker files
(`speccy-init.toml`, `speccy-review.toml`), when each is parsed,
then `model_reasoning_effort` is not present.

Given each Codex reviewer TOML file
(`.codex/agents/reviewer-<persona>.toml`), when parsed, then
`model` and `model_reasoning_effort` are present with values
matching the REQ-003 assignment for the corresponding persona.

Given each Codex phase-worker skill body
(`.agents/skills/speccy-<phase>/SKILL.md`), when its first
non-frontmatter line is read, then the line names the
corresponding `.codex/agents/speccy-<phase>.toml` file and the
`/agent speccy-<phase>` invocation pattern.

Given the matching template files under
`resources/agents/.codex/agents/speccy-<phase>.toml.tmpl` and
`resources/agents/.agents/skills/speccy-<phase>/SKILL.md.tmpl`,
when each is read, then the templated source produces a rendered
file matching the requirements above when `speccy init`
processes it.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Pin shape respects model family capabilities

Every `model:` (Claude Code) and `model` (Codex) value uses an
alias rather than a versioned model ID: `opus`, `sonnet`, or
`haiku` on Claude Code; the matching alias on Codex. No
versioned IDs (e.g. `claude-opus-4-7`, `claude-sonnet-4-6`,
`claude-haiku-4-5`) appear in any shipped frontmatter, skill body,
or agent body in `resources/agents/` or in the in-tree dogfood
pack under `.claude/` and `.codex/`.

Every Opus pin may include an `effort:` field with values up to
`xhigh` or `max`. Every Sonnet pin may include an `effort:` field
with values up to `max` (but never `xhigh`, which is Opus-only).
Every Haiku pin (or Codex equivalent) declares `model:` alone
without an `effort:` or `model_reasoning_effort:` field, since
Haiku 4.5 uses budget-based extended thinking rather than the new
effort enum.

<done-when>
- No file under `resources/agents/.claude/`,
  `resources/agents/.codex/`, `resources/agents/.agents/`, or the
  in-tree dogfood pack at `.claude/` and `.codex/` contains a
  versioned Anthropic model ID in `model:` or `model` frontmatter
  fields.
- Every Haiku-pinned file omits `effort:` (Claude Code) or
  `model_reasoning_effort` (Codex).
- Every Sonnet-pinned file's `effort:` (or Codex equivalent) value
  is one of `low`, `medium`, `high`, `max` — never `xhigh`.
- Every Opus-pinned file's `effort:` (or Codex equivalent) value
  is one of `low`, `medium`, `high`, `xhigh`, `max`.
</done-when>

<behavior>
- Given every shipped agent file under `.claude/agents/` and
  `.codex/agents/`, when scanned for the literal substrings
  `claude-opus-`, `claude-sonnet-`, or `claude-haiku-`, then zero
  matches are found.
- Given every Haiku-pinned file, when its frontmatter is parsed,
  then no key named `effort` or `model_reasoning_effort` is
  present.
- Given every Sonnet-pinned file, when its `effort` /
  `model_reasoning_effort` value is read, then the value is not
  `xhigh`.
- Given every Opus-pinned file, when its `effort` /
  `model_reasoning_effort` value is read, then the value is one of
  the documented Opus 4.7 levels (`low`, `medium`, `high`,
  `xhigh`, `max`).
</behavior>

<scenario id="CHK-005">
Given every file under `resources/agents/` and the in-tree dogfood
pack at `.claude/` and `.codex/`, when grepped for the literal
substrings `claude-opus-`, `claude-sonnet-`, or `claude-haiku-`,
then zero matches are found in shipped frontmatter or body
content.

Given each Haiku-pinned file (the agent files for `speccy-init`
on Claude Code, plus the equivalent on Codex, plus the Codex
TOML for `speccy-review`), when each one's frontmatter (or TOML
top-level keys) is parsed, then no key named `effort` (on Claude
Code) or `model_reasoning_effort` (on Codex) is present.

Given each Sonnet-pinned file (the three Claude Code phase-worker
agents for `tasks`, `work`, `ship`; the three Codex equivalents;
the Sonnet-tier reviewer agents on each host), when each one's
`effort` or `model_reasoning_effort` value is read, then the
value is one of `low`, `medium`, `high`, or `max` — never
`xhigh`.

Given each Opus-pinned file (the three semantic reviewer agents
on each host: `reviewer-business`, `reviewer-tests`,
`reviewer-architecture`), when each one's `effort` or
`model_reasoning_effort` value is read, then the value is one of
`low`, `medium`, `high`, `xhigh`, or `max`.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: speccy init installs pin assignments into fresh projects

Running `speccy init` in a fresh project directory renders the
templated files under `resources/agents/` into the project's
host-native locations with the same pin assignments Speccy uses
in its own in-tree dogfood pack. The rendered output covers both
host packs (Claude Code and Codex) regardless of which host was
selected, since `speccy init` always renders all host packs.

<done-when>
- A fresh `speccy init` invocation in an empty directory creates
  the four new Claude Code phase-worker agent files under
  `.claude/agents/speccy-<phase>.md` with the same model and
  effort frontmatter as the in-tree workspace.
- The same invocation creates the five new Codex phase-worker
  TOML files under `.codex/agents/speccy-<phase>.toml` with the
  same model and reasoning_effort values.
- The same invocation updates the six existing reviewer files on
  each host with the asymmetric pin assignment per REQ-003.
- The fresh `.claude/skills/speccy-<phase>/SKILL.md` files for the
  four mechanical-phase workers contain `context: fork` and
  `agent:` frontmatter.
- The fresh `.claude/skills/speccy-review/SKILL.md` contains
  `model: haiku` frontmatter without `context:` or `agent:` keys.
- The fresh Codex `.agents/skills/speccy-<phase>/SKILL.md` files
  contain the one-line invocation pointer at the top of the body.
</done-when>

<behavior>
- Given a fresh empty directory, when `speccy init` runs, then
  the rendered host packs contain the same pin assignments as
  the in-tree dogfood workspace.
- Given the existing CI host-pack drift check, when it runs after
  this SPEC's tasks land, then the drift check passes because the
  templated `resources/agents/` source and the in-tree rendered
  `.claude/` and `.codex/` outputs remain in sync.
- Given a user running `speccy init --force` in an existing
  project, when the command completes, then files classified
  Skip-on-exists (per SPEC-0027) are not overwritten and files
  classified Render-always are updated with the new pin
  assignments.
</behavior>

<scenario id="CHK-006">
Given a fresh empty directory after `speccy init` runs (in a
test harness that exercises both Claude Code and Codex host
packs), when the rendered file tree is inspected, then it contains
the four new Claude Code phase-worker agent files under
`.claude/agents/speccy-<phase>.md` (for `tasks`, `work`, `ship`,
`init`) and the five new Codex phase-worker TOML files under
`.codex/agents/speccy-<phase>.toml` (for `tasks`, `work`, `ship`,
`init`, `review`).

Given the same rendered tree, when each new agent file's
frontmatter (or TOML keys) is parsed, then the pin values match
the in-tree dogfood pack: Sonnet/medium for `tasks`/`work`/`ship`,
Haiku (model-only) for `init` (and `review` on Codex).

Given the same rendered tree, when the six reviewer files on
each host are inspected, then their frontmatter (or TOML keys)
contain the asymmetric pin assignment per REQ-003.

Given the four mechanical-phase SKILL.md files on Claude Code
in the rendered tree, when each is parsed, then each contains
`context: fork` and an `agent:` key naming the matching agent.

Given the `speccy-review` SKILL.md on Claude Code in the rendered
tree, when parsed, then it contains `model: haiku` without
`context:` or `agent:` keys.

Given the existing CI host-pack drift-check meta-test, when run
against the post-SPEC workspace, then it exits 0 (the templated
source and in-tree rendered outputs match byte-for-byte).
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: README adds model-pinning section and refreshes outdated content

The project `README.md` at the repository root gains a new
top-level section (or subsection under an existing top-level
section, depending on the README's current shape) titled "Model
pinning and host asymmetry" or equivalent. The new section names:

- The full pin assignment table covering all five mechanical
  phases and all six reviewer personas.
- The Claude Code automatic-fork mechanism (skill `context: fork`
  spawns a pinned subagent) versus the Codex `/agent <name>`
  opt-in mechanism.
- The user override path: edit the ejected frontmatter file to
  swap the model alias or remove a pin entirely.
- The reason aliases are used rather than versioned IDs, with a
  note that the user can lock to a specific version by editing.

Separately, the rest of the README is audited for drift against
the current state of the repository. Outdated content (command
names that have moved, shipped skill pack contents that have
changed, ejection paths that have moved per SPEC-0027 retirement
of `.speccy/skills/`, prose referencing retired XML elements per
SPEC-0021, etc.) is corrected. The audit is one-time and not
codified into a meta-test.

<done-when>
- `README.md` contains a new section (titled "Model pinning and
  host asymmetry" or substantially equivalent) that names the pin
  assignment, the host asymmetry, the override path, and the
  alias rationale.
- The new section names every Claude Code and Codex agent file
  that carries a pin (the four phase-worker agent files on Claude
  Code; the five phase-worker TOML files on Codex; the six
  reviewer files on each host).
- The rest of the README has been read end-to-end and any prose
  contradicting the current repository state has been corrected.
- The README's command reference (if present) names the current
  ten Speccy CLI commands (`init`, `plan`, `tasks`, `implement`,
  `review`, `report`, `status`, `next`, `check`, `verify`) and
  no others.
- The README's ejection-path reference (if present) names
  `.claude/`, `.codex/`, and `.agents/` paths and does not
  reference the retired `.speccy/skills/` directory (per
  SPEC-0027).
</done-when>

<behavior>
- Given the post-SPEC `README.md`, when read end-to-end, then
  every CLI command name mentioned matches one of the ten
  currently shipped commands.
- Given the post-SPEC `README.md`, when scanned for the literal
  substring `.speccy/skills/`, then zero matches are found in
  prose describing user-facing paths (matches inside a "historical
  context" or migration-note block are out of scope and the
  reviewer-docs persona judges relevance).
- Given the post-SPEC `README.md`, when its new model-pinning
  section is parsed, then it contains the full pin assignment
  table covering both hosts and both phase workers and reviewers.
</behavior>

<scenario id="CHK-007">
Given `README.md` at the repository root after this SPEC's tasks
land, when scanned for the literal substring "Model pinning" (or
the chosen section title), then at least one match exists at the
start of a top-level or subsection heading.

Given the same file's new pinning section, when read, then it
names the pin tier (Opus / Sonnet / Haiku) and effort level (or
absence thereof) for every mechanical phase
(`speccy-tasks`, `speccy-work`, `speccy-ship`, `speccy-init`,
`speccy-review`) and every reviewer persona
(`reviewer-business`, `reviewer-tests`, `reviewer-architecture`,
`reviewer-security`, `reviewer-style`, `reviewer-docs`).

Given the same section, when read, then it describes both the
Claude Code automatic-fork mechanism (skills with `context: fork`
spawn pinned subagents) and the Codex `/agent <name>` opt-in
invocation path, including the reason for the asymmetry.

Given the same section, when read, then it describes the user
override path (edit the ejected agent file's frontmatter) and
notes that pins use aliases by default so users can lock to a
specific model version by editing.

Given the rest of the post-SPEC `README.md`, when grepped for
mentions of the retired `.speccy/skills/` directory (per
SPEC-0027), then no prose presents it as a current user-facing
path; references at most appear in historical-context blocks.

Given the rest of the post-SPEC `README.md`, when grepped for
CLI command names, then every mentioned name is one of the ten
currently shipped commands (`init`, `plan`, `tasks`,
`implement`, `review`, `report`, `status`, `next`, `check`,
`verify`).
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Phase-worker subagent bodies share a single source via templating

The four new Claude Code phase-worker agent files at
`.claude/agents/speccy-<phase>.md` and the five new Codex
phase-worker TOML files at `.codex/agents/speccy-<phase>.toml`
each include their body content from the shared source under
`resources/modules/skills/speccy-<phase>.md` via MiniJinja
`{% include %}`, mirroring the pattern the six reviewer agents
already use through `resources/modules/personas/`.

No new directory `resources/modules/agents/` is created. The
phase-worker subagent body and the phase-worker skill body share
a single file in `resources/modules/skills/` so an edit to the
phase prompt body propagates to both the skill wrapper and the
subagent wrapper without duplication.

<done-when>
- Each template under
  `resources/agents/.claude/agents/speccy-<phase>.md.tmpl`
  contains
  `{% include "modules/skills/speccy-<phase>.md" %}`.
- Each template under
  `resources/agents/.codex/agents/speccy-<phase>.toml.tmpl`
  includes the same shared body file in its
  `developer_instructions` value.
- The directory `resources/modules/agents/` does not exist.
- The shared body files at `resources/modules/skills/speccy-<phase>.md`
  exist for each of the five phase workers (these already exist
  pre-SPEC; this REQ asserts they remain the single source and
  are not duplicated).
</done-when>

<behavior>
- Given each new template under
  `resources/agents/.claude/agents/speccy-<phase>.md.tmpl`, when
  read, then it contains a MiniJinja include directive naming
  `modules/skills/speccy-<phase>.md`.
- Given each new template under
  `resources/agents/.codex/agents/speccy-<phase>.toml.tmpl`, when
  read, then its `developer_instructions` value includes the same
  shared body via MiniJinja include.
- Given the `resources/modules/` directory, when listed, then it
  does not contain an `agents/` subdirectory.
</behavior>

<scenario id="CHK-008">
Given each template file under
`resources/agents/.claude/agents/speccy-tasks.md.tmpl`,
`speccy-work.md.tmpl`, `speccy-ship.md.tmpl`, and
`speccy-init.md.tmpl`, when read, then each contains the literal
string `{% include "modules/skills/speccy-` followed by the
phase name and `.md" %}`.

Given each template file under
`resources/agents/.codex/agents/speccy-tasks.toml.tmpl`,
`speccy-work.toml.tmpl`, `speccy-ship.toml.tmpl`,
`speccy-init.toml.tmpl`, and `speccy-review.toml.tmpl`, when
read, then each contains a MiniJinja include directive naming
the matching `modules/skills/speccy-<phase>.md` body inside the
`developer_instructions` value.

Given the directory `resources/modules/`, when listed, then it
contains `personas/`, `prompts/`, `skills/`, and (per SPEC-0031)
`examples/` subdirectories but does not contain an `agents/`
subdirectory.

Given each shared body file at
`resources/modules/skills/speccy-<phase>.md` for the five
phase workers, when read, then it exists and is the same source
both the skill wrapper and the new subagent wrapper include.
</scenario>

</requirement>

## Design

### Approach

The change concentrates in two layers: frontmatter edits on
existing shipped files (16 files total: 5 Claude Code SKILL.md
frontmatter edits, 6 Claude Code reviewer agent frontmatter edits,
5 Codex skill body 1-line additions; plus 6 existing Codex
reviewer TOML frontmatter edits — net new file count is 9 across
both hosts) and new files (4 Claude Code phase-worker agent files,
5 Codex phase-worker TOML files). The templating layer under
`resources/agents/` mirrors all of this so `speccy init` renders
the same shape in user projects. No code paths inside `speccy-cli`
or `speccy-core` change.

Implementation order:

1. Land the four new Claude Code phase-worker agent files and the
   four SKILL.md frontmatter additions for `tasks`, `work`,
   `ship`, `init`. Templates and rendered outputs both. (REQ-001,
   REQ-008.)
2. Land the `.claude/skills/speccy-review/SKILL.md` frontmatter
   edit (direct model pin, no fork). (REQ-002.)
3. Land the six Claude Code reviewer frontmatter edits. (REQ-003.)
4. Land the five new Codex phase-worker TOML files, the six
   reviewer TOML frontmatter edits, and the five Codex
   phase-worker skill-body pointer additions. (REQ-004, REQ-008.)
5. Validate pin shape across all files (no versioned IDs; Haiku
   has no effort; Sonnet has no xhigh). (REQ-005.)
6. Verify `speccy init` renders the new files correctly in a
   fresh-directory test, and confirm the existing CI host-pack
   drift check passes. (REQ-006.)
7. Write the new README section and audit the rest of the README
   for current-repo-state drift. (REQ-007.)

The hygiene gates (`cargo test`, `cargo clippy`, `cargo +nightly
fmt --all --check`, `cargo deny check`) run at the end of every
task to catch incidental breakage from template renders.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Use context: fork for mechanical-phase pinning rather than direct skill frontmatter pin

**Context:** Claude Code skills support both a direct
`model:` / `effort:` pin (applies for the current assistant turn
only) and a `context: fork` + `agent:` pin (applies across the
forked subagent's full execution, multiple internal turns, in an
isolated context). Both could deliver the cost-and-time win on a
single phase. The choice affects whether the main session's
context accumulates the worker's intermediate tool output across
a multi-phase loop.

**Decision:** Mechanical phases (`tasks`, `work`, `ship`, `init`)
use `context: fork` with a dedicated subagent definition. The
direct skill-frontmatter pin is reserved for `speccy-review` only
(see DEC-002).

**Alternatives:**

- Direct `model:` + `effort:` on every mechanical-phase SKILL.md
  with no fork (rejected: single-turn pin scope means the override
  reverts after the first response; multi-phase loops bloat the
  main session context with every worker's intermediate output,
  defeating the cost-and-time benefit; future autonomous
  orchestrator inherits a polluted parent context every invocation).
- Skill body dispatches to a subagent via explicit Task-tool
  invocation in the body prose (rejected: requires writing
  dispatch logic into every skill body; `context: fork` is the
  built-in primitive for exactly this case and keeps skill bodies
  focused on the work rather than the dispatch mechanism).
- Document-as-recommendation only with no enforcement (rejected:
  cost-and-time win would require the user to remember to switch
  models before every invocation, violating the
  friction-to-skill-update principle from AGENTS.md).

**Consequences:** Each pinned phase needs two files (the skill
wrapper and the subagent definition) on Claude Code, plus the
equivalent agent TOML on Codex. The reviewer fan-out pattern
(skill spawns subagents) already proves this shape works. The
future autonomous orchestrator gains a clean dispatch path: it
spawns worker subagents directly via Task tool, bypassing the
per-phase skill, with no main-session bloat.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: speccy-review pins via direct skill frontmatter, not context: fork

**Context:** `/speccy-review` is the orchestrator that fans out
to the six reviewer subagents via the Task tool. Claude Code's
documented rule is that subagents cannot spawn other subagents
(per the Claude Code subagent docs). If `/speccy-review` were
itself forked into a subagent per DEC-001, that subagent would
lose Task-tool access to spawn the reviewers, breaking the
fan-out.

**Decision:** `/speccy-review` pins via direct `model: haiku` on
the existing SKILL.md frontmatter, with no `context: fork` and
no `effort:` field. The orchestrator runs in the main session at
Haiku for one assistant turn (the dispatcher turn), spawns the
six reviewer subagents, and exits. Each spawned reviewer subagent
carries its own model and effort pin per REQ-003, independent of
which model spawned it.

**Alternatives:**

- Fork `/speccy-review` to a `speccy-review` subagent that owns
  the fan-out (rejected: subagents cannot spawn subagents on
  Claude Code; the fan-out would break).
- Move the fan-out logic into the main session driver and let the
  user manually invoke each reviewer (rejected: regresses
  ergonomics; today a single `/speccy-review` runs all four
  reviewers in parallel, and unwinding that into a manual
  per-persona invocation would be a significant UX downgrade).
- Keep `/speccy-review` fully unpinned (rejected: the orchestrator
  turn's JSON parsing and dispatch is exactly the kind of
  mechanical work that should not be running at Opus rates).

**Consequences:** `/speccy-review`'s single dispatcher turn is
pinned, but the main session inherits whatever context the
orchestrator generates during that turn (Haiku-grade dispatch
output is small). The pin scope is asymmetric to the other
mechanical phases — single-turn for `review`, sustained for the
other four — and that asymmetry is justified by the fan-out
constraint, not generalizable to other phases.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Use Anthropic model aliases rather than versioned model IDs

**Context:** Claude Code accepts both aliases (`opus`, `sonnet`,
`haiku`) and full versioned model IDs (`claude-opus-4-7`,
`claude-sonnet-4-6`, `claude-haiku-4-5`) in `model:` frontmatter.
Aliases float forward to whichever version Anthropic ships as the
current generation; versioned IDs lock the user's loop to a
specific model for reproducibility.

**Decision:** All shipped frontmatter uses aliases.

**Alternatives:**

- Use versioned IDs everywhere (rejected: every Anthropic model
  ship cycle would require a Speccy amendment to bump every pin,
  trading drift against reproducibility in a way that favors
  invariance over forward motion).
- Use versioned IDs for the reviewer pins (where adversarial
  judgment is load-bearing) and aliases for mechanical phases
  (where work is forgiving) (rejected: introduces a split policy
  that future amendments must reason about; the simpler invariant
  is "always aliases, always editable").

**Consequences:** Speccy's pins float forward as Anthropic
generations evolve, which means a user dogfooding on a stable
project may see behavior changes when Anthropic ships a new
generation. The mitigation is editability: ejected files are
plain text and a user who wants reproducibility can swap aliases
for versioned IDs file-by-file post-`speccy init`.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: Skip measurement; rely on dogfooding for re-pin signals

**Context:** The pin assignment table in this SPEC reflects
informed judgment about which model and effort fits each phase,
but the values are not measured against the alternative. A
benchmarking step could capture token-and-latency deltas on a
representative SPEC under the old session-inherited assignments
versus the new pins, recording the numbers in REPORT.md as
evidence.

**Decision:** The SPEC does not include a measurement task.
Dogfooding governs re-pin signals; if an individual phase feels
under- or over-tier-ed during real loops, a future amendment
re-pins it.

**Alternatives:**

- Include a one-shot measurement step in REPORT.md (rejected:
  adds scope and ceremony without clear v1 value; one data point
  is hard to draw conclusions from without controlling for
  workload variance; the assignment values are defensible on
  their own merits).
- Include measurement plus a documented threshold for re-pin
  amendments in AGENTS.md (rejected: closer to a measurement loop
  than F-5 originally proposed; expands scope beyond what's
  needed for the cost-and-time win to materialize).

**Consequences:** The pin values are believed-correct but not
measured-correct. Future amendments may shift individual
assignments based on dogfood feel; that's the intended evolution
path. The lack of measurement infrastructure means Speccy stays
small.
</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: Conversational skills (brainstorm, plan, amend) stay unpinned

**Context:** The backlog proposed pinning `speccy-plan` /
`speccy-amend` at Opus/max and `speccy-brainstorm` at Opus/xhigh
for "insulation against future default drift." Claude Code's
skill `model:` override applies for the current assistant turn
only; subsequent user replies revert to session model. The
brainstorm Socratic gate, the plan framing dialogue, and the
amendment intent-shift conversation are all multi-turn — a pin
would only steer the opening response and revert immediately.

**Decision:** `speccy-brainstorm`, `speccy-plan`, and
`speccy-amend` SKILL.md files do not gain `model:`, `effort:`,
`context:`, or `agent:` frontmatter. They continue to run at
the parent session's model throughout their multi-turn
dialogues. The assumption is that users start a session at the
latest Opus generation at xhigh or max effort, which is the
contract-writing tier these phases need.

**Alternatives:**

- Add `model:` only to the conversational skills, accepting
  first-turn-only pin scope (rejected: partial pinning is
  confusing; users would not understand why opening responses
  feel different from follow-ups).
- Fork the conversational skills (rejected: breaks the
  multi-turn dialogue mechanic since subagents return one
  message and exit; the brainstorm Socratic gate cannot survive
  a fork).
- Add documentation in each conversational SKILL.md body naming
  the recommended session-model setting (deferred: covered by
  the README's new pinning section per REQ-007; no per-skill
  prose addition needed).

**Consequences:** Users who open their Speccy session at a lower
tier than recommended will experience under-powered framing
dialogue. The cost is documentation discipline: the README's
pinning section names the recommended session model for the
conversational phases, and the SPEC accepts that some users
will run sub-optimally until they internalize the recommendation.
</decision>

<decision id="DEC-006" status="accepted">
#### DEC-006: Codex parity is documented asymmetry, not engineered around

**Context:** Codex CLI exposes per-subagent `model` and
`model_reasoning_effort` in TOML files, but Codex skills do not
accept a model field, and Codex does not expose a
Claude-Code-equivalent `context: fork` primitive. Subagent
spawning on Codex is user-initiated ("Codex only spawns
subagents when you explicitly ask it to") rather than
programmatic.

**Decision:** The Codex cost-and-time win is opt-in: the user
invokes `/agent speccy-<phase>` to activate the pinned agent,
then issues the phase command. Speccy ships TOML subagent files
with the pin assignment, ships a one-line pointer in each
phase-worker skill body so users discover the invocation path,
and documents the asymmetry under the new README pinning
section. No workaround code, no programmatic dispatch, no
shimming.

**Alternatives:**

- Drop automatic dispatch on Claude Code too, so both hosts have
  the same opt-in shape (rejected: surrenders the Claude Code
  win for cross-host symmetry that no user has asked for and
  that yields a worse default on the host that already supports
  better).
- File a backlog entry tracking Codex CLI's eventual ship of a
  programmatic dispatch primitive (rejected by the user during
  brainstorming: documenting the asymmetry in the README is
  sufficient; a backlog entry would be a TODO without an action
  Speccy can take).
- Ship a Speccy-side workaround that wraps Codex's natural-language
  agent invocation in a deterministic interface (rejected: out
  of scope and would couple Speccy to Codex internals).

**Consequences:** Codex users get the pin assignment table but
must remember to invoke `/agent` before running a phase. The
one-line pointer in each Codex phase-worker skill body is the
discovery mechanism for users who haven't read the README. If
Codex CLI later ships a programmatic dispatch primitive, a
future amendment can absorb it, but Speccy does not block on
upstream changes.
</decision>

<decision id="DEC-007" status="accepted">
#### DEC-007: Single-source phase body via resources/modules/skills/

**Context:** The four new Claude Code phase-worker subagent
files and the five new Codex phase-worker TOML files each need
a body to drive the work. Three placement options exist for
the source: duplicate the body verbatim in each new file; create
a new `resources/modules/agents/` directory parallel to
`resources/modules/personas/`; or share the existing
`resources/modules/skills/` body via templated include.

**Decision:** Phase-worker subagent bodies share
`resources/modules/skills/speccy-<phase>.md` with the
corresponding skill wrapper via MiniJinja `{% include %}`. No
new `resources/modules/agents/` directory is created.

**Alternatives:**

- New `resources/modules/agents/` directory for phase workers
  (rejected: introduces a parallel-directory pattern that future
  contributors must reason about; the body content for a phase
  is the same whether wrapped as a skill or a subagent, so a
  shared file is the simpler invariant).
- Duplicate the body verbatim in each new file (rejected:
  doubles maintenance cost for every future phase prompt edit;
  introduces drift risk between skill wrapper and subagent
  wrapper).

**Consequences:** Edits to phase-worker prompts propagate to
both the skill wrapper and the subagent wrapper from a single
source file. The reviewer pattern through
`resources/modules/personas/` proves this works; the extension
to phase workers is a straightforward repeat.
</decision>

### Interfaces

No new CLI surface. No new `speccy <verb>` command, no new flag,
no new schema-versioned wire format. The change is entirely in
the templated host packs under `resources/agents/` and the
rendered in-tree dogfood pack under `.claude/` and `.codex/`,
plus README prose.

The two new frontmatter shapes are documented optional fields in
their respective host CLIs:

- **Claude Code:** `model:`, `effort:`, `context:`, `agent:` (all
  documented at https://code.claude.com/docs/en/skills and
  https://code.claude.com/docs/en/sub-agents).
- **Codex:** `model`, `model_reasoning_effort` (documented at
  https://developers.openai.com/codex/subagents and
  https://developers.openai.com/codex/config-advanced).

### Data changes

None. No CLI-readable file shape changes. The frontmatter
additions are optional keys that the host's own parser already
handles; Speccy's own parsers do not read any of these fields.

### Migration / rollback

**Forward:** The SPEC's tasks land in sequence per the
implementation order above. Each phase-worker subagent file lands
atomically with the corresponding SKILL.md frontmatter edit so
the rendered host pack is never in a half-pinned state.

**Rollback:** Revert the commits. No data state is affected; the
worst case is that an already-ejected user pack carries pinned
files that the user must manually edit out (the same shape as any
other amendment to a user-ejected file). No automatic rollback
sync.

## Assumptions

<assumptions>
- Claude Code's `context: fork` + `agent:` semantics deliver what
  the docs promise: the forked subagent uses the agent's
  declared model and effort across its full internal execution,
  the skill body becomes the subagent's task prompt, and only
  the subagent's final return message reaches the parent session.
- The pin assignment table is good-enough for v1. Adjustments
  may be needed after dogfooding, but the SPEC ships locked
  assignments rather than a measurement-driven re-pin step
  (per DEC-004).
- Codex's `/agent <name>` opt-in invocation is acceptable UX for
  v1; the asymmetry with Claude Code's automatic fork is
  documented in the README rather than engineered around (per
  DEC-006).
- Conversational skills (`brainstorm`, `plan`, `amend`) need no
  pinning for v1; the user opens a session at the latest Opus
  generation at xhigh or max effort, which is the
  contract-writing tier these phases need (per DEC-005).
- The existing CI host-pack drift check covers
  `resources/agents/` paths and will pick up the new phase-worker
  subagent files automatically; no extension to the meta-test is
  needed.
- Phase-worker subagent bodies share the existing
  `resources/modules/skills/` source via templating; no new
  `resources/modules/agents/` directory is needed (per DEC-007).
- `/speccy-review`'s main-session execution with a direct model
  pin (the option-B pattern reserved for this skill alone)
  preserves the existing reviewer fan-out via Task tool. The
  Haiku pin on the orchestrator turn is sufficient for
  JSON-parsing-and-dispatch, since reviewers themselves carry
  their own model pins independent of who spawned them (per
  DEC-002).
- Anthropic's model alias resolution (`opus` → current Opus
  generation, `sonnet` → current Sonnet generation, `haiku` →
  current Haiku generation) is stable and Anthropic does not
  silently swap aliases to point at downgraded models. If that
  assumption is violated, future amendments can swap aliases
  for versioned IDs file-by-file.
- Haiku 4.5's "no effort parameter" property is documented in
  the Anthropic API docs; Claude Code's parser does not reject
  a Haiku frontmatter that omits the effort key, and an
  ejected Haiku-pinned agent file runs at default thinking
  (extended thinking off unless requested via budget tokens)
  without error.
</assumptions>

## Open questions

(All open questions from the brainstorm phase were resolved
before this SPEC was drafted. Future open questions discovered
during implementation will be appended here with the `- [ ]`
checkbox format.)

## Changelog

<changelog>

| Date       | Author          | Summary |
|------------|-----------------|---------|
| 2026-05-19 | agent/claude-1  | Initial draft from F-5 brainstorm; context: fork pattern for mechanical phases, direct pin for /speccy-review, asymmetric reviewer pins, Codex parity at subagent layer, alias-based model values, README refresh in scope. |

</changelog>

## Notes

Rejected alternative framings considered during the brainstorm
phase:

- **Document-as-recommendation only:** Each SKILL.md body
  carries a "recommended model+effort" note with no enforcement;
  user runs `/model` manually. Rejected because the cost-and-time
  win requires the user to remember every time, violating the
  friction-to-skill-update principle from AGENTS.md and leaving
  the primary motivator unaddressed.

- **Direct `model:` + `effort:` on every SKILL.md without
  `context: fork`:** Simplest possible edit (one file per phase).
  Pin applies for one assistant turn only; main session fills up
  with worker output. Rejected as the dominant pattern (kept for
  `/speccy-review` only) because cumulative main-session
  pollution across a multi-phase loop defeats the cost-and-time
  benefit and the future autonomous orchestrator inherits a
  bloated context every invocation. (See DEC-001.)

- **Skill body dispatches to subagent via explicit Task-tool
  call in body prose:** Achieves the same isolation as
  `context: fork` but requires every skill body to script the
  dispatch. Rejected because `context: fork` is the built-in
  primitive for exactly this case; using it keeps skill bodies
  focused on the work rather than the dispatch mechanism. (See
  DEC-001.)

- **Drop per-phase skills entirely; expose only subagent
  surfaces:** Mechanical phases would be invoked via
  `/agent <name>` on both hosts; a single `/speccy-next`
  orchestrator skill would replace per-phase entry points.
  Rejected because it breaks existing user habits
  (`/speccy-work` is the canonical invocation pattern) and
  Speccy is supposed to work identically in or out of an
  autonomous orchestrator, not require one.

The cross-cutting observation from the BACKLOG.md F-5 entry
holds: the leverage lives in making rendered prompts stronger
and the host-native skill packs sharper, not in growing CLI
surface. This SPEC ships zero new CLI commands and zero new
flags; the entire change is in skill packs and README prose.

The verified facts from the brainstorm phase that load-bearing
decisions depend on:

- Claude Code subagents support `model:` (alias or full ID) and
  `effort:` frontmatter; `context: fork` + `agent:` makes a
  skill execute in a forked subagent. (Claude Code docs.)
- Codex subagents (`.codex/agents/*.toml`) support `model` and
  `model_reasoning_effort` in TOML; Codex skills do not.
  (OpenAI Codex docs.)
- Claude Code subagents cannot spawn other subagents. (Claude
  Code subagent docs.)
- Opus 4.7 effort levels: `low`, `medium`, `high`, `xhigh`,
  `max`. Sonnet 4.6 effort levels: `low`, `medium`, `high`,
  `max` (no `xhigh`). Haiku 4.5 does not support the new
  effort parameter; thinking is budget-based.

The next step after this SPEC's draft lands is
`/speccy-tasks SPEC-0032` to decompose the SPEC into a checklist
of agent-sized tasks in TASKS.md.
