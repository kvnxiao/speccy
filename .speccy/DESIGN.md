# Speccy Design

> Canonical design for Speccy.
>
> Speccy is an AI-first, lightweight feedback engine for spec-driven
> development. It does not try to enforce determinism on LLMs. It makes
> intent, proof, and drift mechanically visible so agents and humans
> can catch divergence before it ships.

---

# Mission

Speccy is a deterministic CLI that lets humans and AI agents collaborate
on software with bounded drift. It exists because LLM nondeterminism
compounds: small misreadings of intent accumulate across features until
what shipped no longer matches what was asked for.

Speccy does not try to make LLMs deterministic. It makes the **contract**
between user intent and shipped behavior visible, so drift is loud the
moment it happens.

Speccy is built for a post AI-assisted engineering world where agents
draft specs, decompose tasks, implement, review adversarially, and
report. The human role is to:

- state intent
- answer material product questions
- approve or reject major tradeoffs
- perform final acceptance

The CLI is intentionally thin. The intelligence lives at the edges:
in skills, prompts, personas, and reviewers. The Rust CLI does not
call LLMs in v1.

---

# Stance: Feedback, Not Enforcement

LLMs do not reliably follow instructions. Treating Speccy as an
enforcement system would be a category error: every gate we add is
just another instruction an LLM can fail to obey, and every failure
mode of enforcement (false positives, blocked-but-actually-fine,
agent works around the gate) is worse than visibility.

So Speccy is a **feedback engine**:

- The CLI tells you what looks off; you decide.
- `speccy verify` is the only command that exits non-zero on
  problems. CI calls it. Local runs print findings and exit zero.
- There is no `--strict` mode, no policy file, no configurable
  enforcement. Speccy is opinionated about what to surface and
  silent about what to do about it.
- Skills wrap this feedback into agent workflows. The skill layer
  is where the loops live, where personas are defined, and where
  intelligence about "what to do next" gets exercised.

If Speccy ever feels like it's getting in the way, that's a bug in
Speccy, not in the user's workflow.

---

# Five Proper Nouns

| Noun | What it is | Where it lives |
|---|---|---|
| **Vision** | Product north star | `VISION.md` |
| **Spec** | One bounded behavior contract | `specs/NNNN-slug/SPEC.md` + `spec.toml` |
| **Requirement** | One observable behavior with a done condition | Heading in SPEC.md + entry in `spec.toml` |
| **Task** | One implementation slice sized for one agent | Line in `TASKS.md` |
| **Check** | One proof obligation (executable or manual) | Entry in `spec.toml` |

That is the complete conceptual vocabulary. Capability, milestone,
release, decision, amendment, assumption, constraint, invariant,
question, scenario, claim, lease, validation, evidence, finding,
review, and amendment are all either cut, derived from artifact
state, or rendered as freeform markdown sections inside SPEC.md /
TASKS.md.

---

# Core Development Loop

The full loop is five phases, alternating between planning and
agent-orchestrated loops:

```
1. plan        agent writes SPEC.md + spec.toml (PRD-shaped)
2. tasks       agent writes TASKS.md (sized for one sub-agent each)
3. impl loop   main agent spawns implementer sub-agents per task
4. review loop main agent spawns reviewer sub-agents per persona per task
5. report      agent writes REPORT.md and opens PR
```

The CLI does not spawn sub-agents. The CLI does not run loops. The
CLI renders prompts, queries artifact state, runs checks, and
records nothing about its own execution. Loops live in the harness
or in skills.

---

# CLI Surface

Ten commands. Two optional flags (`--kind`, `--persona`). Each
command maps to a specific lifecycle phase or query. No state-
transition verbs. No mode toggles.

```text
speccy init                       Scaffold .speccy/ + host skill pack
speccy plan [SPEC-ID]             Phase 1 prompt
                                    no arg:  Vision -> new SPEC scaffold
                                    SPEC-ID: amend existing SPEC.md
speccy tasks SPEC-ID              Phase 2 prompt
                                    TASKS.md absent:  initial generation
                                    TASKS.md present: amendment prompt
speccy implement TASK-ID          Phase 3 prompt (implementer)
speccy review TASK-ID             Phase 4 prompt (reviewer)
                                    --persona business | tests | security
                                              | style | architecture | docs
speccy report SPEC-ID             Phase 5 prompt (REPORT.md)
speccy status                     Show state, lint findings  (--json)
speccy next [--kind K]            Next actionable thing      (--json)
                                    --kind implement -> next [ ] task
                                    --kind review    -> next [?] task
                                    default          -> highest-priority
speccy check [CHK-ID]             Run check(s); non-zero on failure
speccy verify                     CI gate: lint + speccy check
```

The split between `implement` and `review` is deliberate: they are
different lifecycle phases that happen to both operate on tasks,
and conflating them under a generic `prompt --persona` flag was
miscategorising "what loop am I in" as "which sub-type of
reviewer." `--persona` lives only on `review` because review is
the only phase with parallel sub-types.

`speccy tasks SPEC-ID --commit` is a sub-action that records the
SPEC.md hash into TASKS.md frontmatter after the agent finishes
writing it. Used by skills, not typed directly.

That is the complete public surface. Anything else is a skill
responsibility.

---

# File Layout

```text
.speccy/
  speccy.toml
  VISION.md
  AGENTS.md
  specs/
    0001-user-signup/
      SPEC.md            Frontmatter + PRD prose + Decisions + Changelog
      TASKS.md           Frontmatter (gen hash) + checklist + notes
      spec.toml          Requirement <-> Check mapping (machine contract)
      REPORT.md          Frontmatter (outcome) + summary (end of loop)

skills/                  Shipped with Speccy; copied by `speccy init`
  claude-code/
  codex/
  shared/
    personas/
    prompts/
```

`skills/` is a top-level directory in the Speccy workspace. `speccy
init` copies the appropriate host pack into the user's project at
the host-native location (e.g. `.claude/commands/` for Claude Code,
`.codex/skills/` for Codex).

Decisions (ADRs) are not a separate folder. Each spec's `## Design
> Decisions` subsection holds the architectural choices made for
that spec. Project-wide conventions that span specs belong in
`AGENTS.md`.

---

# Workflow Phases

## Phase 1: Planning

```sh
speccy plan
```

Renders a deterministic prompt that asks the agent to:

- read `VISION.md`
- propose the first SPEC slice
- write `specs/NNNN-slug/SPEC.md` (PRD-shaped, see template)
- write `specs/NNNN-slug/spec.toml` (IDs + check definitions)
- surface material questions inline in SPEC.md

If `SPEC-ID` is passed, the prompt instead asks for a minimal
amendment to the existing SPEC.md.

## Phase 2: Task decomposition

```sh
speccy tasks SPEC-001
```

Renders a prompt that asks the agent to:

- read the SPEC.md
- decompose into ordered tasks small enough for one sub-agent
- group by phase if useful
- reference REQ IDs each task covers
- write `specs/NNNN-slug/TASKS.md`

After the agent writes TASKS.md, the skill calls:

```sh
speccy tasks SPEC-001 --commit
```

This records the current SPEC.md sha256 hash and timestamp into
TASKS.md frontmatter (`spec_hash_at_generation`, `generated_at`).
Used for staleness detection in later phases.

If TASKS.md already exists, the prompt is an **amendment** prompt:
preserve completed tasks, modify or remove invalidated tasks, add
new ones for new requirements.

## Phase 3: Implementation loop (skill-orchestrated)

The `/speccy-work` skill, run by the main agent, executes this loop:

```text
loop:
  next = `speccy next --kind implement --json`
  if next is empty: break

  prompt = `speccy implement {next.task}`
  spawn implementer sub-agent with prompt

  sub-agent:
    - flips [ ] -> [~] with session marker
    - implements the task
    - runs `speccy check` locally (fail-fast on red)
    - leaves inline notes for out-of-scope work or unknowns
    - flips [~] -> [?]
```

Concurrency is the main agent's choice. Two sub-agents may pick
different `[ ]` tasks in parallel; they will conflict in git if
they touch the same files. Speccy does not lock.

## Phase 4: Review loop (skill-orchestrated)

The `/speccy-review` skill, run by the main agent, executes this:

```text
loop:
  next = `speccy next --kind review --json`
  if next is empty: break

  for persona in next.personas:
    prompt = `speccy review {next.task} --persona {persona}`
    spawn reviewer sub-agent with prompt

    sub-agent:
      - reads task + diff + SPEC.md
      - appends "Review ({persona}, pass|blocking): ..." inline

  if all persona reviews PASS:
    flip [?] -> [x]
  else:
    flip [?] -> [ ] and append "Retry: ..." note
```

Failed tasks return to `[ ]`. The main agent reads `speccy next
--kind implement --json` again and Phase 3 picks them back up.

The default reviewer persona fan-out is: **business**, **tests**,
**security**, **style**. The other personas (**architecture**,
**docs**) are available via `--persona` but not in the default fan-
out. Projects can override the default set in `speccy.toml` later
if necessary; v1 ships with this default.

## Phase 5: Report and PR

When `speccy next` returns empty for both `--kind` values, the loop
is complete.

```sh
speccy report SPEC-001
```

Renders a prompt that asks the agent to write `REPORT.md`
summarizing:

- requirements satisfied
- tasks completed (with retry counts derived from inline notes)
- out-of-scope items absorbed
- deferred or known limitations
- check results summary

The skill then opens a PR via `gh` (or equivalent). Speccy does not
touch GitHub.

---

# TASKS.md State Model

Four checkbox states. All standard markdown.

| Marker | Meaning | Who sets it |
|---|---|---|
| `[ ]` | Needs work (new or retry) | Initial generation; reviewer on blocking |
| `[~]` | In-progress (claimed by an implementer) | Implementer when starting |
| `[?]` | Implementation done, awaiting review | Implementer when finishing |
| `[x]` | All persona reviews passed | Main agent after review loop |

A retry is just `[ ]` with prior notes attached. We do not introduce
a `[!]` state because the inline notes already say "this is a
retry; see review findings." Adding a state would add cases for
skills to handle without adding information.

## Conventions for inline notes

Implementer notes:

```markdown
- [~] **T-002** (session-abc, 2026-05-11T18:00Z): Add password_hash column
  - Covers: REQ-002
  - Suggested files: `migrations/`, `db/schema/users.ts`
```

When the implementer finishes:

```markdown
- [?] **T-002**: Add password_hash column
  - Covers: REQ-002
  - Suggested files: `migrations/`, `db/schema/users.ts`
  - Implementer note (session-abc): Renamed existing `password` column.
    Added migration to hash plaintext rows. **Out of scope**: touched
    `tests/migration_helpers.ts` to fix a test helper assuming plaintext.
```

After review:

```markdown
- [ ] **T-002**: Add password_hash column
  - Covers: REQ-002
  - Implementer note (session-abc): ...
  - Review (business, pass): matches REQ-002 intent.
  - Review (tests, pass): hash assertion present.
  - Review (security, blocking): bcrypt cost 10; policy requires >=12.
    See `src/auth/password.ts:14`.
  - Review (style, pass): conventions OK.
  - Retry: address bcrypt cost.
```

The implementer picking this up reads all notes, addresses
blockers, flips back to `[~]`, and so on.

## Concurrent pickup

`[~]` with a session marker is enough for `speccy next --kind
implement` to skip in-progress tasks. If two agents race to claim
the same `[ ]` task, git will conflict on the TASKS.md edit and one
will lose. That is acceptable for v1.

A future harness may add file-locking, ticket queues, or worktree
isolation. Speccy v1 does not.

---

# Artifacts

## VISION.md

The greenfield seed. Not a full spec.

Recommended sections:

```markdown
# Vision

## Product
What we are building and why.

## Users
Who will use this. What they need.

## V1.0 outcome
What "done enough to call v1" means.

## Constraints
Things we are not free to violate.

## Non-goals
Things we are explicitly not building.

## Quality bar
What "good enough to ship" looks like.

## Known unknowns
Things we expect to learn during construction.
```

VISION.md is markdown; Speccy does not parse its structure. The
`speccy plan` prompt asks the agent to read it.

## speccy.toml

```toml
schema_version = 1

[project]
name = "taskify"
root = ".."
```

That is the complete configuration. There is no `[policy]` block,
no `[env]` block, no review identity setting, no
`[[global_checks]]` array. Project-level conventions and toolchain
context belong in `AGENTS.md`, which every skill prompt already
loads. If the CLI ever needs structured access to environment
metadata, the block will come back with a real purpose; until
then, it isn't here.

## SPEC.md (PRD-shaped template)

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
- Concrete outcomes this spec must achieve.

## Non-goals
- Explicitly out of scope. Things readers might assume but shouldn't.

## User stories
- As a new visitor, I want to create an account with email/password
  so that I can save my work between sessions.
- As a returning user, I want a clear error when I try to sign up
  with an email that already exists.

## Requirements

### REQ-001: Account creation
Users can create an account with email and password.

**Done when:** A valid signup request persists a user record and
returns a session token; duplicate email returns 409 with an
actionable message.

**Behavior:**
- Given no account exists for `alice@example.com`, when a signup
  request submits valid credentials, then a user record is
  persisted and the response includes a session token.
- Given an account already exists for `alice@example.com`, when a
  signup request submits the same email, then the response is 409
  with an error message containing "already exists".
- Given a signup request submits an invalid email format, when
  processed, then the response is 400 with a validation error.

**Covered by:** CHK-001, CHK-002, CHK-003

### REQ-002: Password storage
Passwords are hashed before persistence; plaintext never touches
storage.

**Done when:** Inspection of the users table shows hashed values;
a direct DB query for the password column never returns plaintext.

**Behavior:**
- Given a signup request with password `correct horse battery
  staple`, when the user record is persisted, then the password
  column contains a hash and never the original string.
- Given the users table is dumped to logs, when inspected, then
  no plaintext passwords appear.

**Covered by:** CHK-004

## Design

### Approach
[1-2 paragraphs of technical approach.]

### Decisions

#### DEC-001: Password hashing algorithm
**Status:** Accepted
**Context:** Signup requires password auth without hosted services.
**Decision:** bcrypt with cost factor 12.
**Alternatives:** Hosted auth (deferred, requires email
infrastructure); argon2 (deferred, no clear need yet); plaintext
with separate KMS (rejected: KMS not yet provisioned).
**Consequences:** App owns credential storage risk. Security
review must inspect password handling on every auth-touching
change. Cost factor revisits required when hardware baselines
shift.

#### DEC-002: Session storage
**Status:** Accepted
**Context:** Signup must return something a returning user can
present to authenticate later requests.
**Decision:** JWT signed with project secret, 24h expiry, stored
in httpOnly Secure cookie.
**Alternatives:** Server-side sessions in Redis (rejected: adds
infrastructure dependency); long-lived API tokens (rejected:
revocation story is poor).
**Consequences:** Stateless auth; horizontal scaling is trivial.
Token revocation requires key rotation or a blocklist (deferred).

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
- Email uniqueness enforced at the DB layer via index.

## Changelog

| Date       | Author          | Summary |
|------------|-----------------|---------|
| 2026-05-11 | agent/claude-1  | Initial draft from VISION.md |

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

Supersession is stored on the **new** spec (the one doing the
replacing) via `supersedes`. The inverse direction is **computed**
by walking the supersedes graph across all specs in the workspace;
no `superseded_by` field is stored. This keeps lineage
single-sourced; the older spec does not need to be updated when a
new spec replaces it.

`status` transitions:

```text
in-progress -> implemented      All tasks [x], REPORT.md written, PR merged.
in-progress -> dropped          Intent abandoned. Add a Changelog row stating why.
implemented -> superseded       A later spec declared `supersedes` pointing here.
in-progress -> superseded       Rare; replaced before completion.
```

Skills (specifically `/speccy-ship` and `/speccy-amend`) update `status`.
The CLI doesn't auto-transition state — it surfaces inconsistencies via lint
(e.g. `status: implemented` but tasks aren't all `[x]`).

### Changelog table

The `## Changelog` table is the in-doc lineage. Every material change to
SPEC.md after initial draft adds a row:

| Date | Author | Summary |
|------|--------|---------|
| 2026-05-11 | agent/claude-1 | Initial draft |
| 2026-05-13 | agent/claude-1 | REQ-002 bcrypt cost bumped to 12 per security review F-001 |
| 2026-05-14 | human/kevin | Dropped REQ-003 (magic-link auth) — out of v1 scope |

The Changelog replaces both the cut delta markers and the cut amendment
artifact. It is git-history-redundant by design — git tells you *what*
changed; the Changelog summarizes *why* and is loaded into every prompt
that reads SPEC.md.

Reviewer personas read the Changelog to understand recent intent
shifts. The skill prompt for `/speccy-amend` instructs the agent to
append a Changelog row whenever it edits SPEC.md.

### Lint behavior

Speccy lints three things in SPEC.md:

1. Required frontmatter fields are present.
2. Every `REQ-NNN` heading in SPEC.md matches a `[[requirements]]` row
   in `spec.toml` (and vice versa).
3. Any unchecked `- [ ]` in the **Open questions** section is reported
   in `speccy status` as a soft signal.

Nothing else in SPEC.md is parsed or enforced. The template is a
convention; the agent's skill prompts nudge the shape.

### Tests in English first (TDD convention)

The `**Behavior:**` block under each requirement is the **higher-level
test specification** in prose. Each bullet is one Given/When/Then
scenario that maps to one or more Checks. These describe integration
or end-to-end behavior at the requirement level.

Unit-level tests live in TASKS.md (see below) as `**Tests to write:**`
bullets on each task. This split is intentional:

- **SPEC.md behavior**: what the system does, observable from outside.
  Maps to `kind = "test"` checks that exercise the integration surface.
- **TASKS.md tests**: what each implementation slice must verify.
  Maps to unit tests the implementer writes before code.

Agents writing implementation code translate these prose tests into
executable tests in the project's framework, then implement to make
them pass. Speccy does not enforce TDD ordering (red-before-green); it
makes the test obligations visible and the reviewer-tests persona
checks that they're meaningful.

### Brownfield posture

There is no greenfield/brownfield mode toggle, no `origin` field,
and no per-requirement delta markers. Brownfield-aware spec
authoring is the planner skill's job:

- The planner persona detects existing code, lockfiles, and
  conventions in the repo.
- It reads enough context to write SPEC.md prose that accurately
  reflects "this behavior already exists" vs "this is new."
- When a new spec changes a previously-shipped spec, the new spec's
  frontmatter sets `supersedes: [SPEC-NNN]` and the prose explicitly
  references which prior behavior is being changed.

The combination of `frontmatter.status`, `frontmatter.supersedes`,
and the `## Changelog` table is sufficient to track spec evolution
without per-requirement annotations. Reviewers reading a SPEC.md
see immediately what state it's in, what (if anything) it replaces,
and how it has evolved.

## TASKS.md format

```markdown
---
spec: SPEC-001
spec_hash_at_generation: sha256:abc...123
generated_at: 2026-05-11T18:00:00Z
---

# Tasks: SPEC-001 User signup

## Phase 1: Schema
- [ ] **T-001**: Add `users` table migration with unique email index
  - Covers: REQ-001
  - Tests to write:
    - When the migration runs forward, the `users` table exists with
      a unique index on `email`.
    - When the same email is inserted twice, the second insert fails
      with a uniqueness violation.
  - Suggested files: `migrations/`, `db/schema/users.ts`

- [ ] **T-002**: Add `password_hash` column to `users`
  - Covers: REQ-002
  - Tests to write:
    - When a user row is inserted with `password_hash`, the column
      stores a non-null hashed value.
    - The schema rejects rows missing `password_hash`.
  - Suggested files: `migrations/`, `db/schema/users.ts`

## Phase 2: API
- [ ] **T-003**: Implement `POST /api/signup` handler
  - Covers: REQ-001
  - Tests to write:
    - When a request includes valid credentials, the handler returns
      200 with a session token and persists a user row.
    - When a request includes a duplicate email, the handler returns
      409 with an error message containing "already exists".
    - When a request includes an uppercase email, it is normalized
      to lowercase before insertion.
    - When a request includes a malformed email, the handler returns
      400 with a validation error.
  - Suggested files: `src/auth/signup.ts`, `tests/auth/signup.spec.ts`

- [ ] **T-004**: Wire password hashing into signup flow
  - Covers: REQ-002
  - Tests to write:
    - When signup succeeds, the persisted `password_hash` is a valid
      hash (not the original password).
    - When the hashing routine is called with the same input twice,
      the resulting hashes differ (salt is applied).
  - Suggested files: `src/auth/signup.ts`, `src/auth/password.ts`
```

Conventions:

- `**T-NNN**` IDs are stable within the file.
- `Covers:` is parsed by `speccy next` to know which requirements
  a task touches.
- `Tests to write:` lists unit-level test obligations in prose. The
  implementer translates each bullet into an executable test in the
  project's framework, **writes the test before implementing the
  code path**, and ensures it passes before flipping the task to
  `[?]`.
- `Suggested files:` is advisory; Speccy does not enforce write
  scope.
- Phase headings are decorative.

The `Tests to write:` convention is what makes TDD legible without
making it a CLI gate. Skills prompt the implementer to write tests
first; the reviewer-tests persona checks that the listed tests exist
and meaningfully exercise the claimed behavior. Speccy itself
doesn't verify the order of edits — that's a review concern.

Speccy parses TASKS.md only to:

- count `[ ]` / `[~]` / `[?]` / `[x]` markers
- find the next actionable task line
- detect "suggested files" hints
- read inline notes for status reporting

It does not validate note format or persona-review prose.

## spec.toml

`spec.toml` is the machine contract for the requirement-to-check
mapping. Spec metadata (id, slug, title, status, lifecycle) lives in
the SPEC.md frontmatter; task-generation freshness lives in TASKS.md
frontmatter. spec.toml is just the proof chain.

```toml
schema_version = 1

[[requirements]]
id = "REQ-001"
checks = ["CHK-001", "CHK-002"]

[[requirements]]
id = "REQ-002"
checks = ["CHK-003"]

[[checks]]
id = "CHK-001"
kind = "test"
command = "npm test -- signup.spec.ts"
proves = "Valid signup persists a user."

[[checks]]
id = "CHK-002"
kind = "test"
command = "npm test -- signup-duplicate.spec.ts"
proves = "Duplicate email returns 409."

[[checks]]
id = "CHK-003"
kind = "test"
command = "npm test -- password-storage.spec.ts"
proves = "Passwords are hashed before persistence."
```

Notes:

- No `[spec]` block. Spec metadata (id, slug, title, status,
  created, supersedes) lives in SPEC.md frontmatter.
- No `[tasks_generation]` block. Spec-hash freshness lives in
  TASKS.md frontmatter (`spec_hash_at_generation`,
  `generated_at`).
- No `inputs` field on checks. The freshness model it enables
  relies on agents declaring correct globs, which is exactly the
  kind of thing LLMs get wrong silently.
- No `critical` flag. All requirements are treated equally.
- No `origin` field. Brownfield context is the planner skill's
  responsibility (see "Brownfield posture" above).
- No `[design]` block. Design lives in SPEC.md narrative.
- No `[[tasks]]` table. Tasks live in TASKS.md.
- No `[claim]` table. No leases in v1.
- `kind` is a free-form string. Conventions: `test`, `command`,
  `manual`. Projects may add others (`property`, `static`,
  `schema`, `snapshot`, `migration`). Speccy does not parse the
  value beyond display.

## REPORT.md

Generated by the agent at the end of Phase 5. Speccy renders the
prompt; the agent writes the file.

Suggested shape:

```markdown
---
spec: SPEC-001
outcome: delivered
generated_at: 2026-05-11T19:00:00Z
---

# Report: SPEC-001 User signup

## Outcome
delivered | partial | abandoned

## Requirements coverage
- REQ-001 Account creation -- CHK-001, CHK-002 PASS
- REQ-002 Password storage -- CHK-003 PASS

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
```

REPORT.md is the durable record of what happened during the loop.
Future agents reading the repo can reconstruct intent from SPEC.md
and execution history from REPORT.md.

## Decisions (inline ADRs)

Decisions live inside each SPEC.md under `## Design > ### Decisions`
as `#### DEC-NNN: <title>` sub-headings. They follow the classic
ADR shape:

- **Status:** Accepted | Proposed | Rejected | Superseded
- **Context:** Why this decision needs to be made.
- **Decision:** What was chosen.
- **Alternatives:** Other options considered, with brief reason
  each was rejected or deferred.
- **Consequences:** What this commits the project to.

> **Decisions are a documented convention, not a CLI noun.** Speccy
> does not parse decision blocks beyond surfacing them in prompts.
> There is no `speccy decision` command, no separate lifecycle, no
> linting of decision shape. The structure is a convention skill
> prompts nudge agents toward.

`DEC-NNN` IDs are scoped to the spec (like `REQ-NNN` and `CHK-NNN`).
Two specs can both have `DEC-001`; they're local.

When a later spec changes a decision made in an earlier spec, the
later spec records the supersession in its own `### Decisions` block
and references the prior spec in prose:

```markdown
#### DEC-001: Password hashing algorithm
**Status:** Accepted
**Context:** SPEC-001 chose bcrypt cost 12. Subsequent benchmarking
showed argon2id is faster at equivalent security on current
hardware.
**Decision:** Migrate to argon2id with project-standard parameters.
**Supersedes:** SPEC-001 / DEC-001.
**Consequences:** ...
```

Project-wide conventions that aren't tied to any one spec belong in
`AGENTS.md` as prose. AGENTS.md is loaded into every prompt; it's
the natural home for "this is how we do things across all features."

The reviewer-architecture persona reads `### Decisions` blocks in
the SPEC.md it's reviewing. The reviewer-docs persona may notice
when an implementation has drifted from a decision the spec records.

---

# Checks

A Check is a proof obligation. The CLI executes checks but does
not record execution results.

## Definition

```toml
[[checks]]
id = "CHK-001"
kind = "test"
command = "npm test -- signup.spec.ts"
proves = "Valid signup persists a user."
```

Required fields: `id`, `kind`, `proves`. Executable checks also
require `command`. Manual checks require `prompt` instead of
`command`:

```toml
[[checks]]
id = "CHK-MANUAL-001"
kind = "manual"
prompt = "Sign up via the UI; confirm duplicate email shows the
         error toast."
proves = "End-to-end signup UX surfaces duplicate-email error."
```

## Execution

```sh
speccy check               # run all checks across all specs
speccy check CHK-001       # run one check
```

Behavior:

- Executes the `command` from the project root.
- Streams stdout / stderr live to the terminal.
- Exits **non-zero** if any check fails.
- Prints a summary at the end.

No record is written. CI (which calls `speccy verify`, which calls
`speccy check`) is where execution artifacts live (job logs, test
reports, etc.). Committing check records into the repo on every
run was rejected as ceremony in earlier design iterations.

For manual checks, `speccy check CHK-MANUAL-001` prints the prompt
and exits zero with a "manual; verify and proceed" note. There is
no manual-evidence file; the reviewer-business or reviewer-tests
persona is expected to vouch for it.

## No freshness, no records

Speccy does not store hashes of inputs, environment, or definitions
against check runs. The rationale:

- Storing input globs requires agents to declare them correctly;
  they don't. Wrong inputs lead to false-fresh and false-stale,
  both of which mislead downstream prompts.
- CI is the authoritative place for "did this pass on HEAD?" Local
  runs are advisory.
- The proof chain in v1 is: SPEC.md says what behavior matters;
  spec.toml says what checks prove it; `speccy check` runs them
  on demand. If you want to know "are the checks passing right
  now?", you run `speccy check`.

This is a deliberate simplification. The previous design's freshness
machinery added complexity for negligible benefit in a feedback
system.

---

# Review

Review is an adversarial proof challenge. The CLI renders prompts;
the skill layer orchestrates multiple reviewer personas in parallel.

## Personas

Speccy ships with these personas (markdown skill files):

| Persona | Catches |
|---|---|
| `business` | Does the implementation match SPEC.md intent? Are user stories satisfied? Are non-goals respected? |
| `tests` | Are checks meaningful or vacuous? Edge cases covered? Are negative cases asserted? Is the test exercising the actual behavior, or testing the mock? |
| `security` | Auth, input validation, secrets, injection, sensitive data exposure, access control |
| `style` | Project conventions per `AGENTS.md`, lint compliance, naming, dead code |
| `architecture` | Cross-spec invariants, design adherence, layering, premature abstraction, ADR drift |
| `docs` | Comments, READMEs, inline SPEC.md decisions and AGENTS.md updated to match the change |

The default fan-out (run when the skill does a full review) is:

```
business, tests, security, style
```

Architecture and docs are available via explicit `--persona` but
not in the default set. A future change may make the fan-out
project-configurable; v1 does not.

## CLI invocation

```sh
speccy review T-003 --persona security
```

Renders a prompt that includes:

- the relevant SPEC.md (full, including its `### Decisions` block)
- the task line from TASKS.md (with all prior notes)
- the diff for the task's claimed work
- `AGENTS.md`
- the persona's review-style guidance from
  `skills/shared/personas/reviewer-{persona}.md`

The reviewer sub-agent reads the prompt, performs the review, and
appends an inline note to the task in TASKS.md:

```markdown
- Review (security, blocking): bcrypt cost 10; policy requires >=12.
  See `src/auth/password.ts:14`.
```

Or:

```markdown
- Review (security, pass): No new auth surface. Password hashing
  routes through the existing module. OK.
```

## State transitions

The reviewer sub-agent **does not** flip the task's checkbox. That
would create a race when multiple persona reviewers run in
parallel. The main agent's `/speccy-review` skill flips state
after all persona reviews have completed for the task:

- All `pass` -> `[?]` becomes `[x]`.
- Any `blocking` -> `[?]` becomes `[ ]`, plus a `Retry:` note
  summarizing the blockers.

This puts state-mutation atomicity in one place (the orchestrating
skill) and keeps persona sub-agents to a single inline append per
review.

## Why personas live in skills, not CLI

The CLI cannot know what "security" means in this project. The
skill prompt does. By making personas markdown skill files, three
things become possible:

1. Add a new persona without changing the CLI.
2. Swap persona definitions when models improve.
3. Projects can override shipped personas in
   `.speccy/skills/personas/reviewer-security.md` -- the CLI
   prefers project-local over shipped.

---

# Amendment

Amendments are not a separate first-class artifact in v1. The
amendment story is a **skill concern** built from existing CLI
primitives.

## What happens when SPEC.md needs to change

The `/speccy-amend SPEC-001` skill orchestrates:

```sh
speccy plan SPEC-001         # renders "amend this SPEC.md" prompt
# Agent edits SPEC.md surgically (preserves what works)

speccy tasks SPEC-001        # renders "amend TASKS.md" prompt
                             # because TASKS.md already exists
# Agent edits TASKS.md surgically:
#   - keeps [x] tasks (already done) unless invalidated by changes
#   - keeps [~] / [?] tasks unless invalidated
#   - flips invalidated [x] tasks back to [ ] with "spec amended" note
#   - adds new tasks for new requirements
#   - removes tasks for dropped requirements

speccy tasks SPEC-001 --commit
# Records new spec_hash + timestamp into TASKS.md frontmatter
```

The cleverness lives in the skill prompt templates:

- `prompts/plan-amend.md` instructs: "do not rewrite the spec;
  produce a minimal diff against existing SPEC.md."
- `prompts/tasks-amend.md` instructs: "preserve [x] tasks unless
  the spec change invalidates them; add a 'spec amended' note next
  to flipped tasks."

The CLI renders these context-aware prompts based on whether the
target file exists. No `speccy amend` command; the existing
commands are sufficient.

## Lineage

Speccy does not maintain an amendment registry. Two mechanisms
cover the lineage need:

1. **`## Changelog` table in SPEC.md.** Curated, prose-summarized
   history of material edits. Each row records date, author, and
   summary. This is what gets loaded into review and amendment
   prompts so future agents understand recent intent shifts.
2. **Git history.** Authoritative literal lineage. `git log SPEC.md`
   and `git log TASKS.md` show every change ever made.

The previous design's `amendments/` folder and `AMD-NNN` IDs were
ceremony that duplicated git's job. The Changelog table replaces
both at far lower cost.

---

# Spec Staleness Detection

When SPEC.md is edited mid-loop (between Phase 2 and Phase 5),
TASKS.md may no longer reflect the current spec. Speccy detects
this two ways:

1. **Content hash.** TASKS.md frontmatter's
   `spec_hash_at_generation` stores the sha256 of SPEC.md at the
   time TASKS.md was generated. `speccy status` recomputes the
   current hash and compares.
2. **Modification time.** `speccy status` also compares SPEC.md
   mtime against TASKS.md mtime as a fallback signal.

If either drifts, `speccy status` reports:

```text
SPEC-001: TASKS.md may be stale.
  Hash drift: SPEC.md sha256 changed since tasks were generated.
  Run /speccy-amend to reconcile.
```

This is a soft warning. The user / skill decides what to do. No
gate fires.

---

# Skills / Harness Layer

Speccy v1 ships official skill packs alongside the CLI. They are
not optional polish; they are how the system becomes usable end-
to-end without each project inventing its own integration.

## What ships in v1

```
skills/
  claude-code/
    speccy-init.md
    speccy-plan.md
    speccy-tasks.md
    speccy-work.md           Implementation loop
    speccy-review.md         Review loop
    speccy-amend.md          SPEC.md + TASKS.md surgical edit
    speccy-ship.md           Run report, open PR
  codex/
    (parallel structure)
  shared/
    personas/
      planner.md
      implementer.md
      reviewer-business.md
      reviewer-tests.md
      reviewer-security.md
      reviewer-style.md
      reviewer-architecture.md
      reviewer-docs.md
    prompts/
      plan-greenfield.md
      plan-amend.md
      tasks-generate.md
      tasks-amend.md
      implementer.md
      reviewer-<persona>.md
      report.md
```

## `speccy init` host detection

```sh
speccy init                  # detects host from environment
speccy init --host claude-code
speccy init --host codex
```

Init copies `skills/<host>/*` into the host-native location:

- Claude Code: `.claude/commands/*.md`
- Codex: `.codex/skills/*.md`

The user gets immediate access to `/speccy-plan`, `/speccy-work`,
`/speccy-review`, `/speccy-amend`, `/speccy-ship` in their host
without any further setup.

## Workflow recipes

Each top-level skill is a recipe:

- `/speccy-init` -- bootstrap the project
- `/speccy-plan` -- Phase 1 (Vision -> SPEC)
- `/speccy-tasks` -- Phase 2 (SPEC -> TASKS)
- `/speccy-work` -- Phase 3 (impl loop)
- `/speccy-review` -- Phase 4 (review loop)
- `/speccy-amend` -- Mid-loop spec change
- `/speccy-ship` -- Phase 5 (report + PR)

A typical full-loop session in Claude Code looks like:

```
/speccy-plan
[agent writes SPEC.md + spec.toml]

/speccy-tasks SPEC-001
[agent writes TASKS.md, then speccy tasks --commit]

/speccy-work SPEC-001
[main agent loops, spawning impl sub-agents until all tasks are [?]]

/speccy-review SPEC-001
[main agent loops, spawning review sub-agents per persona per task;
 flips state; loop alternates with /speccy-work until all tasks [x]]

/speccy-ship SPEC-001
[agent writes REPORT.md, opens PR]
```

The CLI is invoked many times during this; the skill knows when.

## Persona definitions

Each persona file is a markdown skill describing:

- the role (one paragraph)
- review focus areas (bullet list)
- what to look for that is easy to miss
- format of the inline note to append
- a worked example

Example skeleton for `reviewer-security.md`:

```markdown
# Reviewer Persona: Security

## Role
You are an adversarial security reviewer for one task in one spec.
You read the SPEC.md, the task's diff, and the implementer notes.
You produce a single inline note appended to the task in TASKS.md.

## Focus
- Authentication and authorization boundaries
- Input validation and injection vectors
- Secret handling, credential storage, token lifecycle
- Sensitive data exposure in logs, errors, responses
- Race conditions affecting authorization
- Cryptographic primitives and parameter choices

## What to look for that's easy to miss
- Plaintext leaks in logs even when storage is hashed
- Authorization checks that pass before resource lookup (TOCTOU)
- Error messages that disclose user existence
- Missing rate limiting on auth endpoints

## Inline note format
Append exactly one bullet to the task:

- Review (security, pass | blocking): <one-line summary>.
  <optional file:line refs and details>.

## Example
- Review (security, blocking): bcrypt cost 10; policy requires
  >=12. See `src/auth/password.ts:14`.
```

These files are the durable surface where review intelligence
lives. They are upgradeable as models improve; the CLI is not.

---

# JSON Interfaces

Two commands have stable JSON contracts.

## `speccy status --json`

```json
{
  "schema_version": 1,
  "repo_sha": "abc123",
  "specs": [
    {
      "id": "SPEC-001",
      "slug": "user-signup",
      "title": "User signup",
      "status": "in-progress",
      "supersedes": [],
      "superseded_by": [],
      "tasks": {
        "open": 3,
        "in_progress": 1,
        "awaiting_review": 0,
        "done": 2
      },
      "stale": false,
      "stale_reasons": [],
      "open_questions": 1,
      "lint": {
        "errors": [],
        "warnings": ["VAL-001: CHK-003 missing 'proves' field"]
      }
    }
  ]
}
```

By default, `speccy status` shows only specs with `status: in-progress`
plus any with stale evidence or lint errors regardless of status.
Specs with `status: implemented`, `dropped`, or `superseded` are
excluded from the default view but always present in `--json` output
so harnesses can filter as needed.

The `superseded_by` field is **computed** at query time by walking
every parsed SPEC.md's `frontmatter.supersedes` and inverting the
relation. It does not appear in any SPEC.md frontmatter on disk.

## `speccy next --json`

When the next actionable thing is implementation:

```json
{
  "schema_version": 1,
  "kind": "implement",
  "spec": "SPEC-001",
  "task": "T-003",
  "task_line": "Implement POST /api/signup",
  "covers": ["REQ-001"],
  "suggested_files": ["src/auth/signup.ts", "tests/auth/signup.spec.ts"],
  "prompt_command": "speccy implement T-003"
}
```

When the next actionable thing is review:

```json
{
  "schema_version": 1,
  "kind": "review",
  "spec": "SPEC-001",
  "task": "T-003",
  "task_line": "Implement POST /api/signup",
  "personas": ["business", "tests", "security", "style"],
  "prompt_command_template": "speccy review T-003 --persona {persona}"
}
```

The skill iterates over `personas` and invokes
`prompt_command_template` for each.

When all tasks are `[x]` and the report is pending:

```json
{
  "schema_version": 1,
  "kind": "report",
  "spec": "SPEC-001",
  "prompt_command": "speccy report SPEC-001"
}
```

When nothing is actionable but state is incomplete (e.g. all tasks
`[~]` claimed by other sessions):

```json
{
  "schema_version": 1,
  "kind": "blocked",
  "reason": "all open tasks are claimed by other sessions"
}
```

These are the only two contracts a harness needs. Everything else
is text output to humans.

---

# Lint Codes

Speccy emits a small set of deterministic lint codes. None depend
on LLM judgment. All have stable prefixes (`SPC-` for spec
structure, `REQ-` for requirements, `VAL-` for check definitions,
`TSK-` for task structure).

```text
SPC-001  spec.toml missing required field
SPC-002  SPEC.md heading REQ-NNN not found in spec.toml
SPC-003  spec.toml requirement REQ-NNN has no matching SPEC.md heading
SPC-004  SPEC.md frontmatter missing required field (id/slug/title/status/created)
SPC-005  SPEC.md frontmatter status value is not one of: in-progress,
         implemented, dropped, superseded
SPC-006  status = superseded but no other spec in the workspace
         declares `supersedes` pointing to this spec
SPC-007  status = implemented but tasks are not all [x] (informational)

REQ-001  Requirement has no covering check
REQ-002  Requirement's check IDs reference non-existent checks

VAL-001  Check missing 'proves' field
VAL-002  Check kind 'test' or 'command' missing 'command' field
VAL-003  Check kind 'manual' missing 'prompt' field
VAL-004  Check command is a known no-op (e.g. 'true', ':', 'exit 0')

TSK-001  TASKS.md task references non-existent REQ ID
TSK-002  TASKS.md task ID format invalid (expected T-NNN)
TSK-003  Spec hash mismatch: TASKS.md may be stale relative to SPEC.md
TSK-004  TASKS.md frontmatter missing required field
         (spec/spec_hash_at_generation/generated_at)

QST-001  SPEC.md has unchecked open question (soft signal)

JSON-001 status --json schema version mismatch (informational)
```

VAL-004 (no-op command) is the **only** lint code that flags
"weak proof" mechanically. Everything else about proof quality
goes to review.

Lint codes are stable: changing a code's meaning between minor
versions is a breaking change. Adding new codes is fine.

---

# What We Deliberately Don't Do

These are not v1 features. Each was considered and rejected.

| Cut | Reason |
|---|---|
| Capability map (`CAP-NNN`) | Specs are flat. Grouping via tags or filesystem is sufficient. |
| Milestone state machine | Replaced by tag-based releases + a checklist file if the project wants one. |
| Release readiness as separate gate | Same: git tag + checklist. Not first-class. |
| Decision (ADR) as a separate noun | Decisions live inline in SPEC.md as `### Decisions` blocks. No separate folder, no CLI command, no lifecycle machinery. |
| Amendment as TOML | Replaced by SPEC.md frontmatter `status` + `## Changelog` table. |
| Assumption / Constraint / Invariant / Question as TOML | All collapse into SPEC.md narrative sections. |
| Scenario as separate noun | Folded into `Requirement.done_when` prose. |
| Per-requirement delta markers (`[ADDED]`/`[MODIFIED]`/`[REMOVED]`) | SPEC.md frontmatter `status` + `supersedes` + `## Changelog` table cover lifecycle. |
| Archive folder for completed specs | Frontmatter `status` is the indicator. Filesystem reorganization adds friction with no information gain. |
| Task `writes` globs and scope enforcement | LLMs declare them wrong; enforcement was net-negative. |
| Claim files / leases | No locking. Markdown checkbox + session marker is enough. |
| TDD exception registry | Don't gate on TDD. Review's job. |
| `critical` flag on requirements | All requirements equal. |
| `origin` field | Brownfield context is the planner skill's responsibility, not a TOML field. |
| Check `inputs` and freshness hashing | Wrong inputs poison the model worse than no inputs. CI runs checks. |
| Check evidence records | CI captures execution; no need to commit. |
| `--strict` flag | Opinionated, not configurable. |
| Validation kind enum | Free-form string with conventions. |
| Solo review policy toggle | Different sessions / personas suffice. |
| In-process LLM calls | CLI renders prompts; never invokes models. |
| Worktree orchestration | Harness concern. |
| Distributed locks | Harness concern. |
| External tracker sync | Harness concern. |
| Plugin ecosystem | Premature. |
| Identity provider integration | Premature. |
| Runtime telemetry | Out of scope. |
| Mutation testing | Out of scope. |
| Semantic dependency analysis | Out of scope. |
| Bad-test detection beyond no-op commands | Review owns this. |

The point is not that these features are wrong. The point is that
v1 is small enough to trust.

---

# Comparison to Peers

Brief positioning. None of these are wrong; Speccy borrows from
each.

| Tool | Strength Speccy borrows | Speccy diverges by |
|---|---|---|
| **OpenSpec** | Lightweight change proposals, low-ceremony | Smaller surface; more focused on greenfield loop |
| **Spec Kit** | `/specify` `/plan` `/tasks` opinionated flow, PRD-shaped templates | Speccy adds adversarial review loop, multi-persona |
| **Kiro** | Steering files for durable agent context | We use `AGENTS.md` + `skills/`; no IDE coupling |
| **GSD** | Milestone-driven verification, autonomy levels | Speccy drops formal milestones; verification stays |
| **BMAD** | Phased context engineering, agent personas | Personas in skills, not built-ins; phases match |
| **Cursor rules** | Rule-folder layering for persistent context | `AGENTS.md` + `.claude/rules/` adopted directly |

Speccy's distinctive bet: **multi-persona adversarial review run by
the same agent host that did the implementation**, with state and
notes living in markdown the same agent will read in the next
iteration. That is where drift gets caught in this system.

---

# Threat Model

V1 makes these failures loud:

- Spec has no requirements
- Requirement has no covering check
- Check has no proof claim
- Check command is a known no-op
- TASKS.md references requirements that don't exist
- TASKS.md is stale relative to SPEC.md (hash or mtime drift)
- Open question in SPEC.md is unchecked
- Reviewer persona returns `blocking`
- Task is `[?]` but at least one persona review is missing

V1 intentionally does not catch:

- Semantic correctness of any check
- Whether the implementation actually meets `done_when`
- Whether the reviewer was thorough
- Whether the agent invented assumptions in implementer notes
- Whether the PR description matches REPORT.md
- Whether the project will work end-to-end in production
- Architecture drift across specs

Those failures are review's job, the human's job, or out of scope
for a feedback engine.

---

# Operational Details

Implementation choices and conventions. Each was considered and
locked in so implementers don't rediscover them.

## Parsing stack

| Concern | Crate | Version pin |
|---|---|---|
| Markdown body | `comrak` (CommonMark + GFM tables) | latest stable |
| YAML deserialization | `serde-saphyr` (serde adapter over `saphyr-parser`) | exact `0.0.x` |
| TOML deserialization | `toml` | latest stable |
| Targeted regex | `regex` (only for ID extraction from heading text) | latest stable |

**YAML choice rationale.** The Rust YAML ecosystem is in flux as of
May 2026: dtolnay's `serde_yaml` is deprecated, and the most common
"fork" `serde_yml` was archived in September 2025 with RUSTSEC-2025-0068
(unsound, panics on malformed input). `serde-saphyr` is the live
choice — built on `saphyr-parser` (the actively-developed pure-Rust
YAML 1.2 parser, successor to `yaml-rust`), with direct-to-struct
deserialization (no `Value` AST roundtrip), panic-free on malformed
input, and configurable resource budgets that defend against
Billion-Laughs attacks. The first-party `saphyr-serde` is announced
but not yet released; `serde-saphyr` is the practical choice today.

Caveat: `serde-saphyr` is `0.0.x` (pre-`0.1.0`). Pin exact version and
expect a minor refactor when it stabilizes. Acceptable tradeoff vs.
shipping a CI gatekeeper on top of an actively-unsafe crate.

**Frontmatter extraction is DIY.** The `---` fence parser is ~4 lines
of string slicing returning `&str` slices for the YAML chunk and the
markdown body. `gray_matter` was considered and rejected: it would
pull in `yaml-rust2` transitively for zero gain over a tiny custom
splitter, and tying frontmatter parsing to one specific YAML crate
makes future migration harder.

**Pure-regex parsing was considered and rejected.** SPEC.md contains
fenced code blocks with example markdown (this document does too),
and regex cannot reliably skip those contexts. The 4-crate cost is
worth the robustness.

TASKS.md is parsed by walking the comrak event stream for list
items, detecting the leading checkbox glyph (`[ ]` / `[~]` / `[?]`
/ `[x]`), then using a small regex on the item's inline content to
extract the `T-NNN` ID from a strong/bold span. Sub-list items are
notes attached to the parent task. No regex for structure; regex
only for ID extraction from already-isolated heading or strong-text
nodes.

## Spec ID allocation

`speccy plan` (creating a new spec) and `speccy spec <slug>` scan
`specs/` for the maximum `NNNN-` prefix and increment. Gaps left
by dropped specs are not recycled.

## `speccy init` behavior

Refuses to run if `.speccy/` already exists, unless `--force` is
passed. Before doing anything destructive, prints the list of
files that would be created or overwritten.

Host detection for skill-pack copy:

1. `--host <name>` flag if passed
2. Presence of `.claude/` -> Claude Code
3. Presence of `.codex/` -> Codex
4. Presence of `.cursor/` -> Cursor
5. Fall back to `claude-code` and print a warning

The user can re-run `speccy init --host <other> --force` to swap.

## Section heading discovery in SPEC.md

Case-insensitive exact match. `## Open Questions`, `## open
questions`, `## OPEN QUESTIONS` all match. Hyphens and spaces in
heading text are treated equivalently for matching. Unknown
headings are ignored, not flagged.

## Frontmatter dates

- `created`: ISO 8601 date (`YYYY-MM-DD`)
- `generated_at`, `recorded_at`: ISO 8601 datetime in UTC
  (`YYYY-MM-DDTHH:MM:SSZ`)

Missing optional frontmatter fields are treated identically to
empty lists or null. The parser does not distinguish.

## Persona file resolution

Lookup order:

1. Project-local: `.speccy/skills/personas/reviewer-X.md`
2. Shipped (copied at `speccy init` time): `.claude/commands/`
   or equivalent host location

If the project-local override exists but is malformed, lint warns
and the CLI falls through to the shipped version.

## Subdirectory naming

Spec folders: `NNNN-slug-with-hyphens`. Slug is lowercase ASCII
only. Lint warns on uppercase or non-ASCII. Mismatch between
`frontmatter.slug` and the actual folder name is a lint error.

## Schema version

`spec.toml` requires `schema_version = 1`. SPEC.md / TASKS.md /
REPORT.md frontmatter implicitly target schema version 1; no
declaration needed. The CLI rejects unknown `schema_version`
values with a clear error.

## `speccy verify` exit code

Binary. `0` if all checks pass and lint is clean; `1` otherwise.
Detailed breakdown is available via `speccy verify --json`. CI
scripts only check the exit code; downstream tooling parses the
JSON if it needs structured failure info.

## `speccy next` priority

When multiple specs have actionable work:

1. Lowest spec ID first.
2. Within a spec, prefer `[?]` review-ready tasks over `[ ]` open
   tasks (so reviews don't accumulate).
3. `--kind implement` or `--kind review` overrides the within-spec
   preference and filters to the requested kind across all specs.

## `speccy check` execution

Serial. Each check runs through the project's default shell
(`sh -c` on Unix, `cmd /c` on Windows). No timeout — the user
Ctrl+Cs if something hangs. Working directory is the project root
(the directory containing `.speccy/`). Stdout and stderr stream
live to the terminal. Exit code is the first non-zero exit code
encountered, or `0` if all checks pass.

## `speccy review` diff scoping

Reviewer prompt includes the diff between the working tree and
`HEAD`. This is what the implementer just produced, including
uncommitted edits — the natural moment of review.

If the working tree is clean (e.g. the implementer already
committed), the diff is taken between `HEAD` and `HEAD~1`. If that
also yields nothing relevant, the prompt notes "no diff available;
review based on SPEC.md and task notes alone."

## Prompt context budget

When a rendered prompt approaches the host model's context limit,
sections are dropped in this order until the prompt fits:

1. `## Notes` from SPEC.md (drop first)
2. Answered `Open questions` entries (keep unchecked ones)
3. SPEC.md `## Changelog` rows older than the 5 most recent
4. TASKS.md review notes older than the 3 most recent per task
5. Other specs' summaries (if multi-spec context was being shown)

If a prompt still doesn't fit after these drops, the CLI prints a
warning and emits the prompt anyway; the host model handles
truncation. v1 does not implement smarter retrieval.

---

# Implementation Sequence

In this order:

1. Artifact parser: `spec.toml`, `speccy.toml`, SPEC.md (YAML
   frontmatter + heading extraction + Changelog table), TASKS.md
   (YAML frontmatter + state extraction), REPORT.md frontmatter
2. `speccy init` -- scaffold + host skill copy
3. Lint engine with the codes listed above
4. `speccy status` (text + `--json`)
5. `speccy plan` (greenfield prompt rendering)
6. `speccy tasks` (initial + amendment prompts)
7. `speccy tasks --commit` (record spec_hash)
8. `speccy next` (text + `--json`)
9. `speccy implement` prompt rendering
10. `speccy review` with persona rendering
11. `speccy check` (executable, manual, no-op detection)
12. `speccy report` prompt rendering
13. `speccy verify` (lint + check)
14. Skill packs: Claude Code, Codex, shared personas
15. Worked example: dogfood Speccy's own development in
    `.speccy/specs/` once enough of the above lands

The implementation sequence is itself the first project we should
drive through Speccy. By the time step 13 lands, Speccy's own
SPECs should exist for steps 1-12.

---

# Success Criteria

Speccy v1 is complete enough when:

- A solo developer can run `speccy init` in a fresh repo and reach
  their first green check via the shipped skills without inventing
  process.
- The same developer can run `speccy init` in an existing repo at
  any point in its life and use Speccy productively on a small
  slice without reverse-engineering the whole codebase.
- An AI coding agent driven by the shipped skills can complete a
  full Plan -> Tasks -> Impl -> Review -> Report loop on a non-
  trivial spec without needing the human to chain commands manually.
- Reviewer personas catch at least one class of drift per review
  run on representative work (the proof here is the dogfooded
  Speccy itself).
- `speccy verify` is a reliable CI gate: passes when the proof
  shape is intact, fails when it isn't, never flakes on its own
  state.
- Speccy drives its own development. The repo contains
  `.speccy/specs/` for the implementation sequence above, with
  passing checks and review records.

Speccy v1 does not need to autonomously ship software. It needs
to make autonomous software construction less blind, and to make
the next greenfield project I (or anyone using it) build feel
qualitatively different from "ask the agent to do everything and
hope."

---

# Long-Term Vision

Speccy aims to become the **deterministic feedback substrate** that
multi-agent harnesses can build on. Future layers (not v1):

- Concurrent task pickup with file-locking or task queues
- Worktree orchestration per task
- Cross-spec dependency reasoning
- Project-level dashboard / kanban UI consuming `status --json`
- Production telemetry feedback into spec state
- Cross-repository orchestration

The foundation should remain unchanged across these layers:

> Explicit, inspectable, feedback-only contracts between intent
> and shipped behavior, with adversarial multi-persona review as
> the primary drift-detection mechanism.
