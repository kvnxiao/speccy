//! Private one-shot migration tool for SPEC-0029.
//!
//! Converts a TASKS.md from the legacy markdown-bullet authoring
//! conventions to the new XML schema:
//!
//! - `- Implementer note (session-X):` plus its six indented sub-bullets
//!   becomes `<implementer-note session="X">…</implementer-note>`.
//! - `- Review (<persona>, <verdict>[, retry]): <prose>` becomes `<review
//!   persona="<persona>" verdict="<verdict>">…</review>`. The optional `,
//!   retry` annotation is dropped — the new schema attributes retries
//!   implicitly by source position (SPEC-0029 DEC-008).
//! - `- Retry: <prose>` becomes `<retry>…</retry>`.
//!
//! The transitional state machine recognises BOTH legacy bullets AND the
//! new XML element form, so a re-run against an already-migrated TASKS.md
//! produces byte-identical output (SPEC-0029 REQ-006 idempotency
//! contract). Free Markdown prose, `<task-scenarios>` bodies,
//! `Suggested files:` bullets, frontmatter, phase headings, and every
//! other byte are preserved verbatim.
//!
//! After T-003 applies this tool to the in-tree corpus, the binary is
//! dead code that a follow-on SPEC may delete (SPEC-0029 DEC-005). The
//! tool is NOT a `speccy` CLI subcommand and does not surface in
//! `speccy --help`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use regex::Regex;
use std::sync::OnceLock;
use thiserror::Error;

/// Errors surfaced while migrating a TASKS.md file.
#[derive(Debug, Error)]
pub enum MigrateError {
    /// Reading or writing the source file failed.
    #[error("I/O error for {path}")]
    Io {
        /// File whose IO failed.
        path: Utf8PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The migrated output failed to re-parse under the shipped
    /// `task_xml` parser. The migration tool refuses to write files it
    /// cannot re-parse, keeping the in-tree corpus in a green state.
    #[error("post-migration parse failed for {path}: {source}")]
    PostParseFailed {
        /// File whose migrated output failed to re-parse.
        path: Utf8PathBuf,
        /// Underlying parser error (boxed to keep `Result` small).
        #[source]
        source: Box<speccy_core::ParseError>,
    },
}

/// Outcome of migrating one TASKS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The input was already in the new canonical form; no bytes were
    /// written.
    Unchanged,
    /// Legacy bullets were converted and the file was rewritten.
    Migrated,
}

/// Migrate one TASKS.md in place.
///
/// Reads `path`, runs the transitional state machine, and writes the
/// result back when (and only when) the output differs from the input.
///
/// The migrated output is re-parsed under the shipped
/// [`speccy_core::parse::parse_task_xml`] parser before any write; if
/// the re-parse fails, the function returns
/// [`MigrateError::PostParseFailed`] and leaves the file untouched.
///
/// # Errors
///
/// Returns [`MigrateError::Io`] on filesystem failure and
/// [`MigrateError::PostParseFailed`] when the migrated output is not
/// parseable.
pub fn migrate_file(path: &Utf8Path) -> Result<Outcome, MigrateError> {
    let source = fs_err::read_to_string(path.as_std_path()).map_err(|source| MigrateError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let migrated = migrate(&source);
    if migrated == source {
        return Ok(Outcome::Unchanged);
    }
    speccy_core::parse::parse_task_xml(&migrated, path).map_err(|source| {
        MigrateError::PostParseFailed {
            path: path.to_path_buf(),
            source: Box::new(source),
        }
    })?;
    fs_err::write(path.as_std_path(), &migrated).map_err(|source| MigrateError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Outcome::Migrated)
}

/// Migrate one TASKS.md source string.
///
/// When the input carries no legacy bullets, the output is byte-for-byte
/// identical to the input — that property anchors the idempotency
/// contract (SPEC-0029 REQ-006 ¶4).
///
/// The transformation is line-oriented and preserves byte content
/// outside `<task>` bodies; inside a task body, it touches only lines
/// that match one of the three legacy bullet conventions plus their
/// indented continuation lines.
#[must_use = "migrated source must be written back to disk"]
pub fn migrate(source: &str) -> String {
    let mut out = String::with_capacity(source.len().saturating_add(64));
    let mut state = State::Outside;
    for line in source.split_inclusive('\n') {
        let (content, eol) = split_eol(line);
        process_line(content, eol, &mut state, &mut out);
    }
    // If the file ends mid-legacy-block (no terminator line), close it
    // out so the produced source still parses.
    if let State::InLegacyBlock(kind) = state {
        emit_legacy_close(kind, "\n", &mut out);
    }
    out
}

fn split_eol(line_with_eol: &str) -> (&str, &str) {
    if let Some(stripped) = line_with_eol.strip_suffix('\n') {
        (stripped, "\n")
    } else {
        (line_with_eol, "")
    }
}

#[derive(Debug, Clone, Copy)]
enum LegacyKind {
    ImplementerNote,
    Review,
    Retry,
}

#[derive(Debug, Clone, Copy)]
enum State {
    /// Before `<tasks>`, after `</tasks>`, or between tasks.
    Outside,
    /// Inside `<task>` body, outside any nested element or legacy
    /// bullet block.
    InTaskBody,
    /// Inside a nested element that already exists in XML form. The
    /// element name names the close tag we are waiting for.
    InVerbatimNested(NestedKind),
    /// Inside a legacy bullet block whose continuation lines are being
    /// dedented and reframed into an XML element body.
    InLegacyBlock(LegacyKind),
}

#[derive(Debug, Clone, Copy)]
enum NestedKind {
    TaskScenarios,
    ImplementerNote,
    Review,
    Retry,
}

impl NestedKind {
    const fn close_tag(self) -> &'static str {
        match self {
            NestedKind::TaskScenarios => "</task-scenarios>",
            NestedKind::ImplementerNote => "</implementer-note>",
            NestedKind::Review => "</review>",
            NestedKind::Retry => "</retry>",
        }
    }
}

fn process_line(content: &str, eol: &str, state: &mut State, out: &mut String) {
    // Phase 1: inside a legacy block we either continue (dedent and
    // emit) or terminate (emit close tag and fall through to a fresh
    // dispatch against the same line).
    //
    // A continuation that is itself a legacy bullet (after the
    // 2-space dedent) — e.g. `  - Review (security, pass): ...`
    // nested under an implementer note — terminates the current
    // legacy block before being dispatched, so misnested reviewer or
    // retry notes get lifted to top-level XML elements instead of
    // being swallowed into the surrounding implementer-note body.
    if let State::InLegacyBlock(kind) = *state {
        if is_legacy_continuation(content) {
            let dedented = content.get(2..).unwrap_or("");
            if is_legacy_bullet(dedented) {
                emit_legacy_close(kind, eol_for_close(eol), out);
                *state = State::InTaskBody;
                process_line(dedented, eol, state, out);
                return;
            }
            out.push_str(dedented);
            out.push_str(eol);
            return;
        }
        emit_legacy_close(kind, eol_for_close(eol), out);
        *state = State::InTaskBody;
        // Fall through and re-dispatch this line under InTaskBody.
    }

    match *state {
        State::Outside => {
            out.push_str(content);
            out.push_str(eol);
            if let Some(name) = tag_name(content)
                && name == TagName::Open("task")
            {
                *state = State::InTaskBody;
            }
        }
        State::InTaskBody => dispatch_in_task_body(content, eol, state, out),
        State::InVerbatimNested(kind) => {
            out.push_str(content);
            out.push_str(eol);
            if content.trim() == kind.close_tag() {
                *state = State::InTaskBody;
            }
        }
        State::InLegacyBlock(_) => {
            // Phase 1 above already consumed or terminated this line.
            // Reaching this arm would mean a missed transition; emit
            // verbatim defensively rather than panicking.
            out.push_str(content);
            out.push_str(eol);
        }
    }
}

fn dispatch_in_task_body(content: &str, eol: &str, state: &mut State, out: &mut String) {
    // Tag-shaped lines: `<task>`, `</task>`, `<task-scenarios>`, etc.
    if let Some(name) = tag_name(content) {
        match name {
            TagName::Close("task") => {
                out.push_str(content);
                out.push_str(eol);
                *state = State::Outside;
                return;
            }
            TagName::Open("task-scenarios") => {
                out.push_str(content);
                out.push_str(eol);
                *state = State::InVerbatimNested(NestedKind::TaskScenarios);
                return;
            }
            TagName::Open("implementer-note") => {
                out.push_str(content);
                out.push_str(eol);
                *state = State::InVerbatimNested(NestedKind::ImplementerNote);
                return;
            }
            TagName::Open("review") => {
                out.push_str(content);
                out.push_str(eol);
                *state = State::InVerbatimNested(NestedKind::Review);
                return;
            }
            TagName::Open("retry") => {
                out.push_str(content);
                out.push_str(eol);
                *state = State::InVerbatimNested(NestedKind::Retry);
                return;
            }
            _ => {
                out.push_str(content);
                out.push_str(eol);
                return;
            }
        }
    }

    // Legacy implementer note opening.
    if let Some(session) = match_implementer_note(content) {
        out.push_str("<implementer-note session=\"");
        out.push_str(&session);
        out.push_str("\">");
        out.push_str(eol);
        *state = State::InLegacyBlock(LegacyKind::ImplementerNote);
        return;
    }
    // Legacy review opening.
    if let Some((persona, verdict, rest)) = match_review(content) {
        out.push_str("<review persona=\"");
        out.push_str(&persona);
        out.push_str("\" verdict=\"");
        out.push_str(&verdict);
        out.push_str("\">");
        out.push_str(eol);
        if !rest.is_empty() {
            out.push_str(&rest);
            out.push_str(eol);
        }
        *state = State::InLegacyBlock(LegacyKind::Review);
        return;
    }
    // Legacy retry opening.
    if let Some(rest) = match_retry(content) {
        out.push_str("<retry>");
        out.push_str(eol);
        if !rest.is_empty() {
            out.push_str(&rest);
            out.push_str(eol);
        }
        *state = State::InLegacyBlock(LegacyKind::Retry);
        return;
    }

    // Free Markdown prose — emit verbatim.
    out.push_str(content);
    out.push_str(eol);
}

/// A continuation of a legacy bullet block: line starts with two or
/// more spaces. Truly empty lines and lines starting at column 0
/// terminate the block.
fn is_legacy_continuation(content: &str) -> bool {
    content.starts_with("  ")
}

/// True when `content` matches one of the three legacy bullet openers.
/// Used to detect misnested reviewer / retry notes inside an enclosing
/// implementer-note's continuation lines so they can be lifted to
/// top-level XML elements.
fn is_legacy_bullet(content: &str) -> bool {
    match_implementer_note(content).is_some()
        || match_review(content).is_some()
        || match_retry(content).is_some()
}

fn eol_for_close(observed_eol: &str) -> &'static str {
    // The migrated output always uses `\n` line endings (the in-tree
    // corpus is `\n`-only). Mirror that for the close tag insertion.
    if observed_eol.is_empty() {
        // The terminating line had no newline (e.g. EOF in the middle
        // of a body); insert a newline so the close tag lands on its
        // own line as the XML scanner requires.
        "\n"
    } else {
        "\n"
    }
}

fn emit_legacy_close(kind: LegacyKind, eol: &str, out: &mut String) {
    match kind {
        LegacyKind::ImplementerNote => out.push_str("</implementer-note>"),
        LegacyKind::Review => out.push_str("</review>"),
        LegacyKind::Retry => out.push_str("</retry>"),
    }
    out.push_str(eol);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagName<'a> {
    Open(&'a str),
    Close(&'a str),
}

fn tag_name(content: &str) -> Option<TagName<'_>> {
    let trimmed = content.trim();
    if let Some(caps) = open_tag_re().captures(trimmed) {
        let name = caps.get(1).map(|m| m.as_str())?;
        return Some(TagName::Open(name));
    }
    if let Some(caps) = close_tag_re().captures(trimmed) {
        let name = caps.get(1).map(|m| m.as_str())?;
        return Some(TagName::Close(name));
    }
    None
}

fn match_implementer_note(content: &str) -> Option<String> {
    let caps = implementer_note_re().captures(content)?;
    let session = caps.get(1).map(|m| m.as_str().to_owned())?;
    Some(session)
}

fn match_review(content: &str) -> Option<(String, String, String)> {
    let caps = review_re().captures(content)?;
    let persona = caps.get(1).map(|m| m.as_str().to_owned())?;
    let verdict = caps.get(2).map(|m| m.as_str().to_owned())?;
    let rest = caps
        .get(3)
        .map(|m| m.as_str().to_owned())
        .unwrap_or_default();
    Some((persona, verdict, rest))
}

fn match_retry(content: &str) -> Option<String> {
    let caps = retry_re().captures(content)?;
    Some(
        caps.get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default(),
    )
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn open_tag_re() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r#"^<([a-z][a-z-]*)(?:\s[^>]*)?>$"#).unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn close_tag_re() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^</([a-z][a-z-]*)>$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn implementer_note_re() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    // Legacy convention: `- Implementer note (session-X):` at column 0.
    // The literal `session-` inside the parens was a label in the
    // legacy syntax; in the new XML form the attribute name IS
    // `session`, so we drop the redundant prefix and capture only the
    // identifier `X`. Every legacy session value in the in-tree corpus
    // begins with `session-`, so the prefix is always present.
    CELL.get_or_init(|| Regex::new(r"^- Implementer note \(session-([^)]+)\):\s*$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn review_re() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    // Legacy convention: `- Review (persona, verdict[, retry]): prose`
    // at column 0. The optional ", retry" annotation is dropped during
    // migration — the new schema attributes retries by source position
    // (SPEC-0029 DEC-008).
    CELL.get_or_init(|| {
        Regex::new(r"^- Review \(([a-z]+), (pass|blocking)(?:, retry)?\):\s*(.*)$").unwrap()
    })
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn retry_re() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^- Retry:\s*(.*)$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_passes_through() {
        assert_eq!(migrate(""), "");
    }

    #[test]
    fn input_without_tasks_passes_through() {
        let src = "---\nfoo: bar\n---\n\n# Heading\n\nSome prose.\n";
        assert_eq!(migrate(src), src);
    }

    #[test]
    fn detects_implementer_note_session() {
        let session = match_implementer_note("- Implementer note (session-T001):");
        assert_eq!(session.as_deref(), Some("T001"));
    }

    #[test]
    fn rejects_implementer_note_without_session_prefix() {
        // Defensive: the legacy convention always wrote `session-X`, so a
        // bullet like `- Implementer note (T001):` shouldn't match. It
        // would fall through to verbatim emission and surface as a
        // verify-time anomaly instead of being silently mis-migrated.
        assert!(match_implementer_note("- Implementer note (T001):").is_none());
    }

    #[test]
    fn detects_review_with_optional_retry_annotation() {
        let m = match_review("- Review (business, pass, retry): some prose");
        assert_eq!(
            m,
            Some((
                "business".to_owned(),
                "pass".to_owned(),
                "some prose".to_owned()
            ))
        );
    }

    #[test]
    fn detects_review_two_token_form() {
        let m = match_review("- Review (tests, blocking): prose");
        assert_eq!(
            m,
            Some((
                "tests".to_owned(),
                "blocking".to_owned(),
                "prose".to_owned()
            ))
        );
    }

    #[test]
    fn detects_retry() {
        let m = match_retry("- Retry: do the thing");
        assert_eq!(m.as_deref(), Some("do the thing"));
    }

    #[test]
    fn rejects_indented_legacy_bullet() {
        // Indented bullets must NOT match — they are sub-bullets inside
        // an existing block.
        assert!(match_implementer_note("  - Implementer note (session-X):").is_none());
        assert!(match_review("  - Review (business, pass): foo").is_none());
        assert!(match_retry("  - Retry: foo").is_none());
    }

    #[test]
    fn is_legacy_bullet_detects_all_three_kinds() {
        assert!(is_legacy_bullet("- Implementer note (session-x):"));
        assert!(is_legacy_bullet("- Review (business, pass): prose"));
        assert!(is_legacy_bullet("- Retry: prose"));
        assert!(!is_legacy_bullet("- Suggested files: a.rs"));
        assert!(!is_legacy_bullet("- Covers: REQ-001"));
        assert!(!is_legacy_bullet("free prose"));
    }

    #[test]
    fn nested_review_inside_implementer_note_lifts_to_top_level() {
        // A `- Review (...)` bullet 2-space-indented as a continuation of a
        // legacy `- Implementer note (...)` must terminate the surrounding
        // implementer-note and emit as its own top-level `<review>`,
        // matching the in-tree SPEC-0018 anomaly fix.
        let src = concat!(
            "<tasks spec=\"SPEC-0099\">\n",
            "<task id=\"T-001\" state=\"completed\" covers=\"REQ-001\">\n",
            "Title\n",
            "- Implementer note (session-x):\n",
            "  - Completed: did the thing\n",
            "  - Review (security, pass): no new surface\n",
            "- Review (tests, pass): all good\n",
            "<task-scenarios>\n- placeholder.\n</task-scenarios>\n",
            "</task>\n",
            "</tasks>\n",
        );
        let migrated = migrate(src);
        assert!(
            migrated.contains("<review persona=\"security\" verdict=\"pass\">"),
            "nested security review must lift to top-level: {migrated}"
        );
        assert!(
            !migrated.contains("\n- Review ("),
            "no `^- Review (` should survive at column 0: {migrated}"
        );
        let body_end = migrated
            .find("</implementer-note>")
            .expect("implementer-note close present");
        let before_close = migrated.get(..body_end).expect("prefix present");
        assert!(
            !before_close.contains("- Review ("),
            "no legacy review bullet should remain inside the implementer-note body: {migrated}"
        );
    }

    #[test]
    fn tag_name_detects_open_and_close() {
        assert_eq!(tag_name("<task>"), Some(TagName::Open("task")));
        assert_eq!(
            tag_name(r#"<task id="T-001" state="pending" covers="REQ-001">"#),
            Some(TagName::Open("task"))
        );
        assert_eq!(tag_name("</task>"), Some(TagName::Close("task")));
        assert_eq!(tag_name("- Bullet line"), None);
        assert_eq!(tag_name("free prose"), None);
    }
}
