---
spec: SPEC-0044
spec_hash_at_generation: aa9f48b9e95d15e81774c174809f9f57647c5bafb60339535375c7e0103f1e9b
generated_at: 2026-05-24T23:30:07Z
---
# Tasks: SPEC-0044 `speccy init --force` overwrites all shipped files; reviewer-persona carve-out is removed

<task id="T-001" state="completed" covers="REQ-001 REQ-002">
## Remove the reviewer-persona carve-out from `speccy init --force`

Delete the `is_host_native_reviewer_file` helper and its caller
branch in `append_host_pack_items` (`speccy-cli/src/init.rs`), so
every rendered host-pack file is classified by `classify_content`
uniformly. The `use speccy_core::personas::ALL as PERSONAS_ALL;`
import at `init.rs:19` was the sole consumer of `personas::ALL` in
this file and goes with the helper; `personas::ALL` itself stays
(consumed by `speccy-core/src/next.rs`,
`speccy-core/src/parse/journal_xml/mod.rs`, and integration tests
outside `init.rs`).

Rewrite the carve-out-related prose in `init.rs` to describe one
uniform rule:

- Module docstring at `init.rs:9-10` — drop the "(T-008: three-way
  classification replacing Skip-on-exists)" parenthetical or restate
  it without the `Skip-on-exists` term.
- `Action` enum docstring at `init.rs:74-88` — delete the
  "Host-native reviewer files… are user-customisable and classified
  `Action::Unchanged` when they already exist (regardless of byte
  equality)" paragraph entirely.
- `Action::Unchanged` variant docstring at `init.rs:93-95` — drop
  the "(or is a user-tunable reviewer file that is Skip-on-exists)"
  parenthetical.
- `build_plan` comment at `init.rs:206-209` — rewrite so it no
  longer claims reviewer agent files are classified Skip-on-exists.
- `execute_plan` comment at `init.rs:328` — drop the "or reviewer
  Skip-on-exists" suffix.

Update `docs/ARCHITECTURE.md` in three places:

- Lines 323-327 ("There is no project-local persona override…")
  — drop the "classifies them Skip-on-exists so local edits to a
  persona's body survive `speccy init --force`" clause.
- Lines 1680-1685 ("Projects edit the host-native sub-agent file
  in place…") — drop the "speccy init classifies those files
  Skip-on-exists so a local edit survives speccy init --force"
  trailing sentence.
- Lines 1887-1890 ("Recovery from an unwanted overwrite is via
  git checkout…") — drop the trailing "Host-native reviewer files
  (`.claude/agents/reviewer-<persona>.md` and the Codex twin) remain
  Skip-on-exists so local persona-body edits survive `--force`"
  sentence so the three-way rule reads uniformly.

Flip the two tests that today assert preservation:

- `t002_claude_reviewer_agent_files_preserve_user_edits_under_force`
  in `speccy-cli/tests/init.rs:599-638` — rename to
  `t002_claude_reviewer_agent_files_overwrite_user_edits_under_force`,
  invert the final `assert!(after.ends_with(sentinel), …)` so it
  asserts the sentinel is gone and the body matches the shipped
  bundle, and update the SPEC-0027 REQ-002 attribution to SPEC-0044
  REQ-001 / CHK-001.
- `t002_codex_reviewer_agent_files_preserve_user_edits_under_force`
  in `speccy-cli/tests/init.rs:723-?` — same rename, same inversion
  for the `.codex/agents/reviewer-business.toml` path, retarget
  attribution to SPEC-0044 REQ-001 / CHK-002.

Revise or delete the plan-summary test:

- `t002_claude_init_force_plan_summary_marks_reviewer_agents_and_skills_unchanged`
  at `init.rs:670-720` — the inversion check ("must NOT contain
  `(!) overwritten` for reviewer paths") no longer holds. Either
  delete the test (its remaining assertion — that reviewer paths
  appear in the plan summary at all — is redundant with the new
  CHK-001/CHK-002 scenarios) or rewrite it to assert that
  byte-identical reviewer files show `unchanged` while files
  differing from the bundle show `(!) overwritten`. Pick whichever
  keeps the file smaller.

Update the recreate-on-delete test for naming/attribution only:

- `t002_claude_reviewer_agent_files_recreate_when_deleted_under_force`
  at `init.rs:641-667` — the Create-on-absent behaviour is unchanged
  (deletion still triggers a fresh render). Drop the "Skip-on-exists"
  framing from its inline comment, retarget the SPEC-0027 attribution
  to SPEC-0044 REQ-001 (Create-on-absent is universal under the new
  uniform rule), and rename to
  `t002_claude_reviewer_agent_files_recreate_when_deleted` (drop the
  `_under_force` suffix; the behaviour holds without `--force` too
  since the file is absent).

Run the standard hygiene gate after the changes land:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

<task-scenarios>
Given a freshly created temporary directory `root`,
when `speccy init --host claude-code` runs with `cwd = root` and
exits 0, the bytes `\n# sentinel-edit-12345\n` are appended to
`root/.claude/agents/reviewer-business.md`, and `speccy init --host
claude-code --force` then runs with `cwd = root`,
then the command exits 0,
and `root/.claude/agents/reviewer-business.md` no longer contains
the substring `sentinel-edit-12345`,
and the captured stdout contains a line whose trimmed start matches
`(!) overwritten` followed by whitespace and the relative path
`.claude/agents/reviewer-business.md`.

Given a freshly created temporary directory `root`,
when `speccy init --host codex` runs with `cwd = root` and exits 0,
the bytes `\n# sentinel-edit-67890\n` are appended to
`root/.codex/agents/reviewer-security.toml`, and `speccy init
--host codex --force` then runs with `cwd = root`,
then the command exits 0,
and `root/.codex/agents/reviewer-security.toml` no longer contains
the substring `sentinel-edit-67890`,
and the captured stdout contains a line whose trimmed start matches
`(!) overwritten` followed by whitespace and the relative path
`.codex/agents/reviewer-security.toml`.

Given the source file `speccy-cli/src/init.rs` after this task
lands,
when grepped for the identifier `is_host_native_reviewer_file`,
then no match is found,
and when grepped for the literal substring `Skip-on-exists`,
then no match is found,
and when grepped for `use speccy_core::personas::ALL as PERSONAS_ALL`,
then no match is found.

Given the file `docs/ARCHITECTURE.md` after this task lands,
when grepped for the literal substring `Skip-on-exists`,
then no match is found,
and when scanned with two lines of context around any mention of
`reviewer-`, then no surviving prose claims that user edits to
those files survive `--force`.

Given the workspace after this task lands,
when `cargo test --workspace` runs,
then it exits 0,
and when `cargo clippy --workspace --all-targets --all-features
-- -D warnings` runs,
then it exits 0 with no `dead_code` or `unused_imports` warnings
attributable to the carve-out removal,
and when `cargo +nightly fmt --all --check` runs,
then it exits 0.

Suggested files: `speccy-cli/src/init.rs`,
`speccy-cli/tests/init.rs`, `docs/ARCHITECTURE.md`
</task-scenarios>
</task>
