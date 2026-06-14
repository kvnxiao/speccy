## Speccy conventions

> Managed by `/speccy-bootstrap`; edits inside this section are
> overwritten on re-run. Put project-specific rules in a sibling
> section.

Speccy keeps intent and shipped behavior in sync through a five-phase
loop. Your harness already surfaces each skill's `description` for
routing — read those for the per-skill contract. The order and entry
points:

1. **Plan** — `/speccy-brainstorm` (fuzzy asks) → `/speccy-plan` →
   `/speccy-decompose`.
2. **Impl** — `/speccy-work`, one task per invocation.
3. **Review** — `/speccy-review`, per-task adversarial fan-out.
4. **Vet** — `/speccy-vet`, the pre-ship holistic drift gate.
5. **Ship** — `/speccy-ship`, writes `REPORT.md` and opens the PR.

`/speccy-orchestrate` drives phases 2–4 autonomously; `/speccy-amend`
handles a mid-loop SPEC change.

Per-task implementer notes and reviewer verdicts live in the journal at
`.speccy/specs/NNNN-slug/journal/T-NNN.md`, sibling to `SPEC.md` and
`TASKS.md`.

CI: wire `speccy verify` into whichever CI the project uses. It fails on
broken proof shape (missing requirement coverage, malformed task state)
and passes when intact — informational by design, not a blocker.
