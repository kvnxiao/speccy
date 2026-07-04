# Speccy Implementation Plan: Rust Walking Skeleton

Status: roadmap, not started
Date: 2026-07-04

Build sequencing for the Speccy MVP. This doc sequences the build; `DESIGN.md` and `TERMINOLOGY.md` specify behavior. Every milestone names a deliverable and points to the DESIGN section that owns it — when a bullet and DESIGN disagree, DESIGN wins, so this doc never restates mechanics, enum values, or transition rules. Resolved prerequisites (project home, license, resolution rules, gate enumeration, workspace identity, payload shapes) are in `DECISION-LOG.md`; the milestones assume them.

Each milestone is a one-line goal, its deliverables, and one **Done when** line — the single observable behavior that proves the slice works. Exhaustive per-behavior assertions live in the tests.

The sequence is **vertical-slice-first**: M1 builds the thinnest real end-to-end loop so the product's "does this feel lightweight?" test arrives at M1, not at the end. Later milestones thicken each layer. Dogfood runs as recurring pressure after M1 and M4, not only as a final exam.

The project is not scaffolded yet. M0 runs `cargo init` at this repo's root.

## Build choices not in DESIGN

DESIGN owns product decisions (Rust + `minijinja`, JSONL store, statuses, gates). This doc owns only build-level choices DESIGN is silent on:

- Single cargo package: `src/lib.rs` with modules `store`, `state`, `ops`, `lease`, `gitx`, `evidence`, `render`, `cli`, plus `src/main.rs`. Split into a workspace only when it hurts.
- Git operations shell out to the `git` CLI via `std::process`; no `gix`/`libgit2` dependency.
- Templates embedded in the binary (`rust-embed` or `include_dir`), rendered strict-undefined.
- Core dependencies: `clap`, `serde`, `serde_json`, `serde-saphyr` (YAML; `serde_yaml` is archived), `minijinja`, `thiserror`/`anyhow`, `sha2`, `jiff`, `fd-lock` or `fs4` (lease, store, and command lock files), `ulid`, `rust-embed`, `similar` (three-way merge, M4+), `insta` (golden tests, dev).

## Milestones

### M0 — Thin shell

Goal: a CLI that compiles behind quality gates.

- [ ] `cargo init` at the repo root (lib + bin), `.gitignore`, `rustfmt.toml`, clippy config
- [ ] clap CLI skeleton: `speccy --version`, `speccy doctor` (stub), `speccy ctl` subcommand tree returning structured not-implemented JSON errors
- [ ] JSON envelope (`{ok, data | error{code, message, details[]}}`) and error taxonomy (`thiserror`: `lease_held`, `validation_failed`, `invalid_transition`, `not_a_git_repo`, `dirty_worktree`, …) — § Controller API Surface; shapes in `SCHEMAS.md`
- [ ] CI + `clippy -D warnings` gate on the primary platform (cross-OS matrix arrives with the store at M1)

**Done when:** `cargo clippy -- -D warnings && cargo test` pass and `speccy ctl run next --json` returns a structured not-implemented error.

### M1 — Minimal real loop

Goal: prove the whole product loop end-to-end before hardening internals. Single agent, no lease contention, review-kind evidence only — the trust layer, concurrency, and exception paths come later. Git is real here (not stubbed) so `integrated` snapshots and resume are the production mechanics from day one.

- [ ] Store: workspace identity + layout, `SPECCY_HOME` override; atomic whole-file writes (temp→fsync→rename) and append JSONL; event enum + replay projection; `project.yaml` load with defaults — § Storage Model
- [ ] IDs: `SPEC-YYYYMMDD-XXXX` public refs; opaque ULID `spec_id`/`run_id`
- [ ] Spec drafting: `spec start`/`status`, `record-draft`/`patch-draft` (structural lint), `record-decision` → approved + immutability, auto-open revision N+1 — § Acceptance Ledger, § Spec Draft and Run State; payloads in `SCHEMAS.md`
- [ ] Run loop: `run start` (gates: approved revision, git repo, clean worktree; instantiates task graph), `run next` (directive engine, derived transitions, `applied_transitions`, closed action vocabulary), `task claim`/`record-handoff`, `packet task` — § Deterministic Loop Driving, § Task
- [ ] Minimal-real git: `gitx` repo/dirty/HEAD/branch/commit/diff; run branch create + recorded base; `baseline_commit` at claim; snapshot commit at `integrated` — § Run Branch and Snapshot Policy
- [ ] Single review path: `evidence record` (review kind), `finding record`, `requirement set-status`, `packet verification`, `packet review` — § Requirement Resolution Rules, § Review Packet
- [ ] Minimal human CLI: `speccy status`, `speccy review`

**Done when:** a fake harness drives request → spec card → approve → task → snapshot → `verified` review packet through the real CLI, and killing the session mid-task then re-invoking `run next` resumes correctly because the snapshot is real.

→ **Recurring dogfood #1 (CLI-only, fake harness):** run a toy spec through the loop and file friction to `OPEN-ITEMS.md` before thickening.

### M2 — Trust the loop

Goal: make `verified` mean something, and make concurrent and interrupted runs safe.

- [ ] Run lease: issue/renew via `run next --agent`, `--lease <token>` on mutating ops, `lease_held` on contention, expired-lease clearing + resume attribution — § Run Lease and Concurrent Writers, § Resume and Crash Recovery
- [ ] Store hardening: per-workspace store lock over event appends; fail-closed truncated-tail detection naming the byte offset — § Storage Model
- [ ] Out-of-band commit detection → `escalated` policy gate; resume derivation (task status + round + dirtiness vs snapshot) with dirty-diff attribution — § Run Branch and Snapshot Policy, § Resume and Crash Recovery
- [ ] Command evidence: `evidence collect` (shell exec, timeout/byte caps, hashing, before/after dirty-state, `--requirements`/`--requests`) + workspace command lock — § Acceptance Ledger, § Run Lease and Concurrent Writers
- [ ] Evidence trust rules: `evidence record` refuses agent-pasted command output; artifact required for browser/api on high/critical; full requirement resolution rules + `verified` gate — § Requirement Resolution Rules
- [ ] Findings carried forward into next-round verification packets — § Repeat Review Rounds
- [ ] Provenance scan over task and integrated diffs — § Provenance Hygiene

**Done when:** a run cannot reach `verified` without recorded evidence for every requirement, pasted command output is refused, a second agent gets `lease_held`, and the provenance scan flags a seeded leak in a product file while ignoring it in a pack file.

### M3 — Human surface (CLI-complete)

Goal: the full human-facing endpoints, where ceremony is actually felt.

- [ ] Packets: `packet escalation`; state-aware `packet review` buckets (Proven / Accepted risk / Needs you) — § Review Packet, § Escalation Packet
- [ ] Run decisions: `run record-decision` (incl. `rework` → `implementing` + `RT<n>`, `defer`, `carry_forward`) and `run record-ship` (`verified` → `submitted`, records `change_ref`) — § Carry-Forward Decisions
- [ ] Human CLI: `status`, `list` (+`--query`/`--json` selector resolution), `review` (+`--evidence`), `accept` (idempotent, uses recorded `change_ref`), `archive`, `cancel`, `new`, `export review`, with selector inference — § CLI/Admin Flow
- [ ] Planning-packet prior-context candidates over active (non-archived) specs — § Carry-Forward Decisions

**Done when:** `review` renders the right state-aware packet and infers the current spec unambiguously, `rework` returns the run to `implementing` with an appended `RT<n>`, and `accept` is idempotent after landing.

### M4 — Renderer + install packs + personas

Goal: the two-harness forcing function (Claude Code + Codex both first-class) and the full reviewer roster.

- [ ] `minijinja` strict-undefined environment with markdown/YAML-frontmatter and TOML escaping filters
- [ ] Embedded template bundle: shared partials + Claude/Codex overlays, conditional exports, context (`target`, `capabilities`, `names`, `paths`, `controller`, `pack`) — § Harness-Aware Template Rendering
- [ ] Entry skills (brainstorm, plan, implement, ship) + role/subagent prompts; `/speccy-plan` route preflight; structured-question tool only in entry skills — § Harness Skills
- [ ] Reviewer persona fan-out: full roster (`spec-fidelity`, `defects`, `security`, `style`) rendered one subagent per persona per target with per-persona `model`, fanned out at task and run-gate review — § Reviewer Personas
- [ ] `speccy install`: harness auto-detect, idempotent create/repair, would-write preview, `--target`/`--check`/`--dry-run`, writes `project.yaml` + `pack-lock.yaml` + defensive `.gitignore` block — § Install Flow
- [ ] `speccy install --update`: three-way merge over rendered outputs; conflicts to `.speccy/pack-updates/<timestamp>/`

**Done when:** golden render tests pass for both targets, install-twice is idempotent, `--check` catches hash drift, and a roster change in `project.yaml` adds/removes persona files on re-install.

→ **Recurring dogfood #2 (in-harness):** install packs into a toy repo and run one real spec through Claude Code and Codex; file friction to `OPEN-ITEMS.md`.

Note: `/speccy-ship` PR opening is harness-side prose; the controller's ship support is only `run record-ship` (M3).

### M5 — Exception paths

Goal: add the non-happy-path branches without bloating the main loop. Split by what `WALKTHROUGH.md` exercises.

- [ ] Amendment at the escalation gate: supersede-on-approval, run branch reuse, escalation snapshot — § Run Branch and Snapshot Policy, § Capability Escalation and Give-Up Policy
- [ ] Critical-tier accepted-risk confirmation gate — § Requirement Resolution Rules
- [ ] Command allow policy (lint at `record-draft`, refuse at `evidence collect`) — § Acceptance Ledger
- [ ] Capability escalation / give-up: cap exhaustion, blocked requirement, resource caps → `escalated` — § Capability Escalation and Give-Up Policy

**Done when:** every non-happy path in `WALKTHROUGH.md` has an integration test.

### M6 — Hardening + dogfood

Goal: decide what deserves to survive, then use it for real.

- [ ] Crash-matrix test: kill the fake harness at each loop phase, resume, assert the directive
- [ ] Concurrency stress: multi-process event append; concurrent `finding record`/`evidence record`; command-lock serialization
- [ ] Golden render tests for every managed file × both targets (`insta`)
- [ ] Full fake-harness E2E through the real CLI: spec start → approve → run next loop → verified → ship stub → accept
- [ ] `speccy doctor` full checks: git present, store writable, packs fresh
- [ ] Real dogfood on one toy repo; convert dogfood friction into cuts before adding features

**Done when:** `cargo test` is green including E2E and crash matrix, one dogfooded spec reaches `verified` with an honest review packet, and anything that felt ceremonial is cut, deferred, or justified.

## Sequencing notes

- M0 → M1 → M2 are strictly ordered. M3 and M4 both depend on M2 and are independent of each other. M5 depends on M2. M6 needs everything.
- Deferred MVP scope (decision index, delta-scoped re-review, command-evidence dedup, automatic merge detection, and the rest) is owned by § Later Capabilities in `DESIGN.md` — not restated here.
- Deferred-to-dogfood open questions (Q3 artifact shape, Q7 vacuity threshold, Q11 packet format, Q18 redaction model, exports beyond `export review`) live in `OPEN-ITEMS.md`.
- Prior context is split from the archived-inclusive decision index: M3 delivers prior-context candidates over active specs; the decision index is deferred to Later Capabilities (it must land before multi-spec dogfooding, which the single-spec dogfood does not reach).
