---
id: SPEC-0024
slug: meaningful-hash-semantics
title: Meaningful hash semantics and per-spec status selectors
status: implemented
created: 2026-05-17
supersedes: []
---

# SPEC-0024: Meaningful hash semantics and per-spec status selectors

## Summary

Three concerns share one root cause: `speccy status` produces signal
that does not mean what it says.

First, `SpecMd.sha256` (`parse/spec_md.rs:149`) hashes the entire raw
file bytes including frontmatter. That means cosmetic frontmatter
edits — most commonly the `status: in-progress` → `status: implemented`
flip at ship time — invalidate the stored
`spec_hash_at_generation` in TASKS.md even though no substantive
content drifted. Evidence: the SPEC-0021 ship commit (`ead80f8`)
explicitly says "refresh TASKS.md's spec_hash_at_generation to match"
after the status flip. The workflow is paying a tax for a hash that
covers the wrong scope.

Second, there is no cross-check between folder digits (`0024-...`),
SPEC.md frontmatter `id:`, and TASKS.md frontmatter `spec:`. A file
moved to the wrong folder, or a copy-paste error in either
frontmatter, would silently produce a workspace where the three
identifiers disagree. The existing `commit_frontmatter` guard
(`speccy-core/src/tasks.rs:89`) catches CLI-arg vs TASKS.md
disagreement but not folder vs SPEC.md vs TASKS.md.

Third, `speccy status` always renders the workspace-wide attention
list (or nothing) and offers no per-spec view. Operators who want to
inspect one spec must either grep the text output or pipe `--json`
through `jq`. The same filtering that hides clean implemented specs
is silent: no footer hint, no way to opt out short of `--json`.

This spec rewrites the SPEC.md content hash to cover body + frontmatter
minus `status`, adds a 3-way ID consistency check (lint rule plus
command-guard at hash-write time), and extends `speccy status` with a
positional `SPEC-NNNN` selector plus an `--all` flag. Migration is a
manual reconciliation task that runs after the implementation work
lands; no CLI migration command is added.

## Goals

<goals>
- `SpecMd.sha256` hashes the SPEC.md body (bytes after the closing
  frontmatter fence) plus a canonical re-serialization of the parsed
  frontmatter with `status` excluded. Status flips and other
  status-only edits no longer cause `hash-drift`.
- The canonical re-serialization is deterministic across runs and
  Rust toolchain versions: a stable YAML emitter with sorted keys
  produces the same bytes for the same parsed frontmatter regardless
  of source-file whitespace, key order, or comments.
- Future frontmatter fields contribute to the hash by default. The
  exclusion list is hard-coded and contains only `status`. Adding a
  new exclusion requires editing this list (and a SPEC amendment).
- A new lint rule fires on any mismatch among (folder digits derived
  via `derive_spec_id_from_dir`), SPEC.md frontmatter `id:`, and
  TASKS.md frontmatter `spec:`. Level: error.
- `speccy tasks SPEC-NNNN --commit` refuses to persist
  `spec_hash_at_generation` when the same 3-way check fails, with a
  distinct error variant that names which identifier disagrees.
- `speccy status SPEC-NNNN` renders only that spec, unfiltered, in
  both text and JSON modes.
- `speccy status --all` renders every spec, unfiltered, in both text
  and JSON modes. With `--json`, the output shape matches today's
  `--json` (every spec, no filter).
- `speccy status` with no arguments keeps the current attention-list
  behavior, but appends a one-line footer naming how many specs were
  hidden and how to see them (`{N} specs hidden; pass --all to see
  them`). The footer is only rendered when N > 0.
- Existing TASKS.md `spec_hash_at_generation` values across this
  repo are reconciled to the new hash function as part of this
  spec's implementation tasks. After reconciliation, `speccy status`
  reports zero specs as `hash-drift` stale.
</goals>

## Non-goals

<non-goals>
- No CLI migration command. The hash regeneration is a one-shot
  reconciliation done by the AI agent in the implementation task
  list (one TASKS.md edit per spec, verified by `speccy status`).
  Building a `speccy migrate-hashes` for an event that happens once
  is dead weight after this spec lands.
- No parse-time enforcement of the 3-way ID check. `speccy status`,
  `speccy verify`, and the lint engine must continue to function
  (and surface the mismatch) when IDs disagree. Hard-failing at
  parse time would block the diagnostic surface that flags the
  problem.
- No change to the location of `spec_hash_at_generation`. The
  field continues to live in TASKS.md frontmatter, not SPEC.md.
- No change to the existing `commit_frontmatter` two-way check
  between CLI argument and TASKS.md `spec:`. The new 3-way check
  composes on top; the existing variant stays for backward
  compatibility with that error path.
- No change to which other staleness reasons exist
  (`HashDrift`, `MtimeDrift`, `BootstrapPending`). Only the
  computation of `HashDrift` changes.
- No change to the `show_in_text_view` filter logic for the
  no-args case. The filter stays; the footer just announces it.
- No JSON schema bump. The shape of `--json` and the JsonSpec
  fields are unchanged. Adding the positional selector restricts
  the rendered set but does not alter field shapes. `schema_version`
  stays at 1.
- No new staleness signal for "frontmatter-only edit since
  hash committed." Once the hash scope excludes `status`, that
  signal is intentionally invisible; if a future need arises to
  detect frontmatter-only edits, that lands in its own spec.
- No removal or rewrite of the existing `SpecIdMismatch` variant
  on `CommitError`. The new variant is a peer.
</non-goals>

## User Stories

<user-stories>
- As a developer shipping a spec, I want to flip `status:
  in-progress` → `status: implemented` without spuriously
  invalidating every TASKS.md hash. Today the workflow pays a
  hand-reconcile tax on every ship; after this spec, status flips
  are hash-neutral.
- As a developer running `speccy status` to see the workspace, I
  want a per-spec selector so I can inspect SPEC-0023 without
  scrolling through the other 22 specs' output.
- As a developer running `speccy status` and seeing nothing, I want
  to know whether the workspace is genuinely clean or just filtered.
  The footer makes the filter visible without forcing me to read the
  code.
- As a developer who has moved a spec folder or hand-edited
  frontmatter and corrupted the ID alignment, I want the lint rule
  to surface the mismatch loudly and the `speccy tasks --commit`
  guard to refuse to persist a hash against a broken triple.
- As an AI agent following the migration tasks for this spec, I want
  the verification step to be a single command (`speccy status`)
  that proves zero `hash-drift` across the workspace; no per-spec
  spot-checking.
- As a future reader of this codebase, I want the hash semantics
  documented in the SPEC.md so I do not have to reverse-engineer
  why the hash excludes `status` from the parse function alone.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Hash excludes `status` and covers body + canonical frontmatter

`SpecMd.sha256` hashes the SPEC.md body bytes (everything after the
closing `---` fence) plus a canonical re-serialization of the parsed
frontmatter with the `status` field omitted. The re-serialization
uses a stable YAML emitter with sorted keys, so the hash is
independent of source-file whitespace, key order, and comments.

<done-when>
- `speccy-core/src/parse/spec_md.rs` computes `sha256` as
  `Sha256::digest(canonical_frontmatter_bytes || body_bytes)` where
  `body_bytes` is everything after the closing `---` fence (inclusive
  of the trailing newline that follows the fence) and
  `canonical_frontmatter_bytes` is the YAML serialization of the
  parsed `SpecFrontmatter` struct minus `status`, with keys sorted
  alphabetically and a fixed line ending (`\n`).
- The canonical serialization is implemented via a helper function
  in `parse/spec_md.rs` (or a small new module under `parse/`) that
  takes a `&SpecFrontmatter` and returns the canonical bytes.
- The function is total over valid `SpecFrontmatter` values; it
  returns `Result` only if a serializer fails (treated as an internal
  error, not user-facing).
- The hash field exclusion list is a private `const &[&str]` in
  `parse/spec_md.rs` named `HASH_EXCLUDED_FRONTMATTER_FIELDS`
  containing exactly `["status"]`. Adding a new exclusion requires
  editing this constant.
- Unit tests in `parse/spec_md.rs` assert:
  - Flipping `status: in-progress` → `status: implemented` on
    otherwise-identical SPEC.md produces the same `sha256`.
  - Editing the body (any byte change after the closing fence)
    produces a different `sha256`.
  - Editing any non-`status` frontmatter field (`slug`, `title`,
    `created`, `supersedes`, `id`) produces a different `sha256`.
  - Reordering frontmatter keys in the source file produces the
    same `sha256`.
  - Adding or removing whitespace before/after the closing `---`
    fence does not affect the canonical re-serialization for the
    frontmatter portion (whitespace in the body still affects the
    body bytes).
- `SpecMd.raw` continues to hold the unmodified raw file bytes for
  callers that need the source text. Only `SpecMd.sha256`'s
  computation changes.
- The doc comment on `SpecMd.sha256` is updated to describe the new
  scope: "sha256 of canonical(frontmatter \ {status}) ++ body bytes.
  Stable across status flips and frontmatter cosmetics."
</done-when>

<behavior>
- Given two SPEC.md files identical except for `status`, when each
  is parsed, then `SpecMd.sha256` returns the same 32-byte digest.
- Given two SPEC.md files identical except for one byte of body
  content, when each is parsed, then `SpecMd.sha256` returns
  different digests.
- Given two SPEC.md files with the same parsed frontmatter (sans
  status) and body but different source-file key ordering, when each
  is parsed, then `SpecMd.sha256` returns the same digest.
- Given a SPEC.md with a `supersedes: []` field and an otherwise-
  identical SPEC.md with no `supersedes` line (defaulted to empty),
  when each is parsed, then `SpecMd.sha256` returns the same digest
  because the parsed `SpecFrontmatter.supersedes` is identical in
  both cases.
</behavior>

<scenario id="CHK-001">
Given a SPEC.md fixture with `status: in-progress` and a copy with
`status: implemented` (otherwise byte-identical),
when both are parsed,
then `SpecMd.sha256` is equal.

Given a SPEC.md fixture and a copy with one extra newline appended
to the body,
when both are parsed,
then `SpecMd.sha256` differs.

Given a SPEC.md fixture and a copy where `slug` and `title` lines
have been swapped in source order,
when both are parsed,
then `SpecMd.sha256` is equal (canonical serialization sorts keys).

Given a SPEC.md fixture and a copy where `slug` has been changed,
when both are parsed,
then `SpecMd.sha256` differs.

Given the constant `HASH_EXCLUDED_FRONTMATTER_FIELDS` in
`parse/spec_md.rs`,
when read,
then it contains exactly the single entry `"status"`.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: 3-way ID consistency lint rule

A new lint rule fires when the three identifiers — folder digits
(via `derive_spec_id_from_dir`), SPEC.md frontmatter `id:`, and
TASKS.md frontmatter `spec:` — do not all agree. Level: error.

<done-when>
- A new lint code is added under the appropriate family. Naming:
  `TSK-005` if the existing TSK numbering continues (after
  TSK-001, TSK-003, TSK-004), or a new code in whichever family
  the implementer judges most natural; the choice is documented
  in the implementer's commit message and the SPEC.md Notes
  section if it deviates from `TSK-005`.
- The rule fires once per spec when the triple disagrees,
  reporting which identifier(s) are out of step.
- The rule's `Diagnostic` carries `spec_id` set to the SPEC.md
  frontmatter `id:` (if parseable; otherwise the folder-derived
  ID as fallback, so the diagnostic groups correctly in `speccy
  status` output).
- The rule's message names all three values explicitly so a reader
  can tell which one to fix (e.g., "folder=`SPEC-0024`,
  SPEC.md.id=`SPEC-1234`, TASKS.md.spec=`SPEC-0024`").
- The rule is skipped (not fired) when any of the three is
  unobtainable: SPEC.md missing/unparseable (folder ID still
  derivable, TASKS.md spec still readable), TASKS.md missing, or
  folder digits not in the `^\d{4}-` shape. Upstream parse-error
  diagnostics already cover those failure modes; the 3-way rule
  exists to catch the case where all three parse but disagree.
- Unit tests in the lint engine assert: fires on mismatch; does
  not fire on agreement; does not fire when TASKS.md is absent;
  does not fire when SPEC.md is unparseable; the diagnostic
  message contains all three observed values.
- The rule is registered with the existing lint runner so it
  appears in `speccy status` and `speccy verify` output without
  any caller change.
</done-when>

<behavior>
- Given a spec where folder digits, SPEC.md `id:`, and TASKS.md
  `spec:` all match, when lint runs, then the new rule emits no
  diagnostic.
- Given a spec where SPEC.md `id:` says `SPEC-1234` but the folder
  is `0024-...` and TASKS.md `spec:` says `SPEC-0024`, when lint
  runs, then the new rule emits one error diagnostic naming all
  three observed values.
- Given a spec with no TASKS.md, when lint runs, then the new rule
  emits no diagnostic (it has no third observation to compare).
- Given a spec whose SPEC.md fails to parse, when lint runs, then
  the new rule emits no diagnostic (parse-error rules surface the
  upstream problem).
</behavior>

<scenario id="CHK-002">
Given a workspace fixture with a spec whose folder digits, SPEC.md
`id:`, and TASKS.md `spec:` all read `SPEC-0001`,
when lint runs,
then the new rule emits no diagnostic for that spec.

Given a workspace fixture with a spec whose folder is `0024-…`,
SPEC.md `id: SPEC-1234`, and TASKS.md `spec: SPEC-0024`,
when lint runs,
then exactly one diagnostic from the new rule is emitted, its
level is `Error`, and its message contains the literal strings
`SPEC-0024`, `SPEC-1234`, and `SPEC-0024` so the operator can see
the discrepancy.

Given a workspace fixture with a spec that has no TASKS.md,
when lint runs,
then the new rule emits no diagnostic (existing rules cover the
missing-TASKS case).

Given a workspace fixture with a spec whose SPEC.md is unparseable,
when lint runs,
then the new rule emits no diagnostic (SPC parse-error rules cover
the upstream failure).

Given the lint rule registry,
when grep'd for the new code,
then the rule is registered and its severity is `Level::Error`.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Hash-write command-guard at `speccy tasks --commit`

`speccy tasks SPEC-NNNN --commit` refuses to persist
`spec_hash_at_generation` when the 3-way ID check fails. The error
is distinct from the existing `SpecIdMismatch` variant and names
which identifier is out of step. TASKS.md is not modified.

<done-when>
- `CommitError` (`speccy-core/src/tasks.rs:25`) gains a new variant
  named `IdTripleMismatch` (or equivalent) carrying the three
  observed values (folder-derived ID, SPEC.md `id:`, TASKS.md
  `spec:`) and the message form: "ID disagreement among
  folder=`{folder}`, SPEC.md.id=`{spec_md}`, TASKS.md.spec=
  `{tasks_md}`; refusing to commit (file untouched)".
- `commit_frontmatter` (or its CLI caller in
  `speccy-cli/src/tasks.rs`) performs the 3-way check before any
  write. On disagreement, it returns the new error variant; the
  TASKS.md file is not opened for writing.
- The check is performed in addition to the existing CLI-arg vs
  TASKS.md-spec check. If both checks would fail, the new
  3-way error fires (it is a superset signal).
- The guard does not depend on the new lint rule's wiring; it is
  computed in-line in the `--commit` path so a future lint
  refactor does not weaken the persistence guard.
- Unit tests in `speccy-core/src/tasks.rs` assert: returns the new
  error variant on disagreement; TASKS.md is byte-unchanged on
  disagreement; succeeds and writes when all three agree.
- The CLI surfaces the error message verbatim (no rewrap) so the
  three observed values land in the user's terminal.
</done-when>

<behavior>
- Given `speccy tasks SPEC-0024 --commit` invoked in a workspace
  where folder is `0024-…`, SPEC.md `id: SPEC-0024`, TASKS.md
  `spec: SPEC-0024`, when the command runs, then the hash is
  written and the command exits 0.
- Given `speccy tasks SPEC-0024 --commit` invoked where SPEC.md
  `id: SPEC-1234` (folder and TASKS.md agree on `SPEC-0024`),
  when the command runs, then the command exits non-zero with the
  new error variant and TASKS.md is byte-unchanged on disk.
- Given `speccy tasks SPEC-0024 --commit` invoked where TASKS.md
  `spec: SPEC-9999` (folder and SPEC.md agree on `SPEC-0024`),
  when the command runs, then the command exits non-zero with the
  new error variant; the existing `SpecIdMismatch` variant may
  also be reachable here depending on which check fires first, and
  that is acceptable as long as TASKS.md is not modified.
- Given a workspace where all three IDs agree, when the command
  runs, then the existing happy-path behavior (write hash + ts) is
  unchanged byte-for-byte.
</behavior>

<scenario id="CHK-003">
Given a workspace with folder `0024-…`, SPEC.md `id: SPEC-0024`,
and TASKS.md `spec: SPEC-0024`,
when `speccy tasks SPEC-0024 --commit` runs,
then it exits 0 and TASKS.md `spec_hash_at_generation` is updated
to the new hash.

Given a workspace with folder `0024-…`, SPEC.md `id: SPEC-1234`,
and TASKS.md `spec: SPEC-0024`,
when `speccy tasks SPEC-0024 --commit` runs,
then it exits non-zero, prints a message containing all three
observed IDs, and TASKS.md is byte-identical to its pre-command
state.

Given the `CommitError` enum,
when read,
then it contains a new variant distinct from `SpecIdMismatch`
carrying three string fields (folder, spec_md, tasks_md).

Given the new variant's `Display` impl,
when formatted,
then the message contains the literal strings of all three
observed identifiers so the user sees which one is wrong.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `speccy status` per-spec and `--all` selectors

`speccy status` accepts an optional positional `SPEC-NNNN` argument
and an `--all` flag. With a positional, it renders only that spec,
unfiltered. With `--all`, it renders every spec, unfiltered. With
neither, it keeps today's filtered attention-list behavior and adds
a one-line footer naming how many specs were hidden.

<done-when>
- `StatusArgs` (`speccy-cli/src/status.rs:50`) gains two new fields:
  `selector: Option<String>` (the positional `SPEC-NNNN`) and
  `all: bool` (the flag).
- The clap parser at the CLI surface accepts both: a positional
  argument matching `SPEC-\d{4}` and a `--all` flag. Passing both
  is a parse-time error ("specify either SPEC-NNNN or --all, not
  both"). Passing neither is the default attention-list mode.
- An unknown positional (e.g. a spec ID that no spec in the
  workspace claims) returns a clear CLI error: "no spec with id
  `SPEC-NNNN` in workspace; available: SPEC-0001 … SPEC-NNNN" (or
  similar). The error returns non-zero; no partial output is
  printed.
- In text mode, `speccy status SPEC-0023` renders the same per-spec
  block as today's filtered renderer (header + tasks line + lint
  line + optional stale/questions/parse-error lines), regardless of
  whether the spec would otherwise be in the attention list.
- In text mode, `speccy status --all` renders the per-spec block
  for every spec in workspace order, regardless of filter state.
  If the workspace is empty, it prints "No specs in workspace."
  (matching today's empty-workspace message).
- In text mode, `speccy status` (no selector, no `--all`) keeps the
  filtered set today's renderer emits, then appends one footer line
  when at least one spec was filtered out: `{N} specs hidden; pass
  --all to see them`. The footer is suppressed when no specs were
  hidden. The footer is printed after the workspace-lint block (if
  any) and is separated from it by a blank line.
- In JSON mode (`--json`):
  - `speccy status SPEC-NNNN --json` renders a JSON object whose
    top-level shape is unchanged (same fields as today) but whose
    `specs` array contains exactly one entry: the requested spec.
    The same not-found error rule applies (no partial JSON).
  - `speccy status --all --json` renders the same top-level shape
    as today's `--json` (every spec). No `schema_version` bump.
  - `speccy status --json` (no selector, no `--all`) renders the
    same shape as today's `--json` (every spec; JSON ignores the
    text-mode filter). No footer is added to JSON; the JSON shape
    does not change.
- The renderers (`render_text` and `build_json`) are factored so the
  selector and `--all` decisions happen before rendering, not inside
  it. A small selector resolver returns a `Vec<&SpecView<'_>>` to
  render plus a count of hidden specs (for the footer).
- The CLI usage string and `--help` output describe the new
  selector and flag in the same wording as the rest of `speccy`'s
  positional-argument commands (e.g. `speccy check`).
- Unit tests assert: parse rejects both-selectors; unknown spec id
  errors out cleanly; positional renders one spec; `--all` renders
  all; no-args attention list emits the footer when filtered specs
  exist and omits it otherwise; JSON shape is unchanged for `--all`
  and `--json` paths; JSON for positional is `specs: [one]`.
- The integration test that today covers `speccy status` is extended
  to cover the three new paths (positional, `--all`, footer
  presence).
</done-when>

<behavior>
- Given a workspace with 23 specs of which 21 are stale, when
  `speccy status` runs, then today's attention-list output is
  followed by `21 specs hidden; pass --all to see them`. (Numbers
  illustrative; the actual number is computed at runtime.)
- Given a workspace with 23 specs all clean, when `speccy status`
  runs, then today's "No in-progress specs need attention."
  message is printed and no footer is appended.
- Given `speccy status SPEC-0023` in a workspace where SPEC-0023 is
  clean implemented (would otherwise be filtered), when the command
  runs, then the per-spec block for SPEC-0023 is rendered and exit
  code is 0.
- Given `speccy status SPEC-9999` in a workspace with no such spec,
  when the command runs, then the command exits non-zero with a
  message naming the missing ID; no partial output is printed.
- Given `speccy status --all SPEC-0023`, when the command runs,
  then it exits non-zero at parse time naming the conflict.
- Given `speccy status --all --json`, when the command runs, then
  the JSON output is byte-equivalent (modulo whitespace) to today's
  `speccy status --json`.
</behavior>

<scenario id="CHK-004">
Given this repo's `.speccy/specs/` after the migration task in
REQ-005 completes (zero stale specs),
when `speccy status` runs,
then the output is the existing "No in-progress specs need
attention." message and no `specs hidden` footer is printed.

Given a workspace fixture where 3 of 5 specs are stale,
when `speccy status` runs,
then the output ends with the line `2 specs hidden; pass --all to
see them` (assuming 2 implemented-clean specs got filtered).

Given `speccy status SPEC-0023` in this repo (assuming SPEC-0023 is
clean implemented),
when the command runs,
then the per-spec block for SPEC-0023 is rendered (header + tasks
line + lint line, no stale line) and exit code is 0.

Given `speccy status SPEC-9999`,
when the command runs,
then exit code is non-zero, stderr names the missing ID, and
nothing is written to stdout.

Given `speccy status --all SPEC-0023`,
when the command runs,
then the CLI parser rejects the combination at parse time and exit
is non-zero.

Given `speccy status --all --json`,
when the command runs,
then the output is JSON with the same top-level shape as today's
`speccy status --json` (same fields, every spec rendered).

Given `speccy status SPEC-0023 --json`,
when the command runs,
then the output is JSON whose `specs` array has length 1 and whose
sole entry has `id: "SPEC-0023"`.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Manual reconciliation of stored hashes

After REQ-001 lands, every existing TASKS.md's stored
`spec_hash_at_generation` is wrong-shape under the new hash function.
The reconciliation is performed as the final implementation task: an
AI agent computes the new hash for each spec via the new code path
and writes it into the corresponding TASKS.md. Verification is a
single `speccy status` invocation that must show zero `hash-drift`.

<done-when>
- The TASKS.md decomposition for this spec includes a dedicated
  reconciliation task ordered last (after all code-change tasks).
- That task uses the new `SpecMd.sha256` (e.g. via a one-shot
  invocation of `speccy tasks SPEC-NNNN --commit` per spec, or
  equivalent direct write) to update every TASKS.md in
  `.speccy/specs/` to the new hash shape.
- After the reconciliation task completes, `speccy status` (the
  default attention-list mode) reports no spec with `hash-drift` in
  its stale reasons. Specifically, the JSON output's
  `specs[*].stale_reasons` arrays contain no occurrence of the
  string `hash-drift` for any spec.
- The reconciliation does not regenerate `generated_at` timestamps
  unnecessarily. If `speccy tasks --commit` rewrites both fields,
  that is acceptable; if a manual edit is used, only
  `spec_hash_at_generation` need change. The end-state criterion is
  the absence of `hash-drift`, not byte preservation of
  `generated_at`.
- The reconciliation task's handoff note records the count of
  TASKS.md files modified and confirms the `speccy status`
  verification passed.
- No CLI migration command is added. The reconciliation is a
  one-shot agent task documented in TASKS.md, not a recurring
  command.
</done-when>

<behavior>
- Given the implementation tasks for REQ-001 through REQ-004 have
  all completed, when the reconciliation task runs, then every
  TASKS.md with a real (non-`bootstrap-pending`) hash has that hash
  rewritten to the value `SpecMd.sha256` now returns.
- Given the reconciliation task has completed, when `speccy status
  --all --json` is invoked, then no spec's `stale_reasons` contains
  `hash-drift`.
- Given a spec whose TASKS.md frontmatter `spec_hash_at_generation`
  is `bootstrap-pending`, when the reconciliation task runs, then
  that spec is skipped (the existing TSK-002 / lint flow surfaces
  the bootstrap case; the migration task does not auto-resolve it).
</behavior>

<scenario id="CHK-005">
Given the reconciliation task has completed on this repo,
when `speccy status --all --json` is invoked,
then no spec object in the output has the string `hash-drift` in
its `stale_reasons` array.

Given the reconciliation task has completed on this repo,
when `speccy status` is invoked with no selector,
then the output is the "No in-progress specs need attention."
message and exit code is 0.

Given the reconciliation task's handoff note,
when read,
then it records the count of TASKS.md files modified and the
`speccy status` verification result.

Given a spec with TASKS.md `spec_hash_at_generation:
bootstrap-pending` (if any exists at reconciliation time),
when the reconciliation task runs,
then that spec is left untouched and the bootstrap state is
surfaced by the existing lint diagnostics rather than auto-resolved.
</scenario>

</requirement>

## Design

### Approach

Implementation order:

1. Add the canonical frontmatter re-serialization helper and
   `HASH_EXCLUDED_FRONTMATTER_FIELDS` constant in
   `speccy-core/src/parse/spec_md.rs`. Cover with the REQ-001 unit
   tests before changing `SpecMd.sha256`'s body.
2. Switch `SpecMd.sha256`'s computation to
   `Sha256::digest(canonical_frontmatter || body)`. Update the field's
   doc comment. Re-run the workspace test suite; everywhere a stored
   fixture hash is compared, the fixtures need new expected values
   (which the implementer regenerates by running the canonical
   function on the fixture content).
3. Add the new lint rule (REQ-002) under `speccy-core/src/lint/rules/`.
   Wire it into the rule registry. Cover with the REQ-002 unit tests.
4. Add the new `CommitError` variant and the 3-way check in
   `speccy-core/src/tasks.rs` (REQ-003). Cover with the REQ-003 unit
   tests. Verify the CLI surface in `speccy-cli/src/tasks.rs` reports
   the error verbatim.
5. Extend `StatusArgs`, the clap parser, the `render_text` /
   `build_json` paths, and the integration tests for REQ-004.
   Implement the selector resolver as a single function returning
   `(Vec<&SpecView>, hidden_count)`; both renderers consume that.
6. Reconcile every TASKS.md in this repo using the new hash function
   (REQ-005). Verify via `speccy status --all --json` that no
   `hash-drift` remains.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Canonical re-serialization of frontmatter, not byte-strip

**Status:** Accepted

**Context:** Two ways to exclude `status` from the hash: (a) parse
the frontmatter, drop `status`, re-serialize canonically, concat
with body bytes; (b) byte-strip the `status:` line from the raw
file and hash the rest. Option (b) is simpler to write but fragile:
whitespace before/after the line, key order, comments, multiline
values, and any future YAML quirk would change the hash even when
the parsed meaning is unchanged.

**Decision:** Option (a). Parse, drop `status`, re-serialize with
sorted keys via a stable emitter, concat with raw body bytes. The
canonical output is fully a function of the parsed
`SpecFrontmatter` value, so source-file cosmetics are erased.

**Consequences:** Hash stability is tied to the serializer's
determinism. The implementation must use a YAML emitter whose
output is stable across runs and toolchain versions (and ideally
deterministic across the dependency's own patch versions). If a
future serializer dependency change perturbs the canonical output,
that is a coordinated migration — but it would behave correctly
the moment all stored hashes are recomputed against the new
output. The byte-strip alternative would have made that migration
permanent (every comment or whitespace edit forever).
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Default-include for future frontmatter fields

**Status:** Accepted

**Context:** When new frontmatter fields are added (a future
`owner:`, `priority:`, whatever), they could default to "included
in hash" (drift surfaces on edit unless explicitly excluded) or
default to "excluded from hash" (drift is silent unless explicitly
included).

**Decision:** Default-include. The exclusion list
(`HASH_EXCLUDED_FRONTMATTER_FIELDS`) is a hard-coded constant
containing only `status`. Adding a new exclusion requires editing
the constant and amending this spec.

**Consequences:** A new frontmatter field automatically contributes
to the hash. If the new field is purely cosmetic, the author will
notice the spurious drift on the first ship after the field is
introduced and can amend this spec to add the field to the
exclusion list. The alternative (default-exclude) would silently
let semantically meaningful fields stop affecting drift detection,
which is a worse failure mode than noisy hashes.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: ID consistency enforced as lint + command-guard, not parse-time

**Status:** Accepted

**Context:** Three options for enforcing the 3-way ID check: (a)
parse-time error (any consumer of `parse::spec_md` errors out on
mismatch), (b) lint rule (informational/error diagnostic visible in
`speccy status` and `speccy verify`), (c) command-guard (refuses
to persist hashes when the triple disagrees).

**Decision:** Lint + command-guard (b + c). Skip parse-time.

**Consequences:** `speccy status` and `speccy verify` keep working
when IDs disagree, which is precisely what makes the mismatch
visible to the operator. The lint rule advertises the problem; the
command-guard prevents the operator from compounding the problem
by writing a hash against the broken triple. Parse-time enforcement
would silence both diagnostic surfaces and force the operator to
discover the problem by reading parse errors instead of status
output — wrong direction. The cost of the layered approach is two
implementations of the same predicate; both share a helper to keep
them in sync.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: Migration is a one-shot agent task, not a CLI command

**Status:** Accepted

**Context:** Switching the hash function invalidates every stored
`spec_hash_at_generation`. Two paths to reconcile: (a) add a
`speccy migrate-hashes` (or `speccy tasks --recompute-all`) CLI
command, (b) document the reconciliation as the last implementation
task and let an AI agent loop over every spec via the existing
`speccy tasks --commit` path.

**Decision:** Option (b). The TASKS.md for this spec has a final
reconciliation task that the agent completes after the code
changes land. `speccy status` is the verification gate.

**Consequences:** No new CLI surface for a one-shot event. The
reconciliation is part of this spec's history (visible in the
commit log) and the verification result is recorded in the
handoff note. Future projects that add Speccy mid-flight will not
inherit this migration — they will have no pre-existing hashes to
reconcile.
</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: `speccy status` no-args keeps the filter; `--all` opts out

**Status:** Accepted

**Context:** Three plausible default behaviors when `speccy status`
is invoked without arguments: (a) keep today's attention-list
filter, (b) flip the default to show all specs, (c) introduce a
`--filtered` flag and let the default be "all". Option (b) breaks
the current ergonomic contract for users who already invoke
`speccy status` regularly. Option (c) inverts the default which is
the same break with extra steps.

**Decision:** Option (a). No-args keeps the filter. `--all`
explicitly opts out. The footer announces the filter so it stops
being silent.

**Consequences:** Existing muscle memory and shipped skills that
invoke `speccy status` continue to work without rewriting. New
users learn the filter exists on first run via the footer. The
positional selector covers the "I want one specific spec" case
that prompted this work in the first place.
</decision>

<decision id="DEC-006" status="accepted">
#### DEC-006: JSON output ignores the text-mode filter

**Status:** Accepted

**Context:** `speccy status --json` today renders every spec
regardless of the filter. The new selector and `--all` flag could
either continue that (JSON is always unfiltered, modulo a positional
that narrows to one spec) or extend the filter to JSON for symmetry
with text mode.

**Decision:** JSON stays unfiltered by default. `--json` (no
selector, no `--all`) renders every spec, matching today's
behavior. `--all --json` is the explicit, equivalent invocation
for callers that want to be unambiguous. `SPEC-NNNN --json`
renders exactly one spec.

**Consequences:** JSON consumers (shipped skills, jq pipelines)
are unaffected by the text-mode filter behavior. The text-mode
filter is a UX affordance; JSON is a programmatic interface where
unfiltered output is more useful. Symmetry between the two would
have hidden specs from `--json` callers who today rely on the full
list, which would be a silent regression.
</decision>

### Interfaces

- `speccy-core/src/parse/spec_md.rs`:
  - `const HASH_EXCLUDED_FRONTMATTER_FIELDS: &[&str] = &["status"];`
  - `fn canonical_frontmatter_for_hash(fm: &SpecFrontmatter) ->
    Result<Vec<u8>, ParseError>` (or similar; signature finalised
    during implementation).
  - `SpecMd.sha256` field semantics changed; field type and name
    unchanged.
- `speccy-core/src/lint/rules/`: new rule file or block in
  `tsk.rs` (implementer's choice). Public surface unchanged; the
  new rule registers via the existing rules registry.
- `speccy-core/src/tasks.rs`:
  - `CommitError::IdTripleMismatch { folder: String, spec_md:
    String, tasks_md: String }` (or equivalent; name finalised
    during implementation).
  - `commit_frontmatter` performs the new 3-way check before any
    write.
- `speccy-cli/src/status.rs`:
  - `StatusArgs.selector: Option<String>` (a SPEC-NNNN string).
  - `StatusArgs.all: bool`.
  - New selector-resolver function returning `(Vec<&SpecView<'_>>,
    hidden_count)` consumed by both renderers.
  - `render_text` emits the new footer when `hidden_count > 0` in
    no-args mode; suppressed otherwise.
- `speccy-cli/src/main.rs` clap surface: positional `SPEC-NNNN`
  argument plus `--all` flag on `status`; mutual exclusion enforced
  at parse time.

### Data changes

None to artifact grammars. SPEC.md, TASKS.md, REPORT.md, and JSON
shapes are unchanged. The semantic meaning of `SpecMd.sha256`
changes (computation scope) but the field type and downstream
storage in TASKS.md frontmatter are unchanged.

### Migration / rollback

Forward: covered by REQ-005. After code lands, the reconciliation
task rewrites every TASKS.md `spec_hash_at_generation` to the new
hash. Verified by `speccy status` reporting no `hash-drift`.

Rollback: revert the commits. Stored hashes after reconciliation
would then show as drifted under the reverted code (because they
were computed under the new function). A second reconciliation pass
(against the reverted code) would be required to restore quiet
state. This is acceptable because rollback is rare and the
reconciliation is mechanical.

## Open Questions

<open-question resolved="true">
Resolved (DEC-001): Use canonical re-serialization of frontmatter,
not byte-strip of the `status:` line. The implementer picks a YAML
emitter that produces deterministic output; if the existing
`serde-saphyr` dependency lacks a stable emitter, the implementer
either pulls in a peer (e.g., `serde_yaml`'s emitter, or a small
canonical-YAML helper) or hand-rolls the emitter for the bounded
schema (`SpecFrontmatter` has six fields). The choice is captured
in the implementation commit message, not amended into this spec.
</open-question>

<open-question resolved="true">
Resolved (DEC-002): Future frontmatter fields default to
included-in-hash. The exclusion list is hard-coded and contains
only `status`.
</open-question>

<open-question resolved="true">
Resolved (DEC-003): The 3-way ID check is enforced both as a lint
rule and as a command-guard at `speccy tasks --commit`. Not at
parse time — that would silence the diagnostic surfaces that flag
the problem.
</open-question>

<open-question resolved="true">
Resolved (DEC-004): Migration is the last implementation task in
TASKS.md, not a CLI command. The agent reconciles every TASKS.md
after the code changes land; `speccy status` is the verification
gate.
</open-question>

<open-question resolved="true">
Resolved (DEC-005): `speccy status` no-args keeps today's filter.
`--all` opts out. A one-line footer announces the filter when it
hid specs.
</open-question>

<open-question resolved="true">
Resolved (DEC-006): JSON output is unfiltered by default. The
positional selector narrows JSON to one spec; `--all --json`
matches today's `--json` shape; `--json` alone matches today's
`--json` shape (every spec).
</open-question>

## Assumptions

<assumptions>
- A deterministic YAML emitter is available (either in an existing
  workspace dependency, a small new dependency, or a hand-rolled
  canonical writer over the six-field `SpecFrontmatter` schema).
  If the implementer discovers no acceptable option, that surfaces
  as a friction note and this spec amends to specify the choice.
- `derive_spec_id_from_dir`
  (`speccy-core/src/workspace.rs:244`) is the canonical way to get
  the folder-derived ID and is reused by both the lint rule and the
  command-guard. If the regex it uses ever changes, both consumers
  pick up the change uniformly.
- The existing `commit_frontmatter` `SpecIdMismatch` variant stays;
  the new variant is a peer. Backward compatibility with that error
  path matters because shipped skills may match on the variant
  shape; adding a new variant is additive.
- The `--all` and positional selector combinations covered in
  REQ-004 (positional alone, `--all` alone, neither, both
  rejected) are exhaustive. Future enhancements (e.g., glob
  selectors, multiple positional IDs) land in their own spec.
- The integration tests for `speccy status` use the existing
  fixture format; no new fixture machinery is required. If the
  implementer needs to add fixtures for "5 specs with 3 stale" for
  CHK-004's footer test, that is fixture work within the existing
  fixture conventions.
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-17 | human/kevin | Initial draft. Three changes under one theme: hash excludes `status` and other frontmatter cosmetics; 3-way ID consistency check (lint + command-guard); `speccy status` positional and `--all` selectors with hidden-count footer. Migration handled as a one-shot reconciliation task. |
</changelog>

## Notes

This spec was prompted by three separate observations that turned
out to share one cause: `speccy status` produced signal that did
not mean what it said.

- "Text formatter dropping specs" was actually intentional filtering
  (`show_in_text_view`), but the filter was silent. REQ-004 makes it
  loud.
- "Hash-drift everywhere" was real drift on older specs whose
  TASKS.md stored hashes against an earlier SPEC.md content state
  (genuine, mid-loop drift during the XML migrations of
  SPEC-0019/0020/0022). The fresh-after-ship specs (SPEC-0021,
  SPEC-0023) showed clean because their ship commits explicitly
  refreshed the hash. That manual refresh is the tax REQ-001
  eliminates.
- "No per-spec status" was a real gap. REQ-004's positional
  selector closes it.

The naming of the new lint code (`TSK-005` or another) is left to
the implementer; pick what reads most naturally in the rule registry
context.

The reconciliation task in REQ-005 must run after every other task
in this spec is complete; the TASKS.md decomposition orders it last
intentionally.
