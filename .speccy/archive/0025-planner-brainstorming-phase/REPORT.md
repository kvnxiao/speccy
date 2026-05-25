---
spec: SPEC-0025
outcome: delivered
generated_at: 2026-05-17T22:30:00Z
---

# Report: SPEC-0025 Brainstorming skill for atomizing intent before SPEC creation

<report spec="SPEC-0025">

## Outcome

delivered

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
Shipped skill `speccy-brainstorm` lives at
`resources/modules/skills/speccy-brainstorm.md` and is host-wrapped
for Claude Code (`resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`)
and Codex (`resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`).
Both wrapper templates are byte-identical at 592 bytes and follow the
sibling `{% include "modules/skills/speccy-brainstorm.md" %}` shape.
Re-ejected dogfood mirrors at `.claude/skills/speccy-brainstorm/SKILL.md`
and `.agents/skills/speccy-brainstorm/SKILL.md` match the rendered
bundle byte-for-byte. CHK-001 is covered by
`speccy-cli/tests/init.rs::dogfood_outputs_match_committed_tree`
(byte-identity), `t005_claude_code_wrapper_shape_and_body` and
`t006_codex_wrapper_shape_and_body` in
`speccy-cli/tests/skill_packs.rs` (wrapper frontmatter + body shape
over the now-8-entry `SKILL_NAMES`), and `copy_claude_code_pack_skill_md`
/ `copy_codex_pack_skill_md` in `speccy-cli/tests/init.rs` (real
`speccy init` against a temp dir).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
Skill body teaches the Socratic flow with a prose hard gate. CHK-002
is covered by eight new content-shape tests at
`speccy-cli/tests/skill_packs.rs:1861-2036`:
`brainstorm_module_body_names_four_artifacts`,
`..._names_two_to_three_soft_guidance`,
`..._teaches_one_question_at_a_time`,
`..._carries_prose_hard_gate` (asserts the strong-gate language AND
the absence of any `<HARD-GATE>` machine sentinel per DEC-003),
`..._names_four_routing_destinations` (`## Summary` / `## Assumptions`
/ `## Open Questions` / `## Notes` plus `### Decisions` /
`<decision>` escalation), `..._names_both_terminal_actions`
(`{{ cmd_prefix }}speccy-plan` AND `{{ cmd_prefix }}speccy-amend`),
`..._uses_cmd_prefix_consistently` (regression guard against literal
`/speccy-plan` leaks under no-prefix hosts), and
`brainstorm_rendered_outputs_use_host_specific_prefix` (runs both
host packs end-to-end through `render_host_pack` and asserts Claude
Code gets `/speccy-plan` / `/speccy-amend` while Codex gets bare
forms and no slashed forms). Mutations that strip `## Hard gate`,
delete a destination, remove the amendment branch, or re-introduce
literal `/speccy-plan` in source each turn at least one test red.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
`resources/modules/skills/speccy-plan.md` references
`{{ cmd_prefix }}speccy-brainstorm` as a recommended precursor for
fuzzy asks at lines 16-19. The stale "inlines `AGENTS.md`" /
"inlines the nearest parent `MISSION.md`" wording is removed
(zero hits for the five retired substrings; replaced with the
accurate "host harness auto-loads `AGENTS.md`" and "names the
nearest parent `MISSION.md` path" wording per SPEC-0023
REQ-005/REQ-006). Amendment-form description preserved as a
single-pass surgical edit with no mandatory brainstorm step. CHK-003
rides on `speccy-cli/tests/init.rs::dogfood_outputs_match_committed_tree`
for byte-identity between source and dogfood mirrors; a coordinated
mutation across both layers would slip through unique-grep
content-shape coverage, but that is the convention shared with prior
shipped-skill body edits and accepted under the spec's non-goal
"No new test infrastructure beyond the existing skill-pack and init
enumeration assertions."
</coverage>

## Task summary

3 tasks total, all `state="completed"`. Two retries:

- T-001 retried once. Original blocking on (a) business — REQ-002
  done-when item 6 required pointing at both `/speccy-plan` and
  `/speccy-amend` as terminal actions but the rev2 body only named
  the new-spec path; (b) tests — `<task-scenarios>` named grep-style
  assertions (case-insensitive "one question at a time", four
  artifact labels, "2-3" soft guidance, hard-gate prose, four
  routing destinations, terminal actions per host) but no test
  enforced them. Rev3 added the amendment branch with explicit prose
  on why (`speccy plan SPEC-NNNN` would skip TASKS.md reconcile +
  Changelog row + spec-hash re-record and produce hash drift) plus
  the eight content-shape tests above.
- T-002 retried once. Style blocked on two byte-shape drifts: both
  new `.tmpl` files ended with a trailing newline while every sibling
  wrapper ends with no final `\n`; Claude and Codex `description:`
  lines diverged (Claude said `before invoking /speccy-plan`, Codex
  said bare `speccy-plan`). Rev3 stripped the trailing newlines and
  aligned both descriptions to the bare form, making the two `.tmpl`
  files byte-identical at 592 bytes.
- T-003 retried zero times.

No SPEC amendment was triggered; the Changelog row dated 2026-05-17
"Amend (full pivot)" predates the loop and reflects the user-initiated
pivot from "Phase 1 inside greenfield planner" to "standalone shipped
skill" before any task entered `state="in-progress"`.

## Out-of-scope items absorbed

- T-002 rev2: renamed renderer-internal tests in
  `speccy-cli/src/render.rs`
  (`render_host_pack_claude_code_emits_seven_skills`
  → `..._emits_eight_skills`, and the Codex equivalent) so the
  bundle-count assertion matches the now-8-entry `SKILL_NAMES`. Not
  on the suggested-files list, but the mechanical consequence of
  growing the bundle.
- T-003: extended `speccy-core/tests/fixtures/in_tree_id_snapshot.json`
  with the SPEC-0025 entry so
  `every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot`
  passes (the snapshot fixture is per-spec and grows when a new spec
  is added).
- T-001 rev3: added eight new content-shape tests at
  `speccy-cli/tests/skill_packs.rs:1861-2036` to make REQ-002's
  `<behavior>` and `<scenario id="CHK-002">` grep-style assertions
  executable. Not on the suggested-files list (which named only the
  module body), but required to close the tests reviewer's blocking
  finding that mutations stripping the hard gate, routing
  destinations, or "one question at a time" left every existing test
  green.

## Skill updates

(none)

## Deferred / known limitations

- "exactly_seven" naming carryover at
  `speccy-cli/tests/skill_packs.rs:1149/1175/1258/1284`: function
  names and assertion messages still say "exactly_seven" even though
  the comparator `SKILL_NAMES` is now 8 entries. The tests still
  pass correctly (the comparator is the slice, not a literal `7`),
  but a future failure would print a misleading count. Tests-persona
  flagged for a separate follow-up across both retries; out of scope
  for this spec.
- Pre-existing local-only `result_large_err` clippy errors on
  `speccy-core::error::ParseError` (43 errors, Windows toolchain
  only; CI green at baseline `c5b632c` and on this branch).
  Confirmed not introduced by SPEC-0025 via stash-then-clippy at
  baseline. Out of scope for SPEC-0025; flag for a future spec
  touching `error.rs`.
- Skill-loop primitive gap: when a retry in an early task (T-001)
  invalidates a later completed task's work (T-003 owned the
  original re-eject of dogfood mirrors), there is no clean primitive
  to re-open the later task to refresh its outputs. Today the
  workaround is to bundle the re-eject into the next pending task's
  scope (T-002 retry here) or to add a fresh task. Surfaced in
  T-001 rev3's Discovered issues; worth a follow-up entry, not
  blocking for SPEC-0025.
- Dogfood byte-identity is one-sided drift protection only: a
  coordinated mutation across both the source body and the dogfood
  mirror would leave all tests green. REQ-003's CHK-003 content
  greps ride on byte-identity rather than dedicated content-shape
  tests, matching the established convention for shipped-skill body
  edits. Worth a follow-up entry if dogfooding shows silent skill-body
  content drift; consistent with the SPEC's "No new test
  infrastructure beyond the existing skill-pack and init enumeration
  assertions" non-goal.

</report>
