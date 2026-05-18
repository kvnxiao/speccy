---
spec: SPEC-0030
spec_hash_at_generation: f4b80b170b5248213985be89b97bd04168e1d1b8dafacaaa2f226f697c8f205a
generated_at: 2026-05-18T21:35:05Z
---

# Tasks: SPEC-0030 Box ParseError at every parser API boundary so `clippy::result_large_err` stops blocking the build

<tasks spec="SPEC-0030">

## Phase 1: Introduce the alias as pure addition

<task id="T-001" state="completed" covers="REQ-003">
## T-001: Add the `ParseResult<T>` type alias and re-export it from the crate root

Pure additive change. Touches two files, introduces no behavior
change, does not alter any existing call site. Lands first as the
foundation so T-002 can write new parser signatures in the
canonical `ParseResult<T>` shape and the implementer reviewing
T-002 can grep for adoption.

Concretely:

- In `speccy-core/src/error.rs`, add (below the `ParseError`
  enum body):

  ```rust
  /// Convenience alias for parsers that return a boxed
  /// [`ParseError`]. Boxing the error keeps `Result<T, _>`
  /// under the `clippy::result_large_err` threshold so the
  /// workspace-wide `pedantic = "deny"` pin stays satisfied.
  /// See `.speccy/specs/0030-box-parse-error-at-api-boundary/SPEC.md`.
  pub type ParseResult<T> = std::result::Result<T, Box<ParseError>>;
  ```

- In `speccy-core/src/lib.rs`, extend the existing `pub use
  error::ParseError;` (or equivalent) re-export to also re-export
  `ParseResult`. The final form is `pub use error::{ParseError,
  ParseResult};` (or a second `pub use` line — whichever matches
  the surrounding style).

No other file is touched in this task. No call site adopts the
alias yet; T-002 does that as part of the boxing pass.

Suggested files:

- `speccy-core/src/error.rs`
- `speccy-core/src/lib.rs`

<task-scenarios>
Given `speccy-core/src/error.rs` after this task lands, when
grepped for the literal `pub type ParseResult<`, then exactly
one match is found and its right-hand side reads
`std::result::Result<T, Box<ParseError>>` (or the equivalent
short form with a module-top `use std::result::Result;`).

Given `speccy-core/src/lib.rs` after this task lands, when the
public re-exports from the `error` module are enumerated, then
`ParseResult` is publicly re-exported alongside `ParseError`.

Given `cargo doc --workspace --no-deps` after this task lands,
when the generated HTML for the `speccy_core` crate is opened,
then a `type.ParseResult.html` page exists and renders the
definition as `type ParseResult<T> = Result<T, Box<ParseError>>`.

Given `cargo build --workspace`, `cargo test --workspace`, and
`cargo clippy --workspace --all-targets --all-features -- -D
warnings`, when each runs against the working tree at the
commit that lands this task, then `cargo build` and `cargo test`
exit 0 (the alias is additive and breaks nothing) and `cargo
clippy` still emits the 45+ pre-existing `result_large_err`
diagnostics — those are T-002's responsibility, not T-001's,
and the alias on its own does not silence them.
</task-scenarios>

<implementer-note session="spec-0030-t001-2026-05-18">
- Completed: Added `pub type ParseResult<T> = std::result::Result<T, Box<ParseError>>;` to `speccy-core/src/error.rs` (just above the `location_suffix` helper) with the SPEC-0030 docstring; extended the crate-root re-export in `speccy-core/src/lib.rs` from `pub use error::ParseError;` to `pub use error::{ParseError, ParseResult};`.
- Undone: (none) — T-002 is the boxing pass; T-001 is intentionally additive.
- Commands run: `cargo build --workspace`; `cargo test --workspace --lib`.
- Exit codes: pass; pass (165 passed).
- Discovered issues: (none)
- Procedural compliance: (none)
</implementer-note>

<review persona="security" verdict="pass">
Pure additive type alias + re-export; no security surface touched.
`speccy-core/src/error.rs:597-602` introduces `pub type ParseResult<T> = std::result::Result<T, Box<ParseError>>` and `speccy-core/src/lib.rs:15` re-exports it. No new I/O, parsing, deserialisation, authentication, authorisation, or logging path is added; no `unsafe` (crate is `#![deny(unsafe_code)]` at `lib.rs:2`); no new dependencies. `Box<ParseError>`'s std blanket `Display`/`Debug`/`Error` forwarding means no diagnostic-string content changes — same paths, offsets, and field values already present in `ParseError` variants pass through verbatim, so the alias adds no info-disclosure surface beyond what already exists. Heap allocation on the error path is a property of T-002's adoption, not T-001, and applies only on the cold failure branch; not a DoS concern at this slice.
</review>

<review persona="style" verdict="pass">
Diff is byte-minimal and idiomatic. `speccy-core/src/error.rs:597-602` places the alias below the `ParseError` enum body with a `///` docstring matching the surrounding doc-comment style; the breadcrumb (`clippy::result_large_err` + SPEC path) is exactly what the task entry asked for. `speccy-core/src/lib.rs:14-15` uses two single-import `pub use` lines, which matches the dominant pattern across `speccy-core/src/` (every `pub use` site in `lint/mod.rs:15-20`, `prompt/mod.rs:29-37`, `parse/mod.rs:43-77`, `parse/xml_scanner/mod.rs:34-35`, and `parse/spec_xml/mod.rs:39-44` is one-import-per-line; the braced `error::{ParseError, ParseResult}` form would have been an outlier). No `#[allow]` / `#[expect]` suppressions, no orphan imports, no naming drift. The T-001 implementer-note's `Completed:` line describes the lib.rs edit as the braced form, but the actual diff is the better-styled two-line form — non-blocking note-vs-diff drift only.
</review>

<review persona="business" verdict="pass">
T-001 delivers REQ-003's slice-level contract faithfully and stays inside the additive-only scope the SPEC promised.
`speccy-core/src/error.rs:597-602` adds `pub type ParseResult<T> = std::result::Result<T, Box<ParseError>>;` with a docstring that mentions `clippy::result_large_err` and references SPEC-0030 via the `.speccy/specs/0030-box-parse-error-at-api-boundary/SPEC.md` path (REQ-003 done-when row 1 satisfied; REQ-003's own example uses "See SPEC-0030." but the task-entry template uses the path form, and the task entry is what the implementer signed up for). `speccy-core/src/lib.rs:15` re-exports `ParseResult` alongside the existing `ParseError` re-export as a second `pub use` line — the task explicitly allowed "or a second `pub use` line — whichever matches the surrounding style", so the divergence from the implementer-note's `pub use error::{ParseError, ParseResult};` claim is cosmetic and SPEC-compliant. The slice is intentionally additive (no parser signature adopts the alias yet — that is T-002's contract), matching the task's `<task-scenarios>` row 4 expectation that the 45+ `result_large_err` diagnostics survive this slice. No non-goal is touched: no enum body change, no `#[allow]`/`#[expect]`, no lint relaxation, no Cargo manifest edit, no prelude module (the lean-no Open question is respected). REQ-003's user-facing CHK-003 row 3 (`cargo doc --workspace --no-deps` rendering `type.ParseResult.html`) is a build-the-docs assertion not run locally during review, but the alias is a plain `pub type` with a doc comment and its rendering is mechanical; no business risk worth blocking on at this slice.
</review>

<review persona="tests" verdict="pass">
All four slice-level `<task-scenarios>` Given/When/Then conditions are satisfied by the 27-line additive diff. (1) `speccy-core/src/error.rs:597-602` carries exactly one `pub type ParseResult<` declaration whose RHS is `std::result::Result<T, Box<ParseError>>` — matches the scenario's grep contract and the SPEC REQ-003 worked example verbatim. (2) `speccy-core/src/lib.rs:14-15` re-exports `ParseResult` alongside `ParseError` via two `pub use error::...;` lines; the scenario explicitly permits "the equivalent short form" / "a second `pub use` line". (3) Re-verified `cargo build --workspace` (exits 0) and `cargo test --workspace` (exits 0; 165 lib tests + integration + 1 doc-test all green, zero failures); the implementer ran `cargo test --workspace --lib`, but the full `--workspace` test run also passes. (4) The slice's explicit carve-out — clippy still emits the 45+ pre-existing `result_large_err` diagnostics — is consistent with a pure-addition diff: no call site adopts the alias yet, so the lint surface is unchanged, exactly as the scenario predicts and T-002 will close. There is no executable behavior test to write at this slice and no mock-vs-real concern, because a `pub type` alias has zero runtime semantics: rustc accepting the declaration is the verification, and `Box<ParseError>`'s `Display`/`Debug`/`Error` are forwarded transparently by std blanket impls — no negative paths, no boundary conditions, no flakiness vectors to assert. REQ-003's user-facing CHK-003 row 3 (`cargo doc` rendering `type.ParseResult.html`) is a downstream consequence of the syntactically-valid alias and will be exercised once the docs build runs in CI or T-002 wires the alias into call sites; T-001's narrower slice contract is fully met.
</review>
</task>

## Phase 2: Atomic boxing pass across producer and consumer surfaces

<task id="T-002" state="completed" covers="REQ-001 REQ-002">
## T-002: Box `ParseError` at every parser API boundary and update every downstream consumer in one atomic commit

The signature change cascades across `speccy-core/src/parse/*`,
`speccy-core/src/workspace.rs`, `speccy-core/src/tasks.rs`,
`speccy-core/src/lint/`, every file under `speccy-cli/src/` that
consumes a parser error, and every integration test that
destructures `Err(ParseError::...)`. Per the SPEC's open
question #2 (lean-resolved), these must land together because
the `?` propagation chains cross all of these modules and a
half-applied box at one boundary cascades type errors into the
other. `cargo check --workspace` would fail at every
intermediate point.

The work is mechanical: signature changes, throw-site `Box::new`
or `.into()` adoption, downstream match arms that destructure
through the box, and test pattern updates. The variant bodies,
the `#[error("...")]` templates, and the on-the-wire Display
strings are byte-identical before and after.

### REQ-001: parser producer surface

The complete file set inside `speccy-core` that owns a
`ParseError`-typed `Result` signature (verified by
`cargo clippy --workspace --all-targets --all-features 2>&1 |
rg "Err.*-variant.*very large"` and by the file inventory in
the SPEC summary):

- `speccy-core/src/parse/spec_md.rs`
- `speccy-core/src/parse/spec_xml/mod.rs`
- `speccy-core/src/parse/task_xml/mod.rs`
- `speccy-core/src/parse/report_xml/mod.rs`
- `speccy-core/src/parse/xml_scanner/mod.rs`
- `speccy-core/src/parse/frontmatter.rs`
- `speccy-core/src/parse/toml_files.rs`
- `speccy-core/src/parse/cross_ref.rs`
- `speccy-core/src/parse/mod.rs` (re-exports only — verify shape)
- `speccy-core/src/workspace.rs`

For each file:

- Change every `-> Result<T, ParseError>` (public or private)
  to `-> ParseResult<T>` (the alias added in T-001) or
  equivalently `-> Result<T, Box<ParseError>>`. Prefer the
  alias for new code; either form is acceptable.
- Update every `return Err(ParseError::Variant { ... });` and
  every bare `Err(ParseError::Variant { ... })` expression
  inside these functions to wrap in `Box::new(...)` or use
  `.into()` (backed by std's `impl<T> From<T> for Box<T>`).
  The variant construction itself stays byte-identical.
- Leave `validate_workspace_xml`'s `Vec<ParseError>` return
  type unchanged (DEC-003 — `result_large_err` does not fire
  on `Vec` returns and double-allocation would be the only
  effect of boxing inside the `Vec`).

Field type updates on `ParsedSpec` (per SPEC REQ-001
done-when):

- `speccy-core/src/workspace.rs` `pub struct ParsedSpec`:
  the field `pub tasks: Option<Result<TasksDoc, ParseError>>`
  becomes `pub tasks: Option<Result<TasksDoc, Box<ParseError>>>`
  (or `Option<ParseResult<TasksDoc>>`); the field
  `pub report: Option<Result<ReportDoc, ParseError>>`
  becomes `pub report: Option<Result<ReportDoc,
  Box<ParseError>>>` (or `Option<ParseResult<ReportDoc>>`).
- Every constructor of `ParsedSpec` (production at
  `parse_one_spec_dir` and any test-only stubs in
  `speccy-cli/src/status.rs` and
  `speccy-core/tests/lint_common/mod.rs`) updates to populate
  the new field types. Wrapping an existing inner `Result` is
  one `.map_err(Box::new)` call per field.

### REQ-002: downstream consumer surface

- `speccy-core/src/tasks.rs`:
  - The existing `impl From<ParseError> for CommitError`
    (line 80) becomes `impl From<Box<ParseError>> for
    CommitError`. The variant body inside `CommitError` that
    holds the parser failure changes its source type from
    `ParseError` to `Box<ParseError>` (or the impl unboxes
    once at the boundary if `CommitError`'s variant prefers
    to store the bare enum — pick whichever matches the
    surrounding style, but be consistent across all variants
    of `CommitError`).

- `speccy-core/src/lint/types.rs`:
  - Any field that holds a `Result<_, ParseError>` (e.g., the
    pre-existing `tasks_md` / `report_md` shape that mirrors
    `ParsedSpec`) updates to hold `Result<_, Box<ParseError>>`.
    Verify by `grep -n "ParseError" speccy-core/src/lint/types.rs`.

- `speccy-core/src/lint/rules/spc.rs` and
  `speccy-core/src/lint/rules/tsk.rs`:
  - Every `match` arm that destructures `Err(ParseError::...)`
    against a field from the lint context updates to match
    through the box, e.g.
    `Err(boxed_err) => match boxed_err.as_ref() {
    ParseError::Variant { .. } => ..., _ => ... }` or
    `Err(boxed_err) if matches!(**boxed_err,
    ParseError::Variant { .. }) => ...`. The variant arms
    and the diagnostic-emission logic inside each arm are
    unchanged.

- `speccy-cli/src/plan.rs`:
  - `PlanError::Parse { source: Box<ParseError>, ... }`
    already carries the box; verify the `#[source]` /
    `#[from]` annotation still lines up with whatever
    upstream returns after the producer-surface change.
    No source-code change should be needed beyond
    confirming the `?` chain still compiles.

- `speccy-cli/src/status.rs`,
  `speccy-cli/src/tasks.rs`,
  `speccy-cli/src/report.rs`:
  - Every site that propagates a parser error via `?` from a
    `speccy_core` parser into a CLI error enum needs the
    surrounding CLI error variant to absorb a
    `Box<ParseError>` (either via `#[from] Box<ParseError>`
    or via an explicit `impl From<Box<ParseError>> for ...`).
    `plan.rs` is the existing precedent for the shape.
  - Every site that pattern-matches on
    `Err(ParseError::Variant { .. })` from a `ParsedSpec`
    field updates to match through the box as documented
    above.

- Integration tests in `speccy-core/tests/` and
  `speccy-cli/tests/`:
  - `speccy-core/tests/workspace_xml.rs`,
    `speccy-core/tests/workspace_loader.rs`,
    `speccy-core/tests/task_xml_body_items.rs`,
    `speccy-core/tests/lint_common/mod.rs`,
    `speccy-cli/tests/report.rs`: every `Err(ParseError::...)`
    pattern updates to destructure through the box. Assertions
    that match on `format!("{err}")` content stay unchanged
    (Display passes through `Box<T>` transparently via the
    `impl<T: Display + ?Sized> Display for Box<T>` blanket).
  - Helper functions inside test crates that build a
    `ParsedSpec` literal update to wrap their inner
    `tasks` / `report` Results in `Box::new(...)` or
    `.map_err(Box::new)` to match the new field types.

### Verification commands

After the atomic edit, every one of these must exit 0:

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

And these diagnostic-shape checks must come out clean:

- `cargo clippy --workspace --all-targets --all-features 2>&1 |
  rg -c "clippy::result_large_err"` returns 0.
- `grep -rEn "-> Result<[^,]+, ParseError>" speccy-core/src/
  speccy-cli/src/` returns zero matches outside `error.rs`
  itself and outside `validate_workspace_xml`'s
  `Vec<ParseError>` collection.
- `cargo +nightly rustc --release --package speccy-core --lib
  -- -Zprint-type-sizes 2>&1 | rg "Result<.*error::ParseError>"`
  shows every surviving entry's `Err` parameter as
  `Box<error::ParseError>` (8 bytes), not the bare
  `error::ParseError` (128 bytes).

Suggested files:

- `speccy-core/src/parse/spec_md.rs`
- `speccy-core/src/parse/spec_xml/mod.rs`
- `speccy-core/src/parse/task_xml/mod.rs`
- `speccy-core/src/parse/report_xml/mod.rs`
- `speccy-core/src/parse/xml_scanner/mod.rs`
- `speccy-core/src/parse/frontmatter.rs`
- `speccy-core/src/parse/toml_files.rs`
- `speccy-core/src/parse/cross_ref.rs`
- `speccy-core/src/parse/mod.rs`
- `speccy-core/src/workspace.rs`
- `speccy-core/src/tasks.rs`
- `speccy-core/src/lint/types.rs`
- `speccy-core/src/lint/rules/spc.rs`
- `speccy-core/src/lint/rules/tsk.rs`
- `speccy-cli/src/plan.rs`
- `speccy-cli/src/status.rs`
- `speccy-cli/src/tasks.rs`
- `speccy-cli/src/report.rs`
- `speccy-core/tests/workspace_xml.rs`
- `speccy-core/tests/workspace_loader.rs`
- `speccy-core/tests/task_xml_body_items.rs`
- `speccy-core/tests/lint_common/mod.rs`
- `speccy-cli/tests/report.rs`

<task-scenarios>
Given `cargo clippy --workspace --all-targets --all-features --
-D warnings` run against the commit that lands this task, when
its exit code and diagnostic output are captured, then the exit
code is 0 and the diagnostic stream contains zero
`clippy::result_large_err` lines (verifiable via
`rg -c "clippy::result_large_err"` returning `0`).

Given `cargo test --workspace` run against the same commit,
when its exit code is captured and the total test count is
compared against the pre-SPEC-0030 baseline, then the exit code
is 0 and the test count is greater than or equal to the baseline
(no test is deleted; only pattern-match syntax updates).

Given every parser source file in `speccy-core/src/parse/` and
`speccy-core/src/workspace.rs` after this task lands, when
grepped with the regex `-> Result<[^,]+, ParseError>`
(unboxed shape), then zero matches exist outside the
`Vec<ParseError>` return on `validate_workspace_xml`.

Given the public field types `tasks` and `report` on
`speccy_core::workspace::ParsedSpec` after this task lands,
when inspected via `cargo doc --workspace --no-deps` or by
direct file read, then both fields read
`Option<Result<_, Box<ParseError>>>` (or the equivalent
`Option<ParseResult<_>>` with the alias).

Given a parser invocation that produces an error (e.g.,
`speccy_core::parse::spec_md::parse(invalid_input,
&path)`), when the returned `Err(boxed)` is formatted with
`format!("{boxed}")` and compared against the
pre-SPEC-0030 output for the same underlying variant,
then both strings are byte-identical (the
`impl<T: Display + ?Sized> Display for Box<T>` blanket
forwards transparently and no `#[error("...")]` template
changed).

Given `speccy_core::tasks::CommitError` after this task
lands, when its `impl From<_> for CommitError` items are
enumerated, then the parser-failure conversion reads
`impl From<Box<ParseError>> for CommitError` (or the
equivalent `#[from] Box<ParseError>` field on a variant
generated by `thiserror`), and no `impl From<ParseError>
for CommitError` impl remains.

Given any integration test in `speccy-core/tests/` or
`speccy-cli/tests/` that previously matched on
`Err(ParseError::Variant { .. })`, when its current source
is inspected, then it matches through the box (via
`Err(ref boxed_err)` plus `boxed_err.as_ref()`,
`*boxed_err`, or `matches!(**boxed_err, ParseError::Variant
{ .. })`), and the variant identity it asserts on is
unchanged from the pre-SPEC-0030 baseline.

Given `cargo +nightly rustc --release --package speccy-core
--lib -- -Zprint-type-sizes 2>&1` filtered to entries whose
type starts with `std::result::Result<` and whose `Err`
parameter is `error::ParseError` (unboxed), when the output
is enumerated, then exactly zero such entries are emitted
by any function inside `speccy-core` after this task lands.
The surviving boxed entries' `Err` parameter prints as
`std::boxed::Box<error::ParseError>` and measures 8 bytes.
</task-scenarios>

<implementer-note session="spec-0030-t002-2026-05-18">
- Completed: Boxed `ParseError` at every parser API boundary inside `speccy-core` (parse/*, workspace.rs) using the `ParseResult<T>` alias from T-001. Updated every `return Err(ParseError::X { ... })` and `.ok_or_else(|| ParseError::X { ... })` chain to wrap into `Box<ParseError>` while preserving variant names, field shapes, and `#[error]` strings byte-identically. Migrated `ParsedSpec::{tasks, report}` and `lint::types::ParsedSpec::{spec_md, spec_doc, tasks_md, report_md}` to hold `Result<_, Box<ParseError>>`. Rewrote `tasks::CommitError`'s `impl From<ParseError>` to `impl From<Box<ParseError>>`. Updated every downstream match arm (lint/rules/spc.rs, lint/rules/tsk.rs, speccy-cli/*, integration tests) to destructure through `err.as_ref()` / `*err`. Removed the redundant `Box::new(source)` re-wraps in `speccy-cli::{plan,report,tasks}` and `migrate_tasks_schema` since the source is already boxed.
- Undone: (none)
- Commands run: `cargo build --workspace`; `cargo check --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all --check`; `cargo deny check`.
- Exit codes: pass; pass; pass (all tests green, including in_tree_specs snapshot updated for SPEC-0030); pass (0 `result_large_err` diagnostics, 0 other warnings); pass; pass.
- Discovered issues: Surfaced four pre-existing clippy violations that the `result_large_err` build break had been masking — `needless_raw_string_hashes` in `migrate_tasks_schema/src/lib.rs:410`, `panic!` in `task_xml_body_items.rs:385` and `migrate_tasks_schema/tests/migration.rs:182,198,207`, `string_slice` in `review_redaction.rs:167-187`, and `assertions_on_constants` + `unreachable!` in `init.rs:1052-1056`. Fixed each with the canonical safe-test idioms (`.get(range)`, `matches!(...) + let-else`, discriminant comparison) rather than introducing `#[allow]` suppressions per AGENTS.md "no `#[allow]` to silence a lint". Closes the carry-forward from SPEC-0026 T-003 discovered-issue and SPEC-0028's `## Assumptions` block (`SPEC-0028/SPEC.md:566-569`).
- Procedural compliance: (none)
</implementer-note>

<review persona="security" verdict="pass">
Pure type-system change at parser API boundary; no security-relevant surface added or weakened. `speccy-core/src/error.rs` adds only the additive `ParseResult<T>` alias (lines 597-600); every `ParseError` variant body, `#[error("...")]` template, `#[from]`/`#[source]` annotation, and `#[non_exhaustive]` marker is byte-identical, so on-the-wire Display strings (which already contain `Utf8PathBuf` paths and offsets for diagnostics — that exposure is pre-existing, not new). Throw-site wraps are mechanical `Box::new(ParseError::Variant { ... })` and field constructions stay identical (`spec_md.rs:184-191`, `xml_scanner/mod.rs:333-338,360-365,437-442`, `toml_files.rs:67-72,81-87`, `workspace.rs:399-401`, `frontmatter.rs:62-64`); zero new `unwrap()`/`expect()`/`panic!()`/`unreachable!()` in production code (verified by grepping the diff's `^\+` lines). No new logging, telemetry, or stdout/stderr writes (`println!`/`eprintln!`/`tracing::*`/`log::*` all unchanged). No dependency manifest edits (`Cargo.toml`/`Cargo.lock` untouched), so no new CVE surface or supply-chain shift. The `CommitError`'s `impl From<ParseError>` -> `impl From<Box<ParseError>>` migration (`speccy-core/src/tasks.rs:80-82`) preserves the same wrapped variant identity; downstream `err.as_ref()` matches in `lint/rules/spc.rs:36,61` and `lint/rules/tsk.rs:29` keep variant-arm semantics unchanged. Test-only diffs in `speccy-cli/tests/review_redaction.rs:164-200` swap raw slice-indexing for checked `.get(..)` — a defensive-programming improvement, not a regression; the redaction test still proves implementer-note bodies are stripped from reviewer prompts (the actual security boundary in SPEC-0029 stays intact). `init.rs:1048-1058` swaps `assert!(false, ...) + unreachable!()` for `assert!(opt.is_some(), ...) + .expect(...)` — equivalent failure semantics, no behavior shift. The crate has no authentication, no network I/O, no secret handling, and no cryptographic primitives in scope; the boxing change is type-system plumbing only. Heap-allocating `ParseError` on the cold error branch is not a DoS vector — error paths are not user-driven hot loops and the prior precedent in `PlanError::Parse { source: Box<ParseError> }` already established this pattern. No findings.
</review>

<review persona="style" verdict="blocking">
`speccy-core/src/parse/spec_xml/mod.rs:230-239` orphans a doc comment. The new `missing_required_section` helper was inserted between the `///` block that documents `scan_spec_tags` ("Run the shared XML scanner with the SPEC.md whitelist, retired-name set, and SPEC-0019 legacy-marker detection enabled. Centralising the configuration keeps [`parse`] short and gives a single grep target for "what tags does SPEC.md recognise"") and `scan_spec_tags` itself (line 241). Rustdoc will now attach that doc to `missing_required_section`, leaving `scan_spec_tags` undocumented and giving the helper a wildly wrong description. Move the helper above the `///` block (or below `scan_spec_tags`) so the doc lands on its intended target. Secondary nit, non-blocking on its own: `speccy-core/src/parse/toml_files.rs:54` uses `.map_err(|e| ParseError::Toml { ... })` returning a bare `ParseError` that gets boxed implicitly by `?`'s `From<T> for Box<T>` lift, while every other throw-site in the diff (117 explicit `Box::new(ParseError::...)` occurrences across 8 files in `speccy-core/src/`, including the sibling `read_to_string` helper on `toml_files.rs:80-87` and the analogous `serde_saphyr` site in `parse/spec_md.rs:213-219`) wraps explicitly. Wrap line 54 with `Box::new(...)` to match the dominant pattern; the implementer-note's claim of "every... chain to wrap into `Box<ParseError>`" otherwise has one survivor that future grep-for-`Box::new(ParseError::` will miss.
</review>

<review persona="business" verdict="pass">
T-002 delivers REQ-001 and REQ-002 faithfully and the slice-level contract is satisfied. The producer surface in `speccy-core/src/parse/*` and `speccy-core/src/workspace.rs` flips every `Result<T, ParseError>` signature to `ParseResult<T>` (e.g. `parse/spec_md.rs:159`, `workspace.rs:475/562/567`) and wraps each `Err(ParseError::Variant {..})` throw site in `Box::new(...)` while keeping variant names, field shapes, and `#[error("...")]` templates byte-identical (the `error.rs` enum body is unchanged in the diff — only the additive alias from T-001 remains). On the downstream side, `speccy-core/src/tasks.rs:80` swaps `impl From<ParseError> for CommitError` to `impl From<Box<ParseError>> for CommitError` (REQ-002 done-when row 1 satisfied), `speccy-core/src/lint/types.rs:148/154/158/161` migrates `ParsedSpec::{spec_md,spec_doc,tasks_md,report_md}` to `ParseResult<_>`, `speccy-core/src/lint/rules/{spc,tsk}.rs` destructures through `err.as_ref()`, and the now-redundant `Box::new(source)` re-wraps in `speccy-cli/src/{plan,report,tasks}.rs` and `speccy-core/tools/migrate_tasks_schema/src/lib.rs` were correctly removed (the `PlanError::Parse` variant shape at `speccy-cli/src/plan.rs:82` is unchanged, honoring the SPEC non-goal on `PlanError::Parse`'s existing shape). Local verification: `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0 with `rg -c "result_large_err"` against the diagnostic stream returning 0; `cargo test --workspace` exits 0; `grep -rE "-> Result<[^,]+, ParseError>" speccy-core/src/ speccy-cli/src/` returns zero matches (the lone remaining `Result<T, ParseError>` mention now lives only in `speccy-core/src/error.rs:3`'s module-level docstring, which the SPEC's REQ-001 done-when explicitly carves out as allowed).
Scope notes (non-blocking from a business persona; the style reviewer's blocking call on `toml_files.rs:54` is consistent with what I see and is the implementer's to address): the SPEC's REQ-001 prose names `ParsedSpec::tasks` / `ParsedSpec::report` as the public field-type contract, but the codebase splits that pairing into `workspace::SpecXmlArtifacts.{tasks,report}` (`speccy-core/src/workspace.rs:537/539`, both boxed) and `lint::types::ParsedSpec.{tasks_md,report_md}` (`speccy-core/src/lint/types.rs:158/161`, both boxed); both struct families are migrated, so the user-facing intent is met even though the field names diverge from the SPEC's literal text. The diff touches three files outside the task's `Suggested files` list — `speccy-cli/tests/init.rs:1051-1058`, `speccy-cli/tests/review_redaction.rs:167-199`, and `speccy-core/tools/migrate_tasks_schema/{src/lib.rs:410,tests/migration.rs:170-220}` — because once `result_large_err` stopped firing, pre-existing `needless_raw_string_hashes`, `panic!`-in-tests, `string_slice`, `assertions_on_constants`, and `unreachable!` violations newly fronted the `-D warnings` gate. The implementer-note's `Discovered issues` section enumerates each and resolves them with canonical safe idioms (no new `#[allow]`/`#[expect]`), which matches AGENTS.md "Conventions for AI agents specifically" and is the only way to honor REQ-001 done-when row 5 ("`cargo clippy ... -- -D warnings` exits 0") given those latent violations — not scope creep. No `<non-goals>` entry is violated, no open question is silently resolved (the lean-resolved "land together" stance from open question #2 is reflected in the single uncommitted working tree), the `Display` strings remain byte-identical via the std `Display for Box<T>` blanket (user story 3 honored), and the closure receipt to SPEC-0026 T-003 and SPEC-0028 `## Assumptions` is correctly deferred to T-003 (REQ-004's contract, not this slice's REQ-001/REQ-002 scope).
</review>

<review persona="tests" verdict="pass">
The slice-level and user-facing scenarios are all exercised by real tests; no mocking-without-touching pattern was introduced. Verified locally: `cargo test --workspace` exits 0 with 660 tests passing across 62 binaries (slice-scenario row 2 — "exit code 0 and test count >= baseline"). `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0 with zero `result_large_err` lines (slice-scenario row 1). Every previously match-on-`Err(ParseError::Variant)` test was rewritten to destructure through the box and still asserts on the original variant identity: `speccy-core/tests/workspace_loader.rs:140` (`match err.as_ref() { ParseError::StraySpecToml { path } => ... }` keeps the `path == &stray` field assertion), `:261` (`matches!(err.as_ref(), ParseError::LegacyMarker { .. })` keeps the `format!("{err}")` substring asserts on the suggested-element message), `:419` (`matches!(err.as_ref(), ParseError::DuplicateMarkerId { .. })`); `speccy-core/tests/task_xml_body_items.rs:151/183/212/247/283` all keep their inner `if task_id == "T-001" && value == "..."` guards intact across the box-destructure; `speccy-core/src/parse/spec_md.rs:557/579` and `parse/frontmatter.rs:158` likewise. `format!("{err}")` and `err.to_string()` assertion sites pass through `Box<ParseError>`'s blanket `Display` impl without modification — verified by re-running `task_xml_body_items::missing_session_attribute_surfaces_dedicated_variant` and `workspace_loader::stray_legacy_marker_spec_md_surfaces_as_legacy_marker_error`, both of which assert on `msg.contains(...)` substrings that survive the boxing unchanged (user-facing scenario row 5 satisfied). The `CommitError::From<Box<ParseError>>` migration at `speccy-core/src/tasks.rs:80-83` is covered by the existing `tasks_commit` integration suite (no regressions). Negative-path coverage is preserved: every test that expected an `Err` of a specific variant still asserts on that variant — none collapsed to `is_err()` or to a `_` catch-all.

Three test-only refactors warrant explicit scrutiny because they aren't mechanical box-threading but landed in the same diff to satisfy `-D warnings` after `result_large_err` stopped masking sibling violations; all three preserve assertion strength:
1. `speccy-core/tests/task_xml_body_items.rs:341-391` — the old terminal `(other_a, other_b) => panic!("body_items variant drift across round-trip: ...")` catch-all was retired (workspace-wide `panic = deny`) and replaced with `assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b), "body_items variant drift...")` at the top of the loop. Functionally equivalent: any cross-variant pair now fails the discriminant `assert_eq!` before the inner match runs, so the round-trip is still caught when, e.g., an `ImplementerNote` round-trips as a `Review`. The new `_ => {}` arm is reachable only when discriminants match but a future variant lacks an explicit arm — that's a separate latent gap (a new `BodyItem::*` variant would be silently un-asserted), but pre-existing in shape (the old `panic!` would fire only for cross-variant pairs, not same-variant unmatched fields), so the regression surface is identical.
2. `speccy-core/tools/migrate_tasks_schema/tests/migration.rs:170-220` — `match` arms with `other => panic!(...)` were replaced with `assert!(matches!(item_n, BodyItem::Variant { .. }), "...") + let BodyItem::Variant { .. } = item_n else { return; };`. The `assert!` fires (and the test panics) before the `let-else { return; }` can execute, so the failure semantics match the prior `panic!` exactly. The `else { return; }` is structurally unreachable in a passing test; a future contributor who weakens the `assert!` to a warning would mask the test bug, but that risk is symmetric to weakening any `assert!` and not a current-state defect.
3. `speccy-cli/tests/review_redaction.rs:164-200` and `speccy-cli/tests/init.rs:1048-1058` — `[range]` slice indexing was swapped for `.get(range).expect("...")` and `assert!(false) + unreachable!()` for `assert!(opt.is_some()) + .expect(...)`. Equivalent panic-on-failure semantics with better diagnostic messages.

One minor finding worth recording but not blocking from this persona's perspective: the user-facing CHK-001 type-sizes scenario ("zero entries whose `Err` parameter is `error::ParseError` (unboxed) ... after this task lands") reads literally over `-Zprint-type-sizes` output, and `cargo +nightly rustc --release -p speccy-core --lib -- -Zprint-type-sizes` still emits 7 lines with an unboxed `Err` — e.g., `Result<RawSpeccyConfig, error::ParseError>` (128 B), `Result<usize, error::ParseError>` (128 B), `Result<(String, ElementSpan), error::ParseError>` (128 B), and `ControlFlow<Result<Infallible, error::ParseError>, ...>` desugarings (128 B). These are not function signatures — they're compiler-emitted intermediate types for `map_err(|e| ParseError::Toml { ... })?` closure bodies (`toml_files.rs:54`) and for the `validate_workspace_xml`-side `Vec<ParseError>` aggregation path which DEC-003 explicitly carves out. Because (a) `clippy::result_large_err` only fires on `fn -> Result<T, E>` signatures and exits 0 cleanly, (b) the SPEC's primary contract is the lint (per its `## Summary` and Goal 1), and (c) the literal grep that catches these intermediates would also catch the DEC-003-blessed `Vec<ParseError>` collection path, the over-tight scenario text is a wording artifact, not a missed migration. The style reviewer's blocking call on `toml_files.rs:54` would eliminate one of the seven; the remaining six belong to the carved-out aggregation path. The `cargo clippy` and signature-grep scenarios (which are the lint contract) are both clean. No tests-persona blocker.
</review>

<retry>
Move the `missing_required_section` helper out from between the rustdoc block and `scan_spec_tags` in `speccy-core/src/parse/spec_xml/mod.rs`, and wrap the implicit-box `map_err` outlier at `speccy-core/src/parse/toml_files.rs:54` in `Box::new(...)` to match the explicit-`Box::new` pattern used at the other 117 throw-sites.
- style: `speccy-core/src/parse/spec_xml/mod.rs:230-239` — the new `missing_required_section` helper was inserted between the `///` doc block describing `scan_spec_tags` and `scan_spec_tags` itself (line 241). Rustdoc misattributes the doc comment to the helper, leaving `scan_spec_tags` undocumented. Move the helper above the doc block or below `scan_spec_tags`.
- style (non-blocking nit, fix while in the area): `speccy-core/src/parse/toml_files.rs:54` uses `.map_err(|e| ParseError::Toml { ... })` and relies on `?`'s implicit `From<T> for Box<T>` lift; every other throw-site in the diff (117 of 118) uses explicit `Box::new(ParseError::...)`. Wrap for grep-discoverability and consistency.
</retry>

<implementer-note session="spec-0030-t002-retry1-2026-05-18">
- Completed: Relocated `missing_required_section` to sit after `scan_spec_tags` in `speccy-core/src/parse/spec_xml/mod.rs` so the `///` block at lines 230-233 reattaches to its intended target `scan_spec_tags`; wrapped the `toml::from_str` `map_err` closure at `speccy-core/src/parse/toml_files.rs:54-58` in explicit `Box::new(ParseError::Toml { ... })` to match the dominant pattern used at every other throw-site.
- Undone: (none) — both retry items are resolved in this slice. Noticed but intentionally left alone (out of scope for the retry contract): `speccy-core/src/error.rs:3` module docstring still reads "All public parsers return [`Result<T, ParseError>`]" while public signatures now return `ParseResult<T>` (`Result<T, Box<ParseError>>`); pre-existing drift from the original T-002 pass, not flagged by any persona, deferred to a future doc-accuracy slice rather than expanded scope here.
- Commands run: `cargo build --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace`; `cargo +nightly fmt --all --check`; `cargo deny check`; `cargo clippy --workspace --all-targets --all-features 2>&1 | grep -c "clippy::result_large_err"`; `grep -rEn "Result<[^,>]+, ParseError>" speccy-core/src/ speccy-cli/src/`.
- Exit codes: pass; pass (0 warnings); pass (full workspace green incl. the doc-test); pass; pass; `0` (zero `result_large_err` diagnostics); one hit at `speccy-core/src/error.rs:3` (docstring only — no function signature carries the unboxed shape).
- Discovered issues: (none new) — the `error.rs:3` docstring drift noted above is pre-existing, not introduced by this retry.
- Procedural compliance: (none)
</implementer-note>

<review persona="business" verdict="pass">
The retry closes both style-blocking items without introducing new scope, and REQ-001/REQ-002 remain satisfied at HEAD.
The helper relocation in `speccy-core/src/parse/spec_xml/mod.rs` is verified: `scan_spec_tags` sits at line 234, immediately under the `///` rustdoc block at lines 230-233 ("Run the shared XML scanner with the SPEC.md whitelist..."), and `missing_required_section` now sits at line 251 — rustdoc reattaches to its intended target with no name or signature drift. The `toml_files.rs:54-58` outlier is wrapped in explicit `Box::new(ParseError::Toml { ... })`, matching the dominant explicit-box pattern at every other throw-site; the sibling `read_to_string` at lines 81-87 already uses the same shape, so the pair now reads consistently for future `grep "Box::new(ParseError::"` discovery. Re-verified the SPEC's REQ-001 done-when row 2 contract: `grep "-> Result<[^,]+, ParseError>"` against `speccy-core/src/` and `speccy-cli/src/` returns zero matches; the REQ-002 done-when row 1 contract: `grep "From<ParseError>"` against the workspace returns zero matches (only `From<Box<ParseError>>` survives); and `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0 with zero `result_large_err` lines.
The implementer-note's transparency about the pre-existing `speccy-core/src/error.rs:3` module-level docstring drift ("All public parsers return [`Result<T, ParseError>`]") is consistent with REQ-001's done-when carve-out ("The `error.rs` file may still mention the bare `ParseError` in its own variant definitions and docstrings"), so deferring that doc-accuracy edit to a future slice respects the "surgical changes" principle in AGENTS.md and keeps the retry narrow to the two items the `<retry>` block named. No `<non-goals>` entry is freshly touched by this retry. No open question is silently resolved. No `#[allow]`/`#[expect]` was added to silence anything. Display passthrough remains byte-identical via the std `Display for Box<T>` blanket; the slice is ready for the orchestrator to flip back to `completed`.
</review>

<review persona="security" verdict="pass">
Retry round introduces no security surface; both round-1 fixes are pure refactors with zero attacker-reachable behavior change. The `missing_required_section` helper relocation (`speccy-core/src/parse/spec_xml/mod.rs:251`, now after `scan_spec_tags` at line 234) is a position-only edit — body identical, no new code, no Display string drift. The `toml_files.rs:54-58` change wraps the existing `ParseError::Toml { path, message }` construction in explicit `Box::new(...)` (functionally equivalent to the prior `?`-implicit `From<T> for Box<T>` lift); the variant body, the `path: path.to_path_buf()` capture, and the `message: e.to_string()` capture are byte-identical to the round-1 diff, so the Display output an attacker could trigger by feeding a malformed `speccy.toml` is unchanged. No new logging or telemetry (verified `grep ^+` in the round-2 diff for `println!`/`eprintln!`/`tracing::`/`log::` — zero matches). No new `unwrap()`/`expect()`/`panic!()`/`unreachable!()`/`todo!()` in production code. No dependency manifest edit (`Cargo.toml`/`Cargo.lock` untouched between round 1 and round 2). The pre-existing module-level docstring at `speccy-core/src/error.rs:3` that the implementer-note flags as drift is doc-only, no semantic surface, and carries no info-disclosure risk. Round-1's no-findings security posture stands at HEAD: type-system plumbing only, no auth/authz/crypto/secret/network/deserialization-trust-boundary change.
</review>

<review persona="tests" verdict="pass">
Retry leaves the test corpus intact and every previously-asserted slice-level and user-facing scenario still passes at HEAD. Re-verified locally: `cargo test --workspace` exits 0 with 660 tests passing (slice-scenario row 2 — "test count >= baseline" — preserved exactly; the retry added, deleted, and skipped zero tests). `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0 with zero `result_large_err` lines (slice-scenario row 1 still satisfied). Neither retry edit touched an assertion or test body: the helper relocation in `speccy-core/src/parse/spec_xml/mod.rs` is pure file-order (rustdoc at 230-233 now correctly attaches to `scan_spec_tags` at line 234, and `missing_required_section` lives at line 251 — both signatures byte-identical to round 1), and the `toml_files.rs:54-58` `Box::new(ParseError::Toml { path: path.to_path_buf(), message: e.to_string() })` rewrap keeps the variant fields and `Display` text byte-identical to round 1, so the existing in-file `speccy_toml`-test assertions `matches!(*err, ParseError::UnsupportedSchemaVersion { value: 2, .. })` at `toml_files.rs:136` and `matches!(err.as_ref(), ParseError::Io { path: errpath, .. } if errpath == path)` at `toml_files.rs:147` still match and both ran green. The throw-site grep contract the round-1 retry was meant to enforce — `grep -c "Box::new(ParseError::" speccy-core/src/parse/toml_files.rs` — now reads 3 (the new line-55 wrap plus the pre-existing pair at lines 70 and 82), so the file-local pattern is uniform and future contributors grepping for `Box::new(ParseError::` will see every throw-site. The CHK-001 type-sizes wording-artifact noted in round 1 shrinks by one entry after the retry (the `toml::from_str` closure no longer returns a bare `ParseError`), and the remaining unboxed-`Err` `-Zprint-type-sizes` lines all belong to closure desugarings and the DEC-003-blessed `Vec<ParseError>` aggregation path; the lint and signature-grep scenarios remain the authoritative contract and stay clean. No mocking surface, no negative-path collapse, no flakiness vector introduced; the retry is a strict no-op from the tests-persona view.
</review>

<review persona="style" verdict="pass">
Both round-1 retry items land cleanly at HEAD with no fresh style drift. `speccy-core/src/parse/spec_xml/mod.rs`: the `///` rustdoc block at lines 230-233 ("Run the shared XML scanner with the SPEC.md whitelist...") now sits immediately above `scan_spec_tags` at line 234, with `missing_required_section` relocated to line 251 — rustdoc reattaches the doc to its intended target and `missing_required_section` is intentionally undocumented (a trivial three-line `ParseError`-constructor helper, consistent with the surrounding undocumented short helpers like `find_attr` at `task_xml/mod.rs`). `speccy-core/src/parse/toml_files.rs:54-58` is now `.map_err(|e| Box::new(ParseError::Toml { ... }))`, matching the explicit-`Box::new` pattern used at the other 117 throw-sites (verified: `Grep "Box::new\(ParseError::"` in `speccy-core/src/` returns 118 occurrences across 8 files, with zero implicit-box `.map_err(|e| ParseError::...)` survivors and zero bare `return Err(ParseError::...)` / `ok_or_else(|| ParseError::...)` outliers). Imports tracked correctly: `lint/types.rs` drops the now-unused `use crate::error::ParseError;` and replaces it with `use crate::error::ParseResult;`; every other module retains both because it still constructs `ParseError` variants at throw-sites. No new `#[allow(...)]` / `#[expect(...)]` was added in production code by the diff (`git diff HEAD --unified=0 -- speccy-core/src/ speccy-cli/src/ | rg "^\+.*#\[(allow|expect)"` returns zero matches), so the workspace `allow_attributes = "deny"` policy stays satisfied. Test-only refactors made necessary by `panic = "deny"` / `string_slice = "deny"` / `assertions_on_constants = "deny"` / `unreachable = "deny"` after `result_large_err` stopped masking them — discriminant-comparison loop guard in `task_xml_body_items.rs:347-351`, `assert!(matches!(...)) + let-else { return; }` in `migrate_tasks_schema/tests/migration.rs:170-220`, `.get(range).expect(...)` slice access in `review_redaction.rs:164-200`, and `assert!(opt.is_some()) + .expect(...)` in `init.rs:1048-1058` — all use canonical safe idioms compatible with the strict lint set, with no `#[allow]` suppressions, and the implementer-note's `Discovered issues` section enumerates each fix transparently. Hygiene gate clean at HEAD: `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0, `cargo +nightly fmt --all --check` exits 0, `cargo test --workspace` exits 0. The pre-existing `speccy-core/src/error.rs:3` module-docstring drift ("All public parsers return [`Result<T, ParseError>`]") is correctly deferred — the SPEC's REQ-001 done-when carve-out explicitly allows `error.rs` to mention bare `ParseError` in its own docstrings, the implementer-note flags the drift transparently under `Undone:`, and AGENTS.md "Surgical changes" backs leaving it for a future doc-accuracy slice rather than widening the retry. The `SpecXmlArtifacts` rustdoc at `speccy-core/src/workspace.rs:531-532` still phrases the contract as `Result<_, Box<ParseError>>` even though the field types use the `ParseResult<_>` alias — that's a cosmetic doc-vs-code phrasing nit, not blocking; both forms are explicitly named equivalents in the task entry, and surfacing the desugared shape in user-facing rustdoc is arguably clearer than naming the alias.
</review>
</task>

## Phase 3: Close out the carry-forward on prior specs

<task id="T-003" state="completed" covers="REQ-004">
## T-003: Append `## Changelog` rows to SPEC-0026 and SPEC-0027 referencing SPEC-0030 as the closure

Pure documentation task. Lands after T-002 (so the
verification step `cargo clippy --workspace --all-targets
--all-features -- -D warnings` can exit 0 at this commit and
the row's claim — "closed by SPEC-0030" — is true at HEAD).

Concretely:

- In `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md`:
  - Locate the existing `## Changelog` block (wrapped in a
    `<changelog>` element per SPEC-0019/0020 conventions).
  - Append a new row dated `2026-05-18` (or the actual commit
    date) with a `reason` column referencing SPEC-0030 as the
    closure of the T-003 carry-forward. Suggested wording:
    "Closed T-003 discovered-issue carry-forward
    (`clippy::result_large_err` against `ParseError`); fixed
    in SPEC-0030 by boxing at parser API boundary."

- In `.speccy/specs/0027-host-native-personas/SPEC.md`:
  - Locate the existing `## Changelog` block.
  - Append a parallel row dated `2026-05-18` with a `reason`
    column referencing SPEC-0030 as the closure of the
    inherited carve-out. Suggested wording: "Closed inherited
    `clippy::result_large_err` carve-out (from SPEC-0026
    T-003); fixed in SPEC-0030."

No edit to SPEC-0028's `## Assumptions` block. Per REQ-004's
prose ("SPEC-0028 is not amended for this — it is shipped and
its prose stays as the historical record"), the assumption
that "the existing pin survives" is left in place as the
historical record of why SPEC-0028 itself did not fix the
lint.

No edit to `.speccy/BACKLOG.md` here — flipping SPEC-0030's
own status row to `implemented` is `speccy-ship`'s job, not
this task's.

Suggested files:

- `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md`
- `.speccy/specs/0027-host-native-personas/SPEC.md`

<task-scenarios>
Given `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md`
after this task lands, when its `<changelog>` element body is
read, then at least one row references `SPEC-0030` and names
the T-003 `result_large_err` carry-forward as the closed item.

Given `.speccy/specs/0027-host-native-personas/SPEC.md` after
this task lands, when its `<changelog>` element body is read,
then at least one row references `SPEC-0030` and names the
inherited `result_large_err` carve-out as the closed item.

Given `.speccy/specs/0028-retire-mtime-drift-stale-signal/SPEC.md`
after this task lands, when its `<assumptions>` block is read,
then the pre-existing line about `clippy::result_large_err`
"remains out of scope ... the existing pin survives" is
unchanged (SPEC-0028's prose is the historical record per
REQ-004; it is not edited).

Given `speccy verify` run against the working tree at the
commit that lands this task, when its exit code is captured,
then the exit code is 0 and no proof-shape diagnostic is
emitted against SPEC-0026, SPEC-0027, or SPEC-0030.

Given the standard hygiene gate
(`cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly
fmt --all --check`, `cargo deny check`) run against the same
commit, when each command's exit code is captured, then all
four exit 0.
</task-scenarios>

<implementer-note session="spec-0030-t003-2026-05-18">
- Completed: Appended a new dated `2026-05-18` row to the `<changelog>` element body in `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md` referencing SPEC-0030 as the closure of the T-003 `result_large_err` carry-forward, and a parallel row to `.speccy/specs/0027-host-native-personas/SPEC.md` referencing SPEC-0030 as the closure of the inherited carve-out. SPEC-0028 `## Assumptions` block was left untouched per REQ-004 prose (historical record).
- Undone: `.speccy/BACKLOG.md` status flip for SPEC-0030 to `implemented` is `speccy-ship`'s job, not T-003's; deferred to the ship phase per REQ-004 done-when ¶3.
- Commands run: `cargo run --bin speccy -- verify`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo +nightly fmt --all --check`; `cargo deny check`.
- Exit codes: pass (30 specs, 158 requirements, 196 scenarios, 0 errors); pass; pass; pass; pass.
- Discovered issues: (none)
- Procedural compliance: (none)
</implementer-note>

<review persona="business" verdict="pass">
Diff faithfully delivers REQ-004's documentation contract: one 2026-05-18 row appended inside the `<changelog>` element body of both SPEC-0026 (`.speccy/specs/0026-skill-router-anti-triggers/SPEC.md:797`) and SPEC-0027 (`.speccy/specs/0027-host-native-personas/SPEC.md:930`), each referencing SPEC-0030 and naming the closed item (T-003 `result_large_err` carry-forward; inherited carve-out). SPEC-0028's `## Assumptions` block is untouched, matching REQ-004 prose that explicitly leaves SPEC-0028 as the historical record. `.speccy/BACKLOG.md` is correctly deferred to `speccy-ship`. The task entry's "suggested wording" references a `reason` column but both target files actually use a `Summary` column; the implementer correctly used the file's real schema rather than fabricating a column — the slice contract is on substring content, not column name.
</review>

<review persona="security" verdict="pass">
Pure documentation slice — two `<changelog>` rows appended to `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md:797` and `.speccy/specs/0027-host-native-personas/SPEC.md:930`. No code paths, auth boundaries, input handling, crypto primitives, secrets, logging sinks, or new dependencies are touched; the row content (dates, `human/kevin` author label matching pre-existing convention, spec IDs, clippy lint name, the public Rust type path `speccy_core::error::ParseError`) carries no sensitive data and introduces no injection, traversal, or disclosure surface. SPEC-0028's `## Assumptions` block is untouched (`git diff 14f8cac HEAD -- .speccy/specs/0028-retire-mtime-drift-stale-signal/SPEC.md` is empty), satisfying the historical-record scenario without back-rewriting shipped prose.
</review>

<review persona="style" verdict="pass">
Both appended `<changelog>` rows match the existing row style exactly. Date `2026-05-18` (`.speccy/specs/0026-skill-router-anti-triggers/SPEC.md:797`, `.speccy/specs/0027-host-native-personas/SPEC.md:930`) is ISO-8601, matching the pre-existing `2026-05-17` row in each file; the `human/kevin` author label, the three-cell pipe layout, and the surrounding `<changelog>` element framing all line up with SPEC-0019/0020 conventions. Backtick code spans (`clippy::result_large_err`, `speccy_core::error::ParseError`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `ParseError`) match the prose style used elsewhere in both SPEC files; spec-id references (`SPEC-0030`, `SPEC-0026 T-003`) use the existing dotted-dash form. No raw `|` characters leak into table cells, no `#[allow]`/`#[expect]`-equivalent suppression annotations are introduced (the slice is docs-only), and no formatter would re-wrap the single-line row format. No style drift to flag.
</review>

<review persona="tests" verdict="pass">
The four doc-shaped task-scenarios are mechanically satisfied by the diff. `git diff` against the SPEC files shows: (1) a 2026-05-18 row at `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md:797` whose body references `SPEC-0030` and names the T-003 `result_large_err` carry-forward; (2) a parallel row at `.speccy/specs/0027-host-native-personas/SPEC.md:930` referencing `SPEC-0030` and the inherited carve-out from SPEC-0026 T-003 / SPEC-0028; (3) no diff against `.speccy/specs/0028-retire-mtime-drift-stale-signal/SPEC.md`, so the "the existing pin survives" line at SPEC-0028.md:566-569 is preserved as historical record per REQ-004. The `speccy verify` and standard-hygiene scenarios are exit-code claims that properly belong to T-002 / CI rather than to this doc-only slice; nothing in the changelog string appends could regress them. Per SPEC-0029 redaction, the implementer-note self-claim of pass exit codes is correctly out of scope for this verdict.
</review>
</task>

</tasks>
