---
id: SPEC-0015
slug: host-skill-layout
title: Host skill packs use SKILL.md directory format
status: implemented
created: 2026-05-14
supersedes: []
---

# SPEC-0015: Host skill packs use SKILL.md directory format

## Summary

Speccy ships skill packs into the host-native skill directory so
that the loop skills are invokable in the user's agent immediately
after `speccy init`. Pre-v1 (no public release yet), both shipped
host packs use a flat-file layout:
`skills/claude-code/speccy/<verb>.md` copies to
`.claude/commands/speccy/<verb>.md`;
`skills/codex/speccy/<verb>.md` copies to
`.codex/skills/speccy/<verb>.md`.

Two problems with that layout, surfaced while reviewing speccy's
own skills against the host vendors' published skill formats:

1. **Codex's skill discovery is broken.** Per OpenAI's Codex docs,
   Codex scans `.agents/skills/` and walks it for any sub-directory
   containing a `SKILL.md` file. Speccy's Codex pack ships
   `init.md`, `plan.md`, etc. directly under `.codex/skills/`,
   which Codex does not scan, and the files aren't named
   `SKILL.md` anyway. The pack therefore never appears in Codex's
   skill selector. It works only as a flat prompt collection, not
   as skills.
2. **Claude Code's pack is installed as slash commands, not as
   skills.** Files at `.claude/commands/<name>.md` are slash
   commands. Claude Code skills proper live at
   `.claude/skills/<name>/SKILL.md` and gain natural-language
   activation through the description field. Today the speccy
   pack only triggers via the slash-command form typed by the
   user; the harness doesn't reliably consider the pack when the
   user describes intent in prose ("draft a spec for X", "let's
   implement the tasks").

This spec migrates both host packs to the canonical SKILL.md
directory format and updates the install destinations to the
host's published skills directory:

- Claude Code: `.claude/skills/<name>/SKILL.md`
- Codex: `.agents/skills/<name>/SKILL.md` (per OpenAI's official
  Codex docs at `developers.openai.com/codex/skills`)

The bundle source layout is restructured to mirror the install
destinations 1:1.

The change is intentionally narrow: layout, destinations, and
frontmatter shape only. Shared resources (`skills/shared/personas/`,
`skills/shared/prompts/`) are not host skills and stay where they
are. Because v1 has not been released yet, there are no production
installs out there to migrate — the change is a straight cut-over.

## Goals

- Codex pack is discoverable as Codex skills (the actually-broken
  case today).
- Claude Code pack supports natural-language activation by living
  under `.claude/skills/` with a description that names concrete
  trigger phrases.
- Bundle source layout mirrors install destinations 1:1, so a
  contributor reading `skills/<host>/<name>/` sees exactly what
  lands in the user's project.
- Each shipped SKILL.md has the `name` + `description` frontmatter
  fields both hosts require for skill discovery.

## Non-goals

- New host support. Cursor remains deferred per SPEC-0002 DEC-002.
- Plugin manifest / marketplace publishing. Out of scope for v1.
- Preserving the `/speccy:<verb>` colon-namespaced slash command
  form. The colon namespace requires a Claude Code plugin manifest;
  v1 ships skills with hyphenated names (`speccy-plan`,
  `speccy-tasks`, etc.) and the slash form becomes `/speccy-plan`.
- Installing to global Codex paths (`$HOME/.agents/skills/` or
  `$HOME/.codex/skills/`). `speccy init` only writes to
  project-local destinations.
- Installing to `.codex/skills/` as a secondary Codex destination.
  OpenAI's published docs list `.agents/skills/` as the Codex
  project-local scan path; openai/codex's own repo uses
  `.codex/skills/` for its internal dogfood, but that's the CLI's
  self-development setup rather than the documented user-facing
  convention. If `.codex/skills/` later turns out to be the only
  path that works in practice, a follow-up spec adds it.
- Changes to shared personas or shared prompts. They aren't host
  skills and aren't affected.

## User stories

- As a Codex user, after `speccy init` I can ask Codex to "draft a
  spec for X" and Codex's skill selector surfaces `speccy-plan`
  among candidate skills.
- As a Claude Code user, after `speccy init` I can say "implement
  the tasks" without typing a slash command and Claude considers
  the `speccy-work` skill.
- As a speccy contributor, when I look at
  `skills/claude-code/speccy-plan/SKILL.md`, the file is
  byte-identical to what `speccy init` would write at
  `.claude/skills/speccy-plan/SKILL.md` for a user.

## Requirements

### REQ-001: Bundle source layout

Restructure `skills/claude-code/` and `skills/codex/` so each
shipped host skill is a directory containing a `SKILL.md` file.

**Done when:**
- Each of the 7 shipped skills (`speccy-init`, `speccy-plan`,
  `speccy-tasks`, `speccy-work`, `speccy-review`, `speccy-ship`,
  `speccy-amend`) lives at `skills/claude-code/<name>/SKILL.md`
  and `skills/codex/<name>/SKILL.md`.
- The legacy `skills/<host>/speccy/` flat-file directories are
  removed.
- `skills/shared/` is unchanged.
- The embedded bundle (`speccy-cli/src/embedded.rs`) compiles
  without changes to the macro invocation; only the underlying
  directory tree shifts.

**Behavior:**
- Given a checkout of the repo, when `cargo build -p speccy-cli`
  runs, then the embedded bundle exposes
  `claude-code/speccy-plan/SKILL.md` as a walkable file.
- Given the embedded bundle, when iterated for both hosts, then
  exactly 7 `SKILL.md` files exist per host, named after the 7
  speccy verbs.
- Given the embedded bundle, when walked, then no flat-file
  `<verb>.md` exists at `claude-code/speccy/` or `codex/speccy/`.

**Covered by:** CHK-001, CHK-002

### REQ-002: Install destinations

`speccy init` copies each host pack into the host's published
project-local skills directory, preserving the per-skill
directory layout.

**Done when:**
- For host `claude-code`, files copy to
  `.claude/skills/<name>/SKILL.md`.
- For host `codex`, files copy to `.agents/skills/<name>/SKILL.md`
  (the path listed in OpenAI's Codex docs as the project-local
  scan location).
- Destination directories (including the per-skill subdir) are
  created recursively if missing.
- File contents at the destination are byte-identical to the
  embedded bundle.
- `speccy init --force` still preserves user-authored files in
  the host skills directory whose paths aren't part of the
  shipped bundle (SPEC-0002 DEC-003 unchanged in spirit; the
  classification operates on the new layout).

**Behavior:**
- Given a fresh repo with `.claude/`, when `speccy init` runs,
  then `.claude/skills/speccy-plan/SKILL.md` exists with content
  byte-identical to the embedded bundle's
  `claude-code/speccy-plan/SKILL.md`.
- Given a fresh repo with `.codex/`, when `speccy init --host
  codex` runs, then `.agents/skills/speccy-plan/SKILL.md` exists
  with content byte-identical to the embedded bundle's
  `codex/speccy-plan/SKILL.md`.
- Given `.claude/skills/my-personal-skill/SKILL.md` exists, when
  `speccy init --force` runs, then that file is byte-identical
  before and after.

**Covered by:** CHK-003, CHK-004

### REQ-003: SKILL.md frontmatter shape

Every shipped SKILL.md declares `name` and `description` in YAML
frontmatter so both Claude Code and Codex recognise the file as a
skill.

**Done when:**
- Every shipped SKILL.md (both hosts) opens with a YAML
  frontmatter block delimited by `---` on its own line above and
  below.
- The frontmatter contains exactly two required keys: `name` and
  `description`. Other keys may be added later but none are
  required in v1.
- `name` is the hyphenated skill slug (e.g. `speccy-plan`),
  matching the directory name. No colons; no `speccy:` namespace
  prefix. Codex and Claude Code skill names do not use colons in
  v1.
- `description` is a single-line string (no multi-line YAML
  scalars) so it parses identically in both hosts' loaders.

**Behavior:**
- Given a shipped SKILL.md, when parsed as YAML frontmatter, then
  the parser returns a mapping with the keys `name` and
  `description`.
- Given the bundle iterated over both hosts, when each SKILL.md
  is read, then `name` equals the directory the file lives in.
- Given a shipped SKILL.md, when the description is inspected,
  then it is a single-line non-empty string.

**Covered by:** CHK-005

### REQ-004: Description quality for natural-language activation

Each shipped description leads with what the skill does, then
states concrete trigger phrases a user might say. Phase-numbered
internal jargon ("Phase 1.", "Phase 2.") is removed from
descriptions.

**Done when:**
- No shipped description starts with the substring "Phase " (with
  a trailing digit), which is the current anti-pattern.
- Each shipped description contains at least one trigger-phrase
  marker. For v1 the marker is the literal substring "Use when"
  (case-insensitive), which is the convention chosen in this
  spec's design notes.
- Each shipped description is ≤ 500 characters so Codex's ~2%
  context budget for the skills list isn't strained.

**Behavior:**
- Given any shipped SKILL.md, when the description is inspected,
  then it does not match `^Phase \d` (Python regex).
- Given any shipped SKILL.md, when the description is searched
  case-insensitively, then it contains `use when`.
- Given any shipped SKILL.md, when the description's character
  count is measured, then it is at most 500.

**Covered by:** CHK-006

## Design

### Approach

The change is mostly a `git mv` plus a frontmatter rewrite. The
copy mechanism in `speccy-cli/src/init.rs` already walks any tree
the embedded bundle exposes; the bundle restructure plus the
host destination edit in `speccy-cli/src/host.rs` carries most of
the work. New checks live in `speccy-cli/tests/skill_packs.rs`
(the same file that already validates shipped skill content for
SPEC-0014).

The description rewrites use the proposals already drafted while
auditing the pack with the skill-creator skill in the same
session that produced this spec. Each rewrite leads with what the
skill does, then includes a "Use when..." clause naming concrete
user phrases.

### Decisions

#### DEC-001: SKILL.md directory format over flat .md files

**Status:** Accepted
**Context:** Codex discovers skills by walking a skills directory
for sub-directories containing `SKILL.md`. Claude Code skills use
the same shape. Flat `<verb>.md` files at the legacy paths are
not discovered as skills by either host.
**Decision:** Each shipped skill lives in its own directory with
a `SKILL.md` file inside. The directory name is the skill name.
**Alternatives:**
- Keep flat files. Rejected — Codex pack remains undiscoverable.
- Single file with multiple skill blocks. Rejected — not the
  format either host supports.
**Consequences:** Bundle directory depth grows by one level per
host. The bundle's `include_dir!` macro handles arbitrary depth,
so the embedded copy path is unchanged.

#### DEC-002: `.agents/skills/` for Codex (per official docs)

**Status:** Accepted
**Context:** OpenAI's published Codex docs at
`developers.openai.com/codex/skills` list `.agents/skills/` as
the project-local scan path Codex walks (CWD, parent dirs up to
repo root, and `$HOME/.agents/skills/` for user-global). The
openai/codex repo itself uses `.codex/skills/<name>/SKILL.md`
for its own dogfood, and a maintainer in #9682 said skills
"must be placed in a `.codex` path" — but that statement reads
as guidance about the CLI's own development setup, not about
where the documented project-local scan happens. The two
sources disagree on appearance only; the docs are authoritative
for what end-user installs should target.
**Decision:** Ship to `.agents/skills/<name>/SKILL.md`. Match
the documented Codex scan path so the install works for the
broadest population of Codex users without a workaround.
**Alternatives:**
- `.codex/skills/`. Rejected because it's not in the docs'
  scan-path list; risks Codex not finding the pack on a vanilla
  install.
- Ship to both `.codex/skills/` and `.agents/skills/`. Rejected
  — duplicates content and risks Codex treating the same skill
  twice with subtly different parent paths.
**Consequences:** If a Codex setup turns out to scan
`.codex/skills/` and not `.agents/skills/`, that's a follow-up
spec adding the secondary destination. Today's bias is toward
the documented path.

#### DEC-003: Hyphenated names without `speccy:` namespace prefix

**Status:** Accepted
**Context:** Both Claude Code skills and Codex skills use flat
skill names (kebab-case, no colons). The current `speccy:<verb>`
form works because `.claude/commands/speccy/` happens to be
treated as a subdirectory namespace in Claude Code's slash command
loader, but that mechanism is specific to commands and does not
carry over to the skills directory. Preserving the colon would
require a Claude Code plugin manifest, which is out of v1 scope.
**Decision:** Skill names are hyphenated: `speccy-init`,
`speccy-plan`, etc. The slash command form becomes
`/speccy-plan`. Natural-language activation does not depend on
the slash command form.
**Alternatives:**
- Ship as a Claude Code plugin to preserve the colon. Rejected —
  marketplace integration is a separate, larger workstream.
- Flat names without the `speccy-` prefix (`plan`, `tasks`).
  Rejected — collides with other skills users might install.
**Consequences:** Users typing `/speccy:plan` after upgrade will
get "unknown command". This is an incompatibility that must be
called out in the changelog row and surfaced when SPEC-0002's
Changelog is amended to point at this spec.

### Interfaces

```rust
// speccy-cli/src/host.rs
impl HostChoice {
    pub const fn destination_segments(self) -> [&'static str; 2] {
        match self {
            HostChoice::ClaudeCode => [".claude", "skills"], // was [".claude","commands"]
            HostChoice::Codex      => [".agents", "skills"], // was [".codex","skills"]
        }
    }
}
```

Bundle source layout becomes:

```text
skills/
  claude-code/
    speccy-init/SKILL.md
    speccy-plan/SKILL.md
    speccy-tasks/SKILL.md
    speccy-work/SKILL.md
    speccy-review/SKILL.md
    speccy-ship/SKILL.md
    speccy-amend/SKILL.md
  codex/
    (same seven directories)
  shared/
    personas/  (unchanged)
    prompts/   (unchanged)
```

Install destinations:

```text
.claude/skills/speccy-init/SKILL.md   # was .claude/commands/speccy/init.md
.claude/skills/speccy-plan/SKILL.md   # was .claude/commands/speccy/plan.md
... (and so on)

.agents/skills/speccy-init/SKILL.md   # was .codex/skills/speccy/init.md
.agents/skills/speccy-plan/SKILL.md   # was .codex/skills/speccy/plan.md
... (and so on)
```

### Data changes

- `git mv skills/claude-code/speccy/<verb>.md
  skills/claude-code/speccy-<verb>/SKILL.md` for each of 7 verbs.
- Same for `skills/codex/`.
- Each new SKILL.md gains `name: speccy-<verb>` in frontmatter
  (Claude Code pack had no `name` field; Codex pack had
  `name: speccy:<verb>` which becomes `name: speccy-<verb>`).
- Each description rewritten per REQ-004's shape (drafts already
  authored in the planning session).
- `speccy-cli/src/host.rs` `destination_segments` table:
  `ClaudeCode` row changes from `["commands"]` to `["skills"]`.
- `speccy-cli/src/embedded.rs` doc-comment refreshed to reflect
  the new layout (the macro invocation itself is unchanged).
- Existing tests in `speccy-cli/tests/skill_packs.rs` and
  `speccy-cli/tests/init.rs` updated to assert the new paths.
  New CHK-001..CHK-006 tests added alongside.

### Migration / rollback

- Forward: ship as one PR. No production installs exist yet
  (pre-v1), so there's nothing to migrate. The repo itself
  re-runs `speccy init --force` after the bundle restructure to
  pick up the new layout under `.claude/skills/`, and
  `speccy init --force --host codex` to seed `.agents/skills/`
  for dogfood. The legacy `.claude/commands/speccy/` directory
  is removed by hand as part of the same PR.
- Rollback: `git revert` of the introducing commit. The legacy
  bundle layout is restored. Anyone who installed the new
  layout in their working tree re-runs `speccy init --force` to
  pick up the reverted pack.

## Open questions

- [ ] Should we also write to `.codex/skills/` (in addition to
      `.agents/skills/`) for Codex, to cover Codex setups that
      scan the legacy path? Defer to a follow-up spec; only
      worth doing once we have a Codex install where the
      documented `.agents/skills/` path is provably not scanned.

## Assumptions

- Both Claude Code and Codex skill loaders accept the YAML
  frontmatter shape `name: <slug>` / `description: <one-line
  string>` with `---` delimiters. Verified against OpenAI's
  Codex docs and Anthropic's Claude Code docs in 2026-05.
- The embedded bundle's `include_dir!` macro continues to walk
  arbitrary directory depth without configuration. Verified in
  SPEC-0002 implementation; no behavioural change expected.
- Codex's skill discovery walks any directory under
  `.agents/skills/` for sub-directories with `SKILL.md` inside,
  per the published scan paths at
  `developers.openai.com/codex/skills`. The 7 speccy skills do
  not need to live directly under `.agents/skills/` — nested
  under `.agents/skills/speccy-*/` is fine.

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-14 | agent/claude | Initial draft from skill-creator audit of speccy's own shipped packs. Replaces flat-file `<verb>.md` layout with SKILL.md directory format; moves Claude Code destination from `.claude/commands/` to `.claude/skills/`; moves Codex destination from `.codex/skills/` to `.agents/skills/` per OpenAI's official Codex docs (`developers.openai.com/codex/skills`). Pre-v1, so no shipped-install migration is in scope. |

## Notes

This spec is itself a dogfood pass: the audit that surfaced the
broken Codex pack was driven by the skill-creator skill running
against speccy's own shipped pack. The motivating insight — that
files at `.claude/commands/` are slash commands, while skills
proper live at `.claude/skills/` and gain natural-language
activation — came from cross-referencing OpenAI's Codex docs and
Anthropic's Claude Code docs against speccy's flat-file pack.

SPEC-0002 REQ-004 previously said destinations are
`.claude/commands/` (Claude Code) and `.codex/skills/` (Codex).
This spec changes both: Claude Code to `.claude/skills/`, Codex
to `.agents/skills/`. SPEC-0002's REQ-004 grows a Changelog row
pointing at SPEC-0015 in the same PR; that amendment is
mechanical (one wording change + one Changelog row).

The shipped skill descriptions deliberately reference user
phrases ("draft a spec", "implement the tasks", "review what was
built") so the natural-language activation has a hook even when
the user doesn't mention "speccy" by name. Future iterations can
optimise the descriptions further via the skill-creator's
description-optimization loop; for v1 the descriptions drafted
in this session are the starting point.
