//! Parser for per-task journal files (`journal/T-NNN.md`).
//!
//! SPEC-0037 REQ-001 introduces a sibling artifact under each spec
//! directory at `.speccy/specs/NNNN-slug/journal/T-NNN.md`. The file
//! shape is YAML frontmatter (`spec`, `task`, `generated_at`) followed
//! by a chronological sequence of bare `<implementer>`, `<review>`,
//! and `<blockers>` element blocks. No wrapper element groups them —
//! the filename + frontmatter bind the file to its task and spec.
//!
//! Element attribute schemas (SPEC-0037 REQ-003):
//! - `<implementer>`: `date`, `model`, `round` (all required).
//! - `<review>`: `date`, `model`, `persona`, `verdict`, `round`.
//! - `<blockers>`: `date`, `round`.
//!
//! `date` is ISO8601 with seconds and timezone designator;
//! `round` is a positive integer; `model` is a non-empty string with a
//! documented slash-suffix convention for effort
//! (e.g. `claude-opus-4.8[1m]/low`) — the slash-suffix is not
//! parser-validated.

use crate::error::ParseError;
use crate::error::ParseResult;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
use crate::parse::xml_scanner::ElementSpan;
use crate::parse::xml_scanner::RawTag;
use crate::parse::xml_scanner::ScanConfig;
use crate::parse::xml_scanner::collect_code_fence_byte_ranges;
use crate::parse::xml_scanner::scan_tags;
use crate::personas::ALL as PERSONAS_ALL;
use camino::Utf8Path;
use regex::Regex;
use std::sync::OnceLock;

/// Closed set of `<review verdict="...">` values, on-disk form.
pub const ALLOWED_REVIEW_VERDICTS: &[&str] = &["pass", "blocking"];

/// Closed whitelist of element names recognised inside a journal file.
pub const JOURNAL_ELEMENT_NAMES: &[&str] = &["implementer", "review", "blockers"];

/// Parsed per-task journal file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalDoc {
    /// `spec:` field from the frontmatter, e.g. `SPEC-0037`.
    pub spec: String,
    /// `task:` field from the frontmatter, e.g. `T-001`.
    pub task: String,
    /// `generated_at:` field from the frontmatter.
    pub generated_at: String,
    /// Chronological sequence of activity blocks.
    pub entries: Vec<JournalEntry>,
}

/// One activity block inside a journal file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalEntry {
    /// `<implementer date="..." model="..." round="...">`.
    Implementer {
        /// ISO8601 timestamp.
        date: String,
        /// Free-form model identifier (with optional `/effort` suffix).
        model: String,
        /// Positive integer round counter.
        round: u32,
        /// Verbatim body.
        body: String,
        /// Span of the `<implementer>` open tag.
        span: ElementSpan,
    },
    /// `<review date="..." model="..." persona="..." verdict="..."
    /// round="...">`.
    Review {
        /// ISO8601 timestamp.
        date: String,
        /// Free-form model identifier.
        model: String,
        /// Persona name (from [`crate::personas::ALL`]).
        persona: String,
        /// `pass` or `blocking`.
        verdict: String,
        /// Positive integer round counter.
        round: u32,
        /// Verbatim body.
        body: String,
        /// Span of the `<review>` open tag.
        span: ElementSpan,
    },
    /// `<blockers date="..." round="...">`.
    Blockers {
        /// ISO8601 timestamp.
        date: String,
        /// Positive integer round counter.
        round: u32,
        /// Verbatim body.
        body: String,
        /// Span of the `<blockers>` open tag.
        span: ElementSpan,
    },
}

impl JournalEntry {
    /// Convenience: the round number on any entry variant.
    #[must_use = "the round number drives REQ-004 sequence validation"]
    pub fn round(&self) -> u32 {
        match self {
            JournalEntry::Implementer { round, .. }
            | JournalEntry::Review { round, .. }
            | JournalEntry::Blockers { round, .. } => *round,
        }
    }

    /// Convenience: the element local name.
    #[must_use = "the element name is used in diagnostics"]
    pub fn element_name(&self) -> &'static str {
        match self {
            JournalEntry::Implementer { .. } => "implementer",
            JournalEntry::Review { .. } => "review",
            JournalEntry::Blockers { .. } => "blockers",
        }
    }
}

fn iso8601_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    #[expect(
        clippy::unwrap_used,
        reason = "compile-time literal regex; covered by unit tests"
    )]
    CELL.get_or_init(|| {
        Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[+-]\d{2}:\d{2})$").unwrap()
    })
}

fn round_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    #[expect(
        clippy::unwrap_used,
        reason = "compile-time literal regex; covered by unit tests"
    )]
    CELL.get_or_init(|| Regex::new(r"^[1-9][0-9]*$").unwrap())
}

/// Parse a journal file source into a [`JournalDoc`].
///
/// # Errors
///
/// Returns [`ParseError`] when frontmatter is missing or malformed,
/// any required attribute is absent, any attribute value fails the
/// schema regex, or the round counter sequence violates REQ-004
/// (first round must be 1, monotonic non-decreasing, no skips).
pub fn parse(source: &str, path: &Utf8Path) -> ParseResult<JournalDoc> {
    let split = split_frontmatter(source, path)?;
    let (yaml_raw, body, body_offset) = match split {
        Split::Some { yaml, body } => {
            let offset = source.len().checked_sub(body.len()).ok_or_else(|| {
                Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: 0,
                    reason: "frontmatter splitter produced an inconsistent body offset".to_owned(),
                })
            })?;
            (yaml.to_owned(), body, offset)
        }
        Split::None => {
            return Err(Box::new(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("journal file at {path}"),
            }));
        }
    };

    let spec = extract_yaml_field(&yaml_raw, "spec").ok_or_else(|| {
        Box::new(ParseError::MissingField {
            field: "spec".to_owned(),
            context: format!("journal frontmatter at {path}"),
        })
    })?;
    let task = extract_yaml_field(&yaml_raw, "task").ok_or_else(|| {
        Box::new(ParseError::MissingField {
            field: "task".to_owned(),
            context: format!("journal frontmatter at {path}"),
        })
    })?;
    let generated_at = extract_yaml_field(&yaml_raw, "generated_at").ok_or_else(|| {
        Box::new(ParseError::MissingField {
            field: "generated_at".to_owned(),
            context: format!("journal frontmatter at {path}"),
        })
    })?;

    if !iso8601_regex().is_match(&generated_at) {
        return Err(Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: "<frontmatter>".to_owned(),
            attribute: "generated_at".to_owned(),
            value: generated_at,
            reason: "expected ISO8601 timestamp `YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)`".to_owned(),
            offset: 0,
        }));
    }

    let code_fence_ranges = collect_code_fence_byte_ranges(source);
    let cfg = ScanConfig {
        whitelist: JOURNAL_ELEMENT_NAMES,
        structure_shaped_names: JOURNAL_ELEMENT_NAMES,
    };
    let raw_tags = scan_tags(source, body, body_offset, &code_fence_ranges, path, &cfg)?;

    let entries = assemble_entries(raw_tags, source, path)?;

    validate_round_sequence(&entries, path)?;

    Ok(JournalDoc {
        spec,
        task,
        generated_at,
        entries,
    })
}

fn assemble_entries(
    tags: Vec<RawTag>,
    source: &str,
    path: &Utf8Path,
) -> ParseResult<Vec<JournalEntry>> {
    let mut entries: Vec<JournalEntry> = Vec::new();
    let mut stack: Vec<RawTag> = Vec::new();
    for t in tags {
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
                        "close tag `</{}>` does not match open `<{}>`",
                        t.name, open.name
                    ),
                }));
            }
            if !stack.is_empty() {
                return Err(Box::new(ParseError::MalformedMarker {
                    path: path.to_path_buf(),
                    offset: open.span.start,
                    reason: "journal elements must not be nested".to_owned(),
                }));
            }
            let body = source
                .get(open.body_start..t.body_end_after_tag)
                .unwrap_or("")
                .to_owned();
            entries.push(build_entry(&open, body, path)?);
        } else {
            stack.push(t);
        }
    }
    if let Some(open) = stack.first() {
        return Err(Box::new(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: open.span.start,
            reason: format!("open tag `<{}>` is never closed", open.name),
        }));
    }
    Ok(entries)
}

fn build_entry(open: &RawTag, body: String, path: &Utf8Path) -> ParseResult<JournalEntry> {
    match open.name.as_str() {
        "implementer" => {
            let allowed: &[&str] = &["date", "model", "round"];
            require_only_allowed(open, allowed, path)?;
            let date = require_attr(open, "date", path)?;
            validate_iso8601(open, "date", &date, path)?;
            let model = require_attr(open, "model", path)?;
            if model.is_empty() {
                return Err(Box::new(ParseError::InvalidJournalAttribute {
                    path: path.to_path_buf(),
                    element: open.name.clone(),
                    attribute: "model".to_owned(),
                    value: model,
                    reason: "model must be a non-empty string".to_owned(),
                    offset: open.span.start,
                }));
            }
            let round_raw = require_attr(open, "round", path)?;
            let round = parse_round(open, &round_raw, path)?;
            Ok(JournalEntry::Implementer {
                date,
                model,
                round,
                body,
                span: open.span,
            })
        }
        "review" => {
            let allowed: &[&str] = &["date", "model", "persona", "verdict", "round"];
            require_only_allowed(open, allowed, path)?;
            let date = require_attr(open, "date", path)?;
            validate_iso8601(open, "date", &date, path)?;
            let model = require_attr(open, "model", path)?;
            if model.is_empty() {
                return Err(Box::new(ParseError::InvalidJournalAttribute {
                    path: path.to_path_buf(),
                    element: open.name.clone(),
                    attribute: "model".to_owned(),
                    value: model,
                    reason: "model must be a non-empty string".to_owned(),
                    offset: open.span.start,
                }));
            }
            let persona = require_attr(open, "persona", path)?;
            if !PERSONAS_ALL.contains(&persona.as_str()) {
                return Err(Box::new(ParseError::InvalidJournalAttribute {
                    path: path.to_path_buf(),
                    element: open.name.clone(),
                    attribute: "persona".to_owned(),
                    value: persona,
                    reason: format!("persona must be one of {}", PERSONAS_ALL.join(", ")),
                    offset: open.span.start,
                }));
            }
            let verdict = require_attr(open, "verdict", path)?;
            if !ALLOWED_REVIEW_VERDICTS.contains(&verdict.as_str()) {
                return Err(Box::new(ParseError::InvalidJournalAttribute {
                    path: path.to_path_buf(),
                    element: open.name.clone(),
                    attribute: "verdict".to_owned(),
                    value: verdict,
                    reason: format!(
                        "verdict must be one of {}",
                        ALLOWED_REVIEW_VERDICTS.join(", ")
                    ),
                    offset: open.span.start,
                }));
            }
            let round_raw = require_attr(open, "round", path)?;
            let round = parse_round(open, &round_raw, path)?;
            Ok(JournalEntry::Review {
                date,
                model,
                persona,
                verdict,
                round,
                body,
                span: open.span,
            })
        }
        "blockers" => {
            let allowed: &[&str] = &["date", "round"];
            require_only_allowed(open, allowed, path)?;
            let date = require_attr(open, "date", path)?;
            validate_iso8601(open, "date", &date, path)?;
            let round_raw = require_attr(open, "round", path)?;
            let round = parse_round(open, &round_raw, path)?;
            Ok(JournalEntry::Blockers {
                date,
                round,
                body,
                span: open.span,
            })
        }
        other => Err(Box::new(ParseError::UnknownMarkerName {
            path: path.to_path_buf(),
            marker_name: other.to_owned(),
            offset: open.span.start,
        })),
    }
}

fn require_attr(open: &RawTag, key: &str, path: &Utf8Path) -> ParseResult<String> {
    open.attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .ok_or_else(|| {
            Box::new(ParseError::MissingField {
                field: key.to_owned(),
                context: format!("<{}> in {path}", open.name),
            })
        })
}

fn require_only_allowed(open: &RawTag, allowed: &[&str], path: &Utf8Path) -> ParseResult<()> {
    for (k, _) in &open.attrs {
        if !allowed.contains(&k.as_str()) {
            return Err(Box::new(ParseError::UnknownMarkerAttribute {
                path: path.to_path_buf(),
                marker_name: open.name.clone(),
                attribute: k.clone(),
                offset: open.span.start,
                allowed: allowed.join(", "),
            }));
        }
    }
    Ok(())
}

fn validate_iso8601(open: &RawTag, attr: &str, value: &str, path: &Utf8Path) -> ParseResult<()> {
    if iso8601_regex().is_match(value) {
        return Ok(());
    }
    Err(Box::new(ParseError::InvalidJournalAttribute {
        path: path.to_path_buf(),
        element: open.name.clone(),
        attribute: attr.to_owned(),
        value: value.to_owned(),
        reason: "expected ISO8601 timestamp `YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)`".to_owned(),
        offset: open.span.start,
    }))
}

fn parse_round(open: &RawTag, raw: &str, path: &Utf8Path) -> ParseResult<u32> {
    if !round_regex().is_match(raw) {
        return Err(Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: "round".to_owned(),
            value: raw.to_owned(),
            reason: "round must be a positive integer (regex `[1-9][0-9]*`)".to_owned(),
            offset: open.span.start,
        }));
    }
    raw.parse::<u32>().map_err(|err| -> Box<ParseError> {
        Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: "round".to_owned(),
            value: raw.to_owned(),
            reason: format!("round overflows u32: {err}"),
            offset: open.span.start,
        })
    })
}

fn validate_round_sequence(entries: &[JournalEntry], path: &Utf8Path) -> ParseResult<()> {
    let Some(first) = entries.first() else {
        return Ok(());
    };
    if first.round() != 1 {
        return Err(Box::new(ParseError::InvalidJournalRoundSequence {
            path: path.to_path_buf(),
            reason: format!(
                "first block must have round=\"1\" (got round=\"{}\"); the first round counter must always start at 1",
                first.round()
            ),
        }));
    }
    let mut current = 1u32;
    for entry in entries {
        let r = entry.round();
        if r == current {
            continue;
        }
        if r == current.saturating_add(1) {
            current = r;
            continue;
        }
        if r < current {
            return Err(Box::new(ParseError::InvalidJournalRoundSequence {
                path: path.to_path_buf(),
                reason: format!(
                    "non-monotonic round counter: saw round=\"{r}\" after round=\"{current}\""
                ),
            }));
        }
        return Err(Box::new(ParseError::InvalidJournalRoundSequence {
            path: path.to_path_buf(),
            reason: format!(
                "round counter skipped: saw round=\"{r}\" after round=\"{current}\" with no intervening round=\"{}\"",
                current.saturating_add(1)
            ),
        }));
    }
    Ok(())
}

fn extract_yaml_field(yaml: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(prefix.as_str()) {
            return Some(
                rest.trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_owned(),
            );
        }
    }
    None
}

#[cfg(test)]
#[expect(
    clippy::panic,
    reason = "test-only assertions use panic! in match arms with descriptive messages"
)]
mod tests {
    use super::*;
    use camino::Utf8Path;
    use indoc::indoc;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/journal/T-001.md")
    }

    fn frontmatter() -> &'static str {
        indoc! {r"
            ---
            spec: SPEC-0042
            task: T-001
            generated_at: 2026-05-21T18:00:00Z
            ---

        "}
    }

    fn make(body: &str) -> String {
        format!("{}{}", frontmatter(), body)
    }

    #[test]
    fn happy_path_one_implementer() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="claude-opus-4.8[1m]/low" round="1">
            body
            </implementer>
        "#});
        let doc = parse(&src, path()).expect("parse");
        assert_eq!(doc.spec, "SPEC-0042");
        assert_eq!(doc.task, "T-001");
        assert_eq!(doc.entries.len(), 1);
        match doc.entries.first().expect("one entry") {
            JournalEntry::Implementer {
                date, model, round, ..
            } => {
                assert_eq!(date, "2026-05-21T18:00:00Z");
                assert_eq!(model, "claude-opus-4.8[1m]/low");
                assert_eq!(*round, 1);
            }
            other => panic!("expected implementer, got {other:?}"),
        }
    }

    #[test]
    fn date_only_is_rejected() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21" model="m" round="1">
            body
            </implementer>
        "#});
        let err = parse(&src, path()).expect_err("date-only must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalAttribute { attribute, .. } if attribute == "date"),
            "got {err:?}"
        );
    }

    #[test]
    fn empty_model_is_rejected() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="" round="1">
            body
            </implementer>
        "#});
        let err = parse(&src, path()).expect_err("empty model must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalAttribute { attribute, .. } if attribute == "model"),
            "got {err:?}"
        );
    }

    #[test]
    fn unknown_implementer_attribute_is_rejected() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="1" session="x">
            body
            </implementer>
        "#});
        let err = parse(&src, path()).expect_err("unknown session= must fail");
        assert!(
            matches!(err.as_ref(), ParseError::UnknownMarkerAttribute { attribute, allowed, .. } if attribute == "session" && allowed == "date, model, round"),
            "got {err:?}"
        );
    }

    #[test]
    fn invalid_verdict_is_rejected() {
        let src = make(indoc! {r#"
            <review date="2026-05-21T18:00:00Z" model="m" persona="tests" verdict="invalid" round="1">
            body
            </review>
        "#});
        let err = parse(&src, path()).expect_err("bad verdict must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalAttribute { attribute, .. } if attribute == "verdict"),
            "got {err:?}"
        );
    }

    #[test]
    fn blockers_with_required_attrs_parses() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="1">
            i
            </implementer>

            <blockers date="2026-05-21T18:30:00Z" round="1">
            do this
            </blockers>
        "#});
        let doc = parse(&src, path()).expect("parse");
        assert_eq!(doc.entries.len(), 2);
        assert!(matches!(
            doc.entries.get(1).expect("two entries"),
            JournalEntry::Blockers { .. }
        ));
    }

    #[test]
    fn first_round_must_be_1() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="2">
            body
            </implementer>
        "#});
        let err = parse(&src, path()).expect_err("first-round-2 must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalRoundSequence { reason, .. } if reason.contains("first block must have round=\"1\"")),
            "got {err:?}"
        );
    }

    #[test]
    fn round_skip_is_rejected() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="1">
            a
            </implementer>

            <implementer date="2026-05-21T19:00:00Z" model="m" round="3">
            c
            </implementer>
        "#});
        let err = parse(&src, path()).expect_err("round-skip must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalRoundSequence { reason, .. } if reason.contains("skipped")),
            "got {err:?}"
        );
    }

    #[test]
    fn non_monotonic_round_is_rejected() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="2">
            a
            </implementer>

            <implementer date="2026-05-21T19:00:00Z" model="m" round="1">
            b
            </implementer>
        "#});
        // First-round check fires first; that is also a violation.
        let err = parse(&src, path()).expect_err("non-monotonic must fail");
        assert!(matches!(
            err.as_ref(),
            ParseError::InvalidJournalRoundSequence { .. }
        ));
    }

    #[test]
    fn multiple_blocks_at_same_round_are_legal() {
        let src = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="1">
            i1
            </implementer>

            <review date="2026-05-21T18:10:00Z" model="m" persona="tests" verdict="pass" round="1">
            r1
            </review>

            <review date="2026-05-21T18:20:00Z" model="m" persona="business" verdict="pass" round="1">
            r2
            </review>

            <implementer date="2026-05-21T19:00:00Z" model="m" round="2">
            i2
            </implementer>
        "#});
        let doc = parse(&src, path()).expect("parse");
        assert_eq!(doc.entries.len(), 4);
    }

    #[test]
    fn missing_frontmatter_is_rejected() {
        let src = indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="1">
            body
            </implementer>
        "#};
        let err = parse(src, path()).expect_err("missing frontmatter must fail");
        assert!(
            matches!(err.as_ref(), ParseError::MissingField { field, .. } if field == "frontmatter"),
            "got {err:?}"
        );
    }
}
