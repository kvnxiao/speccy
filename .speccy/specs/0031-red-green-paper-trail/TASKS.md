---
spec: SPEC-0031
spec_hash_at_generation: ca5e06ac03783873ae44b2d365ef64605efb689ab9b2c77a8dac01d2cb4c1929
generated_at: 2026-05-19T01:21:17Z
---

# Tasks: SPEC-0031 Red-green paper trail in task closure


## Phase 1: Author the canonical worked example

<task id="T-001" state="completed" covers="REQ-004">
## T-001: Author `resources/modules/examples/evidence.md` as the canonical worked example

Pure additive change. Add a new directory `resources/modules/examples/`
under the `RESOURCES` bundle root and write the canonical evidence
worked example to `resources/modules/examples/evidence.md`. Nothing
else in the workspace consumes the file yet (T-002 wires the ejection
path, T-003 commits the in-tree drift-checked copy, T-004 references
the file from the implementer prompt). This task lands first so the
downstream consumers in T-002 / T-003 / T-004 have a stable source
path to point at.

The example body must demonstrate every shape the implementer prompt
and the reviewer-tests persona will rely on:

- An H1 markdown header naming the example (e.g. `# Evidence: T-NNN`
  worked example, or equivalent introductory header for the example
  file).
- A single `<evidence task="T-NNN" spec="SPEC-NNNN">…</evidence>`
  wrapper element block holding the body. The element is a
  writer/reader convention for LLM parseability; it is not parsed by
  the Speccy CLI.
- One `## Session <session-id> (attempt 1)` block carrying:
  - A `Command:` line naming the scoped command the implementer ran
    (any single framework is acceptable — cargo is conventional in
    this workspace; the example is illustrative, not normative).
  - A `<red exit="N">…</red>` element block carrying plausible
    verbatim runner output for the failing run (test name, error
    message, framework structural artifacts).
  - A `<green exit="0">…</green>` element block carrying plausible
    verbatim runner output for the passing run with materially
    different content from the red block.
- One `## Session <session-id> (attempt 2, no test delta)` block
  whose body is a single sentence describing what the second session
  did instead (e.g. doc edit, comment-only cleanup) without
  fabricating red/green output. The literal substring `no test delta`
  must appear in the header or body so reviewer-tests / implementer
  prompts can grep for it.
- A short intro paragraph (1-3 lines) above the `<evidence>` block
  explaining what the file demonstrates so a first-time reader has
  context before the XML wrapper opens.

The example stays small enough that referencing it via progressive
disclosure is materially cheaper than inlining (target body ≤ 60
lines including blank lines so the file remains a quick read). The
SPEC's REQ-004 done-when is the authority on minimum content; this
task does not add anything beyond what the SPEC requires (no
multi-framework sample, no second example file, no per-language
variant).

Suggested files:

- `resources/modules/examples/evidence.md`

<task-scenarios>
Given the workspace after this task lands, when
`resources/modules/examples/evidence.md` is read, then the file
exists, is non-empty, and contains exactly one `<evidence`
open-tag and exactly one matching `</evidence>` close-tag.

Given the same file, when grepped for `<red exit=`, then at least
one match exists and the same session block contains at least one
matching `<green exit="0">` element block whose body is materially
different from the `<red>` block (not identical or near-identical
output).

Given the same file, when grepped for `## Session`, then at least
two `## Session` markdown headers exist in source order: one for
the test-changing first attempt and one for the no-test-delta
retry attempt.

Given the same file, when grepped (case-insensitive) for the
literal substring `no test delta`, then at least one match exists
inside the second session block (header or body).

Given the same file, when its body line count is measured, then
the file is ≤ 60 lines so progressive disclosure remains cheap
for first-time readers.

Given `cargo test --workspace` run against the working tree at
the commit that lands this task, when its exit code is captured,
then the exit code is 0. The file is a pure additive resource;
the `include_dir!` snapshot in `speccy-cli/src/embedded.rs` picks
it up automatically (no `embedded.rs` source change required by
this task).

Given `cargo clippy --workspace --all-targets --all-features --
-D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny
check` run against the same commit, when each exits, then each
exit code is 0 (the resource is a markdown file and does not
trigger any of the four gates).
</task-scenarios>
</task>

## Phase 2: Wire the host-agnostic ejection path

<task id="T-002" state="completed" covers="REQ-004">
## T-002: Add `render_speccy_examples_pack` and wire host-agnostic ejection into `speccy init`

Add a parallel rendering path next to the existing
`render_host_pack` so that `speccy init` emits the entire contents
of `resources/modules/examples/*` (today: just `evidence.md`) to
`.speccy/examples/*` in the user's project, regardless of host
choice. The existing `render_host_pack` is unchanged; the
host-pack ejection still owns `.claude/` / `.codex/` /
`.agents/`.

Concretely:

- In `speccy-cli/src/render.rs`, add a new public function
  `render_speccy_examples_pack() -> Result<Vec<RenderedFile>, RenderError>`
  (or an equivalent function name; the planner's choice if it
  reads cleaner). The function walks the
  `RESOURCES`/`modules/examples/` subtree, treats each file as a
  static byte blob (no MiniJinja templating — the examples are
  host-agnostic markdown), and produces `RenderedFile` entries
  with `rel_path` rooted at `.speccy/examples/<filename>`. Reuse
  the existing `RenderedFile` shape; reuse the existing
  `RenderError` variants (`BundleSubpathMissing` is the natural
  fit if `resources/modules/examples/` is absent — but T-001 has
  guaranteed it exists, so this is a defensive path only).
- In `speccy-cli/src/init.rs`, extend the `build_plan` appender
  chain with a parallel `append_speccy_examples_items` (or
  equivalent) call placed after `append_host_pack_items`. The
  new appender takes the rendered files, joins each `rel_path`
  onto `project_root`, and pushes a `PlanItem` with the standard
  `Action::{Create, Overwrite, Skip}` classification from
  `classify(&destination)`. Examples are not host-native
  reviewer files (per SPEC DEC-004) so they are NOT classified as
  Skip-on-exists; they overwrite under `--force` like other
  template-rendered files.
- In `speccy-cli/tests/init.rs`, add at least two new tests:
  - One asserts that `speccy init --host claude-code` invoked
    against a fresh temp directory produces a file at
    `<tempdir>/.speccy/examples/evidence.md` with content
    byte-identical to
    `RESOURCES.get_file("modules/examples/evidence.md")` (or the
    `render_speccy_examples_pack` output).
  - One asserts the same for `speccy init --host codex` and
    additionally asserts that **no** file lands at any of the
    host-native examples paths (e.g.
    `.codex/agents/speccy-work/examples/evidence.md`,
    `.claude/skills/speccy-work/examples/evidence.md`).
- Do not add the in-tree committed copy in this task. T-003
  commits `.speccy/examples/evidence.md` to the workspace itself
  and adds the drift-check meta-test that asserts in-tree-vs-bundle
  parity. Splitting that out keeps T-002's diff focused on the
  ejection wiring and keeps the drift-check failure mode (which
  fires the moment T-002 lands without T-003) contained to one
  commit boundary.

Hygiene gate considerations:

- After T-002 lands but before T-003 commits the in-tree file,
  running `speccy init --force --host claude-code` against the
  workspace itself would create an uncommitted
  `.speccy/examples/evidence.md`. The existing CI guard
  (`git diff --exit-code .claude .codex .agents`) does not catch
  it (the diff target list does not yet include `.speccy/`),
  so CI stays green on T-002 alone. T-003 widens the CI guard and
  commits the file in the same commit.
- The `tests/init.rs` host-pack drift assertions (`CHK-008`
  family) walk only `render_host_pack` output today; the new
  examples-pack tests added in this task walk
  `render_speccy_examples_pack` output instead and live alongside
  the existing tests as parallel coverage. No existing test is
  modified.

Suggested files:

- `speccy-cli/src/render.rs`
- `speccy-cli/src/init.rs`
- `speccy-cli/tests/init.rs`

<task-scenarios>
Given `speccy-cli/src/render.rs` after this task lands, when its
public items are enumerated, then exactly one new public function
exists whose name matches `render_speccy_examples_pack` (or an
equivalent name agreed with the SPEC's Interfaces section) and
whose return type is `Result<Vec<RenderedFile>, RenderError>`. The
function walks `RESOURCES`/`modules/examples/` and produces
entries whose `rel_path` is rooted at `.speccy/examples/`.

Given `speccy-cli/src/init.rs` after this task lands, when its
`build_plan` body is read, then the appender chain calls the new
examples-pack appender after `append_host_pack_items`. The
examples plan items use the standard `classify(&destination)`
result without the host-native reviewer Skip-on-exists override
(examples are template-rendered, not user-tunable persona files).

Given `speccy init --host claude-code` invoked against a fresh
temp directory after this task lands, when the resulting on-disk
tree is inspected, then a file exists at
`<tempdir>/.speccy/examples/evidence.md` whose bytes match
`RESOURCES.get_file("modules/examples/evidence.md")` (or the
equivalent embedded-bundle accessor) byte-for-byte.

Given `speccy init --host codex` invoked against a fresh temp
directory after this task lands, when the resulting on-disk tree
is inspected, then a file exists at
`<tempdir>/.speccy/examples/evidence.md` byte-identical to the
embedded source, and no file exists at
`<tempdir>/.claude/skills/speccy-work/examples/evidence.md`,
`<tempdir>/.codex/agents/speccy-work/examples/evidence.md`, or
any other host-native examples path.

Given `speccy init --host claude-code` invoked twice against the
same temp directory (the second invocation `--force`), when the
plan output and the on-disk state are inspected, then the second
run reports an Overwrite (not Skip) for `.speccy/examples/evidence.md`
and the file remains byte-identical to the embedded source after
both runs.

Given `cargo build --workspace` and `cargo test --workspace`
against the commit that lands this task, when each is run, then
each exits 0 and the new `tests/init.rs` tests (at least one for
each host plus one --force/idempotency test) pass.

Given `cargo clippy --workspace --all-targets --all-features --
-D warnings` against the same commit, when run, then the exit
code is 0 and no new warnings are emitted by the new code in
`render.rs` or `init.rs` (no `unwrap_used`, `expect_used`,
`panic`, `unreachable`, `indexing_slicing`, `result_large_err`,
etc.). The `RenderError` surface is reused; no new error variants
are introduced.

Given `cargo +nightly fmt --all --check` and `cargo deny check`
against the same commit, when each is run, then each exits 0
(no new dependencies; formatting is idiomatic).
</task-scenarios>
</task>

## Phase 3: Commit the in-tree example, lock in drift checks, widen CI guard

<task id="T-003" state="completed" covers="REQ-004">
## T-003: Commit `.speccy/examples/evidence.md` in-tree, add drift-check meta-test, widen CI workflow diff target list

Lock the in-tree workspace into the same ejection contract every
downstream `speccy init` invocation pays. Without this slice, T-002
emits an uncommitted file every time Speccy dogfoods `speccy init`
against itself, and the existing CI host-pack drift guard does not
catch it.

Concretely:

- Add a committed file at `.speccy/examples/evidence.md` whose
  content is byte-identical to
  `resources/modules/examples/evidence.md` (the source T-001
  authored, now also reachable via the
  `RESOURCES`/`modules/examples/evidence.md` embedded accessor).
  The simplest way to produce this file is to run
  `./target/debug/speccy init --force --host claude-code` against
  the workspace and `git add .speccy/examples/evidence.md`; T-002's
  ejection wiring is the source of truth for what gets written.
- Add a drift-check meta-test in `speccy-cli/tests/init.rs` (or a
  sibling integration-test file if that reads cleaner) that
  mirrors the existing host-pack drift assertion pattern: the
  test reads the bytes of the committed in-tree
  `.speccy/examples/evidence.md` (resolved via a workspace-root
  helper that the existing host-pack test already uses) and the
  bytes of the embedded
  `RESOURCES.get_file("modules/examples/evidence.md")`, then
  asserts equality. A drift surfaces with a clear diagnostic
  naming both paths and pointing the contributor at
  `speccy init --force --host claude-code` as the refresh
  command. Re-use the existing workspace-root resolver / file-read
  helper rather than introducing a new one; the host-pack test at
  `tests/init.rs:850-1000` is the precedent.
- Widen the CI workflow guard in `.github/workflows/ci.yml`. The
  current step (line 55, "Materialized host packs are in sync
  with resources/ source") runs `git diff --exit-code .claude
  .codex .agents` after both `speccy init --force --host <host>`
  invocations. After this task, the diff target list must include
  `.speccy/examples` (or `.speccy` — preferred since it future-proofs
  the guard against further host-agnostic ejection paths) so that
  drift in the ejected examples surface fails CI with the same
  contributor-facing message the host-pack guard emits today. Update
  the failure-message string in the same step so the error names the
  examples surface as well.
- Update the `speccy-cli/tests/ci_workflow.rs` meta-test's
  `DIFF_COMMAND` constant (line 20) to match the widened CI
  workflow guard so the workflow-content guard and the actual CI
  step stay in lockstep. The constant is the literal substring
  the meta-test asserts the workflow contains; it must include
  whatever new directory the CI step now diffs against.

Hygiene gate considerations:

- After this task lands, `git diff --exit-code .claude .codex
  .agents .speccy/examples` (or the equivalent widened form) is
  clean: the in-tree `.speccy/examples/evidence.md` exists and
  matches what `render_speccy_examples_pack` would emit.
- The drift-check meta-test runs under `cargo test --workspace`
  and is the same shape as the host-pack drift assertion the
  workspace already enforces. It catches future divergence
  between `resources/modules/examples/evidence.md` (edited by a
  contributor) and `.speccy/examples/evidence.md` (forgotten in
  the same PR), failing the test with a clear refresh hint.

Suggested files:

- `.speccy/examples/evidence.md`
- `speccy-cli/tests/init.rs`
- `.github/workflows/ci.yml`
- `speccy-cli/tests/ci_workflow.rs`

<task-scenarios>
Given the workspace after this task lands, when
`.speccy/examples/evidence.md` is read, then it exists, is
non-empty, and its bytes are byte-identical to
`resources/modules/examples/evidence.md` (i.e. the embedded
source T-001 authored).

Given `speccy-cli/tests/init.rs` after this task lands, when its
test names are enumerated, then a new test exists whose body
asserts byte-equality between the in-tree
`.speccy/examples/evidence.md` and the embedded
`RESOURCES`/`modules/examples/evidence.md`. The test name follows
the existing host-pack-drift naming style (e.g.
`in_tree_examples_match_embedded`,
`dogfood_examples_match_committed_tree`, or similar).

Given the same drift-check test, when the workspace's in-tree
file is artificially mutated (e.g. one byte flipped) and the
test is re-run, then the test fails with a diagnostic that names
both the in-tree path and the embedded-bundle path, and
references `speccy init --force` as the refresh command. (This
scenario is the negative-path acceptance for the drift check.)

Given `.github/workflows/ci.yml` after this task lands, when its
"Materialized host packs are in sync with resources/ source"
step is read, then the `git diff --exit-code` line names
`.speccy/examples` (or the broader `.speccy`) in addition to the
existing `.claude .codex .agents` targets, and the failure
message names the widened surface in the operator-facing prose.

Given `speccy-cli/tests/ci_workflow.rs` after this task lands,
when its `DIFF_COMMAND` constant (currently
`git diff --exit-code .claude .codex .agents`) is read, then it
matches the widened CI workflow guard byte-for-byte (i.e. the
constant and the workflow agree on the diff target list).

Given `cargo test --workspace` against the commit that lands this
task, when run, then the exit code is 0; the new drift-check
test passes (in-tree file matches embedded source), and the
ci_workflow tests pass against the updated workflow file.

Given `git diff --exit-code .claude .codex .agents .speccy/examples`
(or the equivalent widened form) run after
`./target/debug/speccy init --force --host claude-code` and
`./target/debug/speccy init --force --host codex` against the
workspace at this commit, when each is run, then both diffs exit
0 — the in-tree workspace is in lockstep with the embedded
source.

Given `cargo clippy --workspace --all-targets --all-features --
-D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny
check` against the same commit, when each is run, then each
exit code is 0.
</task-scenarios>
</task>

## Phase 4: Implementer prompt edits (workflow, handoff, evidence shape)

<task id="T-004" state="completed" covers="REQ-001 REQ-002 REQ-003">
## T-004: Edit `resources/modules/prompts/implementer.md` to narrate the red-green workflow, collapse `Commands run`/`Exit codes` into `Hygiene checks`, add the `Evidence` field, and reference the example via progressive disclosure

Single-file edit landing the three implementer-side prompt
contracts:

1. **Workflow narration** (REQ-003): the "Your task" section
   (today: steps 1–7 at `resources/modules/prompts/implementer.md:55-90`)
   is restructured so the numbered steps walk the implementer
   through the red-green sequence in execution order. The
   SPEC enumerates the nine-step shape verbatim; the prompt's
   numbered list must follow that ordering (read SPEC; read
   `<task-scenarios>`; write failing test/scoped command; capture
   red into evidence file; implement code; capture green; run
   hygiene; append handoff note; flip state). The no-test-delta
   retry substitution (steps 3–6 collapse into appending a
   no-test-delta session block) is documented either inside the
   relevant step or as an immediately adjacent note. Compile-
   failure-as-red is named explicitly in the red-capture step.
   The narration stays framework-agnostic: no `cargo test foo`,
   `pnpm test bar`, `pytest`, etc. inside normative prose.

2. **Evidence file shape documentation** (REQ-002): the
   workflow-narration section (or a sibling section if the
   planner chooses a different layout) documents the evidence
   file path shape `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md`
   literally, names both the "session changed tests" and the
   "no test delta" session-block shapes (with the `<red>` /
   `<green>` element block forms for the former and a single-
   sentence body for the latter), names the append-only invariant
   (a session block is added at the end; prior sessions stay
   verbatim), and acknowledges compile-failure-as-red as
   legitimate red evidence. The section stays framework-agnostic
   (no per-framework anchor strings, no `cargo` / `pnpm` / `pytest`
   / `jest` / `vitest` / `mocha` / `rspec` mentions inside the
   evidence-shape documentation).

3. **Handoff template** (REQ-001): the "Handoff template" section
   (today: lines 92-108) is rewritten so the appended
   `<implementer-note session="...">` element block carries six
   fields in this order: `Completed`, `Undone`, `Hygiene checks`,
   `Evidence`, `Discovered issues`, `Procedural compliance`. The
   previous `Commands run` / `Exit codes` parallel-list field pair
   is retired entirely (no transitional form is documented). The
   `Hygiene checks` body is documented as a `| Command | Status |`
   markdown table whose `Status` cells render as `pass (exit 0)` or
   `fail (exit N)`. The `Evidence` body is documented as the
   project-relative path to the per-task evidence file plus a
   ` — ` delimiter plus a one-line red→green summary naming the
   scoped command and its red and green exit codes (substantially
   equivalent prose is acceptable). The section explicitly notes
   that every field is required and that an empty field carries
   the literal placeholder `(none)`.

4. **Progressive-disclosure reference** (REQ-004 reference subset):
   the workflow-narration section references
   `.speccy/examples/evidence.md` literally at least once and
   instructs the implementer to read it via the host Read
   primitive for the full shape on first encounter. An inline
   sketch of the evidence-shape (≤ 5 lines) is the entire
   payload kept inside the prompt body; the 30-ish-line worked
   example is NOT duplicated into the prompt.

The "When you hit friction" section (lines 29-53) is unchanged
unless the planner judges that the friction-fix worked example's
`Commands run` mention has drifted in a way that conflicts with
the new template. (Spot-check: the friction example today does
not reference the handoff template's `Commands run` / `Exit
codes` fields directly, so it should not need to change.)

Hygiene gate considerations:

- The prompt is a `.tmpl` file under `resources/modules/prompts/`;
  the MiniJinja loader and the `speccy implement` command render
  it via `speccy_core::prompt`. The prompt-module loader strict
  mode catches missing `{% include %}` references at render time;
  the SPEC's REQ-007 behavior bullet names this as the
  rendering-time invariant. Edits inside this task do NOT touch
  the include graph (no new `{% include %}` is added; no existing
  one is removed); they edit prose only.
- After this task lands, the existing `tests/render.rs` and
  `tests/init.rs` rendering assertions still pass: `render_host_pack`
  output produces a SKILL.md / prompt whose body matches the new
  field names. Any in-tree assertion that asserted on the literal
  substrings `Commands run:` or `Exit codes:` inside the handoff
  template (none expected — these are writer-side prose, not
  parser-tested) must be updated to match the new shape. Run a
  quick `grep -rn "Commands run\|Exit codes" speccy-cli/tests/
  speccy-core/tests/` before this task closes and update any
  surviving matches the planner did not anticipate.
- `cargo test --workspace` must exit 0; assertions that pin the
  rendered handoff-template text to specific field names are
  updated to the new names where applicable.

Suggested files:

- `resources/modules/prompts/implementer.md`

<task-scenarios>
Given `resources/modules/prompts/implementer.md` after this task
lands, when its "Handoff template" section is read, then the
six field names appear in this order: `Completed`, `Undone`,
`Hygiene checks`, `Evidence`, `Discovered issues`, `Procedural
compliance`. The literal substrings `Commands run:` and
`Exit codes:` do not appear inside the handoff-template section.

Given the same file, when its handoff-template `Hygiene checks`
body is read, then the documented shape is a markdown table with
exactly two columns named `Command` and `Status`, and the example
demonstrates at least one row whose `Status` cell renders as
`pass (exit 0)` or `fail (exit N)`.

Given the same file, when its handoff-template `Evidence` body
is read, then the documented shape names a project-relative path
to `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md` followed by
a ` — ` delimiter and a one-line red→green summary naming the
scoped command and the red/green exit codes (or substantially
equivalent prose).

Given the same file, when its "Your task" section is parsed as
a numbered list, then the steps include, in source order, the
semantic actions: read covered SPEC requirements; read task
scenarios; write failing test or scoped verification command;
capture red into evidence file under a `<red exit="N">` element;
implement code; capture green into a `<green exit="0">` element;
run project hygiene gates; append handoff note; flip state to
`in-review`.

Given the same file, when grepped for the literal substring
`.speccy/specs/`, then at least one match exists inside the
workflow-narration section documenting the evidence file path
shape `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md`.

Given the same file, when grepped for the literal substrings
`<red exit=` and `<green exit=`, then at least one match exists
for each inside the documented session-block shape.

Given the same file, when grepped (case-insensitive) for the
literal substring `no test delta`, then at least one match
exists inside the workflow-narration section documenting the
no-test-change retry path.

Given the same file, when grepped (case-insensitive) for the
literal substring `compile`, `build error`, or `cannot find`,
then at least one match exists inside the red-phase guidance
naming compile-failure-as-red as legitimate red evidence.

Given the same file, when grepped for the literal substring
`.speccy/examples/evidence.md`, then at least one match exists
inside the workflow-narration section's progressive-disclosure
reference.

Given the same file, when grepped for the literal substrings
`cargo test foo`, `pnpm test bar`, `pytest`, `jest`, `vitest`,
`mocha`, or `rspec` inside the workflow-narration's normative
prose, then zero matches are found (worked example asides and
"e.g." parentheticals are out of scope for this anti-pattern
check; the contract is on normative instruction prose).

Given `speccy implement SPEC-NNNN/T-NNN` rendered for any in-tree
task after this task lands, when the rendered prompt's
handoff-template section is captured, then it contains the new
field names verbatim (`Hygiene checks`, `Evidence`) and does not
contain the retired field labels (`Commands run`, `Exit codes`).

Given `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly
fmt --all --check`, and `cargo deny check` against the commit
that lands this task, when each is run, then each exits 0. The
prompt-module loader resolves every `{% include %}` reference
without missing-include diagnostics.
</task-scenarios>
</task>

## Phase 5: Reviewer-tests prompt and persona edits

<task id="T-005" state="completed" covers="REQ-005">
## T-005: Edit `resources/modules/personas/reviewer-tests.md` and `resources/modules/prompts/reviewer-tests.md` to load the evidence file, block on absence or fabrication, and stay framework-agnostic

Two-file edit landing the reviewer-tests-side contracts. The
asymmetry — only the `tests` persona reads evidence; the other
five built-in personas continue to anchor on diff + SPEC alone —
is load-bearing (DEC-003); this task explicitly does NOT touch
the other persona or prompt files.

In `resources/modules/personas/reviewer-tests.md`:

- Add an explicit focus item / numbered step block that names the
  four-step evidence-loading sequence:
  1. Locate the `Evidence:` field inside each `<implementer-note>`
     element body on the task.
  2. Read the referenced evidence file via the host Read primitive.
  3. Treat the absence of the `Evidence:` field, or the absence
     of the referenced evidence file, as a `verdict="blocking"`
     review. The blocking summary names what is missing (no
     `Evidence:` field / evidence file not found at path).
  4. Treat fabricated-looking evidence content as a
     `verdict="blocking"` review.
- Enumerate the fabrication patterns the reviewer must scrutinize.
  At minimum the five SPEC-named patterns must appear (each as a
  bullet or equivalent enumerated item):
  - Output that lacks the structural artifacts a real test/build
    runner would emit (no test names, no error messages, no
    stack frames where applicable).
  - Test names inside the evidence file that do not appear in the
    diff under review.
  - Identical or near-identical red and green output (a real
    red→green transition produces materially different output).
  - Suspiciously clean output that omits the usual verbose
    framework headers, summaries, or timing prose.
  - Output that names a command the rendered `Hygiene checks`
    table also names — the evidence command should be a scoped
    per-test/per-slice invocation, not the full-suite hygiene run.
- Stay framework-agnostic: do not name per-framework anchor
  strings (`test result: FAILED`, ` ✗ `, `FAILED:`, `error[E`,
  `cargo test`, `pnpm test`, `pytest`, `jest`, `vitest`, etc.)
  inside normative guidance. Worked example asides may name
  illustrative output if the planner judges them helpful, but the
  normative instructional prose stays framework-agnostic.

In `resources/modules/prompts/reviewer-tests.md`:

- Add a step (inside the "Your task" section or a sibling section)
  that instructs the reviewer to extract the `Evidence:` path from
  each `<implementer-note>` element body on the task and to read
  the file via the host Read primitive before applying the
  persona's fabrication-pattern guidance to the loaded content.
- The rendered reviewer-tests prompt continues to be a single-file
  prompt rendered by `speccy review SPEC-NNNN/T-NNN --persona
  tests`; the prompt does not gain any new MiniJinja context
  variables. The evidence-loading instruction is prose the
  reviewer executes via the host Read primitive — the CLI does
  not load the file at render time.

Negative-coverage scope (verified before this task closes):

- `resources/modules/personas/reviewer-business.md`,
  `resources/modules/personas/reviewer-security.md`,
  `resources/modules/personas/reviewer-style.md`,
  `resources/modules/personas/reviewer-architecture.md`,
  `resources/modules/personas/reviewer-docs.md` are NOT edited.
- `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `resources/modules/prompts/reviewer-architecture.md`,
  `resources/modules/prompts/reviewer-docs.md` are NOT edited.
- Pre-implementation grep: `grep -rn "Evidence:\|evidence file"
  resources/modules/personas/reviewer-{business,security,style,architecture,docs}.md
  resources/modules/prompts/reviewer-{business,security,style,architecture,docs}.md`
  returns zero matches both before and after this task lands;
  the asymmetry is the design property.

Suggested files:

- `resources/modules/personas/reviewer-tests.md`
- `resources/modules/prompts/reviewer-tests.md`

<task-scenarios>
Given `resources/modules/personas/reviewer-tests.md` after this
task lands, when grepped for the literal substring `Evidence:`,
then at least one match exists inside normative guidance
instructing the reviewer to locate and read the evidence file.

Given the same file, when grepped for the literal substring
`blocking` within prose proximity to `Evidence` or `evidence
file`, then at least one match exists inside guidance naming
evidence absence as a blocking trigger.

Given the same file, when its fabrication-pattern enumeration is
read, then it includes at least the five SPEC-named patterns:
(i) lack of framework artifacts, (ii) test names absent from
diff, (iii) identical or near-identical red and green output,
(iv) suspiciously clean output that omits framework headers /
summaries / timing prose, and (v) evidence-command matching the
hygiene full-suite invocation.

Given the same file, when grepped (case-sensitive) inside
normative guidance for the literal substrings `test result:
FAILED`, ` ✗ `, `FAILED:`, `error[E`, `cargo test`, `pnpm test`,
`pytest`, `jest`, or `vitest`, then zero matches are found.
(Worked-example asides or `<!-- ... -->` annotations are out of
scope for this anti-pattern check; the contract is on normative
instructional prose.)

Given `resources/modules/prompts/reviewer-tests.md` after this
task lands, when its rendered output for a task with one or
more `<implementer-note>` elements is captured, then the
captured prompt contains an instruction to extract the
`Evidence:` path from each `<implementer-note>` body and to read
the file via the host Read primitive.

Given each of the files
`resources/modules/personas/reviewer-{business,security,style,architecture,docs}.md`
and
`resources/modules/prompts/reviewer-{business,security,style,architecture,docs}.md`
after this task lands, when grepped for the literal substrings
`Evidence:` or `evidence file`, then zero matches are found
inside normative guidance. (Pre-existing matches in unrelated
contexts — if any — are out of scope for this check; the
contract is that the new asymmetry is honored.)

Given the rendered review prompts produced by `speccy review
SPEC-NNNN/T-NNN --persona <P>` for each persona in
`speccy_core::personas::ALL` after this task lands, when the six
rendered prompts are captured, then exactly one — the `tests`
persona's prompt — contains an evidence-loading instruction; the
other five contain no such instruction.

Given `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly
fmt --all --check`, and `cargo deny check` against the commit
that lands this task, when each is run, then each exits 0.
</task-scenarios>
</task>

## Phase 6: BACKLOG.md follow-up entry and final hygiene confirmation

<task id="T-006" state="completed" covers="REQ-006 REQ-007">
## T-006: Add the F-9 entry to `.speccy/BACKLOG.md` and confirm the four standard-hygiene gates exit 0 against the cumulative post-SPEC workspace

Pure documentation slice plus the final hygiene-gate closure
that REQ-007 names. Lands last so the implementer-note for this
task is the cumulative proof that the SPEC's four-gate contract
holds against the merged work from T-001 through T-005.

Concretely:

- In `.speccy/BACKLOG.md`, add a new `F-9` entry under Tier 2
  (alongside F-8 and F-6). The entry follows the existing
  four-field format used by F-3, F-4, F-5, F-6, F-8:
  - what is being asked (migrate other inline examples across
    `resources/modules/personas/*.md` and
    `resources/modules/prompts/*.md` to the progressive-disclosure
    pattern this SPEC established);
  - why (per-invocation token cost; reduce duplicated example
    bodies that bloat every persona's rendered prompt);
  - where (the 7 persona files plus the 7-or-so prompt files
    under `resources/modules/`);
  - heuristic / cost / risk (eject when an example is ≥ ~8 lines
    or used by ≥ 2 consuming prompts; risk is over-ejection of
    tiny shape sketches that read more clearly inline).
- The F-9 entry references SPEC-0031 explicitly as the pattern's
  source (e.g. "Pattern established by SPEC-0031 (F-3 red-green
  paper trail)") so a future reader can trace the precedent.
- Do not edit the existing F-3 entry under Tier 1. F-3's closure
  annotation — `F-3: ... — **closed by SPEC-0031 (YYYY-MM-DD)**`
  plus the closure prose — is written at ship time by the
  `speccy-ship` skill, mirroring the F-7 → SPEC-0030 closure
  pattern. This task does NOT pre-emptively close F-3.

After the BACKLOG.md edit lands, this task's hygiene gate run is
the cumulative proof for REQ-007:

- `cargo test --workspace` exits 0 against the cumulative
  workspace (T-001's resource, T-002's ejection wiring + tests,
  T-003's drift-check meta-test, T-004's prompt edits, T-005's
  reviewer-tests edits).
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0. Any surviving warnings are explicit
  carry-forward from prior SPECs (per SPEC §REQ-007 the
  `result_large_err` carry from SPEC-0026 / SPEC-0030 is named
  if it is present in clippy output and noted in REPORT.md).
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0 (no new dependencies added in this
  SPEC).

If any of the four gates fail at this task's run boundary, the
failure is a regression in one of the upstream tasks (T-001
through T-005) and the right move is to surface it in
`Discovered issues` on this task's `<implementer-note>` and to
prefer a targeted fix to the upstream task over folding the fix
into this task. The Cumulative-hygiene check is the SPEC's last
line of defense, not a catch-all sink for upstream slop.

Suggested files:

- `.speccy/BACKLOG.md`

<task-scenarios>
Given `.speccy/BACKLOG.md` after this task lands, when grepped
for the regex `^F-9:`, then exactly one match exists.

Given the same file, when the F-9 entry body is read (the
sequence of bullet lines following the `F-9:` header through to
the next `F-NN:` or `R-NN:` header, or the next `Tier N` /
end-of-file boundary), then it contains four field-equivalents:
- a what bullet naming the inline-example migration to
  progressive disclosure;
- a why bullet citing per-invocation token cost and citing
  SPEC-0031 (or F-3) as the pattern's origin;
- a where bullet naming `resources/modules/personas/*.md` and
  `resources/modules/prompts/*.md`;
- a heuristic / risk bullet naming the eject-vs-inline threshold
  (example ≥ ~8 lines or used by ≥ 2 consuming prompts → eject)
  and the over-ejection risk.

Given the same file, when grepped for the regex `^F-3:` after
this task lands, then exactly one match exists for the existing
F-3 entry under Tier 1 and the entry does NOT carry a `**closed
by SPEC-0031**` annotation (the closure is `speccy-ship`'s
responsibility at the ship-commit boundary, not this task's).

Given the same file, when its Tier-2 section is read, then the
F-9 entry sits alongside F-8 and F-6 (the existing Tier-2
entries) and preserves the existing tier-grouping convention.

Given `cargo test --workspace` against the cumulative workspace
at the commit that lands this task, when run, then the exit code
is 0 and every new test added by T-002 and T-003 (the
host-agnostic ejection tests and the drift-check meta-test)
passes.

Given `cargo clippy --workspace --all-targets --all-features --
-D warnings` against the same commit, when run, then the exit
code is 0. Any surviving warnings are explicit carry-forward
from prior SPECs and named in REPORT.md at ship time.

Given `cargo +nightly fmt --all --check` against the same
commit, when run, then the exit code is 0.

Given `cargo deny check` against the same commit, when run,
then the exit code is 0 (no new dependencies are introduced by
SPEC-0031).

Given the new prompt-module edits from T-004 and T-005, when the
prompt-module loader runs at render time for `speccy implement
SPEC-NNNN/T-NNN` and `speccy review SPEC-NNNN/T-NNN --persona
<P>` against any in-tree task, then every `{% include %}`
reference resolves without missing-include diagnostics. (This
scenario double-covers REQ-007's rendering-time invariant from
T-004/T-005 against the cumulative workspace.)
</task-scenarios>
</task>

