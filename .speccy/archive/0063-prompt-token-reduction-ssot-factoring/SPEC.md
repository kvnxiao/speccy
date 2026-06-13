---
id: SPEC-0063
slug: prompt-token-reduction-ssot-factoring
title: Prompt token reduction + SSOT factoring for `resources/` modules — compress SDLC-loop prose, defer two procedures to existing CLI guarantees, then factor duplicated passages into shared modules
status: implemented
created: 2026-06-12
supersedes: []
archived_at: 2026-06-13
archived_reason: "shipped"
---

# SPEC-0063: Prompt token reduction + SSOT factoring for `resources/` modules — compress SDLC-loop prose, defer two procedures to existing CLI guarantees, then factor duplicated passages into shared modules

## Summary

Every prompt under `resources/modules/**` ejects into users' repos and reloads
into agent context on every turn of the Speccy SDLC loop. The reviewer fan-out
expands into five parallel subagent windows on every review round of every task;
the vet, orchestrate, and phase bodies are resident or per-invocation. Words that
restate a fact already stated at its canonical site, or that re-explain a guarantee
the CLI already provides, are paid for repeatedly at runtime.

This SPEC lands two independent, sequenced changes as two commits on one branch.

**Track 1 (commit 1) — token reduction.** Cut verbose prose and cross-file
restatements from the SDLC-loop modules so each operational fact is stated once,
without ever reducing a deliberate inline-rule-plus-pointer to a bare pointer
(that would force an on-demand file read and cost *more* tokens — SPEC-0049
DEC-002). Track 1 also simplifies two agent procedures to lean on guarantees the
CLI already gives: the work phase stops re-reading the journal after an append
(the CLI validates before any write, so a malformed block can never land), and
the ship phase stops issuing two redundant `speccy status` round-trips (the
single `speccy next --json` already run yields readiness and paths, and the
state-flip is excluded from the spec hash so `TSK-003` cannot fire).

**Track 2 (commit 2) — SSOT factoring.** Token-neutral and organizational: kill
the inlined copies that shadow a canonical source (AGENTS.md calls these bugs) by
consolidating each enumerated duplicated passage into one shared module pulled in
with a MiniJinja `{% include %}` at every former callsite. Because includes
expand at eject time, this saves zero runtime tokens — its value is one source of
truth. Each new module must expand to text equivalent to the copy it replaced,
preserving act-without-read where the passage is a one-liner-plus-pointer. Track 2
runs after Track 1 so it factors the already-compressed wording, and it records a
decision in this SPEC that supersedes SPEC-0034 DEC-001 (which deliberately kept
the plan/amend self-review inline "until both templates stabilized" and
pre-authorized a later extraction).

Verification rests on the existing guardrail tests (`persona_snippets.rs`,
`authoring_commit.rs`) staying green, structural checks over the ejected packs
(no `{%` markers; the eject diff mirrors the source edits), and reviewer audits
recorded in `REPORT.md` — not on a measured token or line-count target.

## Goals

<goals>
- After Track 1, the passages identified during decomposition as redundant
  restatements or over-explanation no longer appear in `resources/modules/**` or
  the ejected packs, while each operational fact still appears once at a canonical
  site and no kept inline-rule-plus-pointer is reduced to a bare pointer.
- The work phase no longer instructs a post-append journal re-read; it relies on
  the CLI's validate-before-write guarantee.
- The ship phase no longer issues the redundant `speccy status` calls in step 1
  and step 3; it relies on the single `speccy next --json` and the hash-neutral
  state-flip.
- After Track 2, each of the six enumerated duplicated passages exists as exactly
  one canonical shared module, `{% include %}`d at every former callsite with no
  inline copy remaining, and every module expands at eject time to text equivalent
  to the copy it replaced.
- A `<decision>` in this SPEC supersedes SPEC-0034 DEC-001, the new self-review
  module references it, and `speccy-brainstorm`'s self-review stays independent.
- Both commits keep the full hygiene suite green and leave the ejected packs free
  of `{%` markers, with each eject diff mirroring the `resources/modules/**` edits.
</goals>

## Non-goals

<non-goals>
- No measured token-reduction or line-count acceptance threshold. The plan's
  ~330–360-line estimate is descriptive context, not a criterion.
- No factoring of the inline-fanout rationale or reviewer Role tail "for tokens."
  They expand at eject time, so factoring them wins zero runtime tokens; they are
  consolidated in Track 2 for SSOT only, never justified as a token play.
- No reduction of the `retry-shape` (or any) inline rule definition to a bare
  pointer. That forces an on-demand long-reference read and costs more tokens;
  SPEC-0049 DEC-002 makes the inline-sentence-plus-pointer deliberate.
- No new CLI command and no git-mutating helper (`speccy commit`, a `branch-guard`
  verb, `vet snapshot`/`revert`). These cross Core Principle #2 (deterministic
  core, the CLI never mutates git) and are explicitly excluded.
- No deferral of the vet base-ref derivation to an existing command. There is no
  clean fit — vet is spec-scoped while `speccy context` is task-scoped — so this
  would require new surface, which is out of scope.
- No edit to the archived SPEC-0034 or SPEC-0049. Their decisions are referenced,
  and SPEC-0034 DEC-001 is superseded, in this SPEC's live record — the archived
  documents stay unedited as dogfood history.
</non-goals>

## User Stories

<user-stories>
- As a reviewer subagent spawned five-up per review round, I want my persona body
  and its shared includes to carry only the facts I act on, so each parallel
  window costs fewer tokens without losing any invariant my review depends on.
- As an agent driving the work and ship phases, I want my procedure to stop
  issuing CLI calls whose result the CLI already guarantees, so the loop does
  fewer round-trips and my context carries less dead instruction.
- As a maintainer of the skill pack, I want each duplicated passage to live in
  exactly one module, so an edit lands once and no inlined copy silently drifts
  from its canonical source.
</user-stories>

## Assumptions

<assumptions>
- Verification rests on existing tests plus structural checks — a named passage is
  absent from source and eject, a required invariant is still present, the eject
  carries no `{%` marker, and a reviewer records the prose audit in `REPORT.md` —
  not on any measured token or line delta.
- The two determinism trims are in-scope behavior changes, bundled into Track 1's
  commit rather than deferred as risky.
- `speccy-decompose` absorbs the implementation plan's per-anchor edit detail and
  test-guardrail map into the `TASKS.md` task bodies, so each task is
  self-contained; `PROMPT-TOKEN-REDUCTION-PLAN.md` is then deleted within Track 1's
  final task, leaving no dangling file dependency.
- The plan's anchors are pre-edit locators. Track 2 tasks must re-locate against
  the Track-1-compressed wording at execution time, because earlier edits drift
  line numbers and phrasings.
- `just reeject` regenerates all three host packs from `resources/`; both commits
  run it and the full hygiene suite before landing.
- The `persona_snippets.rs` and `authoring_commit.rs` assertions are content
  assertions over rendered output, not authorship assertions, so consolidating a
  passage into an included module keeps them green as long as the expanded text
  still carries the asserted strings, headings, and include lines.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: Redundant SDLC-loop prose is removed, each fact preserved once

The module bodies under `resources/modules/**` that decomposition identifies as
redundant restatements or over-explanation are compressed so each operational fact
appears once at its canonical site. No passage that is a deliberate inline rule
followed by a pointer to a longer reference is reduced to the bare pointer
(SPEC-0049 DEC-002): the act-without-read property is preserved. The cut is prose
only — no rendered behavior changes except the two determinism trims specified in
REQ-002 and REQ-003. Among the cut passages are the three
`> Ported from the feature-dev …` provenance blockquotes (in `reviewer-correctness`,
`plan-architect`, `plan-explorer`; Track 1 anchors A6/C6); deleting them is
authorized by REQ-008, which retires the guardrail that previously asserted their
presence.

<done-when>
- Each passage enumerated in the Track 1 task bodies as redundant no longer appears
  in `resources/modules/**` or in the ejected `.claude/` / `.agents/` / `.codex/`
  packs.
- No inline-rule-plus-pointer flagged as "keep inline" is reduced to a bare
  pointer.
- A reviewer audit recorded in `REPORT.md` confirms that every cut passage's fact
  still appears once at a canonical site (no operational fact was lost).
- `cargo test --workspace` passes, including `persona_snippets.rs` and
  `authoring_commit.rs` with every asserted heading, include line, and command
  string still present in the rendered output.
</done-when>

<behavior>
- Given a reviewer persona body that restated a fact already carried by an included
  reference, when Track 1 lands, then the persona carries only the included
  reference's pointer and its own persona-specific content, and the rendered
  persona still contains every string `persona_snippets.rs` asserts.
- Given a one-sentence inline rule paired with a pointer to a long reference, when
  Track 1 lands, then the inline sentence remains (act-without-read), not just the
  pointer.
</behavior>

<scenario id="CHK-001">
Given the `resources/modules/**` tree and the ejected packs after Track 1,
when a reviewer audits the passages the Track 1 tasks enumerate as redundant,
then none of those passages appears in source or eject, no kept inline-rule-plus-
pointer was reduced to a bare pointer, every cut fact still appears once at a
canonical site, and the audit is recorded in `REPORT.md`.
</scenario>

<scenario id="CHK-002">
Given the post-Track-1 tree,
when `cargo test --workspace` runs (notably `persona_snippets.rs` and
`authoring_commit.rs`),
then all guardrail assertions — required headings, `{% include %}` lines, and
command strings in the rendered personas and recipes — pass.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: The work phase drops the post-append journal re-read

`phases/speccy-work.md` no longer instructs the agent to re-read the journal after
`speccy journal append` to confirm the new block landed. The CLI validates an
append before any write, so a malformed block can never land and the re-read
proves nothing; the procedure instead confirms `speccy next --json` shows no
consistency drift.

<done-when>
- The ejected work-phase body contains no instruction to re-read the journal after
  the append to verify the block.
- The body relies on the CLI's validate-before-write guarantee, retaining at most a
  `speccy next --json` consistency check.
- The trim is recorded in `REPORT.md` as a determinism change.
</done-when>

<behavior>
- Given the work phase has just appended an `<implementer>` block, when the
  procedure continues at HEAD after this SPEC, then it does not re-read the journal
  to confirm the append and instead reads `speccy next --json` for drift.
</behavior>

<scenario id="CHK-003">
Given the ejected `phases/speccy-work` body after Track 1,
when it is read,
then it contains no post-append journal re-read step, relies on the CLI's
validate-before-write guarantee, and the determinism trim is noted in `REPORT.md`.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: The ship phase drops two redundant `speccy status` round-trips

`phases/speccy-ship.md` no longer issues `speccy status SPEC-NNNN --json` in step 1
nor the post-flip `speccy status` re-check in step 3. Step 1 relies on the single
`speccy next --json` already run in "When to use" — which yields both readiness
(`next_action.kind == "ship"`) and `spec_md_path` / `tasks_md_path`. Step 3's
re-check is provably unnecessary: the state-flip is excluded from
`spec_hash_at_generation`, so `TSK-003` cannot fire.

<done-when>
- The ejected ship-phase step 1 issues no separate `speccy status SPEC-NNNN --json`
  call and instead uses the `speccy next --json` already run.
- The ejected ship-phase step 3 contains no post-flip `speccy status` re-check.
- Both removals are recorded in `REPORT.md` as determinism changes.
</done-when>

<behavior>
- Given the ship phase needs the spec/task paths and ship readiness, when step 1
  runs at HEAD after this SPEC, then it reads them from the existing
  `speccy next --json` result rather than a new `speccy status` call.
- Given the ship phase has flipped the spec state, when step 3 runs, then it does
  not re-check `speccy status` for a `TSK-003` mismatch.
</behavior>

<scenario id="CHK-004">
Given the ejected `phases/speccy-ship` body after Track 1,
when it is read,
then step 1 contains no `speccy status SPEC-NNNN --json` call (deriving readiness
and paths from `speccy next --json`), step 3 contains no post-flip `speccy status`
re-check, and both removals are recorded in `REPORT.md`.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Each duplicated passage is one canonical module that expands faithfully

Each of the six enumerated duplicated passages — the retry-shape rule summary, the
reviewer Role tail, the vet input-resolution block, the vet no-rollback note, the
inline-fanout rationale, and the plan+amend self-review core — is consolidated into
a single new shared module under `resources/modules/**` and pulled in with
`{% include %}` at every former callsite, with no inline copy of the consolidated
text remaining at any non-canonical callsite. Each new module expands at eject time
to text equivalent to the copy it replaced; where the passage is a one-liner plus a
pointer, the module expands to the full inline sentence, never a bare pointer
(SPEC-0049 DEC-002). `speccy-brainstorm`'s self-review is not folded in — its four
artifact-oriented properties are structurally distinct.

<done-when>
- Six new shared modules exist, one per enumerated passage, each `{% include %}`d
  at every callsite that previously inlined it.
- No non-canonical callsite still carries an inline copy of any consolidated
  passage.
- After `just reeject`, the ejected packs' only change versus their pre-Track-2
  content is include-expansion whose expanded text equals the prior inline text;
  no operational fact changed.
- Each one-liner-plus-pointer module expands to its full inline sentence, not a
  bare pointer.
- `cargo test --workspace` (notably `persona_snippets.rs` and `authoring_commit.rs`)
  stays green, and the ejected packs contain no `{%` marker.
</done-when>

<behavior>
- Given a passage that was verbatim in N callsites, when Track 2 lands, then the
  text lives in one module `{% include %}`d from all N callsites and the rendered
  output at each callsite is byte-equivalent to its pre-Track-2 rendering.
- Given the retry-shape one-liner-plus-pointer, when its module is ejected, then the
  full inline sentence is present at the callsite, preserving act-without-read.
</behavior>

<scenario id="CHK-005">
Given the `resources/modules/**` tree after Track 2,
when a reviewer inspects each of the six enumerated passages,
then each exists as exactly one new module, every former callsite pulls it via
`{% include %}`, and no inline copy of the consolidated text remains at any
non-canonical callsite.
</scenario>

<scenario id="CHK-006">
Given a `just reeject` run after Track 2,
when the ejected packs are diffed against their pre-Track-2 content,
then the only differences are include-expansions whose expanded text equals the
prior inline text, every one-liner-plus-pointer module expands to its full inline
sentence rather than a bare pointer, and no ejected file contains a `{%` marker.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: A decision supersedes SPEC-0034 DEC-001 and brainstorm stays independent

This SPEC carries a `<decision>` that supersedes SPEC-0034 DEC-001, authorizing the
shared plan+amend self-review extraction (REQ-004). The new self-review-core module
carries a comment referencing the supersession. `speccy-brainstorm`'s self-review
remains an independent inline copy. The now-stale
`<!-- … independent copy … per DEC-001 / OQ-b … -->` comments are removed from
`speccy-plan`, `speccy-amend`, and `speccy-brainstorm`.

<done-when>
- This SPEC's `### Decisions` contains a `<decision>` superseding SPEC-0034 DEC-001
  with its rationale.
- The new self-review-core module carries a comment naming the supersession.
- `speccy-brainstorm` retains its own self-review (not folded into the shared
  module).
- The stale DEC-001 / OQ-b comments are absent from `speccy-plan`, `speccy-amend`,
  and `speccy-brainstorm`.
</done-when>

<behavior>
- Given the shared self-review-core module is created, when this SPEC is read, then
  a `<decision>` records the supersession of SPEC-0034 DEC-001 and the module
  references it.
- Given Track 2 lands, when `speccy-brainstorm` is read, then its self-review is
  still present inline and structurally distinct from the shared core.
</behavior>

<scenario id="CHK-007">
Given this SPEC's SPEC.md and the `resources/modules/**` tree after Track 2,
when both are read,
then SPEC.md carries a `<decision>` superseding SPEC-0034 DEC-001, the new
self-review-core module carries the supersession comment, `speccy-brainstorm`
retains its independent self-review, and no `<!-- … DEC-001 / OQ-b … -->` comment
remains in `speccy-plan`, `speccy-amend`, or `speccy-brainstorm`.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Both commits keep the guardrail suite green

Each of the two commits leaves the full hygiene suite passing — the guardrail
tests, lints, formatting, and dependency check all stay green, so the prose and
factoring edits introduce no regression in the rendered output the tests assert
over. The sole authorized guardrail change in this SPEC is REQ-008's retirement of
the brittle `feature-dev` substring assertion in `skill_packs.rs`; every other
guardrail assertion stays intact, and the suite stays green once that assertion is
removed.

<done-when>
- At each commit, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`, and `cargo deny check` all pass.
- `persona_snippets.rs` and `authoring_commit.rs` pass at each commit with their
  asserted headings, include lines, and command strings intact.
</done-when>

<behavior>
- Given Track 1 (then Track 2) edits to `resources/modules/**`, when `just reeject`
  and the hygiene suite run, then all four checks pass.
</behavior>

<scenario id="CHK-008">
Given each of the two commits,
when `cargo test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and
`cargo deny check` run,
then all four pass.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Both commits leave the ejected packs clean and mirroring the source

Each commit's eject is structurally intact: no `{%` MiniJinja marker survives into
any ejected file, and the ejected-pack diff corresponds to that commit's
`resources/modules/**` edits — 1:1 text edits in Track 1, include-expansion in
Track 2. A larger-than-expected eject diff signals a broken wrapper or template
variable and must be investigated before the commit lands.

<done-when>
- No ejected file under `.claude/`, `.agents/`, or `.codex/` contains a `{%`
  marker after either reeject.
- Each commit's ejected-pack diff corresponds to its `resources/modules/**` edits
  (1:1 in Track 1; include-expansion in Track 2), with no unexplained changes.
</done-when>

<behavior>
- Given an ejected pack after either reeject, when scanned for `{%`, then no match
  is found.
- Given the eject diff after either commit, when compared to that commit's source
  edits, then it mirrors them with no unexplained change.
</behavior>

<scenario id="CHK-009">
Given the ejected `.claude/`, `.agents/`, and `.codex/` packs after each reeject,
when scanned,
then no ejected file contains a `{%` marker and the eject diff mirrors the
`resources/modules/**` edits for that commit (1:1 for Track 1; include-expansion
for Track 2).
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: The brittle `feature-dev` attribution guardrail is retired

The `feature_dev_personas_declare_speccy_model_conventions_and_attribution` test in
`speccy-cli/tests/skill_packs.rs` asserts `persona_body.contains("feature-dev")`
over three persona bodies — a substring match against human-curated prose, the
anti-pattern AGENTS.md test hygiene forbids, since it gates an editorial provenance
line and breaks on any legitimate rewrite. That substring assertion is removed and
the test renamed to drop the `_and_attribution` suffix; its structural
model-convention assertions (`model: opus[1m]`, `model = "gpt-5.5"`, no `sonnet`)
are retained, since those check frontmatter structure — a stable surface.
Attribution provenance is henceforth editorial-only, not test-gated. Removing this
assertion is what makes REQ-001's deletion of the three `feature-dev` provenance
blockquotes satisfiable without reddening the suite.

<done-when>
- The `persona_body.contains("feature-dev")` assertion is absent from
  `speccy-cli/tests/skill_packs.rs`.
- The test is renamed to drop `_and_attribution`; its model-convention assertions
  remain.
- `cargo test --workspace` passes with the assertion removed and the three
  provenance blockquotes deleted.
- The retirement is recorded in `REPORT.md`.
</done-when>

<behavior>
- Given the three `feature-dev` provenance blockquotes are deleted, when
  `cargo test --workspace` runs, then no assertion fails on the absent `feature-dev`
  substring, because that assertion no longer exists.
</behavior>

<scenario id="CHK-010">
Given `speccy-cli/tests/skill_packs.rs` after the reconciliation task lands,
when the test file is read and `cargo test --workspace` runs,
then the `persona_body.contains("feature-dev")` assertion is absent, the test name
no longer carries `_and_attribution`, the model-convention assertions remain, and
the suite passes with the three provenance blockquotes deleted from source and
eject.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
**Extract the shared plan+amend self-review core; supersede SPEC-0034 DEC-001.**

SPEC-0034 DEC-001 deliberately kept two inline copies of the self-review core
*"until both templates have stabilized,"* and its OQ-b explicitly left it open to
re-evaluate the duplication weight during a later implementation — i.e. it
pre-authorized a clean extraction once the lists stopped moving. That trigger is
now met: the six shared check properties have stayed verbatim-identical across the
specs authored since, so the divergence DEC-001 hedged against never materialized.
Extracting now *executes DEC-001's deferred follow-up* rather than reversing it; it
removes the two-copy drift risk DEC-001 named as the accepted cost, and is cheap to
undo (pull one property back inline) should plan and amend ever need to diverge.
`speccy-brainstorm` stays independent — DEC-001 was right that its four
artifact-oriented properties are structurally different and no partial covers them.
This is the lowest-priority, token-neutral item in the SPEC; its value is one
source of truth, not runtime tokens.
</decision>

<decision id="DEC-002">
**Retire the brittle `feature-dev` attribution guardrail; attribution is editorial.**

Track 1 anchors A6/C6 delete the three `> Ported from the feature-dev …` provenance
blockquotes, but the SPEC-0053 guardrail
`feature_dev_personas_declare_speccy_model_conventions_and_attribution`
(`speccy-cli/tests/skill_packs.rs`) asserts `persona_body.contains("feature-dev")`,
so deleting the lines reds the suite — an unresolvable conflict between REQ-001 and
REQ-006 that the pre-ship vet gate surfaced. The substring assertion is itself the
fault: AGENTS.md test hygiene explicitly forbids substring-matching human-curated
prose because it gates editorial decisions and breaks on legitimate rewrites. We
retire that assertion (REQ-008) rather than preserve the provenance lines to satisfy
it. Attribution provenance is editorial-only henceforth — kept in prose where a
human finds it useful, surfaced by review if it ever matters, never machine-gated.
The test's structural model-convention assertions are sound (they check frontmatter,
a stable surface) and stay. We deliberately do not replace the check with a
frontmatter `derived_from` field: that would add behavior-neutral metadata to bodies
that reload into agent context every turn, working against this SPEC's
token-reduction intent.
</decision>

## Notes

**One SPEC, two task groups.** Track 1 (token reduction) lands as commit 1 and
Track 2 (SSOT factoring) as commit 2 on the same branch; Track 2 tasks depend on
Track 1 tasks. Rejected framings: *two separate SPECs* (Track 2 factors
Track-1-compressed text, so two specs force a serialization gap with no isolation
benefit) and *Track 1 only* (AGENTS.md calls inlined copies shadowing a canonical
source "bugs," so Track 2 is bug-fixing, not hypothetical-audience feature-creep).

**Outcome-framed, not tier-framed.** The plan orders its edits by ROI into Tiers
A–E (Track 1) and T2-A–F (Track 2). Those tiers map to `TASKS.md` tasks; the
requirements above instead capture the guarantees that must hold when the work is
done. The per-anchor edit detail and test-guardrail map are absorbed into the task
bodies during decomposition, so `PROMPT-TOKEN-REDUCTION-PLAN.md` is a transient
decompose input deleted within Track 1, not a durable SPEC dependency.

**No token-percentage acceptance.** A measured token or line-count target was
deliberately not made a criterion: the plan itself notes that factoring (Track 2)
saves zero runtime tokens because includes expand at eject time, and that
line-count is only a proxy. The verifiable contract is named-passages-absent,
invariants-green, and eject-clean.

**SPEC-0049 DEC-002 binds Track 2.** An inline one-sentence rule paired with a
pointer to a longer canonical reference is a deliberate act-without-read
affordance. Every consolidated one-liner module in Track 2 must expand to the same
inline sentence, never collapse to a bare pointer.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-12 | Kevin Xiao | Initial SPEC: Track 1 compresses SDLC-loop prose and defers two procedures (work re-read, ship status round-trips) to existing CLI guarantees; Track 2 factors six duplicated passages into shared modules and supersedes SPEC-0034 DEC-001. |
| 2026-06-12 | Kevin Xiao | Amend: resolve the REQ-001/REQ-006 contradiction the pre-ship vet gate caught — add REQ-008 retiring the brittle SPEC-0053 `feature-dev` substring guardrail in `skill_packs.rs`, record DEC-002 (attribution is editorial-only, not test-gated), and cross-reference it from REQ-001/REQ-006 so deleting the three `feature-dev` provenance blockquotes (Track 1 anchors A6/C6) keeps the hygiene suite green. |
</changelog>
