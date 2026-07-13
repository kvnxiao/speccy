# Speccy

A spec-driven run controller for coding agents.

Speccy is a small, deterministic higher layer that coding-agent harnesses
(Claude Code, Codex) call. It does not write code and **never launches an LLM**.
It turns an engineering request into a lightweight spec, an acceptance ledger, a
task sequence, and run state, then delegates implementation and verification
through the harness — returning a compact review packet showing what changed,
what was tested, what drifted, and what still needs human judgment.

The design lives in [`docs/research/`](docs/research/) — `DESIGN.md` owns
behavior, `TERMINOLOGY.md` the vocabulary, `SCHEMAS.md` the payload shapes,
`IMPLEMENTATION-PLAN.md` the build order, and `WALKTHROUGH.md` an end-to-end
scenario.

## How it works

- **Deterministic core, prose layer.** State machines, gates, scheduling, and
  evidence bookkeeping are code. Role behavior and review rubrics are editable
  prose rendered into harness packs.
- **Harness-native.** `speccy install` renders repo-local skills and subagents
  for the detected harness(es). Those skills call `speccy ctl …` to drive a run.
- **The loop is one operation.** Install-pack skills repeatedly call
  `speccy ctl run next`, which returns the single next directive (claim a task,
  dispatch a worker, dispatch verifier personas, await a human gate, or halt).
  Sequencing, round counting, snapshots, and gate detection are controller
  decisions, never prose decisions.
- **Evidence, not narrative.** The controller executes `kind: command` evidence
  itself and refuses agent-pasted output; `passed` requires recorded evidence;
  a deterministic provenance scan keeps Speccy identifiers out of shipped code.
- **Zero product-code footprint.** Runtime state lives in `~/.speccy/`
  (override `SPECCY_HOME`); only the rendered packs plus `.speccy/project.yaml`
  and `.speccy/pack-lock.yaml` are committed.

## Usage

```bash
speccy install            # render repo-local harness packs (previews first)
speccy status             # one card per active run
speccy review [--evidence]# the state-aware human packet
speccy accept             # record that a submitted change landed
speccy doctor             # check git, the store, and pack freshness
```

Routine work happens inside the harness via the installed skills
(`/speccy-brainstorm`, `/speccy-plan`, `/speccy-implement`, `/speccy-ship`).
The `speccy ctl …` surface is machine-facing and rarely typed by hand.

## Security posture

`kind: command` evidence runs in the **live workspace** with the session's
inherited environment. The `evidence.command_policy.allow` list is a drift
guardrail — it refuses commands nobody meant to declare — **not** an
authorization boundary: the active harness sandbox is what authorizes what an
approved command may do. Speccy does not add its own command sandbox. What it
does add is accountability: every command's process tree is contained and torn
down before its evidence is recorded, and the exact before/after repository
identity (HEAD, dirty paths, worktree-diff hash) is captured so a command that
mutates the workspace can never pass as a clean check.

## Build

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

Rust 2021, MIT licensed. Git operations shell out to the `git` CLI; templates
are embedded and rendered with `minijinja`.
