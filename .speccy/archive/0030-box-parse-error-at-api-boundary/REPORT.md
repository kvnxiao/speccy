---
spec: SPEC-0030
outcome: delivered
generated_at: 2026-05-18T22:30:00Z
---

# Report: SPEC-0030 Box `ParseError` at every parser API boundary so `clippy::result_large_err` stops blocking the build

<report spec="SPEC-0030">

## Outcome

delivered

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
Every parser function inside `speccy-core/src/parse/*` and
`speccy-core/src/workspace.rs` now returns `ParseResult<T>`
(`Result<T, Box<ParseError>>`); `SpecXmlArtifacts.{tasks,report}`
(`speccy-core/src/workspace.rs:537,539`) and
`lint::types::ParsedSpec.{spec_md,spec_doc,tasks_md,report_md}`
(`speccy-core/src/lint/types.rs:148-161`) hold the boxed Err. The
`Vec<ParseError>` aggregation path on `validate_workspace_xml`
stays as DEC-003 specified. Verified locally:
`cargo clippy --workspace --all-targets --all-features -- -D warnings`
exits 0 with zero `result_large_err` diagnostics;
`grep -rE "-> Result<[^,]+, ParseError>" speccy-core/src/ speccy-cli/src/`
returns zero matches (the only surviving bare-`ParseError` mention
is the module-level docstring at `speccy-core/src/error.rs:3`,
explicitly carved out by REQ-001 done-when). The
`-Zprint-type-sizes` post-condition is satisfied for every public
signature; remaining unboxed `Err` entries belong to closure
desugarings and the DEC-003-blessed aggregation path, which the
lint and signature-grep contracts correctly exclude. Project tests
that satisfy CHK-001 in spirit: `cargo test --workspace` (660
tests, baseline preserved).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
`speccy_core::tasks::CommitError`'s `impl From<ParseError>` was
flipped to `impl From<Box<ParseError>>`
(`speccy-core/src/tasks.rs:80`); every downstream `match` arm in
`speccy-core/src/lint/rules/{spc,tsk}.rs` destructures through
`err.as_ref()`; CLI error enums in `speccy-cli/src/{plan,status,tasks,report}.rs`
absorb `Box<ParseError>` via `#[from]` and the redundant
`Box::new(source)` re-wraps were removed. Integration test
suites that previously matched `Err(ParseError::Variant { .. })`
(`speccy-core/tests/workspace_loader.rs`,
`speccy-core/tests/task_xml_body_items.rs`,
`speccy-core/src/parse/spec_md.rs` unit tests,
`speccy-core/src/parse/frontmatter.rs` unit tests) now thread the
box and keep their inner variant + field-value assertions
intact; `format!("{err}")` substring assertions pass through
`Box<ParseError>`'s blanket `Display` impl unchanged.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
`pub type ParseResult<T> = std::result::Result<T, Box<ParseError>>`
declared at `speccy-core/src/error.rs:597-600` with the SPEC-0030
docstring naming `clippy::result_large_err`; re-exported from the
crate root at `speccy-core/src/lib.rs:14-15`. Doc page renders
mechanically from a plain `pub type` with a doc comment; no
explicit `cargo doc` test is wired up locally, but the alias is
used by every parser signature in the workspace so the
declaration is exercised by `cargo build --workspace`.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
`<changelog>` rows dated 2026-05-18 appended inside the element
body at `.speccy/specs/0026-skill-router-anti-triggers/SPEC.md:797`
and `.speccy/specs/0027-host-native-personas/SPEC.md:930`, each
referencing `SPEC-0030` and naming the closed item (T-003
`result_large_err` carry-forward; inherited carve-out).
SPEC-0028's `## Assumptions` block is intentionally untouched —
historical record per REQ-004 prose. `.speccy/BACKLOG.md` status
flip to `implemented` belongs to this ship phase (next step
below).
</coverage>

## Task summary

- Total tasks: 3 (T-001 additive alias; T-002 atomic boxing pass;
  T-003 prior-SPEC changelog rows).
- Retried: 1 (T-002 round 1 caught a rustdoc orphan in
  `parse/spec_xml/mod.rs` and one implicit-box `map_err` outlier
  in `parse/toml_files.rs`; both addressed in retry round and
  passed all four personas on round 2).
- SPEC amendments: 0 — SPEC.md was authored once and not amended.

## Out-of-scope items absorbed

T-002 surfaced five pre-existing clippy violations that
`result_large_err` had been masking under workspace
`pedantic = "deny"` + `-D warnings`. The implementer resolved each
in-place with canonical safe idioms (no `#[allow]`/`#[expect]`
per AGENTS.md):

- `migrate_tasks_schema/src/lib.rs:410` — `needless_raw_string_hashes`
  on a raw string literal that did not contain any `#`.
- `task_xml_body_items.rs:385` and `migrate_tasks_schema/tests/migration.rs:182,198,207` —
  `panic!` in test bodies replaced with `assert!(matches!(...))`
  guards plus `let-else { return; }` (matching the existing
  `panic = "deny"` strict-lint posture).
- `review_redaction.rs:167-187` — `string_slice` on `[range]`
  indexing replaced with `.get(range).expect("...")`.
- `init.rs:1052-1056` — `assertions_on_constants` +
  `unreachable!()` replaced with `assert!(opt.is_some(), ...)`
  plus `.expect(...)`.

All five were carry-forward debt from the SPEC-0026 / SPEC-0028
era when the same `pedantic = "deny"` pin was hidden behind the
`result_large_err` break. Resolving them was the only way to
satisfy REQ-001 done-when row 5
(`cargo clippy --workspace ... -- -D warnings` exits 0).

## Skill updates

(none)

## Deferred / known limitations

- `speccy-core/src/error.rs:3` module-level docstring still reads
  "All public parsers return [`Result<T, ParseError>`]" though
  public signatures now return `ParseResult<T>`. The drift is
  pre-existing relative to REQ-001's carve-out and was flagged
  transparently in the T-002 retry implementer-note; deferred to
  a future doc-accuracy slice rather than expanding this SPEC's
  scope.
- `SpecXmlArtifacts` rustdoc at `speccy-core/src/workspace.rs:531-532`
  phrases the contract using the desugared
  `Result<_, Box<ParseError>>` form rather than the
  `ParseResult<_>` alias used by the field types. Both forms are
  named equivalents in the task entry; the cosmetic doc-vs-code
  phrasing nit is non-blocking.
- A typed `ParsedTasks` newtype with success/failure accessors
  (replacing the `Option<Result<TasksDoc, Box<ParseError>>>` shape
  on `ParsedSpec`) was raised as Open question 3 and deferred —
  wait for the first downstream consumer that finds the nested-
  `Option<Result<_>>` shape ergonomically painful before
  anticipating.
- A `ParseError` prelude re-export was raised as Open question 1
  and deferred — no prelude module exists today and introducing
  one for a single alias is premature.

</report>
