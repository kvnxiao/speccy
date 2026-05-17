//! Raw-XML-element-structured SPEC.md parser and renderer (SPEC-0020
//! carrier, extended by SPEC-0021's section-level element whitelist).
//!
//! Reads a SPEC.md whose body is ordinary Markdown plus line-isolated raw
//! XML open/close tag pairs drawn from a closed whitelist (`requirement`,
//! `scenario`, `decision`, `open-question`, `changelog`, plus SPEC-0021's
//! top-level `goals`, `non-goals`, `user-stories`, optional `assumptions`,
//! and per-requirement `done-when` / `behavior` sub-sections) and returns a
//! typed [`SpecDoc`]. SPEC-0020's `<spec>` root and `<overview>` section
//! were retired by SPEC-0021 DEC-008 and are now rejected by the parser
//! with a dedicated diagnostic.
//!
//! The element scanner is line-aware and treats element-looking content
//! inside fenced code blocks as Markdown body — never structure. Body
//! content between recognised tags is preserved byte-verbatim; it is not
//! XML payload and `<`, `>`, `&` inside it remain ordinary Markdown
//! characters.
//!
//! [`render`] is the deterministic projection of a [`SpecDoc`] back to
//! Markdown source. It is canonical-not-lossless: only the typed model is
//! emitted, so free Markdown prose that lived outside any element block
//! in the source (Goals, Non-goals, Design narrative, Notes, etc.) does
//! **not** roundtrip. Parse-then-render-then-parse on a rendered document
//! is structurally equivalent (ids, parent links, element names, bodies);
//! parse-then-render-then-parse on an arbitrary hand-authored SPEC.md
//! drops free prose. The SPEC-0020 migration tool (T-003) preserves free
//! prose by writing files directly rather than going through this
//! renderer, mirroring the choice SPEC-0019 T-003 made for the marker
//! renderer.
//!
//! See `.speccy/specs/0020-raw-xml-spec-carrier/SPEC.md` REQ-001/REQ-002/
//! REQ-003 for the contract this module satisfies, and DEC-002/DEC-003
//! for the disjointness invariant and the line-aware scanner decision.

use crate::error::ParseError;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
pub use crate::parse::xml_scanner::ElementSpan;
pub use crate::parse::xml_scanner::HTML5_ELEMENT_NAMES;
use crate::parse::xml_scanner::RawTag;
use crate::parse::xml_scanner::ScanConfig;
use crate::parse::xml_scanner::collect_code_fence_byte_ranges;
pub use crate::parse::xml_scanner::is_html5_element_name;
use crate::parse::xml_scanner::scan_tags;
use crate::parse::xml_scanner::unknown_attribute_error;
use camino::Utf8Path;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Parsed raw-XML-structured SPEC.md.
///
/// `frontmatter_raw` carries the YAML frontmatter payload verbatim. The
/// frontmatter is **not** re-validated here; downstream code (workspace
/// loader, T-005) reuses the existing `SpecFrontmatter` deserialisation.
/// T-001 only validates the element tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecDoc {
    /// YAML frontmatter payload between the opening and closing `---`
    /// fences, verbatim.
    pub frontmatter_raw: String,
    /// Text of the level-1 heading after the `# ` prefix, trimmed.
    pub heading: String,
    /// Raw source bytes, retained so [`ElementSpan`] indices remain valid.
    pub raw: String,
    /// Body of the required `<goals>` top-level element, verbatim.
    pub goals: String,
    /// Span of the `<goals>` open tag.
    pub goals_span: ElementSpan,
    /// Body of the required `<non-goals>` top-level element, verbatim.
    pub non_goals: String,
    /// Span of the `<non-goals>` open tag.
    pub non_goals_span: ElementSpan,
    /// Body of the required `<user-stories>` top-level element, verbatim.
    pub user_stories: String,
    /// Span of the `<user-stories>` open tag.
    pub user_stories_span: ElementSpan,
    /// Body of the optional `<assumptions>` top-level element, verbatim,
    /// when present. SPEC-0021 DEC-005 makes the element optional;
    /// specs without load-bearing assumptions omit it entirely.
    pub assumptions: Option<String>,
    /// Span of the `<assumptions>` open tag, when present.
    pub assumptions_span: Option<ElementSpan>,
    /// Requirements declared by `<requirement>` elements in source order.
    pub requirements: Vec<Requirement>,
    /// Decisions declared by `<decision>` elements in source order.
    pub decisions: Vec<Decision>,
    /// Open questions declared by `<open-question>` elements in source
    /// order.
    pub open_questions: Vec<OpenQuestion>,
    /// Body of the single required `<changelog>` element, verbatim.
    pub changelog_body: String,
    /// Span of the `<changelog>` open tag.
    pub changelog_span: ElementSpan,
}

/// One requirement block (`<requirement>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// Id from the `id="..."` attribute (matches `REQ-\d{3,}`).
    pub id: String,
    /// Markdown body between open and close tags, verbatim (nested
    /// `<done-when>`, `<behavior>`, and `<scenario>` tag lines are
    /// included as literal text — the renderer strips them before
    /// re-emitting from typed state).
    pub body: String,
    /// Body of the required `<done-when>` sub-element, verbatim.
    pub done_when: String,
    /// Span of the `<done-when>` open tag.
    pub done_when_span: ElementSpan,
    /// Body of the required `<behavior>` sub-element, verbatim.
    pub behavior: String,
    /// Span of the `<behavior>` open tag.
    pub behavior_span: ElementSpan,
    /// Nested scenarios in source order.
    pub scenarios: Vec<Scenario>,
    /// Span of the open tag.
    pub span: ElementSpan,
}

/// One scenario block (`<scenario>`), nested inside a requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    /// Id from the `id="..."` attribute (matches `CHK-\d{3,}`).
    pub id: String,
    /// Markdown body between open and close tags, verbatim.
    pub body: String,
    /// Id of the containing `<requirement>` element.
    pub parent_requirement_id: String,
    /// Span of the open tag.
    pub span: ElementSpan,
}

/// One decision block (`<decision>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Id from the `id="..."` attribute (matches `DEC-\d{3,}`).
    pub id: String,
    /// Optional `status="accepted|rejected|deferred|superseded"`.
    pub status: Option<DecisionStatus>,
    /// Markdown body between open and close tags, verbatim.
    pub body: String,
    /// Span of the open tag.
    pub span: ElementSpan,
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

/// One open-question element (`<open-question>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenQuestion {
    /// Optional `resolved="true|false"` attribute value.
    pub resolved: Option<bool>,
    /// Markdown body between open and close tags, verbatim.
    pub body: String,
    /// Span of the open tag.
    pub span: ElementSpan,
}

const ALLOWED_DECISION_STATUSES: &[&str] = &["accepted", "rejected", "deferred", "superseded"];
const ALLOWED_RESOLVED_VALUES: &[&str] = &["true", "false"];

/// Closed whitelist of Speccy structure element names. Must remain
/// disjoint from [`HTML5_ELEMENT_NAMES`]; the disjointness unit test
/// below enforces this at build time.
///
/// SPEC-0021 retired `spec` and `overview` from this list (DEC-008) and
/// added six new entries: the per-requirement `behavior` / `done-when`
/// sub-sections (DEC-002) plus four top-level section wrappers
/// (`goals`, `non-goals`, `user-stories`, `assumptions`).
pub const SPECCY_ELEMENT_NAMES: &[&str] = &[
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
];

/// Element names that used to be in the SPEC-0020 whitelist but were
/// retired by SPEC-0021 DEC-008. The scanner still recognises lines that
/// open or close these tags so it can surface a dedicated
/// [`ParseError::RetiredMarkerName`] diagnostic that names SPEC-0021,
/// instead of silently treating them as Markdown body.
const RETIRED_ELEMENT_NAMES: &[&str] = &["spec", "overview"];

/// Concatenate [`SPECCY_ELEMENT_NAMES`] and [`RETIRED_ELEMENT_NAMES`]
/// to drive structure-shaped malformed-tag diagnostics. Retired names
/// still need malformed-shape diagnostics so that, say, an unclosed
/// `<spec ...` line gets the retirement diagnostic from the scanner
/// rather than silently being treated as Markdown.
fn build_structure_shaped_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> =
        Vec::with_capacity(SPECCY_ELEMENT_NAMES.len() + RETIRED_ELEMENT_NAMES.len());
    names.extend_from_slice(SPECCY_ELEMENT_NAMES);
    names.extend_from_slice(RETIRED_ELEMENT_NAMES);
    names
}

/// Run the shared XML scanner with the SPEC.md whitelist, retired-name
/// set, and SPEC-0019 legacy-marker detection enabled. Centralising the
/// configuration keeps [`parse`] short and gives a single grep target
/// for "what tags does SPEC.md recognise".
fn scan_spec_tags(
    source: &str,
    body: &str,
    body_offset: usize,
    path: &Utf8Path,
) -> Result<Vec<RawTag>, ParseError> {
    let code_fence_ranges = collect_code_fence_byte_ranges(source);
    let structure_shaped_names = build_structure_shaped_names();
    let cfg = ScanConfig {
        whitelist: SPECCY_ELEMENT_NAMES,
        structure_shaped_names: &structure_shaped_names,
        retired_names: RETIRED_ELEMENT_NAMES,
        detect_legacy_markers: true,
    };
    scan_tags(source, body, body_offset, &code_fence_ranges, path, &cfg)
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

/// Parse a raw-XML-structured SPEC.md source string.
///
/// `source` is the file contents; `path` is used only to populate
/// diagnostics — this function does no filesystem IO.
///
/// # Errors
///
/// Returns [`ParseError`] for missing frontmatter or level-1 heading,
/// element-shape problems, unknown element names or attributes,
/// id-pattern violations, duplicate ids, orphan scenarios, empty
/// required bodies, invalid attribute values, or surviving SPEC-0019
/// HTML-comment markers outside fenced code blocks.
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

    let raw_tags = scan_spec_tags(source, body, body_offset, path)?;
    let tree = assemble(raw_tags, source, path)?;

    let mut ctx = ProcessCtx {
        path,
        requirements: Vec::new(),
        decisions: Vec::new(),
        open_questions: Vec::new(),
        goals: None,
        non_goals: None,
        user_stories: None,
        assumptions: None,
        changelog: None,
        req_ids: HashSet::new(),
        chk_ids: HashSet::new(),
        dec_ids: HashSet::new(),
    };

    for block in tree {
        process_block(block, &mut ctx)?;
    }

    let ProcessCtx {
        requirements,
        decisions,
        open_questions,
        goals,
        non_goals,
        user_stories,
        assumptions,
        changelog,
        ..
    } = ctx;

    let (changelog_body, changelog_span) = changelog.ok_or_else(|| ParseError::MissingField {
        field: "<changelog>".to_owned(),
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

    let (goals_body, goals_span) = goals.ok_or_else(|| ParseError::MissingRequiredSection {
        path: path.to_path_buf(),
        element_name: "goals".to_owned(),
    })?;
    let (non_goals_body, non_goals_span) =
        non_goals.ok_or_else(|| ParseError::MissingRequiredSection {
            path: path.to_path_buf(),
            element_name: "non-goals".to_owned(),
        })?;
    let (user_stories_body, user_stories_span) =
        user_stories.ok_or_else(|| ParseError::MissingRequiredSection {
            path: path.to_path_buf(),
            element_name: "user-stories".to_owned(),
        })?;
    let (assumptions_body, assumptions_span) = match assumptions {
        Some((b, s)) => (Some(b), Some(s)),
        None => (None, None),
    };

    Ok(SpecDoc {
        frontmatter_raw,
        heading,
        raw: source.to_owned(),
        goals: goals_body,
        goals_span,
        non_goals: non_goals_body,
        non_goals_span,
        user_stories: user_stories_body,
        user_stories_span,
        assumptions: assumptions_body,
        assumptions_span,
        requirements,
        decisions,
        open_questions,
        changelog_body,
        changelog_span,
    })
}

/// Render a [`SpecDoc`] to its canonical raw-XML SPEC.md form.
///
/// The output is a Markdown document with raw XML element tags carrying
/// Speccy structure:
///
/// 1. Frontmatter fence followed by [`SpecDoc::frontmatter_raw`].
/// 2. A blank line, then the level-1 heading (`# {heading}`).
/// 3. `<goals>`, `<non-goals>`, `<user-stories>` top-level sections.
/// 4. Every [`Requirement`] in [`SpecDoc::requirements`] order. Each
///    requirement renders its prose (with nested `<done-when>`, `<behavior>`,
///    and `<scenario>` tag lines stripped out), then `<done-when>` and
///    `<behavior>`, then every nested [`Scenario`] in
///    [`Requirement::scenarios`] order.
/// 5. Every [`Decision`] in [`SpecDoc::decisions`] order.
/// 6. Every [`OpenQuestion`] in [`SpecDoc::open_questions`] order.
/// 7. Optional `<assumptions>` block, if present.
/// 8. The required `<changelog>` block.
///
/// The renderer is canonical-not-lossless. Free Markdown prose between
/// elements in a hand-authored source (Goals, Non-goals, Design,
/// Migration, Notes, etc.) is **not** preserved — render emits only the
/// typed model. The SPEC-0020 migration tool (T-003) is responsible for
/// preserving free prose by rewriting source files directly rather than
/// going through this renderer; see the module-level doc for the same
/// trade-off SPEC-0019 T-002 documented for the marker renderer.
///
/// # Determinism contract
///
/// - Every element open and close tag occupies its own line. Nothing else
///   shares the tag's line.
/// - Element attributes are emitted in a fixed order. Today the only
///   multi-attribute element is `<decision>`, which always emits `id` before
///   `status`. `<open-question>` carries at most `resolved` only.
/// - Block order follows struct field order ([`SpecDoc::requirements`],
///   [`Requirement::scenarios`], [`SpecDoc::decisions`],
///   [`SpecDoc::open_questions`]) — never source byte offsets.
/// - Element bodies are emitted verbatim except that boundary whitespace is
///   normalised: leading and trailing whitespace-only lines inside the body are
///   dropped, then exactly one `\n` separates the open tag line from the first
///   body byte and exactly one `\n` separates the last non-whitespace body byte
///   from the close tag line. Interior bytes (including fenced code blocks,
///   inline backticks, and literal `<` / `>` / `&`) are preserved
///   byte-for-byte.
/// - Every closing element tag is followed by a single blank line. This is the
///   SPEC-0020 Open Question 2 resolution: the canonical fixture and shipped
///   SPEC.md files all favour visual separation between top-level blocks over
///   diff width, and applying the same rule between nested scenarios keeps the
///   renderer's emission shape uniform (one rule, no special cases). Roundtrip
///   equivalence is structural — not byte-identical — so the rule does not need
///   to match the original source's whitespace.
/// - `render(doc) == render(doc)` byte-for-byte for any valid `doc`.
/// - The renderer never emits the SPEC-0019 `<!-- speccy:` HTML-comment marker
///   form (REQ-002 contract).
///
/// This function cannot fail: a [`SpecDoc`] has already been validated
/// by [`parse`], so every invariant the renderer relies on is
/// guaranteed.
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

    out.push('\n');
    push_element_block(&mut out, "goals", &[], &doc.goals);
    out.push('\n');
    push_element_block(&mut out, "non-goals", &[], &doc.non_goals);
    out.push('\n');
    push_element_block(&mut out, "user-stories", &[], &doc.user_stories);

    for req in &doc.requirements {
        out.push('\n');
        let attrs = [("id", req.id.as_str())];
        push_element_open(&mut out, "requirement", &attrs);
        // The parser stores `Requirement.body` as the verbatim slice
        // between the requirement's open and close tags, which includes
        // nested done-when, behavior, and scenario tag lines as literal
        // text. The renderer re-emits each sub-section from typed state
        // to honour the SPEC-0021 canonical order, so strip those nested
        // tag blocks out of the prose here.
        let prose = strip_nested_requirement_sub_blocks(&req.body);
        push_body(&mut out, &prose);
        push_element_block(&mut out, "done-when", &[], &req.done_when);
        push_element_block(&mut out, "behavior", &[], &req.behavior);
        for sc in &req.scenarios {
            let sc_attrs = [("id", sc.id.as_str())];
            push_element_open(&mut out, "scenario", &sc_attrs);
            push_body(&mut out, &sc.body);
            push_element_close(&mut out, "scenario");
        }
        push_element_close(&mut out, "requirement");
    }

    for dec in &doc.decisions {
        out.push('\n');
        let status_str = dec.status.map(DecisionStatus::as_str);
        let mut attrs: Vec<(&str, &str)> = Vec::with_capacity(2);
        attrs.push(("id", dec.id.as_str()));
        if let Some(s) = status_str.as_ref() {
            attrs.push(("status", s));
        }
        push_element_block(&mut out, "decision", &attrs, &dec.body);
    }

    for q in &doc.open_questions {
        out.push('\n');
        let resolved_str = q.resolved.map(|b| if b { "true" } else { "false" });
        let mut attrs: Vec<(&str, &str)> = Vec::new();
        if let Some(r) = resolved_str.as_ref() {
            attrs.push(("resolved", r));
        }
        push_element_block(&mut out, "open-question", &attrs, &q.body);
    }

    if let Some(assumptions) = &doc.assumptions {
        out.push('\n');
        push_element_block(&mut out, "assumptions", &[], assumptions);
    }

    out.push('\n');
    push_element_block(&mut out, "changelog", &[], &doc.changelog_body);

    out
}

/// Strip nested `<done-when>`, `<behavior>`, and `<scenario>` blocks
/// from a requirement body.
///
/// The parser stores `Requirement.body` as the verbatim source slice
/// between the requirement's open and close tags, which includes all
/// nested SPEC-0021 sub-section tag lines as literal text. [`render`]
/// re-emits those sub-sections from typed state to honour the canonical
/// order, so the tag lines must be stripped from the surrounding prose
/// first.
///
/// We walk line-by-line and drop runs that begin with a sub-section
/// open tag and continue through the matching close tag. The parser
/// has already validated tag shape and nesting, so this scan can rely
/// on balanced single-level structure (sub-sections never nest each
/// other).
fn strip_nested_requirement_sub_blocks(body: &str) -> String {
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
        if (trimmed.starts_with("<scenario ") || trimmed.starts_with("<scenario>"))
            && !trimmed.starts_with("</scenario>")
        {
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
    // Determinism contract: every closing element tag is followed by a
    // single blank line. See [`render`] doc for the rationale.
    out.push('\n');
}

/// Append `body` with normalised boundary whitespace: drop leading
/// whitespace-only lines and trailing whitespace-only lines, then emit
/// the interior bytes verbatim followed by exactly one `\n` before the
/// trailing close tag line.
///
/// "Whitespace-only line" means a sequence of `' '`, `'\t'`, `'\r'`
/// bytes terminated by `'\n'` — i.e. a blank or whitespace-padded blank
/// line. Indentation on the first non-blank line is preserved (e.g. a
/// body that starts with `    code-block-indent` keeps its leading
/// spaces because that line is not whitespace-only).
fn push_body(out: &mut String, body: &str) {
    let interior = trim_blank_boundary_lines(body);
    if interior.is_empty() {
        // `parse` rejects empty required-element bodies, so this branch
        // only fires for hand-built `SpecDoc`s with empty optional
        // elements. Emit nothing between open and close tag lines.
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

struct ProcessCtx<'a> {
    path: &'a Utf8Path,
    requirements: Vec<Requirement>,
    decisions: Vec<Decision>,
    open_questions: Vec<OpenQuestion>,
    goals: Option<(String, ElementSpan)>,
    non_goals: Option<(String, ElementSpan)>,
    user_stories: Option<(String, ElementSpan)>,
    assumptions: Option<(String, ElementSpan)>,
    changelog: Option<(String, ElementSpan)>,
    req_ids: HashSet<String>,
    chk_ids: HashSet<String>,
    dec_ids: HashSet<String>,
}

fn process_block(block: Block, ctx: &mut ProcessCtx<'_>) -> Result<(), ParseError> {
    match block {
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
        Block::Goals { body, span } => {
            assign_top_section(&mut ctx.goals, "goals", body, span, ctx.path)
        }
        Block::NonGoals { body, span } => {
            assign_top_section(&mut ctx.non_goals, "non-goals", body, span, ctx.path)
        }
        Block::UserStories { body, span } => {
            assign_top_section(&mut ctx.user_stories, "user-stories", body, span, ctx.path)
        }
        Block::Assumptions { body, span } => {
            assign_top_section(&mut ctx.assumptions, "assumptions", body, span, ctx.path)
        }
        Block::DoneWhen { span, .. } => Err(ParseError::MalformedMarker {
            path: ctx.path.to_path_buf(),
            offset: span.start,
            reason: "<done-when> element is only allowed inside a <requirement>".to_owned(),
        }),
        Block::Behavior { span, .. } => Err(ParseError::MalformedMarker {
            path: ctx.path.to_path_buf(),
            offset: span.start,
            reason: "<behavior> element is only allowed inside a <requirement>".to_owned(),
        }),
        Block::Changelog { body, span } => {
            if ctx.changelog.is_some() {
                return Err(ParseError::MalformedMarker {
                    path: ctx.path.to_path_buf(),
                    offset: span.start,
                    reason: "more than one <changelog> element".to_owned(),
                });
            }
            ctx.changelog = Some((body, span));
            Ok(())
        }
    }
}

fn assign_top_section(
    slot: &mut Option<(String, ElementSpan)>,
    element_name: &str,
    body: String,
    span: ElementSpan,
    path: &Utf8Path,
) -> Result<(), ParseError> {
    if slot.is_some() {
        return Err(ParseError::DuplicateSection {
            path: path.to_path_buf(),
            element_name: element_name.to_owned(),
            offset: span.start,
        });
    }
    *slot = Some((body, span));
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "single-pass requirement validator; SPEC-0021 added behaviour / done-when sub-section bookkeeping inline rather than splitting and re-walking child blocks"
)]
fn process_requirement(
    id: String,
    body: String,
    children: Vec<Block>,
    span: ElementSpan,
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
    let mut done_when: Option<(String, ElementSpan)> = None;
    let mut behavior: Option<(String, ElementSpan)> = None;

    for child in children {
        match child {
            Block::DoneWhen {
                body: child_body,
                span: child_span,
            } => {
                if done_when.is_some() {
                    return Err(ParseError::DuplicateRequirementSection {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        element_name: "done-when".to_owned(),
                        offset: child_span.start,
                    });
                }
                if behavior.is_some() {
                    return Err(ParseError::RequirementSectionOrder {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        offset: child_span.start,
                        reason: "<done-when> must appear before <behavior>".to_owned(),
                    });
                }
                if !scenarios.is_empty() {
                    return Err(ParseError::RequirementSectionOrder {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        offset: child_span.start,
                        reason: "<done-when> must appear before any <scenario>".to_owned(),
                    });
                }
                done_when = Some((child_body, child_span));
            }
            Block::Behavior {
                body: child_body,
                span: child_span,
            } => {
                if behavior.is_some() {
                    return Err(ParseError::DuplicateRequirementSection {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        element_name: "behavior".to_owned(),
                        offset: child_span.start,
                    });
                }
                if !scenarios.is_empty() {
                    return Err(ParseError::RequirementSectionOrder {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        offset: child_span.start,
                        reason: "<behavior> must appear before any <scenario>".to_owned(),
                    });
                }
                behavior = Some((child_body, child_span));
            }
            Block::Scenario {
                id: child_id,
                body: child_body,
                span: child_span,
            } => {
                if done_when.is_none() {
                    return Err(ParseError::RequirementSectionOrder {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        offset: child_span.start,
                        reason: "<scenario> must appear after <done-when> and <behavior>"
                            .to_owned(),
                    });
                }
                if behavior.is_none() {
                    return Err(ParseError::RequirementSectionOrder {
                        path: ctx.path.to_path_buf(),
                        requirement_id: id.clone(),
                        offset: child_span.start,
                        reason: "<scenario> must appear after <behavior>".to_owned(),
                    });
                }
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
                        "element `{}` is not allowed inside `requirement`",
                        other.element_name()
                    ),
                });
            }
        }
    }

    let (done_when_body, done_when_span) =
        done_when.ok_or_else(|| ParseError::MissingRequirementSection {
            path: ctx.path.to_path_buf(),
            requirement_id: id.clone(),
            element_name: "done-when".to_owned(),
        })?;
    let (behavior_body, behavior_span) =
        behavior.ok_or_else(|| ParseError::MissingRequirementSection {
            path: ctx.path.to_path_buf(),
            requirement_id: id.clone(),
            element_name: "behavior".to_owned(),
        })?;

    if scenarios.is_empty() {
        return Err(ParseError::MalformedMarker {
            path: ctx.path.to_path_buf(),
            offset: span.start,
            reason: format!("requirement `{id}` has no nested scenario elements"),
        });
    }
    if done_when_body.trim().is_empty() {
        return Err(ParseError::EmptyMarkerBody {
            path: ctx.path.to_path_buf(),
            marker_name: "done-when".to_owned(),
            id: Some(id.clone()),
            offset: done_when_span.start,
        });
    }
    if behavior_body.trim().is_empty() {
        return Err(ParseError::EmptyMarkerBody {
            path: ctx.path.to_path_buf(),
            marker_name: "behavior".to_owned(),
            id: Some(id.clone()),
            offset: behavior_span.start,
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
        done_when: done_when_body,
        done_when_span,
        behavior: behavior_body,
        behavior_span,
        scenarios,
        span,
    });
    Ok(())
}

#[derive(Debug)]
enum Block {
    Requirement {
        id: String,
        body: String,
        children: Vec<Block>,
        span: ElementSpan,
    },
    Scenario {
        id: String,
        body: String,
        span: ElementSpan,
    },
    Decision {
        id: String,
        status: Option<DecisionStatus>,
        body: String,
        span: ElementSpan,
    },
    OpenQuestion {
        resolved: Option<bool>,
        body: String,
        span: ElementSpan,
    },
    DoneWhen {
        body: String,
        span: ElementSpan,
    },
    Behavior {
        body: String,
        span: ElementSpan,
    },
    Goals {
        body: String,
        span: ElementSpan,
    },
    NonGoals {
        body: String,
        span: ElementSpan,
    },
    UserStories {
        body: String,
        span: ElementSpan,
    },
    Assumptions {
        body: String,
        span: ElementSpan,
    },
    Changelog {
        body: String,
        span: ElementSpan,
    },
}

impl Block {
    fn span(&self) -> ElementSpan {
        match self {
            Block::Requirement { span, .. }
            | Block::Scenario { span, .. }
            | Block::Decision { span, .. }
            | Block::OpenQuestion { span, .. }
            | Block::DoneWhen { span, .. }
            | Block::Behavior { span, .. }
            | Block::Goals { span, .. }
            | Block::NonGoals { span, .. }
            | Block::UserStories { span, .. }
            | Block::Assumptions { span, .. }
            | Block::Changelog { span, .. } => *span,
        }
    }

    fn element_name(&self) -> &'static str {
        match self {
            Block::Requirement { .. } => "requirement",
            Block::Scenario { .. } => "scenario",
            Block::Decision { .. } => "decision",
            Block::OpenQuestion { .. } => "open-question",
            Block::DoneWhen { .. } => "done-when",
            Block::Behavior { .. } => "behavior",
            Block::Goals { .. } => "goals",
            Block::NonGoals { .. } => "non-goals",
            Block::UserStories { .. } => "user-stories",
            Block::Assumptions { .. } => "assumptions",
            Block::Changelog { .. } => "changelog",
        }
    }
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

fn assemble(raw: Vec<RawTag>, source: &str, path: &Utf8Path) -> Result<Vec<Block>, ParseError> {
    for t in &raw {
        validate_tag_shape(t, path)?;
    }

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
        return Err(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: open.span.start,
            reason: format!("open tag `<{}>` is never closed", open.name),
        });
    }

    Ok(top)
}

fn validate_tag_shape(t: &RawTag, path: &Utf8Path) -> Result<(), ParseError> {
    if !SPECCY_ELEMENT_NAMES.contains(&t.name.as_str()) {
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
        "requirement" | "scenario" => &["id"],
        "decision" => &["id", "status"],
        "open-question" => &["resolved"],
        _ => &[],
    };

    for (k, v) in &t.attrs {
        if !allowed_attrs.contains(&k.as_str()) {
            return Err(unknown_attribute_error(
                path,
                &t.name,
                k,
                t.span.start,
                allowed_attrs,
            ));
        }
        validate_attribute_value(&t.name, k, v, path)?;
    }
    Ok(())
}

fn validate_attribute_value(
    element_name: &str,
    attr: &str,
    value: &str,
    path: &Utf8Path,
) -> Result<(), ParseError> {
    match (element_name, attr) {
        ("requirement", "id") if !req_id_regex().is_match(value) => {
            Err(ParseError::InvalidMarkerId {
                path: path.to_path_buf(),
                marker_name: element_name.to_owned(),
                id: value.to_owned(),
                expected_pattern: r"REQ-\d{3,}".to_owned(),
            })
        }
        ("scenario", "id") if !chk_id_regex().is_match(value) => Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: element_name.to_owned(),
            id: value.to_owned(),
            expected_pattern: r"CHK-\d{3,}".to_owned(),
        }),
        ("decision", "id") if !dec_id_regex().is_match(value) => Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: element_name.to_owned(),
            id: value.to_owned(),
            expected_pattern: r"DEC-\d{3,}".to_owned(),
        }),
        ("decision", "status") if !ALLOWED_DECISION_STATUSES.contains(&value) => {
            Err(ParseError::InvalidMarkerAttributeValue {
                path: path.to_path_buf(),
                marker_name: element_name.to_owned(),
                attribute: attr.to_owned(),
                value: value.to_owned(),
                allowed: ALLOWED_DECISION_STATUSES.join(", "),
            })
        }
        ("open-question", "resolved") if !ALLOWED_RESOLVED_VALUES.contains(&value) => {
            Err(ParseError::InvalidMarkerAttributeValue {
                path: path.to_path_buf(),
                marker_name: element_name.to_owned(),
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
    span: ElementSpan,
    body_start: usize,
    children: Vec<Block>,
}

impl PendingBlock {
    fn finish(self, body: String, path: &Utf8Path) -> Result<Block, ParseError> {
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
            "requirement" => {
                let id = get_attr("id").ok_or_else(|| ParseError::MissingField {
                    field: "id".to_owned(),
                    context: format!("<requirement> element in {path}"),
                })?;
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
                    context: format!("<scenario> element in {path}"),
                })?;
                Ok(Block::Scenario { id, body, span })
            }
            "decision" => {
                let id = get_attr("id").ok_or_else(|| ParseError::MissingField {
                    field: "id".to_owned(),
                    context: format!("<decision> element in {path}"),
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
            "done-when" => Ok(Block::DoneWhen { body, span }),
            "behavior" => Ok(Block::Behavior { body, span }),
            "goals" => Ok(Block::Goals { body, span }),
            "non-goals" => Ok(Block::NonGoals { body, span }),
            "user-stories" => Ok(Block::UserStories { body, span }),
            "assumptions" => Ok(Block::Assumptions { body, span }),
            "changelog" => Ok(Block::Changelog { body, span }),
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
    use super::DecisionStatus;
    use super::HTML5_ELEMENT_NAMES;
    use super::SPECCY_ELEMENT_NAMES;
    use super::is_html5_element_name;
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

    /// SPEC-0021 makes `<goals>`, `<non-goals>`, and `<user-stories>` required
    /// top-level sections; every test fixture that exercises a `<requirement>`
    /// must include them so the parser sees a structurally valid spec.
    fn top_sections() -> &'static str {
        "\n<goals>\nGoals body.\n</goals>\n\n<non-goals>\nNon-goals body.\n</non-goals>\n\n<user-stories>\n- A story.\n</user-stories>\n\n"
    }

    /// SPEC-0021 makes `<done-when>` and `<behavior>` required sub-elements
    /// inside `<requirement>`, before any `<scenario>`. Tests insert this
    /// canned block between their requirement prose and the nested scenarios
    /// so the structural validator sees a complete requirement.
    fn req_intro() -> &'static str {
        "<done-when>\n- placeholder done-when bullet.\n</done-when>\n\n<behavior>\n- placeholder behavior bullet.\n</behavior>\n\n"
    }

    fn make(body_after_heading: &str) -> String {
        format!("{}{}{}", frontmatter(), top_sections(), body_after_heading)
    }

    #[test]
    fn happy_path_requirement_with_scenario() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            Requirement body prose.

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            Given a thing, when X, then Y.
            </scenario>
            </requirement>

            <changelog>
            | Date | Author | Summary |
            </changelog>
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
    fn orphan_scenario_errors_with_id() {
        let src = make(indoc! {r#"
            <scenario id="CHK-001">
            text
            </scenario>

            <changelog>
            row
            </changelog>
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
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            a
            </scenario>
            </requirement>

            <requirement id="REQ-002">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            b
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
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
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            a
            </scenario>
            </requirement>

            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-002">
            b
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
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
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            a
            </scenario>
            </requirement>

            <decision id="DEC-001">
            decision body
            </decision>

            <decision id="DEC-001">
            decision body 2
            </decision>

            <changelog>
            row
            </changelog>
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
            <requirement id=REQ-001>
            body
            </requirement>

            <changelog>
            row
            </changelog>
        "});
        let err = parse(&src, path()).expect_err("unquoted attr must fail");
        assert!(
            matches!(&err, ParseError::MalformedMarker { .. }),
            "got: {err:?}",
        );
    }

    #[test]
    fn non_line_isolated_open_errors() {
        let src = make(indoc! {r#"
            prose <requirement id="REQ-001">
            body

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let err = parse(&src, path()).expect_err("non-isolated open must fail");
        // Prose-prefixed open tags don't match either the strict or the
        // shape regex (the shape regex anchors at the line start), so
        // the parser treats them as Markdown — but the close tag and
        // changelog must still validate. With no recognised open tag,
        // there's no scenario nest, and the `<requirement>` open below
        // an emitted close tag would surface as a mismatch. To assert
        // the error precisely, also test a clean case with `</requirement> prose`:
        assert!(
            matches!(&err, ParseError::MalformedMarker { .. }),
            "got: {err:?}",
        );
    }

    #[test]
    fn non_line_isolated_close_errors() {
        // Close tag with trailing prose is detected by the shape regex
        // and surfaced as malformed.
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement> trailing prose

            <changelog>
            row
            </changelog>
        "#});
        let err = parse(&src, path()).expect_err("non-isolated close must fail");
        assert!(
            matches!(&err, ParseError::MalformedMarker { .. }),
            "got: {err:?}",
        );
    }

    #[test]
    fn unknown_element_name_is_treated_as_markdown_body() {
        // `<rationale>` is not in the whitelist; the scanner must skip
        // it without producing structure, so the parse should succeed
        // and yield only the explicit `<requirement>` element.
        let src = make(indoc! {r#"
            <rationale>
            free prose

            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["REQ-001"]);
    }

    #[test]
    fn html5_element_name_on_own_line_is_markdown_body() {
        let src = make(indoc! {r#"
            <section>
            <details>

            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.requirements.len(), 1);
    }

    #[test]
    fn unknown_attribute_errors() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001" priority="high">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
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
            <requirement id="REQ-1">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
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
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHECK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
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
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <decision id="DECISION-1">
            body
            </decision>

            <changelog>
            row
            </changelog>
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
    fn empty_required_scenario_body_errors() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            requirement prose

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">


            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
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
    fn requirement_body_without_prose_but_with_scenarios_parses() {
        // The parser stores `Requirement.body` as the verbatim slice
        // between open and close tags (including nested scenario tag
        // text), matching the SPEC-0019 contract. A valid scenario
        // therefore satisfies the "non-whitespace body" rule even when
        // the requirement carries no free prose. The renderer (T-002)
        // is responsible for stripping nested scenario tag lines when
        // re-emitting the requirement prose.
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let req = doc.requirements.first().expect("one requirement");
        assert_eq!(req.id, "REQ-001");
        assert_eq!(req.scenarios.len(), 1);
    }

    #[test]
    fn empty_required_changelog_body_errors() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>

            </changelog>
        "#});
        let err = parse(&src, path()).expect_err("empty changelog must fail");
        assert!(
            matches!(
                &err,
                ParseError::EmptyMarkerBody { marker_name, .. } if marker_name == "changelog"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn scenario_body_preserves_bytes_verbatim() {
        let body = "Literal <thinking>, <example>, <T>, A & B, [link](https://example.com)\n\n```rust\nfn x() {}\n```";
        let src = format!(
            "{front}{top}<requirement id=\"REQ-001\">\nintro prose\n\n{intro}<scenario id=\"CHK-001\">\n{body}\n</scenario>\n</requirement>\n\n<changelog>\nrow\n</changelog>\n",
            front = frontmatter().trim_end(),
            top = top_sections(),
            intro = req_intro(),
            body = body,
        );
        let doc = parse(&src, path()).expect("parse should succeed");
        let sc = doc
            .requirements
            .first()
            .and_then(|r| r.scenarios.first())
            .expect("scenario should be present");
        assert!(sc.body.contains("<thinking>"), "body: {:?}", sc.body);
        assert!(sc.body.contains("<example>"));
        assert!(sc.body.contains("<T>"));
        assert!(sc.body.contains("A & B"));
        assert!(sc.body.contains("```rust"));
        assert!(sc.body.contains("[link](https://example.com)"));
    }

    #[test]
    fn open_tag_inside_fenced_code_is_ignored() {
        let src = make(indoc! {r#"
            Example:

            ```markdown
            <requirement id="REQ-999">
            should not be parsed
            </requirement>
            ```

            <requirement id="REQ-001">
            real body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            real scenario
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["REQ-001"]);
    }

    #[test]
    fn inline_backticked_element_is_not_structure() {
        // A structure-shaped line wrapped in inline backticks must be
        // treated as Markdown body content, not a tag. The parser drops
        // backtick code spans from its element scan because the line
        // does not start with `<` after trimming.
        let src = make(indoc! {r#"
            Example inline: `<requirement id="REQ-999">` is not structure.

            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["REQ-001"]);
    }

    #[test]
    fn element_spans_slice_starts_with_lt_and_name() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <decision id="DEC-001" status="accepted">
            decision body
            </decision>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let check = |span: super::ElementSpan, name: &str| {
            let slice = src.get(span.start..span.end).expect("span should slice");
            assert!(
                slice.trim_start().starts_with('<'),
                "span slice did not start with `<`: {slice:?}",
            );
            assert!(
                slice.contains(name),
                "span slice for {name} did not contain the element name: {slice:?}",
            );
        };
        for r in &doc.requirements {
            check(r.span, "requirement");
            for s in &r.scenarios {
                check(s.span, "scenario");
            }
        }
        for d in &doc.decisions {
            check(d.span, "decision");
        }
        check(doc.changelog_span, "changelog");
    }

    #[test]
    fn no_decision_elements_yields_empty_vec() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert!(doc.decisions.is_empty());
    }

    #[test]
    fn open_question_resolved_must_be_true_or_false() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <open-question resolved="maybe">
            text
            </open-question>

            <changelog>
            row
            </changelog>
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
    fn open_question_resolved_true_is_recognized() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <open-question resolved="true">
            text
            </open-question>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let q = doc.open_questions.first().expect("open question present");
        assert_eq!(q.resolved, Some(true));
    }

    #[test]
    fn decision_status_is_recognized() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <decision id="DEC-001" status="accepted">
            body
            </decision>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        let dec = doc.decisions.first().expect("decision present");
        assert_eq!(dec.status, Some(DecisionStatus::Accepted));
    }

    #[test]
    fn missing_frontmatter_uses_existing_error_variant() {
        let src = "# Heading only\n<changelog>\nrow\n</changelog>\n";
        let err = parse(src, path()).expect_err("missing frontmatter must fail");
        assert!(
            matches!(&err, ParseError::MissingField { field, .. } if field == "frontmatter"),
            "got: {err:?}",
        );
    }

    #[test]
    fn missing_level1_heading_uses_existing_error_variant() {
        let src = "---\nid: SPEC-0001\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-11\n---\n\nno heading\n<changelog>\nrow\n</changelog>\n";
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

    #[test]
    fn legacy_html_comment_marker_open_errors_with_dedicated_variant() {
        let src = make(indoc! {r#"
            <!-- speccy:requirement id="REQ-001" -->
            body

            <!-- speccy:scenario id="CHK-001" -->
            text
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->

            <changelog>
            row
            </changelog>
        "#});
        let err = parse(&src, path()).expect_err("legacy marker must fail");
        assert!(
            matches!(err, ParseError::LegacyMarker { .. }),
            "expected LegacyMarker variant, got {err:?}",
        );
        if let ParseError::LegacyMarker {
            ref legacy_form,
            ref suggested_element,
            ..
        } = err
        {
            assert!(
                legacy_form.contains("speccy:requirement"),
                "legacy form did not mention `speccy:requirement`: {legacy_form:?}",
            );
            assert!(
                suggested_element.contains("<requirement"),
                "suggestion did not contain `<requirement`: {suggested_element:?}",
            );
        }
        // The Display impl must also surface both pieces.
        let rendered = format!("{err}");
        assert!(
            rendered.contains("speccy:requirement") && rendered.contains("<requirement"),
            "Display should mention legacy form and suggestion: {rendered}",
        );
    }

    #[test]
    fn legacy_html_comment_marker_close_errors_with_dedicated_variant() {
        let src = make(indoc! {r#"
            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            <!-- /speccy:requirement -->

            <changelog>
            row
            </changelog>
        "#});
        let err = parse(&src, path()).expect_err("legacy close marker must fail");
        assert!(
            matches!(err, ParseError::LegacyMarker { .. }),
            "expected LegacyMarker variant, got {err:?}",
        );
        if let ParseError::LegacyMarker {
            ref legacy_form,
            ref suggested_element,
            ..
        } = err
        {
            assert!(legacy_form.contains("/speccy:requirement"));
            assert_eq!(suggested_element, "</requirement>");
        }
    }

    #[test]
    fn legacy_marker_in_inline_prose_is_not_an_error() {
        // Documentation prose that mentions the legacy form inline (for
        // example wrapped in inline backticks and parentheses as part of
        // ordinary Markdown sentence text) must not trip the LegacyMarker
        // diagnostic. The scanner only flags line-isolated legacy markers
        // — same line-isolation rule the raw XML element scanner enforces
        // for new structure tags.
        let src = make(indoc! {r#"
            History note: SPEC-0019 used HTML-comment markers (e.g.
            `<!-- speccy:requirement id="REQ-001" -->`) which SPEC-0020
            replaces with raw XML element tags.

            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.requirements.len(), 1);
    }

    #[test]
    fn legacy_marker_inside_fenced_code_is_not_an_error() {
        // Documentation about the legacy form inside a fenced code
        // block must not trigger the LegacyMarker diagnostic; it is
        // example text, not structure.
        let src = make(indoc! {r#"
            History note: the SPEC-0019 form looked like this:

            ```markdown
            <!-- speccy:requirement id="REQ-XXX" -->
            ```

            <requirement id="REQ-001">
            body

            <done-when>
            - placeholder.
            </done-when>

            <behavior>
            - placeholder.
            </behavior>

            <scenario id="CHK-001">
            text
            </scenario>
            </requirement>

            <changelog>
            row
            </changelog>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.requirements.len(), 1);
    }

    #[test]
    fn canonical_fixture_parses_cleanly() {
        // Sanity check that the checked-in canonical fixture (used by
        // T-002's roundtrip test) is valid against the T-001 parser.
        // If T-002 needs to evolve the fixture shape, this test is the
        // first line of defence against accidental regressions.
        let src = include_str!("../../../tests/fixtures/spec_xml/canonical.md");
        let doc = parse(src, Utf8Path::new("tests/fixtures/spec_xml/canonical.md"))
            .expect("canonical fixture should parse");
        let ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["REQ-001", "REQ-002"]);
        let chk_ids: Vec<&str> = doc
            .requirements
            .iter()
            .flat_map(|r| r.scenarios.iter().map(|s| s.id.as_str()))
            .collect();
        assert_eq!(chk_ids, vec!["CHK-001", "CHK-002", "CHK-003"]);
        assert_eq!(doc.decisions.len(), 1);
        assert_eq!(doc.open_questions.len(), 1);
        assert!(!doc.goals.trim().is_empty());
        assert!(!doc.non_goals.trim().is_empty());
        assert!(!doc.user_stories.trim().is_empty());
        assert!(doc.assumptions.is_some());
        assert!(doc.changelog_body.contains("Initial canonical fixture"));
    }

    #[test]
    fn speccy_whitelist_is_disjoint_from_html5_element_set() {
        // The disjointness invariant from SPEC-0020 REQ-001 / DEC-002
        // and SPEC-0021 REQ-005. Each Speccy structure element name
        // must be absent from the checked-in HTML5 element set;
        // future additions that collide surface as a build-time test
        // failure. The list below pins the post-SPEC-0021 whitelist
        // names so a future edit can extend the same assertion as new
        // tags ship.
        for &name in SPECCY_ELEMENT_NAMES {
            assert!(
                !is_html5_element_name(name),
                "Speccy element `{name}` collides with the HTML5 element name set",
            );
        }
        for expected in [
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
        ] {
            assert!(
                SPECCY_ELEMENT_NAMES.contains(&expected),
                "post-SPEC-0021 whitelist is missing `{expected}`; SPECCY_ELEMENT_NAMES = {SPECCY_ELEMENT_NAMES:?}",
            );
        }
        for retired in ["spec", "overview"] {
            assert!(
                !SPECCY_ELEMENT_NAMES.contains(&retired),
                "SPEC-0021 DEC-008 retired `{retired}`; it must no longer appear in SPECCY_ELEMENT_NAMES",
            );
        }
        // Sanity: ensure the HTML5 list contains the names called out
        // in REQ-001 — if a future edit accidentally drops `summary`,
        // the collision check above silently loses coverage.
        for required in [
            "html", "head", "body", "title", "summary", "details", "section", "table", "tr", "td",
            "script", "style", "template", "svg", "math",
        ] {
            assert!(
                HTML5_ELEMENT_NAMES.contains(&required),
                "HTML5 element set is missing `{required}`",
            );
        }
    }
}
