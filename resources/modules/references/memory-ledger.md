## Memory ledger entry shape

The repo's loop memory lives at `.speccy/MEMORY.md` — a user-owned,
git-tracked file. `speccy init` never
enumerates or overwrites it, so a `--force` reeject leaves it byte-identical
and learned content survives speccy CLI updates. Its **absence is normal and
silent**: a missing or malformed ledger produces no `speccy verify` error or
warning, and the implementer simply has no slice to load.

### What earns an entry

Record an entry only when the lesson is **durable across specs and not already
enforced** by an existing gate, reviewer persona, or `AGENTS.md`/rule. Recording
nothing is the default outcome: a lesson a future implementer would re-derive
anyway, or one a gate already catches, earns no line. The bar is high on purpose
— the ledger stays small by refusing low-signal intake, not by capping or
evicting.

### The one-line entry shape

Every entry is a **single line** carrying three parts and no narrative:

- **Trigger** — the situation a future implementer matches against to decide
  the entry is relevant to the slice in front of them: a task area, a file
  region, or a recurring situation.
- **Corrective rule** — the action to take next time, stated so the implementer
  can act on it without re-deriving the context.
- **Provenance tag** — a compact bracketed `[SPEC-NNNN/T-NNN]` tag naming the
  SPEC and task that produced the entry, so it is auditable back to its source.

There is no mistake or history field: how the lesson was learned is not forward
signal, only the corrective rule is.

### Authoring discipline

- **Prefer abstract, convention-level wording over fragile code coordinates.**
  An entry phrased as a durable convention survives a refactor that moves or
  renames the construct it came from; an entry pinned to a specific function,
  line, or module name becomes a phantom reference the moment that construct
  changes. Write the rule, not the address.

- **The provenance tag is bracketed and resolves to a real SPEC and task**,
  never a fabricated one. Use the `[SPEC-NNNN/T-NNN]` form; drop the task
  segment to `[SPEC-NNNN]` only for a spec-wide lesson that no single task
  owns. Dangling SPEC/task provenance is the only structurally checkable slice
  of ledger hygiene; the rest is a semantic judgment the ship-time retro owns,
  deliberately not a CLI freshness check. Keeping provenance honest at authoring
  time is what makes that future check possible.

### Worked example

The placeholders below are illustrative — substitute your own values.

```markdown
- Implementing a new CLI subcommand that parses a bounded numeric flag → reach for the existing range-value-parser helper before writing a fresh bounds check; keep validation at the parse boundary. [SPEC-0042/T-001]
```
