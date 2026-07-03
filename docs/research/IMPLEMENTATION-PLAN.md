# Speccy Implementation Plan: Rust Walking Skeleton

Status: roadmap, not started
Date: 2026-07-02

Milestone breakdown for building the Speccy MVP walking skeleton. `DESIGN.md` and `TERMINOLOGY.md` are authoritative for behavior; this document only sequences the build. The project is not scaffolded yet — M0 creates it.

## Engineering decisions

Product decisions — language and template engine (Rust + `minijinja`, Open Question 23), the JSONL-first storage model, statuses, gates — live in `DESIGN.md`. This section records only build-level choices not specified there:

- Single cargo package: `src/lib.rs` with modules `store`, `state`, `ops`, `lease`, `gitx`, `evidence`, `render`, `cli`, plus `src/main.rs`. Split into a workspace only when it hurts.
- Git operations shell out to the `git` CLI via `std::process`; no `gix`/`libgit2` dependency.
- Templates embedded in the binary (`rust-embed` or `include_dir`), rendered in strict-undefined mode.
- Core dependencies: `clap`, `serde`, `serde_json`, `serde-saphyr` (YAML; `serde_yaml` is archived), `minijinja`, `thiserror`/`anyhow`, `sha2`, `jiff` or `chrono`, `fd-lock` or `fs4` (lease file lock), `ulid`, `rust-embed`, `similar` (three-way merge, M7+), `insta` (golden tests, dev).

## Milestones

### M0 — Scaffold

Goal: compilable CLI shell with quality gates.

- [ ] `cargo new speccy` (lib + bin), git init, `.gitignore`, `rustfmt.toml`, clippy config
- [ ] clap CLI skeleton: `speccy --version`, `speccy doctor` (stub), `speccy ctl` subcommand tree (stubs returning structured not-implemented JSON errors)
- [ ] JSON output convention: every `ctl` op returns `{ok, data | error{code, message, details[]}}`
- [ ] Error taxonomy (`thiserror`): `lease_held`, `validation_failed`, `invalid_transition`, `not_a_git_repo`, `dirty_worktree`, …

Verify: `cargo build && cargo clippy -- -D warnings && cargo test`; `speccy ctl run next --json` returns a structured not-implemented error.

### M1 — Run store core

Goal: crash-safe JSONL store under `~/.speccy/`.

- [ ] Workspace identity: derive `workspace_id` from canonicalized repo root; layout `~/.speccy/workspaces/<id>/specs/<spec_id>/runs/<run_id>/`
- [ ] ID generation: `SPEC-YYYYMMDD-XXXX` public refs; opaque ULID-based `spec_id`/`run_id`
- [ ] Atomic write helpers: temp → fsync → rename for whole files; append + fsync + verified read-back for JSONL
- [ ] Event model: serde enum of controller events; replay → in-memory projection; corrupt/truncated-tail detection that fails closed and names the byte offset
- [ ] `.speccy/project.yaml` load with defaults (repair caps, structured-output retry cap, optional task-count/wall-clock caps)

Verify: replay determinism (same events → same projection); truncated-tail rejection; atomic-write crash simulation (interrupt between temp and rename leaves prior state intact).

### M2 — Spec drafting ops

Goal: draft → lint → approve, with immutability.

- [ ] `ctl spec start` (input: `request` required; optional `source`, `title`, `brainstorm_handoff`), `ctl spec status`
- [ ] Minimal spec-draft schema: goal, scope in/out, risk tier (`minimal|standard|high|critical`), requirements (id, statement, evidence request kind), planned tasks, optional intake observations
- [ ] `ctl spec record-draft` (whole candidate), `ctl spec patch-draft` (patch-style); both return structural lint findings (duplicate requirement IDs, missing evidence requests, invalid risk tier, …)
- [ ] `ctl spec record-decision` → approved revision; refuses approval while the draft is lint-dirty; post-approval mutation attempts rejected with `invalid_transition` (Q20)
- [ ] `ctl packet planning` (deterministic, minimal: request, draft state, git signals, output contract)

Verify: test walk draft → lint-fail → patch → lint-pass → approve → mutation refused; malformed payloads return structured lint errors, never panics.

### M3 — Run state machine, next-action, lease

Goal: the deterministic loop heart.

- [ ] `ctl run start` gates: revision approved, workspace is a git repo, clean worktree; instantiates the runtime task graph from the approved revision
- [ ] Run states (`created` → `landed`), task statuses (`queued` → `deferred`), controller-owned round counters, caps read from `project.yaml`
- [ ] `ctl run next`: idempotent directive engine emitting `{run_state, action, subject, round{current,max,scope}, packet_with, record_with, reason}`; applies controller-derived task transitions (`reviewable` → `in_review`, `needs_repair` → `building`, `in_review` → `integrated` + snapshot) before answering; escalation directive on cap exhaustion
- [ ] Run lease: issued/renewed by `run next --agent <id>`, agent-bound token with expiry (OS file lock + lease record); state-mutating ops require the live token via `--lease <token>`; `lease_held` error names the holder; `run next` clears expired leases
- [ ] `ctl packet task`, `ctl task claim` (→ `building`), `ctl task record-handoff` (→ `reviewable`), `ctl run status`

Verify: full happy-path state walk; two concurrent fake agents (second gets `lease_held`); expired-lease takeover; `run next` idempotency (call twice without recording → identical directive); repair-cap exhaustion → `escalated`.

### M4 — Git integration

Goal: snapshots and resume invariants.

- [ ] `gitx` module shelling out to `git`: repo detection, dirty check, HEAD, commit, diff
- [ ] `baseline_commit` recorded at `task claim`; snapshot commit at `integrated`; labeled escalation snapshot before parking
- [ ] Resume derivation: task status + round counter + worktree dirtiness vs last snapshot → "resume partial task, diffed against `baseline_commit`" or "dispatch fresh"
- [ ] Refusals: `not_a_git_repo` and `dirty_worktree` at `run start`

Verify: integration tests against temp git repos — both refusal paths; baseline/snapshot SHAs recorded; kill mid-task then `run next` returns the correct directive with the partial diff attributed.

### M5 — Evidence and verification ops

Goal: the trust layer.

- [ ] `ctl evidence collect`: for `kind: command` the controller spawns the command and captures exit code/stdout/stderr, hashes and stores the artifact; cwd/env/timeout handling
- [ ] `ctl evidence record`: refuses agent-supplied output for `kind: command`; accepts review/browser/manual kinds
- [ ] `ctl finding record`: lease-free like `evidence record`, one file per finding/evidence ID (safe for concurrent reviewer personas)
- [ ] `ctl requirement set-status` with legal-transition checks; `verified` gate refuses while any requirement is unresolved
- [ ] `ctl packet verification`; prior-round findings carried into next-round packets
- [ ] Secret hygiene stub: env scrubbing for stored command output (full redaction model stays open, Q18)

Verify: command evidence executes and hashes; pasted command output refused; N simultaneous `finding record`/`evidence record` writes all land; `verified` refused while a requirement is `pending`; round-2 packet contains round-1 findings.

### M6 — Review/escalation packets + human CLI

Goal: the human-facing endpoints.

- [ ] `ctl packet review` (markdown; Proven / Accepted risk / Needs you buckets)
- [ ] `ctl packet escalation`: requirement-scoped; approaches tried per round, partial work applied, one closing question
- [ ] `ctl run record-decision` (run-scoped gate decisions and waivers), `ctl run record-ship` (`verified` → `submitted`, records `change_ref`)
- [ ] Human CLI: `status`, `list` (+ `--query` selectors, `--json` for skill-side selector resolution), `review`, `accept` (`submitted` → `landed`, optional `--pr`/`--note`), `archive`, `cancel`, `new` (minimal), `export review` (writes the review packet to an explicit destination)
- [ ] Spec selector resolution: full ref resolves exactly; free text searches titles/slugs; ambiguity prints a numbered list

Verify: golden-file tests for both packets; `run record-ship` persists `change_ref` and moves the run to `submitted`; `accept`/`archive` transitions; selector disambiguation.

### M7 — Renderer + install packs

Goal: the two-harness forcing function (Claude Code + Codex both first-class).

- [ ] `minijinja` environment: strict undefined, deterministic output, custom filters for markdown/YAML-frontmatter escaping
- [ ] Template bundle embedded in the binary; shared partials + Claude/Codex overlays; conditional exports; template context per DESIGN (`target`, `capabilities`, `names`, `paths`, `controller`, `pack`)
- [ ] Entry-skill templates (brainstorm, plan, implement, ship) + role/subagent prompts — thin prose driving the `next-action` cycle with the defensive rules from DESIGN
- [ ] `speccy install`: harness auto-detect (`.claude`/`.codex`/`.agents`), idempotent create/repair, `--target`, `--check`, `--dry-run`; writes `.speccy/project.yaml`, `pack-lock.yaml` (pack version + source template IDs + source/rendered SHA-256 + capability flags), defensive `.gitignore` block
- [ ] `speccy install --update`: three-way merge over rendered outputs; conflicts written to `.speccy/pack-updates/<timestamp>/` (may slip to M8)

Verify: golden render tests for every managed file × both targets (`insta` snapshots); install-twice idempotency; hash drift caught by `--check`.

### M8 — End-to-end walking skeleton + dogfood

Goal: prove the loop, then use it.

- [ ] Fake-harness integration test: a script plays the harness — spec start → draft → approve → run start → run next loop (claim, handoff, verify, evidence) → verified → ship stub → accept — via the real CLI against a temp git repo, asserting every state transition and the final review packet
- [ ] Crash-matrix test: kill the fake harness at each loop phase, resume, assert the returned directive is correct
- [ ] `speccy doctor` full checks: git present, store writable, packs fresh
- [ ] Dogfood: install packs into a toy repo, run one real spec through Claude Code; file friction as new `OPEN-ITEMS.md` entries

Verify: `cargo test` green including E2E and crash matrix; one real dogfooded spec reaching `verified` with an honest review packet.

## Sequencing notes

- M1–M5 are strictly ordered. M6 and M7 are independent of each other and can be reordered or parallelized. M8 requires everything.
- Deliberately deferred to dogfooding: Q3 artifact shape (schemas stay serde-internal, no public format promise), Q7 vacuity threshold (M5 ships only the adversarial-review prose), Q11 packet format (markdown only), Q18 full redaction model, and all exports except `speccy export review` (`export spec`/`export run-bundle` wait for real need).
- `/speccy-ship` PR opening is harness-side prose; the skeleton's ship support is only `ctl run record-ship` (the `submitted` transition plus `change_ref` recording).
- Project location is undecided; M0 starts wherever the repo is created.
