//! Writer + placement planner for the pre-ship vet journal (`journal/VET.md`).
//!
//! The sibling [`super`] module *parses* a VET.md file; this module is its
//! inverse — it renders one validated vet block (and, on a fresh file, the
//! YAML frontmatter and section heading) and computes where the next block
//! lands, so the `speccy journal append <SPEC-NNNN>` command can grow the
//! vet journal one block at a time.
//!
//! Division of authority mirrors the per-task journal: the
//! **caller** supplies only identity and judgment (`verdict`, `model`,
//! body); the CLI is the sole authority for every environment-derivable
//! value — `date` on every block, `round` (a `drift-review` opens a round,
//! a non-opening block attaches to the current one), `tasks_hash` on a
//! `gate` (lowercase hex SHA-256 of the sibling TASKS.md read at append
//! time), and invocation sectioning. This module takes those derived values
//! as pre-computed inputs rather than reading the wall clock or the
//! filesystem itself, keeping the renderer deterministic and unit-testable.
//!
//! [`plan_append`] computes the round and the new-section decision from the
//! *typed* parse of the existing file, not a separate text scan.
//! Mid-vet-run the file's last section is still open (no terminal `<gate>`
//! yet), a shape the strict [`super::parse`] rejects but
//! [`super::parse_in_flight`] accepts; the caller parses the existing file
//! once with `parse_in_flight` and hands the resulting [`super::VetDoc`]
//! here, so derivation reuses the parser's own definitions of a section, a
//! block, and a round rather than reimplementing them.
//!
//! Body inertness is likewise the parser's job, not this module's: the
//! append path re-parses the *would-be-new* file through
//! [`super::parse_in_flight`] before writing a byte, so a body smuggling a
//! line-isolated vet tag (which the shared scanner reads as a nested block)
//! produces an unparseable file and is refused at write time. This module
//! therefore validates only what is local to one block — the verdict
//! against the block type's closed domain, `model` non-empty where
//! required, and a non-empty body.

use crate::parse::vet_xml::DRIFT_REVIEW_VERDICTS;
use crate::parse::vet_xml::GATE_VERDICTS;
use crate::parse::vet_xml::HOLISTIC_FIX_VERDICTS;
use crate::parse::vet_xml::SIMPLIFIER_APPLY_VERDICTS;
use crate::parse::vet_xml::SIMPLIFIER_SCAN_VERDICTS;
use crate::parse::vet_xml::VetBlock;
use crate::parse::vet_xml::VetDoc;
use thiserror::Error;

/// The five vet-journal block types `speccy journal append <SPEC>` writes.
///
/// Task-journal block types (`implementer`, `review`, `blockers`) are a
/// separate grammar served by [`crate::parse::journal_xml::serialize`]; this
/// enum is the closed set for `journal/VET.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetBlockKind {
    /// `<drift-review>` — opens a round inside the current section.
    DriftReview,
    /// `<holistic-fix>` — attaches to the current round.
    HolisticFix,
    /// `<simplifier-scan>` — round-less, attaches to the current section.
    SimplifierScan,
    /// `<simplifier-apply>` — round-less, attaches to the current section.
    SimplifierApply,
    /// `<gate>` — terminal block; carries `tasks_hash`.
    Gate,
}

impl VetBlockKind {
    /// The on-disk element local name.
    #[must_use = "the element name is used to render the block"]
    pub fn element_name(self) -> &'static str {
        match self {
            VetBlockKind::DriftReview => "drift-review",
            VetBlockKind::HolisticFix => "holistic-fix",
            VetBlockKind::SimplifierScan => "simplifier-scan",
            VetBlockKind::SimplifierApply => "simplifier-apply",
            VetBlockKind::Gate => "gate",
        }
    }

    /// Whether this block opens a new round (`drift-review`).
    #[must_use = "the round-opening flag drives round derivation"]
    pub fn opens_round(self) -> bool {
        matches!(self, VetBlockKind::DriftReview)
    }

    /// Whether this block *attaches* to an existing round and therefore
    /// requires one to exist in the open section (`holistic-fix`). The
    /// round-less blocks (`simplifier-scan`, `simplifier-apply`, `gate`)
    /// carry no round and never require one.
    #[must_use = "the attaching flag drives the no-round-to-attach guard"]
    pub fn attaches_to_round(self) -> bool {
        matches!(self, VetBlockKind::HolisticFix)
    }

    /// The closed verdict domain for this block type.
    #[must_use = "the verdict domain is used to validate the verdict"]
    fn verdict_domain(self) -> &'static [&'static str] {
        match self {
            VetBlockKind::DriftReview => DRIFT_REVIEW_VERDICTS,
            VetBlockKind::HolisticFix => HOLISTIC_FIX_VERDICTS,
            VetBlockKind::SimplifierScan => SIMPLIFIER_SCAN_VERDICTS,
            VetBlockKind::SimplifierApply => SIMPLIFIER_APPLY_VERDICTS,
            VetBlockKind::Gate => GATE_VERDICTS,
        }
    }

    /// Whether this block carries a `round` and `model` attribute pair.
    #[must_use = "the round-bearing flag drives attribute rendering"]
    fn round_bearing(self) -> bool {
        matches!(self, VetBlockKind::DriftReview | VetBlockKind::HolisticFix)
    }
}

/// Where an append must land and which round it carries, computed by
/// [`plan_append`] from the current document state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendPlan {
    /// `true` when a fresh `## Invocation N` section must be opened before
    /// the block is written (file absent, or the last section is
    /// gate-terminated). The CLI stamps the heading datetime.
    pub open_new_section: bool,
    /// The invocation number for the heading when `open_new_section` is
    /// `true` (the existing count + 1, or 1 on a fresh file). When
    /// `open_new_section` is `false` this is the current (last) section's
    /// number, carried for diagnostics.
    pub invocation_number: u32,
    /// The `round` attribute for round-bearing blocks. Round-less blocks
    /// (`simplifier-scan`, `simplifier-apply`, `gate`) ignore it.
    pub round: u32,
}

/// A vet append that cannot land in the current document shape, surfaced
/// before any write. The CLI maps this to a non-zero exit with VET.md
/// byte-identical (or still absent).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum AppendPlanError {
    /// A non-opening block (`holistic-fix` / `simplifier-*` / `gate`) was
    /// requested against an open invocation section that has no
    /// `drift-review` opening a round. Attaching before a `drift-review`
    /// opens the section's first round is forbidden.
    #[error(
        "`{block}` requires a `drift-review` to open the current invocation \
         section first; the open section has no round to attach to"
    )]
    NoRoundToAttach {
        /// The attaching block's element name.
        block: &'static str,
    },
}

/// Caller-supplied inputs for one vet block, before CLI attribute stamping.
///
/// `verdict` is required for every block; `model` is required for the
/// round-bearing blocks (`drift-review`, `holistic-fix`) and ignored
/// otherwise. `tasks_hash` is consumed only by `gate`. `date` and `round`
/// are CLI-derived and always present.
#[derive(Debug, Clone)]
pub struct VetBlockInputs<'a> {
    /// Block type being rendered.
    pub kind: VetBlockKind,
    /// CLI-stamped ISO8601 timestamp (`date` attribute on every block).
    pub date: &'a str,
    /// CLI-derived round counter (used only by round-bearing blocks).
    pub round: u32,
    /// `--verdict` value; required for every block, validated against the
    /// block type's closed domain.
    pub verdict: Option<&'a str>,
    /// `--model` value; required for `drift-review` and `holistic-fix`.
    pub model: Option<&'a str>,
    /// CLI-derived lowercase hex SHA-256 of TASKS.md; required for `gate`.
    pub tasks_hash: Option<&'a str>,
    /// Block body read from stdin (must be non-empty after trimming).
    pub body: &'a str,
}

/// A validation failure that aborts a vet append before any write.
///
/// These are the block-local checks the renderer owns. Structural validity
/// of the produced file — including body inertness — is the parser's job:
/// the append path re-parses the would-be-new file through
/// [`super::parse_in_flight`] before writing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum VetSerializeError {
    /// `--verdict` was not supplied.
    #[error("`{block}` requires `--verdict`")]
    MissingVerdict {
        /// Element name (e.g. `gate`).
        block: &'static str,
    },
    /// `--verdict` is outside the block type's closed domain.
    #[error("invalid verdict `{value}` for `{block}`; expected one of: {allowed}")]
    UnknownVerdict {
        /// Element name.
        block: &'static str,
        /// The rejected value.
        value: String,
        /// Comma-joined allowed set.
        allowed: String,
    },
    /// `--model` is required for this block type but was missing or empty.
    #[error("`{block}` requires a non-empty `--model`")]
    MissingModel {
        /// Element name.
        block: &'static str,
    },
    /// The block body was empty (or whitespace-only).
    #[error("block body is empty; a vet block must carry a non-empty body on stdin")]
    EmptyBody,
}

/// Render the YAML frontmatter for a freshly created VET.md file.
///
/// The trailing blank line separates frontmatter from the first invocation
/// heading, so concatenating a section heading (see [`render_section_heading`])
/// directly after this string yields a parseable file.
#[must_use = "the rendered frontmatter must be written to create the file"]
pub fn render_fresh_vet_frontmatter(spec: &str, generated_at: &str) -> String {
    format!("---\nspec: {spec}\ngenerated_at: {generated_at}\n---\n\n")
}

/// Render a `## Invocation N — <date>` section heading with the trailing
/// blank line that separates it from the first block.
#[must_use = "the rendered heading must be written to open a section"]
pub fn render_section_heading(number: u32, date: &str) -> String {
    format!("## Invocation {number} — {date}\n\n")
}

/// Compute where a new block of `kind` must land and which round it carries,
/// from the typed in-flight parse of the existing VET.md (`None` on a fresh
/// file).
///
/// The caller parses the existing file with [`super::parse_in_flight`] (which
/// tolerates the open trailing section that exists mid-vet-run) and passes
/// the resulting [`VetDoc`] here, so this planner reuses the parser's own
/// section/block/round structure rather than re-scanning the raw text.
///
/// Invocation sectioning: a new `## Invocation N` section opens
/// when the file is absent or the last section is gate-terminated — for any
/// block type — so a block appended after a section's gate never lands in
/// the closed section. Round derivation: a `drift-review` opens a
/// round (max round in the open section + 1, or 1 in a freshly opened
/// section); a `holistic-fix` attaches to the open section's current
/// (highest) round; the round-less blocks (`simplifier-scan`,
/// `simplifier-apply`, `gate`) carry no round.
///
/// # Errors
///
/// Returns [`AppendPlanError::NoRoundToAttach`] when a `holistic-fix` is
/// requested with no `drift-review` round in the open section to attach to
/// (including a freshly opened or gate-closed section). The round-less
/// blocks never error here.
pub fn plan_append(
    existing: Option<&VetDoc>,
    kind: VetBlockKind,
) -> Result<AppendPlan, AppendPlanError> {
    // Derive the current document state from the typed parse. A fresh file
    // (or one with no invocation sections) has no open section.
    let (section_count, last_section_number, last_section_gated, last_section_highest_round) =
        match existing.and_then(|doc| doc.invocations.last().map(|last| (doc, last))) {
            None => (0u32, None, false, None),
            Some((doc, last)) => {
                // A real VET.md never holds anywhere near `u32::MAX` invocation
                // sections; saturate rather than truncate on the impossible
                // overflow.
                let section_count = u32::try_from(doc.invocations.len()).unwrap_or(u32::MAX);
                let gated = matches!(last.blocks.last(), Some(VetBlock::Gate { .. }));
                let highest_round = last
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        VetBlock::DriftReview { round, .. }
                        | VetBlock::HolisticFix { round, .. } => Some(*round),
                        VetBlock::SimplifierScan { .. }
                        | VetBlock::SimplifierApply { .. }
                        | VetBlock::Gate { .. } => None,
                    })
                    .max();
                (section_count, Some(last.number), gated, highest_round)
            }
        };

    // A new section opens when the file is absent or the last section is
    // gate-terminated; the appended block becomes that section's first.
    let open_new_section = section_count == 0 || last_section_gated;

    // Round-bearing context: a freshly opened section has no round yet;
    // an open section carries its highest round.
    let current_round = if open_new_section {
        None
    } else {
        last_section_highest_round
    };

    // A `holistic-fix` must attach to an existing round; everything else
    // either opens a round or is round-less.
    if kind.attaches_to_round() && current_round.is_none() {
        return Err(AppendPlanError::NoRoundToAttach {
            block: kind.element_name(),
        });
    }

    let round = if kind.opens_round() {
        current_round.map_or(1, |r| r.saturating_add(1))
    } else {
        // `holistic-fix` reuses the current round (guaranteed Some above);
        // round-less blocks ignore this field — record the section's current
        // round (or 1) so the value is meaningful rather than arbitrary.
        current_round.unwrap_or(1)
    };

    let invocation_number = if open_new_section {
        section_count.saturating_add(1)
    } else {
        last_section_number.unwrap_or(1)
    };

    Ok(AppendPlan {
        open_new_section,
        invocation_number,
        round,
    })
}

/// Validate `inputs` and render exactly one vet block.
///
/// The returned string is the block with a trailing newline, ready to be
/// appended after existing content (which already ends in a newline) or
/// after a freshly rendered section heading. On any validation failure no
/// string is produced.
///
/// This validates only the block-local invariants (verdict domain, `model`
/// presence, non-empty body). The structural validity of the resulting
/// file — that the body introduces no nested block or phantom section — is
/// enforced by the append path re-parsing the would-be-new file through
/// [`super::parse_in_flight`] before writing, so a body that would
/// produce an unparseable file is refused at write time even though it
/// renders cleanly here.
///
/// # Errors
///
/// Returns [`VetSerializeError`] when the verdict is missing or out of
/// domain, a round-bearing block has no non-empty `model`, or the body is
/// empty.
pub fn validate_and_render_vet_block(
    inputs: &VetBlockInputs<'_>,
) -> Result<String, VetSerializeError> {
    let element = inputs.kind.element_name();

    if inputs.body.trim().is_empty() {
        return Err(VetSerializeError::EmptyBody);
    }

    let verdict = inputs
        .verdict
        .ok_or(VetSerializeError::MissingVerdict { block: element })?;
    let domain = inputs.kind.verdict_domain();
    if !domain.contains(&verdict) {
        return Err(VetSerializeError::UnknownVerdict {
            block: element,
            value: verdict.to_owned(),
            allowed: domain.join(", "),
        });
    }

    let model = if inputs.kind.round_bearing() {
        let m = inputs
            .model
            .filter(|m| !m.is_empty())
            .ok_or(VetSerializeError::MissingModel { block: element })?;
        Some(m)
    } else {
        None
    };

    // `tasks_hash` is required for `gate` and supplied by the CLI (not the
    // human caller), so a missing value here is a CLI bug, not user input —
    // render an empty attribute value that the parser then rejects loudly
    // rather than silently dropping the attribute. In practice the CLI
    // always supplies it; this branch keeps the renderer total.
    let open = match inputs.kind {
        VetBlockKind::DriftReview | VetBlockKind::HolisticFix => format!(
            "<{element} verdict=\"{verdict}\" round=\"{round}\" date=\"{date}\" model=\"{model}\">",
            round = inputs.round,
            date = inputs.date,
            model = model.unwrap_or_default(),
        ),
        VetBlockKind::SimplifierScan | VetBlockKind::SimplifierApply => {
            format!("<{element} verdict=\"{verdict}\">")
        }
        VetBlockKind::Gate => format!(
            "<{element} verdict=\"{verdict}\" tasks_hash=\"{tasks_hash}\" date=\"{date}\">",
            tasks_hash = inputs.tasks_hash.unwrap_or_default(),
            date = inputs.date,
        ),
    };

    let body = inputs.body.trim_end_matches('\n');
    Ok(format!("{open}\n{body}\n</{element}>\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::vet_xml::parse as parse_vet;
    use crate::parse::vet_xml::parse_in_flight;
    use camino::Utf8Path;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/journal/VET.md")
    }

    /// Parse a (possibly in-flight) VET.md string into the typed document the
    /// planner consumes, mirroring what the CLI does under the lock.
    fn doc_of(file: &str) -> VetDoc {
        parse_in_flight(file, path()).expect("in-flight parse of constructed fixture")
    }

    /// Render a `drift-review` block the way the append path would.
    fn drift_review(verdict: &str, round: u32) -> String {
        validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::DriftReview,
            date: "2026-05-21T18:00:00Z",
            round,
            verdict: Some(verdict),
            model: Some("m"),
            tasks_hash: None,
            body: "drift body",
        })
        .expect("render drift-review")
    }

    fn gate(hash: &str) -> String {
        validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::Gate,
            date: "2026-05-21T18:10:00Z",
            round: 1,
            verdict: Some("passed"),
            model: None,
            tasks_hash: Some(hash),
            body: "shipping",
        })
        .expect("render gate")
    }

    #[test]
    fn fresh_file_drift_review_then_gate_round_trips() {
        // Fresh file: plan opens Invocation 1, drift-review round 1.
        let plan = plan_append(None, VetBlockKind::DriftReview).expect("plan opens section");
        assert!(plan.open_new_section);
        assert_eq!(plan.invocation_number, 1);
        assert_eq!(plan.round, 1);

        let fm = render_fresh_vet_frontmatter("SPEC-0042", "2026-05-21T18:00:00Z");
        let heading = render_section_heading(plan.invocation_number, "2026-05-21T18:00:00Z");
        let file = format!("{fm}{heading}{}{}", drift_review("pass", 1), gate("abc123"));
        let doc = parse_vet(&file, path()).expect("file round-trips");
        assert_eq!(doc.spec, "SPEC-0042");
        assert_eq!(doc.invocations.len(), 1);
        let inv = doc.invocations.first().expect("one invocation");
        assert_eq!(inv.number, 1);
        assert_eq!(inv.blocks.len(), 2);
    }

    #[test]
    fn holistic_fix_on_fresh_file_has_no_round_to_attach() {
        // A holistic-fix on a fresh file (no drift-review opened a round)
        // is the only round-less-context error case.
        let err =
            plan_append(None, VetBlockKind::HolisticFix).expect_err("holistic-fix must error");
        assert!(matches!(err, AppendPlanError::NoRoundToAttach { .. }));
    }

    #[test]
    fn round_less_blocks_open_a_fresh_section_without_a_round() {
        // simplifier-scan / simplifier-apply / gate carry no round, so they
        // can open the first (or next) section without a drift-review.
        for kind in [
            VetBlockKind::SimplifierScan,
            VetBlockKind::SimplifierApply,
            VetBlockKind::Gate,
        ] {
            let plan = plan_append(None, kind).expect("round-less block opens section");
            assert!(plan.open_new_section);
            assert_eq!(plan.invocation_number, 1);
        }
    }

    #[test]
    fn holistic_fix_attaches_to_open_section_round() {
        // An *open* (un-gated) section is not strict-parseable; the CLI parses
        // it with parse_in_flight and the planner derives from that document.
        let fm = render_fresh_vet_frontmatter("SPEC-0042", "2026-05-21T18:00:00Z");
        let heading = render_section_heading(1, "2026-05-21T18:00:00Z");
        let file = format!("{fm}{heading}{}", drift_review("blocking", 1));
        let plan =
            plan_append(Some(&doc_of(&file)), VetBlockKind::HolisticFix).expect("attach plan");
        assert!(!plan.open_new_section);
        assert_eq!(plan.round, 1);
    }

    #[test]
    fn second_drift_review_in_open_section_opens_round_two() {
        let fm = render_fresh_vet_frontmatter("SPEC-0042", "2026-05-21T18:00:00Z");
        let heading = render_section_heading(1, "2026-05-21T18:00:00Z");
        let file = format!("{fm}{heading}{}", drift_review("blocking", 1));
        let plan =
            plan_append(Some(&doc_of(&file)), VetBlockKind::DriftReview).expect("opens round 2");
        assert!(!plan.open_new_section);
        assert_eq!(plan.round, 2);
    }

    #[test]
    fn gate_terminated_section_opens_next_invocation() {
        let fm = render_fresh_vet_frontmatter("SPEC-0042", "2026-05-21T18:00:00Z");
        let heading = render_section_heading(1, "2026-05-21T18:00:00Z");
        let file = format!("{fm}{heading}{}{}", drift_review("pass", 1), gate("h1"));
        // A complete one-section file round-trips through the strict parser.
        parse_vet(&file, path()).expect("gated section parses");
        // A drift-review after a closed section opens Invocation 2, round 1.
        let plan =
            plan_append(Some(&doc_of(&file)), VetBlockKind::DriftReview).expect("opens section 2");
        assert!(plan.open_new_section);
        assert_eq!(plan.invocation_number, 2);
        assert_eq!(plan.round, 1);
    }

    #[test]
    fn simplifier_scan_after_gate_opens_invocation_two() {
        // A simplifier-scan after a gate-terminated section opens a
        // freshly numbered `## Invocation 2`, since it carries no round.
        let fm = render_fresh_vet_frontmatter("SPEC-0042", "2026-05-21T18:00:00Z");
        let heading = render_section_heading(1, "2026-05-21T18:00:00Z");
        let file = format!("{fm}{heading}{}{}", drift_review("pass", 1), gate("h1"));
        let plan = plan_append(Some(&doc_of(&file)), VetBlockKind::SimplifierScan)
            .expect("scan opens the next section");
        assert!(plan.open_new_section);
        assert_eq!(plan.invocation_number, 2);
    }

    #[test]
    fn missing_verdict_is_rejected() {
        let err = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::SimplifierScan,
            date: "2026-05-21T18:00:00Z",
            round: 1,
            verdict: None,
            model: None,
            tasks_hash: None,
            body: "x",
        })
        .expect_err("missing verdict must fail");
        assert!(matches!(err, VetSerializeError::MissingVerdict { .. }));
    }

    #[test]
    fn out_of_domain_verdict_is_rejected() {
        let err = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::DriftReview,
            date: "2026-05-21T18:00:00Z",
            round: 1,
            verdict: Some("maybe"),
            model: Some("m"),
            tasks_hash: None,
            body: "x",
        })
        .expect_err("verdict=maybe must fail");
        assert!(matches!(err, VetSerializeError::UnknownVerdict { .. }));
    }

    #[test]
    fn round_bearing_block_requires_model() {
        let err = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::HolisticFix,
            date: "2026-05-21T18:00:00Z",
            round: 1,
            verdict: Some("addressed"),
            model: None,
            tasks_hash: None,
            body: "x",
        })
        .expect_err("holistic-fix without model must fail");
        assert!(matches!(err, VetSerializeError::MissingModel { .. }));
    }

    #[test]
    fn empty_body_is_rejected() {
        let err = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::SimplifierScan,
            date: "2026-05-21T18:00:00Z",
            round: 1,
            verdict: Some("clean"),
            model: None,
            tasks_hash: None,
            body: "  \n ",
        })
        .expect_err("empty body must fail");
        assert!(matches!(err, VetSerializeError::EmptyBody));
    }

    #[test]
    fn round_less_blocks_render_without_round_or_model() {
        let scan = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::SimplifierScan,
            date: "2026-05-21T18:00:00Z",
            round: 7,
            verdict: Some("candidates"),
            model: Some("ignored"),
            tasks_hash: None,
            body: "candidates",
        })
        .expect("render scan");
        assert!(
            !scan.contains("round=") && !scan.contains("model="),
            "simplifier-scan carries only verdict, got {scan:?}"
        );
    }

    #[test]
    fn rendered_block_round_trips_through_the_in_flight_parser() {
        // The renderer's output, assembled into an open section, must parse
        // under parse_in_flight — the same round-trip the append path runs
        // before writing. A body carrying an inline (non-line-isolated) vet
        // tag mention is inert prose and parses; a body whose own line is a
        // vet tag is rejected (it would nest a block). This pins the
        // block-local renderer against the parser that is the write-time
        // authority, without this module re-deciding what a tag is.
        let fm = render_fresh_vet_frontmatter("SPEC-0042", "2026-05-21T18:00:00Z");
        let heading = render_section_heading(1, "2026-05-21T18:00:00Z");

        let inline = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::DriftReview,
            date: "2026-05-21T18:00:00Z",
            round: 1,
            verdict: Some("blocking"),
            model: Some("m"),
            tasks_hash: None,
            body: "noted an inline <gate verdict=\"passed\"> mention and 3 < 4 in prose",
        })
        .expect("inline tag-lookalike prose renders");
        let open_file = format!("{fm}{heading}{inline}");
        let doc = parse_in_flight(&open_file, path()).expect("inline-prose body parses in-flight");
        assert_eq!(doc.invocations.len(), 1);
        let inv = doc.invocations.first().expect("one invocation");
        assert_eq!(inv.blocks.len(), 1);

        // A body whose own line is a vet open tag renders here (block-local
        // validation passes) but makes the assembled file unparseable — the
        // write-time round-trip is what rejects it.
        let nested = validate_and_render_vet_block(&VetBlockInputs {
            kind: VetBlockKind::DriftReview,
            date: "2026-05-21T18:00:00Z",
            round: 1,
            verdict: Some("blocking"),
            model: Some("m"),
            tasks_hash: None,
            body: "intro\n<gate verdict=\"passed\">\ntail",
        })
        .expect("renderer does not itself reject body markup");
        let nested_file = format!("{fm}{heading}{nested}");
        assert!(
            parse_in_flight(&nested_file, path()).is_err(),
            "a body whose own line is a vet tag must make the file unparseable",
        );
    }
}
