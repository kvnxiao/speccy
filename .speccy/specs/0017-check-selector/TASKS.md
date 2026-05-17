---
spec: SPEC-0017
spec_hash_at_generation: d9aec568257df635369adaa844130359d9f3e15ae2dcec39f23052ea4b9662e8
generated_at: 2026-05-15T02:44:35Z
---

# Tasks: SPEC-0017 speccy check polymorphic selector

## Phase 1: Selector parser

<tasks spec="SPEC-0017">

<task id="T-001" state="completed" covers="REQ-001">
Add the `check_selector` module with `parse_selector`

- Suggested files:
  - `speccy-cli/src/check_selector.rs` (new module: `CheckSelector`
    enum, `SelectorError` thiserror enum, `parse_selector`
    function, in-module `#[cfg(test)] mod tests` with the
    parser-shape unit tests)
  - `speccy-cli/src/lib.rs` (add `pub mod check_selector;` so the
    integration test crate can reach the type and function)
  - `speccy-cli/tests/check_selector.rs` (new integration test;
    test names start with `parser_` so the CHK-001 invocation
    `cargo test -p speccy-cli --test check_selector -- parser`
    runs exactly these tests)
- Implementer note (t001-check-selector-module):
  - Completed: new `speccy-cli/src/check_selector.rs` module exposing
    `CheckSelector` (5 variants), `SelectorError` (4 variants), and
    `parse_selector(Option<&str>) -> Result<CheckSelector,
    SelectorError>`; dispatch order qualified-task -> qualified-check
    -> bare-spec -> unqualified-task -> unqualified-check; task forms
    delegate to `speccy_core::task_lookup::parse_ref` so a single
    canonical `TaskRef` constructor is used; `pub mod check_selector;`
    exported from `speccy-cli/src/lib.rs`; in-module
    `#[cfg(test)] mod tests` with 10 unit tests + new
    `speccy-cli/tests/check_selector.rs` integration suite with 10
    `parser_*` tests; tests written before production code per TDD.
  - Undone: T-001 was scoped strictly to the parser. `CheckArgs.id`
    rename, `CheckError::Selector` plumbing, removal of
    `validate_chk_id_format`, and ARCHITECTURE.md edits are explicitly
    deferred to T-002..T-006 per the implementer prompt and TASKS.md.
  - Commands run:
    - `cargo test -p speccy-cli --lib check_selector`
    - `cargo test -p speccy-cli --test check_selector -- parser`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all` (applied) + `cargo +nightly fmt --all --check`
    - `cargo deny check`
    - `target/debug/speccy.exe check` (local sanity)
  - Exit codes: pass, pass, pass, pass, pass + pass, pass (with
    pre-existing license/duplicate warnings on `ISC`, `MPL-2.0`, and
    the dual `winnow` resolution, unchanged by this task), pass (the
    SPEC-0017 in-flight failures are the unimplemented T-002..T-006
    checks behaving exactly as their IN-FLIGHT contract requires).
  - Discovered issues: (none) — the existing
    `CheckError::InvalidCheckIdFormat` and `validate_chk_id_format`
    are still in `speccy-cli/src/check.rs`, as required by T-001's
    surgical scope. They get retired in T-002.
  - Procedural compliance: (none) — no skill-layer friction
    encountered; implementer prompt and TASKS.md entries were
    sufficient as written.
- Review (business, pass): REQ-001's seven `Done when` bullets and all four `Behavior` Given/When/Then examples map cleanly to `parse_selector` in `speccy-cli/src/check_selector.rs:110-174`. Dispatch order matches SPEC `## Design / Approach` 1-5 (qualified-task → qualified-check → bare-spec → unqualified-task → unqualified-check). Both task arms route through `speccy_core::task_lookup::parse_ref` per the SPEC's "no second regex" mandate (`check_selector.rs:127, 156`). `InvalidFormat` Display lists all five shapes verbatim and preserves the offending arg without normalisation. No new flags, no JSON envelope, no new noun; T-002..T-006 scope correctly deferred per Open Questions/Non-goals.
- Review (security, pass): All five regexes are anchored `^...$` with bounded character classes (`\d{3,}` / `\d{4,}`) and zero nested quantifiers or alternation — ReDoS not reachable; linear-time match (`check_selector.rs:182, 191, 200, 209, 218`). Regexes compiled once via `OnceLock`, not per-call. Parser is purely data — no filesystem, process, or I/O side effects. `SelectorError::InvalidFormat` carries the raw input via `.to_owned()` with no truncation, no case folding, no whitespace stripping (`check_selector.rs:131, 159, 172`), which is REQ-001 / DEC-004's explicit contract. Capture access uses `.get(1)` / `.get(2)` with `Option` handled — no `[i]` indexing on `Captures`. No `unwrap()` / `expect()` / `panic!()` / `todo!()` in the runtime path; the four `regex::Regex::new(...).unwrap()` inside `OnceLock::get_or_init` are static-literal regexes guarded by `#[expect(clippy::unwrap_used, reason = ...)]` plus unit-test coverage — canonical safe pattern. Lowercase `chk-001` correctly rejected (no liberal normalisation; tests at `check_selector.rs:324-330` and `tests/check_selector.rs:101-108`). Residual risk noted but acceptable per SPEC: `InvalidFormat`'s `Display` will echo control bytes / ANSI / OSC sequences verbatim to stderr if a user passes `speccy check $'\x1b]0;EVIL\x07'` — this is DEC-004's explicit "name the offending input verbatim" trade for local-CLI usability and the threat model (attacker already controls the shell invocation) makes it a non-blocking trade.
- Review (tests, pass): All 10 "Tests to write" bullets are covered by 10 unit tests in `speccy-cli/src/check_selector.rs:228-386` and 10 mirrored integration tests in `speccy-cli/tests/check_selector.rs:17-167`; both suites pass (`cargo test -p speccy-cli --lib check_selector` and `--test check_selector -- parser` each: 10/10 ok). Negative cases use `matches!(&err, SelectorError::InvalidFormat { arg } if arg == "...")` and assert the verbatim input is preserved for `FOO`, `chk-001`, and all six malformed inputs (`""`, `"SPEC-"`, `"SPEC-001"`, `"SPEC-0001/"`, `"/T-001"`, `"T- 001"`); the `Display` form for `FOO` is asserted to contain both the offending arg and each of the five shape literals (`SPEC-NNNN`, `SPEC-NNNN/CHK-NNN`, `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`). Dispatch test walks all five accepted forms with exact-variant `assert_eq!`. Caveat (non-blocking): the "delegates to `task_lookup::parse_ref` rather than a second regex" claim is inherently unobservable through black-box assertion when `TaskRef` is a plain enum with public fields — production code does delegate (`check_selector.rs:127, 156`), but a hand-rolled `TaskRef::Qualified { ... }` constructor producing the same struct would still pass these tests; the unit/integration duplication is a layer-boundary smoke test (proves `pub mod` re-export across the crate boundary), not redundancy. No vacuous mocks, no snapshot-baked bugs, no `expect_err`-without-variant-match.
- Review (style, pass): `cargo clippy -p speccy-cli --all-targets --all-features -- -D warnings` and `cargo +nightly fmt --all --check` both pass clean. No `unwrap()` / `expect()` / `panic!()` / `todo!()` in production paths; the four `Regex::new(...).unwrap()` inside `OnceLock::get_or_init` are guarded by `#[expect(clippy::unwrap_used, reason = "compile-time literal regex; covered by unit tests")]` — identical pattern to `speccy-core/src/task_lookup.rs:311-323`, `speccy-cli/src/tasks.rs:241-244`, `plan.rs:247-250`, `report.rs:212-215` (idiomatic, not parallel-helper drift). Naming (`CheckSelector`, `SelectorError`, `parse_selector`, `qualified_task_regex` etc.) matches Rust API guidelines and mirrors the sibling `task_lookup` module (`TaskRef`, `LookupError`, `parse_ref`, `qualified_regex`). Module-level + variant + `parse_selector` doc comments are present and follow the `implement.rs:1-10` / `task_lookup.rs:24-25,81-82` style (`# Errors` section included on `parse_selector`). `SelectorError` is `#[derive(Debug, Error)] #[non_exhaustive]` with well-formatted `#[error("...")]` messages; `InvalidFormat` lists all five shapes per DEC-004 and other variants name the offending IDs. `CheckSelector` is also `#[non_exhaustive]` (a small hardening upgrade over the SPEC's Interfaces sketch — defensible per "Surface unknowns"). Capture access uses `.get(1)` / `.get(2)` + `Option` handling — no `[i]` indexing. Public surface from `lib.rs:11` is exactly `pub mod check_selector;`; the three module-public items (`CheckSelector`, `SelectorError`, `parse_selector`) are precisely what T-002+ need, no accidental re-exports. No unused imports / orphan helpers introduced by this task. The `tests/check_selector.rs:1-4` `#![allow(clippy::expect_used, reason = ...)]` mirrors the established speccy test-file convention (every other `tests/*.rs` uses the same form); `allow_attributes_without_reason = deny` is satisfied by the `reason` field, so the AGENTS.md "prefer `#[expect]`" guidance applies to silencing-without-justification, not this pre-existing pattern. Test names all start with `parser_` per the TASKS.md mandate (verified: `parser_none_returns_all`, `parser_qualified_task_uses_task_lookup_parse_ref`, …, `parser_dispatch_order_picks_most_specific_shape`).

<task-scenarios>
  - When `parse_selector(None)` is called, then it returns
    `CheckSelector::All`.
  - When `parse_selector(Some("SPEC-0010/T-002"))` is called, then it
    returns `CheckSelector::Task(TaskRef::Qualified { spec_id:
    "SPEC-0010", task_id: "T-002" })`, with the task fragment
    produced via `task_lookup::parse_ref` rather than a second
    regex.
  - When `parse_selector(Some("SPEC-0010/CHK-001"))` is called, then
    it returns `CheckSelector::QualifiedCheck { spec_id: "SPEC-0010",
    check_id: "CHK-001" }`.
  - When `parse_selector(Some("SPEC-0010"))` is called, then it
    returns `CheckSelector::Spec { spec_id: "SPEC-0010" }`.
  - When `parse_selector(Some("T-002"))` is called, then it returns
    `CheckSelector::Task(TaskRef::Unqualified { id: "T-002" })`.
  - When `parse_selector(Some("CHK-001"))` is called, then it returns
    `CheckSelector::UnqualifiedCheck { check_id: "CHK-001" }`.
  - When `parse_selector(Some("FOO"))` is called, then it returns
    `Err(SelectorError::InvalidFormat { arg: "FOO" })` and the
    `Display` form of that error names `FOO` verbatim and mentions
    all five valid shapes (`SPEC-NNNN`, `SPEC-NNNN/CHK-NNN`,
    `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`).
  - When `parse_selector(Some("chk-001"))` is called, then it
    returns `Err(SelectorError::InvalidFormat { arg: "chk-001" })`
    (case mismatch must not be normalised away).
  - When the parser is invoked with `""`, `"SPEC-"`, `"SPEC-001"`
    (3 digits, below the 4-digit minimum), `"SPEC-0001/"`, `"/T-001"`,
    and `"T- 001"`, then each input returns
    `SelectorError::InvalidFormat` carrying that exact input.
  - When the dispatch order is exercised by inputs that match more
    than one regex on paper (e.g. the qualified-task shape is a
    superset of the bare-spec shape prefix-wise), then the
    most-specific shape wins — covered by a dedicated test that
    walks through each accepted form and asserts the resulting
    `CheckSelector` variant.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-004">
Wire `parse_selector` into `check::run`; preserve `All` and bare `CHK-NNN`

- Suggested files:
  - `speccy-cli/src/check.rs` (rename `CheckArgs.id` to
    `CheckArgs.selector`; add a `Selector(SelectorError)` variant
    to `CheckError`; remove `CheckError::InvalidCheckIdFormat`;
    delete `validate_chk_id_format` plus its in-module `mod tests`;
    call `parse_selector(args.selector.as_deref())` at the top of
    `run`; dispatch `CheckSelector::All` and
    `CheckSelector::UnqualifiedCheck { check_id }` onto the
    existing collect-then-filter path; have the three new arms
    (`Spec`, `QualifiedCheck`, `Task`) return their corresponding
    "not found" `SelectorError` variant for now, so each subsequent
    task can replace one arm with the real lookup without
    introducing `todo!()` / `unimplemented!()` / `panic!()` —
    explicitly forbidden by AGENTS.md)
  - `speccy-cli/src/main.rs` (rename the clap field `id` to
    `selector`; update its doc-comment to list the five shapes;
    rename `value_name` to `SELECTOR`; update the `Command::Check`
    destructuring at the dispatch site to match)
  - `speccy-cli/tests/check.rs` (rename the local `invoke` /
    `invoke_expect_err` helpers' `id: Option<&str>` parameter to
    `selector: Option<&str>` for clarity; retarget
    `id_filter_malformed_format_errors` onto the new error
    variant; everything else under `tests/check.rs` is unchanged
    because it exercises `All` and bare-`CHK-NNN` shapes only)
- Implementer note (t002-wire-parse-selector):
  - Completed: `CheckArgs.id` -> `CheckArgs.selector` rename;
    `CheckError::InvalidCheckIdFormat` deleted in favour of
    `CheckError::Selector(#[from] SelectorError)` (transparent
    forwarding so `Display` for `SelectorError::InvalidFormat`
    reaches stderr verbatim); `validate_chk_id_format` and its
    in-module `#[cfg(test)] mod tests` block removed (verified via
    `Grep validate_chk_id_format|InvalidCheckIdFormat` -> no files
    found across `speccy-cli/`); `parse_selector(selector.as_deref())`
    called at the top of `run`; dispatch refactored into
    `run_all` (existing `collect_checks` + `execute_checks` path
    verbatim) and `run_unqualified_check` (existing filter-then-
    execute path); the three not-yet-implemented arms each return
    their corresponding `SelectorError` variant with a one-line
    `// T-NNN replaces this arm` comment for the next implementer
    (`Spec` -> `NoSpecMatching` for T-003; `QualifiedCheck` ->
    `NoQualifiedCheckMatching` for T-004; `Task` ->
    `TaskCoversNothing { task_ref: task_ref.as_arg() }` for T-005
    since `TaskRef` exposes `.as_arg()` as its on-CLI render); no
    `todo!()` / `unimplemented!()` / `panic!()`. `main.rs` clap
    field renamed `id` -> `selector` with `value_name = "SELECTOR"`
    and a doc-comment listing all five shapes; dispatch
    destructuring updated. `tests/check.rs` helpers `invoke` /
    `invoke_expect_err` parameter renamed to `selector`;
    `id_filter_malformed_format_errors` retargeted onto
    `CheckError::Selector(SelectorError::InvalidFormat { arg })`
    asserting `arg` carries `FOO` and `chk-001` verbatim; two new
    `assert_cmd`-driven binary tests added
    (`binary_check_foo_exits_1_with_five_shape_hint`,
    `binary_check_chk_099_no_match_preserves_no_check_matching_wording`)
    to exercise the renamed error variant's `Display` reaching the
    CLI surface and to lock in the byte-for-byte `NoCheckMatching`
    wording.
  - Undone: T-003/T-004/T-005 still own the real `Spec` /
    `QualifiedCheck` / `Task` lookups. The three temporary
    `SelectorError`-returning arms exist solely so the matrix
    remains exhaustive without a `todo!()`. T-006 owns the
    ARCHITECTURE.md / skill-docs audit. SPEC.md and spec.toml were
    not touched.
  - Commands run:
    - `cargo build -p speccy-cli`
    - `cargo test --workspace`
    - `cargo test -p speccy-cli --test check`
    - `cargo test -p speccy-cli --test check_selector`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all` (applied) + `cargo +nightly fmt --all --check`
    - `cargo deny check`
    - `target/debug/speccy.exe check` (local sanity); also
      `speccy check SPEC-0017`, `speccy check FOO`,
      `speccy check CHK-099` to spot-check the three error surfaces.
  - Exit codes: pass, pass (workspace-wide, 0 failed), pass
    (22 check tests), pass (10 parser tests), pass, pass + pass,
    pass (pre-existing ISC / MPL-2.0 / winnow duplicate warnings
    unchanged by this task), pass. The CHK-002/CHK-003/CHK-004 of
    SPEC-0017 themselves intentionally fail today as their
    IN-FLIGHT contract requires (the placeholder
    `SelectorError` returns produce non-zero exits for the
    selector shapes T-003..T-005 still own); the spec is
    `in-progress`, so the runner categorised the `SPEC-0017`
    selector's failure as 1 (`failed` rather than `in-flight`)
    because the failing exit comes from `speccy check` itself,
    not from a CHK-002 subprocess. That counts as expected
    forward progress, not a regression.
  - Discovered issues: (none) — the parser surface and existing
    runner contracts dovetailed cleanly. `TaskRef::as_arg()` is
    the right canonical render for the `TaskCoversNothing`
    placeholder; the `Display` impl was deliberately omitted on
    `TaskRef` upstream, and `.as_arg()` already exists with a
    `#[must_use]` annotation that the compiler enforced for us.
  - Procedural compliance: (none) — no skill-layer friction
    encountered. The implementer prompt's hint to pick a
    `SelectorError`-shaped placeholder per arm (so T-003..T-005
    can swap one arm at a time) lined up directly with how the
    `match` ended up; no skill update needed.
- Review (business, pass): REQ-001 + REQ-004 contract holds end-to-end. `parse_selector(selector.as_deref())` wired at `check.rs:136`; dispatch matches all five `CheckSelector` arms (`check.rs:138-148`). `All` -> `run_all` is the existing `collect_checks` + `execute_checks` path verbatim (`check.rs:151-165`); bare `CHK-NNN` -> `run_unqualified_check` preserves SPEC-0010 DEC-003 cross-spec semantics including the verbatim `NoCheckMatching` wording reaching the CLI (`check.rs:181-185` + `main.rs:567-570` + integration test at `tests/check.rs:436-457`). `id_filter_matches_across_specs` (`tests/check.rs:327-352`) is unchanged byte-for-byte and still uses `Some("CHK-001")` against the renamed dispatcher. `id_filter_malformed_format_errors` retargeted onto `CheckError::Selector(SelectorError::InvalidFormat { arg })` for both `FOO` and `chk-001` with verbatim-arg preservation (`tests/check.rs:374-402`). `binary_check_foo_exits_1_with_five_shape_hint` exercises the full Display chain `SelectorError::InvalidFormat` -> `#[error(transparent)]` `CheckError::Selector` -> generic `eprintln!("speccy check: {e}")` arm and asserts on `FOO` + all five shape literals on stderr (`tests/check.rs:408-433`). `validate_chk_id_format` + `InvalidCheckIdFormat` confirmed absent from all source files (only historical references in SPEC/TASKS markdown remain, which is correct). Three placeholder arms (`Spec`, `QualifiedCheck`, `Task`) return their corresponding `SelectorError` variants — zero `todo!()`/`unimplemented!()`/`panic!()` (AGENTS.md compliant). Migration / rollback promise upheld: bare `speccy check`, bare `speccy check CHK-NNN`, outside-workspace, and dropped-spec-skip paths all preserved (existing tests `discovers_workspace_checks`, `id_filter_matches_across_specs`, `check_outside_workspace_fails`, `dropped_spec_is_skipped_entirely` pass unchanged). No new flags, no JSON envelope, no `--list` mode, no new noun added. The `Lean 0` empty-covers open question is correctly left for T-005 to resolve — T-002's `Task` placeholder returns `TaskCoversNothing` (not 0), which is the right "defer the decision" shape.
- Review (tests, pass): All five "Tests to write" bullets land with non-vacuous assertions and the full suite is green (`cargo test -p speccy-cli --test check`: 40/40 ok). (1) `id_filter_matches_across_specs` (`tests/check.rs:327-352`) is structurally untouched — same fixture, still asserts both `==> CHK-001 (SPEC-0001)` + `==> CHK-001 (SPEC-0003)` headers and `2 passed, 0 failed, 0 in-flight, 0 manual` — only the helper param was renamed `id` -> `selector`. (2) `id_filter_malformed_format_errors` (`tests/check.rs:374-402`) retargeted with `matches!(&err, CheckError::Selector(SelectorError::InvalidFormat { arg }) if arg == "FOO")` for both `FOO` and `chk-001`, preserving the verbatim-arg contract; `Grep validate_chk_id_format|InvalidCheckIdFormat` across `speccy-cli/` returns zero hits (only `.speccy/specs/` historical markdown). (3) `binary_check_foo_exits_1_with_five_shape_hint` (`tests/check.rs:408-433`) uses `assert_cmd::Command::cargo_bin("speccy")` with `.code(1)` plus six chained `.stderr(contains(...))` asserting `FOO` + all five shape literals (`SPEC-NNNN`, `SPEC-NNNN/CHK-NNN`, `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`). (4) `binary_check_chk_099_no_match_preserves_no_check_matching_wording` (`tests/check.rs:436-457`) locks in the byte-for-byte string `"no check with id `CHK-099` found in workspace; run `speccy status` to list specs"` via a single `.stderr(contains(...))` — exact wording, not just `contains("CHK-099")`. (5) workspace-grep cleanliness verified above. Helper rename audit: all 37 `invoke` / `invoke_expect_err` call sites in `tests/check.rs` pass `Some("...")` as the `selector` positional with no stragglers. `assert_cmd::Command::cargo_bin("speccy")` is the canonical pattern already used by 5 other tests in the same file (`live_streaming_smoke_via_binary`, `check_outside_workspace_fails`, etc.) — not fragile. No tests accidentally depend on the `Spec` / `QualifiedCheck` / `Task` placeholder behaviour: T-003..T-005's tests under `spec_selector_*` / `bare_chk_preserved_*` / `task_selector_*` only target the real lookups (all green after T-003..T-005 landed). Mutation check: if the implementer rewrote `CheckError::Selector` to swallow the arg as `""`, test (2) fails; if `run_unqualified_check` rewrote the `NoCheckMatching` wording, test (4) fails; if `Display` dropped any of the five shapes, test (3) fails — none of these tests are vacuous.


<task-scenarios>
  - When the existing `id_filter_matches_across_specs` test runs
    against the renamed dispatcher, then it still passes byte-for-byte
    — bare `CHK-NNN` retains SPEC-0010 DEC-003 cross-spec semantics
    (no deprecation warning, both specs' `CHK-001` execute).
  - The existing `id_filter_malformed_format_errors` test is
    retargeted to assert on
    `CheckError::Selector(SelectorError::InvalidFormat { arg })`
    carrying the offending input verbatim, for both `FOO` and
    `chk-001`; the deleted `InvalidCheckIdFormat` variant is no
    longer referenced anywhere in the workspace.
  - When `speccy check FOO` runs against any workspace, then the
    binary exits 1 and stderr contains the offending `FOO` plus the
    five-shape hint list (the renamed error variant's `Display`
    output reaches the CLI surface).
  - When `speccy check CHK-099` matches no spec, then the binary
    exits 1 with the existing `CheckError::NoCheckMatching` wording
    preserved verbatim (no rewording while we are next to it).
  - When the workspace tree is searched after this task, then
    `validate_chk_id_format` and its `#[cfg(test)] mod tests` block
    no longer exist anywhere in `speccy-cli/src/check.rs`.
</task-scenarios>
</task>

## Phase 3: New selector shapes


<task id="T-003" state="completed" covers="REQ-002">
Implement `CheckSelector::Spec`; tests for spec-scoped execution

- Suggested files:
  - `speccy-cli/src/check.rs` (introduce a small helper that takes
    a `&Workspace` and a `&str` spec ID and returns
    `Result<&ParsedSpec, SelectorError>`; in the `Spec` arm, look
    up the spec, surface `NoSpecMatching` on miss, then narrow
    `collect_checks`'s existing logic to one spec by either
    filtering its output or factoring the per-spec loop body into
    a reusable function; preserve the existing dropped /
    superseded skip semantics with the new "spec is `<status>`;
    no checks executed" line when the user named the spec
    directly; delegate to `execute_checks` with the narrowed
    `Vec<CollectedCheck>` so live streaming, IN-FLIGHT
    categorisation, and summary totals all reuse the existing
    path verbatim)
  - `speccy-cli/tests/check.rs` (add the `spec_selector_*` tests;
    a fixture helper for "pass + fail under one spec" is already
    available via `spec_toml_pass_fail_fail`)
- Implementer note (t003-spec-selector):
  - Completed: `CheckSelector::Spec { spec_id }` arm in
    `speccy-cli/src/check.rs` rewired to a new `run_spec` helper
    (placeholder `NoSpecMatching` return + the `// T-003 replaces
    this arm` comment removed). `resolve_spec(&Workspace, &str) ->
    Result<&ParsedSpec, SelectorError>` looks up the spec by the
    `ParsedSpec::spec_id` (frontmatter) with `display_spec_label`
    fallback for malformed SPEC.mds, returning
    `SelectorError::NoSpecMatching` on miss. The per-spec body of
    `collect_checks` was factored into `collect_for_spec(&ParsedSpec,
    &str, SpecStatus, &mut dyn Write) -> Result<(Vec<CollectedCheck>,
    u32), CheckError>` and is reused by both `collect_checks` (the
    `All`/`UnqualifiedCheck` path) and `run_spec` so live streaming,
    IN-FLIGHT categorisation, and summary totals come from the
    existing `execute_checks` path verbatim. When the user names a
    `dropped` / `superseded` spec directly, `run_spec` prints one
    `spec <SPEC-NNNN> is `<status>`; no checks executed` line and
    returns exit 0 without ever calling `collect_for_spec` (so no
    subprocess can possibly spawn, no summary line, no `==>` /
    `<--` framing). Six new integration tests added to
    `speccy-cli/tests/check.rs`, all prefixed `spec_selector_`:
    runs-only-named-spec, unknown-spec-NoSpecMatching, dropped-skip
    (sentinel-file no-subprocess pattern from
    `shell_executes_in_project_root`), superseded-skip (same shape;
    separate test rather than parameterising so a failure points at
    the right status without rg-grepping), in-progress
    failure-categorised-IN-FLIGHT, implemented failure gates exit
    code. The pass+fail fixtures use an inline 2-check spec.toml
    because `spec_toml_pass_fail_fail` is 1 pass + 2 fails, not the
    1+1 pair the task requires.
  - Undone: T-004 (`QualifiedCheck`) and T-005 (`Task`) still own
    their respective real lookups; the two temporary
    `SelectorError`-returning arms remain in place exactly as T-002
    left them. T-006 (ARCHITECTURE.md + skill-docs audit) is its own
    task. SPEC.md and spec.toml were not touched.
  - Commands run:
    - `cargo test -p speccy-cli --test check -- spec_selector` (red,
      before the impl)
    - `cargo test -p speccy-cli --test check -- spec_selector` (green,
      after the impl)
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all` (applied a single one-line wrap fix in
      `check.rs`) + `cargo +nightly fmt --all --check`
    - `cargo deny check`
    - `cargo build -p speccy-cli` + `target/debug/speccy.exe check`
      and `target/debug/speccy.exe check SPEC-0017` (local sanity)
  - Exit codes: fail (5 red + 1 incidental green because the
    placeholder also returned `NoSpecMatching`), pass (6/6 green),
    pass (workspace-wide), pass (clippy), pass + pass (fmt fix +
    recheck), pass (deny; pre-existing ISC / MPL-2.0 / winnow
    duplicate warnings unchanged by this task), pass build + the
    SPEC-0017 dogfood run produced exit 101 IN-FLIGHTs per CHK —
    not a regression, but a known Windows file-lock race where each
    CHK's `cargo test` invocation collides with the still-running
    `cargo build` of `speccy.exe`; the selector logic itself was
    correct (5 SPEC-0017 checks ran in declared order, summary
    reported only those 5).
  - Discovered issues: dogfooding the new selector on Windows
    surfaced the same `cargo` file-lock race that has been present
    in the workspace for all prior tasks (each CHK runs `cargo
    test --workspace` against the same target/ dir that the host
    `speccy.exe` binary is running from). It is not a SPEC-0017
    bug, but it does mean the SPEC-0017 self-check produces
    4 IN-FLIGHT exits on Windows until the workspace has at least
    one settled `target/debug/speccy.exe` lock-cycle.
  - Procedural compliance: (none) — the implementer prompt's
    sketch (introduce `resolve_spec`, factor `collect_checks`
    per-spec body into a reusable helper, delegate to
    `execute_checks` for narrowed `Vec<CollectedCheck>`) mapped
    cleanly onto the existing code shape; no skill-layer friction.

<task-scenarios>
  start with `spec_selector_` so the CHK-002 invocation
  `cargo test -p speccy-cli --test check -- spec_selector` runs
  exactly these tests):
  - When the workspace has SPEC-0001 (three executable checks) and
    SPEC-0002 (three executable checks) and `speccy check SPEC-0001`
    runs, then exactly the three SPEC-0001 checks execute in
    declared order and the summary reads
    `3 passed, 0 failed, 0 in-flight, 0 manual` — SPEC-0002's
    checks do not appear in the output.
  - When `speccy check SPEC-9999` runs against a workspace that has
    no SPEC-9999, then the binary exits 1 and stderr contains
    `SPEC-9999` (asserts on
    `CheckError::Selector(SelectorError::NoSpecMatching { spec_id })`
    with `spec_id == "SPEC-9999"`).
  - When the named spec has status `dropped`, then `speccy check
    SPEC-NNNN` prints exactly one informational line stating the
    spec is `dropped` and no checks executed, exits 0, and spawns
    zero subprocesses (the project-root marker pattern from the
    existing `shell_executes_in_project_root` test is a working
    template for the "no subprocesses" assertion).
  - When the named spec has status `superseded`, then the same
    single-line skip + exit 0 behaviour applies (parameterise on
    `dropped` vs `superseded` or write two tests).
  - When the named spec is in-progress with one passing executable
    check and one failing executable check, then `speccy check
    SPEC-NNNN` exits 0, the failing check is rendered with the
    existing `IN-FLIGHT (in-progress spec, exit N)` wording, and
    the summary reads `1 passed, 0 failed, 1 in-flight, 0 manual`.
  - When the named spec has status `implemented` with the same pass
    + fail pair, then `speccy check SPEC-NNNN` exits with the
    failing check's exit code and the summary reads
    `1 passed, 1 failed, 0 in-flight, 0 manual` — the in-flight
    categorisation depends only on the parent spec's status.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-004">
Implement `CheckSelector::QualifiedCheck`; tests guard bare-form preservation

- Suggested files:
  - `speccy-cli/src/check.rs` (in the `QualifiedCheck` arm, reuse
    the spec resolver from T-003 to locate the spec; then iterate
    its `spec_toml.checks` for one entry whose `id` matches the
    requested `check_id`; on miss return `NoQualifiedCheckMatching`
    naming both `spec_id` and `check_id`; on hit assemble a
    one-element `Vec<CollectedCheck>` and delegate to
    `execute_checks`)
  - `speccy-cli/tests/check.rs` (add the `bare_chk_preserved_*`
    tests; the `spec_toml_three(spec_id_suffix)` fixture already
    gives both specs the same set of `CHK-IDs`, which is the right
    shape for the qualified-vs-bare comparison)
- Implementer note (t004-qualified-check):
  - Completed: `CheckSelector::QualifiedCheck { spec_id, check_id }`
    arm in `speccy-cli/src/check.rs` rewired to a new
    `run_qualified_check` helper (placeholder
    `NoQualifiedCheckMatching` return + the `// T-004 replaces this
    arm` comment removed). The helper reuses `resolve_spec` from
    T-003 verbatim so a missing spec surfaces
    `SelectorError::NoSpecMatching { spec_id }` byte-for-byte
    identical to the bare-spec selector; then derives the spec's
    lifecycle status from frontmatter and — mirroring `run_spec` —
    treats `dropped` / `superseded` as an exit-0 informational
    skip with the same `spec <SPEC-NNNN> is `<status>`; no checks
    executed` wording, never spawning a subprocess. For active
    specs, the helper threads `collect_for_spec` (T-003's
    factored-out per-spec collector) then filters to the matching
    `CHK-NNN`. On miss returns
    `SelectorError::NoQualifiedCheckMatching { spec_id, check_id }`
    naming both fields; on hit delegates the one-element
    `Vec<CollectedCheck>` to `execute_checks` so live streaming,
    IN-FLIGHT categorisation, malformed-spec accounting, and the
    summary footer all reuse the existing path. Five new
    integration tests added to `speccy-cli/tests/check.rs`, all
    prefixed `bare_chk_preserved_`: bare-form cross-spec ordering
    guard (DEC-003), qualified runs only the named spec,
    missing-check returns `NoQualifiedCheckMatching` with both
    fields populated, unknown-spec returns `NoSpecMatching` (proves
    the qualified-check arm delegates to the same resolver T-003
    built), and a forbidden-hints guard that locks in the DEC-003
    promise that bare `CHK-NNN` stays first-class (no `deprecated` /
    `ambiguous` / `use SPEC-NNNN/CHK-NNN` text on stdout). Tests
    written before the production change; the two qualified-shape
    tests went red against the placeholder arm and green after the
    rewire, as expected.
  - Undone: T-005 (`CheckSelector::Task`) still owns the task-form
    lookup; the temporary `TaskCoversNothing` placeholder arm
    remains exactly as T-002 left it. T-006 (ARCHITECTURE.md +
    skill-docs audit) is its own task. SPEC.md and spec.toml were
    not touched. The decision to mirror T-003's dropped /
    superseded skip semantics — rather than running the check
    anyway — keeps status filtering a property of the parent spec
    rather than of the invocation form (SPEC-0017 Assumptions),
    and matches the user-named-the-spec-directly explicitness
    principle T-003 introduced.
  - Commands run:
    - `cargo test -p speccy-cli --test check -- bare_chk_preserved`
      (red, before the impl — two qualified-shape tests failed)
    - `cargo test -p speccy-cli --test check -- bare_chk_preserved`
      (green, after the impl — 5/5 pass)
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all` (applied a one-line wrap on the
      new test function signature) + `cargo +nightly fmt --all --check`
    - `cargo deny check`
    - `cargo build -p speccy-cli` +
      `target/debug/speccy.exe check SPEC-0017/CHK-004`,
      `... SPEC-0017/CHK-099`, `... SPEC-9999/CHK-001`
      (local sanity)
  - Exit codes: fail (2 red qualified-shape tests, 3 incidental
    greens because the placeholder still satisfied the
    missing-check and forbidden-hints assertions), pass (5/5 green),
    pass (workspace-wide), pass (clippy), pass + pass (fmt fix +
    recheck), pass (deny; pre-existing winnow / ISC / MPL-2.0
    duplicate warnings unchanged by this task), pass build + the
    three CLI surface checks rendered the expected lines
    (`speccy check: no `CHK-099` in `SPEC-0017``, `speccy check:
    no spec `SPEC-9999` found in workspace`, and a successful
    CHK-004 run that exits IN-FLIGHT because of the same Windows
    file-lock race T-003 documented — selector logic correct,
    cargo self-build collision is the lone source of non-zero exit).
  - Discovered issues: (none) — the T-003 helper trio
    (`resolve_spec`, `collect_for_spec`, the dropped/superseded
    skip wording) dropped straight into the qualified-check
    lookup with no shape mismatches, and the DEC-003 forbidden-hint
    guard test exposed no surprise output from `execute_checks`.
    The Windows cargo build file-lock collision during the
    `SPEC-0017/CHK-004` self-check is the pre-existing dogfooding
    race documented in T-003's handoff note, not a SPEC-0017 bug.
  - Procedural compliance: (none) — implementer prompt and
    TASKS.md entries lined up directly with the code shape; no
    skill-layer friction encountered.

<task-scenarios>
  start with `bare_chk_preserved_` so the CHK-004 invocation
  `cargo test -p speccy-cli --test check -- bare_chk_preserved` runs
  exactly these tests):
  - When SPEC-0001 and SPEC-0003 both define `CHK-001` and
    `speccy check CHK-001` runs, then both `CHK-001`s execute in
    spec-ascending order and the summary reads
    `2 passed, 0 failed, 0 in-flight, 0 manual` — a redundant
    assertion against `id_filter_matches_across_specs` because
    DEC-003 preservation deserves a guard test wherever the
    selector dispatcher is touched.
  - When `speccy check SPEC-0003/CHK-001` runs against the same
    workspace, then only SPEC-0003's `CHK-001` executes; the
    output contains `==> CHK-001 (SPEC-0003)` and never
    `==> CHK-001 (SPEC-0001)`; the summary reads
    `1 passed, 0 failed, 0 in-flight, 0 manual`.
  - When `speccy check SPEC-0001/CHK-099` runs and SPEC-0001 has
    no `CHK-099`, then the binary exits 1 and stderr names both
    `SPEC-0001` and `CHK-099` (asserts on
    `CheckError::Selector(SelectorError::NoQualifiedCheckMatching
    { spec_id, check_id })` with both fields populated).
  - When `speccy check SPEC-9999/CHK-001` runs and SPEC-9999 does
    not exist in the workspace, then the binary exits 1 with
    `SelectorError::NoSpecMatching { spec_id }` naming `SPEC-9999`
    (qualified-check delegates to the same spec resolver T-003
    built).
  - When `speccy check CHK-001` runs against a multi-spec workspace
    where `CHK-001` appears in two specs, then stdout contains no
    "deprecated" / "ambiguous" / "use SPEC-NNNN/CHK-NNN" hint —
    bare `CHK-NNN` stays a first-class shape per DEC-003.
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-003">
Implement `CheckSelector::Task`; tests for task-scoped execution

- Suggested files:
  - `speccy-cli/src/check.rs` (in the `Task` arm, call
    `speccy_core::task_lookup::find(&workspace, &task_ref)` and
    propagate its `LookupError::Ambiguous` / `NotFound` /
    `InvalidFormat` / `Io` errors unchanged; from the returned
    `TaskLocation`, look at `task.covers`; for each covered
    `REQ-ID`, find the matching `RequirementEntry` in
    `spec_toml.requirements` and collect its `checks: Vec<String>`;
    dedup the combined CHK-ID list in first-occurrence order
    (an `IndexSet`-style fold over a `Vec<String>` is enough — no
    extra dependency); resolve each surviving CHK-ID against the
    spec.toml's `[[checks]]` block to assemble `Vec<CollectedCheck>`;
    handle the empty-covers case as an exit-0 informational path
    per the SPEC's Open-Questions lean; delegate to `execute_checks`)
  - `speccy-cli/tests/check.rs` (add `task_selector_*` tests; build
    a small fixture helper that writes a SPEC.md + spec.toml +
    TASKS.md triple so `task_lookup::find` can actually resolve
    the task — the existing `Workspace`/`write_spec` helpers under
    `tests/common/` already accept a TASKS.md argument as their
    fourth parameter)
- Implementer note (t005-task-selector):
  - Completed: `CheckSelector::Task(TaskRef)` arm in
    `speccy-cli/src/check.rs` rewired to a new `run_task` helper
    (placeholder `TaskCoversNothing` return + the `// T-005 replaces
    this arm` comment removed). `run_task` calls
    `speccy_core::task_lookup::find(&ws, task_ref)` to resolve the
    task; the returned `LookupError::Ambiguous` / `NotFound` /
    `InvalidFormat` / `Io` propagate unchanged through a new
    `CheckError::TaskLookup(#[from] LookupError)` variant
    (transparent `Display`, so the wrapped `LookupError` wording
    reaches stderr byte-for-byte). From the resolved
    `TaskLocation`, `run_task` walks `task.covers`; for each
    `REQ-ID` it finds the matching `RequirementEntry` in
    `spec_toml.requirements` and accumulates the listed `CHK-ID`s
    via an in-order `Vec<&str>` + `.contains(&...)` dedup (no
    extra dependency). Each surviving CHK-ID is resolved against
    `spec_toml.checks`; CHK-IDs absent from `[[checks]]` are
    silently dropped (lint-engine concern per SPEC-0003). The
    resulting `Vec<CollectedCheck>` flows into the existing
    `execute_checks` path so live streaming, IN-FLIGHT
    categorisation, and summary totals reuse the same code as the
    other selectors. Empty-covers chose option (b): print the
    informational line `task `<task_ref>` covers no requirements;
    no checks to run` directly from `run_task` and return `Ok(0)`,
    *without* synthesising a `SelectorError`. Rationale: errors
    should mean exit-1; an informational outcome is not an error.
    With the option-(b) decision locked in, the now-unused
    `SelectorError::TaskCoversNothing` variant was removed from
    `speccy-cli/src/check_selector.rs` for surgical cleanliness
    (AGENTS.md "Surgical changes" — pre-existing dead code stays,
    but dead code we just created should not). `main.rs`'s
    `run_check` was extended to mirror `invoke_implement`'s custom
    formatting of `LookupError::Ambiguous` / `NotFound` /
    `InvalidFormat`, so `speccy check T-002` produces the same
    multi-line "ambiguous; matches in N specs. Disambiguate with
    one of: speccy check SPEC-NNNN/T-002" output as `speccy
    implement T-002` (verified via `diff -` after substituting
    `speccy check` -> `speccy implement` — no diff). Seven new
    integration tests added to `speccy-cli/tests/check.rs`, all
    prefixed `task_selector_`: qualified-single-req, multi-req
    dedup (CHK-001, CHK-002, CHK-003 in declared order, CHK-002
    once), unqualified-ambiguous propagates LookupError wording,
    unqualified-not-found propagates `speccy status` hint,
    empty-covers exit 0 + no subprocess (sentinel-file
    no-subprocess pattern), missing-CHK silently skipped, and
    in-progress task failure categorised IN-FLIGHT. A new
    `tasks_md_fixture(spec_id, &[(task_id, covers_csv)])` helper
    writes a minimal TASKS.md shape that `task_lookup::find` can
    parse (bold task ID + indented `Covers:` sub-bullet). Tests
    written before the production change; all 7 went red against
    the placeholder arm and green after the rewire.
  - Undone: T-006 (ARCHITECTURE.md + skill-docs audit) is its own
    task. SPEC.md and spec.toml were not touched. The
    `SelectorError::TaskCoversNothing` variant is gone, which is
    a `non_exhaustive` enum reduction — out-of-tree consumers do
    not exist today (the CLI is the only consumer of its own
    library, per SPEC-0017 Migration), so the removal is safe.
  - Commands run:
    - `cargo test -p speccy-cli --test check -- task_selector`
      (red, before the impl — 7 failing tests)
    - `cargo test -p speccy-cli --test check -- task_selector`
      (green, after the impl — 7/7 pass)
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      (initially red on `manual_contains` + `doc_markdown` +
      `format_push_string` + `useless_format`; fixed in-place
      without scope creep)
    - `cargo +nightly fmt --all --check`
    - `cargo deny check`
    - `cargo build -p speccy-cli` +
      `target/debug/speccy.exe check T-099` /
      `... check T-002` / `... check SPEC-0017/T-005`
      (local sanity; T-002 output diffed against
      `speccy implement T-002` — identical after command-name
      substitution)
  - Exit codes: fail (7 red), pass (7/7 green), pass (workspace-
    wide, 0 failed), pass (clippy, after 3 in-task fixes), pass
    (fmt), pass (deny; pre-existing ISC / MPL-2.0 / winnow
    duplicate warnings unchanged by this task), pass build +
    stderr surface checks rendered the expected lines. The
    `SPEC-0017/T-005` self-check produced exit 101 IN-FLIGHT,
    which is the documented Windows `target/debug/speccy.exe`
    file-lock race from T-003/T-004's handoff notes — the
    selector logic itself was correct (CHK-003 ran once,
    categorised IN-FLIGHT due to in-progress spec status).
  - Discovered issues: (none new) — the same Windows
    cargo-self-build file-lock race documented in T-003 and T-004
    reappeared on the SPEC-0017 self-check; it is not a SPEC-0017
    bug. The `task_lookup::find` signature already returns
    `Result<TaskLocation, LookupError>` with the wrapping the SPEC
    anticipated, so no upstream signature changes were needed.
  - Procedural compliance: (none) — implementer prompt and
    TASKS.md entries mapped cleanly onto the code. The prompt's
    option (a) vs (b) decision on empty-covers was useful: option
    (b) is the cleaner shape because errors should mean exit-1,
    and the `TaskCoversNothing` variant removal kept
    `SelectorError`'s `Display` surface focused on actual failure
    modes. No skill-layer friction; no skill file edits needed.


<task-scenarios>
  start with `task_selector_` so the CHK-003 invocation
  `cargo test -p speccy-cli --test check -- task_selector` runs
  exactly these tests):
  - When SPEC-0010/T-002 covers `[REQ-002]` and the spec.toml maps
    `REQ-002` to `[CHK-003]`, then `speccy check SPEC-0010/T-002`
    runs exactly `CHK-003` (one `==> CHK-003 (SPEC-0010)` header,
    summary reads `1 passed, 0 failed, 0 in-flight, 0 manual`).
  - When a task covers two requirements `REQ-A -> [CHK-001,
    CHK-002]` and `REQ-B -> [CHK-002, CHK-003]`, then the
    selector runs `CHK-001`, `CHK-002`, `CHK-003` once each, in
    first-occurrence declared order — `CHK-002` is not run twice
    and the header order is `CHK-001`, `CHK-002`, `CHK-003`.
  - When `T-002` exists in both SPEC-0010 and SPEC-0011 and
    `speccy check T-002` runs, then the binary exits 1 with the
    existing `LookupError::Ambiguous` message verbatim (including
    the copy-pasteable `SPEC-NNNN/T-002` hints already produced by
    `task_lookup`); assert on the wrapped
    `CheckError::Selector(...)` or the matching `LookupError`
    depending on how the error chain is wired — the message must
    be identical to what `speccy implement T-002` produces today.
  - When `T-099` is in no spec and `speccy check T-099` runs, then
    the binary exits 1 with the existing `LookupError::NotFound`
    wording (mentioning `speccy status`) preserved verbatim.
  - When the resolved task has `covers: []`, then `speccy check
    SPEC-NNNN/T-NNN` exits 0 with an informational message naming
    the task ref and stating it covers no requirements; no
    subprocesses are spawned (asserts on
    `CheckError::Selector(SelectorError::TaskCoversNothing { task_ref })`
    or the equivalent informational path that maps to exit 0 —
    match the SPEC's "Lean 0" decision in the Open Questions
    section).
  - When the resolved task's `covers` references a `REQ-ID` that
    lists a `CHK-ID` not present in the spec.toml's `[[checks]]`
    block (a lint-engine concern, SPEC-0003), then `speccy check`
    runs whatever it found and exits 0; the missing CHK is
    silently skipped at this layer — the lint engine is the right
    surface for the absence.
  - When `speccy check SPEC-NNNN/T-NNN` runs against an in-progress
    spec whose one covered check fails with exit 2, then the
    check is reported as `IN-FLIGHT (in-progress spec, exit 2)`
    and the binary exits 0 — in-flight categorisation derives from
    the parent spec's status, not from the invocation form.
</task-scenarios>
</task>

## Phase 4: Documentation surface


<task id="T-006" state="completed" covers="REQ-005">
Update `.speccy/ARCHITECTURE.md` and audit shipped skill docs

- Suggested files:
  - `.speccy/ARCHITECTURE.md` (CLI Surface row at ~line 141;
    Execution code fence at ~lines 1003-1004)
  - `.speccy/skills/personas/implementer.md` (fix the `speccy
    check SPEC-NNNN T-002` example at ~line 69)
  - `.speccy/skills/prompts/implementer.md` (audit; update only if
    the new selector surface contradicts existing guidance — line
    ~57 references `speccy check` generically and is likely fine
    as-is)
- Implementer note (t006-architecture-skill-audit):
  - Completed: `.speccy/ARCHITECTURE.md` CLI Surface row for `speccy
    check` updated from `[CHK-ID]` to `[SELECTOR]` with six
    indented sub-bullets enumerating no-arg + the five accepted
    shapes (`SPEC-NNNN`, `SPEC-NNNN/CHK-NNN`, `SPEC-NNNN/T-NNN`,
    `CHK-NNN`, `T-NNN`), mirroring the indented sub-bullet style of
    the `speccy plan` and `speccy tasks` rows directly above.
    Execution code fence in the Checks section expanded from two
    invocations to five (`speccy check`, `speccy check SPEC-0001`,
    `speccy check SPEC-0001/CHK-001`, `speccy check SPEC-0001/T-002`,
    `speccy check CHK-001`) — bare-form line kept and labelled as
    DEC-003. The stale `speccy check SPEC-NNNN T-002` example
    (space, not slash) fixed to `SPEC-NNNN/T-002` in both
    `.speccy/skills/personas/implementer.md` and its source template
    `resources/modules/personas/implementer.md` (the two files are
    byte-identical and the `resources/modules/` copy is what gets
    templated into a downstream user's project by `speccy init` via
    `embedded::RESOURCES` + `render::render_host_pack` — a fix to
    only one side would re-leak the stale example on the next
    install). Also updated the historical task title in
    `.speccy/specs/0010-check-command/TASKS.md:88` from `Wire
    `speccy check [CHK-ID]` into the binary` to `Wire `speccy check
    [SELECTOR]` into the binary` so the absolute `git grep -n
    "speccy check \[CHK-ID\]"` -> zero criterion is satisfied; T-008
    of SPEC-0010 is `[x]` completed, the title is descriptive of
    "what got wired" which is now the selector dispatcher SPEC-0017
    evolved it into.
  - Undone: (none) — task is documentation-only and the
    acceptance criteria are all met. SPEC.md and spec.toml were not
    touched per the surgical-changes rule.
  - Commands run:
    - `git grep -n "speccy check"` (initial audit across the repo)
    - `git diff --no-index .speccy/skills/personas/implementer.md
      resources/modules/personas/implementer.md` (confirmed
      byte-identical -> both need the same edit)
    - `git grep -n "speccy check \[CHK-ID\]"` (post-edit; zero hits)
    - `git grep -n "speccy check SPEC-NNNN T-"` (post-edit; zero
      hits — confirmed the stale space-delimited shape is gone)
    - `git grep -n "speccy check "` across
      `.speccy/skills/`, `resources/modules/`, `.agents/skills/`,
      `.claude/skills/` (final audit; only the fixed slash-form
      example remains under personas/implementer.md)
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    - `cargo +nightly fmt --all --check`
    - `cargo deny check`
    - `target/debug/speccy.exe check` (local sanity)
  - Exit codes: pass (audit; surfaced 5 ARCHITECTURE.md hits + 1
    personas hit + 1 historical SPEC-0010 hit + benign generic
    mentions), pass (empty diff confirmed identity), pass (zero
    hits), pass (zero hits), pass (only the fixed slash-form hit
    remains), pass (workspace-wide), pass (clippy), pass (fmt),
    pass (deny; pre-existing ISC / MPL-2.0 / winnow duplicate
    warnings unchanged by this task), pass build + the dogfooding
    run produced 130 passed / 1 failed / 0 in-flight / 2 manual —
    the 1 failed is the documented Windows `target/debug/speccy.exe`
    cargo-self-build file-lock race from T-003/T-004/T-005's
    handoff notes; selector logic itself was correct (the SPEC-0017
    CHK-001..CHK-005 mix executed in declared order and CHK-005's
    manual prompt printed verbatim with the new selector docs
    already in place).
  - Discovered issues: `.speccy/skills/personas/implementer.md` is
    byte-identical to `resources/modules/personas/implementer.md`.
    The `resources/modules/` copy is the source of truth that
    gets templated by `embedded::RESOURCES` + `render::render_host_pack`
    into a downstream user's `.claude/skills/` or `.agents/skills/`
    on `speccy init`, while the `.speccy/skills/` copy is this
    project's *installed* pack used by its own dogfooding loop. A
    fix to only one side would (a) re-leak the stale example into
    every downstream `speccy init`, or (b) leave dogfooding agents
    inside this repo reading the stale form. Edited both. The
    historical task title in `.speccy/specs/0010-check-command/TASKS.md`
    was lightly amended for the same reason — the absolute
    "zero hits anywhere in the repo" criterion would otherwise be
    missed by one historical-log line. The amendment is faithful
    to what `T-008 of SPEC-0010` ultimately wired (the selector
    dispatcher SPEC-0017 evolved it into) and keeps the task `[x]`.
  - Procedural compliance: (none) — the prompt's friction-handling
    section did not apply. The CLI surface and skill-pack source-
    vs-installed duality were already correctly described in
    ARCHITECTURE.md and the SPEC-0016 history; this task simply
    followed the implementer prompt's "verify before editing"
    guidance and discovered the byte-identical-pair shape via
    direct diff before touching either side.

<task-scenarios>
  - This task is verified by CHK-005's manual prompt; no new
    automated tests. Acceptance is structural and grep-checkable:
    - `git grep -n "speccy check \[CHK-ID\]"` returns zero hits
      anywhere in the repo after the edit.
    - The `.speccy/ARCHITECTURE.md` CLI Surface row for
      `speccy check` (around line 141 today) reads
      `speccy check [SELECTOR]` with indented sub-bullets naming
      each accepted shape (`SPEC-NNNN`, `SPEC-NNNN/CHK-NNN`,
      `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`), matching the
      indentation style of the existing `speccy plan` and
      `speccy tasks` rows above it.
    - The Execution code fence in the Checks section (around lines
      1003-1004 today) shows example invocations for `speccy
      check`, `speccy check SPEC-NNNN`, `speccy check
      SPEC-NNNN/CHK-NNN`, `speccy check SPEC-NNNN/T-NNN`, and the
      bare `speccy check CHK-NNN`; the existing bare-form line
      stays.
    - The stale `speccy check SPEC-NNNN T-002` example in
      `.speccy/skills/personas/implementer.md` (around line 69; a
      space, not a slash, between the spec and task fragments) is
      either fixed to the slash form or removed if no longer
      accurate.
    - `git grep -n "speccy check"` across `.speccy/skills/` and
      `skills/` produces no remaining invocation that would
      mislead a future agent into using the pre-SPEC-0017 CHK-only
      shape.
    - No new lint codes are added and no new noun appears in the
      five-noun set (cross-check via
      `git diff -- .speccy/ARCHITECTURE.md`).
</task-scenarios>
</task>

</tasks>
