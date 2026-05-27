---
spec: SPEC-0048
spec_hash_at_generation: 7f7253b4d707dd60d1ee2be0c5416bb44c6e727da6c22097a8dc21b0553debd0
generated_at: 2026-05-27T06:04:42Z
---
# Tasks: SPEC-0048 Markdown PR body template — `/speccy-ship` assembles markdown from spec artifacts instead of piping raw REPORT.md XML

<task id="T-001" state="pending" covers="REQ-001 REQ-002 REQ-003">
## Author the canonical `pr-body.md` reference, fan out the host mirrors, and rewire `/speccy-ship` step 5 to render the template

Land the canonical reference, its templating partials, both host-portable
mirrors, and the consuming phase-prompt + agent-prompt edits in a single
commit. The SPEC-0038/REQ-007 `chk022_no_orphan_references` lint rejects a
reference file landing without a consuming-body pointer in the same host
pack, so the consumers (phase prompt and both agent prompts) must land
atomically with the reference. This mirrors the SPEC-0047 T-001 pattern
(`retry-shape.md` shipped with its first consumer in one commit).

### Part A — canonical reference file

Create `resources/modules/references/pr-body.md` with the body shape
prescribed by REQ-001:

- Short intent paragraph naming the GitHub-XML-rendering problem and
  the markdown-template fix.
- `## Scope: one SPEC per PR` section carrying the single-SPEC
  assumption and the hand-authored-body fallback clause for multi-SPEC
  branches.
- `## Title` section showing the title argument shape sourced from
  SPEC.md frontmatter `id` and `slug`.
- `## Body template` section containing one fenced markdown block whose
  body has exactly four top-level `##` headings in this order:
  `## Summary`, `## Coverage`, `## Test plan`, `## Reference docs`.
  - The Coverage section opens with the table header
    `| Req | Result | Scenarios | Retries |` followed by the
    `|---|---|---|---|` separator and a `{{ coverage rows }}`
    placeholder line.
  - The Test plan section is exactly five `- [x]` checklist items:
    `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    `cargo +nightly fmt --all --check`,
    `cargo deny check`,
    `speccy verify`. (Sourced verbatim from AGENTS.md `## Standard
    hygiene` plus the ship-recipe step 4 gate.)
  - The Reference docs section carries three bullet links to the
    spec-local `SPEC.md`, `REPORT.md`, and `journal/` paths.
  - The footer line is the `🤖 Generated with [Claude Code]` attribution.
- `## Filling the placeholders` section with one sub-heading per
  placeholder (`{{ spec-dir }}`, `{{ summary }}`, `{{ coverage rows }}`)
  naming the artifact source and any per-column derivation rule.
- `## Anti-patterns` section listing: no raw-XML paste, no fabricated
  rows, no edits to the fixed Test plan checklist, no dropping the
  generated-with-Claude-Code footer (Codex hosts swap it for the Codex
  CLI equivalent).

### Part B — host-portable fan-out

Create the two single-line `MiniJinja` partials so the templating
pipeline produces the dogfood mirrors:

- `resources/agents/.claude/skills/speccy-ship/references/pr-body.md.tmpl`
  containing exactly one non-blank line:
  `{% include "modules/references/pr-body.md" %}`.
- `resources/agents/.agents/skills/speccy-ship/references/pr-body.md.tmpl`
  with the same one-line body.

Sync the dogfood mirrors so they are byte-identical to the canonical
source (the existing pattern: every other reference file under
`.claude/skills/*/references/` and `.agents/skills/*/references/` is
byte-identical to its canonical source per the
`chk022_no_orphan_references` source-to-host parity check):

- `.claude/skills/speccy-ship/references/pr-body.md`
- `.agents/skills/speccy-ship/references/pr-body.md`

Two sync paths are acceptable: rebuild the speccy CLI (the embedded
`include_dir!` bundle picks up the new resources) and run
`speccy init --force --host=claude-code` followed by
`speccy init --force --host=codex` against this repo; or hand-copy
the canonical bytes into both mirror paths. The CI exercises the init
path, but a hand-sync that produces byte-identical files satisfies the
same invariant. Either way, the orphan-reference test asserts byte-
identity, so a stale mirror fails the suite loudly.

### Part C — phase prompt and agent prompt updates (REQ-003 consumers)

Update step 5 of `resources/modules/phases/speccy-ship.md` so the
"open a new PR" branch:

- Names `references/pr-body.md` as the canonical PR body template.
- Instructs the agent to fill the template from `SPEC.md`,
  `REPORT.md`, and the spec-dir path, write the rendered markdown to a
  scratch file, and call
  `gh pr create --title "<spec id> <slug>" --body-file <path>`.
- Explicitly prohibits the
  `--body "$(cat .speccy/specs/NNNN-slug/REPORT.md)"` shape with a
  one-sentence rationale (GitHub does not render the `<report>` /
  `<coverage>` XML wrappers as markdown).
- Carries a multi-SPEC fallback paragraph: branches bundling multiple
  SPECs or carrying unrelated precursor commits fall back to a hand-
  authored body; the template can serve as a per-SPEC starting
  skeleton when hand-authoring, but the recipe does not prescribe
  multi-SPEC composition.

The update-existing-PR branch (`git push` to refresh) and the title
argument shape are unchanged. The phase prompt's closing
"This recipe does not loop." line is unchanged.

Apply the same step 5 update verbatim (modulo the `{{ cmd_prefix }}`
substitution that produces `/speccy-` vs bare `speccy-` references) to
both dogfood mirrors:

- `.claude/agents/speccy-ship.md`
- `.codex/agents/speccy-ship.toml`

After all three sites carry the update, each contains the substring
`references/pr-body.md` at least once, satisfying the
`chk022_no_orphan_references` consuming-body requirement under each
host pack.

### Hygiene + verification

Run the full standard hygiene suite plus `speccy verify` before
flipping to `in-review`:

- `cargo test --workspace` (must include
  `chk022_no_orphan_references` passing for both hosts).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo +nightly fmt --all --check`.
- `cargo deny check`.
- `speccy verify` reporting `0 errors` (the SPC-* lint family treats
  this SPEC as well-formed; the new RPT- and TSK- lint families do not
  apply here — this SPEC ships no REPORT.md and the TASKS.md is the
  one in this file).

<task-scenarios>
Given the workspace at HEAD after this task,
when `resources/modules/references/pr-body.md` is read,
then it contains inside one fenced markdown block the four top-level
headings `## Summary`, `## Coverage`, `## Test plan`,
`## Reference docs` in that order; the Test plan section lists exactly
five `- [x]` items (`cargo test --workspace`, the clippy line, the
nightly fmt line, `cargo deny check`, `speccy verify`); the Scope
section names the hand-authored-body fallback for multi-SPEC branches.
(Covers CHK-001, CHK-002, CHK-003.)

Given the same workspace,
when `cargo test --workspace -- chk022_no_orphan_references` runs,
then the test exits 0 and reports no failure for `pr-body.md` under
either the Claude Code or Codex host pack; the two `.tmpl` partials
contain exactly the single-line include
`{% include "modules/references/pr-body.md" %}`; the two dogfood
mirrors are byte-identical to
`resources/modules/references/pr-body.md`. (Covers CHK-004, CHK-005.)

Given the same workspace,
when `resources/modules/phases/speccy-ship.md`,
`.claude/agents/speccy-ship.md`, and `.codex/agents/speccy-ship.toml`
are read,
then step 5 of each contains the substring `--body-file` and the
substring `references/pr-body.md`, and none of the three files
contains the substring `--body "$(cat`; step 5 of the canonical phase
prompt carries an explicit multi-SPEC fallback paragraph naming hand-
authoring. (Covers CHK-006, CHK-007, CHK-008.)

Suggested files: `resources/modules/references/pr-body.md` (new),
`resources/agents/.claude/skills/speccy-ship/references/pr-body.md.tmpl` (new),
`resources/agents/.agents/skills/speccy-ship/references/pr-body.md.tmpl` (new),
`.claude/skills/speccy-ship/references/pr-body.md` (new),
`.agents/skills/speccy-ship/references/pr-body.md` (new),
`resources/modules/phases/speccy-ship.md`,
`.claude/agents/speccy-ship.md`,
`.codex/agents/speccy-ship.toml`
</task-scenarios>
</task>
