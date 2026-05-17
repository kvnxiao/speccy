---
spec: SPEC-0025
spec_hash_at_generation: 3e9b8048097bbf7f34a4a3c24bb9cf7814487c638af818bb77c530d026be6b0d
generated_at: 2026-05-17T19:55:40Z
---

# Tasks: SPEC-0025 Brainstorming skill for atomizing intent before SPEC creation

<tasks spec="SPEC-0025">

## Phase 1: New skill body

<task id="T-001" state="completed" covers="REQ-002">
Write the speccy-brainstorm module skill body

- Suggested files: `resources/modules/skills/speccy-brainstorm.md`

- Implementer note (session-2026-05-17-T001-rev2):
  - Completed: Wrote `resources/modules/skills/speccy-brainstorm.md` carrying the `# {{ cmd_prefix }}speccy-brainstorm` slug-style heading; a one-paragraph summary that introduces the Socratic exchange and explicit "ephemeral chat" framing (no disk artifact); a "When to use" section discriminating fuzzy asks from sharp asks; an explicit `## Hard gate` section telling the agent not to invoke `{{ cmd_prefix }}speccy-plan` and not to write SPEC.md until the user has approved the framing; six-step `## Steps` section that (1) reads AGENTS.md + scans recent specs, (2) teaches "one question at a time" with multiple-choice preference, (3) produces four numbered artifacts (restated-ask-atomized, 2-3 alternative framings with sketch + rejection reason, silent assumptions, open questions in `- [ ]` format), (4) explicit stop-and-wait, (5) iterate on redirection, (6) invoke `{{ cmd_prefix }}speccy-plan` after approval; a `## Routing brainstorm outputs into SPEC.md` section naming the four destinations (`## Summary`, `<assumptions>` inside `## Assumptions`, `## Open Questions`, `## Notes`) plus `### Decisions` / `<decision>` escalation for load-bearing trade-offs; a `## Key principles` section with "one question at a time", "scale to complexity", "atomize ruthlessly", "no premature implementation", "hard gate before SPEC.md"; closing pointer to `{{ cmd_prefix }}speccy-tasks SPEC-NNNN` as the post-plan next step.
  - Undone: Wrappers + dogfood mirrors land in T-002 and T-003 respectively.
  - Commands run: (file creation only; no commands needed yet — tests for module body shape land in T-002/T-003 via the SKILL_NAMES enumeration)
  - Exit codes: (n/a)
  - Discovered issues: (none)
  - Procedural compliance: (none)

- Review (security, pass): Pure markdown skill body — no code, no
  network or auth surface, no secrets, no logging, no new
  dependencies. Only template variable is the renderer-controlled
  `{{ cmd_prefix }}` (`/` or empty), not user-supplied, so no
  template-injection risk. The "skip the brainstorm" bypass at
  `resources/modules/skills/speccy-brainstorm.md:36-39` is a workflow
  gate (Principle 1 feedback-not-enforcement, codified in SPEC
  DEC-003), not a security boundary. Agent-Read pointers in step 1
  name only in-repo paths (`AGENTS.md`, `.speccy/specs/`); no path
  traversal exposure. No findings.

- Review (style, pass): `resources/modules/skills/speccy-brainstorm.md`
  matches the sibling shipped-skill conventions exactly — leading
  blank line then `# {{ cmd_prefix }}speccy-brainstorm` heading,
  one-paragraph summary, `## When to use` / `## Steps` section spine
  (same shape as `speccy-plan.md` and `speccy-init.md`), prose-only
  `## Hard gate` with no machine markers, ~70-char line wrap
  (max=69, zero lines over 80; tighter than `speccy-init.md` at
  max=118 and on par with `speccy-plan.md` at max=69), LF line
  endings with a trailing newline matching every sibling, bold-lead
  numbered list items mirroring `speccy-init.md`, and the
  `speccy plan` bash block on lines 103-106 follows the same
  raw-CLI-with-inline-comment idiom used in `speccy-plan.md:29-32`
  and `speccy-amend.md:19-21`. No duplicated helpers, no parallel
  patterns, no suppression annotations, no dead prose introduced by
  the diff.

<task-scenarios>
  - Given `resources/modules/skills/speccy-brainstorm.md` after this
    task, when read, then it begins with the `{{ cmd_prefix }}speccy-brainstorm`
    slug-style heading (matching the other shipped skill bodies' `#`
    heading shape) and a one-paragraph summary of the skill's purpose.
  - Given the skill body, when read, then it names four artifacts the
    agent must produce during the brainstorm: (1) a restated ask
    broken into atomic, first-principle requirements; (2) 2-3
    alternative framings with one-sentence sketches and explicit
    rejection reasons; (3) silent assumptions the agent would
    otherwise bake in; (4) open questions in the `- [ ]` checkbox
    format used by the PRD template's `## Open Questions` section.
  - Given the skill body, when read, then it names "2-3" as soft
    guidance for alternative-framing counts and explicitly instructs
    the agent to scale to slice complexity (so a trivial ask may
    surface zero alternatives, a load-bearing one may need four).
  - Given the skill body, when grep'd case-insensitively for
    "one question at a time", then a hit is returned. The skill
    teaches the obra/superpowers interaction discipline of asking
    clarifying questions one at a time rather than batching them.
  - Given the skill body, when read, then it carries an explicit
    hard-gate instruction telling the agent not to invoke
    `{{ cmd_prefix }}speccy-plan` and not to write SPEC.md until the
    user has approved the framing. The gate is named in prose using
    strong language (e.g., "do NOT", "STOP", or "hard gate"). No
    machine marker (XML sentinel, regex hook) is introduced.
  - Given the skill body, when read, then it names the four
    destinations Phase-1-style brainstorm outputs flow into when the
    agent next invokes `/speccy-plan`: `## Summary` (restated ask,
    folded as designed artifact not pasted verbatim);
    `<assumptions>` inside `## Assumptions` (silent assumptions);
    `## Open Questions` (unresolved questions); `## Notes` (rejected
    alternative framings, with `### Decisions` / `<decision>`
    escalation for load-bearing trade-offs).
  - Given the skill body, when read, then it names
    `{{ cmd_prefix }}speccy-plan` as the terminal next step after
    the user has approved the framing.
</task-scenarios>

- Retry: business blocked on the amendment-path terminal pointer —
  `resources/modules/skills/speccy-brainstorm.md:99-110` and the
  closing pointer at lines 148-150 only name
  `{{ cmd_prefix }}speccy-plan` and route amendments through
  `speccy plan SPEC-NNNN`, bypassing the `{{ cmd_prefix }}speccy-amend`
  skill (which owns TASKS.md reconcile + Changelog row + spec-hash
  re-record). Add the `{{ cmd_prefix }}speccy-amend` branch in step 6
  (and mention it in the closing pointer) per REQ-002 done-when item 6.
  Tests blocked on missing executable coverage — the `<task-scenarios>`
  and REQ-002 `<behavior>` / `<scenario id="CHK-002">` enumerate
  grep-style assertions (case-insensitive "one question at a time", the
  four artifact labels, "2-3" as soft guidance, the hard-gate prose
  naming `{{ cmd_prefix }}speccy-plan`, the four destination strings,
  the terminal-action pointer per host) but no test in
  `speccy-cli/tests/` enforces them. Mutating the skill body to delete
  `## Hard gate`, strip "one question at a time", or drop the routing
  destinations leaves every existing test green — `recipe_content_shape`
  (`speccy-cli/tests/skill_packs.rs:509`) only checks `## When to use`
  + one fenced `speccy ...` block. Either add grep-style assertions on
  `resources/modules/skills/speccy-brainstorm.md` here in T-001 or
  hand off explicitly to a follow-up task that codifies the content
  shape. Security and style pass; T-003's style reviewer additionally
  surfaced a cross-task lint that originates in this slice —
  `resources/modules/skills/speccy-brainstorm.md:64` hard-codes
  `/speccy-plan` instead of `{{ cmd_prefix }}speccy-plan`, which then
  bleeds through to the Codex mirror at
  `.agents/skills/speccy-brainstorm/SKILL.md:68` as a literal slash
  under a no-prefix host. Fix the source occurrence in the same retry
  pass and re-eject in T-003 (or a follow-up T-003-equivalent slice)
  to refresh the mirrors.

- Implementer note (session-2026-05-17-T001-rev3):
  - Completed: Rewrote step 6 of `resources/modules/skills/speccy-brainstorm.md` to branch on the path — new-spec invokes `{{ cmd_prefix }}speccy-plan` (with a fenced `speccy plan` bash block to keep the `recipe_content_shape` test happy), amendments invoke `{{ cmd_prefix }}speccy-amend` (not bare `speccy plan SPEC-NNNN`) with prose explicitly naming why: the amend skill orchestrates SPEC.md edit + Changelog + TASKS.md reconcile + spec-hash re-record, which a direct `speccy plan` invocation would skip and produce hash drift. Updated the closing pointer (lines 150-153 in the new file) to name both terminal actions. Fixed the cross-task lint surfaced by T-003's style reviewer: line 64 in the prior body hard-coded `/speccy-plan`; replaced with `{{ cmd_prefix }}speccy-plan` so the Codex mirror no longer ships a stray leading slash under a no-prefix host. Added 8 new content-shape tests at the end of `speccy-cli/tests/skill_packs.rs` (lines 1834-2007) that codify REQ-002 `<behavior>` and `<scenario id="CHK-002">` as executable assertions: `brainstorm_module_body_names_four_artifacts`, `..._names_two_to_three_soft_guidance`, `..._teaches_one_question_at_a_time`, `..._carries_prose_hard_gate` (asserts both the strong gate language and the absence of any `<HARD-GATE>` machine sentinel per DEC-003), `..._names_four_routing_destinations` (covers `## Summary` / `## Assumptions` / `## Open Questions` / `## Notes` plus `### Decisions` / `<decision>`), `..._names_both_terminal_actions` (`{{ cmd_prefix }}speccy-plan` AND `{{ cmd_prefix }}speccy-amend`), `..._uses_cmd_prefix_consistently` (regression guard for the `/speccy-plan` line-64 lint), and `brainstorm_rendered_outputs_use_host_specific_prefix` (renders both host packs and asserts Claude Code gets `/speccy-plan` + `/speccy-amend` while Codex gets bare `speccy-plan` + `speccy-amend` and no slashed forms). A mutation that strips `## Hard gate`, removes a destination, deletes the amendment branch, or re-introduces `/speccy-plan` in the source body now turns at least one of these tests red.
  - Undone: Re-eject of the dogfood mirrors at `.claude/skills/speccy-brainstorm/SKILL.md` and `.agents/skills/speccy-brainstorm/SKILL.md`. Per the prior reviewer note, the re-eject "lands in T-003 (or a follow-up T-003-equivalent slice)"; since T-003 is already `state="completed"` and T-002 retry is still pending with byte-shape changes of its own that also need a re-eject, the cleanest sequencing is: T-002 retry first (fixes wrapper bytes), then a single re-eject pass refreshes all mirrors at once. The `dogfood_outputs_match_committed_tree` test fails until then; this is the architectural choke point firing as designed.
  - Commands run:
    - `cargo test --test skill_packs --no-fail-fast`
    - `cargo test --workspace --no-fail-fast`
    - `cargo +nightly fmt --all --check`
    - `cargo +nightly fmt --all`
    - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - Exit codes: pass (42/42 skill_packs tests including 8 new); 1 fail (`dogfood_outputs_match_committed_tree` only — expected scope-boundary handoff); pass; pass; fail-local (the same pre-existing 43 `result_large_err` clippy errors on `speccy-core::error::ParseError` that T-003's implementer note already documented — Windows-local heuristic, CI green on these commits, NOT introduced by this task).
  - Discovered issues: The dogfood handoff exposes a structural concern: when a retry in an early task (T-001) invalidates a later task's completion (T-003 owned the original re-eject), there's no clean primitive in the skill loop to "re-open T-003 to refresh mirrors". Today the workaround is to bundle the re-eject into the next pending task's scope (T-002 retry here) or to add a fresh T-004. Worth noting in a FOLLOWUPS.md entry; not blocking for SPEC-0025.
  - Procedural compliance: (none)

- Review (business, blocking): REQ-002 done-when item 6 says the skill
  must end by pointing at `/speccy-plan` (or `/speccy-amend` if the
  user wants to amend an existing spec) as the next step after framing
  is approved. The diff at
  `resources/modules/skills/speccy-brainstorm.md:99-110` only names
  `{{ cmd_prefix }}speccy-plan` and routes the amendment path through
  `speccy plan SPEC-0007` directly, bypassing the
  `{{ cmd_prefix }}speccy-amend` skill, which exists precisely so the
  TASKS.md reconciliation, Changelog row, and spec-hash re-record
  (steps 3-6 of `speccy-amend.md`) are not forgotten. A brainstormed
  amendment that lands via `speccy plan SPEC-NNNN` alone will skip
  those steps and produce hash drift. Add the
  `{{ cmd_prefix }}speccy-amend` branch in step 6 (and consider
  mentioning it in the closing pointer at lines 148-150) so the SPEC's
  named terminal action for the amendment path is honored.

- Review (tests, blocking): the implementer note defers content-shape
  assertions to T-002/T-003 "via the SKILL_NAMES enumeration", but
  adding `"speccy-brainstorm"` to `SKILL_NAMES` only runs the existing
  generic iterations (frontmatter parses; `## When to use` heading
  present; one fenced `speccy ...` block;
  `dogfood_outputs_match_committed_tree` byte-identity). None of the
  seven slice-level `<task-scenarios>` are codified as tests —
  `speccy-cli/tests/skill_packs.rs:189` and
  `speccy-cli/tests/init.rs:67` are the only "brainstorm" hits in
  either file, and both are bare enumeration entries. The
  load-bearing grep-style assertions REQ-002 `<behavior>` and
  `<scenario id="CHK-002">` literally name (case-insensitive
  "one question at a time"; the four artifact labels; "2-3" as soft
  guidance scaled to complexity; the hard-gate prose naming
  `{{ cmd_prefix }}speccy-plan` as the gated action; the four
  destination strings `## Summary` / `## Assumptions` /
  `## Open Questions` / `## Notes`; the terminal `/speccy-plan` for
  Claude Code and bare `speccy-plan` for Codex) are not asserted
  anywhere. Mentally rewriting
  `resources/modules/skills/speccy-brainstorm.md` to delete the
  `## Hard gate` section, strip every "one question at a time"
  occurrence, or remove the routing destinations leaves every
  existing test green — `recipe_content_shape`
  (`speccy-cli/tests/skill_packs.rs:509`) only requires `## When to
  use` and one fenced `speccy ...` block, both of which survive the
  mutation. That is the canonical "test passes even when the
  behaviour is gone" failure mode the persona warns about. The
  current file content does satisfy the scenarios on manual
  inspection (steps 1-6 cover the four artifacts; the `## Hard gate`
  section uses "Do NOT" language and names `speccy-plan`; the
  `## Routing brainstorm outputs into SPEC.md` section names all
  four destinations; step 6 + the closing paragraph point at
  `speccy-plan`), so the gap is purely test coverage, not
  implementation — but per the persona contract scenarios are meant
  to be translated into executable tests, not into reviewer
  eyeballing. Fix: either add grep-style assertions against
  `resources/modules/skills/speccy-brainstorm.md` (via the existing
  `SKILLS` bundle or `include_str!`) here in T-001, or hand off
  explicitly to T-002 with a "content-shape assertions for the
  brainstorm body land here" note and verify they exist when T-002
  is reviewed.

- Review (business, pass): rev3 cleanly closes the prior
  amendment-path block. REQ-002 done-when item 6 is satisfied at
  `resources/modules/skills/speccy-brainstorm.md:99-114` where step 6
  branches on path — new-spec invokes `{{ cmd_prefix }}speccy-plan`
  (with fenced `speccy plan` block at lines 105-107), amendment
  invokes `{{ cmd_prefix }}speccy-amend` with explicit prose naming
  why (orchestrates SPEC.md edit + Changelog row + TASKS.md reconcile
  + spec-hash re-record, so the brainstormed amendment doesn't drop
  reconciliation and produce hash drift). The closing pointer at
  lines 156-159 names both terminal actions. The earlier `/speccy-plan`
  hard-coding at line 64 (flagged by T-003's style reviewer) is
  fixed — line 64 now reads `{{ cmd_prefix }}speccy-plan`. The other
  five done-when items also land: four artifacts named at lines
  56-86 (atomized restated ask 59-66, 2-3 framings with sketch +
  rejection 68-75, silent assumptions 77-81, open questions in
  `- [ ]` 83-86); "2-3" marked as soft guidance scaled to slice
  complexity at lines 68-71 ("trivial ask may surface zero
  alternatives; a load-bearing architecture ask may need four"); the
  prose hard gate at lines 30-39 names `{{ cmd_prefix }}speccy-plan`
  as the gated action with no `<HARD-GATE>` machine sentinel per
  DEC-003; routing destinations cover all four PRD sections plus
  `### Decisions` / `<decision>` escalation at lines 120-137; "one
  question at a time" appears as both step 2 (lines 49-54) and as a
  Key Principle (141-143). Non-goals respected: no new CLI command,
  no `## Brainstorm` SPEC.md section invented, output described as
  ephemeral chat (lines 12-14), no `plan-greenfield.md`/`plan-amend.md`
  edits in this slice. The user-controlled "skip the brainstorm"
  bypass at lines 36-39 is an explicit user-override (not an agent
  self-bypass), aligned with DEC-003's prose-gate intent. Open
  Questions in SPEC.md lines 617-631 are resolved `[x]` skip; the
  diff implicitly honors both resolutions (no `.speccy/specs/`
  scanning baked into the prompt — step 1 frames it as agent
  judgment; no special-cased amendment-form brainstorm reference).
  Changelog row 2 (full pivot to standalone skill) is the shape the
  diff reflects, not the original Phase-1-inside-greenfield draft.
  User stories US-1 (solo dev pre-Requirement framing) and US-2
  (agent gets explicit pause instruction) are served end-to-end.
  Tests-persona's coverage gap is a separate concern about test
  enforcement, not about behavioural drift between SPEC and diff.

- Review (tests, pass): rev3 closes the prior coverage gap. Eight
  new tests at `speccy-cli/tests/skill_packs.rs:1861-2036`
  (`brainstorm_module_body_names_four_artifacts`,
  `..._names_two_to_three_soft_guidance`,
  `..._teaches_one_question_at_a_time`,
  `..._carries_prose_hard_gate`,
  `..._names_four_routing_destinations`,
  `..._names_both_terminal_actions`,
  `..._uses_cmd_prefix_consistently`,
  `brainstorm_rendered_outputs_use_host_specific_prefix`) codify the
  REQ-002 `<behavior>` / `<scenario id="CHK-002">` grep-style
  assertions as executable. The tests are mutation-resistant on the
  load-bearing axes: dropping `## Hard gate` strips the
  case-insensitive `"hard gate"` match (line 1911), the strong-gate
  marker check (1914), and the gated-action name (1920); removing
  any of the four artifact labels (`Restated ask`,
  `alternative framings`, `Silent assumptions`, `Open questions`)
  fails the per-label loop (1864-1874); dropping the `- [ ]`
  checkbox instruction fails 1876; deleting the amendment branch
  fails 1957 (`{{ cmd_prefix }}speccy-amend` presence); collapsing a
  routing destination fails 1940 or 1944; re-introducing literal
  `/speccy-plan` in source fails the cmd_prefix regression guard at
  1974-1982; and any drift between source body and rendered output
  per host fails `brainstorm_rendered_outputs_use_host_specific_prefix`
  (2016-2033) which runs `render_host_pack(HostChoice::ClaudeCode)`
  and `(HostChoice::Codex)` end-to-end through the embedded bundle +
  MiniJinja pipeline and asserts both positive presence (`/speccy-plan`
  + `/speccy-amend` on Claude; bare forms on Codex) and negative
  absence (no slashed forms on Codex). Reads go through
  `read_brainstorm_module_body()` (line 1848) which `fs_err::read`s
  the on-disk resource file and `find_rendered_skill()` (line 74)
  which selects from the actual `RenderedFile` vec — neither path
  mocks the system under test. Verified locally:
  `cargo test --test skill_packs` reports all 8 brainstorm tests as
  `ok` in the run output. The DEC-003 negative guard
  (no `<HARD-GATE>` / `<hard-gate>` machine sentinel, line 1923) is
  also asserted; a regression that re-introduces a machine marker
  would fail. Slice-level `<task-scenarios>` for "begins with the
  `{{ cmd_prefix }}speccy-brainstorm` slug-style heading and a
  one-paragraph summary" (scenario 1) isn't asserted by a dedicated
  new test, but the heading slug is exercised indirectly by the
  shared `recipe_content_shape`/frontmatter checks that already
  iterate `SKILL_NAMES` once T-002 adds `"speccy-brainstorm"` to it,
  and `brainstorm_module_body_uses_cmd_prefix_consistently`
  effectively pins the templated form. Non-blocking nit: the new
  tests read with `fs_err::read_to_string(&path).unwrap_or_else(...)`
  rather than the `include_str!` route, which means the disk file
  must be present at test time — fine here since the file is checked
  in, but a future move to embed-only would need the assertion
  retargeted to the `RESOURCES` bundle.

- Review (security, pass): rev3 keeps the slice security-clean. The
  module body remains pure markdown with one renderer-controlled
  template variable (`{{ cmd_prefix }}`, host-aware `/` or empty);
  rev3 actually tightens prompt determinism by replacing the prior
  hard-coded `/speccy-plan` at line 64 with
  `{{ cmd_prefix }}speccy-plan` so the Codex no-prefix host no longer
  ships a stray literal slash (a style fix, not a vuln, but removes
  a small inconsistency in renderer output). The new step-6
  amendment branch at
  `resources/modules/skills/speccy-brainstorm.md:99-114` names
  `{{ cmd_prefix }}speccy-amend` — also renderer-controlled, no user
  input, no template-injection surface. The 8 new content-shape tests
  appended at `speccy-cli/tests/skill_packs.rs:1834-2036` read the
  module body via `fs_err::read_to_string` against a path built from
  `workspace_root().join("resources").join("modules").join("skills").join("speccy-brainstorm.md")`
  (all static joins, no untrusted input, no path-traversal vector),
  assert with `body.contains(...)` substring checks and `assert!`
  macros (no regex eval, no subprocess, no deserialization of
  attacker-controlled content), and exercise `render_host_pack` with
  compile-time `HostChoice` enum variants — same renderer path
  covered by dozens of existing tests. The
  `brainstorm_module_body_carries_prose_hard_gate` assertion that
  `<HARD-GATE>` / `<hard-gate>` is absent codifies DEC-003 as an
  executable invariant, which strengthens the "no opaque machine
  sentinel" posture rather than weakening it. The "skip the
  brainstorm" bypass at lines 36-39 is unchanged from rev2 and
  remains a workflow gate (Principle 1 feedback-not-enforcement,
  DEC-003), not a security boundary; it requires an explicit user
  utterance, not agent self-bypass. No new dependencies (`fs_err` is
  already a workspace dep used by adjacent tests), no secrets, no
  auth, no logging, no crypto, no network surface. No findings.

- Review (style, pass): rev3's edits to
  `resources/modules/skills/speccy-brainstorm.md` and the new tests in
  `speccy-cli/tests/skill_packs.rs:1834-2036` are style-clean. The
  retry's load-bearing fix at line 64 now reads
  `{{ cmd_prefix }}speccy-plan` (was hard-coded `/speccy-plan`),
  eliminating the cross-host prefix leak that T-003's style reviewer
  flagged; a fresh grep for bare `/speccy-` against the whole file
  returns zero hits. The split-step-6 branching prose at lines 99-114
  matches the line-wrap (max=70, 0 lines >80; on par with sibling
  `speccy-plan.md` max=69 and `speccy-work.md` max=69 — well inside
  the project's de facto ceiling that tops out at
  `speccy-init.md` max=118), uses the bold-lead
  `- For a **new SPEC**, ...` / `- For an **amendment**, ...` list
  shape that mirrors the `**Bold-heading.** prose...` idiom used at
  `speccy-init.md:27-86` and steps 1-5 of this file. The fenced
  `speccy plan` bash block at lines 105-107 follows the raw-CLI
  idiom shared with `speccy-tasks.md:19-21` and `speccy-amend.md:19-21`.
  LF line endings throughout, single trailing newline (last byte
  0x0a, 159 lines total), zero trailing whitespace, no
  `<HARD-GATE>` machine sentinel (DEC-003), no `#[allow]` /
  `#[expect]` suppressions added in either file. In the test file:
  the eight new functions sit under the file-level
  `#![expect(clippy::expect_used, reason = "...")]` at lines 1-4 (no
  new per-fn allow), use the project-standard
  `unwrap_or_else(... panic_with_test_message ...)` pattern (line
  1854, 2013) — zero `.unwrap()` anywhere in the file, matching the
  `rust-error-handling.md` rule and the AGENTS.md "never `unwrap()` /
  `expect()` / `panic!()`" line — and read through
  `fs_err::read_to_string` + `workspace_root()` rather than bare
  `std::fs` / `std::path::Path`, honoring the preferred-crates table
  in `rust-dependencies.md`. Function naming
  (`brainstorm_module_body_names_four_artifacts`,
  `..._teaches_one_question_at_a_time`, etc.) matches the existing
  `noun_predicate` style of siblings (`persona_files_present`,
  `recipe_content_shape`, `implementer_prompt_handoff_template`).
  Assertion messages cite the REQ-NNN / CHK-NNN they implement —
  same shape as the existing CHK-banner comments at lines 212-214.
  The `for (host, install_root, expected_plan, expected_amend,
  unexpected) in [...]` table-driven iteration at lines 1993-2012
  reuses the same shape `HOST_SKILL_ROOTS` (line 197) already
  established for per-host iterators. No duplicated helpers
  (`read_brainstorm_module_body` is the brainstorm-specific
  counterpart to the existing `read_persona` / `read_prompt` /
  `read_wrapper_template` helpers at lines 87-101 / 110-119 / 56-72
  and follows the same shape). No imports re-ordered (the file's
  existing `use` block at lines 39-46 is untouched; new tests use
  already-imported `HostChoice` and `render_host_pack`). No dead
  prose, no parallel patterns; the test additions go beyond the
  suggested-file but are the executable counterpart to REQ-002
  `<scenario id="CHK-002">` that rev2's tests reviewer required and
  the implementer note accounts for them explicitly.

</task>

## Phase 2: Host wrappers and skill-enumeration tests

<task id="T-002" state="completed" covers="REQ-001">
Add host wrappers for speccy-brainstorm and extend the skill-enumeration tests

- Suggested files:
  `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`,
  `speccy-cli/tests/skill_packs.rs`,
  `speccy-cli/tests/init.rs`

- Implementer note (session-2026-05-17-T002-rev2):
  - Completed: Added `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl` and `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl` following the existing per-host wrapper pattern: YAML frontmatter with `name: speccy-brainstorm` and a `description:` line naming the brainstorming purpose + trigger phrases, then a single `{% include "modules/skills/speccy-brainstorm.md" %}` directive. Extended `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` from 7 entries to 8 with `"speccy-brainstorm"`. Extended `speccy-cli/tests/init.rs::SKILL_NAMES` similarly, updating the array type from `[&str; 7]` to `[&str; 8]`. Updated two renderer-internal tests in `speccy-cli/src/render.rs::tests` (`render_host_pack_claude_code_emits_seven_skills` → `..._emits_eight_skills` and Codex equivalent) so the bundle-count assertions expect the new total. All renderer + skill-pack enumeration tests pass.
  - Undone: Dogfood files at `.claude/skills/speccy-brainstorm/SKILL.md` and `.agents/skills/speccy-brainstorm/SKILL.md` are not yet committed; T-003 re-ejects.
  - Commands run: `cargo test --workspace`
  - Exit codes: fail (only `dogfood_outputs_match_committed_tree`, expected — fixed by T-003 re-eject)
  - Discovered issues: (none)
  - Procedural compliance: (none)

- Review (security, pass): metadata-only change. The two new
  `SKILL.md.tmpl` files carry static YAML frontmatter (`name:` plus
  a hardcoded `description:` of trigger phrases) and a single
  `{% include "modules/skills/speccy-brainstorm.md" %}` against a
  compile-time-embedded resource path — no runtime input, no
  user-controllable path, no template-injection surface. The
  `SKILL_NAMES` extensions in `speccy-cli/tests/skill_packs.rs:189`
  and `speccy-cli/tests/init.rs:59` (with the `[&str; 7]` → `[&str; 8]`
  bump) are compile-time-checked literals; no unsafe indexing, no
  arithmetic on lengths. No new dependencies, no secrets, no auth,
  no logging, no crypto. Nothing in this slice exposes attack
  surface.

- Review (style, blocking): two byte-shape drifts from the existing
  seven-wrapper pattern.
  (1) Both new `.tmpl` files end with `%}\n` (trailing newline after
  the close of the include directive), while every existing wrapper
  under `resources/agents/.claude/skills/*/SKILL.md.tmpl` and
  `resources/agents/.agents/skills/*/SKILL.md.tmpl` ends with `%}`
  (no trailing newline). The drift propagates through `render_host_pack`
  into the dogfood mirrors: `.claude/skills/speccy-brainstorm/SKILL.md`
  ends with `).\n\n` (extra blank line) where `.claude/skills/speccy-plan/SKILL.md`
  and the other six end with a single `\n`. Strip the trailing newline
  on both `.tmpl` files (file ends after `%}` with no final `\n`) and
  re-eject the dogfood mirrors so the byte-shape matches the other
  seven shipped skills.
  (2) The Claude and Codex `description:` lines diverge (Claude says
  `before invoking /speccy-plan`, Codex says `before invoking speccy-plan`).
  Every other Claude/Codex wrapper pair is byte-identical in its
  description (`speccy-plan`, `speccy-review`, `speccy-ship`,
  `speccy-init`, `speccy-tasks`, `speccy-work`, `speccy-amend` all
  match across hosts). Pick one form — either drop the leading slash
  on the Claude side, or add it on the Codex side — and apply it to
  both wrappers so the per-host wrapper convention stays
  byte-identical across hosts.

<task-scenarios>
  - Given `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
    after this task, when read, then it carries a YAML frontmatter
    block with `name: speccy-brainstorm` and a `description:` line
    naming the brainstorming purpose, followed by exactly one
    `{% include "modules/skills/speccy-brainstorm.md" %}` directive.
    The pattern matches the other speccy-* Claude Code wrappers.
  - Given `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
    after this task, when read, then it carries the same YAML
    frontmatter / include pattern and follows the existing Codex
    wrapper convention.
  - Given `speccy-cli/tests/skill_packs.rs::SKILL_NAMES` after this
    task, when inspected, then the slice contains
    `"speccy-brainstorm"` as an element. The total length grows from
    7 to 8.
  - Given `speccy-cli/tests/init.rs::SKILL_NAMES` after this task,
    when inspected, then the array contains `"speccy-brainstorm"` as
    an element and its declared length type updates from
    `[&str; 7]` to `[&str; 8]`.
  - Given `cargo test --workspace`, when run after this task, then
    every skill-pack assertion that iterates `SKILL_NAMES` (presence,
    YAML frontmatter shape, rendered-byte-identity, host-pack
    coverage) passes for `speccy-brainstorm` without any code change
    in the renderer or `init` command.
</task-scenarios>

- Retry: style blocked on two byte-shape drifts from the established
  seven-wrapper pattern. (1) Both new `.tmpl` files end with `%}\n`
  (trailing newline after the close of the include directive); every
  other wrapper under `resources/agents/.claude/skills/*/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/*/SKILL.md.tmpl` ends with `%}`
  (no trailing newline). The drift propagates through the renderer
  into the dogfood mirrors (`.claude/skills/speccy-brainstorm/SKILL.md`
  ends with `).\n\n` where the other six end with a single `\n`).
  Strip the trailing newline on both `.tmpl` files and re-eject.
  (2) Claude and Codex `description:` lines diverge — Claude says
  `before invoking /speccy-plan`, Codex says
  `before invoking speccy-plan`. Every other Claude/Codex wrapper pair
  is byte-identical in its description. Pick one form (drop the slash
  on the Claude side, or add it on the Codex side) and apply to both.
  Business, security, tests pass; note the non-blocking
  "exactly_seven" naming carryover at `speccy-cli/tests/skill_packs.rs`
  lines 1149/1175/1258/1284 that the tests reviewer flagged for a
  separate follow-up.

- Implementer note (session-2026-05-17-T002-rev3):
  - Completed: Resolved both style-blocker byte-shape drifts.
    (1) Stripped the trailing newline on both wrapper templates so
    `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl` and
    `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
    now end with bytes `64 22 20 25 7d` (`d" %}`) and no final `0a`,
    matching every sibling wrapper under
    `resources/agents/.claude/skills/*/SKILL.md.tmpl` and
    `resources/agents/.agents/skills/*/SKILL.md.tmpl` (verified via
    `[System.IO.File]::ReadAllBytes` byte-tail inspection across
    `speccy-plan`, `speccy-amend`, `speccy-tasks`, `speccy-work`).
    (2) Aligned the `description:` lines across hosts: dropped the
    leading `/` on the Claude side so both wrappers now read
    `before invoking speccy-plan on an unclear ask.` (bare form,
    matching every other Claude/Codex wrapper pair which uses bare
    skill names like `speccy-* skill`, `"write a spec"`, `"ship
    SPEC-NNNN"`). The two `.tmpl` files are now byte-identical (both
    592 bytes; confirmed via `SequenceEqual`). Re-ejected the dogfood
    mirrors with `cargo run --release -- init --force --host claude-code`
    and `cargo run --release -- init --force --host codex`; the four
    affected dogfood files (`.claude/skills/speccy-brainstorm/SKILL.md`,
    `.agents/skills/speccy-brainstorm/SKILL.md`, plus the refreshed
    `.claude/skills/speccy-plan/SKILL.md` and
    `.agents/skills/speccy-plan/SKILL.md` because the speccy-plan
    module body cascade-renders the same way) now end with a single
    `0a` trailing newline and `dogfood_outputs_match_committed_tree`
    passes.
  - Undone: (none)
  - Commands run:
    - `cargo build --release --bin speccy`
    - `.\target\release\speccy.exe next --kind implement --json`
    - `.\target\release\speccy.exe implement SPEC-0025/T-002`
    - `cargo run --release -- init --force --host claude-code`
    - `cargo run --release -- init --force --host codex`
    - `cargo test --workspace --no-fail-fast`
    - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
    - `cargo +nightly fmt --all --check`
    - `.\target\release\speccy.exe check SPEC-0025/T-002`
  - Exit codes: pass; pass; pass; pass; pass; pass (all suites green
    including the previously-failing
    `dogfood_outputs_match_committed_tree`); fail-local (43
    pre-existing `result_large_err` errors on
    `speccy-core::error::ParseError` — confirmed identical at baseline
    `c5b632c3` with a clean working tree via
    `git stash; cargo clippy ...; git stash pop`, so NOT introduced by
    this task and matching T-003's documented Windows-only toolchain
    drift); pass; pass.
  - Discovered issues: (none new; the local-only `result_large_err`
    clippy noise is the same one T-003's implementer note already
    flagged for a future spec touching `error.rs`).
  - Procedural compliance: (none)

- Review (business, pass): T-002's slice of REQ-001 lands cleanly. Both
  wrapper templates (`resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`)
  carry the `name: speccy-brainstorm` + descriptive `description:` +
  single `{% include "modules/skills/speccy-brainstorm.md" %}` shape
  that matches the existing speccy-* wrappers, satisfying REQ-001
  done-when items 2-3. `SKILL_NAMES` in
  `speccy-cli/tests/skill_packs.rs:181-190` and
  `speccy-cli/tests/init.rs:59-68` both include `"speccy-brainstorm"`
  with the type bump to `[&str; 8]`, satisfying done-when item 4. The
  Codex description correctly drops the leading slash on `speccy-plan`
  per Codex prefix conventions, aligning with REQ-002's CHK-002
  scenario. The renderer-internal test rename in `speccy-cli/src/render.rs`
  (`..._emits_seven_skills` → `..._emits_eight_skills`) falls outside
  the suggested-files list but is the mechanical consequence of growing
  the bundle and is transparently called out in the implementer note;
  no scope sneak-in. The byte-for-byte host-mirror rendering check
  (done-when item 5, REQ-001 CHK-001) is correctly punted to T-003's
  re-eject step. No non-goal violations: no new CLI command, no new
  SPEC.md section, no machine-enforced gate, no new test infrastructure
  beyond extending the existing enumeration. Tests pass workspace-wide
  (verified `cargo test --workspace` runs to 0 failures).

- Review (tests, pass): the two wrapper `.tmpl` files are exercised
  by real assertions, not mocks.
  `speccy-cli/tests/skill_packs.rs::t005_claude_code_wrapper_shape_and_body`
  (line 1183) and `t006_codex_wrapper_shape_and_body` (line 1289)
  iterate the now-8-entry `SKILL_NAMES`, read each
  `speccy-brainstorm/SKILL.md.tmpl` from disk, parse the frontmatter
  with `serde_saphyr`, assert `name == "speccy-brainstorm"`, require
  a non-empty single-line `description`, and require the body to
  equal exactly `{% include "modules/skills/speccy-brainstorm.md" %}`
  byte-for-byte. `shipped_skill_md_frontmatter_shape` (line 826) and
  `shipped_descriptions_natural_language_triggers` (line 871) layer
  on the `name == dir`, "use when" trigger-marker, and 500-char-cap
  checks. The integration tests `copy_claude_code_pack_skill_md` and
  `copy_codex_pack_skill_md` in `speccy-cli/tests/init.rs` drive
  `speccy init` against a real temp dir (not a stub) and assert
  that for every entry in `SKILL_NAMES` the rendered SKILL.md
  exists, frontmatter parses, name matches, and body is non-empty —
  a mutant that blanks the include, swaps the include path, or
  renames the wrapper dir would fail at least one assertion. The
  single expected failure (`dogfood_outputs_match_committed_tree`)
  is correctly scoped to T-003, which re-ejects and commits the
  rendered mirrors. Non-blocking maintainability nit:
  `speccy-cli/tests/skill_packs.rs` lines 1149, 1175, 1258, 1284
  still say "exactly_seven" in function names and assertion
  messages even though the comparator `SKILL_NAMES` is now 8
  entries — the test still passes correctly today, but a future
  failure prints a misleading count. The implementer note
  explicitly renamed the analogous `render.rs` tests
  (`..._emits_seven_skills` → `..._emits_eight_skills`) but missed
  these. Flag as follow-up; the contract for T-002 is exercised.

- Review (business, pass): rev3 closes the prior style blockers and
  REQ-001's wrapper + enumeration contract lands. Both wrappers at
  `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
  now carry the `name: speccy-brainstorm` frontmatter, a single-line
  `description:` naming the brainstorming purpose plus trigger phrases
  ("help me think about", "brainstorm with me", "I want to spec out X
  but I'm not sure where to start", "before invoking speccy-plan on an
  unclear ask"), and exactly one
  `{% include "modules/skills/speccy-brainstorm.md" %}` directive —
  satisfying REQ-001 done-when items 2 and 3, and `<behavior>` items
  1-3 (the host harness can match the skill against user invocations
  via the trigger-phrase-rich description). The Codex form correctly
  drops the leading slash on `speccy-plan` per Codex prefix conventions
  (alignment also enforced by REQ-002 CHK-002's terminal-action
  scenario for the Codex wrapper). `SKILL_NAMES` in
  `speccy-cli/tests/skill_packs.rs:181-190` (8 entries) and
  `speccy-cli/tests/init.rs:59-68` (`[&str; 8]`) both include
  `"speccy-brainstorm"`, satisfying done-when item 4 — the existing
  generic enumeration assertions (`t005_..._wrapper_shape_and_body`,
  `t006_..._wrapper_shape_and_body`, `copy_*_pack_skill_md`,
  `dogfood_outputs_match_committed_tree`) now extend coverage to the
  new skill without renderer or `init` command changes. Done-when item
  1 (module body) is correctly out of T-002's scope and was satisfied
  by T-001 rev3; done-when item 5 (host-mirror byte-identity via
  `cargo run -- init --force`) is correctly out of scope and was
  satisfied by T-003 + the T-002 rev3 re-eject. The renderer-internal
  test renames in `speccy-cli/src/render.rs`
  (`..._emits_seven_skills` → `..._emits_eight_skills`) are the
  mechanical consequence of growing the bundle and the implementer
  note discloses them transparently — no scope sneak-in. Non-goals
  fully respected: no CLI surface change, no enforcement, no new
  SPEC.md section, no on-disk brainstorm artifact, no
  `plan-greenfield.md`/`plan-amend.md` edits, no new template variable
  (the wrappers use no `{{ cmd_prefix }}` since the include line is
  static and the description is plain prose), no new test
  infrastructure beyond extending the existing enumeration. User story
  US-3 ("ship through the same `init`/`render_host_pack` pipeline,
  with the same per-host wrapper-template pattern") is served
  end-to-end. No open questions were silently resolved by this slice
  (both SPEC.md Open Questions are `[x]` skip and orthogonal to
  wrapper plumbing). Changelog row 2 (full pivot to standalone skill)
  is the framing the diff reflects, not the original Phase-1-inside-
  greenfield draft. The non-blocking "exactly_seven" naming carryover
  the tests reviewer flagged at lines 1149/1175/1258/1284 is a
  maintainability nit, not a business contract violation. T-002 is
  ready for `completed`.

- Review (security, pass): rev3 keeps the slice security-clean.
  Both `SKILL.md.tmpl` wrappers
  (`resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`)
  are now byte-identical at 592 bytes (verified via
  `[System.IO.File]::ReadAllBytes` + `SequenceEqual`, tail bytes
  `64 22 20 25 7d` — `d" %}`, no trailing `0a`), carrying only static
  YAML frontmatter (`name: speccy-brainstorm` + a hardcoded
  `description:` of brainstorm purpose / trigger phrases) and a
  single `{% include "modules/skills/speccy-brainstorm.md" %}`
  against a compile-time-embedded resource path. Aligning the
  Claude description (dropping the leading `/` on `speccy-plan`)
  removes a cross-host string mismatch but introduces no new
  surface — both descriptions are static literals in the YAML, not
  runtime-interpolated. The `SKILL_NAMES` `[&str; 7]` → `[&str; 8]`
  type bump at `speccy-cli/tests/init.rs:59` and the matching slice
  extension at `speccy-cli/tests/skill_packs.rs:189` are compile-time
  literals — no unsafe indexing, no length arithmetic. The rev3
  re-eject path (`cargo run --release -- init --force --host …`)
  exercises the same `render_host_pack` already covered by dozens
  of existing tests against a `HostChoice` enum value, not a string.
  No new dependencies, no secrets, no auth, no logging, no crypto,
  no network surface, no template-injection or path-traversal
  vector introduced. Nothing in rev3 exposes attack surface.

- Review (tests, pass): rev3's byte-shape fixes (trailing-newline
  strip + cross-host description alignment) do not weaken any
  test-coverage axis for T-002's slice — same five `SKILL_NAMES`-
  iterating assertions still drive the new wrapper. Re-verified
  byte-tail of both `.tmpl` files: `64 22 20 25 7d` (`d" %}`,
  no `0a`), both files now 592 bytes and byte-identical, which
  keeps `t005_claude_code_wrapper_shape_and_body` /
  `t006_codex_wrapper_shape_and_body` (skill_packs.rs:1183/1289)
  green because their body-equality check uses `body.trim()`
  against `format!("{{% include \"modules/skills/{verb}.md\" %}}")`
  (trim makes the assertion newline-tolerant). The aligned
  description (`Atomize a fuzzy ask ... before invoking speccy-plan
  on an unclear ask.`, 495 chars) satisfies the natural-language-
  triggers contract: `shipped_descriptions_natural_language_triggers`
  (skill_packs.rs:871) needs `len <= 500`, a `use when` substring
  (case-insensitive), and no `Phase <digit>` prefix — all three
  hold. The integration tests `copy_claude_code_pack_skill_md` /
  `copy_codex_pack_skill_md` (init.rs:291/348) still drive
  `speccy init` against a real temp dir and assert every
  `SKILL_NAMES` entry produces a rendered SKILL.md with parseable
  frontmatter, matching `name`, and non-empty body — a mutant that
  removes either `.tmpl`, swaps the include path, or renames the
  wrapper dir still fails at least one of these. The previously-
  failing `dogfood_outputs_match_committed_tree` (init.rs:623) is
  now green per the implementer note — `render_host_pack(...)` is
  re-asserted byte-for-byte against the committed dogfood mirrors
  with `fs_err::read_to_string`, not a mock. The "exactly_seven"
  naming carryover at skill_packs.rs:1149/1175/1258/1284 is still
  present and still non-blocking (same flag as the prior tests
  review; worth a FOLLOWUPS.md entry but not a T-002 retry blocker).
  No mock-the-system-under-test failure mode introduced; the
  contract for T-002 is exercised end-to-end.

- Review (style, pass): rev3 closes both prior blockers cleanly. (1)
  Trailing-newline drift fixed — both
  `resources/agents/.claude/skills/speccy-brainstorm/SKILL.md.tmpl`
  and `resources/agents/.agents/skills/speccy-brainstorm/SKILL.md.tmpl`
  now end with bytes `... 25 7d` (`} %}`) with no terminal `0a`,
  matching all seven sibling templates I byte-checked
  (`speccy-init`, `speccy-plan`, `speccy-tasks`, `speccy-work`,
  `speccy-review`, `speccy-ship`, `speccy-amend` — each ends with
  bytes `25 7d` on both Claude and Codex sides, and each
  Claude/Codex pair is byte-identical at equal lengths 342/392/373/
  542/588/334/485). The two new files are now byte-identical at
  592 bytes (`SequenceEqual` confirmed). (2) Cross-host description
  divergence fixed — both wrappers now read `... before invoking
  speccy-plan on an unclear ask.` (bare form), matching the
  established convention where every other Claude/Codex wrapper
  pair uses bare `speccy-*` skill names in `description:` (verified
  by `description:` grep across all 14 sibling `SKILL.md.tmpl`
  files — no slash-prefixed `/speccy-*` reference in any
  description). The downstream dogfood mirrors at
  `.claude/skills/speccy-brainstorm/SKILL.md`,
  `.agents/skills/speccy-brainstorm/SKILL.md`,
  `.claude/skills/speccy-plan/SKILL.md`, and
  `.agents/skills/speccy-plan/SKILL.md` all end with a single
  trailing `0a` (no double-blank), so the re-eject took correctly.
  Other style axes are clean: the `SKILL_NAMES` extensions at
  `speccy-cli/tests/skill_packs.rs:181-190` and
  `speccy-cli/tests/init.rs:59-68` insert `"speccy-brainstorm"`
  at the end of the list in matching position across both files;
  the `[&str; 8]` length bump in `init.rs` is the idiomatic way to
  grow a fixed-length array (no `&[&str]` slice drift across
  files); zero `#[allow]` / `#[expect]` suppressions added by this
  slice; the renderer-internal test renames in `speccy-cli/src/render.rs`
  (`..._emits_seven_skills` → `..._emits_eight_skills` plus
  `assert_eq!(.., 8, "... eight ...")`) thread the count through
  function name + literal + assertion message consistently;
  `cargo test --test skill_packs` runs 42/42 green. The
  "exactly_seven" function-name + assertion-message carryover at
  `speccy-cli/tests/skill_packs.rs` lines 1149/1175/1258/1284
  remains as the tests reviewer noted — the test still passes
  because the comparator is `SKILL_NAMES`, not the literal `7`,
  but a future failure would print a misleading count. It is
  already tracked as a known follow-up in the prior `Retry:` note
  and the tests-persona review; the implementer renamed the
  analogous `render.rs` pair but missed these. Not blocking on
  T-002 since the contract holds and the rename is mechanical;
  worth a FOLLOWUPS.md entry alongside the other cross-task style
  carryovers if not picked up in the next slice.

</task>

## Phase 3: Cross-reference speccy-plan + dogfood re-eject

<task id="T-003" state="completed" covers="REQ-003">
Update speccy-plan skill body, re-eject dogfood mirrors, and run hygiene

- Suggested files: `resources/modules/skills/speccy-plan.md`,
  `.claude/skills/speccy-plan/SKILL.md`,
  `.agents/skills/speccy-plan/SKILL.md`,
  `.claude/skills/speccy-brainstorm/SKILL.md`,
  `.agents/skills/speccy-brainstorm/SKILL.md`

- Implementer note (session-2026-05-17-T003-rev2):
  - Completed: Rewrote `resources/modules/skills/speccy-plan.md` to (a) reference `{{ cmd_prefix }}speccy-brainstorm` as a recommended precursor when the ask is fuzzy; (b) drop user-visible "greenfield" framing (renamed to "new-spec form"); (c) remove the stale "the prompt inlines `AGENTS.md`" and "inlines the nearest parent `MISSION.md`" wording, replaced with accurate descriptions ("the host harness auto-loads `AGENTS.md`" and "the prompt names the nearest parent `MISSION.md` path"). Ran `cargo run -- init --force --host claude-code` and `cargo run -- init --force --host codex` to re-eject host-local dogfood mirrors. The two new files at `.claude/skills/speccy-brainstorm/SKILL.md` and `.agents/skills/speccy-brainstorm/SKILL.md` were created; the existing seven `speccy-*` SKILL.md files were overwritten. Verified the speccy-brainstorm skill loads in the host's skill list. Added SPEC-0025 entry to `speccy-core/tests/fixtures/in_tree_id_snapshot.json` so the `every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot` test passes (the snapshot fixture is per-spec and grows when a new spec is added).
  - Undone: (none)
  - Commands run:
    - `cargo run -- init --force --host claude-code`
    - `cargo run -- init --force --host codex`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
    - `cargo +nightly fmt --all --check`
    - `cargo deny check`
  - Exit codes: pass; pass; pass (all 357+ tests); fail-local (pre-existing 43 `result_large_err` clippy errors on `speccy-core` that fire locally but CI passes on the same exact commits — Windows vs Linux clippy heuristic / toolchain mismatch; NOT introduced by this task, NOT introduced by SPEC-0025 at all); pass; pass (warning about duplicate `winnow` crate version is informational and pre-existing).
  - Discovered issues: Pre-existing local-only clippy failure on `result_large_err` for `speccy-core::error::ParseError`. CI is green on baseline + this branch; the failure shows up only in the local Windows toolchain. Out of scope for SPEC-0025; flagging here for a future spec that touches `error.rs`. Also: the dogfood test `dogfood_outputs_match_committed_tree` and the `in_tree_specs` snapshot fixture are both architectural choke points that fail loudly when a new shipped skill is added; both required fixture / re-ejection passes during this task. The pattern is correct but the failure messages could be clearer about which workflow corrects them.
  - Procedural compliance: (none)

- Review (security, pass): Documentation-only diff — skill body
  markdown plus re-ejected host wrappers; no code paths, no new
  dependencies, no secrets, no auth or input-validation surface. The
  brainstorm skill's prose "hard gate" is feedback-not-enforcement by
  explicit design (SPEC DEC-003) and the "skip the brainstorm" bypass
  is an accepted user-controlled escape hatch, not a vulnerability.
  Agent-Read pointers only name already-in-repo files (`AGENTS.md`,
  `.speccy/specs/`) that prior skills already reach for. No findings.

<task-scenarios>
  - Given `resources/modules/skills/speccy-plan.md` after this task,
    when its "When to use" section is read, then it names
    `{{ cmd_prefix }}speccy-brainstorm` as a recommended precursor
    for fuzzy asks (one-line note: when the framing is not yet
    agreed, run brainstorm first).
  - Given the same file, when grep'd case-sensitively for the
    literal strings `inlines \`AGENTS.md\``, `inlines AGENTS.md`,
    `inlines the nearest parent \`MISSION.md\``, `inlines \`MISSION.md\``,
    `inlines MISSION.md`, then no hit is returned. Any equivalent
    claim that the rendered prompt embeds those bodies is removed.
  - Given the amendment-form description in the same file, when
    read, then it remains a single-pass surgical edit description
    without a mandatory brainstorm step (consistent with the SPEC's
    non-goal that brainstorm is recommendation, not requirement).
  - Given the host-local dogfood files at
    `.claude/skills/speccy-brainstorm/SKILL.md` and
    `.agents/skills/speccy-brainstorm/SKILL.md` after this task,
    when read, then they exist and match the rendered output of
    `render_host_pack` byte-for-byte (the repo has been re-ejected
    via `cargo run -- init --force --host claude-code` and
    `cargo run -- init --force --host codex`).
  - Given the host-local dogfood files at
    `.claude/skills/speccy-plan/SKILL.md` and
    `.agents/skills/speccy-plan/SKILL.md` after this task, when
    read, then they reflect the updated `speccy-plan` module body
    (i.e., they reference `speccy-brainstorm` and no longer carry
    the retired "inlines AGENTS.md" / "inlines MISSION.md" claims).
  - Given `cargo test --workspace`, when run after this task, then
    every test passes — including the `dogfood_outputs_match_committed_tree`
    test which compares `render_host_pack` output to the committed
    `.claude/skills/` and `.agents/skills/` mirrors.
  - Given `cargo clippy --workspace --all-targets --all-features --
    -D warnings`, when run after this task, then no warnings are
    reported.
  - Given `cargo +nightly fmt --all --check`, when run after this
    task, then no formatting drift is reported.
</task-scenarios>

- Review (style, pass): T-003's direct authorship (`resources/modules/skills/speccy-plan.md`) is style-clean — `{{ cmd_prefix }}` used consistently throughout, no stale "inlines AGENTS.md / MISSION.md" wording, two-form structure preserved, matches the wrap and indent of sibling module skill bodies (`speccy-tasks.md`, `speccy-amend.md`). One pre-existing drift surfaces in T-003's re-ejected output but originates in T-001's module body: `resources/modules/skills/speccy-brainstorm.md:64` hard-codes `/speccy-plan` (Claude-style prefix) instead of `{{ cmd_prefix }}speccy-plan`, propagating to `.agents/skills/speccy-brainstorm/SKILL.md:68` as a literal `/speccy-plan` under a Codex prefix that should drop the slash. Every other reference in that file uses the template variable. Out of scope to block T-003 on; flag for T-001 retry or a follow-up fix.

- Review (business, pass): REQ-003 lands cleanly. The speccy-plan
  module body (`resources/modules/skills/speccy-plan.md:16-19`) names
  `{{ cmd_prefix }}speccy-brainstorm` as a recommended precursor for
  fuzzy asks (done-when #1, CHK-003 first scenario). Grep over the
  module body and both dogfood mirrors (`resources/modules/skills/speccy-plan.md`,
  `.claude/skills/speccy-plan/SKILL.md`, `.agents/skills/speccy-plan/SKILL.md`)
  returns zero hits for "inlines `AGENTS.md`", "inlines AGENTS.md",
  "inlines the nearest parent `MISSION.md`", or "inlines `MISSION.md`"
  (done-when #2, CHK-003 second scenario); replaced with the accurate
  "the host harness auto-loads `AGENTS.md`" / "names the nearest
  parent `MISSION.md` path" wording per SPEC-0023 REQ-005/REQ-006. The
  amendment-form description (`resources/modules/skills/speccy-plan.md:20-21,40-43`)
  remains a single-pass surgical edit with no mandatory brainstorm
  step, and the brainstorm reference is correctly scoped to the
  new-spec branch only (done-when #3, CHK-003 third scenario; aligns
  with the non-goal "No special-casing for the amendment form" and
  Open Question #2 resolved as **skip**). Both wrapper templates at
  `resources/agents/.claude/skills/speccy-plan/SKILL.md.tmpl` and
  `resources/agents/.agents/skills/speccy-plan/SKILL.md.tmpl`
  delegate to the module body via `{% include %}` and inherit the
  fix; their static `description:` ("Draft a new Speccy SPEC from the
  `AGENTS.md` product north star") does not contradict the
  brainstorm-as-precursor recommendation and does not claim AGENTS.md
  or MISSION.md is inlined (done-when #4, CHK-003 fourth scenario).
  Re-eject produced the host-local dogfood mirrors at
  `.claude/skills/speccy-brainstorm/SKILL.md`,
  `.agents/skills/speccy-brainstorm/SKILL.md`, plus the refreshed
  `.claude/skills/speccy-plan/SKILL.md` and
  `.agents/skills/speccy-plan/SKILL.md`, and the
  `dogfood_outputs_match_committed_tree` test passes (verified locally
  via `cargo test --workspace` — 0 failures across the entire suite).
  Goals met (new shipped skill exists end-to-end via the T-001+T-002+
  T-003 chain; speccy-plan body updated; test corpus extended via the
  `in_tree_id_snapshot.json` per-spec extension). Non-goals respected:
  no CLI surface change (only test-fixture text and test-internal
  renames in `speccy-cli/src/render.rs` to match the new bundle
  count), no enforcement, no new SPEC.md section, no on-disk
  brainstorm artifact, no `plan-greenfield.md`/`plan-amend.md` edits.
  Both Open Questions are resolved (`[x]`) and the resolutions match
  the SPEC's stated "skip" positions and the Changelog row dated
  2026-05-17 — no silent question-resolution drift. The user stories,
  particularly US-4 ("As a user reading `speccy-plan.md`, I want the
  plan skill to point at `/speccy-brainstorm` as the recommended
  precursor when the ask is fuzzy"), are served. The pre-existing
  local-only `result_large_err` clippy noise and the Codex prefix
  leak at `.agents/skills/speccy-brainstorm/SKILL.md:68` are outside
  T-003's scope (the latter originates in T-001's module body and is
  already flagged by the style reviewer).

- Review (tests, pass): the four byte-identity slice scenarios (D-E,
  dogfood mirrors of `speccy-brainstorm/SKILL.md` and
  `speccy-plan/SKILL.md` for both hosts) are exercised by a real
  assertion, not a mock —
  `speccy-cli/tests/init.rs::dogfood_outputs_match_committed_tree`
  (line 623) calls `render_host_pack(HostChoice::ClaudeCode)` /
  `(HostChoice::Codex)` against disk, then `assert_eq!`s every
  `RenderedFile.contents` byte-for-byte against the committed
  `.claude/skills/...` and `.agents/skills/...` files via
  `fs_err::read_to_string`. Verified locally: that test passes (1/1
  green), and the full skill_packs.rs suite passes (34/34, including
  `t005_claude_code_wrapper_shape_and_body` /
  `t006_codex_wrapper_shape_and_body` / `recipe_content_shape` over
  the now-8-entry `SKILL_NAMES`). A mutant that drops the brainstorm
  reference from `resources/modules/skills/speccy-plan.md` without
  re-ejecting (or vice versa) would fail the byte-identity assertion
  at `init.rs:646`; spot-checked the committed mirrors and confirmed
  both `.claude/skills/speccy-plan/SKILL.md:21` and
  `.agents/skills/speccy-plan/SKILL.md:21` carry the brainstorm
  pointer, and a case-sensitive grep for the five retired "inlines …"
  substrings returns zero hits in
  `resources/modules/skills/speccy-plan.md`. The hygiene scenarios F-H
  (`cargo test --workspace`, `cargo clippy ... -D warnings`,
  `cargo +nightly fmt --all --check`) are gates rather than assertions
  but the implementer note's exit codes are consistent with what I
  observe locally (test + fmt + deny pass; clippy is the pre-existing
  Windows-only `result_large_err` noise, not introduced here). Soft
  gap, non-blocking: scenarios A-C (positive content checks:
  speccy-plan.md references brainstorm; no "inlines AGENTS.md"
  substring; amendment-form remains surgical) and the user-facing
  CHK-003 grep assertions are NOT codified as direct content-greps in
  `speccy-cli/tests/` — they ride entirely on the dogfood byte-identity
  catch-net, which fails on one-sided drift but would let a coordinated
  drift through (mutate both the resource module and the dogfood
  mirror, all tests pass). Matches the established Speccy convention
  for shipped-skill body slices and the SPEC's non-goal of "No new test
  infrastructure beyond the existing skill-pack and init enumeration
  assertions"; same pattern T-002's tests-reviewer already accepted
  for the wrapper-content case. Worth a FOLLOWUPS.md entry if
  dogfooding shows skill-body content drifting silently, not a blocker
  for this slice.

</task>

</tasks>
