//! Raw-XML-element-structured TASKS.md parser.
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
    /// well-formed TASKS.md.
    pub misplaced_journal_elements: Vec<MisplacedJournalElement>,
}

/// One activity-prose element observed inside a TASKS.md `<task>` body.
/// These should live in `journal/T-NNN.md` instead.
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

    /// Parse a state from its on-disk string form.
    ///
    /// Returns `None` for any string outside [`ALLOWED_TASK_STATES`].
    #[must_use = "the parsed state must be inspected"]
    pub fn parse(s: &str) -> Option<Self> {
        Self::from_str(s)
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

/// The closed set of legal state-graph edges.
///
/// Each pair is `(from, to)`. Same-state edges are not listed here; they
/// are handled separately as idempotent no-ops.
pub const LEGAL_TRANSITION_EDGES: &[(TaskState, TaskState)] = &[
    (TaskState::Pending, TaskState::InProgress),
    (TaskState::InProgress, TaskState::InReview),
    (TaskState::InReview, TaskState::Completed),
    (TaskState::InReview, TaskState::Pending),
    (TaskState::InProgress, TaskState::Pending),
    (TaskState::Completed, TaskState::Pending),
];

/// Outcome of classifying a requested `(from, to)` transition against the
/// legal state graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    /// `to` differs from `from` and the edge is in [`LEGAL_TRANSITION_EDGES`].
    /// The caller should splice the new state and write.
    Legal,
    /// `to` equals `from`: an idempotent no-op that exits 0 and leaves the
    /// file byte-identical.
    NoOp,
    /// `to` differs from `from` and the edge is not in the legal graph.
    /// The caller should refuse and exit non-zero.
    Illegal,
}

/// Classify a requested `from -> to` transition against the legal state
/// graph.
///
/// A target equal to the current state is a [`TransitionKind::NoOp`].
/// Otherwise the edge is [`TransitionKind::Legal`] iff it
/// appears in [`LEGAL_TRANSITION_EDGES`]; every other edge is
/// [`TransitionKind::Illegal`].
#[must_use = "the classification decides whether to write or refuse"]
pub fn classify_transition(from: TaskState, to: TaskState) -> TransitionKind {
    if from == to {
        return TransitionKind::NoOp;
    }
    if LEGAL_TRANSITION_EDGES.contains(&(from, to)) {
        TransitionKind::Legal
    } else {
        TransitionKind::Illegal
    }
}

/// Failure mode of [`splice_task_state`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SpliceError {
    /// The task's open-tag span did not contain a `state="..."` attribute,
    /// so there was nothing to rewrite. A well-formed parsed [`Task`]
    /// always carries one, so this signals corruption between parse and
    /// splice.
    #[error("task `{task_id}` open tag carries no `state` attribute to rewrite")]
    NoStateAttribute {
        /// Id of the task whose open tag lacked a `state` attribute.
        task_id: String,
    },
}

/// Byte-surgically rewrite one task's `state` attribute value in `raw`.
///
/// `raw` is the verbatim TASKS.md source; `task` is the parsed [`Task`]
/// whose [`Task::span`] locates its `<task>` open tag. Only the bytes of
/// the `state="..."` attribute *value* are replaced; every other byte of
/// `raw` — frontmatter, bodies, whitespace, and line endings — is
/// preserved verbatim.
///
/// # Errors
///
/// Returns [`SpliceError::NoStateAttribute`] when the open-tag span does
/// not contain a `state="..."` attribute (a corrupt parse).
pub fn splice_task_state(
    raw: &str,
    task: &Task,
    new_state: TaskState,
) -> Result<String, SpliceError> {
    // The open tag is the byte range [span.start, span.end). Locate the
    // `state="..."` attribute value's byte range *within* that range so a
    // `state=...` substring appearing in the body (after span.end) can
    // never match.
    let open_tag = raw.get(task.span.start..task.span.end).unwrap_or("");
    let (val_start_in_tag, val_end_in_tag) =
        state_value_range(open_tag).ok_or_else(|| SpliceError::NoStateAttribute {
            task_id: task.id.clone(),
        })?;

    let val_start = task.span.start.saturating_add(val_start_in_tag);
    let val_end = task.span.start.saturating_add(val_end_in_tag);

    let mut out = String::with_capacity(raw.len());
    out.push_str(raw.get(..val_start).unwrap_or(""));
    out.push_str(new_state.as_str());
    out.push_str(raw.get(val_end..).unwrap_or(""));
    Ok(out)
}

/// Locate the byte range of the `state="..."` attribute *value* (the
/// bytes between the quotes, excluding the quotes) within an open-tag
/// slice. Returns `(value_start, value_end)` offsets relative to
/// `open_tag`, or `None` when no `state="..."` attribute is present.
fn state_value_range(open_tag: &str) -> Option<(usize, usize)> {
    let caps = state_attr_regex().captures(open_tag)?;
    let value = caps.get(1)?;
    Some((value.start(), value.end()))
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn state_attr_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    // Match `state`, optional whitespace, `=`, optional whitespace, a
    // double quote, then capture the value bytes up to the closing quote.
    // Tolerates unusual-but-legal attribute spacing inside the open tag.
    CELL.get_or_init(|| Regex::new(r#"state\s*=\s*"([^"]*)""#).unwrap())
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
    let (frontmatter_raw, body, body_offset) = split_required(source, path, "TASKS.md")?;

    let heading = extract_level1_heading(body, path, "TASKS.md")?;

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
    use super::TransitionKind;
    use super::classify_transition;
    use super::parse;
    use super::splice_task_state;
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

    /// A fixture with a multi-line body, unusual-but-legal
    /// attribute spacing, and CRLF line endings rewrites byte-surgically.
    /// The result differs from the source only in the state value's bytes.
    #[test]
    fn splice_rewrites_only_the_state_value_crlf_multiline_unusual_spacing() {
        // Build a CRLF source with unusual-but-legal spacing around the
        // `state` attribute (`state  =  "pending"`). Two tasks so the
        // splice must hit exactly the first task's attribute.
        let src = concat!(
            "---\r\n",
            "spec: SPEC-0042\r\n",
            "generated_at: 2026-06-09T18:00:00Z\r\n",
            "---\r\n",
            "\r\n",
            "# Tasks: SPEC-0042\r\n",
            "\r\n",
            "<task id=\"T-001\"   state=\"pending\"   covers=\"REQ-001\">\r\n",
            "First line of body.\r\n",
            "\r\n",
            "Second line mentioning state=\"completed\" inside prose.\r\n",
            "\r\n",
            "<task-scenarios>\r\n",
            "Given X, when Y, then Z (T-001).\r\n",
            "</task-scenarios>\r\n",
            "</task>\r\n",
            "\r\n",
            "<task id=\"T-002\" state=\"pending\" covers=\"REQ-002\">\r\n",
            "Body two.\r\n",
            "\r\n",
            "<task-scenarios>\r\n",
            "Given A, when B, then C (T-002).\r\n",
            "</task-scenarios>\r\n",
            "</task>\r\n",
        );
        let doc = parse(src, path()).expect("CRLF fixture should parse");
        let t1 = doc.tasks.first().expect("two tasks parsed");
        assert_eq!(t1.state, TaskState::Pending);

        let rewritten = splice_task_state(src, t1, TaskState::InProgress)
            .expect("splice should locate the state attribute");

        // The only byte difference is `pending` -> `in-progress` at the
        // first task's state value. Reconstruct the expectation by
        // replacing exactly that one occurrence in the open-tag span.
        let expected = src.replacen(
            "id=\"T-001\"   state=\"pending\"",
            "id=\"T-001\"   state=\"in-progress\"",
            1,
        );
        assert_eq!(rewritten, expected);

        // The prose-embedded `state="completed"` and the second task's
        // `state="pending"` are untouched.
        assert!(rewritten.contains("state=\"completed\" inside prose"));
        assert!(
            rewritten.contains("<task id=\"T-002\" state=\"pending\""),
            "second task's state must be untouched",
        );
        // Re-parsing confirms the surgical edit took on the right task.
        let reparsed = parse(&rewritten, path()).expect("rewritten source re-parses");
        assert_eq!(
            reparsed.tasks.first().expect("task one").state,
            TaskState::InProgress,
        );
        assert_eq!(
            reparsed.tasks.get(1).expect("task two").state,
            TaskState::Pending,
        );
    }

    /// Every ordered state pair (16 combinations). Exactly the
    /// six legal edges plus the four same-state no-ops succeed; the other
    /// six are illegal.
    #[test]
    fn classify_covers_all_sixteen_ordered_pairs() {
        let states = [
            TaskState::Pending,
            TaskState::InProgress,
            TaskState::InReview,
            TaskState::Completed,
        ];
        let legal: &[(TaskState, TaskState)] = &[
            (TaskState::Pending, TaskState::InProgress),
            (TaskState::InProgress, TaskState::InReview),
            (TaskState::InReview, TaskState::Completed),
            (TaskState::InReview, TaskState::Pending),
            (TaskState::InProgress, TaskState::Pending),
            (TaskState::Completed, TaskState::Pending),
        ];

        let mut legal_count = 0;
        let mut noop_count = 0;
        let mut illegal_count = 0;
        for from in states {
            for to in states {
                match classify_transition(from, to) {
                    TransitionKind::NoOp => {
                        assert_eq!(from, to, "no-op only when from == to");
                        noop_count += 1;
                    }
                    TransitionKind::Legal => {
                        assert!(
                            legal.contains(&(from, to)),
                            "{from:?} -> {to:?} classified Legal but is not a legal edge",
                        );
                        legal_count += 1;
                    }
                    TransitionKind::Illegal => {
                        assert_ne!(from, to);
                        assert!(
                            !legal.contains(&(from, to)),
                            "{from:?} -> {to:?} classified Illegal but is a legal edge",
                        );
                        illegal_count += 1;
                    }
                }
            }
        }
        assert_eq!(legal_count, 6, "exactly six legal edges");
        assert_eq!(noop_count, 4, "exactly four same-state no-ops");
        assert_eq!(illegal_count, 6, "the remaining six are illegal");
    }

    #[test]
    fn state_parse_rejects_unknown_and_accepts_known() {
        for s in ALLOWED_TASK_STATES {
            assert!(TaskState::parse(s).is_some(), "`{s}` should parse");
        }
        assert!(TaskState::parse("shipped").is_none());
        assert!(TaskState::parse("").is_none());
    }
}
