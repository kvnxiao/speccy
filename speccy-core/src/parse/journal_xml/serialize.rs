//! Writer for per-task journal files (`journal/T-NNN.md`).
//!
//! The sibling [`super`] module *parses* a journal file; this module is its
//! inverse — it renders one validated `<implementer>` / `<review>` /
//! `<blockers>` block and (on a fresh file) the YAML frontmatter, so the
//! `speccy journal append` command can grow a journal one block at a time.
//!
//! Division of authority: the **caller** supplies only
//! identity and judgment (`model`, `persona`, `verdict`, body); the CLI is
//! the sole authority for `date` and `round`, which this module accepts as
//! pre-computed inputs rather than deriving from the wall clock itself
//! (keeping the renderer deterministic and unit-testable). [`derive_round`]
//! computes the round from existing file state under the caller's lock.
//!
//! Validation runs *before* any byte is produced
//! ([`validate_and_render_block`] returns `Err` without emitting output):
//! required attributes per block type, `persona` against the registry,
//! `verdict` against `{pass, blocking}`, and a non-empty body. The caller
//! re-parses the assembled would-be-new file through [`super::parse`] before
//! writing, so a body whose own line is journal markup is rejected
//! there rather than by a pre-scan here. Every block this module renders,
//! appended to a file that already parses, leaves a file that [`super::parse`]
//! accepts.

use crate::parse::journal_xml::ALLOWED_REVIEW_VERDICTS;
use crate::parse::journal_xml::JournalDoc;
use crate::personas::ALL as PERSONAS_ALL;
use thiserror::Error;

/// The three task-journal block types `speccy journal append` writes.
///
/// VET-journal block types (`drift-review`, `gate`, …) are a separate
/// grammar served by a later task; this enum is the closed set for
/// `journal/T-NNN.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBlockKind {
    /// `<implementer>` — opens a new round.
    Implementer,
    /// `<review>` — attaches to the current round.
    Review,
    /// `<blockers>` — attaches to the current round.
    Blockers,
}

impl TaskBlockKind {
    /// The on-disk element local name.
    #[must_use = "the element name is used to render the block"]
    pub fn element_name(self) -> &'static str {
        match self {
            TaskBlockKind::Implementer => "implementer",
            TaskBlockKind::Review => "review",
            TaskBlockKind::Blockers => "blockers",
        }
    }

    /// Whether this block type opens a new round (`implementer`) versus
    /// attaching to the current one (`review` / `blockers`).
    #[must_use = "the round-opening flag drives round derivation"]
    pub fn opens_round(self) -> bool {
        matches!(self, TaskBlockKind::Implementer)
    }
}

/// Caller-supplied inputs for one block, before CLI attribute stamping.
///
/// `model`, `persona`, and `verdict` are optional at this layer because
/// their *requiredness* depends on the block type —
/// [`validate_and_render_block`] enforces which are mandatory. `date` and
/// `round` are CLI-derived and always present.
#[derive(Debug, Clone)]
pub struct BlockInputs<'a> {
    /// Block type being rendered.
    pub kind: TaskBlockKind,
    /// CLI-stamped ISO8601 timestamp (`date` attribute on every block).
    pub date: &'a str,
    /// CLI-derived round counter (`round` attribute on every block).
    pub round: u32,
    /// `--model` value; required for `implementer` and `review`.
    pub model: Option<&'a str>,
    /// `--persona` value; required for `review`.
    pub persona: Option<&'a str>,
    /// `--verdict` value; required for `review`.
    pub verdict: Option<&'a str>,
    /// Block body read from stdin (must be non-empty after trimming).
    pub body: &'a str,
}

/// A validation failure that aborts an append before any write.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SerializeError {
    /// A required attribute for the block type was not supplied.
    #[error("`{block}` requires `{flag}`")]
    MissingAttribute {
        /// Element name (e.g. `review`).
        block: &'static str,
        /// CLI flag the caller omitted (e.g. `--persona`).
        flag: &'static str,
    },
    /// `--persona` is not a registered persona name.
    #[error("invalid persona `{value}`; expected one of: {allowed}")]
    UnknownPersona {
        /// The rejected value.
        value: String,
        /// Comma-joined registry.
        allowed: String,
    },
    /// `--verdict` is outside the closed `{pass, blocking}` domain.
    #[error("invalid verdict `{value}`; expected one of: {allowed}")]
    UnknownVerdict {
        /// The rejected value.
        value: String,
        /// Comma-joined allowed set.
        allowed: String,
    },
    /// The block body was empty (or whitespace-only).
    #[error("block body is empty; a journal block must carry a non-empty body on stdin")]
    EmptyBody,
    /// `--model` was supplied but empty.
    #[error("`{block}` requires a non-empty `--model`")]
    EmptyModel {
        /// Element name.
        block: &'static str,
    },
}

/// An attaching block (`review`/`blockers`) was requested against a journal
/// that has no `implementer` block opening a round, so there is no round to
/// attach to. Returned by [`derive_round`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("no existing round to attach to; append an `implementer` block first")]
#[non_exhaustive]
pub struct NoRoundError;

/// Derive the round for a new block given the current parsed journal.
///
/// A round-opening block (`implementer`) takes `max existing round + 1`, or
/// `1` on a fresh file. An attaching block (`review` / `blockers`) takes the
/// current (highest) round and is rejected with [`NoRoundError`] when no
/// block exists yet — a `review`/`blockers` is forbidden before any
/// `implementer` opened a round.
///
/// # Errors
///
/// Returns [`NoRoundError`] when an attaching block is requested against a
/// journal that has no blocks yet (no round to attach to). The caller maps
/// this to a non-zero exit with the journal untouched.
pub fn derive_round(doc: Option<&JournalDoc>, kind: TaskBlockKind) -> Result<u32, NoRoundError> {
    let highest = doc.and_then(|d| d.entries.iter().map(super::JournalEntry::round).max());
    if kind.opens_round() {
        Ok(highest.map_or(1, |r| r.saturating_add(1)))
    } else {
        highest.ok_or(NoRoundError)
    }
}

/// Render the YAML frontmatter block for a freshly created journal file.
///
/// The trailing blank line separates frontmatter from the first block, so
/// concatenating a rendered block (see [`validate_and_render_block`])
/// directly after this string yields a parseable file.
#[must_use = "the rendered frontmatter must be written to create the file"]
pub fn render_fresh_frontmatter(spec: &str, task: &str, generated_at: &str) -> String {
    format!("---\nspec: {spec}\ntask: {task}\ngenerated_at: {generated_at}\n---\n\n")
}

/// Validate `inputs` and render exactly one journal block.
///
/// The returned string is the block with a trailing newline, ready to be
/// appended after existing content (which already ends in a newline). On any
/// validation failure no string is produced.
///
/// # Errors
///
/// Returns [`SerializeError`] when a required attribute is missing, the
/// persona or verdict is out of domain, the body is empty, the model is
/// empty, or the body contains nested journal markup.
pub fn validate_and_render_block(inputs: &BlockInputs<'_>) -> Result<String, SerializeError> {
    let element = inputs.kind.element_name();

    // An empty body is invalid for every block type. A body whose own line is
    // journal markup is caught by the caller's write-time round-trip through
    // `super::parse`, not pre-scanned here.
    if inputs.body.trim().is_empty() {
        return Err(SerializeError::EmptyBody);
    }

    // Per-block-type required attributes.
    let model = match inputs.kind {
        TaskBlockKind::Implementer | TaskBlockKind::Review => {
            let m = inputs.model.ok_or(SerializeError::MissingAttribute {
                block: element,
                flag: "--model",
            })?;
            if m.is_empty() {
                return Err(SerializeError::EmptyModel { block: element });
            }
            Some(m)
        }
        TaskBlockKind::Blockers => None,
    };

    let persona = match inputs.kind {
        TaskBlockKind::Review => {
            let p = inputs.persona.ok_or(SerializeError::MissingAttribute {
                block: element,
                flag: "--persona",
            })?;
            if !PERSONAS_ALL.contains(&p) {
                return Err(SerializeError::UnknownPersona {
                    value: p.to_owned(),
                    allowed: PERSONAS_ALL.join(", "),
                });
            }
            Some(p)
        }
        TaskBlockKind::Implementer | TaskBlockKind::Blockers => None,
    };

    let verdict = match inputs.kind {
        TaskBlockKind::Review => {
            let v = inputs.verdict.ok_or(SerializeError::MissingAttribute {
                block: element,
                flag: "--verdict",
            })?;
            if !ALLOWED_REVIEW_VERDICTS.contains(&v) {
                return Err(SerializeError::UnknownVerdict {
                    value: v.to_owned(),
                    allowed: ALLOWED_REVIEW_VERDICTS.join(", "),
                });
            }
            Some(v)
        }
        TaskBlockKind::Implementer | TaskBlockKind::Blockers => None,
    };

    // Render the open tag. Attribute order matches the parser's documented
    // schema and the existing reference templates: optional `model`,
    // `persona`, `verdict` slot between the mandatory `date` and `round`.
    let model_attr = model.map(|m| format!(" model=\"{m}\"")).unwrap_or_default();
    let persona_attr = persona
        .map(|p| format!(" persona=\"{p}\""))
        .unwrap_or_default();
    let verdict_attr = verdict
        .map(|v| format!(" verdict=\"{v}\""))
        .unwrap_or_default();
    let open = format!(
        "<{element} date=\"{date}\"{model_attr}{persona_attr}{verdict_attr} round=\"{round}\">",
        date = inputs.date,
        round = inputs.round,
    );

    // Body sits on its own lines between the open and close tags, matching
    // the reference template shape (`<implementer ...>\n<body>\n</implementer>`).
    let body = inputs.body.trim_end_matches('\n');
    Ok(format!("{open}\n{body}\n</{element}>\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::journal_xml::parse as parse_journal;
    use camino::Utf8Path;

    fn path() -> &'static Utf8Path {
        Utf8Path::new("fixture/journal/T-001.md")
    }

    #[test]
    fn derive_round_opens_at_one_on_fresh_file() {
        let r = derive_round(None, TaskBlockKind::Implementer).expect("opens round");
        assert_eq!(r, 1);
    }

    #[test]
    fn derive_round_rejects_attaching_block_on_fresh_file() {
        let err = derive_round(None, TaskBlockKind::Review);
        assert!(err.is_err(), "review on fresh file must error");
    }

    #[test]
    fn fresh_file_then_implementer_parses() {
        let fm = render_fresh_frontmatter("SPEC-0042", "T-001", "2026-06-09T18:00:00Z");
        let block = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Implementer,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("test-model"),
            persona: None,
            verdict: None,
            body: "Completed: did the thing.",
        })
        .expect("render implementer");
        let file = format!("{fm}{block}");
        let doc = parse_journal(&file, path()).expect("parse round-trips");
        assert_eq!(doc.spec, "SPEC-0042");
        assert_eq!(doc.task, "T-001");
        assert_eq!(doc.entries.len(), 1);
        assert_eq!(doc.entries.first().expect("one entry").round(), 1);
    }

    #[test]
    fn review_requires_persona_and_verdict() {
        let missing_persona = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Review,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: None,
            verdict: Some("pass"),
            body: "looks good",
        });
        assert!(matches!(
            missing_persona,
            Err(SerializeError::MissingAttribute {
                flag: "--persona",
                ..
            })
        ));

        let missing_verdict = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Review,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: Some("tests"),
            verdict: None,
            body: "looks good",
        });
        assert!(matches!(
            missing_verdict,
            Err(SerializeError::MissingAttribute {
                flag: "--verdict",
                ..
            })
        ));
    }

    #[test]
    fn unknown_persona_and_verdict_rejected() {
        let bad_persona = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Review,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: Some("not-a-persona"),
            verdict: Some("pass"),
            body: "x",
        });
        assert!(matches!(
            bad_persona,
            Err(SerializeError::UnknownPersona { .. })
        ));

        let bad_verdict = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Review,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: Some("tests"),
            verdict: Some("maybe"),
            body: "x",
        });
        assert!(matches!(
            bad_verdict,
            Err(SerializeError::UnknownVerdict { .. })
        ));
    }

    #[test]
    fn empty_body_rejected() {
        let err = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Implementer,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: None,
            verdict: None,
            body: "   \n  ",
        });
        assert!(matches!(err, Err(SerializeError::EmptyBody)));
    }

    #[test]
    fn round_two_implementer_after_round_one() {
        let fm = render_fresh_frontmatter("SPEC-0042", "T-001", "2026-06-09T18:00:00Z");
        let b1 = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Implementer,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: None,
            verdict: None,
            body: "round one",
        })
        .expect("b1");
        let file1 = format!("{fm}{b1}");
        let doc1 = parse_journal(&file1, path()).expect("parse1");
        let r2 = derive_round(Some(&doc1), TaskBlockKind::Implementer).expect("round2");
        assert_eq!(r2, 2);

        let b2 = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Implementer,
            date: "2026-06-09T18:00:01Z",
            round: r2,
            model: Some("m"),
            persona: None,
            verdict: None,
            body: "round two",
        })
        .expect("b2");
        let file2 = format!("{file1}{b2}");
        let doc2 = parse_journal(&file2, path()).expect("parse2");
        assert_eq!(doc2.entries.len(), 2);
        assert_eq!(doc2.entries.last().expect("last").round(), 2);
    }

    #[test]
    fn review_attaches_to_current_round() {
        let fm = render_fresh_frontmatter("SPEC-0042", "T-001", "2026-06-09T18:00:00Z");
        let imp = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Implementer,
            date: "2026-06-09T18:00:00Z",
            round: 1,
            model: Some("m"),
            persona: None,
            verdict: None,
            body: "impl",
        })
        .expect("imp");
        let file = format!("{fm}{imp}");
        let doc = parse_journal(&file, path()).expect("parse");
        let r = derive_round(Some(&doc), TaskBlockKind::Review).expect("attach round");
        assert_eq!(r, 1);
        let rev = validate_and_render_block(&BlockInputs {
            kind: TaskBlockKind::Review,
            date: "2026-06-09T18:00:01Z",
            round: r,
            model: Some("m"),
            persona: Some("tests"),
            verdict: Some("blocking"),
            body: "found an issue",
        })
        .expect("rev");
        let file2 = format!("{file}{rev}");
        let doc2 = parse_journal(&file2, path()).expect("parse2");
        assert_eq!(doc2.entries.len(), 2);
    }
}
