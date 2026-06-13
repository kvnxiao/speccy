## Memory ledger entry shape

The repo's loop memory lives at `.speccy/MEMORY.md` — a user-owned,
git-tracked file, a sibling of `.speccy/BACKLOG.md`. `speccy init` never
enumerates or overwrites it, so a `--force` reeject leaves it byte-identical
and learned content survives speccy CLI updates. Its **absence is normal and
silent**: a missing or malformed ledger produces no `speccy verify` error or
warning, and the implementer simply has no slice to load.

### The four-part entry shape

Every entry — whether it records a convention the loop followed or a mistake it
made — carries the same four parts. Convention-flavoured and mistake-flavoured
entries differ only by which feed produced them, never in shape:

- **Trigger** — when the entry applies: a task area, a file region, or a
  recurring situation. This is what a future implementer matches against to
  decide the entry is relevant to the slice in front of them.
- **Convention or mistake** — the thing observed: the convention that was
  followed, or the mistake that was made.
- **Corrective rule** — the actionable instruction to follow next time, stated
  so the implementer can act on it without re-deriving the context.
- **Provenance** — the SPEC / task / review that produced the entry, named by
  real identifier so the entry is auditable back to its source.

### Authoring discipline

- **Prefer abstract, convention-level wording over fragile code coordinates.**
  An entry phrased as a durable convention survives a refactor that moves or
  renames the construct it came from; an entry pinned to a specific function,
  line, or module name becomes a phantom reference the moment that construct
  changes. Write the rule, not the address.

- **Provenance must resolve to a real SPEC / task / review identifier**, never
  a fabricated one. Dangling SPEC/task provenance is the only structurally
  checkable slice of ledger hygiene; the rest is a semantic judgment the
  ship-time retro owns, deliberately not a CLI freshness check. Keeping
  provenance honest at authoring time is what makes that future check possible.

### Worked example

The placeholders below are illustrative — substitute your own values.

```markdown
- Trigger: implementing a new CLI subcommand that parses a bounded numeric
  flag.
- Convention: bounded numeric flags are validated with a range value parser at
  the argument layer, not with an ad-hoc check inside the command body.
- Corrective rule: reach for the existing range-value-parser helper before
  writing a fresh bounds check; keep validation at the parse boundary.
- Provenance: SPEC-0042 / T-003 (0042-example-slug), reviewer-style pass.
```
