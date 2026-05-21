---
spec: SPEC-0036
spec_hash_at_generation: 6c924942124c709bd716935c028def8e28ab4d61813eadd21e0e3c36635e199b
generated_at: 2026-05-21T03:33:41Z
---

# Tasks: SPEC-0036 Repin Claude Code speccy-work implementer to opus[1m] / low effort

<tasks spec="SPEC-0036">

<task id="T-001" state="in-progress" covers="REQ-001">
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

<implementer-note session="2026-05-21">
Completed: Swapped `model: sonnet[1m]` → `model: opus[1m]` and `effort: medium` → `effort: low` in both `resources/agents/.claude/agents/speccy-work.md.tmpl` and `.claude/agents/speccy-work.md`. Updated two test files (`speccy-cli/tests/init.rs` and `speccy-cli/tests/init_phase_agents.rs`) that had hardcoded `sonnet[1m]`/`medium` assertions covering all three pinned phases uniformly; refactored each to check per-phase model/effort pairs so `speccy-work` is validated at `opus[1m]`/`low` while `speccy-tasks` and `speccy-ship` remain at `sonnet[1m]`/`medium`.

Undone: Nothing left undone. All scenarios in CHK-001, CHK-002, and CHK-003 are satisfied.

Commands run:
- `cargo test --workspace` (twice, once to discover failing tests, once after fixes)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check` (twice, once to discover formatter diffs, once after applying them)
- `cargo deny check`
- `cargo run -q -p speccy-cli -- check SPEC-0036/T-001`

Exit codes: all 0.

Discovered issues: Two test files contained uniform-loop assertions that assumed all three Claude Code pinned-phase agents share the same model/effort. The SPEC-0036 TASKS.md description mentioned only editing the two agent files, not the tests; the tests broke immediately because the dogfood drift-check test (`dogfood_outputs_match_committed_tree`) itself passed (templates and rendered files are in sync) but the pin-assertion tests encoded the old values. Both test files were updated as part of this task per the "run all four hygiene gates" requirement.

Procedural compliance: No stale skill instructions encountered. No skill files required updating.
</implementer-note>

<review persona="business" verdict="pass">
T-001 satisfies REQ-001 cleanly: both `.claude/agents/speccy-work.md` and `resources/agents/.claude/agents/speccy-work.md.tmpl` now declare `model: opus[1m]` / `effort: low`, with `name:`/`description:` and bodies byte-unchanged (verified via `git diff HEAD`). CHK-001 / CHK-002 / CHK-003 all hold. Non-goals respected: Codex pin untouched, `speccy-tasks` and `speccy-ship` stay on `sonnet[1m]`/`medium`, reviewer pins untouched, no skill-body or shared-phase-body edits, no schema or CLI surface change. The two test-file edits at `speccy-cli/tests/init.rs:1540` and `speccy-cli/tests/init_phase_agents.rs:140` are scope-justified — they refactor uniform-loop assertions that encoded the old pin into per-phase tuples, which is required for the SPEC's Goal #6 hygiene-gate criterion to hold; the implementer surfaced this under "Discovered issues" rather than silently expanding scope. The two open questions (Codex parallel repin, `low` vs `medium`) remain unchecked, matching the SPEC's recommendation.
</review>

<review persona="tests" verdict="blocking">
Test refactors in `speccy-cli/tests/init.rs:1543-1569` and `speccy-cli/tests/init_phase_agents.rs:143-164` correctly tighten the invariant from a single uniform tuple to a per-phase `(phase, model, effort)` table — `speccy-work` is locked at `opus[1m]`/`low` and the other two pinned phases at `sonnet[1m]`/`medium`. I ran each of the three relevant tests locally (`t007_init_renders_claude_code_pin_assignments_matching_dogfood_pack`, `phase_worker_agent_has_model_and_effort_frontmatter`, `dogfood_outputs_match_committed_tree`) and all three pass. A hypothetical regression of any single phase back to a wrong pin would now be caught individually — the refactor strengthens, not weakens, the assertion surface.

Blocking on missing red-then-green evidence. The `<implementer-note session="2026-05-21">` on `T-001` has no `Evidence:` field, and no evidence file exists at the conventional path `.speccy/specs/0036-speccy-work-opus-low-pin/evidence/T-001.md` (the spec directory contains only `SPEC.md` and `TASKS.md`). Per the SPEC-0031 paper-trail convention every other recent spec follows (0031-0035 all carry `Evidence:` pointers to per-task files), the absence is itself the blocking signal — there is no captured runner output proving the two test files transitioned from red (under the new agent-frontmatter values) to green after the refactor. The implementer-note's narrative claim that `cargo test --workspace` was run "twice, once to discover failing tests, once after fixes" is exactly the kind of transition that needs a captured artifact. Please add `Evidence: .speccy/specs/0036-speccy-work-opus-low-pin/evidence/T-001.md` to the implementer-note and write that file with the scoped per-test invocations showing both the red phase (test names from the diff failing before the refactor) and the green phase (same test names passing after).
</review>

<review persona="security" verdict="pass">
No security-relevant changes. This task is a two-line YAML frontmatter swap (`model: sonnet[1m]` → `model: opus[1m]`, `effort: medium` → `effort: low`) in two static markdown/template files, with matching test assertion updates. The speccy CLI does not parse or act on `model`/`effort` frontmatter; those fields are consumed solely by Claude Code's agent dispatch. No auth boundaries, injection surfaces, secret handling, sensitive data exposure, or cryptographic parameters are affected.
</review>

<review persona="style" verdict="blocking">
All four files changed by T-001 are unstaged local edits — none of the implementation has been committed to git. `git status --short` shows `M .claude/agents/speccy-work.md`, `M resources/agents/.claude/agents/speccy-work.md.tmpl`, `M speccy-cli/tests/init.rs`, `M speccy-cli/tests/init_phase_agents.rs`. `git show HEAD:.claude/agents/speccy-work.md` still returns the old `sonnet[1m]`/`medium` values. The on-disk files carry the correct `opus[1m]`/`low` values and the test refactors are correct (per-phase pin table in both `init.rs` and `init_phase_agents.rs`), but none of it is committed. The task was flipped to `in-review` without a commit landing. AGENTS.md mandates "Before any commit lands, all four [hygiene gates] must pass" — the gates were run against the working tree but the work itself was never committed. The fix is straightforward: stage all four modified files and create a commit before the task can be considered in-review. No code changes are required; the on-disk edits are correct.
</review>

<retry>
Add a red→green evidence file proving the test-refactor transition, then commit all four working-tree edits so T-001 actually lands in git.

- tests (blocking): Implementer-note lacks `Evidence:` field; `.speccy/specs/0036-speccy-work-opus-low-pin/evidence/T-001.md` does not exist. Capture the red phase (scoped invocations of `t007_init_renders_claude_code_pin_assignments_matching_dogfood_pack` and `phase_worker_agent_has_model_and_effort_frontmatter` failing against the new agent frontmatter under the pre-refactor uniform-loop assertions) and the green phase (same scoped invocations passing after the per-phase tuple refactor), per the SPEC-0031 paper-trail convention every other recent spec follows.
- style (blocking): The four files touched by T-001 (`.claude/agents/speccy-work.md`, `resources/agents/.claude/agents/speccy-work.md.tmpl`, `speccy-cli/tests/init.rs`, `speccy-cli/tests/init_phase_agents.rs`) plus the evidence file plus this TASKS.md update must land in one commit so the in-review claim matches the git history. Hygiene gates re-run against the post-commit tree must exit 0.
</retry>
</task>

<task id="T-002" state="pending" covers="REQ-002">
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

</tasks>
