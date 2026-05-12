//! TASKS.md parser.
//!
//! Extracts task IDs, state glyphs, covers/suggested-files metadata, and
//! verbatim notes per task. Malformed task IDs are surfaced as
//! recoverable [`TaskWarning`]s; the parse continues so SPEC-0003 (lint)
//! can emit `TSK-002` without re-parsing. See
//! `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-004.

use crate::error::ParseError;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
use crate::parse::markdown::TextSpan;
use crate::parse::markdown::inline_spans;
use crate::parse::markdown::inline_text;
use crate::parse::markdown::parse_markdown;
use crate::parse::toml_files::read_to_string;
use camino::Utf8Path;
use comrak::Arena;
use comrak::arena_tree::Node;
use comrak::nodes::Ast;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use regex::Regex;
use serde::Deserialize;
use std::cell::RefCell;
use std::sync::OnceLock;

/// Parsed TASKS.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasksMd {
    /// YAML frontmatter.
    pub frontmatter: TasksFrontmatter,
    /// Tasks in declared order.
    pub tasks: Vec<Task>,
    /// Recoverable warnings (e.g. malformed task IDs) for the lint engine.
    pub warnings: Vec<TaskWarning>,
}

/// TASKS.md frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TasksFrontmatter {
    /// Spec ID this TASKS.md belongs to (`SPEC-NNNN`).
    pub spec: String,
    /// sha256 of the SPEC.md content captured by `speccy tasks --commit`.
    /// Stored verbatim as a string so non-hash sentinels (e.g.
    /// `bootstrap-pending`) survive round-trip without being rejected.
    pub spec_hash_at_generation: String,
    /// UTC timestamp captured by `speccy tasks --commit`. Stored verbatim
    /// for now.
    pub generated_at: String,
}

/// One parsed task line from TASKS.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Stable `T-NNN` identifier.
    pub id: String,
    /// Title text after the bold ID prefix.
    pub title: String,
    /// Checkbox state.
    pub state: TaskState,
    /// IDs from the `Covers:` bullet, if present.
    pub covers: Vec<String>,
    /// Files from the `Suggested files:` bullet, if present.
    pub suggested_files: Vec<String>,
    /// Verbatim text of every sub-list bullet under the task, in declared
    /// order.
    pub notes: Vec<String>,
    /// 1-indexed line number of the task in the source.
    pub line: usize,
}

/// Checkbox glyph mapping for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// `[ ]`: needs work (new or retry after blocking review).
    Open,
    /// `[~]`: claimed by an implementer.
    InProgress,
    /// `[?]`: implementation done; awaiting review.
    AwaitingReview,
    /// `[x]`: all persona reviews passed.
    Done,
}

impl TaskState {
    /// Glyph form (e.g. `[ ]`).
    #[must_use = "the glyph is the on-disk form"]
    pub const fn as_glyph(self) -> &'static str {
        match self {
            TaskState::Open => "[ ]",
            TaskState::InProgress => "[~]",
            TaskState::AwaitingReview => "[?]",
            TaskState::Done => "[x]",
        }
    }
}

/// Recoverable warning emitted while parsing a task line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskWarning {
    /// 1-indexed source line.
    pub line: usize,
    /// Human-readable message.
    pub message: String,
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn task_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^T-\d{3}$").unwrap())
}

/// Parse a TASKS.md file.
///
/// # Errors
///
/// Returns any [`ParseError`] variant relevant to TASKS.md parsing: I/O,
/// missing or malformed frontmatter, or YAML deserialisation failures.
/// Malformed task IDs do not error; they are surfaced via
/// [`TasksMd::warnings`].
pub fn tasks_md(path: &Utf8Path) -> Result<TasksMd, ParseError> {
    let raw = read_to_string(path)?;
    let frontmatter = parse_frontmatter(&raw, path)?;

    let arena = Arena::new();
    let root = parse_markdown(&arena, &raw);

    let mut tasks = Vec::new();
    let mut warnings = Vec::new();
    collect_tasks(root, &mut tasks, &mut warnings);

    Ok(TasksMd {
        frontmatter,
        tasks,
        warnings,
    })
}

fn parse_frontmatter(raw: &str, path: &Utf8Path) -> Result<TasksFrontmatter, ParseError> {
    let split_result = split_frontmatter(raw, path)?;
    let yaml = match split_result {
        Split::Some { yaml, .. } => yaml,
        Split::None => {
            return Err(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("TASKS.md at {path}"),
            });
        }
    };
    serde_saphyr::from_str(yaml).map_err(|e| ParseError::Yaml {
        label: Some(path.to_string()),
        message: e.to_string(),
    })
}

fn collect_tasks<'a>(
    root: &'a AstNode<'a>,
    tasks: &mut Vec<Task>,
    warnings: &mut Vec<TaskWarning>,
) {
    // Walk every list item in the document. Tasks are top-level items
    // whose paragraph begins with a checkbox glyph; nested items are
    // surfaced as notes via collect_notes.
    for node in root.descendants() {
        let ast = node.data.borrow();
        if !matches!(ast.value, NodeValue::Item(_)) {
            continue;
        }
        let line = ast.sourcepos.start.line;
        drop(ast);
        try_parse_task(node, line, tasks, warnings);
    }
}

fn try_parse_task<'a>(
    item: &'a Node<'a, RefCell<Ast>>,
    line: usize,
    tasks: &mut Vec<Task>,
    warnings: &mut Vec<TaskWarning>,
) {
    let Some(paragraph) = first_child_of_kind(item, |v| matches!(v, NodeValue::Paragraph)) else {
        return;
    };
    let inline = inline_text(paragraph);
    let Some(state) = strip_checkbox(&inline) else {
        return;
    };

    // The remaining inline content must start with a bold span containing
    // the task ID. Comrak yields a Strong node followed by the trailing
    // text. We re-walk the paragraph children to grab the Strong content.
    let Some((raw_id, title)) = extract_bold_id_and_title(paragraph) else {
        warnings.push(TaskWarning {
            line,
            message: format!(
                "task on line {line} has checkbox `{}` but no bold `**T-NNN**` prefix",
                state.as_glyph()
            ),
        });
        return;
    };

    if !task_id_regex().is_match(&raw_id) {
        warnings.push(TaskWarning {
            line,
            message: format!("task on line {line} has malformed ID `{raw_id}`; expected `T-NNN`"),
        });
        return;
    }

    let (covers, suggested_files, notes) = collect_metadata(item);

    tasks.push(Task {
        id: raw_id,
        title: title.trim_start_matches(':').trim().to_owned(),
        state,
        covers,
        suggested_files,
        notes,
        line,
    });
}

fn strip_checkbox(text: &str) -> Option<TaskState> {
    let trimmed = text.trim_start();
    if trimmed.starts_with("[ ]") {
        Some(TaskState::Open)
    } else if trimmed.starts_with("[~]") {
        Some(TaskState::InProgress)
    } else if trimmed.starts_with("[?]") {
        Some(TaskState::AwaitingReview)
    } else if trimmed.starts_with("[x]") || trimmed.starts_with("[X]") {
        Some(TaskState::Done)
    } else {
        None
    }
}

fn extract_bold_id_and_title<'a>(
    paragraph: &'a Node<'a, RefCell<Ast>>,
) -> Option<(String, String)> {
    // The paragraph's direct children alternate: Text("[ ] "),
    // Strong{Text("T-001")}, Text(": rest of title"). We scan for the first
    // Strong child and collect everything after it as the title.
    let mut id: Option<String> = None;
    let mut title = String::new();
    let mut after_strong = false;

    for child in paragraph.children() {
        let ast = child.data.borrow();
        match &ast.value {
            NodeValue::Strong if id.is_none() => {
                id = Some(inline_text(child));
                after_strong = true;
            }
            NodeValue::Text(t) if after_strong => title.push_str(t),
            NodeValue::Code(c) if after_strong => title.push_str(&c.literal),
            NodeValue::LineBreak | NodeValue::SoftBreak if after_strong => title.push(' '),
            _ => {}
        }
    }

    id.map(|raw| (raw, title))
}

fn collect_metadata<'a>(
    item: &'a Node<'a, RefCell<Ast>>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut covers = Vec::new();
    let mut suggested_files = Vec::new();
    let mut notes = Vec::new();

    for child in item.children() {
        let ast = child.data.borrow();
        if !matches!(ast.value, NodeValue::List(_)) {
            continue;
        }
        drop(ast);
        for sub_item in child.children() {
            collect_metadata_from_item(sub_item, &mut covers, &mut suggested_files, &mut notes);
        }
    }

    (covers, suggested_files, notes)
}

fn collect_metadata_from_item<'a>(
    sub_item: &'a Node<'a, RefCell<Ast>>,
    covers: &mut Vec<String>,
    suggested_files: &mut Vec<String>,
    notes: &mut Vec<String>,
) {
    let ast = sub_item.data.borrow();
    if !matches!(ast.value, NodeValue::Item(_)) {
        return;
    }
    drop(ast);
    let Some(paragraph) = first_child_of_kind(sub_item, |v| matches!(v, NodeValue::Paragraph))
    else {
        return;
    };

    let plain = inline_text(paragraph);
    let trimmed = plain.trim();

    if let Some(rest) = trimmed.strip_prefix("Covers:") {
        for id in rest.split(',') {
            let s = id.trim();
            if !s.is_empty() {
                covers.push(s.to_owned());
            }
        }
        notes.push(trimmed.to_owned());
        return;
    }

    if trimmed.starts_with("Suggested files:") {
        let spans = inline_spans(paragraph);
        for span in spans {
            if let TextSpan::Code(literal) = span {
                let s = literal.trim();
                if !s.is_empty() {
                    suggested_files.push(s.to_owned());
                }
            }
        }
        notes.push(trimmed.to_owned());
        return;
    }

    notes.push(trimmed.to_owned());
}

fn first_child_of_kind<'a, F>(
    parent: &'a Node<'a, RefCell<Ast>>,
    predicate: F,
) -> Option<&'a Node<'a, RefCell<Ast>>>
where
    F: Fn(&NodeValue) -> bool,
{
    parent.children().find(|c| {
        let ast = c.data.borrow();
        predicate(&ast.value)
    })
}

#[cfg(test)]
mod tests {
    use super::TaskState;
    use super::tasks_md;
    use camino::Utf8PathBuf;
    use indoc::indoc;
    use tempfile::TempDir;

    struct Fixture {
        _dir: TempDir,
        path: Utf8PathBuf,
    }

    fn write_tmp(content: &str) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir creation should succeed");
        let std_path = dir.path().join("TASKS.md");
        fs_err::write(&std_path, content).expect("writing fixture should succeed");
        let path = Utf8PathBuf::from_path_buf(std_path).expect("tempdir path should be UTF-8");
        Fixture { _dir: dir, path }
    }

    #[test]
    fn parses_states_and_counts() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            spec_hash_at_generation: bootstrap-pending
            generated_at: 2026-05-11T00:00:00Z
            ---

            # Tasks

            ## Phase 1
            - [ ] **T-001**: First open
              - Covers: REQ-001
            - [ ] **T-002**: Second open
            - [~] **T-003**: Running
            - [?] **T-004**: Awaiting review
            - [x] **T-005**: Done
        "};
        let fx = write_tmp(src);
        let parsed = tasks_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.tasks.len(), 5);

        let states: Vec<TaskState> = parsed.tasks.iter().map(|t| t.state).collect();
        assert_eq!(
            states,
            vec![
                TaskState::Open,
                TaskState::Open,
                TaskState::InProgress,
                TaskState::AwaitingReview,
                TaskState::Done,
            ],
        );

        let first = parsed.tasks.first().expect("first task");
        assert_eq!(first.id, "T-001");
        assert_eq!(first.title, "First open");
        assert_eq!(first.covers, vec!["REQ-001".to_owned()]);
    }

    #[test]
    fn parses_covers_with_multiple_ids() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            spec_hash_at_generation: 0
            generated_at: 2026-05-11T00:00:00Z
            ---

            - [ ] **T-001**: thing
              - Covers: REQ-001, REQ-002
        "};
        let fx = write_tmp(src);
        let parsed = tasks_md(&fx.path).expect("parse should succeed");
        let first = parsed.tasks.first().expect("first task");
        assert_eq!(
            first.covers,
            vec!["REQ-001".to_owned(), "REQ-002".to_owned()]
        );
    }

    #[test]
    fn parses_suggested_files_from_backticks() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            spec_hash_at_generation: 0
            generated_at: 2026-05-11T00:00:00Z
            ---

            - [ ] **T-001**: thing
              - Suggested files: `migrations/`, `db/schema/users.ts`
        "};
        let fx = write_tmp(src);
        let parsed = tasks_md(&fx.path).expect("parse should succeed");
        let first = parsed.tasks.first().expect("first task");
        assert_eq!(
            first.suggested_files,
            vec!["migrations/".to_owned(), "db/schema/users.ts".to_owned()]
        );
    }

    #[test]
    fn malformed_task_id_surfaces_warning_not_error() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            spec_hash_at_generation: 0
            generated_at: 2026-05-11T00:00:00Z
            ---

            - [ ] **TASK-001**: malformed prefix
            - [ ] **T-002**: well-formed
        "};
        let fx = write_tmp(src);
        let parsed = tasks_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.tasks.len(), 1);
        let only = parsed.tasks.first().expect("one task");
        assert_eq!(only.id, "T-002");
        assert_eq!(parsed.warnings.len(), 1);
    }

    #[test]
    fn collects_sub_list_notes_in_declared_order() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            spec_hash_at_generation: 0
            generated_at: 2026-05-11T00:00:00Z
            ---

            - [?] **T-001**: x
              - Review (business, pass): looks fine
              - Review (tests, pass): assertions OK
              - Review (security, blocking): bcrypt cost 10
        "};
        let fx = write_tmp(src);
        let parsed = tasks_md(&fx.path).expect("parse should succeed");
        let first = parsed.tasks.first().expect("first task");
        assert_eq!(first.notes.len(), 3);
        assert!(first.notes.first().is_some_and(|n| n.contains("business")));
        assert!(first.notes.get(2).is_some_and(|n| n.contains("security")));
    }

    #[test]
    fn phase_headings_do_not_appear_as_tasks() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            spec_hash_at_generation: 0
            generated_at: 2026-05-11T00:00:00Z
            ---

            ## Phase 1: Schema

            ## Phase 2: API
            - [ ] **T-001**: only task
        "};
        let fx = write_tmp(src);
        let parsed = tasks_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.tasks.len(), 1);
    }
}
