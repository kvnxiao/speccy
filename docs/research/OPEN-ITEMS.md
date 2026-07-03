# Open Items: Doc Review Backlog

Status: all items resolved
Date: 2026-07-02 (backlog cleared)

Findings from the 2026-07-01 consistency review of `DESIGN.md`, `TERMINOLOGY.md`, and `RESEARCH.md`. All items are decided and applied.

This file is a historical decision log. `DESIGN.md` and `TERMINOLOGY.md` are authoritative for current behavior and vocabulary; if an entry here conflicts with them, the design docs win.

## Backlog

Empty.

## Resolved 2026-07-02

- **Loop-contract gaps closed** (walkthrough review): lease transport specified — `run next --agent <id>` issues the token, state-mutating ops pass `--lease <token>`, and the lease is run-scoped so spec-phase ops are not gated; `verified` surfaces as an `await_human_gate` directive with `record_with: run record-ship`, and `/speccy-ship` takes the lease via `run next` before recording; directives gain a `packet_with` field naming the packet op to build before acting; controller-derived task transitions (`reviewable → in_review`, `needs_repair → building`, `in_review → integrated` + snapshot commit) documented as `run next` side effects, idempotent over settled state; `spec start` input documented (`request` required; optional `source`, `title`, `brainstorm_handoff` — the only persistence point for promoted brainstorms); `evidence collect` classified lease-free; `speccy list --json` is the skills' selector-resolution path. Illustrated end to end in `WALKTHROUGH.md`.

- **`ctl` surface regrouped noun-first** (user decision): operations are `speccy ctl <noun> <verb>`, nouns mirroring `TERMINOLOGY.md` (`spec`, `run`, `task`, `packet`, `evidence`, `finding`, `requirement`) — `run next` (was `next-action`), `spec patch-draft` (was `update-spec-draft`), `requirement set-status` (was `update-requirement-status`), `packet review` (was `build-review-packet`), and so on. Convention recorded in "Controller API Surface" in `DESIGN.md`. Older entries in this log use pre-rename names.

- **Controller surface trimmed to 21 ops** (ctl op review): `run-resume` cut — `next-action` alone clears expired leases and re-derives the directive, giving the loop exactly one entry point; `lint-spec-draft` and `lint-acceptance` cut — write ops return structural lint findings per Q6, `record-spec-decision` refuses approval on a lint-dirty draft, and the `verified` gate enforces status completeness; `record-intake-observations` cut — observations are an optional field on the spec draft; `build-escalation-packet` added; `spec-start` takes `--input request.json`.

- **Pre-implementation consistency fixes** (2026-07-02 doc review before M0): ship transition named `ctl record-ship` (`verified` → `submitted`, records `change_ref`); run-scoped decisions get `ctl record-run-decision`, with spec-level `decisions.jsonl` added to the storage tree; `record-intake-observations` assigned to M2, `build-task-packet` to M3, `record-run-decision`/`record-ship`/`export review` to M6; other exports deferred to dogfooding; `record-evidence` documented lease-free alongside `record-finding` in M5; `run_id`/`spec_id` examples switched to ULID shape; YAML crate switched from archived `serde_yaml` to `serde-saphyr`; DESIGN/TERMINOLOGY headers updated to authoritative/2026-07-02.

- **Design Open Questions 6/15/20/25 closed** (user decisions via structured questions): Q6 strict schema validation with bounded repair (structured lint errors, retry cap default 3, then fail closed to `escalated`); Q15 resource caps fail closed to an `escalated` policy gate (rounds plus optional task-count/wall-clock caps in `project.yaml`; speccy cannot meter tokens); Q20 approved revisions are immutable in place — statements and evidence requests frozen, agents propose, humans approve new revisions, verifiers touch status only; Q25 snapshot + reconcile — escalation commits the in-flight diff as a labeled snapshot, the superseding run reconciles on the same branch, rollback stays the explicit human fallback.

- **Design Open Questions 13/16/17/23 closed** (user decisions): parallel writes explicitly out of MVP scope (serial only, lease-enforced; worktree parallelism stays post-MVP); `run-start` refuses dirty worktrees before any run state exists; non-git workspaces unsupported outright (git required for snapshots/`baseline_commit`; added to Non-Goals); implementation language is Rust as a single static binary with `minijinja` as the intended template engine (distribution/license still open).

- **Survey follow-ups folded into `DESIGN.md`** (from `runtime-state-storage-survey.md`): explicit `baseline_commit` recorded per task at claim time; atomic-write discipline (temp → fsync → rename, verified read-back for JSONL appends) stated as a controller requirement in the Storage Model; Later Capabilities note requiring append-only + union-by-event-id merge driver if mutable state ever becomes git-visible. Storage decision itself unchanged: runtime state stays out of the repo.

6. **Controller-executed command evidence** — split kept, per user decision. For `kind: command`, `speccy ctl collect-evidence` executes the command itself and records exit code/stdout/stderr/hash; `record-evidence` refuses agent-pasted output for that kind. Trust narrows to review/browser/manual kinds, which the risk tiers already treat as weaker. The interface stays uniform: one `collect-evidence` operation, controller decides per kind whether to execute or accept supplied content. Applied to `DESIGN.md` (Acceptance Ledger baseline rules, Verification Ownership) and `TERMINOLOGY.md` (Evidence Artifact).

## Resolved 2026-07-01 (decision pass)

1. **`tiny` name collision** — risk tier renamed to `minimal` (`minimal/standard/high/critical`); scope ladder keeps `tiny -> initiative`. Risk table, vacuity tiers, and naming-pairs table updated.
2. **`speccy resume` semantics** — `speccy resume` cut entirely. Resume is a controller capability: `next-action`/`run-resume` deterministically derive the next step for any fresh session. Mechanisms: task statuses (`queued/building/reviewable/in_review/needs_repair/integrated/deferred`) plus controller-owned round counter, git snapshot commits at each task `integrated` boundary (uncommitted diff = in-flight task work), and expired-lease repair inside `run-resume`/`next-action`. Use Case 7 rewritten; new "Resume and Crash Recovery" section in `DESIGN.md`.
3. **Storage dual-story** — decided: runtime state lives in `~/.speccy/` only. Repo `.speccy/` holds exactly `project.yaml` (config + machine-readable policy values such as repair caps) and `pack-lock.yaml`. No `policies/`, `roles/`, or `evidence-presets/` folders — that prose template-renders into the harness packs. `.gitignore` block kept as a labeled defensive backstop. Open Question 1 closed with a decision record. A survey of how BMAD/Spec Kit/GSD/etc. store runtime state is captured in `runtime-state-storage-survey.md`.
4. **Merge detection** — cut from MVP entirely, including the git-native ancestry heuristics. `speccy accept` is a manual human assertion (optional `--pr`/`--note` provenance). Automatic detection moved to Later Capabilities.
5. **"Accept" overload** — statuses made unique: run state `accepted` renamed `landed`; task terminal state is `integrated` (replacing "accepted for integration"); `accepted` now uniquely means the spec status; "acceptance ledger" stays a noun. Naming-pairs table mandates qualified forms.
7. **Run lease / single-writer enforcement** — specified in `DESIGN.md` ("Run Lease and Concurrent Writers") and `TERMINOLOGY.md` ("Run Lease"): `next-action` issues/renews an agent-bound expiring lease; state-mutating ops require the live token; second session gets `lease_held` naming the holder; expired leases cleared by `run-resume`/`next-action`. `record-finding`/`record-evidence` are lease-free — one file per finding/evidence ID, never a shared journal — so concurrent reviewer personas can complete simultaneously. Prior-round findings carry into subsequent round packets.
8. **MVP scope trim** — trim rejected for harnesses: Claude Code AND Codex stay first-class in MVP, deliberately forcing the template renderer's conditional exports to be real from day one. Recorded in "MVP Proposal."
9. **Non-goal phrasing** — reworded: the non-goal is persisting run state/transcripts/per-spec process artifacts; committed harness packs and `.speccy/` project config are deliberate workflow artifacts.
10. **Factory skill-evolution dependency** — conditional added at both deferral sites: horizons beyond hours make per-repo skill self-evolution a prerequisite, not an enhancement.
11. **Evidence-ability routing** — `evidence_ability` is now the first scope-rating factor; low evidence-ability routes away from `speccy_spec` toward harness planning even for large work, and the handoff must say why. Both docs updated.

## Resolved earlier (2026-07-01 consistency review — do not redo)

- `speccy ctl next-action` deterministic loop-driving operation, with controller-owned round counters.
- Entry-skill lists de-counted and pointed at the canonical "Harness Skills" list.
- `RESEARCH.md` Design Status stripped to an informational deferral; stale differentiator fixed.
- Escalation gate: `escalated -> implementing | verifying | cancelled` edges, "Amendment at the Escalation Gate" flow, amendment supersedes the run via new revision + new run.
- Task graph ownership: planned tasks live on the spec revision; the run instantiates its runtime task graph.
- "Hands-Free Run" removed; every run is fully autonomous by design ("Autonomous Execution" in `TERMINOLOGY.md`).
- Explicit acknowledgement that harness packs are the only available integration level, not just the preferred one.
- Repair-round caps are two policy values (per-task, run-level review) enforced by the controller and surfaced through `next-action`.
