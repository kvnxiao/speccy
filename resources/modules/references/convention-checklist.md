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
  tests, and comments must not cite, as the reason a line exists,
  something outside the code — a planning artifact, a project rule, or a
  design doc — because the citation means nothing once the line stands
  alone, and so it is drift the moment it lands. The leak is not just the
  `// per X` form; it spans at least four shapes, and the bare-id form is
  the rarest of them:
  - **Speccy-id citation** — a SPEC/REQ/CHK/DEC/task id named as the reason
    (`// per REQ-NNN`, `//! Tests for SPEC-NNNN T-NNN`).
  - **Descriptive prose pointing at a planning artifact** — natural-language
    that names the SPEC or a future/other spec as the reason, with no
    `// per` framing to flag it (`// every failure mode the spec defines`,
    `// later specs populate this`, `// a later spec can ask for X`). This
    is the most common leaked shape and the easiest to wave through.
  - **Numbered project-rule citation** — a pointer to a numbered rule or
    principle (`(Core principle 2)`, `// cardinal rule #4`, `per AGENTS.md`).
  - **Doc-path citation** — a pointer to a governance/design document or a
    rule file (`see docs/ARCHITECTURE.md`, `(docs/implementation)`, a
    rule-file pointer).

  Requirement→evidence traceability lives in the journal `Evidence:` field
  and CHK roll-call, not the source tree. Keep the reasoning a comment
  conveys; drop the bare pointer. Naming an artifact the code operates on
  (`SPEC.md`, a `.speccy/…` path) is data the code reads or writes, not
  provenance — that stays.

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
