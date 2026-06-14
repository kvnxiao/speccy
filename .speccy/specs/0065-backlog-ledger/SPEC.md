---
id: SPEC-0065
slug: backlog-ledger
title: Backlog ledger — a convention-only `.speccy/BACKLOG.md` register of future-spec candidates
status: in-progress
created: 2026-06-13
supersedes: []
---

# SPEC-0065: Backlog ledger — a convention-only `.speccy/BACKLOG.md` register of future-spec candidates

## Summary

Speccy already ships one user-owned, git-tracked ledger — `.speccy/MEMORY.md`,
the per-repo loop memory of conventions and mistakes (shape in
`resources/modules/references/memory-ledger.md`). It has no companion register
for the other kind of durable signal a loop produces: "this deserves to become
its own future SPEC." Today those candidates scatter across spec-local surfaces
— SPEC `## Non-goals`, `<decision status="deferred">`, REPORT
`<coverage result="deferred">`, and free prose — none of which is a repo-wide
list a planner can read when choosing the next slice, and none of which outlives
the single spec that produced it.

This SPEC introduces `.speccy/BACKLOG.md`: a sibling to `MEMORY.md` holding
future-spec candidates as a flat list of four-field entries. It is
**convention-only** — the Rust CLI never reads, parses, lints, or verify-gates
it, exactly as it ignores `MEMORY.md`. The whole feature is a new reference
module documenting the entry shape and file header, plus skill-body wiring: the
planning skills read the backlog as candidate slices and append to it when
scope is deliberately cut; ship mirrors the future-spec-worthy subset of its
deferred-work section into it; promoting an item into a new SPEC strikes the
entry; and bootstrap names the file in the conventions block so agents discover
it. No Rust changes ship in this SPEC beyond confirming `speccy init`/reeject
leave the user-owned file untouched.

Producers are kept deliberately narrow — plan/brainstorm, ship, and manual edits
only. Review and vet are excluded: review fires per task and would turn the
backlog into a dumping ground, and vet's findings resolve to fix-or-amend and
would double-capture with the adjacent ship step. A backlog entry is a coarse,
repo-level intent and should be rare; a spec that spawns many of them is itself
a signal it was not atomic enough.

## Goals

<goals>
- A future-spec candidate can be recorded, read by the planning skills, and
  retired on promotion entirely through convention, with no `speccy` subcommand
  involved at any step.
- `.speccy/BACKLOG.md` mirrors `MEMORY.md`'s lifecycle: user-owned,
  git-tracked, absent until first written, and never created or overwritten by
  `speccy init`, `speccy init --force`, or reeject.
- Every backlog entry follows one shipped four-field shape — Title /
  What & why / Deferred-because / Provenance — documented in a reference module
  the analog of `memory-ledger.md`.
- The set of producers is exactly plan/brainstorm, ship, and manual edits;
  review and vet never append.
- An agent reading the bootstrap-managed "Speccy conventions" block in
  `AGENTS.md` learns the backlog file exists and what reads and writes it.
</goals>

## Non-goals

<non-goals>
- No `speccy backlog` subcommand, no element grammar, no lint codes, and no
  `speccy verify` participation. The CLI never reads the backlog; a malformed
  or absent file is silent, exactly as `MEMORY.md` is.
- Not a replacement for spec-local deferred surfaces. SPEC `## Non-goals`,
  `<decision status="deferred">`, and REPORT `<coverage result="deferred">`
  stay; `BACKLOG.md` is the repo-wide new-spec register, distinct from them.
  The boundary: a spec-local surface says "not in THIS spec"; the backlog says
  "should become ITS OWN spec."
- No review-phase or vet-phase producer wiring. Those phases are deliberately
  not authorized to append backlog entries.
- No status, priority, or ordering field on entries, and no per-item files or
  `backlog/` subfolder. The backlog is a flat, unordered list in one file.
</non-goals>

## User Stories

<user-stories>
- As a planner framing a new slice, I want a single repo-wide list of
  future-spec candidates to read, so I neither re-derive deferred ideas nor
  lose them across sessions.
- As an implementer shipping a spec, I want the genuinely future-spec-worthy
  deferrals I discovered while building to be captured durably, so they outlive
  the one REPORT.md they were noted in.
- As a maintainer, I want future-spec candidates kept separate from the loop
  memory, so durable repo conventions and one-shot spec candidates do not muddy
  each other's read patterns.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Backlog reference module documents the file header and four-field entry shape

Speccy ships `resources/modules/references/backlog-ledger.md`, the analog of
`memory-ledger.md`, documenting the `.speccy/BACKLOG.md` file header and the
four-field entry shape — Title, What & why, Deferred-because, Provenance — one
line per field, with authoring discipline (terseness, honest provenance)
mirroring the memory ledger. The skill modules that author backlog entries pull
this reference in via `{% include %}`, so the shape reaches their context and
the reference is not orphaned.

<done-when>
- `resources/modules/references/backlog-ledger.md` exists and names exactly the
  four fields Title / What & why / Deferred-because / Provenance plus the file
  header.
- The reference is `{% include %}`d by each skill module that authors backlog
  entries (brainstorm, plan, ship).
- After reejection, the entry-shape content appears in the corresponding
  ejected host skill packs (resource-to-ejected parity holds; no orphaned
  reference).
</done-when>

<behavior>
- Given the resources tree at HEAD, when the backlog reference is read, then it
  documents the four fields and the file header.
- Given a skill module that authors backlog entries, when its `{% include %}`
  directives are resolved, then the backlog reference is among them.
</behavior>

<scenario id="CHK-001">
Given the resources tree and the ejected host packs at HEAD,
when the authoring skill modules are re-ejected and their includes resolved,
then `backlog-ledger.md` is included by at least one authoring skill and its
shape appears in the ejected output — gating orphaning and resource-to-ejected
parity, not mere file existence.
</scenario>

<scenario id="CHK-002">
Given `resources/modules/references/backlog-ledger.md`,
when a reviewer reads it alongside `memory-ledger.md`,
then the four-field shape, file header, and authoring discipline are documented
clearly and parallel the memory ledger's structure. Clarity and parallelism are
a persona-review judgment, not a scriptable assertion.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: `.speccy/BACKLOG.md` is user-owned and outside init's purview

`.speccy/BACKLOG.md` is user-owned and git-tracked. `speccy init`,
`speccy init --force`, and reeject never create, enumerate, or overwrite it; it
is absent until the first entry is written and self-creates with the shipped
header on first append. Its absence is normal and silent — `speccy verify`
emits no diagnostic attributable to a present, absent, or malformed backlog.

<done-when>
- `speccy init` in a fresh repo does not create `.speccy/BACKLOG.md`.
- `speccy init --force` in a repo where `.speccy/BACKLOG.md` already exists
  leaves the file byte-identical.
- `speccy verify` emits no error or warning attributable to the presence,
  absence, or malformed content of `.speccy/BACKLOG.md`.
</done-when>

<behavior>
- Given a fresh repo, when `speccy init` runs, then no `.speccy/BACKLOG.md` is
  created.
- Given a repo with a hand-authored `.speccy/BACKLOG.md`, when
  `speccy init --force` runs, then the file's bytes are unchanged.
</behavior>

<scenario id="CHK-003">
Given a temporary repo containing a sentinel `.speccy/BACKLOG.md` of known
content,
when `speccy init --force` runs,
then the file's sha256 is unchanged and the file is not among the paths the
init reports as ejected.
</scenario>

<scenario id="CHK-004">
Given a fresh temporary repo,
when `speccy init` then `speccy verify` run in sequence,
then no `.speccy/BACKLOG.md` exists afterward and `speccy verify` exits 0 with
no backlog-attributable diagnostic.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Planning skills read the backlog as candidate slices

`/speccy-brainstorm` and `/speccy-plan` read `.speccy/BACKLOG.md`, when present,
as candidate slices while framing new work, so deferred future-spec candidates
resurface at the moment new scope is being chosen. A missing backlog file is
silent and non-fatal.

<done-when>
- The ejected brainstorm and plan skill bodies instruct reading
  `.speccy/BACKLOG.md` as candidate input when framing, and treat its absence
  as normal.
- The instruction reaches the host skill packs via reejection (parity holds).
</done-when>

<behavior>
- Given a repo with a populated backlog, when a planning skill frames new work,
  then existing candidates are surfaced to the user as possible slices.
- Given a repo with no backlog file, when a planning skill runs, then it
  proceeds normally with no error.
</behavior>

<scenario id="CHK-005">
Given the ejected brainstorm and plan skill bodies at HEAD,
when a reviewer reads their framing steps,
then each instructs reading `.speccy/BACKLOG.md` as candidate slices and treats
its absence as silent. Correct placement and wording is a persona-review
judgment.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: Planning skills append a backlog entry on deliberate scope-cut

When `/speccy-brainstorm` or `/speccy-plan` deliberately cuts scope from the
current slice as future-spec-worthy ("not this spec, but its own later"), it
appends a backlog entry in the shipped four-field shape, with provenance naming
the originating spec and phase. A cut that is merely a spec-local Non-goal is
not appended.

<done-when>
- The brainstorm and plan bodies instruct appending a four-field backlog entry
  when scope is deliberately deferred to a future spec, self-creating the file
  with its header if absent.
- The instruction distinguishes a future-spec candidate (append to backlog)
  from a spec-local Non-goal (record in the SPEC's `## Non-goals`).
- A deferred-scope entry recorded this way carries provenance identifying the
  originating spec and phase.
</done-when>

<behavior>
- Given a planning session that cuts a feature as out-of-scope-but-future, when
  the cut is made, then a backlog entry is appended.
- Given a cut that is a spec-local Non-goal only, when the cut is made, then no
  backlog entry is appended.
</behavior>

<scenario id="CHK-006">
Given the ejected planning skill bodies at HEAD,
when a reviewer reads the scope-cut handling,
then it appends a four-field entry for future-spec candidates and routes
spec-local Non-goals to the SPEC's `## Non-goals` instead. The future-vs-local
distinction is a persona-review judgment, not a scriptable assertion.
</scenario>
</requirement>

<requirement id="REQ-005">
### REQ-005: Ship mirrors only the judgment-gated future-spec subset

`/speccy-ship` reviews each entry in its REPORT "Deferred / known limitations"
section and mirrors into `.speccy/BACKLOG.md` only the subset that warrants its
own future SPEC, asking per item "its own future SPEC, or just a limitation of
this one?". Bug-level caveats and small follow-ups stay in REPORT.md and are not
mirrored, so the backlog stays a high-signal new-spec register.

<done-when>
- The ship body instructs a per-item "own future spec vs. local limitation"
  judgment over the REPORT deferred section, mirroring only the former into the
  backlog in the four-field shape with ship-phase provenance.
- Items judged local limitations remain only in REPORT.md and are not appended
  to the backlog.
</done-when>

<behavior>
- Given a REPORT with a future-spec-worthy deferral, when ship runs, then that
  item is appended to the backlog.
- Given a REPORT whose deferrals are all local limitations, when ship runs,
  then the backlog is unchanged.
</behavior>

<scenario id="CHK-007">
Given the ejected ship skill body at HEAD,
when a reviewer reads the deferred-section handling,
then it gates mirroring on the per-item "own future spec?" judgment and excludes
local limitations. The gating judgment is a persona-review judgment, not a
scriptable assertion.
</scenario>
</requirement>

<requirement id="REQ-006">
### REQ-006: Promotion strikes the entry by deletion

When a backlog item is promoted into a new SPEC during planning, the planner
strikes it from `.speccy/BACKLOG.md` by deleting the entry. No struck-through or
"promoted to" residue is retained — git history and the new SPEC's own
provenance are the trail — so the backlog stays a live list of current
candidates.

<done-when>
- The plan body instructs deleting a promoted backlog entry once the
  corresponding SPEC.md is drafted.
- No tombstone or struck-through residue for a promoted item is retained in the
  backlog.
</done-when>

<behavior>
- Given a backlog item chosen as the basis for a new SPEC, when the SPEC is
  drafted, then the item's entry is removed from the backlog.
- Given the backlog after a promotion, when it is read, then it contains no
  struck-through or "promoted to" residue for the removed item.
</behavior>

<scenario id="CHK-008">
Given the ejected plan skill body at HEAD,
when a reviewer reads the promotion handling,
then it instructs a silent delete rather than a tombstone and relies on git and
the new SPEC's provenance for traceability. Adequacy of the instruction is a
persona-review judgment.
</scenario>
</requirement>

<requirement id="REQ-007">
### REQ-007: Bootstrap names the backlog in the conventions block

`/speccy-bootstrap` names `.speccy/BACKLOG.md` in the always-upserted
"## Speccy conventions" block it writes into `AGENTS.md`, in one terse line
orienting agents: future-spec candidates live there; planning reads it,
plan/ship append. The artifact is thereby discoverable to an agent not
currently running a planning or ship skill.

<done-when>
- The bootstrap skill's conventions-block source names the `.speccy/BACKLOG.md`
  path and its one-line read/append role.
- The line reaches the conventions block written into `AGENTS.md` on bootstrap
  (parity holds through reejection).
</done-when>

<behavior>
- Given a repo where bootstrap (re)writes the conventions block, when the block
  is produced, then it contains a line naming `.speccy/BACKLOG.md` and its
  read/append roles.
</behavior>

<scenario id="CHK-009">
Given the ejected bootstrap conventions-block source at HEAD,
when it is scanned,
then it references the `.speccy/BACKLOG.md` path and a one-line role for it. The
path reference is structural (a path the skill operates on); the adequacy of the
wording is a persona-review judgment.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
#### DEC-001: Convention-only — the CLI never touches the backlog

**Context:** A CLI-managed backlog (`speccy backlog add/list`, an element
grammar, lint codes) was considered, as was a thinner read-only
`speccy backlog list --json` index.

**Decision:** Keep the backlog a user-owned convention file, exactly like
`MEMORY.md`. The Rust CLI never reads, parses, lints, or verify-gates it.

**Alternatives:** A CLI-managed structured artifact — rejected: it grows an
enforcement and grammar surface over freeform intent, against the stay-small
and feedback-not-enforcement principles. A read-only list command — rejected:
it still requires the CLI to parse the file, which is the camel's nose for the
full grammar.

**Consequences:** Zero Rust logic for the backlog. Discoverability rests on
skill wiring plus the bootstrap conventions line (REQ-007). There is no machine
query surface, which is acceptable at this scale.
</decision>

<decision id="DEC-002">
#### DEC-002: Producers are plan/brainstorm, ship, and manual only

**Context:** Four phases could plausibly append backlog entries —
plan/brainstorm, ship, review, and vet.

**Decision:** Wire only the deliberate, rate-limited, once-per-spec moments:
plan/brainstorm (planned deferral) and ship (discovered deferral). Manual edits
are always available. Review and vet are not wired.

**Alternatives:** Wiring review — rejected: it fires per task and per round,
and "out of scope, defer" is common reviewer chatter, so it would turn the
backlog into a dumping ground. Wiring vet — rejected: vet findings resolve to
fix-the-code or amend-the-spec, and a genuinely future-spec-worthy finding
surfaces in vet's verdict and is captured at the adjacent ship step, so wiring
vet would double-capture.

**Consequences:** Per-spec backlog growth stays low and deliberate. An idea
emerging mid-build is captured at ship or by hand, not at review time. The
per-spec add rate becomes a focus signal in its own right.
</decision>

<decision id="DEC-003">
#### DEC-003: Promotion strikes the entry by deletion, not a tombstone

**Context:** A promoted item could be deleted outright or struck through with
the promoted SPEC id retained for traceability.

**Decision:** Delete the entry on promotion.

**Alternatives:** A struck-through tombstone carrying the new SPEC id —
rejected: it duplicates traceability that git history and the new SPEC's own
provenance already provide, and accumulates dead noise in what should be a live
working list.

**Consequences:** The backlog reads as current candidates only; the promotion
trail lives in git and the promoted SPEC.
</decision>

<decision id="DEC-004">
#### DEC-004: A separate file, not a section of MEMORY.md

**Context:** Backlog entries could live in their own file or as a tagged
section inside `MEMORY.md`.

**Decision:** A separate `.speccy/BACKLOG.md`, sibling to `MEMORY.md`.

**Alternatives:** A tagged section inside `MEMORY.md` — rejected: it conflates
two lifecycles — durable repo conventions that are never retired versus one-shot
candidates retired on promotion — and muddies both ledgers' read patterns.

**Consequences:** Two sibling ledgers with distinct purposes and read cadences;
agents and the reference docs treat them independently.
</decision>

## Assumptions

<assumptions>
- `.speccy/BACKLOG.md` mirrors `MEMORY.md`'s lifecycle exactly: absent until
  first write, never touched by `speccy init --force` or reeject.
- Entries are a flat list in one file — no per-item files and no `backlog/`
  subfolder — matching `.speccy/MEMORY.md`, not the index-plus-per-file shape of
  external agent memory systems.
- Entries carry no status, priority, or ordering field; they are unordered
  candidates, and prioritization is the planner's job at read time.
- Manual/ad-hoc add needs no tooling — an agent or human edits the file
  directly, the same as `MEMORY.md`; no skill owns the manual path.
</assumptions>

## Open questions

None — the brainstorm resolved framing questions a through e before this SPEC
was drafted.

## Notes

A spec that spawns many backlog items is a signal it was not atomic enough; the
per-spec add rate is itself feedback, on-brand for Speccy's "make drift loud"
stance. This belongs as guidance in the backlog reference and the producing
skill prose, not as an enforced threshold.

Deriving a cross-spec deferred index from existing surfaces (SPEC `## Non-goals`
plus REPORT `<coverage result="deferred">`) was considered and rejected: those
surfaces are spec-local and not new-spec candidates, and deriving an index from
them would require the CLI to parse them — collapsing into the rejected
CLI-managed framing of DEC-001.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-13 | Kevin Xiao | Initial SPEC: convention-only `.speccy/BACKLOG.md` future-spec register — reference module + four-field entry shape (REQ-001), init-immune user-owned lifecycle (REQ-002), planning reads (REQ-003) and appends on scope-cut (REQ-004), ship mirrors the judgment-gated subset (REQ-005), promotion strikes the entry (REQ-006), and bootstrap names it in the conventions block (REQ-007). |
</changelog>
