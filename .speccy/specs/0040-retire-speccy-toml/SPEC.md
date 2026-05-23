---
id: SPEC-0040
slug: retire-speccy-toml
title: "Retire speccy.toml: drop scaffolding, parser, dependency, and TOML-side schema_version"
status: implemented
created: 2026-05-22
supersedes: []
---

# SPEC-0040: Retire `speccy.toml` — drop scaffolding, parser, dependency, and TOML-side `schema_version`

## Summary

`speccy init` scaffolds `.speccy/speccy.toml` with six lines of
content (`schema_version = 1` plus a `[project] name = "..."` block).
Nothing in the codebase reads the parsed value. The parser
(`speccy-core::parse::toml_files`) exists, the types
(`SpeccyConfig`, `ProjectConfig`) are exported, and a single test
round-trips the file — but no downstream consumer ever queries
`ProjectConfig::name` or any other field. Project root discovery
(`workspace::find_root`) walks up looking for the **`.speccy/`
directory**, not `speccy.toml`. The TOML file is dead weight.

This SPEC retires it. `speccy init` no longer writes
`.speccy/speccy.toml`; it writes `.speccy/.gitkeep` instead so
`find_root` keeps working between init and first spec. The parser
module, its types, the `SUPPORTED_SCHEMA_VERSION` constant, the
`guard_schema_version` helper, the
`ParseError::UnsupportedSchemaVersion` enum variant, and the
`speccy_toml` re-exports are all deleted. The CLI-side template
file, render function, plan-item construction, and unit tests go
with them. Every integration test that asserts on
`.speccy/speccy.toml` is removed or rewritten. The `toml` crate is
dropped from `speccy-core/Cargo.toml`'s explicit dependencies
(transitive consumers via other crates are acceptable). README.md
and `docs/ARCHITECTURE.md` lose every `speccy.toml` reference
and the entire TOML-side `schema_version` story.

This is a pre-v1 cleanup: no legacy installed base, no migration
shim, no deprecation period. Broader ARCHITECTURE.md drift cleanup
beyond the sections directly touched by this deletion is deferred
to a follow-up SPEC.

## Goals

<goals>
- `speccy init` produces a workspace where `find_root` can locate
  `.speccy/` without writing any TOML.
- No first-party Rust code parses, models, reads, writes, or
  re-exports the `speccy.toml` file or its `schema_version` field.
- No first-party test references the `.speccy/speccy.toml` path or
  content.
- `speccy-core/Cargo.toml` no longer declares the `toml` crate as an
  explicit dependency.
- README.md and `docs/ARCHITECTURE.md` contain no surviving
  `speccy.toml` references and no surviving `## Schema version`
  section after this SPEC ships.
- `speccy next` emits `next_action.kind = "work"` (not `"implement"`)
  in JSON and `"work"` in text output, aligning the CLI's vocabulary
  with the shipped orchestration skills' work/review/ship/decompose
  terminology.
- The standard hygiene suite (`cargo test --workspace`, clippy,
  `cargo +nightly fmt --all --check`, `cargo deny check`) passes
  after the deletion.
</goals>

## Non-goals

<non-goals>
- No changes to `SPC-001` the lint **rule** in
  `speccy-core/src/lint/rules/spc.rs`. The rule is the SPEC.md
  parse-failure catch-all (verified by ~10 dependent sites across
  tests, fixtures, and the verify command). Only the stale
  description text mentioning `speccy.toml` in
  `docs/ARCHITECTURE.md` changes.
- No changes to the CLI `--json` envelope `schema_version: 1` field
  on `status`, `next`, `vacancy`, or `verify`. That field is a
  distinct downstream-consumer contract (per AGENTS.md quality
  bar: "JSON breaks are versioned via `schema_version`") and stays.
- No broader restructure or rewrite of `docs/ARCHITECTURE.md`
  beyond the sections directly touched by this deletion plus stale
  prose immediately adjacent to those edits. A full
  ARCHITECTURE.md audit is deferred to a separate follow-up SPEC.
- No migration path, deprecation shim, or release-note ceremony.
  Pre-v1; breaking changes are expected.
- No changes to project-root discovery semantics — `find_root`
  continues to walk up looking for the `.speccy/` directory.
</non-goals>

## User Stories

<user-stories>
- As a maintainer of `speccy-core`, I want the parser surface to
  reflect only carriers that something actually reads, so new
  contributors are not misled into thinking `speccy.toml` content
  is consumed somewhere.
- As a user running `speccy init` for the first time, I want the
  scaffolded workspace to contain no files I have to either
  understand or delete; an empty marker directory is enough.
- As a CI maintainer, I want `cargo deny check` to flag fewer
  unused dependencies because the workspace declares only what it
  actually uses.
</user-stories>

## Assumptions

<assumptions>
- Project root discovery uses the **`.speccy/` directory's
  presence**, not the `speccy.toml` file, as the workspace marker.
  Verified in `speccy-core/src/workspace.rs::find_root`, which
  walks up parents checking `path.join(".speccy").is_dir()`.
- The `read_to_string` helper currently hosted in
  `speccy-core::parse::toml_files` has no consumers outside
  `speccy-core::parse::spec_md` and `speccy-core::workspace`.
  Grep-verified at SPEC-draft time.
- No shipped skill body, prompt template, or agent definition file
  under `resources/` reads `.speccy/speccy.toml` content. Grep
  across `resources/` returned zero matches at SPEC-draft time.
- The CLI `--json` envelope `schema_version` field
  (`status_output.rs`, `next_output.rs`, `vacancy.rs`,
  `verify_output.rs`) is a separate downstream-consumer contract
  from the TOML `schema_version` field and is intentionally out of
  scope for this SPEC.
- A future SPEC will brainstorm and draft the broader
  `docs/ARCHITECTURE.md` restructure as a docs-only follow-up.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: `speccy init` writes `.speccy/.gitkeep`, not `.speccy/speccy.toml`

`speccy init` no longer scaffolds a `.speccy/speccy.toml` file. To
preserve workspace-discovery behavior between init and the first
spec (since `find_root` walks up looking for the `.speccy/`
directory), `speccy init` writes a `.speccy/.gitkeep` placeholder.
The gitkeep lives at `.speccy/.gitkeep`, not under a child like
`.speccy/specs/.gitkeep`, because the discovery target is the
parent directory itself.

<done-when>
- After `speccy init` in a fresh directory, `.speccy/.gitkeep`
  exists.
- After `speccy init` in a fresh directory, `.speccy/speccy.toml`
  does **not** exist.
- After `speccy init`, `speccy status` (or any command that
  resolves project root) succeeds from inside the project.
</done-when>

<behavior>
- Given a fresh repo at `/foo/bar` with no `.speccy/`, when
  `speccy init` runs, then `.speccy/.gitkeep` is created and
  `.speccy/speccy.toml` is not.
- Given a workspace freshly scaffolded by `speccy init` and
  containing no specs, when `speccy status` runs from any directory
  at or below the project root, then it succeeds (returns the
  empty-workspace text/JSON output) rather than failing with
  "`.speccy/` directory not found".
- Given a workspace freshly scaffolded by `speccy init`, when the
  user inspects the working tree, then no file named
  `speccy.toml` exists anywhere under `.speccy/`.
</behavior>

<scenario id="CHK-001">
Given a built `speccy` binary at HEAD after this SPEC lands,
when `speccy init` runs in a fresh temp directory,
then `.speccy/.gitkeep` exists and `.speccy/speccy.toml` does not
exist.
</scenario>

<scenario id="CHK-002">
Given a temp directory in which `speccy init` has just run,
when `speccy status` runs from that directory,
then the command exits 0 (workspace discovery via `find_root`
succeeds against the `.speccy/` marker directory).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `speccy-core::parse::toml_files` module is removed in full

The `speccy-core::parse::toml_files` module is deleted, including
every public and private symbol it owns: the `speccy_toml`
parser function, the `SpeccyConfig` and `ProjectConfig` types
(plus their `Raw*` private mirrors), the `SUPPORTED_SCHEMA_VERSION`
constant, the `guard_schema_version` helper, and the in-module
`#[cfg(test)]` test suite. The `ParseError::UnsupportedSchemaVersion`
enum variant in `speccy-core::error` is also removed because its
sole emitter is the deleted parser. The `speccy_toml`,
`SpeccyConfig`, and `ProjectConfig` re-exports under
`speccy-core::parse::mod` are removed.

<done-when>
- `speccy-core/src/parse/toml_files.rs` does not exist.
- `speccy-core::parse::mod` does not re-export `speccy_toml`,
  `SpeccyConfig`, or `ProjectConfig`.
- `speccy-core::error::ParseError::UnsupportedSchemaVersion` is not
  declared.
- A workspace-wide ripgrep for `toml_files`, `SpeccyConfig`,
  `speccy_toml`, `SUPPORTED_SCHEMA_VERSION`, `guard_schema_version`,
  or `UnsupportedSchemaVersion` (case-sensitive, scoped to `*.rs`)
  returns zero hits.
- `cargo check --workspace` passes.
</done-when>

<behavior>
- Given the speccy workspace at HEAD after this SPEC lands, when
  `cargo check --workspace` runs, then it succeeds without
  referencing the deleted module.
- Given a downstream consumer that imported
  `speccy_core::parse::SpeccyConfig`, when it tries to compile
  against HEAD, then it fails with an unresolved-import error
  (acceptable pre-v1).
</behavior>

<scenario id="CHK-003">
Given the speccy workspace at HEAD after this SPEC lands,
when `cargo check --workspace --all-targets --all-features` runs,
then it exits 0.
</scenario>

<scenario id="CHK-004">
Given the speccy workspace at HEAD,
when ripgrep searches `*.rs` files for any of `toml_files`,
`SpeccyConfig`, `ProjectConfig`, `speccy_toml`,
`SUPPORTED_SCHEMA_VERSION`, `guard_schema_version`,
`UnsupportedSchemaVersion` (case-sensitive),
then the search returns zero matching lines.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Shared `read_to_string` helper relocated so `spec_md` and `workspace` keep compiling

The `read_to_string` helper that currently lives inside
`speccy-core::parse::toml_files` (a thin wrapper over `fs_err`
that wraps errors as `ParseError::Io`) is called by
`speccy-core::parse::spec_md` and three callsites in
`speccy-core::workspace`. When `parse::toml_files` is deleted, the
helper must survive at a location both callers can import.
Implementer chooses the destination; the contract is only that the
behavior is preserved and both callers continue to compile against
their existing call sites (with the import path updated).

<done-when>
- `speccy-core::parse::spec_md` continues to read SPEC.md files via
  a `read_to_string`-equivalent helper that wraps I/O errors as
  `ParseError::Io { path, source }`.
- `speccy-core::workspace` continues to read TASKS.md, REPORT.md,
  and SPEC.md files via the same helper.
- `cargo test --workspace` passes (proves both callers function
  end-to-end against real files).
</done-when>

<behavior>
- Given a SPEC.md file at a path that does not exist, when
  `parse::spec_md::spec_md` runs, then it returns
  `ParseError::Io { path, source }` carrying the missing path.
- Given a TASKS.md file at a path that does not exist, when the
  workspace loader tries to parse it, then the parse result
  surfaces `ParseError::Io { path, source }` carrying the missing
  path.
</behavior>

<scenario id="CHK-005">
Given the speccy workspace at HEAD after this SPEC lands,
when `cargo test --workspace` runs,
then it exits 0 (the SPEC.md and TASKS.md / REPORT.md parse paths
both function via the relocated helper).
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `speccy-cli` init code path no longer constructs, classifies, renders, or writes `.speccy/speccy.toml`

`speccy-cli/src/init.rs` drops every symbol that participates in
producing `.speccy/speccy.toml`: the `SPECCY_TOML_TEMPLATE`
top-level `const`, the `render_speccy_toml` function, the
`speccy_toml_path` / `speccy_toml_body` / `speccy_toml_action`
local bindings inside `build_plan`, the corresponding `PlanItem`
push, and the two unit tests `render_speccy_toml_substitutes_name`
and `render_speccy_toml_escapes_quotes`. The template file
`speccy-cli/src/templates/speccy.toml.tmpl` is deleted.

<done-when>
- `speccy-cli/src/templates/speccy.toml.tmpl` does not exist.
- `speccy-cli/src/init.rs` contains no `SPECCY_TOML_TEMPLATE`
  constant and no `render_speccy_toml` function.
- `speccy-cli/src/init.rs::build_plan` does not push a
  `PlanItem` whose destination ends in `speccy.toml`.
- A ripgrep for `speccy.toml` (case-sensitive) scoped to
  `speccy-cli/src/` returns zero hits.
- `cargo check --workspace` passes.
</done-when>

<behavior>
- Given the speccy CLI source at HEAD after this SPEC lands, when
  `cargo check --workspace --all-targets` runs, then it compiles
  without referencing the deleted symbols.
- Given a built `speccy` binary at HEAD, when `speccy init` runs
  and the plan is printed, then no plan line names `speccy.toml`
  as a destination.
</behavior>

<scenario id="CHK-006">
Given the speccy workspace at HEAD,
when ripgrep searches `speccy-cli/src/` (case-sensitive) for the
literal `speccy.toml`,
then the search returns zero matches.
</scenario>

<scenario id="CHK-007">
Given a built `speccy` binary at HEAD,
when `speccy init` runs in a fresh temp directory and the printed
plan is captured,
then no line in the plan output contains the substring
`speccy.toml`.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: No first-party test references `.speccy/speccy.toml` path or content

Every test that asserts on the path or content of
`.speccy/speccy.toml` is removed or rewritten:

- `speccy-cli/tests/init.rs::scaffold_speccy_toml` is deleted in
  full.
- `speccy-core/tests/workspace_loader.rs::workspace_speccy_toml_still_parses`
  is deleted in full.
- `speccy-cli/tests/init.rs::refuse_without_force` is rewritten to
  use a different shipped file as the conflict trigger (e.g. a
  `.claude/skills/speccy-init/SKILL.md` byte mismatch).
- `speccy-cli/tests/init.rs::force_overwrites_shipped_files` is
  rewritten to drop the `speccy.toml` leg; the test continues to
  cover the `--force` semantics via the remaining shipped file
  (e.g. the host-pack SKILL.md leg already present).
- `speccy-cli/tests/init.rs` exit-code test: the `exit-one-conflict`
  sub-case is rewritten with a different shipped file as the
  conflict trigger.
- Any other test reference to `speccy.toml` discovered during
  implementation is removed or rewritten.

A new test (or extended existing test) asserts the REQ-001
gitkeep behavior end-to-end.

<done-when>
- A ripgrep for `speccy.toml` (case-sensitive) scoped to
  `speccy-cli/tests/` and `speccy-core/tests/` returns zero hits.
- `cargo test --workspace --all-features` passes.
- At least one integration test verifies that
  `speccy init` writes `.speccy/.gitkeep` (covers CHK-001).
</done-when>

<behavior>
- Given the speccy workspace at HEAD after this SPEC lands, when
  the full test suite runs, then every previously-existing
  invariant about init's refusal/overwrite/exit-code semantics
  still passes (via the rewritten trigger files).
- Given the same workspace, when the new gitkeep assertion runs,
  then it confirms `.speccy/.gitkeep` is created on a clean init.
</behavior>

<scenario id="CHK-008">
Given the speccy workspace at HEAD,
when ripgrep searches `speccy-cli/tests/` and `speccy-core/tests/`
(case-sensitive) for the literal `speccy.toml`,
then the search returns zero matches.
</scenario>

<scenario id="CHK-009">
Given the speccy workspace at HEAD,
when `cargo test --workspace --all-features` runs,
then it exits 0.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: `toml` crate removed from `speccy-core/Cargo.toml` explicit dependencies

The `toml` crate is removed from `speccy-core/Cargo.toml`'s
`[dependencies]` table (it was used only by the deleted
`parse::toml_files` module). Transitive `toml` reaching
`speccy-core` via another dep is acceptable and is **not**
hunted down — only the explicit first-party declaration is
removed.

<done-when>
- `speccy-core/Cargo.toml` contains no `^toml\s*=` line in any
  `[dependencies]` or `[dev-dependencies]` table.
- `cargo build --workspace` passes.
- `cargo deny check` passes (no new advisories or duplicate
  warnings introduced).
</done-when>

<behavior>
- Given the speccy workspace at HEAD, when `cargo tree -p
  speccy-core` runs, then `toml` either does not appear or
  appears only as a transitive dep beneath another crate (not as
  a direct child of `speccy-core`).
</behavior>

<scenario id="CHK-010">
Given the speccy workspace at HEAD,
when ripgrep searches `speccy-core/Cargo.toml` for a line matching
the regex `^toml\s*=`,
then the search returns zero matches.
</scenario>

<scenario id="CHK-011">
Given the speccy workspace at HEAD,
when `cargo build --workspace` runs,
then it exits 0.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: README.md and `docs/ARCHITECTURE.md` lose every `speccy.toml` reference and the TOML-side `schema_version` story

README.md and `docs/ARCHITECTURE.md` are edited to remove
every surviving mention of `speccy.toml` and the TOML-side
`schema_version` concept. The specific edits are:

- **README.md**:
  - The "Scaffolds `.speccy/speccy.toml` and the `.speccy/specs/`
    skeleton" bullet under the "Step 1: Scaffold the workspace"
    section is rewritten or removed.
  - The line `speccy.toml                   Minimal project
    config (just schema_version + name)` in the "Repo layout
    after `speccy init`" diagram is removed.
  - The closing paragraph "The only TOML left in the layout is
    the workspace-level `.speccy/speccy.toml`" is rewritten or
    removed.

- **`docs/ARCHITECTURE.md`**:
  - The `## speccy.toml` section (TOML body example plus
    surrounding prose) is removed in full.
  - The `## Schema version` section (which references
    `.speccy/speccy.toml`'s `schema_version`) is removed in full.
  - The `speccy.toml` line in the repo-layout diagram (under
    `.speccy/`) is removed.
  - The `SPC-001` lint-catalogue row's parenthetical mention of
    `speccy.toml` (the "stray per-spec spec.toml … the only TOML
    carried at spec level today is the workspace
    `.speccy/speccy.toml`" wording) is trimmed so the row reflects
    SPC-001's actual rule (SPEC.md parse-failure catch-all).
  - The implementation-sequence bullet that names `speccy.toml`
    as one of the parsed artifacts is rewritten.
  - Stale prose immediately adjacent to these edits (within the
    same section) is fixed in place. A broader
    `docs/ARCHITECTURE.md` audit is **out of scope** and is
    captured as a follow-up SPEC.

<done-when>
- A case-sensitive ripgrep for `speccy.toml` across `README.md` and
  `docs/ARCHITECTURE.md` returns zero matches.
- A case-sensitive ripgrep for the heading `## Schema version`
  across `docs/ARCHITECTURE.md` returns zero matches.
- A case-sensitive ripgrep for the heading `## speccy.toml`
  across `docs/ARCHITECTURE.md` returns zero matches.
- The SPC-001 row in the ARCHITECTURE.md lint catalogue describes
  the rule's current behavior (SPEC.md parse-failure catch-all)
  rather than `spec.toml` / `speccy.toml`.
</done-when>

<behavior>
- Given the speccy repo at HEAD after this SPEC lands, when a
  reader skims `README.md` end-to-end, then no mention of
  `speccy.toml` appears.
- Given the same repo, when a reader skims
  `docs/ARCHITECTURE.md` for the lint catalogue and finds the
  `SPC-001` row, then the row describes the SPEC.md parse-failure
  surface (not a stray TOML file).
</behavior>

<scenario id="CHK-012">
Given the speccy repo at HEAD,
when ripgrep searches `README.md` and `docs/ARCHITECTURE.md`
(case-sensitive) for the literal `speccy.toml`,
then the search returns zero matches.
</scenario>

<scenario id="CHK-013">
Given the speccy repo at HEAD,
when ripgrep searches `docs/ARCHITECTURE.md` for the literal
heading strings `## speccy.toml` and `## Schema version`,
then the search returns zero matches.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: `speccy next` emits `kind = "work"` instead of `kind = "implement"`

The `NextAction::Implement` enum variant in
`speccy-core::next` is renamed to `NextAction::Work`. The CLI's
JSON discriminator string in `speccy-cli::next_output::to_json_action`
flips from `"implement"` to `"work"`, and the text renderer in
`render_text_per_spec` / `render_text_workspace` flips its
keyword the same way. Every first-party test assertion that
matched on `"implement"` (as a JSON `kind` field, as the
per-spec text token, or as a workspace-row token) is updated to
match `"work"` instead. Every shipped skill body, agent
definition, and prompt module under `.claude/`, `.codex/`, and
`resources/modules/` that quotes `next_action.kind == "implement"`
is updated to quote `"work"`. The shipped lint catalogue in
`docs/ARCHITECTURE.md` is updated only where it quotes the
JSON discriminator literally; surrounding prose that uses the
English word "implement" as a verb stays untouched. Historical
SPEC artifacts (prior `SPEC.md`, `TASKS.md`, `REPORT.md`,
journal, and evidence files under `.speccy/specs/00**`) are
left as written — they describe what was true at the time and
are not retroactively rewritten.

<done-when>
- `NextAction::Work { task_id }` is declared in
  `speccy-core/src/next.rs`; `NextAction::Implement` does not
  exist.
- `speccy next --json` emits `"kind": "work"` for a pending-task
  spec; it never emits `"kind": "implement"`.
- `speccy next` (text form) emits the token `work` (per-spec
  form: `SPEC-NNNN: work T-NNN`; workspace form: the same token
  in the per-row output).
- A ripgrep for `"implement"` (case-sensitive, scoped to
  `speccy-core/src/`, `speccy-cli/src/`, and `speccy-cli/tests/`)
  returns zero hits as a JSON-literal or text-keyword token.
- A ripgrep for `next_action.kind == "implement"` (or the
  unquoted JSON-tag form `kind: implement` / `"implement"` as a
  next-action discriminator) across `.claude/`, `.codex/`, and
  `resources/modules/` returns zero hits.
- The full hygiene suite (`cargo test --workspace`, clippy,
  `cargo +nightly fmt --all --check`, `cargo deny check`)
  passes.
</done-when>

<behavior>
- Given the speccy workspace at HEAD after this SPEC lands, when
  `speccy next --json` runs against a spec with a pending task,
  then the emitted JSON contains
  `"next_action": { "kind": "work", "task_id": "T-NNN" }`.
- Given the same workspace, when `speccy next SPEC-NNNN` (text
  form) runs against a spec with a pending task, then the
  emitted line ends with the token `work T-NNN`.
- Given a shipped orchestration skill body that dispatches on
  `next_action.kind`, when a maintainer searches it for the
  string `"implement"`, then the only hit (if any) is the
  English verb in prose, not a discriminator quote.
</behavior>

<scenario id="CHK-014">
Given the speccy workspace at HEAD,
when ripgrep searches `speccy-core/src/` and `speccy-cli/src/`
(case-sensitive) for the literal `"implement"`,
then the search returns zero matches.
</scenario>

<scenario id="CHK-015">
Given the speccy workspace at HEAD,
when `cargo test --workspace --all-features` runs,
then it exits 0 (all renamed-keyword assertions pass).
</scenario>

<scenario id="CHK-016">
Given a built `speccy` binary at HEAD and a workspace with a
spec whose next pending task is `T-NNN`,
when `speccy next --json` runs,
then the emitted JSON contains the literal substring
`"kind":"work"` (or `"kind": "work"` after pretty-printing) and
does not contain `"kind":"implement"`.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
The gitkeep file lives at `.speccy/.gitkeep`, not
`.speccy/specs/.gitkeep`. The workspace-discovery target is the
parent `.speccy/` directory (per `workspace::find_root`'s walk-up
loop checking `path.join(".speccy").is_dir()`); preserving that
directory's existence is sufficient. Placing the gitkeep deeper
under `specs/` would over-commit the layout — `init` would imply
that the `specs/` subdirectory is part of the post-init contract,
whereas in practice `specs/` is created lazily by individual spec
authoring (`/speccy-plan`).
</decision>

<decision id="DEC-002">
`docs/ARCHITECTURE.md` cleanup is scoped narrowly to the
sections directly touched by the `speccy.toml` deletion plus
stale prose immediately adjacent inside the same section. A
broader `ARCHITECTURE.md` restructure (verifying every non-trivial
claim against the current code, fixing all drift, and potentially
reorganizing the document) is deferred to a separate follow-up
SPEC. Reason: those two concerns have different blast radii
(code surgery vs docs-only), different reviewer fan-out (code
review vs docs review), and folding both into one SPEC would
tangle the review verdicts. The follow-up SPEC will be
brainstormed and drafted as a docs-only successor after this one
lands.
</decision>

<decision id="DEC-003">
The `ParseError::UnsupportedSchemaVersion` enum variant is removed
along with the module that emits it. Pre-v1, the
`speccy-core::error::ParseError` enum is not a stable contract;
removing the variant is a straightforward dead-code deletion
rather than an enum-evolution exercise. Downstream consumers
that imported the variant get a compile error, which is the
expected pre-v1 breakage.
</decision>

## Notes

**Rejected alternative framings:**

- **(B) Parser-only deletion, file stays scaffolded.** Stop reading
  `speccy.toml` but keep `speccy init` writing it. Rejected: leaves
  a six-line dead artifact in every user's repo. Contradicts the
  retire framing; pre-v1, there is no installed-base argument for
  the half-measure.

- **(C) Repurpose, don't retire.** Give `speccy.toml` a real reason
  to exist (persona overrides, configurable defaults, etc.).
  Rejected: scope creep. AGENTS.md is already the canonical home
  for project conventions per `docs/ARCHITECTURE.md`'s own line:
  *"If the CLI ever needs structured access to environment
  metadata, the block will come back with a real purpose; until
  then, it isn't here."*

**Out-of-scope follow-ups:**

- A broader `docs/ARCHITECTURE.md` restructure (per DEC-002)
  becomes a future docs-only SPEC drafted after this one lands.
- Hunting transitive `toml` consumers in the dependency graph (per
  REQ-006) is not pursued in this SPEC.
- The CLI `--json` envelope `schema_version: 1` field is a distinct
  downstream-consumer contract and is unchanged here.

## Open Questions

(None remaining. All four original brainstorm open questions were
resolved before this SPEC was drafted: gitkeep location is
`.speccy/.gitkeep`; `## Schema version` is deleted outright;
`SPC-001` description is trimmed and the rule stays; pre-v1 means
no migration ceremony; transitive `toml` is acceptable.)

## Changelog

<changelog>
| Date       | Reason                                                   | Author     |
|------------|----------------------------------------------------------|------------|
| 2026-05-22 | Initial draft. Retire `.speccy/speccy.toml`: drop the scaffolded file from `speccy init` (replacing it with `.speccy/.gitkeep` to preserve workspace-discovery), delete the `speccy-core::parse::toml_files` module in full (parser, types, `SUPPORTED_SCHEMA_VERSION`, `guard_schema_version`, `ParseError::UnsupportedSchemaVersion`, re-exports), relocate the shared `read_to_string` helper, delete the CLI template file and rendering code, rewrite every test that references `.speccy/speccy.toml`, drop the explicit `toml` dependency from `speccy-core/Cargo.toml`, and remove every `speccy.toml` reference and the TOML-side `## Schema version` story from `README.md` and `docs/ARCHITECTURE.md`. SPC-001 the lint rule and the CLI `--json` envelope `schema_version: 1` contract are unchanged. Broader `docs/ARCHITECTURE.md` restructure is deferred to a follow-up SPEC. Pre-v1; no migration shim. | Kevin Xiao |
| 2026-05-22 | Add REQ-008 mid-loop: rename `next_action.kind` from `"implement"` to `"work"` in the CLI's JSON and text output (with matching `NextAction::Work` enum rename), and update every first-party test, shipped skill body, agent definition, and ARCHITECTURE.md quote that referenced the old discriminator. Folded into SPEC-0040 because the orchestrator skill already speaks the work/review/ship/decompose vocabulary; the CLI's `"implement"` tag was the odd one out, and shipping the rename alongside the `speccy.toml` retirement keeps the "drop dead vocabulary" theme coherent. Historical SPEC artifacts are not retroactively edited. Pre-v1; no JSON-envelope schema bump. | Kevin Xiao |
</changelog>
