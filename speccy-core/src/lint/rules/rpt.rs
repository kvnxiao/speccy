//! RPT-* rules: REPORT.md proof-shape consistency.
//!
//! Three rules gate `speccy verify` on the same structural invariants the
//! in-tree integration test
//! `every_in_tree_report_md_parses_and_resolves_against_parent_spec`
//! checks under `cargo test`:
//!
//! - `RPT-001` — REPORT.md is present but failed to parse.
//! - `RPT-002` — a `<coverage req="REQ-NNN">` row references a requirement id
//!   that does not appear in the sibling SPEC.md.
//! - `RPT-003` — a scenario id in `<coverage scenarios="...">` does not resolve
//!   under the row's requirement in the sibling SPEC.md.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;

const RPT_001: &str = "RPT-001";
const RPT_002: &str = "RPT-002";
const RPT_003: &str = "RPT-003";

/// Append every RPT-* diagnostic for one spec.
///
/// Short-circuits silently when `spec.report_md` is `None` (REPORT.md
/// absent — the common mid-loop state).
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(report_result) = &spec.report_md else {
        return;
    };

    // RPT-001: REPORT.md present but failed to parse.
    let report_doc = match report_result {
        Ok(doc) => doc,
        Err(err) => {
            out.push(Diagnostic::with_file(
                RPT_001,
                Level::Error,
                spec.spec_id.clone(),
                spec.dir.join("REPORT.md"),
                format!("REPORT.md failed to parse: {err}"),
            ));
            return;
        }
    };

    // RPT-002 and RPT-003 require a successfully-parsed SPEC.md. When
    // spec_doc failed to parse, SPC-001 already surfaces that failure;
    // emitting dangling-ref diagnostics on top would be noise.
    let Some(spec_doc) = spec.spec_doc_ok() else {
        return;
    };

    for cov in &report_doc.coverage {
        // RPT-002: coverage row references a requirement id not in SPEC.md.
        // RPT-003 makes no sense when the requirement is missing, so skip the row.
        let Some(req) = spec_doc.requirements.iter().find(|r| r.id == cov.req) else {
            out.push(Diagnostic::with_file(
                RPT_002,
                Level::Error,
                spec.spec_id.clone(),
                spec.dir.join("REPORT.md"),
                format!(
                    "`{req}` in <coverage> does not match any <requirement id=\"{req}\"> in SPEC.md",
                    req = cov.req,
                ),
            ));
            continue;
        };
        for scenario_id in &cov.scenarios {
            let found = req.scenarios.iter().any(|s| &s.id == scenario_id);
            if !found {
                out.push(Diagnostic::with_file(
                    RPT_003,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.dir.join("REPORT.md"),
                    format!(
                        "scenario `{scenario_id}` in <coverage req=\"{req_id}\"> does not match any <scenario id=\"{scenario_id}\"> nested under `{req_id}` in SPEC.md",
                        scenario_id = scenario_id,
                        req_id = cov.req,
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::lint;
    use crate::error::ParseError;
    use crate::lint::types::Diagnostic;
    use crate::lint::types::Level;
    use crate::lint::types::ParsedSpec;
    use crate::parse::ReportDoc;
    use crate::parse::RequirementCoverage;
    use crate::parse::SpecDoc;
    use crate::parse::report_xml::CoverageResult;
    use crate::parse::xml_scanner::ElementSpan;
    use camino::Utf8PathBuf;

    fn zero_span() -> ElementSpan {
        ElementSpan { start: 0, end: 0 }
    }

    fn make_spec_doc_with_req(req_id: &str, scenario_ids: &[&str]) -> SpecDoc {
        let scenario = |sid: &str| crate::parse::Scenario {
            id: sid.to_owned(),
            body: String::new(),
            parent_requirement_id: req_id.to_owned(),
            span: zero_span(),
        };
        let req = crate::parse::Requirement {
            id: req_id.to_owned(),
            body: String::new(),
            done_when: String::new(),
            done_when_span: zero_span(),
            behavior: String::new(),
            behavior_span: zero_span(),
            scenarios: scenario_ids.iter().map(|s| scenario(s)).collect(),
            span: zero_span(),
        };
        SpecDoc {
            frontmatter_raw: String::new(),
            heading: String::new(),
            raw: String::new(),
            goals: String::new(),
            goals_span: zero_span(),
            non_goals: String::new(),
            non_goals_span: zero_span(),
            user_stories: String::new(),
            user_stories_span: zero_span(),
            assumptions: None,
            assumptions_span: None,
            requirements: vec![req],
            decisions: vec![],
            open_questions: vec![],
            changelog_body: String::new(),
            changelog_span: zero_span(),
        }
    }

    fn make_report_doc(coverage: Vec<RequirementCoverage>) -> ReportDoc {
        ReportDoc {
            frontmatter_raw: String::new(),
            heading: String::new(),
            raw: String::new(),
            spec_id: "SPEC-0035".to_owned(),
            report_span: zero_span(),
            coverage,
        }
    }

    fn make_coverage(req: &str, scenario_ids: &[&str]) -> RequirementCoverage {
        RequirementCoverage {
            req: req.to_owned(),
            result: CoverageResult::Satisfied,
            scenarios: scenario_ids.iter().map(ToString::to_string).collect(),
            body: String::new(),
            span: zero_span(),
        }
    }

    fn parsed_spec_dir() -> Utf8PathBuf {
        Utf8PathBuf::from(".speccy/specs/test-spec")
    }

    fn parsed_spec_md_path() -> Utf8PathBuf {
        parsed_spec_dir().join("SPEC.md")
    }

    fn make_parsed_spec_no_report() -> ParsedSpec {
        // Use an Err for spec_md — rpt::lint never reads it.
        let spec_md_err: crate::error::ParseResult<crate::parse::SpecMd> =
            Err(Box::new(ParseError::MissingField {
                field: "unused-in-rpt-tests".to_owned(),
                context: "test fixture".to_owned(),
            }));
        ParsedSpec {
            spec_id: Some("SPEC-0035".to_owned()),
            dir: parsed_spec_dir(),
            spec_md_path: parsed_spec_md_path(),
            tasks_md_path: None,
            mission_md_path: None,
            spec_md: spec_md_err,
            spec_doc: Ok(make_spec_doc_with_req("REQ-001", &["CHK-001"])),
            tasks_md: None,
            report_md: None,
        }
    }

    #[test]
    fn no_report_md_emits_nothing() {
        let spec = make_parsed_spec_no_report();
        let mut diags: Vec<Diagnostic> = Vec::new();
        lint(&spec, &mut diags);
        assert!(
            diags.is_empty(),
            "no RPT diagnostics when report_md is None"
        );
    }

    #[test]
    fn rpt_001_fires_on_parse_failure() {
        let mut spec = make_parsed_spec_no_report();
        let parse_err: crate::error::ParseResult<ReportDoc> =
            Err(Box::new(ParseError::MissingField {
                field: "<report>".to_owned(),
                context: "REPORT.md at fixture".to_owned(),
            }));
        spec.report_md = Some(parse_err);

        let mut diags: Vec<Diagnostic> = Vec::new();
        lint(&spec, &mut diags);

        assert_eq!(diags.len(), 1, "exactly one diagnostic");
        let d = diags.first().expect("one diagnostic");
        assert_eq!(d.code, "RPT-001");
        assert_eq!(d.level, Level::Error);
        assert_eq!(d.file, Some(spec.dir.join("REPORT.md")));
        assert!(d.line.is_none(), "RPT-001 has no line number");
    }

    #[test]
    fn rpt_002_fires_on_dangling_req_not_on_spec_doc_err() {
        // When spec_doc failed to parse, RPT-002 must NOT fire.
        let mut spec = make_parsed_spec_no_report();
        spec.spec_doc = Err(Box::new(ParseError::MissingField {
            field: "spec_doc".to_owned(),
            context: "fixture".to_owned(),
        }));
        let cov = make_coverage("REQ-001", &["CHK-001"]);
        spec.report_md = Some(Ok(make_report_doc(vec![cov])));

        let mut diags: Vec<Diagnostic> = Vec::new();
        lint(&spec, &mut diags);

        assert!(
            diags.is_empty(),
            "RPT-002 must not fire when spec_doc failed to parse; got: {diags:?}"
        );
    }

    #[test]
    fn rpt_002_fires_once_per_dangling_req() {
        let mut spec = make_parsed_spec_no_report();
        // spec_doc has only REQ-001; REPORT.md references REQ-999.
        let cov = make_coverage("REQ-999", &["CHK-001"]);
        spec.report_md = Some(Ok(make_report_doc(vec![cov])));

        let mut diags: Vec<Diagnostic> = Vec::new();
        lint(&spec, &mut diags);

        let rpt_002: Vec<_> = diags.iter().filter(|d| d.code == "RPT-002").collect();
        assert_eq!(rpt_002.len(), 1, "one RPT-002 for one dangling req");
        assert!(
            rpt_002
                .first()
                .expect("one RPT-002 diagnostic")
                .message
                .contains("REQ-999")
        );

        // No RPT-003 because the row short-circuited.
        let rpt_003: Vec<_> = diags.iter().filter(|d| d.code == "RPT-003").collect();
        assert!(
            rpt_003.is_empty(),
            "RPT-003 must not fire when req was missing"
        );
    }

    #[test]
    fn rpt_003_fires_once_per_dangling_scenario_not_for_valid_ones() {
        let mut spec = make_parsed_spec_no_report();
        // spec_doc has REQ-001 with CHK-001; REPORT.md references CHK-001 and CHK-999.
        let cov = make_coverage("REQ-001", &["CHK-001", "CHK-999"]);
        spec.report_md = Some(Ok(make_report_doc(vec![cov])));

        let mut diags: Vec<Diagnostic> = Vec::new();
        lint(&spec, &mut diags);

        let rpt_003: Vec<_> = diags.iter().filter(|d| d.code == "RPT-003").collect();
        assert_eq!(rpt_003.len(), 1, "one RPT-003 for CHK-999 only");
        let rpt_003_diag = rpt_003.first().expect("one RPT-003 diagnostic");
        assert!(rpt_003_diag.message.contains("CHK-999"));
        assert!(rpt_003_diag.message.contains("REQ-001"));

        // CHK-001 is valid; no diagnostic for it.
        assert!(
            !rpt_003.iter().any(|d| d.message.contains("CHK-001")),
            "CHK-001 is valid; no RPT-003 for it"
        );
    }

    #[test]
    fn well_formed_report_emits_no_rpt_diagnostics() {
        let mut spec = make_parsed_spec_no_report();
        // All rows resolve.
        let cov = make_coverage("REQ-001", &["CHK-001"]);
        spec.report_md = Some(Ok(make_report_doc(vec![cov])));

        let mut diags: Vec<Diagnostic> = Vec::new();
        lint(&spec, &mut diags);

        assert!(
            diags.is_empty(),
            "zero diagnostics for a well-formed REPORT.md; got: {diags:?}"
        );
    }
}
