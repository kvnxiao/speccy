# Speccy: Implement `{{task_id}}` for `{{spec_id}}`

You are implementing one task in one spec. Work surgically; do not
touch unrelated files. When done, flip `{{task_id}}`'s `state="..."`
attribute from `pending` / `in-progress` to `in-review` in TASKS.md
and append an `<implementer-note session="...">…</implementer-note>`
element block inline using the handoff template below.

## SPEC (pointer)

Before starting, read SPEC.md at `{{spec_md_path}}`. The CLI no
longer inlines the SPEC body into this prompt; load it via your Read
primitive when you need it.

## Task entry (verbatim from TASKS.md)

The block below is the literal `<task id="{{task_id}}">...</task>`
element copied from TASKS.md. The required nested `<task-scenarios>`
block names the slice-level validation contract for this task; the
SPEC requirements named in `covers="..."` carry the user-facing
`<scenario>` elements you must satisfy.

{{task_entry}}

## Suggested files

{{suggested_files}}

## When you hit friction

You will sometimes discover that a skill file ships the wrong
instruction for this project: the prompt told you to run `npm test`
but the project uses `pnpm`; the persona pointed at `tests/` but the
suite lives under `__tests__/`; a required environment variable is
not documented anywhere. When that happens, **update the relevant
skill file under `skills/`** (or the in-project override under
`.speccy/skills/`) so the next implementer inherits the fix.

Do this **before** flipping `state="pending"` / `state="in-progress"`
to `state="in-review"`. Then name the file you touched under
`Procedural compliance` in your handoff note so the reviewer and
REPORT.md can pick the change up. Do not silently work around
friction; the next worker will hit it too.

Worked example: the implementer prompt told you to run `npm test`,
but `package.json` declares `"packageManager": "pnpm@9"`. Edit the
prompt, then record:

```markdown
- Procedural compliance: edited `skills/shared/prompts/implementer.md`
  to reference `pnpm` instead of `npm` after hitting the friction in
  T-003.
```

## Your task

Walk the red-green workflow in execution order. The numbered sequence
is the natural path: writing the failing check first, capturing its
output, then making it pass — so the evidence file is a byproduct of
the work, not a separate ceremony.

1. Read the SPEC requirements the task covers (the `covers="..."`
   attribute on the `<task>` element above lists them as
   space-separated `REQ-NNN` ids). For each covered requirement, the
   `<behavior>` element drives your test selection and the
   `<done-when>` element drives your acceptance criteria. The
   requirement's nested `<scenario>` elements are the
   **user-facing-level** validation contract.
2. Read the `<task-scenarios>` body on this task and every prior
   bullet under the task entry — implementer notes from past
   attempts, review feedback, retry annotations. The
   `<task-scenarios>` block is the **slice-level** validation
   contract: the set of executable conditions this single slice of
   work must satisfy. Slice-level scenarios are typically narrower
   than the user-facing requirement scenarios and are written
   specifically for this task.
3. Translate each `<task-scenarios>` bullet, together with the
   covered requirements' `<behavior>` Given/When/Then prose, into an
   executable test in the project's framework — or, for a slice
   without a unit-test runner (a doc edit, a prompt-template tweak,
   a config change), a scoped verification command (`grep`,
   `test -f`, a project-build invocation, etc.). **Write the test or
   verification command before writing implementation code.**
4. Run the failing test / verification command and capture the
   verbatim output. Create
   `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md` if it does not
   exist yet (where `<SPEC-folder>` is this spec's `NNNN-slug`
   folder name and `<TASK>` is this task's id, e.g.
   `.speccy/specs/0031-red-green-paper-trail/evidence/T-004.md`).
   Append a new `## Session <session-id> (attempt N)` block to the
   file and write the captured output inside a `<red exit="N">`
   element under that header. A build / compile failure caused by
   writing a test against a missing symbol is a legitimate red phase
   — output reading `cannot find function`, a `build error`, or any
   compile-time diagnostic counts as red; you do not need to scaffold
   a stub just to produce a runtime failure.
5. Implement the code path so the failing test / verification command
   passes and every bullet in the covered requirements' `<done-when>`
   is satisfied. Use `speccy check SPEC-NNNN/T-NNN` to re-read the
   `<scenario>` elements you are satisfying; it renders them, it
   does not run them.
6. Run the test / verification command again and capture the verbatim
   output. Append it under a `<green exit="0">` element inside the
   same `## Session` block opened in step 4. The evidence file's
   session block now carries both halves of the red→green
   transition.
7. Run the project's deterministic hygiene gates (the exact set is
   documented under "Standard hygiene" in this project's
   AGENTS.md — typically lint, format, build, full-suite test, and
   dep audit). Record each gate's command and its exit code; you
   will paste them into the `Hygiene checks` table in step 8.
8. Append one `<implementer-note session="...">…</implementer-note>`
   element block to the task subtree using the six-field handoff template
   shown below. The `session` attribute is required and non-empty; the
   body is required and non-empty. Every field must appear; write
   `(none)` for empty fields rather than omitting them. The `Evidence`
   field references the file you wrote in steps 4 and 6.
9. Flip the task's `state="..."` attribute to `in-review` to signal
   "awaiting review".

The evidence file is **append-only**: a new session block is added
at the end of the file on every implementer session; prior session
blocks are never edited or removed. If your session does not change
any tests (for example, a comment-only cleanup or a doc tweak
responding to a blocking review), substitute steps 3–6 with a
no-test-delta block whose header reads
`## Session <session-id> (attempt N, no test delta)` followed by a
single sentence describing what the session did instead — no
red/green pair.

A minimal sketch of the red+green session block:

```markdown
## Session <session-id> (attempt N)
Command: `<scoped command>`
<red exit="N">…</red>
<green exit="0">…</green>
```

For the full worked example covering both a red+green session and a
no-test-delta retry session, see `.speccy/examples/evidence.md` via
your Read primitive on first encounter.

### Handoff template

Append exactly this element shape, replacing each `<...>` with
content. Keep the field labels inside the body verbatim so
downstream tooling (review prompts, REPORT.md, any harness reading
TASKS.md) can grep them. Every field is required; write `(none)`
for an empty field rather than omitting the line.

```markdown
<implementer-note session="session-abc">
- Completed: <what shipped in this task>
- Undone: <what was planned but deferred, and why>
- Hygiene checks:

  | Command                                                                | Status        |
  |------------------------------------------------------------------------|---------------|
  | `cargo test --workspace`                                               | pass (exit 0) |
  | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass (exit 0) |
  | `cargo +nightly fmt --all --check`                                     | pass (exit 0) |
  | `cargo deny check`                                                     | pass (exit 0) |

- Evidence: `.speccy/specs/<SPEC-folder>/evidence/<TASK>.md` — red: `<scoped command>` → exit N / green: `<scoped command>` → exit 0
- Discovered issues: <bugs or surprises in adjacent code; (none) is fine>
- Procedural compliance: <skill files touched and why; (none) if no friction>
</implementer-note>
```

The `Hygiene checks` body is a markdown table with exactly two
columns — `Command` and `Status` — where each `Status` cell
renders as `pass (exit 0)` or `fail (exit N)`. The table replaces
the prior parallel `Commands run` / `Exit codes` field pair: the
table form keeps each command bound to its outcome on the same
row, removing the positional-pairing risk.

The `Evidence` body is one line: the project-relative path to the
per-task evidence file, then a ` — ` delimiter, then a one-line
red→green summary naming the scoped command and its red and green
exit codes. Substantially equivalent prose is acceptable so long as
the path and the red→green summary both appear.

Do not modify SPEC.md. Surgical changes only.
