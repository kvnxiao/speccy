---
spec: SPEC-0027
outcome: delivered
generated_at: 2026-05-18T00:50:34Z
---

# Report: SPEC-0027 host-native-personas

<report spec="SPEC-0027">

## Outcome

delivered

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
`speccy-cli/src/init.rs::build_plan` no longer appends
`.speccy/skills/personas/` or `.speccy/skills/prompts/` items — both
`append_user_tunable_dir_items` call sites are gone along with the
helper itself (`append_user_tunable_dir_items`,
`collect_bundle_files`, `has_md_extension`) and the imports of
`speccy_core::personas::PERSONAS`, `speccy_core::prompt::PROMPTS`,
`include_dir::Dir`, and `std::path::Component`. Three new T-003
tests under `speccy-cli/tests/init.rs` pin the contract:
`t003_init_does_not_create_speccy_skills_dir` asserts no
`.speccy/skills/` directory is created on a fresh init;
`t003_init_plan_summary_does_not_mention_speccy_skills` asserts
stdout contains no `.speccy/skills/` substring;
`t003_init_force_preserves_pre_existing_speccy_skills_overrides`
asserts a pre-populated override survives `init --force` byte-for-byte
(DEC-003: init writes nothing into the subtree and deletes nothing
from it). T-005 closes the dogfood half: `git rm -rf .speccy/skills/`
removed the 20 in-tree files, and re-running
`cargo run -- init --force` against both `--host claude-code` and
`--host codex` produces zero `.speccy/skills/` paths in the plan
output. Verified live during this ship pass:
`0 created, 9 overwritten, 6 skipped` with no `.speccy/skills/`
lines anywhere.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
`speccy-cli/src/init.rs::is_host_native_reviewer_file` matches
`.claude/agents/reviewer-<persona>.md` and
`.codex/agents/reviewer-<persona>.toml` for every persona in
`speccy_core::personas::ALL` (path normalised across backslashes for
Windows hosts), and the classifier wire-up in `append_host_pack_items`
collapses `Action::Overwrite | Action::Skip` → `Action::Skip` on
existence while keeping `Action::Create` on absence. All other
host-pack files (skill wrappers under `.claude/skills/`,
`.agents/skills/`, and the `.speccy/speccy.toml` write) flow through
the unchanged `classify(&destination)` and remain Create-or-Overwrite.
Four new T-002 tests in `speccy-cli/tests/init.rs` cover the
contract: sentinel preservation under `init --force` against
`.claude/agents/reviewer-business.md`; deletion + recreation from the
shipped bundle; plan-summary label split (every reviewer-agent path
shows `skip` while every skill-wrapper path shows `overwrite`); and
the Codex twin covering both the sentinel-preserve and delete-then-
recreate scenarios against `.codex/agents/reviewer-business.toml`.
Empirical proof during this ship pass: `init --force --host claude-code`
prints six `skip .claude/agents/reviewer-*.md` lines paired with
eight `overwrite .claude/skills/speccy-*/SKILL.md` lines.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
The six `resources/modules/prompts/reviewer-<persona>.md` templates
have zero matches for `## Persona` and `{{persona_content}}` (verified
by grep across all six). `speccy-cli/src/review.rs` drops the
`persona_content` vars insert, the `resolve_persona_file` call site,
and the `PersonaError` / `resolve_file as resolve_persona_file`
imports; the `ReviewError::Persona(#[from] PersonaError)` variant is
replaced by an inline `ReviewError::UnknownPersona { name, valid }`
that preserves the user-facing `"valid personas: …"` diagnostic
emitted from `speccy-cli/src/main.rs`. The strengthened
`prompt_renders_substitutes_every_placeholder` test in
`speccy-cli/tests/review.rs` asserts the absence of `## Persona`,
`# Reviewer Persona:`, and `{{persona_content}}` against a synthetic
task entry, and the deleted `prompt_renders_picks_up_project_local_persona_override`
test removes the override-resolution contract from the suite. One
caveat surfaced in T-001's business review: the REQ-003 done-when
bullet "stdout contains no occurrence of `# Reviewer Persona:
Business`" reads literally as a property of the entire stdout, which
includes the verbatim `<task>` element copied from TASKS.md. When the
task entry itself happens to quote the retired strings descriptively
(as T-001's does), the literal check would fire — the test correctly
captures the contract's intent via a synthetic fixture, but the
SPEC's wording could be tightened to "the rendered template body"
rather than "the rendered stdout" in a future amendment.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
`speccy-core/src/personas.rs` is now a registry-only module. The
`PERSONAS` static, `PersonaError` enum, `resolve_file`,
`resolve_file_with_warn`, and `persona_file_name` helper are all
deleted along with the `camino::Utf8Path`, `include_dir::{Dir, include_dir}`,
`std::io::Write`, and `thiserror::Error` imports they consumed. The
module-level doc comment is rewritten to describe the surface as the
persona-name registry and to note that body content reaches sub-agents
via the host-pack Jinja `{% include %}` into `.claude/agents/reviewer-*.md`
(or the Codex equivalent), loaded by the host as system context. `ALL`
survives unchanged, declaring `business, tests, security, style,
architecture, docs` in that order. `speccy-core/tests/personas.rs`
retains the three registry tests
(`registry_contains_six_personas_in_declared_order`,
`registry_default_personas_is_first_four_prefix`,
`registry_personas_are_unique`) and drops the seven resolver-chain
tests enumerated in REQ-004's done-when. `speccy-cli/tests/skill_packs.rs`
and `speccy-cli/tests/init.rs` were retargeted to read body bytes via
`speccy_cli::embedded::RESOURCES.get_file(…)` so the legacy-needle
sweep (`persona_and_prompt_sources_have_no_legacy_check_authoring_examples`)
keeps working without re-exporting a deleted symbol.
</coverage>

## Task summary

Five tasks (T-001..T-005) shipped in declared order. All landed on
the first review pass — zero retries across the spec. No SPEC
amendment was triggered during the loop. Phase ordering held:
T-001 dropped the rendered persona body and the
`{{persona_content}}` insertion; T-002 added the host-native
Skip-on-exists classifier; T-003 stopped `init` from writing
`.speccy/skills/`; T-004 deleted the resolver chain; T-005
deleted the in-tree `.speccy/skills/` tree as dogfood proof.

## Out-of-scope items absorbed

- T-001 fixed a pre-existing missing `# Report:` heading in
  `.speccy/specs/0026-skill-router-anti-triggers/REPORT.md`. The
  defect shipped on `main` at commit 68f6fcf and was caught by the
  `every_in_tree_report_md_parses_and_resolves_against_parent_spec`
  test the moment a SPEC-0027 entry pushed the runtime to re-scan
  the report tree. Fixed by inserting
  `# Report: SPEC-0026 skill-router-anti-triggers` between the
  frontmatter close fence and the opening `<report>` tag — not a
  SPEC-0027 concern, but unblocking T-001's test run required it.
- T-001 also regenerated the in-tree dogfood prompt mirrors at
  `.speccy/skills/prompts/reviewer-*.md` to match the new
  bundle, keeping the `dogfood_outputs_match_committed_tree`
  walker green between T-001 and T-005 (the walker contract for
  prompts is removed in T-003, the in-tree files in T-005). This
  was a deliberate keep-CI-green plumbing step rather than a
  scope expansion.
- T-003 left `PERSONAS`/`PROMPTS` imports in the *test* file
  (`speccy-cli/tests/init.rs`) under a SPEC-0027 comment because
  the legacy-needle sweep
  (`persona_and_prompt_sources_have_no_legacy_check_authoring_examples`)
  still consumed `PERSONAS` until T-004's deletion landed. T-004
  removed both imports cleanly. The accommodation is documented
  in both implementer notes.
- T-005 caught a spec-vs-reality drift the SPEC itself missed:
  `### Interfaces` enumerated 18 files under "Files deleted
  (in-tree dogfood)", but the on-disk tree had 20 — the two
  extras (`.speccy/skills/personas/implementer.md` and
  `…/planner.md`) served the same dogfood-override role as the
  reviewer personas. `git rm -rf .speccy/skills/` removed all 20.
  REQ-001 is normative on "the directory is gone," not on file
  count, so the call is consistent; see the deferred item below.

## Skill updates

(none)

## Deferred / known limitations

- SPEC.md `### Interfaces` undercounts the deleted files by two
  (18 listed, 20 on disk). The discrepancy did not block ship —
  REQ-001 is what's normative — but the SPEC text should be
  tightened ("or relaxed to 'the entire `.speccy/skills/` subtree
  including but not limited to these files'") in a follow-up
  amendment so future audits don't trip on the same drift.
- REQ-003's "stdout contains no `# Reviewer Persona: Business`"
  bullet is literally true of the rendered template body but
  trips on task entries that quote the retired strings
  descriptively (T-001's own task entry is the worst-case
  example). The strengthened test fixture handles this correctly;
  a future SPEC amendment could tighten the wording to "the
  rendered template body" rather than "the rendered stdout".
- A pre-existing `clippy::result_large_err` against
  `speccy-core::error::ParseError` (largest variant ≥128 bytes,
  42+ call sites) remains across the workspace. It was
  inherited from before SPEC-0026 T-003 (which explicitly pinned
  it as out-of-scope) and is not regressed by SPEC-0027. Carried
  forward to whichever future SPEC chooses to box the parse
  error.
- `MtimeDrift` continues to fire on shipped SPECs because
  `speccy-ship` flips `status: implemented` in SPEC.md *after*
  TASKS.md is final, leaving SPEC.md's mtime > TASKS.md's mtime
  even though the hash matches (SPEC-0024 excluded `status` from
  the hash scope). This is structurally orthogonal to SPEC-0027
  but surfaced during the loop. Worth a dedicated SPEC that
  either gates `MtimeDrift` on `HashDrift` or retires the signal
  entirely now that the hash is meaningful.

</report>
