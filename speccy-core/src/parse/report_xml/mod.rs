//! Raw-XML-element-structured REPORT.md parser.
//!
//! Reads a REPORT.md whose body is ordinary Markdown plus line-isolated raw
//! XML open/close tag pairs drawn from a small closed whitelist
//! (`report`, `coverage`) and returns a typed [`ReportDoc`]. Reuses the
//! shared scanner ([`crate::parse::xml_scanner`]) so
//! fenced-code-block awareness and tag-shape diagnostics are identical to
//! SPEC.md and TASKS.md parsing.

use crate::error::ParseError;
use crate::error::ParseResult;
use crate::parse::frontmatter::extract_level1_heading;
use crate::parse::frontmatter::split_required;
use crate::parse::xml_scanner::ElementSpan;
use crate::parse::xml_scanner::RawTag;
use crate::parse::xml_scanner::ScanConfig;
use crate::parse::xml_scanner::collect_code_fence_byte_ranges;
use crate::parse::xml_scanner::scan_tags;
use crate::parse::xml_scanner::unknown_attribute_error;
use camino::Utf8Path;
use regex::Regex;
use std::sync::OnceLock;

/// Closed whitelist of Speccy structure element names recognised inside
/// REPORT.md.
pub const REPORT_ELEMENT_NAMES: &[&str] = &["report", "coverage"];

/// Closed set of valid `<coverage result="...">` values, in their
/// on-disk form. Dropped requirements are removed from SPEC.md via
/// amendment rather than carried as a coverage row, so no `dropped`
/// value exists.
pub const ALLOWED_COVERAGE_RESULTS: &[&str] = &["satisfied", "partial", "deferred"];

/// Parsed raw-XML-structured REPORT.md.
///
/// `frontmatter_raw` carries the YAML frontmatter payload verbatim; the
/// `report` parser does not re-validate it. `heading` is the level-1
/// heading text after `# `, trimmed. `spec_id` is the `spec="..."`
/// attribute on the root `<report>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDoc {
    /// YAML frontmatter payload between the opening and closing `---`
    /// fences, verbatim.
    pub frontmatter_raw: String,
    /// Text of the level-1 heading after the `# ` prefix, trimmed.
    pub heading: String,
    /// Raw source bytes, retained so [`ElementSpan`] indices remain valid.
    pub raw: String,
    /// `spec="..."` attribute value on the root `<report>` element
    /// (e.g. `"SPEC-NNNN"`).
    pub spec_id: String,
    /// Span of the root `<report>` open tag.
    pub report_span: ElementSpan,
    /// Coverage rows declared by `<coverage>` elements in source order.
    pub coverage: Vec<RequirementCoverage>,
}

/// Closed set of `<coverage result="...">` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageResult {
    /// `satisfied` — the requirement is fully met. Requires ≥1 scenario id.
    Satisfied,
    /// `partial` — the requirement is partially met. Requires ≥1 scenario id.
    Partial,
    /// `deferred` — the requirement is deferred. May carry an empty
    /// `scenarios` attribute.
    Deferred,
}

impl CoverageResult {
    /// Render back to the on-disk string form.
    #[must_use = "the rendered result is the on-disk form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            CoverageResult::Satisfied => "satisfied",
            CoverageResult::Partial => "partial",
            CoverageResult::Deferred => "deferred",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "satisfied" => Some(CoverageResult::Satisfied),
            "partial" => Some(CoverageResult::Partial),
            "deferred" => Some(CoverageResult::Deferred),
            _ => None,
        }
    }
}

/// One coverage row (`<coverage>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementCoverage {
    /// Requirement id from the `req="..."` attribute (matches `REQ-\d{3,}`).
    pub req: String,
    /// `result="..."` attribute value, parsed.
    pub result: CoverageResult,
    /// `scenarios="..."` attribute value parsed into a list of `CHK-\d{3,}`
    /// ids in source order. May be empty for `deferred`.
    pub scenarios: Vec<String>,
    /// Verbatim body between `<coverage>` and `</coverage>` open and close
    /// tags.
    pub body: String,
    /// Span of the `<coverage>` open tag.
    pub span: ElementSpan,
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn spec_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^SPEC-\d{3,}$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn req_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^REQ-\d{3,}$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn chk_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^CHK-\d{3,}$").unwrap())
}

/// Run the shared XML scanner with the REPORT.md whitelist. Centralising
/// this matches `task_xml::scan_task_tags` so callers have a single
/// grep target for "what tags does REPORT.md recognise".
fn scan_report_tags(
    source: &str,
    body: &str,
    body_offset: usize,
    path: &Utf8Path,
) -> ParseResult<Vec<RawTag>> {
    let code_fence_ranges = collect_code_fence_byte_ranges(source);
    let cfg = ScanConfig {
        whitelist: REPORT_ELEMENT_NAMES,
        structure_shaped_names: REPORT_ELEMENT_NAMES,
    };
    scan_tags(source, body, body_offset, &code_fence_ranges, path, &cfg)
}

/// Parse a raw-XML-structured REPORT.md source string.
///
/// `source` is the file contents; `path` is used only to populate
/// diagnostics — this function does no filesystem IO.
///
/// # Errors
///
/// Returns [`ParseError`] for missing frontmatter or level-1 heading,
/// element-shape problems, unknown element names or attributes,
/// id-pattern violations, invalid coverage results, invalid `scenarios`
/// formats, or coverage rows whose `result` requires a non-empty
/// `scenarios` attribute but did not carry one.
pub fn parse(source: &str, path: &Utf8Path) -> ParseResult<ReportDoc> {
    let (frontmatter_raw, body, body_offset) = split_required(source, path, "REPORT.md")?;

    let heading = extract_level1_heading(body, path, "REPORT.md")?;

    let raw_tags = scan_report_tags(source, body, body_offset, path)?;

    // Up-front shape validation so unknown attributes / id-pattern
    // violations fail before we try to assemble nested blocks.
    for t in &raw_tags {
        validate_tag_shape(t, path)?;
    }

    let tree = assemble(raw_tags, source, path)?;

    // The REPORT.md root contract: exactly one `<report spec="...">`
    // element wrapping zero or more `<coverage>` children.
    let mut root: Option<(String, ElementSpan, Vec<Block>)> = None;
    for block in tree {
        match block {
            Block::Report {
                spec_id,
                span,
                children,
            } => {
                if root.is_some() {
                    return Err(Box::new(ParseError::MalformedMarker {
                        path: path.to_path_buf(),
                        offset: span.start,
                        reason: "more than one <report> root element".to_owned(),
                    }));
                }
                root = Some((spec_id, span, children));
            }
            Block::Coverage { span, .. } => {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<coverage> element must be nested inside <report>".to_owned(),
                }));
            }
        }
    }

    let (spec_id, report_span, children) = root.ok_or_else(|| {
        Box::new(ParseError::MissingField {
            field: "<report>".to_owned(),
            context: format!("REPORT.md at {path}"),
        })
    })?;

    let mut coverage: Vec<RequirementCoverage> = Vec::new();
    for child in children {
        match child {
            Block::Coverage { attrs, body, span } => {
                let row = build_coverage(&attrs, body, span, path)?;
                coverage.push(row);
            }
            Block::Report { span, .. } => {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<report> element must not be nested".to_owned(),
                }));
            }
        }
    }

    Ok(ReportDoc {
        frontmatter_raw,
        heading,
        raw: source.to_owned(),
        spec_id,
        report_span,
        coverage,
    })
}

fn build_coverage(
    attrs: &[(String, String)],
    body: String,
    span: ElementSpan,
    path: &Utf8Path,
) -> ParseResult<RequirementCoverage> {
    // req
    let req = find_attr(attrs, "req").ok_or_else(|| {
        Box::new(ParseError::MissingCoverageAttribute {
            path: path.to_path_buf(),
            attribute: "req".to_owned(),
            offset: span.start,
        })
    })?;
    if !req_id_regex().is_match(&req) {
        return Err(Box::new(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: "coverage".to_owned(),
            id: req.clone(),
            expected_pattern: r"REQ-\d{3,}".to_owned(),
        }));
    }

    // result
    let result_raw = find_attr(attrs, "result").ok_or_else(|| {
        Box::new(ParseError::MissingCoverageAttribute {
            path: path.to_path_buf(),
            attribute: "result".to_owned(),
            offset: span.start,
        })
    })?;
    let result = CoverageResult::from_str(&result_raw).ok_or_else(|| {
        Box::new(ParseError::InvalidCoverageResult {
            path: path.to_path_buf(),
            req: req.clone(),
            value: result_raw.clone(),
            allowed: ALLOWED_COVERAGE_RESULTS.join(", "),
        })
    })?;

    // scenarios — must be *present*, but may be empty for deferred.
    let scenarios_raw = find_attr(attrs, "scenarios").ok_or_else(|| {
        Box::new(ParseError::MissingCoverageAttribute {
            path: path.to_path_buf(),
            attribute: "scenarios".to_owned(),
            offset: span.start,
        })
    })?;
    let scenarios = parse_scenarios(&scenarios_raw, &req, path)?;

    match result {
        CoverageResult::Satisfied if scenarios.is_empty() => {
            return Err(Box::new(ParseError::SatisfiedRequiresScenarios {
                path: path.to_path_buf(),
                req: req.clone(),
            }));
        }
        CoverageResult::Partial if scenarios.is_empty() => {
            return Err(Box::new(ParseError::PartialRequiresScenarios {
                path: path.to_path_buf(),
                req: req.clone(),
            }));
        }
        _ => {}
    }

    Ok(RequirementCoverage {
        req,
        result,
        scenarios,
        body,
        span,
    })
}

fn find_attr(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Parse a `scenarios="..."` value into a list of `CHK-NNN` ids.
///
/// Grammar: zero or more `CHK-\d{3,}` ids separated
/// by single ASCII spaces. The attribute being entirely empty is OK; this
/// function returns an empty vec in that case (the caller enforces the
/// per-result minimum). Leading or trailing whitespace, double spaces,
/// tabs, and any non-`CHK-\d{3,}` token all fail with
/// [`ParseError::InvalidScenariosFormat`], whose Display quotes the
/// grammar verbatim.
fn parse_scenarios(raw: &str, req: &str, path: &Utf8Path) -> ParseResult<Vec<String>> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    for ch in raw.chars() {
        if ch == '\t' || ch == '\r' || ch == '\n' {
            return Err(Box::new(ParseError::InvalidScenariosFormat {
                path: path.to_path_buf(),
                req: req.to_owned(),
                value: raw.to_owned(),
            }));
        }
    }
    let mut scenarios: Vec<String> = Vec::new();
    for token in raw.split(' ') {
        if !chk_id_regex().is_match(token) {
            return Err(Box::new(ParseError::InvalidScenariosFormat {
                path: path.to_path_buf(),
                req: req.to_owned(),
                value: raw.to_owned(),
            }));
        }
        scenarios.push(token.to_owned());
    }
    Ok(scenarios)
}

#[derive(Debug)]
enum Block {
    Report {
        spec_id: String,
        span: ElementSpan,
        children: Vec<Block>,
    },
    Coverage {
        attrs: Vec<(String, String)>,
        body: String,
        span: ElementSpan,
    },
}

fn assemble(raw: Vec<RawTag>, source: &str, path: &Utf8Path) -> ParseResult<Vec<Block>> {
    let mut stack: Vec<PendingBlock> = Vec::new();
    let mut top: Vec<Block> = Vec::new();

    for t in raw {
        if t.is_close {
            let Some(open) = stack.pop() else {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: t.span.start,
                    reason: format!("close tag `</{}>` without matching open", t.name),
                }));
            };
            if open.name != t.name {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: t.span.start,
                    reason: format!(
                        "close tag `</{}>` does not match open tag `<{}>`",
                        t.name, open.name
                    ),
                }));
            }
            let body = source
                .get(open.body_start..t.body_end_after_tag)
                .unwrap_or("")
                .to_owned();
            let block = open.finish(body, path)?;
            if let Some(parent) = stack.last_mut() {
                parent.children.push(block);
            } else {
                top.push(block);
            }
        } else {
            stack.push(PendingBlock {
                name: t.name,
                attrs: t.attrs,
                span: t.span,
                body_start: t.body_start,
                children: Vec::new(),
            });
        }
    }

    if let Some(open) = stack.first() {
        return Err(Box::new(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: open.span.start,
            reason: format!("open tag `<{}>` is never closed", open.name),
        }));
    }

    Ok(top)
}

fn validate_tag_shape(t: &RawTag, path: &Utf8Path) -> ParseResult<()> {
    if !REPORT_ELEMENT_NAMES.contains(&t.name.as_str()) {
        return Err(Box::new(ParseError::UnknownMarkerName {
            path: path.to_path_buf(),
            marker_name: t.name.clone(),
            offset: t.span.start,
        }));
    }
    if t.is_close {
        return Ok(());
    }
    let allowed_attrs: &[&str] = match t.name.as_str() {
        "report" => &["spec"],
        "coverage" => &["req", "result", "scenarios"],
        _ => &[],
    };
    for (k, _v) in &t.attrs {
        if !allowed_attrs.contains(&k.as_str()) {
            return Err(unknown_attribute_error(
                path,
                &t.name,
                k,
                t.span.start,
                allowed_attrs,
            ));
        }
    }
    if t.name == "report"
        && let Some(value) = find_attr(&t.attrs, "spec")
        && !spec_id_regex().is_match(&value)
    {
        return Err(Box::new(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: "report".to_owned(),
            id: value,
            expected_pattern: r"SPEC-\d{3,}".to_owned(),
        }));
    }
    Ok(())
}

#[derive(Debug)]
struct PendingBlock {
    name: String,
    attrs: Vec<(String, String)>,
    span: ElementSpan,
    body_start: usize,
    children: Vec<Block>,
}

impl PendingBlock {
    fn finish(self, body: String, path: &Utf8Path) -> ParseResult<Block> {
        let PendingBlock {
            name,
            attrs,
            span,
            body_start: _,
            children,
        } = self;
        match name.as_str() {
            "report" => {
                let spec_id = find_attr(&attrs, "spec").ok_or_else(|| {
                    Box::new(ParseError::MissingField {
                        field: "spec".to_owned(),
                        context: format!("<report> element in {path}"),
                    })
                })?;
                Ok(Block::Report {
                    spec_id,
                    span,
                    children,
                })
            }
            "coverage" => Ok(Block::Coverage { attrs, body, span }),
            other => Err(Box::new(ParseError::UnknownMarkerName {
                path: path.to_path_buf(),
                marker_name: other.to_owned(),
                offset: span.start,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ALLOWED_COVERAGE_RESULTS;
    use super::CoverageResult;
    use super::REPORT_ELEMENT_NAMES;
    use super::parse;
    use crate::error::ParseError;
    use crate::parse::xml_scanner::HTML5_ELEMENT_NAMES;
    use camino::Utf8Path;
    use indoc::indoc;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/REPORT.md")
    }

    fn frontmatter() -> &'static str {
        "---\nspec: SPEC-0022\noutcome: delivered\ngenerated_at: 2026-05-17T00:00:00Z\n---\n\n# Report: SPEC-0022\n\n"
    }

    fn make(body: &str) -> String {
        format!("{}{}", frontmatter(), body)
    }

    #[test]
    fn happy_path_three_results() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
            REQ-001 is fully satisfied by CHK-001.
            </coverage>

            <coverage req="REQ-002" result="partial" scenarios="CHK-002 CHK-003">
            REQ-002 is partially satisfied; CHK-002 and CHK-003 cover the
            shipping slice.
            </coverage>

            <coverage req="REQ-003" result="deferred" scenarios="">
            REQ-003 deferred to a future spec.
            </coverage>

            </report>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.spec_id, "SPEC-0022");
        assert_eq!(doc.coverage.len(), 3);
        let c1 = doc.coverage.first().expect("three rows");
        assert_eq!(c1.req, "REQ-001");
        assert_eq!(c1.result, CoverageResult::Satisfied);
        assert_eq!(c1.scenarios, vec!["CHK-001".to_owned()]);
        assert!(c1.body.contains("fully satisfied"));
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
    fn invalid_result_passed_lists_valid_set() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="passed" scenarios="CHK-001">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("bad result must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::InvalidCoverageResult { req, value, allowed, .. }
                    if req == "REQ-001"
                        && value == "passed"
                        && allowed == "satisfied, partial, deferred"
            ),
            "got: {err:?}",
        );
        for result in ALLOWED_COVERAGE_RESULTS {
            assert!(
                msg.contains(result),
                "msg `{msg}` missing valid result `{result}`"
            );
        }
    }

    #[test]
    fn dropped_coverage_result_is_rejected() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="dropped" scenarios="">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("dropped must be rejected");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::InvalidCoverageResult { req, value, .. }
                    if req == "REQ-001" && value == "dropped"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn missing_scenarios_attribute_for_satisfied_errors() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("missing scenarios must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::MissingCoverageAttribute { attribute, .. }
                    if attribute == "scenarios"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn satisfied_with_empty_scenarios_errors() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied" scenarios="">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("satisfied + empty must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::SatisfiedRequiresScenarios { req, .. } if req == "REQ-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn partial_with_empty_scenarios_errors() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="partial" scenarios="">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("partial + empty must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::PartialRequiresScenarios { req, .. } if req == "REQ-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn deferred_with_empty_scenarios_succeeds() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="deferred" scenarios="">
            deferred prose.
            </coverage>

            </report>
        "#});
        let doc = parse(&src, path()).expect("deferred + empty should succeed");
        let c = doc.coverage.first().expect("one row");
        assert_eq!(c.result, CoverageResult::Deferred);
        assert!(c.scenarios.is_empty());
    }

    #[test]
    fn double_space_scenarios_quotes_grammar() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied" scenarios="CHK-001  CHK-002">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("double-space scenarios must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::InvalidScenariosFormat { req, value, .. }
                    if req == "REQ-001" && value == "CHK-001  CHK-002"
            ),
            "got: {err:?}",
        );
        assert!(
            msg.contains("single ASCII space separated `CHK-\\d{3,}` ids"),
            "msg `{msg}` must quote the SPEC-0022 grammar verbatim",
        );
    }

    #[test]
    fn tab_scenarios_quotes_grammar() {
        let raw = "CHK-001\tCHK-002";
        let src = make(&format!(
            "<report spec=\"SPEC-0022\">\n\n<coverage req=\"REQ-001\" result=\"satisfied\" scenarios=\"{raw}\">\nbody.\n</coverage>\n\n</report>\n",
        ));
        let err = parse(&src, path()).expect_err("tab in scenarios must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::InvalidScenariosFormat { req, .. } if req == "REQ-001"
            ),
            "got: {err:?}",
        );
        assert!(
            msg.contains("single ASCII space separated `CHK-\\d{3,}` ids"),
            "msg `{msg}` must quote the SPEC-0022 grammar verbatim",
        );
    }

    #[test]
    fn unknown_attribute_on_coverage_lists_valid_set() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied" scenarios="CHK-001" priority="high">
            body.
            </coverage>

            </report>
        "#});
        let err = parse(&src, path()).expect_err("unknown attr must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::UnknownMarkerAttribute {
                    marker_name, attribute, allowed, ..
                } if marker_name == "coverage"
                    && attribute == "priority"
                    && allowed == "req, result, scenarios"
            ),
            "got: {err:?}",
        );
        assert!(
            msg.contains("req, result, scenarios"),
            "msg `{msg}` missing valid set"
        );
    }

    #[test]
    fn report_element_names_disjoint_from_html5() {
        for name in REPORT_ELEMENT_NAMES {
            assert!(
                !HTML5_ELEMENT_NAMES.contains(name),
                "REPORT element `{name}` collides with HTML5 element name set",
            );
        }
    }
}
