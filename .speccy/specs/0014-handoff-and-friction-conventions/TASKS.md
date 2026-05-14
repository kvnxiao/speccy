---
spec: SPEC-0014
spec_hash_at_generation: f0c99db42c3ce6fdffd2b34334460cd67c90cee5de0d456b3027c40ae3788eb2
generated_at: 2026-05-14T03:00:29Z
---

# Tasks: SPEC-0014 Handoff template + friction-to-skill-update conventions

> Hash recorded inline pending the next `speccy tasks SPEC-0014
> --commit` run; matches the sha256 of SPEC.md at draft time.

## Phase 1: Implementer-note handoff template

- [?] **T-001**: Embed the six-field handoff template in the implementer prompt
  - Covers: REQ-001
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs -- implementer_prompt_handoff_template`
      asserts the prompt body contains a fenced ```` ```markdown ```` block
      that includes the six exact labels: `Completed`, `Undone`,
      `Commands run`, `Exit codes`, `Discovered issues`,
      `Procedural compliance`.
    - The test should locate the fenced block (not just any
      substring) so unrelated mentions in prose don't false-positive.
  - Suggested files: `skills/shared/prompts/implementer.md`,
    `speccy-cli/tests/skill_packs.rs`
  - Implementer note (session-spec0014-impl):
    - Completed: added `### Handoff template` subsection to
      `skills/shared/prompts/implementer.md` with a fenced ```markdown
      block listing all six labels verbatim (Completed, Undone, Commands
      run, Exit codes, Discovered issues, Procedural compliance);
      added `fenced_blocks` helper in
      `speccy-cli/tests/skill_packs.rs` plus the
      `implementer_prompt_handoff_template` test using `HANDOFF_LABELS`
      constant. The example block under `## When you hit friction` is
      also fenced ```markdown but does not contain all six labels, so
      the test's `any()` selector lands on the template block.
    - Undone: (none)
    - Commands run: `cargo test -p speccy-cli --test skill_packs --
      implementer_prompt_handoff_template`
    - Exit codes: pass
    - Discovered issues: (none)
    - Procedural compliance: (none)

- [?] **T-002**: Rewrite the prompt's `## Your task` step that produces the note
  - Covers: REQ-001
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs -- implementer_prompt_handoff_referenced_in_task_steps`
      asserts the `## Your task` body references the handoff template
      (by phrase or by repeating the field list) and instructs writing
      `(none)` for empty fields.
    - Negative case: confirm the old freeform "Append one implementer
      note ..." sentence is removed (string-absence check on the
      pre-edit text).
  - Suggested files: `skills/shared/prompts/implementer.md`,
    `speccy-cli/tests/skill_packs.rs`
  - Implementer note (session-spec0014-impl):
    - Completed: rewrote step 5 of `## Your task` to reference the
      "six-field handoff template" and require `(none)` for empty
      fields; intro paragraph also updated to mention "the handoff
      template below". Test uses `section_body` helper to scope to
      `## Your task`, asserts `contains("handoff template")` and
      `contains("(none)")`, and asserts the old freeform sentence
      (`summarizing what you did, including any out-of-scope edits
      made for the test to compile`) is absent anywhere in the prompt.
    - Undone: (none)
    - Commands run: `cargo test -p speccy-cli --test skill_packs --
      implementer_prompt_handoff_referenced_in_task_steps`
    - Exit codes: pass
    - Discovered issues: initial wording wrapped "handoff" / "template"
      across a line break, which made `contains("handoff template")`
      fail; reflowed step 5 so the phrase stays on one line.
    - Procedural compliance: (none)

## Phase 2: Friction-to-skill-update pattern

- [?] **T-003**: Add the `## When you hit friction` section to the implementer prompt
  - Covers: REQ-002
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs -- implementer_prompt_friction_section`
      asserts the literal heading `## When you hit friction` exists
      in the prompt body and that the section contains at least one
      fenced code block whose contents reference a `skills/` path
      (e.g. `skills/claude-code/...` or `skills/shared/...`).
    - The section should sit between `## Suggested files` and
      `## Your task` so an implementer hits it before producing work.
  - Suggested files: `skills/shared/prompts/implementer.md`,
    `speccy-cli/tests/skill_packs.rs`
  - Implementer note (session-spec0014-impl):
    - Completed: inserted `## When you hit friction` between
      `## Suggested files` and `## Your task` in
      `skills/shared/prompts/implementer.md`; section names the pattern,
      gives an `npm test` -> `pnpm` worked example, and includes a
      fenced ```markdown block referencing
      `skills/shared/prompts/implementer.md` so the test's
      `contains("skills/")` check passes. Test also enforces ordering
      via `body.find()` index comparison.
    - Undone: (none)
    - Commands run: `cargo test -p speccy-cli --test skill_packs --
      implementer_prompt_friction_section`
    - Exit codes: pass
    - Discovered issues: (none)
    - Procedural compliance: (none)

- [?] **T-004**: Mirror the friction pointer in the implementer persona
  - Covers: REQ-002
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs -- implementer_persona_friction_reference`
      asserts the `## What to consider` section of
      `skills/shared/personas/implementer.md` contains a bullet that
      mentions friction and points back to the prompt (substring
      match on a stable phrase like
      `update the relevant skill file under skills/`).
  - Suggested files: `skills/shared/personas/implementer.md`,
    `speccy-cli/tests/skill_packs.rs`
  - Implementer note (session-spec0014-impl):
    - Completed: added a friction bullet in `## What to consider` of
      `skills/shared/personas/implementer.md` that references
      `## When you hit friction` in the prompt and uses the stable
      phrase ``update the relevant skill file under `skills/` `` as
      the shared invariant (encoded as the `FRICTION_PHRASE` constant
      in the test). Also updated the persona's `## Output format`
      bullet and `## Example` so they describe the new six-field
      template instead of the old freeform `Out of scope:` shape; this
      keeps the persona internally consistent with the prompt.
    - Undone: (none)
    - Commands run: `cargo test -p speccy-cli --test skill_packs --
      implementer_persona_friction_reference`
    - Exit codes: pass
    - Discovered issues: first wording wrapped the stable phrase
      across two lines so `contains` failed; rewrote the bullet so the
      whole phrase stays on one source line.
    - Procedural compliance: (none)

- [?] **T-005**: Dogfood the pattern in speccy's own AGENTS.md
  - Covers: REQ-002
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs -- agents_md_friction_paragraph`
      reads `AGENTS.md` from the workspace root (via
      `include_str!("../../AGENTS.md")` or `env!("CARGO_MANIFEST_DIR")`
      + `fs_err::read_to_string`) and asserts a paragraph under
      `## Conventions for AI agents specifically` documents the
      friction-to-skill-update pattern. The test should match on a
      stable invariant phrase the implementer commits to in the same
      change (e.g. `"update the relevant skill file under skills/"`).
  - Suggested files: `AGENTS.md`, `speccy-cli/tests/skill_packs.rs`
  - Note: `AGENTS.md` is not part of the embedded bundle, so the test
    reads it from the source tree at compile-time. Document this in
    the test comment so future readers don't expect bundle access.
  - Implementer note (session-spec0014-impl):
    - Completed: appended a friction-loop bullet to
      `## Conventions for AI agents specifically` in `AGENTS.md`. The
      bullet reuses the same `FRICTION_PHRASE` invariant as the persona
      and references `Procedural compliance`. Test uses
      `include_str!("../../AGENTS.md")` (test file path
      `speccy-cli/tests/skill_packs.rs`, two parents up = workspace
      root) per the SPEC's "Assumptions" note; chose `include_str!`
      over the `fs_err` runtime read because it's hermetic and matches
      the existing bundle-content style.
    - Undone: (none)
    - Commands run: `cargo test -p speccy-cli --test skill_packs --
      agents_md_friction_paragraph`
    - Exit codes: pass
    - Discovered issues: initial wording wrapped `Procedural compliance`
      across two lines; reflowed so the literal label sits on one line
      to keep the substring check honest.
    - Procedural compliance: (none) — AGENTS.md is the dogfood target
      for this task, not a friction-fix during the task.

## Phase 3: REPORT.md skill-updates surfacing

- [?] **T-006**: Add `## Skill updates` to the report prompt
  - Covers: REQ-003
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs -- report_prompt_skill_updates_section`
      asserts the literal substring `## Skill updates` is present in
      `skills/shared/prompts/report.md` and that it falls between
      `## Out-of-scope items absorbed` and
      `## Deferred / known limitations` in document order. Verify
      ordering by `body.find()` index comparison.
    - The prompt instructions for that section should mention
      `git diff --name-only -- skills/` or equivalent so the
      report-writing agent has a concrete derivation path.
  - Suggested files: `skills/shared/prompts/report.md`,
    `speccy-cli/tests/skill_packs.rs`
  - Implementer note (session-spec0014-impl):
    - Completed: rewrote the section enumeration in
      `skills/shared/prompts/report.md` step 2 to use literal
      `## Heading` references (one per section) instead of bold-bullet
      `**Heading:**` markup, so the three substrings the test asserts
      (`## Out-of-scope items absorbed`, `## Skill updates`,
      `## Deferred / known limitations`) appear verbatim in the prompt
      body in document order. The `## Skill updates` description names
      `git diff --name-only -- skills/` as the derivation path and
      mandates `(none)` when no skill files moved.
    - Undone: (none)
    - Commands run: `cargo test -p speccy-cli --test skill_packs --
      report_prompt_skill_updates_section`
    - Exit codes: pass
    - Discovered issues: original markup used `**Bold:**` notation so
      the literal `## Skill updates` string was absent from the prompt;
      switching to `## Heading` form satisfies the test and reads more
      naturally as "produce a section with this heading".
    - Procedural compliance: (none)

## Phase 4: Wire-up and verification

- [?] **T-007**: Run the full hygiene sweep
  - Covers: REQ-001, REQ-002, REQ-003
  - Tests to write:
    - This task adds no new tests. It runs the four-command pre-commit
      gate and adjusts any drift the new content causes:
      `cargo test --workspace`,
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      `cargo +nightly fmt --all --check`,
      `cargo deny check`.
    - Also runs `cargo run -p speccy-cli -- verify` to confirm the new
      checks pass and lint is clean.
  - Suggested files: (none directly; iterates as needed)
  - Implementer note (session-spec0014-impl):
    - Completed: ran all four gates plus `speccy verify` (against a
      `--release` build so the running binary isn't held open by
      Windows when the verify driver shells out to `cargo test`).
      `cargo +nightly fmt --all` rewrapped two test functions in
      `speccy-cli/tests/skill_packs.rs`; no manual fixup needed.
    - Undone: (none)
    - Commands run: `cargo test --workspace`;
      `cargo clippy --workspace --all-targets --all-features --
      -D warnings`; `cargo +nightly fmt --all --check`;
      `cargo deny check`; `./target/release/speccy.exe verify`.
    - Exit codes: pass, pass, pass, pass (warnings only, pre-existing),
      pass (111 passed, 0 failed, 0 in-flight, 1 manual).
    - Discovered issues: `cargo run -p speccy-cli -- verify` from
      source fails on Windows because cargo can't replace the running
      `speccy.exe` while the verify driver invokes `cargo test` as a
      sub-process. Running the release binary directly sidesteps the
      file lock and is what should be documented for Windows
      contributors.
    - Procedural compliance: (none) — surfacing the Windows
      `cargo run -- verify` self-replace issue belongs in a follow-up
      doc/spec, not in `skills/` for this task.
