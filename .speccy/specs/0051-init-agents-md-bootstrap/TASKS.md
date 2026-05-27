---
spec: SPEC-0051
spec_hash_at_generation: 8a11c39fac5f9898b8ff16cc90af0ca9a22e1db22e7767b77a535ba8416e0166
generated_at: 2026-05-27T21:29:24Z
---
# Tasks: SPEC-0051 `/speccy-init` AGENTS.md bootstrap — seeds `## Speccy conventions` section and refactors north star Q&A to a brainstorm-style adaptive flow

<task id="T-001" state="completed" covers="REQ-001">
## Create canonical `## Speccy conventions` reference module

Author a new reference module at
`resources/modules/references/agents-md-speccy-conventions.md`
carrying the canonical body of the `## Speccy conventions` section
that `/speccy-init` will upsert into target repos' `AGENTS.md`.

The body opens with a one-line preamble making the upsert contract
visible — the line must name `/speccy-init` as the section's
manager and tell the reader that edits inside the section are
overwritten on re-run, with the corollary that project-specific
additions belong in a sibling section.

The body then carries five subsections in order:

1. **When to use which skill.** One-liner per shipped skill:
   `/speccy-init`, `/speccy-brainstorm`, `/speccy-plan`,
   `/speccy-amend`, `/speccy-decompose`, `/speccy-work`,
   `/speccy-review`, `/speccy-vet`, `/speccy-ship`,
   `/speccy-orchestrate`. Lift the trigger-phrase essence from each
   skill's existing `description:` frontmatter rather than inventing
   new wording.
2. **The dev loop.** Plan → Tasks → Impl → Review → Ship sequence
   plus a one-sentence pointer to the journal-file location for
   per-task `<implementer>` / `<review>` / `<blockers>` records
   (`.speccy/specs/NNNN-slug/journal/T-NNN.md`).
3. **Test hygiene.** Enumerate all five vacuous-test anti-patterns
   in language-neutral phrasing — no Rust idioms like `is_ok()` or
   `unwrap()`. The five patterns: substring-matching human-curated
   prose; copying production constants into the test as a hard-coded
   compare value; asserting only file existence or non-emptiness
   without any property of the content; mocking the function under
   test and asserting the mock was called; assertions so loose any
   input passes. Close the subsection with the "investigate flakes,
   don't retry until green" rule.
4. **Commit hygiene.** State the `Co-Authored-By` trailer
   expectation for AI commits and the preference for narrow,
   well-scoped commits over sprawling ones.
5. **CI gate (suggestion).** One paragraph noting `speccy verify`
   is designed to run as a CI gate that fails on broken proof shape
   and passes when intact. Frame as a suggestion the user wires up
   in whichever CI service they use; name multiple platforms in
   prose (GitHub Actions, GitLab CI, Jenkins, etc.) without
   shipping vendor-specific configuration.

The body must not carry language-specific examples or
platform-specific CI wiring. It must not carry any text that would
embarrass downstream Speccy users — no Speccy-internal references
to `resources/modules/`, no Speccy-repo SPEC IDs, no `just reeject`
shell commands (those are upstream-maintainer concerns the
downstream user never runs).

The module is consumed solely via
`{% include "modules/references/agents-md-speccy-conventions.md" %}`
from the `/speccy-init` skill body (added by T-002); it does not
need its own per-host wrapper under
`resources/agents/.<host>/speccy-references/`. The canonical body
lands in target repos as inline content inside `AGENTS.md`, not as
a separately ejected reference file.

<task-scenarios>
Given the working tree at HEAD after this task,
when a reviewer reads
`resources/modules/references/agents-md-speccy-conventions.md`,
then the file exists, opens with the upsert-contract preamble, and
carries the five named subsections in the documented order with
the content properties described above.

Given the same file,
when a reviewer searches the body for language-specific anti-pattern
names (`is_ok`, `unwrap`, `expect`, `Result<`),
then no match appears in the reference module body.

Given the same file,
when a reviewer searches the body for vendor-specific CI wiring
artifacts (YAML keys like `runs-on:`, `jobs:`, or filenames like
`.github/workflows/`),
then no match appears.

Given the same file,
when a reviewer searches the body for upstream-maintainer-only
content (`just reeject`, `resources/modules/`, `SPEC-0051`),
then no match appears.

Suggested files: `resources/modules/references/agents-md-speccy-conventions.md`
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-002 REQ-005">
## Extend `/speccy-init` skill body with `## Speccy conventions` upsert step and state-matrix description

Edit `resources/modules/phases/speccy-init.md` to add an explicit
upsert step for the `## Speccy conventions` section. The step runs
after the existing scaffolding and north-star phases and instructs
the agent to perform a deterministic upsert keyed on the heading
boundary.

Add an `{% include "modules/references/agents-md-speccy-conventions.md" %}`
directive inside the new step so the canonical body (T-001) is
expanded into the rendered prompt and made available verbatim to
the agent at execution time.

Document the upsert logic explicitly: when the `## Speccy
conventions` heading is absent from `AGENTS.md`, the agent appends
the canonical body (with heading) to the end of the file; when the
heading is present, the agent replaces everything from the heading
to the next top-level `##` heading (or end of file) with the
canonical body. The replace path runs on every invocation
regardless of whether the existing body matches the canonical body
byte-for-byte — there is no "section already current, skip"
optimization.

Document the AGENTS.md state matrix explicitly: north-star
(present / absent) × conventions (present / absent), with four
cells named and the action per cell specified. The two seeding
decisions (north-star and conventions) are made independently —
the skill body must instruct the agent that either section may
exist or not at invocation time, and neither outcome biases the
treatment of the other section.

Document that the missing-file path simply re-bootstraps from
scratch: when an operator runs `/speccy-init` against a repo where
`AGENTS.md` is missing entirely (whether on first init or after a
user deleted it post-init), the skill body must instruct the
agent to write a fresh `AGENTS.md` carrying both sections without
any warning, refusal, or regression-detection ceremony.

Frame the upsert step in the skill body using the heading boundary
as the delimiter: no HTML comment markers fence the region. The
preamble line from the canonical body (T-001) is what makes the
upsert contract visible to downstream readers.

<task-scenarios>
Given the working tree at HEAD after this task,
when a reviewer reads `resources/modules/phases/speccy-init.md`,
then the skill body carries an explicit step for upserting
`## Speccy conventions` after the scaffolding and north-star
phases, with the heading-boundary upsert rule documented and an
`{% include %}` directive expanding the canonical body from
`modules/references/agents-md-speccy-conventions.md`.

Given the same skill body,
when a reviewer searches for the state-matrix documentation,
then the four cells (north-star present/absent × conventions
present/absent) are enumerated with the action per cell specified,
and the body explicitly states the two seeding decisions are made
independently.

Given the same skill body,
when a reviewer searches for any branch that refuses, warns about,
or otherwise special-cases the "user deleted AGENTS.md after prior
init" state,
then no such branch is present — the missing-file path simply
re-bootstraps.

Given the same skill body,
when a reviewer searches for HTML comment markers fencing the
conventions section (`<!-- speccy:conventions:start -->` or
similar),
then no such markers appear; the heading boundary is the sole
upsert delimiter.

Suggested files: `resources/modules/phases/speccy-init.md`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-003 REQ-004">
## Refactor `/speccy-init` skill body's north-star Q&A into a brainstorm-style adaptive flow

Edit `resources/modules/phases/speccy-init.md` to replace today's
fixed seven-question script for the `## Product north star`
section with a brainstorm-style adaptive flow. The new flow
instructs the agent to first inspect the repo (README, manifest
files like `Cargo.toml` / `package.json` / `pyproject.toml`,
top-level source structure, any existing `AGENTS.md` prose) and
gauge per-subsection legibility before deciding draft-vs-Socratic.

Walk the user through the section's subsections in template order:
opening prose (the project description and motivation paragraph),
`### Users`, `### V1.0 outcome`, `### Quality bar`,
`### Known unknowns`. For each subsection, the agent drafts from
repo context when the subsection's content is legible and presents
the draft for user confirmation; the agent falls back to
one-at-a-time Socratic Q&A (multi-choice questions where the
answer space is enumerable) when the content is not legible.

Specify a hard gate explicitly: the agent does not write
`## Product north star` to `AGENTS.md` until every subsection is
user-approved. User redirects iterate the relevant subsection
until approved.

The flow borrows brainstorm-style patterns inline — one question
at a time, multiple-choice when enumerable, draft-and-confirm,
hard gate before write. The skill body must not invoke
`/speccy-brainstorm` (or any other sub-skill) for the north-star
path; the patterns are reimplemented inline so `/speccy-init`
remains self-contained per DEC-002.

Preserve today's freeze-on-first-write behavior for the north-star
section. The skill body must retain the State C branch: when
`AGENTS.md` already contains a `## Product north star` heading
(from any prior init pass or hand-authored), the agent confirms
the existing content is current with the user and proceeds without
modification. The skill body must not introduce any path that
overwrites, diffs against, or re-elicits an existing
`## Product north star` section on re-run.

Add an explicit note in the skill body documenting the asymmetry
vs. the always-upsert conventions section (T-002): the north star
is freeze-on-first-write because the section captures
user-authored content, while the conventions section is canonical
boilerplate sourced from upstream and is safe to refresh on every
invocation.

<task-scenarios>
Given the working tree at HEAD after this task,
when a reviewer reads `resources/modules/phases/speccy-init.md`
for the north-star step,
then the fixed seven-question script ("What are we building, and
why does it matter?", "Who will use it?", etc.) is absent and the
adaptive flow described above is present.

Given the same skill body,
when a reviewer searches for the five subsections in template
order (opening prose, Users, V1.0 outcome, Quality bar, Known
unknowns),
then all five are named in the documented order as the iteration
sequence for approval.

Given the same skill body,
when a reviewer searches for the freeze-on-first-write branch,
then a State C path remains: when `## Product north star` is
already present, the agent skips Q&A entirely and proceeds
without modifying the section.

Given the same skill body,
when a reviewer searches for any invocation of `/speccy-brainstorm`
or other cross-skill dispatch from the north-star path,
then no such invocation appears — patterns are inlined.

Given the same skill body,
when a reviewer searches for the asymmetry note (north-star
freeze vs. conventions upsert),
then the note is present and explains the asymmetry as principled
based on content ownership.

Suggested files: `resources/modules/phases/speccy-init.md`
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-006">
## Update wrapper frontmatter descriptions and reeject host packs

Update the per-host `SKILL.md.tmpl` wrapper frontmatter for
`/speccy-init` so the `description:` field reflects the skill's
expanded scope — seeding both the `## Product north star` section
and the `## Speccy conventions` section into `AGENTS.md`. Today's
description mentions only the north star.

Files to edit:

- `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl`
- `resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl`

The updated description should preserve the existing trigger
phrases ("set up speccy", "init speccy", "add speccy to this
repo", "start a spec-driven workflow somewhere that has no
`.speccy/` yet") and the precondition ("Requires: no
preconditions") and Do NOT clause ("Do NOT trigger when
`.speccy/` already exists — use speccy-amend for SPEC edits or
speccy-plan for a new SPEC instead"). The added clause names the
two seeded sections.

After editing both wrappers, run `just reeject` to refresh the
ejected `.claude/`, `.agents/`, and `.codex/` host packs. The
reeject step renders the updated wrappers and expands the new
canonical body (from T-001) via the `{% include %}` directive
(added by T-002) into every ejected `speccy-init` skill body.

Verify the Rust CLI surface is untouched: this task must not
modify any file under `speccy-cli/src/`, `speccy-core/src/`, or
any other Rust source crate. The change is purely a template +
reeject refactor.

<task-scenarios>
Given the working tree at HEAD after this task,
when a reviewer reads
`resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl` and
`resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl`,
then both `description:` frontmatter fields name both seeded
sections (product north star and Speccy conventions) and retain
the existing trigger phrases, precondition, and Do NOT clause.

Given the same working tree after `just reeject` has run,
when a reviewer reads `.claude/skills/speccy-init/SKILL.md` and
`.agents/skills/speccy-init/SKILL.md`,
then both ejected skill bodies carry the adaptive north-star flow
(T-003), the conventions upsert step (T-002), and the canonical
conventions body (T-001) inlined via the `{% include %}`
directive expansion.

Given the same working tree,
when a reviewer runs `git diff --stat -- 'speccy-cli/**/*.rs' 'speccy-core/**/*.rs' 'speccy-*/**/*.rs'`,
then the diff is empty — no Rust source files were modified by
this task.

Suggested files: `resources/agents/.claude/skills/speccy-init/SKILL.md.tmpl`,
`resources/agents/.agents/skills/speccy-init/SKILL.md.tmpl`
</task-scenarios>
</task>

<task id="T-005" state="pending" covers="REQ-007">
## Run the standard four-gate hygiene suite

Run the four hygiene gates required by AGENTS.md § "Standard
hygiene" before any commit lands:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

Each command must exit 0 with no warnings attributable to this
SPEC.

If any gate fails, investigate the underlying cause and fix it.
Do not skip hooks (`--no-verify`), bypass signing, or use
suppression macros (`#[allow(...)]`) to silence warnings — fix
the underlying issue per AGENTS.md § "Conventions for AI agents
specifically".

This task is sequenced last because T-001 through T-004 modify
template prose only and the gates exercise the Rust crates plus
the templating pipeline; any regression that surfaces here
attributes to render-pipeline interactions with the new include
directive (T-002) rather than to prose-edit work in T-001 or
T-003.

<task-scenarios>
Given the working tree at HEAD after T-001 through T-004 have
landed and `just reeject` has run,
when an operator runs `cargo test --workspace`,
then the command exits 0 with no failed tests.

Given the same working tree,
when an operator runs
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
then the command exits 0 with no warnings.

Given the same working tree,
when an operator runs `cargo +nightly fmt --all --check`,
then the command exits 0 with no formatting drift.

Given the same working tree,
when an operator runs `cargo deny check`,
then the command exits 0 with no dependency advisories.

Suggested files: (none — this task runs commands and reports
results, no edits)
</task-scenarios>
</task>
