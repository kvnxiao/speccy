# AGENTS.md

> Canonical instruction file for AI agents working on **speccy**.
> `CLAUDE.md` is a symlink to this file. On platforms that don't honor
> symlinks, treat the two filenames as required-to-be-identical.

## Product north star

> This is the project-wide product context — what we're building, who
> for, what "done enough to ship v1" looks like. The `speccy-init`
> skill writes (or updates) this section when bootstrapping a
> greenfield repo. Speccy itself has no separate `VISION.md`; the
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

Long-term, speccy is the substrate underneath multi-agent harnesses
that move projects toward completion without humans re-explaining
intent at every step.

### Users

- Solo developers bootstrapping greenfield projects with AI assistance
  who want drift detection without orchestration overhead.
- AI coding agents driven by host skill packs (Claude Code, Codex)
  through a Plan → Tasks → Impl → Review → Report loop.
- (Future) multi-agent harnesses building on Speccy's deterministic
  feedback substrate.

### V1.0 outcome

- Ten-command Rust CLI implementing the surface in
  `.speccy/ARCHITECTURE.md`: `init`, `plan`, `tasks`, `implement`,
  `review`, `report`, `status`, `next`, `check`, `verify`.
- Shipped skill packs for Claude Code and Codex driving the full
  development loop end-to-end without humans chaining commands.
- Speccy's own implementation tracked in `.speccy/specs/` — by the
  time the CLI is real, its history is the proof that it works.
- `speccy verify` runs as a CI gate that fails on broken proof shape
  and passes when intact, with no flakes attributable to its own
  state.

### Quality bar

"Useful for my next greenfield" is the bar. Features justified only
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
- Loader-switch bootstrap friction: `speccy implement <SPEC>/<TASK>`
  can't render its own prompt when the task is to swap the workspace
  loader (surfaced during SPEC-0022 T-007). A direct
  `task_xml::parse`-on-spec-folder fallback would fix it, but the
  shape of the right escape hatch isn't decided yet.

Non-goals and the full list of "what we deliberately don't do" are
catalogued in `.speccy/ARCHITECTURE.md`'s "What We Deliberately Don't
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
   ten commands, no mode toggles, no orchestration runtime. Speccy
   works identically in any project state — there is no
   greenfield/brownfield distinction.

6. **Surface unknowns; never invent.** Ambiguous spec → stop and
   surface it. Can't validate something → say so. Don't fabricate
   check commands. Don't add agent-behavior knobs to the CLI.

## Where the design lives

`.speccy/ARCHITECTURE.md` is the only source of truth for the schema, CLI
surface, lint codes, and implementation sequence. Read it before
touching any code. If a design decision isn't documented, ask before
deciding.

## Authoritative references

These rule files are authoritative for their domains. Load them when
editing files in scope.

- `.claude/rules/rust/*.md` — Rust conventions (error handling, testing,
  linting, dependencies, unsafe, workspaces, performance, documentation,
  defensive programming, code quality). Conflicts between this file and
  a rule are bugs in *this* file — fix this file, not the rule.
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
  undocumented step), do this:
  update the relevant skill file under `skills/` before you finish
  the task, then call out the edit under `Procedural compliance` in
  your implementer handoff note. Speccy dogfoods this loop: the same
  friction-to-skill-update pattern the shipped implementer prompt
  asks downstream users to follow applies here, so the next
  contributor inherits the fix instead of re-discovering it.
