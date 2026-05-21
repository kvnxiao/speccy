---
spec: SPEC-0027
spec_hash_at_generation: 013358c629917e7fd0a9ab45043491f0fa9502441206d17ea8497f993f34c4d4
generated_at: 2026-05-20T18:49:25Z
---

# Tasks: SPEC-0027 Host-native files are the sole canonical persona surface; drop .speccy/skills/ override directory


## Phase 1: Drop the persona body from the CLI-rendered reviewer prompt

<task id="T-001" state="completed" covers="REQ-003">
## T-001: Strip `{{persona_content}}` from reviewer prompt templates and `speccy review`'s render-vars map

Drop the `## Persona` section header and the
`{{persona_content}}` placeholder line from each of the six
reviewer prompt templates under
`resources/modules/prompts/reviewer-<persona>.md` (one per persona:
business, tests, security, style, architecture, docs). Leave the
surrounding sections (`## SPEC (pointer)`, `## Task entry (verbatim
from TASKS.md)`, `## Diff under review`, `## Your task`)
byte-identical so reviewers can confirm the edit is scoped to the
deleted block only.

In the same task, edit `speccy-cli/src/review.rs` so the
rendered-prompt path no longer carries persona body text:

- Drop the `persona_content` key insertion from the `vars`
  `BTreeMap`.
- Drop the call site that resolved the persona body
  (`resolve_persona_file` / `resolve_file`).
- Drop the now-unused imports of `PersonaError` and
  `resolve_file as resolve_persona_file` from `speccy_core::personas`.
- Drop the `ReviewError::Persona` enum variant since no call site
  produces it any more; the surviving persona-name validation
  against `personas::ALL` (today wired to the `PERSONAS_ALL` alias)
  returns a refactored variant such as
  `ReviewError::UnknownPersona { name }` whose payload no longer
  re-exports the deleted `PersonaError` type.

The two edits land together because they are coupled: editing the
templates alone leaves a `{{persona_content}}` variable insertion
in `vars` with no consumer (harmless but stale); editing `review.rs`
alone leaves an unsubstituted `{{persona_content}}` token in the
rendered output. Landing both in one task keeps the workspace green
between commits.

Tests that pin reviewer-prompt shape (under
`speccy-core/tests/prompt_render.rs` or wherever
`{{persona_content}}` / `## Persona` substring assertions live)
flip from "must contain" to "must not contain"; tests that assert
persona-body lines (e.g., the stable `# Reviewer Persona: Business`
first line) survive the rendered prompt are removed or rewritten to
assert absence.

`speccy_core::personas::resolve_file` and its sibling resolver
helpers are NOT deleted in this task — they become unused outside
their own test module, but the deletion belongs to T-004 to keep
the `personas` module surgery isolated. Until then, the resolver
chain survives as dead-but-public API; `cargo clippy --workspace
--all-targets --all-features -- -D warnings` does not flag `pub`
items as `dead_code`, so the workspace stays green between T-001
and T-004.

- Suggested files:
  - `resources/modules/prompts/reviewer-business.md`
  - `resources/modules/prompts/reviewer-tests.md`
  - `resources/modules/prompts/reviewer-security.md`
  - `resources/modules/prompts/reviewer-style.md`
  - `resources/modules/prompts/reviewer-architecture.md`
  - `resources/modules/prompts/reviewer-docs.md`
  - `speccy-cli/src/review.rs`
  - `speccy-core/tests/prompt_render.rs`

<task-scenarios>
  - Given the six reviewer prompt template files at
    `resources/modules/prompts/reviewer-<persona>.md` after this
    task lands, when each file's body is read, then neither the
    literal substring `{{persona_content}}` nor the literal line
    `## Persona` appears in any of the six files.
  - Given the surrounding sections of each of the six template
    files (`## SPEC (pointer)`, `## Task entry (verbatim from
    TASKS.md)`, `## Diff under review`, `## Your task`), when each
    section is compared against its pre-task content, then the
    surrounding sections are byte-identical (the diff is scoped to
    the deleted persona block only).
  - Given a workspace with at least one task in `state="in-review"`,
    when `speccy review <task-ref> --persona business` runs and its
    stdout is captured, then the captured stdout contains neither
    the literal `{{persona_content}}` substring (which would mean
    the placeholder was kept without substitution) nor the literal
    line `# Reviewer Persona: Business` (which would mean the
    persona body was inlined despite the template edit).
  - Given `speccy-cli/src/review.rs` after this task lands, when
    grepped for the identifiers `persona_content`,
    `resolve_persona_file`, and `PersonaError`, then zero matches
    are found in any source line of the file.
  - Given `speccy-cli/src/review.rs` after this task lands, when
    the `ReviewError` enum's variants are enumerated, then no
    variant named `Persona` exists; the surviving persona-name
    validation against `personas::ALL` produces a refactored
    variant (e.g., `UnknownPersona { name }`) instead.
  - Given the test file under `speccy-core/tests/prompt_render.rs`
    (or wherever reviewer-template shape is asserted) after this
    task lands, when each `assert!` / `assert_eq!` mentioning the
    `{{persona_content}}` placeholder or the `## Persona` heading
    is examined, then the assertion expresses "must NOT contain"
    rather than "must contain".
  - Given `cargo test --workspace` after this task lands, when run,
    then the exit code is 0 — all tests pass against the smaller
    rendered-prompt shape.
</task-scenarios>
</task>

## Phase 2: Reshape `speccy init`'s plan around host-native reviewer files

<task id="T-002" state="completed" covers="REQ-002">
## T-002: Classify `.claude/agents/reviewer-*.md` and `.codex/agents/reviewer-*.toml` as Skip-on-exists

Extend the init-plan classifier (the function that decides
`Action::Create` / `Action::Overwrite` / `Action::Skip` for each
rendered host-pack file in `speccy-cli/src/init.rs`) so that when a
rendered file's `rel_path` matches
`.claude/agents/reviewer-<persona>.md` or
`.codex/agents/reviewer-<persona>.toml` for any `<persona>` in
`speccy_core::personas::ALL`, the classification is `Action::Skip`
when the destination exists and `Action::Create` when it does not.
All other host-pack files (skill wrappers under `.claude/skills/`,
`.agents/skills/`, etc., plus the `.speccy/speccy.toml`
configuration write) retain today's Create-or-Overwrite
classification.

The change is scoped to the path-matching predicate inside the
classifier. `Action::Skip` itself is unchanged: per DEC-005, the
variant label and `execute_plan` match arm are reused as-is so
plan-print summaries continue to render the `skip` label for the
affected paths. Update the variant's doc comment to reflect that
it now guards host-native reviewer files rather than the (now
removed in T-003) `.speccy/skills/` user-tunable directories.

Add tests under `speccy-cli/tests/init.rs` that exercise the new
classification end-to-end:

- Sentinel preservation: initialize a tempdir once via `speccy
  init`, append a sentinel line to
  `.claude/agents/reviewer-business.md`, run `speccy init --force`,
  then assert the sentinel line is still present.
- Deletion + recreation: initialize a tempdir once, delete
  `.claude/agents/reviewer-business.md` entirely, run `speccy init
  --force`, then assert the file exists again with the shipped
  persona body (e.g., the stable first-line header
  `# Reviewer Persona: Business`).
- Plan-summary labels: capture stdout of `speccy init --force`
  against an initialized workspace and assert lines for
  `.claude/agents/reviewer-*.md` show `skip` while lines for
  `.claude/skills/speccy-*/SKILL.md` continue to show `overwrite`,
  confirming the classification flip is scoped to reviewer agent
  files only.
- Codex twin: repeat the sentinel-preservation scenario for a
  workspace initialized with `--host codex` and the file path
  `.codex/agents/reviewer-business.toml`.

This task does not touch `.speccy/skills/` plan items; T-003 owns
that removal. Until T-003 lands, the init plan still appends the
`.speccy/skills/personas/` and `.speccy/skills/prompts/` entries
(both already classified `Action::Skip` from prior work); the new
Skip-on-exists classification this task adds is for the
`.<host>/agents/reviewer-*` paths only.

- Suggested files:
  - `speccy-cli/src/init.rs`
  - `speccy-cli/tests/init.rs`

<task-scenarios>
  - Given a tempdir already initialized via `speccy init` (so
    `.claude/agents/reviewer-business.md` exists with the shipped
    content), when a sentinel line `# sentinel-edit-12345` is
    appended to that file and then `speccy init --force` runs
    against the same tempdir, then the file still ends with the
    line `# sentinel-edit-12345` after the run.
  - Given a tempdir already initialized via `speccy init`, when
    `.claude/agents/reviewer-business.md` is deleted entirely and
    then `speccy init --force` runs, then the file exists again
    and its body contains the substring
    `# Reviewer Persona: Business` (the stable first-line header
    from the shipped persona body).
  - Given a `speccy init --force` run against an initialized
    workspace, when stdout is captured and parsed line-by-line
    for the plan summary, then every line whose path matches
    `.claude/agents/reviewer-*.md` shows the `skip` action label
    (not `overwrite`).
  - Given the same plan summary, when lines whose path matches
    `.claude/skills/speccy-*/SKILL.md` are scanned, then those
    show the `overwrite` action label (not `skip`), confirming
    the classification flip is scoped to reviewer agent files
    only.
  - Given a tempdir initialized with `speccy init --host codex`
    so `.codex/agents/reviewer-business.toml` exists, when a
    sentinel line is appended to that file and then `speccy init
    --force --host codex` runs, then the file still contains the
    appended sentinel line afterwards. Symmetrically, deleting
    the file and re-initing recreates it from the shipped Codex
    persona content.
  - Given the init-plan classifier exercised in unit tests against
    each `<persona>` in `speccy_core::personas::ALL`, when the
    classifier is invoked with a `rel_path` of
    `.claude/agents/reviewer-<persona>.md` against a tempdir where
    that file exists, then the returned `Action` is `Action::Skip`;
    when invoked against a tempdir where that file does not exist,
    then the returned `Action` is `Action::Create`.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001">
## T-003: Stop appending `.speccy/skills/` items to the init plan

Drop the code path in `speccy-cli/src/init.rs` that writes the
`.speccy/skills/personas/` and `.speccy/skills/prompts/` directory
contents into the init plan:

- Remove the two `append_user_tunable_dir_items` call sites that
  add the persona-override and prompt-override directories to the
  plan.
- Remove the `append_user_tunable_dir_items` helper function itself.
- Remove the `collect_bundle_files` helper that exists only to
  support `append_user_tunable_dir_items`.
- Remove the imports of `speccy_core::personas::PERSONAS` and
  `speccy_core::prompt::PROMPTS` from the top of `init.rs`; both
  were used only by the removed helper.

Pre-existing `.speccy/skills/` directories in user workspaces are
left alone (per DEC-003): `init` simply stops writing into that
subtree, but never deletes any file there. The in-tree workspace's
own `.speccy/skills/` directory is removed in T-005 as the dogfood
proof step; this task is the CLI-side change that makes the removal
stick.

Add tests under `speccy-cli/tests/init.rs` that exercise the
removal end-to-end:

- Fresh-init absence: against an empty tempdir, `speccy init` runs
  and afterwards `tmpdir.join(".speccy/skills")` does not exist.
- Plan-output silence: the captured stdout of `speccy init` against
  an empty tempdir contains no occurrence of the literal substring
  `.speccy/skills/`.
- Pre-existing file preservation: a tempdir is pre-populated with
  `.speccy/skills/personas/reviewer-business.md` containing
  arbitrary bytes (e.g., `pre-existing override\n`); after `speccy
  init --force` runs, the file still exists with byte-for-byte
  identical content.

Update any existing init-test that asserted presence of
`.speccy/skills/` paths in the plan to assert absence instead. The
test that confirms `.speccy/speccy.toml` is created continues to
pass since the `.speccy/` write step itself is unchanged; only the
`skills/` subtree disappears from the plan.

`speccy-core/src/personas.rs` still exports the `PERSONAS` static
after this task lands — T-004 owns that deletion. Until T-004
lands, `PERSONAS` survives as a `pub` static referenced only by
its own test module; clippy does not flag `pub` items as
`dead_code`, so the workspace stays green between T-003 and T-004.

- Suggested files:
  - `speccy-cli/src/init.rs`
  - `speccy-cli/tests/init.rs`

<task-scenarios>
  - Given a freshly created empty temporary directory used as
    `project_root`, when `speccy init` runs with that directory as
    `cwd`, then `project_root.join(".speccy").join("skills")` does
    not exist on the filesystem after the command completes.
  - Given the same fresh tempdir, when `speccy init` runs and its
    stdout is captured, then the captured output contains no
    occurrence of the literal substring `.speccy/skills/`.
  - Given a tempdir pre-populated with
    `.speccy/skills/personas/reviewer-business.md` containing the
    bytes `pre-existing override\n`, when `speccy init --force`
    runs against it, then the file still exists with byte-for-byte
    identical content (init does not delete, init does not
    rewrite).
  - Given `speccy-cli/src/init.rs` after this task lands, when
    grepped for `append_user_tunable_dir_items`,
    `collect_bundle_files`, `speccy_core::personas::PERSONAS`, and
    `speccy_core::prompt::PROMPTS`, then zero matches are found in
    any source line of the file (helpers removed, imports
    removed).
  - Given the init plan produced for an empty tempdir, when each
    `PlanItem`'s `rel_path` is read, then no item's path starts
    with `.speccy/skills/`.
  - Given `cargo test --workspace` after this task lands, when
    run, then the exit code is 0 — all tests pass against the
    slimmed init plan.
</task-scenarios>
</task>

## Phase 3: Delete the persona-override resolver chain

<task id="T-004" state="completed" covers="REQ-004">
## T-004: Delete `personas::resolve_file` and related resolver-chain surface

Edit `speccy-core/src/personas.rs` to remove the persona-override
resolver chain:

- Delete the `PERSONAS` static (the embedded copy of persona body
  bytes used only by the resolver).
- Delete the `PersonaError` enum.
- Delete the `resolve_file` function.
- Delete the `resolve_file_with_warn` function.
- Delete the `persona_file_name` helper.

Keep the `ALL` constant (and its surrounding doc comments) — that
is the only public surface of the `personas` module that survives
and is consumed by `speccy review --persona` validation (today
imported as `PERSONAS_ALL` from `speccy-cli/src/review.rs`) and by
the speccy-review skill fan-out. Update the module-level doc
comment to remove references to the project-local override
directory and the resolution chain; the module's job is now
"persona name registry" only.

Edit `speccy-core/tests/personas.rs` to match: keep the three
registry tests
(`registry_contains_six_personas_in_declared_order`,
`registry_default_personas_is_first_four_prefix`,
`registry_personas_are_unique`) and delete the seven
resolver-chain tests
(`resolve_local_first_returns_override_content`,
`resolve_local_first_returns_embedded_when_override_missing`,
`resolve_empty_override_falls_through_with_warning`,
`resolve_empty_override_falls_through_silently_when_no_override_present`,
`resolve_unknown_name_returns_unknown_name_error`,
`t002_resolve_reviewer_security_returns_shipped_body_with_pre_move_first_line`,
`resolve_does_not_check_host_native_locations`).

This task lands only after T-001 (`speccy-cli/src/review.rs` no
longer imports `resolve_file` or `PersonaError`) and T-003
(`speccy-cli/src/init.rs` no longer imports `PERSONAS`). With both
predecessor tasks landed, the items deleted here have no live
references outside their own test module (which is deleted in the
same edit).

Verify the full hygiene gate passes after the deletion: `cargo
test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo +nightly fmt --all
--check`, `cargo deny check`. No `dead_code` or `unused_imports`
lint should fire as a consequence of the removals.

- Suggested files:
  - `speccy-core/src/personas.rs`
  - `speccy-core/tests/personas.rs`

<task-scenarios>
  - Given `speccy-core/src/personas.rs` after this task lands,
    when scanned for the identifiers `resolve_file`,
    `resolve_file_with_warn`, `PersonaError`, `persona_file_name`,
    and `PERSONAS`, then none of these identifiers appears as a
    `fn`, `struct`, `enum`, or `static` declaration in the file.
  - Given the same file, when grepped for the `ALL` constant
    declaration, then it survives unchanged and exports the six
    persona names in the order
    `business, tests, security, style, architecture, docs`.
  - Given `speccy-cli/src/review.rs` after this task lands, when
    grepped for `speccy_core::personas::PERSONAS` (the embedded
    bytes static, distinct from the `ALL` registry), then zero
    matches are found.
  - Given the test file `speccy-core/tests/personas.rs` after this
    task lands, when its `#[test]` functions are enumerated, then
    the surviving functions are exactly the three registry-only
    tests (`registry_contains_six_personas_in_declared_order`,
    `registry_default_personas_is_first_four_prefix`,
    `registry_personas_are_unique`) and none of the seven
    resolver-chain tests survive.
  - Given a clean checkout of the workspace after this task
    lands, when `cargo test --workspace` and `cargo clippy
    --workspace --all-targets --all-features -- -D warnings` both
    run, then both exit with status 0.
  - Given the module-level doc comment on
    `speccy-core/src/personas.rs` after this task lands, when
    read, then it describes the module as a persona-name registry
    and contains no reference to a project-local override
    directory, a resolver chain, or an embedded `PERSONAS` bundle.
</task-scenarios>
</task>

## Phase 4: Dogfood the removal in this workspace

<task id="T-005" state="completed" covers="REQ-001">
## T-005: Remove `.speccy/skills/` from this workspace and confirm `init --force` does not recreate it

After T-001 through T-004 have landed, remove the in-tree
`.speccy/skills/` directory from this workspace as the dogfood
proof that the CLI no longer recreates it:

- `git rm -rf .speccy/skills/` (or the equivalent `git rm` walk
  over each of the 18 files listed in SPEC.md `### Interfaces`
  under "Files deleted (in-tree dogfood)").
- Run `cargo run -- init --force --host claude-code` against the
  workspace and confirm the directory is not recreated.
- Run `cargo run -- init --force --host codex` against the
  workspace and confirm the directory is not recreated.
- Run the full hygiene gate (`cargo test --workspace`, `cargo
  clippy --workspace --all-targets --all-features -- -D
  warnings`, `cargo +nightly fmt --all --check`, `cargo deny
  check`) and confirm all four checks pass.

The 18 deleted files are the six
`.speccy/skills/personas/reviewer-<persona>.md` files (one per
persona in `personas::ALL`) plus the twelve
`.speccy/skills/prompts/<name>.md` files (implementer.md,
plan-amend.md, plan-greenfield.md, report.md, reviewer-business.md,
reviewer-tests.md, reviewer-security.md, reviewer-style.md,
reviewer-architecture.md, reviewer-docs.md, tasks-amend.md,
tasks-generate.md). The exhaustive list lives in SPEC.md
`### Interfaces` under "Files deleted (in-tree dogfood)".

This task also serves as the end-to-end verification that the
classification flip from T-002 holds: re-running `init --force`
preserves the user-edited
`.claude/agents/reviewer-*.md` files committed to this workspace
(the workspace's own committed reviewer-agent files are the
"user-edited" surface from the perspective of `init`'s plan).
After this task lands, the workspace's tree contains no
`.speccy/skills/` directory and re-running `init` against it stays
that way.

If the snapshot fixture
`speccy-core/tests/fixtures/in_tree_id_snapshot.json` needs an
entry for `0027-host-native-personas` (per the convention every
prior SPEC has honored — see the T-004 procedural note in
SPEC-0026's TASKS.md), add the SPEC's REQ/CHK/DEC id sets in the
same commit so the in-tree-specs snapshot test stays green.

- Suggested files:
  - `.speccy/skills/personas/reviewer-business.md` (delete)
  - `.speccy/skills/personas/reviewer-tests.md` (delete)
  - `.speccy/skills/personas/reviewer-security.md` (delete)
  - `.speccy/skills/personas/reviewer-style.md` (delete)
  - `.speccy/skills/personas/reviewer-architecture.md` (delete)
  - `.speccy/skills/personas/reviewer-docs.md` (delete)
  - `.speccy/skills/prompts/implementer.md` (delete)
  - `.speccy/skills/prompts/plan-amend.md` (delete)
  - `.speccy/skills/prompts/plan-greenfield.md` (delete)
  - `.speccy/skills/prompts/report.md` (delete)
  - `.speccy/skills/prompts/reviewer-architecture.md` (delete)
  - `.speccy/skills/prompts/reviewer-business.md` (delete)
  - `.speccy/skills/prompts/reviewer-docs.md` (delete)
  - `.speccy/skills/prompts/reviewer-security.md` (delete)
  - `.speccy/skills/prompts/reviewer-style.md` (delete)
  - `.speccy/skills/prompts/reviewer-tests.md` (delete)
  - `.speccy/skills/prompts/tasks-amend.md` (delete)
  - `.speccy/skills/prompts/tasks-generate.md` (delete)
  - `speccy-core/tests/fixtures/in_tree_id_snapshot.json` (add
    SPEC-0027 entry if the snapshot test requires it)

<task-scenarios>
  - Given the workspace after T-001 through T-004 have landed and
    `git rm -rf .speccy/skills/` (or an equivalent per-file `git
    rm` walk) has been executed and committed, when `ls
    .speccy/skills` runs (or `Test-Path .speccy/skills` on
    Windows), then the path does not exist.
  - Given the same workspace, when `cargo run -- init --force
    --host claude-code` runs, then on completion
    `.speccy/skills/` still does not exist on the filesystem.
  - Given the same workspace, when `cargo run -- init --force
    --host codex` runs, then on completion `.speccy/skills/`
    still does not exist on the filesystem.
  - Given the same workspace, when the captured stdout of either
    `init --force` run is scanned for the literal substring
    `.speccy/skills/`, then zero matches are found.
  - Given the same workspace, when the four-tool hygiene gate
    runs (`cargo test --workspace`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, `cargo
    +nightly fmt --all --check`, `cargo deny check`), then all
    four commands exit with status 0.
  - Given the workspace's `.claude/agents/reviewer-*.md` files
    committed to this repository, when `cargo run -- init
    --force --host claude-code` runs against the workspace, then
    afterwards `git diff .claude/agents/reviewer-*.md` reports
    zero modifications (T-002's Skip-on-exists classification
    holds; the workspace's committed reviewer-agent content is
    preserved across re-init).
  - Given the snapshot fixture
    `speccy-core/tests/fixtures/in_tree_id_snapshot.json` after
    this task lands, when the in-tree-specs snapshot test runs,
    then either the fixture already covers SPEC-0027's REQ/CHK/
    DEC id sets or the test does not require a new entry; in
    either case `cargo test --workspace` exits with status 0.
</task-scenarios>
</task>

