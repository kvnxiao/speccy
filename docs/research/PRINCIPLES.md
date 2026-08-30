## **Spec Driven Orchestration Principles**

Agentic orchestration tools must be designed to integrate with any agent harness, and provide an autonomous approach to agentic engineering via concrete checkpoints for human intervention, thereby reducing the realtime need to steer an agent on the fly. The split of guarantees is deliberate: the bookkeeping — sequencing, gates, caps, evidence — is deterministic and auditable, while the implementation and review work delegated to models remains nondeterministic. Speccy promises less supervision and an inspectable record of what happened, not that model output is correct or that a run never needs attention.

Before: iterate on a plan or prompt, and then start implementation in auto-mode, baby sitting on the AI model's output and steering it for directional corrections.

After: focusing more brainpower on the initial plan such that it is as comprehensive as possible, then stepping back to allowing the AI model to implement the plan autonomously. Human intervention concentrates at the gates — reviewing the plan, answering escalations, and reviewing the implementation against recorded evidence — instead of continuous steering.

## **Core Principles**

* **Just another tool available** - simply provides another way of building software in an agent harness; does not replace existing approaches (e.g. regular `/plan` and implement) and should not be used for everything
* **Less is more** - avoid over-engineering and focus on simple, reusable solutions (YAGNI); more code (including comments) leads to contextual overload for the human reviewer at the review stage  
* **Make drift visible for self improvement** - employ sequential self-review and self-improvement via fresh-context agents in implementing long-horizon tasks, resulting in higher quality output
* **Zero product-code / build-time footprint** - tool should not affect product source, the build graph, deployed artifacts, runtime dependencies, or production behavior. Repo-local harness packs such as `.codex`, `.claude`, `.agents`, or `.speccy` policy/prose files are acceptable workflow artifacts when a team wants shared, versioned, editable agent lifecycle instructions. Operational run state, transcripts, raw evidence, screenshots, caches, and databases should remain external or ignored by default.
* **No outbound agent runner** - Speccy commands and subcommands should never call LLMs, coding agents, or AI harnesses. The active harness calls Speccy's deterministic controller tools; Speccy does not launch the harness.
* **Verification is judgment, not proof** - fresh-context judges decide whether written criteria are met; Speccy runs no deterministic checks of its own and says so on every ship card. Judges can miss criteria and defects; Speccy accepts that trade while the codebase remains small. Deterministic checks are added one at a time when dogfooding identifies a failure class judges miss.
* **Harness-neutral model language** - model strength and reasoning effort are named by the user in Speccy's config and mapped per harness to concrete models and effort values. Lifecycle agents reference the user's names; no pack, skill, or design document hardcodes a vendor model.

## **Design**

The idea is to split this into two layers, 1. a deterministic core layer, and 2. a higher-level prose layer (think agent harness skills and subagents). The goal is to capitalize on the idea of using modern AI models and agentic engineering to make its core offering as deterministic as possible, thereby reducing the surface area and potential for non-deterministic, drifted / hallucinated model output.
