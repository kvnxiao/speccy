# Vision

## Product

Speccy is a deterministic Rust CLI that lets humans and AI agents
collaborate on software with bounded drift. It makes the contract
between user intent and shipped behavior mechanically visible: every
Requirement maps to >=1 Check, every Check declares what it proves,
and divergence is surfaced (never enforced) so it can be addressed
before it ships.

Speccy is a feedback engine. The Rust CLI is mechanical -- it renders
prompts, queries artifact state, and runs checks. Intelligence lives
at the edges: in skills, prompts, and personas.

## Users

- Solo developers (primarily Kevin) bootstrapping greenfield projects
  with AI assistance and wanting drift-detection without orchestration
  overhead.
- AI coding agents driven by host skill packs (Claude Code, Codex)
  through a Plan -> Tasks -> Impl -> Review -> Report loop.
- (Future) multi-agent harnesses building on Speccy's deterministic
  feedback substrate.

## V1.0 outcome

- Ten-command Rust CLI implementing the surface in `.speccy/DESIGN.md`:
  `init`, `plan`, `tasks`, `implement`, `review`, `report`, `status`,
  `next`, `check`, `verify`.
- Shipped skill packs for Claude Code and Codex that drive the full
  development loop end-to-end without humans chaining commands.
- Speccy's own implementation is tracked in `.speccy/specs/` and
  dogfoods Speccy itself -- by the time the CLI is real, its history
  is the proof that it works.
- `speccy verify` runs as a CI gate that fails on broken proof shape
  and passes when intact, with no flakes attributable to its own
  state.

## Constraints

- The Rust CLI never invokes LLMs. Intelligence lives in skills only.
- No `--strict` mode, no policy file, no configurable enforcement.
- No orchestration runtime -- loops live in skills, not the CLI.
- Five proper nouns: Vision, Spec, Requirement, Task, Check. No new
  first-class entities without removing one first.
- Standard hygiene before any commit lands: `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo +nightly fmt --all --check`, `cargo deny check`.
- Never `unwrap()` / `expect()` / `panic!` / `unreachable!` / `todo!` /
  `unimplemented!` in production code. Tests may use
  `.expect("descriptive message")`.

## Non-goals

- Multi-agent orchestration runtime, distributed locks, worktree
  isolation, ticket queues.
- IDE plugins, dashboard UI, plugin ecosystem.
- Identity provider integration, external tracker sync.
- Production telemetry, mutation testing, semantic dependency
  analysis.
- Greenfield/brownfield mode toggle -- the CLI works identically
  across project states.
- An enforcement layer. Speccy is feedback-only by stance, not by
  oversight.

## Quality bar

- "Useful for my next greenfield" is the bar. Features justified only
  by hypothetical broader audiences are out of scope for v1.
- A solo developer can run `speccy init` in a fresh repo and reach
  their first green check via shipped skills without inventing
  process.
- An AI agent driven by shipped skills can complete a full Plan ->
  Tasks -> Impl -> Review -> Report loop on a non-trivial spec
  without needing humans to chain commands.
- Reviewer personas catch at least one class of drift per review run
  on representative work (proven via dogfooding Speccy on itself).
- Every command has a stable text output and, where contracted, a
  stable JSON output. JSON breaks are versioned via `schema_version`.

## Known unknowns

- The optimal balance between skill-pack richness and CLI determinism
  surfaces only through dogfooding.
- Persona prompt definitions will iterate as host models change; the
  shipped defaults are best-effort starting points.
- Whether the default persona fan-out (business / tests / security /
  style) holds on real work, or whether it needs to be made
  project-configurable before v1.
- Whether the `serde-saphyr` `0.0.x` dependency surfaces
  stabilization pain (API churn, behavioral changes) before Speccy's
  first release.
