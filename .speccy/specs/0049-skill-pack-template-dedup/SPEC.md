---
id: SPEC-0049
slug: skill-pack-template-dedup
title: Skill pack template dedup — canonical rule bodies stop leaking into wrappers and modules
status: implemented
created: 2026-05-27
supersedes: []
---

# SPEC-0049: Skill pack template dedup — canonical rule bodies stop leaking into wrappers and modules

## Summary

Speccy's `resources/` template tree carries canonical sources for the
`reconcile-policy`, `retry-shape`, and `vet-phases` rules under
`modules/references/` and `modules/skills/partials/`. Those same
rules also appear verbatim inlined at multiple non-canonical
callsites — the `speccy-work` wrappers in both host targets carry
the full retry-shape statement and the full reconcile-policy table,
and `modules/skills/speccy-orchestrate.md` inlines retry-shape again
alongside an `{% include %}` of the full reconcile-policy and the
full vet-phases grammar. After `just reeject`, the duplication
expands across `.claude/`, `.agents/`, and `.codex/` — each consumer
of a rule carries a verbatim copy.

This SPEC eliminates the verbatim copies at source, introduces
invariant-pattern partials (load-bearing trigger or rule inline plus
a pointer at the canonical reference), and normalizes the wrapper
convention so every skill wrapper is either a pure-include of a body
module from `modules/skills/` or a stub-delegate to an agent file.
After the refactor, the LLM driving `/speccy-orchestrate` sees each
rule's load-bearing summary once at the consuming site and follows
the pointer to the canonical reference only when it needs the full
policy table, formal definition, or worked examples. The vet skill
keeps its full Phase 0/1/2/3 grammar because vet owns the rule.

A subsequent amendment (REQ-006) also aligns the orchestrator's
outer-loop dispatch tree with the CLI's `NextAction::{Vet, Ship}`
kind split — friction discovered during the T-005 dogfood pass.
The fix lives in the same orchestrator skill body the dedup work
touches, so it is in scope for this SPEC rather than a follow-up.

This is a template-edit-plus-reeject refactor: no Rust changes, no
new lint commands, no new test infrastructure. Verification is the
standard hygiene suite plus a dogfood pass on a fresh non-trivial
SPEC.

## Goals

<goals>
- After `just reeject`, no file under `.claude/`, `.agents/`, or
  `.codex/` contains the verbatim policy table from
  `reconcile-policy.md` or the verbatim rule statement from
  `retry-shape.md` — except the canonical reference files themselves
  (e.g. `.claude/speccy-references/reconcile-policy.md`) and the
  canonical-owner skill body (`.claude/skills/speccy-vet/SKILL.md`
  keeps the full Phase 0/1/2/3 grammar).
- The LLM driving any orchestration loop reads each rule's
  load-bearing trigger or formal definition inline at the consuming
  site, and follows the pointer to the canonical reference only when
  it needs the full grammar.
- Skill wrappers under
  `resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` follow
  exactly one of two structural patterns for both `.claude` and
  `.agents` hosts: pure-include of a body module from
  `modules/skills/`, or stub-delegate pointer to an agent file.
- The standard four-gate hygiene suite passes after `just reeject`
  and `/speccy-orchestrate` completes a fresh non-trivial SPEC end
  to end without LLM confusion.
</goals>

## Non-goals

<non-goals>
- No Rust changes. `speccy-cli/src/render.rs`, the MiniJinja
  pipeline, and the command surface stay as-is. This SPEC is a
  template-edit-plus-reeject refactor.
- No new `speccy verify` lint codes, no new CLI subcommands, no new
  speccy-level lint tooling.
- No restructuring of `modules/skills/partials/vet-phases.md`. The
  phase grammar's internal shape stays as-is; only its inclusion
  pattern from `speccy-orchestrate.md` changes.
- No size-based gates on prose templates. Acceptance criteria for
  prose changes are content-based per AGENTS.md § "Skill pack source
  of truth" — `≤ N lines` or `exactly N lines` constraints are not
  used.
- No documentation Requirement for the "Skill pack source of truth"
  section in AGENTS.md. That edit ships separately on the SPEC-0048
  branch and is already in the working tree at brainstorm time.
- No changes to `.claude/rules/`. Those rule files are already clean
  and out of scope.
- No reviewer-* persona refactor. The reviewer modules already use
  `{% include %}` for `verdict_return_contract.md`,
  `inline_note_format.md`, `diff_fetch_command.md`, and
  `no_tasks_md_writes.md`. Source is already deduplicated; the
  remaining per-fan-out duplication at ejected layer is the price of
  giving each parallel reviewer the full contract.
</non-goals>

## User Stories

<user-stories>
- As an AI agent driving `/speccy-orchestrate` end-to-end, I want
  each rule's load-bearing invariant inline in the skill body, with
  the full grammar one Read away — so I do not have to choose between
  drowning in 1000+ lines of expanded includes or missing the rule
  entirely.
- As a maintainer editing a rule like `reconcile-policy`, I want to
  change exactly one file (the canonical reference under
  `resources/modules/references/`) and have `just reeject` propagate
  the change everywhere — instead of hunting down four verbatim
  copies that may have already drifted apart.
- As a Speccy dogfooder shipping a refactor, I want the eject
  pipeline to remain mechanical: edit source, run `just reeject`,
  verify the ejected output looks right. No Rust changes, no new
  tooling, no brittle test gates on prose shape.
</user-stories>

## Assumptions

<assumptions>
- Invariant text for `reconcile-policy` and `retry-shape` can be
  expressed concisely while preserving the load-bearing rule meaning
  for the LLM. If a rule does not compress without losing fidelity,
  fall back to sync-marker style or keep the full inline.
- The eject pipeline (`speccy init --force --host <host>` via `just
  reeject`) faithfully renders updated templates without requiring
  Rust changes in `speccy-cli/src/render.rs`. Templating-system code
  changes are out of scope.
- Dogfooding `/speccy-orchestrate` on a fresh SPEC surfaces
  comprehension regressions if any exist. Subtle regressions on rare
  inputs could ship undetected; this is accepted residual risk per
  the "no new mechanical comprehension gates" decision.
- The current `modules/skills/partials/vet-phases.md` is the right
  canonical home for the phase grammar. Restructuring vet phases is
  out of scope.
- No host-specific render-context reason forces the speccy-work
  wrapper to carry retry-shape and reconcile-policy inline rather
  than via a module. If wrapper-vs-module rendering differs in
  available Jinja variables, the work normalization may need a
  different shape than the vet/orchestrate/review pattern.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: No verbatim canonical rule bodies at non-canonical source sites

No file under `resources/` contains a verbatim copy of the canonical
rule content from `modules/references/reconcile-policy.md`,
`modules/references/retry-shape.md`, or
`modules/skills/partials/vet-phases.md`, except those canonical
files themselves and the canonical-owner skill body
(`modules/skills/speccy-vet.md` retains its `{% include %}` of the
full `vet-phases.md`). Every other site that needs the rule uses
`{% include %}` of either the canonical reference or an
invariant-partial wrapper of it.

<done-when>
- The verbatim rule statement from `retry-shape.md` (the sentence
  beginning ``T-NNN`` is in **retry shape** at `<spec-dir>` iff)
  does not appear in any file under `resources/` other than
  `modules/references/retry-shape.md` itself.
- The verbatim policy table from `reconcile-policy.md` (the
  Markdown table with rows for `commit_without_state`,
  `state_completed_no_commit`, etc.) does not appear in any file
  under `resources/` other than `modules/references/reconcile-policy.md`
  itself.
- The verbatim Phase 0/1/2/3 grammar from
  `modules/skills/partials/vet-phases.md` does not appear inlined
  in `modules/skills/speccy-orchestrate.md`;
  `modules/skills/speccy-vet.md` continues to include the full
  grammar via `{% include %}`.
- A new `modules/skills/speccy-work.md` exists carrying the
  speccy-work skill body; the `.claude` and `.agents` speccy-work
  wrappers consume it via `{% include %}` per REQ-003.
- Invariant-partial files exist for `reconcile-policy` and
  `retry-shape` (location at implementer discretion, e.g.
  `modules/references/partials/` or inline in the consuming module
  per DEC-002).
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands, when a
  reviewer searches `resources/` for the distinctive retry-shape
  rule sentence, then the only match is in
  `modules/references/retry-shape.md`.
- Given the same tree, when a reviewer searches for the
  distinctive reconcile-policy table heading (the literal string
  `| `kind`` followed by `commit_without_state` on a later row),
  then the only match is in `modules/references/reconcile-policy.md`.
- Given the same tree, when a reviewer searches for the
  distinctive vet-phases section heading
  `### Phase 0 — bootstrap`, then matches appear only in
  `modules/skills/partials/vet-phases.md` and
  `modules/skills/speccy-vet.md` (via the include expanding at
  render time during dogfood, not in the source body itself).
</behavior>

<scenario id="CHK-001">
Given the working tree at HEAD after this SPEC lands,
when a reviewer audits the source files that previously inlined the
retry-shape rule (the `.claude` and `.agents` speccy-work wrapper
templates plus `resources/modules/skills/speccy-orchestrate.md`)
together with the new `resources/modules/skills/speccy-work.md`,
then each audited file reaches the rule via `{% include %}` of
either the canonical `modules/references/retry-shape.md` or an
invariant-partial wrapper of it — no audited file retains a verbatim
copy of the canonical rule body. The check is reviewer judgment
against the DEC-001 / DEC-002 conventions; no automated prose
substring match is asserted per the no-vacuous-tests rule in
AGENTS.md § "Conventions for AI agents specifically".
</scenario>

<scenario id="CHK-002">
Given the same working tree,
when a reviewer audits the source files that previously inlined the
reconcile-policy table (the `.claude` and `.agents` speccy-work
wrapper templates) together with any module under
`resources/modules/skills/` that consumes the rule,
then each audited file reaches the rule via `{% include %}` of
either the canonical `modules/references/reconcile-policy.md` or an
invariant-partial wrapper — no audited file retains a verbatim copy
of the policy table. Reviewer judgment against the DEC-001 /
DEC-002 conventions confirms structural dedup.
</scenario>

<scenario id="CHK-003">
Given the same working tree,
when a reviewer audits how
`resources/modules/skills/speccy-orchestrate.md` reaches the
vet-phases grammar and how
`resources/modules/skills/speccy-vet.md` reaches it,
then orchestrate carries the DEC-002 invariant text plus a pointer
to `.claude/skills/speccy-vet/SKILL.md` without `{% include %}` of
the full `modules/skills/partials/vet-phases.md`; speccy-vet
continues to `{% include "modules/skills/partials/vet-phases.md" %}`
as the canonical owner of the phase grammar. Reviewer judgment
confirms the canonical-owner exception is preserved exactly where
DEC-002 names it.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Ejected SKILL bodies omit canonical rule grammars at non-owner callsites

After `just reeject`, no file under `.claude/`, `.agents/`, or
`.codex/` contains the verbatim policy table from
`reconcile-policy.md` or the verbatim rule statement from
`retry-shape.md`. Two exceptions: the canonical reference files
themselves (e.g. `.claude/speccy-references/reconcile-policy.md`),
and the canonical-owner skill body
(`.claude/skills/speccy-vet/SKILL.md` and
`.agents/skills/speccy-vet/SKILL.md` retain the full Phase 0/1/2/3
grammar because vet owns it).

<done-when>
- After `just reeject`, the verbatim retry-shape rule statement
  does not appear in `.claude/skills/speccy-orchestrate/SKILL.md`,
  `.claude/skills/speccy-work/SKILL.md`,
  `.claude/skills/speccy-review/SKILL.md`,
  `.claude/agents/speccy-work.md`, or `.codex/agents/speccy-work.toml`
  (nor in their `.agents/` siblings).
- After `just reeject`, the verbatim reconcile-policy table does
  not appear in the same set of ejected files.
- After `just reeject`, the verbatim vet-phases Phase 0/1/2/3
  grammar does not appear in `.claude/skills/speccy-orchestrate/SKILL.md`
  or `.agents/skills/speccy-orchestrate/SKILL.md`.
- After `just reeject`, the verbatim vet-phases grammar continues
  to appear in `.claude/skills/speccy-vet/SKILL.md` and
  `.agents/skills/speccy-vet/SKILL.md` (canonical-owner exception).
- After `just reeject`, the canonical reference files
  (`.claude/speccy-references/reconcile-policy.md`,
  `.claude/speccy-references/retry-shape.md`, and their `.agents/`
  siblings) continue to carry the full rule body.
</done-when>

<behavior>
- Given the eject pipeline has run on the refactored source, when
  a reviewer reads `.claude/skills/speccy-orchestrate/SKILL.md`,
  then they find the invariant text for reconcile-policy,
  retry-shape, and vet-phases inline, each followed by a pointer
  to the canonical reference or canonical-owner skill body — but
  not the full policy table, formal rule statement, or phase
  grammar.
- Given the same pipeline run, when a reviewer reads
  `.claude/skills/speccy-vet/SKILL.md`, then the full Phase 0/1/2/3
  grammar is present (vet is the canonical owner).
- Given the same pipeline run, when a reviewer reads
  `.codex/agents/speccy-work.toml`, then the developer_instructions
  body carries the invariant text inline but not the verbatim rule
  bodies.
</behavior>

<scenario id="CHK-004">
Given the working tree at HEAD after this SPEC lands and
`just reeject` has run,
when a reviewer audits the non-owner ejected files
(`.claude/skills/speccy-orchestrate/SKILL.md`,
`.claude/skills/speccy-work/SKILL.md`,
`.claude/skills/speccy-review/SKILL.md`,
`.claude/agents/speccy-work.md`, `.codex/agents/speccy-work.toml`,
plus their `.agents/` siblings) for the retry-shape rule body,
then each audited file carries the DEC-002 invariant text plus a
pointer to the canonical reference; none retain the full rule
body. The canonical reference files themselves
(`.claude/speccy-references/retry-shape.md` and its `.agents/`
sibling) continue to carry the full rule body. Reviewer judgment
confirms the structural slimming.
</scenario>

<scenario id="CHK-005">
Given the same state,
when a reviewer inspects
`.claude/skills/speccy-orchestrate/SKILL.md` and
`.agents/skills/speccy-orchestrate/SKILL.md` for the
reconcile-policy table,
then both files carry the DEC-002 invariant text plus a pointer to
the canonical reference (`.claude/speccy-references/reconcile-policy.md`
for the Claude host; the corresponding `.agents/` path for Codex);
the full policy table is absent from both.
</scenario>

<scenario id="CHK-006">
Given the same state,
when a reviewer inspects `.claude/skills/speccy-vet/SKILL.md` and
`.agents/skills/speccy-vet/SKILL.md` for the vet phase structure,
then the full Phase 0/1/2/3 grammar remains present in both files —
vet is the canonical owner per DEC-002 and retains the phase bodies
intact across the refactor.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Skill wrappers follow pure-include or stub-delegate, with no canonical body inline

Every skill wrapper under
`resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` (both
`.claude` and `.agents` hosts) follows exactly one of two
structural patterns per DEC-001: pure-include — frontmatter plus a
`{% include %}` of a body module from `modules/skills/`; or
stub-delegate — frontmatter plus a brief pointer telling the LLM to
Read or invoke the corresponding agent file. No wrapper carries
verbatim copies of canonical rule body text (reconcile-policy
table, retry-shape statement, vet-phases grammar). Brief
connective comments and guidance prose around the include or
pointer remain fine.

<done-when>
- `resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` and
  `resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl` no
  longer inline the retry-shape rule body or the reconcile-policy
  table; they take the pure-include shape (`{% include
  "modules/skills/speccy-work.md" %}`) per DEC-001(a).
- Every other skill wrapper under
  `resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` continues
  to match one of the two patterns from DEC-001 (no regression).
- Verification is content-based: a reviewer can grep the wrapper
  files for the distinctive canonical phrases and find no matches
  outside canonical sites. No wrapper-size gate is asserted.
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands, when a
  reviewer reads either speccy-work wrapper template, then they
  find a pure-include of `modules/skills/speccy-work.md` — not the
  retry-shape rule body or the reconcile-policy table.
- Given the same tree, when a reviewer compares the speccy-work
  wrapper against the speccy-vet, speccy-orchestrate, and
  speccy-review wrappers, then all four use the same pure-include
  shape (with the host-specific `speccy-orchestrate-codex-grant`
  addendum allowed in the `.agents` orchestrate wrapper).
- Given the same tree, when a reviewer reads the speccy-ship and
  speccy-decompose wrappers, then they remain in stub-delegate
  shape per DEC-001(b).
</behavior>

<scenario id="CHK-007">
Given the working tree at HEAD after this SPEC lands,
when a reviewer audits every skill wrapper under
`resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` for both
`.claude` and `.agents` hosts,
then no wrapper carries a verbatim copy of any canonical rule body
(retry-shape, reconcile-policy, vet-phases) inline; each wrapper
conforms to one of the two DEC-001 structural patterns. Reviewer
judgment is the verification.
</scenario>

<scenario id="CHK-008">
Given the same working tree,
when a reviewer inspects
`resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl` and
`resources/agents/.agents/skills/speccy-work/SKILL.md.tmpl`,
then both files are structurally a pure-include per DEC-001(a) —
frontmatter plus `{% include "modules/skills/speccy-work.md" %}` —
matching the pattern used by the speccy-vet, speccy-orchestrate, and
speccy-review wrappers. Host-specific addenda (such as the
`speccy-orchestrate-codex-grant.md` include in the `.agents`
orchestrate wrapper) are allowed.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Standard hygiene gates pass after the refactor

After the source-side template edits land and `just reeject` has
run, the standard four-gate hygiene suite continues to pass.

<done-when>
- `cargo test --workspace` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  exits 0.
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0.
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands, when an
  operator runs each gate command in sequence, then each exits 0.
- Given the same tree, when CI runs the equivalent workflow, then
  the workflow passes.
</behavior>

<scenario id="CHK-009">
Given the working tree at HEAD after this SPEC lands and
`just reeject` has run,
when `cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo +nightly fmt --all --check`, and `cargo deny check` each run
in sequence,
then every command exits 0 with no warnings or test failures
attributable to this refactor.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Work-review-ship loop runs end-to-end on the refactored skill pack

After `just reeject`, the work-review-ship loop driven by the
refactored skill pack completes successfully on at least one fresh
non-trivial SPEC.

Comprehension preservation is judged subjectively via the dogfood
observation. No new mechanical comprehension gates are introduced
per the prose-template verification rule in AGENTS.md § "Skill pack
source of truth". The specific aspects of "completes successfully"
live in the done-when block below.

<done-when>
- An operator can invoke `/speccy-orchestrate <fresh-spec>` on a
  non-trivial SPEC and watch the loop drive from first-task
  implementation through pre-ship without humans chaining per-task
  commands.
- Every `<implementer>`, `<review>`, and `<blockers>` element block
  emitted during the dogfood pass parses against the closed-set
  journal grammar (`JNL-*` lint family stays green).
- `speccy verify` exits 0 against the dogfood SPEC at the pre-ship
  boundary.
- A reviewer observing the dogfood pass reports no LLM confusion
  attributable to the refactor (subjective judgment; not a
  mechanical gate).
</done-when>

<behavior>
- Given the working tree at HEAD after this SPEC lands and
  `just reeject` has run, when an operator invokes
  `/speccy-orchestrate` on a fresh non-trivial SPEC, then the
  orchestrator drives the loop to the pre-ship boundary without
  manual intervention.
- Given the same state, when an operator reads the journal files
  at the end of the dogfood pass, then every emitted XML element
  block is well-formed and the closed-set grammar accepts the file.
- Given the same state, when `speccy verify` runs against the
  dogfood SPEC, then it exits 0.
</behavior>

<scenario id="CHK-010">
Given the working tree at HEAD after this SPEC lands and
`just reeject` has run,
when an operator invokes `/speccy-orchestrate <fresh-spec>` on a
non-trivial SPEC and runs the loop to completion,
then the orchestrator reaches the pre-ship boundary, every emitted
`<implementer>` / `<review>` / `<blockers>` element block validates
against the closed-set grammar, and `speccy verify` against the
dogfood SPEC exits 0.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Orchestrator dispatch tree separates `vet` from `ship` per CLI contract

The `speccy-orchestrate` skill body's outer-loop dispatch on
`next_action.kind` distinguishes the CLI's `vet` and `ship` kinds,
which `speccy-core::next::compute_for_spec` emits in sequence after
all tasks reach `state="completed"`. `Vet` is emitted when no fresh
passing vet-gate artifact exists (no `<gate verdict="passed">` block
in `journal/VET.md` whose `tasks_hash` matches the current TASKS.md
SHA-256); `Ship` is emitted after a fresh passing gate lands and
REPORT.md is absent.

Today's skill body lists only `{work, review, ship, decompose}` in
its Loop step 2 dispatch enum and binds the vet workflow under the
`ship` kind. Against the CLI's actual contract this means: on a
freshly task-completed SPEC the orchestrator's first
`speccy next --json` returns `vet` and the skill body would STOP
(kind not in the enum); on a re-query after the gate passes, CLI
returns `ship` and the skill body would re-run the vet workflow it
already completed. The amendment splits the dispatch.

<done-when>
- `resources/modules/skills/speccy-orchestrate.md` Loop step 2
  dispatch enumeration lists `vet` and `ship` as distinct kinds,
  each routed to its own dispatch section.
- The section that runs the speccy-vet skill body inline is bound
  to the `vet` kind (renamed from "Ship dispatch" or split from
  it). The vet-verdict pass/fail reaction prose lives inside the
  Vet dispatch section as the natural exit path of the vet
  workflow.
- A new "Ship dispatch" section is bound to the `ship` kind and
  performs only the user-confirmation step plus the speccy-ship
  sub-agent spawn on confirm; it does not re-run the vet workflow.
- The Lifecycle ASCII tree at the top of the orchestrator body
  reflects the new vet/ship split.
- The Stop conditions section's "`next_action.kind` is not one of
  …" enum lists `work`, `review`, `vet`, `ship`, `decompose`.
- After `just reeject`, `.claude/skills/speccy-orchestrate/SKILL.md`
  and `.agents/skills/speccy-orchestrate/SKILL.md` carry the
  post-amendment dispatch tree.
- The standard four-gate hygiene suite continues to pass.
</done-when>

<behavior>
- Given the working tree at HEAD after this amendment lands and
  `just reeject` has run, when an operator reads
  `resources/modules/skills/speccy-orchestrate.md`, then Loop step
  2's dispatch enumeration carries five kinds (`work`, `review`,
  `vet`, `ship`, `decompose`) and the "Vet dispatch" / "Ship
  dispatch" sections are structurally distinct.
- Given the same tree, when `/speccy-orchestrate` runs against a
  SPEC whose tasks are all completed with no fresh vet-gate
  artifact, then the orchestrator dispatches on `vet`, runs the
  speccy-vet skill body inline, and re-queries; on the re-query
  the CLI emits `ship` and the orchestrator dispatches on `ship`
  to ask the user.
</behavior>

<scenario id="CHK-011">
Given the working tree at HEAD after this amendment lands and
`just reeject` has run,
when a reviewer reads `resources/modules/skills/speccy-orchestrate.md`,
then Loop step 2 enumerates `vet` and `ship` as distinct dispatch
kinds, a dedicated section runs the speccy-vet skill body inline on
`vet` (Phase 0-3 plus the vet-verdict pass/fail reaction), and a
separate "Ship dispatch" section asks the user and spawns
speccy-ship on `ship` without re-running vet.
</scenario>

<scenario id="CHK-012">
Given the same state,
when a reviewer reads the ejected
`.claude/skills/speccy-orchestrate/SKILL.md` and
`.agents/skills/speccy-orchestrate/SKILL.md` files,
then both carry the post-amendment dispatch tree: Loop step 2
enumerates five kinds; the Vet dispatch section is bound to the
`vet` kind and inlines the speccy-vet skill body; the Ship dispatch
section is bound to the `ship` kind and performs only the
user-confirmation + speccy-ship spawn; the Stop conditions enum
lists all five kinds.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
### DEC-001: Wrapper-pattern convention — pure-include or stub-delegate, no inline canonical bodies

Skill wrappers under
`resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` follow
exactly one of two structural patterns:

- **Pure-include.** Frontmatter plus a single `{% include %}` of a
  body module from `modules/skills/`. Brief connective comments
  around the include are fine.
- **Stub-delegate.** Frontmatter plus a brief pointer prose telling
  the LLM to Read or invoke the corresponding agent file under
  `.claude/agents/` (or its `.codex` equivalent).

No wrapper carries verbatim copies of canonical rule body text
(reconcile-policy policy table, retry-shape rule statement,
vet-phases Phase 0/1/2/3 grammar). The wrapper's job is to carry
frontmatter and route to the body; the body lives in a module.

This convention is also documented in AGENTS.md § "Skill pack
source of truth" and is pinned here as the load-bearing decision
behind REQ-003. Acceptance for the convention is content-based per
AGENTS.md § "Verifying prose-template changes" — no size-based
gates ("wrapper ≤ N lines") are enforced; a reviewer who wants to
add brief connective prose around the include or pointer can do so
without tripping a test.
</decision>

<decision id="DEC-002">
### DEC-002: Invariant wording carries the rule's trigger or formal definition inline

At every non-canonical callsite that previously inlined a full
canonical rule body, the refactored content uses an invariant
formulation that carries the load-bearing trigger or formal
definition inline plus a pointer to the canonical reference. The
agreed wording per brainstorm OQ-a/b/c is:

- **`reconcile-policy` invariant.** *"Reconcile policy. When
  `speccy next --json` returns `next_action.kind == "reconcile"`,
  iterate `consistency.drifts[]` and apply the table action per
  entry, then re-query before proceeding. See
  `.claude/speccy-references/reconcile-policy.md` for the full
  policy table."*
- **`retry-shape` invariant.** *"Retry shape. A task is in retry
  shape iff its journal contains both an `<implementer>` element
  and a `<blockers>` element whose `round` attribute matches the
  highest implementer round. Otherwise it's first-attempt shape —
  the strict clean-tree gate applies. See
  `.claude/speccy-references/retry-shape.md`."*
- **`vet-phases` pointer in orchestrate.** *"Vet phases. Phase 0
  bootstraps the journal; Phase 1 runs drift review with an
  autonomous fix-and-retry loop; Phase 2 runs the simplifier
  polish pass; Phase 3 writes the final `<gate>` block. Run in
  order; see `.claude/skills/speccy-vet/SKILL.md` § Phase N for the
  full grammar."*

The rejected alternative was a minimal one-line pointer with no
trigger or definition. The chosen wording carries enough
load-bearing semantics that the LLM can act correctly without an
extra Read in the common case; the reference covers the long tail.

Hosts other than `.claude` (`.agents`, `.codex`) substitute their
own canonical reference paths where applicable; the load-bearing
clause structure (trigger, formal definition, dispatch summary)
stays identical.
</decision>

## Open Questions

All open questions from the brainstorm session (`a` through `h`)
were resolved before this SPEC was drafted. No outstanding
questions remain at draft time.

## Notes

Three alternative framings were considered and rejected during
brainstorm:

- **Minimal source-dedup only.** Swap the verbatim inlines for
  `{% include %}` of the canonical without introducing
  invariant-partial wrappers. Ejected output would still contain
  the full canonical rule bodies at every callsite because
  `{% include %}` expands. Rejected: solves only the maintenance
  problem (priority #3); does nothing for LLM comprehension
  (priority #1) or context bytes per invocation (priority #2).
- **Two-phase split.** Land the source-dedup as one SPEC and the
  invariant-pattern conversion as a follow-up. Rejected at
  brainstorm Q2: chose one unified SPEC because the dedup and
  invariant-pattern work touch overlapping files; a second PR
  would churn the same surface.
- **Lint-only enforcement.** Add a `speccy verify` check that
  flags source-level verbatim duplication and decline to refactor
  existing files. Rejected at brainstorm Q3: explicitly declined
  new lint tooling, and the existing duplication still needs
  cleanup independently of regression prevention.

The `reviewer-*` persona refactor that an earlier audit pass
considered is out of scope. The persona modules under
`resources/modules/personas/` already use `{% include %}` for the
shared snippets (`verdict_return_contract.md`,
`inline_note_format.md`, `diff_fetch_command.md`,
`no_tasks_md_writes.md`); source-side deduplication is already in
place. The remaining duplication at the ejected layer (each
reviewer-* agent file carries the expanded verdict-return contract)
is the acceptable cost of giving each parallel reviewer the full
contract in-context — the contract is short, load-bearing, and
each reviewer runs in its own subagent invocation so the cost is
not additive.

The implementer drafting the new `modules/skills/speccy-work.md` may
take the opportunity to reorganize and clarify its structure if a
meaningful clarity improvement is possible — otherwise port the
current wrapper inline body 1:1, swapping retry-shape and
reconcile-policy for the invariant formulations from DEC-002. This
is implementer judgment per brainstorm OQ-d.

The verification scenarios under each requirement intentionally use
reviewer-audit framing rather than automated prose-substring matching,
per AGENTS.md § "Conventions for AI agents specifically"
no-vacuous-tests rule. Automated grep over canonical rule prose
would gate editorial decisions (rewordings of the canonical rule)
rather than the dedup behavior, so the SPEC enforces dedup socially
via review judgment and via the dogfood pass.

## Changelog

<changelog>
| Date       | Author              | Summary |
|------------|---------------------|---------|
| 2026-05-27 | claude-opus-4-7[1m] | Initial draft. Five requirements: (REQ-001) source-side dedup of canonical rule bodies (retry-shape, reconcile-policy, vet-phases) at non-canonical sites under `resources/`; (REQ-002) ejected slimming across `.claude/`, `.agents/`, and `.codex/` so non-owner ejected files carry only the DEC-002 invariant text plus a pointer at the canonical reference; (REQ-003) wrapper structural consistency — pure-include or stub-delegate per DEC-001, no inline canonical bodies; (REQ-004) standard hygiene gates pass; (REQ-005) work-review-ship dogfood pass completes. Two decisions: DEC-001 (wrapper-pattern convention) and DEC-002 (invariant wording carries the rule's trigger or formal definition inline plus a pointer; chose Option-2 from brainstorm OQ-a/b/c over minimal one-line pointer). Scope is intentionally template-edit-plus-reeject — no Rust changes, no new lint commands. Verification scenarios use reviewer-audit framing per AGENTS.md § "Conventions for AI agents specifically" no-vacuous-tests rule. AGENTS.md "Skill pack source of truth" section was added in parallel on the SPEC-0048 branch and is treated as already-on-main at SPEC start time per OQ-h. |
| 2026-05-27 | claude-opus-4-7[1m] | Amended to add REQ-006: align orchestrator dispatch tree with CLI's `NextAction::{Vet, Ship}` kind split. Friction discovered during the T-005 dogfood pass — the skill body's Loop step 2 dispatch enum lists `{work, review, ship, decompose}` and binds the vet workflow under `ship`, but the CLI emits `vet` (no fresh gate) and `ship` (fresh gate, REPORT.md absent) as distinct kinds in sequence after all tasks reach `state="completed"`. The manual workaround during T-005 (treating `vet` as the trigger for the inline vet workflow, and `ship` as the user-confirmation step) was harmless — same end outcome — but leaves a stale dispatch tree shipped to other repos via `speccy init`. Fix is a focused edit to `resources/modules/skills/speccy-orchestrate.md` (Lifecycle tree, Loop step 2, section rename, new Ship dispatch section, Stop conditions enum, Status reporting examples) plus `just reeject`; T-001..T-005 stay completed because none of their done-when items are invalidated. The Summary gained one sentence acknowledging the amendment's broader scope. |
</changelog>
