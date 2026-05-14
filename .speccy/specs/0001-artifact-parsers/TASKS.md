---
spec: SPEC-0001
spec_hash_at_generation: 9cf2144b2d6b3221f9eb78a54b88508485ce74ad7017e173986109bd1829a626
generated_at: 2026-05-14T03:25:12Z
---

# Tasks: SPEC-0001 Artifact parsers

> `spec_hash_at_generation` is `bootstrap-pending` because this spec
> was decomposed manually before `speccy tasks --commit` (SPEC-0006)
> exists. Backfill the real sha256 the first time that command runs
> against this spec.

## Phase 1: Scaffolding

- [x] **T-001**: Initialise Cargo workspace with `speccy` and `speccy-core` crates
  - Covers: REQ-007
  - Tests to write:
    - `cargo build --workspace` succeeds from a fresh clone.
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
    - `speccy-core/src/lib.rs` has `#![deny(unsafe_code)]` at the top.
  - Suggested files: `Cargo.toml`, `speccy-cli/Cargo.toml`, `speccy-core/Cargo.toml`, `speccy-cli/src/main.rs`, `speccy-core/src/lib.rs`, `.gitignore`

- [x] **T-002**: Pin parsing-stack dependencies and configure `cargo-deny`
  - Covers: REQ-007
  - Tests to write:
    - `cargo deny check` passes (advisories, bans, licenses, sources).
    - `serde-saphyr` is pinned exactly to its chosen `0.0.x` patch in `speccy-core/Cargo.toml`.
    - `toml`, `comrak`, `regex`, `sha2` are pinned to current stable major.minor.
  - Suggested files: `speccy-core/Cargo.toml`, `deny.toml`

## Phase 2: Frontmatter splitter

- [x] **T-003**: Implement frontmatter splitter (DIY string slicing)
  - Covers: REQ-002
  - Tests to write:
    - Valid `---\n<yaml>\n---\n<body>` returns `Some((yaml, body))`.
    - File without leading `---` returns `None`.
    - File with opening fence but no closing fence returns `ParseError::UnterminatedFrontmatter`.
    - CRLF line endings behave identically to LF.
    - Body containing a `---` horizontal rule is **not** treated as the closing fence; only the first `---`-on-its-own-line after the opening fence counts.
    - Empty frontmatter and empty body (`---\n---\n`) returns `Some(("", ""))`.
  - Suggested files: `speccy-core/src/parse/frontmatter.rs`, `speccy-core/tests/frontmatter.rs`

## Phase 3: TOML parsers

- [x] **T-004**: Implement `parse::speccy_toml` and `parse::spec_toml`
  - Covers: REQ-001
  - Tests to write:
    - Valid `speccy.toml` round-trips into `SpeccyConfig`.
    - Valid `spec.toml` round-trips into `SpecToml`; `[[requirements]]` and `[[checks]]` preserve declared order.
    - `schema_version = 2` returns `ParseError::UnsupportedSchemaVersion { file, value: 2 }`.
    - A `[[checks]]` entry with neither `command` nor `prompt` returns an error naming the check ID.
    - A `[[checks]]` entry with both `command` and `prompt` returns an error naming the check ID and the conflict.
    - Missing required fields (e.g. `id`, `proves`) return an error naming the field.
  - Suggested files: `speccy-core/src/parse/toml.rs`, `speccy-core/tests/toml_parsers.rs`

## Phase 4: Markdown parsers

- [x] **T-005**: Implement `parse::spec_md` (frontmatter + REQ headings + Changelog + sha256)
  - Covers: REQ-003
  - Tests to write:
    - Frontmatter deserialises via `serde-saphyr` into `SpecFrontmatter { id, slug, title, status, created, supersedes }`. `supersedes` defaults to an empty `Vec` when omitted from the source. There is no `superseded_by` field on the struct.
    - REQ headings inside fenced code blocks are **not** extracted.
    - The `## Changelog` table (case-insensitive heading match) parses into `Vec<ChangelogRow>`.
    - Absent `## Changelog` heading yields an empty `changelog` vec.
    - `status` outside `{in-progress, implemented, dropped, superseded}` returns a parse error naming the value.
    - sha256 hash changes when any byte changes; identical content yields identical hash.
  - Suggested files: `speccy-core/src/parse/spec_md.rs`, `speccy-core/tests/spec_md_parser.rs`

- [x] **T-006**: Implement `parse::tasks_md` (frontmatter + task state + notes)
  - Covers: REQ-004
  - Tests to write:
    - Frontmatter deserialises (spec, spec_hash_at_generation, generated_at).
    - Task state mapping: `[ ]` -> `Open`, `[~]` -> `InProgress`, `[?]` -> `AwaitingReview`, `[x]` -> `Done`.
    - Bold span `**T-NNN**` extracts task ID; malformed IDs are skipped and produce a recoverable warning on the parse result.
    - `Covers: REQ-001, REQ-002` -> `covers: ["REQ-001", "REQ-002"]`.
    - `` Suggested files: `a`, `b` `` -> `suggested_files: ["a", "b"]`.
    - Sub-list bullets under a task become `notes` in declared order.
    - Phase headings (`## Phase N: ...`) do not appear in parsed output.
  - Suggested files: `speccy-core/src/parse/tasks_md.rs`, `speccy-core/tests/tasks_md_parser.rs`

- [x] **T-007**: Implement `parse::report_md` (frontmatter only; body verbatim)
  - Covers: REQ-005
  - Tests to write:
    - Frontmatter deserialises (spec, outcome, generated_at).
    - `outcome` outside `{delivered, partial, abandoned}` returns a parse error naming the invalid value.
    - Missing `generated_at` returns a parse error naming the field.
    - Body is returned verbatim (no normalisation, no parsing).
  - Suggested files: `speccy-core/src/parse/report_md.rs`, `speccy-core/tests/report_md_parser.rs`

## Phase 5: Cross-reference and supersession graph

- [x] **T-008**: Implement `parse::cross_ref` (SpecMd x SpecToml -> CrossRef)
  - Covers: REQ-006
  - Tests to write:
    - Symmetric: `only_in_spec_md`, `only_in_toml`, `in_both` partition the union of REQ IDs.
    - Deterministic: order in each list matches declared order in the source.
    - Idempotent: calling twice on the same inputs returns equal results.
  - Suggested files: `speccy-core/src/parse/cross_ref.rs`, `speccy-core/tests/cross_ref.rs`

- [x] **T-009**: Implement `parse::supersession_index` (inverse `supersedes` across a workspace)
  - Covers: REQ-008
  - Tests to write:
    - Given SPEC-0017 (no `supersedes`), SPEC-0042 (`supersedes: [SPEC-0017]`), and SPEC-0050 (`supersedes: [SPEC-0017, SPEC-0030]`), `index.superseded_by("SPEC-0017")` returns `["SPEC-0042", "SPEC-0050"]` in input order.
    - `index.dangling_references()` includes `"SPEC-0030"` for the same input.
    - Empty workspace returns an empty index without errors or panics.
    - Calling twice on the same input slice returns equal results.
  - Suggested files: `speccy-core/src/parse/supersession.rs`, `speccy-core/tests/supersession_index.rs`

## Phase 6: API surface and hygiene

- [x] **T-010**: Define and export `ParseError` enum and the public `parse` module path
  - Covers: REQ-007
  - Tests to write:
    - Each `ParseError` variant is reachable from at least one parser path via a unit test.
    - `ParseError` implements `std::error::Error + Send + Sync + 'static`.
    - Public re-exports are stable: `speccy_core::parse::{speccy_toml, spec_toml, spec_md, tasks_md, report_md, cross_ref}` all resolve.
  - Suggested files: `speccy-core/src/error.rs`, `speccy-core/src/lib.rs`

- [x] **T-011**: Lock in CI hygiene gates
  - Covers: REQ-007
  - Tests to write:
    - No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, or `unimplemented!` appears in `speccy-core/src/`. Verifiable via `grep` in CI. (Tests under `tests/` may use `.expect("descriptive message")`.)
    - `cargo +nightly fmt --all --check` is clean.
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
  - Suggested files: `speccy-core/src/lib.rs`, project-root scripts or `xtask` if convenient (CI workflow wiring is deferred to a later spec; this task only ensures the gates pass locally).
