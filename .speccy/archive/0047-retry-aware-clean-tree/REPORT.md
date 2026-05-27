---
spec: SPEC-0047
outcome: implemented
generated_at: 2026-05-27T00:00:00Z
---

# REPORT: SPEC-0047 Retry-aware clean-tree precondition — work dispatch tolerates dirty trees on review-blocked retries

<report spec="SPEC-0047">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-001 created the canonical retry-shape reference at
`resources/modules/references/retry-shape.md` and regenerated the
host-portable mirrors at `.claude/speccy-references/retry-shape.md` and
`.agents/speccy-references/retry-shape.md`. The file documents the rule
statement, the read-only scope constraint, and three worked examples
(retry shape, first-attempt shape, and the awaiting-review edge case
where the highest implementer round has no matching blockers block).
The reference landed atomically with the first `/speccy-work` consumer
in the same commit, satisfying the `chk022_no_orphan_references` lint
(CHK-001/CHK-002/CHK-003 traceable to the worked examples in the
canonical source). Retry count: 4.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
T-001 (Part B) extended `.claude/skills/speccy-work/SKILL.md` and its
`.agents/` mirror with the retry-aware entry precondition: resolve
target task, read the journal and apply the retry-shape rule, then
run `git status --porcelain`. The first-attempt branch retains the
original strict dirty-paths halt surface; the retry branch proceeds
to dispatch without halting. T-003 applied the identical three-step
precondition to `.claude/skills/speccy-orchestrate/SKILL.md` and
mirrors, extending the work dispatch before the speccy-work sub-agent
spawn. Both skill bodies carry the rule text verbatim between the
`<!-- Shared rule: retry-shape. -->` marker pair, byte-identical to
the canonical reference after whitespace normalisation. The
reconcile-policy partial inline was left untouched at its existing
location (CHK-004 first-attempt branch, CHK-005 retry branch,
CHK-006 orchestrator retry dispatch all traceable to the
two-branch precondition prose). Retry count: 4 (T-001 Part B) / 0 (T-003).
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-007 CHK-008">
T-004 inserted a retry-shape check as the new step 2 in
`.claude/agents/speccy-work.md` and its Codex variant
(`.codex/agents/speccy-work.toml`) plus the `resources/agents/...`
template sources. The first-attempt branch continues today's flow
unchanged; the retry branch reads the most recent `<implementer>`
block for prior completed-work context, reads the latest `<blockers>`
block, amends WIP in place (explicit prohibition on `git restore`,
`git clean`, `git checkout`), routes through the SPEC-0045/REQ-001
hygiene gate, and appends `<implementer round="N+1">` with
`Completed` describing the amend. The six-field handoff template and
CHK roll-call convention are unchanged. The `When to use` prose was
updated to note automatic retry-shape detection. The rule text is
byte-identical to the canonical source and the T-001/T-003 inlines
(CHK-007/CHK-008 traceable to the retry-branch prose). Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-009 CHK-010 CHK-011">
T-005 added the bootstrap commit step to `.claude/agents/speccy-decompose.md`
and its `.agents/` mirror and `resources/agents/...` template sources.
The step runs after `speccy lock SPEC-NNNN` and before the
"Suggest the next step" line: narrow `git add <spec-dir>/SPEC.md
<spec-dir>/TASKS.md` stages exactly the two SPEC artefacts; `git diff
--cached --quiet` makes the step idempotent on re-runs (exit 0 → skip
silently); the commit uses a HEREDOC with title
`[SPEC-NNNN]: create spec and decompose tasks`, the SPEC's `title:`
frontmatter value as body, and the host-harness model identifier
(`Speccy Skill Pack` fallback) as the `Co-Authored-By` trailer.
Idempotency and narrow-staging scope documented inline per the task
body (CHK-009 commit shape, CHK-010 narrow staging, CHK-011
idempotency skip all traceable to the agent prompt steps). Retry count: 1.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-012 CHK-013 CHK-014">
T-006 rewrote the `round` attribute description in
`resources/modules/references/journal-blockers.md` and both
host-portable mirrors to remove the "N+1" convention. The `round`
field now documents "the round of the implementer attempt the blockers
describe". The worked example was updated to show the four-element
sequence `<implementer round="1">` → `<review round="1">` (blocking)
→ `<blockers round="1">` → `<implementer round="2">`; no
`<blockers round="2">` appears. The "Amendment-driven blockers"
paragraph was updated to use `round="N"` matching the prior
implementer round. `.claude/skills/speccy-amend/SKILL.md` and its
`.agents/` mirror were updated from `round="N+1"` to `round="N"` in
the amendment-driven blockers directive. The orchestrator/review skill
body was verified unchanged (it already wrote the correct convention).
CHK-012 grep for `round="N+1"` and "the round the implementer should
retry as" returns zero matches in canonical + both mirrors. Retry count: 1.
</coverage>

</report>
