---
spec: SPEC-0038
outcome: implemented
generated_at: 2026-05-22T03:00:00Z
---

# REPORT: SPEC-0038 Skill-pack references — per-skill and host-shared reference files eject every lintable artifact's canonical shape

<report spec="SPEC-0038">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-003 extended `embedded.rs`, `init.rs`, and the MiniJinja template context so
that `speccy init` ejects seven reference files into both host packs. Per-skill
files land at `.claude/skills/<skill>/references/<file>.md` (Claude Code) and
`.agents/skills/<skill>/references/<file>.md` (Codex) via wrapper `.md.tmpl`
files that `{% include %}` the canonical source. Host-shared files land at
`.claude/speccy-references/<file>.md` and `.agents/speccy-references/<file>.md`.
The canonical source lives at `resources/modules/references/` (renamed from the
retired `resources/modules/examples/`). The `chk020_no_orphan_references` test
added in T-006 asserts cross-host byte-identical parity and source-to-host
parity for every shipped reference file. `speccy init --force` refreshes the
new directories in place without disturbing user-authored skill files. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
T-003 wired all seven paths from REQ-002's mapping table into `init.rs` and
`embedded.rs`. The host-shared `speccy-references/` directory ships exactly two
files (`evidence.md` and `journal-blockers.md`); each per-skill `references/`
directory ships exactly its attributed file. The mapping classification is
observable from path first-component without a separate manifest.
`.speccy/ARCHITECTURE.md` (updated in T-007) now references REQ-002's table as
the canonical seven-row mapping source. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-007 CHK-008 CHK-009 CHK-010 CHK-011">
T-001 authored all seven canonical reference files under
`resources/modules/references/`. `journal-implementer.md` carries the six
post-SPEC-0034 field labels in order (`Completed:`, `Undone:`, `Hygiene
checks:`, `Evidence:`, `Discovered issues:`, `Procedural compliance:`); the
pre-SPEC-0034 labels `Commands run:` and `Exit codes:` do not appear as
bullet-line prefixes. `evidence.md` opens with `# Evidence for SPEC-0042 T-001`
(matching `^# Evidence for SPEC-\d{4} T-\d{3}$`) and contains no
`<evidence task=` wrapper element. All five remaining files (`spec.md`,
`tasks.md`, `report.md`, `journal-review.md`, `journal-blockers.md`) match their
respective post-SPEC-0034/0035/0037 shapes. No file contains `TBD`, `TODO`, or
`<...>` placeholder substrings. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-012 CHK-013 CHK-014 CHK-015">
T-004 updated all six consuming skill/phase bodies:
`resources/modules/skills/speccy-plan.md` gained a pointer to
`references/spec.md`; `resources/modules/phases/speccy-tasks.md` gained a
pointer to `references/tasks.md` and had its ~20-line inline TASKS.md fragment
replaced; `resources/modules/phases/speccy-ship.md` gained a pointer to
`references/report.md`; `resources/modules/phases/speccy-work.md` gained
pointers to `references/journal-implementer.md` and
`{{ speccy_references_path }}/evidence.md` and had its ~14-line inline
`<implementer>` block removed; `resources/modules/skills/speccy-review.md`
gained pointers to `references/journal-review.md` and
`{{ speccy_references_path }}/journal-blockers.md`;
`resources/modules/skills/speccy-amend.md` gained a pointer to
`{{ speccy_references_path }}/journal-blockers.md`. Post-change body lengths for
`speccy-tasks.md` and `speccy-work.md` are shorter by more than eight lines.
No consuming body inlines an example shape block of eight or more lines for any
reference-shipping artifact. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-016 CHK-017 CHK-018">
T-002 deleted all four orphan paths (`personas/implementer.md`,
`personas/planner.md`, `examples/evidence.md`, and the `examples/` directory).
Salvageable prose was forwarded: `phases/speccy-work.md` received the
feature-flag/abstraction-layer guardrail, suggested-files-hint-may-be-stale
warning, and `<done-when>`/`<behavior>` re-reading reminder from the deleted
`personas/implementer.md`; `skills/speccy-plan.md` received the bounded-scope
guardrail and "decisions hidden inside requirement prose belong in
`### Decisions`" guidance from the deleted `personas/planner.md`. The
`PERSONA_FILES` const in `skill_body_discovery.rs` was updated to remove both
deleted persona paths. `cargo test --workspace` passes with no failures
attributable to the removals. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-019 CHK-020 CHK-021">
T-005 added one pointer line `{{ speccy_references_path }}/evidence.md` to the
"Evidence loading" section of `resources/modules/personas/reviewer-tests.md`.
The rendered Claude Code sub-agent `.claude/agents/reviewer-tests.md` carries
`.claude/speccy-references/evidence.md` exactly once; the rendered Codex
sub-agent `.codex/agents/reviewer-tests.toml` carries
`.agents/speccy-references/evidence.md` exactly once with no occurrence of the
Claude Code form. DEC-001 records that no other reviewer persona gained a
pointer (their prose operates on the diff, not on a reference-shipping artifact
shape). T-005 required one retry: round-1 review (business and tests personas)
found that the initial template variable form was not rendering correctly for the
Codex host; round-2 implementation fixed the template context resolution and
passed all four personas. Retry count: 1.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-022 CHK-023 CHK-024 CHK-025 CHK-026">
T-006 added `chk020_no_orphan_references` to
`speccy-cli/tests/skill_body_discovery.rs`. The test enumerates reference files
via glob (not a hard-coded list), scans consuming bodies including `.toml`
sub-agent files for path-substring pointers, asserts cross-host byte-identical
parity, and asserts source-to-host parity with failure messages naming the
diverging paths and the byte offset of the first difference. The test runs as
part of `cargo test --workspace` and passes against the workspace post-this-SPEC.
Retry count: 0.
</coverage>

</report>
