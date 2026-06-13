## Speccy conventions

> Managed by `/speccy-init`; edits inside this section are
> overwritten on re-run. Put project-specific additions in a sibling
> section.

### When to use which skill

- `/speccy-init` — bootstrap a new Speccy workspace by scaffolding
  `.speccy/` and seeding both the product north star and this
  conventions section into `AGENTS.md`. Run once per project before
  any other `speccy-*` skill. Re-running refreshes this section.
- `/speccy-brainstorm` — atomize a fuzzy ask into first-principle
  requirements before any `SPEC.md` is written. Use when the user
  says "help me brainstorm", "let's think about X", or when the
  scope is unclear. Stops at a hard gate until the framing is
  user-approved.
- `/speccy-plan` — draft a new `SPEC.md` from the product north
  star. Use when the user says "write a spec", "draft a SPEC", or
  "spec out X". Requires `.speccy/` and `AGENTS.md`.
- `/speccy-amend` — orchestrate a mid-loop SPEC change. Edits
  `SPEC.md` with a Changelog row, reconciles `TASKS.md`, and
  re-records the spec hash. Use when requirements shift or
  `speccy` reports the SPEC and tasks are out of sync.
- `/speccy-decompose` — decompose a SPEC into a checklist of
  agent-sized tasks in `TASKS.md`, or reconcile the list after an
  amendment. Use when the user says "break the spec into tasks" or
  the task list looks stale.
- `/speccy-work` — implement one Speccy task per invocation. With
  an optional `SPEC-NNNN/T-NNN` selector, implements that task;
  without one, resolves the next implementable task. Use when the
  user says "implement T-NNN" or "work the next task".
- `/speccy-review` — review one Speccy task per invocation by
  fanning out adversarial multi-persona review (business, tests,
  security, style by default). Passes the task to `completed` or
  flips it back to `pending` with a blockers block in the journal.
- `/speccy-vet` — run a holistic SPEC-vs-implementation drift
  review at the pre-ship boundary, with an autonomous drift-fix
  retry loop and a simplifier polish pass. Use when the user says
  "check for drift before shipping".
- `/speccy-ship` — close out a Speccy spec: write `REPORT.md`,
  run `speccy verify`, commit, and open a pull request. Use when
  every task is `state="completed"`.
- `/speccy-orchestrate` — drive the full implementation + review
  loop for one SPEC end-to-end by chaining `/speccy-work`,
  `/speccy-review`, and `/speccy-vet` until the spec is
  ready-to-ship. Stops one step before shipping so the operator
  can decide.

### The dev loop

Speccy work moves through five phases:

1. **Plan** — draft `SPEC.md` (`/speccy-plan`, optionally preceded
   by `/speccy-brainstorm`).
2. **Tasks** — decompose into agent-sized work (`/speccy-decompose`).
3. **Impl** — implement one task at a time (`/speccy-work`).
4. **Review** — adversarial per-task review (`/speccy-review`),
   followed by holistic pre-ship drift review (`/speccy-vet`).
5. **Ship** — produce the report and open the PR (`/speccy-ship`).

Per-task implementer notes, reviewer verdicts, and blocker
directives all live in a per-task journal file at
`.speccy/specs/NNNN-slug/journal/T-NNN.md`, sibling to `SPEC.md`
and `TASKS.md`. Inspect that file to follow the conversation
between implementer and reviewer rounds for any given task.

### Test hygiene

A test must gate a real invariant of the system under test — not
editorial decisions, not its own source constant, not the build's
own ability to compile. Do not write any of the following vacuous
shapes:

1. **Substring-matching human-curated prose.** Asserting that a
   specific sentence appears in a hand-authored document (a
   README, an AGENTS file, a SPEC body) gates editorial choices,
   not behavior. Such tests break on legitimate rewrites. If a
   concept must be discoverable in docs, enforce it via review or
   over a stable structural surface (section IDs, frontmatter
   fields), not via substring match.
2. **Copying production constants into the test.** A test that
   hard-codes the same value the production code uses and compares
   them proves only that someone updated both sites in sync — it
   cannot fail in any interesting way. Either derive a property
   of the constant (length, ordering, prefix relation to another
   constant) or delete the test.
3. **File existence or non-emptiness only.** Reading a file
   already gates readability; asserting only that the file is
   non-empty after a successful read is tautological. Assert at
   least one property of the content.
4. **Mocking the function under test and asserting the mock was
   called.** The mock replaces the very behavior the test claims
   to verify. The assertion proves the test plumbing works, not
   the system.
5. **Loose-outcome assertions any input passes.** Assertions so
   permissive that any input satisfies them — checking only that a
   function returned without error when the function is
   infallible, or that an output is non-empty when the function
   always returns non-empty — gate nothing. Pick an assertion that
   would fail for at least one realistic regression.

When a test you wrote is flaky, investigate the flake. Do not
retry it until green; intermittent failures point at real races,
ordering assumptions, or shared state that will bite again later.

### Commit hygiene

- AI-authored commits identify themselves via the `Co-Authored-By`
  trailer in the commit message footer, naming the model and a
  contact address.
- Prefer narrow, well-scoped commits over sprawling ones. One
  logical change per commit makes review, revert, and bisect
  tractable.

### CI gate (suggestion)

`speccy verify` is designed to run as a CI gate. It fails when the
proof shape is broken (missing requirement coverage, malformed
task state, parser-rejected journal elements) and passes when
intact. Wire it into whichever CI service the project uses —
GitHub Actions, GitLab CI, Jenkins, CircleCI, Buildkite, etc. —
so drift surfaces on every push rather than at ship time. The
gate is informational by design: it tells you when the contract
between intent and shipped behavior is visibly broken; it does
not block anyone from making mistakes.
