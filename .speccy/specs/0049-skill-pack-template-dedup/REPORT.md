---
spec: SPEC-0049
outcome: implemented
generated_at: 2026-05-27T20:30:00Z
---

# REPORT: SPEC-0049 Skill pack template dedup — canonical rule bodies stop leaking into wrappers and modules

<report spec="SPEC-0049">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-001 created `resources/modules/skills/speccy-work.md` as the host-neutral canonical skill body, replacing the verbatim retry-shape rule statement and the verbatim reconcile-policy policy table with the DEC-002 invariant formulations. T-002 refactored `resources/modules/skills/speccy-orchestrate.md` to remove three verbatim canonical-rule sites (reconcile-policy include, retry-shape inline body, vet-phases include), replacing each with its DEC-002 invariant text plus a pointer to the canonical reference or canonical-owner skill body. T-003 refactored `resources/modules/skills/speccy-review.md` (reconcile-policy include) and `resources/modules/phases/speccy-work.md` (retry-shape inline body) to the same invariant pattern. After these tasks, grep across `resources/` confirms the distinctive retry-shape rule sentence, the reconcile-policy table rows, and the `### Phase 0 — bootstrap` heading appear only in their canonical/owner files. Retry count: 1 (T-002: 1).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
T-004 ran `just reeject` after the source-side refactor (T-001 through T-003) and audited the ejected output. Post-eject, the verbatim retry-shape rule statement and verbatim reconcile-policy policy table are absent from `.claude/skills/speccy-orchestrate/SKILL.md`, `.claude/skills/speccy-work/SKILL.md`, `.claude/skills/speccy-review/SKILL.md`, `.claude/agents/speccy-work.md`, `.codex/agents/speccy-work.toml`, and all `.agents/` siblings. Each carries DEC-002 invariant text plus a pointer to the host-specific canonical reference path. Canonical-owner exception confirmed: both ejected speccy-vet SKILL.md files retain the full Phase 0/1/2/3 grammar; canonical reference files continue to carry the full rule bodies. Retry count: 1 (T-004: 1).
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-007 CHK-008">
T-004 converted both speccy-work wrapper templates to the pure-include shape per DEC-001(a) — YAML frontmatter plus `{% include "modules/skills/speccy-work.md" %}`. Audit of all skill wrapper templates under `resources/agents/.<host>/skills/<skill>/SKILL.md.tmpl` for both `.claude` and `.agents` hosts confirms no wrapper retains inline canonical rule body text; each conforms to pure-include or stub-delegate per DEC-001. Retry count: 1 (T-004: 1).
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-009">
T-004 ran the standard four-gate hygiene suite after source edits and `just reeject`: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny check` each exit 0. T-006 re-confirmed all four gates pass after the REQ-006 amendment. Retry count: 1 (T-004: 1).
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-010">
T-005 ran a self-dogfood pass using SPEC-0049 itself as the target (the SPEC's authorized fallback). The orchestrator drove the loop from pending tasks through the pre-ship boundary without humans chaining per-task commands; every journal element block parses against the closed-set grammar; `speccy verify` exited 0 at the pre-ship boundary. The reconcile branch was not exercised — accepted residual risk per the SPEC's subjective-comprehension framing. Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-011 CHK-012">
T-006 split the orchestrator's Loop step 2 dispatch enum into five distinct kinds (`work`, `review`, `vet`, `ship`, `decompose`), renamed the former "Ship dispatch" section to "Vet dispatch" (binding the inline speccy-vet workflow to `vet`), and added a new "Ship dispatch" section (binding user-confirm + speccy-ship spawn to `ship`). The Lifecycle ASCII tree, Stop conditions enum, and status reporting examples were updated accordingly. Both ejected files were regenerated via `just reeject` and carry the post-amendment dispatch tree. The invocation-2 simplifier pass applied two post-amendment stale-wording fixes. Retry count: 0.
</coverage>

</report>
