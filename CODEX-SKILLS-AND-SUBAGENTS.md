# Codex Skills + Subagents: Full Technical Summary (2026)

## Executive Summary

Yes — modern OpenAI Codex supports **subagents**, and those subagents can be orchestrated from within workflows implemented as **Skills**.

However, there is an important architectural distinction:

| Capability    | Purpose                                                |
| ------------- | ------------------------------------------------------ |
| Skills        | Reusable workflows, instructions, scripts, conventions |
| Subagents     | Parallel delegated execution contexts                  |
| Custom Agents | Specialized worker identities/configurations           |
| MCP           | External systems/tool integrations                     |

A **Skill is not itself a subagent**, and there is currently no dedicated “spawn_subagent()” primitive inside the Skill format. Instead:

* Skills can instruct Codex to delegate
* Codex can then spawn specialized subagents
* Subagents may operate in parallel
* Custom agents can define models/personas/tool access
* Skills + MCP + Subagents are intended to compose together

The ecosystem is evolving quickly, but OpenAI documentation and community implementations now clearly support this pattern.

---

# Official OpenAI Architecture

## 1. Skills

OpenAI describes Skills as reusable workflow packages containing:

* instructions
* resources
* optional scripts
* conventions
* task automation logic

Typical structure:

```text
.agents/
  skills/
    my-skill/
      SKILL.md
      scripts/
```

OpenAI documentation states that Codex:

* initially loads only skill metadata
* lazily loads SKILL.md when invoked
* can invoke skills explicitly or implicitly

### Official references

* [https://developers.openai.com/codex/skills](https://developers.openai.com/codex/skills)
* [https://agentskills.org](https://agentskills.org)

---

## 2. Subagents

OpenAI introduced native subagents in early 2026.

Subagents are:

* isolated execution contexts
* specialized delegated workers
* parallelizable
* context-window preserving
* useful for large repositories and long-running workflows

OpenAI explicitly describes them as a mechanism for reducing:

* context pollution
* context rot
* noisy intermediate reasoning

### Official references

* [https://developers.openai.com/codex/subagents](https://developers.openai.com/codex/subagents)
* [https://developers.openai.com/codex/concepts/subagents](https://developers.openai.com/codex/concepts/subagents)

---

# The Key Question: Can Skills Spawn Subagents?

## Short Answer

### Practically: YES

### Architecturally: INDIRECTLY

A Skill can:

* instruct Codex to use subagents
* coordinate delegation
* define workflows that require subagents
* orchestrate multi-agent execution

But:

* Skills themselves are not autonomous agent containers
* Subagents are a separate Codex runtime capability

This is the current mental model:

```text
Skill
  -> tells Codex HOW to orchestrate work
  -> Codex runtime decides HOW to spawn/delegate subagents
```

---

# How People Actually Implement It

The dominant community pattern is:

## Skill-as-Orchestrator

Example:

```md
# SKILL.md

When invoked:

- spawn reviewer subagent
- spawn implementation subagent
- spawn testing subagent

Delegate tasks in parallel.
Merge only validated outputs.
```

Codex then interprets the instructions and launches subagents accordingly.

This has become a common workflow style in:

* large repos
* enterprise automation
* long-running coding tasks
* research/refactor workflows

---

# Recommended Modern Codex Architecture

OpenAI documentation now effectively recommends this progression:

## Layer 1 — AGENTS.md

Repository-wide conventions.

```text
AGENTS.md
```

Examples:

* code style
* testing policy
* review rules
* architecture standards

---

## Layer 2 — Skills

Reusable workflows.

Examples:

* release workflow
* migration workflow
* debugging workflow
* PR review workflow

```text
.agents/skills/
```

---

## Layer 3 — MCP

External integrations.

Examples:

* GitHub
* Linear
* Jira
* Figma
* internal docs

---

## Layer 4 — Subagents

Parallel decomposition and specialization.

Examples:

* reviewer
* researcher
* refactorer
* test writer
* dependency auditor

---

# Custom Agents vs Skills

This distinction is important.

| Feature                    | Skill    | Custom Agent |
| -------------------------- | -------- | ------------ |
| Workflow instructions      | Yes      | Optional     |
| Independent context window | No       | Yes          |
| Dedicated model selection  | No       | Yes          |
| Parallel execution         | Indirect | Yes          |
| Persona/specialization     | Limited  | Strong       |
| Tool sandboxing            | Indirect | Yes          |

Community discussions consistently point out:

> If you need specialized runtime behavior, create agents — not just skills.

---

# Typical File Layout

Modern Codex projects commonly look like this:

```text
.codex/
  agents/
    reviewer.toml
    researcher.toml
    tester.toml

.agents/
  skills/
    pr-review/
      SKILL.md

AGENTS.md
```

---

# Example Workflow

## User Request

```text
Review and refactor the authentication system.
```

## Skill Invoked

```text
auth-review skill
```

## Skill Instructions

```text
1. Spawn architecture-review subagent
2. Spawn security-audit subagent
3. Spawn test-analysis subagent
4. Merge findings
5. Produce implementation plan
```

## Runtime Behavior

Codex then:

* creates subagent contexts
* parallelizes exploration
* isolates noisy logs
* returns summarized results to parent context

This is effectively how advanced Codex orchestration now works.

---

# Why Subagents Matter

Subagents solve a major scaling problem for AI coding systems.

Without subagents:

```text
Single giant context
-> noisy logs
-> token bloat
-> degraded reasoning
-> architectural drift
```

With subagents:

```text
Parallel isolated workers
-> cleaner parent context
-> bounded reasoning scopes
-> improved long-task reliability
```

OpenAI explicitly calls out:

* context pollution
* context rot

as core motivations for subagents.

---

# Current Limitations

## 1. Sparse Official Documentation

OpenAI documentation is improving, but community consensus is that:

* many orchestration details remain undocumented
* best practices are still emerging
* CLI behavior changes rapidly

---

## 2. Skills Are Mostly Instructional

Today’s Skills are usually:

* markdown workflows
* prompts
* scripts
* conventions

not fully autonomous orchestration programs.

---

## 3. Coordination Problems

Parallel subagents can:

* overlap edits
* conflict on files
* duplicate reasoning
* diverge architecturally

---

# Emerging Best Practices

## Good Uses for Subagents

### Excellent fits

* codebase exploration
* parallel audits
* large refactors
* dependency analysis
* test generation
* documentation generation
* security review

### Poor fits

* tiny edits
* single-file fixes
* tightly coupled modifications

---

# Recommended Agent Topologies

## Fan-Out Review Pattern

```text
Parent agent
  ├── reviewer
  ├── tester
  ├── architect
  └── researcher
```

---

## Pipeline Pattern

```text
researcher
  -> planner
    -> implementer
      -> reviewer
```

---

# Model Selection Trends

OpenAI and community discussions increasingly recommend:

| Model Type            | Best For              |
| --------------------- | --------------------- |
| Large reasoning model | Parent orchestration  |
| Mini model            | Lightweight subagents |
| Specialized agents    | Narrow workflows      |

Community reports indicate GPT-5.4 mini is optimized specifically for subagent workloads.

---

# Enterprise Direction

OpenAI’s newer “workspace agents” initiative strongly suggests the long-term direction is:

```text
Teams
  -> shared agents
    -> orchestrated subagents
      -> persistent workflows
```

This appears to be the future architecture of enterprise Codex systems.

---

# Key Takeaways

## What is definitely true

* Codex supports subagents
* Skills exist as reusable workflows
* Skills and subagents are designed to work together
* Skills can orchestrate subagent delegation
* Custom agents provide specialized worker behavior
* MCP integrates external systems

---

## What is NOT currently true

* Skills are not themselves subagents
* There is no dedicated “spawn_subagent()” API in SKILL.md
* Skills are not full orchestration runtimes

---

# Most Accurate One-Sentence Summary

> In modern Codex, Skills define reusable workflows, while subagents provide delegated parallel execution; Skills can orchestrate subagents indirectly through runtime instructions and custom-agent delegation patterns.

---

# Primary Resources

## Official OpenAI

* [https://developers.openai.com/codex/overview](https://developers.openai.com/codex/overview)
* [https://developers.openai.com/codex/skills](https://developers.openai.com/codex/skills)
* [https://developers.openai.com/codex/subagents](https://developers.openai.com/codex/subagents)
* [https://developers.openai.com/codex/concepts/subagents](https://developers.openai.com/codex/concepts/subagents)
* [https://developers.openai.com/codex/mcp](https://developers.openai.com/codex/mcp)
* [https://developers.openai.com/codex/guides/agents-md](https://developers.openai.com/codex/guides/agents-md)
* [https://developers.openai.com/codex/plugins](https://developers.openai.com/codex/plugins)

## OpenAI Articles

* [https://openai.com/index/unrolling-the-codex-agent-loop/](https://openai.com/index/unrolling-the-codex-agent-loop/)
* [https://openai.com/index/introducing-workspace-agents-in-chatgpt/](https://openai.com/index/introducing-workspace-agents-in-chatgpt/)
* [https://openai.com/index/introducing-the-codex-app/](https://openai.com/index/introducing-the-codex-app/)

## Community + Research

* [https://spybara.com/openai/codex/history/docs/en/changes/](https://spybara.com/openai/codex/history/docs/en/changes/)
* [https://github.com/lemos999/Codex-Subagent-Orchestrator](https://github.com/lemos999/Codex-Subagent-Orchestrator)
* [https://agentskills.org](https://agentskills.org)
* [https://arxiv.org/abs/2602.14690](https://arxiv.org/abs/2602.14690)
