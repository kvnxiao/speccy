---
id: SPEC-0005
slug: plan-command
title: speccy plan -- render Phase 1 prompts (greenfield + amendment)
status: implemented
created: 2026-05-11
---

# SPEC-0005: speccy plan

## Summary

`speccy plan` is the Phase 1 command. It renders a deterministic
prompt that an agent reads to author or amend a SPEC.md + spec.toml
pair. The CLI never invokes an LLM; it loads context
(AGENTS.md, the nearest parent MISSION.md when amending, the
existing SPEC.md if amending), substitutes placeholders into an
embedded markdown template, and prints the result.

Two forms:

- `speccy plan` (no arg) -- greenfield. Allocates the next available
  `SPEC-NNNN` ID by walking `.speccy/specs/**` (across mission folders
  and flat specs alike), loads `AGENTS.md` (which carries the
  project-wide product north star), and renders `plan-greenfield.md`.
  No `VISION.md` is read: the noun has been retired in favor of
  AGENTS.md.
- `speccy plan SPEC-NNNN` -- amendment. Reads the named SPEC.md,
  walks upward from the spec's directory to find the nearest parent
  `MISSION.md` (if the spec lives under a mission folder), and
  renders `plan-amend.md`. The agent is asked for a minimal surgical
  edit, not a rewrite.

This spec also lands the **shared prompt-rendering infrastructure**
in `speccy_core::prompt` that SPEC-0006 (`tasks`), SPEC-0008
(`implement`), SPEC-0009 (`review`), and SPEC-0011 (`report`)
reuse: template loading, placeholder substitution, AGENTS.md
loading, MISSION.md walking, context-budget trimming, and spec ID
allocation.

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
  me a prompt my agent reads with `AGENTS.md` (carrying the product
  north star) already inlined and a fresh `SPEC-NNNN` ID allocated.
- As a developer amending a spec inside a mission folder, I want
  `speccy plan SPEC-0042` to also inline the nearest parent
  `MISSION.md` so the agent sees the focus-area scope alongside the
  spec being amended.
- As a developer amending a spec mid-loop, I want `speccy plan
  SPEC-0042` to render a "minimal-diff" prompt that respects the
  existing SPEC.md rather than asking for a rewrite.
- As a future SPEC-0006 implementer, I want shared prompt helpers
  in `speccy-core` so my `speccy tasks` command doesn't reimplement
  AGENTS.md loading, MISSION.md walking, template rendering, or
  budget trimming.

## Requirements

<requirement id="REQ-001">
### REQ-001: Greenfield prompt rendering

`speccy plan` with no arg renders the greenfield prompt.

**Done when:**
- The command discovers the project root via
  `speccy_core::workspace::find_root`.
- It allocates the next `SPEC-NNNN` ID via
  `prompt::allocate_next_spec_id` (REQ-003), walking
  `.speccy/specs/**` so flat and mission-grouped specs share one ID
  space.
- It loads AGENTS.md via `prompt::load_agents_md` (REQ-004). Missing
  AGENTS.md is a stderr warning, not an error (greenfield projects
  may not have one yet; the planner agent reading the rendered
  prompt sees a marker indicating conventions are not loaded).
- It loads the embedded `plan-greenfield.md` template via
  `prompt::load_template` and substitutes placeholders:
  `{{agents}}`, `{{next_spec_id}}`. There is no `{{vision}}`
  placeholder.
- It trims the rendered output to the budget via
  `prompt::trim_to_budget` (REQ-006).
- It writes the final rendered prompt to stdout; exits 0.

**Behavior:**
- Given a fresh `.speccy/` with no specs yet, when `speccy plan`
  runs, then `{{next_spec_id}}` substitutes to `SPEC-0001` and
  AGENTS.md content (or the missing-marker) appears at the
  `{{agents}}` site.
- Given specs SPEC-0001 through SPEC-0013 exist (mix of flat and
  mission-grouped), when `speccy plan` runs, then `{{next_spec_id}}`
  substitutes to `SPEC-0014`.
- Given specs SPEC-0001 and SPEC-0003 exist (gap at 0002), then
  `{{next_spec_id}}` substitutes to `SPEC-0004` (no gap recycling,
  per DEC-005).
- Given `speccy plan` runs outside a `.speccy/` workspace, then
  exit code is 1 with a stderr message stating `.speccy/` was not
  found walking up from cwd.

<scenario id="CHK-001">
speccy plan (no arg) renders plan-greenfield.md with agents and next_spec_id placeholders substituted; no vision placeholder exists; output goes to stdout.
</scenario>

<scenario id="CHK-002">
speccy plan run outside a .speccy/ workspace exits 1 with a stderr message stating the workspace was not found walking up from cwd.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Amendment prompt rendering

`speccy plan SPEC-NNNN` renders the amendment prompt.

**Done when:**
- The command parses the SPEC-ID argument. If it doesn't match
  `SPEC-\d{4,}`, exit code 1 with a format-error message.
- It discovers the spec directory matching the ID. The directory
  may live directly under `.speccy/specs/NNNN-slug/` (flat) or
  under a mission folder at `.speccy/specs/[focus]/NNNN-slug/`. If
  no matching directory exists, exit code 1 with a "spec not found"
  message naming the ID.
- It reads the matching SPEC.md via the SPEC-0001 parser. Parse
  errors return exit code 1 with the parser's error message.
- It walks upward from the spec directory looking for the nearest
  `MISSION.md` via `prompt::find_nearest_mission_md` (REQ-007). If
  one exists at `.speccy/specs/[focus]/MISSION.md` (parent of the
  matched spec dir), its content is loaded. If absent, the
  `{{mission}}` placeholder substitutes to a marker indicating the
  spec is ungrouped.
- It loads AGENTS.md (REQ-004).
- It loads the embedded `plan-amend.md` template and substitutes
  placeholders: `{{spec_id}}`, `{{spec_md}}` (full content),
  `{{agents}}`, `{{mission}}` (parent MISSION.md content or
  ungrouped marker), `{{changelog}}` (the existing Changelog
  table rows for context).
- It trims the rendered output to the budget.
- It writes the final rendered prompt to stdout; exits 0.

**Behavior:**
- Given `speccy plan SPEC-0001` and
  `.speccy/specs/0001-artifact-parsers/` exists (flat), when the
  command runs, then the rendered output contains the full SPEC.md
  content, the amendment-mode template language, and a marker at
  `{{mission}}` stating the spec is ungrouped.
- Given `speccy plan SPEC-0042` and
  `.speccy/specs/auth/0042-signup/SPEC.md` exists alongside
  `.speccy/specs/auth/MISSION.md`, then the rendered output
  inlines the MISSION.md content at `{{mission}}`.
- Given `speccy plan SPEC-9999` and no such spec exists, then exit
  code is 1 and stderr names SPEC-9999.
- Given `speccy plan FOO`, then exit code is 1 with a format
  error naming the invalid argument.

<scenario id="CHK-003">
- Given `speccy plan SPEC-0001` and
  `.speccy/specs/0001-artifact-parsers/` exists (flat), when the
  command runs, then the rendered output contains the full SPEC.md
  content, the amendment-mode template language, and a marker at
  `{{mission}}` stating the spec is ungrouped.
- Given `speccy plan SPEC-0042` and
  `.speccy/specs/auth/0042-signup/SPEC.md` exists alongside
  `.speccy/specs/auth/MISSION.md`, then the rendered output
  inlines the MISSION.md content at `{{mission}}`.
- Given `speccy plan SPEC-9999` and no such spec exists, then exit
  code is 1 and stderr names SPEC-9999.
- Given `speccy plan FOO`, then exit code is 1 with a format
  error naming the invalid argument.

speccy plan SPEC-NNNN renders plan-amend.md with the named SPEC.md inlined; the nearest parent MISSION.md is substituted at {{mission}} when present (or an ungrouped marker when absent); invalid ID format exits 1; missing spec exits 1 naming the ID; mission-grouped specs (e.g. .speccy/specs/auth/0042-signup/) resolve correctly.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Spec ID allocation

Allocate the next available `SPEC-NNNN` ID by walking `specs/`
recursively so flat and mission-grouped specs share one ID space.

**Done when:**
- `prompt::allocate_next_spec_id(specs_dir: &Utf8Path) -> String`
  walks `specs_dir` recursively, finds every directory whose name
  matches `^(\d{4,})-`, parses the numeric prefix, and returns
  `max(prefixes) + 1` zero-padded to 4 digits (e.g. `"0014"`).
- Mission folders (subdirectories of `specs_dir` whose names do not
  match the `NNNN-slug` pattern, e.g. `auth/`) are descended into
  but contribute no prefix themselves.
- If `specs_dir` is empty or absent, returns `"0001"`.
- Non-matching directories that aren't mission folders (e.g.
  `_scratch`, `00ab-foo`) are silently ignored.
- The function does directory traversal only; no spec.toml or
  SPEC.md content is read.

**Behavior:**
- Given an empty `specs/`, the allocator returns `"0001"`.
- Given flat `0001-foo` and `0003-bar`, the allocator returns
  `"0004"` (no gap recycling).
- Given `auth/0001-signup` and `billing/0002-invoice` (mission
  folders), the allocator returns `"0003"`.
- Given a mix: flat `0001-foo`, `auth/0002-signup`,
  `billing/0010-invoice`, the allocator returns `"0011"`.
- Given a non-matching directory `_scratch` alongside `0001-foo`,
  the allocator returns `"0002"`.

<scenario id="CHK-004">
- Given an empty `specs/`, the allocator returns `"0001"`.
- Given flat `0001-foo` and `0003-bar`, the allocator returns
  `"0004"` (no gap recycling).
- Given `auth/0001-signup` and `billing/0002-invoice` (mission
  folders), the allocator returns `"0003"`.
- Given a mix: flat `0001-foo`, `auth/0002-signup`,
  `billing/0010-invoice`, the allocator returns `"0011"`.
- Given a non-matching directory `_scratch` alongside `0001-foo`,
  the allocator returns `"0002"`.

allocate_next_spec_id walks specs/** recursively across mission folders and flat specs alike; returns max(existing) + 1 zero-padded to 4 digits; empty specs/ yields 0001; gaps left by dropped specs are not recycled; non-matching directories that are not mission folders are ignored.
</scenario>

</requirement>

<requirement id="REQ-004">
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

<scenario id="CHK-005">
- Given AGENTS.md exists with content `# Agents\n<rest>`, the
  function returns that content verbatim.
- Given AGENTS.md is missing, the function returns the marker and
  stderr contains a warning naming the expected path.
- Given AGENTS.md exists but is unreadable (permission denied),
  the function returns the error-marker and stderr warns.

load_agents_md returns file content when present; returns marker + stderr warning when missing; returns error-marker + stderr warning on I/O error.
</scenario>

</requirement>

<requirement id="REQ-005">
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

<scenario id="CHK-006">
render substitutes every {{name}} placeholder; single-pass means substituted text is not re-scanned for further placeholders.
</scenario>

<scenario id="CHK-007">
render leaves unrecognised {{placeholders}} in place and prints a stderr warning naming each unique unmatched placeholder.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Context-budget trimming (cross-cutting helper)

Drop low-priority sections from the rendered prompt when it
exceeds a budget threshold.

**Done when:**
- `prompt::trim_to_budget(rendered: String, budget: usize) -> TrimResult`
  returns `{ output: String, dropped: Vec<String>, fits: bool }`.
- Drop ordering matches ARCHITECTURE.md "Prompt context budget":
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

<scenario id="CHK-008">
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

trim_to_budget drops sections in ARCHITECTURE.md order until under budget; sets fits=false and warns on stderr when output still exceeds budget after all drops.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Nearest-parent MISSION.md walking (cross-cutting helper)

Walk upward from a spec directory looking for the nearest parent
`MISSION.md`, returning either its content or an ungrouped marker.

**Done when:**
- `prompt::find_nearest_mission_md(spec_dir: &Utf8Path, specs_root: &Utf8Path) -> String`:
  - Walks from `spec_dir`'s parent upward (toward `specs_root`),
    stopping at `specs_root` inclusive.
  - At each level, checks for a `MISSION.md` file. The first one
    found wins.
  - Returns the file content on hit.
  - Returns a literal marker
    `<!-- no parent MISSION.md; spec is ungrouped -->` if no
    MISSION.md is found before reaching `specs_root`.
  - On I/O error reading a found MISSION.md, returns a marker
    `<!-- MISSION.md unreadable at <path>: <err> -->` and prints a
    one-line stderr warning.
- The function does not search outside the `specs_root` subtree.
- The function is consumed by SPEC-0005 (amendment path), SPEC-0008
  (implementer), SPEC-0009 (reviewer), and SPEC-0011 (report).

**Behavior:**
- Given `specs_root = .speccy/specs/` and
  `spec_dir = .speccy/specs/0001-foo/` with no MISSION.md anywhere,
  the function returns the ungrouped marker.
- Given `specs_root = .speccy/specs/` and
  `spec_dir = .speccy/specs/auth/0042-signup/` with
  `.speccy/specs/auth/MISSION.md` present, the function returns
  that file's content verbatim.
- Given a malformed unreadable `MISSION.md`, the function returns
  the unreadable marker and stderr is non-empty.

<scenario id="CHK-009">
- Given `specs_root = .speccy/specs/` and
  `spec_dir = .speccy/specs/0001-foo/` with no MISSION.md anywhere,
  the function returns the ungrouped marker.
- Given `specs_root = .speccy/specs/` and
  `spec_dir = .speccy/specs/auth/0042-signup/` with
  `.speccy/specs/auth/MISSION.md` present, the function returns
  that file's content verbatim.
- Given a malformed unreadable `MISSION.md`, the function returns
  the unreadable marker and stderr is non-empty.

find_nearest_mission_md returns parent MISSION.md content when present in the spec's mission folder; returns an ungrouped marker when no MISSION.md is found up to specs_root; returns an unreadable marker plus stderr warning on I/O error.
</scenario>

</requirement>

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
4. Greenfield: allocate the next spec ID. Amendment: locate the
   spec dir (anywhere under `specs/` including mission folders),
   parse the named SPEC.md, and walk for the nearest parent
   MISSION.md.
5. Load the relevant template.
6. Substitute placeholders.
7. Trim to budget.
8. Write rendered output to stdout.

### Decisions

<decision id="DEC-001" status="accepted">
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
</decision>

<decision id="DEC-002" status="accepted">
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
</decision>

<decision id="DEC-003" status="accepted">
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
</decision>

<decision id="DEC-004" status="accepted">
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
</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: Spec ID allocation is "max + 1"; no gap recycling

**Status:** Accepted (per ARCHITECTURE.md "Spec ID allocation")
**Context:** Gaps left by dropped specs could be recycled. But
recycling means a dropped SPEC-0007 could later mean something
completely different, confusing anyone reading historical
commits or PR descriptions referencing the old ID.
**Decision:** Always allocate `max(existing) + 1`. Dropped specs
leave permanent gaps.
**Alternatives:**
- Recycle gaps -- rejected per historical-ambiguity reasoning.
**Consequences:** Spec ID space grows monotonically. Acceptable.
</decision>

### Interfaces

```rust
// speccy-core additions
pub mod prompt {
    pub fn load_template(name: &str) -> Result<&'static str, PromptError>;
    pub fn render(
        template: &str,
        vars: &BTreeMap<&str, String>,
    ) -> String;
    pub fn load_agents_md(project_root: &Utf8Path) -> String;
    pub fn find_nearest_mission_md(spec_dir: &Utf8Path, specs_root: &Utf8Path) -> String;
    pub fn trim_to_budget(rendered: String, budget: usize) -> TrimResult;
    pub fn allocate_next_spec_id(specs_dir: &Utf8Path) -> String;
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
    ProjectRootNotFound,
    Prompt(PromptError),
    Parse(ParseError),
}
```

Note that `PlanError::VisionMissing` and the corresponding
`VisionIo` variant from the original v1 draft have been removed:
the greenfield path no longer reads any `.speccy/VISION.md`. The
product north star now lives in `AGENTS.md` and is loaded via
`prompt::load_agents_md`, which treats absence as a warning rather
than an error (per DEC-003).

### Data changes

- New `speccy-core/src/prompt/mod.rs` and submodules
  (`template`, `render`, `agents_md`, `mission_md`, `budget`,
  `id_alloc`). The `mission_md` submodule owns
  `find_nearest_mission_md` (REQ-007).
- New `speccy-cli/src/plan.rs` (command logic).
- New embedded templates: `skills/shared/prompts/plan-greenfield.md`
  (placeholders: `{{agents}}`, `{{next_spec_id}}`) and
  `skills/shared/prompts/plan-amend.md` (placeholders: `{{spec_id}}`,
  `{{spec_md}}`, `{{agents}}`, `{{mission}}`, `{{changelog}}`).
  Initial content can be stubs containing only the placeholder
  syntax; SPEC-0013 fills in the real prompts.
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
- [ ] Should `plan-amend.md` include the `## Changelog` rows
  explicitly (separate placeholder) or rely on `{{spec_md}}`
  containing them inline? Latter is simpler; defer to SPEC-0013.
- [ ] Should `speccy plan` (no arg, greenfield) ever attempt to
  scope to a focus area, e.g. via cwd inspection or a flag, or
  should the planner agent always decide placement (flat vs
  mission folder)? v1 ships the latter; revisit if friction emerges.

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

<changelog>
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
| 2026-05-14 | agent/claude | Vision noun retired. Greenfield path no longer reads `.speccy/VISION.md`; product north star now lives in `AGENTS.md`. Amendment path walks upward from the spec dir for the nearest parent `MISSION.md` (introduced REQ-007 + `prompt::find_nearest_mission_md`). REQ-003 spec-ID allocation now walks `specs/**` recursively so flat and mission-grouped specs share one ID space. `plan-greenfield.md` template loses the `{{vision}}` placeholder; `plan-amend.md` gains `{{mission}}`. `PlanError::VisionMissing` / `VisionIo` removed; missing AGENTS.md is a warning, not an error. |
</changelog>

## Notes

This spec is also the de-facto landing place for
`speccy_core::prompt` -- the shared infrastructure that
SPEC-0006, SPEC-0008, SPEC-0009, and SPEC-0011 will reuse. When
those specs are deepened, their REQs should reference these
helpers rather than reinventing context loading.

The actual prompt content (what `plan-greenfield.md` and
`plan-amend.md` say) is SPEC-0013's concern. This spec ships only
the rendering mechanism; SPEC-0013 fills in the durable wording.
