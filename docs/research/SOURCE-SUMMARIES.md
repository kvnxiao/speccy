# Source Summaries: Foundational Documents

**Informational reference. Not authoritative for product decisions.**

This document is a faithful digest of the three foundational source documents behind this
research. Its purpose is convenience: one file to consult for the key takeaways of all three
sources, instead of re-reading each in full.

- It summarizes; it does not interpret for the product. `DESIGN.md` / `TERMINOLOGY.md` are
  authoritative for any product, workflow, naming, or implementation decision.
- Where you need nuance, quotes, or citations, go to the original documents (they live outside
  this repo; only `PRINCIPLES.md` is committed here). This digest is a map, not the territory.

## Sources at a glance

| Article                                 | Origin                                                       | Date     | One-line thesis                                                                                                                                                |
| --------------------------------------- | ------------------------------------------------------------ | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `The New SDLC With Vibe Coding`         | Addy Osmani, Shubham Saboo, Sokratis Kartakis (Google)       | May 2026 | The craft is shifting from writing code to engineering the context, harness, and verification around models.                                                   |
| `Long-Running Multiagent Orchestration` | Analysis of Factory AI's "missions" pattern (from two talks) | May 2026 | Multi-day agentic engineering is made viable by shared state, fresh-context validation, and validation contracts written before code.                          |
| `PRINCIPLES.md`                         | Speccy project principles                                    | Jun 2026 | A harness-neutral orchestration tool should add determinism and beginning/end human gates, never replace the harness, and never be obsoleted by better models. |

---

## 1. The New SDLC With Vibe Coding

_From ad-hoc prompting to Agentic Engineering. Google, May 2026._

### Core thesis

The interface to the machine is moving from **syntax to intent**. Developers express what to
build; models handle implementation; humans keep architecture, correctness, and judgment.
As of early 2026: 85% of professional developers use AI coding agents, 51% daily, ~41% of new
code is AI-generated.

### The vibe-coding → agentic-engineering spectrum

- Not a binary; a spectrum. The differentiator is **how much structure, verification, and human
  judgment surround the AI output** — not whether AI is used.
- Dimensions that move along the spectrum: intent specification, verification, codebase
  understanding, error handling, appropriate scope, risk profile.
- **Verification is the single biggest differentiator.** Two mechanisms work together:
  - **Tests** verify deterministic parts (input → output).
  - **Evals** verify non-deterministic parts (right trajectory, right tools, quality of final
    response), scored via labelled datasets, rubrics, and LM judges.
  - Without both, it is vibe coding no matter how sophisticated the prompts.
- Right position depends on stakes: a weekend prototype can be pure vibe coding; a financial API
  demands agentic engineering.

### What an AI agent is

An agent perceives a goal, plans, acts through tools, observes, and iterates in its own loop.
Five parts: **model** (reasoning), **tools** (connection to the world), **memory** (state across
sessions), **orchestration** (the code that runs the loop), **deployment** (hosting, identity,
observability, infra).

### Context engineering is the real skill

- Quality of AI output depends more on **context quality** than prompt cleverness.
- Six context types: **instructions, knowledge, memory, examples, tools, guardrails**.
- **Static context** (always loaded: system instructions, AGENTS.md/CLAUDE.md/GEMINI.md, global
  memory, personas) is expensive — every token is present every interaction.
- **Dynamic context** (loaded on demand: skill instructions, tool results, RAG docs, windowed
  history) is efficient — you pay only when needed.
- The static/dynamic boundary is a first-class architectural decision, versioned like config.
- **Agent Skills** are the key pattern: portable packages of procedural knowledge loaded only on
  task match, via progressive disclosure (metadata at startup → full instructions on match →
  deep reference only when needed). They solve context rot, absent procedural memory,
  multi-agent overhead, and portability.

### Harness engineering: Agent = Model + Harness

- The model is one input, not the system. The **harness** is the scaffolding that makes a model
  an agent.
- Harness contents: instructions/rule files, tools, sandboxes/execution environments,
  orchestration logic (sub-agent spawning, model routing, handoffs), guardrails/hooks
  (deterministic code at lifecycle points), observability (logs, traces, evals, cost/latency).
- **Most agent failures are configuration failures**, not model failures — a missing tool, vague
  rule, absent guardrail, or noisy context. Benchmark evidence: teams moved a coding agent from
  outside Top 30 to Top 5 on Terminal Bench 2.0 by changing only the harness; LangChain raised a
  score 13.7 points by tuning only prompt/tools/middleware around a fixed model.
- The harness operates in every SDLC phase: configured during requirements/planning, runs
  implementation (sandboxes/tools), drives the test→act→observe feedback loop (orchestration +
  guardrails), and observes review/deploy/maintenance (hooks + observability).

### How AI transforms each SDLC phase

> Pace-of-change caveat: this phase picture reflects mid-2026 and is shifting fast. Teams are
> already experimenting with going directly from specs to review, with AI handling
> implementation, testing, and deployment in the background. The boundaries may look different in
> 12 months; what stays constant is human judgment, taste, and the skill to verify AI output.

- **Requirements/planning:** AI generates user stories, edge cases, API schemas, prototypes;
  requirements become a human↔AI conversation, not a handoff document.
- **Design/architecture:** Stays the most human-centric phase (trade-offs need business/org
  context). AI implements decisions once made.
- **Implementation:** 25–39% productivity gains reported, but nuanced — a METR study found
  experienced devs took 19% _longer_ on some tasks due to verification/debugging overhead. Work
  shifts from writing to reviewing/guiding/verifying.
- **Testing/QA:** Output evaluation (does it compile/pass?) plus trajectory evaluation (was the
  sequence of steps right?). A fluent output that skipped verification is more dangerous than a
  visible error. Wire into a continuous quality flywheel.
- **Code review/deployment:** AI as first-pass reviewer; humans keep context-dependent judgment.
  AI-aware deployment: health monitoring, auto-rollback, risk prediction.
- **Maintenance/evolution:** Most underestimated. Legacy code "too risky to touch" becomes
  navigable, refactorable, migratable.

### The factory model

The developer's output is **not code — it's the system that produces code**: specifications,
agents, tests/quality gates, feedback loops, guardrails. Give agents **success criteria, not
step-by-step instructions**, then let them iterate.

### Developer roles: conductor vs. orchestrator

- **Conductor:** hands-on, real-time, in the IDE, directing each change. Good for complex logic,
  debugging, unfamiliar code. Risk: becomes a throughput bottleneck.
- **Orchestrator:** async, higher abstraction, defines goals, delegates to background/parallel
  agents, reviews results. Needs skills in **specification, decomposition, evaluation, system
  design**.
- **The 80% problem:** AI rapidly produces ~80% of a feature; the last 20% (edge cases, error
  handling, integration, subtle correctness) needs deep context models lack. AI errors have
  evolved from syntax mistakes to insidious conceptual failures that "look right."

### Where coding agents fit

Three places, often all in one day: **in the editor** (inline completion, chat — Copilot,
Cursor, Windsurf), **in the terminal** (goal-driven, multi-file, runs tools/tests — Claude Code,
Codex CLI, Cline), **in the background** (autonomous cloud sandboxes producing PRs — Jules,
Copilot agent mode, Cursor background agents). Right starting point depends on the task, not an
autonomy ladder.

### Building production agents

When the thing being built is itself an agent, it needs its own tools, memory, evals, and
deployment. Google's Agents CLI illustrates the skill-pack model: harness-neutral, installs
skills into whichever coding agent you use, covers the full ADK build→evaluate→deploy→observe
loop. Cross-agent coordination via shared session state, MCP (tools), and A2A (delegation).
Anchoring data point: Anthropic's team ran an experiment in early 2026 where agent teams built a
working **C compiler in Rust over two weeks**, with humans setting direction and reviewing output
but not writing the implementation — the bottleneck moved from writing code to specifying and
verifying it.

### Economics (TCO, not just velocity)

- **Vibe coding = low CapEx, high OpEx:** near-zero entry, but compounding costs — token burn
  from unstructured context and fix-my-mistake loops, a maintenance tax on spaghetti code, and
  security remediation.
- **Agentic engineering = high CapEx, low OpEx:** upfront investment (API schemas, test suites,
  structured context) drops the marginal cost of shipping and maintaining features.
- **Context engineering is a financial lever:** dense high-signal payloads raise first-pass
  success and avoid trial-and-error token loops.
- **Intelligent model routing:** large models for complex work (requirements, architecture,
  initial implementation); smaller/cheaper models for deterministic work (test generation, code
  review, CI/CD monitoring).

### Practical guidance (where to start)

- **Individual developers:** set up an AGENTS.md (start with ~10 lines: stack, conventions, hard
  rules, workflow; add a rule whenever the agent misbehaves); install skill packs; make one
  repetitive workflow your first agent; write tests and evals before generating code; review
  every line that ships (check imports for real packages, verify error handling); keep your own
  debugging/system-design skills sharp.
- **Engineering leaders:** make context engineering first-class (AGENTS.md, prompts, evals, skill
  libraries reviewed/versioned/owned like code); set the bar at the eval, not the demo, with
  explicit rubrics (task success, tool-use quality, trajectory compliance, hallucination, response
  quality); re-shape code review for AI failure modes (hallucinated dependencies, weak error
  handling, subtle correctness gaps); make the prototype-vs-production boundary explicit; treat
  harness components as shared, maintained infrastructure.
- **Organizations:** treat AI-assisted development as an engineering investment, not a
  productivity feature; build the production substrate before scale (CI-run trajectory/final
  evals, run traces, scoped per-agent permissions, security review tuned to generated-code
  failure modes); adopt open standards (MCP, A2A); plan for hybrid human+agent teams with clear
  handoff protocols; hire and develop for judgment over implementation.

### Three durable principles (conclusion)

1. **Structure scales, vibes don't.** Discipline is not optional for software organizations
   depend on.
2. **AI amplifies your engineering culture** — both strengths and weaknesses.
3. **The human role is evolving, not diminishing.** Skills shift from implementation to judgment.
   "Generation is solved. Verification, judgment, and direction are the new craft."

---

## 2. Long-running multi-agent orchestration via Missions

_Analysis of Factory AI's "missions" pattern, drawn from two talks (CTO Eno Reyes; harness lead
Luke Alvoeiro). May 2026. Author is unaffiliated with Factory AI._

### Anchoring data points

- Longest single production mission described: **16 consecutive days**.
- Architecture reportedly viable to **~30 days** before validation overhead and context drift
  make returns negative.

### Human attention is the bottleneck

- Frontier models can plan 50 backlog items; human teams ship a few per day because every task
  needs a human to scope, review, unblock. The most valuable thing an agentic system can do is
  **absorb supervision load**, not be smarter.
- This separates **agent quality from agent throughput.** A system with slightly worse code per
  token but supervisable in batches at milestone boundaries wins on throughput. Missions is
  engineered for that case.

### Five orchestration strategies (taxonomy)

- **Delegation:** an agent spawns another for a sub-task (basis of sub-agents).
- **Creator-verifier:** separate the builder from the checker; a fresh context finds issues the
  cost-biased author won't.
- **Direct communication:** agents DM each other with no central coordinator — fragments state,
  hard to keep coherent.
- **Negotiation:** coordinating over shared resources, ideally positive-sum.
- **Broadcast:** status/constraints flowing one-to-many; unglamorous but essential.

Missions composes **four** (delegation, creator-verifier, broadcast, negotiation) and
**deliberately omits direct communication.** Every agent reads/writes a **single authoritative
shared mission state** instead of messaging peers. This lets a crashed/swapped worker resume by
reading state, and forces contradictions to surface at write-time instead of compounding across
pairwise channels.

### Three-role architecture

- **Orchestrator:** planning. Interrogates the operator on scope, users, stack, integrations;
  produces a plan of features, milestones, and a validation contract.
- **Worker:** implementation. Each gets a **clean context window**, reads its spec, implements,
  commits via git, hands off.
- **Validators:** verification, in two flavors:
  - **Scrutiny validator:** lint, type check, tests, and spawns **parallel code-review
    sub-agents** per feature (review is never done by the author).
  - **User-testing validator:** behaves like QA — launches the app, drives it via browser/computer
    use, fills forms, clicks, verifies end-to-end flows. **Most wall-clock time lives here** (waiting
    on real apps to render).
- Neither validator has seen the implementation code first, so validation is **adversarial** —
  the cheapest correction for author bias.

### Validation contracts must be load-bearing

- The **single most important construct.** A structured list of assertions defining "done,"
  written by the orchestrator during planning, **before any code exists** (can be hundreds of
  assertions).
- Every feature maps to one or more assertions; the sum of features must cover every assertion.
- **Why before code:** tests written after implementation confirm the implementer's decisions
  rather than catch bugs; the agent is cost-biased toward code it just wrote.
- **Each assertion declares how it will be proved.** E.g., "application loads successfully" →
  run dev server, check console for errors, check network tab, screenshot. This makes validation
  mechanically executable, not a vague judgment call.
- **Reward-signal framing:** coding agents are trained against correctness-vs-spec reward
  signals. Formulating a task as an explicit runtime reward signal meets the model on the surface
  it was trained to optimize. Externalizing the success criterion outperforms leaving it
  implicit, and the gap widens with task horizon.

### Serial execution with targeted parallelism

- Naive fan-out (10 workers, 10 features) does not survive contact with software work: parallel
  writers step on each other, duplicate work, make inconsistent decisions; coordination overhead
  eats the gains.
- Missions runs features **serially** — one worker or validator active at a time.
- **Read-heavy sub-tasks parallelize:** codebase search, doc lookup, API research (within a
  feature); code review across features (within a validator). Described as "controlled chaos."
- Corroborated by Google and Augment Code research: decide when to parallelize from the
  read-vs-write ratio and task complexity. Read-heavy work parallelizes; cross-cutting complex
  work does not.

### Skills as the pillar of evolution

- Skills are markdown docs encoding reference material or workflows (like AGENTS.md), but
  missions leans on them harder because the workflow needs a stable surface to refine.
- The orchestrator writes per-role skills at planning time. When a worker hits friction (e.g.,
  "back end needs Docker running first"), it **edits its own skill** to capture the lesson;
  future workers and future missions on the same codebase inherit the fix.
- This is **continuous learning at the project level, written into the codebase, not model
  weights.**
- Fragile in the opposite direction: without the skill-rewriting loop, a 40-task run would
  re-learn the same friction every task and degrade rapidly past ~10 tasks. The mechanism is
  what makes the multi-day horizon viable.

### Structured handoffs and self-healing

- A finishing worker fills a structured handoff: what was completed, what was left undone, what
  commands ran, their exit codes, issues discovered, and whether it followed orchestrator
  procedures.
- Errors caught at milestone boundaries: scope corrective work as a follow-up feature, rescope a
  milestone, or roll back. A workflow that cannot articulate why it succeeded/failed cannot be
  debugged — and an undebuggable workflow cannot run for 16 days.

### Picking the right model per role

- Planning wants slow careful reasoning; implementation wants fast code fluency; validation wants
  precise instruction-following. No single family is best at all three.
- Recommendation: put a **different provider in the validation seat than the implementation
  seat** so the same training data doesn't bias both ends.
- Structure can compensate for weaker models: contracts and checkpoints produce working code even
  with open-weight workers.

### Defending against evolving models

- Put almost all orchestration logic in **prompts and skills** (~700 lines of natural language:
  feature decomposition, failure handling, negotiation), keep **deterministic code thin**
  (bookkeeping: running validation, gating on unresolved handoffs, persisting state).
- General rule: parts that benefit from improving models → prompts; parts that benefit from
  determinism → code. Mixing the layers produces systems that get _worse_ as models improve.

### Brownfield first, greenfield second

- Validation primitives were built against **brownfield** workloads first (legacy modernization,
  migration on real production codebases), forcing validation harnesses that survive messy state.
  Greenfield then fell out for free.
- **Diagnostic for whether to use this workflow at all:** if you can articulate clear validation
  criteria, it will probably succeed. If you cannot validate (no test harness, no observable
  behavior, no inspectable output), it burns hours/tokens and produces something ~85% correct —
  and cleaning up the last 15% costs more than doing it by hand.

### Where the boundary lies

- On an internal benchmark cloning popular tools, small-feature projects ship cleanly;
  100+-feature projects fail — **not because of model coding ability, but because the validation
  harness is the bottleneck.**
- Examples: a Zapier-style system needs Firecracker VMs to validate real workflows; a
  bot-protected OAuth flow can't be driven by a browser agent; multi-region deploys need real
  cloud infra. The frontier is the **localhost↔production gap**, not reasoning.

### Takeaways (as stated)

- Externalize the success criterion; each assertion declares how it's proved.
- Separate the writer from the verifier (fresh context).
- Prefer serial execution with read-heavy parallel sub-tasks over parallel write-heavy tasks.
- Encode workflow logic in prompts/skills; reserve deterministic code for state and gating.
- Make the skills layer continuously editable by workers.
- Treat structured handoffs as a first-class artifact.
- Accept that **validation, not generation, is the binding constraint.**

### Open questions

- How far can serial execution be parallelized without losing coherence?
- How should missions be composed into higher-order workflows?
- What validation harness closes the localhost↔production gap for arbitrary apps?
- What governance is appropriate for software built primarily by agents under a human constitution?

---

## 3. Spec Driven Orchestration Principles

_Speccy project principles. Jun 2026._

### Core thesis

An agentic orchestration tool must integrate with **any** agent harness and provide autonomy via
**concrete human-intervention checkpoints**, removing the need to steer an agent in real time.
Maximize determinism, minimize human context overload, and rely on ever-improving models to
implement the plan. **A correctly designed orchestration tool should never be obsoleted by a
future frontier model.**

### The workflow shift

- **Before:** iterate on a plan/prompt, start implementation in auto-mode, then babysit the
  model's output and steer directional corrections.
- **After:** invest more brainpower up front so the plan is as comprehensive as possible, then
  step back and let the model implement autonomously. Human gates are at the **beginning** (review
  the plan) and the **end** (review the implementation) — no babysitting.

### Core principles

- **Just another tool available:** provides one more way to build software inside a harness; does
  not replace existing approaches (e.g., regular `/plan` and implement) and should not be used for
  everything.
- **Less is more:** avoid over-engineering; simple, reusable solutions (YAGNI). More code (and
  more comments) means contextual overload for the human reviewer at review time.
- **Make drift visible for self-improvement:** use sequential self-review and self-improvement via
  **fresh-context agents** on long-horizon tasks for higher-quality output.
- **Zero product-code / build-time footprint:** must not affect product source, the build graph,
  deployed artifacts, runtime dependencies, or production behavior. Repo-local harness packs
  (`.codex`, `.claude`, `.agents`, `.speccy` policy/prose) are acceptable workflow artifacts when a
  team wants shared, versioned, editable lifecycle instructions. Operational run state, transcripts,
  raw evidence, screenshots, caches, and databases stay external or ignored by default.
- **No outbound agent runner:** Speccy commands never call LLMs, coding agents, or harnesses. The
  active harness calls Speccy's deterministic controller tools; Speccy does not launch the harness.

### Two-layer design

Split the system into:

1. A **deterministic core layer.**
2. A higher-level **prose layer** (harness skills and subagents).

Goal: use modern models and agentic engineering to make the core offering as deterministic as
possible, reducing the surface area for non-deterministic, drifted, or hallucinated model output.

---

## Cross-cutting threads

Themes all three sources share (useful when reasoning about the design):

- **Verification/validation is the new binding constraint**, not generation. (All three.)
- **Externalize the success criterion before implementation** — specs, tests, evals, or a
  validation contract. (SDLC "tests/evals as the contract"; Missions "contract before code";
  Principles "comprehensive plan up front.")
- **Fresh-context verifiers beat author self-review.** (Missions creator-verifier; Principles
  "make drift visible via fresh-context agents.")
- **Determinism where it helps, prose/models where they help.** (Missions prompts-vs-code split;
  Principles two-layer design; SDLC harness = deterministic hooks + model reasoning.)
- **Human attention moves to the edges** — set direction and review outcomes, don't babysit.
  (Missions "human attention is the bottleneck"; SDLC conductor→orchestrator; Principles
  beginning/end gates.)
- **Skills / harness packs are the portable unit of procedural knowledge.** (SDLC Agent Skills;
  Missions self-editing skills; Principles repo-local harness packs.)
