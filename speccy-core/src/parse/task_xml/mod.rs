//! Raw-XML-element-structured TASKS.md parser and renderer.
//!
//! TASKS.md is a YAML frontmatter, a level-1 `# Tasks: ...` heading, and
//! a sequence of bare `<task>` children. The closed element set is
//! `task`, `task-scenarios`, `implementer`, `review`, `blockers`; the
//! latter three are accepted as `RawTag`s so the lint engine can flag
//! them as misplaced (their canonical home is `journal/T-NNN.md`), but
//! they do not assemble into typed `Task` body fields.
//!
//! The `spec` binding for a TASKS.md comes from three sources, all
//! redundant by design: the parent directory name (`NNNN-slug`), the
//! YAML frontmatter `spec:` field, and the parent SPEC.md frontmatter
//! `id:` field. TSK-005 catches disagreement.

use crate::error::ParseError;
use crate::error::ParseResult;
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
///
/// Five names: `task` and `task-scenarios` are the structural carriers.
/// `implementer`, `review`, and `blockers` are accepted so the
/// per-element scanner sees them and downstream lint rules
/// (TSK-006 "no notes elements in TASKS.md") can flag them as misplaced
/// — their canonical destination is `journal/T-NNN.md`.
pub const TASKS_ELEMENT_NAMES: &[&str] = &[
    "task",
    "task-scenarios",
    "implementer",
    "review",
    "blockers",
];

/// Names of activity-prose elements that, when they appear inside a
/// TASKS.md, fire TSK-006. Their canonical home is `journal/T-NNN.md`.
pub const MISPLACED_JOURNAL_ELEMENT_NAMES: &[&str] = &["implementer", "review", "blockers"];

/// Closed set of valid `<task state="...">` values, in their on-disk form.
pub const ALLOWED_TASK_STATES: &[&str] = &["pending", "in-progress", "in-review", "completed"];

/// Parsed raw-XML-structured TASKS.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasksDoc {
    /// YAML frontmatter payload between the opening and closing `---`
    /// fences, verbatim.
    pub frontmatter_raw: String,
    /// Text of the level-1 heading after the `# ` prefix, trimmed.
    pub heading: String,
    /// Raw source bytes, retained so [`ElementSpan`] indices remain valid.
    pub raw: String,
    /// Tasks declared by `<task>` elements in source order.
    pub tasks: Vec<Task>,
    /// Source-ordered record of every `<implementer>`, `<review>`, or
    /// `<blockers>` element observed inside any `<task>` body. Drives the
    /// TSK-006 "no notes elements in TASKS.md" lint. Empty in a
    /// well-formed TASKS.md after SPEC-0037.
    pub misplaced_journal_elements: Vec<MisplacedJournalElement>,
}

/// One activity-prose element observed inside a TASKS.md `<task>` body.
/// SPEC-0037 REQ-006: these should live in `journal/T-NNN.md` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MisplacedJournalElement {
    /// Element local name (`implementer`, `review`, or `blockers`).
    pub element_name: String,
    /// Id of the enclosing `<task>` element.
    pub task_id: String,
    /// Span of the misplaced element's open tag.
    pub span: ElementSpan,
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
    /// Verbatim body between `<task>` and `</task>` open and close tags.
    pub body: String,
    /// Span of the `<task>` open tag.
    pub span: ElementSpan,
}

impl Task {
    /// Derive a one-line summary of the task by returning the first
    /// non-empty line of the body.
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
    /// present.
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

    /// 1-indexed source line of the `<task>` open tag inside the parent
    /// TASKS.md.
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
fn req_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^REQ-\d{3,}$").unwrap())
}

fn scan_task_tags(
    source: &str,
    body: &str,
    body_offset: usize,
    path: &Utf8Path,
) -> ParseResult<Vec<RawTag>> {
    let code_fence_ranges = collect_code_fence_byte_ranges(source);
    let cfg = ScanConfig {
        whitelist: TASKS_ELEMENT_NAMES,
        structure_shaped_names: TASKS_ELEMENT_NAMES,
    };
    scan_tags(source, body, body_offset, &code_fence_ranges, path, &cfg)
}

/// Parse a raw-XML-structured TASKS.md source string.
///
/// # Errors
///
/// Returns [`ParseError`] for missing frontmatter or level-1 heading,
/// element-shape problems, unknown element names or attributes,
/// id-pattern violations, duplicate task ids, invalid task states,
/// invalid `covers` formats, or missing required nested
/// `<task-scenarios>` blocks.
pub fn parse(source: &str, path: &Utf8Path) -> ParseResult<TasksDoc> {
    let split = split_frontmatter(source, path)?;
    let (frontmatter_raw, body, body_offset) = match split {
        Split::Some { yaml, body } => {
            let body_offset = source.len().checked_sub(body.len()).ok_or_else(|| {
                Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: 0,
                    reason: "frontmatter splitter produced an inconsistent body offset".to_owned(),
                })
            })?;
            (yaml.to_owned(), body, body_offset)
        }
        Split::None => {
            return Err(Box::new(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("TASKS.md at {path}"),
            }));
        }
    };

    let heading = extract_level1_heading(body, path)?;

    let raw_tags = scan_task_tags(source, body, body_offset, path)?;

    for t in &raw_tags {
        validate_tag_shape(t, path)?;
    }

    let tree = assemble(raw_tags, source, path)?;

    let mut tasks: Vec<Task> = Vec::new();
    let mut task_ids: HashSet<String> = HashSet::new();
    let mut misplaced: Vec<MisplacedJournalElement> = Vec::new();

    for block in tree {
        match block {
            Block::Task {
                id,
                attrs,
                body,
                children,
                span,
            } => {
                if !task_ids.insert(id.clone()) {
                    return Err(Box::new(ParseError::DuplicateMarkerId {
                        path: path.to_path_buf(),
                        marker_name: "task".to_owned(),
                        id,
                    }));
                }
                let task = build_task(id, &attrs, body, children, span, path, &mut misplaced)?;
                tasks.push(task);
            }
            Block::TaskScenarios { span, .. } => {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: "<task-scenarios> element must be nested inside <task>".to_owned(),
                }));
            }
            Block::JournalLike { name, span, .. } => {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: span.start,
                    reason: format!(
                        "<{name}> element must be nested inside <task> (canonical home: journal/T-NNN.md)"
                    ),
                }));
            }
        }
    }

    Ok(TasksDoc {
        frontmatter_raw,
        heading,
        raw: source.to_owned(),
        tasks,
        misplaced_journal_elements: misplaced,
    })
}

fn build_task(
    id: String,
    attrs: &[(String, String)],
    body: String,
    children: Vec<Block>,
    span: ElementSpan,
    path: &Utf8Path,
    misplaced: &mut Vec<MisplacedJournalElement>,
) -> ParseResult<Task> {
    let state_raw = find_attr(attrs, "state").ok_or_else(|| {
        Box::new(ParseError::MissingTaskAttribute {
            path: path.to_path_buf(),
            task_id: id.clone(),
            attribute: "state".to_owned(),
        })
    })?;
    let state = TaskState::from_str(&state_raw).ok_or_else(|| {
        Box::new(ParseError::InvalidTaskState {
            path: path.to_path_buf(),
            task_id: id.clone(),
            value: state_raw.clone(),
            allowed: ALLOWED_TASK_STATES.join(", "),
        })
    })?;

    let covers_raw = find_attr(attrs, "covers").ok_or_else(|| {
        Box::new(ParseError::MissingTaskAttribute {
            path: path.to_path_buf(),
            task_id: id.clone(),
            attribute: "covers".to_owned(),
        })
    })?;
    let covers = parse_covers(&covers_raw, &id, path)?;

    let (scenarios_body, scenarios_span) = collect_task_children(&id, children, path, misplaced)?;

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

fn collect_task_children(
    task_id: &str,
    children: Vec<Block>,
    path: &Utf8Path,
    misplaced: &mut Vec<MisplacedJournalElement>,
) -> ParseResult<(String, ElementSpan)> {
    let mut scenarios: Option<(String, ElementSpan)> = None;
    for child in children {
        match child {
            Block::TaskScenarios {
                body: child_body,
                span: child_span,
            } => {
                if scenarios.is_some() {
                    return Err(Box::new(ParseError::DuplicateTaskSection {
                        path: path.to_path_buf(),
                        task_id: task_id.to_owned(),
                        element_name: "task-scenarios".to_owned(),
                        offset: child_span.start,
                    }));
                }
                if child_body.trim().is_empty() {
                    return Err(Box::new(ParseError::EmptyMarkerBody {
                        path: path.to_path_buf(),
                        marker_name: "task-scenarios".to_owned(),
                        id: Some(task_id.to_owned()),
                        offset: child_span.start,
                    }));
                }
                scenarios = Some((child_body, child_span));
            }
            Block::JournalLike {
                name,
                span: child_span,
                ..
            } => {
                misplaced.push(MisplacedJournalElement {
                    element_name: name,
                    task_id: task_id.to_owned(),
                    span: child_span,
                });
            }
            Block::Task {
                span: child_span, ..
            } => {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: child_span.start,
                    reason: format!(
                        "<task> element nested inside `<task id=\"{task_id}\">` is not allowed"
                    ),
                }));
            }
        }
    }
    let (scenarios_body, scenarios_span) = scenarios.ok_or_else(|| {
        Box::new(ParseError::MissingTaskSection {
            path: path.to_path_buf(),
            task_id: task_id.to_owned(),
            element_name: "task-scenarios".to_owned(),
        })
    })?;
    Ok((scenarios_body, scenarios_span))
}

fn find_attr(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn parse_covers(raw: &str, task_id: &str, path: &Utf8Path) -> ParseResult<Vec<String>> {
    if raw.is_empty() {
        return Err(Box::new(ParseError::InvalidCoversFormat {
            path: path.to_path_buf(),
            task_id: task_id.to_owned(),
            value: raw.to_owned(),
        }));
    }
    for ch in raw.chars() {
        if ch == '\t' || ch == '\r' || ch == '\n' {
            return Err(Box::new(ParseError::InvalidCoversFormat {
                path: path.to_path_buf(),
                task_id: task_id.to_owned(),
                value: raw.to_owned(),
            }));
        }
    }
    let mut covers: Vec<String> = Vec::new();
    for token in raw.split(' ') {
        if !req_id_regex().is_match(token) {
            return Err(Box::new(ParseError::InvalidCoversFormat {
                path: path.to_path_buf(),
                task_id: task_id.to_owned(),
                value: raw.to_owned(),
            }));
        }
        covers.push(token.to_owned());
    }
    Ok(covers)
}

/// Render a [`TasksDoc`] back to its canonical Markdown form.
///
/// Output: frontmatter, blank line, level-1 heading, blank line, then
/// bare `<task>` children separated by a blank line each.
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

    for task in &doc.tasks {
        let covers_value = task.covers.join(" ");
        let attrs: [(&str, &str); 3] = [
            ("id", task.id.as_str()),
            ("state", task.state.as_str()),
            ("covers", covers_value.as_str()),
        ];
        push_element_open(&mut out, "task", &attrs);
        let prose = strip_nested_body_blocks(&task.body);
        push_body(&mut out, &prose);
        push_element_block(&mut out, "task-scenarios", &[], &task.scenarios_body);
        push_element_close(&mut out, "task");
    }

    out
}

fn strip_nested_body_blocks(body: &str) -> String {
    const STRIPPED: &[&str] = &["task-scenarios", "implementer", "review", "blockers"];
    let mut out = String::with_capacity(body.len());
    let mut in_block: Option<&'static str> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(name) = in_block {
            let close = format!("</{name}>");
            if trimmed.starts_with(close.as_str()) {
                in_block = None;
            }
            continue;
        }
        let mut matched: Option<&'static str> = None;
        for name in STRIPPED {
            let open_prefix_attr = format!("<{name} ");
            let open_prefix_close = format!("<{name}>");
            if trimmed.starts_with(open_prefix_attr.as_str())
                || trimmed.starts_with(open_prefix_close.as_str())
            {
                matched = Some(name);
                break;
            }
        }
        if let Some(name) = matched {
            in_block = Some(name);
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

fn extract_level1_heading(body: &str, path: &Utf8Path) -> ParseResult<String> {
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return Ok(rest.trim().to_owned());
        }
        if trimmed == "#" {
            return Ok(String::new());
        }
    }
    Err(Box::new(ParseError::MissingField {
        field: "level-1 heading".to_owned(),
        context: format!("TASKS.md at {path}"),
    }))
}

#[derive(Debug)]
enum Block {
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
    /// An `<implementer>`, `<review>`, or `<blockers>` element. These
    /// are recognised so TSK-006 can lint them; they assemble into
    /// `MisplacedJournalElement` records, not into a typed body field.
    JournalLike {
        name: String,
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
    if !TASKS_ELEMENT_NAMES.contains(&t.name.as_str()) {
        return Err(Box::new(ParseError::UnknownMarkerName {
            path: path.to_path_buf(),
            marker_name: t.name.clone(),
            offset: t.span.start,
        }));
    }
    if t.is_close {
        return Ok(());
    }
    // For misplaced journal-like elements inside TASKS.md, accept any
    // attributes here — TSK-006 will report the misplacement and lint
    // the journal-file schema in journal_xml.
    let allowed_attrs: &[&str] = match t.name.as_str() {
        "task" => &["id", "state", "covers"],
        "implementer" => &["date", "model", "round"],
        "review" => &["date", "model", "persona", "verdict", "round"],
        "blockers" => &["date", "round"],
        // `task-scenarios` is attribute-free.
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
) -> ParseResult<()> {
    match (element_name, attr) {
        ("task", "id") if !task_id_regex().is_match(value) => {
            Err(Box::new(ParseError::InvalidMarkerId {
                path: path.to_path_buf(),
                marker_name: element_name.to_owned(),
                id: value.to_owned(),
                expected_pattern: r"T-\d{3,}".to_owned(),
            }))
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
    fn finish(self, body: String, path: &Utf8Path) -> ParseResult<Block> {
        let PendingBlock {
            name,
            attrs,
            span,
            body_start: _,
            children,
        } = self;
        match name.as_str() {
            "task" => {
                let id = find_attr(&attrs, "id").ok_or_else(|| {
                    Box::new(ParseError::MissingField {
                        field: "id".to_owned(),
                        context: format!("<task> element in {path}"),
                    })
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
            "implementer" | "review" | "blockers" => Ok(Block::JournalLike { name, span }),
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
        "---\nspec: SPEC-0037\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n# Tasks: SPEC-0037\n\n"
    }

    fn make(body: &str) -> String {
        format!("{}{}", frontmatter(), body)
    }

    #[test]
    fn happy_path_two_bare_tasks_no_wrapper() {
        let src = make(indoc! {r#"
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
        "#});
        let doc = parse(&src, path()).expect("parse should succeed");
        assert_eq!(doc.tasks.len(), 2);
        let t1 = doc.tasks.first().expect("two tasks");
        assert_eq!(t1.id, "T-001");
        assert_eq!(t1.state, TaskState::Pending);
        assert_eq!(t1.covers, vec!["REQ-001".to_owned()]);
        assert!(t1.scenarios_body.contains("(T-001)"));
        let t2 = doc.tasks.get(1).expect("two tasks");
        assert_eq!(t2.id, "T-002");
        assert_eq!(t2.state, TaskState::InProgress);
        assert!(doc.misplaced_journal_elements.is_empty());
    }

    #[test]
    fn misplaced_implementer_in_tasks_md_is_recorded() {
        let src = make(indoc! {r#"
            <task id="T-001" state="completed" covers="REQ-001">
            <task-scenarios>
            text.
            </task-scenarios>
            <implementer date="2026-05-21T18:00:00Z" model="claude" round="1">
            body
            </implementer>
            </task>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed despite misplaced element");
        assert_eq!(doc.misplaced_journal_elements.len(), 1);
        let m = doc
            .misplaced_journal_elements
            .first()
            .expect("one misplaced");
        assert_eq!(m.element_name, "implementer");
        assert_eq!(m.task_id, "T-001");
    }

    #[test]
    fn misplaced_blockers_in_tasks_md_is_recorded() {
        let src = make(indoc! {r#"
            <task id="T-003" state="pending" covers="REQ-001">
            <task-scenarios>
            text.
            </task-scenarios>
            <blockers date="2026-05-21T18:00:00Z" round="1">
            body
            </blockers>
            </task>
        "#});
        let doc = parse(&src, path()).expect("parse should succeed despite misplaced element");
        assert_eq!(doc.misplaced_journal_elements.len(), 1);
        let m = doc
            .misplaced_journal_elements
            .first()
            .expect("one misplaced");
        assert_eq!(m.element_name, "blockers");
    }

    #[test]
    fn invalid_state_names_id_value_and_valid_states() {
        let src = make(indoc! {r#"
            <task id="T-001" state="done" covers="REQ-001">
            <task-scenarios>
            text.
            </task-scenarios>
            </task>
        "#});
        let err = parse(&src, path()).expect_err("bad state must fail");
        let msg = format!("{err}");
        assert!(
            matches!(
                err.as_ref(),
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
    }

    #[test]
    fn zero_task_scenarios_errors_names_task() {
        let src = make(indoc! {r#"
            <task id="T-001" state="pending" covers="REQ-001">
            no scenarios.
            </task>
        "#});
        let err = parse(&src, path()).expect_err("missing task-scenarios must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::MissingTaskSection { task_id, element_name, .. }
                    if task_id == "T-001" && element_name == "task-scenarios"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn duplicate_task_id_errors() {
        let src = make(indoc! {r#"
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
        "#});
        let err = parse(&src, path()).expect_err("duplicate task id must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::DuplicateMarkerId { marker_name, id, .. }
                    if marker_name == "task" && id == "T-001"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn unknown_attribute_on_task_lists_valid_set() {
        let src = make(indoc! {r#"
            <task id="T-001" state="pending" covers="REQ-001" priority="high">
            <task-scenarios>
            text.
            </task-scenarios>
            </task>
        "#});
        let err = parse(&src, path()).expect_err("unknown attr must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::UnknownMarkerAttribute {
                    marker_name, attribute, allowed, ..
                } if marker_name == "task"
                    && attribute == "priority"
                    && allowed == "id, state, covers"
            ),
            "got: {err:?}",
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
