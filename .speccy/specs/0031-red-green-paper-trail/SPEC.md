---
id: SPEC-0031
slug: red-green-paper-trail
title: Red-green paper trail in task closure
status: implemented
created: 2026-05-18
supersedes: []
---

# SPEC-0031: Red-green paper trail in task closure

## Summary

Today the implementer handoff template at
`resources/modules/prompts/implementer.md` asks an implementer to
record six markdown-bullet sub-fields inside the
`<implementer-note session="...">` element appended to TASKS.md when
flipping a task to `state="in-review"`: `Completed`, `Undone`,
`Commands run`, `Exit codes`, `Discovered issues`, and `Procedural
compliance`. The `Commands run` / `Exit codes` pair is the only
deterministic evidence the implementer leaves behind: every other
field is prose. The reviewer-tests persona at
`resources/modules/personas/reviewer-tests.md` then judges the diff,
the `<task-scenarios>` block, and the SPEC requirement scenarios for
adversarial test quality without any visibility into whether the
tests were written **before** the implementation or after it.

The gap matters because structural Check-mapping
(`<requirement> ← <scenario>` count parity, lint codes, `<task>` ←
`<requirement>` coverage links) proves count, not order. A
post-hoc-written test is more likely to be tautological — the
implementer wrote it to match the code they already had in mind —
and tautological tests are exactly the class reviewer-tests is meant
to catch. Without red-state visibility, the reviewer-tests persona
must reverse-engineer test adversarialness from the diff alone,
which is unreliable.

SPEC-0031 upgrades the closure handoff to require a captured
red-then-green paper trail per implementer session. The verbatim
runner output is externalized to a per-task evidence file at
`.speccy/specs/NNNN-slug/evidence/T-NNN.md` rather than inlined into
the `<implementer-note>` body, so that TASKS.md — read repeatedly by
implementer prompts, every reviewer persona, REPORT.md generation,
and `speccy verify` — does not absorb the runner-output volume. The
handoff template's `Commands run` + `Exit codes` parallel-list pair
collapses into a single `Hygiene checks` table covering the
deterministic project gates (lint, fmt, build, full-suite test); the
test-side adversarial paper trail moves to the new `Evidence:`
field, which carries the evidence file path plus a one-line red→green
summary.

The reviewer-tests persona reads the referenced evidence file and
treats absence as a blocking verdict; it also treats fabricated-looking
output (missing framework artifacts, test names not present in the
diff, identical pre/post indicators) as blocking. No other persona
loads the evidence file: business, security, style, architecture,
and docs reviewers stay anchored on diff + SPEC, preserving the
adversarial property the multi-persona fan-out exists to provide
(this mirrors the SPEC-0029 redaction discipline at a different
boundary).

The implementer prompt narrates the natural workflow so writing the
evidence file is the path of least resistance: write the failing
test, run it, capture the red output to the evidence file,
implement the code, run again, capture the green output, run hygiene,
append the handoff note, flip state. The prompt accepts
compile-failure output as a legitimate red signal (a missing symbol
blocking a test from running is still "the test cannot pass without
this implementation"). The prompt stays framework-agnostic — no
per-framework anchor patterns — and references
`.speccy/examples/evidence.md` via progressive disclosure rather
than inlining a 30-line worked example into every implementer
invocation.

The source-of-truth example lives at
`resources/modules/examples/evidence.md` (a new directory under the
embedded `RESOURCES` bundle). `speccy init` ejects it host-agnostically
to `.speccy/examples/evidence.md` in the user's project, alongside
the existing host-specific skill-pack ejection. This adds one new
output to `speccy init` but no new commands and no new flags; the
new ejection path is the smallest CLI-side surface that lets the
implementer prompt reference a known location.

The change is bounded: one new resource file, three prompt-module
edits (implementer prompt, reviewer-tests prompt, reviewer-tests
persona), one new ejection path in `speccy init` with parallel
host-agnostic-pack tests, one in-tree committed example file kept in
sync via the existing drift-check pattern, and one BACKLOG.md hygiene
edit adding F-9 as the follow-up to migrate other inline examples
across the persona/prompt corpus.

## Goals

<goals>
- `resources/modules/prompts/implementer.md` instructs every
  implementer-note that flips a task from `state="in-progress"` to
  `state="in-review"` to reference an evidence file at
  `.speccy/specs/NNNN-slug/evidence/T-NNN.md` and to carry a single
  `Evidence:` field with a one-line red→green summary.
- The handoff template's parallel `Commands run` / `Exit codes`
  fields collapse into a single `Hygiene checks` markdown table
  with `Command | Status` columns where `Status` is `pass (exit N)`
  or `fail (exit N)`.
- `resources/modules/personas/reviewer-tests.md` instructs the
  reviewer-tests persona to read the referenced evidence file and
  treat absence as a `verdict="blocking"` review.
- `resources/modules/personas/reviewer-tests.md` enumerates the
  fabrication patterns the persona must treat as blocking (output
  without framework artifacts, test names not present in the diff,
  identical red/green indicators, etc.) without prescribing
  per-framework anchor strings.
- The other five built-in personas (business, security, style,
  architecture, docs) carry no instruction to load the evidence
  file; their adversarial stance stays anchored on diff + SPEC.
- A new source file at `resources/modules/examples/evidence.md`
  ships a canonical worked example covering both a red+green
  session and a no-test-delta retry session.
- `speccy init` ejects `resources/modules/examples/*` host-agnostically
  to `.speccy/examples/*` in the user's project, parallel to the
  existing host-pack ejection but emitted regardless of host
  choice.
- The in-tree workspace gains a committed
  `.speccy/examples/evidence.md` kept byte-identical to the
  embedded `resources/modules/examples/evidence.md` via the
  existing drift-check meta-test pattern.
- `.speccy/BACKLOG.md` gains an F-9 entry tracking the follow-up
  migration of other inline examples across personas/prompts.
- All four standard-hygiene gates (`cargo test`, `cargo clippy`,
  `cargo +nightly fmt --all --check`, `cargo deny check`) exit 0
  against the post-SPEC workspace.
</goals>

## Non-goals

<non-goals>
- No CLI subcommand surface change. No `speccy evidence`,
  `speccy verify-evidence`, `--check-evidence` flag, or
  configuration knob. The evidence file is a writer-side and
  reviewer-side convention enforced by skill-prompt judgment, not
  by deterministic CLI logic.
- No per-framework anchor checklist in the reviewer-tests persona
  ("cargo: look for `test result: FAILED`"; "pnpm: look for ` ✗ `";
  "pytest: look for `FAILED`"). The persona stays framework-agnostic
  and relies on fresh-context judgment.
- No instruction for personas other than `reviewer-tests` to load
  the evidence file. Business, security, style, architecture, and
  docs reviewers carry no Evidence-related instructions and pay
  zero context cost for the new file.
- No inlining of the full evidence-file shape into the implementer
  prompt body. The prompt includes a 3-line minimal sketch and
  names `.speccy/examples/evidence.md` via the host Read primitive
  for progressive disclosure.
- No host-native deployment of the example file. The example does
  not land under `.claude/skills/speccy-work/examples/` or
  `.codex/agents/speccy-work/examples/`; it lives at
  `.speccy/examples/evidence.md` regardless of host. The host-native
  principle applies to harness-loaded skills, not to LLM-read
  reference content.
- No retroactive evidence-file backfill for tasks that shipped
  before this SPEC lands. Existing `<implementer-note>` elements in
  in-tree TASKS.md files (carrying `Commands run` / `Exit codes`
  markdown sub-bullets under the legacy template) stay verbatim;
  the new shape applies to implementer-notes written after this
  SPEC ships.
- No exemption mechanism for tasks whose `<task-scenarios>` block
  is "non-executable" (e.g. pure prose or doc-only slices). Every
  task must produce evidence even if the runnable command is a
  `grep`, `test -f`, or `cargo build` rather than a unit-test
  runner. The exemption surface would otherwise become a policy
  axis every persona must reason about.
- No inclusion of evidence files in `<spec-hash>` or `<tasks-hash>`
  staleness computation. Evidence is a downstream byproduct of
  implementation, not part of the spec/tasks contract; bringing it
  into the hash would force retries to re-trigger staleness alerts.
- No fold-in of evidence content into REPORT.md. The report agent
  continues to summarize the implementation narratively and does
  not inline runner output. Evidence files remain available on
  disk for post-merge spelunking.
- No verbatim red/green output inlined into the
  `<implementer-note>` body. The Evidence field carries only a
  path and a one-line summary; the verbatim output lives in the
  evidence file.
- No conditional rendering of the implementer-prompt workflow
  narration based on task kind ("if this is a doc task, skip the
  red phase"). The narration is uniform; framework-agnostic
  guidance handles edge cases.
- No XML parser surface change. The evidence file format (markdown
  with embedded `<evidence>` / `<session>` / `<red>` / `<green>`
  tags for LLM parseability) is a writer/reader convention; no Rust
  type, no parser whitelist entry, no element-tree validation.
- No automatic git-history inspection in reviewer-tests. The
  persona judges evidence content, not commit ordering. R-4
  (commit-order TDD check) remains rejected in the backlog for
  the same gameability/coupling reasons.
- No bump of `schema_version` in any frontmatter. The TASKS.md
  schema absorbs a renamed sub-bullet (`Commands run` → `Hygiene
  checks` table) and a new sub-bullet (`Evidence:`) inside
  `<implementer-note>` bodies, but those bodies are unstructured
  markdown payload to the parser (per SPEC-0029 DEC-004); no wire
  format changes.
- No transitional grandfathering for the prompt template. The
  shipped `resources/modules/prompts/implementer.md` only instructs
  the new shape post-SPEC; the previous `Commands run` / `Exit
  codes` template is retired in the same commit that ships the new
  one. Already-shipped `<implementer-note>` bodies using the old
  shape stay verbatim (no migration needed: the field renaming is
  a writer-side instruction change, not a stored-data shape).
</non-goals>

## User Stories

<user-stories>
- As an implementer flipping a task to `state="in-review"`, I want
  the prompt to narrate the red→green workflow in execution order
  so writing the evidence file is the path of least resistance, not
  a separate ceremony I might forget.
- As an implementer working in a language without a unit-test
  runner for a slice (e.g. a doc edit, a config tweak, a
  prompt-template update), I want the prompt to accept a scoped
  `grep` / `test -f` / `cargo build` invocation as legitimate
  red→green evidence so I am not forced to invent a test harness.
- As an implementer running into a compile error when I write a
  test before its referenced symbol exists, I want the prompt to
  acknowledge the compile failure as a valid red signal so I do
  not waste time fabricating a runtime-style failure.
- As the reviewer-tests persona reviewing a task that the
  implementer claims is complete, I want the rendered prompt to
  load the referenced evidence file via my Read primitive so I can
  judge whether the captured red→green trail is real evidence or a
  post-hoc fabrication.
- As the reviewer-tests persona, I want the persona definition to
  enumerate fabrication patterns (output without framework
  artifacts, test names absent from diff, identical red/green
  indicators) so I have a calibrated bar for blocking verdicts
  rather than ad-hoc reasoning.
- As any other reviewer persona (business, security, style,
  architecture, docs), I want my prompt to carry zero
  evidence-related instructions so I judge the diff on its own
  merits and do not anchor on the implementer's claimed test
  outcomes.
- As an implementer running on a retry pass, I want the evidence
  file at `.speccy/specs/NNNN-slug/evidence/T-NNN.md` to be
  append-only so my prior session's red→green proof is preserved
  alongside my new session's, building an audit trail of every
  attempt.
- As an implementer whose retry session changes no tests (e.g. a
  comment-only cleanup), I want a no-test-delta session shape to
  record briefly what the session did without inventing fake
  red/green output.
- As the `speccy init` invoker bootstrapping a new project, I want
  `.speccy/examples/evidence.md` to be ejected automatically
  alongside the host-specific skill pack so the implementer prompt
  can reference a known location.
- As a maintainer watching the in-tree drift checks, I want the
  committed `.speccy/examples/evidence.md` to match the embedded
  `resources/modules/examples/evidence.md` byte-for-byte under the
  same meta-test discipline as the host-pack drift check.
- As a maintainer reading the backlog after this SPEC ships, I
  want an F-9 entry naming the follow-up to migrate other inline
  examples across personas/prompts to progressive disclosure, so
  the pattern this SPEC introduces is not orphaned.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Implementer handoff template collapses Commands run / Exit codes into a Hygiene checks table and adds an Evidence field

`resources/modules/prompts/implementer.md` is edited so that the
"Handoff template" section instructs the implementer to append an
`<implementer-note session="...">` element block whose body
contains the following six fields in order:

- `Completed:` — unchanged
- `Undone:` — unchanged
- `Hygiene checks:` — **new field name**, replaces the previous
  `Commands run:` + `Exit codes:` parallel-list pair. Body is a
  markdown table with exactly two columns, `Command` and `Status`.
  Each row carries one deterministic project gate (lint, fmt,
  build, full-suite test, dep audit, etc.) with `Status` rendered
  as `pass (exit N)` or `fail (exit N)`.
- `Evidence:` — **new field**, body is the project-relative path to
  the per-task evidence file (e.g.
  `.speccy/specs/0031-red-green-paper-trail/evidence/T-001.md`)
  followed by a one-line red→green summary delimited from the path
  with ` — `. The one-line summary names the scoped command and its
  red/green exit codes (e.g.
  `red: cargo test -p speccy-core foo → exit 101 / green: cargo test -p speccy-core foo → exit 0`).
- `Discovered issues:` — unchanged
- `Procedural compliance:` — unchanged

The previous `Commands run:` and `Exit codes:` field names are
retired from the prompt; the prompt does not document a transitional
form that accepts either.

<done-when>
- `resources/modules/prompts/implementer.md`, after this SPEC's
  task lands, contains the literal substring `Hygiene checks:` at
  least once inside its handoff-template section.
- The same file contains the literal substring `Evidence:` at
  least once inside its handoff-template section.
- The same file does NOT contain the literal substrings
  `Commands run:` or `Exit codes:` inside its handoff-template
  section.
- The handoff-template section documents the `Hygiene checks` body
  as a `| Command | Status |` markdown table with `pass (exit N)`
  /`fail (exit N)` cell values demonstrated.
- The handoff-template section documents the `Evidence` body shape
  as `<path> — red: <cmd> → exit N / green: <cmd> → exit 0` (or
  substantially equivalent prose naming the path-plus-summary
  shape).
- The six fields appear in the order documented in this REQ:
  Completed, Undone, Hygiene checks, Evidence, Discovered issues,
  Procedural compliance.
- The handoff-template section explicitly notes that every field
  is required and that an empty field carries the literal
  placeholder `(none)` (mirroring the existing convention for
  `Discovered issues` and `Procedural compliance`).
</done-when>

<behavior>
- Given the post-SPEC `resources/modules/prompts/implementer.md`,
  when its handoff-template section is read, then the field names
  in order are `Completed`, `Undone`, `Hygiene checks`, `Evidence`,
  `Discovered issues`, `Procedural compliance`.
- Given the same file, when grepped for `Commands run:` or
  `Exit codes:` inside the handoff-template section, then zero
  matches are found.
- Given the same file, when its `Hygiene checks` body is read,
  then the documented shape is a two-column markdown table with
  `Command` and `Status` columns, not a parallel-list pair.
- Given the same file, when its `Evidence` body shape is read,
  then the documented shape is `<path> — red: ... / green: ...`
  (or substantially equivalent prose), specifying that the body is
  one line carrying the path plus the red→green summary.
- Given the rendered implementer prompt for any task in any spec
  after this SPEC ships, when the rendered prompt is inspected,
  then it carries the new field names — `Commands run` / `Exit
  codes` no longer appear in shipped prompt output.
</behavior>

<scenario id="CHK-001">
Given the file `resources/modules/prompts/implementer.md` after
this SPEC's task lands, when grepped for the literal substring
`Hygiene checks:` inside the file, then at least one match exists
inside its handoff-template section.

Given the same file, when grepped for the literal substrings
`Commands run:` and `Exit codes:`, then zero matches are found in
the handoff-template section (matches inside a "Changes from prior
template" or similar historical-context block are out of scope —
the canonical instruction must name only the new field).

Given the same file, when grepped for the literal substring
`Evidence:`, then at least one match exists inside its
handoff-template section.

Given the same file, when its handoff-template section is parsed
as ordered markdown bullets, then the six field names appear in
the order: `Completed`, `Undone`, `Hygiene checks`, `Evidence`,
`Discovered issues`, `Procedural compliance`.

Given the same file, when the `Hygiene checks` field's documented
body is read, then it specifies a markdown table with exactly two
columns named `Command` and `Status` and demonstrates at least one
row with `Status` rendered as `pass (exit 0)` or `fail (exit N)`.

Given the same file, when the `Evidence` field's documented body
is read, then it specifies that the body is a project-relative
path to `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md` followed
by a delimiter and a one-line summary naming the scoped command
and its red and green exit codes.

Given the rendered implementer prompt produced by `speccy implement
SPEC-NNNN/T-NNN` for any task after this SPEC ships, when the
rendered prompt's handoff-template section is captured, then it
contains the new field names verbatim and does not contain the
retired `Commands run:` or `Exit codes:` field labels.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Evidence file shape and lifecycle are documented

The evidence file at
`.speccy/specs/NNNN-slug/evidence/T-NNN.md` is a single file per
task, append-only across retry sessions, with a documented shape
that the implementer prompt and the example file both reference:

- The file opens with an H1 markdown header naming the task (e.g.
  `# Evidence: T-001`).
- The file body is wrapped in an
  `<evidence task="T-NNN" spec="SPEC-NNNN">…</evidence>` element
  block. The element is a writer/reader convention for LLM
  parseability; it is not parsed by the Speccy CLI.
- Each implementer session that flips the task to
  `state="in-review"` appends one new session block at the end of
  the file. Session blocks are not reordered, not edited
  retroactively, and not deleted on retry.
- A session that changed tests uses the shape:

  ```markdown
  ## Session <session-id> (attempt N)

  Command: `<scoped command>`

  <red exit="N">
  <verbatim runner output for the failing run>
  </red>

  <green exit="0">
  <verbatim runner output for the passing run>
  </green>
  ```

- A session that did not change tests uses the shape:

  ```markdown
  ## Session <session-id> (attempt N, no test delta)

  <single sentence describing what the session did instead>
  ```

- The implementer prompt explicitly accepts compile-failure output
  as a legitimate red phase: if writing a test that references a
  missing symbol causes a build failure rather than a runtime test
  failure, the build-failure output is the red phase and the
  passing build (with the symbol implemented) is the green phase.
- The implementer prompt stays framework-agnostic: it does not name
  cargo-, pnpm-, pytest-, or any other framework-specific anchor
  strings; the implementer captures whatever their toolchain emits.

<done-when>
- `resources/modules/prompts/implementer.md` documents the evidence
  file path shape `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md`
  literally inside its workflow-narration section.
- The same file documents both the "session changed tests" shape
  (with `<red>` / `<green>` element blocks) and the "no test
  delta" shape (single-sentence summary) as the two acceptable
  session forms.
- The same file documents the append-only invariant: a session
  block is added at the end of the file, prior sessions are
  preserved verbatim.
- The same file documents that a build-time failure (compile
  error preventing a test from running) is an acceptable red phase.
- The same file does not name any per-framework anchor string
  (e.g. `test result: FAILED`, ` ✗ `, `FAILED:`) — the framework
  detection lives in the implementer's local toolchain, not in the
  prompt.
- The evidence-file path inside the `Evidence:` field
  (REQ-001) and the path documented in the workflow narration
  agree on the shape `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md`.
</done-when>

<behavior>
- Given the post-SPEC `resources/modules/prompts/implementer.md`,
  when its workflow-narration section is read, then the evidence
  file path is documented as
  `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md`.
- Given the same file, when the session-block shape is read,
  then both the test-change shape and the no-test-delta shape are
  documented as the two acceptable forms.
- Given the same file, when the append-only invariant is
  documented, then the prompt names this discipline explicitly
  (e.g. "append at the end; never edit or remove a prior
  session").
- Given the same file, when grepped for `cargo`, `pnpm`, `pytest`,
  `jest`, `vitest`, or any other framework-specific name inside
  the evidence-file shape documentation, then zero matches are
  found — the framework is the implementer's local concern.
- Given the same file, when the compile-failure-as-red allowance
  is read, then it explicitly states that a build-time failure
  caused by writing a test against a missing symbol is a
  legitimate red phase.
</behavior>

<scenario id="CHK-002">
Given the file `resources/modules/prompts/implementer.md` after
this SPEC's task lands, when grepped for the literal substring
`.speccy/specs/`, then at least one match exists inside the
workflow-narration section that documents the evidence file path
shape `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md` (the spec-id
folder and task-id filename are placeholders the implementer
substitutes).

Given the same file, when grepped for the literal substring
`<red exit=` and `<green exit=`, then at least one match exists
for each inside the workflow-narration section's documented
session-block shape.

Given the same file, when grepped for the literal substring
`no test delta`, then at least one match exists inside the
workflow-narration section documenting the no-test-change session
shape.

Given the same file, when grepped for the literal substring
`compile`, `build error`, or `cannot find` (case-insensitive), then
at least one match exists inside the workflow-narration section
that documents compile-failure-as-red.

Given the same file, when grepped for the literal substrings
`cargo`, `pnpm`, `pytest`, `jest`, `vitest`, `mocha`, or `rspec`
inside the section that documents the evidence-file shape (i.e.
between the workflow-narration heading and the next sibling
heading), then zero matches are found — the section stays
framework-agnostic.

Given two attempts at the same task (a first attempt that runs the
implementer once and a retry attempt that runs the implementer
again after a blocking review), when the resulting
`evidence/T-NNN.md` file is inspected, then it contains exactly two
`## Session` headers in source order — the first carrying the
initial attempt's session-id, the second carrying the retry's
session-id — and no prior content has been edited or removed.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Implementer prompt narrates the natural red-green workflow

`resources/modules/prompts/implementer.md` is edited so that the
"Your task" section (or a sibling section if the planner chooses a
different layout) walks the implementer through the red-green
workflow in execution order:

1. Read the SPEC requirements the task covers (unchanged from
   today's prompt).
2. Read the `<task-scenarios>` body on this task (unchanged).
3. Translate each slice-level scenario into an executable test (or
   a scoped verification command for non-test slices) in the
   project's framework, **before writing implementation code**.
4. Run the failing test / verification command, capture the
   verbatim output, write it into a new
   `## Session <session-id> (attempt N)` block inside
   `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md` under a
   `<red exit="N">` element. (Create the file if it does not exist
   yet; append the block if it does.)
5. Implement the code path to make the test/command pass.
6. Run the test/command again, capture the verbatim output, append
   it under a `<green exit="0">` element inside the same session
   block.
7. Run the project's deterministic hygiene gates (lint, fmt,
   build, full-suite test, dep audit, etc. — the exact set is
   documented in the project's AGENTS.md "Standard hygiene"
   section). Record the gate-by-gate exit codes for the
   `Hygiene checks` table inside the upcoming handoff note.
8. Append the `<implementer-note session="...">` block to the
   `<task>` element in TASKS.md, populating all six fields
   (Completed, Undone, Hygiene checks, Evidence, Discovered
   issues, Procedural compliance). The `Evidence` field
   references the file path created in step 4.
9. Flip the task's `state="..."` attribute to `in-review` to
   signal "awaiting review".

The prompt names this sequence as the **natural** workflow rather
than a strict gate: a retry session that changes no tests can skip
steps 3–6 and record a no-test-delta session block instead. The
prompt notes that the evidence file always exists before the
handoff note references it, removing the "did I write it before
flipping state?" cognitive overhead.

<done-when>
- `resources/modules/prompts/implementer.md` documents the
  red-green workflow as a numbered sequence inside its "Your
  task" section (or a sibling section).
- The numbered sequence names, in order, the steps: read SPEC
  requirements; read task scenarios; write failing test/command;
  capture red into evidence file; implement code; capture green
  into evidence file; run hygiene gates; append handoff note;
  flip state.
- The numbered sequence documents that for a retry session that
  changes no tests, steps 3–6 are replaced by appending a
  no-test-delta session block to the evidence file.
- The numbered sequence documents that the evidence file is
  created on the first session and appended on subsequent
  sessions; prior sessions are never edited or removed.
- The numbered sequence is framework-agnostic — no per-framework
  command examples (e.g. `cargo test foo`, `pnpm test bar`)
  appear in normative prose (the example file may carry one
  worked sample, but the prompt's narrative stays generic).
- The compile-failure-as-red allowance is documented inside the
  numbered sequence (or as an immediately adjacent note), naming
  this case explicitly.
</done-when>

<behavior>
- Given the post-SPEC `resources/modules/prompts/implementer.md`,
  when its "Your task" section is read, then the red-green
  workflow appears as a numbered sequence whose first two
  remaining steps continue to be "read SPEC requirements" and
  "read task scenarios" (preserving the pre-SPEC ordering) and
  whose subsequent steps interleave test writing, red capture,
  implementation, green capture, hygiene gates, handoff note, and
  state flip in the order documented in this REQ.
- Given the same file, when the no-test-delta retry path is
  documented, then the prompt names the substitution: steps 3–6
  collapse into "append a no-test-delta session block to the
  evidence file" while the remaining steps (hygiene gates,
  handoff note, state flip) proceed normally.
- Given the same file, when the compile-failure-as-red allowance
  is read, then it appears either inline within the relevant
  numbered step or as an immediately adjacent note (not buried in
  a far-away section).
- Given the rendered implementer prompt produced by `speccy
  implement SPEC-NNNN/T-NNN` after this SPEC ships, when the
  rendered prompt's "Your task" section is captured, then the
  numbered red-green workflow appears verbatim from the source
  template (the prompt does not strip or reorder these steps at
  render time).
</behavior>

<scenario id="CHK-003">
Given the file `resources/modules/prompts/implementer.md` after
this SPEC's task lands, when its "Your task" section is parsed as
a numbered list, then the resulting steps include, in order, the
following actions (verbatim wording may vary; semantic ordering is
the contract):

1. Read covered SPEC requirements.
2. Read the task's `<task-scenarios>` body.
3. Write the failing test or scoped verification command for the
   slice contract before any implementation code.
4. Capture the failing output into a new `## Session` block under
   a `<red exit="N">` element inside the evidence file.
5. Implement the code path.
6. Capture the passing output into a `<green exit="0">` element
   in the same session block.
7. Run the project's hygiene gates and record their exit codes.
8. Append the `<implementer-note>` block referencing the evidence
   file path.
9. Flip the task's `state="..."` attribute to `in-review`.

Given the same file, when grepped for the literal substring
`no test delta` or substantially equivalent prose, then the
resulting context names a substitution path: steps 3–6 (or
equivalent) collapse into appending a no-test-delta session block
to the evidence file.

Given the same file, when grepped for the literal substring
`compile`, `build error`, or `cannot find symbol` inside its
red-phase guidance, then at least one match exists naming
compile-failure-as-red as legitimate red evidence.

Given the same file, when its red-green workflow narration is
grepped for any of the strings `cargo test`, `pnpm test`,
`pytest`, `jest`, `vitest`, `mocha`, or `rspec` as a normative
example, then zero matches are found inside normative prose. The
sole non-normative reference (if any) sits inside a worked
example or an "e.g." aside, not in a step that instructs the
implementer.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Evidence example ships as a host-agnostic resource and is referenced via progressive disclosure

A new resource file at
`resources/modules/examples/evidence.md` is added to the
workspace. It carries a canonical worked example demonstrating:

- The H1 header and `<evidence task="..." spec="...">` wrapper.
- One session block with `<red>` / `<green>` element blocks
  carrying plausible verbatim runner output for at least one
  framework (the example may pick any single framework — e.g.
  cargo — for its worked output; the example is illustrative, not
  normative).
- One retry session block marked `(attempt 2, no test delta)`
  with a single-sentence summary instead of red/green output.

`resources/modules/prompts/implementer.md` references this file
via progressive disclosure: the prompt inlines a 3-line minimal
sketch of an evidence block (just enough to anchor the shape) and
instructs the implementer to read
`.speccy/examples/evidence.md` via the host Read primitive for the
full shape when first encountering the convention.

`speccy init` ejects `resources/modules/examples/*` from the
embedded `RESOURCES` bundle to `.speccy/examples/*` in the user's
project root. The ejection is host-agnostic: the same files land
regardless of whether `--host claude-code` or `--host codex` is
selected. The example does not land under any
`.claude/skills/.../examples/` or `.codex/agents/.../examples/`
host-specific path.

The committed in-tree `.speccy/examples/evidence.md` is kept
byte-identical to the embedded
`resources/modules/examples/evidence.md` via a new meta-test
parallel to the existing host-pack drift check
(`speccy-cli/tests/init.rs`).

<done-when>
- `resources/modules/examples/evidence.md` exists in the workspace
  and is non-empty.
- The file contains an `<evidence` open tag and matching
  `</evidence>` close tag.
- The file contains at least one `<red exit=` element block and
  one matching `<green exit=` element block.
- The file contains at least one session block marked as a no-test-delta
  retry (literal substring `no test delta` or substantially equivalent
  wording).
- `resources/modules/prompts/implementer.md` references
  `.speccy/examples/evidence.md` via the host Read primitive at
  least once (literal substring `.speccy/examples/evidence.md`
  present in the prompt body).
- `resources/modules/prompts/implementer.md` inlines a minimal
  evidence-shape sketch (≤ 5 lines) so the implementer has anchor
  context before reading the full example file.
- `speccy init --host claude-code` and `speccy init --host codex`
  both produce `.speccy/examples/evidence.md` in the target
  project root with content byte-identical to
  `resources/modules/examples/evidence.md`.
- A committed `.speccy/examples/evidence.md` exists at the
  workspace root with content byte-identical to
  `resources/modules/examples/evidence.md`.
- A meta-test (mirroring the existing host-pack drift check
  pattern) asserts that the in-tree
  `.speccy/examples/evidence.md` matches the embedded
  `RESOURCES`/`modules/examples/evidence.md` byte-for-byte;
  failure surfaces a clear diagnostic naming both paths.
- `speccy init` does not emit
  `.claude/skills/speccy-work/examples/evidence.md` or
  `.codex/agents/speccy-work/examples/evidence.md` (the example is
  not duplicated under host-native trees).
</done-when>

<behavior>
- Given the post-SPEC workspace, when
  `resources/modules/examples/evidence.md` is read, then it
  contains a canonical worked example covering both a red+green
  session and a no-test-delta retry session.
- Given the post-SPEC `resources/modules/prompts/implementer.md`,
  when its workflow-narration section is read, then it references
  `.speccy/examples/evidence.md` and instructs the implementer to
  read it via the host Read primitive on first encounter.
- Given `speccy init --host <host>` for each host in
  `{ClaudeCode, Codex}` invoked against a fresh project directory,
  when the resulting on-disk tree is inspected, then
  `.speccy/examples/evidence.md` exists with content byte-identical
  to the embedded source.
- Given the committed in-tree `.speccy/examples/evidence.md` after
  this SPEC lands, when its bytes are compared to the embedded
  `RESOURCES`/`modules/examples/evidence.md` bytes, then they are
  identical. A drift between them is a build-time test failure.
- Given a fresh project initialized via `speccy init`, when the
  on-disk tree is grepped for `examples/evidence.md` files, then
  exactly one match exists — at `.speccy/examples/evidence.md`. No
  host-native duplicate exists under `.claude/` or `.codex/`.
</behavior>

<scenario id="CHK-004">
Given the workspace after this SPEC's task lands, when
`resources/modules/examples/evidence.md` is read, then its body
contains:

- At least one `<evidence` open tag and matching `</evidence>`
  close tag.
- At least one `<red exit=` element block with verbatim-looking
  runner output (illustrative for one framework).
- At least one `<green exit="0">` element block paired with the
  red block in the same session.
- At least one `## Session` markdown header marked as a
  no-test-delta retry session.

Given the post-SPEC `resources/modules/prompts/implementer.md`,
when grepped for the literal substring
`.speccy/examples/evidence.md`, then at least one match exists
inside a progressive-disclosure reference (e.g. "see
`.speccy/examples/evidence.md` via your Read primitive for the
full shape").

Given the post-SPEC `resources/modules/prompts/implementer.md`,
when its workflow-narration section is parsed for inline evidence
sketches, then at most a small minimal sketch (≤ 5 lines) appears
inline — the full 30-ish-line example is not duplicated into the
prompt body.

Given `speccy init --host claude-code` run in a fresh empty
directory, when the resulting on-disk tree is inspected, then a
file exists at `.speccy/examples/evidence.md` whose content is
byte-identical to the embedded source at
`RESOURCES`/`modules/examples/evidence.md`.

Given `speccy init --host codex` run in a fresh empty directory,
when the resulting on-disk tree is inspected, then a file exists
at `.speccy/examples/evidence.md` byte-identical to the embedded
source, and no file exists at
`.codex/agents/speccy-work/examples/evidence.md` or any other
host-native examples path.

Given the workspace's in-tree `.speccy/examples/evidence.md`
after this SPEC lands, when its bytes are compared to the embedded
`RESOURCES`/`modules/examples/evidence.md` bytes (via the new
drift-check meta-test), then the comparison yields equality and
the test passes.

Given a host-pack drift check run against the post-SPEC workspace,
when both the existing host-pack drift check and the new
examples-pack drift check execute, then both pass and the test
suite exits 0.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: reviewer-tests persona loads the evidence file and blocks on absence or fabrication

`resources/modules/personas/reviewer-tests.md` is edited so that
the persona definition instructs the reviewer to:

1. Locate the `Evidence:` field inside each `<implementer-note>`
   element block on the task.
2. Read the referenced evidence file via the host Read primitive.
3. Treat the absence of the `Evidence:` field, or the absence of
   the referenced file, as a `verdict="blocking"` review. The
   blocking summary names what is missing (no Evidence field /
   evidence file not found at path).
4. Treat fabricated-looking evidence as a `verdict="blocking"`
   review. The persona definition enumerates fabrication patterns
   the reviewer must scrutinize:
   - Output that lacks structural artifacts a real test/build
     runner would emit (no test names, no error messages, no
     stack frames where applicable).
   - Test names inside the evidence file that do not appear in
     the diff under review.
   - Identical or near-identical red and green output (a real
     red→green transition produces materially different output).
   - Suspiciously clean output that omits the usual verbose
     framework headers, summaries, or timing prose.
   - Output that names a command that the rendered
     `Hygiene checks` table also names (the evidence command
     should be a scoped per-test or per-slice invocation, not the
     full-suite hygiene run).
5. Stay framework-agnostic: the persona definition does not name
   per-framework anchor strings (no "cargo: look for `test result:
   FAILED`"). The persona relies on fresh-context judgment of
   what real runner output for the slice's framework would look
   like.

`resources/modules/prompts/reviewer-tests.md` is edited so the
rendered reviewer-tests prompt walks the reviewer through the
Evidence-loading step explicitly: extract the `Evidence:` path
from the `<implementer-note>` element body, read the file via the
host Read primitive, then apply the persona definition's
fabrication patterns to the loaded content.

The other five built-in personas — `reviewer-business`,
`reviewer-security`, `reviewer-style`, `reviewer-architecture`,
`reviewer-docs` — carry no evidence-related instruction. Their
persona files and rendered prompts continue to anchor on
diff + SPEC + `<task-scenarios>` only. This asymmetry is
deliberate: bundling evidence-loading into every persona would
anchor adversarial reviewers on the implementer's claimed test
outcomes, weakening the multi-persona fan-out's adversarial
property.

<done-when>
- `resources/modules/personas/reviewer-tests.md` contains explicit
  instructions naming the four steps: locate Evidence field, read
  evidence file, treat absence as blocking, treat fabrication as
  blocking.
- The same file enumerates the fabrication patterns documented in
  this REQ (at least the five patterns above; more is acceptable).
- The same file does NOT name per-framework anchor strings
  (`test result: FAILED`, ` ✗ `, `FAILED:`, etc.). The persona
  stays framework-agnostic.
- `resources/modules/prompts/reviewer-tests.md` instructs the
  reviewer to load the evidence file via the host Read primitive
  as a step the rendered prompt walks through.
- `resources/modules/personas/reviewer-{business,security,style,architecture,docs}.md`
  carry no `Evidence:` / evidence-loading instructions (the new
  surface is `reviewer-tests`-only).
- `resources/modules/prompts/reviewer-{business,security,style,architecture,docs}.md`
  carry no evidence-loading instructions.
- The rendered review prompts produced by `speccy review
  SPEC-NNNN/T-NNN --persona <P>` for each persona match the
  evidence-loading asymmetry: only the `tests` persona's rendered
  prompt instructs evidence loading.
</done-when>

<behavior>
- Given the post-SPEC `resources/modules/personas/reviewer-tests.md`,
  when its body is read, then the reviewer-tests focus list
  includes explicit instructions to load the evidence file
  referenced by each `<implementer-note>` element on the task
  under review.
- Given the same file, when its blocking-verdict guidance is
  read, then absence of the `Evidence:` field or absence of the
  referenced file is named as a blocking trigger.
- Given the same file, when its fabrication-pattern enumeration
  is read, then it includes at least the five patterns: lack of
  framework artifacts, test names absent from diff, identical
  red/green output, suspiciously clean output, and
  evidence-command matching the hygiene full-suite invocation.
- Given the same file, when grepped for any framework-specific
  anchor string (`test result: FAILED`, ` ✗ `, `FAILED:`, `error[E`,
  `passing tests`, etc.), then zero matches are found in
  normative guidance.
- Given the post-SPEC `resources/modules/prompts/reviewer-tests.md`,
  when its rendered output for a task with one or more
  `<implementer-note>` elements is captured, then the captured
  prompt instructs the reviewer to extract the `Evidence:` path
  from the `<implementer-note>` body and read the file via the
  host Read primitive.
- Given the rendered `speccy review SPEC-NNNN/T-NNN --persona
  business` prompt after this SPEC ships, when captured, then it
  contains no evidence-loading instruction (the asymmetry holds).
  The same is true for `security`, `style`, `architecture`, and
  `docs` personas.
</behavior>

<scenario id="CHK-005">
Given the file `resources/modules/personas/reviewer-tests.md`
after this SPEC's task lands, when grepped for the literal
substring `Evidence:` (the field name as it appears in the
`<implementer-note>` body), then at least one match exists inside
normative guidance instructing the reviewer to locate and read
the evidence file.

Given the same file, when grepped for the literal substring
`blocking` in proximity to `Evidence` or `evidence file`, then at
least one match exists inside guidance naming evidence absence as
a blocking trigger.

Given the same file, when its fabrication-pattern guidance is
read, then it enumerates at least the five patterns named in
REQ-005's done-when list (lack of framework artifacts, test names
absent from diff, identical red/green output, suspiciously clean
output, evidence-command matching hygiene full-suite invocation).

Given the same file, when grepped for the literal substrings
`test result: FAILED`, ` ✗ `, `FAILED:`, `error[E`, `cargo test`,
`pnpm test`, `pytest`, `jest`, or `vitest` inside normative
guidance, then zero matches are found. (Matches inside a worked
example block or a `<!-- ... -->` aside are out of scope; the
contract is that normative instructional prose stays
framework-agnostic.)

Given the file `resources/modules/prompts/reviewer-tests.md`
after this SPEC's task lands, when its rendered output captures
the substituted instructions for the reviewer, then the rendered
prompt contains an instruction to extract the `Evidence:` path
from each `<implementer-note>` body and read the file via the
host Read primitive.

Given each of the files
`resources/modules/personas/reviewer-{business,security,style,architecture,docs}.md`
and the parallel files
`resources/modules/prompts/reviewer-{business,security,style,architecture,docs}.md`,
when grepped for the literal substring `Evidence:` or
`evidence file`, then zero matches are found inside normative
guidance. The other five personas carry no evidence-related
instructions.

Given a workspace where `speccy review SPEC-NNNN/T-NNN --persona
<P>` is invoked once per persona in
`speccy_core::personas::ALL` after this SPEC ships, when the six
rendered prompts are captured, then exactly one — the `tests`
persona's prompt — contains the evidence-loading instruction.
The other five rendered prompts contain no such instruction.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: BACKLOG.md gains F-9 entry tracking the follow-up migration

`.speccy/BACKLOG.md` is edited to add an F-9 entry under Tier 2
(or Tier 1 if the planner judges higher priority — Tier 2 is the
default) tracking the follow-up to migrate other inline examples
across personas and prompts to the progressive-disclosure pattern
this SPEC establishes.

The F-9 entry follows the existing BACKLOG.md entry format
(what / why / where / cost) and references SPEC-0031 as the SPEC
that established the pattern. The entry names the heuristic for
when to eject vs inline (example > ~8 lines or used by >1 prompt
→ eject) and the risk (over-ejection of small shape sketches).

The existing F-3 entry under Tier 1 stays unchanged at
implementation time. F-3's closure annotation
(`F-3: Red-green paper trail in task closure — **closed by
SPEC-0031 (YYYY-MM-DD)**` with a closure prose paragraph) is
written at ship time by the `speccy-ship` skill, mirroring the
F-7 → SPEC-0030 closure pattern. This SPEC's implementation tasks
do not pre-emptively close F-3 in BACKLOG.md; that edit lands
with the SPEC's ship commit.

<done-when>
- `.speccy/BACKLOG.md` contains an F-9 entry whose header reads
  `F-9: Migrate inline examples in personas and prompts to
  progressive disclosure` (or substantially equivalent wording
  naming the migration and "progressive disclosure").
- The F-9 entry follows the existing four-field format used by
  F-3, F-4, F-5, F-6, F-8: what is being asked, why, where it
  lives, and cost (or risk, when cost is dwarfed by judgment).
- The F-9 entry references SPEC-0031 explicitly as the pattern's
  source ("Pattern established by SPEC-0031 (F-3 red-green
  paper trail)" or substantially equivalent).
- The F-9 entry names the eject-vs-inline heuristic (example > ~8
  lines or used by >1 prompt → eject) and the over-ejection risk.
- The existing F-3 entry remains under Tier 1 (open) after this
  SPEC's implementation tasks land. The F-7 → SPEC-0030 closure
  annotation pattern is the model for what `speccy-ship` will
  apply to F-3 → SPEC-0031 at ship time.
</done-when>

<behavior>
- Given `.speccy/BACKLOG.md` after this SPEC's task lands, when
  grepped for `^F-9:`, then exactly one match exists.
- Given the same file, when the F-9 entry is read, then it names
  the migration target (other inline examples across personas
  and prompts), the motivation (progressive disclosure reduces
  per-invocation token cost), the location (the 7+7 persona and
  prompt files), and the heuristic / risk.
- Given the same file, when the F-9 entry is read, then it
  references SPEC-0031 as the pattern's origin.
- Given the same file, when the F-3 entry is read after this
  SPEC's implementation tasks land but before ship, then the
  entry still appears under Tier 1 without a closure annotation;
  the closure is written by `speccy-ship` at the time the SPEC
  status transitions to `implemented`.
</behavior>

<scenario id="CHK-006">
Given the file `.speccy/BACKLOG.md` after this SPEC's task lands,
when grepped for the regex `^F-9:`, then exactly one match
exists.

Given the same file, when the F-9 entry body is read (the
sequence of bullet lines following the `F-9:` header through to
the next `F-NN:` or `R-NN:` header), then it contains:

- A what bullet describing inline-example migration to
  progressive disclosure.
- A why bullet citing per-invocation token cost and citing
  SPEC-0031 as the pattern's origin.
- A where bullet naming `resources/modules/personas/*.md` and
  `resources/modules/prompts/*.md`.
- A heuristic / risk bullet naming the eject-vs-inline threshold
  (≥ ~8 lines or ≥ 2 consuming prompts → eject) and the
  over-ejection risk.

Given the same file, when grepped for `^F-3:` after this SPEC's
implementation tasks land but before this SPEC's ship commit,
then exactly one match exists for the existing F-3 entry under
Tier 1, and the entry does NOT carry a `**closed by SPEC-0031**`
annotation — that closure is `speccy-ship`'s responsibility at
the ship-commit boundary.

Given the same file's Tier-2 section after this SPEC ships, when
inspected, then the F-9 entry sits under Tier 2 (alongside F-8
and F-6), preserving the existing tier-grouping convention.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Standard hygiene gates exit clean against the post-SPEC workspace

All four standard-hygiene gates documented in
`AGENTS.md` § "Standard hygiene" exit 0 against the post-SPEC
workspace:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

The new tests this SPEC adds (the host-agnostic ejection drift
check; assertion tests for the new prompt-module content) pass
under `cargo test --workspace`. The new ejection wiring in
`speccy-cli/src/render.rs` and `speccy-cli/src/init.rs` compiles
cleanly under the strict workspace clippy lint set documented in
`.claude/rules/rust/rust-linting.md`.

<done-when>
- `cargo test --workspace` exits 0 against the post-SPEC
  workspace. The new tests added under
  `speccy-cli/tests/init.rs` (or a sibling integration-test
  file) pass: at least one test asserts the host-agnostic
  ejection of `.speccy/examples/evidence.md`, and at least one
  test asserts the in-tree drift check between
  `.speccy/examples/evidence.md` and the embedded resource.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0 modulo the carried-forward
  `clippy::result_large_err` warning against
  `speccy_core::error::ParseError` (which is unrelated to this
  SPEC and tracked under F-7 in BACKLOG.md — already closed by
  SPEC-0030 but the underlying carry note may still appear in
  REPORT.md context).
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0 (the SPEC adds no new
  dependencies).
</done-when>

<behavior>
- Given the post-SPEC workspace, when the four standard-hygiene
  gate commands are run in sequence, then each exits 0.
- Given the new ejection wiring in `speccy-cli/src/init.rs` /
  `speccy-cli/src/render.rs`, when compiled under the workspace's
  strict clippy lint set, then no new warnings are emitted (no
  `unwrap_used`, `expect_used`, `panic`, `unreachable`,
  `indexing_slicing`, etc.).
- Given the new prompt-module edits (`implementer.md`,
  `reviewer-tests.md` prompt, `reviewer-tests.md` persona) and
  the new resource file (`evidence.md`), when the prompt-module
  loader runs at render time, then it resolves all references
  without errors — no missing-include diagnostics, no broken
  Jinja substitutions.
</behavior>

<scenario id="CHK-007">
Given the post-SPEC workspace, when `cargo test --workspace` is
run, then the exit code is 0 and the test summary includes at
least one new test name attributable to this SPEC's
host-agnostic-ejection coverage (e.g.
`init::ejects_speccy_examples_for_claude_code`,
`init::ejects_speccy_examples_for_codex`,
`init::in_tree_examples_match_embedded`).

Given the post-SPEC workspace, when `cargo clippy --workspace
--all-targets --all-features -- -D warnings` is run, then the
exit code is 0. Any surviving warnings are explicitly
carried-forward from prior SPECs (e.g. the closed-but-noisy
`result_large_err` carry from SPEC-0026 / SPEC-0030) and named
in REPORT.md.

Given the post-SPEC workspace, when `cargo +nightly fmt --all
--check` is run, then the exit code is 0.

Given the post-SPEC workspace, when `cargo deny check` is run,
then the exit code is 0. The SPEC adds no new dependencies
to either workspace `Cargo.toml` or any member-crate
`Cargo.toml`.

Given the new prompt-module edits, when the prompt-module loader
runs at render time for `speccy implement` and `speccy review`
against any in-tree task, then it resolves every `{% include %}`
reference without missing-include diagnostics.
</scenario>

</requirement>

## Design

### Approach

The implementation lands in roughly six layers. The ordering is
chosen so that no in-flight intermediate state leaves the
workspace failing hygiene gates and so that downstream consumers
have their dependencies in place when their tasks run:

1. **Example resource file**
   (`resources/modules/examples/evidence.md`). Add the new
   directory under `resources/modules/`, write the canonical
   worked example carrying one red+green session and one no-test-delta
   retry session. Confirm `RESOURCES`/`modules/examples/evidence.md`
   resolves via the existing `include_dir!` snapshot mechanism in
   `speccy-cli/src/embedded.rs`. No code change to `embedded.rs`
   is required — the macro auto-snapshots all of `resources/`.

2. **Host-agnostic ejection in `speccy-cli`**
   (`speccy-cli/src/render.rs` + `speccy-cli/src/init.rs`). Add a
   new ejection path emitting `.speccy/examples/*` regardless of
   host choice. The new function (provisional name
   `render_speccy_examples_pack() -> Vec<RenderedFile>`) walks
   `RESOURCES`/`modules/examples/*` and produces
   `.speccy/examples/<filename>` `RenderedFile` entries. `init::run`
   appends the resulting plan items after the host-pack plan
   items so that init still produces the same ordered, dedupable
   on-disk plan it does today.

3. **In-tree drift check**
   (`speccy-cli/tests/init.rs`). Add a meta-test mirroring the
   existing host-pack drift assertion that confirms the committed
   `.speccy/examples/evidence.md` matches the embedded
   `RESOURCES`/`modules/examples/evidence.md` byte-for-byte. The
   test discovers the embedded source via the same `RESOURCES`
   accessor `render.rs` uses.

4. **Implementer prompt edits**
   (`resources/modules/prompts/implementer.md`). Rename `Commands
   run` + `Exit codes` into `Hygiene checks` (table form), add the
   `Evidence` field, narrate the red→green workflow in execution
   order, reference `.speccy/examples/evidence.md` via progressive
   disclosure, and acknowledge compile-failure-as-red. The
   ordering matters: this layer depends on layer 1 (the example
   exists) and layer 2 (the ejection lands the example at the
   referenced path) being in place — but only at the level of the
   referenced path's stability, not at runtime. So layer 4 can
   technically land before layer 2 as long as the path is fixed
   ahead of time.

5. **Reviewer-tests prompt + persona edits**
   (`resources/modules/prompts/reviewer-tests.md` and
   `resources/modules/personas/reviewer-tests.md`). Add
   evidence-loading instructions to the rendered prompt and the
   fabrication-pattern enumeration to the persona. Confirm the
   other five persona files and prompt files are NOT touched —
   the asymmetry is the load-bearing property.

6. **BACKLOG.md hygiene**
   (`.speccy/BACKLOG.md`). Add the F-9 entry under Tier 2. Do
   not pre-close F-3 — the closure annotation lands at
   ship time via the `speccy-ship` skill.

After all layers land:

7. **Commit the in-tree example**
   (`.speccy/examples/evidence.md`). Run the new ejection path
   against the workspace itself (or copy the embedded source by
   hand once-off) and commit the result. The drift-check meta-test
   then enforces ongoing parity.

The canonical task order is therefore:

```text
1 (resource) → 2 (ejection wiring) → 3 (drift test) → 7 (commit in-tree)
                                                         ↓
                              4 (implementer prompt)  5 (reviewer-tests)  6 (BACKLOG)
```

Layers 4, 5, 6 are independent of each other and can run in
parallel after layers 1-3 + 7 have landed (or earlier — they only
need the path convention agreed, which is fixed by the SPEC).

### Decisions

<decision id="DEC-001" status="accepted">
### DEC-001: External evidence file over inline runner output

The evidence paper trail lives in a per-task file at
`.speccy/specs/NNNN-slug/evidence/T-NNN.md` rather than inlined
into the `<implementer-note>` body. The trade-off is real:
inlining keeps everything in one place (TASKS.md) and avoids
adding a new filesystem convention; externalizing introduces a
new directory pattern and requires a path-reference convention
inside the handoff note.

We chose external because TASKS.md is read repeatedly across the
workflow:

- `speccy implement` renders prior `<task>` entries (including
  prior `<implementer-note>` bodies) into the implementer prompt.
- `speccy review` renders the same task entry into each persona's
  rendered prompt (with `<implementer-note>` redacted per
  SPEC-0029, but other body items preserved).
- `speccy report` does not inline TASKS.md but its rendered
  prompt instructs the report agent to read TASKS.md via the host
  Read primitive at REPORT.md authoring time.
- `speccy status` and `speccy next` parse TASKS.md to compute
  state aggregates.
- `speccy verify` walks TASKS.md for proof-shape diagnostics.

Inlining red+green verbatim output (potentially hundreds of lines
per session, doubled or tripled for retry-heavy tasks) would
inflate every one of those reads. Externalizing pays the cost
once at evidence-write time and once per reviewer-tests load, with
no other consumer paying anything.

The external-file convention also gives the reviewer-tests
persona a clean reading boundary: the persona either reads the
file (Evidence field present, file exists) or does not (absence is
a blocking signal). No partial-state ambiguity.
</decision>

<decision id="DEC-002" status="accepted">
### DEC-002: Append-only single file per task over file-per-session

The evidence file is a single
`.speccy/specs/NNNN-slug/evidence/T-NNN.md` per task, append-only
across retry sessions, rather than a separate
`T-NNN-{session-id}.md` per session.

The trade-off:

- File-per-session keeps each session's content small and
  separately diff-able, and an obviously-deleted file is loud.
- File-per-task consolidates retries in one place; reading retry
  history is one open-file action; the file count stays linear in
  task count rather than session count.

We chose file-per-task because retries are rare on Speccy work in
practice (most tasks ship on first attempt), so the per-session
fragmentation would not pay for itself. When retries do happen,
having the full history in one file makes the reviewer's job of
auditing whether a retry session's red→green proof actually
addresses the prior blocking review easier than juggling N files.

Append-only is the discipline: no session block is ever edited or
removed retroactively. The reviewer can rely on the file's
sequence representing the actual implementer history, not a
post-hoc reconstruction.
</decision>

<decision id="DEC-003" status="accepted">
### DEC-003: Tests-only persona loads evidence

Only the `reviewer-tests` persona is instructed to load the
evidence file. The other five built-in personas
(`reviewer-business`, `reviewer-security`, `reviewer-style`,
`reviewer-architecture`, `reviewer-docs`) carry no
evidence-related instructions.

We considered loading evidence into every persona's prompt so
multiple sets of eyes catch fabrication. We rejected it for the
same reason SPEC-0029 redacts implementer notes from reviewer
prompts: anchoring adversarial reviewers on implementer-produced
artifacts (even artifacts as concrete as captured runner output)
weakens the multi-persona fan-out's value. The business reviewer's
job is to judge intent against SPEC; the security reviewer's is
to ask "what if an attacker"; the style reviewer's is to judge
diff form. None of those questions improve when the persona reads
the implementer's claimed test outcomes first.

Parallel-cost angle: five personas × evidence file = four wasted
context reads per review run, with no commensurate signal gain.
Single-persona ownership is faster and cleaner.

If a future SPEC surfaces a need for cross-persona evidence
visibility (e.g. an attack pattern where business reviewers
should challenge fabrication independently), the right move is to
extend the persona definitions one at a time as evidence
accumulates — not to dilute the v1 asymmetry pre-emptively.
</decision>

<decision id="DEC-004" status="accepted">
### DEC-004: Host-agnostic deployment under `.speccy/examples/`

The example file deploys to `.speccy/examples/evidence.md`,
host-agnostic, regardless of whether `speccy init` ran for
Claude Code or Codex. The example does not duplicate under
`.claude/skills/.../examples/` or `.codex/agents/.../examples/`.

We considered host-native deployment (one copy per host tree).
Rejected because:

- Duplication doubles the byte cost and creates drift risk
  (if `speccy init` updates one tree and not the other, evidence
  shape recommendations could diverge).
- The host-native principle applies to skills the harness
  auto-loads. Claude Code's `.claude/skills/` and Codex's
  `.codex/agents/` exist because the host harness scans those
  paths at session start. The evidence example is not a skill —
  it is reference content read on-demand by an LLM via the host
  Read primitive. There is no harness-load reason to colocate
  it with a particular host's skill files.
- The implementer prompt is rendered by `speccy implement` (CLI),
  not by a host skill, and the rendered prompt names the path
  literally. A host-specific path would require the CLI to
  detect the host at render time, adding complexity for zero
  benefit.

The `.speccy/` directory is already the canonical home for
project-level Speccy reference content (`docs/ARCHITECTURE.md`,
`.speccy/BACKLOG.md`, `.speccy/specs/`). Adding `.speccy/examples/`
is consistent with that pattern. The earlier retirement of
`.speccy/skills/` was about skill content (which migrated to
host-native trees); reference content stays under `.speccy/`.
</decision>

<decision id="DEC-005" status="accepted">
### DEC-005: Framework-agnostic prompt and persona

Neither the implementer prompt nor the reviewer-tests persona
names per-framework anchor strings (`cargo test result: FAILED`,
`pnpm ✗`, `pytest FAILED:`, etc.). The implementer captures
whatever their toolchain emits; the reviewer judges plausibility
from fresh context.

We considered shipping a framework-anchor checklist
(e.g. "for cargo: confirm `test result:` appears in red phase";
"for pnpm: confirm error message in red phase"). Rejected because:

- Coverage is open-ended. New frameworks land every year; the
  checklist would constantly need updates and would become a
  documentation maintenance tax.
- Anchoring on a specific string biases the reviewer. A real
  cargo test failure might use slightly different wording across
  versions; reviewer pattern-matching becomes a brittle gate.
- Reviewer judgment from fresh context is what speccy is built
  on. The reviewer-tests persona is asked to assess plausibility:
  does this output look like real runner output? Does the test
  name appear in the diff? Are red and green materially
  different? These are model-judgment calls.

The persona definition enumerates the fabrication *patterns*
(structural absences, name mismatches, identity drift) without
prescribing the framework-specific surface. This is a calibrated
bar (the persona knows what to look for in spirit) without
brittle pattern-matching (it is not handed a per-framework regex
checklist).
</decision>

<decision id="DEC-006" status="accepted">
### DEC-006: Compile-failure-as-red allowance

The implementer prompt explicitly accepts compile-failure output
as a legitimate red phase. The classic TDD model is "write a
failing test, run it, see the failure, write code, see it pass".
In compiled languages (Rust, Go, TypeScript with strict type
checks, Java, Swift, etc.) writing a test that references a yet-unimplemented
function `foo()` typically produces a compile error
(`cannot find function foo in this scope`) rather than a runtime
test failure. The compile error is structurally the same signal:
"the code under test does not exist; this test cannot pass
without an implementation".

We considered requiring runtime test failure as the only valid
red phase. Rejected because:

- It forces the implementer to scaffold a stub
  implementation before writing the test (e.g. `fn foo() {
  unimplemented!() }`) just to get past the compile gate. That
  scaffolding adds noise to the diff and is essentially make-work.
- The reviewer can still distinguish "compile-error red" from
  "no red at all" by reading the evidence file content; the
  fabrication patterns (missing framework artifacts, test names
  absent from diff) still apply.

The prompt names this allowance explicitly so the implementer
does not waste time fabricating runtime failures.
</decision>

<decision id="DEC-007" status="accepted">
### DEC-007: Single-Hygiene-checks table over parallel fields

The implementer handoff template collapses `Commands run` /
`Exit codes` (two parallel list fields with positional pairing)
into a single `Hygiene checks` markdown table with `Command` and
`Status` columns. Each row carries one command and its
`pass (exit 0)` or `fail (exit N)` status.

Positional pairing is error-prone — list lengths get out of sync
under retry edits, and the binding between a command and its exit
code becomes implicit. A table makes the binding structural:
column 1 names the command, column 2 names the outcome on the
same row.

We considered keeping the parallel fields and surfacing the
mis-pairing risk as a reviewer-style concern. Rejected because
the table is cheap and obviously better; deferring it to
reviewer-style would create a recurring noise source for zero
benefit.

The rename also clarifies the semantic split: `Hygiene checks`
captures deterministic project gates (lint, fmt, build, full-suite
test); `Evidence` captures the per-test red→green adversarial
paper trail. Two distinct kinds of proof, two distinct fields.
</decision>

<decision id="DEC-008" status="accepted">
### DEC-008: No CLI surface change beyond a host-agnostic ejection path

This SPEC adds one new ejection path inside `speccy init`'s
rendering pipeline (`render_speccy_examples_pack` or equivalent)
but no new commands, no new flags, and no new lint codes. The
F-3 backlog entry framed this as "live in the rendered prompt
templates inside the Rust CLI, not in new commands or new
skills"; the host-agnostic ejection is a small expansion of that
framing.

We considered shipping `.speccy/examples/evidence.md` as a
hand-committed file with no init-time ejection (Speccy itself
already has the file; other projects' authors would copy it by
hand). Rejected because:

- It violates the "speccy init works in any project state"
  principle: a fresh project should be self-bootstrapping from
  `speccy init`, not require manual file-copying steps.
- It introduces drift risk: hand-copied files diverge over time;
  init-time ejection guarantees parity with the canonical source.

The ejection is host-agnostic specifically because the example
itself is host-agnostic content. Adding a parallel
`render_speccy_examples_pack` next to the existing
`render_host_pack` is a small surface expansion that earns its
existence by removing a manual bootstrapping step.
</decision>

<decision id="DEC-009" status="accepted">
### DEC-009: Evidence excluded from spec / tasks hashes

The evidence file does not contribute to `<spec-hash>` or
`<tasks-hash>` computation. SPEC-0024 introduced
meaningful-hash semantics where SPEC.md and TASKS.md content
shape the hash and trigger staleness alerts when they drift.
Evidence files are a downstream byproduct of implementation,
not part of the spec/tasks contract.

Bringing evidence into the hash would force every retry session
(which appends to the evidence file) to also re-trigger
`HashDrift` staleness alerts. That is the wrong signal:
appending evidence is the workflow working, not the workflow
drifting.

The reviewer-tests persona is the load-bearing check on evidence
quality. The hash gate is a different concern (intent integrity
between SPEC author and downstream consumers) and should not
absorb evidence.
</decision>

<decision id="DEC-010" status="accepted">
### DEC-010: REPORT.md stays narrative; no evidence fold-in

REPORT.md is the load-bearing artifact for the human merge gate
(the human reads the report to decide whether to merge the PR).
Compact, narrative summaries serve that audience; multi-page
runner-output dumps drown it.

We considered folding key evidence snippets into REPORT.md
(e.g. one red→green snippet per task as proof points). Rejected
because:

- REPORT.md authors (run by `speccy report`) are not tasked with
  judging evidence quality — that's the reviewer-tests persona's
  job, already complete by report time.
- The PR diff and the in-tree evidence files are both reachable
  by a reviewer who wants to spelunk; REPORT.md does not need to
  duplicate them.
- Compact narrative survives the human attention budget;
  evidence dumps do not.

`speccy report` continues to instruct the report agent to read
TASKS.md (via the host Read primitive) and summarize what was
done, what was discovered, and what skill files were touched.
Evidence files are not referenced in the report's authoring
prompt.
</decision>

### Interfaces

- `speccy-cli/src/render.rs`:
  - New `render_speccy_examples_pack() -> Result<Vec<RenderedFile>, RenderError>`
    (or equivalent name; planner choice). Walks
    `RESOURCES`/`modules/examples/*` and produces
    `.speccy/examples/<filename>` `RenderedFile` entries.
  - The new function reuses the existing `RenderedFile` and
    `RenderError` types — no new error variants required (the
    surface is a simple bundled-resource walk; failures map to
    existing `BundleMissing` shape).

- `speccy-cli/src/init.rs`:
  - `build_plan` (or its appender chain) appends the
    examples-pack plan items after the host-pack plan items.
  - `append_host_pack_items` is unchanged in semantics; a
    parallel `append_speccy_examples_items` is added.

- `resources/modules/examples/evidence.md`: new file.

- `resources/modules/prompts/implementer.md`: edited — `Commands
  run` / `Exit codes` retired; `Hygiene checks` + `Evidence`
  introduced; workflow narration added; reference to
  `.speccy/examples/evidence.md` added; compile-as-red allowance
  added.

- `resources/modules/prompts/reviewer-tests.md`: edited —
  Evidence-loading instruction added to the rendered prompt body.

- `resources/modules/personas/reviewer-tests.md`: edited —
  Evidence-loading focus item added; fabrication patterns
  enumerated; explicit blocking-on-absence guidance.

- `.speccy/examples/evidence.md`: new committed file
  (kept in sync with the embedded source via a new meta-test).

- `.speccy/BACKLOG.md`: edited — F-9 entry added under Tier 2.

### Data changes

- TASKS.md `<implementer-note>` body format: the markdown payload
  format inside the element evolves from six sub-bullets named
  `Completed` / `Undone` / `Commands run` / `Exit codes` /
  `Discovered issues` / `Procedural compliance` to six fields
  named `Completed` / `Undone` / `Hygiene checks` (table form)
  / `Evidence` / `Discovered issues` / `Procedural compliance`.
  This is a writer-side prompt change, not a parser change: per
  SPEC-0029 DEC-004 the `<implementer-note>` body is unstructured
  markdown payload from the parser's perspective.

- New on-disk artifact: per-task evidence files at
  `.speccy/specs/NNNN-slug/evidence/T-NNN.md`. These are workflow
  byproducts, committed to git alongside the SPEC's content but
  not part of the spec / tasks hash (per DEC-009).

- New on-disk artifact: `.speccy/examples/evidence.md` (committed
  in-tree, host-agnostic ejection target for `speccy init`).

- No `Cargo.toml` changes. No new dependencies.

### Migration / rollback

- **Forward**: the new fields apply to `<implementer-note>`
  bodies written after this SPEC ships. Existing in-tree TASKS.md
  files carrying the legacy `Commands run` / `Exit codes`
  sub-bullets stay verbatim (the parser does not care; the writer
  prompt is the discipline). The next implementer to flip a task
  to `state="in-review"` writes the new shape; no retroactive
  edit is needed.

- **Rollback**: revert this SPEC's commits. The evidence files
  already created up to rollback time remain on disk as
  orphaned but-harmless artifacts (the reviewer-tests persona,
  if reverted, no longer references them; nothing else does).
  Subsequent implementer-notes go back to the legacy `Commands
  run` / `Exit codes` shape.

## Open Questions

- [x] Where do the captured red/green outputs live — inline in
      the handoff note, or in an external file?
      **Resolved during brainstorm and via DEC-001**: external
      evidence file per task at
      `.speccy/specs/NNNN-slug/evidence/T-NNN.md`.
- [x] One file per task append-only, or one file per session?
      **Resolved during brainstorm and via DEC-002**: one file
      per task, append-only across sessions.
- [x] Which personas read the evidence file?
      **Resolved during brainstorm and via DEC-003**: `tests`
      only; the other five personas do not load evidence.
- [x] Host-native deployment (per-host tree) vs host-agnostic
      (single `.speccy/examples/` location)?
      **Resolved during brainstorm and via DEC-004**: host-agnostic.
- [x] Should the implementer prompt and reviewer-tests persona
      include per-framework anchor checklists?
      **Resolved during brainstorm and via DEC-005**: no, both
      stay framework-agnostic.
- [x] Should the implementer prompt accept compile failure as red?
      **Resolved during brainstorm and via DEC-006**: yes,
      explicitly.
- [x] Collapse `Commands run` + `Exit codes` into a single
      `Hygiene checks` table?
      **Resolved during brainstorm and via DEC-007**: yes,
      table form with `Command | Status` columns.
- [x] Does the example file ship via `speccy init` ejection or by
      hand-copying?
      **Resolved during brainstorm and via DEC-008**: init-time
      ejection via a new host-agnostic resource pack.
- [x] Are evidence files included in spec / tasks hashes?
      **Resolved during brainstorm and via DEC-009**: excluded.
- [x] Does REPORT.md fold evidence content in?
      **Resolved during brainstorm and via DEC-010**: no,
      REPORT.md stays narrative.
- [x] Should the SPEC also migrate existing inline examples in
      personas/prompts to progressive disclosure?
      **Resolved during brainstorm**: no, tracked separately as
      F-9 (REQ-006).

## Assumptions

<assumptions>
- Every task's `<task-scenarios>` block can be encoded as a
  runnable command — even if that command is `grep`,
  `test -f`, or `cargo build` for non-test slices. No
  exemption mechanism is needed.
- Each implementer session has a distinct `session="..."`
  attribute value usable as the `## Session <session-id>`
  anchor inside the evidence file (today: `<implementer-note>`
  parsing already requires a non-empty `session` attribute per
  SPEC-0029 DEC-004).
- The `evidence/` directory under each spec folder is committed
  to git; no `.gitignore` entry is needed. Evidence files
  participate in the same PR review as the SPEC's other
  artifacts.
- Markdown with embedded XML tags (`<evidence>`, `<session>`,
  `<red>`, `<green>`) is a writer/reader convention that LLMs
  parse fluently and humans read scanning-headers-first. The
  Speccy CLI does not parse these tags.
- One evidence file per task, append-only across retries, is
  small enough on average (most tasks ship on first attempt)
  that re-reading it on retry costs less than carrying inline
  runner output through every persona's context window.
- The reviewer-tests model has enough fresh-context judgment to
  spot fabrication patterns without a framework-specific
  checklist. (Today: reviewer-tests already judges adversarial
  test quality from diff + SPEC alone, which is a harder call
  than spotting fabricated runner output.)
- `speccy init` is the right vehicle to eject
  `resources/modules/examples/*` to `.speccy/examples/*` in the
  user's project. The new ejection path adds a parallel layer
  next to the existing host-pack ejection without changing the
  command's surface (no new flags, no new commands).
- Adding `.speccy/examples/` as a top-level directory under
  `.speccy/` does not contradict the retirement of
  `.speccy/skills/`. The retired path housed skill content
  (which moved to host-native trees per SPEC-0027); reference
  content under `.speccy/` is the correct shape and is
  consistent with `docs/ARCHITECTURE.md`,
  `.speccy/BACKLOG.md`, and `.speccy/specs/`.
- The carried-forward `clippy::result_large_err` warning that
  SPEC-0030 addressed continues to apply only at the
  `ParseError` boundary. SPEC-0031 adds no new
  `ParseError` variants and does not interact with that
  carry.
- The host-pack drift-check pattern already established in
  `speccy-cli/tests/init.rs` is extensible to a parallel
  examples-pack drift check via a sibling assertion. No new test
  infrastructure is required.
</assumptions>

## Notes

### Rejected alternative framings

These framings were considered during brainstorm and rejected
explicitly; they are recorded here so future readers do not need
to re-litigate them:

- **Inline verbatim red/green output in implementer-note.**
  Rejected via DEC-001: TASKS.md is read repeatedly by every
  persona's rendered prompt, by `speccy report` (instructed to
  Read TASKS.md), by `speccy status` / `speccy next`, and by
  `speccy verify`. Inlining session-level runner output (often
  hundreds of lines, multiplied per retry) would inflate every
  one of those reads. External file pays the cost once at
  write time and once per reviewer-tests load.
- **File per session (T-NNN-{session-id}.md).** Rejected via
  DEC-002: retries are rare; per-session fragmentation does not
  pay for itself. Single append-only file consolidates retry
  history in one place.
- **All personas read evidence.** Rejected via DEC-003: anchoring
  adversarial reviewers on implementer-produced artifacts
  weakens the multi-persona fan-out, mirroring SPEC-0029's
  redaction logic at a different boundary.
- **Per-framework anchor checklist in reviewer-tests.** Rejected
  via DEC-005: framework coverage is open-ended; anchoring on
  specific strings biases the reviewer; reviewer judgment from
  fresh context is the load-bearing property.
- **Host-native deployment of the example file (under
  `.claude/skills/.../examples/` and
  `.codex/agents/.../examples/`).** Rejected via DEC-004: the
  host-native principle applies to harness-loaded skills, not
  to LLM-read reference content; duplication adds drift risk
  for zero benefit.
- **Inline full evidence example in the implementer prompt body.**
  Rejected: 30-line bloat × every implementer invocation;
  progressive disclosure via `.speccy/examples/evidence.md`
  costs each implementer one Read on first contact and zero on
  subsequent invocations.
- **Keep `Commands run` + `Exit codes` as parallel fields.**
  Rejected via DEC-007: positional pairing is error-prone; the
  table form is structurally clearer.
- **Reviewer-tests-only (no implementer-side requirement).**
  Rejected: the reviewer cannot reliably reconstruct red-state
  from the diff alone post-hoc; the discipline must originate at
  implementation time when the test is being written.
- **CLI-side enforcement via `speccy verify` (verify fails if any
  completed task lacks an evidence file).** Deferred (not
  outright rejected): F-3 is scoped to prompt templates per the
  backlog framing. If reviewer-tests judgment proves insufficient
  to catch absences in practice, a follow-up SPEC may extend
  `speccy verify` with an evidence-shape lint. For v1, the
  reviewer-tests persona's blocking-on-absence verdict carries
  the load.

### Investigation findings carried from brainstorm

The brainstorm phase verified the following by code-read; these
notes are preserved here as durable context for the
implementation:

- `resources/modules/prompts/implementer.md` is 111 lines today.
  The handoff template is appended via the
  `<implementer-note session="...">` element block per
  SPEC-0029. The six sub-bullets (`Completed`, `Undone`,
  `Commands run`, `Exit codes`, `Discovered issues`,
  `Procedural compliance`) are positional markdown bullets
  inside the element body.
- `resources/modules/personas/reviewer-tests.md` is 76 lines
  today. It already documents fabrication-adjacent concerns
  (mocks-not-touching-real-code, snapshots-baking-in-bugs,
  empty-test-bodies) but does not name red-state evidence
  specifically.
- `speccy-cli/src/embedded.rs` snapshots `resources/` via
  `include_dir!` (line 34). The macro automatically picks up
  `resources/modules/examples/evidence.md` once added; no
  embedded-bundle code change is required.
- `speccy-cli/src/render.rs::render_host_pack` walks
  `agents/.<install_root>/` only. A parallel
  `render_speccy_examples_pack` (or equivalent) is the natural
  shape for the new host-agnostic ejection.
- `speccy-cli/tests/init.rs` already implements a host-pack
  drift check pattern (line 38 references the workspace-level
  invariant). The new examples-pack drift check mirrors the
  same shape.
- Today's `<implementer-note>` body is markdown payload from the
  parser's perspective (SPEC-0029 DEC-004). Field renaming
  inside the body is purely a writer-side prompt change; no
  parser whitelist edit, no Rust schema change.
- SPEC-0024 introduced meaningful-hash semantics; the
  `spec_hash_at_generation` value in TASKS.md frontmatter
  hashes SPEC.md content, not TASKS.md content, so the
  `<implementer-note>` body format change is hash-neutral. The
  evidence files are not part of either hash per DEC-009.

## Changelog

<changelog>
| Date       | Reason                                                                                                                                                                                                                                                                  | Author     |
|------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------|
| 2026-05-18 | Initial draft. F-3 from BACKLOG.md: red-green paper trail in task closure. Externalized evidence files at `.speccy/specs/NNNN/evidence/T-NNN.md`; handoff template restructured (Hygiene checks table + Evidence field); reviewer-tests blocks on absence/fabrication; host-agnostic example ejection via `speccy init`; F-9 follow-up tracked in BACKLOG.md. | Kevin Xiao |
</changelog>
