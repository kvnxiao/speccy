# Source Summaries

Decision-relevant sources were checked against primary pages on September 5, 2026. These notes separate documented capabilities, reported experience, limited research findings, and Speccy's design choices. Experiments were not reproduced. Live documentation and unpinned repository branches require another check when implementing a harness pack or integration.

## Evidence Used in the Design

### Persisted Context and Separate Evaluation

[Factory's Missions architecture](https://factory.ai/news/missions-architecture) describes orchestration, workers, validators, and a validation contract. [Anthropic's long-running harness report](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) describes external progress artifacts and structured session handoff. These are primary engineering experience reports, not controlled comparisons of Speccy's design.

[Anthropic's application-development report](https://www.anthropic.com/engineering/harness-design-long-running-apps) describes separate implementation and evaluation and reports that separation alone did not eliminate evaluator leniency. Resetting context also has costs. The reports motivate persisted contracts and independent evaluation; they do not establish the optimal review schedule, strict history exclusions, or a universal advantage for serial work.

**Design choice:** retain fresh Task and cumulative Spec review, then measure missed defects, false positives, and repeated review cost. Keep serial implementation because it simplifies ownership. These policies are hypotheses subject to the charter's approval rules.

### Repository Guidance and Existing Checks

[OpenAI's harness-engineering report](https://openai.com/index/harness-engineering/) describes a short repository instruction file that routes agents to maintained documentation, architectural checks, and inspectable application behavior. Its productivity estimates describe one team's experience and are not a controlled prediction for other repositories.

**Design choice:** keep `AGENTS.md` as a map, keep each contract in one authoritative document, and use repository-prescribed checks. Controller-owned execution of product checks remains deferred.

### Harness Configuration and Model Choice

[LangChain's engineering report](https://www.langchain.com/blog/improving-deep-agents-with-harness-engineering) reports Terminal-Bench 2.0 improvement from 52.8% to 66.5% while holding GPT-5.2-Codex fixed and changing the harness. The result supports evaluating prompt, tool, and middleware configuration; it does not establish that model choice is generally secondary.

[The harness-disclosure position paper](https://arxiv.org/abs/2605.23950) frames its argument around comparable frontier models on long-horizon tasks. Only its abstract-level claim is used here; the paper is not a reproduced benchmark.

[Uber's software-factory report](https://www.uber.com/gb/en/blog/efficient-software-factory/) describes workload-specific model routing and reductions in tool loading and polling overhead. Its results do not establish that MCP is intrinsically expensive or that every controller needs a routing optimizer.

**Design choice:** configure global role profiles and explicit Spec overrides, record model and harness settings during evaluation, and defer automated routing and cost-accounting infrastructure.

### Skill Format and Reliability

[Agent Skills](https://agentskills.io/specification) defines `SKILL.md` metadata and progressive disclosure. Discovery paths, permission behavior, invocation syntax, and fresh-context behavior still depend on the harness.

[The skill-reusability study](https://arxiv.org/abs/2608.08453) reports detected defects in 91.8% of 138,133 sampled files across 20,556 repositories under its taxonomy, plus a separate metadata-routing stress test. This abstract-level evidence supports metadata and routing checks. It is not an estimate of operational skill failure or proof that valid metadata produces useful behavior.

**Design choice:** validate the format and exercise concrete requests. Keep the charter skill self-contained; its current workflow does not need additional reference files or scripts.

### Canonical Storage

[SQLite's atomic-commit documentation](https://sqlite.org/atomiccommit.html) defines transaction behavior and the filesystem assumptions on which it depends. External artifact files are not included in a database transaction.

**Design choice:** use one transactional canonical database, publish complete evidence before committing its references, and test interruption boundaries on supported platforms. Atomic database writes do not authenticate agent-produced evidence or stop old workers.

## Harness Compatibility Facts

### Codex

[Skills documentation](https://learn.chatgpt.com/docs/build-skills) describes metadata-first loading and explicit skill invocation. Discovery budgets can shorten or omit entries; a correct file does not ensure an ambiguous request selects it.

[Subagent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents) describes custom agents in TOML and configurable model and effort settings. Current configuration prefers `agents.max_concurrent_threads_per_session`; `agents.max_threads` is a legacy alias. Installation must check the actual client, parent permissions, and fresh-context behavior instead of assuming fixed defaults.

[Plugins](https://learn.chatgpt.com/docs/plugins) distribute skills and connectors through a shared ChatGPT/Codex catalog with a documented submission flow. Plugin support differs by client; distribution does not by itself establish custom-agent installation.

[Native goals](https://learn.chatgpt.com/use-cases/follow-goals) and [App Server goal operations](https://learn.chatgpt.com/docs/app-server) already provide persistent objectives. Persistence alone is not a sufficient product distinction for Speccy.

### Claude Code

[Instruction-loading documentation](https://code.claude.com/docs/en/memory) states that `AGENTS.md` is not automatically loaded. A `CLAUDE.md` import can load it. [Subagent documentation](https://code.claude.com/docs/en/sub-agents) distinguishes fresh contexts from forks; inherited project instructions still need inspection. [Dynamic workflows](https://code.claude.com/docs/en/workflows) have their own execution and input contract and are not interchangeable with an interactive orchestration loop.

**Pack requirement:** check routing, charter import, scoped fresh inputs, permissions, profiles, actual cancellation, human decisions, and resume on each supported installed client. The shared skill format does not establish portability by itself.

## Existing Alternatives

[Spec Kit](https://github.com/github/spec-kit) and [OpenSpec](https://github.com/Fission-AI/OpenSpec) document structured specification workflows. OpenSpec's README includes beta Stores for planning in a separate repository.

[GSD Core's verifier](https://github.com/open-gsd/gsd-core/blob/next/agents/gsd-verifier.md) maps requirement identifiers to implementation evidence and writes a coverage report. Re-verification may read earlier findings. The inspected `next` branch is not a pinned installed release. Persisted requirement evidence is not unique to Speccy; controlled canonical writes and its chosen review-context policy are narrower proposed differences.

[GitHub's sub-issue documentation](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/adding-sub-issues) treats projects and milestones as separate fields. Tracker vocabulary supplies examples, not evidence that Speccy needs hierarchy in its local MVP. README inspection also cannot prove that a competing tool lacks an undocumented capability.

## Further Reading and Unverified Material

The remaining catalogue is a reading queue, not evidence for a requirement. Detailed numerical and methodological claims from these entries are excluded until the primary text and relevant methods are checked:

- [Google's SDLC whitepaper listing](https://www.kaggle.com/whitepaper-the-new-SDLC-with-vibe-coding): the fetched page did not expose the paper text. The companion Sessions and Memory notes also lack a verified primary text in this review.
- Factory talk notes attributed to Luke Alvoeiro: no linked transcript or digest establishes the detailed claims. The linked Factory architecture report above supplies the usable evidence.
- Additional harness and skill abstracts: [2602.14690](https://arxiv.org/abs/2602.14690), [2602.08004](https://arxiv.org/abs/2602.08004), [2607.01456](https://arxiv.org/abs/2607.01456), [2605.22166](https://arxiv.org/abs/2605.22166), [2603.25723](https://arxiv.org/abs/2603.25723), [2604.25850](https://arxiv.org/abs/2604.25850), [2605.26112](https://arxiv.org/abs/2605.26112), [2605.15221](https://arxiv.org/abs/2605.15221), [2606.09498](https://arxiv.org/abs/2606.09498), [2606.14249](https://arxiv.org/abs/2606.14249), [2605.18747](https://arxiv.org/abs/2605.18747), and [2606.20631](https://arxiv.org/abs/2606.20631).
- Practitioner essays by Mitchell Hashimoto, Armin Ronacher, Birgitta Böckeler, and Simon Willison, the Ralph-loop radar entry, OpenAI Symphony, and the Kiro, BMAD, and Tessl catalogue entries remain discovery leads. Their earlier summaries are not adopted as verified requirements.

## Research Gaps and Update Rule

No inspected source validates Speccy's exact review schedule, history exclusions, recovery-budget default, or benefit over a native workflow. Dogfooding must evaluate those choices. Serial ownership and transactional storage are engineering choices with explicit invariants, not claims of benchmark superiority.

When a capability affects implementation, reopen its primary documentation and pin the tested client or repository revision in the implementation's compatibility evidence. When a source becomes unavailable, mark the claim unverified or remove it from the decision basis. Keep quotations and numerical results limited to what the linked source supports, and preserve workload and methodological limits.
