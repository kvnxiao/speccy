# AGENTS.md

> Canonical instruction file for AI agents working on **speccy**.
> `CLAUDE.md` is a symlink to this file. On platforms that don't honor
> symlinks, treat the two filenames as required-to-be-identical.

## Mission

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

5. **Stay small.** Five nouns (Vision, Spec, Requirement, Task, Check),
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
