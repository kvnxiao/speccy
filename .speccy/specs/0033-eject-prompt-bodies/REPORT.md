---
spec: SPEC-0033
outcome: satisfied
generated_at: 2026-05-20T08:30:00Z
---

# REPORT: SPEC-0033 Eject phase prompt bodies into skill files; CLI does state only, no natural-text rendering

The CLI's two architectural jobs — mechanical state queries vs. authored
prompt rendering — are now fully decoupled. Five prompt-rendering verbs
(`plan`, `tasks` render-form, `implement`, `review`, `report`) and the
`trim_to_budget` mechanism are removed. Phase prompt bodies eject into the
host skill pack at `speccy init` time via a per-file three-way classification
(create / unchanged / refuse-or-overwrite). Two new flat verbs (`lock`,
`vacancy`) carry the real-CLI-work that previously hid inside the deleted
rendering paths. `status` and `next` JSON envelopes bump to `schema_version:
2` with resolved paths plus derived `next_action`. Final CLI surface is
seven flat verbs, each doing one job, no mode flags.

<report>

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
Five prompt-rendering verbs and the `trim_to_budget` mechanism deleted from
the CLI. `speccy --help` now lists exactly seven subcommands
(`init`, `status`, `next`, `check`, `verify`, `lock`, `vacancy`). The
`resources/modules/prompts/` directory was deleted as part of ship
cleanup — T-001's implementer-note claimed deletion but the working tree
still carried the 12 files; the ship PR removes them per REQ-001 done-when
bullet 6.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
`speccy lock SPEC-NNNN` exists at `speccy-cli/src/lock.rs` with the
SPEC-named signature. It reuses `speccy_core::tasks::commit_frontmatter`
unchanged (DEC-006). Hash-and-rewrite happy path, missing SPEC, and SPEC.md
parse-failure paths all covered by integration tests in
`speccy-cli/tests/lock.rs` (8 tests).
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
`speccy vacancy [--json]` exists at `speccy-cli/src/vacancy.rs`. Reuses
`speccy_core::prompt::allocate_next_spec_id` unchanged — the open question
about relocating the function is resolved in favor of keeping it in
`prompt::` for v1, recorded in the module-level doc comment. Seven
integration tests in `speccy-cli/tests/vacancy.rs` cover flat / mission /
empty / no-workspace paths plus the bare-text and `--json` envelope shapes.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007 CHK-008">
`--kind` flag removed from `speccy next`; replaced with a positional
`spec_id` and the derived priority rule `decompose > review > implement >
ship`. Eleven integration tests in `speccy-cli/tests/next_derived.rs`
cover in-review priority, implement-after-review-done, decompose-when-no-
TASKS.md, completed+REPORT.md null path, and `--kind` clap rejection. The
legacy `KindFilter` / `compute` / `NextResult` API was removed in T-010
and `next_priority.rs` migrated to the new `compute_for_spec` /
`compute_workspace` API.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-009 CHK-010">
Both `speccy status --json` and `speccy next --json` envelopes bumped to
`schema_version: 2`. Per-spec entries carry `spec_md_path`,
`tasks_md_path` (nullable), `mission_md_path` (nullable) as repo-relative
forward-slash strings. Path resolution lives in
`speccy_core::workspace::resolve_mission_md_path`; the JSON-serialization
layer only formats paths that the scanner already resolved. Five
integration tests in `speccy-cli/tests/status_paths.rs` cover the flat-
spec, mission-folder, and TASKS.md-absent paths. The `to_repo_relative`
helper was de-duplicated into `speccy-cli/src/paths.rs` during the T-005
retry round.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-011 CHK-017 CHK-019 CHK-020 CHK-021 CHK-022">
`speccy init` replaces SPEC-0027's Skip-on-exists semantic with the
three-way classification (create / unchanged / refuse-or-overwrite, with
`--force` overriding the refuse path). Interactive skills (`speccy-init`,
`speccy-brainstorm`, `speccy-plan`, `speccy-amend`, `speccy-review`
orchestrator) eject as full-body SKILL.md only. The three pinned phase
workers (`speccy-tasks`, `speccy-work`, `speccy-ship`) eject as thin
SKILL.md stubs (≤10 non-blank lines, no `context:`/`agent:`/`model:`/
`effort:` frontmatter) plus full-body agent files at
`.claude/agents/speccy-<phase>.md` (`model: sonnet[1m]`, `effort: medium`)
or `.codex/agents/speccy-<phase>.toml` (`model = "gpt-5.5"`,
`model_reasoning_effort = "medium"`). No `speccy-init` or `speccy-review`
agent files ship per DEC-008. Integration coverage in
`speccy-cli/tests/init_three_way.rs` (11 tests) and
`speccy-cli/tests/init_phase_agents.rs` (10 tests).
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-013 CHK-018">
Shared reviewer-persona blocks (verdict-return contract, TASKS.md write
prohibition, inline note format, diff-fetch command) factor into four
topic-named snippet files co-located in `resources/modules/personas/`
(`verdict_return_contract.md`, `no_tasks_md_writes.md`,
`inline_note_format.md`, `diff_fetch_command.md`). The six reviewer
persona body files stay independent and include the snippets via
MiniJinja `{% include %}` at build time; per-persona divergence
(`reviewer-style`'s "Diff-format pitfalls", `reviewer-tests`'s Evidence-
read step) is preserved. No `_partials/` subdirectory exists at any
level. No master template file. Integration coverage in
`speccy-cli/tests/persona_snippets.rs` (11 tests).
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-014 CHK-015">
All shipped skill and phase bodies discover speccy resources via the
CLI's JSON envelopes (`speccy status --json`, `speccy next --json`,
`speccy vacancy --json`) rather than direct filesystem patterns. The
`speccy-plan` greenfield path invokes `speccy vacancy --json` (not
`speccy status --json`) per the payload-minimization mandate. General-
purpose Read/Glob/grep against non-speccy paths (AGENTS.md, Cargo.toml,
source code, etc.) is preserved as a non-violation — the boundary is
speccy-resource-scoped, not blanket filesystem access. Integration
coverage in `speccy-cli/tests/skill_body_discovery.rs` (4 tests).
</coverage>

</report>

## Retry counts

- T-005: 1 retry (style — `to_repo_relative` helper duplication; resolved
  by extracting `speccy-cli/src/paths.rs`).
- T-007: 1 retry (tests — initial pass edited rendered host-pack files
  without editing source modules under `resources/modules/`; retry edited
  the seven source files directly and restored T-006's
  `{% set persona_name %}` + `{% include %}` directives that had been
  lost in the persona source state at HEAD).
- T-008: 1 retry (style — stale doc comment on
  `append_speccy_examples_items` referencing the deleted
  `classify(&destination)` function; `#![allow]` vs `#![expect]`
  mismatch on `clippy::panic_in_result_fn`).

All other tasks (T-001, T-002, T-003, T-004, T-006, T-009, T-010) passed
their first review round without retry.

## Out-of-scope items absorbed

- T-001 deferred tightening the `SPECCY_COMMANDS` constant in
  `speccy-cli/tests/skill_packs.rs` to T-007/T-008/T-010 because the
  list is used as a substring matcher inside SKILL.md bodies that had
  not yet been rewritten. T-007 tightened the list to the current
  seven-verb surface (removed deleted verbs, added `lock`/`vacancy`).
- T-006 incidentally fixed two structural bugs in TASKS.md (missing
  `</implementer-note>` close tag on the first T-005 implementer-note;
  invalid `addendum="true"` attribute on the second T-005 note) that
  blocked the speccy CLI's TASKS.md parser. Honestly disclosed in the
  implementer-note.
- T-007's retry restored T-006's `{% include %}` directives that had
  been lost in the persona source state at HEAD — corrective scope,
  documented in the retry implementer-note.
- T-009 fixed stale `--kind implement` and `--kind review` references
  in four SKILL.md.tmpl `description:` frontmatter fields (left behind
  by T-004's `--kind` removal). Procedural-compliance fix per AGENTS.md.
- T-009 fixed a TASKS.md missing-`</implementer-note>` close tag on
  the first T-008 implementer-note that blocked the parser.
- T-010 updated a stale `modules/prompts/<name>.md` reference in the
  `speccy-cli/src/embedded.rs` doc comment to point at the current
  `modules/phases/` subtree.
- Ship-time cleanup: deleted `resources/modules/prompts/` (12 files)
  that T-001's implementer-note claimed to remove but the working tree
  still carried, satisfying REQ-001 done-when bullet 6.

## Open questions

Four QST-001 unchecked open questions remain in SPEC.md:

1. Whether `speccy vacancy`'s implementation reuses
   `allocate_next_spec_id` directly or relocates the function. Resolved
   at implementation time in T-003: stays in `speccy_core::prompt::`,
   no `speccy_core::specs` module. Documented in the
   `speccy-cli/src/vacancy.rs` module doc comment.

2. Exact filename convention for shared snippets in
   `modules/personas/` and `modules/phases/`. Resolved at
   implementation time in T-006: topic-named (`verdict_return_contract.md`,
   `no_tasks_md_writes.md`, `inline_note_format.md`,
   `diff_fetch_command.md`).

3. Whether `--force` stdout summary lists each overwritten file
   individually or just a tally. Resolved at implementation time in
   T-008: per-file logging with `(!) overwritten` marker matching the
   SPEC-level position.

4. Whether the `speccy-review` orchestrator's body source belongs in
   `resources/modules/skills/` or elsewhere. Resolved at implementation
   time: stays in `resources/modules/skills/speccy-review.md`, consistent
   with other interactive skills under DEC-008.

These questions are info-level lints only and do not block ship per
AGENTS.md "Feedback, not enforcement." Leaving them as `[ ]` in SPEC.md
preserves the historical record of which decisions were left to the
implementation phase.
