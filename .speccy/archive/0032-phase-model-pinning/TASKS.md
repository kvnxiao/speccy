---
spec: SPEC-0032
spec_hash_at_generation: d9430595ac2625c462f854186ed641000403f37899a82e5406c8bd7d1e6ad5ad
generated_at: 2026-05-19T21:28:21Z
---

# Tasks: SPEC-0032 Per-phase model and effort pinning across the lifecycle


## Phase 1: Claude Code phase-worker subagent files

<task id="T-001" state="completed" covers="REQ-001 REQ-008">
## T-001: Land four Claude Code phase-worker subagent files; leave SKILL.md files unpinned

Ship the four mechanical Claude Code phase-worker subagent
definition files. Three are pinned at `model: sonnet[1m]` /
`effort: medium`; `speccy-init` is left deliberately unpinned (no
`model:` or `effort:` field) so the subagent inherits the parent
session's model when invoked. The four matching
`.claude/skills/speccy-<phase>/SKILL.md` files do not gain any
pinning frontmatter — slash-command invocation runs in the parent
session by default per the amended REQ-001 and DEC-001.

The work touches templated source and rendered in-tree dogfood pack in
lockstep so the existing host-pack drift check stays green:

- Create four new templated source files under
  `resources/agents/.claude/agents/speccy-<phase>.md.tmpl` (one per
  phase). Each template's frontmatter carries `name`,
  `description`, and — on the three pinned phases — `model:
  sonnet[1m]` and `effort: medium`. The body of each template is a
  single MiniJinja include directive pointing at the shared phase
  body: `{% include "modules/skills/speccy-<phase>.md" %}`.
- Render the four templates into the in-tree dogfood pack at
  `.claude/agents/speccy-<phase>.md`. Rendered outputs must match the
  in-tree drift-check meta-test's expectations byte-for-byte.
- Verify the four existing Claude Code phase-worker SKILL.md template
  sources at `resources/agents/.claude/skills/speccy-<phase>/SKILL.md.tmpl`
  carry no `context:`, `agent:`, `model:`, or `effort:` keys in
  their YAML frontmatter — only the pre-existing `name:` /
  `description:` pair. Same for the rendered in-tree files at
  `.claude/skills/speccy-<phase>/SKILL.md`.

This task does not touch `.claude/skills/speccy-review/SKILL.md` and
does not create `.claude/agents/speccy-review.md` — REQ-002 keeps the
orchestrator unpinned on the Claude Code side, and T-003 owns the
shared review body edits. The `/agent speccy-<phase>` invocation
pointer that REQ-001 also requires (in the shared phase-worker
skill body) is owned by T-004, which edits
`resources/modules/skills/speccy-<phase>.md` so the pointer
propagates to both hosts in one edit.

Run all four hygiene gates after the edits: `cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check`. The
host-pack drift-check meta-test under `speccy-core/tests/` must pass
because every edited template has a matching rendered output.

Suggested files:

- `resources/agents/.claude/agents/speccy-tasks.md.tmpl` (new)
- `resources/agents/.claude/agents/speccy-work.md.tmpl` (new)
- `resources/agents/.claude/agents/speccy-ship.md.tmpl` (new)
- `resources/agents/.claude/agents/speccy-init.md.tmpl` (new)
- `.claude/agents/speccy-tasks.md` (new rendered)
- `.claude/agents/speccy-work.md` (new rendered)
- `.claude/agents/speccy-ship.md` (new rendered)
- `.claude/agents/speccy-init.md` (new rendered)
- `resources/agents/.claude/skills/speccy-tasks/SKILL.md.tmpl` (verify no pin keys)
- `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` (verify no pin keys)
- `resources/agents/.claude/skills/speccy-ship/SKILL.md.tmpl` (verify no pin keys)
- `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl` (verify no pin keys)
- `.claude/skills/speccy-tasks/SKILL.md` (verify no pin keys)
- `.claude/skills/speccy-work/SKILL.md` (verify no pin keys)
- `.claude/skills/speccy-ship/SKILL.md` (verify no pin keys)
- `.claude/skills/speccy-init/SKILL.md` (verify no pin keys)

<task-scenarios>
Given each of the four post-T-001 SKILL.md files at
`.claude/skills/speccy-<phase>/SKILL.md` for `phase` in {`tasks`,
`work`, `ship`, `init`}, when its YAML frontmatter is parsed, then
none of the keys `context`, `agent`, `model`, or `effort` are
present — only the pre-existing `name:` / `description:` pair.

Given each matching SKILL.md template source at
`resources/agents/.claude/skills/speccy-<phase>/SKILL.md.tmpl` for
the same four phases, when each one's YAML frontmatter is parsed,
then none of the keys `context`, `agent`, `model`, or `effort` are
present.

Given the four new agent files at `.claude/agents/speccy-<phase>.md`
for the same four phases, when each is read, then each file exists
and parses with valid YAML frontmatter containing at least `name:`
and `description:` keys.

Given the three pinned agent files (`speccy-tasks.md`,
`speccy-work.md`, `speccy-ship.md`), when each one's frontmatter is
parsed, then each contains `model: sonnet[1m]` and `effort: medium`.

Given the unpinned `speccy-init.md` agent file, when its frontmatter
is parsed, then it does not contain a key named `model` and does not
contain a key named `effort`.

Given each templated source at
`resources/agents/.claude/agents/speccy-<phase>.md.tmpl` for the four
phases, when read, then each contains the literal string
`{% include "modules/skills/speccy-<phase>.md" %}` with the matching
phase name in the include path.

Given the existing in-tree host-pack drift-check meta-test under
`speccy-core/tests/`, when run after this task lands, then it exits 0
because every edited template has a matching rendered output in the
in-tree dogfood pack.

Given the four hygiene gates (`cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check`) run against
the working tree at the commit that lands this task, when each
exits, then each exit code is 0.
</task-scenarios>
</task>

## Phase 2: Claude Code reviewer pin frontmatter

<task id="T-002" state="completed" covers="REQ-003">
## T-002: Add asymmetric model + effort frontmatter to the six Claude Code reviewer agent files

Add `model:` and `effort:` keys to the YAML frontmatter of the six
existing Claude Code reviewer agent files. The assignment is
asymmetric across the personas and reflects the work-shape tier each
reviewer carries:

- `reviewer-business`, `reviewer-tests`, `reviewer-architecture`:
  `model: opus[1m]`, `effort: xhigh` (semantic adversarial load,
  Opus tier).
- `reviewer-security`: `model: sonnet[1m]`, `effort: high`
  (pattern-plus-judgment load).
- `reviewer-style`, `reviewer-docs`: `model: sonnet[1m]`,
  `effort: medium` (pure-pattern load).

Every value uses the `[1m]` 1M-context-window suffix so each reviewer
has the headroom to read full SPEC + diff + task body without
truncation. Long-form versioned snapshot IDs (`claude-opus-4-7[1m]`,
etc.) do not appear — REQ-005 / T-006 enforces this invariant.

The reviewer body content (below frontmatter, the
`{% include "modules/personas/reviewer-<persona>.md" %}` line) is not
touched in this task. Body edits flow through the shared persona
modules and are owned by T-003 (REQ-009 verdict-return contract).
This task is frontmatter-only.

Edit both the templated source and the rendered in-tree dogfood pack
in lockstep:

- `resources/agents/.claude/agents/reviewer-<persona>.md.tmpl` (template)
- `.claude/agents/reviewer-<persona>.md` (rendered)

Six personas × two files each = twelve file touches. Same mechanical
shape repeated per persona.

Run all four hygiene gates after the edits. The host-pack drift
check must remain green.

Suggested files:

- `resources/agents/.claude/agents/reviewer-business.md.tmpl`
- `resources/agents/.claude/agents/reviewer-tests.md.tmpl`
- `resources/agents/.claude/agents/reviewer-architecture.md.tmpl`
- `resources/agents/.claude/agents/reviewer-security.md.tmpl`
- `resources/agents/.claude/agents/reviewer-style.md.tmpl`
- `resources/agents/.claude/agents/reviewer-docs.md.tmpl`
- `.claude/agents/reviewer-business.md`
- `.claude/agents/reviewer-tests.md`
- `.claude/agents/reviewer-architecture.md`
- `.claude/agents/reviewer-security.md`
- `.claude/agents/reviewer-style.md`
- `.claude/agents/reviewer-docs.md`

<task-scenarios>
Given each of the three Opus-tier reviewer agent files
(`reviewer-business.md`, `reviewer-tests.md`,
`reviewer-architecture.md`) after this task lands, when each one's
YAML frontmatter is parsed, then each contains `model: opus[1m]` and
`effort: xhigh`.

Given `.claude/agents/reviewer-security.md` after this task lands,
when its YAML frontmatter is parsed, then it contains
`model: sonnet[1m]` and `effort: high`.

Given each of the two pattern-tier reviewer agent files
(`reviewer-style.md`, `reviewer-docs.md`) after this task lands,
when each one's YAML frontmatter is parsed, then each contains
`model: sonnet[1m]` and `effort: medium`.

Given the body content (every byte below the YAML frontmatter
delimiter) of each of the six reviewer agent files, when diffed
against the pre-SPEC version, then the body is byte-identical (the
shared `{% include "modules/personas/reviewer-<persona>.md" %}`
directive renders the same body until T-003 edits the persona
modules).

Given the six templated source files at
`resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`, when
each one's frontmatter is parsed, then each carries the same
`model:` and `effort:` values as the matching rendered file at
`.claude/agents/reviewer-<persona>.md`.

Given the existing in-tree host-pack drift-check meta-test, when run
after this task lands, then it exits 0 (templates and rendered
outputs match byte-for-byte).

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code is
0.
</task-scenarios>
</task>

## Phase 3: F-10 absorption (verdict-return + orchestrator consolidation)

<task id="T-003" state="completed" covers="REQ-002 REQ-009">
## T-003: Reviewer-persona bodies return verdicts; orchestrator becomes sole TASKS.md writer

Edit the shared prompt body files under `resources/modules/personas/`
and `resources/modules/skills/` to absorb the F-10 reviewer-verdict
contract. After this task lands, reviewer subagents return their
verdicts to the `/speccy-review` orchestrator as their final message
and are explicitly forbidden from editing TASKS.md from inside the
subagent. The orchestrator parses each spawned reviewer's return
message, consolidates the verdicts, and writes the state transition
to TASKS.md serially in the orchestrator turn — eliminating the
parallel-write race by construction (per DEC-008).

Work surfaces:

- Edit each of the six `resources/modules/personas/reviewer-<persona>.md`
  files (`business`, `tests`, `architecture`, `security`, `style`,
  `docs`) to add prose directing the reviewer to:
  - Emit its review verdict as the subagent's final message in a
    shape structured enough that the orchestrator can parse it
    without ambiguity (at minimum a per-task pass/fail/needs-retry
    decision and, on failure, the `<retry>` body text the reviewer
    wants recorded against the task).
  - Explicitly not edit TASKS.md from inside the reviewer subagent.
- Edit `resources/modules/skills/speccy-review.md` to describe the
  consolidation contract: the orchestrator fans out to the default
  four-persona set (`business`, `tests`, `security`, `style`) by
  default, plus `reviewer-architecture` and `reviewer-docs` as
  explicit-invoke additions; parses each spawned reviewer's return
  message; consolidates the verdicts into a single per-task decision;
  applies the state transition to TASKS.md serially in the
  orchestrator turn (flipping `in-review` → `completed` when every
  spawned reviewer passes, or `in-review` → `pending` with a
  consolidated `<retry>` body when any spawned reviewer fails).
- Re-render the downstream files that include these shared modules:
  the six `.claude/agents/reviewer-<persona>.md` files (which include
  the persona module), the six `.codex/agents/reviewer-<persona>.toml`
  files (which include the persona module via
  `developer_instructions`), `.claude/skills/speccy-review/SKILL.md`
  (which includes the speccy-review skill module), and
  `.agents/skills/speccy-review/SKILL.md` (same shared module).
- Verify `.claude/skills/speccy-review/SKILL.md` YAML frontmatter
  remains unpinned (no `model:`, `effort:`, `context:`, or `agent:`
  keys) per REQ-002 — this task must not introduce any such keys.
- Verify no file at `.claude/agents/speccy-review.md` exists or is
  created. The orchestrator stays in the parent session.

Reviewer prompts must not grant Write or Edit tool scope for
TASKS.md to the persona subagent. If the existing persona body
already enumerates tool grants, the edit removes the TASKS.md
write/edit pathway; if it does not, the edit adds the explicit
prohibition prose.

Run all four hygiene gates after the edits.

Suggested files:

- `resources/modules/personas/reviewer-business.md`
- `resources/modules/personas/reviewer-tests.md`
- `resources/modules/personas/reviewer-architecture.md`
- `resources/modules/personas/reviewer-security.md`
- `resources/modules/personas/reviewer-style.md`
- `resources/modules/personas/reviewer-docs.md`
- `resources/modules/skills/speccy-review.md`
- `.claude/agents/reviewer-business.md` (re-rendered body)
- `.claude/agents/reviewer-tests.md` (re-rendered body)
- `.claude/agents/reviewer-architecture.md` (re-rendered body)
- `.claude/agents/reviewer-security.md` (re-rendered body)
- `.claude/agents/reviewer-style.md` (re-rendered body)
- `.claude/agents/reviewer-docs.md` (re-rendered body)
- `.codex/agents/reviewer-business.toml` (re-rendered body)
- `.codex/agents/reviewer-tests.toml` (re-rendered body)
- `.codex/agents/reviewer-architecture.toml` (re-rendered body)
- `.codex/agents/reviewer-security.toml` (re-rendered body)
- `.codex/agents/reviewer-style.toml` (re-rendered body)
- `.codex/agents/reviewer-docs.toml` (re-rendered body)
- `.claude/skills/speccy-review/SKILL.md` (re-rendered body)
- `.agents/skills/speccy-review/SKILL.md` (re-rendered body)

<task-scenarios>
Given each of the six post-T-003 reviewer persona files under
`resources/modules/personas/reviewer-<persona>.md`, when each is
read, then each contains explicit prose directing the reviewer to
emit its verdict as the subagent's final message and an explicit
prohibition against editing TASKS.md from inside the reviewer
subagent.

Given `resources/modules/skills/speccy-review.md` after this task
lands, when it is read, then it contains a consolidation contract
naming the default four-persona fan-out (`business`, `tests`,
`security`, `style`), the two explicit-invoke personas
(`architecture`, `docs`), the verdict-return shape it expects from
each spawned reviewer, and the serial-write discipline that makes
the orchestrator the sole writer to TASKS.md for review-induced
state transitions.

Given `.claude/skills/speccy-review/SKILL.md` after this task lands,
when its YAML frontmatter is parsed, then none of the keys `model`,
`effort`, `context`, or `agent` are present (REQ-002 invariant).

Given the post-T-003 `.claude/agents/` directory listing, when
scanned, then there is no file named `speccy-review.md` (the
orchestrator does not become a subagent on Claude Code).

Given each of the six reviewer prompt bodies (persona module plus
any tool-grant prose downstream of it), when scanned for explicit
Write or Edit tool grants on TASKS.md, then zero such grants exist.

Given a deterministic test harness exercise of `/speccy-review`
against a task in `state="in-review"` with the default four-persona
fan-out, when two mocked reviewers return fail verdicts and two
return pass verdicts, then the post-run TASKS.md content for that
task shows `state="pending"` with exactly one consolidated `<retry>`
body aggregating the failing reviewers' feedback (not two separate
`<retry>` elements, not a torn partial write).

Given the same harness with all four mocked reviewers returning pass
verdicts, when the orchestrator completes, then the post-run
TASKS.md content for that task shows `state="completed"` and no
`<retry>` body is present.

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code is
0.
</task-scenarios>
</task>

## Phase 4: Codex parallel ships matching pins

<task id="T-004" state="completed" covers="REQ-001 REQ-004 REQ-008">
## T-004: Land three new pinned Codex phase-worker TOML agent files

Add the Codex-side parity for the three pinned mechanical phases
(`tasks`, `work`, `ship`). The cost-and-time win is opt-in via
`/agent speccy-<phase>` on both hosts under the amended SPEC; both
Codex (which never supported auto-fork) and Claude Code (which
retreated from `context: fork` per the third Changelog row /
DEC-001) expose the symmetric opt-in subagent surface. The
`speccy-init` phase ships no Codex TOML agent file (per the
fourth Changelog row / DEC-009 / REQ-010): its load-bearing work
is interactive parent-session Q&A and there is no pinned tier to
opt into. The discovery pointer that earlier drafts placed at the
top of each shared skill body is dropped from this task's scope —
under REQ-010 the matching pinned SKILL.md bodies become thin
stubs whose entire body is the pointer, and that work is owned by
T-009 under Phase 8.

Work surfaces:

- Create three new Codex subagent TOML files at
  `.codex/agents/speccy-tasks.toml`,
  `.codex/agents/speccy-work.toml`, and
  `.codex/agents/speccy-ship.toml` plus their templated sources at
  `resources/agents/.codex/agents/speccy-<phase>.toml.tmpl`. Each
  pinned file declares `name`, `description`, `model = "gpt-5.5"`,
  and `model_reasoning_effort = "medium"`.
- Each TOML's `developer_instructions` value renders the shared
  phase body via MiniJinja
  `{% include "modules/phases/speccy-<phase>.md" %}`. The shared
  source path moved from `resources/modules/skills/` to
  `resources/modules/phases/` under DEC-009 / REQ-010; the
  module-rename mechanical work is owned by T-009 and runs first,
  but T-004's TOML templates name the post-rename path directly
  so they pass T-009's hygiene gates without later edits.
- Verify no file at `.codex/agents/speccy-review.toml` or
  `.codex/agents/speccy-init.toml` exists or is created. The
  orchestrator stays unpinned on Codex per REQ-002 / DEC-002;
  `speccy-init` ships no Codex agent file per DEC-009 / REQ-010.
- The four phase-worker skill body sources at
  `resources/modules/phases/speccy-<phase>.md` (renamed from
  `resources/modules/skills/` by T-009) are not edited in this
  task. Discovery pointers to `/agent speccy-<phase>` live in
  the thin-stub SKILL.md template bodies that T-009 produces, not
  in the shared phase module.
- The `speccy-review` shared body at
  `resources/modules/skills/speccy-review.md` is unchanged in
  this task (its path is unaffected by the T-009 rename, which
  only moves the four phase-worker body files; review's shared
  body stays under `resources/modules/skills/`).

Re-render any downstream files that include the edited shared
modules so the host-pack drift check stays green. Run all four
hygiene gates.

Sequencing note: T-009 (Phase 8) is a prerequisite for the
`resources/modules/phases/` path to exist. T-004 either runs
after T-009 lands the rename or coordinates its TOML
`{% include %}` paths against the renamed location.

Suggested files:

- `resources/agents/.codex/agents/speccy-tasks.toml.tmpl` (new)
- `resources/agents/.codex/agents/speccy-work.toml.tmpl` (new)
- `resources/agents/.codex/agents/speccy-ship.toml.tmpl` (new)
- `.codex/agents/speccy-tasks.toml` (new rendered)
- `.codex/agents/speccy-work.toml` (new rendered)
- `.codex/agents/speccy-ship.toml` (new rendered)
- All files that include the edited shared modules (re-rendered)

<task-scenarios>
Given each of the three new TOML files at
`.codex/agents/speccy-<phase>.toml` for `phase` in {`tasks`, `work`,
`ship`}, when each is read, then each file exists and parses
as valid TOML containing at least `name` and `description` keys.

Given each of the three pinned Codex phase-worker files
(`speccy-tasks.toml`, `speccy-work.toml`, `speccy-ship.toml`), when
its TOML is parsed, then it contains `model = "gpt-5.5"` and
`model_reasoning_effort = "medium"`.

Given the post-T-004 `.codex/agents/` directory listing, when
scanned, then there is no file named `speccy-review.toml` (the
orchestrator stays unpinned on Codex) and no file named
`speccy-init.toml` (per DEC-009 / REQ-010).

Given each templated Codex TOML source at
`resources/agents/.codex/agents/speccy-<phase>.toml.tmpl` for the
three pinned phases, when read, then its `developer_instructions`
value contains a MiniJinja include directive naming
`modules/phases/speccy-<phase>.md`.

Given the existing in-tree host-pack drift-check meta-test, when run
after this task lands, then it exits 0 (templates and rendered
outputs match byte-for-byte).

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code
is 0.
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-004">
## T-005: Add gpt-5.5 + asymmetric model_reasoning_effort to the six Codex reviewer TOML files

Add `model = "gpt-5.5"` to every Codex reviewer TOML file and add a
`model_reasoning_effort` key whose value matches the persona's
work-shape tier. OpenAI does not expose an Opus/Sonnet-style tier
axis on its model identifier, so the asymmetric shape that lives in
both `model` and `effort` on Claude Code lives entirely in
`model_reasoning_effort` on Codex:

- `reviewer-business`, `reviewer-tests`, `reviewer-architecture`:
  `model_reasoning_effort = "high"` (semantic adversarial load).
- `reviewer-security`: `model_reasoning_effort = "medium"`
  (pattern-plus-judgment load).
- `reviewer-style`, `reviewer-docs`: `model_reasoning_effort = "low"`
  (pure-pattern load).

Edit both the templated TOML source and the rendered in-tree dogfood
pack in lockstep:

- `resources/agents/.codex/agents/reviewer-<persona>.toml.tmpl`
- `.codex/agents/reviewer-<persona>.toml`

Six personas × two files each = twelve file touches. The
`developer_instructions` value (carrying the
`{% include "modules/personas/reviewer-<persona>.md" %}` directive)
is not touched in this task — body edits flow through the shared
persona modules and are owned by T-003.

Run all four hygiene gates. The host-pack drift check must remain
green.

Suggested files:

- `resources/agents/.codex/agents/reviewer-business.toml.tmpl`
- `resources/agents/.codex/agents/reviewer-tests.toml.tmpl`
- `resources/agents/.codex/agents/reviewer-architecture.toml.tmpl`
- `resources/agents/.codex/agents/reviewer-security.toml.tmpl`
- `resources/agents/.codex/agents/reviewer-style.toml.tmpl`
- `resources/agents/.codex/agents/reviewer-docs.toml.tmpl`
- `.codex/agents/reviewer-business.toml`
- `.codex/agents/reviewer-tests.toml`
- `.codex/agents/reviewer-architecture.toml`
- `.codex/agents/reviewer-security.toml`
- `.codex/agents/reviewer-style.toml`
- `.codex/agents/reviewer-docs.toml`

<task-scenarios>
Given each of the six Codex reviewer TOML files at
`.codex/agents/reviewer-<persona>.toml` after this task lands, when
each is parsed, then each contains `model = "gpt-5.5"`.

Given each of the three semantic Codex reviewer TOML files
(`reviewer-business.toml`, `reviewer-tests.toml`,
`reviewer-architecture.toml`), when parsed, then each contains
`model_reasoning_effort = "high"`.

Given `.codex/agents/reviewer-security.toml` after this task lands,
when parsed, then it contains `model_reasoning_effort = "medium"`.

Given each of the two pattern-tier Codex reviewer TOML files
(`reviewer-style.toml`, `reviewer-docs.toml`), when parsed, then
each contains `model_reasoning_effort = "low"`.

Given each of the six templated source files at
`resources/agents/.codex/agents/reviewer-<persona>.toml.tmpl`, when
each is parsed, then each carries the same `model` and
`model_reasoning_effort` values as its matching rendered file.

Given the existing in-tree host-pack drift-check meta-test, when run
after this task lands, then it exits 0.

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code
is 0.
</task-scenarios>
</task>

## Phase 5: Pin shape validation

<task id="T-006" state="completed" covers="REQ-005">
## T-006: Add a meta-test enforcing the pin-shape invariants across all shipped files

Lock the pin-shape invariants from REQ-005 into a meta-test under
`speccy-core/tests/` (or the equivalent meta-test home used by the
existing host-pack drift check). The test scans every file under
`resources/agents/.claude/`, `resources/agents/.codex/`,
`resources/agents/.agents/`, the in-tree dogfood pack at `.claude/`
and `.codex/`, and asserts:

- No `model:` (Claude Code) or `model` (Codex) frontmatter value
  contains the literal substrings `claude-opus-`, `claude-sonnet-`,
  or `claude-haiku-` — long-form versioned snapshot IDs do not
  appear in any shipped file.
- No `model:` or `model` value contains the substring `haiku` —
  Haiku is not used anywhere in this SPEC's pin assignment, and the
  test makes that invariant load-bearing rather than aspirational.
- Every Claude Code pinned `model:` value matches the regex
  `^(opus|sonnet)\[1m\]$` exactly. Every Claude Code unpinned file
  declared by REQ-001 / REQ-002 is verified to have no `model:`
  key at all: the `speccy-review` skill
  (`.claude/skills/speccy-review/SKILL.md`) and the four
  mechanical-phase skills
  (`.claude/skills/speccy-tasks/SKILL.md`,
  `.claude/skills/speccy-work/SKILL.md`,
  `.claude/skills/speccy-ship/SKILL.md`,
  `.claude/skills/speccy-init/SKILL.md`). The four mechanical-phase
  skills additionally have no `context:`, `agent:`, or `effort:`
  keys (per amended REQ-001 / DEC-001). The
  `.claude/agents/speccy-init.md` agent file is not in this list
  because it does not exist post-T-009 / REQ-010.
- Every Codex pinned `model` value equals the literal string
  `gpt-5.5`. No Codex agent file `speccy-init.toml` exists (per
  REQ-010 / DEC-009), so there is no Codex-side unpinned
  phase-worker file to check.
- Every Opus-pinned Claude Code file's `effort:` value is one of
  `low`, `medium`, `high`, `xhigh`, `max`.
- Every Sonnet-pinned Claude Code file's `effort:` value is one of
  `low`, `medium`, `high`, `max` — never `xhigh` (Sonnet does not
  support the xhigh tier).
- Every pinned Codex file's `model_reasoning_effort` value is one of
  `low`, `medium`, `high`, `xhigh`.

The test is a pure-Rust scan over the file tree (no host calls, no
network). It exits 0 when all invariants hold and reports the
first-failing file path and the violated invariant when one breaks.

Run the test plus the four hygiene gates after the edit. The new
test must pass against the post-T-001..T-005 working tree.

Suggested files:

- A new meta-test file under `speccy-core/tests/` (the existing
  host-pack drift-check meta-test is the closest analog for shape)

<task-scenarios>
Given the new meta-test added in this task, when it runs against
the post-T-001..T-005 working tree, then it exits 0 because every
shipped pin satisfies the REQ-005 invariants.

Given the same test, when a hypothetical edit replaces any
`model: sonnet[1m]` value in a shipped file with the long-form
`claude-sonnet-4-6[1m]` value, then the test exits non-zero and
names the offending file path and value in its failure message
(verified by running the test against a temporary working-tree
mutation; revert the mutation before committing).

Given the same test, when a hypothetical edit changes any
Sonnet-pinned file's `effort:` value to `xhigh`, then the test exits
non-zero and names the offending file path and value (Sonnet does
not support the xhigh tier per REQ-005). Revert the mutation before
committing.

Given the same test, when a hypothetical edit changes any Codex
pinned file's `model` value to anything other than the literal
`gpt-5.5`, then the test exits non-zero. Revert the mutation before
committing.

Given the same test, when run against the unpinned files
(`.claude/skills/speccy-review/SKILL.md`,
`.claude/skills/speccy-tasks/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.claude/skills/speccy-ship/SKILL.md`,
`.claude/skills/speccy-init/SKILL.md`), then it exits 0 because
the unpinned-file invariant ("no `context` / `agent` / `model` /
`effort` / `model_reasoning_effort` keys present, as applicable
per host") is satisfied — and a hypothetical mutation that adds
any of those keys to an unpinned file makes the test exit
non-zero. Revert any such mutation before committing. The agent
files `.claude/agents/speccy-init.md` and
`.codex/agents/speccy-init.toml` are not in this list because
they do not exist post-T-009 / REQ-010.

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code
is 0.
</task-scenarios>
</task>

## Phase 6: speccy init parity verification

<task id="T-007" state="completed" covers="REQ-006">
## T-007: Verify speccy init renders pin assignments matching the in-tree dogfood pack

Add an integration test (or extend the closest existing one) that
exercises `speccy init` end-to-end in a fresh temporary directory
and asserts the rendered host packs carry the same pin assignments
as the in-tree dogfood workspace.

The test:

- Creates a fresh empty temporary directory.
- Invokes `speccy init` (via the `speccy-cli` binary or its
  programmatic equivalent the existing init test uses).
- Asserts the three new pinned Claude Code phase-worker agent
  files exist at `.claude/agents/speccy-<phase>.md` for `phase`
  in {`tasks`, `work`, `ship`} with `model: sonnet[1m]` and
  `effort: medium` frontmatter, matching the in-tree dogfood
  pack at the repository root.
- Asserts no file at `.claude/agents/speccy-init.md` is created
  in the rendered output (per DEC-009 / REQ-010).
- Asserts the three new pinned Codex phase-worker TOML files
  exist at `.codex/agents/speccy-<phase>.toml` for the same
  three phases with `model = "gpt-5.5"` and
  `model_reasoning_effort = "medium"` keys.
- Asserts no `.codex/agents/speccy-review.toml` file is created
  in the rendered output (REQ-002 / DEC-002 invariant) and no
  `.codex/agents/speccy-init.toml` file is created (per DEC-009
  / REQ-010).
- Asserts the six reviewer files on each host carry the asymmetric
  pin assignment per REQ-003 (Claude Code) and REQ-004 (Codex).
- Asserts the four mechanical-phase SKILL.md files on Claude Code
  in the rendered tree (`.claude/skills/speccy-<phase>/SKILL.md`
  for `tasks`, `work`, `ship`, `init`) carry no `context:`,
  `agent:`, `model:`, or `effort:` frontmatter keys (per amended
  REQ-001 / DEC-001 — slash-command invocation runs in the parent
  session by default).
- Asserts `.claude/skills/speccy-review/SKILL.md` in the rendered
  tree contains no `model:`, `effort:`, `context:`, or `agent:`
  keys.
- Asserts the three pinned-phase SKILL.md rendered files on each
  host (`.claude/skills/speccy-<phase>/SKILL.md` and
  `.agents/skills/speccy-<phase>/SKILL.md` for `tasks`, `work`,
  `ship`) carry thin-stub bodies that name the matching agent
  file and the `/agent speccy-<phase>` invocation pattern per
  REQ-010 / T-009. The `/speccy-init` SKILL.md on each host
  remains full-body (no stub transformation).

Confirm that the existing CI host-pack drift-check meta-test exits 0
against the post-SPEC workspace (templated `resources/agents/`
source matches the in-tree rendered outputs byte-for-byte). The
drift check is already wired and runs as part of `cargo test
--workspace`; this task does not extend it but does verify it stays
green after T-001..T-006.

Run the new test plus the four hygiene gates.

Suggested files:

- A new (or extended) integration test under
  `speccy-cli/tests/` or `speccy-core/tests/` that exercises
  `speccy init` against a fresh tempdir.

<task-scenarios>
Given the new integration test added in this task, when it runs
against the post-T-006 / post-T-009 working tree, then it exits
0: a fresh `speccy init` invocation in a temporary directory
produces the three new pinned Claude Code phase-worker agent
files at `.claude/agents/speccy-<phase>.md` for `phase` in
{`tasks`, `work`, `ship`}, no `.claude/agents/speccy-init.md`
file (per DEC-009 / REQ-010), the three new pinned Codex
phase-worker TOML files at `.codex/agents/speccy-<phase>.toml`
for the same three phases, no
`.codex/agents/speccy-review.toml` and no
`.codex/agents/speccy-init.toml` files, the six reviewer files
on each host with the asymmetric pin assignment, the four
mechanical-phase Claude Code SKILL.md files carrying no
`context:` / `agent:` / `model:` / `effort:` keys, the
`.claude/skills/speccy-review/SKILL.md` rendered file with no
pinning frontmatter, and the three pinned-phase SKILL.md
rendered files on both hosts carrying thin-stub bodies per
REQ-010 (the `/speccy-init` SKILL.md remains full-body).

Given the existing in-tree host-pack drift-check meta-test, when
run against the post-T-001..T-006 / post-T-009 working tree,
then it exits 0 (templated source and in-tree rendered outputs
match byte-for-byte across every edited and newly-created
file).

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code
is 0.
</task-scenarios>
</task>

## Phase 7: README pinning section and drift audit

<task id="T-008" state="completed" covers="REQ-007">
## T-008: Add Model pinning section to README and audit the rest for current-repo-state drift

Two coupled README edits, landed together because they touch the
same file:

1. Add a new top-level section (or subsection under an existing
   top-level section, depending on the README's current shape)
   titled "Model pinning" or substantially equivalent. The section
   names:
   - The full pin assignment table covering all five mechanical
     phases (`speccy-tasks`, `speccy-work`, `speccy-ship`,
     `speccy-init`, `speccy-review`) and all six reviewer personas
     (`reviewer-business`, `reviewer-tests`,
     `reviewer-architecture`, `reviewer-security`,
     `reviewer-style`, `reviewer-docs`).
   - The pin tier for each row (Opus or Sonnet on Claude Code with
     the `[1m]` suffix; `gpt-5.5` with an effort tier on Codex; or
     "unpinned, inherits session" for the phases that are
     deliberately left unpinned: `speccy-init` and `speccy-review`).
   - The effort level (or absence thereof) for each row.
   - The agent-file-existence column: `speccy-tasks`,
     `speccy-work`, and `speccy-ship` ship pinned subagent files
     at `.claude/agents/speccy-<phase>.md` and
     `.codex/agents/speccy-<phase>.toml`. `speccy-init` and
     `speccy-review` ship no agent files (per DEC-009 / REQ-010
     for `init`; per DEC-002 for `review`) — only the SKILL.md
     surface on each host.
   - The opt-in invocation surface that activates the pin on both
     hosts: invoke `/agent speccy-<phase>` (or the host's
     equivalent subagent surface) before running the phase
     command. The slash command on its own (`/speccy-<phase>`)
     runs in the parent session at the parent session's model.
   - The stubbed-SKILL.md shape per REQ-010: for the three
     pinned phases, the SKILL.md body is a thin stub that names
     the agent file as the canonical procedure source. The
     agent file's body is the single on-disk source of truth.
     `/speccy-init`'s SKILL.md and `/speccy-review`'s SKILL.md
     both remain full-body since no subagent file exists for
     either to defer to.
   - A brief note that the SPEC retreated from the
     `context: fork` auto-fork pattern: the silent-by-design
     tool-output isolation produced minutes of dead air in the
     parent session on multi-minute phase work (referenced as a
     design lesson, not a step-by-step recap).
   - The user override path: edit the ejected agent file's
     frontmatter to swap the model alias or remove a pin entirely.
   - The alias rationale: pins use aliases by default so they float
     forward as vendors ship new generations; users who want
     reproducibility can swap an alias for a long-form snapshot ID
     by editing the ejected file.
   - An explicit note that the `/speccy-review` orchestrator stays
     unpinned on both hosts (no agent file on Codex, no model
     frontmatter on Claude Code) because the orchestrator owns
     TASKS.md writes per REQ-009 and needs the parent session's
     full capacity.
2. Audit the rest of the README end-to-end against the current
   state of the repository. Correct any prose that has drifted:
   - The CLI command reference (if present) lists exactly the ten
     currently shipped commands: `init`, `plan`, `tasks`,
     `implement`, `review`, `report`, `status`, `next`, `check`,
     `verify`. No other command names.
   - The ejection-path reference (if present) names `.claude/`,
     `.codex/`, and `.agents/` paths. References to the retired
     `.speccy/skills/` directory (per SPEC-0027) do not appear as
     current user-facing prose; if they appear at all, they live in
     a historical-context or migration-note block.
   - References to retired XML elements (per SPEC-0021) and other
     drift surfaces flagged during the read pass are corrected.
   - Any other prose contradicting the current repository state is
     corrected.

The audit is one-time and not codified into a meta-test (REQ-007
explicitly excludes a README-drift CI check; future drift is caught
by `reviewer-docs` at review time).

Run the four hygiene gates after the edit. README edits do not
trigger any of the four gates directly but the change must not
introduce broken markdown that downstream tooling chokes on.

Suggested files:

- `README.md`

<task-scenarios>
Given `README.md` at the repository root after this task lands, when
scanned for the literal substring "Model pinning" (or the chosen
section title), then at least one match exists at the start of a
top-level or subsection heading.

Given the same file's new pinning section, when read, then it names
the pin tier (Opus or Sonnet on Claude Code with the `[1m]` suffix;
`gpt-5.5` with an effort tier on Codex; or "unpinned, inherits
session") and the effort level (or absence thereof) for every
mechanical phase (`speccy-tasks`, `speccy-work`, `speccy-ship`,
`speccy-init`, `speccy-review`) and every reviewer persona
(`reviewer-business`, `reviewer-tests`, `reviewer-architecture`,
`reviewer-security`, `reviewer-style`, `reviewer-docs`).

Given the same section, when read, then it describes the opt-in
subagent invocation pattern (`/agent speccy-<phase>` or the
host's equivalent) that activates the pin on both hosts, and
names the parent-session-by-default behavior of the
slash-command surface.

Given the same section, when read, then it describes the user
override path (edit the ejected agent file's frontmatter) and notes
that pins use aliases by default so users can lock to a specific
model version by editing.

Given the same section, when read, then it explicitly notes that
`/speccy-review` stays unpinned on both hosts because the
orchestrator owns TASKS.md writes per REQ-009 and needs the parent
session's full capacity.

Given the rest of the post-T-008 `README.md`, when grepped for CLI
command names, then every mentioned name is one of the ten
currently shipped commands (`init`, `plan`, `tasks`, `implement`,
`review`, `report`, `status`, `next`, `check`, `verify`).

Given the rest of the post-T-008 `README.md`, when grepped for
mentions of the retired `.speccy/skills/` directory, then no prose
presents it as a current user-facing path; references at most
appear in historical-context or migration-note blocks.

Given the four hygiene gates run against the working tree at the
commit that lands this task, when each exits, then each exit code
is 0.
</task-scenarios>
</task>

## Phase 8: Skill-stub dedup (REQ-010 / DEC-009)

<task id="T-009" state="completed" covers="REQ-010">
## T-009: Stub the three pinned phase-worker SKILL.md bodies, drop the speccy-init agent, rename modules/skills/ to modules/phases/, rewrite agent description prose, and add the stub-shape meta-test

Land the four coupled mechanical changes that DEC-009 / REQ-010
specify. Each is small in isolation; landing them together keeps
the host-pack drift check green at every step of the work.

Work surfaces:

- **Rename the shared phase modules.** Move the four files at
  `resources/modules/skills/speccy-<phase>.md` for `phase` in
  {`tasks`, `work`, `ship`, `init`} to
  `resources/modules/phases/speccy-<phase>.md`. Create the
  `resources/modules/phases/` directory. Do not move
  `resources/modules/skills/speccy-review.md` — review's shared
  body stays where it is (review is excluded from REQ-010 per
  REQ-002 / REQ-009).
- **Update all `{% include %}` directives** across
  `resources/agents/` that reference the renamed paths. Targets
  to audit (non-exhaustive): three Claude Code agent templates
  at `resources/agents/.claude/agents/speccy-<phase>.md.tmpl`
  for `tasks`/`work`/`ship` (after T-009's delete step, only
  three remain); three Codex agent TOML templates at
  `resources/agents/.codex/agents/speccy-<phase>.toml.tmpl`
  for the same three phases (these come from T-004 — coordinate
  ordering); the Claude Code skill templates at
  `resources/agents/.claude/skills/speccy-<phase>/SKILL.md.tmpl`
  for all four phase names (these change shape entirely — see
  stub step below); the Codex skill templates at
  `resources/agents/.agents/skills/speccy-<phase>/SKILL.md.tmpl`
  for the same four phase names.
- **Stub the three pinned phase-worker SKILL.md template bodies
  on both hosts.** For `phase` in {`tasks`, `work`, `ship`},
  rewrite the bodies of
  `resources/agents/.claude/skills/speccy-<phase>/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/speccy-<phase>/SKILL.md.tmpl`
  so each contains, below its YAML frontmatter, a thin stub of
  bounded short length (≤10 non-blank content lines) that:
  (a) names the matching agent file path
  (`.claude/agents/speccy-<phase>.md` for the Claude Code skill;
  `.codex/agents/speccy-<phase>.toml` for the Codex skill);
  (b) names the `/agent speccy-<phase>` invocation as the pinned
  execution path; and (c) does not contain the substrings
  `## Steps` or `## When to use`. The stub body does not include
  any `{% include "modules/phases/..." %}` directive — the
  procedure lives only in the agent file. Suggested minimal
  body (for `speccy-work` on Claude Code):

  ```
  # /speccy-work

  Read `.claude/agents/speccy-work.md` and follow it, or invoke
  `/agent speccy-work` for the pinned execution path.
  ```

  `/speccy-init`'s SKILL.md template stays full-body (no stub
  edit) — it has no subagent file to defer to. The init SKILL.md
  template's `{% include %}` directive is updated to the
  renamed `modules/phases/speccy-init.md` path.
- **Delete the `speccy-init` agent files.** Remove the rendered
  file at `.claude/agents/speccy-init.md` and the template
  source at `resources/agents/.claude/agents/speccy-init.md.tmpl`.
  No corresponding Codex TOML exists or is created (T-004
  scope already excludes init).
- **Rewrite the three remaining Claude Code agent
  `description:` prose values** on both the rendered file and
  the matching `.tmpl` template source. The rewrites drop the
  literal substring `` via `context: fork` `` and drop any
  reference to specific model/effort tier values (no `Sonnet`,
  `Opus`, `Haiku`, `xhigh`, `medium`, `high`, `low`, or `max`
  outside code fences in the description string). Suggested
  form (for `speccy-work`):

  ```
  description: Implements one Speccy task per invocation.
    Invoke via /agent speccy-work for the pinned execution
    path defined in this file's frontmatter.
  ```

- **Add a pure-Rust meta-test under `speccy-core/tests/`** that
  scans the rendered host pack files and asserts the
  stub-shape invariants from CHK-010: (i) for `phase` in
  {`tasks`, `work`, `ship`}, the rendered SKILL.md body
  byte-length at `.claude/skills/speccy-<phase>/SKILL.md`
  is strictly less than the rendered agent body byte-length
  at `.claude/agents/speccy-<phase>.md`, and the same
  relationship holds for the Codex side
  (`.agents/skills/speccy-<phase>/SKILL.md` vs
  `.codex/agents/speccy-<phase>.toml`'s `developer_instructions`
  value); (ii) each of those six rendered SKILL.md bodies
  contains the literal substring `/agent speccy-<phase>` with
  the matching phase name and a reference to the matching
  agent file path; (iii) each of those six rendered SKILL.md
  bodies does not contain `## Steps` or `## When to use`.
  The meta-test is pure-Rust scan (no host calls, no network)
  and exits 0 against the post-T-009 working tree.

Re-render every downstream file that includes the edited or
moved shared modules so the host-pack drift check stays green.
Run all four hygiene gates.

Suggested files (non-exhaustive):

- `resources/modules/phases/speccy-tasks.md` (new — moved from `modules/skills/`)
- `resources/modules/phases/speccy-work.md` (new — moved)
- `resources/modules/phases/speccy-ship.md` (new — moved)
- `resources/modules/phases/speccy-init.md` (new — moved)
- `resources/modules/skills/speccy-tasks.md` (deleted — moved out)
- `resources/modules/skills/speccy-work.md` (deleted — moved out)
- `resources/modules/skills/speccy-ship.md` (deleted — moved out)
- `resources/modules/skills/speccy-init.md` (deleted — moved out)
- `resources/agents/.claude/skills/speccy-tasks/SKILL.md.tmpl` (body becomes stub)
- `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` (body becomes stub)
- `resources/agents/.claude/skills/speccy-ship/SKILL.md.tmpl` (body becomes stub)
- `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl` (include path updated only; body stays full)
- `resources/agents/.agents/skills/speccy-tasks/SKILL.md.tmpl` (body becomes stub)
- `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl` (body becomes stub)
- `resources/agents/.agents/skills/speccy-ship/SKILL.md.tmpl` (body becomes stub)
- `resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl` (include path updated only; body stays full)
- `resources/agents/.claude/agents/speccy-tasks.md.tmpl` (include path + description rewrite)
- `resources/agents/.claude/agents/speccy-work.md.tmpl` (include path + description rewrite)
- `resources/agents/.claude/agents/speccy-ship.md.tmpl` (include path + description rewrite)
- `resources/agents/.claude/agents/speccy-init.md.tmpl` (deleted)
- `.claude/agents/speccy-init.md` (deleted)
- All matching rendered files under `.claude/` and `.agents/` (re-rendered)
- A new meta-test file under `speccy-core/tests/`

<task-scenarios>
Given the four shared phase-body files at
`resources/modules/phases/speccy-<phase>.md` for `phase` in
{`tasks`, `work`, `ship`, `init`} after this task lands, when
each is read, then each exists. The matching paths at
`resources/modules/skills/speccy-<phase>.md` for the same four
phase names do not exist.

Given each rendered SKILL.md file at
`.claude/skills/speccy-<phase>/SKILL.md` and
`.agents/skills/speccy-<phase>/SKILL.md` for `phase` in
{`tasks`, `work`, `ship`}, when each is read, then each has
≤10 non-blank content lines below its YAML frontmatter, contains
the literal substring `/agent speccy-<phase>` for the matching
phase, contains a reference to the matching agent file path,
and does not contain the literal substrings `## Steps` or
`## When to use`.

Given each of the six rendered SKILL.md files named above, when
its body byte-length is compared against the matching agent
body byte-length, then the SKILL.md body is strictly smaller.

Given `.claude/skills/speccy-init/SKILL.md` and
`.agents/skills/speccy-init/SKILL.md` after this task lands,
when each is read, then each carries the full procedural body
sourced from `resources/modules/phases/speccy-init.md` (the
stub-shape transformation does not apply to `speccy-init`).

Given `.claude/skills/speccy-review/SKILL.md` and
`.agents/skills/speccy-review/SKILL.md` after this task lands,
when each is read, then each carries the full
verdict-consolidation body per REQ-002 / REQ-009 (the
stub-shape transformation does not apply to `speccy-review`).

Given each Claude Code agent file at
`.claude/agents/speccy-<phase>.md` for `phase` in
{`tasks`, `work`, `ship`}, when its frontmatter `description:`
field is read, then it does not contain the literal substring
`` via `context: fork` `` and does not contain the literal
substrings `Sonnet`, `Opus`, `Haiku`, `xhigh`, `medium`, `high`,
`low`, or `max` outside of code fences.

Given the post-T-009 `.claude/agents/` directory listing, when
scanned, then no file named `speccy-init.md` exists. The
matching template file at
`resources/agents/.claude/agents/speccy-init.md.tmpl` also does
not exist.

Given the post-T-009 `.codex/agents/` directory listing, when
scanned, then no file named `speccy-init.toml` exists.

Given each agent template source at
`resources/agents/.claude/agents/speccy-<phase>.md.tmpl` for
`phase` in {`tasks`, `work`, `ship`}, when read, then it
contains the literal string
`{% include "modules/phases/speccy-<phase>.md" %}` (path
post-rename) and does not contain the substring
`{% include "modules/skills/speccy-`.

Given the new pure-Rust meta-test added by this task, when
run against the post-T-009 working tree, then it exits 0
because the stub-shape invariants above all hold.

Given the existing in-tree host-pack drift-check meta-test,
when run after this task lands, then it exits 0 (every edited
template renders byte-identically to its in-tree dogfood-pack
counterpart).

Given the four hygiene gates run against the working tree at
the commit that lands this task, when each exits, then each
exit code is 0.
</task-scenarios>
</task>

