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

- **No provenance or doc-pointer meta-annotation.** Production code,
  tests, and comments must not cite, as the reason a line exists, a Speccy
  id (SPEC/REQ/CHK/DEC/task — `// per REQ-NNN`, `//! Tests for SPEC-NNNN
  T-NNN`) or a governance/design doc (`(Core principle 2)`, `per
  AGENTS.md`, `see docs/ARCHITECTURE.md`, a rule-file pointer). Both
  reference something outside the code that means nothing once it stands
  alone — drift the moment it lands. Requirement→evidence traceability
  lives in the journal `Evidence:` field and CHK roll-call, not the source
  tree. Keep the reasoning a comment carries; drop the bare id or doc
  pointer. Naming an artifact the code operates on (`SPEC.md`, a
  `.speccy/…` path) is data, not meta-annotation — that stays.

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
