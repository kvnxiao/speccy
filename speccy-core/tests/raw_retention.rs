#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Regression lock for the parsed-document `raw` field retention invariant.
//!
//! The `XML-001` foreign-tag-balance lint reads each parsed document's
//! full raw source from its `raw: String` field. These tests pin that the
//! `raw` field is byte-identical to the source passed to the parser, so a
//! future refactor cannot silently store a trimmed or normalised view and
//! starve the lint of the bytes it scans.

use camino::Utf8Path;
use speccy_core::parse::parse_report_xml;
use speccy_core::parse::parse_spec_xml;
use speccy_core::parse::parse_task_xml;

/// Reuse the same canonical valid fixtures the roundtrip tests parse,
/// rather than hand-rolling minimal sources.
fn read_fixture(rel: &str) -> String {
    fs_err::read_to_string(Utf8Path::new(rel).as_std_path())
        .expect("canonical fixture should be readable from the crate root")
}

#[test]
fn spec_doc_retains_byte_identical_raw_source() {
    let source = read_fixture("tests/fixtures/spec_xml/canonical.md");
    let doc = parse_spec_xml(&source, Utf8Path::new("fixture/SPEC.md"))
        .expect("canonical SPEC fixture should parse");

    assert_eq!(
        doc.raw, source,
        "SpecDoc.raw must be byte-identical to the parsed source"
    );
}

#[test]
fn tasks_doc_retains_byte_identical_raw_source() {
    let source = read_fixture("tests/fixtures/task_xml/canonical.md");
    let doc = parse_task_xml(&source, Utf8Path::new("fixture/TASKS.md"))
        .expect("canonical TASKS fixture should parse");

    assert_eq!(
        doc.raw, source,
        "TasksDoc.raw must be byte-identical to the parsed source"
    );
}

#[test]
fn report_doc_retains_byte_identical_raw_source() {
    let source = read_fixture("tests/fixtures/report_xml/canonical.md");
    let doc = parse_report_xml(&source, Utf8Path::new("fixture/REPORT.md"))
        .expect("canonical REPORT fixture should parse");

    assert_eq!(
        doc.raw, source,
        "ReportDoc.raw must be byte-identical to the parsed source"
    );
}
