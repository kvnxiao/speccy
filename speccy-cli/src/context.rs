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
//! SPEC-0056 grows the bundle across tasks T-002..T-006. This task
//! (T-002) establishes the command, the selector contract, and the JSON
//! skeleton with spec identity (REQ-001 / REQ-002) and the intent block
//! (REQ-002) populated. The command performs no writes anywhere.
//!
//! See `.speccy/specs/0056-task-context-bundle/SPEC.md`.

use crate::context_output::ContextBundle;
use crate::context_output::DecisionEntry;
use crate::context_output::Intent;
use crate::context_output::SpecIdentity;
use camino::Utf8Path;
use speccy_core::lint::ParsedSpec;
use speccy_core::parse::SpecDoc;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskLocation;
use speccy_core::task_lookup::find as find_task;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan_with_archive;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContextError {
    /// Selector parsing or resolution failure. Wraps the shared
    /// [`LookupError`] so the dispatcher renders the same selector
    /// diagnostic class `speccy check` produces (REQ-001), via the shared
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
    /// JSON serialisation of the bundle failed.
    #[error("failed to serialise context bundle JSON")]
    JsonSerialise(#[from] serde_json::Error),
    /// I/O failure during discovery or while writing the bundle.
    #[error("I/O error while emitting context bundle")]
    Io(#[from] std::io::Error),
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
/// `task_lookup::find` (REQ-001), assembles the identity + intent bundle
/// (REQ-002), and serialises it. Selector failures and parse failures
/// return an error without writing any partial bundle to `out`. The
/// command performs no writes anywhere in the workspace.
///
/// # Errors
///
/// See [`ContextError`] variants. CLI exit-code mapping (including the
/// `report_lookup_error` parity path for selector failures) lives in the
/// dispatcher.
pub fn run(args: ContextArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), ContextError> {
    let ContextArgs { selector, json } = args;

    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(ContextError::ProjectRootNotFound),
        Err(WorkspaceError::Io(e)) => return Err(ContextError::Io(e)),
        Err(other) => return Err(ContextError::Io(std::io::Error::other(other.to_string()))),
    };

    // Resolve the selector before touching the workspace scan result, so
    // an invalid-format selector fails fast with the shared diagnostic.
    let task_ref = parse_ref(&selector)?;

    // `speccy context` is a read command and never refuses on drift; like
    // `speccy check` it scans active specs only (archived specs are not in
    // scope for a task-context read).
    let ws = scan_with_archive(&project_root, false);

    let location = find_task(&ws, &task_ref)?;
    let bundle = assemble_bundle(&ws, &location)?;

    if json {
        let mut text = serde_json::to_string(&bundle)?;
        text.push('\n');
        out.write_all(text.as_bytes())?;
    } else {
        render_text(&bundle, out)?;
    }
    Ok(())
}

/// Assemble the T-002 bundle slice (identity + intent) from a resolved
/// task location.
fn assemble_bundle(
    ws: &Workspace,
    location: &TaskLocation<'_>,
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

    Ok(ContextBundle {
        schema_version: 1,
        spec: identity,
        intent,
    })
}

/// Project the SPEC.md element tree's intent surfaces into the bundle:
/// the `<goals>` and `<non-goals>` bodies plus every `<decision>` with
/// its id and body, in declared order. The Summary narrative,
/// `<user-stories>`, and non-covered requirement bodies are excluded by
/// construction — they are never read here (REQ-002).
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
    Ok(())
}
