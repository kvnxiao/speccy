//! Marker-structured SPEC.md parser.
//!
//! Reads a SPEC.md whose body is ordinary Markdown plus line-isolated
//! `<!-- speccy:<name> attr="value" ... -->` marker comments and returns a
//! typed [`SpecDoc`]. The marker comments are the machine-readable
//! structure; everything between them is Markdown preserved verbatim. See
//! `.speccy/specs/0019-xml-canonical-spec-md/SPEC.md` REQ-001 and REQ-003.
//!
//! The deterministic renderer ([`render`]) emits a canonical projection of
//! the typed model: frontmatter + level-1 heading + marker blocks in
//! struct order. It does **not** reproduce free-prose Markdown sections
//! that lived outside marker blocks in the source (Goals, Non-goals,
//! Design narrative, etc.). The renderer's job is to produce a
//! deterministic carrier of the parsed model — useful for migrations,
//! prompt slicing, and roundtrip tests — not to losslessly mirror a
//! human-authored SPEC.md. That tradeoff is documented on [`render`].

use crate::error::ParseError;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
use crate::parse::markdown::parse_markdown;
use camino::Utf8Path;
use comrak::Arena;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Parsed marker-structured SPEC.md.
///
/// `frontmatter_raw` carries the YAML frontmatter payload verbatim. The
/// frontmatter is *not* re-validated here — T-005 wires this parser into
/// the workspace loader and reuses the existing `SpecFrontmatter`
/// deserialisation. T-001 only validates the marker tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecDoc {
    /// YAML frontmatter payload between the opening and closing `---`
    /// fences, verbatim.
    pub frontmatter_raw: String,
    /// Text of the level-1 heading after the `# ` prefix, trimmed.
    pub heading: String,
    /// Raw source bytes, retained so [`MarkerSpan`] indices remain valid.
    pub raw: String,
    /// Requirements declared by `speccy:requirement` markers in source
    /// order.
    pub requirements: Vec<Requirement>,
    /// Decisions declared by `speccy:decision` markers in source order.
    pub decisions: Vec<Decision>,
    /// Open questions declared by `speccy:open-question` markers in
    /// source order.
    pub open_questions: Vec<OpenQuestion>,
    /// Body of the single required `speccy:changelog` marker (verbatim).
    pub changelog_body: String,
    /// Span of the `speccy:changelog` start marker.
    pub changelog_span: MarkerSpan,
    /// Body of the optional `speccy:summary` marker, when present.
    pub summary: Option<String>,
    /// Span of the optional `speccy:summary` start marker.
    pub summary_span: Option<MarkerSpan>,
}

/// One requirement block (`speccy:requirement`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// Id from the `id="..."` attribute (matches `REQ-\d{3,}`).
    pub id: String,
    /// Markdown body between start and end markers, scenarios excluded
    /// (i.e. the marker text of nested scenarios is replaced by a
    /// placeholder-free join of the surrounding text — see
    /// [`Self::body`] semantics below).
    pub body: String,
    /// Nested scenarios in source order.
    pub scenarios: Vec<Scenario>,
    /// Span of the start marker.
    pub span: MarkerSpan,
}

/// One scenario block (`speccy:scenario`), nested inside a requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    /// Id from the `id="..."` attribute (matches `CHK-\d{3,}`).
    pub id: String,
    /// Markdown body between start and end markers, verbatim.
    pub body: String,
    /// Id of the containing `speccy:requirement` marker.
    pub parent_requirement_id: String,
    /// Span of the start marker.
    pub span: MarkerSpan,
}

/// One decision block (`speccy:decision`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Id from the `id="..."` attribute (matches `DEC-\d{3,}`).
    pub id: String,
    /// Optional decision status (`accepted|rejected|deferred|superseded`).
    pub status: Option<DecisionStatus>,
    /// Markdown body between start and end markers, verbatim.
    pub body: String,
    /// Span of the start marker.
    pub span: MarkerSpan,
}

/// Closed set of decision statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionStatus {
    /// `accepted`
    Accepted,
    /// `rejected`
    Rejected,
    /// `deferred`
    Deferred,
    /// `superseded`
    Superseded,
}

impl DecisionStatus {
    /// Render back to the on-disk string form.
    #[must_use = "the rendered status is the on-disk form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            DecisionStatus::Accepted => "accepted",
            DecisionStatus::Rejected => "rejected",
            DecisionStatus::Deferred => "deferred",
            DecisionStatus::Superseded => "superseded",
        }
    }
}

/// One open-question marker (`speccy:open-question`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenQuestion {
    /// Optional `resolved="true|false"` attribute value.
    pub resolved: Option<bool>,
    /// Markdown body between start and end markers, verbatim.
    pub body: String,
    /// Span of the start marker.
    pub span: MarkerSpan,
}

/// Byte range covering a marker's *start* tag in the source string.
///
/// `&source[start..end]` always begins with `<!-- speccy:` so diagnostics
/// can point directly at the offending marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkerSpan {
    /// Inclusive byte offset of the leading `<` of the start marker.
    pub start: usize,
    /// Exclusive byte offset just past the trailing `>` of the start
    /// marker.
    pub end: usize,
}

const ALLOWED_DECISION_STATUSES: &[&str] = &["accepted", "rejected", "deferred", "superseded"];
const ALLOWED_RESOLVED_VALUES: &[&str] = &["true", "false"];

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn marker_line_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        Regex::new(r#"^<!--\s+(/?)speccy:([a-z][a-z-]*)((?:\s+[A-Za-z_][\w-]*="[^"]*")*)\s*-->$"#)
            .unwrap()
    })
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn marker_shape_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"<!--\s*/?speccy:").unwrap())
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

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn dec_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^DEC-\d{3,}$").unwrap())
}

/// Parse a marker-structured SPEC.md source string.
///
/// `source` is the file contents; `path` is used only to populate
/// diagnostics — this function does no filesystem IO.
///
/// # Errors
///
/// Returns [`ParseError`] for missing frontmatter or level-1 heading,
/// marker-shape problems, unknown marker names or attributes, id-pattern
/// violations, duplicate ids, orphan scenario markers, empty required
/// bodies, or invalid attribute values.
pub fn parse(source: &str, path: &Utf8Path) -> Result<SpecDoc, ParseError> {
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
                context: format!("SPEC.md at {path}"),
            });
        }
    };

    let heading = extract_level1_heading(body, path)?;

    let code_fence_ranges = collect_code_fence_byte_ranges(source);

    let raw_markers = scan_markers(source, body, body_offset, &code_fence_ranges, path)?;
    let tree = assemble(raw_markers, source, path)?;

    let mut ctx = ProcessCtx {
        path,
        requirements: Vec::new(),
        decisions: Vec::new(),
        open_questions: Vec::new(),
        summary: None,
        changelog: None,
        req_ids: HashSet::new(),
        chk_ids: HashSet::new(),
        dec_ids: HashSet::new(),
    };

    for block in tree {
        match block {
            Block::Spec { children, .. } => process_blocks(children, &mut ctx)?,
            other => process_block(other, &mut ctx)?,
        }
    }

    let ProcessCtx {
        requirements,
        decisions,
        open_questions,
        summary,
        changelog,
        ..
    } = ctx;

    let (changelog_body, changelog_span) = changelog.ok_or_else(|| ParseError::MissingField {
        field: "speccy:changelog".to_owned(),
        context: format!("SPEC.md at {path}"),
    })?;

    if changelog_body.trim().is_empty() {
        return Err(ParseError::EmptyMarkerBody {
            path: path.to_path_buf(),
            marker_name: "changelog".to_owned(),
            id: None,
            offset: changelog_span.start,
        });
    }

    let (summary_body, summary_span) = match summary {
        Some((b, s)) => (Some(b), Some(s)),
        None => (None, None),
    };

    Ok(SpecDoc {
        frontmatter_raw,
        heading,
        raw: source.to_owned(),
        requirements,
        decisions,
        open_questions,
        changelog_body,
        changelog_span,
        summary: summary_body,
        summary_span,
    })
}

struct ProcessCtx<'a> {
    path: &'a Utf8Path,
    requirements: Vec<Requirement>,
    decisions: Vec<Decision>,
    open_questions: Vec<OpenQuestion>,
    summary: Option<(String, MarkerSpan)>,
    changelog: Option<(String, MarkerSpan)>,
    req_ids: HashSet<String>,
    chk_ids: HashSet<String>,
    dec_ids: HashSet<String>,
}

fn process_blocks(blocks: Vec<Block>, ctx: &mut ProcessCtx<'_>) -> Result<(), ParseError> {
    for block in blocks {
        process_block(block, ctx)?;
    }
    Ok(())
}

fn process_block(block: Block, ctx: &mut ProcessCtx<'_>) -> Result<(), ParseError> {
    match block {
        Block::Spec { .. } => Err(ParseError::MalformedMarker {
            path: ctx.path.to_path_buf(),
            offset: 0,
            reason: "nested speccy:spec markers are not allowed".to_owned(),
        }),
        Block::Requirement {
            id,
            body,
            children,
            span,
        } => process_requirement(id, body, children, span, ctx),
        Block::Scenario { id, span, .. } => Err(ParseError::ScenarioOutsideRequirement {
            path: ctx.path.to_path_buf(),
            scenario_id: Some(id),
            offset: span.start,
        }),
        Block::Decision {
            id,
            status,
            body,
            span,
        } => {
            if !ctx.dec_ids.insert(id.clone()) {
                return Err(ParseError::DuplicateMarkerId {
                    path: ctx.path.to_path_buf(),
                    marker_name: "decision".to_owned(),
                    id,
                });
            }
            ctx.decisions.push(Decision {
                id,
                status,
                body,
                span,
            });
            Ok(())
        }
        Block::OpenQuestion {
            resolved,
            body,
            span,
        } => {
            ctx.open_questions.push(OpenQuestion {
                resolved,
                body,
                span,
            });
            Ok(())
        }
        Block::Summary { body, span } => {
            if ctx.summary.is_some() {
                return Err(ParseError::MalformedMarker {
                    path: ctx.path.to_path_buf(),
                    offset: span.start,
                    reason: "more than one speccy:summary marker".to_owned(),
                });
            }
            ctx.summary = Some((body, span));
            Ok(())
        }
        Block::Changelog { body, span } => {
            if ctx.changelog.is_some() {
                return Err(ParseError::MalformedMarker {
                    path: ctx.path.to_path_buf(),
                    offset: span.start,
                    reason: "more than one speccy:changelog marker".to_owned(),
                });
            }
            ctx.changelog = Some((body, span));
            Ok(())
        }
    }
}

fn process_requirement(
    id: String,
    body: String,
    children: Vec<Block>,
    span: MarkerSpan,
    ctx: &mut ProcessCtx<'_>,
) -> Result<(), ParseError> {
    if !ctx.req_ids.insert(id.clone()) {
        return Err(ParseError::DuplicateMarkerId {
            path: ctx.path.to_path_buf(),
            marker_name: "requirement".to_owned(),
            id,
        });
    }
    let mut scenarios: Vec<Scenario> = Vec::new();
    for child in children {
        match child {
            Block::Scenario {
                id: child_id,
                body: child_body,
                span: child_span,
            } => {
                if !ctx.chk_ids.insert(child_id.clone()) {
                    return Err(ParseError::DuplicateMarkerId {
                        path: ctx.path.to_path_buf(),
                        marker_name: "scenario".to_owned(),
                        id: child_id,
                    });
                }
                if child_body.trim().is_empty() {
                    return Err(ParseError::EmptyMarkerBody {
                        path: ctx.path.to_path_buf(),
                        marker_name: "scenario".to_owned(),
                        id: Some(child_id),
                        offset: child_span.start,
                    });
                }
                scenarios.push(Scenario {
                    id: child_id,
                    body: child_body,
                    parent_requirement_id: id.clone(),
                    span: child_span,
                });
            }
            other => {
                return Err(ParseError::MalformedMarker {
                    path: ctx.path.to_path_buf(),
                    offset: other.span().start,
                    reason: format!(
                        "marker `{}` is not allowed inside `requirement`",
                        other.marker_name()
                    ),
                });
            }
        }
    }
    if scenarios.is_empty() {
        return Err(ParseError::MalformedMarker {
            path: ctx.path.to_path_buf(),
            offset: span.start,
            reason: format!("requirement `{id}` has no nested scenario markers"),
        });
    }
    if body.trim().is_empty() {
        return Err(ParseError::EmptyMarkerBody {
            path: ctx.path.to_path_buf(),
            marker_name: "requirement".to_owned(),
            id: Some(id.clone()),
            offset: span.start,
        });
    }
    ctx.requirements.push(Requirement {
        id,
        body,
        scenarios,
        span,
    });
    Ok(())
}

#[derive(Debug)]
enum Block {
    Spec {
        children: Vec<Block>,
        span: MarkerSpan,
    },
    Summary {
        body: String,
        span: MarkerSpan,
    },
    Requirement {
        id: String,
        body: String,
        children: Vec<Block>,
        span: MarkerSpan,
    },
    Scenario {
        id: String,
        body: String,
        span: MarkerSpan,
    },
    Decision {
        id: String,
        status: Option<DecisionStatus>,
        body: String,
        span: MarkerSpan,
    },
    OpenQuestion {
        resolved: Option<bool>,
        body: String,
        span: MarkerSpan,
    },
    Changelog {
        body: String,
        span: MarkerSpan,
    },
}

impl Block {
    fn span(&self) -> MarkerSpan {
        match self {
            Block::Spec { span, .. }
            | Block::Summary { span, .. }
            | Block::Requirement { span, .. }
            | Block::Scenario { span, .. }
            | Block::Decision { span, .. }
            | Block::OpenQuestion { span, .. }
            | Block::Changelog { span, .. } => *span,
        }
    }

    fn marker_name(&self) -> &'static str {
        match self {
            Block::Spec { .. } => "spec",
            Block::Summary { .. } => "summary",
            Block::Requirement { .. } => "requirement",
            Block::Scenario { .. } => "scenario",
            Block::Decision { .. } => "decision",
            Block::OpenQuestion { .. } => "open-question",
            Block::Changelog { .. } => "changelog",
        }
    }
}

#[derive(Debug, Clone)]
struct RawMarker {
    name: String,
    is_end: bool,
    attrs: Vec<(String, String)>,
    span: MarkerSpan,
    body_start: usize,
    body_end_after_marker: usize,
}

fn scan_markers(
    source: &str,
    body: &str,
    body_offset: usize,
    code_fence_ranges: &[(usize, usize)],
    path: &Utf8Path,
) -> Result<Vec<RawMarker>, ParseError> {
    let mut markers: Vec<RawMarker> = Vec::new();
    let mut line_start_in_body: usize = 0;

    while line_start_in_body <= body.len() {
        let remainder = body.get(line_start_in_body..).unwrap_or("");
        let (line, next_start_in_body) = if let Some(nl) = remainder.find('\n') {
            let line_end =
                line_start_in_body
                    .checked_add(nl)
                    .ok_or_else(|| ParseError::MalformedMarker {
                        path: path.to_path_buf(),
                        offset: 0,
                        reason: "byte arithmetic overflow during line scan".to_owned(),
                    })?;
            let next = line_end
                .checked_add(1)
                .ok_or_else(|| ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: 0,
                    reason: "byte arithmetic overflow during line scan".to_owned(),
                })?;
            (body.get(line_start_in_body..line_end).unwrap_or(""), next)
        } else if remainder.is_empty() {
            break;
        } else {
            (remainder, body.len().saturating_add(1))
        };

        let abs_line_start = body_offset.checked_add(line_start_in_body).ok_or_else(|| {
            ParseError::MalformedMarker {
                path: path.to_path_buf(),
                offset: 0,
                reason: "byte arithmetic overflow during line scan".to_owned(),
            }
        })?;
        let abs_line_end_excl = body_offset
            .checked_add(line_start_in_body)
            .and_then(|v| v.checked_add(line.len()))
            .ok_or_else(|| ParseError::MalformedMarker {
                path: path.to_path_buf(),
                offset: 0,
                reason: "byte arithmetic overflow during line scan".to_owned(),
            })?;

        if range_inside_any_fence(abs_line_start, abs_line_end_excl, code_fence_ranges) {
            line_start_in_body = next_start_in_body;
            continue;
        }

        if marker_shape_regex().find(line).is_some() {
            let body_start = body_offset
                .checked_add(next_start_in_body.min(body.len()))
                .unwrap_or(source.len());
            let marker =
                parse_marker_line(line, abs_line_start, abs_line_end_excl, body_start, path)?;
            markers.push(marker);
        }

        line_start_in_body = next_start_in_body;
    }

    Ok(markers)
}

fn parse_marker_line(
    line: &str,
    abs_line_start: usize,
    abs_line_end_excl: usize,
    body_start: usize,
    path: &Utf8Path,
) -> Result<RawMarker, ParseError> {
    let trimmed = line.trim_start();
    let leading_ws = line.len().saturating_sub(trimmed.len());
    let abs_marker_offset =
        abs_line_start
            .checked_add(leading_ws)
            .ok_or_else(|| ParseError::MalformedMarker {
                path: path.to_path_buf(),
                offset: 0,
                reason: "byte arithmetic overflow during line scan".to_owned(),
            })?;

    let line_for_regex = trimmed.trim_end();
    let Some(caps) = marker_line_regex().captures(line_for_regex) else {
        let reason = if line.trim() != line_for_regex || !line_for_regex.starts_with("<!--") {
            "speccy marker comments must appear on their own lines".to_owned()
        } else if !line_for_regex.ends_with("-->") {
            "marker comment is missing the closing `-->`".to_owned()
        } else {
            "attribute values must be double-quoted and well-formed".to_owned()
        };
        return Err(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: abs_marker_offset,
            reason,
        });
    };

    let slash = caps.get(1).map_or("", |m| m.as_str());
    let name = caps
        .get(2)
        .map(|m| m.as_str().to_owned())
        .unwrap_or_default();
    let attr_blob = caps.get(3).map_or("", |m| m.as_str());
    let is_end = slash == "/";

    let mut attrs: Vec<(String, String)> = Vec::new();
    if is_end {
        if !attr_blob.trim().is_empty() {
            return Err(ParseError::MalformedMarker {
                path: path.to_path_buf(),
                offset: abs_marker_offset,
                reason: "end marker may not carry attributes".to_owned(),
            });
        }
    } else {
        for ac in attribute_regex().captures_iter(attr_blob) {
            let k = ac.get(1).map(|m| m.as_str().to_owned()).unwrap_or_default();
            let v = ac.get(2).map(|m| m.as_str().to_owned()).unwrap_or_default();
            attrs.push((k, v));
        }
    }

    let span = MarkerSpan {
        start: abs_marker_offset,
        end: abs_line_end_excl,
    };

    Ok(RawMarker {
        name,
        is_end,
        attrs,
        span,
        body_start,
        body_end_after_marker: abs_marker_offset,
    })
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

fn collect_code_fence_byte_ranges(source: &str) -> Vec<(usize, usize)> {
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
        context: format!("SPEC.md at {path}"),
    })
}

fn assemble(raw: Vec<RawMarker>, source: &str, path: &Utf8Path) -> Result<Vec<Block>, ParseError> {
    // Validate marker names + attributes up front, then build a tree by
    // matching start/end markers using a stack.
    for m in &raw {
        validate_marker_shape(m, path)?;
    }

    let mut stack: Vec<PendingBlock> = Vec::new();
    let mut top: Vec<Block> = Vec::new();

    for m in raw {
        if m.is_end {
            let Some(open) = stack.pop() else {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: m.span.start,
                    reason: format!("end marker `/{}` without matching start", m.name),
                });
            };
            if open.name != m.name {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: m.span.start,
                    reason: format!(
                        "end marker `/{}` does not match open start marker `{}`",
                        m.name, open.name
                    ),
                });
            }
            let body = source
                .get(open.body_start..m.body_end_after_marker)
                .unwrap_or("")
                .to_owned();
            let block = open.finish(body, source, path)?;
            if let Some(parent) = stack.last_mut() {
                parent.children.push(block);
            } else {
                top.push(block);
            }
        } else {
            stack.push(PendingBlock {
                name: m.name,
                attrs: m.attrs,
                span: m.span,
                body_start: m.body_start,
                children: Vec::new(),
            });
        }
    }

    if let Some(open) = stack.first() {
        return Err(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: open.span.start,
            reason: format!("start marker `{}` is never closed", open.name),
        });
    }

    Ok(top)
}

fn validate_marker_shape(m: &RawMarker, path: &Utf8Path) -> Result<(), ParseError> {
    let allowed_names = [
        "spec",
        "summary",
        "requirement",
        "scenario",
        "decision",
        "open-question",
        "changelog",
    ];
    if !allowed_names.contains(&m.name.as_str()) {
        return Err(ParseError::UnknownMarkerName {
            path: path.to_path_buf(),
            marker_name: m.name.clone(),
            offset: m.span.start,
        });
    }

    if m.is_end {
        return Ok(());
    }

    let allowed_attrs: &[&str] = match m.name.as_str() {
        "requirement" | "scenario" | "decision" => &["id", "status"],
        "open-question" => &["resolved"],
        _ => &[],
    };

    for (k, v) in &m.attrs {
        let allowed_here: &[&str] = match (m.name.as_str(), k.as_str()) {
            ("requirement" | "scenario", "id") => &["id"],
            ("decision", "id" | "status") => &["id", "status"],
            ("open-question", "resolved") => &["resolved"],
            _ => allowed_attrs,
        };
        if !allowed_here.contains(&k.as_str()) {
            return Err(ParseError::UnknownMarkerAttribute {
                path: path.to_path_buf(),
                marker_name: m.name.clone(),
                attribute: k.clone(),
                offset: m.span.start,
            });
        }
        validate_attribute_value(&m.name, k, v, path)?;
    }
    Ok(())
}

fn validate_attribute_value(
    marker_name: &str,
    attr: &str,
    value: &str,
    path: &Utf8Path,
) -> Result<(), ParseError> {
    match (marker_name, attr) {
        ("requirement", "id") if !req_id_regex().is_match(value) => {
            Err(ParseError::InvalidMarkerId {
                path: path.to_path_buf(),
                marker_name: marker_name.to_owned(),
                id: value.to_owned(),
                expected_pattern: r"REQ-\d{3,}".to_owned(),
            })
        }
        ("scenario", "id") if !chk_id_regex().is_match(value) => Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: marker_name.to_owned(),
            id: value.to_owned(),
            expected_pattern: r"CHK-\d{3,}".to_owned(),
        }),
        ("decision", "id") if !dec_id_regex().is_match(value) => Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: marker_name.to_owned(),
            id: value.to_owned(),
            expected_pattern: r"DEC-\d{3,}".to_owned(),
        }),
        ("decision", "status") if !ALLOWED_DECISION_STATUSES.contains(&value) => {
            Err(ParseError::InvalidMarkerAttributeValue {
                path: path.to_path_buf(),
                marker_name: marker_name.to_owned(),
                attribute: attr.to_owned(),
                value: value.to_owned(),
                allowed: ALLOWED_DECISION_STATUSES.join(", "),
            })
        }
        ("open-question", "resolved") if !ALLOWED_RESOLVED_VALUES.contains(&value) => {
            Err(ParseError::InvalidMarkerAttributeValue {
                path: path.to_path_buf(),
                marker_name: marker_name.to_owned(),
                attribute: attr.to_owned(),
                value: value.to_owned(),
                allowed: ALLOWED_RESOLVED_VALUES.join(", "),
            })
        }
        _ => Ok(()),
    }
}

#[derive(Debug)]
struct PendingBlock {
    name: String,
    attrs: Vec<(String, String)>,
    span: MarkerSpan,
    body_start: usize,
    children: Vec<Block>,
}

impl PendingBlock {
    fn finish(self, body: String, _source: &str, path: &Utf8Path) -> Result<Block, ParseError> {
        let PendingBlock {
            name,
            attrs,
            span,
            body_start: _,
            children,
        } = self;

        let get_attr = |key: &str| -> Option<String> {
            attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
        };

        match name.as_str() {
            "spec" => Ok(Block::Spec { children, span }),
            "summary" => Ok(Block::Summary { body, span }),
            "requirement" => {
                let id = get_attr("id").ok_or_else(|| ParseError::MissingField {
                    field: "id".to_owned(),
                    context: format!("speccy:requirement marker in {path}"),
                })?;
                // Body returned here includes everything between the start
                // marker's terminating newline and the end marker. For the
                // `Requirement.body` field we keep it verbatim; the child
                // scenarios are also stored separately. T-002's renderer
                // can normalize boundaries.
                Ok(Block::Requirement {
                    id,
                    body,
                    children,
                    span,
                })
            }
            "scenario" => {
                let id = get_attr("id").ok_or_else(|| ParseError::MissingField {
                    field: "id".to_owned(),
                    context: format!("speccy:scenario marker in {path}"),
                })?;
                Ok(Block::Scenario { id, body, span })
            }
            "decision" => {
                let id = get_attr("id").ok_or_else(|| ParseError::MissingField {
                    field: "id".to_owned(),
                    context: format!("speccy:decision marker in {path}"),
                })?;
                let status = match get_attr("status").as_deref() {
                    Some("accepted") => Some(DecisionStatus::Accepted),
                    Some("rejected") => Some(DecisionStatus::Rejected),
                    Some("deferred") => Some(DecisionStatus::Deferred),
                    Some("superseded") => Some(DecisionStatus::Superseded),
                    Some(_) | None => None,
                };
                Ok(Block::Decision {
                    id,
                    status,
                    body,
                    span,
                })
            }
            "open-question" => {
                let resolved = match get_attr("resolved").as_deref() {
                    Some("true") => Some(true),
                    Some("false") => Some(false),
                    Some(_) | None => None,
                };
                Ok(Block::OpenQuestion {
                    resolved,
                    body,
                    span,
                })
            }
            "changelog" => Ok(Block::Changelog { body, span }),
            other => Err(ParseError::UnknownMarkerName {
                path: path.to_path_buf(),
                marker_name: other.to_owned(),
                offset: span.start,
            }),
        }
    }
}

/// Deterministically render a [`SpecDoc`] back to a canonical
/// marker-structured Markdown string.
///
/// The output shape is:
///
/// 1. YAML frontmatter fenced by `---` lines, payload taken verbatim from
///    [`SpecDoc::frontmatter_raw`].
/// 2. A blank line, then the level-1 heading (`# {heading}`).
/// 3. Optional `speccy:summary` block, if present.
/// 4. Every [`Requirement`] in [`SpecDoc::requirements`] order. Each
///    requirement renders its body, then every nested [`Scenario`] in
///    [`Requirement::scenarios`] order.
/// 5. Every [`Decision`] in [`SpecDoc::decisions`] order.
/// 6. Every [`OpenQuestion`] in [`SpecDoc::open_questions`] order.
/// 7. The `speccy:changelog` block.
///
/// The renderer is **not** a fully faithful inverse of [`parse`]: free
/// Markdown prose that lived outside any marker block in the source
/// (Goals, Non-goals, Design narrative, Notes, etc.) is not reproduced.
/// Render emits only the typed model. Parse-then-render-then-parse on a
/// rendered document is structurally equivalent (ids, parent links,
/// marker names, bodies); parse-then-render-then-parse on an arbitrary
/// hand-authored SPEC.md drops free prose. See the module-level
/// documentation for context.
///
/// # Determinism contract
///
/// - Every marker comment occupies its own line.
/// - Marker attributes are emitted in a fixed order: `id` first, then any other
///   supported attributes in alphabetical order. Today the only multi-attribute
///   markers are `decision` (`id`, then `status`) and `open-question`
///   (`resolved` — no `id`, so the rule degenerates).
/// - Block order follows struct field order, never source byte offsets.
/// - Marker bodies are emitted verbatim except that the boundary whitespace is
///   normalized: exactly one `\n` between the start marker line and the first
///   body byte, and exactly one `\n` between the last non-whitespace body byte
///   and the end marker line. Interior bytes are preserved exactly (no
///   re-wrapping, no Markdown reformatting).
/// - `render(doc) == render(doc)` byte-for-byte for any valid `doc`.
///
/// This function cannot fail: a [`SpecDoc`] has already been validated by
/// [`parse`], so every invariant the renderer relies on is guaranteed.
#[must_use = "the rendered Markdown string is the canonical projection of the SpecDoc"]
pub fn render(doc: &SpecDoc) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&doc.frontmatter_raw);
    if !doc.frontmatter_raw.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("---\n\n");
    out.push_str("# ");
    out.push_str(&doc.heading);
    out.push('\n');

    if let Some(summary) = &doc.summary {
        out.push('\n');
        push_marker_block(&mut out, "summary", &[], summary);
    }

    for req in &doc.requirements {
        out.push('\n');
        let attrs = [("id", req.id.as_str())];
        push_marker_start(&mut out, "requirement", &attrs);
        // The parser stores the verbatim body between the requirement's
        // start and end markers — which includes nested scenario marker
        // text. We re-emit scenarios from the typed model below to
        // honor `Requirement.scenarios` order, so strip nested scenario
        // blocks out of the prose here.
        let prose = strip_nested_scenario_blocks(&req.body);
        push_body(&mut out, &prose);
        for sc in &req.scenarios {
            let sc_attrs = [("id", sc.id.as_str())];
            push_marker_start(&mut out, "scenario", &sc_attrs);
            push_body(&mut out, &sc.body);
            push_marker_end(&mut out, "scenario");
        }
        push_marker_end(&mut out, "requirement");
    }

    for dec in &doc.decisions {
        out.push('\n');
        let status_str = dec.status.map(DecisionStatus::as_str);
        let mut attrs: Vec<(&str, &str)> = Vec::with_capacity(2);
        attrs.push(("id", dec.id.as_str()));
        if let Some(s) = status_str.as_ref() {
            attrs.push(("status", s));
        }
        push_marker_start(&mut out, "decision", &attrs);
        push_body(&mut out, &dec.body);
        push_marker_end(&mut out, "decision");
    }

    for q in &doc.open_questions {
        out.push('\n');
        let resolved_str = q.resolved.map(|b| if b { "true" } else { "false" });
        let mut attrs: Vec<(&str, &str)> = Vec::new();
        if let Some(r) = resolved_str.as_ref() {
            attrs.push(("resolved", r));
        }
        push_marker_start(&mut out, "open-question", &attrs);
        push_body(&mut out, &q.body);
        push_marker_end(&mut out, "open-question");
    }

    out.push('\n');
    push_marker_block(&mut out, "changelog", &[], &doc.changelog_body);

    out
}

/// Remove nested `speccy:scenario` blocks from a requirement body.
///
/// The parser stores `Requirement.body` as the verbatim source slice
/// between the requirement's start and end markers, which includes
/// nested scenario markers as literal text. The renderer re-emits
/// scenarios from typed state to honor [`Requirement::scenarios`]
/// ordering, so the scenario marker lines must be stripped from the
/// prose first.
///
/// We walk line-by-line and drop runs that begin with a scenario start
/// marker and continue through the matching end marker. The parser has
/// already validated marker shape and nesting, so this scan can rely on
/// a balanced single-level structure (scenarios never nest scenarios).
fn strip_nested_scenario_blocks(body: &str) -> String {
    let start_marker = "<!-- speccy:scenario";
    let end_marker = "<!-- /speccy:scenario";
    let mut out = String::with_capacity(body.len());
    let mut in_scenario = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !in_scenario {
            if trimmed.starts_with(start_marker) {
                in_scenario = true;
                continue;
            }
            out.push_str(line);
        } else if trimmed.starts_with(end_marker) {
            in_scenario = false;
        }
    }
    out
}

fn push_marker_block(out: &mut String, name: &str, attrs: &[(&str, &str)], body: &str) {
    push_marker_start(out, name, attrs);
    push_body(out, body);
    push_marker_end(out, name);
}

fn push_marker_start(out: &mut String, name: &str, attrs: &[(&str, &str)]) {
    out.push_str("<!-- speccy:");
    out.push_str(name);
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(v);
        out.push('"');
    }
    out.push_str(" -->\n");
}

fn push_marker_end(out: &mut String, name: &str) {
    out.push_str("<!-- /speccy:");
    out.push_str(name);
    out.push_str(" -->\n");
}

/// Append `body` with normalized boundary whitespace: drop leading
/// whitespace-only lines and trailing whitespace-only lines, then emit
/// the interior bytes verbatim followed by exactly one `\n` before the
/// trailing end marker.
///
/// "Whitespace-only line" means a sequence of `' '`, `'\t'`, `'\r'`
/// bytes terminated by `'\n'` — i.e. a blank or whitespace-padded blank
/// line. Indentation on the first non-blank line is preserved (e.g. a
/// body that starts with `    code-block-indent` keeps its leading
/// spaces because that line is not whitespace-only).
fn push_body(out: &mut String, body: &str) {
    let interior = trim_blank_boundary_lines(body);
    if interior.is_empty() {
        // `parse` rejects empty required-marker bodies, so this branch
        // only fires for hand-built `SpecDoc`s with empty optional
        // markers. Emit nothing between start and end marker lines.
        return;
    }
    out.push_str(interior);
    out.push('\n');
}

/// Return the slice of `body` with leading and trailing
/// whitespace-only lines removed. See [`push_body`] for the definition
/// of "whitespace-only line".
fn trim_blank_boundary_lines(body: &str) -> &str {
    let bytes = body.as_bytes();
    // Find the byte index of the first non-whitespace-only line.
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
            // Consume the trailing '\n'.
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

    // Find the byte index just past the last non-whitespace-only line.
    let mut end: usize = bytes.len();
    let mut cursor: usize = bytes.len();
    while cursor > start {
        // Find the start of the line ending at `cursor`.
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

#[cfg(test)]
mod tests {
    use super::DecisionStatus;
    use super::parse;
    use crate::error::ParseError;
    use camino::Utf8Path;
    use indoc::indoc;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/SPEC.md")
    }

    fn frontmatter() -> &'static str {
        "---\nid: SPEC-0001\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-11\n---\n\n# Title\n"
    }

    fn make(body_after_heading: &str) -> String {
        format!("{}{}", frontmatter(), body_after_heading)
    }

    #[test]
    fn happy_path_requirement_with_scenario() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            Requirement body prose.

            <!-- speccy:scenario id="CHK-001" -->
            Given a thing, when X, then Y.
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            | Date | Author | Summary |
            <!-- /speccy:changelog -->
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.requirements.len(), 1);
        let req = doc.requirements.first().expect("one requirement");
        assert_eq!(req.id, "REQ-001");
        assert_eq!(req.scenarios.len(), 1);
        let sc = req.scenarios.first().expect("one scenario");
        assert_eq!(sc.id, "CHK-001");
        assert_eq!(sc.parent_requirement_id, "REQ-001");
        assert!(sc.body.contains("Given a thing"));
    }

    #[test]
    fn orphan_scenario_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("orphan scenario must fail");
        assert!(
            matches!(
                &err,
                ParseError::ScenarioOutsideRequirement { scenario_id, .. }
                    if scenario_id.as_deref() == Some("CHK-001")
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn duplicate_chk_id_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            a
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:requirement id="REQ-002" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            b
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("dup must fail");
        assert!(
            matches!(
                &err,
                ParseError::DuplicateMarkerId { marker_name, id, .. }
                    if marker_name == "scenario" && id == "CHK-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn duplicate_req_id_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            a
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-002" -->
            b
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("dup must fail");
        assert!(
            matches!(
                &err,
                ParseError::DuplicateMarkerId { marker_name, id, .. }
                    if marker_name == "requirement" && id == "REQ-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn duplicate_dec_id_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            a
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:decision id="DEC-001" -->
            decision body
            <!-- /speccy:decision -->

            <!-- speccy:decision id="DEC-001" -->
            decision body 2
            <!-- /speccy:decision -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("dup must fail");
        assert!(
            matches!(
                &err,
                ParseError::DuplicateMarkerId { marker_name, id, .. }
                    if marker_name == "decision" && id == "DEC-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn unquoted_attribute_errors() {
        let src = make(indoc! {r"
            <!-- speccy:requirement id=REQ-001 -->
            body
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "});
        let err = parse(&src, path()).expect_err("unquoted attr must fail");
        assert!(
            matches!(&err, ParseError::MalformedMarker { .. }),
            "got: {err:?}",
        );
    }

    #[test]
    fn non_line_isolated_marker_errors() {
        let src = make(indoc! {r#"
            prose before <!-- speccy:requirement id="REQ-001" --> more prose
            body
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("non-isolated must fail");
        assert!(
            matches!(&err, ParseError::MalformedMarker { .. }),
            "got: {err:?}",
        );
    }

    #[test]
    fn unknown_marker_name_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:rationale id="RAT-001" -->
            body
            <!-- /speccy:rationale -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("unknown name must fail");
        assert!(
            matches!(
                &err,
                ParseError::UnknownMarkerName { marker_name, .. } if marker_name == "rationale"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn unknown_attribute_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" priority="high" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("unknown attr must fail");
        assert!(
            matches!(
                &err,
                ParseError::UnknownMarkerAttribute { marker_name, attribute, .. }
                    if marker_name == "requirement" && attribute == "priority"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn invalid_req_id_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-1" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("bad REQ id must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidMarkerId { marker_name, id, .. }
                    if marker_name == "requirement" && id == "REQ-1"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn invalid_chk_id_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHECK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("bad CHK id must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidMarkerId { marker_name, .. } if marker_name == "scenario"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn invalid_dec_id_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:decision id="DECISION-1" -->
            body
            <!-- /speccy:decision -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("bad DEC id must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidMarkerId { marker_name, .. } if marker_name == "decision"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn empty_required_body_errors() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->

            <!-- speccy:scenario id="CHK-001" -->


            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("empty scenario must fail");
        assert!(
            matches!(
                &err,
                ParseError::EmptyMarkerBody { marker_name, .. } if marker_name == "scenario"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn scenario_body_preserves_bytes_verbatim() {
        let body = "Literal <T>, A & B, [link](https://example.com)\n\n```rust\nfn x() {}\n```";
        let src = format!(
            "{front}\n<!-- speccy:requirement id=\"REQ-001\" -->\nintro\n\n<!-- speccy:scenario id=\"CHK-001\" -->\n{body}\n<!-- /speccy:scenario -->\n<!-- /speccy:requirement -->\n\n<!-- speccy:changelog -->\nrow\n<!-- /speccy:changelog -->\n",
            front = frontmatter().trim_end(),
            body = body,
        );
        let doc = parse(&src, path()).expect("parse should succeed");
        let sc = doc
            .requirements
            .first()
            .and_then(|r| r.scenarios.first())
            .expect("scenario should be present");
        assert!(sc.body.contains("<T>"), "got body: {:?}", sc.body);
        assert!(sc.body.contains("A & B"));
        assert!(sc.body.contains("```rust"));
        assert!(sc.body.contains("[link](https://example.com)"));
    }

    #[test]
    fn marker_inside_fenced_code_is_ignored() {
        let src = make(indoc! {r#"
            Example:

            ```markdown
            <!-- speccy:requirement id="REQ-999" -->
            should not be parsed
            <!-- /speccy:requirement -->
            ```

            <!-- speccy:requirement id="REQ-001" -->
            real body

            <!-- speccy:scenario id="CHK-001" -->
            real scenario
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["REQ-001"]);
    }

    #[test]
    fn marker_spans_slice_starts_with_speccy_prefix() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:decision id="DEC-001" status="accepted" -->
            decision body
            <!-- /speccy:decision -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let check = |span: super::MarkerSpan| {
            let slice = src.get(span.start..span.end).expect("span should slice");
            assert!(
                slice.trim_start().starts_with("<!-- speccy:"),
                "span slice did not start with `<!-- speccy:`: {slice:?}",
            );
        };
        for r in &doc.requirements {
            check(r.span);
            for s in &r.scenarios {
                check(s.span);
            }
        }
        for d in &doc.decisions {
            check(d.span);
        }
        check(doc.changelog_span);
    }

    #[test]
    fn no_decision_markers_yields_empty_vec() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert!(doc.decisions.is_empty());
    }

    #[test]
    fn open_question_resolved_must_be_true_or_false() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:open-question resolved="maybe" -->
            text
            <!-- /speccy:open-question -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let err = parse(&src, path()).expect_err("invalid resolved must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidMarkerAttributeValue { marker_name, attribute, value, .. }
                    if marker_name == "open-question" && attribute == "resolved" && value == "maybe"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn decision_status_is_recognized() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <!-- speccy:decision id="DEC-001" status="accepted" -->
            body
            <!-- /speccy:decision -->

            <!-- speccy:changelog -->
            row
            <!-- /speccy:changelog -->
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let dec = doc.decisions.first().expect("decision present");
        assert_eq!(dec.status, Some(DecisionStatus::Accepted));
    }

    #[test]
    fn missing_frontmatter_errors_with_existing_variant() {
        let src = "# Heading only\n<!-- speccy:changelog -->\nrow\n<!-- /speccy:changelog -->\n";
        let err = parse(src, path()).expect_err("missing frontmatter must fail");
        assert!(
            matches!(&err, ParseError::MissingField { field, .. } if field == "frontmatter"),
            "got: {err:?}",
        );
    }

    #[test]
    fn missing_level1_heading_errors() {
        let src = "---\nid: SPEC-0001\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-11\n---\n\nno heading\n<!-- speccy:changelog -->\nrow\n<!-- /speccy:changelog -->\n";
        let err = parse(src, path()).expect_err("missing heading must fail");
        assert!(
            matches!(
                &err,
                ParseError::MissingField { field, .. } if field == "level-1 heading"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn unterminated_frontmatter_surfaces_existing_variant() {
        let src = "---\nid: SPEC-0001\nno closing fence\n";
        let err = parse(src, path()).expect_err("unterminated must fail");
        assert!(
            matches!(&err, ParseError::UnterminatedFrontmatter { .. }),
            "got: {err:?}",
        );
    }
}
