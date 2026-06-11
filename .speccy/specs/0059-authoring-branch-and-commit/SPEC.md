---
id: SPEC-0059
slug: authoring-branch-and-commit
title: Branch-guard and consolidated git commits for the authoring skills (plan / decompose / amend)
status: implemented
created: 2026-06-11
supersedes: []
---

# SPEC-0059: Branch-guard and consolidated git commits for the authoring skills (plan / decompose / amend)

## Summary

The three SPEC-authoring skills — `speccy-plan`, `speccy-decompose`, and
`speccy-amend` — leave the working tree in an inconsistent git state.
Today `speccy-decompose` is the only one that commits: its step-4
bootstrap commit lands `SPEC.md` + `TASKS.md` as one
`[SPEC-NNNN]: create spec and decompose tasks` commit, but it commits
onto **whatever branch HEAD is** — including the default branch.
`speccy-plan` writes `SPEC.md` and never commits; `speccy-amend` edits
`SPEC.md`, reconciles `TASKS.md`, and re-locks the hash, but leaves all
of that uncommitted. The `spec-NNNN-slug` feature branch is created by
hand before any of this runs.

This SPEC closes both gaps with a single shape. (1) A **branch-guard
prelude**: before any authoring-skill commit, ensure HEAD is on a
feature branch, creating and switching to `spec-NNNN-slug` only when
HEAD is the default branch or a detached HEAD. (2) A **per-skill
commit** at each skill's natural success point, so the authored
artifacts always land committed on a feature branch rather than dirtying
the default branch. The combined `create spec and decompose tasks`
bootstrap commit splits into a `plan` commit (SPEC.md) and a `decompose`
commit (TASKS.md), per the decision that each authoring skill commits
its own delta.

All git mutation stays in the skill layer — the Rust CLI continues to
never invoke git (`docs/ARCHITECTURE.md`: "The binary never invokes
`git add`, `git commit`, `git restore`, `git clean`, or `git stash`").
This is therefore a change to the skill-pack prose under
`resources/modules/` plus a `just reeject`, not a CLI change.

The commit prose is already duplicated: the
`resources/modules/skills/partials/review-fanout.md` atomic-commit-on-
review-pass section and the `resources/modules/phases/speccy-decompose.md`
bootstrap commit are two hand-rolled copies of the same
"stage → skip-if-nothing → commit with title/body/`Co-Authored-By`"
shape. Rather than add a third copy, this SPEC extracts one shared
commit recipe that all callsites include, paying down existing
duplication per the project's deduplication rule. The branch-guard is a
separate prelude included only by the three authoring skills; the
work/review commit deliberately stays unguarded and continues to land on
whatever HEAD is.

## Goals

<goals>
- Running `speccy-plan` / `speccy-decompose` / `speccy-amend` while HEAD
  is on the default branch first creates and switches to a
  `spec-NNNN-slug` feature branch, so no authored artifact is committed
  to the default branch.
- `speccy-plan` commits `SPEC.md` alone; `speccy-decompose` commits
  `TASKS.md` alone; `speccy-amend` commits its reconcile delta — each
  with its own `[SPEC-NNNN]:`-prefixed title.
- One shared reference module carries the commit recipe, included by the
  authoring skills and by the existing work/review and decompose
  callsites, with no verbatim copy of the recipe left in any individual
  file.
- The existing work/review-pass commit behaviour is observably
  unchanged after the refactor (same grepped title format, same `git add
  -A` staging, same trailer, same single-parent commit).
</goals>

## Non-goals

<non-goals>
- No CLI change. The Rust binary still never runs git; no `speccy
  branch` verb, no `speccy commit` verb. All branch/commit behaviour
  lives in skill-pack prose.
- No new mode toggle, config file, or policy knob. Branch creation is
  unconditional under its trigger; it is not gated behind a flag.
- No change to `speccy-work` or `speccy-ship` commit behaviour beyond
  the behaviour-preserving extraction of the shared recipe. They are not
  branch-guarded by this SPEC.
- No interactive confirmation prompt before branch creation. The branch
  is created automatically and the action is surfaced, not gated.
- No stashing or refusal on a dirty working tree. Unrelated uncommitted
  changes are left untouched and carried onto the new branch by git.
</non-goals>

## User Stories

<user-stories>
- As a solo developer starting a new spec from the default branch, I
  want the authoring skills to put me on a `spec-NNNN-slug` branch and
  commit the artifacts there automatically, so I stop hand-creating the
  branch and hand-committing the scaffolding.
- As an agent driving `speccy-amend` mid-loop, I want the amendment
  (SPEC edit, reconciled tasks, journal blockers) committed in one step
  so the next `speccy-work` clean-tree gate is not tripped by an
  uncommitted amendment.
- As a maintainer of the skill pack, I want the commit recipe defined
  once and included everywhere, so a future change to commit shape does
  not require editing three divergent copies.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Branch-guard prelude ensures a feature branch before committing

Before its commit step, each authoring skill (`speccy-plan`,
`speccy-decompose`, `speccy-amend`) runs a branch-guard prelude that
guarantees HEAD is on a feature branch. When HEAD is the default branch
or a detached HEAD, the prelude creates and switches to a branch named
`spec-NNNN-slug`, derived as the literal `spec-` prefix followed by the
spec directory basename (`NNNN-slug`); mission-foldered specs
(`[focus]/NNNN-slug`) get the same flat `spec-NNNN-slug` name with the
focus segment dropped. When HEAD is already on any other branch, the
prelude reuses it and creates nothing. Branch creation is automatic with
no confirmation prompt, and emits a single one-line notice naming the
created branch.

<done-when>
- On the default branch, invoking an authoring skill results in HEAD on
  a new `spec-NNNN-slug` branch before the commit step runs.
- On a detached HEAD, the same `spec-NNNN-slug` branch is created and
  switched to.
- On a non-default branch (e.g. an existing `spec-NNNN-slug` or an
  unrelated feature branch), no new branch is created and HEAD is
  unchanged.
- For a spec at `.speccy/specs/[focus]/0059-authoring-branch-and-commit/`,
  the derived branch is `spec-0059-authoring-branch-and-commit` (no
  focus segment).
- A one-line notice naming the created branch is emitted exactly when a
  branch is created, and not when an existing branch is reused.
</done-when>

<behavior>
- Given HEAD is on the default branch, when the branch-guard prelude
  runs, then it creates `spec-NNNN-slug`, switches to it, and emits the
  creation notice.
- Given HEAD is on an unrelated feature branch, when the prelude runs,
  then it makes no branch and emits no notice.
- Given a mission-foldered spec dir, when the branch name is derived,
  then the focus segment is excluded and the name is flat
  `spec-NNNN-slug`.
</behavior>

<scenario id="CHK-001">
Given the rendered skill pack at HEAD after this SPEC lands,
when the `speccy-plan`, `speccy-decompose`, and `speccy-amend` skill/phase
sources are inspected,
then each includes the shared branch-guard prelude ahead of its commit
step, and the prelude text derives the branch name as `spec-` + spec
dir basename and creates the branch only on the default-or-detached
condition.
</scenario>

<scenario id="CHK-002">
Given the branch-guard prelude text,
when its branch-creation condition is read,
then it creates `spec-NNNN-slug` when HEAD is the default branch or
detached, reuses the current branch otherwise, and the creation notice
is emitted only on the create path.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Default-branch detection chain

The branch-guard prelude identifies the repository's default branch via
an ordered chain: the remote symbolic ref `origin/HEAD` when a remote
exists; otherwise `git config init.defaultBranch`; otherwise, when
neither resolves, HEAD is treated as the default branch when its name is
`main` or `master`.

<done-when>
- With a remote whose `origin/HEAD` points at `origin/main`, the default
  branch is detected as `main`.
- With no remote but `init.defaultBranch` set to `trunk`, the default
  branch is detected as `trunk`.
- With no remote and `init.defaultBranch` unset, HEAD on `main` or
  `master` is treated as the default branch; HEAD on any other name is
  treated as a feature branch.
</done-when>

<behavior>
- Given a remote is present, when default-branch detection runs, then it
  reads `origin/HEAD` and uses that branch name.
- Given no remote and `init.defaultBranch` is set, when detection runs,
  then it uses the configured value.
- Given no remote and no `init.defaultBranch`, when detection runs, then
  it falls back to matching HEAD against the set `{main, master}`.
</behavior>

<scenario id="CHK-003">
Given the branch-guard prelude text at HEAD after this SPEC lands,
when its default-branch detection steps are read,
then they specify the three-tier chain in order — `origin/HEAD`, then
`git config init.defaultBranch`, then a `{main, master}` name match —
with each tier used only when the prior one does not resolve.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `speccy-plan` commits SPEC.md after a successful write

After `speccy-plan` finishes writing `SPEC.md` (post self-review),
it commits `SPEC.md` alone with the title `[SPEC-NNNN]: create spec`,
using the shared commit recipe with narrow staging of the SPEC.md path.

<done-when>
- After `speccy-plan` completes on a fresh spec, a commit titled
  `[SPEC-NNNN]: create spec` exists whose changed paths are limited to
  the spec's `SPEC.md`.
- The commit carries the `Co-Authored-By` trailer per the shared
  recipe's identity-sourcing rule.
- Re-running `speccy-plan` against an unchanged `SPEC.md` produces no
  new commit (idempotent).
</done-when>

<behavior>
- Given `SPEC.md` was just written, when `speccy-plan` reaches its
  commit step, then it commits only `SPEC.md` with the `create spec`
  title.
- Given `SPEC.md` is unchanged from HEAD, when the commit step runs,
  then nothing is staged and no commit is made.
</behavior>

<scenario id="CHK-004">
Given the `speccy-plan` skill source at HEAD after this SPEC lands,
when its commit step is inspected,
then it invokes the shared commit recipe with the title
`[SPEC-NNNN]: create spec` and a staging set limited to the spec's
`SPEC.md`, and it runs after the self-review pass.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: `speccy-decompose` commits TASKS.md alone

`speccy-decompose`'s commit step commits `TASKS.md` alone with the title
`[SPEC-NNNN]: decompose tasks`, running after `speccy lock`. This
replaces the prior combined `[SPEC-NNNN]: create spec and decompose
tasks` bootstrap commit; `SPEC.md` is committed by `speccy-plan`
(REQ-003), not here.

<done-when>
- After `speccy-decompose` completes, a commit titled
  `[SPEC-NNNN]: decompose tasks` exists whose changed paths are limited
  to the spec's `TASKS.md`.
- The prior combined `create spec and decompose tasks` commit message no
  longer appears in the `speccy-decompose` source.
- Re-running `speccy-decompose` against an unchanged `TASKS.md` produces
  no new commit (idempotent).
</done-when>

<behavior>
- Given `TASKS.md` was written and `speccy lock` has run, when the
  commit step runs, then it commits only `TASKS.md` with the
  `decompose tasks` title.
- Given `TASKS.md` is unchanged from HEAD, when the commit step runs,
  then nothing is staged and no commit is made.
</behavior>

<scenario id="CHK-005">
Given the `speccy-decompose` phase source at HEAD after this SPEC lands,
when its commit step is inspected,
then it invokes the shared commit recipe with the title
`[SPEC-NNNN]: decompose tasks` and a staging set limited to the spec's
`TASKS.md`, runs after `speccy lock`, and the string
`create spec and decompose tasks` is absent from the source.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: `speccy-amend` commits its reconcile delta

After the amendment is reconciled and `speccy status` reports no
`TSK-003` mismatch, `speccy-amend` commits its delta — `SPEC.md`, the
reconciled `TASKS.md` when one exists, and any journal blocker files it
appended this run — with the title `[SPEC-NNNN]: amend — <why>`, where
`<why>` is a title-length phrase derived from the newest `## Changelog`
row added during this amend. When the spec has no `TASKS.md` yet, the
commit contains `SPEC.md` (and any journal files) without failing on the
absent tasks file.

<done-when>
- After an amend on a spec with a `TASKS.md`, a commit titled
  `[SPEC-NNNN]: amend — <why>` exists whose changed paths include
  `SPEC.md`, the reconciled `TASKS.md`, and any journal blocker files
  appended this run.
- After an amend on a spec with no `TASKS.md`, the same-shaped commit
  exists containing `SPEC.md` (plus any journal files) with no error
  from the missing `TASKS.md`.
- The `<why>` phrase in the commit title is sourced from the newest
  `## Changelog` row added this run, not separately prompted.
</done-when>

<behavior>
- Given an amend reconciled a spec that has a `TASKS.md`, when the
  commit step runs, then it stages SPEC.md, the reconciled TASKS.md, and
  appended journal blockers, and commits them under the `amend — <why>`
  title.
- Given an amend on a spec with no TASKS.md, when the commit step runs,
  then it commits SPEC.md (and any journal files) and does not fail on
  the absent TASKS.md.
- Given the amend appended a Changelog row, when the commit title is
  built, then `<why>` is derived from that newest row.
</behavior>

<scenario id="CHK-006">
Given the `speccy-amend` skill source at HEAD after this SPEC lands,
when its commit step is inspected,
then it invokes the shared commit recipe with the title
`[SPEC-NNNN]: amend — <why>`, a staging set covering SPEC.md plus
TASKS.md-when-present plus appended journal files, runs after the
`TSK-003`-clear check, and sources `<why>` from the newest `## Changelog`
row.
</scenario>

<scenario id="CHK-007">
Given a spec with no `TASKS.md`,
when the amend commit step's staging set is read,
then it tolerates the absent `TASKS.md` (commits SPEC.md and any journal
files) rather than requiring the tasks file to exist.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: One shared commit-recipe reference module

A single reference module under `resources/modules/references/` carries
the commit recipe: stage the configured path set, run `git diff --cached
--quiet`, skip the commit silently when nothing is staged, otherwise
commit with the supplied title, body, and `Co-Authored-By` identity
trailer. The staging breadth (`git add -A` versus a narrow path list) is
the only behaviour-varying parameter besides the title and body; the
idempotency check is the unified stage-then-`git diff --cached --quiet`
form for every caller.

<done-when>
- The reference module exists and states the stage →
  `git diff --cached --quiet` skip-if-empty → commit recipe once.
- The recipe is parameterised by staging set and message; it reuses the
  existing `identity-sourcing` trailer rule rather than restating it.
- The recipe's idempotency check is the single stage-then-`git diff
  --cached --quiet` form (not a per-caller `git status --porcelain`
  variant).
</done-when>

<behavior>
- Given the shared module, when its recipe is read, then staging breadth
  is the sole behaviour-varying parameter besides title/body and the
  trailer is delegated to the identity-sourcing rule.
- Given any caller's staging mode, when the recipe runs, then the
  skip-if-nothing decision is made by `git diff --cached --quiet` after
  staging.
</behavior>

<scenario id="CHK-008">
Given the rendered skill pack at HEAD after this SPEC lands,
when the references directory is inspected,
then exactly one shared module defines the stage →
`git diff --cached --quiet` → commit recipe, parameterised by staging
set and message, and delegating the trailer to the identity-sourcing
rule.
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: Existing commit callsites refactor onto the shared recipe, behaviour-preserving

The two existing hand-rolled commit copies — the atomic-commit-on-
review-pass section in
`resources/modules/skills/partials/review-fanout.md` and the bootstrap
commit in `resources/modules/phases/speccy-decompose.md` — are rewritten
to include the shared recipe (REQ-006) rather than restating it. The
refactor is behaviour-preserving for the work/review commit: its title
stays `[SPEC-NNNN/T-NNN]: <task title>` (the prefix the CLI consistency
check greps), its staging stays `git add -A`, its trailer is unchanged,
and it remains a single-parent commit. The unified stage-then-`git diff
--cached --quiet` check yields the same commit/skip outcome the prior
`git status --porcelain` pre-check produced for the `git add -A`
callsite.

<done-when>
- No file under `resources/modules/` contains a verbatim second copy of
  the stage/skip/commit recipe; `review-fanout.md` and
  `speccy-decompose.md` pull it via `{% include %}`.
- The work/review-pass commit, after refactor, still emits the
  `[SPEC-NNNN/T-NNN]: <task title>` title, stages via `git add -A`, and
  produces a single-parent commit.
- For a clean working tree the refactored review-pass commit step makes
  no commit (same outcome as the prior `git status --porcelain`
  pre-check).
</done-when>

<behavior>
- Given the refactored `review-fanout.md`, when its commit step is read,
  then it includes the shared recipe with `-A` staging and the
  `[SPEC-NNNN/T-NNN]:` title format unchanged.
- Given a clean working tree at the review-pass commit step, when the
  unified idempotency check runs, then it skips the commit, matching the
  prior behaviour.
</behavior>

<scenario id="CHK-009">
Given the rendered skill pack at HEAD after this SPEC lands,
when `review-fanout.md` and `speccy-decompose.md` are inspected,
then both include the shared commit recipe via `{% include %}` and
neither restates the `git diff --cached --quiet` commit recipe inline.
</scenario>

<scenario id="CHK-010">
Given the refactored review-pass commit step,
when its rendered text is inspected,
then the commit title format `[SPEC-NNNN/T-NNN]: <task title>`, the
`git add -A` staging, and the single-parent commit shape are all
retained verbatim.
</scenario>

</requirement>

<requirement id="REQ-008">
### REQ-008: Branch-guard is scoped to the authoring skills only

The branch-guard prelude (REQ-001) lives in its own shared module that
is `{% include %}`'d only by the `speccy-plan`, `speccy-decompose`, and
`speccy-amend` sources. It is not included by `speccy-work` or
`speccy-ship`, and the work/review-pass commit remains unguarded —
landing on whatever HEAD is, per its existing "the skill body does not
check the current git branch" contract.

<done-when>
- The branch-guard module is included by the plan, decompose, and amend
  sources.
- The branch-guard module is not included by the `speccy-work` or
  `speccy-ship` sources.
- `review-fanout.md`'s "commits land on whatever HEAD is" /
  "does not check the current git branch" statement is retained.
</done-when>

<behavior>
- Given the skill-pack sources, when include directives are enumerated,
  then the branch-guard module appears under plan/decompose/amend and is
  absent from work/ship.
- Given the review-pass commit section, when read after the refactor,
  then it still declares that commits land on whatever HEAD is.
</behavior>

<scenario id="CHK-011">
Given the rendered skill pack at HEAD after this SPEC lands,
when include directives are enumerated across the skill/phase sources,
then the branch-guard module is included by exactly the plan, decompose,
and amend sources and by no others, and the review-pass commit retains
its unguarded "lands on whatever HEAD is" statement.
</scenario>

</requirement>

<requirement id="REQ-009">
### REQ-009: Authoring commits use narrow staging with no clean-tree gate

The three authoring-skill commits stage only their own spec artifacts
(narrow `git add <paths>`), so unrelated dirty paths in the working tree
are neither staged nor committed. No clean-tree gate guards the
authoring skills: they proceed on a dirty tree, and `git switch -c`
carries any unrelated uncommitted changes onto the newly created branch.

<done-when>
- Each authoring commit's staging is a narrow path list, never `git add
  -A` / `git add .`.
- With unrelated dirty paths present, an authoring commit contains only
  the spec artifacts and the unrelated paths remain modified in the
  working tree.
- The authoring skills contain no clean-tree refusal gate before their
  branch/commit steps.
</done-when>

<behavior>
- Given unrelated modified files plus a changed SPEC.md, when the plan
  commit runs, then only SPEC.md is committed and the unrelated files
  stay dirty.
- Given a dirty tree on the default branch, when the branch-guard
  creates `spec-NNNN-slug`, then the unrelated changes are carried onto
  the new branch (not stashed, not blocking).
</behavior>

<scenario id="CHK-012">
Given the three authoring-skill sources at HEAD after this SPEC lands,
when their commit and branch-guard steps are inspected,
then each stages a narrow spec-artifact path list (no `git add -A` /
`git add .`) and none contains a clean-tree refusal gate.
</scenario>

</requirement>

<requirement id="REQ-010">
### REQ-010: Non-git projects degrade gracefully

In a project with no git repository, the three authoring skills still
write their files (`SPEC.md` / `TASKS.md`) and skip the branch-guard and
commit steps without erroring, preserving Speccy's "works identically in
any project state" property.

<done-when>
- In a non-git directory, `speccy-plan` writes `SPEC.md` and completes
  without a git error.
- The branch-guard and commit steps are skipped (not attempted) when no
  git repository is present.
</done-when>

<behavior>
- Given a directory that is not a git repository, when an authoring
  skill runs, then it writes its artifact and skips branch/commit
  without surfacing a git failure.
</behavior>

<scenario id="CHK-013">
Given the branch-guard and shared commit-recipe modules at HEAD after
this SPEC lands,
when their preconditions are read,
then both specify a no-git-repository short-circuit that skips the
branch/commit steps without erroring.
</scenario>

</requirement>

## Assumptions

<assumptions>
- The spec directory basename is exactly `NNNN-slug`, so the branch name
  derives mechanically as `spec-` + basename; mission-foldered specs
  (`[focus]/NNNN-slug`) get a flat `spec-NNNN-slug` branch because SPEC
  IDs are workspace-unique and the focus segment adds no disambiguation.
- The new-spec path producing two commits (`plan`, then `decompose`)
  instead of today's single combined bootstrap commit is acceptable
  history shape.
- `git switch -c` carrying unrelated uncommitted changes onto the new
  branch (no stash, no block) is the desired behaviour.
- The existing `resources/modules/references/identity-sourcing.md`
  trailer rule is reused by the shared recipe for every callsite rather
  than restated.
- In the amend-then-re-decompose flow, committing the reconciled
  `TASKS.md` that a follow-on `speccy-decompose` then regenerates and
  re-commits is acceptable (the superseded tasks state in history is
  tolerated).
</assumptions>

## Decisions

<decision id="DEC-001">
All git mutation stays in the skill layer. A `speccy branch` (or
`speccy commit`) CLI verb is rejected: it would violate the standing
architecture invariant that the binary never invokes git
(`docs/ARCHITECTURE.md`, "CLI stays read-only"). Branch and commit
behaviour is expressed as skill-pack prose only.
</decision>

<decision id="DEC-002">
The commit logic is consolidated onto one shared recipe module rather
than adding fresh per-skill prose. `review-fanout.md` and
`speccy-decompose.md` already carry two divergent hand-rolled copies of
the same commit shape; a third copy would compound the drift the
project's deduplication rule exists to prevent.
</decision>

<decision id="DEC-003">
The branch-guard lives at the commit point inside the three authoring
skills, not in `speccy-orchestrate`. The authoring skills run standalone
(not only under the orchestrator), so the guard must sit where the
commit happens; and the orchestrator starts after `speccy-decompose`,
too late to protect the plan and decompose commits.
</decision>

<decision id="DEC-004">
The idempotency check is unified to "stage, then `git diff --cached
--quiet`, skip if nothing staged." This single form subsumes both
existing checks — `review-fanout.md`'s `git status --porcelain`
pre-stage check and `speccy-decompose.md`'s `git diff --cached --quiet`
post-stage check — with identical commit/skip outcomes. For the `git add
-A` review callsite the only difference is one harmless no-op `git add
-A` on an already-clean tree, which leaves the observable result
unchanged.
</decision>

<decision id="DEC-005">
Each authoring skill commits its own delta (plan → SPEC.md; decompose →
TASKS.md; amend → its reconcile delta), splitting today's combined
`create spec and decompose tasks` bootstrap commit into two commits on
the new-spec path. Separate commits keep each skill's output traceable
and let `speccy-plan` run-then-stop leave `SPEC.md` already committed.
</decision>

## Notes

Rejected framings carried from brainstorm, recorded for provenance:

- **`speccy branch` CLI verb** — rejected per DEC-001 (CLI never runs
  git).
- **Branch-guard owned by `speccy-orchestrate`** — rejected per DEC-003
  (authoring skills run standalone; orchestrator starts too late to
  protect the plan/decompose commits).
- **Pure-add (write fresh branch/commit prose, leave the existing
  copies untouched)** — rejected per DEC-002 (would leave a third
  hand-rolled commit copy, against the deduplication rule).

## Open Questions

None — the five questions raised during brainstorm were resolved and
folded into the Requirements, Assumptions, and Decisions above
(mission-foldered branch name → flat; guard in work/ship → no;
amend `<why>` source → newest Changelog row; default-branch terminal
fallback → `{main, master}` name set; idempotency check → unified
stage-then-`git diff --cached --quiet`).

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-11 | claude-opus-4-8[1m] | Initial SPEC: branch-guard prelude + per-skill commits for plan/decompose/amend, consolidated onto one shared commit recipe. |
</changelog>
