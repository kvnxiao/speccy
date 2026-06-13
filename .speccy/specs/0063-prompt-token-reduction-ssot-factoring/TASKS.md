---
spec: SPEC-0063
spec_hash_at_generation: 8214de57b0948060596f804c12b6ff3e89e0ad2fb2b038c22f4f44cc9d3d23b5
generated_at: 2026-06-13T03:06:42Z
---
# Tasks: SPEC-0063 Prompt token reduction + SSOT factoring for `resources/` modules — compress SDLC-loop prose, defer two procedures to existing CLI guarantees, then factor duplicated passages into shared modules

<task id="T-001" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-006 REQ-007">
## Track 1 — compress SDLC-loop prose and defer two procedures to existing CLI guarantees (commit 1)

Track 1 is **commit 1** of two. It cuts redundant prose from the SDLC-loop
modules under `resources/modules/**` so each operational fact appears once at its
canonical site, and lands two determinism trims (work-phase re-read, ship-phase
status round-trips). Edit **source only** — never edit `.claude/`, `.agents/`, or
`.codex/` (ejected output). After editing, run `just reeject` (regenerates all
three host packs: it runs `speccy init --force --host claude-code` then
`--host codex`; the Codex host writes both `.agents/` and `.codex/`). The
review-pass `git add -A` captures source edits + regenerated packs in this task's
single atomic commit.

**Binding constraints (do not violate):**
- **SPEC-0049 DEC-002 — act-without-read.** An inline one-sentence rule paired with
  a pointer to a longer canonical reference is deliberate: the inline sentence lets
  the agent act without an on-demand file read. Never reduce such a one-liner to
  the bare pointer — that costs *more* tokens. Cut only true restatements and
  over-explanation.
- **Token savings come only from deleting words.** Do not factor anything into
  `{% include %}` modules here — that is Track 2 (T-002) and saves zero runtime
  tokens.
- **Locate by anchor, not line number.** The quoted anchors below were captured
  pre-edit; earlier edits in the same file drift line numbers. After applying one
  edit, re-locate the next anchor.

Apply the edits in ROI order, Tier A → E. Estimated ~330–360 source lines removed
(descriptive context, not an acceptance threshold).

### Tier A — hot reviewer fan-out (×5 parallel windows per review round)

- **A1 `references/cli-stamps.md`** — collapse the three restatements (8 → ~5
  lines). Keep the facts: the CLI owns the block's `date`, `round`, open/close
  tags, and the journal's frontmatter/sectioning; pipe only the inner body on
  stdin (no override flag); validation runs before any write, so a malformed body
  leaves the journal byte-identical.
- **A2 `references/identity-sourcing.md`** — keep the `## Sourcing your recorded
  identity` heading, the two bullets (Model segment / Effort suffix), and the
  fallback. Cut the "a sub-agent records its definition-file effort even when
  dispatched from a higher-effort parent session" digression (anchor:
  `sub-agent` … `records its definition-file effort`) and the named env vars
  (`CLAUDE_EFFORT` / `CLAUDE_CODE_EFFORT_LEVEL`), collapsing to "never read it from
  a runtime env override."
- **A3 `personas/verdict_return_contract.md`** — three cuts: (1) delete the whole
  `## The --model value is required` section (it restates `identity-sourcing.md`,
  which is included immediately after via the
  `modules/references/identity-sourcing.md` include; the requirement is already
  visible in the Step-1 `--model <your-model>` command — optionally add one
  trailing clause to Step 1 noting `--model` is required, built per the rule
  below); (2) trim the `round` sentence (anchor: `Here \`round\` is the journal's
  current implementer round`) to the operational facts only (append rejected if no
  `<implementer>` for the round; per-file lock serialises parallel appends); (3)
  compress the thin-verdict rationale (anchor: `Do not restate the full review
  body in the thin verdict`) to one clause.
- **A4 `personas/reviewer-tests.md`** — keep the `## Evidence loading` heading.
  Replace the inline "Shape recognition" roll-call code block (anchor: `Shape
  recognition — a valid Evidence roll call looks like:` through the closing fence)
  with a one-line pointer to `evidence.md` (already cited:
  `Canonical evidence file shape: {{ speccy_references_path }}/evidence.md`). State
  the journal path `.speccy/specs/NNNN-slug/journal/T-NNN.md` once (it is repeated
  across steps 1–3). Tighten the two framing paragraphs around the
  fabrication-pattern list (anchors: `Scrutinise the loaded evidence for these
  fabrication patterns` and `Stay framework-agnostic`) to one sentence each.
- **A5 `personas/reviewer-style.md`** — keep the `## Diff-format pitfalls`
  heading. Compress the newline-marker explanation (anchor: `The "No newline at
  end of file" marker is the canonical case`) to ~10 lines: keep the trap
  statement, the byte probe `tail -c 1 <path> | od -An -tx1`, and the "cite it in
  your review" line; drop the generalization paragraph (anchor: `The same caution
  applies to any rendered-output invariant`). Collapse the four `## Out of scope`
  bullets to two (all four say the version-control envelope is the orchestrator's,
  not style's), and drop the escape-hatch paragraph (anchor: `If you genuinely
  believe a style-relevant invariant requires a specific git-state`) — the same
  "surface as a one-line aside" move is restated under "Grounding a lint-driven
  verdict."
- **A6 `personas/reviewer-correctness.md`** — delete the `> Ported from the
  feature-dev …` provenance line (anchor: `Ported from`).

### Tier B — resident / per-invocation skills

- **B1 `skills/speccy-vet.md`** — cut the annotated VET.md markdown example
  (anchor: the fenced ```markdown``` block starting `spec: SPEC-NNNN` through its
  closing fence) and replace with one pointer line; the CLI creates and stamps all
  of it and the skill never writes it. Compress `### Single-writer rule` (anchor:
  `The **CLI's per-file append lock owns write serialization**`) to "all VET writes
  go through `speccy journal append`; never edit the file by hand." Trim
  `## When to invoke directly` (it repeats `## When to use`). Tighten `## Why this
  skill runs in a top-level session` — **settle this wording now**; T-002 (T2-E)
  factors it into a shared module, so the canonical phrasing is decided here.
- **B2 `skills/partials/vet-phases.md`** — compress the git-internals essay
  (anchor: `Two git facts drive the mechanism below:` through `never a pre-existing
  unrelated one).`) to ~8 lines: the two journal-exclusion facts plus "snapshot
  with a plain `--include-untracked` stash; restore code with the tracked-only
  checkout below; never `git stash pop`." **Keep the verbatim 4-command block**
  (anchor: `# Revert code to the pre-sub-agent snapshot`). Cut the inline
  re-explanations that already point to "see Protect the journal from rollback
  above" (anchors: `The journal is swept into the` … `stash by
  \`--include-untracked\`` and `As in Phase 1, \`--include-untracked\` sweeps the
  journal`) — keep the pointer, drop the re-explanation. Trim the Phase 3 generic
  cli-stamps restatement (anchor: `It stamps \`date\` (UTC now).`) — keep the
  vet-specific `tasks_hash`/sectioning lines.
- **B3 `skills/speccy-orchestrate.md`** — cut the prose paragraph that re-narrates
  the ASCII Lifecycle diagram (anchor: `The orchestrator owns the outer loop, the
  per-task retry counter`) down to a 2-line ownership note; keep the diagram.
  Compress `## Context discipline` (it re-explains inline-fanout a third time plus
  status-hints-not-state). Tighten the canonical inline-fanout statement (anchor:
  `**Why fan-outs run inline in this skill's session.**`) — **settle this wording
  now** (T-002 T2-E factors it).
- **B4 `phases/speccy-init.md`** (note: the init body lives under `phases/`, not
  `skills/`) — drop the doubly-enumerated 4-cell matrix walk (anchor: the prose
  spelling out "(north star absent + conventions absent) runs both…"). Cut the
  "Asymmetry vs. the conventions upsert" rationale subsection (anchor:
  `Asymmetry`). De-dup the independence statement (stated at "Make the two seeding
  decisions independently" and again later).
- **B5 `skills/speccy-brainstorm.md`** — cut `## Key principles` (it recaps the
  Steps); keep only the net-new "No premature implementation" point, folded into
  Step 3. (The stale `<!-- … independent copy … -->` comment is removed in T-002
  T2-F, not here.)
- **B6 `skills/speccy-review.md`** — cut the post-fan-out paragraph (anchor: `It
  drives the review-induced writes` … `exclusively through the CLI verbs the
  partial above details`) — `review-fanout` (included just above) already details
  these verbs. Compress the inline-fanout paragraph (anchor: `Because sub-agents
  cannot spawn sub-agents`) — **settle this wording now** (T-002 T2-E factors it).
- **B7 `skills/speccy-amend.md`** — drop the restated "narrow staging /
  when-exists / idempotent" trio around the commit step (anchor: `narrow file-list
  staging`) — `commit-recipe.md` owns it. Drop the redundant "Placeholder leakage"
  bullet if it merely restates the mechanical/semantic split above it. **Do not**
  remove the narrow-staging command strings or branch-guard prose that
  `authoring_commit.rs` asserts (see the test-guardrail map) — cut only guarantees
  that *duplicate* `commit-recipe.md`.
- **B8 `skills/speccy-plan.md`** — trim commit-step guarantees that duplicate
  `commit-recipe.md` (anchor: `narrow file-list staging` / `idempotent`). Drop the
  redundant "Placeholder leakage" bullet (same as B7). Same guardrail caveat as B7.

### Tier C — phases & personas (subagent bodies) + the two determinism wins

- **C1 `phases/speccy-work.md` (prose cuts)** — compress the "Reviewer north-star
  map" (anchor: `**Reviewer north-star map.**`) to 4 one-line bullets. Cut the
  inline minimal Evidence roll-call example (anchor: `Minimal Evidence roll-call
  shape`) — it duplicates `evidence.md`, already pointed to (anchor: `Canonical
  evidence file shape:`). Trim the round-derivation prose (anchor: `Here \`round\`
  derives as`) and the `--model` slash-suffix prose (anchor: `\`--model\` is
  required. Encode reasoning effort`) — both duplicate the included
  `cli-stamps.md` / `identity-sourcing.md`.
- **C1 determinism (REQ-002)** — `phases/speccy-work.md`: remove the post-append
  journal re-read (anchor: `After the append, re-read the journal and confirm the
  new`). The CLI validates an append before any write, so a malformed block can
  never land and the re-read proves nothing. Reduce to: confirm `speccy next
  --json` shows no consistency drift.
- **C2 determinism (REQ-003)** — `phases/speccy-ship.md`, two removals: (step 1)
  drop the `speccy status SPEC-NNNN --json` call (anchor: step 1's ```bash block
  with `speccy status SPEC-NNNN --json`); the `speccy next --json` already run in
  `## When to use` yields both readiness (`next_action.kind == "ship"`) and
  `spec_md_path` / `tasks_md_path`, so rewrite step 1 to rely on that single
  query. (Step 3) drop the post-flip `speccy status` re-check (anchors: `Confirm
  the workspace is still clean:` plus the following `speccy status SPEC-NNNN
  --json` and `should report no \`TSK-003\` mismatch`); the flip is excluded from
  `spec_hash_at_generation`, so `TSK-003` cannot fire and the re-check is provably
  unnecessary.
- **C3 `phases/speccy-decompose.md`** — trim the `speccy lock` re-explanation
  (anchor: `\`speccy lock\` edits TASKS.md's frontmatter in place`) to "requires
  TASKS.md to already exist." Light-trim the "Key constraints" bullets (anchor:
  `Key constraints:`) that merely restate parser errors the CLI emits, but **keep**
  the two non-obvious gotchas (frontmatter-then-heading with no blank line; the
  `bootstrap-pending` sentinel plus don't-lock-before-the-file-exists).
- **C4 `personas/vet-implementer.md`** — cut the "Why the body is structured"
  rationale (anchor: `Why the body is structured`). De-dup the verdict-semantics
  overlap with the per-verdict guidance above it. Leave the base-ref fallback block
  in place (it moves in T-002 T2-C).
- **C5 `personas/vet-simplifier.md`** — collapse the 4×-restated Phase-2 scope
  boundary (anchor: `Focus scope`) to one statement. De-dup the
  no-git-stash/rollback note (it feeds T-002 T2-D).
- **C6 `personas/plan-architect.md` and `personas/plan-explorer.md`** — delete the
  `> Ported from the feature-dev …` provenance line (anchor: `Ported from`) in
  each.

### Tier D — shared recipe preambles (expand into 3–4 callsites)

- **D1 `references/commit-recipe.md`** — delete the "single source of truth / no
  verbatim copy" meta-preamble (anchor: `This module is the single source of truth
  for how a skill turns`). **Keep** the two-parameter description (staging breadth;
  title/body) and everything from `### No-git short-circuit` onward, including the
  single `git diff --cached --quiet` idempotency check, the
  `modules/references/identity-sourcing.md` include, and `git rev-parse
  --is-inside-work-tree` + "not a git repository" / "without erroring".
- **D2 `references/branch-guard.md`** — delete the meta-preamble (anchor: `This
  module is the single source of truth for the branch-guard prelude`). **Keep** the
  one-parameter (spec-dir) line and everything from `### No-git short-circuit`
  onward — the three detection tiers in order, "does not resolve", `spec-` +
  basename, `git switch -c`, detached, reuse, the creation notice, "only on the
  create path" / "not on the reuse path", and the no-git short-circuit.
- **D3 `skills/partials/review-fanout.md`** — in the spawn prompt (anchor: `Open
  your per-task context read with a`), replace the field-by-field bundle
  enumeration with a terse instruction telling the reviewer to run `speccy context
  SPEC-NNNN/T-NNN --json` for the bundle, read the diff with the bundle's suggested
  diff command, apply the persona's criteria, and drill into prior rounds with
  `speccy journal show SPEC-NNNN/T-NNN --round N [--block <type>]` if needed.
  **Keep** the verdict-contract and dirty-tree paragraphs that follow (anchors:
  `Follow the verdict-return contract in your agent file` and `The working tree may
  be dirty`), and keep `git add -A`, `[SPEC-NNNN/T-NNN]:`, and "Commits land on
  whatever HEAD is". **Do not** introduce `git status --porcelain`, `git diff
  --cached --quiet`, or a `branch-guard.md` include into this file.

### Tier E — cold references (read on demand)

- **E `references/evidence.md`** — drop one of the three worked scenarios (anchor:
  `Scenario 3`); the first two already teach the red/green + roll-call shape.
- **E `references/retry-shape.md`** — drop the parenthetical beginning "(In
  practice the round-3 implementer's atomic-commit step would have already landed…"
  (anchor: `In practice the round-3`; note the capital "I").

### Test-guardrail map (keep these green)

`cargo test --workspace` must stay green. The load-bearing assertions:
- **`persona_snippets.rs`** — each reviewer persona keeps its `{% include %}` lines
  for `verdict_return_contract`, `diff_fetch_command`, `inline_note_format`; the
  rendered persona contains `Do not edit TASKS.md directly`; `reviewer-style` keeps
  the `## Diff-format pitfalls` heading; `reviewer-tests` keeps the `## Evidence
  loading` heading; ejected personas contain no `{%` markup (includes fully
  expand).
- **`authoring_commit.rs`** — `commit-recipe.md` keeps exactly one `git diff
  --cached --quiet`, includes `identity-sourcing.md`, does not itself contain
  `## Sourcing your recorded identity`, and keeps `git rev-parse
  --is-inside-work-tree` + the not-a-git-repository / without-erroring prose;
  `branch-guard.md` keeps the three detection tiers in order + "does not resolve" +
  `spec-`/basename + `git switch -c` + detached + reuse + notice + "only on the
  create path" / "not on the reuse path" + the no-git short-circuit;
  `review-fanout.md` keeps `git add -A`, `[SPEC-NNNN/T-NNN]:`, "Commits land on
  whatever HEAD is", and does not contain `git status --porcelain` / `git diff
  --cached --quiet` / a `branch-guard.md` include; decompose keeps `[SPEC-NNNN]:
  decompose tasks` + `git add <spec-dir>/TASKS.md`.
- Also watch `skill_packs` and `skill_body_discovery` (no dangling references, no
  orphaned modules).

### Close-out

Run `just reeject`, then the full hygiene suite — `cargo test --workspace`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo
+nightly fmt --all --check`, `cargo deny check` — all must pass. Spot-check a
couple of ejected `.claude/...` files: they contain the expected compressed text
and no `{%` marker, and the ejected-pack diff mirrors the `resources/modules/**`
edits 1:1 (a larger-than-expected eject diff signals a broken wrapper or template
variable — investigate before completing). Record the prose audit (every cut
fact still appears once at a canonical site) and the two determinism trims in
`REPORT.md` at ship time. The transient `PROMPT-TOKEN-REDUCTION-PLAN.md` working
doc was deleted at decompose time; no action needed here.

<task-scenarios>
Given the `resources/modules/**` tree and the ejected packs after this task,
when a reviewer audits the Tier A–E passages enumerated above,
then none of those passages appears in source or eject, no kept
inline-rule-plus-pointer (e.g. the `retry-shape` / `evidence` pointers) was
reduced to a bare pointer, every cut fact still appears once at a canonical site,
and the audit is recorded in `REPORT.md` (CHK-001).

Given the post-edit tree,
when `cargo test --workspace` runs — notably `persona_snippets.rs` and
`authoring_commit.rs`,
then all guardrail assertions pass: required headings (`## Diff-format pitfalls`,
`## Evidence loading`), the `{% include %}` lines, and the command strings in the
rendered personas and recipes are intact (CHK-002).

Given the ejected `phases/speccy-work` body after this task,
when it is read,
then it contains no post-append journal re-read step and instead relies on the
CLI's validate-before-write guarantee, retaining at most a `speccy next --json`
consistency check (CHK-003).

Given the ejected `phases/speccy-ship` body after this task,
when it is read,
then step 1 contains no `speccy status SPEC-NNNN --json` call (deriving readiness
and paths from the `speccy next --json` already run) and step 3 contains no
post-flip `speccy status` re-check (CHK-004).

Given the four hygiene gates after `just reeject`,
when `cargo test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny
check` run,
then all four pass (CHK-008).

Given the ejected `.claude/`, `.agents/`, and `.codex/` packs after this reeject,
when scanned,
then no ejected file contains a `{%` marker and the eject diff mirrors the
`resources/modules/**` edits 1:1 (CHK-009).

Suggested files: `resources/modules/references/cli-stamps.md`,
`resources/modules/references/identity-sourcing.md`,
`resources/modules/references/commit-recipe.md`,
`resources/modules/references/branch-guard.md`,
`resources/modules/references/evidence.md`,
`resources/modules/references/retry-shape.md`,
`resources/modules/personas/verdict_return_contract.md`,
`resources/modules/personas/reviewer-tests.md`,
`resources/modules/personas/reviewer-style.md`,
`resources/modules/personas/reviewer-correctness.md`,
`resources/modules/personas/vet-implementer.md`,
`resources/modules/personas/vet-simplifier.md`,
`resources/modules/personas/plan-architect.md`,
`resources/modules/personas/plan-explorer.md`,
`resources/modules/skills/speccy-vet.md`,
`resources/modules/skills/speccy-orchestrate.md`,
`resources/modules/skills/speccy-brainstorm.md`,
`resources/modules/skills/speccy-review.md`,
`resources/modules/skills/speccy-amend.md`,
`resources/modules/skills/speccy-plan.md`,
`resources/modules/skills/partials/vet-phases.md`,
`resources/modules/skills/partials/review-fanout.md`,
`resources/modules/phases/speccy-work.md`,
`resources/modules/phases/speccy-ship.md`,
`resources/modules/phases/speccy-decompose.md`,
`resources/modules/phases/speccy-init.md`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-004 REQ-005 REQ-006 REQ-007">
## Track 2 — factor six duplicated passages into shared modules and supersede SPEC-0034 DEC-001 (commit 2)

Track 2 is **commit 2** of two and runs **after** T-001 lands, so it factors the
already-compressed Track-1 wording. It is token-neutral and organizational: it
kills inlined copies that shadow a canonical source (AGENTS.md calls these bugs) by
consolidating each enumerated passage into one shared module pulled in via
`{% include "modules/<dir>/<file>.md" %}` at every former callsite. Includes expand
at eject time, so this saves zero runtime tokens — the value is one source of
truth.

**Binding constraints:**
- **Re-locate every anchor against the Track-1-compressed wording.** T-001 changed
  line numbers and phrasings; the anchors below are pre-Track-1 locators. Re-read
  each callsite before splitting.
- **Each new module must expand to text equivalent to the copy it replaced.** Where
  the passage is a one-liner-plus-pointer, the module expands to the full inline
  sentence, never a bare pointer (SPEC-0049 DEC-002). Mirror the existing
  include-only exemplar `references/reconcile-summary.md` (an inline rule one-liner
  + a `{{ speccy_references_path }}/<ref>.md` pointer).
- **The six new modules are include-only.** They need no `.tmpl` wrapper, eject no
  standalone file, and are not subject to the orphan/parity scan (`chk022`). Use the
  `{% include "modules/<dir>/<file>.md" %}` convention; verify `{{
  speccy_references_path }}` / `{{ speccy_personas_path }}` is in scope at any
  callsite whose module body references it.

### T2-A — `references/retry-shape-summary.md` (NEW), mirror `reconcile-summary.md`

Retry-shape is the lone rule summary that was copy-pasted instead of factored.
Create the include-only module carrying the inline one-liner **plus** pointer:

> **Retry shape.** A task is in retry shape iff its journal contains both an
> `<implementer>` element and a `<blockers>` element whose `round` attribute
> matches the highest implementer round. Otherwise it's first-attempt shape — the
> strict clean-tree gate applies. See `{{ speccy_references_path }}/retry-shape.md`
> for the full rule, read-only scope, worked examples, and the "implementer
> awaiting review" edge case.

Replace the three verbatim inline copies with the include at:
`skills/speccy-orchestrate.md` (anchor: `**Retry shape.** A task is in retry shape
iff`), `phases/speccy-work.md` (anchor: `**Retry shape.** A task is in retry` …
`shape iff`), and `skills/speccy-work.md` (anchor: `**Retry shape.** A task is in
retry shape iff`). `{{ speccy_references_path }}` is confirmed in scope at all
three callsites.

### T2-B — `personas/review-role-tail.md` (NEW)

The reviewer "Role" tail — "You append one `<review>` block and return a thin
verdict; the orchestrating skill flips the task's `state` attribute." — is verbatim
in 4 of 7 reviewers (`reviewer-style`, `reviewer-business`, `reviewer-architecture`,
`reviewer-tests`) and paraphrased in the other 3 (`reviewer-security`,
`reviewer-correctness`, `reviewer-docs`). Consolidate to one wording, put it in the
new module, and `{% include %}` it in the `## Role` section of all 7. Keep each
persona's persona-specific Role lead-in sentence; only the shared tail moves.
(`persona_snippets.rs` asserts rendered content, not authorship, so this is safe as
long as the expanded text still reads naturally and the asserted strings survive.)

### T2-C — `personas/vet-input-resolution.md` (NEW)

The vet trio is the only un-factored group. Factor the **consumer-side** shared
block — the `git diff <base-ref>` (no `...HEAD`) rationale plus the fallback
resolution recipe (anchor: `ls -d .speccy/specs/NNNN-*/`) — which is identical
between `personas/vet-reviewer.md` (anchors: `**Use \`git diff <base-ref>\`** (no
\`...HEAD\`)` and the fallback block) and `personas/vet-implementer.md` (same two
anchors). `{% include %}` it from both personas. **Leave `skills/partials/
vet-phases.md` Phase-0 derivation in place** — it *produces* `<base-ref>`
(producer-side), which is different text from the consumer-side block. Re-read all
three before splitting to land the boundary correctly.

### T2-D — `personas/vet-no-rollback.md` (NEW)

"Don't run `git stash` / `reset` / `restore` / `checkout`; the caller owns
rollback." Duplicated in `personas/vet-implementer.md` (under its `## Snapshot
handling` heading) and `personas/vet-simplifier.md`. Factor into the new module and
`{% include %}` it in both personas. **Do not conflate with the existing
reviewer-scoped `personas/no_working_tree_mutation.md`** — that is a distinct
parallel-checkout concern; leave it untouched.

### T2-E — `skills/partials/inline-fanout-rationale.md` (NEW), consolidate-then-factor

"Sub-agents cannot spawn sub-agents, so the fan-out runs inline in the top-level
session." Present in three different wordings in `skills/speccy-orchestrate.md`
(anchor: `**Why fan-outs run inline`), `skills/speccy-vet.md` (anchor: `## Why this
skill runs in a top-level session`), and `skills/speccy-review.md` (anchor: `Because
sub-agents cannot spawn sub-agents`). Pick ONE canonical phrasing — use the
compressed Track-1 wording settled in T-001 (B1/B3/B6) — put it in the new module,
and `{% include %}` it from all three. Each callsite may keep a one-line
site-specific lead-in. In `speccy-orchestrate.md`, verify any back-reference (e.g.
"see 'Why fan-outs run inline' above") still resolves after the include.

### T2-F — `references/spec-self-review-core.md` (NEW), extract shared self-review

Create the include-only module carrying the shared mechanical/semantic preamble
plus the 6 common check properties that `speccy-plan.md` and `speccy-amend.md`
share verbatim (use the Track-1-compressed wording). Add this comment line in the
module (REQ-005):

> `<!-- Shared self-review core for plan + amend; supersedes SPEC-0034 DEC-001 (lists stabilized → extracted). Brainstorm's pre-check is intentionally separate. -->`

`{% include %}` the module in `skills/speccy-plan.md` (anchor: `**Self-review
pass.**`) and `skills/speccy-amend.md` (anchor: `**Self-review pass.**`); amend
keeps its two deltas (Changelog row presence, surgical-diff shape) inline after the
include. **Leave `skills/speccy-brainstorm.md` self-review independent** — its four
artifact-oriented properties are structurally distinct and no partial covers it.
Remove the now-stale `<!-- … independent copy … per DEC-001 / OQ-b … -->` comments
from `skills/speccy-plan.md`, `skills/speccy-amend.md`, and
`skills/speccy-brainstorm.md` (brainstorm's comment references the plan/amend copies
that no longer exist as copies). SPEC.md already carries DEC-001 superseding
SPEC-0034 DEC-001 — no SPEC.md edit is needed; the module comment above is the
remaining REQ-005 obligation.

Keep `authoring_commit.rs`'s plan/amend assertions green: the `**Self-review
pass.**` ordering relative to the commit-recipe include, and the narrow-staging /
title strings, must survive.

### Close-out

Run `just reeject`, then the full hygiene suite — `cargo test --workspace`, `cargo
clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly
fmt --all --check`, `cargo deny check` — all must pass. Diff the ejected packs
against their pre-Track-2 content: the only differences must be include-expansions
whose expanded text equals the prior inline text, every one-liner-plus-pointer
module (T2-A, T2-F where applicable) expands to its full inline sentence rather than
a bare pointer, and no ejected file contains a `{%` marker. Record the SSOT-factoring
audit in `REPORT.md` at ship time.

<task-scenarios>
Given the `resources/modules/**` tree after this task,
when a reviewer inspects each of the six enumerated passages (retry-shape summary,
reviewer Role tail, vet input-resolution, vet no-rollback note, inline-fanout
rationale, plan+amend self-review core),
then each exists as exactly one new include-only module, every former callsite pulls
it via `{% include %}`, and no inline copy of the consolidated text remains at any
non-canonical callsite (CHK-005).

Given a `just reeject` run after this task,
when the ejected packs are diffed against their pre-Track-2 content,
then the only differences are include-expansions whose expanded text equals the
prior inline text, every one-liner-plus-pointer module expands to its full inline
sentence rather than a bare pointer, and no ejected file contains a `{%` marker
(CHK-006).

Given this SPEC's SPEC.md and the `resources/modules/**` tree after this task,
when both are read,
then SPEC.md carries DEC-001 superseding SPEC-0034 DEC-001, the new
`spec-self-review-core.md` module carries the supersession comment,
`skills/speccy-brainstorm.md` retains its independent self-review, and no
`<!-- … DEC-001 / OQ-b … -->` comment remains in `speccy-plan`, `speccy-amend`, or
`speccy-brainstorm` (CHK-007).

Given the four hygiene gates after `just reeject`,
when `cargo test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo +nightly fmt --all --check`, and `cargo deny
check` run,
then all four pass (CHK-008).

Given the ejected `.claude/`, `.agents/`, and `.codex/` packs after this reeject,
when scanned,
then no ejected file contains a `{%` marker and the eject diff is include-expansion
mirroring the `resources/modules/**` edits (CHK-009).

Suggested files: `resources/modules/references/retry-shape-summary.md`,
`resources/modules/personas/review-role-tail.md`,
`resources/modules/personas/vet-input-resolution.md`,
`resources/modules/personas/vet-no-rollback.md`,
`resources/modules/skills/partials/inline-fanout-rationale.md`,
`resources/modules/references/spec-self-review-core.md`,
`resources/modules/skills/speccy-orchestrate.md`,
`resources/modules/skills/speccy-work.md`,
`resources/modules/skills/speccy-vet.md`,
`resources/modules/skills/speccy-review.md`,
`resources/modules/skills/speccy-plan.md`,
`resources/modules/skills/speccy-amend.md`,
`resources/modules/skills/speccy-brainstorm.md`,
`resources/modules/phases/speccy-work.md`,
`resources/modules/personas/vet-reviewer.md`,
`resources/modules/personas/vet-implementer.md`,
`resources/modules/personas/vet-simplifier.md`,
`resources/modules/personas/reviewer-style.md`,
`resources/modules/personas/reviewer-business.md`,
`resources/modules/personas/reviewer-architecture.md`,
`resources/modules/personas/reviewer-tests.md`,
`resources/modules/personas/reviewer-security.md`,
`resources/modules/personas/reviewer-correctness.md`,
`resources/modules/personas/reviewer-docs.md`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-001 REQ-006 REQ-007 REQ-008">
## Reconciliation — retire the brittle feature-dev guardrail and complete the deferred A6/C6 provenance-line deletion (commit 3)

This task lands as **commit 3**, an amendment-driven reconciliation after T-001/T-002.
The 2026-06-12 amendment (DEC-002, REQ-008) authorizes retiring the brittle SPEC-0053
`feature-dev` substring guardrail, which unblocks the Track 1 A6/C6 provenance-line
deletions that T-001 deliberately deferred (deleting them earlier would have reddened
the suite). Edit **source only** — never `.claude/`, `.agents/`, or `.codex/` (ejected
output). After editing, run `just reeject`. The review-pass `git add -A` captures
source edits + regenerated packs in this task's single atomic commit.

### Retire the guardrail (REQ-008)

- In `speccy-cli/tests/skill_packs.rs`, in the test
  `feature_dev_personas_declare_speccy_model_conventions_and_attribution`: delete the
  `feature-dev` attribution assertion block — the
  `// The persona body carries a \`feature-dev\` attribution line.` comment, the
  `read_persona(...)` binding that feeds only this assertion, and the `assert!` on
  `persona_body.contains("feature-dev")`. **Keep** the structural model-convention
  assertions: Claude `model: opus[1m]`, Codex `model = "gpt-5.5"`, and both
  `!...contains("sonnet")` checks. Rename the test to
  `feature_dev_personas_declare_speccy_model_conventions` (drop `_and_attribution`).
  If `read_persona` is left unused inside this test after the deletion, remove the
  now-dangling local use here — but leave the shared `read_persona` helper itself if
  other tests call it.

### Complete the deferred A6/C6 deletion (residual REQ-001)

- Delete the two-line `> Ported from the feature-dev …` provenance blockquote at the
  top of each of `resources/modules/personas/reviewer-correctness.md`,
  `resources/modules/personas/plan-architect.md`, and
  `resources/modules/personas/plan-explorer.md`. Remove any blank line the blockquote
  leaves orphaned so the heading/body spacing stays clean. These are Track 1 anchors
  A6 (reviewer-correctness) and C6 (plan-architect, plan-explorer), now unblocked.

### Close-out

Run `just reeject`, then the full hygiene suite — `cargo test --workspace`, `cargo
clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt
--all --check`, `cargo deny check` — all must pass (the renamed test passes with the
provenance lines gone). Confirm `grep -rn 'Ported from the .feature-dev' resources/
.claude/ .agents/ .codex/` returns nothing (source + eject) and no ejected file
contains a `{%` marker. Record the guardrail retirement (REQ-008) and the completed
A6/C6 cut in `REPORT.md` at ship time.

<task-scenarios>
Given `speccy-cli/tests/skill_packs.rs` and the three persona bodies after this task,
when the test file is read and `cargo test --workspace` runs,
then the `persona_body.contains("feature-dev")` assertion is absent, the test name no
longer carries `_and_attribution`, the model-convention assertions remain, the three
provenance blockquotes are gone from source and eject, and the suite passes (CHK-010).

Given the post-edit tree,
when the four hygiene gates run (`cargo test --workspace`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`,
`cargo deny check`),
then all four pass (CHK-008).

Given the ejected `.claude/`, `.agents/`, and `.codex/` packs after this reeject,
when scanned,
then no ejected file contains a `{%` marker and the eject diff mirrors the
`resources/modules/**` edits — the three deleted provenance blockquotes (CHK-009).

Suggested files: `speccy-cli/tests/skill_packs.rs`,
`resources/modules/personas/reviewer-correctness.md`,
`resources/modules/personas/plan-architect.md`,
`resources/modules/personas/plan-explorer.md`
</task-scenarios>
</task>
