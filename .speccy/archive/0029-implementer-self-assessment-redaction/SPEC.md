---
id: SPEC-0029
slug: implementer-self-assessment-redaction
title: Implementer self-assessment redaction in reviewer prompts
status: implemented
created: 2026-05-18
supersedes: []
archived_at: 2026-05-23
archived_reason: "v1 milestone shipped"
---

# SPEC-0029: Implementer self-assessment redaction in reviewer prompts

## Summary

`speccy review <SPEC>/<TASK>` today substitutes
`location.task_entry_raw` — a verbatim slice of TASKS.md from
`<task>` open tag through `</task>` close tag — into the rendered
prompt's `{{task_entry}}` placeholder (`speccy-cli/src/review.rs:132`).
That slice carries the implementer's own self-assessment block: a
`- Implementer note (session-...)` markdown bullet with six
sub-bullets (`Completed`, `Undone`, `Commands run`, `Exit codes`,
`Discovered issues`, `Procedural compliance`). Inlining the
implementer's account of the work directly into every reviewer
persona's prompt anchors the reviewer on the implementer's framing
and weakens the adversarial property the multi-persona fan-out
exists to provide. A tests reviewer reading `Exit codes: 0` stops
asking whether the tests are adversarial; a security reviewer
reading `Discovered issues: (none)` stops searching independently.

This SPEC retires the legacy markdown-bullet conventions that
carry these notes and promotes them to first-class XML element
children of `<task>`. Three new elements are added to the
`task_xml` whitelist:

- `<implementer-note session="...">` carries the implementer's
  self-assessment payload. Required non-empty `session` attribute;
  body MUST be non-empty.
- `<review persona="..." verdict="...">` carries a single
  persona's verdict and prose. `persona` is constrained to
  `speccy_core::personas::ALL`; `verdict` is a closed enum
  (`pass`, `blocking`) exposed as `ReviewVerdict` in Rust,
  parallel to the existing `TaskState`.
- `<retry>` carries the actionable retry instruction following a
  blocking review. Attribute-free for v1; persona attribution is
  implied by position (follows a blocking `<review>`).

`task_xml` exposes the three kinds via a new
`Task.body_items: Vec<BodyItem>` accessor that interleaves them in
document order — preserving the per-task history the
`speccy report` retry-counting flow depends on.

`speccy review` switches its `{{task_entry}}` substitution from
the raw byte slice to a redacted projection that filters
`<implementer-note>` children. Every other element body — prose,
`<task-scenarios>`, `<review>`, `<retry>`, and the `Suggested
files:` bullet — is preserved verbatim in document order. The
redaction is uniform across personas (no per-persona
configuration), silent (no placeholder line), and scoped to the
review-prompt renderer only. `speccy implement`, `speccy report`,
raw TASKS.md, and REPORT.md all continue to expose
`<implementer-note>` content verbatim.

The migration is one-shot: all in-tree TASKS.md files under
`.speccy/specs/` convert from the legacy markdown-bullet
conventions to the new XML form in a single commit. The parser
only accepts the new form post-migration. `Task.notes()` — today
a markdown-bullet scanner used by `speccy report` for retry
counting — is replaced by `Task.body_items()`; consumers migrate,
tests update, the legacy accessor is removed.

The change is bounded: the on-disk TASKS.md schema grows three
elements, the parser grows one typed surface, `speccy-cli` grows
one redacted-projection call site, two writer-side skill prompts
change, and the in-tree corpus converts in lockstep. No CLI flag
is added, no schema_version is bumped (`spec_hash_at_generation`
hashes SPEC.md, not TASKS.md, so the migration is hash-neutral),
and no transitional grandfathering is offered.

## Goals

<goals>
- `speccy review <SPEC>/<TASK>` produces a rendered prompt whose
  `{{task_entry}}` substitution contains zero text drawn from any
  `<implementer-note>` element body, for every persona, for every
  task that has accumulated such notes.
- `task_xml` recognises three new structure elements
  (`<implementer-note>`, `<review>`, `<retry>`) as nested children
  of `<task>`, validates their attributes against closed sets,
  and surfaces them on `Task` as a single ordered
  `body_items: Vec<BodyItem>` accessor preserving document order.
- The `ReviewVerdict` enum lands in `speccy-core` as a closed set
  `{Pass, Blocking}` mirroring the existing `TaskState` pattern.
- `speccy-work` and `speccy-review` skill prompts emit the new
  XML elements; the legacy `- Implementer note (...)`,
  `- Review (persona, verdict):`, and `- Retry:` markdown
  conventions are retired from skill output.
- All in-tree TASKS.md files (every spec under `.speccy/specs/`
  that has shipped or is in flight) convert to the new schema in
  a single migration commit.
- `Task.notes()` is removed; every caller migrates to
  `Task.body_items()`.
- `speccy verify` exits clean across the workspace
  post-migration with no new diagnostics attributable to the
  schema change.
</goals>

## Non-goals

<non-goals>
- No asymmetric persona behaviour (e.g. business keeps notes
  while tests/security/style strip). The adversarial property
  demands uniform inputs across personas so their findings remain
  commensurable; per-persona configuration adds a policy axis
  every future persona must answer and violates the stay-small
  principle.
- No strip-at-write-time. Implementer notes are persisted in
  TASKS.md and remain available to `speccy implement` (so the
  implementer sees their own prior notes on retry) and to
  `speccy report` (whose `## Skill updates` section depends on
  `Procedural compliance` lines). The redaction is purely a
  review-prompt rendering concern.
- No XML promotion of other markdown-bullet conventions inside
  the `<task>` body. The `Suggested files:` bullet is a planner
  artifact (written at `speccy tasks` time, not during the
  implement/review/retry loop) and stays as markdown.
- No transitional parser that accepts both the legacy
  markdown-bullet conventions and the new XML form. The
  migration is one commit; the parser only accepts the new form
  after it lands.
- No `--include-implementer-notes` debug flag on `speccy review`,
  no `--no-redact` escape hatch, no environment-variable knob.
  Opacity at the review-prompt boundary is the point of the SPEC;
  consumers wanting to inspect the implementer's notes read
  TASKS.md directly.
- No placeholder marker in the rendered prompt ("notes redacted",
  "this task was implemented; details hidden", etc.). The
  rendered prompt's header already conveys `state="in-review"`;
  absence of `<implementer-note>` content carries no false
  signal.
- No change to `speccy implement`'s rendered prompt. It continues
  to substitute `task_entry_raw` unredacted; the implementer
  needs their own prior notes for retry context.
- No change to `speccy report`'s rendered prompt or REPORT.md
  authoring contract. The report agent reads TASKS.md via the
  host Read primitive (not via an inlined substitution), and
  REPORT.md's `## Skill updates` section continues to derive
  from `Procedural compliance` lines inside `<implementer-note>`
  bodies.
- No `schema_version` bump on TASKS.md frontmatter. The
  on-wire shape of TASKS.md grows three child element names but
  no metadata field changes. `spec_hash_at_generation` is
  computed over SPEC.md, not TASKS.md, so the migration is
  hash-neutral and does not introduce drift.
- No new CLI command (`speccy migrate-tasks-schema` or similar).
  The one-shot migration runs as the first task of this SPEC
  via a script under `speccy-core/tools/` (or equivalent), not a
  shipped CLI subcommand.
- No promotion of implementer-note sub-bullet structure
  (`Completed`, `Undone`, `Commands run`, `Exit codes`,
  `Discovered issues`, `Procedural compliance`) to nested XML
  elements. The body of `<implementer-note>` stays as markdown
  payload; the writer-side skill prompt is the discipline that
  produces the six sub-bullets, not the parser.
- No `requested-by` (or equivalent) attribute on `<retry>`. The
  retry's persona attribution is implied by the immediately
  preceding `<review verdict="blocking">`; explicit attribution
  is deferred until evidence shows the implicit form is
  insufficient.
</non-goals>

## User Stories

<user-stories>
- As a reviewer-tests persona reviewing a task that the
  implementer claims is complete, I want the rendered prompt to
  withhold the implementer's `Exit codes: 0` line so I am forced
  to evaluate the test suite's adversarial quality from the diff
  rather than ratifying the implementer's self-assessment.
- As a reviewer-security persona, I want the rendered prompt to
  withhold the implementer's `Discovered issues: (none)` framing
  so I conduct an independent audit instead of pattern-matching
  on the implementer's omission.
- As a reviewer-business persona running a same-persona retry
  pass, I want to see my own prior `Review (business, blocking)`
  note and the implementer's `Retry:` response so I can verify
  the specific concern was addressed — but not the implementer's
  framing of how they addressed it.
- As the implementer on a retry pass, I want `speccy implement`
  to show me my own prior `<implementer-note>` content so I
  remember what I already tried and what I claimed worked.
- As the report author running `speccy ship`, I want TASKS.md to
  continue carrying `<implementer-note>` bodies in full so my
  `## Skill updates` section can derive the touched-skill list
  from the `Procedural compliance` payload.
- As a maintainer reading `task_xml.rs` six months from now, I
  want the body-item kinds (implementer note, review, retry) to
  be structural XML elements with closed-set validation rather
  than markdown-prefix conventions, so adding a new body-item
  kind is a schema change reviewers can see in the parser
  whitelist rather than a silent skill-prompt change.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: TASKS.md schema gains three new structure elements

The `task_xml` element whitelist (today
`["tasks", "task", "task-scenarios"]` at
`speccy-core/src/parse/task_xml/mod.rs:30`) grows to include
`implementer-note`, `review`, and `retry`. All three are nested
children of `<task>`, repeatable, and allowed in any source
order — interleaving with `<task-scenarios>` and free Markdown
prose is permitted. The new elements have closed attribute sets:

- `<implementer-note>` — required attribute `session`
  (non-empty string; no further format validation in v1). Body
  MUST be non-empty (empty bodies parse to a dedicated error
  variant).
- `<review>` — required attributes `persona` (constrained to
  `speccy_core::personas::ALL`) and `verdict` (closed set
  `{pass, blocking}`). Body MAY be empty (a terse `verdict="pass"`
  review with no prose is a legitimate signal).
- `<retry>` — attribute-free. Body SHOULD be non-empty in
  practice (a retry instruction with no content is a writer-side
  bug) but the parser does not enforce it.

<done-when>
- `TASKS_ELEMENT_NAMES` in `speccy-core/src/parse/task_xml/mod.rs`
  enumerates exactly six names:
  `["tasks", "task", "task-scenarios", "implementer-note", "review", "retry"]`.
- `validate_tag_shape` in the same module recognises each new
  element and rejects unknown attributes with the existing
  `unknown_attribute_error` diagnostic shape.
- `<implementer-note>` without a `session` attribute, or with
  `session=""`, fails parsing with a dedicated error variant.
- `<implementer-note>` with an empty body (after whitespace
  trimming) fails parsing with a dedicated error variant. The
  diagnostic prose hints that an empty body implies the task
  has not been implemented yet by any implementer.
- `<review>` with `persona` outside `speccy_core::personas::ALL`
  fails parsing; `<review>` with `verdict` outside `{pass,
  blocking}` fails parsing.
- A round-trip test (parse → render → parse) on a fixture
  carrying all three new elements interleaved with
  `<task-scenarios>` and free prose preserves element identity,
  attribute values, body contents, and source order.
</done-when>

<behavior>
- Given a TASKS.md fixture carrying one `<implementer-note
  session="...">`, one `<review persona="business"
  verdict="pass">`, and one `<retry>` element nested inside a
  single `<task>`, when `parse_task_xml` runs, then the resulting
  `Task` carries all three in source order on its typed body-item
  accessor (REQ-002 specifies the accessor's shape).
- Given a TASKS.md fixture with `<implementer-note>` lacking a
  `session` attribute, when `parse_task_xml` runs, then it
  returns a `ParseError` variant whose Display names the missing
  attribute and the task id that carried the offending element.
- Given a TASKS.md fixture with `<review persona="kerrigan"
  verdict="pass">`, when `parse_task_xml` runs, then it returns
  a `ParseError` whose Display lists the valid persona set drawn
  from `speccy_core::personas::ALL`.
- Given a TASKS.md fixture with `<review persona="business"
  verdict="maybe">`, when `parse_task_xml` runs, then it returns
  a `ParseError` whose Display lists the closed verdict set
  `{pass, blocking}`.
- Given a TASKS.md fixture where `<implementer-note>`,
  `<review>`, and `<retry>` are nested incorrectly (e.g., a
  `<review>` at the root, or a `<retry>` outside any `<task>`),
  when `parse_task_xml` runs, then it returns a `ParseError`
  with the existing "must be nested inside <task>" diagnostic
  shape.
</behavior>

<scenario id="CHK-001">
Given a TASKS.md whose `<task id="T-001" state="in-review"
covers="REQ-001">` body contains, in source order, the literal
substrings `<task-scenarios>`, `<implementer-note
session="session-1">`, `<review persona="business"
verdict="pass">`, `<retry>`, and the matching close tags for
each, when `parse_task_xml` runs against that source, then the
parse succeeds, the resulting `TasksDoc` contains exactly one
`Task` with `id == "T-001"`, and that `Task`'s body-item
accessor (per REQ-002) yields a `Vec<BodyItem>` whose variants
appear in the order `ImplementerNote, Review, Retry`.

Given a TASKS.md where `<implementer-note>` carries no
`session` attribute, when `parse_task_xml` runs, then it
returns a `ParseError` variant introduced by this SPEC whose
Display includes the substring "session" and the offending task
id.

Given a TASKS.md where `<implementer-note session="x">` carries
an empty body (only whitespace between the open and close
tags), when `parse_task_xml` runs, then it returns the dedicated
empty-body `ParseError` variant whose Display hints at the
"task not yet implemented" interpretation.

Given a TASKS.md where `<review persona="not-a-persona"
verdict="pass">` carries an invalid persona, when
`parse_task_xml` runs, then it returns a `ParseError` whose
Display enumerates the valid persona set drawn from
`speccy_core::personas::ALL`.

Given a fixture round-tripped through `parse_task_xml`
followed by `task_xml::render`, when the rendered output is
re-parsed, then the second parse yields a structurally
equivalent `TasksDoc` (same task ids, same body-item kinds in
the same order, same attribute values, same body contents
modulo the canonical-not-lossless contract documented in
`task_xml::render`).
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: `Task.body_items()` typed accessor preserves source order

The `Task` struct in
`speccy-core/src/parse/task_xml/mod.rs` gains a typed accessor
`body_items: Vec<BodyItem>` (field, method, or both — exact
shape is a planner choice) that exposes the three new element
kinds interleaved in document order. The `BodyItem` enum is:

```rust
pub enum BodyItem {
    ImplementerNote { session: String, body: String, span: ElementSpan },
    Review { persona: String, verdict: ReviewVerdict, body: String, span: ElementSpan },
    Retry { body: String, span: ElementSpan },
}
```

`ReviewVerdict` is a new closed-set enum in `speccy-core` parallel
to the existing `TaskState`:

```rust
pub enum ReviewVerdict { Pass, Blocking }
```

`ReviewVerdict::as_str` and `ReviewVerdict::from_str` mirror
`TaskState`'s shape (the wire strings are `"pass"` and
`"blocking"`).

<done-when>
- `BodyItem` is declared in `speccy-core/src/parse/task_xml/mod.rs`
  (or a sibling submodule) as a public enum with the three
  variants enumerated above, each carrying the attributes plus
  the verbatim body string and the source-span metadata.
- `ReviewVerdict` is declared in `speccy-core` as a closed-set
  enum with `as_str`, `from_str` mirroring `TaskState`. Wire
  strings are `"pass"` and `"blocking"`.
- `Task` exposes the body-item collection (field or accessor) as
  `Vec<BodyItem>` preserving document order across all three
  kinds.
- Round-trip parse → render → parse is structurally equivalent
  for fixtures interleaving the three kinds.
- `cargo test --workspace` exits 0 with the new accessor's tests
  passing.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0 modulo the carried-forward
  `result_large_err` against `ParseError`.
</done-when>

<behavior>
- Given a `Task` parsed from a fixture with the source-order
  sequence `[<implementer-note>, <review>, <retry>, <implementer-note>, <review>]`,
  when `task.body_items()` (or the equivalent field access) is
  read, then it yields exactly five entries in the same order
  with the correct enum variants.
- Given a `BodyItem::Review { verdict, .. }` extracted from a
  parsed task, when `verdict` is matched, then it matches
  exactly one of `ReviewVerdict::Pass | ReviewVerdict::Blocking`
  — no string-typed fallback.
- Given the renderer `task_xml::render`, when called on a
  `TasksDoc` whose tasks carry mixed body-item kinds, then the
  rendered output emits each `BodyItem` in its source position
  using the canonical XML element shape, and re-parsing the
  output reconstructs the same `Vec<BodyItem>` in the same
  order.
</behavior>

<scenario id="CHK-002">
Given a TASKS.md fixture whose `<task>` body contains, in
source order, one `<implementer-note>`, one `<task-scenarios>`,
one `<review verdict="blocking">`, one `<retry>`, and one
`<implementer-note>` (the retry-session note), when
`parse_task_xml` runs and the resulting `Task.body_items` is
inspected, then it returns a `Vec<BodyItem>` of length 4 (the
`<task-scenarios>` block is carried separately on the existing
`scenarios_body` field, not in `body_items`) whose variants in
order are `ImplementerNote, Review { verdict: Blocking, .. },
Retry, ImplementerNote`.

Given `ReviewVerdict::Pass.as_str()`, when invoked, then it
returns `"pass"`; given `ReviewVerdict::Blocking.as_str()`, then
`"blocking"`. Given `ReviewVerdict::from_str("pass")`, then
`Some(ReviewVerdict::Pass)`; given `ReviewVerdict::from_str("blocking")`,
then `Some(ReviewVerdict::Blocking)`; given any other input,
then `None`.

Given `task_xml::render` invoked on a `TasksDoc` carrying tasks
with mixed `body_items`, when the output is re-parsed via
`parse_task_xml`, then the second parse yields tasks whose
`body_items` matches the first parse's `body_items` field by
field (kind, attributes, body content) and in the same order.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: `speccy review` performs implementer self-assessment redaction

The `{{task_entry}}` substitution emitted by
`speccy-cli/src/review.rs` is changed from the raw
`location.task_entry_raw` string to a redacted projection that
filters out `<implementer-note>` elements. The projection
preserves every other body item — `<task-scenarios>`, `<review>`
elements (peer review notes from prior persona passes), `<retry>`
elements, the `Suggested files:` markdown bullet, and any free
prose between elements — verbatim and in document order.

The redaction lives in the parser layer (`task_xml` or a sibling
helper module under `speccy-core`), exposed to `speccy-cli` via a
typed function on `Task` (e.g.
`Task::render_for_review_prompt() -> String`) or a free-standing
helper. `speccy-cli/src/review.rs` calls the new helper and
substitutes its output into `{{task_entry}}`; it does not perform
string-level filtering itself.

The redaction is uniform across personas: there is no per-persona
branching, no `Persona` parameter on the redaction function, and
no configuration knob.

The redaction is silent: no placeholder line, no `<!-- notes
redacted -->`-style marker, no "(implementer notes withheld)"
prose. The rendered prompt looks indistinguishable between a
task with `<implementer-note>` children and one without, except
for the omitted element bodies themselves.

When a task has no `<implementer-note>` children, the redacted
projection is structurally identical to the unredacted form (the
redactor is a no-op when there is nothing to remove).

<done-when>
- `speccy-cli/src/review.rs:132` (today
  `vars.insert("task_entry", location.task_entry_raw.clone());`)
  is changed to substitute a redacted task-entry string drawn
  from a new `task_xml`-layer helper.
- The new helper lives in `speccy-core` (not in `speccy-cli`)
  and operates on the typed `Task` (or on the
  `(TasksDoc, Task)` pair when source-bytes context is needed
  for span resolution).
- The new helper does not branch on persona name.
- A test asserts that for a task carrying `<implementer-note>`,
  `<task-scenarios>`, `<review>`, and `<retry>` elements, the
  redacted output contains every body byte of the latter three
  elements verbatim and zero bytes of the `<implementer-note>`
  body.
- A test asserts that for a task carrying no
  `<implementer-note>` element, the redacted output is byte-for-byte
  identical to the unredacted `task_entry_raw` slice.
- A test asserts that the rendered review prompt contains no
  occurrence of the string `<implementer-note` for any persona
  when the task carries one or more such elements.
</done-when>

<behavior>
- Given a task with two `<implementer-note>` elements (initial
  + retry-session), three `<review>` elements (one per pass),
  one `<retry>` element, and one `<task-scenarios>` element,
  when `speccy review <SPEC>/<task-id> --persona business` runs
  and the rendered prompt is captured, then the captured prompt
  contains every `<review>` body, the `<retry>` body, the
  `<task-scenarios>` body, the prose body, and the `Suggested
  files:` line verbatim and in source order, and contains zero
  characters of either `<implementer-note>` body.
- Given the same task, when `speccy review` runs against each
  of the six built-in personas (business, tests, security,
  style, architecture, docs) in turn, then the captured prompts
  carry byte-identical `{{task_entry}}` substitutions across
  all six (redaction is uniform).
- Given a task carrying no `<implementer-note>` element (e.g.
  a freshly-flipped `state="in-review"` task that hasn't been
  reviewed once yet), when `speccy review` runs, then the
  captured prompt's `{{task_entry}}` is byte-identical to what
  the pre-SPEC implementation would have produced from the same
  task source.
- Given a rendered review prompt produced after this SPEC
  ships, when scanned for the substring "Commands run:" or
  "Exit codes:" or "Discovered issues:" or "Procedural
  compliance:", then no match is found inside the
  `## Task entry` section of the prompt (these strings live
  only inside `<implementer-note>` payload, which is redacted).
</behavior>

<scenario id="CHK-003">
Given a TASKS.md fixture where `<task id="T-001"
state="in-review" covers="REQ-001">` carries (in source order):
free prose, `<task-scenarios>` with non-empty body,
`<implementer-note session="s1">` with the six-sub-bullet
payload, `<review persona="business" verdict="blocking">` with
prose, `<retry>` with prose, `<implementer-note session="s1-retry">`
with the retry-session payload, and `<review persona="business"
verdict="pass">` with prose, when `speccy review SPEC-NNNN/T-001
--persona business` runs against a workspace containing that
fixture and the rendered prompt is captured, then:

- The captured prompt contains every byte of the free prose,
  `<task-scenarios>` body, both `<review>` bodies (with their
  persona / verdict attributes), and the `<retry>` body
  verbatim and in source order.
- The captured prompt contains zero bytes drawn from either
  `<implementer-note>` body (neither `s1` nor `s1-retry`).
- The captured prompt does not contain any of the substrings
  "Commands run:", "Exit codes:", "Discovered issues:",
  "Procedural compliance:", "Undone:", or "Completed:" inside
  its `## Task entry` section.
- The captured prompt does not contain any placeholder-style
  marker indicating redaction occurred (no "redacted", "notes
  withheld", "implementer notes hidden", or similar prose).

Given the same fixture, when `speccy review SPEC-NNNN/T-001
--persona <P>` is invoked once per persona in
`speccy_core::personas::ALL` and the six rendered prompts are
diffed pairwise on the `## Task entry` section, then the
section is byte-identical across all six.

Given a TASKS.md fixture where `<task id="T-002"
state="in-review" covers="REQ-001">` carries no
`<implementer-note>` element (the task just flipped to
in-review and no implementer-note was written yet), when
`speccy review SPEC-NNNN/T-002 --persona business` runs, then
the parser rejects the input with the empty-body /
missing-note error variant introduced by REQ-001 — surfacing
the "task not yet implemented" interpretation rather than
silently rendering an unredacted prompt.
</scenario>

</requirement>

<requirement id="REQ-004">
### REQ-004: Redaction is scoped to the review-prompt renderer only

`speccy implement`, `speccy report`, raw TASKS.md on disk, and
the REPORT.md authoring contract are unaffected by the redaction
introduced in REQ-003. Specifically:

- `speccy-cli/src/implement.rs:107` continues to substitute
  `location.task_entry_raw.clone()` into its `{{task_entry}}`
  placeholder. The implementer needs to see their own prior
  `<implementer-note>` content on retry to remember what they
  already tried.
- `speccy-cli/src/report.rs` continues to render the report
  prompt by naming `{{tasks_md_path}}` (it never inlined
  TASKS.md body); the report agent reads TASKS.md via the host
  Read primitive and sees `<implementer-note>` content in full.
- `resources/modules/prompts/report.md` continues to instruct
  the report author to derive the `## Skill updates` section
  from `Procedural compliance` lines inside `<implementer-note>`
  bodies.
- TASKS.md on disk continues to carry `<implementer-note>`
  bodies in full after migration; the redaction is purely a
  rendering-time transformation in the review path.

<done-when>
- `speccy-cli/src/implement.rs:107` is byte-identical before
  and after this SPEC's implementation (it continues to use
  `location.task_entry_raw`).
- `speccy-cli/src/report.rs` carries no call to the new
  redaction helper.
- `resources/modules/prompts/report.md` still references
  `Procedural compliance` lines (the report prompt is
  unchanged).
- A test asserts that the rendered `speccy implement` prompt
  for a task with one or more `<implementer-note>` elements
  contains the substring `<implementer-note` in its
  `## Task entry` section (i.e. the implementer prompt is NOT
  redacted).
- After the migration tooling runs (REQ-006), every in-tree
  TASKS.md still carries `<implementer-note>` element bodies
  verbatim — the migration is a syntactic conversion of the
  carrier element, not a content strip.
</done-when>

<behavior>
- Given a workspace where a task has been implemented once and
  carries a single `<implementer-note>` element, when
  `speccy implement <SPEC>/<TASK>` runs, then the rendered
  prompt's `## Task entry` section contains the verbatim
  `<implementer-note>` body — the redaction does not apply
  here.
- Given the same workspace, when `speccy report <SPEC>` runs
  (after all tasks complete), then the rendered prompt names
  `{{tasks_md_path}}` and instructs the agent to read TASKS.md
  via the Read primitive — the report agent gets the unredacted
  source.
- Given the migrated TASKS.md after REQ-006 lands, when its raw
  bytes are read by any consumer outside `speccy review`, then
  every `<implementer-note>` body is present verbatim.
</behavior>

<scenario id="CHK-004">
Given a workspace whose TASKS.md carries one or more
`<implementer-note>` elements, when `speccy implement
SPEC-NNNN/T-NNN` runs and the rendered prompt is captured,
then the captured prompt contains the substring
`<implementer-note` at least once inside its `## Task entry`
section.

Given the same workspace, when `speccy report SPEC-NNNN` runs
and the rendered prompt is captured, then the prompt does not
inline TASKS.md content (it names `{{tasks_md_path}}` and
instructs the report agent to read it) and the prompt's
`## Skill updates` instruction still references "Procedural
compliance lines in the inline implementer notes above" or
substantially equivalent prose.

Given any in-tree TASKS.md after REQ-006 migration, when
grepped for the literal substring `<implementer-note`, then at
least one match exists in every TASKS.md that previously
carried `- Implementer note (session-...)` markdown bullets.
The migration converted the carrier element; it did not strip
the payload.
</scenario>

</requirement>

<requirement id="REQ-005">
### REQ-005: Writer-side skill prompts emit the new XML elements

The shipped skill prompts that produce implementer notes,
peer reviews, and retry notes update to emit XML elements
instead of the legacy markdown-bullet conventions:

- `resources/modules/prompts/implementer.md` (rendered by
  `speccy implement`) instructs the implementer to append an
  `<implementer-note session="...">` element block at task
  closure, with the six required sub-bullets as markdown payload
  inside the element body. The legacy
  `- Implementer note (session-...):` bullet form is retired
  from the prompt.
- The reviewer persona prompts at
  `resources/modules/prompts/reviewer-*.md` (rendered by
  `speccy review`) instruct each persona to append a `<review
  persona="..." verdict="...">` element block carrying the
  verdict and prose. The legacy
  `- Review (persona, verdict): ...` bullet form is retired.
- The `speccy-review` orchestrating skill at
  `.claude/skills/speccy-review/SKILL.md` (mirrored under
  `resources/agents/`) instructs the orchestrator to write a
  `<retry>` element block when at least one persona reports
  `verdict="blocking"`. The legacy `- Retry: ...` bullet form is
  retired.
- All six reviewer persona prompts
  (`reviewer-business`, `reviewer-tests`, `reviewer-security`,
  `reviewer-style`, `reviewer-architecture`, `reviewer-docs`)
  update in lockstep — the wording of the emit-an-XML-element
  instruction is identical across personas.

<done-when>
- `grep -rn "Implementer note (session" resources/ .claude/`
  returns zero matches in skill / prompt files (the legacy
  markdown form is retired from authoring instructions).
- `grep -rn "Review (\\w*, pass\\|Review (\\w*, blocking"
  resources/ .claude/` returns zero matches in skill / prompt
  files.
- `grep -rn "- Retry:" resources/ .claude/` returns zero
  matches in skill / prompt files.
- `resources/modules/prompts/implementer.md` contains the
  literal substring `<implementer-note session=` in its
  authoring instructions.
- All six `resources/modules/prompts/reviewer-*.md` contain the
  literal substring `<review persona=` in their authoring
  instructions.
- `.claude/skills/speccy-review/SKILL.md` (and its mirror under
  `resources/agents/`) contains the literal substring `<retry>`
  in its blocking-verdict instruction.
- After this SPEC lands, the in-tree TASKS.md files contain
  zero `- Implementer note (session-`, `- Review (`, or
  `- Retry:` markdown bullets attributable to the legacy
  authoring convention. (REQ-006 enforces the migration; this
  REQ enforces the writer-side change going forward.)
</done-when>

<behavior>
- Given the rendered `speccy implement` prompt after this SPEC
  lands, when its "Closing a task" section (or equivalent) is
  read, then it instructs the implementer to write an
  `<implementer-note session="...">` block, not a
  `- Implementer note (session-...):` markdown bullet.
- Given the rendered `speccy review` prompt for any persona
  after this SPEC lands, when its "Append a review note"
  section (or equivalent) is read, then it instructs the
  persona to write a `<review persona="..." verdict="...">`
  block, not a `- Review (persona, verdict):` markdown
  bullet.
- Given the `.claude/skills/speccy-review/SKILL.md` orchestrator
  skill after this SPEC lands, when its blocking-verdict branch
  is read, then it instructs writing a `<retry>` element
  block rather than a `- Retry:` markdown bullet.
</behavior>

<scenario id="CHK-005">
Given `resources/modules/prompts/implementer.md` after this
SPEC lands, when grepped for the substring
`<implementer-note session=`, then at least one match exists
in the prompt's task-closure instruction.

Given any of the six files
`resources/modules/prompts/reviewer-{business,tests,security,style,architecture,docs}.md`
after this SPEC lands, when grepped for the substring
`<review persona=`, then at least one match exists in the
file's review-note authoring instruction.

Given `.claude/skills/speccy-review/SKILL.md` (and its mirror
`resources/agents/...`) after this SPEC lands, when grepped
for the substring `<retry>`, then at least one match exists in
the blocking-verdict orchestration step.

Given the same files, when grepped for the legacy markdown
forms (`- Implementer note (session-`, `- Review (`, `- Retry:`)
inside authoring instructions, then zero matches are found.

Given the `.claude/` and `resources/` trees after this SPEC
lands, when scanned for any reference to "Implementer note"
or "Review (persona," or "Retry:" prefix conventions, then
the only remaining mentions live in historical SPEC bodies or
this SPEC's own prose — not in shipped authoring
instructions.
</scenario>

</requirement>

<requirement id="REQ-006">
### REQ-006: One-shot migration converts all in-tree TASKS.md to the new schema

A migration script under `speccy-core/tools/` (exact path is a
planner choice) converts every in-tree TASKS.md to the new XML
schema in a single commit. The migration is parser-driven, not
regex-based: it parses each TASKS.md with a transitional parser
that recognises both the legacy markdown-bullet conventions and
the new XML elements, then emits the canonical XML form via
`task_xml::render`. The transitional parser exists only inside
the migration script; the shipped `task_xml` parser only accepts
the new form.

After the migration commit lands:

- Every in-tree TASKS.md across `.speccy/specs/*/TASKS.md` parses
  cleanly under the new `task_xml` whitelist.
- `speccy verify` exits 0 with zero new diagnostics attributable
  to the schema change.
- `spec_hash_at_generation` values stored in TASKS.md frontmatter
  are unchanged (that hash is over SPEC.md, not TASKS.md; the
  migration is hash-neutral by construction).
- No TASKS.md retains a `- Implementer note (session-`,
  `- Review (`, or `- Retry:` markdown bullet attributable to
  the legacy authoring convention.

The migration script is invoked manually as part of the SPEC's
first task; it does not ship as a `speccy` CLI subcommand.

<done-when>
- `grep -rn "^- Implementer note (session-"
  .speccy/specs/*/TASKS.md` returns zero matches.
- `grep -rn "^- Review (\\w*, " .speccy/specs/*/TASKS.md`
  returns zero matches.
- `grep -rn "^- Retry: " .speccy/specs/*/TASKS.md` returns zero
  matches.
- `grep -rn "<implementer-note session=" .speccy/specs/*/TASKS.md`
  returns at least one match per TASKS.md that previously
  carried a legacy `- Implementer note` bullet (the migration
  is a syntactic carrier conversion).
- `speccy verify` exits 0 against the migrated workspace.
- The script lives under `speccy-core/tools/` (or equivalent
  in-repo location) and is documented in a brief README beside
  it. It is not registered as a `speccy` CLI subcommand and is
  not surfaced in `--help`.
- The migration commit references SPEC-0029 in its message.
</done-when>

<behavior>
- Given the in-tree corpus of TASKS.md files before this SPEC's
  migration task runs, when each is parsed by the new
  `task_xml` parser, then at least one parse fails (the legacy
  markdown-bullet convention is no longer accepted).
- Given the same corpus after the migration task lands, when
  each TASKS.md is parsed by the new `task_xml` parser, then
  every parse succeeds and yields a `TasksDoc` with the
  expected `body_items` content.
- Given `speccy verify` after the migration lands, when run,
  then the exit code is 0 and the workspace summary shows zero
  errors and zero new warnings attributable to the schema
  change (the carried-forward `result_large_err` clippy
  warning is unrelated and stays).
- Given the migration script after the SPEC ships, when invoked
  on a workspace whose TASKS.md files have already been
  migrated, then it is a no-op (idempotent re-run).
</behavior>

<scenario id="CHK-006">
Given the in-tree workspace after this SPEC's first task lands,
when `grep -rn "^- Implementer note (session-"
.speccy/specs/*/TASKS.md` runs from the project root, then
zero matches are found.

Given the same workspace, when `grep -rn "<implementer-note
session=" .speccy/specs/*/TASKS.md` runs, then at least one
match exists in every TASKS.md whose corresponding SPEC
reached `status="implemented"` before SPEC-0029 (i.e. every
spec that accumulated implementer notes during its ship).

Given `cargo test --workspace` after the migration lands, when
run, then the exit code is 0 — including any test that parses
the in-tree TASKS.md corpus (e.g.
`speccy-core/tests/in_tree_tasks_reports.rs`).

Given `speccy verify` after the migration lands, when run with
its JSON output captured, then the captured output reports zero
errors and zero new warnings; any surviving warnings are
explicitly carried forward from prior SPECs (e.g. the
`result_large_err` carry from SPEC-0026) and named in REPORT.md.

Given the migration script, when run twice in succession
against the same workspace, then the second run is a no-op (no
file modifications, exit code 0).
</scenario>

</requirement>

<requirement id="REQ-007">
### REQ-007: `Task.notes()` is removed; consumers migrate to `Task.body_items()`

The `Task.notes()` accessor in
`speccy-core/src/parse/task_xml/mod.rs:167-182` is removed. Its
only known consumer — `speccy report`'s retry-counting flow in
`speccy-cli/src/report.rs` — migrates to
`Task.body_items()` and filters `BodyItem::Retry` variants. All
tests that exercise `Task.notes()` are deleted or rewritten
against `body_items()`.

`Task.suggested_files()` stays unchanged. It scans for the
`- Suggested files:` markdown bullet, which is a planner
artifact (written at `speccy tasks` time, not during the
implement / review / retry loop) and is not in scope for the
XML migration.

<done-when>
- `grep -n "fn notes\\|Task::notes\\|\\.notes()" speccy-core/ speccy-cli/`
  returns zero matches in non-test source files.
- `speccy-cli/src/report.rs`'s retry-counting code path
  references `body_items` (or equivalent typed access) rather
  than `notes()`.
- All tests previously exercising `Task.notes()` are either
  removed (if testing a behaviour that no longer exists) or
  rewritten against `body_items()`.
- `cargo test --workspace` exits 0 after the migration.
- `cargo clippy --workspace --all-targets --all-features -- -D
  warnings` exits 0 modulo the carried-forward
  `result_large_err` against `ParseError`.
</done-when>

<behavior>
- Given the `Task` API surface after this SPEC lands, when
  enumerated via rustdoc, then there is no `notes()` accessor
  and the retry-counting capability is reachable only via
  `body_items()` matched against `BodyItem::Retry`.
- Given `speccy report <SPEC>` rendered against a fixture where
  some tasks carry one or more `<retry>` elements, when the
  rendered prompt is captured, then its retry-summary section
  reports the correct count per task (one increment per
  `BodyItem::Retry` variant on that task) — the migration
  preserves the existing retry-count semantics.
- Given a test fixture that previously asserted
  `Task.notes()` returned a vector containing certain
  markdown bullets, when migrated to assert against
  `Task.body_items()`, then the new assertion captures the
  same semantic content via typed variants.
</behavior>

<scenario id="CHK-007">
Given the file `speccy-core/src/parse/task_xml/mod.rs` after
this SPEC lands, when grepped for the literal substring `fn
notes(`, then zero matches are found inside an `impl Task`
block.

Given `speccy-cli/src/report.rs` after this SPEC lands, when
grepped for the substring `.notes()`, then zero matches are
found in production source lines.

Given `cargo test -p speccy-core` after this SPEC lands, when
run, then the exit code is 0 and no test name references
`notes` as a legacy accessor (any surviving "notes"-named test
exercises `body_items()` or an unrelated feature).

Given a `speccy report SPEC-NNNN` run against a fixture where
exactly two tasks carry one `<retry>` element each and the
remaining tasks carry none, when the rendered prompt's retry
summary is read, then the summary attributes one retry to each
of the two tasks and zero retries to the others — matching the
pre-SPEC behaviour of `Task.notes()`-based counting.
</scenario>

</requirement>

## Design

### Approach

The implementation lands in roughly six layers, with the
migration task front-loaded so the schema change is observable
in the in-tree corpus before any downstream consumer is rewired:

1. **Schema + parser layer**
   (`speccy-core/src/parse/task_xml/mod.rs` plus the shared XML
   scanner under `speccy-core/src/parse/xml_scanner/`). Add the
   three new element names to the whitelist, declare the
   `BodyItem` enum and `ReviewVerdict` enum, extend
   `validate_tag_shape` with the new attribute sets, extend the
   `assemble` step to recognise the new children and push them
   into the task's body-item collection, and extend the
   `render` step to emit them in document order.

2. **`ParseError` variants**
   (`speccy-core/src/parse/error.rs`). Add dedicated variants
   for missing `session` attribute, empty `<implementer-note>`
   body, invalid `verdict`, and invalid `persona`. Each variant
   carries the offending task id and the source offset so
   diagnostics name the failure site.

3. **Migration script**
   (`speccy-core/tools/migrate-tasks-schema.rs` or equivalent).
   A small standalone binary that parses each in-tree TASKS.md
   with a transitional parser, emits the canonical XML form via
   `task_xml::render`, and rewrites the file in place. The
   transitional parser is private to the migration tool and
   never ships with the CLI. The migration commit edits the
   28-ish in-tree TASKS.md files in lockstep.

4. **Renderer for review**
   (`speccy-cli/src/review.rs`, with the redaction helper in
   `speccy-core/src/parse/task_xml/mod.rs` or a sibling). Switch
   the `{{task_entry}}` substitution from `task_entry_raw` to
   the redacted projection. The projection re-renders the task
   from its parsed form, omitting `<implementer-note>` children.

5. **Writer-side skill prompts**
   (`resources/modules/prompts/implementer.md`,
   `resources/modules/prompts/reviewer-*.md`,
   `.claude/skills/speccy-review/SKILL.md` plus its mirror under
   `resources/agents/`). Update authoring instructions to emit
   the new XML elements; retire the legacy markdown-bullet
   forms from the shipped prompts.

6. **Consumer migration**
   (`speccy-cli/src/report.rs` and tests). Remove `Task.notes()`,
   migrate `speccy report`'s retry counting to filter
   `BodyItem::Retry` variants, update tests.

The task-level ordering matters: the migration commit (layer 3)
must precede the layer-4 redaction switch, because the redaction
helper depends on parsing `<implementer-note>` elements that the
migration just installed. Layer 1's parser change must also
precede the migration, because the migration script writes the
canonical form and exercises the new parser on its own output as
a sanity check. The canonical sequence is therefore
`(1) parser → (3) migration → (4-6) downstream rewiring → (5)
writer skills` so that no in-flight intermediate state leaves the
workspace unparseable by `speccy verify`.

### Decisions

<decision id="DEC-001" status="accepted">
### DEC-001: Bundled scope rather than layered

The schema migration of TASKS.md (adding three new structure
elements with full parser support) and the reviewer-prompt
redaction ship in one SPEC, not as a narrow F-8 plus a
follow-on schema SPEC. A narrow F-8 would have introduced a
typed `body_items()` surface with a markdown-prefix
implementation, leaving the persistence shape as a markdown
convention until a later SPEC migrated it to XML.

The trade-off is real: bundled means a larger single SPEC with
schema, parser, writer skills, migration tooling, and reviewer
redaction all landing together. Layered would have been
smaller per slice. We chose bundled because:

- The typed surface never has a transient
  markdown-prefix-only implementation that callers learn to
  depend on.
- The migration cost is dominated by edit volume across the
  28-ish in-tree TASKS.md files, not by per-implementation
  difficulty. Running the migration twice (once for the typed
  facade, once for the XML promotion) would burn the same edit
  budget while leaving a longer window where the convention
  and the schema disagree.
- The persistence shape is the durable artifact; landing it
  alongside the typed surface is honest about what the abstraction
  protects.

The bundled SPEC is correspondingly larger but still
sub-architecture in scope (no new CLI command, no new flag, no
new lint code).
</decision>

<decision id="DEC-002" status="accepted">
### DEC-002: Silent redaction

The redacted projection emits no placeholder line indicating
that `<implementer-note>` content was removed. Three
alternatives were considered and rejected:

- **Visible marker** (e.g. `<!-- implementer notes redacted
  -->` or "Implementer notes withheld from this prompt
  intentionally."). Rejected because the marker itself
  anchors: a reviewer reading "notes withheld" infers "the
  implementer wrote something noteworthy you can't see," which
  is the bias the redaction is meant to avoid.
- **Digest replacement** (e.g. "Implementer claims complete;
  verify against diff."). Rejected because the residue carries
  no actionable signal and adds rendering complexity.
- **Reorder + reframe** (keep notes but move to end of prompt
  with disclaimer). Rejected because anchoring is subconscious;
  prompt-level labels don't reliably defeat it.

The rendered prompt's header already conveys `state="in-review"`,
so absence of `<implementer-note>` content carries no false
signal about whether the task was attempted.
</decision>

<decision id="DEC-003" status="accepted">
### DEC-003: Uniform across personas

The redaction applies identically to all six built-in personas
(business, tests, security, style, architecture, docs) and to
any future persona. There is no per-persona configuration knob,
no asymmetric strip ("business keeps notes, others don't"), and
no architecture-vs-business carve-out.

Asymmetric was considered seriously: business reviewer
legitimately needs intent signal to judge "should this have
triggered an amendment?", and the implementer notes carry that.
We rejected asymmetry because:

- Per-persona configuration adds a policy axis every new persona
  must answer, violating stay-small.
- Bias propagation worsens when persona outputs reason from
  non-commensurable evidence bases — a business finding
  grounded in implementer rationale and a tests finding
  grounded only in diff are harder to reconcile and weigh.
- The legitimate "business needs intent" case is better served
  by ensuring the SPEC encodes intent well enough that
  implementer notes aren't load-bearing. If the SPEC is too
  thin for business to judge from SPEC + diff + `<task-scenarios>`
  alone, that is a SPEC bug, fixed via `speccy-amend`, not by
  leaking notes through the review-prompt surface.
- Peer review notes (`<review>` elements from prior persona
  passes) stay visible, which already provides cross-persona
  signal for same-persona retries and for catching scope drift.
</decision>

<decision id="DEC-004" status="accepted">
### DEC-004: `<implementer-note>` requires `session` attribute and non-empty body

`<implementer-note>` parses successfully only when `session` is
present and non-empty AND the element body (after whitespace
trimming) is non-empty. Both invariants surface as dedicated
`ParseError` variants when violated.

The non-empty-body invariant is load-bearing: an empty
`<implementer-note>` implies the task carries the structural
marker of "has been implemented" without the payload that
substantiates the claim. The most likely explanation is that
the writer-side skill emitted the open and close tags before
populating the body — i.e. the task has not been implemented
yet by any implementer. Surfacing this as a parse error rather
than a silent empty render forces the caller to fix the
upstream state (re-run the implementer, or fix the skill
prompt) instead of producing a misleadingly-empty review
prompt.

The `session` attribute is a required non-empty string but the
parser does not enforce any further format (e.g. a regex
matching `session-<date>-<spec>-<task>`). Format is writer-side
discipline, not parser concern. If the convention drifts, the
fix is in the skill prompt; the parser stays permissive on the
value as long as it is present and non-empty.
</decision>

<decision id="DEC-005" status="accepted">
### DEC-005: One-shot migration, no transitional grandfathering

The shipped `task_xml` parser only accepts the new XML form
post-migration. There is no transitional period during which
both the legacy markdown-bullet conventions and the new XML
elements parse successfully.

Grandfathering both forms was considered and rejected because:

- The migration is bounded: 28-ish in-tree TASKS.md files
  across one repository (this one). There is no
  external-consumer corpus to convert.
- A transitional parser doubles the parser's surface area and
  the test matrix — every parse-related fixture would need
  variants for both forms.
- "One commit, one canonical form" is the cleanest
  audit trail. A reviewer reading `git log` sees the migration
  as a single intentional change, not as a series of partial
  conversions.

The migration script is private to the implementation, lives
under `speccy-core/tools/`, and is not surfaced as a `speccy`
CLI subcommand. Once the migration commit lands, the script is
dead code that the next SPEC may delete.
</decision>

<decision id="DEC-006" status="accepted">
### DEC-006: Replace `Task.notes()` outright

`Task.notes()` is removed in the same commit that introduces
`Task.body_items()`. We do not ship a deprecation period during
which both coexist.

The legacy accessor has exactly one production consumer
(`speccy report`'s retry-counting in
`speccy-cli/src/report.rs`) and a small number of test sites.
Carrying both accessors during a deprecation period would mean
keeping the legacy markdown-bullet scanner around (because
`notes()` walks the markdown body) alongside the new typed
accessor — which contradicts the migration's goal of having
exactly one canonical body-item representation.
</decision>

<decision id="DEC-007" status="accepted">
### DEC-007: `ReviewVerdict` is a closed enum parallel to `TaskState`

The verdict surface uses a Rust enum
(`ReviewVerdict::{Pass, Blocking}`) with `as_str` and `from_str`
methods mirroring the existing `TaskState` pattern. Wire strings
are `"pass"` and `"blocking"`. The parser rejects any other
value with a dedicated `ParseError` variant that names the
closed set.

The closed-enum shape is preferred over a string-typed field
for the same reasons `TaskState` is an enum: pattern matching
on the verdict in downstream consumers is exhaustive
(`speccy-ship`'s "all tasks completed" gate can compose with
"any review blocking" checks without string comparison), and
adding a new verdict in the future is a compile-time event
across consumers rather than a runtime string drift.
</decision>

<decision id="DEC-008" status="accepted">
### DEC-008: `<retry>` is attribute-free for v1

The `<retry>` element carries no attributes. Persona
attribution is implied by source position: a `<retry>` element
follows the `<review verdict="blocking">` element that
triggered it. This is the same convention the legacy markdown
bullets carried implicitly (`- Retry:` followed
`- Review (persona, blocking):`).

A `requested-by="<persona>"` attribute was considered to make
attribution explicit. We rejected it for v1 because:

- The position-following convention is unambiguous in practice
  — a `<retry>` not preceded by a blocking `<review>` is a
  writer-side bug that the orchestrating skill should never
  emit.
- An optional attribute that no consumer currently reads adds
  schema surface without immediate payoff.
- If future analysis surfaces a load-bearing need (e.g. a
  reviewer wants to know whether the retry instruction came
  from business or tests when the same task accumulated
  blockers from both), adding the attribute is a forward
  schema extension that does not break the v1 shape.
</decision>

### Interfaces

- `speccy-core::parse::task_xml`:
  - `TASKS_ELEMENT_NAMES` grows to
    `["tasks", "task", "task-scenarios", "implementer-note", "review", "retry"]`.
  - New `BodyItem` enum (variants: `ImplementerNote`,
    `Review`, `Retry`), each carrying attributes + verbatim body
    + `ElementSpan`.
  - New `Task.body_items: Vec<BodyItem>` field (or accessor),
    preserving document order across kinds.
  - `Task.notes()` removed.
  - `Task` carries a new redaction helper (exact name a planner
    choice, e.g. `Task::render_for_review_prompt(source: &str) -> String`)
    that emits the task subtree with `<implementer-note>` children
    omitted.
- `speccy-core::ReviewVerdict`:
  - New closed-set enum (`Pass`, `Blocking`) with `as_str`,
    `from_str` mirroring `TaskState`.
- `speccy-core::parse::error::ParseError`:
  - New variants for missing-`session`, empty-implementer-note-body,
    invalid-verdict, invalid-persona inside `<review>`.
- `speccy-cli::review`:
  - `vars.insert("task_entry", ...)` switches from
    `location.task_entry_raw.clone()` to the redacted output of
    the new `task_xml`-layer helper.

### Data changes

- TASKS.md schema: three new nested children of `<task>`
  (`<implementer-note session="...">`, `<review persona="..."
  verdict="...">`, `<retry>`). Repeatable, source-ordered,
  interleavable with each other and with `<task-scenarios>`.
- All 28-ish in-tree TASKS.md files migrate from the legacy
  markdown-bullet conventions to the new XML form in a single
  commit. No `spec_hash_at_generation` change (that hash is
  over SPEC.md).

### Migration / rollback

- **Forward**: a one-shot script under `speccy-core/tools/`
  parses each TASKS.md with a transitional both-forms parser,
  renders the canonical XML form, and rewrites the file. The
  shipped `task_xml` parser only accepts the new form
  post-migration.
- **Rollback**: revert the migration commit (which restores the
  legacy markdown-bullet form across all in-tree TASKS.md files)
  and revert the parser whitelist commit (which restores
  legacy-only acceptance). No on-disk artifact other than
  TASKS.md is touched by the migration; rollback is a `git
  revert` away.

## Open Questions

- [x] Should `<retry>` carry a `requested-by` attribute? **Resolved
      via DEC-008**: no, attribute-free for v1; position-following
      convention is unambiguous.
- [x] Should the migration ship as a `speccy migrate-tasks-schema`
      CLI subcommand? **Resolved via DEC-005 and the non-goals**:
      no, the migration is a private script under
      `speccy-core/tools/` not surfaced via `--help`.
- [x] Should the rendered prompt carry a redaction marker?
      **Resolved via DEC-002**: no, silent redaction.
- [x] Should `Task.notes()` be deprecated with a window or removed
      outright? **Resolved via DEC-006**: removed outright.
- [x] Where does the redaction helper live — `task_xml` or a
      sibling module under `speccy-core`? **Folded into the
      design's "Approach" section**: in `task_xml` (or a sibling
      under `speccy-core`), exact module is a planner choice.

## Assumptions

<assumptions>
- The `<implementer-note>` body remains as markdown payload
  (the six sub-bullets: `Completed`, `Undone`, `Commands run`,
  `Exit codes`, `Discovered issues`, `Procedural compliance`).
  The XML wrapper provides classification; the payload format
  is writer-side skill discipline. The parser validates only
  that the body is non-empty; it does not enforce the six
  sub-bullets' presence or order.
- Order preservation matters: `Task.body_items` is a single
  `Vec<BodyItem>` interleaving implementer notes, reviews, and
  retries in source order. The retry-counting flow in
  `speccy report` depends on per-task ordering (counting one
  increment per `<retry>` element regardless of position).
- `Task.suggested_files()` stays as a markdown-bullet accessor.
  The `- Suggested files:` bullet is written by `speccy tasks`
  at task-generation time, not during the implement / review /
  retry loop, and is therefore out of scope for the XML
  migration. Promoting it to a structural element would expand
  scope without serving REQ-003's adversarial-property goal.
- `spec_hash_at_generation` (the SPEC.md sha256 stored in
  TASKS.md frontmatter, per SPEC-0024) is unaffected by the
  TASKS.md schema change. The hash is computed over SPEC.md
  bytes; TASKS.md content changes do not flow into the hash.
- The pre-existing `clippy::result_large_err` warning against
  `speccy_core::error::ParseError` (carried forward from
  SPEC-0026 T-003) remains out of scope. SPEC-0029 adds new
  `ParseError` variants and inherits the carry-forward
  warning; F-7 in the backlog tracks the underlying cleanup.
</assumptions>

## Notes

### Rejected alternative framings

These framings were considered during brainstorm and rejected
explicitly; they are recorded here so future readers do not
need to re-litigate them:

- **Asymmetric strip** (business keeps notes; tests / security /
  style strip). Rejected per DEC-003: per-persona configuration
  violates stay-small, worsens bias propagation across persona
  outputs, and the "business needs intent" concern is better
  served by encoding intent in the SPEC via `speccy-amend`.
- **Soft strip — prompt-instruction only** (keep notes, instruct
  the reviewer to ignore them). Rejected: anchoring is
  subconscious; prompt-level labels don't reliably defeat it.
- **Digest replacement** (collapse the implementer block to a
  one-line "Implementer claims complete; verify against
  diff."). Rejected: the residue carries no actionable signal
  and adds rendering complexity.
- **Reorder + reframe** (keep notes but move to end of prompt
  with disclaimer). Rejected for the same anchoring reason as
  soft strip.
- **Strip at TASKS.md write time** (don't persist notes at
  all). Rejected: REPORT.md and implementer retry context
  depend on persistence; the redaction belongs at the
  review-prompt rendering boundary, not the storage layer.
- **Layered F-8 (narrow + follow-on)** (typed
  `body_items()` facade with markdown-prefix implementation in
  this SPEC; XML promotion of implementer notes / reviews /
  retries in a follow-on SPEC). Considered seriously and
  rejected per DEC-001: bundled keeps the typed surface from
  having a transient markdown-prefix-only implementation that
  consumers learn to depend on.
- **Light path** (private classifier inside `task_xml` with no
  public typed surface). Rejected: loses forward-compatibility
  and leaves prefix-matching scattered across consumers.

### Investigation findings carried from brainstorm

The brainstorm phase verified the following by code-read; these
notes are preserved here as durable context for the
implementation:

- `speccy-cli/src/review.rs:132` substitutes
  `location.task_entry_raw.clone()` into `{{task_entry}}`. The
  raw slice is built by `task_lookup::extract_entry_from_raw`
  walking `tasks_md.raw` from the `<task>` open tag forward to
  the `</task>` close tag.
- Both `speccy review` and `speccy implement` consume
  `task_entry_raw`. The redaction must apply to review only.
- `speccy report` does NOT inline TASKS.md content into its
  rendered prompt — it names `{{tasks_md_path}}` and instructs
  the report agent to use the host Read primitive. REPORT.md's
  `## Skill updates` section depends on `Procedural compliance`
  lines existing in TASKS.md on disk.
- The pre-SPEC `task_xml` element whitelist is exactly
  `["tasks", "task", "task-scenarios"]`. Implementer notes,
  peer reviews, and retry notes are markdown bullets in
  `Task.body`, not structured elements.
- `Task.notes()` and `Task.suggested_files()` are the existing
  markdown-bullet scanners; the codebase has already accepted
  prefix-string classification as a parser-layer convention.
  This SPEC retires the implementer-note / review / retry
  branches of that convention and replaces them with structural
  XML elements; the `Suggested files:` branch stays.

## Changelog

<changelog>
| Date       | Reason                                                                                                                                                                                                                                                       | Author     |
|------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------|
| 2026-05-18 | Initial draft. Bundled scope per DEC-001: TASKS.md schema gains `<implementer-note>`, `<review>`, `<retry>` elements; `speccy review` redacts `<implementer-note>` from its rendered prompt; one-shot migration converts the in-tree corpus; legacy `Task.notes()` retired. | Kevin Xiao |
</changelog>
