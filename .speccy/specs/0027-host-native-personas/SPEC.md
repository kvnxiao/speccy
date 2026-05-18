---
id: SPEC-0027
slug: host-native-personas
title: Host-native files are the sole canonical persona surface; drop .speccy/skills/ override directory
status: implemented
created: 2026-05-17
supersedes: []
---

# SPEC-0027: Host-native files are the sole canonical persona surface; drop .speccy/skills/ override directory

## Summary

Persona body content currently reaches the reviewer sub-agent twice
on every fan-out. At `speccy init` time, the Jinja template under
`resources/agents/.claude/agents/reviewer-<persona>.md.tmpl` (and its
Codex twin under `resources/agents/.codex/agents/reviewer-<persona>.toml.tmpl`)
inlines the persona body via `{% include "modules/personas/reviewer-<persona>.md" %}`,
so the file that lands on disk at `.claude/agents/reviewer-<persona>.md`
already carries the full persona text. The host pre-loads that file
as the sub-agent's system context when `speccy-review` spawns it.
Then, when the sub-agent runs `speccy review <task> --persona <p>`,
the CLI fills the `{{persona_content}}` placeholder in
`resources/modules/prompts/reviewer-<persona>.md` by calling
`speccy_core::personas::resolve_file`, which inlines the persona body
a second time into the rendered prompt the sub-agent reads as its
task message.

The two paths consult different sources. The Jinja include reads
straight from the embedded `resources/modules/personas/` bundle. The
CLI resolver checks `<project_root>/.speccy/skills/personas/reviewer-<persona>.md`
first and falls back to the embedded bundle. A user who edits the
override file at `.speccy/skills/personas/` only changes the
CLI-rendered half — the host-loaded system context still carries the
embedded default. Half-effective override is the worst of both
worlds: it looks tunable, only partially is, and the partial nature
is invisible from the file tree.

The override directory's sibling, `.speccy/skills/prompts/`, is dead
weight in a different way: `init` writes files there, but
`speccy_core::prompt::load_template` only reads from the embedded
`PROMPTS` bundle. No code consults `.speccy/skills/prompts/`. A user
who edits a prompt template locally gets zero behavioural change.

This SPEC eliminates the double-inlining and the half-wired override
hook in one move. Host-native files (`.claude/agents/reviewer-*.md`
and `.codex/agents/reviewer-*.toml`) become the sole user-tunable
persona surface. They are classified as Skip-on-exists at init time,
so `--force` no longer clobbers user edits to them. The CLI-rendered
reviewer prompt drops the `## Persona` block and its
`{{persona_content}}` substitution entirely: the sub-agent already
has the persona loaded as system context via the host's sub-agent
machinery, so re-inlining is noise. `personas::resolve_file` and its
override-chain helpers are deleted along with the `PERSONAS` static
that backs them. The `.speccy/skills/` write step in `speccy init` is
removed; pre-existing `.speccy/skills/` directories in user
workspaces are left alone (not deleted) since `speccy init` should
not silently destroy files outside its current plan.

The direction mirrors SPEC-0023's choice to stop inlining AGENTS.md
and SPEC.md into rendered prompts (REQ-005 / REQ-006): when host
machinery already delivers content to the agent through some other
channel, the CLI prompt stops carrying a redundant copy. Here the
host channel is the sub-agent system context rather than the host's
Read primitive, but the principle is the same — single source per
content stream, no shadow copies.

## Goals

<goals>
- `speccy init` no longer creates `.speccy/skills/` or any subtree
  under it. The fresh-workspace footprint shrinks; the dead
  `prompts/` half goes away and the half-wired `personas/` override
  goes with it.
- Host-native reviewer files (`.claude/agents/reviewer-<persona>.md`
  for Claude Code; `.codex/agents/reviewer-<persona>.toml` for
  Codex) are classified as Skip-on-exists by `init`. `speccy init`
  creates them on a fresh workspace; `speccy init --force` leaves
  them untouched when they exist. Users who edit persona bodies
  locally keep their edits across re-init.
- CLI-rendered reviewer prompts no longer carry the persona body.
  The `## Persona` block and `{{persona_content}}` placeholder are
  removed from every `resources/modules/prompts/reviewer-<persona>.md`
  template, and the `persona_content` variable insertion is removed
  from `speccy review`'s render-vars map.
- The persona-override resolver and its associated types
  (`personas::resolve_file`, `resolve_file_with_warn`, `PersonaError`,
  the speccy-core-side `PERSONAS` static, the `init.rs` import of
  `PERSONAS`/`PROMPTS`, the `append_user_tunable_dir_items` helper)
  are deleted. `personas::ALL` (the registry of valid persona names
  used by `speccy review --persona` validation) is the only public
  surface of the `personas` module that survives.
- The in-tree `.speccy/skills/` directory is removed from this
  workspace as dogfood proof. Re-running `speccy init --force` after
  the SPEC lands does not recreate it.
</goals>

## Non-goals

<non-goals>
- No splitting of host-native reviewer files into a "frontmatter is
  shipped, body is tunable" two-tier scheme. The whole file is
  Skip-on-exists. If a user clobbers the YAML/TOML frontmatter, they
  recover by deleting the file and re-running `speccy init` — the
  same recovery path users already have for any host-pack file.
- No `--force-personas` flag, no `--reset-agents` flag, no `init`
  sub-mode that selectively re-overwrites the Skip-on-exists files.
  Deleting the file and re-initing is the documented recovery path.
- No active deletion of pre-existing `.speccy/skills/` directories
  in user workspaces during `init`. The directory becomes orphaned
  (nothing reads from it post-SPEC), but `speccy init` should not
  silently remove files outside its current write plan. Users who
  want to clean up run `rm -rf .speccy/skills/` themselves.
- No introduction of a `.speccy/overrides/` directory, no rename of
  `.speccy/skills/` to anything else. The override mechanism goes
  away entirely; renaming a half-wired directory would just preserve
  the confusion under a new name.
- No change to the host-pack render pipeline (`render_host_pack` and
  the MiniJinja loader stay unchanged). Persona body still arrives
  in `.claude/agents/reviewer-*.md` via `{% include %}` at render
  time; only the init-plan classification of the rendered file
  changes (Overwrite → Skip-on-exists).
- No change to `speccy-core/src/prompt/template.rs` or its `PROMPTS`
  static. Prompt templates remain embedded-only; there was no
  override chain to remove because the CLI never consulted one.
- No new `speccy check` lint enforcing "no `.speccy/skills/`
  references in skill bodies". Drift between docs and reality is
  caught by the dogfood proof (running `init --force` produces a
  workspace with no `.speccy/skills/` written).
- No change to the `speccy-review` skill's spawn instructions or its
  default fan-out (business, tests, security, style). The mechanism
  that delivers persona content to sub-agents (the host loading
  `.claude/agents/reviewer-*.md` as system context) is already in
  place and is what this SPEC consolidates on.
- No change to Codex skill body files at `.agents/skills/speccy-*/SKILL.md`.
  Those are the Codex skill pack (per SPEC-0015) and have no persona
  content; the user-tunable surface there is a separate concern
  that this SPEC does not address.
</non-goals>

## User Stories

<user-stories>
- As a solo developer who customizes a reviewer's focus list for my
  project (e.g., adding "watch for prompt-injection vectors in
  tool-use code paths" to the security persona), I want one place to
  edit. I open `.claude/agents/reviewer-security.md`, edit the body
  prose, save. `speccy init --force` on a later run preserves my
  edit. I do not need to know about an override directory, a
  resolver chain, or a double-inlining mechanism.
- As a maintainer of speccy, I want the rendered prompt that
  `speccy review` emits to be the minimum text a sub-agent needs
  beyond what its system context already carries. Persona body is
  delivered by the host as system context; the rendered prompt
  carries only the task-specific delta (which task, which SPEC,
  which diff to fetch, what deliverable). No redundant copy of
  text the sub-agent has already read.
- As a developer auditing the speccy workspace tree, I want every
  on-disk file to have an obvious consumer. `.speccy/skills/personas/`
  having a half-wired consumer and `.speccy/skills/prompts/` having
  no consumer at all is the kind of cruft that quietly accumulates
  cost; after this SPEC, every file under `.speccy/` is read by
  something the CLI or skills actually do.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: `speccy init` no longer writes `.speccy/skills/`

`speccy init` against a fresh workspace produces no
`.speccy/skills/` directory and no files under any subtree of it.
`speccy init --force` against a workspace where `.speccy/skills/`
exists from a prior init (or from manual user creation) leaves the
directory and its contents untouched.

<done-when>
- `speccy init` against an empty directory creates `.speccy/` with
  `speccy.toml` and host-pack files (`.claude/skills/`,
  `.claude/agents/`, etc.) but no `.speccy/skills/` directory.
- The `init` plan summary printed to stdout contains no line
  referencing a path under `.speccy/skills/`.
- The init code path in `speccy-cli/src/init.rs` no longer imports
  `speccy_core::personas::PERSONAS` or `speccy_core::prompt::PROMPTS`,
  no longer calls `append_user_tunable_dir_items`, and the
  `append_user_tunable_dir_items` helper itself is removed from the
  module.
- `speccy init --force` against this workspace (where
  `.speccy/skills/` has been removed by this SPEC) does not
  recreate the directory.
- A test in `speccy-cli/tests/init.rs` asserts that running `init`
  on a tempdir produces no `.speccy/skills/` directory.
</done-when>

<behavior>
- Given an empty directory `tmp/`, when `speccy init` runs with
  `cwd = tmp/`, then `tmp/.speccy/skills/` does not exist after the
  run.
- Given a directory `tmp/` containing a pre-existing
  `.speccy/skills/personas/reviewer-business.md` with arbitrary
  content, when `speccy init --force` runs, then the file is
  present and unchanged after the run (init does not delete it,
  init does not rewrite it).
- Given `speccy init`'s stdout output, when scanned for the
  substring `.speccy/skills/`, then zero matches are found.
</behavior>

<scenario id="CHK-001">
Given a freshly created empty temporary directory used as
`project_root`,
when `speccy init` runs with that directory as `cwd`,
then `project_root.join(".speccy").join("skills")` does not exist
on the filesystem after the command completes.

Given the same fresh tempdir,
when `speccy init` runs and its stdout is captured,
then the captured output contains no occurrence of the literal
substring `.speccy/skills/`.

Given a tempdir pre-populated with
`.speccy/skills/personas/reviewer-business.md` containing the bytes
`pre-existing override\n`,
when `speccy init --force` runs against it,
then the file still exists with byte-for-byte identical content
after the run.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Host-native reviewer files are Skip-on-exists under `--force`

Files rendered to `.claude/agents/reviewer-*.md` (Claude Code) and
`.codex/agents/reviewer-*.toml` (Codex) are classified by the init
plan as Create when absent and Skip when present. `speccy init`
against a fresh workspace creates them; `speccy init --force`
against a workspace where they exist (even with user edits) leaves
them untouched. All other host-pack files (skill wrappers under
`.claude/skills/` and `.agents/skills/`, etc.) continue to be
Overwrite-on-exists under `--force`.

<done-when>
- Init plan classification for any rendered file whose `rel_path`
  matches `.claude/agents/reviewer-<persona>.md` or
  `.codex/agents/reviewer-<persona>.toml` (for `<persona>` in
  `speccy_core::personas::ALL`) is Skip when the destination
  already exists, Create when it does not.
- All other rendered host-pack files retain today's
  Create-or-Overwrite classification.
- The plan summary printed to stdout shows `skip` for these paths
  on `--force` runs against a workspace where the files exist, and
  `create` on first init.
- A test in `speccy-cli/tests/init.rs` asserts: after seeding
  `.claude/agents/reviewer-business.md` with a sentinel string,
  running `init --force` preserves the sentinel.
- A test asserts: deleting `.claude/agents/reviewer-business.md`
  and running `init --force` re-creates it from the embedded
  bundle.
</done-when>

<behavior>
- Given a tempdir already initialized once via `speccy init` (so
  `.claude/agents/reviewer-business.md` exists with the shipped
  content), when a user appends a sentinel line `# my local edit`
  to that file and then runs `speccy init --force`, then the file
  still contains the sentinel line after the run.
- Given the same tempdir, when the user deletes
  `.claude/agents/reviewer-business.md` entirely and then runs
  `speccy init --force`, then the file is recreated with the
  shipped persona body.
- Given a `speccy init --force` run against an initialized
  workspace, when its stdout plan summary is scanned, then lines
  referencing `.claude/agents/reviewer-*.md` and
  `.codex/agents/reviewer-*.toml` show the `skip` action label
  (not `overwrite`).
- Given that same plan summary, when lines referencing
  `.claude/skills/speccy-*/SKILL.md` are scanned, then those show
  the `overwrite` action label (not `skip`), confirming the
  classification change is scoped to reviewer agent files only.
</behavior>

<scenario id="CHK-002">
Given a tempdir initialized once via `speccy init` and then
modified so `.claude/agents/reviewer-business.md` ends with the
appended line `# sentinel-edit-12345`,
when `speccy init --force` runs against that tempdir,
then the file `.claude/agents/reviewer-business.md` still ends with
the line `# sentinel-edit-12345`.

Given a tempdir initialized once via `speccy init` and then
modified by deleting `.claude/agents/reviewer-business.md`,
when `speccy init --force` runs against that tempdir,
then the file `.claude/agents/reviewer-business.md` exists again
and its body contains the substring `# Reviewer Persona: Business`
(the stable first-line header from the shipped persona body).

Given the same `speccy init --force` invocation against an
initialized workspace,
when stdout is captured and parsed line-by-line for the plan
summary,
then every line whose path matches `.claude/agents/reviewer-*.md`
shows the `skip` action and every line whose path matches
`.claude/skills/speccy-*/SKILL.md` shows the `overwrite` action.

Given an analogous tempdir where the host is `codex` (init was run
with `--host codex`),
when the same edit-then-`init --force` sequence runs against
`.codex/agents/reviewer-business.toml`,
then the user edit is preserved across the re-init.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: CLI-rendered reviewer prompts contain no persona body

The six reviewer prompt templates under
`resources/modules/prompts/reviewer-<persona>.md` no longer contain
the `## Persona` section or the `{{persona_content}}` placeholder.
The `speccy review` command's render-vars map no longer inserts a
`persona_content` entry. The rendered prompt that `speccy review`
emits to stdout contains neither an unsubstituted `{{persona_content}}`
token nor an inlined copy of the persona body.

<done-when>
- Each of the six `resources/modules/prompts/reviewer-<persona>.md`
  files contains zero occurrences of the literal substring
  `{{persona_content}}` and zero occurrences of the heading
  `## Persona` (the section header that immediately preceded the
  placeholder).
- `speccy-cli/src/review.rs` no longer calls
  `speccy_core::personas::resolve_file` or any sibling resolver
  helper, and no longer inserts a `persona_content` key into the
  `vars` `BTreeMap`.
- The output of `speccy review SPEC-NNNN/T-NNN --persona business`
  against an in-review task contains no occurrence of the stable
  first-line header `# Reviewer Persona: Business` from the persona
  body (which would indicate the body got inlined).
- The output of `speccy review SPEC-NNNN/T-NNN --persona business`
  contains no occurrence of the literal `{{persona_content}}`
  (which would indicate the placeholder was retained without
  substitution).
- The test under `speccy-core/tests/prompt_render.rs` (or the
  test file that asserts reviewer-template shape) is updated to
  assert these absences.
</done-when>

<behavior>
- Given each of the six reviewer prompt template files at
  `resources/modules/prompts/reviewer-<persona>.md`, when read,
  then the body contains neither the substring `{{persona_content}}`
  nor the line `## Persona`.
- Given the `speccy review` command invoked on an in-review task
  with `--persona business`, when its stdout output is captured,
  then the output contains neither the substring
  `{{persona_content}}` nor the substring `# Reviewer Persona:
  Business`.
- Given a code search across `speccy-cli/src/` for the identifier
  `persona_content` (as a `BTreeMap` insert key or a string
  literal), when executed, then zero call-sites remain.
</behavior>

<scenario id="CHK-003">
Given each of the six files
`resources/modules/prompts/reviewer-business.md`,
`resources/modules/prompts/reviewer-tests.md`,
`resources/modules/prompts/reviewer-security.md`,
`resources/modules/prompts/reviewer-style.md`,
`resources/modules/prompts/reviewer-architecture.md`, and
`resources/modules/prompts/reviewer-docs.md`,
when each file's body is read,
then neither the literal substring `{{persona_content}}` nor the
literal line `## Persona` appears in any of the six files.

Given a workspace with at least one task in `state="in-review"`,
when `speccy review <task-ref> --persona business` runs and its
stdout is captured,
then the captured stdout contains neither `{{persona_content}}`
nor the persona-body first-line `# Reviewer Persona: Business`.

Given `speccy-cli/src/review.rs` and any sibling files in
`speccy-cli/src/`,
when grepped for the identifier `persona_content`,
then no match is found.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Persona-override resolver and its dependents are removed

The persona-override resolver chain in `speccy_core::personas` is
deleted: `resolve_file`, `resolve_file_with_warn`, `PersonaError`,
the `persona_file_name` helper, and the `PERSONAS` static all go
away. `personas::ALL` (the persona name registry consumed by
`speccy review --persona` validation and by the speccy-review skill
fan-out) is the only surface of the module that remains.
`speccy-cli/src/review.rs` drops its imports of the removed types
and removes the `ReviewError::Persona` variant. The
`speccy-core/tests/personas.rs` test file is rewritten to cover
only the registry (ALL contents, ordering, prefix-of-default
invariant); the eight tests that exercised the override chain are
deleted.

<done-when>
- `speccy-core/src/personas.rs` contains: the `ALL` constant, any
  tests that exercise `ALL` directly, and module-level doc
  comments. It does NOT contain `resolve_file`,
  `resolve_file_with_warn`, `PersonaError`, `persona_file_name`,
  or `PERSONAS`.
- `speccy-cli/src/review.rs` no longer imports `PersonaError`,
  `resolve_file as resolve_persona_file`, or the speccy-core-side
  `PERSONAS` static. The `ReviewError::Persona` variant is gone;
  the `ReviewError` enum is correspondingly smaller.
- `speccy-core/tests/personas.rs` retains tests for
  `registry_contains_six_personas_in_declared_order`,
  `registry_default_personas_is_first_four_prefix`, and
  `registry_personas_are_unique`. The remaining tests
  (`resolve_local_first_returns_override_content`,
  `resolve_local_first_returns_embedded_when_override_missing`,
  `resolve_empty_override_falls_through_with_warning`,
  `resolve_empty_override_falls_through_silently_when_no_override_present`,
  `resolve_unknown_name_returns_unknown_name_error`,
  `t002_resolve_reviewer_security_returns_shipped_body_with_pre_move_first_line`,
  `resolve_does_not_check_host_native_locations`) are deleted.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` passes; no `dead_code` or `unused_imports` lints are
  triggered by the removals.
- `cargo test --workspace` passes.
</done-when>

<behavior>
- Given `speccy-core/src/personas.rs` after the SPEC lands, when
  grepped for `resolve_file`, `PersonaError`, or `PERSONAS`, then
  zero matches are found.
- Given `speccy-cli/src/review.rs` after the SPEC lands, when
  grepped for `persona_content`, `resolve_persona_file`,
  `PersonaError`, or `personas::PERSONAS`, then zero matches are
  found.
- Given the project after the SPEC lands, when `cargo test
  --workspace` runs, then it exits 0.
- Given the project after the SPEC lands, when `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` runs,
  then it exits 0.
</behavior>

<scenario id="CHK-004">
Given the source file `speccy-core/src/personas.rs` after the SPEC
lands,
when scanned for the identifiers `resolve_file`,
`resolve_file_with_warn`, `PersonaError`, `persona_file_name`, and
`PERSONAS`,
then none of these identifiers appears as a `fn`, `struct`, `enum`,
or `static` declaration in the file.

Given the source file `speccy-cli/src/review.rs` after the SPEC
lands,
when scanned for the identifiers `persona_content`,
`resolve_persona_file`, `PersonaError`, and `speccy_core::personas::PERSONAS`,
when each is checked,
then none appears in the file.

Given the test file `speccy-core/tests/personas.rs` after the SPEC
lands,
when its test functions are enumerated,
then the surviving functions are exactly the three registry-only
tests (`registry_contains_six_personas_in_declared_order`,
`registry_default_personas_is_first_four_prefix`,
`registry_personas_are_unique`) and none of the seven
resolver-chain tests survive.

Given a clean checkout of the workspace after the SPEC lands,
when `cargo test --workspace` and `cargo clippy --workspace
--all-targets --all-features -- -D warnings` both run,
then both exit with status 0.
</scenario>

</requirement>

## Design

### Approach

This SPEC lands in five mechanical steps. The Rust code touched is
limited to three files (`speccy-cli/src/init.rs`,
`speccy-cli/src/review.rs`, `speccy-core/src/personas.rs`); the
content edits touch six prompt templates and remove one in-tree
directory.

1. **Edit the six reviewer prompt templates.** For each of
   `resources/modules/prompts/reviewer-business.md`,
   `reviewer-tests.md`, `reviewer-security.md`, `reviewer-style.md`,
   `reviewer-architecture.md`, and `reviewer-docs.md`: delete the
   `## Persona` section header and the `{{persona_content}}`
   placeholder line that follows it. Leave the surrounding sections
   (`## SPEC (pointer)`, `## Task entry (verbatim from TASKS.md)`,
   `## Diff under review`, `## Your task`) byte-identical.

2. **Edit `speccy-cli/src/review.rs`.** Remove the `persona_content`
   key insertion from the `vars` `BTreeMap` at line 129. Remove
   the call to `resolve_persona_file` at line 112. Remove the
   imports of `PersonaError`, `resolve_file as resolve_persona_file`
   at lines 25-26. Remove the `ReviewError::Persona` variant from
   the error enum at lines 58-60. The persona-name validation
   against `PERSONAS_ALL` at lines 101-106 remains; it now returns
   a refactored error variant (e.g., `ReviewError::UnknownPersona { name }`)
   that does not re-export the deleted `PersonaError` type.

3. **Edit `speccy-cli/src/init.rs`.** Remove the imports of
   `speccy_core::personas::PERSONAS` and
   `speccy_core::prompt::PROMPTS` at lines 18-19. Remove the two
   `append_user_tunable_dir_items` calls at lines 199-203. Remove
   the `append_user_tunable_dir_items` helper function (lines
   245-266) and the `collect_bundle_files` helper (lines 268-300)
   that supports only it. Extend `append_host_pack_items` (lines
   219-235) to classify each rendered file's destination path:
   if the `rel_path` matches `.claude/agents/reviewer-<persona>.md`
   or `.codex/agents/reviewer-<persona>.toml` for any `<persona>`
   in `speccy_core::personas::ALL`, classify Skip-on-exists rather
   than Overwrite-on-exists. Keep `Action::Skip`; it now serves
   this new purpose.

4. **Edit `speccy-core/src/personas.rs`.** Delete the resolver
   chain: `PERSONAS` static, `PersonaError` enum, `resolve_file`
   function, `resolve_file_with_warn` function, `persona_file_name`
   helper. Keep `ALL` and its surrounding doc comments. Update the
   module-level doc comment to remove references to the
   project-local override and resolution chain. Delete the seven
   resolver-chain tests in `speccy-core/tests/personas.rs`; keep
   the three registry tests.

5. **Remove `.speccy/skills/` from this workspace.** `git rm -rf
   .speccy/skills/`. Run `cargo run -- init --force --host
   claude-code` and `cargo run -- init --force --host codex` to
   confirm the directory is not recreated. Run the full hygiene
   gate (`cargo test --workspace`, `cargo clippy --workspace
   --all-targets --all-features -- -D warnings`, `cargo +nightly
   fmt --all --check`, `cargo deny check`).

#### Init-plan classification matrix

The behavioural shift in `init.rs` is small but worth listing
explicitly so reviewers can confirm the scope. The table shows
classification for each kind of file in the init plan, before and
after the SPEC.

| File category | Today | After SPEC |
|---|---|---|
| `.speccy/speccy.toml` | Create / Overwrite | (unchanged) |
| `.claude/skills/speccy-*/SKILL.md` | Create / Overwrite | (unchanged) |
| `.agents/skills/speccy-*/SKILL.md` | Create / Overwrite | (unchanged) |
| `.claude/agents/reviewer-*.md` | Create / **Overwrite** | Create / **Skip** |
| `.codex/agents/reviewer-*.toml` | Create / **Overwrite** | Create / **Skip** |
| `.speccy/skills/personas/reviewer-*.md` | Create / **Skip** | **Not written at all** |
| `.speccy/skills/prompts/<name>.md` | Create / **Skip** | **Not written at all** |

Only the bottom four rows change. The `.speccy/skills/` rows
disappear from the plan entirely (no `PlanItem` is appended for
them). The two `.<host>/agents/reviewer-*` rows flip from
Overwrite-on-exists to Skip-on-exists; their Create-on-absent
behaviour is unchanged.

#### Why no two-tier (frontmatter / body) split

Host-native reviewer files mix machine-readable frontmatter (`name`,
`description` fields the host router consumes) with user-tunable
body prose (the persona instructions). A naive Skip-on-exists rule
treats the whole file as user-tunable, which means a careless edit
to the frontmatter can break sub-agent loading. Two-tier handling
(re-render frontmatter, preserve body) would prevent that, at the
cost of a file-surgery step in `init`'s render pipeline.

DEC-002 below resolves this trade-off in favour of whole-file
Skip-on-exists. The frontmatter is small (two fields, both
documented in the host's own docs) and the recovery path (delete
the file, re-init) is already familiar from every other host-pack
file. Adding file-surgery to the renderer is more machinery than
the failure mode justifies for v1.

### Decisions

<decision id="DEC-001" status="accepted">
#### DEC-001: Drop `{{persona_content}}` from CLI-rendered prompts; trust host sub-agent definition as the sole persona-delivery channel

**Status:** Accepted

**Context:** The CLI's reviewer prompt currently inlines the
persona body via `{{persona_content}}`, on the premise that the
rendered prompt should be self-contained — readable as-is by any
reviewer agent without depending on out-of-band state. But the
sub-agent that runs `speccy review --persona <p>` is, in practice,
always spawned through a host primitive (Claude Code's `Task` tool
with `subagent_type: "reviewer-<p>"`, or Codex's named-agent
resolution) that pre-loads `.claude/agents/reviewer-<p>.md` or
`.codex/agents/reviewer-<p>.toml` as the sub-agent's system
context. The persona body reaches the sub-agent through that
channel regardless of what the CLI prompt carries.

**Decision:** Drop `{{persona_content}}` and the `## Persona`
section entirely from the rendered prompt. The rendered prompt
becomes the task-specific delta only: which task, which SPEC, which
diff to fetch, what deliverable. The persona body lives in exactly
one location: the host-native sub-agent definition file.

**Alternatives:**

- *Keep `{{persona_content}}` and wire the resolver chain
  symmetrically into the host-pack render at init time*, so both
  paths consult the same `.speccy/skills/personas/` override
  source. Rejected: doubles the maintenance surface, keeps the
  half-wired feel, and `.speccy/skills/prompts/` would still need
  separate handling. The simpler move is to delete the override
  hook entirely.
- *Drop the `## Persona` block but replace it with a one-line
  pointer (`Your persona definition is loaded as your system
  context — read it there`)*. Rejected as noise; the sub-agent
  has the context loaded whether or not the prompt mentions it,
  and the pointer would be the only line in the rendered prompt
  whose meaning depended on the spawn channel.
- *Keep both paths but document the inconsistency in
  `ARCHITECTURE.md`*. Rejected: documenting a foot-gun is not a
  fix; either the override works for both paths or neither path
  should depend on it.

**Consequences:** The rendered prompt is no longer fully
self-contained when invoked outside a sub-agent spawn (e.g., a user
running `speccy review <task> --persona business` manually at the
terminal for inspection). Such a user reads the persona body
separately at `.claude/agents/reviewer-business.md`. This is
consistent with SPEC-0023's direction for SPEC.md and AGENTS.md
(the rendered prompt names the path; the agent reads the file via
the host's Read primitive). For sub-agent invocations (the normal
case), nothing observable changes: the sub-agent still receives the
persona body, just from one source instead of two.
</decision>

<decision id="DEC-002" status="accepted">
#### DEC-002: Whole-file Skip-on-exists for host-native reviewer files; no frontmatter / body split

**Status:** Accepted

**Context:** `.claude/agents/reviewer-*.md` and
`.codex/agents/reviewer-*.toml` carry both host-machinery
frontmatter (`name`, `description`) and user-tunable persona body
prose. A two-tier classification (re-render frontmatter, preserve
body) would protect users from clobbering host-side fields.

**Decision:** Treat the whole file as user-tunable. Skip on exists
under `--force`; Create on absent. If a user damages the
frontmatter, they delete the file and re-init — the same recovery
path used for any other host-pack file.

**Alternatives:**

- *Re-render frontmatter on `--force` while preserving body.*
  Rejected: requires file-surgery in the renderer (parse YAML/TOML,
  swap fields, re-serialize), and the host-machinery fields are
  small and stable. The complexity ceiling on the renderer is more
  expensive than the foot-gun.
- *Move persona bodies into a sidecar file referenced from the
  host-native definition.* Rejected: Claude Code and Codex do not
  support sidecar references in agent definitions; the persona body
  must be inlined into the host-native file for the host to load
  it.

**Consequences:** A user who edits the frontmatter accidentally
will get a host-router miss (skill description mismatch) or a
sub-agent load failure. Recovery is one shell command (`rm
.claude/agents/reviewer-<persona>.md && speccy init --force`). The
documented v1 surface is small enough that this trade-off is
acceptable.
</decision>

<decision id="DEC-003" status="accepted">
#### DEC-003: Leave pre-existing `.speccy/skills/` directories alone; do not auto-delete

**Status:** Accepted

**Context:** Users who initialized their workspace before this SPEC
have `.speccy/skills/personas/` and `.speccy/skills/prompts/`
directories with files in them. After the SPEC, those directories
become orphaned (nothing reads them). The init plan could
proactively delete them on `--force`, or leave them alone.

**Decision:** Leave them alone. `speccy init` should not silently
delete files outside its current write plan. The orphan is
harmless; users who want to clean up run `rm -rf .speccy/skills/`
themselves.

**Alternatives:**

- *Delete `.speccy/skills/` on `init --force` post-SPEC.* Rejected:
  destructive operations belong behind an explicit user action
  (a separate command or a `--clean` flag), not as a side-effect
  of `init`. The principle "init should not silently destroy
  files" outweighs the minor cleanliness gain.
- *Print a stderr warning on `init` when `.speccy/skills/` is
  detected, recommending manual deletion.* Considered for cosmetic
  guidance but ultimately rejected to keep `init`'s output stable;
  this SPEC's `## Migration / rollback` section provides the same
  guidance for users who notice.

**Consequences:** Some workspaces will carry orphan
`.speccy/skills/` directories indefinitely until the user notices.
Acceptable trade-off; no functional impact.
</decision>

<decision id="DEC-004" status="accepted">
#### DEC-004: No `.speccy/overrides/` rename; eliminate the override mechanism entirely

**Status:** Accepted

**Context:** During the brainstorm conversation that produced this
SPEC, one option was to rename `.speccy/skills/` to
`.speccy/overrides/` to clarify intent and resolve the
naming-collision with `.claude/skills/`. The override mechanism
itself (project-local persona file → embedded bundle fallback)
would have been retained, just renamed.

**Decision:** No rename. The override mechanism is eliminated; the
directory does not survive in any form.

**Alternatives:**

- *Rename to `.speccy/overrides/personas/` and keep the resolver
  chain.* Rejected: half-wired overrides under a clearer name are
  still half-wired. The double-inlining problem is solved by
  removing one of the two inlines, not by renaming the half that
  partially worked.
- *Promote the override mechanism to fully wired (both Jinja
  include at init render time and CLI resolver chain consult the
  same project-local source).* Rejected: more machinery for a
  customization surface that has no user demand. v1 is "useful for
  my next greenfield" (per `AGENTS.md`); persona-body customization
  has not surfaced as a need in any of the SPECs that have shipped
  to date. If it surfaces later, the host-native files are already
  the right place to edit; no new override machinery is needed.

**Consequences:** Users who want different persona content per
project edit `.claude/agents/reviewer-<persona>.md` (or the Codex
equivalent) directly. The "single source per content stream"
property holds for personas as it now holds for AGENTS.md and
SPEC.md (after SPEC-0023).
</decision>

<decision id="DEC-005" status="accepted">
#### DEC-005: `Action::Skip` is retained and repurposed; no new classification enum variant

**Status:** Accepted

**Context:** `Action::Skip` exists today to mark
`.speccy/skills/personas/` and `.speccy/skills/prompts/` as
"don't overwrite under `--force`". After the SPEC, those paths are
removed from the plan entirely, but Skip-on-exists semantics is
still needed for the host-native reviewer files. The clean choice
is whether to retain `Action::Skip` or rename it (e.g.,
`Action::SkipIfExists`).

**Decision:** Retain `Action::Skip` as-is; its meaning shifts from
"persona/prompt user-tunable bodies" to "host-native reviewer
files." The variant label and behaviour are unchanged.

**Alternatives:**

- *Rename to `Action::SkipIfExists` for clarity.* Considered;
  rejected because the existing label is already accurate (a file
  classified Skip is skipped, full stop), and the rename would add
  diff noise to every Action match arm.
- *Inline the Skip semantics directly into `execute_plan`'s match
  arms without an enum variant.* Rejected: the plan-print summary
  needs a label (`skip`) for the affected paths; keeping the
  variant keeps the summary logic uniform.

**Consequences:** Future PRs that look at `Action::Skip` see it
guarding host-native reviewer files instead of `.speccy/skills/`.
The variant doc comment is updated to reflect the new use.
</decision>

### Interfaces

Files edited (Rust source):
- `speccy-cli/src/init.rs` — drop two `append_user_tunable_dir_items`
  calls and the helper itself; drop the `collect_bundle_files`
  helper; drop imports of `PERSONAS` and `PROMPTS`; extend
  `append_host_pack_items` (or `classify`) to recognize the two
  reviewer agent path patterns as Skip-on-exists.
- `speccy-cli/src/review.rs` — drop persona-content resolver call,
  the `persona_content` map entry, and the `ReviewError::Persona`
  variant.
- `speccy-core/src/personas.rs` — delete the resolver chain;
  retain only `ALL` and its registry tests.

Files edited (content):
- `resources/modules/prompts/reviewer-business.md`
- `resources/modules/prompts/reviewer-tests.md`
- `resources/modules/prompts/reviewer-security.md`
- `resources/modules/prompts/reviewer-style.md`
- `resources/modules/prompts/reviewer-architecture.md`
- `resources/modules/prompts/reviewer-docs.md`

Files deleted (in-tree dogfood):
- `.speccy/skills/personas/reviewer-business.md`
- `.speccy/skills/personas/reviewer-tests.md`
- `.speccy/skills/personas/reviewer-security.md`
- `.speccy/skills/personas/reviewer-style.md`
- `.speccy/skills/personas/reviewer-architecture.md`
- `.speccy/skills/personas/reviewer-docs.md`
- `.speccy/skills/prompts/implementer.md`
- `.speccy/skills/prompts/plan-amend.md`
- `.speccy/skills/prompts/plan-greenfield.md`
- `.speccy/skills/prompts/report.md`
- `.speccy/skills/prompts/reviewer-architecture.md`
- `.speccy/skills/prompts/reviewer-business.md`
- `.speccy/skills/prompts/reviewer-docs.md`
- `.speccy/skills/prompts/reviewer-security.md`
- `.speccy/skills/prompts/reviewer-style.md`
- `.speccy/skills/prompts/reviewer-tests.md`
- `.speccy/skills/prompts/tasks-amend.md`
- `.speccy/skills/prompts/tasks-generate.md`

Tests edited:
- `speccy-core/tests/personas.rs` — delete seven resolver-chain
  tests; keep three registry tests.
- `speccy-cli/tests/init.rs` — add tests asserting `.speccy/skills/`
  is not created, that user edits to `.claude/agents/reviewer-*.md`
  survive `--force`, and that deleting then re-initing recreates
  the file.
- Any existing `init.rs` tests that asserted the presence of
  `.speccy/skills/` paths in the plan are updated to assert their
  absence.

Bundles and renderer unchanged:
- `resources/agents/.claude/agents/reviewer-*.md.tmpl` (Jinja
  templates) are unchanged.
- `resources/agents/.codex/agents/reviewer-*.toml.tmpl` are
  unchanged.
- `resources/modules/personas/reviewer-*.md` (embedded persona
  bodies) are unchanged.
- `speccy-cli/src/render.rs` (host-pack render pipeline) is
  unchanged.
- `speccy-core/src/prompt/template.rs` and its `PROMPTS` static are
  unchanged.

### Data changes

None. No artifact grammar, schema, frontmatter field, or `.speccy/`
layout change beyond the removal of `.speccy/skills/`.

### Migration / rollback

**Forward.** For users on a workspace initialized pre-SPEC who
upgrade to a post-SPEC speccy CLI: pull the new CLI, run `speccy
init --force` once. The `.speccy/skills/` directory becomes an
orphan with no consumers. Users who want to clean it up run `rm
-rf .speccy/skills/` once; users who don't notice it carry the
orphan harmlessly. Persona-body edits the user made under
`.speccy/skills/personas/` are not auto-migrated to
`.claude/agents/reviewer-*.md` — if a user had a customized
override there, they manually copy the desired body into
`.claude/agents/reviewer-<persona>.md` (between the YAML
frontmatter and EOF). A note in this SPEC's `## Open Questions`
captures whether to ship a one-off migration helper; the default
answer is no for v1.

**Rollback.** Revert the SPEC commit. The renderer pipeline is
unchanged, so reverting restores the prior init plan exactly. Any
user edits that were preserved by Skip-on-exists semantics under
the post-SPEC CLI will be overwritten on the next `--force` run
under the reverted CLI — a strict regression. This is a one-way
door for users who relied on the new behaviour, but rollback is
expected to be rare; the change is small and well-tested.

## Open Questions

- [ ] Should we ship a one-off migration helper (`speccy migrate
      personas` or similar) that copies user-edited
      `.speccy/skills/personas/reviewer-*.md` bodies into
      `.claude/agents/reviewer-*.md` (preserving the host
      frontmatter)? Default answer for v1: no. The CLI gains a new
      command surface for a one-off migration that benefits only
      users who customized the half-wired override. If telemetry
      or dogfooding shows non-zero adoption of the override, this
      reopens.

## Assumptions

<assumptions>
- Claude Code's sub-agent mechanism continues to load
  `.claude/agents/<name>.md` (frontmatter + body) as the sub-agent's
  system context when the parent agent invokes the `Task` tool
  with `subagent_type: "<name>"`. Documented behaviour as of
  2026-05-17; the speccy-review skill relies on this today.
- Codex's named-agent resolution continues to load
  `.codex/agents/<name>.toml`'s `developer_instructions` field as
  the sub-agent's system context when the parent agent spawns a
  sub-agent by name. Verified against the `.codex/agents/` shape
  documented in OpenAI's Codex sub-agents reference; the
  speccy-review skill relies on this today.
- No existing workspace contains hand-edited
  `.speccy/skills/personas/reviewer-*.md` files whose edits would
  silently disappear from review behaviour post-SPEC. The dogfood
  workspace (this repo) has only shipped-default content there;
  external user impact is bounded by Speccy's pre-v1 adoption
  (which is currently zero external users beyond the author per
  `AGENTS.md`'s "useful for my next greenfield" scope).
- The host's persona-body load mechanism delivers the body once to
  the sub-agent (as system context) and not also as the task
  message. If a future host change started double-injecting the
  body, that would re-introduce the duplication this SPEC
  eliminates from the CLI side; the host-side fix would be a
  separate concern.
- `Action::Skip` semantics in the init plan are correct for the
  host-native reviewer file case: if the file exists, do nothing;
  if it does not exist, fall through to Create. (Confirmed by
  reading `execute_plan` at lines 340-358 of `init.rs`.)
- The MiniJinja renderer's `{% include "modules/personas/<file>" %}`
  expansion continues to source from the embedded
  `resources/modules/personas/` bundle (via the loader rooted in
  `speccy-cli/src/render.rs`), unaffected by the deletion of the
  speccy-core-side `PERSONAS` static (which was a separate
  embedded copy used only by the now-deleted resolver).
</assumptions>

## Changelog

<changelog>
| Date       | Author      | Summary |
|------------|-------------|---------|
| 2026-05-17 | human/kevin | Initial draft. Persona body is currently inlined twice: once into host-native `.claude/agents/reviewer-*.md` and `.codex/agents/reviewer-*.toml` at init time via Jinja `{% include %}`, and once into the CLI-rendered reviewer prompt via `{{persona_content}}` filled by `personas::resolve_file`. Only the CLI-rendered path honors `.speccy/skills/personas/`, making the override half-effective and confusing. `.speccy/skills/prompts/` is dead weight (nothing reads from it). This SPEC consolidates persona delivery on the host-native file as the single source, classifies it as Skip-on-exists under `--force`, drops `{{persona_content}}` from the rendered prompt, deletes the resolver chain and its tests, and removes `.speccy/skills/` from the init plan and from this workspace. Mirrors SPEC-0023's direction (REQ-005 / REQ-006 stopped inlining AGENTS.md and SPEC.md). |
</changelog>

## Notes

The brainstorm that produced this SPEC walked through three
options for resolving the half-wired override:

1. **Eject persona body to host-native files only, drop the CLI
   inlining** — the path chosen here.
2. **Promote `.speccy/skills/personas/` to a fully-wired
   canonical source, render host-native files from it on init.**
   Rejected: makes `speccy init --force` a build step that users
   must re-run after every persona edit, and introduces a
   "canonical source vs rendered artifact" distinction inside
   `.speccy/` that didn't exist before.
3. **Wire the host-pack render pipeline to also consult
   `.speccy/skills/personas/` as part of the Jinja include
   resolution.** Rejected: doubles renderer surface, keeps the
   override directory's confusing-but-now-symmetric shape, and
   `.speccy/skills/prompts/` would still be unaddressed.

Option 1 was chosen because it deletes the most code, eliminates
the most asymmetry, and aligns with SPEC-0023's already-shipped
direction of "name paths, don't inline content."

The user-customization story under this SPEC is: edit
`.claude/agents/reviewer-<persona>.md` (or `.codex/agents/reviewer-<persona>.toml`)
directly. Re-running `speccy init --force` preserves the edit. To
discard a customization and return to the shipped default, delete
the file and re-init. This is the same shape as how SPEC.md,
TASKS.md, and AGENTS.md are user-owned files that `init` does not
touch after first creation — the SPEC is making host-native
reviewer files behave the same way.
