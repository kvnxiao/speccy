# AGENTS.md

> Canonical instruction file for AI agents working on **speccy**.
> `CLAUDE.md` is a symlink to this file. On platforms that don't honor
> symlinks, treat the two filenames as required-to-be-identical.

## Mission

Speccy is a lightweight, deterministic CLI that lets humans and AI agents
collaborate on software with bounded drift. It exists because LLM
non-determinism accumulates: small misreadings of intent compound over
features until what shipped no longer matches what was asked for.

Speccy does not try to make LLMs deterministic. It makes the **contract**
between user intent and shipped behavior machine-checkable, so drift
becomes loud the moment it happens.

Long-term, speccy is the deterministic substrate underneath any
multi-agent harness that wants to take a greenfield app from zero to v1.0
without a human re-explaining the intent at every step.

## Core principles

The durable beliefs. Schema and CLI will evolve; these shouldn't.

1. **Behavior contracts, not lifecycle ceremony.** Speccy enforces "did
   you prove the requested behavior?", not "did you follow the workflow?"
   The load-bearing artifact is the requirement → scenario → validation
   chain. Status states are bookkeeping.

2. **Deterministic core, intelligent edges.** The CLI is thin, mechanical,
   predictable. The intelligence — what to build, how to structure it,
   how to test it — lives in skills, prompts, and Spec narratives. Don't
   bake agent behavior into Rust code that gets worse as models improve.

3. **Proof strength matters.** `command = "true"` is not proof. A test
   that asserts `1 == 1` is not proof. Validations carry an assurance
   tier; raw shell is a low-assurance escape hatch, never the sole
   completion proof for a phase.

4. **Make drift loud, fast.** Every requirement must be covered by ≥1
   non-trivial validation. Every covering validation must have fresh
   evidence on HEAD's sha before a phase can close. Lint rules are
   mechanical and predictable — never LLM-graded.

5. **Stay small.** Speccy is a substrate for multi-agent orchestration,
   not the orchestrator. No in-process agent runner. No GitHub/Linear
   sync in core. No TUI. If a feature gets worse as models get better,
   it doesn't belong here.

6. **Surface unknowns; never invent.** If a Spec is ambiguous, stop and
   surface it. Don't silently pick between interpretations. If you can't
   validate something, say so — don't fake a validation.

## Where the design lives

`DESIGN.md` at repo root is the source of truth for the schema, CLI
surface, lint codes, and implementation sequence. Read it before touching
any code. If a design decision isn't in `DESIGN.md`, ask before deciding.

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
