---
spec: SPEC-0042
outcome: implemented
generated_at: 2026-05-23T20:00:00Z
---

# REPORT: SPEC-0042 Archive completed specs -- `speccy archive SPEC-NNNN` relocates shipped/dropped/superseded specs out of the active set

<report spec="SPEC-0042">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-002 added the `speccy archive` subcommand to `speccy-cli` with a positional
`SPEC-NNNN` argument and `--reason`, `--force`, and `--json` flags. The new
`speccy-cli/src/archive.rs` module resolves the source path by scanning
`.speccy/specs/*/SPEC.md`, invokes `git mv` to relocate the directory to
`.speccy/archive/NNNN-slug/`, and creates the archive parent directory on
first use. CHK-001 (success path: exit 0, source absent, destination present,
git rename recorded) and CHK-002 (non-existent ID: exit non-zero, no
filesystem mutation) are covered by `speccy-cli/tests/archive_text.rs`.
CHK-003 (missing positional: clap exit 2) is covered by the same suite.
Retry count: 1.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
T-002 implemented the status gate: when `status` is `in-progress` and
`--force` was not passed, the command exits non-zero with a stderr message
naming the current status and the three archivable statuses (`implemented`,
`dropped`, `superseded`); with `--force`, archival proceeds and the relocated
SPEC.md's `status:` field is unchanged. CHK-004 (`in-progress` without
`--force`: refusal), CHK-005 (`in-progress` with `--force`: proceeds, status
still `in-progress`), and CHK-006 (`superseded` without `--force`: succeeds)
are covered in `speccy-cli/tests/archive_text.rs`.
Retry count: 1.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-007 CHK-008 CHK-009">
T-001 extended `SpecFrontmatter` with `archived_at: Option<jiff::civil::Date>`
and `archived_reason: Option<String>` in `speccy-core/src/parse/spec_md.rs`.
T-002 wired the frontmatter mutation into `archive.rs`: `archived_at` is
appended unconditionally (UTC date at archival time), and `archived_reason`
is appended only when `--reason` is passed. The `--reason` value parser
rejects strings containing newline characters at clap argument-parse time.
CHK-007 (both fields written in order after `supersedes:`), CHK-008 (no
`archived_reason` when `--reason` absent), and CHK-009 (`--reason` with
literal newline: clap exit 2) are covered in `speccy-cli/tests/archive_text.rs`.
Retry count: 1.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-010 CHK-011">
T-001 added `"archived_at"` and `"archived_reason"` to
`HASH_EXCLUDED_FRONTMATTER_FIELDS` in `speccy-core/src/parse/spec_md.rs`
alongside the existing `"status"` exclusion. The `canonical_frontmatter_for_hash`
function skips all three keys when building the `Sha256` byte stream.
CHK-010 (sha256 invariant after adding both archive fields) is covered by
the `sha256_invariant_under_archive_fields_addition` unit test; CHK-011
(sha256 differs when body bytes change) is covered by the regression guard
test in the same file. The doc comment on `SpecMd.sha256` was updated to list
all three excluded fields.
Retry count: 1.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-012 CHK-013">
T-005 extended `speccy-core/src/prompt/id_alloc.rs` with
`allocate_next_spec_id_across_dirs(dirs: &[&Utf8Path])` and updated
`speccy-cli/src/vacancy.rs` to pass both `.speccy/specs/` and
`.speccy/archive/` to it. The scan is robust to an absent archive directory.
CHK-012 (specs={0001} union archive={0002} returns `next_spec_id=SPEC-0003`)
is covered by `vacancy_json_archive_blocks_id_reuse` in
`speccy-cli/tests/vacancy.rs` and unit tests in `id_alloc.rs`. CHK-013
(archiving a spec does not change `next_spec_id`) is covered by
`vacancy_next_id_unchanged_when_spec_moves_from_specs_to_archive`.
Retry count: 1.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-014 CHK-015 CHK-016">
T-007 confirmed and regression-guarded that the workspace scan shared by
`status` (default), `next`, `check`, `verify`, and `lock` globs only
`.speccy/specs/*/SPEC.md`. A `discover_active_specs` helper in
`speccy-core/src/workspace.rs` centralises the scope with a doc comment
referencing SPEC-0042 REQ-006. CHK-014 (`status --json` after archive:
SPEC-0001 absent), CHK-015 (`check SPEC-0001` after archive: non-zero with
"not found"), and CHK-016 (`verify` after archiving all shipped specs: exits 0)
are covered in `speccy-cli/tests/archive_invisibility.rs`.
Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-017 CHK-018 CHK-019">
T-006 added `--include-archive` to the `Status` subcommand args in
`speccy-cli/src/main.rs` and extended `speccy-cli/src/status.rs` to union
`.speccy/archive/*/SPEC.md` into the scan when the flag is set. Archived specs
are excluded from the attention filter and carry `archived_at` and
`archived_reason` in the JSON entry. Text-mode output includes an
`[archived YYYY-MM-DD]` marker. CHK-017 (`--include-archive --json`: archived
entry present with `archived_at`), CHK-018 (`--json` without flag: archived
entry absent), and CHK-019 (`next --include-archive`: clap exit 2) are
covered in `speccy-cli/tests/status_include_archive.rs`.
Retry count: 0.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-020 CHK-021 CHK-022">
T-004 added `orphan_candidates_on_archive(workspace, archiving)` in
`speccy-core/src/parse/supersession.rs` implementing the three-condition
detection algorithm from the SPEC. `archive.rs` calls this before `git mv`
and emits a per-orphan warning to stderr; the `--json` `warnings` array
carries one `{spec, reason}` entry per orphan. Archival always proceeds.
CHK-020 (sole declarer archived: warning emitted, exit 0), CHK-021 (natural
case -- older superseded spec archived: no warning), and CHK-022 (two
declarers: no warning) are covered by unit tests in
`speccy-core/tests/supersession.rs` and end-to-end tests in
`speccy-cli/tests/archive_warnings.rs`. The `warnings` field is always
present as `[]` when no orphan fires.
Retry count: 0.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-023 CHK-024">
T-003 added `ArchiveReceipt`, `ArchivedSpec`, and `ArchiveWarning` structs
with `serde::Serialize` in `speccy-cli/src/archive.rs`. Under `--json`,
stdout is exactly one JSON object on success and empty on failure. The
`archived_reason` field serialises as JSON `null` (not omitted) when
`--reason` is absent; `warnings` is always `[]` when no orphan fires.
CHK-023 (`--json --reason "ship cleanup"`: receipt parses, fields correct)
and CHK-024 (`--json` on `in-progress` without `--force`: stdout empty,
non-zero exit) are covered in `speccy-cli/tests/archive_json.rs`.
Retry count: 0.
</coverage>

<coverage req="REQ-010" result="satisfied" scenarios="CHK-025 CHK-026">
T-008 updated `AGENTS.md` to replace the "Seven-command Rust CLI" line with
count-agnostic phrasing pointing to `docs/ARCHITECTURE.md` as the authoritative
surface, and rewrote the "Stay small" principle item to drop the literal
`seven commands` integer. `docs/ARCHITECTURE.md` gained an `archive` entry
in its CLI command surface section with a description consistent with the
surrounding entries, a note on `--include-archive` being status-only, and an
updated `vacancy` entry noting its archive scan. The sweep also covered
`README.md`. CHK-025 (`rg 'Seven-command' AGENTS.md` prints zero matches) and
CHK-026 (`rg 'speccy archive' docs/ARCHITECTURE.md` prints at least one match)
are satisfied.
Retry count: 1.
</coverage>

</report>

## Notes

T-002 was the most complex landing, requiring two implementer rounds to
resolve evidence-contract and style blockers from the first review. T-005
similarly required two rounds to write the evidence file that the tests
reviewer required and to fix stale doc comments. The underlying test work in
both cases was sound from round 1 and was not regressed in round 2.

The `--force` bypass for `in-progress` specs intentionally leaves the
`status:` field untouched in the relocated SPEC.md -- per DEC-005, a
`--force`-archived spec stays `in-progress` because mutating status
silently would rewrite intent without the user asking for it.