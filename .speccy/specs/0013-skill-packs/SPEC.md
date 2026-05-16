---
id: SPEC-0013
slug: skill-packs
title: skill-packs -- markdown content for personas, prompts, and host recipes
status: implemented
created: 2026-05-11
---

# SPEC-0013: skill packs

## Summary

SPEC-0013 ships the **markdown content** that drives speccy's
intelligence layer. The mechanism for embedding, copying, and
loading this content was already established by SPEC-0002
(`init` copy + `include_dir!` bundle) and SPEC-0005 (prompt
template loader). This spec fills the bundle with real content:
8 persona files, 11 prompt templates, and 14 host recipe skills
(7 each for Claude Code and Codex).

No new Rust code. No new CLI surface. The deliverable is the
markdown content itself, packaged in the locations the earlier
specs already expect.

The initial content is "good enough to dogfood speccy on itself"
-- not polished. Iteration via subsequent PRs is expected and
encouraged. The persona content shape and recipe structure are
the **durable contracts** (REQ-005, REQ-006); the specific prose
is iteratable.

## Goals

- Every shipped file exists at the path SPEC-0002 / SPEC-0005 /
  SPEC-0008 / SPEC-0009 / SPEC-0011 expect.
- Personas (planner, implementer, six reviewers) follow a
  consistent content shape so agents can reliably consume them.
- Host recipe skills orchestrate the multi-step loops
  (`speccy:work`, `speccy:review`) correctly per ARCHITECTURE.md's
  workflow phases.
- Each top-level recipe is loadable by its host without parse
  errors.

## Non-goals

- No Rust code. The mechanism is already shipped.
- No polish. Initial content prioritises functional clarity.
- No host-specific GUI integrations (markdown is the surface).
- No content for hosts beyond Claude Code and Codex in v1
  (cursor support deferred per SPEC-0002 DEC-002).
- No marketing copy. Files are tactical, not promotional.

## User stories

- As a developer who ran `speccy init`, I want `/speccy:plan` in
  my host to load and produce the expected agent behaviour
  (call `speccy plan`, read AGENTS.md's product north star,
  propose the first SPEC slice).
- As a reviewer-agent reading the `reviewer-security.md` persona,
  I want clear guidance on what to look for, what's easy to
  miss, and what format to use for the inline note.
- As a future spec author iterating on persona content, I want
  the file structure stable enough that PR diffs are coherent.

## Requirements

<!-- speccy:requirement id="REQ-001" -->
### REQ-001: Shared persona files

`skills/shared/personas/` contains 8 files in pure markdown
(no host-specific frontmatter).

**Done when:**
- The following files exist and are non-empty:
  - `planner.md`
  - `implementer.md`
  - `reviewer-business.md`
  - `reviewer-tests.md`
  - `reviewer-security.md`
  - `reviewer-style.md`
  - `reviewer-architecture.md`
  - `reviewer-docs.md`
- Each file follows the content shape in REQ-005.
- File names match `personas::ALL` (SPEC-0009 DEC-001) for the
  6 reviewer personas; planner and implementer follow the same
  prefix-free naming.

**Behavior:**
- A presence test asserts each file exists and has non-zero
  byte length.
- A shape test asserts each reviewer file contains the required
  sections (role, focus, what to look for that's easy to miss,
  inline note format, example).

<!-- speccy:scenario id="CHK-001" -->
skills/shared/personas/ contains the 8 expected files (planner, implementer, six reviewers) and each is non-empty.
<!-- /speccy:scenario -->
<!-- speccy:scenario id="CHK-002" -->
Reviewer persona file names match personas::ALL exactly (business, tests, security, style, architecture, docs).
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-002" -->
### REQ-002: Shared prompt templates

`skills/shared/prompts/` contains 11 template files with
`{{NAME}}` placeholders for substitution by the prompt-rendering
commands.

**Done when:**
- The following files exist and are non-empty:
  - `plan-greenfield.md`, `plan-amend.md`
  - `tasks-generate.md`, `tasks-amend.md`
  - `implementer.md`
  - `reviewer-business.md`, `reviewer-tests.md`,
    `reviewer-security.md`, `reviewer-style.md`,
    `reviewer-architecture.md`, `reviewer-docs.md`
  - `report.md`
- Each template uses the placeholder names the corresponding
  CLI command substitutes (REQs across SPEC-0005 / 0006 /
  0008 / 0009 / 0011).
- Templates contain at least one of each declared placeholder.
- The placeholder names are valid identifiers
  (`[a-zA-Z_][a-zA-Z0-9_]*`).

**Behavior:**
- For each command's REQ that names placeholders, a smoke test
  loads the template and asserts every named placeholder is
  present at least once.
- A negative test confirms unknown placeholders (typos like
  `{{spec_idd}}`) are NOT present.

<!-- speccy:scenario id="CHK-003" -->
skills/shared/prompts/ contains the 11 expected templates (plan-greenfield/amend, tasks-generate/amend, implementer, six reviewers, report) and each is non-empty.
<!-- /speccy:scenario -->
<!-- speccy:scenario id="CHK-004" -->
Each template contains the placeholders its corresponding CLI command (per REQs in SPEC-0005/0006/0008/0009/0011) substitutes; placeholder names are valid identifiers.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-003" -->
### REQ-003: Claude Code recipe skills

`skills/claude-code/` contains 7 recipe skills with Claude Code
frontmatter.

**Done when:**
- The following files exist with valid Claude Code skill
  frontmatter:
  - `speccy/init.md`
  - `speccy/plan.md`
  - `speccy/tasks.md`
  - `speccy/work.md`
  - `speccy/review.md`
  - `speccy/amend.md`
  - `speccy/ship.md`
- Each file's frontmatter follows Claude Code's convention
  (at minimum: `---\ndescription: ...\n---` opening; YAML
  parseable).
- Each file's body follows the recipe content shape (REQ-006).

**Behavior:**
- A presence test asserts each file exists.
- A frontmatter test parses each file's YAML frontmatter and
  asserts a non-empty `description` field.

<!-- speccy:scenario id="CHK-005" -->
- A presence test asserts each file exists.
- A frontmatter test parses each file's YAML frontmatter and
  asserts a non-empty `description` field.

skills/claude-code/speccy/ contains the 7 expected recipes (init, plan, tasks, work, review, amend, ship) with valid YAML frontmatter and non-empty description fields.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-004" -->
### REQ-004: Codex recipe skills

`skills/codex/` contains 7 recipe skills with Codex frontmatter.

**Done when:**
- The same 7 file names as REQ-003 exist under `skills/codex/`.
- Each file's frontmatter follows Codex conventions (parseable
  YAML with the required fields per Codex docs).
- Each file's body matches its Claude Code counterpart in
  structure, adapted for Codex's invocation idioms where they
  differ.

**Behavior:**
- A presence test asserts each file exists.
- A frontmatter test parses each file's YAML and asserts
  presence of Codex-required fields.

<!-- speccy:scenario id="CHK-006" -->
- A presence test asserts each file exists.
- A frontmatter test parses each file's YAML and asserts
  presence of Codex-required fields.

skills/codex/speccy/ contains the 7 parallel recipes with valid Codex frontmatter.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-005" -->
### REQ-005: Persona content shape

Each reviewer persona file follows a consistent shape.

**Done when:**
- Each `reviewer-<name>.md` file contains, in order:
  1. `# Reviewer Persona: <Capitalised name>`.
  2. `## Role` -- one paragraph naming the persona's adversarial
     stance.
  3. `## Focus` -- a bulleted list of areas the persona
     prioritises.
  4. `## What to look for that's easy to miss` -- bulleted list
     of failure modes specific to this persona.
  5. `## Inline note format` -- specifies the
     `Review (<persona>, pass|blocking): ...` line format.
  6. `## Example` -- one worked example of a `pass` or
     `blocking` note for this persona.
- Planner and implementer personas follow an analogous shape
  adapted to their roles (Role / Focus / What to consider /
  Output format / Example).

**Behavior:**
- A shape-checking test parses each persona file's markdown
  headings and asserts the required sections are present in
  declared order.

<!-- speccy:scenario id="CHK-007" -->
- A shape-checking test parses each persona file's markdown
  headings and asserts the required sections are present in
  declared order.

Each reviewer persona file has the required headings in declared order: # Reviewer Persona, ## Role, ## Focus, ## What to look for that's easy to miss, ## Inline note format, ## Example.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-006" -->
### REQ-006: Recipe content shape

Each top-level recipe skill follows a consistent shape.

**Done when:**
- Each `speccy-<name>.md` recipe file contains:
  1. Frontmatter (host-specific, per REQ-003 / REQ-004).
  2. A short intro paragraph naming what the recipe does.
  3. A "When to use" section.
  4. A numbered step-by-step list of CLI invocations and agent
     actions. CLI commands are wrapped in fenced code blocks.
  5. (For loop recipes -- `speccy:work`, `speccy:review`,
     `speccy:amend`) -- explicit loop conditions and exit
     criteria.
- The CLI commands referenced in the steps match the v1
  CLI surface (the ten commands from ARCHITECTURE.md).

**Behavior:**
- A test loads each recipe and asserts presence of an intro
  paragraph, a "When to use" heading, and at least one
  fenced code block with a `speccy` command.

<!-- speccy:scenario id="CHK-008" -->
- A test loads each recipe and asserts presence of an intro
  paragraph, a "When to use" heading, and at least one
  fenced code block with a `speccy` command.

Each top-level recipe has an intro paragraph, a 'When to use' heading, and at least one fenced code block with a speccy command from the v1 surface.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
<!-- speccy:requirement id="REQ-007" -->
### REQ-007: Files load in their host

The shipped content is loadable by Claude Code and Codex
without errors.

**Done when:**
- For Claude Code: a manual smoke test confirms each recipe
  loads in the host (file appears in the slash-command picker;
  invoking it runs the documented steps without parse errors).
- For Codex: the same smoke test in the Codex host.
- This requirement is a **manual** check; the kind is `manual`
  and the prompt instructs the verifier on the exact steps.

**Behavior:**
- The `manual` check prompt names each recipe and the verifier
  steps (run `speccy init`, then invoke each recipe in turn).
- Pass criterion: every recipe loads and runs the first CLI
  invocation it documents.

<!-- speccy:scenario id="CHK-009" -->
- The `manual` check prompt names each recipe and the verifier
  steps (run `speccy init`, then invoke each recipe in turn).
- Pass criterion: every recipe loads and runs the first CLI
  invocation it documents.

Manually verify each shipped recipe loads in its host: (1) Run speccy init in a fresh repo with .claude/ present. (2) Invoke each of /speccy:init, /speccy:plan, /speccy:tasks, /speccy:work, /speccy:review, /speccy:amend, /speccy:ship in Claude Code; confirm each loads without parse errors and executes its first documented CLI invocation. (3) Repeat with .codex/ and Codex as the host. Pass criterion: every recipe loads and the first step runs.
<!-- /speccy:scenario -->
<!-- /speccy:requirement -->
## Design

### Approach

This spec ships content into the embedded bundle. The
implementer-agent writes 8 + 11 + 7 + 7 = 33 markdown files,
each landing at the path the corresponding command expects.

Initial content is intentionally minimal -- enough to
demonstrate the workflow end-to-end. PRs iterating on content
quality are welcome after v1 ships.

### Decisions

<!-- speccy:decision id="DEC-001" status="accepted" -->
#### DEC-001: Personas in `shared/` only; not duplicated per host

**Status:** Accepted
**Context:** Personas are host-agnostic content. Putting them
under `skills/shared/personas/` avoids the maintenance burden
of keeping two copies aligned.
**Decision:** `skills/shared/personas/reviewer-<name>.md` is
the only location. The CLI loads from here (via the embedded
bundle); project-local overrides go to
`.speccy/skills/personas/` per SPEC-0009 DEC-002.
**Alternatives:**
- One copy per host -- rejected. Duplication.
**Consequences:** SPEC-0009's persona resolver looks only in
shared paths.
<!-- /speccy:decision -->
<!-- speccy:decision id="DEC-002" status="accepted" -->
#### DEC-002: Prompt templates in `shared/prompts/` only

**Status:** Accepted (same reasoning as DEC-001)
**Decision:** All prompt templates live in
`skills/shared/prompts/`. Per-host content is only the recipe
skills.
**Consequences:** Adding a new prompt-rendering command (e.g.
a future `speccy amend-status`) only needs one template file.
<!-- /speccy:decision -->
<!-- speccy:decision id="DEC-003" status="accepted" -->
#### DEC-003: Reviewer persona names mirror `personas::ALL`

**Status:** Accepted (alignment with SPEC-0009 DEC-001)
**Context:** File names must match the registry exactly so
`personas::resolve_file("security", ...)` finds
`reviewer-security.md`.
**Decision:** Reviewer file naming is
`reviewer-<exact-registry-name>.md`. The six names are
`business`, `tests`, `security`, `style`, `architecture`,
`docs`.
**Consequences:** Adding a persona is a coordinated change
across SPEC-0009 (registry) + SPEC-0013 (content).
<!-- /speccy:decision -->
<!-- speccy:decision id="DEC-004" status="accepted" -->
#### DEC-004: Initial content prioritises clarity over polish

**Status:** Accepted
**Context:** Persona prompts and recipe orchestration will
iterate as models change and as projects discover what works.
Spending dozens of hours polishing v1 content delays the
broader bootstrap.
**Decision:** Initial content is "good enough to demonstrate
the workflow." PRs improving wording, examples, or
robustness are explicitly welcome after v1 ships.
**Alternatives:**
- Block v1 on polished content -- rejected. Diminishing
  returns; dogfooding is the better signal.
**Consequences:** Reviewer agents in v1 may produce slightly
generic reviews; iteration improves this over time.
<!-- /speccy:decision -->
<!-- speccy:decision id="DEC-005" status="accepted" -->
#### DEC-005: One reviewer prompt template per persona, not one shared

**Status:** Accepted
**Context:** The `reviewer-<persona>.md` prompt template could
be one shared file with a `{{persona_content}}` placeholder OR
six files (one per persona). Sharing is simpler but limits
per-persona customisation of the template itself (not just
the inlined persona content).
**Decision:** Six separate prompt template files. Each can
diverge if useful; v1 may have near-identical content across
the six, but the door is open.
**Alternatives:**
- One shared `reviewer.md` template -- rejected. Limits future
  per-persona evolution.
**Consequences:** SPEC-0009's template lookup is
`reviewer-<persona>.md` (not `reviewer.md`). Six files to
maintain instead of one.
<!-- /speccy:decision -->
### Data changes

No Rust code. New markdown files only:

- `skills/shared/personas/` -- 8 files.
- `skills/shared/prompts/` -- 11 files.
- `skills/claude-code/` -- 7 files.
- `skills/codex/` -- 7 files.

The embedded bundle from SPEC-0002 (and SPEC-0005's prompt
bundle, if those are merged per SPEC-0005's open question)
picks up the new files automatically via `include_dir!`.

### Migration / rollback

Greenfield content. Rollback via `git revert`. Depends on
SPEC-0002 (copy mechanism), SPEC-0005..0011 (commands that
load these files).

## Open questions

- [ ] Should `planner.md` and `implementer.md` personas have a
  separate template structure from the reviewer six? Probably
  yes; their roles are positive (do the work) vs the
  reviewers' adversarial (find issues). Defer to content
  author.
- [ ] Should `speccy/amend.md` orchestrate both `speccy plan
  SPEC-ID` and `speccy tasks SPEC-ID`, or only one? Per
  ARCHITECTURE.md "Amendment": both. The recipe should reflect this.
- [ ] Should the report prompt template (`report.md`) suggest
  a specific REPORT.md frontmatter shape? Yes -- match
  SPEC-0001 REQ-005 (spec, outcome, generated_at). Document
  in content.

## Assumptions

- SPEC-0002's `include_dir!` bundle is set up to include the
  `skills/` tree at the workspace root.
- SPEC-0009's `personas::ALL` is the source of truth for the
  six reviewer names; this spec mirrors it.
- Each host's frontmatter conventions are stable enough at v1
  time to author against without breaking changes.

## Changelog

<!-- speccy:changelog -->
| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition. |
| 2026-05-14 | agent/claude | Noun swap (Vision → Mission) lands in ARCHITECTURE.md; planner user story re-points at `AGENTS.md` north star (VISION.md is deleted). Persona content unchanged. |
<!-- /speccy:changelog -->

## Notes

This spec closes the bootstrap. With SPEC-0013 deepened, all
13 specs have full SPEC.md / spec.toml / TASKS.md triples and
the implementation can proceed in any order respecting the
dependency graph in PLANNING.md.

The content here is the most iteratable part of speccy. Future
PRs improving persona wording, recipe orchestration, or
prompt clarity are all in-scope -- they don't require a new
spec, just edits to the files this spec ships. The lint
engine's stability registry (SPEC-0003 REQ-007) doesn't apply
to content; it applies to lint codes.
