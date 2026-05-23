---
spec: SPEC-0040
spec_hash_at_generation: 521caca0dd64a61db79fc1ca0c6444c197b92ff70a0029e52430857c9c96451f
generated_at: 2026-05-22T23:15:34Z
---
# Tasks: SPEC-0040 Retire `speccy.toml` — drop scaffolding, parser, dependency, and TOML-side `schema_version`

<task id="T-001" state="completed" covers="REQ-002 REQ-003 REQ-005">
## Delete `speccy-core::parse::toml_files` and relocate the shared `read_to_string` helper

Delete `speccy-core/src/parse/toml_files.rs` in full: the
`speccy_toml` parser function, the `SpeccyConfig` and
`ProjectConfig` types (plus their `RawSpeccyConfig` /
`RawProject` private mirrors), the `SUPPORTED_SCHEMA_VERSION`
constant, the `guard_schema_version` helper, and the in-module
`#[cfg(test)]` suite (`parses_valid_speccy_toml`,
`rejects_unknown_schema_version`, `io_error_names_the_path`).
Remove the `pub mod toml_files;` declaration and the three
re-exports (`speccy_toml`, `SpeccyConfig`, `ProjectConfig`) from
`speccy-core/src/parse/mod.rs`. Delete the
`ParseError::UnsupportedSchemaVersion` enum variant from
`speccy-core/src/error.rs` (its sole emitter is gone, and
pre-v1 the enum is not a stable contract — DEC-003).

The `read_to_string` helper currently lives at the bottom of
`toml_files.rs` and has three import sites:
`speccy-core/src/parse/spec_md.rs:26` (`use crate::parse::toml_files::read_to_string;`)
and `speccy-core/src/workspace.rs:492 / 579 / 584`
(`crate::parse::toml_files::read_to_string(...)` — three call
sites, one helper). REQ-003 requires the helper to survive at
a location both callers can import. Pick a destination: either
a tiny new private module like `speccy-core/src/parse/fs.rs`
(or `parse/io.rs`) carrying the function as
`pub(crate) fn read_to_string`, or surface it directly on
`parse/mod.rs`. The contract is only that the behavior
(wrapping `fs_err::read_to_string` errors as
`ParseError::Io { path, source }`) is preserved and both
existing call sites compile against the new path.

Delete `speccy-core/tests/workspace_loader.rs::workspace_speccy_toml_still_parses`
in full — it is the sole external consumer of
`speccy_core::parse::speccy_toml` and must go for
`cargo check --workspace` to pass after the parser is removed.

Leave the lint registry / `SPC-001` rule alone (Non-goals
section in SPEC.md — that's REQ-007's territory, only the
ARCHITECTURE.md row description changes there).

<task-scenarios>
Given the speccy workspace at HEAD after this task,
when ripgrep searches `speccy-core/src/` (case-sensitive) for
any of `toml_files`, `SpeccyConfig`, `ProjectConfig`,
`speccy_toml`, `SUPPORTED_SCHEMA_VERSION`,
`guard_schema_version`, or `UnsupportedSchemaVersion`,
then the search returns zero matches (covers CHK-004's
`*.rs` half restricted to `speccy-core/src/`).

Given the speccy workspace at HEAD after this task,
when `cargo check --workspace --all-targets --all-features`
runs,
then it exits 0 (covers CHK-003 — `spec_md` and `workspace`
still compile against the relocated helper).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace` runs,
then it exits 0 (covers CHK-005 — the SPEC.md /
TASKS.md / REPORT.md parse paths function end-to-end via the
new helper home).

Given a SPEC.md file at a path that does not exist on disk,
when `speccy_core::parse::spec_md` is invoked against that
path,
then it returns `ParseError::Io { path, source }` carrying
the missing path (proves the relocated helper still wraps I/O
errors as `ParseError::Io` per REQ-003 behavior).

Suggested files:
`speccy-core/src/parse/toml_files.rs` (delete),
`speccy-core/src/parse/mod.rs` (drop module + re-exports;
optionally add the new helper home),
`speccy-core/src/parse/spec_md.rs` (update import),
`speccy-core/src/workspace.rs` (update three import call
sites),
`speccy-core/src/error.rs` (drop
`ParseError::UnsupportedSchemaVersion`),
`speccy-core/tests/workspace_loader.rs` (delete
`workspace_speccy_toml_still_parses`).
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-006">
## Drop explicit `toml` dependency from `speccy-core/Cargo.toml` and migrate dev-time consumers

Remove the `toml = { workspace = true }` line from
`[dependencies]` in `speccy-core/Cargo.toml`. REQ-006's
done-when also forbids the crate from `[dev-dependencies]`,
so confirm no `[dev-dependencies]` entry exists either. The
crate was added for the deleted `parse::toml_files` module
(REQ-002); after T-001 the explicit declaration has no
first-party consumer in the production tree, **but** two
test files under `speccy-core/tests/` still reach for it:

- `speccy-core/tests/pin_shape.rs:170` parses Codex agent
  TOMLs via `toml::from_str::<TomlPins>(&contents)`.
- `speccy-core/tests/skill_stub_shape.rs:87` parses Codex
  agent TOMLs via `toml::from_str` and walks `toml::Value`.

Both files validate shape of files emitted by
`speccy-cli`'s `render_host_pack`. Two practical paths to
clear them:

- **(A) Relocate**: move both files to `speccy-cli/tests/`,
  which already declares `toml = { workspace = true }` under
  `[dev-dependencies]` (line 36) — they validate
  `speccy-cli`-rendered output anyway, so the home is
  natural. Update any `use speccy_core::...` imports for
  the new crate context.
- **(B) Rewrite in place**: keep both files where they are
  but remove the `toml::` dependency by parsing the specific
  TOML fields they touch via a small purpose-built parser
  (the assertions read narrow fields like
  `developer_instructions` and `model`).

Implementer picks the cheaper path. Assertion semantics
(file existence, presence of expected keys, value-shape
checks) must be preserved either way.

Transitive `toml` reaching `speccy-core` through another
crate (e.g. via a dev-dep that itself depends on `toml`) is
acceptable per REQ-006 and SPEC-0040 Notes — only the
explicit first-party declaration is removed; no transitive
chasing.

<task-scenarios>
Given the speccy workspace at HEAD after this task,
when ripgrep searches `speccy-core/Cargo.toml`
(case-sensitive) for any line matching the regex
`^toml\s*=`,
then the search returns zero matches (covers CHK-010).

Given the speccy workspace at HEAD after this task,
when `cargo build --workspace` runs,
then it exits 0 (covers CHK-011).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace` runs,
then it exits 0 (proves the relocated or rewritten Codex
agent shape tests still function without the `toml` crate
on the `speccy-core` side).

Given the speccy workspace at HEAD after this task,
when `cargo deny check` runs,
then it exits 0 (no new advisories or duplicate warnings
introduced by the dependency change).

Suggested files:
`speccy-core/Cargo.toml` (drop the explicit `toml = ...`
line),
`speccy-core/tests/pin_shape.rs` (relocate to
`speccy-cli/tests/` or rewrite in place),
`speccy-core/tests/skill_stub_shape.rs` (relocate to
`speccy-cli/tests/` or rewrite in place).
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001 REQ-004 REQ-005">
## Switch `speccy init` from `.speccy/speccy.toml` to `.speccy/.gitkeep` and rewrite affected tests

In `speccy-cli/src/init.rs`:

- Delete the `SPECCY_TOML_TEMPLATE` top-level `const`
  (currently `speccy-cli/src/init.rs:23`,
  `include_str!("templates/speccy.toml.tmpl")`).
- Delete the `render_speccy_toml` function (lines 332–335).
- Delete the `speccy_toml_path` / `speccy_toml_body` /
  `speccy_toml_action` / `content` block at the head of
  `build_plan` (lines 214–223), including the `PlanItem`
  push for `.speccy/speccy.toml`.
- Delete the two unit tests
  `render_speccy_toml_substitutes_name` and
  `render_speccy_toml_escapes_quotes` (lines 409–423).

In place of the deleted `speccy.toml` `PlanItem`, scaffold a
`.speccy/.gitkeep` `PlanItem` so `workspace::find_root`
keeps locating `.speccy/` between init and the first spec
(DEC-001 pins the gitkeep at `.speccy/.gitkeep`, not under
a child directory). The file content is empty bytes (or a
one-line marker if the renderer's `classify_content` needs
deterministic content for Conflict detection — either is
fine as long as it's stable across runs). Use the existing
`classify_content` three-way scheme so the file is
`Create` on absent / `Unchanged` on byte-identical /
`Conflict` on differs (same scheme as every other shipped
file).

Delete the template file
`speccy-cli/src/templates/speccy.toml.tmpl`.

In `speccy-cli/tests/init.rs`:

- Delete `scaffold_speccy_toml` (lines 99–116) in full.
- Rewrite `refuse_without_force` (lines 132–146) to trigger
  the conflict on a different shipped file — e.g.
  pre-write a `.claude/skills/speccy-init/SKILL.md` with
  byte-mismatching content. Keep the assertion shape:
  exit 1 + stderr contains `--force`.
- Rewrite `force_overwrites_shipped_files` (lines 148–195)
  to drop the `.speccy/speccy.toml` leg entirely. The
  `.claude/skills/speccy-init/SKILL.md` leg already present
  proves `--force` overwrites. Drop the toml assertions and
  keep the SKILL.md frontmatter checks.
- Rewrite the `exit-one-conflict` sub-case inside
  `exit_codes` (lines 811–816) to use a non-toml shipped
  file as the differing pre-existing content; keep the
  `exit 1` assertion.

Add a new test (e.g. `scaffold_gitkeep`) asserting REQ-001
end-to-end: after `speccy init` in a fresh temp dir,
`.speccy/.gitkeep` exists AND `.speccy/speccy.toml` does
NOT exist AND `speccy status` (invoked from the same dir)
exits 0 against the empty workspace. The combined
assertion covers both CHK-001 and CHK-002 in one fixture.

After this task, a case-sensitive ripgrep for `speccy.toml`
scoped to `speccy-cli/src/` and `speccy-cli/tests/` returns
zero hits (covers CHK-006 and CHK-008's `speccy-cli/tests/`
half). The `speccy-core/tests/` half of CHK-008 is covered
by T-001's deletion of `workspace_speccy_toml_still_parses`.

<task-scenarios>
Given a built `speccy` binary at HEAD after this task,
when `speccy init` runs in a fresh temp directory,
then `.speccy/.gitkeep` exists at the workspace root and
`.speccy/speccy.toml` does not exist (covers CHK-001).

Given a temp directory in which `speccy init` has just
run,
when `speccy status` runs from that directory,
then the command exits 0 — `workspace::find_root` succeeds
against the `.speccy/` marker directory (covers CHK-002).

Given a built `speccy` binary at HEAD after this task,
when `speccy init` runs in a fresh temp directory and the
printed plan is captured,
then no line in the plan output contains the substring
`speccy.toml` (covers CHK-007).

Given the speccy workspace at HEAD after this task,
when ripgrep searches `speccy-cli/src/` and
`speccy-cli/tests/` (case-sensitive) for the literal
`speccy.toml`,
then the search returns zero matches (covers CHK-006 and
the `speccy-cli/tests/` half of CHK-008).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0 — the rewritten refuse / force / exit-code
tests still pass against the new conflict triggers (covers
CHK-009).

Suggested files:
`speccy-cli/src/init.rs` (delete template const, render
function, build_plan participation, two unit tests; add
gitkeep PlanItem),
`speccy-cli/src/templates/speccy.toml.tmpl` (delete),
`speccy-cli/tests/init.rs` (delete `scaffold_speccy_toml`,
rewrite `refuse_without_force` /
`force_overwrites_shipped_files` / the `exit-one-conflict`
sub-case, add `scaffold_gitkeep`).
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-007">
## Strip `speccy.toml` references and the `## Schema version` story from `README.md` and `docs/ARCHITECTURE.md`

In `README.md`:

- Line 117: rewrite or remove the "Scaffolds
  `.speccy/speccy.toml` and the `.speccy/specs/` skeleton"
  bullet under "Step 1: Scaffold the workspace". A rewrite
  that keeps the bullet (e.g. "Scaffolds the `.speccy/`
  directory and the `.speccy/specs/` skeleton") is fine.
- Line 216: remove the `speccy.toml    Minimal project
  config (just schema_version + name)` line inside the
  "Repo layout after `speccy init`" diagram.
- Lines 245–247: rewrite or remove the closing
  paragraph "The only TOML left in the layout is the
  workspace-level `.speccy/speccy.toml`." The paragraph
  earlier asserts the requirement-to-scenario graph lives
  in-band; preserving that part is fine, dropping the
  TOML-claim sentence is required.

In `docs/ARCHITECTURE.md`:

- Line 211: remove the `speccy.toml` line under
  `.speccy/` in the file-layout diagram.
- Lines 756–772: delete the entire `## speccy.toml`
  section (heading, fenced TOML block, surrounding prose
  paragraph about `[policy]` / `[env]` / `[[global_checks]]`
  and the
  "If the CLI ever needs structured access to environment
  metadata, the block will come back with a real purpose"
  closing line). The follow-up SPEC referenced in DEC-002
  may eventually re-frame this material, but for this SPEC
  the section disappears outright.
- Lines 2091–2093: trim the `SPC-001` row in the lint
  catalogue's `code-block` summary so it reflects the
  rule's actual current behavior (the SPEC.md
  parse-failure catch-all surfaced by
  `speccy-core/src/lint/rules/spc.rs`) rather than the
  stale "Stray per-spec spec.toml ... the only TOML
  carried at spec level today is the workspace
  `.speccy/speccy.toml`" wording. Cross-check the rule's
  actual message against `spc.rs` before writing the new
  row so the catalogue does not introduce its own drift.
- Lines 2403–2408: delete the entire `## Schema version`
  section.
- Lines 2464–2467: rewrite the implementation-sequence
  bullet that begins "Artifact parser: `speccy.toml`, SPEC.md
  (YAML frontmatter + XML element tree ...)" to drop the
  `speccy.toml` mention while preserving the SPEC.md /
  TASKS.md / REPORT.md description.
- Fix stale prose immediately adjacent to the above edits
  (within the same section) when the deletion leaves a
  dangling clause or stranded transition. Broader
  ARCHITECTURE.md drift cleanup is **out of scope** per
  DEC-002 and the Non-goals list — defer it to the
  follow-up SPEC.

After this task, case-sensitive ripgreps over `README.md`
and `docs/ARCHITECTURE.md` return zero matches for
`speccy.toml`, `## Schema version`, and `## speccy.toml`.

<task-scenarios>
Given the speccy repo at HEAD after this task,
when ripgrep searches `README.md` and
`docs/ARCHITECTURE.md` (case-sensitive) for the literal
`speccy.toml`,
then the search returns zero matches (covers CHK-012).

Given the speccy repo at HEAD after this task,
when ripgrep searches `docs/ARCHITECTURE.md`
(case-sensitive) for the literal headings
`## speccy.toml` and `## Schema version`,
then the search returns zero matches (covers CHK-013).

Given the speccy repo at HEAD after this task,
when a reader skims the `SPC-001` row in
`docs/ARCHITECTURE.md`'s lint catalogue,
then the row describes the SPEC.md parse-failure
catch-all surface (the rule's actual current behavior)
rather than a stray spec.toml or workspace
`.speccy/speccy.toml`.

Given the speccy repo at HEAD after this task,
when `cargo test --workspace` runs,
then the pre-existing pins in `speccy-cli/tests/init.rs`
(`architecture_doc_pins_*`, `reviewer_tests_persona_pins_*`)
continue to pass — none of those load-bearing
assertions touch `speccy.toml` or
`## Schema version`, so they survive intact.

Suggested files:
`README.md`,
`docs/ARCHITECTURE.md`,
`speccy-core/src/lint/rules/spc.rs` (read-only — consult
to derive the corrected SPC-001 row description; do not
modify per Non-goals).
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-008">
## Rename `NextAction::Implement` → `Work` and flip the `next_action.kind` JSON/text discriminator

In `speccy-core/src/next.rs`:

- Rename the `NextAction::Implement { task_id }` variant to
  `NextAction::Work { task_id }`. Update the matching
  arm in `compute_for_spec` accordingly.
- Update the priority-rule module doc-comment (lines ~13–19)
  so the `state="pending"` branch line reads
  `kind = "work"` instead of `kind = "implement"`. Update the
  Rust-doc enumeration in the doc-comment for
  `compute_for_spec` (the `# Priority` block) the same way:
  `Implement` becomes `Work` in the listed step.

In `speccy-cli/src/next_output.rs`:

- In `to_json_action`, change the `NextAction::Implement`
  match arm to `NextAction::Work` and flip the literal `kind`
  string from `"implement"` to `"work"`.
- In `render_text_per_spec`, change the `NextAction::Implement`
  match arm to `NextAction::Work` and flip the rendered token
  in the format string from `implement` to `work`.
- In `render_text_workspace` (and any other text renderer that
  matches on the enum), do the equivalent rename + keyword
  flip.
- Update the two unit-test helpers near the bottom of
  `next_output.rs` (`NextAction::Implement { ... }`
  construction at lines ~230 and ~273) to construct
  `NextAction::Work { ... }`.

In `speccy-cli/tests/`:

- `next_json.rs`: update every assertion that matched on
  `"kind": "implement"` (or the unquoted JSON-tag form) so
  it asserts on `"work"` instead. The `kind must be implement`
  failure-message strings should also be updated for
  legibility.
- `next_derived.rs`: the `--kind implement → clap error` test
  (around line 285) stays — `--kind` was removed and any
  string still produces a clap error — but update the test
  name + message strings if they reference "implement" by
  word; the actual `--kind implement` invocation is fine as
  a probe since clap rejects it on the absence of the
  argument, not on the value.
- `next_text.rs`: update text-rendering assertions that match
  on the `implement` keyword to match `work`.
- `verify.rs`: only update if it asserts on the discriminator
  (which the orchestrator did not see in this scan; verify
  via ripgrep before editing).

In shipped skill and agent material under
`.claude/`, `.codex/`, and `resources/modules/`:

- `.claude/agents/speccy-work.md`, `.codex/agents/speccy-work.toml`,
  `resources/modules/phases/speccy-work.md`: any line that
  quotes `next_action.kind == "implement"` flips to
  `next_action.kind == "work"`.
- `resources/modules/skills/speccy-orchestrate.md`: this body
  was already speaking `work` terminology; cross-check that
  it does not also reference `"implement"` and update if so.
- Any other body returned by a ripgrep for
  `next_action.kind` (or `kind = "implement"`) across
  `.claude/`, `.codex/`, and `resources/modules/` gets the
  same treatment.

In `docs/ARCHITECTURE.md`:

- Find every place that documents `speccy next`'s priority
  rule using the literal token `"implement"` (or
  `kind = "implement"`) as the JSON discriminator and flip
  the token to `"work"`. Leave the English verb
  "implement" alone where it appears in prose about the
  implementer phase.

Historical SPEC artifacts under `.speccy/specs/0007/`,
`/0023/`, `/0032/`, `/0033/` and their journal/evidence files
are **read-only**: do not retroactively rewrite them. They
describe what was true at the time and are part of the
project's history.

Hygiene gate before flipping the task to `in-review`:
`cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly
fmt --all --check`, `cargo deny check`. All four must pass.

<task-scenarios>
Given the speccy workspace at HEAD after this task,
when ripgrep searches `speccy-core/src/` and `speccy-cli/src/`
(case-sensitive) for the literal `"implement"`,
then the search returns zero matches (covers CHK-014).

Given the speccy workspace at HEAD after this task,
when `cargo test --workspace --all-features` runs,
then it exits 0 (covers CHK-015).

Given a built `speccy` binary at HEAD after this task,
when `speccy next --json` runs in a workspace with a spec
whose first pending task is `T-NNN`,
then the emitted JSON contains the substring `"kind":"work"`
(or `"kind": "work"` after pretty-printing) and does not
contain `"kind":"implement"` (covers CHK-016).

Given the speccy workspace at HEAD after this task,
when ripgrep searches `.claude/agents/`, `.codex/agents/`,
and `resources/modules/` (case-sensitive) for the substring
`next_action.kind == "implement"`,
then the search returns zero matches.

Suggested files:
`speccy-core/src/next.rs`,
`speccy-cli/src/next_output.rs`,
`speccy-cli/tests/next_json.rs`,
`speccy-cli/tests/next_text.rs`,
`speccy-cli/tests/next_derived.rs`,
`.claude/agents/speccy-work.md`,
`.codex/agents/speccy-work.toml`,
`resources/modules/phases/speccy-work.md`,
`resources/modules/skills/speccy-orchestrate.md`,
`docs/ARCHITECTURE.md`.
</task-scenarios>
</task>
