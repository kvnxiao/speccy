---
spec: SPEC-0034
outcome: satisfied
generated_at: 2026-05-20T22:00:00Z
---

# REPORT: SPEC-0034 Self-review pass in authoring-phase skills (plan, amend, brainstorm)

Three authoring-phase skills (`/speccy-plan`, `/speccy-amend`,
`/speccy-brainstorm`) now run a fixed-shape self-review (or pre-check)
pass at handoff. The pass lives inline in each skill's MiniJinja
template under `resources/modules/skills/` and ejects into both Claude
Code and Codex host packs via SPEC-0033's pipeline. Mechanical issues
(string-matchable: `TBD`/`TODO`, `<...>` placeholders, "and"/"also"
inside `<requirement>`, missing alpha-prefix ordinals) fix inline
without surfacing; semantic issues (LLM-judged) surface via literal
template strings — `- [ ] {ordinal}. **Self-review caught:** {issue}`
rows in `## Open Questions` for plan/amend, a verbatim opening/closing
chat preamble for brainstorm. Plan checks six properties (routing
fidelity, atomization, scope-traces, internal consistency, placeholder
leakage, ambiguity); amend adds two (Changelog row presence,
surgical-diff shape); brainstorm checks four artifact properties
(atomized restated requirements, structurally distinct framings,
load-bearing assumptions, shape-changing open questions). Two
supporting changes ride along: `## Open Questions` is alpha-prefix
`- [ ] a.` ... `- [ ] z.` lock-step across all three templates with a
26-cap scope-smell note, and a discretionary collapse-parallels
heuristic (`/speccy-brainstorm`) with symmetric expansion discretion
(`/speccy-plan`). Two cleanups land alongside: REQ-011 retires the
amendment branch from `/speccy-plan` (all amendment traffic routes
through `/speccy-amend`), and REQ-012 removes "greenfield" terminology
from live workflow surfaces (preserving only prose that explicitly
denies the greenfield/brownfield distinction). One amendment-time
addition (REQ-013) documents the TASKS.md output shape inside the
`speccy-tasks` phase template with a concrete frontmatter + heading +
`covers="REQ-001 REQ-002"` example fragment, fixing the TSK-004
parse-error pattern that surfaced during this SPEC's own
implementation loop.

<report spec="SPEC-0034">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
All three authoring-phase skill templates carry a self-review (or
pre-check) section at the handoff position.
`resources/modules/skills/speccy-plan.md` step 3 sits between the
SPEC.md write (step 2) and the `/speccy-tasks` handoff suggestion
(step 5). `resources/modules/skills/speccy-amend.md` step 3 sits
between the diff-write + Changelog-append (step 2) and the next-step
handoff. `resources/modules/skills/speccy-brainstorm.md` step 4 sits
between the four-artifact internal draft (step 3) and the chat
presentation (step 5). T-008 re-ejected all three to both Claude Code
(`.claude/skills/speccy-<phase>/SKILL.md`) and Codex
(`.agents/skills/speccy-<phase>/SKILL.md`) host packs; the dogfood
`dogfood_outputs_match_committed_tree` test passes against the
committed tree.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
The plan self-review section names six check properties verbatim:
routing fidelity, atomization, scope-traces, internal consistency,
placeholder leakage, ambiguity. Each carries a one- or two-sentence
description defining what passing looks like. The routing-fidelity
property explicitly documents its conditional scope: "applies only
when brainstorm ran for this SPEC. When brainstorm was skipped,
scope-traces alone covers the equivalent verification against the
user's stated ask." Verified in T-005 against
`resources/modules/skills/speccy-plan.md` and re-verified by T-008
against the ejected `.claude/skills/speccy-plan/SKILL.md`.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
The amend self-review section names the six shared properties from
REQ-002 plus two amend-specific additions: "Changelog row presence"
and "Surgical-diff shape". Each amend-specific property carries a
description parallel in style to the six shared properties. The
amend template explicitly notes the diff-shape check fires only in
the amend surface (`/speccy-plan` writes new SPEC.md content rather
than a diff). Verified in T-006 against
`resources/modules/skills/speccy-amend.md` and re-verified by T-008
against the ejected `.claude/skills/speccy-amend/SKILL.md`.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005">
The brainstorm pre-check section names exactly four check properties:
atomized restated requirements, structurally distinct framings,
load-bearing assumptions, shape-changing open questions. The
template defines "structurally distinct" (excluding false
alternatives that collapse to the same SPEC shape), "load-bearing"
(would change SPEC shape if wrong, distinct from mechanical filler),
and "shape-changing" (answers would change which requirements appear,
not just describing prose). Open question OQ-c (does the pre-check
fire on amendments routed through brainstorm?) resolved in T-007:
yes, fires on every brainstorm invocation.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-006">
All three skill templates carry the mechanical/semantic split with
the verbatim tie-breaker "If judging requires reading semantics, it
is semantic." Each template lists the concrete mechanical patterns
(TBD/TODO strings, "and"/"also" inside requirements, untouched
`<...>` placeholders, missing alpha-prefix ordinals) and instructs
the agent to fix mechanical issues inline without writing to
`## Open Questions` or chat. The tie-breaker line is word-for-word
identical across plan/amend/brainstorm so the agent applies the
split consistently.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-007 CHK-008">
The plan and amend templates each carry the literal template string
`- [ ] {ordinal}. **Self-review caught:** {issue}` exactly once,
inside the documented surfacing block for `## Open Questions`. The
brainstorm template carries a fixed-format chat preamble with a
verbatim opening line (`**Self-review caught the following before
presenting artifacts:**`) and a verbatim closing line (`Proceeding
with the four artifacts below.`), both backtick-quoted in the
template body as strings the agent uses unchanged. OQ-a (exact
preamble wording) resolved in T-007 by adopting the SPEC's candidate
wording verbatim.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-009">
All three templates contain an explicit no-loop instruction within
or immediately adjacent to the self-review (or pre-check) section.
Plan and amend: "Run this pass exactly once after writing SPEC.md.
Do not re-check after applying fixes." Brainstorm: "run this
internal review pass exactly once. Do not re-check after the
artifacts are presented." The surrounding step flow in each template
contains no branch that would re-enter the section.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-010">
The self-review prose lives in the three MiniJinja source files
under `resources/modules/skills/` (plan, amend, brainstorm). No file
outside this directory carries authoritative self-review prose; no
`_partials/` extraction was performed (OQ-b resolved per DEC-001 to
keep two independent copies for the plan/amend overlap, with a
parallel-copy comment in each template). T-008's re-eject produces
identical self-review section bodies in both
`.claude/skills/speccy-<phase>/SKILL.md` and
`.agents/skills/speccy-<phase>/SKILL.md`, modulo the existing
`{{ cmd_prefix }}` host-substitution variable.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-011 CHK-012">
All three authoring-phase skill templates carry alpha-prefix
`- [ ] a.` ... `- [ ] z.` format guidance for `## Open Questions` in
lock-step (plan step 4, amend step 2 addition, brainstorm step 3.4 +
routing section). Each template carries the 26-cap scope-smell note
word-for-word and the going-forward-only caveat (framing varies
appropriately by skill: amend "unless touched by this amendment";
brainstorm "sessions begun before this format was adopted"; plan
"existing SPECs retain their current `- [ ]` formatting unless
explicitly amended"). The amend template additionally instructs the
agent to preserve existing ordinals on edit (no renumbering) and
allocate the next free letter when appending — CHK-012 satisfied
mechanically. T-003 verified all three properties against the
upstream sources; T-008 propagated to ejected SKILL.md files.
</coverage>

<coverage req="REQ-010" result="satisfied" scenarios="CHK-013 CHK-014">
The brainstorm template carries a "Collapse-parallels heuristic"
paragraph at step 3.1 using "MAY" (not "MUST") wording, naming
reader cognitive load as the goal, and giving a concrete worked
example (R1-R6 collapse to one requirement with sub-bullets a-f).
The plan template carries a symmetric expansion paragraph at step 2:
"MAY expand each sub-bullet to its own atomic `<requirement>` block
... or keep them grouped under one `<requirement>` with a
`<done-when>` bullet list ... Agent discretion; neither choice is
surfaced as a self-review issue." Neither template enforces the
heuristic; neither self-review surfaces failure to collapse or
expand. The pairing is genuinely inverse (brainstorm collapses,
plan may re-expand) — directionality follows the workflow.
</coverage>

<coverage req="REQ-011" result="satisfied" scenarios="CHK-015 CHK-016">
The amendment branch is removed from
`resources/modules/skills/speccy-plan.md` at all six named
locations: lede, "When to use", "Steps" step 1 (replaced
identify-amendment-vs-new-spec branch with direct `speccy vacancy
--json` query), step 2 (dropped `**Amendment**:` sub-branch and
collapsed the old step 3 into step 2), and the frontmatter
`description:` line on both wrapper `.tmpl` files
(`resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl` and
`resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl`). Zero
matches for "Amendment", "amend an existing", or "SPEC-NNNN
argument" in either the module body or the ejected
`.claude/skills/speccy-plan/SKILL.md` and
`.agents/skills/speccy-plan/SKILL.md` files post-T-008. The
retirement is documented by omission — no `// removed for SPEC-0034`
markers, no "sole amendment path" callouts elsewhere.
</coverage>

<coverage req="REQ-012" result="satisfied" scenarios="CHK-017 CHK-018">
Claiming uses of "greenfield" removed from all six named live
workflow surfaces: `AGENTS.md`, `README.md`, `docs/ARCHITECTURE.md`,
`resources/modules/phases/speccy-init.md`,
`resources/modules/skills/speccy-plan.md`, and
`speccy-cli/tests/skill_body_discovery.rs`. The three remaining
post-edit hits (`AGENTS.md:121`, `README.md:96`,
`docs/ARCHITECTURE.md:865`) all appear in prose that explicitly
denies the greenfield/brownfield distinction. The
`chk015_speccy_plan_uses_vacancy_not_status_for_greenfield_id` test
was renamed to `chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id`
and simplified — the body no longer partitions on `**Amendment**` (that
anchor disappeared under REQ-011) and asserts `speccy vacancy --json`
present plus `speccy status --json` absent directly. Frozen
historical SPECs under `.speccy/specs/NNNN-*/` are untouched.
</coverage>

<coverage req="REQ-013" result="satisfied" scenarios="CHK-019">
`resources/modules/phases/speccy-tasks.md` Step 2 carries a concrete
example fragment showing all three required TASKS.md structural
elements: YAML frontmatter with `spec:`, `spec_hash_at_generation:`,
and `generated_at:`; the `# Tasks: SPEC-0007 My feature title`
level-1 heading; and two `<task>` lines — one with single-REQ
`covers="REQ-001"` and one with multi-REQ space-separated
`covers="REQ-001 REQ-002"`. A Key Constraints paragraph names the
TSK-004 (`InvalidCoversFormat`) hazard explicitly. The new
`chk019_speccy_tasks_template_documents_output_shape` test in
`speccy-cli/tests/skill_body_discovery.rs` asserts both literal
substrings `# Tasks: SPEC-` and `covers="REQ-001 REQ-002"` appear in
the module body. Re-eject propagated to both
`.claude/agents/speccy-tasks.md` and `.codex/agents/speccy-tasks.toml`.
</coverage>

</report>

## Retry counts

- T-001: 1 retry (business — wrapper-template frontmatter still
  carried amendment-trigger phrases on the `description:` line; tests
  — missing red-then-green evidence file and `Evidence:` field on the
  implementer-note). Resolved in attempt-2 by editing the two wrapper
  `.tmpl` files and creating `evidence/T-001.md`.
- T-002: 1 retry (business — out-of-scope mutation: the attempt-1
  implementer rewrote `resources/modules/skills/speccy-amend.md` step
  4 into a single-pass flow that hands off hash recording to
  `/speccy-tasks`, plus a `LOOP_RECIPES = &[]` change in the dogfood
  test constant, neither authorized by REQ-011 or REQ-012; style —
  two surviving claiming uses of "greenfield" at
  `docs/ARCHITECTURE.md:608` (`### Greenfield bootstrap` heading)
  and `README.md:148-158` (Greenfield/Brownfield state labels) that
  the original grep missed). Resolved in attempt-2 by reverting the
  amend-recipe mutation, restoring `LOOP_RECIPES` to its prior shape,
  renaming the ARCHITECTURE heading to `### AGENTS.md bootstrap`,
  rewriting the README state labels to State A / State B / State C
  matching `speccy-init.md`, and surfacing the amend single-pass
  question as `<open-question>` d. in SPEC.md.

T-003, T-004, T-005, T-006, T-007, T-008, and T-009 all passed their
first review round without retry.

## Out-of-scope items absorbed

- T-001 attempt-2 fixed the two wrapper `.tmpl` files'
  `description:` line (`resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl`) —
  outside the attempt-1 scope of the module body itself, but within
  REQ-011 bullet 4's intent ("skill template's frontmatter
  `description:` line"). The interpretation gap (module body vs
  wrapper frontmatter) is surfaced explicitly in the attempt-2
  implementer-note's "Discovered issues" block.
- T-002 attempt-1 surfaced a stale `LOOP_RECIPES` entry in
  `speccy-cli/tests/skill_packs.rs` referencing
  `speccy-amend/SKILL.md` as a loop recipe (a stale claim since the
  amend module had already been changed to "This recipe does not
  loop." pre-T-002). Attempt-2 reverted this change after the
  business reviewer correctly flagged it as bundled with the
  unauthorized amend-recipe mutation; the LOOP_RECIPES question
  remains genuinely open and is deferred to a future amendment per
  the deferral surfaced as `<open-question>` d.
- T-008 re-eject reverted a local-only override of
  `.claude/agents/speccy-work.md`'s model field
  (`opus[1m]`/`low` → upstream template's `sonnet[1m]`/`medium`).
  The override was a deliberate per-session boost during T-003
  through T-007 implementation; the re-eject correctly reverted it,
  satisfying the dogfood test invariant. The override was
  re-applied (and re-reverted again before ship) outside the SPEC's
  scope — this REPORT records the artifact, not the working-state
  oscillation.
- T-009 included a re-eject of `.claude/agents/speccy-tasks.md` and
  `.codex/agents/speccy-tasks.toml` to propagate the example
  fragment into both host packs. Re-eject is standard hygiene after
  module-level edits, but the work was bundled into T-009 to keep
  the dogfood test green at task close.
- Ship-time revert: the local `.claude/agents/speccy-work.md` model
  override (`opus[1m]`/`low`) was reverted back to the upstream
  template values (`sonnet[1m]`/`medium`) before this REPORT.md
  landed, so the committed tree matches the renderer's output.

## Open questions

Four QST-001 unchecked open questions remain in SPEC.md:

1. (a) Exact prose of the fixed chat-preamble template string for
   `/speccy-brainstorm` semantic surfacings. Resolved at
   implementation time in T-007 by adopting the SPEC's candidate
   wording verbatim — opening
   `**Self-review caught the following before presenting artifacts:**`,
   closing `Proceeding with the four artifacts below.` — both
   backtick-quoted in the template body and labeled as
   "verbatim, unchanged" strings the agent uses unmodified.

2. (b) Whether to factor a shared partial under
   `resources/modules/skills/_partials/` for the six shared
   plan/amend check properties. Resolved at implementation time in
   T-005 and T-006 per DEC-001: two independent copies, with a
   parallel-copy comment in each template pointing at the sibling.
   The duplication is small (~10 lines per template), the cost of
   independent evolution is high, and a future amendment can extract
   cleanly if both templates stabilize.

3. (c) Whether the brainstorm pre-check fires on amendments routed
   through `/speccy-brainstorm`. Resolved at implementation time in
   T-007: yes, fires on every brainstorm invocation, including when
   brainstorm is used as a front-end to amendment flows. The
   template carries an explicit statement to this effect.

4. (d) Whether the `speccy-amend` recipe should become single-pass
   (retire the `### Loop exit criteria` shape and hand off hash
   recording to `/speccy-tasks` exclusively). Surfaced during T-002
   review as out of scope for SPEC-0034 after the attempt-1
   implementer bundled an unauthorized rewrite. Deferred to a
   follow-up SPEC or amendment if dogfooding surfaces the need.

These questions are info-level lints only and do not block ship per
AGENTS.md "Feedback, not enforcement." Leaving them as `[ ]` in
SPEC.md preserves the historical record of which decisions were
left to (or surfaced during) the implementation phase.
