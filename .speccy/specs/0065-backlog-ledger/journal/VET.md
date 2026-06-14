---
spec: SPEC-0065
generated_at: 2026-06-14T03:42:54Z
---

## Invocation 1 — 2026-06-14T03:42:54Z

<drift-review verdict="blocking" round="1" date="2026-06-14T03:42:54Z" model="claude-opus-4-8[1m]">
Backlog wiring matches the SPEC per-requirement (reference, init-immunity, reads, append-on-cut, ship-mirror, strike, bootstrap line all present and ejected with parity), but no producer commits the .speccy/BACKLOG.md mutation, and the ship body contradicts itself about it — so the SPEC's "captured durably, so they outlive the REPORT" promise is not realized.

- REQ-005, user-story-2 ("captured durably, so they outlive the one REPORT.md") → ship step 3 claims the backlog append "lands in the same ship commit (step 6)", but step 6's explicit staging enumeration lists only SPEC.md, TASKS.md, REPORT.md, the .speccy/MEMORY.md mutation, and code changes — .speccy/BACKLOG.md is absent from that list, so an agent following step 6 literally leaves the mirrored entry uncommitted. Internal contradiction in one phase body. See resources/modules/phases/speccy-ship.md:126-127 and resources/modules/phases/speccy-ship.md:150-152 (ejected: .claude/agents/speccy-ship.md, .codex/agents/speccy-ship.toml).
- REQ-004, REQ-006, goal-1 ("recorded ... and retired on promotion") → plan's commit step deliberately narrow-stages only <spec-dir>/SPEC.md and forbids `git add -A`; .speccy/BACKLOG.md is outside <spec-dir>/, so a plan run that appends (REQ-004) or strikes (REQ-006) a backlog entry leaves that mutation uncommitted, and the plan body never instructs committing it. The recorded/retired state does not survive a branch switch. See resources/modules/skills/speccy-plan.md:65-78 (append), resources/modules/skills/speccy-plan.md:95-101 (strike), resources/modules/skills/speccy-plan.md:121-123 (narrow-stage commit).
- REQ-004, user-story-2 → brainstorm calls the backlog append "the one disk write the brainstorm itself performs" but has no commit step and never instructs committing it; the entry rides into the subsequent plan invocation as an uncommitted working-tree change. Same durability gap as the plan path. See resources/modules/skills/speccy-brainstorm.md:187-205.
</drift-review>
<holistic-fix verdict="addressed" round="1" date="2026-06-14T03:48:27Z" model="claude-opus-4-8[1m]">
Summary: Wired every backlog producer to commit its `.speccy/BACKLOG.md` mutation, closing the durability gap across ship, plan, and brainstorm.

Addressed:
- REQ-005, user-story-2 (ship internal contradiction: step 3 promised the backlog append "lands in the same ship commit (step 6)" but step 6's staging enumeration omitted `.speccy/BACKLOG.md`) → added `.speccy/BACKLOG.md` to step 6's explicit staging list in `resources/modules/phases/speccy-ship.md:150-152`, so the enumeration now matches the step-3 promise. Re-ejected with parity to `.claude/agents/speccy-ship.md:285` and `.codex/agents/speccy-ship.toml:284`.
- REQ-004, REQ-006, goal-1 (plan narrow-stages only `<spec-dir>/SPEC.md` and forbids `git add -A`, leaving an appended or struck backlog entry — outside `<spec-dir>/` — uncommitted) → widened the plan staging-breadth parameter in `resources/modules/skills/speccy-plan.md:121-130` to also stage `.speccy/BACKLOG.md` whenever it exists (`test -f .speccy/BACKLOG.md && git add .speccy/BACKLOG.md` — chosen over `git diff --quiet` so a first-append still-untracked file is also caught). The commit-recipe idempotency check (`git diff --cached --quiet`) still fires because SPEC.md is always newly written, so the staged backlog change rides along. Re-ejected with parity to `.claude/skills/speccy-plan/SKILL.md:361` and `.agents/skills/speccy-plan/SKILL.md:361`.
- REQ-004, user-story-2 (brainstorm appends the backlog entry as "the one disk write the brainstorm itself performs" but never commits it, so it rides into the next plan run uncommitted) → brainstorm is intentionally commit-free and ephemeral, so rather than add a commit step there I documented the hand-off: `resources/modules/skills/speccy-brainstorm.md:200-205` now states the writing skill it invokes (`speccy-plan`, step 8) sweeps the dirty `.speccy/BACKLOG.md` into its commit, and the plan staging change above (existence-guarded, not append-conditional) picks up a brainstorm-written entry. Re-ejected to `.claude/skills/speccy-brainstorm/SKILL.md` and `.agents/skills/speccy-brainstorm/SKILL.md`.

Side discoveries:
- Ship's step 6 is a broad bundle commit (code changes included), so `.speccy/BACKLOG.md` would likely have been swept up by a broad `git add` regardless; the fix makes the enumeration honest rather than relying on that, resolving the flagged internal contradiction directly.
- The brainstorm→plan durability chain only holds for the new-SPEC hand-off; an amendment brainstorm routes to `speccy-amend`, but brainstorm only appends future-spec-candidate entries while framing a new slice, so the amend path does not normally produce an uncommitted backlog entry. No action taken; noted for the next reviewer.
- Hygiene suite green: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny check`, plus the targeted `resource_prose_hygiene` suite.
</holistic-fix>
<drift-review verdict="pass" round="2" date="2026-06-14T03:50:43Z" model="claude-opus-4-8[1m]">
Round-1's three blocking durability findings are all genuinely addressed in the current diff with ejected parity, and the per-requirement coverage (REQ-001..007) holds end-to-end; one residual brainstorm-to-amend edge case is flagged for the human, not blocking.

Verified against the current diff (round-2+ scrutiny of each prior "Addressed" claim):
- REQ-005, user-story-2 (ship step3<->step6 contradiction) → resolved: speccy-ship.md:150-152 now stages ".speccy/MEMORY.md mutation and any .speccy/BACKLOG.md mutation from the retro (step 3)", matching the step-3 promise at speccy-ship.md:126; ejected with parity to .claude/agents/speccy-ship.md:284-285 and .codex/agents/speccy-ship.toml.
- REQ-004/REQ-006/goal-1 (plan narrow-stage dropped backlog) → resolved: speccy-plan.md:121-133 stages `test -f .speccy/BACKLOG.md && git add .speccy/BACKLOG.md`, existence-guarded so a first-append untracked file is caught; the commit-recipe idempotency check (commit-recipe.md:44 `git diff --cached --quiet`) fires non-zero because SPEC.md is always newly written, so the staged backlog rides along. Parity at .claude/skills/speccy-plan/SKILL.md:361 and .agents/skills/speccy-plan/SKILL.md:361.
- REQ-004, user-story-2 (brainstorm append uncommitted) → resolved by documented hand-off: speccy-brainstorm.md:204-210 routes the dirty backlog into speccy-plan step 8's commit; new-SPEC chain is sound. Parity at .claude/skills/speccy-brainstorm/SKILL.md:209 and .agents/skills/speccy-brainstorm/SKILL.md:209.

Flagged for human decision (not blocking — outside the SPEC's stated producer set per DEC-002):
- The implementer's carried-forward side discovery is real: brainstorm step 7's append (speccy-brainstorm.md:190-203) is not structurally gated to the new-SPEC path — the body itself notes amendment framings are still brainstorm sessions (speccy-brainstorm.md:130-133). A brainstorm-then-amend session that defers a future-spec candidate appends to .speccy/BACKLOG.md, but speccy-amend stages only SPEC.md/TASKS.md/journal files (speccy-amend.md:124-129, no BACKLOG.md), leaving that entry uncommitted. The reasoned carve-out (future-spec candidates arise while framing a NEW slice, and DEC-002 scopes producers to plan/brainstorm/ship for new-spec framing) is sound, so the human decides whether the amend path warrants a follow-up rather than a re-fix this loop.

No scope creep from the fixes: staging widening is existence-guarded and idempotent, no new CLI flag/API/config key, DEC-001 convention-only posture intact (init.rs:274,310 assert init-immunity and verify-silence). No non-goal violated.
</drift-review>
<simplifier-scan verdict="clean">
No behavior-preserving simplification candidates worth applying in the SPEC-0065 diff.

The only executable code is two new tests in speccy-cli/tests/init.rs
(force_preserves_speccy_backlog_ledger, fresh_init_does_not_create_speccy_backlog_ledger).
They mirror the adjacent memory-ledger tests, already use a host loop rather than
hand-unrolled blocks, and each assertion (sha256 identity, BACKLOG.md absent from the
plan summary, no verify diagnostic) gates a distinct SPEC requirement — none redundant.
The trailing .code(0) after .success() is a deliberate readability echo, not dead code.

Everything else in the diff is shipped resource prose (skill bodies, the new
backlog-ledger.md reference, agents-md conventions), reeject-generated ejected files
under .claude/.agents/.codex, and SPEC/TASKS/journal records — all outside a
behavior-preserving code simplifier's remit (prose is authoring judgment per AGENTS.md,
ejected files are never hand-edited, records are history).
</simplifier-scan>
<gate verdict="passed" tasks_hash="05941e061f855f5bae6e69b66e1ba3149692ce6ac4f86e0686b5e3a7f3f6ffbd" date="2026-06-14T03:52:06Z">
Drift cleared on round 2 (ship/plan/brainstorm now commit the .speccy/BACKLOG.md mutation, closing the durability gap); simplifier scan clean.
</gate>
