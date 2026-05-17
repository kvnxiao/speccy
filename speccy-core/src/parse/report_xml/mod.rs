//! Raw-XML-element-structured REPORT.md parser and renderer (SPEC-0022
//! REQ-002 / REQ-003 carrier).
//!
//! Reads a REPORT.md whose body is ordinary Markdown plus line-isolated raw
//! XML open/close tag pairs drawn from a small closed whitelist
//! (`report`, `coverage`) and returns a typed [`ReportDoc`]. Reuses the
//! shared scanner ([`crate::parse::xml_scanner`]) introduced by T-001 so
//! fenced-code-block awareness and tag-shape diagnostics are identical to
//! SPEC.md and TASKS.md parsing.
//!
//! See `.speccy/specs/0022-xml-canonical-tasks-report/SPEC.md` REQ-002
//! and REQ-003 for the contract this module satisfies.

use crate::error::ParseError;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
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

/// Closed set of valid `<coverage result="...">` values, in their on-disk
/// form. The legacy `dropped` value is intentionally absent — SPEC-0022
/// requires dropped requirements to be removed from SPEC.md via amendment
/// rather than carried as a coverage row.
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
    /// (e.g. `"SPEC-0022"`).
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
) -> Result<Vec<RawTag>, ParseError> {
    let code_fence_ranges = collect_code_fence_byte_ranges(source);
    let cfg = ScanConfig {
        whitelist: REPORT_ELEMENT_NAMES,
        structure_shaped_names: REPORT_ELEMENT_NAMES,
        retired_names: &[],
        detect_legacy_markers: false,
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
pub fn parse(source: &str, path: &Utf8Path) -> Result<ReportDoc, ParseError> {
    let split = split_frontmatter(source, path)?;
    let (frontmatter_raw, body, body_offset) = match split {
        Split::Some { yaml, body } => {
            let body_offset = source.len().checked_sub(body.len()).ok_or_else(|| {
                ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: 0,
                    reason: "frontmatter splitter produced an inconsistent body offset".to_owned(),
                }
            })?;
            (yaml.to_owned(), body, body_offset)
        }
        Split::None => {
            return Err(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("REPORT.md at {path}"),
            });
        }
    };

    let heading = extract_level1_heading(body, path)?;

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
                    return Err(ParseError::MalformedMarker {
                        path: path.to_path_buf(),
                        offset: span.start,
                        reason: "more than one <report> root element".to_owned(),
                    });
                }
                root = Some((spec_id, span, children));
            }
            Block::Coverage { span, .. } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<coverage> element must be nested inside <report>".to_owned(),
                });
            }
        }
    }

    let (spec_id, report_span, children) = root.ok_or_else(|| ParseError::MissingField {
        field: "<report>".to_owned(),
        context: format!("REPORT.md at {path}"),
    })?;

    let mut coverage: Vec<RequirementCoverage> = Vec::new();
    for child in children {
        match child {
            Block::Coverage {
                attrs,
                body,
                span,
                attrs_present,
            } => {
                let row = build_coverage(&attrs, &attrs_present, body, span, path)?;
                coverage.push(row);
            }
            Block::Report { span, .. } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<report> element must not be nested".to_owned(),
                });
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
    attrs_present: &[String],
    body: String,
    span: ElementSpan,
    path: &Utf8Path,
) -> Result<RequirementCoverage, ParseError> {
    // req
    let req = find_attr(attrs, "req").ok_or_else(|| ParseError::MissingCoverageAttribute {
        path: path.to_path_buf(),
        attribute: "req".to_owned(),
        offset: span.start,
    })?;
    if !req_id_regex().is_match(&req) {
        return Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: "coverage".to_owned(),
            id: req.clone(),
            expected_pattern: r"REQ-\d{3,}".to_owned(),
        });
    }

    // result
    let result_raw =
        find_attr(attrs, "result").ok_or_else(|| ParseError::MissingCoverageAttribute {
            path: path.to_path_buf(),
            attribute: "result".to_owned(),
            offset: span.start,
        })?;
    let result =
        CoverageResult::from_str(&result_raw).ok_or_else(|| ParseError::InvalidCoverageResult {
            path: path.to_path_buf(),
            req: req.clone(),
            value: result_raw.clone(),
            allowed: ALLOWED_COVERAGE_RESULTS.join(", "),
        })?;

    // scenarios — must be *present*, but may be empty for deferred.
    if !attrs_present.iter().any(|k| k == "scenarios") {
        return Err(ParseError::MissingCoverageAttribute {
            path: path.to_path_buf(),
            attribute: "scenarios".to_owned(),
            offset: span.start,
        });
    }
    let scenarios_raw = find_attr(attrs, "scenarios").unwrap_or_default();
    let scenarios = parse_scenarios(&scenarios_raw, &req, path)?;

    match result {
        CoverageResult::Satisfied if scenarios.is_empty() => {
            return Err(ParseError::SatisfiedRequiresScenarios {
                path: path.to_path_buf(),
                req: req.clone(),
            });
        }
        CoverageResult::Partial if scenarios.is_empty() => {
            return Err(ParseError::PartialRequiresScenarios {
                path: path.to_path_buf(),
                req: req.clone(),
            });
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
/// Grammar (SPEC-0022 REQ-002): zero or more `CHK-\d{3,}` ids separated
/// by single ASCII spaces. The attribute being entirely empty is OK; this
/// function returns an empty vec in that case (the caller enforces the
/// per-result minimum). Leading or trailing whitespace, double spaces,
/// tabs, and any non-`CHK-\d{3,}` token all fail with
/// [`ParseError::InvalidScenariosFormat`], whose Display quotes the
/// grammar verbatim.
fn parse_scenarios(raw: &str, req: &str, path: &Utf8Path) -> Result<Vec<String>, ParseError> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    for ch in raw.chars() {
        if ch == '\t' || ch == '\r' || ch == '\n' {
            return Err(ParseError::InvalidScenariosFormat {
                path: path.to_path_buf(),
                req: req.to_owned(),
                value: raw.to_owned(),
            });
        }
    }
    let mut scenarios: Vec<String> = Vec::new();
    for token in raw.split(' ') {
        if !chk_id_regex().is_match(token) {
            return Err(ParseError::InvalidScenariosFormat {
                path: path.to_path_buf(),
                req: req.to_owned(),
                value: raw.to_owned(),
            });
        }
        scenarios.push(token.to_owned());
    }
    Ok(scenarios)
}

/// Render a [`ReportDoc`] to its canonical raw-XML REPORT.md form.
///
/// The output is a Markdown document with raw XML element tags carrying
/// Speccy structure:
///
/// 1. Frontmatter fence followed by [`ReportDoc::frontmatter_raw`].
/// 2. A blank line, then the level-1 heading (`# {heading}`).
/// 3. The root `<report spec="...">` block wrapping every coverage row in
///    [`ReportDoc::coverage`] order.
///
/// `render(doc) == render(doc)` byte-for-byte for any valid `doc`.
/// Free Markdown prose between `<coverage>` blocks is **not** preserved:
/// the renderer projects only the typed model, mirroring SPEC-0020's
/// canonical-not-lossless contract.
#[must_use = "the rendered Markdown string is the canonical projection of the ReportDoc"]
pub fn render(doc: &ReportDoc) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&doc.frontmatter_raw);
    if !doc.frontmatter_raw.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("---\n\n");
    out.push_str("# ");
    out.push_str(&doc.heading);
    out.push_str("\n\n");

    push_element_open(&mut out, "report", &[("spec", doc.spec_id.as_str())]);
    out.push('\n');
    for row in &doc.coverage {
        let scenarios_value = row.scenarios.join(" ");
        let attrs: [(&str, &str); 3] = [
            ("req", row.req.as_str()),
            ("result", row.result.as_str()),
            ("scenarios", scenarios_value.as_str()),
        ];
        push_element_block(&mut out, "coverage", &attrs, &row.body);
    }
    push_element_close(&mut out, "report");

    out
}

fn push_element_block(out: &mut String, name: &str, attrs: &[(&str, &str)], body: &str) {
    push_element_open(out, name, attrs);
    push_body(out, body);
    push_element_close(out, name);
}

fn push_element_open(out: &mut String, name: &str, attrs: &[(&str, &str)]) {
    out.push('<');
    out.push_str(name);
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(v);
        out.push('"');
    }
    out.push_str(">\n");
}

fn push_element_close(out: &mut String, name: &str) {
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
    // Match `spec_xml`/`task_xml` determinism contract: every close tag is
    // followed by a single blank line.
    out.push('\n');
}

fn push_body(out: &mut String, body: &str) {
    let interior = trim_blank_boundary_lines(body);
    if interior.is_empty() {
        return;
    }
    out.push_str(interior);
    out.push('\n');
}

fn trim_blank_boundary_lines(body: &str) -> &str {
    let bytes = body.as_bytes();
    let mut start: usize = 0;
    let mut cursor: usize = 0;
    while cursor < bytes.len() {
        let line_start = cursor;
        let mut all_ws = true;
        while cursor < bytes.len() && bytes.get(cursor) != Some(&b'\n') {
            match bytes.get(cursor) {
                Some(b' ' | b'\t' | b'\r') => {}
                _ => all_ws = false,
            }
            cursor = cursor.saturating_add(1);
        }
        if cursor < bytes.len() {
            cursor = cursor.saturating_add(1);
        }
        if all_ws {
            start = cursor;
        } else {
            start = line_start;
            break;
        }
    }
    if start >= bytes.len() {
        return "";
    }

    let mut end: usize = bytes.len();
    let mut cursor: usize = bytes.len();
    while cursor > start {
        let mut line_end = cursor;
        let mut probe = cursor;
        if probe > start && bytes.get(probe.saturating_sub(1)) == Some(&b'\n') {
            probe = probe.saturating_sub(1);
            line_end = probe;
        }
        let mut line_start = probe;
        while line_start > start && bytes.get(line_start.saturating_sub(1)) != Some(&b'\n') {
            line_start = line_start.saturating_sub(1);
        }
        let line = bytes.get(line_start..line_end).unwrap_or(&[]);
        let all_ws = line.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r'));
        if all_ws {
            end = line_start;
            cursor = line_start;
        } else {
            end = line_end;
            break;
        }
    }
    body.get(start..end).unwrap_or("")
}

fn extract_level1_heading(body: &str, path: &Utf8Path) -> Result<String, ParseError> {
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return Ok(rest.trim().to_owned());
        }
        if trimmed == "#" {
            return Ok(String::new());
        }
    }
    Err(ParseError::MissingField {
        field: "level-1 heading".to_owned(),
        context: format!("REPORT.md at {path}"),
    })
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
        attrs_present: Vec<String>,
        body: String,
        span: ElementSpan,
    },
}

fn assemble(raw: Vec<RawTag>, source: &str, path: &Utf8Path) -> Result<Vec<Block>, ParseError> {
    let mut stack: Vec<PendingBlock> = Vec::new();
    let mut top: Vec<Block> = Vec::new();

    for t in raw {
        if t.is_close {
            let Some(open) = stack.pop() else {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: t.span.start,
                    reason: format!("close tag `</{}>` without matching open", t.name),
                });
            };
            if open.name != t.name {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: t.span.start,
                    reason: format!(
                        "close tag `</{}>` does not match open tag `<{}>`",
                        t.name, open.name
                    ),
                });
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
            let attrs_present: Vec<String> = t.attrs.iter().map(|(k, _)| k.clone()).collect();
            stack.push(PendingBlock {
                name: t.name,
                attrs: t.attrs,
                attrs_present,
                span: t.span,
                body_start: t.body_start,
                children: Vec::new(),
            });
        }
    }

    if let Some(open) = stack.first() {
        return Err(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: open.span.start,
            reason: format!("open tag `<{}>` is never closed", open.name),
        });
    }

    Ok(top)
}

fn validate_tag_shape(t: &RawTag, path: &Utf8Path) -> Result<(), ParseError> {
    if !REPORT_ELEMENT_NAMES.contains(&t.name.as_str()) {
        return Err(ParseError::UnknownMarkerName {
            path: path.to_path_buf(),
            marker_name: t.name.clone(),
            offset: t.span.start,
        });
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
        return Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: "report".to_owned(),
            id: value,
            expected_pattern: r"SPEC-\d{3,}".to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug)]
struct PendingBlock {
    name: String,
    attrs: Vec<(String, String)>,
    attrs_present: Vec<String>,
    span: ElementSpan,
    body_start: usize,
    children: Vec<Block>,
}

impl PendingBlock {
    fn finish(self, body: String, path: &Utf8Path) -> Result<Block, ParseError> {
        let PendingBlock {
            name,
            attrs,
            attrs_present,
            span,
            body_start: _,
            children,
        } = self;
        match name.as_str() {
            "report" => {
                let spec_id =
                    find_attr(&attrs, "spec").ok_or_else(|| ParseError::MissingField {
                        field: "spec".to_owned(),
                        context: format!("<report> element in {path}"),
                    })?;
                Ok(Block::Report {
                    spec_id,
                    span,
                    children,
                })
            }
            "coverage" => Ok(Block::Coverage {
                attrs,
                attrs_present,
                body,
                span,
            }),
            other => Err(ParseError::UnknownMarkerName {
                path: path.to_path_buf(),
                marker_name: other.to_owned(),
                offset: span.start,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ALLOWED_COVERAGE_RESULTS;
    use super::CoverageResult;
    use super::REPORT_ELEMENT_NAMES;
    use super::parse;
    use super::render;
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
                &err,
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
    fn legacy_dropped_result_is_rejected() {
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
                &err,
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
                &err,
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
                &err,
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
                &err,
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
                &err,
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
                &err,
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
                &err,
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
    fn render_then_reparse_field_equal() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
            sat body.
            </coverage>

            <coverage req="REQ-002" result="partial" scenarios="CHK-002 CHK-003">
            partial body.
            </coverage>

            <coverage req="REQ-003" result="deferred" scenarios="">
            deferred body.
            </coverage>

            </report>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let rendered = render(&doc);
        let doc2 = parse(&rendered, path()).expect("rendered REPORT.md must reparse");
        assert_eq!(doc.spec_id, doc2.spec_id);
        assert_eq!(doc.coverage.len(), doc2.coverage.len());
        for (a, b) in doc.coverage.iter().zip(doc2.coverage.iter()) {
            assert_eq!(a.req, b.req);
            assert_eq!(a.result, b.result);
            assert_eq!(a.scenarios, b.scenarios);
            assert_eq!(a.body.trim(), b.body.trim());
        }
    }

    #[test]
    fn render_is_idempotent() {
        let src = make(indoc! {r#"
            <report spec="SPEC-0022">

            <coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
            sat body.
            </coverage>

            </report>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let first = render(&doc);
        let second = render(&doc);
        assert_eq!(first, second, "render must be byte-identical on repeat");
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
