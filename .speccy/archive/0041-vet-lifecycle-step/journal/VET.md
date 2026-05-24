---
spec: SPEC-0041
generated_at: 2026-05-23T03:27:42Z
---

## Invocation 1 — 2026-05-23T03:27:42Z

<drift-review verdict="blocking" round="1" date="2026-05-23T03:31:33Z" model="claude-opus-4.7[1m]/high">
Two blocking gaps against SPEC-0041 done-when criteria plus a hygiene-gate failure.
- REQ-004 done-when 3 → `rg -n 'VET\.md' resources/modules/skills/speccy-orchestrate.md` returns zero matches; the done-when requires at least one match per file (paired with `speccy-vet.md`, which has many). See resources/modules/skills/speccy-orchestrate.md (no VET.md mention anywhere).
- AGENTS.md "Standard hygiene" / SPEC-0041 T-001+T-002 hygiene gate → `cargo test --workspace` fails on `architecture_md_documents_xml_element_grammar` and `architecture_md_pins_no_public_speccy_fmt`. Both tests join `.speccy/ARCHITECTURE.md` but the file lives at `docs/ARCHITECTURE.md` after commit af84caf. See speccy-core/tests/docs_sweep.rs:29 and :54.
- Side observation (resolved by `speccy init --force` during this round) → committed `.claude/skills/speccy-vet/SKILL.md` and `.agents/skills/speccy-vet/SKILL.md` were stale relative to the renderer output (`dogfood_outputs_match_committed_tree` panicked). Re-running init regenerated both; leaving the working-tree edits uncommitted alongside the drift fixes.
</drift-review>

<holistic-fix verdict="addressed" round="1" date="2026-05-23T03:33:00Z" model="claude-opus-4.7[1m]/low">
Summary: addressed both blocking findings; hygiene suite is green; ejected packs re-rendered.

Addressed:
- REQ-004 done-when 3 (VET.md in speccy-orchestrate) → added "(and, for the holistic gate, in `.speccy/specs/NNNN-slug/journal/VET.md`)" parenthetical to the Status reporting / journals paragraph in resources/modules/skills/speccy-orchestrate.md (line 238). Re-ran `speccy init --force --host claude-code` and `... --host codex` so the ejected `.claude/skills/speccy-orchestrate/SKILL.md` and `.agents/skills/speccy-orchestrate/SKILL.md` carry the same mention (line 235 / 236 respectively).
- Hygiene-gate failure (docs_sweep paths) → fixed `speccy-core/tests/docs_sweep.rs:29` and `:54` to join "docs"/"ARCHITECTURE.md" instead of ".speccy"/"ARCHITECTURE.md". The two doc-comment headers at lines 9 and 11 already said `docs/ARCHITECTURE.md`; only the path literals were stale from commit af84caf.
- Side discovery (dogfood drift) → resolved as a no-op by `speccy init --force` during the diagnosis; `.claude/skills/speccy-vet/SKILL.md` and `.agents/skills/speccy-vet/SKILL.md` are now in sync with the renderer template at `resources/modules/skills/speccy-vet.md`.

Hygiene gate:
- `cargo test --workspace` → ok (no failures across all packages).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean.
- `cargo +nightly fmt --all --check` → silent (no diff).
- `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok (only stale-license-allowance warnings, all pre-existing).
- `cargo run --bin speccy -- verify` → 0 errors, 20 warnings, 59 info across 41 specs.
</holistic-fix>

<drift-review verdict="pass" round="2" date="2026-05-23T03:34:01Z" model="claude-opus-4.7[1m]/high">
Round 1's two blocking bullets are addressed against the current working-tree diff and the side observation is resolved.
- REQ-004 done-when 3 verified: VET.md now appears in resources/modules/skills/speccy-orchestrate.md:238 and propagated to both ejected copies via the renderer.
- Hygiene gate confirmed green across all four standard gates plus `speccy verify`; `dogfood_outputs_match_committed_tree` now passes.
- Walked the remaining REQ-001..REQ-006 done-when criteria against the diff as a unit: zero `speccy-holistic-gate` / `HOLISTIC.md` literals outside the permitted SPEC-0039 + SPEC-0041 history paths; all nine `vet-*` persona files exist with non-empty bodies; `code-simplifier` dispatch sites are absent from the three speccy-vet bodies; `/speccy-ship SPEC-NNNN` survives in `resources/modules/skills/speccy-vet.md:572` (post-pass chain) while removed from `speccy-review.md` and `speccy-work.md`; `speccy-vet SPEC-NNNN` present in both of those bodies (review:183, work:125). `speccy next --json` for SPEC-0041 returns `kind="vet"` as designed for an unvetted spec.
</drift-review>

<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates worth applying in the cumulative SPEC-0041 diff against main. The substantive Rust additions (`NextAction::Vet`, `vet_gate_is_fresh_pass`, `last_gate_block`, `attribute_value`, `sha256_hex`) are already idiomatic — single responsibility per function, `let Some(..) else` chains for `Option` flow, no `unwrap`/`expect` in production code, doc comments on every helper. The skill / persona / template diffs are predominantly renames + small content updates that match existing patterns in the repo. Tightening would trade local clarity for fewer lines.
</simplifier-scan>

<gate verdict="passed" tasks_hash="5782811f867690ba3ee377d6a848c285e8c6431a47441a7def4beeca28a97231" date="2026-05-23T03:35:15Z">
Drift cleared on round 2 after fixing REQ-004 done-when 3 (VET.md mention in speccy-orchestrate) and the unrelated docs_sweep test path regression that was blocking the hygiene gate; simplifier scan clean; all four standard hygiene gates green plus `speccy verify` 0 errors.
</gate>

