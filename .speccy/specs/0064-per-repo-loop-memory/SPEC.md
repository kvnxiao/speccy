---
id: SPEC-0064
slug: per-repo-loop-memory
title: Per-repo loop memory — an eject-safe `.speccy/MEMORY.md` the implementer reads before acting, grown by a ship-time retro from the loop's own conventions and mistakes
status: in-progress
created: 2026-06-13
supersedes: []
---

# SPEC-0064: Per-repo loop memory — an eject-safe `.speccy/MEMORY.md` the implementer reads before acting, grown by a ship-time retro from the loop's own conventions and mistakes

## Summary

Speccy makes drift loud, but it has no way to *carry forward* what a repo
learns about itself. Conventions an implementer rediscovers each spec, and
the friction the loop hits — repeated blocking review verdicts, retry rounds,
recurring `<blockers>` — evaporate when a spec ships. The next spec starts
cold and the same mistakes recur. The mission-orchestration model this SPEC
draws from treats project-level skills as the learning layer: a worker that
hits friction edits durable guidance, and future workers inherit the fix, so a
run doesn't degrade as it lengthens. Speccy cannot port that literally —
skill bodies are ejected output that `speccy init --force` / reeject
overwrites — so the learned content must live in a **repo-owned surface the
eject pipeline never touches**, while the *mechanism* that reads and grows it
ships in the (ejected) skill prose.

This SPEC adds a two-tier per-repo memory. The new tier is a working ledger at
`.speccy/MEMORY.md` — user-owned, git-tracked, never enumerated by `speccy
init` (the same "never overwritten" bucket as the existing `.speccy/BACKLOG.md`).
The durable tier is the surface the repo already has: `AGENTS.md` and rule
files, which every subagent's host already auto-loads. The loop grows the
ledger from two feeds through one pipeline — conventions it follows and
mistakes it makes — at a **ship-time retro** that mines the same REPORT.md /
journal evidence already produced at ship. The retro also consolidates stable
entries up into the durable tier (human-gated) and retires entries that point
at code the repo no longer has, so the ledger stays bounded and never feeds a
phantom forward. On the read side, the **implementer** loads the relevant
slice of the ledger before writing code; reviewers and vet deliberately do not
read it, preserving adversarial freshness. The whole feature is soft guidance —
context injection only, never a `speccy verify` gate.

The bet on capture: molding is **incremental**, never a brownfield
detection pass at init. An agent reading the code it is already working on
understands the repo's conventions for free, in any language or framework; a
detection pass would need per-stack heuristics and would optimize prematurely
for value that compounds anyway as the loop runs.

## Goals

<goals>
- A repo-owned `.speccy/MEMORY.md` ledger that `speccy init --force` /
  reeject leaves byte-identical, so learned content survives speccy CLI
  updates.
- The implementer loads the relevant slice of the ledger before writing
  code, so a recorded convention or past mistake is avoided rather than
  re-flagged downstream.
- A ship-time retro grows the ledger from the loop's own evidence
  (conventions followed, blocking verdicts, retry rounds, blockers) using one
  capture pipeline for both feeds.
- The retro consolidates stable entries into the durable tier
  (`AGENTS.md` / rules) under human approval and retires phantom-referencing
  entries, keeping the ledger bounded.
- The memory is soft: a malformed or absent `.speccy/MEMORY.md` produces no
  `speccy verify` error or warning.
</goals>

## Non-goals

<non-goals>
- No `speccy memory` CLI verb in this SPEC. Capture, slicing, and
  consolidation are prose-layer behaviours; the CLI gains nothing here. A
  verb is a documented follow-up (see Notes), not v1.
- No init-time convention detection. Nothing scans the repo at `speccy init`
  to seed the ledger; molding is incremental through the loop only.
- No freshness-hashing / inputs-verification engine. Phantom-reference hygiene
  is a retro-time judgment, not a CLI mechanism — this is the previously-cut
  "Check inputs and freshness hashing" feature and stays cut.
- No ingestion of pre-existing standards docs. A repo that documents its
  standards points `AGENTS.md` at them as it already would; speccy grows no
  surface to discover or copy them.
- No new lint family, no enforcement, no `--strict` coupling. Memory never
  blocks a ship.
- Reviewers and vet do not read the ledger. No feed-forward attach point is
  added to the reviewer or vet bodies.
</non-goals>

## User Stories

<user-stories>
- As a solo developer dogfooding speccy across many specs in one repo, I want
  the loop to remember the conventions and mistakes of *this* repo, so later
  specs stop relitigating decisions earlier ones already settled.
- As an implementer subagent starting a task, I want the repo's accumulated
  guidance in context before I write code, so I follow the established pattern
  the first time instead of getting flipped back in review.
- As a maintainer upgrading the speccy CLI and re-ejecting the skill pack, I
  want my repo's learned memory to survive untouched, so an upgrade never
  costs me the project's accumulated knowledge.
- As a reviewer persona, I want to judge the diff without the implementer's
  accumulated rationalizations injected into my context, so my review stays
  adversarial.
</user-stories>

## Assumptions

<assumptions>
- "Learn" means textual heuristics injected into agent context — not
  fine-tuning, embeddings, or a RAG/vector store. Every requirement here is
  markdown an agent reads.
- Capture and feed-forward live entirely in the skill/prose layer; the CLI
  stays mechanical and invokes no model. No CLI change is in scope.
- The ledger is git-tracked, so every memory mutation is reviewable in the PR
  that produced it.
- Stale guidance is worse than none (the rationale that cut input-freshness
  hashing), so retire/supersede is in scope now, not deferred.
- Convention-molding and mistake-learning share one entry format and one
  ledger; they differ only in feed source, not in shape or storage.
- The durable tier (`AGENTS.md` / rules) is already auto-loaded by every
  subagent's host, so only the new ledger needs a new read instruction.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: `.speccy/MEMORY.md` is a repo-owned ledger the eject pipeline never overwrites

The working tier of memory is a single file at `.speccy/MEMORY.md`, a sibling
of `.speccy/BACKLOG.md`. `speccy init` does not enumerate or scaffold it (it
falls in the same user-authored, never-planned-against bucket as `BACKLOG.md`),
so neither a first `init` nor `init --force` / `just reeject` creates, edits,
or deletes it. The file is absent until the loop first writes it; its absence
is normal and silent.

<done-when>
- `speccy init --force --host claude-code` run in a repo that already has a
  non-empty `.speccy/MEMORY.md` leaves that file byte-identical.
- `speccy init` in a repo without `.speccy/MEMORY.md` does not create one.
- The file's canonical path and its user-owned/never-overwritten status are
  documented where the `.speccy/` layout is described.
</done-when>

<behavior>
- Given a repo with a populated `.speccy/MEMORY.md`, when the full eject runs
  for every shipped host, then no host's file-classification pass plans a
  Create, Conflict, or overwrite against it.
- Given a fresh repo, when `speccy init` runs, then `.speccy/MEMORY.md` does
  not appear.
</behavior>

<scenario id="CHK-001">
Given a repo with `.speccy/MEMORY.md` holding arbitrary non-empty content,
when `speccy init --force --host claude-code` and `--host codex` both run,
then `.speccy/MEMORY.md` is byte-identical before and after (verified by a
hash comparison in a test that seeds the file and runs init), proving the
ledger sits outside the set of files the eject pipeline enumerates.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: A memory entry carries trigger, content, corrective rule, and provenance

Every ledger entry encodes four parts so it is actionable and auditable: a
**trigger** (when the entry applies — a task area, file region, or situation),
the **convention or mistake** it records, the **corrective rule** to follow,
and **provenance** naming the SPEC / task / review that produced it. The format
is a documented prose/markdown convention (no parser, no CLI grammar in v1).
Convention-flavoured and mistake-flavoured entries share this one shape.

<done-when>
- The entry shape (the four parts above) is documented in a shipped reference
  the capture and feed-forward prose both point at.
- A retro-written entry, inspected during dogfooding, carries all four parts.
- Provenance references a real SPEC/task/review identifier, not a fabricated
  one.
</done-when>

<behavior>
- Given the loop captures a lesson from a blocking verdict, when the entry is
  written, then it names the SPEC/task whose review produced it in its
  provenance.
- Given a convention the implementer followed, when it is captured, then it is
  stored in the same four-part shape as a mistake entry, distinguished only by
  feed source.
</behavior>

<scenario id="CHK-002">
Given this SPEC (or a later one) ships through the loop with at least one
recorded friction signal,
when the ship-time retro writes a `.speccy/MEMORY.md` entry,
then manual inspection of that entry confirms it contains a trigger, the
convention/mistake, a corrective rule, and provenance resolving to the
producing SPEC/task — a dogfood check, since entry quality is a semantic
property no prose-substring test can gate.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: The implementer loads the relevant ledger slice before writing code

The implementer body instructs the agent, before the reuse survey and any
code write, to read `.speccy/MEMORY.md` when present and load the slice
relevant to the current task's area — mirroring the existing "load the
relevant slice, drill in on demand" shape the journal context bundle already
uses. When the file is absent the step is a silent no-op. The instruction
lives only in the canonical module body (`resources/modules/phases/speccy-work.md`)
and reaches every host via the existing `{% include %}`; no copy is inlined
into a host wrapper.

<done-when>
- The feed-forward instruction is present in the canonical implementer module
  and absent as any duplicated inline copy in host wrappers.
- After `just reeject`, the ejected implementer agent body carries the
  instruction with no Conflict against the rendered module.
- With no `.speccy/MEMORY.md` present, the implementer proceeds without error
  or comment about memory.
</done-when>

<behavior>
- Given a `.speccy/MEMORY.md` whose entry's trigger matches the task area,
  when the implementer runs, then it reads that slice before writing code and
  its implementation follows the recorded corrective rule.
- Given no ledger file, when the implementer runs, then behaviour is identical
  to today's loop.
</behavior>

<scenario id="CHK-003">
Given `.speccy/MEMORY.md` seeded with one convention entry whose trigger
matches a task's area,
when the implementer subagent is run on that task during dogfooding,
then its produced diff conforms to the recorded convention without the
convention being restated in the task prompt — a dogfood check that the read
actually changes behaviour.
</scenario>

<scenario id="CHK-004">
Given the canonical implementer module and the ejected host bodies after
`just reeject`,
when the source-of-truth placement is checked,
then the feed-forward block exists once in
`resources/modules/phases/speccy-work.md` and no shadowing inline copy exists
in any `resources/agents/.<host>/` wrapper — a structural check against the
no-duplicate-snippet invariant.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: A ship-time retro captures both conventions and mistakes from the loop's own evidence

The ship phase gains a retro step at the REPORT.md write boundary (before the
ship commit) that distills the just-completed loop into ledger entries. It
mines the evidence already on disk — REPORT.md coverage, the per-task journal
(`<blockers>`, review verdict flips, retry rounds), and the spec diff — and
appends convention and/or mistake entries to `.speccy/MEMORY.md`, or records
that the loop yielded no durable lesson. Appends follow a one-entry-per-write
discipline so the prose-layer write stays serial. Capture happens only here,
not continuously mid-loop, so it cannot race the parallel-review shared tree
and so cross-spec patterns can aggregate before being recorded.

<done-when>
- The ship phase module includes a retro step positioned at the REPORT.md
  write boundary, before the commit that bundles the loop's changes.
- The retro draws on REPORT.md / journal / diff evidence, not on a separate
  re-derivation of the work.
- A loop with recorded friction yields at least one mistake-flavoured
  candidate entry; a loop with none records "no durable lesson" explicitly.
- The resulting ledger mutation lands in the same ship commit as REPORT.md.
</done-when>

<behavior>
- Given a shipped spec whose journal shows ≥1 blocking verdict or retry round,
  when the retro runs, then it produces ≥1 mistake-flavoured entry citing that
  evidence.
- Given a clean loop with no friction and no new convention, when the retro
  runs, then it writes no entry and says so, rather than inventing one.
</behavior>

<scenario id="CHK-005">
Given a spec shipped through the loop whose journal contains at least one
blocking-then-passed review round,
when the ship-time retro runs during dogfooding,
then `.speccy/MEMORY.md` gains an entry whose provenance cites that round and
whose corrective rule addresses the cause — a dogfood check that the retro
turns real friction into a durable lesson.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: The retro consolidates stable entries into the durable tier and dedups, keeping the ledger bounded

The same retro step bounds the ledger so it does not grow monotonically. It
proposes promoting stable, repeatedly-affirmed entries up into the durable
tier (`AGENTS.md` / rules) for **human approval**; on approval the entry moves
to the durable tier and is removed from the ledger. It dedups candidates both
within the ledger and against the repo's existing durable docs, so guidance
already stated in `AGENTS.md`, a rule file, or whatever the repo points at is
not re-recorded. Promotion never happens silently.

<done-when>
- The retro proposes ledger→durable promotion of stable entries and requires
  human approval before the durable-tier edit lands.
- A promoted entry is removed from `.speccy/MEMORY.md` so it is not stored
  twice.
- A candidate already covered by an existing durable doc is dropped, not
  appended.
- Promotion and consolidation are human-gated, not automatic.
</done-when>

<behavior>
- Given a ledger entry affirmed across multiple specs, when the retro runs,
  then it surfaces a promotion proposal and, only after approval, edits the
  durable tier and deletes the ledger entry.
- Given a candidate lesson already documented in `AGENTS.md`, when the retro
  evaluates it, then it is deduped away rather than added to the ledger.
</behavior>

<scenario id="CHK-006">
Given a `.speccy/MEMORY.md` containing a stable entry plus a candidate already
covered by `AGENTS.md`,
when the retro runs during dogfooding,
then the stable entry is offered for human-gated promotion (and on approval
leaves the ledger) while the already-covered candidate is dropped — a dogfood
check that consolidation and dedup keep the ledger bounded.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: The retro retires entries that reference code the repo no longer has

To prevent the ledger from feeding phantom constructs forward as the repo is
refactored, the retro re-validates entries against the current tree and
retires or rewrites any whose referenced construct is gone. The defense is
authoring discipline (prefer abstract, convention-level wording over fragile
code coordinates) plus this retro-time garbage-collection — a semantic
judgment, deliberately not a CLI freshness check. The only structurally
checkable slice of hygiene (dangling SPEC/task provenance) is noted as the
sole part a future CLI could ever validate.

<done-when>
- The retro re-validates ledger entries against the current tree and
  retires/rewrites phantom-referencing ones.
- Entry authoring guidance prefers abstract convention wording over fragile
  code coordinates.
- No CLI freshness-hashing mechanism is added.
</done-when>

<behavior>
- Given a ledger entry referencing a module that a later spec deleted, when
  the retro runs, then that entry is retired or rewritten so it is not fed
  forward.
- Given an abstractly-worded convention entry that names no specific
  construct, when the tree changes, then it survives unchanged.
</behavior>

<scenario id="CHK-007">
Given a `.speccy/MEMORY.md` entry whose provenance or referenced construct no
longer resolves in the tree,
when the retro runs during dogfooding,
then that entry is retired or rewritten and is not present in the slice the
next implementer loads — a dogfood check that hygiene prevents phantom
feed-forward.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Memory is soft guidance — never a `speccy verify` gate

The ledger participates in no lint family and gates nothing. `speccy verify`
does not read `.speccy/MEMORY.md`, so a malformed, empty, or absent ledger
produces no error or warning and never fails CI. Memory influences agents only
by being read into context; it has no enforcement surface.

<done-when>
- `speccy verify` emits no memory-related error, warning, or info code for any
  state of `.speccy/MEMORY.md` (malformed, empty, or absent).
- No new lint code references the ledger.
</done-when>

<behavior>
- Given a syntactically garbage `.speccy/MEMORY.md`, when `speccy verify`
  runs, then it passes exactly as if the file were absent.
</behavior>

<scenario id="CHK-008">
Given a repo whose `.speccy/MEMORY.md` contains deliberately malformed
content,
when `speccy verify --json` runs,
then its lint output contains no error, warning, or info entry attributable to
the ledger — a structural check that memory has no enforcement surface.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
Two-tier memory — an eject-safe working ledger (`.speccy/MEMORY.md`)
consolidated into the durable tier (`AGENTS.md` / rules) — is chosen over two
rejected framings. **Durable-only** (everything proposed straight into
`AGENTS.md`) gives noisy cross-spec mistake patterns no place to aggregate
before they are confident, so each transient observation either pollutes the
always-loaded north star or is dropped before the pattern emerges.
**Ledger-only** (one growing file, no promotion) becomes a second, monotonically
growing source of truth competing with `AGENTS.md`. The two-tier model is a
working-vs-long-term-memory split: stable knowledge graduates out and noise
expires, which is what keeps the ledger bounded (REQ-005).
</decision>

<decision id="DEC-002">
The ledger lives at `.speccy/MEMORY.md`, a top-level sibling of
`.speccy/BACKLOG.md`. That precedent is decisive: `BACKLOG.md` is user-authored,
git-tracked, and never enumerated by `speccy init` or any resource template, so
a memory file in the same location inherits "never overwritten by reeject" for
free with no new carve-out in the init file-classification logic.
</decision>

<decision id="DEC-003">
Molding is incremental only; there is no init-time convention-detection pass.
A detection pass would need per-language/framework heuristics to infer "good
conventions"; incremental capture inherits the agent's in-context understanding
of the code it is already reading, for free and language-agnostically. Cost
accepted: brownfield value is delayed because the ledger starts empty and fills
as the loop runs. This is a deliberate bet that compounding capture plus
improving models beats a brittle up-front scan.
</decision>

<decision id="DEC-004">
Capture happens at a single ship-time retro, not continuously mid-loop. A
mid-loop write would reintroduce the parallel-review shared-tree mutation race
(siblings poisoning each other's source). Deferring to the ship boundary also
lets cross-spec patterns aggregate before being recorded, and reuses the
REPORT.md / journal evidence already produced at ship as source material. One
retro mines both feeds — conventions followed and mistakes made — at once.
</decision>

<decision id="DEC-005">
Feed-forward is implementer-only; reviewers and vet do not read the ledger.
The durable tier (`AGENTS.md` / rules) is already auto-loaded by every
subagent's host, so only the new ledger needs a new read — and confining that
read to the implementer keeps reviewers adversarial. Injecting accumulated
"we decided X is fine" lessons into a reviewer hands it the implementer's
rationalizations and blunts the review. The implementer fixing the convention
upstream is what prevents churn; the reviewer needs nothing extra.
</decision>

<decision id="DEC-006">
Consolidation into the durable tier is human-gated, never automatic. Promotion
edits `AGENTS.md` / rules — the always-loaded north star and conventions — so a
human approves each graduation. This keeps machine-proposed lessons from
silently mutating hand-authored intent.
</decision>

<decision id="DEC-007">
Phantom-reference hygiene (REQ-006) is a retro-time semantic judgment plus
abstract-authoring discipline — explicitly not a CLI freshness-hashing engine.
That engine is the previously-cut "Check inputs and freshness hashing" feature,
cut because "wrong inputs poison the model worse than no inputs." Building it
here would re-add the cut surface and force every entry into fragile,
machine-checkable code coordinates. The only structurally checkable slice
(dangling SPEC/task provenance) is the sole part a future CLI verb could ever
validate; semantic staleness stays the retro's job.
</decision>

<decision id="DEC-008">
v1 is prose plus a file convention: skills Read and edit `.speccy/MEMORY.md`
directly at a fixed canonical path, with a one-entry-per-write discipline to
keep appends serial. A `speccy memory show/append/list/lint` CLI verb — the
twin of `speccy journal` — is deferred (see Notes) rather than built now,
because the entry grammar is newly invented and hardening an unvalidated grammar
into the deterministic core is premature. The journal itself graduated from
hand-managed markdown to a CLI verb only after its shape proved out; memory
follows the same path.
</decision>

<decision id="DEC-009">
Verification is structural invariants plus dogfooding scenarios; no scenario
asserts that specific sentences appear in any skill or subagent body.
Substring-matching curated prose gates editorial decisions and breaks on
legitimate rewrites (the project's standing test-hygiene rule). So Checks are
limited to (a) genuine structural invariants — init leaves the ledger
byte-identical (CHK-001), the feed-forward block has no shadowing inline copy
(CHK-004), `speccy verify` emits no memory lint (CHK-008) — and (b) dogfooding
Given/When/Then scenarios validated by running the loop and inspecting real
output (CHK-002/003/005/006/007). The real test of effectiveness is dogfooding.
</decision>

## Notes

Rejected framings (durable-only, ledger-only) are recorded in DEC-001.

**Deferred `speccy memory` CLI verb.** A mechanical surface mirroring
`speccy journal` — `memory show [--role] [--scope] [--json]` (deterministic
sliced read), `memory append` (lock-serialized write), `memory list --json`,
and a shape/dangling-provenance `memory lint` foldable into `speccy verify` —
is a plausible follow-up. The append-serialization it would buy is a real
win over the prose-layer one-entry-per-write discipline. Promote it to its own
SPEC only once **(a)** the entry grammar has survived roughly 5–10 specs of
dogfooding unchanged and **(b)** the prose-layer append race actually bites —
the same two-condition trigger by which `journal` graduated.

**Acknowledged tension with the cut inputs feature.** `.speccy/MEMORY.md` is an
inputs mechanism, and speccy cut input-freshness hashing because wrong inputs
poison worse than none. The design answers the poisoning risk without the cut
machinery: guidance is soft (REQ-007), sliced and implementer-only on read
(DEC-005), GC'd for phantoms (REQ-006), and human-gated on promotion (DEC-006).
Reviewers should confirm these defenses hold rather than treating the ledger as
free of that risk.

**Accepted v1 softness.** Without a CLI verb, "load the relevant slice" is the
implementer's own judgment over the ledger file, and "bounded" rests on retro
discipline rather than a hard cap. Both are accepted for v1 and are the first
things the deferred CLI verb would harden. Whether the single-file ledger
should split into per-area files as it grows is also a follow-up, not v1.

**Scope boundary.** Changes land in `resources/modules/` (the implementer phase
body, the ship phase body, and a new shared reference documenting the entry
shape and ledger location) plus the `.speccy/` layout docs. The speccy Rust CLI
is not touched.

## Open Questions

None — all framing questions (memory tier model, incremental vs init-detection,
capture trigger, consolidation gate, read scope, hygiene mechanism, CLI-vs-prose,
pre-existing-docs handling) were resolved during brainstorm and are recorded as
DEC-001 through DEC-009.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-13 | Kevin Xiao | Initial SPEC: per-repo two-tier loop memory — eject-safe `.speccy/MEMORY.md` ledger (REQ-001) with a four-part entry shape (REQ-002), implementer-only feed-forward (REQ-003), a ship-time retro that captures both conventions and mistakes (REQ-004), consolidates into the durable tier and dedups (REQ-005), and retires phantom references (REQ-006); all soft, never a verify gate (REQ-007). |
</changelog>
