---
spec: SPEC-0066
spec_hash_at_generation: 8a1c80a52ccc227239f525172f6d34648795d3f00c134d24f9c5012e71978e10
generated_at: 2026-06-21T09:12:29Z
---
# Tasks: SPEC-0066 Pre-ship provenance cleanup — broaden the provenance convention and add a dedicated vet cleanup pass

<task id="T-001" state="completed" covers="REQ-001">
## Broaden the shared provenance convention bullet

Edit the "No provenance or doc-pointer meta-annotation" bullet in
`resources/modules/references/convention-checklist.md` so it names three
reference classes beyond the `// per X` form it currently illustrates, each
with a concrete negative example drawn from the eight leaked forms:
(a) descriptive prose that points at a planning artifact as the reason a line
exists, with no `// per` framing (e.g. "every failure mode the spec defines",
"later specs populate", "a later spec can ask"); (b) numbered project-rule
citations (e.g. "cardinal rule #4", parallel to the existing
"(Core principle 2)" example); (c) doc-path citations (e.g.
"(docs/implementation)"). Keep the runtime-artifact carve-out intact — naming
a path the code operates on (`SPEC.md`, a `.speccy/…` path) stays data, not
provenance — and keep the example ids as generic placeholders (the
resource-prose hygiene lint bans concrete Speccy ids outside references).

This file is the convention SSOT, `{% include %}`d by the implementer
self-review in `resources/modules/phases/speccy-work.md` and the style
reviewer in `resources/modules/personas/reviewer-style.md`; one edit reaches
both early gates. Run `just reeject` so the regenerated ejected packs under
`.claude/`, `.agents/`, and `.codex/` carry the broadened bullet.

<task-scenarios>
Given the resources tree and the ejected host packs after this task,
when the work-phase and style-reviewer modules are re-ejected and their
includes resolved,
then the broadened provenance bullet appears in both the ejected
implementer-self-review body and the ejected style-reviewer body — gating
include-wiring and resource-to-ejected parity, not file existence.

Given the broadened bullet,
when a reviewer reads it against the eight leaked forms,
then none of those forms can be rationalised as acceptable and the carve-out
still clearly permits naming a path the code operates on.

Suggested files: `resources/modules/references/convention-checklist.md`,
plus the `just reeject` regenerated mirrors under `.claude/`, `.agents/`,
`.codex/`.
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-002 REQ-003">
## Add the single-concern `vet-provenance` subagent (body + per-host wrappers)

Create the dedicated provenance-cleanup subagent, modeled on `vet-simplifier`.
The subagent is named `vet-provenance`.

Body — `resources/modules/personas/vet-provenance.md`, mirroring the shape of
`resources/modules/personas/vet-simplifier.md`: provenance cleanup as the
**sole** review dimension (no competing remit); `{% include
"modules/references/convention-checklist.md" %}` for the provenance definition
rather than an inline copy; apply-mode instructions to rewrite offending
comment/doc/test-doc prose so the bare provenance pointer is dropped while the
intent the comment conveys is preserved; explicit carve-out respect; edits
confined to prose, never logic (behaviour-preserving). Its verdict-return
contract records through the orchestrator's gate summary and a thin verdict —
it appends **no** new journal block (the CLI block set is closed).

Wrappers — a Claude Code `.md.tmpl` at
`resources/agents/.claude/agents/vet-provenance.md.tmpl` and a Codex
`.toml.tmpl` at `resources/agents/.codex/agents/vet-provenance.toml.tmpl`, each
`{% include %}`ing the body. Because the pass is apply-mode, the Claude wrapper
carries **no** read-only `tools:` grant (mirror `vet-simplifier.md.tmpl`).
Frontmatter declares the medium pin (`model: opus[1m]`, `effort: medium` on
Claude; the Codex analog on the Codex wrapper). The description is
angle-bracket-free, ≤1024 chars, and carries a "Use when …" clause with no
"Do NOT trigger" clause (subagent wrapper). Run `just reeject` so the subagent
ejects to `.claude/agents/vet-provenance.md` and `.codex/agents/vet-provenance.toml`.

<task-scenarios>
Given the resources tree after this task,
when the persona body's includes resolve,
then the convention reference is included and the provenance-definition text
is not also duplicated inline in the body.

Given the wrapper templates,
when their frontmatter is parsed,
then the Claude wrapper omits a read-only `tools:` restriction, both declare
the medium model/effort pin, and the description passes the angle-bracket and
"Use when …" hygiene checks.

Given the resources tree and the ejected host packs after re-ejection,
when the wrappers are ejected,
then the subagent is present in both `.claude/agents/` and `.codex/agents/`
with the persona body inlined.

Given the persona body,
when a reviewer reads it,
then it reads as a single-concern provenance pass instructing
intent-preserving rewrite, carve-out respect, and prose-only scope.

Suggested files: `resources/modules/personas/vet-provenance.md`,
`resources/agents/.claude/agents/vet-provenance.md.tmpl`,
`resources/agents/.codex/agents/vet-provenance.toml.tmpl`, plus the
`just reeject` regenerated subagent mirrors.
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-004">
## Wire the pre-ship provenance-cleanup phase into the vet flow

Add a dedicated provenance-cleanup phase to
`resources/modules/skills/partials/vet-phases.md`, slotted as a distinct
numbered phase immediately after the Phase 2 simplifier pass and before the
gate phase; renumber the gate phase so it remains the last phase (the single
`<gate>`-appending step stays final). The new phase dispatches the
`vet-provenance` subagent once over the cumulative `diff_command` from the
Phase 0 context bundle, and reuses the same journal-safe snapshot/rollback
sequence the simplifier phase uses. It records its outcome **only** through the
existing CLI-owned `<gate>` block's free-text one-line summary plus the verdict
returned to the orchestrator — it issues no `speccy journal append --block` for
a new type, adds no new `VetBlockKind`, and adds **no** new field to the
orchestrator return block (consistent with DEC-002's no-new-structured-surface
intent). Update `resources/modules/skills/speccy-vet.md` only as far as needed
to name the new phase in its loop/return narrative; leave the per-task review
fan-out untouched (the pass is pre-ship only). Run `just reeject` so the
regenerated vet skill packs carry the new phase.

<task-scenarios>
Given the ejected vet skill pack and the CLI block set after this task,
when the vet phase flow is read,
then it contains a provenance-cleanup phase that spawns the `vet-provenance`
subagent over the cumulative diff before the gate, references no new `--block`
type, and the CLI's journal block set is unchanged.

Given the vet phase prose and the per-task review skill body,
when a reviewer reads them,
then the provenance phase runs once over the cumulative diff at pre-ship and
records via the gate summary plus verdict, and the per-task review fan-out has
gained no provenance persona.

Suggested files: `resources/modules/skills/partials/vet-phases.md`,
`resources/modules/skills/speccy-vet.md`, plus the `just reeject` regenerated
vet skill mirrors.
</task-scenarios>
</task>
