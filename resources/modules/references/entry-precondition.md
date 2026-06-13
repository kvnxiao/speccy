**Clean-tree gate.** With the target task resolved and its shape classified by
the retry-shape rule, run `git status --porcelain` and branch:

- **First-attempt shape + non-empty stdout** → stop without dispatching the
  implementer and surface the dirty paths on stderr. The gate keeps a turn from
  starting on top of unrelated working-tree changes.
- **First-attempt shape + empty stdout** → proceed.
- **Retry shape** → proceed regardless of stdout. The dirty paths are the prior
  pass's WIP that the retry implementer amends in place; no dirty-paths surface
  is written.

An orchestrator runs this gate before spawning the worker; the worker re-runs it
defensively at its own entry.
