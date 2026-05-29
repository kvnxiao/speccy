## Sourcing your recorded identity

When you record your own identity in a `model="..."` attribute, build
the value from two independently sourced parts: the model segment and
the optional effort suffix. Do not infer either from the skill-pack
name, the persona name, or an inherited environment variable.

- **Model segment — from the host's in-context identifier, verbatim.**
  Use the resolved long-form model identifier your host states
  in-context (for example, a host line such as
  `The exact model ID is claude-opus-4-8[1m]`). Transcribe it exactly,
  preserving version punctuation as the host writes it — keep the
  hyphen form (`claude-opus-4-8`), never normalise it to a dotted form
  (`claude-opus-4.8`), and never substitute a configured alias. Where a
  host states no resolved identifier in-context, fall back to the
  `model:` value in your own agent definition file.

- **Effort suffix — from your own definition file.** When your host
  exposes a reasoning-effort knob, read the effort from your own
  sub-agent definition file (`effort:` on Claude Code,
  `model_reasoning_effort` on Codex) and append it as a slash-suffix
  (e.g. `claude-opus-4-8[1m]/low`). Never derive the effort from
  `CLAUDE_EFFORT` or any other inherited environment variable: a
  sub-agent pinned to a low effort that is dispatched from a
  higher-effort parent session still records its own definition-file
  effort. A host with no effort knob omits the suffix entirely.

- **Override limitation.** The `CLAUDE_CODE_EFFORT_LEVEL` runtime
  override is deliberately not read. A run that sets it still records
  the effort declared in the agent definition file, not the override
  value.
