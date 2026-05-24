---
spec: SPEC-0042
generated_at: 2026-05-23T17:16:55Z
---

## Invocation 1 — 2026-05-23T17:16:55Z

<drift-review verdict="pass" round="1" date="2026-05-23T17:18:00-04:00" model="claude-opus-4-7[1m]/high">
SPEC-0042 implementation cleanly satisfies all ten requirements end-to-end: `speccy archive` subcommand with status gate, `--force` bypass, `--reason` validation, frontmatter mutation (`archived_at`/`archived_reason`), hash exclusion in `HASH_EXCLUDED_FRONTMATTER_FIELDS`, `git mv` with rollback, JSON receipt envelope (schema_version=1, `null` reason, `[]` warnings), orphan-supersession detection, vacancy union scan across `.speccy/specs/` + `.speccy/archive/`, hot-path invisibility regression tests for status/next/check/verify/lock, `speccy status --include-archive` with text marker + JSON archive fields, and the count-agnostic AGENTS.md + ARCHITECTURE.md doc updates. Workspace compiles clean. No drift between diff and SPEC; CLAUDE.md is a symlink so its AGENTS.md tracking is automatic. SPEC REQ-002 prose says "four allowed-for-archive statuses" while its own `<done-when>` says three — implementation correctly follows `<done-when>` (three: implemented/dropped/superseded); flagging here as informational, not a diff-vs-SPEC drift to fix in code.
</drift-review>

(Note: between round 1 and round 2 the user directed a mid-loop SPEC amendment expanding REQ-010's scope to also rewrite the `## Core principles` "Stay small" item and sweep `README.md` + `docs/ARCHITECTURE.md` of stale `seven-command` / `seven-verb` prose. SPEC.md REQ-010 done-when item 4 was rewritten, a new done-when added, and a Changelog row appended. AGENTS.md, README.md, and docs/ARCHITECTURE.md were edited inline. `speccy lock SPEC-0042` re-recorded the SPEC hash in TASKS.md. Round 2 reviews the post-amendment state.)

<drift-review verdict="blocking" round="2" date="2026-05-23T18:00:00-04:00" model="claude-opus-4-7[1m]/high">
README.md retains a hard-pinned "seven commands" enumeration that mechanically escapes the new done-when rg sweep but contradicts the amendment's stated purpose, and TASKS.md T-008 still claims "Stay small" was left unmodified.
- REQ-010 (amendment behavioral intent + Changelog row) -> README.md lines 29-30 "Seven flat commands", lines 33-41 a 7-row CLI table missing `archive`, line 45 "the seven CLI verbs above" all pin a specific command count and will be factually wrong once `archive` ships. The literal `rg 'seven[- ]command|seven[- ]verb|seven commands|seven verbs'` regex returns zero matches only because the prose interleaves words ("Seven flat commands" -> "Seven flat" / "flat commands", "seven CLI verbs"), so the new done-when item passes mechanically while the amendment's stated motivation ("the next CLI verb that lands would re-introduce the same churn the SPEC is trying to eliminate") is unsatisfied. See `README.md:29-31`, `README.md:33-41`, `README.md:45`.
- REQ-010 (TASKS-vs-SPEC reconciliation after amendment) -> TASKS.md T-008 body still says "The `## Core principles` 'Stay small' item is left unmodified" even though the amendment requires rewriting that item (and the implementation correctly did so). A future re-reader of TASKS.md will see instructions that contradict both the SPEC and the actual diff. See `TASKS.md:729`.
</drift-review>

<holistic-fix verdict="addressed" round="2" date="2026-05-23T18:30:00-04:00" model="claude-opus-4-7[1m]/low">
Summary: Rewrote README.md's count-pinned CLI prose and corrected TASKS.md T-008's outdated instruction so the SPEC, tasks, and docs all agree.

Addressed:
- REQ-010 (README.md still pins "Seven flat commands" / "seven CLI verbs above" / 7-row table missing `archive`) → Rewrote README.md:29-32 to "deliberately lean — a small set of flat commands… The current command list lives in `docs/ARCHITECTURE.md`"; added a `speccy archive` row to the representative CLI table (README.md:42); changed the trailing phrase from "the seven CLI verbs above" to "the CLI verbs above" (README.md:46). The broader sweep `rg -n '[Ss]even[- ]?(?:command|verb|flat command|CLI verb)' README.md` and case-insensitive `seven` both return zero matches.
- REQ-010 (TASKS.md T-008 contradicts the amendment by claiming "Stay small" is left unmodified) → Rewrote that bullet (TASKS.md:729-731) to state the "Stay small" item is also rewritten to drop the literal `seven commands` integer while keeping its substance.

Hygiene: `cargo test --workspace` green, `cargo clippy --workspace --all-targets --all-features -- -D warnings` green, `cargo +nightly fmt --all --check` green, `cargo deny check` green, `speccy verify` clean.
</holistic-fix>

<drift-review verdict="pass" round="3" date="2026-05-23T18:45:00-04:00" model="claude-opus-4-7[1m]/high">
Round-2 fix closes both findings. README.md no longer pins a specific command count: the "Seven flat commands" prose is now "deliberately lean — a small set of flat commands… The current command list lives in `docs/ARCHITECTURE.md`" (README.md:29-32), the representative CLI table includes a `speccy archive` row (README.md:42), and the trailing phrase reads "the CLI verbs above" (README.md:46); README.md "Stay small" point 5 reads "Five nouns… a small flat command surface" with no integer (README.md:408-410). TASKS.md T-008 body (lines 729-731) now explicitly states the `## Core principles` "Stay small" item is also rewritten to drop the literal `seven commands` integer. REQ-010's done-when gate `rg -n 'seven[- ]command|seven[- ]verb|seven commands|seven verbs' AGENTS.md README.md docs/ARCHITECTURE.md` returns zero matches. The one residual `seven-row` hit in `docs/ARCHITECTURE.md:1803` refers to a SPEC-0038 reference-file row mapping (artifact→path), not the CLI command count, and is correctly outside the REQ-010 sweep. CLAUDE.md is a symlink to AGENTS.md so the AGENTS.md edits carry through automatically. No new drift introduced by the fix.
</drift-review>

<simplifier-scan verdict="clean">
No behavior-preserving simplifications worth applying; the SPEC-0042 diff already follows project conventions (defensive validation at boundaries, `OnceLock` regex caching, `must_use` on receipts, `#[expect(reason=...)]` for unwraps on compile-time literals, `fs_err` + `camino` everywhere). Notable items considered and rejected: the `to_forward_slash` redundant-looking `MAIN_SEPARATOR == '/'` guard actually preserves Unix paths containing literal backslashes; the `assemble` thin wrapper over `assemble_with_archive` keeps the existing stable public API; the `strip_prefix("SPEC-")` ok_or_else branch in `locate_spec_dir` is unreachable-after-validation but is the standard defensive pattern called out in `.claude/rules/rust/rust-defensive-programming.md`; `allocate_next_spec_id` kept as a one-line wrapper preserves existing call sites.
</simplifier-scan>

<gate verdict="passed" tasks_hash="7b9203f4306c437ca9478af0d6005c0ac9447ad4e9dee2c969579c65e7dbfb84" date="2026-05-23T17:32:28Z">
Drift cleared after a mid-loop SPEC amendment broadened REQ-010 to sweep README.md and docs/ARCHITECTURE.md stale count prose; one round of drift fix addressed both findings; simplifier scan returned clean. Ready to ship.
</gate>
