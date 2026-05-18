---
spec: SPEC-0029
spec_hash_at_generation: 42b6d85ac87798b4b3a3a828db4a7a838b7675290f7054f68d83a25bb4bf2e0b
generated_at: 2026-05-18T04:42:31Z
---

# Tasks: SPEC-0029 Implementer self-assessment redaction in reviewer prompts

<tasks spec="SPEC-0029">

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

<implementer-note session="2026-05-18-spec0029-t001">
- Completed: Grew `TASKS_ELEMENT_NAMES` to six names; added
  `ALLOWED_REVIEW_VERDICTS`, the `ReviewVerdict` enum (with `as_str`
  and `from_str` mirroring `TaskState`), the `BodyItem` enum
  (three variants `ImplementerNote`, `Review`, `Retry`), the
  `Task.body_items: Vec<BodyItem>` field, and the
  `redact_implementer_notes(task_entry: &str, task: &Task) -> String`
  redaction helper in `task_xml`. Extended `validate_tag_shape`
  with the new elements' closed attribute sets, extended the
  assemble step's `Block` enum and `PendingBlock::finish` with
  three new variants, factored the children walk into a new
  `collect_task_children` helper (also addresses the
  `too_many_lines` clippy on the previous `build_task` shape),
  and extended the renderer to emit body items in source order
  after `<task-scenarios>` via a new `push_body_item` helper and
  a generalised `strip_nested_body_blocks` that strips all four
  inline element kinds from the prose carrier. Added four new
  `ParseError` variants (`MissingImplementerNoteSession`,
  `EmptyImplementerNoteBody`, `InvalidReviewVerdict`,
  `InvalidReviewPersona`) with `Display` prose that names the
  offending task id and either the closed-set or the hint
  text. Re-exported `BodyItem`, `ReviewVerdict`, and
  `redact_implementer_notes` from `speccy_core::parse`. Wrote a
  new integration test file `task_xml_body_items.rs` exercising
  the source-order property, every new `ParseError` variant,
  `ReviewVerdict::as_str`/`from_str` round-trip, the round-trip
  parse → render → parse equivalence for mixed body items, and
  both halves of the redaction helper's contract (strips
  `<implementer-note>` bodies and is byte-identical when none
  are present). Updated the `report.rs` test-side `Task` stub
  to carry the new `body_items` field.
- Undone: T-002 (migration tool), T-003 (apply migration +
  `Task.notes()` removal + `speccy report` retry-counting
  consumer migration), T-004 (wire the redaction helper into
  `speccy review`), T-005 (writer-side skill prompts) are
  explicitly out of scope per the task slicing decided in
  `/speccy-tasks`. The legacy `Task.notes()` accessor is
  intentionally still live — `speccy report`'s retry counting
  continues to find `- Retry:` markdown bullets until T-003
  lands.
- Commands run: `cargo build --workspace --tests`;
  `cargo test --workspace`; `cargo clippy --workspace
  --all-targets --all-features` (modulo the carried-forward
  `result_large_err` against `ParseError` per `<assumptions>`);
  `cargo +nightly fmt --all --check`; `cargo deny check`;
  `cargo run --quiet -- verify`.
- Exit codes: 0; 0; 0 modulo `result_large_err` carry; 0; 0; 0
  (29 specs, 154 requirements, 192 scenarios, 0 errors).
- Discovered issues: the in-tree id-set snapshot fixture at
  `speccy-core/tests/fixtures/in_tree_id_snapshot.json` was
  missing an entry for SPEC-0029, which surfaced as a hard
  failure in
  `every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot`.
  Added the SPEC-0029 entry (7 requirements, 7 scenarios, 8
  decisions). The fixture's update cadence ("hand-edit each
  time a new spec lands") is structural friction; if it keeps
  surfacing on every spec it should grow into a generator
  rather than a checked-in fixture, but that is out of scope
  for this slice.
- Procedural compliance: no shipped skill files touched; the
  `speccy-work` / `speccy-implement` prompt instructions
  matched the task as decomposed. The legacy markdown-bullet
  form is used for THIS implementer note (T-005 retires the
  convention from shipped skill prompts; T-003's migration
  will sweep this bullet into the new
  `<implementer-note session="...">` XML form as part of the
  in-tree corpus migration).
</implementer-note>

<review persona="security" verdict="pass">
parser-layer schema addition with no
network/filesystem/credential surface; new closed-set validations
(`verdict ∈ {pass, blocking}`, `persona ∈ personas::ALL`,
required non-empty `session`, non-whitespace
`<implementer-note>` body) tighten the input boundary rather
than loosen it. `redact_implementer_notes`
(`speccy-core/src/parse/task_xml/mod.rs:907`) has the right
shape: byte-identity short-circuit when `body_items` carries no
`ImplementerNote`, line-oriented strip otherwise; both halves
exercised in `speccy-core/tests/task_xml_body_items.rs:411`
(positive — also asserts no placeholder smuggling) and
`:506` (byte-equality). No new dependencies, no `unsafe`, no
`unwrap()` outside the pre-existing `#[expect(...)]`-annotated
static regex builders. Sub-blocking observation for future
hardening (not for this slice): the redactor's caller-discipline
contract — `task_entry` and `task` must derive from the same
parse — is not enforced at the API boundary; if a future caller
passes a `Task` whose `body_items` is empty alongside a
`task_entry` byte-slice from a different task that does contain
`<implementer-note>` text, the short-circuit returns the raw
slice unredacted. T-004's call site derives both from the same
`task_lookup::find` result so this is moot today; consider a
debug-assert or single-parse helper at the T-004 wire-up to
pin the invariant structurally.
</review>

<review persona="business" verdict="pass">
T-001 substantively delivers the
parser-layer slice contracted by REQ-001 + REQ-002 (covers
attribute). All eight slice-level `<task-scenarios>` assertions
have matching tests in
`speccy-core/tests/task_xml_body_items.rs` (length-4
source-order, missing/empty `session`, empty body with the
"not yet" hint, invalid verdict listing `pass, blocking`,
invalid persona enumerating `personas::ALL`, parse→render→parse
round-trip, `ReviewVerdict::as_str/from_str`, and both halves of
the redaction helper's contract). REQ-001 done-when bullets all
trace to diff: `TASKS_ELEMENT_NAMES` is exactly the six names,
`validate_tag_shape` grew the new attribute closed sets, the
four new `ParseError` variants carry the required `Display`
substrings. REQ-002 done-when likewise: `BodyItem` enum has the
three SPEC-mandated variants with the named fields,
`ReviewVerdict` is the closed `{Pass, Blocking}` enum parallel
to `TaskState` with `as_str`/`from_str` mirroring its shape, and
`Task.body_items: Vec<BodyItem>` preserves document order.
Non-goals all honoured: the redaction helper takes no `Persona`
parameter (DEC-003 uniformity preserved), emits no placeholder
marker (DEC-002 silent), and the byte-identity contract for
zero-`<implementer-note>` tasks is asserted directly in
`redact_is_byte_identical_when_no_implementer_note_present`.
REQ-003/004/005/006/007 are correctly out of slice scope (T-002
→ T-005); the `Undone` bullet accurately catalogues what stays
behind. The redaction helper lands in this slice rather than
T-004 per the task body's explicit planner choice — the SPEC's
"Approach" section places the helper in `speccy-core`'s parser
layer, so the parser-layer-completeness framing matches intent.
No silent resolution of any `[x]`-closed open question detected.
</review>

<review persona="style" verdict="blocking">
broken intra-doc link introduced by
this diff. `speccy-core/src/parse/task_xml/mod.rs:950` writes
`[`task::body_items`]` (lowercase `task`); `cargo doc -p
speccy-core --no-deps` emits `warning: unresolved link to
`task::body_items` ... no item named `task` in scope` under the
default `rustdoc::broken_intra_doc_links` lint. Every other
intra-doc link in the new docblock spells it correctly (line
930 `[`Task`]`, line 932 `[`Task::body_items`]`, line 936
`[`BodyItem::ImplementerNote`]`), so this is a one-character
typo, not a design call. Fix to `[`Task::body_items`]` to match
the rest of the file. The `prompt/mod.rs` ambiguous-link
warning surfaced in the same `cargo doc` run is pre-existing
and out of scope for this slice.
</review>

<review persona="tests" verdict="pass">
`speccy-core/tests/task_xml_body_items.rs`
exercises the real `parse_task_xml`, `render_task_xml`, and
`redact_implementer_notes` production helpers with no
mocking; all 10 slice-level scenarios from `<task-scenarios>`
are covered — length-4 ordering with `<task-scenarios>`
excluded from `body_items`; `MissingImplementerNoteSession`
with `T-001` named in `Display`; the `session=""`
empty-string boundary collapsed to the same variant as
missing (a useful adversarial test that catches a
`.is_none()`-only check that would let `session=""`
through); `EmptyImplementerNoteBody` with the "not been
implemented" hint substring asserted; `InvalidReviewVerdict`
listing `pass, blocking` and pinning `value == "maybe"`;
`InvalidReviewPersona` enumerating every `personas::ALL`
entry in the `Display`; the parse → render → parse
round-trip matching variant/attribute/body field-by-field;
`ReviewVerdict::as_str` / `from_str` round-trip including
`"PASS"` / `""` / `"maybe"` → `None` (case-sensitivity
asserted); the redaction strip against an interleaved
fixture (zero `<implementer-note` substring, zero
`Exit codes:` / `Discovered issues:` /
`Procedural compliance:` sub-bullet markers, zero
placeholder marker prose like `redacted` / `withheld` /
`hidden`, and `<review>` / `<retry>` / `<task-scenarios>` /
free-prose bodies all preserved); and the byte-identity
no-op on a task carrying no `<implementer-note>`.
Substituting a deliberately broken
`redact_implementer_notes` (e.g. returning `""`, stripping
`<review>` instead of `<implementer-note>`, or hard-coding
the `has_note` short-circuit to `true`) fails the suite, so
the tests are adversarial against obvious wrong
implementations. Two minor gaps worth naming but not
blocking for this slice: (1) the strip test uses
`contains(...)` substring checks on `<review>` / `<retry>`
/ `<task-scenarios>` bodies rather than asserting
byte-verbatim preservation of those subtrees — a bug that
mangled the `<review persona="business" verdict="blocking">`
open tag while leaving the body text intact would slip
through (the user-facing CHK-003 byte-level pin lands at
T-004 via the `speccy review` prompt capture, which is the
explicit boundary the slice notes defer to). (2) REQ-001
`<behavior>` ¶5 (a `<review>` at the root, or `<retry>`
outside any `<task>`, surfacing the "must be nested inside
`<task>`" diagnostic) is implemented in `parse` but not
exercised by a test — an obviously-wrong implementation
that silently accepted a root-level `<review>` would not
be caught here. Both gaps are user-facing-level (REQ-001
`<behavior>`) rather than slice-level (T-001's
`<task-scenarios>`) so they do not block this task; flag
for T-004 to pin the byte-level redaction contract at the
review-prompt-capture boundary.
</review>

<retry>
style (blocking) flagged a broken rustdoc intra-doc link
in the `redact_implementer_notes` docblock at
`speccy-core/src/parse/task_xml/mod.rs` (the
`` [`task::body_items`] `` reference with lowercase `task`).
Fix: change to `` [`Task::body_items`] `` to match the
surrounding correctly-cased links (`Task`, `BodyItem`,
`BodyItem::ImplementerNote`). After the fix, confirm with
`cargo doc -p speccy-core --no-deps 2>&1 | grep -i broken`
returning empty. Business / tests / security personas all
reported `pass`; tests and security each flagged non-blocking
observations (byte-verbatim preservation in the redactor strip
test, root-level / outside-task nesting tests, and caller
discipline for the redactor's `(task_entry, task)` pairing) —
none of those block this slice and they are pre-noted for
T-004.
</retry>

<implementer-note session="2026-05-18-spec0029-t001-retry">
- Completed: Fixed the broken rustdoc intra-doc link in the
  `redact_implementer_notes` docblock at
  `speccy-core/src/parse/task_xml/mod.rs:950` — changed
  `` [`task::body_items`] `` (lowercase `task`) to
  `` [`Task::body_items`] `` to match the surrounding
  correctly-cased intra-doc links (`Task` at line 930,
  `Task::body_items` at line 932, `BodyItem::ImplementerNote`
  at line 936). One-character casing fix, no behavioural
  change.
- Undone: Tasks T-002 through T-005 remain out of scope for
  this slice per the original task slicing; their statuses are
  unchanged. The pre-existing `render` ambiguous-link rustdoc
  warning (function vs. module at `prompt/mod.rs`) was flagged
  by the style reviewer as pre-existing and out of scope; it
  stays untouched.
- Commands run: `cargo doc -p speccy-core --no-deps`
  (pre-fix and post-fix); `cargo test --workspace`;
  `cargo clippy --workspace --all-targets --all-features`
  (modulo the carried-forward `result_large_err` against
  `ParseError` per `<assumptions>`);
  `cargo +nightly fmt --all --check`; `cargo deny check`;
  `cargo run --quiet -- verify`.
- Exit codes: 0 (pre-fix: 1 warning `unresolved link to
  task::body_items` + 1 pre-existing ambiguous `render`
  warning); 0 (post-fix: only the pre-existing `render`
  ambiguous-link warning); 0; 0 modulo `result_large_err`
  carry; 0; 0; 0 (29 specs, 154 requirements, 192 scenarios,
  0 errors, 0 warnings, 48 info).
- Discovered issues: none specific to this retry. The
  non-blocking observations the tests and security reviewers
  flagged on the original pass (byte-verbatim preservation in
  the redactor strip test, root-level / outside-task nesting
  tests for the new XML elements, and caller-discipline for
  `redact_implementer_notes`'s `(task_entry, task)` pairing)
  remain pre-noted for T-004; they are user-facing-level
  (REQ-001 `<behavior>` / REQ-003 byte-pin at the
  review-prompt-capture boundary) rather than slice-level for
  T-001 and do not block this retry.
- Procedural compliance: no shipped skill files touched; the
  `speccy-work` skill's pending → in-progress → in-review
  flow matched the retry as described. The legacy
  markdown-bullet form is used for this retry implementer
  note (T-005 retires the convention from shipped skill
  prompts; T-003's migration sweeps both this bullet and the
  parent T-001 implementer note into the new
  `<implementer-note session="...">` XML form as part of the
  in-tree corpus migration).
</implementer-note>

<review persona="security" verdict="pass">
re-review of the retry confirms no new
attack surface. The four new closed-set `ParseError` variants
in `speccy-core/src/error.rs:531-594` tighten the parse-time
input boundary (missing/empty `session`, empty
`<implementer-note>` body after `.trim()`, verdict outside
`{pass, blocking}`, persona outside `personas::ALL`). The
`build_implementer_note` helper at
`speccy-core/src/parse/task_xml/mod.rs:730-745` correctly
collapses missing-`session` and empty-string-`session` to the
same variant via `unwrap_or_default()` + `.is_empty()`,
closing a `session=""` bypass that a `.is_none()`-only check
would have left open (verified by
`speccy-core/tests/task_xml_body_items.rs` covering both
cases under `MissingImplementerNoteSession`). The
`redact_implementer_notes` helper at
`speccy-core/src/parse/task_xml/mod.rs:955-980` is the
security-critical primitive of the SPEC; its tests at
`speccy-core/tests/task_xml_body_items.rs:411` adversarially
assert zero `<implementer-note` substring, zero payload
sub-bullet leakage (`Exit codes:`, `Discovered issues:`,
`Procedural compliance:`), and zero placeholder-marker prose
(`redacted` / `withheld` / `hidden` / `notes omitted`) in the
redacted output, plus byte-identity at `:506` when no notes
are present. No new dependencies, no `unsafe`, no `unwrap()`
/ `expect()` / `panic!()` in production code, and error
`Display` strings expose only local workspace paths, task
ids, offsets, and the offending attribute value (no secrets,
no remote-controllable input — TASKS.md is developer-edited
local state). Sub-blocking observation carried forward for
T-004 (unchanged from the original pass note): the
redactor's `(task_entry, task)` caller-discipline contract is
not enforced at the API boundary — if a future caller passes
a `task_entry` byte-slice that contains `<implementer-note`
text alongside a `Task` whose `body_items` happens to be
empty (e.g. cross-task slice/parse mismatch), the
`has_note`-false short-circuit at line 960-962 returns the
raw slice unredacted. T-004's call site derives both from a
single `task_lookup::find` result so this is unreachable
today; consider a debug-assert or a typed wrapper (e.g.
`TaskWithEntry { entry: &str, task: &Task }`) at the T-004
wire-up to pin the invariant structurally rather than via
call-site convention.
</review>

<review persona="style" verdict="pass">
diff conforms to project Rust conventions —
the broken intra-doc link the previous style pass flagged
(`speccy-core/src/parse/task_xml/mod.rs:950`) is fixed and
`cargo doc -p speccy-core --no-deps` emits no new warnings
beyond the pre-existing `render` ambiguity. New `pub` items
(`ReviewVerdict`, `BodyItem`, `redact_implementer_notes`,
`ALLOWED_REVIEW_VERDICTS`) all carry contextual `#[must_use =
"..."]` and doc comments mirroring the surrounding
`TaskState` / `Task` shape; `ReviewVerdict::from_str`'s
trait-not-implemented divergence is silenced via
`#[expect(clippy::should_implement_trait, reason = "...")]`
rather than `#[allow]` per `AGENTS.md` "Conventions for AI
agents specifically". No `unwrap()`/`expect()`/`panic!()` in
production code; the regex `OnceLock` builders' `#[expect]`
reasons are unchanged. `cargo clippy --workspace --all-targets
--all-features` is clean modulo the carried-forward
`result_large_err` documented in `<assumptions>` (no new
variants are flagged beyond what SPEC-0026 T-003 already
carries). The four new `ParseError` variants follow the
file's snake-case discriminant pattern, `thiserror`
`#[error("...")]` shape, and field-doc convention; their
`Display` substrings (`session`, the task id, the
not-yet-implemented hint, the closed `pass, blocking` /
`personas::ALL` sets) match what the slice-level scenarios
pin. `strip_nested_body_blocks` is a clean generalisation of
the deleted `strip_nested_task_scenarios` rather than a
parallel helper, and `find_attr(...).unwrap_or_default()` in
`build_implementer_note` / `build_review` diverges from the
surrounding `find_attr(...).ok_or_else(|| Missing...)`
pattern only where SPEC-0029 REQ-001 mandates a dedicated
`ParseError` variant rather than the generic
`MissingTaskAttribute` shape — intentional, not drift.
Suggestion (non-blocking): the two `match block` arms that
reject `Block::ImplementerNote` / `Review` / `Retry` at the
top-level loop are byte-identical to the arms in the
`for child in children` loop further down (lines 454-474 vs.
511-531), six copies of the same `MalformedMarker { ...,
reason: "<X> element must be nested inside <task>" }`
shape; the existing `TaskScenarios` arm was already
duplicated this way, so extending the same pattern is
consistent rather than net-new — but a small
`Block::nesting_violation_reason(&self) -> Option<&'static
str>` helper would collapse the duplication once a future
slice has reason to touch this region.
</review>

<review persona="tests" verdict="pass">
`speccy-core/tests/task_xml_body_items.rs`
drives the real `parse_task_xml`, `render_task_xml`, and
`redact_implementer_notes` helpers with no mocks; all 10
passing assertions trace to the eight `<task-scenarios>`
entries. Length-4 ordering is positively asserted against a
5-child fixture so an implementation that pushed
`<task-scenarios>` into `body_items` fails on length. Error
variants are pinned by `matches!` on the discriminant AND by
substring assertions on `Display` (`"session"` + `"T-001"`
for missing-`session` at line 156-157; `"not been
implemented" || "not yet"` for empty body at line 217-220;
`"pass"` + `"blocking"` for invalid verdict at line 255-256;
every `personas::ALL` entry enumerated for invalid persona at
line 289-294). The empty-`session=""` boundary collapses to
the same variant as missing
(`speccy-core/tests/task_xml_body_items.rs:163`), catching a
`find_attr(...).is_none()`-only check that would let
`session=""` through. `ReviewVerdict::from_str` round-trips
`"PASS"` / `""` / `"maybe"` → `None` so case-sensitivity is
pinned. The redaction strip test
(`speccy-core/tests/task_xml_body_items.rs:411`) asserts
absence of the `<implementer-note` substring AND absence of
every implementer-note payload sub-bullet marker AND absence
of placeholder-style prose (`redacted`, `withheld`, `hidden`,
`notes omitted`) AND verbatim presence of every other body
item — so a broken impl that stripped `<review>` instead, or
smuggled in a marker, or stripped too eagerly, fails. The
byte-identity contract for the no-`<implementer-note>` case
is asserted directly (`assert_eq!(redacted, entry_raw)` at
line 529). Mental rewrites that would break the contract —
returning `""`, stripping the wrong element, hard-coding
`has_note = true`, swapping `from_str` to return
`Some(Pass)`, omitting one of the four `ParseError`
discriminants — all fail at least one test. Three slice-level
gaps worth naming but not blocking: (1) the round-trip body
comparison uses `.trim()` rather than byte-verbatim
(`speccy-core/tests/task_xml_body_items.rs:340,361,379,382`),
so a renderer that ate or added internal whitespace inside a
body would slip through — though REQ-002 explicitly defers to
`task_xml::render`'s canonical-not-lossless contract so this
is by design; (2) no test asserts that
`<implementer-note session="x" foo="bar">` fires the
`UnknownAttribute` diagnostic — REQ-001's done-when bullet
"rejects unknown attributes via the existing
`unknown_attribute_error` diagnostic shape" is implemented at
`speccy-core/src/parse/task_xml/mod.rs:1189-1196` but a
regression that removed the `"implementer-note"` / `"review"`
arms would let extra attributes through and the suite would
still go green; (3) REQ-001 `<behavior>` ¶5 (`<review>` at
the root, `<retry>` outside any `<task>`) is implemented at
`speccy-core/src/parse/task_xml/mod.rs:454-471` and `:511-528`
but no test exercises either error path. Gap (3) is
user-facing-level per REQ-001 `<behavior>` and pre-flagged by
the implementer's own tests-persona note as deferred to T-004
byte-level pinning; gap (2) is a slice-level done-when item
that should grow a one-line test in this slice or land
alongside the migration in T-002/T-003.
</review>

<review persona="business" verdict="pass">
re-review of the retry confirms the
parser-layer slice contracted by REQ-001 + REQ-002 substantively
delivers what the SPEC promises. Every REQ-001 done-when bullet
traces to diff: `TASKS_ELEMENT_NAMES` is exactly the six
SPEC-mandated names
(`speccy-core/src/parse/task_xml/mod.rs:36-43`);
`validate_tag_shape` grew the new closed attribute sets at
`:1189-1194` (`implementer-note` accepts only `session`,
`review` accepts only `persona` + `verdict`, `retry` and
`task-scenarios` attribute-free), rejecting via the existing
`unknown_attribute_error` diagnostic shape. The four new
`ParseError` variants in `speccy-core/src/error.rs:531-594`
carry the SPEC-required `Display` substrings (`session` plus
the task id; the "may not yet have been implemented" hint
satisfying CHK-001 ¶3; the closed `{pass, blocking}` set; the
`personas::ALL` enumeration). Every REQ-002 done-when bullet
likewise: `BodyItem` exposes the three SPEC-mandated variants
with the named fields
(`speccy-core/src/parse/task_xml/mod.rs:171-213`);
`ReviewVerdict` is the closed `{Pass, Blocking}` enum at
`:116-150` parallel to `TaskState` with `as_str`/`from_str`
mirroring its shape (case-sensitive `Option<Self>`, wire
strings `"pass"`/`"blocking"`); `Task.body_items:
Vec<BodyItem>` at `:228-232` preserves document order and
excludes `<task-scenarios>` per CHK-002 ¶1. All eight
slice-level `<task-scenarios>` assertions have matching tests
in `speccy-core/tests/task_xml_body_items.rs`. Non-goals
honoured: the redaction helper at `:954-980` takes no
`Persona` parameter (DEC-003 uniformity preserved), emits no
placeholder marker (DEC-002 silent — strip-only, never
insert), and the byte-identity contract when no
`<implementer-note>` is present is directly asserted. The
redaction helper landing here rather than at T-004 matches
the SPEC's Approach section (lines 941-946) placing it in the
parser layer; the slice's planner-choice framing tracks the
SPEC explicitly. REQ-003/004/005/006/007 correctly stay
out of slice scope; the `Undone` bullet accurately catalogues
what stays behind, including the intentionally-live
`Task.notes()` accessor pending T-003 and the deferred
`speccy review` wire-up at T-004. All five SPEC open questions
are `[x]`-resolved upstream with decision references; no
silent resolution by the diff. The tests-persona blocker is a
style/test-coverage finding (REQ-001 done-when bullet 2
attribute-rejection coverage) rather than a business-intent
drift; from the user-perspective lens the slice is faithful
to REQ-001 + REQ-002. The retry fixed the prior style blocker
without re-litigating any goal or non-goal.
</review>

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

<implementer-note session="2026-05-18-spec0029-t002">
- Completed: Built a new workspace member crate at
  `speccy-core/tools/migrate_tasks_schema/` (`Cargo.toml`,
  `src/lib.rs`, `src/main.rs`, `README.md`,
  `tests/migration.rs`). The transitional state machine in
  `lib.rs` walks the source line-by-line tracking task
  boundaries and four nested-element kinds; legacy bullets at
  column 0 (`- Implementer note (session-X):`,
  `- Review (persona, verdict[, retry]): prose`,
  `- Retry: prose`) trigger a `Legacy(kind)` state that
  dedents continuation lines (2-space prefix → strip) until
  a column-0 line terminates the block, at which point a
  close tag is emitted and the terminating line is
  re-dispatched. Already-XML nested elements
  (`<task-scenarios>`, `<implementer-note>`, `<review>`,
  `<retry>`) pass through verbatim, so a second pass over
  migrated output is byte-identical (idempotency). The
  `migrate_file(path)` driver re-parses the produced bytes
  through the shipped
  `speccy_core::parse::parse_task_xml` before writing; if
  re-parse fails the file is left untouched and
  `MigrateError::PostParseFailed` (carrying a boxed
  `ParseError` to keep `Result` size small) surfaces.
  Wrote 19 tests total: 9 unit tests in `lib.rs` (regex
  detectors, indented-bullet rejection, tag-name
  classification) plus 10 integration tests in
  `tests/migration.rs` covering the four required scenarios
  (legacy → XML conversion with byte preservation of
  `<task-scenarios>`/`Suggested files:`/free prose; output
  parses cleanly under the shipped parser yielding the
  expected `body_items` typed sequence; already-migrated
  fixture is byte-identical pass-through; freshly-decomposed
  task with no body items passes through unchanged) plus
  the four `<task-scenarios>` scenarios (idempotency on two
  runs, hash neutrality on a 64-character
  `spec_hash_at_generation` value, `migrate_file` writes
  only when content differs, the migration tool does not
  surface in `speccy --help`) plus the
  `Review (persona, verdict, retry)` third-token-drop
  contract (DEC-008). Added `default-members =
  ["speccy-cli", "speccy-core"]` to the root `Cargo.toml`
  so workspace-root `cargo run -- verify` continues to
  resolve unambiguously to the `speccy` binary after the
  new `migrate-tasks-schema` binary joined the workspace.
  Dry-ran the tool end-to-end against
  `.speccy/specs/0014/TASKS.md` (7 legacy implementer-note
  bullets → 7 `<implementer-note>` elements, body content
  byte-preserved) and against
  `.speccy/specs/0016/TASKS.md` (77 new XML elements,
  0 surviving legacy bullets, idempotent on the second
  run, `spec_hash_at_generation` byte-identical) in
  `/tmp/` so the workspace stayed clean.
- Undone: T-003 (apply the migration to the in-tree corpus,
  migrate `speccy report`'s retry counting from
  `Task.notes()` to `BodyItem::Retry` filtering, and
  remove `Task.notes()`), T-004 (wire
  `redact_implementer_notes` into `speccy review`'s
  `{{task_entry}}` substitution), T-005 (writer-side skill
  prompts) all stay out of scope per the original task
  slicing. The migration tool exists and is tested but is
  not yet run against the in-tree corpus.
- Commands run: `cargo build -p migrate-tasks-schema`;
  `cargo test -p migrate-tasks-schema`;
  `cargo test --workspace`;
  `cargo clippy --workspace --all-targets --all-features`
  (modulo the carried-forward `result_large_err` against
  `ParseError` per `<assumptions>`);
  `cargo +nightly fmt --all --check`;
  `cargo deny check`;
  `cargo run --quiet -- verify`;
  dry-run sanity: `cargo run -p migrate-tasks-schema -- /tmp/<copy>.md`
  against SPEC-0014 and SPEC-0016 TASKS.md in temp paths
  (twice each to confirm second run is `no change`).
- Exit codes: 0; 0 (19 passed); 0; 0 modulo `result_large_err`
  carry (47 errors, all `ParseError`-shaped in
  `speccy-core/src/parse/**`, same shape as pre-T-001 + the
  T-001 carry-forward); 0; 0 (advisories/bans/licenses/sources
  ok); 0 (29 specs, 154 requirements, 192 scenarios,
  0 errors, 0 warnings, 48 info); 0/0 (`migrated`/`no change`
  on each dry-run pair).
- Discovered issues: Adding the new workspace binary made
  workspace-root `cargo run -- verify` ambiguous between
  `speccy` and `migrate-tasks-schema`. Fixed in this commit
  by setting `default-members = ["speccy-cli", "speccy-core"]`
  at workspace level; the migration tool is explicitly
  excluded from default-members and is invoked via
  `cargo run -p migrate-tasks-schema -- ...` per its README.
  Also: the SPEC's examples (SPEC.md:277 and SPEC.md:132)
  are inconsistent about whether the `session` attribute
  value preserves or strips the legacy `session-` prefix
  (`session="session-1"` vs `session="s1"`); chose to strip
  the prefix during migration because the new element's
  attribute name IS `session`, making the prefix redundant.
  Every legacy session value in the in-tree corpus matches
  `^session-` so the strip is unambiguous, and the regex
  pinning that prefix means a malformed legacy bullet (no
  prefix) falls through to verbatim emission rather than
  being silently mis-migrated. This is asserted by the
  `rejects_implementer_note_without_session_prefix` unit
  test. The SPEC does not constrain the format of `session`
  so either choice is in-spec; the strip choice is
  forward-compatible (if a future caller wants to preserve
  the legacy identity, they can prefix `session-` back).
- Procedural compliance: no shipped skill files touched;
  the `speccy-work` / `speccy-implement` prompt
  instructions matched the task as decomposed. The legacy
  markdown-bullet form is used for THIS implementer note
  (T-005 retires the convention from shipped skill
  prompts; T-003's migration sweeps this bullet — and
  every other in-tree TASKS.md — into the new
  `<implementer-note session="...">` XML form using the
  tool built in this slice).
</implementer-note>
<review persona="security" verdict="pass">
private one-shot CLI tool, no
network/shell-out/deserialization, no secret handling; reads and
writes only user-supplied paths via `fs-err`. `unsafe_code = "forbid"`
inherited from the workspace; production code carries no
`unwrap`/`panic`/`indexing_slicing` (the three `OnceLock` regex
`.unwrap()`s are guarded by `#[expect(... reason = ...)]` against
compile-time literal patterns with unit-test coverage, matching the
codebase's `result_large_err` carry-forward convention). Regexes
(`speccy-core/tools/migrate_tasks_schema/src/lib.rs:411,425,435`)
are anchored and use negated character classes — no ReDoS surface.
`migrate_file` refuses to write output it cannot re-parse under the
shipped `parse_task_xml` (`lib.rs:92`), preventing corrupted-corpus
states. Tool is explicitly excluded from `default-members` and not a
`speccy` subcommand, so it cannot be reached via the shipped CLI
surface; `binary_does_not_appear_in_speccy_cli_subcommands`
(`tests/migration.rs:518`) statically asserts this.
</review>
<review persona="tests" verdict="pass">
the 10 integration tests in
`speccy-core/tools/migrate_tasks_schema/tests/migration.rs` plus 9
unit tests in `src/lib.rs` exercise the real `migrate`, `migrate_file`,
and shipped `parse_task_xml` production paths with zero mocking, and
cover all six `<task-scenarios>` for T-002. `migrated_output_parses_via_shipped_task_xml`
(`migration.rs:132`) anchors the ordering contract by reading
`body_items[0/1/2]` with explicit `BodyItem::ImplementerNote`,
`BodyItem::Review { verdict: ReviewVerdict::Blocking, .. }`, and
`BodyItem::Retry` variant matches — so a regression that emits the
three new elements in the wrong order would fail. `migration_tool_is_idempotent_on_two_runs`
(`migration.rs:250`) asserts both `second == first` AND `first != src`,
catching a degenerate no-op migration. `migrate_file_writes_only_when_content_differs`
(`migration.rs:436`) writes a real tempfile, reads it back, and
verifies the `Outcome::Migrated → Outcome::Unchanged` transition
across two runs. `legacy_review_with_retry_annotation_drops_third_token`
(`migration.rs:485`) pins the DEC-008 third-token drop with a
negative `!migrated.contains("retry\"")` assertion that catches
attribute leakage. `rejects_implementer_note_without_session_prefix`
(`lib.rs:460`) and `rejects_indented_legacy_bullet` (`lib.rs:501`)
cover the negative-classification edges. Noting two soft spots that
don't block: (1) `binary_does_not_appear_in_speccy_cli_subcommands`
(`migration.rs:519`) is a static string-grep on `speccy-cli/Cargo.toml`,
not an actual `speccy --help` parse — the in-test comment acknowledges
this; the scenario's "or the equivalent CLI inspection" wording
accepts it but a runtime `assert_cmd` against `speccy --help` would
be stronger. (2) `MigrateError::PostParseFailed` (`lib.rs:92,49`) has
no test that drives the branch with a hand-crafted input that
migrates into invalid XML — the defensive guard is currently
exercised only on the happy path. Neither gap is blocking for this
slice; the happy-path coverage and the strict ordered-variant
assertion give enough adversarial signal to catch the realistic
regressions a migration tool faces.
</review>
<review persona="business" verdict="pass">
the slice delivers REQ-006's T-002 promise
— a private `speccy-core/tools/migrate_tasks_schema/` binary crate
carrying a both-forms transitional parser, idempotent re-run,
hash-neutral on `spec_hash_at_generation`, and a parse-verify gate
before writing (`src/lib.rs:92`). All six `<task-scenarios>` map to
concrete tests in `tests/migration.rs`: ¶1 legacy→XML conversion
with byte-preserved `<task-scenarios>`/`Suggested files:`
(`legacy_bullets_convert_to_canonical_xml_form`), ¶2 typed
`body_items` shape (`migrated_output_parses_via_shipped_task_xml`),
¶3 already-migrated byte-identical
(`already_migrated_input_is_byte_identical_output` +
`migrate_file_writes_only_when_content_differs`), ¶4 twice-is-no-op
(`migration_tool_is_idempotent_on_two_runs`), ¶5 hash neutrality
(`migration_is_hash_neutral`), ¶6 not-a-speccy-subcommand
(`binary_does_not_appear_in_speccy_cli_subcommands`). Scope stays
inside T-002: tool exists and is tested, not yet run against the
corpus (T-003), no consumer rewiring, no skill-prompt changes,
no `Task.notes()` removal. SPEC non-goals respected — no
`speccy migrate-tasks-schema` subcommand (workspace
`default-members` excludes the tool, `speccy-cli/Cargo.toml` has
no reference), transitional parser is private to the tool, no
`schema_version` bump, no `<implementer-note>` body strip
(legacy sub-bullet payload survives dedented as top-level bullets
per `legacy_bullets_convert_to_canonical_xml_form:113-118`). One
judgment call worth surfacing for the T-003 review: the
implementer silently resolved a SPEC ambiguity by stripping the
legacy `session-` prefix from the captured identifier
(`SPEC.md:277` shows `session="session-1"` while the new
TASKS.md uses `session="s1"`); DEC-004 places no format
constraint on `session` so the choice is in-spec and
forward-compatible, and it is locked behind
`rejects_implementer_note_without_session_prefix`
(`src/lib.rs:460`). Informational, not blocking.
</review>
<review persona="style" verdict="pass">
conventions match the codebase — the
`OnceLock<Regex>` factory pattern with
`#[expect(clippy::unwrap_used, reason = "...")]`
(`lib.rs:381-436`) mirrors `speccy-core/src/workspace.rs:589`,
`task_lookup.rs:290`, and `prompt/id_alloc.rs:71`; the thiserror
enum with `#[source]` plus `Box<ParseError>` keeps `Result` size
down per the existing `result_large_err` carry; tests use
`.expect("descriptive")` everywhere (no `unwrap`/`panic!` in
production or test paths); `camino::Utf8Path` + `fs-err` per the
workspace rules; module/public-item docs are populated; lints
inherit via `[lints] workspace = true`. One nit, non-blocking:
`eol_for_close` (`lib.rs:314-325`) is a dead conditional — both
arms of the `if observed_eol.is_empty()` branch return the
literal `"\n"`, so the function is equivalent to
`const fn _ -> &'static str { "\n" }`. Clippy doesn't trip on it
(the lint is `branches_sharing_code`, nursery), but the signature
and call site (`process_line:191`) read as if the EOL adapts to
observed input when it doesn't. Either inline `"\n"` at the call
site and delete the function, or make the branching real (e.g.
return `observed_eol` when non-empty, `"\n"` when empty). Worth
a follow-up touch alongside T-003's cleanup pass, not a merge
blocker.
</review>

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

<implementer-note session="2026-05-18-spec0029-t003">
- Completed: Replaced `count_retries` in `speccy-cli/src/report.rs`
  with `task.body_items.iter().filter(|item| matches!(item,
  BodyItem::Retry { .. })).count()`. Removed the `RETRY_PREFIX`
  constant and the legacy markdown-bullet-scan path. Updated the
  module docblock to reflect the new derivation. Rewrote the
  `report.rs` unit tests to construct `Task` values with typed
  `body_items` (`implementer_note`, `review`, `retry` test helpers
  parallel to the production variants), replacing the legacy
  `task_with_notes` markdown-string builder; tests cover positive
  count, no-`Retry` zero case, the adversarial case where an
  `<implementer-note>`/`<review>` body literally contains the string
  `Retry:` (must NOT count), and the singular/plural retry summary
  rendering. Deleted `Task.notes()` (and its docblock) from
  `speccy-core/src/parse/task_xml/mod.rs`. Updated
  `speccy-cli/tests/report.rs`'s `convert_legacy_to_xml` legacy
  fixture builder to translate `- Implementer note (session-X):`,
  `- Review (persona, verdict): ...`, and `- Retry: ...` bullets
  into the corresponding `<implementer-note>`, `<review>`, and
  `<retry>` XML elements so the existing CHK-003/CHK-004 integration
  tests retain identical end-to-end coverage post-migration. Ran the
  migration tool from `speccy-core/tools/migrate_tasks_schema/`
  against all 29 in-tree `.speccy/specs/*/TASKS.md` files; first
  pass converted SPEC-0014, 0016, 0017, 0018, 0019, 0020, 0022,
  0023, 0024, 0025, 0026, 0027, 0028, 0029 (the rest carried no
  legacy bullets and were `no change`). Re-ran the migration against
  every spec to confirm idempotency: zero files modified on the
  second pass. The migration commit must reference SPEC-0029 in its
  message per REQ-006 done-when bullet 7 (this is the SPEC ship
  workflow's job; the migration changes sit alongside the T-003
  source changes in this task's working tree).
- Undone: T-004 (wire `redact_implementer_notes` into `speccy review`'s
  `{{task_entry}}` substitution and add the persona-uniformity and
  `Suggested files:` byte-preservation integration tests) and T-005
  (writer-side skill prompts) remain out of scope per the original
  task slicing. The pre-existing `clippy::result_large_err` warning
  against `speccy_core::error::ParseError` stays carried forward per
  this SPEC's `<assumptions>`.
- Commands run: `cargo build --workspace --tests`;
  `cargo test --workspace` (post-rewire baseline);
  `cargo run --release -p migrate-tasks-schema -- .speccy/specs/*/TASKS.md`
  (initial sweep);
  `cargo run --release -p migrate-tasks-schema -- <same paths>`
  (idempotency re-run);
  `cargo test -p migrate-tasks-schema` (post-tool-patch);
  `cargo run --release -p migrate-tasks-schema -- .speccy/specs/0018-remove-check-execution/TASKS.md`
  (re-migrate SPEC-0018 with the patched tool);
  `cargo test --workspace`;
  `cargo clippy --workspace --all-targets --all-features`
  (modulo the carried-forward `result_large_err`);
  `cargo +nightly fmt --all --check`;
  `cargo deny check`;
  `cargo run --release --bin speccy -- verify`;
  `grep -rEn '^- (Review \(|Retry: |Implementer note \(session-)' .speccy/specs/*/TASKS.md`
  (post-migration verification — zero matches expected).
- Exit codes: 0; 0; 0 (14 migrated, 15 no change); 0 (0 migrated,
  29 no change); 0 (11 unit + 10 integration); 0 (1 migrated);
  0 (all suites green); 0 modulo carried-forward `result_large_err`
  (46 errors, all `ParseError`-shaped); 0; 0
  (advisories/bans/licenses/sources ok); 0 (29 specs, 154
  requirements, 192 scenarios, 0 errors, 0 warnings, 48 info);
  0 (zero matches surviving).
- Discovered issues: The initial migration sweep faithfully
  preserved a writer-side anomaly in SPEC-0018's pre-migration
  TASKS.md: every reviewer note for T-003/T-004/T-005/T-006 was
  written as a 2-space-indented sub-bullet of the parent
  `- Implementer note (session-X):` (rather than as a top-level
  column-0 `- Review (...)` bullet, which is the convention every
  other spec uses). After dedenting those continuation lines, the
  reviewer notes ended up as `- Review (persona, verdict): ...`
  lines inside the `<implementer-note>` body. That violates two
  things at once: (1) REQ-006 done-when's "zero `^- Review (` matches"
  contract; (2) the SPEC's intent — those misnested reviews would be
  redacted along with the implementer note by `redact_implementer_notes`
  in T-004, losing reviewer signal that should remain visible across
  personas. Patched the migration tool's state machine: when a
  2-space-indented continuation line, after dedent, matches one of
  the three legacy bullet patterns, terminate the surrounding legacy
  block and re-dispatch the dedented line so it gets lifted to a
  top-level XML element. Added the
  `nested_review_inside_implementer_note_lifts_to_top_level` and
  `is_legacy_bullet_detects_all_three_kinds` unit tests in
  `speccy-core/tools/migrate_tasks_schema/src/lib.rs` to lock the
  behavior. Re-migrated SPEC-0018 from its pre-migration source to
  exercise the patched tool against the in-tree anomaly; result:
  21 previously-misnested `Review (...)` notes are now top-level
  `<review persona="..." verdict="...">` elements, zero legacy
  bullets survive in any TASKS.md, and the idempotency property
  still holds on a second pass across all 29 files. This is a
  T-002-tool-shaped fix landed inside T-003's scope rather than a
  retroactive T-002 amendment, because (a) the friction surfaced
  only when T-003 ran the tool against the real in-tree corpus,
  and (b) the patch is a structural correctness improvement that
  the SPEC-0029 done-when contract requires either way.
- Procedural compliance: edited
  `speccy-core/tools/migrate_tasks_schema/src/lib.rs` (the migration
  tool from T-002) to lift misnested legacy bullets out of an
  enclosing implementer-note continuation. The tool is private to
  this SPEC (not a shipped skill / not a `speccy` subcommand), so
  no `skills/` files were touched; flagging here under procedural
  compliance because the fix lives in a previous task's domain.
  The legacy markdown-bullet form is intentionally used for this
  implementer note itself — T-005 retires the convention from
  shipped writer-side skill prompts, and T-003's migration tool
  (now patched) will sweep this note into the new XML form on the
  next pass.
</implementer-note>

<review persona="tests" verdict="pass">
Slice-scenario coverage is solid end-to-end. The retry-counting refactor in
`speccy-cli/src/report.rs:255-260` is exercised by a triad of unit tests that
together kill the obvious mutants: `count_retries_counts_retry_body_items`
(positive, mixed-kind, 7 items → 2 retries),
`count_retries_zero_when_no_retry_variant` (negative), and the load-bearing
adversarial test `count_retries_ignores_non_retry_variants_with_retry_in_body`
(`speccy-cli/src/report.rs:432-443`) which embeds the literal substring `Retry:`
inside both an `<implementer-note>` body and a `<review>` body and asserts the
count stays at 0 — this catches any regression to a body-text scan and verifies
the typed-variant gate is the only counter. The end-to-end retry rendering is
re-pinned by `retry_count_appears_in_rendered_retry_summary` after the legacy
`convert_legacy_to_xml` helper was correctly updated to translate `- Retry: ...`
bullets into `<retry>` elements (`speccy-cli/tests/report.rs:90-91`), so the
CHK-003 integration assertion `- T-001: 2 retries` still exercises a real
fixture path. The removal of `Task.notes()` is confirmed by manual grep (zero
matches across `speccy-core` / `speccy-cli` non-test source) and the migration
of the in-tree corpus is asserted by `every_in_tree_tasks_md_parses_and_has_populated_scenarios`
(every TASKS.md parses under the new whitelist); 89 `<implementer-note session=`
matches across 11 migrated spec files satisfy the carrier-conversion contract.
Singular/plural retry-summary edges (`format_retry_summary_uses_singular_for_one_retry`)
adversarially assert `!summary.contains("1 retries")`, so a regression to
`"{count} retries"` everywhere would fail loudly. `cargo test --workspace` is
green (every suite reports 0 failed). One non-blocking quality smell for
follow-up: `retry_count_exact_prefix_only_lowercase_ignored`
(`speccy-cli/tests/report.rs:472-501`) is now vacuous — the legacy strings
`Retry on bcrypt cost`, `retry: lowercase ignored`, and `Retried: past tense`
fall through `convert_legacy_to_xml`'s `else` arm and end up as plain `- ` markdown
bullets that the parser never reads, so the assertion `"- T-001: 0 retries"`
would pass with arbitrary garbage in those slots; the test no longer
discriminates prefix shapes since prefix discrimination is no longer a
production concern. The adversarial work it nominally guards is now correctly
covered at the unit layer by `count_retries_ignores_non_retry_variants_with_retry_in_body`,
so deletion (or a rewrite that embeds `Retry:` inside an `<implementer-note>`
body via the XML fixture path) would be cleaner, but the property under test
is not actually at risk.
</review>

<review persona="security" verdict="pass">
T-003 introduces no new auth, secret, network, shell-exec, or path-traversal
surface — the diff is parser/renderer plumbing plus a migration of an
in-tree corpus. Closed-set validation for `<review verdict>` and
`<review persona>` (`speccy-core/src/parse/task_xml/mod.rs:702-732`) and
the dedicated `ParseError` variants for missing `session`, empty
implementer-note body, invalid verdict, and invalid persona
(`speccy-core/src/error.rs:531-595`) tighten input validation versus the
removed `Task.notes()` markdown scanner. The retry-counting migration in
`speccy-cli/src/report.rs:255-260` is strictly more conservative than the
old `RETRY_PREFIX.starts_with` scan: an `<implementer-note>` or `<review>`
body literally containing `Retry:` no longer inflates the count
(covered by `count_retries_ignores_non_retry_variants_with_retry_in_body`).
Error messages surface local `Utf8PathBuf`, task id, and byte offset
only — acceptable for a single-tenant developer CLI; no remote disclosure
boundary exists. One defense-in-depth note (non-blocking) for follow-up:
`redact_implementer_notes` in `speccy-core/src/parse/task_xml/mod.rs:822-836`
relies on `<implementer-note>` open and close tags being line-isolated
(it `trim_start`s each line and matches a prefix). A hand-edited or
non-canonical TASKS.md placing `<implementer-note session="x">leak</implementer-note>`
on a single line would bypass redaction and the body would render into
the reviewer prompt. The current writer-side flow (canonical XML render
via `task_xml::render`'s `push_element_block`, plus the migration tool's
canonical emission) never produces that shape, and the rustdoc on
`redact_implementer_notes` documents the contract; flagged here so a
future hardening pass can switch to a structural re-render driven by
`task.body_items` span metadata if the inline-tag risk ever materialises
(e.g. once `<implementer-note>` payload is sourced from user content
rather than skill-emitted canonical form).
</review>

<review persona="business" verdict="pass">
The diff delivers what REQ-006 / REQ-007 promise. `Task.notes()` is gone
(`grep "fn notes\|Task::notes\|\.notes()" speccy-core/src speccy-cli/src`
returns zero matches); `speccy-cli/src/report.rs:255-260` derives retries
from `body_items.iter().filter(matches!(BodyItem::Retry { .. }))` per
REQ-007 done-when bullet 2; the in-tree corpus is fully migrated
(`grep -rEn '^- (Implementer note \(session-|Review \(|Retry: )'
.speccy/specs/*/TASKS.md` returns zero matches per REQ-006 done-when
bullets 1-3); 11 TASKS.md files now carry `<implementer-note session=>`
elements per REQ-006 done-when bullet 4; `speccy verify` exits clean
(0 errors, 0 warnings, 48 info) per REQ-006 done-when bullet 5.
Retry-count semantic preservation (REQ-007 user-facing scenario CHK-007
and `<task-scenarios>` bullet 5 here) holds 1:1 on every spot-checked
spec: SPEC-0016 (2→2), SPEC-0018 (2→2), SPEC-0020 (4→4), SPEC-0025
(2→2), SPEC-0027 (0→0) — pre-migration `grep -c "^- Retry: "` against
the HEAD blob matches post-migration `grep -c "<retry>"` in the
working tree. The `count_retries_ignores_non_retry_variants_with_retry_in_body`
test (`speccy-cli/src/report.rs:431-444`) covers exactly the adversarial
case business cares about: a typed-variant filter cannot be tricked by
an `<implementer-note>` or `<review>` body that mentions "Retry:" in
prose, which the removed `RETRY_PREFIX.starts_with` scan could in
principle have miscounted. Non-goals respected: no `schema_version`
bump, no transitional grandfathering parser shipped (the transitional
logic stays private to `speccy-core/tools/migrate_tasks_schema/`), no
new CLI subcommand, `Task.suggested_files()` left untouched
(`Suggested files:` bullet counts preserved in spot checks — SPEC-0020:
7→7). The mid-T-003 patch to the migration tool that lifts misnested
2-space-indented reviewer notes out of enclosing implementer-note
continuations (called out under "Discovered issues" in the implementer
note) is the right move for the user story REQ-007 / REQ-003 jointly
serve: 21 previously-misnested `Review (persona, verdict)` lines in
SPEC-0018 are now top-level `<review persona="..." verdict="...">`
elements that the SPEC-0029 reviewer can still see across personas,
instead of getting redacted along with the enclosing implementer note
in T-004. One non-blocking observation for the ship task, not for
T-003: REQ-006 done-when bullet 7 ("the migration commit references
SPEC-0029 in its message") is a property of the future commit message
and can only be verified when the SPEC ships — the implementer note
explicitly defers this to the SPEC ship workflow, which is the right
call.
</review>

<review persona="style" verdict="pass">
Diff is style-clean against AGENTS.md and the `.claude/rules/rust/`
set. Naming and shape match existing project patterns: `ReviewVerdict`
mirrors `TaskState` (closed enum, `as_str` / `from_str`, parallel
`ALLOWED_*` slice); `BodyItem` variants reuse the existing
`ElementSpan` and `(String, String)` attribute carriers;
`build_implementer_note` / `build_review` / `collect_task_children`
(`speccy-core/src/parse/task_xml/mod.rs:579-732`) follow the
established `build_task` helper style; the new test helpers
(`task_with_body_items`, `implementer_note`, `review`, `retry` in
`speccy-cli/src/report.rs:286-340`) cleanly retire `task_with_notes`
without inventing parallel scaffolding. Lint compliance holds:
`cargo clippy --workspace --all-targets --all-features` produces only
the carried-forward `clippy::result_large_err` errors against
`speccy_core::error::ParseError` (the four new variants are
small-payload and do not enlarge the worst-case variant past the
pre-existing 128-byte `InvalidMarkerAttributeValue`), exactly the
carry-forward the SPEC `<assumptions>` and the implementer note
describe; `cargo +nightly fmt --all --check` is clean;
`cargo test --workspace` is green. The one new lint suppression is
the correctly-shaped `#[expect(clippy::should_implement_trait,
reason = "...")]` on `ReviewVerdict::from_str`
(`speccy-core/src/parse/task_xml/mod.rs:142-145`) — mirrors
`TaskState::from_str`'s `Option<Self>` shape, no `#[allow]`
introduced anywhere in the production diff. Dead code from the
migration is properly purged: `RETRY_PREFIX` constant and the old
`task_with_notes` helper removed in lockstep with `Task::notes()`.
Minor non-blocking nits, none worth blocking on: (1)
`strip_nested_body_blocks` (`speccy-core/src/parse/task_xml/mod.rs:873-904`)
does `format!("</{name}>")` and two `format!("<{name} ...")`
allocations per line per element name (up to 9 small heap allocations
per body line in the worst case); the original
`strip_nested_task_scenarios` had the same shape and rendering is not
hot, so this matches surrounding style, but a pre-computed
`[(&'static str, &'static str, &'static str); 4]` table of
`(open_attr, open_close, close)` prefixes would eliminate the
per-line work if a future pass wants to tighten it. (2)
`redact_implementer_notes` (`speccy-core/src/parse/task_xml/mod.rs:937-961`)
accepts the attribute-less `<implementer-note>` open shape even
though the parser rejects it via `MissingImplementerNoteSession`;
defensible as defence-in-depth and consistent with
`strip_nested_body_blocks`'s symmetry, so not worth changing. (3) The
new test file `speccy-core/tests/task_xml_body_items.rs:1-4` uses
`#![allow(clippy::expect_used, reason = "...")]` rather than the
`#![expect(...)]` form AGENTS.md prefers, but ~22 other test files
under `speccy-core/tests/` and `speccy-cli/tests/` use the same
`#![allow]` shape (and `allow-expect-in-tests = true` in
`clippy.toml` means the attribute is redundant either way), so
flagging for awareness rather than blocking — the same nit would
land against the majority of the existing test suite.
</review>

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

<implementer-note session="2026-05-18-spec0029-t004">
- Completed: switched `speccy-cli/src/review.rs:132` from the raw
  `location.task_entry_raw.clone()` to
  `redact_implementer_notes(&location.task_entry_raw, location.task)`,
  added a `use speccy_core::parse::redact_implementer_notes;`
  import, and inlined a comment naming SPEC-0029 REQ-003 / REQ-004
  and DEC-002 (silent redaction). Added a new integration test file
  at `speccy-cli/tests/review_redaction.rs` carrying five tests:
  (1) full-mixed-body redaction shape — every `<review>` /
  `<retry>` / `<task-scenarios>` body and the `Suggested files:`
  line pass through verbatim, every byte of both
  `<implementer-note>` bodies is removed, and the six implementer
  sub-bullet labels (`Commands run:`, `Exit codes:`, etc.) are
  absent from the redacted entry; (2) no placeholder marker
  (`redacted`, `withheld`, `notes omitted`, `notes hidden`,
  `implementer notes hidden`) survives in the rendered entry —
  pinning DEC-002; (3) byte-identical `## Task entry` slice across
  all six `speccy_core::personas::ALL` personas — pinning the
  uniform-redaction contract; (4) no-op redaction when the task
  carries no `<implementer-note>` element; (5) `speccy implement`
  rendered against the same fixture-task carries
  `<implementer-note` and both note bodies verbatim — pinning the
  REQ-004 carve-out. Confirmed by code-read that
  `speccy-cli/src/implement.rs:107` and `speccy-cli/src/report.rs`
  carry no `redact_implementer_notes` call; the only review-side
  change is the one-line substitution swap. Smoke-tested live on
  `speccy review SPEC-0029/T-003 --persona business` against the
  in-tree corpus: zero line-anchored `<implementer-note` tags in
  the rendered review (down from 2 in the rendered implement
  prompt), while the four prose mentions of the literal substring
  `<implementer-note session=` inside sibling `<review>` element
  bodies pass through verbatim — confirming the redactor is
  element-scoped and does not over-strip on text-substring matches.
- Undone: (none for this task. T-005 — writer-side skill prompt
  updates — and T-006 — REPORT.md + ship — remain pending and were
  out of scope here.)
- Commands run:
  1. `cargo test -p speccy-cli --test review_redaction`
     (pre-switch baseline, expected red on the redaction-shape
     assertion)
  2. `cargo test -p speccy-cli --test review_redaction`
     (post-switch, all 5 green)
  3. `cargo test --workspace`
  4. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  5. `cargo +nightly fmt --all` then `cargo +nightly fmt --all --check`
  6. `cargo run --quiet --bin speccy -- verify`
  7. Smoke check: `cargo run --quiet --bin speccy -- review SPEC-0029/T-003 --persona business`
     and `cargo run --quiet --bin speccy -- implement SPEC-0029/T-003`,
     grepping for `<implementer-note` and `<implementer-note session=`.
- Exit codes: 1 (expected: redaction not yet wired), 0, 0,
  46-result-large-err errors (carried forward from SPEC-0026 T-003
  per SPEC-0029 assumptions; no new clippy diagnostics added by
  T-004), 0, 0, 0.
- Discovered issues: the integration test's first cut keyed its
  task-entry slice on the `## Task entry` markdown heading and the
  next `## ` heading, but the task body itself carries a
  `## T-NNN: <title>` heading at line 2 — so the slice terminated
  one line into the body and every assertion saw an empty entry.
  Recovered by anchoring on the `<task id="..." state=` open tag
  through the matching `</task>` close tag, which is exactly the
  span the renderer substitutes. The first `<task id=` occurrence
  in the rendered prompt is the descriptive prose literal
  `<task id="{{task_id}}">...</task>` (no state attribute);
  filtered by requiring a `state=` attribute on the open tag.
  Worth filing as F-N if the test scaffolding for other slice-level
  redaction work grows — for now the helper is local to this test
  file and unlikely to recur.
- Procedural compliance: (none — no shipped skill files needed
  edits during this task. The writer-side skill prompts that emit
  `<implementer-note>` and the orchestrator that writes `<retry>`
  are T-005's scope, not T-004's.)
</implementer-note>

<review persona="security" verdict="pass">
redaction is correctly scoped to the
review-prompt renderer only. `speccy-cli/src/review.rs:137`
routes `task_entry_raw` through `redact_implementer_notes`;
`speccy-cli/src/implement.rs:107` is byte-identical pre/post
per REQ-004; `speccy-cli/src/report.rs` carries no helper call
and consumes typed `body_items` rather than inlining note
bytes — the three trust boundaries match SPEC-0029's
contract. No new I/O, no new dependencies, no
credential/path/log surface introduced; no information-leak
vector through error messages (the helper is infallible and
emits only a `String`). Integration tests in
`speccy-cli/tests/review_redaction.rs` directly pin the
six-sub-bullet labels (`Commands run:`, `Exit codes:`,
`Discovered issues:`, `Procedural compliance:`, `Undone:`,
`Completed:`) as forbidden substrings inside the rendered
`## Task entry` and assert byte-identity across all six
personas — closing the asymmetric-leak vector DEC-003 names.
Note for posterity (non-blocking, not in scope here): the
line-based redactor at
`speccy-core/src/parse/task_xml/mod.rs:937` keys on
`trim_start().starts_with("<implementer-note ")` /
`"</implementer-note>"`, which assumes the canonical multi-line
rendering (open tag on its own line, body, close on its own
line). The parser also accepts an inline single-line form;
if a non-canonical TASKS.md ever lands (hand-edited, or a
future writer-skill regression), the state machine would
either skip the close or get stuck swallowing subsequent
lines. Acceptable for v1 because the migration script renders
canonical form and the writer-side skills under T-005 will
too — but worth filing as F-N to re-render through
`task_xml::render` (or anchor on `BodyItem::ImplementerNote`
spans) before someone hand-edits a TASKS.md and silently
defeats the contract. Per AGENTS.md's feedback-not-enforcement
framing this is a robustness gap, not a confidentiality
bypass; an adversarial reviewer agent can read TASKS.md
directly via its Read primitive regardless.
</review>

<review persona="business" verdict="pass">
the diff satisfies the user-facing
contract REQ-003 and REQ-004 name. `speccy-cli/src/review.rs`
switches `{{task_entry}}` to the redacted projection at the
one site SPEC-0029 prescribes (line 137, formerly 132); the
helper lives in `speccy-core::parse::redact_implementer_notes`
satisfying REQ-003 done-when bullet 2 (helper in `speccy-core`,
not `speccy-cli`); it takes no `Persona` parameter, honouring
DEC-003 (uniform across personas) and the "no asymmetric
persona behaviour" non-goal; and it inserts no placeholder
prose, honouring DEC-002 and the "no placeholder marker"
non-goal. `speccy-cli/src/implement.rs:107` is byte-identical
pre/post, serving the "as the implementer on a retry pass…"
user story; `speccy-cli/src/report.rs` carries no
`redact_implementer_notes` call, serving the "as the report
author running `speccy ship`…" user story (REPORT.md still
derives skill updates from `Procedural compliance` lines).
The five integration tests in
`speccy-cli/tests/review_redaction.rs` pin every business-level
invariant from the task's `<task-scenarios>` block: redaction
shape on mixed body items (`<review>`, `<retry>`,
`<task-scenarios>`, free prose, and the `Suggested files:`
bullet all survive verbatim while both `<implementer-note>`
bodies and the six sub-bullet labels are zero-byte stripped);
byte-identical `## Task entry` across all six personas in
`speccy_core::personas::ALL`; no-op behaviour when no
`<implementer-note>` exists; absence of placeholder markers
(`redacted`, `withheld`, `notes omitted`, etc.); and the
REQ-004 carve-out asserting `speccy implement` carries
`<implementer-note` verbatim. No new CLI flag, no
`schema_version` bump, no scope creep beyond the one-line
substitution swap and the dedicated test file — every diff
line traces back to a REQ-003 or REQ-004 done-when bullet.
Two minor coverage notes that are not blocking: (1) the
placeholder-marker scan covers `notes hidden` and
`implementer notes hidden` but not the bare token `hidden`
the user-facing scenario also names — the
`redacted`/`withheld`/`notes omitted` triad already enforces
the silent-redaction contract in practice, and the missing
bare `hidden` is unlikely to ever surface in a renderer that
emits nothing; (2) the tests assert each preserved body item
is present but don't pin "in source order" as a separate
invariant — order is implicit in the renderer (verified by
the round-trip tests in REQ-002's slice) and adding a
positional check here would gild the lily. No business-level
drift between SPEC and diff. Open questions remain resolved;
the changelog row is honoured.
</review>

<review persona="style" verdict="pass">
the slice matches project conventions
cleanly. `speccy-cli/src/review.rs:27` slots
`redact_implementer_notes` into the alphabetical
one-import-per-line `speccy_core::*` block; the call-site
comment at `:129-136` mirrors the SPEC-0023 reference style
already in that function; the `(&str, &Task)` parameter shape
follows the borrow-when-you-can guidance and matches sibling
helpers in `speccy-core::parse::task_xml`. No new
`#[allow]`/`#[expect]` suppressions added in production code
and no `unwrap`/`expect`/`panic` slipped in. The new
`speccy-cli/tests/review_redaction.rs` reuses the existing
inner-attribute pattern (`expect_used`/`unwrap_in_result` with
`reason = "..."`) that every other integration-test file under
`speccy-cli/tests/` already uses, and its helper layout
(`mod common`, `capture_*_stdout`, `indoc`-based fixtures,
`seed_*_workspace`) mirrors `tests/review.rs` and
`tests/implement.rs`. Naming (`task_entry_section`,
`tasks_md_full_mixed_body`, `tasks_md_no_implementer_note`) is
consistent with the project's snake_case + intent-describing
convention. Clippy is clean on the three files this task
touches; the 46 `result_large_err` diagnostics visible in a
full workspace run are pre-existing per SPEC-0026 T-003 and
the implementer note explicitly flags them as carried forward,
not introduced. Two minor nits (non-blocking, would not gate
merge): (1) the literal `0usize` suffix on
`speccy-cli/tests/review_redaction.rs:165` is redundant — the
later `search_from + rel` resolves to `usize` via inference;
trimming to `let mut search_from = 0;` matches the project's
prevailing terseness, but the explicit form is defensible as
intent-signaling for an index variable. (2) `task_entry_section`
at `:161-188` performs several `rendered[a..b]` slices guarded
by `.find()`-derived char-boundary indices; the project's
`string_slice = "deny"` lint allows this pattern (clippy
passes), and the function is test-only, but a future
refactor to a `&str::split_once`/iterator-based parse would
be more idiomatic and clippy-future-proof — fine to leave for
a separate cleanup.
</review>

<review persona="tests" verdict="pass">
the five integration tests in
`speccy-cli/tests/review_redaction.rs` exercise the real
`speccy_cli::review::run` and `speccy_cli::implement::run`
entry points against a synthetic workspace (no mocks of the
redaction surface) and pin every slice-level invariant the
task's `<task-scenarios>` block names that maps to the review
prompt: redaction shape on mixed body items (`<review>` /
`<retry>` / `<task-scenarios>` / free prose / `Suggested files:`
pass through, both `<implementer-note>` bodies and the six
sub-bullet labels `Commands run:` / `Exit codes:` /
`Discovered issues:` / `Procedural compliance:` / `Undone:` /
`Completed:` are stripped at byte level via explicit
`!entry.contains(...)` assertions), absence of placeholder
prose (`redacted` / `withheld` / `notes omitted` / `notes
hidden` / `implementer notes hidden`) — pinning DEC-002,
`assert_eq!` byte-identity of the `## Task entry` slice across
all six `speccy_core::personas::ALL` personas — pinning the
uniform-redaction contract on the actual rendered prompt
(catches any future `Persona` parameter on the helper or
per-persona branch in `review.rs`), and the REQ-004 carve-out
asserting the rendered `speccy implement` prompt for the same
fixture-task carries `<implementer-note` plus both note bodies
verbatim. Mentally reverting the call-site swap at
`speccy-cli/src/review.rs:137` back to `task_entry_raw.clone()`
would red-line every redaction-shape assertion in
`review_prompt_redacts_every_implementer_note_body_and_preserves_other_items`,
and replacing `redact_implementer_notes` with a stub that
inserted a placeholder string would red-line
`review_prompt_has_no_placeholder_marker_indicating_redaction`
— the tests would fail in obvious ways under plausible
regressions, so this is real coverage, not mock-shaped passing.
Two non-blocking coverage observations: (1)
`review_prompt_with_no_implementer_note_renders_unchanged_task_entry`
asserts substring containment (free prose, `<task-scenarios>`,
`Suggested files:`) rather than strict byte-identity to
`task_entry_raw` — the task's `<task-scenarios>` block names
byte-identity as the contract ("byte-identical to the
unredacted `task_entry_raw` slice that the pre-SPEC code would
have substituted"); the helper at
`speccy-core/src/parse/task_xml/mod.rs:942-944` does honour
byte-identity (early-return `task_entry.to_owned()`), but the
test would still pass if a future regression inserted a
trailing newline or blank line in the no-op path. A
`prop_assert_eq!(entry, expected_raw)` against the exact raw
slice the lookup produces would tighten this; leaving as a
follow-up nit rather than blocking because the current helper
is provably byte-identical by inspection and the substring
assertions catch the more common regression class. (2) The
final `<task-scenarios>` paragraph names a `speccy report`
no-inlining invariant ("does NOT inline TASKS.md bytes…
byte-identical to its pre-SPEC behavior"); this is covered
indirectly by pre-existing tests in `speccy-cli/tests/report.rs`
(the `{{tasks_md_path}}` substitution check at line 565), so
T-004 does not need a duplicate report-prompt assertion. The
diff is acceptable as-is from the tests-persona perspective;
the byte-identity nit is a future tightening, not a blocking
finding.
</review>

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

<implementer-note session="t005-session-1">
- Completed: retired the three legacy markdown-bullet conventions
  from every shipped authoring instruction. (a)
  `resources/modules/prompts/implementer.md` task-closure step 6
  and the handoff template fenced block now instruct emitting an
  `<implementer-note session="...">…</implementer-note>` element
  block; the six required sub-bullets stay inside the element body
  as markdown payload (DEC-004). (b) The six
  `resources/modules/prompts/reviewer-{business,tests,security,style,architecture,docs}.md`
  prompts updated in lockstep — the emit-an-XML-element wording
  ("Append one `<review persona="..." verdict="...">…</review>`
  element block ... Choose `verdict="pass"` ... or
  `verdict="blocking"` ...") is byte-identical across the six
  files. (c) The `resources/modules/skills/speccy-review.md`
  source (and its `.claude` + `.agents` host wrappers under
  `resources/agents/`) instruct writing a `<retry>…</retry>`
  element block instead of a `- Retry: ...` bullet when any
  persona is `verdict="blocking"`. (d) The `description:` line in
  both speccy-review SKILL.md wrappers now references the
  `<retry>` element. Also retired one `Retry: spec amended; ...`
  authoring instruction in `resources/modules/skills/speccy-amend.md`
  and `resources/modules/prompts/tasks-amend.md` so the
  amendment-loop wording stays consistent, and renamed the
  `## Retry summary` subheading in `resources/modules/prompts/report.md`
  from `derived from inline `Retry:` notes` to
  `derived from inline `<retry>` elements`. The six reviewer
  persona bodies under `resources/modules/personas/reviewer-*.md`
  and the implementer persona under `resources/modules/personas/implementer.md`
  carry matching example blocks in the new XML form. Finally
  re-ran `speccy init --force --host claude-code` and
  `speccy init --force --host codex` so the dogfooded in-project
  `.claude/skills/`, `.codex/agents/`, and `.agents/skills/`
  trees regenerate from the updated source; the in-project
  `.claude/agents/reviewer-*.md` and `.codex/agents/reviewer-*.toml`
  files (which the renderer skip-preserves under user edits) were
  hand-edited in lockstep to match the new persona-module output.
- Undone: (none) — every bullet in `<task-scenarios>` is
  satisfied and every `<done-when>` line under REQ-005 is met by
  the diff.
- Commands run: `cargo run -q --bin speccy -- implement
  SPEC-0029/T-005` (prompt render); `cargo run -q --bin speccy --
  init --force --host claude-code` (dogfood regenerate);
  `cargo run -q --bin speccy -- init --force --host codex`
  (dogfood regenerate); `cargo test --workspace`;
  `cargo run -q --bin speccy -- verify`;
  `cargo +nightly fmt --all --check`; seven grep verifications
  per the task body's verification list (`grep -rn
  "Implementer note (session" resources/ .claude/`, `grep -rn "-
  Review (" resources/ .claude/`, `grep -rn "^- Retry:"
  resources/ .claude/`, `grep -rn "<implementer-note session="
  resources/`, `grep -rn "<review persona=" resources/`, `grep
  -rn "<retry>" .claude/skills/speccy-review/`).
- Exit codes: pass (prompt rendered), pass (regenerate),
  pass (regenerate), pass (all suites green), pass (verify
  exit 0; 0 errors, 0 warnings, 48 info, 29 specs / 154
  requirements / 192 scenarios), pass (fmt clean), pass (the
  three legacy-form greps return zero matches; the three
  new-form greps return ≥1, ≥6, and ≥1 matches respectively).
- Discovered issues: (1) `cargo test --workspace`'s first run
  failed `dogfood_outputs_match_committed_tree` because the
  in-project `.agents/skills/speccy-amend/SKILL.md` (a
  dogfooded render) was stale relative to the just-edited
  `resources/modules/skills/speccy-amend.md` source. Fixed by
  re-running `speccy init --force --host codex`; the test then
  passed. Not a bug — expected behaviour of the dogfood gate.
  (2) `cargo test --workspace`'s second run failed
  `implementer_prompt_handoff_referenced_in_task_steps` because
  my first draft of step 6 in `implementer.md` wrapped the words
  "handoff" and "template" across a newline, so the test's
  literal `task_section.contains("handoff template")` substring
  scan missed them; fixed by re-flowing step 6 so "handoff
  template" sits on one line. (3) Pre-existing
  `clippy::result_large_err` continues to fire 46 times against
  `speccy_core::error::ParseError` — carried-forward per
  SPEC-0029 Assumptions (and originally per SPEC-0026 T-003 /
  backlog F-7); out of scope for T-005, and `speccy verify`
  already exits 0 with that warning logged as info.
- Procedural compliance: (none) — no shipped skill file
  carried a friction-inducing instruction during this slice;
  the writer-side prompt updates this task ships ARE the
  surface that downstream implementers and reviewers will hit
  next, but those edits land as the deliverable rather than as
  procedural-compliance side-channel fixes.
</implementer-note>

<review persona="security" verdict="pass">
Pure writer-side prompt text edits; no auth, secrets, crypto, deps, or executable code paths touched. No injection surface introduced: `{{persona}}` substitutes a value drawn from the closed `speccy_core::personas::ALL` set (per SPEC-0029 REQ-001 / REQ-002), not user input. Adversarial-review property the broader SPEC defends is unaffected by this slice — redaction is REQ-003's job, not REQ-005's, and the writer-side instruction here continues to emit `<implementer-note>` bodies in full (the implementer prompt is documented as un-redacted under REQ-004, which is correct). One non-blocking observation: the example block in each reviewer-*.md hard-codes `verdict="pass"` (e.g. `resources/modules/prompts/reviewer-security.md:67`); a careless reviewer pattern-matching the example could rubber-stamp `pass`, but that is a review-quality / business concern, not a security one.
</review>

<review persona="business" verdict="pass">
Diff cleanly satisfies REQ-005's done-when contract and all five slice-level scenarios. Verification: (1) `resources/modules/prompts/{implementer,reviewer-*}.md` and `.claude/skills/speccy-review/SKILL.md` plus its two mirror `.tmpl` files all retire the legacy `- Implementer note (session-`, `- Review (`, and `- Retry:` bullets — zero legacy-form grep matches in `resources/` and `.claude/`. (2) New-form greps land where REQ-005 says they must: `<implementer-note session=` in `implementer.md`, `<review persona=` in all six `reviewer-*.md` prompts, `<retry>` in `.claude/skills/speccy-review/SKILL.md`. (3) Lockstep contract holds — md5sum of all six `reviewer-*.md` prompt files is byte-identical (`5beb66f5…`), so the emit-an-XML-element instruction matches across personas as the task scenarios require. (4) Implementer handoff template keeps the six sub-bullets (`Completed`, `Undone`, `Commands run`, `Exit codes`, `Discovered issues`, `Procedural compliance`) as markdown payload inside the `<implementer-note>` body, honouring DEC-004's "body stays as markdown payload" framing. (5) Non-goals respected: no `speccy implement` / `speccy report` / TASKS.md schema touch (REQ-004 scope), no per-persona configuration, no placeholder marker prose. The persona-file edits under `resources/modules/personas/` are not in the task's "Files to update" list but are coherent lockstep updates of sibling writer-side authoring instructions, not scope creep. No `<retry>` example wording divergence between `.claude/skills/speccy-review/SKILL.md` and the shared `resources/modules/skills/speccy-review.md` — both carry the same `<one-line summary…>` / `<optional bullets…>` shape, so the Codex render path stays in sync with Claude.
</review>

<review persona="style" verdict="pass">
Pure writer-side prompt-text diff; no Rust code touched, so the AGENTS.md / `.claude/rules/rust/*.md` standard-hygiene gates (clippy, fmt, deny) do not apply. The project's only style contract in scope is the slice's own lockstep-wording rule, and it holds exactly: `md5sum resources/modules/prompts/reviewer-{business,tests,security,style,architecture,docs}.md` returns identical hash `5beb66f501d750fa3cc5f39d58ac2a9e` across all six files, so the emit-an-XML-element instruction is byte-identical per the four-th `<task-scenarios>` bullet. Indentation style for the new XML element examples (four-space preformatted blocks, not triple-backtick fences) matches the surrounding `git diff "$base"...HEAD` example block in each prompt — consistent with the file's pre-existing convention. The `<implementer-note>` template body switches from two-space-indented nested sub-bullets to flat top-level `- Completed:` bullets, which is the correct shape now that the bullets live inside the element rather than under an outer list item; field labels stay verbatim so downstream greps still match. No new `#[allow(...)]` / `#[expect(...)]` suppressions, no dead imports, no naming drift (`session` / `persona` / `verdict` attribute names match the closed sets declared on the T-001 parser side). One minor consistency seam worth noting but not blocking: the prompt-file example hardcodes `verdict="pass"` (e.g. `resources/modules/prompts/reviewer-style.md:69`) while the matching persona-file example flips to `verdict="blocking"` (`resources/modules/personas/reviewer-style.md:55`); the divergence is defensible — the prompt teaches the shape, the persona demonstrates an adversarial case — and both halves of the persona pair stay internally consistent across all six reviewer flavours.
</review>

<review persona="tests" verdict="pass">
Slice's `<task-scenarios>` are grep-based by design (REQ-005 / CHK-005 explicitly contract grep substrings, not behavioural tests), and every grep the task names verifies against the working tree: `<implementer-note session=` lives in `resources/modules/prompts/implementer.md:100`; `<review persona=` lives in every one of `resources/modules/prompts/reviewer-{business,tests,security,style,architecture,docs}.md:64`; `<retry>` lives in `.claude/skills/speccy-review/SKILL.md:87` and reaches both mirror render paths via the shared `resources/modules/skills/speccy-review.md` include. Legacy-form negative greps all return zero matches across `resources/`, `.claude/`, `.agents/`, and `.codex/` — including the `Implementer note (session`, `- Review (`, and `^- Retry:` patterns the done-when contract enumerates. Lockstep contract on the six reviewer prompts holds: pairwise `diff` against `reviewer-tests.md` returns empty for all five siblings, so the emit-an-XML-element instruction is byte-identical as the fourth scenario requires. `cargo test --workspace` exits 0 (no regressions in the parser/render tests that T-001/T-002/T-004 added — `speccy-cli/tests/review_redaction.rs` and `speccy-cli/tests/report.rs` exercise the actual `<implementer-note>` / `<review>` / `<retry>` parse-render behaviour the writer-side prompts produce, which is the right test split given this slice ships no executable code). `cargo run --bin speccy -- verify` exits 0 with 29 specs / 154 requirements / 192 scenarios verified and zero errors. One observation worth naming, not blocking: there is no project-level test that locks the new XML-element authoring instructions into `skill_packs.rs` (the file already enforces placeholder presence and absence of retired tokens like `{{vision}}` / `{{persona_content}}` per SPEC-0023 / SPEC-0027, but does not yet assert `read_prompt("implementer.md").contains("<implementer-note session=")` or the lockstep-byte-identity property across the six reviewer prompts). The SPEC's CHK-005 is grep-driven so the gap is consistent with the validation philosophy, and the dogfooded `speccy-review` loop itself would surface a regression on the next pass — but the next slice that touches reviewer prompts has no compile-time guardrail. Not in scope for T-005 to add it.
</review>

</task>

</tasks>
