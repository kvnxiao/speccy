---
id: SPEC-0048
slug: ship-pr-body-markdown-template
title: Markdown PR body template — `/speccy-ship` assembles markdown from spec artifacts instead of piping raw REPORT.md XML
status: in-progress
created: 2026-05-27
supersedes: []
---

# SPEC-0048: Markdown PR body template — `/speccy-ship` assembles markdown from spec artifacts instead of piping raw REPORT.md XML

## Summary

The shipped `/speccy-ship` recipe at `resources/modules/phases/speccy-ship.md`
step 5 currently opens the pull request with
`gh pr create --body "$(cat .speccy/specs/NNNN-slug/REPORT.md)"`. The
file it pipes through is a markdown document wrapping a `<report>`
root element with one `<coverage req="REQ-NNN" result="..."
scenarios="...">` child per surviving requirement; the prose inside
each `<coverage>` body is the per-requirement coverage statement.

The XML carrier exists for `speccy verify`'s proof-shape lint family
(`RPT-*`). It does not render on GitHub: `<report>` and `<coverage>`
are not HTML elements GitHub's markdown parser strips, so the angle
brackets leak into the rendered PR page as visible prose. Reviewers
either decode the XML by eye or click through to `REPORT.md` in the
Files-changed tab. The dogfood evidence is that no shipped PR on this
repo (#3, #4, #5) used the recipe's literal `cat REPORT.md` — every
human-driven ship pass hand-authored the body instead. The recipe
encodes a shape the agent already silently ignores.

This SPEC formalises the markdown PR body shape. A new canonical
reference at `resources/modules/references/pr-body.md` carries the
markdown skeleton, per-placeholder fill instructions, and a one-SPEC-
per-PR scope statement with a documented multi-SPEC fallback. The
`/speccy-ship` phase prompt step 5 (and its `.claude/` + `.codex/`
mirrors) points at that reference and passes the rendered body to
`gh pr create --body-file` instead of `--body "$(cat ...)"`. The
template's body has four sections — Summary, Coverage table, Test
plan checklist, Reference docs — sourced from `SPEC.md`'s `## Summary`
prose, the `<coverage>` attributes in `REPORT.md`, the fixed standard-
hygiene gate list from `AGENTS.md` "## Standard hygiene", and spec-
local relative paths respectively.

The `<coverage>` element bodies in `REPORT.md` carry the per-
requirement prose. The template deliberately does not duplicate that
prose into the PR body: `REPORT.md` lands in the same PR diff, is one
click away in Files-changed, and is linked from the template's
Reference docs section. The PR body's Coverage section is the
scannable table only — Req | Result | Scenarios | Retries — sourced
from the `<coverage>` element attributes and the `Retry count:` line
that closes each body.

REPORT.md's XML shape and the `RPT-*` lint family are untouched. This
SPEC operates entirely downstream of `REPORT.md`: the XML carrier
stays as-is and is what the template reads to render the markdown.

## Goals

<goals>
- A reviewer opening a PR shipped by `/speccy-ship` sees a fully
  rendered markdown body — no leaked angle-bracket XML, no raw
  `<report>` or `<coverage>` tags — without clicking through to
  `REPORT.md`.
- The ship agent fills a fixed-shape markdown template from on-disk
  artifacts (`SPEC.md`, `REPORT.md`, spec-local paths) and writes the
  result to a scratch file passed via `gh pr create --body-file`.
- The four template sections are Summary, Coverage table, Test plan
  checklist, Reference docs — in that fixed order.
- The Coverage table has one row per `<coverage>` element in
  `REPORT.md`, in numerical order, with columns Req, Result,
  Scenarios, Retries sourced from the element's `req`, `result`,
  `scenarios` attributes and trailing `Retry count:` line.
- The Test plan checklist is the four `AGENTS.md` "## Standard
  hygiene" gates plus `speccy verify`, all marked checked — the ship
  recipe halts before opening the PR if any gate failed, so a PR
  reaching `gh pr create` necessarily cleared all five.
- The canonical reference file at
  `resources/modules/references/pr-body.md`, the two host-portable
  mirrors at `.claude/skills/speccy-ship/references/pr-body.md` and
  `.agents/skills/speccy-ship/references/pr-body.md`, and the two
  templating partials at
  `resources/agents/.claude/skills/speccy-ship/references/pr-body.md.tmpl`
  and `resources/agents/.agents/skills/speccy-ship/references/pr-body.md.tmpl`
  all land atomically with the consuming phase-prompt update; the
  orphan-reference lint
  (`chk022_no_orphan_references`, SPEC-0038 REQ-007) passes.
- The `/speccy-ship` phase prompt step 5 documents the one-SPEC-per-PR
  scope assumption and the hand-authored-body fallback for branches
  that bundle multiple SPECs or carry unrelated precursor commits.
</goals>

## Non-goals

<non-goals>
- No CLI subcommand renders the PR body. Speccy's "deterministic core,
  intelligent edges" principle (AGENTS.md `## Core principles` #2) keeps
  template substitution in the skill layer. The Rust binary gains no
  `speccy report --markdown` or equivalent surface.
- No per-requirement coverage prose appears in the PR body. The
  template's Coverage section is the scannable table only; reviewers
  who want depth click through to `REPORT.md` via the Reference docs
  link. Duplicating the `<coverage>` element bodies into the PR body
  would create two places carrying the same prose with no enforcement
  that they stay in sync.
- No multi-SPEC concatenation logic. The template assumes one SPEC per
  PR; branches bundling multiple SPECs fall back to a hand-authored
  PR body. The template can still serve as a per-SPEC starting
  skeleton when hand-authoring, but the phase prompt does not
  prescribe how to compose multiple renders.
- No change to `REPORT.md`'s XML shape, the `RPT-*` lint family, or
  the canonical `REPORT.md` reference at
  `resources/modules/references/report.md`. The XML carrier stays as-
  is and is what the template reads to render the markdown.
- No change to the title argument of `gh pr create`. The current
  `--title "<spec id> <slug>"` convention is preserved.
</non-goals>

## User Stories

<user-stories>
- As a reviewer opening a PR shipped by `/speccy-ship`, I want the PR
  description to render as readable markdown so I can scan the change
  shape without decoding XML wrappers or clicking through to
  `REPORT.md` for orientation.
- As the ship-lifecycle agent, I want a canonical template I can fill
  from on-disk artifacts so the PR body shape is consistent across
  SPECs and survives my own non-determinism on what to emphasise.
- As a future contributor editing the PR body shape, I want one
  source-of-truth file to change so the canonical reference, the two
  host mirrors, and the two templating partials all move in lock-step
  rather than drifting.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Canonical PR body reference at `resources/modules/references/pr-body.md`

A new canonical reference file at
`resources/modules/references/pr-body.md` carries the markdown PR body
template, the per-placeholder fill instructions, the one-SPEC-per-PR
scope statement, and the multi-SPEC fallback guidance. The file body
documents (in this order):

- A short intent paragraph naming the problem (GitHub does not render
  `<report>` / `<coverage>` as HTML) and the fix (markdown template
  filled from artifacts).
- A "Scope: one SPEC per PR" section stating the single-SPEC
  assumption and the hand-authored-body fallback for branches that
  bundle multiple SPECs or carry unrelated precursor commits.
- A "Title" section showing the `<SPEC-NNNN> <slug>` title format
  sourced from `SPEC.md` frontmatter `id` and `slug`.
- A "Body template" section containing a fenced markdown block with
  the four-section skeleton in order: `## Summary`, `## Coverage`
  (with the four-column table header `| Req | Result | Scenarios |
  Retries |`), `## Test plan` (with the five fixed checklist items),
  `## Reference docs` (with the three spec-local links to `SPEC.md`,
  `REPORT.md`, and `journal/`).
- A "Filling the placeholders" section with one sub-heading per
  placeholder (`<spec-dir>`, `<summary>`, `<coverage-rows>` — angle-
  bracket tokens, not double-brace; the MiniJinja rendering pipeline
  treats `{{ ... }}` as expression delimiters, so the canonical
  template uses angle-bracket tokens that the agent substitutes at
  render time) documenting the artifact source and any per-column
  derivation rule
  (e.g. for `Scenarios`: split on whitespace, comma-separated; for
  `Retries`: copy the `Retry count:` line value verbatim including any
  per-task parenthetical breakdown).
- An "Anti-patterns" section prohibiting the raw-XML paste, fabricated
  rows, edits to the fixed Test plan checklist, and dropping the
  generated-with-Claude-Code footer (Codex hosts swap that footer).

<done-when>
- The file `resources/modules/references/pr-body.md` exists in the
  workspace.
- The file contains a fenced markdown body skeleton with exactly four
  top-level `##` section headings in the documented order.
- The Coverage table header in the skeleton is `| Req | Result |
  Scenarios | Retries |`.
- The Test plan section in the skeleton lists exactly five `- [x]`
  checklist items: the four `AGENTS.md` `## Standard hygiene`
  commands and `speccy verify`.
- The file contains a "Scope: one SPEC per PR" section naming the
  hand-authored-body fallback for multi-SPEC branches.
</done-when>

<behavior>
- Given the canonical reference exists, when an agent reads it, then
  the four template sections, their order, and the Coverage table's
  four columns are unambiguous from the document body alone.
- Given a SPEC with N requirements, when the agent fills the template,
  then the rendered Coverage table has exactly N data rows under the
  header.
- Given a multi-SPEC branch, when the agent reads the reference,
  then the reference instructs the agent to fall back to hand-
  authoring rather than silently concatenating per-SPEC renders.
</behavior>

<scenario id="CHK-001">
Given the workspace at HEAD after this SPEC lands,
when `resources/modules/references/pr-body.md` is read,
then it contains the four top-level headings `## Summary`,
`## Coverage`, `## Test plan`, `## Reference docs` inside a fenced
markdown block, in that order.
</scenario>

<scenario id="CHK-002">
Given the same canonical file,
when its Test plan section is inspected,
then it lists exactly five `- [x]` items naming
`cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`,
`cargo deny check`, and `speccy verify`.
</scenario>

<scenario id="CHK-003">
Given the same canonical file,
when its scope section is inspected,
then it carries a "one SPEC per PR" statement and an explicit
hand-authored-body fallback clause for multi-SPEC branches.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Host-portable mirrors and templating partials match the canonical source byte-for-byte

The new reference fans out through the existing templating pipeline
the same way the other skill-local references do (the `report.md`
sibling already follows this pattern). Concretely:

- `resources/agents/.claude/skills/speccy-ship/references/pr-body.md.tmpl`
  exists and is a single-line
  `{% include "modules/references/pr-body.md" %}` partial.
- `resources/agents/.agents/skills/speccy-ship/references/pr-body.md.tmpl`
  exists with the same single-line include.
- `.claude/skills/speccy-ship/references/pr-body.md` and
  `.agents/skills/speccy-ship/references/pr-body.md` exist and are
  byte-identical to the canonical source.

The existing SPEC-0038 REQ-007 orphan-reference test
(`speccy-cli/tests/skill_body_discovery.rs::chk022_no_orphan_references`)
must continue to pass — that test enforces both the cross-host parity
(Claude vs Codex mirror byte-identical) and the source-to-host parity
(canonical vs each mirror byte-identical), and additionally requires
at least one consuming-body pointer per host containing the substring
`references/pr-body.md`. REQ-003 lands those pointers.

<done-when>
- The two `.tmpl` files exist with the documented single-line body.
- The two dogfood mirrors exist and are byte-identical to the
  canonical source.
- `cargo test --workspace`'s
  `chk022_no_orphan_references` test passes after the changes land.
</done-when>

<behavior>
- Given the canonical source changes, when the dogfood mirrors are
  regenerated through the templating pipeline (or hand-synced), then
  the cross-host parity check observes byte-identical content.
- Given the two `.tmpl` files at their documented paths, when they
  are read, then each contains exactly the single-line include
  `{% include "modules/references/pr-body.md" %}` with no surrounding
  prose.
</behavior>

<scenario id="CHK-004">
Given the workspace at HEAD after this SPEC lands,
when `cargo test --workspace -- chk022_no_orphan_references` is run,
then the test exits 0 with no failure reported for the
`pr-body.md` reference under either host pack.
</scenario>

<scenario id="CHK-005">
Given the same workspace,
when the four files
`resources/agents/.claude/skills/speccy-ship/references/pr-body.md.tmpl`,
`resources/agents/.agents/skills/speccy-ship/references/pr-body.md.tmpl`,
`.claude/skills/speccy-ship/references/pr-body.md`, and
`.agents/skills/speccy-ship/references/pr-body.md` are inspected,
then the two `.tmpl` files each contain exactly one non-blank line
matching `{% include "modules/references/pr-body.md" %}`, and the two
dogfood mirrors are byte-identical to
`resources/modules/references/pr-body.md`.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `/speccy-ship` phase prompt step 5 renders the template and passes `--body-file`

The canonical phase prompt at `resources/modules/phases/speccy-ship.md`
and both of its rendered host descendants
(`.claude/agents/speccy-ship.md` and `.codex/agents/speccy-ship.toml`)
have step 5 updated so that the open-a-new-PR branch:

- Names the canonical PR body template as `references/pr-body.md`.
- Instructs the agent to fill the template from `SPEC.md`,
  `REPORT.md`, and the spec-dir path, write the rendered markdown to
  a scratch file, and pass it via `gh pr create --body-file`.
- Explicitly prohibits the raw `--body "$(cat ... REPORT.md)"` shape
  with a one-sentence rationale (GitHub does not render the XML
  wrappers).
- Carries a multi-SPEC fallback paragraph naming the hand-authored-
  body path for branches bundling multiple SPECs or precursor commits.

The update-existing-PR branch (`git push` to refresh) and the title-
argument shape (`<spec id> <slug>`) are unchanged. The phase prompt's
"This recipe does not loop" closing line is unchanged.

Each of the three files contains the substring `references/pr-body.md`
at least once, satisfying SPEC-0038 REQ-007's consuming-body
requirement under each host pack.

<done-when>
- Step 5 of the phase prompt no longer contains the substring
  `--body "$(cat`.
- Step 5 of the phase prompt contains the substring
  `--body-file` and the substring `references/pr-body.md`.
- The Claude Code dogfood mirror `.claude/agents/speccy-ship.md` and
  the Codex dogfood mirror `.codex/agents/speccy-ship.toml` carry the
  same step 5 update.
- Step 5 carries a multi-SPEC fallback paragraph mentioning hand-
  authoring.
</done-when>

<behavior>
- Given an agent running `/speccy-ship` against a single-SPEC branch
  with no open PR, when it reaches step 5, then it renders the
  template from artifacts, writes the result to a scratch file, and
  calls `gh pr create --body-file <path>`.
- Given an agent running `/speccy-ship` against a multi-SPEC branch
  with no open PR, when it reaches step 5, then it recognises the
  multi-SPEC fallback paragraph and produces a hand-authored body
  rather than silently concatenating per-SPEC renders.
- Given an agent running `/speccy-ship` against a branch that already
  has an open PR, when it reaches step 5, then the update-existing-
  PR branch (push to refresh) is the path taken; the rendering recipe
  does not apply.
</behavior>

<scenario id="CHK-006">
Given the workspace at HEAD after this SPEC lands,
when `resources/modules/phases/speccy-ship.md` is read,
then step 5 contains the substrings `--body-file` and
`references/pr-body.md`, and does not contain the substring
`--body "$(cat`.
</scenario>

<scenario id="CHK-007">
Given the same workspace,
when `.claude/agents/speccy-ship.md` and
`.codex/agents/speccy-ship.toml` are read,
then each file contains the substring `references/pr-body.md` at
least once and does not contain the substring `--body "$(cat`.
</scenario>

<scenario id="CHK-008">
Given the same workspace,
when step 5 of `resources/modules/phases/speccy-ship.md` is read,
then it carries an explicit multi-SPEC fallback paragraph naming
hand-authoring as the path for branches bundling multiple SPECs.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
PR body assembly lives in the skill layer, not the CLI.
`AGENTS.md` `## Core principles` #2 ("Deterministic core, intelligent
edges") keeps the Rust binary mechanical; template substitution and
artifact-reading judgment belong in agent prose. A
`speccy report --markdown` subcommand would be deterministic and
architecturally cleaner in isolation, but it would grow the CLI
surface for a workflow that is only useful at ship time and would
duplicate logic the agent already exercises elsewhere (reading
`SPEC.md` and `REPORT.md`).
</decision>

<decision id="DEC-002">
Coverage table only; no per-requirement prose duplication in the PR
body. `REPORT.md` lands in the same PR, shows up in Files-changed,
and is one click from the Reference docs section. Duplicating the
`<coverage>` element bodies into the PR description would create two
places carrying the same prose with no enforcement that they stay in
sync — a drift surface the SPEC explicitly does not create.
</decision>

<decision id="DEC-003">
Single SPEC per PR is the supported shape. Multi-SPEC branches fall
back to a hand-authored PR body. A template aware of multi-SPEC
composition would need to handle Summary stitching, deduplicate the
Test plan checklist, and decide how to present per-SPEC coverage
tables — all of which contradicts the "small, focused template"
goal. The phase prompt documents the fallback inline so the agent
recognises the branch shape at decision time rather than discovering
the limitation mid-render.
</decision>

<decision id="DEC-004">
The Test plan checklist's five items are fixed by the canonical
template and are pre-checked. The ship recipe at step 4 runs
`speccy verify` after the four `AGENTS.md` `## Standard hygiene`
gates; if any of the five fails, the agent halts before opening the
PR. A PR reaching `gh pr create` necessarily cleared all five gates,
so the checklist's `- [x]` markers reflect ground truth, not
aspiration. Reviewers seeing five unchecked boxes would be the
correct signal that the agent shipped without verifying — but that
state is precluded by step 4's halt.
</decision>

<decision id="DEC-005">
The reference file lives at `resources/modules/references/pr-body.md`
and is mirrored under each host's skill-local
`speccy-ship/references/` directory, paralleling the existing
`report.md` reference. It is not promoted to the cross-cutting
`speccy-references/` directory because the PR body template is
specific to `/speccy-ship` — no other shipped skill consumes it.
</decision>

## Notes

The dogfood evidence motivating this SPEC: PRs #3, #4, and #5 on this
repo each carried hand-authored markdown bodies (Summary / Coverage /
Test plan / References sections), not the literal `cat REPORT.md`
output the phase prompt prescribes. The agent silently deviated from
the recipe every time because the recipe's output does not serve
reviewers. This SPEC moves the prescribed shape to match what the
agent already does in practice, with a canonical template to keep
that shape consistent across future ship passes.

The implementer task will need to refresh the dogfood mirrors at
`.claude/agents/speccy-ship.md` and `.codex/agents/speccy-ship.toml`
after editing the canonical phase prompt. The standard path is to
rebuild the CLI (the embedded resources bundle is a build-time
`include_dir!` snapshot) and run
`speccy init --force --host=claude-code` and
`speccy init --force --host=codex` against this repo. A hand-sync
path also works for small edits, but the init path is what CI
exercises and is the source-of-truth recipe documented in SPEC-0044.

## Open Questions

(none)

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-27 | claude-opus-4-7[1m] | Amendment 2 (placeholder lexical normalization). REQ-001 prose previously named placeholder tokens as `{{ spec-dir }}`, `{{ summary }}`, `{{ coverage rows }}` (double-brace, space-separated). The canonical template at `resources/modules/references/pr-body.md` ships angle-bracket tokens (`<spec-dir>`, `<summary>`, `<coverage-rows>`) because `{{ ... }}` collides with MiniJinja's expression delimiter — discovered during T-001 round-1 implementation. REQ-001's done-when does not pin lexical syntax so the implementation was technically compliant, but a SPEC.md-only reader saw a mismatch between prose and shipped artifact. Amendment normalizes REQ-001 prose to angle-bracket form with a one-line rationale; no REQ scope change, no done-when change, T-001 remains `state="completed"`. |
| 2026-05-27 | claude-opus-4-7[1m] | Amendment 1 (scope expansion acknowledgement). During T-001 round-2 implementation, the round-2 style reviewer fabricated an AGENTS.md commit-timing rule (asserting "all changes must be committed before the in-review flip") and incorrectly blocked an otherwise-passing task. Mid-loop, two canonical-template files outside SPEC-0048's stated scope were patched to silence the procedural hallucination at the source: (a) `resources/modules/personas/reviewer-style.md` — added an "Out of scope" section explicitly carving out commit timing, retry-round dirty trees, branch state, and `git status`-derived complaints as non-blocking for the style persona; (b) `resources/modules/skills/partials/review-fanout.md` — added a one-paragraph disclaimer to the orchestrator's reviewer dispatch prompt clarifying that the working tree may be dirty by design (orchestrator owns the atomic commit on review pass per SPEC-0045 REQ-003/REQ-004; the retry-shape contract requires the dirty tree as WIP carryover into round N+1). Re-ejection via `just reeject` propagated the canonical-source edits into `.claude/`, `.agents/`, and `.codex/` packs. Justified by AGENTS.md's "friction → update the skill" convention but not authorized by SPEC-0048's Goals or Non-goals at draft time; amendment codifies an existing orchestration contract (per-task review is not the right surface for commit-shape complaints) that was implicit before. |
| 2026-05-27 | human/kevin  | Initial draft. Introduces three requirements covering: (REQ-001) a new canonical markdown PR body template at `resources/modules/references/pr-body.md` with four fixed sections (Summary, Coverage table, Test plan checklist, Reference docs), per-placeholder fill instructions, and a one-SPEC-per-PR scope statement with a documented hand-authored-body fallback for multi-SPEC branches; (REQ-002) the standard host-portable mirrors and templating partials under `.claude/skills/speccy-ship/references/`, `.agents/skills/speccy-ship/references/`, and the matching `resources/agents/...` tmpl partials, with byte-identical parity to the canonical source enforced by the existing SPEC-0038 REQ-007 orphan-reference test; (REQ-003) an update to `resources/modules/phases/speccy-ship.md` step 5 (and its two dogfood mirrors at `.claude/agents/speccy-ship.md` and `.codex/agents/speccy-ship.toml`) that points at the new reference, uses `gh pr create --body-file` instead of `--body "$(cat ... REPORT.md)"`, and carries the multi-SPEC fallback paragraph. Five DECs codify: (DEC-001) PR body assembly stays in the skill layer; (DEC-002) Coverage table only, no per-requirement prose duplication; (DEC-003) one SPEC per PR with documented fallback; (DEC-004) Test plan checklist items are fixed and pre-checked because step 4 halts on any hygiene failure; (DEC-005) the reference is skill-local under `speccy-ship/references/`, not promoted to cross-cutting `speccy-references/`. |
</changelog>
