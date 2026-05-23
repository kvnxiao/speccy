# Speccy

A deterministic feedback engine for spec-driven development with AI agents.

When humans and AI agents collaborate on software over time, intent
and shipped behaviour tend to drift apart. Speccy makes the contract
between them **visible**, so the drift becomes loud the moment it
happens.

Speccy is a feedback engine, not an enforcement system. Shipped
skills wrap its CLI into an end-to-end loop that your agent harness
can drive on your behalf.

> **Status:** v0.1.0, work in progress. Speccy is dogfooded by Speccy
> itself; its own implementation is tracked under `.speccy/specs/`.

---

## How it works

After `speccy init`, day-to-day usage happens **inside your agent
harness**, not in the shell. You type slash-commands (e.g.
`/speccy-plan`), and the shipped skills invoke the underlying CLI on
your behalf. The CLI is intentionally thin: it renders prompts
deterministically, queries workspace state, and runs checks. The
intelligence (loops, personas, and "what to do next" decisions)
lives in skill files alongside `.speccy/`.

Consequently, the human-facing CLI surface is small. Seven flat
commands, each with one job; `--json` toggles representation, not
content:

| Command          | When you type it                                                          |
| ---------------- | ------------------------------------------------------------------------- |
| `speccy init`    | Once per repo, to bootstrap.                                              |
| `speccy status`  | Occasionally, to inspect workspace state.                                 |
| `speccy check`   | Occasionally, to render the Given/When/Then scenarios for one or more specs. |
| `speccy verify`  | In CI, as the gate.                                                       |
| `speccy next`    | Mostly invoked by the shipped skills to resolve the next actionable task. |
| `speccy lock`    | Invoked by `/speccy-tasks` to record the SPEC.md hash into TASKS.md after decomposition. |
| `speccy vacancy` | Invoked by `/speccy-plan` to allocate the next free `SPEC-NNNN`. |

Phase prompts (`plan`, `tasks`, `implement`, `review`, `report`) are
not CLI commands. They live as skill bodies in the host pack and
drive the loop entirely through the seven CLI verbs above. The
deterministic core does state queries, hash recording, and proof-
shape lint only; it never renders natural-text prompts.

All of this lives inside your repo. The `.speccy/` workspace sits at
the repo root, and the host skill pack is copied into your local
`.claude/`, `.agents/`, or `.codex/` folder; nothing is written to a
global `~/.claude/` or `~/.codex/` skills location. While the shipped
skills and reviewer personas are generic enough to serve as a
reasonable starting point, you will likely get better results by
tuning them to the conventions, vocabulary, and tooling of your own
repo. Committing those edits alongside the code means every
contributor on the same harness inherits the same tuning, so agent
output stays consistent across the team rather than drifting per
developer.

The same locality makes uninstalling trivial. Delete `.speccy/` and
the host skill files placed under `.claude/` (for Claude Code) or
`.agents/` and `.codex/` (for Codex), and you are back where you
started.

---

## Install

Speccy is not yet published to crates.io, so installation is from
source:

```bash
# from a local clone
git clone https://github.com/kvnxiao/speccy
cd speccy
cargo install --path speccy-cli --locked

# or directly
cargo install --git https://github.com/kvnxiao/speccy speccy --locked
```

Subsequently, confirm the binary is on `PATH`:

```bash
speccy --version
```

The shipped skill packs target two agent harnesses: **Claude Code**
and **Codex**.

---

## Onboarding

Speccy works identically whether the project is greenfield (no code
yet) or brownfield (existing code, lockfiles, and conventions).
**There is no mode flag, and no separate brownfield workflow**; the
CLI does the same thing in both cases. The only thing that shifts
between them is how much of the **product north star** already lives
in your project's root `AGENTS.md`.

### Step 1: Scaffold the workspace

From the repo root:

```bash
speccy init
```

This command:

- Refuses to run if `.speccy/` already exists. Pass `--force` to
  refresh the shipped files in place after a `speccy` upgrade.
- Detects the host harness from `.claude/` or `.codex/` on disk.
  Pass `--host claude-code` or `--host codex` to override detection.
- Scaffolds the `.speccy/` directory and the `.speccy/specs/` skeleton.
- Copies the host skill pack into the host-native location:
  - Claude Code: `.claude/skills/speccy-*/` and `.claude/agents/`
  - Codex: `.agents/skills/speccy-*/` and `.codex/agents/`
- Prints every file it will create or overwrite **before** writing
  anything.

It should be noted that `speccy init` never edits `AGENTS.md`.
Seeding the product north star is the next step's responsibility,
and it belongs to a skill rather than the CLI.

> Alternatively, if you would rather drive the whole bootstrap from
> the agent harness, invoke the `speccy-init` skill (e.g.
> `/speccy-init` in Claude Code). It calls `speccy init` and
> subsequently walks the `AGENTS.md` step interactively.

### Step 2: Make sure `AGENTS.md` carries the product north star

Speccy loads the repo-root `AGENTS.md` into every rendered prompt,
and that file carries two things side by side:

- A **product north star**: what you are building, who for, what
  "good enough to ship v1" looks like, and what is explicitly out
  of scope. This is loaded into every planner, implementer, and
  reviewer prompt.
- **Project conventions**: hygiene rules, agent behavioural
  expectations, and references to language and tooling rule files.

The `speccy-init` skill handles three cases without ever overwriting
existing content:

- **State A — `AGENTS.md` missing entirely.** The skill walks
  through a short Q&A (what, who for, v1 outcome, quality bar, known
  unknowns, and non-goals) and writes a fresh `AGENTS.md` whose
  first section is `## Product north star`.
- **State B — `AGENTS.md` exists, but no `## Product north star`
  section.** The skill runs a narrower Q&A and **appends** a
  `## Product north star` section without touching what is already
  there.
- **State C — `AGENTS.md` already has a product north star.** The
  skill leaves `AGENTS.md` alone, and you simply confirm that the
  existing content is current before moving on.

If you are not on a host the `speccy-init` skill ships for, you
populate `AGENTS.md` by hand; this repo's own `AGENTS.md` is a
working example.

### Step 3: Drive specs end-to-end from your agent harness

From this point onward, the workflow is entirely slash-commands. The
golden path consists of five recipes:

```text
/speccy-plan      Phase 1: draft SPEC.md + spec.toml from the north star
/speccy-tasks     Phase 2: decompose the SPEC into TASKS.md
/speccy-work      Phase 3: implementer sub-agent loop, task by task
/speccy-review    Phase 4: adversarial multi-persona review loop
/speccy-ship      Phase 5: write REPORT.md, open the PR
```

If you would rather not chain the per-task recipes by hand, the
shipped orchestrator drives the full loop end-to-end:

```text
/speccy-orchestrate   Chain /speccy-work and /speccy-review across every
                      task in one SPEC, then hand off to /speccy-holistic-gate
                      for the pre-ship drift check before stopping at the
                      ship boundary.
/speccy-holistic-gate Pre-ship SPEC-vs-implementation drift review with an
                      autonomous fix-retry loop; invoked by the orchestrator
                      and also runnable on its own.
```

In addition, for mid-loop scope changes, there is one more recipe:

```text
/speccy-amend     Surgically edit SPEC.md and reconcile TASKS.md
```

Each skill knows which `speccy` CLI commands to invoke, when, and in
what order, so you are not expected to chain them manually.
Naturally, if a skill is wrong or missing a step, you should fix the
skill file directly under `.claude/skills/` or `.agents/skills/`;
thereafter, the next contributor inherits the fix rather than
rediscovering the friction themselves.

See [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for the
canonical contract on what each phase produces, and on how the state
machine transitions between them.

---

## Repo layout after `speccy init`

```text
AGENTS.md                       Product north star + conventions (root)
CLAUDE.md                       Symlink to AGENTS.md (Claude Code reads this)

.speccy/
  specs/
    NNNN-slug/                  One spec, flat layout
      SPEC.md                   Frontmatter + PRD prose + nested <requirement>/<scenario>/<decision> elements + Changelog
      TASKS.md                  Frontmatter (spec_hash_at_generation) + <task> elements + inline implementer/reviewer notes
      REPORT.md                 Frontmatter (outcome) + <report>/<coverage> elements (end of loop)

.claude/                        (if host is Claude Code)
  skills/speccy-{init,brainstorm,plan,tasks,work,review,amend,ship,orchestrate,holistic-gate}/
                                Ten workflow recipes. The interactive skills
                                (init, brainstorm, plan, review, amend, orchestrate,
                                holistic-gate) eject as full-body SKILL.md; the three
                                pinned phase workers (tasks, work, ship) eject as thin
                                SKILL.md stubs pointing at the matching agent file.
  agents/speccy-{tasks,work,ship}.md   Pinned phase-worker sub-agents (full body)
  agents/reviewer-*.md                 Six reviewer persona sub-agents
  agents/holistic-{reviewer,implementer}.md
                                Two holistic-loop sub-agents driven by
                                /speccy-holistic-gate (drift review + drift fix)

.agents/                        (if host is Codex)
  skills/speccy-*/                     Ten Codex skill SKILL.md files
.codex/
  agents/speccy-{tasks,work,ship}.toml Pinned phase-worker sub-agents
  agents/reviewer-*.toml               Six reviewer persona sub-agents
  agents/holistic-{reviewer,implementer}.toml
                                Codex twins of the two holistic-loop sub-agents
```

The requirement-to-scenario graph lives in-band as XML element tags
inside `SPEC.md`; there is no per-spec `spec.toml`.

Specs may additionally be grouped under an optional **mission
folder** (`.speccy/specs/[focus]/MISSION.md` plus one folder per
spec inside the focus). Grouping is opt-in, however. Solo projects
with a single focus area typically stay flat, and mission folders
earn their existence only when two or more related specs share
enough context that loading them together at plan time becomes
cheaper than rediscovering the context on every run.

---

## Model pinning

Speccy's shipped skill packs pin specific model and effort tiers for
each phase of the development loop. The pin assignment is asymmetric
and reflects the work-shape of each role: mechanical phases pin a
mid-tier model so they run cheaply; adversarial reviewers pin a
higher tier so they catch real drift. The orchestrator phases
(`/speccy-init` and `/speccy-review`) stay unpinned and inherit
whatever model the parent session is using.

### Pin assignment

| Phase / persona         | Claude Code (`.claude/agents/...md`)    | Codex (`.codex/agents/...toml`)              | Agent file ships? |
| ----------------------- | --------------------------------------- | -------------------------------------------- | ----------------- |
| `speccy-tasks`          | `model: sonnet[1m]`, `effort: medium`   | `model = "gpt-5.5"`, reasoning effort medium | yes               |
| `speccy-work`           | `model: opus[1m]`, `effort: low`        | `model = "gpt-5.5"`, reasoning effort medium | yes               |
| `speccy-ship`           | `model: sonnet[1m]`, `effort: medium`   | `model = "gpt-5.5"`, reasoning effort medium | yes               |
| `speccy-init`           | unpinned, inherits session              | unpinned, inherits session                   | no                |
| `speccy-review`         | unpinned, inherits session              | unpinned, inherits session                   | no                |
| `reviewer-business`     | `model: opus[1m]`, `effort: xhigh`      | `model = "gpt-5.5"`, reasoning effort high   | yes               |
| `reviewer-tests`        | `model: opus[1m]`, `effort: xhigh`      | `model = "gpt-5.5"`, reasoning effort high   | yes               |
| `reviewer-architecture` | `model: opus[1m]`, `effort: xhigh`      | `model = "gpt-5.5"`, reasoning effort high   | yes               |
| `reviewer-security`     | `model: sonnet[1m]`, `effort: high`     | `model = "gpt-5.5"`, reasoning effort medium | yes               |
| `reviewer-style`        | `model: sonnet[1m]`, `effort: medium`   | `model = "gpt-5.5"`, reasoning effort low    | yes               |
| `reviewer-docs`         | `model: sonnet[1m]`, `effort: medium`   | `model = "gpt-5.5"`, reasoning effort low    | yes               |

The `[1m]` suffix selects the 1M-context variant on Claude Code so
each agent can read the full SPEC, the full diff, and the relevant
TASKS.md slice without truncation. On Codex, `gpt-5.5` covers every
pinned role and the asymmetric work-shape is carried entirely by
`model_reasoning_effort`.

The three pinned phase workers (`speccy-tasks`, `speccy-work`,
`speccy-ship`) ship sub-agent files. `speccy-init` and
`speccy-review` ship no agent file on either host — only the
SKILL.md surface — because both phases need to drive the parent
session directly (interactive Q&A in init's case; serial TASKS.md
writes in review's case).

### Activating a pin (opt-in)

The pin lives in the agent file, not the slash-command surface.
Typing `/speccy-work` runs the workflow in the **parent session at
the parent session's model**; the agent file is ignored. To
activate the pin, invoke the sub-agent explicitly before running
the phase:

- **Claude Code:** `/agent speccy-work` (or use the host's sub-agent
  spawning tool), then run `/speccy-work`.
- **Codex:** invoke the equivalent sub-agent spawner against
  `.codex/agents/speccy-work.toml`, then run `/speccy-work`.

For the three pinned phases, the SKILL.md body is a thin stub
that points at the matching agent file as the canonical procedure
source. The agent file's body is the single on-disk source of
truth for that phase. `/speccy-init`'s SKILL.md and
`/speccy-review`'s SKILL.md both remain full-body because there is
no sub-agent file for either to defer to.

The `/speccy-review` orchestrator stays unpinned on both hosts
deliberately: it is the sole writer to `TASKS.md` during the review
loop (reviewer sub-agents return their verdicts and the
orchestrator serializes the state transition), and it needs the
parent session's full capacity to fan out, parse return messages,
and consolidate verdicts without dropping state. Pinning it to a
sub-agent would either force a serial-write race or strand the
verdict-consolidation logic in a context that does not own
`TASKS.md`.

> **Design lesson.** An earlier draft of this work auto-forked the
> mechanical phases into pinned sub-agents via Claude Code's
> `context: fork` mechanism. Auto-forking hides the sub-agent's
> tool output from the parent session by design, which on
> multi-minute phase work produces minutes of dead air in the
> parent UI with no progress signal. The opt-in `/agent` surface
> preserves the cost-and-time pin without the silent-by-design UX
> cost.

### Overriding a pin

The shipped pins are defaults, not policy. To swap models or
remove a pin entirely, edit the agent file's YAML or TOML
frontmatter under `.claude/agents/` or `.codex/agents/` and commit
the change. Examples:

- Lock `speccy-work` to a specific Claude version for
  reproducibility: change `model: opus[1m]` to
  `model: claude-opus-4-7[1m]` in
  `.claude/agents/speccy-work.md`.
- Run a reviewer at a lighter tier in a cost-sensitive repo:
  change `model: opus[1m]` to `model: sonnet[1m]` (and adjust
  `effort:` accordingly) in
  `.claude/agents/reviewer-business.md`.
- Remove the pin entirely so the sub-agent inherits the parent
  session's model: delete the `model:` and `effort:` lines.

Pins use **aliases** (`sonnet[1m]`, `opus[1m]`, `gpt-5.5`) rather
than long-form versioned snapshot IDs by default so they float
forward as vendors ship newer generations of each tier. Users who
want byte-stable reproducibility across a release boundary can
lock to a specific version by editing the alias to a long-form ID
in the ejected file.

---

## CI integration

Add `speccy verify` to your pipeline as a gate:

```yaml
- name: speccy verify
  run: speccy verify
```

It exits non-zero when the proof shape is broken (missing required
frontmatter, requirements without covering checks, spec or task hash
drift, no-op check commands, and so on), and zero otherwise. Pass
`--json` for a schema-versioned envelope that downstream tooling can
parse.

`speccy verify` is the **only** command that exits non-zero on
findings. Everything else surfaces problems and exits zero, so that
drift stays loud while the CLI itself never blocks you mid-loop.

---

## Design

Speccy commits to six durable principles:

1. **Feedback, not enforcement.** Speccy makes drift visible; it
   does not block agents from making mistakes.
2. **Deterministic core, intelligent edges.** The Rust CLI is
   mechanical: it renders prompts, queries state, and runs checks.
   It does not call LLMs.
3. **Proof shape, not proof scores.** Every Requirement maps to at
   least one Check, and every Check declares what it proves. One
   structural anti-pattern is flagged (no-op commands as sole
   proof); the rest is review's job.
4. **Review owns semantic judgment.** Multi-persona adversarial
   review (business, tests, security, and style by default) is the
   mechanism by which drift gets caught. Personas live as markdown
   skills.
5. **Stay small.** Five nouns (Mission, Spec, Requirement, Task, and
   Check), seven commands, and no mode toggles. `--json` toggles
   representation, never content.
6. **Surface unknowns; never invent.** An ambiguous spec means stop
   and surface the ambiguity; an inability to validate something
   means say so out loud.

The full architecture (schema, lint codes, JSON contracts, and the
deliberate list of "what we do not do") lives in
[`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md).

---

## License

MIT. See [`LICENSE`](./LICENSE).
