# migrate-tasks-schema

> SPEC-0029 (private, one-shot) — convert TASKS.md from the legacy
> markdown-bullet authoring conventions to the new XML schema.

This binary is **not** a `speccy` subcommand and does not surface in
`speccy --help`. It lives under `speccy-core/tools/` because it is a
private migration utility that exists only to run once against the
in-tree corpus during the SPEC-0029 ship (T-003). After T-003 lands,
the binary is dead code that a follow-on SPEC may delete (SPEC-0029
DEC-005).

## What it does

For each `<task>` body inside the input TASKS.md, the tool rewrites
the three legacy authoring conventions to their XML element form:

| Legacy                                       | New XML form                                                         |
| -------------------------------------------- | -------------------------------------------------------------------- |
| `- Implementer note (session-X):` + sub-bullets | `<implementer-note session="X">…</implementer-note>`                |
| `- Review (<persona>, <verdict>): <prose>`   | `<review persona="<persona>" verdict="<verdict>">…</review>`         |
| `- Retry: <prose>`                           | `<retry>…</retry>`                                                   |

The optional `, retry` annotation on legacy `- Review (persona,
verdict, retry):` lines is dropped during migration — the new schema
attributes retries by source position (SPEC-0029 DEC-008).

All other bytes (frontmatter, phase headings, free prose,
`<task-scenarios>` bodies, `Suggested files:` bullets,
`spec_hash_at_generation` value) are preserved verbatim. The hash is
over SPEC.md, not TASKS.md, so the migration is hash-neutral by
construction.

## Properties

- **Idempotent.** Re-running the tool against an already-migrated
  TASKS.md is a no-op (zero file modifications, exit code 0). The
  transitional state machine accepts both forms; on the second run
  there are no legacy bullets to convert, so the produced bytes
  equal the input bytes and nothing is written.
- **Hash-neutral.** `spec_hash_at_generation` (SPEC-0024) is computed
  over SPEC.md bytes; TASKS.md content changes do not flow into the
  hash. The migration touches only TASKS.md.
- **Parse-verified.** Before writing, the migrated output is
  re-parsed under `speccy_core::parse::parse_task_xml`. If the
  re-parse fails, the tool exits non-zero and leaves the file
  untouched.

## Invocation

```sh
cargo run -p migrate-tasks-schema -- <PATH>...
```

For the in-tree corpus run (T-003):

```sh
cargo run -p migrate-tasks-schema -- .speccy/specs/*/TASKS.md
```

Each path produces one line of output: `<path>: migrated` or
`<path>: no change`.

## When NOT to use this

This tool is for the **one-shot in-tree migration**. After T-003
lands, every TASKS.md under `.speccy/specs/` is in the new schema and
the shipped writer-side skill prompts (T-005) emit the new XML
elements directly. There is no ongoing need for the tool. It is not
exposed via `speccy --help`; downstream consumers writing TASKS.md
from scratch should follow the writer-side skill conventions, not
invoke this migration.
