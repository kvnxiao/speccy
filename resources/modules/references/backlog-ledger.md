## Backlog ledger entry shape

The repo's future-spec register lives at `.speccy/BACKLOG.md` — a user-owned,
git-tracked file, sibling to `MEMORY.md` and distinct from it. `speccy init`,
`speccy init --force`, and reeject never create, enumerate, or overwrite it, so
learned content survives speccy CLI updates. Its **absence is normal and
silent**: a missing or malformed file produces no `speccy verify` error or
warning, and the CLI never reads it. The backlog is a flat, unordered list of
candidate specs — ideas worth their own SPEC later, not deferrals within a spec
already in flight.

### The file header

When the file self-creates on first append, the producing skill copies in this
preamble verbatim so the lifecycle stays legible to the next reader:

```markdown
# Speccy backlog — future-spec candidates

> User-owned, git-tracked, never created or overwritten by `speccy init`,
> `speccy init --force`, or reeject. Absence is normal and silent; the CLI
> never reads this file. Distinct from `MEMORY.md` (durable loop conventions)
> and from spec-local deferred surfaces (`## Non-goals`, deferred decisions,
> deferred coverage): each entry below should become its OWN spec. Promotion
> retires an entry by deletion. See
> `resources/modules/references/backlog-ledger.md` for the entry shape.
```

### The four-field entry shape

Every entry carries the same four fields, one line per field:

- **Title** — the prospective spec named in a phrase.
- **What & why** — what the spec would deliver plus the value it carries: the
  case for building it.
- **Deferred-because** — why it is not being built now: out of the current
  slice, needs infrastructure that does not exist yet, or blocked on some
  named prerequisite.
- **Provenance** — the originating spec and phase that surfaced the candidate,
  e.g. `SPEC-NNNN, ship` or `SPEC-NNNN, plan`, or `manual` for a hand-added
  entry.

### Authoring discipline

- **Terse.** One phrase per field. The backlog is a working list scanned at
  plan time, not a design document; a candidate that needs a paragraph to
  justify wants its own brainstorm, not a longer backlog line.

- **Provenance must resolve to a real spec and phase**, never a fabricated one
  — or `manual` when added by hand. Honest provenance is what lets a reader
  trace a candidate back to the moment it surfaced.

- **Promotion strikes the entry by deletion.** When a candidate becomes its own
  SPEC, delete its line; the promotion trail lives in git history and the new
  SPEC's own provenance. The backlog reads as current candidates only, never a
  tombstone field.

- **Many entries from one spec's loop is a focus smell.** The per-spec add rate
  is itself feedback: a single spec spawning a long tail of backlog entries
  signals the slice was drawn too wide or the work kept discovering adjacent
  scope. This is a signal to weigh, not an enforced threshold — nothing gates
  on it.

### Worked example

The placeholders below are illustrative — substitute your own values.

```markdown
- Title: Cross-repo spec linking.
- What & why: let a SPEC in one repo reference requirements in another so a
  shared contract has one source of truth; removes the copy-paste drift between
  the two repos that share the protocol.
- Deferred-because: needs a cross-repo resolution surface that does not exist
  yet — out of the current single-repo slice.
- Provenance: SPEC-0042, ship.
```
