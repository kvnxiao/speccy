## Convention-drift checklist

Re-read your own diff against the existing codebase and the project's
own conventions before handing off. These are the recurring categories
where mechanical and convention drift slips through a green hygiene
gate yet still costs a later review round. Catching them here — in the
diff you already have open — is far cheaper than a bounce-and-respawn.

- **Match local conventions.** Make the diff read as though the
  surrounding code's author wrote it: follow the established naming,
  error-handling, and import-ordering patterns of the files you touch.
  If the neighbouring code propagates errors one way and yours does
  another, or your imports fight the project's formatter, align with
  what is already there.

- **Docs match code.** Any comment, docstring, or documentation you
  add or touch must describe what the code actually does. Stale or
  aspirational prose that no longer matches the behaviour is drift.

- **No Speccy-id provenance in code.** Production code, tests, and
  comments must not cite a Speccy id (SPEC/REQ/CHK/DEC/task) as the reason
  a line exists — `// per REQ-NNN`, `//! Tests for SPEC-NNNN T-NNN`,
  `// As per DEC-NNN`. Speccy is a means to produce the code, not a part
  of it; once Speccy is removed the citation references nothing, so it is
  drift the moment it lands. Requirement→evidence traceability already
  lives in the journal `Evidence:` field and CHK roll-call, not in the
  source tree. Keep the reasoning a comment carries; drop only the bare id.

- **No false complexity.** Do not add abstraction, indirection, or
  configurability the change does not require. In particular, do not
  split a function into pieces that push the file past its own
  existing complexity ceiling — keep the shape consistent with how the
  rest of the file is structured.

- **Re-apply the project's own hard rules.** Whatever invariants the
  project's conventions declare, hold your diff to them. Two recurring
  traps:
  - **No vacuous or constant-copy tests.** A test must gate a real
    invariant. A test that re-asserts a hard-coded copy of a
    production constant, or only checks that something exists or is
    non-empty, cannot fail in any interesting way — derive a real
    property or drop it.
  - **Suppressions carry a justification.** Every lint or warning
    suppression you add must state why it is there, never a bare
    silencer.
