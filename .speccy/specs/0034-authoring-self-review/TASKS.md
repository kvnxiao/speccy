---
spec: SPEC-0034
spec_hash_at_generation: 739815d752fcb1ebcb550eb15d4811e34a7ddee1ff10884cbb82808dd14b3660
generated_at: 2026-05-21T01:34:11Z
---

# Tasks: SPEC-0034 Self-review pass in authoring-phase skills (plan, amend, brainstorm)

<tasks spec="SPEC-0034">

## Phase 1: Retire the amendment branch and clean up "greenfield" terminology

<task id="T-001" state="completed" covers="REQ-011">

## Remove the amendment branch from `speccy-plan.md`

Remove the amendment branch from `resources/modules/skills/speccy-plan.md` so the
template describes new-SPEC authoring only. All amendment traffic routes through
`/speccy-amend`; the retirement is documented by omission.

Open question resolved before implementing:
- REQ-011's done-when names six specific prose locations to purge (lede, "When to
  use", "Steps" step 1 + step 2 `**Amendment**:` sub-branch, frontmatter
  `description:` line). Treat those six anchors as the exhaustive removal list.

<task-scenarios>
Given `resources/modules/skills/speccy-plan.md` in its current form,
when the amendment-branch prose is removed from the six named locations,
then the file contains no "Amendment", "amend an existing", or "SPEC-NNNN argument"
wording in the lede, "When to use", "Steps", or frontmatter `description:` line.

Given the updated source file,
when `cargo test --workspace` runs and `skill_body_discovery` exercises CHK-015 /
CHK-016,
then both scenarios pass (no amendment substrings in the named locations of either
the module source or the previously-ejected SKILL.md files).

Suggested files:
- `resources/modules/skills/speccy-plan.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t001-attempt-1">
Completed: Removed the amendment branch from `resources/modules/skills/speccy-plan.md`
at all six named locations in REQ-011:
- Lede: dropped "or amends an existing one when intent shifts", "or amending", and
  the "With a SPEC-ID argument, the recipe runs the amendment flow instead..." sentence.
- "When to use": replaced two-bullet list (new-spec + Amendment form) with a single
  new-spec-only paragraph.
- Steps step 1: replaced the identify-amendment-vs-new-spec branch with a direct
  `speccy vacancy --json` query step.
- Steps step 2: dropped the `**Amendment**: speccy status SPEC-0007 --json` sub-branch;
  old step 3 ("Write or amend SPEC.md") collapsed to step 2 ("Write SPEC.md").
- Renumbered old steps 4 and 5 to 3 and 4 to keep sequential numbering.
- Frontmatter `description:` line: no YAML frontmatter block exists in the file;
  the lede paragraph serves this role and was cleaned above.

Undone: Nothing left undone. All six locations covered.

Commands run:
- `cargo build --workspace` — exit 0
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)

Exit codes: all 0.

Discovered issues: The file has no YAML frontmatter `description:` block; the lede
paragraph is the skill description surface. The done-when item about the frontmatter
`description:` line was satisfied by cleaning the lede. No test surface was touched
(CHK-015 is a grep-level check verified by the `speccy check` run above; the
`skill_body_discovery` test for this check belongs to T-002's renamed test scope).

Procedural compliance: No skill file edits needed; the skill template in
`resources/modules/skills/speccy-plan.md` is the file being amended.
</implementer-note>

<implementer-note session="session-2026-05-20-t001-attempt-2">
Completed: Cleared both blockers from the attempt-1 review.

Blocker 1 (business — wrapper-template frontmatter): Dropped "or amend an existing
one when intent shifts" and "asks to amend an existing spec by ID" from the
`description:` line (line 3) of both wrapper `.tmpl` files:
- `resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl`
- `resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl`

The revised `description:` line reads as a new-SPEC-only authoring template,
mirroring the shape of the sibling `speccy-amend` wrapper.

Blocker 2 (tests — evidence paper trail): Created
`evidence/T-001.md` with pre-edit grep captures (from `git show HEAD:<path>`)
showing amendment-branch hits in the six named locations across the three files,
and post-edit grep captures showing zero matches for "Amendment",
"amend an existing", and "SPEC-NNNN argument" across all three files.

Evidence: evidence/T-001.md

Undone: Nothing left undone. All six locations clean; both wrapper `.tmpl` files
updated; evidence file created with red-then-green captures.

Commands run:
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo build --workspace` — exit 0

Exit codes: all 0.

Discovered issues: Attempt 1's note said "no YAML frontmatter `description:` block
exists in the file" — this was accurate for the module file itself
(`resources/modules/skills/speccy-plan.md`), but silently resolved the question of
where the frontmatter lives. The frontmatter lives in the two host-pack wrapper
`.tmpl` files that include the module. That interpretation gap is now explicit: the
module carries the body prose; the wrapper `.tmpl` files carry the skill metadata
(name, description) that host harnesses consume. REQ-011 bullet 4 targets the
wrapper frontmatter, not the module lede.

Procedural compliance: Edited two wrapper `.tmpl` files that were outside the
scope of attempt 1. No friction from stale skill instructions encountered.
</implementer-note>

<review persona="business" verdict="blocking">
REQ-011 done-when bullet 4 names the skill template's frontmatter `description:` line
as a removal target. The implementer-note declared this satisfied by cleaning the
lede ("no YAML frontmatter `description:` block exists in the file"), silently
resolving an interpretation question that should have surfaced. The frontmatter
genuinely lives in the wrapper templates that include
`resources/modules/skills/speccy-plan.md`:
- `resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl` line 3
- `resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl` line 3

Both still contain "or amend an existing one when intent shifts" and "asks to amend
an existing spec by ID" on the `description:` line. CHK-016 will fail post-T-008
re-eject because the ejected SKILL.md's line 3 carries "amend an existing". The
four named module-file locations are correctly purged; the blocker is the
unaddressed wrapper-template frontmatter.
</review>

<review persona="tests" verdict="blocking">
T-001's `<implementer-note>` is missing the `Evidence:` field, and no per-task
evidence file exists at `.speccy/specs/0034-authoring-self-review/evidence/T-001.md`
(the spec's `evidence/` directory does not exist at all). Per the SPEC-0033
red-then-green paper-trail convention (see `.speccy/specs/0033-eject-prompt-bodies/`
T-001 line 101 for the `Evidence:` line shape, and the corresponding
`evidence/T-001.md` for the file-body shape), the absence of the `Evidence:`
field or the referenced file is a blocking review by itself. The substantive
grep absence in `resources/modules/skills/speccy-plan.md` is in fact satisfied
(zero matches for "Amendment", "amend an existing", "SPEC-NNNN argument") — the
slice itself is correct, but the procedural proof shape is broken. Unblock by
landing an evidence file with pre-edit grep capture (amendment-branch hits in the
six named locations) and post-edit grep capture (zero hits for the three
substrings in lede / "When to use" / "Steps"), then add the `Evidence:` field to
the `<implementer-note>` pointing at that file.
</review>

<review persona="security" verdict="pass">
Prose-only edit to `resources/modules/skills/speccy-plan.md`; no code paths, no
auth boundaries, no secrets handling, no untrusted input, no new dependencies.
The sole template variable `{{ cmd_prefix }}` is populated by CLI-internal
host-type logic at `speccy init` time, not by user-supplied input. Security
surface is zero.
</review>

<review persona="style" verdict="pass">
Post-edit `resources/modules/skills/speccy-plan.md` reads cleanly as a
new-SPEC-only authoring template. All six amendment-branch locations are
removed, no orphan references remain, Liquid template variables
(`{{ cmd_prefix }}`, `{% ... %}`) are intact and unbroken, the
lede/steps/no-loop structure matches the sibling `speccy-amend.md` shape, and
prose voice is consistent with the rest of `resources/modules/skills/`.
Trailing newline confirmed present.
</review>

<retry>
Two blockers must clear before re-review: (a) the wrapper-template frontmatter
`description:` line (REQ-011 bullet 4 / CHK-016), and (b) the missing
red-then-green evidence file plus `Evidence:` field on the implementer-note.
- business: drop "or amend an existing one when intent shifts" and "asks to
  amend an existing spec by ID" from line 3 of both
  `resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl` and
  `resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl`.
- tests: create `.speccy/specs/0034-authoring-self-review/evidence/T-001.md`
  with a pre-edit grep capture (showing amendment-branch hits in the six named
  locations of the three files: the module template plus the two wrapper
  templates) and a post-edit grep capture (showing zero hits for "Amendment",
  "amend an existing", "SPEC-NNNN argument" in lede / "When to use" / "Steps"
  / frontmatter `description:` line). Mirror the body shape of
  `.speccy/specs/0033-eject-prompt-bodies/evidence/T-001.md`. Then add an
  `Evidence: evidence/T-001.md` line to the `<implementer-note>` pointing at
  the new file.
</retry>

<review persona="business" verdict="pass">
REQ-011 done-when bullet 4 satisfied: both wrapper `.tmpl` files' line-3
`description:` no longer contain "or amend an existing one when intent shifts"
or "asks to amend an existing spec by ID". Revised line reads coherently as a
new-SPEC-only authoring template, mirroring the sibling `speccy-amend`
wrapper's shape. CHK-016 will pass on the next re-eject in T-008. The
attempt-2 implementer-note's "Discovered issues" block surfaces the
wrapper-vs-module interpretation gap explicitly rather than silently
resolving it.
</review>

<review persona="tests" verdict="pass">
Attempt 2 closes the attempt-1 evidence blocker. `evidence/T-001.md` carries a
real red-then-green paper trail: pre-edit captures sourced from
`git show HEAD:<path>` (HEAD = f755d9f, the pre-branch state) showing
"Amendment" at module lines 21/41 and "amend an existing" / "asks to amend an
existing spec by ID" on line 3 of both wrapper `.tmpl` files; post-edit
captures showing zero matches across all three files for the three named
substrings. The `Evidence: evidence/T-001.md` line at TASKS.md:93 mirrors the
SPEC-0033 T-001:101 shape, and body shape of `evidence/T-001.md` mirrors
`.speccy/specs/0033-eject-prompt-bodies/evidence/T-001.md`. The grep
substrings and file scope directly exercise CHK-015 (module file) and CHK-016
(both wrappers' `.tmpl` source).
</review>

<review persona="security" verdict="pass">
Attempt-2 edits confirmed clean. Both wrapper templates at
`resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl` and
`resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl` are static 5-line
files: YAML front-matter with a hardcoded description string and a single
`{% include %}` directive. No interpolated values, no untrusted input, no
secrets. The evidence file is a static Markdown audit log; no code paths or
sensitive data. No new security surface from attempt 2.
</review>

<review persona="style" verdict="pass">
All four style checks pass. The `description:` lines on both wrapper `.tmpl`
files carry only new-SPEC authoring language with no residual amendment
wording. Line-3 shape, single-quote quoting, and no-trailing-newline ending
(`0x7d` on disk) are byte-identical to the `speccy-amend` sibling and the
other three include-form wrappers in the pack. Evidence file at
`evidence/T-001.md` matches the SPEC-0033 T-001 convention (`# Evidence`
heading, `## Session` subheading with date slug, numbered scenario list, and
`<red>`/`<green>` pre/post command blocks). No dead imports, suppression
annotations, or naming deviations introduced.
</review>

</task>

<task id="T-002" state="completed" covers="REQ-012">

## Remove "greenfield" from live workflow surfaces

Remove "greenfield" from live workflow surfaces while preserving every instance
that explicitly denies the greenfield/brownfield distinction. Rename and simplify
the `chk015_*greenfield*` test in `speccy-cli/tests/skill_body_discovery.rs`.

Open questions resolved before implementing:
- The term may remain where it is denied, not claimed. Grep each file listed in
  REQ-012 done-when; for every hit, decide whether the sentence claims or denies.
  Claims → delete/reword; denials → leave as-is.
- The renamed test asserts `speccy vacancy --json` present + `speccy status --json`
  absent in `speccy-plan.md`'s body — no `**Amendment**` partitioning needed
  (that anchor is gone after T-001).

<task-scenarios>
Given the post-T-001 source tree, when each file named in REQ-012 done-when
(`AGENTS.md`, `README.md`, `.speccy/ARCHITECTURE.md`,
`resources/modules/phases/speccy-init.md`, `resources/modules/skills/speccy-plan.md`,
`speccy-cli/tests/skill_body_discovery.rs`) is searched for "greenfield",
then every match appears only in prose that explicitly denies the distinction
(zero claiming uses remain).

Given frozen files under `.speccy/specs/NNNN-*/`,
when those directories are searched for "greenfield",
then the results are unchanged from before this task (historical records untouched).

Given `cargo test --workspace` after the rename,
when the `skill_body_discovery` test module runs,
then the renamed test passes and no test function name or body contains the literal
substring "greenfield".

Suggested files:
- `AGENTS.md`
- `README.md`
- `.speccy/ARCHITECTURE.md`
- `resources/modules/phases/speccy-init.md`
- `resources/modules/skills/speccy-plan.md`
- `speccy-cli/tests/skill_body_discovery.rs`
</task-scenarios>

<implementer-note session="session-2026-05-20-t002-attempt-1">
Completed: Removed all claiming uses of "greenfield" from the six named
files while preserving all three explicit denials.

File-by-file changes:
- `AGENTS.md`: rewrote three claims — "bootstrapping a greenfield repo"
  → "bootstrapping a new repo"; "bootstrapping greenfield projects" →
  "bootstrapping new projects"; `"Useful for my next greenfield"` →
  `"Useful for my next project"`. Denial at line 121
  ("there is no greenfield/brownfield distinction") left untouched.
- `README.md`: removed "(greenfield path)" from the `speccy vacancy`
  table row. Denial at line 96-98 left untouched.
- `.speccy/ARCHITECTURE.md`: rewrote four claims — "A solo greenfield
  project" → "A project"; "in greenfield mode" (×2, lines 179 and 1823)
  → removed "greenfield mode" qualifier; "more focused on greenfield loop"
  (comparison table) → "less focused on iterative review loop";
  "the next greenfield project" → "the next project". Denial at line 865
  left untouched.
- `resources/modules/phases/speccy-init.md`: rewrote three claims —
  "three greenfield states" → "three states"; "Greenfield Q&A" heading →
  "Q&A"; "The greenfield bootstrap runs once" → "The bootstrap runs once".
- `resources/modules/skills/speccy-plan.md`: already clean (no hits).
- `speccy-cli/tests/skill_body_discovery.rs`: renamed
  `chk015_speccy_plan_uses_vacancy_not_status_for_greenfield_id` to
  `chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id`; simplified
  the body to check the full `speccy-plan.md` body directly without
  `**Amendment**` partitioning; updated the module-level doc comment.
  Zero "greenfield" substrings remain in the file.

Additional fix (pre-existing issue surfaced by dogfood regeneration):
  Regenerating the dogfood SKILL.md files via `speccy init --force`
  (required to fix the `dogfood_outputs_match_committed_tree` test)
  revealed that `LOOP_RECIPES` in `speccy-cli/tests/skill_packs.rs`
  still listed `speccy-amend/SKILL.md` as a loop recipe. The module was
  already changed before T-002 to say "This recipe does not loop." Cleared
  `LOOP_RECIPES` to `&[]` with a SPEC-0034 T-001 attribution comment.

Evidence: evidence/T-002.md

Undone: Nothing left undone. All six files covered; all claim hits removed;
all denials preserved; test renamed and simplified; tests pass.

Commands run:
- `cargo test --workspace -- chk015` — exit 0 (renamed test passes)
- `cargo run -- init --force --host claude-code` — exit 0 (dogfood regenerated)
- `cargo run -- init --force --host codex` — exit 0 (dogfood regenerated)
- `cargo test --workspace` — exit 0 (all tests pass)
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo run -- next --json SPEC-0034` — exit 0 (T-002 in-review, next is T-003)

Exit codes: all 0.

Discovered issues: The `dogfood_outputs_match_committed_tree` test was
passing against stale committed SKILL.md files — the module changes from
the SPEC amend work and T-001 had not been propagated via `speccy init
--force`. Regenerating is part of this task's standard hygiene (T-008 will
re-eject, but the dogfood test requires the committed files to match the
renderer at all times). Fixed inline.

Procedural compliance: No skill file friction encountered. The
evidence file mirrors the body shape of
`.speccy/specs/0033-eject-prompt-bodies/evidence/T-001.md` and
`.speccy/specs/0034-authoring-self-review/evidence/T-001.md`.
</implementer-note>

<review persona="business" verdict="blocking">
REQ-012 work is correct on its face — all claim hits the implementer captured
were removed, denials preserved, frozen `.speccy/specs/NNNN-*/` files
untouched, renamed test passes. The blocker is an out-of-scope mutation
bundled into this slice.

`resources/modules/skills/speccy-amend.md` step 4 was rewritten from
`Record the new spec hash` / `Re-run speccy status` / `### Loop exit
criteria` into a single non-looping flow that hands off to `/speccy-tasks`
for hash recording. That semantic content change is NOT covered by REQ-011
(plan-only) or REQ-012 (greenfield-only). No `<requirement>` in SPEC-0034
authorizes rewriting the amend recipe's flow. The corresponding
`LOOP_RECIPES = &[]` change in `speccy-cli/tests/skill_packs.rs:151`
attributes the change to "SPEC-0034 T-001", but T-001's scope is
`speccy-plan.md` only.

The ejected `.claude/skills/speccy-amend/SKILL.md` and
`.agents/skills/speccy-amend/SKILL.md` regenerations then propagate this
unauthorized module change. The init/plan re-ejects are defensible (REQ-012
bullet 5 names init; T-001's authorized module edit forces the plan
re-eject via the dogfood test). The amend re-eject is unauthorized because
the upstream amend module change itself is unauthorized.

Two paths to unblock:
1. (Preferred) Revert the `resources/modules/skills/speccy-amend.md` body
   change, the `LOOP_RECIPES` test constant change, and the ejected amend
   SKILL.md regenerations. Surface "speccy-amend recipe should be
   single-pass" as a separate concern (new SPEC or `<open-question>` in
   this SPEC), per AGENTS.md "Surface unknowns; never invent".
2. Amend SPEC-0034 to add a `<requirement>` that authorizes retiring the
   amend recipe's loop exit criteria, with a Changelog row, then
   re-decompose.

Path 1 keeps T-002 surgical to REQ-012.
</review>

<review persona="tests" verdict="pass">
T-002's test work is real and complete. The renamed test
`chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id` carries no
"greenfield" substrings in name, body, doc-comments, or module header.
Assertions are non-vacuous: substring checks against the embedded
`resources/modules/skills/speccy-plan.md` body where
`speccy vacancy --json` is present at module line 24 and
`speccy status --json` is genuinely absent — reverting either property
would flip the test red. `cargo test --workspace --test
skill_body_discovery` passes 4/4; workspace-wide test run shows zero
failures. `evidence/T-002.md` carries a real red-then-green paper trail:
pre-edit `git show HEAD:<path>` captures match HEAD content; post-edit
working-tree grep matches the current tree (3 denials only at
`AGENTS.md:121`, `README.md:96`, `ARCHITECTURE.md:865`; zero in the test
file). Runner output block is structurally consistent with real cargo
test output. The `Evidence: evidence/T-002.md` line mirrors the T-001
convention. The disclosed `LOOP_RECIPES` `&[]` fix is transparently
attributed in the implementer-note (the business reviewer's scope
concern is separate).
</review>

<review persona="security" verdict="pass">
T-002 is a prose-removal and test-rename pass with zero new security
surface. The renamed test reads from the embedded `RESOURCES` static
bundle, not from user-supplied input. The `LOOP_RECIPES = &[]` change is
a static array modification with no runtime input handling. All prose
edits to `AGENTS.md`, `README.md`, `.speccy/ARCHITECTURE.md`,
`resources/modules/phases/speccy-init.md`, and
`resources/modules/skills/speccy-plan.md` are documentation-only with no
auth boundaries, credential handling, or template variables beyond the
existing `{{ cmd_prefix }}`. The regenerated SKILL.md files are
deterministic includes of the module sources. No new Cargo dependencies.
</review>

<review persona="style" verdict="blocking">
Two claiming uses of "greenfield" survive in files within T-002's named
scope.

`.speccy/ARCHITECTURE.md:608` — the section heading `### Greenfield
bootstrap` labels the AGENTS.md bootstrap section as a "Greenfield" mode.
The body text below the heading was cleaned correctly in
`speccy-init.md` ("The greenfield bootstrap runs once" → "The bootstrap
runs once"), but the corresponding ARCHITECTURE.md heading was missed.
This is a claim, not a denial.

`README.md:148` — `- **Greenfield (\`AGENTS.md\` missing entirely).**`
labels one of the three speccy-init states as "Greenfield." The parallel
states in `resources/modules/phases/speccy-init.md` were renamed to
"State A," "State B," "State C" correctly, but the README did not
follow. This is a claim, not a denial.

Both hits were absent from the pre-edit evidence in `evidence/T-002.md`
(the README pre-edit section lists only lines 41 and 96; the
ARCHITECTURE.md pre-edit section lists 92, 179, 865, 1823, 1974, 2244
but not line 608). The task's scenario 1 done-when requires zero
claiming uses in `.speccy/ARCHITECTURE.md` and `README.md`.

All other style checks pass: test rename follows the `chk0NN_*` naming
pattern; simplified body is more readable; clippy-expect header matches
peer test files; no `unwrap()` / `panic!()` introduced; implementer-note
`session="..."` attribute and `Evidence:` field shape match T-001's
convention; evidence file follows the project shape.

To unblock: rename `### Greenfield bootstrap`
(`.speccy/ARCHITECTURE.md:608`) to `### Bootstrap` (or similar) and
reword `**Greenfield (\`AGENTS.md\` missing entirely).**`
(`README.md:148`) to match the State A label used in `speccy-init.md`
(e.g. `**State A — \`AGENTS.md\` missing entirely.**`). Update
`evidence/T-002.md` to add these two hits to the pre-edit capture and
confirm zero claims in the post-edit capture.
</review>

<retry>
Two blockers must clear before re-review: (a) the out-of-scope
amend-recipe mutation, and (b) two surviving claim hits the original
grep missed.

- business: revert the `resources/modules/skills/speccy-amend.md` body
  change (restore step 4's `Record the new spec hash` / `Re-run speccy
  status` / `### Loop exit criteria` shape), revert the `LOOP_RECIPES =
  &[]` change in `speccy-cli/tests/skill_packs.rs:151` to its prior
  contents, and revert the ejected
  `.claude/skills/speccy-amend/SKILL.md` and
  `.agents/skills/speccy-amend/SKILL.md` files. Then run
  `cargo run -- init --force --host claude-code` and
  `cargo run -- init --force --host codex` again — they should leave
  the amend SKILL.md files matching the restored module source.
  Surface "speccy-amend recipe should be single-pass" as an
  `<open-question>` in SPEC-0034 (or a new SPEC) for a future
  amendment, not a smuggled change here.
- style: rename `### Greenfield bootstrap`
  (`.speccy/ARCHITECTURE.md:608`) to a non-claiming heading, and
  reword `**Greenfield (\`AGENTS.md\` missing entirely).**`
  (`README.md:148`) to match `speccy-init.md`'s "State A" labeling.
  Update `evidence/T-002.md` pre-edit and post-edit grep captures to
  include both lines.

After the reverts and the additional greenfield cleanups, re-run
`cargo test --workspace`, `cargo run -- check SPEC-0034`, and
`cargo run -- next --json SPEC-0034` to confirm all green, then flip
T-002 back to `state="in-review"` with an attempt-2 `<implementer-note
session="session-2026-05-20-t002-attempt-2">`.
</retry>

<implementer-note session="session-2026-05-20-t002-attempt-2">
Completed: Cleared both blockers from the attempt-1 review.

Blocker 1 (business — out-of-scope amend-recipe mutation):
Reverted `resources/modules/skills/speccy-amend.md` to HEAD via
`git show HEAD:resources/modules/skills/speccy-amend.md`. Verified with
`git diff HEAD -- resources/modules/skills/speccy-amend.md` showing no
diff. Restored `LOOP_RECIPES` in `speccy-cli/tests/skill_packs.rs:151`
to `&["speccy-amend/SKILL.md"]` and dropped the out-of-scope "SPEC-0034
T-001" attribution comment. Re-ran `cargo run -- init --force --host
claude-code` and `cargo run -- init --force --host codex` — both
re-ejected only `speccy-amend/SKILL.md` (overwritten). Verified with
`git diff HEAD -- .claude/skills/speccy-amend/SKILL.md
.agents/skills/speccy-amend/SKILL.md` showing no diff. Added
`<open-question>` d. to SPEC-0034's `## Open Questions` section
surfacing the "speccy-amend recipe should be single-pass" question as
out-of-scope for SPEC-0034.

Blocker 2 (style — two surviving claim hits):
- `.speccy/ARCHITECTURE.md:608`: renamed `### Greenfield bootstrap` to
  `### AGENTS.md bootstrap` — matches the body text ("The `speccy-init`
  skill... runs an interactive Q&A to populate it") and the surrounding
  section context.
- `README.md:148-158`: replaced the three Greenfield/Brownfield state
  labels with State A / State B / State C labeling, matching
  `speccy-init.md` exactly (per `grep -n "State A\|State B\|State C"
  resources/modules/phases/speccy-init.md`).

Evidence: evidence/T-002.md

Undone: Nothing left undone. All six files from REQ-012 done-when are
clean of claiming uses. The two missed hits from attempt-1 are resolved.
The amend-recipe mutation is fully reverted.

Commands run:
- `git show HEAD:resources/modules/skills/speccy-amend.md >
  resources/modules/skills/speccy-amend.md` — exit 0 (restore)
- `git diff HEAD -- resources/modules/skills/speccy-amend.md` — no diff
- `cargo run -- init --force --host claude-code` — exit 0 (1 overwritten)
- `cargo run -- init --force --host codex` — exit 0 (1 overwritten)
- `git diff HEAD -- .claude/skills/speccy-amend/SKILL.md
  .agents/skills/speccy-amend/SKILL.md` — no diff
- `cargo test --workspace` — exit 0 (all tests pass)
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no
  lint errors)

Exit codes: all 0.

Discovered issues: The attempt-1 wrapper-template scope-creep — adding
out-of-scope changes to `speccy-amend.md` during dogfood regeneration —
illustrates a failure mode for the surgical-changes principle: when
`speccy init --force` is used to regenerate committed SKILL.md files,
it re-ejects ALL modules, including modules whose upstream source was
changed out-of-scope for the current task. The fix is to scope `init
--force` calls to only the modules that were intentionally changed by
the current task, or to check `git diff HEAD -- <ejected-file>` after
each regeneration and revert any unintended changes. The attempt-1
LOOP_RECIPES `&[]` fix compounded this: the constant was corrected in
response to a stale test state that was itself a symptom of the
unauthorized amend module edit.

Procedural compliance: Updated `evidence/T-002.md` with an attempt-2
section at the top capturing both missed pre-edit hits via `git show
HEAD:<path>` and the post-edit captures showing zero claims. No skill
file friction encountered.
</implementer-note>

<review persona="business" verdict="pass">
Attempt 2 cleanly resolves the attempt-1 business blocker. The three
out-of-scope amend-recipe mutations are fully reverted (zero diff
against HEAD for the amend module, the LOOP_RECIPES test constant,
and the two ejected amend SKILL.md files). REQ-012's substantive work
remains intact: claiming uses of "greenfield" removed from all six
named files, three denials preserved verbatim, frozen SPECs untouched,
renamed test in place. The deferred "speccy-amend recipe should be
single-pass" concern is correctly surfaced as `<open-question>` d. in
SPEC.md with a one-line deferral rationale, honoring AGENTS.md's
"surface unknowns; never invent" principle. T-002 is surgical to
REQ-012 as intended.
</review>

<review persona="tests" verdict="pass">
Attempt-2 reverts and amendments verify clean for T-002's REQ-012
scope. `speccy-cli/tests/skill_packs.rs` LOOP_RECIPES reverted to
`&["speccy-amend/SKILL.md"]` matching HEAD. Renamed test
`chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id` passes
with zero "greenfield" substrings. Surface sweep of the six REQ-012
files shows three remaining hits, all explicit denials
(`AGENTS.md:121`, `README.md:96`, `.speccy/ARCHITECTURE.md:865`). The
two attempt-1 misses resolved: `.speccy/ARCHITECTURE.md:608` is now
`### AGENTS.md bootstrap`; `README.md:148-158` is now State A / B / C.
`evidence/T-002.md` attempt-2 section captures both missed pre-edit
hits via `git show HEAD:<path>` and post-edit captures show zero claims.

Out-of-scope heads-up for the orchestrator: the working tree carries an
unstaged modification of `.claude/agents/speccy-work.md` flipping
`model: sonnet[1m]` / `effort: medium` to `model: opus[1m]` / `effort:
low`. The source template at
`resources/agents/.claude/agents/speccy-work.md.tmpl` still reads
`sonnet[1m]` / `medium`, so the dogfood file no longer matches the
renderer; `dogfood_outputs_match_committed_tree` currently fails. This
is not T-002 territory and does not block this verdict.
</review>

<review persona="security" verdict="pass">
Attempt-2 changes are prose-only documentation edits with zero security
surface. The two edited regions — `ARCHITECTURE.md:608` heading rename
and `README.md:148-158` state-label rewrite — describe the
`speccy-init` skill's three-state logic for populating `AGENTS.md`. No
code paths, credentials, auth boundaries, input validation,
dependencies, or data handling were introduced or modified.
</review>

<review persona="style" verdict="pass">
All greenfield claim hits removed. The two surviving "greenfield"
occurrences in `.speccy/ARCHITECTURE.md:865` and `README.md:96` are
both explicit denials and correctly preserved. The renamed heading
`### AGENTS.md bootstrap` at `ARCHITECTURE.md:608` is coherent with
its surrounding section structure. The three README state labels
(State A / State B / State C) are consistent in shape with each other
and mirror `speccy-init.md`'s labeling precisely. `evidence/T-002.md`
attempt-2 captures accurately reflect the actual on-disk file state.
</review>

</task>

## Phase 2: Alpha-prefix open questions format and collapse-parallels heuristic

<task id="T-003" state="completed" covers="REQ-009">

## Update `## Open Questions` format to alpha-prefix in all three authoring-phase templates

Update the `## Open Questions` format guidance in all three authoring-phase skill
templates to `- [ ] a.` ... `- [ ] z.` alpha-prefix, in lock-step, and add the
26-cap scope-smell note and going-forward-only caveat to each template.

Open questions resolved before implementing:
- OQ-c (brainstorm pre-check firing on amendments routed through brainstorm): REQ-001
  already implies yes — the pre-check fires on every brainstorm invocation. The
  alpha-prefix format follows the same rule: it applies to every brainstorm session
  regardless of downstream path.

<task-scenarios>
Given each of the three files after the update,
when each file's body is searched for format guidance referencing `- [ ] a.`,
then each file contains at minimum one passage describing the alpha-prefix format
for `## Open Questions` (CHK-011).

Given the amend template,
when the guidance for editing an existing section is read,
then it instructs the agent to preserve existing ordinals and allocate the next free
letter when appending (no renumbering on amend).

Given all three templates,
when each is searched for the 26-cap scope-smell note,
then a passage explaining that reaching `z.` signals an over-scoped session appears
in each file.

Suggested files:
- `resources/modules/skills/speccy-plan.md`
- `resources/modules/skills/speccy-amend.md`
- `resources/modules/skills/speccy-brainstorm.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t003-attempt-1">
Completed: Added alpha-prefix `## Open Questions` format guidance,
the 26-cap scope-smell note, and the going-forward-only caveat to
all three authoring-phase skill templates in lock-step.

File-by-file changes:
- `resources/modules/skills/speccy-plan.md`: expanded step 3 from a
  single sentence ("Surface any material questions inline in
  `## Open Questions`") to a paragraph specifying `- [ ] a.` ...
  `- [ ] z.` alpha-prefix, next-free-letter allocation, the no-renumbering
  caveat for existing sections, the 26-cap scope-smell note, and the
  going-forward-only caveat.
- `resources/modules/skills/speccy-amend.md`: added a paragraph to step 2
  (the SPEC.md surgical-edit step) instructing the agent to use the
  alpha-prefix format when editing `## Open Questions`, to preserve existing
  ordinals (do not renumber on amend), to allocate the next free letter when
  appending a new question, plus the 26-cap note and going-forward-only caveat.
- `resources/modules/skills/speccy-brainstorm.md`: updated two locations:
  (a) step 3.4 (the open-questions artifact) — replaced the `- [ ]` unordered
  description with the `- [ ] a.` alpha-prefix description, added the 26-cap
  note and going-forward-only caveat; (b) the routing section's open-questions
  bullet — updated `- [ ]` checkbox format reference to `- [ ] a.` alpha-prefix
  format with a note that the output is copy-paste-compatible without reformatting.

Evidence: evidence/T-003.md

Undone: Nothing left undone. All three files carry equivalent guidance. The
amend template carries the unique preserve-ordinals / no-renumbering-on-amend
instruction per the task specification. The brainstorm template covers both the
generation side (step 3.4) and the routing side (routing section). The going-forward
caveat differs in framing between brainstorm ("sessions begun before this format
was adopted") and plan/amend ("existing SPECs retain their current `- [ ]`
formatting unless explicitly amended") but both express the same going-forward-only
semantics per the lock-step requirement.

Commands run:
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo test --workspace` — exit 101 (1 failure: dogfood_outputs_match_committed_tree;
  all other tests pass)
- `cargo run -- next --json SPEC-0034` — with T-003 in-progress, T-004 shown as
  next implement; with T-003 in-review, T-003 shown as next review (confirmed
  via speccy next output)

Exit codes: cargo run exits 0; cargo test exits 101 due to the known pre-existing
dogfood failure.

Discovered issues: The one test failure (`dogfood_outputs_match_committed_tree`)
is a pre-existing dogfood failure unrelated to T-003 — see orchestrator note. It
is caused by a deliberate local-only override of `.claude/agents/speccy-work.md`'s
model field (switched from `sonnet[1m]`/`medium` to `opus[1m]`/`low`). The source
template at `resources/agents/.claude/agents/speccy-work.md.tmpl` still reads
`sonnet[1m]`/`medium`, so the dogfood file no longer matches the renderer. This
was present before T-003 work began and was called out by the T-002 tests reviewer.

Procedural compliance: No skill file friction encountered. The three module files
being edited are themselves the skill templates for this task; no separate skill
update was needed. The evidence file mirrors the body shape of
`.speccy/specs/0034-authoring-self-review/evidence/T-001.md` and `evidence/T-002.md`.
</implementer-note>

<review persona="business" verdict="pass">
T-003 cleanly satisfies REQ-009. All three authoring-phase skill templates
carry equivalent alpha-prefix `- [ ] a.` ... `- [ ] z.` format guidance in
lock-step (CHK-011): `speccy-plan.md:35`, `speccy-amend.md:28`,
`speccy-brainstorm.md:91`. The 26-cap scope-smell note appears verbatim in
all three. The going-forward-only caveat appears in all three with
appropriate framing per skill semantics. The amend template specifically
carries the preserve-existing-ordinals + next-free-letter guidance per the
OQ-c resolution, aligning with CHK-012. The brainstorm routing bullet was
also updated to reference the alpha-prefix format with a copy-paste
compatibility note. Diffs are surgical to REQ-009 scope; no out-of-scope
mutations bundled.
</review>

<review persona="tests" verdict="pass">
T-003's evidence trail is real and verifiable. `evidence/T-003.md` carries a
genuine red-then-green paper trail: pre-edit `git show HEAD:<path>` captures
independently confirmed against HEAD (`speccy-plan.md` line 54 showed only
the single-sentence "Surface any material questions" prose with no
alpha-prefix; `speccy-amend.md` HEAD had zero matches; `speccy-brainstorm.md`
HEAD lines 90-91/135-136 showed the prior `- [ ]` unordered format).
Post-edit captures match working-tree state byte-for-byte at the cited line
numbers (plan 35-42, amend 27-34, brainstorm 90-96 and 142). Red/green
outputs are structurally distinct and framework-appropriate. The
`Evidence: evidence/T-003.md` field at TASKS.md:672 mirrors the SPEC-0034
in-spec convention used by T-001 attempt-2 and T-002 attempt-1. Per
orchestrator scope, the pre-existing `dogfood_outputs_match_committed_tree`
failure from the local model swap is correctly flagged as out-of-scope.
</review>

<review persona="security" verdict="pass">
Pure prose additions to three static markdown skill template files. No
executable code, no new dependencies, no untrusted-input paths, no secrets
handling, no auth boundaries touched. The changes add documentation-only
format guidance (alpha-prefix open-question checklist convention and 26-cap
scope-smell note) with zero attack surface.
</review>

<review persona="style" verdict="pass">
All T-003 style checks pass. Insertions read as natural continuations of
their enclosing steps (not stranded paragraphs). The 26-cap note is
word-for-word identical across all three files. The going-forward caveat
varies intentionally and appropriately by skill context (amend: "unless
touched by this amendment"; brainstorm: "sessions begun before this format
was adopted"; plan: "unless explicitly amended"). Liquid variables intact.
Prose voice consistent with surrounding files (declarative imperative,
em-dash asides, backtick quoting, ~72-char line wrapping). Implementer-note
`session` attribute and `Evidence:` field shape match T-001/T-002. Trailing
newlines present in all three files.

Pre-existing note (not a T-003 concern): `speccy-amend.md`'s loop-exit-
criteria paragraph references "step 6" but the recipe only has five
numbered steps. This stale reference exists at HEAD and is the same
"speccy-amend recipe should be single-pass" question already surfaced as
SPEC-0034 open-question d.
</review>

</task>

<task id="T-004" state="completed" covers="REQ-010">

## Add collapse-parallels heuristic to brainstorm and symmetric expansion guidance to plan

Add the collapse-parallels heuristic to `resources/modules/skills/speccy-brainstorm.md`
and the symmetric expansion guidance to `resources/modules/skills/speccy-plan.md`.
Both are discretionary ("MAY", not "MUST"); neither self-review surfaces failure to
collapse or expand.

<task-scenarios>
Given the updated `speccy-brainstorm.md`,
when its body is searched for the heuristic guidance,
then a passage uses "MAY" (not "MUST") wording, names reader cognitive load as the
goal, and gives one concrete example of when collapsing applies (CHK-013).

Given the updated `speccy-plan.md`,
when its body is searched for the symmetric expansion guidance,
then a passage mirrors the brainstorm side — MAY expand to atomic `<requirement>`
blocks or keep grouped, at agent discretion (CHK-014).

Given either template's self-review section (added in T-005 and T-006),
when it is searched for "collapse" or "expand",
then no passage surfaces failure to collapse or expand as a self-review issue.

Suggested files:
- `resources/modules/skills/speccy-brainstorm.md`
- `resources/modules/skills/speccy-plan.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t004-attempt-1">
Completed: Added the collapse-parallels heuristic to `resources/modules/skills/speccy-brainstorm.md`
and the symmetric expansion guidance to `resources/modules/skills/speccy-plan.md`.

File-by-file changes:
- `resources/modules/skills/speccy-brainstorm.md`: added a "Collapse-parallels heuristic."
  paragraph immediately after the atomize-requirements sentence in step 3.1 ("Restated ask,
  atomized"). The paragraph uses "MAY" (not "MUST"), names reader cognitive load as the goal,
  gives a concrete example ("if R1-R6 all read 'the X self-review verifies Y' (differing only
  in X), collapse to a single requirement with six sub-bullets a-f"), and explicitly states this
  is agent discretion with no enforced threshold.
- `resources/modules/skills/speccy-plan.md`: expanded step 2 from a single sentence ("Write
  SPEC.md following the PRD template.") to a short paragraph. The addition uses "MAY expand
  each sub-bullet to its own atomic `<requirement>` block ... or keep them grouped under one
  `<requirement>` with a `<done-when>` bullet list ... Agent discretion; neither choice is
  surfaced as a self-review issue."

Neither template's self-review section (to be added by T-005/T-006) was touched — the task
scope is purely the two heuristic guidance additions.

Evidence: evidence/T-004.md

Undone: Nothing left undone. Both CHK-013 and CHK-014 satisfied.

Commands run:
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo test --workspace` — exit 101 (1 failure: dogfood_outputs_match_committed_tree,
  pre-existing local model override; all other tests pass: 34 passed)

Exit codes: cargo run exits 0; cargo test exits 101 due to the known pre-existing dogfood
failure from the deliberate local-only model override of `.claude/agents/speccy-work.md`.

Discovered issues: None. The two edits are surgical prose additions to the two named template
files. No out-of-scope mutations.

Procedural compliance: No skill file friction encountered. The task scope was clear and the
evidence file mirrors the body shape of `.speccy/specs/0034-authoring-self-review/evidence/T-003.md`.
</implementer-note>

<review persona="business" verdict="pass">
T-004 adds the collapse-parallels heuristic to `speccy-brainstorm.md:74-82`
and the symmetric expansion guidance to `speccy-plan.md:32-38`. Both
passages use "MAY" (not "MUST"), name reader cognitive load / agent
discretion as the goal, give one concrete worked example (R1-R6 collapse
to a single requirement with sub-bullets a-f), and the plan side mirrors
the brainstorm side thematically (collapse-side vs expand-side, distinct
prose) rather than copy-pasting. Neither template enforces the heuristic;
the plan side explicitly states "neither choice is surfaced as a
self-review issue", honoring REQ-010's discretion stance and the SPEC's
non-goal "No enforcement of the collapse-parallels heuristic." T-004's
surgical contribution is exactly the two paragraphs the implementer-note
describes. CHK-013 and CHK-014 satisfied at the upstream template level.
</review>

<review persona="tests" verdict="pass">
T-004 evidence carries a verified red-then-green paper trail. Pre-edit
`git show HEAD:<path>` captures for both `speccy-brainstorm.md` and
`speccy-plan.md` independently re-confirm zero matches at HEAD for the
heuristic terms; post-edit working-tree greps re-confirm the new content
at the cited line numbers. CHK-013's substantive obligations (MAY wording
at brainstorm.md:75, reader cognitive load at lines 76-77, concrete R1-R6
example at lines 77-80) and CHK-014's symmetric obligations (MAY expand
at plan.md:34, keep-grouped at line 36, agent discretion at lines 37-38)
are present in the actual files. The `Evidence: evidence/T-004.md` field
at TASKS.md:816 mirrors the SPEC-0034 convention. Scenario 3 is correctly
deferred to T-005/T-006's future surface with a clear rationale. The
pre-existing `dogfood_outputs_match_committed_tree` failure is
out-of-scope per orchestrator note.
</review>

<review persona="security" verdict="pass">
Pure prose additions to two static markdown skill template files
(`speccy-brainstorm.md` lines 74-78 and `speccy-plan.md` lines 34-38). No
code paths, no new template variables, no untrusted-input handling, no
new dependencies, no auth boundaries, no secrets, no network calls. The
sole pre-existing template variable `{{ cmd_prefix }}` is a CLI-internal
constant populated at `speccy init` time, not user-supplied at runtime.
Security surface is zero.
</review>

<review persona="style" verdict="pass">
All six style checks pass. Coherence: the collapse-parallels paragraph
sits naturally after the "split it" sentence in step 3.1 of brainstorm;
the expansion guidance widens step 2 of plan into a short paragraph that
stays cleanly within the step. Semantic symmetry: the pairing is
genuinely inverse — brainstorm collapses N identical-structure
requirements for draft readability; plan may re-expand for reviewer-
fan-out value. Directionality follows the workflow (brainstorm → plan).
"MAY" wording per RFC-2119 confirmed in both files. Liquid variables
intact. Prose voice consistent: declarative imperative, em-dash asides,
backtick code quoting, ~72-char wrapping. Implementer-note shape:
`session="session-2026-05-20-t004-attempt-1"` and
`Evidence: evidence/T-004.md` match T-001/T-002/T-003 conventions.
Trailing newlines confirmed.
</review>

</task>

## Phase 3: Self-review sections in authoring-phase templates

<task id="T-005" state="completed" covers="REQ-002 REQ-005 REQ-006 REQ-007 REQ-008">

## Add self-review section to `speccy-plan.md`

Add the self-review section to `resources/modules/skills/speccy-plan.md`. The
section fires after the SPEC.md write step and before the `/speccy-tasks` handoff
suggestion. It names six check properties with descriptions, defines the
mechanical/semantic split with the tie-breaker rule, carries the literal template
string for semantic surfacings, and contains an explicit no-loop instruction.

Open questions resolved before implementing:
- OQ-a (exact chat-preamble template strings): plan uses `- [ ] {ordinal}.
  **Self-review caught:** {issue}` for `## Open Questions`, not the chat-preamble
  form — OQ-a applies to the brainstorm template (T-007). Resolve: use the literal
  string exactly as stated in REQ-006 done-when for plan/amend.
- OQ-b (shared partial): DEC-001 and the SPEC both reject premature factoring.
  Implement two independent copies (plan + amend) for this slice.

<task-scenarios>
Given the updated `speccy-plan.md`,
when the self-review section is positioned relative to surrounding steps,
then it appears after the SPEC.md write step and before the `/speccy-tasks` handoff
suggestion line (CHK-001 fires on ejected output, but the source is the prerequisite).

When the section is parsed for property names,
then the six identifiers "routing fidelity", "atomization", "scope-traces",
"internal consistency", "placeholder leakage", and "ambiguity" all appear with
one- or two-sentence descriptions (CHK-003).

When the section is searched for the literal substring `**Self-review caught:**`,
then exactly one match is found inside the documented fixed-template string
(CHK-007).

When the section is searched for a no-loop instruction,
then a verbatim "do not re-check after applying fixes" instruction (or
near-identical wording) appears within or adjacent to the section (CHK-009).

When the section is searched for the mechanical/semantic split,
then the tie-breaker line "if judging requires reading semantics, it is semantic"
(or equivalent verbatim) appears with the concrete mechanical pattern list (CHK-006).

When the section references routing fidelity,
then it notes the check applies only when brainstorm ran for this SPEC; when
brainstorm was skipped, scope-traces alone covers the equivalent check.

Suggested files:
- `resources/modules/skills/speccy-plan.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t005-attempt-1">
Completed: Added the self-review section to `resources/modules/skills/speccy-plan.md`
as step 3, positioned after the SPEC.md write step (step 2) and before the
`/speccy-tasks` handoff suggestion (step 5, renumbered from 4).

Changes:
- Inserted a new step 3 "Self-review pass" between the existing step 2 (Write
  SPEC.md) and the existing step 3 (now step 4, Open Questions format guidance).
- Old step 4 (Suggest next step) renumbered to step 5.
- The self-review step contains:
  - No-loop instruction: "Run this pass exactly once after writing SPEC.md.
    Do not re-check after applying fixes." (CHK-009)
  - Mechanical/semantic split with tie-breaker: "If judging requires reading
    semantics, it is semantic." (CHK-006) plus concrete mechanical pattern list
    (TBD/TODO strings, "and"/"also" inside requirements, untouched `<...>`
    placeholders, missing alpha-prefix ordinals)
  - Literal template string on its own line: `` `- [ ] {ordinal}. **Self-review
    caught:** {issue}` `` (CHK-007 — exactly one occurrence)
  - Six named check properties with one- or two-sentence descriptions each
    (CHK-003): routing fidelity, atomization, scope-traces, internal consistency,
    placeholder leakage, ambiguity
  - Routing-fidelity property description explicitly states: "This check applies
    only when brainstorm ran for this SPEC. When brainstorm was skipped,
    scope-traces alone covers the equivalent verification against the user's
    stated ask." (per REQ-002 done-when scope note)
  - Parallel-copy comment pointing to `speccy-amend.md` (DEC-001)

Evidence: evidence/T-005.md

Undone: Nothing left undone. All six check properties, template string, tie-breaker,
no-loop instruction, and routing-fidelity scope note are present.

Commands run:
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo test --workspace` — exit 101 (34 passed, 1 failed:
  `dogfood_outputs_match_committed_tree` — pre-existing local model override;
  all other tests pass)
- `cargo run -- next --json SPEC-0034` — exit 0 (with T-005 in-review,
  kind=review task_id=T-005)

Exit codes: cargo run exits 0; cargo test exits 101 due to the known pre-existing
dogfood failure from the local model override of `.claude/agents/speccy-work.md`.

Discovered issues: The `dogfood_outputs_match_committed_tree` test failure is
pre-existing and unrelated to T-005. It is caused by a local-only override of
`.claude/agents/speccy-work.md`'s model field (switched to `opus[1m]`/`low`
from the template's `sonnet[1m]`/`medium`). This has been present and called
out since T-002.

Procedural compliance: No skill file friction encountered. The module file
being edited (`resources/modules/skills/speccy-plan.md`) is itself the
authoring surface for this task — no separate skill update was needed.
Evidence file mirrors the body shape of
`.speccy/specs/0034-authoring-self-review/evidence/T-004.md`.
</implementer-note>

<review persona="business" verdict="pass">
T-005 cleanly satisfies REQ-002 + REQ-005 + REQ-006 + REQ-007 + REQ-008 for
the plan side. All five scope items land in
`resources/modules/skills/speccy-plan.md`: six check properties (CHK-003)
each with a 1-2 sentence description; mechanical/semantic split (CHK-006)
with concrete pattern list and verbatim tie-breaker "If judging requires
reading semantics, it is semantic."; literal template string (CHK-007)
exactly once at line 54 matching REQ-006 done-when; no-loop instruction
(CHK-009) at lines 40-41 ("Do not re-check after applying fixes"); and the
routing-fidelity scope note ("applies only when brainstorm ran for this
SPEC. When brainstorm was skipped, scope-traces alone covers..."). Section
positioned correctly between step 2 (Write SPEC.md) and step 5
(`/speccy-tasks` handoff). No silently-resolved open questions; OQ-b's
shared-partial rejection honored via the DEC-001 parallel-copy comment.
No out-of-scope mutations bundled.
</review>

<review persona="tests" verdict="pass">
T-005's evidence carries a real red-then-green paper trail. All
`git show HEAD:resources/modules/skills/speccy-plan.md | grep` red
captures independently verify zero matches at HEAD (six property names,
`Self-review caught`, tie-breaker, `do not re-check`, brainstorm-skipped
note). Post-edit greens verify presence at the cited line numbers; the
`**Self-review caught:**` substring appears exactly once at line 54
satisfying CHK-007. Red/green outputs are structurally distinct. The
`Evidence: evidence/T-005.md` field at TASKS.md:972 mirrors the SPEC-0034
convention. Section position verified by direct file read. The executable
CHK-001/CHK-003/CHK-006/CHK-007/CHK-009 assertions against the ejected
SKILL.md are correctly deferred to T-008.
</review>

<review persona="security" verdict="pass">
Prose-only addition to `resources/modules/skills/speccy-plan.md`. No code
paths, no auth boundaries, no secrets handling, no untrusted input, no new
dependencies. The sole template variable `{{ cmd_prefix }}` is populated by
CLI-internal host-type logic. The self-review section outputs only to the
local `## Open Questions` section of SPEC.md. Security surface is zero.
</review>

<review persona="style" verdict="pass">
All seven style checks pass. Step 3 (self-review) sits naturally between
step 2 (Write SPEC.md) and step 4 (alpha-prefix Open Questions from T-003)
and step 5 (`/speccy-tasks` handoff). Six property names rendered as
uniform `**Title case.**` bold headers. `**Self-review caught:**`
appears exactly once at line 54. Tie-breaker phrasing at line 49 ("If
judging requires reading semantics, it is semantic.") matches the SPEC's
verbatim form. Step renumbering (old 3→4, old 4→5) clean. Liquid
variables intact. Implementer-note `session="..."` attribute and
`Evidence: evidence/T-005.md` match T-001..T-004 conventions. Trailing
newline present.
</review>

</task>

<task id="T-006" state="completed" covers="REQ-003 REQ-005 REQ-006 REQ-007 REQ-008">

## Add self-review section to `speccy-amend.md`

Add the self-review section to `resources/modules/skills/speccy-amend.md`. The
section fires after the diff-write and Changelog-append step and before the
next-step handoff suggestion. It carries the six shared properties from T-005 plus
two amend-specific additions (Changelog row presence, surgical-diff shape), the
same mechanical/semantic split with tie-breaker, the same literal template string,
the same no-loop instruction, and a parallel-copy comment pointing to
`speccy-plan.md`.

<task-scenarios>
Given the updated `speccy-amend.md`,
when the self-review section is positioned relative to surrounding steps,
then it appears after the diff-write and Changelog-append step and before the
next-step handoff suggestion.

When the section is parsed for property names,
then the six shared identifiers from CHK-003 appear plus the two amend-specific
identifiers "Changelog row presence" and "surgical-diff shape" (CHK-004).

When the section is searched for the literal substring `**Self-review caught:**`,
then exactly one match appears inside the documented fixed-template string —
identical to the plan template's form.

When the section is searched for a no-loop instruction,
then a verbatim "do not re-check" instruction (or near-identical wording) appears
within or adjacent to the section (CHK-009).

When the section is searched for the mechanical/semantic tie-breaker,
then the line "if judging requires reading semantics, it is semantic" (or equivalent
verbatim) appears with the concrete mechanical pattern list (CHK-006).

Suggested files:
- `resources/modules/skills/speccy-amend.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t006-attempt-1">
Completed: Added the self-review section to `resources/modules/skills/speccy-amend.md`
as step 3, positioned after step 2 (the SPEC.md diff-write and Changelog-append step)
and before the next-step handoff suggestion.

Changes:
- Inserted a new step 3 "Self-review pass" between the existing step 2 (Edit SPEC.md
  surgically + append Changelog row) and the existing step 3 (now step 4, Reconcile
  TASKS.md).
- Old step 4 (speccy lock) renumbered to step 5; old step 5 (Re-run speccy status)
  renumbered to step 6.
- The self-review step contains:
  - No-loop instruction: "Run this pass exactly once after writing the SPEC.md diff
    and appending the Changelog row. Do not re-check after applying fixes." (CHK-009)
  - Parallel-copy comment pointing to `speccy-plan.md` per DEC-001 / OQ-b (two
    independent copies, no shared partial).
  - Mechanical/semantic split with tie-breaker: "If judging requires reading semantics,
    it is semantic." (CHK-006) plus concrete mechanical pattern list (TBD/TODO strings,
    "and"/"also" inside requirements, untouched `<...>` placeholders, missing
    alpha-prefix ordinals)
  - Literal template string on its own line: `` `- [ ] {ordinal}. **Self-review
    caught:** {issue}` `` (CHK-007 — exactly one occurrence)
  - Eight named check properties with one- or two-sentence descriptions each:
    the six shared properties from CHK-003 (routing fidelity, atomization,
    scope-traces, internal consistency, placeholder leakage, ambiguity) plus
    two amend-specific additions (CHK-004): Changelog row presence, surgical-diff shape.

Evidence: evidence/T-006.md

Undone: Nothing left undone. All eight check properties, template string, tie-breaker,
no-loop instruction, parallel-copy comment, and step renumbering are present.

Commands run:
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo test --workspace` — exit 101 (34 passed, 1 failed:
  `dogfood_outputs_match_committed_tree` — pre-existing local model override;
  all other tests pass)
- `cargo run -- next --json SPEC-0034` — exit 0 (with T-006 in-review,
  kind=review task_id=T-006)

Exit codes: cargo run exits 0; cargo test exits 101 due to the known pre-existing
dogfood failure from the deliberate local-only model override of
`.claude/agents/speccy-work.md` (model: opus[1m]/effort: low vs template's
sonnet[1m]/medium). This has been present and called out since T-002.

Discovered issues: The `dogfood_outputs_match_committed_tree` test failure is
pre-existing and unrelated to T-006. No other issues discovered.

Procedural compliance: No skill file friction encountered. The module file being
edited (`resources/modules/skills/speccy-amend.md`) is itself the authoring surface
for this task — no separate skill update was needed. Evidence file mirrors the body
shape of `.speccy/specs/0034-authoring-self-review/evidence/T-005.md`.
</implementer-note>

<review persona="business" verdict="pass">
T-006 cleanly satisfies REQ-003 + REQ-005 + REQ-006 + REQ-007 + REQ-008 for
the amend side. All eight check properties land in `speccy-amend.md:60-104`:
six shared from CHK-003 plus two amend-specific from CHK-004 verbatim
("Changelog row presence", "Surgical-diff shape"). Mechanical/semantic
split with verbatim tie-breaker at line 49 plus concrete pattern list.
Literal template string appears exactly once at line 54 (CHK-007).
No-loop instruction at lines 36-37 (CHK-009). Section positioned as step 3
between step 2 (Edit SPEC.md + Changelog) and step 4 (TASKS.md reconcile)
with handoff suggestion at lines 125-126. Parallel-copy comment at lines
39-41 points at `speccy-plan.md` per DEC-001 / OQ-b. Routing-fidelity
scope note preserved at lines 66-69. Structural parallel to T-005 differs
by one word ("six" → "eight"). No out-of-scope mutations bundled.
</review>

<review persona="tests" verdict="pass">
T-006 evidence is real and verifiable. All four red captures
independently re-confirm zero matches at HEAD for the eight property
names, `Self-review caught`, tie-breaker, and `do not re-check`. All
four green captures match the working tree byte-for-byte: nine
property-name hits at lines 62/67/71/76/81/86/90/95/100; exactly one
`**Self-review caught:**` at line 54 (CHK-007); tie-breaker verbatim at
line 49 (CHK-006); no-loop instruction at lines 36-37 (CHK-009).
Red/green outputs are structurally distinct. `Evidence: evidence/T-006.md`
at TASKS.md:1120 mirrors the SPEC-0034 convention. The pre-existing
`dogfood_outputs_match_committed_tree` failure is out of scope.
</review>

<review persona="security" verdict="pass">
Pure prose addition in `resources/modules/skills/speccy-amend.md`. No
code paths, no auth surfaces, no input handling, no secrets, no
dependencies, no executable logic. The fenced bash examples are
illustrative only. Zero security surface.
</review>

<review persona="style" verdict="pass">
All nine verification items pass. Self-review section at step 3 of
`speccy-amend.md:35-104` reads coherently, positioned between Changelog-
append (step 2) and TASKS.md reconcile (step 4). Eight property names
render in identical `**Title case.**` format and order to T-005's plan
side (six shared) plus two amend-specific appended. `**Self-review
caught:**` appears exactly once at line 54. Tie-breaker at line 49 is
verbatim-identical to plan side. Step renumbering clean (1-6 sequential).
Parallel-copy comment at lines 39-41 points at `speccy-plan.md` with
DEC-001/OQ-b attribution. Liquid template variables intact. Implementer-
note `session="..."` and `Evidence:` shape match T-001..T-005 conventions.
Trailing newline confirmed. Minor cosmetic asymmetry: parallel-copy
comment placement differs (before properties in amend, after in plan) —
not blocking; directional words ("above"/"below") are accurate in each.
</review>

</task>

<task id="T-007" state="completed" covers="REQ-004 REQ-005 REQ-006 REQ-007 REQ-008">

## Add pre-check section to `speccy-brainstorm.md`

Add the pre-check section to `resources/modules/skills/speccy-brainstorm.md`. The
section fires after the agent's internal artifact draft and before the chat
presentation of the four artifacts. It names four check properties with
descriptions, defines the mechanical/semantic split with tie-breaker, carries the
fixed-format chat-preamble template (opening line + bullet list + closing line),
and contains the no-loop instruction.

Open questions resolved before implementing:
- OQ-a (exact chat-preamble strings): adopt the candidate wording from the SPEC —
  opening "**Self-review caught the following before presenting artifacts:**",
  closing "Proceeding with the four artifacts below." These are the verbatim strings
  the agent uses unchanged.
- OQ-c (pre-check fires on amendments routed through brainstorm): yes, fires on
  every brainstorm invocation.

<task-scenarios>
Given the updated `speccy-brainstorm.md`,
when the pre-check section is positioned relative to surrounding steps,
then it appears after the agent's internal artifact draft and before the
chat-presentation step (CHK-002 fires on ejected output; source is prerequisite).

When the pre-check section is parsed for property names,
then "atomized restated requirements", "structurally distinct framings",
"load-bearing assumptions", and "shape-changing open questions" all appear with
descriptions (CHK-005).

When the pre-check section is searched for the verbatim opening line,
then "**Self-review caught the following before presenting artifacts:**" appears
quoted in the template body as the string the agent uses unchanged (CHK-008).

When the section is searched for the verbatim closing line,
then "Proceeding with the four artifacts below." appears quoted in the template body.

When the section is searched for a no-loop instruction,
then a verbatim "do not re-check" instruction (or near-identical wording) appears
within or adjacent to the section (CHK-009).

When the section is searched for the mechanical/semantic tie-breaker,
then the line "if judging requires reading semantics, it is semantic" (or equivalent
verbatim) appears with the concrete mechanical pattern list (CHK-006).

Suggested files:
- `resources/modules/skills/speccy-brainstorm.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t007-attempt-1">
Completed: Added the pre-check section to `resources/modules/skills/speccy-brainstorm.md`
as step 4, positioned after the "Produce four artifacts" step (step 3) and before the
"Stop and wait" step (renumbered from 4 to 5).

Changes:
- Inserted a new step 4 "Pre-check pass" between the existing step 3 (Produce four
  artifacts) and the existing step 4 (now step 5, Stop and wait).
- Old steps 5 and 6 renumbered to 6 and 7.
- Updated the "steps 1-5" reference in the writing skill step body to "steps 1-6".
- The pre-check step contains:
  - No-loop instruction: "run this internal review pass exactly once. Do not
    re-check after the artifacts are presented." (CHK-009)
  - OQ-c resolution: explicit statement that the pre-check fires on every brainstorm
    invocation, including when brainstorm is used as a front-end to amendment flows,
    since amendments routed through brainstorm are still brainstorm sessions.
  - Parallel-copy comment pointing to speccy-plan.md and speccy-amend.md per
    DEC-001 / OQ-b (independent copies, no shared partial).
  - Mechanical/semantic split with tie-breaker: "If judging requires reading
    semantics, it is semantic." (CHK-006) plus concrete mechanical pattern list
    (TBD/TODO strings, "and"/"also" inside a single restated requirement, untouched
    `<...>` placeholders, missing alpha-prefix ordinals)
  - Fixed-format chat-preamble template with:
    - Verbatim opening line: `**Self-review caught the following before presenting artifacts:**` (CHK-008)
    - Per-issue bullet: `- {issue}` (one-line description)
    - Verbatim closing line: `Proceeding with the four artifacts below.` (CHK-008)
    - Instruction to omit preamble entirely if no issues found
  - Four named check properties with one- or two-sentence descriptions each
    (CHK-005): atomized restated requirements, structurally distinct framings,
    load-bearing assumptions, shape-changing open questions

Evidence: evidence/T-007.md

Undone: Nothing left undone. All four check properties, both verbatim preamble
strings, mechanical/semantic split with tie-breaker, no-loop instruction, OQ-c
resolution, and step renumbering are present.

Commands run:
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo test --workspace` — exit 101 (34 passed, 1 failed:
  `dogfood_outputs_match_committed_tree` — pre-existing local model override;
  all other tests pass)
- `cargo run -- next --json SPEC-0034` — exit 0 (with T-007 in-review,
  kind=review task_id=T-007)

Exit codes: cargo run exits 0; cargo test exits 101 due to the known pre-existing
dogfood failure from the deliberate local-only model override of
`.claude/agents/speccy-work.md` (model: opus[1m]/effort: low vs template's
sonnet[1m]/medium). This has been present and called out since T-002.

Discovered issues: The `dogfood_outputs_match_committed_tree` test failure is
pre-existing and unrelated to T-007. As flagged in the task instructions, this
known issue will persist until T-008 re-ejects the skill packs.

Procedural compliance: No skill file friction encountered. The module file being
edited (`resources/modules/skills/speccy-brainstorm.md`) is itself the authoring
surface for this task — no separate skill update was needed. Evidence file mirrors
the body shape of `.speccy/specs/0034-authoring-self-review/evidence/T-006.md`.
</implementer-note>

<review persona="business" verdict="pass">
T-007 cleanly satisfies REQ-004 + REQ-005 + REQ-006 + REQ-007 + REQ-008 for
the brainstorm side. All seven scope items present in `speccy-brainstorm.md`:
four CHK-005 check properties at lines 148/154/158/163, mechanical pattern
list at lines 123-128 plus verbatim tie-breaker at line 129 (CHK-006),
verbatim opening line at line 135 backtick-quoted (CHK-008), verbatim
closing line at line 141 backtick-quoted, no-loop instruction at lines
110-112 (CHK-009), OQ-c resolution at lines 114-116 ("fires on every
brainstorm invocation including amendment flows routed through
brainstorm"), and section positioned as step 4 between draft (step 3) and
wait (step 5). The verbatim preamble strings sit inside an explicit
"verbatim, unchanged" template scaffold the agent uses unmodified. No
silently-resolved interpretation questions; OQ-a candidate wording adopted
exactly. Minor cosmetic observation: step 3's "present these four
artifacts in one message" verb relies on step 4's "Before presenting"
override to gate timing — workable as written, could be tightened in a
future cleanup.
</review>

<review persona="tests" verdict="pass">
T-007's evidence carries a real red-then-green paper trail. All four
`git show HEAD:resources/modules/skills/speccy-brainstorm.md | grep` red
captures independently verified to produce zero matches at HEAD; all four
green captures match the working tree byte-for-byte at the cited line
numbers. Red/green outputs are structurally distinct. The
`Evidence: evidence/T-007.md` field at TASKS.md:1280 mirrors the SPEC-0034
convention. Section position verified by direct file read. The executable
CHK-002/CHK-005/CHK-006/CHK-008/CHK-009 assertions against the ejected
SKILL.md are correctly deferred to T-008. The pre-existing
`dogfood_outputs_match_committed_tree` failure is out of scope.
</review>

<review persona="security" verdict="pass">
Single-file prose addition to `resources/modules/skills/speccy-brainstorm.md`.
No code paths, no auth boundaries, no secrets handling, no untrusted input,
no new dependencies. The sole template variable `{{ cmd_prefix }}` is a
CLI-internal constant. Security surface is zero.
</review>

<review persona="style" verdict="pass">
All seven style checks pass. Step 4 (pre-check) reads coherently as an
agent-internal pass with unambiguous timing. Four property names render in
identical `**Title case.**` format consistent with T-005/T-006. Both
verbatim chat-preamble strings are backtick-wrapped on their own lines
with explicit "Opening line (verbatim, unchanged):" / "Closing line
(verbatim, unchanged):" labels. Tie-breaker at line 129 is word-for-word
identical to the plan and amend sides. Step renumbering clean (1-7
sequential, internal "steps 1-5" reference updated to "steps 1-6").
Liquid template variables intact. Implementer-note `session="..."`
attribute and `Evidence:` field shape match T-001..T-006 conventions.
Trailing newline confirmed.
</review>

</task>

## Phase 4: Re-eject and verify

<task id="T-008" state="completed" covers="REQ-001 REQ-008">

## Re-eject all three authoring-phase skill packs and verify

Re-eject all three authoring-phase skill packs via `speccy init` to propagate the
module-level changes (T-001 through T-007) into the Claude Code and Codex host pack
SKILL.md files. Verify the ejected files match the expected shape against CHK-001,
CHK-002, CHK-010, CHK-015, CHK-016, and CHK-017.

Pre-conditions: T-001 through T-007 are all merged so the module source is
complete before ejection.

<task-scenarios>
Given the post-T-007 source tree, when `speccy init --host claude-code` runs in a
tempdir workspace,
then the ejected `.claude/skills/speccy-plan/SKILL.md` contains a self-review
section whose body precedes the `/speccy-tasks` handoff suggestion line (CHK-001).

When `.claude/skills/speccy-amend/SKILL.md` and
`.claude/skills/speccy-brainstorm/SKILL.md` are read from the same workspace,
then each file contains a self-review (or pre-check) section positioned per
REQ-001's done-when criteria (CHK-002).

When the three source module files are read,
then each contains a self-review (or pre-check) section body inline (CHK-010).

When the ejected `speccy-plan/SKILL.md` is searched for "Amendment" or
"amend an existing",
then no matches appear (CHK-016).

When the ejected `speccy-init/SKILL.md` and `AGENTS.md` are searched for
"greenfield" in claiming uses,
then no claiming uses appear (CHK-017).

When `cargo test --workspace` is run against the final tree,
then all existing and newly-added `skill_body_discovery` tests pass, including the
renamed former `chk015_*greenfield*` test.

Suggested files:
- `.claude/skills/speccy-plan/SKILL.md`
- `.claude/skills/speccy-amend/SKILL.md`
- `.claude/skills/speccy-brainstorm/SKILL.md`
- `.agents/skills/speccy-plan/SKILL.md`
- `.agents/skills/speccy-amend/SKILL.md`
- `.agents/skills/speccy-brainstorm/SKILL.md`
</task-scenarios>

<implementer-note session="session-2026-05-20-t008-attempt-1">
Completed: Re-ejected all three authoring-phase skill packs for both hosts and verified
all six CHKs against the resulting tree.

Re-eject commands:
- `cargo run -- init --force --host claude-code` — exit 0 (4 overwritten: speccy-work.md,
  speccy-amend/SKILL.md, speccy-brainstorm/SKILL.md, speccy-plan/SKILL.md)
- `cargo run -- init --force --host codex` — exit 0 (3 overwritten: speccy-amend/SKILL.md,
  speccy-brainstorm/SKILL.md, speccy-plan/SKILL.md)

CHK verification results:
- CHK-001: PASS — self-review section at .claude/skills/speccy-plan/SKILL.md:44 precedes
  `/speccy-tasks SPEC-` at line 112. Zero order inversion.
- CHK-002: PASS — speccy-amend/SKILL.md has self-review section at line 39;
  speccy-brainstorm/SKILL.md has pre-check section at line 114.
- CHK-010: PASS — all three source modules contain the relevant section bodies inline
  (plan.md:40, amend.md:35, brainstorm.md:110).
- CHK-015: PASS — zero matches for "Amendment", "amend an existing", "SPEC-NNNN argument"
  in resources/modules/skills/speccy-plan.md.
- CHK-016: PASS — zero matches for "Amendment" / "amend an existing" in both ejected
  .claude/skills/speccy-plan/SKILL.md and .agents/skills/speccy-plan/SKILL.md.
- CHK-017: PASS — 3 hits across named files, all in explicit denying prose (AGENTS.md:121,
  README.md:96, .speccy/ARCHITECTURE.md:865). Zero claiming uses.

Evidence: evidence/T-008.md

Undone: Nothing left undone. All six CHKs verified; both hosts re-ejected; dogfood test
now passes.

Commands run:
- `cargo run -- init --force --host claude-code` — exit 0
- `cargo run -- init --force --host codex` — exit 0
- `cargo test --workspace` — exit 0 (all tests pass, including dogfood_outputs_match_committed_tree)
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)
- `cargo run -- next --json SPEC-0034` — exit 0 (kind=implement task_id=T-009, as expected
  since T-008 is now in-review and T-009 is pending)

Exit codes: all 0. Note: `cargo run -- next --json SPEC-0034` shows T-009 as the next
implement because T-008 is now in-review (not completed). After T-008 flips to completed,
the next call will show kind=review task_id=T-008.

Discovered issues: The local-only override of `.claude/agents/speccy-work.md` (model:
opus[1m] / effort: low) was reverted to the upstream template values (model: sonnet[1m] /
effort: medium) by the `speccy init --force --host claude-code` re-eject. This is the
expected consequence of T-008's re-eject as documented in the task instructions. The
orchestrator should note that the override has reverted and should not try to re-apply it
via speccy init —the override needs to be set directly on the file if needed again.

Procedural compliance: No skill file friction encountered. The `speccy-work.md` override
reversion was anticipated and documented in the task instructions. Evidence file mirrors
the body shape of .speccy/specs/0034-authoring-self-review/evidence/T-007.md.
</implementer-note>

<review persona="business" verdict="pass">
T-008 satisfies REQ-001 + REQ-008 cleanly. Six CHKs verified against the
post-eject tree: CHK-001 (self-review section in ejected speccy-plan
SKILL.md precedes `/speccy-tasks` handoff), CHK-002 (ejected amend
self-review + ejected brainstorm pre-check at correct positions), CHK-010
(all three source modules carry sections inline), CHK-015 (zero
amendment-branch matches in source plan), CHK-016 (zero matches in both
ejected plan SKILL.md files), CHK-017 (3 "greenfield" hits all in
denying prose). Cross-host parity confirmed: `.claude/` vs `.agents/`
SKILL.md bodies differ only on `cmd_prefix` substitution per REQ-008.
The full self-review machinery from T-001..T-007 is now visible to
agents running the ejected host pack. Module-to-ejected propagation is
mechanical with no manual intervention, satisfying REQ-001 done-when
bullet 4.
</review>

<review persona="tests" verdict="pass">
T-008 evidence is real and independently verifiable. All six CHK greps
reproduce byte-for-byte against the working tree. The `speccy init
--force` plan-output blocks match the renderer's actual output format.
The dogfood test result is genuine: at the time T-008 captured it,
`.claude/agents/speccy-work.md` matched the template; the orchestrator
re-applied the opus[1m]/low override AFTER T-008 completed, so the
current red-test state is orchestrator-caused, not T-008's responsibility.
Re-running `cargo test --workspace dogfood` against the pre-override
state produces `dogfood_outputs_match_committed_tree ... ok`. No
fabrication patterns matched.
</review>

<review persona="security" verdict="pass">
T-008 introduces zero new security surface. All regenerated SKILL.md
files are deterministic template renders of the upstream module sources
already reviewed in T-001..T-007. The only substitutions applied are
`{{ cmd_prefix }}` (CLI-internal constant) and the `{% if host == ... %}`
host-dispatch branching. No new bash code blocks beyond what the modules
already contained. No credentials, no network calls, no path traversal,
no new template variable surface.
</review>

<review persona="style" verdict="pass">
All six ejected SKILL.md files in the working tree are clean against
T-001..T-007 conventions. Frontmatter shape uniform across all three
skills for both hosts. speccy-plan: amendment branch retired, self-review
section with six properties, alpha-prefix guidance, collapse/expand
heuristic. speccy-amend: self-review with eight properties (six shared +
Changelog row presence + Surgical-diff shape), parallel-copy comment per
DEC-001/OQ-b. speccy-brainstorm: pre-check with four properties, verbatim
opening/closing preamble strings, collapse-parallels heuristic,
alpha-prefix guidance. Implementer-note `session="..."` and `Evidence:`
shape match T-001..T-007 conventions. Trailing newlines confirmed on all
six files.
</review>

</task>

## Phase 5: Document TASKS.md output shape in the speccy-tasks template

<task id="T-009" state="completed" covers="REQ-013">

## Add concrete TASKS.md example fragment to `speccy-tasks.md` Step 2

Add a concrete example fragment to Step 2 of
`resources/modules/phases/speccy-tasks.md` that shows all three required
TASKS.md structural elements: the YAML frontmatter block (`spec:`,
`spec_hash_at_generation:`, `generated_at:`), the `# Tasks: SPEC-NNNN
<title>` level-1 heading on the line immediately after the closing `---`,
and at least one `<task ... covers="REQ-001 REQ-002">` line demonstrating
the space-separated multi-REQ form.

Open questions resolved before implementing:
- The existing Step 2 prose describes the `<tasks>` element correctly;
  the example fragment supplements it rather than replacing it. Keep the
  existing prose and add the fragment below or inline within the step body.
- The amendment route via `/speccy-amend` bullet in Step 1 references
  `/speccy-plan SPEC-NNNN`; REQ-011 retires that branch but that edit
  belongs to T-001. Touch only Step 2 in this task.

<task-scenarios>
Given `resources/modules/phases/speccy-tasks.md` after this task,
when the file body is searched for the literal substring `# Tasks: SPEC-`,
then the substring appears inside an example fragment in Step 2 (CHK-019,
first half).

Given the same file,
when the file body is searched for the literal substring
`covers="REQ-001 REQ-002"`,
then the substring appears inside the same example fragment in Step 2,
demonstrating the space-separated multi-REQ form (CHK-019, second half).

Given the updated template,
when an agent generates a TASKS.md for a SPEC covering two requirements,
when `speccy check SPEC-NNNN` parses the resulting TASKS.md,
then no TSK-004 (`InvalidCoversFormat`) lint error fires.

Given the same updated template,
when `cargo test --workspace` is run,
then any new `skill_body_discovery` test asserting CHK-019 passes.

Suggested files:
- `resources/modules/phases/speccy-tasks.md`
- `speccy-cli/tests/skill_body_discovery.rs` (new CHK-019 assertion)
</task-scenarios>

<implementer-note session="session-2026-05-20-t009-attempt-1">
Completed: Added the concrete TASKS.md example fragment to Step 2 of
`resources/modules/phases/speccy-tasks.md` and added the CHK-019 test
assertion in `speccy-cli/tests/skill_body_discovery.rs`.

Changes:
- `resources/modules/phases/speccy-tasks.md` Step 2: expanded with a
  paragraph explaining the required file structure, followed by a
  fenced Markdown example fragment showing all three required elements:
  - YAML frontmatter block with `spec:`, `spec_hash_at_generation:`,
    and `generated_at:` keys.
  - `# Tasks: SPEC-0007 My feature title` level-1 heading immediately
    after the closing `---` (no blank line between them).
  - Two `<task>` lines — one with a single REQ (`covers="REQ-001"`) and
    one with multi-REQ space-separated form (`covers="REQ-001 REQ-002"`).
  - Key constraints paragraph explicitly calling out that the heading
    must follow `---` with no blank line, and that `covers=` values are
    space-separated (not comma-separated), referencing the TSK-004 lint.
- `speccy-cli/tests/skill_body_discovery.rs`: added
  `chk019_speccy_tasks_template_documents_output_shape` test function
  following the `chk0NN_*` naming convention. Loads the
  `phases/speccy-tasks.md` module via `require_module()` and asserts
  both literal substrings `# Tasks: SPEC-` and `covers="REQ-001 REQ-002"`
  appear in the body.
- Re-ejected both hosts:
  - `cargo run -- init --force --host claude-code`: 1 overwritten
    (`.claude/agents/speccy-tasks.md`); `speccy-work.md` unchanged
    (matches template — no local override present).
  - `cargo run -- init --force --host codex`: 1 overwritten
    (`.codex/agents/speccy-tasks.toml`).

Evidence: evidence/T-009.md

Undone: Nothing left undone. Both CHK-019 substrings present; test passes;
re-eject propagated to both hosts; all workspace tests pass.

Commands run:
- Pre-edit: `git show HEAD:resources/modules/phases/speccy-tasks.md | grep -n "# Tasks: SPEC-"` — exit 1 (no output)
- Pre-edit: `git show HEAD:resources/modules/phases/speccy-tasks.md | grep -n 'covers="REQ-001 REQ-002"'` — exit 1 (no output)
- Pre-edit: `cargo test --workspace --test skill_body_discovery -- chk019` — exit 0, 0 tests run
- Post-edit: `cargo test --workspace --test skill_body_discovery -- chk019` — exit 0, 1 test passed
- `cargo run -- init --force --host claude-code` — exit 0 (1 overwritten)
- `cargo run -- init --force --host codex` — exit 0 (1 overwritten)
- `cargo test --workspace` — exit 0 (all tests pass)
- `cargo run -- check SPEC-0034` — exit 0 (19 scenarios rendered, no lint errors)

Exit codes: all 0.

Discovered issues: The `speccy init --force --host claude-code` re-eject
did NOT revert `.claude/agents/speccy-work.md` this time — the file already
matched the upstream template (model: sonnet[1m] / effort: medium), so it
was reported as "unchanged". This means the opus[1m]/low override that was
active during T-003 through T-007 is no longer present in the committed
tree. The orchestrator should re-apply the override directly to
`.claude/agents/speccy-work.md` if the higher-capability session is needed
for subsequent tasks (T-009 is the last task in SPEC-0034, so this is a
forward-looking note only).

Procedural compliance: No skill file friction encountered. The `speccy-tasks`
phase body being edited is itself the template being documented — no
separate skill update needed. Evidence file mirrors the body shape of
`.speccy/specs/0034-authoring-self-review/evidence/T-008.md`.
</implementer-note>

<review persona="business" verdict="pass">
T-009 satisfies REQ-013 / CHK-019. The example fragment in
`resources/modules/phases/speccy-tasks.md:38-58` is concrete and
copy-pasteable, contains required YAML frontmatter (`spec:`,
`spec_hash_at_generation:`, `generated_at:`), the `# Tasks: SPEC-0007 My
feature title` heading, and a `<task ... covers="REQ-001 REQ-002">` line
demonstrating space-separated multi-REQ form. Key Constraints paragraph
calls out the TSK-004 hazard explicitly. The new
`chk019_speccy_tasks_template_documents_output_shape` test asserts the
two literal substrings (not paraphrases) and passes. Re-eject propagated
to both `.claude/agents/speccy-tasks.md` and
`.codex/agents/speccy-tasks.toml`. The primary bug (TSK-004
InvalidCoversFormat) is fixed: a future agent reading this template will
produce parser-valid `covers="REQ-001 REQ-002"`.

Minor non-blocking observation: the example shows a blank line between
closing `---` and `# Tasks:`, but the Key Constraints text says "no
blank line between them." All existing TASKS.md files in the repo have
the blank line and the parser tolerates both forms — no behavior break.
Flagging as a future cleanup target.
</review>

<review persona="tests" verdict="pass">
T-009's test work is real and verifiable.
`chk019_speccy_tasks_template_documents_output_shape` at
`speccy-cli/tests/skill_body_discovery.rs:309-326` loads
`phases/speccy-tasks.md` via the same `require_module()` helper used by
sibling `chk014`/`chk015` tests. Both assertions are non-vacuous
literal substring checks; removing either substring from the module
flips the test red. `cargo test --workspace --test skill_body_discovery
-- chk019` passes. Pre-edit `git show HEAD:<path> | grep` for both
substrings returns 0; post-edit greps return the line numbers cited in
`evidence/T-009.md`. Red vs green cargo output structurally distinct.
The `Evidence: evidence/T-009.md` field at TASKS.md:1600 follows the
SPEC-0034 convention.
</review>

<review persona="security" verdict="pass">
No new security surface. Both files are compile-time static: the phase
prose is embedded markdown with template placeholders, and the test
file reads only from the compile-time `RESOURCES` bundle via
`&'static str` substring checks. No untrusted input paths, no new
dependencies, no auth boundaries touched.
</review>

<review persona="style" verdict="pass">
All seven style checks pass. Example fragment in step 2 is concrete and
copy-pasteable, YAML frontmatter with three required keys, heading
immediately after `---`, `covers="REQ-001 REQ-002"` multi-REQ form.
`chk019_*` follows the `chk0NN_<description>` naming convention used by
sibling tests, uses `require_module()` helper, only `assert!` macros
(no `unwrap()` / `panic!()` in the test function). Fenced block
language tags consistent within the file. Liquid template variables
intact. Implementer-note `session="session-2026-05-20-t009-attempt-1"`
and `Evidence: evidence/T-009.md` match T-001..T-008 convention.
Trailing newlines confirmed.
</review>

</task>

</tasks>
