---
spec: SPEC-0064
spec_hash_at_generation: df36aeac00c99b12030d9ff08144f9f590ce098d6a4a097ad79f595af88f4457
generated_at: 2026-06-13T04:49:24Z
---
# Tasks: SPEC-0064 Per-repo loop memory — an eject-safe `.speccy/MEMORY.md` the implementer reads before acting, grown by a ship-time retro from the loop's own conventions and mistakes

<task id="T-001" state="completed" covers="REQ-002 REQ-006">
## Author the shared `memory-ledger.md` entry-shape reference

Create `resources/modules/references/memory-ledger.md`, a new host-neutral
reference that is the single source of truth for what a ledger entry looks
like. Both the implementer read step (T-002) and the ship-time retro (T-003)
`{% include %}` this one file rather than restating the format — the same
deduplicate-snippets discipline `reuse-survey-implementer.md` follows, so the
no-duplicate-snippet invariant (REQ-003 / CHK-004) is satisfiable.

Document, leanly (the file ejects into users' repos and reloads into agent
context on every prompt — write what an agent needs to act on, cut
meta-annotation):

- The canonical working-tier path `.speccy/MEMORY.md` and its status:
  user-owned, git-tracked, a sibling of `.speccy/BACKLOG.md`, never
  enumerated or overwritten by `speccy init`. Absence is normal and silent.
- The four-part entry shape every entry carries: **trigger** (when it
  applies — a task area, file region, or situation), the **convention or
  mistake** recorded, the **corrective rule** to follow, and **provenance**
  (the SPEC / task / review that produced it). Convention-flavoured and
  mistake-flavoured entries share this one shape, differing only by feed
  source.
- Authoring discipline (REQ-006, authoring half): prefer abstract,
  convention-level wording over fragile code coordinates so an entry survives
  refactors and does not feed a phantom construct forward. Provenance must
  resolve to a real SPEC/task/review identifier, never a fabricated one; note
  that dangling SPEC/task provenance is the only structurally-checkable slice
  a future CLI verb could ever validate (DEC-007) — semantic staleness stays
  the retro's job (T-003).

Use obviously fictional placeholders (`SPEC-0042`, `T-003`,
`0042-example-slug`) in any worked example — this is shipped template content.
This task covers REQ-006 only for the authoring-discipline and
dangling-provenance wording in the reference; the retro-time re-validation and
garbage-collection mechanism is T-003.

The reference is not yet `{% include %}`d anywhere after this task (T-002 and
T-003 wire it in), so `just reeject` is a no-op for the rendered host trees.
Run it anyway and commit any regenerated output so the tree stays in sync.

<task-scenarios>
Given the repo after this task,
when `cargo test --workspace` runs,
then `dogfood_outputs_match_committed_tree` passes — the committed eject tree
is in sync with the renderer (this task introduces no include site, so the
render output is unchanged).

Given `resources/modules/references/memory-ledger.md`,
when a maintainer inspects it,
then it documents the canonical `.speccy/MEMORY.md` path with its
never-overwritten/user-owned status, all four entry parts (trigger,
convention/mistake, corrective rule, provenance), and the abstract-authoring
discipline — a manual-inspection check, since reference quality is a semantic
property and DEC-009 forbids substring-matching curated prose in tests.

Suggested files: `resources/modules/references/memory-ledger.md`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-003">
## Add the implementer feed-forward read step to the work phase body

Modify `resources/modules/phases/speccy-work.md` to instruct the implementer,
before the bounded reuse survey (current step 6) and any code write, to read
`.speccy/MEMORY.md` when present and load the slice whose trigger matches the
current task's area — mirroring the existing "load the relevant slice, drill
in on demand" shape the journal context bundle uses. When the file is absent
the step is a silent no-op (behaviour identical to today's loop). Insert the
step in the numbered recipe and renumber the subsequent steps. The step
`{% include "modules/references/memory-ledger.md" %}` for the entry shape
(authored in T-001) rather than restating it. The instruction lives only in
this canonical module body; the host wrappers
(`resources/agents/.claude/agents/speccy-work.md.tmpl`,
`resources/agents/.codex/agents/speccy-work.toml.tmpl`) already pull the whole
body via `{% include "modules/phases/speccy-work.md" %}`, so no inline copy is
added to any wrapper. Do not add a feed-forward read to the reviewer or vet
bodies (DEC-005 — reviewers stay adversarial).

After editing, run `just reeject` and commit the regenerated `.claude/` /
`.agents/` / `.codex/` trees so the ejected implementer agent body carries the
expanded reference with no drift.

Add a structural placement test (CHK-004), modelled on
`speccy-cli/tests/persona_snippets.rs`: assert the
`{% include "modules/references/memory-ledger.md" %}` directive appears in
`resources/modules/phases/speccy-work.md`, and assert no host wrapper under
`resources/agents/` contains that include directive inline (the reference
reaches every host transitively through the phase-body include, never as a
shadowing copy). This keys on the include-directive structural surface, not on
curated prose, so it complies with DEC-009. CHK-003 (the read actually changes
the produced diff) is semantic and validated only by dogfooding.

<task-scenarios>
Given the repo after this task,
when `cargo test --workspace` runs,
then the new CHK-004 structural test passes (the include exists once in the
canonical work-phase body, no wrapper inlines a shadowing copy) and
`dogfood_outputs_match_committed_tree` passes (the regenerated implementer
agent body is committed in sync with the renderer).

Given a `.speccy/MEMORY.md` seeded with one convention entry whose trigger
matches a task's area,
when the implementer subagent runs that task during dogfooding,
then its produced diff conforms to the recorded corrective rule without the
convention being restated in the task prompt — a manual-inspection dogfood
check that the read changes behaviour.

Given no `.speccy/MEMORY.md` present,
when the implementer runs,
then it proceeds with no error or comment about memory — behaviour identical
to today's loop (manual-inspection / dogfood check).

Suggested files: `resources/modules/phases/speccy-work.md`,
`speccy-cli/tests/memory_feedforward.rs`,
`resources/agents/.claude/agents/speccy-work.md.tmpl`,
`resources/agents/.codex/agents/speccy-work.toml.tmpl`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-004 REQ-005 REQ-006">
## Add the ship-time retro step to the ship phase body

Modify `resources/modules/phases/speccy-ship.md` to add a retro step at the
REPORT.md write boundary — after REPORT.md is written (current step 2) and
before the ship commit that bundles the loop's changes (current step 5) —
and renumber the subsequent steps. The retro lands in this phase body, not in
the thin ship SKILL stub. `{% include "modules/references/memory-ledger.md" %}`
for the entry shape (T-001) rather than restating it.

The retro distills the just-completed loop into ledger mutations, mining the
evidence already on disk — REPORT.md coverage, the per-task journal
(`<blockers>`, review verdict flips, retry rounds), and the spec diff — not a
fresh re-derivation of the work (REQ-004). It:

- Appends convention and/or mistake entries to `.speccy/MEMORY.md` in the
  four-part shape, one entry per write so the prose-layer write stays serial;
  a loop with recorded friction yields ≥1 mistake-flavoured entry citing that
  evidence, and a clean loop with no durable lesson records that explicitly
  rather than inventing one (REQ-004).
- Proposes promoting stable, repeatedly-affirmed entries up into the durable
  tier (`AGENTS.md` / rules) for **human approval**; on approval the entry
  moves to the durable tier and is removed from the ledger so it is not stored
  twice. Dedups candidates within the ledger and against the repo's existing
  durable docs, dropping anything already covered. Promotion is never silent
  or automatic (REQ-005, DEC-006).
- Re-validates existing ledger entries against the current tree and
  retires/rewrites any whose referenced construct is gone, so the ledger never
  feeds a phantom forward (REQ-006, retro half). This is the semantic
  garbage-collection complement to the abstract-authoring discipline T-001
  documents; explicitly not a CLI freshness-hashing mechanism (DEC-007).

The resulting ledger mutation lands in the same ship commit as REPORT.md.
After editing, run `just reeject` and commit the regenerated host trees.

All three covered requirements verify by dogfooding only (CHK-005/006/007):
the retro's capture, consolidation/dedup, and phantom-GC are semantic and have
no automated gate beyond `just reeject` cleanliness. SPEC-0064 shipping through
the loop is itself the first dogfood instance.

<task-scenarios>
Given the repo after this task,
when `cargo test --workspace` runs,
then `dogfood_outputs_match_committed_tree` passes — the regenerated ship-phase
agent body carrying the retro step is committed in sync with the renderer.

Given a spec shipped through the loop whose journal contains at least one
blocking-then-passed review round,
when the ship-time retro runs during dogfooding,
then `.speccy/MEMORY.md` gains an entry whose provenance cites that round and
whose corrective rule addresses the cause; a clean frictionless loop instead
records "no durable lesson" — a manual-inspection dogfood check.

Given a `.speccy/MEMORY.md` holding a stable entry, a candidate already covered
by `AGENTS.md`, and an entry whose referenced construct no longer resolves in
the tree,
when the retro runs during dogfooding,
then the stable entry is offered for human-gated promotion (and on approval
leaves the ledger), the already-covered candidate is dropped, and the
phantom-referencing entry is retired/rewritten so it is not in the slice the
next implementer loads — a manual-inspection dogfood check.

Suggested files: `resources/modules/phases/speccy-ship.md`
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-001 REQ-007">
## Prove and document that the CLI treats `.speccy/MEMORY.md` as an invisible user file

`.speccy/MEMORY.md` sits in the same never-planned-against bucket as
`.speccy/BACKLOG.md`: `speccy init` only scaffolds `.speccy/` and renders the
host pack, and `speccy verify` reads neither file. No production code changes
(DEC-002 — the eject pipeline never enumerates user files, so the property
holds for free). This task pins both halves of that invisibility with
regression tests and documents the file's place in the layout.

Add the CHK-001 init byte-identity test to `speccy-cli/tests/init.rs`, modelled
on `force_preserves_user_files`: seed `.speccy/MEMORY.md` with arbitrary
non-empty content, run `init --force --host claude-code` and
`init --force --host codex` in the same fixture, and assert the file is
byte-identical (string equality on read-back) after both, proving the ledger
sits outside the set of files the eject pipeline enumerates. Also assert a
fresh `init` in a repo without the file does not create one.

Add the CHK-008 verify-softness test to `speccy-cli/tests/verify.rs`, using the
`invoke(root, json)` helper: build an otherwise-clean workspace, write a
deliberately malformed `.speccy/MEMORY.md`, run `verify --json`, and assert no
`lint.errors` / `lint.warnings` / `lint.info` entry is attributable to the
ledger. Frame it as "lint output is unchanged whether or not the malformed file
is present" so the test cannot rot when an unrelated lint family is later
added. Add no new lint code referencing the ledger.

Document the file in `docs/ARCHITECTURE.md`'s `.speccy/` File Layout tree: add
a `MEMORY.md` line at the top level of `.speccy/` (sibling of the specs tree)
noting it is user-owned, git-tracked, and never enumerated/overwritten by
`speccy init`, mirroring `BACKLOG.md`.

<task-scenarios>
Given a repo with `.speccy/MEMORY.md` holding arbitrary non-empty content,
when `init --force --host claude-code` and `--host codex` both run,
then the file is byte-identical before and after (CHK-001), and a fresh `init`
in a repo without the file creates none — asserted by `cargo test --workspace`.

Given an otherwise-clean workspace with a deliberately malformed
`.speccy/MEMORY.md`,
when `speccy verify --json` runs,
then its lint output carries no error, warning, or info entry attributable to
the ledger — equivalently, the lint output matches the no-MEMORY baseline
(CHK-008), asserted by `cargo test --workspace`.

Given `docs/ARCHITECTURE.md` after this task,
when the `.speccy/` File Layout block is read,
then it lists `MEMORY.md` with its user-owned / never-overwritten status — a
manual-inspection check (DEC-009 forbids substring-matching curated docs prose).

Suggested files: `speccy-cli/tests/init.rs`, `speccy-cli/tests/verify.rs`,
`docs/ARCHITECTURE.md`
</task-scenarios>
</task>
