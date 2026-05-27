---
spec: SPEC-0047
generated_at: 2026-05-27T00:00:00Z
---

## Invocation 1 — 2026-05-27T00:00:00Z

<drift-review verdict="pass" round="1" date="2026-05-27T00:00:00Z" model="claude-opus-4-7[1m]/medium">
Diff implements all five SPEC-0047 REQs without scope creep: retry-shape reference shipped to all three host locations byte-identically; `/speccy-work` and `/speccy-orchestrate` carry the marker-bounded retry-aware precondition; `/speccy-work` agent prompt grows the retry-mode branch with N+1 round increment and explicit no-`git restore`/`clean`/`checkout` policy; `/speccy-decompose` gains the narrow-stage idempotent bootstrap commit step with HEREDOC body; `journal-blockers.md` and `/speccy-amend` documentation aligned to the `<blockers round="N">` convention with no surviving `round="N+1"` or "retry as" wording. Marker comments appear exactly once per consumer site; inlined rule content matches the canonical reference verbatim across `.claude`, `.agents`, `.codex`, and `resources/modules` mirrors.
</drift-review>

<simplifier-scan verdict="clean">
SPEC-0047 code changes are tight; the two new shared-marker helpers are appropriately scoped to their respective test crates and no behavior-preserving simplification is worth applying without expanding diff surface.
</simplifier-scan>

<gate verdict="passed" tasks_hash="ea260d49dc7adc8ae08f961169e2cca3d5ea330f5c6064bde43697dccef14e9d" date="2026-05-27T00:00:00Z">
Drift review pass on round 1; simplifier scan clean. SPEC-0047 ready for ship.
</gate>
