# Speccy Research Folder

Design and research workspace for `speccy`, a spec-driven run controller for coding agents. No code lives here yet; `IMPLEMENTATION-PLAN.md` M0 creates the project elsewhere.

## Doc map and authority

Each topic has exactly one authoritative home. When editing, change the owner and let the other docs point; never restate mechanics or enum values in a second doc.

| Doc | Owns | Status |
| --- | --- | --- |
| `DESIGN.md` | Behavior and mechanics: state machines, gates, caps, lease protocol, resume, evidence rules, storage trees + git policy, CLI surfaces (human + `ctl`), install/update behavior, packs, packet contents, MVP scope, open questions with decision records. | Authoritative |
| `TERMINOLOGY.md` | Vocabulary: term definitions, canonical enum values (run states, task statuses, requirement statuses, spec statuses, risk tiers, human status buckets), naming pairs, ID scopes, lifecycle language, CLI naming guidance. | Authoritative |
| `IMPLEMENTATION-PLAN.md` | Build sequencing and engineering choices not in DESIGN (crate layout, dependencies, git-CLI shell-out, template embedding), milestones M0–M8 with task checklists. | Authoritative for build order |
| `WALKTHROUGH.md` | Illustrative end-to-end scenario with mocked command outputs (install → brainstorm → plan → implement → ship → accept → archive). If it conflicts with DESIGN/TERMINOLOGY, the design docs win. | Illustration |
| `OPEN-ITEMS.md` | Historical decision log from doc reviews. If it conflicts with DESIGN/TERMINOLOGY, the design docs win. | Log |
| `RESEARCH.md` | Ecosystem survey snapshot (2026-06-30). Informational; superseded statements possible. | Snapshot |
| `runtime-state-storage-survey.md` | Prior-art survey of runtime-state storage in comparable tools (2026-07-01). Informational. | Snapshot |
| `spec-driven-orchestration-principles.md`, `long-running-multiagent-orchestration.md`, `The New SDLC With Vibe Coding/` | Source material that seeded the design. | Source |

## Editing rules

- Enum values and their definitions change in `TERMINOLOGY.md` only; DESIGN diagrams may list values but never redefine them.
- Mechanics (how the controller behaves) change in `DESIGN.md` only; TERMINOLOGY entries keep a one-paragraph definition plus a pointer to the owning DESIGN section.
- Command lists (`speccy` human CLI, `speccy ctl` operations, `speccy install` flags) live in `DESIGN.md`; other docs show at most a few examples.
- Decisions get a dated decision record in `DESIGN.md` (inline or in Open Questions) and a log entry in `OPEN-ITEMS.md`.
