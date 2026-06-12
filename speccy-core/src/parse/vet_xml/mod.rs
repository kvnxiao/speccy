//! Parser for the pre-ship vet journal (`journal/VET.md`).
//!
//! SPEC-0055 REQ-004 freezes the VET.md grammar that until now lived
//! only in skill prose and the tolerant `<gate>` scanner in
//! [`crate::next`]. The file shape is YAML frontmatter (`spec`,
//! `generated_at`) followed by one or more `## Invocation N — <ISO8601>`
//! sections, each holding a chronological sequence of vet blocks that
//! terminates in exactly one `<gate>`.
//!
//! Block attribute schemas (SPEC-0055 REQ-004):
//! - `<drift-review>`: `verdict` (`pass|blocking`), `round`, `date`, `model`
//!   (all required).
//! - `<holistic-fix>`: `verdict` (`addressed|blocking|stuck`), `round`, `date`,
//!   `model` (all required).
//! - `<simplifier-scan>`: `verdict` (`clean|candidates`) only.
//! - `<simplifier-apply>`: `verdict` (`applied|blocking`) only.
//! - `<gate>`: `verdict` (`passed|failed`), `tasks_hash`, `date` — the terminal
//!   block, exactly one per invocation section and the last block in its
//!   section.
//!
//! `date` is ISO8601 with seconds and timezone designator; `round` is a
//! positive integer that resets per invocation section; `model` is a
//! non-empty string (the slash-suffix effort convention is not
//! parser-validated, matching [`crate::parse::journal_xml`]).
//!
//! This module is the source of truth for the grammar (DEC-005). The
//! tolerant `<gate>` scanner in [`crate::next`] keeps its independent
//! read path for the freshness check; both are expected to agree on the
//! final passing gate.

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
use crate::parse::xml_scanner::is_html5_element_name;
use crate::parse::xml_scanner::scan_tags;
use camino::Utf8Path;
use regex::Regex;
use std::sync::OnceLock;

/// Closed set of `<drift-review verdict="...">` values.
pub const DRIFT_REVIEW_VERDICTS: &[&str] = &["pass", "blocking"];

/// Closed set of `<holistic-fix verdict="...">` values.
pub const HOLISTIC_FIX_VERDICTS: &[&str] = &["addressed", "blocking", "stuck"];

/// Closed set of `<simplifier-scan verdict="...">` values.
pub const SIMPLIFIER_SCAN_VERDICTS: &[&str] = &["clean", "candidates"];

/// Closed set of `<simplifier-apply verdict="...">` values.
pub const SIMPLIFIER_APPLY_VERDICTS: &[&str] = &["applied", "blocking"];

/// Closed set of `<gate verdict="...">` values.
pub const GATE_VERDICTS: &[&str] = &["passed", "failed"];

/// Closed whitelist of element names recognised inside a VET.md file.
pub const VET_ELEMENT_NAMES: &[&str] = &[
    "drift-review",
    "holistic-fix",
    "simplifier-scan",
    "simplifier-apply",
    "gate",
];

/// Parsed pre-ship vet journal file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VetDoc {
    /// `spec:` field from the frontmatter, e.g. `SPEC-0042`.
    pub spec: String,
    /// `generated_at:` field from the frontmatter.
    pub generated_at: String,
    /// Invocation sections in document order.
    pub invocations: Vec<Invocation>,
}

/// One `## Invocation N — <ISO8601>` section and its blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The `N` in `## Invocation N`.
    pub number: u32,
    /// The ISO8601 datetime on the heading line.
    pub date: String,
    /// Blocks in document order; terminates in exactly one `Gate`.
    pub blocks: Vec<VetBlock>,
}

/// One vet block inside an invocation section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VetBlock {
    /// `<drift-review verdict="pass|blocking" round date model>`.
    DriftReview {
        /// `pass` or `blocking`.
        verdict: String,
        /// Positive integer round counter (resets per section).
        round: u32,
        /// ISO8601 timestamp.
        date: String,
        /// Free-form model identifier.
        model: String,
        /// Verbatim body.
        body: String,
        /// Span of the open tag.
        span: ElementSpan,
    },
    /// `<holistic-fix verdict="addressed|blocking|stuck" round date model>`.
    HolisticFix {
        /// `addressed`, `blocking`, or `stuck`.
        verdict: String,
        /// Positive integer round counter (resets per section).
        round: u32,
        /// ISO8601 timestamp.
        date: String,
        /// Free-form model identifier.
        model: String,
        /// Verbatim body.
        body: String,
        /// Span of the open tag.
        span: ElementSpan,
    },
    /// `<simplifier-scan verdict="clean|candidates">`.
    SimplifierScan {
        /// `clean` or `candidates`.
        verdict: String,
        /// Verbatim body.
        body: String,
        /// Span of the open tag.
        span: ElementSpan,
    },
    /// `<simplifier-apply verdict="applied|blocking">`.
    SimplifierApply {
        /// `applied` or `blocking`.
        verdict: String,
        /// Verbatim body.
        body: String,
        /// Span of the open tag.
        span: ElementSpan,
    },
    /// `<gate verdict="passed|failed" tasks_hash date>` — terminal.
    Gate {
        /// `passed` or `failed`.
        verdict: String,
        /// Lowercase hex SHA-256 of the sibling TASKS.md at append time.
        tasks_hash: String,
        /// ISO8601 timestamp.
        date: String,
        /// Verbatim body.
        body: String,
        /// Span of the open tag.
        span: ElementSpan,
    },
}

impl VetBlock {
    /// The element local name, used in diagnostics.
    #[must_use = "the element name is used in diagnostics"]
    pub fn element_name(&self) -> &'static str {
        match self {
            VetBlock::DriftReview { .. } => "drift-review",
            VetBlock::HolisticFix { .. } => "holistic-fix",
            VetBlock::SimplifierScan { .. } => "simplifier-scan",
            VetBlock::SimplifierApply { .. } => "simplifier-apply",
            VetBlock::Gate { .. } => "gate",
        }
    }

    /// Byte offset of this block's open tag in the source.
    #[must_use = "the offset is used to partition blocks into sections"]
    fn offset(&self) -> usize {
        match self {
            VetBlock::DriftReview { span, .. }
            | VetBlock::HolisticFix { span, .. }
            | VetBlock::SimplifierScan { span, .. }
            | VetBlock::SimplifierApply { span, .. }
            | VetBlock::Gate { span, .. } => span.start,
        }
    }

    /// The round counter for blocks that carry one (`drift-review`,
    /// `holistic-fix`); `None` for the round-less block types.
    #[must_use = "the round drives per-section sequence validation"]
    fn round(&self) -> Option<u32> {
        match self {
            VetBlock::DriftReview { round, .. } | VetBlock::HolisticFix { round, .. } => {
                Some(*round)
            }
            VetBlock::SimplifierScan { .. }
            | VetBlock::SimplifierApply { .. }
            | VetBlock::Gate { .. } => None,
        }
    }
}

/// Matches a line-isolated open tag (`<name ...>`), capturing the
/// element name. Used to detect tags the shared scanner deliberately
/// flows through as Markdown body so this parser can reject unknown
/// vet-block names rather than silently swallow them.
fn open_tag_name_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    #[expect(
        clippy::unwrap_used,
        reason = "compile-time literal regex; covered by unit tests"
    )]
    CELL.get_or_init(|| Regex::new(r"^<([a-z][a-z0-9-]*)(\s[^>]*)?>$").unwrap())
}

/// Matches an `## Invocation N — <ISO8601>` heading line, capturing the
/// invocation number and the datetime. The separator between the number
/// and the date is an em dash (`—`) in the shape the vet skill produces;
/// a plain hyphen is also accepted to stay tolerant of hand edits.
fn invocation_heading_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    #[expect(
        clippy::unwrap_used,
        reason = "compile-time literal regex; covered by unit tests"
    )]
    CELL.get_or_init(|| {
        Regex::new(r"(?m)^##\s+Invocation\s+([1-9][0-9]*)\s+(?:—|-)\s+(\S+)\s*$").unwrap()
    })
}

/// A located invocation heading: its byte offset in the source plus the
/// parsed number and datetime.
struct HeadingMatch {
    offset: usize,
    number: u32,
    date: String,
}

/// Whether the document's *last* invocation section is required to end in
/// a terminal `<gate>`, or may be left open.
///
/// The two parse entry points ([`parse`] and [`parse_in_flight`]) differ
/// only by this flag (DEC-008): strict parsing rejects an un-gated last
/// section (the complete-file grammar `speccy verify` relies on), while
/// in-flight parsing tolerates it (the shape that exists mid-vet-run, after
/// a `drift-review` and before its `gate`, which `journal append`'s
/// derivation and `journal show`'s reads both need). Every other rule —
/// frontmatter, tag/heading scanning, block
/// assembly, round sequencing, and the gate rules for *non-last* sections —
/// is identical between the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LastSection {
    /// The last section must end in a terminal `<gate>` (strict grammar).
    MustBeGated,
    /// The last section may lack a `<gate>` (in-flight derivation).
    MayBeOpen,
}

/// Parse a *complete* VET.md source into a [`VetDoc`] under the strict
/// grammar: every invocation section, including the last, must end in a
/// terminal `<gate>`.
///
/// This is the authority for complete files — `speccy verify`. For reading
/// or deriving state from an in-flight file whose last section is still
/// open (the mid-vet-run shape `journal show` and `journal append` both
/// see), use [`parse_in_flight`].
///
/// # Errors
///
/// Returns [`ParseError`] when frontmatter is missing or malformed, the
/// document holds no `## Invocation` heading, any block has a missing or
/// unknown attribute or an out-of-domain verdict, a block falls outside
/// any invocation section, or the per-section round sequence is invalid.
/// A section's gate-structure violation (no terminal `gate`, a
/// non-terminal `gate`, or a second `gate`) returns the dedicated
/// [`ParseError::VetGateStructure`] so callers can distinguish it from
/// other grammar failures (SPEC-0055 REQ-007 routes it to `VET-002`).
pub fn parse(source: &str, path: &Utf8Path) -> ParseResult<VetDoc> {
    parse_with_mode(source, path, LastSection::MustBeGated)
}

/// Parse an *in-flight* VET.md source into a [`VetDoc`], tolerating an
/// open (un-gated) last invocation section (DEC-008).
///
/// Identical to [`parse`] in every respect except that the last section is
/// allowed to lack a terminal `<gate>` — the shape a VET.md has mid-vet-run
/// after a `drift-review` is appended but before its `gate`. Used by
/// `journal append`'s vet path to derive invocation/round state from the
/// existing file and to validate the would-be-new file before writing, so
/// the parser is the single authority over both derivation and what may
/// land on disk (no separate tolerant scan or body-markup guard). Also used
/// by `journal show` for VET.md, whose vet-flow call sites read the journal
/// mid-run while the last section is still open.
///
/// A *complete* (fully gated) file parses identically under this function
/// and [`parse`]: the relaxation only ever exempts an un-gated last
/// section, so a gated last section is validated in full.
///
/// # Errors
///
/// Returns the same [`ParseError`]s as [`parse`], except that an un-gated
/// *last* section is accepted. An un-gated *non-last* section, a
/// non-terminal or duplicate `gate` in any section, and every other
/// grammar violation are still rejected.
pub fn parse_in_flight(source: &str, path: &Utf8Path) -> ParseResult<VetDoc> {
    parse_with_mode(source, path, LastSection::MayBeOpen)
}

/// Shared parse pipeline behind [`parse`] and [`parse_in_flight`]; see
/// [`LastSection`] for the single behavioural difference.
fn parse_with_mode(
    source: &str,
    path: &Utf8Path,
    last_section: LastSection,
) -> ParseResult<VetDoc> {
    let (yaml_raw, body, body_offset) = split_required(source, path, "vet journal")?;

    let spec = extract_yaml_field(&yaml_raw, "spec").ok_or_else(|| {
        Box::new(ParseError::MissingField {
            field: "spec".to_owned(),
            context: format!("vet journal frontmatter at {path}"),
        })
    })?;
    let generated_at = extract_yaml_field(&yaml_raw, "generated_at").ok_or_else(|| {
        Box::new(ParseError::MissingField {
            field: "generated_at".to_owned(),
            context: format!("vet journal frontmatter at {path}"),
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
        whitelist: VET_ELEMENT_NAMES,
        structure_shaped_names: VET_ELEMENT_NAMES,
    };
    reject_unknown_block_tags(body, body_offset, &code_fence_ranges, path)?;
    let raw_tags = scan_tags(source, body, body_offset, &code_fence_ranges, path, &cfg)?;
    let (blocks, body_ranges) = assemble_flat(
        raw_tags,
        source,
        path,
        "vet blocks must not be nested",
        |open, block_body| build_block(open, block_body, path),
    )?;

    // A `## Invocation N — …` line is only a real section heading when it sits
    // in free document body — not inside a fenced code block (`code_fence_ranges`)
    // and not inside a vet block body (`body_ranges`). This mirrors the
    // fence-awareness `reject_unknown_block_tags` already has.
    let headings = collect_invocation_headings(source, &code_fence_ranges, &body_ranges);
    let invocations = partition_into_sections(&headings, blocks, path, last_section)?;

    Ok(VetDoc {
        spec,
        generated_at,
        invocations,
    })
}

/// Reject any line-isolated open tag whose name is neither a recognised
/// vet block nor a plain HTML5 element.
///
/// The shared scanner ([`scan_tags`]) deliberately flows non-whitelisted
/// tags through as Markdown body (so foreign HTML like `<details>`
/// survives). VET.md is a closed grammar, though: a speccy-shaped tag
/// like `<drift-revue>` is a typo'd block, not prose, and the SPEC-0055
/// REQ-004 grammar must reject it loudly rather than silently drop it.
/// HTML5 element names stay tolerated to match the workspace-wide
/// foreign-HTML contract.
fn reject_unknown_block_tags(
    body: &str,
    body_offset: usize,
    code_fence_ranges: &[(usize, usize)],
    path: &Utf8Path,
) -> ParseResult<()> {
    let mut cursor = 0usize;
    for line in body.split_inclusive('\n') {
        let line_start = body_offset.saturating_add(cursor);
        cursor = cursor.saturating_add(line.len());
        // Skip lines inside fenced code blocks.
        if code_fence_ranges
            .iter()
            .any(|(s, e)| line_start >= *s && line_start < *e)
        {
            continue;
        }
        let trimmed = line.trim();
        let Some(caps) = open_tag_name_regex().captures(trimmed) else {
            continue;
        };
        let Some(name) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        if VET_ELEMENT_NAMES.contains(&name) || is_html5_element_name(name) {
            continue;
        }
        return Err(Box::new(ParseError::UnknownMarkerName {
            path: path.to_path_buf(),
            marker_name: name.to_owned(),
            offset: line_start,
        }));
    }
    Ok(())
}

fn build_block(open: &RawTag, body: String, path: &Utf8Path) -> ParseResult<VetBlock> {
    match open.name.as_str() {
        "drift-review" => {
            require_only_allowed(open, &["verdict", "round", "date", "model"], path)?;
            let verdict = require_one_of(open, "verdict", DRIFT_REVIEW_VERDICTS, path)?;
            let round = require_round(open, path)?;
            let date = require_iso8601(open, "date", path)?;
            let model = require_nonempty(open, "model", path)?;
            Ok(VetBlock::DriftReview {
                verdict,
                round,
                date,
                model,
                body,
                span: open.span,
            })
        }
        "holistic-fix" => {
            require_only_allowed(open, &["verdict", "round", "date", "model"], path)?;
            let verdict = require_one_of(open, "verdict", HOLISTIC_FIX_VERDICTS, path)?;
            let round = require_round(open, path)?;
            let date = require_iso8601(open, "date", path)?;
            let model = require_nonempty(open, "model", path)?;
            Ok(VetBlock::HolisticFix {
                verdict,
                round,
                date,
                model,
                body,
                span: open.span,
            })
        }
        "simplifier-scan" => {
            require_only_allowed(open, &["verdict"], path)?;
            let verdict = require_one_of(open, "verdict", SIMPLIFIER_SCAN_VERDICTS, path)?;
            Ok(VetBlock::SimplifierScan {
                verdict,
                body,
                span: open.span,
            })
        }
        "simplifier-apply" => {
            require_only_allowed(open, &["verdict"], path)?;
            let verdict = require_one_of(open, "verdict", SIMPLIFIER_APPLY_VERDICTS, path)?;
            Ok(VetBlock::SimplifierApply {
                verdict,
                body,
                span: open.span,
            })
        }
        "gate" => {
            require_only_allowed(open, &["verdict", "tasks_hash", "date"], path)?;
            let verdict = require_one_of(open, "verdict", GATE_VERDICTS, path)?;
            let tasks_hash = require_nonempty(open, "tasks_hash", path)?;
            let date = require_iso8601(open, "date", path)?;
            Ok(VetBlock::Gate {
                verdict,
                tasks_hash,
                date,
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

/// Collect the `## Invocation N — <date>` section headings from free
/// document body, ignoring any heading-shaped line that lands inside a
/// fenced code block (`code_fence_ranges`) or inside a vet block body
/// (`block_body_ranges`). Both range slices are half-open `[start, end)`
/// byte spans into `source`.
fn collect_invocation_headings(
    source: &str,
    code_fence_ranges: &[(usize, usize)],
    block_body_ranges: &[(usize, usize)],
) -> Vec<HeadingMatch> {
    let in_any_range = |offset: usize, ranges: &[(usize, usize)]| {
        ranges.iter().any(|(s, e)| offset >= *s && offset < *e)
    };
    invocation_heading_regex()
        .captures_iter(source)
        .filter_map(|caps| {
            let offset = caps.get(0)?.start();
            if in_any_range(offset, code_fence_ranges) || in_any_range(offset, block_body_ranges) {
                return None;
            }
            let number = caps.get(1)?.as_str().parse::<u32>().ok()?;
            let date = caps.get(2)?.as_str().to_owned();
            Some(HeadingMatch {
                offset,
                number,
                date,
            })
        })
        .collect()
}

/// Assign each block to the invocation section that immediately precedes
/// it, then validate per-section structure (round sequence and the
/// terminal-gate rules).
fn partition_into_sections(
    headings: &[HeadingMatch],
    blocks: Vec<VetBlock>,
    path: &Utf8Path,
    last_section: LastSection,
) -> ParseResult<Vec<Invocation>> {
    let Some(first) = headings.first() else {
        return Err(Box::new(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: 0,
            reason: "vet journal has no `## Invocation N — <date>` section heading".to_owned(),
        }));
    };

    // Reject a block that appears before the first invocation heading.
    if let Some(stray) = blocks.iter().find(|b| b.offset() < first.offset) {
        return Err(Box::new(ParseError::MalformedMarker {
            path: path.to_path_buf(),
            offset: stray.offset(),
            reason: format!(
                "<{}> block appears before any `## Invocation` section heading",
                stray.element_name()
            ),
        }));
    }

    let mut invocations: Vec<Invocation> = headings
        .iter()
        .map(|h| Invocation {
            number: h.number,
            date: h.date.clone(),
            blocks: Vec::new(),
        })
        .collect();

    for block in blocks {
        // The owning section is the last heading whose offset precedes
        // the block. `headings` is in document order, so a reverse scan
        // finds it; the pre-first-heading case is already rejected.
        let idx = headings
            .iter()
            .rposition(|h| h.offset <= block.offset())
            .unwrap_or(0);
        if let Some(inv) = invocations.get_mut(idx) {
            inv.blocks.push(block);
        }
    }

    let last_idx = invocations.len().saturating_sub(1);
    for (idx, inv) in invocations.iter().enumerate() {
        // Only the final section may be left open, and only in in-flight mode.
        let allow_open = idx == last_idx && matches!(last_section, LastSection::MayBeOpen);
        validate_section(inv, path, allow_open)?;
    }
    Ok(invocations)
}

/// Validate one section's round sequence and gate placement.
///
/// `allow_open` exempts a section with *no* `<gate>` from the
/// "no terminal gate" error — set only for the last section in in-flight
/// mode (DEC-008). A section that *does* carry a gate is validated in full
/// regardless (the gate must be terminal and unique), so the relaxation
/// never weakens the gate rules for a closed section.
fn validate_section(inv: &Invocation, path: &Utf8Path, allow_open: bool) -> ParseResult<()> {
    // Round sequence: the rounds carried by drift-review / holistic-fix
    // blocks reset per section, start at 1, and advance monotonically
    // with no skips.
    validate_round_sequence(inv, path)?;

    // Gate placement: exactly one gate, and it is the last block.
    let gate_positions: Vec<usize> = inv
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| matches!(b, VetBlock::Gate { .. }).then_some(i))
        .collect();
    match gate_positions.as_slice() {
        // An open last section (in-flight derivation) carries no gate yet.
        [] if allow_open => Ok(()),
        [] => Err(Box::new(ParseError::VetGateStructure {
            path: path.to_path_buf(),
            offset: section_offset(inv),
            reason: format!(
                "invocation {} section has no terminal `<gate>` block",
                inv.number
            ),
        })),
        [gate_idx] => {
            if *gate_idx + 1 != inv.blocks.len() {
                return Err(Box::new(ParseError::VetGateStructure {
                    path: path.to_path_buf(),
                    offset: section_offset(inv),
                    reason: format!(
                        "invocation {} section has a `<gate>` block that is not the last block in its section",
                        inv.number
                    ),
                }));
            }
            Ok(())
        }
        _ => Err(Box::new(ParseError::VetGateStructure {
            path: path.to_path_buf(),
            offset: section_offset(inv),
            reason: format!(
                "invocation {} section has {} `<gate>` blocks; exactly one is allowed",
                inv.number,
                gate_positions.len()
            ),
        })),
    }
}

fn validate_round_sequence(inv: &Invocation, path: &Utf8Path) -> ParseResult<()> {
    let mut current: Option<u32> = None;
    for block in &inv.blocks {
        let Some(r) = block.round() else { continue };
        match current {
            None => {
                if r != 1 {
                    return Err(Box::new(ParseError::InvalidJournalRoundSequence {
                        path: path.to_path_buf(),
                        reason: format!(
                            "invocation {} section: first round-bearing block must have round=\"1\" (got round=\"{r}\")",
                            inv.number
                        ),
                    }));
                }
                current = Some(1);
            }
            Some(cur) => {
                if r == cur {
                    continue;
                }
                if r == cur.saturating_add(1) {
                    current = Some(r);
                    continue;
                }
                if r < cur {
                    return Err(Box::new(ParseError::InvalidJournalRoundSequence {
                        path: path.to_path_buf(),
                        reason: format!(
                            "invocation {} section: non-monotonic round counter (saw round=\"{r}\" after round=\"{cur}\")",
                            inv.number
                        ),
                    }));
                }
                return Err(Box::new(ParseError::InvalidJournalRoundSequence {
                    path: path.to_path_buf(),
                    reason: format!(
                        "invocation {} section: round counter skipped (saw round=\"{r}\" after round=\"{cur}\")",
                        inv.number
                    ),
                }));
            }
        }
    }
    Ok(())
}

fn section_offset(inv: &Invocation) -> usize {
    inv.blocks.first().map_or(0, VetBlock::offset)
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
        Utf8Path::new("fixture/journal/VET.md")
    }

    fn frontmatter() -> &'static str {
        indoc! {r"
            ---
            spec: SPEC-0042
            generated_at: 2026-05-21T18:00:00Z
            ---

        "}
    }

    fn make(body: &str) -> String {
        format!("{}{}", frontmatter(), body)
    }

    /// The full-grammar happy-path fixture: frontmatter, one invocation
    /// section, all five block types, terminal gate.
    fn full_grammar_fixture() -> String {
        make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="blocking" round="1" date="2026-05-21T18:00:00Z" model="m">
            drift found
            </drift-review>

            <holistic-fix verdict="addressed" round="1" date="2026-05-21T18:05:00Z" model="m">
            fixed
            </holistic-fix>

            <simplifier-scan verdict="candidates">
            candidates
            </simplifier-scan>

            <simplifier-apply verdict="applied">
            applied
            </simplifier-apply>

            <gate verdict="passed" tasks_hash="abc123" date="2026-05-21T18:10:00Z">
            shipping
            </gate>
        "#})
    }

    #[test]
    fn full_grammar_parses_with_structure_intact() {
        let doc = parse(&full_grammar_fixture(), path()).expect("full-grammar fixture must parse");
        assert_eq!(doc.spec, "SPEC-0042");
        assert_eq!(doc.invocations.len(), 1);
        let inv = doc.invocations.first().expect("one invocation");
        assert_eq!(inv.number, 1);
        assert_eq!(inv.date, "2026-05-21T18:00:00Z");
        assert_eq!(inv.blocks.len(), 5);
        let names: Vec<&str> = inv.blocks.iter().map(VetBlock::element_name).collect();
        assert_eq!(
            names,
            vec![
                "drift-review",
                "holistic-fix",
                "simplifier-scan",
                "simplifier-apply",
                "gate",
            ]
        );
        match inv.blocks.first().expect("drift-review") {
            VetBlock::DriftReview {
                verdict,
                round,
                model,
                ..
            } => {
                assert_eq!(verdict, "blocking");
                assert_eq!(*round, 1);
                assert_eq!(model, "m");
            }
            other => panic!("expected drift-review, got {other:?}"),
        }
        match inv.blocks.get(4).expect("gate") {
            VetBlock::Gate {
                verdict,
                tasks_hash,
                ..
            } => {
                assert_eq!(verdict, "passed");
                assert_eq!(tasks_hash, "abc123");
            }
            other => panic!("expected gate, got {other:?}"),
        }
    }

    #[test]
    fn two_invocation_sections_partition_by_heading() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m">
            ok
            </drift-review>

            <gate verdict="passed" tasks_hash="h1" date="2026-05-21T18:10:00Z">
            g1
            </gate>

            ## Invocation 2 — 2026-05-21T19:00:00Z

            <simplifier-scan verdict="clean">
            clean
            </simplifier-scan>

            <gate verdict="passed" tasks_hash="h2" date="2026-05-21T19:10:00Z">
            g2
            </gate>
        "#});
        let doc = parse(&src, path()).expect("two-section fixture must parse");
        assert_eq!(doc.invocations.len(), 2);
        let first = doc.invocations.first().expect("inv 1");
        assert_eq!(first.blocks.len(), 2);
        let second = doc.invocations.get(1).expect("inv 2");
        assert_eq!(second.number, 2);
        assert_eq!(second.blocks.len(), 2);
        assert!(matches!(
            second.blocks.first().expect("scan"),
            VetBlock::SimplifierScan { .. }
        ));
    }

    #[test]
    fn heading_shaped_line_in_block_body_or_fence_is_not_a_section() {
        // A `## Invocation N — <token>` line inside a block body — whether
        // as free prose or wrapped in a ``` fence — is content, not a real
        // section heading. Heading detection must be fence/body-aware (like
        // `reject_unknown_block_tags`) so the surrounding section is not
        // mis-partitioned and spuriously rejected for "no terminal gate".
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m">
            quoting the prior section verbatim:
            ## Invocation 2 — 2026-05-21T19:00:00Z
            and inside a fence:
            ```
            ## Invocation 3 — 2026-05-21T20:00:00Z
            ```
            </drift-review>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let doc = parse(&src, path())
            .expect("heading-shaped lines inside a block body must not open new sections");
        // Exactly one real section, from the only free-body heading.
        assert_eq!(doc.invocations.len(), 1);
        let inv = doc.invocations.first().expect("one invocation");
        assert_eq!(inv.number, 1);
        // Both blocks land in that single section; the body keeps the
        // heading-shaped text verbatim rather than being split off.
        assert_eq!(inv.blocks.len(), 2);
        match inv.blocks.first().expect("drift-review") {
            VetBlock::DriftReview { body, .. } => {
                assert!(
                    body.contains("## Invocation 2 —") && body.contains("## Invocation 3 —"),
                    "body should retain heading-shaped lines verbatim, got {body:?}"
                );
            }
            other => panic!("expected drift-review, got {other:?}"),
        }
    }

    #[test]
    fn unknown_block_is_rejected() {
        // A speccy-shaped block name outside the closed grammar (here a
        // typo'd `drift-revue`) must be rejected loudly, not silently
        // dropped as Markdown body the way the shared scanner would.
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-revue verdict="pass">
            body
            </drift-revue>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("unknown block must fail");
        assert!(
            matches!(err.as_ref(), ParseError::UnknownMarkerName { marker_name, .. } if marker_name == "drift-revue"),
            "got {err:?}"
        );
    }

    #[test]
    fn foreign_html_in_body_is_tolerated() {
        // A plain HTML5 element inside a block body must flow through as
        // prose, matching the workspace-wide foreign-HTML contract.
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m">
            <details>
            collapsible note
            </details>
            </drift-review>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let doc = parse(&src, path()).expect("foreign HTML must be tolerated in body");
        let inv = doc.invocations.first().expect("one invocation");
        assert_eq!(inv.blocks.len(), 2);
        match inv.blocks.first().expect("drift-review") {
            VetBlock::DriftReview { body, .. } => {
                assert!(
                    body.contains("<details>"),
                    "body should retain foreign HTML"
                );
            }
            other => panic!("expected drift-review, got {other:?}"),
        }
    }

    #[test]
    fn unknown_attribute_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m" extra="x">
            body
            </drift-review>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("unknown attribute must fail");
        assert!(
            matches!(err.as_ref(), ParseError::UnknownMarkerAttribute { attribute, .. } if attribute == "extra"),
            "got {err:?}"
        );
    }

    #[test]
    fn out_of_domain_verdict_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="maybe" round="1" date="2026-05-21T18:00:00Z" model="m">
            body
            </drift-review>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("verdict=maybe must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalAttribute { attribute, value, .. } if attribute == "verdict" && value == "maybe"),
            "got {err:?}"
        );
    }

    #[test]
    fn non_terminal_gate_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>

            <simplifier-scan verdict="clean">
            after the gate
            </simplifier-scan>
        "#});
        let err = parse(&src, path()).expect_err("non-terminal gate must fail");
        assert!(
            matches!(err.as_ref(), ParseError::VetGateStructure { reason, .. } if reason.contains("not the last block")),
            "got {err:?}"
        );
    }

    #[test]
    fn two_gates_in_one_section_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <gate verdict="failed" tasks_hash="h1" date="2026-05-21T18:10:00Z">
            g1
            </gate>

            <gate verdict="passed" tasks_hash="h2" date="2026-05-21T18:20:00Z">
            g2
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("two gates must fail");
        assert!(
            matches!(err.as_ref(), ParseError::VetGateStructure { reason, .. } if reason.contains("exactly one is allowed")),
            "got {err:?}"
        );
    }

    #[test]
    fn missing_gate_in_section_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m">
            no gate follows
            </drift-review>
        "#});
        let err = parse(&src, path()).expect_err("missing gate must fail");
        assert!(
            matches!(err.as_ref(), ParseError::VetGateStructure { reason, .. } if reason.contains("no terminal `<gate>`")),
            "got {err:?}"
        );
    }

    #[test]
    fn round_reset_per_section_is_legal() {
        // Each section's round-bearing blocks start fresh at 1.
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="blocking" round="1" date="2026-05-21T18:00:00Z" model="m">
            d1
            </drift-review>

            <holistic-fix verdict="addressed" round="1" date="2026-05-21T18:05:00Z" model="m">
            f1
            </holistic-fix>

            <gate verdict="failed" tasks_hash="h1" date="2026-05-21T18:10:00Z">
            g1
            </gate>

            ## Invocation 2 — 2026-05-21T19:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T19:00:00Z" model="m">
            d2
            </drift-review>

            <gate verdict="passed" tasks_hash="h2" date="2026-05-21T19:10:00Z">
            g2
            </gate>
        "#});
        let doc = parse(&src, path()).expect("round-reset fixture must parse");
        assert_eq!(doc.invocations.len(), 2);
    }

    #[test]
    fn first_round_not_one_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="2" date="2026-05-21T18:00:00Z" model="m">
            d
            </drift-review>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("first round 2 must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalRoundSequence { reason, .. } if reason.contains("must have round=\"1\"")),
            "got {err:?}"
        );
    }

    #[test]
    fn round_skip_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="blocking" round="1" date="2026-05-21T18:00:00Z" model="m">
            d1
            </drift-review>

            <holistic-fix verdict="addressed" round="3" date="2026-05-21T18:05:00Z" model="m">
            f
            </holistic-fix>

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("round skip must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalRoundSequence { reason, .. } if reason.contains("skipped")),
            "got {err:?}"
        );
    }

    #[test]
    fn missing_frontmatter_is_rejected() {
        let src = indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#};
        let err = parse(src, path()).expect_err("missing frontmatter must fail");
        assert!(
            matches!(err.as_ref(), ParseError::MissingField { field, .. } if field == "frontmatter"),
            "got {err:?}"
        );
    }

    #[test]
    fn no_invocation_heading_is_rejected() {
        let src = make(indoc! {r#"
            <gate verdict="passed" tasks_hash="h" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("missing invocation heading must fail");
        assert!(
            matches!(err.as_ref(), ParseError::MalformedMarker { reason, .. } if reason.contains("Invocation")),
            "got {err:?}"
        );
    }

    #[test]
    fn missing_tasks_hash_on_gate_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <gate verdict="passed" date="2026-05-21T18:10:00Z">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("gate without tasks_hash must fail");
        assert!(
            matches!(err.as_ref(), ParseError::MissingField { field, .. } if field == "tasks_hash"),
            "got {err:?}"
        );
    }

    #[test]
    fn bad_gate_date_is_rejected() {
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <gate verdict="passed" tasks_hash="h" date="2026-05-21">
            g
            </gate>
        "#});
        let err = parse(&src, path()).expect_err("date-only gate must fail");
        assert!(
            matches!(err.as_ref(), ParseError::InvalidJournalAttribute { attribute, .. } if attribute == "date"),
            "got {err:?}"
        );
    }

    #[test]
    fn in_flight_accepts_open_trailing_section() {
        // The mid-vet-run shape: one section with a drift-review and no gate
        // yet. Strict parse rejects it; in-flight parse accepts it (DEC-008),
        // so `journal append` can derive state from it.
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="blocking" round="1" date="2026-05-21T18:00:00Z" model="m">
            drift found, no gate yet
            </drift-review>
        "#});
        let strict = parse(&src, path()).expect_err("strict must reject an open last section");
        assert!(
            matches!(strict.as_ref(), ParseError::VetGateStructure { reason, .. } if reason.contains("no terminal `<gate>`")),
            "got {strict:?}"
        );
        let doc = parse_in_flight(&src, path()).expect("in-flight must accept an open section");
        assert_eq!(doc.invocations.len(), 1);
        let inv = doc.invocations.first().expect("one invocation");
        assert_eq!(inv.number, 1);
        assert_eq!(inv.blocks.len(), 1);
        assert!(matches!(
            inv.blocks.first(),
            Some(VetBlock::DriftReview { round, .. }) if *round == 1
        ));
    }

    #[test]
    fn in_flight_matches_strict_on_a_complete_file() {
        // A fully gate-terminated file parses identically under both entry
        // points: the in-flight relaxation only ever exempts an *un-gated*
        // last section, so a gated one is validated in full.
        let src = full_grammar_fixture();
        let strict = parse(&src, path()).expect("strict parses a complete file");
        let in_flight = parse_in_flight(&src, path()).expect("in-flight parses a complete file");
        assert_eq!(
            strict, in_flight,
            "a complete file parses identically under parse and parse_in_flight"
        );
    }

    #[test]
    fn in_flight_rejects_open_non_last_section() {
        // Only the LAST section may be open. An un-gated *earlier* section is
        // still rejected even in in-flight mode — a real VET.md never has an
        // open section followed by a later one.
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m">
            first section never gated
            </drift-review>

            ## Invocation 2 — 2026-05-21T19:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T19:00:00Z" model="m">
            open second section
            </drift-review>
        "#});
        let err = parse_in_flight(&src, path()).expect_err("un-gated non-last section must fail");
        assert!(
            matches!(err.as_ref(), ParseError::VetGateStructure { reason, .. } if reason.contains("invocation 1") && reason.contains("no terminal `<gate>`")),
            "got {err:?}"
        );
    }

    #[test]
    fn line_isolated_vet_tag_in_body_is_rejected() {
        // A body line that *is* a vet open tag is read as a nested block by the
        // shared scanner (vet blocks must not nest), so the file is rejected by
        // both parse and parse_in_flight. This is the body-inertness invariant
        // the append path's write-time round-trip relies on (DEC-008): a body
        // smuggling a structural line cannot produce a parseable file, so no
        // separate body-markup guard is needed.
        let src = make(indoc! {r#"
            ## Invocation 1 — 2026-05-21T18:00:00Z

            <drift-review verdict="pass" round="1" date="2026-05-21T18:00:00Z" model="m">
            intro
            <gate verdict="passed">
            tail
            </drift-review>
        "#});
        let strict = parse(&src, path()).expect_err("nested vet open tag in body must fail");
        assert!(
            matches!(strict.as_ref(), ParseError::MalformedMarker { .. }),
            "got {strict:?}"
        );
        assert!(
            parse_in_flight(&src, path()).is_err(),
            "in-flight parse must reject the same body-smuggled tag"
        );
    }
}
