# Speccy File Formats and Lints

> The file-format and lint contract: the on-disk layout, every artifact
> template and its element grammar, the TASKS.md state model, the
> per-task and per-SPEC journals, staleness detection, and the full lint
> code registry.
>
> Part of the Speccy docs set: [ARCHITECTURE](./ARCHITECTURE.md) (design
> rationale) · [CLI](./CLI.md) (commands) · SCHEMA (file formats + lints,
> this file) · [WORKFLOW](./WORKFLOW.md) (loop + harness).

---

## File layout

```text
AGENTS.md                Project-wide product north star + agent conventions
                         (root, not inside .speccy/)

.speccy/
  MEMORY.md              User-owned, git-tracked loop-memory ledger. The
                         working tier of per-repo memory; sibling of
                         BACKLOG.md. `speccy init` never enumerates,
                         creates, or overwrites it (not in the eject
                         pipeline's file set), and `speccy verify` never
                         reads it. Absent until the loop first writes it.
  specs/
    0001-user-signup/                Ungrouped spec (no mission folder)
      SPEC.md            Frontmatter + PRD prose + nested XML element tree
                         (<requirement>/<scenario>/<decision>/<open-question>
                         /<changelog>); the requirement-to-scenario graph is
                         carried in-band by these elements
      TASKS.md           Frontmatter (spec_hash_at_generation, generated_at)
                         + bare <task>/<task-scenarios> XML tree (no
                         <tasks> wrapper; no implementer / review prose)
      REPORT.md          Frontmatter (outcome) + <report>/<coverage> XML tree
                         (end of loop)
      journal/           Per-task activity journal (see "TASKS.md
        T-001.md         per-task journal" below). One T-NNN.md per task
        T-002.md         that has been claimed by an implementer; each
        T-003.md         carries <implementer>/<review>/<blockers> blocks.
    auth/                            Mission folder (optional grouping)
      MISSION.md                     Scope/context for this focus area
      0002-signup/
        SPEC.md
        TASKS.md
        REPORT.md
      0003-password-reset/
        SPEC.md
        ...

resources/               Shipped with Speccy; rendered/copied by `speccy init`
  modules/               Single-source bodies (no host duplication)
    personas/            Reviewer, vet, and plan persona bodies,
                         plus co-located snippets included from
                         those bodies.
    phases/              Agent bodies for the pinned phase workers
                         and the init phase.
    skills/              Interactive skill bodies plus the SKILL.md
                         bodies for the pinned phase workers
                         (which defer to the matching agent file).
                         `partials/` holds sharable skill fragments
                         included from multiple skill bodies.
    references/          Canonical reference files shared across
                         skills. Skill-local refs eject into each
                         skill's `references/` subdirectory;
                         host-shared refs eject under
                         `<host>/speccy-references/`.
  agents/                Per-host wrappers (MiniJinja-templated)
    .claude/             Renders to <project>/.claude/{skills,agents}/
    .agents/             Renders to <project>/.agents/skills/ (Codex)
    .codex/              Renders to <project>/.codex/agents/ (Codex)
```

There is no `resources/modules/prompts/` directory and no CLI-embedded
phase prompt body. Phase prose ships as host skill content; the CLI does
not render natural text. Reviewer persona content lives at the
host-native sub-agent files (`.claude/agents/reviewer-<persona>.md` and
the Codex twin) and there is no project-local
`.speccy/skills/personas/` override.

`AGENTS.md` lives at project root, not inside `.speccy/`. Every project
already keeps `AGENTS.md` (and often `CLAUDE.md` as a symlink) at the
root for the broader agent ecosystem; Speccy reads the file in place
rather than asking projects to duplicate it under `.speccy/`. AGENTS.md
carries both the product north star (what we're building, who for, v1
outcome, quality bar) and the cross-cutting agent conventions (hygiene,
rule files, behavioral expectations). Section the file explicitly so
reviewer-business and reviewer-architecture personas can find the
product context, while reviewer-style finds the conventions.

Mission folders are optional. A flat project with one focus area may
have zero MISSION.md files: specs live directly under
`.speccy/specs/NNNN-slug/`. When grouping emerges, the planner skill
creates `.speccy/specs/[focus]/MISSION.md` and writes new specs into the
focus folder. Existing flat specs may be moved into a focus folder
retroactively; spec IDs do not change.

`resources/modules/{personas,phases,skills,references}/` are the single
source of truth, and `resources/agents/` carries the per-host wrappers
as MiniJinja templates. `speccy init` renders those wrappers into the
user's project at the host-native location. The full per-host file map
lives in [WORKFLOW.md → What ships](./WORKFLOW.md#what-ships).

There is no project-local persona override directory. The host-native
sub-agent files under `.claude/agents/` and `.codex/agents/` are the
sole canonical persona surface. They participate in the same uniform
Create / Unchanged / Conflict classification as every other file
`speccy init` writes; under `--force` a differing file is overwritten
with the shipped bundle content. Users who customise a persona body
preserve their edits via git (commit before running `--force`, restore
from history afterwards).

Decisions (ADRs) are not a separate folder. Each spec's
`## Design > Decisions` subsection holds the architectural choices made
for that spec. Project-wide conventions that span specs belong in
`AGENTS.md`. Cross-spec context bounded to one focus area belongs in
that focus area's `MISSION.md`.

---

## AGENTS.md bootstrap

The project-wide product north star ("what we're building, why, who for,
what 'good enough to ship v1' looks like") is **not** a Speccy noun. It
lives as a section inside `AGENTS.md` at the repo root. AGENTS.md is
loaded into every rendered prompt, so the north star is always in
context for any planner, implementer, or reviewer agent.

When `AGENTS.md` is missing or lacks a product north star section, the
**`speccy-bootstrap` skill** (not the CLI) runs an interactive Q&A to
populate it. The skill detects three states:

1. AGENTS.md missing entirely → bootstrap from scratch via full Q&A
   (product, users, v1 outcome, constraints, non-goals, quality bar,
   known unknowns).
2. AGENTS.md exists with process conventions but no `## Product north
   star` section (or equivalent) → narrower Q&A; append the section.
3. AGENTS.md already has a north star → leave alone; confirm with the
   user.

The skill never overwrites: always append, or stop. The CLI's
`speccy init` only scaffolds `.speccy/` and copies the host skill pack;
it never edits `AGENTS.md`.

---

## MISSION.md

Optional parent-context artifact for a focus area. Not required: a flat
single-focus project may have zero MISSION.md files. When present, it
lives at `.speccy/specs/[focus]/MISSION.md` and the planner / implementer
/ reviewer skills walk upward from any spec path looking for the nearest
MISSION.md and include it in rendered prompts.

The project-wide product north star does **not** live here; it lives in
`AGENTS.md` at the repo root. MISSION.md is narrower: the scope of one
focus area within the broader product.

Recommended sections:

```markdown
# Mission: <focus name>

## Scope
What this focus area covers. What it doesn't.

## Why now
The motivation driving this initiative, and any deadline / sequencing
constraints.

## Specs in scope
- SPEC-NNN: short title
- SPEC-NNN: short title

## Cross-spec invariants
Things every spec in this mission must respect (auth model, data
ownership, error semantics, etc.).

## Open questions
Things we expect to learn as specs land.
```

MISSION.md is markdown; Speccy does not parse its structure beyond
detecting its presence to scope prompts. No `MIS-NNN` lint codes exist
in v1. No `speccy mission` command exists. Mission is a
filesystem-and-skill convention, not a CLI-aware noun. (This is a
deliberate v1 simplification; promote to a parsed noun later only if
dogfooding shows pain.)

---

## SPEC.md

PRD-shaped template:

```markdown
---
id: SPEC-001
slug: user-signup
title: User signup
status: in-progress
created: 2026-05-11
supersedes: []
---

# SPEC-001: User signup

## Summary
2-4 paragraphs. What this spec covers, why it matters, how it fits
into the broader product.

## Goals

<goals>
- Concrete outcomes this spec must achieve.
</goals>

## Non-goals

<non-goals>
- Explicitly out of scope. Things readers might assume but shouldn't.
</non-goals>

## User stories

<user-stories>
- As a new visitor, I want to create an account with email/password
  so that I can save my work between sessions.
- As a returning user, I want a clear error when I try to sign up
  with an email that already exists.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Account creation

Users can create an account with email and password.

<done-when>
- A valid signup request persists a user record and returns a
  session token.
- A duplicate email returns 409 with an actionable message.
</done-when>

<behavior>
- Given no account exists for `alice@example.com`, when a signup
  request submits valid credentials, then a user record is
  persisted and the response includes a session token.
- Given an account already exists for `alice@example.com`, when a
  signup request submits the same email, then the response is 409
  with an error message containing "already exists".
- Given a signup request submits an invalid email format, when
  processed, then the response is 400 with a validation error.
</behavior>

<scenario id="CHK-001">
Given no account exists for alice@example.com,
when the signup endpoint receives a valid request,
then a user row is persisted and the response includes a session
token.
</scenario>

<scenario id="CHK-002">
Given an account already exists for alice@example.com,
when a signup request submits the same email,
then the response is 409 with an error containing "already exists".
</scenario>

<scenario id="CHK-003">
Given a signup request with a malformed email,
when the handler runs,
then the response is 400 with a validation error.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: Password storage

Passwords are hashed before persistence; plaintext never touches
storage.

<done-when>
- Inspection of the users table shows hashed values; a direct DB
  query for the password column never returns plaintext.
</done-when>

<behavior>
- Given a signup request with password `correct horse battery
  staple`, when the user record is persisted, then the password
  column contains a hash and never the original string.
- Given the users table is dumped to logs, when inspected, then
  no plaintext passwords appear.
</behavior>

<scenario id="CHK-004">
Given a signup request with password `correct horse battery staple`,
when the user record is persisted,
then the password column contains a hash and never the original
string.
</scenario>
</requirement>

## Design

### Approach
[1-2 paragraphs of technical approach.]

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Password hashing algorithm
**Context:** Signup requires password auth without hosted services.
**Decision:** bcrypt with cost factor 12.
**Alternatives:** Hosted auth (deferred, requires email
infrastructure); argon2 (deferred, no clear need yet).
**Consequences:** App owns credential storage risk. Security
review must inspect password handling on every auth-touching
change.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Session storage
**Context:** Signup must return something a returning user can
present to authenticate later requests.
**Decision:** JWT signed with project secret, 24h expiry, stored
in httpOnly Secure cookie.
**Alternatives:** Server-side sessions in Redis (rejected: adds
infrastructure dependency); long-lived API tokens (rejected:
revocation story is poor).
**Consequences:** Stateless auth; horizontal scaling is trivial.
Token revocation requires key rotation or a blocklist (deferred).
</decision>

### Interfaces
- `POST /api/signup` -- accepts `{email, password}`, returns
  `{token}` or `{error}`.
- `users` table -- new columns: `email` (unique index),
  `password_hash`.

### Data changes
- Migration: `users` table with unique email index.

### Migration / rollback
- Forward: standard migration.
- Rollback: drop columns; no data loss because feature is new.

## Open questions
- [ ] Should signup auto-login the user, or just create the account?
- [x] Email case-sensitivity? -- Normalize to lowercase on write.

## Assumptions

<assumptions>
- Email uniqueness enforced at the DB layer via index.
</assumptions>

## Changelog

<changelog>
| Date       | Author          | Summary |
|------------|-----------------|---------|
| 2026-05-11 | agent/claude-1  | Initial draft from AGENTS.md north star |
</changelog>

## Notes
Free-form context for future agents and reviewers.
```

### Frontmatter

The YAML frontmatter is the single source of truth for spec lifecycle:

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Stable spec ID (`SPEC-NNN`). |
| `slug` | yes | Folder-name slug. |
| `title` | yes | Human-readable title. |
| `status` | yes | One of: `in-progress`, `implemented`, `dropped`, `superseded`. |
| `created` | yes | ISO date when the spec was first drafted. |
| `supersedes` | no | List of prior spec IDs this one replaces. Omit or `[]` if none. |

Supersession is stored on the **new** spec (the one doing the replacing)
via `supersedes`. The inverse direction is **computed** by walking the
supersedes graph across all specs in the workspace; no `superseded_by`
field is stored. This keeps lineage single-sourced; the older spec does
not need to be updated when a new spec replaces it.

`status` transitions:

```text
in-progress -> implemented      All tasks state="completed", REPORT.md written, PR merged.
in-progress -> dropped          Intent abandoned. Add a Changelog row stating why.
implemented -> superseded       A later spec declared `supersedes` pointing here.
in-progress -> superseded       Rare; replaced before completion.
```

Skills (specifically `/speccy-ship` and `/speccy-amend`) update
`status`. The CLI doesn't auto-transition state; it surfaces
inconsistencies via lint (e.g. `status: implemented` but some tasks have
`state != "completed"`).

### Changelog table

The `## Changelog` table is the in-doc lineage. Every material change to
SPEC.md after initial draft adds a row:

| Date | Author | Summary |
|------|--------|---------|
| 2026-05-11 | agent/claude-1 | Initial draft |
| 2026-05-13 | agent/claude-1 | REQ-002 bcrypt cost bumped to 12 per security review F-001 |
| 2026-05-14 | human/kevin | Dropped REQ-003 (magic-link auth), out of v1 scope |

The Changelog is git-history-redundant by design: git tells you *what*
changed; the Changelog summarizes *why* and is loaded into every prompt
that reads SPEC.md. Reviewer personas read it to understand recent
intent shifts. The `/speccy-amend` skill appends a row whenever it edits
SPEC.md.

### Element grammar

The machine-readable structure inside `SPEC.md` is carried by
line-isolated **raw XML element tags** wrapping ordinary Markdown. The
Markdown body remains valid Markdown: `<T>` / `A & B` style content
inside a scenario does not need XML escaping, fenced code blocks pass
through verbatim, and the parser is line-aware rather than being a full
XML document parser.

Every Speccy element open tag and close tag occupies its own line.
Opening tags may carry double-quoted attributes; closing tags carry only
the element name with a leading slash.

```markdown
<requirement id="REQ-001">
### REQ-001: Render selected scenarios

Plain Markdown prose remains plain Markdown.

<done-when>
- Implementer-visible acceptance criteria as a bullet list.
</done-when>

<behavior>
- Given/When/Then prose that drives test selection.
</behavior>

<scenario id="CHK-001">
Given a task covers REQ-001,
when `speccy check SPEC-NNNN/T-NNN` runs,
then only REQ-001's scenarios are rendered.
</scenario>
</requirement>
```

Top-level intent sections are wrapped the same way:

```markdown
<goals>
- Concrete outcomes this spec must achieve.
</goals>

<non-goals>
- Explicitly out of scope.
</non-goals>

<user-stories>
- As a <role>, I want <capability> so that <benefit>.
</user-stories>

<assumptions>
- Optional. Preconditions the spec relies on; omit entirely if none.
</assumptions>
```

A Speccy element tag sharing a line with non-whitespace prose is a parse
error. Attribute values without surrounding double quotes are a parse
error. Unknown attributes on a known Speccy element are a parse error.
Element-shaped text outside the whitelist on its own line is treated as
Markdown body content (no parse error, no structural element).

| Element | Cardinality | Location | Attributes |
|---|---|---|---|
| `goals` | required, single | top-level | none |
| `non-goals` | required, single | top-level | none |
| `user-stories` | required, single | top-level | none |
| `assumptions` | optional, 0 or 1 | top-level | none |
| `requirement` | required, 1+ | top-level | `id="REQ-NNN"` |
| `done-when` | required, single | inside `<requirement>`, before `<behavior>` | none |
| `behavior` | required, single | inside `<requirement>`, after `<done-when>` and before `<scenario>` | none |
| `scenario` | required, 1+ inside each requirement | inside `<requirement>`, after `<behavior>` | `id="CHK-NNN"` |
| `decision` | optional, 0+ | top-level | `id="DEC-NNN"`, optional `status="accepted\|rejected\|deferred\|superseded"` |
| `open-question` | optional, 0+ | top-level | optional `resolved="true\|false"` |
| `changelog` | required, single | top-level | none |

Open-tag forms in canonical order:

```markdown
<goals>
<non-goals>
<user-stories>
<requirement id="REQ-001">
<done-when>
<behavior>
<scenario id="CHK-001">
<decision id="DEC-001" status="accepted">
<open-question resolved="false">
<assumptions>
<changelog>
```

The Speccy element whitelist is **disjoint from the HTML5 element name
set** by construction: a `<section>` or `<details>` line in a SPEC.md
body is unambiguously prose, never Speccy structure. The disjointness
invariant is enforced by a unit test against a checked-in copy of the
WHATWG element index. New structural additions must avoid HTML5 element
names; that test catches accidental collisions at build time.

IDs and nesting:

- Requirement ids match `REQ-\d{3,}`.
- Scenario ids match `CHK-\d{3,}`.
- Decision ids match `DEC-\d{3,}`.
- A `<scenario>` element must be nested inside exactly one
  `<requirement>` element; the parent requirement is recorded as
  `scenario.parent_requirement_id`.
- Duplicate `REQ-`, `CHK-` (within one spec), or `DEC-` ids are parse
  errors.
- The body of each required element (`requirement`, `scenario`,
  `changelog`) must contain non-whitespace Markdown.
- Element-shaped lines hidden inside fenced code blocks or inline
  backticks are treated as code content, not structure. SPEC.md files
  that document Speccy's own grammar put example tags inside fenced code
  blocks so the scanner does not promote them.

The canonical on-disk form is deterministic: element tags are
line-isolated; element
attributes appear in a stable order; requirement and scenario order
follows document order; Markdown bodies are preserved verbatim except
for trailing whitespace normalization at element boundaries. The
canonical form is a grammar contract enforced by the parser, not a
formatter. There is no public `speccy fmt` command.

### Lint behavior

Speccy lints three things in SPEC.md:

1. Required frontmatter fields are present.
2. The element tree is well-formed: every `<requirement>` has at least
   one nested `<scenario>`, every id matches its regex, and no ids
   duplicate within a spec.
3. Any unchecked `- [ ]` in the **Open questions** section is reported
   in `speccy status` as a soft signal.

Nothing else in SPEC.md is parsed or enforced. The template is a
convention; the agent's skill prompts nudge the shape.

### Tests in English first (TDD convention)

The `<behavior>` block under each requirement is the **higher-level test
specification** in prose. Each bullet is one Given/When/Then scenario
that maps to one or more Checks. These describe integration or
end-to-end behavior at the requirement level.

Unit-level tests live in TASKS.md as `<task-scenarios>` element blocks
nested inside each `<task>`. This split is intentional:

- **SPEC.md behavior**: what the system does, observable from outside.
  Maps to `<scenario>` element blocks nested under each requirement; the
  project's integration tests must satisfy them.
- **TASKS.md `<task-scenarios>`**: what each implementation slice must
  verify. Maps to unit tests the implementer writes before code.

Agents writing implementation code translate these prose tests into
executable tests in the project's framework, then implement to make them
pass. Speccy does not run those tests and does not enforce TDD ordering
(red-before-green); it makes the test obligations visible and the
reviewer-tests persona checks that they're meaningful.

### Brownfield posture

There is no greenfield/brownfield mode toggle, no `origin` field, and no
per-requirement delta markers. Brownfield-aware spec authoring is the
planner skill's job:

- The planner persona detects existing code, lockfiles, and conventions
  in the repo.
- It reads enough context to write SPEC.md prose that accurately
  reflects "this behavior already exists" vs "this is new."
- When a new spec changes a previously-shipped spec, the new spec's
  frontmatter sets `supersedes: [SPEC-NNN]` and the prose explicitly
  references which prior behavior is being changed.

The combination of `frontmatter.status`, `frontmatter.supersedes`, and
the `## Changelog` table is sufficient to track spec evolution without
per-requirement annotations.

---

## TASKS.md

`TASKS.md` is Markdown with structure carried by raw XML element tags.
Frontmatter records the generating spec hash; the body holds each task
as a bare `<task>` element directly under the `# Tasks: SPEC-NNNN ...`
heading (no wrapper element). The spec binding resolves from the
frontmatter `spec:` field plus the parent directory name; there is no
redundant `spec="..."` attribute on the body root.

```markdown
---
spec: SPEC-001
spec_hash_at_generation: sha256:abc...123
generated_at: 2026-05-11T18:00:00Z
---

# Tasks: SPEC-001 User signup

## Phase 1: Schema

<task id="T-001" state="pending" covers="REQ-001">
## T-001: Add `users` table migration with unique email index

<task-scenarios>
Given a fresh database,
when the migration runs forward,
then the `users` table exists with a unique index on `email`.

Given an existing row with email `alice@example.com`,
when a second insert uses the same email,
then the insert fails with a uniqueness violation.
</task-scenarios>

- Suggested files: `migrations/`, `db/schema/users.ts`
</task>

<task id="T-002" state="pending" covers="REQ-002">
## T-002: Add `password_hash` column to `users`

<task-scenarios>
Given a row inserted with a non-empty `password_hash` value,
when the row is read back,
then the column stores the hashed value verbatim.

Given an insert without `password_hash`,
when the database constraint fires,
then the row is rejected.
</task-scenarios>

- Suggested files: `migrations/`, `db/schema/users.ts`
</task>

## Phase 2: API

<task id="T-003" state="pending" covers="REQ-001">
## T-003: Implement `POST /api/signup` handler

<task-scenarios>
Given a request with valid credentials,
when the handler runs,
then it returns 200 with a session token and persists a user row.

Given a request with a duplicate email,
when the handler runs,
then it returns 409 with a message containing "already exists".

Given a request with an uppercase email,
when the handler runs,
then the email is normalized to lowercase before insertion.

Given a request with a malformed email,
when the handler runs,
then it returns 400 with a validation error.
</task-scenarios>

- Suggested files: `src/auth/signup.ts`, `tests/auth/signup.spec.ts`
</task>

<task id="T-004" state="pending" covers="REQ-002">
## T-004: Wire password hashing into signup flow

<task-scenarios>
Given a successful signup,
when the user row is inspected,
then `password_hash` is a valid hash and is not the plaintext password.

Given the hashing routine invoked twice with identical input,
when the resulting hashes are compared,
then they differ (salt is applied).
</task-scenarios>

- Suggested files: `src/auth/signup.ts`, `src/auth/password.ts`
</task>
```

### Element grammar

The element shapes mirror the SPEC.md grammar (line-isolated open and
close tags, double-quoted attributes, deterministic canonical form).

| Element | Cardinality | Parent | Required attributes | Notes |
|---|---|---|---|---|
| `task` | required, 1+ | top-level (bare under `# Tasks:` heading) | `id="T-NNN"`, `state="..."`, `covers="REQ-NNN[ REQ-NNN]*"` | Body is Markdown plus exactly one `<task-scenarios>` element. No `<implementer>` / `<review>` / `<blockers>` element may appear inside a `<task>` body; that activity prose lives in the sibling `journal/T-NNN.md` file. |
| `task-scenarios` | required, single per `<task>` | inside `<task>` | none | Slice-level Given/When/Then prose. Must be non-empty. |

Only `task` and `task-scenarios` are live Speccy element names inside a
TASKS.md body. The closed XML element set across all Speccy artifacts is,
per artifact:

- SPEC.md: `goals`, `non-goals`, `user-stories`, `assumptions`,
  `requirement`, `done-when`, `behavior`, `scenario`, `decision`,
  `open-question`, `changelog`
- TASKS.md: `task`, `task-scenarios`
- REPORT.md: `report`, `coverage`
- `journal/T-NNN.md`: `implementer`, `review`, `blockers`
- `journal/VET.md`: `drift-review`, `holistic-fix`, `simplifier-scan`,
  `simplifier-apply`, `gate`

`implementer`, `review`, and `blockers` only ever appear inside
`journal/T-NNN.md`, never in TASKS.md (see `TSK-006` in
[Lint codes](#lint-codes)).

Valid `state` attribute values are exactly `pending`, `in-progress`,
`in-review`, `completed`. The `covers` attribute is one or more
`REQ-\d{3,}` ids separated by single ASCII spaces. Every covered
requirement id is cross-checked against the parent SPEC.md element tree
at workspace load time. Unknown attributes on a known Speccy element are
parse errors.

Conventions:

- `T-NNN` ids in `<task id="...">` are unique within the file. The
  level-2 heading inside the body is decorative for human readers; the
  parser reads the id from the attribute.
- `covers="..."` is parsed by `speccy next` to know which requirements a
  task touches.
- `<task-scenarios>` carries the slice-level validation contract. The
  implementer translates each Given/When/Then in the block into an
  executable test in the project's framework, **writes the test before
  implementing the code path**, and ensures it passes before flipping
  the task's `state` to `in-review`.
- `Suggested files:` bullets are advisory; Speccy does not enforce write
  scope.
- Phase headings outside `<task>` elements are decorative.

The `<task-scenarios>` convention is what makes TDD legible without
making it a CLI gate. Skills prompt the implementer to write tests
first; the reviewer-tests persona checks that the listed scenarios exist
as tests and meaningfully exercise the claimed behavior.

Speccy parses TASKS.md to read each task's `id`, `state`, and `covers`
from the `<task>` element attributes, read the slice-level scenarios
from the nested `<task-scenarios>` block, find the next actionable task
(`state="pending"`), and detect "suggested files" hints in the task
body. It does not validate journal prose; that lives in the sibling
`journal/T-NNN.md` file.

### State model

Task states, carried by the `state` attribute on each `<task>` element.
Every transition between these states is written by
`speccy task transition` (a byte-surgical splice over the closed legal
graph), never by hand-editing the `state` attribute. The "Who sets it"
column names the skill that *invokes* the command at each edge.

| `state` value | Meaning | Who sets it (via `speccy task transition`) |
|---|---|---|
| `pending` | Needs work (new or retry) | Initial generation; reviewer/amend on blocking |
| `in-progress` | Claimed by an implementer | Implementer when starting |
| `in-review` | Implementation done, awaiting review | Implementer when finishing |
| `completed` | All persona reviews passed | Reviewer skill at exit of review primitive |

The closed legal graph the command enforces is: `pending→in-progress`,
`in-progress→in-review`, `in-review→completed`, `in-review→pending`,
`in-progress→pending`, `completed→pending`; a same-state target is an
idempotent no-op and any other edge is rejected (see
[CLI.md → `task transition`](./CLI.md#cli-surface)).

A retry is just `state="pending"` with prior activity entries attached
in the per-task journal. We do not introduce a fifth state because the
journal entries already say "this is a retry; see review findings."
Adding a state would add cases for skills to handle without adding
information.

---

## TASKS.md per-task journal

Implementer handoff prose, reviewer verdicts, and amendment-driven
blocker directives **do not live inside the `<task>` element body in
TASKS.md**. They live in a sibling `journal/T-NNN.md` file under the same
spec directory:

```text
.speccy/specs/0001-user-signup/
  SPEC.md
  TASKS.md
  REPORT.md
  journal/
    T-001.md
    T-002.md
    T-003.md
```

The journal directory sits alongside `SPEC.md`, `TASKS.md`, and
`REPORT.md`. A journal file is created on the first `<implementer>` write
(round 1 of an implementer attempt) and accumulates one `<implementer>`
block per round plus N `<review>` blocks per round of fan-out plus at
most one `<blockers>` block per round (when a reviewer blocks or an
amendment flips the task back to `pending`).

Every block is written by `speccy journal append`, which stamps `date`,
derives `round`, writes the frontmatter on first append, and serializes
concurrent appenders with a per-file advisory lock (see
[CLI.md](./CLI.md#cli-surface)). Callers, the implementer phase, each
reviewer persona, and the orchestrator's `<blockers>` directive, supply
only identity/judgment inputs (`model`, `persona`, `verdict`) and the
block body on stdin; they never author `date` or `round` themselves.
Reviewer sub-agents append their own `<review>` blocks rather than
returning them for a single writer to transcribe; the append lock, not a
sole-writer convention, is what keeps concurrent appends from
interleaving.

Each `journal/T-NNN.md` file has YAML frontmatter binding it to its task
plus a chronological body of bare `<implementer>`, `<review>`, and
`<blockers>` element blocks (no wrapper element):

```markdown
---
spec: SPEC-0001
task: T-002
generated_at: 2026-05-11T18:00:00Z
---

<implementer date="2026-05-11T18:00:00Z" model="claude-opus-4.8[1m]/low" round="1">
Renamed existing `password` column. Added migration to hash
plaintext rows. **Out of scope**: touched
`tests/migration_helpers.ts` to fix a test helper assuming
plaintext.
</implementer>

<review persona="business" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-opus-4.8[1m]/high" round="1">
Matches REQ-002 intent.
</review>

<review persona="tests" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-opus-4.8[1m]/medium" round="1">
Hash assertion present.
</review>

<review persona="security" verdict="blocking" date="2026-05-11T19:00:00Z" model="claude-opus-4-8[1m]/high" round="1">
bcrypt cost 10; policy requires >=12. See `src/auth/password.ts:14`.
</review>

<review persona="style" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-sonnet-4-6[1m]/medium" round="1">
Conventions OK.
</review>

<review persona="correctness" verdict="pass" date="2026-05-11T19:00:00Z" model="claude-opus-4-8[1m]/high" round="1">
Control flow and error handling sound.
</review>

<blockers date="2026-05-11T19:00:00Z" round="2">
Address bcrypt cost.
</blockers>
```

### Journal binding rules

Two bindings tie a journal file to its task and spec:

- **Filename ↔ task.** `journal/T-NNN.md` carries activity for the
  `<task id="T-NNN">` in the sibling TASKS.md. The frontmatter's `task:`
  field must agree with the filename digits; mismatches fire `JNL-003`.
- **Frontmatter ↔ spec.** The frontmatter's `spec:` field must agree
  with the parent directory's spec id and the sibling TASKS.md
  frontmatter's `spec:` field; mismatches fire `JNL-003`.

The frontmatter requires exactly three fields: `spec` (matching
`SPEC-\d{3,}`), `task` (matching `T-\d{3,}`), and `generated_at` (ISO8601
timestamp with seconds and timezone designator).

### Journal element grammar

| Element | Cardinality | Parent | Required attributes | Notes |
|---|---|---|---|---|
| `implementer` | 1+ per round, ≥1 round total | bare under frontmatter | `date`, `model`, `round` | Implementer handoff for one round. Body is Markdown using the multi-field handoff template (Completed / Undone / Commands run / Exit codes / Discovered issues / Procedural compliance). |
| `review` | 1+ per reviewed round | bare under frontmatter | `date`, `model`, `persona`, `verdict`, `round` | One reviewer's verdict for one round. `verdict` is `pass` or `blocking`; `persona` is one of the persona registry values. |
| `blockers` | 0 or 1 per round | bare under frontmatter | `date`, `round` | Directive carried across a retry boundary: either reviewer-aggregated blockers or an amendment-driven blocker. Body names what the next round must address. |

All attributes listed are required; there are no optional attributes in
the journal schema. Attribute value rules:

- `date`: full ISO8601 with seconds and timezone designator (regex
  `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[+-]\d{2}:\d{2})$`).
  `generated_at` in frontmatter uses the same format.
- `model`: non-empty string. The agreed skill-layer convention encodes
  effort via a slash suffix (e.g. `claude-opus-4.8[1m]/low`,
  `claude-sonnet-4-6[1m]/medium`); the parser does NOT validate
  slash-suffix internal structure; it only enforces non-empty.
- `round`: positive integer (regex `^[1-9][0-9]*$`).
- `verdict`: closed value set `{pass, blocking}`.
- `persona`: closed persona registry (`business`, `tests`, `security`,
  `style`, `correctness`, `architecture`, `docs`).

### Round monotonicity

The journal parser validates round sequence within a single file:

- The first `<implementer>` block must have `round="1"`.
- The `round` counter is monotonic non-decreasing across blocks.
- Counter must not skip values (no jumping from N to N+2 without an
  intervening N+1 block).
- Multiple blocks at the same round are allowed (one `<implementer>`
  plus N `<review>` plus at most one `<blockers>` per round).

Shape violations under either binding or monotonicity surface as
`JNL-003`.

### Journal lint activation

A `JNL-*` lint family enforces the journal contract. All `JNL-*` codes
default to `Level::Error` and gate `speccy verify`; their full entries
live in [Lint codes](#lint-codes). Tasks at `state="in-progress"` or
`state="in-review"` are silently skipped by all three JNL codes; the
family never runs mid-loop, so a half-written journal in flight is not a
lint error. The activation gate lives in the lint runner; each rule does
its own work assuming activation is granted.

`<implementer>`, `<review>`, and `<blockers>` elements are not in the
allow-list for TASKS.md bodies. If any of them appears inside a `<task>`
element in TASKS.md, `TSK-006` fires at `Level::Error` regardless of task
state, naming which element appeared, the containing task id, and the
canonical fix (move the block to `journal/T-NNN.md`). `TSK-006` fires
before any `JNL-*` diagnostic on the same task, because a misplaced
element in TASKS.md is more fundamental than a journal-shape issue.

### Lifecycle reading

An implementer picking up a task reads TASKS.md to find the next
`state="pending"` task, then reads `journal/T-NNN.md` (directly, or via
`speccy journal show`) to learn what prior rounds did, what reviewers
blocked, and what an amendment-driven `<blockers>` directive (if any)
asks the next round to address. The implementer then flips `state` back
to `in-progress` via `speccy task transition`, appends a new
`<implementer>` block via `speccy journal append` (the CLI derives the
next `round` value), does the work, flips `state` to `in-review`, and
exits.

---

## VET.md per-SPEC journal

Pre-ship drift review (the `/speccy-vet` skill) maintains a single
per-SPEC journal at `.speccy/specs/NNNN-slug/journal/VET.md`, sibling to
`SPEC.md`, `TASKS.md`, and the per-task `T-NNN.md` journal files. Every
block is written through `speccy journal append` against a bare
`SPEC-NNNN` selector: each vet sub-agent appends its own
`<drift-review>` / `<holistic-fix>` / `<simplifier-scan>` /
`<simplifier-apply>` block and returns a thin verdict, and the skill
appends the terminal `<gate>` block on exit. The CLI is the authority for
`date`, `round`, the `gate` block's `tasks_hash`, and the
`## Invocation N` sectioning, so callers supply only identity/judgment
inputs and the block body; the per-file append lock serializes
concurrent appenders.

The file opens with YAML frontmatter (`spec`, `generated_at`), then one
`## Invocation N — <ISO8601>` section per skill invocation. The CLI owns
the sectioning: when the file is absent or its last section is already
gate-terminated, `speccy journal append` opens the next `## Invocation N`
with a CLI-stamped datetime before writing the block, so a non-gate block
appended after a gate never lands in the closed section. Each section may
carry, in order of appearance:

- `<drift-review>`: output of one drift-reviewer sub-agent round. Opens
  a round.
- `<holistic-fix>`: output of one drift-implementer sub-agent round.
  Attaches to the current round; pairs with the preceding
  `<drift-review>`.
- `<simplifier-scan>`: output of the Phase 2 candidate scan
  (read-only).
- `<simplifier-apply>`: output of the Phase 2 apply round, when
  candidates were applied.
- `<gate>`: **terminal** block for the section. Exactly one per
  invocation, appended by every vet exit path (including the Phase 0
  early exits) via `speccy journal append --block gate` before the skill
  returns its `<orchestrator-verdict>` to its caller.

The `<gate>` block carries the durable signal `speccy next` reads to
decide whether the SPEC is freshly vetted. Shape:

```text
<gate verdict="passed|failed" tasks_hash="<lowercase-hex-sha256>" date="<ISO8601>">
<one-line human-readable summary>
</gate>
```

Attributes:

- `verdict`: `passed` when the skill's `<orchestrator-verdict>` will
  carry `verdict="pass"`; `failed` otherwise (including every Phase 0
  early-exit path).
- `tasks_hash`: lowercase hex SHA-256 of `<spec-dir>/TASKS.md` bytes,
  computed by `speccy journal append` immediately before writing the
  block (callers cannot supply it). Anchors the gate verdict to a
  specific TASKS.md revision so an amendment after the gate passed forces
  a re-vet on the next `speccy next` resolution.
- `date`: ISO8601 datetime with seconds and timezone designator.

The resolver reads the **last** `<gate>` block in the file. A SPEC with
all tasks `state="completed"` and either
no VET.md, a trailing `verdict="failed"` block, or a `verdict="passed"`
block whose `tasks_hash` does not match the on-disk TASKS.md SHA-256
resolves to `NextAction::Vet`. Only a trailing `verdict="passed"` block
whose `tasks_hash` matches advances the resolver past the vet step.

---

## Concurrent pickup

`state="in-progress"` on the `<task>` element is enough for `speccy next`
to skip in-progress tasks via the resolver's state-based priority (there
is no `--kind` flag, see [CLI.md → `speccy next`](./CLI.md#speccy-next-priority)).
If two agents race to *claim* the same `state="pending"` task, git will
conflict on the TASKS.md edit and one will lose. That is acceptable for
v1: task claiming is not locked.

Journal writes, by contrast, are serialized by the CLI. Both
`journal/T-NNN.md` and `VET.md` appends go through `speccy journal
append`, which takes a per-file advisory lock (blocking acquire with a
10-second timeout) around the read→derive→validate→write sequence.
Several reviewer or vet sub-agents can therefore append to the same
journal concurrently without interleaving or losing blocks, and `round` /
invocation derivation stays consistent under contention. This is the one
place Speccy v1 takes a lock; it is internal to the append command, with
no caller flags. A future harness may still add ticket queues or worktree
isolation for the unlocked task-claim race.

---

## REPORT.md

Written by the agent at the end of Phase 5 via the `/speccy-ship` skill
body. Speccy itself does not author REPORT.md and never renders
natural-text prompts.

REPORT.md is Markdown with requirement coverage carried by raw XML
element tags, mirroring SPEC.md and TASKS.md. Outcome and narrative
sections remain plain Markdown.

Suggested shape:

```markdown
---
spec: SPEC-001
outcome: delivered
generated_at: 2026-05-11T19:00:00Z
---

# Report: SPEC-001 User signup

<report spec="SPEC-001">

## Outcome
delivered | partial | abandoned

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
Account creation: project tests in `tests/auth/signup.spec.ts`
exercise CHK-001 and CHK-002 end to end.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
Password storage: project tests in `tests/auth/password.spec.ts`
exercise CHK-003.
</coverage>

## Task summary
- 6 tasks completed
- 1 task retried twice (T-002: bcrypt cost policy)
- 1 task triggered SPEC amendment (T-005 surfaced unknown about
  session TTL)

## Out-of-scope items absorbed
- `tests/migration_helpers.ts` updated alongside T-002
  (implementer note)

## Deferred / known limitations
- Rate limiting on signup endpoint (flagged by security review;
  deferred to SPEC-002)

## PR
[link filled in by agent after `gh pr create`]

</report>
```

### Element grammar

| Element | Cardinality | Parent | Required attributes | Notes |
|---|---|---|---|---|
| `report` | required, single | top-level | `spec="SPEC-NNNN"` | Wraps every `<coverage>` element in the file. |
| `coverage` | required, exactly one per surviving SPEC requirement | inside `<report>` | `req="REQ-NNN"`, `result="..."`, `scenarios="CHK-NNN[ CHK-NNN]*"` | Body is plain Markdown explanatory prose. |

Valid `result` attribute values are exactly `satisfied`, `partial`, and
`deferred`:

- `satisfied`: every scenario nested under the requirement in SPEC.md is
  exercised by a project test that the implementer or reviewer can point
  at.
- `partial`: some scenarios are exercised; others remain. The body prose
  names which ones and why.
- `deferred`: coverage is intentionally pushed to a later spec.
  `scenarios=""` is permitted on `deferred` rows.

There is **no** `dropped` value. If a requirement is genuinely no longer
in scope it is removed from SPEC.md via amendment (with a Changelog row
stating why) rather than carried as a `<coverage>` row. The renderer
enforces "exactly one `<coverage>` per surviving SPEC requirement"; a
requirement dropped from the SPEC disappears from REPORT.md alongside it.

`scenarios` is one or more `CHK-\d{3,}` ids separated by single ASCII
spaces. Each scenario id must be nested under the matching
`<requirement>` in SPEC.md; dangling ids are `RPT-*` lint errors (see
[Lint codes](#lint-codes)).

The grammar is enforced at workspace-load time by the `RPT-*` lint
family. The `partition_lint` demotion pass downgrades those codes to
`Level::Info` when the owning SPEC.md is `status: in-progress`, so an
in-flight amendment loop is never blocked by a REPORT.md that has not yet
been written. REPORT.md is the durable record of what happened during the
loop: future agents reading the repo can reconstruct intent from SPEC.md
and execution history from REPORT.md.

---

## Decisions (inline ADRs)

Decisions live inside each SPEC.md as `<decision id="DEC-NNN">` elements,
conventionally under a `## Design > ### Decisions` (or `## Decisions`)
heading. The element carries an optional
`status="accepted|rejected|deferred|superseded"` attribute; the body is
free Markdown that follows the classic ADR shape:

- **Context:** Why this decision needs to be made.
- **Decision:** What was chosen.
- **Alternatives:** Other options considered, with brief reason each was
  rejected or deferred.
- **Consequences:** What this commits the project to.

Decisions are parsed elements, not a CLI lifecycle. The parser validates
`<decision>` ids (duplicates are parse errors) and the `status`
attribute domain, and `speccy context` surfaces every decision body in
its intent section. There is no `speccy decision` command, no separate
lifecycle, and no linting of the ADR shape inside the body; that
structure is a convention skill prompts nudge agents toward.

`DEC-NNN` IDs are scoped to the spec (like `REQ-NNN` and `CHK-NNN`). Two
specs can both have `DEC-001`; they're local.

When a later spec changes a decision made in an earlier spec, the later
spec records the supersession in its own `### Decisions` block and
references the prior spec in prose:

```markdown
<decision id="DEC-001" status="accepted">
#### DEC-001: Password hashing algorithm
**Context:** SPEC-001 chose bcrypt cost 12. Subsequent benchmarking
showed argon2id is faster at equivalent security on current
hardware.
**Decision:** Migrate to argon2id with project-standard parameters.
**Supersedes:** SPEC-001 / DEC-001.
**Consequences:** ...
</decision>
```

Project-wide conventions that aren't tied to any one spec belong in
`AGENTS.md` as prose. AGENTS.md is loaded into every prompt; it's the
natural home for "this is how we do things across all features." The
reviewer-architecture persona reads `### Decisions` blocks in the SPEC.md
it's reviewing; the reviewer-docs persona may notice when an
implementation has drifted from a decision the spec records.

---

## Checks

A Check is an English validation scenario: a durable description of
behavior a requirement must satisfy. The CLI renders scenarios; it does
not execute them. Whether the project tests actually satisfy a scenario
is a question for project CI and for the reviewer-tests persona.

Scenarios live inside SPEC.md as `<scenario id="CHK-NNN">` elements
nested under their parent `<requirement id="REQ-NNN">`:

```markdown
<requirement id="REQ-001">
### REQ-001: Account creation
...

<scenario id="CHK-001">
Given no account exists for alice@example.com,
when the signup endpoint receives a valid request,
then a user row is persisted and the response includes a session token.
</scenario>
</requirement>
```

Required attribute: `id` matching `CHK-\d{3,}`. Unknown attributes on a
`<scenario>` element are parse errors. Empty or whitespace-only scenario
bodies are parse errors naming the containing `CHK-NNN`. Scenarios are
typically Given/When/Then prose, but the CLI does not parse the inner
structure. The body is preserved verbatim except for trailing whitespace
normalisation at element boundaries.

`speccy check` renders selected scenarios and exits; the selector grammar
and rendering behavior live in
[CLI.md → `speccy check` rendering](./CLI.md#speccy-check-rendering).

---

## Spec staleness detection

When SPEC.md is edited mid-loop (between Phase 2 and Phase 5), TASKS.md
may no longer reflect the current spec. Speccy detects this via the
content hash: TASKS.md frontmatter's `spec_hash_at_generation` stores the
sha256 of SPEC.md at the time TASKS.md was generated. `speccy status`
recomputes the current hash and compares; a mismatch is the sole stale
signal beyond the `bootstrap-pending` sentinel.

If it drifts, `speccy status` reports:

```text
SPEC-001: TASKS.md may be stale.
  Hash drift: SPEC.md sha256 changed since tasks were generated.
  Run /speccy-amend to reconcile.
```

This is a soft warning. The user / skill decides what to do. No gate
fires.

---

## Lint codes

Speccy emits a small set of deterministic lint codes. None depend on LLM
judgment. All have stable prefixes: `SPC-` for spec structure, `REQ-` for
requirements, `TSK-` for task structure, `QST-` for open questions,
`RPT-` for REPORT.md proof shape, `JNL-` for `journal/T-NNN.md` per-task
journal proof shape, `VET-` for `journal/VET.md` per-SPEC vet journal
proof shape, and `XML-` for foreign-tag balance across parsed artifacts.
The canonical, append-only list is the CLI's lint registry; a snapshot
test pins it. The summary below mirrors the registry.

```text
SPC-001  SPEC.md could not be read or its element tree failed to
         parse (Level::Error). Catch-all surface for I/O and
         element-tree parse errors against `SPEC.md`.
SPC-002  SPEC.md element tree malformed: heading declares an ID but
         no matching `<requirement>` element exists
SPC-003  SPEC.md element tree malformed: `<requirement>` element
         exists but SPEC.md has no matching `### REQ-NNN` heading
SPC-004  SPEC.md frontmatter missing required field
         (id / slug / title / status / created)
SPC-005  SPEC.md frontmatter status value is not one of:
         in-progress, implemented, dropped, superseded
SPC-006  status = superseded but no other spec in the workspace
         declares `supersedes` pointing to this spec
SPC-007  status = implemented but some tasks have state != "completed"
         (informational)

REQ-001  Requirement has no nested <scenario> element

TSK-001  TASKS.md task references non-existent REQ ID
TSK-002  TASKS.md task ID format invalid (expected T-NNN)
TSK-003  Spec hash mismatch: TASKS.md may be stale relative to
         SPEC.md (warning, not error)
TSK-004  TASKS.md frontmatter missing required field
         (spec / spec_hash_at_generation / generated_at)
TSK-005  Spec ID disagreement: folder digits, SPEC.md frontmatter
         `id:`, and TASKS.md frontmatter `spec:` must all agree
         (error; skipped when any of the three is unobtainable so
         upstream parse-error diagnostics cover those cases)
TSK-006  Misplaced journal element in TASKS.md: an `<implementer>`,
         `<review>`, or `<blockers>` element appears inside a
         `<task>` body. These elements only ever live in
         `journal/T-NNN.md` (Level::Error). Not gated by task
         state: fires identically against pending, in-progress,
         in-review, and completed tasks. Fires before any JNL-*
         diagnostic on the same task.

JNL-001  Task `state="pending"` but `journal/T-NNN.md` exists
         (Level::Error). A pending task has no implementer history;
         a journal file is unexpected.
JNL-002  Task `state="completed"` but `journal/T-NNN.md` is missing
         (Level::Error). Every completed task must carry its
         journal as the durable record of how it was implemented
         and reviewed.
JNL-003  Task `state="completed"` and `journal/T-NNN.md` has a
         shape or binding violation (Level::Error). Covers
         filename ↔ frontmatter `task:` mismatch, frontmatter
         `spec:` ↔ parent-dir mismatch, missing or unparseable
         frontmatter, attribute-schema violations on
         `<implementer>` / `<review>` / `<blockers>`, and
         round-monotonicity violations (first round must be 1,
         monotonic non-decreasing, no skipped rounds).
         The JNL-* family silently skips tasks at
         `state="in-progress"` or `state="in-review"`: a
         half-written journal in flight is not a lint error.

VET-001  `journal/VET.md` fails the frozen `vet_xml` grammar
         (Level::Error). Covers missing or malformed frontmatter, a
         bad block shape, an attribute value outside its domain, and
         an invalid per-section round sequence. Fires only when the
         file exists; a spec with no VET.md emits no VET-* code
         (absence is the resolver's concern, not lint's).
VET-002  `journal/VET.md` violates the terminal-`<gate>` structure
         (Level::Error). Fires when an invocation section other than
         the last lacks a terminal `gate`, a `gate` is not the last
         block in its section, or a section holds more than one
         `gate`. Like VET-001, runs only when VET.md exists.

QST-001  SPEC.md has unchecked open question (informational)

RPT-001  REPORT.md present but failed to parse (Level::Error).
         Fires when `ParsedSpec.report_md` is `Some(Err(_))`.
         Covers every failure the parser returns: missing `spec="..."`
         attribute on the root `<report>` element, malformed
         `<coverage>` shape, fenced-code-block boundary violations,
         and any other parse error. The diagnostic message includes
         the underlying parse error rendered via its Display impl.
RPT-002  `<coverage req="REQ-NNN">` row references a requirement id
         that has no matching `<requirement id="REQ-NNN">` in the
         sibling SPEC.md (Level::Error). Fires once per dangling
         reference. Does not fire when SPEC.md itself failed to parse
         (SPC-001 owns that surface). When RPT-002 fires for a row,
         RPT-003 does not fire for any of that row's scenarios.
RPT-003  Scenario id in `<coverage scenarios="...">` does not resolve
         to a `<scenario id="...">` nested under the named requirement
         in the sibling SPEC.md (Level::Error). Fires once per
         dangling scenario id. Suppressed for rows where RPT-002
         already fired (the row is already broken at the requirement
         level; one diagnostic per row rather than N).

XML-001  Orphan foreign (non-whitelisted) XML tag in a parsed
         artifact (Level::Error): a close tag with no matching
         preceding open, or a non-void open tag with no matching
         following close. One diagnostic per orphan tag, naming the
         artifact path and the offending 1-indexed source line.
         Covers SPEC.md, TASKS.md, REPORT.md, and existing per-task
         `journal/T-NNN.md` files. Balance is name-scoped (a
         per-name stack) and fence-aware; cross-name nesting is not
         enforced.
```

`REQ-002` and `REQ-003` are registry-only entries kept for stability:
both fired pre-XML-canonical-SPEC.md but are no longer reachable at parse
time (the parser rejects orphan scenarios before lint runs). Their slots
stay in the snapshot so removing them would be a breaking change.

Nothing in this list grades scenario quality mechanically. The CLI flags
presence and reference shape only; whether a scenario is meaningful and
whether the project tests actually cover it goes to review.

Lint codes are stable: changing a code's meaning between minor versions
is a breaking change. Adding new codes is fine.

---

## Parsing stack and file-shape details

Implementation choices that decide how the artifacts above are read.

### Parsing stack

| Concern | Crate | Version pin |
|---|---|---|
| Markdown body | `comrak` (CommonMark + GFM tables) | latest stable |
| YAML deserialization | `serde-saphyr` (serde adapter over `saphyr-parser`) | exact `0.0.x` |
| TOML deserialization | `toml` | latest stable |
| Targeted regex | `regex` (only for ID extraction from heading text) | latest stable |

`serde-saphyr` is built on `saphyr-parser` (the actively-developed
pure-Rust YAML 1.2 parser), with direct-to-struct deserialization (no
`Value` AST roundtrip), panic-free on malformed input, and configurable
resource budgets that defend against Billion-Laughs attacks. It is
`0.0.x` (pre-`0.1.0`): pin the exact version and expect a minor refactor
when it stabilizes.

Frontmatter extraction is DIY: the `---` fence parser is a few lines of
string slicing returning `&str` slices for the YAML chunk and the
markdown body, which avoids tying frontmatter parsing to one specific
YAML crate. Pure-regex structural parsing was rejected because SPEC.md
contains fenced code blocks with example markdown that regex cannot
reliably skip.

TASKS.md, REPORT.md, and `journal/T-NNN.md` share the same line-aware XML
element scanner as SPEC.md. It extracts the bare `<task>` /
`<task-scenarios>` tree from TASKS.md (no `<tasks>` wrapper), the
`<report>` / `<coverage>` tree from REPORT.md, and the chronological
bare-element sequence of `<implementer>` / `<review>` / `<blockers>`
blocks (no wrapper) from `journal/T-NNN.md`. Body Markdown inside each
element is preserved verbatim except for trailing whitespace
normalization at element boundaries. No regex is used for structure;
element opens, closes, and attributes are parsed line-by-line with
fenced-code awareness.

### Section heading discovery in SPEC.md

Case-insensitive exact match. `## Open Questions`, `## open questions`,
`## OPEN QUESTIONS` all match. Hyphens and spaces in heading text are
treated equivalently for matching. Unknown headings are ignored, not
flagged.

### Frontmatter dates

- `created`: ISO 8601 date (`YYYY-MM-DD`)
- `generated_at`: ISO 8601 datetime in UTC (`YYYY-MM-DDTHH:MM:SSZ`)

Missing optional frontmatter fields are treated identically to empty
lists or null. The parser does not distinguish.

### Subdirectory naming

Spec folders: `NNNN-slug-with-hyphens`. Slug is lowercase ASCII only: the
workspace scanner enumerates only directories matching
`^\d{4}-[a-z0-9-]+$`, so a folder with an uppercase or non-ASCII name is
simply not recognised as a spec (no lint fires). There is no lint
cross-checking `frontmatter.slug` against the folder name; the field is
required to be present (`SPC-004`) but its value is not validated against
the directory.
