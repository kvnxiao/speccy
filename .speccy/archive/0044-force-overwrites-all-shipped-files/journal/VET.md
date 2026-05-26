---
spec: SPEC-0044
generated_at: 2026-05-24T23:39:57Z
---

## Invocation 1 — 2026-05-24T23:39:57Z

<drift-review verdict="pass" round="1" date="2026-05-24T23:50:00Z" model="claude-opus-4-7[1m]/high">
SPEC-0044 carve-out removal is implemented end-to-end: `is_host_native_reviewer_file` and the `PERSONAS_ALL` import are gone, `classify_content` runs uniformly for every host-pack file, `Skip-on-exists` is purged from `docs/ARCHITECTURE.md` and `speccy-cli/src/init.rs`, the two preserve-tests are flipped to assert overwrite (CHK-001/CHK-002 behaviour confirmed for Claude `reviewer-business.md` and Codex `reviewer-business.toml`), the plan-summary inversion test is deleted, the recreate-when-deleted test is renamed, and `cargo test --workspace` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` both pass clean. Minor SPEC-vs-impl naming variance worth a glance but not blocking: CHK-002 in SPEC.md names `.codex/agents/reviewer-security.toml` as the Codex sentinel target, but `t002_codex_reviewer_agent_files_overwrite_user_edits_under_force` at `speccy-cli/tests/init.rs:677` instead exercises `.codex/agents/reviewer-business.toml`; the uniform classification path is structural so behaviour is equivalent across personas, but both behavioural reviewer tests now fix the persona dimension to `business`, dropping the cross-persona spot check CHK-002 was deliberately written for. Easy follow-up if you want literal scenario alignment; otherwise pass.
</drift-review>

<simplifier-scan verdict="clean">
SPEC-0044 working diff (init.rs, ARCHITECTURE.md, tests/init.rs) is itself a simplification — it deletes the carve-out helper and uniformizes classification. Residual candidates (defensive `if force` in `execute_plan::Conflict`, claude/codex test asymmetry where codex bundles overwrite+recreate while claude splits them) either weaken defense-in-depth or require expanding the diff to refactor adjacent test structure. No clean wins.
</simplifier-scan>

<gate verdict="passed" tasks_hash="741f39e8ddef96eb4117be9f23760bd6098a5861fe98c0aee54580d29636af14" date="2026-05-24T23:55:00Z">
Drift review pass on round 1; simplifier scan clean; no implementer or revert needed.
</gate>

## Invocation 2 — 2026-05-24T23:48:13Z

<drift-review verdict="pass" round="1" date="2026-05-25T00:05:00Z" model="claude-opus-4-7[1m]/high">
Invocation 1's sole advisory — CHK-002 in SPEC.md naming `.codex/agents/reviewer-security.toml` while the Codex behavioural test exercised `reviewer-business.toml` — is resolved at `speccy-cli/tests/init.rs:677-733`: the test now reads/edits/asserts against `.codex/agents/reviewer-security.toml`, the sentinel-overwrite + byte-equality + `(!) overwritten` substring assertions all reference that path, and the recreate-on-delete tail asserts `name = "reviewer-security"` in the restored Codex frontmatter (matching the Codex bundle for that persona). The cross-persona spot check CHK-002 was written for is back. Carve-out removal still holds end-to-end: `is_host_native_reviewer_file`, the `PERSONAS_ALL` import in `init.rs`, and every `Skip-on-exists` literal are absent from `speccy-cli/src/init.rs` and `docs/ARCHITECTURE.md`; `cargo test --workspace --test init t002_` runs three tests green. REQ-001 / CHK-001 / CHK-002 / CHK-003 and REQ-002 / CHK-004 / CHK-005 / CHK-006 all satisfied as a unit. No scope creep, no user-story gap, no new drift.
</drift-review>

<simplifier-scan verdict="clean">
SPEC-0044 diff is skill-pack/agent-config markdown and TOML with no behavior-bearing code; the only delta since invocation 1's clean scan is a single test path-name fix (reviewer-business.toml → reviewer-security.toml), which has no simplification surface.
</simplifier-scan>

<gate verdict="passed" tasks_hash="741f39e8ddef96eb4117be9f23760bd6098a5861fe98c0aee54580d29636af14" date="2026-05-24T23:48:13Z">
Re-vet after CHK-002 path fix; drift pass round 1; simplifier clean.
</gate>
