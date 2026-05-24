---
spec: SPEC-0036
spec_hash_at_generation: 6c924942124c709bd716935c028def8e28ab4d61813eadd21e0e3c36635e199b
generated_at: 2026-05-21T03:33:41Z
---

# Tasks: SPEC-0036 Repin Claude Code speccy-work implementer to opus[1m] / low effort


<task id="T-001" state="completed" covers="REQ-001">
## T-001: Repin Claude Code `speccy-work` agent frontmatter to `opus[1m]` / `low`

Flip the `model:` and `effort:` YAML frontmatter values on the
Claude Code `speccy-work` agent from `sonnet[1m]` / `medium` to
`opus[1m]` / `low`. The edit lands in lockstep across the
templated source and the in-tree dogfood pack so the existing
host-pack drift-check meta-test stays green.

Two files change:

- `resources/agents/.claude/agents/speccy-work.md.tmpl` (template
  source): replace `model: sonnet[1m]` with `model: opus[1m]` and
  `effort: medium` with `effort: low`. The body (the single
  `{% include "modules/phases/speccy-work.md" %}` line) is
  unchanged.
- `.claude/agents/speccy-work.md` (rendered in-tree dogfood file):
  same two-line frontmatter swap. The body (the rendered include
  output, every byte below the closing `---` frontmatter
  delimiter) is unchanged.

Both files keep their existing `name: speccy-work` and
`description:` fields byte-identical. No other frontmatter key is
added or removed. No other agent file is touched: `speccy-tasks`,
`speccy-ship`, `speccy-init`, and every `reviewer-*` agent on
either host stay at their SPEC-0032 pins.

The Codex side (`.codex/agents/speccy-work.toml` and its template
source) is explicitly out of scope per the SPEC's non-goals — do
not edit it.

Run all four hygiene gates after the edit: `cargo test
--workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo +nightly fmt --all
--check`, `cargo deny check`. The host-pack drift-check meta-test
under `speccy-cli/tests/` must remain green because the template
and the rendered file move together.

Suggested files:

- `resources/agents/.claude/agents/speccy-work.md.tmpl`
- `.claude/agents/speccy-work.md`

<task-scenarios>
Given `.claude/agents/speccy-work.md` after this task lands, when
its YAML frontmatter is parsed, then `model` equals the literal
string `opus[1m]` and `effort` equals the literal string `low`.

Given `resources/agents/.claude/agents/speccy-work.md.tmpl` after
this task lands, when its YAML frontmatter is parsed, then
`model` equals `opus[1m]` and `effort` equals `low`.

Given each of the two edited files, when the YAML frontmatter is
parsed, then the `name:` value remains `speccy-work` and the
`description:` value is byte-identical to its pre-SPEC content
(only `model:` and `effort:` lines change).

Given each of the two edited files, when the body content
(everything below the second `---` frontmatter terminator) is
diffed against the pre-SPEC version, then the diff is empty.

Given the existing in-tree host-pack drift-check meta-test under
`speccy-cli/tests/init.rs` (the `dogfood_outputs_match_committed_tree`
test or its equivalent guard), when run after this task lands,
then it exits 0.

Given the four hygiene gates (`cargo test --workspace`, `cargo
clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check`) run
against the working tree at the commit that lands this task,
when each exits, then each exit code is 0.

Given the Codex `speccy-work` files at
`.codex/agents/speccy-work.toml` and
`resources/agents/.codex/agents/speccy-work.toml.tmpl`, when
diffed against their pre-SPEC contents, then the diff is empty
(this task does not touch the Codex side).
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-002">
## T-002: Update README pin-assignment table row and `speccy-work` override example

Update the project `README.md` so it describes the post-SPEC
shipped state of the Claude Code `speccy-work` pin. Two surgical
edits:

- In the "Pin assignment" table under `## Model pinning`, change
  the row whose first column is `speccy-work` so the Claude Code
  column reads `model: opus[1m]`, `effort: low`. Leave the Codex
  column (`model = "gpt-5.5"`, reasoning effort medium) and the
  "Agent file ships?" column (`yes`) unchanged. Every other row
  in the table (`speccy-tasks`, `speccy-ship`, `speccy-init`,
  `speccy-review`, and all six `reviewer-*` rows) stays exactly
  as it shipped before this SPEC.
- In the "Overriding a pin" section, the `speccy-work` worked
  example that today reads "Lock `speccy-work` to a specific
  Claude version for reproducibility: change
  `model: sonnet[1m]` to `model: claude-sonnet-4-6[1m]` in
  `.claude/agents/speccy-work.md`" must update so the "before"
  value matches the new shipped frontmatter: name
  `model: opus[1m]` as the "before" alias and a Claude Opus
  snapshot ID (e.g. `claude-opus-4-7[1m]`) as the lock target.
  Equivalent prose is fine as long as the "before" value matches
  the new shipped frontmatter and the override target references
  an Opus snapshot rather than a Sonnet snapshot.

After the edits, the README must contain zero lines that pair
`speccy-work` with `sonnet[1m]` (or with `effort: medium`) on the
same line, outside of any historical context already living
inside `.speccy/specs/0032-*/` (out of scope for this README
edit).

No other README section changes. No frontmatter, no schema, no
CLI surface, no module body. The two prose-language touches are
the whole edit.

Run all four hygiene gates after the edit. The README is
markdown, so the gates are unaffected by its content, but the
project convention is that every commit runs them.

Suggested files:

- `README.md`

<task-scenarios>
Given `README.md` after this task lands, when the "Pin
assignment" table is parsed and the row whose first column is
`speccy-work` is read, then the Claude Code column contains the
literal substrings `opus[1m]` and `low`, and contains neither
`sonnet[1m]` nor `medium` on that row.

Given the same `README.md`, when grepped for any line containing
both the literal substring `speccy-work` and the literal
substring `sonnet[1m]`, then zero matches are found.

Given the same `README.md`, when the "Overriding a pin" section
is read, then the `speccy-work` worked example names
`model: opus[1m]` as the "before" alias and a Claude Opus
snapshot ID (any member of the `claude-opus-*` family) as the
lock target.

Given the same `README.md`, when the "Pin assignment" table's
other ten rows (`speccy-tasks`, `speccy-ship`, `speccy-init`,
`speccy-review`, `reviewer-business`, `reviewer-tests`,
`reviewer-architecture`, `reviewer-security`, `reviewer-style`,
`reviewer-docs`) are diffed against their pre-SPEC contents,
then the diff is empty for those rows.

Given the same `README.md`, when the `speccy-work` row's Codex
column is read, then it contains `gpt-5.5` and `reasoning effort
medium` (unchanged from pre-SPEC).

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit
code is 0.
</task-scenarios>
</task>

