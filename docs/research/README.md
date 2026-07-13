# Speccy Research Folder

Design and research workspace for `speccy`, a spec-driven run controller for coding agents. The implementation is the two-crate Cargo workspace at this repo's root (`speccy-core`, `speccy-cli`); these docs remain the authoritative behavior source.

## Doc map and authority

Each topic has exactly one authoritative home. When editing, change the owner and let the other docs point; never restate mechanics or enum values in a second doc.

| Doc | Owns | Status |
| --- | --- | --- |
| `DESIGN.md` | Behavior and mechanics: state machines, gates, caps, lease protocol, resume, requirement resolution rules, evidence rules, branch/snapshot policy, storage trees + git policy, CLI surfaces (human + `ctl`), install/update behavior, packs, packet contents, MVP scope. Also owns canonical enum values (run states, task statuses, requirement statuses, risk tiers, directive actions), each defined with the state machine that owns it. | Authoritative |
| `TERMINOLOGY.md` | Vocabulary: a compact proper-noun glossary (one line per term, pointing to DESIGN for mechanics), naming discipline, spec status values, and ID scopes. Names the status vocabularies and points to DESIGN for their values. | Authoritative for vocabulary |
| `SCHEMAS.md` | Controller I/O payload shapes: the JSON envelope, the `run next` directive, and every `--input` payload. Implemented; intentionally unstable before 1.0. | Authoritative for payload shapes |
| `IMPLEMENTATION-PLAN.md` | Build sequencing and engineering choices not in DESIGN (crate layout, dependencies, git-CLI shell-out, template embedding), the vertical-slice milestones M0–M6 with task checklists. Sequences the build; DESIGN specifies behavior. | Authoritative for build order |
| `WALKTHROUGH.md` | Illustrative end-to-end scenario with mocked command outputs, sectioned by lifecycle area and bounded by the human checkpoints (setup → plan → implement → ship → accept; archive later when historical context goes stale). Each section leads with what the human types and reads, then shows the controller operations behind it — both the human's-eye view and the protocol reference. If it conflicts with DESIGN/TERMINOLOGY, the design docs win. | Illustration |
| `OPEN-ITEMS.md` | The live surface for undecided work: backlog, open questions (historical Q-numbers), dogfood watch list. | Live backlog |
| `DECISION-LOG.md` | Durable decisions and the alternatives they rejected, grouped by area, in current terminology. Records why, to stop settled questions being re-litigated; not a chronological log. If it conflicts with DESIGN/TERMINOLOGY, the design docs win. | Rationale record |
| `PRINCIPLES.md` | Founding principles: harness neutrality, zero footprint, no outbound agent runner, deterministic core + prose layer. | Source |
| `SOURCE-SUMMARIES.md` | Digest of the three foundational source documents. The other two originals live outside this repo. | Snapshot |

## Editing rules

- Enum values change in `DESIGN.md`, with the state machine that owns them (run states, task statuses, requirement statuses, risk tiers, directive actions); `TERMINOLOGY.md` names the vocabularies and points to those sections, and it does not restate values.
- Mechanics (how the controller behaves) change in `DESIGN.md` only; TERMINOLOGY entries keep a one-line definition plus a pointer to the owning DESIGN section.
- Payload shapes change in `SCHEMAS.md` only; DESIGN and WALKTHROUGH show at most abbreviated examples.
- Command lists (`speccy` human CLI, `speccy ctl` operations, `speccy install` flags) live in `DESIGN.md`; other docs show at most a few examples.
- Design docs state current behavior plainly, without dated decision citations; new decisions get a log entry in `DECISION-LOG.md`, and open/watch items live in `OPEN-ITEMS.md`.
