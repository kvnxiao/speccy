---
id: SPEC-0001
slug: artifact-parsers
title: Artifact parsers for Speccy's five file types
status: implemented
created: 2026-05-11
---

# SPEC-0001: Artifact parsers

## Summary

Speccy's CLI is mechanical: it renders prompts, queries artifact
state, and runs checks. Every command in the implementation sequence
depends on being able to read and interpret the five artifact files
Speccy understands -- `speccy.toml`, `spec.toml`, `SPEC.md`,
`TASKS.md`, `REPORT.md`.

This spec lands the parser foundation as a reusable library
(`speccy-core`) consumed by the `speccy` binary and any future
tooling. The work is deliberately narrow: parse, validate
`schema_version`, expose typed structs, return a single structured
error type. No CLI command is added in this spec; later specs
(SPEC-0002 onward) consume the parser API.

The parser must handle Speccy's known quirks:

- Markdown fenced code blocks containing example REQ headings.
  `.speccy/DESIGN.md` is itself a worked example -- the design doc
  embeds `### REQ-NNN:` headings inside its own SPEC.md template
  examples. A naive regex scan would mis-identify them.
- CRLF line endings on Windows (the primary dev environment).
- Absent optional frontmatter fields treated identically to empty
  lists / `None`.
- YAML payloads delivered via `serde-saphyr` because `serde_yaml` is
  deprecated and `serde_yml` was archived per RUSTSEC-2025-0068.

## Goals

- A single deterministic API for reading every Speccy artifact.
- Refuse unknown `schema_version` with a clear error rather than
  silently misinterpreting a future schema.
- Skip fenced code blocks when scanning for REQ headings.
- Expose enough structure for SPEC-0003 (lint) and downstream commands
  to do their work without re-parsing files.
- No panics reachable from public APIs.

## Non-goals

- No semantic validation of prose. ("Done when:" being meaningful is
  review's job.)
- No prompt rendering, command execution, or I/O beyond file reads.
- No tolerance for unknown `schema_version`. Speccy v1 targets
  `schema_version = 1` only.
- No fancy YAML features (anchors, merge keys, custom tags). Speccy
  frontmatter is flat key-value plus arrays.
- No normalisation of SPEC.md content. The sha256 is over raw bytes.

## User stories

- As a future `speccy status` implementer, I want a single function
  call that returns parsed state for every spec under `specs/` so I
  can build the status view without re-implementing parsing.
- As a future `speccy lint` implementer, I want cross-references
  between SPEC.md REQ headings and `spec.toml` `[[requirements]]` rows
  so I can emit SPC-002 and SPC-003 mechanically.
- As a future `speccy tasks --commit` implementer, I want the SPEC.md
  parser to expose a stable sha256 hash of the SPEC.md byte content
  so I can record it into TASKS.md frontmatter without normalising
  prose myself.

## Requirements

### REQ-001: TOML config parsers

Parse `speccy.toml` and `spec.toml` into typed structs.

**Done when:**
- A valid `speccy.toml` deserialises into
  `SpeccyConfig { project: { name, root } }`.
- A valid `spec.toml` deserialises into
  `SpecToml { requirements: Vec<RequirementEntry>, checks: Vec<CheckEntry> }`.
- Each `CheckEntry` carries `id`, `kind` (free-form string), `proves`,
  and exactly one of `command` or `prompt`.
- `schema_version` other than `1` returns an error naming the file
  and the offending value.
- Missing required fields return an error naming the field and the
  parent table.

**Behavior:**
- Given a `spec.toml` with `schema_version = 1`, three
  `[[requirements]]` rows, and three `[[checks]]` rows, when parsed,
  the resulting struct contains all rows in declared order.
- Given a `spec.toml` with `schema_version = 2`, when parsed, the
  result is an error containing the string `schema_version = 2`.
- Given a `spec.toml` where a `[[checks]]` entry has neither
  `command` nor `prompt`, when parsed, the result is an error naming
  the check ID.
- Given a `spec.toml` where a `[[checks]]` entry has both `command`
  and `prompt`, when parsed, the result is an error naming the check
  ID and the conflict.

**Covered by:** CHK-001, CHK-002, CHK-007

### REQ-002: Frontmatter extraction

Split markdown files into `(yaml_frontmatter, body)` pairs without
pulling in a third-party crate dedicated to the job.

**Done when:**
- A file beginning with `---\n<yaml>\n---\n<body>` splits into
  `Some((yaml, body))`.
- A file without a leading `---` fence returns `None`.
- A file with an opening `---` but no closing `---` fence returns a
  structured `ParseError`.
- Both `\n` and `\r\n` line endings are handled.

**Behavior:**
- Given a SPEC.md with valid frontmatter, when split, the YAML
  fragment ends at the line before the closing fence and the body
  starts at the line after.
- Given a SPEC.md whose body begins with `---` (a horizontal rule),
  when split, the splitter does not treat the horizontal rule as the
  closing fence. Only the *first* `---`-on-its-own-line after the
  opening fence counts.
- Given a file that is exactly `---\n---\n` (empty frontmatter, empty
  body), when split, the result is `Some(("", ""))`.

**Covered by:** CHK-003

### REQ-003: SPEC.md parsing

Parse SPEC.md frontmatter, REQ headings, and the `## Changelog`
table.

**Done when:**
- Frontmatter deserialises via `serde-saphyr` into
  `SpecFrontmatter { id, slug, title, status, created, supersedes }`.
  `supersedes` defaults to an empty `Vec` when omitted from the
  source file. There is no `superseded_by` field on the frontmatter;
  the inverse direction is computed via REQ-008.
- REQ headings are extracted as
  `Vec<ReqHeading { id, title, line }>`, matched by the regex
  `^REQ-\d{3}: ` applied only to heading text yielded by `comrak` --
  not raw text, and never from inside fenced code blocks.
- The `## Changelog` table is parsed into
  `Vec<ChangelogRow { date, author, summary }>` from any GFM table
  immediately under a case-insensitive `## Changelog` heading.
- `status` is validated against the closed set `{in-progress,
  implemented, dropped, superseded}`.
- A sha256 hash of the full SPEC.md byte content (including
  frontmatter) is computed and exposed on the parsed struct.

**Behavior:**
- Given a SPEC.md containing a fenced code block with `### REQ-999:`
  inside it, when parsed, `REQ-999` is **not** present in the
  extracted REQ heading list.
- Given a SPEC.md with `status: superseded` and `superseded_by: []`,
  when parsed, the parse itself succeeds. The inconsistency is
  SPEC-0003's job to surface as `SPC-006`.
- Given a SPEC.md with no `## Changelog` heading, when parsed,
  `changelog` is an empty vec.
- Given a SPEC.md modified by one byte, when parsed, the sha256 hash
  differs from the prior parse's hash.

**Covered by:** CHK-004, CHK-005

### REQ-004: TASKS.md parsing

Parse TASKS.md frontmatter, task lines with state, and inline notes.

**Done when:**
- Frontmatter deserialises into
  `TasksFrontmatter { spec, spec_hash_at_generation, generated_at }`.
- Tasks are extracted as
  `Vec<Task { id, title, state, covers, notes, suggested_files }>`.
- `state` maps from the checkbox glyph: `[ ]` -> `Open`,
  `[~]` -> `InProgress`, `[?]` -> `AwaitingReview`,
  `[x]` -> `Done`.
- Task `id` is extracted from a bold span at the start of the task's
  inline content matching `^T-\d{3}$`.
- Sub-list items under a task are collected verbatim into `notes` in
  declared order.
- Bullets matching `Covers: <CSV>` produce the parsed `covers` vec.
- Bullets matching `Suggested files: <backtick-CSV>` produce the
  parsed `suggested_files` vec.
- Phase headings (`## Phase N: ...`) are ignored for parsing
  purposes; the parser does not produce a `Phase` struct.

**Behavior:**
- Given a TASKS.md with two `[ ]`, one `[~]`, one `[?]`, and one
  `[x]` task, when parsed, the resulting state counts are
  `{open: 2, in_progress: 1, awaiting_review: 1, done: 1}`.
- Given a task whose ID bold span is malformed (e.g. `**TASK-001**`),
  when parsed, the task is skipped and a recoverable warning is
  surfaced via the result so SPEC-0003 (lint) can emit `TSK-002`.
- Given a task with three sub-list `Review (...)` bullets, when
  parsed, `notes` contains all three preserving declared order.

**Covered by:** CHK-006

### REQ-005: REPORT.md frontmatter parsing

Parse REPORT.md frontmatter; return the body verbatim without further
structural parsing.

**Done when:**
- Frontmatter deserialises into
  `ReportFrontmatter { spec, outcome, generated_at }`.
- `outcome` is validated against the closed set
  `{delivered, partial, abandoned}`.
- The REPORT.md body is returned verbatim as a `String`.

**Behavior:**
- Given a REPORT.md with `outcome: rejected`, when parsed, the result
  is an error naming the invalid value.
- Given a REPORT.md missing the `generated_at` field, when parsed,
  the result is an error naming the missing field.

**Covered by:** CHK-008

### REQ-006: Cross-reference SPEC.md against spec.toml

Produce the symmetric diff between REQ headings in SPEC.md and
`[[requirements]]` rows in `spec.toml`.

**Done when:**
- A pure function takes a parsed `SpecMd` and a parsed `SpecToml`
  and returns
  `CrossRef { only_in_spec_md: Vec<String>, only_in_toml: Vec<String>, in_both: Vec<String> }`.
- Ordering inside each list matches declared order in the source.
- The function is deterministic and idempotent.

**Behavior:**
- Given a SPEC.md with `REQ-001`, `REQ-002`, `REQ-003` and a
  `spec.toml` with `REQ-001`, `REQ-002`, `REQ-004`, when
  cross-referenced, the result is
  `only_in_spec_md = ["REQ-003"]`, `only_in_toml = ["REQ-004"]`,
  `in_both = ["REQ-001", "REQ-002"]`.

**Covered by:** CHK-009

### REQ-007: Public API and hygiene for `speccy-core`

Expose the parser surface as a stable library API and lock in the
project's quality gates from day one.

**Done when:**
- `speccy-core` exposes a public module path
  `speccy_core::parse::{speccy_toml, spec_toml, spec_md, tasks_md, report_md, cross_ref}`.
- Each parser takes a `&Path` and returns
  `Result<T, ParseError>` where `ParseError` is a single enum
  covering all parse failures with structured variants (no
  string-only errors).
- The crate sets `#![deny(unsafe_code)]` at its root.
- No `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, or
  `unimplemented!` appears anywhere in `speccy-core/src/`.
  (Tests may use `.expect("descriptive message")`.)
- `cargo clippy -p speccy-core --all-targets --all-features -- -D warnings`
  is clean.

**Behavior:**
- Given a fresh checkout, when `cargo build --workspace` runs, it
  compiles without warnings under the project's `-D warnings` gate.
- Given a parser failure surfaced to a consumer, the error message
  identifies the file path or label, the line number where possible,
  and the human-readable reason.

**Covered by:** CHK-010, CHK-011

### REQ-008: Supersession index across the workspace

Compute the inverse `superseded_by` relation by scanning every
parsed SPEC.md's `frontmatter.supersedes`.

**Done when:**
- A pure function takes `&[&SpecMd]` and returns a
  `SupersessionIndex` keyed by spec ID.
- For any spec ID `Y`, `index.superseded_by(Y)` returns the set of
  IDs `X` whose `frontmatter.supersedes` contains `Y`, in declared
  order across the input slice.
- A spec ID referenced via `supersedes` but absent from the input
  slice is surfaced via `index.dangling_references()` so SPEC-0003
  (lint) can emit a diagnostic without re-scanning.
- The function is deterministic and idempotent.

**Behavior:**
- Given SPEC-0017 (no `supersedes`), SPEC-0042
  (`supersedes: [SPEC-0017]`), and SPEC-0050
  (`supersedes: [SPEC-0017, SPEC-0030]`), when indexed, then
  `index.superseded_by("SPEC-0017") = ["SPEC-0042", "SPEC-0050"]`
  and `index.dangling_references()` contains `"SPEC-0030"`.
- Given a workspace with no `supersedes` declarations anywhere, when
  indexed, the result is empty (no panics, no errors).
- Given the same input slice in the same order twice, the index
  values are equal.

**Covered by:** CHK-012

## Design

### Approach

A Cargo workspace with two crates: `speccy-core` (library) and
`speccy` (binary CLI, scaffolded but mostly empty in this spec). The
library exposes one parser function per artifact type, all sharing a
single `ParseError` enum.

The parsing stack matches `.speccy/DESIGN.md`'s "Operational details
/ Parsing stack" section:

- `toml` for TOML files.
- `comrak` (CommonMark + GFM) for markdown event streams. We walk the
  event stream rather than rendering -- that is what lets us reliably
  skip fenced code blocks when looking for REQ headings.
- `serde-saphyr` for YAML frontmatter. Direct-to-struct
  deserialisation, panic-free on malformed input. Pinned exact
  `0.0.x` per DESIGN.md guidance. Expected to need a minor refactor
  when `0.1.0` ships.
- `regex` only for narrow ID extraction (REQ-NNN, T-NNN) from
  already-isolated heading or strong-text node content. Never used
  for structural parsing.
- DIY frontmatter splitter (a four-line string-slice routine). No
  `gray_matter` dependency -- we do not want to lock SPEC.md
  frontmatter parsing to a YAML crate other than the one we already
  chose.
- `sha2` for hashing SPEC.md content.

### Decisions

#### DEC-001: Workspace layout

**Status:** Accepted
**Context:** Rust scaffolding has to land somewhere; this spec is
the first place that needs it.
**Decision:** Cargo workspace with two crates:
- `speccy-core/` -- library; all parsing, rendering, lint
  logic.
- `speccy-cli/` -- thin binary CLI.

**Alternatives:**
- Single crate with `src/lib.rs` + `src/main.rs` -- rejected.
  Blocks later splits without churn (e.g. an embeddable parser for
  an IDE plugin or a daemon process consuming `speccy-core`).
- One crate per command -- rejected. Premature; ten commands in one
  binary is fine and matches the CLI's deliberate thinness.

**Consequences:** Cargo workspace overhead is minor. Library is
reusable across future Speccy host integrations. All future specs
land code in one of these two crates.

#### DEC-002: `serde-saphyr` for YAML frontmatter

**Status:** Accepted (per `.speccy/DESIGN.md` "Operational details")
**Context:** Existing Rust YAML crates are either deprecated
(`serde_yaml`) or archived as unsafe (`serde_yml`,
RUSTSEC-2025-0068).
**Decision:** Use `serde-saphyr` with exact `0.0.x` version pinning.
**Alternatives:**
- `serde_yml` -- rejected. Archived; RUSTSEC-2025-0068 (unsound,
  panics on malformed input).
- Raw `saphyr` + manual mapping -- rejected. Roughly 5x more code to
  maintain than a serde adapter.

**Consequences:** Frontmatter parsing is safe and direct-to-struct.
Pre-`0.1.0` API may break; we accept a minor refactor when it
stabilises. Recorded as a known unknown in `VISION.md`.

#### DEC-003: `comrak` event walk, not rendering

**Status:** Accepted
**Context:** SPEC.md contains fenced code blocks with example REQ
headings. Pure-regex parsing would misidentify them.
**Decision:** Walk the `comrak` event stream and extract only
heading and strong-text nodes that live outside fenced code
contexts. Use `regex` only on the inline content of those isolated
nodes.
**Alternatives:**
- Pure regex -- rejected. Cannot reliably skip code fences.
- `pulldown-cmark` -- rejected. Smaller GFM-table support than
  `comrak`; more work to match `comrak`'s tracked-extensions
  behavior.

**Consequences:** Adds a CommonMark dependency. Worth it; this is
the robustness `.speccy/DESIGN.md` specifically calls out.

#### DEC-004: Hash SPEC.md raw bytes for staleness detection

**Status:** Accepted
**Context:** TASKS.md frontmatter needs `spec_hash_at_generation`
populated by SPEC-0006 (`speccy tasks --commit`).
**Decision:** Compute sha256 of raw SPEC.md bytes (including
frontmatter). No normalisation.
**Alternatives:**
- Normalised hash (e.g. strip trailing whitespace, normalise line
  endings) -- rejected. Introduces ambiguity about what
  "normalised" means and can mask real edits.
- Modification time only -- rejected. Too easily fooled by `touch`
  or by tools that rewrite files identically.

**Consequences:** Any SPEC.md byte change invalidates the hash.
Whitespace-only edits trigger staleness warnings. We accept this;
the remedy is `/speccy:amend` (covered by SPEC-0005, SPEC-0006).

#### DEC-005: Single-direction supersession (no stored `superseded_by`)

**Status:** Accepted
**Context:** SPEC.md frontmatter could store both `supersedes` (this
spec replaces those) and `superseded_by` (those specs replaced this
one), as the initial `.speccy/DESIGN.md` sketch did. The bidirectional
form requires editing the older spec whenever a new one supersedes
it, which is easy to skip and easy to get out of sync.
**Decision:** Store only `supersedes` on the new spec. Compute
`superseded_by` by walking the supersedes graph across all parsed
SPEC.mds at query time. Expose this as `supersession_index`
(REQ-008).
**Alternatives:**
- Bidirectional pointers -- rejected. Doubles maintenance cost;
  drifts silently; introduces a lint failure mode (one direction
  set, the other forgotten) that adds noise without adding
  information.
- Derive `status: superseded` from the graph too -- rejected.
  Lifecycle status is an explicit author signal (the older spec's
  maintainer affirms "yes, this is done; SPEC-NNN replaces it"),
  not a graph-derived inference.

**Consequences:** Lineage is single-sourced; new specs declare their
replacements in one place. `.speccy/DESIGN.md` is amended in the
same change to drop `superseded_by` from the frontmatter table and
to redefine `SPC-006` ("status = superseded but no other spec
declares `supersedes` pointing here") accordingly.

### Interfaces

Public API surface from `speccy-core`:

```rust
pub mod parse {
    pub fn speccy_toml(path: &Path) -> Result<SpeccyConfig, ParseError>;
    pub fn spec_toml(path: &Path) -> Result<SpecToml, ParseError>;
    pub fn spec_md(path: &Path) -> Result<SpecMd, ParseError>;
    pub fn tasks_md(path: &Path) -> Result<TasksMd, ParseError>;
    pub fn report_md(path: &Path) -> Result<ReportMd, ParseError>;
    pub fn cross_ref(spec: &SpecMd, toml: &SpecToml) -> CrossRef;
    pub fn supersession_index<'a>(specs: &'a [&'a SpecMd])
        -> SupersessionIndex<'a>;
}

pub struct SpecMd {
    pub frontmatter: SpecFrontmatter,
    pub requirements: Vec<ReqHeading>,
    pub changelog: Vec<ChangelogRow>,
    pub raw: String,
    pub sha256: [u8; 32],
}

pub struct SpecFrontmatter {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: SpecStatus,
    pub created: NaiveDate,         // chrono or time; choose at impl
    pub supersedes: Vec<String>,    // empty if omitted in the file
}

pub struct SupersessionIndex<'a> {
    // implementation detail; exposes:
    //   fn superseded_by(&self, id: &str) -> &[&'a str]
    //   fn dangling_references(&self) -> &[&'a str]
}
```

Concrete struct definitions are an implementation detail landed by
tasks. The shape above is the durable contract.

### Data changes

- New top-level Rust workspace: `Cargo.toml` (workspace manifest),
  `speccy-cli/Cargo.toml`, `speccy-core/Cargo.toml`.
- New `speccy-core/src/lib.rs` and submodules under
  `speccy-core/src/parse/`.
- New `speccy-cli/src/main.rs` (stub that prints
  `speccy CLI; no commands implemented yet` and exits 2 -- actual
  command wiring lands in SPEC-0002 onward).
- `.gitignore` additions for `target/`.

No removal or rename of existing files. `AGENTS.md` stays at the
project root.

### Migration / rollback

- Forward: greenfield Rust scaffolding. No migration.
- Rollback: delete `Cargo.toml`, `crates/`, `target/`,
  `.speccy/specs/0001-artifact-parsers/`. Nothing else in the repo
  depends on the parser yet.

## Open questions

- [ ] Should `ParseError` carry source spans (line ranges) for every
  variant, or only line numbers? Spans are nicer for tooling but
  cost more to plumb. Defer to first downstream consumer (likely
  SPEC-0003 lint), which will tell us what it actually needs.
- [x] Where does `AGENTS.md` live? -- At the project root (existing
  location). `.speccy/DESIGN.md`'s file-layout sketch showing
  `.speccy/AGENTS.md` is conceptual. Every host already loads
  `AGENTS.md` from the project root, and the symlink from
  `CLAUDE.md` reinforces that. `.speccy/DESIGN.md` should be amended
  to reflect this in a later, non-blocking pass.

## Assumptions

- Files are UTF-8. Non-UTF-8 input is a parse error (we do not
  attempt encoding detection).
- The repository targets Rust stable for normal compilation;
  `cargo +nightly fmt` is the only nightly-only invocation (matches
  `CLAUDE.md` hygiene).
- `comrak` is configured with GFM extensions enabled (tables, task
  lists, strikethrough). Without these, the `## Changelog` table and
  task-list parsing do not work.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from `.speccy/DESIGN.md` decomposition (bootstrap of speccy itself). |

## Notes

This spec is the keystone for SPEC-0002 through SPEC-0012; every
later command consumes the parser surface defined here. Land it
first; review it adversarially with the architecture persona because
its public API is hard to evolve once other specs depend on it.

The implementation order inside this spec is non-obvious: scaffold
the workspace first, then implement the frontmatter splitter, then
the TOML parsers, then the markdown parsers (which depend on both
splitter and TOML cross-references). The TASKS.md decomposition
reflects that order.
