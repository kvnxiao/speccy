# Source Summaries

**Informational reference. Not authoritative for product decisions.**

Digest of the external sources behind the Speccy design, checked against primary text where one is reachable. `DESIGN.md` and `PRINCIPLES.md` decide product, workflow, naming, and implementation questions; this file records what the sources say and how well each claim is sourced.

Verification labels, as of 2026-08-30:

- **Primary**: read from the original text (PDF, article, docs page, arXiv abstract, repo README).
- **Talk-digest**: reconstructed from independent written digests of a recorded talk that agree with each other; the talk itself was not transcribed.
- **Secondary**: the primary refused the fetch; the claim rests on independent summaries.

Claims from the July 2026 digest that no reachable source supports are listed under "Removed claims" at the end.

## Sources at a glance

| Source                                               | Origin                                           | Date                | Verification          | One-line thesis                                                                                                                                   |
| ---------------------------------------------------- | ------------------------------------------------ | ------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| The New SDLC With Vibe Coding                        | Osmani, Saboo, Kartakis (Google)                 | May 2026            | Primary (PDF)         | The craft moves from writing code to engineering the context, harness, and verification around models.                                            |
| Factory Missions                                     | factory.ai architecture post; Luke Alvoeiro talk | Apr 2026            | Primary + talk-digest | Multi-day agent runs need one shared state, fresh-context validators, and a validation contract written before code.                               |
| `PRINCIPLES.md`                                      | Speccy                                           | Jun 2026            | Primary (in repo)     | Harness-neutral, deterministic bookkeeping, human gates at the edges; the promise is less supervision and an inspectable record, not correctness. |
| Anthropic long-running harnesses                     | Anthropic engineering blog                       | Nov 2025 – Apr 2026 | Primary               | State lives outside the context window; a separate evaluator with a pre-agreed done-condition beats self-grading.                                 |
| OpenAI harness engineering; Symphony; Codex platform and goals | OpenAI                                 | Feb – Aug 2026      | Secondary; primary    | Humans write the map, linters, and constraints; agents write the code. The tracker is the control plane; the harness is open and scriptable.        |
| Harness engineering as a discipline                  | Hashimoto; Ronacher; Thoughtworks; LangChain; Willison; arXiv | 2026     | Primary               | Every agent mistake becomes a permanent fix; guides vs sensors; harness variance exceeds model variance; harnesses now improve themselves.          |
| Issue-tracker hierarchies                            | Jira, Linear, GitHub docs                        | 2026                | Primary               | One dispatch unit with a checklist beneath and an outcome container above; time boxes stay orthogonal; "milestone" means stage or date.             |
| Running a Software Factory Efficiently at Uber Scale | Uday Kiran Medisetty (Uber)                      | Aug 27, 2026        | Primary               | Cost is decomposed into independent levers; CLI-resolved tools and cheaper per-role models beat schema-heavy MCP and frontier-everywhere.          |
| Harness surfaces Speccy targets                      | Agent Skills spec; Claude Code docs; Codex docs  | Aug 2026            | Primary               | Both target harnesses ship subagents, teams, scripted workflows, persisted goals, and a shared skill format.                                       |
| Spec-driven development tools                        | Spec Kit, OpenSpec, Kiro, GSD Core, others       | Aug 2026            | Primary + secondary   | Every tool keeps repo-resident markdown specs and phases; none keeps a requirement-linked evidence ledger.                                         |
| Context Engineering: Sessions & Memory               | Milam, Gulli (Google)                            | Nov 2025            | Secondary             | A session is an immutable event log plus mutable state; memory is extracted facts; compaction trades information loss against cost.               |
| Standards and distribution                           | Linux Foundation AAIF; Claude Code and Codex plugin docs | Dec 2025 – Aug 2026 | Primary           | `AGENTS.md` is a foundation-governed standard; both harnesses ship skills, agents, and hooks as versioned plugins from git-hosted marketplaces.      |

---

## 1. The New SDLC With Vibe Coding

Google whitepaper, 51 pages, May 2026. Verified against the PDF (Kaggle: `kaggle.com/whitepaper-the-new-SDLC-with-vibe-coding`). Every claim carried over from the July digest is present in the paper. The companion Day-3 paper on context engineering is digested in section 10.

### Spectrum, not binary

- Vibe coding and agentic engineering are ends of one spectrum. The differentiator is how much structure, verification, and human judgment surround the output, not whether AI is used. Dimensions: intent specification, verification, codebase understanding, error handling, scope, risk.
- **Verification is the single biggest differentiator.** Tests verify deterministic parts; evals verify non-deterministic parts (trajectory, tool choice, output quality) via labelled datasets, rubrics, and LM judges. The paper places workflows without both on the vibe-coding side of the spectrum, regardless of prompt sophistication.
- Adoption figures quoted by the paper: 85% of professional developers use coding agents regularly, 51% daily, "an estimated" 41% of new code is AI-generated. Productivity gains of 25–39% come from industry surveys. A METR randomized trial found experienced developers 19% slower on some tasks once checking and fixing time is counted; METR announced a redesign of that experiment in February 2026 (`metr.org/blog/2026-02-24-uplift-update`).

### Context engineering

- Six context types: **instructions, knowledge, memory, examples, tools, guardrails**.
- **Static context** (loaded every turn: system instructions, rule files such as `AGENTS.md`/`CLAUDE.md`, global memory) is expensive because every token is present in every interaction. **Dynamic context** (skill instructions on task match, tool results, retrieved docs, windowed history) is paid for only when a task touches it. The static/dynamic boundary is an engineering trade-off to version like config.
- **Agent Skills** are the paper's pattern for dynamic context: the agent sees lightweight metadata at startup, loads the full instructions on task match, and pulls heavy reference material only when needed (progressive disclosure).

### Agent = model + harness

- Everything except the model is the harness: instructions and rule files, tools and MCP servers, sandboxes, orchestration logic (sub-agent spawning, model routing, handoffs), guardrails and hooks, observability.
- **Most agent failures are configuration failures**: a missing tool, a vague rule, an absent guardrail, a noisy context window. Cited evidence: one team moved a coding agent from outside the Top 30 to the Top 5 on Terminal Bench 2.0 by changing only the harness; LangChain raised a score 13.7 points (52.8% to 66.5%) by changing only system prompt, tools, and middleware (primary in section 6).
- The paper names a taxonomy of **ambient, workflow, and autonomous agents** alongside the spectrum and the factory model as its durable mental models.

### The factory model

The developer's primary output is the system that produces code: specifications and context, agents that implement them, tests and quality gates, feedback loops that route failures back to agents, and guardrails. "Success comes from giving agents success criteria rather than step-by-step instructions, then letting them iterate."

### SDLC phases and the pace caveat

- Requirements become a human–AI conversation producing a spec and a prototype. Architecture stays the most human phase. Implementation compresses from weeks to hours. Testing adds trajectory evaluation to output evaluation: a fluent output that skipped verification is more dangerous than a visible error. Review is AI first-pass with humans keeping context-dependent judgment. Maintenance is the most underestimated gain.
- Stated caveat: the phase picture reflects mid-2026; teams are "already experimenting with workflows where developers go directly from specs to review, with AI agents handling implementation, testing, and deployment in the background."

### Conductor vs orchestrator, and the 80% problem

- **Conductor**: real-time, in the IDE, directing each change. **Orchestrator**: async, hands goals to one or more agents and reviews what comes back; needs specification, decomposition, evaluation, and system-design skills.
- **The 80% problem**: agents produce roughly 80% of a feature quickly; the remaining 20% (edge cases, error handling, integration, subtle correctness) needs context the models lack. AI errors have moved from syntax mistakes to conceptual failures that look right.
- Anchoring example: an Anthropic agent team built a working C compiler in Rust over two weeks with humans setting direction and reviewing output; "the bottleneck moved from writing the code to specifying what it should do and verifying that the agents did it" (see section 4).

### Economics

Vibe coding is low CapEx, high OpEx (token burn from unstructured context and fix-my-mistake loops, maintenance tax, security remediation). Agentic engineering is high CapEx, low OpEx. Dense high-signal context raises first-pass success. Route hard reasoning to large models and routine work to small cheap ones.

### Three durable principles

1. Structure scales, vibes do not.
2. AI amplifies your engineering culture, strengths and weaknesses alike.
3. The human role is evolving, not diminishing: from implementation to judgment.

---

## 2. Factory Missions

Two sources. **Primary**: "How Missions Work" (`factory.ai/news/missions-architecture`, April 10, 2026) and the missions announcement (`factory.ai/news/missions`). **Talk-digest**: Luke Alvoeiro, "Missions: Multi-Agent Systems That Ship for Days" (AI Engineer), reconstructed from three independent digests that agree.

### Scale (primary)

- Longest mission: 16 days. 14% of missions exceed 24 hours. Median mission is about 2 hours against a median interactive session of about 8 minutes.
- Slack-clone example: 16.5 hours; 61 implementation features plus 21 fix features (a 34% correction ratio); 38.8k lines, 52.5% of them tests; 90% coverage.
- Brownfield and greenfield both appear as first-class use cases: a COBOL-to-Java migration (33.8 h), a production memory-leak investigation (24.2 h), a Tauri note-taking app (30 h), an HTTP benchmarking tool (22.3 h).
- Factory's own open questions: how far parallelization can go, and long-horizon correctness.

### Human attention is the bottleneck (talk-digest)

Models can plan far more backlog items than humans can scope, review, and unblock. The system's job is to absorb supervision load. Batch supervision scales human attention better than small per-token quality gains. Reported effect: a team's concurrent workstreams rose from about 10 to about 30.

### Orchestration strategies (talk-digest)

Five patterns: delegation, creator-verifier, direct communication, negotiation, broadcast. Missions combines four and **omits direct communication**: every agent reads and writes one authoritative shared mission state. A crashed or swapped worker resumes from state, and contradictions surface at write time.

### Three roles (primary, detail from talk-digest)

- **Orchestrator** plans: interrogates the operator, decomposes into milestones and features, writes the validation contract and per-role skills.
- **Workers** implement one feature each in a clean context window, commit via git, and hand off.
- **Validators** come in two kinds. The **scrutiny validator** runs lint, type checks, tests, and dedicated review agents. The **user-testing validator** drives the running app like QA (launch, click, fill forms, verify flows) and is where most wall-clock time goes. Validators have not seen the implementation, so validation is adversarial. Validators surface issues to the orchestrator, which scopes targeted fix features.
- Model per role: planning wants slow careful reasoning, implementation wants fast code fluency, validation wants precise instruction-following. Using a different provider in the validation seat reduces shared-model bias. Open-weight workers produce working code under strong scaffolding.

### Validation contract (primary, detail from talk-digest)

- A finite checklist of testable behavioral assertions, written by the orchestrator during planning, before any code exists; up to hundreds of assertions for a complex project. Every feature maps to assertions, and the features together must cover every assertion.
- Written before code because "tests written after implementation don't catch bugs, they confirm decisions."
- Validation almost never passes on the first attempt; about 60% of time and tokens go to implementation.

### Serial execution with read-only parallelism (primary + talk-digest)

Features run serially: one worker or validator active at a time. Parallel agents on a real codebase step on each other's changes. Read-only sub-tasks parallelize: codebase search, doc and API research within a feature, code review across features within a validator.

### Structured handoffs (talk-digest)

A finishing worker records what completed and what was skipped, the commands executed and their exit codes, issues discovered, and whether it followed orchestrator procedures. Errors are caught at milestone boundaries and scoped as fix features, rescoped milestones, or rollbacks.

### Prompts over code (talk-digest)

Almost all orchestration logic (feature decomposition, failure handling, negotiation) lives in about 700 lines of prompts and skills; deterministic code is thin bookkeeping (run validation, gate on unresolved handoffs, persist state). Parts that benefit from improving models go in prompts; parts that benefit from determinism go in code.

### Horizon (talk-digest)

The team believes 30-day missions are within reach. The July digest's stronger form ("viable to ~30 days before validation overhead and context drift make returns negative") is not in any reachable source.

---

## 3. Spec Driven Orchestration Principles

`PRINCIPLES.md`, Speccy, June 2026. Read directly; the summary below matches the current text.

- **Split of guarantees.** Bookkeeping (sequencing, gates, caps, evidence) is deterministic and auditable; implementation and review delegated to models stay nondeterministic. Speccy promises less supervision and an inspectable record of what happened, not that model output is correct or that a run never needs attention.
- **Before/after.** Before: iterate on a plan, start auto-mode implementation, babysit and steer. After: spend more brainpower on the plan, then step back; human attention concentrates at the gates (plan review, escalations, implementation review against recorded evidence).
- **Just another tool**: one more way to build inside a harness; does not replace `/plan`-and-implement and should not be used for everything.
- **Less is more**: YAGNI; more code and comments overload the human reviewer at the review gate.
- **Make drift visible**: sequential self-review via fresh-context agents on long-horizon tasks.
- **Zero product-code / build-time footprint**: no effect on product source, build graph, deployed artifacts, runtime dependencies, or production behavior. Repo-local harness packs (`.codex`, `.claude`, `.agents`, `.speccy` policy/prose) are acceptable workflow artifacts; run state, transcripts, evidence, screenshots, caches, and databases stay external or ignored by default.
- **No outbound agent runner**: Speccy commands never call LLMs, coding agents, or harnesses; the harness calls Speccy's deterministic controller.
- **Two layers**: a deterministic core and a prose layer (harness skills and subagents), to shrink the surface exposed to drifted or hallucinated model output.

---

## 4. Anthropic long-running harnesses

Three engineering posts plus the Managed Agents architecture note provide the primary sources.

### Effective harnesses for long-running agents (Justin Young, Nov 26, 2025)

- Agents work in discrete sessions with no memory of the previous one, like engineers on shifts. The harness is two configurations: an **initializer agent** (runs once: `init.sh`, a progress file, initial git commits, a feature list) and a **coding agent** (each later session: one feature, clean handoff).
- The **feature list is JSON** with every feature initially marked failing; over 200 features for the claude.ai-clone example. "It is unacceptable to remove or edit tests because this could lead to missing or buggy functionality."
- Session start ritual: `pwd`, read git log and `claude-progress.txt`, pick the next incomplete feature. Commit with descriptive messages; revert when needed.
- Failure modes and fixes: premature victory declarations (fix: structured feature inventory), context exhaustion mid-implementation (fix: one feature per session), undocumented bugs persisting (fix: mandatory end-to-end testing via browser automation), lost project state (fix: git plus progress notes on startup).

### Building a C compiler with a team of parallel Claudes (Nicholas Carlini, Feb 5, 2026)

- 16 parallel Claude Opus 4.6 instances, about two weeks, nearly 2,000 Claude Code sessions, about $20,000 (2B input tokens, 140M output tokens). Result: a 100,000-line C compiler in Rust that builds Linux 6.9 on x86, ARM, and RISC-V and passes 99% of GCC's torture tests.
- Coordination through a shared git repo and lock files in a `current_tasks/` directory: claim, work, pull, merge, push, unlock.
- **Tests are the specification.** "It's important that the task verifier is nearly perfect, otherwise Claude will solve the wrong problem." Monolithic goals stalled until tests gave granular comparison points against GCC.
- The human designed the environment: test suites and CI, README and progress docs, subsampled tests to avoid context pollution, specialized roles for code quality, docs, and performance. The output is unoptimized and the author is uneasy about shipping software no human has verified.

### Harness design for long-running application development (Prithvi Rajasekaran, Mar 24, 2026)

- Three roles. **Planner** expands a 1–4 sentence prompt into a full product spec. **Generator** implements in sprints. **Evaluator** drives the running app through Playwright, inspects API and database state, and files bug reports with code citations.
- **Sprint contracts**: before coding, generator and evaluator agree on testable success criteria (27 criteria for one level-editor sprint). The evaluator must approve the contract before implementation begins.
- Result: a solo agent produced a broken game in 20 minutes for $9; the harness produced a working, refineable one in 6 hours for $200.
- Findings: "Separating generation from evaluation is far more tractable than making a generator critical of its own work." Context resets with structured handoffs beat compaction. Generators under-scope without planning scaffolding and wrap up early when they sense the context limit.
- Guidance on evolution: the harness encodes assumptions about model limitations; as models improve, stress-test those assumptions and remove scaffolding that no longer earns its place. Evaluator tuning happened through prompt iteration, not code.

### Managed Agents (Apr 8, 2026)

Three virtualized components: a **session** (durable append-only event log outside the context window), a **harness** (stateless orchestration loop, recoverable by `wake(sessionId)`), and a **sandbox** (isolated execution). The harness owns session state, error handling, and context engineering; the model owns reasoning and tool choice. Credentials never reach the sandbox.

---

## 5. OpenAI: harness engineering, Codex platform, Codex goals

### Harness engineering: leveraging Codex in an agent-first world (Ryan Lopopolo, Feb 2026; secondary — reconstructed from a near-verbatim notes page and InfoQ)

- About five months, three engineers growing to seven, roughly one million lines and 1,500 merged PRs, zero hand-written code; single runs "upwards of six hours"; "about 1/10th the time" of writing by hand.
- `AGENTS.md` is a **map, not a manual**: about 100 lines that act as a table of contents into `docs/` — `design-docs/` (with an index and `core-beliefs`), `exec-plans/` split into `active/`, `completed/`, and a `tech-debt-tracker`, `product-specs/`, `references/`, plus `DESIGN.md`, `QUALITY_SCORE.md`, `RELIABILITY.md`, `SECURITY.md`. "Too much guidance becomes non-guidance… agents end up pattern-matching locally." "From the agent's point of view, anything it can't access in-context while running effectively doesn't exist." Custom linters check knowledge-base freshness and cross-links; a recurring doc-gardening agent prunes stale docs.
- **Architecture is enforced mechanically.** One rigid dependency order per domain (Types → Config → Repo → Service → Runtime → UI), cross-cutting concerns through a single `Providers` interface, structural tests in CI, and lint error messages that carry remediation instructions for the agent. "Custom linters were the biggest low-hanging fruit."
- **The application is made legible to the agent.** Chrome DevTools Protocol in the agent runtime provides DOM snapshots and screenshots; an ephemeral local observability stack per worktree is queried with LogQL and PromQL for prompts such as "ensure service startup completes in under 800 ms."
- **Review is agent-to-agent by default.** The agent self-reviews, requests further agent reviews, answers review inline, and often squashes and merges; "humans may review but aren't required to." Humans "validate outcomes and translate user feedback into acceptance criteria."
- **Merge philosophy:** "minimal blocking merge gates," short-lived PRs, flaky tests handled by follow-up runs. "In a system where agent throughput far exceeds human attention, corrections are cheap, and waiting is expensive."
- **Entropy control:** "golden principles" (prefer shared utilities, validate at data boundaries) encoded as lint; background tasks grade quality and open one-minute-review refactor PRs. "Technical debt is like a high-interest loan." Weekly human garbage collection did not scale.
- Failure mode: undocumented decisions in chat threads led agents to wrong choices. "If you can articulate what it is about the code you don't like, the next step is to write that down." Daily syncs became more important as architecture changed faster than humans noticed.
- Stated unknowns: long-run architectural coherence of a fully agent-written system, where human judgment adds the most leverage, and how the system evolves as models improve.

### Symphony (OpenAI, Apr 27, 2026; primary `SPEC.md`)

- A long-running daemon that turns an issue tracker (Linear, GitHub, Jira via adapters) into the control plane for Codex agents: poll every 30 s, one persistent workspace per issue, launch a Codex app-server session, retry with backoff, reconcile against tracker state. OpenAI reported a sixfold rise in merged PRs on internal teams in its first three weeks.
- **Orchestrator state is intentionally in memory**; recovery after restart comes from "tracker + filesystem," not a database. "Single authority: the orchestrator owns all state mutations." Adapters only read; ticket writes happen through provider-native tools the agent calls.
- Policy lives in `WORKFLOW.md`, committed to the repo: YAML front matter (tracker, polling, workspace hooks `after_create` / `before_run` / `after_run` / `before_remove`, concurrency, timeouts) plus a strictly templated prompt body that fails on unknown variables.
- A successful run ends at a "workflow-defined handoff state (for example `Human Review`), not necessarily `Done`." Trust posture is deployment-defined and must be documented; the spec warns against treating tracker content as trusted.
- Non-goals: a web UI, a general workflow engine, built-in ticket-editing logic, a mandated sandbox or approval posture. Symphony launches the agent itself; Speccy does not.

### Issue-tracker hierarchies (Jira, Linear, GitHub; primary docs)

- **Jira**: Epic → Story/Task → Sub-task by default; Initiative and custom levels above Epic require Plans (Premium). Sprints and Versions are orthogonal time and release boxes.
- **Linear**: Initiative → Project → Milestone → Issue → Sub-issue, with Cycles as the orthogonal time box. A milestone is an ordered "meaningful stage of completion" *inside* a project, not a date.
- **GitHub**: Issue → Sub-issue up to eight levels (GA 2025), issue types per organization, and Milestones as due-dated release groupings; sub-issues inherit the parent's projects and milestones.
- Common shape: one dispatch unit (issue) with a checklist beneath it (sub-issue/sub-task), an outcome container above it, and time boxes kept orthogonal to the hierarchy. "Milestone" means a stage in Linear and a date in Jira and GitHub.

### Codex as a platform (Aug 19–20, 2026; primary)

The harness under the Codex app, CLI, and IDE extension is open source (Apache-2.0): `codex exec` for bounded workflows in scripts and CI, the Codex SDK to start, resume, and stream tasks, and `app-server` for persistent conversations, streamed events, tool exposure, and approval flows. The harness owns conversation state, tool execution, sandbox and approval policy, and multi-turn continuity; model access stays separate.

### Codex goals (`/goal`, CLI 0.128.0, Apr 30, 2026; primary cookbook)

- A goal is a persisted, thread-scoped objective that survives restarts and multi-hour pauses; stored in app-server state, managed via `thread/goal/set|get|clear`.
- A strong goal states the desired outcome, the verification surface (tests, benchmarks, artifacts), constraints to preserve, resource budget, iteration policy, and blocked-stop conditions: "end state verified by [specific evidence] while preserving [constraints]."
- After each turn Codex audits progress against evidence (files changed, commands run, tests passed, artifacts). Budget exhaustion is reported as distinct from completion.
- Recommended for migrations, flaky-test hunts, multi-step refactors, and research artifacts; not for one-line edits or vague targets.

---

## 6. Harness engineering as a discipline

### My AI Adoption Journey (Mitchell Hashimoto, Feb 5, 2026; primary — the post that named the discipline)

Six stages, from abandoning chatbots for agents to keeping agents running continuously. Harness engineering is "the idea that anytime you find an agent makes a mistake, you take the time to engineer a solution such that the agent never makes that mistake again." The fixes are documents of known-bad behaviors (Ghostty's `AGENTS.md`), purpose-built tools (a screenshot utility, a filtered test runner), and pairing each doc update with the tool that enforces it. Kept by hand: the thinking he enjoys, final decisions on delegated work, and judgment about what to delegate.

### The Coming Loop and Agentic Coding Recommendations (Armin Ronacher, Jun 23, 2026 and Jun 12, 2025; primary)

- Two nested loops: the **agent loop** inside a coding agent (call a tool, read the result, edit, test) and the **harness loop** outside it that decides whether the work is really done and then continues the session, injects a message, starts fresh with modified context, or routes the task elsewhere. Harness loops work for mechanical work that yields temporary or translated artifacts (ports, performance sweeps, security scans).
- Warnings: "present-day hands-off harnesses like Claude Code with ultracode produce worse code than what we were producing last autumn"; "looping is powerful but it removes responsibility more and more." The question he poses is how a responsible human keeps supervising.
- The 2025 recommendations: fast, unambiguous tools ("crashes are acceptable; hangs are problematic"); log everything to files so the agent can diagnose; "the dumbest possible thing that will work"; plain CLI tools over MCP servers for reliability.

### Harness engineering for coding agent users (Birgitta Böckeler, martinfowler.com, Apr 2, 2026; primary)

- A harness is everything in a coding agent except the model. Two control kinds: **guides** (feedforward: docs, rules, examples, bootstrap scripts, codemods) steer before the agent acts; **sensors** (feedback: linters, structural tests, pre-commit hooks, AI review) observe after and drive self-correction. Guides alone lack verification; sensors alone let mistakes repeat.
- Each is **computational** (deterministic, milliseconds) or **inferential** (slow, non-deterministic, semantic). Computational sensors catch structural problems and miss semantic ones.
- **Harnessability**: strong typing, clear module boundaries, and framework conventions make a codebase legible to agents; legacy debt lowers it.
- Cautions: neither control kind reliably catches misdiagnosis or misunderstood requirements; the behavioral harness is underdeveloped and over-relies on AI-written tests; keeping a large harness coherent is unsolved.

### Harness engineering and agent feedback: exploring AI coding sensors (Böckeler and Ford, Thoughtworks, May 13, 2026; primary)

Empirical follow-up to the taxonomy. A coding agent equipped with computational sensors (ESLint, Semgrep, dependency-cruiser, coverage reports, mutation testing) raised code quality over time, including test coverage, without human prompting. Stated framing: harness engineering "isn't about total automation; it's really about situational awareness for the developer."

### Improving Deep Agents with harness engineering (LangChain, Feb 17, 2026; primary)

- The primary behind the Terminal-Bench 2.0 figure the Google paper cites: GPT-5.2-Codex held fixed, 52.8% (about Top 30) to 66.5% (Top 5).
- Levers: a system prompt restructured around plan → build → verify → fix; middleware hooks (`PreCompletionChecklistMiddleware`, `LocalContextMiddleware` for environment discovery, `LoopDetectionMiddleware` to interrupt repeated edits); a "reasoning sandwich" that spends maximum reasoning on planning and verification and medium on implementation.
- Most impactful change: forcing self-verification. "The most common failure pattern was that the agent wrote a solution, re-read its own code, confirmed it looks ok, and stopped."
- Traces drove the loop: an automated trace-analyzer skill read failures across runs and proposed harness changes. Stated lesson: guardrails that address current model weaknesses will become obsolete as capabilities improve.

### Agentic Engineering Patterns (Simon Willison, Feb 23, 2026, ongoing; and "Vibe coding and agentic engineering are getting closer than I'd like", May 6, 2026; primary)

- A pattern guide for professionals using coding agents (`simonwillison.net/guides/agentic-engineering-patterns/`); chapters so far: "Writing code is cheap now" and "Red/green TDD" (test-first gives agents a concrete target and shorter output with little prompting).
- The May post reports that as agents got more reliable he stopped reviewing every line, even for production code, and treats agents like a trusted team: monitor for problems rather than inspect everything up front. Replacement evidence is behavioral: automated tests and documentation as guardrails, and real use ("if you've got a vibe coded thing which you have used every day for the past two weeks, that's much more valuable").

### Ralph loop (Thoughtworks Technology Radar vol. 34, Apr 2026, ring Assess; primary)

A fixed prompt fed to an agent in a loop; each iteration starts a fresh context, picks one task from a spec or plan, implements it, and restarts. State accumulates in files and git, not in conversation memory. Benefit: simplicity versus multi-agent orchestration and no quality degradation from accumulated context. Cost: significant tokens from re-reading context each iteration. Related Radar entries: team of coding agents (Assess), coding agent swarms (Caution).

### Research notes (arXiv abstracts; primary)

- **Harness Engineering for Agentic AI Coding Tools: An Exploratory Study** (2602.14690; 2,853 repos): context files dominate and are often the only mechanism, with `AGENTS.md` emerging as the interoperable standard; skills and subagents are rare, and skills that exist are mostly static instructions.
- **Agent Skills: A Data-Driven Analysis** (2602.08004, Bosch Research and CMU; 40,285 skills): median skill body 1,414 tokens.
- **From Anatomy to Smells: An Empirical Study of SKILL.md** (2607.01456; 238 files): a taxonomy of 13 high-level and 44 low-level components; over 99% of files carry at least one "skill smell," and smells persist as skills evolve; ships an automated detector.
- **What Keeps Agent Skills from Being Reusable?** (2608.08453; 138,133 files across 20,556 repos): 91.8% of skills have at least one defect, dominated by weak routing metadata, oversized or unclear bodies, and disorganized resources rather than security issues; skills with sound routing metadata are retrieved more successfully. Recommends spec-aware generation, linting, and self-repair.
- **Adapting the Interface, Not the Model** (2605.22166): adapting the runtime harness (observation, tool use, action execution, feedback) instead of model weights improved 116 of 126 model–environment configurations in deterministic rule-governed domains (average relative gain 88.5%); harnesses adapted on one model transferred to 17 others, so the harness encodes the environment, not the model.
- **Natural-Language Agent Harnesses** (2603.25723): harness policy as editable documents that a runtime interprets into agent calls, handoffs, state updates, validation gates, and artifact contracts; matches code-based harnesses with much shorter static policy.
- **Agentic Harness Engineering** (2604.25850): automatic harness evolution; the gains came from evolving tools, middleware, and long-term memory rather than the system prompt (Terminal-Bench 2 pass@1 69.7% to 77.0% over ten iterations).
- **From Model Scaling to System Scaling** (2605.26112): names memory substrate, context constructor, skill routing, orchestration loop, and verification-and-governance as the harness layers, and proposes harness-level benchmarks (trajectory quality, memory hygiene, context efficiency, verification cost).
- **Stop Comparing LLM Agents Without Disclosing the Harness** (2605.23950, May 2026): the "Binding Constraint Thesis" — on long-horizon tasks harness-induced variance exceeds model-induced variance, with documented rank reversals; proposes a harness disclosure standard for evaluations.
- **Effective Harness Engineering for Algorithm Discovery with Coding Agents** (2605.15221): under a fixed budget, "generating fewer algorithms while thinking more deeply about each one achieved higher scores"; stronger models produced evaluation hacks at higher rates, so hack detection matters more as models improve.
- **Self-Harness** (2606.09498) and **HarnessX** (2606.14249): harnesses that mine their own failure traces, propose minimal edits, and accept them only after regression testing; relative gains up to 132% (Self-Harness) and +14.5% average across five benchmarks (HarnessX). Self-editing harnesses are now a research thread with reported results.
- **Code as Agent Harness** survey (2605.18747, May 2026): code as harness *interface* (reasoning, acting, environment modeling), harness *mechanisms* (planning, memory and context, tools, feedback-guided debugging), and *scaling* (roles, topology, convergence criteria).
- **Harnessing Agent Skills** (2606.20631): ten architectural patterns and a four-layer reference architecture for skill-mediated agents — Supply Chain, Mediation, Execution Control, Evidence & Feedback.
- **Critique — "What is harness engineering?"** (Stuart Miller, May 8, 2026): the discipline is platform engineering, control-plane design, and SRE under a new name; the one new ingredient is a stochastic component. Useful as a check on novelty claims.

---

## 7. Running a Software Factory Efficiently at Uber Scale

Uber engineering blog, Uday Kiran Medisetty, August 27, 2026. Primary.

### What it reports

- Spend is modeled as Users × Sessions/User × Turns/Session × Requests/Turn × Tokens/Request × Price/Token, and each lever is optimized independently. February to August 2026: weekly active users up 7x, agentic requests up 9.4x, cost per 1,000 requests down 34% at constant model, cost per session down 52% from the June peak.
- Over 70% of PRs are attributed to local or cloud agents. 3,600+ skills, 30K+ skill executions per day. A growing share of sessions are started by **managed agents** (code review, self-healing CI, E2E PRs with visual validation, alert triage), not humans. Moving workloads into managed environments gives full control over model routing, harness, and spend.
- Agents are layered task-specific → domain → cross-domain → general-purpose; the more specific the layer, the more control over cost, quality, and model choice.
- **uReview** is benchmarked on real PRs with known bugs graded easy/medium/hard and scored on precision, recall, F1, cost per review, latency, timeouts, and **noise**; model choice is picked from the Pareto frontier and re-evaluated as the frontier shifts every few weeks.
- Pre-loading MCP schemas cost 50–70K tokens per session; **CLI-resolved tool calls** and tool search removed it. **Code-mode** skills batch several tool actions into one script instead of one model turn per action (55–100% token savings on SQL examples).
- Defaults: subagents run a weaker, cheaper model because their tasks are well-defined; reasoning effort defaults to medium; compaction triggers at 400K tokens even on 1M-context models; interactive sessions use a 1-hour prompt cache TTL, subagents 5 minutes.
- Outcome-denominated cost (per merged PR, per review, per alert) plus quality signals (revert rate, F1, MTTR) are the managed-agent metrics. Future work: auto-generating skill updates from recorded execution traces.

### Takeaways that bear on Speccy

- A CLI that returns JSON is the cheap tool surface; an MCP server is the expensive one. This supports keeping `speccy ctl` as the only controller interface and leaving MCP deferred.
- Per-role model and effort selection for well-defined roles (worker, verifier, reviewer persona) is standard practice at scale, not an optimization to defer.
- Reviewer output should be measured on noise as well as recall: false-positive findings cost the human attention the tool exists to save.
- Everything else in the article (cost dashboards, spend nudges, context graphs, MCP gateways) is enterprise-scale infrastructure with no bearing on a single-developer run controller.

---

## 8. Harness surfaces Speccy targets (state as of Aug 2026)

The claims below come from specification text and vendor documentation. Version numbers are the documents' own.

### Standards governance

The Linux Foundation formed the **Agentic AI Foundation (AAIF)** on December 9, 2025, with three founding projects: MCP (Anthropic), goose (Block), and **`AGENTS.md`** (OpenAI). Eight platinum members (AWS, Anthropic, Block, Bloomberg, Cloudflare, Google, Microsoft, OpenAI) and 47 founding members in total; 170+ members by April 2026. `AGENTS.md` is described as "a consistent source of project-specific guidance needed to operate reliably across different repositories and toolchains" and is used by more than 60,000 open-source projects. It is a foundation-governed standard, not a vendor convention.

### Agent Skills specification (`agentskills.io`)

- A skill is a directory with `SKILL.md` (YAML frontmatter plus markdown body) and optional `scripts/`, `references/`, `assets/`. Required frontmatter: `name` (1–64 chars, lowercase alphanumerics and hyphens, must match the directory name) and `description` (1–1024 chars, what it does and when to use it). Optional: `license`, `compatibility`, `metadata`, experimental `allowed-tools`.
- Progressive disclosure: metadata (~100 tokens) at startup for every skill; the body (<5,000 tokens recommended, under 500 lines) on activation; resources on demand, one reference level deep. `skills-ref validate` checks a skill.
- Published as an open standard December 18, 2025; about 40 products support it, including Claude Code, Codex, Copilot, Cursor, Gemini CLI, Goose, and OpenCode.

### Claude Code

- **Subagents** (`.claude/agents/*.md`, or `~/.claude/agents/`, or `--agents` JSON): frontmatter `name`, `description`, `tools` / `disallowedTools`, `model` (family alias, full ID, or `inherit`), `permissionMode`, `maxTurns`, `skills` (preloaded), `mcpServers`, `hooks` (PreToolUse, PostToolUse, Stop), `memory` (`user` | `project` | `local`), `background`, `effort`, `isolation: worktree`. Fresh subagents get the CLAUDE.md hierarchy and a git status snapshot but no conversation history; fork subagents inherit the full conversation and prompt cache. Default depth limit 3, 20 concurrent. Subagent descriptions together should stay under 15,000 tokens.
- **Agent teams** (experimental, `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`; not available under `-p`): a lead plus teammates that each run a full session, share a task list with dependency tracking and file-locked claiming, and message each other directly through per-agent mailbox files. Limitations: no session resumption of in-process teammates, task status can lag, one team per session, no nested teams. Hooks `TeammateIdle`, `TaskCreated`, `TaskCompleted` can block with exit code 2. Docs recommend starting with research and review, not parallel implementation, and giving each teammate its own files.
- **Dynamic workflows** (`.claude/workflows/*.js`, `/workflows`, keyword `ultracode`): a JavaScript script with `agent()`, `parallel()`, `pipeline()`, `phase()` orchestrates dozens to hundreds of subagents; the script, not Claude, decides what runs next and intermediate results live in script variables. Determinism is enforced (`Date.now()`, `Math.random()`, `new Date()` throw) so a relaunched run replays completed agents from cache and reruns from the first changed prompt. Limits: no mid-run user input, 16 concurrent agents, 1,000 agents per run, 4,096 items per fan-out. Runs are resumable within a session and replayable after `--resume`.
- Comparison table from the docs: subagents and skills keep the plan in Claude's context; teams keep it in a shared task list; workflows keep it in a script.
- **Plugins**: a directory with an optional `.claude-plugin/plugin.json` (`name` is the skill namespace, `version` gates updates) and root-level `skills/`, `agents/`, `hooks/hooks.json`, `.mcp.json`, `.lsp.json`, `monitors/`, `bin/`, `settings.json`, and `workflows/`. Installed from git-hosted marketplaces (`/plugin marketplace add`, `/plugin install`), from `--plugin-dir` or `--plugin-url` for testing, or auto-loaded from `~/.claude/skills/<name>/` via `claude plugin init`. Plugin skills are namespaced `/plugin:skill`; project and user `.claude/agents/` override same-named plugin agents; plugin agents do not support `hooks`, `mcpServers`, or `permissionMode`. Anthropic runs a curated official marketplace and a reviewed community marketplace; `claude plugin validate` runs the same check as the review pipeline.

### Codex

- **Skills** are discovered from `.agents/skills` in every directory from the working directory to the repo root, then `$HOME/.agents/skills`, `/etc/codex/skills`, and bundled skills; invoked implicitly on match or explicitly with `$name`. An optional `agents/openai.yaml` adds interface metadata, `policy.allow_implicit_invocation`, and dependencies.
- **Custom agents** (`.codex/agents/*.toml` or `~/.codex/agents/`): required `name`, `description`, `developer_instructions`; optional `model`, `model_reasoning_effort`, `sandbox_mode`, `mcp_servers`, `nickname_candidates`; other settings inherit from `config.toml`. Built-ins `default`, `worker`, `explorer` can be overridden by name. MultiAgentV2 (CLI 0.128.0, Apr 30, 2026) defaults: `max_threads` 6, `max_depth` 1, 1800 s per worker; spawns past the cap queue rather than fail; children receive role instructions rather than forked history by default.
- **Goals** (`/goal`): see section 5. **Hooks** are stable and `--full-auto` was replaced by persisted permission profiles (Apr 2026). An in-app browser drives local dev servers for visual verification. `AGENTS.md` truncates past 32 KiB.
- **Plugins** (marketplace launched March 27, 2026): a directory with `.codex-plugin/plugin.json` plus optional `skills/`, `.mcp.json`, `.app.json` (OAuth/API-token service connectors), and `assets/`; all manifest paths are `./`-relative. Marketplaces are `marketplace.json` catalogues at three scopes: OpenAI's official directory, repository (`.agents/plugins/marketplace.json`), and personal (`~/.agents/plugins/marketplace.json`); `/plugin marketplace add org/repo[@branch|#tag]`, `/plugin install name@marketplace`, `/reload-plugins`. Enterprise policies: `INSTALLED_BY_DEFAULT`, `AVAILABLE`, `NOT_AVAILABLE`. Self-serve publishing to the official directory was still "coming soon" as of May 2026. Codex plugins bundle skills and connectors but not custom agents.

---

## 9. Spec-driven development tools (Aug 2026)

Primary READMEs for Spec Kit, OpenSpec, and GSD Core; secondary comparison articles for the rest.

| Tool                       | Artifacts and location                                                                                                                  | Workflow                                                                                                   | Notes                                                                                                                                           |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| GitHub Spec Kit            | Repo-resident markdown: constitution, spec, plan, tasks, checklist                                                                      | `/speckit.constitution` → `specify` → `clarify` → `plan` → `tasks` → `analyze` → `implement` → `checklist` | v1.0 (Aug 2026), MIT, installs into 30+ agents; `analyze` cross-checks artifacts for consistency.                                                |
| OpenSpec                   | `openspec/changes/<change>/{proposal.md, specs/, design.md, tasks.md}`; specs use WHEN/THEN scenarios and ADDED/MODIFIED/REMOVED deltas | `/opsx:explore` → `propose` → `apply` → `archive` (archive folds deltas into `openspec/specs`)             | Positioned for brownfield change tracking; "fluid not rigid, iterative not waterfall"; 30+ tools.                                                |
| AWS Kiro                   | `requirements.md` (EARS notation), `design.md`, `tasks.md`                                                                              | Requirements → design → tasks; agent hooks fire on file save                                               | Integrated IDE/CLI inside the AWS perimeter.                                                                                                     |
| GSD Core (Open GSD)        | Repo-local `STATE.md`, `CONTEXT.md`, and planning artifacts                                                                             | Discuss → plan → execute (parallel waves in fresh 200k contexts) → verify (dedicated agents) → ship        | Successor to get-shit-done (archived June 26, 2026 at 64.6k stars); supports Claude Code, Codex, OpenCode, Copilot, Cursor, Gemini CLI, others. |
| BMAD, Tessl, GSD 2.0 forks | Role-separated agent chains (BMAD); `.tessl/` spec tiles plus a library-spec registry (Tessl)                                            | Full-SDLC role handoffs                                                                                    | Heavier ceremony; Tessl targets API hallucination via registry specs.                                                                           |

Common shape: the spec is the source of truth, artifacts are markdown committed to the repo, and the tool's own workflow is prompt-driven inside the host harness. Differentiators are spec format (constitution, EARS, deltas), whether the tool orchestrates implementation or only writes specs, and lock-in. None of the surveyed tools records requirement-linked verification evidence or keeps run state outside the repository; verification is a phase, not a ledger.

---

## 10. Context Engineering: Sessions & Memory

Google whitepaper (Day 3 of the Kaggle agents series), Kimberly Milam and Antonio Gulli, November 2025. **Secondary**: digested from an independent summary; the Kaggle PDF was not fetched.

- A **session** is an ephemeral container for one conversation: an immutable, timestamped **event log** (messages, tool calls, results) plus mutable key-value **state**. **Memory** persists across sessions as extracted facts. Trust hierarchy for memories: user-stated > observed > inferred.
- Six context sources compete for the token budget: system instructions, conversation history, tool definitions, memories, retrieved documents, output structures.
- Compaction strategies trade information loss against cost: truncation (high loss, cheap), keep-last-N (medium), recursive LLM summarization (low loss, expensive); the paper recommends truncation followed by summarization of what is kept.
- Memory kinds: declarative (facts and events) and procedural (skills and behaviors). Generation is an ETL pipeline: extract from the session, consolidate (dedupe, validate), store indexed, retrieve semantically. **Memory-as-a-tool** (the agent decides when to store and recall) is preferred over automatic injection for auditability and token economy.
- Multi-agent systems use either one shared history (tightly coordinated workflows) or separate per-agent histories connected over A2A.

---

## Cross-cutting threads

- **Verification, not generation, is the binding constraint.** Across the sources, verification recurs as the limiting factor: the Google paper calls it the single biggest differentiator; Factory says validation almost never passes first try; Carlini says the verifier must be nearly perfect or the agent solves the wrong problem; the algorithm-discovery paper adds that stronger models game evaluations more, so a judge must look for satisfied-by-weakening.
- **The harness, not the model, sets long-horizon quality.** The Binding Constraint Thesis (2605.23950), LangChain's fixed-model 13.7-point gain, and the runtime-adaptation paper's cross-model transfer support this claim. Evaluations should disclose the harness alongside the result.
- **Fewer, deeper units of work.** The sources favor deeper work on fewer units: "Generating fewer algorithms while thinking more deeply about each one" (2605.15221), one feature per Anthropic session, and a permanent fix per Hashimoto mistake.
- **Write the done-condition before the code.** Factory's validation contract, Anthropic's sprint contract and failing-by-default feature list, Codex goals' "end state verified by [evidence]", Carlini's tests-as-spec, the Google factory model's "success criteria, not step-by-step instructions."
- **Separate the writer from the verifier, in a fresh context.** Factory validators, Anthropic's evaluator ("far more tractable than making a generator critical of its own work"), Ralph's fresh iteration, GSD's verify agents.
- **State lives outside the context window.** Factory's single shared mission state, Anthropic's progress file and Managed Agents' append-only session log, Ralph's files-and-git, GSD's `STATE.md`, Codex goals' persisted thread state, Claude workflows' script variables and cached agent results.
- **The sources favor serial writes and parallel reads.** Factory states the policy; Claude Code's team docs recommend research and review with one file owner per teammate; Carlini's parallel writers needed lock files and merges.
- **Prompts for what improves with models, code for what needs determinism.** Factory's 700-lines-of-prose / thin-bookkeeping split, the NLAH paper, PRINCIPLES' two layers. Anthropic and LangChain add the corollary: scaffolding that compensates for a model limitation should be removed when the limitation goes away. The runtime-adaptation paper (2605.22166) adds the converse: a harness that encodes the environment transfers across models.
- **Guides and sensors, not more instructions.** OpenAI's linters and structural CI tests, Böckeler's taxonomy, and Carlini's test suites support encoding taste as a deterministic sensor where possible. The Google paper likewise attributes most failures to configuration.
- **Human attention moves to the edges and must be spent on signal.** Factory's supervision-load framing, the Google conductor-to-orchestrator shift, PRINCIPLES' gates; Uber adds that reviewer noise is a first-class cost; Willison reports line-by-line review giving way to behavioral evidence (tests, real use) even for production code.
- **Skills are the portable unit of procedural knowledge**, now under one open spec both target harnesses read and a foundation-governed `AGENTS.md`. Static context is expensive; keep `AGENTS.md` a map and push detail behind progressive disclosure. Most published skills are defective in routing metadata and body size (91.8% in the 138K-file study), so `name`/`description` quality and body length are the first things to lint.

## What changed since the July 2026 digest

- The two target harnesses grew their own long-running and multi-agent primitives: Codex goals (persisted objective with evidence-based completion), Claude Code dynamic workflows (deterministic, replayable orchestration scripts), agent teams (shared task list, peer messaging), subagent `memory`, `isolation: worktree`, and per-agent hooks. Harness-native loops with resume now exist on both sides.
- Factory omits direct agent-to-agent messaging by design; Claude Code ships it through agent teams. The two positions coexist, and the team docs still recommend independent file ownership and research-first use.
- The Agent Skills format is a cross-vendor standard with a validator, size guidance, and a measured median body of about 1,400 tokens; two large-scale studies find most published skills defective in routing metadata or body size. `AGENTS.md` moved under Linux Foundation governance (AAIF, Dec 2025).
- Both harnesses distribute skills, agents, hooks, and MCP config as versioned plugins from git-hosted marketplaces with repository- and user-scoped catalogues; Codex's official directory does not yet accept self-serve submissions.
- Anthropic published the three-role planner/generator/evaluator harness with sprint contracts and the explicit advice to remove scaffolding as models improve.
- OpenAI open-sourced the Codex harness (`codex exec`, SDK, `app-server`) and documented a year of agent-first development around linters, layering constraints, and a `docs/` knowledge base.
- Spec-driven tooling consolidated: Spec Kit hit 1.0, GSD moved to GSD Core, OpenSpec settled on proposal/delta artifacts. None added evidence ledgers.

## Removed claims

Present in the July 2026 digest, attributed to the Factory analysis, and absent from every reachable written source (the architecture post, the announcement, three talk digests, the Stack Overflow and Zero Prime interviews with Eno Reyes). They may originate in a second, untranscribed talk; treat them as unsupported until a source is found.

- Workers edit their own skills mid-mission, and without that loop a 40-task run re-learns friction and degrades past about 10 tasks. The verified form is weaker: the orchestrator writes per-role skills at planning time.
- Brownfield-first, greenfield-second build order.
- The diagnostic "if you cannot validate, the run produces something ~85% correct and the last 15% costs more than doing it by hand."
- "100+ feature projects fail because the validation harness is the bottleneck" (the primary example is 61 + 21 features and passed).
- The localhost↔production gap examples (Firecracker VMs, bot-protected OAuth, multi-region deploys).
- The four closing open questions (composition into higher-order workflows, governance under a human constitution, and so on). Factory's own stated open questions are parallelization limits and long-horizon correctness.
