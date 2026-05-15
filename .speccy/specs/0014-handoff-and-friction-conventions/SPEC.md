---
id: SPEC-0014
slug: handoff-and-friction-conventions
title: Handoff template + friction-to-skill-update conventions in shipped skills
status: implemented
created: 2026-05-13
---

# SPEC-0014: Handoff template + friction-to-skill-update conventions

## Summary

SPEC-0014 tightens two conventions inside the shipped skill bundle so
speccy plays well underneath a long-horizon multi-agent harness:

1. The implementer-note template names six structured handoff fields
   (completed, undone, commands run, exit codes, discovered issues,
   procedural compliance) rather than the current freeform prose.
2. The implementer prompt + persona document the
   **friction-to-skill-update** pattern: when a worker hits recurring
   friction (wrong package manager, missing env var, undocumented step),
   they update the relevant skill markdown before flipping `[~] -> [?]`,
   so the next worker inherits the fix. The report prompt surfaces any
   such updates in a `## Skill updates` section in REPORT.md.

No new Rust code. No new CLI surface. The deliverable is markdown
content under `skills/shared/`, plus a paragraph in the project's own
`AGENTS.md` to dogfood the pattern, plus three new content-shape
tests under `speccy-cli/tests/skill_packs.rs`.

The motivation is external: the Factory AI "missions" architecture
(Xiao, 2026-05) identifies structured handoffs and a worker-editable
skills layer as the two strongest predictors of long-horizon
multi-agent run quality. Speccy already exposes the substrate; this
spec teaches the shipped skills to use it.

## Goals

- Implementer notes that downstream readers (reviewers, future
  implementers, harness telemetry) can parse without judgement calls
  about prose shape.
- A documented, dogfooded loop where implementers update skill files
  in-flight, and REPORT.md captures which ones moved.
- Content-shape tests that fail loudly if the conventions drift out of
  the shipped bundle.

## Non-goals

- No CLI surface change. The behaviour lives entirely in shipped
  markdown.
- No new lint codes. The article-recommended patterns are skill-pack
  conventions, not deterministic checks.
- No formal handoff schema (TOML, JSON Schema). The handoff fields are
  bullet conventions inside the implementer's TASKS.md note. A future
  spec may formalise the shape if a harness needs it; v1 does not.
- No automation of skill rewrites. Workers edit skill markdown the
  same way they edit any other file. Speccy does not detect, validate,
  or replay these edits.
- No backporting to closed specs. SPEC-0001..0013 keep their existing
  implementer notes; the new convention applies prospectively.

## User stories

- As an implementer-agent picking up `T-NNN`, I want the prompt to
  spell out exactly which fields belong in my closing note so I don't
  have to invent structure on the fly.
- As a reviewer-agent reading a `[?]` task, I want the implementer's
  note to tell me what was attempted, what shipped, and what was
  deferred without re-reading the diff.
- As an implementer-agent hitting friction (`pnpm` vs `npm` in this
  repo; a database URL that defaults to the wrong host), I want
  explicit permission and a documented place to update the skill so
  the next implementer doesn't re-discover it.
- As an orchestrator reading REPORT.md three months later, I want a
  single section that lists which skill files moved during the run.

## Requirements

### REQ-001: Six-field handoff template in implementer prompt

The shipped implementer prompt (`skills/shared/prompts/implementer.md`)
embeds a structured handoff template enumerating six fields.

**Done when:**
- `skills/shared/prompts/implementer.md` contains a fenced markdown
  template block (```` ```markdown ```` fence) showing the
  implementer-note shape with all six handoff fields, each on its own
  sub-bullet under the implementer note.
- The six field labels appear verbatim in the template:
  `Completed`, `Undone`, `Commands run`, `Exit codes`,
  `Discovered issues`, `Procedural compliance`.
- The prompt's `## Your task` step that today says "Append one
  implementer note ..." is rewritten to reference the template and
  require all six fields, with the convention that empty fields are
  written as `- Completed: (none)` rather than omitted.

**Behavior:**
- A content-shape test loads the template from the embedded bundle
  and asserts all six field labels are present inside a fenced code
  block.
- A second test asserts the `## Your task` body references the
  template and instructs filling every field.

**Covered by:** CHK-001, CHK-002

### REQ-002: Friction-to-skill-update pattern documented

The shipped implementer prompt and implementer persona document the
loop: when a worker hits recurring friction, they update the relevant
skill markdown before flipping `[~] -> [?]`, and they note the file
they touched in the implementer note under `Procedural compliance`.

**Done when:**
- `skills/shared/prompts/implementer.md` contains a section
  (heading: `## When you hit friction`) that names the pattern,
  gives one worked example (e.g. "the implementer prompt told you to
  run `npm test` but the project uses `pnpm`"), and instructs the
  implementer to (a) update the relevant file under `skills/` and
  (b) mention the file in `Procedural compliance`.
- `skills/shared/personas/implementer.md` references the same pattern
  in `## What to consider` with a one-bullet pointer back to the
  prompt section.
- The project's own `AGENTS.md` adds a short paragraph under
  `## Conventions for AI agents specifically` documenting the pattern
  for speccy's own contributors, since speccy dogfoods speccy.

**Behavior:**
- A content test asserts the prompt contains a heading named exactly
  `## When you hit friction` and at least one fenced example referring
  to a `skills/` path.
- A content test asserts the persona file references the friction
  pattern (string match on the friction heading slug or a recognisable
  phrase) and points back to the prompt.
- A content test asserts `AGENTS.md` contains a recognisable phrase
  about the friction-to-skill-update loop (substring match against an
  invariant sentence shipped in the spec).

**Covered by:** CHK-003, CHK-004, CHK-005

### REQ-003: Skill-update surfacing in REPORT.md

The shipped report prompt (`skills/shared/prompts/report.md`)
instructs the report-writing agent to add a `## Skill updates`
section listing any `skills/**` files touched during the run.

**Done when:**
- `skills/shared/prompts/report.md` is amended so that the section
  enumeration in `2.` includes `## Skill updates` (after
  `## Out-of-scope items absorbed`, before
  `## Deferred / known limitations`).
- The report prompt instructs the agent to: list each modified
  `skills/**` file (one bullet per file) with a one-line summary of
  what changed and which task surfaced the friction; if no skill files
  were touched, write `(none)` rather than omitting the section.

**Behavior:**
- A content test loads the report prompt and asserts the literal
  string `## Skill updates` appears in the body and that the
  `## Out-of-scope` / `## Skill updates` / `## Deferred` ordering is
  preserved.

**Covered by:** CHK-006

## Design

### Approach

Pure content edit. The shipped bundle re-embeds at the next build via
`include_dir!` (machinery already in SPEC-0002). The new tests live
alongside the existing skill-pack content-shape tests in
`speccy-cli/tests/skill_packs.rs` and follow the same pattern: load a
file from `bundle_dir`, assert literal substrings or fenced-block
shape.

The implementer-note convention applies only to notes written **after
this spec lands**. Existing TASKS.md notes are not retrofitted.
Reviewer personas continue to consume the old freeform notes correctly
because the new template is a superset of what they already expect.

### Decisions

#### DEC-001: Six fields verbatim, not paraphrased

**Status:** Accepted
**Context:** The article enumerates the six handoff fields. The
template could use any synonyms (e.g. "Done" vs "Completed",
"Commands" vs "Commands run"), but a stable contract is cheaper than
a flexible one for a downstream harness that may grep these labels.
**Decision:** The six labels are fixed strings:
`Completed`, `Undone`, `Commands run`, `Exit codes`,
`Discovered issues`, `Procedural compliance`. The content-shape test
asserts the exact strings.
**Alternatives:**
- Loose synonyms with a reviewer-tests persona enforcement. Rejected:
  pushes the contract into a place review can't reliably enforce.
**Consequences:** Renaming any field is a breaking change to the
template (caught by CHK-001/002 and any consuming harness). Worth the
stability for downstream tooling.

#### DEC-002: Skill-update surfacing lives in REPORT.md, not in TASKS.md notes

**Status:** Accepted
**Context:** Skill edits could be recorded per-task in the
implementer note (under `Procedural compliance`), per-spec in
REPORT.md, or both. Per-task notes already capture *which task
surfaced* the friction; REPORT.md is the natural rollup view for an
orchestrator reading after the loop closes.
**Decision:** Both. `Procedural compliance` in the implementer note
records the friction discovery and the file touched at task scope.
`## Skill updates` in REPORT.md aggregates across all tasks for the
spec.
**Alternatives:**
- Only per-task. Rejected: an orchestrator reading 40 tasks shouldn't
  have to grep every implementer note to learn which skills moved.
- Only in REPORT.md. Rejected: loses the link from a specific friction
  point to the file that fixed it.
**Consequences:** The report prompt must instruct the agent to derive
the skill-updates list from inline notes (already in context) +
`git diff --name-only -- skills/` if available. The prompt change is
minor; the orchestrating skill (`/speccy:ship`) already has git access.

#### DEC-003: AGENTS.md edit is in scope; user-project AGENTS.md is not

**Status:** Accepted
**Context:** `AGENTS.md` is project-specific. `speccy init` does not
copy AGENTS.md into a user's project. Documenting the pattern in
speccy's own AGENTS.md is dogfooding; it doesn't propagate to users
of speccy unless they read it.
**Decision:** This spec edits speccy's own AGENTS.md to document the
pattern for speccy's contributors. The shipped implementer prompt is
the propagation surface to users: when they run `speccy init`, the
prompt arrives in their `.claude/commands/` and the pattern arrives
with it.
**Alternatives:**
- Have `speccy init` template an AGENTS.md fragment into the user's
  project. Rejected: AGENTS.md is project-curated; speccy shouldn't
  inject into it.
**Consequences:** REQ-002 splits naturally into "prompt + persona"
(propagates) and "AGENTS.md" (speccy-only dogfooding).

#### DEC-004: No new CLI surface, no new lint codes

**Status:** Accepted
**Context:** The recommendation set could have spawned a
`speccy friction-log` command, a `SKL-NNN` lint family, or a TOML
schema for handoffs. Each would be net-negative against the "stay
small" principle.
**Decision:** Content-only spec. CLI surface unchanged. Lint codes
unchanged. The deliverable is markdown and three content tests.
**Alternatives:** See above.
**Consequences:** Downstream harnesses that want to grep handoffs do
so on stable string labels (DEC-001). That's the contract.

### Data changes

No Rust code. Modified markdown files:

- `skills/shared/prompts/implementer.md` (REQ-001, REQ-002)
- `skills/shared/prompts/report.md` (REQ-003)
- `skills/shared/personas/implementer.md` (REQ-002)
- `AGENTS.md` (REQ-002)

New test functions in `speccy-cli/tests/skill_packs.rs`:

- `implementer_prompt_handoff_template`
- `implementer_prompt_handoff_referenced_in_task_steps`
- `implementer_prompt_friction_section`
- `implementer_persona_friction_reference`
- `agents_md_friction_paragraph`
- `report_prompt_skill_updates_section`

### Migration / rollback

Content-only. Rollback via `git revert`. No data migrations. Depends
on SPEC-0002 (bundle copy), SPEC-0008 (implementer prompt loader),
SPEC-0011 (report prompt loader), SPEC-0013 (existing content-shape
tests harness).

## Open questions

- [ ] Should the `Procedural compliance` field be split into two
  (`Procedural compliance` for general workflow adherence vs
  `Skill updates` for the friction loop specifically)? Leaning toward
  no: one field, with skill paths called out when relevant, keeps the
  bullet count at six. Revisit if implementers consistently leave
  `Procedural compliance` empty.
- [ ] Should the report prompt's `## Skill updates` section be
  promoted to required REPORT.md frontmatter (e.g. a
  `skills_updated: ["skills/shared/prompts/implementer.md"]` list)?
  Defer: prose section first, formal field only if a harness needs it.
- [ ] Should there be a corresponding section in the planner prompt
  encouraging the planner to update skills when it discovers something
  during planning? Probably yes eventually; out of scope here.

## Assumptions

- The embedded bundle from SPEC-0002 re-includes any new content in
  `skills/shared/prompts/` and `skills/shared/personas/`
  automatically. (No `include_dir!` glob change required.)
- SPEC-0013's content-shape test harness (`bundle_dir`,
  `read_bundle_file`, `panic_with_test_message`) is reusable for these
  three new tests.
- AGENTS.md is accessible to the test process as a file at the
  workspace root; the test reads it via `include_str!("../../AGENTS.md")`
  or `fs_err::read_to_string(env!("CARGO_MANIFEST_DIR") + ...)`.
  Either pattern works; implementer picks one.
- Reviewers will not retroactively re-review SPEC-0001..0013 against
  the new convention.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-13 | human/kevin  | Initial draft after reviewing Factory AI "missions" architecture against speccy's design. |

## Notes

The motivation document is
`https://kevinxiao.ca/blog/long-running-multi-agent-orchestration-via-missions`.
Two of the article's seven takeaways are directly addressable inside
shipped skills without CLI changes:

- Takeaway 6: "Treat structured handoffs as first-class artifacts."
  REQ-001 + REQ-003 land this.
- Takeaway 5: "Make skills continuously editable by workers."
  REQ-002 + REQ-003 land this.

The other five takeaways either already match speccy's stance
(prompt-centric orchestration, separate writer from verifier,
externalised success criteria, validation as binding constraint) or
are harness territory (serial-with-parallel execution discipline,
model-agnostic role assignment). Speccy stays small; the harness
above speccy gets richer.
