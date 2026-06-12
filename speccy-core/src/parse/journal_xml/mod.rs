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

pub mod serialize;

use crate::error::ParseError;
use crate::error::ParseResult;
use crate::parse::frontmatter::split_required;
use crate::parse::journal_common::assemble_flat;
use crate::parse::journal_common::extract_yaml_field;
use crate::parse::journal_common::is_iso8601;
use crate::parse::journal_common::require_iso8601;
use crate::parse::journal_common::require_nonempty;
use crate::parse::journal_common::require_one_of;
use crate::parse::journal_common::require_only_allowed;
use crate::parse::journal_common::require_round;
use crate::parse::xml_scanner::ElementSpan;
use crate::parse::xml_scanner::RawTag;
use crate::parse::xml_scanner::ScanConfig;
use crate::parse::xml_scanner::collect_code_fence_byte_ranges;
use crate::parse::xml_scanner::scan_tags;
use crate::personas::ALL as PERSONAS_ALL;
use camino::Utf8Path;

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

/// The highest round across the entries, or `None` when there are none.
///
/// The single definition both journal views resolve "latest round"
/// through: `speccy journal show --round latest` and `speccy context`'s
/// inlined-journal section call this so the two cannot drift
/// (SPEC-0060 DEC-001).
#[must_use = "the resolved round selects which blocks to keep"]
pub fn latest_round(entries: &[JournalEntry]) -> Option<u32> {
    entries.iter().map(JournalEntry::round).max()
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
    let (yaml_raw, body, body_offset) = split_required(source, path, "journal file")?;

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

    if !is_iso8601(&generated_at) {
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

    let (entries, _body_ranges) = assemble_flat(
        raw_tags,
        source,
        path,
        "journal elements must not be nested",
        |open, body| build_entry(open, body, path),
    )?;

    validate_round_sequence(&entries, path)?;

    Ok(JournalDoc {
        spec,
        task,
        generated_at,
        entries,
    })
}

fn build_entry(open: &RawTag, body: String, path: &Utf8Path) -> ParseResult<JournalEntry> {
    match open.name.as_str() {
        "implementer" => {
            require_only_allowed(open, &["date", "model", "round"], path)?;
            let date = require_iso8601(open, "date", path)?;
            let model = require_nonempty(open, "model", path)?;
            let round = require_round(open, path)?;
            Ok(JournalEntry::Implementer {
                date,
                model,
                round,
                body,
                span: open.span,
            })
        }
        "review" => {
            require_only_allowed(
                open,
                &["date", "model", "persona", "verdict", "round"],
                path,
            )?;
            let date = require_iso8601(open, "date", path)?;
            let model = require_nonempty(open, "model", path)?;
            let persona = require_one_of(open, "persona", PERSONAS_ALL, path)?;
            let verdict = require_one_of(open, "verdict", ALLOWED_REVIEW_VERDICTS, path)?;
            let round = require_round(open, path)?;
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
            require_only_allowed(open, &["date", "round"], path)?;
            let date = require_iso8601(open, "date", path)?;
            let round = require_round(open, path)?;
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
    fn correctness_review_persona_parses_as_registry_valid() {
        // REQ-005 behavior: a `correctness` review block must be accepted
        // as registry-valid, not rejected as an unknown persona. This is
        // coupled to `personas::ALL` via the `PERSONAS_ALL.contains(...)`
        // validation; reverting that check to a hardcoded four-name list
        // would fail this test.
        let src = make(indoc! {r#"
            <review date="2026-05-21T18:00:00Z" model="m" persona="correctness" verdict="pass" round="1">
            body
            </review>
        "#});
        let doc = parse(&src, path()).expect("correctness review must parse");
        let persona = doc.entries.iter().find_map(|e| match e {
            JournalEntry::Review { persona, .. } => Some(persona.as_str()),
            _ => None,
        });
        assert_eq!(persona, Some("correctness"));
    }

    #[test]
    fn unknown_review_persona_is_rejected() {
        // The flip side of registry-coupling: a persona name absent from
        // `personas::ALL` must be rejected as an unknown persona.
        let src = make(indoc! {r#"
            <review date="2026-05-21T18:00:00Z" model="m" persona="nonsense" verdict="pass" round="1">
            body
            </review>
        "#});
        let err = parse(&src, path()).expect_err("unknown persona must fail");
        assert!(
            matches!(
                err.as_ref(),
                ParseError::InvalidJournalAttribute { attribute, value, .. }
                    if attribute == "persona" && value == "nonsense"
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn latest_round_resolves_highest_and_handles_empty() {
        let two = make(indoc! {r#"
            <implementer date="2026-05-21T18:00:00Z" model="m" round="1">
            a
            </implementer>

            <implementer date="2026-05-21T19:00:00Z" model="m" round="2">
            b
            </implementer>
        "#});
        let doc = parse(&two, path()).expect("parse two rounds");
        assert_eq!(latest_round(&doc.entries), Some(2));

        let empty = make("");
        let doc = parse(&empty, path()).expect("parse zero entries");
        assert!(doc.entries.is_empty(), "fixture has no blocks");
        assert_eq!(latest_round(&doc.entries), None);
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
