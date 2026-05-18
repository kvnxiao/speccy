#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! SPEC-0029 T-001 integration tests for the typed body-item surface,
//! the redaction helper, and the new `ParseError` variants.
//!
//! The unit tests in `speccy-core/src/parse/task_xml/mod.rs` cover the
//! existing surface (state validation, covers grammar, duplicate ids,
//! etc.). The cases here are slice-level scenarios drawn from
//! SPEC-0029's `<task-scenarios>` and REQ-001 / REQ-002 `<behavior>`
//! Given/When/Then prose. The fixtures live inline rather than as
//! separate files because each one exercises a single error variant or
//! ordering property — keeping the assertions and source together makes
//! the contract easy to read.

use camino::Utf8Path;
use indoc::indoc;
use speccy_core::ParseError;
use speccy_core::parse::BodyItem;
use speccy_core::parse::ReviewVerdict;
use speccy_core::parse::TasksDoc;
use speccy_core::parse::parse_task_xml;
use speccy_core::parse::redact_implementer_notes;
use speccy_core::parse::render_task_xml;

fn path() -> &'static Utf8Path {
    Utf8Path::new("fixture/TASKS.md")
}

fn frontmatter() -> &'static str {
    "---\nspec: SPEC-0099\n---\n\n# Tasks: SPEC-0099\n\n"
}

fn make(body: &str) -> String {
    format!("{}{}", frontmatter(), body)
}

fn parse(source: &str) -> TasksDoc {
    parse_task_xml(source, path()).expect("fixture should parse")
}

/// Slice-scenario 1: a `<task>` carrying one `<implementer-note>`,
/// one `<task-scenarios>`, one `<review verdict="blocking">`, one
/// `<retry>`, and one retry-session `<implementer-note>` parses into
/// a `Task` whose `body_items` carries the four non-`<task-scenarios>`
/// elements in source order. The `<task-scenarios>` element lives on
/// the existing `scenarios_body` field (length-4 assertion per
/// CHK-002 ¶1).
#[test]
fn body_items_preserves_source_order_across_mixed_kinds() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        Free prose before the scenarios.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="s1">
        - Completed: shipped the thing.
        - Undone: (none)
        </implementer-note>

        <review persona="business" verdict="blocking">
        scope drift concern.
        </review>

        <retry>
        narrow the scope to the original ask.
        </retry>

        <implementer-note session="s1-retry">
        - Completed: scope narrowed.
        - Undone: (none)
        </implementer-note>
        </task>

        </tasks>
    "#});
    let doc = parse(&src);
    let task = doc.tasks.first().expect("one task");
    assert_eq!(task.id, "T-001");
    assert_eq!(task.body_items.len(), 4);
    let first = task.body_items.first().expect("len 4");
    assert!(
        matches!(
            first,
            BodyItem::ImplementerNote { session, body, .. }
                if session == "s1" && body.contains("shipped the thing"),
        ),
        "first body item must be the s1 implementer note, got: {first:?}",
    );
    let second = task.body_items.get(1).expect("len 4");
    assert!(
        matches!(
            second,
            BodyItem::Review { persona, verdict: ReviewVerdict::Blocking, body, .. }
                if persona == "business" && body.contains("scope drift"),
        ),
        "second body item must be the business/blocking review, got: {second:?}",
    );
    let third = task.body_items.get(2).expect("len 4");
    assert!(
        matches!(third, BodyItem::Retry { body, .. } if body.contains("narrow the scope")),
        "third body item must be the retry, got: {third:?}",
    );
    let fourth = task.body_items.get(3).expect("len 4");
    assert!(
        matches!(
            fourth,
            BodyItem::ImplementerNote { session, body, .. }
                if session == "s1-retry" && body.contains("scope narrowed"),
        ),
        "fourth body item must be the retry-session implementer note, got: {fourth:?}",
    );
    // `<task-scenarios>` continues to live on `scenarios_body`, not in
    // `body_items` — CHK-002 ¶1's explicit length-4-not-5 assertion.
    assert!(
        task.scenarios_body.contains("Given X"),
        "scenarios_body must carry the task-scenarios block body",
    );
}

/// Slice-scenario 2: `<implementer-note>` without a `session` attribute
/// surfaces as `MissingImplementerNoteSession`. CHK-001 ¶2.
#[test]
fn missing_session_attribute_surfaces_dedicated_variant() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note>
        - Completed: stuff.
        </implementer-note>
        </task>

        </tasks>
    "#});
    let err = parse_task_xml(&src, path()).expect_err("missing session must fail");
    let msg = format!("{err}");
    assert!(
        matches!(
            err.as_ref(),
            ParseError::MissingImplementerNoteSession { task_id, .. } if task_id == "T-001",
        ),
        "got: {err:?}",
    );
    assert!(msg.contains("session"), "msg must name `session`: {msg}");
    assert!(msg.contains("T-001"), "msg must name task id: {msg}");
}

/// Slice-scenario 2b: empty `session=""` is treated the same as a
/// missing attribute (writer-side intent is identical; both imply the
/// session was never populated).
#[test]
fn empty_session_attribute_is_rejected_like_missing() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="">
        - Completed: stuff.
        </implementer-note>
        </task>

        </tasks>
    "#});
    let err = parse_task_xml(&src, path()).expect_err("empty session must fail");
    assert!(
        matches!(
            err.as_ref(),
            ParseError::MissingImplementerNoteSession { task_id, .. } if task_id == "T-001",
        ),
        "got: {err:?}",
    );
}

/// Slice-scenario 3: `<implementer-note session="x">` with an empty
/// body surfaces as `EmptyImplementerNoteBody`. CHK-001 ¶3.
#[test]
fn empty_implementer_note_body_surfaces_dedicated_variant() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="x">
        </implementer-note>
        </task>

        </tasks>
    "#});
    let err = parse_task_xml(&src, path()).expect_err("empty body must fail");
    let msg = format!("{err}");
    assert!(
        matches!(
            err.as_ref(),
            ParseError::EmptyImplementerNoteBody { task_id, .. } if task_id == "T-001",
        ),
        "got: {err:?}",
    );
    assert!(
        msg.contains("not been implemented") || msg.contains("not yet"),
        "msg must hint at the not-yet-implemented interpretation: {msg}",
    );
}

/// Slice-scenario 4: `<review verdict="maybe">` surfaces as
/// `InvalidReviewVerdict` whose `Display` lists `pass` and `blocking`.
/// CHK-001 invalid-verdict scenario.
#[test]
fn invalid_verdict_surfaces_dedicated_variant_with_closed_set() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <review persona="business" verdict="maybe">
        prose.
        </review>
        </task>

        </tasks>
    "#});
    let err = parse_task_xml(&src, path()).expect_err("invalid verdict must fail");
    let msg = format!("{err}");
    assert!(
        matches!(
            err.as_ref(),
            ParseError::InvalidReviewVerdict { task_id, value, allowed, .. }
                if task_id == "T-001"
                    && value == "maybe"
                    && allowed == "pass, blocking",
        ),
        "got: {err:?}",
    );
    assert!(msg.contains("pass"), "msg must list `pass`: {msg}");
    assert!(msg.contains("blocking"), "msg must list `blocking`: {msg}");
}

/// Slice-scenario 5: `<review persona="kerrigan">` surfaces as
/// `InvalidReviewPersona` whose `Display` enumerates the valid persona
/// set drawn from `speccy_core::personas::ALL`. CHK-001 ¶4.
#[test]
fn invalid_persona_surfaces_dedicated_variant_with_personas_set() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <review persona="kerrigan" verdict="pass">
        prose.
        </review>
        </task>

        </tasks>
    "#});
    let err = parse_task_xml(&src, path()).expect_err("invalid persona must fail");
    let msg = format!("{err}");
    assert!(
        matches!(
            err.as_ref(),
            ParseError::InvalidReviewPersona { task_id, value, .. }
                if task_id == "T-001" && value == "kerrigan",
        ),
        "got: {err:?}",
    );
    for persona in speccy_core::personas::ALL {
        assert!(
            msg.contains(persona),
            "msg `{msg}` must list valid persona `{persona}`",
        );
    }
}

/// Slice-scenario 6: round-trip parse → render → parse on a fixture
/// with mixed body items preserves the typed model.
#[test]
fn round_trip_preserves_body_items_order_and_attributes() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose before.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="s1">
        - Completed: a
        - Undone: b
        </implementer-note>

        <review persona="tests" verdict="pass">
        looks good.
        </review>

        <retry>
        addressed the blocking concern.
        </retry>

        <implementer-note session="s1-retry">
        - Completed: c
        </implementer-note>
        </task>

        </tasks>
    "#});
    let doc = parse(&src);
    let rendered = render_task_xml(&doc);
    let doc2 = parse_task_xml(&rendered, path()).expect("rendered output must re-parse");
    assert_eq!(doc.tasks.len(), doc2.tasks.len(), "task count drift");
    let t1 = doc.tasks.first().expect("two tasks");
    let t2 = doc2.tasks.first().expect("two tasks");
    assert_eq!(t1.id, t2.id);
    assert_eq!(t1.state, t2.state);
    assert_eq!(t1.covers, t2.covers);
    assert_eq!(t1.scenarios_body.trim(), t2.scenarios_body.trim());
    assert_eq!(
        t1.body_items.len(),
        t2.body_items.len(),
        "body_items length drift across round-trip",
    );
    for (a, b) in t1.body_items.iter().zip(t2.body_items.iter()) {
        assert_eq!(
            std::mem::discriminant(a),
            std::mem::discriminant(b),
            "body_items variant drift across round-trip: {a:?} vs {b:?}"
        );
        match (a, b) {
            (
                BodyItem::ImplementerNote {
                    session: sa,
                    body: ba,
                    ..
                },
                BodyItem::ImplementerNote {
                    session: sb,
                    body: bb,
                    ..
                },
            ) => {
                assert_eq!(sa, sb, "implementer-note session drift");
                assert_eq!(ba.trim(), bb.trim(), "implementer-note body drift");
            }
            (
                BodyItem::Review {
                    persona: pa,
                    verdict: va,
                    body: ba,
                    ..
                },
                BodyItem::Review {
                    persona: pb,
                    verdict: vb,
                    body: bb,
                    ..
                },
            ) => {
                assert_eq!(pa, pb, "review persona drift");
                assert_eq!(va, vb, "review verdict drift");
                assert_eq!(ba.trim(), bb.trim(), "review body drift");
            }
            (BodyItem::Retry { body: ba, .. }, BodyItem::Retry { body: bb, .. }) => {
                assert_eq!(ba.trim(), bb.trim(), "retry body drift");
            }
            _ => {}
        }
    }
}

/// `ReviewVerdict::as_str` / `from_str` round-trip. CHK-002 ¶2.
#[test]
fn review_verdict_round_trip() {
    assert_eq!(ReviewVerdict::Pass.as_str(), "pass");
    assert_eq!(ReviewVerdict::Blocking.as_str(), "blocking");
    assert_eq!(ReviewVerdict::from_str("pass"), Some(ReviewVerdict::Pass));
    assert_eq!(
        ReviewVerdict::from_str("blocking"),
        Some(ReviewVerdict::Blocking),
    );
    // Case-sensitive (mirrors `TaskState::from_str`).
    assert_eq!(ReviewVerdict::from_str("PASS"), None);
    assert_eq!(ReviewVerdict::from_str(""), None);
    assert_eq!(ReviewVerdict::from_str("maybe"), None);
}

/// Slice-scenario 7a: redaction helper invoked on a task carrying
/// `<implementer-note>` children produces output that contains every
/// non-`<implementer-note>` body byte verbatim and zero
/// `<implementer-note` substrings.
#[test]
fn redact_strips_implementer_notes_and_preserves_everything_else() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose before.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="s1">
        - Completed: shipped.
        - Exit codes: 0
        - Discovered issues: (none)
        - Procedural compliance: (none)
        </implementer-note>

        <review persona="business" verdict="blocking">
        a blocking concern.
        </review>

        <retry>
        narrow scope.
        </retry>

        <implementer-note session="s1-retry">
        - Completed: scope narrowed.
        - Procedural compliance: touched skill X.
        </implementer-note>

        <review persona="business" verdict="pass">
        looks good now.
        </review>
        </task>

        </tasks>
    "#});
    let doc = parse(&src);
    let task = doc.tasks.first().expect("one task");
    let entry_raw = task_entry_raw(&src, task);

    let redacted = redact_implementer_notes(&entry_raw, task);
    assert!(
        !redacted.contains("<implementer-note"),
        "redacted output must not contain `<implementer-note`: {redacted}",
    );
    assert!(
        !redacted.contains("Exit codes:"),
        "redacted output must not contain implementer-note payload sub-bullets: {redacted}",
    );
    assert!(
        !redacted.contains("Discovered issues:"),
        "redacted output must not contain implementer-note payload sub-bullets: {redacted}",
    );
    assert!(
        !redacted.contains("Procedural compliance:"),
        "redacted output must not contain implementer-note payload sub-bullets: {redacted}",
    );
    // Other body items survive verbatim.
    assert!(
        redacted.contains("a blocking concern."),
        "redacted output must preserve the blocking review body: {redacted}",
    );
    assert!(
        redacted.contains("looks good now."),
        "redacted output must preserve the pass review body: {redacted}",
    );
    assert!(
        redacted.contains("narrow scope."),
        "redacted output must preserve the retry body: {redacted}",
    );
    assert!(
        redacted.contains("Given X, when Y, then Z."),
        "redacted output must preserve the task-scenarios body: {redacted}",
    );
    assert!(
        redacted.contains("prose before."),
        "redacted output must preserve free prose: {redacted}",
    );
    // No marker prose smuggled in.
    for marker in &["redacted", "withheld", "hidden", "notes omitted"] {
        assert!(
            !redacted.contains(marker),
            "redacted output must contain no placeholder-style marker `{marker}`: {redacted}",
        );
    }
}

/// Slice-scenario 7b: redaction helper on a task carrying no
/// `<implementer-note>` is byte-identical to the raw task entry. This
/// is the byte-identity contract — the redactor is a no-op when there
/// is nothing to remove.
#[test]
fn redact_is_byte_identical_when_no_implementer_note_present() {
    let src = make(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-002" state="in-review" covers="REQ-001">
        prose before.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <review persona="business" verdict="pass">
        no notes; just a verdict.
        </review>
        </task>

        </tasks>
    "#});
    let doc = parse(&src);
    let task = doc.tasks.first().expect("one task");
    let entry_raw = task_entry_raw(&src, task);

    let redacted = redact_implementer_notes(&entry_raw, task);
    assert_eq!(
        redacted, entry_raw,
        "redactor must be byte-identical to raw entry when task carries no implementer notes",
    );
}

/// Re-implements `task_lookup::extract_entry_from_raw` so the redaction
/// helper's byte-identity contract can be asserted against the same
/// raw-slice shape `speccy review` will substitute at the call site
/// (T-004 wires this up; here we pin the contract).
fn task_entry_raw(raw: &str, task: &speccy_core::parse::Task) -> String {
    let start = task.span.start;
    let Some(after) = raw.get(start..) else {
        return String::new();
    };
    let mut end_offset = after.len();
    let mut cursor: usize = 0;
    for line in after.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']).trim();
        cursor = cursor.saturating_add(line.len());
        if trimmed == "</task>" {
            end_offset = cursor;
            break;
        }
    }
    let slice = after.get(..end_offset).unwrap_or(after);
    slice.trim_end_matches(['\r', '\n']).to_owned()
}
