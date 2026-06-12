---
spec: SPEC-0062
spec_hash_at_generation: 29f573d8f69ad5de4b930a0b92de60ca81588db30a3bc14d78fa298f9401bc21
generated_at: 2026-06-12T18:33:59Z
---
# Tasks: SPEC-0062 Retire the last hand-rolled tag recognizer — reconcile's recovery offset derives from the canonical scanner

<task id="T-001" state="completed" covers="REQ-001">
## Re-derive the reconcile recovery offset from `scan_tags`; delete the hand-rolled scan

Replace `consistency.rs::last_well_formed_offset` — the last hand-rolled tag
recognizer in production — with a thin helper homed in the `journal_xml` parse
module that reuses the canonical `xml_scanner::scan_tags` token stream (DEC-001).

Add `pub fn last_well_formed_offset(source: &str, path: &Utf8Path) -> usize` to
`speccy-core/src/parse/journal_xml/mod.rs`, reusing the exact preamble
`journal_xml::parse` already composes (`mod.rs:150,182-187`):
`split_required(source, path, "journal file")` → `collect_code_fence_byte_ranges`
→ `ScanConfig { whitelist: JOURNAL_ELEMENT_NAMES, structure_shaped_names:
JOURNAL_ELEMENT_NAMES }` → `scan_tags(...)`. Then walk the returned
`Vec<RawTag>` with a depth counter: on an open tag (`!is_close`) increment depth;
on a close tag, if depth was 1, record `tag.body_end_after_tag` as the running
result and drop to depth 0. Return the last recorded close offset, initialized to
`0`. Per DEC-002 the offset comes from the token stream (the end of the last
well-formed depth-0 close), never from the failed parse's `ParseError.offset`.
Two error paths both collapse to `0`, the legitimate "nothing closed cleanly"
value: `split_required` returning `Err` (missing/empty frontmatter — the
Assumptions block and CHK-003) and `scan_tags` returning `Err` (byte-arithmetic
overflow, impossible for an in-memory string). The helper returns `usize`, not a
`Result` — it runs only after strict parse already failed.

Route `consistency.rs::detect_journal_drift` (`consistency.rs:426`) to call
`journal_xml::last_well_formed_offset(source, &journal_path)` (the `use
crate::parse::journal_xml;` import at `consistency.rs:19` already exists). Delete
the private `last_well_formed_offset` fn and its
`#[expect(clippy::similar_names, ...)]` (`consistency.rs:445-519`).

Relocate the two unit tests `last_well_formed_offset_finds_close_of_implementer`
and `last_well_formed_offset_zero_when_no_close_tag` (`consistency.rs:548-560`)
into the `journal_xml` tests module, pointing them at the new helper. The new
helper begins with `split_required`, so each fixture must carry minimal valid
journal frontmatter (`---\nspec: SPEC-0042\ntask: T-001\ngenerated_at:
2026-01-01T00:00:00Z\n---\n`) ahead of the body, otherwise the frontmatter guard
short-circuits to `0` before any scan. Keep the expected values unchanged: the
finder case still returns the byte past `</implementer>`; the zero case still
returns `0` (now via the frontmatter guard rather than via no-close-found — same
value, endorsed by CHK-003).

Audit non-test `speccy-core` and `speccy-cli` source for hand-rolled tag-scan
patterns (`find("<`, `find('<'`, `format!("<{`). Confirm the only remaining
matches are block renderers that *emit* tags (the SPEC-0061 CHK-006 exclusion),
that no recognizer scanning input survives, and that `consistency.rs`'s
`find('<')` loop is gone. Record the audit result in the per-task journal so
`/speccy-ship` can lift it into REPORT.md.

Do not touch `xml_scanner` (DEC-003 — no new recognition primitive), the
`DriftDetails::JournalXmlMalformed` shape, the `last_well_formed_byte_offset`
field, or the reconcile truncation policy (Non-goals).

<task-scenarios>
Given the post-task `speccy-core` source,
when a reviewer audits non-test code for hand-rolled tag-scan patterns
(`find("<`, `find('<'`, `format!("<{`),
then the only matches are block renderers that emit tags (not recognizers
scanning input), the `consistency.rs::last_well_formed_offset` `find('<')` loop
no longer exists, and the audit is recorded in the journal for REPORT.md.

Given a per-task journal with a well-formed `<implementer>…</implementer>` block
followed by trailing bytes that make `journal_xml::parse` fail,
when `detect_journal_drift` computes `last_well_formed_byte_offset`,
then the value equals the byte offset just past the `</implementer>` close tag —
identical to the value the pre-SPEC hand-rolled scan returned, proven by the
preserved `consistency_detect.rs` `journal_xml_malformed` fixture
(`find("</implementer>") + "</implementer>".len()`) and the
`details.last_well_formed_byte_offset` assertion in
`speccy-cli/tests/consistency.rs` staying green unchanged.

Given a corrupt per-task journal that is missing its YAML frontmatter (so
`split_required` rejects it before any tag scan),
when `detect_journal_drift` computes `last_well_formed_byte_offset`,
then the value is 0.

Given the workspace after this task,
when `cargo test --workspace` and `cargo clippy --workspace --all-targets
--all-features -- -D warnings` run,
then both pass — the relocated unit tests are green, the two preserved fixtures
are green at their unchanged expected values, and removing the `#[expect]` left no
unfulfilled-expectation lint and no dead imports.

Suggested files: `speccy-core/src/parse/journal_xml/mod.rs`,
`speccy-core/src/consistency.rs`,
`speccy-core/tests/consistency_detect.rs` (verification only),
`speccy-cli/tests/consistency.rs` (verification only)
</task-scenarios>
</task>

<task id="T-002" state="pending" covers="REQ-002">
## Pin the fence-blindness fix with a recovery-offset regression test

Add a regression test proving that a line-isolated journal close tag inside a
fenced code block is not counted as a structural close when reconcile computes
the recovery offset — the fence-blindness divergence class SPEC-0061 left in this
one scanner, now fixed for free because the offset derives from the fence-aware
`scan_tags` (T-001).

Add a `#[test]` to `speccy-core/tests/consistency_detect.rs` (alongside the
existing `detect_journal_xml_malformed_*` test so it reuses the `make_spec_dir` /
`detect` harness and exercises the full `detect_journal_drift` path end-to-end).
Construct a per-task journal with valid frontmatter and a well-formed
`<implementer>…</implementer>` (or `<review>`) block whose structural close ends
at byte X; then, after byte X, place a body region containing a line-isolated
`</implementer>` *inside a fenced code block* (its `>` ending at byte Y > X);
then trailing content that makes `journal_xml::parse` fail (so the malformed
branch runs). Assert the computed `details.last_well_formed_byte_offset == X`
(the real structural close), not Y (the fenced occurrence).

Satisfy CHK-004's "recorded pre-fix run" clause: capture, once, the value the
pre-SPEC hand-rolled scan produced for this exact input. Recover the deleted
`last_well_formed_offset` body from git (`git show HEAD~:speccy-core/src/...` for
the pre-SPEC revision, or stash T-001 and run against the old function), run it
against the fixture, and confirm it yields Y (the fence-blind, wrong offset).
Record both the pre-fix Y and the post-fix X in the test's comment and in the
per-task journal for REPORT.md. Do not commit the resurrected old code — it is a
one-shot measurement to prove the bug the test guards.

This task adds test code only; it relies on T-001 having moved the read path onto
`scan_tags`.

<task-scenarios>
Given a per-task journal whose last well-formed structural close (`</implementer>`
or `</review>`) ends at byte X, whose body thereafter contains a line-isolated
journal close tag inside a fenced code block ending at byte Y > X, and whose
trailing content makes `journal_xml::parse` fail,
when `detect_journal_drift` computes `last_well_formed_byte_offset` after the fix,
then the value is X (the structural close), not Y (the fenced occurrence).

Given the same fixture run against the pre-SPEC hand-rolled scan recovered from
git,
when its `last_well_formed_byte_offset` is measured,
then the value reflects Y — the fence-blindness bug — and that pre-fix value is
recorded in the test comment and the journal for REPORT.md.

Given the workspace after this task,
when `cargo test --workspace` runs,
then the new regression test passes (asserting X).

Suggested files: `speccy-core/tests/consistency_detect.rs`
</task-scenarios>
</task>
