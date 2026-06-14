---
spec: SPEC-0065
spec_hash_at_generation: 67e8f7b223eed51d41e69b403f26bb86c5b969a2c99565cde6b1d15a21090094
generated_at: 2026-06-14T02:48:38Z
---
# Tasks: SPEC-0065 Backlog ledger — a convention-only `.speccy/BACKLOG.md` register of future-spec candidates

<task id="T-001" state="completed" covers="REQ-001">
## Create the `backlog-ledger.md` reference module

Create `resources/modules/references/backlog-ledger.md`, structurally
parallel to `resources/modules/references/memory-ledger.md`. It documents:

- the `.speccy/BACKLOG.md` file header — the preamble a producing skill
  copies in when the file self-creates on first append: user-owned,
  git-tracked, never created or overwritten by `speccy init`/reeject,
  absence normal and silent, distinct from `MEMORY.md`;
- the four-field entry shape, one line per field — **Title** (the
  prospective spec in a phrase), **What & why** (what it delivers plus the
  value, the case for building it), **Deferred-because** (why not now —
  out of current slice / needs infra / blocked on X), **Provenance**
  (originating spec + phase, e.g. `SPEC-NNNN, ship` or `manual`);
- authoring discipline mirroring the memory ledger — terse, honest
  provenance that resolves to a real spec/phase;
- the guidance that many backlog entries spawned from one spec's loop is a
  focus smell: the per-spec add rate is itself feedback, not an enforced
  threshold.

As a `modules/references/` file it sits under the prose-hygiene
worked-instance carve-out, so its example entry MAY use the `SPEC-0042`
family ids exactly as `memory-ledger.md` does. This task is the
`{% include %}` target that T-002/T-003/T-004 depend on and adds no ejected
footprint on its own (a reference ejects only once a consumer includes it),
so it MUST land before them or `render_host_pack` fails and dogfood parity
breaks. Run `just reeject` and commit (no ejected delta expected here).

<task-scenarios>
Given the resources tree at HEAD after this task,
when `resources/modules/references/backlog-ledger.md` is read,
then it documents the file header and the four fields
Title / What & why / Deferred-because / Provenance, parallel in shape to
`memory-ledger.md`. Clarity and parallelism is a persona-review judgment.

Suggested files: `resources/modules/references/backlog-ledger.md` (new),
`resources/modules/references/memory-ledger.md` (structural template)
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-001 REQ-003 REQ-004 REQ-006">
## Wire speccy-plan to read, append, and strike backlog entries

Edit the skill body `resources/modules/skills/speccy-plan.md` (plan is a
full-include skill — its body, not the wrapper, carries reference includes;
see the existing reference includes in that file):

- **Read (REQ-003):** at framing/candidate intake, read `.speccy/BACKLOG.md`
  when present and surface its entries as candidate slices; treat absence as
  silent and non-fatal.
- **Append (REQ-004):** when scope is deliberately cut as a *future-spec
  candidate*, append a four-field entry (self-creating the file with its
  header if absent), provenance naming the originating spec + plan phase.
  Route a merely spec-local exclusion to the SPEC's `## Non-goals` instead —
  the backlog is for "should become its own spec," not "not in this spec."
- **Strike (REQ-006):** when a backlog item is promoted into a new SPEC,
  delete the entry outright — no struck-through or "promoted to" residue;
  git history and the new SPEC's own provenance are the trail.
- **Include (REQ-001):** add `{% include "modules/references/backlog-ledger.md" %}`
  at the authoring step so the entry shape reaches plan's context.

Run `just reeject`, commit the regenerated `.claude/`/`.codex/`/`.agents/`
packs; `cargo test --workspace` (dogfood parity) must stay green. Keep new
prose generic — `SPEC-NNNN`/`T-NNN` placeholders only; skill bodies are not
under the references carve-out.

<task-scenarios>
Given the ejected speccy-plan skill body at HEAD after this task,
when a reviewer reads its framing, scope-cut, and promotion handling,
then it reads `.speccy/BACKLOG.md` as candidate slices (absence silent),
appends a four-field entry on deliberate future-spec scope-cut while routing
spec-local exclusions to `## Non-goals`, and strikes a promoted entry by
silent deletion. These are persona-review judgments, not scriptable
assertions.

Given the resources tree and ejected packs at HEAD,
when the packs are re-ejected,
then `backlog-ledger.md` is included by speccy-plan and its shape appears in
the ejected output (parity holds; gates orphaning, not file existence).

Suggested files: `resources/modules/skills/speccy-plan.md`, regenerated
ejected trees (via `just reeject`)
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-001 REQ-003 REQ-004">
## Wire speccy-brainstorm to read and append backlog entries

Edit the skill body `resources/modules/skills/speccy-brainstorm.md`
(brainstorm is a full-include skill — the include goes in the body):

- **Read (REQ-003):** at "Explore project context", read `.speccy/BACKLOG.md`
  when present and fold its entries into the candidate framings; absence is
  silent.
- **Append (REQ-004):** when the Socratic exchange deliberately defers a
  future-spec candidate out of the current slice, append a four-field entry
  with originating-spec + brainstorm-phase provenance, distinct from the
  spec-local Non-goals routing the skill already performs.
- **Include (REQ-001):** add
  `{% include "modules/references/backlog-ledger.md" %}` at the authoring
  step.

No strike here — brainstorm writes no SPEC and never promotes (REQ-006 is
plan-only). Run `just reeject`, commit ejected packs; `cargo test
--workspace` green. Generic placeholders only.

<task-scenarios>
Given the ejected speccy-brainstorm skill body at HEAD after this task,
when a reviewer reads its context-exploration and scope-cut handling,
then it reads `.speccy/BACKLOG.md` as candidate input (absence silent) and
appends a four-field entry for deliberately deferred future-spec candidates,
routing spec-local exclusions to Non-goals. Persona-review judgments.

Suggested files: `resources/modules/skills/speccy-brainstorm.md`, regenerated
ejected trees
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-001 REQ-005">
## Wire speccy-ship to mirror the judgment-gated future-spec subset

Edit the **phase** body `resources/modules/phases/speccy-ship.md` (ship is a
phase, not a skill; it already `{% include %}`s `memory-ledger.md` at its
MEMORY retro — the backlog include and step belong adjacent to it, NOT in a
skill body):

- **Mirror (REQ-005):** at the REPORT "Deferred / known limitations"
  handling, add a per-item judgment — "its own future SPEC, or just a
  limitation of this one?" — and mirror ONLY the future-spec-worthy subset
  into `.speccy/BACKLOG.md` as four-field entries with ship-phase provenance.
  Items judged local limitations stay in REPORT.md only and are not
  appended.
- **Include (REQ-001):** add
  `{% include "modules/references/backlog-ledger.md" %}` adjacent to the
  existing `memory-ledger.md` include.

Run `just reeject`, commit ejected packs; `cargo test --workspace` green.
Generic placeholders only.

<task-scenarios>
Given the ejected speccy-ship phase body at HEAD after this task,
when a reviewer reads its deferred-section handling,
then it gates mirroring on the per-item "own future spec?" judgment, mirrors
only that subset into `.speccy/BACKLOG.md` with ship provenance, and leaves
local limitations in REPORT.md. Persona-review judgment.

Given the resources tree and ejected packs at HEAD,
when the packs are re-ejected,
then `backlog-ledger.md` is included by the ship phase and its shape appears
in the ejected output (parity holds).

Suggested files: `resources/modules/phases/speccy-ship.md`, regenerated
ejected trees
</task-scenarios>
</task>

<task id="T-005" state="pending" covers="REQ-007">
## Name the backlog in the bootstrap conventions block

Edit the conventions reference
`resources/modules/references/agents-md-speccy-conventions.md` — the
canonical always-upserted "## Speccy conventions" block source the bootstrap
phase pulls in via `{% include %}` (do NOT edit the bootstrap phase body;
the block text lives in this reference). Add one terse line naming
`.speccy/BACKLOG.md` and its role: it holds future-spec candidates; planning
reads it, plan/ship append. Place it near the existing journal-location
line.

Run `just reeject`, commit ejected packs; `cargo test --workspace` green.
Keep the line terse — the conventions block ejects into users' `AGENTS.md`
on every bootstrap.

<task-scenarios>
Given the ejected conventions reference at HEAD after this task,
when it is scanned,
then it references the `.speccy/BACKLOG.md` path and a one-line read/append
role for it. The path reference is structural data; the wording's adequacy
is a persona-review judgment.

Suggested files:
`resources/modules/references/agents-md-speccy-conventions.md`, regenerated
ejected trees
</task-scenarios>
</task>

<task id="T-006" state="pending" covers="REQ-002">
## Regression-test that init never touches a user-owned `.speccy/BACKLOG.md`

Test-only — no production change. `speccy init` enumerates a fixed manifest
of shipped files and never touches user-owned files outside it, exactly as
`.speccy/MEMORY.md` already enjoys; do NOT add an explicit exclusion list to
init source (that would be the wrong design and contradict the MEMORY
precedent). Mirror the MEMORY test pair
(`fresh_init_does_not_create_speccy_memory_ledger`,
`force_preserves_speccy_memory_ledger` in `speccy-cli/tests/init.rs`):

- `fresh_init_does_not_create_speccy_backlog_ledger` — run `speccy init` in a
  fresh repo, assert `.speccy/BACKLOG.md` does not exist afterward.
- `force_preserves_speccy_backlog_ledger` — seed a sentinel
  `.speccy/BACKLOG.md` of known content, run `speccy init --force` for both
  hosts, assert the file's bytes (sha256) are unchanged and it is not among
  the paths init reports as ejected.
- Optionally extend with a CHK-004 assertion: with the sentinel present,
  `speccy verify` exits 0 and emits no backlog-attributable diagnostic.

`cargo test --workspace` green.

<task-scenarios>
Given a temporary repo containing a sentinel `.speccy/BACKLOG.md`,
when `speccy init --force` runs for both hosts,
then the file's sha256 is unchanged and it is not among the ejected paths.

Given a fresh temporary repo,
when `speccy init` then `speccy verify` run in sequence,
then no `.speccy/BACKLOG.md` exists and `speccy verify` exits 0 with no
backlog-attributable diagnostic.

Suggested files: `speccy-cli/tests/init.rs`
</task-scenarios>
</task>
