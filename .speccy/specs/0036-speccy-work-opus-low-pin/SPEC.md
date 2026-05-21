---
id: SPEC-0036
slug: speccy-work-opus-low-pin
title: Repin Claude Code speccy-work implementer to opus[1m] / low effort
status: implemented
created: 2026-05-20
supersedes: []
---

# SPEC-0036: Repin Claude Code `speccy-work` implementer to `opus[1m]` / low effort

## Summary

SPEC-0032 pinned the three Claude Code mechanical phase-worker
subagents (`speccy-tasks`, `speccy-work`, `speccy-ship`) to
`model: sonnet[1m]` / `effort: medium`. The reasoning was that
implementer-grade work is bulk Sonnet volume rather than the
semantic adversarial load that justifies an Opus pin on the three
heavy reviewer personas. Dogfooding on this repository has shifted
that judgement for the implementer specifically: the
`speccy-work` agent reads SPEC.md, TASKS.md, and the relevant
module source, then writes tests-first code under the project's
hygiene gates. The work benefits more from Opus's higher
single-pass quality than from Sonnet's lower per-token cost when
the per-task token volume is modest and the cost of a wrong edit
is a full re-loop through review. The `effort: low` setting on
Opus delivers materially better implementer output than Sonnet
medium while keeping latency and spend close to the prior tier.

SPEC-0036 is scoped narrowly: only the Claude Code `speccy-work`
pin moves. The other two pinned mechanical phases (`speccy-tasks`
and `speccy-ship`) stay on `sonnet[1m]` / medium — `tasks` is
decomposition shaped like template-filling, and `ship` is
report-writing plus a CLI dry-run; neither carries the same
single-pass-quality risk as the implementer. The Codex parallel
pin (`.codex/agents/speccy-work.toml`) is out of scope: OpenAI's
model identifier does not expose an Opus-vs-Sonnet axis, so the
Codex side already lives entirely in `model_reasoning_effort`,
and the Codex tier choice is a separate judgement call for a
future amendment if needed. The reviewer pins from SPEC-0032 are
also untouched.

Two files change frontmatter values, and the project README's
pin-assignment table and one override example update to match. No
CLI surface change, no schema change, no shared-body or template
structure change.

## Goals

<goals>
- The in-tree dogfood agent file at
  `.claude/agents/speccy-work.md` declares `model: opus[1m]` and
  `effort: low` in its YAML frontmatter. All other frontmatter
  fields (`name`, `description`) and the `{% include ... %}` body
  reference are unchanged.
- The matching template source at
  `resources/agents/.claude/agents/speccy-work.md.tmpl` declares
  the same `model: opus[1m]` and `effort: low` values so
  `speccy init` in a fresh project renders the updated pin.
- The project `README.md` pin-assignment table row for
  `speccy-work` shows `model: opus[1m]`, `effort: low` in the
  Claude Code column. The Codex column on the same row is
  unchanged.
- The README override example that demonstrates "lock
  `speccy-work` to a specific Claude version" updates its swap
  target from `model: sonnet[1m] → model: claude-sonnet-4-6[1m]`
  to a swap that mentions the new pin (e.g. `model: opus[1m] →
  model: claude-opus-4-7[1m]`), so the worked example stays
  consistent with the new shipped frontmatter.
- The existing CI host-pack drift-check meta-test
  (`speccy-core/tests/host_pack_drift.rs` or its equivalent
  guard) continues to pass: the templated source under
  `resources/agents/.claude/agents/speccy-work.md.tmpl` and the
  in-tree rendered file at `.claude/agents/speccy-work.md`
  remain byte-aligned modulo the templating header that the
  drift check already accounts for.
- All four standard-hygiene gates (`cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features --
  -D warnings`, `cargo +nightly fmt --all --check`,
  `cargo deny check`) exit 0 against the post-SPEC workspace.
</goals>

## Non-goals

<non-goals>
- No change to the Codex `speccy-work` pin at
  `.codex/agents/speccy-work.toml` or its template under
  `resources/agents/.codex/agents/speccy-work.toml.tmpl`. The
  Codex side stays at `model = "gpt-5.5"` and
  `model_reasoning_effort = "medium"`. Re-tuning Codex's
  implementer tier, if warranted, is a separate amendment.
- No change to the `speccy-tasks` or `speccy-ship` Claude Code
  pins. Both stay at `sonnet[1m]` / medium. This SPEC is the
  implementer's repin only, not a sweep of all three mechanical
  phases.
- No change to any reviewer pin on either host. The six
  reviewer pin assignments from SPEC-0032 are untouched.
- No change to the speccy-work skill body, the shared phase
  body at `resources/modules/phases/speccy-work.md`, or the
  stub SKILL.md at `.claude/skills/speccy-work/SKILL.md`.
  Procedure is unchanged; only the model/effort pin moves.
- No change to the agent or skill frontmatter shape. No new
  optional fields; `schema_version` does not move. The same
  `name` / `description` / `model` / `effort` quartet that
  SPEC-0032 introduced is reused with new values for `model`
  and `effort`.
- No change to the project's effort enum. `low` is already in
  the documented Opus effort range (`low`, `medium`, `high`,
  `xhigh`, `max`) per SPEC-0032 REQ-005, so the new pin needs
  no enum extension or validation update.
- No retroactive change to historical specs or to Speccy's own
  REPORT.md trail. The new pin applies from this SPEC's tasks
  landing forward; users who already ran `speccy init` are
  unaffected until they re-run init or copy the new frontmatter
  manually.
- No introduction of a measurement step. The SPEC does not
  bench Opus low vs Sonnet medium on implementer work; the
  judgement is qualitative and revisable via a future amendment
  if dogfooding contradicts it.
- No update to the conversational-skill frontmatter
  (`speccy-brainstorm`, `speccy-plan`, `speccy-amend`); those
  remain unpinned per SPEC-0032's reasoning.
</non-goals>

## User Stories

<user-stories>
- As a Speccy user who has invoked `/agent speccy-work` on
  Claude Code to opt into the pinned implementer path, I want
  the subagent to activate at Opus (1M context) on `low` effort
  rather than Sonnet medium, so single-pass implementation
  quality is higher and the cost of a wrong edit (a full
  re-loop through `/speccy-review`) drops.
- As a Speccy user running `speccy init` in a fresh project
  today, I want the ejected
  `.claude/agents/speccy-work.md` to carry the
  `opus[1m]` / `low` pin out of the box so I inherit the
  current implementer tuning without re-deriving it from
  AGENTS.md.
- As a Speccy user reading the README's pin-assignment table
  to discover which subagent runs at which tier, I want the
  `speccy-work` row's Claude Code column to match the
  frontmatter that actually ships, so the docs do not lie about
  the in-tree state.
- As a Speccy user following the README override example for
  pinning `speccy-work` to a dated Claude snapshot, I want the
  example's "before" value to match what ships, so copying it
  produces a working override rather than a no-op.
- As a maintainer reviewing this SPEC's diff, I want the change
  surface to be three files (`SPEC.md` aside): the templated
  source, the rendered in-tree file, and the README. Everything
  else stays still — no shared-body edit, no skill-body edit,
  no Codex-side touch, no new CLI flag.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Claude Code `speccy-work` agent files declare `opus[1m]` / `low`

The in-tree dogfood agent file at
`.claude/agents/speccy-work.md` and its templated source at
`resources/agents/.claude/agents/speccy-work.md.tmpl` both carry
YAML frontmatter declaring `model: opus[1m]` and `effort: low`.
All other frontmatter fields (`name: speccy-work`,
`description:`) and the body content (a single
`{% include "modules/phases/speccy-work.md" %}` line in the
template; the include's rendered text in the dogfood file)
are unchanged from the pre-SPEC version.

<done-when>
- `.claude/agents/speccy-work.md` YAML frontmatter contains
  `model: opus[1m]` and `effort: low`. The frontmatter does
  not contain any other model alias (`sonnet[1m]`, `haiku`,
  etc.) and does not contain an `effort` value other than
  `low`.
- `resources/agents/.claude/agents/speccy-work.md.tmpl` YAML
  frontmatter contains `model: opus[1m]` and `effort: low`,
  matching the dogfood file byte-for-byte in the frontmatter
  block (the body block continues to be the
  `{% include ... %}` directive in the template and the
  rendered include output in the dogfood file).
- The `name:` and `description:` fields in both files are
  byte-identical to their pre-SPEC values.
- The body content (everything below the `---` frontmatter
  terminator) is byte-identical to its pre-SPEC content in
  each file.
- The existing CI host-pack drift-check meta-test exits 0
  against the post-SPEC workspace (templated source and
  in-tree rendered output remain in sync per the test's
  established comparison rules).
</done-when>

<behavior>
- Given `.claude/agents/speccy-work.md` after this SPEC's
  tasks land, when its YAML frontmatter is parsed, then
  `model` equals the literal string `opus[1m]` and `effort`
  equals the literal string `low`.
- Given
  `resources/agents/.claude/agents/speccy-work.md.tmpl`
  after this SPEC's tasks land, when its YAML frontmatter is
  parsed, then `model` equals `opus[1m]` and `effort` equals
  `low`.
- Given both files, when the body content (everything below
  the second `---` line) is diffed against the pre-SPEC
  version, then the diff is empty.
- Given the post-SPEC workspace, when the existing CI
  host-pack drift-check runs, then it exits 0.
</behavior>

<scenario id="CHK-001">
Given `.claude/agents/speccy-work.md` after this SPEC's tasks
land, when its YAML frontmatter is parsed, then `model` equals
`opus[1m]` and `effort` equals `low`.
</scenario>

<scenario id="CHK-002">
Given `resources/agents/.claude/agents/speccy-work.md.tmpl`
after this SPEC's tasks land, when its YAML frontmatter is
parsed, then `model` equals `opus[1m]` and `effort` equals
`low`.
</scenario>

<scenario id="CHK-003">
Given the post-SPEC workspace, when the CI host-pack
drift-check meta-test runs, then it exits 0 (the templated
source and the in-tree rendered output remain in sync).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: README pin-assignment table and override example reflect the new pin

The project `README.md`'s "Pin assignment" table row for
`speccy-work` shows `model: opus[1m]`, `effort: low` in the
Claude Code column. The Codex column on the same row is
unchanged (`model = "gpt-5.5"`, reasoning effort medium). The
table's surrounding rows (`speccy-tasks`, `speccy-ship`,
`speccy-init`, `speccy-review`, and all six reviewer rows)
are unchanged.

The README's "Overriding a pin" worked example that today
reads "Lock `speccy-work` to a specific Claude version for
reproducibility: change `model: sonnet[1m]` to
`model: claude-sonnet-4-6[1m]` in
`.claude/agents/speccy-work.md`" updates so that the "before"
value matches what now ships: the example references
`model: opus[1m]` as the alias and a Claude Opus snapshot ID
(e.g. `claude-opus-4-7[1m]`) as the locked version. Equivalent
prose is acceptable as long as the example's "before" value
matches the new shipped frontmatter and the override target
references an Opus snapshot rather than a Sonnet snapshot.

<done-when>
- The `README.md` "Pin assignment" table's `speccy-work` row
  reads `model: opus[1m]`, `effort: low` in the Claude Code
  column.
- The `speccy-work` row's Codex column is unchanged.
- The other ten rows in the same table (`speccy-tasks`,
  `speccy-ship`, `speccy-init`, `speccy-review`,
  `reviewer-business`, `reviewer-tests`,
  `reviewer-architecture`, `reviewer-security`,
  `reviewer-style`, `reviewer-docs`) are unchanged from
  their pre-SPEC contents.
- The README's "Overriding a pin" `speccy-work` example
  names `model: opus[1m]` as the "before" alias and an Opus
  snapshot ID as the lock target.
- The README contains no remaining reference to
  `speccy-work` paired with `model: sonnet[1m]` or
  `effort: medium` (other than as historical context inside
  the SPEC-0032 references, which live under
  `.speccy/specs/0032-*/` and are out of scope).
</done-when>

<behavior>
- Given `README.md` after this SPEC's tasks land, when the
  "Pin assignment" table is parsed, then the row whose
  first column is `speccy-work` has a Claude Code column
  containing the substrings `opus[1m]` and `low` (and no
  occurrence of `sonnet[1m]` or `effort: medium` on that
  row).
- Given the same README, when grepped for the literal
  substring `speccy-work` together with `sonnet[1m]` on
  the same line, then zero matches are found.
- Given the same README, when the "Overriding a pin"
  section is read, then the worked `speccy-work` example
  names `opus[1m]` rather than `sonnet[1m]` as the
  "before" alias.
</behavior>

<scenario id="CHK-004">
Given `README.md` after this SPEC's tasks land, when the
"Pin assignment" table row for `speccy-work` is read, then
the Claude Code column contains the literal substrings
`opus[1m]` and `low`, and contains neither `sonnet[1m]` nor
`medium` on that row.
</scenario>

<scenario id="CHK-005">
Given `README.md` after this SPEC's tasks land, when the
"Overriding a pin" `speccy-work` worked example is read,
then the example names `model: opus[1m]` as the "before"
alias and a Claude Opus snapshot ID (any of the
`claude-opus-*` family) as the lock target.
</scenario>

<scenario id="CHK-006">
Given `README.md` after this SPEC's tasks land, when grepped
for any line containing both `speccy-work` and `sonnet[1m]`,
then zero matches are found.
</scenario>

</requirement>

## Open Questions

- [ ] Should the Codex `speccy-work` pin move in lockstep (e.g.
      bump `model_reasoning_effort` from `medium` to something
      else, since Codex has no Opus-vs-Sonnet axis to flip)?
      Recommendation: leave Codex unchanged in this SPEC. OpenAI's
      effort dial is the only knob and the Codex implementer
      tier choice is independent of the Claude Code one;
      re-tuning it belongs in a separate amendment driven by
      Codex-side dogfooding, not piggybacked here. Promote if
      Codex dogfooding surfaces a concrete need.
- [ ] Should `effort: low` on Opus be reconsidered as
      `effort: medium` once dogfooding produces a few full loops
      on the new pin? Recommendation: ship `low` first. The
      pin is editable post-eject and the README documents the
      override path; if implementer quality on `low` proves
      insufficient, a follow-up amendment bumps to `medium`
      with the same surgical change shape as this SPEC.

## Changelog

<changelog>
| Date       | Reason                                       | Author |
|------------|----------------------------------------------|--------|
| 2026-05-20 | Initial draft. Repin Claude Code `speccy-work` from `sonnet[1m]`/medium to `opus[1m]`/low in both the in-tree dogfood agent file and the templated source under `resources/agents/.claude/agents/`. Update the README pin-assignment table row and the override worked example to match. Codex side, reviewer pins, and the other two mechanical phase pins (`speccy-tasks`, `speccy-ship`) are deliberately out of scope. | Kevin Xiao |
</changelog>
