---
spec: SPEC-0039
spec_hash_at_generation: 168f69a88535da4cdda603f24e6c488daaa5bead0b69f82f15238c9f33cc01dc
generated_at: 2026-05-22T06:58:12Z
---
# Tasks: SPEC-0039 Cross-harness orchestration port — orchestrator and holistic-gate skills ship from shared modules with thin per-host adapters in both packs

<task id="T-001" state="completed" covers="REQ-001">
## Factor the orchestrator pilot into `resources/modules/skills/speccy-orchestrate.md`

Lift the host-neutral body of the orchestrator pilot at
`.claude/skills/speccy-orchestrate/SKILL.md` into a new shared module
at `resources/modules/skills/speccy-orchestrate.md`. The shared body
contains the startup integrity check, the outer dispatch loop, the
per-task retry counter, status-line writes, and stop conditions. Drop
the YAML frontmatter — frontmatter lives in the per-host wrappers
that T-004 authors.

Pilot-to-shared transformation rules:

- Host variance at sub-agent-spawn dispatch points is expressed via
  inline `{% if host == "claude-code" %}…{% else %}…{% endif %}`
  blocks per DEC-001 mechanism A. The precedent to follow is
  `resources/modules/skills/speccy-review.md` lines 74-115 (Claude
  branch instructs the `Task` tool with `subagent_type:`, Codex
  branch invokes the native sub-agent-spawn primitive against the
  same target name). Restrict `{% if host %}` blocks to dispatch
  points only — non-dispatch prose is host-neutral.
- The per-task retry budget appears as the literal integer `5` in
  the body (e.g. "after 5 rounds the orchestrator…"). No jinja
  variable reference (`{{ … }}`), no template default, no env
  lookup. CHK-002 grep'd against this file must find `5` paired
  with the per-task retry budget on at least one line.
- Sub-agent names dispatched from the orchestrator must use the
  post-DEC-002 forms (`holistic-reviewer`, `holistic-implementer`)
  and never the pilot-era forms (`speccy-holistic-reviewer`,
  `speccy-holistic-fixer`). The orchestrator dispatches to
  `/speccy-work`, `/speccy-review`, and `/speccy-holistic-gate` —
  the holistic-gate skill is the renamed lifecycle skill per
  DEC-003.
- The shared body MUST NOT reference `ARCHITECTURE.md`, `.speccy/ARCHITECTURE.md`,
  or any other repo-local doc. Shipped skill bodies stay portable to
  any speccy-using repo. If the pilot mentions `ARCHITECTURE.md`,
  drop the reference or replace it with a cite of the relevant
  SPEC (e.g. SPEC-0039 itself).

<task-scenarios>
Given the source tree at HEAD after this task,
when `ls resources/modules/skills/speccy-orchestrate.md` runs,
then the path exists and contains non-empty body content.

Given the same file,
when `rg -nU '\{% if host == "claude-code" %\}' resources/modules/skills/speccy-orchestrate.md` runs,
then it prints at least one match — proving the dispatch divergence block is present.

Given the same file,
when `rg -n '\b5\b' resources/modules/skills/speccy-orchestrate.md` runs,
then at least one match pairs the literal `5` with the per-task retry budget; and `rg -n '\{\{.*retry|budget.*\}\}' resources/modules/skills/speccy-orchestrate.md` prints zero matches — proving the integer is hardcoded inline, not a jinja variable.

Given the same file,
when `rg -n 'speccy-holistic-(?:review|reviewer|fixer)|ARCHITECTURE\.md' resources/modules/skills/speccy-orchestrate.md` runs,
then it prints zero matches — proving no legacy names and no project-local doc reference survive in the shared body.

Suggested files: `resources/modules/skills/speccy-orchestrate.md` (new), `.claude/skills/speccy-orchestrate/SKILL.md` (read-only source for the factoring).
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001">
## Factor the holistic-review pilot into `resources/modules/skills/speccy-holistic-gate.md`

Lift the host-neutral body of the holistic-review pilot at
`.claude/skills/speccy-holistic-review/SKILL.md` into a new shared
module at `resources/modules/skills/speccy-holistic-gate.md`. The new
filename uses the post-DEC-003 lifecycle skill name. The shared body
covers Phase 0 (bootstrap and integrity), Phase 1 (drift review +
fix loop with budget 3), Phase 2 (simplifier polish), the HOLISTIC.md
journal contract, the defer-write-before-rollback rule, and the
return contract. Drop the YAML frontmatter — frontmatter lives in the
per-host wrappers that T-004 authors.

Pilot-to-shared transformation rules:

- Same `{% if host == "claude-code" %}…{% else %}…{% endif %}`
  block discipline as T-001: dispatch points only, non-dispatch prose
  is host-neutral. The Claude branch instructs the `Task` tool with
  `subagent_type:`; the Codex branch invokes Codex's native
  sub-agent-spawn primitive.
- The drift-fix round budget appears as the literal integer `3` in
  the body. No jinja variable reference, no template default, no
  env lookup. CHK-002 grep'd against this file must find `3`
  paired with the drift-fix round budget on at least one line.
- Sub-agent dispatch targets use the post-DEC-002 names
  (`holistic-reviewer`, `holistic-implementer`). The shared body
  MUST NOT mention the legacy names (`speccy-holistic-reviewer`,
  `speccy-holistic-fixer`) or the legacy lifecycle skill name
  (`speccy-holistic-review`).
- The shared body MUST NOT reference `ARCHITECTURE.md` or any other
  repo-local doc.

<task-scenarios>
Given the source tree at HEAD after this task,
when `ls resources/modules/skills/speccy-holistic-gate.md` runs,
then the path exists and contains non-empty body content.

Given the same file,
when `rg -nU '\{% if host == "claude-code" %\}' resources/modules/skills/speccy-holistic-gate.md` runs,
then it prints at least one match.

Given the same file,
when `rg -n '\b3\b' resources/modules/skills/speccy-holistic-gate.md` runs,
then at least one match pairs the literal `3` with the drift-fix round budget; and `rg -n '\{\{.*round|budget.*\}\}' resources/modules/skills/speccy-holistic-gate.md` prints zero matches.

Given the same file,
when `rg -n 'speccy-holistic-(?:review|reviewer|fixer)|ARCHITECTURE\.md' resources/modules/skills/speccy-holistic-gate.md` runs,
then it prints zero matches.

Suggested files: `resources/modules/skills/speccy-holistic-gate.md` (new), `.claude/skills/speccy-holistic-review/SKILL.md` (read-only source for the factoring).
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001">
## Factor the two holistic-loop sub-agent pilots into `resources/modules/personas/`

Lift the host-neutral bodies of the two holistic-loop sub-agent
pilots into shared persona modules:

- `.claude/agents/speccy-holistic-reviewer.md` → `resources/modules/personas/holistic-reviewer.md`
  (focus list, round-2+ scrutiny rules, verdict-return contract).
- `.claude/agents/speccy-holistic-fixer.md` → `resources/modules/personas/holistic-implementer.md`
  (scope, hygiene-gate, verdict-return contract).

Persona module bodies do not carry YAML frontmatter — frontmatter
(model pin, effort level, allowed tools) lives in the per-host agent
templates that T-004 authors. Each new persona body is host-neutral
prose: if a pilot section reads as Claude-Code-specific (e.g.
"return your verdict via the Task tool's final message"), generalize
to host-agnostic wording (e.g. "return your verdict via your final
message"). The existing
`resources/modules/personas/reviewer-business.md` and
`resources/modules/personas/reviewer-tests.md` are the structural
templates to mirror.

The two new persona bodies MUST NOT reference `ARCHITECTURE.md` or
any other repo-local doc. If the pilot's sub-agent body mentions
the legacy lifecycle skill name (`speccy-holistic-review`) or the
legacy sister sub-agent name (`speccy-holistic-fixer` →
`holistic-implementer`, etc.), rewrite to use the post-DEC-002 /
post-DEC-003 names.

<task-scenarios>
Given the source tree at HEAD after this task,
when `ls resources/modules/personas/holistic-reviewer.md resources/modules/personas/holistic-implementer.md` runs,
then both paths exist with non-empty bodies.

Given the same files,
when `rg -n 'speccy-holistic-(?:review|reviewer|fixer)|ARCHITECTURE\.md' resources/modules/personas/holistic-reviewer.md resources/modules/personas/holistic-implementer.md` runs,
then it prints zero matches.

Given the same files,
when each is read,
then neither begins with a YAML frontmatter block (no leading `---` fence) — proving the persona module file carries only the body content for include into per-host agent templates.

Suggested files: `resources/modules/personas/holistic-reviewer.md` (new), `resources/modules/personas/holistic-implementer.md` (new), `.claude/agents/speccy-holistic-reviewer.md` (read-only source), `.claude/agents/speccy-holistic-fixer.md` (read-only source), `resources/modules/personas/reviewer-business.md` (structural reference), `resources/modules/personas/reviewer-tests.md` (structural reference).
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-002 REQ-003">
## Author Codex grant module + per-host wrappers + per-host agent templates; regenerate; delete legacy pilots

Compose the per-host adapters for the orchestration loop on top of
the shared modules from T-001/T-002/T-003, regenerate the ejected
skill packs via `speccy init --force`, and retire the three legacy
pilot paths whose names change under DEC-002/DEC-003. Concrete sub-steps:

1. **Codex permission-grant module (REQ-003)** — author
   `resources/modules/skills/speccy-orchestrate-codex-grant.md`
   containing a self-contained explanation of how a Codex user grants
   the orchestrator skill permission to spawn sub-agents. Source the
   prose from `CODEX-SKILLS-AND-SUBAGENTS.md` at the repo root.
2. **Four `SKILL.md.tmpl` wrappers (REQ-002)** — author
   `resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl`,
   `resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl`,
   `resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl`,
   `resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl`.
   Each wrapper carries YAML frontmatter (host-native skill metadata)
   and a single `{% include "modules/skills/<skill>.md" %}` directive
   pulling in the matching shared body. The Codex orchestrate wrapper
   additionally includes
   `{% include "modules/skills/speccy-orchestrate-codex-grant.md" %}`
   AFTER the host-neutral body include — that is the additive
   selective-include of DEC-001 mechanism B. The Claude orchestrate
   wrapper MUST NOT include the grant module and MUST NOT carry any
   permission-grant prose inline.
3. **Four per-host agent templates (REQ-002)** — author
   `resources/agents/.claude/agents/holistic-reviewer.md.tmpl`,
   `resources/agents/.claude/agents/holistic-implementer.md.tmpl`
   (each with `model: opus[1m]` and the effort level carried over
   from the pilot file in `.claude/agents/`),
   `resources/agents/.codex/agents/holistic-reviewer.toml.tmpl`,
   `resources/agents/.codex/agents/holistic-implementer.toml.tmpl`
   (each with TOML `model =` set to a Codex model identifier — not
   `opus[1m]` — and a Codex reasoning-effort setting). Each template
   body `{% include %}`s its matching `resources/modules/personas/<name>.md`.
   Side-by-side host pins per DEC-006; no abstraction layer.
4. **Render + cleanup** — run `speccy init --force` from the repo
   root, then `git rm` the three legacy pilot paths whose names
   change: `.claude/skills/speccy-holistic-review/SKILL.md`,
   `.claude/agents/speccy-holistic-reviewer.md`,
   `.claude/agents/speccy-holistic-fixer.md`. The pre-existing
   `.claude/skills/speccy-orchestrate/SKILL.md` is regenerated in
   place by the render and stays under that path (per Notes).
5. **Byte-identity verification (CHK-004)** — after staging the
   render + deletes, `git status --porcelain .claude/ .agents/ .codex/`
   should print zero lines — proving the committed ejected trees
   match the source byte-for-byte.

<task-scenarios>
Given the source tree at HEAD after this task,
when `ls resources/modules/skills/speccy-orchestrate-codex-grant.md resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl resources/agents/.claude/agents/holistic-reviewer.md.tmpl resources/agents/.claude/agents/holistic-implementer.md.tmpl resources/agents/.codex/agents/holistic-reviewer.toml.tmpl resources/agents/.codex/agents/holistic-implementer.toml.tmpl` runs,
then all nine paths exist and `ls` exits 0.

Given the same checkout,
when `rg -n 'speccy-orchestrate-codex-grant' resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl` runs,
then it prints at least one match; and when `rg -n 'speccy-orchestrate-codex-grant|grant.*subagent|permission.*spawn' resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl` runs, then it prints zero matches.

Given the same checkout,
when `rg -n '^model:' resources/agents/.claude/agents/holistic-reviewer.md.tmpl resources/agents/.claude/agents/holistic-implementer.md.tmpl` runs,
then each file's match contains the literal substring `opus[1m]`; and when `rg -n '^model =' resources/agents/.codex/agents/holistic-reviewer.toml.tmpl resources/agents/.codex/agents/holistic-implementer.toml.tmpl` runs, then each file's match has a non-empty value that does not contain `opus`.

Given the same checkout,
when `rg -n 'speccy-holistic-(?:review|reviewer|fixer)' resources/` runs,
then it prints zero matches — proving no shipped template references the legacy lifecycle or sub-agent names.

Given the same checkout after `speccy init --force` has run and the three legacy paths have been `git rm`'d,
when `git status --porcelain .claude/ .agents/ .codex/` runs,
then it prints zero lines; and when `test ! -e .claude/skills/speccy-holistic-review/SKILL.md && test ! -e .claude/agents/speccy-holistic-reviewer.md && test ! -e .claude/agents/speccy-holistic-fixer.md` runs, then it exits 0; and when `ls .claude/skills/speccy-holistic-gate/SKILL.md .claude/agents/holistic-reviewer.md .claude/agents/holistic-implementer.md .agents/skills/speccy-orchestrate/SKILL.md .agents/skills/speccy-holistic-gate/SKILL.md .codex/agents/holistic-reviewer.toml .codex/agents/holistic-implementer.toml` runs, then all seven rendered paths exist.

Given the rendered output,
when `rg -c 'permission' .agents/skills/speccy-orchestrate/SKILL.md` runs,
then it reports at least one match in a line referencing the sub-agent-spawn grant; and when `rg -c 'permission' .claude/skills/speccy-orchestrate/SKILL.md` runs against any line referencing the sub-agent-spawn grant, then it reports zero matches.

Suggested files: `resources/modules/skills/speccy-orchestrate-codex-grant.md` (new), `resources/agents/.claude/skills/speccy-orchestrate/SKILL.md.tmpl` (new), `resources/agents/.claude/skills/speccy-holistic-gate/SKILL.md.tmpl` (new), `resources/agents/.agents/skills/speccy-orchestrate/SKILL.md.tmpl` (new), `resources/agents/.agents/skills/speccy-holistic-gate/SKILL.md.tmpl` (new), `resources/agents/.claude/agents/holistic-reviewer.md.tmpl` (new), `resources/agents/.claude/agents/holistic-implementer.md.tmpl` (new), `resources/agents/.codex/agents/holistic-reviewer.toml.tmpl` (new), `resources/agents/.codex/agents/holistic-implementer.toml.tmpl` (new), `.claude/skills/speccy-orchestrate/SKILL.md` (regenerated by render), `.claude/skills/speccy-holistic-review/SKILL.md` (deleted), `.claude/agents/speccy-holistic-reviewer.md` (deleted), `.claude/agents/speccy-holistic-fixer.md` (deleted), `.claude/skills/speccy-holistic-gate/SKILL.md` (new render output), `.claude/agents/holistic-reviewer.md` (new render output), `.claude/agents/holistic-implementer.md` (new render output), `.agents/skills/speccy-orchestrate/` (new render output tree), `.agents/skills/speccy-holistic-gate/` (new render output tree), `.codex/agents/holistic-reviewer.toml` (new render output), `.codex/agents/holistic-implementer.toml` (new render output), `CODEX-SKILLS-AND-SUBAGENTS.md` (read-only source for grant prose).
</task-scenarios>
</task>

<task id="T-005" state="completed" covers="REQ-005">
## Retire the legacy Codex prose-spawn idiom in `speccy-review.md` and update pinning test assertions

Rewrite the Codex `{% else %}` branch of
`resources/modules/skills/speccy-review.md` (currently at lines 95-114
of the host-divergence block) so it instructs the agent to invoke
Codex's native sub-agent-spawn primitive rather than "prose-spawn the
four reviewer subagents by name". The replacement wording should
mirror the Claude branch's structure (per-persona spawn + the same
review-prompt body) but use Codex's documented native syntax for the
spawn call. Source the canonical Codex spawn syntax from
`CODEX-SKILLS-AND-SUBAGENTS.md` at the repo root.

Update `speccy-cli/tests/skill_packs.rs::speccy_review_skill_prefers_native_subagents`
to match the new rendered Codex output:

- The "Codex branch: step 4 must not mention `subagent_type:` and must
  name each persona in prose as `\`reviewer-<persona>\`" assertion
  becomes "must invoke Codex's native sub-agent-spawn primitive and
  must reference each persona's name". Pin the new native-primitive
  needle string(s) the new Codex render emits.
- The Claude negative assertion (`!claude_body.contains("Prose-spawn the four reviewer subagents")`)
  is no longer load-bearing once that string is gone from the source
  — replace it with a positive assertion that pins the Claude render's
  current shape, or delete the negative if its successor exists
  elsewhere in the file.

After the rewrite, run `speccy init --force` to regenerate the ejected
skill packs and stage the resulting diff under `.claude/` and `.agents/`
in the same commit.

CHK-013 verification: `rg -in 'prose.?spawn' resources/modules resources/agents/.agents resources/agents/.codex`
must print zero matches after this task lands.

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -in 'prose.?spawn' resources/modules resources/agents/.agents resources/agents/.codex` runs,
then it prints zero matches.

Given the same checkout,
when `cargo test -p speccy-cli --test skill_packs` runs,
then it exits 0 — the updated test assertions match the new rendered output.

Given the rendered Codex output post-`speccy init --force`,
when `.agents/skills/speccy-review/SKILL.md` is read,
then its sub-agent dispatch step uses Codex's native sub-agent-spawn primitive (per `CODEX-SKILLS-AND-SUBAGENTS.md`) and contains no occurrence of the case-insensitive substring `prose-spawn`.

Given the same checkout,
when `git status --porcelain .claude/skills/speccy-review .agents/skills/speccy-review` runs after `speccy init --force` and staging,
then it prints zero lines — proving the regenerated ejected output is committed in this task's diff.

Suggested files: `resources/modules/skills/speccy-review.md`, `speccy-cli/tests/skill_packs.rs`, `.claude/skills/speccy-review/SKILL.md` (regenerated), `.agents/skills/speccy-review/SKILL.md` (regenerated), `CODEX-SKILLS-AND-SUBAGENTS.md` (read-only source for native syntax).
</task-scenarios>
</task>

<task id="T-006" state="completed" covers="REQ-004">
## Strip the `ARCHITECTURE.md` reference from `speccy-core/src/prompt/id_alloc.rs`

Rewrite the doc comment at `speccy-core/src/prompt/id_alloc.rs:3` to
cite only the SPEC that the constant derives from (SPEC-0005 DEC-005)
rather than `.speccy/ARCHITECTURE.md`. The current text reads:

```
//! Per SPEC-0005 DEC-005 and `.speccy/ARCHITECTURE.md` "Spec ID allocation":
```

After this task it should read (or equivalent) — citing the SPEC as
the source of the no-gap-recycling decision without mentioning
`ARCHITECTURE.md`:

```
//! Per SPEC-0005 DEC-005 ("Spec ID allocation"):
```

This is a one-shot sweep, not a lint family addition (per DEC-004).
After the edit, verify the scope is correctly bounded — `AGENTS.md`,
per-crate `README.md` files, the project-local `.speccy/specs/`
history, and `speccy-cli/tests/init.rs` / `speccy-core/tests/docs_sweep.rs`
(which assert on the project-local `ARCHITECTURE.md`) all remain
exempt and continue to mention `ARCHITECTURE.md` as they did before.

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -n 'ARCHITECTURE\.md' speccy-core/src speccy-cli/src resources` runs,
then it prints zero matches.

Given the same checkout,
when `rg -c 'ARCHITECTURE\.md' AGENTS.md` runs,
then it reports at least one match (the project-local `AGENTS.md` continues to reference `ARCHITECTURE.md`); and when `rg -l 'ARCHITECTURE\.md' .speccy/specs/` runs, then it prints at least one path (existing SPEC history is preserved); and when `rg -l 'ARCHITECTURE\.md' speccy-core/tests speccy-cli/tests` runs, then it prints at least one path (the docs-sweep + init tests still reference the project-local file).

Given the same checkout,
when `cargo test --workspace` runs,
then it exits 0 — proving no test broke as a side effect of the doc-comment edit.

Suggested files: `speccy-core/src/prompt/id_alloc.rs`.
</task-scenarios>
</task>

<task id="T-007" state="completed" covers="REQ-006">
## Reposition multi-agent orchestration as a shipped v1.0 artifact across `README.md`, `.speccy/ARCHITECTURE.md`, and `AGENTS.md`

Rewrite the three project-local doc surfaces that frame Speccy's
positioning so the multi-agent orchestration loop reads as a shipped
v1.0 artifact rather than a `(Future)` layer downstream harnesses
would build. Concrete edits per file:

- **`README.md`** — extend the slash-command recipe table in the
  "Step 3: Drive specs end-to-end from your agent harness" section
  (currently five rows: `/speccy-plan`, `/speccy-tasks`,
  `/speccy-work`, `/speccy-review`, `/speccy-ship`) to add a row
  for `/speccy-orchestrate` (and, where relevant, mention
  `/speccy-holistic-gate` as the gating step it delegates to before
  ship). Frame the orchestrator as the opinionated end-to-end
  driver that chains the five existing recipes. Cross-check that
  the "Repo layout after `speccy init`" tree (currently lines
  196-225) lists the orchestrate + holistic-gate skill directories
  in both the `.claude/skills/` and `.agents/skills/` lines after
  T-004 lands.

- **`.speccy/ARCHITECTURE.md`** — rewrite the
  "Long-Term Vision" section (currently lines 2515-2533) so the
  bullet list under "Future layers (not v1)" no longer includes
  multi-agent orchestration. The remaining future layers
  (concurrent task pickup with file-locking, worktree orchestration
  per task, cross-spec dependency reasoning, dashboard/kanban UI,
  production-telemetry feedback, cross-repository orchestration)
  stay on the list. Add a short paragraph (or update the
  introductory sentence) acknowledging that the implementation +
  review orchestration loop now ships as part of the skill layer.
  Also audit the "Skills / Harness Layer" tree (around line 1711)
  and ensure the orchestrate + holistic-gate skills are catalogued
  there.

- **`AGENTS.md` "Product north star"** — rewrite line 34
  ("Long-term, speccy is the substrate underneath multi-agent
  harnesses that move projects toward completion without humans
  re-explaining intent at every step.") to drop the long-term
  framing for the orchestration loop specifically; the substrate
  framing still applies to future multi-host or cross-repo
  harnesses. Rewrite line 44 ("(Future) multi-agent harnesses
  building on Speccy's deterministic feedback substrate.") to
  remove the `(Future)` parenthetical — the in-pack orchestration
  loop is current; only out-of-pack harnesses remain future.
  Update the "V1.0 outcome" bullets to include the shipped
  orchestration loop alongside the existing seven-command CLI and
  shipped skill packs bullets.

Guardrails: the rewrite is prose-only. No CLI surface change, no
schema change, no `<requirement>`/`<task>` element changes elsewhere.
The six durable principles in `## Core principles` survive verbatim.
The seven-command CLI surface ("`init`, `status`, `next`, `check`,
`verify`, `lock`, `vacancy`") is unchanged. The "stay-small" framing
stays — the orchestrator is described as a skill-layer artifact, not
as a new CLI verb. Do not delete the "Future layers (not v1)" list
header itself; only remove the multi-agent-orchestration bullet from
its body.

This task runs after T-004 has landed the rendered orchestrate +
holistic-gate skills under `.claude/` and `.agents/`, so the
`README.md` recipe table can reference paths that exist on disk and
the architecture-doc skill-tree audit reflects the real shipped
layout.

<task-scenarios>
Given the source tree at HEAD after this task,
when `rg -n 'speccy-orchestrate' README.md` runs,
then it prints at least one match in a line introducing the orchestrator recipe in the slash-command table.

Given the same checkout,
when the "Long-Term Vision" section of `.speccy/ARCHITECTURE.md` is read,
then no bullet under "Future layers (not v1)" references multi-agent orchestration.

Given the same checkout,
when `rg -n '\(Future\) multi-agent' AGENTS.md` runs,
then it prints zero matches; and when `rg -nU 'Long-term, speccy is the substrate underneath multi-agent harnesses' AGENTS.md` runs, then it prints zero matches.

Given the same checkout,
when `rg -n '^\d+\. \*\*Feedback, not enforcement\.\*\*' AGENTS.md` and `rg -nU 'Stay small\.' AGENTS.md` run,
then each prints at least one match — proving the six durable principles in `## Core principles` survived the rewrite verbatim.

Given the same checkout,
when `rg -c '`init`, `status`, `next`, `check`, `verify`, `lock`, `vacancy`' AGENTS.md` runs,
then it reports at least one match — proving the seven-command CLI surface description survived the rewrite intact.

Suggested files: `README.md`, `.speccy/ARCHITECTURE.md`, `AGENTS.md`.
</task-scenarios>
</task>
