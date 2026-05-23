---
spec: SPEC-0034
spec_hash_at_generation: 739815d752fcb1ebcb550eb15d4811e34a7ddee1ff10884cbb82808dd14b3660
generated_at: 2026-05-21T01:34:11Z
---

# Tasks: SPEC-0034 Self-review pass in authoring-phase skills (plan, amend, brainstorm)


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
(`AGENTS.md`, `README.md`, `docs/ARCHITECTURE.md`,
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
- `docs/ARCHITECTURE.md`
- `resources/modules/phases/speccy-init.md`
- `resources/modules/skills/speccy-plan.md`
- `speccy-cli/tests/skill_body_discovery.rs`
</task-scenarios>
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
</task>

