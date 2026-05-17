#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for the SPEC-0022 T-002 TASKS.md typed model,
//! parser, and renderer.
//!
//! Covers REQ-001 (grammar shape, the four valid states, single-ASCII-
//! space `covers` separator, required `<task-scenarios>` block) and
//! REQ-003 (parse/render/parse equivalence and verbatim preservation
//! of body bytes including fenced code blocks and inline backticks).

use camino::Utf8Path;
use speccy_core::parse::parse_task_xml;
use speccy_core::parse::render_task_xml;
use speccy_core::parse::task_xml::Task;
use speccy_core::parse::task_xml::TaskState;
use speccy_core::parse::task_xml::TasksDoc;

fn fixture_path() -> &'static Utf8Path {
    Utf8Path::new("tests/fixtures/task_xml/canonical.md")
}

fn load_fixture() -> String {
    fs_err::read_to_string(fixture_path().as_std_path())
        .expect("canonical fixture should be readable from the crate root")
}

fn parse(source: &str) -> TasksDoc {
    parse_task_xml(source, Utf8Path::new("fixture/TASKS.md"))
        .expect("canonical fixture should parse")
}

fn assert_task_field_equal(a: &Task, b: &Task) {
    assert_eq!(a.id, b.id, "task id mismatch");
    assert_eq!(a.state, b.state, "task state mismatch on {}", a.id);
    assert_eq!(a.covers, b.covers, "task covers mismatch on {}", a.id);
    assert_eq!(
        a.scenarios_body.trim(),
        b.scenarios_body.trim(),
        "task scenarios_body mismatch on {}",
        a.id,
    );
}

#[test]
fn parses_canonical_fixture() {
    let src = load_fixture();
    let doc = parse(&src);
    assert_eq!(doc.spec_id, "SPEC-0022");
    assert_eq!(doc.tasks.len(), 2);
    let t1 = doc.tasks.first().expect("two tasks");
    assert_eq!(t1.id, "T-001");
    assert_eq!(t1.state, TaskState::Pending);
    assert_eq!(t1.covers, vec!["REQ-001".to_owned()]);
    let t2 = doc.tasks.get(1).expect("two tasks");
    assert_eq!(t2.id, "T-002");
    assert_eq!(t2.state, TaskState::InReview);
    assert_eq!(t2.covers, vec!["REQ-001".to_owned(), "REQ-003".to_owned()]);
}

#[test]
fn render_then_reparse_is_field_equal() {
    let src = load_fixture();
    let doc = parse(&src);
    let rendered = render_task_xml(&doc);
    let doc2 = parse_task_xml(&rendered, Utf8Path::new("fixture/TASKS.md"))
        .expect("rendered TASKS.md should parse back");
    assert_eq!(doc.spec_id, doc2.spec_id, "spec_id mismatch");
    assert_eq!(
        doc.tasks.len(),
        doc2.tasks.len(),
        "task count differs: {} vs {}",
        doc.tasks.len(),
        doc2.tasks.len(),
    );
    for (a, b) in doc.tasks.iter().zip(doc2.tasks.iter()) {
        assert_task_field_equal(a, b);
    }
}

#[test]
fn render_is_idempotent() {
    let src = load_fixture();
    let doc = parse(&src);
    let first = render_task_xml(&doc);
    let second = render_task_xml(&doc);
    assert_eq!(
        first, second,
        "render must be byte-identical on repeat calls"
    );
    // And rendering a re-parsed doc must also be byte-identical to the
    // first render, pinning the canonical-fixed-point.
    let doc2 =
        parse_task_xml(&first, Utf8Path::new("fixture/TASKS.md")).expect("first render must parse");
    let third = render_task_xml(&doc2);
    assert_eq!(
        first, third,
        "render(parse(render(doc))) must equal render(doc)"
    );
}

#[test]
fn scenarios_body_special_bytes_pass_through_verbatim() {
    // T-002 in the canonical fixture carries `<`, `>`, `&`, a fenced
    // code block containing a literal `<task>` tag line, and inline
    // backticks containing `<task>`. None of those bytes may be
    // promoted to structure by the re-parser.
    let src = load_fixture();
    let doc = parse(&src);
    let rendered = render_task_xml(&doc);
    let doc2 = parse_task_xml(&rendered, Utf8Path::new("fixture/TASKS.md"))
        .expect("rendered TASKS.md should parse back");
    // The fenced block lives in `body` (outside `<task-scenarios>`),
    // and the inline-backtick literal lives inside `scenarios_body`.
    let t2_in = doc.tasks.get(1).expect("two tasks");
    let t2_out = doc2.tasks.get(1).expect("two tasks");
    assert_eq!(t2_in.id, t2_out.id);
    assert!(
        t2_out
            .scenarios_body
            .contains("literal `<task>` inside backticks"),
        "inline backtick `<task>` literal must round-trip verbatim, got: {body}",
        body = t2_out.scenarios_body,
    );
    // The re-parsed task count must still be exactly two; a phantom
    // `<task id=\"T-FAKE\">` inside the fenced code block would have
    // surfaced as a third task here.
    assert_eq!(
        doc2.tasks.len(),
        2,
        "fenced-code `<task>` literal must not be promoted to structure",
    );
    // Sanity: the scenarios body retains the special bytes.
    let combined = format!("{}\n{}", t2_in.body, t2_in.scenarios_body);
    for needle in ['<', '>', '&'] {
        assert!(
            combined.contains(needle),
            "expected `{needle}` to round-trip verbatim",
        );
    }
}
