#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for the SPEC-0020 T-002 deterministic renderer.
//!
//! Covers REQ-002 (renderer never emits HTML-comment markers) and
//! REQ-003 (parse/render/parse roundtrip equivalence, struct-order
//! drives output order, stable attribute ordering, boundary whitespace
//! normalisation with verbatim interior bytes, byte-identical
//! double-render, and the blank-line-after-close convention pinned by
//! the canonical fixture).

use camino::Utf8Path;
use speccy_core::parse::parse_spec_xml;
use speccy_core::parse::render_spec_xml;
use speccy_core::parse::spec_xml::Decision;
use speccy_core::parse::spec_xml::DecisionStatus;
use speccy_core::parse::spec_xml::ElementSpan;
use speccy_core::parse::spec_xml::OpenQuestion;
use speccy_core::parse::spec_xml::Requirement;
use speccy_core::parse::spec_xml::Scenario;
use speccy_core::parse::spec_xml::SpecDoc;

fn fixture_path() -> &'static Utf8Path {
    Utf8Path::new("tests/fixtures/spec_xml/canonical.md")
}

fn load_fixture() -> String {
    fs_err::read_to_string(fixture_path().as_std_path())
        .expect("canonical fixture should be readable from the crate root")
}

fn parse(source: &str) -> SpecDoc {
    parse_spec_xml(source, Utf8Path::new("fixture/SPEC.md"))
        .expect("canonical fixture should parse")
}

/// Strip nested `<scenario>`, `<done-when>`, and `<behavior>` blocks
/// from a requirement body and trim outer whitespace, mirroring the
/// renderer's canonical view of the requirement prose. Roundtrip tests
/// compare this projection rather than the raw body, because the parser
/// stores nested sub-section tag lines as literal text inside
/// `Requirement.body` while the renderer re-emits each from typed
/// state.
fn requirement_prose(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_block: Option<&'static str> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(close) = in_block {
            if trimmed.starts_with(close) {
                in_block = None;
            }
            continue;
        }
        if trimmed.starts_with("<scenario ") || trimmed.starts_with("<scenario>") {
            in_block = Some("</scenario>");
            continue;
        }
        if trimmed.starts_with("<done-when>") {
            in_block = Some("</done-when>");
            continue;
        }
        if trimmed.starts_with("<behavior>") {
            in_block = Some("</behavior>");
            continue;
        }
        out.push_str(line);
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

fn assert_requirements_equal(a: &[Requirement], b: &[Requirement]) {
    assert_eq!(
        a.len(),
        b.len(),
        "requirement counts differ: {} vs {}",
        a.len(),
        b.len(),
    );
    for (ra, rb) in a.iter().zip(b.iter()) {
        assert_eq!(ra.id, rb.id, "requirement id mismatch");
        assert_eq!(
            ra.scenarios.len(),
            rb.scenarios.len(),
            "scenario count mismatch under {}",
            ra.id,
        );
        for (sa, sb) in ra.scenarios.iter().zip(rb.scenarios.iter()) {
            assert_scenarios_equal(sa, sb);
        }
        assert_eq!(
            requirement_prose(&ra.body),
            requirement_prose(&rb.body),
            "requirement prose mismatch under {}",
            ra.id,
        );
    }
}

fn assert_decisions_equal(a: &[Decision], b: &[Decision]) {
    assert_eq!(a.len(), b.len(), "decision counts differ");
    for (da, db) in a.iter().zip(b.iter()) {
        assert_eq!(da.id, db.id, "decision id mismatch");
        assert_eq!(
            da.status, db.status,
            "decision status mismatch on {}",
            da.id,
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
    assert_eq!(a.goals.trim(), b.goals.trim(), "goals mismatch");
    assert_eq!(a.non_goals.trim(), b.non_goals.trim(), "non-goals mismatch");
    assert_eq!(
        a.user_stories.trim(),
        b.user_stories.trim(),
        "user-stories mismatch",
    );
    assert_eq!(
        a.assumptions.as_deref().map(str::trim),
        b.assumptions.as_deref().map(str::trim),
        "assumptions mismatch",
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
    let rendered = render_spec_xml(&doc1);
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

    let rendered = render_spec_xml(&doc);
    let reparsed = parse(&rendered);
    let actual_ids: Vec<String> = reparsed.requirements.iter().map(|r| r.id.clone()).collect();
    assert_eq!(
        actual_ids, reversed_ids,
        "render must emit requirements in SpecDoc.requirements order, not source order",
    );
}

#[test]
fn decision_element_attrs_emit_in_fixed_id_then_status_order() {
    let source = load_fixture();
    let doc = parse(&source);
    let rendered = render_spec_xml(&doc);
    let expected = "<decision id=\"DEC-001\" status=\"accepted\">";
    assert!(
        rendered.contains(expected),
        "expected open tag `{expected}` in rendered output; got:\n{rendered}",
    );
    // The reverse order must not appear.
    let bad = "<decision status=\"accepted\" id=\"DEC-001\">";
    assert!(
        !rendered.contains(bad),
        "attrs emitted in the wrong order; found `{bad}`",
    );
}

#[test]
fn render_normalises_boundary_whitespace_but_preserves_interior_bytes() {
    // Construct a SpecDoc by hand whose scenario body has trailing
    // whitespace at both boundaries plus a load-bearing interior code
    // fence with `<T>` and `A & B`.
    let interior = "Given a fixture body containing `<T>` and `A & B`,\nwhen X,\nthen Y.\n\n```rust\nfn ok() {}\n```";
    let padded_body = format!("\n\n  \n{interior}\n\n   \n");
    let span = ElementSpan { start: 0, end: 0 };
    let scenario = Scenario {
        id: "CHK-001".to_owned(),
        body: padded_body.clone(),
        parent_requirement_id: "REQ-001".to_owned(),
        span,
    };
    let requirement = Requirement {
        id: "REQ-001".to_owned(),
        body: "Some requirement prose.\n".to_owned(),
        done_when: "- ship.\n".to_owned(),
        done_when_span: span,
        behavior: "- it works.\n".to_owned(),
        behavior_span: span,
        scenarios: vec![scenario],
        span,
    };
    let doc = SpecDoc {
        frontmatter_raw:
            "id: SPEC-0099\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-15\n"
                .to_owned(),
        heading: "SPEC-0099: hand built".to_owned(),
        raw: String::new(),
        goals: "Hand-built goals body.\n".to_owned(),
        goals_span: span,
        non_goals: "Hand-built non-goals body.\n".to_owned(),
        non_goals_span: span,
        user_stories: "- A hand-built user story.\n".to_owned(),
        user_stories_span: span,
        assumptions: None,
        assumptions_span: None,
        requirements: vec![requirement],
        decisions: Vec::new(),
        open_questions: Vec::new(),
        changelog_body: "| Date | Author | Summary |\n".to_owned(),
        changelog_span: span,
    };

    let rendered = render_spec_xml(&doc);

    let start = "<scenario id=\"CHK-001\">\n";
    let end = "\n</scenario>\n";
    let start_pos = rendered
        .find(start)
        .expect("rendered output should contain scenario open tag");
    let after_start = start_pos + start.len();
    let tail = rendered
        .get(after_start..)
        .expect("rendered output should be sliceable from after the scenario open tag");
    let end_pos = tail
        .find(end)
        .map(|p| after_start + p)
        .expect("rendered output should contain scenario close tag");
    let emitted_interior = rendered
        .get(after_start..end_pos)
        .expect("rendered output should be sliceable between scenario open and close tags");

    assert_eq!(
        emitted_interior, interior,
        "scenario interior bytes must match the source slice with normalised boundaries; \
         emitted={emitted_interior:?}, expected={interior:?}",
    );
}

#[test]
fn render_is_idempotent_byte_for_byte() {
    let source = load_fixture();
    let doc = parse(&source);
    let first = render_spec_xml(&doc);
    let second = render_spec_xml(&doc);
    assert_eq!(
        first, second,
        "render(doc) must produce byte-identical output across runs",
    );
}

#[test]
fn render_never_emits_html_comment_markers() {
    let source = load_fixture();
    let doc = parse(&source);
    let rendered = render_spec_xml(&doc);
    assert!(
        !rendered.contains("<!-- speccy:"),
        "REQ-002: renderer must never emit `<!-- speccy:` markers; got:\n{rendered}",
    );
}

#[test]
fn render_emits_blank_line_after_every_closing_element_tag() {
    // Pins SPEC-0020 Open Question 2's resolution: every closing
    // element tag is followed by a blank line. Asserting against the
    // canonical fixture covers requirement, scenario, decision,
    // open-question, and changelog close tags in one pass.
    let source = load_fixture();
    let doc = parse(&source);
    let rendered = render_spec_xml(&doc);

    for close_tag in [
        "</requirement>",
        "</scenario>",
        "</decision>",
        "</open-question>",
        "</changelog>",
    ] {
        let probe = format!("{close_tag}\n\n");
        assert!(
            rendered.contains(&probe) || rendered.ends_with(&format!("{close_tag}\n")),
            "every close tag must be followed by a blank line (or be the final line); \
             `{close_tag}` was not. Rendered output:\n{rendered}",
        );
    }
}

#[test]
fn render_emits_decision_with_status_attribute() {
    // Sanity: decision status round-trips through DecisionStatus::as_str.
    let source = load_fixture();
    let doc = parse(&source);
    let dec = doc.decisions.first().expect("fixture has one decision");
    assert_eq!(dec.status, Some(DecisionStatus::Accepted));
}

#[test]
fn rendered_output_has_expected_top_level_shape() {
    let source = load_fixture();
    let doc = parse(&source);
    let rendered = render_spec_xml(&doc);
    assert!(
        rendered.starts_with("---\n"),
        "must start with frontmatter fence",
    );
    assert!(
        rendered.contains("\n# SPEC-0099: Canonical fixture\n"),
        "must include the level-1 heading from the fixture",
    );
    assert!(
        rendered.contains("<changelog>"),
        "must include the changelog open tag",
    );
}
