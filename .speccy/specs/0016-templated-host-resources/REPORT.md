---
spec: SPEC-0016
outcome: delivered
generated_at: 2026-05-14T23:42:18Z
---

# SPEC-0016: Templated host resources and reviewer subagents

## Outcome

**delivered** — all six requirements satisfied; all 13 tasks closed
after two retries and one mid-loop SPEC amendment to reconcile
DEC-004 with REQ-002 / REQ-004 expansion requirements.

<report spec="SPEC-0016">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005 CHK-006">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-007">
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-008">
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-009 CHK-010">
</coverage>

</report>

## Task summary

- **Total tasks:** 13 (T-001 through T-013).
- **Retried tasks:** 2.
  - **T-006** (Codex SKILL.md wrappers) — flipped back because the
    tests reviewer blocked on (a) a missing description-matches-legacy
    test that became structurally impossible to write after T-008
    deleted the legacy oracle, and (b) wrappers using bare
    `{% include %}` rather than DEC-004's then-prescribed
    `{% raw %}{% include %}{% endraw %}` form. Both blockers were
    resolved by amending the SPEC; no code or test changes were
    needed on the retry pass.
  - **T-008** (legacy `skills/` tree deletion) — flipped back because
    the style reviewer blocked on a duplicate test helper
    (`workspace_root_path()` introduced at
    `speccy-cli/tests/skill_packs.rs:53` shadowed the pre-existing
    `workspace_root()` at `:898`). The retry deleted
    `workspace_root_path()` and retargeted the three named callers
    (`read_wrapper_template`, `bundle_layout_has_skill_md_per_host`,
    `bundle_legacy_flat_layout_absent`) onto `workspace_root()`.
- **SPEC amendments:** 1.
  - **DEC-004 rewrite** (Changelog 2026-05-14). The original
    invariant "every `{% include %}` of a module body inside a
    wrapper template is wrapped in `{% raw %}...{% endraw %}`"
    contradicted REQ-002 (rendered SKILL.md must contain
    `/speccy-tasks` for Claude Code and bare `speccy-tasks` for
    Codex, via `{{ cmd_prefix }}` expansion inside the included
    module body) and REQ-004 (rendered `speccy-review` step 4 must
    diverge per host via `{% if host == "claude-code" %}` blocks).
    The amendment makes bare `{% include %}` the canonical wrapper
    form, retitles DEC-004 ("Wrapper includes use bare
    `{% include %}`; persona bodies must avoid `\"\"\"`"), and
    documents the two-layer safety net (strict-undefined MiniJinja
    mode plus `t010_persona_bodies_have_no_toml_triple_quote`)
    that replaces the rejected `{% raw %}` wrapping. REQ-003 /
    REQ-006 / Assumptions wording and TASKS.md T-005 / T-006 /
    T-009 / T-010 "Tests to write" bullets were updated to match.
    Both T-006 retry blockers traced back to this SPEC defect; the
    amendment closed them.

## Out-of-scope items absorbed

Edits made during the loop that were not part of the planned task
scope but were necessary for tests to compile or for the work to
land cleanly:

- **`.editorconfig` `[*.tmpl] insert_final_newline = false`
  carve-out** (added during T-005 style review). Without the
  carve-out, the editor's default trailing-newline rule produced
  a double newline at the wrapper boundary (the included module
  body already supplies a trailing newline via the renderer's
  `keep_trailing_newline = true`), breaking byte-equivalence with
  the legacy SKILL.md and the `dogfood_outputs_match_committed_tree`
  guard. Documented inline in `.editorconfig` with a SPEC-0016
  reference.
- **Codex SKILL.md.tmpl wrapper byte-shape normalisation** (during
  T-007). The T-006 Codex wrappers shipped with an extra blank line
  between the close `---` fence and the `{% include %}` line plus
  an extra trailing newline; the T-005 Claude Code wrappers did
  not. T-005/T-006 wrapper-shape tests use `body.trim()` so both
  shapes passed those tests, but the byte-equality oracles in
  `tests/init.rs` failed once the renderer landed. T-007 normalised
  all seven Codex wrappers to the Claude Code shape for renderer
  trailing-newline-contract uniformity.
- **`clippy::unnecessary_trailing_comma` regression fix at
  `speccy-cli/tests/init.rs:679`** (introduced by `cargo fmt` during
  T-013 work; fixed inline before flipping T-011 to `[x]`).
- **Filesystem-read fallback for two SKILLS-based test helpers in
  `speccy-cli/tests/skill_packs.rs`** (during T-007). The `SKILLS`
  embedded constant was retired in favour of `RESOURCES`; the
  three callers in T-003's transient byte-equivalence tests were
  switched to direct filesystem reads via a new
  `workspace_root_path` helper rather than the embedded bundle.
  (That `workspace_root_path` helper later became T-008's
  style-blocker and was dedup'd in T-008's retry — see Task
  summary above.)

## Skill updates

- **`resources/modules/skills/speccy-work.md`** (step 3, both hosts
  via renderer) — the main agent applied a pre-T-001 disambiguation
  fix to the legacy `skills/claude-code/speccy-work/SKILL.md` and
  `skills/codex/speccy-work/SKILL.md` step 3 examples before
  spawning the T-001 implementer (bare `speccy implement T-NNN` is
  ambiguous across all 16 specs in this repo; the disambiguated
  form `speccy implement SPEC-NNNN/T-NNN` resolves the collision).
  T-003 absorbed the friction fix into the new host-neutral
  `resources/modules/skills/speccy-work.md` module body, and the
  renderer propagates it to both hosts' rendered SKILL.md outputs.
  Surfaced in T-001's procedural-compliance note as the only
  in-flight skill edit for the entire run.

## Deferred / known limitations

- **Codex prose-spawn reliability** (Open question 1, SPEC.md:826).
  Will Codex consistently spawn the four reviewer subagents based
  on the prose instruction "Spawn reviewer-business, reviewer-tests,
  reviewer-security, reviewer-style in parallel"? OpenAI's docs
  describe prose-spawn as the canonical pattern, but real-world
  reliability is a known unknown — only surfaceable through
  dogfooding. **Mitigation:** the CLI fallback (`speccy review T-NNN
  --persona X`) is wired unconditionally in the rendered
  `.agents/skills/speccy-review/SKILL.md`, so a missed prose-spawn
  degrades to the existing inline-render path with no loss of
  function. Re-evaluate after the first non-trivial Codex-driven
  spec run.
- **`bundle_subpath()` cleanup** (Open question 2, SPEC.md:835).
  Resolved during T-007 (the helper was removed cleanly once the
  renderer landed); no follow-up needed.
- **Persona-body Jinja-token guard test**
  (`persona_bodies_have_no_jinja_tokens`) — suggested by the
  amended DEC-004 as belt-and-braces hygiene to catch a future
  stray `{{` / `{%` token in a persona body before it produces a
  loud render-time error. Strict-undefined mode covers the failure
  mode today; the guard test would catch the regression at
  `cargo test` time with a clear filename. Deferred to a future
  spec; not blocking v1.
- **Style reviewer's comment-vs-code drift nit at
  `speccy-cli/tests/skill_packs.rs:1215-1224`** (T-006 section
  divider comment still mentions `{% raw %}{% include ... %}{% endraw %}`
  while the assertion at `:1310` expects plain `{% include %}`).
  Inherited verbatim from T-005's identical divider. Both T-005 and
  T-006 wrappers now conform to the canonical bare-`{% include %}`
  form per the amended DEC-004, so the assertion is SPEC-aligned;
  the divider comments are stale rather than wrong. Not worth a
  separate task; will surface naturally on the next edit to the
  section.
