---
id: SPEC-0025
slug: planner-brainstorming-phase
title: Brainstorming skill for atomizing intent before SPEC creation
status: implemented
created: 2026-05-17
supersedes: []
---

# SPEC-0025: Brainstorming skill for atomizing intent before SPEC creation

## Summary

Today the loop from "fuzzy idea" to "written SPEC" is one step:
the user invokes `/speccy-plan` (or, internally,
`speccy plan`) and the agent writes `SPEC.md` in a single pass. There
is no structured pause where the agent breaks the user's ask down
into atomic, first-principles requirements before it commits to
`<requirement>` blocks. Catching framing drift after the SPEC lands
is expensive — an amendment, a Changelog row, a tasks reconciliation,
and (if implementation has started) the in-flight task list to
re-justify. This is the cheapest point in the loop to catch intent
drift, and today the loop does not use it.

This spec adds a standalone shipped skill, `/speccy-brainstorm`,
that runs **before** `/speccy-plan`. It walks the user through a
Socratic clarification: restate the ask, atomize it into
first-principle requirements, surface 2-3 alternative framings with
trade-offs, list the open questions whose answers would change the
SPEC shape, and stop and wait for the user to confirm or redirect.
Only after the user approves the framing does the agent invoke
`/speccy-plan` to write `SPEC.md`. Brainstorming is generic across
both new-spec and amendment paths — though in practice the
amendment path rarely needs it, since the framing is already locked
in by the existing SPEC.

Inspiration: [obra/superpowers brainstorming
skill](https://github.com/obra/superpowers/blob/main/skills/brainstorming/SKILL.md).
Speccy adopts the "explore → questions one at a time → 2-3 approaches
→ design → user-approval hard gate → invoke writing skill" shape,
trimmed to Speccy's stay-small principles: no new CLI command, no new
artifact on disk, no automated tooling beyond prose. The brainstorm
output is ephemeral chat; salient outputs flow into SPEC.md when the
agent runs `/speccy-plan` next.

This is a skill-only change. The Rust CLI surface, the artifact
grammars (SPEC.md / TASKS.md / REPORT.md), the prompt templates
(`plan-greenfield.md` / `plan-amend.md`), and the renderer's
template-variable set are all unchanged. The CLI does not enforce the
brainstorm step; per Principle 1 ("feedback, not enforcement") and
Principle 2 ("deterministic core, intelligent edges"), the skill
instructs and the host agent honors.

## Goals

<goals>
- A new shipped skill `speccy-brainstorm` lives at
  `resources/modules/skills/speccy-brainstorm.md` and is host-wrapped
  for both Claude Code (`.claude/skills/speccy-brainstorm/SKILL.md`)
  and Codex (`.agents/skills/speccy-brainstorm/SKILL.md`) via the
  existing `resources/agents/.../skills/<verb>/SKILL.md.tmpl`
  delegation pattern.
- The skill body teaches a Socratic flow: explore project context;
  ask clarifying questions one at a time; propose 2-3 alternative
  framings with trade-offs; restate the user's ask as atomic,
  first-principle requirements; list the open questions whose
  answers would change the SPEC shape; stop and wait for user
  confirmation before invoking `/speccy-plan`.
- The skill body carries an explicit hard-gate instruction:
  "do not invoke `/speccy-plan` and do not write SPEC.md until the
  user has approved the framing." The gate is prose-marked, not
  CLI-enforced.
- The skill describes brainstorm outputs as ephemeral chat (no disk
  artifact). When the agent next invokes `/speccy-plan`, the salient
  outputs are folded into SPEC.md via the PRD template's existing
  sections — `## Summary` (restated ask), `## Assumptions` (silent
  assumptions surfaced during brainstorm), `## Open Questions`
  (questions still outstanding), `## Notes` (rejected alternative
  framings), and `### Decisions` / `<decision>` blocks (when a
  trade-off is load-bearing).
- `resources/modules/skills/speccy-plan.md` is updated to reference
  `/speccy-brainstorm` as an optional precursor for fuzzy asks. The
  existing two paths (new-spec form, amendment form) continue to
  work without invoking brainstorm first; the brainstorm is a
  recommendation, not a requirement. The stale "the prompt inlines
  `AGENTS.md`" / "inlines the nearest parent `MISSION.md`" wording
  (rotten since SPEC-0023 REQ-005 / REQ-006) is corrected in the
  same edit.
- The shipped-skills test corpus (`speccy-cli/tests/skill_packs.rs`,
  `speccy-cli/tests/init.rs`) is updated to include
  `speccy-brainstorm` in its enumeration so the new skill is
  scaffolded by `speccy init` and shipped through both host packs.
- The `speccy-brainstorm` skill does not depend on any new CLI
  command. No new noun, no new `speccy` subcommand. The skill body
  is markdown only.
</goals>

## Non-goals

<non-goals>
- No CLI surface change. No new `speccy brainstorm` command, no new
  flag, no new template variable. The Rust CLI binary's behavior is
  unchanged; only the shipped-skill bundle gains one file.
- No enforcement. The CLI does not parse brainstorm output, does
  not verify the agent paused, and does not refuse to proceed to
  `/speccy-plan` without a brainstorm. Per Principle 1 (feedback,
  not enforcement) and Principle 2 (deterministic core, intelligent
  edges).
- No new SPEC.md section. Brainstorm output flows into the PRD
  template's existing sections (`## Summary`, `## Assumptions`,
  `## Open Questions`, `## Notes`, `### Decisions`). Inventing a
  `## Brainstorm` section would be procedural ceremony with no
  downstream consumer; same reasoning as the rejected
  `## Brainstorm` section in the original DEC-002 of this spec.
- No structured / machine-readable brainstorm artifact on disk. The
  brainstorm dialogue happens in the agent-user chat and disappears
  after the user invokes `/speccy-plan`. Salient outputs are
  captured in SPEC.md; the rest is conversational scaffolding. This
  is the deliberate departure from obra/superpowers, which writes
  a dated design doc to `docs/superpowers/specs/`.
- No change to `plan-greenfield.md` or `plan-amend.md`. The
  rendered planner prompt continues to be single-pass; the
  brainstorm pause lives in the new skill, not in the renderer.
- No prescribed length for the brainstorm. The "2-3 alternative
  framings" count is soft guidance; the agent judges based on the
  ask's complexity. A trivial ask may need zero alternatives; a
  load-bearing architecture ask may need four.
- No special-casing for the amendment form. The brainstorm skill is
  framing-agnostic; both new-spec and amendment paths can use it. In
  practice the amendment path rarely needs brainstorming because the
  framing is locked in by the existing SPEC, but nothing in the
  skill prevents it.
- No new test infrastructure beyond the existing skill-pack and
  init enumeration assertions, which are extended to cover the new
  skill name.
</non-goals>

## User Stories

<user-stories>
- As a solo developer with a fuzzy idea, I want a skill that breaks
  the idea down into atomic, first-principle requirements before
  any SPEC is written, so I can redirect the framing while it is
  still cheap (pre-Requirement) rather than after `<requirement>`
  blocks have hardened.
- As an AI agent rendering planning artifacts, I want an explicit
  hard-gate instruction telling me to pause and wait for user
  approval before invoking `/speccy-plan`, so I do not skip the
  clarification step and silently bake in assumptions.
- As a maintainer of the shipped skill pack, I want
  `speccy-brainstorm` to ship alongside the other speccy-* skills
  through the same `init`/`render_host_pack` pipeline, with the
  same per-host wrapper-template pattern, so the new skill does not
  require a parallel installation mechanism.
- As a user reading `resources/modules/skills/speccy-plan.md`, I
  want the plan skill to point at `/speccy-brainstorm` as the
  recommended precursor when the ask is fuzzy, so I know to reach
  for it without having to read the brainstorm skill body first.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Shipped speccy-brainstorm skill exists and is host-wrapped

A new shipped skill `speccy-brainstorm` exists at
`resources/modules/skills/speccy-brainstorm.md` and is wrapped for
both Claude Code and Codex via the existing wrapper-template
pattern. The skill ships to `.claude/skills/speccy-brainstorm/SKILL.md`
and `.agents/skills/speccy-brainstorm/SKILL.md` when `speccy init`
runs against a project.

<done-when>
- `resources/modules/skills/speccy-brainstorm.md` is non-empty and
  carries the `{{ cmd_prefix }}speccy-brainstorm` slug-style heading
  the other shipped skill bodies use.
- `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  exists and delegates to the module body via
  `{% include "modules/skills/speccy-brainstorm.md" %}` with a
  YAML frontmatter `name:` / `description:` pair.
- `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
  exists and follows the same pattern as the Claude Code wrapper.
- `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` and
  `speccy-cli/tests/init.rs::SKILL_NAMES` both include
  `speccy-brainstorm` so the existing presence / rendering / scaffold
  assertions cover it.
- `cargo run -- init --force` against this workspace overwrites the
  host-local mirrors at `.claude/skills/speccy-brainstorm/SKILL.md`
  and `.agents/skills/speccy-brainstorm/SKILL.md`, matching the
  embedded bundle byte-for-byte.
</done-when>

<behavior>
- Given a fresh workspace, when `speccy init --force --host claude-code`
  is run, then `.claude/skills/speccy-brainstorm/SKILL.md` is
  created with the rendered module body inside.
- Given the same fresh workspace, when `speccy init --force --host codex`
  is run, then `.agents/skills/speccy-brainstorm/SKILL.md` is
  created with the rendered module body inside.
- Given the shipped skill files, when their frontmatter is read,
  then the `name: speccy-brainstorm` field is present and the
  `description:` field names the brainstorming purpose so the host
  harness can match the skill against user invocations.
</behavior>

<scenario id="CHK-001">
Given a fresh workspace where `speccy init --force --host claude-code`
has just run,
when `.claude/skills/speccy-brainstorm/SKILL.md` is read,
then the file exists, is non-empty, carries a `name: speccy-brainstorm`
frontmatter line, and its body matches the rendered output of
`render_host_pack(HostChoice::ClaudeCode)` for the
`speccy-brainstorm` wrapper byte-for-byte.

Given the same fresh workspace, when
`speccy init --force --host codex` has just run, then
`.agents/skills/speccy-brainstorm/SKILL.md` exists with the same
properties (name, non-empty body, byte-identical to the Codex
rendered output).

Given `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` and
`speccy-cli/tests/init.rs::SKILL_NAMES`, when inspected, then both
slices contain the `"speccy-brainstorm"` entry.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Brainstorm skill body teaches the Socratic flow with a hard gate

The skill body at `resources/modules/skills/speccy-brainstorm.md`
describes a Socratic clarification flow: explore the project context,
ask clarifying questions one at a time, propose 2-3 alternative
framings with trade-offs, restate the ask as atomic first-principle
requirements, list the open questions whose answers would change the
SPEC shape, and stop and wait for user approval before invoking
`/speccy-plan`.

<done-when>
- The skill body names the four artifacts the agent must produce:
  (a) a restated ask broken into atomic, first-principle requirements;
  (b) 2-3 alternative framings considered, each with a one-sentence
  sketch and the reason for rejecting it in favor of the chosen
  framing; (c) silent assumptions the agent would otherwise bake in;
  (d) open questions in the `- [ ]` checkbox format used by the PRD
  template's `## Open Questions` section, so the brainstorm output
  is copy-pasteable into the eventual SPEC.md.
- The skill body names "2-3" as the suggested count for alternative
  framings and explicitly marks the count as soft guidance the agent
  should scale to the slice complexity.
- The skill body carries a hard-gate instruction: "do not invoke
  `/speccy-plan` and do not write SPEC.md until the user has
  approved the framing." The gate is named explicitly in prose; no
  machine marker is introduced.
- The skill body names the routing for salient brainstorm outputs
  when the agent eventually invokes `/speccy-plan`: restated ask
  informs `## Summary`; silent assumptions land under `<assumptions>`
  inside `## Assumptions`; open questions land under
  `## Open Questions`; rejected framings land under `## Notes`, or
  in `### Decisions` as a `<decision>` block when load-bearing.
- The skill body teaches "one question at a time" as an explicit
  interaction discipline, mirroring the obra/superpowers approach.
- The skill body ends by pointing at `/speccy-plan` (or
  `/speccy-amend` if the user wants to amend an existing spec) as
  the next step after framing is approved.
</done-when>

<behavior>
- Given the rendered Claude Code skill file at
  `.claude/skills/speccy-brainstorm/SKILL.md`, when grep'd for "hard
  gate" or equivalent pause language, then a hit is returned and the
  surrounding prose names "/speccy-plan" as the gated action.
- Given the rendered skill file, when grep'd for "one question at a
  time", then a hit is returned.
- Given the rendered skill file, when grep'd for the four
  destination strings (`## Summary`, `## Assumptions`,
  `## Open Questions`, `## Notes`), then each appears at least
  once in the routing-instruction section.
- Given the rendered skill file, when read end-to-end, then it names
  `/speccy-plan` as the terminal next step after the user has
  approved the framing.
</behavior>

<scenario id="CHK-002">
Given the embedded
`resources/modules/skills/speccy-brainstorm.md`,
when read,
then it names four artifacts (restated ask broken into atomic
first-principle requirements; 2-3 alternative framings with sketches
and rejection reasons; silent assumptions; open questions in the
`- [ ]` checkbox format), names "2-3" as soft guidance scaled to
slice complexity, carries an explicit hard-gate instruction naming
`/speccy-plan` as the gated action, and names the four destination
sections (`## Summary`, `## Assumptions`, `## Open Questions`,
`## Notes`) plus `### Decisions` / `<decision>` for load-bearing
trade-offs in its routing-instruction section.

Given the same skill body,
when grep'd case-insensitively for `one question at a time`,
then a hit is returned.

Given the rendered Claude Code wrapper at
`.claude/skills/speccy-brainstorm/SKILL.md`,
when read end-to-end,
then `/speccy-plan` is named as the terminal next step (the action
the agent invokes after the user has approved the framing).

Given the rendered Codex wrapper at
`.agents/skills/speccy-brainstorm/SKILL.md`,
when read end-to-end,
then `speccy-plan` (no leading slash, per Codex prefix conventions)
is named as the terminal next step.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: speccy-plan skill body references brainstorm as optional precursor

`resources/modules/skills/speccy-plan.md` is updated to point at
`/speccy-brainstorm` as a recommended precursor when the user's ask
is fuzzy. The stale "the prompt inlines `AGENTS.md`" /
"inlines the nearest parent `MISSION.md`" wording (rotten since
SPEC-0023 REQ-005 / REQ-006) is corrected in the same edit. The
existing two-form structure (new-spec, amendment) is preserved; the
brainstorm reference is a recommendation, not a gate.

<done-when>
- `resources/modules/skills/speccy-plan.md` names
  `{{ cmd_prefix }}speccy-brainstorm` as a recommended precursor in
  the "When to use" or equivalent section, with a one-line note
  about when to reach for it (fuzzy ask, framing not yet agreed).
- The file no longer contains the substrings "inlines `AGENTS.md`"
  or "inlines AGENTS.md" or "inlines the nearest parent
  `MISSION.md`" or equivalent claims that the rendered prompt
  embeds those bodies. After SPEC-0023 REQ-005 / REQ-006, the
  host harness auto-loads `AGENTS.md` and the rendered prompt names
  the MISSION.md path for Read primitive use.
- The amendment-form description in the same file is preserved as a
  single-pass surgical edit description; the brainstorm reference is
  framing-agnostic but not duplicated on the amendment branch.
- Wrapper templates under `resources/agents/.claude/skills/speccy-plan/`
  and `resources/agents/.agents/skills/speccy-plan/` are reviewed;
  any wrapper carrying independent description text gets the same
  update. Wrappers that delegate to the module body inherit the fix.
</done-when>

<behavior>
- Given `resources/modules/skills/speccy-plan.md` after this spec
  lands, when read, then it names `speccy-brainstorm` as a
  recommended precursor for fuzzy asks.
- Given the same file, when grep'd case-sensitively for "inlines
  `AGENTS.md`", "inlines AGENTS.md", "inlines the nearest parent",
  or "inlines `MISSION.md`", then no hit is returned.
- Given each wrapper template under `resources/agents/`, when its
  description text is read, then the text does not contradict the
  brainstorm-as-precursor recommendation and does not claim that
  AGENTS.md or MISSION.md is inlined into the rendered prompt.
</behavior>

<scenario id="CHK-003">
Given `resources/modules/skills/speccy-plan.md` after this spec
lands,
when its "When to use" section is read,
then it names `{{ cmd_prefix }}speccy-brainstorm` as a recommended
precursor for fuzzy asks.

Given the same file,
when grep'd for "inlines `AGENTS.md`", "inlines AGENTS.md",
"inlines the nearest parent `MISSION.md`", or "inlines `MISSION.md`",
then no hit is returned.

Given the amendment-form description in the same file,
when read,
then it remains a single-pass surgical edit description with no
mandatory brainstorm step (consistent with the non-goal that the
amendment path can but rarely needs brainstorming).

Given each wrapper template under
`resources/agents/.claude/skills/speccy-plan/` and
`resources/agents/.agents/skills/speccy-plan/`,
when its description / frontmatter text is read,
then the text does not contradict the brainstorm-as-precursor
recommendation and does not claim AGENTS.md or MISSION.md is inlined
into the rendered prompt.
</scenario>

</requirement>

## Design

### Approach

Implementation order:

1. Write the new shipped skill body at
   `resources/modules/skills/speccy-brainstorm.md`. The body teaches
   the Socratic flow (explore → questions one at a time → 2-3
   approaches with trade-offs → restate the ask as atomic
   first-principle requirements → list silent assumptions and open
   questions → hard-gate pause for user approval), with the
   routing instruction folded into the post-approval guidance.
2. Add wrapper templates at
   `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
   and `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
   following the exact pattern of the existing speccy-* wrappers
   (YAML frontmatter with `name:` and `description:` fields, then a
   `{% include "modules/skills/speccy-brainstorm.md" %}` line).
3. Update `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` and
   `speccy-cli/tests/init.rs::SKILL_NAMES` to include the new entry.
   Both arrays are hard-coded slices; the change is mechanical. The
   length of the init.rs slice is part of its type (`[&str; 7]`); the
   array literal type updates to `[&str; 8]` automatically.
4. Edit `resources/modules/skills/speccy-plan.md` to reference
   `/speccy-brainstorm` as a recommended precursor and to correct
   the stale "inlines `AGENTS.md`" / "inlines the nearest parent
   `MISSION.md`" wording. Sanity-check the wrapper templates under
   `resources/agents/.claude/skills/speccy-plan/` and
   `resources/agents/.agents/skills/speccy-plan/`; both currently
   delegate to the module body via `{% include %}`, so they inherit
   the fix.
5. Re-eject the host-local skill files via `cargo run -- init
   --force --host claude-code` and `cargo run -- init --force --host
   codex` in this repo so the dogfooded `.claude/skills/` and
   `.agents/skills/` mirrors reflect the new skill body and the
   updated speccy-plan skill body.
6. Verify by reading the rendered `.claude/skills/speccy-brainstorm/SKILL.md`
   and `.agents/skills/speccy-brainstorm/SKILL.md` files, then run
   the full hygiene gate (`cargo test --workspace`, `cargo clippy
   --workspace --all-targets --all-features -- -D warnings`,
   `cargo +nightly fmt --all --check`, `cargo deny check`).

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Brainstorm is a standalone skill, not a phase inside `/speccy-plan`

**Status:** Accepted

**Context:** The original SPEC-0025 draft (see Changelog row dated
2026-05-17 "Initial draft") proposed adding a Phase 1 brainstorm
block to the rendered `speccy plan` (greenfield) prompt. During
dogfooding the framing was reconsidered: brainstorming is generic
to "creative work", not specific to writing a new SPEC. Embedding
it inside `/speccy-plan` means amendment can't use it (DEC-001 of
the original draft explicitly scoped to greenfield-only), and the
two-phase prompt structure complicates the renderer's job
unnecessarily. The obra/superpowers brainstorming skill takes the
generic standalone-skill shape; it's a clean model for Speccy.

**Decision:** Brainstorming lives in a standalone shipped skill,
`speccy-brainstorm`. The skill body teaches the Socratic flow; the
`speccy plan` rendered prompt is unchanged. `/speccy-plan` mentions
brainstorming as a recommended precursor for fuzzy asks; the rest
of the loop is unchanged.

**Consequences:** The skill is framing-agnostic — both new-spec and
amendment paths can use it. The CLI surface stays at ten commands.
The brainstorm pause is prose-marked (matches Principle 1: feedback,
not enforcement). The original draft's plan-greenfield template
edits are reverted; nothing in the rendered prompt knows about the
brainstorm step. The skill enumeration in the test corpus needs the
new entry, but no other CLI surface or renderer change is required.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Brainstorm output is ephemeral chat, no disk artifact

**Status:** Accepted

**Context:** Considered writing brainstorm output to a dated file
under `.speccy/brainstorm/YYYY-MM-DD-<topic>.md` (the obra/superpowers
shape, which writes to `docs/superpowers/specs/<date>-design.md`).
That would create a new on-disk concept Speccy has to track,
version, and reason about. It would also bloat downstream prompts
that need to name brainstorm history. Two reasons it is the wrong
shape for Speccy:
- "Stay small" (Principle 5): every new on-disk concept adds noun
  surface. The five nouns (Mission, Spec, Requirement, Task, Check)
  are deliberately fixed; brainstorm-as-noun would push that to six.
- Salient brainstorm outputs already have natural homes in SPEC.md
  (`## Summary` for restated ask, `## Assumptions` for surfaced
  assumptions, `## Open Questions` for unresolved questions,
  `## Notes` for rejected framings, `### Decisions` for load-bearing
  trade-offs). Routing into existing sections keeps the durable
  record where readers will look.

**Decision:** Brainstorm dialogue is ephemeral. The agent writes
nothing to disk during the brainstorm. After the user approves the
framing, the agent invokes `/speccy-plan`, which writes SPEC.md;
the brainstorm's salient outputs are folded into SPEC.md's
existing sections at that point.

**Consequences:** SPEC.md stays the canonical durable record; the
brainstorm chat disappears after the SPEC is written. Nothing is
lost because the salient outputs are captured in the sections that
already exist. If a brainstorm produces context that does not fit
any existing section, that is a signal to either drop the context
or expand a section's scope — not to invent a new section.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Hard-gate pause is prose-marked, not machine-enforced

**Status:** Accepted

**Context:** obra/superpowers uses an XML-like `<HARD-GATE>` tag
inside the skill body to surface the gate visually, but the gate is
still prose to the agent — there is no parser. Considered adding a
machine-detectable sentinel that the CLI could enforce. That would
require the CLI to parse the agent's chat output, which it does not
do today and which collides with Principle 2 ("deterministic core,
intelligent edges").

**Decision:** The hard gate is prose. The skill body names the gate
explicitly ("do not invoke `/speccy-plan` and do not write SPEC.md
until the user has approved the framing") in language strong enough
that an attentive agent will honor it. The CLI does not enforce.

**Consequences:** Drift can still happen if an agent ignores the
gate. That class of drift is exactly what reviewer fan-out is
designed to catch (Principle 4: review owns semantic judgment). The
CLI surface stays small (Principle 5). If dogfooding shows agents
routinely skip the gate, the response is a stronger prompt or a
reviewer persona that checks for brainstorm residue in SPEC.md, not
a CLI parser.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: Skill body is static — no new template variables

**Status:** Accepted

**Context:** Considered interpolating project-specific context into
the brainstorm skill (e.g., a list of recent SPEC titles to spot
overlap). The marginal information value is low: the host harness
auto-loads `AGENTS.md`, the agent can use its Read primitive to
scan `.speccy/specs/` on demand, and adding a new variable means
more state for the wrapper template renderer to populate.

**Decision:** Skill body is static markdown. The only template
variable used is the existing `{{ cmd_prefix }}` (already populated
by the renderer's host-aware template context for `/` vs no-prefix
hosts). No new variable is introduced.

**Consequences:** The renderer change for this spec is zero — only
new files are added. Future skill-body iterations require only
markdown edits, no renderer changes. If a future spec wants
project-aware brainstorming (e.g., highlighting overlapping recent
specs), it adds a renderer variable then; this spec stays small.
</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: Alternative-framings count (2-3) is soft guidance

**Status:** Accepted

**Context:** Considered specifying hard counts ("propose exactly
three alternatives") or no counts at all. Hard counts produce
mechanical artifacts (the third alternative on a trivial ask is
"don't do this at all" — true but useless). No counts produce wide
variance where some agents propose one alternative and call it
done. Soft guidance with a noted "scale to slice complexity" caveat
is the right middle. Same shape as the original SPEC-0025 draft's
3-5 / 2-3 soft-counts decision (DEC-005 of that draft), preserved
under the new framing.

**Decision:** The skill names "2-3" as the suggested count for
alternative framings, with explicit language that this is soft
guidance and the agent should scale to the ask's complexity.

**Consequences:** Brainstorm output stays proportional to ask
complexity rather than artifactual length. If dogfooding shows
agents under-deliver on simple asks (always producing the minimum),
revisit the framing then.
</decision>

### Interfaces

- `resources/modules/skills/speccy-brainstorm.md` — new file. The
  shipped skill body teaching the Socratic flow.
- `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  — new file. Claude Code host wrapper.
- `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
  — new file. Codex host wrapper.
- `resources/modules/skills/speccy-plan.md` — edited to reference
  `/speccy-brainstorm` as a recommended precursor and to correct
  the stale "inlines `AGENTS.md`" / "inlines the nearest parent
  `MISSION.md`" wording.
- `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` — extended with
  `"speccy-brainstorm"`.
- `speccy-cli/tests/init.rs::SKILL_NAMES` — extended with
  `"speccy-brainstorm"`; the array literal type updates from
  `[&str; 7]` to `[&str; 8]`.
- `.claude/skills/speccy-brainstorm/SKILL.md` and
  `.agents/skills/speccy-brainstorm/SKILL.md` (host-local dogfood
  mirrors) — generated via `cargo run -- init --force` after the
  resource edits land.
- `resources/modules/prompts/plan-greenfield.md` and
  `resources/modules/prompts/plan-amend.md` — unchanged.

### Data changes

None. No artifact grammar or schema change.

### Migration / rollback

- Forward: ship the new skill file plus the speccy-plan body edit.
  No state migration needed.
- Rollback: delete the new skill files and revert the speccy-plan
  body edit. No artifact in `.speccy/specs/` becomes invalid; only
  the next `speccy init --force` invocation drops the new skill
  mirror.

## Open Questions

- [x] Should the brainstorm skill's "explore project context" step
  reference `.speccy/specs/` history (e.g., "before brainstorming,
  scan recent SPEC titles to spot overlap")? — **Skip.** Let the
  agent decide whether to scan recent specs based on the ask's
  complexity rather than baking the scan into the prompt. More
  prompt text for a behavior the agent can do via Read on its own
  is the worse trade. Revisit if dogfooding shows duplicate framings
  sneaking through brainstorm.
- [x] Should the speccy-plan amendment-form description carry its
  own brainstorm reference (a "for amendments to fuzzy intent shifts,
  brainstorm first" pointer)? — **Skip.** The brainstorm skill is
  framing-agnostic; agents can reach for it on either path. The
  amendment path's framing is usually locked in by the existing
  SPEC, so a forced reference would be ceremony without payoff.
  Revisit if dogfooding shows amendment-form drift.

## Assumptions

<assumptions>
- The host agent (Claude Code, Codex, others) honors a prose-marked
  hard-gate instruction inside a skill body. Empirically true across
  SPEC-0023's Read-pointer pattern and the obra/superpowers
  brainstorming skill, which uses the same prose-gate shape.
- The shipped-skill scaffold pattern (module body at
  `resources/modules/skills/<name>.md` + per-host wrappers at
  `resources/agents/.<install_root>/skills/<name>/SKILL.md.tmpl`)
  scales to a new skill without renderer changes. SPEC-0016 set
  this up; the renderer walks `agents/<install_root>/` recursively,
  so adding a new sub-directory is picked up automatically.
- The PRD template's `## Summary`, `## Assumptions`,
  `## Open Questions`, `## Notes`, and `### Decisions` sections
  remain the canonical homes for summary-like, assumption-like,
  question-like, note-like, and load-bearing-decision content
  respectively. If `.speccy/ARCHITECTURE.md` later restructures
  these, the routing-instruction section in
  `speccy-brainstorm.md` is the trailing edge that needs an update.
- A skill body without a backing CLI command is a tractable new
  pattern. The existing seven shipped skills each map to a CLI
  command (or to `init`), but the skill mechanism itself doesn't
  require a CLI command — it just requires a markdown body the
  host harness can load. Adding `speccy-brainstorm` as the first
  skill-without-command is a small precedent that, if dogfooding
  shows it confuses hosts, can be revisited.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-17 | human/kevin | Initial draft. Greenfield planner prompt gains a Phase 1 brainstorming block with restated ask + assumptions + alternative framings + open questions; pause-and-wait boundary marked by a horizontal rule; Phase 2 routes salient outputs into existing SPEC.md sections. Amendment form unchanged. Prompt-template-only; no CLI surface change. |
| 2026-05-17 | human/kevin | Amend (full pivot). Drop the "greenfield phase 1" framing entirely. Brainstorming becomes a standalone shipped skill `speccy-brainstorm`, framing-agnostic across new-spec and amendment paths, inspired by obra/superpowers's brainstorming skill. The skill body teaches a Socratic flow with a prose hard gate before invoking `/speccy-plan`. No new CLI command, no disk artifact for brainstorm output. The original draft's `plan-greenfield.md` and template tests are reverted; this spec ships a new skill file plus a speccy-plan-skill body update referencing the brainstorm as recommended precursor for fuzzy asks. |
| 2026-05-17 | human/kevin | Resolve both open questions as **skip**. The agent decides whether to scan `.speccy/specs/` history during brainstorm based on the ask's complexity (not via baked-in prompt text); the amendment-form description does not carry a forced brainstorm reference (framing-agnostic skill is sufficient). Both default positions promoted to decisions; revisit only if dogfooding shows duplicate framings or amendment-form drift. |
</changelog>

## Notes

This spec was rewritten mid-implementation after the user pointed
out that the original framing (Phase 1 inside the greenfield
planner) entangled brainstorming with a specific planner form and
missed the deeper goal: a generic primitive for atomizing intent
before committing to a SPEC. The pivot trades some inline
ergonomics (Phase 1 in the rendered prompt) for a cleaner shape
(brainstorm as a separate, framing-agnostic skill). obra/superpowers
was the load-bearing reference: its `skills/brainstorming/SKILL.md`
demonstrates that a pure-skill primitive without a backing CLI
command can carry the hard-gate clarification flow effectively, as
long as the prose is strong enough.

Stay-small applies: no new CLI command, no new noun, no new SPEC.md
section, no new on-disk artifact, no machine-enforced gate. Each
was considered and rejected; see the Decisions block. If
dogfooding earns any of these in, they become their own amendment
or a successor spec.
