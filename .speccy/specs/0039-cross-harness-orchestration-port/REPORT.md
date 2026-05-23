---
spec: SPEC-0039
outcome: implemented
generated_at: 2026-05-22T22:45:00Z
---

# REPORT: SPEC-0039 Cross-harness orchestration port — orchestrator and holistic-gate skills ship from shared modules with thin per-host adapters in both packs

<report spec="SPEC-0039">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003 CHK-004">
T-001, T-002, and T-003 factored the four hand-written pilot bodies into
`resources/modules/`: `speccy-orchestrate.md` (orchestrator loop body with
per-task retry budget hardcoded as `5`), `speccy-holistic-gate.md` (holistic
drift-fix loop with round budget hardcoded as `3`), `holistic-reviewer.md`,
and `holistic-implementer.md` persona modules. Each module restricts inline
`{% if host == "claude-code" %}` blocks to sub-agent-spawn dispatch points
only; non-dispatch prose is host-neutral. The three legacy pilot paths
(`.claude/skills/speccy-holistic-review/SKILL.md`,
`.claude/agents/speccy-holistic-reviewer.md`,
`.claude/agents/speccy-holistic-fixer.md`) were deleted; their successors
under the post-DEC-002/DEC-003 names exist. `speccy init --force` regenerates
the ejected trees byte-for-byte from source; `git status --porcelain .claude/
.agents/ .codex/` prints zero lines after regeneration. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-005 CHK-006 CHK-007">
T-004 authored the eight per-host adapter files: two Claude `SKILL.md.tmpl`
wrappers, two Codex (`.agents/`) `SKILL.md.tmpl` wrappers, two Claude agent
templates (`.md.tmpl` with `model: opus[1m]`), and two Codex agent templates
(`.toml.tmpl` with a Codex model identifier, not `opus[1m]`). Sub-agent names
use the post-DEC-002 forms (`holistic-reviewer`, `holistic-implementer`)
throughout -- no reference to the legacy `speccy-` prefix or `fixer` name
survives in any template under `resources/`. Running `speccy init --force`
materialises all seven rendered outputs (two Claude skills, three Claude
agents/skills, two Codex agents). Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-008 CHK-009 CHK-010">
T-004 authored `resources/modules/skills/speccy-orchestrate-codex-grant.md`
containing a self-contained prose explanation of the Codex sub-agent-spawn
permission grant. The Codex `speccy-orchestrate` wrapper
(`resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl`) includes
the grant module via a second `{% include %}` directive after the host-neutral
body; the Claude wrapper contains no reference to the grant module. The
rendered Codex `.agents/skills/speccy-orchestrate/SKILL.md` contains at least
one `permission` match; the rendered Claude `.claude/skills/speccy-orchestrate/SKILL.md`
contains zero matches on grant-related lines. Retry count: 0.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-011 CHK-012">
T-006 rewrote the doc comment at `speccy-core/src/prompt/id_alloc.rs:3` from
citing `docs/ARCHITECTURE.md` to citing only SPEC-0005 DEC-005. After the
edit, `rg -n 'ARCHITECTURE\.md' speccy-core/src speccy-cli/src resources`
prints zero matches. The exempt project-local surfaces (`AGENTS.md`,
`speccy-core/tests/docs_sweep.rs`, `speccy-cli/tests/init.rs`,
`.speccy/specs/`) continue to reference `ARCHITECTURE.md` as before.
`cargo test --workspace` exits 0. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-013 CHK-014">
T-005 rewrote the Codex `{% else %}` branch of
`resources/modules/skills/speccy-review.md` to use Codex's native
sub-agent-spawn primitive in place of the legacy prose-spawn idiom. The
matching assertions in `speccy-cli/tests/skill_packs.rs::speccy_review_skill_prefers_native_subagents`
were updated: the old `"Prose-spawn the four reviewer subagents"` literal pin
was replaced with a negative `!contains("prose-spawn")` guard and a new
positive assertion pinning the native-primitive wording. `rg -in
'prose.?spawn' resources/modules resources/agents/.agents resources/agents/.codex`
prints zero matches. `cargo test -p speccy-cli --test skill_packs` exits 0.
Retry count: 0.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-015 CHK-016 CHK-017">
T-007 rewrote the three project-local positioning surfaces. `README.md` now
includes `/speccy-orchestrate` in the slash-command recipe table framing it as
the opinionated end-to-end driver, and the repo layout tree lists the
orchestrate + holistic-gate skill directories under both `.claude/skills/` and
`.agents/skills/`. `docs/ARCHITECTURE.md`'s "Long-Term Vision" section no
longer lists multi-agent orchestration under "Future layers (not v1)"; a
paragraph acknowledges the orchestration loop as a current skill-layer
artifact. `AGENTS.md`'s "Product north star" drops the `(Future)` parenthetical
and the "Long-term, speccy is the substrate underneath multi-agent harnesses"
sentence; the shipped orchestration loop appears among the v1.0 outcomes. All
six durable principles under `## Core principles` survive verbatim; the
seven-command CLI surface description is unchanged. Retry count: 0.
</coverage>

</report>

## Notes

The TSK-003 warning visible in `speccy status` output (stored
`spec_hash_at_generation` differing from current SPEC-0039 hash) reflects the
REQ-006 amendment added mid-loop. The hash was updated to `168f69a8...` after
the amendment landed; the residual mismatch in the final status output is a
known artefact of the SPEC receiving one further prose touch after the hash
update, and was user-triaged as acceptable for ship (see
`journal/HOLISTIC.md`).

The holistic gate returned `verdict="fail" rounds="1"` due to four reviewer
findings. Finding #2 (stale `spec_hash_at_generation`) was fixed surgically.
Findings #1, #3, #4 were user-deferred as out-of-SPEC-0039 scope per the
`journal/HOLISTIC.md` triage record. The user explicitly confirmed ship is
safe.
