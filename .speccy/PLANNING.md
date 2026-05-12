# Speccy Bootstrap Planning

> Outlines for SPEC-0002 through SPEC-0013. SPEC-0001
> (`.speccy/specs/0001-artifact-parsers/`) is already deepened; the
> rest are sketched at REQ/CHK/Task granularity so the breakdown can
> be reviewed end-to-end before any single spec is filled in.
>
> Each outline below collapses to a future
> `.speccy/specs/NNNN-slug/{SPEC.md, spec.toml, TASKS.md}` triple.
> Numbering and dependencies are load-bearing; everything else can be
> rephrased without disturbing downstream specs.

---

## Spec summary

| #    | Slug              | One-line scope                                                                            | Depends on      | Status   |
|------|-------------------|-------------------------------------------------------------------------------------------|-----------------|----------|
| 0001 | artifact-parsers  | `speccy-core` library: parse all five artifact files + cross-ref + supersession index     | --              | Deepened |
| 0002 | init-command      | `speccy init` scaffold + host detection + skill-pack copy                                 | 0001            | Deepened |
| 0003 | lint-engine       | All SPC / REQ / VAL / TSK / QST lint codes as structured diagnostics                      | 0001            | Deepened |
| 0004 | status-command    | `speccy status` text + `--json` (workspace scan, stale, supersession)                     | 0001, 0003      | Deepened |
| 0005 | plan-command      | `speccy plan [SPEC-ID]` greenfield + amendment prompt rendering                           | 0001            | Deepened |
| 0006 | tasks-command     | `speccy tasks SPEC-ID [--commit]` decomposition + amend + hash record                     | 0001, 0005      | Deepened |
| 0007 | next-command      | `speccy next [--kind]` priority + JSON contract                                           | 0001, 0004      | Deepened |
| 0008 | implement-command | `speccy implement TASK-ID` Phase 3 implementer prompt                                     | 0001, 0004, 0005 | Deepened |
| 0009 | review-command    | `speccy review TASK-ID --persona` Phase 4 prompt + persona-file lookup                    | 0001, 0005, 0008 | Deepened |
| 0010 | check-command     | `speccy check [CHK-ID]` shell execution + manual prompts + no-op detection                | 0001, 0004      | Deepened |
| 0011 | report-command    | `speccy report SPEC-ID` Phase 5 prompt rendering                                          | 0001, 0004, 0005 | Deepened |
| 0012 | verify-command    | `speccy verify` CI gate (lint + check; binary exit + `--json`)                            | 0003, 0010      | Deepened |
| 0013 | skill-packs       | `claude-code/` + `codex/` + `shared/personas/` + `shared/prompts/`                        | 0002, 0005-0011 | Deepened |

The `Status` column is the resumability marker. "Outlined" means
the entry exists in this file but no
`.speccy/specs/NNNN-slug/` directory yet. "Deepened" means the
directory exists with `SPEC.md`, `spec.toml`, and `TASKS.md`. Update
this column as each spec is deepened so a future session can pick up
without re-reading the whole doc.

---

## Dependency graph

```text
0001 artifact-parsers    [deepened: SPEC.md + spec.toml + TASKS.md]
  |
  +-- 0002 init-command
  |
  +-- 0003 lint-engine
  |     |
  |     +-- 0004 status-command
  |     +-- 0012 verify-command
  |
  +-- 0005 plan-command
  +-- 0006 tasks-command
  +-- 0007 next-command
  +-- 0008 implement-command
  +-- 0009 review-command
  +-- 0010 check-command
  |     |
  |     +-- 0012 verify-command
  |
  +-- 0011 report-command
  |
  +-- 0013 skill-packs (depends on 0002 for copy mechanism,
                        0005-0011 for the prompts those commands load)
```

Suggested landing order (linear, but several can parallelise after
0001): **0001 -> 0002 -> 0003 -> 0004 -> 0010 -> 0012 -> 0005 ->
0006 -> 0007 -> 0008 -> 0009 -> 0011 -> 0013**. Landing 0012 (verify)
mid-stream gives us a CI gate before the prompt-rendering specs land,
which means each later spec can ship behind a green CI.

---

## SPEC-0002: init-command

**Slug:** `init-command`
**Depends on:** SPEC-0001
**Surface:** `speccy init [--host <name>] [--force]`

**Scope.** Scaffold a fresh `.speccy/` workspace and copy the
appropriate host skill pack into the host-native location. Detect
the host from environment; allow `--host` override; refuse to
overwrite an existing `.speccy/` unless `--force` is passed.

**Requirements:**
- REQ-001: Scaffold `.speccy/speccy.toml` (schema_version=1, project
  block) and `.speccy/VISION.md` (template stub) when neither
  exists.
- REQ-002: Refuse to run if `.speccy/` already exists, unless
  `--force` is passed. Always print the list of files that would be
  created or overwritten before mutating.
- REQ-003: Host detection precedence: `--host <name>` flag wins;
  otherwise probe for `.claude/`, `.codex/`, `.cursor/` in that
  order; fall back to `claude-code` with a warning.
- REQ-004: Copy the embedded `skills/<host>/*` bundle into the
  host-native location (`.claude/commands/` for Claude Code,
  `.codex/skills/` for Codex, etc.). Preserve file mode where
  applicable.
- REQ-005: Exit codes -- 0 on success, 1 on user error (existing
  workspace without `--force`, unknown `--host` value), 2 on
  internal failure (I/O error).

**Checks (sketch):** integration tests scaffold in a tmpdir,
assert file tree, verify `--force` overwrites, verify host
detection precedence, verify exit codes.

**Tasks (sketch):** scaffold-writer module; host-detector module;
skill-pack copier; embedded-resource bundling via `include_dir!` or
`rust-embed`; integration test harness with tmpdir fixtures.

---

## SPEC-0003: lint-engine

**Slug:** `lint-engine`
**Depends on:** SPEC-0001
**Surface:** library-only; no new CLI command. Consumed by
SPEC-0004 (`status`) and SPEC-0012 (`verify`).

**Scope.** Take parsed artifacts from `speccy-core::parse` and emit
structured lint findings with stable codes (SPC-/REQ-/VAL-/TSK-/
QST-/JSON-). Pure: no I/O, no exit-code policy. All semantic
judgement stays in review.

**Requirements:**
- REQ-001: Emit SPC-001..SPC-007 against `SpecToml`, `SpecMd`, and
  the supersession index (SPC-006 specifically uses
  `supersession_index.superseded_by`).
- REQ-002: Emit REQ-001..REQ-002 against the requirement <-> check
  graph (missing covers, dangling check refs).
- REQ-003: Emit VAL-001..VAL-004 against check definitions; VAL-004
  matches a closed set of known no-op commands (`true`, `:`,
  `exit 0`, `cmd /c exit 0`, and obvious variants).
- REQ-004: Emit TSK-001..TSK-004 against `TasksMd`.
- REQ-005: Emit QST-001 (soft signal, info-level) for unchecked
  open questions.
- REQ-006: Public `Diagnostic { code, level, message, file, line }`
  type; stable codes; `level` in `{error, warn, info}`. Adding
  codes is non-breaking; changing a code's meaning is.

**Checks (sketch):** per-code unit test with a fixture that
triggers exactly that code and no others; full-corpus snapshot
test.

**Tasks (sketch):** rule-per-code modules under `lint::rules`; a
single `lint::run` entry point taking a parsed spec bundle; fixture
corpus; severity table; snapshot tests.

---

## SPEC-0004: status-command

**Slug:** `status-command`
**Depends on:** SPEC-0001, SPEC-0003
**Surface:** `speccy status [--json]`

**Scope.** Scan `.speccy/specs/`, parse each spec, run lint,
compute stale flags, compute `superseded_by`, aggregate task state
counts, render text + `--json`. Text view defaults to in-progress
specs plus any with errors regardless of status; `--json` always
shows all specs.

**Requirements:**
- REQ-001: Workspace scan -- discover every `NNNN-slug/` directory
  under `.speccy/specs/` and parse its artifact set.
- REQ-002: Stale detection -- compare TASKS.md
  `spec_hash_at_generation` against the computed SPEC.md sha256
  AND compare mtimes. Emit `stale` with `stale_reasons`.
- REQ-003: Task state aggregation -- counts of `[ ]` / `[~]` /
  `[?]` / `[x]` per spec.
- REQ-004: Inverse supersession -- call `supersession_index` over
  all parsed specs and surface `superseded_by` per spec in the JSON
  output.
- REQ-005: Text view -- filtered to `status: in-progress` plus
  anything with errors. Format follows DESIGN.md.
- REQ-006: `--json` -- stable contract matching DESIGN.md's
  `speccy status --json` schema, including `schema_version: 1`.

**Checks (sketch):** golden snapshot tests for text + JSON;
stale-detection unit tests (hash drift, mtime drift, both);
empty-workspace edge case.

**Tasks (sketch):** workspace scanner; stale detector; task-state
aggregator; text renderer; JSON serialiser with `schema_version`
field.

---

## SPEC-0005: plan-command

**Slug:** `plan-command`
**Depends on:** SPEC-0001
**Surface:** `speccy plan [SPEC-ID]`

**Scope.** Render the Phase 1 prompt. No-arg form is greenfield
(reads VISION.md, asks the agent to propose the next SPEC slice).
SPEC-ID form is amendment (reads existing SPEC.md, asks for a
minimal diff). Output is the rendered prompt to stdout; no file
mutation.

**Requirements:**
- REQ-001: No-arg form renders `prompts/plan-greenfield.md`,
  inlining VISION.md and AGENTS.md.
- REQ-002: Arg form renders `prompts/plan-amend.md`, inlining the
  named SPEC.md (with its Changelog) and AGENTS.md.
- REQ-003: Auto-allocate the next available `SPEC-NNNN` ID by
  scanning `specs/`; gaps are not recycled.
- REQ-004: Context-budget trimming per DESIGN.md ordering when the
  rendered prompt approaches the host's limit.

**Checks (sketch):** snapshot tests for both forms; ID allocation
handles gaps and existing specs; AGENTS.md is loaded from the
project root.

**Tasks (sketch):** template loader (embedded resources); ID
allocator; prompt assembler with section ordering; budget trimmer.

---

## SPEC-0006: tasks-command

**Slug:** `tasks-command`
**Depends on:** SPEC-0001
**Surface:** `speccy tasks SPEC-ID [--commit]`

**Scope.** Render the Phase 2 prompt. Initial form (TASKS.md
absent) asks the agent to decompose the spec. Amendment form
(TASKS.md present) asks for surgical edits preserving completed
tasks. `--commit` is a sub-action that updates TASKS.md frontmatter
with the current SPEC.md sha256 and a UTC timestamp.

**Requirements:**
- REQ-001: Initial form renders `prompts/tasks-generate.md`.
- REQ-002: Amendment form renders `prompts/tasks-amend.md` with
  the current TASKS.md inlined plus the SPEC.md diff hints.
- REQ-003: `--commit` -- compute SPEC.md sha256, rewrite ONLY the
  frontmatter block of TASKS.md (`spec_hash_at_generation` +
  `generated_at`); preserve body byte-for-byte.
- REQ-004: `--commit` refuses if TASKS.md is missing; error
  message names the next step.

**Checks (sketch):** snapshot tests for both prompt forms;
`--commit` round-trip preserves body; hash matches SPEC.md content;
`--commit` without TASKS.md fails cleanly.

**Tasks (sketch):** prompt assemblers (initial + amend); SPEC.md
hasher hook into `speccy-core` (already covered by REQ-003 in
SPEC-0001); frontmatter rewriter that preserves body bytes;
`--commit` sub-action wiring.

---

## SPEC-0007: next-command

**Slug:** `next-command`
**Depends on:** SPEC-0001
**Surface:** `speccy next [--kind implement|review] [--json]`

**Scope.** Scan task state across the workspace, apply the priority
rules in DESIGN.md, and emit the next actionable task as text or
`--json`. `--kind` filters to implement (`[ ]`) or review (`[?]`)
across all specs.

**Requirements:**
- REQ-001: Priority -- lowest spec ID first; within a spec, prefer
  `[?]` (awaiting review) over `[ ]` (open) when no `--kind` is
  given.
- REQ-002: `--kind implement` returns the next `[ ]` task;
  `--kind review` returns the next `[?]` task plus its persona
  fan-out list.
- REQ-003: `--json` contract matches DESIGN.md (`kind` in
  `{implement, review, report, blocked}`); `schema_version: 1`.
- REQ-004: When all specs are complete (all tasks `[x]`) but
  REPORT.md is missing, emit `kind: report`.
- REQ-005: When no actionable work exists but state is incomplete
  (e.g. all open tasks claimed `[~]`), emit
  `kind: blocked` with a reason string.

**Checks (sketch):** priority-ordering unit tests; `--kind`
filtering; JSON snapshot per kind; `blocked` reason strings.

**Tasks (sketch):** priority scanner; kind filter; JSON assembler;
text-output formatter.

---

## SPEC-0008: implement-command

**Slug:** `implement-command`
**Depends on:** SPEC-0001
**Surface:** `speccy implement TASK-ID`

**Scope.** Render the Phase 3 implementer prompt for one task.
Locate the task by ID across all specs; inline the relevant
SPEC.md, the task entry with all prior notes, AGENTS.md, and the
`implementer.md` skill content.

**Requirements:**
- REQ-001: Locate the task by `T-NNN` across the workspace. Error
  clearly on not-found or ambiguous (same `T-NNN` in two specs).
- REQ-002: Render `prompts/implementer.md` with SPEC.md (full
  including Decisions), the task entry, all prior notes,
  AGENTS.md, and the suggested files block inlined.
- REQ-003: Context-budget trimming per DESIGN.md ordering.

**Checks (sketch):** snapshot test; ambiguous-ID error; not-found
error; budget activation.

**Tasks (sketch):** task locator; prompt assembler; budget
trimmer.

---

## SPEC-0009: review-command

**Slug:** `review-command`
**Depends on:** SPEC-0001
**Surface:** `speccy review TASK-ID --persona <name>`

**Scope.** Render the Phase 4 reviewer prompt for one persona on
one task. Resolve persona file (project-local override > shipped),
inline the relevant SPEC.md (with Decisions), task + all notes,
the relevant diff, and AGENTS.md.

**Requirements:**
- REQ-001: Locate the task; resolve the persona file via the
  lookup order (`.speccy/skills/personas/reviewer-X.md` first,
  shipped second). Lint warns and falls through if a local
  override is malformed.
- REQ-002: Compute the diff -- working tree vs HEAD by default; if
  the tree is clean, HEAD vs HEAD~1; if neither yields content,
  note "no diff available; review based on SPEC.md and task notes
  alone."
- REQ-003: Render `prompts/reviewer-<persona>.md` with all
  artifacts inlined.
- REQ-004: Reject unknown persona names; list available personas in
  the error.
- REQ-005: Persona files are the durable extension surface; the
  CLI does not hardcode persona content, only the lookup order
  and the rendering template.

**Checks (sketch):** persona-resolution precedence; diff fallback
chain; snapshot test per persona; unknown-persona error.

**Tasks (sketch):** persona resolver; diff computer (shell out to
`git` or use `gix`); prompt assembler; persona name registry.

---

## SPEC-0010: check-command

**Slug:** `check-command`
**Depends on:** SPEC-0001
**Surface:** `speccy check [CHK-ID]`

**Scope.** Execute the `command` for executable checks (kind =
`test` or `command`); print the prompt and exit zero with an
advisory note for manual checks (kind = `manual`). Stream
stdout/stderr live. No record is written.

**Requirements:**
- REQ-001: Discover every check via `spec.toml` parsing across all
  specs. With no argument, run all; with `CHK-ID`, run only that
  one.
- REQ-002: Executable checks run serially through the project
  shell (`sh -c` on Unix, `cmd /c` on Windows) from the project
  root.
- REQ-003: Stream stdout/stderr live. No buffering, no timeout
  (user Ctrl+C if needed).
- REQ-004: Exit code -- first non-zero exit code encountered, or 0
  if all pass.
- REQ-005: Manual checks print the prompt and a one-line "manual;
  verify and proceed" note; do not affect exit code.
- REQ-006: Unknown `CHK-ID` returns a clear error listing what was
  found.

**Checks (sketch):** integration tests with fixture spec.tomls and
dummy commands (cross-platform shell selection); manual-check
rendering; exit-code propagation; unknown-CHK-ID error.

**Tasks (sketch):** check collector; shell invoker; manual-check
renderer; live-stream I/O plumbing.

---

## SPEC-0011: report-command

**Slug:** `report-command`
**Depends on:** SPEC-0001
**Surface:** `speccy report SPEC-ID`

**Scope.** Render the Phase 5 report prompt for one spec. Inline
SPEC.md, TASKS.md (with all notes including retry counts),
AGENTS.md, and the `report.md` skill template.

**Requirements:**
- REQ-001: Render `prompts/report.md` with all relevant artifacts
  inlined.
- REQ-002: Refuse when the spec has any non-`[x]` tasks; error
  names the offending task IDs and states.
- REQ-003: Compute retry counts per task from inline notes
  (count of `Retry:` markers in the notes block).

**Checks (sketch):** snapshot test; refuse-incomplete error;
retry-count derivation correctness.

**Tasks (sketch):** prompt assembler; retry-counter; completeness
gate.

---

## SPEC-0012: verify-command

**Slug:** `verify-command`
**Depends on:** SPEC-0003, SPEC-0010
**Surface:** `speccy verify [--json]`

**Scope.** The CI gate. Run lint across all specs, then run every
executable check. Binary exit -- 0 if lint is clean and every
check passes; 1 otherwise. `--json` emits structured failure
breakdown.

**Requirements:**
- REQ-001: Run lint across all specs; collect every diagnostic at
  level `error`.
- REQ-002: Run every executable check (skip manual); collect
  results.
- REQ-003: Exit code is binary -- 0 (lint error-free AND all checks
  pass) or 1 (anything else).
- REQ-004: `--json` emits
  `{schema_version: 1, lint: [diagnostics...], checks: [results...]}`.
- REQ-005: Never flakes on its own state (no nondeterminism in
  ordering, hash computation, or rendering).

**Checks (sketch):** end-to-end with passing fixture (exit 0);
with lint failure (exit 1); with check failure (exit 1); JSON
shape stability.

**Tasks (sketch):** lint runner integration; check runner
integration; exit-code aggregator; JSON shape.

---

## SPEC-0013: skill-packs

**Slug:** `skill-packs`
**Depends on:** SPEC-0002 (for the copy mechanism), SPEC-0005..0011
(for the prompts those commands load)
**Surface:** none (markdown files only); consumed by SPEC-0002 at
init time and by every prompt-rendering command at runtime.

**Scope.** Ship the markdown skill packs that drive the full loop
in Claude Code and Codex. Includes per-host top-level recipe
skills, shared persona files, and shared prompt templates loaded
by the CLI.

**Requirements:**
- REQ-001: `skills/claude-code/` ships `speccy-init`,
  `speccy-plan`, `speccy-tasks`, `speccy-work`, `speccy-review`,
  `speccy-amend`, `speccy-ship` as markdown skills with Claude
  Code frontmatter.
- REQ-002: `skills/codex/` ships the parallel set with Codex
  frontmatter.
- REQ-003: `skills/shared/personas/` ships planner, implementer,
  reviewer-business, reviewer-tests, reviewer-security,
  reviewer-style, reviewer-architecture, reviewer-docs (8 files).
- REQ-004: `skills/shared/prompts/` ships plan-greenfield,
  plan-amend, tasks-generate, tasks-amend, implementer,
  reviewer-<persona> (one per persona), report (loaded by the
  matching CLI commands).
- REQ-005: Each persona file follows DESIGN.md's "Persona
  definitions" sketch -- role, focus, what's easy to miss, inline
  note format, worked example.
- REQ-006: Top-level skill files orchestrate the multi-step loops
  (`speccy-work` runs the implement loop; `speccy-review` runs
  the review fan-out; `speccy-amend` orchestrates plan + tasks
  edits).

**Checks (sketch):** file-presence (every listed file exists);
markdown lint per shipped file; manual checks that each pack
loads in its host with no errors and the recipe skills run a
trivial scenario end-to-end.

**Tasks (sketch):** author each persona file; author each prompt
template; author each top-level recipe skill per host; package as
embedded resources so SPEC-0002 can copy them at init time.

---

## Cross-cutting concerns

These show up in several specs. Status reflects where each one
landed; future specs should reuse rather than reinvent.

- **Workspace scan.** Landed as `speccy_core::workspace::{find_root, scan, stale_for}` in SPEC-0004. Reused by SPEC-0010, SPEC-0012, and any future command that needs project-root discovery or per-spec enumeration.
- **AGENTS.md inlining.** Landed as `speccy_core::prompt::load_agents_md` in SPEC-0005 (DEC-003: missing AGENTS.md is a warning + marker, not an error). To be reused by SPEC-0006, SPEC-0008, SPEC-0009, SPEC-0011.
- **Template loading + placeholder substitution.** Landed as `speccy_core::prompt::{load_template, render}` in SPEC-0005 with simple single-pass `{{NAME}}` substitution (DEC-001). Templates ship embedded in the binary via `include_dir!` (DEC-002, consistent with SPEC-0002).
- **Embedded resources.** `include_dir!` mechanism introduced in SPEC-0002 for skill packs. SPEC-0005 reuses it for prompt templates under `skills/shared/prompts/`. Open question on SPEC-0005: the bundle may move to `speccy-core` so both crates share one copy; implementer call.
- **Context-budget trimming.** Landed as `speccy_core::prompt::trim_to_budget` in SPEC-0005 with hardcoded 80,000-char budget (DEC-004). DESIGN.md drop ordering applied; warns on stderr when output still exceeds budget after all drops. To be reused by SPEC-0006, SPEC-0008, SPEC-0009, SPEC-0011.
- **Spec ID allocation.** Landed as `speccy_core::prompt::allocate_next_spec_id` in SPEC-0005 (DEC-005: `max + 1`; no gap recycling). Used by `speccy plan` (no-arg form).
- **JSON envelope (`schema_version: 1`).** Conventions established in SPEC-0004 (`status`) and matched in SPEC-0012 (`verify`): `schema_version` first, `repo_sha` via shell-out to git (empty if unavailable; SPEC-0004 DEC-003), structured (not stringified) lint diagnostics (SPEC-0004 DEC-002). SPEC-0007 (`next`) should follow the same shape.
- **Captured check execution.** Landed as `speccy_core::exec::run_checks_captured` in SPEC-0012 (DEC-001) -- pipes child stdio and tees to stderr, returning structured `CheckResult` per check. SPEC-0010's CLI continues to use inherited stdio; the library API is the verify-friendly variant.
- **Task lookup.** Landed as `speccy_core::task_lookup::{parse_ref, find}` in SPEC-0008 (DEC-001) -- accepts unqualified `T-NNN` (searches all specs) and qualified `SPEC-NNNN/T-NNN` (scopes to one spec); ambiguous unqualified IDs return `LookupError::Ambiguous` with candidate-spec list. Reused by SPEC-0009.
- **Persona registry.** Landed as `speccy_core::personas::{ALL, resolve_file}` in SPEC-0009 (DEC-001 / DEC-002). `ALL` is the six-name source of truth; SPEC-0007's `DEFAULT_PERSONAS` is the first 4 elements. Resolver looks at `.speccy/skills/personas/` (project-local override) before the embedded bundle; host-native locations are NOT in the chain.
- **Embedded skill content.** Landed in SPEC-0013 -- 33 markdown files total: 8 personas (planner, implementer, 6 reviewers), 11 prompt templates (one per phase / persona), 7 Claude Code recipes + 7 Codex recipes. Loaded by SPEC-0002 (init copy), SPEC-0005/0006/0008/0009/0011 (template rendering), and SPEC-0009 (persona resolution).

---

## What this planning doc is NOT

- Not a substitute for individual SPEC.md files. Each outline
  collapses into a full
  `.speccy/specs/NNNN-slug/{SPEC.md, spec.toml, TASKS.md}` triple
  when its turn comes.
- Not load-bearing in the running CLI. `speccy` does not read
  `PLANNING.md`. The file exists for human iteration on the
  bootstrap plan only and can be deleted once SPEC-0013 is
  shipped and Speccy is dogfooding itself.
- Not a status tracker. Spec status lives in each SPEC.md's
  frontmatter once that spec is deepened; this doc is purely a
  decomposition aid.
