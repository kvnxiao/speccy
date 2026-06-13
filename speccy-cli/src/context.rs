//! `speccy context` command logic.
//!
//! `speccy context <task-selector> [--json]` resolves a task selector and
//! emits one schema-versioned bundle scoped to that task. The bundle is a
//! single read that replaces a loop subagent's multi-step entry recipe
//! (full SPEC.md + TASKS.md + journal + `speccy check`).
//!
//! This module owns selector resolution (reusing `task_lookup` exactly as
//! `speccy check` does, so the two commands accept the same grammar and
//! produce the same selector diagnostics) and bundle assembly. The
//! envelope's `Serialize` shape lives in [`crate::context_output`].
//!
//! The bundle carries spec identity and the intent block; the selected
//! task's verbatim `<task>` entry and the covering requirements — resolved
//! through the shared core walk so `context` and `check` cannot diverge;
//! the selected task's per-task journal in full, reusing `journal show`'s
//! block projection so the two JSON journal views cannot drift; an absent
//! journal yields an explicit empty marker and a successful exit. The
//! navigation aids are a sibling-task index (id/state/covers only), the
//! repo-relative SPEC.md / TASKS.md / journal paths, and a best-effort
//! suggested merge-base diff command computed from git state — git
//! unavailability degrades the diff command to a `main`-baseline fallback,
//! never errors the bundle. The consistency section carries the
//! workspace-level status `speccy next` computes (via the shared
//! `consistency::detect` through `ShellGitProbe`) plus only the drift
//! entries scoped to the selected task — other tasks' drifts never appear.
//! `speccy context` is a read command and never refuses on drift; surfacing
//! the status at read time is the feedback mechanism. The command performs
//! no writes anywhere.

use crate::context_output::BundleJournal;
use crate::context_output::BundlePaths;
use crate::context_output::ContextBundle;
use crate::context_output::CoveringRequirement;
use crate::context_output::DecisionEntry;
use crate::context_output::Intent;
use crate::context_output::ScenarioEntry;
use crate::context_output::SiblingEntry;
use crate::context_output::SpecIdentity;
use crate::context_output::TaskEntry;
use crate::journal_show_output::to_json_journal_block;
use crate::journal_show_output::to_json_journal_block_attrs;
use crate::paths::to_repo_relative;
use camino::Utf8Path;
use speccy_core::consistency::ConsistencyBlock;
use speccy_core::consistency::ShellGitProbe;
use speccy_core::consistency::detect as detect_consistency;
use speccy_core::context::resolve_covering_requirements;
use speccy_core::lint::ParsedSpec;
use speccy_core::parse::SpecDoc;
use speccy_core::parse::latest_round;
use speccy_core::parse::parse_journal_xml;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskLocation;
use speccy_core::task_lookup::find as find_task;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::scan_with_archive;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContextError {
    /// Selector parsing or resolution failure. Wraps the shared
    /// [`LookupError`] so the dispatcher renders the same selector
    /// diagnostic class `speccy check` produces, via the shared
    /// `report_lookup_error` helper.
    #[error(transparent)]
    TaskLookup(#[from] LookupError),
    /// The resolved spec's SPEC.md element tree failed to parse, so the
    /// identity and intent slices cannot be assembled.
    #[error("SPEC.md element tree for `{spec_id}` failed to parse; cannot assemble context bundle")]
    SpecDocUnavailable {
        /// The spec whose element tree could not be parsed.
        spec_id: String,
    },
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// The selected task's journal file exists but failed to parse under
    /// the per-task journal grammar.
    #[error("journal at {path} failed to parse; cannot assemble context bundle")]
    JournalParse {
        /// The unparseable journal path.
        path: camino::Utf8PathBuf,
        /// Underlying parse error.
        #[source]
        source: Box<speccy_core::error::ParseError>,
    },
    /// JSON serialisation of the bundle failed.
    #[error("failed to serialise context bundle JSON")]
    JsonSerialise(#[from] serde_json::Error),
    /// I/O failure during discovery or while writing the bundle.
    #[error("I/O error while emitting context bundle")]
    Io(#[from] std::io::Error),
}

impl From<WorkspaceError> for ContextError {
    fn from(err: WorkspaceError) -> Self {
        match err {
            WorkspaceError::Io(e) => ContextError::Io(e),
            other => ContextError::Io(std::io::Error::other(other.to_string())),
        }
    }
}

/// `speccy context` arguments.
#[derive(Debug, Clone)]
pub struct ContextArgs {
    /// Task selector: `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN`
    /// (qualified). Same grammar as `speccy check`.
    pub selector: String,
    /// Emit the JSON envelope. Without it, the same bundle content renders
    /// in a human-readable text form — `--json` toggles representation,
    /// never content (the workspace-wide convention).
    pub json: bool,
}

/// Run `speccy context` from `cwd`, writing the rendered bundle to `out`.
///
/// Resolves the selector through `task_lookup::parse_ref` then
/// `task_lookup::find`, assembles the identity + intent bundle along with
/// the selected task entry and its covering requirements, and serialises
/// it. Selector failures and parse
/// failures return an error without writing any partial bundle to `out`.
/// The command performs no writes anywhere in the workspace.
///
/// # Errors
///
/// See [`ContextError`] variants. CLI exit-code mapping (including the
/// `report_lookup_error` parity path for selector failures) lives in the
/// dispatcher.
pub fn run(args: ContextArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), ContextError> {
    let ContextArgs { selector, json } = args;

    let project_root = crate::cwd::resolve_root(cwd, ContextError::ProjectRootNotFound)?;

    // Resolve the selector before touching the workspace scan result, so
    // an invalid-format selector fails fast with the shared diagnostic.
    let task_ref = parse_ref(&selector)?;

    // `speccy context` is a read command and never refuses on drift; like
    // `speccy check` it scans active specs only (archived specs are not in
    // scope for a task-context read).
    let ws = scan_with_archive(&project_root, false);

    let location = find_task(&ws, &task_ref)?;
    let bundle = assemble_bundle(&ws, &location, &project_root, cwd)?;

    if json {
        let mut text = serde_json::to_string(&bundle)?;
        text.push('\n');
        out.write_all(text.as_bytes())?;
    } else {
        render_text(&bundle, out)?;
    }
    Ok(())
}

/// Assemble the context bundle (identity + intent; task entry and covering
/// requirements; inlined journal; sibling index, paths, and suggested diff
/// command) from a resolved task location.
///
/// `project_root` relativises the surfaced file paths; `cwd` roots the
/// best-effort git probe for the suggested diff command.
fn assemble_bundle(
    ws: &Workspace,
    location: &TaskLocation<'_>,
    project_root: &Utf8Path,
    cwd: &Utf8Path,
) -> Result<ContextBundle, ContextError> {
    let spec = resolve_spec(ws, &location.spec_id)?;

    let spec_md = spec
        .spec_md_ok()
        .ok_or_else(|| ContextError::SpecDocUnavailable {
            spec_id: location.spec_id.clone(),
        })?;
    let spec_doc = spec
        .spec_doc_ok()
        .ok_or_else(|| ContextError::SpecDocUnavailable {
            spec_id: location.spec_id.clone(),
        })?;

    let identity = SpecIdentity {
        id: spec_md.frontmatter.id.clone(),
        title: spec_md.frontmatter.title.clone(),
        status: spec_md.frontmatter.status.as_str().to_owned(),
    };
    let intent = build_intent(spec_doc);
    let task = build_task_entry(location);
    let requirements = build_requirements(location, spec_doc);
    let journal_path = journal_path(location);
    let journal = build_journal(&journal_path)?;
    let siblings = build_siblings(location);
    let paths = build_paths(location, &journal_path, project_root);
    let diff_command = crate::git::suggested_diff_command(cwd);
    let consistency = build_consistency(spec, &location.spec_id, &location.task.id, project_root);

    Ok(ContextBundle {
        schema_version: 1,
        spec: identity,
        intent,
        task,
        requirements,
        journal,
        siblings,
        paths,
        diff_command,
        consistency,
    })
}

/// Build the consistency section: the same workspace-level status
/// `speccy next` computes (via the shared [`detect_consistency`] through a
/// [`ShellGitProbe`] rooted at the project root, exactly as `next.rs`
/// does), with the drift list filtered to the selected task only — other
/// tasks' drifts never appear regardless of count. The
/// aggregate `status` is preserved verbatim from the workspace scan: a
/// task with no drift of its own still surfaces a non-ok status when other
/// tasks drift, so the read-time feedback is honest. `speccy
/// context` never refuses on drift, so this never affects the exit code.
fn build_consistency(
    spec: &ParsedSpec,
    spec_id: &str,
    task_id: &str,
    project_root: &Utf8Path,
) -> ConsistencyBlock {
    let probe = ShellGitProbe::new(project_root);
    let block = detect_consistency(spec_id, spec, &probe);
    ConsistencyBlock {
        status: block.status,
        drifts: block
            .drifts
            .into_iter()
            .filter(|d| d.task_id == task_id)
            .collect(),
    }
}

/// Resolve `<spec-dir>/journal/<task-id>.md` — the canonical per-task
/// journal path, shared by the journal section and the surfaced
/// journal path field, so the two cannot disagree.
fn journal_path(location: &TaskLocation<'_>) -> camino::Utf8PathBuf {
    location
        .spec_dir
        .join("journal")
        .join(format!("{}.md", location.task.id))
}

/// Build the sibling-task index: every other task in the spec as
/// id/state/covers only — never any body text — in TASKS.md declared order,
/// excluding the selected task.
fn build_siblings(location: &TaskLocation<'_>) -> Vec<SiblingEntry> {
    let selected = &location.task.id;
    location
        .tasks_md
        .tasks
        .iter()
        .filter(|t| &t.id != selected)
        .map(|t| SiblingEntry {
            id: t.id.clone(),
            state: t.state.as_str().to_owned(),
            covers: t.covers.clone(),
        })
        .collect()
}

/// Build the repo-relative path triple — SPEC.md, TASKS.md, and the task's
/// journal file — for follow-up targeted reads. The journal path
/// is surfaced whether or not the file exists yet.
fn build_paths(
    location: &TaskLocation<'_>,
    journal_path: &Utf8Path,
    project_root: &Utf8Path,
) -> BundlePaths {
    BundlePaths {
        spec_md: to_repo_relative(&location.spec_dir.join("SPEC.md"), project_root),
        tasks_md: to_repo_relative(&location.spec_dir.join("TASKS.md"), project_root),
        journal: to_repo_relative(journal_path, project_root),
    }
}

/// Inline the selected task's per-task journal into the bundle.
///
/// Resolves `<spec-dir>/journal/<task-id>.md` (the same path `speccy journal
/// show` uses) and, when it exists, parses it via `journal_xml` and projects
/// every block in file order through the shared
/// [`to_json_journal_block`] mapping — so `context` and `journal show`
/// cannot drift. When the file is absent the bundle
/// carries an explicit `exists: false` marker with zero blocks and emission
/// still succeeds: a round-1 implementer legitimately has no journal yet.
fn build_journal(journal_path: &Utf8Path) -> Result<BundleJournal, ContextError> {
    let src = match fs_err::read_to_string(journal_path.as_std_path()) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BundleJournal {
                exists: false,
                spec: None,
                task: None,
                generated_at: None,
                blocks: Vec::new(),
                prior_rounds: Vec::new(),
            });
        }
        Err(e) => return Err(ContextError::Io(e)),
    };

    let doc =
        parse_journal_xml(&src, journal_path).map_err(|source| ContextError::JournalParse {
            path: journal_path.to_path_buf(),
            source,
        })?;

    // Inline only the latest round's blocks; prior
    // rounds become an attributes-only index, with their full prose
    // reachable via `speccy journal show --round N`. `latest_round` is the
    // shared resolver `journal show --round latest` also calls, so the two
    // views cannot drift. A journal that parses to zero entries
    // yields `None` here, hence empty `blocks` and `prior_rounds` with
    // `exists: true`.
    //
    // The two partitions are total and disjoint: every entry's round either
    // equals the highest (→ `blocks`, in full) or is strictly below it
    // (→ `prior_rounds`, attributes only); none is dropped or duplicated.
    let highest = latest_round(&doc.entries);
    let blocks = doc
        .entries
        .iter()
        .filter(|entry| highest.is_some_and(|r| entry.round() == r))
        .map(to_json_journal_block)
        .collect();
    let prior_rounds = doc
        .entries
        .iter()
        .filter(|entry| highest.is_some_and(|r| entry.round() < r))
        .map(to_json_journal_block_attrs)
        .collect();
    Ok(BundleJournal {
        exists: true,
        spec: Some(doc.spec),
        task: Some(doc.task),
        generated_at: Some(doc.generated_at),
        blocks,
        prior_rounds,
    })
}

/// Project the resolved task's `<task>` entry into the bundle: the parsed
/// `id`, `state`, and `covers` alongside the verbatim body bytes.
fn build_task_entry(location: &TaskLocation<'_>) -> TaskEntry {
    let task = location.task;
    TaskEntry {
        id: task.id.clone(),
        state: task.state.as_str().to_owned(),
        covers: task.covers.clone(),
        body: task.body.clone(),
    }
}

/// Resolve the task's covering requirements through the shared core walk
/// (so `context` and `check` cannot diverge) and
/// project each into the bundle with its done-when, behavior, and
/// scenarios. Requirements arrive deduplicated in covers-list order; a
/// `covers` token referencing a missing requirement is skipped by the
/// shared walk exactly as `speccy check` reports it.
fn build_requirements(location: &TaskLocation<'_>, spec_doc: &SpecDoc) -> Vec<CoveringRequirement> {
    resolve_covering_requirements(location.task, spec_doc)
        .into_iter()
        .map(|req| CoveringRequirement {
            id: req.id.clone(),
            body: req.body.clone(),
            done_when: req.done_when.clone(),
            behavior: req.behavior.clone(),
            scenarios: req
                .scenarios
                .iter()
                .map(|s| ScenarioEntry {
                    id: s.id.clone(),
                    body: s.body.clone(),
                })
                .collect(),
        })
        .collect()
}

/// Project the SPEC.md element tree's intent surfaces into the bundle:
/// the `<goals>` and `<non-goals>` bodies plus every `<decision>` with
/// its id and body, in declared order. The Summary narrative,
/// `<user-stories>`, and non-covered requirement bodies are excluded by
/// construction — they are never read here.
fn build_intent(spec_doc: &SpecDoc) -> Intent {
    let decisions = spec_doc
        .decisions
        .iter()
        .map(|d| DecisionEntry {
            id: d.id.clone(),
            body: d.body.clone(),
        })
        .collect();
    Intent {
        goals: spec_doc.goals.clone(),
        non_goals: spec_doc.non_goals.clone(),
        decisions,
    }
}

/// Locate the spec in the workspace by its `SPEC-NNNN` identifier. The
/// task lookup already proved the spec exists and parsed, so the
/// not-found branch is defensive.
fn resolve_spec<'w>(ws: &'w Workspace, spec_id: &str) -> Result<&'w ParsedSpec, ContextError> {
    ws.specs
        .iter()
        .find(|s| s.spec_id.as_deref() == Some(spec_id))
        .ok_or_else(|| ContextError::SpecDocUnavailable {
            spec_id: spec_id.to_owned(),
        })
}

/// Render the bundle in the human-readable text form. `--json` toggles
/// representation only, so the text form carries the same content. It
/// carries no stability guarantee (agents always pass `--json`).
fn render_text(bundle: &ContextBundle, out: &mut dyn Write) -> Result<(), ContextError> {
    writeln!(out, "schema_version: {}", bundle.schema_version)?;
    writeln!(out, "spec: {} — {}", bundle.spec.id, bundle.spec.title)?;
    writeln!(out, "status: {}", bundle.spec.status)?;
    writeln!(out, "\n## Goals\n{}", bundle.intent.goals.trim_end())?;
    writeln!(
        out,
        "\n## Non-goals\n{}",
        bundle.intent.non_goals.trim_end()
    )?;
    if bundle.intent.decisions.is_empty() {
        writeln!(out, "\n## Decisions\n(none)")?;
    } else {
        writeln!(out, "\n## Decisions")?;
        for dec in &bundle.intent.decisions {
            writeln!(out, "\n### {}\n{}", dec.id, dec.body.trim_end())?;
        }
    }

    writeln!(
        out,
        "\n## Task {} [{}] covers: {}",
        bundle.task.id,
        bundle.task.state,
        if bundle.task.covers.is_empty() {
            "(none)".to_owned()
        } else {
            bundle.task.covers.join(" ")
        },
    )?;
    writeln!(out, "{}", bundle.task.body.trim_end())?;

    if bundle.requirements.is_empty() {
        writeln!(out, "\n## Covering requirements\n(none)")?;
    } else {
        writeln!(out, "\n## Covering requirements")?;
        for req in &bundle.requirements {
            writeln!(out, "\n### {}\n{}", req.id, req.body.trim_end())?;
            writeln!(out, "\n#### done-when\n{}", req.done_when.trim_end())?;
            writeln!(out, "\n#### behavior\n{}", req.behavior.trim_end())?;
            for scenario in &req.scenarios {
                writeln!(
                    out,
                    "\n#### scenario {}\n{}",
                    scenario.id,
                    scenario.body.trim_end()
                )?;
            }
        }
    }

    render_journal(&bundle.journal, out)?;

    if bundle.siblings.is_empty() {
        writeln!(out, "\n## Sibling tasks\n(none)")?;
    } else {
        writeln!(out, "\n## Sibling tasks")?;
        for sib in &bundle.siblings {
            let covers = if sib.covers.is_empty() {
                "(none)".to_owned()
            } else {
                sib.covers.join(" ")
            };
            writeln!(out, "- {} [{}] covers: {covers}", sib.id, sib.state)?;
        }
    }

    writeln!(out, "\n## Paths")?;
    writeln!(out, "- SPEC.md: {}", bundle.paths.spec_md)?;
    writeln!(out, "- TASKS.md: {}", bundle.paths.tasks_md)?;
    writeln!(out, "- journal: {}", bundle.paths.journal)?;

    writeln!(out, "\n## Suggested diff command\n{}", bundle.diff_command)?;
    render_consistency(&bundle.consistency, out)?;
    Ok(())
}

/// Render the journal section in the text form: the latest round's blocks in
/// full, then — when the journal carries earlier rounds — an attributes-only
/// prior-rounds index. An absent journal renders an
/// explicit empty marker. `--json` toggles representation only, so this walks
/// the same `blocks` / `prior_rounds` partition the JSON renderer emits.
fn render_journal(journal: &BundleJournal, out: &mut dyn Write) -> Result<(), ContextError> {
    if !journal.exists {
        writeln!(out, "\n## Journal\n(none — task has no journal yet)")?;
        return Ok(());
    }
    writeln!(out, "\n## Journal")?;
    for block in &journal.blocks {
        let persona = block
            .persona
            .as_deref()
            .map_or_else(String::new, |p| format!(" persona={p}"));
        let verdict = block
            .verdict
            .as_deref()
            .map_or_else(String::new, |v| format!(" verdict={v}"));
        writeln!(
            out,
            "\n### {} round={}{persona}{verdict}\n{}",
            block.block,
            block.round,
            block.body.trim_end(),
        )?;
    }
    if !journal.prior_rounds.is_empty() {
        writeln!(out, "\n### Prior rounds (index)")?;
        for attrs in &journal.prior_rounds {
            let persona = attrs
                .persona
                .as_deref()
                .map_or_else(String::new, |p| format!(" persona={p}"));
            let verdict = attrs
                .verdict
                .as_deref()
                .map_or_else(String::new, |v| format!(" verdict={v}"));
            writeln!(
                out,
                "- {} round={}{persona}{verdict}",
                attrs.block, attrs.round,
            )?;
        }
    }
    Ok(())
}

/// Render the task-scoped consistency section in the text form: the
/// workspace-level status plus one line per drift entry scoped to the
/// selected task. Enum values are rendered through their serde
/// `snake_case` form so the text and JSON labels agree.
fn render_consistency(
    consistency: &ConsistencyBlock,
    out: &mut dyn Write,
) -> Result<(), ContextError> {
    let status = serde_json::to_value(consistency.status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    writeln!(out, "\n## Consistency\nstatus: {status}")?;
    if consistency.drifts.is_empty() {
        writeln!(out, "drifts: (none for this task)")?;
    } else {
        for drift in &consistency.drifts {
            let kind = serde_json::to_value(drift.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            writeln!(out, "- {} {} [{}]", drift.task_id, kind, drift.tasks_state)?;
        }
    }
    Ok(())
}
