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

Consequently, the human-facing CLI surface is small:

| Command         | When you type it                          |
| --------------- | ----------------------------------------- |
| `speccy init`   | Once per repo, to bootstrap.              |
| `speccy status` | Occasionally, to inspect workspace state. |
| `speccy check`  | Occasionally, to run proofs locally.      |
| `speccy verify` | In CI, as the gate.                       |

Everything else (`plan`, `tasks`, `implement`, `review`, `report`,
and `next`) is invoked by the shipped skills.

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
- Scaffolds `.speccy/speccy.toml` and the `.speccy/specs/` skeleton.
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

- **Greenfield (`AGENTS.md` missing entirely).** The skill walks
  through a short Q&A (what, who for, v1 outcome, quality bar, known
  unknowns, and non-goals) and writes a fresh `AGENTS.md` whose
  first section is `## Product north star`.
- **Brownfield without a north star (`AGENTS.md` exists, but no
  `## Product north star` section).** The skill runs a narrower Q&A
  and **appends** a `## Product north star` section without touching
  what is already there.
- **Brownfield with a north star already present.** The skill leaves
  `AGENTS.md` alone, and you simply confirm that the existing
  content is current before moving on.

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

See [`.speccy/ARCHITECTURE.md`](./.speccy/ARCHITECTURE.md) for the
canonical contract on what each phase produces, and on how the state
machine transitions between them.

---

## Repo layout after `speccy init`

```text
AGENTS.md                       Product north star + conventions (root)
CLAUDE.md                       Symlink to AGENTS.md (Claude Code reads this)

.speccy/
  speccy.toml                   Minimal project config (just schema_version + name)
  specs/
    NNNN-slug/                  One spec, flat layout
      SPEC.md                   Frontmatter + PRD prose + Decisions + Changelog
      TASKS.md                  Checklist + inline implementer/reviewer notes
      spec.toml                 Requirement <-> Check mapping
      REPORT.md                 Written at end of loop

.claude/                        (if host is Claude Code)
  skills/speccy-*/              Workflow recipes (init, plan, tasks, work, review, ship, amend)
  agents/reviewer-*.md          Reviewer persona sub-agents

.agents/                        (if host is Codex)
  skills/speccy-*/
.codex/
  agents/reviewer-*.toml
```

Specs may additionally be grouped under an optional **mission
folder** (`.speccy/specs/[focus]/MISSION.md` plus one folder per
spec inside the focus). Grouping is opt-in, however. Solo projects
with a single focus area typically stay flat, and mission folders
earn their existence only when two or more related specs share
enough context that loading them together at plan time becomes
cheaper than rediscovering the context on every run.

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
   Check), ten commands, and no mode toggles.
6. **Surface unknowns; never invent.** An ambiguous spec means stop
   and surface the ambiguity; an inability to validate something
   means say so out loud.

The full architecture (schema, lint codes, JSON contracts, and the
deliberate list of "what we do not do") lives in
[`.speccy/ARCHITECTURE.md`](./.speccy/ARCHITECTURE.md).

---

## License

MIT. See [`LICENSE`](./LICENSE).
