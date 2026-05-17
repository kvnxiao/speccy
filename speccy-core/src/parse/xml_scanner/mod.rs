//! Shared line-aware raw-XML element scanner.
//!
//! Factored out of [`crate::parse::spec_xml`] so SPEC.md, TASKS.md, and
//! REPORT.md parsers can all reuse the same line-isolation, fenced-code-
//! block awareness, and tag-shape diagnostics. The scanner emits
//! [`RawTag`]s for every recognised element open or close tag whose name
//! is in the caller-supplied whitelist. Tag-shaped lines whose names are
//! **not** in the whitelist are silently treated as Markdown body content
//! (matching the SPEC-0020 contract: foreign HTML like `<details>` flows
//! through verbatim).
//!
//! Higher-level concerns — block assembly, attribute validation, typed
//! model construction — stay in the per-artifact parsers. The scanner
//! only knows three things:
//!
//! 1. Which lines isolate a recognised open or close tag.
//! 2. Which byte ranges lie inside a fenced code block (and therefore do not
//!    carry structure).
//! 3. How to surface a structured diagnostic for malformed-but-
//!    structure-shaped tag lines.
//!
//! See SPEC-0022 REQ-003 and SPEC-0020 DEC-002 / DEC-003 for the
//! contract this module satisfies.

mod html5_names;

use crate::error::ParseError;
use crate::parse::markdown::parse_markdown;
use camino::Utf8Path;
use comrak::Arena;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
pub use html5_names::HTML5_ELEMENT_NAMES;
pub use html5_names::is_html5_element_name;
use regex::Regex;
use std::sync::OnceLock;

/// Byte range covering an element's *open* tag in the source string.
///
/// `&source[start..end]` always begins with `<` followed by the
/// recognised element name so diagnostics can re-point at the offending
/// tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementSpan {
    /// Inclusive byte offset of the leading `<` of the open tag.
    pub start: usize,
    /// Exclusive byte offset just past the trailing `>` of the open tag.
    pub end: usize,
}

/// One scanned raw open or close tag.
///
/// Bodies are not extracted here; callers walk the returned tags
/// pairwise (the parser assembles `Block`s and slices source between
/// `body_start` of an open tag and `body_end_after_tag` of its matching
/// close tag).
#[derive(Debug, Clone)]
pub struct RawTag {
    /// Element name as it appeared in the source (lowercase by regex).
    pub name: String,
    /// `true` when the tag was `</name>`, `false` for `<name ...>`.
    pub is_close: bool,
    /// Attribute (name, value) pairs in source order. Empty for close
    /// tags.
    pub attrs: Vec<(String, String)>,
    /// Span covering this tag in the source.
    pub span: ElementSpan,
    /// Absolute byte offset where the body content begins (immediately
    /// after the newline that terminates the open-tag line). For close
    /// tags this is the offset just past the tag itself — matching the
    /// open tag's `body_start` semantics so callers can compare offsets
    /// uniformly.
    pub body_start: usize,
    /// Absolute byte offset of this tag's start, used to bound the body
    /// of the matching open tag when this is a close tag.
    pub body_end_after_tag: usize,
}

/// Configuration for one scan pass.
///
/// Callers supply the whitelist of element names they recognise as
/// structure. Tag-shaped lines whose names are outside the whitelist
/// pass through as Markdown body content unless they also appear in
/// `retired_names` (which surface a [`ParseError::RetiredMarkerName`])
/// or `structure_shaped_names` (which surface
/// [`ParseError::MalformedMarker`] for malformed tag shapes).
///
/// The default for `structure_shaped_names` is the whitelist itself; if
/// a caller wants to flag malformed-but-retired names too they should
/// pass the union.
#[derive(Debug, Clone, Copy)]
pub struct ScanConfig<'a> {
    /// Element names the caller wants extracted as [`RawTag`]s.
    pub whitelist: &'a [&'a str],
    /// Element names whose malformed tag shapes should produce
    /// [`ParseError::MalformedMarker`] diagnostics rather than be
    /// silently treated as Markdown. Typically the union of
    /// `whitelist` and `retired_names`.
    pub structure_shaped_names: &'a [&'a str],
    /// Element names that produce
    /// [`ParseError::RetiredMarkerName`] diagnostics.
    pub retired_names: &'a [&'a str],
    /// When `true`, surface SPEC-0019 HTML-comment
    /// `<!-- speccy:... -->` markers outside fenced code blocks as
    /// [`ParseError::LegacyMarker`] diagnostics.
    pub detect_legacy_markers: bool,
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn open_tag_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        // `<NAME>` or `<NAME attr="value" ...>`. Attribute values must be
        // double-quoted; unquoted values are a parse error and fall
        // through the shape regex below for a clearer diagnostic.
        Regex::new(r#"^<([a-z][a-z-]*)((?:\s+[A-Za-z_][\w-]*="[^"]*")*)\s*>$"#).unwrap()
    })
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn close_tag_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^</([a-z][a-z-]*)\s*>$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn shape_open_regex() -> &'static Regex {
    // Detects a line that *looks* like an open tag (`<name...>`) so we
    // can produce structured diagnostics for malformed cases (unquoted
    // attribute values, junk after the closing `>`, etc.) instead of
    // silently treating them as Markdown body.
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^<([a-z][a-z-]*)(\s|>)").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn shape_close_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^</([a-z][a-z-]*)").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn attribute_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r#"\s+([A-Za-z_][\w-]*)="([^"]*)""#).unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn legacy_marker_regex() -> &'static Regex {
    // Matches the SPEC-0019 HTML-comment marker form when it is the only
    // non-whitespace content on a line. Capture 1: optional leading
    // slash (close marker). Capture 2: element name.
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        Regex::new(r"(?m)^\s*<!--\s*(/?)speccy:([a-z][a-z-]*)(?:\s[^>]*)?-->\s*$").unwrap()
    })
}

/// Build an [`ParseError::UnknownMarkerAttribute`] diagnostic carrying
/// the element name, attribute name, byte offset of the open tag, and
/// the comma-separated set of valid attribute names. SPEC-0022 REQ-003
/// requires every reusing parser to surface the valid set, so this
/// helper centralises the formatting.
#[must_use = "the constructed ParseError must be returned to the caller"]
pub fn unknown_attribute_error(
    path: &Utf8Path,
    element_name: &str,
    attribute: &str,
    offset: usize,
    allowed: &[&str],
) -> ParseError {
    ParseError::UnknownMarkerAttribute {
        path: path.to_path_buf(),
        marker_name: element_name.to_owned(),
        attribute: attribute.to_owned(),
        offset,
        allowed: allowed.join(", "),
    }
}

/// Scan `body` (the post-frontmatter portion of `source`, beginning at
/// `body_offset` in `source`) for line-isolated open and close tags
/// belonging to `cfg.whitelist`.
///
/// `code_fence_ranges` is the output of [`collect_code_fence_byte_ranges`]
/// over `source`; lines fully inside any range are skipped.
///
/// # Errors
///
/// - [`ParseError::LegacyMarker`] when a SPEC-0019 HTML-comment marker appears
///   outside a fenced code block (only when `cfg.detect_legacy_markers` is
///   `true`).
/// - [`ParseError::RetiredMarkerName`] when a tag whose name is in
///   `cfg.retired_names` appears.
/// - [`ParseError::MalformedMarker`] when a structure-shaped line is malformed
///   (unquoted attribute, missing `>`, content before/after the tag) and the
///   name is in `cfg.structure_shaped_names`.
/// - [`ParseError::MalformedMarker`] for byte-arithmetic overflow (defensive;
///   cannot fire for documents that fit in memory).
pub fn scan_tags(
    source: &str,
    body: &str,
    body_offset: usize,
    code_fence_ranges: &[(usize, usize)],
    path: &Utf8Path,
    cfg: &ScanConfig<'_>,
) -> Result<Vec<RawTag>, ParseError> {
    let mut tags: Vec<RawTag> = Vec::new();
    let mut line_start_in_body: usize = 0;

    while line_start_in_body <= body.len() {
        let Some(line_info) = next_line(body, body_offset, line_start_in_body, path)? else {
            break;
        };

        if !range_inside_any_fence(
            line_info.abs_line_start,
            line_info.abs_line_end_excl,
            code_fence_ranges,
        ) {
            classify_line(source, body, body_offset, &line_info, &mut tags, path, cfg)?;
        }

        line_start_in_body = line_info.next_start_in_body;
    }

    Ok(tags)
}

#[derive(Debug, Clone, Copy)]
struct LineInfo<'a> {
    line: &'a str,
    abs_line_start: usize,
    abs_line_end_excl: usize,
    next_start_in_body: usize,
}

fn next_line<'a>(
    body: &'a str,
    body_offset: usize,
    line_start_in_body: usize,
    path: &Utf8Path,
) -> Result<Option<LineInfo<'a>>, ParseError> {
    let remainder = body.get(line_start_in_body..).unwrap_or("");
    let (line, next_start_in_body) = if let Some(nl) = remainder.find('\n') {
        let line_end = line_start_in_body
            .checked_add(nl)
            .ok_or_else(|| overflow_error(path))?;
        let next = line_end
            .checked_add(1)
            .ok_or_else(|| overflow_error(path))?;
        (body.get(line_start_in_body..line_end).unwrap_or(""), next)
    } else if remainder.is_empty() {
        return Ok(None);
    } else {
        (remainder, body.len().saturating_add(1))
    };

    let abs_line_start = body_offset
        .checked_add(line_start_in_body)
        .ok_or_else(|| overflow_error(path))?;
    let abs_line_end_excl = abs_line_start
        .checked_add(line.len())
        .ok_or_else(|| overflow_error(path))?;

    Ok(Some(LineInfo {
        line,
        abs_line_start,
        abs_line_end_excl,
        next_start_in_body,
    }))
}

fn overflow_error(path: &Utf8Path) -> ParseError {
    ParseError::MalformedMarker {
        path: path.to_path_buf(),
        offset: 0,
        reason: "byte arithmetic overflow during line scan".to_owned(),
    }
}

fn classify_line(
    source: &str,
    body: &str,
    body_offset: usize,
    line_info: &LineInfo<'_>,
    tags: &mut Vec<RawTag>,
    path: &Utf8Path,
    cfg: &ScanConfig<'_>,
) -> Result<(), ParseError> {
    let line = line_info.line;
    let abs_line_start = line_info.abs_line_start;
    let abs_line_end_excl = line_info.abs_line_end_excl;
    let next_start_in_body = line_info.next_start_in_body;

    if cfg.detect_legacy_markers
        && let Some(legacy) = detect_legacy_marker(line, abs_line_start, path)
    {
        return Err(legacy);
    }

    let trimmed = line.trim_start();
    let leading_ws = line.len().saturating_sub(trimmed.len());
    let abs_tag_offset = abs_line_start
        .checked_add(leading_ws)
        .unwrap_or(abs_line_start);
    let line_for_regex = trimmed.trim_end();

    if let Some(caps) = open_tag_regex().captures(line_for_regex) {
        let name = caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        if cfg.retired_names.contains(&name.as_str()) {
            return Err(ParseError::RetiredMarkerName {
                path: path.to_path_buf(),
                marker_name: name,
                offset: abs_tag_offset,
            });
        }
        if !cfg.whitelist.contains(&name.as_str()) {
            return Ok(());
        }
        let attr_blob = caps.get(2).map_or("", |m| m.as_str());
        let body_start = body_offset
            .checked_add(next_start_in_body.min(body.len()))
            .unwrap_or(source.len());
        tags.push(build_open_tag(
            name,
            attr_blob,
            abs_tag_offset,
            abs_line_end_excl,
            body_start,
        ));
        Ok(())
    } else if let Some(caps) = close_tag_regex().captures(line_for_regex) {
        let name = caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        if cfg.retired_names.contains(&name.as_str()) {
            return Err(ParseError::RetiredMarkerName {
                path: path.to_path_buf(),
                marker_name: name,
                offset: abs_tag_offset,
            });
        }
        if !cfg.whitelist.contains(&name.as_str()) {
            return Ok(());
        }
        tags.push(RawTag {
            name,
            is_close: true,
            attrs: Vec::new(),
            span: ElementSpan {
                start: abs_tag_offset,
                end: abs_line_end_excl,
            },
            body_start: abs_line_end_excl,
            body_end_after_tag: abs_tag_offset,
        });
        Ok(())
    } else {
        detect_malformed_tag(
            line,
            trimmed,
            abs_tag_offset,
            path,
            cfg.structure_shaped_names,
        )
    }
}

fn build_open_tag(
    name: String,
    attr_blob: &str,
    abs_tag_offset: usize,
    abs_line_end_excl: usize,
    body_start: usize,
) -> RawTag {
    let mut attrs: Vec<(String, String)> = Vec::new();
    for ac in attribute_regex().captures_iter(attr_blob) {
        let k = ac.get(1).map(|m| m.as_str().to_owned()).unwrap_or_default();
        let v = ac.get(2).map(|m| m.as_str().to_owned()).unwrap_or_default();
        attrs.push((k, v));
    }
    RawTag {
        name,
        is_close: false,
        attrs,
        span: ElementSpan {
            start: abs_tag_offset,
            end: abs_line_end_excl,
        },
        body_start,
        body_end_after_tag: abs_tag_offset,
    }
}

fn detect_legacy_marker(line: &str, abs_line_start: usize, path: &Utf8Path) -> Option<ParseError> {
    let caps = legacy_marker_regex().captures(line)?;
    let raw_match = caps.get(0).map_or("", |m| m.as_str()).trim();
    let leading_ws = line.len().saturating_sub(line.trim_start().len());
    let abs_offset = abs_line_start
        .checked_add(leading_ws)
        .unwrap_or(abs_line_start);
    let slash = caps.get(1).map_or("", |m| m.as_str());
    let name = caps.get(2).map_or("", |m| m.as_str());
    let suggested = if slash == "/" {
        format!("</{name}>")
    } else {
        format!("<{name} ...>")
    };
    Some(ParseError::LegacyMarker {
        path: path.to_path_buf(),
        offset: abs_offset,
        legacy_form: raw_match.to_owned(),
        suggested_element: suggested,
    })
}

fn detect_malformed_tag(
    line: &str,
    trimmed: &str,
    abs_tag_offset: usize,
    path: &Utf8Path,
    structure_shaped_names: &[&str],
) -> Result<(), ParseError> {
    if let Some(shape_caps) = shape_open_regex().captures(trimmed) {
        let name = shape_caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        if structure_shaped_names.contains(&name.as_str()) {
            return Err(ParseError::MalformedMarker {
                path: path.to_path_buf(),
                offset: abs_tag_offset,
                reason: diagnose_malformed_open(line, trimmed, &name),
            });
        }
    } else if let Some(shape_caps) = shape_close_regex().captures(trimmed) {
        let name = shape_caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        if structure_shaped_names.contains(&name.as_str()) {
            let reason = if line.trim() != line.trim_end() || line.trim_start() != trimmed {
                "speccy XML close tags must appear on their own line".to_owned()
            } else if !trimmed.trim_end().ends_with('>') {
                "speccy XML close tag is missing the closing `>`".to_owned()
            } else {
                "speccy XML close tag is malformed".to_owned()
            };
            return Err(ParseError::MalformedMarker {
                path: path.to_path_buf(),
                offset: abs_tag_offset,
                reason,
            });
        }
    }
    Ok(())
}

/// Build a human-readable reason for a malformed open tag line.
fn diagnose_malformed_open(line: &str, trimmed: &str, _name: &str) -> String {
    let stripped = trimmed.trim_end();
    if line.trim_start() != trimmed || line.trim_end() != line {
        return "speccy XML element tags must appear on their own line".to_owned();
    }
    if !stripped.ends_with('>') {
        return "speccy XML open tag is missing the closing `>`".to_owned();
    }
    "attribute values must be double-quoted".to_owned()
}

fn range_inside_any_fence(
    line_start: usize,
    line_end_excl: usize,
    fences: &[(usize, usize)],
) -> bool {
    for (s, e) in fences {
        if line_start >= *s && line_end_excl <= *e {
            return true;
        }
    }
    false
}

/// Collect byte ranges (start..end exclusive) covering every fenced
/// code block in `source`. Lines whose entire span falls inside one of
/// these ranges are not scanned for structure tags.
#[must_use = "the returned ranges drive fence-awareness in scan_tags"]
pub fn collect_code_fence_byte_ranges(source: &str) -> Vec<(usize, usize)> {
    let arena = Arena::new();
    let root = parse_markdown(&arena, source);

    let mut ranges: Vec<(usize, usize)> = Vec::new();
    walk_for_code_fences(root, source, &mut ranges);
    ranges
}

fn walk_for_code_fences<'a>(root: &'a AstNode<'a>, source: &str, out: &mut Vec<(usize, usize)>) {
    for node in root.descendants() {
        let ast = node.data.borrow();
        if let NodeValue::CodeBlock(_) = &ast.value {
            let start_line = ast.sourcepos.start.line;
            let end_line = ast.sourcepos.end.line;
            if let Some((s, e)) = line_range_to_byte_range(source, start_line, end_line) {
                out.push((s, e));
            }
        }
    }
}

fn line_range_to_byte_range(
    source: &str,
    start_line_1: usize,
    end_line_1: usize,
) -> Option<(usize, usize)> {
    if start_line_1 == 0 || end_line_1 < start_line_1 {
        return None;
    }
    let mut line_no: usize = 1;
    let mut start_byte: Option<usize> = None;
    let mut end_byte: Option<usize> = None;
    let mut current_line_start: usize = 0;

    for (idx, ch) in source.char_indices() {
        if line_no == start_line_1 && start_byte.is_none() {
            start_byte = Some(current_line_start);
        }
        if ch == '\n' {
            if line_no == end_line_1 {
                end_byte = Some(idx.checked_add(1)?);
                break;
            }
            line_no = line_no.checked_add(1)?;
            current_line_start = idx.checked_add(1)?;
        }
    }
    if line_no == start_line_1 && start_byte.is_none() {
        start_byte = Some(current_line_start);
    }
    if end_byte.is_none() {
        end_byte = Some(source.len());
    }
    Some((start_byte?, end_byte?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8Path;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/test.md")
    }

    fn scan(source: &str, cfg: &ScanConfig<'_>) -> Result<Vec<RawTag>, ParseError> {
        let fences = collect_code_fence_byte_ranges(source);
        scan_tags(source, source, 0, &fences, path(), cfg)
    }

    fn cfg<'a>(whitelist: &'a [&'a str]) -> ScanConfig<'a> {
        ScanConfig {
            whitelist,
            structure_shaped_names: whitelist,
            retired_names: &[],
            detect_legacy_markers: false,
        }
    }

    #[test]
    fn returns_open_close_spans_for_whitelisted_tags() {
        let src = "before\n<task id=\"T-1\">\nbody bytes\n</task>\nafter\n";
        let tags = scan(src, &cfg(&["task"])).expect("scan should succeed");
        assert_eq!(tags.len(), 2);
        let open = tags.first().expect("open tag");
        let close = tags.get(1).expect("close tag");
        assert_eq!(open.name, "task");
        assert!(!open.is_close);
        assert_eq!(open.attrs, vec![("id".to_owned(), "T-1".to_owned())]);
        assert!(close.is_close);
        // Body bytes lie between open.body_start and close.body_end_after_tag.
        let body = src
            .get(open.body_start..close.body_end_after_tag)
            .expect("body slice");
        assert_eq!(body, "body bytes\n");
    }

    #[test]
    fn tags_outside_whitelist_are_markdown() {
        // `requirement` is not in the caller's whitelist for this test —
        // we are pretending to be a TASKS-only scan that does not know
        // about requirement.
        let src = "<requirement id=\"REQ-1\">\nstuff\n</requirement>\n";
        let tags = scan(src, &cfg(&["task"])).expect("scan should succeed");
        assert!(tags.is_empty(), "non-whitelisted tags must not appear");
    }

    #[test]
    fn tags_inside_fenced_code_block_are_markdown() {
        let src = "```\n<task id=\"T-1\">\nbody\n</task>\n```\n";
        let tags = scan(src, &cfg(&["task"])).expect("scan should succeed");
        assert!(
            tags.is_empty(),
            "fenced-code tag lines must not surface as structure: got {tags:?}",
        );
    }

    #[test]
    fn tags_inside_tilde_fence_are_markdown() {
        let src = "~~~\n<task id=\"T-1\">\nbody\n</task>\n~~~\n";
        let tags = scan(src, &cfg(&["task"])).expect("scan should succeed");
        assert!(
            tags.is_empty(),
            "tilde-fenced tag lines must not surface: got {tags:?}"
        );
    }

    #[test]
    fn unknown_attribute_error_carries_allowed_set() {
        let err =
            unknown_attribute_error(path(), "task", "priority", 42, &["id", "state", "covers"]);
        let msg = format!("{err}");
        assert!(msg.contains("priority"), "msg missing attr name: {msg}");
        assert!(msg.contains("task"), "msg missing element name: {msg}");
        assert!(msg.contains("42"), "msg missing offset: {msg}");
        assert!(
            msg.contains("id, state, covers"),
            "msg missing allowed set: {msg}"
        );
        assert!(
            matches!(
                &err,
                ParseError::UnknownMarkerAttribute {
                    marker_name,
                    attribute,
                    offset,
                    allowed,
                    ..
                } if marker_name == "task"
                    && attribute == "priority"
                    && *offset == 42
                    && allowed == "id, state, covers"
            ),
            "got: {err:?}",
        );
    }

    /// SPEC-0022 REQ-003: the combined whitelist used by SPEC, TASKS,
    /// and REPORT callers must remain disjoint from the HTML5 element
    /// set. This pins the names introduced by SPEC-0022
    /// (`tasks`, `task`, `task-scenarios`, `report`, `coverage`) so
    /// future edits cannot quietly collide.
    #[test]
    fn combined_whitelist_is_disjoint_from_html5_element_set() {
        let combined: &[&str] = &[
            // SPEC.md (SPEC-0020 + SPEC-0021):
            "requirement",
            "scenario",
            "decision",
            "open-question",
            "changelog",
            "behavior",
            "done-when",
            "goals",
            "non-goals",
            "user-stories",
            "assumptions",
            // TASKS.md (SPEC-0022 REQ-001):
            "tasks",
            "task",
            "task-scenarios",
            // REPORT.md (SPEC-0022 REQ-002):
            "report",
            "coverage",
        ];
        for name in combined {
            assert!(
                !is_html5_element_name(name),
                "Speccy element `{name}` collides with HTML5 element name set",
            );
        }
        for new_name in ["tasks", "task", "task-scenarios", "report", "coverage"] {
            assert!(
                !HTML5_ELEMENT_NAMES.contains(&new_name),
                "SPEC-0022 element `{new_name}` is in HTML5_ELEMENT_NAMES",
            );
        }
    }
}
