---
id: SPEC-0042
slug: archive-completed-specs
title: Archive completed specs — `speccy archive SPEC-NNNN` relocates shipped/dropped/superseded specs out of the active set
status: implemented
created: 2026-05-23
supersedes: []
archived_at: 2026-05-23
archived_reason: "v1 milestone shipped"
---

# SPEC-0042: Archive completed specs — `speccy archive SPEC-NNNN` relocates shipped/dropped/superseded specs out of the active set

## Summary

A SPEC's artifacts (`TASKS.md`, `journal/`, optional `evidence/`,
`REPORT.md`) are ephemeral by design — they exist to let implementer
and reviewer agents communicate during the development loop and become
obsolete the moment the SPEC ships. `SPEC.md` itself is the only
durable artifact, and even it is mostly historical context once the
feature is in the product.

Speccy's own dogfood repo today carries 41 specs under
`.speccy/specs/`, all `status: implemented`. The active set is now
mostly noise: `speccy status`, glob enumeration, IDE file pickers, and
LLM context windows all pay the cost of carrying every completed spec
alongside the one or two specs actually being worked on. The signal
that matters — "what's in flight?" — is buried.

This SPEC adds `speccy archive SPEC-NNNN`, an eighth CLI subcommand
that relocates a shipped (or dropped, or superseded) spec from
`.speccy/specs/NNNN-slug/` to `.speccy/archive/NNNN-slug/`. Hot-path
commands (`status`, `next`, `check`, `verify`, `lock`) ignore the
archive directory entirely; `vacancy` scans it (because archived
specs retain their SPEC-NNNN IDs); `status --include-archive` opts
back into archive visibility for browsing. The archive is a location,
not a deletion — every artifact moves intact via `git mv`, and the
spec can be unarchived manually with the reverse `git mv` if the
archival turns out to have been premature.

The CLI surface line in `AGENTS.md`'s V1.0-outcome section, which
currently pins a specific command count ("Seven-command Rust CLI"),
is replaced with a count-agnostic phrasing inside this SPEC. The
CLI is allowed to grow as scoped product needs surface; the
"stay-small" principle remains, but the contract no longer carries
a hardcoded integer.

## Goals

<goals>
- `speccy archive SPEC-NNNN` is a new CLI subcommand that relocates
  `.speccy/specs/NNNN-slug/` to `.speccy/archive/NNNN-slug/` via
  `git mv`, preserving the canonical directory name unchanged across
  the move.
- The command refuses to archive a spec whose `status` is
  `in-progress`, exiting non-zero with a stderr message naming the
  blocking status. The `--force` flag bypasses the gate without
  mutating any frontmatter field other than what R-frontmatter
  prescribes.
- The command mutates SPEC.md frontmatter to add `archived_at` (UTC
  date `YYYY-MM-DD`) unconditionally, and `archived_reason: "..."`
  only when `--reason "..."` was passed.
- `archived_at` and `archived_reason` are excluded from SPEC.md
  hash input so archival produces no hash drift; unarchival therefore
  requires no frontmatter cleanup for hash correctness.
- Archived specs retain their SPEC-NNNN IDs: `speccy vacancy` scans
  both `.speccy/specs/` and `.speccy/archive/` when computing
  free SPEC IDs, and the archived ID is not returned as available.
- The active-set commands `speccy status`, `next`, `check`, `verify`,
  and `lock` do not discover or operate on specs in
  `.speccy/archive/` under any flag combination.
- `speccy status --include-archive` additionally surfaces archived
  specs in its output. The flag is independent of and combinable with
  the existing `--all` flag (which broadens the attention filter on
  the non-archived set).
- The archive command emits a warning when archiving the spec orphans
  a still-active `status: superseded` spec — i.e., when the spec
  being archived is the only active spec declaring `supersedes: [X]`
  for some active SPEC-X. The warning names the affected SPEC-X and
  notes that `SPC-006` will fire on it. Archival proceeds.
- The archive command supports `--json` for scripted callers; the
  output shape includes the archived ID, the new path, the recorded
  `archived_at`, optional `archived_reason`, and any orphan warnings
  emitted.
- `AGENTS.md`'s V1.0-outcome line that names "Seven-command Rust CLI"
  is updated to a count-agnostic phrasing as part of this SPEC.
</goals>

## Non-goals

<non-goals>
- No `speccy unarchive` subcommand. The escape hatch for an
  erroneous archive is manual `git mv .speccy/archive/NNNN-slug
  .speccy/specs/`. Frontmatter cleanup (removing `archived_at` /
  `archived_reason`) is cosmetic only; the hash exclusion keeps
  SPEC.md's sha256 invariant either way.
- No bulk archive (`--all`, `--status=implemented`, `--older-than`,
  glob arguments). The command takes exactly one positional
  `SPEC-NNNN`. Bulk archival of today's 41 implemented specs is a
  shell loop.
- No automatic recursive archival. Archiving SPEC-Y does not also
  archive SPEC-X that Y supersedes; orphaning is surfaced as a
  warning (per Goals), not auto-resolved.
- No archive listing subcommand. `ls .speccy/archive/` and standard
  shell tooling are sufficient; `speccy status --include-archive`
  covers the structured-query case for the only command where the
  user is likely to want it.
- No `--include-archive` flag on `speccy check` or `speccy verify`.
  Archive specs are intentionally not lintable on demand; if
  archeological linting becomes a real need, that is a follow-up
  SPEC, not v1.
- No mutation of `status` during archival, including under `--force`.
  A `--force`-archived `in-progress` spec stays `in-progress` in
  frontmatter; the user accepted the warning and bypassed the gate,
  so cleanup of its own status (if desired) is on them.
- No transitional alias. `speccy archive` lands cleanly with no
  legacy-name fallback (there is no legacy name to begin with).
- No new lint family for archive-orphaned supersession chains. The
  existing `SPC-006` ("status = superseded but no other spec declares
  it as a supersedes target") covers Scenario 2's downstream effect;
  the archive command's warning is the upstream nudge.
</non-goals>

## User Stories

<user-stories>
- As a solo developer running Speccy on my own project, I want
  shipped specs out of the active set so `speccy status` and my
  editor's file picker stop drowning me in completed work, and the
  one or two specs actually in flight are the only ones I see by
  default.
- As an AI coding agent driven by a Speccy skill pack, I want
  `speccy next` and `speccy status` to omit archived specs from
  their JSON output, so my decision about "what to work on next"
  is not paginated through dozens of `status: implemented` entries
  whose REPORT.md is already in the tree.
- As a future maintainer revisiting the project six months from
  now, I want archived specs to remain on-disk under
  `.speccy/archive/` with their full artifact trail (TASKS.md,
  journal/, REPORT.md), so I can read why a decision was made
  without paying the active-set tax during normal work.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `speccy archive SPEC-NNNN` subcommand introduced; relocates spec dir via `git mv`

A new CLI subcommand `speccy archive` is added to `speccy-cli`. It
takes one positional argument, a `SPEC-NNNN` ID, and three optional
flags: `--reason <STRING>`, `--force`, and `--json`. On success it
relocates `.speccy/specs/NNNN-slug/` to `.speccy/archive/NNNN-slug/`
via `git mv`, preserving the canonical directory name. The
`.speccy/archive/` parent directory is created on first archival if
absent.

<done-when>
- `speccy archive --help` documents the positional `SPEC-NNNN` and the
  three flags `--reason`, `--force`, `--json`.
- `speccy archive` with no positional argument exits 2 with a stderr
  message naming the missing argument.
- `speccy archive SPEC-9999` (non-existent ID) exits non-zero with a
  stderr message naming the ID and noting no matching spec directory
  was found under `.speccy/specs/`.
- A successful archival of an `implemented` spec leaves the workspace
  with `.speccy/archive/NNNN-slug/` existing and
  `.speccy/specs/NNNN-slug/` absent, and the move is recorded in
  `git status` as a rename (not a delete + add).
- The relocated directory's name is byte-identical to the source
  directory's name — no rename, no date prefix.
</done-when>

<behavior>
- Given a workspace with `.speccy/specs/0001-artifact-parsers/` and
  no `.speccy/archive/`, when `speccy archive SPEC-0001` succeeds,
  then `.speccy/archive/` is created and contains exactly
  `0001-artifact-parsers/` with the full subtree (`SPEC.md`,
  `TASKS.md`, `journal/`, `REPORT.md`, and any other prior contents).
- Given the same workspace pre-archive, when `speccy archive
  SPEC-0001` is run inside a git repo, then `git status` reports
  the move as one or more rename entries under
  `.speccy/specs/0001-artifact-parsers/` → `.speccy/archive/0001-artifact-parsers/`,
  not separate delete + add entries.
- Given a workspace where `SPEC-9999` does not exist, when
  `speccy archive SPEC-9999` runs, then no filesystem mutation occurs
  and the process exits non-zero.
</behavior>

<scenario id="CHK-001">
Given a built `speccy` binary at HEAD after this SPEC lands and a
workspace where `.speccy/specs/0001-artifact-parsers/SPEC.md` has
`status: implemented`,
when `speccy archive SPEC-0001` runs,
then the process exits 0, `.speccy/archive/0001-artifact-parsers/SPEC.md`
exists, `.speccy/specs/0001-artifact-parsers/` does not exist, and
`git status --porcelain=v1 -z` reports a rename for the spec
directory.
</scenario>

<scenario id="CHK-002">
Given the same binary and a workspace with no SPEC-9999,
when `speccy archive SPEC-9999` runs,
then the process exits non-zero, stderr names `SPEC-9999` and the
absent spec directory, and no entry under `.speccy/archive/` is
created.
</scenario>

<scenario id="CHK-003">
Given the same binary,
when `speccy archive` runs with no positional argument,
then the process exits 2 (clap arg-parse failure) and stderr names
the missing positional.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Status gate refuses `in-progress`; `--force` bypasses without mutation

The archive command reads SPEC.md frontmatter `status` before
moving. If `status` is `in-progress` and `--force` was not passed,
the command exits non-zero with a stderr message naming the spec's
current status and the four allowed-for-archive statuses
(`implemented`, `dropped`, `superseded`). With `--force`, the
archival proceeds regardless of `status`; no frontmatter field is
mutated beyond what REQ-003 prescribes — in particular, `status` is
left unchanged.

<done-when>
- `speccy archive SPEC-NNNN` on an `in-progress` spec exits non-zero;
  stderr contains both the current `status` value and the names of
  the three archivable statuses.
- `speccy archive SPEC-NNNN --force` on an `in-progress` spec
  succeeds. After the move, the relocated SPEC.md still has
  `status: in-progress`.
- `speccy archive SPEC-NNNN` on `status: implemented`,
  `status: dropped`, or `status: superseded` succeeds without
  `--force`.
- The status-gate stderr message names the specific allowed-for-archive
  statuses, not a generic "not archivable" line — agents and humans
  reading the error should be able to act on it without consulting
  docs.
</done-when>

<behavior>
- Given a SPEC with `status: in-progress` and no `--force`, when
  `speccy archive` runs, then the process exits non-zero and the
  filesystem is unchanged.
- Given a SPEC with `status: in-progress` and `--force`, when
  `speccy archive` runs, then the process exits 0, the spec is
  relocated to `.speccy/archive/`, and the relocated SPEC.md's
  `status:` line still reads `in-progress`.
- Given a SPEC with `status: implemented`, when `speccy archive`
  runs without `--force`, then the process exits 0 and the spec
  relocates.
</behavior>

<scenario id="CHK-004">
Given a built `speccy` binary at HEAD and a workspace where
`SPEC-0042/SPEC.md` has `status: in-progress`,
when `speccy archive SPEC-0042` runs,
then the process exits non-zero, stderr contains the string
`in-progress` and the names of all three archivable statuses, and
`.speccy/specs/0042-*/` still exists.
</scenario>

<scenario id="CHK-005">
Given the same binary and the same workspace,
when `speccy archive SPEC-0042 --force` runs,
then the process exits 0, `.speccy/archive/0042-*/SPEC.md` exists,
and `rg -n '^status:' .speccy/archive/0042-*/SPEC.md` prints
`status: in-progress`.
</scenario>

<scenario id="CHK-006">
Given a SPEC with `status: superseded` in the active workspace,
when `speccy archive SPEC-NNNN` runs (no `--force`),
then the process exits 0 and the spec relocates.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Archive command writes `archived_at` (UTC) and optional `archived_reason` into SPEC.md frontmatter

The archive command, after passing the status gate but before
running `git mv`, edits the source SPEC.md to append two new keys to
its YAML frontmatter:

- `archived_at: YYYY-MM-DD` — the current UTC date at the moment of
  archival, written unconditionally on every archive invocation.
- `archived_reason: "..."` — the value of the `--reason` flag if
  passed. Omitted from frontmatter entirely when `--reason` was not
  passed. Stored as a YAML double-quoted scalar; the implementation
  rejects newline characters in the reason string at argument-parse
  time.

The fields are appended after the existing frontmatter keys
(`id`, `slug`, `title`, `status`, `created`, `supersedes`) in the
same YAML document. If the spec is later unarchived manually and the
user wants the fields gone, they edit them out; the hash exclusion
in REQ-004 means the sha256 is invariant either way.

<done-when>
- After a successful `speccy archive SPEC-0001`, the relocated
  SPEC.md frontmatter contains `archived_at: <today's UTC date>`.
- After `speccy archive SPEC-0001 --reason "cleanup of shipped v0 work"`,
  the relocated SPEC.md frontmatter additionally contains
  `archived_reason: "cleanup of shipped v0 work"`.
- After `speccy archive SPEC-0001` with no `--reason`, the relocated
  SPEC.md frontmatter contains zero occurrences of
  `archived_reason`.
- `speccy archive SPEC-0001 --reason "line1\nline2"` (literal newline
  in the string) exits 2 with a clap-level argument validation error.
- The `archived_at` date matches `date -u +%Y-%m-%d` taken at the
  moment of archival within a 1-day tolerance (CI clocks vary).
</done-when>

<behavior>
- Given a fresh archive invocation at UTC 2026-05-23, when `speccy
  archive SPEC-NNNN` runs, then the relocated SPEC.md frontmatter
  contains the literal line `archived_at: 2026-05-23`.
- Given a `--reason` containing only ASCII printable characters,
  when archive runs, then the value is serialized as a YAML
  double-quoted scalar with backslash-escaping for embedded quotes
  per standard YAML rules.
- Given an archive invocation without `--reason`, when the relocated
  SPEC.md is parsed, then its frontmatter parses successfully and
  has no `archived_reason` field.
</behavior>

<scenario id="CHK-007">
Given a built `speccy` binary at HEAD and a workspace where
`.speccy/specs/0001-artifact-parsers/SPEC.md` has
`status: implemented`,
when `speccy archive SPEC-0001 --reason "shipped 2025-12-15"` runs at
UTC 2026-05-23,
then the relocated SPEC.md frontmatter contains
`archived_at: 2026-05-23` and
`archived_reason: "shipped 2025-12-15"`, in that order, after the
existing `supersedes:` line.
</scenario>

<scenario id="CHK-008">
Given the same binary and a fresh workspace,
when `speccy archive SPEC-0001` runs without `--reason`,
then `rg -n '^archived_reason:' .speccy/archive/0001-*/SPEC.md`
prints zero matches and `rg -n '^archived_at:' .speccy/archive/0001-*/SPEC.md`
prints exactly one match.
</scenario>

<scenario id="CHK-009">
Given the same binary,
when `speccy archive SPEC-0001 --reason "$(printf 'a\nb')"` runs
(literal newline in the reason),
then the process exits 2 and stderr names `--reason` as the
offending argument.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: SPEC.md hash input excludes `archived_at` and `archived_reason`

The `HASH_EXCLUDED_FRONTMATTER_FIELDS` constant in
`speccy-core/src/parse/spec_md.rs` (which today excludes only
`status`) is extended to also exclude `archived_at` and
`archived_reason`. Consequently, the
`canonical_frontmatter_for_hash` function skips both keys when
building the byte stream that feeds `Sha256`, and `SpecMd.sha256` is
invariant across archival.

Unarchival (a manual `git mv` back into `.speccy/specs/`) leaves the
sha256 unchanged whether or not the user strips the `archived_at` /
`archived_reason` fields. Cleanup is cosmetic.

<done-when>
- `HASH_EXCLUDED_FRONTMATTER_FIELDS` in
  `speccy-core/src/parse/spec_md.rs` contains the three strings
  `"status"`, `"archived_at"`, `"archived_reason"`.
- A unit test parses a SPEC.md fixture twice — once before adding
  `archived_at: 2026-05-23\narchived_reason: "test"` to the
  frontmatter, once after — and asserts the two `SpecMd.sha256`
  values are byte-equal.
- A unit test parses a SPEC.md fixture twice — once with `status:
  implemented`, once with `status: dropped` (and the matching list
  of supersedes) — and asserts the sha256 stays equal (regression
  guard for the existing `status` exclusion that ships alongside
  the new ones).
- `cargo doc -p speccy-core` mentions `archived_at` and
  `archived_reason` in the hash-exclusion documentation block.
</done-when>

<behavior>
- Given a SPEC.md with frontmatter
  `id: SPEC-0001\nslug: x\ntitle: Y\nstatus: implemented\ncreated:
  2026-01-01\nsupersedes: []` and no archive fields, when its
  sha256 is computed, then the value is some `H1`.
- Given the same SPEC.md with the two archive fields appended to
  the frontmatter (`archived_at: 2026-05-23\narchived_reason:
  "foo"`), when its sha256 is recomputed, then the value is also
  `H1`.
- Given a SPEC.md where the body bytes change by even one byte,
  when its sha256 is recomputed, then the value differs from `H1`
  (the body is still hashed; only the named frontmatter fields are
  excluded).
</behavior>

<scenario id="CHK-010">
Given a unit-test fixture SPEC.md with status `implemented` and no
archive fields,
when `SpecMd::parse_raw` returns a `SpecMd` with `sha256 = H1`,
and a second fixture identical except for added
`archived_at: 2026-05-23\narchived_reason: "any"` lines after
`supersedes: []`,
when its `SpecMd::parse_raw` returns sha256 `H2`,
then `H1 == H2` byte-equal.
</scenario>

<scenario id="CHK-011">
Given the same first fixture and a third fixture identical except
that one byte of the body text changed,
when both are parsed,
then their sha256 values differ (regression: body is still hashed).
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Archived specs retain their IDs; `speccy vacancy` scans both `.speccy/specs/` and `.speccy/archive/`

Archiving a spec does not free its SPEC-NNNN slot for reuse by a
future `speccy vacancy --json`. The vacancy resolver scans both
`.speccy/specs/` and `.speccy/archive/` when enumerating taken IDs,
returning the smallest unused ID across the union as
`next_spec_id`.

<done-when>
- `speccy vacancy --json` on a workspace where `.speccy/specs/`
  contains `0001-foo/`, `0003-bar/` and `.speccy/archive/`
  contains `0002-baz/` returns `next_spec_id: SPEC-0004` (not
  `SPEC-0002`).
- A unit test in `speccy-core` constructs a fake workspace with
  this layout, invokes the vacancy logic, and asserts the result.
- The vacancy resolver's discovery glob covers
  `.speccy/specs/*/SPEC.md` AND `.speccy/archive/*/SPEC.md`.
- The vacancy text-mode output (without `--json`) reflects the
  same semantics — the human-readable line names the next free ID
  computed across both directories.
</done-when>

<behavior>
- Given an empty `.speccy/archive/` and `.speccy/specs/` containing
  IDs 0001 through 0041 with no gaps, when `speccy vacancy --json`
  runs, then it returns `SPEC-0042`.
- Given the same `.speccy/specs/` (0001–0041 contiguous) and
  `.speccy/archive/` containing `0042-foo/`, when `speccy vacancy
  --json` runs, then it returns `SPEC-0043` (not `SPEC-0042`).
- Given a workspace where the most recent archive is `SPEC-0050`
  and `.speccy/specs/` contains 0001–0049 contiguous, when
  vacancy runs, then it returns `SPEC-0051`.
</behavior>

<scenario id="CHK-012">
Given a built `speccy` binary at HEAD and a workspace where
`.speccy/specs/` contains exactly `0001-foo/` and `.speccy/archive/`
contains exactly `0002-bar/`,
when `speccy vacancy --json` runs,
then stdout contains `"next_spec_id":"SPEC-0003"`.
</scenario>

<scenario id="CHK-013">
Given the same binary,
when a SPEC is archived from `0001-foo/` and then `speccy vacancy
--json` runs immediately after,
then the returned `next_spec_id` is unchanged from what it would
have been pre-archive (the archived spec still occupies its slot).
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Hot-path commands ignore `.speccy/archive/` by default

The commands `speccy status`, `speccy next`, `speccy check`,
`speccy verify`, and `speccy lock` do not discover or operate on
specs under `.speccy/archive/`. Their workspace-scan logic globs
only `.speccy/specs/*/SPEC.md`; the archive directory is never
opened, never linted, never reflected in their JSON or text output.
`speccy vacancy` is the only command that scans archive (per REQ-005).

<done-when>
- `speccy status` after archiving SPEC-0001 prints zero references
  to `SPEC-0001` in its text and zero entries for `SPEC-0001` in
  its `--json` `specs` array.
- `speccy next --json` after archiving every active spec prints an
  empty `specs` array (no archived specs are surfaced).
- `speccy check SPEC-0001` after archiving exits non-zero with a
  "spec not found" message (the selector resolves against
  `.speccy/specs/` only).
- `speccy verify` after archiving every shipped spec exits 0 on a
  workspace that previously verified — no archived spec's TASKS.md
  hash drift, journal completeness, or REPORT.md proof shape is
  re-checked.
- `speccy lock` after archiving SPEC-0001 leaves the relocated
  `SPEC.md` and `TASKS.md` untouched (lock does not enumerate
  archive paths).
</done-when>

<behavior>
- Given a workspace where SPEC-0001 has just been archived, when
  `speccy status --json` runs, then no entry in the returned
  `specs` array has `id == "SPEC-0001"`.
- Given the same workspace, when `speccy check SPEC-0001` runs,
  then the process exits non-zero with stderr naming the missing
  selector (the archived path is not considered).
- Given a workspace with SPEC-0001 archived and SPEC-0002 active
  with one `state="in-progress"` task, when `speccy next --json`
  runs, then the returned `next_action` resolves against SPEC-0002
  only and the JSON's `specs` array contains only SPEC-0002.
</behavior>

<scenario id="CHK-014">
Given a built `speccy` binary at HEAD and a workspace where
SPEC-0001 has been archived (relocated to `.speccy/archive/`),
when `speccy status --json` runs,
then `jq '.specs | map(select(.id == "SPEC-0001")) | length'` over
the output returns `0`.
</scenario>

<scenario id="CHK-015">
Given the same workspace,
when `speccy check SPEC-0001` runs,
then the process exits non-zero and stderr contains a "not found"
message referencing `SPEC-0001`.
</scenario>

<scenario id="CHK-016">
Given a workspace where every active spec passes `speccy verify`
pre-archive, when half of them are archived and `speccy verify`
runs again,
then the process exits 0 (no archived-spec lints surface) and the
JSON output reflects only the still-active specs.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: `speccy status --include-archive` opt-in surfaces archived specs

`speccy status` accepts a new `--include-archive` boolean flag
(default `false`). When set, the command's workspace scan
additionally globs `.speccy/archive/*/SPEC.md` and merges the
results into its `specs` array (text and JSON modes). The flag is
independent of the existing `--all` flag: `--all` broadens the
attention filter on non-archived specs (showing implemented ones
that are otherwise hidden); `--include-archive` opens visibility
into the archive directory itself. The two flags compose
straightforwardly — passing both shows the union.

Archived specs surfaced via `--include-archive` carry their
frontmatter as parsed, including the new `archived_at` and
`archived_reason` fields, in the JSON entry (alongside existing
fields like `id`, `slug`, `status`, `tasks`, `lint`).

No other command (`next`, `check`, `verify`, `lock`, `vacancy`)
accepts `--include-archive`. `vacancy`'s scan is unconditional
per REQ-005.

<done-when>
- `speccy status --help` documents `--include-archive`.
- `speccy status --include-archive --json` on a workspace with
  archived SPEC-0001 includes an entry with `id == "SPEC-0001"`
  in the `specs` array; without `--include-archive` it does not.
- `speccy status --all --include-archive --json` returns the
  union of "active specs that pass `--all`'s broadening" and
  "archived specs". The result is a superset of either flag alone.
- The JSON entry for an archived spec includes `archived_at` and
  (if present) `archived_reason` fields under the spec record;
  the existing `mission_md_path` and other fields are populated
  consistently with active-spec entries.
- `speccy next --include-archive` exits 2 with a clap "unknown
  flag" error (the flag is not added to the other commands).
</done-when>

<behavior>
- Given a workspace with SPEC-0001 archived, when `speccy status
  --json` runs without flags, then `SPEC-0001` is absent from the
  output.
- Given the same workspace, when `speccy status --include-archive
  --json` runs, then `SPEC-0001` is present in the output and the
  entry includes the `archived_at` field.
- Given a workspace with SPEC-0001 archived and SPEC-0002 active
  with `status: implemented`, when `speccy status --all
  --include-archive --json` runs, then both `SPEC-0001` and
  `SPEC-0002` appear; with only `--include-archive`, SPEC-0002 may
  still be hidden under the attention filter unless its tasks
  require attention.
</behavior>

<scenario id="CHK-017">
Given a built `speccy` binary at HEAD and a workspace where
SPEC-0001 has been archived,
when `speccy status --include-archive --json` runs,
then `jq '.specs[] | select(.id == "SPEC-0001") | .archived_at'`
prints a date in `YYYY-MM-DD` form.
</scenario>

<scenario id="CHK-018">
Given the same binary and the same workspace,
when `speccy status --json` runs (no flags),
then `jq '.specs | map(select(.id == "SPEC-0001")) | length'`
prints `0`.
</scenario>

<scenario id="CHK-019">
Given the same binary,
when `speccy next --include-archive` runs,
then the process exits 2 and stderr contains an "unknown flag" or
equivalent clap error naming `--include-archive`.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Archive warns on supersession-chain orphaning; archival proceeds

Before performing the move, the archive command walks the
supersedes graph to detect the "Scenario 2 orphan" case:

1. Read the spec being archived (call it SPEC-Y). Collect its
   `supersedes` list as candidate orphan victims.
2. For each ID `X` in that list:
   a. If SPEC-X exists in `.speccy/specs/` (i.e., is active) AND
      its frontmatter `status` is `superseded`, AND
   b. No other active spec besides SPEC-Y declares `supersedes:
      [..., X, ...]`,

   then SPEC-X is about to become orphaned: still active, still
   marked `superseded`, but with no active spec declaring the
   supersession.

For every orphan-victim X detected, the archive command emits a
warning line to stderr naming SPEC-Y, SPEC-X, and noting that
`SPC-006` will fire on SPEC-X after the move. The `--json` output
includes a `warnings: [{ spec: "SPEC-X", reason: "orphaned-supersession" }, ...]`
array. Archival proceeds regardless — the warning is informational.

When SPEC-Y has an empty `supersedes` list, or every X in its list
falls outside the orphan-detection cases (e.g., X is already in
archive, X's status is not `superseded`, or another active spec
also declares the supersession), no warning is emitted.

<done-when>
- A unit test constructs a fake workspace where SPEC-0019 is active
  with `status: superseded` and SPEC-0021 is active with
  `supersedes: [SPEC-0019]` (no other active spec declares it).
  Archiving SPEC-0021 emits a warning naming SPEC-0019; archival
  proceeds.
- A unit test where SPEC-0019 is active with `status: superseded`
  and BOTH SPEC-0021 and SPEC-0022 declare `supersedes:
  [SPEC-0019]`. Archiving SPEC-0021 emits no warning (SPEC-0022
  still names SPEC-0019).
- A unit test where SPEC-0019 is already archived. Archiving
  SPEC-0021 (which still declares `supersedes: [SPEC-0019]`) emits
  no orphan warning (SPEC-0019 is not in the active set; nothing to
  orphan).
- A unit test where SPEC-Y has `supersedes: []`. Archiving SPEC-Y
  emits no orphan warning.
- In every case, archival proceeds after the warning is emitted (or
  not). The warning never blocks the move.
- The `--json` output `warnings` array is empty `[]` (not omitted)
  when no warnings are emitted, so scripted callers can rely on
  the field's presence.
</done-when>

<behavior>
- Given SPEC-Y with `supersedes: [SPEC-X]`, SPEC-X active with
  `status: superseded` and no other declarer, when `speccy archive
  SPEC-Y` runs, then stderr contains a line naming both SPEC-Y and
  SPEC-X, the `--json` `warnings` array contains one entry, and the
  process exits 0 with the move completed.
- Given the same setup plus SPEC-Z also declaring `supersedes:
  [SPEC-X]`, when archive runs, then no orphan warning is emitted
  and `warnings` is empty.
- Given SPEC-X already archived (not in active set), when archive
  runs on SPEC-Y that declares `supersedes: [SPEC-X]`, then no
  warning is emitted regardless of SPEC-X's status.
</behavior>

<scenario id="CHK-020">
Given a built `speccy` binary at HEAD and a workspace where
SPEC-0019 is active with `status: superseded` and SPEC-0021 is
the sole active declarer of `supersedes: [SPEC-0019]`,
when `speccy archive SPEC-0021 --json` runs,
then the process exits 0, stderr contains a warning line naming
both `SPEC-0019` and `SPEC-0021`, and stdout JSON contains
`"warnings": [{"spec":"SPEC-0019","reason":"orphaned-supersession"}]`.
</scenario>

<scenario id="CHK-021">
Given the same binary and a workspace where SPEC-0019 is the
"older" spec with `status: superseded` and SPEC-0021 is the active
superseding spec,
when `speccy archive SPEC-0019 --json` runs (archiving the older
one, the natural case),
then the process exits 0 and stdout JSON contains
`"warnings": []`.
</scenario>

<scenario id="CHK-022">
Given a workspace where SPEC-0019 is active with `status:
superseded` and BOTH SPEC-0021 and SPEC-0022 declare `supersedes:
[SPEC-0019]`,
when `speccy archive SPEC-0021 --json` runs,
then `warnings` is `[]` in the JSON output.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: `speccy archive --json` emits a stable receipt shape

When `--json` is passed, stdout is a single JSON document with the
following shape on success:

```json
{
  "schema_version": 1,
  "archived": {
    "id": "SPEC-NNNN",
    "slug": "...",
    "from": ".speccy/specs/NNNN-slug",
    "to": ".speccy/archive/NNNN-slug",
    "archived_at": "YYYY-MM-DD",
    "archived_reason": "..." | null
  },
  "warnings": [
    { "spec": "SPEC-X", "reason": "orphaned-supersession" }
  ]
}
```

`archived_reason` is JSON `null` when the `--reason` flag was not
passed. `warnings` is always an array (empty `[]` when no warnings).
The `schema_version` integer follows the same pattern as `speccy
status --json` and `speccy vacancy --json`.

On failure (status gate refused, spec not found, parse error,
filesystem error), the process exits non-zero and no JSON document
is written to stdout; the error message goes to stderr.

<done-when>
- `speccy archive SPEC-NNNN --json` on success prints exactly one
  JSON object to stdout, matching the shape above, parseable by
  `jq`.
- `archived_reason` is JSON `null` when `--reason` is absent;
  otherwise it is a JSON string.
- `warnings` is `[]` (not omitted, not `null`) when no orphan
  warning fires.
- On status-gate refusal (no `--force` on `in-progress`), stdout is
  empty and stderr carries the human-readable error.
- A unit test parses the JSON output and asserts each field's type
  and value against a constructed fixture.
</done-when>

<behavior>
- Given a successful archive without `--reason`, when `--json` is
  passed, then `archived_reason` in the output is `null`.
- Given a successful archive with `--reason "x"`, when `--json` is
  passed, then `archived_reason` is `"x"`.
- Given a refused archive (status gate), when `--json` is passed,
  then stdout is empty and the process exits non-zero.
</behavior>

<scenario id="CHK-023">
Given a built `speccy` binary at HEAD and a workspace where SPEC-0001
has `status: implemented`,
when `speccy archive SPEC-0001 --json --reason "ship cleanup"` runs,
then stdout JSON parses successfully and
`jq -r '.archived.archived_reason'` prints `ship cleanup` and
`jq -r '.archived.to'` prints `.speccy/archive/0001-artifact-parsers`.
</scenario>

<scenario id="CHK-024">
Given the same binary and a workspace with SPEC-0042 still
`in-progress`,
when `speccy archive SPEC-0042 --json` runs (no `--force`),
then stdout is empty, the process exits non-zero, and stderr
contains the status-gate message.
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: `AGENTS.md`'s V1.0-outcome CLI-surface line becomes count-agnostic

The line under `## Product north star` → `### V1.0 outcome` in
`AGENTS.md` that today reads "Seven-command Rust CLI implementing
the surface in `docs/ARCHITECTURE.md`: `init`, `status`, `next`,
`check`, `verify`, `lock`, `vacancy`." is rewritten to no longer pin
a specific command count. The replacement phrasing names speccy's
CLI as deliberately lean and points at `docs/ARCHITECTURE.md` for
the current command surface, without committing to a numeric count
that this SPEC (and likely future small additions) would otherwise
have to keep amending.

The "stay-small" principle under `## Core principles` retains its
substance (caution against gratuitous CLI growth) but its phrasing
is also rewritten to drop the literal `seven commands` integer.
Otherwise the next CLI verb that lands would re-introduce the same
count-pinning churn this SPEC is trying to eliminate. The same
sweep covers `README.md` and stale `seven-command` / `seven-verb`
prose in `docs/ARCHITECTURE.md`.

`docs/ARCHITECTURE.md`'s CLI section is updated to list the new
`archive` command alongside the existing seven, with a short
description matching the conventions of the existing entries.

<done-when>
- `rg -n 'Seven-command' AGENTS.md` prints zero matches after this
  SPEC lands.
- `rg -n '\barchive\b' docs/ARCHITECTURE.md` prints at least one
  match inside the CLI command list section, with adjacent prose
  describing the command per the conventions of the other six
  entries.
- The replacement phrasing in `AGENTS.md` mentions
  `docs/ARCHITECTURE.md` as the authoritative surface, and includes
  no specific integer count of CLI commands.
- The `## Core principles` section's "Stay small" item retains its
  principle (caution against gratuitous CLI growth) but no longer
  names a specific integer command count.
- `rg -n 'seven[- ]command|seven[- ]verb|seven commands|seven verbs'
  AGENTS.md README.md docs/ARCHITECTURE.md` prints zero matches
  after this SPEC lands (the sweep covers the three top-level docs;
  archived `.speccy/specs/` and historical journals are out of
  scope).
</done-when>

<behavior>
- Given the source tree at HEAD after this SPEC lands, when a
  reader scans `AGENTS.md`'s `### V1.0 outcome` section, then no
  specific number of CLI commands is named.
- Given the same checkout, when a reader scans
  `docs/ARCHITECTURE.md`'s CLI surface section, then the `archive`
  command is documented alongside the existing seven.
</behavior>

<scenario id="CHK-025">
Given the source tree at HEAD,
when `rg -n 'Seven-command|seven-command|7-command' AGENTS.md` runs,
then it prints zero matches.
</scenario>

<scenario id="CHK-026">
Given the same checkout,
when `rg -nU 'speccy archive' docs/ARCHITECTURE.md` runs,
then it prints at least one match inside the CLI command surface
section.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
**Frontmatter mutation over sidecar `.archived.toml`.** The
archival metadata lives in SPEC.md frontmatter as `archived_at` and
`archived_reason`, not in a sibling file. Rationale: keeping the
archive metadata co-located with the spec it describes minimizes
the "where do I look" surface for a maintainer reading an archived
spec, and the hash exclusion in REQ-004 neutralizes the only real
cost (sha256 drift on archived specs). A sidecar file would have
required a new file convention and a new parser path for what is,
fundamentally, two more fields of static metadata.
</decision>

<decision id="DEC-002">
**Flat archive layout preserving the canonical `NNNN-slug/` name.**
Archive directory layout is `.speccy/archive/NNNN-slug/`, with no
date prefix and no year-bucketed parent. Rationale: identity
portability across the active/archived boundary matters more than
visual scan-by-date. Tools that resolve "where is SPEC-0042" can
simply check `.speccy/specs/` first and fall back to
`.speccy/archive/`; with a date-prefixed rename, the tool would
need to glob `*-0042-*` and accept any date prefix, and commit
messages referencing `SPEC-0042-foo` as a path hint would silently
rot. Year-bucketing is premature at v1's solo-developer scale; if
the archive grows past a few hundred entries, a future SPEC can
reorganize.
</decision>

<decision id="DEC-003">
**CLI subcommand over skill-only orchestration.** Archival lives in
the Rust binary, not as a markdown-only skill that calls `speccy
status --json` and orchestrates `git mv`. Rationale: the
status-gate enforcement (REQ-002) and the orphan-detection logic
(REQ-008) must behave identically across host packs (Claude Code,
Codex, and any future host). Markdown-body discipline is fragile;
Rust enforcement is consistent. The cost is the eighth CLI verb,
addressed by REQ-010's `AGENTS.md` revision.
</decision>

<decision id="DEC-004">
**Singular archive; no `--all` or filter flags in v1.** The
command takes exactly one positional `SPEC-NNNN`. Today's bootstrap
case (41 specs to archive) is handled by a shell loop:
`for id in $(speccy status --json | jq -r '.specs[] | select(.status == "implemented") | .id'); do speccy archive $id; done`.
Rationale: bulk semantics raise questions (atomic vs. best-effort,
filter language, glob semantics) that a one-shot v1 doesn't need
to settle. If repeated dogfood use surfaces a real pain point, a
follow-up SPEC can add `--all` or filter flags with the benefit of
real signal.
</decision>

<decision id="DEC-005">
**`in-progress` is the only blocked status; `--force` is pure
bypass.** The status gate refuses only `in-progress`. The three
archivable statuses (`implemented`, `dropped`, `superseded`) all
mean "no further loop work expected"; archival is the natural
follow-up. `--force` bypasses the gate without mutating any
frontmatter — in particular, a `--force`-archived `in-progress`
spec stays `in-progress` in frontmatter. Rationale: the user
opted in to the bypass; auto-mutating `status` to `dropped` would
silently rewrite their intent under their feet. If they want to
mark it dropped, they edit SPEC.md and archive normally.
</decision>

<decision id="DEC-006">
**Warn-not-block on supersession-chain orphaning.** When archival
would leave an active `status: superseded` spec without any active
declarer (Scenario 2), the command emits a warning naming the
affected SPEC-X but proceeds. The `SPC-006` lint already covers
the downstream effect ("status = superseded but no other spec
declares it as a supersedes target"); the archive warning is the
upstream nudge. Rationale: blocking would force users to either
also archive the orphan (which they may not want) or to first
mutate the orphan's `status` field (which is an edit unrelated to
the archive). A warning surfaces the consequence without dictating
the resolution.
</decision>

<decision id="DEC-007">
**No `speccy unarchive` subcommand.** The escape hatch for
erroneous archival is manual `git mv .speccy/archive/NNNN-slug
.speccy/specs/`. Rationale: unarchival is rare (the natural
archival pattern produces correct results almost always), and the
hash exclusion in REQ-004 makes manual cleanup of the
`archived_at` / `archived_reason` fields purely cosmetic. Adding a
ninth CLI verb to handle the rare reverse direction violates the
"stay small" principle without observable benefit. If unarchival
turns out to be common in practice, a follow-up SPEC reverses
this decision with the benefit of real signal.
</decision>

## Notes

The implementation work this SPEC entails spans both
`speccy-core` (workspace scanning, hash exclusion, vacancy logic,
the supersedes-orphan detection algorithm) and `speccy-cli` (the
new subcommand, flag parsing, JSON output shape, the `--include-archive`
status flag, and the `AGENTS.md` + `docs/ARCHITECTURE.md` edits).
Decomposition into tasks happens in the next phase
(`/speccy-tasks SPEC-0042`).

The bootstrap migration of today's 41 implemented specs is not a
requirement of this SPEC — it is a user action taken after the
command ships. A maintainer running `speccy archive SPEC-0001` ...
through `SPEC-0041` (via shell loop) is the natural first use of
the feature.

<assumptions>
- The user's claim that `speccy status` already hides completed
  specs behind an attention filter, with an existing `--all` flag
  to override, is consistent with `speccy-cli/src/status.rs` at
  HEAD (verified during planning). REQ-007's framing of
  `--include-archive` as additive-with-`--all` rests on that
  behavior remaining intact.
- `git mv` is available in every environment where `speccy` runs,
  because the workspace lives in a git repo by `speccy init`'s
  convention. The archive command shells out to `git mv` (or its
  Rust equivalent via the existing tooling Speccy already uses for
  filesystem operations) rather than calling `std::fs::rename`
  directly, so git's rename detection works in subsequent commits.
- The `SPC-006` lint exists today in `speccy-core` and fires on
  active specs whose `status: superseded` is not matched by any
  active spec's `supersedes:` list. REQ-008's warning text
  ("`SPC-006` will fire on SPEC-X after the move") refers to this
  exact lint; if the lint family is renamed in a future SPEC, the
  archive command's stderr text follows suit.
- Manual unarchival (the reverse `git mv`) is rare enough in
  practice that not providing a CLI affordance for it is acceptable
  v1 friction. If dogfood signal proves this assumption wrong, a
  follow-up SPEC introduces `speccy unarchive` (per DEC-007's
  escape clause).
- The "archive is a location, not a deletion" framing — archived
  specs retain their IDs, their artifacts, and the ability to be
  referenced by `supersedes:` from active specs — is preserved by
  the design and is not a goal to be re-tested per se; it
  manifests in REQ-005 (ID retention), REQ-008 (orphan
  detection considers archive presence), and the lack of any
  REQ that destroys content.
</assumptions>

## Open Questions

(None.)

## Changelog

<changelog>
| Date       | Reason                                                   | Author     |
|------------|----------------------------------------------------------|------------|
| 2026-05-23 | Initial draft. Add `speccy archive SPEC-NNNN` subcommand that relocates a shipped/dropped/superseded spec from `.speccy/specs/NNNN-slug/` to `.speccy/archive/NNNN-slug/` via `git mv`, preserving the canonical name. Frontmatter gains `archived_at` (UTC date, unconditional) and optional `archived_reason` (only with `--reason`); both fields are excluded from SPEC.md hash input via `HASH_EXCLUDED_FRONTMATTER_FIELDS`. Hot-path commands (`status`, `next`, `check`, `verify`, `lock`) ignore the archive directory; `vacancy` scans it because archived specs retain their IDs. `speccy status --include-archive` opts archived specs back into the status output. Archival warns on supersession-chain orphaning (Scenario 2) but proceeds. No `speccy unarchive` and no bulk archive in v1; manual `git mv` is the reverse-escape hatch and a shell loop handles bulk. `AGENTS.md`'s "Seven-command Rust CLI" line is rephrased to drop the specific command count. | Kevin Xiao |
| 2026-05-23 | REQ-010 amendment: extend the count-pin sweep to the `## Core principles` "Stay small" item in `AGENTS.md`, the equivalent "Stay small" item in `README.md`, and stale `seven-command` / `seven-verb` prose in `docs/ARCHITECTURE.md`. The original done-when item 4 ("Stay small unmodified by this SPEC") is replaced by a count-agnostic rewrite of the same line; a new done-when adds an `rg` check across all three top-level docs. Reason: the next CLI verb that lands would re-introduce the same churn the SPEC is trying to eliminate, so the sweep is scoped now rather than deferred to a follow-up SPEC. | Kevin Xiao |
</changelog>
