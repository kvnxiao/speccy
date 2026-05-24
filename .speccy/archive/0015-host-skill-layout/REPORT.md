---
spec: SPEC-0015
outcome: delivered
generated_at: 2026-05-14T23:55:00Z
---

# SPEC-0015: Host skill packs use SKILL.md directory format

## Outcome

**delivered** — all four requirements satisfied; all 8 tasks closed
on the first pass with zero retries and zero SPEC amendments. Both
shipped host packs migrated from the flat `<verb>.md` layout to the
canonical `<skill-name>/SKILL.md` directory format. The Claude Code
pack now installs to `.claude/skills/` (was `.claude/commands/`) and
the Codex pack installs to `.agents/skills/` (was `.codex/skills/`),
both per their vendors' documented project-local scan paths. Each
SKILL.md carries `name` + `description` frontmatter rewritten for
natural-language activation.

<report spec="SPEC-0015">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003 CHK-004">
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-005">
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-006">
</coverage>

</report>

## Task summary

- **Total tasks:** 8 (T-001 through T-008).
- **Retried tasks:** 0.
- **SPEC amendments:** 0.

Phase 1 (T-001, T-002) restructured the bundle and added layout
tests. Phase 2 (T-003, T-004) rewrote descriptions and added the
frontmatter/description-quality tests. Phase 3 (T-005, T-006) moved
the Claude Code install destination from `.claude/commands/` to
`.claude/skills/` and rewired the existing `init.rs` integration
tests to the new SKILL.md paths. Phase 4 (T-007) amended SPEC-0002
REQ-004's prose to point at this spec's new destinations. Phase 5
(T-008) ran the four-command hygiene gate plus `speccy verify`
against the release binary.

## Out-of-scope items absorbed

Edits made during the loop that were not part of the planned task
scope but were necessary for the work to land cleanly:

- **SPEC-0002 REQ-004 amendment (T-007)** — the planned scope of
  this task was a single Changelog row plus a wording update on
  REQ-004's destinations. T-007's implementer additionally swept
  the Behavior and Non-goals bullets that named
  `.claude/commands/speccy/plan.md` and similar paths so the
  cross-spec story stays internally consistent. Documented in the
  appended SPEC-0002 Changelog row dated 2026-05-14.
- **`speccy-cli/tests/init.rs` path rewires (T-006)** — beyond the
  enumerated line numbers in TASKS.md, the implementer touched
  every `include_str!` constant at the top of the file
  (`SHIPPED_CLAUDE_SPECCY_INIT`, `SHIPPED_CODEX_SPECCY_INIT`) to
  retarget the new bundle paths. Pure mechanical follow-on of the
  bundle restructure in T-001; no test behaviour changes.
- **Legacy `.claude/commands/speccy/` directory removal at the
  repo root** — the migration note in the SPEC required removing
  the directory by hand in the same PR; absorbed during T-005 along
  with the destination flip.

## Skill updates

(none)

This spec edits the bundle *layout* and the SKILL.md *frontmatter*
across all 14 shipped files (7 verbs × 2 hosts), but does not edit
any `skills/**` content in response to in-flight friction. Every
`Procedural compliance` line across T-001..T-008 reads `(none)`.

## Deferred / known limitations

- **Open question 1: secondary `.codex/skills/` destination** —
  deliberately deferred per DEC-002. OpenAI's published Codex docs
  list `.agents/skills/` as the project-local scan path; the
  openai/codex repo itself uses `.codex/skills/` for its internal
  dogfood, but that reads as CLI-self-development guidance rather
  than the documented user-facing convention. Ship to the documented
  path; revisit only if a Codex install in practice fails to scan
  `.agents/skills/`. SPEC-0016 inherited this deferral and added
  reviewer subagent infrastructure under both paths without
  reopening the question.
- **`/speccy:<verb>` colon-namespaced slash command form** —
  deliberately retired per DEC-003 / Non-goals. Users typing
  `/speccy:plan` after upgrade get "unknown command"; the
  replacement is `/speccy-plan` (hyphen). The colon namespace
  requires a Claude Code plugin manifest, which is out of v1 scope.
- **Codex natural-language activation reliability** — the SPEC's
  user stories claim Codex's skill selector surfaces `speccy-plan`
  when the user says "draft a spec for X." That's the documented
  behaviour but real-world reliability is a known unknown only
  surfaceable through dogfooding. SPEC-0016 picked up the related
  "Codex prose-spawn reliability" question for reviewer fan-out;
  the same caveat applies to skill discovery.
