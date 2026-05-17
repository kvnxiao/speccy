---
spec: SPEC-0001
spec_hash_at_generation: 1058b377f82fd0598437f483970b85dde96c5587e608e6fbf8ea32b47b19e545
generated_at: 2026-05-17T17:37:23Z
---

# Tasks: SPEC-0001 Artifact parsers

> `spec_hash_at_generation` is `bootstrap-pending` because this spec
> was decomposed manually before `speccy tasks --commit` (SPEC-0006)
> exists. Backfill the real sha256 the first time that command runs
> against this spec.

## Phase 1: Scaffolding

<tasks spec="SPEC-0001">

<task id="T-001" state="completed" covers="REQ-007">
Initialise Cargo workspace with `speccy` and `speccy-core` crates

- Suggested files: `Cargo.toml`, `speccy-cli/Cargo.toml`, `speccy-core/Cargo.toml`, `speccy-cli/src/main.rs`, `speccy-core/src/lib.rs`, `.gitignore`

<task-scenarios>
  - `cargo build --workspace` succeeds from a fresh clone.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
  - `speccy-core/src/lib.rs` has `#![deny(unsafe_code)]` at the top.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-007">
Pin parsing-stack dependencies and configure `cargo-deny`

- Suggested files: `speccy-core/Cargo.toml`, `deny.toml`


<task-scenarios>
  - `cargo deny check` passes (advisories, bans, licenses, sources).
  - `serde-saphyr` is pinned exactly to its chosen `0.0.x` patch in `speccy-core/Cargo.toml`.
  - `toml`, `comrak`, `regex`, `sha2` are pinned to current stable major.minor.
</task-scenarios>
</task>

## Phase 2: Frontmatter splitter


<task id="T-003" state="completed" covers="REQ-002">
Implement frontmatter splitter (DIY string slicing)

- Suggested files: `speccy-core/src/parse/frontmatter.rs`, `speccy-core/tests/frontmatter.rs`


<task-scenarios>
  - Valid `---\n<yaml>\n---\n<body>` returns `Some((yaml, body))`.
  - File without leading `---` returns `None`.
  - File with opening fence but no closing fence returns `ParseError::UnterminatedFrontmatter`.
  - CRLF line endings behave identically to LF.
  - Body containing a `---` horizontal rule is **not** treated as the closing fence; only the first `---`-on-its-own-line after the opening fence counts.
  - Empty frontmatter and empty body (`---\n---\n`) returns `Some(("", ""))`.
</task-scenarios>
</task>

## Phase 3: TOML parsers


<task id="T-004" state="completed" covers="REQ-001">
Implement `parse::speccy_toml` and `parse::spec_toml`

- Suggested files: `speccy-core/src/parse/toml.rs`, `speccy-core/tests/toml_parsers.rs`


<task-scenarios>
  - Valid `speccy.toml` round-trips into `SpeccyConfig`.
  - Valid `spec.toml` round-trips into `SpecToml`; `[[requirements]]` and `[[checks]]` preserve declared order.
  - `schema_version = 2` returns `ParseError::UnsupportedSchemaVersion { file, value: 2 }`.
  - A `[[checks]]` entry with neither `command` nor `prompt` returns an error naming the check ID.
  - A `[[checks]]` entry with both `command` and `prompt` returns an error naming the check ID and the conflict.
  - Missing required fields (e.g. `id`, `proves`) return an error naming the field.
</task-scenarios>
</task>

## Phase 4: Markdown parsers


<task id="T-005" state="completed" covers="REQ-003">
Implement `parse::spec_md` (frontmatter + REQ headings + Changelog + sha256)

- Suggested files: `speccy-core/src/parse/spec_md.rs`, `speccy-core/tests/spec_md_parser.rs`

<task-scenarios>
  - Frontmatter deserialises via `serde-saphyr` into `SpecFrontmatter { id, slug, title, status, created, supersedes }`. `supersedes` defaults to an empty `Vec` when omitted from the source. There is no `superseded_by` field on the struct.
  - REQ headings inside fenced code blocks are **not** extracted.
  - The `## Changelog` table (case-insensitive heading match) parses into `Vec<ChangelogRow>`.
  - Absent `## Changelog` heading yields an empty `changelog` vec.
  - `status` outside `{in-progress, implemented, dropped, superseded}` returns a parse error naming the value.
  - sha256 hash changes when any byte changes; identical content yields identical hash.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-004">
Implement `parse::tasks_md` (frontmatter + task state + notes)

- Suggested files: `speccy-core/src/parse/tasks_md.rs`, `speccy-core/tests/tasks_md_parser.rs`

<task-scenarios>
  - Frontmatter deserialises (spec, spec_hash_at_generation, generated_at).
  - Task state mapping: `[ ]` -> `Open`, `[~]` -> `InProgress`, `[?]` -> `AwaitingReview`, `[x]` -> `Done`.
  - Bold span `**T-NNN**` extracts task ID; malformed IDs are skipped and produce a recoverable warning on the parse result.
  - `Covers: REQ-001, REQ-002` -> `covers: ["REQ-001", "REQ-002"]`.
  - `` Suggested files: `a`, `b` `` -> `suggested_files: ["a", "b"]`.
  - Sub-list bullets under a task become `notes` in declared order.
  - Phase headings (`## Phase N: ...`) do not appear in parsed output.
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-005">
Implement `parse::report_md` (frontmatter only; body verbatim)

- Suggested files: `speccy-core/src/parse/report_md.rs`, `speccy-core/tests/report_md_parser.rs`


<task-scenarios>
  - Frontmatter deserialises (spec, outcome, generated_at).
  - `outcome` outside `{delivered, partial, abandoned}` returns a parse error naming the invalid value.
  - Missing `generated_at` returns a parse error naming the field.
  - Body is returned verbatim (no normalisation, no parsing).
</task-scenarios>
</task>

## Phase 5: Cross-reference and supersession graph


<task id="T-008" state="completed" covers="REQ-006">
Implement `parse::cross_ref` (SpecMd x SpecToml -> CrossRef)

- Suggested files: `speccy-core/src/parse/cross_ref.rs`, `speccy-core/tests/cross_ref.rs`

<task-scenarios>
  - Symmetric: `only_in_spec_md`, `only_in_toml`, `in_both` partition the union of REQ IDs.
  - Deterministic: order in each list matches declared order in the source.
  - Idempotent: calling twice on the same inputs returns equal results.
</task-scenarios>
</task>

<task id="T-009" state="completed" covers="REQ-008">
Implement `parse::supersession_index` (inverse `supersedes` across a workspace)

- Suggested files: `speccy-core/src/parse/supersession.rs`, `speccy-core/tests/supersession_index.rs`


<task-scenarios>
  - Given SPEC-0017 (no `supersedes`), SPEC-0042 (`supersedes: [SPEC-0017]`), and SPEC-0050 (`supersedes: [SPEC-0017, SPEC-0030]`), `index.superseded_by("SPEC-0017")` returns `["SPEC-0042", "SPEC-0050"]` in input order.
  - `index.dangling_references()` includes `"SPEC-0030"` for the same input.
  - Empty workspace returns an empty index without errors or panics.
  - Calling twice on the same input slice returns equal results.
</task-scenarios>
</task>

## Phase 6: API surface and hygiene


<task id="T-010" state="completed" covers="REQ-007">
Define and export `ParseError` enum and the public `parse` module path

- Suggested files: `speccy-core/src/error.rs`, `speccy-core/src/lib.rs`

<task-scenarios>
  - Each `ParseError` variant is reachable from at least one parser path via a unit test.
  - `ParseError` implements `std::error::Error + Send + Sync + 'static`.
  - Public re-exports are stable: `speccy_core::parse::{speccy_toml, spec_toml, spec_md, tasks_md, report_md, cross_ref}` all resolve.
</task-scenarios>
</task>

<task id="T-011" state="completed" covers="REQ-007">
Lock in CI hygiene gates

- Suggested files: `speccy-core/src/lib.rs`, project-root scripts or `xtask` if convenient (CI workflow wiring is deferred to a later spec; this task only ensures the gates pass locally).

<task-scenarios>
  - No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, or `unimplemented!` appears in `speccy-core/src/`. Verifiable via `grep` in CI. (Tests under `tests/` may use `.expect("descriptive message")`.)
  - `cargo +nightly fmt --all --check` is clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
</task-scenarios>
</task>

</tasks>
