---
spec: SPEC-0018
outcome: delivered
generated_at: 2026-05-15T22:00:00Z
---

# SPEC-0018: Remove check execution; checks become validation scenarios

## Outcome

**delivered** — all five requirements satisfied. `speccy check` is
now a render-only command and `speccy verify` is a shape-only
validator; neither spawns child processes. The `[[checks]]` schema
shrinks to `{ id, scenario }`, all 20 in-tree `spec.toml` files
were mechanically migrated, `speccy_core::exec` is deleted, and
shipped skills + ARCHITECTURE.md teach the new feedback-only
contract. The arbitrary-command sink that previously lived in
`speccy check` is gone at the source level, locked by both
`#[serde(deny_unknown_fields)]` on `RawCheck` and a static
`include_str!`-based regression guard.

## Requirements coverage

| Requirement | Scenarios | Result |
|-------------|-----------|--------|
| REQ-001: Check schema collapses to `id` and `scenario` | CHK-001 | proved |
| REQ-002: `speccy check` renders scenarios only | CHK-002 | proved |
| REQ-003: `speccy verify` is shape-only | CHK-003 | proved |
| REQ-004: Execution code and tests are deleted | CHK-004 | proved |
| REQ-005: Docs and shipped skills teach the new contract | CHK-005 | proved |

Scenarios are validated by unit and integration tests in
`speccy-core/src/parse/toml_files.rs`,
`speccy-core/tests/in_tree_spec_tomls.rs`,
`speccy-cli/tests/check.rs`, `speccy-cli/tests/verify.rs`,
`speccy-cli/tests/init.rs`, and `speccy-cli/tests/ci_workflow.rs`.
The post-SPEC reviewer-tests contract (do not treat `speccy check`
exit codes as evidence) and the ARCHITECTURE.md "Feedback, Not
Enforcement" stance are pinned by verbatim content assertions in
`speccy-cli/tests/init.rs`.

## Task summary

- **Total tasks:** 6 (T-001 through T-006).
- **Retried tasks:** 2 (T-005 and T-006, one retry round each).
- **SPEC amendments:** 0.

T-001 added a bridge parser carrying both legacy and new check
shapes so the migration in T-004 could be mechanical. T-002
rewrote `speccy-cli::check::run` to render scenarios instead of
spawning commands and replaced the legacy `<-- CHK-NNN PASS|FAIL`
framing with `==> CHK-NNN (SPEC-NNNN): <first line>` headers plus
a `N scenarios rendered across M specs` summary. T-003 stripped
`speccy verify` of all execution paths, bumped `--json` to
`schema_version = 2`, removed `outcome`/`exit_code`/`duration_ms`,
and added the new REQ-003 lint for scenarios unreferenced by any
requirement. T-004 migrated every in-tree `spec.toml` to the new
schema, applied `#[serde(deny_unknown_fields)]` to `RawCheck`,
deleted `speccy-core/src/exec.rs` plus `tests/exec_captured.rs`,
removed the now-unreachable VAL-* lint family, and added a
workspace-scope parse oracle. T-005 swept ARCHITECTURE.md and
shipped prompts/personas to teach the scenario contract; the retry
added six content-pin tests and broadened the legacy-needle walk
to source files. T-006 added two CI-shape regression tests
(`project_test_commands_run_directly_before_speccy_verify`,
`speccy_verify_step_does_not_run_project_tests`) and the legacy
authoring guard `rendered_outputs_have_no_legacy_check_authoring_examples`;
the retry paired a compile-time `include_str!` grep over
`speccy-cli/src/check.rs` (rejecting `Command::new`,
`process::Command`, `.spawn(`) with a runtime `assert_cmd`
invocation, regression-guarding the structural absence of process
spawning in the check path.

## Out-of-scope items absorbed

Edits made during the loop that were not part of the planned task
scope but were necessary for the work to land cleanly:

- **VAL-* lint family removed during T-004** —
  `speccy-core/src/lint/rules/val.rs`, `tests/lint_val.rs`, and the
  `val-004-no-op` fixture were deleted along with their registry
  entries (`VAL-001`..`VAL-004`) and the registry snapshot was
  regenerated. The rules became structurally unreachable once
  `#[serde(deny_unknown_fields)]` started rejecting legacy
  `kind`/`command`/`prompt`/`proves` fields at the parser layer;
  AGENTS.md's "Surgical changes" rule keeps pre-existing dead
  code, but the VAL-* surface only became dead in T-004 itself,
  so it was removed in the same commit set. SPC-001 still
  surfaces the surviving empty-scenario error.
- **`CheckEntry` construction call sites updated in T-001** —
  test fixtures in `speccy-core/src/parse/cross_ref.rs` and the
  test module of `speccy-core/src/exec.rs` were edited surgically
  to populate the new `scenario` field. The latter file was then
  deleted entirely in T-004.
- **REQ-003 lint added in T-003** — the SPEC names "scenario rows
  not referenced by any requirement" as a verify error, but no
  existing lint rule covered that direction; T-003 added
  `REQ-003` to `speccy-core::lint::rules::req` and registered it
  at `Level::Error`, refreshing the lint-registry snapshot.
- **`SPEC_0018_LEGACY_NEEDLES` promoted to module scope in
  T-005's retry** — the original `legacy_needles` array was a
  local in `rendered_outputs_have_no_legacy_check_authoring_examples`;
  the retry promoted it to a module-level `[&str; 6]` constant so
  the three new walk-broadening tests
  (`architecture_doc_has_no_legacy_check_authoring_examples`,
  `persona_and_prompt_sources_have_no_legacy_check_authoring_examples`,
  and the extended rendered-outputs guard) could share one
  needle list.

## Procedural compliance

No shipped skill files required edits during this spec. The
dogfooded materialisation loop (`speccy init --force`) was used
twice — once in T-005 to refresh `.claude/`, `.codex/`, and
`.agents/` packs against the updated `resources/modules/` sources,
and once during T-006 to confirm `dogfood_outputs_match_committed_tree`
still passes byte-for-byte.

## Notes

This is the first step of the sequence that removes test-running
responsibility from Speccy. SPEC-0019 will remove the `spec.toml`
carrier entirely by moving requirement/scenario structure into a
canonical marker-structured `SPEC.md`. SPEC-0020 will then switch
that carrier from HTML-comment markers to raw XML element tags, and
SPEC-0021 will apply the same raw XML carrier to `TASKS.md` and
`REPORT.md`. With execution semantics gone, those carrier moves no
longer have to preserve any executable-command vocabulary.

The two non-blocking nits flagged during review and not addressed
here:

- `parses_multiline_scenario_verbatim`
  (`speccy-core/src/parse/toml_files.rs:367-384`) uses `.contains`
  rather than exact-string equality; a future indentation
  normalisation regression would still pass.
- `lint/registry.rs:9` describes the registry as "append-only
  across minor versions" while T-004 removed VAL-001..004 (pre-v1
  hard break, justified by the SPEC's "no compatibility shim"
  Non-goal).
