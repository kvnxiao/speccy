//! `speccy check` command logic.
//!
//! Discovers the project root, scans `.speccy/specs/`, resolves the
//! SPEC-0017 selector against the scenarios reached via
//! `SpecDoc.requirements[*].scenarios` (the `speccy:scenario` markers
//! nested under each `speccy:requirement` marker in SPEC.md), and
//! renders the English validation scenario for each selected check.
//! Renders only — no child processes spawn (SPEC-0018 REQ-002).
//!
//! See `.speccy/specs/0018-remove-check-execution/SPEC.md` and
//! `.speccy/specs/0019-xml-canonical-spec-md/SPEC.md`.

use crate::check_selector::CheckSelector;
use crate::check_selector::SelectorError;
use crate::check_selector::parse_selector;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::lint::ParsedSpec;
use speccy_core::parse::Scenario;
use speccy_core::parse::SpecStatus;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find as find_task;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::collections::BTreeSet;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CheckError {
    /// Selector parsing or resolution failure (see [`SelectorError`]).
    #[error(transparent)]
    Selector(#[from] SelectorError),
    /// Task-form selector failed to resolve via `task_lookup::find`. The
    /// wrapped error carries the existing `LookupError` `Display` wording
    /// (e.g. `Ambiguous`, `NotFound`) byte-for-byte so the message is
    /// identical to `speccy implement` / `speccy review` against the same
    /// task reference.
    #[error(transparent)]
    TaskLookup(#[from] LookupError),
    /// No `speccy:scenario` marker nested under any
    /// `SpecDoc.requirements[*].scenarios` entry across the workspace
    /// carried the requested ID.
    #[error("no check with id `{id}` found in workspace; run `speccy status` to list specs")]
    NoCheckMatching {
        /// Check ID that produced no match.
        id: String,
    },
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during discovery or while writing framing output.
    #[error("I/O error during check rendering")]
    Io(#[from] std::io::Error),
}

/// `speccy check` arguments.
#[derive(Debug, Clone, Default)]
pub struct CheckArgs {
    /// Optional polymorphic positional selector. When `None`, every
    /// discovered check renders. Accepted shapes: `SPEC-NNNN`,
    /// `SPEC-NNNN/CHK-NNN`, `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`. See
    /// [`crate::check_selector::parse_selector`].
    pub selector: Option<String>,
}

/// One scenario enriched with the `spec_id` of its parent spec.
#[derive(Debug, Clone)]
struct CollectedCheck {
    spec_id: String,
    entry: Scenario,
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`CheckError::Io`] if `std::env::current_dir` fails, or if
/// the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, CheckError> {
    let std_path = std::env::current_dir()?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|path| {
        CheckError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "current working directory is not valid UTF-8: {}",
                path.display()
            ),
        ))
    })
}

/// Run `speccy check` from `cwd`. Returns the intended process exit code.
///
/// Per SPEC-0018 REQ-002, exit code is non-zero only for selector,
/// lookup, parse, or workspace errors. Scenario contents never gate the
/// exit code.
///
/// `out` receives framing lines (`==>`, indented continuations, summary).
/// `err` receives malformed-spec warnings.
///
/// # Errors
///
/// See [`CheckError`] variants. CLI exit-code mapping lives in the
/// dispatcher.
pub fn run(
    args: CheckArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let CheckArgs { selector } = args;

    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(CheckError::ProjectRootNotFound),
        Err(WorkspaceError::Io(e)) => return Err(CheckError::Io(e)),
        Err(other) => {
            return Err(CheckError::Io(std::io::Error::other(other.to_string())));
        }
    };

    let parsed = parse_selector(selector.as_deref())?;

    match parsed {
        CheckSelector::All => run_all(&project_root, out, err),
        CheckSelector::UnqualifiedCheck { check_id } => {
            run_unqualified_check(&check_id, &project_root, out, err)
        }
        CheckSelector::Spec { spec_id } => run_spec(&spec_id, &project_root, out, err),
        CheckSelector::QualifiedCheck { spec_id, check_id } => {
            run_qualified_check(&spec_id, &check_id, &project_root, out, err)
        }
        CheckSelector::Task(task_ref) => run_task(&task_ref, &project_root, out, err),
    }
}

fn run_all(
    project_root: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let ws = scan(project_root);
    let (all_checks, malformed) = collect_checks(&ws, err)?;

    if all_checks.is_empty() {
        writeln!(out, "No checks defined.")?;
        return Ok(i32::from(malformed > 0));
    }

    render_checks(&all_checks, out, malformed)
}

fn run_unqualified_check(
    check_id: &str,
    project_root: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let ws = scan(project_root);
    let (all_checks, malformed) = collect_checks(&ws, err)?;

    let filtered: Vec<CollectedCheck> = all_checks
        .into_iter()
        .filter(|c| c.entry.id == check_id)
        .collect();

    if filtered.is_empty() {
        return Err(CheckError::NoCheckMatching {
            id: check_id.to_owned(),
        });
    }

    render_checks(&filtered, out, malformed)
}

fn run_spec(
    spec_id: &str,
    project_root: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let ws = scan(project_root);
    let spec = resolve_spec(&ws, spec_id)?;

    let spec_status = spec
        .spec_md
        .as_ref()
        .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);

    // When the user names a dropped / superseded spec directly, make the
    // skip explicit (run_all silently skips them; here we surface it).
    if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
        writeln!(
            out,
            "spec {spec_id} is `{}`; no checks rendered",
            spec_status.as_str(),
        )?;
        return Ok(0);
    }

    let label = spec
        .spec_id
        .clone()
        .unwrap_or_else(|| display_spec_label(&spec.dir));
    let (checks, malformed) = collect_for_spec(spec, &label, err)?;

    if checks.is_empty() {
        writeln!(out, "No checks defined.")?;
        return Ok(i32::from(malformed > 0));
    }

    render_checks(&checks, out, malformed)
}

fn run_qualified_check(
    spec_id: &str,
    check_id: &str,
    project_root: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let ws = scan(project_root);
    let spec = resolve_spec(&ws, spec_id)?;

    let spec_status = spec
        .spec_md
        .as_ref()
        .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);

    if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
        writeln!(
            out,
            "spec {spec_id} is `{}`; no checks rendered",
            spec_status.as_str(),
        )?;
        return Ok(0);
    }

    let label = spec
        .spec_id
        .clone()
        .unwrap_or_else(|| display_spec_label(&spec.dir));
    let (spec_checks, malformed) = collect_for_spec(spec, &label, err)?;

    let matched: Vec<CollectedCheck> = spec_checks
        .into_iter()
        .filter(|c| c.entry.id == check_id)
        .collect();

    if matched.is_empty() {
        return Err(CheckError::Selector(
            SelectorError::NoQualifiedCheckMatching {
                spec_id: spec_id.to_owned(),
                check_id: check_id.to_owned(),
            },
        ));
    }

    render_checks(&matched, out, malformed)
}

/// Resolve a task selector via `task_lookup::find`, then walk
/// `spec_doc.requirements` for each REQ-ID the task covers and collect
/// every `req.scenarios` entry (deduplicated by scenario ID,
/// first-occurrence requirement-declared order), and render them.
///
/// Empty-covers is informational (exit 0): the line names the task ref
/// and states it covers no requirements. A REQ-ID in `task.covers` that
/// does not match any `req.id` under `spec_doc.requirements` is
/// silently skipped at this layer — the lint engine's TSK-001 is the
/// right surface for that absence.
fn run_task(
    task_ref: &TaskRef,
    project_root: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let ws = scan(project_root);
    let location = find_task(&ws, task_ref)?;

    if location.task.covers.is_empty() {
        writeln!(
            out,
            "task `{task_ref}` covers no requirements; no checks to render",
            task_ref = task_ref.as_arg(),
        )?;
        return Ok(0);
    }

    let spec = resolve_spec(&ws, &location.spec_id)?;

    let Ok(spec_doc) = spec.spec_doc.as_ref() else {
        // Parent SPEC.md marker tree failed to parse (or a stray
        // spec.toml is present); surface via collect_for_spec (one-shot
        // warning) and return an empty render set.
        let label = spec
            .spec_id
            .clone()
            .unwrap_or_else(|| display_spec_label(&spec.dir));
        let (_checks, malformed) = collect_for_spec(spec, &label, err)?;
        writeln!(out, "No checks defined.")?;
        return Ok(i32::from(malformed > 0));
    };

    let label = spec
        .spec_id
        .clone()
        .unwrap_or_else(|| display_spec_label(&spec.dir));

    // Accumulate scenarios in declared requirement order, deduplicating
    // on first occurrence so a scenario nested under multiple covered
    // REQs (impossible today since scenarios are owned by exactly one
    // requirement, but kept for symmetry with the pre-SPEC-0019 path)
    // renders exactly once.
    let mut collected: Vec<CollectedCheck> = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();
    for req_id in &location.task.covers {
        let Some(req) = spec_doc.requirements.iter().find(|r| &r.id == req_id) else {
            continue;
        };
        for scenario in &req.scenarios {
            if !seen_ids.iter().any(|s| s == &scenario.id) {
                seen_ids.push(scenario.id.clone());
                collected.push(CollectedCheck {
                    spec_id: label.clone(),
                    entry: scenario.clone(),
                });
            }
        }
    }

    if collected.is_empty() {
        writeln!(out, "No checks defined.")?;
        return Ok(0);
    }

    render_checks(&collected, out, 0)
}

/// Locate a spec in the workspace by its `SPEC-NNNN` identifier.
fn resolve_spec<'w>(ws: &'w Workspace, spec_id: &str) -> Result<&'w ParsedSpec, SelectorError> {
    ws.specs
        .iter()
        .find(|s| {
            let label = s
                .spec_id
                .clone()
                .unwrap_or_else(|| display_spec_label(&s.dir));
            label == spec_id
        })
        .ok_or_else(|| SelectorError::NoSpecMatching {
            spec_id: spec_id.to_owned(),
        })
}

/// Render each selected check's scenario. Header is:
/// `==> CHK-NNN (SPEC-NNNN): <scenario first line>`. Continuation lines
/// are indented by two spaces. Closes with a count summary:
/// `N scenarios rendered across M specs`.
fn render_checks(
    checks: &[CollectedCheck],
    out: &mut dyn Write,
    malformed: u32,
) -> Result<i32, CheckError> {
    let mut spec_set: BTreeSet<String> = BTreeSet::new();
    for c in checks {
        render_one(c, out)?;
        spec_set.insert(c.spec_id.clone());
    }

    let n = checks.len();
    let m = spec_set.len();
    writeln!(out, "{n} scenarios rendered across {m} specs")?;

    Ok(i32::from(malformed > 0))
}

fn render_one(c: &CollectedCheck, out: &mut dyn Write) -> Result<(), CheckError> {
    let scenario = c.entry.body.as_str();
    let mut lines = scenario.lines();
    let first = lines.next().unwrap_or("");
    writeln!(out, "==> {} ({}): {}", c.entry.id, c.spec_id, first)?;
    for cont in lines {
        writeln!(out, "  {cont}")?;
    }
    Ok(())
}

fn collect_checks(
    ws: &Workspace,
    err: &mut dyn Write,
) -> Result<(Vec<CollectedCheck>, u32), CheckError> {
    let mut out = Vec::new();
    let mut malformed: u32 = 0;
    for parsed in &ws.specs {
        let label = parsed
            .spec_id
            .clone()
            .unwrap_or_else(|| display_spec_label(&parsed.dir));
        let spec_status = parsed
            .spec_md
            .as_ref()
            .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);
        // Skip defunct specs entirely: their scenarios should not render
        // in the run-all path (run_spec/run_qualified_check surface the
        // skip explicitly when the user names them).
        if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
            continue;
        }
        let (mut spec_checks, spec_malformed) = collect_for_spec(parsed, &label, err)?;
        out.append(&mut spec_checks);
        malformed = malformed.saturating_add(spec_malformed);
    }
    Ok((out, malformed))
}

/// Collect every nested `speccy:scenario` from one spec's SPEC.md
/// marker tree, tagged with the parent spec's label. Returns the
/// collected scenarios plus a 1-or-0 malformed count so callers can
/// fold it into a workspace total.
fn collect_for_spec(
    spec: &ParsedSpec,
    label: &str,
    err: &mut dyn Write,
) -> Result<(Vec<CollectedCheck>, u32), CheckError> {
    match &spec.spec_doc {
        Ok(doc) => {
            let mut collected = Vec::new();
            for req in &doc.requirements {
                for scenario in &req.scenarios {
                    collected.push(CollectedCheck {
                        spec_id: label.to_owned(),
                        entry: scenario.clone(),
                    });
                }
            }
            Ok((collected, 0))
        }
        Err(e) => {
            writeln!(
                err,
                "speccy check: warning: {label} SPEC.md marker tree failed to parse: {e}; skipping",
            )?;
            Ok((Vec::new(), 1))
        }
    }
}

fn display_spec_label(dir: &Utf8Path) -> String {
    dir.file_name()
        .map_or_else(|| dir.to_string(), ToOwned::to_owned)
}
