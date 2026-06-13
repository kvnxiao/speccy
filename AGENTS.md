# AGENTS.md

> Canonical instruction file for AI agents working on **speccy**. `CLAUDE.md` is
> a symlink to this file; on platforms without symlinks, keep the two identical.

## Product north star

> Maintained by the `speccy-init` skill; Speccy has no separate `VISION.md`.
> Note: "Mission" is reserved for the Speccy noun (a focus-area grouping under
> `.speccy/specs/[focus]/MISSION.md`). Mission folders are optional and earn
> their existence only when 2+ related specs share plan-time context.

Speccy is a lightweight CLI that lets humans and AI agents collaborate on
software with bounded drift. LLM non-determinism accumulates: small misreadings
of intent compound until what shipped no longer matches what was asked for.
Speccy doesn't try to make LLMs deterministic — it makes the contract between
intent and shipped behavior **visible**, so drift is loud the moment it happens.
A feedback engine, not an enforcement system.

Shipped skills and subagent packs drive the per-task work + review loop and the
pre-ship drift gate end-to-end on a single host. Beyond that, Speccy aims to be
the substrate for cross-host and cross-repository harnesses that move projects
toward completion without humans re-explaining intent at every step.

### Users

- Solo developers bootstrapping new projects with AI assistance who want drift
  detection without orchestration overhead.
- AI coding agents driven by host skill packs (Claude Code, Codex) through a
  Plan → Tasks → Impl → Review → Report loop.
- Multi-agent harnesses building on Speccy's deterministic feedback substrate
  (in-pack orchestration ships in MVP; cross-host and cross-repo harnesses are
  future work).

### Minimal viable product

- A lean Rust CLI implementing the surface in `docs/ARCHITECTURE.md`. Phase
  prompts live in the shipped skill bodies; the binary never renders
  natural-text prompts that skills and subagent bodies already cover.
- Skill packs for AI harnesses driving the full development loop, including an
  orchestration loop (`/speccy-orchestrate` chained with `/speccy-vet`) from
  first-task implementation through pre-ship drift review, without humans
  chaining commands.
- Speccy's own implementation tracked in `.speccy/specs/` — its history is the
  proof that it works.
- `speccy verify` as a CI gate: fails on broken proof shape, passes when intact,
  no flakes attributable to its own state.

### Quality bar

"Useful for my next project" is the bar. Features justified only by hypothetical
broader audiences are out of scope for the MVP.

- A solo developer can run `speccy init` in a fresh repo and reach a first green
  check via shipped skills without inventing process.
- An agent completes a full Plan → Tasks → Impl → Review → Report loop on a
  non-trivial spec without humans chaining commands.
- Reviewer personas catch at least one class of drift per review run on
  representative work (proven by dogfooding Speccy on itself).
- Every command has stable text output and, where contracted, stable JSON
  output; JSON breaks are versioned via `schema_version`.

### Known unknowns

- The right balance between skill-pack richness and CLI determinism (only
  dogfooding will tell).
- Persona prompts will iterate as host models change; shipped defaults are
  starting points.
- Whether the default persona fan-out holds on real work or needs to become
  project-configurable.

Non-goals live in `docs/ARCHITECTURE.md` → "What We Deliberately Don't Do".

## Core principles

Durable beliefs; schema and CLI will evolve, these shouldn't.

1. **Feedback, not enforcement.** Speccy makes drift visible; it does not block
   mistakes. `speccy verify` fails CI on broken proof shape; everything else is
   informational. No `--strict` mode, no policy file, no configurable
   enforcement.

2. **Deterministic core, intelligent edges.** The CLI is mechanical: renders
   prompts, queries state, runs checks. Workflow loops, personas, and "what
   next" intelligence live in the skill layer. The CLI never calls LLMs.

3. **Proof shape, not proof scores.** Every Requirement maps to ≥1 Check; every
   Check declares what it proves. The CLI flags structural breakage only
   (requirement with no scenario, empty scenario body, dangling references);
   check _quality_ goes to review.

4. **Review owns semantic judgment.** Multi-persona adversarial review is where
   drift gets caught. Personas are markdown skills; the CLI just renders their
   prompts. Speccy never grades tests algorithmically.

5. **Stay small.** Five nouns (Mission, Spec, Requirement, Task, Check), a small
   flat command surface (list in `docs/ARCHITECTURE.md`), no mode toggles, no
   orchestration runtime, no greenfield/brownfield distinction.

6. **Surface unknowns; never invent.** Ambiguous spec → stop and surface it.
   Can't validate → say so. Don't fabricate check commands.

## Where the design lives

`docs/ARCHITECTURE.md` is the only source of truth for the schema, CLI surface,
lint codes, and implementation sequence. Read it before touching code. If a
design decision isn't documented, ask before deciding.

## Skill pack source of truth

Everything under `.claude/`, `.agents/`, and `.codex/` is **ejected output**,
never source. Source lives under `resources/`:

- `resources/modules/skills/*.md` — host-neutral skill bodies.
- `resources/modules/skills/partials/*.md` — shared skill fragments.
- `resources/modules/phases/*.md` — agent body templates used by subagent
  wrappers.
- `resources/modules/personas/*.md` — reviewer/vet persona bodies plus shared
  persona snippets.
- `resources/modules/references/*.md` — shared rule files.
- `resources/agents/.<host>/…` — per-host wrappers (`*.md.tmpl` for skills,
  `*.toml.tmpl` for Codex subagents) carrying frontmatter and pulling module
  bodies in via MiniJinja `{% include %}`.

Template variables wrappers may reference are catalogued in
`docs/ARCHITECTURE.md` → "Per-host template variables".

Rules:

- **Never edit `.claude/`, `.agents/`, or `.codex/` directly.** Edit
  `resources/` and run `just reeject` (or `speccy init --force
--host <host>`
  per host); ejected edits are overwritten on the next init.
- **Deduplicate snippets.** Text shared by multiple wrappers or modules goes in
  a `resources/modules/…` file, `{% include %}`d from each callsite. Inlined
  copies shadowing a canonical source are bugs.
- **Acceptance criteria for prose edits check content, not size.** "Does the
  wrapper include the right module?" is fine; "wrapper ≤ N lines" gates add
  friction without catching anything content checks don't.
- **Keep ejected content lean.** Modules and wrappers eject into users' repos
  and reload into agent context on every prompt. Write what an agent needs to
  act on; cut meta-annotation (cross-file source-of-truth notes,
  module-relationship explanations, rationale that doesn't change behavior).
  Authoring judgment, not a test gate.
- **Read-only tool grants are Claude Code-only.** The read-only agents
  (`plan-explorer`, `plan-architect`, `reviewer-*`, `vet-reviewer`) declare a
  read-only `tools:` grant in their Claude Code wrapper frontmatter. Codex
  honors no per-subagent tool restriction (per its config reference,
  developers.openai.com/codex/config-reference, 2026-05) — its read-only posture
  is prose-enforced via each persona body. Don't add a `tools` field to Codex
  `.toml` wrappers expecting it to be honored.

### Authoring resource prose

Bodies under `resources/modules/**` are pseudocode-in-English, not essays. They
reload into agent context on every prompt and eject into users' repos, so
terseness is correctness. Every body — and every `.tmpl` wrapper — follows one
shape:

1. **Discovery layer (wrapper frontmatter).** `name` is kebab-case and matches
   the folder; `description` is verb-first and carries both a "Use when …" and
   a "Do NOT trigger …" clause. This routing metadata is load-bearing — keep it
   sharp.
2. **No redundant preamble.** The body opens straight into logic; it does not
   restate the frontmatter description.
3. **Numbered phases / logic blocks.** Number phases and sub-number logic
   blocks. Lead with defensive early-exits so the happy path reads flat; order
   each block precondition → action → exit transition. No nested `if → if`.
4. **CLI steps name their success signal.** Where a step's success is not
   self-evident, append one terse `→ expected: <signal>` clause — one clause,
   never a paragraph.
5. **Explicit Exit / Return contract.** Every skill body ends with what it
   produced (artifact written, state flip, journal append, commit) and where
   control goes next.
6. **Progressive disclosure.** Skill and phase bodies stay focused; canonical
   shapes and long worked examples live under `modules/references/`, pulled in
   via `{% include %}`. No body inlines a shape a reference owns.
7. **Examples are concrete worked instances.** The `modules/references/` files
   carry one coherent worked example — the `SPEC-0042` widget-render-timeout
   walkthrough — with concrete ids (`REQ-001`, `T-001`, `CHK-001`, `DEC-001`),
   so `<task id="T-001" covers="REQ-001 REQ-002">` reads as an example, not a
   blank. Slots, rule variables, command args (`speccy journal show
   SPEC-NNNN/T-NNN`), and path templates (`.speccy/specs/NNNN-slug/…`) stay
   placeholders even inside references.

**No artifact-ID provenance outside references.** In every body that is *not* a
worked-instance reference — skills, phases, personas, partials, shared rule
files — never cite a real Speccy SPEC/REQ/DEC/task as rationale: an agent in
another repo has no idea what a concrete id refers to, and the citation invites
hallucination. Use only the generic placeholders `SPEC-NNNN` / `REQ-NNN` /
`DEC-NNN` / `T-NNN` / `CHK-NNN`. The `modules/references/` worked instance is
the sole carve-out (item 7) — concrete `REQ`/`DEC`/`CHK`/`T` ids and the
whitelisted `SPEC-0042` are allowed there. Two bans hold everywhere, references
included: any SPEC id other than `SPEC-0042`, and CLI lint codes cited by
number.

**CLI lint codes describe behavior, not history.** Name what a lint does ("the
spec-hash-mismatch lint", "the misplaced-journal-element lint"); don't cite its
code (`TSK-003`, `JNL-001`) — references included.

**Enforced by** `speccy-cli/tests/resource_prose_hygiene.rs`: the ID-ban lint
over `resources/modules/**`, which carves out the `modules/references/`
directory for the worked-instance ids per item 7.

## Authoritative references

Load these when editing files in their scope:

- `.claude/rules/rust/*.md` — Rust conventions. Conflicts between this file and
  a rule are bugs in _this_ file — fix this file, not the rule.
- `.claude/rules/github-actions/*.md` — CI workflow conventions.

## Standard hygiene

Before any commit lands, all four must pass:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

## Conventions for AI agents specifically

- Identify yourself in commits via the `Co-Authored-By` trailer.
- Prefer narrow, well-scoped commits over sprawling ones.
- If a test you wrote is flaky, investigate the flake — don't retry until green.
- Don't write vacuous tests. A test must gate a real invariant of the system
  under test. Specifically, never:
  - Substring-match human-curated prose (`AGENTS.md`, `README.md`, `SPEC.md`
    bodies) — that gates editorial decisions and breaks on legitimate rewrites.
    Enforce doc concepts via review or a lint over a stable structural surface
    (section IDs, frontmatter fields) instead.
  - Re-assert a hard-coded copy of a production constant — it can only prove
    both sites were updated in sync. Derive a property (length, ordering, prefix
    relation) or delete the test.
  - Assert only file existence or non-emptiness — `read_to_string` failing
    already gates readability.
  - Mock the function under test and assert the mock was called.
  - Assert outcomes so loose any input passes (`is_ok()` on an infallible
    function, `!is_empty()` on a function that always returns non-empty).

  Heuristic: "If I delete this test, what real regression goes uncaught?" If
  none, the test is vacuous.

- Never `unwrap()` / `expect()` / `panic!()` / `unreachable!()` / `todo!()` /
  `unimplemented!()` in production code. Tests may use
  `.expect("descriptive message")`.
- Don't index slices, `Vec`, or `serde_json::Value` with `[i]`; use `.get(i)`
  and handle the `Option`.
- Don't silence lints with `#[allow(...)]`; use `#[expect(..., reason = "...")]`
  so the suppression auto-expires when the underlying issue resolves.
- Agent-behavior knobs belong in skills or prompts, never in the CLI.
- Never put real Speccy identifiers (spec IDs, slugs, repo URLs, branch names)
  in shipped template / reference / skill bodies — they land in other people's
  repos via `speccy init`. Use obviously fictional placeholders (`SPEC-0042`,
  `0042-example-slug`, `acme/widget`, `feature/example-branch`) labeled
  "illustrative example — substitute your own values." Worked-instance
  references under `resources/modules/references/` are the one place the
  concrete `SPEC-0042` artifact-id family (`REQ-001`, `T-001`, `CHK-001`,
  `DEC-001`, slug `0042-widget-render-timeout`) is load-bearing rather than
  placeholder — see "Authoring resource prose" item 7. Speccy's own artifacts
  under `.speccy/specs/` are local dogfood evidence and stay Speccy-specific.
- Never cite, as the reason a line of production code, test, or comment
  exists, a Speccy id (SPEC/REQ/CHK/DEC/task — `// per REQ-NNN`, `//! Tests
  for SPEC-NNNN T-NNN`) or a governance/design doc (`(Core principle 2)`, `per
  AGENTS.md`, `see docs/ARCHITECTURE.md`, a rule-file pointer). Speccy is a
  means to produce the code, not a part of it, and the code should not be
  coupled to the docs that govern it — once either is removed the citation
  references nothing, so it is drift the moment it lands. Requirement→evidence
  traceability lives in the journal `Evidence:` field and CHK roll-call
  (`.speccy/specs/NNNN-slug/journal/T-NNN.md`), not in the source tree. Keep
  the reasoning a comment carries; drop the bare id or doc pointer. Naming an
  artifact the code actually operates on (`SPEC.md`, `TASKS.md`, a `.speccy/…`
  path) is data, not meta-annotation. (This governs production code and tests;
  the sibling "No artifact-ID provenance outside references" rule under
  "Authoring resource prose" governs resource bodies.)
- Hit friction from a stale or wrong instruction in a shipped skill (wrong
  command, missing env var, undocumented step)? Fix the source module under
  `resources/` before finishing the task, run `just reeject`, and call out the
  edit under `Procedural
compliance` in your `<implementer>` block. Never patch
  the ejected file.

## Implementer / reviewer activity records

Implementer handoffs, reviewer verdicts, and blocker directives live in
`.speccy/specs/NNNN-slug/journal/T-NNN.md` (sibling to `SPEC.md` and
`TASKS.md`): YAML frontmatter (`spec`, `task`, `generated_at`) plus the
closed-set elements `<implementer>`, `<review>`, and `<blockers>`. These
elements are rejected inside `TASKS.md` `<task>` bodies (`TSK-006`). Full
grammar, attribute schemas, and the `JNL-001`/`JNL-002`/`JNL-003` lints:
`docs/ARCHITECTURE.md` → "TASKS.md per-task journal".
