---
id: SPEC-0009
slug: review-command
title: speccy review -- render Phase 4 reviewer prompt per persona
status: in-progress
created: 2026-05-11
---

# SPEC-0009: speccy review

## Summary

`speccy review TASK-ID --persona <name>` is the Phase 4 command.
It renders the prompt that one reviewer sub-agent reads to produce
an inline review note on one task, scoped to one persona's
perspective.

The fan-out across multiple personas is the **orchestrating
skill's** job (`/speccy-review` in SPEC-0013), not the CLI's. The
CLI per-invocation renders exactly one persona's prompt. This
keeps the CLI deterministic: no concurrency, no parallel
processes, no shared state.

Three pieces of context get inlined:

- **Task context** -- the spec's SPEC.md (including Decisions),
  the task entry from TASKS.md with all prior notes, AGENTS.md.
- **Diff** -- the work the implementer just produced. Working
  tree vs HEAD by default; HEAD vs HEAD~1 if the tree is clean;
  a "no diff available" note if neither yields content.
- **Persona content** -- the markdown for the named persona,
  resolved via `.speccy/skills/personas/reviewer-<name>.md`
  (project-local override) before the embedded bundle.

The persona registry covers six names: the four default fan-out
personas (`business`, `tests`, `security`, `style` -- shared with
SPEC-0007's `DEFAULT_PERSONAS`) plus two off-by-default personas
(`architecture`, `docs`). Unknown names error with the list.

## Goals

- One CLI surface per persona-per-task review.
- Persona registry covers six names; project-local overrides
  work without rebuilding speccy.
- Diff is best-effort with a clear fallback chain so reviews work
  whether the implementer committed or not.
- Reuse SPEC-0008's `task_lookup` (no duplication).
- Reuse SPEC-0005's prompt infrastructure.

## Non-goals

- No CLI-level fan-out. One persona per invocation.
- No state mutation. The reviewer-agent writes inline notes to
  TASKS.md; the CLI never edits any file.
- No diff against arbitrary refs (always HEAD-based).
- No host-native location in the persona lookup chain (see
  DEC-002).
- No project-level fan-out config in v1.

## User stories

- As `/speccy-review` (the review-loop skill), I want to invoke
  `speccy review T-003 --persona security` for each persona in
  the fan-out list and get back a prompt per persona.
- As a project that wants stricter security review, I want to
  override the shipped persona by writing
  `.speccy/skills/personas/reviewer-security.md` and have the CLI
  pick it up automatically.
- As a reviewer-agent, I want the rendered prompt to include the
  diff the implementer produced, so I'm reviewing the actual
  work, not just SPEC.md.

## Requirements

### REQ-001: Persona registry

The full set of six persona names is exposed as a constant.

**Done when:**
- `speccy_core::personas::ALL` is a `&'static [&'static str]` with
  values `["business", "tests", "security", "style",
  "architecture", "docs"]`.
- `SPEC-0007::DEFAULT_PERSONAS` is a subset:
  `["business", "tests", "security", "style"]`.
- The two lists are derived from the same source of truth (the
  full ALL list), not duplicated.

**Behavior:**
- Adding a new persona means appending to one constant.
- A test asserts `DEFAULT_PERSONAS` is a prefix of `ALL`.

**Covered by:** CHK-001

### REQ-002: Persona file resolution

Resolve persona content in order: project-local override first;
embedded bundle second.

**Done when:**
- `personas::resolve_file(name, project_root) -> Result<String,
  PersonaError>` resolves in this order:
  1. `<project_root>/.speccy/skills/personas/reviewer-<name>.md`
     (UTF-8 read; returned verbatim).
  2. Embedded bundle entry
     `skills/shared/personas/reviewer-<name>.md` via
     `prompt::load_template`.
- If the project-local file exists but is empty or unreadable,
  the function logs a warning to stderr and falls through to the
  embedded version (lint will surface this separately).
- If neither location has the persona, return
  `PersonaError::NotFound`.
- The host-native location (`.claude/commands/` etc.) is
  **not** in the resolution chain.

**Behavior:**
- Given `.speccy/skills/personas/reviewer-security.md` exists
  with content `# Custom security`, `resolve_file("security",
  ...)` returns that content.
- Given that override does NOT exist but the embedded bundle has
  `reviewer-security.md`, the function returns the embedded
  content.
- Given neither exists, `PersonaError::NotFound { name:
  "security" }`.
- Given the project-local override is an empty file, the
  function warns on stderr and returns the embedded content.

**Covered by:** CHK-002, CHK-003

### REQ-003: `--persona` argument validation

`--persona` is required and must be one of the six registry
names.

**Done when:**
- Missing `--persona` returns exit code 1 with a usage message
  listing the registry names.
- A `--persona` value not in `ALL` returns exit code 1 with a
  message naming the unknown value and listing the valid names.
- Case-sensitive matching (e.g. `--persona Security` is
  rejected).

**Behavior:**
- `speccy review T-001 --persona security` -> succeeds (security
  is in ALL).
- `speccy review T-001 --persona unknown` -> exit 1; stderr
  contains `unknown` and the six valid names.
- `speccy review T-001` (no --persona) -> exit 1 with usage.

**Covered by:** CHK-004

### REQ-004: Diff computation with fallback chain

Compute the diff to inline into the prompt; fall back gracefully.

**Done when:**
- The command shells out to `git diff HEAD` first.
- If that command produces empty output AND exits 0 (working
  tree is clean), shell out to `git diff HEAD~1 HEAD`.
- If that also produces empty output (or fails -- e.g. no
  parent commit), use the literal note string
  `<!-- no diff available; review based on SPEC.md and task notes alone -->`.
- If `git` is not on PATH or the directory is not a git repo,
  use the same fallback note.
- The diff content (or fallback note) is captured as a `String`
  and substituted into the template at `{{diff}}`.

**Behavior:**
- Given uncommitted edits exist, `{{diff}}` contains the
  `git diff HEAD` output.
- Given the working tree is clean and there's at least one
  commit ahead of `HEAD~1`, `{{diff}}` contains the HEAD vs
  HEAD~1 diff.
- Given a fresh repo with one commit (no HEAD~1) and clean
  working tree, `{{diff}}` is the fallback note.
- Given `git` isn't installed, `{{diff}}` is the fallback note
  (no error).

**Covered by:** CHK-005

### REQ-005: Render reviewer prompt

Render the Phase 4 prompt for the resolved persona.

**Done when:**
- The command:
  1. Looks up the task via `task_lookup::find` (SPEC-0008
     REQ-002).
  2. Computes the diff (REQ-004).
  3. Resolves the persona content (REQ-002).
  4. Loads the embedded `reviewer-<persona>.md` template via
     `prompt::load_template` (note: this is the prompt
     template, not the persona content -- they live in
     different bundle paths).
  5. Substitutes placeholders: `{{spec_id}}`, `{{spec_md}}`,
     `{{task_id}}`, `{{task_entry}}`, `{{diff}}`, `{{persona}}`
     (name), `{{persona_content}}` (resolved persona file),
     `{{agents}}`.
  6. Trims to budget.
  7. Writes to stdout.

**Behavior:**
- The rendered prompt contains the persona content (whether
  project-local or embedded).
- The diff (or fallback note) appears where `{{diff}}` was.
- Task entry includes the task line plus every sub-list bullet,
  same as SPEC-0008 DEC-004.

**Covered by:** CHK-006

### REQ-006: Reuse SPEC-0008's task lookup; error mapping

Task lookup errors propagate through the same shape as SPEC-0008.

**Done when:**
- `speccy review` uses `task_lookup::find` directly. No
  duplicate lookup code in the binary crate.
- `LookupError::InvalidFormat / NotFound / Ambiguous` map to the
  same exit codes and stderr messages as SPEC-0008.
- Ambiguity stderr shows
  `speccy review SPEC-NNNN/T-NNN --persona <name>` as the
  copy-pasteable correction (the `--persona` flag is included
  for completeness).

**Behavior:**
- An ambiguous task ref produces the same error shape as
  SPEC-0008, with the suggested command rewritten for review.

**Covered by:** CHK-007

## Design

### Approach

The command lives in `speccy-cli/src/review.rs`. New helpers
land in `speccy-core/src/personas.rs` (registry + file
resolver). The diff computer lives in
`speccy-cli/src/git.rs` (binary-crate, since `speccy verify`
also shells out to git and could share later).

Flow per invocation:

1. Discover project root.
2. Scan workspace.
3. Parse `--persona`; validate against `personas::ALL`.
4. Parse `TASK-ID`; locate the task via `task_lookup::find`.
5. Compute the diff (HEAD; HEAD~1; fallback note).
6. Resolve the persona content (project-local; embedded).
7. Load the `reviewer-<persona>.md` prompt template.
8. Substitute placeholders.
9. Trim to budget.
10. Write to stdout.

### Decisions

#### DEC-001: Persona registry hardcoded; six names

**Status:** Accepted (per DESIGN.md)
**Context:** DESIGN.md fixes six personas with the default
fan-out subset.
**Decision:** `personas::ALL` is a const slice. No runtime
extensibility.
**Alternatives:**
- Project-level registry config -- rejected for v1.
- Auto-discovery from `.speccy/skills/personas/` -- rejected.
  Magic; the CLI shouldn't define semantics from file presence.
**Consequences:** Adding a persona is a code change. Acceptable
for v1.

#### DEC-002: Persona lookup: project-local first; embedded second; no host-native

**Status:** Accepted
**Context:** DESIGN.md is ambiguous on where the "shipped"
persona files live at runtime: the file layout section
references `skills/shared/personas/` (embedded), but the
"Persona file resolution" section mentions
`.claude/commands/` as the shipped path. The host-native files
have host-specific frontmatter and aren't suitable for direct
inlining as persona content.
**Decision:** Lookup order is:
1. `.speccy/skills/personas/reviewer-<name>.md` (project-local
   override; pure markdown).
2. Embedded bundle `skills/shared/personas/reviewer-<name>.md`
   (shipped; pure markdown).

The host-native location (`.claude/commands/`,
`.codex/skills/`) is **out** of the resolution chain.
**Alternatives:**
- Add host-native as a third lookup step -- rejected.
  Host-native files have host-specific frontmatter; treating
  them as persona content would inject confusing instructions
  into the review prompt.
- Project-local only -- rejected. Forces every project to
  author its own personas.
**Consequences:** DESIGN.md may want a one-line clarification
in a future amendment. The behaviour is unambiguous at the
spec level.

#### DEC-003: Diff via shell-out to git; documented fallback chain

**Status:** Accepted (per DESIGN.md "speccy review diff scoping")
**Context:** Reviewers need to see the work. The implementer
might have committed or not.
**Decision:** Shell out to `git`:
1. `git diff HEAD` (working tree vs HEAD).
2. If empty + clean: `git diff HEAD~1 HEAD`.
3. If still empty or git fails: literal fallback note.

No use of `gix` or similar library; one subprocess (sometimes
two) per invocation is cheap.
**Alternatives:**
- `gix` library -- rejected for v1; same reasoning as
  SPEC-0004 DEC-003.
- Only `git diff HEAD` (no fallback) -- rejected. Misses
  committed implementations.
**Consequences:** Reviewers always have *something* under
`{{diff}}` -- either real content or a clear "no diff" note.

#### DEC-004: `--persona` is required; no implicit default

**Status:** Accepted
**Context:** DESIGN.md notes review "is the only phase with
parallel sub-types," and the persona fan-out is the
orchestrating skill's job. The CLI runs one persona at a time.
**Decision:** `--persona` has no default; missing it is a
usage error.
**Alternatives:**
- Default to `business` -- rejected. Silently doing the wrong
  thing is worse than an explicit error.
- Loop over default fan-out -- rejected. The CLI doesn't
  fan out; that's the skill's job.
**Consequences:** Skills calling `speccy review` always pass
`--persona`. Manual invocations always need it too.

### Interfaces

```rust
// speccy-core additions
pub mod personas {
    pub const ALL: &[&str] = &[
        "business", "tests", "security",
        "style", "architecture", "docs",
    ];

    pub fn resolve_file(
        name: &str,
        project_root: &Path,
    ) -> Result<String, PersonaError>;
}

pub enum PersonaError {
    UnknownName { name: String, valid: &'static [&'static str] },
    NotFound { name: String },
    Io(std::io::Error),
}

// SPEC-0007 reuses ALL: DEFAULT_PERSONAS is the first 4 elements.

// speccy binary
pub fn run(args: ReviewArgs) -> Result<(), ReviewError>;

pub struct ReviewArgs {
    pub task_ref: String,
    pub persona: String,
}

pub enum ReviewError {
    Lookup(LookupError),
    Persona(PersonaError),
    Git(std::io::Error),                 // surfaces fallback note
    ProjectRootNotFound,
    Prompt(PromptError),
}
```

### Data changes

- New `speccy-core/src/personas.rs`.
- New `speccy-cli/src/review.rs`.
- New `speccy-cli/src/git.rs` (diff helper; future-shared
  with SPEC-0012 if useful).
- New embedded templates and persona files (SPEC-0013 fills
  these; this spec adds stubs):
  - `skills/shared/personas/reviewer-{business,tests,security,style,architecture,docs}.md`
  - `skills/shared/prompts/reviewer-{business,tests,security,style,architecture,docs}.md`

### Migration / rollback

Greenfield. Depends on SPEC-0001, SPEC-0004, SPEC-0005, SPEC-0008.

## Open questions

- [ ] Should there be one prompt template shared across all
  personas with `{{persona}}` substitution, or one template per
  persona? Per-persona is more flexible; defer to SPEC-0013's
  judgement on content shape.
- [ ] Should the diff be size-capped before inlining? A huge
  refactor produces a 50k-line diff that blows the budget.
  Budget trimming handles this generically; defer.
- [ ] Should reviewers see the `## Changelog` of the SPEC.md
  separately? The full SPEC.md inlines it; no separate
  placeholder needed in v1.

## Assumptions

- `task_lookup::find` from SPEC-0008 is stable.
- `prompt::*` helpers from SPEC-0005 are stable.
- `git` is on PATH on developer machines and CI runners; absence
  triggers the fallback note (no error).

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from DESIGN.md decomposition. |

## Notes

The persona registry (DEC-001) is the source of truth for
SPEC-0007's `DEFAULT_PERSONAS`. SPEC-0007 imports from this spec
rather than defining its own list -- the two have to stay
aligned. Implementer: when landing this spec, refactor SPEC-0007
to consume `personas::ALL[..4]` if it doesn't already.

DEC-002 resolves a real ambiguity in DESIGN.md. The DESIGN.md
amendment to align with this decision is a non-blocking
follow-up (a one-line edit to the "Persona file resolution"
section).
