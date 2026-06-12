//! `speccy journal show <selector> [--json] [--round latest|N]
//! [--verdict V] [--block TYPE]` command logic (SPEC-0055 REQ-006).
//!
//! Parses the resolved journal and emits its frontmatter plus the blocks
//! that survive the three conjunctive filters. Target inference follows
//! DEC-004, identical to `journal append`: a task selector
//! (`T-NNN` / `SPEC-NNNN/T-NNN`) resolves the per-task journal at
//! `<spec-dir>/journal/<task-id>.md` (parsed by `journal_xml`); a bare
//! `SPEC-NNNN` selector resolves `<spec-dir>/journal/VET.md` (parsed
//! in-flight by `vet_xml`, so a mid-vet-run VET.md whose last invocation
//! section is still open — no terminal `<gate>` yet — reads cleanly).
//!
//! The filters compose conjunctively: `--round latest|N` keeps the blocks
//! of the highest (or named) round; `--verdict V` keeps blocks whose
//! verdict equals `V`; `--block TYPE` keeps blocks of that element type. A
//! block lacking a filtered-on dimension (e.g. an `implementer` block under
//! `--verdict`) is dropped by that filter, since it cannot match. For
//! VET.md the round dimension resets per invocation section, so the round
//! filter applies within the **last** invocation section — the slice the
//! vet flow's call sites need.
//!
//! `--json` toggles representation, never content: the same filtered view
//! renders either as the schema-versioned JSON envelope or as text. A
//! missing journal file exits non-zero (the known call sites run only after
//! blocks exist, so absence is a loud anomaly).

use crate::check_selector::bare_spec_regex;
use crate::journal_show_output::FilteredInvocation;
use crate::journal_show_output::FilteredJournal;
use crate::journal_show_output::render_json;
use crate::journal_show_output::render_text;
use crate::journal_show_output::task_view;
use crate::journal_show_output::vet_view;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::parse::JournalEntry;
use speccy_core::parse::VetBlock;
use speccy_core::parse::parse_journal_xml;
use speccy_core::parse::parse_vet_in_flight;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find as find_task;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::scan;
use std::io::Write;
use thiserror::Error;

/// The `--round` filter: keep the highest round, or a specific round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundFilter {
    /// `--round latest`: keep only the highest round's blocks.
    Latest,
    /// `--round N`: keep only round `N`'s blocks.
    Exact(u32),
}

/// `speccy journal show` arguments.
#[derive(Debug, Clone)]
pub struct ShowArgs {
    /// Positional selector: a task selector for the per-task journal, or a
    /// bare `SPEC-NNNN` for VET.md.
    pub selector: String,
    /// Emit the JSON envelope rather than text.
    pub json: bool,
    /// Optional `--round latest|N` filter.
    pub round: Option<RoundFilter>,
    /// Optional `--verdict V` filter.
    pub verdict: Option<String>,
    /// Optional `--block TYPE` filter (element local name).
    pub block: Option<String>,
}

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ShowError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Selector failed to parse or resolve via `task_lookup`.
    #[error(transparent)]
    TaskLookup(#[from] LookupError),
    /// A bare spec selector resolved to no spec in the workspace.
    #[error("spec `{spec_id}` not found in any spec directory")]
    SpecNotFound {
        /// The canonical spec id that resolved to nothing.
        spec_id: String,
    },
    /// The resolved journal file does not exist. The known call sites only
    /// run after blocks must exist, so absence is surfaced as an anomaly.
    #[error("journal not found at {path}; nothing to show")]
    JournalNotFound {
        /// The journal path that was expected to exist.
        path: Utf8PathBuf,
    },
    /// The journal file failed to parse under its grammar.
    #[error("journal at {path} failed to parse")]
    Parse {
        /// The unparseable journal path.
        path: Utf8PathBuf,
        /// Underlying parse error.
        #[source]
        source: Box<speccy_core::error::ParseError>,
    },
    /// Reading the journal or serializing output failed.
    #[error("journal show I/O failed")]
    Io {
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// JSON serialization of the envelope failed.
    #[error("failed to serialize journal show envelope")]
    Serialize {
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },
}

/// Run `speccy journal show` from `cwd`, writing the rendered output to
/// `out`.
///
/// Routes by selector shape (DEC-004): a bare `SPEC-NNNN` resolves VET.md;
/// any other selector is parsed as a task reference and resolves the
/// per-task journal. Parses the resolved file, applies the conjunctive
/// filters, and writes the JSON envelope (`--json`) or the text form.
///
/// # Errors
///
/// Returns a [`ShowError`] when discovery fails, the selector does not
/// resolve, the journal file is absent or unparseable, or an I/O /
/// serialization step fails.
pub fn run(args: ShowArgs, cwd: &Utf8Path, out: &mut impl Write) -> Result<(), ShowError> {
    let ShowArgs {
        selector,
        json,
        round,
        verdict,
        block,
    } = args;

    let project_root = crate::cwd::resolve_root(cwd, ShowError::ProjectRootNotFound)?;

    let view = if bare_spec_regex().is_match(&selector) {
        resolve_vet(
            &project_root,
            &selector,
            round,
            verdict.as_deref(),
            block.as_deref(),
        )?
    } else {
        resolve_task(
            &project_root,
            &selector,
            round,
            verdict.as_deref(),
            block.as_deref(),
        )?
    };

    if json {
        let envelope = render_json(&view);
        let s =
            serde_json::to_string(&envelope).map_err(|source| ShowError::Serialize { source })?;
        writeln!(out, "{s}").map_err(|source| ShowError::Io { source })?;
    } else {
        render_text(&view, out).map_err(|source| ShowError::Io { source })?;
    }
    Ok(())
}

/// Resolve a task selector and build the filtered per-task journal view.
fn resolve_task(
    project_root: &Utf8Path,
    selector: &str,
    round: Option<RoundFilter>,
    verdict: Option<&str>,
    block: Option<&str>,
) -> Result<FilteredJournal, ShowError> {
    let task_ref: TaskRef = parse_ref(selector)?;
    let ws = scan(project_root);
    let location = find_task(&ws, &task_ref)?;
    let task_id = location.task.id.clone();
    let journal_path = location
        .spec_dir
        .join("journal")
        .join(format!("{task_id}.md"));

    let src = match fs_err::read_to_string(journal_path.as_std_path()) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ShowError::JournalNotFound { path: journal_path });
        }
        Err(source) => return Err(ShowError::Io { source }),
    };
    let doc = parse_journal_xml(&src, &journal_path).map_err(|source| ShowError::Parse {
        path: journal_path.clone(),
        source,
    })?;

    // Round filter first: resolve `latest` against the whole file, then keep
    // only the named round's blocks. The other filters compose on top.
    let highest = doc.entries.iter().map(JournalEntry::round).max();
    let (target_round, latest_round) = match round {
        Some(RoundFilter::Latest) => (highest, highest),
        Some(RoundFilter::Exact(n)) => (Some(n), None),
        None => (None, None),
    };

    let blocks: Vec<JournalEntry> = doc
        .entries
        .iter()
        .filter(|e| target_round.is_none_or(|r| e.round() == r))
        .filter(|e| verdict.is_none_or(|v| journal_entry_verdict(e) == Some(v)))
        .filter(|e| block.is_none_or(|b| e.element_name() == b))
        .cloned()
        .collect();

    Ok(task_view(doc, latest_round, blocks))
}

/// Resolve a bare spec selector and build the filtered VET.md view.
fn resolve_vet(
    project_root: &Utf8Path,
    selector: &str,
    round: Option<RoundFilter>,
    verdict: Option<&str>,
    block: Option<&str>,
) -> Result<FilteredJournal, ShowError> {
    let ws = scan(project_root);
    let spec = ws
        .specs
        .iter()
        .find(|s| s.spec_id.as_deref() == Some(selector))
        .ok_or_else(|| ShowError::SpecNotFound {
            spec_id: selector.to_owned(),
        })?;
    let journal_path = spec.dir.join("journal").join("VET.md");

    let src = match fs_err::read_to_string(journal_path.as_std_path()) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ShowError::JournalNotFound { path: journal_path });
        }
        Err(source) => return Err(ShowError::Io { source }),
    };
    // In-flight parse: `journal show` is a read command and its known call
    // sites run mid-vet-run, when the last invocation section is still open
    // (a `drift-review` / `simplifier-scan` has landed but its terminal
    // `<gate>` has not). The strict parser would reject that legitimate
    // shape, so the parser the vet flow's reads need is the in-flight one.
    // A complete (gated) file parses identically under both, and structural
    // corruption is still rejected — only an open *last* section is tolerated.
    let doc = parse_vet_in_flight(&src, &journal_path).map_err(|source| ShowError::Parse {
        path: journal_path.clone(),
        source,
    })?;

    // VET.md rounds reset per invocation section: `--round latest|N` applies
    // within the LAST invocation section only. `latest` resolves against the
    // last section's highest round.
    let last_idx = doc.invocations.len().checked_sub(1);
    let last_highest = last_idx
        .and_then(|i| doc.invocations.get(i))
        .and_then(|inv| inv.blocks.iter().filter_map(vet_block_round).max());
    let (target_round, latest_round) = match round {
        Some(RoundFilter::Latest) => (last_highest, last_highest),
        Some(RoundFilter::Exact(n)) => (Some(n), None),
        None => (None, None),
    };

    let mut invocations: Vec<FilteredInvocation> = Vec::new();
    for (idx, inv) in doc.invocations.iter().enumerate() {
        // The round filter is scoped to the last section. Other sections keep
        // every block under a round filter (they have no "latest"/Nth round
        // to match against); the verdict/block filters still apply to all.
        let is_last = last_idx == Some(idx);
        let blocks: Vec<VetBlock> = inv
            .blocks
            .iter()
            .filter(|b| {
                if is_last {
                    target_round.is_none_or(|r| vet_block_round(b) == Some(r))
                } else {
                    // A round filter excludes earlier sections entirely so
                    // "latest round" never leaks blocks from a prior
                    // invocation.
                    target_round.is_none()
                }
            })
            .filter(|b| verdict.is_none_or(|v| vet_block_verdict(b) == v))
            .filter(|b| block.is_none_or(|name| b.element_name() == name))
            .cloned()
            .collect();
        if !blocks.is_empty() {
            invocations.push(FilteredInvocation {
                number: inv.number,
                date: inv.date.clone(),
                blocks,
            });
        }
    }

    Ok(vet_view(doc, latest_round, invocations))
}

/// The verdict of a per-task journal block, or `None` for the verdict-less
/// `implementer` / `blockers` blocks (which a `--verdict` filter drops).
fn journal_entry_verdict(entry: &JournalEntry) -> Option<&str> {
    match entry {
        JournalEntry::Review { verdict, .. } => Some(verdict.as_str()),
        JournalEntry::Implementer { .. } | JournalEntry::Blockers { .. } => None,
    }
}

/// The verdict of a vet block — every vet block carries one.
fn vet_block_verdict(block: &VetBlock) -> &str {
    match block {
        VetBlock::DriftReview { verdict, .. }
        | VetBlock::HolisticFix { verdict, .. }
        | VetBlock::SimplifierScan { verdict, .. }
        | VetBlock::SimplifierApply { verdict, .. }
        | VetBlock::Gate { verdict, .. } => verdict.as_str(),
    }
}

/// The round counter of a vet block, or `None` for the round-less block
/// types (`simplifier-scan` / `simplifier-apply` / `gate`).
fn vet_block_round(block: &VetBlock) -> Option<u32> {
    match block {
        VetBlock::DriftReview { round, .. } | VetBlock::HolisticFix { round, .. } => Some(*round),
        VetBlock::SimplifierScan { .. }
        | VetBlock::SimplifierApply { .. }
        | VetBlock::Gate { .. } => None,
    }
}
