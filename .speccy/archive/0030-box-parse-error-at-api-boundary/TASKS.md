---
spec: SPEC-0030
spec_hash_at_generation: f4b80b170b5248213985be89b97bd04168e1d1b8dafacaaa2f226f697c8f205a
generated_at: 2026-05-18T21:35:05Z
---

# Tasks: SPEC-0030 Box ParseError at every parser API boundary so `clippy::result_large_err` stops blocking the build


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
</task>

