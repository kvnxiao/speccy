//! `speccy check` command logic.
//!
//! Discovers the project root, scans `.speccy/specs/`, collects every
//! `[[checks]]` entry from successfully-parsed spec.toml files, and
//! executes them through the host shell. Manual checks render their
//! prompt and never spawn a subprocess. Executable checks inherit
//! stdio so child output streams live.
//!
//! See `.speccy/specs/0010-check-command/SPEC.md`.

use crate::check_selector::CheckSelector;
use crate::check_selector::SelectorError;
use crate::check_selector::parse_selector;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::exec::shell_command;
use speccy_core::lint::ParsedSpec;
use speccy_core::parse::CheckEntry;
use speccy_core::parse::CheckPayload;
use speccy_core::parse::SpecStatus;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find as find_task;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
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
    /// No spec.toml across the workspace contained a `[[checks]]` entry
    /// with the requested ID.
    #[error("no check with id `{id}` found in workspace; run `speccy status` to list specs")]
    NoCheckMatching {
        /// Check ID that produced no match.
        id: String,
    },
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during discovery or while writing framing output.
    #[error("I/O error during check execution")]
    Io(#[from] std::io::Error),
    /// `std::process::Command::status` failed to spawn the shell.
    #[error("failed to spawn shell process for {check_id}")]
    ChildSpawn {
        /// Check whose command could not be spawned.
        check_id: String,
        /// Underlying spawn error.
        #[source]
        source: std::io::Error,
    },
}

/// `speccy check` arguments.
#[derive(Debug, Clone, Default)]
pub struct CheckArgs {
    /// Optional polymorphic positional selector. When `None`, every
    /// discovered check runs. Accepted shapes: `SPEC-NNNN`,
    /// `SPEC-NNNN/CHK-NNN`, `SPEC-NNNN/T-NNN`, `CHK-NNN`, `T-NNN`. See
    /// [`crate::check_selector::parse_selector`].
    pub selector: Option<String>,
}

/// One check enriched with the `spec_id` and parent-spec lifecycle
/// status (drives in-flight categorisation and header lines).
#[derive(Debug, Clone)]
struct CollectedCheck {
    spec_id: String,
    spec_status: SpecStatus,
    entry: CheckEntry,
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

/// Run `speccy check` from `cwd`. Returns the intended process exit code
/// (per REQ-004: first non-zero from any executable check, or 1 when at
/// least one spec.toml failed to parse, or 0 otherwise).
///
/// `out` receives framing lines (`==>`, `<--`, summary, manual prompts).
/// `err` receives malformed-spec warnings. Child stdout/stderr streams
/// live via inherited stdio (bypassing both writers).
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

    execute_checks(&all_checks, project_root, out, malformed)
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

    execute_checks(&filtered, project_root, out, malformed)
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

    // When the user names the spec directly, make the skip explicit
    // (run_all silently skips dropped / superseded; here we surface it).
    if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
        writeln!(
            out,
            "spec {spec_id} is `{}`; no checks executed",
            spec_status.as_str(),
        )?;
        return Ok(0);
    }

    let label = spec
        .spec_id
        .clone()
        .unwrap_or_else(|| display_spec_label(&spec.dir));
    let (checks, malformed) = collect_for_spec(spec, &label, spec_status, err)?;

    if checks.is_empty() {
        writeln!(out, "No checks defined.")?;
        return Ok(i32::from(malformed > 0));
    }

    execute_checks(&checks, project_root, out, malformed)
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

    // Mirror run_spec: when the user names a dropped / superseded spec
    // directly, surface the skip explicitly instead of silently running
    // its checks. The status filter is a property of the parent spec,
    // not of the invocation form (see SPEC-0017 Assumptions).
    if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
        writeln!(
            out,
            "spec {spec_id} is `{}`; no checks executed",
            spec_status.as_str(),
        )?;
        return Ok(0);
    }

    let label = spec
        .spec_id
        .clone()
        .unwrap_or_else(|| display_spec_label(&spec.dir));
    let (spec_checks, malformed) = collect_for_spec(spec, &label, spec_status, err)?;

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

    execute_checks(&matched, project_root, out, malformed)
}

/// Resolve a task selector via `task_lookup::find`, collect every check
/// proving the requirements the task covers (deduplicated, first-occurrence
/// declared order), and delegate to [`execute_checks`].
///
/// Empty-covers is treated as informational (exit 0) per SPEC-0017's
/// Open-Question "Lean 0" decision: the line names the task ref and states
/// it covers no requirements; no subprocess spawns. CHK-IDs listed in
/// `[[requirements]].checks` but absent from `[[checks]]` are silently
/// skipped at this layer — the lint engine (SPEC-0003) is the right
/// surface for the absence.
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
            "task `{task_ref}` covers no requirements; no checks to run",
            task_ref = task_ref.as_arg(),
        )?;
        return Ok(0);
    }

    let spec = resolve_spec(&ws, &location.spec_id)?;
    let spec_status = spec
        .spec_md
        .as_ref()
        .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);

    // For each covered REQ-ID, walk its [[requirements]].checks list and
    // accumulate CHK-IDs in declared order, deduplicating on first
    // occurrence so a CHK proving multiple covered REQs appears once.
    let Ok(spec_toml) = spec.spec_toml.as_ref() else {
        // Parent spec.toml failed to parse; the task ref resolved against
        // SPEC.md / TASKS.md, so surface the warning via collect_for_spec
        // and let it return an empty check set.
        let label = spec
            .spec_id
            .clone()
            .unwrap_or_else(|| display_spec_label(&spec.dir));
        let (_checks, malformed) = collect_for_spec(spec, &label, spec_status, err)?;
        writeln!(out, "No checks defined.")?;
        return Ok(i32::from(malformed > 0));
    };

    let mut ordered_check_ids: Vec<&str> = Vec::new();
    for req_id in &location.task.covers {
        let Some(req_entry) = spec_toml.requirements.iter().find(|r| &r.id == req_id) else {
            // Covered REQ-ID is not present in [[requirements]] — a lint
            // engine concern (SPEC-0003), silently skipped here.
            continue;
        };
        for chk_id in &req_entry.checks {
            if !ordered_check_ids.contains(&chk_id.as_str()) {
                ordered_check_ids.push(chk_id.as_str());
            }
        }
    }

    let label = spec
        .spec_id
        .clone()
        .unwrap_or_else(|| display_spec_label(&spec.dir));

    // Resolve each surviving CHK-ID against [[checks]]; drop any that the
    // [[requirements]] block referenced but [[checks]] does not define
    // (lint-engine concern, silently skipped here).
    let mut collected: Vec<CollectedCheck> = Vec::new();
    for chk_id in &ordered_check_ids {
        if let Some(entry) = spec_toml.checks.iter().find(|c| c.id == *chk_id) {
            collected.push(CollectedCheck {
                spec_id: label.clone(),
                spec_status,
                entry: entry.clone(),
            });
        }
    }

    if collected.is_empty() {
        writeln!(out, "No checks defined.")?;
        return Ok(0);
    }

    execute_checks(&collected, project_root, out, 0)
}

/// Locate a spec in the workspace by its `SPEC-NNNN` identifier.
///
/// Matches against [`ParsedSpec::spec_id`] (pulled from SPEC.md
/// frontmatter when parsing succeeded; falls back to the directory-derived
/// label otherwise), so a malformed SPEC.md still resolves through the
/// directory name.
///
/// # Errors
///
/// Returns [`SelectorError::NoSpecMatching`] when no spec under
/// `.speccy/specs/` matches the requested identifier.
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

fn execute_checks(
    checks: &[CollectedCheck],
    project_root: &Utf8Path,
    out: &mut dyn Write,
    malformed: u32,
) -> Result<i32, CheckError> {
    let mut passed: u32 = 0;
    let mut failed: u32 = 0;
    let mut in_flight: u32 = 0;
    let mut manual: u32 = 0;
    let mut first_gating_nonzero: Option<i32> = None;

    for c in checks {
        match &c.entry.payload {
            CheckPayload::Prompt(prompt) => {
                render_manual(c, prompt, out)?;
                manual = manual.saturating_add(1);
            }
            CheckPayload::Command(command) => {
                let code = run_executable(c, command, project_root, out)?;
                if code == 0 {
                    passed = passed.saturating_add(1);
                } else if matches!(c.spec_status, SpecStatus::InProgress) {
                    in_flight = in_flight.saturating_add(1);
                } else {
                    failed = failed.saturating_add(1);
                    if first_gating_nonzero.is_none() {
                        first_gating_nonzero = Some(code);
                    }
                }
            }
        }
    }

    writeln!(
        out,
        "{passed} passed, {failed} failed, {in_flight} in-flight, {manual} manual",
    )?;

    let exit = first_gating_nonzero.unwrap_or(i32::from(malformed > 0));
    Ok(exit)
}

fn render_manual(c: &CollectedCheck, prompt: &str, out: &mut dyn Write) -> Result<(), CheckError> {
    writeln!(out, "==> {} ({}, manual):", c.entry.id, c.spec_id)?;
    if prompt.ends_with('\n') {
        out.write_all(prompt.as_bytes())?;
    } else {
        writeln!(out, "{prompt}")?;
    }
    writeln!(out, "<-- {} MANUAL (verify and proceed)", c.entry.id)?;
    Ok(())
}

fn run_executable(
    c: &CollectedCheck,
    command: &str,
    project_root: &Utf8Path,
    out: &mut dyn Write,
) -> Result<i32, CheckError> {
    writeln!(
        out,
        "==> {} ({}): {}",
        c.entry.id, c.spec_id, c.entry.proves,
    )?;
    out.flush()?;

    let mut cmd = shell_command(command, project_root);
    let status = cmd.status().map_err(|source| CheckError::ChildSpawn {
        check_id: c.entry.id.clone(),
        source,
    })?;
    let code = status.code().unwrap_or(-1);

    if code == 0 {
        writeln!(out, "<-- {} PASS", c.entry.id)?;
    } else if matches!(c.spec_status, SpecStatus::InProgress) {
        writeln!(
            out,
            "<-- {} IN-FLIGHT (in-progress spec, exit {code})",
            c.entry.id,
        )?;
    } else {
        writeln!(out, "<-- {} FAIL (exit {code})", c.entry.id)?;
    }
    Ok(code)
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
        // Skip defunct specs entirely: their checks should never run.
        if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
            continue;
        }
        let (mut spec_checks, spec_malformed) = collect_for_spec(parsed, &label, spec_status, err)?;
        out.append(&mut spec_checks);
        malformed = malformed.saturating_add(spec_malformed);
    }
    Ok((out, malformed))
}

/// Collect every `[[checks]]` entry from one spec.toml, tagged with the
/// parent spec's label and lifecycle status. Returns the collected checks
/// plus a 1-or-0 malformed count (so callers can fold it into a workspace
/// total).
///
/// Caller is responsible for dropped / superseded status filtering: this
/// helper is reused by `run_all` (which already filters them out before
/// calling) and `run_spec` (which prints an explicit skip line and never
/// reaches this helper for defunct specs).
fn collect_for_spec(
    spec: &ParsedSpec,
    label: &str,
    spec_status: SpecStatus,
    err: &mut dyn Write,
) -> Result<(Vec<CollectedCheck>, u32), CheckError> {
    match &spec.spec_toml {
        Ok(toml) => {
            let collected = toml
                .checks
                .iter()
                .map(|check| CollectedCheck {
                    spec_id: label.to_owned(),
                    spec_status,
                    entry: check.clone(),
                })
                .collect();
            Ok((collected, 0))
        }
        Err(e) => {
            writeln!(
                err,
                "speccy check: warning: {label} spec.toml failed to parse: {e}; skipping",
            )?;
            Ok((Vec::new(), 1))
        }
    }
}

fn display_spec_label(dir: &Utf8Path) -> String {
    dir.file_name()
        .map_or_else(|| dir.to_string(), ToOwned::to_owned)
}
