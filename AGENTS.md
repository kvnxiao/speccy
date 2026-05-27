# AGENTS.md

> Canonical instruction file for AI agents working on **speccy**. `CLAUDE.md` is a symlink to this file. On platforms that don't honor symlinks, treat the two filenames as required-to-be-identical.

## Product north star

> This is the project-wide product context — what we're building, who
> for, what "done enough to ship v1" looks like. The `speccy-init`
> skill writes (or updates) this section when bootstrapping a
> new repo. Speccy itself has no separate `VISION.md`; the
> always-loaded `AGENTS.md` carries this content for every prompt.
>
> The word "Mission" is reserved for the Speccy noun (a focus-area
> grouping under `.speccy/specs/[focus]/MISSION.md`). Don't conflate.
> Mission folders are **optional**: a flat single-focus project may
> have zero `MISSION.md` files, with every spec living directly
> under `.speccy/specs/NNNN-slug/`. Mission folders earn their
> existence only when 2+ related specs share enough context that
> loading them together at plan time is cheaper than rediscovering
> it.

Speccy is a lightweight CLI that lets humans and AI agents collaborate
on software with bounded drift. LLM non-determinism accumulates: small
misreadings of intent compound until what shipped no longer matches
what was asked for.

Speccy does not try to make LLMs deterministic. It makes the contract
between intent and shipped behavior **visible**, so drift is loud the
moment it happens. Speccy is a feedback engine, not an enforcement
system.

Speccy's shipped skills and subagent packs drive the per-task work + review
loop and the pre-ship drift gate end-to-end on a single host. Beyond that, speccy
aims to be the substrate underneath cross-host and cross-repository
harnesses that move projects toward completion without humans
re-explaining intent at every step.

### Users

- Solo developers bootstrapping new projects with AI assistance
  who want drift detection without orchestration overhead.
- AI coding agents driven by host skill packs (Claude Code, Codex)
  through a Plan → Tasks → Impl → Review → Report loop.
- Multi-agent harnesses building on Speccy's deterministic feedback
  substrate — the in-pack orchestration loop ships in v1; cross-host
  and cross-repository harnesses remain future work.

### V1.0 outcome

- A lean Rust CLI implementing the surface in
  `docs/ARCHITECTURE.md`. The surface is intentionally small — see
  the `## Core principles` "Stay small" rule — but the exact command
  list lives in the architecture doc, not in this north star, so
  additions like `archive` do not require this section to churn.
  Phase prompts (plan / tasks / implement / review / report) live in
  the shipped skill bodies, not in the CLI; the binary never renders
  natural-text prompts.
- Shipped skill packs for Claude Code and Codex driving the full
  development loop end-to-end without humans chaining commands.
- A shipped orchestration loop in both skill packs
  (`/speccy-orchestrate` chained with `/speccy-vet`) that
  drives one SPEC from first-task implementation through pre-ship
  drift review without humans chaining per-task commands.
- Speccy's own implementation tracked in `.speccy/specs/` — by the
  time the CLI is real, its history is the proof that it works.
- `speccy verify` runs as a CI gate that fails on broken proof shape
  and passes when intact, with no flakes attributable to its own
  state.

### Quality bar

"Useful for my next project" is the bar. Features justified only
by hypothetical broader audiences are out of scope for v1.

- A solo developer can run `speccy init` in a fresh repo and reach
  their first green check via shipped skills without inventing process.
- An AI agent driven by shipped skills can complete a full
  Plan → Tasks → Impl → Review → Report loop on a non-trivial spec
  without humans chaining commands.
- Reviewer personas catch at least one class of drift per review run
  on representative work (proven via dogfooding Speccy on itself).
- Every command has stable text output and, where contracted, stable
  JSON output. JSON breaks are versioned via `schema_version`.

### Known unknowns

- Optimal balance between skill-pack richness and CLI determinism
  surfaces only through dogfooding.
- Persona prompt definitions will iterate as host models change;
  shipped defaults are best-effort starting points.
- Whether the default persona fan-out (business / tests / security /
  style) holds on real work, or whether it needs to become
  project-configurable before v1.
- Whether the `serde-saphyr` `0.0.x` dependency surfaces stabilization
  pain (API churn, behavioral changes) before Speccy's first release.

Non-goals and the full list of "what we deliberately don't do" are
catalogued in `docs/ARCHITECTURE.md`'s "What We Deliberately Don't
Do" table. Constraints are catalogued in `## Core principles` below
and in `## Standard hygiene`.

## Core principles

Durable beliefs. Schema and CLI will evolve; these shouldn't.

1. **Feedback, not enforcement.** Speccy makes drift visible; it does
   not block agents from making mistakes. `speccy verify` fails CI when
   proof shape is broken. Everything else is informational. No
   `--strict` mode, no policy file, no configurable enforcement.

2. **Deterministic core, intelligent edges.** The CLI is mechanical:
   renders prompts, queries state, runs checks. Workflow loops, persona
   definitions, and "what to do next" intelligence live in the skill
   layer. The Rust CLI does not call LLMs.

3. **Proof shape, not proof scores.** Every Requirement maps to ≥1
   Check; every Check declares what it proves. The CLI flags one
   structural anti-pattern (no-op commands as sole proof). Everything
   else about check quality goes to review.

4. **Review owns semantic judgment.** Multi-persona adversarial review
   (business, tests, security, style by default) is where drift gets
   caught. Personas live as markdown skills; the CLI just renders
   their prompts. Speccy never tries to grade tests algorithmically.

5. **Stay small.** Five nouns (Mission, Spec, Requirement, Task, Check),
   a small, flat command surface (see `docs/ARCHITECTURE.md` for the
   current list), no mode toggles, no orchestration runtime. Speccy
   works identically in any project state — there is no
   greenfield/brownfield distinction.

6. **Surface unknowns; never invent.** Ambiguous spec → stop and
   surface it. Can't validate something → say so. Don't fabricate
   check commands. Don't add agent-behavior knobs to the CLI.

## Where the design lives

`docs/ARCHITECTURE.md` is the only source of truth for the schema, CLI
surface, lint codes, and implementation sequence. Read it before
touching any code. If a design decision isn't documented, ask before
deciding.

## Skill pack source of truth

Everything an agent sees as a skill, subagent, or reference file
under `.claude/`, `.agents/`, or `.codex/` is **ejected output**, not
source. The single source of truth lives under `resources/`:

- `resources/modules/skills/*.md` — host-neutral skill bodies
  (`speccy-orchestrate.md`, `speccy-vet.md`, …).
- `resources/modules/skills/partials/*.md` — sharable skill fragments
  (`vet-phases.md`, `review-fanout.md`).
- `resources/modules/phases/*.md` — agent body templates used by
  subagent wrappers (`speccy-work.md`, `speccy-ship.md`, …).
- `resources/modules/personas/*.md` — reviewer/vet persona bodies
  plus shared persona snippets (`verdict_return_contract.md`,
  `inline_note_format.md`, `diff_fetch_command.md`,
  `no_tasks_md_writes.md`).
- `resources/modules/references/*.md` — shared rule files
  (`reconcile-policy.md`, `retry-shape.md`, `evidence.md`,
  `journal-*.md`, …).
- `resources/agents/.<host>/…` — per-host wrappers (`*.md.tmpl` for
  Claude Code and Codex skills; `*.toml.tmpl` for Codex subagents).
  Wrappers carry frontmatter and pull module bodies in via MiniJinja
  `{% include %}` directives at render time.

`speccy init --force --host <host>` (or `just reeject` to refresh
both Claude Code and Codex at once) renders every wrapper, expands
includes, and writes the result to `.claude/`, `.agents/`, and
`.codex/`.

**Editing rule:** never edit files under `.claude/`, `.agents/`, or
`.codex/` directly. Edit the source under `resources/` and then
run `just reeject` to regenerate the ejected output. Any change
made to the harness folders is overwritten on the next init.

**Deduplicating snippets:** when the same text would appear in
multiple wrappers or modules, extract it to a `resources/modules/…`
file and `{% include %}` it from each callsite. Verbatim inlined
copies that shadow a canonical source are a bug — replace them with
the include.

## Authoritative references

These rule files are authoritative for their domains. Load them when
editing files in scope.

- `.claude/rules/rust/*.md` — Rust conventions (code quality, defensive
  programming, dependencies, documentation, performance, workspaces).
  Conflicts between this file and a rule are bugs in *this* file — fix
  this file, not the rule.
- `.claude/rules/github-actions/*.md` — CI workflow conventions (action
  versioning, runner selection, caching).

## Standard hygiene

Before any commit lands, all four must pass:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +nightly fmt --all --check`
- `cargo deny check`

## Conventions for AI agents specifically

- Identify yourself in commits via the `Co-Authored-By` trailer.
- Prefer narrow, well-scoped commits over sprawling ones.
- If a test you wrote is flaky, investigate the flake — don't retry
  until green.
- Never `unwrap()` / `expect()` / `panic!()` / `unreachable!()` /
  `todo!()` / `unimplemented!()` in production code. Tests may use
  `.expect("descriptive message")`.
- Don't index with `[i]` on slices, `Vec`, or `serde_json::Value`. Use
  `.get(i)` and handle the `Option`.
- Don't add `#[allow(...)]` to silence a lint. Use
  `#[expect(..., reason = "...")]` so the suppression is auto-removed
  when the underlying issue resolves.
- If you're tempted to add agent-behavior knobs to the CLI, stop — that
  belongs in skills or prompts, not in deterministic code.
- When you hit friction caused by a stale or wrong instruction in a
  shipped skill (wrong command, missing environment variable, an
  undocumented step), do this: update the relevant source module
  under `resources/modules/` (or wrapper under `resources/agents/`)
  before you finish the task, run `just reeject`, then call out the
  edit under `Procedural compliance` in your `<implementer>` block.
  Never patch the ejected file under `.claude/`, `.agents/`, or
  `.codex/` directly — see `## Skill pack source of truth`. Speccy
  dogfoods this loop: the same friction-to-skill-update pattern the
  shipped implementer prompt asks downstream users to follow applies
  here, so the next contributor inherits the fix instead of
  re-discovering it.

## Implementer / reviewer activity records

Implementer handoff prose, reviewer verdicts, and amendment-driven
blocker directives live in `.speccy/specs/NNNN-slug/journal/T-NNN.md`
— a per-task journal file sibling to `SPEC.md` and `TASKS.md`. The
journal carries the closed-set XML elements `<implementer>`,
`<review>`, and `<blockers>` under a small YAML frontmatter
(`spec`, `task`, `generated_at`). These elements do not appear
inside `<task>` bodies in `TASKS.md`; the parser rejects them
there. See `docs/ARCHITECTURE.md` "TASKS.md per-task journal"
for the full grammar, attribute schemas, the `JNL-001` / `JNL-002`
/ `JNL-003` lint family, and the `TSK-006` "no journal elements in
TASKS.md" rule.
