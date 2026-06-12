//! Text and JSON renderers for `speccy journal show` (SPEC-0055 REQ-006).
//!
//! `speccy journal show <selector>` parses the resolved journal — a task
//! selector resolves `<spec-dir>/journal/<task-id>.md` via the per-task
//! `journal_xml` parser, a bare `SPEC-NNNN` selector resolves
//! `<spec-dir>/journal/VET.md` via the `vet_xml` parser (DEC-004) — applies
//! the three conjunctive filters (`--round`, `--verdict`, `--block`), and
//! emits the surviving blocks. `--json` toggles representation, never
//! content: the same filtered blocks render either as a schema-versioned
//! JSON envelope (mirroring `next`/`status`) or as human-readable text.
//!
//! The envelope's `schema_version` is the first field, pinned at `1` pre-v1.
//! For a task journal it carries the frontmatter (`spec`, `task`,
//! `generated_at`) and a flat `blocks` array; for VET.md it carries the
//! frontmatter (`spec`, `generated_at`) and `invocations`, each holding its
//! own `blocks` array — rounds reset per invocation section, so the
//! `--round latest|N` filter applies within the last invocation section
//! (the slice the vet flow's call sites need).

use serde::Serialize;
use speccy_core::parse::JournalDoc;
use speccy_core::parse::JournalEntry;
use speccy_core::parse::VetBlock;
use speccy_core::parse::VetDoc;

/// The resolved-and-filtered view of a journal, ready to render in either
/// representation. One variant per journal kind (DEC-004).
#[derive(Debug, Clone)]
pub enum FilteredJournal {
    /// A per-task journal (`journal/<task-id>.md`).
    Task {
        /// `spec:` frontmatter field.
        spec: String,
        /// `task:` frontmatter field.
        task: String,
        /// `generated_at:` frontmatter field.
        generated_at: String,
        /// The highest round when `--round latest` was applied, so the
        /// envelope can surface the resolved round number; `None`
        /// otherwise.
        latest_round: Option<u32>,
        /// Surviving blocks after the conjunctive filters.
        blocks: Vec<JournalEntry>,
    },
    /// The pre-ship vet journal (`journal/VET.md`).
    Vet {
        /// `spec:` frontmatter field.
        spec: String,
        /// `generated_at:` frontmatter field.
        generated_at: String,
        /// The highest round in the last invocation section when
        /// `--round latest` was applied; `None` otherwise.
        latest_round: Option<u32>,
        /// Surviving invocation sections, each carrying its surviving
        /// blocks. Sections that lose every block to the filters are
        /// dropped.
        invocations: Vec<FilteredInvocation>,
    },
}

/// One invocation section's surviving blocks for the vet-journal view.
#[derive(Debug, Clone)]
pub struct FilteredInvocation {
    /// The `N` in `## Invocation N`.
    pub number: u32,
    /// The ISO8601 datetime on the heading line.
    pub date: String,
    /// Surviving blocks after the conjunctive filters.
    pub blocks: Vec<VetBlock>,
}

// ---------------------------------------------------------------------------
// JSON envelopes
// ---------------------------------------------------------------------------

/// JSON envelope for a task-journal `show`.
#[derive(Debug, Clone, Serialize)]
pub struct JsonTaskJournal {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// `spec:` frontmatter field.
    pub spec: String,
    /// `task:` frontmatter field.
    pub task: String,
    /// `generated_at:` frontmatter field.
    pub generated_at: String,
    /// The round `--round latest` resolved to; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_round: Option<u32>,
    /// Surviving blocks after the conjunctive filters.
    pub blocks: Vec<JsonJournalBlock>,
}

/// JSON envelope for a vet-journal (`VET.md`) `show`.
#[derive(Debug, Clone, Serialize)]
pub struct JsonVetJournal {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// `spec:` frontmatter field.
    pub spec: String,
    /// `generated_at:` frontmatter field.
    pub generated_at: String,
    /// The round (within the last invocation section) `--round latest`
    /// resolved to; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_round: Option<u32>,
    /// Surviving invocation sections.
    pub invocations: Vec<JsonInvocation>,
}

/// A single invocation section inside the vet JSON envelope.
#[derive(Debug, Clone, Serialize)]
pub struct JsonInvocation {
    /// The `N` in `## Invocation N`.
    pub number: u32,
    /// The ISO8601 datetime on the heading line.
    pub date: String,
    /// Surviving blocks after the conjunctive filters.
    pub blocks: Vec<JsonVetBlock>,
}

/// One per-task journal block, flattened for JSON. Round-less and
/// persona-less variants omit those fields.
#[derive(Debug, Clone, Serialize)]
pub struct JsonJournalBlock {
    /// Element local name: `implementer`, `review`, or `blockers`.
    pub block: &'static str,
    /// ISO8601 timestamp.
    pub date: String,
    /// Round counter.
    pub round: u32,
    /// Model identity; present for `implementer` and `review`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Reviewer persona; present for `review`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// Verdict; present for `review`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    /// Verbatim block body.
    pub body: String,
}

/// The attributes-only projection of one per-task journal block: the same
/// attribute fields as [`JsonJournalBlock`] with **no `body` field at all**
/// (SPEC-0060 REQ-002 / DEC-004). Used by `speccy context`'s
/// `journal.prior_rounds` index so within-task history stays visible without
/// paying for its prose.
///
/// A separate struct (rather than a serde-skip flag on [`JsonJournalBlock`])
/// keeps the full-block type's `body` invariant intact for `journal show`
/// consumers, and guarantees the `body` key is absent from this shape's
/// serialization — never emitted empty — so index entries are unambiguously
/// distinguishable from full blocks.
#[derive(Debug, Clone, Serialize)]
pub struct JsonJournalBlockAttrs {
    /// Element local name: `implementer`, `review`, or `blockers`.
    pub block: &'static str,
    /// ISO8601 timestamp.
    pub date: String,
    /// Round counter.
    pub round: u32,
    /// Model identity; present for `implementer` and `review`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Reviewer persona; present for `review`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// Verdict; present for `review`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
}

/// One vet-journal block, flattened for JSON. Round-less block types omit
/// `round`/`model`; only `gate` carries `tasks_hash`.
#[derive(Debug, Clone, Serialize)]
pub struct JsonVetBlock {
    /// Element local name: `drift-review`, `holistic-fix`,
    /// `simplifier-scan`, `simplifier-apply`, or `gate`.
    pub block: &'static str,
    /// Verdict (every vet block carries one).
    pub verdict: String,
    /// ISO8601 timestamp; present on the blocks that carry one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Round counter; present for `drift-review` / `holistic-fix`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<u32>,
    /// Model identity; present for `drift-review` / `holistic-fix`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Lowercase hex SHA-256 of TASKS.md; present only on `gate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks_hash: Option<String>,
    /// Verbatim block body.
    pub body: String,
}

// ---------------------------------------------------------------------------
// JSON renderers
// ---------------------------------------------------------------------------

/// Build the JSON envelope for a filtered journal view.
#[must_use = "the JSON payload is the output of `speccy journal show --json`"]
pub fn render_json(view: &FilteredJournal) -> JournalShowJson {
    match view {
        FilteredJournal::Task {
            spec,
            task,
            generated_at,
            latest_round,
            blocks,
        } => JournalShowJson::Task(JsonTaskJournal {
            schema_version: 1,
            spec: spec.clone(),
            task: task.clone(),
            generated_at: generated_at.clone(),
            latest_round: *latest_round,
            blocks: blocks.iter().map(to_json_journal_block).collect(),
        }),
        FilteredJournal::Vet {
            spec,
            generated_at,
            latest_round,
            invocations,
        } => JournalShowJson::Vet(JsonVetJournal {
            schema_version: 1,
            spec: spec.clone(),
            generated_at: generated_at.clone(),
            latest_round: *latest_round,
            invocations: invocations
                .iter()
                .map(|inv| JsonInvocation {
                    number: inv.number,
                    date: inv.date.clone(),
                    blocks: inv.blocks.iter().map(to_json_vet_block).collect(),
                })
                .collect(),
        }),
    }
}

/// The serializable envelope, one variant per journal kind. Serializes as
/// the inner object directly (untagged), so `schema_version` stays the
/// first field of the emitted object.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum JournalShowJson {
    /// A per-task journal envelope.
    Task(JsonTaskJournal),
    /// A vet-journal envelope.
    Vet(JsonVetJournal),
}

/// Project one parsed [`JournalEntry`] into its flat [`JsonJournalBlock`]
/// JSON shape. Shared so `speccy context`'s inlined-journal section
/// (SPEC-0056 REQ-004) reuses this exact mapping rather than re-deriving a
/// parallel journal-to-JSON projection — the same anti-drift discipline the
/// SPEC applies to `check`/`context`.
#[must_use = "the projected block is the journal section of the bundle / show envelope"]
pub fn to_json_journal_block(entry: &JournalEntry) -> JsonJournalBlock {
    match entry {
        JournalEntry::Implementer {
            date,
            model,
            round,
            body,
            ..
        } => JsonJournalBlock {
            block: "implementer",
            date: date.clone(),
            round: *round,
            model: Some(model.clone()),
            persona: None,
            verdict: None,
            body: body.clone(),
        },
        JournalEntry::Review {
            date,
            model,
            persona,
            verdict,
            round,
            body,
            ..
        } => JsonJournalBlock {
            block: "review",
            date: date.clone(),
            round: *round,
            model: Some(model.clone()),
            persona: Some(persona.clone()),
            verdict: Some(verdict.clone()),
            body: body.clone(),
        },
        JournalEntry::Blockers {
            date, round, body, ..
        } => JsonJournalBlock {
            block: "blockers",
            date: date.clone(),
            round: *round,
            model: None,
            persona: None,
            verdict: None,
            body: body.clone(),
        },
    }
}

/// Project one parsed [`JournalEntry`] into its attributes-only
/// [`JsonJournalBlockAttrs`] shape — the same attribute mapping as
/// [`to_json_journal_block`] minus the `body` (SPEC-0060 REQ-002 / DEC-004).
/// Mirrors the full-block projection so the two cannot drift in which
/// optionals each block type carries.
#[must_use = "the projected attrs are an entry of the bundle's prior-rounds index"]
pub fn to_json_journal_block_attrs(entry: &JournalEntry) -> JsonJournalBlockAttrs {
    match entry {
        JournalEntry::Implementer {
            date, model, round, ..
        } => JsonJournalBlockAttrs {
            block: "implementer",
            date: date.clone(),
            round: *round,
            model: Some(model.clone()),
            persona: None,
            verdict: None,
        },
        JournalEntry::Review {
            date,
            model,
            persona,
            verdict,
            round,
            ..
        } => JsonJournalBlockAttrs {
            block: "review",
            date: date.clone(),
            round: *round,
            model: Some(model.clone()),
            persona: Some(persona.clone()),
            verdict: Some(verdict.clone()),
        },
        JournalEntry::Blockers { date, round, .. } => JsonJournalBlockAttrs {
            block: "blockers",
            date: date.clone(),
            round: *round,
            model: None,
            persona: None,
            verdict: None,
        },
    }
}

fn to_json_vet_block(block: &VetBlock) -> JsonVetBlock {
    match block {
        VetBlock::DriftReview {
            verdict,
            round,
            date,
            model,
            body,
            ..
        } => JsonVetBlock {
            block: "drift-review",
            verdict: verdict.clone(),
            date: Some(date.clone()),
            round: Some(*round),
            model: Some(model.clone()),
            tasks_hash: None,
            body: body.clone(),
        },
        VetBlock::HolisticFix {
            verdict,
            round,
            date,
            model,
            body,
            ..
        } => JsonVetBlock {
            block: "holistic-fix",
            verdict: verdict.clone(),
            date: Some(date.clone()),
            round: Some(*round),
            model: Some(model.clone()),
            tasks_hash: None,
            body: body.clone(),
        },
        VetBlock::SimplifierScan { verdict, body, .. } => JsonVetBlock {
            block: "simplifier-scan",
            verdict: verdict.clone(),
            date: None,
            round: None,
            model: None,
            tasks_hash: None,
            body: body.clone(),
        },
        VetBlock::SimplifierApply { verdict, body, .. } => JsonVetBlock {
            block: "simplifier-apply",
            verdict: verdict.clone(),
            date: None,
            round: None,
            model: None,
            tasks_hash: None,
            body: body.clone(),
        },
        VetBlock::Gate {
            verdict,
            tasks_hash,
            date,
            body,
            ..
        } => JsonVetBlock {
            block: "gate",
            verdict: verdict.clone(),
            date: Some(date.clone()),
            round: None,
            model: None,
            tasks_hash: Some(tasks_hash.clone()),
            body: body.clone(),
        },
    }
}

// ---------------------------------------------------------------------------
// Text renderer
// ---------------------------------------------------------------------------

/// Render the same filtered content as human-readable text into `out`.
/// `--json` toggles representation, never content, so this walks the
/// identical filtered view the JSON renderer does.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] if a write to `out` fails.
pub fn render_text(view: &FilteredJournal, out: &mut impl std::io::Write) -> std::io::Result<()> {
    match view {
        FilteredJournal::Task {
            spec,
            task,
            generated_at,
            latest_round,
            blocks,
        } => {
            writeln!(out, "{spec} {task} (generated {generated_at})")?;
            if let Some(r) = latest_round {
                writeln!(out, "latest round: {r}")?;
            }
            for block in blocks {
                render_text_journal_block(block, out)?;
            }
        }
        FilteredJournal::Vet {
            spec,
            generated_at,
            latest_round,
            invocations,
        } => {
            writeln!(out, "{spec} VET.md (generated {generated_at})")?;
            if let Some(r) = latest_round {
                writeln!(out, "latest round: {r}")?;
            }
            for inv in invocations {
                writeln!(
                    out,
                    "## Invocation {n} — {date}",
                    n = inv.number,
                    date = inv.date
                )?;
                for block in &inv.blocks {
                    render_text_vet_block(block, out)?;
                }
            }
        }
    }
    Ok(())
}

fn render_text_journal_block(
    entry: &JournalEntry,
    out: &mut impl std::io::Write,
) -> std::io::Result<()> {
    match entry {
        JournalEntry::Implementer {
            date, model, round, ..
        } => writeln!(out, "- implementer round={round} date={date} model={model}"),
        JournalEntry::Review {
            date,
            model,
            persona,
            verdict,
            round,
            ..
        } => writeln!(
            out,
            "- review round={round} date={date} persona={persona} verdict={verdict} model={model}",
        ),
        JournalEntry::Blockers { date, round, .. } => {
            writeln!(out, "- blockers round={round} date={date}")
        }
    }
}

fn render_text_vet_block(block: &VetBlock, out: &mut impl std::io::Write) -> std::io::Result<()> {
    match block {
        VetBlock::DriftReview {
            verdict,
            round,
            date,
            ..
        } => writeln!(
            out,
            "- drift-review round={round} verdict={verdict} date={date}"
        ),
        VetBlock::HolisticFix {
            verdict,
            round,
            date,
            ..
        } => writeln!(
            out,
            "- holistic-fix round={round} verdict={verdict} date={date}"
        ),
        VetBlock::SimplifierScan { verdict, .. } => {
            writeln!(out, "- simplifier-scan verdict={verdict}")
        }
        VetBlock::SimplifierApply { verdict, .. } => {
            writeln!(out, "- simplifier-apply verdict={verdict}")
        }
        VetBlock::Gate {
            verdict,
            tasks_hash,
            date,
            ..
        } => writeln!(
            out,
            "- gate verdict={verdict} date={date} tasks_hash={tasks_hash}"
        ),
    }
}

// ---------------------------------------------------------------------------
// Construction from parsed docs (no filtering — filters live in journal_show)
// ---------------------------------------------------------------------------

/// Build an unfiltered task view from a parsed [`JournalDoc`]. The caller
/// in `journal_show` applies the filters before rendering.
#[must_use = "the view is the input to the renderers"]
pub fn task_view(
    doc: JournalDoc,
    latest_round: Option<u32>,
    blocks: Vec<JournalEntry>,
) -> FilteredJournal {
    FilteredJournal::Task {
        spec: doc.spec,
        task: doc.task,
        generated_at: doc.generated_at,
        latest_round,
        blocks,
    }
}

/// Build a vet view from a parsed [`VetDoc`]'s frontmatter plus the
/// already-filtered invocation sections.
#[must_use = "the view is the input to the renderers"]
pub fn vet_view(
    doc: VetDoc,
    latest_round: Option<u32>,
    invocations: Vec<FilteredInvocation>,
) -> FilteredJournal {
    FilteredJournal::Vet {
        spec: doc.spec,
        generated_at: doc.generated_at,
        latest_round,
        invocations,
    }
}
