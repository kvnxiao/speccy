---
spec: SPEC-0049
spec_hash_at_generation: 121256122258e93b222a710619f2fa4d6455b32767629e2af93cf3f3eee2abf8
generated_at: 2026-05-27T08:16:49Z
---
# Tasks: SPEC-0049 Skill pack template dedup — canonical rule bodies stop leaking into wrappers and modules

<task id="T-001" state="completed" covers="REQ-001">
## Create `modules/skills/speccy-work.md` with DEC-002 invariant body

Add `resources/modules/skills/speccy-work.md` as the new
host-neutral canonical skill body for `speccy-work`. Port today's
speccy-work wrapper template body (the inline content of
`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` and
`resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`,
which carry identical bodies modulo host-specific paths) into the
new module, swapping the two verbatim canonical rule bodies per
DEC-002:

- Replace the `<!-- Shared rule: retry-shape. -->` through
  `<!-- End shared rule: retry-shape. -->` block (lines ~13-95 of
  the current wrapper) with the DEC-002 retry-shape invariant:
  "Retry shape. A task is in retry shape iff its journal contains
  both an `<implementer>` element and a `<blockers>` element whose
  `round` attribute matches the highest implementer round.
  Otherwise it's first-attempt shape — the strict clean-tree gate
  applies. See `{{ speccy_references_path }}/retry-shape.md`."
- Replace the `<!-- Shared partial: reconcile-policy. -->` through
  `<!-- End shared partial: reconcile-policy. -->` block (lines
  ~97-210 of the current wrapper) with the DEC-002
  reconcile-policy invariant: "Reconcile policy. When
  `speccy next --json` returns `next_action.kind == \"reconcile\"`,
  iterate `consistency.drifts[]` and apply the table action per
  entry, then re-query before proceeding. See
  `{{ speccy_references_path }}/reconcile-policy.md` for the full
  policy table."

Use `{{ cmd_prefix }}` for slash-command rendering and
`{{ speccy_references_path }}` for canonical-reference paths.
Use `{% if host == "claude-code" %}` conditionals where the body
needs host-specific paths (e.g. the agent-file pointer
`.claude/agents/speccy-work.md` vs `.codex/agents/speccy-work.toml`).
Per the SPEC's Notes section, the implementer may reorganize the
body for clarity if a meaningful improvement is possible;
otherwise port the wrapper body 1:1 with the two DEC-002
substitutions above. Inline DEC-002 prose at each consuming site
is the simplest path; standalone invariant-partial files under
`modules/references/partials/` are optional implementer judgment.

The new module exists but is not yet consumed by the speccy-work
wrappers — the wrapper switch to pure-include lands in T-004. Run
`just reeject` after the edit to confirm no ejected diff is
produced from this task alone (the new module has no consumers
yet).

<task-scenarios>
Given the working tree at HEAD after this task,
when a reviewer reads `resources/modules/skills/speccy-work.md`,
then the file exists, opens with a heading referencing
`{{ cmd_prefix }}speccy-work`, and carries the speccy-work flow
body with the DEC-002 retry-shape invariant and reconcile-policy
invariant inline — not the verbatim retry-shape rule statement or
the verbatim reconcile-policy policy table.

Given the same tree,
when a reviewer greps `resources/modules/skills/speccy-work.md`
for the distinctive retry-shape rule sentence (`is in **retry
shape** at` followed within a few words by `iff`),
then no match is found in that file.

Given the same tree,
when a reviewer greps `resources/modules/skills/speccy-work.md`
for the distinctive reconcile-policy table row
(`commit_without_state` adjacent to `auto_fixable`),
then no match is found in that file.

Given the same tree,
when `just reeject` runs and the operator inspects `git status`,
then no ejected file under `.claude/`, `.agents/`, or `.codex/`
shows a diff attributable solely to this task — the new module is
unconsumed by any wrapper yet (wrapper switching is T-004's
work).

Suggested files: `resources/modules/skills/speccy-work.md`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001">
## Refactor `modules/skills/speccy-orchestrate.md` — remove three verbatim canonical bodies

Edit `resources/modules/skills/speccy-orchestrate.md` to remove
three verbatim canonical-rule sites and replace each with its
DEC-002 invariant formulation:

1. **Reconcile-policy include** (in the "Startup integrity check"
   section, currently around lines 130-132). Replace the
   `<!-- Shared partial: reconcile-policy. -->` through
   `<!-- End shared partial: reconcile-policy. -->` block (the
   `{% include "modules/references/reconcile-policy.md" %}` and
   its bounding marker comments) with the DEC-002 reconcile-policy
   invariant prose plus a pointer to
   `{{ speccy_references_path }}/reconcile-policy.md`.
2. **Retry-shape inline body** (in the "Work dispatch" section,
   currently around lines 209-291). Replace the
   `<!-- Shared rule: retry-shape. -->` through
   `<!-- End shared rule: retry-shape. -->` block with the DEC-002
   retry-shape invariant prose plus a pointer to
   `{{ speccy_references_path }}/retry-shape.md`.
3. **Vet-phases include** (in the "Ship dispatch" section,
   currently around line 341). Replace the
   `{% include "modules/skills/partials/vet-phases.md" %}` line
   with the DEC-002 vet-phases pointer prose: "Vet phases. Phase 0
   bootstraps the journal; Phase 1 runs drift review with an
   autonomous fix-and-retry loop; Phase 2 runs the simplifier
   polish pass; Phase 3 writes the final `<gate>` block. Run in
   order; see `.claude/skills/speccy-vet/SKILL.md` § Phase N for
   the full grammar." Hosts other than `.claude` substitute their
   own ejected path via Jinja conditional or
   `{{ speccy_references_path }}` analogue.

Leave `resources/modules/skills/speccy-vet.md` untouched — it
continues to `{% include "modules/skills/partials/vet-phases.md" %}`
as the canonical owner of the phase grammar per DEC-002.

Run `just reeject` to propagate to
`.claude/skills/speccy-orchestrate/SKILL.md` and
`.agents/skills/speccy-orchestrate/SKILL.md`.

<task-scenarios>
Given the working tree at HEAD after this task and `just reeject`
having run,
when a reviewer reads `resources/modules/skills/speccy-orchestrate.md`,
then it no longer contains the inline retry-shape rule body, no
longer contains the `{% include "modules/references/reconcile-policy.md" %}`
line, and no longer contains the `{% include "modules/skills/partials/vet-phases.md" %}`
line — each site carries DEC-002 invariant prose plus a pointer
instead.

Given the same tree,
when a reviewer reads `.claude/skills/speccy-orchestrate/SKILL.md`
and `.agents/skills/speccy-orchestrate/SKILL.md` after the eject,
then neither file carries the verbatim retry-shape rule statement
(the `is in **retry shape** at` ... `iff` paragraph), the
verbatim reconcile-policy policy table (the
`commit_without_state | auto_fixable` row pattern), or the Phase
0/1/2/3 grammar (the `### Phase 0 — bootstrap` heading). Each
carries DEC-002 invariant text plus a pointer to the canonical
reference path or canonical-owner skill body.

Given the same tree,
when a reviewer reads `resources/modules/skills/speccy-vet.md`,
then it still contains `{% include "modules/skills/partials/vet-phases.md" %}`
unchanged — the canonical-owner exception per DEC-002 is
preserved.

Suggested files: `resources/modules/skills/speccy-orchestrate.md`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001">
## Refactor `modules/skills/speccy-review.md` and `modules/phases/speccy-work.md` — remove remaining verbatim canonical bodies

Edit the two remaining consuming source files to replace their
verbatim canonical rule bodies with the DEC-002 invariant
formulations:

1. **`resources/modules/skills/speccy-review.md`** (in the
   "Entry precondition" section, currently around lines 48-50).
   Replace the `<!-- Shared partial: reconcile-policy. -->`
   through `<!-- End shared partial: reconcile-policy. -->` block
   (the `{% include "modules/references/reconcile-policy.md" %}`
   and its bounding marker comments) with the DEC-002
   reconcile-policy invariant prose plus a pointer to
   `{{ speccy_references_path }}/reconcile-policy.md`.
2. **`resources/modules/phases/speccy-work.md`** (currently
   around lines 91-173). Replace the
   `<!-- Shared rule: retry-shape. -->` through
   `<!-- End shared rule: retry-shape. -->` block (the verbatim
   retry-shape rule statement, the read-only-scope discussion,
   and the two worked examples) with the DEC-002 retry-shape
   invariant prose plus a pointer to
   `{{ speccy_references_path }}/retry-shape.md`.

Run `just reeject` to propagate the changes to all four consuming
ejected files: `.claude/skills/speccy-review/SKILL.md`,
`.agents/skills/speccy-review/SKILL.md`,
`.claude/agents/speccy-work.md`, and
`.codex/agents/speccy-work.toml`.

<task-scenarios>
Given the working tree at HEAD after this task and `just reeject`
having run,
when a reviewer reads `resources/modules/skills/speccy-review.md`,
then it no longer contains `{% include "modules/references/reconcile-policy.md" %}`;
the include is replaced by DEC-002 reconcile-policy invariant
prose plus a pointer to the canonical reference path.

Given the same tree,
when a reviewer reads `resources/modules/phases/speccy-work.md`,
then it no longer contains the inline retry-shape rule body (no
`## Rule statement` heading inside the file, no `is in **retry
shape** at` paragraph, no `## Worked example 1 — retry shape`
heading), each replaced by DEC-002 retry-shape invariant prose
plus a pointer.

Given the same tree,
when a reviewer reads `.claude/skills/speccy-review/SKILL.md`,
`.agents/skills/speccy-review/SKILL.md`,
`.claude/agents/speccy-work.md`, and
`.codex/agents/speccy-work.toml` after the eject,
then none of these files carry the verbatim retry-shape rule
statement or the verbatim reconcile-policy policy table; each
carries DEC-002 invariant text plus a pointer to the
host-specific canonical reference path.

Suggested files: `resources/modules/skills/speccy-review.md`,
`resources/modules/phases/speccy-work.md`
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-002 REQ-003 REQ-004">
## Refactor speccy-work wrappers to pure-include, re-eject, audit ejected output, verify hygiene

Edit the two speccy-work wrapper templates to take the
pure-include shape per DEC-001(a):

- `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` →
  YAML frontmatter plus
  `{% include "modules/skills/speccy-work.md" %}` (the module
  added in T-001), with nothing else in the body.
- `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl` →
  same shape (frontmatter plus the same include).

Remove the existing inline retry-shape rule body, reconcile-policy
table, "Entry precondition" paragraph, and "Hygiene gate"
paragraph from both wrapper templates — all of that lives in
`modules/skills/speccy-work.md` after T-001. After the edit, each
wrapper is structurally identical in shape to the speccy-vet,
speccy-orchestrate, and speccy-review wrappers — frontmatter plus
a single body include (the `.agents` orchestrate wrapper's
host-specific `speccy-orchestrate-codex-grant.md` addendum is the
documented exception per DEC-001).

Run `just reeject` to propagate every source-side change from
T-001 through this task into the ejected packs at `.claude/`,
`.agents/`, and `.codex/`.

Audit the ejected output:

- Confirm `.claude/skills/speccy-work/SKILL.md`,
  `.claude/skills/speccy-orchestrate/SKILL.md`,
  `.claude/skills/speccy-review/SKILL.md`,
  `.claude/agents/speccy-work.md`,
  `.codex/agents/speccy-work.toml`, and their `.agents/` siblings
  no longer contain the verbatim retry-shape rule statement, the
  verbatim reconcile-policy policy table, or — for the
  orchestrate file specifically — the verbatim vet-phases Phase
  0/1/2/3 grammar.
- Confirm `.claude/skills/speccy-vet/SKILL.md` and
  `.agents/skills/speccy-vet/SKILL.md` continue to carry the full
  Phase 0/1/2/3 grammar (canonical-owner exception per DEC-002).
- Confirm `.claude/speccy-references/reconcile-policy.md`,
  `.claude/speccy-references/retry-shape.md`, and their `.agents/`
  siblings continue to carry the full rule bodies (canonical
  reference files).
- Confirm every other skill wrapper under
  `resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` for both
  `.claude` and `.agents` hosts still conforms to one of DEC-001's
  two patterns (pure-include or stub-delegate). No regression in
  any other wrapper.

Finally, run the standard four-gate hygiene suite and confirm
each exits 0: `cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, `cargo deny check`.

<task-scenarios>
Given the working tree at HEAD after this task and `just reeject`
having run,
when a reviewer reads `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`
and `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`,
then each consists of YAML frontmatter plus a single
`{% include "modules/skills/speccy-work.md" %}` directive — no
inline retry-shape body, no inline reconcile-policy table, no
inline "Entry precondition" or "Hygiene gate" paragraphs.

Given the same tree,
when a reviewer audits the five ejected file paths named in
REQ-002 (`.claude/skills/speccy-orchestrate/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.claude/skills/speccy-review/SKILL.md`,
`.claude/agents/speccy-work.md`, `.codex/agents/speccy-work.toml`)
and their `.agents/` siblings,
then none of them contain the verbatim retry-shape rule statement
(the `is in **retry shape** at` ... `iff` paragraph), the
verbatim reconcile-policy policy table (the
`commit_without_state | auto_fixable` row pattern), or — in the
orchestrate file — the verbatim vet-phases grammar (the
`### Phase 0 — bootstrap` heading).

Given the same tree,
when a reviewer reads `.claude/skills/speccy-vet/SKILL.md` and
`.agents/skills/speccy-vet/SKILL.md`,
then both files continue to carry the full Phase 0/1/2/3 grammar
with the `### Phase 0 — bootstrap` heading intact — canonical-owner
exception per DEC-002 preserved.

Given the same tree,
when a reviewer scans every skill wrapper template under
`resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` for both
`.claude` and `.agents` hosts,
then each conforms to one of DEC-001's two structural patterns
(pure-include of a body module from `modules/skills/`, or
stub-delegate pointer to an agent file); no wrapper retains
inline canonical rule body text.

Given the same tree,
when an operator runs `cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, and `cargo deny check` in
sequence,
then each command exits 0 with no warnings or test failures
attributable to this refactor.

Suggested files: `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl`,
`resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-005">
## Dogfood pass — run `/speccy-orchestrate` end-to-end on a non-trivial SPEC

Drive the refactored skill pack end-to-end to validate REQ-005's
"work-review-ship loop runs end-to-end" claim. The implementer
picks a non-trivial SPEC for the dogfood pass — either a fresh
small SPEC scaffolded for this purpose, or the current SPEC-0049
itself if it still has pending or in-review tasks after T-001
through T-004 land (in which case the orchestrator picks up from
the next implementable task).

Steps:

1. Identify the target SPEC (`SPEC-NNNN`) for the dogfood pass and
   record the choice in this task's `<implementer>` block.
2. Invoke `{{ cmd_prefix }}speccy-orchestrate SPEC-NNNN` from a
   top-level session and observe the loop drive from the next
   pending task through to the pre-ship boundary without humans
   chaining per-task commands.
3. After the loop reaches the pre-ship boundary with a
   `verdict="pass"` from the holistic gate (or stops with a
   reportable status), inspect each per-task journal at
   `.speccy/specs/NNNN-slug/journal/T-NNN.md` and confirm every
   `<implementer>`, `<review>`, and `<blockers>` element block
   parses against the closed-set journal grammar (`JNL-*` lint
   family stays green via `speccy verify`).
4. Run `speccy verify SPEC-NNNN` and confirm it exits 0.

Capture subjective observations of LLM comprehension in the
`<implementer>` block: did the orchestrator follow the DEC-002
invariants correctly when applying the retry-shape rule and the
reconcile policy? Did it consult `.claude/speccy-references/...`
when it needed the long-form rule? Per AGENTS.md § "Skill pack
source of truth", no new mechanical comprehension gates are
introduced; reviewer judgment of the implementer's observations
is the verification.

If the dogfood pass surfaces a comprehension regression, flip the
task back to `pending` with a `<blockers>` block describing the
issue — the implementer of the retry round adjusts the DEC-002
wording or restructures the consuming module to fix the
regression before re-attempting the dogfood.

<task-scenarios>
Given the working tree at HEAD after T-004 lands and
`just reeject` has run,
when an operator invokes
`{{ cmd_prefix }}speccy-orchestrate <target-spec>` on a
non-trivial SPEC,
then the orchestrator drives the implementation loop from the
next pending task through to the pre-ship boundary without
manual intervention beyond the SPEC selection — no LLM-confusion
halts attributable to the refactor.

Given the loop reaches the pre-ship boundary,
when the operator reads each per-task journal at
`.speccy/specs/NNNN-slug/journal/T-NNN.md` for the dogfood SPEC,
then every `<implementer>`, `<review>`, and `<blockers>` element
block is well-formed and the closed-set journal grammar accepts
the files (the `JNL-*` lint family stays green).

Given the same state,
when `speccy verify <target-spec>` runs,
then it exits 0 with no proof-shape violations attributable to
the refactor.

Suggested files: (no source edits; the implementer's journal at
`.speccy/specs/0049-skill-pack-template-dedup/journal/T-005.md`
captures the dogfood evidence)
</task-scenarios>
</task>
