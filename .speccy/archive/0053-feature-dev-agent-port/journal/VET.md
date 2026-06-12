---
spec: SPEC-0053
generated_at: 2026-05-29T20:14:04Z
---

## Invocation 1 — 2026-05-29T20:14:04Z

<drift-review verdict="pass" round="1" date="2026-05-29T20:31:47-07:00" model="claude-opus-4-8[1m]/high">
SPEC-0053 is satisfied as a unit: all ten requirements' done-when bullets and scenarios (CHK-001..014) are met, the five-persona default fan-out / seven-total registry is wired through `personas::ALL` + `default_personas()` + `parse/journal_xml` validation + `review-fanout.md` + rendered review/orchestrate skills, the three new personas render host-neutral to both hosts with includes expanded and `opus[1m]`/`gpt-5.5` models (no `sonnet`), all ten read-only wrappers carry the read-only `tools:` grant while the five writers retain full access, the Codex-parity non-support is recorded in AGENTS.md, `can we brainstorm` is in both rendered descriptions, and `cargo test`/`clippy`/`+nightly fmt --check`/`deny check` plus idempotent `just reeject` all pass at HEAD; no non-goal is violated (no new commands, no new artifact files, confidence-≥80 gate scoped to correctness only, no parallel fan-out config), and the unplanned `speccy-work.md`/`speccy-orchestrate` SKILL edits are legitimate (REQ-005 fan-out doc-sync and an AGENTS.md-endorsed friction-driven `</implementer>` close-tag fix). Two non-blocking quality nits for the human's discretion, surfaced rather than blocked since neither alters behavior, fails a scenario, or breaks a done-when: (1) cross-task consistency artifact this SPEC introduced — the `// Six wrappers, one per shipped reviewer persona` comment is now stale after the persona count moved to seven (the adjacent `REVIEWER_PERSONAS` const and the `exactly seven ... reviewer wrappers` assertion message, both edited by this SPEC, say seven). See speccy-cli/tests/skill_packs.rs:972 and speccy-cli/tests/skill_packs.rs:1523. Already self-flagged in journal/T-001.md as a follow-up. (2) REQ-006's prose says brainstorm/plan invoke `plan-explorer` "unconditionally," but the shipped wiring gates on "when the ask touches existing code" — a sensible greenfield refinement that still satisfies every REQ-006 done-when bullet and CHK-008. See resources/modules/skills/speccy-brainstorm.md and resources/modules/skills/speccy-plan.md.
</drift-review>

<simplifier-scan verdict="candidates">
One genuine test-helper triplication in the SPEC-0053 additions to `speccy-cli/tests/skill_packs.rs`; everything else is mechanical count bumps and prose left alone.

- `speccy-cli/tests/skill_packs.rs:119`, `:207`, `:267` — `rendered_correctness_body`, `rendered_plan_explorer_body`, and `rendered_plan_architect_body` are three byte-for-byte-identical 11-line helpers differing only in the agent name baked into `let rel = format!("{dir}/agents/<name>.{suffix}")`. All three were added by this diff, in a file the diff already modifies. Collapse them into a single `rendered_agent_body(host, dir, name, suffix) -> String` and pass the persona name at the three callsites. Behavior-preserving (same render, same panic-on-missing path, same owned `String` return); removes ~22 lines of duplication. Do not fold in `find_rendered_agent` (line 2146) — it has a different signature (borrows an already-rendered slice, returns `&str`) and supports a render-once/lookup-many loop, so merging would regress that callsite. Leave `rendered_decompose_body` (line 361) separate too: it keys on `install_root` (`.agents` vs `.codex`) rather than a fixed dir and is not identical.
</simplifier-scan>

<simplifier-apply verdict="applied">
Collapsed the three byte-identical helpers in `speccy-cli/tests/skill_packs.rs` into one `rendered_agent_body(host, dir, name, suffix)`, updated all six callsites; all four hygiene gates pass.
</simplifier-apply>

<gate verdict="passed" tasks_hash="53134f99b9e632234769f467adcd5914b05a260632b1a7e945fb20475bdd7c46" date="2026-05-29T20:22:13Z">
Drift cleared on round 1; simplifier collapsed three duplicate test helpers and applied with all hygiene gates green. SPEC-0053 holistically matches its implementation.
</gate>
