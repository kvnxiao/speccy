---
spec: SPEC-0053
outcome: implemented
generated_at: 2026-05-29T20:46:34Z
---

# REPORT: SPEC-0053 Port feature-dev agents into speccy — correctness reviewer, plan-explorer, plan-architect, read-only tool hardening, brainstorm trigger phrase

<report spec="SPEC-0053">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002">
T-001 added `resources/modules/personas/reviewer-correctness.md`, an
adversarial reviewer scoped to correctness/logic defects only (logic and
control-flow errors, null/`Option`/`Result` mishandling, off-by-one and
boundary conditions, non-security races/deadlocks, resource leaks). The
body `{% include %}`s the four shared review-contract snippets
(`verdict_return_contract.md`, `inline_note_format.md`,
`diff_fetch_command.md`, `no_tasks_md_writes.md`), names the four
deferral targets (vulnerabilities → security, conventions → style, SPEC
intent → business, coverage → tests) as out of its own lane, states the
confidence-≥80 reporting threshold with Critical/Important severity
grouping, and returns speccy's `<review>` verdict contract. CHK-001's
structural test confirms both rendered wrappers exist with includes fully
expanded; CHK-002 asserts the four deferral targets and the literal `80`
threshold are present. A round-1 tests-persona blocker required hardening
the structural assertions; the slice landed clean on round 2. Retry
count: 1.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-003">
T-002 added `resources/modules/personas/plan-explorer.md`, a read-only
codebase-grounding agent that traces feature implementations from entry
points through abstraction layers and emits a report covering entry
points/core files, execution flows with data transformations,
architecture layers and patterns, and a dependency map, each with
`file:line` references. The body uses an advisory, non-verdict contract:
it returns a report, never a `pass`/`blocking` verdict, never writes
`TASKS.md` or flips task state, and deliberately omits the `<review>`
verdict-contract snippets. CHK-003's structural test drives the real
`render_host_pack` MiniJinja pipeline for both hosts and asserts the
rendered body carries no `<review` marker. A round-1 style-persona
blocker drove a comment-only fix; landed round 2. Retry count: 1.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-004">
T-003 added `resources/modules/personas/plan-architect.md`, a read-only
architecture-design agent that analyzes existing codebase patterns and
emits a blueprint: component design, a file map of files to
create/modify, data flow, and a build sequence rendered as an ordered
checklist whose items are agent-sized (one item ≈ one Speccy task). It
shares plan-explorer's advisory, non-verdict contract (no verdict, no
state mutation, no `<review>` snippets). CHK-004's structural test
confirms both rendered wrappers exist with includes expanded and carry no
`<review` marker, and that the body specifies agent-sized build-sequence
items. Round-1 tests- and style-persona blockers in one round drove a
fix; landed round 2. Retry count: 1.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-005 CHK-006">
T-004 added an aggregate packaging-invariant test alongside
`speccy-cli/tests/skill_packs.rs` that, for `reviewer-correctness`,
`plan-explorer`, and `plan-architect`, asserts each Claude wrapper's
`model` is `opus[1m]`, each Codex wrapper's `model` is `gpt-5.5`, no
wrapper declares `sonnet`, and each persona body carries a `feature-dev`
attribution line. CHK-006 covers the model/attribution assertions; CHK-005
confirms `just reeject` is idempotent — a second consecutive run leaves a
clean working tree, proving source and ejected wrappers are consistent and
no ejected file was hand-edited. Clean first pass. Retry count: 0.
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-007 CHK-014">
T-001 inserted `"correctness"` into `speccy-core/src/personas.rs` `ALL`
at index 4 (after `style`, before `architecture`) and widened
`next.rs::default_personas()` from `ALL.get(..4)` to `ALL.get(..5)`, so
the default fan-out now dispatches five reviewers and `personas::ALL`
totals seven. The dependent registry/derivation tests (`personas.rs`
inline tests, `tests/personas.rs`), the persona-name lists in
`skill_packs.rs`, and the `review-fanout.md` documentation partial (both
the Claude Code and Codex host dispatch branches naming
`reviewer-correctness`) were updated to the five-default / seven-total
shape. Because `parse/journal_xml` validates `<review persona="…">`
against `personas::ALL`, the same registry entry makes a
`<review persona="correctness">` block parseable. CHK-007 confirms
`default_personas()` equals
`["business","tests","security","style","correctness"]` and that the
rendered `speccy-review` skill dispatches `reviewer-correctness`; CHK-014
confirms the journal parser accepts the `correctness` persona name as
registry-valid. Landed with REQ-001 on round 2. Retry count: 1.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-008">
T-005 updated `resources/modules/skills/speccy-brainstorm.md` and
`speccy-plan.md` to invoke `plan-explorer` unconditionally in their
codebase-context step, with routing prose that folds the explorer's
ephemeral grounding report into existing SPEC.md sections (Summary prose
and `<requirement>` grounding) and explicitly does not create a new
report artifact file. CHK-008's structural test confirms each ejected
skill body references invoking the `plan-explorer` subagent and that
neither references creating a new `*.md` report artifact outside the
SPEC.md routing targets. A round-1 tests-persona blocker drove a fix;
landed round 2. Retry count: 1.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-009">
T-005 updated `resources/modules/skills/speccy-decompose.md` to invoke
`plan-architect` before authoring `TASKS.md`, consume its build-sequence
checklist as the candidate task list while retaining final `<task>`
authorship (merge/split/reorder/number), and direct that load-bearing
blueprint decisions be promoted into SPEC.md `### Decisions` (DEC-NNN)
blocks rather than buried in task prose. CHK-009's structural test
confirms the ejected `speccy-decompose` body references invoking
`plan-architect`, names the build-sequence checklist as candidate tasks,
and references promoting decisions into `### Decisions`. Landed with
REQ-006 on round 2. Retry count: 1.
</coverage>

<coverage req="REQ-008" result="satisfied" scenarios="CHK-010 CHK-011">
T-006 added an explicit read-only `tools:` grant (`Read`, `Grep`, `Glob`,
`LS`, `Bash`, `WebFetch`; no `Edit`/`Write`/`NotebookEdit`) to every
read-only agent wrapper — `plan-explorer`, `plan-architect`,
`reviewer-correctness`, the six existing `reviewer-*` (business, tests,
security, style, architecture, docs), and `vet-reviewer` — while leaving
the five writer agents (`speccy-work`, `speccy-decompose`,
`speccy-ship`, `vet-implementer`, `vet-simplifier`) with full grants.
CHK-010 confirms each of the ten read-only Claude wrappers declares a
`tools:` field including `Read` and excluding `Edit`/`Write`; CHK-011
confirms the five writer wrappers were not narrowed to the read-only set.
Clean first pass. Retry count: 0.
</coverage>

<coverage req="REQ-009" result="satisfied" scenarios="CHK-012">
T-006 determined that Codex does not honor a per-subagent tool
restriction: its `.codex/agents/*.toml` agent-definition format exposes
only `config_file`, `description`, and `nickname_candidates` per agent,
with tool gating living at MCP-server, app/connector, and global scope
(verified against the Codex config reference at
developers.openai.com/codex/config-reference, 2026-05). The finding was
recorded as a durable note in `AGENTS.md` ("Read-only agent tool grants —
Claude Code vs Codex parity"), stating that the Codex read-only posture
remains prose-enforced through each persona body's advisory contract and
that no `tools` field should be added to the Codex `.toml` wrappers
expecting it to be honored. CHK-012 confirms the durable note states
explicitly whether Codex honors per-subagent tool restriction. Clean
first pass. Retry count: 0.
</coverage>

<coverage req="REQ-010" result="satisfied" scenarios="CHK-013">
T-007 added the phrase `can we brainstorm` to the `speccy-brainstorm`
skill description's trigger-phrase list in the source under `resources/`,
so that after `just reeject` the rendered skill description for both
hosts carries the phrase alongside the existing triggers. CHK-013's
structural test confirms the rendered `speccy-brainstorm` skill
description frontmatter contains the substring `can we brainstorm` for
both hosts. A round-1 tests-persona blocker drove a fix; landed round 2.
Retry count: 1.
</coverage>

</report>

## Notes

This SPEC is the bootstrap case for the very agents it ships:
`plan-explorer` and `plan-architect` did not yet exist to ground or
design SPEC-0053 itself, so the brainstorm grounded the codebase
manually. That is a one-time bootstrap condition — future
`speccy-plan` / `speccy-decompose` runs will lean on the now-shipped
plan-time agents.

The read-only tool hardening (REQ-008/REQ-009) was bundled with the
feature-dev port deliberately: the three new read-only personas
establish the read-only-grant precedent, and shipping them while
leaving the six existing reviewers wide-open would have been
inconsistent. The Codex side of that hardening is prose-enforced
rather than mechanical because Codex does not honor a per-subagent
`tools` restriction (REQ-009).
