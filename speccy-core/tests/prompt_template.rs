#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::prompt::load_template`.
//! Covers SPEC-0005 REQ-005 (template loading) via the public API.

use speccy_core::prompt::PromptError;
use speccy_core::prompt::load_template;

#[test]
fn loads_plan_greenfield_template_from_embedded_bundle() {
    let body = load_template("plan-greenfield.md")
        .expect("plan-greenfield.md must ship in the embedded bundle");
    assert!(
        body.contains("{{next_spec_id}}"),
        "greenfield template missing {{next_spec_id}} placeholder",
    );
    assert!(
        body.contains("{{agents}}"),
        "greenfield template missing {{agents}} placeholder",
    );
    assert!(
        !body.contains("{{vision}}"),
        "greenfield template must not contain the retired {{vision}} placeholder",
    );
}

#[test]
fn loads_plan_amend_template_from_embedded_bundle() {
    let body =
        load_template("plan-amend.md").expect("plan-amend.md must ship in the embedded bundle");
    assert!(
        body.contains("{{spec_id}}"),
        "amend template missing {{spec_id}} placeholder",
    );
    assert!(
        body.contains("{{spec_md}}"),
        "amend template missing {{spec_md}} placeholder",
    );
    assert!(
        body.contains("{{mission}}"),
        "amend template missing {{mission}} placeholder for nearest-parent MISSION.md",
    );
}

// --------------------------------------------------------------------
// SPEC-0016 T-002: embedded path moved from `skills/shared/prompts/`
// to `resources/modules/prompts/`. The renderer must return the file
// body byte-identical to the on-disk source after the move.
// --------------------------------------------------------------------

/// On-disk source under the new T-002 location. If the move dropped or
/// rewrote the file, this `include_str!` fails to compile, surfacing
/// the regression before the test even runs.
const PLAN_GREENFIELD_SOURCE: &str =
    include_str!("../../resources/modules/prompts/plan-greenfield.md");

#[test]
fn t002_plan_greenfield_load_template_is_byte_identical_to_source() {
    let loaded = load_template("plan-greenfield.md")
        .expect("plan-greenfield.md must ship in the embedded bundle");
    assert_eq!(
        loaded, PLAN_GREENFIELD_SOURCE,
        "load_template must return the on-disk body byte-identical \
         after T-002 moved the source from skills/shared/prompts/ to \
         resources/modules/prompts/",
    );
}

#[test]
fn unknown_template_name_returns_template_not_found() {
    let err = load_template("nonexistent.md")
        .expect_err("unknown template name must return TemplateNotFound");
    assert!(
        matches!(err, PromptError::TemplateNotFound { ref name } if name == "nonexistent.md"),
        "expected TemplateNotFound{{ name: \"nonexistent.md\" }}, got {err:?}",
    );
}

// --------------------------------------------------------------------
// SPEC-0020 T-007: the shipped plan-greenfield prompt must teach the
// raw XML element grammar. The skill flow itself can only be executed
// by an LLM agent, so this test lowers the bar to the underlying
// contract: the template body must (a) contain a `<requirement` open
// tag in its example fixture, (b) contain a `<scenario` open tag in
// its example fixture, and (c) contain no `<!-- speccy:` substring
// anywhere — the SPEC-0019 carrier was fully superseded.
// --------------------------------------------------------------------

#[test]
fn spec_0020_plan_greenfield_template_teaches_xml_element_grammar() {
    let body = load_template("plan-greenfield.md")
        .expect("plan-greenfield.md must ship in the embedded bundle");
    assert!(
        body.contains("<requirement"),
        "plan-greenfield template must teach the `<requirement` open tag \
         after SPEC-0020 (raw XML element carrier)",
    );
    assert!(
        body.contains("<scenario"),
        "plan-greenfield template must teach the `<scenario` open tag \
         after SPEC-0020 (raw XML element carrier)",
    );
    assert!(
        !body.contains("<!-- speccy:"),
        "plan-greenfield template must not teach the SPEC-0019 legacy \
         `<!-- speccy:` marker form after SPEC-0020 superseded the carrier",
    );
}

#[test]
fn spec_0020_plan_amend_template_teaches_xml_element_grammar() {
    let body =
        load_template("plan-amend.md").expect("plan-amend.md must ship in the embedded bundle");
    assert!(
        body.contains("<requirement"),
        "plan-amend template must teach the `<requirement` open tag \
         after SPEC-0020 (raw XML element carrier)",
    );
    assert!(
        body.contains("<scenario"),
        "plan-amend template must teach the `<scenario` open tag \
         after SPEC-0020 (raw XML element carrier)",
    );
    assert!(
        !body.contains("<!-- speccy:"),
        "plan-amend template must not teach the SPEC-0019 legacy \
         `<!-- speccy:` marker form after SPEC-0020 superseded the carrier",
    );
}

#[test]
fn spec_0020_implementer_template_teaches_xml_element_grammar() {
    let body =
        load_template("implementer.md").expect("implementer.md must ship in the embedded bundle");
    assert!(
        body.contains("<scenario"),
        "implementer template must teach the `<scenario` element form \
         after SPEC-0020 (raw XML element carrier)",
    );
    assert!(
        !body.contains("<!-- speccy:"),
        "implementer template must not teach the SPEC-0019 legacy \
         `<!-- speccy:` marker form after SPEC-0020 superseded the carrier",
    );
}
