---
spec: SPEC-0040
outcome: implemented
generated_at: 2026-05-22T23:59:00Z
---

# REPORT: SPEC-0040 Retire `speccy.toml` — drop scaffolding, parser, dependency, and TOML-side `schema_version`

<report spec="SPEC-0040">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-003 replaced the `.speccy/speccy.toml` `PlanItem` in `speccy-cli/src/init.rs`
with a `.speccy/.gitkeep` `PlanItem` (empty content, stable across runs, using
the existing `classify_content` three-way scheme). The `SPECCY_TOML_TEMPLATE`
`include_str!` constant and the `render_speccy_toml` function were deleted. The
template file `speccy-cli/src/templates/speccy.toml.tmpl` was deleted. After
`speccy init` in a fresh temp directory, `.speccy/.gitkeep` exists and
`.speccy/speccy.toml` does not exist. `speccy status` invoked from the freshly
scaffolded directory exits 0 -- `workspace::find_root` locates `.speccy/` via
the marker directory. A new integration test `scaffold_gitkeep` in
`speccy-cli/tests/init.rs` asserts both CHK-001 and CHK-002 end-to-end. Retry
count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
T-001 deleted `speccy-core/src/parse/toml_files.rs` in full, including the
`speccy_toml` parser function, the `SpeccyConfig` and `ProjectConfig` types
(and their `RawSpeccyConfig` / `RawProject` private mirrors), the
`SUPPORTED_SCHEMA_VERSION` constant, the `guard_schema_version` helper, and the
in-module test suite. The `pub mod toml_files;` declaration and the three
re-exports (`speccy_toml`, `SpeccyConfig`, `ProjectConfig`) were removed from
`speccy-core/src/parse/mod.rs`. The `ParseError::UnsupportedSchemaVersion`
variant was deleted from `speccy-core/src/error.rs` (DEC-003). A ripgrep for
`toml_files`, `SpeccyConfig`, `ProjectConfig`, `speccy_toml`,
`SUPPORTED_SCHEMA_VERSION`, `guard_schema_version`, or `UnsupportedSchemaVersion`
scoped to `*.rs` returns zero hits. `cargo check --workspace
--all-targets --all-features` exits 0. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005">
T-001 relocated the `read_to_string` helper from `toml_files.rs` to a new
`speccy-core/src/parse/fs.rs` module (`pub(crate) fn read_to_string`),
preserving its behavior of wrapping `fs_err::read_to_string` errors as
`ParseError::Io { path, source }`. Import sites in
`speccy-core/src/parse/spec_md.rs` and the three call sites in
`speccy-core/src/workspace.rs` were updated to the new path. `cargo test
--workspace` exits 0, proving both callers function end-to-end via the
relocated helper. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006 CHK-007">
T-003 removed from `speccy-cli/src/init.rs` every symbol that produced
`.speccy/speccy.toml`: the `SPECCY_TOML_TEMPLATE` constant, the
`render_speccy_toml` function, the `speccy_toml_path` / `speccy_toml_body` /
`speccy_toml_action` local bindings inside `build_plan`, the corresponding
`PlanItem` push, and the two unit tests `render_speccy_toml_substitutes_name`
and `render_speccy_toml_escapes_quotes`. The template file
`speccy-cli/src/templates/speccy.toml.tmpl` was deleted. A ripgrep for
`speccy.toml` scoped to `speccy-cli/src/` returns zero hits. A built `speccy`
binary's init plan output contains no line with `speccy.toml`. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008 CHK-009">
T-001 deleted `speccy-core/tests/workspace_loader.rs::workspace_speccy_toml_still_parses`
in full. T-003 deleted `speccy-cli/tests/init.rs::scaffold_speccy_toml`, rewrote
`refuse_without_force` to trigger conflict on a `.claude/skills/speccy-init/SKILL.md`
byte mismatch, rewrote `force_overwrites_shipped_files` to drop the
`speccy.toml` leg and cover `--force` semantics via the SKILL.md leg, and
rewrote the `exit-one-conflict` sub-case inside `exit_codes` to use a non-TOML
shipped file as the conflict trigger. A new `scaffold_gitkeep` test covers
CHK-001 / CHK-002. A ripgrep for `speccy.toml` scoped to `speccy-cli/tests/`
and `speccy-core/tests/` returns zero hits. `cargo test --workspace
--all-features` exits 0. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-010 CHK-011">
T-002 removed the explicit `toml = { workspace = true }` line from
`speccy-core/Cargo.toml`'s `[dependencies]`. The two test files that depended on
the `toml` crate (`speccy-core/tests/pin_shape.rs` and
`speccy-core/tests/skill_stub_shape.rs`) were relocated via `git mv` to
`speccy-cli/tests/`, which already declared `toml = { workspace = true }` in its
`[dev-dependencies]`. Assertion semantics are byte-identical to their pre-T-002
form. A ripgrep for `^toml\s*=` in `speccy-core/Cargo.toml` returns zero hits.
`cargo build --workspace` and `cargo deny check` both exit 0. Retry count: 0.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-012 CHK-013">
T-004 edited `README.md` to rewrite the "Scaffolds `.speccy/speccy.toml`"
bullet, remove the `speccy.toml` line from the repo-layout diagram, and drop the
closing TOML-claim paragraph. T-004 edited `.speccy/ARCHITECTURE.md` to remove
the `## speccy.toml` section, the `## Schema version` section, the
`speccy.toml` line in the file-layout diagram, and the stale TOML wording from
the `SPC-001` lint-catalogue row (the row now describes the SPEC.md
parse-failure catch-all per the actual rule in `spc.rs`) and the
implementation-sequence bullet. A ripgrep for `speccy.toml` across `README.md`
and `.speccy/ARCHITECTURE.md` returns zero hits. A ripgrep for `## speccy.toml`
and `## Schema version` in `.speccy/ARCHITECTURE.md` returns zero hits. Retry
count: 0.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-014 CHK-015 CHK-016">
T-005 renamed `NextAction::Implement { task_id }` to `NextAction::Work { task_id }`
in `speccy-core/src/next.rs` and updated the module doc-comment and
`compute_for_spec` priority-rule doc. In `speccy-cli/src/next_output.rs` the
`to_json_action` literal flipped from `"implement"` to `"work"` and all text
renderer match arms were updated. Unit-test helper constructions of
`NextAction::Implement` were updated to `NextAction::Work`. Test assertions in
`speccy-cli/tests/next_json.rs`, `next_text.rs`, and `next_derived.rs` were
updated to match `"work"`. Shipped skill and agent files under `.claude/`,
`.codex/`, and `resources/modules/` that quoted `next_action.kind == "implement"`
were updated to `"work"`. `.speccy/ARCHITECTURE.md` literal JSON discriminator
quotes were flipped. A ripgrep for `"implement"` scoped to `speccy-core/src/`
and `speccy-cli/src/` returns zero hits. `cargo test --workspace --all-features`
exits 0. A built binary emits `"kind":"work"` (not `"kind":"implement"`) for a
pending-task spec. Historical SPEC artifacts were not retroactively edited.
Retry count: 0.
</coverage>

</report>
