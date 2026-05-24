---
spec: SPEC-0038
spec_hash_at_generation: 28ddb67d358a43bcf2486db34f47c36102b060f917056ce5d65f598ed8a4d188
generated_at: 2026-05-23T07:36:29Z
---
# Tasks: SPEC-0038 Skill-pack references — per-skill and host-shared reference files eject every lintable artifact's canonical shape

<task id="T-001" state="completed" covers="REQ-003">
## Author the seven canonical reference files under `resources/modules/references/`

Create the `resources/modules/references/` directory and write the seven reference files with canonical post-SPEC-0034 content:

- `spec.md` — worked SPEC.md instance with frontmatter, `<requirement id="REQ-NNN">`, `<scenario id="CHK-NNN">`, `<done-when>`, `<behavior>` elements.
- `tasks.md` — worked TASKS.md instance with YAML frontmatter (`spec:`, `spec_hash_at_generation:`, `generated_at:`), `# Tasks: SPEC-NNNN` heading, and a `<task covers="REQ-001 REQ-002">` element showing space-separated `covers=`.
- `report.md` — worked REPORT.md instance with `spec:`, `outcome:`, `generated_at:` frontmatter, `<report spec="SPEC-NNNN">` root, and `<coverage req="REQ-NNN" result="..." scenarios="...">` rows.
- `journal-implementer.md` — worked `<implementer>` block using the six post-SPEC-0034 fields in order: `Completed:`, `Undone:`, `Hygiene checks:`, `Evidence:`, `Discovered issues:`, `Procedural compliance:`. Must not contain `Commands run:` or `Exit codes:` as bullet-line prefixes.
- `journal-review.md` — worked `<review persona="..." verdict="..." model="..." date="..." round="...">` block with all five required attributes.
- `evidence.md` — worked evidence file opening with `# Evidence for SPEC-NNNN T-NNN` heading (matching `^# Evidence for SPEC-\d{4} T-\d{3}$`), followed by bare `<red>` / `<green>` blocks. No `<evidence task=` wrapper element.
- `journal-blockers.md` — worked `<blockers date="..." round="...">` block with both required attributes.

None of the seven files may contain `TBD`, `TODO`, or `<...>` placeholder substrings.

<task-scenarios>
Given `resources/modules/references/` is created at HEAD after this task,
when each of the seven files is read,
then each contains non-empty worked-instance content (no `<...>` placeholders), all post-SPEC-0034 field names are present in `journal-implementer.md` in the canonical order, `evidence.md`'s first line matches `^# Evidence for SPEC-\d{4} T-\d{3}$`, and `evidence.md` contains no `<evidence task=` substring.

Suggested files: `resources/modules/references/` (new directory and seven new `.md` files)
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-005">
## Delete orphan files and salvage prose into successor locations

Remove the four orphan paths:
- `resources/modules/personas/implementer.md`
- `resources/modules/personas/planner.md`
- `resources/modules/examples/evidence.md`
- `resources/modules/examples/` (directory)

Fold salvageable prose into successor locations:
- `resources/modules/phases/speccy-work.md` — add salvaged "What to consider" bullets from the deleted `personas/implementer.md` (feature-flag/abstraction-layer guardrail, suggested-files-hint-may-be-stale warning, `<done-when>` / `<behavior>` re-reading reminder). Do not salvage the example block (that becomes `references/journal-implementer.md` in T-001).
- `resources/modules/skills/speccy-plan.md` — add salvaged bullets from the deleted `personas/planner.md` (bounded-scope guardrail, "decisions hidden inside requirement prose belong in `### Decisions`" guidance). Do not salvage the anecdotal email-signup example.

**Salvage discipline**: shipped skill bodies (`resources/modules/skills/`, `resources/modules/phases/`, `resources/modules/personas/`) MUST NOT reference repo-local docs like `docs/ARCHITECTURE.md`. The deleted `personas/planner.md:43` mentions ARCHITECTURE.md; that reference is dropped during salvage, not forwarded. Skill bodies must stay portable to any speccy-using repo.

Remove `"personas/implementer.md"` and `"personas/planner.md"` from the `PERSONA_FILES` const in `speccy-cli/tests/skill_body_discovery.rs`.

<task-scenarios>
Given the source tree at HEAD after this task,
when the four orphan paths are checked for existence,
then none exist; when `phases/speccy-work.md` is grep'd for at least one salvaged bullet substring, then at least one match occurs; when `skills/speccy-plan.md` is grep'd for at least one salvaged bullet, then at least one match occurs; when `skill_body_discovery.rs` is scanned for the `PERSONA_FILES` const, then neither `personas/implementer.md` nor `personas/planner.md` appears in it; when `cargo test --workspace` runs, then no test fails due to missing persona files.

Suggested files: `resources/modules/personas/implementer.md`, `resources/modules/personas/planner.md`, `resources/modules/examples/evidence.md`, `resources/modules/phases/speccy-work.md`, `resources/modules/skills/speccy-plan.md`, `speccy-cli/tests/skill_body_discovery.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001 REQ-002">
## Wire reference files into `speccy init` — extend `embedded.rs`, `init.rs`, and the `MiniJinja` template context

Extend the embed pipeline, init plan, and host template context so the new directories ship to both host packs and so consuming bodies can emit host-rooted pointers.

**Embed + init plan**:
- Add the seven source files from `resources/modules/references/` to `speccy-cli/src/embedded.rs` (the embedded resource manifest). The seven files share one canonical source; both host packs ship byte-identical copies via the templating step below.
- Update `speccy-cli/src/init.rs` to include `create` plan entries for:
  - `.claude/skills/speccy-plan/references/spec.md`
  - `.claude/skills/speccy-tasks/references/tasks.md`
  - `.claude/skills/speccy-ship/references/report.md`
  - `.claude/skills/speccy-work/references/journal-implementer.md`
  - `.claude/skills/speccy-review/references/journal-review.md`
  - `.claude/speccy-references/evidence.md`
  - `.claude/speccy-references/journal-blockers.md`
  - Parallel set under `.agents/` for the Codex host pack.
- Decide between two ejection paths (both consistent with the SPEC's "templating mechanism unchanged" constraint):
  1. Mirror each canonical source file into wrapper trees under `resources/agents/.claude/skills/<skill>/references/<file>.md.tmpl`, `resources/agents/.claude/speccy-references/<file>.md.tmpl`, and their `.agents/` siblings, each containing a single `{% include "modules/references/<file>.md" %}` directive — so the existing `render_host_pack` walk picks them up. **Preferred** since it keeps the rendering path uniform; the wrapper files are mechanical and the byte-identical invariant in REQ-007 catches drift.
  2. Add a third render pass (analogous to the existing `render_speccy_examples_pack` that this SPEC's T-002 retires) that copies `modules/references/*` into both host packs without templating.
- Confirm `speccy init --force` semantics: the new directories refresh in place without disturbing user-authored skill files outside `references/`.

**Host template context** (`speccy-cli/src/host.rs`):
- Add a new `speccy_references_path` key (or equivalent — name is implementer's call provided REQ-004's host-rooted pointer form renders cleanly) to `TemplateContext`, resolving to `".claude/speccy-references"` for Claude Code and `".agents/speccy-references"` for Codex (per DEC-002; note Codex sub-agents live at `.codex/agents/*.toml` but their speccy-references pointer resolves to `.agents/...`, not `.codex/...`).
- Update the docstring on `HostChoice::template_context` to enumerate the new key.
- Extend `template_context_claude_code_renders_expected_keys` and `template_context_codex_renders_expected_keys` in `host.rs` to assert the new key renders to the expected value for each host.

Consuming bodies in T-004 / T-005 then write `{{ speccy_references_path }}/<file>.md` to emit a host-rooted pointer at render time. Skill-local pointers stay relative (`references/<file>.md`) and need no context plumbing.

<task-scenarios>
Given a fresh tempdir where `speccy init --host claude-code` has run exactly once after this task,
when the directory tree is listed,
then all seven Claude Code reference paths exist as regular files with non-empty content; when `speccy init --host codex` runs against a sibling tempdir, then all seven Codex paths exist; when each Claude Code file is byte-compared to its Codex counterpart, then every pair is byte-identical; when the `host.rs` template-context tests run, then both `claude-code` and `codex` cases assert the new `speccy_references_path` key renders to `.claude/speccy-references` and `.agents/speccy-references` respectively.

Suggested files: `speccy-cli/src/embedded.rs`, `speccy-cli/src/init.rs`, `speccy-cli/src/host.rs`, `resources/agents/.claude/skills/*/references/*.md.tmpl` (option 1 wrapper stubs), `resources/agents/.claude/speccy-references/*.md.tmpl` (option 1 wrapper stubs), and the `.agents/` siblings.
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-004">
## Add one-line path pointers to consuming skill/phase/sub-agent bodies; remove inline example blocks ≥ 8 lines

Update consuming source bodies to carry one pointer line per reference file and remove any inline example shape block of eight or more lines for the same artifact:

- `resources/modules/skills/speccy-plan.md` → pointer to `references/spec.md`.
- `resources/modules/phases/speccy-tasks.md` → pointer to `references/tasks.md`; remove/shrink the ~20-line inline TASKS.md fragment at current lines 38-58.
- `resources/modules/phases/speccy-ship.md` → pointer to `references/report.md`.
- `resources/modules/phases/speccy-work.md` → pointer to `references/journal-implementer.md`; remove the ~14-line inline `<implementer>` block example at current lines 73-87; add pointer to `speccy-references/evidence.md` (Tera-templated host prefix).
- `resources/modules/skills/speccy-review.md` → pointer to `references/journal-review.md`; pointer to `speccy-references/journal-blockers.md` (Tera-templated).
- `resources/modules/skills/speccy-amend.md` → pointer to `speccy-references/journal-blockers.md` (Tera-templated).

Pointer form for skill-local: `references/<file>.md`. Pointer form for host-shared: use the `speccy_references_path` template context key T-003 added (or whatever name T-003 settled on) — bodies write `{{ speccy_references_path }}/<file>.md`, which `MiniJinja` renders to `.claude/speccy-references/<file>.md` for Claude Code and `.agents/speccy-references/<file>.md` for Codex. Verify post-change body length for `speccy-tasks.md` and `speccy-work.md` is shorter by at least 8 lines compared to pre-change.

<task-scenarios>
Given the rendered `.claude/agents/speccy-tasks.md` post-this-task,
when grep'd for `references/tasks.md`, then exactly one match occurs and the body is shorter than before by at least 8 lines; given the rendered `.claude/agents/speccy-work.md`, when grep'd for `references/journal-implementer.md`, then exactly one match occurs; when grep'd for `.claude/speccy-references/evidence.md`, then exactly one match occurs; when grep'd for `Commands run:` or `Exit codes:` as start-of-bullet-line prefixes, then zero matches occur.

Suggested files: `resources/modules/skills/speccy-plan.md`, `resources/modules/phases/speccy-tasks.md`, `resources/modules/phases/speccy-ship.md`, `resources/modules/phases/speccy-work.md`, `resources/modules/skills/speccy-review.md`, `resources/modules/skills/speccy-amend.md`
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-006">
## Add cross-skill pointer to `reviewer-tests.md`; audit other reviewer personas per DEC-001

Update `resources/modules/personas/reviewer-tests.md` to add one pointer line `{{ speccy_references_path }}/evidence.md` (rendering to `.claude/speccy-references/evidence.md` for Claude Code and `.agents/speccy-references/evidence.md` for Codex via the context key T-003 added) inside the existing "Evidence loading" section. Preserve the four-step procedure and fabrication-pattern bullets unchanged.

Audit the remaining reviewer personas (`reviewer-business.md`, `reviewer-security.md`, `reviewer-style.md`, `reviewer-architecture.md`, `reviewer-docs.md`) against the DEC-001 outcome: no pointer is added to any of them (their prose operates on the diff, not a reference-shipping artifact shape).

<task-scenarios>
Given the rendered `.claude/agents/reviewer-tests.md` post-this-task,
when grep'd for `.claude/speccy-references/evidence.md`, then exactly one match occurs; given the rendered `.codex/agents/reviewer-tests.toml`, when grep'd for `.agents/speccy-references/evidence.md`, then exactly one match occurs and zero matches for the `.claude/...` form; given each of the other five reviewer persona rendered outputs, when grep'd for any `references/` or `speccy-references/` path, then zero matches occur.

Suggested files: `resources/modules/personas/reviewer-tests.md`
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-007">
## Add `chk0NN_no_orphan_references` test to `skill_body_discovery.rs`

Add a new test function (next free `chkNNN_*` number after existing tests) to `speccy-cli/tests/skill_body_discovery.rs` that:

1. Enumerates ejected reference files by globbing the fresh-init'd host-pack tree: `.claude/skills/*/references/*.md` ∪ `.claude/speccy-references/*.md` (and the `.agents/` counterparts for Codex).
2. For each reference file computes the path-substring(s) a consuming body would use (`references/<file>.md` for skill-local; `.claude/speccy-references/<file>.md` / `.agents/speccy-references/<file>.md` for host-shared).
3. Scans every consuming body (`.md` under `.claude/skills/*/` and `.claude/agents/*.md`; `.md` and `.toml` under `.agents/skills/*/` and `.codex/agents/*.toml`) for the path substring via raw-content grep (no TOML parser required).
4. Fails with a named-orphan message if any reference file has zero consuming-body matches.
5. Asserts cross-host byte-identical parity: for every reference file, the Claude Code path content equals the Codex path content.
6. Asserts source-to-host parity: for every reference file, the `resources/modules/references/<file>.md` source content equals each host's ejected copy; failure message names the diverging path triple and the byte offset of the first difference.

The enumeration uses a glob, not a hard-coded file list, so a future eighth reference file does not require a test update.

<task-scenarios>
Given the workspace post-this-task, when `cargo test --workspace -- chkNNN_no_orphan_references` runs, then it passes; given the workspace with one extra file placed at `.claude/skills/speccy-plan/references/orphan.md` and no consuming pointer added, when the test runs, then it fails with a message containing `orphan.md`; given the workspace where `.claude/skills/speccy-plan/references/spec.md` has been altered to differ in one byte from its Codex counterpart, when the test runs, then it fails naming both paths; given the workspace where `reviewer-tests.toml`'s Codex pointer line is removed, when the test runs, then it fails naming `evidence.md` and the Codex host pack.

Suggested files: `speccy-cli/tests/skill_body_discovery.rs`
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-002">
## Update `docs/ARCHITECTURE.md` to document the seven-row reference-file mapping (repo-local; final step)

Per REQ-002's `<done-when>` clause, `docs/ARCHITECTURE.md`'s "Skill packs" (or equivalent) section either documents the seven-row artifact→reference-file mapping directly, or links to SPEC-0038 REQ-002 as the source of truth. This is a repo-local documentation task — `docs/ARCHITECTURE.md` is a byproduct of *this* speccy-development repo dogfooding the tool on itself; it is NOT shipped to downstream speccy users.

Discipline guard: shipped skill bodies (`resources/modules/skills/`, `resources/modules/phases/`, `resources/modules/personas/`) and shipped reference files (`resources/modules/references/`) must NOT mention `ARCHITECTURE.md`. ARCHITECTURE.md is repo-local content; shipped skill content must stay portable to any speccy-using repo. T-002's salvage discipline guard covers this for the orphan-personas case; T-007 audits the post-T-001/T-002/T-004/T-005 state of `resources/` for any residual ARCHITECTURE.md mentions and removes any that slipped in.

This task runs last so the documentation update reflects the final shape of REQ-002's mapping after all other tasks have landed.

<task-scenarios>
Given the source tree at HEAD after this task,
when `docs/ARCHITECTURE.md` is read, then its "Skill packs" (or equivalent) section either lists the seven artifact→reference-file rows from REQ-002 or links to SPEC-0038 REQ-002 as the canonical source; when `resources/modules/` and `resources/agents/` are grep'd recursively for the literal substring `ARCHITECTURE.md`, then zero matches occur.

Suggested files: `docs/ARCHITECTURE.md`
</task-scenarios>
</task>
