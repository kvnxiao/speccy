---
spec: SPEC-0017
outcome: delivered
generated_at: 2026-05-14T23:55:00Z
---

# SPEC-0017: `speccy check [SELECTOR]` polymorphic dispatch

## Outcome

**delivered** — all five requirements satisfied; all 6 tasks closed
on the first pass with zero retries and zero SPEC amendments.
`speccy check` accepts five positional shapes (`SPEC-NNNN`,
`SPEC-NNNN/CHK-NNN`, `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`) plus the
no-arg `All` form. The new dispatch routes task-shaped selectors
through `speccy_core::task_lookup::find` for identical
`Ambiguous` / `NotFound` wording as `speccy implement`, scopes
qualified checks to a single spec while preserving SPEC-0010
DEC-003's bare-form cross-spec semantics, and delegates every
execution arm to the existing `execute_checks` path so live
streaming, IN-FLIGHT categorisation, and summary totals are reused
verbatim.

## Requirements coverage

| Requirement | Checks | Result |
|-------------|--------|--------|
| REQ-001: Polymorphic selector parser | CHK-001 | proved |
| REQ-002: Spec-scoped execution (`SPEC-NNNN`) | CHK-002 | proved |
| REQ-003: Task-scoped execution (`T-NNN`, `SPEC-NNNN/T-NNN`) | CHK-003 | proved |
| REQ-004: Bare `CHK-NNN` preserved; qualified check scoped | CHK-004 | proved |
| REQ-005: Documentation surface updated | CHK-005 | proved (manual) |

CHK-001..CHK-004 resolve to executable `cargo test` invocations
(`--test check_selector -- parser`, `--test check -- spec_selector`,
`--test check -- task_selector`, `--test check -- bare_chk_preserved`).
CHK-005 is the documentation-surface manual check from the SPEC's
own design.

## Task summary

- **Total tasks:** 6 (T-001 through T-006).
- **Retried tasks:** 0.
- **SPEC amendments:** 0.

T-001 introduced the `check_selector` module with the parser plus
10 unit + 10 integration tests, dispatching qualified-task →
qualified-check → bare-spec → unqualified-task → unqualified-check.
T-002 wired the parser into `check::run`, renamed `CheckArgs.id` to
`CheckArgs.selector`, retired `validate_chk_id_format` +
`CheckError::InvalidCheckIdFormat`, and added two `assert_cmd`-driven
binary tests that lock in the five-shape hint output and the
verbatim `NoCheckMatching` wording. T-003..T-005 each implemented
one selector arm (`Spec`, `QualifiedCheck`, `Task`) with the matching
integration tests; T-005 wired
`CheckError::TaskLookup(#[from] LookupError)` so `task_lookup`'s
existing error wording reaches stderr byte-for-byte. T-006 updated
`.speccy/ARCHITECTURE.md` and audited shipped skill docs for the
new selector surface.

## Out-of-scope items absorbed

Edits made during the loop that were not part of the planned task
scope but were necessary for the work to land cleanly:

- **`SelectorError::TaskCoversNothing` variant removed during
  T-005** — the variant was introduced as a placeholder by T-002
  so the matching matrix stayed exhaustive without a `todo!()`.
  T-005's empty-covers decision landed on option (b) (print an
  informational line and return `Ok(0)`, *not* synthesise an
  error). With that decision locked in, the placeholder variant
  was dead code we had just created. AGENTS.md's "Surgical
  changes" rule keeps pre-existing dead code but says
  just-created dead code should not survive — so the variant
  was removed in the same task. Documented in T-005's
  implementer note.
- **Historical task-title amendment in
  `.speccy/specs/0010-check-command/TASKS.md:88`** — T-006's
  acceptance criterion was `git grep -n "speccy check \[CHK-ID\]"`
  → zero hits anywhere in the repo. One historical task title
  in the SPEC-0010 task log used the old `[CHK-ID]` shape and
  blocked the absolute zero. Edited to `[SELECTOR]`; SPEC-0010
  T-008 stays `[x]` (the wording is descriptive of what got
  wired, which is now the SPEC-0017 selector dispatcher).
- **`.speccy/skills/personas/implementer.md` +
  `resources/modules/personas/implementer.md` paired edit
  (T-006)** — fixing the stale `speccy check SPEC-NNNN T-002`
  example (a space instead of a slash) in only one of the two
  byte-identical files would either re-leak the stale example
  on every downstream `speccy init` (if only `.speccy/skills/`
  was fixed) or leave this repo's own dogfooding loop reading
  the stale form (if only `resources/modules/` was fixed). T-006
  fixed both.

## Skill updates

(none)

T-006 edits `.speccy/skills/personas/implementer.md` and
`resources/modules/personas/implementer.md` to fix a stale
`speccy check` example, but that is the *planned* scope of T-006
under REQ-005 (documentation surface), not a friction edit
surfaced during an unrelated task. Every `Procedural compliance`
line across T-001..T-006 reads `(none)`.

## Deferred / known limitations

- **Windows `cargo`-self-build file-lock race during
  self-dogfooding** — T-003 / T-004 / T-005 each ran the new
  selector against SPEC-0017's own spec.toml at the end of the
  task. Each CHK-NNN's `cargo test` invocation collides with the
  still-running `cargo build` of `speccy.exe` on the same
  `target/debug/` directory, producing spurious exit-101
  IN-FLIGHT results on Windows. This is the same workspace race
  documented in SPEC-0014 T-007 and SPEC-0016 T-008's implementer
  notes; it predates SPEC-0017 and is not a selector defect. The
  workaround is to run `./target/release/speccy.exe verify`
  rather than `cargo run -p speccy-cli -- verify`. A follow-up
  spec could either codify the Windows guidance in the
  contributor docs or change the verify driver to deferred-exec
  the running binary; out of scope for v1.
- **Display-side echo of control bytes in
  `SelectorError::InvalidFormat`** — the security reviewer noted
  that `speccy check $'\x1b]0;EVIL\x07'` would echo control
  bytes verbatim to stderr because the parser preserves the
  offending input without truncation, case folding, or
  whitespace stripping per DEC-004. The threat model (attacker
  already controls the shell invocation) makes this a non-
  blocking trade for local-CLI usability. Documented in T-001's
  security-review note; revisit if speccy ever grows a
  non-interactive surface that takes selectors from less-trusted
  callers.
- **Stale section-divider comments in
  `speccy-cli/tests/skill_packs.rs:1215-1224`** — inherited from
  SPEC-0016 T-005/T-006 (mentioned in SPEC-0016's REPORT.md
  under the same heading). Not touched by SPEC-0017 because the
  divider comments are inside test scaffolding unrelated to the
  selector surface. Will surface naturally on the next edit to
  that section.
