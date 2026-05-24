---
id: SPEC-0044
slug: force-overwrites-all-shipped-files
title: "`speccy init --force` overwrites all shipped files; reviewer-persona carve-out is removed"
status: implemented
created: 2026-05-24
supersedes: []
---

# SPEC-0044: `speccy init --force` overwrites all shipped files; reviewer-persona carve-out is removed

## Summary

`speccy init --force` today carves out twelve files
(`.claude/agents/reviewer-<persona>.md` and
`.codex/agents/reviewer-<persona>.toml` for the six personas in
`speccy_core::personas::ALL`) and classifies them Skip-on-exists.
Every other shipped file is overwritten. SPEC-0027 REQ-002 / DEC-002
chose this carve-out to protect user edits to persona bodies; the
documented recovery path was `rm <file> && speccy init --force` to
restore a shipped default.

Dogfooding the v1 surface surfaced a different cost. Users reach for
`--force` when they want a clean ejection — "blow away whatever I had,
re-render from the shipped bundle." The carve-out makes that
guarantee partial in a way that is invisible from the CLI invocation;
the user only discovers the asymmetry by reading the plan summary and
spotting `unchanged` next to a reviewer agent file they expected to
see overwritten. The recovery loop (`rm` then `--force`) is one extra
step per persona file the user wants to reset, and it is not
discoverable without reading SPEC-0027 or `src/init.rs`.

This SPEC removes the carve-out. After it lands, `speccy init --force`
classifies every rendered host-pack file uniformly: absent → Create,
byte-identical → Unchanged, differs → Conflict (refuse without
`--force`, overwrite with `--force`). The twelve reviewer agent files
participate in that rule with no exception. Users who customise a
persona body keep their edits via the same mechanism they already use
for any other shipped file under version control: commit before
running `--force`, or rely on `git stash` / `git restore` to recover
afterwards.

The CLI code change is small (delete one helper and the branch in
`append_host_pack_items` that calls it). The documentation surface
change is larger: `docs/ARCHITECTURE.md` mentions the Skip-on-exists
behaviour in three places, the `InitArgs::force` docstring describes
it, the `Action` enum's docstring catalogues it, and the
`init_three_way.rs` / `init.rs` test suites contain assertions that
the carve-out holds. All of these must move together so that the
shipped code, the architecture doc, and the test fixtures agree on
one rule.

This SPEC ships as a pre-v1.0.0 simplification of the `--force`
contract. SPEC-0027 stays archived; its REQ-001 (no `.speccy/skills/`),
REQ-003 (no persona body in CLI-rendered prompts), and REQ-004
(resolver chain removed) remain in effect. Only REQ-002 / DEC-002 of
SPEC-0027 are reversed by this work — the rest of SPEC-0027's
direction is unchanged.

## Goals

<goals>
- `speccy init --force` against a workspace where
  `.claude/agents/reviewer-business.md` (or any other persona file
  shipped via either host pack) contains a user edit overwrites that
  file back to the shipped bundle content with no further user
  action.
- The init plan summary printed to stdout shows `(!) overwritten`
  for any reviewer agent file whose on-disk content differs from the
  shipped bundle, identical to how it treats `.claude/skills/speccy-*/SKILL.md`
  and every other rendered host-pack file.
- The `is_host_native_reviewer_file` helper and the matching branch
  inside `append_host_pack_items` (`speccy-cli/src/init.rs`) are
  deleted; the classification path uses `classify_content` uniformly
  for every rendered host-pack file.
- `docs/ARCHITECTURE.md` no longer states that host-native reviewer
  files are Skip-on-exists; references to the carve-out are deleted
  or replaced with the new uniform rule.
- The `Action` enum and `InitArgs::force` docstrings in
  `speccy-cli/src/init.rs` describe one rule for `--force`: every
  rendered host-pack file is Create / Unchanged / Conflict, with no
  per-file exception.
</goals>

## Non-goals

<non-goals>
- No new `--force-personas` or `--reset-personas` flag. The uniform
  rule replaces the carve-out outright; a two-tier surface would
  preserve the confusion this SPEC removes.
- No CLI-side backup, copy-to-`.bak`, or diff-print mechanism. Users
  who care about persona-body edits commit them or stash them
  before running `--force`; `speccy init` does not grow a snapshot
  responsibility.
- No change to the rendering pipeline (`render_host_pack`,
  MiniJinja templates under `resources/agents/`). The bytes the
  renderer emits for `.claude/agents/reviewer-business.md` are
  unchanged; only the classification of the destination changes.
- No change to SPEC-0027 REQ-001 (no `.speccy/skills/`), REQ-003
  (CLI prompts carry no persona body), or REQ-004 (resolver chain
  removed). Only REQ-002 / DEC-002 of SPEC-0027 are reversed.
- No change to the `Action` enum variants. `Create`, `Unchanged`,
  and `Conflict` remain the three classifications; the carve-out
  was implemented inside `append_host_pack_items`, not as an
  `Action` variant.
- No migration helper that copies pre-existing user edits anywhere
  before overwriting. Users who want their edits preserved use
  git; the CLI does not impersonate a version-control system.
</non-goals>

## User Stories

<user-stories>
- As a solo developer who ran `speccy init` months ago and has since
  forgotten about it, I want `speccy init --force` to reset the
  whole `.claude/agents/` and `.codex/agents/` directories to the
  shipped bundle in one command. I should not have to read
  ARCHITECTURE.md to discover that twelve specific files need to
  be deleted first.
- As a user upgrading from a pre-SPEC-0044 speccy CLI, I want the
  first `speccy init --force` run after the upgrade to either
  succeed cleanly (if my reviewer files are byte-identical to the
  shipped bundle) or refuse atomically (if they differ and I have
  not passed `--force`). The two-tier outcome ("succeeded but
  silently skipped twelve files") goes away.
- As a contributor reading `speccy-cli/src/init.rs` to understand
  how `--force` works, I want one classification rule that applies
  to every file in the plan. The current per-path exception
  (`is_host_native_reviewer_file`) is invisible from the call site
  and forces a second function read to confirm what `--force`
  means.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `speccy init --force` overwrites host-native reviewer agent files when they differ from the shipped bundle

`speccy init --force` classifies `.claude/agents/reviewer-<persona>.md`
and `.codex/agents/reviewer-<persona>.toml` (for any `<persona>` in
`speccy_core::personas::ALL`) using the same `classify_content`
function as every other rendered host-pack file. When the on-disk
content differs from the shipped bundle, the action is `Conflict`;
under `--force` the file is overwritten and the plan summary records
`(!) overwritten`. The branch in `append_host_pack_items` that
classified these paths as `Unchanged` on exists is removed; the
`is_host_native_reviewer_file` helper is deleted.

<done-when>
- Given a workspace initialized once via `speccy init`, when a user
  appends a sentinel line to `.claude/agents/reviewer-business.md`
  and then runs `speccy init --host claude-code --force`, the file
  on disk after the run is byte-identical to the shipped bundle
  content (no sentinel line remains).
- The plan summary printed to stdout on that same run contains a
  line matching `(!) overwritten\s+.claude/agents/reviewer-business.md`.
- The equivalent flow against `.codex/agents/reviewer-security.toml`
  with `--host codex --force` overwrites the file and prints
  `(!) overwritten` for that path.
- The function `is_host_native_reviewer_file` does not exist in
  `speccy-cli/src/init.rs` after the SPEC lands.
- The `use speccy_core::personas::ALL as PERSONAS_ALL;` import at
  `speccy-cli/src/init.rs:19` is removed; the carve-out helper was
  its sole consumer in that file, so it goes with the helper. The
  `speccy_core::personas::ALL` constant itself remains (consumed by
  `speccy-core/src/next.rs`, `speccy-core/src/parse/journal_xml/mod.rs`,
  and several integration tests outside `init.rs`).
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0 with no `dead_code` or `unused_imports`
  warnings attributable to the removal.
</done-when>

<behavior>
- Given a tempdir initialized once via `speccy init --host
  claude-code` so `.claude/agents/reviewer-business.md` matches the
  shipped bundle exactly, when a user appends `# sentinel-edit-12345`
  to that file and then runs `speccy init --host claude-code
  --force` against the same tempdir, then the file no longer
  contains the substring `sentinel-edit-12345` after the run.
- Given the same tempdir before any user edit, when `speccy init
  --host claude-code --force` runs, then the plan summary line for
  `.claude/agents/reviewer-business.md` reads `unchanged` because
  the on-disk content matches the planned content; no `(!) overwritten`
  appears.
- Given a tempdir initialized once via `speccy init --host codex`
  with `.codex/agents/reviewer-security.toml` containing user edits,
  when `speccy init --host codex --force` runs, then the file is
  restored to the shipped bundle content and the plan summary
  records `(!) overwritten` for that path.
- Given a tempdir initialized once with reviewer files matching the
  shipped bundle, when a user runs `speccy init` (without `--force`)
  after no edits, then every reviewer file is logged `unchanged`
  and the command exits 0 — the uniform rule produces no false
  conflicts when content matches.
</behavior>

<scenario id="CHK-001">
Given a freshly created temporary directory `root`,
when `speccy init --host claude-code` runs with `cwd = root` and
exits 0,
then `root/.claude/agents/reviewer-business.md` exists with the
shipped bundle content.

Given the same `root` after the first init,
when the bytes `\n# sentinel-edit-CHK-001\n` are appended to
`root/.claude/agents/reviewer-business.md`,
and `speccy init --host claude-code --force` runs with `cwd = root`,
then the command exits 0,
and `root/.claude/agents/reviewer-business.md` no longer contains
the substring `sentinel-edit-CHK-001`,
and the captured stdout contains a line whose trimmed start matches
`(!) overwritten` followed by whitespace and the relative path
`.claude/agents/reviewer-business.md`.
</scenario>

<scenario id="CHK-002">
Given a freshly created temporary directory `root`,
when `speccy init --host codex` runs with `cwd = root` and exits 0,
then `root/.codex/agents/reviewer-security.toml` exists with the
shipped bundle content.

Given the same `root`,
when the bytes `\n# sentinel-edit-CHK-002\n` are appended to
`root/.codex/agents/reviewer-security.toml`,
and `speccy init --host codex --force` runs with `cwd = root`,
then the command exits 0,
and `root/.codex/agents/reviewer-security.toml` no longer contains
the substring `sentinel-edit-CHK-002`,
and the captured stdout contains a line whose trimmed start matches
`(!) overwritten` followed by whitespace and the relative path
`.codex/agents/reviewer-security.toml`.
</scenario>

<scenario id="CHK-003">
Given the file `speccy-cli/src/init.rs` after this SPEC lands,
when grepped for the identifier `is_host_native_reviewer_file`,
then no match is found,
and when grepped for the literal string `Skip-on-exists`,
then no match is found.

Given the workspace after this SPEC lands,
when `cargo clippy --workspace --all-targets --all-features -- -D
warnings` runs,
then it exits 0 with no warnings attributable to a now-unused
`PERSONAS_ALL` import in `speccy-cli/src/init.rs`.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Documentation, docstrings, and tests describe one uniform `--force` rule with no per-file exception

`docs/ARCHITECTURE.md` no longer states that host-native reviewer
files are Skip-on-exists. The three current occurrences (around
lines 326, 1684, and 1889) are rewritten to describe the uniform
classification (`Create` / `Unchanged` / `Conflict`) that
`classify_content` applies to every rendered host-pack file. The
`Action` enum docstring (init.rs:74-88), the
`Action::Unchanged` variant docstring (init.rs:93-95), the module
docstring (init.rs:9-10), the `build_plan` comment (init.rs:206-209),
and the `execute_plan` comment (init.rs:328) are rewritten to drop
the per-file exception language. The four `t002_*` tests in
`speccy-cli/tests/init.rs` that assert the carve-out are flipped to
assert the new uniform rule.

<done-when>
- `docs/ARCHITECTURE.md` contains zero occurrences of the literal
  substring `Skip-on-exists`.
- `docs/ARCHITECTURE.md` contains zero occurrences of the literal
  substring `survive` within five lines of a mention of
  `reviewer-` (the prose that previously stated user edits survive
  `--force` is gone).
- `speccy-cli/src/init.rs` contains zero occurrences of the literal
  substring `Skip-on-exists` and zero references to SPEC-0027
  REQ-002 in code comments.
- The tests `t002_claude_reviewer_agent_files_preserve_user_edits_under_force`
  and `t002_codex_reviewer_agent_files_preserve_user_edits_under_force`
  in `speccy-cli/tests/init.rs` are rewritten to assert the
  inverse: under `--force` the sentinel user edit is overwritten.
- The test `t002_claude_init_force_plan_summary_marks_reviewer_agents_and_skills_unchanged`
  is either deleted or rewritten so the assertion is consistent
  with the new uniform rule (a byte-identical reviewer file shows
  `unchanged`; a differing one shows `(!) overwritten`).
- The test `t002_claude_reviewer_agent_files_recreate_when_deleted_under_force`
  remains valid as a Create-on-absent regression guard; its name
  and inline comment lose the `Skip-on-exists` framing.
- `cargo test --workspace` exits 0.
- `cargo +nightly fmt --all --check` exits 0.
</done-when>

<behavior>
- Given `docs/ARCHITECTURE.md` after the SPEC lands, when scanned
  line-by-line, then no line contains the substring `Skip-on-exists`.
- Given `speccy-cli/src/init.rs` after the SPEC lands, when its
  module-level doc comment and the `InitArgs::force` and `Action`
  variant docstrings are read, then they describe one rule
  (Create / Unchanged / Conflict applies uniformly) without naming
  reviewer files as an exception.
- Given `speccy-cli/tests/init.rs` after the SPEC lands, when its
  function list is enumerated, then any surviving test that
  references `.claude/agents/reviewer-` or
  `.codex/agents/reviewer-` asserts overwrite-under-force, not
  preservation.
</behavior>

<scenario id="CHK-004">
Given the file `docs/ARCHITECTURE.md` after this SPEC lands,
when grepped for the literal substring `Skip-on-exists`,
then zero matches are found.

Given the same file,
when grepped (with two lines of context above and below) for the
substring `reviewer-`,
then no surviving prose claims that user edits to those files
survive `--force`.
</scenario>

<scenario id="CHK-005">
Given the source file `speccy-cli/src/init.rs` after this SPEC
lands,
when grepped for the literal substring `Skip-on-exists`,
then zero matches are found.

Given the same source file,
when its `InitArgs::force` docstring and the `Action` enum
docstring are read,
then both describe the uniform Create / Unchanged / Conflict rule
without carve-out language.
</scenario>

<scenario id="CHK-006">
Given the test file `speccy-cli/tests/init.rs` after this SPEC
lands,
when its `#[test]`-annotated functions are enumerated,
then any test whose name begins with `t002_` and references a
reviewer agent path asserts that `--force` overwrites the user
edit (not preserves it), or the test is absent entirely.

Given the workspace after this SPEC lands,
when `cargo test --workspace` runs,
then it exits 0.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001" status="accepted">
### DEC-001: Uniform `--force` rule; no `--force-personas` flag

**Status:** Accepted

**Context:** Two paths to the new behaviour exist: (a) remove the
carve-out entirely so `--force` overwrites every shipped file with
no exception; (b) keep `--force` preserving reviewer personas and
add a new `--force-personas` flag for users who want the
clean-slate behaviour. Path (b) preserves backward compatibility
with SPEC-0027 REQ-002 for users who have already adopted the
carve-out semantics.

**Decision:** Path (a). One flag, one rule. `--force` means
"overwrite every shipped file that differs from the bundle." No
two-tier opt-in surface.

**Alternatives:**
- *Add `--force-personas` and keep the carve-out as the default
  `--force` behaviour.* Rejected: the carve-out's confusion is
  exactly that the flag's meaning is partial. Adding a second
  flag deepens the partial-meaning problem rather than resolving
  it. Users who eject expect "clean slate"; the current name
  `--force` already promises that and should deliver.
- *Remove the carve-out but add a `--preserve-personas` opt-in
  flag for users who want the old behaviour.* Rejected on the
  same grounds: an opt-in flag for a SPEC-0027-style preservation
  is feature growth justified only by hypothetical user demand.
  Speccy v1's scope is "useful for my next greenfield"; until a
  user surfaces the need, the surface stays small.
- *Keep the carve-out and document it more loudly.* Rejected:
  documentation cannot fix a CLI flag whose name implies one
  thing and whose behaviour partially does something else.

**Consequences:** Users who customised persona bodies under the
old behaviour and then run `speccy init --force` post-SPEC will
have their edits overwritten if they did not commit or stash
them first. The recovery story is git, not CLI machinery: commit
before `--force`, restore from history after. This matches every
other file `speccy init` writes (skill bodies, wrappers, etc.).
</decision>

<decision id="DEC-002" status="accepted">
### DEC-002: SPEC-0027 stays archived; no resurrection or amendment

**Status:** Accepted

**Context:** SPEC-0027 is archived (status: implemented, archived
2026-05-23). Its REQ-002 / DEC-002 capture the carve-out this SPEC
reverses. Three handling paths exist: (a) leave SPEC-0027 archived
and let SPEC-0044 supersede REQ-002 / DEC-002 in prose; (b)
un-archive SPEC-0027 and amend it via `speccy-amend`; (c) supersede
SPEC-0027 wholesale by listing it in SPEC-0044's `supersedes`
frontmatter field.

**Decision:** Path (a). SPEC-0044's Summary and Changelog name the
specific REQ / DEC IDs being reversed. The `supersedes` frontmatter
stays empty (matching every other SPEC in this workspace's history,
which has never used a non-empty `supersedes`).

**Alternatives:**
- *Un-archive SPEC-0027 and amend.* Rejected: SPEC-0027's other
  requirements (REQ-001, REQ-003, REQ-004) shipped intact and are
  not being reversed. Amending a four-requirement archived SPEC
  to flip one requirement creates a hybrid state (partly shipped,
  partly in-flight) for a SPEC that is otherwise done. A new SPEC
  with a tight scope is cleaner.
- *List SPEC-0027 in SPEC-0044's `supersedes` field.* Rejected: the
  field implies whole-SPEC supersession; SPEC-0044 reverses one
  REQ and one DEC out of four, not the whole spec. The frontmatter
  would be misleading.

**Consequences:** Future readers tracing the history of `--force`
semantics read both SPECs in sequence: SPEC-0027 introduces the
carve-out with its rationale; SPEC-0044 reverses it with the
dogfood-driven rationale that the carve-out's cost exceeded its
benefit in practice. The archived SPEC remains accurate as a record
of "what shipped at v1 milestone" — SPEC-0044 changes the v1
milestone post-archive but pre-1.0.0 release.
</decision>

<decision id="DEC-003" status="accepted">
### DEC-003: No CLI backup or snapshot machinery before overwrite

**Status:** Accepted

**Context:** Removing the carve-out means users who edited persona
bodies risk losing those edits on the next `--force` run. One
mitigation is for the CLI to write a `.bak` copy of any file
classified `Conflict` before overwriting it, so users have a
recovery path that does not depend on version control.

**Decision:** No. `speccy init` does not impersonate a version
control system. Users who want recovery on overwrite use git
(`git diff`, `git stash`, commit before running). The recovery
story is uniform with every other shipped file the CLI manages.

**Alternatives:**
- *Write a `.bak` copy alongside any overwritten file.* Rejected:
  introduces a new on-disk artifact (`reviewer-business.md.bak`)
  with its own questions (how many backups to keep, when to
  delete, what `.gitignore` advice to give). The complexity ceiling
  exceeds the user need; users who care about edits are already
  using git.
- *Print a per-file diff to stderr before overwriting and prompt
  for confirmation.* Rejected: `speccy init --force` is meant to
  run non-interactively in scripts. Adding a prompt forks the CLI
  into interactive and non-interactive code paths for one specific
  flag combination.
- *Refuse to overwrite if any reviewer file differs, even under
  `--force`, until the user passes `--force-personas`.* Rejected:
  this is DEC-001 path (b) under a different name; same reasons
  apply.

**Consequences:** Users who customise persona bodies must use git
to preserve their edits. Speccy's documentation (the
speccy-init skill body) should mention this workflow once so users
discover it; the CLI does not.
</decision>

## Notes

The brainstorm conversation that produced this SPEC walked through
three framings:

1. **Remove the carve-out outright.** The path this SPEC takes.
2. **Add a `--force-personas` opt-in flag.** Considered and rejected
   in DEC-001; it deepens the partial-meaning problem of `--force`
   rather than resolving it.
3. **Keep the current behaviour and document it more clearly.**
   Considered and rejected; documentation cannot fix a CLI flag
   whose name implies clean-slate but whose behaviour partially
   does something else.

The motivation traces to dogfooding speccy on this repository. The
v1 surface is "useful for my next greenfield"; the carve-out's cost
(invisible partial guarantee, undiscoverable recovery path) outweighs
its benefit (protecting persona-body edits from accidental
overwrite) once users move under version control, which v1 assumes
as the baseline collaboration model.

The simplification mirrors a pattern already established by
SPEC-0023 and SPEC-0027 itself: when a feature carries hidden
machinery that surfaces only via documentation or source-reading,
the simplest fix is usually to delete the machinery rather than to
document it more clearly. SPEC-0027 deleted the
`.speccy/skills/personas/` override directory for the same reason
(half-wired override → confusing); SPEC-0044 deletes the
`is_host_native_reviewer_file` carve-out for the same reason
(per-file exception in `--force` → confusing partial guarantee).

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-24 | human/kevin | Initial draft. Reverses SPEC-0027 REQ-002 / DEC-002. The Skip-on-exists carve-out for `.claude/agents/reviewer-<persona>.md` and `.codex/agents/reviewer-<persona>.toml` (twelve files total across both host packs) is removed; `speccy init --force` now classifies every rendered host-pack file using the same `classify_content` rule (Create / Unchanged / Conflict). The `is_host_native_reviewer_file` helper and its caller branch in `append_host_pack_items` are deleted. `docs/ARCHITECTURE.md` and the `init.rs` docstrings are updated to describe one uniform rule. Tests in `speccy-cli/tests/init.rs` that asserted the carve-out are rewritten to assert the new behaviour. SPEC-0027's REQ-001, REQ-003, and REQ-004 remain in effect; only REQ-002 / DEC-002 are reversed. SPEC-0027 stays archived per DEC-002 of this SPEC. |
</changelog>
