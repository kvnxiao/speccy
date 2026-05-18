---
spec: SPEC-0027
spec_hash_at_generation: acdb32cbaea84b3be978bc1be61366ae2fec5fe21405753d357d0022cd467657
generated_at: 2026-05-18T00:01:22Z
---

# Tasks: SPEC-0027 Host-native files are the sole canonical persona surface; drop .speccy/skills/ override directory

<tasks spec="SPEC-0027">

## Phase 1: Drop the persona body from the CLI-rendered reviewer prompt

<task id="T-001" state="completed" covers="REQ-003">
## T-001: Strip `{{persona_content}}` from reviewer prompt templates and `speccy review`'s render-vars map

- Review (business, pass): REQ-003 satisfied. The six
  `resources/modules/prompts/reviewer-*.md` templates have zero
  matches for `## Persona` and `{{persona_content}}` (verified by
  grep). `speccy-cli/src/review.rs:108` replaces the
  `ReviewError::Persona(PersonaError::UnknownName{..})` construction
  with the inline `ReviewError::UnknownPersona { name, valid }`
  variant, dropping the `vars.insert("persona_content", ...)` line
  and the `resolve_persona_file` call; `speccy-cli/src/main.rs:391`
  pattern-matches the new variant and preserves the
  `"valid personas: …"` user-facing diagnostic verbatim. The
  strengthened `prompt_renders_substitutes_every_placeholder` test
  in `speccy-cli/tests/review.rs:203-238` asserts the absence of
  `## Persona`, `# Reviewer Persona:`, and `{{persona_content}}` on
  a synthetic task entry that doesn't itself mention the retired
  strings — the test correctly captures REQ-003's intent. Note: the
  done-when bullet "output of speccy review … contains no occurrence
  of `# Reviewer Persona: Business`" reads literally as a property of
  the entire stdout, which includes the verbatim `<task>` element
  copied from TASKS.md; this particular T-001's task entry quotes the
  retired strings descriptively (see lines 31, 34, 43, 55, 69-79, 106,
  117-119, 122, 133 of the rendered prompt). That tension is benign —
  the contract is about the template not carrying the persona body,
  and the template change is clean — but worth surfacing for future
  spec wording. Slice-level `<task-scenarios>` all pass; `cargo test
  --workspace` exit 0. Non-goals respected: `personas::resolve_file`
  and the override resolver chain remain in place (correctly deferred
  to T-004), and `speccy init`'s plan is untouched (correctly
  deferred to T-002/T-003).

- Implementer note (session-2026-05-17-T001):
  - Completed: Removed the `## Persona` heading and `{{persona_content}}` line from all six `resources/modules/prompts/reviewer-<persona>.md` templates (business, tests, security, style, architecture, docs) — surrounding sections byte-identical. Edited `speccy-cli/src/review.rs`: dropped `PersonaError` and `resolve_file as resolve_persona_file` imports, dropped the `resolve_persona_file` call, dropped the `persona_content` insert from the `vars` map, replaced `ReviewError::Persona(#[from] PersonaError)` with a new `ReviewError::UnknownPersona { name, valid: &'static [&'static str] }` variant (preserves the user-facing "valid personas: …" diagnostic), updated module docs to reference SPEC-0027. Updated `speccy-cli/src/main.rs` to match the new variant and drop the `PersonaError` import. Updated `speccy-cli/tests/skill_packs.rs` to drop `persona_content` from the required-placeholders list and to assert no reviewer template contains `{{persona_content}}` or `## Persona`. Rewrote `speccy-cli/tests/review.rs`: deleted the override-pickup test (`prompt_renders_picks_up_project_local_persona_override`), strengthened `prompt_renders_substitutes_every_placeholder` to also assert the absence of `# Reviewer Persona:` and `## Persona` and added `{{persona_content}}` to the retired-placeholder loop. Regenerated the six in-tree dogfood prompt mirrors at `.speccy/skills/prompts/reviewer-<persona>.md` to match the new bundle (these are slated for removal in T-005 but the existing `dogfood_outputs_match_committed_tree` walker would fail otherwise; T-003 will remove the walker contract for prompts).
  - Undone: (none)
  - Commands run: `cargo run --quiet -- tasks SPEC-0027 --commit`; `cargo test --workspace` (initial — 2 failures: dogfood walker mismatch, snapshot fixture missing SPEC-0027); sync dogfood mirrors; add SPEC-0027 entry to `speccy-core/tests/fixtures/in_tree_id_snapshot.json` (REQ-001..004 + CHK-001..004 + DEC-001..005); fix pre-existing missing `# Report:` heading in `.speccy/specs/0026-skill-router-anti-triggers/REPORT.md` (peripheral bug surfaced by `every_in_tree_report_md_parses_and_resolves_against_parent_spec`); `cargo test --workspace` (final — all suites green).
  - Exit codes: final `cargo test --workspace` exit 0.
  - Discovered issues: (1) `.speccy/specs/0026-skill-router-anti-triggers/REPORT.md` shipped without a level-1 `# Report:` heading after the frontmatter, which made `every_in_tree_report_md_parses_and_resolves_against_parent_spec` fail (parse error `missing required field "level-1 heading"`). Pre-existing on `main` since commit 68f6fcf; every other in-tree REPORT.md (0001-0025) carries the heading. Fixed in-place by inserting `# Report: SPEC-0026 skill-router-anti-triggers` between the frontmatter close fence and the `<report spec="SPEC-0026">` opening tag. Not introduced by this SPEC. (2) The init-test dogfood walker (`dogfood_outputs_match_committed_tree`, lines 661-692 of `speccy-cli/tests/init.rs`) compares the embedded `PROMPTS` bundle against `.speccy/skills/prompts/<name>`; this contract is being deleted across SPEC-0027 (T-003 removes init's writes to that subtree; T-005 deletes the in-tree files). For T-001's slice, the in-tree mirrors were resynced so the walker stays green; the walker block itself will be removed in T-003 or T-005.
  - Procedural compliance: (none — friction stemmed from a pre-existing REPORT.md defect and a dogfood walker that SPEC-0027 retires later; no skill-file edit needed.)


Drop the `## Persona` section header and the
`{{persona_content}}` placeholder line from each of the six
reviewer prompt templates under
`resources/modules/prompts/reviewer-<persona>.md` (one per persona:
business, tests, security, style, architecture, docs). Leave the
surrounding sections (`## SPEC (pointer)`, `## Task entry (verbatim
from TASKS.md)`, `## Diff under review`, `## Your task`)
byte-identical so reviewers can confirm the edit is scoped to the
deleted block only.

In the same task, edit `speccy-cli/src/review.rs` so the
rendered-prompt path no longer carries persona body text:

- Drop the `persona_content` key insertion from the `vars`
  `BTreeMap`.
- Drop the call site that resolved the persona body
  (`resolve_persona_file` / `resolve_file`).
- Drop the now-unused imports of `PersonaError` and
  `resolve_file as resolve_persona_file` from `speccy_core::personas`.
- Drop the `ReviewError::Persona` enum variant since no call site
  produces it any more; the surviving persona-name validation
  against `personas::ALL` (today wired to the `PERSONAS_ALL` alias)
  returns a refactored variant such as
  `ReviewError::UnknownPersona { name }` whose payload no longer
  re-exports the deleted `PersonaError` type.

The two edits land together because they are coupled: editing the
templates alone leaves a `{{persona_content}}` variable insertion
in `vars` with no consumer (harmless but stale); editing `review.rs`
alone leaves an unsubstituted `{{persona_content}}` token in the
rendered output. Landing both in one task keeps the workspace green
between commits.

Tests that pin reviewer-prompt shape (under
`speccy-core/tests/prompt_render.rs` or wherever
`{{persona_content}}` / `## Persona` substring assertions live)
flip from "must contain" to "must not contain"; tests that assert
persona-body lines (e.g., the stable `# Reviewer Persona: Business`
first line) survive the rendered prompt are removed or rewritten to
assert absence.

`speccy_core::personas::resolve_file` and its sibling resolver
helpers are NOT deleted in this task — they become unused outside
their own test module, but the deletion belongs to T-004 to keep
the `personas` module surgery isolated. Until then, the resolver
chain survives as dead-but-public API; `cargo clippy --workspace
--all-targets --all-features -- -D warnings` does not flag `pub`
items as `dead_code`, so the workspace stays green between T-001
and T-004.

- Suggested files:
  - `resources/modules/prompts/reviewer-business.md`
  - `resources/modules/prompts/reviewer-tests.md`
  - `resources/modules/prompts/reviewer-security.md`
  - `resources/modules/prompts/reviewer-style.md`
  - `resources/modules/prompts/reviewer-architecture.md`
  - `resources/modules/prompts/reviewer-docs.md`
  - `speccy-cli/src/review.rs`
  - `speccy-core/tests/prompt_render.rs`

<task-scenarios>
  - Given the six reviewer prompt template files at
    `resources/modules/prompts/reviewer-<persona>.md` after this
    task lands, when each file's body is read, then neither the
    literal substring `{{persona_content}}` nor the literal line
    `## Persona` appears in any of the six files.
  - Given the surrounding sections of each of the six template
    files (`## SPEC (pointer)`, `## Task entry (verbatim from
    TASKS.md)`, `## Diff under review`, `## Your task`), when each
    section is compared against its pre-task content, then the
    surrounding sections are byte-identical (the diff is scoped to
    the deleted persona block only).
  - Given a workspace with at least one task in `state="in-review"`,
    when `speccy review <task-ref> --persona business` runs and its
    stdout is captured, then the captured stdout contains neither
    the literal `{{persona_content}}` substring (which would mean
    the placeholder was kept without substitution) nor the literal
    line `# Reviewer Persona: Business` (which would mean the
    persona body was inlined despite the template edit).
  - Given `speccy-cli/src/review.rs` after this task lands, when
    grepped for the identifiers `persona_content`,
    `resolve_persona_file`, and `PersonaError`, then zero matches
    are found in any source line of the file.
  - Given `speccy-cli/src/review.rs` after this task lands, when
    the `ReviewError` enum's variants are enumerated, then no
    variant named `Persona` exists; the surviving persona-name
    validation against `personas::ALL` produces a refactored
    variant (e.g., `UnknownPersona { name }`) instead.
  - Given the test file under `speccy-core/tests/prompt_render.rs`
    (or wherever reviewer-template shape is asserted) after this
    task lands, when each `assert!` / `assert_eq!` mentioning the
    `{{persona_content}}` placeholder or the `## Persona` heading
    is examined, then the assertion expresses "must NOT contain"
    rather than "must contain".
  - Given `cargo test --workspace` after this task lands, when run,
    then the exit code is 0 — all tests pass against the smaller
    rendered-prompt shape.
</task-scenarios>
</task>

## Phase 2: Reshape `speccy init`'s plan around host-native reviewer files

<task id="T-002" state="completed" covers="REQ-002">
## T-002: Classify `.claude/agents/reviewer-*.md` and `.codex/agents/reviewer-*.toml` as Skip-on-exists

- Review (business, pass): REQ-002 satisfied. `is_host_native_reviewer_file` in
  `speccy-cli/src/init.rs:259` matches `.claude/agents/reviewer-<p>.md` and
  `.codex/agents/reviewer-<p>.toml` for every persona in `personas::ALL`
  (path normalised across backslashes for Windows hosts — sensible defensive
  touch). The classifier wire-up at `init.rs:228` collapses
  `Overwrite | Skip` → `Action::Skip` on existence and keeps `Action::Create`
  on absence; all other host-pack paths (skill wrappers under
  `.claude/skills/`, `.agents/skills/`) flow through the unchanged
  `classify(&destination)` and continue to overwrite. Empirical proof from a
  live `init --force` run shows the contract intact: six
  `.claude/agents/reviewer-*.md` lines marked `skip`, eight
  `.claude/skills/speccy-*/SKILL.md` lines marked `overwrite`, zero
  `.speccy/skills/` lines. The four new tests in
  `speccy-cli/tests/init.rs:622-779` cover sentinel preservation,
  delete-and-recreate, plan-summary labels (asserting both halves of the
  classification split), and the Codex twin. Non-goals respected: no
  `--force-personas`, no two-tier frontmatter/body split, no change to
  `render_host_pack`. The `Action::Skip` doc comment was updated to drop the
  retired legacy reference and document the new sole use.

- Implementer note (session-2026-05-17-T002):
  - Completed: In `speccy-cli/src/init.rs`, added `is_host_native_reviewer_file(rel_path)` predicate that returns true iff `rel_path` equals `.claude/agents/reviewer-<persona>.md` or `.codex/agents/reviewer-<persona>.toml` for any `<persona>` in `speccy_core::personas::ALL`. Extended `append_host_pack_items` to route the rendered file's classification through this predicate: when matched, `Overwrite` and `Skip` both collapse to `Action::Skip`; otherwise the existing `classify(&destination)` result flows through unchanged. Added `use speccy_core::personas::ALL as PERSONAS_ALL;` import. Updated the `Action::Skip` doc comment to document the dual use (legacy `.speccy/skills/` directories slated for SPEC-0027 removal, plus the new host-native reviewer-file Skip-on-exists case). All other host-pack files (skill wrappers under `.claude/skills/`, `.agents/skills/`, etc., plus `.speccy/speccy.toml`) retain today's Create-or-Overwrite classification. Added four new tests under `speccy-cli/tests/init.rs`: (1) `t002_claude_reviewer_agent_files_preserve_user_edits_under_force` — append a sentinel to `.claude/agents/reviewer-business.md`, run `init --force`, assert the sentinel survives; (2) `t002_claude_reviewer_agent_files_recreate_when_deleted_under_force` — delete the file, run `init --force`, assert it is recreated with the shipped `# Reviewer Persona: Business` header; (3) `t002_claude_init_force_plan_summary_marks_reviewer_agents_skip_and_skills_overwrite` — capture stdout and assert all six `.claude/agents/reviewer-*.md` lines show `skip` while every `.claude/skills/<verb>/SKILL.md` line shows `overwrite`; (4) `t002_codex_reviewer_agent_files_preserve_user_edits_under_force` — Codex twin covering both sentinel-preservation and deletion-recreation against `.codex/agents/reviewer-business.toml`.
  - Undone: (none)
  - Commands run: `cargo test --workspace` after the edit — all suites green (no new failures introduced; the four new tests all pass).
  - Exit codes: `cargo test --workspace` exit 0.
  - Discovered issues: (none)
  - Procedural compliance: (none)


Extend the init-plan classifier (the function that decides
`Action::Create` / `Action::Overwrite` / `Action::Skip` for each
rendered host-pack file in `speccy-cli/src/init.rs`) so that when a
rendered file's `rel_path` matches
`.claude/agents/reviewer-<persona>.md` or
`.codex/agents/reviewer-<persona>.toml` for any `<persona>` in
`speccy_core::personas::ALL`, the classification is `Action::Skip`
when the destination exists and `Action::Create` when it does not.
All other host-pack files (skill wrappers under `.claude/skills/`,
`.agents/skills/`, etc., plus the `.speccy/speccy.toml`
configuration write) retain today's Create-or-Overwrite
classification.

The change is scoped to the path-matching predicate inside the
classifier. `Action::Skip` itself is unchanged: per DEC-005, the
variant label and `execute_plan` match arm are reused as-is so
plan-print summaries continue to render the `skip` label for the
affected paths. Update the variant's doc comment to reflect that
it now guards host-native reviewer files rather than the (now
removed in T-003) `.speccy/skills/` user-tunable directories.

Add tests under `speccy-cli/tests/init.rs` that exercise the new
classification end-to-end:

- Sentinel preservation: initialize a tempdir once via `speccy
  init`, append a sentinel line to
  `.claude/agents/reviewer-business.md`, run `speccy init --force`,
  then assert the sentinel line is still present.
- Deletion + recreation: initialize a tempdir once, delete
  `.claude/agents/reviewer-business.md` entirely, run `speccy init
  --force`, then assert the file exists again with the shipped
  persona body (e.g., the stable first-line header
  `# Reviewer Persona: Business`).
- Plan-summary labels: capture stdout of `speccy init --force`
  against an initialized workspace and assert lines for
  `.claude/agents/reviewer-*.md` show `skip` while lines for
  `.claude/skills/speccy-*/SKILL.md` continue to show `overwrite`,
  confirming the classification flip is scoped to reviewer agent
  files only.
- Codex twin: repeat the sentinel-preservation scenario for a
  workspace initialized with `--host codex` and the file path
  `.codex/agents/reviewer-business.toml`.

This task does not touch `.speccy/skills/` plan items; T-003 owns
that removal. Until T-003 lands, the init plan still appends the
`.speccy/skills/personas/` and `.speccy/skills/prompts/` entries
(both already classified `Action::Skip` from prior work); the new
Skip-on-exists classification this task adds is for the
`.<host>/agents/reviewer-*` paths only.

- Suggested files:
  - `speccy-cli/src/init.rs`
  - `speccy-cli/tests/init.rs`

<task-scenarios>
  - Given a tempdir already initialized via `speccy init` (so
    `.claude/agents/reviewer-business.md` exists with the shipped
    content), when a sentinel line `# sentinel-edit-12345` is
    appended to that file and then `speccy init --force` runs
    against the same tempdir, then the file still ends with the
    line `# sentinel-edit-12345` after the run.
  - Given a tempdir already initialized via `speccy init`, when
    `.claude/agents/reviewer-business.md` is deleted entirely and
    then `speccy init --force` runs, then the file exists again
    and its body contains the substring
    `# Reviewer Persona: Business` (the stable first-line header
    from the shipped persona body).
  - Given a `speccy init --force` run against an initialized
    workspace, when stdout is captured and parsed line-by-line
    for the plan summary, then every line whose path matches
    `.claude/agents/reviewer-*.md` shows the `skip` action label
    (not `overwrite`).
  - Given the same plan summary, when lines whose path matches
    `.claude/skills/speccy-*/SKILL.md` are scanned, then those
    show the `overwrite` action label (not `skip`), confirming
    the classification flip is scoped to reviewer agent files
    only.
  - Given a tempdir initialized with `speccy init --host codex`
    so `.codex/agents/reviewer-business.toml` exists, when a
    sentinel line is appended to that file and then `speccy init
    --force --host codex` runs, then the file still contains the
    appended sentinel line afterwards. Symmetrically, deleting
    the file and re-initing recreates it from the shipped Codex
    persona content.
  - Given the init-plan classifier exercised in unit tests against
    each `<persona>` in `speccy_core::personas::ALL`, when the
    classifier is invoked with a `rel_path` of
    `.claude/agents/reviewer-<persona>.md` against a tempdir where
    that file exists, then the returned `Action` is `Action::Skip`;
    when invoked against a tempdir where that file does not exist,
    then the returned `Action` is `Action::Create`.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001">
## T-003: Stop appending `.speccy/skills/` items to the init plan

- Review (business, pass): REQ-001 satisfied. `build_plan` in
  `speccy-cli/src/init.rs:198` no longer calls
  `append_user_tunable_dir_items(&PERSONAS, …)` or
  `append_user_tunable_dir_items(&PROMPTS, …)` — both call sites replaced by
  a SPEC-0027 explanatory comment. The helpers themselves
  (`append_user_tunable_dir_items`, `collect_bundle_files`,
  `has_md_extension`) and their imports (`include_dir::Dir`,
  `std::path::Component`, `speccy_core::personas::PERSONAS`,
  `speccy_core::prompt::PROMPTS`) are gone from the module. Three new tests
  in `speccy-cli/tests/init.rs:556-614` exercise the contract: empty-init
  produces no `.speccy/skills/` dir; stdout contains no `.speccy/skills/`
  substring; pre-existing `.speccy/skills/personas/reviewer-business.md`
  with arbitrary bytes survives `init --force` byte-for-byte (DEC-003).
  Non-goals respected: no active deletion of pre-existing
  `.speccy/skills/` (matches the SPEC's explicit non-goal that
  `init` should not silently remove files outside its current write plan).
  Note: the implementer left `PERSONAS`/`PROMPTS` imports in the test
  file (`speccy-cli/tests/init.rs`) under a SPEC-0027 comment because
  `PERSONAS` still feeds the legacy-needle sweep
  (`persona_and_prompt_sources_have_no_legacy_check_authoring_examples`)
  until T-004 deletes the static. That accommodation is documented in the
  implementer note and unwound in T-004's diff — clean dependency
  layering between phases.

- Implementer note (session-2026-05-17-T003):
  - Completed: Edited `speccy-cli/src/init.rs`: dropped the two `append_user_tunable_dir_items(&PERSONAS, …)` and `append_user_tunable_dir_items(&PROMPTS, …)` calls from `build_plan` (replaced by a SPEC-0027 explanatory comment); removed the `append_user_tunable_dir_items`, `collect_bundle_files`, and `has_md_extension` helpers; removed imports of `speccy_core::personas::PERSONAS`, `speccy_core::prompt::PROMPTS`, `include_dir::Dir`, and `std::path::Component`. Updated the `Action::Skip` doc comment to drop the legacy `.speccy/skills/` reference (it now documents the host-native reviewer Skip-on-exists case from T-002 only). Edited `speccy-cli/tests/init.rs`: deleted the `.speccy/skills/personas/`+`prompts/` walker block from `dogfood_outputs_match_committed_tree` (replaced by a SPEC-0027 comment); deleted the `let persona = read_file(…)` byte-equality assertion from `copy_claude_code_pack_skill_md` that pinned `.speccy/skills/personas/reviewer-security.md`; removed the now-unused `SHIPPED_PERSONA_SECURITY` const and the `has_md_ext` helper. Added three T-003 tests: (1) `t003_init_does_not_create_speccy_skills_dir` — empty tempdir + `init --host claude-code`, assert `.speccy/skills` does not exist; (2) `t003_init_plan_summary_does_not_mention_speccy_skills` — capture stdout, assert no `.speccy/skills/` substring; (3) `t003_init_force_preserves_pre_existing_speccy_skills_overrides` — pre-populate `.speccy/skills/personas/reviewer-business.md`, run `init --force`, assert byte-for-byte identical (DEC-003: init never writes into the subtree and never deletes from it). Re-added the `PERSONAS`/`PROMPTS` imports to the test file with a SPEC-0027 comment noting `PERSONAS` survives until T-004 (still consulted by `persona_and_prompt_sources_have_no_legacy_check_authoring_examples`); `PROMPTS` is permanent.
  - Undone: (none)
  - Commands run: `cargo test --workspace` — all suites green; `cargo clippy --workspace --all-targets --all-features -- -D warnings` — fails on the pre-existing `result_large_err` against `speccy-core::error::ParseError` (42+ sites, documented as out-of-scope by SPEC-0026 T-003 procedural compliance; not introduced by SPEC-0027).
  - Exit codes: `cargo test --workspace` exit 0; `cargo clippy …` exits non-zero on the pre-existing `result_large_err` only.
  - Discovered issues: (1) `clippy::result_large_err` on `speccy-core::error::ParseError` (largest variant ≥128 bytes, 42+ call sites) remains pre-existing — pinned out-of-scope by SPEC-0026 T-003 and not regressed by this slice. (2) The init.rs test's `PERSONAS`/`PROMPTS` imports are partially decoupled from the `.speccy/skills/` walker (which is gone) but still feed `persona_and_prompt_sources_have_no_legacy_check_authoring_examples`; T-004 will remove the `PERSONAS` half when the static is deleted.
  - Procedural compliance: (none — the only friction was the imports needing to stay in the test file for one legacy-needle sweep; that's a test-side accommodation, not a skill-file edit.)


Drop the code path in `speccy-cli/src/init.rs` that writes the
`.speccy/skills/personas/` and `.speccy/skills/prompts/` directory
contents into the init plan:

- Remove the two `append_user_tunable_dir_items` call sites that
  add the persona-override and prompt-override directories to the
  plan.
- Remove the `append_user_tunable_dir_items` helper function itself.
- Remove the `collect_bundle_files` helper that exists only to
  support `append_user_tunable_dir_items`.
- Remove the imports of `speccy_core::personas::PERSONAS` and
  `speccy_core::prompt::PROMPTS` from the top of `init.rs`; both
  were used only by the removed helper.

Pre-existing `.speccy/skills/` directories in user workspaces are
left alone (per DEC-003): `init` simply stops writing into that
subtree, but never deletes any file there. The in-tree workspace's
own `.speccy/skills/` directory is removed in T-005 as the dogfood
proof step; this task is the CLI-side change that makes the removal
stick.

Add tests under `speccy-cli/tests/init.rs` that exercise the
removal end-to-end:

- Fresh-init absence: against an empty tempdir, `speccy init` runs
  and afterwards `tmpdir.join(".speccy/skills")` does not exist.
- Plan-output silence: the captured stdout of `speccy init` against
  an empty tempdir contains no occurrence of the literal substring
  `.speccy/skills/`.
- Pre-existing file preservation: a tempdir is pre-populated with
  `.speccy/skills/personas/reviewer-business.md` containing
  arbitrary bytes (e.g., `pre-existing override\n`); after `speccy
  init --force` runs, the file still exists with byte-for-byte
  identical content.

Update any existing init-test that asserted presence of
`.speccy/skills/` paths in the plan to assert absence instead. The
test that confirms `.speccy/speccy.toml` is created continues to
pass since the `.speccy/` write step itself is unchanged; only the
`skills/` subtree disappears from the plan.

`speccy-core/src/personas.rs` still exports the `PERSONAS` static
after this task lands — T-004 owns that deletion. Until T-004
lands, `PERSONAS` survives as a `pub` static referenced only by
its own test module; clippy does not flag `pub` items as
`dead_code`, so the workspace stays green between T-003 and T-004.

- Suggested files:
  - `speccy-cli/src/init.rs`
  - `speccy-cli/tests/init.rs`

<task-scenarios>
  - Given a freshly created empty temporary directory used as
    `project_root`, when `speccy init` runs with that directory as
    `cwd`, then `project_root.join(".speccy").join("skills")` does
    not exist on the filesystem after the command completes.
  - Given the same fresh tempdir, when `speccy init` runs and its
    stdout is captured, then the captured output contains no
    occurrence of the literal substring `.speccy/skills/`.
  - Given a tempdir pre-populated with
    `.speccy/skills/personas/reviewer-business.md` containing the
    bytes `pre-existing override\n`, when `speccy init --force`
    runs against it, then the file still exists with byte-for-byte
    identical content (init does not delete, init does not
    rewrite).
  - Given `speccy-cli/src/init.rs` after this task lands, when
    grepped for `append_user_tunable_dir_items`,
    `collect_bundle_files`, `speccy_core::personas::PERSONAS`, and
    `speccy_core::prompt::PROMPTS`, then zero matches are found in
    any source line of the file (helpers removed, imports
    removed).
  - Given the init plan produced for an empty tempdir, when each
    `PlanItem`'s `rel_path` is read, then no item's path starts
    with `.speccy/skills/`.
  - Given `cargo test --workspace` after this task lands, when
    run, then the exit code is 0 — all tests pass against the
    slimmed init plan.
</task-scenarios>
</task>

## Phase 3: Delete the persona-override resolver chain

<task id="T-004" state="completed" covers="REQ-004">
## T-004: Delete `personas::resolve_file` and related resolver-chain surface

- Review (business, pass): REQ-004 satisfied. `speccy-core/src/personas.rs`
  is now a registry-only module — `PERSONAS` (the embedded body bytes),
  `PersonaError`, `resolve_file`, `resolve_file_with_warn`, and
  `persona_file_name` are all gone; the module-level doc comment rewritten
  to describe the surface as a persona-name registry and to point at
  SPEC-0027 + SPEC-0009. `ALL` survives unchanged, declaring the six
  personas in `business, tests, security, style, architecture, docs`
  order. `speccy-core/tests/personas.rs` retains the three registry tests
  (`registry_contains_six_personas_in_declared_order`,
  `registry_default_personas_is_first_four_prefix`,
  `registry_personas_are_unique`) and drops the seven resolver-chain tests
  enumerated in the SPEC done-when. Integration tests that previously
  consumed `PERSONAS` directly (`speccy-cli/tests/skill_packs.rs` and
  `speccy-cli/tests/init.rs`) were retargeted to read body bytes via
  `speccy_cli::embedded::RESOURCES.get_file("modules/personas/<name>")`
  / `…/prompts/<name>` — the renaming keeps the legacy-needle sweep
  intact without re-exporting a deleted symbol. `cargo test --workspace`
  exit 0; the only clippy fire is the pre-existing
  `result_large_err` on `speccy-core::error::ParseError` (documented as
  out-of-scope under SPEC-0026 T-003 + carried forward here). Non-goals
  respected: no change to the prompt template pipeline; no introduction
  of a renamed override directory.

- Implementer note (session-2026-05-17-T004):
  - Completed: Rewrote `speccy-core/src/personas.rs` to be a registry-only module: removed the `PERSONAS` static (the embedded copy of persona body bytes), the `PersonaError` enum, the `resolve_file` and `resolve_file_with_warn` functions, and the `persona_file_name` helper. Dropped the now-unused `use camino::Utf8Path;`, `use include_dir::{Dir, include_dir};`, `use std::io::Write;`, and `use thiserror::Error;` imports. Updated the module-level doc comment to describe the module as the persona-name registry only, cite SPEC-0027, and explain that persona body content reaches sub-agents via the host-pack renderer's Jinja `{% include %}` into `.claude/agents/reviewer-<persona>.md` / the Codex equivalent. Retained the `ALL` constant and its declared-order documentation. Rewrote `speccy-core/tests/personas.rs` to host only the three registry tests (`registry_contains_six_personas_in_declared_order`, `registry_default_personas_is_first_four_prefix`, `registry_personas_are_unique`); deleted the seven resolver-chain tests, the `make_tmp_root` / `write_override` helpers, and the `camino`/`speccy_core::personas::{PersonaError, resolve_file, resolve_file_with_warn}` imports. Retargeted the two integration test files that still consumed the deleted `PERSONAS` static: `speccy-cli/tests/skill_packs.rs` now reads persona/prompt bodies via `speccy_cli::embedded::RESOURCES.get_file("modules/personas/<name>")` (and the symmetric `modules/prompts/<name>` form); `speccy-cli/tests/init.rs` now iterates `RESOURCES.get_dir("modules/personas")` and `…/prompts` for the SPEC-0018 legacy-needle sweep, dropping the `PERSONAS`/`PROMPTS` imports and adding `use speccy_cli::embedded::RESOURCES;`.
  - Undone: (none)
  - Commands run: `cargo test --workspace` after the deletion + retargeting — all suites green.
  - Exit codes: `cargo test --workspace` exit 0.
  - Discovered issues: (pre-existing `clippy::result_large_err` on `speccy-core::error::ParseError`, documented under T-003 and SPEC-0026 T-003; not regressed by this slice).
  - Procedural compliance: (none)


Edit `speccy-core/src/personas.rs` to remove the persona-override
resolver chain:

- Delete the `PERSONAS` static (the embedded copy of persona body
  bytes used only by the resolver).
- Delete the `PersonaError` enum.
- Delete the `resolve_file` function.
- Delete the `resolve_file_with_warn` function.
- Delete the `persona_file_name` helper.

Keep the `ALL` constant (and its surrounding doc comments) — that
is the only public surface of the `personas` module that survives
and is consumed by `speccy review --persona` validation (today
imported as `PERSONAS_ALL` from `speccy-cli/src/review.rs`) and by
the speccy-review skill fan-out. Update the module-level doc
comment to remove references to the project-local override
directory and the resolution chain; the module's job is now
"persona name registry" only.

Edit `speccy-core/tests/personas.rs` to match: keep the three
registry tests
(`registry_contains_six_personas_in_declared_order`,
`registry_default_personas_is_first_four_prefix`,
`registry_personas_are_unique`) and delete the seven
resolver-chain tests
(`resolve_local_first_returns_override_content`,
`resolve_local_first_returns_embedded_when_override_missing`,
`resolve_empty_override_falls_through_with_warning`,
`resolve_empty_override_falls_through_silently_when_no_override_present`,
`resolve_unknown_name_returns_unknown_name_error`,
`t002_resolve_reviewer_security_returns_shipped_body_with_pre_move_first_line`,
`resolve_does_not_check_host_native_locations`).

This task lands only after T-001 (`speccy-cli/src/review.rs` no
longer imports `resolve_file` or `PersonaError`) and T-003
(`speccy-cli/src/init.rs` no longer imports `PERSONAS`). With both
predecessor tasks landed, the items deleted here have no live
references outside their own test module (which is deleted in the
same edit).

Verify the full hygiene gate passes after the deletion: `cargo
test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo +nightly fmt --all
--check`, `cargo deny check`. No `dead_code` or `unused_imports`
lint should fire as a consequence of the removals.

- Suggested files:
  - `speccy-core/src/personas.rs`
  - `speccy-core/tests/personas.rs`

<task-scenarios>
  - Given `speccy-core/src/personas.rs` after this task lands,
    when scanned for the identifiers `resolve_file`,
    `resolve_file_with_warn`, `PersonaError`, `persona_file_name`,
    and `PERSONAS`, then none of these identifiers appears as a
    `fn`, `struct`, `enum`, or `static` declaration in the file.
  - Given the same file, when grepped for the `ALL` constant
    declaration, then it survives unchanged and exports the six
    persona names in the order
    `business, tests, security, style, architecture, docs`.
  - Given `speccy-cli/src/review.rs` after this task lands, when
    grepped for `speccy_core::personas::PERSONAS` (the embedded
    bytes static, distinct from the `ALL` registry), then zero
    matches are found.
  - Given the test file `speccy-core/tests/personas.rs` after this
    task lands, when its `#[test]` functions are enumerated, then
    the surviving functions are exactly the three registry-only
    tests (`registry_contains_six_personas_in_declared_order`,
    `registry_default_personas_is_first_four_prefix`,
    `registry_personas_are_unique`) and none of the seven
    resolver-chain tests survive.
  - Given a clean checkout of the workspace after this task
    lands, when `cargo test --workspace` and `cargo clippy
    --workspace --all-targets --all-features -- -D warnings` both
    run, then both exit with status 0.
  - Given the module-level doc comment on
    `speccy-core/src/personas.rs` after this task lands, when
    read, then it describes the module as a persona-name registry
    and contains no reference to a project-local override
    directory, a resolver chain, or an embedded `PERSONAS` bundle.
</task-scenarios>
</task>

## Phase 4: Dogfood the removal in this workspace

<task id="T-005" state="completed" covers="REQ-001">
## T-005: Remove `.speccy/skills/` from this workspace and confirm `init --force` does not recreate it

- Review (business, pass): REQ-001's dogfood half satisfied. The
  in-tree `.speccy/skills/` directory is gone (verified by `ls
  .speccy/skills/` → "no such file"). A fresh `cargo run -- init --force
  --host claude-code` does not recreate it: stdout shows zero
  `.speccy/skills/` lines, six `.claude/agents/reviewer-*.md` lines as
  `skip` (T-002 holding), eight `.claude/skills/speccy-*/SKILL.md` lines
  as `overwrite`, plus the `.speccy/speccy.toml` overwrite — final
  tallies `0 created, 9 overwritten, 6 skipped`. The implementer caught a
  spec-vs-reality drift the SPEC itself missed: `### Interfaces`
  enumerated 18 files, but the on-disk tree had 20 (the two extras —
  `implementer.md` and `planner.md` under `.speccy/skills/personas/` —
  served the same dogfood-override role as the reviewer personas). The
  implementer flagged this in the discovered-issues note and removed all
  20 via `git rm -rf .speccy/skills/`; that is the right call — REQ-001
  is normative on "no `.speccy/skills/` exists in this workspace,"
  not on the count of files removed, and the SPEC's enumeration is
  descriptive. The snapshot fixture at
  `speccy-core/tests/fixtures/in_tree_id_snapshot.json:638-660` adds the
  SPEC-0027 entry with REQ-001..004, CHK-001..004, DEC-001..005 — id
  sets match the SPEC.md verbatim. `cargo test --workspace`,
  `cargo +nightly fmt --all --check`, and `cargo deny check` all exit 0;
  `cargo clippy …` exits non-zero only on the pre-existing
  `result_large_err` carried forward. Non-goals respected: `init`
  itself still doesn't delete `.speccy/skills/`; the human implementer
  ran the `git rm` directly, which is the SPEC's documented recovery
  path. Follow-up note worth raising in the report: SPEC.md's
  `### Interfaces` should be re-tightened (or relaxed to "the tree
  including but not limited to these files") before SPEC-0027 ships, so
  future audits don't trip on the same 18-vs-20 drift.

- Implementer note (session-2026-05-17-T005):
  - Completed: Ran `git rm -rf .speccy/skills/` to remove the 20 in-tree dogfood files (8 personas + 12 prompts; the persona walk surfaced two extra files — `implementer.md` and `planner.md` — beyond the 18 listed in SPEC `### Interfaces`, both legitimate persona bodies that shipped via the pre-SPEC override directory). Verified by re-running `cargo run -- init --force --host claude-code` and `cargo run -- init --force --host codex` that `.speccy/skills/` is not recreated in either case; the plan-summary output shows the six `.claude/agents/reviewer-*.md` lines with `skip` action (T-002's Skip-on-exists holding) and the eight `.claude/skills/speccy-*/SKILL.md` lines with `overwrite` action, with no `.speccy/skills/` lines anywhere. Ran the full hygiene gate: `cargo test --workspace` exits 0; `cargo +nightly fmt --all --check` exits 0 (applied formatter to fix a one-line wrap in `speccy-cli/tests/skill_packs.rs::read_prompt`); `cargo deny check` exits 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails only on the pre-existing `result_large_err` against `speccy-core::error::ParseError` (42+ sites, documented under SPEC-0026 T-003 and SPEC-0027 T-003 as out-of-scope).
  - Undone: (none)
  - Commands run: `git rm -rf .speccy/skills/`; `cargo run -- init --force --host claude-code`; `cargo run -- init --force --host codex`; `cargo test --workspace`; `cargo +nightly fmt --all`; `cargo +nightly fmt --all --check`; `cargo deny check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  - Exit codes: `git rm`, both init runs, cargo test, fmt, deny all exit 0. Clippy fails on pre-existing `result_large_err` only.
  - Discovered issues: (1) `.speccy/skills/personas/` also held two non-reviewer persona bodies — `implementer.md` and `planner.md` — that the SPEC `### Interfaces` "Files deleted (in-tree dogfood)" enumeration omitted (it listed only the six `reviewer-<persona>.md` files plus the twelve `.speccy/skills/prompts/*.md` files for a total of 18, but the actual on-disk tree had 20). These two files served the same dogfood-override role as the reviewer personas and are removed by the same `git rm -rf .speccy/skills/` walk. (2) `clippy::result_large_err` against `speccy-core::error::ParseError` remains pre-existing and out-of-scope (documented under SPEC-0026 T-003).
  - Procedural compliance: (none — the SPEC's recovery path for cleaning up `.speccy/skills/` was documented as `rm -rf` and that is what landed; no skill-file edit needed.)


After T-001 through T-004 have landed, remove the in-tree
`.speccy/skills/` directory from this workspace as the dogfood
proof that the CLI no longer recreates it:

- `git rm -rf .speccy/skills/` (or the equivalent `git rm` walk
  over each of the 18 files listed in SPEC.md `### Interfaces`
  under "Files deleted (in-tree dogfood)").
- Run `cargo run -- init --force --host claude-code` against the
  workspace and confirm the directory is not recreated.
- Run `cargo run -- init --force --host codex` against the
  workspace and confirm the directory is not recreated.
- Run the full hygiene gate (`cargo test --workspace`, `cargo
  clippy --workspace --all-targets --all-features -- -D
  warnings`, `cargo +nightly fmt --all --check`, `cargo deny
  check`) and confirm all four checks pass.

The 18 deleted files are the six
`.speccy/skills/personas/reviewer-<persona>.md` files (one per
persona in `personas::ALL`) plus the twelve
`.speccy/skills/prompts/<name>.md` files (implementer.md,
plan-amend.md, plan-greenfield.md, report.md, reviewer-business.md,
reviewer-tests.md, reviewer-security.md, reviewer-style.md,
reviewer-architecture.md, reviewer-docs.md, tasks-amend.md,
tasks-generate.md). The exhaustive list lives in SPEC.md
`### Interfaces` under "Files deleted (in-tree dogfood)".

This task also serves as the end-to-end verification that the
classification flip from T-002 holds: re-running `init --force`
preserves the user-edited
`.claude/agents/reviewer-*.md` files committed to this workspace
(the workspace's own committed reviewer-agent files are the
"user-edited" surface from the perspective of `init`'s plan).
After this task lands, the workspace's tree contains no
`.speccy/skills/` directory and re-running `init` against it stays
that way.

If the snapshot fixture
`speccy-core/tests/fixtures/in_tree_id_snapshot.json` needs an
entry for `0027-host-native-personas` (per the convention every
prior SPEC has honored — see the T-004 procedural note in
SPEC-0026's TASKS.md), add the SPEC's REQ/CHK/DEC id sets in the
same commit so the in-tree-specs snapshot test stays green.

- Suggested files:
  - `.speccy/skills/personas/reviewer-business.md` (delete)
  - `.speccy/skills/personas/reviewer-tests.md` (delete)
  - `.speccy/skills/personas/reviewer-security.md` (delete)
  - `.speccy/skills/personas/reviewer-style.md` (delete)
  - `.speccy/skills/personas/reviewer-architecture.md` (delete)
  - `.speccy/skills/personas/reviewer-docs.md` (delete)
  - `.speccy/skills/prompts/implementer.md` (delete)
  - `.speccy/skills/prompts/plan-amend.md` (delete)
  - `.speccy/skills/prompts/plan-greenfield.md` (delete)
  - `.speccy/skills/prompts/report.md` (delete)
  - `.speccy/skills/prompts/reviewer-architecture.md` (delete)
  - `.speccy/skills/prompts/reviewer-business.md` (delete)
  - `.speccy/skills/prompts/reviewer-docs.md` (delete)
  - `.speccy/skills/prompts/reviewer-security.md` (delete)
  - `.speccy/skills/prompts/reviewer-style.md` (delete)
  - `.speccy/skills/prompts/reviewer-tests.md` (delete)
  - `.speccy/skills/prompts/tasks-amend.md` (delete)
  - `.speccy/skills/prompts/tasks-generate.md` (delete)
  - `speccy-core/tests/fixtures/in_tree_id_snapshot.json` (add
    SPEC-0027 entry if the snapshot test requires it)

<task-scenarios>
  - Given the workspace after T-001 through T-004 have landed and
    `git rm -rf .speccy/skills/` (or an equivalent per-file `git
    rm` walk) has been executed and committed, when `ls
    .speccy/skills` runs (or `Test-Path .speccy/skills` on
    Windows), then the path does not exist.
  - Given the same workspace, when `cargo run -- init --force
    --host claude-code` runs, then on completion
    `.speccy/skills/` still does not exist on the filesystem.
  - Given the same workspace, when `cargo run -- init --force
    --host codex` runs, then on completion `.speccy/skills/`
    still does not exist on the filesystem.
  - Given the same workspace, when the captured stdout of either
    `init --force` run is scanned for the literal substring
    `.speccy/skills/`, then zero matches are found.
  - Given the same workspace, when the four-tool hygiene gate
    runs (`cargo test --workspace`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, `cargo
    +nightly fmt --all --check`, `cargo deny check`), then all
    four commands exit with status 0.
  - Given the workspace's `.claude/agents/reviewer-*.md` files
    committed to this repository, when `cargo run -- init
    --force --host claude-code` runs against the workspace, then
    afterwards `git diff .claude/agents/reviewer-*.md` reports
    zero modifications (T-002's Skip-on-exists classification
    holds; the workspace's committed reviewer-agent content is
    preserved across re-init).
  - Given the snapshot fixture
    `speccy-core/tests/fixtures/in_tree_id_snapshot.json` after
    this task lands, when the in-tree-specs snapshot test runs,
    then either the fixture already covers SPEC-0027's REQ/CHK/
    DEC id sets or the test does not require a new entry; in
    either case `cargo test --workspace` exits with status 0.
</task-scenarios>
</task>

</tasks>
