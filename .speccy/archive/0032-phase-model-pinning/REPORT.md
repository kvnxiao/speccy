---
spec: SPEC-0032
outcome: delivered
generated_at: 2026-05-19T18:00:00Z
---

# Report: SPEC-0032 Per-phase model and effort pinning across the lifecycle

<report spec="SPEC-0032">

## Outcome

delivered

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
Three Claude Code phase-worker subagent files ship at
`.claude/agents/speccy-tasks.md`, `.claude/agents/speccy-work.md`,
and `.claude/agents/speccy-ship.md`, each with
`model: sonnet[1m]` and `effort: medium`. No
`.claude/agents/speccy-init.md` ships (dropped under DEC-009 in
T-009). The four matching `.claude/skills/speccy-<phase>/SKILL.md`
files for `tasks`/`work`/`ship`/`init` carry no `context:`,
`agent:`, `model:`, or `effort:` keys — slash-command invocation
runs in the parent session. Three pinned-phase SKILL.md bodies
(`tasks`, `work`, `ship`) are thin stubs that name the matching
agent file and the `/agent speccy-<phase>` invocation pattern per
REQ-010; `/speccy-init`'s SKILL.md remains full-body. The pinned
phase-worker subagent template sources at
`resources/agents/.claude/agents/speccy-<phase>.md.tmpl` include
the shared phase body via
`{% include "modules/phases/speccy-<phase>.md" %}`. Pinned by
`speccy-core/tests/skill_stub_shape.rs` (stub-shape invariants,
description-prose drift) plus the existing host-pack drift check.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
`.claude/skills/speccy-review/SKILL.md` and its templated source
at `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`
carry no `model:`, `effort:`, `context:`, or `agent:` frontmatter
fields — only the pre-existing `name:` / `description:` pair. The
skill body describes the REQ-009 consolidation flow (sole-writer
discipline, fan-out parsing, serial state transitions). No file
exists at `.claude/agents/speccy-review.md` — the orchestrator
runs in the parent session at the parent session's model so it
retains Task-tool access for the reviewer fan-out and the parent
session's full context capacity for verdict consolidation. The
sibling Codex skill at `.agents/skills/speccy-review/SKILL.md`
mirrors the unpinned shape.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
The six Claude Code reviewer agent files at
`.claude/agents/reviewer-<persona>.md` carry asymmetric pins:
`reviewer-business`, `reviewer-tests`, `reviewer-architecture` at
`model: opus[1m]` / `effort: xhigh`; `reviewer-security` at
`model: sonnet[1m]` / `effort: high`; `reviewer-style`,
`reviewer-docs` at `model: sonnet[1m]` / `effort: medium`. Every
Claude Code reviewer pin uses the `[1m]` 1M-context-window
suffix. Reviewer bodies (below frontmatter) carry the REQ-009
verdict-return-contract edits and are otherwise byte-identical to
the pre-SPEC version. Matching templates under
`resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`
carry the same pin assignments. Pinned by
`speccy-core/tests/pin_shape.rs` (Opus/Sonnet alias regex, effort
enum membership, Sonnet-cannot-use-xhigh invariant).
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
Three new pinned Codex phase-worker TOML files ship at
`.codex/agents/speccy-tasks.toml`,
`.codex/agents/speccy-work.toml`, and
`.codex/agents/speccy-ship.toml`, each declaring
`model = "gpt-5.5"` and `model_reasoning_effort = "medium"`. No
`.codex/agents/speccy-init.toml` ships (dropped under DEC-009 in
T-009) and no `.codex/agents/speccy-review.toml` ships (the
orchestrator stays unpinned on Codex too). The six existing
Codex reviewer TOML files at `.codex/agents/reviewer-<persona>.toml`
carry `model = "gpt-5.5"` plus per-persona
`model_reasoning_effort`: `"high"` for `reviewer-business`,
`reviewer-tests`, `reviewer-architecture`; `"medium"` for
`reviewer-security`; `"low"` for `reviewer-style`,
`reviewer-docs`. The three pinned Codex phase-worker SKILL.md
bodies (`.agents/skills/speccy-<phase>/SKILL.md` for
`tasks`/`work`/`ship`) are thin stubs that name the matching
`.codex/agents/speccy-<phase>.toml` and the
`/agent speccy-<phase>` invocation per REQ-010. Pinned by
`speccy-core/tests/pin_shape.rs` (gpt-5.5 equality check,
effort-enum membership) and `skill_stub_shape.rs` (stub-shape
invariants on the Codex half).
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-005">
No long-form versioned model IDs (`claude-opus-`, `claude-sonnet-`,
`claude-haiku-`, dated `gpt-5.5-YYYY-MM-DD` strings) appear in any
shipped file under `resources/agents/`, `.claude/`, or `.codex/`.
No `haiku`-tier alias appears in any `model:` or `model` value.
Every Claude Code `model:` value matches the regex
`^(opus|sonnet)\[1m\]$`. Every Codex `model` value equals the
literal string `gpt-5.5`. Every Sonnet-pinned Claude Code file's
`effort:` is one of `low`/`medium`/`high`/`max` (never `xhigh`,
which is Opus-only). Every Opus-pinned Claude Code file's
`effort:` is one of `low`/`medium`/`high`/`xhigh`/`max`. Every
pinned Codex file's `model_reasoning_effort` is one of
`low`/`medium`/`high`/`xhigh`. Pinned by
`speccy-core/tests/pin_shape.rs` (executable lock on every
invariant above; bumped allowed-tier list to include `xhigh` for
Codex mid-T-006 after the project owner corrected the brainstorm's
allowed-tier claim).
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-006">
The existing host-pack drift check (`dogfood_outputs_match_committed_tree`)
asserts template-vs-rendered byte identity across
`.claude/`, `.codex/`, and `.agents/` and passes against the
post-SPEC workspace. The dogfood-pack invariant covers the
`speccy init` contract: a fresh `speccy init` run would produce
the same pin assignments the in-tree pack carries (Sonnet[1m]/
medium on the three Claude Code phase-worker agents; `gpt-5.5`
with effort-tier on Codex; the asymmetric reviewer pins; unpinned
SKILL.md files on the four mechanical phase slash-commands and on
`/speccy-review`; thin-stub SKILL.md bodies pointing at the
matching agent file on the three pinned phases). Pinned by
`speccy-cli/tests/init.rs::t007_init_renders_pin_assignments_into_fresh_directory`,
the host-pack drift check, and the rendered-pack tests under
`speccy-cli/tests/skill_packs.rs`.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-007">
`README.md` gained a new top-level `## Model pinning` section
that names: the pin assignment table across all five mechanical
phases (`tasks`, `work`, `ship`, `init`, `review`) and all six
reviewer personas (`reviewer-business`, `reviewer-tests`,
`reviewer-architecture`, `reviewer-security`, `reviewer-style`,
`reviewer-docs`) with Claude Code (`opus[1m]` / `sonnet[1m]` plus
`effort:` tier) and Codex (`gpt-5.5` plus reasoning-effort tier)
columns; the agent-file-existence column showing which phases
ship subagent files on each host; the opt-in `/agent speccy-<phase>`
invocation surface and the parent-session-by-default for slash
commands; the unpinned `/speccy-review` rationale (sole writer to
TASKS.md per REQ-009, needs parent session capacity); the user
override path with concrete examples (e.g. swap `sonnet[1m]` for
`claude-sonnet-4-6[1m]`); the alias-vs-snapshot rationale; the
auto-fork retreat as a design lesson. The rest of the README was
audited end-to-end: every `speccy <cmd>` mention names one of the
ten shipped commands; no prose presents `.speccy/skills/` as a
current user-facing path; no retired SPEC-0021 XML elements are
named; the Repo-layout block lists the new
`agents/speccy-{tasks,work,ship}.md`/`.toml` pinned phase-worker
files alongside the `reviewer-*` agents on both hosts. Per the
SPEC's "audit is one-time and not codified into a meta-test"
clause, future drift is caught by `reviewer-docs` at review time
rather than by CI.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-008">
Each of the three pinned Claude Code phase-worker agent templates
at `resources/agents/.claude/agents/speccy-<phase>.md.tmpl` for
`tasks`/`work`/`ship` includes the shared phase body via
`{% include "modules/phases/speccy-<phase>.md" %}`. The three
matching Codex TOML templates at
`resources/agents/.codex/agents/speccy-<phase>.toml.tmpl` include
the same shared body inside their `developer_instructions` value.
The shared phase-body sources moved from
`resources/modules/skills/` to `resources/modules/phases/` per
DEC-009 (T-009); the four shared body files at
`resources/modules/phases/speccy-<phase>.md` for
`tasks`/`work`/`ship`/`init` exist; the matching files under
`resources/modules/skills/` for the same four phases were
removed. No `resources/modules/agents/` directory was created.
The three pinned-phase SKILL.md template bodies on both hosts
carry hardcoded thin-stub prose and do not include any
`modules/phases/` directive — eliminating the byte-identical
SKILL.md/agent body duplication by construction. Pinned by
`speccy-core/tests/skill_stub_shape.rs` (the `modules/phases/`
rename invariant and the asymmetric-inclusion invariant) plus the
host-pack drift check.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-009">
Each of the six `resources/modules/personas/reviewer-<persona>.md`
bodies carries the verdict-return contract: the reviewer returns
its verdict to the orchestrator via its final message as a
`<review persona="..." verdict="...">` element block, and the body
explicitly forbids editing TASKS.md from inside the reviewer
subagent. The orchestrator body at
`resources/modules/skills/speccy-review.md` carries the
consolidation contract: it names the default four-persona fan-out
(business, tests, security, style) plus the two explicit-invoke
personas (architecture, docs), describes the verdict shape it
expects, and names the serial-write discipline that makes the
orchestrator the sole TASKS.md writer for review-induced state
transitions. No reviewer prompt or persona body instructs the
persona to write TASKS.md, and the parallel-write race that
arose under the prior shape is eliminated by construction. The
SPEC-0032 closure itself is a dogfood proof point: T-008's review
ran four reviewers in parallel and the orchestrator (this
session) wrote the `state="completed"` flip and the four
`<review>` blocks serially in a single turn with no torn file.
</coverage>

<coverage req="REQ-010" result="satisfied" scenarios="CHK-010">
The three rendered phase-worker SKILL.md files on each host
(`.claude/skills/speccy-<phase>/SKILL.md` and
`.agents/skills/speccy-<phase>/SKILL.md` for
`tasks`/`work`/`ship`) carry ≤10 non-blank content lines below
the YAML frontmatter delimiter, contain the literal substring
`/agent speccy-<phase>` for their matching phase, and reference
their matching agent file path (`.claude/agents/speccy-<phase>.md`
on Claude Code, `.codex/agents/speccy-<phase>.toml` on Codex).
The six pinned SKILL.md bodies are byte-shorter than the matching
agent bodies and contain no `## Steps` or `## When to use`
literals — the agent file is now the single on-disk source of
truth. The `speccy-init` agent file
(`.claude/agents/speccy-init.md` and its `.tmpl` source) was
removed and no `.codex/agents/speccy-init.toml` exists.
`/speccy-review`'s SKILL.md remains full-body per the REQ-002 and
REQ-009 carve-outs. The three remaining Claude Code phase-worker
agent `description:` values were rewritten to drop the literal
substring `` via `context: fork` `` and to drop references to
specific model or effort tier values. The shared body files
moved from `resources/modules/skills/speccy-<phase>.md` to
`resources/modules/phases/speccy-<phase>.md` for all four phase
workers; no files exist at the old paths for the same four phase
names. Pinned by
`speccy-core/tests/skill_stub_shape.rs` (stub-shape line count,
agent-vs-SKILL byte-length, description-prose drift, agent-file
absence, modules-rename invariant) and the host-pack drift check.
</coverage>

## Task summary

- Total tasks: 9 (T-001 Claude Code phase-worker agents; T-002
  Claude Code reviewer pins; T-003 REQ-009 verdict-return /
  consolidation contract; T-004 Codex phase-worker TOMLs;
  T-005 Codex reviewer pins; T-006 REQ-005 pin-shape meta-test;
  T-007 `speccy init` fresh-render verification; T-008 README
  Model pinning section + drift audit; T-009 SKILL stub dedup +
  `speccy-init` agent removal + `modules/phases/` rename +
  description-prose rewrite + stub-shape meta-test).
- Retried: 4 (T-001, T-003, T-007, T-009). T-001 retry narrowed
  scope after the third Changelog row reverted the
  `context: fork` SKILL.md edits; T-003 retry caught a
  `<review persona="manual-consolidation">` parse break in T-009's
  prior closure block (fixed inline to `business`); T-007 retry
  unstuck a stale evidence reference to a renamed test function;
  T-009 retry rewrote the TOML-template fix path after the
  retry guidance conflicted with the existing
  `t010_codex_reviewer_wrapper_shape_and_body` invariant
  (renderer-level fix landed in `speccy-cli/src/render.rs` rather
  than appending `\n` to template sources).
- SPEC amendments: 4 across the loop. (1) Initial draft from
  brainstorm. (2) Three coupled edits stripping Haiku entirely,
  absorbing F-10 (reviewer fan-out returns verdicts; orchestrator
  becomes sole TASKS.md writer; new REQ-009 + DEC-008), and
  tightening alias shape to `opus[1m]` / `sonnet[1m]` on Claude
  Code and `gpt-5.5` on Codex. (3) Dropping `context: fork` after
  T-002 surfaced the silent multi-minute UX cost on
  `/speccy-work`; the cost-and-time pin retreated to opt-in via
  `/agent speccy-<phase>`. (4) Stubbing per-phase SKILL.md bodies
  and dropping the `speccy-init` agent (new REQ-010 + DEC-009),
  renaming `resources/modules/skills/` to
  `resources/modules/phases/`, and rewriting the three remaining
  phase-worker agent `description:` values.

## Out-of-scope items absorbed

- Snapshot fixture for SPEC-0033 was added to
  `speccy-core/tests/fixtures/in_tree_id_snapshot.json` inline in
  T-001 to unblock the hygiene gate; the sibling WIP draft would
  otherwise have failed `every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot`
  on every task that ran the gate. Confirmed pre-existing on the
  WIP base commit (`4d18a51`) via `git stash` / re-run.
- Snapshot fixture entry plus 10 missing
  `</scenario>\n</requirement>` blank-line insertions for SPEC-0034
  were absorbed in T-004 for the same reason — the WIP base
  commit carried the pre-existing breakage and the hygiene gate
  could not pass against it without the fix.
- Pre-existing render-vs-committed drift on
  `.claude/agents/reviewer-docs.md` (missing REQ-003 pin keys and
  the REQ-009 verdict-return-contract body) blocked
  `dogfood_outputs_match_committed_tree` on the WIP base and was
  fixed in T-007 by deleting the committed file and re-running
  `speccy init --force --host claude-code`. Reviewer files are
  skip-on-exists under `--force`, so the standard refresh path
  could not regenerate the rendered output; the deletion +
  re-render is the documented escape hatch.
- REQ-005 allowed-tier list for Codex's `model_reasoning_effort`
  was corrected mid-T-006 after the project owner identified that
  `gpt-5.5` accepts `xhigh` (the brainstorm phase had recorded
  `low/medium/high` only). SPEC.md, T-006's `<task-scenarios>`,
  the SPEC's "verified facts" paragraph, and the test's
  allowed-list constant were updated in one pass; `spec_hash_at_generation`
  was bumped to match the new SPEC hash. Shipped Codex pins still
  use `low/medium/high` only — `xhigh` is forward-compatible
  headroom, not an active pin.
- Repo-layout code block in `README.md` under-described the
  post-SPEC-0032 host packs in T-008. CHK-007 implies the
  ejection-path reference must reflect current state, so the
  block was updated to list the three pinned phase-worker agent
  files alongside the existing `reviewer-*` entries on both
  hosts in the same edit pass.

## Skill updates

- `resources/modules/personas/reviewer-style.md` (plus its propagated
  rendered copies at `.claude/agents/reviewer-style.md` and
  `.codex/agents/reviewer-style.toml`) gained a new "Diff-format
  pitfalls" section in an orchestrator-led bypass during the T-009
  retry. The hardening landed because a prior reviewer-style turn
  produced a false-positive trailing-newline retry; the new
  guidance teaches the persona to read trailing-newline state from
  byte-level evidence rather than `git diff` markers, semantically
  describing the `\ No newline at end of file` marker without
  embedding the raw backslash glyph (which previously broke the
  Codex reviewer-style TOML's basic-string parse). The bypass is
  documented in detail in the T-009 retry implementer-note's
  `Procedural compliance` body and was applied on the trunk so
  every future `/speccy-review` invocation across every spec
  benefits.

## Deferred / known limitations

- The `skill_stub_shape.rs` meta-test does not assert the
  trailing-newline invariant directly — the style reviewer caught
  the T-009 retry regression via `.editorconfig` reading rather
  than a failing test. Hardening the meta-test to assert
  `last_byte == 0x0A` on the six stub outputs (and arguably on
  every rendered SKILL.md / agent file) would convert this class
  of regression from a reviewer-only catch into a CI-level one.
  Flagged in T-009's implementer note for a future amendment or
  follow-up SPEC; not load-bearing for SPEC-0032's contract.
- The `t004_codex_agent_dev_instructions_len` portion of CHK-010
  invariant (i) skips gracefully when the three pinned Codex
  phase-worker TOMLs are absent; T-004 brought those files into
  existence, so the check now activates as designed.
  Documented here for future contributors who may wonder why the
  skip path exists.
- The Codex reviewer pins ship at `low`/`medium`/`high` only;
  `xhigh` is allowed by the meta-test but not yet exercised. A
  future reviewer with a heavier work shape (or a Codex generation
  that benefits from `xhigh` on the existing personas) can
  re-pin file-by-file without a meta-test update.
- The auto-fork retreat described in the third Changelog row is
  preserved as a design lesson rather than a re-litigation lever.
  If a future Claude Code release ships a fork primitive that
  streams subagent tool output back to the parent session, a
  follow-up SPEC may revisit DEC-001; until then the opt-in
  `/agent speccy-<phase>` path is the v1 contract on both hosts.
- The conversational skills (`brainstorm`, `plan`, `amend`) are
  deliberately unpinned per DEC-005; users opening a session
  below the recommended Opus/xhigh tier will experience
  under-powered framing dialogue. The README's `## Model
  pinning` section names the recommended session model for the
  conversational phases; no per-skill prose addition was added.
- SPEC-0034 (`status: draft`) carries an invalid status value
  that causes `speccy_verify_exits_zero_on_migrated_in_tree_workspace`
  to fail on the WIP base commit. T-003 confirmed the failure
  pre-dates SPEC-0032's loop via stash/unstash; the fix belongs
  in SPEC-0034 itself, not here.

</report>
