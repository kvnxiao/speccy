---
spec: SPEC-0037
spec_hash_at_generation: 8cbb9ae7787e6405e89edf0841d3d0d5ee34c5e957129a796b5af23287aa7513
generated_at: 2026-05-23T07:36:29Z
---
# Tasks: SPEC-0037 Per-task journal files — eject implementer / review / blockers from TASKS.md

<task id="T-001" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005 REQ-006 REQ-007">
## T-001: Parser and lint foundation for per-task journal files

Land the CLI-side code foundation for the new journal artifact:
parse `journal/T-NNN.md`, validate the renamed + shrunken closed
XML element set (drop `<implementer-note>`, `<retry>`, AND the
redundant `<tasks>` wrapper), and fire the new `JNL-*` lint
family and the TSK-family "no notes in TASKS.md" rule.
Documentation updates to `docs/ARCHITECTURE.md` and
`AGENTS.md` are split out to T-002 (decomposition revision on
2026-05-21). After this task lands, every TASKS.md in the
workspace — including SPEC-0037's own — will trip the new lints
(legacy elements written by T-001 through T-006 implementers
following current skill bodies, plus the redundant `<tasks>`
wrapper). Two integration tests
(`speccy-core/tests/in_tree_tasks_reports.rs::every_in_tree_tasks_md_parses_and_has_populated_scenarios`
and
`speccy-cli/tests/verify_after_migration.rs::speccy_verify_exits_zero_on_migrated_in_tree_workspace`)
are temporarily marked `#[ignore]` as part of T-001 — the
in-tree corpus violates the new parser contract until T-007's
migration sweep resolves it. T-007 removes the `#[ignore]`
attributes as the final step of the sweep.

The remaining items in T-001 form one atomic code contract:
drop the legacy element names from the allow-list (hard cutover
per the decomposition decision), accept the renamed forms with
the new attribute schemas, validate `JNL-*` lifecycle-aware per
task state, and surface the misplaced-element rule on TASKS.md.

### Parser additions

Add a new parser module for `journal/T-NNN.md` alongside the
existing `task_xml/`, `spec_xml/`, and `report_xml/` modules under
`speccy-core/src/parse/`. The journal parser reads YAML
frontmatter (`spec:`, `task:`, `generated_at:`) followed by a
chronological sequence of bare `<implementer>`, `<review>`, and
`<blockers>` element blocks. There is no wrapper element grouping
the blocks (DEC-002); the filename + frontmatter binds the file
to its task and spec.

Closed-set allow-list updates land in the existing parser surface
(per DEC-007 + DEC-008, the closed XML element set shrinks from
six to five):

- Add `implementer`, `review`, `blockers` to the allow-list.
- Remove `implementer-note`, `retry`, AND `tasks` from the
  allow-list entirely (no alias period — hard cutover per DEC-007
  and DEC-008). After this lands, the five allowed elements are
  exactly `task`, `task-scenarios`, `implementer`, `review`,
  `blockers`.
- The TASKS.md parser changes its root expectation: it now parses
  bare `<task>` children directly under the `# Tasks: SPEC-NNNN ...`
  heading (no `<tasks>` wrapper). The `spec` binding resolves
  exclusively from the frontmatter `spec:` field and the parent
  directory name; the wrapper's redundant `spec="..."` attribute
  no longer participates in binding resolution because the
  wrapper itself is rejected.
- `<implementer>` requires `date`, `model`, `round` attributes;
  legacy `session=` is no longer in the allow-list.
- `<review>` requires `date`, `model`, `persona`, `verdict`,
  `round`. `verdict` keeps its closed value set
  `{pass, blocking}`; `persona` keeps its existing persona registry.
- `<blockers>` requires `date`, `round`.
- All listed attributes are required; there are no optional
  attributes in the new schema.

Attribute value validation:

- `date`: full ISO8601 with seconds and timezone designator
  (regex `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[+-]\d{2}:\d{2})$`).
- `model`: non-empty string. Do NOT validate slash-suffix internal
  structure — that's a documented convention, not parser-enforced.
- `round`: positive integer (regex `^[1-9][0-9]*$`).
- The frontmatter's `generated_at` field uses the same ISO8601
  format as the `date` attribute.

### Round monotonic validation

Per REQ-004, the journal parser validates round counter sequence
within a single `journal/T-NNN.md` file:

- The first `<implementer>` block must have `round="1"`.
- Round counter must be monotonic non-decreasing across blocks.
- Round counter must not skip values (no jumping from round N to
  round N+2 without an intervening round N+1 block).
- Multiple blocks at the same round are allowed (one `<implementer>`
  + N `<review>` + at most one `<blockers>` per round).

These checks surface as `JNL-003` diagnostics.

### JNL-* lint family

Add `speccy-core/src/lint/rules/jnl.rs` carrying three new lint
codes, registered in `speccy-core/src/lint/registry.rs`:

- `JNL-001` (error): fires on any task at `state="pending"` whose
  corresponding `journal/T-NNN.md` exists.
- `JNL-002` (error): fires on any task at `state="completed"`
  whose corresponding `journal/T-NNN.md` is missing.
- `JNL-003` (error): fires on any task at `state="completed"`
  whose journal file has shape or binding violations — filename
  ↔ frontmatter `task:` mismatch, frontmatter `spec:` ↔ parent
  spec dir mismatch, missing/unparseable frontmatter fields,
  attribute-schema violations, or round-counter violations.

Tasks at `state="in-progress"` or `state="in-review"` are
silently skipped by all three codes — the lint never runs
mid-loop (DEC-004).

### TSK-family "no notes in TASKS.md" rule

Extend `speccy-core/src/lint/rules/tsk.rs` with a new lint code
(next available TSK-NNN slot) that fires at severity error on
any `<implementer>`, `<review>`, or `<blockers>` element parsed
inside a TASKS.md file. The rule is NOT lifecycle-gated by task
state — it fires regardless of state.

### Net-new task retention (REQ-007 regression coverage)

The TASKS.md "no notes" rule must NOT fire on net-new `<task>`
elements appended mid-implementation. State-attribute mutations
must not trigger any new diagnostic. The `spec_hash_at_generation`
mechanism is unchanged — net-new task appends do NOT trigger
TSK-003 staleness.

### Hygiene

Run the four required gates against the working tree:
- `cargo test --workspace --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

The two integration tests named above are marked `#[ignore]`
with reasons pointing at SPEC-0037 T-007; they re-enable as
part of T-007's migration sweep.

Suggested files:

- `speccy-core/src/parse/journal_xml/` (new module)
- `speccy-core/src/parse/task_xml/mod.rs` (allow-list updates, drop `<tasks>` wrapper)
- `speccy-core/src/lint/rules/jnl.rs` (new lint family)
- `speccy-core/src/lint/rules/tsk.rs` (extend with TSK-006 no-notes rule)
- `speccy-core/src/lint/registry.rs` (register new codes)
- `speccy-core/tests/lint_jnl.rs` (new test surface)
- `speccy-core/tests/in_tree_tasks_reports.rs` (`#[ignore]` the legacy-corpus test)
- `speccy-cli/tests/verify_after_migration.rs` (`#[ignore]` the migrated-workspace test)

<task-scenarios>
Given a tempdir spec at `.speccy/specs/0042-example/` with TASKS.md
declaring `<task id="T-001" state="completed">` and a
`journal/T-001.md` carrying valid frontmatter and at least one
well-formed `<implementer date="..." model="..." round="1">body</implementer>`
block, when `speccy check SPEC-0042` runs, then no `JNL-*` lint
diagnostic fires.

Given a tempdir spec with `<task id="T-001" state="pending">` and
a `journal/T-001.md` file present, when `speccy verify` runs, then
exit code is non-zero and the JSON envelope lists a `JNL-001`
diagnostic.

Given a tempdir spec with `<task id="T-002" state="completed">`
and NO `journal/T-002.md` file, when `speccy verify` runs, then
exit is non-zero and the envelope lists a `JNL-002` diagnostic.

Given a tempdir spec with a `journal/T-003.md` whose frontmatter
declares `task: T-999` mismatching the filename, when `speccy
verify` runs, then exit is non-zero and the envelope lists a
`JNL-003` filename↔frontmatter mismatch diagnostic.

Given tempdir specs at `in-progress` or `in-review` with malformed
journal files, when `speccy check` runs, then no `JNL-*`
diagnostic fires for those tasks.

Given a journal containing an `<implementer>` with `date="2026-05-21"`
(date-only, not full timestamp), when `speccy check --json` runs
against a completed task, then exit is non-zero and a parse error
names `date` and the ISO8601-with-time requirement.

Given a journal containing an `<implementer>` with empty `model`,
when `speccy check --json` runs against a completed task, then
exit is non-zero and a parse error names `model` and the
non-empty constraint.

Given a journal containing legacy `session="..."` on `<implementer>`,
when the parser reads it, then an unknown-attribute error fires.

Given a journal with `<implementer round="1">`, `<review round="1">`,
`<implementer round="2">`, `<review round="2">`, when read against
a completed task, then no round-validation diagnostic fires.

Given a journal whose first `<implementer>` has `round="2"`,
when `speccy verify` runs, then a `JNL-003` first-round-must-be-1
diagnostic fires.

Given a journal with `<implementer round="1">`, `<implementer round="3">`,
when read, then a round-skip diagnostic fires naming missing round 2.

Given a journal with `<implementer round="2">` then `<implementer round="1">`,
when read, then a non-monotonic-round diagnostic fires.

Given a TASKS.md containing legacy `<implementer-note ...>`, `<retry>`,
or `<tasks spec="...">` element, when `speccy check --json` runs,
then exit is non-zero and parse errors name each as unknown.

Given a TASKS.md with bare `<task ...>` children directly under
`# Tasks: SPEC-NNNN ...` (no wrapper, no `spec="..."` attribute
in body), when `speccy check --json` runs, then exit is 0 and the
JSON envelope enumerates the parsed `<task>` elements.

Given a TASKS.md whose `<task>` body contains an `<implementer>`,
`<review>`, or `<blockers>` element, when `speccy verify` runs,
then exit is non-zero and a TSK-family diagnostic fires naming
the misplaced element and the canonical fix at `journal/T-NNN.md`.
The rule fires regardless of task state.

Given a TASKS.md with a net-new `<task id="T-NNN" state="pending">`
appended, when `speccy check` runs, then no new diagnostic fires
and no TSK-003 staleness is triggered.

Given full lifecycle state transitions (`pending` → `in-progress`
→ `in-review` → `completed`, and `completed` → `pending` via
amend), when `speccy check` runs after each mutation, then no
lint diagnostic fires.

Given the post-task source tree, when `in_tree_tasks_reports.rs`
and `verify_after_migration.rs` are read, then both carry
`#[ignore]` attributes whose reasons name SPEC-0037 T-007 as
unblocker.

Given the four hygiene gates run against the working tree at the
landing commit (with the two named tests ignored), then each
exits 0.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-011">
## T-002: Architectural docs sweep — ARCHITECTURE.md + AGENTS.md describe the new journal shape

Update the authoritative prose docs to describe the new journal
artifact, the renamed + shrunken closed XML element set, the
`JNL-*` lint family, the TSK-family "no notes in TASKS.md" rule,
and the bare-`<task>` TASKS.md shape. Then run host-pack
regeneration via `speccy init --force` and confirm zero drift
across `.claude/`, `.codex/`, and `.agents/`.

This task was split out of T-001's original scope on
2026-05-21 (decomposition revision; see SPEC.md Changelog).
T-001 lands the Rust code + tests; T-002 lands the prose docs
that describe what T-001 built. Sequencing T-002 before the
skill-body tasks (T-004 / T-005 / T-006) ensures their reviewers
and implementers read an accurate ARCHITECTURE.md while
authoring their own edits.

### `docs/ARCHITECTURE.md` updates

Add sections describing:

- The `journal/T-NNN.md` artifact: directory location (sibling
  to `evidence/`), frontmatter shape (`spec`, `task`,
  `generated_at`), bare-element body, binding rules
  (filename ↔ task, frontmatter ↔ spec).
- The closed XML element set updated to name `implementer` and
  `blockers` in place of legacy `implementer-note` and `retry`,
  with `tasks` removed entirely. Cardinality documented as five
  (down from six).
- The TASKS.md structural shape: bare `<task>` children directly
  under the `# Tasks:` heading; binding resolves from filename +
  parent directory + frontmatter `spec:` field, not from a
  wrapper attribute.
- The new `JNL-001`, `JNL-002`, `JNL-003` lint codes with their
  state-gating rules (pending demands clean slate, completed
  demands well-formed journal, in-progress / in-review skip).
- The TSK-family "no notes elements in TASKS.md" rule.
- The element attribute schemas with the slash-suffix `model`
  convention documented as skill-layer convention (not
  parser-enforced).

The `# Review` section's `## CLI invocation` and `## State
transitions` subsections also describe the journal-based model
(reviewer subagent returns `<review>` element to orchestrator;
orchestrator writes serially to `journal/T-NNN.md`; blocking
verdicts append a `<blockers>` block to the journal).

No outdated references to `<implementer-note>`, `<retry>`, or
`<tasks spec="...">` may remain outside historical-context or
changelog sections.

### `AGENTS.md` updates

Update wherever AGENTS.md discusses the implement/review loop
or agent activity records to reference `journal/T-NNN.md`
instead of TASKS.md activity prose. Add a short
"Implementer / reviewer activity records" section pointing at
the journal grammar in ARCHITECTURE.md.

### Host-pack regeneration

After the doc edits land, run `speccy init --force --host
claude-code` and `--host codex`, then verify
`git diff --exit-code .claude .codex .agents` exits 0.

### Hygiene

The four code-level hygiene gates from T-001 stay green (no
Rust source changes). The two integration tests ignored by
T-001 remain ignored until T-007.

Suggested files:

- `docs/ARCHITECTURE.md`
- `AGENTS.md`
- `.claude/`, `.codex/`, `.agents/` (auto-regenerated by
  `speccy init --force`; no manual edits to materialized files)

<task-scenarios>
Given the post-task source tree, when `docs/ARCHITECTURE.md`
is searched for the literal substrings `journal/T-NNN.md` and
`JNL-001`, then both substrings appear in the file body.

Given the post-task source tree, when `docs/ARCHITECTURE.md`
is searched for `<implementer-note>`, `<retry>`, or `<tasks spec=`
outside the explicit retirement-cutover paragraph, then no
matches appear in live-workflow prose.

Given the post-task source tree, when `AGENTS.md` is searched
for `journal/T-NNN.md`, then it appears in the prose describing
the implement/review loop.

Given the materialized host packs, when `speccy init --force`
is run for both hosts, then `git diff --exit-code .claude .codex
.agents` exits 0.

Given the four hygiene gates run against the working tree at the
landing commit, then each exits 0; the two integration tests
ignored by T-001 remain ignored.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-009">
## T-003: Reviewer return contract requires `model` on returned `<review>`

Update `resources/modules/personas/verdict_return_contract.md` so
each reviewer persona's returned `<review>` element carries a
required `model` attribute (with the slash-suffix convention for
effort). The orchestrator in `speccy-review.md` transcribes the
model verbatim from each subagent's returned `<review>` into the
per-task journal entry. Without this contract update, the
orchestrator cannot reliably record per-reviewer model identity
across heterogeneous reviewer subagents.

The contract must explicitly name:

- `model` as required on the returned `<review>` element.
- The slash-suffix encoding for effort (e.g.
  `model="claude-opus-4.7[1m]/low"`).
- The orchestrator-side transcription rule: verbatim copy of the
  `model` attribute into the journal entry; no inference from
  skill-pack identity alone.
- A no-substitute clause: if a reviewer subagent returns a
  `<review>` element without `model`, the orchestrator surfaces
  the contract violation rather than inventing a model value.

The implementer side of the contract is unchanged.

Worked-example `<review>` snippets in
`resources/modules/personas/inline_note_format.md` and in each
of the six `reviewer-{architecture,business,docs,security,style,tests}.md`
source files MUST carry `model="..."` on their example `<review>`
elements (otherwise reviewer subagents following the worked
example would produce a non-conforming `<review>` that the
no-substitute clause rejects).

Suggested files:

- `resources/modules/personas/verdict_return_contract.md`
- `resources/modules/personas/inline_note_format.md`
- `resources/modules/personas/reviewer-{architecture,business,docs,security,style,tests}.md`

<task-scenarios>
Given the post-task source tree, when
`resources/modules/personas/verdict_return_contract.md` is read,
then its body contains the four contract clauses (required model
attribute, slash-suffix encoding, verbatim transcription,
no-substitute clause).

Given `inline_note_format.md` and each of the six
`reviewer-*.md` source files, when read, then each worked-example
`<review>` element carries a `model="..."` attribute.

Given the materialized host packs, when `speccy init --force` is
run for both hosts and `git diff --exit-code` is checked, then
no drift is reported.

Given the four hygiene gates run against the working tree at the
landing commit, then each exits 0.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-008">
## T-004: `speccy-work.md` skill writes `<implementer>` to journal

Update `resources/modules/phases/speccy-work.md` so the
implementer phase body instructs the implementer agent to append
an `<implementer>` block to `journal/T-NNN.md` instead of inlining
an `<implementer-note>` inside the `<task>` body in TASKS.md.

The instruction must:

- Name the journal file path (`journal/T-NNN.md`) as the write
  target.
- Tell the implementer to create the file with valid YAML
  frontmatter (`spec: SPEC-NNNN`, `task: T-NNN`,
  `generated_at: <ISO8601>`) if it does not yet exist on round 1.
- Name the required attributes on `<implementer>`: `date`,
  `model`, `round` — all required, no optional.
- Document the slash-suffix convention for `model`.
- Tell the implementer that `round` is a monotonic integer from
  1, incremented on each post-blocker retry attempt.
- Remove any remaining references to writing `<implementer-note>`
  into TASKS.md.

Suggested files:

- `resources/modules/phases/speccy-work.md`

<task-scenarios>
Given the post-task source tree, when
`resources/modules/phases/speccy-work.md` is read, then its body
names `journal/T-NNN.md` as the write target for `<implementer>`
blocks.

Given the same file grepped for `<implementer-note`, then no
matches appear in live workflow prose (anti-write prohibition
guards excepted).

Given the same file, when read, then it documents the required
`date`, `model`, `round` attributes, the slash-suffix convention,
and the monotonic-from-1 semantics of `round`.

Given the materialized host packs after `speccy init --force`,
when `git diff --exit-code` is checked, then no drift is reported.

Given the four hygiene gates run against the working tree at the
landing commit, then each exits 0.
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-008">
## T-005: `speccy-review.md` skill writes `<review>` blocks to journal

Update `resources/modules/skills/speccy-review.md` so the
reviewer-fan-out orchestrator appends each persona's `<review>`
block to `journal/T-NNN.md` instead of into the `<task>` body in
TASKS.md.

The instruction must:

- Name the journal file path (`journal/T-NNN.md`) as the write
  target for `<review>` blocks.
- Preserve the existing DEC-008 concurrency contract: reviewer
  subagents return `<review>` elements to the orchestrator; the
  orchestrator writes them to the journal serially.
- Tell the orchestrator to transcribe the `model` attribute
  verbatim from each returned `<review>` per
  `verdict_return_contract.md` (REQ-009 from T-003).
- Name the required attributes on `<review>`: `date`, `model`,
  `persona`, `verdict`, `round` — all required.
- Remove any remaining references to writing `<review>` blocks
  into TASKS.md.
- Spawn-prompt example snippets MUST include `model` in the
  example `<review>` element.
- Replace `<retry>` references with `<blockers>` (renamed by T-001).

Also update `.tmpl` frontmatter `description:` strings in
`resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`
and `.agents/skills/speccy-review/SKILL.md.tmpl` to reflect the
`<blockers>` rename.

Suggested files:

- `resources/modules/skills/speccy-review.md`
- `resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl`
- `resources/agents/.agents/skills/speccy-review/SKILL.md.tmpl`

<task-scenarios>
Given the post-task source tree, when
`resources/modules/skills/speccy-review.md` is read, then its body
names `journal/T-NNN.md` as the write target for `<review>` blocks
and the legacy references to appending into the `<task>` body in
TASKS.md are absent.

Given the same file, when read, then it preserves the DEC-008
serial-orchestrator-write contract.

Given the same file, when read, then it documents the verbatim
`model` transcription rule and the required attribute set
(`date`, `model`, `persona`, `verdict`, `round`).

Given the same file grepped for `<retry`, then no matches appear
in live workflow prose.

Given the materialized host packs after `speccy init --force`,
when `git diff --exit-code` is checked, then no drift is reported.

Given the four hygiene gates run against the working tree at the
landing commit, then each exits 0.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-008">
## T-006: `speccy-amend.md` skill writes amendment-driven `<blockers>` to journal

Update `resources/modules/skills/speccy-amend.md` so the amend
flow appends a `<blockers>` element to `journal/T-NNN.md` when
flipping a completed task back to `state="pending"` due to a SPEC
change. Today's language appends the legacy `<retry>` element
inside the `<task>` body in TASKS.md; that destination must change
to the journal file and the element name must rename to
`<blockers>`.

The instruction must:

- Name the journal file path (`journal/T-NNN.md`) as the write
  target for amendment-driven blocker entries.
- Use the renamed element name `<blockers>` (not `<retry>`).
- Name the required attributes: `date`, `round`.
- Describe the `round = N+1` rule (or `round=1` if no prior
  journal) for amendment-driven blocker entries.
- Preserve the existing semantic: `<blockers>` body describes
  the amendment-driven blocker directive.
- The state-flip semantics (`completed` → `pending`) are
  unchanged; only the write target and element name change.

Also sweep `resources/modules/personas/verdict_return_contract.md`
for any remaining `<retry>` references in the lede (T-005 docs
reviewer forward observation).

Suggested files:

- `resources/modules/skills/speccy-amend.md`
- `resources/modules/personas/verdict_return_contract.md` (sweep)

<task-scenarios>
Given the post-task source tree, when
`resources/modules/skills/speccy-amend.md` is read, then the
amend-flow step names `journal/T-NNN.md` as the destination for
amendment-driven `<blockers>` blocks.

Given the same file grepped for `<retry`, then no matches appear
in live workflow prose.

Given the same file, when read, then it documents the required
attribute set on `<blockers>` (`date`, `round`) and the
`round = N+1` rule.

Given the same file, when read, then it preserves the
`completed` → `pending` state-flip semantics.

Given `verdict_return_contract.md`, when read, then no stale
`<retry>` or "retry note" references remain in the lede.

Given the materialized host packs after `speccy init --force`,
when `git diff --exit-code` is checked, then no drift is reported.

Given the four hygiene gates run against the working tree at the
landing commit, then each exits 0.
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-010">
## T-007: Migration of SPEC-0001 through SPEC-0037 (all 37 in-tree specs)

Migrate every legacy `<implementer-note session="...">`,
`<review persona="..." verdict="...">`, and `<retry>` element
from TASKS.md in `.speccy/specs/0001-*/` through
`.speccy/specs/0037-*/` (inclusive of SPEC-0037 itself) into
per-task `journal/T-NNN.md` files under the new schema, AND
strip the redundant `<tasks spec="SPEC-NNNN">...</tasks>` wrapper
from every TASKS.md so its `<task>` children sit bare under the
`# Tasks:` heading. Both transformations land in the same sweep
because the parser cutover in T-001 made both forms unparseable
simultaneously.

### Why SPEC-0037 is included

Implementer + reviewer agents working on T-001 through T-006
follow the un-updated skill bodies (T-004 / T-005 / T-006 update
them, but their own implementer entries land before each skill
swap takes effect). Those agents write activity prose in legacy
format directly into SPEC-0037's own TASKS.md. T-001's parser
cutover immediately makes that format unparseable, leaving
SPEC-0037's TASKS.md in the same broken state as every historical
spec. T-007 sweeps all 37 specs uniformly.

T-007's own implementer entry is the exception — by the time the
migration agent runs, the new schema is the only valid format and
the agent writes its own `<implementer>` block directly to
`.speccy/specs/0037-task-journal-files/journal/T-007.md` in the
new format.

### Per-spec migration procedure

For each spec directory in numerical order (0001 first,
0037 last):

1. Read the legacy TASKS.md to inventory its
   `<implementer-note>`, `<review>`, and `<retry>` elements
   grouped by task ID, plus the location of the `<tasks
   spec="SPEC-NNNN">` opening and `</tasks>` closing wrapper.
2. For each task with activity-prose elements, create
   `.speccy/specs/NNNN-slug/journal/T-NNN.md` with frontmatter
   (`spec`, `task`, `generated_at`) and a body containing a
   chronological sequence of `<implementer>`, `<review>`,
   `<blockers>` elements under the new schema.
3. Element renames: `<implementer-note>` → `<implementer>`;
   `<retry>` → `<blockers>`; `<review>` keeps name but gains
   required `date`, `model`, `round`; `<tasks spec="...">`
   wrapper removed entirely.
4. Attribute inference from legacy `session="..."` strings:
   parse the date into ISO8601 form; map attempt-N → `round=N`;
   `model` uses exact string where named, else
   `claude-opus-4.7[1m]` post-2026-05-21 or
   `claude-sonnet-4-6[1m]/medium` pre-2026-05-21, or
   `claude-unknown` if genuinely unknowable.
5. Strip the legacy elements from TASKS.md after the journal
   file is written.
6. Strip the `<tasks spec="SPEC-NNNN">` wrapper from TASKS.md.
7. Run `speccy check SPEC-NNNN` after each spec's migration.

### SPEC-0037 special handling

For SPEC-0037 itself: activity prose for T-001..T-006 is migrated
to `journal/T-001.md` through `journal/T-006.md`. T-007's own
`<implementer>` block is written directly to `journal/T-007.md`
in the new format. `<task>` definitions for T-001..T-007 stay
byte-identical (only activity prose is removed).

### Out-of-scope guards

Per REQ-010's done-when:
- All historical SPEC.md, REPORT.md, evidence/ stay byte-identical.
- Non-activity parts of TASKS.md (frontmatter, task definitions,
  `<task-scenarios>` bodies) stay byte-identical.

### Hygiene

After all 37 specs are migrated:
- `speccy verify` exits 0 across the workspace.
- Four hygiene gates pass.
- Host-pack drift-check meta-test stays green.
- The two `#[ignore]` attributes on the corpus tests
  (`every_in_tree_tasks_md_parses_and_has_populated_scenarios`,
  `speccy_verify_exits_zero_on_migrated_in_tree_workspace`)
  are removed as the final step.

Suggested files:

- `.speccy/specs/0001-*/TASKS.md` through `.speccy/specs/0037-*/TASKS.md`
- `.speccy/specs/0001-*/journal/T-NNN.md` through `.speccy/specs/0037-*/journal/T-NNN.md`
- `speccy-core/tests/in_tree_tasks_reports.rs` (remove `#[ignore]`)
- `speccy-cli/tests/verify_after_migration.rs` (remove `#[ignore]`)

<task-scenarios>
Given the post-task source tree, when each TASKS.md file under
`.speccy/specs/000[1-9]-*/`, `.speccy/specs/00[1-3][0-6]-*/`,
and `.speccy/specs/0037-task-journal-files/` is searched for the
literal substrings `<implementer-note`, `<implementer `, `<review `,
`<retry`, `<blockers`, and `<tasks`, then no matches appear in any
of the 37 files.

Given the post-task source tree, when each of those 37 TASKS.md
files is parsed, then exactly one `# Tasks:` heading appears
followed immediately by one-or-more bare `<task>` children with
no intervening wrapper element.

Given the post-task source tree, when `speccy verify` runs across
the workspace, then exit is 0 with zero `JNL-*` errors, zero
TSK-family "no notes" errors, and zero parse errors.

Given the post-task source tree, when each spec directory at
`.speccy/specs/0001-*/` through `.speccy/specs/0037-*/` is
inspected, then every `state="completed"` task in its TASKS.md
has a corresponding `journal/T-NNN.md` file with valid
frontmatter and well-formed `<implementer>` / `<review>` /
`<blockers>` elements conforming to REQ-003's attribute schema.

Given the post-task source tree, when
`.speccy/specs/0037-task-journal-files/journal/T-001.md` through
`journal/T-006.md` are read, then each carries the activity prose
that lived inside the corresponding `<task>` body in SPEC-0037's
pre-migration TASKS.md, with persona, verdict, and body prose
preserved (best-effort attribution under the new schema).

Given the post-task source tree, when
`.speccy/specs/0037-task-journal-files/journal/T-007.md` is read,
then it carries the migration agent's own `<implementer>` block
under the new schema.

Given the post-task source tree, when each historical SPEC.md is
diffed against its HEAD~ version, then the diff is empty for all
36 historical SPEC.md files.

Given the post-task source tree, when each historical REPORT.md
and `evidence/` directory is diffed against HEAD~, then the diff
is empty.

Given the post-task source tree, when each TASKS.md's frontmatter
and `<task>` element definitions are diffed against HEAD~, then
only the activity-prose elements and the `<tasks>` wrapper are
removed; frontmatter and task-definition surfaces stay
byte-identical across all 37 files.

Given the four hygiene gates run against the working tree at the
landing commit (with the two formerly-`#[ignore]`d corpus tests
un-ignored and passing), then each exits 0.

Given the materialized host packs after `speccy init --force`,
when `git diff --exit-code .claude .codex .agents` runs, then no
drift is reported between resources/ source and materialized packs.
</task-scenarios>
</task>
</content>
</invoke>