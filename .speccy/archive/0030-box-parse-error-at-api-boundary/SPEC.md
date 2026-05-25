---
id: SPEC-0030
slug: box-parse-error-at-api-boundary
title: Box ParseError at every parser API boundary so clippy::result_large_err stops blocking the build
status: implemented
created: 2026-05-18
supersedes: []
archived_at: 2026-05-23
archived_reason: "v1 milestone shipped"
---

# SPEC-0030: Box ParseError at every parser API boundary so `clippy::result_large_err` stops blocking the build

## Summary

`speccy-core::error::ParseError` measures 128 bytes wide (verified
via `cargo +nightly rustc -Zprint-type-sizes`). The workspace pins
`clippy::all = deny` and `pedantic = deny` (`Cargo.toml`'s
`[workspace.lints.clippy]`), which makes `clippy::result_large_err`
a hard error at exactly the 128-byte threshold. As a result, every
parser function in `speccy-core` that returns `Result<T, ParseError>`
(45+ call sites across `parse/spec_md.rs`, `parse/spec_xml/`,
`parse/task_xml/`, `parse/report_xml/`, `parse/xml_scanner/`,
`parse/frontmatter.rs`, `parse/toml_files.rs`, and `workspace.rs`)
fails `cargo clippy --workspace --all-targets --all-features`. The
project's standard hygiene gate (AGENTS.md "Standard hygiene")
cannot pass.

The largest variants (`UnknownMarkerAttribute`, `LegacyMarker`,
`InvalidMarkerAttributeValue`, `DuplicateRequirementSection`,
`MissingImplementerNoteSession`, `RequirementSectionOrder`) each
carry one `Utf8PathBuf` (24 B) plus three to four owned `String`
fields (24 B each) plus a `usize` (8 B). Even after layout
optimisation the variant payload reaches the 128-byte threshold.
`ParseError` has 40+ variants and every variant participates in
the same `Err` channel, so per-call-site `#[allow]` is not viable
and would conflict with AGENTS.md's "no `#[allow(...)]` to silence
a lint" rule.

The pre-existing pin has been carried forward across three prior
specs:

- SPEC-0026 T-003 explicitly carved it out as out-of-scope
  ("pre-existing on main/6ed6e39 baseline ... Recommend a
  follow-up SPEC to box the large ParseError variants",
  `.speccy/specs/0026-skill-router-anti-triggers/TASKS.md:364`).
- SPEC-0027 inherited the same carve-out
  (`.speccy/specs/0026-skill-router-anti-triggers/TASKS.md:486`).
- SPEC-0028's `## Assumptions` reaffirmed: "the existing pin
  survives"
  (`.speccy/specs/0028-retire-mtime-drift-stale-signal/SPEC.md:566-569`).

SPEC-0030 is that follow-up. It boxes `ParseError` at every public
and private parser API boundary in `speccy-core` so the lint stops
firing, while keeping the enum shape, every variant, every
`#[error("...")]` template, and every `Display` string bit-identical
to today's output. Downstream callers that already wrapped a
`ParseError` in their own enum variant (e.g.
`speccy_cli::plan::PlanError::Parse { source: Box<ParseError>, ... }`,
`speccy-cli/src/plan.rs:74-83`) already used this pattern as a
precedent; SPEC-0030 generalises it from one consumer site to the
producer surface itself.

## Goals

<goals>
- `cargo clippy --workspace --all-targets --all-features` exits 0
  with zero `clippy::result_large_err` diagnostics against
  `speccy-core::error::ParseError`. No new `#[allow]` /
  `#[expect]` suppressions are introduced anywhere in the
  workspace as part of this change.
- Every public and private function inside `speccy-core` that
  previously returned `Result<T, ParseError>` (or stored
  `Result<T, ParseError>` in a public field) returns or stores
  `Result<T, Box<ParseError>>` instead. This includes the
  parsers in `speccy-core/src/parse/*`, the workspace loader's
  `parse_one_*` helpers, and the `ParsedSpec::tasks` /
  `ParsedSpec::report` fields.
- `speccy_core::error::ParseError`'s enum definition is unchanged:
  same variant names, same field names and types, same
  `thiserror` `#[error("...")]` strings, same `#[from]` impls,
  same `#[source]` annotations, same `#[non_exhaustive]` marker.
  No variant is added, removed, renamed, or restructured.
- The on-the-wire `Display` strings emitted by every variant
  (the messages that downstream tests grep for) are bit-identical
  before and after this SPEC. `Box<ParseError>`'s `Display` impl
  forwards transparently to the inner enum's via the std
  `impl<T: Display + ?Sized> Display for Box<T>` blanket; no
  custom impl is needed.
- Downstream consumers (`speccy_cli`, `speccy_core::tasks`,
  `speccy_core::lint`, integration tests) update their throw
  sites, propagation `?` chains, and `match` arms to thread
  `Box<ParseError>` through. Pattern matches that previously
  destructured `ParseError::Variant { .. }` now destructure
  through the box (`*err` or `&**boxed`) but keep the same
  variant arms and the same logic.
- The four existing standard-hygiene commands in AGENTS.md pass:
  `cargo test --workspace`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`, `cargo deny check`.
- A type alias `pub type ParseResult<T> = Result<T, Box<ParseError>>;`
  is added to `speccy-core/src/error.rs` so call sites that opt
  in get a short, readable signature; legacy uses of the long
  form remain valid (the alias is additive, not substitutive).
</goals>

## Non-goals

<non-goals>
- No restructure of `ParseError` into a
  `struct { path: Utf8PathBuf, kind: ParseErrorKind }` shape.
  That alternative (Strategy B in the planning notes) would have
  factored the shared `path` field out of every variant and
  collapsed the per-variant byte count, but it touches every
  construct site, every `match` arm, and every downstream
  consumer — a much larger diff for the same lint outcome.
  Deferring it preserves the option for a future cleanup SPEC
  if a stronger motivation emerges (e.g., needing to ask "what
  file failed?" without per-variant matching).
- No per-variant boxing
  (`UnknownMarkerAttribute(Box<UnknownMarkerAttributePayload>)`).
  That alternative (Strategy C) introduces a payload struct per
  large variant, multiplies the type surface, and produces a
  worse `Debug` output. The boundary-level box is one indirection
  in one place; per-variant boxing scatters indirections across
  the enum.
- No relaxation of workspace lint configuration. The denied lint
  set in `Cargo.toml`'s `[workspace.lints.clippy]` stays exactly
  as it is. SPEC-0030 fixes the underlying size; it does not
  silence the lint.
- No `#[allow(clippy::result_large_err)]` or
  `#[expect(clippy::result_large_err, ...)]` attribute added
  anywhere. AGENTS.md "Conventions for AI agents specifically"
  forbids `#[allow]` to silence lints; this SPEC respects that.
- No change to the public `Display` strings emitted by any
  variant. `assert!(format!("{err}").contains(...))` patterns
  in downstream tests continue to pass without modification.
- No new error variants. If a parser site discovers an
  uncovered failure mode while threading the box through, it
  is out of scope for SPEC-0030 and goes to a follow-up SPEC.
- No change to `[#non_exhaustive]` on `ParseError`. The enum
  stays non-exhaustive; the box is orthogonal to that marker.
- No change to `speccy-cli::plan::PlanError::Parse`'s existing
  shape. It already carries `source: Box<ParseError>`; this
  SPEC merely makes the surrounding signatures match the
  pattern it already uses.
- No edit to the workspace-level Cargo manifest's
  `large_enum_variant` lint pin (currently `warn`). That lint
  fires on the enum definition itself; if it surfaces after the
  signature change (it won't, because the enum body is
  unchanged), it gets a dedicated follow-up.
- No `Cow<ParseError>` / `Arc<ParseError>` /
  `SmallBox<ParseError>` alternative carrier. `Box` is the
  idiomatic Rust choice for "this is large, allocate it on the
  heap"; the other carriers solve different problems (shared
  ownership, small-size optimisation) that don't apply here.
- No micro-optimisation pass through the parser hot path to
  shrink `Result<T, Box<ParseError>>` further (e.g., niche-
  optimised tags). One pointer width on the Err side is enough
  to clear the lint by 120 bytes; further shrinking is
  premature.
</non-goals>

## User Stories

<user-stories>
- As a Speccy contributor running the standard hygiene gate
  before commit, I want `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` to exit 0. Today the command
  exits 101 with 45+ `result_large_err` diagnostics, and every
  PR in flight has to either skip the gate or carry the failure
  forward as out-of-scope.
- As a Speccy contributor reading `speccy-core/src/error.rs`, I
  want the enum's variant list, field names, and `#[error]`
  strings to be the source of truth I match on when writing
  downstream tests. After SPEC-0030 the enum body is unchanged;
  the `Box` lives at the function boundary, not inside the
  type definition.
- As an author of an integration test that asserts on parser
  error messages (e.g., `tests/workspace_xml.rs` and
  `tests/lint_common/mod.rs`), I want every assertion that
  matches on `format!("{err}")` or `err.to_string()` to keep
  passing without modification. SPEC-0030 preserves the
  on-the-wire Display strings exactly.
- As the implementer of a downstream consumer
  (`speccy-cli/src/plan.rs`, `speccy-cli/src/status.rs`,
  `speccy-cli/src/tasks.rs`, `speccy-core/src/tasks.rs`), I
  want the `?` operator to thread parser errors through my
  function without an explicit `.map_err(Box::new)` at every
  call site. `impl<T> From<T> for Box<T>` in std handles the
  one-way lift from a bare `ParseError` (constructed at the
  throw site) into a `Box<ParseError>` Err position, and
  `Box<ParseError>` propagates through nested calls
  transparently when both sides agree on the box.
- As a future contributor adding a new parser function, I want
  the standard signature shape to be `fn parse_x(...) ->
  ParseResult<X>` (the type alias added by this SPEC). The
  alias documents the convention in one place, so new code
  doesn't accidentally regress to a bare `Result<T, ParseError>`
  and reintroduce the lint failure.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `ParseError` is boxed at every parser function boundary in `speccy-core`

Every function inside `speccy-core` whose signature today reads
`-> Result<T, ParseError>` (for any concrete `T`) changes to
`-> Result<T, Box<ParseError>>`. This covers public APIs (e.g.
`parse::spec_md::parse`, `parse::spec_xml::parse`,
`parse::task_xml::parse`, `parse::report_xml::parse`,
`parse::frontmatter::split`, `parse::toml_files::parse_speccy_toml`)
and every private helper that fans out from them
(`xml_scanner::scan_*`, `report_xml::parse_scenarios`,
`report_xml::extract_level1_heading`, `report_xml::assemble`,
`report_xml::validate_tag_shape`, `workspace::parse_spec_doc`,
`workspace::parse_one_tasks_xml`, `workspace::parse_one_report_xml`,
and every other site flagged by `cargo clippy` with
`clippy::result_large_err` against `error::ParseError`).

The public field types `ParsedSpec::tasks` and `ParsedSpec::report`
change from `Option<Result<TasksDoc, ParseError>>` /
`Option<Result<ReportDoc, ParseError>>` to
`Option<Result<TasksDoc, Box<ParseError>>>` /
`Option<Result<ReportDoc, Box<ParseError>>>` to match the
underlying parsers.

The free function `validate_workspace_xml` keeps returning
`Vec<ParseError>` (an owned collection of diagnostics, not a
`Result`); `result_large_err` does not apply to non-`Result`
returns. The diagnostic-collection path is unaffected.

Each throw site inside a parser body that previously wrote
`return Err(ParseError::Variant { ... })` updates to
`return Err(Box::new(ParseError::Variant { ... }))` or the
equivalent `.into()` shorthand backed by the standard library's
`impl<T> From<T> for Box<T>`. The variant construction itself
is byte-identical to today's code.

<done-when>
- `cargo clippy --workspace --all-targets --all-features` exits
  0. `grep -c "result_large_err" <clippy_output>` is 0.
- `grep -rn "Result<.*, ParseError>" speccy-core/src/` outside
  the `error.rs` definition file and outside the
  `validate_workspace_xml`-returned `Vec<ParseError>` site
  returns zero matches. (The `error.rs` file may still mention
  the bare `ParseError` in its own variant definitions and
  docstrings; the type alias `ParseResult<T>` is the
  canonical shape for new code.)
- `speccy_core::workspace::ParsedSpec`'s public fields
  `tasks` and `report` have type
  `Option<Result<_, Box<ParseError>>>` after this requirement
  lands.
- `cargo +nightly rustc --release --package speccy-core --lib
  -- -Zprint-type-sizes 2>&1 | rg "Result<.*, error::ParseError>"`
  no longer prints any entry with an `Err` payload ≥128 bytes;
  the surviving `Result<T, Box<ParseError>>` entries have an
  `Err` payload of `Box<error::ParseError>`: 8 bytes.
- `cargo test --workspace`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`, `cargo deny check`
  all exit 0.
</done-when>

<behavior>
- Given the file `speccy-core/src/parse/spec_md.rs` after
  this requirement lands, when its public `fn parse(...)`
  signature is inspected, then its return type is
  `Result<SpecMd, Box<ParseError>>` (or the equivalent
  `ParseResult<SpecMd>` alias).
- Given every other parser function listed above, when each
  signature is inspected, then the `Err` parameter on its
  `Result` is `Box<ParseError>`, not the bare `ParseError`.
- Given the workspace loader's `ParsedSpec` struct, when its
  field types are inspected via `cargo doc` or direct file
  read, then `tasks` and `report` are
  `Option<Result<_, Box<ParseError>>>`.
- Given a parser invocation that produces an error (e.g.
  `parse::spec_md::parse(invalid_input, path)`), when the
  returned `Err(boxed)` is formatted with `format!("{boxed}")`
  or `format!("{}", *boxed)`, then the resulting string
  matches the pre-SPEC-0030 Display output for the same
  underlying `ParseError` variant byte-for-byte.
- Given a downstream `?` chain that previously propagated
  `ParseError` through a function returning
  `Result<T, ParseError>`, when both ends of the chain are
  updated to `Result<T, Box<ParseError>>` together, then
  the `?` operator continues to compile and behaves
  identically at runtime.
</behavior>

<scenario id="CHK-001">
Given the file `speccy-core/src/parse/spec_md.rs` after
this requirement lands,
when its public `pub fn parse(...)` signature is read,
then the return type is exactly `Result<SpecMd, Box<ParseError>>`
(or `ParseResult<SpecMd>`, the alias defined in REQ-003).

Given every parser file in `speccy-core/src/parse/` and the
workspace loader `speccy-core/src/workspace.rs`,
when grepped for the regex
`-> Result<[^,]+, ParseError>` (i.e., the unboxed shape),
then exactly zero matches are returned outside the
diagnostic-collection helper `validate_workspace_xml`'s
`Vec<ParseError>` return.

Given the workspace loader's `ParsedSpec` struct definition,
when its public fields are enumerated,
then `tasks: Option<Result<TasksDoc, Box<ParseError>>>` and
`report: Option<Result<ReportDoc, Box<ParseError>>>` appear
with the boxed Err.

Given `cargo +nightly rustc --release --package speccy-core
--lib -- -Zprint-type-sizes 2>&1` after this requirement lands,
when filtered for entries whose `Err` parameter is
`error::ParseError` (unboxed),
then exactly zero such entries are emitted by any function
inside `speccy-core` (the only surviving raw-`ParseError`
mention is the `Vec<ParseError>` and `Option<ParseError>`
in `validate_workspace_xml`'s aggregation path).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Downstream consumers in `speccy-cli` and `speccy-core` thread `Box<ParseError>` through their own error types and `match` arms

Every site outside `speccy-core/src/parse/` and
`speccy-core/src/workspace.rs` that consumes a `ParseError`
updates to consume `Box<ParseError>`. Concretely:

- `speccy_core::tasks::CommitError`'s existing
  `impl From<ParseError> for CommitError`
  (`speccy-core/src/tasks.rs:80`) becomes
  `impl From<Box<ParseError>> for CommitError`. The
  variant on `CommitError` that holds the parser failure
  changes its source type to `Box<ParseError>` or remains
  a transparent wrapper around the box.
- `speccy_cli::plan::PlanError::Parse`'s `source` field is
  already `Box<ParseError>` (`speccy-cli/src/plan.rs:82`).
  Confirm no change is needed there, but verify the
  `#[from]`/`#[source]` annotations still line up after
  the upstream signature change in REQ-001.
- Every `match` arm in `speccy-cli/` and `speccy-core/`
  that previously destructured `Err(ParseError::Variant {
  .. })` updates to destructure through the box, e.g.
  `Err(boxed_err) => match *boxed_err { ParseError::Variant
  { .. } => ... }` or the equivalent reference form
  `Err(ref boxed_err) => match boxed_err.as_ref() { ... }`.
  Variant arms and guard logic are unchanged.
- Every assertion in integration tests
  (`speccy-core/tests/workspace_xml.rs`,
  `speccy-core/tests/workspace_loader.rs`,
  `speccy-core/tests/task_xml_body_items.rs`,
  `speccy-core/tests/lint_common/mod.rs`, and every other
  `tests/*.rs` that touches `ParseError`) is updated so
  that `Err(boxed)` patterns destructure through the box.
  Assertions on `format!("{err}")` content remain
  unchanged (Display passes through the box transparently).

The boundary between `speccy-core` and `speccy-cli`
remains the canonical place where the box is constructed:
`speccy-cli` does not unbox into a bare `ParseError` and
re-box. It either holds `Box<ParseError>` directly in its
own error variants or propagates via `?` into another
`Box<ParseError>`-typed `Err`.

<done-when>
- `grep -rn "From<ParseError>" speccy-core/ speccy-cli/`
  returns either zero matches or matches that all read
  `From<Box<ParseError>>`. The unboxed `From<ParseError>`
  impl is removed.
- `grep -rn "Err(ParseError::" speccy-core/ speccy-cli/`
  returns matches only inside `speccy-core/src/parse/` and
  `speccy-core/src/workspace.rs` (the construction sites
  that wrap into a box on the same line). Downstream
  consumer match arms use the `*boxed_err` or
  `boxed_err.as_ref()` destructure pattern.
- `cargo test --workspace -p speccy-core -p speccy-cli`
  exits 0. The full workspace test count is preserved
  (no tests are deleted; only their pattern-matching
  syntax updates).
- `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` exits 0.
</done-when>

<behavior>
- Given `speccy-core/src/tasks.rs`'s `CommitError` after
  this requirement lands, when its `From` impls are
  enumerated, then the parser-failure conversion reads
  `impl From<Box<ParseError>> for CommitError`, not
  `impl From<ParseError> for CommitError`.
- Given any test in `speccy-core/tests/` that previously
  matched on `Err(ParseError::Foo { .. })`, when its
  current source is inspected, then it matches on
  `Err(ref boxed_err)` (or equivalent) and destructures
  the inner variant via `boxed_err.as_ref()` or
  `*boxed_err` — the variant identity check itself is
  unchanged.
- Given `speccy-cli/src/plan.rs`'s `PlanError::Parse`
  variant, when inspected after this requirement lands,
  then its `source` field type is unchanged
  (`Box<ParseError>`) and its `#[source]` annotation is
  unchanged.
- Given any `format!("{err}")` or `err.to_string()` call
  on a propagated `Box<ParseError>` (or its embedding in
  a downstream error enum's `Display`), when the
  resulting string is captured, then it equals the
  pre-SPEC-0030 string for the same underlying variant
  byte-for-byte.
</behavior>

<scenario id="CHK-002">
Given `speccy-core/src/tasks.rs`'s `CommitError` after
this requirement lands,
when the `impl From<_> for CommitError` lines are
enumerated by grep,
then the parser-failure conversion is
`impl From<Box<ParseError>> for CommitError` and no
`impl From<ParseError> for CommitError` impl remains.

Given the integration test
`speccy-core/tests/workspace_xml.rs` after this
requirement lands,
when its `Err` pattern matches against parser failures
are read,
then they destructure the box (e.g., via
`Err(ref boxed_err)` plus `boxed_err.as_ref()` /
`*boxed_err`) and assert on the same variant identity
that the pre-SPEC-0030 test asserted on.

Given `cargo test --workspace` after this requirement
lands,
when its run completes, then the exit code is 0 and
the total test count is greater than or equal to the
pre-SPEC-0030 count (no tests are removed; only their
pattern syntax updates).

Given a downstream consumer (e.g.,
`speccy-cli/src/status.rs`) that propagates a parser
failure via `?` from a `Box<ParseError>`-typed `Err`
into its own error enum,
when the propagation site is inspected after this
requirement lands,
then the `?` chain compiles without an explicit
`.map_err(...)` call (the consumer's error enum
exposes a `From<Box<ParseError>>` impl, either by
`#[from]` on a variant field or by an explicit
`impl` block).
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: A `ParseResult<T>` type alias documents the convention

A new public type alias

```rust
/// Convenience alias for parsers that return a boxed [`ParseError`].
/// Boxing the error keeps `Result<T, _>` under the
/// `clippy::result_large_err` threshold so the
/// workspace-wide `pedantic = "deny"` pin stays satisfied.
/// See SPEC-0030.
pub type ParseResult<T> = std::result::Result<T, Box<ParseError>>;
```

is added to `speccy-core/src/error.rs`. The alias is
exported from the crate root (`speccy-core/src/lib.rs`
re-exports `error::{ParseError, ParseResult}` together).
Parser function signatures may use the alias or the
expanded form; both are equivalent. New code is
encouraged via the alias's docstring to prefer the alias.

The alias is additive. Existing call sites that spell
the expanded `Result<T, Box<ParseError>>` form are not
required to migrate to the alias in this SPEC.

<done-when>
- `speccy-core/src/error.rs` contains a `pub type
  ParseResult<T> = std::result::Result<T, Box<ParseError>>;`
  declaration with a doc comment that mentions
  `clippy::result_large_err` and references SPEC-0030.
- `speccy-core/src/lib.rs` re-exports `ParseResult`
  alongside the existing `ParseError` re-export, so
  downstream consumers can write
  `use speccy_core::ParseResult;`.
- `cargo doc --workspace --no-deps` builds successfully
  and the rendered docs show `ParseResult` as a public
  type alias under `speccy_core`.
- `cargo test --workspace` exits 0; the alias does not
  break any existing call site.
</done-when>

<behavior>
- Given `speccy-core/src/error.rs` after this
  requirement lands, when the file is grepped for the
  literal `pub type ParseResult<`, then exactly one
  match is found.
- Given `speccy-core/src/lib.rs` after this requirement
  lands, when its `pub use` re-exports are enumerated,
  then `ParseResult` appears alongside `ParseError`.
- Given a new parser function written after this
  requirement lands, when its signature is written as
  `fn parse_x(input: &str) -> ParseResult<X>`, then it
  compiles without further import gymnastics beyond
  `use crate::error::{ParseError, ParseResult};` (or
  the crate-root re-export from `speccy_core`).
</behavior>

<scenario id="CHK-003">
Given `speccy-core/src/error.rs` after this
requirement lands,
when grepped for `pub type ParseResult<`,
then exactly one match exists and the right-hand side
expands to `std::result::Result<T, Box<ParseError>>`
(or the equivalent `Result<T, Box<ParseError>>`
shorthand with a `use std::result::Result;` at module
top).

Given `speccy-core/src/lib.rs` after this requirement
lands,
when its public re-exports are enumerated,
then `ParseResult` is publicly re-exported alongside
`ParseError`.

Given `cargo doc --workspace --no-deps` after this
requirement lands,
when its generated HTML is searched for the type alias
page `speccy_core/type.ParseResult.html`,
then the page exists and its rendered definition shows
`type ParseResult<T> = Result<T, Box<ParseError>>`.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: The `result_large_err` carve-out is removed from prior-SPEC `## Assumptions`/notes and the AGENTS.md project log is updated

SPEC-0028's `## Assumptions` block contains the line "The
pre-existing `clippy::result_large_err` against
`speccy_core::error::ParseError` (carried forward from
SPEC-0026 T-003) remains out of scope. SPEC-0028 does
not add new lint suppressions; the existing pin survives."
(`.speccy/specs/0028-retire-mtime-drift-stale-signal/SPEC.md:566-569`).
After SPEC-0030 lands and `cargo clippy --workspace
--all-targets --all-features -- -D warnings` exits 0, the
carve-out is no longer factual.

SPEC-0028 is not amended for this — it is shipped and
its prose stays as the historical record of why
SPEC-0028 itself did not fix the lint. The closure is
recorded instead in:

- SPEC-0026 and SPEC-0027 each receive a one-row
  `## Changelog` entry noting that the T-003 discovered
  issue ("`clippy::result_large_err` is denied
  workspace-wide but `speccy-core::parse::error::ParseError`
  triggers it ... Recommend a follow-up SPEC to box the
  large ParseError variants",
  `.speccy/specs/0026-skill-router-anti-triggers/TASKS.md:364`)
  was closed by SPEC-0030.
- The implementer note for the SPEC-0030 task that
  finishes REQ-001 cites both SPEC-0026 T-003 and
  SPEC-0028's `## Assumptions` block in its
  `Discovered issues` section as the carry-forward
  receipts.
- No edit to `AGENTS.md` itself; the SPEC index in
  `.speccy/BACKLOG.md` (the project-wide backlog) gets
  the SPEC-0030 row promoted from `pending` to
  `implemented` as part of `speccy-ship`.

<done-when>
- `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md`
  carries a new `## Changelog` row dated 2026-05-18 (or
  the actual ship date) referencing SPEC-0030 as the
  closure of the T-003 carry-forward.
- `.speccy/specs/0027-host-native-personas/SPEC.md`
  carries a parallel `## Changelog` row referencing
  SPEC-0030 as the closure of the inherited carve-out.
- `.speccy/BACKLOG.md` lists SPEC-0030 with
  status `implemented` after `speccy-ship` runs (this
  edit is part of the ship phase, not of REQ-004's own
  implementation; REQ-004 just guarantees the prior
  specs' `## Changelog` updates land in the same PR
  that flips the lint to passing).
- `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` and `cargo test --workspace` both
  exit 0 in the commit that lands the prior-SPEC
  changelog rows.
</done-when>

<behavior>
- Given
  `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md`
  after this requirement lands, when its `## Changelog`
  block is read, then at least one row references
  SPEC-0030 and names the T-003 discovered-issue as
  the closed item.
- Given
  `.speccy/specs/0027-host-native-personas/SPEC.md`
  after this requirement lands, when its `## Changelog`
  block is read, then at least one row references
  SPEC-0030 and names the inherited `result_large_err`
  carve-out as the closed item.
- Given `.speccy/BACKLOG.md` after SPEC-0030 ships,
  when scanned for `SPEC-0030`, then the row's status
  column reads `implemented`.
</behavior>

<scenario id="CHK-004">
Given
`.speccy/specs/0026-skill-router-anti-triggers/SPEC.md`
after this requirement lands,
when grepped for the literal substring `SPEC-0030`,
then at least one match falls inside the
`## Changelog` block's `<changelog>` element body and
that match's `reason` column names the T-003
`result_large_err` carry-forward as the closed item.

Given
`.speccy/specs/0027-host-native-personas/SPEC.md`
after this requirement lands,
when grepped for the literal substring `SPEC-0030`,
then at least one match falls inside the
`## Changelog` block and names the inherited
`result_large_err` carve-out as the closed item.

Given `.speccy/BACKLOG.md` after `speccy-ship`
completes for SPEC-0030,
when the row whose first column reads `SPEC-0030` is
inspected,
then its `status` column reads `implemented`.

Given `cargo clippy --workspace --all-targets
--all-features -- -D warnings` run against `HEAD`
after this requirement lands,
when its exit code is captured,
then the exit code is 0 and its diagnostic output
contains zero `clippy::result_large_err` lines.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001" status="accepted">
### DEC-001: Box at the API boundary, not inside the enum

Three viable strategies were considered:

- **A (chosen): box at the API boundary.** Every parser function
  returns `Result<T, Box<ParseError>>`; the enum body is
  untouched. One indirection lives at the function boundary;
  Display strings pass through transparently.
- **B: restructure `ParseError` to
  `{ path: Utf8PathBuf, kind: ParseErrorKind }`.** Factor the
  shared `path` field out of every variant. Cleaner long-term
  taxonomy (any caller can ask "what file failed?" without
  per-variant matching) but touches every variant, every
  construct site, every `match` arm, and every `#[error("...")]`
  template (path interpolation moves to the wrapper). Much
  larger blast radius for the same lint outcome.
- **C: box per-variant payloads.** Replace
  `UnknownMarkerAttribute { path, marker_name, attribute,
  offset, allowed }` with
  `UnknownMarkerAttribute(Box<UnknownMarkerAttributePayload>)`.
  Smaller per-variant diff than B, but scatters indirections
  across the enum, multiplies type surface, and worsens
  `Debug` output.

Strategy A is chosen. It matches the existing precedent in
`speccy-cli/src/plan.rs:74-83` (already boxes `ParseError` in
`PlanError::Parse { source: Box<ParseError> }` for exactly
the same `result_large_err` budget reason). Generalising the
boundary-level box from one consumer to the producer surface
is the consistent move. B remains available as a future
cleanup SPEC if a stronger motivation emerges (e.g., needing
to ask "what file failed?" without per-variant matching).
</decision>

<decision id="DEC-002" status="accepted">
### DEC-002: Add a `ParseResult<T>` alias, do not enforce its use

A type alias documents the convention in one place
(`speccy-core/src/error.rs`) and gives new code a one-token
signature, but existing call sites are not forced to migrate
to the alias in this SPEC. The migration to the alias is
mechanical and a future contributor can do it in one pass; the
alias's docstring (mentioning `clippy::result_large_err` and
referencing SPEC-0030) provides the breadcrumb. Enforcing
alias use in SPEC-0030 would balloon the diff for no
behavioural benefit.
</decision>

<decision id="DEC-003" status="accepted">
### DEC-003: `validate_workspace_xml` stays as `Vec<ParseError>`

`speccy_core::workspace::validate_workspace_xml`
(`speccy-core/src/workspace.rs:517`) returns
`Vec<ParseError>`, not `Result<_, ParseError>`. The
`clippy::result_large_err` lint fires only on the `Err`
position of a `Result`; a `Vec<ParseError>` collection of
diagnostics is unaffected. Boxing inside the `Vec` would
double the allocation (heap-allocated `Box<ParseError>`
inside a heap-allocated `Vec`) for zero lint benefit, so
the diagnostic-collection path stays as it is. Downstream
consumers iterate the `Vec` and clone elements out by value
when needed; nothing about that path changes.
</decision>

<decision id="DEC-004" status="accepted">
### DEC-004: No `From<ParseError> for Box<ParseError>` helper impl

The standard library provides
`impl<T> From<T> for Box<T>` for any sized `T`. That
covers the throw-site lift (`Err(ParseError::Foo { .. }.into())`
or `Err(Box::new(ParseError::Foo { .. }))`) and the `?`
operator's implicit conversion. No SPEC-0030-specific
`From` impl is added. If a downstream error enum needs to
absorb a `Box<ParseError>` via `#[from]`, it declares
`#[from] Box<ParseError>` on its own variant; the
`thiserror` macro handles the conversion.
</decision>

<decision id="DEC-005" status="accepted">
### DEC-005: Keep `Display` and `Debug` semantically equivalent through the box

The std blanket `impl<T: Display + ?Sized> Display for Box<T>`
forwards `Display` transparently; same for `Debug` and
`Error`. SPEC-0030 does not add a wrapper newtype around
`Box<ParseError>` (e.g., `pub struct BoxedParseError(Box<ParseError>)`)
because the wrapper would require manual `Display` / `Debug`
/ `std::error::Error` impls and provides no semantic
benefit. The raw `Box<ParseError>` is the public surface.
</decision>

<decision id="DEC-006" status="accepted">
### DEC-006: No CLI flag, no environment variable, no policy file

Per AGENTS.md Principle 5 ("stay small — no mode toggles, no
policy file") and Principle 1 ("feedback, not enforcement"),
SPEC-0030 introduces no configuration surface. The fix is
purely a type-system change in `speccy-core`; nothing in
the user-visible CLI behaviour changes.
</decision>

## Open questions

- [ ] Should the `ParseResult<T>` alias be exported from
      `speccy_core`'s prelude module (if a prelude exists) in
      addition to the crate-root re-export? Lean no — there's
      no prelude module today and adding one for one alias is
      premature. Revisit if a future SPEC introduces a
      prelude for other reasons.
- [ ] Should the implementer task that lands REQ-001 split
      `speccy-core/src/parse/` and `speccy-core/src/workspace.rs`
      into two passes (one PR each) or land them together?
      Lean together — the `?` propagation chains cross both
      modules, and a half-applied box at one boundary would
      cascade type errors into the other. The decomposition
      into separate Speccy tasks is fine; the commit shape is
      one-shot.
- [ ] After REQ-001 lands, is the
      `Option<Result<TasksDoc, Box<ParseError>>>` shape on
      `ParsedSpec` ergonomically painful enough for downstream
      consumers (status, lint, verify) to justify a follow-up
      that introduces a typed `ParsedTasks` newtype with its
      own success/failure accessors? Defer — wait for the
      first downstream consumer that complains; do not
      anticipate.

## Assumptions

<assumptions>
- The standard library's `impl<T> From<T> for Box<T>` is
  available on the project's MSRV (rust-version = "1.95" per
  `Cargo.toml` `[workspace.package]`). Verified: this impl
  has been in std since Rust 1.4 (2015); MSRV is well above
  that floor.
- `thiserror = "2.0"`'s `#[error("...")]` template
  rendering composes correctly through `Box<ParseError>`'s
  forwarding `Display`. Verified empirically by the existing
  `speccy-cli/src/plan.rs:74-83` precedent, which prints
  `PlanError::Parse { source: Box<ParseError> }` correctly
  today.
- No downstream consumer outside this repository depends on
  the signature of any function in
  `speccy-core/src/parse/*` or
  `speccy-core/src/workspace.rs`. Speccy is pre-v1.0; the
  `speccy-core` crate is not yet published to crates.io.
  Internal callers (`speccy-cli`, integration tests) update
  in lockstep with the producer surface.
- The `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` invocation in AGENTS.md's "Standard
  hygiene" block is the canonical gate. The local-fast variant
  `cargo clippy --workspace` (no `--all-features`) currently
  passes today because `result_large_err` lives in the
  `pedantic` group and is only escalated to `deny` via the
  `--all-features` / explicit lint set; the full hygiene
  gate is what fails today and what this SPEC fixes.
- The total `Result<T, ParseError>` byte size is dominated
  by the `Ok` variant for large `T` (e.g.,
  `Result<SpecDoc, ParseError>` is 352 bytes, of which
  `SpecDoc` itself contributes the majority). Boxing only
  the `Err` side does not shrink the `Ok` side; the
  `Result` envelope after this SPEC is still
  `max(size_of_T, 8) + niche/discriminant` bytes. That is
  intentional and expected — `result_large_err` lints
  specifically on the `Err` variant's byte count, not on
  the total `Result` size, and this SPEC's contract is with
  the lint, not with `Result` envelope size.
</assumptions>

## Changelog

<changelog>
| Date       | Reason                                       | Author |
|------------|----------------------------------------------|--------|
| 2026-05-18 | Initial draft. Box `ParseError` at every parser API boundary in `speccy-core` so `clippy::result_large_err` stops blocking the workspace hygiene gate. Closes the carry-forward from SPEC-0026 T-003 (echoed in SPEC-0027 and SPEC-0028 `## Assumptions`). | Kevin Xiao |
</changelog>
