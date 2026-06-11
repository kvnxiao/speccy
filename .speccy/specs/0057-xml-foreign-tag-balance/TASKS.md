---
spec: SPEC-0057
spec_hash_at_generation: c490330da6c93550273f2eb4fc2b0e8cc3e5aadd85e3f6037c7571e4bf386b17
generated_at: 2026-06-11T01:05:32Z
---
# Tasks: SPEC-0057 Unbalanced foreign-tag lint — `speccy verify` flags leaked orphan XML tags in parsed artifacts

<task id="T-001" state="completed" covers="REQ-002 REQ-003">
## Add the `VOID_ELEMENT_NAMES` set and the `scan_foreign_tags` scanner helper

Lay the mechanical foundation the `XML-001` lint consumes (SPEC DEC-004).
No lint, no diagnostics, no balance logic land in this task — only two
reusable scanner primitives plus their unit tests. The scanner's
permissive passthrough (`scan_tags`) is left untouched.

In `speccy-core/src/parse/xml_scanner/html5_names.rs`, add
`pub const VOID_ELEMENT_NAMES: &[&str]` holding exactly the 14 HTML5 void
element names from REQ-002 (`area`, `base`, `br`, `col`, `embed`, `hr`,
`img`, `input`, `link`, `meta`, `param`, `source`, `track`, `wbr`) and an
`is_void_element_name(name: &str) -> bool` helper. Re-export both from
`xml_scanner/mod.rs` alongside the existing
`pub use html5_names::HTML5_ELEMENT_NAMES;`.

In `speccy-core/src/parse/xml_scanner/mod.rs`, add a
`pub struct ForeignTag { pub name: String, pub is_close: bool, pub line:
u32 }` and a
`pub fn scan_foreign_tags(source: &str, code_fence_ranges: &[(usize,
usize)], whitelist: &[&str]) -> Vec<ForeignTag>`. It is the inverse of
`scan_tags`: walk the source line-by-line reusing the existing private
`next_line` walk and `range_inside_any_fence` skip, match each line
against the existing private `open_tag_regex()` / `close_tag_regex()`, and
push a `ForeignTag` for every matched tag whose element name is **outside**
`whitelist` (mirroring the `!cfg.whitelist.contains(...)` arms in
`classify_line`, but emitting on the non-whitelisted branch instead of
returning early). Whitelisted names are skipped. Lines inside any fenced
range are skipped (this is the REQ-003 fence exemption — it lives in the
helper, not the lint). Track a 1-indexed line counter as the walk
advances and stamp it on each `ForeignTag`. Self-closing `<foo/>` does not
match the strict `open_tag_regex` and is therefore never returned (SPEC
Assumptions). The helper does no balance computation and no void
filtering — callers decide.

Add scanner unit tests in the `xml_scanner` `#[cfg(test)] mod tests`
block proving the mechanism, and a test in `html5_names` proving the void
set is a real subset of `HTML5_ELEMENT_NAMES` (a structural invariant, not
a copy of the literal).

<task-scenarios>
Given a source with a bare foreign `<custom>` open on its own line at
line 3 and a bare `</custom>` close on its own line at line 7, when
`scan_foreign_tags` runs with a whitelist that does not contain `custom`,
then it returns two `ForeignTag`s — `{name:"custom", is_close:false,
line:3}` and `{name:"custom", is_close:true, line:7}`.

Given a source whose only `</custom>` line sits inside a triple-backtick
fenced block, when `scan_foreign_tags` runs, then it returns no record for
that line (fence exemption).

Given a source containing a bare `<requirement>` line and the whitelist
`["requirement"]`, when `scan_foreign_tags` runs, then no record is
returned (whitelisted structural tags are excluded — only foreign tags
are reported).

Given a source containing a bare self-closing `<custom/>` line, when
`scan_foreign_tags` runs, then no record is returned.

Given `VOID_ELEMENT_NAMES`, then every entry is also a member of
`HTML5_ELEMENT_NAMES`.

Suggested files: `speccy-core/src/parse/xml_scanner/html5_names.rs`,
`speccy-core/src/parse/xml_scanner/mod.rs`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-002 REQ-003 REQ-004 REQ-005">
## Implement the `XML-001` balance lint over SPEC.md / TASKS.md / REPORT.md and register it

Create the `XML-001` lint family and wire it into the engine and registry.
This task makes the lint observable end-to-end over the three
parsed-document artifacts; the journal artifact is added in T-003.

Create `speccy-core/src/lint/rules/xml.rs` with
`pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>)`. For each of
the three parsed artifacts that parsed successfully, build a
`(raw, path, whitelist)` triple and run the balance pass:
- SPEC.md: `spec.spec_doc_ok().raw`, `spec.spec_md_path`,
  `spec_xml::SPECCY_ELEMENT_NAMES`.
- TASKS.md: `spec.tasks_md_ok().raw`, `spec.tasks_md_path`,
  `task_xml::TASKS_ELEMENT_NAMES`.
- REPORT.md: `spec.report_md_ok().raw`, `spec.dir.join("REPORT.md")`
  (there is no `report_md_path` field on `ParsedSpec` — derive it),
  `report_xml::REPORT_ELEMENT_NAMES`.

Balance pass (SPEC DEC-002): compute `collect_code_fence_byte_ranges(raw)`,
call `scan_foreign_tags(raw, &fences, whitelist)`, then walk the returned
`ForeignTag`s maintaining a `HashMap<String, Vec<u32>>` of per-name
open-line stacks. A foreign **open** whose name is NOT in
`VOID_ELEMENT_NAMES` pushes its line onto that name's stack (void-named
opens are never pushed — REQ-002). A foreign **close** pops its name's
stack; an empty stack means the close is a dangling orphan — emit
`XML-001` at the close's line. After the walk, every line still on any
stack is a dangling open — emit `XML-001` at that line. Cross-name nesting
is deliberately not enforced (DEC-002).

Emit via `Diagnostic::with_location("XML-001", Level::Error,
spec.spec_id.clone(), path, line, message)`. Use a single shared message
template parameterized **only** by the tag name (REQ-001: the open-orphan
and close-orphan cases must share one template, differing only in the
substituted tag name and line — do not branch the wording on
open-vs-close). The path and 1-indexed line are carried by
`with_location` and surfaced by the diagnostic renderer (REQ-005); the
message text itself need only name the offending tag.

Register: append `("XML-001", Level::Error)` to `REGISTRY` in
`speccy-core/src/lint/registry.rs` and re-bless
`speccy-core/tests/snapshots/lint_registry.snap` so the snapshot test
pins `XML-001` at `error`. Wire `pub mod xml;` into
`speccy-core/src/lint/rules/mod.rs` and add `rules::xml::lint(spec, &mut
diagnostics);` to the per-spec loop in `speccy-core/src/lint/mod.rs`
(update the module-doc lint-family list there too).

Add integration tests in a new `speccy-core/tests/lint_xml.rs` following
the `lint_common` helper and tempdir-workspace conventions used by
`lint_jnl.rs`. Note the `partition_lint` demotion (`speccy-cli`): Error
diagnostics on `in-progress` specs are demoted to Info, so the
verify-exit fixture must use a spec with `status: implemented` for the
non-zero exit to hold.

<task-scenarios>
Given a fixture `TASKS.md` whose body ends with a bare `</content>` line
and then a bare `</invoke>` line with no matching opens, when the lint
engine runs, then exactly two `XML-001` Error diagnostics fire, one per
orphan close line, each naming that `TASKS.md` and the correct line
(REQ-001 / CHK-001).

Given a fixture artifact containing a foreign non-void open tag on its
own line with no matching close anywhere after it, when the lint engine
runs, then exactly one `XML-001` diagnostic names that open tag's line
(REQ-001 / CHK-002).

Given a fixture artifact with a balanced `<details>`…`</details>` pair,
when the lint engine runs, then no `XML-001` diagnostic fires (REQ-001 /
CHK-003).

Given one fixture with a lone `<br>` line and a second with a lone
non-void foreign open, when the lint engine runs over both, then the
`<br>` fixture produces zero `XML-001` and the non-void fixture produces
exactly one (REQ-002 / CHK-004).

Given a fixture whose only orphan foreign close sits inside a fenced code
block, when the lint engine runs, then no `XML-001` fires; and a foreign
close outside any fence still fires regardless of fenced occurrences of
the same name (REQ-003 / CHK-005).

Given a fixture spec whose `SPEC.md`, `TASKS.md`, and `REPORT.md` each
contain exactly one dangling foreign tag, when the lint engine runs, then
exactly three `XML-001` diagnostics are produced, one per artifact with
the correct file path (REQ-004 / CHK-006).

Given a fixture workspace (spec `status: implemented`) whose sole lint
finding is one dangling foreign tag in a parsed artifact, when `speccy
verify` runs against it, then the process exits non-zero and the rendered
output names the artifact path and the orphan tag's 1-indexed line
(REQ-005 / CHK-008).

Suggested files: `speccy-core/src/lint/rules/xml.rs`,
`speccy-core/src/lint/rules/mod.rs`, `speccy-core/src/lint/mod.rs`,
`speccy-core/src/lint/registry.rs`,
`speccy-core/tests/snapshots/lint_registry.snap`,
`speccy-core/tests/lint_xml.rs`
</task-scenarios>
</task>

<task id="T-003" state="completed" covers="REQ-004">
## Extend `XML-001` detection to on-demand `journal/T-NNN.md` files

Cover the fourth parsed-artifact class (SPEC DEC-003: journals are
defense-in-depth, reached on demand rather than via a `ParsedSpec` field).
Extend `speccy-core/src/lint/rules/xml.rs` to derive journal paths the
same way the `JNL-*` rules do: `let journal_dir = spec.dir.join("journal");`
then, for each task in `spec.tasks_md_ok().tasks`,
`journal_dir.join(format!("{}.md", task.id))`. For each path that
`exists()`, read it with `fs_err::read_to_string` and run the same balance
pass used for the parsed documents, with the journal whitelist
`journal_xml::JOURNAL_ELEMENT_NAMES`. Emit `XML-001` naming the journal
file path and the orphan line. Do not add a journal field to `ParsedSpec`.

Add the journal integration test to `speccy-core/tests/lint_xml.rs`,
building an on-disk tempdir workspace with a `journal/` subdir as
`lint_jnl.rs` does.

<task-scenarios>
Given a fixture spec with a task whose `journal/T-001.md` exists and
contains a dangling foreign tag, when the lint engine runs, then exactly
one `XML-001` diagnostic is produced whose file is that journal file and
whose line is the orphan tag's line (REQ-004 / CHK-007).

Given a fixture spec whose journal files contain only balanced foreign
tags (or none), when the lint engine runs, then no `XML-001` diagnostic
fires for any journal file.

Suggested files: `speccy-core/src/lint/rules/xml.rs`,
`speccy-core/tests/lint_xml.rs`
</task-scenarios>
</task>

<task id="T-004" state="completed" covers="REQ-006">
## Lock raw-source retention on `SpecDoc` / `TasksDoc` / `ReportDoc` with a regression test

The `XML-001` lint depends on each parsed document retaining its full raw
source (`pub raw: String`, already present today). Add a regression test
that pins this property so a future refactor cannot silently drop the
field the lint reads. No production code changes are expected — this task
is the test lock only.

Add a test (a new `speccy-core/tests/raw_retention.rs`, or an equivalent
`#[cfg(test)]` block on the parser modules) that parses a valid fixture
source for each of `SPEC.md`, `TASKS.md`, and `REPORT.md` via
`parse_spec_xml`, `parse_task_xml`, and `parse_report_xml`, then asserts
each resulting document's `raw` field is byte-identical to the source
string it was parsed from. Reuse the existing valid-fixture helpers where
available rather than hand-rolling minimal sources.

<task-scenarios>
Given valid fixture sources for `SPEC.md`, `TASKS.md`, and `REPORT.md`,
when each is parsed, then `SpecDoc.raw`, `TasksDoc.raw`, and
`ReportDoc.raw` are each byte-identical (`==`) to their respective source
string (REQ-006 / CHK-009).

Suggested files: `speccy-core/tests/raw_retention.rs`,
`speccy-core/src/parse/spec_xml/mod.rs`,
`speccy-core/src/parse/task_xml/mod.rs`,
`speccy-core/src/parse/report_xml/mod.rs`
</task-scenarios>
</task>
