---
id: SPEC-0046
slug: rename-speccy-tasks-to-decompose
title: Rename the `/speccy-tasks` skill to `/speccy-decompose`
status: in-progress
created: 2026-05-25
supersedes: []
---

# SPEC-0046: Rename the `/speccy-tasks` skill to `/speccy-decompose`

## Summary

The shipped phase-2 skill is invoked as `/speccy-tasks` but its
purpose is to decompose a `SPEC.md` into `TASKS.md`. The verb
"tasks" reads as a noun and follows the pattern of list/query
verbs in adjacent CLIs (`gh issue list`, `git stash list`), which
trains users to expect the skill enumerates an existing task list
rather than producing one. The actual sibling verbs in the speccy
skill pack — `plan`, `brainstorm`, `work`, `review`, `ship`, `vet`,
`amend`, `orchestrate` — are all action verbs. Renaming to
`/speccy-decompose` aligns the name with the action and removes the
naming foot-gun.

The Rust CLI is unaffected: no `speccy tasks` subcommand exists
(it was removed in SPEC-0033), so this rename is purely a
skill-pack and documentation change. Skill names are referenced
by humans and harnesses, not persisted in `SPEC.md` / `TASKS.md`
artifacts, so the rename is forward-only — already-completed
specs do not need rewriting.

## Goals

<goals>
- Every shipped skill file, agent definition, and resource template
  under the names `speccy-tasks` (filenames, frontmatter `name:`
  slugs, file-internal references) is renamed to `speccy-decompose`,
  including the four installed pack locations
  (`.claude/skills/`, `.claude/agents/`, `.agents/skills/`,
  `.codex/agents/`) and their `resources/` template counterparts.
- Every cross-skill reference to `/speccy-tasks` or `speccy-tasks`
  in shipped skill bodies (`speccy-plan`, `speccy-brainstorm`,
  `speccy-orchestrate`, `speccy-work`, and any others discovered
  during implementation) is updated to point at the new name.
- `docs/ARCHITECTURE.md` and `README.md` use `speccy-decompose` for
  every reference to the phase-2 skill, including diagrams,
  invocation examples, and the pinned-phase-worker enumeration.
- Init integration tests (`init.rs`, `init_phase_agents.rs`,
  `pin_shape.rs`) assert against the new file paths and skill
  names; after the rename, `cargo test --workspace` passes.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo +nightly fmt --all --check`,
  and `cargo deny check` all pass on the rename commit.
</goals>

## Non-goals

<non-goals>
- No behavior change. The renamed skill body is byte-identical to
  the old one apart from the name slug and any self-references.
- No CLI binary changes. `speccy tasks` is not a real subcommand
  and no CLI surface is touched.
- No backwards-compatibility shim. The old skill name is removed,
  not aliased; existing harnesses that hard-coded `speccy-tasks`
  must update to the new name on their next pull. This matches
  the project's standing rule against backwards-compatibility
  shims for skill names.
- No rename of any other skill (`speccy-work`, `speccy-review`,
  etc.). The borderline case for `speccy-work → speccy-implement`
  is deferred; this SPEC is scoped to `tasks → decompose` only.
- No edits to existing archived spec content under
  `.speccy/archive/`. Archived specs may reference the old name in
  historical context and are not rewritten.
</non-goals>

## User Stories

<user-stories>
- As a developer first opening the speccy skill list, I want the
  phase-2 skill name to describe its action (decompose) so that I
  can tell from the name alone that it produces a task list rather
  than listing tasks.
- As a harness author chaining the loop, I want skill names that
  use parallel action verbs (plan / decompose / work / review /
  ship) so I can scan the pipeline without re-reading skill
  descriptions.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Rename installed skill files and frontmatter slugs

The four installed skill artifacts that today live at
`.claude/skills/speccy-tasks/SKILL.md`,
`.claude/agents/speccy-tasks.md`,
`.agents/skills/speccy-tasks/SKILL.md`, and
`.codex/agents/speccy-tasks.toml` are renamed to use
`speccy-decompose` in both the directory/file path and any
internal `name:` / `slug:` frontmatter field. Companion
`references/` content under the renamed skill directories moves
with the parent directory.

<done-when>
- After the rename, `find . -path ./target -prune -o -name '*speccy-tasks*' -print` (or its PowerShell equivalent) returns no matches outside `.speccy/archive/`.
- The four new paths exist:
  `.claude/skills/speccy-decompose/SKILL.md`,
  `.claude/agents/speccy-decompose.md`,
  `.agents/skills/speccy-decompose/SKILL.md`,
  `.codex/agents/speccy-decompose.toml`.
- Each renamed file's frontmatter `name:` (or TOML `name = `) field
  reads `speccy-decompose`, not `speccy-tasks`.
- Files moved via `git mv` so history is preserved on the rename;
  `git log --follow` on a new path resolves to commits on the old
  path.
</done-when>

<behavior>
- Given a fresh clone at HEAD after this SPEC lands, when a developer
  lists skills via the host harness, then the phase-2 skill appears
  as `speccy-decompose` and `speccy-tasks` does not appear.
- Given the renamed skill files, when their frontmatter is parsed,
  then the `name:` value matches the parent directory or file basename
  (whichever the host pack convention expects).
</behavior>

<scenario id="CHK-001">
Given the working tree at HEAD after this SPEC lands,
when a grep for `speccy-tasks` runs over the tree excluding
`.speccy/archive/` and `target/`,
then no matches are returned.
</scenario>

<scenario id="CHK-002">
Given the same tree,
when `.claude/skills/speccy-decompose/SKILL.md`,
`.claude/agents/speccy-decompose.md`,
`.agents/skills/speccy-decompose/SKILL.md`, and
`.codex/agents/speccy-decompose.toml` are read,
then each exists and its `name:` / `name = ` frontmatter field
equals `speccy-decompose`.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Rename resource templates that generate installed skills

The template files under `resources/agents/` and
`resources/modules/phases/` that produce the installed skill
artifacts during `speccy init` are renamed in lock-step with
REQ-001 so `speccy init` writes the new names on fresh installs.
This covers the four `.tmpl` files mirroring the install paths and
the phase body `resources/modules/phases/speccy-tasks.md`, plus any
internal references inside those templates.

<done-when>
- `resources/agents/.claude/skills/speccy-decompose/SKILL.md.tmpl`,
  `resources/agents/.agents/skills/speccy-decompose/SKILL.md.tmpl`,
  `resources/agents/.claude/agents/speccy-decompose.md.tmpl`, and
  `resources/agents/.codex/agents/speccy-decompose.toml.tmpl` exist.
- `resources/modules/phases/speccy-decompose.md` exists in place of
  `resources/modules/phases/speccy-tasks.md`.
- Template bodies contain no `speccy-tasks` token.
</done-when>

<behavior>
- Given a fresh project, when `speccy init` runs against it, then the
  installed skill artifacts use the `speccy-decompose` name and no
  `speccy-tasks` file is created.
</behavior>

<scenario id="CHK-003">
Given the working tree at HEAD,
when `speccy init` runs in a tempdir against a stub `AGENTS.md`,
then the resulting `.claude/skills/`, `.claude/agents/`,
`.agents/skills/`, and `.codex/agents/` trees contain
`speccy-decompose` entries and no `speccy-tasks` entries.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Update cross-skill and documentation references

Every reference to `/speccy-tasks` or `speccy-tasks` outside the
rename targets themselves is updated to `/speccy-decompose` /
`speccy-decompose`. This covers shipped skill bodies that suggest
the next step in the loop (`speccy-plan`, `speccy-brainstorm`,
`speccy-orchestrate`, `speccy-work`, and any others discovered
during implementation), `docs/ARCHITECTURE.md` (the
pinned-phase-workers enumeration, invocation examples, and the
component table), `README.md` (the loop diagram, the phase table,
and any prose), and the per-skill model-config table.

<done-when>
- Searching the working tree (excluding `.speccy/archive/`,
  `target/`, and `.git/`) for the literal `speccy-tasks` returns
  zero matches.
- Cross-skill "next step" suggestions that previously named
  `/speccy-tasks` now name `/speccy-decompose`.
- `docs/ARCHITECTURE.md` and `README.md` render with the new name
  in every diagram, table, and prose mention.
</done-when>

<behavior>
- Given a developer reading `README.md`, when they reach the loop
  diagram, then the phase-2 box reads `/speccy-decompose` (not
  `/speccy-tasks`).
- Given the `/speccy-plan` skill body, when an agent reaches the
  final "suggest next step" instruction, then it suggests
  `/speccy-decompose SPEC-NNNN`.
</behavior>

<scenario id="CHK-004">
Given the working tree at HEAD,
when a recursive grep for `speccy-tasks` runs over the tree
excluding `.speccy/archive/`, `target/`, and `.git/`,
then no matches are returned.
</scenario>

<scenario id="CHK-005">
Given the renamed `resources/modules/skills/speccy-plan.md` and
`resources/modules/skills/speccy-brainstorm.md`,
when each file is grepped for `/speccy-decompose`,
then at least one match is returned in each file (the
"suggest the next step" line).
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Update init integration tests to assert the new name

The init integration tests under `speccy-cli/tests/` that assert
the shape of the installed skill pack (`init.rs`,
`init_phase_agents.rs`, `pin_shape.rs`) reference
`speccy-decompose` in every path, slash-prefixed invocation, and
bare slug assertion. After the rename, `cargo test --workspace`
passes with zero failures and zero warnings.

<done-when>
- No test file under `speccy-cli/tests/` contains the literal
  `speccy-tasks`.
- Path assertions previously checking
  `.claude/skills/speccy-tasks/SKILL.md` now check
  `.claude/skills/speccy-decompose/SKILL.md`.
- `cargo test --workspace` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0.
- `cargo +nightly fmt --all --check` exits 0.
- `cargo deny check` exits 0.
</done-when>

<behavior>
- Given the rename commit, when CI runs the standard hygiene suite,
  then all four checks pass.
</behavior>

<scenario id="CHK-006">
Given the working tree at HEAD after this SPEC lands,
when `cargo test --workspace` runs,
then it exits 0 and no test is filtered out as `ignored` that
was not previously ignored on `main`.
</scenario>

<scenario id="CHK-007">
Given the same tree,
when each of `cargo clippy --workspace --all-targets --all-features
-- -D warnings`, `cargo +nightly fmt --all --check`, and
`cargo deny check` runs,
then each exits 0.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
The rename is forward-only with no alias for the old name. Skill
names are not persisted in `SPEC.md` / `TASKS.md` artifacts (only
the per-task journal frontmatter references skills via the agent
identifier, which is set by the host harness at dispatch time, not
by the skill name itself), so completed work does not need
rewriting. Maintaining `speccy-tasks` as a deprecated alias would
violate the project's standing rule against backwards-compatibility
shims (AGENTS.md "Conventions for AI agents").
</decision>

<decision id="DEC-002">
Files are moved with `git mv` (or platform-equivalent rename that
preserves history) rather than deleted-and-recreated, so
`git log --follow` on the new paths surfaces the full history.
This matters because the phase-2 body has accumulated meaningful
changes through earlier specs and severing that history would make
future archaeology harder.
</decision>

<decision id="DEC-003">
This SPEC does not co-rename `speccy-work` to `speccy-implement`
even though the borderline case was discussed. `speccy-work` is
the established name through SPEC-0033 with heavy downstream
references and no observed user confusion. Bundling a second
rename would multiply the diff surface for marginal gain.
A follow-up SPEC may revisit `speccy-work` if dogfooding
surfaces actual confusion.
</decision>

## Notes

The companion bug discovered while scoping this rename — the CLI
suggesting a deleted `speccy plan` subcommand from `speccy next`'s
terminal stderr — was fixed directly in commit `d7fc3dc` ahead of
this SPEC, not folded in. That fix also covered a status-render
bug where `--include-archive` short-circuited on archive-only
workspaces. Those changes are out of scope here and intentionally
already landed.

## Changelog

<changelog>
| Date       | Author      | Summary                                  |
|------------|-------------|------------------------------------------|
| 2026-05-25 | kevin+claude | initial draft                           |
</changelog>
