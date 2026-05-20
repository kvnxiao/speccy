---
id: SPEC-0032
slug: phase-model-pinning
title: Per-phase model and effort pinning across the lifecycle
status: implemented
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
Sonnet runs, and everything else inherits the parent session. The
asymmetric assignment is deliberate. Business, tests, and
architecture reviewers carry semantic adversarial load and stay on
Opus at xhigh. Security carries pattern-plus-judgment load and
moves to Sonnet at high. Style and docs reviewers carry pure
pattern load and move to Sonnet at medium. The implementer-grade
mechanical phases (`tasks`, `work`, `ship`) pin to Sonnet at medium.
The remaining two phases (`init` scaffolding, `review` orchestrator)
are intentionally left unpinned — they inherit the parent session's
model. Haiku is deliberately not used anywhere in this SPEC: it
mirrors Codex's current limitation (Codex skills cannot pin a model
at all), it sidesteps Haiku 4.5's smaller context window for work
that may need to read substantial repository state, and on the
`review` orchestrator specifically it would starve the
verdict-consolidation work introduced in REQ-009 (F-10 absorption).

The cost-and-time win is desirable but Claude Code's automatic-fork
primitive (the auto-fork shape an earlier draft mandated; see DEC-001
and the third Changelog row) carried a UX cost severe enough on
multi-minute phase work to disqualify it as the default. A forked
subagent's intermediate tool output stays inside its isolated
context by design; only the final return message reaches the parent
session. For a single-shot phase that completes in seconds the
silence is harmless, but a real `/speccy-work` invocation touches
ten-plus files and runs four hygiene gates, and during those minutes
the parent session shows no streaming output, no progress signal,
and no way for the user to tell if the subagent is alive, broken,
or making progress. The user cannot interrupt sensibly because
there is nothing in the parent session to interrupt. The
cost-and-time pin is therefore preserved as an opt-in path rather
than an automatic one: Speccy ships three pinned
`.claude/agents/speccy-<phase>.md` subagent files for `tasks`,
`work`, and `ship`. Each carries `model: sonnet[1m]` and
`effort: medium`. The `speccy-init` phase does not ship an agent
file (per DEC-009 / the fourth Changelog row): its load-bearing
work is the interactive 7-question Q&A that composes
`## Product north star`, which is parent-session work, and there
is no pinned tier it would opt into. Users who want the pinned
execution path for `tasks` / `work` / `ship` invoke the subagent
explicitly via the host's subagent surface (`Agent` tool /
`/agent <name>`). The three matching pinned-phase SKILL.md files
on Claude Code carry thin-stub bodies that defer to the agent
file as the canonical procedure source; the agent file's body is
the single on-disk source of truth per DEC-009. The four
`/speccy-<phase>` slash commands themselves (including
`/speccy-init`) carry no `context:` or `agent:` frontmatter and
run in the parent session by default with full streaming visibility
into tool output. The `/speccy-review` skill likewise stays
unpinned, unforked, and full-body (no stub treatment): it must keep
Task-tool access to fan out to the reviewer subagents (Claude
Code's documented rule is that subagents cannot spawn other
subagents), and the orchestrator turn is no longer pure
dispatch — REQ-009 makes it the sole writer to TASKS.md for
review-induced state transitions, which is load that needs the
parent session's full capacity rather than a
deliberately-downgraded tier.

Codex's CLI exposes per-subagent `model` and `model_reasoning_effort`
in `.codex/agents/*.toml` files. Codex skills do not accept a
model field, and Codex does not expose a Claude-Code-equivalent
auto-fork primitive — but Speccy no longer relies on auto-fork on
either host, so the Codex shape and the Claude Code shape are
symmetric: the cost-and-time win is opt-in on both hosts via the
subagent surface. The three new `.codex/agents/` phase-worker TOML
files (mirroring the three pinned Claude Code phase-worker agent
files for `tasks`, `work`, and `ship`) plus the existing six
reviewer TOML files carry the same pin assignments as the Claude
Code half. The three matching pinned-phase Codex skill bodies at
`.agents/skills/speccy-<phase>/SKILL.md` carry thin-stub bodies
that defer to `.codex/agents/speccy-<phase>.toml` as the canonical
procedure source and recommend `/agent speccy-<phase>` as the
pinned execution path, mirroring the Claude Code stub-skill shape
per DEC-009. The pin assignment table and the opt-in invocation
surface are documented under a new "Model pinning" section in the
project README.

The three conversational skills (`speccy-brainstorm`, `speccy-plan`,
`speccy-amend`) stay unpinned. They are multi-turn dialogues with
the user; Claude Code's skill `model:` override only applies for
the first assistant turn after invocation, so a frontmatter pin
would only steer the opening response and revert immediately. The
assumption is that the user starts a session at the latest Opus
generation at xhigh or max effort, which is the contract-writing
tier these phases need; if the session is on something cheaper,
the user is in control of switching via `/model`.

Pin values use stable model identifiers rather than long-form
versioned IDs. On Claude Code, the model field is an Anthropic
alias suffixed with the `[1m]` 1M-context-window selector
(`opus[1m]`, `sonnet[1m]`) so the forked subagent has the headroom
to read substantial repository state without truncation; the
default 200K-context variant is rejected because phase workers
routinely read TASKS.md plus SPEC.md plus multiple modules in one
pass. On Codex, the model field is the current GPT generation
(`gpt-5.5` as of this SPEC) and the tier shape lives entirely in
`model_reasoning_effort` since OpenAI does not expose an
Opus/Sonnet/Haiku-style tier axis on the model identifier. Codex
does not expose a context-window selector on its model field.
Both alias forms float forward as their vendors ship new
generations; the user is free to lock to a specific version by
editing the ejected files post-`speccy init`, e.g. swapping
`sonnet[1m]` for `claude-sonnet-4-6[1m]` on Claude Code or
swapping `gpt-5.5` for a specific GPT-5.5 dated snapshot on Codex.

All ejection paths (the `.claude/skills/`, `.claude/agents/`,
`.codex/agents/`, and `.agents/skills/` trees rendered by
`speccy init` into a fresh project) carry the same pin assignments
that Speccy uses in its own in-tree dogfood pack. The change is
bounded: three new pinned Claude Code phase-worker subagent files
(`tasks`, `work`, `ship` at `sonnet[1m]`/medium; no `speccy-init`
agent ships per DEC-009); three new Codex phase-worker agent TOML
files (mirroring shape; no Codex agent for `speccy-init` or for
the unpinned `/speccy-review`); frontmatter additions to the six
existing reviewer files on each host; stub-shape edits to the
three pinned phase-worker SKILL.md bodies on both hosts so they
defer to the matching subagent per DEC-009; no frontmatter edits
on any `.claude/skills/speccy-<phase>/SKILL.md` files themselves —
they stay unpinned so slash-command invocation runs in the parent
session with full visibility; reviewer-prompt edits so each
reviewer returns a structured verdict to the orchestrator rather
than writing to TASKS.md directly (REQ-009); orchestrator body
edits in `/speccy-review` so the parent session consolidates the
verdict messages and applies state transitions to TASKS.md
serially; the shared phase-body sources move from
`resources/modules/skills/` to `resources/modules/phases/` per
DEC-009; one new README section; one README audit pass for
current-repo-state accuracy.

## Goals

<goals>
- The four `/speccy-<phase>` slash commands for `tasks`, `work`,
  `ship`, and `init` on Claude Code run in the parent session at
  the parent session's model with full streaming visibility into
  tool output. None of the four `.claude/skills/speccy-<phase>/SKILL.md`
  files carry `context:`, `agent:`, `model:`, or `effort:`
  frontmatter — they are unpinned by default.
- Users who want the pinned execution path for one of the
  three pinned mechanical phases (`tasks`, `work`, `ship`) can
  invoke the matching subagent explicitly via the host's
  subagent surface (`Agent` tool / `/agent <name>`). Each of
  the three implementer-grade subagents runs at Sonnet (1M
  context) / medium effort via the
  `.claude/agents/speccy-<phase>.md` frontmatter pin.
  `speccy-init` ships no subagent file (per DEC-009): its
  load-bearing work is an interactive 7-question Q&A composing
  the project's `## Product north star`, which is parent-session
  work, and there is no pinned tier to opt into.
- Every invocation of `/speccy-review` on Claude Code runs its
  orchestrator turn in the parent session at the parent session's
  model (no frontmatter pin), preserving Task-tool access to spawn
  the reviewer fan-out (four personas by default — business,
  tests, security, style — with `reviewer-architecture` and
  `reviewer-docs` available as explicit-invoke additions) and the
  parent session's full capacity for verdict consolidation per
  REQ-009.
- When the user opts into the pinned subagent path explicitly,
  the subagent's intermediate tool output stays inside its
  isolated context per the host's normal subagent semantics; only
  the subagent's final return message reaches the parent session.
  When the user runs the slash command directly, full
  tool-output streaming is preserved in the parent session.
- Each reviewer subagent returns a structured verdict to the
  `/speccy-review` orchestrator as its final message and does not
  write to TASKS.md directly. The orchestrator parses the six
  return messages, consolidates them, and is the sole writer to
  TASKS.md for review-induced state transitions. Parallel reviewer
  writes to the same TASKS.md file are eliminated by construction.
- Each reviewer subagent on Claude Code runs at the model and
  effort matching its work shape: `reviewer-business`,
  `reviewer-tests`, and `reviewer-architecture` at `opus[1m]` /
  xhigh; `reviewer-security` at `sonnet[1m]` / high;
  `reviewer-style` and `reviewer-docs` at `sonnet[1m]` / medium.
  Every Claude Code pin uses the `[1m]` 1M-context-window suffix.
- Each reviewer subagent on Codex pins to `gpt-5.5` (the current
  GPT generation) and expresses the work-shape tier via
  `model_reasoning_effort`: semantic reviewers
  (`reviewer-business`, `reviewer-tests`, `reviewer-architecture`)
  at `high`; the pattern-plus-judgment reviewer
  (`reviewer-security`) at `medium`; pure-pattern reviewers
  (`reviewer-style`, `reviewer-docs`) at `low`. OpenAI does not
  expose an Opus/Sonnet-style tier axis on the model identifier,
  so the tier shape is entirely effort-driven on Codex while it
  is model-plus-effort on Claude Code.
- Each pinned mechanical phase on Codex (`speccy-tasks`,
  `speccy-work`, `speccy-ship`) gains a
  `.codex/agents/speccy-<phase>.toml` subagent file with the same
  pin assignment as its Claude Code counterpart, invocable via
  `/agent <name>`. `/speccy-review` does not get a Codex agent
  file: it stays unpinned on both hosts because the orchestrator
  is load-bearing per REQ-009, and a Codex agent file pinning it
  to a tier weaker than the user's session would silently regress.
  `/speccy-init` does not get a Codex agent file either (per
  DEC-009).
- The three pinned phase-worker SKILL.md bodies (`tasks`, `work`,
  `ship`) carry thin-stub bodies that defer to their matching
  subagent file as the canonical procedure source. The stub
  names the agent file path and the `/agent speccy-<phase>`
  invocation pattern; the agent file's body (included from
  `resources/modules/phases/speccy-<phase>.md`) carries the full
  procedural Steps and When-to-use sections. `/speccy-init`'s
  SKILL.md remains full-body since no subagent exists to defer
  to. The stub shape is symmetric across both hosts (Claude Code
  SKILL.md points to `.claude/agents/speccy-<phase>.md`; Codex
  SKILL.md points to `.codex/agents/speccy-<phase>.toml`).
- The pin shape respects each model family's effort-parameter
  support: Opus pins include an `effort:` field with values up to
  `xhigh` or `max`; Sonnet pins include an `effort:` field with
  values up to `max` (never `xhigh`, which is Opus-only). Haiku is
  not used in any pin in this SPEC.
- Every Claude Code `model:` value is an Anthropic alias suffixed
  with the `[1m]` 1M-context-window selector (`opus[1m]`,
  `sonnet[1m]`). Every Codex `model` value is the current GPT
  generation identifier (`gpt-5.5`). Long-form versioned snapshot
  IDs do not appear in any shipped file; the ejected files are
  user-editable so a project that wants to lock to a specific
  snapshot can do so post-eject (e.g. swap `sonnet[1m]` for
  `claude-sonnet-4-6[1m]` on Claude Code or `gpt-5.5` for a dated
  snapshot on Codex).
- `speccy init` in a fresh project renders all new and updated
  agent files with the same pin assignments as Speccy's own in-tree
  dogfood pack.
- The phase-worker subagent body on Claude Code derives from
  `resources/modules/phases/speccy-<phase>.md` via MiniJinja
  `{% include %}`, the same shared-body pattern the reviewer
  agents already use through `resources/modules/personas/`. The
  Codex agent TOML files include their bodies the same way. The
  three matching pinned-phase SKILL.md template bodies on both
  hosts do not include the shared phase module: their bodies are
  hardcoded thin-stub prose per DEC-009. (The earlier shared
  source path under `resources/modules/skills/` was renamed to
  `resources/modules/phases/` to reflect what the modules
  actually carry — phase-level procedure, not skill-wrapped
  recipes.)
- The project README gains a new "Model pinning" section that
  names the assignment table, describes the opt-in subagent
  invocation path that delivers the pin on both hosts
  (`Agent` / `/agent speccy-<phase>`), names the
  slash-command-runs-in-parent-session default, and tells users
  how to override individual pins.
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
- No pin enforcement. A user can ignore the `/agent
  speccy-<phase>` pointer on either host and run the slash command
  directly, in which case the work executes in the parent session
  at the parent session's model. Speccy makes the recommended
  shape easy and obvious; it does not block alternative paths.
- No conversational-skill pins. `/speccy-brainstorm`,
  `/speccy-plan`, and `/speccy-amend` do not gain `model:`,
  `effort:`, `context:`, or `agent:` frontmatter additions. They
  continue to run at the parent session's model and effort
  throughout their multi-turn dialogues.
- No version-locked snapshot IDs. `model:` values are aliases
  (`opus[1m]` / `sonnet[1m]` on Claude Code; `gpt-5.5` on Codex)
  so the pin floats forward as each vendor ships new generations.
  A user who wants their loop to behave deterministically across
  a model upgrade can edit the ejected files to lock a specific
  dated snapshot; Speccy does not lock on their behalf.
- No measurement step. The SPEC does not include a benchmarking
  task that records token-and-latency deltas before and after the
  pin lands. Dogfooding governs whether an individual pin needs
  re-tuning, and a re-pin is a future amendment, not a
  measurement-loop product.
- No Haiku tier. The SPEC ships zero Haiku pins. Phases that were
  candidates for Haiku in earlier drafts (`/speccy-init` and the
  `/speccy-review` orchestrator) are left unpinned and inherit the
  parent session instead. Codex skills cannot pin a model at all,
  so the unpinned shape also mirrors the Codex contract one-to-one
  on those phases. A future Haiku release that grows the
  effort enum and a wider context window can be re-evaluated via
  amendment.
- No parallel writes to TASKS.md from reviewer subagents. Per
  REQ-009, reviewers return verdicts to the orchestrator as their
  final message and never edit TASKS.md directly. The orchestrator
  is the sole writer for review-induced state transitions, and the
  race condition that arises when six parallel subagents target
  the same file is eliminated by construction rather than papered
  over with locking or retries.
- No automatic-dispatch implementation on either host. The
  cost-and-time win is opt-in via `/agent speccy-<phase>` on both
  hosts (this is a deliberate retreat from the auto-fork pattern
  that the SPEC's prior draft mandated on Claude Code — see DEC-001
  and the third Changelog row for the UX-cost reasoning). The
  SPEC does not ship workarounds for Codex's missing
  skill-spawns-subagent primitive nor reintroduce
  `context: fork` on Claude Code; the symmetric opt-in shape is
  the v1 default on both hosts.
- No new `resources/modules/agents/` directory. Phase-worker
  subagent bodies and the matching SKILL.md template bodies were
  previously co-sourced from `resources/modules/skills/`; under
  DEC-009 the shared source is renamed to
  `resources/modules/phases/` (Speccy-workflow naming, not
  host-harness naming) and the inclusion becomes asymmetric: the
  agent template still includes the shared phase module, but the
  three pinned SKILL.md template bodies are hardcoded thin-stub
  prose that defer to the matching subagent. Procedural duplication
  is eliminated by construction: only the agent file carries the
  full body. No `resources/modules/agents/` directory is created
  because the body is a phase artifact, not a host-skin artifact.
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
- As a Speccy user finishing a contract-writing phase in a Claude
  Code session open at Opus 4.7 / xhigh, when I want the
  implementation work to drop to Sonnet 4.6 / medium I invoke
  `/agent speccy-work` first so the pinned subagent runs the work
  at the cheaper tier; I am not forced through an auto-fork that
  hides the subagent's tool output from my parent session for
  minutes at a stretch.
- As a Speccy user invoking `/speccy-work` directly without the
  `/agent` opt-in, I want the work to run in my parent session
  with full streaming visibility into every tool call so I can see
  progress, intervene if something looks wrong, and never face a
  silent multi-minute window where I cannot tell if the worker is
  alive, broken, or making progress.
- As a Speccy user iterating across `/agent speccy-work`,
  `/speccy-review`, `/agent speccy-ship` in a single chat session,
  I want each opt-in subagent's intermediate tool output to stay
  out of my main session context so the session does not bloat
  with every worker's file reads, code edits, and command output
  as the loop progresses.
- As a Speccy user invoking `/speccy-review`, I want the
  orchestrator to run at my parent session's model so it has the
  context capacity to parse the parallel reviewer return messages
  (four by default, up to six when architecture and docs are
  explicitly invoked), consolidate their verdicts, and apply
  state transitions to TASKS.md serially in a single dispatcher
  turn — without losing Task-tool access to spawn the reviewer
  subagents in the first place.
- As a Speccy user iterating on the review loop, I want the
  reviewer subagents to return their verdicts to the orchestrator
  as their final message rather than writing to TASKS.md directly,
  so I never see a torn write from two parallel reviewers landing
  in the same file at the same instant.
- As the `reviewer-business` persona reviewing a task that the
  implementer claims is complete, I want my own subagent to run at
  `opus[1m]` / xhigh so my semantic adversarial reasoning has the
  capacity to catch drift between SPEC intent and shipped diff
  rather than missing nuance at a lower tier, and the 1M-context
  variant so I can read the full SPEC plus diff plus task body
  without truncation.
- As the `reviewer-style` persona reviewing the same task, I want
  my subagent to run at `sonnet[1m]` / medium so my pattern-matching
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
- As a maintainer auditing the in-tree workspace, I want zero
  Haiku pins anywhere in the shipped pack so I never have to reason
  about Haiku's smaller context window or absent effort enum
  during routine review of agent frontmatter.
- As a Speccy user who wants to dogfood the loop on Sonnet-only
  (no Opus access in my plan), I want every ejected agent file to
  be a plain text file I can edit to change the pin, so I can
  swap all six reviewers and all three pinned phase workers to
  Sonnet variants by hand without fighting the CLI.
- As a Speccy user pinning a project to a specific model snapshot
  for reproducibility, I want the ejected files to use floating
  aliases by default but accept full snapshot IDs when I edit
  them, so I can swap `model: opus[1m]` for
  `model: claude-opus-4-7[1m]` in the Claude Code files I care
  about (and `model: gpt-5.5` for a dated GPT-5.5 snapshot on the
  Codex side) while leaving the rest of the pack on aliases.
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
  pin assignments, opt-in subagent invocation surface), so I do
  not need to grep the code to discover what the documentation
  has drifted past.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Three Claude Code phase-worker subagents ship pinned for opt-in invocation

Three new subagent files ship at `.claude/agents/speccy-tasks.md`,
`.claude/agents/speccy-work.md`, and
`.claude/agents/speccy-ship.md`. Each declares `model: sonnet[1m]`
and `effort: medium` in its frontmatter. No
`.claude/agents/speccy-init.md` ships (per DEC-009 / the fourth
Changelog row): `/speccy-init`'s load-bearing work is interactive
parent-session Q&A and there is no pinned tier it would opt into.

The four matching slash-command skills at
`.claude/skills/speccy-tasks/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.claude/skills/speccy-ship/SKILL.md`, and
`.claude/skills/speccy-init/SKILL.md` carry no `context:`,
`agent:`, `model:`, or `effort:` frontmatter keys — only the
pre-existing `name:` / `description:` pair. The slash command
runs in the parent session by default. Users who want the
cost-and-time-win pinned execution path for one of the three
pinned phases invoke the subagent explicitly via the host's
subagent surface (Claude Code's `Agent` tool or the
`/agent speccy-<phase>` pattern). The earlier
`context: fork` / `agent:` auto-fork wiring was tried during
T-001 and reverted before T-002 landed; see the third Changelog
row and DEC-001 for the UX-cost reasoning.

The three pinned-phase SKILL.md bodies are thin stubs that defer
to their matching subagent file per DEC-009 / REQ-010. The stub
body names the agent file as the canonical procedure source and
the `/agent speccy-<phase>` invocation as the pinned execution
path; the agent file's body (included from
`resources/modules/phases/speccy-<phase>.md`) carries the full
procedural Steps and When-to-use sections. `/speccy-init`'s
SKILL.md remains full-body — no subagent file exists to defer
to.

<done-when>
- The three new agent files at `.claude/agents/speccy-tasks.md`,
  `.claude/agents/speccy-work.md`, and
  `.claude/agents/speccy-ship.md` exist.
- No file exists at `.claude/agents/speccy-init.md` (per
  DEC-009).
- Each new agent file's frontmatter contains `name:`,
  `description:`, `model: sonnet[1m]`, and `effort: medium`
  fields.
- Each new agent file's body includes the corresponding shared
  phase body via
  `{% include "modules/phases/speccy-<phase>.md" %}` in the
  templated source under
  `resources/agents/.claude/agents/speccy-<phase>.md.tmpl`,
  matching the existing reviewer-agent template pattern.
- The four `.claude/skills/speccy-<phase>/SKILL.md` files (for
  `phase` in {`tasks`, `work`, `ship`, `init`}) do not contain
  `context:`, `agent:`, `model:`, or `effort:` keys in their
  YAML frontmatter.
- The three pinned-phase SKILL.md bodies (for `tasks`, `work`,
  `ship`) are thin stubs that name the matching agent file and
  the `/agent speccy-<phase>` invocation pattern, per REQ-010.
  `/speccy-init`'s SKILL.md remains full-body (no stub
  transformation).
- Invoking `/speccy-work` on Claude Code in a session set to any
  model runs the work in the parent session at that model with
  streaming tool output visible in the parent session.
- Invoking `/agent speccy-work` on Claude Code activates a
  subagent that runs at Sonnet 1M context (the test is
  observational: the host reports the activated subagent's
  pinned model).
</done-when>

<behavior>
- Given the post-SPEC `.claude/skills/speccy-work/SKILL.md`, when
  its YAML frontmatter is parsed, then it does not contain
  `context`, `agent`, `model`, or `effort` keys.
- Given the post-SPEC `.claude/agents/speccy-work.md`, when its
  YAML frontmatter is parsed, then it contains `model: sonnet[1m]`
  and `effort: medium` keys with those literal values.
- Given the post-SPEC `.claude/agents/` directory, when listed,
  then it contains no file named `speccy-init.md` (per DEC-009).
- Given `resources/agents/.claude/agents/speccy-work.md.tmpl`,
  when read, then it includes
  `{% include "modules/phases/speccy-work.md" %}` so the
  rendered agent file inlines the shared phase body.
- Given the three pinned-phase SKILL.md template sources at
  `resources/agents/.claude/skills/speccy-<phase>/SKILL.md.tmpl`
  for `phase` in {`tasks`, `work`, `ship`}, when each is read,
  then each carries a thin-stub body that names the matching
  agent file and the `/agent speccy-<phase>` invocation
  pattern.
</behavior>

<scenario id="CHK-001">
Given each of the four files at
`.claude/skills/speccy-tasks/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.claude/skills/speccy-ship/SKILL.md`, and
`.claude/skills/speccy-init/SKILL.md` after this SPEC's tasks
land, when each one's YAML frontmatter is parsed, then none of
the keys `context`, `agent`, `model`, or `effort` are present.

Given the three pinned agent files at
`.claude/agents/speccy-tasks.md`,
`.claude/agents/speccy-work.md`, and
`.claude/agents/speccy-ship.md`, when each is read, then each
file exists and its YAML frontmatter contains
`model: sonnet[1m]` and `effort: medium`.

Given `.claude/agents/speccy-init.md`, when checked for
existence, then no file is present (per DEC-009).

Given each templated source at
`resources/agents/.claude/agents/speccy-<phase>.md.tmpl` for
`phase` in {`tasks`, `work`, `ship`}, when read, then it
contains the literal string
`{% include "modules/phases/speccy-<phase>.md" %}` with the
matching phase name. No template at
`resources/agents/.claude/agents/speccy-init.md.tmpl` exists.

Given each phase-worker shared body source at
`resources/modules/phases/speccy-<phase>.md` for `phase` in
{`tasks`, `work`, `ship`, `init`}, when read, then each file
exists at the renamed path. No matching files exist at
`resources/modules/skills/speccy-<phase>.md` for the same four
phase names (per REQ-010 rename).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: speccy-review stays unpinned and runs in the parent session

`.claude/skills/speccy-review/SKILL.md` carries no `model:`,
`effort:`, `context:`, or `agent:` frontmatter fields beyond its
existing `name:` / `description:` pair. The skill body is updated
per REQ-009 to describe the summarize-back-to-orchestrator
consolidation flow, but no pinning frontmatter is added. The
reviewer fan-out mechanism (Task-tool spawning of the reviewer
subagents from the orchestrator's body — four personas by default,
plus `reviewer-architecture` and `reviewer-docs` as explicit-invoke
additions) is preserved. The
orchestrator turn runs in the parent session at the parent
session's model and effort, which is sufficient capacity for the
verdict-consolidation work introduced in REQ-009 and which sidesteps
the context-window risk that a Haiku pin would introduce.

<done-when>
- `.claude/skills/speccy-review/SKILL.md` YAML frontmatter does not
  contain `model:`, `effort:`, `context:`, or `agent:` keys.
- The skill body (below frontmatter) reflects the REQ-009
  consolidation flow.
- No new file at `.claude/agents/speccy-review.md` is created on
  Claude Code.
</done-when>

<behavior>
- Given the post-SPEC `.claude/skills/speccy-review/SKILL.md`,
  when its YAML frontmatter is parsed, then it does not contain
  `model`, `effort`, `context`, or `agent` keys.
- Given an invocation of `/speccy-review` on Claude Code in a
  session set to any model, when the orchestrator turn executes,
  then the Task-tool calls that spawn the reviewer fan-out (the
  four default personas, plus any explicit-invoke additions)
  succeed (preserving fan-out behavior) and the orchestrator runs
  at the parent session's model.
</behavior>

<scenario id="CHK-002">
Given `.claude/skills/speccy-review/SKILL.md` after this SPEC's
tasks land, when its YAML frontmatter is parsed, then none of the
keys `model`, `effort`, `context`, or `agent` are present.

Given the templated source at
`resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`,
when read, then its frontmatter mirrors the rendered file:
`model` / `effort` / `context` / `agent` keys all absent.

Given the post-SPEC `.claude/agents/` directory contents, when
listed, then there is no file named `speccy-review.md` (the
orchestrator does not become a subagent on Claude Code).
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
  `model: opus[1m]`, `effort: xhigh`.
- `reviewer-security`: `model: sonnet[1m]`, `effort: high`.
- `reviewer-style`, `reviewer-docs`: `model: sonnet[1m]`,
  `effort: medium`.

Every Claude Code reviewer pin uses the `[1m]` 1M-context-window
suffix so each reviewer has the headroom to read full SPEC + diff
+ task body without truncation. The reviewer body content (below
frontmatter) is updated per REQ-009 to direct the reviewer to
return its verdict to the orchestrator rather than write to
TASKS.md directly; aside from that REQ-009 edit, body content is
unchanged in every file.

<done-when>
- Each of the six reviewer agent files contains `model:` and
  `effort:` keys in its YAML frontmatter with the values named
  above.
- The body content (below frontmatter) of each reviewer agent
  file is unchanged from the pre-SPEC version except for the
  REQ-009 edits to the verdict-return contract.
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
  then `model: opus[1m]` and `effort: xhigh` are present.
- Given `reviewer-security.md`, when its frontmatter is parsed,
  then `model: sonnet[1m]` and `effort: high` are present.
- Given `reviewer-style.md` and `reviewer-docs.md`, when each
  one's frontmatter is parsed, then `model: sonnet[1m]` and
  `effort: medium` are present.
- Given the body content (below frontmatter) of each reviewer
  file, when diffed against the pre-SPEC version, then the body
  is byte-identical except for the REQ-009 verdict-return-contract
  edits.
</behavior>

<scenario id="CHK-003">
Given `.claude/agents/reviewer-business.md`,
`.claude/agents/reviewer-tests.md`, and
`.claude/agents/reviewer-architecture.md` after this SPEC's tasks
land, when each one's YAML frontmatter is parsed, then each
contains `model: opus[1m]` and `effort: xhigh`.

Given `.claude/agents/reviewer-security.md` after this SPEC's
tasks land, when its YAML frontmatter is parsed, then it contains
`model: sonnet[1m]` and `effort: high`.

Given `.claude/agents/reviewer-style.md` and
`.claude/agents/reviewer-docs.md` after this SPEC's tasks land,
when each one's YAML frontmatter is parsed, then each contains
`model: sonnet[1m]` and `effort: medium`.

Given the body content (below frontmatter) of each of the six
reviewer agent files, when diffed against the pre-SPEC version,
then the only body differences are the REQ-009 verdict-return
contract edits.

Given the matching template files under
`resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`,
when each one is read, then its frontmatter carries the same
`model:` and `effort:` values as the rendered output above.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Codex parallel ships matching pins at the subagent layer

Four new Codex subagent files exist at
`.codex/agents/speccy-tasks.toml`,
`.codex/agents/speccy-work.toml`,
`.codex/agents/speccy-ship.toml`, and
`.codex/agents/speccy-init.toml`. The three pinned files
(`speccy-tasks`, `speccy-work`, `speccy-ship`) declare
`name`, `description`, `model = "gpt-5.5"`, and
`model_reasoning_effort = "medium"`. The unpinned file
(`speccy-init.toml`) declares `name`, `description`, and the
`developer_instructions` include, but no `model` field and no
`model_reasoning_effort` field — the agent inherits the parent
session's model when invoked via `/agent speccy-init`. No Codex
agent file is created for `speccy-review`: the orchestrator stays
unpinned on Codex as well as on Claude Code (a pinned Codex agent
file would silently downgrade the orchestrator below the user's
session model, regressing REQ-009 capacity). Each new file's
`developer_instructions` field includes the shared phase body via
`{% include "modules/skills/speccy-<phase>.md" %}` in the template
source.

The six existing Codex reviewer files at
`.codex/agents/reviewer-<persona>.toml` gain `model = "gpt-5.5"`
on every file and `model_reasoning_effort` keys whose values
match the work-shape tier of each persona:
`reviewer-business`, `reviewer-tests`, `reviewer-architecture`
at `"high"`; `reviewer-security` at `"medium"`;
`reviewer-style`, `reviewer-docs` at `"low"`. OpenAI does not
expose an Opus/Sonnet-style tier axis on its model identifier,
so the asymmetric semantic-vs-pattern shape that lives in both
`model` and `effort` on Claude Code lives entirely in
`model_reasoning_effort` on Codex.

The four Codex phase-worker skill bodies at
`.agents/skills/speccy-<phase>/SKILL.md` (for `tasks`, `work`,
`ship`, `init`) each gain a one-line pointer at the top of the
body (after the frontmatter, before the existing skill content)
naming the corresponding `.codex/agents/speccy-<phase>.toml`
invocation path with the literal text "for the cost-and-time-win
execution path, invoke this skill via `/agent speccy-<phase>`
first" (or substantially equivalent prose). The
`speccy-review` Codex skill body gains no such pointer (no
matching Codex agent file exists).

<done-when>
- Four new files exist at `.codex/agents/speccy-tasks.toml`,
  `.codex/agents/speccy-work.toml`,
  `.codex/agents/speccy-ship.toml`, and
  `.codex/agents/speccy-init.toml`.
- No file exists at `.codex/agents/speccy-review.toml`.
- The three pinned files (`speccy-tasks`, `speccy-work`,
  `speccy-ship`) contain `name`, `description`,
  `model = "gpt-5.5"`, and `model_reasoning_effort = "medium"`
  keys.
- The unpinned file (`speccy-init.toml`) contains `name` and
  `description` but does not contain `model` or
  `model_reasoning_effort` keys.
- Each new file's `developer_instructions` field renders the
  shared phase body via templated include.
- The six existing reviewer TOML files contain `model = "gpt-5.5"`
  and a `model_reasoning_effort` whose value matches the persona's
  work-shape tier: `"high"` for `reviewer-business`,
  `reviewer-tests`, `reviewer-architecture`; `"medium"` for
  `reviewer-security`; `"low"` for `reviewer-style`,
  `reviewer-docs`.
- The four Codex phase-worker SKILL.md bodies for `tasks`, `work`,
  `ship`, `init` each contain a one-line pointer at the top of
  the body naming the corresponding TOML file and the
  `/agent <name>` invocation. The `speccy-review` Codex skill
  body contains no such pointer.
</done-when>

<behavior>
- Given each new file at
  `.codex/agents/speccy-<phase>.toml` for `phase` in
  {`tasks`, `work`, `ship`, `init`}, when parsed, then `name`
  and `description` keys are present.
- Given `.codex/agents/speccy-work.toml`, when parsed, then
  `model = "gpt-5.5"` and `model_reasoning_effort = "medium"` are
  present.
- Given `.codex/agents/speccy-init.toml`, when parsed, then
  `model` is not present and `model_reasoning_effort` is not
  present.
- Given the post-SPEC `.codex/agents/` directory listing, when
  scanned, then there is no file named `speccy-review.toml`.
- Given each Codex reviewer TOML file, when parsed, then
  `model = "gpt-5.5"` is present and `model_reasoning_effort`
  carries the value matching the persona's work-shape tier
  (`"high"` for semantic reviewers, `"medium"` for security,
  `"low"` for style and docs).
- Given each pinned Codex phase-worker SKILL.md file
  (`tasks`, `work`, `ship`, `init`), when its body is read, then
  the first non-frontmatter line names the corresponding TOML
  file path and the `/agent` invocation pattern.
</behavior>

<scenario id="CHK-004">
Given `.codex/agents/speccy-tasks.toml`,
`.codex/agents/speccy-work.toml`,
`.codex/agents/speccy-ship.toml`, and
`.codex/agents/speccy-init.toml` after this SPEC's tasks land,
when each is read, then each file exists and parses as TOML.

Given the post-SPEC `.codex/agents/` directory listing, when
scanned, then there is no file named `speccy-review.toml`.

Given each of the three pinned Codex phase-worker files
(`speccy-tasks.toml`, `speccy-work.toml`, `speccy-ship.toml`),
when its TOML is parsed, then it contains `model = "gpt-5.5"`
and `model_reasoning_effort = "medium"`.

Given the unpinned Codex phase-worker file (`speccy-init.toml`),
when parsed, then `model` is not present and
`model_reasoning_effort` is not present.

Given each Codex reviewer TOML file
(`.codex/agents/reviewer-<persona>.toml`), when parsed, then
`model = "gpt-5.5"` is present; `model_reasoning_effort` is
`"high"` for `reviewer-business`, `reviewer-tests`, and
`reviewer-architecture`, `"medium"` for `reviewer-security`,
and `"low"` for `reviewer-style` and `reviewer-docs`.

Given each pinned Codex phase-worker skill body
(`.agents/skills/speccy-<phase>/SKILL.md` for `phase` in
{`tasks`, `work`, `ship`, `init`}), when its first non-frontmatter
line is read, then the line names the corresponding
`.codex/agents/speccy-<phase>.toml` file and the
`/agent speccy-<phase>` invocation pattern.

Given the matching template files under
`resources/agents/.codex/agents/speccy-<phase>.toml.tmpl` and
`resources/agents/.agents/skills/speccy-<phase>/SKILL.md.tmpl`
for the four pinned phases, when each is read, then the templated
source produces a rendered file matching the requirements above
when `speccy init` processes it.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Pin shape respects model family capabilities

Every pinned model on Claude Code in this SPEC is Opus or Sonnet
— Haiku is not used. Every Claude Code `model:` value is an
Anthropic alias suffixed with the `[1m]` 1M-context-window
selector (`opus[1m]` or `sonnet[1m]`). On Codex, every `model`
value is `gpt-5.5` (the current GPT generation); Codex does not
expose a context-window selector on its model identifier and
does not expose an Opus/Sonnet-style tier axis, so the work-shape
tier on the Codex side lives entirely in
`model_reasoning_effort`. No long-form versioned snapshot IDs
(e.g. `claude-opus-4-7`, `claude-sonnet-4-6`,
`claude-opus-4-7[1m]`, dated `gpt-5.5-YYYY-MM-DD` strings) appear
in any shipped frontmatter, skill body, or agent body in
`resources/agents/` or in the in-tree dogfood pack under
`.claude/` and `.codex/`.

Every Opus pin on Claude Code includes an `effort:` field with a
value drawn from `low`, `medium`, `high`, `xhigh`, `max`. Every
Sonnet pin on Claude Code includes an `effort:` field with a
value drawn from `low`, `medium`, `high`, `max` — never `xhigh`,
which is Opus-only. Every pinned `model_reasoning_effort` on
Codex carries a value from `low`, `medium`, `high`, `xhigh`.
There are no Haiku pins to constrain.

<done-when>
- No file under `resources/agents/.claude/`,
  `resources/agents/.codex/`, `resources/agents/.agents/`, or the
  in-tree dogfood pack at `.claude/` and `.codex/` contains a
  long-form versioned model ID in `model:` or `model` frontmatter
  fields.
- No file under those trees contains a `model:` value equal to
  `haiku` or `haiku[1m]` (or any other Haiku-tier alias) and no
  Codex file contains a `model` value referencing a Haiku-equivalent
  cheap-tier model.
- Every Claude Code `model:` value matches the regex
  `^(opus|sonnet)\[1m\]$`. Every Codex `model` value equals
  the literal string `gpt-5.5`.
- Every Sonnet-pinned Claude Code file's `effort:` value is one
  of `low`, `medium`, `high`, `max` — never `xhigh`.
- Every Opus-pinned Claude Code file's `effort:` value is one of
  `low`, `medium`, `high`, `xhigh`, `max`.
- Every pinned Codex file's `model_reasoning_effort` value is one
  of `low`, `medium`, `high`, `xhigh`.
</done-when>

<behavior>
- Given every shipped agent file under `.claude/agents/` and
  `.codex/agents/`, when scanned for the literal substrings
  `claude-opus-`, `claude-sonnet-`, or `claude-haiku-`, then zero
  matches are found.
- Given every shipped agent and skill file under `.claude/` and
  `.codex/`, when scanned for any `model:` or `model` value
  containing the substring `haiku`, then zero matches are found.
- Given every shipped Codex agent file, when its `model` value is
  read, then it equals the literal string `gpt-5.5`.
- Given every Sonnet-pinned Claude Code file, when its `effort`
  value is read, then the value is not `xhigh`.
- Given every Opus-pinned Claude Code file, when its `effort`
  value is read, then the value is one of the documented Opus
  effort levels (`low`, `medium`, `high`, `xhigh`, `max`).
</behavior>

<scenario id="CHK-005">
Given every file under `resources/agents/` and the in-tree dogfood
pack at `.claude/` and `.codex/`, when grepped for the literal
substrings `claude-opus-`, `claude-sonnet-`, or `claude-haiku-`,
then zero matches are found in shipped frontmatter or body
content.

Given every file under those trees, when grepped for `model:`
(Claude Code) or `model` (Codex) values containing the substring
`haiku`, then zero matches are found.

Given each Claude Code pinned file (the three phase-worker agent
files for `tasks`, `work`, `ship`; the six reviewer agent files),
when each one's `model:` value is read, then it matches the regex
`^(opus|sonnet)\[1m\]$`.

Given each Codex pinned file (the three phase-worker TOML files
for `tasks`, `work`, `ship`; the six reviewer TOML files), when
each one's `model` value is read, then it equals the literal
string `gpt-5.5`.

Given each Sonnet-pinned Claude Code file (the three phase-worker
agents for `tasks`, `work`, `ship`; the Sonnet-tier reviewer
agents), when each one's `effort` value is read, then the value
is one of `low`, `medium`, `high`, or `max` — never `xhigh`.

Given each Opus-pinned Claude Code file (the three semantic
reviewer agents: `reviewer-business`, `reviewer-tests`,
`reviewer-architecture`), when each one's `effort` value is read,
then the value is one of `low`, `medium`, `high`, `xhigh`, or
`max`.

Given each pinned Codex file, when its `model_reasoning_effort`
value is read, then the value is one of `low`, `medium`, `high`,
or `xhigh`.
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
  effort frontmatter as the in-tree workspace (Sonnet[1m]/medium
  on three of them, no `model:` or `effort:` on the
  `speccy-init` agent).
- The same invocation creates the four new Codex phase-worker
  TOML files under `.codex/agents/speccy-<phase>.toml` for `tasks`,
  `work`, `ship`, `init` with the same model and reasoning_effort
  values as the in-tree workspace (`model = "gpt-5.5"` and
  `model_reasoning_effort = "medium"` on three; no `model` or
  `model_reasoning_effort` on `speccy-init.toml`). No
  `.codex/agents/speccy-review.toml` is created.
- The same invocation updates the six existing reviewer files on
  each host with the asymmetric pin assignment per REQ-003.
- The fresh `.claude/skills/speccy-<phase>/SKILL.md` files for the
  four mechanical-phase workers carry no `context:`, `agent:`,
  `model:`, or `effort:` frontmatter keys (slash-command invocation
  runs in the parent session by default per REQ-001).
- The fresh `.claude/skills/speccy-review/SKILL.md` contains no
  `model:`, `effort:`, `context:`, or `agent:` frontmatter keys.
- The fresh `.claude/skills/speccy-<phase>/SKILL.md` and
  `.agents/skills/speccy-<phase>/SKILL.md` files for `tasks`,
  `work`, `ship`, `init` each contain the one-line
  `/agent speccy-<phase>` invocation pointer at the top of the
  body (rendered from the shared source per REQ-008).
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
`init`) and the four new Codex phase-worker TOML files under
`.codex/agents/speccy-<phase>.toml` (for `tasks`, `work`, `ship`,
`init`); no `.codex/agents/speccy-review.toml` file is created.

Given the same rendered tree, when each new agent file's
frontmatter (or TOML keys) is parsed, then the pin values match
the in-tree dogfood pack: Sonnet[1m]/medium on Claude Code and
`gpt-5.5`/medium on Codex for `tasks`/`work`/`ship`, and no
`model` or effort field on `speccy-init` on either host.

Given the same rendered tree, when the six reviewer files on
each host are inspected, then their frontmatter (or TOML keys)
contain the asymmetric pin assignment per REQ-003 on Claude Code
(`opus[1m]` / `sonnet[1m]` with `effort:` tiers) and per REQ-004
on Codex (`gpt-5.5` with `model_reasoning_effort` tiers).

Given the four mechanical-phase SKILL.md files on Claude Code
in the rendered tree, when each is parsed, then none of the keys
`context`, `agent`, `model`, or `effort` are present (per the
slash-command-runs-in-parent-session default in REQ-001).

Given the `speccy-review` SKILL.md on Claude Code in the rendered
tree, when parsed, then it contains no `model:`, `effort:`,
`context:`, or `agent:` keys.

Given each of the four phase-worker SKILL.md files in the
rendered tree (on both hosts), when each is read, then the first
non-frontmatter content paragraph names the corresponding
`/agent speccy-<phase>` invocation pattern.

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
pinning" or equivalent. The new section names:

- The full pin assignment table covering all five mechanical
  phases and all six reviewer personas.
- The opt-in invocation surface on both hosts: invoke
  `/agent speccy-<phase>` (or use the host's equivalent subagent
  spawning tool) before running the phase command to activate the
  pinned subagent. The slash command on its own runs in the
  parent session at the parent session's model.
- The reason the SPEC retreated from the auto-fork pattern: the
  silent-by-design tool-output isolation produces minutes of
  dead air in the parent session on multi-minute phase work
  (referenced as a design lesson rather than a step-by-step UX
  recap).
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
- `README.md` contains a new section (titled "Model pinning" or
  substantially equivalent) that names the pin assignment, the
  opt-in subagent invocation surface that delivers the pin on
  both hosts, the override path, and the alias rationale.
- The new section names every Claude Code and Codex agent file
  that carries a pin (the four phase-worker agent files on Claude
  Code with the `[1m]` 1M-context suffix on three of them; the
  four phase-worker TOML files on Codex pinned to `gpt-5.5`; the
  six reviewer files on each host).
- The new section explicitly notes that the `/speccy-review`
  orchestrator stays unpinned on both hosts (no agent file on
  Codex, no model frontmatter on Claude Code) because the
  orchestrator owns TASKS.md writes per REQ-009 and needs the
  parent session's full capacity.
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
names the pin tier (Opus or Sonnet on Claude Code; `gpt-5.5` with
an effort tier on Codex; or "unpinned, inherits session") and the
effort level (or absence thereof) for every mechanical phase
(`speccy-tasks`, `speccy-work`, `speccy-ship`, `speccy-init`,
`speccy-review`) and every reviewer persona
(`reviewer-business`, `reviewer-tests`, `reviewer-architecture`,
`reviewer-security`, `reviewer-style`, `reviewer-docs`).

Given the same section, when read, then it describes the
opt-in subagent invocation pattern (`/agent speccy-<phase>` or
the host's equivalent) that activates the pin on both hosts, and
names the parent-session-by-default behavior of the
slash-command surface.

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

The three pinned Claude Code phase-worker agent files at
`.claude/agents/speccy-<phase>.md` and the three pinned Codex
phase-worker TOML files at `.codex/agents/speccy-<phase>.toml`
(`tasks`, `work`, `ship` on each host) each include their body
content from the shared source under
`resources/modules/phases/speccy-<phase>.md` via MiniJinja
`{% include %}`, mirroring the pattern the six reviewer agents
already use through `resources/modules/personas/`. Per DEC-009,
the shared source was renamed from `resources/modules/skills/`
to `resources/modules/phases/` to reflect what the modules
carry (phase-level procedure, not skill-wrapped recipes).

No new directory `resources/modules/agents/` is created. The
inclusion shape post-DEC-009 is asymmetric: the agent template
still includes the shared phase module; the matching pinned
SKILL.md template body is hardcoded thin-stub prose that names
the agent file as the procedure source. The duplication that
the prior symmetric-inclusion design rendered on disk
(byte-identical bodies in both the agent file and the SKILL.md
output) is eliminated by construction.

<done-when>
- Each template under
  `resources/agents/.claude/agents/speccy-<phase>.md.tmpl` for
  `phase` in {`tasks`, `work`, `ship`} contains
  `{% include "modules/phases/speccy-<phase>.md" %}`.
- Each template under
  `resources/agents/.codex/agents/speccy-<phase>.toml.tmpl` for
  `tasks`, `work`, `ship` includes the same shared body file
  in its `developer_instructions` value.
- No template files exist at
  `resources/agents/.claude/agents/speccy-init.md.tmpl` or
  `resources/agents/.codex/agents/speccy-init.toml.tmpl` (per
  DEC-009).
- The directory `resources/modules/agents/` does not exist.
- The shared body files at
  `resources/modules/phases/speccy-<phase>.md` exist for each
  of the four phase workers (`tasks`, `work`, `ship`, `init`).
  The `init` body file remains in `resources/modules/phases/`
  for consumption by `/speccy-init`'s SKILL.md, even though no
  subagent includes it.
- The directory `resources/modules/skills/` does not contain
  `speccy-tasks.md`, `speccy-work.md`, `speccy-ship.md`, or
  `speccy-init.md` files (post-rename).
- The three pinned-phase SKILL.md template bodies on both
  hosts (Claude Code and Codex) do not contain any
  `{% include "modules/phases/..." %}` directives; their
  bodies are hardcoded stub prose per REQ-010.
</done-when>

<behavior>
- Given each new template under
  `resources/agents/.claude/agents/speccy-<phase>.md.tmpl` for
  `phase` in {`tasks`, `work`, `ship`}, when read, then it
  contains a MiniJinja include directive naming
  `modules/phases/speccy-<phase>.md`.
- Given each new template under
  `resources/agents/.codex/agents/speccy-<phase>.toml.tmpl` for
  `tasks`, `work`, `ship`, when read, then its
  `developer_instructions` value includes the same shared body
  via MiniJinja include from `modules/phases/`.
- Given the `resources/modules/` directory, when listed, then it
  does not contain an `agents/` subdirectory and does contain a
  `phases/` subdirectory.
- Given the three pinned SKILL.md template sources for `tasks`,
  `work`, `ship` on either host, when each is read, then no
  `{% include "modules/phases/speccy-<phase>.md" %}` directive
  appears (the SKILL.md body is hardcoded stub prose).
</behavior>

<scenario id="CHK-008">
Given each template file under
`resources/agents/.claude/agents/speccy-tasks.md.tmpl`,
`speccy-work.md.tmpl`, and `speccy-ship.md.tmpl`, when read,
then each contains the literal string
`{% include "modules/phases/speccy-` followed by the phase name
and `.md" %}`. No template file exists at
`resources/agents/.claude/agents/speccy-init.md.tmpl`.

Given each template file under
`resources/agents/.codex/agents/speccy-tasks.toml.tmpl`,
`speccy-work.toml.tmpl`, and `speccy-ship.toml.tmpl`, when
read, then each contains a MiniJinja include directive naming
the matching `modules/phases/speccy-<phase>.md` body inside
the `developer_instructions` value. There is no
`speccy-review.toml.tmpl` or `speccy-init.toml.tmpl` under
`resources/agents/.codex/agents/`.

Given the directory `resources/modules/`, when listed, then it
contains `phases/`, `personas/`, `prompts/`, and (per
SPEC-0031) `examples/` subdirectories but does not contain an
`agents/` subdirectory.

Given each shared body file at
`resources/modules/phases/speccy-<phase>.md` for the four
mechanical phases (`tasks`, `work`, `ship`, `init`), when read,
then it exists. The `init` body file is consumed by
`/speccy-init`'s SKILL.md only (no subagent file exists for
init); the other three are consumed by their matching agent
files via `{% include %}` while the matching SKILL.md template
bodies are hardcoded thin-stub prose per REQ-010.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: Reviewer fan-out returns verdicts; orchestrator owns TASKS.md writes

Each reviewer subagent (the six `reviewer-<persona>` agents on
Claude Code — `business`, `tests`, `architecture`, `security`,
`style`, `docs` — plus their Codex counterparts) returns its
review verdict to the `/speccy-review` orchestrator as its final
message and does not write to TASKS.md directly. The verdict is
structured enough that the orchestrator can parse it without
ambiguity: at minimum a per-task pass/fail/needs-retry decision
and, on failure, the `<retry>` body text the reviewer wants
recorded against the task. Each reviewer body
(`resources/modules/personas/reviewer-<persona>.md`) is edited
to direct the persona to return the verdict via its final message
and to explicitly forbid editing TASKS.md from inside the
subagent.

The `/speccy-review` orchestrator
(`.claude/skills/speccy-review/SKILL.md` and
`.agents/skills/speccy-review/SKILL.md`) is the sole writer to
TASKS.md for review-induced state transitions. The orchestrator
fans out to the reviewer subagents (four by default — business,
tests, security, style — with architecture and docs as
explicit-invoke additions), collects their return messages,
consolidates the verdicts, and applies state transitions to
TASKS.md serially: flipping a task from `in-review` to
`completed` when every spawned reviewer passes, or to `pending`
with a consolidated `<retry>` body when any spawned reviewer
fails. The orchestrator body is updated under
`resources/modules/skills/speccy-review.md` (the shared source
for both host skills) to describe this consolidation contract.

<done-when>
- Each `resources/modules/personas/reviewer-<persona>.md` body
  contains an explicit instruction that the reviewer returns its
  verdict in its final message and does not edit TASKS.md.
- `resources/modules/skills/speccy-review.md` contains an
  explicit consolidation contract: parse each spawned reviewer's
  return message, consolidate into a single per-task verdict,
  write the state transition to TASKS.md serially in the
  orchestrator turn.
- No reviewer prompt instructs the persona to write to TASKS.md;
  no reviewer prompt grants the persona the Write or Edit tool
  scope for TASKS.md.
- The orchestrator prompt names TASKS.md as the file it
  exclusively writes for review-induced state transitions.
</done-when>

<behavior>
- Given each reviewer persona body, when read, then it contains
  prose directing the reviewer to emit its verdict as a structured
  final message and explicitly forbidding direct edits to
  TASKS.md.
- Given the orchestrator body, when read, then it contains prose
  describing how to consolidate the spawned reviewers' return
  messages into a single per-task verdict and apply the state
  transition to TASKS.md from the orchestrator turn itself.
- Given a `/speccy-review` invocation against a task in
  `state="in-review"` where the spawned reviewers split (some
  pass, some fail), when the orchestrator finishes, then TASKS.md
  shows the task flipped to `state="pending"` with a single
  consolidated `<retry>` body that aggregates the failing
  reviewers' feedback — not one write per reviewer, not a torn
  file.
- Given a `/speccy-review` invocation where every spawned
  reviewer passes, when the orchestrator finishes, then TASKS.md
  shows the task flipped to `state="completed"` and no `<retry>`
  body is appended.
</behavior>

<scenario id="CHK-009">
Given each of the six reviewer persona files under
`resources/modules/personas/reviewer-<persona>.md`, when read,
then each contains an explicit instruction that the reviewer
returns its verdict via the subagent's final message and an
explicit prohibition against editing TASKS.md from within the
reviewer subagent.

Given `resources/modules/skills/speccy-review.md`, when read,
then it contains a consolidation contract that names the default
four-persona fan-out (business, tests, security, style), the
two explicit-invoke personas (architecture, docs), the
verdict-return shape it expects from each spawned reviewer, and
the serial-write discipline for TASKS.md from the orchestrator
turn.

Given a test harness that runs `/speccy-review` against a task
in `state="in-review"` with the default four-persona fan-out
and a deterministic reviewer mock where two reviewers fail and
two pass, when the orchestrator completes, then the post-run
TASKS.md content for that task shows `state="pending"` and
contains exactly one `<retry>` body that aggregates the failing
reviewers' feedback (not two separate `<retry>` elements, not a
partial write).

Given the same harness with the default four-persona fan-out
and all four spawned reviewers passing, when the orchestrator
completes, then the post-run TASKS.md content for that task
shows `state="completed"` and no `<retry>` body is present.
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: Per-phase SKILL.md bodies are thin stubs; `speccy-init` agent is dropped; shared phase bodies move to `resources/modules/phases/`

The three pinned phase-worker SKILL.md rendered files
(`.claude/skills/speccy-<phase>/SKILL.md` and
`.agents/skills/speccy-<phase>/SKILL.md` for `phase` in
{`tasks`, `work`, `ship`}) carry thin-stub bodies that defer
to the matching subagent. The stub body names the matching
agent file (`.claude/agents/speccy-<phase>.md` for Claude Code,
`.codex/agents/speccy-<phase>.toml` for Codex) as the canonical
procedure source and the `/agent speccy-<phase>` invocation as
the pinned execution path. The agent file's body is the single
on-disk source of truth for the phase's implementation
procedure; the SKILL.md no longer carries the procedural
`When to use` / `Steps` sections.

The `speccy-init` agent file (`.claude/agents/speccy-init.md`)
and its template source
(`resources/agents/.claude/agents/speccy-init.md.tmpl`) are
removed. `/speccy-init`'s SKILL.md remains full-body. The
interactive 7-question Q&A that composes the project's
`## Product north star` section is parent-session work; a
subagent context provides nothing in return for the
ejected-file maintenance cost, and there is no pinned tier to
opt into (per the third Changelog row / DEC-001). No Codex
`.codex/agents/speccy-init.toml` is created either — T-004's
TOML scaffolding scope shrinks to three pinned phases.

The shared phase-body sources move from
`resources/modules/skills/speccy-<phase>.md` to
`resources/modules/phases/speccy-<phase>.md` (Speccy-workflow
naming, not host-harness naming). All `{% include %}`
directives across `resources/agents/` are updated to the new
path. The four shared-body files (one per mechanical phase,
including `init`) move together; the three matching pinned
agent templates include the renamed module while the four
SKILL.md template bodies no longer include any shared module
(their bodies are hardcoded stub prose).

The three remaining phase-worker agent `description:` values
(on `.claude/agents/speccy-tasks.md`,
`.claude/agents/speccy-work.md`, and
`.claude/agents/speccy-ship.md`, plus the matching
`.tmpl` template sources) are rewritten to drop both the
literal substring `` via `context: fork` `` and any reference
to specific model or effort tier values. The description names
what the agent does and the invocation surface only; model and
effort are declared in their own frontmatter keys and need not
repeat in description prose.

`/speccy-review` is explicitly excluded from this contract.
Its SKILL.md remains full-body per REQ-002 and REQ-009 — the
orchestrator's consolidation work runs in the parent session
at the parent session's model and the body carries the
verdict-consolidation contract directly.

<done-when>
- Each of the six rendered SKILL.md files at
  `.claude/skills/speccy-<phase>/SKILL.md` and
  `.agents/skills/speccy-<phase>/SKILL.md` for `phase` in
  {`tasks`, `work`, `ship`} has body content of bounded short
  length (≤10 non-blank lines below the YAML frontmatter
  delimiter) and names both the matching agent file path and
  the `/agent speccy-<phase>` invocation pattern.
- Each of the three Claude Code agent files
  (`.claude/agents/speccy-tasks.md`,
  `.claude/agents/speccy-work.md`,
  `.claude/agents/speccy-ship.md`) and each of the three Codex
  TOML files (`.codex/agents/speccy-tasks.toml`,
  `.codex/agents/speccy-work.toml`,
  `.codex/agents/speccy-ship.toml`) includes the renamed
  shared module at `resources/modules/phases/speccy-<phase>.md`
  via MiniJinja `{% include %}` in its templated source.
- The four shared body files at
  `resources/modules/phases/speccy-<phase>.md` for `phase` in
  {`tasks`, `work`, `ship`, `init`} exist.
- No files exist at
  `resources/modules/skills/speccy-<phase>.md` for the same
  four phase names.
- `.claude/agents/speccy-init.md`,
  `resources/agents/.claude/agents/speccy-init.md.tmpl`, and
  `.codex/agents/speccy-init.toml` do not exist.
- The three remaining Claude Code agent file `description:`
  values do not contain the literal substring
  `` via `context: fork` `` and do not name specific model
  tiers (no occurrences of `Sonnet`, `Opus`, `Haiku` outside
  code fences) or effort levels (no occurrences of `xhigh`,
  `medium`, `high`, `low`, `max` outside code fences).
- A pure-Rust meta-test under `speccy-core/tests/` exists that
  asserts the stub-shape invariants and exits 0 against the
  post-T-009 working tree.
- `/speccy-review`'s SKILL.md body remains the full
  verdict-consolidation contract per REQ-002 and REQ-009;
  no stub edit is applied to it.
</done-when>

<behavior>
- Given each of the three pinned SKILL.md rendered files
  (`.claude/skills/speccy-<phase>/SKILL.md` for `phase` in
  {`tasks`, `work`, `ship`}), when its body is parsed, then it
  contains the literal substring `/agent speccy-<phase>` with
  the matching phase name and a reference to the
  `.claude/agents/speccy-<phase>.md` file path.
- Given each of the three Codex SKILL.md rendered files
  (`.agents/skills/speccy-<phase>/SKILL.md` for the same
  phases), when its body is parsed, then it contains the
  literal substring `/agent speccy-<phase>` and a reference
  to the `.codex/agents/speccy-<phase>.toml` file path.
- Given each of the six pinned SKILL.md rendered files, when
  diffed against the matching agent file body, then the
  SKILL.md body byte-length is strictly less than the agent
  body byte-length and the SKILL.md body does not contain the
  literal substrings `## Steps` or `## When to use`.
- Given each agent file at `.claude/agents/speccy-<phase>.md`
  for `phase` in {`tasks`, `work`, `ship`}, when its
  frontmatter is parsed, then its `description:` value does
  not contain `` via `context: fork` `` and does not name a
  specific model or effort tier outside a code fence.
- Given the post-amendment `.claude/agents/` directory, when
  listed, then no file `speccy-init.md` exists.
- Given the post-amendment `.codex/agents/` directory, when
  listed, then no file `speccy-init.toml` exists.
- Given the post-amendment `resources/modules/` directory,
  when listed, then it contains a `phases/` subdirectory with
  four `.md` files (`speccy-tasks.md`, `speccy-work.md`,
  `speccy-ship.md`, `speccy-init.md`) and contains no
  `skills/speccy-<phase>.md` file for the same four phases.
- Given `/speccy-review`'s SKILL.md, when parsed, then it
  carries the full verdict-consolidation contract per REQ-002
  and REQ-009 (no stub edit applies).
</behavior>

<scenario id="CHK-010">
Given each of the three rendered SKILL.md files at
`.claude/skills/speccy-tasks/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`, and
`.claude/skills/speccy-ship/SKILL.md` after this SPEC's tasks
land, when each is read, then each has ≤10 non-blank content
lines below the YAML frontmatter delimiter, contains the
literal substring `/agent speccy-<phase>` for its matching
phase, and contains a reference to its matching agent file
path at `.claude/agents/speccy-<phase>.md`.

Given the three matching `.agents/skills/speccy-<phase>/SKILL.md`
rendered files for the Codex host (same three phases), when
each is read, then it has the same stub-shape invariants as
its Claude Code counterpart but references its matching
`.codex/agents/speccy-<phase>.toml` as the procedure source.

Given each of the six pinned SKILL.md rendered files, when
each is byte-compared against its matching agent file body,
then the SKILL.md body byte-length is strictly less than the
agent body byte-length and the SKILL.md body does not
contain the literal substrings `## Steps` or `## When to use`.

Given each of the three Claude Code pinned agent files at
`.claude/agents/speccy-tasks.md`,
`.claude/agents/speccy-work.md`, and
`.claude/agents/speccy-ship.md`, when each frontmatter
`description:` field is read, then it does not contain the
literal substring `` via `context: fork` `` and does not name
the literal substrings `Sonnet`, `Opus`, `Haiku`, `xhigh`,
`high`, `medium`, `low`, or `max` outside code fences.

Given the post-SPEC `.claude/agents/` directory listing, when
scanned, then no file named `speccy-init.md` exists. The
matching template under
`resources/agents/.claude/agents/speccy-init.md.tmpl` also
does not exist.

Given the post-SPEC `.codex/agents/` directory listing, when
scanned, then no file named `speccy-init.toml` exists.

Given the post-SPEC `resources/modules/` directory listing,
when read, then it contains `phases/`, `personas/`,
`prompts/`, and (per SPEC-0031) `examples/` subdirectories.
The `phases/` subdirectory contains exactly four `.md` files
(`speccy-tasks.md`, `speccy-work.md`, `speccy-ship.md`,
`speccy-init.md`). The directory `resources/modules/skills/`
contains no `speccy-<phase>.md` files for the four phase
workers (implementation may drop the directory if it becomes
empty post-rename or leave it for any reviewer/orchestrator
shared bodies that remain there per REQ-009).

Given a pure-Rust meta-test under `speccy-core/tests/` that
scans the rendered host pack files and asserts the stub-shape
invariants above, when run against the post-T-009 working
tree, then it exits 0.

Given `.claude/skills/speccy-review/SKILL.md` and
`.agents/skills/speccy-review/SKILL.md` after this SPEC's
tasks land, when each is read, then each carries the full
verdict-consolidation body per REQ-002 and REQ-009 (no
stub-shape transformation applies).
</scenario>

</requirement>

## Design

### Approach

The change concentrates in three layers: frontmatter edits on
existing shipped files (6 Claude Code reviewer agent frontmatter
additions for `model:` + `effort:`; 6 Codex reviewer TOML
frontmatter additions for `model` + `model_reasoning_effort`;
4 phase-worker shared-skill-body one-line invocation pointer
additions under `resources/modules/skills/` that render into
both hosts; reviewer-persona body edits per REQ-009 to direct
the verdict-return contract; and `speccy-review` shared body
edits per REQ-009 to describe the consolidation contract), new
files (4 Claude Code phase-worker agent files; 4 Codex
phase-worker TOML files; all pinned per REQ-001 / REQ-004), and
prose (README pinning section, README drift audit). The four
`.claude/skills/speccy-<phase>/SKILL.md` files themselves gain
no frontmatter additions — slash-command invocation runs in the
parent session per DEC-001. The templating layer under
`resources/agents/` mirrors all the file-shape changes so
`speccy init` renders the same shape in user projects. No code
paths inside `speccy-cli` or `speccy-core` change.

Implementation order:

1. Land the four new Claude Code phase-worker agent files for
   `tasks`, `work`, `ship`, `init` (three pinned at
   `sonnet[1m]`/medium; `init` left without a `model:` field).
   Templates and rendered outputs both. No frontmatter edits to
   the four `.claude/skills/speccy-<phase>/SKILL.md` files.
   (REQ-001, REQ-008.)
2. Land the six Claude Code reviewer frontmatter edits with the
   `[1m]` 1M-context suffix and the asymmetric effort tiers.
   (REQ-003.)
3. Land the F-10 absorption edits to reviewer-persona bodies
   (verdict-return contract; no TASKS.md writes from inside the
   reviewer subagent) and to the `speccy-review` shared body
   (consolidation contract; sole-writer-to-TASKS.md discipline).
   (REQ-009.)
4. Land the four new Codex phase-worker TOML files (pinned at
   `gpt-5.5` / medium for `tasks`/`work`/`ship`; `speccy-init`
   left without `model`/`model_reasoning_effort`), the six Codex
   reviewer TOML frontmatter edits (pinned at `gpt-5.5` with
   per-persona `model_reasoning_effort` tiers), and the four
   one-line `/agent speccy-<phase>` invocation-pointer additions
   to the shared phase-worker skill body sources under
   `resources/modules/skills/` (the same edit renders into both
   the Claude Code `.claude/skills/speccy-<phase>/SKILL.md` and
   the Codex `.agents/skills/speccy-<phase>/SKILL.md` outputs).
   (REQ-001, REQ-004, REQ-008.)
5. Validate pin shape across all files (no long-form versioned
   IDs; no Haiku anywhere; Claude pins are `^(opus|sonnet)\[1m\]$`
   with valid effort levels; Codex pins are `gpt-5.5` with
   `model_reasoning_effort` in `low`/`medium`/`high`/`xhigh`). (REQ-005.)
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
#### DEC-001: Mechanical-phase pin lives in opt-in subagent files, not auto-fork SKILL.md frontmatter

**Context:** Claude Code skills support both a direct
`model:` / `effort:` pin (applies for the current assistant turn
only) and a `context: fork` + `agent:` pin (applies across the
forked subagent's full execution, multiple internal turns, in an
isolated context). An earlier draft of this SPEC chose
`context: fork` on the four mechanical-phase SKILL.md files as
the mechanism for sustaining the cost-and-time pin across a
multi-turn phase. T-001 implemented that wiring; the first real
`/speccy-work` invocation against T-002 surfaced the
disqualifying UX cost: a forked subagent's intermediate tool
output stays inside its isolated context by design, and on a
phase that touches a dozen files plus four hygiene gates the
parent session shows minutes of dead air with no progress
signal. The user could not tell whether the subagent was alive,
broken, or making progress, and could not interrupt sensibly
because the parent session was simply waiting. The
cost-and-time win is real but the silence cost on multi-minute
phase work is worse.

**Decision:** The cost-and-time pin lives in the
`.claude/agents/speccy-<phase>.md` subagent files (and the
matching Codex TOML files). Users opt into the pin by invoking
the subagent explicitly via the host's subagent surface
(`Agent` tool / `/agent speccy-<phase>`). The
`.claude/skills/speccy-<phase>/SKILL.md` files carry no
`context:`, `agent:`, `model:`, or `effort:` frontmatter; the
slash-command invocation runs in the parent session at the
parent session's model with full streaming visibility into tool
output. A one-line invocation pointer at the top of each
phase-worker skill body (rendered from the shared source per
DEC-007) makes the opt-in path discoverable from the slash
command itself.

**Alternatives:**

- Keep `context: fork` and accept the silent UX (rejected: the
  user explicitly experienced this on T-002 and the silence
  produced a worse outcome than the cost saving — the subagent
  did the work but the user could not tell, retreated, and the
  task was left in a half-state requiring manual recovery).
- Direct `model:` + `effort:` on every mechanical-phase SKILL.md
  with no fork (rejected: single-turn pin scope means the override
  reverts after the first response; the user effectively cannot
  rely on the pin holding across the multi-turn work the phase
  needs).
- Skill body dispatches to a subagent via explicit Task-tool
  invocation in the body prose (rejected: requires writing
  dispatch logic into every skill body; the same opt-in shape is
  cleaner when the user controls the dispatch).
- Document-as-recommendation only with no shipped subagent files
  (rejected: cost-and-time win would require the user to remember
  to switch models before every invocation, violating the
  friction-to-skill-update principle from AGENTS.md; shipping the
  pinned subagent files is the right friction-reducing layer).

**Consequences:** Each pinned phase still ships two files (the
slash-command skill and the subagent definition) on Claude Code,
plus the equivalent agent TOML on Codex — the shipped surface
area is unchanged, only the auto-fork wiring is dropped. Users
who want the cost-and-time pin must invoke `/agent
speccy-<phase>` before the phase command; users who want full
streaming visibility in the parent session can run the slash
command directly. Both hosts now expose symmetric opt-in
shapes; see DEC-006 for the resulting reframing of the prior
host-asymmetry decision.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: speccy-review stays unpinned because the orchestrator owns TASKS.md writes

**Context:** `/speccy-review` is the orchestrator that fans out
to the reviewer subagents via the Task tool (four personas by
default — business, tests, security, style — with
`reviewer-architecture` and `reviewer-docs` as explicit-invoke
additions). Claude Code's documented rule is that subagents
cannot spawn other subagents, so `/speccy-review` cannot itself
become a forked subagent without breaking the fan-out
mechanism it depends on. An earlier draft of this SPEC pinned
the orchestrator to Haiku via direct skill frontmatter on the
theory that the orchestrator turn was pure JSON parsing and
dispatch — small enough work for the cheapest tier. The F-10 absorption (REQ-009) invalidates that
theory: the orchestrator is now the sole writer to TASKS.md for
review-induced state transitions, parsing every spawned
reviewer's return message and consolidating them into
per-task verdicts. That work is not pure dispatch and risks
truncation on a Haiku-tier context window. Codex compounds the
problem: Codex skills cannot pin a model at all, so any Codex
parity would have to live in a `.codex/agents/speccy-review.toml`
subagent file, and pinning that file to a tier below the user's
session model would silently downgrade the consolidation work.

**Decision:** `/speccy-review` carries no `model:`, `effort:`,
`context:`, or `agent:` frontmatter on either host. The skill
runs in the parent session at whatever model the user opened the
session with. No `.codex/agents/speccy-review.toml` file is
created. The reviewer fan-out via Task tool is preserved.

**Alternatives:**

- Pin `/speccy-review` to Haiku via direct skill frontmatter
  (rejected: the F-10 consolidation work needs more context than
  Haiku 4.5 offers; the work is no longer pure dispatch).
- Pin `/speccy-review` to Sonnet via direct skill frontmatter
  (rejected: single-turn pin scope means the parent session
  inherits a polluted context after the orchestrator turn anyway,
  and the user may already be running on Opus for the rest of
  the loop — forcing a downgrade for one turn is the wrong
  default. Inheriting session gives the user control via
  `/model` if they want a cheaper tier).
- Ship a `.codex/agents/speccy-review.toml` pinned to `gpt-5.5`
  with `model_reasoning_effort = "medium"` (rejected: same
  silent-downgrade risk as the Haiku pin, plus it would force
  Codex users to invoke via `/agent` to keep the contract their
  Claude Code peers get from inheriting session).
- Fork `/speccy-review` to a `speccy-review` subagent that owns
  the fan-out (rejected: subagents cannot spawn subagents on
  Claude Code; the fan-out would break).

**Consequences:** Users running their loop at Opus pay Opus rates
for the orchestrator's consolidation turn. That cost is
acceptable because the consolidation is load-bearing for
TASKS.md correctness under REQ-009. Users who want a cheaper
orchestrator turn can `/model sonnet` before invoking
`/speccy-review` — the skill respects the session model.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Use floating aliases with the `[1m]` selector on Claude Code and `gpt-5.5` on Codex

**Context:** Claude Code accepts Anthropic aliases (`opus`,
`sonnet`, `haiku`), the same aliases with the `[1m]`
1M-context-window selector (`opus[1m]`, `sonnet[1m]`), and full
versioned model IDs (`claude-opus-4-7`, `claude-opus-4-7[1m]`,
etc.) in `model:` frontmatter. Codex accepts OpenAI model
identifiers (`gpt-5.5` as the current generation, dated snapshots
like `gpt-5.5-2026-04-15`, etc.). The pin shape choice on each
host trades reproducibility against forward motion as new
generations ship, and the choice of context-window variant on
Claude Code trades subagent headroom against (small) per-turn
cost.

**Decision:** Claude Code pins use the alias with the `[1m]`
1M-context-window suffix (`opus[1m]`, `sonnet[1m]`). Codex pins
use the bare current-generation identifier (`gpt-5.5`). Both
forms float forward as their vendors ship new generations. Long-
form versioned snapshot IDs do not appear in any shipped file.

**Alternatives:**

- Use long-form versioned IDs everywhere (rejected: every vendor
  model-ship cycle would require a Speccy amendment to bump every
  pin, trading drift against reproducibility in a way that favors
  invariance over forward motion).
- Use bare aliases without the `[1m]` selector on Claude Code
  (rejected: phase workers routinely read TASKS.md plus SPEC.md
  plus multiple modules in one pass, and reviewers read full SPEC
  plus diff plus task body; the default 200K window risks
  truncation on non-trivial repos and the 1M variant headroom is
  cheap insurance).
- Split alias policy across reviewer pins vs mechanical pins
  (rejected: introduces a split policy that future amendments
  must reason about; the simpler invariant is "always aliases,
  always editable").
- Use a Codex-equivalent context-window selector (rejected:
  Codex does not expose one on its model identifier as of this
  SPEC).

**Consequences:** Speccy's pins float forward as Anthropic and
OpenAI ship new generations, which means a user dogfooding on a
stable project may see behavior changes when a new generation
ships. The mitigation is editability: ejected files are plain
text and a user who wants reproducibility can swap aliases for
long-form snapshot IDs file-by-file post-`speccy init`. The
`[1m]` selector locks Claude Code pins to the 1M-context-window
variant of whatever generation the alias resolves to; if a future
generation drops the 1M variant, the alias plus suffix may fail
to resolve and a future amendment will revisit.
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
#### DEC-006: Both hosts use symmetric opt-in subagent invocation; no workarounds

**Context:** Codex CLI exposes per-subagent `model` and
`model_reasoning_effort` in TOML files, but Codex skills do not
accept a model field, and Codex does not expose a
Claude-Code-equivalent `context: fork` primitive. Subagent
spawning on Codex is user-initiated ("Codex only spawns
subagents when you explicitly ask it to") rather than
programmatic. An earlier draft of this SPEC accepted that
asymmetry and shipped auto-fork on Claude Code (`context: fork`
on the four phase-worker SKILL.md files) plus opt-in on Codex
(`/agent <name>` pointer). The auto-fork half of that
asymmetry was abandoned mid-implementation; see DEC-001 for the
silent-UX reasoning. The result is that both hosts now expose
the same opt-in shape.

**Decision:** The cost-and-time win is opt-in on both hosts: the
user invokes `/agent speccy-<phase>` (or the host's equivalent
subagent surface) to activate the pinned agent, then issues the
phase command. Speccy ships four phase-worker subagent files on
each host (three pinned, one unpinned `speccy-init` on each),
ships a one-line invocation pointer in the shared phase-worker
skill body so users discover the invocation path on either host,
and documents the opt-in surface under the new README pinning
section. `/speccy-review` ships no agent file on either host
(see DEC-002) because pinning the orchestrator below the user's
session model would silently downgrade the REQ-009 consolidation
work; users invoke `/speccy-review` as a regular skill and
inherit their session model. No workaround code, no programmatic
dispatch, no shimming.

**Alternatives:**

- Reintroduce `context: fork` on Claude Code so the Claude Code
  side auto-pins (rejected: the silent multi-minute UX on
  `/speccy-work` was the immediate trigger for retreating from
  auto-fork; no host-side primitive change has made that
  silence acceptable).
- File a backlog entry tracking Codex CLI's eventual ship of a
  programmatic dispatch primitive (rejected by the user during
  brainstorming: documenting the opt-in path in the README is
  sufficient; a backlog entry would be a TODO without an action
  Speccy can take).
- Ship a Speccy-side workaround that wraps Codex's natural-language
  agent invocation in a deterministic interface (rejected: out
  of scope and would couple Speccy to Codex internals).

**Consequences:** Users on both hosts get the pin assignment
table and must remember to invoke `/agent` before running a
phase if they want the pin. The one-line pointer at the top of
each phase-worker skill body is the discovery mechanism for
users who haven't read the README. If a future host CLI ships a
silent-but-progress-streaming fork primitive that solves the UX
cost DEC-001 retreated from, a future amendment can revisit;
Speccy does not block on upstream changes.
</decision>

<decision id="DEC-007" status="accepted">
#### DEC-007: Single-source phase body via resources/modules/phases/

**Context:** The pinned Claude Code phase-worker subagent files
and the pinned Codex phase-worker TOML files (for `tasks`,
`work`, `ship` on each host) each need a body to drive the
work. Three placement options exist for the source: duplicate
the body verbatim in each new file; create a new
`resources/modules/agents/` directory parallel to
`resources/modules/personas/`; or share a single body source
via templated include.

**Decision:** Phase-worker subagent bodies share
`resources/modules/phases/speccy-<phase>.md` (renamed from the
earlier `resources/modules/skills/` path per DEC-009) via
MiniJinja `{% include %}`. No `resources/modules/agents/`
directory is created. Under DEC-009 the inclusion shape is
asymmetric: agent templates include the shared phase module;
the three matching pinned-phase SKILL.md template bodies are
hardcoded thin-stub prose that defer to the agent file, so the
shared module renders only on the agent side. `/speccy-init`'s
SKILL.md still includes the shared `init` phase module (init
ships no subagent, so the SKILL.md is the only consumer).

**Alternatives:**

- New `resources/modules/agents/` directory for phase workers
  (rejected: introduces a parallel-directory pattern that future
  contributors must reason about; the body content for a phase
  is a phase-level artifact regardless of how it gets delivered,
  so a single phase-named directory is the simpler invariant).
- Duplicate the body verbatim in each new file (rejected:
  doubles maintenance cost for every future phase prompt edit;
  introduces drift risk between skill wrapper and subagent
  wrapper).
- Keep the directory name as `resources/modules/skills/`
  (rejected at DEC-009: the name follows host-harness vocabulary
  but the body is a Speccy-workflow artifact; `phases/` names
  what the module is, not how it gets delivered).

**Consequences:** Edits to phase-worker prompts propagate to
the subagent wrapper (and, for `init`, the skill wrapper) from
a single source file. The reviewer pattern through
`resources/modules/personas/` proves this works; the extension
to phase workers is a straightforward repeat with one structural
difference DEC-009 introduces: SKILL.md templates for the three
pinned phases hardcode their own stub bodies rather than
including the shared phase module, so the duplication the
symmetric-inclusion shape would have rendered on disk is
eliminated by construction.
</decision>

<decision id="DEC-008" status="accepted">
#### DEC-008: Concentrate TASKS.md writes in the review orchestrator (F-10 absorption)

**Context:** The pre-amendment review fan-out had each reviewer
subagent edit TASKS.md directly to record its verdict (pass /
fail with `<retry>` body). The default fan-out is four parallel
reviewers (business, tests, security, style), with two more
(architecture, docs) joining when explicitly invoked — up to six
parallel writers in the worst case. Even four concurrent
subagents writing to the same file is a race condition: the
host runs the subagents concurrently, and there is no host-level
locking on TASKS.md edits. In practice the race may bite rarely
(most runs have one reviewer finish before the next starts to
write), but "rarely" is the wrong invariant for the file that
drives the review loop's state machine.

**Decision:** Reviewer subagents are pure verdict producers.
Each reviewer returns its verdict to the `/speccy-review`
orchestrator via its final message and is explicitly forbidden
from editing TASKS.md. The orchestrator parses each spawned
reviewer's return message, consolidates them into a single
per-task verdict, and writes the state transition to TASKS.md
serially in the orchestrator turn. The race is eliminated by
construction rather than papered over with file locking or
retry-on-conflict logic.

**Alternatives:**

- Keep direct reviewer writes; add file locking (rejected: adds
  host-level coordination machinery for a problem that
  disappears under the orchestrator-as-sole-writer shape;
  locking is a worse invariant than "only one writer exists").
- Keep direct reviewer writes; add a post-merge reconciliation
  pass in the orchestrator (rejected: still leaves a window
  where TASKS.md is in a torn state; the orchestrator's
  reconciliation runs after the writes, not in place of them).
- Move the verdict-consolidation work into a dedicated
  consolidator subagent the orchestrator spawns after the
  reviewers return (rejected: subagents cannot spawn subagents
  on Claude Code, so the orchestrator must do the consolidation
  itself anyway).

**Consequences:** The orchestrator's work shape changes from
pure dispatch to dispatch-plus-consolidation, which is the
direct reason DEC-002 drops the earlier Haiku pin on
`/speccy-review`. Reviewer return messages must be structured
enough that the orchestrator can parse them without ambiguity;
the shared prompt body for `resources/modules/skills/speccy-review.md`
and the per-persona bodies under `resources/modules/personas/`
are updated to define the verdict shape and forbid direct
TASKS.md edits.
</decision>

<decision id="DEC-009" status="accepted">
#### DEC-009: Stub per-phase SKILL.md bodies and drop the `speccy-init` agent (source-of-truth dedup)

**Context:** Under the post-third-Changelog-row design, each
`.claude/skills/speccy-<phase>/SKILL.md` and its matching
`.claude/agents/speccy-<phase>.md` rendered both the same shared
body via `{% include "modules/skills/speccy-<phase>.md" %}`.
The on-disk rendered files were byte-identical bodies, which
gave every ejected user project two files containing the same
procedural prose per phase. The third Changelog row's retreat
from `context: fork` left this duplication intact: the fix
focused on the auto-fork UX cost (silent multi-minute dead
air in the parent session), not the source-of-truth question.
The duplication is wasted ejected-file surface area and
sets up the future multi-agent orchestrator (which will be a
single full-workflow SKILL driving each phase via the
subagent surface) to inherit an ambiguous source-of-truth
contract from day one. Separately, the `speccy-init` agent
file landed in T-001 for symmetry with the three pinned
phase-workers, but `/speccy-init`'s load-bearing work is the
interactive 7-question Q&A that composes `## Product north
star`. Interactive Q&A in a subagent context offers nothing
the parent session doesn't already do, and `speccy-init` has
no pinned tier to opt into (it inherits the parent session's
model per the third Changelog row / DEC-001) — so the agent
file is ceremony for zero value.

**Decision:** Three coupled changes land together. (1) The
three pinned phase-worker SKILL.md bodies (`tasks`, `work`,
`ship`) become thin stubs that defer to their matching
subagent. The stub body names the agent file as the
canonical procedure source and the `/agent speccy-<phase>`
invocation as the pinned execution path. The agent file's
body is the single on-disk source of truth. (2) The
`speccy-init` agent file is removed entirely; its SKILL.md
remains full-body. (3) The shared phase-body sources move
from `resources/modules/skills/` to
`resources/modules/phases/` (Speccy-workflow naming, not
host-harness naming) to reflect what the modules actually
carry (phase-level procedure, not skill-wrapped recipes).

**Alternatives:**

- **Hard defer.** Make `/speccy-<phase>` a discovery-only
  surface that emits "use `/agent speccy-<phase>`" and exits
  without doing the work. Rejected: forces user friction
  every invocation; the parent session reading the agent
  file and following it is a cheaper one-extra-Read-tool-call
  cost. The slash command stays functional.
- **Auto-dispatch via Task tool from inside the skill body.**
  Skill body says "spawn the subagent via Task tool with
  subagent_type=speccy-<phase>." Rejected: mechanically
  equivalent to the third Changelog row's retreated
  `context: fork` — visible Task dispatch instead of silent
  fork, but the same load-bearing problem that the parent
  session can't intervene mid-stream on multi-minute work.
- **Keep the duplication; ship as-is.** Rejected: every
  future phase-prompt edit ships in two ejected files; the
  future multi-agent orchestrator inherits ambiguous
  source-of-truth.
- **Keep `speccy-init`'s agent for symmetry.** Rejected:
  symmetric ceremony for zero functional value. Interactive
  Q&A doesn't benefit from a subagent context, and there is
  no pinned tier to opt into.
- **Rename `resources/modules/skills/` to
  `resources/modules/agents/`.** Rejected: that name follows
  the host-harness vocabulary ("skill" / "agent" are host
  concepts), but the body content is a Speccy-workflow
  artifact (the procedure for one phase of the lifecycle).
  `phases/` names what the module is, not how it gets
  delivered.

**Consequences:** One on-disk source of truth per phase (the
agent file's body). `/speccy-work` invoked directly still
works: the parent session reads
`.claude/agents/speccy-work.md` via the Read tool primitive
and follows the procedure at the parent session's model.
`/agent speccy-work` invoked directly runs the same procedure
at the pinned `sonnet[1m]` / medium tier per the agent file's
frontmatter. T-001 and T-002 closure records remain
immutable; the SKILL.md stub edits and the `speccy-init`
agent deletion land in a new T-009 under Phase 8. T-004's
pointer-prepend step is dropped (the stub IS the pointer);
T-006's pin-shape meta-test gains the stub-shape assertions;
T-007's `speccy init` parity verification updates assertions
to three pinned phase-worker agents per host instead of four;
T-008's README pinning section drops the `speccy-init` agent
row. DEC-007's earlier "no new
`resources/modules/agents/` directory" decision is
generalized: shared phase bodies live at
`resources/modules/phases/` (host-agnostic), which is a
cleaner semantic than the `resources/modules/skills/`
filename it replaces.
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
- Claude Code's `Agent` tool / `/agent <name>` subagent
  invocation surface delivers what the docs promise: invoking
  `/agent speccy-<phase>` activates a subagent that runs at the
  pinned `model:` / `effort:` declared in the agent's
  frontmatter; the skill body becomes the subagent's task prompt
  and the subagent's final return message reaches the parent
  session.
- The pin assignment table is good-enough for v1. Adjustments
  may be needed after dogfooding, but the SPEC ships locked
  assignments rather than a measurement-driven re-pin step
  (per DEC-004).
- The opt-in `/agent <name>` invocation surface is acceptable
  UX for v1 on both hosts. The one-line invocation pointer at
  the top of each phase-worker skill body, plus the README
  pinning section, is sufficient discovery surface; no
  programmatic dispatch is needed (per DEC-001, DEC-006).
- Conversational skills (`brainstorm`, `plan`, `amend`) need no
  pinning for v1; the user opens a session at the latest Opus
  generation at xhigh or max effort, which is the
  contract-writing tier these phases need (per DEC-005).
- The existing CI host-pack drift check covers
  `resources/agents/` paths and will pick up the new phase-worker
  subagent files automatically; no extension to the meta-test is
  needed.
- Phase-worker subagent bodies share the
  `resources/modules/phases/` source (renamed from the earlier
  `resources/modules/skills/` path per DEC-009) via templating;
  no new `resources/modules/agents/` directory is needed (per
  DEC-007 / DEC-009). The inclusion shape is asymmetric: the
  agent templates include the shared phase module, but the
  three matching pinned-phase SKILL.md template bodies are
  hardcoded thin-stub prose that defer to the agent file.
- `/speccy-review`'s main-session execution with no frontmatter
  pin preserves the existing reviewer fan-out via Task tool and
  gives the orchestrator the parent session's full context
  capacity for REQ-009 consolidation work (per DEC-002).
- Anthropic's model alias resolution (`opus` → current Opus
  generation, `sonnet` → current Sonnet generation) and the
  `[1m]` 1M-context-window selector are stable; Anthropic does
  not silently swap aliases to point at downgraded models and
  does not retire the `[1m]` variant of a generation without a
  successor. If either assumption is violated, future amendments
  can swap aliases for long-form snapshot IDs file-by-file.
- OpenAI's `gpt-5.5` alias resolves to the current GPT-5.5
  generation stably. If OpenAI retires `gpt-5.5` without a
  forward-compatible successor alias, a future amendment swaps
  the Codex pins file-by-file.
- Reviewer subagents can return verdicts as their final message
  in a shape structured enough that the orchestrator can parse
  the spawned reviewers' returns (four for the default fan-out,
  up to six with explicit-invoke additions) and consolidate
  them into a single per-task decision without ambiguity. The
  shared prompt body for reviewers under
  `resources/modules/personas/` and the orchestrator body under
  `resources/modules/skills/speccy-review.md` define that shape;
  non-conforming reviewer output is treated as a fail-closed
  signal that the orchestrator surfaces back to the user rather
  than silently dropping (per DEC-008, REQ-009).
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
| 2026-05-19 | agent/claude-2  | Three coupled amendments: (1) strip Haiku entirely — speccy-init agent and speccy-review skill drop the `model:` field, inherit parent session model (mirrors Codex's no-pin-on-skills contract; sidesteps Haiku 4.5 context-window risk); (2) absorb F-10 from BACKLOG.md — reviewer fan-out returns verdicts as final messages, the orchestrator becomes sole writer to TASKS.md, eliminating the parallel-write race by construction (new REQ-009, new DEC-008); (3) tighten alias shape — Claude Code pins use the `[1m]` 1M-context-window suffix (`opus[1m]` / `sonnet[1m]`), Codex pins use the GPT-5.5 alias (`gpt-5.5`) with `model_reasoning_effort` carrying the tier. The three changes are coupled: dropping Haiku on /speccy-review is only safe because F-10 promoted the orchestrator from pure dispatch to TASKS.md-writing consolidation, and the `[1m]` suffix supplies the context headroom that consolidation work needs. Also corrected fan-out count throughout: default fan-out is four (business, tests, security, style); architecture and docs are explicit-invoke additions. Reviewer persona file count remains six. F-10 entry removed from .speccy/BACKLOG.md as part of the same change. |
| 2026-05-19 | agent/claude-3  | Drop `context: fork` as the Claude Code mechanical-phase pin mechanism. Discovered mid-implementation (between T-001 in-review and T-002 in-progress): the auto-fork pattern hides the subagent's tool output from the parent session by design, and on multi-minute phase work that produces minutes of dead air in the parent session with no progress signal, no way to tell if the subagent is alive, broken, or making progress. Single-shot phases tolerate the silence; `/speccy-work` does not. The cost-and-time pin is preserved as an opt-in path: the four `.claude/agents/speccy-<phase>.md` subagent files still ship pinned at `sonnet[1m]`/medium (three of them; `speccy-init` unpinned), invocable via the host's `Agent` / `/agent <name>` surface. The four `/speccy-<phase>` slash commands run in the parent session by default with full streaming visibility. The Claude Code / Codex asymmetry described in the prior draft collapses: both hosts now expose opt-in subagent invocation; neither auto-forks. DEC-001 flips its decision; DEC-006 reworks the asymmetry framing into a symmetry framing; REQ-001 / REQ-006 / REQ-007 / CHK-001 / CHK-006 update to drop the `context: fork` / `agent:` mandates from SKILL.md. T-001 (in-review) is partially reverted (agent files kept; SKILL.md edits dropped); T-002 (reviewer pinning, in-progress) is unaffected and continues. |
| 2026-05-19 | agent/claude-4  | Stub per-phase SKILL.md bodies and drop the `speccy-init` agent. The three pinned phase-worker SKILL.md bodies (`tasks`, `work`, `ship`) become thin stubs that defer to their matching subagent at `.claude/agents/speccy-<phase>.md` (Claude Code) or `.codex/agents/speccy-<phase>.toml` (Codex); the agent file body becomes the single on-disk source of truth for each phase's implementation procedure. `speccy-init`'s agent file is removed entirely — its load-bearing work is the interactive 7-question Q&A that composes the project's `## Product north star` section, which is parent-session work; no subagent context is appropriate and there is no pinned tier to opt into. The shared phase-body sources move from `resources/modules/skills/` to `resources/modules/phases/` (Speccy-workflow naming, not host-harness naming). The three remaining phase-worker agent `description:` values are rewritten to drop `via context: fork` and to drop references to specific model/effort tier values (those live in their own frontmatter keys, not in description prose). A new REQ-010 + DEC-009 capture the contract. T-004's pointer-prepend step is dropped (the stub IS the pointer); a new T-009 under Phase 8 implements the dedup. `/speccy-review` is explicitly excluded — orchestrator stays full-body per REQ-002 and REQ-009. |

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
  with worker output. Rejected because cumulative main-session
  pollution across a multi-phase loop defeats the cost-and-time
  benefit and the future autonomous orchestrator inherits a
  bloated context every invocation. (See DEC-001.) An earlier
  draft of this SPEC kept the direct-pin pattern for
  `/speccy-review` only; the F-10 absorption (REQ-009, DEC-008)
  removed that exception by promoting the orchestrator from pure
  dispatch to TASKS.md-writing consolidation, which no longer
  fits a downgraded tier.

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

- Claude Code subagents support `model:` (alias or full ID, with
  optional `[1m]` 1M-context-window selector) and `effort:`
  frontmatter; `context: fork` + `agent:` makes a skill execute
  in a forked subagent. (Claude Code docs.)
- Codex subagents (`.codex/agents/*.toml`) support `model` and
  `model_reasoning_effort` in TOML; Codex skills do not. Codex
  does not expose a context-window selector on its model
  identifier. (OpenAI Codex docs.)
- Claude Code subagents cannot spawn other subagents. (Claude
  Code subagent docs.)
- Opus 4.7 effort levels: `low`, `medium`, `high`, `xhigh`,
  `max`. Sonnet 4.6 effort levels: `low`, `medium`, `high`,
  `max` (no `xhigh`). Claude Code accepts the `[1m]`
  1M-context-window suffix on Opus and Sonnet aliases.
- Codex `model_reasoning_effort` accepts `low`, `medium`, `high`,
  and `xhigh` as tier values; the work-shape tier of each persona
  maps to one of those four on the Codex side. SPEC-0032's shipped
  reviewer pins use `low`/`medium`/`high` only; `xhigh` is the
  forward-compatible headroom for future heavier reviewer work.
- The F-10 absorption removes the assumption (held by an earlier
  draft of this SPEC) that the `/speccy-review` orchestrator
  turn is pure JSON dispatch. Under REQ-009 it is the sole
  writer to TASKS.md for review-induced state transitions, which
  is load-bearing work the orchestrator must run at parent
  session capacity.

The next step after this SPEC's draft lands is
`/speccy-tasks SPEC-0032` to decompose the SPEC into a checklist
of agent-sized tasks in TASKS.md.
