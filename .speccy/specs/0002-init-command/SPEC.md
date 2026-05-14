---
id: SPEC-0002
slug: init-command
title: speccy init -- scaffold workspace, detect host, copy skill pack
status: implemented
created: 2026-05-11
---

# SPEC-0002: speccy init

## Summary

`speccy init` is the user's entry point: a single command that takes
a developer from `cd new-repo` to ready-to-run. It scaffolds the
`.speccy/` workspace, detects which host (Claude Code, Codex) the
project is set up for, and copies the matching skill pack to the
host-native location so the user can invoke `/speccy:plan`,
`/speccy:work`, etc. immediately.

The command is idempotent-friendly: it refuses to run on an existing
`.speccy/` workspace unless `--force` is passed, and even with
`--force` it only overwrites speccy-shipped files, never user-authored
ones. Exit codes are deliberate so CI scripts and harnesses can
distinguish user errors from internal failures.

Skill packs ship embedded in the binary via `include_dir!`. There is
no runtime download, no network access, no host configuration file --
the choice of pack is determined entirely by `--host` and project
filesystem signals.

## Goals

- Single command takes a fresh repo to a working speccy workspace.
- Refusal/overwrite semantics are predictable for repeat runs and CI.
- Exit codes cleanly distinguish user error (1) from internal failure
  (2) so harnesses can act on them.
- Skill content is versioned with the binary -- updating skills means
  releasing a new speccy.

## Non-goals

- No interactive prompts. `speccy init` never asks the user a question.
- No remote skill fetching. The bundle is compile-time embedded.
- No host other than Claude Code and Codex in v1. `.cursor/` is
  detected but refuses cleanly; future specs add cursor support.
- No mutation of project files outside `.speccy/` and the host's
  native skill directory (`.claude/skills/` for Claude Code or
  `.agents/skills/` for Codex, per SPEC-0015).

## User stories

- As a solo developer starting a new repo, I want `speccy init` to
  scaffold `.speccy/` and place the Claude Code skill files where
  Claude Code will find them, with no further setup.
- As a Codex user, I want `--host codex` to work without me knowing
  exactly where Codex expects its skill files.
- As a CI maintainer, I want `speccy init` to fail with exit code 1
  if the workspace already exists, so my pipeline doesn't silently
  re-init.
- As a Cursor user, I want a clear message that v1 does not yet ship
  a Cursor pack, rather than silent fallback to a different host's
  skills.

## Requirements

### REQ-001: Scaffold the `.speccy/` workspace

Create `.speccy/speccy.toml` and `.speccy/VISION.md` with template
content when neither exists.

**Done when:**
- `.speccy/speccy.toml` is written with `schema_version = 1` and a
  `[project]` block; `name` is the parent directory's name.
- `.speccy/VISION.md` is written with all the template sections
  defined in `.speccy/ARCHITECTURE.md` "VISION.md" section: Product, Users,
  V1.0 outcome, Constraints, Non-goals, Quality bar, Known unknowns.
- Both files validate against the SPEC-0001 parser without errors.

**Behavior:**
- Given a fresh repo at `/foo/bar` with no `.speccy/`, when
  `speccy init` runs successfully, then `.speccy/speccy.toml` exists
  with `name = "bar"`.
- Given a fresh repo, when `speccy init` runs, then `.speccy/VISION.md`
  exists and contains the heading `## Product`, `## Users`, and the
  remaining template sections in declared order.
- Given the templates change between speccy releases, when `speccy
  init` runs, then the scaffolded files reflect the templates baked
  into the current binary.

**Covered by:** CHK-001, CHK-002

### REQ-002: Existence check and `--force` semantics

Refuse to run if `.speccy/` already exists unless `--force` is
passed. Always print the list of files that would be created or
overwritten before mutating.

**Done when:**
- Without `--force`, an existing `.speccy/` causes exit code 1 and a
  stderr message naming the conflicting path.
- With `--force`, speccy-shipped files in `.speccy/` and the host's
  skill directory are overwritten with the current binary's content.
- User-authored files in the host's skill directory (any file whose
  name is not in the shipped bundle) are never touched.
- A summary of "would create" / "would overwrite" lines prints to
  stdout before any file is written.

**Behavior:**
- Given `.speccy/speccy.toml` exists, when `speccy init` runs without
  `--force`, then exit code is 1 and stderr contains the path string
  `.speccy/`.
- Given `.speccy/speccy.toml` exists, when `speccy init --force`
  runs, then `.speccy/speccy.toml` is overwritten with the fresh
  template content.
- Given `.claude/skills/speccy-plan/SKILL.md` and
  `.claude/skills/my-personal-skill/SKILL.md` both exist, when
  `speccy init --force` runs, then `speccy-plan/SKILL.md` is
  overwritten (it's shipped) and `my-personal-skill/SKILL.md` is
  left byte-identical.
- Given any successful run, when `speccy init` finishes, then a
  "Created N files" or "Overwrote N files" summary appears on stdout.

**Covered by:** CHK-003, CHK-004, CHK-005

### REQ-003: Host detection precedence

Detect the host from project signals; allow `--host <name>` override.
Refuse cleanly when `.cursor/` is detected without an explicit
override.

**Done when:**
- `--host <name>` always wins, regardless of which host directories
  exist.
- Without `--host`: probe in order `.claude/`, `.codex/`, `.cursor/`.
  First match wins.
- `.cursor/` detection (without `--host` override) exits 1 with a
  message stating v1 ships no Cursor pack and suggesting
  `--host claude-code` or `--host codex` explicitly.
- No host directories and no `--host`: fall back to `claude-code`
  with a printed warning naming the chosen host and the reason.
- `--host` with an unsupported name exits 1 listing supported names
  (currently `claude-code`, `codex`).

**Behavior:**
- Given a repo with `.claude/`, when `speccy init` runs, then the
  Claude Code skill pack is chosen and copied.
- Given a repo with both `.claude/` and `.codex/`, when `speccy init`
  runs, then `.claude/` wins (probed first).
- Given a repo with only `.cursor/`, when `speccy init` runs, then
  exit code is 1 and stderr names cursor + suggests the
  `--host claude-code` or `--host codex` override.
- Given `speccy init --host cursor`, then exit code is 1 with the
  same message (cursor is not a supported `--host` value in v1).
- Given a repo with no host directories, when `speccy init` runs,
  then claude-code is chosen and a warning is printed on stderr.

**Covered by:** CHK-006

### REQ-004: Skill-pack copy

Copy `skills/<host>/*` from the embedded bundle into the host-native
*skills* location, preserving the per-skill `<name>/SKILL.md`
directory shape so the pack is discoverable as host-native skills
(SPEC-0015 supersedes the original flat `.claude/commands/` layout).

**Done when:**
- For host `claude-code`, files are copied to `.claude/skills/<name>/`.
- For host `codex`, files are copied to `.agents/skills/<name>/`.
- The destination directory is created (recursively) if missing.
- File contents in the destination are byte-identical to the
  embedded bundle.
- Shared resources (`skills/shared/personas/`, `skills/shared/prompts/`)
  are also copied: personas to a location prompt-rendering commands
  (SPEC-0005..0011) can find them; prompts likewise.

**Behavior:**
- Given a fresh repo with `.claude/`, when `speccy init` runs, then
  every `SKILL.md` file in the embedded `skills/claude-code/` tree
  has a byte-identical counterpart under
  `.claude/skills/<name>/SKILL.md`.
- Given a fresh repo with no `.codex/`, when
  `speccy init --host codex` runs, then `.agents/skills/` is
  created and populated with `<name>/SKILL.md` per shipped skill.
- Given the embedded bundle contains a file
  `skills/shared/personas/reviewer-security.md`, when `speccy init`
  runs, then a project-local override location holds that file (the
  exact path is the implementer's choice within `.speccy/skills/`
  per ARCHITECTURE.md "Persona file resolution").

**Covered by:** CHK-007, CHK-008

### REQ-005: Exit codes

Predictable exit codes for CI and harnesses.

**Done when:**
- `0` on success (workspace scaffolded; skill pack copied).
- `1` on user error: existing workspace without `--force`; unknown
  `--host` value; `.cursor/` detected without override.
- `2` on internal failure: I/O error reading embedded resources or
  writing the destination.
- Exit-1 messages are actionable: name the offending condition and
  the corrective action.
- Exit-2 messages include the underlying `std::io::Error` for debugging.

**Behavior:**
- Given a fresh repo, when `speccy init` runs to completion, then
  exit code is 0.
- Given `.speccy/` exists without `--force`, then exit code is 1.
- Given `--host unknown`, then exit code is 1 and stderr lists
  `claude-code, codex`.
- Given a read-only project root (simulated permissions denied),
  when `speccy init` attempts to write, then exit code is 2 and
  stderr contains the underlying I/O error message.

**Covered by:** CHK-009

## Design

### Approach

The command lives in `speccy-cli/src/init.rs`. The embedded skill
bundle is set up via the `include_dir!` macro at `speccy-cli/src/embedded.rs`
which exposes a static `Dir` tree the copy logic walks.

Host detection is a small pure function in `speccy-cli/src/host.rs`
that takes `(host_flag: Option<&str>, project_root: &Path)` and
returns `HostChoice` (or an error variant). The scaffold writer and
the skill-pack copier are independent modules so they can be tested
in isolation.

The CLI binary uses whatever arg parser the implementer prefers (clap
is conventional). Subcommand dispatch lives in `main.rs`.

### Decisions

#### DEC-001: Embedded resources via `include_dir!`

**Status:** Accepted
**Context:** The CLI must ship skill packs without runtime downloads.
**Decision:** Use the `include_dir!` macro to embed `skills/` into
the binary at compile time. The macro exposes a static `Dir` tree the
init command walks.
**Alternatives:**
- `rust-embed` -- rejected. Adds runtime features (compression, hot
  reload) we don't benefit from; more dependency surface.
- Bake assets into `const &[u8]` arrays manually -- rejected. Doesn't
  scale; loses path semantics.
**Consequences:** Skill packs are versioned with the binary.
Updating skills requires releasing a new speccy version. Acceptable
in v1; revisit if skill iteration cadence outpaces release cadence.

#### DEC-002: Cursor detected but unsupported in v1

**Status:** Accepted
**Context:** `.speccy/ARCHITECTURE.md` lists `.cursor/` as a host signal
but no `skills/cursor/` pack ships in v1. Silent fallback would
violate "surface unknowns; never invent" (CLAUDE.md).
**Decision:** Detect `.cursor/` as a host signal but refuse to
proceed without `--host claude-code` or `--host codex`. Print a clear
message stating Cursor support is planned but not in v1.
**Alternatives:**
- Silent fallback to claude-code -- rejected. Surprising.
- Drop cursor detection -- rejected. Cursor users deserve a useful
  diagnostic, not a confused-fallback experience.
- Copy claude-code pack into `.cursor/rules/` -- rejected. Cursor's
  skill loading conventions differ from Claude Code's; cross-host
  re-use is the cursor pack's job when it lands.
**Consequences:** Cursor users get an explicit, actionable message.
A future spec adds `skills/cursor/` and removes this DEC.

#### DEC-003: `--force` scope limited to speccy-shipped files

**Status:** Accepted
**Context:** A user may author their own skills in `.claude/commands/`
alongside speccy-shipped skills. `--force` must not destroy their
work.
**Decision:** `--force` overwrites only files whose names match the
shipped bundle. Non-conflicting files in the destination are left
byte-identical.
**Alternatives:**
- Overwrite everything in the destination -- rejected. Destructive.
- Refuse if any non-speccy file exists in the destination --
  rejected. Too restrictive for projects that mix skills.
**Consequences:** `--force` is safe to run repeatedly. The
"speccy's own files only" rule is the principle of least surprise.

#### DEC-004: Host detection probe order

**Status:** Accepted
**Context:** Multiple host directories may exist in the same repo (a
developer who uses both Claude Code and Codex, say). Without a clear
probe order the chosen host depends on filesystem iteration order.
**Decision:** Probe in declared order: `.claude/`, `.codex/`,
`.cursor/`. First match wins.
**Alternatives:**
- Alphabetical -- rejected. Equivalent to the above but less explicit.
- Error if multiple host directories exist -- rejected. Too pushy;
  many repos have both.
**Consequences:** Claude Code wins when ambiguous. Users with mixed
setups can override via `--host`.

### Interfaces

```rust
pub fn run(args: InitArgs) -> Result<(), InitError>;

pub struct InitArgs {
    pub host: Option<String>,
    pub force: bool,
}

pub enum HostChoice {
    ClaudeCode,
    Codex,
}

pub enum InitError {
    WorkspaceExists { path: PathBuf },
    UnknownHost { name: String, supported: &'static [&'static str] },
    CursorDetected,
    Io(std::io::Error),
}
```

The CLI maps `InitError` variants to exit codes:
- `WorkspaceExists`, `UnknownHost`, `CursorDetected` -> exit code 1.
- `Io(_)` -> exit code 2.

### Data changes

- New `speccy-cli/src/init.rs` (command logic).
- New `speccy-cli/src/host.rs` (detection).
- New `speccy-cli/src/embedded.rs` (include_dir! root).
- New `speccy-cli/src/templates/` (VISION.md template, speccy.toml
  template).
- `speccy-cli/Cargo.toml` adds `include_dir` and an arg-parsing
  crate.
- `skills/claude-code/`, `skills/codex/`, `skills/shared/` directories
  exist at the repo root (initially stub content via `.gitkeep`;
  SPEC-0013 fills them in).

### Migration / rollback

Greenfield command. Rollback is `git revert` of the introducing
commit; no data migration since the command creates net-new files.

## Open questions

- [ ] Should `speccy init` print a "next steps" hint after success
  (e.g. "Run /speccy:plan in your host to start")? Likely yes;
  defer the exact wording to first dogfood pass.
- [ ] Should `--force` regenerate `VISION.md` if it already exists?
  Pragmatic answer: only if the existing file is byte-identical to
  the unmodified template (hash compare). Defer the exact policy to
  implementer.

## Assumptions

- The embedded skill bundle exists at build time under
  `skills/claude-code/`, `skills/codex/`, `skills/shared/personas/`,
  and `skills/shared/prompts/`. Initial implementation can use stub
  content (one-line markdown files); SPEC-0013 fills the real content.
- Arg parsing uses `clap` or similar; the exact crate is
  implementer's choice and not part of the public contract.
- The CLI runs from any cwd; the project root is detected as the
  current working directory unless overridden by a flag (not in v1).

## Changelog

| Date       | Author       | Summary |
|------------|--------------|---------|
| 2026-05-11 | human/kevin  | Initial draft from ARCHITECTURE.md decomposition (bootstrap of speccy). |
| 2026-05-14 | agent/claude | REQ-004 destinations updated by SPEC-0015. Claude Code pack moves from `.claude/commands/` to `.claude/skills/<name>/SKILL.md`; Codex pack moves from `.codex/skills/` to `.agents/skills/<name>/SKILL.md` per OpenAI's official Codex scan paths. Layout shifts from flat `<verb>.md` files to SKILL.md directory format. Pre-v1: no shipped-install migration needed. |

## Notes

SPEC-0002 has a soft dependency on SPEC-0013 (skill packs): the copy
mechanism needs files to copy. The two specs can be in flight in
parallel -- SPEC-0002 ships the mechanism with stub skill content;
SPEC-0013 fills the real skill content into the same directories.
Neither blocks the other so long as the directory tree at
`skills/<host>/` exists.

The cursor-refusal behaviour (DEC-002) is a v1-specific stance.
When `skills/cursor/` lands, remove DEC-002, remove the CursorDetected
error variant, and let host detection identify cursor like any other
host.
