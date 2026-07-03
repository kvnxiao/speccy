# Speccy Research Folder

Design and research workspace for `speccy`, a spec-driven run controller for coding agents. Code lives at this repo's root once `IMPLEMENTATION-PLAN.md` M0 scaffolds the Rust crate.

## Doc map and authority

Each topic has exactly one authoritative home. When editing, change the owner and let the other docs point; never restate mechanics or enum values in a second doc.

| Doc | Owns | Status |
| --- | --- | --- |
| `DESIGN.md` | Behavior and mechanics: state machines, gates, caps, lease protocol, resume, requirement resolution rules, evidence rules, branch/snapshot policy, storage trees + git policy, CLI surfaces (human + `ctl`), install/update behavior, packs, packet contents, MVP scope, open questions with decision records. | Authoritative |
| `TERMINOLOGY.md` | Vocabulary: term definitions, canonical enum values (run states, task statuses, requirement statuses, spec statuses, risk tiers, human status buckets), naming pairs, ID scopes, lifecycle language, CLI naming guidance. | Authoritative |
| `SCHEMAS.md` | Controller I/O payload shapes: the JSON envelope, the `run next` directive, and every `--input` payload. Provisional until M2 implements them. | Authoritative for payload shapes |
| `IMPLEMENTATION-PLAN.md` | Build sequencing and engineering choices not in DESIGN (crate layout, dependencies, git-CLI shell-out, template embedding), the M0 readiness checklist, milestones M0–M8 with task checklists. | Authoritative for build order |
| `WALKTHROUGH.md` | Illustrative end-to-end scenario with mocked command outputs (install → brainstorm → plan → implement → ship → accept → archive). If it conflicts with DESIGN/TERMINOLOGY, the design docs win. | Illustration |
| `OPEN-ITEMS.md` | Historical decision log from doc reviews. If it conflicts with DESIGN/TERMINOLOGY, the design docs win. | Log |
| `PRINCIPLES.md` | Founding principles: harness neutrality, zero footprint, no outbound agent runner, deterministic core + prose layer. | Source |
| `SOURCE-SUMMARIES.md` | Digest of the three foundational source documents. The other two originals live outside this repo. | Snapshot |

## Editing rules

- Enum values and their definitions change in `TERMINOLOGY.md` only; DESIGN diagrams may list values but never redefine them.
- Mechanics (how the controller behaves) change in `DESIGN.md` only; TERMINOLOGY entries keep a one-paragraph definition plus a pointer to the owning DESIGN section.
- Payload shapes change in `SCHEMAS.md` only; DESIGN and WALKTHROUGH show at most abbreviated examples.
- Command lists (`speccy` human CLI, `speccy ctl` operations, `speccy install` flags) live in `DESIGN.md`; other docs show at most a few examples.
- Decisions get a dated decision record in `DESIGN.md` (inline or in Open Questions) and a log entry in `OPEN-ITEMS.md`.
