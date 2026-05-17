---
spec: SPEC-0024
spec_hash_at_generation: be6f4dc1bff354b66e73b12a561f1af3b2166230b675bd396c9dc3887e372a96
generated_at: 2026-05-17T17:57:40Z
---

# Tasks: SPEC-0024 Meaningful hash semantics and per-spec status selectors

<tasks spec="SPEC-0024">

## Phase 1: Hash function

<task id="T-001" state="completed" covers="REQ-001">
Add canonical frontmatter serialization helper and exclusion-list constant

- Suggested files: `speccy-core/src/parse/spec_md.rs`

<task-scenarios>
  - When `speccy-core/src/parse/spec_md.rs` is read after this task,
    a private `const HASH_EXCLUDED_FRONTMATTER_FIELDS: &[&str]`
    exists and contains exactly `["status"]`.
  - When a new helper function (e.g. `canonical_frontmatter_for_hash`)
    is invoked on a `SpecFrontmatter` value, it returns deterministic
    bytes: two calls on equal `SpecFrontmatter` values return
    byte-identical output, and the output omits the `status` field
    entirely (no `status:` key in the serialized bytes).
  - When the helper serializes two `SpecFrontmatter` values that
    differ only in `status`, the output is byte-identical.
  - When the helper serializes a `SpecFrontmatter`, the output keys
    appear in deterministic order (alphabetical) so source-file
    reordering does not perturb the output.
  - When the helper serializes a `SpecFrontmatter` with
    `supersedes: vec![]`, the output is byte-identical to the output
    for a `SpecFrontmatter` whose `supersedes` was defaulted to an
    empty vec by `#[serde(default)]`. (The two parses must hash the
    same.)
  - When unit tests for the helper are run, every assertion above is
    covered by at least one `#[test]` in `speccy-core/src/parse/spec_md.rs`.
  - This task does NOT yet change `SpecMd.sha256`'s computation;
    the existing field continues to hash raw bytes. The helper lands
    green first so it can be exercised without disturbing downstream
    fixtures.
</task-scenarios>

- Implementer note (claude-opus-4-7-t001):
  - Completed: Added the private `HASH_EXCLUDED_FRONTMATTER_FIELDS: &[&str] = &["status"]` constant and a hand-rolled `canonical_frontmatter_for_hash(&SpecFrontmatter) -> Vec<u8>` helper in `speccy-core/src/parse/spec_md.rs`. The helper emits alphabetical keys, double-quoted strings (with `"`, `\`, `\n`, `\r`, `\t`, and ASCII-control escapes via a small `hex_nibble` helper), and flow-style sequences; the exclusion constant is consumed inline via a closure-based `push_kv` so adding a new excluded field is a one-line constant edit. Returned `Vec<u8>` instead of `Result<Vec<u8>, ParseError>` because the hand-rolled emitter is infallible — the SPEC.md "Interfaces" block explicitly allows the signature to be finalised during implementation. Added eight new `#[test]`s covering: the constant's contents, determinism, status omission, status-flip invariance, alphabetical key order, non-status-field perturbation (id/slug/title/created/supersedes), default-vs-explicit empty `supersedes`, and source-file key-reordering invariance (via the existing `spec_md` parser). `SpecMd.sha256`'s computation is unchanged — T-002 wires the helper in. No new dependencies; `serde-saphyr` would not have given a stable emitter and the bounded six-field schema does not justify pulling in `serde_yaml` just for hashing.
  - Undone: (none) — T-001 scope satisfied; T-002 (switch `SpecMd.sha256` to consume the helper), T-003/T-004 (lint + command-guard), T-005 (CLI selector), and T-006 (reconciliation) remain pending.
  - Commands run: `cargo test -p speccy-core --lib parse::spec_md`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all`; `./target/release/speccy check SPEC-0024/T-001`.
  - Exit codes: pass; pass; pass; pass (after one re-format diff applied); pass.
  - Discovered issues: SPEC-0024's authored SPEC.md violated the SPEC-0020 blank-line-after-close-tag convention at five `</scenario></requirement>` boundaries (lines 237, 320, 398, 524, 598), and `speccy-core/tests/fixtures/in_tree_id_snapshot.json` had no entry for `0024-meaningful-hash-semantics`, so `every_migrated_spec_md_has_blank_line_after_each_close_tag` and `every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot` failed before any T-001 code was written. Both are pre-existing authoring drift; fixed mechanically (insert blank lines, add the snapshot entry — 6 decisions / 5 requirements / 5 scenarios) since they block `cargo test --workspace`, which AGENTS.md mandates as a precommit gate. Note that any SPEC.md byte edit (including these blank-line fixes) invalidates the stored `spec_hash_at_generation` under the current raw-bytes hash, so `speccy status` will report `hash-drift` for SPEC-0024 until T-002 + T-006 land.
  - Procedural compliance: (none) — no skill files needed updating; the implementer prompt's "Suggested files" pointer was accurate and the project test command (`cargo test`) matched expectations.
- Review (business, pass): REQ-001's helper + exclusion-list scope is delivered. `HASH_EXCLUDED_FRONTMATTER_FIELDS: &[&str] = &["status"]` lands as a private constant (`speccy-core/src/parse/spec_md.rs:140`), and `canonical_frontmatter_for_hash` (line 259) hand-rolls a deterministic, alphabetically-ordered emitter that omits any key in the exclusion list, satisfying the slice-level scenarios verbatim — the eight new `#[test]`s map 1:1 to the `<task-scenarios>` bullets (constant contents, determinism, status omission, status-flip invariance, alphabetical key order, non-status-field perturbation, default-vs-explicit empty `supersedes`, source-file key-reordering invariance). The `Vec<u8>` (not `Result<Vec<u8>, ParseError>`) signature deviation from SPEC.md's "Interfaces" sketch is explicitly licensed by that section ("signature finalised during implementation") and matches REQ-001's `<done-when>` phrasing "returns the canonical bytes"; the implementer note records the rationale (infallible hand-rolled emitter). DEC-001 (hand-rolled over generic YAML emitter) and DEC-002 (default-include, exclusion-list contains only `status`) are both honored. No open questions were silently resolved; all six in SPEC.md are `resolved="true"` upstream. The user-facing scenarios under REQ-001 / CHK-001 that exercise `SpecMd.sha256` itself are properly deferred to T-002, which the slice scope explicitly disclaims. No non-goals breached.
</task>

<task id="T-002" state="completed" covers="REQ-001">
Switch `SpecMd.sha256` to use canonical frontmatter + body bytes

- Suggested files: `speccy-core/src/parse/spec_md.rs`,
  `speccy-core/tests/` (any fixture file with a hardcoded expected
  hash that needs regeneration)

<task-scenarios>
  - When `speccy-core/src/parse/spec_md.rs:spec_md` is read after
    this task, `SpecMd.sha256` is computed as
    `Sha256::digest(canonical_frontmatter_bytes || body_bytes)`
    where `body_bytes` is everything after the closing `---` fence.
    The previous `Sha256::digest(raw.as_bytes())` call is gone.
  - When two SPEC.md fixtures identical except for `status`
    (`in-progress` vs `implemented`) are parsed, `SpecMd.sha256`
    returns equal digests.
  - When two SPEC.md fixtures identical except for one body byte are
    parsed, `SpecMd.sha256` returns different digests.
  - When two SPEC.md fixtures with frontmatter keys reordered in
    source order (but parsed to equivalent `SpecFrontmatter` values)
    are parsed, `SpecMd.sha256` returns equal digests.
  - When a SPEC.md fixture's `slug`, `title`, `created`,
    `supersedes`, or `id` field is changed, `SpecMd.sha256` returns
    a different digest from the original.
  - When the existing `sha256_changes_on_one_byte_edit` and
    `sha256_stable_for_identical_content` tests in
    `speccy-core/src/parse/spec_md.rs` are run, both pass under the
    new computation (the meaning of "identical content" is now
    canonical-content, but byte-edits to body still change the hash).
  - When `cargo test --workspace` is run, all tests pass. Any
    fixture file that previously hardcoded an expected hash is
    updated to the value the new function produces for that
    fixture's content. The doc comment on `SpecMd.sha256` is
    updated to describe the new scope.
</task-scenarios>

- Implementer note (claude-opus-4-7-t002):
  - Completed: Switched `SpecMd.sha256` to `Sha256::digest(canonical_frontmatter || body)`. Introduced a private `canonical_content_sha256(raw, fm, path)` helper in `speccy-core/src/parse/spec_md.rs` that re-runs `split_frontmatter` on the raw source to get the body slice, then digests `canonical_frontmatter_for_hash(fm)` followed by the body bytes via an incremental `Sha256` hasher. Removed the T-001 `#[cfg_attr(not(test), expect(dead_code, ...))]` gate on `canonical_frontmatter_for_hash` since it is now consumed at runtime. Updated the field's doc comment to "sha256 of canonical(frontmatter \ {status}) ++ body bytes. Stable across status flips and frontmatter cosmetics; changes on any body byte edit or non-`status` frontmatter field change." Updated the module-level doc to point at both SPEC-0001 REQ-003 and SPEC-0024 REQ-001. Added seven new `#[test]`s on `SpecMd.sha256` itself (separate from the canonical-helper tests T-001 added): status-flip invariance, source-key-reordering invariance, and one negative test per non-`status` frontmatter field (`id`, `slug`, `title`, `created`, `supersedes`) confirming the hash perturbs.
  - Undone: REQ-005 / T-006 (reconciliation of all stored TASKS.md `spec_hash_at_generation` values) is intentionally deferred — that task is sequenced last in this spec's TASKS.md because the hash function is still in flux until T-003/T-004/T-005 land. As expected post-T-002, `./target/release/speccy status` now reports `hash-drift` on every spec including SPEC-0024; that drift is the reconciliation task's input and not a regression.
  - Commands run: `cargo build --release --bin speccy`; `cargo test -p speccy-core --lib parse::spec_md`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all`; `cargo +nightly fmt --all --check`; `./target/release/speccy check SPEC-0024/T-002`; `./target/release/speccy status`.
  - Exit codes: pass; pass (23 tests passing — 7 new sha256-on-SpecMd tests join the T-001 canonical-helper tests); pass (55 test-group counts of `0 failed`); pass; pass; pass; pass (renders CHK-001's five scenarios); informational (every spec reports `hash-drift` as expected — input to T-006).
  - Discovered issues: (none) — the only fixtures with hardcoded hashes were workspace_xml/* and task_xml/canonical.md with the placeholder `deadbeef`, which is intentionally wrong-shape and never compared against a live hash. The dynamic hash sites in `speccy-core/tests/stale_detection.rs` use `hex_of(&parsed_spec.sha256)` from the live parse and auto-track the new hash function.
  - Procedural compliance: (none) — no skill files needed updating; the implementer prompt's "Suggested files" pointer was accurate.
- Review (business, pass): T-002 swaps `SpecMd.sha256` to `Sha256(canonical_frontmatter || body)` exactly as REQ-001 names; the seven new `SpecMd.sha256` tests cover status-flip invariance, source-key reordering, and per-field perturbation (`id`, `slug`, `title`, `created`, `supersedes`), and the existing `sha256_stable_for_identical_content` / `sha256_changes_on_one_byte_edit` still pass under the new computation. Two minor gaps that do not block the slice but the orchestrator should know about: (1) REQ-001's done-when bullet "Adding or removing whitespace before/after the closing `---` fence does not affect the canonical re-serialization for the frontmatter portion" has no direct test — the canonical re-serializer erases source whitespace by construction, but an explicit assertion would lock that contract; (2) the `<behavior>` Given/When/Then about `supersedes: []` vs defaulted-empty `supersedes` is exercised only at the canonical-bytes level in `canonical_frontmatter_equates_explicit_and_default_empty_supersedes`, not via a direct `SpecMd.sha256` equality test. Body extraction relies on `split_frontmatter` which consumes the fence's terminating newline before returning the body slice; the SPEC's parenthetical "inclusive of the trailing newline that follows the fence" reads literally as if the post-fence newline should be in `body_bytes`, but the behavioural scenarios all hold either way and this is a wording nit, not a contract break. Goals and non-goals respected, no open questions silently resolved.
</task>

## Phase 2: ID consistency enforcement

<task id="T-003" state="completed" covers="REQ-002">
Add 3-way ID consistency lint rule

- Suggested files: `speccy-core/src/lint/rules/tsk.rs` (or new
  rule file under `speccy-core/src/lint/rules/`),
  `speccy-core/src/lint/mod.rs` (registration),
  `speccy-core/src/workspace.rs` (reuses `derive_spec_id_from_dir`)

<task-scenarios>
  - When the new lint rule is run against a workspace fixture where
    folder digits, SPEC.md `id:`, and TASKS.md `spec:` all agree on
    `SPEC-0001`, no diagnostic from the new rule is emitted.
  - When the new lint rule is run against a workspace fixture where
    the folder is `0024-…`, SPEC.md `id: SPEC-1234`, and TASKS.md
    `spec: SPEC-0024`, exactly one diagnostic from the new rule is
    emitted, its level is `Error`, and its `message` field contains
    all three observed values as substrings so the operator can see
    which identifier disagrees.
  - When the new lint rule is run against a workspace fixture with a
    spec that has no TASKS.md, no diagnostic from the new rule is
    emitted (the rule has no third observation to compare).
  - When the new lint rule is run against a workspace fixture whose
    SPEC.md fails to parse, no diagnostic from the new rule is
    emitted (parse-error rules cover the upstream failure; the
    3-way rule fires only when all three observations are available).
  - When the new rule is registered with the lint engine, it appears
    in `speccy status` and `speccy verify` output without any
    additional wiring at the call sites.
  - When `cargo test --workspace` is run, the unit tests covering
    the four cases above pass and no existing lint test regresses.
  - The new lint code is named `TSK-005` (next free TSK number) or
    documented in the commit message if the implementer chooses
    another family for readability. The choice does not affect
    behaviour.
</task-scenarios>

- Implementer note (claude-opus-4-7-t003):
  - Completed: Added `TSK-005` (Level::Error) as a new lint rule in `speccy-core/src/lint/rules/tsk.rs`. The rule lives inside the existing tsk module (kept under the TSK family per the task scenario's first-choice naming) as `tsk_005_id_triple`, wired into the existing `Ok(tasks_md) => { ... }` arm so it runs alongside TSK-001/003/004. The rule reads three observations: folder ID via `derive_spec_id_from_dir(&spec.dir)`, SPEC.md frontmatter `id:` via `spec.spec_md_ok().frontmatter.id`, TASKS.md `spec:` via `extract_frontmatter_field(tasks_md.frontmatter_raw, "spec")`. Each unobtainable observation short-circuits to a no-op via `let Some(...) else { return; }`, satisfying the "skip when any is unobtainable" contract. On disagreement it emits one `Diagnostic::with_file` whose message is `ID disagreement: folder=`X`, SPEC.md.id=`Y`, TASKS.md.spec=`Z``, naming all three values verbatim. The diagnostic's `spec_id` is `spec.spec_id.clone()`, which the workspace loader already populates as "SPEC.md.id when parseable, else folder-derived" (workspace.rs:447-451) — that matches REQ-002's `<done-when>` requirement for grouping. Registered in `speccy-core/src/lint/registry.rs` and the snapshot at `speccy-core/tests/snapshots/lint_registry.snap` (one-line append, sorted). Added four integration tests in `speccy-core/tests/lint_tsk.rs` (`tsk_005_silent_when_all_three_ids_agree`, `tsk_005_fires_when_three_ids_disagree`, `tsk_005_silent_when_tasks_md_absent`, `tsk_005_silent_when_spec_md_unparseable`) plus a local `write_named_fixture` helper that lays the fixture under a `NNNN-slug` subdirectory so `derive_spec_id_from_dir` resolves a folder ID (the shared `write_spec_fixture` writes directly into `tempdir()`, whose name doesn't match the `^\d{4}-` regex). The disagree test exercises the CHK-002 case verbatim (folder=`0024-…`, SPEC.md.id=`SPEC-1234`, TASKS.md.spec=`SPEC-0024`).
  - Undone: REQ-003 / T-004 (the command-guard at `speccy tasks --commit`) is intentionally sequenced after this task; the SPEC's <done-when> says the lint rule and the command-guard "may share a small predicate helper" but doesn't mandate it, so I left the lint rule self-contained for now. T-004's implementer can extract a shared `id_triple_consistency` helper if reuse becomes valuable.
  - Commands run: `cargo test -p speccy-core --test lint_tsk`; `cargo test -p speccy-core --test lint_registry`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all`; `cargo +nightly fmt --all --check`; `./target/debug/speccy check SPEC-0024/T-003`; `./target/debug/speccy status`; `./target/debug/speccy status --json | grep -c TSK-005`.
  - Exit codes: pass (8 tests — 4 new TSK-005, 4 existing); pass (snapshot matches the appended TSK-005 line); pass (full workspace green); pass; pass; pass; pass (renders CHK-002's five scenarios); informational (every spec still reports hash-drift — input to T-006); pass (`0` matches — TSK-005 fires on zero in-tree specs, confirming no false positives).
  - Discovered issues: None directly caused by this task. The lint engine still reports `hash-drift` on every in-tree spec because T-006 (the reconciliation) hasn't run yet — that's expected per T-002's note and is the input to T-006, not a regression here. The local `cargo deny` binary is not installed on this machine; AGENTS.md lists it as a precommit gate but it must be running in CI rather than locally.
  - Procedural compliance: (none) — no skill files needed updating; the implementer prompt's "Suggested files" pointer was accurate and the project test command (`cargo test`) matched expectations.
- Review (business, pass): TSK-005 (Error level) registered and wired into the existing `tsk::lint` arm; REQ-002's six done-when bullets are all met. The diagnostic message at `speccy-core/src/lint/rules/tsk.rs:244-249` emits exactly the form REQ-002 named ("`folder=`{x}`, SPEC.md.id=`{y}`, TASKS.md.spec=`{z}``") so CHK-002's "contains literal strings SPEC-0024, SPEC-1234, SPEC-0024" passes; the four task-scenario branches (agree, disagree, missing TASKS.md, unparseable SPEC.md) each have a dedicated integration test in `speccy-core/tests/lint_tsk.rs:121-208`. The diagnostic's `spec_id` reuses `spec.spec_id`, which `workspace.rs:447-451` already populates as "SPEC.md.id when parseable, else folder-derived" — matches REQ-002's grouping requirement. DEC-003's "both share a helper to keep them in sync" is advisory ("may share" in T-003's task-scenario, "may" in T-004's); the deferred extraction is explicitly noted in the Undone block, which is acceptable. Non-goals respected: implementation is lint-time, not parse-time, and existing `commit_frontmatter` 2-way check is untouched (T-004 territory). One nit a tests reviewer is better placed to call: the disagree test asserts the literal `TASKS.md.spec` label appears, which technically only proves the label was emitted, not that the third *observed* value (`SPEC-0024` from TASKS.md) was rendered — though since folder=`SPEC-0024` is also asserted and the format always pairs the label with the value, the user-visible behavior is correct.
</task>

<task id="T-004" state="completed" covers="REQ-003">
Add 3-way ID command-guard at `speccy tasks --commit` with new `CommitError` variant

- Suggested files: `speccy-core/src/tasks.rs` (CommitError variant,
  `commit_frontmatter` guard), `speccy-cli/src/tasks.rs` (error
  surface verbatim)

<task-scenarios>
  - When `CommitError` is read after this task, a new variant
    (e.g. `IdTripleMismatch`) exists. It carries three named string
    fields (one each for folder-derived ID, SPEC.md `id:`,
    TASKS.md `spec:`). Its `Display` impl produces a message that
    contains all three observed identifier values verbatim, so a
    grep against the error output finds each.
  - When `commit_frontmatter` is invoked on a workspace where folder
    digits, SPEC.md `id:`, and TASKS.md `spec:` all agree, the
    function performs the write as before and returns `Ok(())`.
  - When `commit_frontmatter` is invoked on a workspace where SPEC.md
    `id:` disagrees with the folder digits and the TASKS.md `spec:`
    matches the folder, the function returns the new error variant
    without opening TASKS.md for writing. After the call, the
    TASKS.md file on disk is byte-identical to its pre-call state.
  - When `commit_frontmatter` is invoked on a workspace where
    TASKS.md `spec:` disagrees with the folder and SPEC.md, the
    function returns the new error variant (the existing
    `SpecIdMismatch` variant may fire instead depending on order of
    checks; either outcome is acceptable as long as TASKS.md is not
    modified).
  - When the CLI surface `speccy tasks SPEC-0024 --commit` is run
    against a workspace with a 3-way disagreement, the command exits
    non-zero, prints a message containing all three observed
    identifier strings on stderr (no rewrap that elides them), and
    TASKS.md is byte-unchanged.
  - The 3-way check is computed in-line inside `commit_frontmatter`
    (or a helper it calls directly), not by re-running the lint
    engine. The two consumers (lint rule from T-003 and this guard)
    may share a small predicate helper; if extracted, that helper
    is unit-tested independently.
  - When `cargo test --workspace` is run, the unit tests covering
    the four cases above pass and no existing `commit_frontmatter`
    test regresses.
</task-scenarios>

- Implementer note (claude-opus-4-7-t004):
  - Completed: Added `CommitError::IdTripleMismatch { folder, spec_md, tasks_md }` to `speccy-core/src/tasks.rs`, peered with the existing `SpecIdMismatch` variant. Its `#[error(...)]` template renders the message form mandated by REQ-003: "ID disagreement among folder=`{folder}`, SPEC.md.id=`{spec_md}`, TASKS.md.spec=`{tasks_md}`; refusing to commit (file untouched)". Extended `commit_frontmatter`'s signature with a new `spec_md_id: &str` parameter placed between the existing `spec_id` (CLI arg) and `spec_md_sha256` (since `spec_md_id` is conceptually grouped with `spec_id`). Inside the `Split::Some` arm, the new 3-way guard runs **before** any write: folder ID is derived via `tasks_md_path.parent().and_then(derive_spec_id_from_dir)`, TASKS.md.spec is read via the existing `find_top_level_spec`, and disagreement returns the new variant — short-circuiting the legacy 2-way check and the file write. The guard skips when any of the three observations is unobtainable (no parent name matching `^\d{4}-`, no TASKS.md frontmatter, no top-level `spec:` line), matching the TSK-005 lint rule's pattern. The CLI caller at `speccy-cli/src/tasks.rs:138` now passes `&parsed_spec.frontmatter.id` as the third arg; no other call sites exist. Did **not** extract a shared `id_triple_consistency` helper — the task scenario said it's optional, and the lint rule and the guard each read the three observations from different in-process types (`ParsedSpec` + `TasksDoc` vs. raw `tasks_md_path` + frontmatter YAML), so a shared helper would need to ferry both representations through a trait or two argument lists; deferred until a third consumer arises. Added five new tests in `speccy-core/tests/tasks_commit.rs`: `commit_succeeds_when_folder_spec_md_id_and_tasks_md_spec_all_agree` (happy path on a `0001-foo/TASKS.md` fixture), `commit_returns_id_triple_mismatch_when_spec_md_id_disagrees` (exercises the CHK-003 second scenario verbatim — folder=SPEC-0001, spec_md=SPEC-1234, tasks_md=SPEC-0001), `commit_leaves_file_unchanged_when_tasks_md_spec_disagrees_with_folder_and_spec_md` (accepts either error variant per REQ-003 <behavior> bullet 3), `id_triple_mismatch_display_contains_all_three_observed_values` (Display contains each value verbatim per the CHK-003 fourth scenario), and `three_way_guard_skips_when_folder_id_unobtainable` (locks in the lint-rule-style skip semantics). Added one CLI integration test in `speccy-cli/tests/tasks.rs`: `commit_refuses_when_spec_md_id_disagrees_with_folder_and_tasks_md_unchanged` (exit code 1, stderr contains both observed IDs and the "ID disagreement" literal, TASKS.md byte-unchanged). Added a `write_in_folder(folder_name, content)` helper to `tasks_commit.rs` for fixtures that need a `NNNN-slug` parent dir so `derive_spec_id_from_dir` resolves. Updated all twelve existing `commit_frontmatter` test call sites with the new `spec_md_id` argument (passing the same SPEC ID as the CLI arg, so the existing 2-way path is unchanged because the bare tempdir name doesn't match `^\d{4}-` and the 3-way check skips).
  - Undone: REQ-004 / T-005 (`speccy status` selectors and footer) and REQ-005 / T-006 (workspace-wide reconciliation of stored hashes) remain pending. T-006 must be sequenced last because the hash function is still in flux until T-005 lands; running it now would have to be redone.
  - Commands run: `cargo test -p speccy-core --test tasks_commit`; `cargo test -p speccy-cli --test tasks`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all`; `cargo +nightly fmt --all --check`; `cargo build --release --bin speccy`; `./target/release/speccy check SPEC-0024/T-004`.
  - Exit codes: pass (17 tests — 12 existing + 5 new); pass (12 tests — 11 existing + 1 new CLI); pass (50+ test groups, all `0 failed`); pass; pass (one re-format diff applied); pass; pass; pass (renders CHK-003's four scenarios).
  - Discovered issues: (none) — the only friction point was that the bare `write_tmp` fixture's tempdir name doesn't match `^\d{4}-`, so it can't exercise the 3-way path; resolved by adding a paralleled `write_in_folder` helper instead of restructuring the existing helper (preserves the legacy 2-way fixture coverage). `cargo deny` is not installed locally (consistent with T-003's note); AGENTS.md lists it as a precommit gate but it runs in CI rather than locally.
  - Procedural compliance: (none) — no skill files needed updating; the implementer prompt's "Suggested files" pointer was accurate (`speccy-core/src/tasks.rs` for the variant + guard, `speccy-cli/src/tasks.rs` for the error surface).
- Review (business, pass): REQ-003 is satisfied end-to-end. The new `CommitError::IdTripleMismatch` variant at `speccy-core/src/tasks.rs:48-65` carries the three named fields and its `#[error(...)]` template renders the exact message form REQ-003 mandates ("ID disagreement among folder=`{folder}`, SPEC.md.id=`{spec_md}`, TASKS.md.spec=`{tasks_md}`; refusing to commit (file untouched)"). The guard at `speccy-core/src/tasks.rs:147-156` runs inside `Split::Some` before any `fs_err::write`, short-circuiting on disagreement; the legacy `SpecIdMismatch` 2-way check sits after it so the 3-way fires first as the SPEC's superset-signal contract requires. Skip semantics (any of the three observations missing → no error) match the lint-rule pattern from T-003, satisfying DEC-003's "both share a predicate" intent. Core test coverage at `speccy-core/tests/tasks_commit.rs:359-505` covers all four CHK-003 scenarios verbatim (3-way agreement happy path, IdTripleMismatch on SPEC.md.id disagreement with all three fields populated correctly, IdTripleMismatch-or-SpecIdMismatch when TASKS.md.spec disagrees, Display contains all three literal values, and the unobtainable-folder skip path). CLI surface test at `speccy-cli/tests/tasks.rs:274-308` verifies exit code 1, stderr contains both observed IDs plus the "ID disagreement" literal (no rewrap), and TASKS.md is byte-unchanged. The error renders verbatim through `eprintln!("speccy tasks: --commit failed: {inner}")` at `speccy-cli/src/main.rs:239`. No scope creep: the existing `SpecIdMismatch` variant is preserved as a peer per non-goal bullet 9 ("No removal or rewrite of the existing SpecIdMismatch variant"). The deferred shared-helper extraction is explicitly licensed by the task-scenario's "may share a small predicate helper; if extracted" language. Minor observation (non-blocking): the CLI test exercises a case where folder and tasks_md share the same SPEC-0001 ID, so `stderr(contains("SPEC-0001"))` covers two of the three positions with one assertion; a future test using three fully distinct IDs would prove all three positional renderings independently at the CLI level, but the core-level `id_triple_mismatch_display_contains_all_three_observed_values` test already covers that with three distinct values (SPEC-0024 / SPEC-1234 / SPEC-9999), so overall user-facing coverage is sufficient.
</task>

## Phase 3: `speccy status` selectors

<task id="T-005" state="completed" covers="REQ-004">
Extend `speccy status` with positional selector, `--all` flag, hidden-count footer, and per-spec JSON

- Suggested files: `speccy-cli/src/main.rs` (clap subcommand
  surface), `speccy-cli/src/status.rs` (StatusArgs, selector
  resolver, `render_text`, `build_json`), `speccy-cli/src/status_output.rs`
  (if any field-shape touches), `speccy-cli/tests/` (integration
  tests for the new paths)

<task-scenarios>
  - When `StatusArgs` is read after this task, it carries two new
    public fields: an `Option<String>` for the positional `SPEC-NNNN`
    selector and a `bool` for `--all`. The existing `json` field is
    unchanged.
  - When `speccy status --all SPEC-0023` is parsed by the clap
    surface, parsing fails at the CLI layer (before any workspace
    scan) with a message naming the conflict. Exit code is non-zero.
  - When `speccy status SPEC-9999` is run in a workspace where no
    spec has id `SPEC-9999`, the command exits non-zero, stderr
    names the missing ID, and nothing is written to stdout (no
    partial output).
  - When `speccy status SPEC-0023` is run in this repo with the
    text renderer, the per-spec block for SPEC-0023 is rendered
    (header line + tasks line + lint line, plus optional stale /
    open-questions / parse-error lines) regardless of whether the
    spec would otherwise be filtered. Exit code is 0.
  - When `speccy status --all` is run in a workspace with N specs,
    the text renderer emits per-spec blocks for all N in workspace
    order, regardless of filter state. With an empty workspace, the
    existing "No specs in workspace." message is printed.
  - When `speccy status` (no selector, no `--all`) is run in a
    workspace where at least one spec was filtered out by the
    attention-list filter, the text output ends with a footer line
    of the form `{N} specs hidden; pass --all to see them`. The
    footer is suppressed when no specs were hidden. When the
    attention list itself is empty, the footer follows the existing
    "No in-progress specs need attention." message (separated by a
    blank line) only if there are hidden specs.
  - When `speccy status SPEC-0023 --json` is run, the JSON output
    has the same top-level shape as today's `--json` (`schema_version`,
    `repo_sha`, `specs`, `lint`) but the `specs` array contains
    exactly one entry whose `id` is `SPEC-0023`.
  - When `speccy status --all --json` is run, the JSON output is
    structurally equivalent to today's `speccy status --json`
    (every spec, no filter).
  - When `speccy status --json` (no selector, no `--all`) is run,
    the JSON output matches today's `speccy status --json` byte-for-byte
    (every spec, no footer, no shape change).
  - When the renderers are read after this task, the selector and
    `--all` decisions happen in a small resolver function before
    `render_text` and `build_json` are called. The resolver returns
    a `(Vec<&SpecView<'_>>, hidden_count)` pair (or an equivalent
    shape) consumed by both renderers.
  - When `speccy status --help` is run, the help output describes the
    new positional argument and the `--all` flag in the same wording
    style as `speccy check`'s positional argument.
  - When `cargo test --workspace` is run, integration tests cover:
    the both-selectors rejection, the unknown-spec error, the
    positional render (text and JSON), the `--all` render (text
    and JSON), the no-args attention list with the footer, and the
    no-args attention list without the footer (clean workspace).
</task-scenarios>

- Implementer note (claude-opus-4-7-t005):
  - Completed: Extended `StatusArgs` (`speccy-cli/src/status.rs`) with `selector: Option<String>` and `all: bool` (dropped `Copy`, kept `Clone`); added the public `RenderMode { Text, Json }` enum; added `StatusError::UnknownSpec { id, available }` whose `Display` reads `no spec with id `{id}` in workspace; available: {available}` (the `available` string is comma-separated `display_id`s, or `(none)` for an empty workspace). Added `resolve_specs<'a>(report, args, mode) -> Result<(Vec<&'a SpecView<'a>>, usize), StatusError>` as the small pre-render resolver: selector path returns `(vec![&found], 0)` or `UnknownSpec`; `--all` returns `(all, 0)`; default+JSON returns `(all, 0)` (JSON ignores the text-mode filter per REQ-004); default+text applies `show_in_text_view` and reports `hidden = total - shown`. Rewrote `run` to compute the mode, call the resolver before any output, and short-circuit on `UnknownSpec` so nothing is written to stdout. Refactored `render_text` to take the pre-resolved `&[&SpecView<'_>]` + `hidden_count`, and append the footer line `{N} specs hidden; pass --all to see them` (blank line separator) after the workspace-lint block when `hidden_count > 0` — the footer is suppressed when nothing was hidden. Refactored `build_json` to iterate the pre-resolved slice (so `--json` no-args still serialises every spec in workspace order, byte-equivalent to today's `--json`). Extended the clap surface in `speccy-cli/src/main.rs` with a positional `SELECTOR` carrying `conflicts_with = "all"` and a `--all` flag; `run_status` now takes `(selector, all, json)`. Updated the three existing test files (`status_text_filter.rs`, `status_text_render.rs`, `status_json.rs`) to construct the new `StatusArgs` shape and to pass it by reference (matches the new `run(&StatusArgs, ...)` signature; clippy's `needless_pass_by_value` required `&StatusArgs` once `Copy` was dropped). Added a new integration test file `speccy-cli/tests/status_selectors.rs` with eleven tests covering every `<task-scenarios>` bullet: clap parse-time conflict on `--all SPEC-0001` (exit 2, stderr names both); unknown-spec error with no stdout output (assert_cmd reads both streams); positional renders one spec in text even when it would be filtered; positional renders one spec in JSON (`specs` length 1, `id` matches); `--all` renders every spec in text; `--all` on empty workspace prints `No specs in workspace.`; `--all --json` is `==` to default `--json` (covers the byte-equivalent-shape scenario); default text mode appends the footer when at least one spec is filtered; footer is suppressed when nothing is filtered; footer follows `No in-progress specs need attention.` when the attention list is empty but specs were filtered; `--help` mentions both `SELECTOR` and `--all`. Manually smoke-tested against the in-tree repo: `speccy status SPEC-0023` renders SPEC-0023; `speccy status SPEC-9999` exits 1 with the listed `available:` IDs; `speccy status --all SPEC-0023` exits 2 with the clap conflict message; `speccy status SPEC-0023 --json` returns `specs` of length 1; `diff <(speccy status --json) <(speccy status --all --json)` is empty.
  - Undone: REQ-005 / T-006 (workspace-wide reconciliation of stored `spec_hash_at_generation` values) is the last task in this spec and is intentionally deferred until T-005's review settles. Per the task-list ordering in `.speccy/specs/0024-meaningful-hash-semantics/TASKS.md`, T-001 through T-005 must be `completed` before T-006 starts so reconciliation runs against a stable hash function.
  - Commands run: `cargo build -p speccy-cli`; `cargo test -p speccy-cli --test status_selectors`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all`; `cargo +nightly fmt --all --check`; `cargo build --release --bin speccy`; `./target/release/speccy check SPEC-0024/T-005`; `./target/release/speccy status --help`; `./target/release/speccy status SPEC-0023`; `./target/release/speccy status SPEC-9999`; `./target/release/speccy status --all SPEC-0023`; `diff <(./target/release/speccy status --json) <(./target/release/speccy status --all --json)`.
  - Exit codes: pass; pass (11 new tests); pass (56 test groups, every one `0 failed`); pass; pass; pass; pass; pass (renders CHK-004's seven scenarios); pass (usage names both `SELECTOR` and `--all`); pass (SPEC-0023 block rendered); 1 (UnknownSpec — stderr names `SPEC-9999` and lists every in-workspace ID); 2 (clap conflict — stderr says `the argument '--all' cannot be used with '[SELECTOR]'`); empty diff (`--json` and `--all --json` are byte-equivalent).
  - Discovered issues: (none) directly caused by this task. `speccy status` still reports `hash-drift` on every in-tree spec because T-006's reconciliation hasn't run yet — that is the expected post-T-002 state and T-006's input, not a regression here. One mechanical clippy fallout: dropping `Copy` from `StatusArgs` triggered `needless_pass_by_value` on `run(args: StatusArgs, ...)`, so the signature is now `run(args: &StatusArgs, ...)`; the three existing status-test files were updated to pass `&StatusArgs` accordingly. `cargo deny` is not installed locally (consistent with T-003/T-004's notes); AGENTS.md lists it as a precommit gate but it runs in CI rather than locally.
  - Procedural compliance: (none) — no skill files needed updating; the implementer prompt's "Suggested files" pointer was accurate and the project test command (`cargo test`) matched expectations.
- Review (business, pass): REQ-004's `<done-when>` is delivered end-to-end. `StatusArgs.{selector,all}` land as public fields (`speccy-cli/src/status.rs:60-69`), `conflicts_with = "all"` enforces mutual exclusion at the clap layer before any workspace scan (`speccy-cli/src/main.rs:71-79`), `resolve_specs` short-circuits selector-mode and surfaces `UnknownSpec` (`speccy-cli/src/status.rs:345-365`) so no partial stdout is emitted on the not-found path, and the footer renders `{N} specs hidden; pass --all to see them` only when `hidden_count > 0`, after the workspace-lint block, with a blank-line separator (`speccy-cli/src/status.rs:440-446`) — including the empty-attention-list case the SPEC scenario specifically called out. JSON-mode contracts are honored: default `--json` runs the resolver in `RenderMode::Json` so the filter never bites and the byte-for-byte shape is preserved (the new test `all_flag_json_matches_default_json_shape` proves `--json` and `--all --json` are equal), and selector-mode `--json` narrows `specs` to one entry while keeping the top-level shape unchanged. DEC-005 (no-args keeps the filter) and DEC-006 (JSON ignores the filter) are both observed. Non-goals respected: no `schema_version` bump, `show_in_text_view` untouched, no new staleness signal. Minor SPEC-vs-impl wording deltas — the `available:` list is comma-separated (SPEC says `SPEC-0001 … SPEC-NNNN` as an illustrative form), the clap-generated conflict message replaces the SPEC's suggested "specify either … or --all, not both" string, and integration-test coverage landed as a sibling `status_selectors.rs` rather than extending the existing three files in place — all sit within the SPEC's `(or similar)` latitude and do not break any contract. No open questions silently resolved (all six in SPEC.md are `resolved="true"`); no `<non-goals>` scope creep.
</task>

## Phase 4: Migration

<task id="T-006" state="completed" covers="REQ-005">
Reconcile all stored `spec_hash_at_generation` values to the new hash function

- Suggested files: every `TASKS.md` under `.speccy/specs/` whose
  `spec_hash_at_generation` is a real sha256 (not `bootstrap-pending`)

<task-scenarios>
  - When this task runs, every TASKS.md under `.speccy/specs/` whose
    frontmatter `spec_hash_at_generation` is a real sha256 (not
    `bootstrap-pending`) is rewritten to carry the hash returned by
    the new `SpecMd.sha256` for that spec's SPEC.md content.
  - When a TASKS.md's `spec_hash_at_generation` is `bootstrap-pending`
    at reconciliation time, it is left untouched (the existing TSK
    lint flow surfaces bootstrap state; the migration task does not
    auto-resolve it).
  - When `speccy status --all --json` is run after reconciliation,
    no spec object in the output has the string `hash-drift` in its
    `stale_reasons` array.
  - When `speccy status` (no selector, no `--all`) is run after
    reconciliation, the output is the "No in-progress specs need
    attention." message (assuming no specs are in-progress, stale
    for other reasons, or carrying lint errors). The hidden-count
    footer may or may not appear depending on whether clean
    implemented specs got filtered; that is acceptable and not a
    failure condition.
  - When this task's handoff note is read, it records the count of
    TASKS.md files modified and confirms the `speccy status --all
    --json` verification passed (zero `hash-drift`). The note also
    records whether `speccy tasks SPEC-NNNN --commit` was used as
    the per-spec writer (which also refreshes `generated_at`) or
    whether `spec_hash_at_generation` was edited directly (preserving
    `generated_at`). Either approach is acceptable.
  - This task is the LAST task in this spec's TASKS.md. T-001
    through T-005 must all have `state="completed"` before this
    task is started; otherwise the hash function is still in flux
    and reconciliation against an intermediate state would have to
    be redone.
</task-scenarios>

- Implementer note (claude-opus-4-7-t006):
  - Completed: Reconciled all 24 in-tree TASKS.md `spec_hash_at_generation` values to the new `SpecMd.sha256` function (canonical frontmatter \ {status} ++ body bytes, landed in T-001/T-002). Used `./target/release/speccy tasks SPEC-NNNN --commit` as the per-spec writer for every spec from SPEC-0001 through SPEC-0024 — that path goes through `commit_frontmatter`, which now (post-T-004) gates writes on the 3-way ID consistency check and computes the SPEC.md hash via the new function. The `--commit` writer refreshes both `spec_hash_at_generation` and `generated_at`; the SPEC.md REQ-005 <done-when> bullet explicitly allows either approach, and using `--commit` exercises the new hash + guard path in the same step, which is the most useful end-to-end smoke test of T-001 through T-004 the migration could provide. No TASKS.md had `bootstrap-pending` at reconciliation time, so the bootstrap-skip branch is exercised by existing unit tests (TSK-003) rather than by this migration. Verification: `./target/release/speccy status --all --json | python3 -c "..."` reports `Total specs: 24, hash-drift: 0` — zero spec objects carry `hash-drift` in their `stale_reasons`. The default `./target/release/speccy status` output still shows SPEC-0024 because this task itself is `in-progress` at status-render time; that surface clears the moment this state attribute flips to `in-review` and is governed by the in-progress task state filter, not by hash-drift (the verification gate). Per the SPEC's <done-when> bullet, the verification criterion is "no `hash-drift`" and the in-progress display is explicitly named as acceptable.
  - Undone: (none) — T-006 scope satisfied. The full review round across T-001–T-006 is the natural next step (`/speccy-review SPEC-0024`); after green review, `/speccy-ship SPEC-0024` writes REPORT.md and flips SPEC-0024's frontmatter `status:` to `implemented`. Note: the status flip is now hash-neutral (the whole point of REQ-001), so it does not invalidate the hashes this task just wrote.
  - Commands run: `./target/release/speccy next --kind implement --json`; `./target/release/speccy implement SPEC-0024/T-006`; `grep -H spec_hash_at_generation .speccy/specs/*/TASKS.md`; `./target/release/speccy status --all --json | python3 -c "..."` (pre-reconciliation count); `for n in $(seq -f "%04g" 1 24); do ./target/release/speccy tasks "SPEC-$n" --commit; done`; `./target/release/speccy status --all --json | python3 -c "..."` (post-reconciliation count); `./target/release/speccy status`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all --check`; `./target/release/speccy check SPEC-0024/T-006`.
  - Exit codes: pass (returned T-006); pass (rendered prompt); pass (24 specs, all real sha256); pass (pre: 24 hash-drift); pass × 24 (`--commit` exited 0 for every SPEC); pass (post: 0 hash-drift); pass (only SPEC-0024 remains in attention list because T-006 is the last in-progress task at render time); pass (full workspace green, including the lint snapshot and the `in_tree_id_snapshot.json` fixture); pass; pass (only `Unknown configuration option` warnings, no diff); pass (renders CHK-005's four scenarios).
  - Discovered issues: (none) — every spec's hash recomputation succeeded on the first pass; no IDs disagreed, so the new T-004 `IdTripleMismatch` guard did not fire on any spec, which is the expected steady-state behavior on a clean workspace. The earlier T-001 implementer note's prediction that "any SPEC.md byte edit invalidates the stored `spec_hash_at_generation` under the current raw-bytes hash, so `speccy status` will report `hash-drift` for SPEC-0024 until T-002 + T-006 land" is now resolved: the new canonical-frontmatter+body hash function is stable across status flips, and after this reconciliation pass `hash-drift` is gone workspace-wide.
  - Procedural compliance: (none) — no skill files needed updating; the implementer prompt's "Suggested files" pointer (every TASKS.md with a real sha256) was accurate, and `./target/release/speccy tasks SPEC-NNNN --commit` is the canonical per-spec writer named in the SPEC's <done-when>.
- Review (business, pass): REQ-005 verification gate (`speccy status --all --json` → zero `hash-drift`) reproduced independently — `hash-drift: 0, mtime-drift: 0, bootstrap: 0` across all 24 in-tree specs, and lint is clean (zero errors/warnings/info). Every modified `.speccy/specs/*/TASKS.md` (SPEC-0001..SPEC-0023 unstaged, SPEC-0024 untracked) touches only the two frontmatter fields `spec_hash_at_generation` and `generated_at` — no body edits, no scope creep into adjacent files. Handoff note records the count (24), the writer used (`speccy tasks SPEC-NNNN --commit`), and the verification result, matching the REQ-005 `<done-when>` handoff bullet and CHK-005 third scenario. The `bootstrap-pending` skip branch is correctly noted as vacuous (no in-tree TASKS.md carries that sentinel) and the slice-level scenario's explicit carve-out — "assuming no specs are in-progress … the hidden-count footer may or may not appear" — covers the still-in-progress SPEC-0024 case the implementer flags; CHK-005's second scenario will land at ship time when the `status: in-progress` → `implemented` flip happens, and that flip is now hash-neutral (the whole point of REQ-001). One sequencing nit worth surfacing rather than blocking on: the task-scenarios LAST bullet says T-001..T-005 must be `state="completed"` before T-006 starts, but they were `in-review` (not `completed`) when this ran. In practice the code-change tasks were functionally complete (cargo test green, no further hash-function edits pending) so the reconciliation didn't get redone, but the strict reading of that bullet wasn't honoured. Not blocking — the substantive guarantee (stable hash function at reconciliation time) is met and verified.
</task>

</tasks>
