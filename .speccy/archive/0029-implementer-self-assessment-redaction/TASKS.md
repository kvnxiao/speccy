---
spec: SPEC-0029
spec_hash_at_generation: 42b6d85ac87798b4b3a3a828db4a7a838b7675290f7054f68d83a25bb4bf2e0b
generated_at: 2026-05-18T04:42:31Z
---

# Tasks: SPEC-0029 Implementer self-assessment redaction in reviewer prompts


## Phase 1: Schema and parser layer

<task id="T-001" state="completed" covers="REQ-001 REQ-002">
## T-001: Grow `task_xml` whitelist with `<implementer-note>`, `<review>`, `<retry>`; declare `BodyItem` and `ReviewVerdict`

Land the schema-and-parser layer in one atomic edit. This task is
pure-additive against the legacy markdown-bullet payload: existing
in-tree TASKS.md files that still carry `- Implementer note (...)`,
`- Review (...)`, and `- Retry:` bullets continue to parse (the
bullets remain as free Markdown inside `<task>` body) — the new
parser additions only kick in once a TASKS.md actually carries the
new XML elements. The downstream migration of the in-tree corpus
(T-002 / T-003) is deliberately a separate slice so the parser
change can land with full test coverage before being exercised on
real data.

The work spans three files in `speccy-core`:

### `speccy-core/src/parse/task_xml/mod.rs`

- Grow `TASKS_ELEMENT_NAMES` from
  `["tasks", "task", "task-scenarios"]` to exactly
  `["tasks", "task", "task-scenarios", "implementer-note", "review", "retry"]`.
- Extend `validate_tag_shape` to recognise each new element's
  closed attribute set and reject unknown attributes via the
  existing `unknown_attribute_error` diagnostic shape:
  - `<implementer-note>`: required `session` attribute; no
    other attributes accepted.
  - `<review>`: required `persona` (constrained to
    `speccy_core::personas::ALL`) and `verdict` (constrained to
    `{pass, blocking}`); no other attributes accepted.
  - `<retry>`: attribute-free; any attribute is unknown.
- Declare `pub enum BodyItem { ImplementerNote { session: String,
  body: String, span: ElementSpan }, Review { persona: String,
  verdict: ReviewVerdict, body: String, span: ElementSpan }, Retry
  { body: String, span: ElementSpan } }` (exact field names follow
  existing `task_xml` conventions; the three variants and the
  attributes they carry are SPEC-mandated). `ElementSpan` reuses
  the existing source-span type already on `Task`.
- Add a `body_items: Vec<BodyItem>` field (or accessor — planner
  choice) on `Task`, populated by the assemble step in source order
  across all three kinds. `<task-scenarios>` continues to live on
  the existing `scenarios_body` field and is NOT carried in
  `body_items` (per CHK-002's explicit length-4 assertion against
  a 5-child fixture).
- Extend the assemble step to recognise each new element nested
  inside `<task>` and push the corresponding `BodyItem` variant
  onto the new collection.
- Extend `task_xml::render` to emit each `BodyItem` in source
  position using the canonical XML element shape.
- Add the redaction helper (exact name a planner choice — e.g.
  `Task::render_for_review_prompt(source: &str) -> String` or a
  free-standing helper in a sibling module under
  `speccy-core::parse::task_xml`). The helper re-renders the
  task's body in source order, omitting every `<implementer-note>`
  child. T-004 wires it into `speccy review`; landing it here keeps
  the parser-layer surface complete in one commit.

### `speccy-core/src/parse/error.rs`

Add four new `ParseError` variants (exact names follow the file's
existing snake-case discriminant pattern):

- Missing-`session` on `<implementer-note>`. `Display` names the
  attribute and the offending task id (per REQ-001 behavior 2 /
  CHK-001 ¶2).
- Empty-`<implementer-note>`-body. `Display` hints at the
  "task not yet implemented" interpretation (per DEC-004 / CHK-001
  ¶3 / CHK-003 ¶3).
- Invalid `verdict` on `<review>`. `Display` enumerates the closed
  set `{pass, blocking}` (per REQ-001 behavior 4 / CHK-001
  invalid-verdict scenario).
- Invalid `persona` on `<review>`. `Display` enumerates the valid
  persona set drawn from `speccy_core::personas::ALL` (per REQ-001
  behavior 3 / CHK-001 ¶4). The `Display` lookup of
  `personas::ALL` keeps the diagnostic forward-compatible if a
  future SPEC adds a persona.

The `clippy::result_large_err` warning carried forward from
SPEC-0026 T-003 stays carried forward (per `<assumptions>`); these
new variants inherit the existing `#[expect]` suppression rather
than adding a new one.

### `speccy-core/src/lib.rs` (or wherever `TaskState` lives)

Declare `pub enum ReviewVerdict { Pass, Blocking }` parallel to
the existing `TaskState`, with `as_str` returning `"pass"` /
`"blocking"` and `from_str` mirroring `TaskState::from_str`'s
shape (`Option<ReviewVerdict>` return, lowercase match). Re-export
from `speccy_core` so callers reach it as `speccy_core::ReviewVerdict`.

### Tests

Add `speccy-core/tests/task_xml_body_items.rs` (or extend an
existing `task_xml`-focused test file — planner choice) covering:

- Round-trip on a fixture with all three new elements interleaved
  with `<task-scenarios>` and free prose. Parse, then render, then
  re-parse; assert the second parse yields a structurally
  equivalent `TasksDoc` (same task ids, same `body_items` ordering
  and content, same scenario body).
- Source-order preservation across mixed-kind fixtures (per
  CHK-002 ¶1).
- The four new `ParseError` variants fire under their respective
  malformed inputs and the `Display` prose includes the asserted
  substrings (per CHK-001 paragraphs and REQ-001 behaviors 2-4).
- `ReviewVerdict::as_str` / `from_str` round-trip for both
  variants; `from_str` returns `None` for any other string (per
  CHK-002 ¶2).

`cargo test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings` (modulo the carried-forward
`result_large_err`), and `cargo +nightly fmt --all --check` must
pass at task close.

<task-scenarios>
Given a synthetic TASKS.md fixture whose `<task id="T-001"
state="in-review" covers="REQ-001">` carries, in source order,
one `<implementer-note session="s1">…</implementer-note>`, one
`<task-scenarios>…</task-scenarios>`, one `<review
persona="business" verdict="blocking">…</review>`, one
`<retry>…</retry>`, and one `<implementer-note
session="s1-retry">…</implementer-note>`, when `parse_task_xml`
runs against the fixture, then the parse succeeds and the
resulting `Task.body_items` has length 4 with variants in order
`ImplementerNote`, `Review { verdict: Blocking, .. }`, `Retry`,
`ImplementerNote` (the `<task-scenarios>` element lives on
`scenarios_body`, not in `body_items`, per CHK-002 ¶1).

Given a TASKS.md fixture where `<implementer-note>` carries no
`session` attribute, when `parse_task_xml` runs, then it returns
the new missing-`session` `ParseError` variant whose `Display`
includes both the substring `session` and the offending task id.

Given a TASKS.md fixture where `<implementer-note session="x">`
has an empty body (only whitespace between open and close tags),
when `parse_task_xml` runs, then it returns the new empty-body
`ParseError` variant whose `Display` mentions the "task not yet
implemented" interpretation.

Given a TASKS.md fixture where `<review persona="business"
verdict="maybe">` carries an out-of-set verdict, when
`parse_task_xml` runs, then it returns the new invalid-verdict
`ParseError` variant whose `Display` lists the closed set `pass`
and `blocking`.

Given a TASKS.md fixture where `<review persona="kerrigan"
verdict="pass">` carries an out-of-set persona, when
`parse_task_xml` runs, then it returns the new invalid-persona
`ParseError` variant whose `Display` enumerates the valid persona
set drawn from `speccy_core::personas::ALL`.

Given a TASKS.md fixture whose `<task>` body carries mixed
`<implementer-note>`, `<review>`, and `<retry>` elements, when
the fixture is passed through `parse_task_xml` → `task_xml::render`
→ `parse_task_xml`, then the second `TasksDoc` is structurally
equivalent to the first (same task ids, same `body_items` length,
same per-position variant kind, same attribute values, same body
contents) modulo the canonical-not-lossless contract already
documented on `task_xml::render`.

Given `ReviewVerdict::Pass.as_str()` and
`ReviewVerdict::Blocking.as_str()`, when invoked, then they
return `"pass"` and `"blocking"` respectively; given
`ReviewVerdict::from_str` invoked on `"pass"`, `"blocking"`,
`"PASS"`, `""`, and `"maybe"`, then it returns
`Some(ReviewVerdict::Pass)`, `Some(ReviewVerdict::Blocking)`,
`None`, `None`, `None` respectively (the case-sensitivity mirrors
`TaskState::from_str`).

Given the redaction helper (e.g.
`Task::render_for_review_prompt`) invoked on a `Task` carrying
two `<implementer-note>` children, three `<review>` children,
one `<retry>` child, and one `<task-scenarios>` child, when its
output is captured, then the output contains every body byte of
the `<review>`, `<retry>`, and `<task-scenarios>` children
verbatim and contains zero bytes drawn from either
`<implementer-note>` body. Given the same helper invoked on a
`Task` with no `<implementer-note>` child, when its output is
compared byte-for-byte against the unredacted task-entry slice
that `task_lookup::extract_entry_from_raw` would have produced,
then the two strings are byte-identical.
</task-scenarios>

- Suggested files: `speccy-core/src/parse/task_xml/mod.rs`,
  `speccy-core/src/parse/error.rs`,
  `speccy-core/src/lib.rs`,
  `speccy-core/tests/task_xml_body_items.rs` (new).
</task>

## Phase 2: One-shot migration of the in-tree corpus

<task id="T-002" state="completed" covers="REQ-006">
## T-002: Build the migration tool with a both-forms transitional parser and idempotency tests

Implement the migration binary that converts a TASKS.md from the
legacy markdown-bullet conventions to the new XML schema. Land
the tool with full unit-test coverage before applying it to the
in-tree corpus (T-003 runs it; this task only builds and tests it).

The tool lives under `speccy-core/tools/` (exact path is a
planner choice — e.g. `speccy-core/tools/migrate_tasks_schema/`
with a `Cargo.toml` declaring a small binary crate, or a single
`tools/migrate_tasks_schema.rs` registered as a workspace binary).
It is NOT a `speccy` CLI subcommand; it does not surface in
`--help`. A brief `README.md` sits beside the tool documenting
how to invoke it.

The transitional parser is private to the tool and never ships
with `speccy-core`'s public API. It recognises both:

- The legacy markdown-bullet conventions inside `<task>` body:
  `- Implementer note (session-...):` followed by the six
  sub-bullets, `- Review (<persona>, <verdict>): <prose>` lines,
  and `- Retry: <prose>` lines.
- The new XML element form already accepted by the shipped
  `task_xml` parser (so a re-run of the tool against an
  already-migrated TASKS.md is a no-op).

For each `<task>` body, the tool:

1. Walks the source in document order.
2. Classifies each markdown chunk: if it matches a legacy bullet
   convention, emit the corresponding new XML element; if it is
   already an XML element from the new whitelist, preserve it
   verbatim; otherwise preserve the free Markdown verbatim.
3. Re-emits the converted body, then re-renders the full TASKS.md
   via `task_xml::render` (or an equivalent in-tool renderer that
   uses `task_xml`'s public types) to produce the canonical form.
4. Writes the result back to the same path. If the produced bytes
   match the input bytes (i.e. the file is already migrated), the
   tool writes nothing and reports "no change."

Two cross-cutting properties the implementer must hold:

- **Hash neutrality**: the tool does not touch the
  `spec_hash_at_generation` frontmatter value. The hash is over
  SPEC.md, not TASKS.md (per `<assumptions>`), so the migration
  is hash-neutral by construction.
- **Idempotency**: running the tool twice in succession against
  the same TASKS.md produces the same output on the first run and
  a no-op on the second.

### Tests

Add `speccy-core/tools/migrate_tasks_schema/tests/` (or
equivalent) covering:

- A synthetic fixture carrying one task with all three legacy
  bullet conventions (implementer note, review, retry) is
  converted to the canonical XML form. The output parses cleanly
  under the shipped `task_xml` parser. `Task.body_items` on the
  parsed output yields the expected typed sequence.
- A synthetic fixture already in the new XML form passes through
  the tool unchanged (idempotency on already-migrated input).
- A synthetic fixture mixing legacy bullets and new XML elements
  (the "partial-migration" hypothetical) is normalized to a
  single canonical form with all legacy bullets converted and all
  XML elements preserved.
- A synthetic fixture carrying a `<task>` whose body has no legacy
  bullets and no new XML elements (a freshly-decomposed task that
  never reached `in-review`) passes through unchanged.

`cargo test --workspace` includes the tool's tests and must pass.
The tool's binary builds under `cargo build --workspace`.

<task-scenarios>
Given a synthetic TASKS.md fixture carrying one `<task>` whose
body contains, in source order: free prose, `<task-scenarios>`
with non-empty body, a legacy `- Implementer note (session-s1):`
bullet with the six-sub-bullet payload, a `- Review (business,
blocking): <prose>` line, and a `- Retry: <prose>` line, when
the migration tool runs against the fixture, then the produced
output replaces the three legacy bullets with one
`<implementer-note session="s1">...</implementer-note>`, one
`<review persona="business" verdict="blocking">...</review>`,
and one `<retry>...</retry>` block in source order. The
`<task-scenarios>` and free-prose content are byte-preserved.

Given the produced output from the prior scenario, when fed to
the shipped `parse_task_xml`, then the parse succeeds and the
resulting `Task.body_items` yields `[ImplementerNote { session:
"s1", body: <six-bullet payload>, .. }, Review { persona:
"business", verdict: Blocking, body: <prose>, .. }, Retry { body:
<prose>, .. }]` in that order.

Given a synthetic fixture already in the new XML form (no legacy
bullets remain), when the migration tool runs, then the output
bytes equal the input bytes and the tool reports "no change."

Given the migration tool run twice in succession on the same
TASKS.md, when both runs complete, then the second run is a
no-op (zero file modifications, exit code 0) — the tool is
idempotent by inspection of the second run's output.

Given a synthetic TASKS.md whose `spec_hash_at_generation`
frontmatter carries a 64-character hex value, when the migration
tool runs against it, then the output's `spec_hash_at_generation`
value is byte-identical to the input value (the migration is
hash-neutral by construction).

Given the migration tool's binary after this task lands, when
`speccy --help` (or the equivalent CLI inspection) is invoked,
then the migration tool does NOT appear in the subcommand list
(it lives under `speccy-core/tools/`, not under `speccy-cli`).
</task-scenarios>

- Suggested files: `speccy-core/tools/migrate_tasks_schema/` (new
  module / binary crate), `speccy-core/tools/README.md` (new
  brief documentation), workspace `Cargo.toml` (register binary
  if needed).
</task>

<task id="T-003" state="completed" covers="REQ-006 REQ-007">
## T-003: Run the migration on the in-tree corpus, migrate `speccy report` retry counting, and remove `Task.notes()`

Apply T-002's migration tool to every TASKS.md under
`.speccy/specs/`, migrate the one production consumer of
`Task.notes()` to `Task.body_items()`, and remove the legacy
accessor — all in a single commit. The three sub-changes are
tightly coupled: the migration removes the legacy markdown
bullets that `Task.notes()` scanned, so after migration
`notes()` would return an empty `Vec` for every task and
`speccy report`'s retry counting would silently report zero
retries everywhere. The commit must update both halves atomically.

### Apply the migration

Run the T-002 tool against the in-tree corpus
(`.speccy/specs/*/TASKS.md`). Verify each file's resulting
diff converts only the three legacy bullet conventions to their
XML equivalents and leaves all other content byte-identical
(`<task-scenarios>` bodies, free prose, `Suggested files:`
bullets, frontmatter, phase headings).

The migration commit message references SPEC-0029 (per REQ-006
done-when bullet 7).

### Migrate `speccy report`'s retry counting

`speccy-cli/src/report.rs` references `Task.notes()` for the
retry-counting flow that produces the `## Retry summary` section
(or equivalent — exact section name a planner read). Replace the
markdown-bullet scan with iteration over `task.body_items` and
counting `BodyItem::Retry` variants. Semantics:

- Pre-SPEC: one increment per `- Retry:` markdown bullet in the
  task body.
- Post-SPEC: one increment per `BodyItem::Retry` variant in
  `task.body_items`.

The two counts are equivalent by construction (the migration
converts one carrier to the other 1:1 per task).

### Remove `Task.notes()`

Delete the `Task.notes()` accessor at
`speccy-core/src/parse/task_xml/mod.rs:167-182`. Remove every
internal call site (the `report.rs` migration above is the only
known production caller). Remove or rewrite every test that
exercises `Task.notes()`; rewritten tests assert against
`Task.body_items()` instead.

`Task.suggested_files()` stays untouched (per REQ-007 and per
`<assumptions>` — `Suggested files:` is a planner-side
markdown convention out of scope for the XML migration).

### Verification

After the commit:

- `grep -rn "^- Implementer note (session-" .speccy/specs/*/TASKS.md`
  returns zero matches.
- `grep -rn "^- Review (" .speccy/specs/*/TASKS.md` returns zero
  matches.
- `grep -rn "^- Retry: " .speccy/specs/*/TASKS.md` returns zero
  matches.
- `grep -rn "<implementer-note session=" .speccy/specs/*/TASKS.md`
  returns at least one match for every TASKS.md that previously
  carried `- Implementer note` bullets.
- `grep -n "fn notes\\|Task::notes\\|\\.notes()" speccy-core/
  speccy-cli/` returns zero matches in non-test source.
- `cargo test --workspace` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0 (modulo carried-forward `result_large_err`).
- `cargo run --bin speccy -- verify` exits 0 with zero new
  diagnostics attributable to the schema change.
- `cargo run --bin speccy -- report SPEC-NNNN` rendered against
  a migrated spec carrying `<retry>` elements emits the same
  retry counts the pre-SPEC `Task.notes()`-based path would have
  emitted on the equivalent pre-migration source.

<task-scenarios>
Given the in-tree workspace before this task's commit lands, when
`grep -rn "^- Implementer note (session-" .speccy/specs/*/TASKS.md`
runs, then at least one match exists (the legacy bullets are
present). When the same grep runs after this task's commit, then
zero matches exist.

Given the in-tree workspace after this task's commit, when each
TASKS.md is parsed via `parse_task_xml`, then every parse
succeeds. For every TASKS.md that previously carried at least
one `- Implementer note (session-` bullet, the parsed
`TasksDoc` contains at least one `Task` whose `body_items`
includes a `BodyItem::ImplementerNote` variant (the migration
converted the carrier; it did not strip the payload).

Given `cargo run --bin speccy -- verify` after this task's
commit, when run and its JSON output captured, then the captured
output reports zero errors and zero new warnings attributable
to the schema change. Any surviving warnings are explicitly
carried forward from prior SPECs (the `result_large_err` carry
from SPEC-0026 is the only known case) and unchanged in count.

Given `cargo run --bin speccy -- report SPEC-NNNN` invoked
against an arbitrary migrated spec whose `Task.body_items`
includes exactly N `BodyItem::Retry` variants across all tasks,
when the rendered prompt's retry-summary section is read, then
the summary attributes a total of N retries — matching the
count `Task.notes()`-based pre-SPEC code would have emitted on
the equivalent pre-migration source.

Given `speccy-core/src/parse/task_xml/mod.rs` after this task's
commit, when grepped for the literal substring `fn notes(`,
then zero matches exist inside any `impl Task` block. Given
`speccy-cli/src/report.rs` after this task's commit, when grepped
for the substring `.notes()`, then zero matches exist in
production source lines.

Given `cargo test --workspace` after this task's commit, when
run, then the exit code is 0 — including any in-tree-corpus
test (e.g. `speccy-core/tests/in_tree_tasks_reports.rs` if it
exists) that parses every TASKS.md under `.speccy/specs/`.

Given the migration tool re-run against the migrated workspace
after this task's commit, when the second run completes, then
zero files are modified (idempotency holds against the real
in-tree corpus, not just synthetic fixtures).
</task-scenarios>

- Suggested files: every file matching
  `.speccy/specs/*/TASKS.md` (migration output, ~28 files),
  `speccy-cli/src/report.rs` (retry-counting migration),
  `speccy-core/src/parse/task_xml/mod.rs` (remove `Task.notes()`),
  any test under `speccy-core/tests/` or `speccy-cli/tests/`
  exercising `Task.notes()`.
</task>

## Phase 3: Redact `<implementer-note>` from the review-prompt rendering

<task id="T-004" state="completed" covers="REQ-003 REQ-004">
## T-004: Switch `speccy review`'s `{{task_entry}}` substitution to the redacted projection; verify implement/report unchanged

Wire the redaction helper (built in T-001) into `speccy review`.
Change `speccy-cli/src/review.rs:132` from
`vars.insert("task_entry", location.task_entry_raw.clone());`
to substitute the redacted projection drawn from a typed `Task`
loaded via `parse_task_xml`. The redaction call site lives in
`speccy-cli`; the redaction logic itself lives in `speccy-core`
(per REQ-003 done-when bullet 2: "the new helper lives in
`speccy-core` (not in `speccy-cli`)").

The redaction is uniform across personas: no `Persona` parameter
on the helper, no per-persona branch in `review.rs`, no
configuration knob.

The redaction is silent: no placeholder line, no XML comment, no
"(implementer notes withheld)" prose. The rendered prompt is
indistinguishable between a task with `<implementer-note>`
children and one without, except for the omitted bodies
themselves (per DEC-002).

### Verify implement/report unchanged (REQ-004)

`speccy-cli/src/implement.rs:107` continues to substitute
`location.task_entry_raw.clone()` byte-for-byte — the implementer
prompt is NOT redacted (per REQ-004 done-when bullet 1). Confirm
this by code-read and add an integration test asserting the
rendered implement prompt for a task carrying one or more
`<implementer-note>` elements contains the substring
`<implementer-note` in its `## Task entry` section.

`speccy-cli/src/report.rs` already names `{{tasks_md_path}}`
rather than inlining TASKS.md content (the report prompt was
already unaffected before this SPEC); the only `report.rs` change
in this SPEC is the retry-counting migration done in T-003.
Confirm by code-read that no new redaction helper call appears
in `report.rs`.

`resources/modules/prompts/report.md` still references
`Procedural compliance` lines (the report prompt content is
unchanged by this SPEC).

### Tests

Add integration tests under `speccy-cli/tests/` (or extend an
existing review-focused test file — planner choice) covering the
behavior assertions in CHK-003 and CHK-004:

- A task carrying mixed body items renders a `{{task_entry}}`
  substitution that contains every `<review>` body, every
  `<retry>` body, the `<task-scenarios>` body, the free prose,
  and the `Suggested files:` line verbatim — and contains zero
  bytes of any `<implementer-note>` body.
- The same task rendered across all six built-in personas
  (`speccy_core::personas::ALL`) produces six byte-identical
  `{{task_entry}}` substitutions (uniform redaction).
- A task carrying no `<implementer-note>` element renders a
  `{{task_entry}}` substitution byte-identical to the pre-SPEC
  raw `location.task_entry_raw` slice (the redactor is a no-op
  when there is nothing to remove).
- The rendered review prompt contains no occurrence of the
  substrings `Commands run:`, `Exit codes:`, `Discovered issues:`,
  `Procedural compliance:`, `Undone:`, or `Completed:` inside
  its `## Task entry` section.
- The rendered review prompt contains no placeholder-style
  marker (`redacted`, `withheld`, `hidden`, etc.).
- `speccy implement` rendered for the same fixture-task contains
  the substring `<implementer-note` in its `## Task entry`
  section (verifies REQ-004 contract — implement prompt is NOT
  redacted).

`cargo test --workspace`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, and `cargo +nightly fmt --all
--check` must pass at task close.

<task-scenarios>
Given a TASKS.md fixture under a synthetic workspace where
`<task id="T-001" state="in-review" covers="REQ-001">` carries,
in source order, free prose, `<task-scenarios>` with non-empty
body, `<implementer-note session="s1">` with the six-sub-bullet
payload, `<review persona="business" verdict="blocking">` with
prose, `<retry>` with prose, `<implementer-note session="s1-retry">`
with the retry-session payload, and `<review persona="business"
verdict="pass">` with prose, when `speccy review SPEC-NNNN/T-001
--persona business` runs against the fixture and the rendered
prompt is captured, then the captured prompt's `## Task entry`
section contains every byte of the free prose, the
`<task-scenarios>` body, both `<review>` bodies (with their
attribute values), and the `<retry>` body verbatim and in source
order. The captured prompt contains zero bytes drawn from either
`<implementer-note>` body. The captured prompt contains none of
the substrings `Commands run:`, `Exit codes:`, `Discovered
issues:`, `Procedural compliance:`, `Undone:`, or `Completed:`
inside the `## Task entry` section.

Given the same fixture, when `speccy review SPEC-NNNN/T-001
--persona <P>` is invoked once per `P` in
`speccy_core::personas::ALL` (six runs total) and the captured
prompts are diffed pairwise on the `## Task entry` section, then
the section is byte-identical across all six (uniform redaction).

Given a TASKS.md fixture where `<task id="T-002"
state="in-review" covers="REQ-002">` carries no
`<implementer-note>` element (e.g. a freshly-flipped task that
hit `in-review` without an implementer note ever being written
— or after this SPEC lands, a task whose `<implementer-note>`
parse rejected per REQ-001's empty-body diagnostic), when the
parser is given an instance where the `<implementer-note>` is
literally absent (not empty), `speccy review SPEC-NNNN/T-002
--persona business` runs, then the captured prompt's
`{{task_entry}}` substitution is byte-identical to the
unredacted `task_entry_raw` slice that the pre-SPEC code would
have substituted (the redactor is a no-op when there is no
`<implementer-note>` to remove). Note: REQ-001 already enforces
empty-body rejection; a task that reaches `state="in-review"`
without an implementer note at all is a writer-side anomaly,
not a parse failure.

Given the captured rendered review prompt from any of the above
scenarios, when scanned for placeholder-style prose
(`redacted`, `withheld`, `hidden`, `notes omitted`, or
substantially equivalent), then no match is found.

Given the same fixture under the synthetic workspace, when
`speccy implement SPEC-NNNN/T-001` runs and the rendered prompt
is captured, then the captured prompt's `## Task entry` section
contains the substring `<implementer-note` at least once (the
implementer prompt is NOT redacted, per REQ-004).

Given the same workspace, when `speccy report SPEC-NNNN` runs
and the rendered prompt is captured, then the captured prompt
names `{{tasks_md_path}}` (or its rendered absolute path),
instructs the report agent to read TASKS.md via the host Read
primitive, and does NOT inline TASKS.md bytes — the report
prompt's surface is byte-identical to its pre-SPEC behavior
modulo the existing `Procedural compliance` reference (which
this SPEC does not touch).
</task-scenarios>

- Suggested files: `speccy-cli/src/review.rs` (call-site switch
  at line 132), `speccy-cli/tests/review_redaction.rs` (new
  integration tests — or extend an existing review-focused test
  file).
</task>

## Phase 4: Writer-side skill prompts

<task id="T-005" state="completed" covers="REQ-005">
## T-005: Update shipped skill prompts to emit `<implementer-note>`, `<review>`, and `<retry>` XML elements

Retire the legacy markdown-bullet authoring conventions from the
shipped skill prompts. Each prompt below is updated in lockstep
so the wording of the emit-an-XML-element instruction is
identical across the six reviewer personas.

### Files to update

- `resources/modules/prompts/implementer.md` — replace the
  task-closure instruction's "append a `- Implementer note
  (session-...):`" guidance with "append an `<implementer-note
  session="...">…</implementer-note>` element block" guidance.
  The six required sub-bullets (`Completed`, `Undone`, `Commands
  run`, `Exit codes`, `Discovered issues`, `Procedural
  compliance`) remain inside the element body as markdown payload
  (per DEC-004's "body stays as markdown payload" framing).
- `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `resources/modules/prompts/reviewer-architecture.md`,
  `resources/modules/prompts/reviewer-docs.md` — replace each
  prompt's "append a `- Review (<persona>, <verdict>): …`"
  guidance with "append a `<review persona="<persona>"
  verdict="<verdict>">…</review>` element block" guidance. The
  emit-an-XML-element wording is identical across the six files.
- `.claude/skills/speccy-review/SKILL.md` and its mirror under
  `resources/agents/` (exact path discovered by a `glob` — the
  agent mirror layout was established by a prior SPEC) —
  replace the blocking-verdict orchestration step's "write a
  `- Retry: …`" guidance with "write a `<retry>…</retry>`
  element block" guidance.

### Verification

- `grep -rn "Implementer note (session" resources/ .claude/`
  returns zero matches in skill / prompt files (the legacy form
  is retired from authoring instructions).
- `grep -rn "- Review (" resources/ .claude/` returns zero
  matches in skill / prompt files.
- `grep -rn "^- Retry:" resources/ .claude/` returns zero
  matches in skill / prompt files.
- `grep -rn "<implementer-note session=" resources/` returns at
  least one match (in `implementer.md`).
- `grep -rn "<review persona=" resources/` returns at least six
  matches (one per reviewer persona file).
- `grep -rn "<retry>" .claude/skills/speccy-review/` returns at
  least one match.
- `cargo test --workspace` continues to pass (any test
  exercising the shipped prompt content updates in this commit).
- `cargo run --bin speccy -- verify` exits 0 against the
  workspace (the verifier reads skill prompts but does not
  validate their authoring instructions; the gate is "doesn't
  regress").

This task does not modify the in-tree corpus of TASKS.md (T-003
already migrated those). It also does not modify `speccy review`
or `speccy implement`'s `task_xml`-layer behavior (T-001 / T-004
already wired those). It is a pure writer-side update.

<task-scenarios>
Given `resources/modules/prompts/implementer.md` after this
task's commit, when grepped for the substring
`<implementer-note session=`, then at least one match exists in
the prompt's task-closure instruction. When the same file is
grepped for the legacy substring `Implementer note (session-`,
then zero matches exist in authoring instructions.

Given each of the six files
`resources/modules/prompts/reviewer-{business,tests,security,style,architecture,docs}.md`
after this task's commit, when grepped for the substring
`<review persona=`, then at least one match exists per file in
its review-note authoring instruction. When the same files are
grepped for the legacy substring `- Review (`, then zero
matches exist in authoring instructions.

Given `.claude/skills/speccy-review/SKILL.md` (and its mirror
under `resources/agents/`) after this task's commit, when
grepped for the substring `<retry>`, then at least one match
exists in the blocking-verdict orchestration step. When the
same files are grepped for the legacy substring `- Retry:`,
then zero matches exist in authoring instructions.

Given the six reviewer persona prompts side-by-side after this
task's commit, when their emit-an-XML-element instructions are
compared, then the wording of the instruction is identical
across the six files (lockstep update; differences live in
persona-specific concern framing elsewhere in the prompt, not
in the emit instruction itself).

Given the entire `.claude/` and `resources/` trees after this
task's commit, when scanned for surviving "Implementer note",
"Review (persona,", or "Retry:" prefix conventions, then the
only remaining mentions live inside historical SPEC bodies
(under `.speccy/specs/`) or this SPEC's own prose, not in
shipped authoring instructions.

Given `cargo run --bin speccy -- verify` after this task's
commit, when run, then the exit code is 0 (the verifier reads
skill content but does not regress on the rewritten authoring
instructions).
</task-scenarios>

- Suggested files: `resources/modules/prompts/implementer.md`,
  `resources/modules/prompts/reviewer-business.md`,
  `resources/modules/prompts/reviewer-tests.md`,
  `resources/modules/prompts/reviewer-security.md`,
  `resources/modules/prompts/reviewer-style.md`,
  `resources/modules/prompts/reviewer-architecture.md`,
  `resources/modules/prompts/reviewer-docs.md`,
  `.claude/skills/speccy-review/SKILL.md`,
  `resources/agents/speccy-review/SKILL.md` (mirror, exact path
  via glob).
</task>

