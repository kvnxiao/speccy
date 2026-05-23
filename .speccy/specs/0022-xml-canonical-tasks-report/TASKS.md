---
spec: SPEC-0022
spec_hash_at_generation: 070fd3bc3992fe88dd6ee77bcb34824cbda5e956511f42481d82b063574891fd
generated_at: 2026-05-17T17:37:24Z
---

# Tasks: SPEC-0022 Raw XML element tags for TASKS.md and REPORT.md

## Phase 1: Shared scanner, typed models, parsers, and renderers


<task id="T-001" state="completed" covers="REQ-003">
Factor the line-aware XML element scanner into a shared module

- Suggested files: `speccy-core/src/parse/xml_scanner/mod.rs`,
  `speccy-core/src/parse/xml_scanner/html5_names.rs` (move from
  `spec_xml/`), `speccy-core/src/parse/spec_xml/mod.rs`,
  `speccy-core/src/parse/mod.rs`

<task-scenarios>
  - When the shared scanner runs on input containing an element tag
    drawn from a caller-supplied whitelist, it returns the open/close
    tag spans and verbatim body bytes; tags whose names are not in the
    whitelist are treated as Markdown text.
  - When the shared scanner runs on input whose structure-shaped tag
    lines appear inside a fenced code block (``` or ~~~), the tag is
    treated as Markdown body content and does not appear in the
    returned element list.
  - When the shared scanner encounters an unknown attribute on a
    whitelisted element, it returns a diagnostic naming the element,
    attribute, byte offset, and the set of valid attributes.
  - When `speccy-core::parse::spec_xml` is re-pointed at the shared
    scanner, every existing `spec_xml` parse and renderer test still
    passes byte-for-byte (no regression in SPEC.md parsing).
  - When the HTML5-disjointness unit test runs over the combined
    whitelist used by SPEC, TASKS, and REPORT callers, every new
    TASKS/REPORT element name (`tasks`, `task`, `task-scenarios`,
    `report`, `coverage`) is asserted absent from the checked-in
    HTML5 element name set.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-003">
TASKS.md typed model, parser, and renderer

- Suggested files: `speccy-core/src/parse/task_xml/mod.rs`,
  `speccy-core/src/parse/mod.rs`,
  `speccy-core/tests/fixtures/task_xml/canonical.md`,
  `speccy-core/tests/task_xml_roundtrip.rs`,
  `speccy-core/src/error.rs`

<task-scenarios>
  - When `parse` runs on a TASKS.md whose root `<tasks spec="SPEC-0022">`
    wraps two `<task id="T-001" state="pending" covers="REQ-001">`
    blocks each containing a non-empty `<task-scenarios>` body, it
    returns a `TasksDoc` with two `Task` entries whose `id`, `state`,
    `covers`, and `scenarios_body` fields match.
  - When `parse` runs on a `<task>` with `state="done"` (not in the
    enum `pending|in-progress|in-review|completed`), parsing fails
    with a diagnostic that names the task id, the rejected state
    value, and the four valid states.
  - When `parse` runs on a `<task>` containing zero `<task-scenarios>`
    blocks, parsing fails and names the task.
  - When `parse` runs on a `<task>` containing two `<task-scenarios>`
    blocks, parsing fails with a duplicate-element diagnostic.
  - When `parse` runs on a `<task>` whose `covers` attribute is
    missing, parsing fails and names the task.
  - When `parse` runs on a `<task>` whose `covers` value is
    `"REQ-001  REQ-002"` (double space) or contains a tab, parsing
    fails with a diagnostic that quotes the SPEC-0022 grammar
    ("single ASCII space separated `REQ-\d{3,}` ids").
  - When `parse` runs on a TASKS.md with two `<task id="T-001">`
    blocks, parsing fails with a duplicate-id diagnostic.
  - When `parse` runs on a `<task>` carrying an attribute outside
    `{id, state, covers}`, parsing fails with an unknown-attribute
    diagnostic that lists the valid attribute set.
  - When `render` runs on a `TasksDoc`, then re-parsing the rendered
    bytes yields a `TasksDoc` whose tasks list is field-by-field
    equal to the input (not via `Debug` string).
  - When `render` runs twice on the same `TasksDoc`, the outputs are
    byte-identical (idempotence).
  - When `render` runs on a `Task` whose `scenarios_body` contains
    `<` / `>` / `&` / a fenced Markdown code block / a literal
    `<task>` token inside backticks, those bytes pass through
    verbatim and the re-parser does not promote them to structure.
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-002 REQ-003">
REPORT.md typed model, parser, and renderer

- Suggested files: `speccy-core/src/parse/report_xml/mod.rs`,
  `speccy-core/src/parse/mod.rs`,
  `speccy-core/tests/fixtures/report_xml/canonical.md`,
  `speccy-core/tests/report_xml_roundtrip.rs`,
  `speccy-core/src/error.rs`

<task-scenarios>
  - When `parse` runs on a REPORT.md whose root `<report spec="SPEC-0022">`
    wraps three `<coverage req="REQ-NNN" result="..." scenarios="...">`
    blocks (one each of `satisfied`, `partial`, `deferred`), it
    returns a `ReportDoc` with three `RequirementCoverage` entries
    whose `req`, `result`, `scenarios`, and `body` fields match.
  - When `parse` runs on `<coverage result="passed">` (not in the enum
    `satisfied|partial|deferred`), parsing fails and lists the three
    valid result values. The legacy `dropped` value must also be
    rejected to enforce the SPEC-0022 amendment.
  - When `parse` runs on `<coverage result="satisfied">` whose
    `scenarios` attribute is missing entirely, parsing fails with a
    missing-required-attribute diagnostic.
  - When `parse` runs on `<coverage result="satisfied" scenarios="">`,
    parsing fails because `satisfied` requires at least one scenario
    id.
  - When `parse` runs on `<coverage result="partial" scenarios="">`,
    parsing fails because `partial` requires at least one scenario id.
  - When `parse` runs on `<coverage result="deferred" scenarios="">`,
    parsing succeeds and yields a coverage row with an empty
    scenarios vector (the attribute must be present-but-empty).
  - When `parse` runs on `<coverage scenarios="CHK-001  CHK-002">`
    (double space), parsing fails with a grammar diagnostic mirroring
    the TASKS.md attribute-format error.
  - When `parse` runs on a `<coverage>` carrying an attribute outside
    `{req, result, scenarios}`, parsing fails with an unknown-attribute
    diagnostic listing the valid set.
  - When `render` runs on a `ReportDoc`, re-parsing yields a
    `ReportDoc` whose coverage list is field-by-field equal to the
    input.
  - When `render` runs twice on the same `ReportDoc`, the outputs
    are byte-identical.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-001 REQ-002">
Workspace-load cross-reference validation across SPEC, TASKS, and REPORT

- Suggested files: `speccy-core/src/parse/cross_ref.rs`,
  `speccy-core/src/workspace.rs`,
  `speccy-core/tests/fixtures/workspace_xml/`,
  `speccy-core/tests/workspace_xml.rs`

<task-scenarios>
  - When workspace loading runs on a spec folder whose TASKS.md
    contains a task with `covers="REQ-999"` while the parent SPEC.md
    has no `REQ-999`, it fails with a dangling-requirement diagnostic
    that names the task id, the missing requirement id, and the
    TASKS.md path.
  - When workspace loading runs on a spec folder whose REPORT.md
    contains a `<coverage req="REQ-999">` while the parent SPEC.md
    has no `REQ-999`, it fails with a dangling-requirement diagnostic
    that names the coverage element and the REPORT.md path.
  - When workspace loading runs on a spec folder whose REPORT.md
    contains a `<coverage req="REQ-001" scenarios="CHK-099">` while
    `REQ-001` in SPEC.md has no `CHK-099` nested under it, it fails
    with a dangling-scenario diagnostic that names the requirement,
    the missing scenario id, and the REPORT.md path.
  - When workspace loading runs on a spec folder where SPEC.md has
    `REQ-001` and `REQ-002` but REPORT.md has only one `<coverage>`
    for `REQ-001`, it fails with a missing-coverage diagnostic that
    names every uncovered requirement (REQ-002 in this case).
  - When workspace loading runs on a spec folder with no REPORT.md
    yet (in-flight implementation), missing-coverage validation is
    skipped (it only runs when REPORT.md is present), but TASKS.md
    dangling-requirement validation still runs.
  - When workspace loading runs on a valid post-migration spec
    folder fixture, it succeeds with no diagnostics attributable to
    SPEC-0022 cross-ref validation.
</task-scenarios>
</task>

## Phase 2: Migration tool and in-tree rewrite


<task id="T-005" state="completed" covers="REQ-004">
Build `xtask/migrate-task-report-xml-0022`

- Suggested files:
  `xtask/migrate-task-report-xml-0022/Cargo.toml`,
  `xtask/migrate-task-report-xml-0022/src/main.rs`,
  `xtask/migrate-task-report-xml-0022/src/lib.rs`,
  `xtask/migrate-task-report-xml-0022/tests/fixtures/`,
  workspace root `Cargo.toml` (add the workspace member)

<task-scenarios>
  - When the migration runs on a TASKS.md fixture whose task list
    includes `- [ ] **T-001**` (open), `- [~] **T-002**`
    (in-progress), `- [?] **T-003**` (in-review), and `- [x] **T-004**`
    (done), the rewritten file emits `<task>` blocks whose `state`
    attributes are `pending`, `in-progress`, `in-review`, and
    `completed` respectively.
  - When the migration runs on a task line followed by a
    `- Covers: REQ-001, REQ-002` bullet, the rewritten task element
    has `covers="REQ-001 REQ-002"` (single ASCII space).
  - When the migration runs on a task whose body contains a
    `- Tests to write:` bullet sub-list, that sub-list is wrapped in
    a `<task-scenarios>` block in the rewritten task.
  - When the migration runs on a task with no `Tests to write:` (or
    equivalent task-local validation prose), migration fails with a
    diagnostic that names the task id and the missing prose, and
    emits no rewritten file.
  - When the migration runs on a TASKS.md, the frontmatter block, the
    level-1 heading line, phase headings (`## Phase N: ...`), and any
    `Suggested files:` / implementer-note / retry-note / review-note
    bullets are preserved as Markdown body content (byte-identical
    relative to their original location inside the new `<task>`
    block).
  - When the migration runs on a REPORT.md fixture containing a
    requirements-coverage table row mapping `REQ-001` to `Satisfied`
    with scenario list `CHK-001, CHK-002`, the rewritten file emits
    `<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">`.
  - When the migration runs on a REPORT.md row whose result column
    is `Dropped` (legacy value), migration fails with a diagnostic
    that names the requirement and instructs the user to amend
    SPEC.md to drop the requirement instead.
  - When the migration runs on a REPORT.md, the outcome, task
    summary, skill updates, out-of-scope, and deferred-limitations
    sections are preserved as Markdown body content byte-for-byte.
  - When the migration runs on the worked TASKS.md and REPORT.md
    fixtures, re-parsing the rewritten files with the post-T-002 /
    T-003 parsers succeeds and yields typed `TasksDoc` / `ReportDoc`
    values whose ids, states, and coverage results match the
    fixtures.
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-004">
Apply migration across `.speccy/specs/*/{TASKS,REPORT}.md` and confirm `speccy verify`

- Suggested files:
  `.speccy/specs/0001-artifact-parsers/TASKS.md` through
  `.speccy/specs/0022-xml-canonical-tasks-report/TASKS.md`,
  `.speccy/specs/*/REPORT.md`,
  `speccy-core/tests/in_tree_tasks_reports.rs`

<task-scenarios>
  - When the migration tool has run across every
    `.speccy/specs/*/TASKS.md` and `.speccy/specs/*/REPORT.md` in the
    workspace, each file parses successfully with the post-T-002 /
    T-003 parsers (asserted by an in-tree-corpus integration test
    analogous to SPEC-0021's `in_tree_specs.rs`).
  - When the in-tree corpus integration test reads each migrated
    TASKS.md, every task has a populated `<task-scenarios>` body
    (no empty bodies smuggled through).
  - When the in-tree corpus integration test reads each migrated
    REPORT.md, every coverage element's `req` and `scenarios` ids
    resolve against the parent SPEC.md.
  - When `speccy verify` runs against the post-migration workspace,
    it exits zero with no diagnostics attributable to SPEC-0022's
    parser or cross-ref validation.
  - When `git diff` is taken between the pre-migration and
    post-migration in-tree TASKS.md and REPORT.md files, the diff
    only adds/removes element tag lines and the canonical bullet
    restructure required by the new grammar (no prose changes
    inside task scenarios, no frontmatter changes, no heading text
    changes).
</task-scenarios>
</task>

## Phase 3: Switch callers to the typed models, delete heuristic parsers


<task id="T-007" state="completed" covers="REQ-003">
Switch CLI commands and prompts to the typed TASKS/REPORT models; delete the old heuristic parsers

- Suggested files:
  `speccy-core/src/parse/mod.rs` (drop `tasks_md`/`report_md`
  re-exports), `speccy-core/src/parse/tasks_md.rs` (delete),
  `speccy-core/src/parse/report_md.rs` (delete),
  `speccy-core/src/workspace.rs`, `speccy-core/src/next.rs`,
  `speccy-core/src/tasks.rs`, `speccy-core/src/task_lookup.rs`,
  `speccy-core/src/lint/rules/tsk.rs`,
  `speccy-core/src/lint/rules/spc.rs`,
  `speccy-cli/src/status.rs`, `speccy-cli/src/tasks.rs`,
  `speccy-cli/src/report.rs`,
  `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/reviewer-*.md`,
  `resources/modules/prompts/report.md`

<task-scenarios>
  - When `speccy next` runs on the post-migration in-tree workspace,
    its stdout (text mode) and `--json` schema are byte-identical to
    the pre-migration snapshot (golden-file test against fixtures
    captured before T-007 lands).
  - When `speccy status` runs on the post-migration in-tree
    workspace, its stdout and `--json` schema are byte-identical to
    the pre-migration snapshot.
  - When `speccy implement <task-id>` renders the implementer prompt
    for a TASKS.md authored under the new grammar, the rendered
    prompt cites the task's `<task-scenarios>` body and the
    requirement(s) named in `covers` as the validation contract.
  - When `speccy review <task-id>` renders the per-persona review
    prompts, those prompts cite `<task-scenarios>` (task-local) and
    `<scenario>` (requirement-level) distinctly, per the SPEC-0022
    user story about distinguishing slice-level from
    user-facing-level validation.
  - When `speccy report <spec-id>` renders the report prompt, it
    tells the agent to emit `<coverage>` elements rather than a
    Markdown coverage table.
  - When `speccy verify` runs against the post-migration in-tree
    workspace, it exits zero and uses the typed models (no fallback
    to the legacy `tasks_md` / `report_md` heuristic parsers).
  - When `grep -r "tasks_md::\|TasksMd\b\|report_md::\|ReportMd\b"`
    runs over `speccy-core/src` and `speccy-cli/src` after this
    task, there are zero hits outside the deleted-module shim or
    historical comments — the heuristic parsers have been removed
    and every caller reads the new typed models.
  - When `cargo build --workspace`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    and `cargo +nightly fmt --all --check` all run after the caller
    switch, all four exit zero.
</task-scenarios>
</task>

## Phase 4: Docs, prompts, skill packs, and cleanup


<task id="T-008" state="completed" covers="REQ-005">
Sweep ARCHITECTURE.md, prompts, and shipped skill packs to teach the new grammar

- Suggested files: `docs/ARCHITECTURE.md`,
  `resources/modules/prompts/tasks-generate.md`,
  `resources/modules/prompts/report.md`,
  `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `.claude/skills/`, `.codex/`,
  `resources/agents/.agents/`, `resources/agents/.codex/`

<task-scenarios>
  - When `docs/ARCHITECTURE.md` is read after this task, then the
    TASKS.md and REPORT.md element-names tables contain rows for
    `tasks`, `task`, `task-scenarios`, `report`, and `coverage`
    (each with cardinality, parent, and attribute columns).
  - When `docs/ARCHITECTURE.md` is read after this task, the four
    task states are documented as XML attribute values (`pending`,
    `in-progress`, `in-review`, `completed`); the old checkbox
    glyphs (`[ ]`, `[~]`, `[?]`, `[x]`) appear only in a clearly
    labelled migration-history note.
  - When `docs/ARCHITECTURE.md` is read after this task, the
    coverage result enum is documented as exactly
    `satisfied | partial | deferred` (no `dropped`), with a sentence
    noting that dropped requirements are removed from SPEC.md via
    amendment.
  - When `resources/modules/prompts/tasks-generate.md` is read after
    this task, it instructs the agent to emit XML-structured
    TASKS.md with `<tasks>`, `<task>`, and `<task-scenarios>`
    elements rather than checkbox bullets and `Tests to write:`
    sub-lists as the machine contract.
  - When `resources/modules/prompts/report.md` is read after this
    task, it instructs the agent to emit `<report>` and `<coverage>`
    elements rather than a Markdown coverage table.
  - When implementer and reviewer prompts under
    `resources/modules/prompts/` are read after this task, they cite
    `<task-scenarios>` and `<scenario>` by name, and read task state
    from the `state` XML attribute rather than a checkbox glyph.
  - When shipped skill packs under `.claude/skills/`, `.codex/`,
    `resources/agents/.agents/`, and `resources/agents/.codex/` are
    read after this task, any reference to TASKS.md or REPORT.md
    structure mentions the new XML element grammar rather than the
    checkbox/coverage-table conventions.
  - When a grep for the literal strings `- [ ] **T-`, `- [x] **T-`,
    `- [~] **T-`, `- [?] **T-`, or "requirements coverage table"
    runs across active (non-historical) guidance after this task,
    any hits are confined to migration-context documentation. Active
    prompts and ARCHITECTURE.md contain zero such hits.
</task-scenarios>
</task>

<task id="T-009" state="completed" covers="REQ-004">
Delete the ephemeral migration tool

- Suggested files: `xtask/migrate-task-report-xml-0022/` (delete),
  workspace root `Cargo.toml` (remove the workspace member entry)

<task-scenarios>
  - When the final implementation commit lands,
    `xtask/migrate-task-report-xml-0022/` no longer exists on disk.
  - When the workspace root `Cargo.toml` is read after this task,
    it lists no `migrate-task-report-xml-0022` workspace member.
  - When `cargo build --workspace`, `cargo test --workspace`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
    and `cargo +nightly fmt --all --check` all run after the
    migration tool is removed, all four exit zero.
</task-scenarios>

session-t009-2026-05-16
  - Completed: Removed `xtask/migrate-task-report-xml-0022/`
    directory via `rm -rf` and dropped the workspace member entry
    from the root `Cargo.toml`. The `members` array now reads
    `["speccy-cli", "speccy-core"]`. The empty `xtask/` directory
    was left in place per task instructions (no parent `xtask`
    crate).
  - Commands run: `rm -rf xtask/migrate-task-report-xml-0022`
    (pass), `cargo build --workspace` (pass, 0.04s, no rebuild),
    `cargo test --workspace` (pass, all suites green including
    workspace_xml's 6 tests),
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings` (pass, clean),
    `cargo +nightly fmt --all --check` (pass, EXIT=0; the
    `float_literal_trailing_zero` warnings are pre-existing
    rustfmt-config noise, not from this change),
    `grep -rn 'migrate-task-report-xml-0022\|migrate_task_report_xml_0022' .`
    (pass — only SPEC.md and TASKS.md hits, all documentary).
    Verified `Cargo.lock` has zero references via separate grep.
  - Discovered issues: (none)
  - Procedural compliance: (none)
</task>

