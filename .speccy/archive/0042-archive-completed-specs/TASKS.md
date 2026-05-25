---
spec: SPEC-0042
spec_hash_at_generation: e877be5a1c56a970882a36028953baf6dbffeed6835fdd044e050424e046bd67
generated_at: 2026-05-23T17:22:14Z
---
# Tasks: SPEC-0042 Archive completed specs — `speccy archive SPEC-NNNN` relocates shipped/dropped/superseded specs out of the active set

<task id="T-001" state="completed" covers="REQ-003 REQ-004">
## Extend SPEC.md frontmatter with `archived_at` / `archived_reason` and exclude them from the spec hash

In `speccy-core/src/parse/spec_md.rs`:

- Extend `SpecFrontmatter` with two new optional fields:
  - `archived_at: Option<jiff::civil::Date>` — parsed from the
    `archived_at: YYYY-MM-DD` YAML key.
  - `archived_reason: Option<String>` — parsed from the
    `archived_reason: "..."` YAML key.
- Add both keys to `HASH_EXCLUDED_FRONTMATTER_FIELDS` so
  `canonical_frontmatter_for_hash` skips them when feeding bytes to
  `Sha256`. The existing `status` exclusion stays.
- Update the doc comment on `SpecMd.sha256` (line 50 in current
  HEAD) to list `archived_at` and `archived_reason` alongside
  `status` as fields excluded from the hash; the rationale paragraph
  references SPEC-0042 DEC-001.
- Update the YAML emit path in `canonical_frontmatter_for_hash`
  (the `push_kv` block, lines 274+ in current HEAD) to keep its
  alphabetical-key ordering — the new fields are excluded so the
  exact emission order does not matter, but documentation should
  still note them.
- The two new keys, when present in source YAML, parse without
  error. When absent, both `Option` fields are `None`. The fields
  do not affect the spec status gate (`status` parsing is unchanged).

In `speccy-core/src/parse/spec_md.rs` tests:

- Add `sha256_invariant_under_archive_fields_addition`: parse a
  fixture without the archive fields, parse a second fixture
  identical except for `archived_at: 2026-05-23` and
  `archived_reason: "test"` appended after `supersedes:`, assert
  the two `SpecMd.sha256` values are byte-equal.
- Extend `hash_excluded_frontmatter_fields_contains_only_status`
  (current test name) and rename to
  `hash_excluded_frontmatter_fields_set` (or equivalent),
  asserting the exclusion set is exactly
  `{"status", "archived_at", "archived_reason"}`.
- Add `archive_fields_parse_when_present`: parse a fixture with
  both archive fields populated, assert the parsed
  `SpecFrontmatter` carries the expected `Some(...)` values.
- Add `archive_fields_absent_when_missing`: parse a fixture with
  no archive fields, assert both `Option` fields are `None`.

In `speccy-cli/src/status.rs` (and any JSON serialization path
that emits frontmatter into the `--json` output):

- Extend the JSON entry for a spec to include `archived_at` and
  `archived_reason` keys when the underlying `SpecFrontmatter`
  carries them. The keys are omitted when the fields are `None`,
  not serialized as JSON `null` (keep the existing convention of
  omitting absent optional fields).
- For non-archived specs (the only kind that exists today), the
  JSON entries remain byte-identical to pre-SPEC-0042 output.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a SPEC.md fixture with frontmatter declaring `id`, `slug`,
`title`, `status: implemented`, `created`, `supersedes: []` and no
archive fields,
when `SpecMd::parse_raw` returns `SpecMd { sha256: H1, .. }`,
and a second fixture identical except for two extra lines
`archived_at: 2026-05-23` and `archived_reason: "test"` appended
between `supersedes:` and the closing `---`,
when `SpecMd::parse_raw` returns `SpecMd { sha256: H2, .. }`,
then `H1 == H2` byte-equal (covers CHK-010).

Given the first fixture and a third fixture identical except that
one ASCII byte of the prose body has changed,
when both are parsed,
then their `sha256` values differ (covers CHK-011; regression
guard that body bytes are still hashed).

Given a SPEC.md fixture with `archived_at: 2026-05-23` and
`archived_reason: "shipped 2025-12-15"` in its frontmatter,
when `SpecMd::parse_raw` returns the parsed `SpecFrontmatter`,
then `archived_at == Some(date!(2026-05-23))` and
`archived_reason == Some("shipped 2025-12-15".to_string())`.

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-core/src/parse/spec_md.rs`,
`speccy-cli/src/status.rs`,
`speccy-cli/tests/status_json.rs` (or equivalent),
`speccy-core/tests/spec_md_parsing.rs` (or wherever the existing
spec-md tests live).
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-002 REQ-003">
## Introduce `speccy archive SPEC-NNNN`: subcommand, status gate, frontmatter mutation, `git mv`

In `speccy-cli/src/main.rs` (or the clap command tree wherever it
lives at HEAD):

- Add a new `Archive` subcommand to the `Command` enum with:
  - Positional `spec_id: String` (parsed as `SpecId` after the
    existing convention).
  - `--reason <STRING>` optional, single-line scalar. Reject newline
    characters at clap value-parse time (custom value parser).
  - `--force` boolean flag.
  - `--json` boolean flag (output shape lands in T-003; this task
    only adds the flag and a placeholder code path).

In `speccy-cli/src/archive.rs` (new file):

- `pub fn run(args: &ArchiveArgs, workspace: &Workspace) -> Result<...>`
  is the entry point. The function:
  1. Resolves the source path from the SPEC-NNNN ID by scanning
     `.speccy/specs/*/SPEC.md` for a matching `id` frontmatter.
     If no match, return a "not found" error that exits non-zero
     with stderr naming the ID.
  2. Parses SPEC.md via the existing `SpecMd::parse_raw` (T-001
     already taught it about the archive fields, though they will
     be `None` here).
  3. Checks `frontmatter.status`. If `SpecStatus::InProgress` and
     `--force` was not set, returns a refusal error whose stderr
     message names the current status value and the three allowed
     statuses (`implemented`, `dropped`, `superseded`).
  4. Computes the archive destination as
     `.speccy/archive/<slug>/` (preserving the directory name
     exactly).
  5. Reads the SPEC.md source bytes, mutates the frontmatter YAML
     to append `archived_at: <today UTC>` after `supersedes:`, and
     `archived_reason: "<value>"` after `archived_at:` when
     `--reason` was passed. Writes the new SPEC.md bytes back to
     the source path *before* `git mv` so the moved file already
     carries the new frontmatter (avoids a second commit-worthy
     edit at the destination).
  6. Creates `.speccy/archive/` if absent.
  7. Invokes `git mv <source-dir> <archive-dir>` via the existing
     command-shelling helper (whatever `speccy-cli` already uses
     for git interactions). On failure, surface the underlying
     git error to stderr and exit non-zero. Roll back the
     frontmatter mutation if the `git mv` fails (so the source
     SPEC.md is restored to pre-archive state).
  8. On success, print a human-readable confirmation line to
     stdout naming the source and destination paths and the
     recorded `archived_at`. The `--json` path is a placeholder
     this task; T-003 fills it in.

In `speccy-cli/src/archive.rs` tests (and/or
`speccy-cli/tests/archive_text.rs`):

- A fixture workspace at `tests/fixtures/archive_workspace/` with
  a few SPECs at varying statuses.
- Test: archive an `implemented` spec → exit 0, source dir gone,
  archive dir present, SPEC.md inside archive has `archived_at`.
- Test: archive an `in-progress` spec without `--force` → exit
  non-zero, stderr names the current status and the three
  allowed statuses, no filesystem change.
- Test: archive an `in-progress` spec with `--force` → exit 0,
  spec moves, status field in moved SPEC.md is still
  `in-progress` (the gate bypass does not mutate status).
- Test: archive `SPEC-9999` (non-existent) → exit non-zero,
  stderr names the missing ID.
- Test: `speccy archive` with no positional → clap exit 2.
- Test: `--reason "line1\nline2"` (literal newline) → clap exit 2.
- Test: `--reason "shipped 2025-12-15"` on an `implemented` spec
  → exit 0, archived SPEC.md frontmatter contains
  `archived_reason: "shipped 2025-12-15"`.
- Test: archive without `--reason` → archived SPEC.md frontmatter
  contains no `archived_reason` field.
- Test: `git status --porcelain=v1 -z` after archive reports a
  rename for the spec directory (not delete + add).

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a built `speccy` binary at HEAD after this task and a
fixture workspace where `.speccy/specs/0001-artifact-parsers/SPEC.md`
has `status: implemented`,
when `speccy archive SPEC-0001` runs,
then the process exits 0,
`.speccy/archive/0001-artifact-parsers/SPEC.md` exists,
`.speccy/specs/0001-artifact-parsers/` does not exist,
`git status --porcelain=v1 -z` reports a rename, and the moved
SPEC.md frontmatter contains `archived_at` set to today's UTC
date (covers CHK-001).

Given the same binary,
when `speccy archive SPEC-9999` runs against a workspace with no
SPEC-9999,
then the process exits non-zero, stderr names `SPEC-9999`, and
`.speccy/archive/` is unchanged (covers CHK-002).

Given the same binary,
when `speccy archive` runs with no positional argument,
then the process exits 2 with a clap "missing argument" error
naming the positional (covers CHK-003).

Given a fixture workspace where SPEC-0042 has `status:
in-progress`,
when `speccy archive SPEC-0042` runs (no `--force`),
then the process exits non-zero, stderr contains the substring
`in-progress` and the three substrings `implemented`, `dropped`,
`superseded`, and the source directory is unchanged (covers
CHK-004).

Given the same fixture,
when `speccy archive SPEC-0042 --force` runs,
then the process exits 0, the spec moves to
`.speccy/archive/0042-*/`, and the moved SPEC.md's `status:` line
is still `status: in-progress` (covers CHK-005).

Given a fixture where SPEC-0050 has `status: superseded`,
when `speccy archive SPEC-0050` runs without `--force`,
then the process exits 0 and the spec moves (covers CHK-006).

Given a fixture where SPEC-0001 is `implemented`,
when `speccy archive SPEC-0001 --reason "ship cleanup"` runs at
UTC 2026-05-23,
then the moved SPEC.md frontmatter contains both
`archived_at: 2026-05-23` and `archived_reason: "ship cleanup"`,
in that order, after the existing `supersedes:` line (covers
CHK-007).

Given the same fixture,
when `speccy archive SPEC-0001` runs without `--reason`,
then `rg -n '^archived_reason:' .speccy/archive/0001-*/SPEC.md`
prints zero matches and `rg -n '^archived_at:'
.speccy/archive/0001-*/SPEC.md` prints exactly one match (covers
CHK-008).

Given the same binary,
when `speccy archive SPEC-0001 --reason "$(printf 'a\nb')"` runs
with a literal newline in the reason,
then the process exits 2 with a clap value-parser error naming
`--reason` (covers CHK-009).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-cli/src/main.rs`,
`speccy-cli/src/archive.rs` (new),
`speccy-cli/tests/archive_text.rs` (new),
`speccy-cli/tests/fixtures/archive_workspace/` (new).
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-009">
## Implement `speccy archive --json` receipt output shape

In `speccy-cli/src/archive.rs`:

- Add a `pub struct ArchiveReceipt` (or equivalent) with
  `serde::Serialize` deriving:
  ```rust
  struct ArchiveReceipt {
      schema_version: u32,        // == 1
      archived: ArchivedSpec,
      warnings: Vec<ArchiveWarning>,
  }
  struct ArchivedSpec {
      id: String,                 // "SPEC-NNNN"
      slug: String,
      from: String,               // ".speccy/specs/NNNN-slug"
      to: String,                 // ".speccy/archive/NNNN-slug"
      archived_at: String,        // "YYYY-MM-DD"
      archived_reason: Option<String>,
  }
  struct ArchiveWarning { /* T-004 populates */ }
  ```
- When `--json` is passed, after a successful archive the command
  prints exactly one JSON object to stdout (the receipt), then
  exits 0. The placeholder text-mode confirmation line from T-002
  is replaced by JSON; no other stdout output is emitted under
  `--json`.
- When `--json` is passed and the command fails (status gate,
  not-found, parse error, filesystem error), stdout is empty and
  stderr carries the human-readable error. Exit code is non-zero.
- `archived_reason` is `serde::Serialize`d such that absent values
  emit JSON `null` (not omitted). Use `Option<String>` with no
  `skip_serializing_if` attribute.
- `warnings` is always present as `[]` when no warnings — T-004
  will populate the array; this task ensures the field exists and
  defaults to an empty `Vec`.

In `speccy-cli/tests/archive_json.rs` (new):

- Test: `speccy archive SPEC-0001 --json` on an `implemented`
  spec → exit 0, stdout parses as the receipt JSON,
  `schema_version == 1`, `archived.id == "SPEC-0001"`,
  `archived.archived_reason == null`, `warnings == []`.
- Test: `--json --reason "ship cleanup"` → `archived_reason ==
  "ship cleanup"` (JSON string).
- Test: `--json` on an `in-progress` spec without `--force` →
  stdout empty, stderr non-empty, exit non-zero.
- Test: receipt's `from` and `to` paths use forward slashes (the
  speccy convention; `Utf8Path` rendering already enforces this)
  even on Windows runners.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a built `speccy` binary at HEAD and a workspace where
SPEC-0001 has `status: implemented`,
when `speccy archive SPEC-0001 --json --reason "ship cleanup"`
runs,
then stdout JSON parses, `jq -r '.schema_version'` prints `1`,
`jq -r '.archived.id'` prints `SPEC-0001`,
`jq -r '.archived.archived_reason'` prints `ship cleanup`, and
`jq -r '.archived.to'` prints
`.speccy/archive/0001-artifact-parsers` (covers CHK-023).

Given the same binary and a workspace where SPEC-0042 is
`in-progress`,
when `speccy archive SPEC-0042 --json` runs (no `--force`),
then stdout is empty, the process exits non-zero, and stderr
contains the status-gate message (covers CHK-024).

Given the same binary,
when `speccy archive SPEC-0001 --json` runs without `--reason`,
then the JSON output's `archived.archived_reason` is JSON `null`
(not omitted, not the literal string `"null"`).

Given the same binary,
when `speccy archive SPEC-0001 --json` runs against a workspace
where no orphan-supersession is triggered,
then JSON output's `warnings` field is the empty array `[]`
(present, not omitted).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-cli/src/archive.rs`,
`speccy-cli/tests/archive_json.rs` (new).
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-008">
## Detect supersession-chain orphan and emit warning on archive

In `speccy-core/src/parse/supersession.rs` (or wherever the
existing supersedes-graph helpers live):

- Add `pub fn orphan_candidates_on_archive(
      workspace: &Workspace,
      archiving: SpecId,
  ) -> Vec<SpecId>`
  that implements the detection algorithm from SPEC-0042 REQ-008:
  1. Read the spec being archived (call it `archiving`). Collect
     its `supersedes` list.
  2. For each `X` in that list:
     a. If `X` is active (present in `.speccy/specs/`), AND
     b. `X`'s frontmatter status is `superseded`, AND
     c. No other active spec besides `archiving` declares `X` in
        its `supersedes` list,
     then `X` is an orphan candidate.
  3. Return the sorted (`SpecId` order) list of orphan candidates.

  When `archiving` has an empty `supersedes` list, or every
  candidate fails one of (a)/(b)/(c), the returned `Vec` is
  empty.

In `speccy-cli/src/archive.rs`:

- Before invoking `git mv`, call
  `orphan_candidates_on_archive(...)`.
- For each orphan candidate `X`, write a warning line to stderr:
  ```
  warning: archiving SPEC-Y will orphan SPEC-X
  (SPEC-X has status: superseded and no other active spec
  declares supersedes: [SPEC-X]; SPC-006 will fire on SPEC-X
  after the move).
  ```
  (Wrap the message body cleanly; the example above shows
  intent.)
- For the `--json` path (T-003 receipt), populate the `warnings`
  array with one entry per orphan candidate:
  ```json
  { "spec": "SPEC-X", "reason": "orphaned-supersession" }
  ```
  The `ArchiveWarning` struct from T-003 gains its concrete
  shape here.
- Archival proceeds regardless of warnings.

In `speccy-core/tests/supersession.rs` (or wherever existing
supersedes tests live):

- Test (Scenario 2 — warn): workspace with SPEC-0019 active
  `status: superseded`, SPEC-0021 active `supersedes:
  [SPEC-0019]` as the sole declarer →
  `orphan_candidates_on_archive(ws, "SPEC-0021") ==
  vec!["SPEC-0019"]`.
- Test (Scenario 1 — natural, no warn): same workspace,
  `orphan_candidates_on_archive(ws, "SPEC-0019") == vec![]`.
- Test (multi-declarer — no warn): SPEC-0019 active `superseded`,
  SPEC-0021 and SPEC-0022 both declare `supersedes: [SPEC-0019]`
  → archiving SPEC-0021 returns empty.
- Test (archived target — no warn): SPEC-0019 already in archive
  (not active), SPEC-0021 still declares `supersedes:
  [SPEC-0019]` → empty.
- Test (empty supersedes — no warn): SPEC-Y has `supersedes: []`
  → empty.

In `speccy-cli/tests/archive_warnings.rs` (new):

- End-to-end test of each scenario using fixture workspaces:
  - Warn case: assert stderr contains the warning line, JSON
    `warnings` contains one entry, exit 0 (archival proceeds).
  - Natural case: assert stderr has no warning, JSON `warnings`
    is `[]`.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a workspace where SPEC-0019 is active with
`status: superseded` and SPEC-0021 is the sole active spec
declaring `supersedes: [SPEC-0019]`,
when `speccy archive SPEC-0021 --json` runs,
then the process exits 0, stderr contains a warning line naming
both `SPEC-0019` and `SPEC-0021`, and stdout JSON contains
`"warnings": [{"spec":"SPEC-0019","reason":"orphaned-supersession"}]`
(covers CHK-020).

Given the same workspace,
when `speccy archive SPEC-0019 --json` runs (archiving the
older, superseded spec — the natural case),
then the process exits 0 and stdout JSON contains
`"warnings": []` (covers CHK-021).

Given a workspace where SPEC-0019 is active with
`status: superseded` and BOTH SPEC-0021 and SPEC-0022 declare
`supersedes: [SPEC-0019]`,
when `speccy archive SPEC-0021 --json` runs,
then `warnings` is `[]` in the JSON output (covers CHK-022).

Given a workspace where SPEC-0019 is already in archive and
SPEC-0021 is active with `supersedes: [SPEC-0019]`,
when `speccy archive SPEC-0021 --json` runs,
then `warnings` is `[]` (the orphan-target is not in the active
set).

Given a workspace where SPEC-0030 has `supersedes: []`,
when `speccy archive SPEC-0030 --json` runs,
then `warnings` is `[]` (the source spec supersedes nothing).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-core/src/parse/supersession.rs`,
`speccy-cli/src/archive.rs`,
`speccy-core/tests/supersession.rs` (or equivalent),
`speccy-cli/tests/archive_warnings.rs` (new).
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-005">
## Extend `speccy vacancy` to scan `.speccy/archive/` so archived IDs remain occupied

In `speccy-core/src/vacancy.rs` (or wherever the existing vacancy
resolver lives):

- Extend the workspace scan that enumerates taken SPEC IDs to
  glob both `.speccy/specs/*/SPEC.md` and
  `.speccy/archive/*/SPEC.md`. The union of taken IDs from both
  globs is what `next_spec_id` is computed against (smallest
  unused `SPEC-NNNN` ≥ `SPEC-0001`).
- The scan is robust to `.speccy/archive/` being absent (treat
  as empty) and to malformed/unparseable SPEC.md files inside
  archive (skip and continue — the user must not have an archive
  scan fail vacancy when one corrupt archive entry exists).
- The vacancy command's text-mode output is unchanged in shape;
  the computed `next_spec_id` simply reflects the broader scan.

In `speccy-core/tests/vacancy.rs`:

- Test: `.speccy/specs/` contains `0001-foo/`, `0003-bar/`;
  `.speccy/archive/` contains `0002-baz/` →
  `next_spec_id == "SPEC-0004"` (the archive entry blocks reuse
  of 0002).
- Test: `.speccy/specs/` is contiguous 0001–0041, no archive →
  `next_spec_id == "SPEC-0042"` (unchanged behavior).
- Test: `.speccy/specs/` is 0001–0041, `.speccy/archive/`
  contains 0042 → `next_spec_id == "SPEC-0043"`.
- Test: `.speccy/archive/` contains one unparseable SPEC.md →
  vacancy still completes without error; that ID is treated as
  occupied if the slug-extractable-from-dirname matches NNNN
  pattern, else skipped.

In `speccy-cli/tests/vacancy_json.rs`:

- Test: end-to-end `speccy vacancy --json` on a workspace with
  an archived spec → `next_spec_id` reflects the archive
  occupation.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a built `speccy` binary at HEAD and a workspace where
`.speccy/specs/` contains exactly `0001-foo/` and
`.speccy/archive/` contains exactly `0002-bar/`,
when `speccy vacancy --json` runs,
then stdout contains `"next_spec_id":"SPEC-0003"` (covers CHK-012).

Given the same binary,
when SPEC-0001 is archived from `0001-foo/` and `speccy vacancy
--json` runs immediately after,
then `next_spec_id` is unchanged from its pre-archive value
(the archived spec still occupies its slot — covers CHK-013).

Given a workspace where `.speccy/archive/` does not exist,
when `speccy vacancy --json` runs,
then the scan completes without error and returns the smallest
ID unused under `.speccy/specs/`.

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-core/src/vacancy.rs`,
`speccy-core/tests/vacancy.rs`,
`speccy-cli/tests/vacancy_json.rs`.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-007">
## Add `speccy status --include-archive` flag for archive visibility

In `speccy-cli/src/main.rs` (the `Status` subcommand args
struct):

- Add a new boolean flag `include_archive: bool` (default
  `false`) alongside the existing `all: bool` flag.
- Document both flags' independence in the long help text: `--all`
  broadens the attention filter on non-archived specs;
  `--include-archive` adds archived specs to the scan.

In `speccy-cli/src/status.rs`:

- The workspace scan path is extended to optionally glob
  `.speccy/archive/*/SPEC.md` when `include_archive == true`,
  unioning the results with the active scan before any filtering.
- The attention filter (used when `--all` is `false`) treats
  archived specs as never "needing attention" — they are
  surfaced only because `--include-archive` opted them in, and
  do not participate in the attention-list footer count of
  hidden specs.
- The JSON entry for an archived spec carries the new
  `archived_at` and `archived_reason` fields populated from
  frontmatter (T-001 made these parseable; T-006 wires them
  into the JSON path under the `include_archive` branch).
- The text-mode output for an archived spec includes a marker
  (e.g. `[archived 2026-05-23]`) so a reader can tell from the
  text rendering which entries came from archive.

In `speccy-cli/tests/status_include_archive.rs` (new):

- Test: workspace with one active spec and one archived spec.
  - `speccy status --json` → archived spec absent from output.
  - `speccy status --include-archive --json` → both specs
    present; archived entry has populated `archived_at`.
  - `speccy status --all --include-archive --json` → both
    present, plus any otherwise-hidden active specs that
    `--all` surfaces.
- Test: `speccy next --include-archive` → clap exit 2 with
  "unrecognized argument" naming the flag.
- Test: `speccy check --include-archive` → clap exit 2.
- Test: `speccy verify --include-archive` → clap exit 2.
- Test: `speccy lock --include-archive` → clap exit 2.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a built `speccy` binary at HEAD and a workspace where
SPEC-0001 has been archived,
when `speccy status --include-archive --json` runs,
then `jq '.specs[] | select(.id == "SPEC-0001") | .archived_at'`
prints a date in `YYYY-MM-DD` form (covers CHK-017).

Given the same binary and workspace,
when `speccy status --json` runs without flags,
then `jq '.specs | map(select(.id == "SPEC-0001")) | length'`
prints `0` (covers CHK-018).

Given the same binary,
when `speccy next --include-archive` runs,
then the process exits 2 and stderr contains an "unknown flag"
or equivalent clap error naming `--include-archive` (covers
CHK-019).

Given a workspace with one archived spec and one active
`implemented` spec,
when `speccy status --all --include-archive --json` runs,
then both specs appear in the output and the archived entry
carries the `archived_at` field.

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-cli/src/main.rs`,
`speccy-cli/src/status.rs`,
`speccy-cli/tests/status_include_archive.rs` (new).
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-006">
## Verify hot-path commands ignore `.speccy/archive/` under every flag combination

The workspace-scan paths for `speccy status` (default mode),
`next`, `check`, `verify`, and `lock` already glob
`.speccy/specs/*/SPEC.md` and never reach into
`.speccy/archive/`. This task verifies that invariant
end-to-end with integration tests and adds a regression guard,
in case a future change widens a scanner's glob pattern.

In `speccy-core/src/workspace.rs` (or wherever the discovery
helper lives):

- Confirm the discovery function used by `status` (default),
  `next`, `check`, `verify`, `lock` only globs
  `.speccy/specs/`. If today's code paths each construct their
  own glob inline (rather than calling a shared helper), extract
  a `discover_active_specs(workspace_root: &Utf8Path) ->
  Vec<SpecPath>` function so the "active set" scoping lives in
  one place. The `include_archive` opt-in from T-006 stays
  separate as `discover_archived_specs(...)`.
- Document on `discover_active_specs` that it deliberately
  excludes `.speccy/archive/`, with a one-line reference to
  SPEC-0042 REQ-006.

In `speccy-cli/tests/archive_invisibility.rs` (new):

- Set up a fixture workspace with active specs and one archived
  spec.
- Test: `speccy status --json` (no flags) → archived spec absent
  from output.
- Test: `speccy next --json` → archived spec not surfaced; the
  resolver does not return a `next_action` against it.
- Test: `speccy check SPEC-NNNN` where NNNN is the archived
  spec's ID → exit non-zero with a "spec not found" message.
- Test: `speccy verify` over a workspace where the only specs
  needing lint attention are archived → exit 0 (archive lint
  state is ignored).
- Test: `speccy lock SPEC-NNNN` against the archived spec's ID
  → exit non-zero with a "spec not found" or equivalent error;
  the archived TASKS.md frontmatter is unchanged.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`.

<task-scenarios>
Given a built `speccy` binary at HEAD and a workspace where
SPEC-0001 has been archived,
when `speccy status --json` runs (no flags),
then `jq '.specs | map(select(.id == "SPEC-0001")) | length'`
returns `0` (covers CHK-014).

Given the same workspace,
when `speccy check SPEC-0001` runs,
then the process exits non-zero and stderr contains a "not
found" message referencing `SPEC-0001` (covers CHK-015).

Given a workspace where every active spec passes `speccy verify`
pre-archive,
when half of the implemented specs are archived and `speccy
verify` runs again,
then the process exits 0 and the JSON output reflects only the
still-active specs (covers CHK-016).

Given a workspace where SPEC-0001 has been archived,
when `speccy lock SPEC-0001` runs,
then the process exits non-zero and the archived
`.speccy/archive/0001-*/TASKS.md` frontmatter is unchanged.

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0.

Suggested files: `speccy-core/src/workspace.rs`,
`speccy-cli/tests/archive_invisibility.rs` (new),
plus any existing test files that exercise status/next/check/verify/lock
discovery.
</task-scenarios>
</task>

<task id="T-008" state="completed" covers="REQ-010">
## Update `AGENTS.md` and `docs/ARCHITECTURE.md` for the new command and the count-agnostic CLI surface

In `AGENTS.md`:

- Locate the `### V1.0 outcome` line under `## Product north star`
  that reads "Seven-command Rust CLI implementing the surface in
  `docs/ARCHITECTURE.md`: `init`, `status`, `next`, `check`,
  `verify`, `lock`, `vacancy`."
- Replace it with a count-agnostic phrasing such as: "A lean Rust
  CLI implementing the surface in `docs/ARCHITECTURE.md`. The
  surface is intentionally small — see the `## Core principles`
  'Stay small' rule — but the exact command list lives in the
  architecture doc, not in this north star."
- The replacement names `docs/ARCHITECTURE.md` as the
  authoritative source and contains no specific integer count of
  commands.
- The `## Core principles` "Stay small" item is also rewritten to
  drop the literal `seven commands` integer while keeping its
  substance (caution against gratuitous CLI growth), per REQ-010.

In `docs/ARCHITECTURE.md`:

- Locate the existing CLI command surface section that
  enumerates `init`, `status`, `next`, `check`, `verify`,
  `lock`, `vacancy`.
- Add an entry for `archive` consistent with the surrounding
  prose conventions. Short description: "Relocates a shipped,
  dropped, or superseded SPEC from `.speccy/specs/NNNN-slug/` to
  `.speccy/archive/NNNN-slug/` via `git mv`; archived specs
  retain their SPEC-NNNN IDs and are invisible to
  hot-path commands."
- Add a short note that `--include-archive` exists on `speccy
  status` only (per SPEC-0042 REQ-007).
- Add a short note under the existing `vacancy` entry that its
  scan covers `.speccy/archive/` so archived IDs remain
  reserved (per SPEC-0042 REQ-005).

This task makes no functional source changes; it is doc-only.

Hygiene gate: `cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all four must pass before
flipping to `in-review`. (The doc edits should not affect any of
these, but the gate runs anyway as a regression guard.)

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -n 'Seven-command|seven-command|7-command' AGENTS.md`
runs,
then it prints zero matches (covers CHK-025).

Given the same checkout,
when `rg -nU 'speccy archive' docs/ARCHITECTURE.md` runs,
then it prints at least one match inside the CLI command surface
section (covers CHK-026).

Given the same checkout,
when a reader scans `AGENTS.md`'s `### V1.0 outcome` section,
then the prose references `docs/ARCHITECTURE.md` as the
authoritative CLI surface and names no specific integer count
of commands.

Given the same checkout,
when a reader scans `docs/ARCHITECTURE.md`'s `vacancy` entry,
then the prose notes that `vacancy` scans both `.speccy/specs/`
and `.speccy/archive/` (referencing SPEC-0042 REQ-005).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0 (no functional regressions from doc-only
changes).

Suggested files: `AGENTS.md`, `CLAUDE.md` (symlink — confirm
it tracks the AGENTS.md edit), `docs/ARCHITECTURE.md`.
</task-scenarios>
</task>
