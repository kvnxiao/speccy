# Speccy: Implement `{{task_id}}` for `{{spec_id}}`

You are implementing one task in one spec. Work surgically; do not
touch unrelated files. When done, flip `{{task_id}}` from `[ ]` /
`[~]` to `[?]` in TASKS.md and append your implementer note inline
using the handoff template below.

## Project conventions

{{agents}}

## SPEC (full)

{{spec_md}}

## Task entry (verbatim from TASKS.md)

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

Do this **before** flipping `[~]` -> `[?]`. Then name the file you
touched under `Procedural compliance` in your handoff note so the
reviewer and REPORT.md can pick the change up. Do not silently work
around friction; the next worker will hit it too.

Worked example: the implementer prompt told you to run `npm test`,
but `package.json` declares `"packageManager": "pnpm@9"`. Edit the
prompt, then record:

```markdown
- Procedural compliance: edited `skills/shared/prompts/implementer.md`
  to reference `pnpm` instead of `npm` after hitting the friction in
  T-003.
```

## Your task

1. Read the SPEC requirements the task covers (`Covers: REQ-NNN`).
2. Read every prior bullet under the task entry: implementer notes
   from past attempts, review feedback, retry annotations.
3. Translate each `Tests to write:` bullet into an executable test
   in the project's framework. **Write the test before the code it
   exercises.**
4. Implement the code path so the tests pass. Run the project's
   own test command (`cargo test`, `pnpm test`, etc.) and fail fast
   on red. Use `speccy check SPEC-NNNN/T-NNN` to re-read the
   `speccy:scenario` marker blocks you are satisfying; it renders
   them, it does not run them.
5. Add one implementer note to the task subtree using the six-field
   handoff template shown below. Every field must appear; write
   `(none)` for empty fields rather than omitting them. Out-of-scope
   edits made for the test to compile belong under `Discovered issues`
   (peripheral bug) or `Procedural compliance` (skill-layer friction
   fix), with a one-line reason.
6. Flip the task checkbox to `[?]` to signal "awaiting review".

### Handoff template

Append exactly this shape, replacing each `<...>` with content. Keep
the field labels verbatim so downstream tooling (review prompts,
REPORT.md, any harness reading TASKS.md) can grep them.

```markdown
- Implementer note (session-abc):
  - Completed: <what shipped in this task>
  - Undone: <what was planned but deferred, and why>
  - Commands run: <one bullet per command run during the task>
  - Exit codes: <pass/fail per command above, in the same order>
  - Discovered issues: <bugs or surprises in adjacent code; (none) is fine>
  - Procedural compliance: <skill files touched and why; (none) if no friction>
```

Do not modify SPEC.md. Surgical changes only.
