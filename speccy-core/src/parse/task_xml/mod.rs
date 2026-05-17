//! Raw-XML-element-structured TASKS.md parser and renderer (SPEC-0022
//! REQ-001 / REQ-003 carrier).
//!
//! Reads a TASKS.md whose body is ordinary Markdown plus line-isolated raw
//! XML open/close tag pairs drawn from a small closed whitelist
//! (`tasks`, `task`, `task-scenarios`) and returns a typed [`TasksDoc`].
//! Reuses the shared scanner ([`crate::parse::xml_scanner`]) introduced by
//! T-001, so fenced-code-block awareness and tag-shape diagnostics are
//! identical to SPEC.md parsing.
//!
//! See `.speccy/specs/0022-xml-canonical-tasks-report/SPEC.md` REQ-001
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
use std::collections::HashSet;
use std::sync::OnceLock;

/// Closed whitelist of Speccy structure element names recognised inside
/// TASKS.md.
pub const TASKS_ELEMENT_NAMES: &[&str] = &["tasks", "task", "task-scenarios"];

/// Closed set of valid `<task state="...">` values, in their on-disk form.
pub const ALLOWED_TASK_STATES: &[&str] = &["pending", "in-progress", "in-review", "completed"];

/// Parsed raw-XML-structured TASKS.md.
///
/// `frontmatter_raw` carries the YAML frontmatter payload verbatim; the
/// `tasks` parser does not re-validate it. `heading` is the level-1
/// heading text after `# `, trimmed. `spec_id` is the `spec="..."`
/// attribute on the root `<tasks>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasksDoc {
    /// YAML frontmatter payload between the opening and closing `---`
    /// fences, verbatim.
    pub frontmatter_raw: String,
    /// Text of the level-1 heading after the `# ` prefix, trimmed.
    pub heading: String,
    /// Raw source bytes, retained so [`ElementSpan`] indices remain valid.
    pub raw: String,
    /// `spec="..."` attribute value on the root `<tasks>` element
    /// (e.g. `"SPEC-0022"`).
    pub spec_id: String,
    /// Span of the root `<tasks>` open tag.
    pub tasks_span: ElementSpan,
    /// Tasks declared by `<task>` elements in source order.
    pub tasks: Vec<Task>,
}

/// Closed set of `<task state="...">` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// `pending` — work not yet started.
    Pending,
    /// `in-progress` — work in flight.
    InProgress,
    /// `in-review` — awaiting reviewer sign-off.
    InReview,
    /// `completed` — shipped.
    Completed,
}

impl TaskState {
    /// Render back to the on-disk string form.
    #[must_use = "the rendered state is the on-disk form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            TaskState::Pending => "pending",
            TaskState::InProgress => "in-progress",
            TaskState::InReview => "in-review",
            TaskState::Completed => "completed",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskState::Pending),
            "in-progress" => Some(TaskState::InProgress),
            "in-review" => Some(TaskState::InReview),
            "completed" => Some(TaskState::Completed),
            _ => None,
        }
    }
}

/// One task block (`<task>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Id from the `id="..."` attribute (matches `T-\d{3,}`).
    pub id: String,
    /// `state="..."` attribute value, parsed.
    pub state: TaskState,
    /// `covers="..."` attribute value parsed into a list of `REQ-\d{3,}`
    /// ids in source order.
    pub covers: Vec<String>,
    /// Verbatim body of the required nested `<task-scenarios>` block.
    pub scenarios_body: String,
    /// Span of the `<task-scenarios>` open tag.
    pub scenarios_span: ElementSpan,
    /// Verbatim body between `<task>` and `</task>` open and close tags,
    /// including any `<task-scenarios>` tag lines as literal text. The
    /// renderer strips the nested block out before re-emitting from the
    /// typed model.
    pub body: String,
    /// Span of the `<task>` open tag.
    pub span: ElementSpan,
}

impl Task {
    /// Derive a one-line summary of the task by returning the first
    /// non-empty line of the body. Used by `speccy next` to populate
    /// the `task_line` field of its result, replacing the legacy
    /// "checkbox + bold ID + title" extraction.
    ///
    /// Returns an empty string when the body has no non-blank lines.
    #[must_use = "the title is used as the next-command task line"]
    pub fn title(&self) -> String {
        self.body
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .to_owned()
    }

    /// Extract the `Suggested files:` bullet from the task body, when
    /// present. Returns each file token in source order. Matches lines
    /// of the form `- Suggested files: a.rs, b.rs` (case-insensitive on
    /// the label).
    #[must_use = "the suggested files drive prompt rendering"]
    pub fn suggested_files(&self) -> Vec<String> {
        for line in self.body.lines() {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            else {
                continue;
            };
            // Strip optional leading bold marker `**`.
            let rest = rest.trim_start();
            let label_match = rest
                .strip_prefix("Suggested files:")
                .or_else(|| rest.strip_prefix("**Suggested files**:"))
                .or_else(|| rest.strip_prefix("Suggested files**:"));
            if let Some(after) = label_match {
                return after
                    .split(',')
                    .map(|s| s.trim().trim_matches('`').to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        Vec::new()
    }

    /// Collect every body bullet (top-level `- ` line) as a verbatim
    /// note string, with the leading marker stripped. Used by `speccy
    /// report` to count `Retry:` notes per task.
    #[must_use = "task notes drive retry counting in report rendering"]
    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        for line in self.body.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                out.push(rest.trim().to_owned());
            }
        }
        out
    }

    /// 1-indexed source line of the `<task>` open tag inside the parent
    /// TASKS.md. Computed from `span.start` against `source` so callers
    /// that already hold the raw bytes don't pay for a second parse.
    #[must_use = "the line number is used to extract verbatim task entries"]
    pub fn line_in(&self, source: &str) -> usize {
        let Some(prefix) = source.get(..self.span.start) else {
            return 1;
        };
        prefix
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            .saturating_add(1)
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn task_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^T-\d{3,}$").unwrap())
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

/// Run the shared XML scanner with the TASKS.md whitelist. Centralising
/// this matches `spec_xml::scan_spec_tags` so callers have a single
/// grep target for "what tags does TASKS.md recognise".
fn scan_task_tags(
    source: &str,
    body: &str,
    body_offset: usize,
    path: &Utf8Path,
) -> Result<Vec<RawTag>, ParseError> {
    let code_fence_ranges = collect_code_fence_byte_ranges(source);
    let cfg = ScanConfig {
        whitelist: TASKS_ELEMENT_NAMES,
        structure_shaped_names: TASKS_ELEMENT_NAMES,
        retired_names: &[],
        detect_legacy_markers: false,
    };
    scan_tags(source, body, body_offset, &code_fence_ranges, path, &cfg)
}

/// Parse a raw-XML-structured TASKS.md source string.
///
/// `source` is the file contents; `path` is used only to populate
/// diagnostics — this function does no filesystem IO.
///
/// # Errors
///
/// Returns [`ParseError`] for missing frontmatter or level-1 heading,
/// element-shape problems, unknown element names or attributes,
/// id-pattern violations, duplicate task ids, invalid task states,
/// invalid `covers` formats, or missing required nested
/// `<task-scenarios>` blocks.
#[expect(
    clippy::too_many_lines,
    reason = "single-pass TASKS.md validator; splitting body offset bookkeeping and root-element classification across helpers would obscure the linear flow"
)]
pub fn parse(source: &str, path: &Utf8Path) -> Result<TasksDoc, ParseError> {
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
                context: format!("TASKS.md at {path}"),
            });
        }
    };

    let heading = extract_level1_heading(body, path)?;

    let raw_tags = scan_task_tags(source, body, body_offset, path)?;

    // Up-front shape validation so unknown attributes / id-pattern
    // violations fail before we try to assemble nested blocks.
    for t in &raw_tags {
        validate_tag_shape(t, path)?;
    }

    let tree = assemble(raw_tags, source, path)?;

    // The TASKS.md root contract: exactly one `<tasks spec="...">`
    // element wrapping zero or more `<task>` children. Free top-level
    // Markdown (heading, phase prose) is allowed alongside `<tasks>`,
    // but no other speccy structure is allowed at the top level.
    let mut root: Option<(String, ElementSpan, Vec<Block>)> = None;
    for block in tree {
        match block {
            Block::Tasks {
                spec_id,
                span,
                children,
            } => {
                if root.is_some() {
                    return Err(ParseError::MalformedMarker {
                        path: path.to_path_buf(),
                        offset: span.start,
                        reason: "more than one <tasks> root element".to_owned(),
                    });
                }
                root = Some((spec_id, span, children));
            }
            Block::Task { span, .. } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<task> element must be nested inside <tasks>".to_owned(),
                });
            }
            Block::TaskScenarios { span, .. } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<task-scenarios> element must be nested inside <task>".to_owned(),
                });
            }
        }
    }

    let (spec_id, tasks_span, children) = root.ok_or_else(|| ParseError::MissingField {
        field: "<tasks>".to_owned(),
        context: format!("TASKS.md at {path}"),
    })?;

    let mut tasks: Vec<Task> = Vec::new();
    let mut task_ids: HashSet<String> = HashSet::new();
    for child in children {
        match child {
            Block::Task {
                id,
                attrs,
                body,
                children: task_children,
                span,
            } => {
                if !task_ids.insert(id.clone()) {
                    return Err(ParseError::DuplicateMarkerId {
                        path: path.to_path_buf(),
                        marker_name: "task".to_owned(),
                        id,
                    });
                }
                let task = build_task(id, &attrs, body, task_children, span, path)?;
                tasks.push(task);
            }
            Block::TaskScenarios { span, .. } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<task-scenarios> element must be nested inside <task>".to_owned(),
                });
            }
            Block::Tasks { span, .. } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<tasks> element must not be nested".to_owned(),
                });
            }
        }
    }

    Ok(TasksDoc {
        frontmatter_raw,
        heading,
        raw: source.to_owned(),
        spec_id,
        tasks_span,
        tasks,
    })
}

fn build_task(
    id: String,
    attrs: &[(String, String)],
    body: String,
    children: Vec<Block>,
    span: ElementSpan,
    path: &Utf8Path,
) -> Result<Task, ParseError> {
    // State.
    let state_raw = find_attr(attrs, "state").ok_or_else(|| ParseError::MissingTaskAttribute {
        path: path.to_path_buf(),
        task_id: id.clone(),
        attribute: "state".to_owned(),
    })?;
    let state = TaskState::from_str(&state_raw).ok_or_else(|| ParseError::InvalidTaskState {
        path: path.to_path_buf(),
        task_id: id.clone(),
        value: state_raw.clone(),
        allowed: ALLOWED_TASK_STATES.join(", "),
    })?;

    // Covers.
    let covers_raw =
        find_attr(attrs, "covers").ok_or_else(|| ParseError::MissingTaskAttribute {
            path: path.to_path_buf(),
            task_id: id.clone(),
            attribute: "covers".to_owned(),
        })?;
    let covers = parse_covers(&covers_raw, &id, path)?;

    // task-scenarios: exactly one.
    let mut scenarios: Option<(String, ElementSpan)> = None;
    for child in children {
        match child {
            Block::TaskScenarios {
                body: child_body,
                span: child_span,
            } => {
                if scenarios.is_some() {
                    return Err(ParseError::DuplicateTaskSection {
                        path: path.to_path_buf(),
                        task_id: id.clone(),
                        element_name: "task-scenarios".to_owned(),
                        offset: child_span.start,
                    });
                }
                if child_body.trim().is_empty() {
                    return Err(ParseError::EmptyMarkerBody {
                        path: path.to_path_buf(),
                        marker_name: "task-scenarios".to_owned(),
                        id: Some(id.clone()),
                        offset: child_span.start,
                    });
                }
                scenarios = Some((child_body, child_span));
            }
            Block::Task {
                span: child_span, ..
            }
            | Block::Tasks {
                span: child_span, ..
            } => {
                return Err(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: child_span.start,
                    reason: format!(
                        "element nested inside `<task id=\"{id}\">` is not allowed here"
                    ),
                });
            }
        }
    }

    let (scenarios_body, scenarios_span) =
        scenarios.ok_or_else(|| ParseError::MissingTaskSection {
            path: path.to_path_buf(),
            task_id: id.clone(),
            element_name: "task-scenarios".to_owned(),
        })?;

    Ok(Task {
        id,
        state,
        covers,
        scenarios_body,
        scenarios_span,
        body,
        span,
    })
}

fn find_attr(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Parse a `covers="..."` value into a list of `REQ-NNN` ids.
///
/// Grammar (SPEC-0022 REQ-001): one or more `REQ-\d{3,}` ids separated
/// by single ASCII spaces. The empty string, leading or trailing
/// whitespace, double spaces, tabs, and any non-`REQ-\d{3,}` token all
/// fail with [`ParseError::InvalidCoversFormat`], whose Display
/// quotes the grammar verbatim.
fn parse_covers(raw: &str, task_id: &str, path: &Utf8Path) -> Result<Vec<String>, ParseError> {
    if raw.is_empty() {
        return Err(ParseError::InvalidCoversFormat {
            path: path.to_path_buf(),
            task_id: task_id.to_owned(),
            value: raw.to_owned(),
        });
    }
    // Reject any non-` ` ASCII whitespace and any non-ASCII bytes up
    // front; the grammar requires single ASCII spaces only.
    for ch in raw.chars() {
        if ch == '\t' || ch == '\r' || ch == '\n' {
            return Err(ParseError::InvalidCoversFormat {
                path: path.to_path_buf(),
                task_id: task_id.to_owned(),
                value: raw.to_owned(),
            });
        }
    }
    // Split on single ASCII space. We use `split(' ')` rather than
    // `split_whitespace` so a double space surfaces an empty token
    // and trips the regex below.
    let mut covers: Vec<String> = Vec::new();
    for token in raw.split(' ') {
        if !req_id_regex().is_match(token) {
            return Err(ParseError::InvalidCoversFormat {
                path: path.to_path_buf(),
                task_id: task_id.to_owned(),
                value: raw.to_owned(),
            });
        }
        covers.push(token.to_owned());
    }
    Ok(covers)
}

/// Render a [`TasksDoc`] to its canonical raw-XML TASKS.md form.
///
/// The output is a Markdown document with raw XML element tags carrying
/// Speccy structure:
///
/// 1. Frontmatter fence followed by [`TasksDoc::frontmatter_raw`].
/// 2. A blank line, then the level-1 heading (`# {heading}`).
/// 3. The root `<tasks spec="...">` block wrapping every task in
///    [`TasksDoc::tasks`] order. Each task emits its body prose with the nested
///    `<task-scenarios>` block stripped out, then re-emits `<task-scenarios>`
///    from typed state in canonical position.
///
/// `render(doc) == render(doc)` byte-for-byte for any valid `doc`.
/// Free Markdown prose between `<task>` blocks (phase headings, notes,
/// implementer-note bullets) is **not** preserved: the renderer projects
/// only the typed model, mirroring SPEC-0020's `render_spec_xml`
/// canonical-not-lossless contract.
#[must_use = "the rendered Markdown string is the canonical projection of the TasksDoc"]
pub fn render(doc: &TasksDoc) -> String {
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

    push_element_open(&mut out, "tasks", &[("spec", doc.spec_id.as_str())]);
    out.push('\n');
    for (idx, task) in doc.tasks.iter().enumerate() {
        if idx > 0 {
            // Blank line between tasks. The element-close blank-line rule
            // (one blank line after every close tag) keeps the inner
            // `</task-scenarios>` and `</task>` separated; here we just
            // need a separator between successive `<task>` blocks.
        }
        let covers_value = task.covers.join(" ");
        let attrs: [(&str, &str); 3] = [
            ("id", task.id.as_str()),
            ("state", task.state.as_str()),
            ("covers", covers_value.as_str()),
        ];
        push_element_open(&mut out, "task", &attrs);
        let prose = strip_nested_task_scenarios(&task.body);
        push_body(&mut out, &prose);
        push_element_block(&mut out, "task-scenarios", &[], &task.scenarios_body);
        push_element_close(&mut out, "task");
    }
    push_element_close(&mut out, "tasks");

    out
}

fn strip_nested_task_scenarios(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_block = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if in_block {
            if trimmed.starts_with("</task-scenarios>") {
                in_block = false;
            }
            continue;
        }
        if trimmed.starts_with("<task-scenarios>") {
            in_block = true;
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
    // Match `spec_xml`'s determinism contract: every close tag is
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
        context: format!("TASKS.md at {path}"),
    })
}

#[derive(Debug)]
enum Block {
    Tasks {
        spec_id: String,
        span: ElementSpan,
        children: Vec<Block>,
    },
    Task {
        id: String,
        attrs: Vec<(String, String)>,
        body: String,
        children: Vec<Block>,
        span: ElementSpan,
    },
    TaskScenarios {
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
    if !TASKS_ELEMENT_NAMES.contains(&t.name.as_str()) {
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
        "tasks" => &["spec"],
        "task" => &["id", "state", "covers"],
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
        validate_attribute_value(&t.name, k, v, path, t.span.start)?;
    }
    Ok(())
}

fn validate_attribute_value(
    element_name: &str,
    attr: &str,
    value: &str,
    path: &Utf8Path,
    offset: usize,
) -> Result<(), ParseError> {
    match (element_name, attr) {
        ("tasks", "spec") if !spec_id_regex().is_match(value) => Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: element_name.to_owned(),
            id: value.to_owned(),
            expected_pattern: r"SPEC-\d{3,}".to_owned(),
        }),
        ("task", "id") if !task_id_regex().is_match(value) => Err(ParseError::InvalidMarkerId {
            path: path.to_path_buf(),
            marker_name: element_name.to_owned(),
            id: value.to_owned(),
            expected_pattern: r"T-\d{3,}".to_owned(),
        }),
        // `state` and `covers` values are validated later, when the
        // task body assembles, because the diagnostics name the task
        // id rather than the raw attribute offset.
        _ => {
            let _ = offset;
            Ok(())
        }
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
        match name.as_str() {
            "tasks" => {
                let spec_id =
                    find_attr(&attrs, "spec").ok_or_else(|| ParseError::MissingField {
                        field: "spec".to_owned(),
                        context: format!("<tasks> element in {path}"),
                    })?;
                Ok(Block::Tasks {
                    spec_id,
                    span,
                    children,
                })
            }
            "task" => {
                let id = find_attr(&attrs, "id").ok_or_else(|| ParseError::MissingField {
                    field: "id".to_owned(),
                    context: format!("<task> element in {path}"),
                })?;
                Ok(Block::Task {
                    id,
                    attrs,
                    body,
                    children,
                    span,
                })
            }
            "task-scenarios" => Ok(Block::TaskScenarios { body, span }),
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
    use super::ALLOWED_TASK_STATES;
    use super::TASKS_ELEMENT_NAMES;
    use super::TaskState;
    use super::parse;
    use crate::error::ParseError;
    use crate::parse::xml_scanner::HTML5_ELEMENT_NAMES;
    use camino::Utf8Path;
    use indoc::indoc;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/TASKS.md")
    }

    fn frontmatter() -> &'static str {
        "---\nspec: SPEC-0022\n---\n\n# Tasks: SPEC-0022\n\n"
    }

    fn make(body: &str) -> String {
        format!("{}{}", frontmatter(), body)
    }

    #[test]
    fn happy_path_two_tasks() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending" covers="REQ-001">
            Task one prose.

            <task-scenarios>
            Given X, when Y, then Z (T-001).
            </task-scenarios>
            </task>

            <task id="T-002" state="in-progress" covers="REQ-001 REQ-002">
            Task two prose.

            <task-scenarios>
            Given A, when B, then C (T-002).
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.spec_id, "SPEC-0022");
        assert_eq!(doc.tasks.len(), 2);
        let t1 = doc.tasks.first().expect("two tasks");
        assert_eq!(t1.id, "T-001");
        assert_eq!(t1.state, TaskState::Pending);
        assert_eq!(t1.covers, vec!["REQ-001".to_owned()]);
        assert!(t1.scenarios_body.contains("(T-001)"));
        let t2 = doc.tasks.get(1).expect("two tasks");
        assert_eq!(t2.id, "T-002");
        assert_eq!(t2.state, TaskState::InProgress);
        assert_eq!(t2.covers, vec!["REQ-001".to_owned(), "REQ-002".to_owned()]);
        assert!(t2.scenarios_body.contains("(T-002)"));
    }

    #[test]
    fn invalid_state_names_id_value_and_valid_states() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="done" covers="REQ-001">
            <task-scenarios>
            text.
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("bad state must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                &err,
                ParseError::InvalidTaskState { task_id, value, allowed, .. }
                    if task_id == "T-001"
                        && value == "done"
                        && allowed == "pending, in-progress, in-review, completed"
            ),
            "got: {err:?}",
        );
        for state in ALLOWED_TASK_STATES {
            assert!(
                msg.contains(state),
                "msg `{msg}` missing valid state `{state}`"
            );
        }
        assert!(msg.contains("T-001"), "msg `{msg}` missing task id");
        assert!(msg.contains("done"), "msg `{msg}` missing rejected value");
    }

    #[test]
    fn zero_task_scenarios_errors_names_task() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending" covers="REQ-001">
            no scenarios.
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("missing task-scenarios must fail");
        assert!(
            matches!(
                &err,
                ParseError::MissingTaskSection { task_id, element_name, .. }
                    if task_id == "T-001" && element_name == "task-scenarios"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn duplicate_task_scenarios_errors() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending" covers="REQ-001">
            <task-scenarios>
            first.
            </task-scenarios>

            <task-scenarios>
            second.
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("duplicate task-scenarios must fail");
        assert!(
            matches!(
                &err,
                ParseError::DuplicateTaskSection { task_id, element_name, .. }
                    if task_id == "T-001" && element_name == "task-scenarios"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn missing_covers_names_task() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending">
            <task-scenarios>
            text.
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("missing covers must fail");
        assert!(
            matches!(
                &err,
                ParseError::MissingTaskAttribute { task_id, attribute, .. }
                    if task_id == "T-001" && attribute == "covers"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn double_space_covers_quotes_grammar() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending" covers="REQ-001  REQ-002">
            <task-scenarios>
            text.
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("double-space covers must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                &err,
                ParseError::InvalidCoversFormat { task_id, value, .. }
                    if task_id == "T-001" && value == "REQ-001  REQ-002"
            ),
            "got: {err:?}",
        );
        assert!(
            msg.contains("single ASCII space separated `REQ-\\d{3,}` ids"),
            "msg `{msg}` must quote the SPEC-0022 grammar verbatim",
        );
    }

    #[test]
    fn tab_covers_quotes_grammar() {
        // A tab between REQ ids should trip the same diagnostic.
        let raw = "REQ-001\tREQ-002";
        let src = make(&format!(
            "<tasks spec=\"SPEC-0022\">\n\n<task id=\"T-001\" state=\"pending\" covers=\"{raw}\">\n<task-scenarios>\ntext.\n</task-scenarios>\n</task>\n\n</tasks>\n",
        ));
        let err = parse(&src, path()).expect_err("tab in covers must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                &err,
                ParseError::InvalidCoversFormat { task_id, .. } if task_id == "T-001"
            ),
            "got: {err:?}",
        );
        assert!(
            msg.contains("single ASCII space separated `REQ-\\d{3,}` ids"),
            "msg `{msg}` must quote the SPEC-0022 grammar verbatim",
        );
    }

    #[test]
    fn duplicate_task_id_errors() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending" covers="REQ-001">
            <task-scenarios>
            a.
            </task-scenarios>
            </task>

            <task id="T-001" state="pending" covers="REQ-001">
            <task-scenarios>
            b.
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("duplicate task id must fail");
        assert!(
            matches!(
                &err,
                ParseError::DuplicateMarkerId { marker_name, id, .. }
                    if marker_name == "task" && id == "T-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn unknown_attribute_on_task_lists_valid_set() {
        let src = make(indoc! {r#"
            <tasks spec="SPEC-0022">

            <task id="T-001" state="pending" covers="REQ-001" priority="high">
            <task-scenarios>
            text.
            </task-scenarios>
            </task>

            </tasks>
        "#});
        let err = parse(&src, path()).expect_err("unknown attr must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                &err,
                ParseError::UnknownMarkerAttribute {
                    marker_name, attribute, allowed, ..
                } if marker_name == "task"
                    && attribute == "priority"
                    && allowed == "id, state, covers"
            ),
            "got: {err:?}",
        );
        assert!(
            msg.contains("id, state, covers"),
            "msg `{msg}` missing valid set"
        );
    }

    #[test]
    fn tasks_element_names_disjoint_from_html5() {
        for name in TASKS_ELEMENT_NAMES {
            assert!(
                !HTML5_ELEMENT_NAMES.contains(name),
                "TASKS element `{name}` collides with HTML5 element name set",
            );
        }
    }
}
