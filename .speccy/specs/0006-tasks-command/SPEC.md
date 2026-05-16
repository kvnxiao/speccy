---
id: SPEC-0006
slug: tasks-command
title: speccy tasks -- render Phase 2 prompt + record TASKS.md hash
status: implemented
created: 2026-05-11
---

# SPEC-0006: speccy tasks

## Summary

`speccy tasks SPEC-ID` is the Phase 2 command. It renders the
prompt an agent reads to decompose a SPEC into tasks (initial
form) or to amend an existing TASKS.md surgically (amendment
form). Form selection is automatic: TASKS.md absent -> initial
template; present -> amendment template.

`speccy tasks SPEC-ID --commit` is a sub-action used by skills
after the agent finishes writing TASKS.md. It computes the current
SPEC.md sha256, writes it into TASKS.md's
`spec_hash_at_generation` frontmatter field, and updates
`generated_at` to the current UTC timestamp. The body bytes of
TASKS.md are preserved exactly -- only the two frontmatter fields
change.

This is the first prompt-emitting command that also writes a file
(under `--commit`), so it establishes the body-byte-preserving
frontmatter rewrite pattern. The pattern is narrow (only this
command needs it in v1), so it lives in `speccy-core::tasks`
rather than a generic helper.

## Goals

<goals>
- One CLI surface for initial + amendment decomposition prompts;
  form auto-detected.
- `--commit` records the spec hash so staleness detection
  (SPEC-0003 TSK-003, SPEC-0004 stale_for) has accurate input.
- Body of TASKS.md is preserved byte-for-byte across `--commit`
  invocations -- no markdown reformatting, no whitespace
  normalisation.
- Initial-bootstrap specs (with `spec_hash_at_generation:
  bootstrap-pending`) graduate to real hashes the first time
  `--commit` runs against them.
</goals>

## Non-goals

<non-goals>
- No partial commit. `--commit` always rewrites both fields
  together (hash + timestamp); they're a unit.
- No prompt mode under `--commit`. The flags are mutually
  exclusive in effect: prompt-rendering when `--commit` is absent;
  file-writing when present.
- No agent invocation. The CLI never calls an LLM.
- No interactive confirmation. `--commit` is non-interactive.
- No automatic discovery of which spec was edited; the SPEC-ID
  must be passed explicitly.
</non-goals>

## User stories

<user-stories>
- As a skill orchestrating Phase 2, I want to invoke
  `speccy tasks SPEC-0001` once to get the decomposition prompt,
  then `speccy tasks SPEC-0001 --commit` after the agent writes
  TASKS.md, so the spec-hash is recorded for staleness detection.
- As a developer mid-loop, I want `speccy tasks SPEC-0001` (with
  an existing TASKS.md) to give me the amendment prompt that
  asks the agent to preserve completed tasks, not start over.
- As a future SPEC-0003 (lint) consumer, I want TASKS.md
  frontmatter's `spec_hash_at_generation` to be accurate so the
  TSK-003 staleness diagnostic fires only when SPEC.md actually
  drifted.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Initial prompt rendering

When TASKS.md is absent for the given spec, render the initial
Phase 2 prompt.

<done-when>
- The command resolves SPEC-ID to its spec directory (see REQ-005).
- It parses the existing SPEC.md.
- It loads AGENTS.md via `speccy_core::prompt::load_agents_md`
  (SPEC-0005 REQ-004).
- It loads the embedded `tasks-generate.md` template via
  `speccy_core::prompt::load_template`.
- It substitutes placeholders: `{{spec_id}}`, `{{spec_md}}` (full
  content of the SPEC.md, including frontmatter), `{{agents}}`.
- It trims to budget via `speccy_core::prompt::trim_to_budget`.
- It writes the rendered prompt to stdout; exits 0.
</done-when>

<behavior>
- Given `.speccy/specs/0001-foo/SPEC.md` exists and TASKS.md does
  not, when `speccy tasks SPEC-0001` runs, then the rendered
  prompt contains the SPEC.md content where `{{spec_md}}` was.
- Given the SPEC.md parses without error, the initial-mode
  prompt language is rendered (not amendment-mode).
</behavior>

<scenario id="CHK-001">
- Given `.speccy/specs/0001-foo/SPEC.md` exists and TASKS.md does
  not, when `speccy tasks SPEC-0001` runs, then the rendered
  prompt contains the SPEC.md content where `{{spec_md}}` was.
- Given the SPEC.md parses without error, the initial-mode
  prompt language is rendered (not amendment-mode).

speccy tasks SPEC-NNNN with TASKS.md absent renders tasks-generate.md with spec_id, spec_md, and agents placeholders substituted; output goes to stdout.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Amendment prompt rendering

When TASKS.md is present for the given spec, render the
amendment Phase 2 prompt.

<done-when>
- The command parses both SPEC.md and TASKS.md.
- It loads AGENTS.md.
- It loads the embedded `tasks-amend.md` template.
- It substitutes placeholders: `{{spec_id}}`, `{{spec_md}}`,
  `{{tasks_md}}` (full content of existing TASKS.md), `{{agents}}`.
- It trims to budget.
- It writes the rendered prompt to stdout; exits 0.
</done-when>

<behavior>
- Given both SPEC.md and TASKS.md exist for SPEC-0001, when
  `speccy tasks SPEC-0001` runs, then the rendered prompt
  contains the TASKS.md content where `{{tasks_md}}` was.
- The amendment-mode prompt language asks the agent to preserve
  completed tasks unless invalidated by spec changes; it does
  NOT ask for a fresh decomposition.
</behavior>

<scenario id="CHK-002">
- Given both SPEC.md and TASKS.md exist for SPEC-0001, when
  `speccy tasks SPEC-0001` runs, then the rendered prompt
  contains the TASKS.md content where `{{tasks_md}}` was.
- The amendment-mode prompt language asks the agent to preserve
  completed tasks unless invalidated by spec changes; it does
  NOT ask for a fresh decomposition.

speccy tasks SPEC-NNNN with TASKS.md present renders tasks-amend.md with spec_id, spec_md, tasks_md, and agents placeholders substituted; amendment-mode template (not initial) is selected.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `--commit` records spec hash and timestamp

The `--commit` sub-action rewrites TASKS.md's frontmatter with
the current SPEC.md sha256 and UTC timestamp.

<done-when>
- The command parses SPEC.md and computes its sha256 (matches
  SPEC-0001 REQ-003's `SpecMd.sha256`).
- It reads TASKS.md raw bytes.
- It rewrites the frontmatter:
  - `spec_hash_at_generation` set to the 64-char hex of the
    sha256.
  - `generated_at` set to the current UTC timestamp in ISO 8601
    `YYYY-MM-DDTHH:MM:SSZ` form.
  - The `spec` field is preserved (or set to the canonical
    SPEC-ID if missing).
  - Any other frontmatter fields the agent added are preserved
    byte-identically.
- It writes TASKS.md back to disk.
- Exits 0.
</done-when>

<behavior>
- Given TASKS.md frontmatter `spec_hash_at_generation:
  bootstrap-pending` (the bootstrap sentinel), when `speccy
  tasks SPEC-0001 --commit` runs, then the sentinel is replaced
  with the real sha256 hex.
- Given TASKS.md frontmatter has the right hash already and
  `generated_at` from yesterday, `--commit` still updates
  `generated_at` to the current UTC moment.
- Given TASKS.md frontmatter is absent entirely, `--commit`
  prepends a fresh frontmatter block with `spec`, `spec_hash_at_generation`,
  `generated_at` (in that order) followed by the original body
  bytes.
- Given TASKS.md frontmatter's `spec` field does not match the
  command's SPEC-ID arg, exit code is 1 with a clear error
  naming both IDs.
</behavior>

<scenario id="CHK-003">
commit_frontmatter writes the SPEC.md sha256 as 64-char hex into spec_hash_at_generation and sets generated_at to the supplied UTC ISO 8601 timestamp.
</scenario>

<scenario id="CHK-004">
commit_frontmatter overwrites the bootstrap-pending sentinel with the real sha256 hex on first invocation.
</scenario>

<scenario id="CHK-005">
commit_frontmatter returns SpecIdMismatch when frontmatter spec field differs from arg; prepends new frontmatter when entirely absent; preserves other frontmatter fields byte-identically when present.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `--commit` preserves TASKS.md body bytes

Body content (everything after the closing `---` frontmatter
fence) is preserved byte-identically across `--commit`.

<done-when>
- After `--commit`, the body bytes of TASKS.md are unchanged from
  before -- byte-by-byte equality of everything after the closing
  `---` line.
- Line endings in the body are preserved (CRLF stays CRLF; LF
  stays LF).
- Trailing whitespace in the body is preserved.
- The frontmatter region (between the opening `---` and the
  closing `---`, exclusive) may be rewritten, but only the two
  managed fields change semantically.
</done-when>

<behavior>
- Given a TASKS.md with CRLF line endings in the body, when
  `--commit` runs, then the body line endings remain CRLF.
- Given a TASKS.md with trailing whitespace on some task lines,
  when `--commit` runs, then trailing whitespace is preserved.
- Given a TASKS.md whose frontmatter has the field order `spec`,
  `generated_at`, `spec_hash_at_generation` (non-canonical
  order), when `--commit` runs, then the field order is
  preserved (no canonicalisation in v1).
</behavior>

<scenario id="CHK-006">
- Given a TASKS.md with CRLF line endings in the body, when
  `--commit` runs, then the body line endings remain CRLF.
- Given a TASKS.md with trailing whitespace on some task lines,
  when `--commit` runs, then trailing whitespace is preserved.
- Given a TASKS.md whose frontmatter has the field order `spec`,
  `generated_at`, `spec_hash_at_generation` (non-canonical
  order), when `--commit` runs, then the field order is
  preserved (no canonicalisation in v1).

Body bytes (everything after the closing --- fence) are byte-identical before and after commit_frontmatter; CRLF line endings, trailing whitespace, and arbitrary content are all preserved verbatim.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: SPEC-ID argument validation and state checks

Validate the argument and the workspace state.

<done-when>
- If the SPEC-ID argument doesn't match `SPEC-\d{4,}`, exit code
  1 with a format-error message.
- If no spec directory matches the ID, exit code 1 with a "spec
  not found" message naming the ID.
- If SPEC.md fails to parse (any form -- initial, amendment, or
  `--commit`), exit code 1 with the parser error.
- For `--commit` only: if TASKS.md is absent, exit code 1 with a
  clear "TASKS.md not found; was it generated by the agent?"
  message.
</done-when>

<behavior>
- `speccy tasks FOO` -> exit 1, format error.
- `speccy tasks SPEC-9999` -> exit 1, "spec not found".
- `speccy tasks SPEC-0001 --commit` with no TASKS.md -> exit 1,
  TASKS.md-missing error.
- `speccy tasks SPEC-0001` (TASKS.md missing -- initial form) ->
  exit 0, renders the initial prompt (not an error).
</behavior>

<scenario id="CHK-007">
speccy tasks with invalid ID format exits 1; with unknown SPEC-NNNN exits 1; SPEC.md parse failure exits 1 with the parser error.
</scenario>

<scenario id="CHK-008">
speccy tasks SPEC-NNNN --commit with TASKS.md absent exits 1 with a TasksMdNotFound error naming the missing path; the same command without --commit succeeds (initial prompt rendered).
</scenario>

</requirement>

## Design

### Approach

The command lives in `speccy-cli/src/tasks.rs`. The frontmatter
rewrite logic lives in `speccy-core/src/tasks.rs` as
`commit_frontmatter(tasks_md_path, spec_id, spec_md_hash, now) ->
Result<(), CommitError>` -- a narrow helper, not part of the
general `prompt` module.

Flow per invocation:

1. Discover project root.
2. Parse SPEC-ID argument; resolve to spec directory.
3. Parse SPEC.md (always; both prompt-rendering and `--commit`
   need it).
4. Branch:
   - `--commit`: read TASKS.md raw bytes; rewrite frontmatter;
     write back.
   - No `--commit`, TASKS.md absent: render initial prompt to
     stdout.
   - No `--commit`, TASKS.md present: parse TASKS.md; render
     amendment prompt to stdout.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: `--commit` is a sub-action, not a separate verb

**Status:** Accepted (per ARCHITECTURE.md "CLI Surface")
**Context:** ARCHITECTURE.md keeps the CLI surface flat: ten commands,
no mode toggles. Adding `speccy commit` or `speccy tasks-commit`
as a separate verb would inflate the surface for a narrow
sub-action.
**Decision:** `--commit` is a flag on `speccy tasks`. Without it,
the command renders a prompt. With it, the command writes
frontmatter. The two modes are mutually exclusive in effect.
**Alternatives:**
- Separate verb (e.g. `speccy commit-tasks`) -- rejected. Bloats
  the surface.
- Sub-command (`speccy tasks commit SPEC-NNN`) -- rejected.
  Skills already use the flat surface; sub-commands add a layer.
**Consequences:** Skills that orchestrate both phases call
`speccy tasks SPEC-NNN` then `speccy tasks SPEC-NNN --commit`.
Clear and consistent.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Body-byte-preserving frontmatter rewrite

**Status:** Accepted
**Context:** The agent's TASKS.md prose is hand-written; round-
tripping through a markdown formatter would change whitespace,
list ordering, or escape sequences in subtle ways. Diff-noise on
every `--commit` invocation would erode trust in the tool.
**Decision:** Read TASKS.md as raw bytes. Locate the frontmatter
region via the SPEC-0001 frontmatter splitter. Rewrite only the
two managed fields in the YAML region. Concatenate: new
frontmatter + closing fence + original body bytes verbatim.
**Alternatives:**
- Parse YAML + body, re-serialise both -- rejected. Markdown
  reformatting is lossy.
- Use a markdown AST round-trip -- rejected. Same problem,
  larger surface area.
**Consequences:** The implementation needs careful byte-level
handling. The trade-off is a no-diff guarantee for the body,
which is what users care about.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Form auto-detected by TASKS.md presence

**Status:** Accepted
**Context:** Two prompt forms (initial decomposition vs
amendment) could be selected by an explicit `--initial` /
`--amend` flag. But that flag is determined entirely by
filesystem state; making the user pass it would be redundant
and error-prone.
**Decision:** TASKS.md absent -> initial template; present ->
amendment template. No flag.
**Alternatives:**
- `--initial` / `--amend` flags -- rejected. Filesystem-derived
  state belongs at the CLI, not in the user's hands.
**Consequences:** Skills don't need to track state externally;
the filesystem is the source of truth.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: UTC ISO 8601 timestamps; second precision

**Status:** Accepted
**Context:** TASKS.md `generated_at` needs a stable, parseable
format.
**Decision:** UTC, ISO 8601, second precision:
`YYYY-MM-DDTHH:MM:SSZ`. The `Z` suffix is canonical UTC; no
offset variants.
**Alternatives:**
- Millisecond precision -- rejected for v1. Second precision is
  sufficient for staleness detection and avoids noisy diffs on
  back-to-back commits.
- Local timezone -- rejected. Repos shared across timezones
  would diff on every commit.
**Consequences:** Two `--commit` invocations within the same
second produce byte-identical frontmatter -- a nice property for
testing.
</decision>

### Interfaces

```rust
// speccy-core additions
pub mod tasks {
    pub fn commit_frontmatter(
        tasks_md_path: &Path,
        spec_id: &str,
        spec_md_sha256: &[u8; 32],
        now_utc: DateTime<Utc>,
    ) -> Result<(), CommitError>;
}

pub enum CommitError {
    TasksMdNotFound { path: PathBuf },
    SpecIdMismatch { in_file: String, in_arg: String },
    Io(std::io::Error),
    Parse(ParseError),
}

// speccy binary
pub fn run(args: TasksArgs) -> Result<(), TasksError>;

pub struct TasksArgs {
    pub spec_id: String,
    pub commit: bool,
}

pub enum TasksError {
    InvalidSpecIdFormat { arg: String },
    SpecNotFound { id: String },
    ProjectRootNotFound,
    Prompt(PromptError),
    Parse(ParseError),
    Commit(CommitError),
}
```

### Data changes

- New `speccy-core/src/tasks.rs` (frontmatter rewriter +
  CommitError).
- New `speccy-cli/src/tasks.rs` (command logic).
- New embedded templates at `skills/shared/prompts/tasks-generate.md`
  and `skills/shared/prompts/tasks-amend.md` (initial stubs;
  SPEC-0013 fills in real prompts).
- `speccy-core/Cargo.toml` adds `chrono` (or `time`) for
  UTC timestamp formatting.

### Migration / rollback

Greenfield. Rollback via `git revert`. Depends on SPEC-0001
(parsers + SpecMd.sha256), SPEC-0004 (workspace::scan), and
SPEC-0005 (prompt helpers) -- all deepened.

## Open questions

- [ ] Should `--commit` regenerate `generated_at` even when the
  hash hasn't changed? Yes for v1 (deterministic "this --commit
  happened at time T") even if redundant. Defer revision until
  a real use case suggests otherwise.
- [ ] Should the canonical field order be enforced on rewrite, or
  preserve agent-introduced ordering? Preserve in v1 (DEC-002
  consequence). A future lint could normalise.
- [ ] Should we also accept `--commit` with no SPEC-ID and infer
  it from `git diff`? No for v1; explicit is simpler.

## Assumptions

<assumptions>
- `speccy_core::prompt::{load_template, render, load_agents_md,
  trim_to_budget}` from SPEC-0005 are available.
- `speccy_core::workspace::find_root` and spec discovery from
  SPEC-0004 are available.
- The SPEC-0001 frontmatter splitter handles CRLF correctly
  (per SPEC-0001 REQ-002), enabling the body-byte preservation
  guarantee on Windows checkouts.
- `chrono::Utc::now()` (or equivalent) is acceptable for a
  monotonic-ish "now" -- exact monotonicity isn't required.
</assumptions>

## Changelog

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
</changelog>

## Notes

The body-byte-preserving rewrite (DEC-002) is the load-bearing
part of this spec. Every `--commit` invocation that produces a
git diff outside the frontmatter region is a bug; the test for
REQ-004 should be merciless about byte-equality.

When SPEC-0008 (`implement`) lands, it will read TASKS.md to
locate the task being implemented. SPEC-0008 doesn't write
TASKS.md (the implementer-agent does), so the body-preservation
guarantee here doesn't extend across that boundary -- only across
`--commit`.
