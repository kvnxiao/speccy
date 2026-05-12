---
id: SPEC-0005
slug: plan-command
title: speccy plan -- render Phase 1 prompts (greenfield + amendment)
status: in-progress
created: 2026-05-11
---

# SPEC-0005: speccy plan

## Summary

`speccy plan` is the Phase 1 command. It renders a deterministic
prompt that an agent reads to author or amend a SPEC.md + spec.toml
pair. The CLI never invokes an LLM; it loads context
(VISION.md, AGENTS.md, the existing SPEC.md if amending),
substitutes placeholders into an embedded markdown template, and
prints the result.

Two forms:

- `speccy plan` (no arg) -- greenfield. Reads VISION.md, allocates
  the next available `SPEC-NNNN` ID, renders `plan-greenfield.md`.
- `speccy plan SPEC-NNNN` -- amendment. Reads the named SPEC.md,
  renders `plan-amend.md` (the agent is asked for a minimal
  surgical edit, not a rewrite).

This spec also lands the **shared prompt-rendering infrastructure**
in `speccy_core::prompt` that SPEC-0006 (`tasks`), SPEC-0008
(`implement`), SPEC-0009 (`review`), and SPEC-0011 (`report`)
reuse: template loading, placeholder substitution, AGENTS.md
loading, context-budget trimming, and spec ID allocation.

## Goals

- One CLI surface for both greenfield and amendment phases.
- Stable, simple template substitution (`{{NAME}}` placeholders);
  no Turing-complete templating engine.
- Shared prompt-rendering helpers in `speccy-core` so later
  prompt-emitting commands don't reinvent context loading or
  budget trimming.
- Deterministic output: same workspace state -> byte-identical
  rendered prompt across runs.

## Non-goals

- No LLM invocation. The CLI never calls a model.
- No interactive prompt selection. Templates ship embedded; not
  user-selectable in v1.
- No conditionals, loops, or filters in templates. Simple
  substitution only.
- No file mutation. `plan` is read-only; the agent it prompts
  writes SPEC.md and spec.toml.
- No per-host context-budget tuning in v1. One hardcoded budget
  (see DEC-004).

## User stories

- As a developer starting a new spec, I want `speccy plan` to give
  me a prompt my agent reads with VISION.md + AGENTS.md already
  inlined so the agent has the right context.
- As a developer amending a spec mid-loop, I want `speccy plan
  SPEC-0042` to render a "minimal-diff" prompt that respects the
  existing SPEC.md rather than asking for a rewrite.
- As a future SPEC-0006 implementer, I want shared prompt helpers
  in `speccy-core` so my `speccy tasks` command doesn't reimplement
  AGENTS.md loading, template rendering, or budget trimming.

## Requirements

### REQ-001: Greenfield prompt rendering

`speccy plan` with no arg renders the greenfield prompt.

**Done when:**
- The command discovers the project root via
  `speccy_core::workspace::find_root`.
- It reads `.speccy/VISION.md`. If missing, exit code 1 with a
  clear message naming the expected path.
- It scans `.speccy/specs/` and allocates the next `SPEC-NNNN` ID
  via `prompt::allocate_next_spec_id` (REQ-003).
- It loads AGENTS.md via `prompt::load_agents_md` (REQ-004).
- It loads the embedded `plan-greenfield.md` template via
  `prompt::load_template` and substitutes placeholders:
  `{{vision}}`, `{{agents}}`, `{{next_spec_id}}`.
- It trims the rendered output to the budget via
  `prompt::trim_to_budget` (REQ-006).
- It writes the final rendered prompt to stdout; exits 0.

**Behavior:**
- Given `.speccy/VISION.md` exists and no specs yet, when
  `speccy plan` runs, then `{{next_spec_id}}` substitutes to
  `SPEC-0001`.
- Given specs SPEC-0001 through SPEC-0013 exist, when `speccy
  plan` runs, then `{{next_spec_id}}` substitutes to `SPEC-0014`.
- Given specs SPEC-0001 and SPEC-0003 exist (gap at 0002), then
  `{{next_spec_id}}` substitutes to `SPEC-0004` (no gap
  recycling, per DEC-005).
- Given VISION.md is missing, then exit code is 1 with a stderr
  message naming `.speccy/VISION.md`.

**Covered by:** CHK-001, CHK-002

### REQ-002: Amendment prompt rendering

`speccy plan SPEC-NNNN` renders the amendment prompt.

**Done when:**
- The command parses the SPEC-ID argument. If it doesn't match
  `SPEC-\d{4,}`, exit code 1 with a format-error message.
- It discovers the spec directory matching the ID. If no matching
  directory exists, exit code 1 with a "spec not found" message
  naming the ID.
- It reads the matching SPEC.md via the SPEC-0001 parser. Parse
  errors return exit code 1 with the parser's error message.
- It loads AGENTS.md (REQ-004).
- It loads the embedded `plan-amend.md` template and substitutes
  placeholders: `{{spec_id}}`, `{{spec_md}}` (full content),
  `{{agents}}`, `{{changelog}}` (the existing Changelog table
  rows for context).
- It trims the rendered output to the budget.
- It writes the final rendered prompt to stdout; exits 0.

**Behavior:**
- Given `speccy plan SPEC-0001` and
  `.speccy/specs/0001-artifact-parsers/` exists, when the command
  runs, then the rendered output contains the full SPEC.md content
  and the amendment-mode template language.
- Given `speccy plan SPEC-9999` and no such spec exists, then exit
  code is 1 and stderr names SPEC-9999.
- Given `speccy plan FOO`, then exit code is 1 with a format
  error naming the invalid argument.

**Covered by:** CHK-003

### REQ-003: Spec ID allocation

Allocate the next available `SPEC-NNNN` ID by scanning `specs/`.

**Done when:**
- `prompt::allocate_next_spec_id(specs_dir: &Path) -> String`
  reads every immediate subdirectory of `specs_dir` matching
  `^\d{4}-`, parses the numeric prefix, and returns
  `max(prefixes) + 1` zero-padded to 4 digits (e.g. `"0014"`).
- If `specs_dir` is empty or absent, returns `"0001"`.
- Non-matching directories are silently ignored.
- The function does only the directory listing -- no per-spec
  parsing.

**Behavior:**
- Given an empty `specs/`, the allocator returns `"0001"`.
- Given `0001-foo` and `0003-bar`, the allocator returns `"0004"`
  (no gap recycling).
- Given `0042-foo` only, the allocator returns `"0043"`.
- Given a non-matching directory `_scratch` alongside `0001-foo`,
  the allocator returns `"0002"`.

**Covered by:** CHK-004

### REQ-004: AGENTS.md loading (cross-cutting helper)

Load AGENTS.md from the project root for inclusion in every
prompt-rendering command's output.

**Done when:**
- `prompt::load_agents_md(project_root: &Path) -> String`:
  - Returns the file content if `<project_root>/AGENTS.md` exists.
  - Returns a literal marker
    `<!-- AGENTS.md missing; project conventions not loaded -->`
    AND prints a one-line stderr warning if the file is missing.
  - On I/O error, returns a marker
    `<!-- AGENTS.md unreadable: <err> -->` and stderr warning.
- The function is consumed by SPEC-0005 (this spec), SPEC-0006,
  SPEC-0008, SPEC-0009, SPEC-0011.

**Behavior:**
- Given AGENTS.md exists with content `# Agents\n<rest>`, the
  function returns that content verbatim.
- Given AGENTS.md is missing, the function returns the marker and
  stderr contains a warning naming the expected path.
- Given AGENTS.md exists but is unreadable (permission denied),
  the function returns the error-marker and stderr warns.

**Covered by:** CHK-005

### REQ-005: Template loading and placeholder substitution (cross-cutting helper)

Load embedded prompt templates and substitute `{{NAME}}`
placeholders.

**Done when:**
- `prompt::load_template(name: &str) -> Result<&'static str, PromptError>`
  returns the template body from the embedded bundle. `name` is
  the file name within `skills/shared/prompts/` (e.g.
  `"plan-greenfield.md"`).
- Unknown template names return
  `PromptError::TemplateNotFound { name }`.
- `prompt::render(template: &str, vars: &BTreeMap<&str, String>) -> String`
  substitutes every `{{name}}` (case-sensitive, exact-match) with
  the corresponding value from `vars`.
- Substitution is **single-pass**: substituted text is not
  re-scanned for placeholders.
- Unrecognised placeholders (`{{foo}}` where `foo` is not in
  `vars`) are left in place AND a stderr warning is printed naming
  each unique unmatched placeholder.

**Behavior:**
- Given template `"hello {{name}}"` and `vars = {"name": "world"}`,
  render returns `"hello world"`.
- Given template `"{{a}} {{b}}"` and
  `vars = {"a": "{{b}}", "b": "x"}`, render returns
  `"{{b}} x"` -- single-pass means the substituted `{{b}}` from
  `a` is NOT re-scanned.
- Given template `"{{unknown}}"` with empty vars, render returns
  `"{{unknown}}"` and stderr contains a warning naming `unknown`.
- Given `load_template("nope.md")`, the result is
  `PromptError::TemplateNotFound { name: "nope.md" }`.

**Covered by:** CHK-006, CHK-007

### REQ-006: Context-budget trimming (cross-cutting helper)

Drop low-priority sections from the rendered prompt when it
exceeds a budget threshold.

**Done when:**
- `prompt::trim_to_budget(rendered: String, budget: usize) -> TrimResult`
  returns `{ output: String, dropped: Vec<String>, fits: bool }`.
- Drop ordering matches DESIGN.md "Prompt context budget":
  1. `## Notes` section content (from inlined SPEC.md sections).
  2. Answered `## Open questions` entries (`- [x]` items).
  3. `## Changelog` rows older than the 5 most recent.
  4. Task review notes older than the 3 most recent per task
     (n/a for plan; relevant when SPEC-0008 inlines TASKS.md).
  5. Other specs' summaries (n/a for plan; reserved for
     multi-spec context).
- After applying every applicable drop step, if the result still
  exceeds the budget, `fits = false`; the output is emitted
  anyway with a stderr warning naming the budget overrun.
- The default budget threshold is 80,000 characters
  (see DEC-004).
- The function is consumed by SPEC-0005, SPEC-0006, SPEC-0008,
  SPEC-0009, SPEC-0011.

**Behavior:**
- Given a 60,000-char rendered prompt and an 80,000 budget,
  `output == rendered`, `dropped = []`, `fits = true`.
- Given a 100,000-char rendered prompt with a 5,000-char `##
  Notes` section and 30,000 chars of answered open questions:
  dropping `## Notes` yields 95,000 (still over), then dropping
  answered questions yields 65,000 (under). Returns
  `dropped = ["## Notes", "answered open questions"]`,
  `fits = true`.
- Given a 200,000-char prompt where even all five drop steps
  leave the result at 150,000, `fits = false`, stderr warns about
  the overrun, and the 150,000-char output is emitted.

**Covered by:** CHK-008

## Design

### Approach

The command lives in `speccy-cli/src/plan.rs`. Cross-cutting
helpers (`load_template`, `render`, `load_agents_md`,
`trim_to_budget`, `allocate_next_spec_id`) live in
`speccy-core/src/prompt/`. The embedded prompt bundle is
the same one SPEC-0002 introduced for skill packs; the
implementer may either share access to one bundle in
`speccy-core` or keep parallel bundles. The spec contract is
"templates are loaded from an embedded bundle"; the bundle
location is implementation detail.

Flow per invocation:

1. Discover project root.
2. Branch on argument presence (greenfield vs amendment).
3. Load AGENTS.md.
4. Load VISION.md (greenfield) or the named SPEC.md (amendment).
5. Allocate the next spec ID (greenfield only).
6. Load the relevant template.
7. Substitute placeholders.
8. Trim to budget.
9. Write rendered output to stdout.

### Decisions

#### DEC-001: Simple `{{NAME}}` placeholder substitution

**Status:** Accepted
**Context:** Templates need variable inlining. Full templating
engines (Handlebars, Tera, Liquid) add complexity -- conditionals,
loops, partials, filters -- none of which we need in v1.
**Decision:** Simple, single-pass `{{NAME}}` string substitution.
Names are alphanumeric + underscore. No conditionals, no loops,
no filters.
**Alternatives:**
- Handlebars / Tera -- rejected. Too much surface area for v1.
- Hand-rolled regex with no public abstraction -- rejected. We
  want a stable function so SPEC-0006+ have a clear contract.
**Consequences:** Templates are dead-simple to author. If logic
is needed, the CLI computes it ahead of time and passes the
result as a variable.

#### DEC-002: Prompt templates ship embedded in the binary

**Status:** Accepted
**Context:** Consistent with SPEC-0002 DEC-001 (skill packs via
`include_dir!`).
**Decision:** Use `include_dir!` to embed
`skills/shared/prompts/` at compile time. Templates are accessed
by name (file name within the bundle).
**Alternatives:**
- `const &str` literals in source -- rejected. Doesn't scale;
  loses path semantics.
- Runtime fetch -- rejected. No network access for the CLI.
**Consequences:** Updating prompts requires a speccy release.
Matches SPEC-0002 DEC-001 reasoning.

#### DEC-003: AGENTS.md missing is a warning, not an error

**Status:** Accepted
**Context:** AGENTS.md is expected at the project root, but a
fresh repo might not have one yet. Erroring out blocks the very
first `speccy plan` invocation in a new repo.
**Decision:** Missing AGENTS.md returns a marker string and emits
a stderr warning. The rendered prompt still goes out; the agent
reading it sees the marker and knows project conventions are
not loaded.
**Alternatives:**
- Error out -- rejected. Blocks first-time use.
- Silently substitute empty string -- rejected. No signal to the
  agent that conventions are missing.
**Consequences:** Agents reading the rendered prompt may produce
work that doesn't follow conventions if AGENTS.md is missing.
Acceptable; the marker makes the gap visible to both the agent
and the developer.

#### DEC-004: Hardcoded context budget at 80,000 characters in v1

**Status:** Accepted
**Context:** Host context windows vary (Claude ~200k tokens,
GPT-4 ~128k tokens, smaller models far less). Speccy doesn't
know which model will read the prompt.
**Decision:** Hardcode 80,000 characters (~20,000 tokens). Safe
default across all modern hosts.
**Alternatives:**
- Per-host config in `speccy.toml` -- rejected for v1. Adds
  configuration surface; revisit when concrete need arises.
- No budget enforcement -- rejected. Long-running specs would
  blow past limits silently.
**Consequences:** Users with smaller-context hosts may still hit
limits; users with larger hosts get more aggressive trimming
than needed. Documented as a known limitation.

#### DEC-005: Spec ID allocation is "max + 1"; no gap recycling

**Status:** Accepted (per DESIGN.md "Spec ID allocation")
**Context:** Gaps left by dropped specs could be recycled. But
recycling means a dropped SPEC-0007 could later mean something
completely different, confusing anyone reading historical
commits or PR descriptions referencing the old ID.
**Decision:** Always allocate `max(existing) + 1`. Dropped specs
leave permanent gaps.
**Alternatives:**
- Recycle gaps -- rejected per historical-ambiguity reasoning.
**Consequences:** Spec ID space grows monotonically. Acceptable.

### Interfaces

```rust
// speccy-core additions
pub mod prompt {
    pub fn load_template(name: &str) -> Result<&'static str, PromptError>;
    pub fn render(
        template: &str,
        vars: &BTreeMap<&str, String>,
    ) -> String;
    pub fn load_agents_md(project_root: &Path) -> String;
    pub fn trim_to_budget(rendered: String, budget: usize) -> TrimResult;
    pub fn allocate_next_spec_id(specs_dir: &Path) -> String;
}

pub enum PromptError {
    TemplateNotFound { name: String },
    Io(std::io::Error),
}

pub struct TrimResult {
    pub output: String,
    pub dropped: Vec<String>,
    pub fits: bool,
}

pub const DEFAULT_BUDGET: usize = 80_000;  // chars

// speccy binary
pub fn run(args: PlanArgs) -> Result<(), PlanError>;

pub struct PlanArgs {
    pub spec_id: Option<String>,         // None = greenfield
}

pub enum PlanError {
    InvalidSpecIdFormat { arg: String },
    SpecNotFound { id: String },
    VisionMissing,
    ProjectRootNotFound,
    Prompt(PromptError),
    Parse(ParseError),
}
```

### Data changes

- New `speccy-core/src/prompt/mod.rs` and submodules
  (`template`, `render`, `agents_md`, `budget`, `id_alloc`).
- New `speccy-cli/src/plan.rs` (command logic).
- New embedded templates: `skills/shared/prompts/plan-greenfield.md`
  and `skills/shared/prompts/plan-amend.md` (initial content can
  be stubs containing only the placeholder syntax; SPEC-0013
  fills in the real prompts).
- The `include_dir!` bundle from SPEC-0002 may move to
  `speccy-core` so both crates can share it.

### Migration / rollback

Greenfield code. Rollback via `git revert`. Depends on
SPEC-0001 (parsers + supersession_index, for amendment form)
and SPEC-0002 (embedded-bundle mechanism).

## Open questions

- [ ] Should the embedded prompt bundle live in `speccy-core` (so
  SPEC-0005+ share it with SPEC-0002's init) or stay in the
  binary crate? Implementer call at first prompt-command landing.
- [ ] When the greenfield template would benefit from a
  `{{next_spec_slug_hint}}` placeholder (an LLM-friendly guess at
  the slug based on VISION.md), should we compute it now or
  leave the agent to invent it? Likely leave it to the agent for
  v1; SPEC-0013 author should ensure the template asks for a
  slug.
- [ ] Should `plan-amend.md` include the `## Changelog` rows
  explicitly (separate placeholder) or rely on `{{spec_md}}`
  containing them inline? Latter is simpler; defer to SPEC-0013.

## Assumptions

- `speccy_core::workspace::find_root` (from SPEC-0004) is
  available for project-root discovery.
- The embedded prompt bundle exists at build time. SPEC-0013
  fills in the real prompt content; initial implementation can
  use stub templates with just the placeholders.
- `BTreeMap<&str, String>` (ordered map) for vars gives
  deterministic iteration; HashMap would also work but ordering
  helps debugging.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from DESIGN.md decomposition. |

## Notes

This spec is also the de-facto landing place for
`speccy_core::prompt` -- the shared infrastructure that
SPEC-0006, SPEC-0008, SPEC-0009, and SPEC-0011 will reuse. When
those specs are deepened, their REQs should reference these
helpers rather than reinventing context loading.

The actual prompt content (what `plan-greenfield.md` and
`plan-amend.md` say) is SPEC-0013's concern. This spec ships only
the rendering mechanism; SPEC-0013 fills in the durable wording.
