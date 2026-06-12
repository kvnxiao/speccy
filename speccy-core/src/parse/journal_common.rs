//! Shared helpers for the journal-family parsers — the per-task journal
//! (`journal/T-NNN.md`, [`crate::parse::journal_xml`]) and the vet
//! journal (`journal/VET.md`, [`crate::parse::vet_xml`]).
//!
//! Both files carry YAML frontmatter followed by a flat (non-nesting)
//! sequence of line-isolated XML blocks whose attributes share one
//! schema vocabulary (`date`, `round`, `model`, `verdict`), so the
//! fetch-and-validate helpers, the value regexes, and the flat block
//! assembler live here once.

use crate::error::ParseError;
use crate::error::ParseResult;
use crate::parse::xml_scanner::RawTag;
use camino::Utf8Path;
use regex::Regex;
use std::sync::OnceLock;

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

/// Whether `value` is an ISO8601 timestamp with seconds and timezone
/// designator (`YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)`). Used directly for
/// frontmatter `generated_at` values, which have no carrying [`RawTag`].
pub fn is_iso8601(value: &str) -> bool {
    iso8601_regex().is_match(value)
}

/// Fetch a required attribute value from `open`'s attribute list.
pub fn require_attr(open: &RawTag, key: &str, path: &Utf8Path) -> ParseResult<String> {
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

/// Reject any attribute on `open` outside the `allowed` set.
pub fn require_only_allowed(open: &RawTag, allowed: &[&str], path: &Utf8Path) -> ParseResult<()> {
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

/// Fetch a required attribute whose value must be one of `allowed`.
pub fn require_one_of(
    open: &RawTag,
    attr: &str,
    allowed: &[&str],
    path: &Utf8Path,
) -> ParseResult<String> {
    let value = require_attr(open, attr, path)?;
    if !allowed.contains(&value.as_str()) {
        return Err(Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: attr.to_owned(),
            value,
            reason: format!("{attr} must be one of {}", allowed.join(", ")),
            offset: open.span.start,
        }));
    }
    Ok(value)
}

/// Fetch a required attribute that must be an ISO8601 timestamp.
pub fn require_iso8601(open: &RawTag, attr: &str, path: &Utf8Path) -> ParseResult<String> {
    let value = require_attr(open, attr, path)?;
    if !is_iso8601(&value) {
        return Err(Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: attr.to_owned(),
            value,
            reason: "expected ISO8601 timestamp `YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)`".to_owned(),
            offset: open.span.start,
        }));
    }
    Ok(value)
}

/// Fetch a required attribute that must be a non-empty string.
pub fn require_nonempty(open: &RawTag, attr: &str, path: &Utf8Path) -> ParseResult<String> {
    let value = require_attr(open, attr, path)?;
    if value.is_empty() {
        return Err(Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: attr.to_owned(),
            value,
            reason: format!("{attr} must be a non-empty string"),
            offset: open.span.start,
        }));
    }
    Ok(value)
}

/// Fetch the required `round` attribute as a positive integer.
pub fn require_round(open: &RawTag, path: &Utf8Path) -> ParseResult<u32> {
    let raw = require_attr(open, "round", path)?;
    if !round_regex().is_match(&raw) {
        return Err(Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: "round".to_owned(),
            value: raw,
            reason: "round must be a positive integer (regex `[1-9][0-9]*`)".to_owned(),
            offset: open.span.start,
        }));
    }
    raw.parse::<u32>().map_err(|err| -> Box<ParseError> {
        Box::new(ParseError::InvalidJournalAttribute {
            path: path.to_path_buf(),
            element: open.name.clone(),
            attribute: "round".to_owned(),
            value: raw.clone(),
            reason: format!("round overflows u32: {err}"),
            offset: open.span.start,
        })
    })
}

/// Extract a top-level scalar field from a YAML frontmatter payload by
/// line prefix, stripping surrounding quotes. Deliberately narrow — the
/// journal frontmatter schema is three flat string fields, so no YAML
/// crate is involved.
pub fn extract_yaml_field(yaml: &str, field: &str) -> Option<String> {
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

/// Result of [`assemble_flat`]: the typed blocks plus each block's
/// inner-body byte range (half-open `[start, end)` spans into the
/// source). Callers that don't need the ranges ignore them.
pub type AssembledFlat<T> = (Vec<T>, Vec<(usize, usize)>);

/// Assemble line-isolated open/close tag pairs into typed blocks via
/// `build`, enforcing the journal-family flat structure: blocks never
/// nest (`nested_reason` names the violation in the diagnostic), every
/// close matches the innermost open, and nothing is left unclosed.
pub fn assemble_flat<T>(
    tags: Vec<RawTag>,
    source: &str,
    path: &Utf8Path,
    nested_reason: &str,
    mut build: impl FnMut(&RawTag, String) -> ParseResult<T>,
) -> ParseResult<AssembledFlat<T>> {
    let mut blocks: Vec<T> = Vec::new();
    let mut body_ranges: Vec<(usize, usize)> = Vec::new();
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
                    reason: nested_reason.to_owned(),
                }));
            }
            let body = source
                .get(open.body_start..t.body_end_after_tag)
                .unwrap_or("")
                .to_owned();
            body_ranges.push((open.body_start, t.body_end_after_tag));
            blocks.push(build(&open, body)?);
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
    Ok((blocks, body_ranges))
}
