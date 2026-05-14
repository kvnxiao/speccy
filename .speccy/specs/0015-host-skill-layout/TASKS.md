---
spec: SPEC-0015
spec_hash_at_generation: 0462ade40ffa257a0eb9461c44e2ec2113f67b192cac21d19c4273c80490ac35
generated_at: 2026-05-14T04:30:11Z
---

# Tasks: SPEC-0015 host-skill-layout

> Hash recorded inline pending the next `speccy tasks SPEC-0015
> --commit` run; matches the sha256 of SPEC.md at draft time.

## Phase 1: Bundle restructure

- [x] **T-001**: Restructure the host packs from flat-file to SKILL.md
  - Covers: REQ-001, REQ-003
  - Tests to write:
    - No new tests in this task. The task moves files and edits
      frontmatter; the existing `claude_code_recipes`,
      `codex_recipes`, and `recipe_content_shape` tests in
      `speccy-cli/tests/skill_packs.rs` must continue to pass after
      this task by updating the `RECIPE_FILES` and `LOOP_RECIPES`
      constants to the new paths (e.g. `speccy-init/SKILL.md`,
      `speccy-plan/SKILL.md`, ...). The existing `init.rs`
      integration tests that hard-code `.claude/commands/speccy/...`
      and `.codex/skills/speccy/...` paths will fail after T-005;
      do not rewire them in this task.
  - Suggested files:
    - `skills/claude-code/speccy-init/SKILL.md` (new; body from
      legacy `skills/claude-code/speccy/init.md`)
    - `skills/claude-code/speccy-plan/SKILL.md`
    - `skills/claude-code/speccy-tasks/SKILL.md`
    - `skills/claude-code/speccy-work/SKILL.md`
    - `skills/claude-code/speccy-review/SKILL.md`
    - `skills/claude-code/speccy-ship/SKILL.md`
    - `skills/claude-code/speccy-amend/SKILL.md`
    - `skills/codex/speccy-init/SKILL.md` (new; body from legacy
      `skills/codex/speccy/init.md`)
    - `skills/codex/speccy-plan/SKILL.md`
    - `skills/codex/speccy-tasks/SKILL.md`
    - `skills/codex/speccy-work/SKILL.md`
    - `skills/codex/speccy-review/SKILL.md`
    - `skills/codex/speccy-ship/SKILL.md`
    - `skills/codex/speccy-amend/SKILL.md`
    - Delete legacy directories: `skills/claude-code/speccy/` and
      `skills/codex/speccy/`.
    - Frontmatter edits:
      - Claude Code pack: each new SKILL.md gains
        `name: speccy-<verb>` (the pack previously had no `name`
        field).
      - Codex pack: each `name: speccy:<verb>` becomes
        `name: speccy-<verb>` (drop the colon).
    - Body preserved as-is in this task; description text is
      rewritten in T-003.
    - `speccy-cli/tests/skill_packs.rs` (update
      `RECIPE_FILES`/`LOOP_RECIPES` constants to new paths;
      tighten `claude_code_recipes` to also enforce
      `require_name = true` so both packs are checked uniformly).

- [x] **T-002**: Add bundle layout tests (CHK-001, CHK-002)
  - Covers: REQ-001
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs --
      bundle_layout_has_skill_md_per_host` walks the embedded
      bundle (`speccy_cli::embedded::SKILLS`) and asserts that
      for both hosts (`claude-code`, `codex`) and each of the
      seven skill names (`speccy-init`, `speccy-plan`,
      `speccy-tasks`, `speccy-work`, `speccy-review`,
      `speccy-ship`, `speccy-amend`) there is exactly one file
      at `<host>/<name>/SKILL.md` and that file has non-empty
      contents.
    - `cargo test -p speccy-cli --test skill_packs --
      bundle_legacy_flat_layout_absent` asserts the embedded
      bundle does **not** contain any `*.md` file under
      `claude-code/speccy/` or `codex/speccy/` — the legacy
      flat directories are gone.
  - Suggested files:
    - `speccy-cli/tests/skill_packs.rs`

## Phase 2: Description rewrites

- [x] **T-003**: Rewrite each shipped SKILL.md description for natural-language activation
  - Covers: REQ-004
  - Tests to write:
    - No new tests in this task. The quality assertions are
      added in T-004; this task makes them pass.
    - As a guard for human reviewers, each rewritten description
      must:
      - Lead with what the skill does (no "Phase N" prefix).
      - Contain `Use when` (case-insensitive).
      - Be at most 500 characters.
      - Reference at least two concrete user phrases a reader
        might say in prose (e.g. for `speccy-plan`: "draft a
        spec", "spec out X").
    - The seven rewrites drafted during the SPEC-0015 planning
      session are the starting point; an implementer may refine
      wording as long as the constraints above hold.
  - Suggested files:
    - `skills/claude-code/speccy-init/SKILL.md` (frontmatter
      `description:` only; body preserved)
    - `skills/claude-code/speccy-plan/SKILL.md`
    - `skills/claude-code/speccy-tasks/SKILL.md`
    - `skills/claude-code/speccy-work/SKILL.md`
    - `skills/claude-code/speccy-review/SKILL.md`
    - `skills/claude-code/speccy-ship/SKILL.md`
    - `skills/claude-code/speccy-amend/SKILL.md`
    - Same seven files under `skills/codex/`.
    - The Claude Code and Codex descriptions may be identical
      per skill; cross-host divergence is allowed but not
      required.

- [x] **T-004**: Add frontmatter shape and description quality tests (CHK-005, CHK-006)
  - Covers: REQ-003, REQ-004
  - Tests to write:
    - `cargo test -p speccy-cli --test skill_packs --
      shipped_skill_md_frontmatter_shape` walks the bundle, parses
      each SKILL.md's YAML frontmatter via the existing
      `RecipeFrontmatter` struct, and asserts:
      - `name` is present and non-empty.
      - `name` equals the SKILL.md's parent directory name
        (e.g. `speccy-plan`).
      - `description` is present, non-empty, and single-line
        (contains no `\n`).
    - `cargo test -p speccy-cli --test skill_packs --
      shipped_descriptions_natural_language_triggers` walks the
      bundle and for each SKILL.md asserts:
      - The description does not start with `Phase ` followed by
        a digit (regex `^Phase \d`).
      - The description contains the substring `use when`
        (case-insensitive).
      - The description is at most 500 characters.
  - Suggested files:
    - `speccy-cli/tests/skill_packs.rs`

## Phase 3: Install destination

- [x] **T-005**: Move Claude Code install destination to `.claude/skills/`
  - Covers: REQ-002
  - Tests to write:
    - No new tests added in this task; the destination change
      makes T-006's tests pass. The existing init.rs integration
      tests that hard-code `.claude/commands/speccy/...` will
      break here and get rewired in T-006.
  - Suggested files:
    - `speccy-cli/src/host.rs` (change
      `HostChoice::ClaudeCode::destination_segments` from
      `[".claude", "commands"]` to `[".claude", "skills"]`; update
      the `HostChoice::ClaudeCode` doc comment to say "destination
      `.claude/skills/`" instead of "destination
      `.claude/commands/`").
    - `speccy-cli/src/embedded.rs` (doc-comment refresh: the
      claude-code pack is now copied to `.claude/skills/<name>/`
      and the codex pack to `.codex/skills/<name>/`; the macro
      invocation itself is unchanged).

- [x] **T-006**: Rewire init tests for the new destinations and add CHK-003 / CHK-004
  - Covers: REQ-002
  - Tests to write:
    - Update every `.claude/commands/speccy/<verb>.md` reference
      in `speccy-cli/tests/init.rs` (search hits at lines 128,
      148, 163, 173, 209, 250, 267, 273) to
      `.claude/skills/speccy-<verb>/SKILL.md`. Same for the
      `--force preserves user files` test at line 163/173 —
      `.claude/commands/my-personal-skill.md` becomes
      `.claude/skills/my-personal-skill/SKILL.md` so the
      assertion still exercises the "user-authored files
      preserved" behaviour at the new path.
    - Update every `.codex/skills/speccy/<verb>.md` reference
      (lines 195, 213, 298, 301) to
      `.codex/skills/speccy-<verb>/SKILL.md`.
    - Update the `include_str!` constants at the top of
      `init.rs` (`SHIPPED_CLAUDE_SPECCY_INIT`,
      `SHIPPED_CODEX_SPECCY_INIT`) to the new bundle paths.
    - `cargo test -p speccy-cli --test init --
      copy_claude_code_pack_skill_md` asserts that after `speccy
      init` runs in a fresh fixture with `.claude/`, every shipped
      skill in `skills/claude-code/` has a byte-identical
      counterpart at `.claude/skills/<name>/SKILL.md` (CHK-003).
    - `cargo test -p speccy-cli --test init --
      copy_codex_pack_skill_md` asserts the same for the Codex
      pack at `.codex/skills/<name>/SKILL.md` (CHK-004).
    - Remove the old `copy_claude_code_pack` and
      `copy_codex_pack` tests, or rename them in place — they
      cover the same behaviour at the new paths and the new
      checks supersede them.
  - Suggested files:
    - `speccy-cli/tests/init.rs`

## Phase 4: SPEC-0002 amendment

- [x] **T-007**: Amend SPEC-0002 REQ-004 and Changelog to point at SPEC-0015
  - Covers: REQ-002
  - Why this REQ: T-007 keeps SPEC-0002's prose honest about
    the install destinations REQ-002 changes. The amendment
    itself adds no new behaviour; it documents the cross-spec
    supersession at the requirement level.
  - Tests to write:
    - No code tests. The amendment is a documentation edit.
    - Manually run `cargo run -p speccy-cli --release -- status`
      after the edit; assert SPEC-0002 still lints clean (no new
      errors or warnings).
  - Suggested files:
    - `.speccy/specs/0002-init-command/SPEC.md` — under REQ-004's
      "Done when" bullets, replace `.claude/commands/` with
      `.claude/skills/<name>/` (Codex destination unchanged).
      Under "Behavior" bullets that name
      `.claude/commands/speccy/plan.md` or
      `.claude/commands/my-personal-skill.md`, update to the new
      paths. Under "Non-goals", replace `.claude/commands/`.
      Append a Changelog row:
      `| 2026-05-14 | agent/claude | REQ-004 destinations updated by SPEC-0015 (Claude Code pack moves from .claude/commands/ to .claude/skills/<name>/; layout changes from flat .md to SKILL.md directory format). Old destination is deprecated; users with prior installs must rm -rf .claude/commands/speccy/. |`

## Phase 5: Verify

- [x] **T-008**: Run the full hygiene sweep
  - Covers: REQ-001, REQ-002, REQ-003, REQ-004
  - Tests to write:
    - This task adds no new tests. It runs the four-command
      pre-commit gate and adjusts any drift the new layout causes:
      `cargo test --workspace`,
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
      `cargo +nightly fmt --all --check`,
      `cargo deny check`.
    - Then runs `./target/release/speccy.exe verify` (or
      `cargo run -p speccy-cli --release -- verify` on platforms
      where the release binary isn't already built) to confirm
      the spec.toml checks under SPEC-0015 all pass.
    - On Windows, prefer running the pre-built release binary
      directly rather than `cargo run -p speccy-cli -- verify`
      from source — the verify driver shells out to `cargo test`
      and cargo cannot replace a running `speccy.exe` on Windows,
      which causes a spurious "file in use" failure. This is
      documented in SPEC-0014 T-007's implementer note.
  - Suggested files: (none directly; iterates as needed)
