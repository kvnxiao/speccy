#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::string_slice,
    reason = "test asserts byte-exact slices of the rendered ASCII output"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "test asserts byte-exact slices of the rendered ASCII output"
)]
//! Integration tests for the SPEC-0019 T-002 deterministic renderer.
//!
//! Covers REQ-003: parse/render/parse roundtrip equivalence, struct-order
//! drives output order, stable attribute ordering, boundary whitespace
//! normalization with verbatim interior bytes, and byte-identical
//! double-render output.

use camino::Utf8Path;
use speccy_core::parse::Decision;
use speccy_core::parse::DecisionStatus;
use speccy_core::parse::OpenQuestion;
use speccy_core::parse::Requirement;
use speccy_core::parse::Scenario;
use speccy_core::parse::SpecDoc;
use speccy_core::parse::parse_spec_markers;
use speccy_core::parse::render_spec_markers;

fn fixture_path() -> &'static Utf8Path {
    Utf8Path::new("tests/fixtures/spec_markers/canonical.md")
}

fn load_fixture() -> String {
    fs_err::read_to_string(fixture_path().as_std_path())
        .expect("canonical fixture should be readable from the crate root")
}

fn parse(source: &str) -> SpecDoc {
    parse_spec_markers(source, Utf8Path::new("fixture/SPEC.md"))
        .expect("canonical fixture should parse")
}

fn assert_requirements_equal(a: &[Requirement], b: &[Requirement]) {
    assert_eq!(
        a.len(),
        b.len(),
        "requirement counts differ: {} vs {}",
        a.len(),
        b.len()
    );
    for (ra, rb) in a.iter().zip(b.iter()) {
        assert_eq!(ra.id, rb.id, "requirement id mismatch");
        assert_eq!(
            ra.scenarios.len(),
            rb.scenarios.len(),
            "scenario count mismatch under {}",
            ra.id
        );
        for (sa, sb) in ra.scenarios.iter().zip(rb.scenarios.iter()) {
            assert_scenarios_equal(sa, sb);
        }
        // The parser stores `Requirement.body` as the verbatim source
        // slice between the requirement's start and end markers, which
        // includes nested scenario marker lines as literal text. The
        // renderer re-emits scenarios from typed state, so we compare
        // the *prose* portion (scenarios stripped) rather than the
        // full body string.
        assert_eq!(
            requirement_prose(&ra.body),
            requirement_prose(&rb.body),
            "requirement prose mismatch under {}",
            ra.id,
        );
    }
}

/// Strip nested `speccy:scenario` blocks from a requirement body and
/// collapse outer whitespace, mirroring the renderer's canonical view
/// of the requirement prose.
fn requirement_prose(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_scenario = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !in_scenario {
            if trimmed.starts_with("<!-- speccy:scenario") {
                in_scenario = true;
                continue;
            }
            out.push_str(line);
        } else if trimmed.starts_with("<!-- /speccy:scenario") {
            in_scenario = false;
        }
    }
    out.trim().to_owned()
}

fn assert_scenarios_equal(a: &Scenario, b: &Scenario) {
    assert_eq!(a.id, b.id, "scenario id mismatch");
    assert_eq!(
        a.parent_requirement_id, b.parent_requirement_id,
        "scenario parent link mismatch on {}",
        a.id,
    );
    assert_eq!(
        a.body.trim(),
        b.body.trim(),
        "scenario body mismatch on {}",
        a.id,
    );
}

fn assert_decisions_equal(a: &[Decision], b: &[Decision]) {
    assert_eq!(a.len(), b.len(), "decision counts differ");
    for (da, db) in a.iter().zip(b.iter()) {
        assert_eq!(da.id, db.id, "decision id mismatch");
        assert_eq!(
            da.status, db.status,
            "decision status mismatch on {}",
            da.id
        );
        assert_eq!(
            da.body.trim(),
            db.body.trim(),
            "decision body mismatch on {}",
            da.id,
        );
    }
}

fn assert_open_questions_equal(a: &[OpenQuestion], b: &[OpenQuestion]) {
    assert_eq!(a.len(), b.len(), "open-question counts differ");
    for (qa, qb) in a.iter().zip(b.iter()) {
        assert_eq!(qa.resolved, qb.resolved, "open-question resolved mismatch");
        assert_eq!(
            qa.body.trim(),
            qb.body.trim(),
            "open-question body mismatch",
        );
    }
}

fn assert_specdocs_structurally_equal(a: &SpecDoc, b: &SpecDoc) {
    assert_eq!(a.heading, b.heading, "heading mismatch");
    assert_requirements_equal(&a.requirements, &b.requirements);
    assert_decisions_equal(&a.decisions, &b.decisions);
    assert_open_questions_equal(&a.open_questions, &b.open_questions);
    assert_eq!(
        a.summary.as_deref().map(str::trim),
        b.summary.as_deref().map(str::trim),
        "summary mismatch",
    );
    assert_eq!(
        a.changelog_body.trim(),
        b.changelog_body.trim(),
        "changelog body mismatch",
    );
}

#[test]
fn parse_render_parse_roundtrip_is_structurally_equivalent() {
    let source = load_fixture();
    let doc1 = parse(&source);
    let rendered = render_spec_markers(&doc1);
    let doc2 = parse(&rendered);
    assert_specdocs_structurally_equal(&doc1, &doc2);
}

#[test]
fn render_emits_requirements_in_struct_order_not_source_order() {
    let source = load_fixture();
    let mut doc = parse(&source);
    let original_ids: Vec<String> = doc.requirements.iter().map(|r| r.id.clone()).collect();
    doc.requirements.reverse();
    let reversed_ids: Vec<String> = doc.requirements.iter().map(|r| r.id.clone()).collect();
    assert_ne!(
        original_ids, reversed_ids,
        "test precondition: fixture should have >1 requirement",
    );

    let rendered = render_spec_markers(&doc);
    let reparsed = parse(&rendered);
    let actual_ids: Vec<String> = reparsed.requirements.iter().map(|r| r.id.clone()).collect();
    assert_eq!(
        actual_ids, reversed_ids,
        "render must emit requirements in SpecDoc.requirements order, not source order",
    );
}

#[test]
fn decision_marker_attrs_emit_in_fixed_id_then_status_order() {
    let source = load_fixture();
    let doc = parse(&source);
    let rendered = render_spec_markers(&doc);
    let expected = "<!-- speccy:decision id=\"DEC-001\" status=\"accepted\" -->";
    assert!(
        rendered.contains(expected),
        "expected marker line `{expected}` in rendered output; got:\n{rendered}",
    );
    // The reverse order must not appear.
    let bad = "<!-- speccy:decision status=\"accepted\" id=\"DEC-001\" -->";
    assert!(
        !rendered.contains(bad),
        "attrs emitted in the wrong order; found `{bad}`",
    );
}

#[test]
fn render_normalizes_boundary_whitespace_but_preserves_interior_bytes() {
    // Construct a SpecDoc by hand whose scenario body has trailing
    // whitespace at both boundaries plus a load-bearing interior code
    // fence with `<T>` and `A & B`.
    let interior = "Given a fixture body containing `<T>` and `A & B`,\nwhen X,\nthen Y.\n\n```rust\nfn ok() {}\n```";
    let padded_body = format!("\n\n  \n{interior}\n\n   \n");
    let span = speccy_core::parse::MarkerSpan { start: 0, end: 0 };
    let scenario = Scenario {
        id: "CHK-001".to_owned(),
        body: padded_body.clone(),
        parent_requirement_id: "REQ-001".to_owned(),
        span,
    };
    let requirement = Requirement {
        id: "REQ-001".to_owned(),
        body: "Some requirement prose.\n".to_owned(),
        scenarios: vec![scenario],
        span,
    };
    let doc = SpecDoc {
        frontmatter_raw:
            "id: SPEC-0099\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-15\n"
                .to_owned(),
        heading: "SPEC-0099: hand built".to_owned(),
        raw: String::new(),
        requirements: vec![requirement],
        decisions: Vec::new(),
        open_questions: Vec::new(),
        changelog_body: "| Date | Author | Summary |\n".to_owned(),
        changelog_span: span,
        summary: None,
        summary_span: None,
    };

    let rendered = render_spec_markers(&doc);

    let start = "<!-- speccy:scenario id=\"CHK-001\" -->\n";
    let end = "\n<!-- /speccy:scenario -->\n";
    let start_pos = rendered
        .find(start)
        .expect("rendered output should contain scenario start marker");
    let after_start = start_pos + start.len();
    let end_pos = rendered[after_start..]
        .find(end)
        .map(|p| after_start + p)
        .expect("rendered output should contain scenario end marker");
    let emitted_interior = &rendered[after_start..end_pos];

    assert_eq!(
        emitted_interior, interior,
        "scenario interior bytes must match the source slice with normalized boundaries; \
         emitted={emitted_interior:?}, expected={interior:?}",
    );
}

#[test]
fn render_is_idempotent_byte_for_byte() {
    let source = load_fixture();
    let doc = parse(&source);
    let first = render_spec_markers(&doc);
    let second = render_spec_markers(&doc);
    assert_eq!(
        first, second,
        "render(doc) must produce byte-identical output across runs",
    );
}

#[test]
fn rendered_output_is_parseable_and_has_expected_top_level_shape() {
    let source = load_fixture();
    let doc = parse(&source);
    let rendered = render_spec_markers(&doc);
    assert!(
        rendered.starts_with("---\n"),
        "must start with frontmatter fence"
    );
    assert!(
        rendered.contains("\n# SPEC-0099: Canonical fixture\n"),
        "must include the level-1 heading from the fixture",
    );
    assert!(
        rendered.contains("<!-- speccy:changelog -->"),
        "must include the changelog marker",
    );
    // Decision status round-trips through DecisionStatus::as_str.
    let dec = doc.decisions.first().expect("fixture has one decision");
    assert_eq!(dec.status, Some(DecisionStatus::Accepted));
}
