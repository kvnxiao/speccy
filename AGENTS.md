# AGENTS.md

This file provides guidance to AI agents working with code in this repository. `CLAUDE.md` is a symlink to this file; always edit `AGENTS.md`.

## What Speccy is

A spec-driven **run controller** for coding-agent harnesses (Claude Code, Codex). It turns an engineering request into a spec, an acceptance ledger, a task sequence, and run state, then delegates implementation and verification back through the harness — returning a compact review packet.

Two invariants constrain every change. Break either and the design is violated:

- **Never launches an LLM.** No `speccy` command, subcommand, or code path may call an LLM, coding agent, or AI harness. The harness calls Speccy; Speccy is deterministic. (`PRINCIPLES.md`)
- **Zero product-code footprint.** Runtime state lives in `~/.speccy/` (override `SPECCY_HOME`), never the repo. Shipped file contents carry no Speccy provenance — see the provenance scan (`src/provenance.rs`, DESIGN § Provenance Hygiene).

## Commands

```bash
cargo build
cargo test --all                              # unit + integration (tests/)
cargo test --test e2e                         # one integration test file
cargo test --test hardening golden_all_managed_files  # one test by name
cargo clippy --all-targets -- -D warnings     # CI gate; warnings are errors
cargo fmt --all --check                        # CI gate
cargo insta review                            # accept/reject golden snapshot changes
```

CI runs fmt + clippy + `cargo test --all` on Linux, macOS, and Windows. Git-backed tests need a git identity configured.

**Snapshot tests** (`insta`) live in `tests/snapshots/*.snap` and cover rendered pack files for both harness targets. Changing a template under `templates/` will fail these until you review with `cargo insta review` and commit the updated `.snap`.

## Architecture

Two layers by design (`PRINCIPLES.md`, DESIGN § Architecture):

1. **Deterministic core** (this Rust crate) — state machines, gates, scheduling, evidence bookkeeping, git snapshots. All code.
2. **Prose layer** — role behavior and review rubrics, authored as `templates/*.j2` and rendered into repo-local harness packs the harness executes. Editable prose, not code.

### The loop is one operation

Speccy runs no agents, so it does not drive the loop — the installed harness skills do, by repeatedly calling one controller operation:

```bash
speccy ctl run next --run <id> --agent <id> --json
```

`run next` reads run state and returns the single next directive (`claim_task`, `dispatch_worker`, `dispatch_verifier`, `await_human_gate`, or `halt`). It is also the **only mutation point for derived state**: it clears expired leases, applies transitions that have no recording operation, and reports them in `applied_transitions`. Sequencing, round counting, cap enforcement, and gate detection are controller decisions — never prose decisions. `run next` is idempotent: an unrecorded directive is returned again verbatim (the `lease`, `applied_transitions`, and `resume` fields are excluded from that comparison). This is what makes crash resume work — see `src/directive.rs`.

### Event-sourced store

The source of truth is an append-only `events.jsonl` per spec and per run under `~/.speccy/workspaces/<id>/`. Read models (`src/projection.rs`) are rebuilt by replaying events; there is no mutable state file to corrupt. Appends use a per-workspace lock and verified read-back so a crash never leaves a half-written transition. Serial writes are enforced by a run-level **lease** (`src/lease.rs`); concurrent reviewer findings are additive and lease-free.

### CLI surface

Two families, both in `src/cli.rs`:

- `speccy ctl <noun> <verb>` — machine-facing controller ops the packs call. Always emit the JSON envelope (`{ok, data | error{code,message,details}}`); a failed op prints its envelope and exits nonzero. Dispatched by `src/ops.rs`.
- `speccy <verb>` — human-facing (`status`, `review`, `accept`, `list`, `new`, `install`, `doctor`, …). Render text (or `--json`). Handled by `src/humancli.rs`.

### Module map

| Module | Responsibility |
| --- | --- |
| `ops.rs` | `ctl` dispatch + operation logic; validates every write against a schema |
| `directive.rs` | `run next` — directive engine and derived-transition mutation point |
| `store.rs` / `event.rs` / `projection.rs` | append-only JSONL store, event vocabulary, replay read models |
| `model.rs` | domain types and canonical enum vocabularies |
| `lease.rs` | run-level one-writer lease |
| `evidence.rs` | `evidence collect` — controller executes `kind: command` evidence itself |
| `provenance.rs` | deterministic scan keeping Speccy identifiers out of shipped code |
| `gitx.rs` | git via `git` CLI shell-out (no libgit2); snapshots, branches, diffs |
| `packets.rs` | deterministic work-order / review packets |
| `render.rs` / `install.rs` | `minijinja` harness-aware pack rendering; `speccy install` |
| `lint.rs` | structural lint of spec drafts |
| `config.rs` | `.speccy/project.yaml` policy load |
| `humancli.rs` / `cli.rs` / `main.rs` | human commands, clap surface, binary entry |

## Design docs are authoritative — edit the owner

`docs/research/` is the design workspace, and **each topic has exactly one authoritative home**. When you change behavior, change the owning doc; do not restate mechanics or enum values in a second doc. Before implementing a change to behavior, read the owning doc — it decides, not the code.

- `DESIGN.md` — behavior and mechanics: state machines, gates, caps, lease, resume, evidence rules, branch/snapshot policy, storage, CLI surfaces, packs, packet contents, MVP scope. **Owns all canonical enum values** (run states, task/requirement statuses, risk tiers, directive actions), each defined with its state machine.
- `TERMINOLOGY.md` — vocabulary glossary; names status vocabularies, points to DESIGN for values.
- `SCHEMAS.md` — controller I/O payload shapes: the JSON envelope, the `run next` directive, every `--input` payload.
- `IMPLEMENTATION-PLAN.md` — build order, milestones M0–M6, build-level choices DESIGN is silent on.
- `WALKTHROUGH.md` — illustrative end-to-end scenario. If it conflicts with DESIGN/TERMINOLOGY, they win.
- `DECISION-LOG.md` — durable decisions and rejected alternatives, to stop settled questions being re-litigated.
- `OPEN-ITEMS.md` — live backlog, open questions, dogfood watch list.
- `PRINCIPLES.md` — founding principles (source).

When adding an enum value or a new transition: change `DESIGN.md` first (with the state machine that owns it), then implement in `model.rs` / `directive.rs`. When changing a payload: change `SCHEMAS.md` first.

## Conventions

- Rust rules live in `.claude/rules/rust/` (code quality, defensive programming, dependencies, performance, documentation) and GitHub Actions pinning in `.claude/rules/github-actions/`. Follow them; they are not restated here.
- Preferred crates already in use: `jiff` (not chrono/time), `serde-saphyr` (YAML; serde_yaml is archived), `minijinja`, `ulid`, `sha2`, `fs4`, `similar`. Single package for now — split to a workspace only when it hurts (IMPLEMENTATION-PLAN § Build choices).
- Keep changes surgical and small. The product thesis is *less ceremony*; more code and comments become reviewer context overload at the review gate.
