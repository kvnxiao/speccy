---
id: SPEC-0008
slug: implement-command
title: speccy implement -- render Phase 3 implementer prompt
status: implemented
created: 2026-05-11
---

# SPEC-0008: speccy implement

## Summary

`speccy implement TASK-ID` is the Phase 3 command. It renders the
prompt that an implementer sub-agent reads to perform one task: a
template inlining the relevant SPEC.md, the task entry from
TASKS.md with all its prior notes, suggested files, AGENTS.md, and
the implementer skill content.

Task IDs are spec-scoped per DESIGN.md ("`**T-NNN**` IDs are stable
within the file"). The command accepts two argument forms:

- **Unqualified** `T-NNN` -- searches every spec; succeeds only
  when exactly one spec has a matching task.
- **Qualified** `SPEC-NNNN/T-NNN` -- scopes the lookup to one
  spec; unambiguous.

Unqualified IDs that match multiple specs return an
`Ambiguous` error listing the candidate spec IDs. This is a real
error, not auto-resolved -- ambiguity at the call site has no
correct guess.

This spec also lands the **shared task-lookup helper**
(`speccy_core::task_lookup::find`) that SPEC-0009 (`review`)
reuses.

## Goals

- One CLI surface for the Phase 3 prompt.
- Task lookup handles both unqualified and qualified forms.
- Ambiguous unqualified IDs error cleanly with candidates.
- Reuse SPEC-0005's prompt infrastructure (template, render,
  AGENTS.md, budget trimming).
- Land `task_lookup` in `speccy-core` so SPEC-0009 doesn't
  reinvent it.

## Non-goals

- No file mutation. The implementer-agent writes code and flips
  task state; the CLI just renders the prompt.
- No batch mode. One task per invocation.
- No auto-disambiguation. Ambiguous unqualified IDs error.
- No execution of `speccy check` from this command (the
  implementer-agent runs it manually per the template's
  instructions).

## User stories

- As `/speccy-work` (the implementation-loop skill), I want
  `speccy implement T-003` to give me a prompt I can hand to an
  implementer sub-agent with full context.
- As a developer with a multi-spec workspace where T-001 exists
  in both SPEC-0001 and SPEC-0002, I want a clear error message
  listing both candidates when I run `speccy implement T-001`,
  so I can re-run with `speccy implement SPEC-0001/T-001`.
- As a future SPEC-0009 implementer, I want a shared
  `task_lookup::find` helper so my review command doesn't
  duplicate the lookup logic.

## Requirements

### REQ-001: Task reference parsing

Parse the task argument into a typed `TaskRef` value.

**Done when:**
- The argument is parsed against two patterns:
  - Unqualified: `^T-\d{3,}$` -> `TaskRef::Unqualified { id }`.
  - Qualified: `^SPEC-\d{4,}/T-\d{3,}$` ->
    `TaskRef::Qualified { spec_id, task_id }`.
- Anything else returns `LookupError::InvalidFormat { arg }`.

**Behavior:**
- `"T-001"` parses to `Unqualified { id: "T-001" }`.
- `"SPEC-0001/T-001"` parses to `Qualified { spec_id:
  "SPEC-0001", task_id: "T-001" }`.
- `"T-1"` (no zero padding to 3 digits) parses successfully
  (we accept any digit count `>= 3` per the regex, since 4+ is
  also valid).
- `"FOO"`, `"T-"`, `"SPEC-0001/FOO"` all return
  `InvalidFormat` naming the input verbatim.

**Covered by:** CHK-001

### REQ-002: Workspace task lookup

Locate the matching task across the workspace.

**Done when:**
- `task_lookup::find(workspace, task_ref)` returns
  `Ok(TaskLocation)` when exactly one task matches:
  - `Unqualified`: every parsed spec's TASKS.md is searched; the
    one match wins.
  - `Qualified`: only the named spec is searched.
- If no spec has the task, returns `LookupError::NotFound`.
- If two or more specs have the same unqualified `T-NNN`,
  returns `LookupError::Ambiguous` (see REQ-003).
- Specs whose TASKS.md failed to parse are skipped silently --
  they can't contain the task and shouldn't poison the lookup.

**Behavior:**
- Given SPEC-0001 has T-001 and SPEC-0002 has T-002, when
  looking up `Unqualified { id: "T-001" }`, returns
  `Ok(TaskLocation { spec_id: "SPEC-0001", ... })`.
- Given SPEC-0001 has T-001 and SPEC-0002 has T-001, when
  looking up `Unqualified { id: "T-001" }`, returns
  `Err(Ambiguous { task_id: "T-001", candidate_specs:
  ["SPEC-0001", "SPEC-0002"] })`.
- Given the same workspace, when looking up
  `Qualified { spec_id: "SPEC-0001", task_id: "T-001" }`,
  returns `Ok(...)` (qualified bypasses ambiguity).
- Given no spec has T-999, lookup returns `NotFound`.

**Covered by:** CHK-002

### REQ-003: Ambiguity error with candidate list

Ambiguous unqualified IDs return a structured error.

**Done when:**
- `LookupError::Ambiguous { task_id, candidate_specs }` carries
  the requested `task_id` and the list of spec IDs where it was
  found.
- `candidate_specs` is in ascending spec-ID order (matches
  workspace::scan ordering).
- The CLI maps this error to exit code 1 with a stderr message
  listing each candidate as `speccy implement SPEC-NNNN/T-NNN`
  for the user to copy-paste.

**Behavior:**
- Given T-001 exists in SPEC-0001, SPEC-0003, SPEC-0005, the
  error's `candidate_specs` is `["SPEC-0001", "SPEC-0003",
  "SPEC-0005"]`.
- The stderr message reads:
  ```
  Error: T-001 is ambiguous; matches in 3 specs.
  Disambiguate with one of:
    speccy implement SPEC-0001/T-001
    speccy implement SPEC-0003/T-001
    speccy implement SPEC-0005/T-001
  ```

**Covered by:** CHK-003

### REQ-004: Render implementer prompt

Render the Phase 3 prompt to stdout.

**Done when:**
- The command loads the embedded `implementer.md` template via
  `prompt::load_template`.
- It substitutes placeholders:
  - `{{spec_id}}` -- the spec's SPEC-NNNN.
  - `{{spec_md}}` -- full SPEC.md content (including frontmatter).
  - `{{task_id}}` -- the resolved T-NNN.
  - `{{task_entry}}` -- the full task subtree from TASKS.md:
    the task line itself plus every sub-list bullet (notes,
    Covers, Suggested files, prior implementer notes, review
    notes).
  - `{{suggested_files}}` -- the parsed `Task.suggested_files`
    rendered as a comma-separated list (empty string if none).
  - `{{agents}}` -- AGENTS.md content via `prompt::load_agents_md`.
- The rendered prompt is trimmed via `prompt::trim_to_budget`.
- The trimmed output goes to stdout; exit code 0.

**Behavior:**
- Given SPEC-0001 with T-001 and three sub-list notes under
  T-001, the rendered prompt contains all three notes in
  declared order where `{{task_entry}}` was.
- Given AGENTS.md is missing, the rendered output includes the
  marker (per SPEC-0005 REQ-004) and the command still succeeds.
- Given a task whose `Covers:` bullet lists REQ-001 and REQ-002,
  the rendered output includes those IDs (within the task_entry
  inline).

**Covered by:** CHK-004

### REQ-005: Argument and state error handling

Map all error paths to exit codes and informative stderr.

**Done when:**
- `LookupError::InvalidFormat` -> exit 1, stderr names the bad
  argument and shows the accepted formats.
- `LookupError::NotFound` -> exit 1, stderr names the task and
  suggests `speccy status` to find candidates.
- `LookupError::Ambiguous` -> exit 1, stderr shows the
  candidate list per REQ-003.
- `WorkspaceError::NoSpeccyDir` (from outside a workspace) ->
  exit 1, stderr names the issue.
- `ParseError` on SPEC.md or TASKS.md -> exit 1, stderr shows
  the parser error.
- `PromptError::TemplateNotFound` -> exit 2 (internal failure;
  the embedded bundle should always have this template).

**Behavior:**
- Every exit-1 error path has an actionable stderr message
  (names what's wrong and what to do next).
- Exit codes are deterministic given identical inputs.

**Covered by:** CHK-005

## Design

### Approach

`speccy_core::task_lookup` is the new shared module. It exposes
`find` plus the `TaskRef`, `TaskLocation`, and `LookupError`
types. The command lives in `speccy-cli/src/implement.rs` and
is a thin wrapper: parse args, call `task_lookup::find`, assemble
prompt, write to stdout.

Flow per invocation:

1. Discover project root.
2. Scan workspace.
3. Parse the `TASK-ID` argument into a `TaskRef`.
4. Locate the task via `task_lookup::find`.
5. Load AGENTS.md.
6. Load the `implementer.md` template.
7. Substitute placeholders.
8. Trim to budget.
9. Write to stdout.

### Decisions

#### DEC-001: Task lookup as a shared `speccy-core` helper

**Status:** Accepted
**Context:** Both this spec and SPEC-0009 (`review`) need to
locate a task by `T-NNN`. Duplicating the logic guarantees
divergence over time.
**Decision:** Land `speccy_core::task_lookup::find` in this
spec. SPEC-0009 reuses it.
**Alternatives:**
- Per-command lookup -- rejected. Duplication risk.
**Consequences:** SPEC-0009's deps include this spec.

#### DEC-002: Qualified form uses `/` as the separator

**Status:** Accepted
**Context:** The qualified form needs an unambiguous separator.
Options: `/`, `:`, `#`, `.`. Most shells handle all of them
without escaping.
**Decision:** Use `/` -- reads as a path-like scope (`SPEC-0001`
is the "directory", `T-001` the "file").
**Alternatives:**
- `:` -- rejected. Conflicts with URL syntax; aesthetics weaker.
- `.` -- rejected. Less intuitive for "scope" semantics.
**Consequences:** No shell escaping concerns; the form reads
naturally.

#### DEC-003: Ambiguity is always an error; no auto-disambiguation

**Status:** Accepted
**Context:** When `T-001` exists in multiple specs, the CLI
could pick the lowest spec ID, the most-recently-modified one,
or ask interactively.
**Decision:** Error out with the candidate list. No
auto-resolution.
**Alternatives:**
- Pick lowest spec ID -- rejected. Surprising "right answer";
  the user might want a different spec.
- Interactive prompt -- rejected. The CLI is non-interactive
  (matches SPEC-0002 stance).
**Consequences:** Users with multi-spec workspaces who duplicate
`T-NNN` IDs (legitimate per DESIGN.md) must always qualify or
ensure each invocation is unambiguous in practice.

#### DEC-004: Task entry inlines the full sub-list

**Status:** Accepted
**Context:** The implementer needs every prior note: their own
past implementer notes (for retries), review notes (to address
blockers), the Covers bullet, suggested files, and any
freeform bullets.
**Decision:** `{{task_entry}}` includes the task's bold-ID line
PLUS every sub-list bullet under it, in declared order.
**Alternatives:**
- Per-field placeholders for each note type -- rejected. Too
  rigid; agents may add bullets in unstructured ways.
**Consequences:** The template author can rely on `{{task_entry}}`
being the complete picture for the task.

### Interfaces

```rust
// speccy-core additions
pub mod task_lookup {
    pub fn find<'a>(
        workspace: &'a Workspace,
        task_ref: &TaskRef,
    ) -> Result<TaskLocation<'a>, LookupError>;

    pub fn parse_ref(arg: &str) -> Result<TaskRef, LookupError>;
}

pub enum TaskRef {
    Unqualified { id: String },
    Qualified { spec_id: String, task_id: String },
}

pub struct TaskLocation<'a> {
    pub spec_id: String,
    pub spec_md: &'a SpecMd,
    pub tasks_md: &'a TasksMd,
    pub task: &'a Task,
    pub task_entry_raw: String,    // rendered sub-tree
}

pub enum LookupError {
    InvalidFormat { arg: String },
    NotFound { task_ref: String },
    Ambiguous { task_id: String, candidate_specs: Vec<String> },
}

// speccy binary
pub fn run(args: ImplementArgs) -> Result<(), ImplementError>;
pub struct ImplementArgs { pub task_ref: String }
```

### Data changes

- New `speccy-core/src/task_lookup.rs`.
- New `speccy-cli/src/implement.rs`.
- New embedded template at
  `skills/shared/prompts/implementer.md` (stub; SPEC-0013
  fills in real content).

### Migration / rollback

Greenfield. Rollback via `git revert`. Depends on SPEC-0001,
SPEC-0004 (workspace::scan), SPEC-0005 (prompt helpers).

## Open questions

- [ ] Should the qualified form also accept `SPEC-NNNN T-NNN`
  (space-separated)? Not in v1; `/` is the only form.
- [ ] Should `task_entry_raw` strip prior `Retry:` notes when
  rendering for a fresh implementation attempt? No for v1 --
  the implementer benefits from seeing why prior attempts were
  blocked.

## Assumptions

- The SPEC-0001 parser's TASKS.md handling exposes `Task.notes`
  as a Vec preserving declared order (per SPEC-0001 REQ-004).
- `prompt::load_template`, `render`, `load_agents_md`,
  `trim_to_budget` from SPEC-0005 are stable.
- The `implementer.md` template is in the embedded bundle by
  the time this command ships (SPEC-0013 fills it; initial stub
  is sufficient for testing).

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from DESIGN.md decomposition. |

## Notes

`task_lookup` is shared infrastructure. When SPEC-0009 deepens
(this turn), it should reference this module rather than
reimplementing. Both specs land in the same turn, so the
implementer can develop them in parallel with the lookup helper
landing first.
