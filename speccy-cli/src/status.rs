//! `speccy status` command logic.
//!
//! Discovers the project root, scans `.speccy/specs/`, runs the lint
//! engine, and assembles a [`StatusReport`] that the renderers in
//! [`crate::status_output`] turn into text or JSON. The command is
//! strictly read-only.

use crate::git::repo_sha;
use crate::status_output::JsonDiagnostic;
use crate::status_output::JsonLintBlock;
use crate::status_output::JsonOutput;
use crate::status_output::JsonSpec;
use crate::status_output::JsonTaskCounts;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::lint;
use speccy_core::lint::Diagnostic;
use speccy_core::lint::Level;
use speccy_core::workspace::Staleness;
use speccy_core::workspace::TaskCounts;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::count_open_questions;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use speccy_core::workspace::stale_for;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StatusError {
    /// `.speccy/` not found walking up from cwd.
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
    /// JSON serialisation failed (should be unreachable for our owned
    /// types).
    #[error("failed to serialise status JSON")]
    JsonSerialise(#[from] serde_json::Error),
}

/// `speccy status` arguments.
#[derive(Debug, Clone, Copy)]
pub struct StatusArgs {
    /// Emit JSON instead of the filtered text view.
    pub json: bool,
}

/// One spec's derived view used by both renderers.
#[derive(Debug)]
pub struct SpecView<'a> {
    /// Borrowed handle into the owning [`Workspace`].
    pub parsed: &'a speccy_core::lint::ParsedSpec,
    /// Identifier used for display (`spec_id` if present, else "?").
    pub display_id: String,
    /// Title pulled from frontmatter, else fallback string.
    pub display_title: String,
    /// Status string from frontmatter, else "unknown" when parse failed.
    pub display_status: String,
    /// Lint diagnostics whose `spec_id` matches this spec.
    pub diagnostics: Vec<Diagnostic>,
    /// Aggregated task state counts.
    pub task_counts: TaskCounts,
    /// Staleness result.
    pub staleness: Staleness,
    /// Count of unchecked open-question bullets.
    pub open_questions: usize,
    /// Inverse supersedes list copied for stable rendering.
    pub superseded_by: Vec<String>,
    /// First parse error encountered for this spec, if any.
    pub parse_error: Option<String>,
}

/// Full status report assembled from a [`Workspace`] and a lint pass.
#[derive(Debug)]
pub struct StatusReport<'a> {
    /// Owning workspace.
    pub workspace: &'a Workspace,
    /// One view per spec, in workspace order.
    pub specs: Vec<SpecView<'a>>,
    /// Diagnostics with `spec_id = None` (workspace-level).
    pub workspace_diagnostics: Vec<Diagnostic>,
    /// `repo_sha` from `git rev-parse HEAD`, or `""` if unavailable.
    pub repo_sha: String,
}

/// Synthetic lint code emitted by `status` (not by `speccy_core::lint`)
/// for each `supersedes:` reference that names a spec not present in
/// the workspace. Workspace-scoped (no `spec_id`).
const WS_DANGLING_SUPERSEDES: &str = "WS-001";

/// Build a [`StatusReport`] from a workspace and a precomputed lint
/// pass. The lint pass is precomputed (rather than computed inside) so
/// integration tests can swap it.
#[must_use = "the assembled report is the input to the renderers"]
pub fn assemble<'a>(
    workspace: &'a Workspace,
    diagnostics: Vec<Diagnostic>,
    repo_sha_value: String,
) -> StatusReport<'a> {
    let (per_spec, mut workspace_diagnostics) = partition_diagnostics(diagnostics);
    synthesize_workspace_diagnostics(workspace, &mut workspace_diagnostics);

    let specs: Vec<SpecView<'a>> = workspace
        .specs
        .iter()
        .map(|parsed| {
            let diags = parsed
                .spec_id
                .as_ref()
                .map(|id| per_spec_for(id, &per_spec))
                .unwrap_or_default();
            build_view(workspace, parsed, diags)
        })
        .collect();

    StatusReport {
        workspace,
        specs,
        workspace_diagnostics,
        repo_sha: repo_sha_value,
    }
}

fn synthesize_workspace_diagnostics(workspace: &Workspace, out: &mut Vec<Diagnostic>) {
    for dangling in workspace.supersession.dangling_references() {
        out.push(Diagnostic {
            code: WS_DANGLING_SUPERSEDES,
            level: Level::Warn,
            message: format!(
                "supersession references unknown spec `{dangling}`; either the target is missing from the workspace or the reference is a typo"
            ),
            spec_id: None,
            file: None,
            line: None,
        });
    }
}

fn per_spec_for(id: &str, per_spec: &[(String, Vec<Diagnostic>)]) -> Vec<Diagnostic> {
    per_spec
        .iter()
        .find(|(spec_id, _)| spec_id == id)
        .map(|(_, diags)| diags.clone())
        .unwrap_or_default()
}

fn partition_diagnostics(
    diagnostics: Vec<Diagnostic>,
) -> (Vec<(String, Vec<Diagnostic>)>, Vec<Diagnostic>) {
    let mut per_spec: Vec<(String, Vec<Diagnostic>)> = Vec::new();
    let mut workspace_level: Vec<Diagnostic> = Vec::new();
    for diag in diagnostics {
        match &diag.spec_id {
            Some(id) => {
                if let Some(existing) = per_spec.iter_mut().find(|(s, _)| s == id) {
                    existing.1.push(diag);
                } else {
                    per_spec.push((id.clone(), vec![diag]));
                }
            }
            None => workspace_level.push(diag),
        }
    }
    (per_spec, workspace_level)
}

fn count_by_level(diagnostics: &[Diagnostic]) -> (usize, usize, usize) {
    let mut errors = 0;
    let mut warnings = 0;
    let mut info = 0;
    for diag in diagnostics {
        match diag.level {
            Level::Error => errors += 1,
            Level::Warn => warnings += 1,
            Level::Info => info += 1,
        }
    }
    (errors, warnings, info)
}

fn build_view<'a>(
    workspace: &Workspace,
    parsed: &'a speccy_core::lint::ParsedSpec,
    diagnostics: Vec<Diagnostic>,
) -> SpecView<'a> {
    let (display_id, display_title, display_status) = display_fields(parsed);
    let task_counts = parsed
        .tasks_md_ok()
        .map_or(TaskCounts::default(), TaskCounts::from_tasks);
    let staleness = parsed.spec_md_ok().map_or(Staleness::fresh(), |spec| {
        stale_for(
            spec,
            parsed.tasks_md_ok(),
            parsed.spec_md_mtime,
            parsed.tasks_md_mtime,
        )
    });
    let open_questions = parsed.spec_md_ok().map_or(0, count_open_questions);
    let superseded_by = parsed
        .spec_id
        .as_ref()
        .map(|id| workspace.supersession.superseded_by(id).to_vec())
        .unwrap_or_default();
    let parse_error = first_parse_error(parsed);

    SpecView {
        parsed,
        display_id,
        display_title,
        display_status,
        diagnostics,
        task_counts,
        staleness,
        open_questions,
        superseded_by,
        parse_error,
    }
}

fn display_fields(parsed: &speccy_core::lint::ParsedSpec) -> (String, String, String) {
    if let Ok(spec) = &parsed.spec_md {
        return (
            spec.frontmatter.id.clone(),
            spec.frontmatter.title.clone(),
            spec.frontmatter.status.as_str().to_owned(),
        );
    }
    let id = parsed
        .spec_id
        .clone()
        .unwrap_or_else(|| "SPEC-?".to_owned());
    (id, "<unparseable>".to_owned(), "unknown".to_owned())
}

fn first_parse_error(parsed: &speccy_core::lint::ParsedSpec) -> Option<String> {
    if let Err(e) = &parsed.spec_md {
        return Some(format!("SPEC.md: {e}"));
    }
    if let Err(e) = &parsed.spec_doc {
        return Some(format!("SPEC.md (elements): {e}"));
    }
    if let Some(Err(e)) = &parsed.tasks_md {
        return Some(format!("TASKS.md: {e}"));
    }
    None
}

/// Whether a spec should be shown in the default (filtered) text view.
#[must_use = "filter result drives text rendering"]
pub fn show_in_text_view(view: &SpecView<'_>) -> bool {
    if view.display_status == "in-progress" {
        return true;
    }
    let has_lint_errors = view
        .diagnostics
        .iter()
        .any(|d| matches!(d.level, Level::Error));
    has_lint_errors || view.staleness.stale || view.parse_error.is_some()
}

/// Run `speccy status` from `cwd`, writing the result to `out` and
/// `err` for human-facing output. Returns the process exit code.
///
/// # Errors
///
/// Returns [`StatusError`] when the cwd cannot be resolved or
/// `.speccy/` cannot be found.
pub fn run(
    args: StatusArgs,
    cwd: &Utf8Path,
    out: &mut dyn std::io::Write,
) -> Result<(), StatusError> {
    let project_root = find_root(cwd)?;
    let workspace = scan(&project_root);
    let diagnostics = lint::run(&workspace.as_lint_workspace());
    let sha = repo_sha(&project_root);
    let report = assemble(&workspace, diagnostics, sha);

    if args.json {
        let json = build_json(&report)?;
        let mut text = serde_json::to_string_pretty(&json)?;
        text.push('\n');
        write_all(out, text.as_bytes())?;
    } else {
        render_text(&report, out)?;
    }
    Ok(())
}

fn write_all(out: &mut dyn std::io::Write, bytes: &[u8]) -> Result<(), StatusError> {
    out.write_all(bytes).map_err(StatusError::Cwd)
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`StatusError::Cwd`] if `std::env::current_dir` fails, or
/// [`StatusError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, StatusError> {
    let std_path = std::env::current_dir().map_err(StatusError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| StatusError::CwdNotUtf8)
}

fn render_text(report: &StatusReport<'_>, out: &mut dyn std::io::Write) -> Result<(), StatusError> {
    if report.workspace.specs.is_empty() {
        write_line(out, "No specs in workspace.")?;
        return Ok(());
    }

    let shown: Vec<&SpecView<'_>> = report
        .specs
        .iter()
        .filter(|v| show_in_text_view(v))
        .collect();
    if shown.is_empty() {
        write_line(out, "No in-progress specs need attention.")?;
    } else {
        for view in shown {
            render_spec_text(view, out)?;
        }
    }

    if !report.workspace_diagnostics.is_empty() {
        write_line(out, "")?;
        write_line(out, "Workspace lint:")?;
        for diag in &report.workspace_diagnostics {
            let line = format!(
                "  {code} ({level}): {msg}",
                code = diag.code,
                level = diag.level.as_str(),
                msg = diag.message,
            );
            write_line(out, &line)?;
        }
    }

    Ok(())
}

fn render_spec_text(view: &SpecView<'_>, out: &mut dyn std::io::Write) -> Result<(), StatusError> {
    let header = format!(
        "{id} {status}: {title}",
        id = view.display_id,
        status = view.display_status,
        title = view.display_title,
    );
    write_line(out, &header)?;

    let counts = view.task_counts;
    let tasks_line = format!(
        "  tasks: {open} open, {ip} in-progress, {ar} awaiting review, {done} done",
        open = counts.open,
        ip = counts.in_progress,
        ar = counts.awaiting_review,
        done = counts.done,
    );
    write_line(out, &tasks_line)?;

    let (errors, warnings, info) = count_by_level(&view.diagnostics);
    let lint_line = format!("  lint: {errors} errors, {warnings} warnings, {info} info");
    write_line(out, &lint_line)?;

    if view.staleness.stale {
        let reasons: Vec<&str> = view.staleness.reasons.iter().map(|r| r.as_str()).collect();
        let stale_line = format!("  stale: {}", reasons.join(", "));
        write_line(out, &stale_line)?;
    }

    if view.open_questions > 0 {
        let q_line = format!("  open questions: {}", view.open_questions);
        write_line(out, &q_line)?;
    }

    if let Some(err) = &view.parse_error {
        let err_line = format!("  parse error: {err}");
        write_line(out, &err_line)?;
    }

    Ok(())
}

fn write_line(out: &mut dyn std::io::Write, line: &str) -> Result<(), StatusError> {
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\n');
    out.write_all(&bytes).map_err(StatusError::Cwd)
}

/// Build the JSON output payload from a `StatusReport`.
///
/// # Errors
///
/// Returns [`StatusError::JsonSerialise`] if a downstream serializer
/// fails. With the current owned types this is unreachable, but the
/// signature stays a `Result` to keep room for future fields that
/// could introduce error cases.
pub fn build_json(report: &StatusReport<'_>) -> Result<JsonOutput, StatusError> {
    let specs: Vec<JsonSpec> = report.specs.iter().map(json_spec).collect();
    let workspace_lint = JsonLintBlock::from_diagnostics(&report.workspace_diagnostics);
    Ok(JsonOutput {
        schema_version: 1,
        repo_sha: report.repo_sha.clone(),
        specs,
        lint: workspace_lint,
    })
}

fn json_spec(view: &SpecView<'_>) -> JsonSpec {
    let frontmatter_supersedes = view
        .parsed
        .spec_md_ok()
        .map(|s| s.frontmatter.supersedes.clone())
        .unwrap_or_default();
    JsonSpec {
        id: view.display_id.clone(),
        slug: view
            .parsed
            .spec_md_ok()
            .map(|s| s.frontmatter.slug.clone())
            .unwrap_or_default(),
        title: view.display_title.clone(),
        status: view.display_status.clone(),
        supersedes: frontmatter_supersedes,
        superseded_by: view.superseded_by.clone(),
        tasks: JsonTaskCounts {
            open: view.task_counts.open,
            in_progress: view.task_counts.in_progress,
            awaiting_review: view.task_counts.awaiting_review,
            done: view.task_counts.done,
        },
        stale: view.staleness.stale,
        stale_reasons: view
            .staleness
            .reasons
            .iter()
            .map(|r| r.as_str().to_owned())
            .collect(),
        open_questions: view.open_questions,
        lint: JsonLintBlock::from_diagnostics(&view.diagnostics),
        parse_error: view.parse_error.clone(),
    }
}

/// Helpers for converting diagnostics into JSON shape.
impl JsonLintBlock {
    /// Group diagnostics by level into `errors`/`warnings`/`info`
    /// arrays, preserving input order within each level.
    #[must_use = "the grouped lint block goes into JSON output"]
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut info = Vec::new();
        for diag in diagnostics {
            let entry = JsonDiagnostic {
                code: diag.code.to_owned(),
                level: diag.level.as_str().to_owned(),
                message: diag.message.clone(),
                file: diag.file.as_ref().map(ToString::to_string),
                line: diag.line,
            };
            match diag.level {
                Level::Error => errors.push(entry),
                Level::Warn => warnings.push(entry),
                Level::Info => info.push(entry),
            }
        }
        JsonLintBlock {
            errors,
            warnings,
            info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpecView;
    use super::Staleness;
    use super::TaskCounts;
    use super::show_in_text_view;
    use camino::Utf8PathBuf;
    use speccy_core::lint::ParsedSpec;
    use speccy_core::workspace::StaleReason;

    fn fake_parsed() -> ParsedSpec {
        ParsedSpec {
            spec_id: Some("SPEC-0001".to_owned()),
            dir: Utf8PathBuf::from("/tmp"),
            spec_md_path: Utf8PathBuf::from("/tmp/SPEC.md"),
            tasks_md_path: None,
            spec_md: Err(speccy_core::ParseError::NonUtf8Path(
                "test-fixture".to_owned(),
            )),
            spec_doc: Err(speccy_core::ParseError::NonUtf8Path(
                "test-fixture".to_owned(),
            )),
            tasks_md: None,
            report_md: None,
            spec_md_mtime: None,
            tasks_md_mtime: None,
        }
    }

    fn fake_view<'a>(
        parsed: &'a ParsedSpec,
        status: &str,
        stale: bool,
        errors: usize,
    ) -> SpecView<'a> {
        let staleness = if stale {
            Staleness {
                stale: true,
                reasons: vec![StaleReason::HashDrift],
            }
        } else {
            Staleness::fresh()
        };
        let diagnostics = (0..errors)
            .map(|i| speccy_core::lint::Diagnostic {
                code: "SPC-001",
                level: speccy_core::lint::Level::Error,
                message: format!("err {i}"),
                spec_id: Some("SPEC-0001".to_owned()),
                file: None,
                line: None,
            })
            .collect();

        SpecView {
            parsed,
            display_id: "SPEC-0001".to_owned(),
            display_title: "x".to_owned(),
            display_status: status.to_owned(),
            diagnostics,
            task_counts: TaskCounts::default(),
            staleness,
            open_questions: 0,
            superseded_by: Vec::new(),
            parse_error: None,
        }
    }

    #[test]
    fn in_progress_specs_always_shown() {
        let parsed = fake_parsed();
        let view = fake_view(&parsed, "in-progress", false, 0);
        assert!(show_in_text_view(&view));
    }

    #[test]
    fn clean_implemented_specs_hidden() {
        let parsed = fake_parsed();
        let view = fake_view(&parsed, "implemented", false, 0);
        assert!(!show_in_text_view(&view));
    }

    #[test]
    fn stale_implemented_spec_shown() {
        let parsed = fake_parsed();
        let view = fake_view(&parsed, "implemented", true, 0);
        assert!(show_in_text_view(&view));
    }

    #[test]
    fn implemented_with_lint_errors_shown() {
        let parsed = fake_parsed();
        let view = fake_view(&parsed, "implemented", false, 1);
        assert!(show_in_text_view(&view));
    }
}
