---
id: SPEC-0038
slug: skill-pack-references
title: Skill-pack references — per-skill and host-shared reference files eject every lintable artifact's canonical shape
status: implemented
created: 2026-05-21
supersedes: []
---

# SPEC-0038: Skill-pack references — per-skill and host-shared reference files eject every lintable artifact's canonical shape

## Summary

Speccy's shipped skills (`/speccy-plan`, `/speccy-tasks`, `/speccy-ship`,
`/speccy-work`, `/speccy-review`, `/speccy-amend`) and their sibling
sub-agents (`reviewer-business`, `reviewer-tests`, `reviewer-security`,
`reviewer-style`, plus the off-default `reviewer-architecture`,
`reviewer-docs`) drive LLM agents through writing seven kinds of
lintable artifacts: `SPEC.md`, `TASKS.md`, `REPORT.md`, the per-task
journal's `<implementer>` block, the per-task journal's `<review>`
block, the per-task journal's `<blockers>` block, and the per-task
`evidence/T-NNN.md` paper-trail file. The CLI lints each artifact's
proof shape (`SPC-*`, `REQ-*`, `TSK-*`, `RPT-*`, `QST-*`,
`JNL-*`); when a skill produces a malformed artifact, the lint catches
it and the agent burns tokens correcting on a second attempt.

The shipped skill bodies today describe each artifact's shape in prose
and, where they do carry an inline example, frequently disagree with
the canonical post-SPEC-0034 shape. The implementer handoff template
inside `resources/modules/phases/speccy-work.md` (rendered into
`.claude/agents/speccy-work.md:88-90` and `.claude/agents/speccy-work.md:124-125`)
still teaches the pre-SPEC-0034 six-field set
`Completed / Undone / Commands run / Exit codes / Discovered issues /
Procedural compliance`. SPEC-0034's REQ-001 replaced that with
`Completed / Undone / Hygiene checks / Evidence / Discovered issues /
Procedural compliance` (see `.speccy/specs/0034-authoring-self-review/SPEC.md:267`).
The journal file at `.speccy/specs/0037-task-journal-files/journal/T-007.md:34-54`
was authored against the stale prompt and carries the old field names
in real implementer work. The same diagnosis applies to the evidence
file shape: `resources/modules/examples/evidence.md` wraps its
`<red>` / `<green>` blocks in an `<evidence task="..." spec="...">`
wrapper element from SPEC-0031; SPEC-0034's REQ-002 canonized a
header-convention shape (`# Evidence for SPEC-NNNN T-NNN` followed by
bare `<red>` / `<green>` blocks, no wrapper) which real in-tree
evidence files at `.speccy/specs/0034-authoring-self-review/evidence/T-007.md`
follow. The example file lives at `resources/modules/examples/evidence.md`
but is referenced by no Tera `{% include %}`, no Read pointer, and no
prose anywhere in the shipped skill bodies. It is an orphan with stale
content.

Two persona files in `resources/modules/personas/` —
`implementer.md` and `planner.md` — are similarly orphaned:
neither is `{% include %}`-ed by any wrapper template under
`resources/agents/`; only the integration tests at
`speccy-cli/tests/skill_body_discovery.rs:87-88` load them. The
`implementer.md` body's worked example (lines 76-95) also carries
the pre-SPEC-0034 field names. The `planner.md` body's worked
example (lines 58-64) is anecdotal prose, not a worked artifact.

The fix is to ship the canonical shape of each lintable artifact as
an ejected reference file under a `references/` directory inside the
owning skill's host-pack folder (Claude Code:
`.claude/skills/<skill>/references/<artifact>.md`; Codex:
`.agents/skills/<skill>/references/<artifact>.md`), and for the two
cross-skill cases (evidence shape, journal `<blockers>` shape) ship
the reference into host-shared `speccy-references/` directories at
host root (`.claude/speccy-references/<file>.md` and
`.agents/speccy-references/<file>.md`). The skill bodies that today
describe each artifact in prose gain a one-line pointer to the
reference file and lose any inline example shape block of eight or
more lines. The orphan files in `resources/modules/personas/` and
`resources/modules/examples/` are removed; salvageable prose folds
into the owning skill or phase body. A new test in
`speccy-cli/tests/skill_body_discovery.rs` asserts every shipped
reference file is reached by at least one path-substring pointer
from a SKILL.md, phase body, or sub-agent body inside the same
host pack — so the orphan-evidence-file failure mode that motivated
this SPEC cannot recur silently.

The pattern matches the skill-creator anatomy's third progressive
disclosure level ("references/ — Docs loaded into context as
needed"); speccy skills today exploit only the first two levels
(frontmatter metadata + SKILL.md body) and are missing the third.
This SPEC closes that gap on artifact-reference content. Self-review
prose triplication across `/speccy-plan`, `/speccy-amend`, and
`/speccy-brainstorm` (DEC-001 in SPEC-0034) is procedure content,
not artifact reference, and stays out of scope — see `## Notes`
for the follow-up signal.

## Goals

<goals>
- Each Speccy skill that produces or consumes a lintable artifact
  ships a per-skill `references/` directory under both host packs
  (`.claude/skills/<skill>/references/`,
  `.agents/skills/<skill>/references/`), ejected by `speccy init`
  with byte-identical content across hosts.
- Cross-skill reference content (multi-consumer) ships in host-shared
  `speccy-references/` directories at host root: `.claude/speccy-references/`
  and `.agents/speccy-references/`. The two cross-skill cases in scope
  are the evidence-file shape (`evidence.md`, referenced by
  `/speccy-work`'s implementer-side phase body and the
  `reviewer-tests` sub-agent body) and the journal `<blockers>`
  shape (`journal-blockers.md`, referenced by `/speccy-review`'s
  skill body and `/speccy-amend`'s skill body).
- Source-side canonical layout: `resources/modules/examples/` is
  renamed to `resources/modules/references/`. Reference files live
  there as the single canonical source and template identically
  into both host packs at init time. No host-specific reference
  content; both host packs receive byte-identical files.
- Reference file naming inside `references/` and `speccy-references/`
  directories is lowercase plain (no `-REFERENCE` suffix, no
  `EXAMPLE-` prefix). The names in scope are `spec.md`, `tasks.md`,
  `report.md`, `journal-implementer.md`, `journal-review.md`,
  `evidence.md`, `journal-blockers.md`.
- Reference content reflects post-SPEC-0034 canonical shape: the
  `<implementer>` six-field handoff template uses `Hygiene checks` /
  `Evidence` (not pre-SPEC-0034 `Commands run` / `Exit codes`); the
  evidence file uses the `# Evidence for SPEC-NNNN T-NNN` header
  convention with bare `<red>` / `<green>` blocks (no `<evidence>`
  wrapper element).
- Each consuming body (SKILL.md, phase body, sub-agent body)
  contains a one-line path pointer to its reference file using the
  relative form `references/<file>.md` (for skill-local pointers
  from within the owning skill's body) or the host-rooted form
  `.claude/speccy-references/<file>.md` /
  `.agents/speccy-references/<file>.md` (for shared cross-skill
  pointers from a body whose owning skill does not own the
  reference). No consuming body inlines an example shape block of
  eight or more lines for any artifact whose canonical shape ships
  as a reference file.
- Orphan files in `resources/modules/` cleared: `personas/implementer.md`,
  `personas/planner.md`, `examples/evidence.md`, and the now-empty
  `examples/` directory no longer exist after this SPEC.
  Salvageable prose from the deleted persona files folds into
  `phases/speccy-work.md` (from `implementer.md`) and
  `skills/speccy-plan.md` (from `planner.md`); the deleted evidence
  file's content becomes `references/evidence.md` reshaped to the
  post-SPEC-0034 header-convention shape.
- The reviewer personas under `resources/modules/personas/reviewer-*.md`
  that consume a cross-skill reference gain their host-rooted
  pointer in this SPEC. The load-bearing case is
  `reviewer-tests.md`, which gains its pointer to
  `speccy-references/evidence.md`; other reviewer personas are
  audited and gain pointers if and only if their existing prose
  describes a reference-shipping artifact.
- `speccy-cli/tests/skill_body_discovery.rs` gains a new test
  (`chk0NN_no_orphan_references`) asserting every reference file
  shipped in the rendered skill-pack directory tree (under
  `.claude/skills/*/references/`, `.agents/skills/*/references/`,
  `.claude/speccy-references/`, `.agents/speccy-references/`) is
  reached by at least one path-substring pointer in a SKILL.md
  (`.md`), phase body (`.md`), or sub-agent body (`.md` for Claude
  Code at `.claude/agents/*.md`, `.toml` for Codex at
  `.codex/agents/*.toml`) inside the same host pack.
</goals>

## Non-goals

<non-goals>
- No CLI surface change. The seven CLI verbs (`init`, `status`,
  `next`, `check`, `verify`, `lock`, `vacancy`) retain their current
  command shapes, JSON envelopes, and lint families. The
  orphan-references check lives in the test layer
  (`skill_body_discovery.rs`), not in `speccy verify`.
- No new lint code family (`SKL-NNN` or otherwise) added to
  `speccy verify` for skill-pack hygiene. Skill-pack content is a
  test-time concern, not a CI gate inside the deterministic CLI.
- No change to the seven-noun set or the seven-command surface;
  this SPEC ships shipped-content changes only.
- No reconsideration of DEC-001 from SPEC-0034 (independent
  self-review prose copies across `/speccy-plan`,
  `/speccy-amend`, and `/speccy-brainstorm`). The triplicated
  prose is procedure content (rules an agent follows), not
  artifact reference content (shape of a file an agent produces).
  The divergence-allowance argument DEC-001 made still applies.
  Surfaced in `## Notes` as a follow-up signal for a separate
  SPEC once the `references/` pattern proves out on artifact
  content.
- No introduction of a host-agnostic `.speccy/examples/` or
  `.speccy/references/` directory for reference content.
  Reference files live in host-namespaced folders alongside the
  skill packs they support; `.speccy/` remains purely about
  project state (specs, missions, evidence, journals).
- No host-specific reference file content. The canonical source
  under `resources/modules/references/` templates identically
  into both host packs; reference files do not branch on host
  identity, and the cross-host test in REQ-007 asserts the
  byte-identical invariant.
- No YAML frontmatter on reference files. Each reference file is
  plain Markdown; the lint test matches pointers by path
  substring, not by frontmatter schema.
- No change to the existing personas under
  `resources/modules/personas/reviewer-*.md` beyond adding the
  cross-skill pointer that REQ-004 requires for any persona
  consuming a reference-shipping artifact's shape. Persona prose,
  focus lists, fabrication-pattern lists, and verdict-return
  contracts are out of scope; only the cross-skill pointer is
  touched.
- No introduction of inline example shape blocks anywhere in the
  consuming bodies after this SPEC's diff lands. The whole point
  of the `references/` pattern is to remove the duplication; a
  body that points to a reference file and also inlines the same
  shape "for convenience" defeats the pattern.
- No retroactive migration of past in-tree journal files
  (`.speccy/specs/*/journal/*.md`) to fix pre-SPEC-0034
  `Commands run` / `Exit codes` field naming. Existing journals
  are historical record and stay as they were written. The fix
  applies to the reference content that drives future journal
  authoring.
- No change to the Tera-templating mechanism in
  `speccy-cli/src/render.rs` or the `embedded.rs` resource
  manifest beyond adding the new `references/` and
  `speccy-references/` directories as ejected resources. The
  template engine itself is unchanged; the layout produced gains
  new sibling directories.
- No CI change. The existing standard-hygiene gate
  (`cargo test --workspace`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo +nightly
  fmt --all --check`, `cargo deny check`) continues as-is. The
  new `chk0NN_no_orphan_references` test participates in
  `cargo test --workspace` as a sibling of the existing CHK-*
  tests in `skill_body_discovery.rs`.
- No deletion or modification of any existing reviewer persona's
  fabrication-pattern list, `<diff_fetch_command>` include, or
  `<verdict_return_contract>` include. Reviewer personas are
  audited for cross-skill pointers (REQ-004), nothing else.
- No new skill or agent file. The set of shipped skills
  (speccy-init, speccy-plan, speccy-brainstorm, speccy-tasks,
  speccy-work, speccy-review, speccy-amend, speccy-ship) and
  shipped sub-agents (reviewer-business, reviewer-tests,
  reviewer-security, reviewer-style, reviewer-architecture,
  reviewer-docs, speccy-tasks, speccy-work, speccy-ship)
  remains unchanged in count and naming.
</non-goals>

## User Stories

<user-stories>
- As an LLM agent invoked via `/speccy-plan`, I want a one-line
  pointer to a worked-instance reference of SPEC.md shape that
  I can Read on first encounter, so that the SPEC.md I produce
  passes `speccy verify`'s `SPC-*` / `REQ-*` lints on the first
  attempt rather than after a correction round.
- As an LLM agent invoked via `/speccy-tasks`, I want a
  worked-instance reference of TASKS.md shape (frontmatter
  layout, `# Tasks: SPEC-NNNN` heading constraint, space-separated
  `covers="REQ-001 REQ-002"` form) at a stable path my skill body
  points to, so that the TASKS.md I produce passes the `TSK-*`
  lints without re-deriving the constraint each session.
- As an LLM agent invoked via `/speccy-ship`, I want a
  worked-instance reference of REPORT.md shape (frontmatter,
  `<report>` root with required `spec="..."` attribute,
  `<coverage>` rows with `req=` / `result=` / `scenarios=`
  attributes) at a stable path my skill body points to, so that
  the REPORT.md I produce passes the `RPT-*` lints SPEC-0035
  added on the first attempt.
- As an LLM implementer agent invoked via `/speccy-work`, I want
  a worked-instance reference of the post-SPEC-0034
  `<implementer>` six-field handoff template (`Completed`,
  `Undone`, `Hygiene checks`, `Evidence`, `Discovered issues`,
  `Procedural compliance`) at a stable path my phase body points
  to, so that the journal entries I write carry the canonical
  field names rather than the pre-SPEC-0034 `Commands run` /
  `Exit codes` naming that the current phase body still teaches
  (and that SPEC-0037/T-007's journal still records as a result).
- As an LLM `reviewer-tests` sub-agent spawned by `/speccy-review`,
  I want a worked-instance reference of the evidence-file shape
  (header convention `# Evidence for SPEC-NNNN T-NNN`, bare
  `<red>` / `<green>` blocks, no `<evidence>` wrapper element)
  at a stable shared path my agent body points to, so that I
  can flag fabricated-looking evidence against a known-good
  baseline rather than against an evolving prose description.
- As a Speccy contributor editing a shipped skill body or
  adding a new reference file, I want the orphan-references
  test in `skill_body_discovery.rs` to fail loudly when I add
  a reference file with no consuming pointer, so that the
  `references/` pattern does not degrade into orphaned content
  over time. The existing `resources/modules/examples/evidence.md`
  orphan that this SPEC removes is the failure mode the test
  prevents.
- As a Codex user invoking `/speccy-work` or the
  `reviewer-tests` sub-agent, I want the same reference files
  my Claude Code counterpart sees, with the same content at
  host-mirrored paths (`.agents/skills/<skill>/references/<file>.md`
  instead of `.claude/skills/<skill>/references/<file>.md`,
  `.agents/speccy-references/<file>.md` instead of
  `.claude/speccy-references/<file>.md`), so that the two host
  packs produce equivalent artifact shapes from equivalent
  prompts.
- As a future Speccy contributor extending the skill packs with
  a new lintable artifact (e.g., a hypothetical `MISSION.md`
  reference once mission folders accumulate enough usage), I
  want the `references/` pattern to be the established home for
  the new artifact's shape, so that I do not reinvent the layout
  decision and the orphan-references test catches my orphan if
  I forget the pointer.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Reference files ship in two location classes ejected by `speccy init`

Each Speccy host pack ships two classes of reference file location.
Per-skill references (single-consumer) live inside the owning
skill's directory under a `references/` subfolder. Host-shared
references (multi-consumer, cross-skill) live in a
`speccy-references/` directory at host root. Both classes ship
in both host packs (Claude Code and Codex) with byte-identical
content across hosts and across location classes per artifact.

The canonical source side is a new directory at
`resources/modules/references/` (renamed from the existing
`resources/modules/examples/` per REQ-005). `speccy-cli/src/init.rs`
and `speccy-cli/src/embedded.rs` template the files in that
directory into both host packs at init time. The Tera-include
mechanism in `speccy-cli/src/render.rs` is unchanged; the
init plan gains additional `create` entries for the new directories
and their files.

The per-skill `references/` directory ships under
`.claude/skills/<skill>/references/<file>.md` (Claude Code) and
`.agents/skills/<skill>/references/<file>.md` (Codex). The
host-shared `speccy-references/` directory ships under
`.claude/speccy-references/<file>.md` (Claude Code) and
`.agents/speccy-references/<file>.md` (Codex). REQ-002 enumerates
which artifact lives in which class.

<done-when>
- A freshly-init'd Claude Code host pack contains a
  `references/` subdirectory under every skill in REQ-002's
  table at row classification "skill-local", and contains a
  `.claude/speccy-references/` directory containing every file
  in REQ-002's table at row classification "host-shared".
- A freshly-init'd Codex host pack contains the same layout
  under `.agents/skills/<skill>/references/` and
  `.agents/speccy-references/`.
- For every reference file in REQ-002's table, the byte content
  at the Claude Code path equals the byte content at the Codex
  path (byte-identical across hosts).
- For every host-shared reference file in REQ-002's table, the
  byte content at `.claude/speccy-references/<file>.md` equals
  the byte content at `.agents/speccy-references/<file>.md`
  (byte-identical across hosts within the host-shared class).
- For every reference file in REQ-002's table, the byte content
  at the canonical source path under
  `resources/modules/references/<file>.md` equals the byte
  content at each ejected host path (byte-identical from source
  to both host packs). REQ-007's test asserts this invariant.
- The canonical source under `resources/modules/references/`
  exists; `resources/modules/examples/` no longer exists.
- `speccy init --force` against an existing workspace refreshes
  the new directories in place without disturbing user-authored
  skill files (the standard `init --force` semantics
  established by SPEC-0002).
- `speccy init` against a fresh tempdir creates the new
  directories as part of its plan-then-write step (the
  directories appear in the plan summary as `create` entries
  alongside the existing skill / agent file creates).
</done-when>

<behavior>
- Given a fresh tempdir, when `speccy init --host claude-code`
  runs, then `.claude/skills/speccy-plan/references/spec.md`
  exists after the command completes.
- Given the same fresh tempdir post-init, when
  `.claude/speccy-references/evidence.md` is read, then it
  exists and its byte content equals the byte content of
  `.agents/speccy-references/evidence.md` after a parallel
  `speccy init --host codex` in a sibling tempdir.
- Given an existing workspace that pre-dates this SPEC (no
  `references/` directories anywhere in the host packs),
  when `speccy init --force --host claude-code` runs, then
  the new directories appear in the host pack as `create`
  entries in the plan summary and on disk after the write
  step, and user-authored files in `.claude/skills/<skill>/`
  outside the `references/` subdirectory are untouched.
</behavior>

<scenario id="CHK-001">
Given a fresh tempdir where `speccy init --host claude-code` has
run exactly once,
when the directory tree is listed,
then `.claude/skills/speccy-plan/references/spec.md`,
`.claude/skills/speccy-tasks/references/tasks.md`,
`.claude/skills/speccy-ship/references/report.md`,
`.claude/skills/speccy-work/references/journal-implementer.md`,
`.claude/skills/speccy-review/references/journal-review.md`,
`.claude/speccy-references/evidence.md`, and
`.claude/speccy-references/journal-blockers.md` all exist as
regular files.
</scenario>

<scenario id="CHK-002">
Given the same fresh tempdir with a parallel
`speccy init --host codex` run against a sibling tempdir,
when each reference file at a Claude Code path is byte-compared
against its Codex counterpart
(`.claude/skills/speccy-plan/references/spec.md` vs
`.agents/skills/speccy-plan/references/spec.md`, etc.),
then every pair is byte-identical.
</scenario>

<scenario id="CHK-003">
Given the source tree at HEAD after this SPEC's implementation
lands,
when `resources/modules/` is listed,
then `resources/modules/references/` exists and contains the
seven canonical reference files; `resources/modules/examples/`
does not exist as a directory.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Each lintable artifact has exactly one canonical reference file at a documented path

Every lintable Speccy artifact has exactly one reference file
under a documented path. The path's first component
(`<skill>/references/` vs `speccy-references/`) classifies the
file as skill-local (single-consumer) or host-shared
(multi-consumer). The classification is observable from the
shipped path; no separate manifest declares it.

The mapping from artifact to reference file path is:

- a. SPEC.md → `speccy-plan/references/spec.md` (skill-local)
- b. TASKS.md → `speccy-tasks/references/tasks.md` (skill-local)
- c. REPORT.md → `speccy-ship/references/report.md` (skill-local)
- d. journal `<implementer>` block → `speccy-work/references/journal-implementer.md` (skill-local)
- e. journal `<review>` block → `speccy-review/references/journal-review.md` (skill-local)
- f. evidence file (`evidence/T-NNN.md`) → `speccy-references/evidence.md` (host-shared)
- g. journal `<blockers>` block → `speccy-references/journal-blockers.md` (host-shared)

Rows (f) and (g) are host-shared because two skills consume
each: row (f)'s evidence shape is referenced by `/speccy-work`'s
phase body (implementer-side, writes evidence) and by the
`reviewer-tests` sub-agent body (reviewer-side, reads
evidence); row (g)'s blockers shape is referenced by
`/speccy-review`'s skill body (writes a review-induced
blockers element) and `/speccy-amend`'s skill body (writes
an amendment-induced blockers element). All other rows have
exactly one consuming body and live skill-local.

<done-when>
- After this SPEC's implementation lands, each of the seven
  paths above exists in both host packs (per REQ-001's
  cross-host parity).
- No additional reference file ships for any artifact named in
  scope (no `spec-example.md` plus `spec.md`, no
  `journal-implementer-v2.md`, etc.). Exactly one canonical
  file per artifact.
- The architecture doc's "Skill packs" or equivalent section
  in `docs/ARCHITECTURE.md` documents the seven-row mapping
  (or this SPEC's REQ-002 table is the source of truth and
  ARCHITECTURE.md links to it).
- The reference file at row (d) `journal-implementer.md` is
  located under `speccy-work/references/` (not under
  `speccy-review/references/`) because the implementer is the
  canonical writer of an `<implementer>` block. Symmetrically,
  row (e) `journal-review.md` lives under `speccy-review/`
  because the reviewer-orchestrator is its canonical writer.
</done-when>

<behavior>
- Given a freshly-init'd host pack, when the seven paths in
  REQ-002's table are checked for existence, then all seven
  exist as files.
- Given the same host pack, when the `speccy-references/`
  directory at host root is listed, then exactly two files
  appear: `evidence.md` and `journal-blockers.md`.
- Given the same host pack, when each per-skill `references/`
  directory is listed, then each contains exactly the
  skill-local files attributed to that skill in REQ-002's
  table (no additional files, no missing files).
</behavior>

<scenario id="CHK-004">
Given a fresh tempdir post-`speccy init --host claude-code`,
when `.claude/skills/speccy-work/references/` is listed,
then exactly one file is present: `journal-implementer.md`.
</scenario>

<scenario id="CHK-005">
Given the same fresh tempdir,
when `.claude/speccy-references/` is listed,
then exactly two files are present: `evidence.md` and
`journal-blockers.md`.
</scenario>

<scenario id="CHK-006">
Given the same fresh tempdir,
when each of the seven paths in REQ-002's table is checked,
then every path exists as a regular file with non-empty
content.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Reference content reflects post-SPEC-0034 canonical shape

The seven reference files contain canonical worked-instance
content for their artifact, not abstract schema dumps with
`<...>` placeholders. Where SPEC-0034 canonized a post-SPEC-0034
shape that differs from pre-SPEC-0034 prompts, the reference
content uses the post-SPEC-0034 shape exclusively.

The two load-bearing shape decisions canonized by SPEC-0034 are:

- The `<implementer>` six-field handoff template per SPEC-0034
  REQ-001: in order, `Completed`, `Undone`, `Hygiene checks`,
  `Evidence`, `Discovered issues`, `Procedural compliance`.
  The names `Commands run` and `Exit codes` (the pre-SPEC-0034
  field names that the current `phases/speccy-work.md` body
  still teaches at lines 117-119) do not appear as field labels
  inside `journal-implementer.md`. They may appear inside the
  body of the `Hygiene checks` field as natural-language prose
  (since hygiene checks describe commands run and exit codes
  observed); they do not appear as bullet-prefix field labels.
- The evidence file shape per SPEC-0034 REQ-002: a Markdown
  heading `# Evidence for SPEC-NNNN T-NNN` opens the file
  (with NNNN and NNN substituted to a realistic worked-instance
  pair), followed by per-attempt sub-sections containing bare
  `<red>` and `<green>` element blocks (with concrete
  Given/When/Then exit-code semantics matching real runner
  output). The pre-SPEC-0034 `<evidence task="..." spec="...">`
  wrapper element (the shape `resources/modules/examples/evidence.md`
  carries today) does not appear inside `evidence.md`.

The remaining five reference files (`spec.md`, `tasks.md`,
`report.md`, `journal-review.md`, `journal-blockers.md`) reflect
the established post-SPEC-0033 / post-SPEC-0035 / post-SPEC-0037
shapes for their respective artifacts (raw-XML SPEC.md per
SPEC-0019..SPEC-0022; TASKS.md heading constraint per the existing
`phases/speccy-tasks.md` step 2 example; REPORT.md per SPEC-0035's
RPT lint requirements; journal grammar per SPEC-0037's
per-task journal file).

<done-when>
- `journal-implementer.md` contains the literal substrings
  `Completed:`, `Undone:`, `Hygiene checks:`, `Evidence:`,
  `Discovered issues:`, and `Procedural compliance:` in that
  order at the start of bullet lines inside its `<implementer>`
  worked-instance block.
- `journal-implementer.md` does not contain the literal
  substrings `Commands run:` or `Exit codes:` at the start of
  bullet lines inside its `<implementer>` worked-instance block
  (substring-occurrences elsewhere in narrative prose are
  allowed).
- `evidence.md` contains a line matching the regex
  `^# Evidence for SPEC-\d{4} T-\d{3}$` at file offset 0.
- `evidence.md` contains at least one `<red>` ... `</red>`
  block and at least one `<green>` ... `</green>` block.
- `evidence.md` does not contain the literal substring
  `<evidence task=` anywhere in the file (the pre-SPEC-0034
  wrapper element name).
- `spec.md` contains a YAML frontmatter block with `id:`,
  `slug:`, `title:`, `status:`, `created:`; contains the
  literal substring `<requirement id="REQ-` at least once;
  contains `<scenario id="CHK-` at least once; contains
  `<done-when>` and `<behavior>` element openings.
- `tasks.md` contains a YAML frontmatter block with `spec:`,
  `spec_hash_at_generation:`, `generated_at:`; contains a
  `# Tasks: SPEC-` heading; contains the literal substring
  `covers="REQ-001 REQ-002"` (the space-separated multi-REQ
  form) inside at least one `<task>` element.
- `report.md` contains a YAML frontmatter with `spec:`,
  `outcome:`, `generated_at:`; contains `<report spec="SPEC-`
  as a root element opener with the required `spec="..."`
  attribute; contains at least one `<coverage req="REQ-`
  element with `result=` and `scenarios=` attributes.
- `journal-review.md` contains a `<review persona="..."
  verdict="..." model="..." date="..." round="...">`
  element opener with all five required attributes per
  SPEC-0037's journal grammar.
- `journal-blockers.md` contains a `<blockers date="..."
  round="...">` element opener with both required attributes
  per SPEC-0037's journal grammar.
- No reference file contains the literal substring `TBD`,
  `TODO`, or `<...>` (placeholder leakage check parallel to
  the SPEC.md placeholder check).
</done-when>

<behavior>
- Given the shipped `journal-implementer.md` content, when
  read line by line, then the six post-SPEC-0034 field labels
  appear as bullet-line prefixes in order; the two pre-SPEC-0034
  labels do not appear as bullet-line prefixes.
- Given the shipped `evidence.md` content, when the first line
  is read, then it matches the regex
  `^# Evidence for SPEC-\d{4} T-\d{3}$`; when the file body
  is scanned for the substring `<evidence task=`, the substring
  is absent.
- Given the shipped `spec.md` content, when an LLM agent reads
  it on first invocation of `/speccy-plan`, then the agent can
  produce a SPEC.md whose shape passes the existing `SPC-*` /
  `REQ-*` / `QST-*` lints in `speccy verify` on the first
  attempt (the operative property is "the reference is
  sufficient as a shape source").
</behavior>

<scenario id="CHK-007">
Given the shipped `.claude/skills/speccy-work/references/journal-implementer.md`
post-this-SPEC,
when the file is scanned for the literal substrings
`Completed:`, `Undone:`, `Hygiene checks:`, `Evidence:`,
`Discovered issues:`, `Procedural compliance:`,
then all six substrings appear in the file at positions
strictly increasing in offset (the post-SPEC-0034 canonical
order).
</scenario>

<scenario id="CHK-008">
Given the same file,
when scanned for `Commands run:` and `Exit codes:` as
start-of-bullet-line prefixes (i.e. matching the regex
`^- Commands run:` or `^- Exit codes:` after any preceding
whitespace),
then zero matches occur.
</scenario>

<scenario id="CHK-009">
Given the shipped `.claude/speccy-references/evidence.md`
post-this-SPEC,
when the first non-empty line is read,
then it matches `^# Evidence for SPEC-\d{4} T-\d{3}$`.
</scenario>

<scenario id="CHK-010">
Given the same file,
when scanned for the literal substring `<evidence task=`,
then zero matches occur.
</scenario>

<scenario id="CHK-011">
Given each of the seven shipped reference files
post-this-SPEC,
when scanned for the placeholder substrings `TBD`, `TODO`,
or `<...>`,
then zero matches occur across all seven files (the
placeholder-leakage invariant parallels SPEC.md's same
property).
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Consuming bodies carry one-line path pointers and no inline example shape blocks of eight or more lines

Every consuming body — SKILL.md, phase body, sub-agent body —
that references an artifact whose canonical shape ships as a
reference file in REQ-002 contains exactly one line carrying
the reference file's path. The line's prose form is implementer
discretion (e.g., `**Reference:** [\`spec.md\`](references/spec.md)`
or `See \`references/spec.md\` for the canonical SPEC.md shape.`)
provided the line contains the path substring the test in REQ-007
matches. Skill-local pointers use the relative form
`references/<file>.md`; host-shared pointers use the host-rooted
form `.claude/speccy-references/<file>.md` (in a Claude Code
body) or `.agents/speccy-references/<file>.md` (in a Codex
body).

The reverse half of the requirement: no consuming body inlines
an example shape block of eight or more lines for any artifact
in REQ-002. The eight-line threshold is the F-9 backlog
heuristic and matches the ejection threshold the brainstorm
agreed on. Inline shape sketches under eight lines are
permitted (a brief illustrative fragment that helps the prose
flow); the canonical shape lives in the reference file. The
test in REQ-007 enforces the inline-block ceiling.

The consuming bodies in scope are:

- `resources/modules/skills/speccy-plan.md` → pointer to
  `references/spec.md`.
- `resources/modules/phases/speccy-tasks.md` → pointer to
  `references/tasks.md`. Note: the current body already
  carries a ~20-line inline TASKS.md fragment at lines 38-58;
  that fragment shrinks to a pointer line plus any sub-8-line
  sketch the body still wants to carry inline.
- `resources/modules/phases/speccy-ship.md` → pointer to
  `references/report.md`.
- `resources/modules/phases/speccy-work.md` → pointers to
  `references/journal-implementer.md` and to
  `.claude/speccy-references/evidence.md` /
  `.agents/speccy-references/evidence.md` (the dual-host
  form expanded by the template; the body Tera-renders into
  Claude-Code or Codex output, and each rendering carries
  the host-appropriate path). The current body already
  carries a ~14-line inline `<implementer>` block example
  at lines 73-87; that block disappears.
- `resources/modules/skills/speccy-review.md` → pointers to
  `references/journal-review.md` and to
  `.claude/speccy-references/journal-blockers.md` /
  `.agents/speccy-references/journal-blockers.md`.
- `resources/modules/skills/speccy-amend.md` → pointer to
  `.claude/speccy-references/journal-blockers.md` /
  `.agents/speccy-references/journal-blockers.md`.
- `resources/modules/personas/reviewer-tests.md` → pointer to
  `.claude/speccy-references/evidence.md` /
  `.agents/speccy-references/evidence.md`.

Other reviewer personas (`reviewer-business`, `reviewer-security`,
`reviewer-style`, plus the off-default `reviewer-architecture`,
`reviewer-docs`) are audited; a persona gains a pointer if and
only if its existing prose describes a reference-shipping
artifact whose shape it would otherwise re-derive. The audit's
outcome is recorded in `<decision id="DEC-001">` under
`### Decisions`.

<done-when>
- Each consuming body listed above contains exactly one line
  carrying the reference file path string the body needs to
  reach (the path substring REQ-007's test matches). The line
  may be wrapped or formatted any way the body's surrounding
  prose calls for; the operative invariant is "the path
  substring appears exactly once per reference, on its own
  Read-pointer line".
- No consuming body contains an inline example shape block of
  eight or more lines for any artifact in REQ-002. The
  test in REQ-007 enforces this by counting consecutive
  non-blank lines inside fenced code blocks or raw-XML
  shape sketches inside the body.
- The current ~20-line inline TASKS.md fragment in
  `phases/speccy-tasks.md` is replaced by a pointer line plus
  prose; the new body length is shorter than the pre-this-SPEC
  body by approximately the size of the removed inline
  fragment.
- The current ~14-line inline `<implementer>` block in
  `phases/speccy-work.md` is replaced by a pointer line plus
  prose; the new body length is correspondingly shorter.
- Every reviewer persona that points to a reference file has
  exactly one pointer line per referenced file; reviewer
  personas with no reference-shipping artifact in their prose
  carry no pointer.
- The rendered output of `cargo run -- init --force --host
  claude-code` against a tempdir produces consuming bodies in
  `.claude/skills/<skill>/SKILL.md`, `.claude/agents/<agent>.md`,
  and (for Codex) `.agents/skills/<skill>/SKILL.md`,
  `.codex/agents/<agent>.toml` that carry the host-appropriate
  pointer path (relative vs `.claude/` vs `.agents/`) in each
  rendered output.
</done-when>

<behavior>
- Given the rendered Claude Code skill body
  `.claude/skills/speccy-plan/SKILL.md` post-this-SPEC, when
  the body is scanned for the literal substring
  `references/spec.md`, then exactly one match occurs.
- Given the rendered Claude Code phase body
  `.claude/agents/speccy-work.md` post-this-SPEC, when the
  body is scanned for the literal substring
  `references/journal-implementer.md`, then exactly one match
  occurs; when scanned for `.claude/speccy-references/evidence.md`,
  then exactly one match occurs.
- Given the rendered Codex skill body
  `.agents/skills/speccy-work/SKILL.md` post-this-SPEC, when
  scanned for `references/journal-implementer.md`, then
  exactly one match occurs; when scanned for
  `.agents/speccy-references/evidence.md`, then exactly one
  match occurs (the host-templated form, NOT the Claude-Code
  `.claude/...` path).
- Given the rendered Claude Code sub-agent file
  `.claude/agents/reviewer-tests.md` post-this-SPEC, when
  scanned for `.claude/speccy-references/evidence.md`, then
  exactly one match occurs.
- Given the rendered Codex sub-agent file
  `.codex/agents/reviewer-tests.toml` post-this-SPEC, when
  scanned for `.agents/speccy-references/evidence.md`, then
  exactly one match occurs.
- Given any consuming body post-this-SPEC, when the body is
  scanned for inline raw-XML shape sketches or fenced code
  blocks of eight or more consecutive non-blank lines that
  match an artifact-shape pattern (e.g., a `<requirement
  id="REQ-NNN">` opener inside a block longer than seven
  lines), then zero matches occur.
</behavior>

<scenario id="CHK-012">
Given the rendered `.claude/skills/speccy-plan/SKILL.md`
post-this-SPEC,
when the body is grep'd for the literal substring
`references/spec.md`,
then exactly one match occurs and the matching line is a
self-contained pointer line (not embedded inside a fenced
code block, not part of a multi-line sentence wrapping the
pointer in unrelated prose).
</scenario>

<scenario id="CHK-013">
Given the rendered Claude Code phase body
`.claude/agents/speccy-work.md` post-this-SPEC,
when scanned for the substring `Commands run:` or
`Exit codes:` as start-of-bullet-line prefixes inside a
worked example block (matching `^[\s]*Commands run:` or
`^[\s]*Exit codes:` inside lines indented as part of an
`<implementer>` block),
then zero matches occur.
</scenario>

<scenario id="CHK-014">
Given the rendered Codex sub-agent file
`.codex/agents/reviewer-tests.toml` post-this-SPEC,
when scanned for `.agents/speccy-references/evidence.md`,
then exactly one match occurs and no parallel match for
`.claude/speccy-references/evidence.md` occurs (the host-templated
path renders as the Codex form, not the Claude Code form).
</scenario>

<scenario id="CHK-015">
Given the rendered `.claude/agents/speccy-tasks.md`
post-this-SPEC,
when the body is measured for length compared to its
pre-SPEC length at HEAD,
then the post-SPEC body is shorter by at least eight lines
(the inline TASKS.md fragment at the current lines 38-58
shrank to a pointer line).
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Orphan files in `resources/modules/` no longer exist; salvageable prose moves into successor locations

The following paths in the source tree no longer exist after this
SPEC's diff lands:

- `resources/modules/personas/implementer.md` (orphan persona; not
  Tera-included by any production wrapper).
- `resources/modules/personas/planner.md` (orphan persona; not
  Tera-included by any production wrapper).
- `resources/modules/examples/evidence.md` (orphan example; not
  Read-pointed by any consuming body).
- `resources/modules/examples/` (the directory; now empty after
  evidence.md's removal, replaced semantically by
  `resources/modules/references/` per REQ-001).

Salvageable prose from each deleted file moves into a documented
successor location. The criterion for "salvageable" is: the prose
adds material that is not already present in the successor body.
Boilerplate, role descriptions duplicating the phase body, or
example blocks superseded by the new reference file content do
not count as salvageable; they are dropped without forwarding.
The successor locations are:

- `phases/speccy-work.md` receives any salvageable non-duplicate
  prose from the deleted `personas/implementer.md`. Candidate
  paragraphs: the "What to consider" bullets that name
  `<done-when>` / `<behavior>` re-reading, the
  suggested-files-hint-may-be-stale warning, the
  feature-flag/abstraction-layer guardrail. The example block
  at lines 76-95 of the deleted persona is NOT salvaged here
  (it is the post-SPEC-0034 reshape that becomes
  `references/journal-implementer.md` per REQ-003).
- `skills/speccy-plan.md` receives any salvageable non-duplicate
  prose from the deleted `personas/planner.md`. Candidate
  paragraphs: the "bounded scope; one SPEC answers one product
  question; refuse to bundle" guardrail, the "decisions hidden
  inside requirement prose belong in `### Decisions`" guidance.
  The anecdotal "email signup" example at lines 58-64 of the
  deleted persona is NOT salvaged (anecdote, not artifact
  shape).
- The deleted `examples/evidence.md`'s content moves into
  `resources/modules/references/evidence.md` reshaped to the
  post-SPEC-0034 header-convention shape per REQ-003. The
  pre-SPEC-0034 `<evidence task="..." spec="...">` wrapper
  the deleted file used does not appear in the new file; the
  inner `<red>` / `<green>` session content's spirit (the
  fabrication-pattern-resistant structure) is preserved.

The test references in `speccy-cli/tests/skill_body_discovery.rs:87-88`
(`PERSONA_FILES` const containing `"personas/implementer.md"` and
`"personas/planner.md"`) are removed from the const, since the files
no longer exist.

<done-when>
- The four paths listed above no longer exist in the source
  tree after this SPEC's diff lands.
- `resources/modules/references/evidence.md` exists and
  contains content that originated in the deleted
  `examples/evidence.md` reshaped to post-SPEC-0034 form per
  REQ-003.
- `phases/speccy-work.md` contains at least one paragraph or
  bullet point that originated as a salvaged "What to
  consider" item from the deleted `personas/implementer.md`
  (the spec writer's discretion which paragraphs qualify;
  the operative invariant is "the salvage destination exists
  and carries non-zero salvaged content").
- `skills/speccy-plan.md` similarly contains at least one
  salvaged bullet or paragraph from the deleted
  `personas/planner.md`.
- The `PERSONA_FILES` const in `skill_body_discovery.rs` no
  longer lists `personas/implementer.md` or
  `personas/planner.md`.
- The four standard-hygiene commands in AGENTS.md pass after
  this SPEC's diff (no test that previously depended on the
  deleted persona files fails as a result of their removal).
</done-when>

<behavior>
- Given the source tree at HEAD after this SPEC's
  implementation lands, when each of the four orphan paths
  is checked for existence, then none exist.
- Given the source tree at the same HEAD, when
  `resources/modules/references/evidence.md` is read, then
  it exists and its first non-empty line matches
  `^# Evidence for SPEC-\d{4} T-\d{3}$` per REQ-003.
- Given the source tree at the same HEAD, when
  `phases/speccy-work.md` is grep'd for the literal substring
  of one chosen salvaged bullet from the deleted
  `personas/implementer.md` (e.g., the
  feature-flag/abstraction-layer guardrail bullet), then at
  least one match occurs.
- Given the same HEAD, when `cargo test --workspace` runs,
  then `skill_body_discovery.rs` does not fail due to a
  missing `personas/implementer.md` or `personas/planner.md`
  module load (the const that listed them has been updated).
</behavior>

<scenario id="CHK-016">
Given the source tree at HEAD after this SPEC's
implementation lands,
when `resources/modules/personas/implementer.md`,
`resources/modules/personas/planner.md`,
`resources/modules/examples/evidence.md`, and the directory
`resources/modules/examples/` are checked for existence,
then none of the four paths exist.
</scenario>

<scenario id="CHK-017">
Given the same HEAD,
when `resources/modules/references/evidence.md` is read,
then the file matches REQ-003's post-SPEC-0034 evidence-shape
invariants: the first non-empty line matches
`^# Evidence for SPEC-\d{4} T-\d{3}$` and the file contains no
`<evidence task=` substring anywhere.
</scenario>

<scenario id="CHK-018">
Given the same HEAD,
when `speccy-cli/tests/skill_body_discovery.rs` is opened,
then the `PERSONA_FILES` constant does not contain
`"personas/implementer.md"` or `"personas/planner.md"`.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: Reviewer-tests persona gains its cross-skill pointer to `speccy-references/evidence.md`

The shipped `resources/modules/personas/reviewer-tests.md` body
currently contains an "Evidence loading" section
(lines 42-95 today) that describes the evidence-file shape and
fabrication patterns in prose. After this SPEC's diff lands,
that section gains a one-line pointer to the host-shared
`speccy-references/evidence.md` reference file (host-templated
form: `.claude/speccy-references/evidence.md` in the Claude Code
rendered output, `.agents/speccy-references/evidence.md` in the
Codex rendered output) using the same pointer-line convention
REQ-004 establishes. The fabrication-pattern bullets and the
four-step "walk these four steps before forming a verdict"
procedure stay; only the pointer-line addition is in scope.

This requirement is separate from REQ-004 because the persona
file is reviewer-fan-out content, not a SKILL.md or phase body.
The reviewer persona's role is to apply fabrication-pattern
heuristics; the reference file is its known-good baseline. The
pointer's presence is the load-bearing signal that the persona
has the baseline to compare against; without it, the
fabrication-pattern check is reasoning-from-prose-description
rather than reasoning-from-reference.

Other reviewer personas (`reviewer-business.md`,
`reviewer-security.md`, `reviewer-style.md`, plus the
off-default `reviewer-architecture.md`, `reviewer-docs.md`) are
audited as part of this requirement. A persona gains a pointer
if and only if its existing prose describes an artifact-shape
the persona would otherwise re-derive. The audit's outcome
records in `### Decisions` as `DEC-001`. Initial expectation:
only `reviewer-tests.md` gains a pointer; the other personas'
prose operates on the diff and SPEC.md content (not on a
reference-shipping artifact shape), so they do not need
pointers.

<done-when>
- `resources/modules/personas/reviewer-tests.md` contains a
  one-line pointer to `speccy-references/evidence.md` (the
  host-rooted form; the renderer expands the Claude Code or
  Codex prefix at template time).
- The "Evidence loading" section's four-step procedure and
  fabrication-pattern bullets are preserved; only the
  pointer-line addition is the diff in this persona file.
- The rendered Claude Code sub-agent
  `.claude/agents/reviewer-tests.md` carries the
  `.claude/speccy-references/evidence.md` form of the pointer
  exactly once.
- The rendered Codex sub-agent
  `.codex/agents/reviewer-tests.toml` carries the
  `.agents/speccy-references/evidence.md` form of the pointer
  exactly once (inside whatever TOML instruction-string field
  the persona body templates into).
- `### Decisions / DEC-001` records the audit outcome: which
  reviewer personas (besides `reviewer-tests`) gained pointers
  if any, and the reasoning for the personas that did not.
</done-when>

<behavior>
- Given the rendered `.claude/agents/reviewer-tests.md`
  post-this-SPEC, when grep'd for
  `.claude/speccy-references/evidence.md`, then exactly one
  match occurs.
- Given the rendered `.codex/agents/reviewer-tests.toml`
  post-this-SPEC, when grep'd for
  `.agents/speccy-references/evidence.md`, then exactly one
  match occurs.
- Given the same Codex `.toml` file, when grep'd for
  `.claude/speccy-references/evidence.md` (the Claude Code
  form), then zero matches occur.
- Given the other reviewer persona files
  (`reviewer-business.md`, `reviewer-security.md`,
  `reviewer-style.md`, `reviewer-architecture.md`,
  `reviewer-docs.md`), when grep'd for any pointer to a
  `references/*.md` or `speccy-references/*.md` path, then
  the count matches what DEC-001 declares for each persona
  (typically zero, per the audit's initial expectation).
</behavior>

<scenario id="CHK-019">
Given the rendered `.claude/agents/reviewer-tests.md`
post-this-SPEC,
when the body is scanned for the literal substring
`.claude/speccy-references/evidence.md`,
then exactly one match occurs.
</scenario>

<scenario id="CHK-020">
Given the rendered `.codex/agents/reviewer-tests.toml`
post-this-SPEC,
when the body is scanned for the literal substring
`.agents/speccy-references/evidence.md`,
then exactly one match occurs and no match for the Claude
Code form (`.claude/speccy-references/evidence.md`) occurs.
</scenario>

<scenario id="CHK-021">
Given DEC-001's recorded audit outcome and the rendered
sub-agent files for each non-tests reviewer persona
post-this-SPEC,
when each sub-agent file is scanned for any pointer to a
`references/` or `speccy-references/` path,
then the count matches the count DEC-001 records for that
persona (i.e., DEC-001 is the source of truth and the
implementation matches it).
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: `skill_body_discovery.rs` gains a `chk0NN_no_orphan_references` test asserting every shipped reference file is reached by at least one pointer in a consuming body

A new test function in `speccy-cli/tests/skill_body_discovery.rs`
asserts that every reference file ejected by `speccy init` is
reached by at least one path-substring pointer from a consuming
body inside the same host pack. The test's name follows the
existing `chk0NN_*` naming convention used by CHK-014 through
CHK-019 today; the SPEC does not commit to a specific NNN, only
to the convention.

The test enumerates ejected reference files by globbing the
fresh-init'd host pack tree:

- Claude Code: `.claude/skills/*/references/*.md` ∪
  `.claude/speccy-references/*.md`.
- Codex: `.agents/skills/*/references/*.md` ∪
  `.agents/speccy-references/*.md`.

For each ejected reference file, the test computes the set of
"path substrings" that a consuming body might use to point to
it. The substring set per file is exactly one string for
skill-local files (`references/<file>.md`) and exactly one
host-specific string per host for shared files
(`.claude/speccy-references/<file>.md` for the Claude Code
host pack, `.agents/speccy-references/<file>.md` for the
Codex host pack).

The test then scans every consuming body in the same host pack
for the path substring. Consuming bodies are:

- Claude Code: every `.md` file under `.claude/skills/*/` (the
  SKILL.md and any sibling references the host pack carries),
  plus every `.md` file under `.claude/agents/*.md` (the
  sub-agent files).
- Codex: every `.md` file under `.agents/skills/*/SKILL.md`
  plus every `.toml` file under `.codex/agents/*.toml`. The
  TOML scan reads the instruction-string fields the persona
  body templates into; the test grep-scans the raw TOML file
  content for the path substring (the TOML parser is not
  required — substring presence is sufficient).

For each ejected reference file, the test fails the assertion
if zero consuming bodies in the same host pack contain the
file's path substring. The failure message names the orphan
reference file and the host pack it shipped in.

The test additionally asserts two byte-identical invariants:

- **Cross-host parity** (REQ-001): for every reference file,
  the byte content at the Claude Code path equals the byte
  content at the Codex path.
- **Source-to-host parity**: for every reference file, the
  byte content at the canonical source path under
  `resources/modules/references/<file>.md` equals the byte
  content at each host's ejected path. This catches a
  templating-engine bug where one host pack drifts from the
  source. Both per-skill and host-shared files participate;
  the source-to-host check fires for every shipped reference
  file in REQ-002's table.

The failure message names the diverging path triple (or pair
for the cross-host check) and the byte offset of the first
difference.

<done-when>
- `speccy-cli/tests/skill_body_discovery.rs` contains a new
  test function named per the `chk0NN_*` convention (the
  specific NNN is implementer-allocated to the next free
  number).
- The test runs as part of `cargo test --workspace` and
  passes against the workspace post-this-SPEC.
- If the test is run against an artificial workspace where
  one reference file has been planted with no consuming
  pointer (a fault injection), the test fails with a
  message naming the orphan file.
- If the test is run against an artificial workspace where
  one reference file's content diverges between
  `.claude/skills/X/references/Y.md` and
  `.agents/skills/X/references/Y.md`, the test fails with a
  message naming the diverging pair.
- If the test is run against an artificial workspace where
  one host pack's reference file diverges from the source at
  `resources/modules/references/<file>.md` (e.g., a
  templating-engine bug rewrote the Claude Code copy but
  left Codex matching the source), the test fails with a
  message naming the diverging source-to-host pair.
- The test scans `.toml` sub-agent files (not just `.md`) so
  the Codex sub-agent pointers in REQ-006 register as
  consumers.
- The test enumerates reference files via glob, not via a
  hard-coded list, so a future SPEC adding an eighth
  reference file does not need to update the test.
</done-when>

<behavior>
- Given the workspace post-this-SPEC, when `cargo test
  --workspace -- chk0NN_no_orphan_references` runs, then
  the test passes (every reference file ejected by
  `speccy init` is reached by at least one pointer).
- Given the workspace at this same HEAD with one reference
  file artificially planted at
  `.claude/skills/speccy-plan/references/orphan.md` and no
  consuming body updated, when the test runs, then it
  fails with a message naming `orphan.md` and the host
  pack `claude-code`.
- Given the workspace at this same HEAD with one reference
  file's byte content artificially diverged between Claude
  Code and Codex paths, when the test runs, then it fails
  with a message naming the diverging path pair.
- Given the workspace at this same HEAD with one host pack's
  reference file artificially diverged from the canonical
  source at `resources/modules/references/<file>.md`, when
  the test runs, then it fails with a message naming the
  source-to-host divergence and the affected host pack.
- Given the workspace at this same HEAD with a Codex
  sub-agent's `.toml` body grep-matching the host-rooted
  pointer to its reference file, when the test runs, then
  the corresponding reference file's "reachable consumers"
  set includes the `.toml` file (`.toml` scanning is
  enabled, not just `.md`).
</behavior>

<scenario id="CHK-022">
Given the workspace post-this-SPEC,
when `cargo test --workspace -- chk0NN_no_orphan_references`
runs to completion,
then the test exits with status pass.
</scenario>

<scenario id="CHK-023">
Given the same workspace with one extra file placed at
`.claude/skills/speccy-plan/references/orphan.md` containing
arbitrary Markdown and no consuming body updated to point at
it,
when the test runs,
then it fails with a message that contains the substring
`orphan.md`.
</scenario>

<scenario id="CHK-024">
Given the same workspace where `.claude/skills/speccy-plan/references/spec.md`
has been altered to differ in one byte from
`.agents/skills/speccy-plan/references/spec.md`,
when the test runs,
then it fails with a message that names both paths.
</scenario>

<scenario id="CHK-025">
Given the same workspace where the `reviewer-tests` Codex
sub-agent `.toml` has its `.agents/speccy-references/evidence.md`
pointer line removed,
when the test runs,
then it fails with a message naming `evidence.md` and the
Codex host pack (the test reads `.toml` files; removing the
pointer from the `.md` Claude Code version is not enough to
avoid the assertion).
</scenario>

<scenario id="CHK-026">
Given the same workspace where one host pack's reference
file content has been artificially altered so it no longer
matches the canonical source at
`resources/modules/references/<file>.md`,
when the test runs,
then it fails with a message identifying the source-to-host
divergence (which host pack drifted, which reference file,
and the byte offset of the first difference).
</scenario>

</requirement>

## Open Questions

(All open questions surfaced during the brainstorm and
self-review have been resolved or promoted to `### Decisions`.
This section is intentionally empty; future amendments will
append questions here as they surface.)

## Design

### Decisions

<decision id="DEC-001">
**Reviewer persona audit: only `reviewer-tests` gains a pointer in this SPEC.**

REQ-006 requires the audit; this decision records the outcome.
Each non-tests reviewer persona's prose was scanned for any
description of an artifact-shape the persona would otherwise
re-derive. The findings:

- `reviewer-business.md`: prose describes mapping
  `<done-when>` to the diff. No reference-shipping artifact
  shape is re-derived; the persona operates on SPEC.md and
  the diff as they exist. No pointer added.
- `reviewer-security.md`: prose describes security-relevant
  patterns (auth boundaries, input validation, secrets).
  No reference-shipping artifact shape is re-derived; the
  persona operates on the diff. No pointer added.
- `reviewer-style.md`: prose describes style conventions
  per AGENTS.md and the project's lints. No
  reference-shipping artifact shape is re-derived. No
  pointer added.
- `reviewer-architecture.md` (off-default): prose describes
  cross-spec architectural concerns. No
  reference-shipping artifact shape is re-derived. No
  pointer added.
- `reviewer-docs.md` (off-default): prose describes
  documentation concerns including comments, READMEs, and
  AGENTS.md updates. No reference-shipping artifact shape
  is re-derived. No pointer added.
- `reviewer-tests.md`: prose describes the evidence file
  shape (the `Evidence loading` section). The persona
  would otherwise re-derive the post-SPEC-0034 shape from
  natural-language description. **Pointer added** to
  `speccy-references/evidence.md` per REQ-006.

The audit outcome can change in a future SPEC if a reviewer
persona gains prose that re-derives a reference-shipping
artifact's shape (e.g., a hypothetical `reviewer-business`
extension that re-derives the `<requirement>` shape would
warrant a `references/spec.md` pointer). The test in REQ-007
catches the inverse failure (a reference file with no
consumer); it does not catch the "consumer's prose
re-derives a shape the reference would have supplied"
failure, which stays a review judgement.
</decision>

<decision id="DEC-002">
**Host-shared references live at `.claude/speccy-references/` and `.agents/speccy-references/` rather than at `.speccy/examples/` or `.speccy/references/`.**

The brainstorm considered three locations for cross-skill
reference content:

- Per-skill duplication: ship `evidence.md` in both
  `speccy-work/references/` and `speccy-review/references/`
  (and similarly for `journal-blockers.md`). Rejected:
  duplication multiplies maintenance and risks drift if one
  copy is updated without the other.
- Host-agnostic `.speccy/examples/` or `.speccy/references/`:
  one canonical copy reachable from both host packs by
  `.speccy/...` path. Rejected: `.speccy/` is reserved for
  project state (specs, missions, evidence, journals) per
  the architecture's namespace convention; host-specific
  ejected content belongs in host-namespaced folders.
- Host-shared `.claude/speccy-references/` and
  `.agents/speccy-references/`: one canonical source under
  `resources/modules/references/`, templated into a
  host-namespaced shared folder at host root. **Selected.**
  Each host pack is self-contained; the cross-host
  byte-identical invariant in REQ-001 enforces parity.

The Codex placement deserves a separate note: Codex sub-agents
live at `.codex/agents/*.toml`, not under `.agents/`. The
brainstorm initially proposed `.codex/speccy-references/` to
co-locate the shared folder with the sub-agents (its
load-bearing readers). The decision moved to
`.agents/speccy-references/` because `.agents/` is the more
widely-accepted standard for agent skill packs, and the
sub-agent's TOML pointer line carrying the absolute
`.agents/...` path resolves cleanly (the harness reads from
repo root, not from `.codex/` cwd).
</decision>

<decision id="DEC-003">
**Reference filenames are lowercase plain (no suffix, no prefix).**

The brainstorm considered three naming conventions:

- `<ARTIFACT>-REFERENCE.md` (uppercase, suffixed): the form
  the early preview used. Reads as "the reference for this
  artifact." Verbose; "REFERENCE" inside a `references/`
  directory is redundant.
- `EXAMPLE-<ARTIFACT>.md` (uppercase, prefixed): reads as "a
  worked instance." Similarly verbose; "EXAMPLE" inside a
  `references/` directory is redundant.
- `<artifact>.md` (lowercase plain): the skill-creator
  anatomy's convention. The examples it ships use this form
  (`references/aws.md`, `references/schemas.md`). Inside a
  `references/` namespace the role is self-evident from the
  directory name; the filename names the artifact, nothing
  more. **Selected.**

The names committed in REQ-002's table are `spec.md`,
`tasks.md`, `report.md`, `evidence.md`,
`journal-implementer.md`, `journal-review.md`,
`journal-blockers.md`. Hyphenated lowercase for the journal
sub-types; bare lowercase for the artifact whose name is
already one token.
</decision>

<decision id="DEC-004">
**`speccy init`'s flag surface stays at `--host` and `--force`; no `--quiet-references` flag is added.**

This SPEC adds roughly seven `create` entries to the init plan
summary per `--host` invocation (one per reference file ejected
into the targeted host pack). The brainstorm and review
considered adding a `--quiet-references` flag that would
collapse those per-file lines into a single summary line ("create:
7 reference files"). The decision is to leave the plan summary
verbose and not grow the flag surface.

Reason: `speccy init` is a one-time bootstrap (or rare
`--force` refresh). The noise lives in a place a contributor
sees once and never again on a given workspace. Adding a flag
for a cosmetic, one-time concern violates Principle 5 ("stay
small, no orchestration runtime") and walks back the
seven-command surface guarantee the architecture commits to.
The two existing flags (`--host {claude-code|codex}` and
`--force`) cover the load-bearing init choices; a third flag
for plan-summary verbosity does not earn its keep.

Trade-off acknowledged: the plan summary is longer than it was
pre-this-SPEC. Mitigation: the new entries follow the same
`create:` line format as the existing entries, so a contributor
scanning the summary processes them at the same rate.
Promotable to a follow-up SPEC only if dogfooding surfaces the
verbosity as concrete friction (e.g., breaks an automated
post-init verification step) — speculative cosmetic concerns do
not promote.
</decision>

## Notes

**DEC-001 from SPEC-0034 stays in force; reconsidering it is a
follow-up SPEC, not this one.**

SPEC-0034's DEC-001 records the decision to keep three
independent copies of the self-review mechanical/semantic split
prose across `speccy-plan.md`, `speccy-amend.md`, and
`speccy-brainstorm.md`. The reason DEC-001 gave was
divergence-allowance during dogfooding: the three surfaces' check
property counts (6 / 8 / 4) and their mechanical-pattern lists
differ enough that a single shared partial would either be
under-fitting for one surface or over-fitting for another.

This SPEC establishes per-skill `references/` and host-shared
`speccy-references/` as the canonical disclosure mechanism. The
mechanical cost of sharing prose drops materially under the new
pattern: a hypothetical `references/self-review-properties.md`
partial referenced via one-line pointers from each of the three
skills would replace the three near-duplicate copies. DEC-001's
divergence-allowance argument still applies — the three surfaces
really do diverge in property count and pattern coverage. But
the cost-side calculus has changed; the divergence cost is
roughly the same, while the duplication cost dropped.

This SPEC does not reopen DEC-001. The follow-up signal is:
if and when the self-review prose stabilises across plan /
amend / brainstorm (i.e., a change to one surface lands without
needing a parallel change in the other two for two or more
consecutive amendments), a follow-up SPEC can revisit DEC-001
and adopt the shared partial. The test in REQ-007 generalises
cleanly to procedure references too — its substring-match
mechanism does not care whether the reference is artifact shape
or procedure prose.

**Alternative framings considered and rejected during the
brainstorm.**

- *Pure per-skill `references/`, no host-shared folder.* Rejected:
  cross-skill content would require either per-skill duplication
  (drift risk) or `../skills/X/references/...` relative traversal
  (path-fragility). Host-shared `speccy-references/` co-located
  with each host pack avoids both pathologies. See REQ-001 and
  DEC-002.
- *Host-agnostic `.speccy/examples/` for shared content.* The F-9
  backlog text literally proposed this. Rejected: keeps `.speccy/`
  purely about project state; host-specific ejected content
  belongs in host-namespaced folders. See DEC-002.
- *Filenames as `<ARTIFACT>-REFERENCE.md` or `EXAMPLE-<ARTIFACT>.md`.*
  Used in the brainstorm's preview as an illustration. Rejected:
  the skill-creator anatomy's lowercase-plain convention is
  cleaner. See DEC-003.
- *Adding an `SKL-NNN` lint family to `speccy verify`.* Rejected
  as Principle 5 violation (CLI polices spec proof shape, not
  skill-pack content). Test surface owns this. See REQ-007
  location and the goals → non-goals split.
- *Inline example shape first, eject to `references/` later.*
  Rejected: skill-creator anatomy treats `references/` as the
  default disclosure mechanism, not a downstream optimisation;
  inlining-then-ejecting doubles editing work for no gain.
- *Pattern-proof first (ship one reference file end-to-end,
  defer the rest).* Rejected: scope locked at full coverage in
  this SPEC.

**Why this SPEC ships full coverage in one diff rather than
slicing per reference file.**

The seven reference files share three load-bearing properties:
(1) the location-class layout (per-skill vs host-shared), (2)
the post-SPEC-0034 content-shape invariants where applicable,
(3) the pointer-from-consuming-body discipline. Slicing per
reference file would force three or four duplicate SPECs each
re-deriving the same layout/shape/pointer decisions. Bundling
into one SPEC pays the bigger diff once and lets the test in
REQ-007 cover all seven uniformly from day one. The brainstorm
explicitly evaluated the pattern-proof-first alternative and
the user rejected it in favour of full coverage.

## Changelog

<changelog>
| Date       | Reason                                                   | Author     |
|------------|----------------------------------------------------------|------------|
| 2026-05-21 | Initial draft. Ship a per-skill `references/` directory pattern plus a host-shared `speccy-references/` directory pattern, each ejected by `speccy init` into both Claude Code and Codex host packs, carrying canonical worked-instance reference files for every lintable Speccy artifact (SPEC.md, TASKS.md, REPORT.md, evidence file, journal `<implementer>` / `<review>` / `<blockers>` blocks). Reshape stale content to post-SPEC-0034 canonical field names. Clear three orphan files under `resources/modules/personas/` and `resources/modules/examples/`. Add a `chk0NN_no_orphan_references` test that asserts every shipped reference file has at least one consuming pointer and that cross-host byte-identical parity holds. Motivated by F-12 and F-9 in `.speccy/BACKLOG.md` and by the skill-creator anatomy's level-3 progressive-disclosure pattern, which speccy skills today skip entirely. | Kevin Xiao |
| 2026-05-22 | CHK-017 amendment. Dropped the "verifiable via git history" lineage clause from CHK-017. The clause required the new `references/evidence.md` to provably descend from the deleted `examples/evidence.md` via git history, which is not mechanically testable and adds no value beyond REQ-003's shape invariants (header regex match + no `<evidence task=` wrapper substring). The post-SPEC-0034 reshape rewrites the wrapper and re-anchors the heading; whatever lineage survives is incidental. CHK-017 now asserts only the testable shape invariants; the implementer remains free to start from the old file's content or write fresh. | Kevin Xiao |
</changelog>
