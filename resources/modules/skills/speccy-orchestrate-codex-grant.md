
## Codex sub-agent-spawn permission grant

Codex requires an **explicit user grant** before any skill is allowed
to spawn sub-agents. Without the grant, the dispatch steps above
return a permission error instead of spawning `speccy-work`,
the four `reviewer-*` personas, or the `vet-*` leaf sub-agents,
and the outer loop cannot make progress.

### Granting the permission

On first invocation, Codex prompts the user once per session to
authorize sub-agent spawning for this skill. Approve the prompt to
proceed; the grant is scoped to the current Codex session by default.

To persist the grant across sessions for this skill, add the
following to your Codex project configuration (typically
`.codex/config.toml`):

```toml
[skills.speccy-orchestrate]
allow_subagent_spawn = true
```

With the entry in place, the orchestrator dispatches sub-agents
without an interactive prompt on every session.

### Revoking the permission

Remove the `allow_subagent_spawn` entry from `.codex/config.toml` (or
flip it to `false`), then restart Codex. The next invocation of
`speccy-orchestrate` will prompt for the grant again.

### Why this exists

Sub-agent spawn is a privileged operation: each spawn launches a
delegated execution context with its own model, tool surface, and
working-tree access. Requiring an explicit grant keeps the
sub-agent boundary visible to the user — a skill cannot silently
fan out unbounded delegated work.

This grant is **Codex-specific**. Claude Code's `Task` tool does not
require an equivalent permission step — Claude approves tool calls
through the standard tool-use confirmation flow rather than a
skill-level permission gate.

For background on Codex's sub-agent model, see
`developers.openai.com/codex/concepts/subagents`.
