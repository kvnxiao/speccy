#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for the SPEC-0022 T-003 REPORT.md typed model,
//! parser, and renderer.
//!
//! Covers REQ-002 (grammar shape, the three valid coverage results,
//! single-ASCII-space `scenarios` separator, deferred-may-be-empty rule)
//! and REQ-003 (parse/render/parse equivalence).

use camino::Utf8Path;
use speccy_core::parse::CoverageResult;
use speccy_core::parse::ReportDoc;
use speccy_core::parse::RequirementCoverage;
use speccy_core::parse::parse_report_xml;
use speccy_core::parse::render_report_xml;

fn fixture_path() -> &'static Utf8Path {
    Utf8Path::new("tests/fixtures/report_xml/canonical.md")
}

fn load_fixture() -> String {
    fs_err::read_to_string(fixture_path().as_std_path())
        .expect("canonical fixture should be readable from the crate root")
}

fn parse(source: &str) -> ReportDoc {
    parse_report_xml(source, Utf8Path::new("fixture/REPORT.md"))
        .expect("canonical fixture should parse")
}

fn assert_coverage_field_equal(a: &RequirementCoverage, b: &RequirementCoverage) {
    assert_eq!(a.req, b.req, "coverage req mismatch");
    assert_eq!(a.result, b.result, "coverage result mismatch on {}", a.req);
    assert_eq!(
        a.scenarios, b.scenarios,
        "coverage scenarios mismatch on {}",
        a.req
    );
    assert_eq!(
        a.body.trim(),
        b.body.trim(),
        "coverage body mismatch on {}",
        a.req,
    );
}

#[test]
fn parses_canonical_fixture() {
    let src = load_fixture();
    let doc = parse(&src);
    assert_eq!(doc.spec_id, "SPEC-0022");
    assert_eq!(doc.coverage.len(), 3);
    let c1 = doc.coverage.first().expect("three rows");
    assert_eq!(c1.req, "REQ-001");
    assert_eq!(c1.result, CoverageResult::Satisfied);
    assert_eq!(c1.scenarios, vec!["CHK-001".to_owned()]);
    let c2 = doc.coverage.get(1).expect("three rows");
    assert_eq!(c2.req, "REQ-002");
    assert_eq!(c2.result, CoverageResult::Partial);
    assert_eq!(
        c2.scenarios,
        vec!["CHK-002".to_owned(), "CHK-003".to_owned()]
    );
    let c3 = doc.coverage.get(2).expect("three rows");
    assert_eq!(c3.req, "REQ-003");
    assert_eq!(c3.result, CoverageResult::Deferred);
    assert!(c3.scenarios.is_empty());
}

#[test]
fn render_then_reparse_is_field_equal() {
    let src = load_fixture();
    let doc = parse(&src);
    let rendered = render_report_xml(&doc);
    let doc2 = parse_report_xml(&rendered, Utf8Path::new("fixture/REPORT.md"))
        .expect("rendered REPORT.md should parse back");
    assert_eq!(doc.spec_id, doc2.spec_id, "spec_id mismatch");
    assert_eq!(
        doc.coverage.len(),
        doc2.coverage.len(),
        "coverage count differs",
    );
    for (a, b) in doc.coverage.iter().zip(doc2.coverage.iter()) {
        assert_coverage_field_equal(a, b);
    }
}

#[test]
fn render_is_idempotent() {
    let src = load_fixture();
    let doc = parse(&src);
    let first = render_report_xml(&doc);
    let second = render_report_xml(&doc);
    assert_eq!(
        first, second,
        "render must be byte-identical on repeat calls"
    );
    let doc2 = parse_report_xml(&first, Utf8Path::new("fixture/REPORT.md"))
        .expect("first render must parse");
    let third = render_report_xml(&doc2);
    assert_eq!(
        first, third,
        "render(parse(render(doc))) must equal render(doc)"
    );
}
